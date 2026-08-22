use vergen_git2::{Build, Cargo, Emitter, Git2, Rustc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Emitter::default()
        .add_instructions(&Build::all_build())?
        .add_instructions(&Cargo::all_cargo())?
        .add_instructions(&Git2::all_git())?
        .add_instructions(&Rustc::all_rustc())?
        .emit()?;
    Ok(())
}
