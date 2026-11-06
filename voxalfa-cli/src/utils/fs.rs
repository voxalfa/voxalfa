use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

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

pub fn write_file<P, B>(path: P, content: B) -> Result<()>
where
    P: AsRef<Path>,
    B: AsRef<[u8]>,
{
    File::options()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path.as_ref())?
        .write_all(content.as_ref())?;

    Ok(())
}
