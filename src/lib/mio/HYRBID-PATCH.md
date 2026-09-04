mio 1.2.3, vendored from crates.io with one change (WP-J1):
src/sys/unix/tcp.rs `accept()` — on `target_os = "wasi"` use
`TcpStream::set_nonblocking(true)` instead of `fcntl(F_SETFL, O_NONBLOCK)`.
wasi-libc's fcntl returns EBADF for wasip2 socket fds, so tokio's
`TcpListener::accept()` dropped every connection. Wired in through
`[patch.crates-io]` in every service crate; drop this directory once upstream
mio ships the fix.
