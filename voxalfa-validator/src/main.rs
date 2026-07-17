use std::error::Error;

use voxalfa_validator::{
    cli::CliReporter, ts_utils::context::TSContext, validator::DocumentValidator,
};

fn main() -> Result<(), Box<dyn Error>> {
    let source = r#"
        ;; @version 1.0

        [#] title="Hello World"
        [#] author={"Foo Bar", "Jane Doe"}
        [#] composer="Bob"
        [#] description="Lorem Ipsum."
        [#] release={2026}

        [$] key={C} | bpm={100} | time={4,4} | voices={S,T,A,B}

        ---

        [$] repeat={true}

        [^] p={1} | cre={3,6}

        [S] |d :r !m :f |`s :l` !t :- |da+1 :- !  :  ||
        [T] |d :r !m :f |`s :l` !t :- |da+1 :- !  :  ||
        [A] |d :r !m :f |`s :l` !t :- |da+1 :- !  :  ||
        [B] |d :r !m :f |`s :l` !t :- |da+1 :- !  :  ||

        [1] do `re` mi\(fa) so la ti_i do_o. ~ ~
"#;

    let mut context = TSContext::new()?;
    let validator = DocumentValidator::new(source);
    let output = validator.validate(&mut context);

    let mut cli_reporter = CliReporter::default();

    // println!("{:?}", output.resolve_column_width(true));

    cli_reporter.register("hello.solfa", source, output.diagnostics);
    cli_reporter.display_report();

    Ok(())
}
