use std::env;

use redis::streams::{StreamId, StreamKey, StreamReadReply};
use redis::RedisResult;
use redis::{from_redis_value, AsyncCommands};
use uuid::Uuid;

const STREAM_NAME: &str = "paint-game";

/// The key we use to a assign a user-id's UUID in the redis stream
const USER_ID_KEY: &str = "user_id";

/// The key we use to assign player data in the redis stream
const DATA_ID_KEY: &str = "data";

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

    /// Write to the stream
    /// redis cmd: XADD paint-game * user_id <user_id> data <data>
    pub async fn write_line(&mut self, user_id: Uuid, data: String) {
        let _: String = self
            .connection_manager
            .xadd(
                STREAM_NAME,
                "*",
                &[(USER_ID_KEY, user_id.to_string()), (DATA_ID_KEY, data)],
            )
            .await
            .unwrap();
    }

    /// Read the entire stream and serialise data
    /// redis cmd: XREAD STREAMS paint-game 0
    pub async fn read_all_lines(&mut self) -> Vec<String> {
        let reply: StreamReadReply = self
            .connection_manager
            .xread(&[STREAM_NAME], &[0])
            .await
            .unwrap();

        let mut all_data: Vec<String> = Vec::new();
        for StreamKey { key: _, ids } in reply.keys {
            for StreamId { id: _, map: zz } in ids {
                // TODO: dont use unwrap
                let r_data: RedisResult<String> = from_redis_value(&zz.get(DATA_ID_KEY).unwrap());
                let data = r_data.unwrap();
                all_data.push(data)
            }
        }
        return all_data;
    }
}
