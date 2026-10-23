# libvisio oracle

Dev-only harness that builds libvisio's CLI tools so tests can compare
Larrygram's `.vsd` parsing against an independent implementation of the
same format. libvisio is **never** a runtime dependency and is never
linked into Larrygram (AGENTS.md §5, ADR 0010).

The directory contains a build script and this README. Nothing else:
no git submodule and no wrapper crate — both were rejected in the
P1-02 review (ADR 0010). The script pins the source itself; the
confinement of libvisio-derived code to `crates/vsd-binary` is ADR 0003.

## The tools

libvisio ships six converters under `src/conv/`; all six are built and
installed into `<build-dir>/bin/`:

| Tool | Purpose |
|---|---|
| `vsd2raw` | `.vsd` → raw parsed structure dump |
| `vss2raw` | `.vss` stencil → raw parsed structure dump |
| `vsd2xhtml` | `.vsd` → XHTML rendering |
| `vss2xhtml` | `.vss` → XHTML rendering |
| `vsd2text` | `.vsd` → plain text |
| `vss2text` | `.vss` → plain text |

## Building

```sh
tools/libvisio-oracle/build.sh            # status (the default command)
tools/libvisio-oracle/build.sh build      # clone, configure, make, install
tools/libvisio-oracle/build.sh clean      # remove bin/ and src/
```

or, through xtask:

```sh
cargo xtask oracle status
cargo xtask oracle build [--build-dir <path>] [--jobs <n>]
cargo xtask oracle clean
```

The script is POSIX `sh` and works on macOS and Linux. Options:
`--build-dir <path>` (or the `LARRYGRAM_LIBVISIO_BUILD_DIR` environment
variable) selects the build directory; `--jobs <n>` overrides make's
parallelism.

The build directory defaults to
`/Volumes/Dara/larrygram/build/libvisio-oracle/`, on the external
drive, so it is built once and shared by every worktree (AGENTS.md
§3). The libvisio source is cloned into `<build-dir>/src/libvisio/`
at the commit pinned in `NOTICE` and `docs/sources.md` (source 6):
`c7c970775b750a4bf7fd1b4364d69bac9815bfcb`. Only that build directory
holds libvisio's source; nothing from it is ever committed.

`status` prints `built` and exits 0 when all six tools exist under
`<build-dir>/bin/`, otherwise `not built`, also exiting 0. It only
fails (non-zero) when the build directory exists but cannot be read.

### Build dependencies

libvisio is an autotools C++17 project. Besides a compiler, git,
autoconf, automake, libtool and pkg-config, it needs gperf, perl,
boost headers, librevenge-0.0, libxml-2.0 and icu-uc.

- macOS (Homebrew):
  `brew install autoconf automake libtool pkg-config gperf boost icu4c libxml2 librevenge`
  and point the build at the keg-only libraries, e.g.

  ```sh
  export PKG_CONFIG_PATH="$(brew --prefix icu4c)/lib/pkgconfig"
  export CPPFLAGS="-I$(brew --prefix)/include"
  export LDFLAGS="-L$(brew --prefix)/lib"
  ```

  (Homebrew's own compiler shims add these paths for brewed compilers;
  the Xcode toolchain does not, and `icu4c` is keg-only.)
- Debian/Ubuntu: `build-essential autoconf automake libtool pkg-config
  gperf libboost-dev librevenge-0.0-dev libxml2-dev libicu-dev perl`.

## Optional for the gate

A missing oracle build **never** fails the quality gate (ADR 0010):
libvisio's build is heavy, version dependent on the host, and not
portable to a clean Windows runner. Tests that need the oracle:

- are `#[ignore]`d, so a plain `cargo test` never runs them;
- find the tools via `LARRYGRAM_LIBVISIO_ORACLE`, which must point at
  the directory holding the binaries (usually `<build-dir>/bin`), and
  skip with a message when it is unset or not a directory;
- treat a missing oracle as a **failure** when
  `LARRYGRAM_REQUIRE_ORACLE=1` is also set (CI on Linux and macOS);
  Windows CI does not build the oracle and does not set it.

Run the smoke test locally after building:

```sh
cargo xtask oracle build
LARRYGRAM_LIBVISIO_ORACLE=/Volumes/Dara/larrygram/build/libvisio-oracle/bin \
  cargo test -p larrygram-vsd-binary -- --ignored
```

The helper that reads these variables is a copy-paste of ~10 lines per
consumer crate (see `crates/vsd-binary/tests/common/mod.rs`); the
`cargo xtask oracle` task is the only Rust code that wraps `build.sh`.

## License

libvisio is MPL-2.0 (verified from its `COPYING`), commit
`c7c970775b750a4bf7fd1b4364d69bac9815bfcb`. This harness only builds
it; no libvisio code is copied into Larrygram. Code derived from
libvisio lives exclusively in `crates/vsd-binary` (MPL-2.0, see
`NOTICE`).
