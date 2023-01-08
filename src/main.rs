use std::time::Instant;

use actix::{Actor, StreamHandler};
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use session::WsSession;

mod session;

/// Define HTTP actor

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;
}

/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                println!("PING");
                ctx.pong(&msg);
            }
            Ok(ws::Message::Text(text)) => {
                println!("TEXT: {}", text);

                ctx.text(text);
            }
            Ok(ws::Message::Binary(bin)) => {
                ctx.binary(bin);
            }
            _ => (),
        }
    }
}

async fn index(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    let resp = ws::start(WsSession {}, &req, stream);
    resp
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().route("/ws/", web::get().to(index)))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}


