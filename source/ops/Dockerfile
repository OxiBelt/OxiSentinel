# syntax=docker/dockerfile:1.7

ARG RUST_VERSION=1.96.0

FROM --platform=$BUILDPLATFORM rust:${RUST_VERSION}-alpine AS build

WORKDIR /workspace

RUN apk add --no-cache musl-dev

COPY Cargo.toml Cargo.lock ./
COPY source/Cargo.toml source/Cargo.toml
COPY source/src source/src
COPY fuzz/Cargo.toml fuzz/Cargo.toml
COPY fuzz/fuzz_targets fuzz/fuzz_targets

ARG RUST_TARGET=x86_64-unknown-linux-musl
ARG RUST_TARGET_CPU=

RUN rustup target add "${RUST_TARGET}"
RUN if [ -n "${RUST_TARGET_CPU}" ]; then \
      export RUSTFLAGS="-C target-cpu=${RUST_TARGET_CPU}"; \
    fi; \
    cargo build --locked --release --target "${RUST_TARGET}" \
      -p oxisentinel \
      --bin oxisentinel \
      --bin oxisentinelctl

FROM build AS parser-tests

RUN cargo test --locked -p oxisentinel parser

FROM scratch

ARG RUST_TARGET=x86_64-unknown-linux-musl

COPY --from=build /workspace/target/${RUST_TARGET}/release/oxisentinel /usr/local/bin/oxisentinel
COPY --from=build /workspace/target/${RUST_TARGET}/release/oxisentinelctl /usr/local/bin/oxisentinelctl

LABEL org.opencontainers.image.title="OxiSentinel"
LABEL org.opencontainers.image.description="Container-only OxiSentinel analyzer runtime and control utility."

ENV PATH="/usr/local/bin:/usr/bin:/bin"

ENTRYPOINT ["/usr/local/bin/oxisentinel"]
