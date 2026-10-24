#!/bin/sh
# Build libvisio's CLI tools for use as a Larrygram test oracle.
#
# Dev-only harness, see ADR 0010. libvisio is never a runtime
# dependency of Larrygram and is never linked into it: the tools are
# compared against in tests only.
#
# usage: build.sh [build|clean|status] [--build-dir <path>] [--jobs <n>]

set -eu

# Pinned libvisio commit (NOTICE; docs/sources.md source 6). Keep the
# two in sync.
LIBVISIO_COMMIT=c7c970775b750a4bf7fd1b4364d69bac9815bfcb
LIBVISIO_URL=https://git.libreoffice.org/libvisio

# The six CLI tools under src/conv/ that `status` checks. Keep in sync
# with BINARIES in xtask/src/oracle.rs.
BINARIES="vsd2raw vss2raw vsd2xhtml vss2xhtml vsd2text vss2text"

# On this machine everything large lives on the external drive
# (AGENTS.md, "Disk usage"); CI overrides with --build-dir or
# LARRYGRAM_LIBVISIO_BUILD_DIR.
DEFAULT_BUILD_DIR=/Volumes/Dara/larrygram/build/libvisio-oracle

build_dir=${LARRYGRAM_LIBVISIO_BUILD_DIR:-$DEFAULT_BUILD_DIR}
jobs=
cmd=status

die() {
    echo "build.sh: $*" >&2
    exit 2
}

usage() {
    cat <<EOF
usage: build.sh [build|clean|status] [--build-dir <path>] [--jobs <n>]

commands:
  build             clone libvisio at the pinned commit, then configure,
                    make and make install (no-op when already built)
  clean             remove <build-dir>/bin and <build-dir>/src
  status            print "built" or "not built" (the default command);
                    exits 0 unless the build dir cannot be read

options:
  --build-dir <path>  build directory
                      (default: \$LARRYGRAM_LIBVISIO_BUILD_DIR or $DEFAULT_BUILD_DIR)
  --jobs <n>          parallel jobs for make (default: number of CPUs)
  -h, --help          show this help

The build needs a C++17 compiler, autoconf, automake, libtool, pkg-config,
gperf, perl, boost headers, librevenge-0.0, libxml-2.0 and icu-uc.
EOF
}

while [ $# -gt 0 ]; do
    case $1 in
        build|clean|status)
            cmd=$1
            shift
            ;;
        --build-dir)
            [ $# -ge 2 ] || die "--build-dir needs a path"
            build_dir=$2
            shift 2
            ;;
        --build-dir=*)
            build_dir=${1#--build-dir=}
            shift
            ;;
        --jobs)
            [ $# -ge 2 ] || die "--jobs needs a number"
            jobs=$2
            shift 2
            ;;
        --jobs=*)
            jobs=${1#--jobs=}
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        -*)
            die "unknown option: $1 (try --help)"
            ;;
        *)
            die "unknown command: $1 (try --help)"
            ;;
    esac
done

[ -n "$build_dir" ] || die "--build-dir needs a non-empty path"
if [ -n "$jobs" ]; then
    case $jobs in
        *[!0-9]*|0) die "--jobs needs a positive integer, got: $jobs" ;;
    esac
fi

src_dir=$build_dir/src/libvisio

# Prints "built" or "not built". Exits non-zero only on a hard error:
# the build directory exists but is not readable as a directory.
status() {
    if [ -e "$build_dir" ] && [ ! -d "$build_dir" ]; then
        die "build dir is not a directory: $build_dir"
    fi
    if [ -d "$build_dir" ] && [ ! -r "$build_dir" ]; then
        die "cannot read build dir: $build_dir"
    fi
    for tool in $BINARIES; do
        if [ ! -f "$build_dir/bin/$tool" ]; then
            echo "not built"
            return 0
        fi
    done
    echo "built"
    return 0
}

# Number of CPUs; nproc on Linux, sysctl on macOS, getconf elsewhere.
cpus() {
    if command -v nproc >/dev/null 2>&1; then
        nproc
    elif command -v sysctl >/dev/null 2>&1; then
        sysctl -n hw.ncpu 2>/dev/null || echo 2
    elif command -v getconf >/dev/null 2>&1; then
        getconf _NPROCESSORS_ONLN 2>/dev/null || echo 2
    else
        echo 2
    fi
}

# Make sure the source tree is at the pinned commit. The shallow clone
# only carries the tip, so when upstream has moved on, fetch the exact
# commit (or the full history as a last resort).
checkout_pinned() {
    if git -C "$src_dir" checkout --quiet "$LIBVISIO_COMMIT" 2>/dev/null; then
        return 0
    fi
    echo "build.sh: $LIBVISIO_COMMIT is not in the clone; fetching it" >&2
    git -C "$src_dir" fetch --depth 1 origin "$LIBVISIO_COMMIT" ||
        git -C "$src_dir" fetch --unshallow origin
    git -C "$src_dir" checkout --quiet "$LIBVISIO_COMMIT"
}

# Clone libvisio at the pinned commit unless the tree is already there.
ensure_source() {
    if [ -d "$src_dir" ] && [ -n "$(ls -A "$src_dir" 2>/dev/null || true)" ]; then
        if [ -d "$src_dir/.git" ]; then
            checkout_pinned
        else
            echo "build.sh: reusing existing source tree $src_dir (not a git checkout)" >&2
        fi
        return 0
    fi
    mkdir -p "$build_dir/src"
    git clone --depth 1 "$LIBVISIO_URL" "$src_dir"
    checkout_pinned
}

# Clone, configure, make, make install. Fails if the tools are not all
# in <build-dir>/bin afterwards.
build() {
    built=$(status)
    if [ "$built" = built ]; then
        echo "build.sh: already built in $build_dir/bin"
        return 0
    fi
    mkdir -p "$build_dir"
    ensure_source
    if [ -z "$jobs" ]; then
        jobs=$(cpus)
    fi
    echo "build.sh: building libvisio in $src_dir (-j$jobs, prefix $build_dir)"
    cd "$src_dir"
    ./autogen.sh
    # --disable-tests: libvisio's own cppunit unit tests are not part of
    # the oracle; the CLI tools are all we need.
    ./configure --prefix="$build_dir" --disable-werror --disable-tests
    make -j"$jobs"
    make install
    built=$(status)
    [ "$built" = built ] ||
        die "make install finished but not every tool is in $build_dir/bin"
}

clean() {
    rm -rf "$build_dir/bin" "$build_dir/src"
    echo "build.sh: removed $build_dir/bin and $build_dir/src"
}

case $cmd in
    status) status ;;
    build) build ;;
    clean) clean ;;
    *) die "unknown command: $cmd" ;;
esac
