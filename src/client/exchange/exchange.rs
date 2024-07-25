use anyhow::{Error, Result};



use crate::{
    client::Connected,
    protocol::{File, Piece,  PieceExchange, PieceExchangeAck},
    Client,
};

pub struct Exchange<S: ExchangeState> {
    pub inner: Client<Connected>,
    pub state: S,
}

pub trait ExchangeState {}
pub struct Ready {
    // pub peers: HashSet<String>,
    // pub mypkg: MyPkg,
    // pub ack: MyPkgAck,
}
impl ExchangeState for Ready {}

impl Exchange<Ready> {
    pub async fn exchange(&mut self, pieces: [u64; 2], file: File) -> Result<(), Error> {
        let pe = PieceExchange { pieces, file };
        self.inner.write(pe).await?;
        // self.inner.write_message_type(&MessageType::PieceExchange(pe)).await?;
        let pa: PieceExchangeAck = self.inner.read().await?;
        dbg!(pa);
        Ok(())
    }
    pub async fn send(&mut self, piece: Piece) -> Result<(), Error> {
        self.inner.write(piece).await
        // self.inner
        //     .write_message_type(&MessageType::Piece(piece))
        //     .await
    }
}
