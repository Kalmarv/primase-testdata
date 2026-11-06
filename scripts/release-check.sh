#!/usr/bin/env bash
# Fail when the crate (src/, manifests/, examples/, Cargo.toml) differs from the crates.io release of the version
# Cargo.toml states. Consumers depend on the published crate and the manifests are compiled in, so a change that is
# not released is a change no consumer gets. A version newer than crates.io's passes: CI's release job publishes it.
set -euo pipefail

crate=primase-testdata
cd "$(dirname "$0")/.."

local_version=$(cargo metadata --no-deps --format-version 1 | jq -r --arg c "$crate" '.packages[] | select(.name == $c) | .version')
# the sparse index: one JSON line per published version, no API token or User-Agent needed
index=$(curl -fsS "https://index.crates.io/pr/im/$crate")
published=$(jq -r 'select(.yanked | not) | .vers' <<<"$index" | sort -V | tail -n 1)

if [[ "$local_version" != "$published" ]]; then
    if [[ $(printf '%s\n%s\n' "$published" "$local_version" | sort -V | tail -n 1) == "$local_version" ]]; then
        echo "$crate $local_version is newer than crates.io's $published: the release job publishes it once CI passes on main"
        exit 0
    fi
    echo "::error::$crate $local_version is older than crates.io's $published"
    exit 1
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
curl -fsSL "https://static.crates.io/crates/$crate/$crate-$published.crate" | tar -xz -C "$work"
release="$work/$crate-$published"
mv "$release/Cargo.toml.orig" "$release/Cargo.toml.ours"

changed=()
for path in src manifests examples; do
    while IFS= read -r line; do changed+=("$line"); done < <(diff -rq "$release/$path" "$path" || true)
done
cmp -s "$release/Cargo.toml.ours" Cargo.toml || changed+=("Cargo.toml differs")

if ((${#changed[@]})); then
    echo "::error::$crate $local_version is on crates.io, but the crate has changed since: bump the version, add its CHANGELOG section and publish (CONTRIBUTING.md, Releasing)"
    printf '  %s\n' "${changed[@]}"
    exit 1
fi
echo "$crate $local_version: the crate is the crates.io release"
