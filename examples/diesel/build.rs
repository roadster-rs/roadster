use vergen::Emitter;
use vergen_gitcl::Gitcl;

#[allow(clippy::disallowed_macros)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gitcl = Gitcl::builder().sha(true).build();
    Emitter::default().add_instructions(&gitcl)?.emit()?;

    // https://docs.rs/diesel_migrations/2.2.0/diesel_migrations/macro.embed_migrations.html#automatic-rebuilds
    println!("cargo:rerun-if-changed=migrations");

    Ok(())
}
