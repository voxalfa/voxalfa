use tree_sitter::{Language, Parser, Query, Tree};

use crate::error::Result;

const ERROR_QUERY: &str = r"
    (ERROR) @error.syntax
    (MISSING) @error.missing
";

pub struct TSContext {
    pub parser: Parser,
    pub language: Language,
    pub error_query: Query,
}

impl TSContext {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        let language = tree_sitter_voxalfa::LANGUAGE.into();
        let error_query = Query::new(&language, ERROR_QUERY)?;

        parser.set_language(&language)?;

        Ok(Self {
            parser,
            language,
            error_query,
        })
    }

    pub fn parse(&mut self, source: &[u8]) -> Option<Tree> {
        self.parser.parse(source, None)
    }
}
