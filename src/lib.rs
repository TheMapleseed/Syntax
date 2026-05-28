//! # secure_pipeline
//!
//! Pure Rust (std-only) packet transport for ASCII text with a strict wire alphabet,
//! collision-proof space escaping, and native security boundaries.
//!
//! ## Security model
//!
//! - **Wire safety:** encoded packets match `^[A-Za-z0-9_-]+$`.
//! - **Plaintext policy:** printable ASCII only (`0x20`..=`0x7E`); control bytes rejected.
//! - **Rate limiting:** 500 characters per second (process-wide, shared by encode/decode).
//! - **No size cap:** there is no hard maximum input length; rate limiting is the throughput guard.
//! - **Sink safety:** after decode, use parameterized SQL, shell argv arrays, and context-specific
//!   escaping for each downstream interpreter. Wire safety does not imply SQL/shell safety.
//!
//! ## Example
//!
//! ```
//! use secure_pipeline::{decode_packet, encode_packet};
//!
//! let encoded = encode_packet("hello safe world").unwrap();
//! let decoded = decode_packet(&encoded).unwrap();
//! assert_eq!(decoded, "hello safe world");
//! ```

mod base64url;
pub mod concurrent;
pub mod pipeline;
pub mod validation;

pub use concurrent::{ConcurrentPipeline, PipelineConfig};
pub use pipeline::{decode_packet, encode_packet, PipelineError};
pub use validation::{validate_wire_format, ValidationError};
