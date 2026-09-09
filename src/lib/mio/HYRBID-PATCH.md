mio 1.2.3, vendored from crates.io with one change (WP-J1):
src/sys/unix/tcp.rs `accept()` — on `target_os = "wasi"` use
`TcpStream::set_nonblocking(true)` instead of `fcntl(F_SETFL, O_NONBLOCK)`.
wasi-libc's fcntl returns EBADF for wasip2 socket fds, so tokio's
`TcpListener::accept()` dropped every connection. Wired in through
`[patch.crates-io]` in every service crate; drop this directory once upstream
mio ships the fix.

Second change (WP-J4): src/net/tcp/stream.rs `write_vectored()` — on
`target_os = "wasi"`, when more than one buffer is non-empty, copy them into one
`Vec<u8>` and issue a single `write`. std's wasip2 `write_vectored` is
`io::default_write_vectored` (first non-empty buffer only) while tokio's
`is_write_vectored()` is true on every target, so h2 chains every DATA frame
>= 256 B and its header and payload leave in two `send()` calls; with Nagle on
the second waits for the peer's delayed ACK. Native `writev` is untouched.
