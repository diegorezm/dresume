use serde::Serialize;
use typst::syntax::{LinkedNode, highlight, parse};

/// A highlighted range of Typst source.
#[derive(Debug, Clone, Serialize)]
pub struct Token {
    /// UTF-16 offsets, so they can be used directly by JavaScript editors.
    pub from: usize,
    pub to: usize,
    /// CSS class from Typst's highlighter, e.g. `typ-func`.
    pub tag: &'static str,
}

/// Highlights Typst source with the compiler's own parser. Ranges may nest
/// (e.g. a function inside a strong span) and come out in document order.
pub fn highlight_typst(text: &str) -> Vec<Token> {
    // Byte offset -> UTF-16 offset, valid at every char boundary.
    let mut utf16 = vec![0; text.len() + 1];
    let mut units = 0;
    for (i, c) in text.char_indices() {
        utf16[i] = units;
        units += c.len_utf16();
    }
    utf16[text.len()] = units;

    let root = parse(text);
    let mut tokens = Vec::new();
    collect(&LinkedNode::new(&root), &utf16, &mut tokens);
    tokens
}

fn collect(node: &LinkedNode, utf16: &[usize], out: &mut Vec<Token>) {
    if let Some(tag) = highlight(node) {
        let range = node.range();
        if !range.is_empty() {
            out.push(Token { from: utf16[range.start], to: utf16[range.end], tag: tag.css_class() });
        }
    }
    for child in node.children() {
        collect(&child, utf16, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_utf16_offsets() {
        let tokens = highlight_typst("// é😀\n#let x = 1");
        let comment = &tokens[0];
        assert_eq!((comment.from, comment.to, comment.tag), (0, 6, "typ-comment"));
        assert!(tokens.iter().any(|t| t.tag == "typ-key" && t.from == 8));
    }
}
