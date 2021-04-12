use std::rc::Rc;

use crate::parser::CssSelectorAttribute;

use super::parser;
use html5ever::parse_document;
use html5ever::tendril::stream::TendrilSink;
use html5ever::tendril::StrTendril;
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
            let is_matching_selector = selector
                .to_owned()
                .name
                .map_or_else(|| true, |v| v == name.local.to_string());
            let is_matching_selector_attribute = selector.attribute.to_owned().map_or_else(
                || true,
                |c| {
                    enum Sign {
                        Empty,
                        Equal,
                        BeginWith,
                        EndWith,
                        Contain,
                    }

                    let (identifier, sign, value) = match c {
                        CssSelectorAttribute::ID(id) => ("id".to_string(), Sign::Equal, id),
                        CssSelectorAttribute::Class(class) => {
                            ("class".to_string(), Sign::Equal, class)
                        }
                        CssSelectorAttribute::Attribute(attr, AttributeSign::Equal, Some(val)) => {
                            (attr, Sign::Equal, val)
                        }
                        CssSelectorAttribute::Attribute(
                            attr,
                            AttributeSign::BeginWith,
                            Some(val),
                        ) => (attr, Sign::BeginWith, val),
                        CssSelectorAttribute::Attribute(
                            attr,
                            AttributeSign::EndWith,
                            Some(val),
                        ) => (attr, Sign::EndWith, val),
                        CssSelectorAttribute::Attribute(
                            attr,
                            AttributeSign::Contain,
                            Some(val),
                        ) => (attr, Sign::Contain, val),
                        _ => (String::new(), Sign::Empty, "".to_string()),
                    };
                    attrs
                        .borrow()
                        .iter()
                        .filter(|v| {
                            v.name.local.to_string() == identifier
                                && match sign {
                                    Sign::Equal => v.value.to_string() == *value,
                                    Sign::BeginWith => {
                                        v.value.to_string().starts_with(value.as_str())
                                    }
                                    Sign::EndWith => v.value.to_string().ends_with(value.as_str()),
                                    Sign::Contain => v.value.to_string().contains(value.as_str()),
                                    _ => false,
                                }
                        })
                        .collect::<Vec<_>>()
                        .len()
                        == 1
                },
            );

            let is_matching_node = is_matching_selector && is_matching_selector_attribute;
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
                        attribute: None,
                    },
                    CssSelector {
                        name: Some("span".to_string()),
                        attribute: None,
                    },
                    CssSelector {
                        name: Some("span".to_string()),
                        attribute: Some(CssSelectorAttribute::ID("1".to_string())),
                    },
                    CssSelector {
                        name: Some("span".to_string()),
                        attribute: Some(CssSelectorAttribute::Class("3".to_string())),
                    },
                ],
                r#"<span class="3">TEST 3</span>"#,
                1,
            ),
            (
                // Css expression made of a single css selector down into the html
                vec![CssSelector {
                    name: Some("span".to_string()),
                    attribute: Some(CssSelectorAttribute::ID("3".to_string())),
                }],
                r#"<span id="3"><span class="7">TEST 7</span><span class="8">TEST 8</span><span class="9">TEST 9</span></span>"#,
                1,
            ),
            (
                // Css expression returning several nodes
                vec![
                    CssSelector {
                        name: Some("span".to_string()),
                        attribute: None,
                    },
                    CssSelector {
                        name: Some("span".to_string()),
                        attribute: None,
                    },
                    CssSelector {
                        name: Some("span".to_string()),
                        attribute: None,
                    },
                    CssSelector {
                        name: Some("span".to_string()),
                        attribute: None,
                    },
                    CssSelector {
                        name: Some("span".to_string()),
                        attribute: None,
                    },
                    CssSelector {
                        name: Some("span".to_string()),
                        attribute: None,
                    },
                    CssSelector {
                        name: Some("span".to_string()),
                        attribute: None,
                    },
                ],
                r#"<span class="1">TEST 1</span><span class="2">TEST 2</span><span class="3">TEST 3</span><span class="4">TEST 4</span><span class="5">TEST 5</span><span class="6">TEST 6</span><span class="7">TEST 7</span><span class="8">TEST 8</span><span class="9">TEST 9</span>"#,
                9,
            ),
            (
                // Css expression with strict equality attribute selector
                vec![
                    CssSelector {
                        name: Some("div".to_string()),
                        attribute: None,
                    },
                    CssSelector {
                        name: Some("div".to_string()),
                        attribute: None,
                    },
                    CssSelector {
                        name: None,
                        attribute: Some(CssSelectorAttribute::Attribute(
                            "data-val".to_string(),
                            AttributeSign::Equal,
                            Some("2".to_string()),
                        )),
                    },
                ],
                r#"<div data-val="2">TEST 2</div><div data-val="2"><div data-val="2">TEST 4</div><div data-val="1">TEST 5</div><div data-val="2">TEST 6</div></div><div data-val="2">TEST 8</div>"#,
                3,
            ),
            (
                // Css expression with beginning with attribute selector
                vec![
                    CssSelector {
                        name: Some("div".to_string()),
                        attribute: None,
                    },
                    CssSelector {
                        name: Some("div".to_string()),
                        attribute: None,
                    },
                    CssSelector {
                        name: None,
                        attribute: Some(CssSelectorAttribute::Attribute(
                            "data-val".to_string(),
                            AttributeSign::BeginWith,
                            Some("5".to_string()),
                        )),
                    },
                ],
                r#"<div data-val="5678">TEST 10</div>"#,
                1,
            ),
            (
                // Css expression with ending with attribute selector
                vec![
                    CssSelector {
                        name: Some("div".to_string()),
                        attribute: None,
                    },
                    CssSelector {
                        name: Some("div".to_string()),
                        attribute: None,
                    },
                    CssSelector {
                        name: None,
                        attribute: Some(CssSelectorAttribute::Attribute(
                            "data-val".to_string(),
                            AttributeSign::EndWith,
                            Some("5".to_string()),
                        )),
                    },
                ],
                r#"<div data-val="797985">TEST 12</div>"#,
                1,
            ),
            (
                // Css expression with containing attribute selector
                vec![
                    CssSelector {
                        name: Some("div".to_string()),
                        attribute: None,
                    },
                    CssSelector {
                        name: Some("div".to_string()),
                        attribute: None,
                    },
                    CssSelector {
                        name: None,
                        attribute: Some(CssSelectorAttribute::Attribute(
                            "data-val".to_string(),
                            AttributeSign::Contain,
                            Some("756".to_string()),
                        )),
                    },
                ],
                r#"<div data-val="67567">TEST 11</div>"#,
                1,
            ),
            (
                // Css expression with an unexisting node
                vec![
                    CssSelector {
                        name: Some("li".to_string()),
                        attribute: None,
                    },
                    CssSelector {
                        name: Some("div".to_string()),
                        attribute: None,
                    },
                ],
                r#""#,
                0,
            ),
        ];

        for (css_selectors, expected_html, matching_node_count) in scenarios {
            let content = fs::read_to_string(
                env::var("CARGO_MANIFEST_DIR").unwrap() + "/src/filter/test.html",
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
                    String::from_utf8(
                        bg.to_bytes()
                            .into_iter()
                            .filter(|c| *c as char != '\n')
                            .collect()
                    )
                    .unwrap()
                    .as_ref(),
                    "><"
                ),
                expected_html
            );
        }
    }
}
