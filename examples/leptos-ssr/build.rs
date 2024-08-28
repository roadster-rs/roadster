use vergen::Emitter;
use vergen_gitcl::GitclBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gitcl = GitclBuilder::default().sha(true).build()?;
    Emitter::default().add_instructions(&gitcl)?.emit()?;

    Ok(())
}
