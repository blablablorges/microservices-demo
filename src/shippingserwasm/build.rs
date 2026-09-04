use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Resolves both locally and inside the Docker build context.
    let (proto, dir) = if Path::new("../../protos/demo.proto").exists() {
        ("../../protos/demo.proto", "../../protos")
    } else {
        ("protos/demo.proto", "protos")
    };

    // build_transport(false): the generated clients would reference
    // tonic::transport::{Channel, Endpoint}, which live behind the `channel`
    // feature this crate cannot enable on wasip2.
    tonic_prost_build::configure()
        .build_transport(false)
        .compile_protos(&[proto], &[dir])?;
    Ok(())
}
