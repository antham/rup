// Represents the sign used by the css attribute selector
#[derive(Debug, Clone, PartialEq)]
pub enum AttributeSign {
    Empty,
    // Represents =
    Equal,
    // Represents *=
    Contain,
    // Represents ^=
    BeginWith,
    // Represents $=
    EndWith,
}

// Represents an element attribute (e.g. #id, .class, ....)
#[derive(Debug, Clone, PartialEq)]
pub enum CssSelectorAttribute {
    Empty,
    // Represents a css id selector like #efg
    ID(String),
    // Represents a css class selector like .abcd
    Class(String),
    // Represents a css attribute selector like [target=_blank]
    Attribute(String, AttributeSign, Option<String>),
    // Represents a css pseudo=class selector like :last-child
    PseudoClass(String, Option<String>),
}

// Represents a css combinator (e.g. : A B, A + B, A > B)
#[derive(Debug, Clone, PartialEq)]
pub enum CssCombinator {
    // Represents the space combinator that selects nodes that are descendants of the first element, A B
    Descendant,
    // Represents the child combinator that selects nodes that are direct children of the first element, A > B
    DirectChild,
    // Represents the combinator that selects adjacent siblings. This means that the second element directly follows the first, and both share the same parent, A + B
    AdjacentSibling,
}

impl Default for CssCombinator {
    fn default() -> Self {
        CssCombinator::Descendant
    }
}

// Represents a css selector (e.g. div#id, div.class, ....)
#[derive(Debug, Default, Clone, PartialEq)]
pub struct CssSelector {
    pub name: Option<String>,
    pub attributes: Vec<CssSelectorAttribute>,
    pub combinator: CssCombinator,
}

// Parse a string made of css selectors
pub fn parse(expression: String) -> Vec<CssSelector> {
    let mut nodes: Vec<CssSelector> = vec![];
    let mut expressions_parsed = Vec::<String>::new();
    let mut acc = String::new();
    let mut open_square_bracket_detected = false;

    for c in expression.chars() {
        // This match define which class of selector is currently processed
        match c {
            // The space character is the delimiter between 2 css selectors
            ' ' if !open_square_bracket_detected => {
                expressions_parsed.push(acc.to_owned());
                acc.clear();
            }
            '[' => {
                open_square_bracket_detected = true;
                acc.push(c);
            }
            ']' => {
                open_square_bracket_detected = false;
                acc.push(c);
            }
            _ => {
                acc.push(c);
            }
        }
    }

    if !acc.is_empty() {
        expressions_parsed.push(acc.to_owned());
    }

    let mut current_node_combinator = CssCombinator::Descendant;

    // All over the for, the continue statement is used to bypass the character being processed
    for expression_parsed in expressions_parsed {
        match expression_parsed.as_str() {
            ">" => {
                current_node_combinator = CssCombinator::DirectChild;
                continue;
            }
            "+" => {
                current_node_combinator = CssCombinator::AdjacentSibling;
                continue;
            }
            _ => (),
        }

        let mut current_node_attribute = CssSelectorAttribute::Empty;
        let mut previous_char = char::default();
        let mut current_node = CssSelector::default();
        current_node.combinator = current_node_combinator.to_owned();
        current_node_combinator = CssCombinator::default();

        for c in expression_parsed.chars() {
            // This match define which class of selector is currently processed
            match c {
                '#' if !matches!(
                    current_node_attribute,
                    CssSelectorAttribute::Attribute { .. }
                ) =>
                {
                    if CssSelectorAttribute::Empty != current_node_attribute {
                        current_node.attributes.push(current_node_attribute);
                    }
                    current_node_attribute = CssSelectorAttribute::ID(String::new());
                    previous_char = c;
                    continue;
                }
                '.' if !matches!(
                    current_node_attribute,
                    CssSelectorAttribute::Attribute { .. }
                ) =>
                {
                    if CssSelectorAttribute::Empty != current_node_attribute {
                        current_node.attributes.push(current_node_attribute);
                    }
                    current_node_attribute = CssSelectorAttribute::Class(String::new());
                    previous_char = c;
                    continue;
                }
                '[' => {
                    if CssSelectorAttribute::Empty != current_node_attribute {
                        current_node.attributes.push(current_node_attribute);
                    }
                    current_node_attribute =
                        CssSelectorAttribute::Attribute(String::new(), AttributeSign::Empty, None);
                    previous_char = c;
                    continue;
                }
                ':' if !matches!(
                    current_node_attribute,
                    CssSelectorAttribute::Attribute { .. }
                ) =>
                {
                    if CssSelectorAttribute::Empty != current_node_attribute {
                        current_node.attributes.push(current_node_attribute);
                    }
                    current_node_attribute = CssSelectorAttribute::PseudoClass(String::new(), None);
                    previous_char = c;
                    continue;
                }
                _ => (),
            }

            // This match will save the reference tied to the selector (e.g. red2 in .red2, id in #id, ....)
            match current_node_attribute {
                CssSelectorAttribute::Class(s) => {
                    current_node_attribute = CssSelectorAttribute::Class(s + c.to_string().as_ref())
                }
                CssSelectorAttribute::ID(s) => {
                    current_node_attribute = CssSelectorAttribute::ID(s + c.to_string().as_ref())
                }
                // The attr (e.g. 2 in div:nth-child(2)) is used as a marker, if it's not defined we have to add any character to define the pseudo-class
                // if it's defined, it means we are collecting characters for the attribute
                CssSelectorAttribute::PseudoClass(ref s, ref attr) => match c {
                    ')' | '(' => {
                        previous_char = c;
                        continue;
                    }
                    _ if previous_char == '(' => {
                        current_node_attribute =
                            CssSelectorAttribute::PseudoClass(s.to_owned(), Some(c.to_string()))
                    }
                    _ if attr != &None => {
                        current_node_attribute = CssSelectorAttribute::PseudoClass(
                            s.to_owned(),
                            Some(attr.to_owned().unwrap() + c.to_string().as_ref()),
                        )
                    }
                    _ if attr == &None => {
                        current_node_attribute = CssSelectorAttribute::PseudoClass(
                            s.to_owned() + c.to_string().as_ref(),
                            None,
                        )
                    }
                    _ => (),
                },
                // The sign (e.g. : =, ~=, ...) is used as a marker, if it's not defined we have to add any character to the left operand
                // if it's defined, we are completing the right operand
                CssSelectorAttribute::Attribute(ref left_operand, ref sign, ref right_operand) => {
                    match c {
                        // This mark the end of an attribute
                        ']' => {
                            previous_char = c;
                            continue;
                        }
                        '=' if previous_char == '^' && sign == &AttributeSign::Empty => {
                            current_node_attribute = CssSelectorAttribute::Attribute(
                                left_operand.to_owned(),
                                AttributeSign::BeginWith,
                                None,
                            )
                        }
                        '=' if previous_char == '$' && sign == &AttributeSign::Empty => {
                            current_node_attribute = CssSelectorAttribute::Attribute(
                                left_operand.to_owned(),
                                AttributeSign::EndWith,
                                None,
                            )
                        }
                        '=' if previous_char == '*' && sign == &AttributeSign::Empty => {
                            current_node_attribute = CssSelectorAttribute::Attribute(
                                left_operand.to_owned(),
                                AttributeSign::Contain,
                                None,
                            )
                        }
                        '=' if sign == &AttributeSign::Empty => {
                            current_node_attribute = CssSelectorAttribute::Attribute(
                                left_operand.to_owned(),
                                AttributeSign::Equal,
                                None,
                            );
                            previous_char = c;
                            continue;
                        }
                        '*' | '$' | '^' if right_operand == &None => {
                            previous_char = c;
                            continue;
                        }
                        _ if sign == &AttributeSign::Empty => {
                            current_node_attribute = CssSelectorAttribute::Attribute(
                                left_operand.to_owned() + c.to_string().as_ref(),
                                sign.to_owned(),
                                None,
                            )
                        }
                        '\'' | '"' if sign != &AttributeSign::Empty => {
                            previous_char = c;
                            continue;
                        }
                        _ if sign != &AttributeSign::Empty => {
                            current_node_attribute = CssSelectorAttribute::Attribute(
                                left_operand.to_owned(),
                                sign.to_owned(),
                                Some(
                                    right_operand.to_owned().unwrap_or(String::new())
                                        + c.to_string().as_ref(),
                                ),
                            )
                        }
                        _ => (),
                    }
                }
                // This will save the name of the dom element if it exists for instance div
                CssSelectorAttribute::Empty => match current_node.name {
                    Some(s) => current_node.name = Some(s + c.to_string().as_ref()),
                    None => current_node.name = Some(c.to_string()),
                },
            }

            previous_char = c;
        }

        if CssSelectorAttribute::Empty != current_node_attribute {
            current_node.attributes.push(current_node_attribute);
        }
        nodes.push(current_node);
    }

    nodes
}

#[cfg(test)]
mod tests {
    use crate::parser::CssCombinator;

    use super::{parse, AttributeSign, CssSelector, CssSelectorAttribute};
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_expression() {
        assert_eq!(
            parse(
                r#"div span #blue div#purple div.orange .green div.red :first-of-type > span#test p:first-child span:nth-child(2) [data-id='1234'] a[href*='hello'] div[data-class$="red1"] span[role^="complementary"] div#test1.test2.test3:first-child div.test5 + span.test6 [src="chrome:///file.js#test"] div[src="hello world"]"#
                    .to_string()
            ),
            vec![
                CssSelector {
                    name: Some("div".to_string()),
                    attributes: vec![],
                    combinator: CssCombinator::Descendant,
                },
                CssSelector {
                    name: Some("span".to_string()),
                    attributes: vec![],
                    combinator: CssCombinator::Descendant,
                },
                CssSelector {
                    name: None,
                    attributes: vec![CssSelectorAttribute::ID("blue".to_string())],
                    combinator: CssCombinator::Descendant,
                },
                CssSelector {
                    name: Some("div".to_string()),
                    attributes: vec![CssSelectorAttribute::ID("purple".to_string())],
                    combinator: CssCombinator::Descendant,
                },
                CssSelector {
                    name: Some("div".to_string()),
                    attributes: vec![CssSelectorAttribute::Class("orange".to_string())],
                    combinator: CssCombinator::Descendant,
                },
                CssSelector {
                    name: None,
                    attributes: vec![CssSelectorAttribute::Class("green".to_string())],
                    combinator: CssCombinator::Descendant,
                },
                CssSelector {
                    name: Some("div".to_string()),
                    attributes: vec![CssSelectorAttribute::Class("red".to_string())],
                    combinator: CssCombinator::Descendant,
                },
                CssSelector {
                    name: None,
                    attributes: vec![CssSelectorAttribute::PseudoClass("first-of-type".to_string(), None)],
                    combinator: CssCombinator::Descendant,
                },
               CssSelector {
                    name: Some("span".to_string()),
                    attributes: vec![CssSelectorAttribute::ID("test".to_string())],
                    combinator: CssCombinator::DirectChild,
                },
                CssSelector {
                    name: Some("p".to_string()),
                    attributes: vec![CssSelectorAttribute::PseudoClass("first-child".to_string(), None)],
                    combinator: CssCombinator::Descendant,
                },
                CssSelector {
                    name: Some("span".to_string()),
                    attributes: vec![CssSelectorAttribute::PseudoClass("nth-child".to_string(), Some("2".to_string()))],
                    combinator: CssCombinator::Descendant,
                },
                CssSelector {
                    name: None,
                    attributes: vec![CssSelectorAttribute::Attribute("data-id".to_string(), AttributeSign::Equal , Some("1234".to_string()))],
                    combinator: CssCombinator::Descendant,
                },
                CssSelector {
                    name: Some("a".to_string()),
                    attributes: vec![CssSelectorAttribute::Attribute("href".to_string(), AttributeSign::Contain ,Some("hello".to_string()))],
                    combinator: CssCombinator::Descendant,
                },
                CssSelector {
                    name: Some("div".to_string()),
                    attributes: vec![CssSelectorAttribute::Attribute("data-class".to_string(), AttributeSign::EndWith ,Some("red1".to_string()))],
                    combinator: CssCombinator::Descendant,
                },
                CssSelector {
                    name: Some("span".to_string()),
                    attributes: vec![CssSelectorAttribute::Attribute("role".to_string(), AttributeSign::BeginWith ,Some("complementary".to_string()))],
                    combinator: CssCombinator::Descendant,
                },
                CssSelector {
                    name: Some("div".to_string()),
                    attributes: vec![
                        CssSelectorAttribute::ID("test1".to_string()),
                        CssSelectorAttribute::Class("test2".to_string()),
                        CssSelectorAttribute::Class("test3".to_string()),
                        CssSelectorAttribute::PseudoClass("first-child".to_string(), None),
                    ],
                    combinator: CssCombinator::Descendant,
                },
                CssSelector {
                    name: Some("div".to_string()),
                    attributes: vec![
                        CssSelectorAttribute::Class("test5".to_string()),
                    ],
                    combinator: CssCombinator::Descendant,
                },
                CssSelector {
                    name: Some("span".to_string()),
                    attributes: vec![
                        CssSelectorAttribute::Class("test6".to_string()),
                    ],
                    combinator: CssCombinator::AdjacentSibling,
                },
                CssSelector {
                    name: None,
                    attributes: vec![CssSelectorAttribute::Attribute("src".to_string(), AttributeSign::Equal ,Some("chrome:///file.js#test".to_string()))],
                    combinator: CssCombinator::Descendant,
                },
                CssSelector {
                    name: Some("div".to_string()),
                    attributes: vec![CssSelectorAttribute::Attribute("src".to_string(), AttributeSign::Equal ,Some("hello world".to_string()))],
                    combinator: CssCombinator::Descendant,
                }
            ]
        );
    }
}
