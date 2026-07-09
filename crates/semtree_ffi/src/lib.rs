use std::ffi::c_char;

use semtree_grammar::parse_semtree_dsl;
use semtree_parser::Parser;
use semtree_red::SyntaxNode;
use semtree_runtime::{ParseSession, ParserBackend, UnifiedParseResult};

/// Opaque handle to a parsed tree, holding ownership of the parse result.
pub struct SemTreeTree {
    _green: semtree_green::GreenNode,
    root: SyntaxNode,
    error_count: usize,
}

/// Opaque handle to a grammar-driven incremental parse session.
pub struct SemTreeSession {
    session: ParseSession,
}

/// Opaque handle to a node (borrowed from the tree).
pub struct SemTreeNode {
    node: SyntaxNode,
}

fn tree_from_unified(result: UnifiedParseResult) -> SemTreeTree {
    let root = result.syntax;
    let error_count = result.errors.len();
    SemTreeTree {
        _green: result.green_tree,
        root,
        error_count,
    }
}

fn cstr_from_ptr<'a>(ptr: *const c_char, len: usize) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) };
    std::str::from_utf8(slice).ok()
}

// ── Legacy API (built-in Rust parser) ────────────────────────────────────────

/// Parse a source string into a SemTree tree.
///
/// Returns null on failure. The caller must free the result with `semtree_tree_free`.
///
/// # Safety
///
/// `source` must point to a valid UTF-8 byte buffer of at least `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn semtree_parse(source: *const c_char, len: usize) -> *mut SemTreeTree {
    let Some(src) = cstr_from_ptr(source, len) else {
        return std::ptr::null_mut();
    };

    let result = Parser::parse(src);
    let root = result.syntax();
    let tree = SemTreeTree {
        _green: result.green_tree,
        root,
        error_count: 0,
    };
    Box::into_raw(Box::new(tree))
}

// ── Session API (grammar-driven incremental parsing) ───────────────────────

/// Create a parse session from a SemTree DSL grammar string.
///
/// `backend`: 0 = auto, 1 = recursive descent, 2 = GLR.
///
/// # Safety
///
/// `grammar_source` must point to a valid UTF-8 buffer of at least `grammar_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn semtree_session_create(
    grammar_source: *const c_char,
    grammar_len: usize,
    backend: u8,
) -> *mut SemTreeSession {
    let Some(src) = cstr_from_ptr(grammar_source, grammar_len) else {
        return std::ptr::null_mut();
    };
    let grammar = match parse_semtree_dsl(src) {
        Ok(g) => g,
        Err(_) => return std::ptr::null_mut(),
    };
    let backend = match backend {
        1 => ParserBackend::RecursiveDescent,
        2 => ParserBackend::Glr,
        _ => ParserBackend::Auto,
    };
    let session = ParseSession::new(grammar, backend);
    Box::into_raw(Box::new(SemTreeSession { session }))
}

/// Parse source in a session (full parse, resets incremental state).
///
/// # Safety
///
/// `session` must be valid. `source` must be a valid UTF-8 buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn semtree_session_parse(
    session: *mut SemTreeSession,
    source: *const c_char,
    len: usize,
) -> *mut SemTreeTree {
    if session.is_null() {
        return std::ptr::null_mut();
    }
    let Some(src) = cstr_from_ptr(source, len) else {
        return std::ptr::null_mut();
    };
    let session = unsafe { &mut *session };
    let result = session.session.parse(src);
    Box::into_raw(Box::new(tree_from_unified(result)))
}

/// Apply an edit and incrementally reparse.
///
/// # Safety
///
/// `session` must be valid. `new_text` must be a valid UTF-8 buffer of `new_text_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn semtree_session_edit(
    session: *mut SemTreeSession,
    start: u32,
    old_end: u32,
    new_text: *const c_char,
    new_text_len: usize,
) -> *mut SemTreeTree {
    if session.is_null() {
        return std::ptr::null_mut();
    }
    let new_text_str = if new_text.is_null() || new_text_len == 0 {
        ""
    } else {
        match cstr_from_ptr(new_text, new_text_len) {
            Some(s) => s,
            None => return std::ptr::null_mut(),
        }
    };

    let session = unsafe { &mut *session };
    let result = session.session.edit(start, old_end, new_text_str);
    Box::into_raw(Box::new(tree_from_unified(result)))
}

/// Free a session created by `semtree_session_create`.
///
/// # Safety
///
/// `session` must be null or a pointer returned by `semtree_session_create`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn semtree_session_free(session: *mut SemTreeSession) {
    if !session.is_null() {
        drop(unsafe { Box::from_raw(session) });
    }
}

/// Get the number of parse errors in the last tree.
///
/// # Safety
///
/// `tree` must be a valid pointer returned by a parse function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn semtree_tree_error_count(tree: *const SemTreeTree) -> usize {
    if tree.is_null() {
        return 0;
    }
    unsafe { (*tree).error_count }
}

// ── Tree / node navigation ──────────────────────────────────────────────────

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

    const MINI_GRAMMAR: &str = r#"
language test
keyword fn
Function := "fn" Identifier "(" ")" "{" "}"
"#;

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

        unsafe {
            semtree_node_free(root as *mut SemTreeNode);
            semtree_tree_free(tree);
        }
    }

    #[test]
    fn test_session_incremental() {
        let grammar = MINI_GRAMMAR;
        let session =
            unsafe { semtree_session_create(grammar.as_ptr() as *const c_char, grammar.len(), 1) };
        assert!(!session.is_null());

        let source = "fn foo() {}";
        let tree = unsafe {
            semtree_session_parse(session, source.as_ptr() as *const c_char, source.len())
        };
        assert!(!tree.is_null());

        let tree2 =
            unsafe { semtree_session_edit(session, 6, 6, "x".as_ptr() as *const c_char, 1) };
        assert!(!tree2.is_null());
        // Incremental reparse may report partial errors from the edited region.
        assert!(unsafe { semtree_tree_error_count(tree2) } <= 1);

        unsafe {
            semtree_tree_free(tree);
            semtree_tree_free(tree2);
            semtree_session_free(session);
        }
    }

    #[test]
    fn test_null_safety() {
        let kind = unsafe { semtree_node_kind(std::ptr::null()) };
        assert_eq!(kind, 0);

        unsafe {
            semtree_tree_free(std::ptr::null_mut());
            semtree_node_free(std::ptr::null_mut());
            semtree_session_free(std::ptr::null_mut());
        }
    }
}
