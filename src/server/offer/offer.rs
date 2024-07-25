use std::{borrow::BorrowMut, collections::HashSet, ops::Deref};

use crate::{
    protocol::{MyPkg, MyPkgAck, NegotiateMyPkg, NegotiateMyPkgAck, MessageType},
    server::exchange::{Exchange, Ready},
    server::Connected,
    Server,
};
use anyhow::{bail, Error};
use std::sync::{Arc, RwLock};

pub struct Offer<S: OfferState> {
    inner: Server<Connected>,
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
    pub fn new(server: Server<Connected>) -> Offer<OfferMsg> {
        Offer {
            inner: server,
            state: OfferMsg,
        }
    }
    pub async fn wait_for_mypkg(mut self) -> Result<Offer<Negotiate>, Error> {
        let mypkg: MyPkg = self.borrow_mut().inner.read().await?;
        dbg!(&mypkg);
        let accept: MyPkgAck;
        if let Some(md5sum) = self.inner.get(mypkg.md5sum.to_owned()) {
            println!("cache hit {}", &mypkg.md5sum);
            accept = MyPkgAck {
                md5sum: Some(mypkg.md5sum.to_owned()),
                files: Some(vec![]),
            };
            self.borrow_mut().inner.write(accept.clone()).await?;
            // self.borrow_mut().inner.write_message_type(&MessageType::MyPkgAck(accept.clone())).await?;
        } else {
            self.inner.set(mypkg.md5sum.to_owned());
            println!("cache miss {}", &mypkg.md5sum);
            accept = MyPkgAck {
                md5sum: Some(mypkg.md5sum.to_owned()),
                files: None,
            };
            self.borrow_mut().inner.write(accept.clone()).await?;
            // self.borrow_mut().inner.write_message_type(&MessageType::MyPkgAck(accept.clone())).await?;
        }
        Ok(Offer {
            inner: self.inner,
            state: Negotiate::new(mypkg, accept),
        })
    }
}

impl Offer<Negotiate> {
    pub fn add_peers(mut self, peers: HashSet<String>) -> Self {
        for peer in peers {
            self.state.peers.insert(peer);
        }
        self
    }
    pub fn peers(&self) -> Vec<String> {
        self.state.peers.iter().map(|v| v.to_owned()).collect()
    }
    pub async fn negotiate(mut self) -> Result<Exchange<Ready>, Error> {
        let neg_msg: NegotiateMyPkg = self.borrow_mut().inner.read().await?;
        dbg!(&neg_msg);
        let neg_ack_msg = NegotiateMyPkgAck {
            md5sum: neg_msg.md5sum,
            peers: Some(self.peers()),
        };
        // self.borrow_mut().inner.write(&neg_ack_msg).await?;
        // self.borrow_mut().inner.write_message_type(&MessageType::NegotiateMyPkgAck(neg_ack_msg)).await?;
        self.borrow_mut().inner.write(neg_ack_msg).await?;
        Ok(Exchange {
            inner: self.inner,
            state: Ready {
                peers: self.state.peers,
                mypkg: self.state.mypkg,
                ack: self.state.ack,
            },
        })
    }
}
