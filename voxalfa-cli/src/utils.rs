use std::fs;

use glob::glob;
use voxalfa_validator::{MultiStepValidator, output::FinalOutput};

use crate::{error::Error, types::SourceFile};

pub fn read_files(file_paths: Vec<String>) -> Result<Vec<SourceFile>, Error> {
    let mut results = Vec::new();

    for pattern in file_paths {
        for entry in glob(&pattern)? {
            let path_buf = entry?;

            if path_buf.is_file() {
                let path = path_buf.to_string_lossy().to_string();
                let content = fs::read_to_string(&path_buf)?;

                results.push(SourceFile { path, content });
            }
        }
    }

    Ok(results)
}

pub fn parse_file(content: &str) -> Result<FinalOutput, Error> {
    let mut validator = MultiStepValidator::init()?;
    let output = validator.process(content);

    Ok(output)
}
