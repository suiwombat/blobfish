use std::collections::HashSet;

use anyhow::Error;
use blobfish::protocol::{MyPkg, MyPkgAck, NegotiateMyPkg, NegotiateMyPkgAck, BLOCK_SIZE};
use blobfish::Server;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Server::new("127.0.0.1:8080".into(), HashSet::new())
        .await?
        .serve()
        .await?;
    Ok(())
}
