use voxalfa_validator::MultiStepValidator;

use crate::Formatter;

pub fn assert_formatted_snapshot(source_name: &str, content: &str) {
    let mut validator = MultiStepValidator::init().unwrap();
    let output = validator.analyze(content);

    assert!(!output.has_error(), "{:?}", output.diagnostics);

    let formatter = Formatter::new(&output);
    let mut buffer = Vec::new();

    formatter.format(&mut buffer).unwrap();

    let output = format!(
        "FILE: {}\n\n=== SOURCE ===\n{}\n\n=== FORMATTED ===\n{}",
        source_name,
        content,
        String::from_utf8(buffer).unwrap()
    );

    insta::assert_snapshot!(source_name, output)
}
