use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Resolves both locally and inside the Docker build context.
    let (proto, dir) = if Path::new("../../protos/demo.proto").exists() {
        ("../../protos/demo.proto", "../../protos")
    } else {
        ("protos/demo.proto", "protos")
    };

    // build_transport(false): the generated client's connect() shortcut is not
    // used — main.rs builds the Channel from an Endpoint itself.
    tonic_prost_build::configure()
        .build_transport(false)
        .compile_protos(&[proto], &[dir])?;
    Ok(())
}
