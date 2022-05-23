// #![deny(warnings)]
use std::collections::HashMap;
use std::env;
use std::fmt::Debug;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use futures_util::{SinkExt, StreamExt, TryFutureExt};
use futures_util::stream::{SplitSink, SplitStream};
use log::{debug, error, info};
use redis::{AsyncCommands, from_redis_value};
use redis::aio;
use redis::aio::ConnectionManager;
use redis::RedisResult;
use redis::streams::{StreamId, StreamKey, StreamReadOptions, StreamReadReply};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;
use warp::Filter;
use warp::ws::{Message, WebSocket};


/// Our state of currently connected users.
///
/// - Key is their id
/// - Value is a sender of `warp::ws::Message`
type Users = Arc<RwLock<HashMap<u128, mpsc::UnboundedSender<Message>>>>;


const STREAM_NAME: &str = "paint-game";

async fn connect() -> redis::aio::ConnectionManager {
    //format - host:port
    // let redis_host_name =
    //     env::var("REDIS_HOSTNAME").expect("missing environment variable REDIS_HOSTNAME");

    let redis_password = env::var("REDIS_PASSWORD").unwrap_or_default();
    //if Redis server needs secure connection
    let uri_scheme = match env::var("IS_TLS") {
        Ok(_) => "rediss",
        Err(_) => "redis",
    };
    let redis_conn_url = format!("{}://:{}@localhost:6379", uri_scheme, redis_password);
    redis::Client::open(redis_conn_url)
        .expect("Invalid connection URL")
        .get_tokio_connection_manager()
        .await
        .expect("failed to connect to Redis")
}

async fn write_to_stream(conn: &mut redis::aio::ConnectionManager, user_id: Uuid, data: String) {
    // TODO: remove this variable declation?
    let _: String = conn
        .xadd(
            STREAM_NAME,
            "*",
            &[("user_id", user_id.to_string()), ("data", data)],
        )
        .await
        .unwrap();
}

// Here we block the stream and only read new values
// cmd: XREAD BLOCK 0 STREAMS paint-game $
async fn blocking_read_from_stream(conn: &mut redis::aio::ConnectionManager) -> StreamReadReply {
    let reply: StreamReadReply = conn
        .xread_options(
            &[STREAM_NAME],
            &["$"],
            &StreamReadOptions::default().noack().block(0),
        )
        .await
        .unwrap();
    reply
}

// Here we read the entire stream
// cmd: XREAD STREAMS paint-game 0
async fn read_entire_stream(conn: &mut redis::aio::ConnectionManager) -> StreamReadReply {
    let reply: StreamReadReply = conn.xread(&[STREAM_NAME], &[0]).await.unwrap();
    reply
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let users = Users::default();
    let chat = warp::path("game")
        // add users as a filter
        .and(warp::any().map(move|| users.clone()))
        // add websocket filter
        .and(warp::ws())
        .map(|users: Users, ws: warp::ws::Ws| {
            ws.on_upgrade(move | socket| user_connected(users, socket))
        });

    let index = warp::path("static")
        .and(warp::fs::dir("static"));

    warp::serve(index.or(chat)).run(([127, 0, 0, 1], 3030)).await;
}



async fn initial_dump(conn_manager: &mut ConnectionManager,
                      users: &Users,
                      my_id: Uuid) {
    /// Read the entire redis stream and send each entry back on the websocket

    debug!("Initial dump for user_id {:?}", my_id);

    let stream_data = read_entire_stream(conn_manager).await;
    for StreamKey { key, ids } in stream_data.keys {
        for StreamId { id, map: zz } in ids {
            // TODO: dont use unwrap
            let r_data: RedisResult<String> = from_redis_value(&zz.get("data").unwrap());
            let data = r_data.unwrap();

            // TODO: understand why i cant do users.read().await.get(user_id)... damn Rust...
            for (&user_id, tx) in users.read().await.iter() {
                if my_id.to_u128_le() == user_id{
                    if let Err(_disconnected) = tx.send(Message::text(&data)) {
                        // The tx is disconnected, our `user_disconnected` code
                        // should be happening in another task, nothing more to
                        // do here.
                    }
                    break;
                }
            }
        }
    }
}

async fn user_connected(users: Users, ws: WebSocket) {
    /// For every new user
    ///     Create a user_id
    ///     Send back all data points from redis to the user
    ///     Spawn tokio task to read data from stream and send back on ws connection

    let user_id = Uuid::new_v4();
    debug!("New user_id {:?}", user_id);

    // Use an unbounded channel to handle buffering and flushing of messages to the websocket.
    // Additionally this is convenient since user_ws_rx (SplitStream<WebSocket>) implements the
    // drop trait and therefore can never be cloned, so here we can use tx (UnboundedSender<Message>)
    // to as a reference for each user.
    let (tx, rx) = mpsc::unbounded_channel();
    let mut rx = UnboundedReceiverStream::new(rx);

    users.write().await.insert(user_id.to_u128_le(), tx);

    // debug!("All users:");
    // for (&user_id, tx) in users.read().await.iter() {
    //     debug!("{:?}", &user_id);
    // }

    let (mut user_ws_tx, mut user_ws_rx) = ws.split();
    let mut conn_manager = connect().await;

    tokio::task::spawn(async move {
        while let Some(message) = rx.next().await {
            user_ws_tx
                .send(message)
                .unwrap_or_else(|e| {
                    error!("websocket send error: {}", e);
                })
                .await;
        }
    });

    initial_dump(&mut conn_manager, &users, user_id).await;

    handle_incoming_data(user_ws_rx, &mut conn_manager, &users, user_id).await;

    handle_disconnecting(&users, user_id).await;

}

async fn handle_disconnecting(users:  &Users, my_id: Uuid) {
    /// Code run after a user disconnects their ws connection
    debug!("Good bye user_id {:?}", my_id);
    users.write().await.remove(&my_id.to_u128_le());
}

async fn handle_incoming_data(mut user_ws_rx: SplitStream<WebSocket>,
                              conn_manager: &mut ConnectionManager,
                              users: &Users,
                              my_id: Uuid){
    /// When we receive data from the ws we store it on the redis stream and send it on the
    /// UnboundedReceiver
    while let Some(result) = user_ws_rx.next().await {
        let message: Message = match result {
            Ok(msg) => msg,
            Err(e) => {
                error!("websocket error from my_id {:?}: {:?}", my_id, e);
                break;
            }
        };
        let user_data = if let Ok(s) = message.to_str() {
            s
        } else {
            return;
        };

        debug!("my_id {:?} sent the following data {:?}", my_id, user_data);

        // send the data to all users except the my_id
        for (&user_id, tx) in users.read().await.iter() {
            if my_id.to_u128_le() != user_id {
                if let Err(_disconnected) = tx.send(Message::text(user_data)) {
                    // The tx is disconnected, our `user_disconnected` code
                    // should be happening in another task, nothing more to
                    // do here.
                }
            }
        }
        write_to_stream(conn_manager, my_id, user_data.to_string()).await;
    }
}
