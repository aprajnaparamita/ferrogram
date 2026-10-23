//! Fuzz target for the package reader (task P2-09, ADR 0017 section 4).
//!
//! The package reader is the entry point for untrusted bytes, so raw bytes go
//! straight into `read_document` with the default [`Limits`]. A panic fails
//! the fuzz run; an error is a valid outcome. When the bytes do read, the
//! document is written back and re-read to catch write-side panics too.
//! `write_document` returns `Err` until task P2-07 implements it: any
//! `Result` is accepted there and only a panic fails.

#![no_main]

use larrygram_vsdx::{Limits, Package, read_document, write_document};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(document) = read_document(data, &Limits::DEFAULT) {
        // Round-trips through the writer: any result is fine, no panic allowed.
        if let Ok(written) = write_document(&document, &Package::new()) {
            let _ = read_document(&written, &Limits::DEFAULT);
        }
    }
});
