fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().build_client(false).compile(
        &["proto/endervision.proto", "proto/weaver.proto", "proto/acrobat.proto"],
        &["proto"]
    )?;
    Ok(())
}
