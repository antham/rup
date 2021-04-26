use std::{cell::RefCell, rc::Rc};

use crate::parser::{CssSelector, CssSelectorAttribute};

use super::parser;
use html5ever::tendril::stream::TendrilSink;
use html5ever::tendril::StrTendril;
use html5ever::{parse_document, Attribute};
use markup5ever_rcdom::{Handle, Node, NodeData, RcDom};
use parser::AttributeSign;

// Filters html nodes matching the given css expression
pub fn filter(content: String, css_selectors: &Vec<parser::CssSelector>) -> Vec<Rc<Node>> {
    let root_node = parse_document(RcDom::default(), Default::default())
        .one(StrTendril::from(content.as_str()));
    filter_matching_nodes(root_node.document.to_owned(), &css_selectors, 0)
}

// Traverses the DOM recursively to filter matching nodes
fn filter_matching_nodes(
    node: Handle,
    css_selectors: &Vec<parser::CssSelector>,
    index: usize,
) -> Vec<Rc<Node>> {
    match node.data {
        NodeData::Document => node.children.take().iter().fold(vec![], |mut acc, n| {
            for nc in filter_matching_nodes(n.to_owned(), css_selectors, index) {
                acc.push(nc);
            }
            acc
        }),
        NodeData::Element {
            ref name,
            ref attrs,
            ..
        } => {
            if index == css_selectors.len() {
                return vec![];
            }

            let selector = css_selectors.get(index).unwrap();

            let is_matching_node = is_matching_selector_name(selector, name.local.as_ref())
                && is_matching_selector_attributes(selector, attrs);
            let next_index = if is_matching_node { index + 1 } else { index };

            if next_index == css_selectors.len() && is_matching_node {
                vec![node.to_owned()]
            } else {
                node.children.take().iter().fold(vec![], |mut acc, n| {
                    for nc in filter_matching_nodes(n.to_owned(), css_selectors, next_index) {
                        acc.push(nc);
                    }
                    acc
                })
            }
        }
        _ => vec![],
    }
}

fn is_matching_selector_name(selector: &CssSelector, element_name: impl AsRef<str>) -> bool {
    selector
        .to_owned()
        .name
        .map_or_else(|| true, |v| v.as_str() == element_name.as_ref())
}

fn is_matching_selector_attributes(
    selector: &CssSelector,
    attrs: &RefCell<Vec<Attribute>>,
) -> bool {
    selector.attributes.iter().fold(true, |acc, c| {
        if acc == false {
            false
        } else {
            attrs
                .borrow()
                .iter()
                .flat_map(|attr| {
                    let v = if attr.name.local.to_string() == "class" {
                        attr.value
                            .to_string()
                            .split_whitespace()
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                    } else {
                        vec![attr.value.to_string()]
                    };

                    vec![attr.name.local.as_ref()]
                        .repeat(v.len())
                        .into_iter()
                        .zip(v)
                        .collect::<Vec<_>>()
                })
                .into_iter()
                .filter(|v| match &c {
                    CssSelectorAttribute::ID(id) => (*v).0 == "id" && (*v).1 == id.to_owned(),
                    CssSelectorAttribute::Class(class) => {
                        (*v).0 == "class" && (*v).1 == class.to_owned()
                    }
                    CssSelectorAttribute::Attribute(attr, AttributeSign::Empty, None) => {
                        (*v).0 == *attr
                    }
                    CssSelectorAttribute::Attribute(attr, AttributeSign::Equal, Some(val)) => {
                        (*v).0 == *attr && (*v).1 == *val
                    }
                    CssSelectorAttribute::Attribute(attr, AttributeSign::Contain, Some(val)) => {
                        (*v).0 == *attr && (*v).1.contains(val.as_str())
                    }
                    CssSelectorAttribute::Attribute(attr, AttributeSign::BeginWith, Some(val)) => {
                        (*v).0 == *attr && (*v).1.starts_with(val.as_str())
                    }
                    CssSelectorAttribute::Attribute(attr, AttributeSign::EndWith, Some(val)) => {
                        (*v).0 == *attr && (*v).1.ends_with(val.as_str())
                    }
                    _ => false,
                })
                .collect::<Vec<_>>()
                .len()
                == 1
        }
    })
}

#[cfg(test)]
mod tests {
    use super::super::parser::*;
    use super::*;
    use crate::parser::CssSelectorAttribute;
    use html5ever::serialize;
    use markup5ever::serialize::TraversalScope;
    use markup5ever_rcdom::SerializableHandle;
    use serialize::SerializeOpts;
    use std::{env, fs};

    #[test]
    fn filter_documents() {
        let scenarios = vec![
            (
                // Css expression made of a chain of css selectors
                vec![
                    CssSelector {
                        name: Some("div".to_string()),
                        attributes: vec![],
                    },
                    CssSelector {
                        name: Some("span".to_string()),
                        attributes: vec![],
                    },
                    CssSelector {
                        name: Some("span".to_string()),
                        attributes: vec![CssSelectorAttribute::ID("1".to_string())],
                    },
                    CssSelector {
                        name: Some("span".to_string()),
                        attributes: vec![CssSelectorAttribute::Class("3".to_string())],
                    },
                ],
                "chain_of_css.html",
                r#"<span class="3">TEST 3</span>"#,
                1,
            ),
            (
                // Css expression made of a single css selector down into the html
                vec![CssSelector {
                    name: Some("span".to_string()),
                    attributes: vec![CssSelectorAttribute::ID("3".to_string())],
                }],
                "single_css_selector.html",
                r#"<span id="3"><span class="7">TEST 7</span><span class="8">TEST 8</span><span class="9">TEST 9</span></span>"#,
                1,
            ),
            (
                // Css expression returning several nodes
                vec![
                    CssSelector {
                        name: Some("span".to_string()),
                        attributes: vec![],
                    },
                    CssSelector {
                        name: Some("span".to_string()),
                        attributes: vec![],
                    },
                    CssSelector {
                        name: Some("span".to_string()),
                        attributes: vec![],
                    },
                    CssSelector {
                        name: Some("span".to_string()),
                        attributes: vec![],
                    },
                    CssSelector {
                        name: Some("span".to_string()),
                        attributes: vec![],
                    },
                    CssSelector {
                        name: Some("span".to_string()),
                        attributes: vec![],
                    },
                    CssSelector {
                        name: Some("span".to_string()),
                        attributes: vec![],
                    },
                ],
                "several_nodes.html",
                r#"<span class="1">TEST 1</span><span class="2">TEST 2</span><span class="3">TEST 3</span><span class="4">TEST 4</span><span class="5">TEST 5</span><span class="6">TEST 6</span><span class="7">TEST 7</span><span class="8">TEST 8</span><span class="9">TEST 9</span>"#,
                9,
            ),
            (
                // Css expression with strict equality attribute selector
                vec![
                    CssSelector {
                        name: Some("div".to_string()),
                        attributes: vec![],
                    },
                    CssSelector {
                        name: Some("div".to_string()),
                        attributes: vec![],
                    },
                    CssSelector {
                        name: None,
                        attributes: vec![CssSelectorAttribute::Attribute(
                            "data-val".to_string(),
                            AttributeSign::Equal,
                            Some("2".to_string()),
                        )],
                    },
                ],
                "strict_equality_selector.html",
                r#"<div data-val="2">TEST 2</div><div data-val="2"><div data-val="2">TEST 4</div><div data-val="1">TEST 5</div><div data-val="2">TEST 6</div></div><div data-val="2">TEST 8</div>"#,
                3,
            ),
            (
                // Css expression with beginning with attribute selector
                vec![
                    CssSelector {
                        name: Some("div".to_string()),
                        attributes: vec![],
                    },
                    CssSelector {
                        name: Some("div".to_string()),
                        attributes: vec![],
                    },
                    CssSelector {
                        name: None,
                        attributes: vec![CssSelectorAttribute::Attribute(
                            "data-val".to_string(),
                            AttributeSign::BeginWith,
                            Some("5".to_string()),
                        )],
                    },
                ],
                "with_beginning_selector.html",
                r#"<div data-val="5678">TEST 10</div>"#,
                1,
            ),
            (
                // Css expression with ending with attribute selector
                vec![
                    CssSelector {
                        name: Some("div".to_string()),
                        attributes: vec![],
                    },
                    CssSelector {
                        name: Some("div".to_string()),
                        attributes: vec![],
                    },
                    CssSelector {
                        name: None,
                        attributes: vec![CssSelectorAttribute::Attribute(
                            "data-val".to_string(),
                            AttributeSign::EndWith,
                            Some("5".to_string()),
                        )],
                    },
                ],
                "with_ending_selector.html",
                r#"<div data-val="797985">TEST 12</div>"#,
                1,
            ),
            (
                // Css expression with containing attribute selector
                vec![
                    CssSelector {
                        name: Some("div".to_string()),
                        attributes: vec![],
                    },
                    CssSelector {
                        name: Some("div".to_string()),
                        attributes: vec![],
                    },
                    CssSelector {
                        name: None,
                        attributes: vec![CssSelectorAttribute::Attribute(
                            "data-val".to_string(),
                            AttributeSign::Contain,
                            Some("756".to_string()),
                        )],
                    },
                ],
                "containing_selector.html",
                r#"<div data-val="67567">TEST 11</div>"#,
                1,
            ),
            (
                // Css expression with matching attribute selector
                vec![
                    CssSelector {
                        name: Some("div".to_string()),
                        attributes: vec![],
                    },
                    CssSelector {
                        name: Some("div".to_string()),
                        attributes: vec![],
                    },
                    CssSelector {
                        name: None,
                        attributes: vec![CssSelectorAttribute::Attribute(
                            "data-val".to_string(),
                            AttributeSign::Empty,
                            None,
                        )],
                    },
                ],
                "matching_selector.html",
                r#"<div data-val="1"><div data-val="1">TEST 1</div><div data-val="2">TEST 2</div><div data-val="1">TEST 3</div></div><div data-val="2"><div data-val="2">TEST 4</div><div data-val="1">TEST 5</div><div data-val="2">TEST 6</div></div><div data-val="1"><div data-val="1">TEST 9</div><div data-val="2">TEST 8</div><div data-val="1">TEST 9</div></div><div data-val="3"><div data-val="5678">TEST 10</div><div data-val="67567">TEST 11</div><div data-val="797985">TEST 12</div></div>"#,
                4,
            ),
            (
                // Css expression with mutli attribute
                vec![
                    CssSelector {
                        name: Some("div".to_string()),
                        attributes: vec![],
                    },
                    CssSelector {
                        name: Some("span".to_string()),
                        attributes: vec![
                            CssSelectorAttribute::Class("test2".to_string()),
                            CssSelectorAttribute::Attribute(
                                "data-attr".to_string(),
                                AttributeSign::Equal,
                                Some("test".to_string()),
                            ),
                            CssSelectorAttribute::Class("test3".to_string()),
                            CssSelectorAttribute::Class("test1".to_string()),
                        ],
                    },
                ],
                "multi_attributes.html",
                r#"<span data-attr="test" class="test1 test2 test3">TEST 1</span>"#,
                1,
            ),
            (
                // Css expression with an unexisting node
                vec![
                    CssSelector {
                        name: Some("li".to_string()),
                        attributes: vec![],
                    },
                    CssSelector {
                        name: Some("div".to_string()),
                        attributes: vec![],
                    },
                ],
                "unexisting_node.html",
                r#""#,
                0,
            ),
        ];

        for (css_selectors, filename, expected_html, matching_node_count) in scenarios {
            let content = fs::read_to_string(
                env::var("CARGO_MANIFEST_DIR").unwrap() + "/src/filter/" + filename,
            )
            .unwrap();
            let nodes = filter(content, &css_selectors);

            assert_eq!(nodes.len(), matching_node_count);

            let mut bg = bytebuffer::ByteBuffer::new();

            for node in nodes {
                let t = SerializableHandle::from(node.to_owned());
                let mut b = bytebuffer::ByteBuffer::new();
                let traversal_scope = SerializeOpts {
                    scripting_enabled: true,
                    traversal_scope: TraversalScope::IncludeNode,
                    create_missing_parent: false,
                };

                serialize(&mut b, &t, traversal_scope).unwrap();
                bg.write_bytes(b.to_bytes().as_ref());
            }

            debug_assert_eq!(
                regex::Regex::new(">\\s*<").unwrap().replace_all(
                    String::from_utf8(bg.to_bytes().into_iter().collect())
                        .unwrap()
                        .as_ref(),
                    "><"
                ),
                expected_html
            );
        }
    }
}
