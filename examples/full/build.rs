#[cfg(feature = "grpc")]
use std::env;
#[cfg(feature = "grpc")]
use std::path::PathBuf;
use vergen::Emitter;
use vergen_gitcl::GitclBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gitcl = GitclBuilder::default().sha(true).build()?;
    Emitter::default().add_instructions(&gitcl)?.emit()?;

    #[cfg(feature = "grpc")]
    {
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
        tonic_prost_build::configure()
            .file_descriptor_set_path(out_dir.join("helloworld_descriptor.bin"))
            .compile_protos(&["proto/helloworld.proto"], &["proto"])
            .unwrap();
    }

    Ok(())
}
