use actix::clock::Instant;
use actix_web::web::Data;
use actix_web_actors::ws::{ProtocolError, WebsocketContext};

use std::collections::HashMap;
use uuid::Uuid;

use crate::websocket::Ws;

pub type Rooms = HashMap<Uuid, Vec<WebsocketContext<Ws>>>;

pub struct WsConnection {
    room: Uuid,
    heartbeat: Instant,
    context: WebsocketContext<Ws>,
    id: Uuid,
}

impl WsConnection {
    pub fn new_room(room: Uuid, context: WebsocketContext<Ws>) -> Self {
        Self {
            id: Uuid::new_v4(),
            room,
            context,
            heartbeat: Instant::now(),
        }
    }
}
