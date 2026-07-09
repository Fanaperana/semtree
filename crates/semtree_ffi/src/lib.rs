use std::ffi::c_char;

use semtree_parser::{ParseResult, Parser};
use semtree_red::SyntaxNode;

/// Opaque handle to a parsed tree, holding ownership of the parse result.
pub struct SemTreeTree {
    _result: ParseResult,
    root: SyntaxNode,
}

/// Opaque handle to a node (borrowed from the tree).
pub struct SemTreeNode {
    node: SyntaxNode,
}

/// Parse a source string into a SemTree tree.
///
/// Returns null on failure. The caller must free the result with `semtree_tree_free`.
///
/// # Safety
///
/// `source` must point to a valid UTF-8 byte buffer of at least `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn semtree_parse(source: *const c_char, len: usize) -> *mut SemTreeTree {
    if source.is_null() {
        return std::ptr::null_mut();
    }
    let slice = unsafe { std::slice::from_raw_parts(source as *const u8, len) };
    let src = match std::str::from_utf8(slice) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = Parser::parse(src);
    let root = result.syntax();
    let tree = Box::new(SemTreeTree {
        _result: result,
        root,
    });
    Box::into_raw(tree)
}

/// Get the root node of the tree.
///
/// # Safety
///
/// `tree` must be a valid pointer returned by `semtree_parse`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn semtree_tree_root(tree: *const SemTreeTree) -> *const SemTreeNode {
    if tree.is_null() {
        return std::ptr::null();
    }
    let tree = unsafe { &*tree };
    let node = Box::new(SemTreeNode {
        node: tree.root.clone(),
    });
    Box::into_raw(node) as *const SemTreeNode
}

/// Get the syntax kind of a node (as u16).
///
/// # Safety
///
/// `node` must be a valid pointer returned by `semtree_tree_root` or `semtree_node_child`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn semtree_node_kind(node: *const SemTreeNode) -> u16 {
    if node.is_null() {
        return 0;
    }
    let node = unsafe { &*node };
    node.node.kind().0
}

/// Get the text content of a node. Writes into buf, returns number of bytes written.
/// If buf is null or buf_len is 0, returns the required buffer size.
///
/// # Safety
///
/// `node` must be a valid pointer. `buf` must point to a writable buffer of at least `buf_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn semtree_node_text(
    node: *const SemTreeNode,
    buf: *mut c_char,
    buf_len: usize,
) -> usize {
    if node.is_null() {
        return 0;
    }
    let node = unsafe { &*node };
    let text = node.node.text();
    let bytes = text.as_bytes();

    if buf.is_null() || buf_len == 0 {
        return bytes.len();
    }

    let copy_len = bytes.len().min(buf_len);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, copy_len);
    }
    copy_len
}

/// Get the number of child nodes.
///
/// # Safety
///
/// `node` must be a valid pointer returned by `semtree_tree_root` or `semtree_node_child`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn semtree_node_child_count(node: *const SemTreeNode) -> usize {
    if node.is_null() {
        return 0;
    }
    let node = unsafe { &*node };
    node.node.children().len()
}

/// Get a child node by index. Returns null if out of bounds.
/// The returned node must be freed with `semtree_node_free`.
///
/// # Safety
///
/// `node` must be a valid pointer returned by `semtree_tree_root` or `semtree_node_child`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn semtree_node_child(
    node: *const SemTreeNode,
    index: usize,
) -> *const SemTreeNode {
    if node.is_null() {
        return std::ptr::null();
    }
    let node = unsafe { &*node };
    let children = node.node.children();
    match children.into_iter().nth(index) {
        Some(child) => {
            let child_node = Box::new(SemTreeNode { node: child });
            Box::into_raw(child_node) as *const SemTreeNode
        }
        None => std::ptr::null(),
    }
}

/// Get the start offset of a node (in bytes from source start).
///
/// # Safety
///
/// `node` must be a valid pointer returned by `semtree_tree_root` or `semtree_node_child`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn semtree_node_start(node: *const SemTreeNode) -> u32 {
    if node.is_null() {
        return 0;
    }
    let node = unsafe { &*node };
    u32::from(node.node.text_range().start())
}

/// Get the end offset of a node (in bytes from source start).
///
/// # Safety
///
/// `node` must be a valid pointer returned by `semtree_tree_root` or `semtree_node_child`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn semtree_node_end(node: *const SemTreeNode) -> u32 {
    if node.is_null() {
        return 0;
    }
    let node = unsafe { &*node };
    u32::from(node.node.text_range().end())
}

/// Free a tree allocated by `semtree_parse`.
///
/// # Safety
///
/// `tree` must be a valid pointer returned by `semtree_parse`, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn semtree_tree_free(tree: *mut SemTreeTree) {
    if !tree.is_null() {
        drop(unsafe { Box::from_raw(tree) });
    }
}

/// Free a node allocated by `semtree_tree_root` or `semtree_node_child`.
///
/// # Safety
///
/// `node` must be a valid pointer returned by `semtree_tree_root` or `semtree_node_child`, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn semtree_node_free(node: *mut SemTreeNode) {
    if !node.is_null() {
        drop(unsafe { Box::from_raw(node) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_and_navigate() {
        let source = "fn main() {}";
        let tree = unsafe { semtree_parse(source.as_ptr() as *const c_char, source.len()) };
        assert!(!tree.is_null());

        let root = unsafe { semtree_tree_root(tree) };
        assert!(!root.is_null());

        let kind = unsafe { semtree_node_kind(root) };
        assert!(kind > 0);

        let child_count = unsafe { semtree_node_child_count(root) };
        assert!(child_count > 0);

        let start = unsafe { semtree_node_start(root) };
        let end = unsafe { semtree_node_end(root) };
        assert_eq!(start, 0);
        assert!(end > 0);

        let text_len = unsafe { semtree_node_text(root, std::ptr::null_mut(), 0) };
        assert!(text_len > 0);

        let mut buf = vec![0u8; text_len + 1];
        let written =
            unsafe { semtree_node_text(root, buf.as_mut_ptr() as *mut c_char, buf.len()) };
        assert_eq!(written, text_len);

        unsafe {
            semtree_node_free(root as *mut SemTreeNode);
            semtree_tree_free(tree);
        }
    }

    #[test]
    fn test_null_safety() {
        let kind = unsafe { semtree_node_kind(std::ptr::null()) };
        assert_eq!(kind, 0);

        let count = unsafe { semtree_node_child_count(std::ptr::null()) };
        assert_eq!(count, 0);

        let text_len = unsafe { semtree_node_text(std::ptr::null(), std::ptr::null_mut(), 0) };
        assert_eq!(text_len, 0);

        let child = unsafe { semtree_node_child(std::ptr::null(), 0) };
        assert!(child.is_null());

        unsafe {
            semtree_tree_free(std::ptr::null_mut());
            semtree_node_free(std::ptr::null_mut());
        }
    }
}
