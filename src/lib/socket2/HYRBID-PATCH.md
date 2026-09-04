socket2 0.6.5, vendored from crates.io with one change (WP-J2):
src/sys/unix.rs `set_nonblocking()` — on `target_os = "wasi"` go through
`std::net::TcpStream::set_nonblocking` instead of
`fcntl(F_GETFL/F_SETFL, O_NONBLOCK)`. Same wasi-libc defect as the mio patch:
fcntl returns EBADF for wasip2 socket fds, so hyper-util's HttpConnector
(hyper-util/src/client/legacy/connect/http.rs, "tcp set_nonblocking error")
failed on the first byte of every outbound gRPC call from a wasm guest —
recommendation dialling productcatalog through a tonic Channel. Wired in
through `[patch.crates-io]` in every service crate; drop this directory once
socket2 handles wasi.
