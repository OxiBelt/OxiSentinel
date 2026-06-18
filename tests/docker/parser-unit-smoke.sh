#!/usr/bin/env sh
set -eu

if ! command -v docker >/dev/null 2>&1; then
  echo "skip: docker is not available"
  exit 0
fi

DOCKER_BUILDKIT=1 docker build \
  --file source/ops/Dockerfile \
  --target parser-tests \
  --tag "${OXISENTINEL_PARSER_TEST_IMAGE:-oxisentinel:parser-tests}" \
  .

