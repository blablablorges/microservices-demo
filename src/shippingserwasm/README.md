# shippingservice wasm

This is the wasm-ready version of the shipping-service.

Prerequisites:

* rustup: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
* wasm32-wasip2: `rustup target add wasm32-wasip2`

## deployment

`src/main.rs` is the one source built for both targets — a tonic gRPC server
that binds its own TCP listener.

Native binary:
```
cargo build --release --bin server
```
Wasm component (a WASI P2 command component, runs long-lived under
`containerd-shim-wasmtime`):
```
cargo build --release --target wasm32-wasip2 --bin server
```
