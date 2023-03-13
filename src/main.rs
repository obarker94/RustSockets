use futures::SinkExt;
use std::{
    collections::{HashMap, HashSet},
    io::Error,
    net::SocketAddr,
    sync::Arc,
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{broadcast, Mutex},
};

use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let (tx, _rx) = broadcast::channel::<String>(10);

    let try_socket = TcpListener::bind("127.0.0.1:8383").await;
    let listener = try_socket.expect("Failed to bind");

    // Create a Mutex-protected HashSet to store the connections
    // let connections: Arc<Mutex<HashSet<SocketAddr>>> = Arc::new(Mutex::new(HashSet::new()));

    // Create a Mutex-protected HashMap to store a vector of connections against a lobby name
    let connections: Arc<Mutex<HashMap<String, Vec<SocketAddr>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let connections = connections.clone();
                tokio::spawn(handle_connection(
                    stream,
                    tx.clone(),
                    tx.subscribe(),
                    connections,
                ));
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
    connections: Arc<Mutex<HashMap<String, Vec<SocketAddr>>>>,
) {
    // get the peer address
    let addr = stream
        .peer_addr()
        .expect("connected streams should have a peer address");

    // get the lobby name from the url
    let mut buf = [0; 1024];
    let n = stream.peek(&mut buf).await.unwrap();
    let path = String::from_utf8_lossy(&buf[..n]);
    let path = path.split(" ").nth(1).unwrap();
    let path = path.trim_start_matches("/");
    println!("Path: {}", path);

    // add the lobby (path) to the connections if it doesnt exist
    {
        let mut connections = connections.lock().await;
        if !connections.contains_key(path) {
            connections.insert(path.to_string(), Vec::new());
        }
        connections
            .get_mut(path)
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

    // send message to broadcast channel
    let rx_task = tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            let msg = msg.expect("Error during reading from websocket");
            tx.send(msg.to_string()).unwrap();
        }
    });

    // read from broadcast channel and send to client
    let tx_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let msg = tokio_tungstenite::tungstenite::Message::Text(msg.to_string());
            println!("Sending message {} to {}", msg, addr);
            let sent_msg = write.send(msg).await;
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
