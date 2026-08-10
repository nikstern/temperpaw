#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 3 || $# -gt 4 ]]; then
  echo "usage: $0 ASSET_IMAGE OUTPUT_IMAGE BUILD_SHA [BUILD_VERSION]" >&2
  exit 2
fi

asset_image=$1
output_image=$2
build_sha=$3
build_version=${4:-durable-reactions-dev}

asset_arch=$(docker image inspect "$asset_image" --format '{{.Architecture}}')
asset_build_sha=$(docker image inspect "$asset_image" --format '{{range .Config.Env}}{{println .}}{{end}}' | sed -n 's/^BUILD_SHA=//p')

if [[ "$asset_arch" != "arm64" ]]; then
  echo "asset image must be arm64; got: $asset_arch" >&2
  exit 1
fi
if [[ "$asset_build_sha" != "$build_sha" ]]; then
  echo "asset image BUILD_SHA mismatch: expected $build_sha, got $asset_build_sha" >&2
  exit 1
fi

docker build \
  --platform linux/amd64 \
  --file Dockerfile.validation-amd64 \
  --build-arg "ASSET_IMAGE=$asset_image" \
  --build-arg "BUILD_SHA=$build_sha" \
  --build-arg "BUILD_VERSION=$build_version" \
  --tag "$output_image" \
  .

output_arch=$(docker image inspect "$output_image" --format '{{.Architecture}}')
output_build_sha=$(docker image inspect "$output_image" --format '{{range .Config.Env}}{{println .}}{{end}}' | sed -n 's/^BUILD_SHA=//p')

if [[ "$output_arch" != "amd64" ]]; then
  echo "output image must be amd64; got: $output_arch" >&2
  exit 1
fi
if [[ "$output_build_sha" != "$build_sha" ]]; then
  echo "output image BUILD_SHA mismatch: expected $build_sha, got $output_build_sha" >&2
  exit 1
fi

docker image inspect "$output_image" --format '{{.Architecture}} {{.Os}} {{.Id}} {{.Created}}'
