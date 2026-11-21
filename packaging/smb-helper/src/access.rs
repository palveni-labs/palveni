use std::{io, ops::Range, time::Duration};

use smb2::{
    ClientConfig, DfsTargetPolicy, DirectoryReader, FileAttributes, FileIdentity, FileReader,
    SmbClient, Tree, msg::logoff::LogoffRequest, types::Command,
};
use tokio::{io::AsyncWriteExt, time::timeout};

use crate::{
    selection,
    wire::{self, Entry, Operation, Reply, Request},
};

const SETUP_TIMEOUT: Duration = Duration::from_secs(18);
const IO_TIMEOUT: Duration = Duration::from_secs(12);
const CLOSE_TIMEOUT: Duration = Duration::from_secs(2);
const BLOCK: u64 = 256 * 1024;

enum Failure {
    Invalid,
    Unavailable,
    Range(u64),
}

impl From<smb2::Error> for Failure {
    fn from(_: smb2::Error) -> Self {
        Self::Unavailable
    }
}

enum Output {
    Page(Vec<Entry>, Option<u32>),
    File {
        reader: Box<FileReader>,
        name: String,
        selection: Range<u64>,
        total: u64,
        partial: bool,
    },
}

struct Target {
    config: ClientConfig,
    share: String,
    base: Vec<String>,
    item: String,
    operation: Operation,
}

impl Target {
    fn parse(req: Request) -> Result<Self, Failure> {
        let (share, base) = selection::share_path(&req.path).ok_or(Failure::Invalid)?;
        if req.host.is_empty()
            || req.host.len() > 255
            || req
                .host
                .chars()
                .any(|c| c.is_control() || matches!(c, '/' | '\\' | '@' | '%' | '?' | '#'))
            || req.port == Some(0)
            || req.username.len() > 256
            || req.secret.len() > 4096
            || req
                .username
                .chars()
                .chain(req.secret.chars())
                .any(char::is_control)
        {
            return Err(Failure::Invalid);
        }
        let (domain, username) = req.username.split_once('\\').unwrap_or(("", &req.username));
        if username.is_empty() || username.eq_ignore_ascii_case("guest") || username.contains('\\')
        {
            return Err(Failure::Invalid);
        }
        let id = match &req.operation {
            Operation::List { parent, limit, .. } if (1..=200).contains(limit) => parent,
            Operation::Open { id, range }
                if !id.is_empty() && range.as_ref().is_none_or(|r| r.len() <= 256) =>
            {
                id
            }
            _ => return Err(Failure::Invalid),
        };
        let item = selection::item_path(id).ok_or(Failure::Invalid)?;
        let config = ClientConfig {
            addr: format!("{}:{}", req.host, req.port.unwrap_or(445)),
            username: username.to_owned(),
            domain: domain.to_owned(),
            password: req.secret,
            timeout: IO_TIMEOUT,
            response_timeout: Some(IO_TIMEOUT),
            auto_reconnect: false,
            compression: false,
            dfs_enabled: false,
            dfs_target_policy: DfsTargetPolicy::SameServerOnly,
            require_encryption: true,
            ..Default::default()
        };
        Ok(Self {
            config,
            share,
            base,
            item,
            operation: req.operation,
        })
    }
}

struct CheckedPath {
    path: String,
    identity: Option<FileIdentity>,
}

async fn checked_path(
    client: &mut SmbClient,
    tree: &mut Tree,
    base: &[String],
    item: &str,
    folder: bool,
) -> Result<CheckedPath, Failure> {
    let parts: Vec<_> = base
        .iter()
        .map(String::as_str)
        .chain(item.split('/').filter(|p| !p.is_empty()))
        .collect();
    if parts.len() > selection::MAX_DEPTH {
        return Err(Failure::Invalid);
    }
    let mut path = String::new();
    let mut identity = None;
    for (i, part) in parts.iter().enumerate() {
        if !path.is_empty() {
            path.push('/');
        }
        path.push_str(part);
        let last = i + 1 == parts.len();
        let info = if last {
            let (info, id) = client.stat_with_identity(tree, &path).await?;
            identity = id;
            info
        } else {
            client.stat(tree, &path).await?
        };
        // stat opens the reparse point itself, including intermediate directory components.
        if info.attributes.is_reparse_point()
            || info.is_link()
            || info.is_directory != (!last || folder)
            || i >= base.len()
                && (part.starts_with('.')
                    || info.attributes.is_hidden()
                    || info.attributes.contains(FileAttributes::SYSTEM))
        {
            return Err(Failure::Invalid);
        }
    }
    Ok(CheckedPath { path, identity })
}

async fn list(
    mut reader: DirectoryReader,
    parent: &str,
    offset: u32,
    limit: u32,
) -> Result<Output, Failure> {
    let result = async {
        let mut eligible = 0u32;
        let mut entries = Vec::with_capacity(limit as usize);
        while let Some(batch) = reader.next_batch().await? {
            for row in batch {
                if row.is_link()
                    || row.attributes.is_reparse_point()
                    || row.attributes.is_hidden()
                    || row.attributes.contains(FileAttributes::SYSTEM)
                    || row.name.starts_with('.')
                {
                    continue;
                }
                let Some(id) = selection::child_id(parent, &row.name) else {
                    continue;
                };
                if eligible >= offset {
                    if entries.len() == limit as usize {
                        return Ok(Output::Page(entries, Some(eligible)));
                    }
                    entries.push(Entry {
                        id,
                        name: row.name,
                        folder: row.is_directory,
                    });
                }
                eligible = eligible.checked_add(1).ok_or(Failure::Unavailable)?;
            }
        }
        Ok(Output::Page(entries, None))
    }
    .await;
    reader.close().await?;
    result
}

async fn prepare(
    client: &mut SmbClient,
    tree: &mut Tree,
    base: &[String],
    item: &str,
    operation: Operation,
) -> Result<Output, Failure> {
    let folder = matches!(operation, Operation::List { .. });
    let checked = checked_path(client, tree, base, item, folder).await?;
    match operation {
        Operation::List { offset, limit, .. } => {
            if !checked.path.is_empty() {
                match client.resolve(tree, &checked.path).await {
                    Ok(name) if name.path.to_lowercase() != checked.path.to_lowercase() => {
                        return Err(Failure::Invalid);
                    }
                    Err(e) if e.kind() != smb2::ErrorKind::Unsupported => {
                        return Err(Failure::Unavailable);
                    }
                    _ => {}
                }
            }
            let reader = client.open_directory_reader(tree, &checked.path).await?;
            let page = list(reader, item, offset, limit).await?;
            if !checked.path.is_empty() {
                let after = checked_path(client, tree, base, item, true).await?;
                if after.identity != checked.identity {
                    return Err(Failure::Invalid);
                }
            }
            Ok(page)
        }
        Operation::Open { range, .. } => {
            let reader = client.open_file_reader(tree, &checked.path).await?;
            let named = reader.resolved_path();
            let same_path = named.is_none_or(|p| p.to_lowercase() == checked.path.to_lowercase());
            let same_object = match (checked.identity, reader.version().identity) {
                (Some(first), Some(open)) => {
                    first.index_number == open.index_number
                        && (open.volume_serial == 0
                            || first.volume_serial == 0
                            || open.volume_serial == first.volume_serial)
                }
                _ => named.is_some() || base.is_empty(),
            };
            if reader.info().attributes.is_reparse_point() || !same_path || !same_object {
                reader.close().await?;
                return Err(Failure::Invalid);
            }
            let total = reader.size();
            let Some((selection, partial)) = selection::bytes(range.as_deref(), total) else {
                reader.close().await?;
                return Err(Failure::Range(total));
            };
            Ok(Output::File {
                reader: Box::new(reader),
                name: item.rsplit('/').next().unwrap_or_default().to_owned(),
                selection,
                total,
                partial,
            })
        }
    }
}

async fn send_file(
    out: &mut tokio::io::Stdout,
    reader: &FileReader,
    range: Range<u64>,
) -> io::Result<()> {
    let mut pos = range.start;
    while pos < range.end {
        let requested = (range.end - pos).min(BLOCK);
        let data = timeout(IO_TIMEOUT, reader.read_at(pos, requested))
            .await?
            .map_err(|_| io::ErrorKind::ConnectionReset)?;
        if data.is_empty() || data.len() as u64 > requested {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        timeout(Duration::from_secs(90), out.write_all(&data)).await??;
        pos += data.len() as u64;
    }
    timeout(IO_TIMEOUT, out.flush()).await?
}

async fn reply(result: Result<Output, Failure>) -> io::Result<()> {
    let mut out = tokio::io::stdout();
    let header = match result {
        Ok(Output::Page(entries, next)) => Reply::Page { entries, next },
        Ok(Output::File {
            reader,
            name,
            selection,
            total,
            partial,
        }) => {
            let header = Reply::File {
                name,
                start: selection.start,
                len: selection.end - selection.start,
                total,
                partial,
            };
            let sent = async {
                wire::send(&mut out, &header, wire::REPLY_BYTES).await?;
                send_file(&mut out, &reader, selection).await
            }
            .await;
            let _ = timeout(CLOSE_TIMEOUT, reader.close()).await;
            return sent;
        }
        Err(Failure::Invalid) => Reply::Invalid,
        Err(Failure::Unavailable) => Reply::Unavailable,
        Err(Failure::Range(total)) => Reply::Range { total },
    };
    timeout(IO_TIMEOUT, wire::send(&mut out, &header, wire::REPLY_BYTES)).await?
}

pub async fn run() -> io::Result<()> {
    let req: Request = timeout(
        IO_TIMEOUT,
        wire::receive(&mut tokio::io::stdin(), wire::COMMAND_BYTES),
    )
    .await??;
    let target = match Target::parse(req) {
        Ok(target) => target,
        Err(e) => return reply(Err(e)).await,
    };
    let mut client = match timeout(SETUP_TIMEOUT, SmbClient::connect(target.config)).await {
        Ok(Ok(client)) => client,
        _ => return reply(Err(Failure::Unavailable)).await,
    };
    let tree = timeout(IO_TIMEOUT, client.connect_share(&target.share)).await;
    let result = match tree {
        Ok(Ok(mut tree)) => {
            let output = timeout(
                SETUP_TIMEOUT,
                prepare(
                    &mut client,
                    &mut tree,
                    &target.base,
                    &target.item,
                    target.operation,
                ),
            )
            .await
            .unwrap_or(Err(Failure::Unavailable));
            let sent = reply(output).await;
            let _ = timeout(CLOSE_TIMEOUT, client.disconnect_share(&tree)).await;
            sent
        }
        _ => reply(Err(Failure::Unavailable)).await,
    };
    let _ = timeout(
        CLOSE_TIMEOUT,
        client
            .connection()
            .execute(Command::Logoff, &LogoffRequest, None),
    )
    .await;
    result
}
