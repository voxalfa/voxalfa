use std::fs;

use voxalfa_validator::{MultiStepValidator, output::FinalOutput};

use crate::{error::Error, types::SourceFile};

pub fn read_files(file_paths: Vec<String>) -> Result<Vec<SourceFile>, Error> {
    let mut results = Vec::new();

    for path in file_paths {
        let content = fs::read_to_string(&path)?;
        let file = SourceFile { path, content };

        results.push(file);
    }

    Ok(results)
}

pub fn parse_file(content: &str) -> Result<FinalOutput, Error> {
    let mut validator = MultiStepValidator::init()?;
    let output = validator.process(content);

    Ok(output)
}
