use tree_sitter::Node;

use crate::{ast::symbols::ScopeId, ts_utils::range::Range};

#[derive(Debug)]
pub struct AssignmentData<'a> {
    pub scope_id: ScopeId,
    pub full_range: Range,
    pub key_range: Range,
    pub key_name: String,
    pub key_node: Node<'a>,
    pub value_node: Node<'a>,
    pub value_range: Range,
}
