pub mod handlers;
pub mod auth;

use axum::{routing::post, Router};

pub fn create_router() -> Router {
    Router::new().route("/api/v1/hackrx/run", post(handlers::hackrx_run))
}
