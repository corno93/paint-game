use std::collections::HashMap;
use std::convert::From;
use std::fmt;
use std::result;
use std::sync::Arc;

use redis::RedisError;
use tokio::sync::mpsc::error::{SendError, TryRecvError};
use tokio::sync::{mpsc, RwLock};
use warp::ws::Message;

/// Threadsafe hashmap which represents all the active users.
/// Contains an id as key and a mpsc sender that points to the user's websocket
pub type Users = Arc<RwLock<HashMap<u128, mpsc::UnboundedSender<Message>>>>;

#[derive(Debug)]
pub enum Error {
    Db(RedisError),
    SendMsg(SendError<Message>),
    RecvMsg(TryRecvError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Db(err) => err.fmt(f),
            Error::SendMsg(err) => err.fmt(f),
            Error::RecvMsg(err) => err.fmt(f),
        }
    }
}

impl From<RedisError> for Error {
    fn from(error: RedisError) -> Self {
        Error::Db(error)
    }
}

impl From<SendError<Message>> for Error {
    fn from(error: SendError<Message>) -> Self {
        Error::SendMsg(error)
    }
}

impl From<TryRecvError> for Error {
    fn from(error: TryRecvError) -> Self {
        Error::RecvMsg(error)
    }
}

pub type Result<T> = result::Result<T, Error>;
