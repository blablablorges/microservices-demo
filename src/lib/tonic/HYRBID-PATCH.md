tonic 0.14.6, vendored from crates.io with one change (WP-J2):
src/transport/channel/uds_connector.rs — the cfg predicates that pick the real
`tokio::net::UnixStream` connector vs. the never-used dummy were
`not(target_os = "windows")` / `target_os = "windows"`; they are now
`unix` / `not(unix)`. wasm32-wasip2 is neither unix nor windows, so as
published the `channel` feature pulls `tokio::net::UnixStream` in on wasip2,
where tokio has no UDS, and the crate does not compile. tonic's server half
already gates its UDS code with `#[cfg(unix)]` (src/transport/server/mod.rs);
this makes the client half match. Windows behaviour is unchanged (still the
dummy `DuplexStream`), only the error string is target-neutral now.
Recommendation needs `channel` for its outbound gRPC call to productcatalog.
Wired in through `[patch.crates-io]`; drop this directory once upstream tonic
ships the fix.
