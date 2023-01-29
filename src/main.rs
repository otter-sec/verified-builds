use endpoints::{health, index, verify};
use warp::Filter;

mod endpoints;
mod parameters;

#[tokio::main]
async fn main() {
    let index = warp::get().and(warp::path::end()).and_then(index);

    let health = warp::path("health").and(warp::path::end()).and_then(health);

    let verify = warp::path("verify").and(warp::query()).and_then(verify);

    let routes = index.or(health).or(verify);

    warp::serve(routes).run(([127, 0, 0, 1], 3000)).await;
}
