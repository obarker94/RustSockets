// rewrite with tokio tungstenite

use futures::SinkExt;
use std::io::Error;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::broadcast,
};

use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let try_socket = TcpListener::bind("127.0.0.1:8383").await;
    let listener = try_socket.expect("Failed to bind");

    let (tx, _rx) = broadcast::channel::<String>(10);

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                tokio::spawn(handle_connection(stream, tx.clone(), tx.subscribe()));
            }
            Err(e) => {
                println!("failed to accept connection: {}", e);
            }
        }
    }
}

async fn handle_connection(
    stream: TcpStream,
    tx: broadcast::Sender<String>,
    mut rx: broadcast::Receiver<String>,
) {
    let addr = stream
        .peer_addr()
        .expect("connected streams should have a peer address");

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    let (mut write, mut read) = ws_stream.split();

    // send message to broadcast channel
    tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            let msg = msg.expect("Error during reading from websocket");
            tx.send(msg.to_string()).unwrap();
        }
    });

    // read from broadcast channel and send to client
    tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let msg = tokio_tungstenite::tungstenite::Message::Text(msg.to_string());
            println!("Sending message {} to {}", msg, addr);
            write.send(msg).await.unwrap();
            // want to write here to socket but write doesnt have any send methods?
        }
    });
}
