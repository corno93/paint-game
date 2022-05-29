use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};
use warp::ws::Message;

/// Threadsafe hashmap which represents all the active users.
/// Contains an id as key and a mpsc sender that points to the user's websocket
pub type Users = Arc<RwLock<HashMap<u128, mpsc::UnboundedSender<Message>>>>;
