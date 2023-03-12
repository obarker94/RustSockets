use std::{
    collections::HashMap,
    net::{SocketAddr, TcpListener},
    sync::{Arc, Mutex},
    thread::spawn,
};

use tungstenite::{
    accept_hdr,
    handshake::server::{Request, Response},
};

fn main() {
    let server = TcpListener::bind("127.0.0.1:3012").unwrap();

    let lobbies: Arc<Mutex<HashMap<String, Vec<SocketAddr>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    for stream in server.incoming() {
        let lobbies_clone = Arc::clone(&lobbies);
        spawn(move || {
            let mut lobby_name = String::new();

            let callback = |req: &Request, response: Response| {
                println!("Recieved - Ws Handshake @ path: {}", req.uri().path());

                lobby_name = req.uri().path().to_string();

                Ok(response)
            };

            let websocket = accept_hdr(stream.unwrap(), callback);

            match websocket {
                Ok(mut websocket) => {
                    let peer_addr = websocket.get_mut().peer_addr();

                    match peer_addr {
                        Ok(peer_addr) => {
                            let clone_name = lobby_name.clone();
                            lobbies_clone
                                .lock()
                                .unwrap()
                                .entry(clone_name)
                                .or_insert_with(Vec::new)
                                .push(peer_addr);

                            println!("Client connected @ {}", peer_addr);

                            loop {
                                let msg = websocket.read_message();
                                println!("Lobby list {:?}", lobbies_clone.lock().unwrap());
                                match msg {
                                    Ok(msg) => {
                                        if msg.is_binary() || msg.is_text() {
                                            websocket.write_message(msg).unwrap();
                                        }
                                    }
                                    Err(_) => {
                                        println!("Client disconnected @ {}", peer_addr);
                                        let mut lobbies = lobbies_clone.lock().unwrap();
                                        if let Some(connections) = lobbies.get_mut(&lobby_name) {
                                            connections.retain(|&x| x != peer_addr);
                                            if connections.is_empty() {
                                                lobbies.remove(&lobby_name);
                                            }
                                        }
                                        println!("Lobby list {:?}", *lobbies);
                                        break;
                                    }
                                }

                                // if client disconnected remove them from the lobby
                            }
                        }
                        Err(_) => {
                            println!("Failed to get peer address");
                            return;
                        }
                    };
                }
                Err(_) => println!("Websocket handshake failed"),
            }
        });
    }
}
