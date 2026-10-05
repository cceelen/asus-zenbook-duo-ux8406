#!/bin/sh
# Meson dist script: put the crates into the source archive and point cargo
# at them, so that a build from the archive needs no network.
set -eu
cd "$MESON_DIST_ROOT"
mkdir -p .cargo
cargo vendor --locked vendor > .cargo/config.toml
