use crate::{
    client_args::Cli,
    protocol::{
        MessageType, MyPkg, ToMessageType, BLOCK_SIZE, BLOCK_SIZE_LESS_HEADER, HEADER_SIZE,
        MSG_SIZE, MSG_TYPE,
    },
};
use anyhow::{bail, Result};
use futures::future::BoxFuture;
use serde::{de, Serialize};
use std::borrow::BorrowMut;
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

pub struct ClientConnection(TcpStream);
pub struct Client<S: ClientState> {
    // state is our marker
    // conn is available on all states and should be exposed as &mut self for read/writes
    inner: ClientConnection,
    // extra is a generic field for use within different states to squirrel data
    state: S,
}

pub trait ClientState {}
pub struct Connected;
pub struct Disconnected;

impl ClientState for Connected {}
impl ClientState for Disconnected {}

impl Client<Disconnected> {
    pub async fn open(addr: String) -> Result<Client<Connected>> {
        Ok(Client {
            inner: ClientConnection(TcpStream::connect(addr).await?),
            state: Connected,
        })
    }
}
pub struct ReadResult {
    pub message_type: u16,
    pub raw_msg: Vec<u8>,
}

impl Client<Connected> {
    pub async fn close(&mut self) -> Result<()> {
        self.inner.0.shutdown().await.map_err(anyhow::Error::from)
    }
    pub async fn write_message_type(&mut self, t: &MessageType) -> Result<()> {
        let b = t.serialize_inner()?;
        let message_type = t.message_type();
        let mut buf = [0; BLOCK_SIZE];
        for chunk in b.chunks(BLOCK_SIZE_LESS_HEADER) {
            let length = chunk.len() as u16;
            buf[0..MSG_SIZE].copy_from_slice(&length.to_be_bytes());
            buf[MSG_SIZE..HEADER_SIZE].copy_from_slice(&message_type.to_be_bytes());
            buf[HEADER_SIZE..HEADER_SIZE + chunk.len()].copy_from_slice(&chunk);
            self.inner
                .0
                .write_all(&buf[..HEADER_SIZE + length as usize])
                .await?;
        }
        Ok(())
    }
    pub async fn read_message_type(&mut self) -> Result<ReadResult> {
        let mut buf = [0; BLOCK_SIZE];
        let mut raw_msg = vec![];
        loop {
            self.inner.0.read_exact(&mut buf[..HEADER_SIZE]).await?;
            let prefix_length = u16::from_be_bytes([buf[0], buf[1]]) as usize;
            if prefix_length > BLOCK_SIZE_LESS_HEADER {
                bail!("invalid frame length {}", prefix_length);
            }
            let message_type = u16::from_be_bytes([buf[2], buf[3]]);
            if !MessageType::is_valid_message_type(message_type) {
                bail!("invalid message type {}", message_type);
            }
            let n = self
                .inner
                .0
                .read_exact(&mut buf[HEADER_SIZE..HEADER_SIZE + prefix_length])
                .await?;

            if n > 0 {
                raw_msg.extend_from_slice(&buf[HEADER_SIZE..HEADER_SIZE + n]);
            }
            if prefix_length < BLOCK_SIZE_LESS_HEADER {
                // return MessageType::deserialize(message_type, &raw_msg);
                return Ok(ReadResult {
                    message_type,
                    raw_msg,
                });
            }
        }
    }
    pub async fn read<T: de::DeserializeOwned>(&mut self) -> Result<T> {
        let r = self.read_message_type().await?;
        let msg: T = serde_bencode::from_bytes(&r.raw_msg).map_err(|e| dbg!(e))?;
        Ok(msg)
    }
    pub async fn write<T: ToMessageType>(&mut self, t: T) -> Result<()> {
        self.write_message_type(&t.to_message_type()).await
    }
    // pub async fn write<T: ToMessageType>(&mut self, t: T) -> Result<()> {
    //     self.write_message_type(&t.to_message_type()).await
    // }
    // pub async fn write(&mut self, t: &MessageType) -> Result<()> {
    //     let b = t.serialize_inner()?;
    //     let message_type = t.message_type();
    //     let mut buf = [0; BLOCK_SIZE];
    //     for chunk in b.chunks(BLOCK_SIZE_LESS_HEADER) {
    //         let length = chunk.len() as u16;
    //         buf[0..MSG_SIZE].copy_from_slice(&length.to_be_bytes());
    //         buf[MSG_SIZE..HEADER_SIZE].copy_from_slice(&message_type.to_be_bytes());
    //         buf[HEADER_SIZE..HEADER_SIZE + chunk.len()].copy_from_slice(&chunk);
    //         self.inner
    //             .0
    //             .write_all(&buf[..HEADER_SIZE + length as usize])
    //             .await?;
    //     }
    //     Ok(())
    // }
    // pub async fn read(&mut self) -> Result<MessageType> {
    //     let mut buf = [0; BLOCK_SIZE];
    //     let mut raw_msg = vec![];
    //     loop {
    //         self.inner
    //             .0
    //             .read_exact(&mut buf[..HEADER_SIZE])
    //             .await?;
    //         let prefix_length = u16::from_be_bytes([buf[0], buf[1]]) as usize;
    //         if prefix_length > BLOCK_SIZE_LESS_HEADER {
    //             bail!("invalid frame length {}", prefix_length);
    //         }
    //         let message_type = u16::from_be_bytes([buf[2], buf[3]]);
    //         if !MessageType::is_valid_message_type(message_type) {
    //             bail!("invalid message type {}", message_type);
    //         }
    //         let n = self
    //             .inner
    //             .0
    //             .read_exact(&mut buf[HEADER_SIZE..HEADER_SIZE + prefix_length])
    //             .await?;

    //         if n > 0 {
    //             raw_msg.extend_from_slice(&buf[HEADER_SIZE..HEADER_SIZE + n]);
    //         }
    //         if prefix_length < BLOCK_SIZE_LESS_HEADER {
    //             return MessageType::deserialize(message_type, &raw_msg);
    //         }
    //     }
    // }
    // pub async fn read<T: de::DeserializeOwned>(&mut self) -> Result<T> {
    //     let mut buf = [0; BLOCK_SIZE];
    //     let mut raw_msg = vec![];
    //     // listen for a response
    //     loop {
    //         self.inner.0.read_exact(&mut buf[..HEADER_SIZE]).await?;
    //         let prefix_length = u16::from_be_bytes([buf[0], buf[1]]) as usize;
    //         if prefix_length > BLOCK_SIZE_LESS_HEADER {
    //             bail!("invalid frame length {}", prefix_length);
    //         }
    //         let n = self
    //             .inner
    //             .0
    //             .read_exact(&mut buf[HEADER_SIZE..HEADER_SIZE + prefix_length])
    //             .await?;

    //         if n > 0 {
    //             raw_msg.extend_from_slice(&buf[HEADER_SIZE..HEADER_SIZE + n]);
    //         }
    //         if prefix_length < BLOCK_SIZE_LESS_HEADER {
    //             break;
    //         }
    //     }
    //     let msg: T = serde_bencode::from_bytes(&raw_msg).map_err(|e| dbg!(e))?;
    //     Ok(msg)
    // }
}
