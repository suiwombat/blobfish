use crate::{
    client_args::Cli,
    protocol::{MyPkg, MyPkgAccepted, BLOCK_SIZE},
};
use anyhow::{bail, Result};
use futures::future::BoxFuture;
use std::borrow::BorrowMut;
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

pub struct Client<S: ClientState> {
    // state is our marker
    state: Box<ActualClientState>,
    // conn is available on all states and should be exposed as &mut self for read/writes
    conn: TcpStream,
    // extra is a generic field for use within different states to squirrel data
    extra: S,
}

pub trait ClientState {}
pub struct Connected;
pub struct Closed;

impl ClientState for Connected {}
// impl ClientState for OfferMyPkg {}
// impl ClientState for SendMyPkg {}
// impl ClientState for SendMyPkgPartially {}
impl ClientState for Closed {}

impl Client<Closed> {
    pub async fn open(addr: String) -> Result<Client<Connected>> {
        Ok(Client {
            conn: TcpStream::connect(addr).await?,
            state: Connected::Initial,
        })
    }
}

impl Client<Connected> {
    pub fn upload_mypkg(self, mypkg: MyPkg) -> Client<Connected> {
        Client {
            conn: self.conn,
            state: Connected::OfferMyPkg(OfferMyPkg { mypkg }),
        }
    }

    pub async fn close(&mut self) -> Result<()> {
        self.conn.shutdown().await.map_err(anyhow::Error::from)
    }

    pub fn state(self) -> Connected {
        self.state
    }
}

// impl Client<OfferMyPkg> {
//     pub async fn handle<'a>(&self, mut state: HandlerState<'a>) {
//         // let arc_self = Arc::new(self);
//         // println!("{}", session.data);
//         while let Some(next) = state.0(self).await {
//             state = next
//         }
//         // println!("{}", session.data);
//     }
// }

impl Client<Connected> {
    pub async fn send(mut self) -> Result<Client<Connected>> {
        match self.state {
            Connected::OfferMyPkg(offer) => {
                let b = serde_bencode::to_bytes(&offer.mypkg)?;
                for chunk in b.chunks(BLOCK_SIZE) {
                    self.conn.write_all(chunk).await?;
                }

                let mut buf = [0; BLOCK_SIZE];
                let mut raw_msg = vec![];
                // listen for a response
                loop {
                    let n = self.conn.read(&mut buf).await?;
                    raw_msg.extend_from_slice(&buf[..n]);
                    if n == 0 || n < BLOCK_SIZE {
                        break;
                    }
                }

                let msg: MyPkgAccepted =
                    serde_bencode::from_bytes(&raw_msg).map_err(|e| dbg!(e))?;
                Ok(Client {
                    conn: self.conn,
                    state: Connected::SendMyPkg(SendMyPkg { mypkg: offer.mypkg }),
                })
            }
            _ => bail!("wat"),
        }
    }
}

// impl Client<OfferMyPkg> {}
pub struct Session {
    pub data: String,
}

pub struct HandlerState<'a>(pub fn(&Client<Connected>) -> BoxFuture<'a, Option<HandlerState<'a>>>);
