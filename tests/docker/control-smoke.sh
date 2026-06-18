#!/usr/bin/env sh
set -eu

if ! command -v docker >/dev/null 2>&1; then
  echo "skip: docker is not available"
  exit 0
fi

IMAGE="${OXISENTINEL_IMAGE:-oxisentinel:test}"
CONTAINER="${OXISENTINEL_CONTAINER:-oxisentinel-test}"
PARSE_OUTPUT="${TMPDIR:-/tmp}/oxisentinel-parse-$$.out"

DOCKER_BUILDKIT=1 docker build --file source/ops/Dockerfile --tag "${IMAGE}" .
docker rm --force "${CONTAINER}" >/dev/null 2>&1 || true

cleanup() {
  docker rm --force "${CONTAINER}" >/dev/null 2>&1 || true
  rm -f "${PARSE_OUTPUT}" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

docker run --detach --name "${CONTAINER}" "${IMAGE}" >/dev/null

health_output="$(docker exec --interactive "${CONTAINER}" oxisentinelctl health)"

case "${health_output}" in
  *"oxisentinel control listening on 127.0.0.1:8080"* )
    ;;
  *)
    echo "unexpected health output: ${health_output}" >&2
    exit 1
    ;;
esac

if docker exec --interactive "${CONTAINER}" oxisentinelctl parse >"${PARSE_OUTPUT}" 2>&1; then
  echo "oxisentinelctl parse should not be exposed in the runtime image" >&2
  cat "${PARSE_OUTPUT}" >&2
  exit 1
fi

if ! grep -q "unknown command: parse" "${PARSE_OUTPUT}"; then
  echo "unexpected parse rejection output:" >&2
  cat "${PARSE_OUTPUT}" >&2
  exit 1
fi
