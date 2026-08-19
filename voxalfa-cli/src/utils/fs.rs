use std::{fs, path::PathBuf};

use glob::glob;
use voxalfa_core::{output::FinalOutput, validation::Validator};

use crate::{
    error::{Error, Result},
    types::SourceFile,
};

pub fn read_files(file_paths: &[String]) -> Result<Vec<SourceFile>> {
    let mut results = Vec::new();

    for pattern in file_paths {
        for entry in glob(pattern)? {
            let path_buf = entry?;

            if path_buf.is_file() {
                let file = read_file(path_buf)?;

                results.push(file);
            }
        }
    }

    if results.is_empty() {
        Err(Error::NoFileMatch)
    } else {
        Ok(results)
    }
}

pub fn read_file(path_buf: PathBuf) -> Result<SourceFile> {
    let file_path = path_buf.to_string_lossy().to_string();
    let content = fs::read_to_string(&path_buf)?;

    Ok(SourceFile {
        path: file_path,
        content,
    })
}

pub fn parse_file(content: &str) -> Result<FinalOutput> {
    let mut validator = Validator::init()?;
    let output = validator.analyze(content);

    Ok(output)
}
