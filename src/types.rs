use std::collections::HashMap;
use std::convert::From;
use std::fmt;
use std::result;
use std::sync::Arc;

use redis::RedisError;
use tokio::sync::mpsc::error::{SendError, TryRecvError};
use tokio::sync::{mpsc, RwLock};
use warp::ws::Message;
use serde::{Deserialize, Serialize};

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


#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Line {
    attrs: LineAttrs,
    className: String,
}

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)]
struct LineAttrs {
    stroke: String,
    strokeWidth: i16,
    lineCap: String,
    points: Vec<f32>
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn it_works() {
        let line = r#"
        {
            "attrs":
            {
                "stroke": "/#df4b26",
                "strokeWidth": 5,
                "lineCap": "round",
                "points":
                [
                    164.00105119637686,
                    856.3408239439925,
                    164.00105119637686,
                    856.3408239439925,
                    170.00108965478088,
                    856.3408239439925
                ]
            },
            "className": "Line"
        }
        "#;
        let l: Line = serde_json::from_str(line).unwrap();
        assert_eq!(l.attrs.stroke, "/#df4b26");
        assert_eq!(l.attrs.strokeWidth, 5);
        assert_eq!(l.attrs.lineCap, "round");
        assert_eq!(l.attrs.points, vec![
            164.00105119637686,
                    856.3408239439925,
                    164.00105119637686,
                    856.3408239439925,
                    170.00108965478088,
                    856.3408239439925]);
        assert_eq!(l.className, "Line")
    }
}
