use actix_web::{web, App, HttpServer};
use websocket::websocket_router;
mod rooms;
mod websocket;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().route("/ws/", web::get().to(websocket_router)))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
