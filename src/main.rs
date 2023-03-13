use futures::SinkExt;
use std::{collections::HashMap, io::Error, net::SocketAddr, sync::Arc};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{broadcast, Mutex},
};

use futures_util::StreamExt;

#[derive(Debug, Clone)]
struct LobbyMessage {
    lobby: String,
    message: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let (tx, _rx) = broadcast::channel::<LobbyMessage>(10);

    let try_socket = TcpListener::bind("127.0.0.1:8383").await;
    let listener = try_socket.expect("Failed to bind");

    // Create a Mutex-protected HashMap to store a vector of connections against a lobby name
    let connections: Arc<Mutex<HashMap<String, Vec<SocketAddr>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let connections = connections.clone();

                // get the lobby name from the url
                let mut buf = [0; 1024];
                let n = stream.peek(&mut buf).await.unwrap();
                let path = String::from_utf8_lossy(&buf[..n]);
                let path = path.split(" ").nth(1).unwrap();
                let path = path.trim_start_matches("/").to_string();

                let path = path.clone();
                tokio::spawn(handle_connection(
                    stream,
                    tx.clone(),
                    tx.subscribe(),
                    connections,
                    path,
                ));
            }
            Err(e) => {
                println!("failed to accept connection: {}", e);
            }
        }
    }
}

async fn handle_connection<'a>(
    stream: TcpStream,
    tx: broadcast::Sender<LobbyMessage>,
    mut rx: broadcast::Receiver<LobbyMessage>,
    connections: Arc<Mutex<HashMap<String, Vec<SocketAddr>>>>,
    path: String,
) {
    // get the peer address
    let addr = stream
        .peer_addr()
        .expect("connected streams should have a peer address");

    println!("Path: {}", path);

    // add the lobby (path) to the connections if it doesnt exist
    {
        let path = path.clone();
        let mut connections = connections.lock().await;
        if !connections.contains_key(path.as_str()) {
            connections.insert(path.to_string(), Vec::new());
        }
        connections
            .get_mut(path.as_str())
            .expect("Incorrect lobby was created or not found")
            .push(addr);
        println!("Connection established: {} - List: {:?}", addr, connections);
    }

    // convert the TCPStream to a websocket
    let ws_stream = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws_stream) => ws_stream,
        Err(e) => {
            println!("Error during the websocket handshake: {}", e);
            return;
        }
    };

    let (mut write, mut read) = ws_stream.split();

    let path = Arc::new(path.to_string());

    // send message to broadcast channel
    let rx_task = tokio::spawn({
        let path = path.clone(); // clone the Arc
        async move {
            while let Some(msg) = read.next().await {
                let msg = msg.expect("Error during reading from websocket");
                tx.send(LobbyMessage {
                    lobby: path.to_string(),
                    message: msg.to_string(),
                })
                .expect("Error sending message to broadcast channel");
            }
        }
    });

    // read from broadcast channel and send to client
    let tx_task = tokio::spawn({
        let path = path.clone(); // clone the Arc
        async move {
            while let Ok(msg) = rx.recv().await {
                let tokio_msg = tokio_tungstenite::tungstenite::Message::Text(msg.message);

                // only recieve messages from the connected lobby (path)
                if msg.lobby != *path {
                    continue;
                }
                println!("Sending message {} to {}", tokio_msg, addr);
                let sent_msg = write.send(tokio_msg).await;

                match sent_msg {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error sending message to {}: {}", addr, e);

                        // remove the peer address from the vector of connections of the connected
                        // lobby when the stream is closed
                        let mut connections = connections.lock().await;
                        let lobby = connections
                            .iter()
                            .find(|(_, v)| v.contains(&addr))
                            .expect("Lobby not found")
                            .0
                            .to_string();
                        connections
                            .get_mut(&lobby)
                            .expect("Incorrect lobby was created or not found")
                            .retain(|&x| x != addr);

                        // if no connections are in lobby then remove the lobby
                        if connections.get(&lobby).unwrap().len() == 0 {
                            connections.remove(&lobby);
                        }
                        println!("Connections: {:?}", connections);

                        break;
                    }
                }
            }
        }
    });

    // remove the peer address from the connections when the stream is closed
    tokio::spawn(async move {
        tokio::select! {
            result = rx_task => {
                if let Err(e) = result {
                    println!("Error reading from websocket: {:?}", e);
                }
            }
            result = tx_task => {
                if let Err(e) = result {
                    println!("Error writing to websocket: {:?}", e);
                }
            }
        }

        println!("Connection closed: {}", addr);
    });
}
