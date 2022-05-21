// #![deny(warnings)]
use std::collections::HashMap;
use std::env;
use std::fmt::Debug;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use futures_util::{SinkExt, StreamExt, TryFutureExt};
use futures_util::stream::SplitSink;
use log::{debug, error, info};
use redis::{AsyncCommands, from_redis_value};
use redis::aio;
use redis::aio::ConnectionManager;
use redis::RedisResult;
use redis::streams::{StreamId, StreamKey, StreamReadOptions, StreamReadReply};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;
use warp::Filter;
use warp::ws::{Message, WebSocket};

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

async fn write_to_stream(conn: &mut redis::aio::ConnectionManager, user_id: Uuid, data: String) {
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

    let chat = warp::path("game")
        // add websocket filter
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| {
            ws.on_upgrade(move | socket| user_connected(socket))
        });

    let index = warp::path("static")
        .and(warp::fs::dir("static"));

    warp::serve(index.or(chat)).run(([127, 0, 0, 1], 3030)).await;
}



async fn initial_dump(conn_manager: &mut ConnectionManager,
                      mut user_ws_tx: SplitSink<WebSocket, Message>,
                      user_id: Uuid){

    debug!("Initial dump for user_id {:?}", user_id);

    let stream_data = read_entire_stream( conn_manager).await;
    for StreamKey { key, ids } in stream_data.keys {
        for StreamId { id, map: zz } in ids {
            let r_data: RedisResult<String> = from_redis_value(&zz.get("data").unwrap());
            let data = r_data.unwrap();
            let r_user_id: RedisResult<String> = from_redis_value(&zz.get("user_id").unwrap());
            let user_id = r_user_id.unwrap();

            debug!("Initial dump data from user_id {:?} is {:?}", user_id, data);

            user_ws_tx
                .send(Message::text(data))
                .unwrap_or_else(|e| {
                    error!("websocket send error for initial dump: {}", e);
                })
                .await;
        }
    }
}

async fn user_connected(ws: WebSocket) {
    /// For every new user
    ///     Create a user_id
    ///     Send back all data points
    ///

    let user_id = Uuid::new_v4();
    debug!("New user_id {:?}", user_id);

    let (mut user_ws_tx, mut user_ws_rx) = ws.split();

    let mut conn_manager = connect().await;

    initial_dump(&mut conn_manager, user_ws_tx, user_id).await;



    // let data = read_entire_stream(&mut conn_manager).await;
    //


    // everytime we hear data from redis that is not ours, send back to user_ws_tx
    // tokio::task::spawn(async move {
    //     let mut listen_conn_manager = connect().await;
    //     loop {
    //         let data = blocking_read_from_stream(&mut listen_conn_manager).await;
    //
    //         for StreamKey { key, ids } in data.keys {
    //             for StreamId { id, map: zz } in ids {
    //                 let r_x: RedisResult<String> = from_redis_value(&zz.get("x").unwrap());
    //                 let x = r_x.unwrap();
    //                 let r_y: RedisResult<String> = from_redis_value(&zz.get("y").unwrap());
    //                 let y = r_y.unwrap();
    //                 let r_user_id: RedisResult<String> =
    //                     from_redis_value(&zz.get("user_id").unwrap());
    //                 let user_id_from_stream = r_user_id.unwrap();
    //
    //                 let yy = user_id_from_stream.parse::<u16>().unwrap();
    //
    //                 debug!(
    //                     "READING FROM STREAM: My user {:?}. Stream user {:?}, x {:?}, y {:?}",
    //                     my_id, user_id_from_stream, x, y
    //                 );
    //
    //                 // only send back other user's data
    //                 if yy != my_id as u16 {
    //                     let response = format!("{{\"x\": {}, \"y\": {}}}", x, y);
    //                     user_ws_tx
    //                         .send(Message::text(response))
    //                         .unwrap_or_else(|e| {
    //                             error!("websocket send error: {}", e);
    //                         })
    //                         .await;
    //                 }
    //             }
    //
    //             // TODO: do we need to acknowledge each stream and message ID
    //             //       once all messages are correctly processed
    //         }
    //     }
    // });


    // everytime we receive data lets append it to our redis stream
    while let Some(result) = user_ws_rx.next().await {
        let message: Message = match result {
            Ok(msg) => msg,
            Err(e) => {
                // error!("websocket error(uid={}): {}", user_id, e);
                break;
            }
        };
        let user_data = if let Ok(s) = message.to_str() {
            s
        } else {
            return;
        };

        debug!("user_id {:?} sent the following data {:?}", user_id, user_data);

        write_to_stream(&mut conn_manager, user_id, user_data.to_string()).await;
    }

    // when the user disconnections this code will run
    debug!("Good bye user_id {:?}", user_id);

    // if the user ever disconnects, the above while let will break, thus executing
    // this code below
    // user_disconnected(my_id, &users).await;
}



async fn user_disconnected(my_id: usize, users: &Users) {
    info!("good bye user: {}", my_id);

    // Stream closed up, so remove from the user list
    users.write().await.remove(&my_id);
}
