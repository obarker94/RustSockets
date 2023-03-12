use std::{
    collections::HashMap,
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread::spawn,
};

use tungstenite::{
    accept_hdr,
    handshake::server::{Request, Response},
    WebSocket,
};

fn main() {
    let server = TcpListener::bind("127.0.0.1:3012").unwrap();

    // create an Arc<Mutex<HashMap>> of lobbies with a key of the lobby name and a value of connected clients
    let lobbies: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));

    for stream in server.incoming() {
        let lobbies_clone = Arc::clone(&lobbies);
        spawn(move || {
            let callback = |req: &Request, response: Response| {
                println!("Received a new ws handshake");
                println!("The request's path is: {}", req.uri().path());

                // add lobby to list
                lobbies_clone
                    .lock()
                    .unwrap()
                    .insert(req.uri().path().to_string(), "test".to_string());

                Ok(response)
            };

            let mut websocket = accept_hdr(stream.unwrap(), callback).unwrap();

            loop {
                let msg = websocket.read_message().unwrap();
                println!("Lobby list {:?}", lobbies_clone.lock().unwrap());
                if msg.is_binary() || msg.is_text() {
                    websocket.write_message(msg).unwrap();
                }
            }
        });
    }
}
