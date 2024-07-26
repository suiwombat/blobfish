use crate::{
    protocol::{
        MessageType, ToMessageType, BLOCK_SIZE, BLOCK_SIZE_LESS_HEADER, HEADER_SIZE, MSG_SIZE,
    },
    server::Offer,
};
use anyhow::{bail, Error, Result};
use serde::de;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

pub struct ServerConnection(TcpListener);
pub struct Server<S: ServerState> {
    // extra is a generic field for use within different states to squirrel data
    state: S,
}

pub trait ServerState {}
pub struct Initial;
pub struct Listening {
    listener: ServerConnection,
    cache: Arc<RwLock<HashSet<String>>>,
    peers: HashSet<String>,
}
pub struct Connected {
    socket: TcpStream,
    cache: Arc<RwLock<HashSet<String>>>,
}
pub struct Disconnected;

impl ServerState for Initial {}
impl ServerState for Listening {}
impl ServerState for Connected {}

impl Server<Initial> {
    // TODO later for swarm
    // pub async fn open(addr: String) -> Result<Server<Connected>> {
    //     Ok(Server {
    //         inner: ServerConnection(TcpStream::connect(addr).await?),
    //         state: Connected,
    //     })
    // }
    pub async fn new(listener: TcpListener, peers: HashSet<String>) -> Result<Server<Listening>> {
        Ok(Server {
            state: Listening {
                listener: ServerConnection(listener),
                cache: Arc::new(RwLock::new(HashSet::new())),
                peers,
            },
        })
    }
}
impl Server<Listening> {
    pub async fn serve(&self, ctx: CancellationToken) -> Result<(), Error> {

        let tracker = TaskTracker::new();
        loop {
            tokio::select! {
                _ = ctx.cancelled() => {
                    tracker.close();
                    tracker.wait().await;
                    return Ok(());
                }
                event = self.state.listener.0.accept() => {
                    match event {
                        Ok((socket, _)) => {
                            let peers = self.state.peers.clone();
                            let cache = self.state.cache.clone();
                            let conn = Server {
                                state: Connected { socket, cache },
                            };
                            // Spawn a new task to handle the connection
                            tracker.spawn(async move {
                                if let Err(e) = conn.wait_for_offer(peers).await {
                                    eprintln!("Failed to handle connection: {:?}", e);
                                }
                            });
                        },
                        Err(e) => {dbg!(e);}
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ReadResult {
    pub message_type: u16,
    pub raw_msg: Vec<u8>,
}

impl Server<Connected> {
    pub async fn wait_for_offer(self, peers: HashSet<String>) -> Result<()> {
        let state = Offer::new(self);
        state
            .wait_for_mypkg()
            .await?
            .add_peers(peers)
            .negotiate()
            .await?
            .exchange()
            .await?;
        // .receive()
        // .await?;

        // tricky, how do i do multiple files? i need a loop or something
        Ok(())
    }
    pub fn get(&self, v: String) -> Option<String> {
        if let Some(v) = self.state.cache.read().unwrap().get(&v) {
            return Some(v.to_owned());
        }
        None
    }
    pub fn set(&mut self, v: String) -> bool {
        self.state.cache.write().unwrap().insert(v)
    }
    pub async fn close(&mut self) -> Result<()> {
        self.state
            .socket
            .shutdown()
            .await
            .map_err(anyhow::Error::from)
    }
    pub async fn write_message_type(&mut self, t: &MessageType) -> Result<()> {
        let b = t.serialize_inner()?;
        let message_type = t.message_type();
        let mut buf = [0; BLOCK_SIZE];
        for chunk in b.chunks(BLOCK_SIZE_LESS_HEADER) {
            let length = chunk.len() as u16;
            buf[0..MSG_SIZE].copy_from_slice(&length.to_be_bytes());
            buf[MSG_SIZE..HEADER_SIZE].copy_from_slice(&message_type.to_be_bytes());
            buf[HEADER_SIZE..HEADER_SIZE + chunk.len()].copy_from_slice(chunk);
            self.state
                .socket
                .write_all(&buf[..HEADER_SIZE + length as usize])
                .await?;
        }
        Ok(())
    }
    pub async fn read_message_type(&mut self) -> Result<ReadResult> {
        let mut buf = [0; BLOCK_SIZE];
        let mut raw_msg = vec![];
        loop {
            self.state
                .socket
                .read_exact(&mut buf[..HEADER_SIZE])
                .await?;
            let prefix_length = u16::from_be_bytes([buf[0], buf[1]]) as usize;
            if prefix_length > BLOCK_SIZE_LESS_HEADER {
                bail!("invalid frame length {}", prefix_length);
            }
            let message_type = u16::from_be_bytes([buf[2], buf[3]]);
            if !MessageType::is_valid_message_type(message_type) {
                bail!("invalid message type {}", message_type);
            }
            let n = self
                .state
                .socket
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
        let msg: T = serde_bencode::from_bytes(&r.raw_msg).map_err(|e| {
            println!("we detected a message type of {}, but got an error saying {}. are you asking for the correct message type?", r.message_type, &e);
            e
        })?;
        Ok(msg)
    }
    pub async fn write<T: ToMessageType>(&mut self, t: T) -> Result<()> {
        self.write_message_type(&t.to_message_type()).await
    }
}
