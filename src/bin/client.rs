use anyhow::{bail, Error};
use blobfish::{
    client::Offer,
    client_args::{Cli, Commands},
    protocol::{MyPkg, Piece, BLOCK_SIZE},
    // client::{Client, Connected, HandlerState, Session},
    Client,
};
use clap::Parser;
use serde_bytes::ByteBuf;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Cli::parse();

    match args.command {
        Commands::Upload { name, file } => {
            let mypkg = MyPkg::new(name, file).unwrap();
            // let conn = Client::open(args.connect_to).await?;
            let mut state = Offer::new(Client::open(args.connect_to).await?)
                .offer(mypkg.clone())
                .await?
                .add_peers(vec!["127.0.0.1:2040".into()])
                .negotiate()
                .await?;
            // tricky, how do i do multiple files? i need a loop or something
            let mut buf: [u8; BLOCK_SIZE] = [0; BLOCK_SIZE];
            for file in mypkg.files {
                dbg!(&file);
                let piece_count = file.clone().chunk_count();
                println!("send file {} piece_count {}", file.path, piece_count);
                state
                    .exchange([0, piece_count as u64], file.clone())
                    .await?;
                let read_at = file.read_at()?;
                for piece in 0..piece_count {
                    read_at(piece as u64, &mut buf)?;
                    // println!("{}", piece);
                    let p = Piece {
                        piece: piece as u64,
                        ack: None,
                        data: ByteBuf::from(&buf),
                    };
                    state.send(p).await?
                }
            }
            // tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            Ok(())
        }
        Commands::Download { file } => Ok(()),
        Commands::List => Ok(()),
    }
}
