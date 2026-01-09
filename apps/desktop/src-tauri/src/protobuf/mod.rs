//! Protobuf parser and writer for Portfolio Performance binary files.
//!
//! PP files (.portfolio) are ZIP archives containing `data.portfolio` with format:
//! - Bytes 0-5: Header "PPPBV1" (Portfolio Performance Protobuf Version 1)
//! - Bytes 6+: Protobuf Client message

pub mod parser;
mod schema;
pub mod writer;

pub use parser::{parse_portfolio_file, parse_to_client};
pub use writer::{serialize_client, write_portfolio_file};

/// Magic header for PP protobuf format
pub const HEADER: &[u8] = b"PPPBV1";

/// Header length (6 bytes magic only - protobuf starts immediately after)
pub const HEADER_LEN: usize = 6;
