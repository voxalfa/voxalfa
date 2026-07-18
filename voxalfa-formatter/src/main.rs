use std::error::Error;

use voxalfa_formatter::Formatter;
use voxalfa_validator::ts_utils::context::TSContext;

fn main() -> Result<(), Box<dyn Error>> {
    let source = r#"
        ;; @version 1.0

        [#] title="Hello World" ; inline comment
        [#] author={"Foo Bar", "Jane Doe"}
        [#] composer="Bob"
        [#] release={2026}
        [#] description="Lorem Ipsum."

        [$] key={C} | bpm={100} | time={4,4} | voices={S,T,A,B}

        ---

        [$] repeat={true}

        [^] p={1} | cre={3,6}

        [S] |d:r!m :f ||
        [T] |d :r !m :f ||
        [A] |d :r !m :f ||
        [B] |d :r !m :f ||
        [1] do w`re` mi\(fa)

"#;

    let mut ts_context = TSContext::new()?;
    let output = Formatter::default()
        .format(source, &mut ts_context)
        .unwrap();

    println!("{output}");

    Ok(())
}
