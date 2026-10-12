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

## Decision

`tools/libvisio-oracle/` contains a build script, the libvisio source
as a git submodule, and a small wrapper that exposes the CLI tools to
Rust tests.

### Layout

```
tools/
  libvisio-oracle/
    README.md            # how to build; what it does; optional-for-gate
    build.sh             # bootstraps, configures, makes
    libvisio/            # git submodule at the pinned commit
    bin/                 # built CLI tools land here; gitignored
    src/                 # a thin Rust wrapper crate (optional)
```

`tools/libvisio-oracle` is **excluded** from the Cargo workspace
(`exclude = ["tools/libvisio-oracle"]` is already in the root
`Cargo.toml`). The wrapper, if it exists, is its own crate.

### Build script (`build.sh`)

- Args: `tools/libvisio-oracle/build.sh [build|clean|status]`.
- `status`: print "built" or "not built" and exit 0 either way.
- `build`:
  - If `bin/vsd2raw` exists, exit 0.
  - Else ensure `libvisio/` is a populated checkout. If the submodule
    is empty, run `git submodule update --init` (or a shallow
    `git clone` if the host machine does not have submodules
    configured, with the commit hash pinned in `build.sh`).
  - `cd libvisio && ./autogen.sh && ./configure --prefix="$PWD/.." --disable-werror`
  - `make -j$(sysctl -n hw.ncpu 2>/dev/null || nproc)` into `bin/`.
- `clean`: `make distclean` or `rm -rf bin/`.

The build dir lives at
`/Volumes/Dara/ferrogram/build/libvisio-oracle/` (separate from the
worktree to keep the main drive small). The script accepts a
`--build-dir` override for CI.

### How tests use it

A test in `crates/vsd-binary/tests/oracle.rs` (P1-07) calls the
helper `ferrogram_libvisio_oracle::binary_path("vsd2raw")`, which:

- Returns `Ok(PathBuf)` if `bin/vsd2raw` exists.
- Returns `Err(Skip("libvisio oracle not built; run
  tools/libvisio-oracle/build.sh"))` otherwise.

Tests annotated `#[ignore = "requires libvisio oracle"]` call
`Command::new(binary_path("vsd2raw")?)` and compare its output to
our `vsd-binary` parser. The `cargo xtask oracle` task (P1-07) wraps
`build.sh status` and emits a `PASS` / `SKIP` line.

### Optional for the gate

- `cargo xtask gate` does **not** require the oracle. The oracle is
  built on demand, not at gate time.
- The xtask `oracle` task is a separate command:
  `cargo xtask oracle [--build|--status]`. CI runs it before the
  gate if the build dir exists; otherwise it is skipped.
- A test that requires the oracle is gated: by default it is
  `#[ignore]`d. Setting `FERROGRAM_REQUIRE_ORACLE=1` (CI uses this
  on Linux and macOS runners where the build environment is set up)
  flips the gate so the test is included and a missing binary is a
  failure.

### Why the build is optional

- libvisio's build is heavy (boost + librevenge + icu), version
  dependent on the host, and not portable to a clean Windows runner
  without vcpkg. The KICKOFF quality gate must run on three OSes with
  no network and no system packages; a build that depends on apt /
  brew / choco cannot be in the gate.
- A test that depends on the oracle is opt-in. The default `cargo
  test` run is fast and portable. The oracle-specific tests are
  behind `FERROGRAM_REQUIRE_ORACLE=1`.

### Why not in xtask

`tools/` is the standard Cargo location for dev-only, non-workspace
code. The KICKOFF.md layout already lists `tools/libvisio-oracle/`
as the home. xtask is for tasks that build the workspace; the oracle
builds an external C++ project and is therefore in `tools/`.

## Consequences

- The libvisio binary is built once, shared by every worktree on the
  same machine (it lives on `/Volumes/Dara/ferrogram/build/`).
- Adding more oracle binaries (`vsd2xhtml`, `vss2raw`) is one
  `make` invocation; no Rust code changes.
- A test that uses the oracle can be run locally with
  `cargo test -p ferrogram-vsd-binary -- --ignored` after building.
- CI on macOS and Linux runs the oracle build and
  `FERROGRAM_REQUIRE_ORACLE=1 cargo test`. CI on Windows skips it.
- A future P1-07 task owns the harness itself; P1-02 only designs
  the shape.

## Alternatives considered

- **Build libvisio once at workspace bootstrap and check the binaries
  in.** Rejected: the binaries are 10+ MB of C++ with platform-
  specific libraries; checking them in is hostile to contributors.
- **Make the oracle a runtime dependency in `Cargo.toml`.** Rejected:
  violates KICKOFF and AGENTS.md (MPL confinement, dev-only).
- **Use a Docker image for the oracle.** Rejected: macOS CI does not
  run Docker natively; the local development story is the same code,
  not a different OS.
- **Write a Rust wrapper that re-implements the libvisio binary
  protocol.** Rejected: enormous scope, reinvents the work libvisio
  already did, and violates clean-room (the wrapper would be a
  derivation of libvisio, putting MPL code outside `vsd-binary`).
