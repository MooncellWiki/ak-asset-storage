// Web adapter layer - HTTP/REST API interface
// This layer provides web interface to the application

pub(crate) mod dto;
pub(crate) mod error;
pub(crate) mod handlers;
pub(crate) mod middleware;
pub(crate) mod routes;
pub(crate) mod server;
pub(crate) mod state;
pub(crate) mod utils;

pub use server::start;
