//! `island serve`: the people dashboard and Human Creator.

use warp::Filter;

pub mod auth;
pub mod pages;
pub mod server;
pub mod view;

/// A method filter for everything that only reads. Monitors and proxies ask
/// with HEAD as readily as with GET, and hyper drops the body from a HEAD
/// response for us, so both belong on every read route.
pub fn read() -> impl Filter<Extract = (), Error = warp::Rejection> + Clone {
    warp::get().or(warp::head()).unify()
}
