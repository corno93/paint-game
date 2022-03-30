// #![deny(warnings)]
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use futures_util::{SinkExt, StreamExt, TryFutureExt};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};
use warp::Filter;
// extern crate redis;
use redis::aio;
use redis::streams::{StreamId, StreamKey, StreamReadOptions, StreamReadReply};
use redis::RedisResult;
use redis::{from_redis_value, AsyncCommands};

use std::env;
use std::fmt::Debug;

/// Our global unique user id counter.
static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

/// Our state of currently connected users.
///
/// - Key is their id
/// - Value is a sender of `warp::ws::Message`
type Users = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Message>>>>;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
struct MouseEvent {
    x: u16,
    y: u16,
}

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

async fn write_to_stream(conn: &mut redis::aio::ConnectionManager, data: MouseEvent, user_id: u16) {
    let _: String = conn
        .xadd(
            STREAM_NAME,
            "*",
            &[("x", data.x), ("y", data.y), ("user_id", user_id)],
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

    // Keep track of all connected users, key is usize, value
    // is a websocket sender.
    let users = Users::default();
    // Turn our "state" into a new Filter...
    let users = warp::any().map(move || users.clone());

    let chat = warp::path("chat")
        .and(warp::ws())
        .and(users)
        .map(|ws: warp::ws::Ws, users| {
            // This will call our function if the handshake succeeds.
            ws.on_upgrade(move |socket| user_connected(socket, users))
        });

    // GET / -> index html
    let index = warp::path("static").and(warp::fs::dir("static"));

    let routes = index.or(chat);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

async fn user_connected(ws: WebSocket, users: Users) {
    let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);
    info!("new user: {}", my_id);

    let (mut user_ws_tx, mut user_ws_rx) = ws.split();

    // if your a new user, send back the entire stream
    let mut conn_manager = connect().await;
    let data = read_entire_stream(&mut conn_manager).await;
    let h = 5;

    for StreamKey { key, ids } in data.keys {
        for StreamId { id, map: zz } in ids {
            let r_x: RedisResult<String> = from_redis_value(&zz.get("x").unwrap());
            let x = r_x.unwrap();
            let r_y: RedisResult<String> = from_redis_value(&zz.get("y").unwrap());
            let y = r_y.unwrap();
            let r_user_id: RedisResult<String> = from_redis_value(&zz.get("user_id").unwrap());
            let user_id_from_stream = r_user_id.unwrap();

            let yy = user_id_from_stream.parse::<u16>().unwrap();

            debug!(
                "INITIAL READING: My user {:?}. Stream user {:?}, x {:?}, y {:?}",
                my_id, user_id_from_stream, x, y
            );

            let response = format!("{{\"x\": {}, \"y\": {}}}", x, y);
            user_ws_tx
                .send(Message::text(response))
                .unwrap_or_else(|e| {
                    error!("websocket send error: {}", e);
                })
                .await;
        }

        // TODO: do we need to acknowledge each stream and message ID
        //       once all messages are correctly processed
    }

    // everytime we hear data from redis that is not ours, send back to user_ws_tx
    tokio::task::spawn(async move {
        let mut listen_conn_manager = connect().await;
        loop {
            let data = blocking_read_from_stream(&mut listen_conn_manager).await;

            for StreamKey { key, ids } in data.keys {
                for StreamId { id, map: zz } in ids {
                    let r_x: RedisResult<String> = from_redis_value(&zz.get("x").unwrap());
                    let x = r_x.unwrap();
                    let r_y: RedisResult<String> = from_redis_value(&zz.get("y").unwrap());
                    let y = r_y.unwrap();
                    let r_user_id: RedisResult<String> =
                        from_redis_value(&zz.get("user_id").unwrap());
                    let user_id_from_stream = r_user_id.unwrap();

                    let yy = user_id_from_stream.parse::<u16>().unwrap();

                    debug!(
                        "READING FROM STREAM: My user {:?}. Stream user {:?}, x {:?}, y {:?}",
                        my_id, user_id_from_stream, x, y
                    );

                    // only send back other user's data
                    if yy != my_id as u16 {
                        let response = format!("{{\"x\": {}, \"y\": {}}}", x, y);
                        user_ws_tx
                            .send(Message::text(response))
                            .unwrap_or_else(|e| {
                                error!("websocket send error: {}", e);
                            })
                            .await;
                    }
                }

                // TODO: do we need to acknowledge each stream and message ID
                //       once all messages are correctly processed
            }
        }
    });

    // everytime we receive data lets append it to our redis stream
    while let Some(result) = user_ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                error!("websocket error(uid={}): {}", my_id, e);
                break;
            }
        };
        let msg = if let Ok(s) = msg.to_str() {
            s
        } else {
            return;
        };

        // we dont actually use mouse_event but this acts as a validation step
        let mouse_event: MouseEvent = serde_json::from_str(msg).unwrap();
        debug!("WRITING: User {:?} Mouse event {:?}", my_id, mouse_event);

        // save to the redis stream
        write_to_stream(&mut conn_manager, mouse_event, my_id as u16).await;
    }

    // if the user ever disconnects, the above while let will break, thus executing
    // this code below
    user_disconnected(my_id, &users).await;
}

async fn user_disconnected(my_id: usize, users: &Users) {
    info!("good bye user: {}", my_id);

    // Stream closed up, so remove from the user list
    users.write().await.remove(&my_id);
}
