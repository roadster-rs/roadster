use vergen::Emitter;
use vergen_gitcl::Gitcl;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gitcl = Gitcl::builder().sha(true).build();
    Emitter::default().add_instructions(&gitcl)?.emit()?;

    Ok(())
}
