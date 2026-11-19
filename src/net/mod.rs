//! The network edge of the service, written here rather than taken from
//! crates: JSON, HTTP/1.1, SHA-256 and base64. The engine does not use
//! any of it.

pub mod codec;
pub mod http;
pub mod json;
