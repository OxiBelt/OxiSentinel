#!/usr/bin/env sh
set -eu

if ! command -v docker >/dev/null 2>&1; then
  echo "skip: docker is not available"
  exit 0
fi

if ! command -v journalctl >/dev/null 2>&1; then
  echo "skip: journalctl is not available"
  exit 0
fi

if ! docker info --format '{{json .Plugins.Log}}' 2>/dev/null | grep -q '"journald"'; then
  echo "skip: Docker journald log driver is not available"
  exit 0
fi

IMAGE="${OXISENTINEL_IMAGE:-oxisentinel:test}"
CONTAINER="${OXISENTINEL_CONTAINER:-oxisentinel-journald-test}"

DOCKER_BUILDKIT=1 docker build --file source/ops/Dockerfile --tag "${IMAGE}" .
docker rm --force "${CONTAINER}" >/dev/null 2>&1 || true

cleanup() {
  docker rm --force "${CONTAINER}" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

docker run \
  --detach \
  --log-driver journald \
  --name "${CONTAINER}" \
  "${IMAGE}" >/dev/null

sleep 1

journal_output="$(
  journalctl --output json CONTAINER_NAME="${CONTAINER}" --lines 5 --no-pager 2>/dev/null || true
)"

if [ -z "${journal_output}" ]; then
  echo "skip: no journald records visible for ${CONTAINER}"
  exit 0
fi

output="$(
  printf '%s\n' "${journal_output}" \
    | docker run --rm --interactive --entrypoint /usr/local/bin/oxisentinelctl "${IMAGE}" parse --source auto --input -
)"

case "${output}" in
  *'"source":"docker_journald"'*) ;;
  *)
    echo "unexpected journald parser output: ${output}" >&2
    exit 1
    ;;
esac
