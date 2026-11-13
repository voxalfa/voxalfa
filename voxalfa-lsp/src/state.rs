use async_lsp::lsp_types::Url;
use ropey::Rope;
use voxalfa_core::output::FinalOutput;

#[derive(Debug)]
pub struct Document {
    pub uri: Url,
    pub source: String,
    pub data: FinalOutput,
    pub rope: Rope,
}

impl Document {
    pub fn new(uri: Url, source: String, data: FinalOutput) -> Self {
        Self {
            rope: Rope::from_str(&source),
            uri,
            source,
            data,
        }
    }
}
