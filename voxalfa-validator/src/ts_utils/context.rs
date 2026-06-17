use tree_sitter::{Language, Parser, Query, Tree};

use crate::error::Result;

const QUERY: &str = r"
    (ERROR) @error.syntax
    (MISSING) @error.missing
    (inline_comment) @comment
    (language_directive) @directive
";

pub struct TSContext {
    pub parser: Parser,
    pub language: Language,
    pub query: Query,
}

impl TSContext {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        let language = tree_sitter_voxalfa::LANGUAGE.into();
        let query = Query::new(&language, QUERY)?;

        parser.set_language(&language)?;

        Ok(Self {
            parser,
            language,
            query,
        })
    }

    pub fn parse(&mut self, source: &[u8]) -> Option<Tree> {
        self.parser.parse(source, None)
    }
}
