# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Fork of Google's [Online Boutique](https://github.com/GoogleCloudPlatform/microservices-demo) with several services rewritten in Rust and built two ways — native container and WASI P2 Wasm component — for comparing execution on Kubernetes.

## Build commands

Each `*serwasm` crate (`shippingserwasm`, `cartserwasm`, `recommendationserwasm`, `syntheticservicewasm`) has one binary, `server`, at `src/main.rs`, built for both targets from the same source:

```sh
cd src/<SERVICE>serwasm
cargo build --release --bin server                          # native
cargo build --release --target wasm32-wasip2 --bin server    # wasm component
```

Required toolchain: `rustup target add wasm32-wasip2`. The crate's `.cargo/config.toml` adds `-C target-feature=+crt-static` (both native targets) and `-C target-cpu=x86-64-v3` (x86_64 only, the SUT ISA — never `target-cpu=native`); inside Docker an explicit `--target` is needed so those rustflags don't land on proc-macros, which can't be statically linked. The wasm target's config adds `--cfg tokio_unstable` (tokio gates `net` on wasm behind it).

The wasm build produces a WASI P2 *command* component that binds its own TCP listener and runs long-lived under the unmodified upstream `containerd-shim-wasmtime` (RuntimeClass `wasmtime`) — no `serve.rs` host, no `--lib`/cdylib, no `wasi:http`, no `WASMTIME_*` env.

### Full cluster deploy (Skaffold)

```sh
skaffold run        # build + deploy all services
skaffold dev         # rebuild on code change
skaffold delete       # cleanup
kubectl port-forward deployment/frontend 8080:8080
```

### Test a Rust service

```sh
# shipping gRPC client against a running server
cd src/shippingservice
cargo run --bin shipping-client
```

## Images

Skaffold builds and pushes every artifact to the cluster-local registry (`build.local.push: true` — kind included, since a wasm image can't be `kind load`ed). Native images use each crate's Dockerfile: `cargo build --target <triple> --bin server --release` in a `rust:1.92-bookworm` builder, copied into a `scratch` runtime stage (static binary, `USER 65534`). Wasm images go through the custom builder `hack/wasm-image.sh`: `cargo build --target wasm32-wasip2` → `oci-tar-builder` (keeps the wasm layer media type the shim precompiles from) → `ctr images import`/`push`. Needs `oci-tar-builder` on PATH, and on kind, `KIND_NODE` set so the script can `docker cp`/`docker exec` into the node's containerd instead of using the host's.

## Architecture

One `src/main.rs` per `*serwasm` crate is a tonic 0.14 gRPC server, `#[tokio::main(flavor = "current_thread")]` on both targets (same scheduler on both sides for a fair comparison), serving via `Server::serve_with_incoming(TcpListenerStream::new(listener))` over a `TcpListener` it binds itself. `src/core.rs` holds the service logic behind traits; datastore/outbound adapters (the `redis` crate, a tonic `Channel`) are the same source on both targets. Outbound addresses (`REDIS_ADDR`, `PRODUCT_CATALOG_SERVICE_ADDR`) are resolved once at startup to an IP literal and handed to the client as such — tokio's/hyper-util's resolver needs a blocking thread, which wasip2 cannot spawn.

### Vendored dependency fixes (`src/lib/{mio,socket2,tonic}`)

Each is a vendored crate with one behavioural change for wasip2, wired into every service crate via an identical `[patch.crates-io]` table (asserted, along with identical `[profile.release]` and `.cargo/config.toml`, by `experimentes/orchestrator/deployment_symmetry.py`). One line each — see the crate's `HYRBID-PATCH.md` for the full story:

- `mio` — `TcpStream::accept()` used `fcntl(F_SETFL, O_NONBLOCK)`, which wasi-libc answers with `EBADF` for wasip2 sockets; now `set_nonblocking(true)`.
- `socket2` — same `fcntl` defect in `set_nonblocking()`, which broke hyper-util's `HttpConnector` on the first byte of every outbound dial.
- `tonic` — the `channel` feature's UDS connector was gated `not(windows)`, unconditionally pulling in `tokio::net::UnixStream`, which wasip2 doesn't have; now gated `cfg(unix)`.

### Services

| Service | Language | Notes |
|---|---|---|
| `shippingserwasm` | Rust | Shipping cost/tracking — no external deps |
| `recommendationserwasm` | Rust | Product recommendations — outbound gRPC to productcatalog via a tonic `Channel` |
| `cartserwasm` | Rust | Shopping cart — Redis via the `redis` crate |
| `syntheticservicewasm` | Rust | Benchmark-only synthetic service — compute/network/data workloads, outbound gRPC to productcatalog |
| `shippingservice` | Rust (native only) | `shipping-client` test binary only |
| All others | Go/Python/Node/Java/C# | Upstream Google microservices, unchanged |

### Kustomize overlays (`./kustomize/overlays/`)

| Overlay | Description |
|---|---|
| `baseline` | All services as containers |
| `wasi-vanilla` | shipping as Wasm |
| `wasi-grpc` | shipping + recommendation as Wasm |
| `wasi-tcp` | shipping + recommendation + cart as Wasm |
| `wasi-all` | shipping + recommendation + cart + synthetic as Wasm |
| `synthetic-baseline` | Synthetic service as container, standalone |
| `synthetic-wasm` | Synthetic service as Wasm, standalone |

Each also has a `*-with-autoscaling` variant (HPA added).

### Kustomize pairs

Each service has `with-<svc>-docker` and `with-<svc>-wasm` components under `kustomize/components/`; the `-wasm` manifest is the `-docker` manifest plus `image:` and `runtimeClassName: wasmtime` — nothing else may differ (checked by `deployment_symmetry.py`).

### Proto definitions

All gRPC service definitions in `./protos/demo.proto`. Each crate's `build.rs` calls `tonic-prost-build` with `.build_transport(false)` — the generated client would otherwise reference `tonic::transport::{Channel, Endpoint}`, which live behind the `channel` feature that only recommendation/synthetic enable (cart and shipping don't need it).

## Key dependencies

- **tonic 0.14** / **tonic-prost 0.14** — gRPC
- **tokio 1.53**
- **redis 0.32** (`tokio-comp`) — cart, synthetic
