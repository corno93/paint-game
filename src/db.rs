use std::env;
use std::fmt::Debug;

use redis::AsyncCommands;
use redis::streams::StreamReadReply;
use uuid::Uuid;

const STREAM_NAME: &str = "paint-game";

#[derive(Clone)]
pub struct Db {
    connection_manager: redis::aio::ConnectionManager,
}

impl Db {
    /// Establish connection to redis. Store connection for future use across all threads.
    pub async fn init() -> Db {
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

        Db {
            connection_manager: redis::Client::open(redis_conn_url)
                .expect("Invalid connection URL")
                .get_tokio_connection_manager()
                .await
                .expect("failed to connect to Redis"),
        }
    }

    /// Write to the stream (redis cmd: XADD paint-game * user_id <user_id> data <data>)
    pub async fn write_line(&mut self, user_id: Uuid, data: String) {
        let _: String = self
            .connection_manager
            .xadd(
                STREAM_NAME,
                "*",
                &[("user_id", user_id.to_string()), ("data", data)],
            )
            .await
            .unwrap();
    }

    /// Read the entire stream (redis cmd: XREAD STREAMS paint-game 0)
    pub async fn read_all(&mut self) -> StreamReadReply {
        let reply: StreamReadReply = self
            .connection_manager
            .xread(&[STREAM_NAME], &[0])
            .await
            .unwrap();
        reply
    }
}
