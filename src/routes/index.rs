use warp::ws::{Message, WebSocket};
use warp::Filter;

pub fn index_route() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("static").and(warp::fs::dir("static"))
}
