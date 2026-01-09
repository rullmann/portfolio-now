//! Protobuf parser for Portfolio Performance binary files.
//!
//! PP files (.portfolio) are ZIP archives containing `data.portfolio` with format:
//! - Bytes 0-5: Header "PPPBV1" (Portfolio Performance Protobuf Version 1)
//! - Bytes 6-7: Additional header bytes
//! - Bytes 8+: Protobuf Client message

pub mod parser;
mod schema;

pub use parser::{parse_portfolio_file, parse_to_client};

/// Magic header for PP protobuf format
pub const HEADER: &[u8] = b"PPPBV1";

/// Header length (6 bytes magic only - protobuf starts immediately after)
pub const HEADER_LEN: usize = 6;
