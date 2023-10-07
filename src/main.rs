use std::net::TcpListener;
use zerotoprod::run;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let listener = TcpListener::bind("127.0.0.1:8000").expect("Failed to bind port");
    run(listener)?.await.map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
}