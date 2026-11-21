use std::io;

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const COMMAND_BYTES: usize = 32 * 1024;
pub const REPLY_BYTES: usize = 2 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub host: String,
    pub port: Option<u16>,
    pub path: String,
    pub username: String,
    pub secret: String,
    pub operation: Operation,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase", deny_unknown_fields)]
pub enum Operation {
    List {
        parent: String,
        offset: u32,
        limit: u32,
    },
    Open {
        id: String,
        range: Option<String>,
    },
}

#[derive(Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub folder: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase", deny_unknown_fields)]
pub enum Reply {
    Page {
        entries: Vec<Entry>,
        next: Option<u32>,
    },
    File {
        name: String,
        start: u64,
        len: u64,
        total: u64,
        partial: bool,
    },
    Invalid,
    Unavailable,
    Range {
        total: u64,
    },
}

pub async fn receive<T: DeserializeOwned>(
    input: &mut (impl AsyncRead + Unpin),
    limit: usize,
) -> io::Result<T> {
    let len = input.read_u32().await? as usize;
    if len == 0 || len > limit {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let mut data = vec![0; len];
    input.read_exact(&mut data).await?;
    serde_json::from_slice(&data).map_err(|_| io::ErrorKind::InvalidData.into())
}

pub async fn send<T: Serialize>(
    output: &mut (impl AsyncWrite + Unpin),
    value: &T,
    limit: usize,
) -> io::Result<()> {
    let data = serde_json::to_vec(value).map_err(|_| io::ErrorKind::InvalidData)?;
    if data.len() > limit {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let len = u32::try_from(data.len()).map_err(|_| io::ErrorKind::InvalidData)?;
    output.write_u32(len).await?;
    output.write_all(&data).await?;
    output.flush().await
}
