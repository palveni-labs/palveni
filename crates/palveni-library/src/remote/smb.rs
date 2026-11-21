//! The SMB client runs in a separate process with one request, one connection and no inherited environment.
#[path = "../../../../packaging/smb-helper/src/wire.rs"]
mod wire;

use std::{
    io,
    pin::Pin,
    process::Stdio,
    sync::{Arc, LazyLock},
    task::{Context, Poll},
    time::Duration,
};

use palveni_core::{
    LibraryKind,
    connections::{ConnectionEntry, ConnectionEntryKind, ConnectionPage, Credentials},
};
use reqwest::Url;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt, ReadBuf, Take},
    process::{Child, ChildStdout, Command},
    sync::{OwnedSemaphorePermit, Semaphore},
    time::timeout,
};

use super::{RemoteError, RemoteStream};
use wire::{Operation, Reply};

// Keep the slot until the child exits, including when an HTTP listener disconnects during a seek.
static CHILDREN: LazyLock<Arc<Semaphore>> = LazyLock::new(|| Arc::new(Semaphore::new(6)));

struct Process {
    child: Option<(Child, OwnedSemaphorePermit)>,
}

impl Drop for Process {
    fn drop(&mut self) {
        if let Some((mut child, permit)) = self.child.take()
            && let Ok(runtime) = tokio::runtime::Handle::try_current()
        {
            runtime.spawn(async move {
                let _permit = permit;
                if timeout(Duration::from_secs(3), child.wait()).await.is_err() {
                    let _ = child.kill().await;
                }
            });
        }
    }
}

fn spawn(
    permit: OwnedSemaphorePermit,
) -> Result<(Process, tokio::process::ChildStdin, ChildStdout), RemoteError> {
    let exe = std::env::current_exe().map_err(|_| RemoteError::Unavailable)?;
    let dir = exe.parent().ok_or(RemoteError::Unavailable)?;
    let mut command = Command::new(dir.join(format!(
        "palveni-smb-helper{}",
        std::env::consts::EXE_SUFFIX
    )));
    command
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::MetadataExt;
        if std::fs::metadata("/proc/self")
            .map_err(|_| RemoteError::Unavailable)?
            .uid()
            == 0
        {
            command.gid(65534).uid(65534);
        }
    }
    let mut child = command.spawn().map_err(|_| RemoteError::Unavailable)?;
    let stdin = child.stdin.take().ok_or(RemoteError::Unavailable)?;
    let stdout = child.stdout.take().ok_or(RemoteError::Unavailable)?;
    Ok((
        Process {
            child: Some((child, permit)),
        },
        stdin,
        stdout,
    ))
}

async fn invoke(
    address: &Url,
    credentials: &Credentials,
    operation: Operation,
) -> Result<(Reply, ChildStdout, Process), RemoteError> {
    let request = wire::Request {
        host: address.host_str().ok_or(RemoteError::Invalid)?.to_owned(),
        port: address.port(),
        path: address.path().to_owned(),
        username: credentials.username.clone(),
        secret: credentials.secret.clone(),
        operation,
    };
    let permit = CHILDREN
        .clone()
        .try_acquire_owned()
        .map_err(|_| RemoteError::Unavailable)?;
    let (process, mut input, mut output) = spawn(permit)?;
    wire::send(&mut input, &request, wire::COMMAND_BYTES)
        .await
        .map_err(|_| RemoteError::Unavailable)?;
    input
        .shutdown()
        .await
        .map_err(|_| RemoteError::Unavailable)?;
    drop(input);
    let reply = wire::receive(&mut output, wire::REPLY_BYTES)
        .await
        .map_err(|_| RemoteError::Unavailable)?;
    match reply {
        Reply::Invalid => Err(RemoteError::Invalid),
        Reply::Unavailable => Err(RemoteError::Unavailable),
        Reply::Range { total } => Err(RemoteError::Range { total }),
        _ => Ok((reply, output, process)),
    }
}

fn file_kind(name: &str) -> ConnectionEntryKind {
    if crate::classify::classify(LibraryKind::Video, name).is_some() {
        ConnectionEntryKind::Video
    } else if crate::classify::classify(LibraryKind::Audiobook, name).is_some() {
        ConnectionEntryKind::Audio
    } else {
        ConnectionEntryKind::File
    }
}

pub(super) async fn entries(
    address: &Url,
    credentials: &Credentials,
    parent: &str,
    offset: u32,
    limit: u32,
) -> Result<ConnectionPage, RemoteError> {
    let (reply, _output, _process) = invoke(
        address,
        credentials,
        Operation::List {
            parent: parent.to_owned(),
            offset,
            limit,
        },
    )
    .await?;
    let Reply::Page { entries, next } = reply else {
        return Err(RemoteError::Unavailable);
    };
    if entries.len() > limit as usize || next.is_some_and(|n| n <= offset) {
        return Err(RemoteError::Unavailable);
    }
    Ok(ConnectionPage {
        entries: entries
            .into_iter()
            .map(|entry| ConnectionEntry {
                kind: if entry.folder {
                    ConnectionEntryKind::Folder
                } else {
                    file_kind(&entry.name)
                },
                id: entry.id,
                name: entry.name,
            })
            .collect(),
        next_offset: next,
    })
}

struct MediaPipe {
    stdout: Take<ChildStdout>,
    _process: Process,
}

impl AsyncRead for MediaPipe {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let before = buf.filled().len();
        if buf.remaining() == 0 || self.stdout.limit() == 0 {
            return Poll::Ready(Ok(()));
        }
        match Pin::new(&mut self.stdout).poll_read(cx, buf) {
            Poll::Ready(Ok(())) if buf.filled().len() == before => {
                Poll::Ready(Err(io::ErrorKind::UnexpectedEof.into()))
            }
            result => result,
        }
    }
}

pub(super) async fn open(
    address: &Url,
    credentials: &Credentials,
    id: &str,
    range: Option<&str>,
) -> Result<RemoteStream, RemoteError> {
    let op = Operation::Open {
        id: id.to_owned(),
        range: range.map(str::to_owned),
    };
    let (reply, output, process) = invoke(address, credentials, op).await?;
    let Reply::File {
        name,
        start,
        len,
        total,
        partial,
    } = reply
    else {
        return Err(RemoteError::Unavailable);
    };
    let mime = match file_kind(&name) {
        ConnectionEntryKind::Audio => crate::classify::audio_content_type(&name),
        ConnectionEntryKind::Video => crate::local::video_content_type(&name),
        _ => return Err(RemoteError::Invalid),
    };
    if !partial && (start != 0 || len != total)
        || partial && len == 0
        || start.checked_add(len).is_none_or(|end| end > total)
    {
        return Err(RemoteError::Unavailable);
    }
    Ok(RemoteStream {
        reader: Box::pin(MediaPipe {
            stdout: output.take(len),
            _process: process,
        }),
        start,
        len,
        total,
        partial,
        mime: mime.to_owned(),
    })
}
