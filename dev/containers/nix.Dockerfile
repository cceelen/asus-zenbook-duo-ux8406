# syntax=docker/dockerfile:1
# Nix: build the packages of the flake. Nix builds into its store, so there
# are no package files to take out; the image keeps the results as links:
# /second-screen, /tcc-guard, /keyboard-bpf and /gnome-keys.
#
#   docker build -f dev/containers/nix.Dockerfile -t zbd-nix .
#
# Not offline: Nix fetches nixpkgs and the build tools from cache.nixos.org
# (so no --network=none here). The derivations themselves need no network:
# the crates are in the store from Cargo.lock. Whether the build sandbox is
# on is stated in the build log below.

FROM docker.io/nixos/nix:latest
COPY . /build
WORKDIR /build
# path: so that Nix sees every file of the tree, tracked by git or not.
RUN set -eux; \
    id; grep -E '^(sandbox|build-users-group)' /etc/nix/nix.conf || true; \
    nix --extra-experimental-features 'nix-command flakes' show-config | grep -E '^(sandbox|build-users-group) '; \
    for p in second-screen:asus-zenbook-duo-ux8406-second-screen tcc-guard:asus-zenbook-duo-ux8406-tcc-guard \
             keyboard-bpf:asus-zenbook-duo-ux8406-keyboard-bpf \
             gnome-keys:gnome-shell-extension-asus-zenbook-duo-ux8406-keys; do \
        nix --extra-experimental-features 'nix-command flakes' build -L \
            --print-build-logs --out-link "/${p%%:*}" "path:/build#${p#*:}"; \
    done
