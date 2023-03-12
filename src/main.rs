use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::broadcast,
};

#[tokio::main]
async fn main() {
    println!("Connection starting!");
    // create a TCP Listener and bind to an address
    // we will await this until it is read to accept connection.
    // unwrap() will panic if it cannot bind - i.e. the port is aleady in use.
    let listener = TcpListener::bind("127.0.0.1:1338").await.unwrap();

    // create a communication channel to send messages between clients and the server.
    let (tx, _rx) = broadcast::channel(10);

    loop {
        // we destructure the socket and address from the listener after the connection is accepted.
        // the connection may not be accepted if the listener is not ready to accept connections.
        let (mut socket, addr) = listener.accept().await.unwrap();

        let tx = tx.clone();
        let mut rx = tx.subscribe();

        tokio::spawn(async move {
            // split the socket into a reader and writer.
            let (reader, mut writer) = socket.split();

            // we create a buffer to store the incoming data.
            let mut reader = BufReader::new(reader);
            let mut line = String::new();

            println!("Connection established! {}", addr);

            loop {
                tokio::select! {
                result = reader.read_line(&mut line) => {

                if result.unwrap() == 0 {
                    break;
                }
                tx.send((line.clone(), addr)).unwrap();
                line.clear();
                }
                result = rx.recv() => {
                    let (msg, other_addr) = result.unwrap();

                    if addr != other_addr {
                        writer.write_all(msg.as_bytes()).await.unwrap();
                    }
                }
                }
            }
        });
    }
}
