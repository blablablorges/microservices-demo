# Copyright 2026

# One Rust version across every service crate of both SUTs (WP-H2).
FROM --platform=$BUILDPLATFORM mirror.gcr.io/rust:1.92-bookworm AS builder

RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

COPY protos /build/protos
COPY src/lib /build/src/lib
COPY src/cartserwasm/ /build/src/cartserwasm/

WORKDIR /build/src/cartserwasm

# An explicit --target keeps the crt-static rustflag off host units (proc-macros
# cannot be linked statically); the output then lands under target/<triple>/.
ARG TARGETARCH
RUN case "$TARGETARCH" in amd64) T=x86_64-unknown-linux-gnu ;; arm64) T=aarch64-unknown-linux-gnu ;; *) echo "unsupported TARGETARCH=$TARGETARCH" >&2; exit 1 ;; esac \
 && cargo build --bin server --release --target "$T" \
 && cp "target/$T/release/server" /server

# WP-H2: static binary (crt-static via .cargo/config.toml), no base image — the wasm image next to it has none either.
FROM scratch
COPY --from=builder /server /server
USER 65534
ENV PORT="7070"
ENV REDIS_ADDR="redis-cart:6379"
EXPOSE 7070
ENTRYPOINT ["/server"]
