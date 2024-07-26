use blobfish::Server;
use std::collections::HashSet;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    let token = CancellationToken::new();
    Server::new(listener, HashSet::new())
        .await?
        .serve(token.clone())
        .await?;
    Ok(())
}
