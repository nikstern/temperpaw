#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "usage: $0 SERVER_IMAGE ASSET_IMAGE OUTPUT_IMAGE BUILD_SHA" >&2
  exit 2
fi

server_image=$1
asset_image=$2
output_image=$3
build_sha=$4

for image in "$server_image" "$asset_image"; do
  observed_sha=$(docker image inspect "$image" --format '{{range .Config.Env}}{{println .}}{{end}}' | sed -n 's/^BUILD_SHA=//p')
  if [[ "$observed_sha" != "$build_sha" ]]; then
    echo "$image BUILD_SHA mismatch: expected $build_sha, got $observed_sha" >&2
    exit 1
  fi
done

[[ $(docker image inspect "$server_image" --format '{{.Architecture}}') == amd64 ]]
[[ $(docker image inspect "$asset_image" --format '{{.Architecture}}') == arm64 ]]

docker build \
  --platform linux/amd64 \
  --file Dockerfile.validation-assets \
  --build-arg "SERVER_IMAGE=$server_image" \
  --build-arg "ASSET_IMAGE=$asset_image" \
  --tag "$output_image" \
  .

docker image inspect "$output_image" --format '{{.Architecture}} {{.Os}} {{.Id}} {{.Created}}'
