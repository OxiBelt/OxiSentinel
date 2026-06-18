#!/usr/bin/env sh
set -eu

if ! command -v docker >/dev/null 2>&1; then
  echo "skip: docker is not available"
  exit 0
fi

IMAGE="${OXISENTINEL_IMAGE:-oxisentinel:test}"
CONTAINER="${OXISENTINEL_CONTAINER:-oxisentinel-test}"

DOCKER_BUILDKIT=1 docker build --file source/ops/Dockerfile --tag "${IMAGE}" .
docker rm --force "${CONTAINER}" >/dev/null 2>&1 || true

cleanup() {
  docker rm --force "${CONTAINER}" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

docker run --detach --name "${CONTAINER}" "${IMAGE}" >/dev/null

output="$(
  printf '%s\n' \
    '{"log":"{\"level\":\"INFO\",\"service\":\"oxibelt\",\"msg\":\"allowed\"}\n","stream":"stdout","time":"2026-06-18T10:00:00.000000000Z"}' \
    | docker exec --interactive "${CONTAINER}" oxisentinelctl parse --source auto --input -
)"

case "${output}" in
  *'"source":"docker_logs"'*'"message":"allowed"'*) ;;
  *)
    echo "unexpected parser output: ${output}" >&2
    exit 1
    ;;
esac
