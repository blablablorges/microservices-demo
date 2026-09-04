FROM --platform=$BUILDPLATFORM mirror.gcr.io/rust:1.92-bookworm AS builder

RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

COPY protos /build/protos
COPY src/lib /build/src/lib
COPY src/shippingserwasm/ /build/src/shippingserwasm/

WORKDIR /build/src/shippingserwasm

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
ENV PORT="50051"
EXPOSE 50051
ENTRYPOINT ["/server"]
