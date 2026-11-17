use std::{
    ops::{Deref, Range},
    sync::Arc,
};

use gix::error::{OptionExt, Result, ResultExt, bail, message};
use tree_sitter::Node;

use super::Language;

#[derive(Debug)]
pub(super) struct Content {
    source: Arc<str>,
    range: Range<usize>,
    body: Range<usize>,
}

impl Content {
    fn new(source: &Arc<str>, range: Range<usize>) -> Self {
        Self {
            source: Arc::clone(source),
            body: range.end..range.end,
            range,
        }
    }

    pub(super) fn same_shell(&self, other: &Self) -> bool {
        self.source[self.range.start..self.body.start] == other.source[other.range.start..other.body.start]
            && self.source[self.body.end..self.range.end] == other.source[other.body.end..other.range.end]
    }
}

impl Deref for Content {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.source[self.range.clone()]
    }
}

impl PartialEq for Content {
    fn eq(&self, other: &Self) -> bool {
        self.deref() == other.deref()
    }
}

#[cfg(test)]
impl From<&str> for Content {
    fn from(text: &str) -> Self {
        Self::new(&Arc::from(text), 0..text.len())
    }
}

#[derive(Debug)]
pub(super) struct Item {
    pub kind: &'static str,
    pub name: String,
    pub header: String,
    pub footer: String,
    pub content: Content,
    pub children: Vec<Item>,
    pub test_only: bool,
    pub contains_tests: bool,
}

pub(super) fn parse(source: &str, language: Language) -> Result<Vec<Item>> {
    let source: Arc<str> = Arc::from(source);
    let grammar = match language {
        Language::Rust => tree_sitter_rust::LANGUAGE,
        Language::Toml => tree_sitter_toml_ng::LANGUAGE,
        Language::Markdown => tree_sitter_md::LANGUAGE,
    };
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&grammar.into())
        .or_raise(|| message("Could not load the structural diff grammar"))?;
    let tree = parser
        .parse(source.as_bytes(), None)
        .ok_or_raise(|| message("Could not parse the structural diff input"))?;
    if tree.root_node().has_error() {
        bail!(message(
            "Input contains syntax errors; refusing to display potentially non-structural code"
        ));
    }
    Ok(match language {
        Language::Rust => rust_children(tree.root_node(), &source),
        Language::Toml => toml_children(tree.root_node(), &source),
        Language::Markdown => markdown(tree.root_node(), &source),
    })
}

fn rust_children(parent: Node<'_>, source: &Arc<str>) -> Vec<Item> {
    let mut items = Vec::new();
    let mut attributes = Vec::new();
    let tuple = parent.kind() == "ordered_field_declaration_list";
    let mut visibility = None;
    let test_only = inner_test_only(parent, source);
    let mut cursor = parent.walk();
    for node in parent.named_children(&mut cursor) {
        if node.kind() == "attribute_item" {
            attributes.push(node);
            continue;
        }
        if tuple && node.kind() == "visibility_modifier" {
            visibility = Some(node);
            continue;
        }
        if matches!(node.kind(), "line_comment" | "block_comment") {
            continue;
        }
        let mut item = if tuple {
            tuple_field(node, visibility.take(), source)
        } else {
            let Some(item) = rust_item(node, source) else {
                continue;
            };
            item
        };
        item.test_only |= test_only || attributes.iter().any(|attribute| test_attribute(*attribute, source));
        if let Some(first) = attributes.first() {
            item.content.range = first.start_byte()..node.end_byte();
            item.header = attributes
                .iter()
                .map(|attribute| indented(source, *attribute, attribute.byte_range()))
                .chain(std::iter::once(item.header))
                .collect::<Vec<_>>()
                .join("\n");
            attributes.clear();
        }
        item.contains_tests = item.test_only || item.children.iter().any(|child| child.contains_tests);
        items.push(item);
    }
    items
}

fn tuple_field(node: Node<'_>, visibility: Option<Node<'_>>, source: &Arc<str>) -> Item {
    let start = visibility.map_or(node.start_byte(), |visibility| visibility.start_byte());
    let mut header = indented(source, node, start..node.end_byte());
    if source.as_bytes().get(node.end_byte()) == Some(&b',') {
        header.push(',');
    }
    Item {
        kind: "field",
        // Tuple fields are positional; filtering must not retain test-field indices.
        name: String::new(),
        header,
        footer: String::new(),
        content: Content::new(source, start..node.end_byte()),
        children: Vec::new(),
        test_only: false,
        contains_tests: false,
    }
}

fn inner_test_only(parent: Node<'_>, source: &str) -> bool {
    let mut cursor = parent.walk();
    parent
        .named_children(&mut cursor)
        .any(|node| node.kind() == "inner_attribute_item" && test_attribute(node, source))
}

fn test_attribute(node: Node<'_>, source: &str) -> bool {
    let Some(attribute) = node.named_child(0) else {
        return false;
    };
    let Some(path) = attribute.named_child(0) else {
        return false;
    };
    let path = text(source, path);
    if matches!(path.rsplit("::").next(), Some("test" | "bench")) {
        return true;
    }
    if path != "cfg" {
        return false;
    }
    let Some(arguments) = attribute.child_by_field_name("arguments") else {
        return false;
    };
    let mut cursor = arguments.walk();
    let tokens: Vec<_> = arguments
        .named_children(&mut cursor)
        .filter(|node| !matches!(node.kind(), "line_comment" | "block_comment"))
        .collect();
    cfg_without_tests(&tokens, source) == Some(false)
}

// Unknown platform and feature predicates must not hide potentially production code.
fn cfg_without_tests(tokens: &[Node<'_>], source: &str) -> Option<bool> {
    if let [name] = tokens {
        return (text(source, *name) == "test").then_some(false);
    }
    let [name, arguments] = tokens else {
        return None;
    };
    if name.kind() != "identifier" || arguments.kind() != "token_tree" {
        return None;
    }
    let mut cursor = arguments.walk();
    let children: Vec<_> = arguments
        .children(&mut cursor)
        .filter(|node| !matches!(node.kind(), "line_comment" | "block_comment"))
        .collect();
    let children = children.get(1..children.len().saturating_sub(1))?;
    let mut values = children
        .split(|node| node.kind() == ",")
        .filter(|tokens| !tokens.is_empty())
        .map(|tokens| cfg_without_tests(tokens, source));
    match text(source, *name) {
        "all" | "any" => {
            let decisive = text(source, *name) == "any";
            let mut known = true;
            for value in values {
                match value {
                    Some(value) if value == decisive => return Some(decisive),
                    None => known = false,
                    _ => {}
                }
            }
            known.then_some(!decisive)
        }
        "not" => {
            let value = values.next()?;
            if values.next().is_some() {
                return None;
            }
            value.map(|value| !value)
        }
        _ => None,
    }
}

fn rust_item(node: Node<'_>, source: &Arc<str>) -> Option<Item> {
    let kind = match node.kind() {
        "function_item" | "function_signature_item" => "function",
        "struct_item" => "struct",
        "union_item" => "union",
        "enum_item" => "enum",
        "enum_variant" => "variant",
        "field_declaration" => "field",
        "impl_item" => "impl",
        "trait_item" => "trait",
        "mod_item" => "module",
        "const_item" => "const",
        "static_item" => "static",
        "type_item" | "associated_type" => "type",
        "use_declaration" => "use",
        "macro_definition" | "macro_invocation" => "macro",
        "extern_crate_declaration" | "foreign_mod_item" => "extern",
        "inner_attribute_item" => "attribute",
        _ => return None,
    };
    let name = node
        .child_by_field_name("name")
        .or_else(|| node.child_by_field_name("macro"))
        .map_or_else(
            || match kind {
                "impl" => ["trait", "type"]
                    .into_iter()
                    .filter_map(|field| node.child_by_field_name(field))
                    .map(|part| sanitized(source, part, part.byte_range()))
                    .collect::<Vec<_>>()
                    .join(" for "),
                "extern" => sanitized(
                    source,
                    node,
                    node.start_byte()
                        ..node
                            .child_by_field_name("body")
                            .map_or(node.end_byte(), |body| body.start_byte()),
                )
                .trim_end()
                .to_owned(),
                _ => sanitized(source, node, node.byte_range()),
            },
            |name| text(source, name).to_owned(),
        );
    let mut item = Item {
        kind,
        name,
        header: String::new(),
        footer: String::new(),
        content: Content::new(source, node.byte_range()),
        children: Vec::new(),
        test_only: false,
        contains_tests: false,
    };
    if node.kind() == "macro_definition" {
        let name = node.child_by_field_name("name")?;
        item.header = indented(source, node, node.start_byte()..name.end_byte()) + " { … }";
    } else if let Some(body) = node.child_by_field_name("body") {
        item.test_only = inner_test_only(body, source);
        if kind == "function" {
            item.header = indented(source, node, node.start_byte()..body.start_byte()) + "{ … }";
        } else {
            item.content.body = body.start_byte() + 1..body.end_byte() - 1;
            item.header = indented(source, node, node.start_byte()..body.start_byte() + 1);
            let closing_prefix = &source[line_start(source, body.end_byte() - 1)..body.end_byte() - 1];
            item.footer = format!(
                "{}{}",
                if closing_prefix.chars().all(char::is_whitespace) {
                    closing_prefix.to_owned()
                } else {
                    indentation(source, node)
                },
                sanitized(source, node, body.end_byte() - 1..node.end_byte())
            );
            item.children = rust_children(body, source);
        }
    } else {
        item.header = indented(source, node, node.byte_range());
    }
    if matches!(kind, "field" | "variant") && source.as_bytes().get(node.end_byte()) == Some(&b',') {
        if item.footer.is_empty() {
            item.header.push(',');
        } else {
            item.footer.push(',');
        }
    }
    Some(item)
}

fn toml_children(parent: Node<'_>, source: &Arc<str>) -> Vec<Item> {
    let mut cursor = parent.walk();
    parent
        .named_children(&mut cursor)
        .filter_map(|node| {
            let table = matches!(node.kind(), "table" | "table_array_element");
            if !table && node.kind() != "pair" {
                return None;
            }
            let key = node.named_child(0)?;
            let header = if table {
                let close = if node.kind() == "table" { "]" } else { "]]" };
                let end = source[key.end_byte()..node.end_byte()].find(close)? + key.end_byte() + close.len();
                format!("{}{}", indentation(source, node), &source[node.start_byte()..end])
            } else {
                let value = node.named_child(1)?;
                format!(
                    "{}{}…",
                    indentation(source, node),
                    &source[node.start_byte()..value.start_byte()]
                )
            };
            Some(Item {
                kind: if table { "table" } else { "key" },
                name: text(source, key).to_owned(),
                header,
                footer: String::new(),
                content: Content::new(source, node.byte_range()),
                children: if table { toml_children(node, source) } else { Vec::new() },
                test_only: false,
                contains_tests: false,
            })
        })
        .collect()
}

fn markdown(root: Node<'_>, source: &Arc<str>) -> Vec<Item> {
    let mut headings = Vec::new();
    markdown_headings(root, &mut headings);
    let mut items = Vec::new();
    let preamble = &source[..headings
        .first()
        .map_or(source.len(), |heading| line_start(source, heading.start_byte()))];
    if !preamble.trim().is_empty() {
        items.push(Item {
            kind: "document",
            name: "(document)".into(),
            header: "(document)".into(),
            footer: String::new(),
            content: Content::new(source, 0..preamble.len()),
            children: Vec::new(),
            test_only: false,
            contains_tests: false,
        });
    }
    items.extend(markdown_items(&headings, source, source.len()));
    items
}

fn markdown_headings<'a>(node: Node<'a>, headings: &mut Vec<Node<'a>>) {
    if matches!(node.kind(), "atx_heading" | "setext_heading") {
        headings.push(node);
    } else {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            markdown_headings(child, headings);
        }
    }
}

fn markdown_items(mut headings: &[Node<'_>], source: &Arc<str>, end: usize) -> Vec<Item> {
    let mut items = Vec::new();
    while let Some((&heading, tail)) = headings.split_first() {
        let next = tail
            .iter()
            .position(|next| heading_level(*next) <= heading_level(heading))
            .map_or(headings.len(), |position| position + 1);
        let section_end = headings
            .get(next)
            .map_or(end, |next| line_start(source, next.start_byte()));
        let mut cursor = heading.walk();
        let header_end = heading
            .named_children(&mut cursor)
            .filter(|child| child.kind() != "block_continuation")
            .last()
            .map_or(heading.end_byte(), |child| child.end_byte());
        items.push(Item {
            kind: "heading",
            name: heading
                .child_by_field_name("heading_content")
                .map(|content| {
                    let mut cursor = content.walk();
                    let end = content
                        .named_children(&mut cursor)
                        .filter(|child| child.kind() != "block_continuation")
                        .last()
                        .map_or(content.end_byte(), |child| child.end_byte());
                    source[content.start_byte()..end].trim().to_owned()
                })
                .unwrap_or_default(),
            header: source[line_start(source, heading.start_byte())..header_end]
                .trim_end()
                .to_owned(),
            footer: String::new(),
            content: Content::new(source, line_start(source, heading.start_byte())..section_end),
            children: markdown_items(&headings[1..next], source, section_end),
            test_only: false,
            contains_tests: false,
        });
        headings = &headings[next..];
    }
    items
}

fn heading_level(heading: Node<'_>) -> usize {
    let mut cursor = heading.walk();
    heading
        .named_children(&mut cursor)
        .find_map(|child| match child.kind() {
            "atx_h1_marker" | "setext_h1_underline" => Some(1),
            "atx_h2_marker" | "setext_h2_underline" => Some(2),
            "atx_h3_marker" => Some(3),
            "atx_h4_marker" => Some(4),
            "atx_h5_marker" => Some(5),
            "atx_h6_marker" => Some(6),
            _ => None,
        })
        .unwrap_or(6)
}

fn text<'a>(source: &'a str, node: Node<'_>) -> &'a str {
    &source[node.byte_range()]
}

fn indentation(source: &str, node: Node<'_>) -> String {
    let prefix = &source[line_start(source, node.start_byte())..node.start_byte()];
    prefix
        .chars()
        .map(|c| if c.is_whitespace() { c } else { ' ' })
        .collect()
}

fn line_start(source: &str, byte: usize) -> usize {
    source[..byte].rfind('\n').map_or(0, |newline| newline + 1)
}

fn indented(source: &str, node: Node<'_>, range: Range<usize>) -> String {
    format!("{}{}", indentation(source, node), sanitized(source, node, range))
}

fn sanitized(source: &str, node: Node<'_>, range: Range<usize>) -> String {
    let mut replacements = Vec::new();
    redactions(node, &range, &mut replacements);
    let mut output = String::new();
    let mut start = range.start;
    for (hidden, replacement) in replacements {
        output.push_str(&source[start..hidden.start]);
        output.push_str(replacement);
        start = hidden.end;
    }
    output.push_str(&source[start..range.end]);
    output
}

fn redactions(node: Node<'_>, visible: &Range<usize>, out: &mut Vec<(Range<usize>, &'static str)>) {
    if node.end_byte() <= visible.start || node.start_byte() >= visible.end {
        return;
    }
    let replacement = match node.kind() {
        "block" => Some("{ … }"),
        "closure_expression" => Some("…"),
        "token_tree" | "token_tree_pattern" => Some(match node.child(0).map(|child| child.kind()) {
            Some("(") => "(…)",
            Some("[") => "[…]",
            _ => "{ … }",
        }),
        "line_comment" | "block_comment" => Some(""),
        _ => None,
    };
    if let Some(replacement) = replacement {
        if node.start_byte() >= visible.start && node.end_byte() <= visible.end {
            out.push((node.byte_range(), replacement));
        }
        return;
    }
    let value = matches!(node.kind(), "const_item" | "static_item" | "enum_variant" | "attribute")
        .then(|| node.child_by_field_name("value"))
        .flatten();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if Some(child) == value && child.start_byte() >= visible.start && child.end_byte() <= visible.end {
            out.push((child.byte_range(), "…"));
        } else {
            redactions(child, visible, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Language, parse};

    #[test]
    fn mixed_markdown_headings_share_a_hierarchy() -> gix_testtools::TestResult {
        let items = parse(
            "Preamble

Guide
=====

Details
-------

### Nested

Hidden prose

# Next
",
            Language::Markdown,
        )?;
        assert_eq!(items.len(), 3, "the preamble and top-level headings are siblings");
        let guide = &items[1];
        assert_eq!(guide.header, "Guide\n=====", "setext formatting is retained");
        assert_eq!(guide.children.len(), 1, "the level-two heading belongs to the guide");
        assert_eq!(guide.children[0].name, "Details");
        assert_eq!(guide.children[0].children[0].header, "### Nested");
        assert!(
            guide.children[0].children[0].content.contains("Hidden prose"),
            "prose participates in comparison without appearing in the header"
        );
        Ok(())
    }

    #[test]
    fn markdown_headings_keep_container_markers_without_next_line_continuations() -> gix_testtools::TestResult {
        for (source, header, name) in [
            (
                "> # Quoted
> secret_prose
",
                "> # Quoted",
                "Quoted",
            ),
            (
                "- # Listed

  secret_prose
",
                "- # Listed",
                "Listed",
            ),
            (
                "> Guide
> =====
> secret_prose
",
                "> Guide
> =====",
                "Guide",
            ),
        ] {
            let items = parse(source, Language::Markdown)?;
            assert_eq!(
                items.len(),
                1,
                "container markers are part of the heading, not a preamble"
            );
            assert_eq!(items[0].header, header, "only the source heading lines are displayed");
            assert_eq!(
                items[0].name, name,
                "next-line continuation markers aren't part of the title"
            );
        }
        Ok(())
    }

    #[test]
    fn closing_braces_retain_source_indentation_unless_inline() -> gix_testtools::TestResult {
        let items = parse(
            "mod odd {
    fn hidden() { secret(); }
  }
",
            Language::Rust,
        )?;
        assert_eq!(
            items[0].footer, "  }",
            "a multiline closing brace retains its original indentation"
        );
        let items = parse("mod inline { fn hidden() { secret(); } }", Language::Rust)?;
        assert_eq!(
            items[0].footer, "}",
            "an inline closing brace aligns with its container"
        );
        Ok(())
    }
}
