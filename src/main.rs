use crate::routes::connect_user::connect_user;
use crate::routes::{connect_user, index};

mod routes;
mod db;
mod types;

use types::Users;
use warp::Filter;
use warp::ws::{Message, WebSocket};

#[tokio::main]
async fn main() {
    env_logger::init();

    let users = Users::default();
    let connect_user_route = routes::connect_user::connect_user_route(users);
    let index_route = routes::index::index_route();

    let all_routes = index_route.or(connect_user_route);
    warp::serve(all_routes).run(([127, 0, 0, 1], 3030)).await;
}
