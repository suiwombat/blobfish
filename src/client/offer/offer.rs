use std::{borrow::BorrowMut, collections::HashSet, ops::Deref};

use crate::{
    client::exchange::{Exchange, Ready},
    client::Connected,
    protocol::{MyPkg, MyPkgAck,MessageType,  NegotiateMyPkg, NegotiateMyPkgAck},
    Client,
};
use anyhow::{bail, Error};

pub struct Offer<S: OfferState> {
    inner: Client<Connected>,
    state: S,
}

pub trait OfferState {}
pub struct OfferMsg;
pub struct Negotiate {
    mypkg: MyPkg,
    ack: MyPkgAck,

    peers: HashSet<String>,
}

impl Negotiate {
    fn new(mypkg: MyPkg, ack: MyPkgAck) -> Self {
        Self {
            mypkg,
            ack,
            peers: HashSet::new(),
        }
    }
}

impl OfferState for OfferMsg {}
impl OfferState for Negotiate {}

impl Offer<OfferMsg> {
    pub fn new(client: Client<Connected>) -> Offer<OfferMsg> {
        Offer {
            inner: client,
            state: OfferMsg,
        }
    }
    pub async fn offer(mut self, mypkg: MyPkg) -> Result<Offer<Negotiate>, Error> {
        self.borrow_mut().inner.write(mypkg.clone()).await?;
        // self.borrow_mut().inner.write_message_type(&MessageType::MyPkg(mypkg.clone())).await?;
        let ack: MyPkgAck = self.borrow_mut().inner.read().await?;
        match ack {
            MyPkgAck {
                md5sum: None,
                files: None,
            } => {
                // peer is not interested, lets give up
                bail!("peer is not interested in {}", mypkg.md5sum)
            }
            _ => {
                // interested, prepare to send
                Ok(Offer {
                    inner: self.inner,
                    state: Negotiate::new(mypkg, ack),
                })
            }
        }
    }
}

impl Offer<Negotiate> {
    pub fn add_peers(mut self, peers: Vec<String>) -> Self {
        for peer in peers {
            self.state.peers.insert(peer);
        }
        self
    }
    pub fn peers(&self) -> Vec<String> {
        self.state.peers.iter().map(|v| v.to_owned()).collect()
    }
    pub async fn negotiate(mut self) -> Result<Exchange<Ready>, Error> {
        let msg = NegotiateMyPkg {
            md5sum: self.state.mypkg.md5sum.to_owned(),
        };
        self.borrow_mut()
            .inner
            .write(msg)
            // .write_message_type(&MessageType::NegotiateMyPkg(msg))
            .await
            .map_err(|e| dbg!(e))?;
        let resp: NegotiateMyPkgAck = self.borrow_mut().inner.read().await?;

        dbg!(&resp);
        if let Some(peers) = resp.peers {
            for peer in peers {
                self.state.peers.insert(peer);
            }
        }

        Ok(Exchange {
            inner: self.inner,
            state: Ready {
                // mypkg: self.state.mypkg,
                // ack: self.state.ack,
                // peers: self.state.peers,
            },
        })
    }
}
