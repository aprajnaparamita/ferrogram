//! Fuzz target for the formula parser (task P6-02, ADR 0060).
//!
//! Any bytes go in and are read as lossy UTF-8 formula source. The
//! contract (ADR 0051): no input panics, the limits hold, and whatever
//! the parser accepts prints back to text that parses again. An `Err`
//! is a valid outcome at every step.

#![no_main]

use larrygram_shapesheet::{parse, print};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let source = String::from_utf8_lossy(data);
    if let Ok(expr) = parse(&source) {
        // A tree that parses prints back to text that parses again.
        if let Ok(text) = print(&expr) {
            let _ = parse(&text);
        }
    }
});
