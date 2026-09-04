# Copyright 2024 Google LLC
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

FROM --platform=$BUILDPLATFORM mirror.gcr.io/rust:1.92-bookworm AS builder

RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

COPY protos /build/protos
COPY src/lib /build/src/lib
COPY src/recommendationserwasm/ /build/src/recommendationserwasm/

WORKDIR /build/src/recommendationserwasm

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
ENV PORT="8080"
EXPOSE 8080
ENTRYPOINT ["/server"]
