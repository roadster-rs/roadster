#[cfg(feature = "grpc")]
use std::env;
#[cfg(feature = "grpc")]
use std::path::PathBuf;
use vergen::EmitBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    EmitBuilder::builder().git_sha(true).emit()?;

    #[cfg(feature = "grpc")]
    {
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
        tonic_build::configure()
            .file_descriptor_set_path(out_dir.join("helloworld_descriptor.bin"))
            .compile(&["proto/helloworld.proto"], &["proto"])
            .unwrap();
    }

    Ok(())
}
