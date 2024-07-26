pub mod client;
pub mod client_args;
pub mod protocol;
pub mod server;
pub mod upload;

// re-export from sub-crates
pub use client::client::Client;
pub use server::server::Server;

#[cfg(test)]
mod tests {
    use super::*;

    use anyhow::Error;
    use protocol::hash_file;
    use serde_bytes::ByteBuf;
    use std::collections::HashSet;
    use tokio::net::TcpListener;
    use tokio_util::sync::CancellationToken;

    use crate::{
        client::Offer,
        client_args::Commands,
        protocol::{MyPkg, Piece, BLOCK_SIZE},
        Client, Server,
    };

    #[tokio::test]
    async fn test_end_to_end() -> Result<(), Error> {
        async fn client(name: String, file: Vec<String>, server_addr: String) -> Result<(), Error> {
            println!("connect to {}", &server_addr);
            let mypkg = MyPkg::new(name, file)?;
            let mut state = Offer::new(Client::open(server_addr).await?)
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
                    let p = Piece {
                        piece: piece as u64,
                        ack: None,
                        data: ByteBuf::from(buf),
                    };
                    state.send(p).await?
                }
            }
            Ok(())
        }

        async fn server(
            server_addr: TcpListener,
            ctx: CancellationToken,
        ) -> Result<(), Box<dyn std::error::Error>> {
            println!(
                "listen on {}",
                &server_addr.local_addr().unwrap().to_string()
            );
            Server::new(server_addr, HashSet::new())
                .await?
                .serve(ctx)
                .await?;
            Ok(())
        }

        let server_listener = TcpListener::bind("127.0.0.1:8080").await?;
        let server_addr = server_listener.local_addr().unwrap().clone();
        let ctx = CancellationToken::new();
        let server_ctx = ctx.clone();

        let server_handle =
            tokio::spawn(async move { server(server_listener, server_ctx).await.unwrap() });

        let name = "test_end_to_end".into();
        let crushingit = "src/fixtures/crushingit.gif";
        let wombatchew = "src/fixtures/wombatchew.gif";
        let file = vec![crushingit.into(), wombatchew.into()];
        let client_handle =
            tokio::spawn(async move { client(name, file, server_addr.to_string()).await.unwrap() });
        client_handle.await?;
        ctx.cancel();
        server_handle.await?;

        let original_crushingit = hash_file(crushingit)?;
        let original_wombatchew = hash_file(wombatchew)?;
        let blobfish_crushingit = hash_file(&format!(
            "data/{}/{}",
            &original_crushingit.md5sum,
            original_crushingit.filename()
        ))?;
        let blobfish_wombatchew = hash_file(&format!(
            "data/{}/{}",
            &original_wombatchew.md5sum,
            original_wombatchew.filename()
        ))?;

        assert_eq!(original_crushingit.md5sum, blobfish_crushingit.md5sum);
        assert_eq!(original_wombatchew.md5sum, blobfish_wombatchew.md5sum);
        Ok(())
    }
}
