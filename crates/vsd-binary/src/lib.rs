// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Read-only import of binary .vsd, .vss and .vst files into the Ferrogram model.
//!
//! This crate is the only place where code ported or derived from libvisio may live.
//! It is licensed MPL-2.0; see NOTICE at the repository root.
//!
//! Part of Ferrogram. Alpha: this crate is a placeholder until its phase lands.

#![forbid(unsafe_code)]
#![deny(clippy::indexing_slicing)]
