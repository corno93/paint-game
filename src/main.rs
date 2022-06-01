use crate::routes::{connect_user::connect_user_route, index::index_route};

mod db;
mod routes;
mod types;

use db::Db;
use types::Users;

// Filter needs to be here. I think Rust is auto inferring the types for connect_user_route,
// and index_route. Without `use warp::Filter` Rust complains index_route does not have
// method 'or'.
use warp::Filter;

#[tokio::main]
async fn main() {
    env_logger::init();

    let users = Users::default();
    let db = Db::init().await;

    let connect_user_route = connect_user_route(users, db);
    let index_route = index_route();
    let all_routes = index_route.or(connect_user_route);

    warp::serve(all_routes).run(([127, 0, 0, 1], 3030)).await;
}
