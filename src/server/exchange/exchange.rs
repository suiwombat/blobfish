use anyhow::{bail, Error, Result};
use std::{collections::HashSet, io::Read};

use crate::{
    protocol::{
        File, MessageType, MyPkg, MyPkgAck, Piece, PieceAck, PieceExchange, PieceExchangeAck,
    },
    server::Connected,
    Server,
};

pub struct Exchange<S: ExchangeState> {
    pub inner: Server<Connected>,
    pub state: S,
}

pub trait ExchangeState {}
pub struct Ready {
    pub peers: HashSet<String>,
    pub mypkg: MyPkg,
    pub ack: MyPkgAck,
}
pub struct Running {
    pub pieces: [u64; 2],
    pub file: File,
    pub last_state: Ready,
}
impl ExchangeState for Ready {}
impl ExchangeState for Running {}

impl Exchange<Ready> {
    pub async fn exchange(mut self) -> Result<()> {
        //Result<Self, Error> {
        for file in self.state.mypkg.clone().files {
            let pe: PieceExchange = self.inner.read().await?;
            // TODO use pe to divine if we can ask for pieces instead of all pieces
            // we would need a cache smart enough to know bitfields and pieces received
            // the cache on Server<Connected> would know, but it only holds HasSet<String>
            // whihc is just md5sums.  future spot is there
            let pa = PieceExchangeAck { pieces: None };
            self.inner.write(pa).await?;

            let [start, end] = pe.pieces;
            self.receive([start, end], file).await?;
        }
        Ok(())
        // Ok(self)
        // Ok(Exchange {
        //     inner: self.inner,
        //     state: Running {
        //         pieces: pe.pieces,
        //         file: pe.file,
        //         last_state: self.state,
        //     },
        // })
    }
    async fn receive(&mut self, pieces: [u64; 2], file: File) -> Result<()> {
        let mut contigious = 0;
        let [start, end] = pieces;
        let filename = file.filename();
        println!("looping from {};{} for file {}", start, end, &filename);
        let mut write_at = file.write_at(filename)?;
        for i in start..end {
            let p: Piece = self.inner.read().await?;
            // println!("piece received {}", &p.piece);
            // if p.Piece < start || p.Piece > end+1 {
            //     return nil, fmt.Errorf("received an unexpected piece from our peer; piece %v [%v,%v]", p.Piece, start, end)
            // }
            if p.piece < start || p.piece > end {
                bail!(
                    "piece is out of bounds {} is not within {}:{}",
                    p.piece,
                    start,
                    end
                );
            }
            write_at(p.piece, p.data.as_slice())?;
            if p.piece == 0 || p.piece - 1 == contigious {
                contigious = p.piece;
            }
            if let Some(ack) = p.ack {
                println!("ack requested for {} sending {}", &ack, &contigious);
                // if *p.Ack != contiguousPiece {
                //     delta := contiguousPiece - *p.Ack
                //     slog.Error("out of sync in pieceExchage", "delta", delta)
                // }
                let pa = PieceAck { piece: contigious };
                self.inner.write(pa).await?;
                // self.inner.write(&MessageType::PieceAck(pa)).await?;
            }
        }
        Ok(())
    }
}

impl Exchange<Running> {
    // pub async fn receive(mut self) -> Result<Exchange<Ready>> {
    //     let mut contigious = 0;
    //     println!(
    //         "looping from {};{} for file {}",
    //         &self.state.pieces[0], &self.state.pieces[1], self.state.file.path
    //     );
    //     let [start, end] = self.state.pieces;
    //     for i in start..end {
    //         let p: Piece = self.inner.read().await?;
    //         println!("piece received {}", &p.piece);
    //         // if p.Piece < start || p.Piece > end+1 {
    //         //     return nil, fmt.Errorf("received an unexpected piece from our peer; piece %v [%v,%v]", p.Piece, start, end)
    //         // }
    //         if p.piece < start || p.piece > end {
    //             bail!(
    //                 "piece is out of bounds {} is not within {}:{}",
    //                 p.piece,
    //                 start,
    //                 end
    //             );
    //         }
    //         if p.piece == 0 || p.piece - 1 == contigious {
    //             contigious = p.piece;
    //         }
    //         if let Some(ack) = p.ack {
    //             println!("ack requested for {} sending {}", &ack, &contigious);
    //             // if *p.Ack != contiguousPiece {
    //             //     delta := contiguousPiece - *p.Ack
    //             //     slog.Error("out of sync in pieceExchage", "delta", delta)
    //             // }
    //             let pa = PieceAck { piece: contigious };
    //             // self.inner.write(&pa).await?;
    //             self.inner.write_message_type(MessageType::PieceAck(pa)).await?;
    //         }
    //     }
    //     Ok(Exchange {
    //         inner: self.inner,
    //         state: self.state.last_state,
    //     })
    // }
}
