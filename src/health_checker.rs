use std::net::SocketAddr;
use warp::Filter;

pub async fn run(addr: impl Into<SocketAddr>) {
    let health = warp::path::end().map(warp::reply);

    let app = warp::any().and(health);

    warp::serve(app).run(addr).await;
}
