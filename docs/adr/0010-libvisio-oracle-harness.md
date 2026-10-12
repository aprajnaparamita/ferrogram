# 0010: libvisio oracle harness

- Status: accepted
- Date: 2026-10-12
- Task: P1-02

## Context

KICKOFF.md says: "Build its CLI tools in a dev-only harness under
`tools/libvisio-oracle/`. Candidates are vsd2raw, vsd2xhtml and vss2raw;
check which ones exist." These binaries are an oracle: tests call them
to compare our parsed output against an independent implementation of
the same .vsd format.

P1-01 recorded in `docs/sources.md`:

- libvisio source commit `c7c970775b750a4bf7fd1b4364d69bac9815bfcb`
  (2026-10-09), MPL-2.0.
- Six CLI tools under `src/conv/`: `vsd2raw`, `vss2raw`, `vsd2xhtml`,
  `vss2xhtml`, `vsd2text`, `vss2text`. Source-only; not built in Phase 1.
- Build dependencies: boost, gperf, icu, librevenge (`librevenge-0.0`,
  `librevenge-stream-0.0`, `librevenge-generators-0.0`), libxml2, perl.
  Autoconf 2.69, automake 1.11, libtool. C++.

libvisio is **not** a runtime dependency and never linked into
Ferrogram (AGENTS.md, KICKOFF.md). The oracle is a dev tool only.

The P1-02 review (M3) found three issues with an earlier draft:

- A git submodule under `tools/libvisio-oracle/libvisio/` would put
  libvisio's full source tree (C++, autotools, generated files) into
  every worktree on the small main drive. Submodules are also hard to
  pin and to upgrade.
- A separate `tools/libvisio-oracle/` wrapper crate outside the
  workspace, depended on by `crates/vsd-binary` tests via a path
  dependency, is fragile (the path-dependency machinery is brittle
  when the wrapper is also workspace-excluded) and adds a second
  Cargo.toml to maintain.
- Tests depending on a wrapper crate would pull libvisio's build
  environment into the test graph even when the test is skipped.

## Decision

`tools/libvisio-oracle/` contains a build script, a small `README`,
and **nothing else**. There is no submodule and no wrapper crate.
The build script clones libvisio at the pinned commit into a build
directory on the external drive and installs the CLI tools there.
Tests find the tools via the `FERROGRAM_LIBVISIO_ORACLE` environment
variable and skip with a clear message when it is unset or empty,
unless `FERROGRAM_REQUIRE_ORACLE=1`.

### Layout

```
tools/
  libvisio-oracle/
    README.md            # how to build; what it does; optional-for-gate
    build.sh             # POSIX sh; clones, configures, makes
    # No submodule, no wrapper crate, no Cargo.toml.
```

`tools/libvisio-oracle` is already excluded from the Cargo workspace
(`exclude = ["tools/libvisio-oracle"]` in the root `Cargo.toml`).

### Build script (`build.sh`)

- Args: `tools/libvisio-oracle/build.sh [build|clean|status]`.
- The build directory defaults to
  `/Volumes/Dara/ferrogram/build/libvisio-oracle/`. Override with
  `--build-dir <path>` or the `FERROGRAM_LIBVISIO_BUILD_DIR`
  environment variable. CI sets `FERROGRAM_LIBVISIO_BUILD_DIR` to
  whatever the runner exposes (e.g. `$HOME/libvisio-oracle` on
  Linux).
- `status`: prints `built` and exits 0 if all six binaries exist
  under `<build-dir>/bin/`. Prints `not built` and exits 0 if
  anything is missing. Exits non-zero only on a hard error (cannot
  read the directory).
- `build`:
  - If `status` is `built`, exit 0.
  - `mkdir -p "$BUILD_DIR"`. If `<build-dir>/src/libvisio/` is
    empty, `git clone --depth 1` libvisio from
    `https://git.libreoffice.org/libvisio` into that directory and
    `git -C src/libvisio checkout c7c970775b750a4bf7fd1b4364d69bac9815bfcb`
    (the commit hash from `NOTICE` and `docs/sources.md` source 6).
    The `--depth 1` clone avoids pulling the full history; the
    checkout restores the exact commit we want.
  - `cd src/libvisio && ./autogen.sh && ./configure --prefix="$BUILD_DIR" --disable-werror`.
  - `make -j"$(nproc 2>/dev/null || sysctl -n hw.ncpu)"`
  - `make install` (which writes to `$BUILD_DIR/bin/`).
  - On any error, exit non-zero with the failing command's output.
- `clean`: `rm -rf "$BUILD_DIR/bin" "$BUILD_DIR/src"`. Exit 0.
- A `--jobs N` flag overrides the parallelism (CI uses `-j 2`).
- The script is POSIX sh; no bash-isms. It must work on macOS,
  Linux, and the build containers used by CI.

### How tests find the oracle

Tests in any crate's `tests/` directory (e.g.
`crates/vsd-binary/tests/oracle_smoke.rs` in Phase 7, P1-07 only
authors the harness and the smoke test) call a small helper:

```rust
// Pseudocode for the helper; lives in the crate's own `tests/common/`
// or a tiny `xtask`-published helper crate (no path dep on
// tools/libvisio-oracle).
fn oracle_dir() -> Option<PathBuf> {
    let p = std::env::var_os("FERROGRAM_LIBVISIO_ORACLE")?;
    let p = PathBuf::from(p);
    if p.is_dir() { Some(p) } else { None }
}
```

The helper returns `Some(path)` when `FERROGRAM_LIBVISIO_ORACLE` is
set to a directory that exists, and `None` otherwise. Tests that
need the oracle are `#[ignore]` by default and are included only
when the helper returns `Some`. Setting `FERROGRAM_REQUIRE_ORACLE=1`
in addition to `FERROGRAM_LIBVISIO_ORACLE` flips the gate so a
missing oracle is a failure (CI on Linux and macOS does this).

The helper is published as a tiny **dev-dependency** in each
consumer crate. There is no central `tools/libvisio-oracle` crate;
the helper is a copy-paste of ~10 lines plus a unit test. The
`xtask` `oracle` task (below) is the only place that wraps
`build.sh`.

### `cargo xtask oracle` task

A separate xtask command:

```
cargo xtask oracle status
cargo xtask oracle build [--build-dir <path>]
cargo xtask oracle clean
```

`status` runs `build.sh status` and prints `oracle: built` or
`oracle: not built` for the gate summary. `build` runs `build.sh
build`. `clean` runs `build.sh clean`. CI runs `cargo xtask oracle
status` before the gate on Linux and macOS; if `not built`, the
oracle-using tests are skipped (they are `#[ignore]`).

### Why no submodule

- The libvisio source is ~100 MB once you include its autotools
  generated files and a full clone. Worktrees share `.git` but each
  has its own `tools/libvisio-oracle/libvisio/` directory; that
  fills the main drive.
- The commit is already pinned in `NOTICE`; the build script
  re-derives the source from that pin at build time. A submodule
  just adds a second place to update the pin.
- A shallow `git clone` is faster than `git submodule update` and
  is the same code path on every machine.

### Why no wrapper crate

- A wrapper crate outside the workspace, depended on by tests in
  `crates/vsd-binary` via a `path = "../../tools/libvisio-oracle"`
  dependency, would still compile against the host's toolchain on
  every `cargo test` run, even when the test is `#[ignore]`d. The
  compile-only cost is small, but the maintenance cost of a second
  Cargo.toml is not.
- A wrapper crate is also the wrong place for the oracle: the
  oracle is a binary protocol (subprocess + stdout), not a Rust API.
  The 10-line `oracle_dir()` helper is the entire Rust surface.

### Why the build is optional

- libvisio's build is heavy (boost + librevenge + icu), version
  dependent on the host, and not portable to a clean Windows runner
  without vcpkg. The KICKOFF quality gate must run on three OSes
  with no network and no system packages; a build that depends on
  apt / brew / choco cannot be in the gate.
- A test that depends on the oracle is opt-in. The default `cargo
  test` run is fast and portable. The oracle-specific tests are
  behind `FERROGRAM_REQUIRE_ORACLE=1`.

### Why not in xtask

`tools/` is the standard Cargo location for dev-only, non-workspace
code. The KICKOFF.md layout already lists `tools/libvisio-oracle/`
as the home. xtask is for tasks that build the workspace; the oracle
builds an external C++ project and is therefore in `tools/`.

## Consequences

- The libvisio binary is built once, shared by every worktree on
  the same machine (it lives on `/Volumes/Dara/ferrogram/build/`,
  overridable per-machine or per-CI-run).
- Adding more oracle binaries (`vsd2xhtml`, `vss2raw`) is one
  `make` invocation; no Rust code changes.
- A test that uses the oracle can be run locally with
  `FERROGRAM_LIBVISIO_ORACLE=/Volumes/Dara/ferrogram/build/libvisio-oracle/bin \
   cargo test -p ferrogram-vsd-binary -- --ignored`
  after building.
- CI on macOS and Linux runs the oracle build and
  `FERROGRAM_REQUIRE_ORACLE=1 cargo test`. CI on Windows skips it.
- A future P1-07 task owns the harness itself; P1-02 only designs
  the shape.
- The `xtask` `oracle` arm is the only Rust code that calls
  `build.sh`. Everything else uses
  `FERROGRAM_LIBVISIO_ORACLE` directly.

## Alternatives considered

- **Git submodule.** Rejected (M3): puts libvisio's full source
  tree into every worktree on the small main drive; harder to
  upgrade; the build script already pins the commit.
- **Wrapper crate outside the workspace, depended on by tests.**
  Rejected (M3): a second Cargo.toml for ~10 lines of code; a
  path dependency on a workspace-excluded crate is fragile.
- **Build libvisio once at workspace bootstrap and check the
  binaries in.** Rejected: the binaries are 10+ MB of C++ with
  platform-specific libraries; checking them in is hostile to
  contributors.
- **Make the oracle a runtime dependency in `Cargo.toml`.** Rejected:
  violates KICKOFF and AGENTS.md (MPL confinement, dev-only).
- **Use a Docker image for the oracle.** Rejected: macOS CI does not
  run Docker natively; the local development story is the same code,
  not a different OS.
- **Write a Rust wrapper that re-implements the libvisio binary
  protocol.** Rejected: enormous scope, reinvents the work libvisio
  already did, and violates clean-room (the wrapper would be a
  derivation of libvisio, putting MPL code outside `vsd-binary`).
