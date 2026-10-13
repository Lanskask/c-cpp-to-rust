#!/bin/bash
# Reference build of tinyjson (RUN-NOTES "## Reference build"): GCC 16 + libstdc++, -std=c++17, -include locale, -O0.
# Builds the C++ driver (work/dump) and the patched upstream tests (work/ref/tinyjsonTests). Paths are absolute and
# nothing runs with a pilot directory as the working directory.
# Usage: bash build_ref.sh        (any cwd)
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HARNESS="$(dirname "$HERE")"
SRC="$(cd "$HARNESS/../../src" && pwd)"          # pilots/tinyjson/src (upstream clone, read only)
CXX="${CXX:-/opt/homebrew/bin/g++-16}"
FLAGS=(-std=c++17 -include locale -O0 -w)
mkdir -p "$HARNESS/work/ref"
cd "$HARNESS/work"                               # a git-ignored scratch dir as cwd
"$CXX" --version | head -1
"$CXX" "${FLAGS[@]}" -I "$SRC/src" "$HARNESS/dump.cpp" -o "$HARNESS/work/dump"
"$CXX" "${FLAGS[@]}" -I "$SRC/src" -I "$SRC/test" "$HERE/tinyjsonTests.cpp" -o "$HARNESS/work/ref/tinyjsonTests"
echo "built: $HARNESS/work/dump  $HARNESS/work/ref/tinyjsonTests"
