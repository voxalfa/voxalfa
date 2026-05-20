use tree_sitter::Node;

use crate::ast::symbols::AssignmentData;

#[derive(Debug)]
pub struct AssignmentDataSource<'a> {
    pub key_node: Node<'a>,
    pub value_node: Node<'a>,
    pub data: AssignmentData,
}
