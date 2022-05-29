use std::env;


use futures_util::{SinkExt, StreamExt, TryFutureExt};
use futures_util::stream::{SplitSink, SplitStream};
use log::{debug, error, info};
use redis::{AsyncCommands, from_redis_value};
use redis::aio;
use redis::aio::ConnectionManager;
use redis::RedisResult;
use redis::streams::{StreamId, StreamKey, StreamReadOptions, StreamReadReply};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;
use warp::Filter;
use warp::ws::{Message, WebSocket};
use crate::db;
use crate::db::Db;

use crate::types::Users;


pub fn connect_user_route(
    users: Users,
    db: db::Db
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
     warp::path("connect_user")
        // add users as a filter
        .and(warp::any().map(move|| users.clone()))
         // add db as a filter
        .and(warp::any().map(move|| db.clone()))
        // add websocket filter
        .and(warp::ws())
        .map(|users: Users, db: Db, ws: warp::ws::Ws| {
            ws.on_upgrade(move | socket| connect_user(users,db,  socket))
        })
}

/// Runs when a user connects and the websocket upgrade is successful.
/// - create a user_id and an mpsc unbounbed_channel. store these in threadsafe Users
/// - return all data saved in db to the user
/// - in tokio task, everytime we recieve data on mpsc unbounbed_channel, send straight back on
/// websocket
/// - for all data recieved on the websocket we send it back to all users (except the user its
/// from) and store on redis stream.
pub async fn connect_user(users: Users, mut db: Db, ws: WebSocket) {

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

    initial_dump(&mut db, &users, user_id).await;

    // In a tokio task, loop forever listening to the receiving end of the mpsc unbounded_channel.
    // When we get data, send back on the user's websocket.
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

    handle_incoming_data(user_ws_rx, &mut db, &users, user_id).await;

    handle_disconnecting(&users, user_id).await;
}


/// Read the entire redis stream and send each entry back on the websocket
async fn initial_dump(db: &mut Db,
                      users: &Users,
                      my_id: Uuid) {

    debug!("Initial dump for user_id {:?}", my_id);

    let stream_data = db.read_all().await;
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



/// Code run after a user disconnects their websocket connection
async fn handle_disconnecting(users:  &Users, my_id: Uuid) {

    debug!("Good bye user_id {:?}", my_id);
    users.write().await.remove(&my_id.to_u128_le());
}

/// When we receive data from the websocket we send it back to all users (except the user its
/// from) and store on redis stream
async fn handle_incoming_data(mut user_ws_rx: SplitStream<WebSocket>,
                              db: &mut Db,
                              users: &Users,
                              my_id: Uuid){
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
        db.write_line(my_id, user_data.to_string()).await;
    }
}
