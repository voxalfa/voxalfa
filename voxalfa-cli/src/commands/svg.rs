use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use clap::Parser;
use voxalfa_svg::render_pages_with;

use crate::{
    error::Result,
    reporter::CliReporter,
    utils::fs::{parse_file, read_files},
};

#[derive(Parser)]
pub struct SvgParams {
    /// Absolute file paths or patterns
    files: Vec<String>,
    #[clap(short, long)]
    output: Option<String>,
}

pub fn execute(params: SvgParams) -> Result<()> {
    let files = read_files(&params.files)?;
    let mut reporter = CliReporter::new(1);

    for file in &files {
        let output = parse_file(&file.content)?;

        if output.has_error() {
            reporter.register_diagnostics(file, output.diagnostics);
        } else {
            let pages = render_pages_with(&output, Default::default())?;
            let file_path = Path::new(&file.path);
            let default_path = file_path.with_extension("svg");
            let base_path = match params.output.as_ref().map(PathBuf::from) {
                Some(path) if path.extension().is_some() => path,
                Some(path) => path.join(
                    default_path
                        .file_name()
                        .expect("file path should have a name"),
                ),
                None => default_path.clone(),
            };

            for (index, page) in pages.iter().enumerate() {
                let output_path = page_path(&base_path, index, pages.len());

                File::options()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(output_path)?
                    .write_all(page.as_bytes())?;
            }
        }
    }

    reporter.finalize();

    Ok(())
}

fn page_path(path: &Path, index: usize, page_count: usize) -> PathBuf {
    if page_count == 1 {
        return path.to_path_buf();
    }

    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("page");
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("svg");

    path.with_file_name(format!("{stem}-{}.{extension}", index + 1))
}
