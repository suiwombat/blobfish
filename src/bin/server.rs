use std::collections::HashSet;



use blobfish::Server;



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Server::new("127.0.0.1:8080".into(), HashSet::new())
        .await?
        .serve()
        .await?;
    Ok(())
}
