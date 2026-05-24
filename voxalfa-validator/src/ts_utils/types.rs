use tree_sitter::Node;

use crate::ts_utils::range::Range;

#[derive(Debug)]
pub struct AssignmentData<'a> {
    pub key_range: Range,
    pub key_name: String,
    pub key_node: Node<'a>,
    pub value_node: Node<'a>,
    pub value_range: Range,
}
