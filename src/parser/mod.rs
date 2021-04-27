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

// Represents a css selector (e.g. div#id, div.class, ....)
#[derive(Debug, Default, Clone, PartialEq)]
pub struct CssSelector {
    pub name: Option<String>,
    pub attributes: Vec<CssSelectorAttribute>,
}

/// Parse a string made of css selectors
pub fn parse(expression: String) -> Vec<CssSelector> {
    let mut current_node = CssSelector::default();
    let mut current_node_attribute = CssSelectorAttribute::Empty;
    let mut nodes: Vec<CssSelector> = vec![];
    let mut previous_char: char = '°';

    // All over the for, the continue statement is used to not record the character being processed
    for c in expression.chars() {
        // This match define which class of selector is currently processed
        match c {
            // The space character is the delimiter between 2 css selectors
            ' ' => {
                if CssSelectorAttribute::Empty != current_node_attribute {
                    current_node
                        .attributes
                        .push(current_node_attribute.to_owned());
                    current_node_attribute = CssSelectorAttribute::Empty;
                }
                nodes.push(current_node);
                current_node = CssSelector::default();
                previous_char = c;
                continue;
            }
            '#' => {
                if CssSelectorAttribute::Empty != current_node_attribute {
                    current_node.attributes.push(current_node_attribute);
                }
                current_node_attribute = CssSelectorAttribute::ID(String::new());
                previous_char = c;
                continue;
            }
            '.' => {
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
            ':' => {
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
    nodes
}

#[cfg(test)]
mod tests {
    use super::{parse, AttributeSign, CssSelector, CssSelectorAttribute};

    #[test]
    fn parse_expression() {
        assert_eq!(
            parse(
                r#"div span #blue div#purple .green div.red :first-of-type p:first-child span:nth-child(2) [data-id='1234'] a[href*='hello'] div[data-class$="red1"] span[role^="complementary"] div#test1.test2.test3:first-child"#
                    .to_string()
            ),
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
                    name: None,
                    attributes: vec![CssSelectorAttribute::ID("blue".to_string())],
                },
                CssSelector {
                    name: Some("div".to_string()),
                    attributes: vec![CssSelectorAttribute::ID("purple".to_string())],
                },
                CssSelector {
                    name: None,
                    attributes: vec![CssSelectorAttribute::Class("green".to_string())],
                },
                CssSelector {
                    name: Some("div".to_string()),
                    attributes: vec![CssSelectorAttribute::Class("red".to_string())],
                },
                CssSelector {
                    name: None,
                    attributes: vec![CssSelectorAttribute::PseudoClass("first-of-type".to_string(), None)],
                },
                CssSelector {
                    name: Some("p".to_string()),
                    attributes: vec![CssSelectorAttribute::PseudoClass("first-child".to_string(), None)],
                },
                CssSelector {
                    name: Some("span".to_string()),
                    attributes: vec![CssSelectorAttribute::PseudoClass("nth-child".to_string(), Some("2".to_string()))],
                },
                CssSelector {
                    name: None,
                    attributes: vec![CssSelectorAttribute::Attribute("data-id".to_string(), AttributeSign::Equal , Some("1234".to_string()))],
                },
                CssSelector {
                    name: Some("a".to_string()),
                    attributes: vec![CssSelectorAttribute::Attribute("href".to_string(), AttributeSign::Contain ,Some("hello".to_string()))],
                },
                CssSelector {
                    name: Some("div".to_string()),
                    attributes: vec![CssSelectorAttribute::Attribute("data-class".to_string(), AttributeSign::EndWith ,Some("red1".to_string()))],
                },
                CssSelector {
                    name: Some("span".to_string()),
                    attributes: vec![CssSelectorAttribute::Attribute("role".to_string(), AttributeSign::BeginWith ,Some("complementary".to_string()))],
                },
                CssSelector {
                    name: Some("div".to_string()),
                    attributes: vec![
                        CssSelectorAttribute::ID("test1".to_string()),
                        CssSelectorAttribute::Class("test2".to_string()),
                        CssSelectorAttribute::Class("test3".to_string()),
                        CssSelectorAttribute::PseudoClass("first-child".to_string(), None),
                    ],
                },
            ]
        );
    }
}
