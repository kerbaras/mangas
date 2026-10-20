//! Generates the MANGA Plus protobuf types from `proto/mangaplus.proto` with `protox` (a pure
//! Rust compiler, so no `protoc` is needed) and `prost-build`.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/mangaplus.proto");
    let descriptors = protox::compile(["mangaplus.proto"], ["proto"])?;
    prost_build::Config::new().compile_fds(descriptors)?;
    Ok(())
}
