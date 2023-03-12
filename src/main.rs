use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
};

#[tokio::main]
async fn main() {
    println!("Connection starting!");
    // create a TCP Listener and bind to an address
    // we will await this until it is read to accept connection.
    // unwrap() will panic if it cannot bind - i.e. the port is aleady in use.
    let listener = TcpListener::bind("127.0.0.1:1338").await.unwrap();

    loop {
        // we destructure the socket and address from the listener after the connection is accepted.
        // the connection may not be accepted if the listener is not ready to accept connections.
        let (mut socket, _addr) = listener.accept().await.unwrap();

        tokio::spawn(async move {
            // split the socket into a reader and writer.
            let (reader, mut writer) = socket.split();

            // we create a buffer to store the incoming data.
            let mut reader = BufReader::new(reader);
            let mut line = String::new();

            println!("Connection established! {}", _addr);

            loop {
                let bytes_read = reader.read_line(&mut line).await.unwrap();
                if bytes_read == 0 {
                    break;
                }

                // we write the whole buffer to the socket and unwrap the result. it could panic if the socket is not ready to write.
                //
                // i.e. outgoing
                writer.write_all(line.as_bytes()).await.unwrap();
                line.clear();
            }
        });
    }
}
