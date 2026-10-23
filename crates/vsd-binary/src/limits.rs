// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Input caps for the binary Visio import (ADR 0071).
//!
//! Every size or count that comes from the file is checked against a
//! [`Limits`] cap before memory is allocated for it (AGENTS.md section
//! 6: capped allocations on input-derived values). Exceeding a cap is
//! [`crate::Error::LimitExceeded`], never a panic.

/// The caps applied to one import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limits {
    /// The maximum number of compound-file streams.
    pub max_streams: u32,
    /// The maximum uncompressed size of one stream, in bytes.
    pub max_stream_bytes: u64,
    /// The maximum uncompressed size of all streams together, in bytes.
    pub max_total_uncompressed: u64,
    /// The maximum uncompressed-to-compressed ratio of one compressed
    /// block (decompression-bomb guard).
    pub max_compression_ratio: u64,
    /// The maximum number of chunk records in the `VisioDocument` stream.
    pub max_chunks: u32,
    /// The maximum nesting level of chunk records.
    pub max_chunk_level: u16,
    /// The maximum number of shapes in the whole document.
    pub max_shapes: u32,
    /// The maximum total size of imported text, in bytes.
    pub max_text_bytes: u64,
}

impl Limits {
    /// The caps for normal files (ADR 0071).
    pub const DEFAULT: Self = Self {
        max_streams: 1024,
        max_stream_bytes: 64 * 1024 * 1024,
        max_total_uncompressed: 512 * 1024 * 1024,
        max_compression_ratio: 200,
        max_chunks: 1_000_000,
        max_chunk_level: 64,
        max_shapes: 100_000,
        max_text_bytes: 64 * 1024 * 1024,
    };
}

impl Default for Limits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_caps_are_positive_and_consistent() {
        let limits = Limits::default();
        assert_eq!(limits, Limits::DEFAULT);
        assert!(limits.max_streams > 0);
        assert!(limits.max_stream_bytes > 0);
        assert!(limits.max_total_uncompressed >= limits.max_stream_bytes);
        assert!(limits.max_compression_ratio > 1);
        assert!(limits.max_chunks > 0);
        assert!(limits.max_chunk_level > 0);
        assert!(limits.max_shapes > 0);
        assert!(limits.max_text_bytes > 0);
    }
}
