// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Smoke test for the libvisio oracle harness (P1-07, ADR 0010).
//!
//! Ignored by default: it runs `vsd2raw --help` from the oracle build,
//! which the gate does not require. After `cargo xtask oracle build`:
//!
//! ```sh
//! LARRYGRAM_LIBVISIO_ORACLE=/Volumes/Dara/larrygram/build/libvisio-oracle/bin \
//!   cargo test -p larrygram-vsd-binary -- --ignored
//! ```
//!
//! With `LARRYGRAM_REQUIRE_ORACLE=1` (CI on Linux and macOS) a missing
//! oracle fails this test instead of skipping it. CI on Windows does
//! not build the oracle and does not set the variable.

mod common;

#[test]
#[ignore = "needs the libvisio oracle; see ADR 0010"]
fn vsd2raw_help_mentions_its_name() {
    let result = common::smoke("vsd2raw");
    assert!(result.is_ok(), "{result:?}");
}
