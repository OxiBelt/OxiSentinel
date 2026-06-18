#!/usr/bin/env sh
set -eu

IMAGE="oxisentinel:latest"
PLATFORM="linux/amd64"
TARGET_CPU="x86-64-v2"
OUTPUT="--load"

usage() {
  cat <<'USAGE'
usage: source/ops/build-image.sh [--image IMAGE] [--platform PLATFORM] [--target-cpu CPU] [--push]

supported targets:
  linux/amd64   target-cpu x86-64-v2, x86-64-v3, x86-64-v4
  linux/arm64   target-cpu generic
  linux/riscv64 target-cpu generic or riscv64gc
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --image)
      shift
      IMAGE="${1:?missing value for --image}"
      ;;
    --platform)
      shift
      PLATFORM="${1:?missing value for --platform}"
      ;;
    --target-cpu)
      shift
      TARGET_CPU="${1:?missing value for --target-cpu}"
      ;;
    --push)
      OUTPUT="--push"
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

case "${PLATFORM}:${TARGET_CPU}" in
  linux/amd64:x86-64-v2|linux/amd64:x86-64-v3|linux/amd64:x86-64-v4)
    RUST_TARGET="x86_64-unknown-linux-musl"
    RUST_TARGET_CPU="${TARGET_CPU}"
    ARTIFACT_SUFFIX="linux-amd64-${TARGET_CPU}"
    ;;
  linux/arm64:generic|linux/arm64:arm64)
    RUST_TARGET="aarch64-unknown-linux-musl"
    RUST_TARGET_CPU=""
    ARTIFACT_SUFFIX="linux-arm64-generic"
    ;;
  linux/riscv64:generic|linux/riscv64:riscv64gc)
    RUST_TARGET="riscv64gc-unknown-linux-musl"
    RUST_TARGET_CPU=""
    ARTIFACT_SUFFIX="linux-riscv64-riscv64gc"
    ;;
  *)
    echo "unsupported image target platform=${PLATFORM} target_cpu=${TARGET_CPU}" >&2
    usage >&2
    exit 2
    ;;
esac

echo "building ${IMAGE} for ${PLATFORM} (${RUST_TARGET}, artifact suffix ${ARTIFACT_SUFFIX})"

docker buildx build \
  --file source/ops/Dockerfile \
  --platform "${PLATFORM}" \
  --build-arg "RUST_TARGET=${RUST_TARGET}" \
  --build-arg "RUST_TARGET_CPU=${RUST_TARGET_CPU}" \
  --label "org.opencontainers.image.ref.name=${IMAGE}" \
  --label "org.oxisentinel.artifact.oxisentinel=oxisentinel-${ARTIFACT_SUFFIX}" \
  --label "org.oxisentinel.artifact.oxisentinelctl=oxisentinelctl-${ARTIFACT_SUFFIX}" \
  --tag "${IMAGE}" \
  ${OUTPUT} \
  .
