#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
evidence=${GLASSBOARD_LINUX_EVIDENCE:-"$root/.audit/linux-x11"}
image=${GLASSBOARD_LINUX_IMAGE:-glassboard-linux-test:local}
platform=${GLASSBOARD_LINUX_PLATFORM:-}
architecture=${platform##*/}
if [ -z "$architecture" ]; then
    architecture=$(docker info --format '{{.Architecture}}')
fi
case "$architecture" in aarch64) architecture=arm64;; x86_64) architecture=amd64;; esac
name="glassboard-linux-verify-$$"
mkdir -p "$evidence"
evidence=$(cd "$evidence" && pwd)
options=()
if [ -n "$platform" ]; then options+=(--platform "$platform"); fi

docker build "${options[@]+"${options[@]}"}" -t "$image" "$root/scripts/linux"
trap 'docker rm -f "$name" >/dev/null 2>&1 || true' EXIT
docker run --rm --name "$name" "${options[@]+"${options[@]}"}" --shm-size=1g \
    --mount "type=bind,source=$root,target=/source,readonly" \
    --mount "type=bind,source=$evidence,target=/evidence" \
    --mount "type=volume,source=glassboard-linux-cargo-registry,target=/usr/local/cargo/registry" \
    --mount "type=volume,source=glassboard-linux-cargo-git,target=/usr/local/cargo/git" \
    --mount "type=volume,source=glassboard-linux-target-$architecture,target=/target" \
    --mount "type=volume,source=glassboard-linux-npm,target=/root/.npm" \
    -e "GLASSBOARD_LINUX_DPI=${GLASSBOARD_LINUX_DPI:-96 192}" \
    "$image" bash -c '
        set -euo pipefail
        tar -C /source --exclude=.git --exclude=.audit --exclude=node_modules \
            --exclude=target --exclude=dist --exclude=.astro --exclude="._*" -cf - . \
            | tar -C /workspace -xf -
        bash scripts/linux/container.sh
    ' 2>&1 | tee "$evidence/run.log"
