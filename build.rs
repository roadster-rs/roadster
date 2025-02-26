use rustc_version::{Channel, version_meta};

#[allow(clippy::disallowed_macros)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if Channel::Nightly == version_meta()?.channel {
        println!("cargo:rustc-cfg=rustc_unstable");
    }

    Ok(())
}
