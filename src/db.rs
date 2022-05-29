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

const STREAM_NAME: &str = "paint-game";

pub async fn connect() -> redis::aio::ConnectionManager {
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

/// Write to the stream (redis cmd: XADD paint-game * user_id <user_id> data <data>)
pub async fn write_to_stream(conn: &mut redis::aio::ConnectionManager, user_id: Uuid, data: String) {
    let _: String = conn
        .xadd(
            STREAM_NAME,
            "*",
            &[("user_id", user_id.to_string()), ("data", data)],
        )
        .await
        .unwrap();
}
//
// /// Block the stream and only read new values (redis cmd: XREAD BLOCK 0 STREAMS paint-game $)
// async fn blocking_read_from_stream(conn: &mut redis::aio::ConnectionManager) -> StreamReadReply {
//     let reply: StreamReadReply = conn
//         .xread_options(
//             &[STREAM_NAME],
//             &["$"],
//             &StreamReadOptions::default().noack().block(0),
//         )
//         .await
//         .unwrap();
//     reply
// }

/// Read the entire stream (redis cmd: XREAD STREAMS paint-game 0)
pub async fn read_entire_stream(conn: &mut redis::aio::ConnectionManager) -> StreamReadReply {
    let reply: StreamReadReply = conn.xread(&[STREAM_NAME], &[0]).await.unwrap();
    reply
}
