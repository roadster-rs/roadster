#[cfg(feature = "grpc")]
use std::env;
#[cfg(feature = "grpc")]
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
