use std::error::Error;

use vergen_gitcl::{Build, Emitter, Gitcl};

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-env-changed=NIGHTLY_BUILD");

    let build = Build::all().build_date(true).build();
    let git = Gitcl::all().sha(true).build();

    Emitter::new()
        .add_instructions(&build)?
        .add_instructions(&git)?
        .emit()?;

    Ok(())
}
