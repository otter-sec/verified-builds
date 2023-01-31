#![feature(exit_status_error)]

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use endpoints::{buffer_hash, dockerfile, health, index, program_hash, verify};
use warp::Filter;

mod endpoints;
mod parameters;

#[tokio::main]
async fn main() {
    let index = warp::get().and(warp::path::end()).and_then(index);

    let health = warp::path("health").and(warp::path::end()).and_then(health);

    let cache = Arc::new(RwLock::new(HashMap::new()));
    let verify = warp::path("verify")
        .and(warp::query())
        .and_then(move |x| verify(x, cache.clone()));

    let routes = index
        .or(health)
        .or(verify)
        .or(warp::path("dockerfile")
            .and(warp::query())
            .and_then(dockerfile))
        .or(warp::path("program_hash")
            .and(warp::query())
            .and_then(program_hash))
        .or(warp::path("buffer_hash")
            .and(warp::query())
            .and_then(buffer_hash));

    warp::serve(routes).run(([127, 0, 0, 1], 3000)).await;
}
