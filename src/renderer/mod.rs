use colored::*;
use html5ever::{
    serialize::{AttrRef, Serialize, Serializer, TraversalScope},
    QualName,
};
use markup5ever::LocalName;
use markup5ever::{local_name, namespace_url, ns};
use markup5ever_rcdom::{Node, SerializableHandle};
use regex::{Regex, RegexBuilder};
use serde;
use std::{collections::HashMap, io, rc::Rc};

// Represents an HTML tree transformed to a JSON tree
#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
#[serde(untagged)]
enum SNode {
    Regular {
        name: String,
        #[serde(rename(serialize = "type", deserialize = "type"))]
        kind: String,
        text: Option<String>,
        attributes: Option<HashMap<String, Option<String>>>,
        children: Option<Vec<SNode>>,
    },
    Comment {
        #[serde(rename(serialize = "type", deserialize = "type"))]
        kind: String,
        text: Option<String>,
    },
    DocType {
        name: String,
        #[serde(rename(serialize = "type", deserialize = "type"))]
        kind: String,
    },
    ProcessingInstruction {
        name: String,
        #[serde(rename(serialize = "type", deserialize = "type"))]
        kind: String,
        text: Option<String>,
    },
}

#[derive(Clone, Default)]
pub struct SerializeSettings {
    is_color_enabled: bool,
    should_render_text_only: bool,
    should_render_attributes: bool,
    attributes: Vec<String>,
    is_json_enabled: bool,
}
pub struct SerializeSettingsBuilder {
    serialize_settings: SerializeSettings,
}

impl SerializeSettingsBuilder {
    pub fn new() -> Self {
        SerializeSettingsBuilder {
            serialize_settings: SerializeSettings::default(),
        }
    }

    pub fn enable_color(&mut self) {
        self.serialize_settings.is_color_enabled = true;
    }

    pub fn should_render_text_only(&mut self) {
        self.serialize_settings.should_render_text_only = true;
    }

    pub fn should_render_attributes(&mut self, attributes: Vec<String>) {
        self.serialize_settings.should_render_attributes = true;
        self.serialize_settings.attributes = attributes;
    }

    pub fn render_json(&mut self) {
        self.serialize_settings.is_json_enabled = true;
    }

    fn build(&mut self) -> SerializeSettings {
        self.serialize_settings.to_owned()
    }
}

// Output a node list as a list of html markup or JSON strings
pub fn serialize_nodes(
    mut settings_builder: SerializeSettingsBuilder,
    nodes: Vec<Rc<Node>>,
) -> io::Result<String> {
    let settings = settings_builder.build();

    if settings.is_json_enabled {
        Ok(serialize_nodes_to_json(&nodes))
    } else {
        serialize_nodes_to_html(settings, &nodes)
    }
}

fn serialize_nodes_to_json(nodes: &Vec<Rc<Node>>) -> String {
    serde_json::to_string(&nodes.iter().fold(vec![], |mut acc, node| {
        acc.push(convert_node_to_snode(node));
        acc
    }))
    .unwrap()
}

fn convert_node_to_snode(node: &Rc<Node>) -> Option<SNode> {
    match &node.data {
        markup5ever_rcdom::NodeData::Element { name, attrs, .. } => {
            let children = node.children.take();
            let attributes = attrs.take();

            Some(SNode::Regular {
                name: name.local.trim().to_string(),
                attributes: if attributes.is_empty() {
                    None
                } else {
                    Some(attributes.iter().fold(HashMap::new(), |mut acc, attr| {
                        acc.insert(
                            attr.name.local.to_string(),
                            if attr.value.is_empty() {
                                None
                            } else {
                                Some(attr.value.to_string())
                            },
                        );
                        acc
                    }))
                },
                children: {
                    let cs = children
                        .iter()
                        .flat_map(|n| convert_node_to_snode(&n))
                        .collect::<Vec<SNode>>();

                    if cs.is_empty() {
                        None
                    } else {
                        Some(cs)
                    }
                },
                kind: "regular".to_string(),
                text: children
                    .iter()
                    .flat_map(|n| match n.data {
                        markup5ever_rcdom::NodeData::Text { ref contents } => {
                            Some(contents.take().to_string())
                        }
                        _ => None,
                    })
                    .fold(Some(String::new()), |text, e| text.map(|v| v + e.as_ref()))
                    .map(|v| {
                        Regex::new(r"\s+")
                            .unwrap()
                            .replace_all(v.as_str(), " ")
                            .trim()
                            .to_string()
                    })
                    .filter(|v| !v.is_empty()),
            })
        }
        markup5ever_rcdom::NodeData::Doctype { ref name, .. } => Some(SNode::DocType {
            name: name.to_string(),
            kind: "doctype".to_string(),
        }),
        markup5ever_rcdom::NodeData::Comment { ref contents } => Some(SNode::Comment {
            kind: "comment".to_string(),
            text: Some(contents.trim().to_string()),
        }),
        markup5ever_rcdom::NodeData::ProcessingInstruction {
            ref target,
            ref contents,
        } => Some(SNode::ProcessingInstruction {
            name: target.to_string(),
            kind: "processing_instruction".to_string(),
            text: Some(contents.trim().to_string()),
        }),
        markup5ever_rcdom::NodeData::Document => panic!("Can't serialize Document node itself"),
        _ => None,
    }
}

fn serialize_nodes_to_html(
    settings: SerializeSettings,
    nodes: &Vec<Rc<Node>>,
) -> io::Result<String> {
    nodes.iter().fold(Ok(String::new()), |acc, node| {
        let mut buffer = String::new();
        let serializer = SerializableHandle::from(node.to_owned());

        let mut ser: HtmlSerializer = HtmlSerializer::new(settings.to_owned(), &mut buffer);
        serializer.serialize(&mut ser, TraversalScope::IncludeNode)?;

        if !buffer.is_empty() {
            match acc {
                // Every extra whitespaces is removed and a new line is added to separate every node
                // to be processed easily in a pipe
                Ok(s) => Ok((if s.is_empty() { s } else { s + "\n" })
                    + RegexBuilder::new(r">\s+<")
                        .build()
                        .unwrap()
                        .replace_all(buffer.as_str().trim(), "><")
                        .replace("\n", "")
                        .as_ref()),
                e => e,
            }
        } else {
            acc
        }
    })
}

#[derive(Default)]
struct ElemInfo {
    html_name: Option<LocalName>,
    ignore_children: bool,
}

// This serializer is cloned from https://github.com/servo/html5ever/blob/57eb334c0ffccc6f88d563419f0fbeef6ff5741c/html5ever/src/serialize/mod.rs#L77
pub struct HtmlSerializer<'a> {
    colorizer: Colorizer,
    settings: SerializeSettings,
    stack: Vec<ElemInfo>,
    buffer: &'a mut String,
}

impl<'a> HtmlSerializer<'a> {
    pub fn new(settings: SerializeSettings, buffer: &'a mut String) -> Self {
        HtmlSerializer {
            colorizer: Colorizer::new(settings.is_color_enabled),
            settings,
            stack: vec![ElemInfo {
                html_name: None,
                ignore_children: false,
            }],
            buffer,
        }
    }

    fn parent(&mut self) -> &mut ElemInfo {
        if self.stack.len() == 0 {
            self.stack.push(Default::default());
        }
        self.stack.last_mut().unwrap()
    }

    fn write_escaped(&mut self, text: &str, attr_mode: bool) -> String {
        text.chars().fold(String::new(), |acc, c| {
            acc + match c {
                '&' => String::from("&amp;"),
                '\u{00A0}' => String::from("&nbsp;"),
                '"' if attr_mode => String::from("&quot;"),
                '<' if !attr_mode => String::from("&lt;"),
                '>' if !attr_mode => String::from("&gt;"),
                c => c.to_string(),
            }
            .as_str()
        })
    }
}

impl<'b> Serializer for HtmlSerializer<'b> {
    fn start_elem<'a, AttrIter>(&mut self, name: QualName, attrs: AttrIter) -> io::Result<()>
    where
        AttrIter: Iterator<Item = AttrRef<'a>>,
    {
        let html_name = match name.ns {
            ns!(html) => Some(name.local.clone()),
            _ => None,
        };

        if self.parent().ignore_children {
            self.stack.push(ElemInfo {
                html_name,
                ignore_children: true,
            });
            return Ok(());
        }

        if !self.settings.should_render_text_only && !self.settings.should_render_attributes {
            self.buffer.push_str(
                self.colorizer
                    .colorize(format!("<{}", tagname(&name).trim()), Color::Magenta)
                    .as_str(),
            );
        }
        for (name, value) in attrs {
            if !self.settings.should_render_text_only && !self.settings.should_render_attributes {
                self.buffer.push_str(" ");
                match name.ns {
                    ns!() => (),
                    ns!(xml) => self
                        .buffer
                        .push_str(self.colorizer.colorize("xml:", Color::Magenta).as_ref()),
                    ns!(xmlns) => {
                        if name.local != local_name!("xmlns") {
                            self.buffer.push_str(
                                self.colorizer.colorize("xmlns:", Color::Magenta).as_ref(),
                            );
                        }
                    }
                    ns!(xlink) => self
                        .buffer
                        .push_str(self.colorizer.colorize("xlink:", Color::Magenta).as_ref()),
                    _ => {
                        self.buffer.push_str(
                            self.colorizer
                                .colorize("unknown_namespace:", Color::Magenta)
                                .as_ref(),
                        );
                    }
                }

                self.buffer.push_str(
                    self.colorizer
                        .colorize(name.local.trim(), Color::Yellow)
                        .trim(),
                );
                self.buffer
                    .push_str(self.colorizer.colorize("=\"", Color::Magenta).as_ref());
                let v = self
                    .colorizer
                    .colorize(self.write_escaped(value.trim(), true), Color::Green);
                self.buffer.push_str(v.to_string().as_str());
                self.buffer
                    .push_str(self.colorizer.colorize("\"", Color::Magenta).as_ref());
            }

            if self.settings.should_render_attributes {
                if self
                    .settings
                    .attributes
                    .contains(&name.local.trim().to_string())
                {
                    self.buffer.push_str(value.trim());
                    self.buffer.push(' ');
                }
            }
        }

        if !self.settings.should_render_text_only && !self.settings.should_render_attributes {
            self.buffer
                .push_str(self.colorizer.colorize(">", Color::Magenta).as_ref());
        }

        let ignore_children = name.ns == ns!(html)
            && match name.local {
                local_name!("area")
                | local_name!("base")
                | local_name!("basefont")
                | local_name!("bgsound")
                | local_name!("br")
                | local_name!("col")
                | local_name!("embed")
                | local_name!("frame")
                | local_name!("hr")
                | local_name!("img")
                | local_name!("input")
                | local_name!("keygen")
                | local_name!("link")
                | local_name!("meta")
                | local_name!("param")
                | local_name!("source")
                | local_name!("track")
                | local_name!("wbr") => true,
                _ => false,
            };

        self.stack.push(ElemInfo {
            html_name,
            ignore_children,
        });

        Ok(())
    }

    fn end_elem(&mut self, name: QualName) -> io::Result<()> {
        let info = match self.stack.pop() {
            Some(info) => info,
            _ => panic!("no ElemInfo"),
        };
        if info.ignore_children {
            return Ok(());
        }

        if !self.settings.should_render_text_only && !self.settings.should_render_attributes {
            self.buffer.push_str(
                self.colorizer
                    .colorize(format!("</{}>", tagname(&name).trim()), Color::Magenta)
                    .as_ref(),
            );
        }

        Ok(())
    }

    fn write_text(&mut self, text: &str) -> io::Result<()> {
        let escape = match self.parent().html_name {
            Some(local_name!("style"))
            | Some(local_name!("script"))
            | Some(local_name!("xmp"))
            | Some(local_name!("iframe"))
            | Some(local_name!("noembed"))
            | Some(local_name!("noframes"))
            | Some(local_name!("plaintext"))
            | Some(local_name!("noscript")) => false,
            _ => true,
        };

        if self.settings.should_render_attributes
            || (self.settings.should_render_text_only
                && (text.trim().is_empty()
                    || Some(local_name!("style")) == self.parent().html_name
                    || Some(local_name!("script")) == self.parent().html_name))
        {
            return Ok(());
        }

        if escape {
            let v = self.write_escaped(text, false);
            self.buffer.push_str(v.trim());
        } else {
            self.buffer.push_str(text.trim());
        }

        if self.settings.should_render_text_only {
            self.buffer.push_str(" ");
        }

        Ok(())
    }

    fn write_comment(&mut self, text: &str) -> io::Result<()> {
        if !self.settings.should_render_text_only && !self.settings.should_render_attributes {
            self.buffer.push_str(
                self.colorizer
                    .colorize(format!("<!--{}-->", text.trim()), Color::Blue)
                    .as_ref(),
            );
        }
        Ok(())
    }

    fn write_doctype(&mut self, name: &str) -> io::Result<()> {
        if !self.settings.should_render_text_only && !self.settings.should_render_attributes {
            self.buffer.push_str(
                self.colorizer
                    .colorize(format!("<!DOCTYPE {}>", name.trim()), Color::Blue)
                    .as_ref(),
            );
        }
        Ok(())
    }

    fn write_processing_instruction(&mut self, target: &str, data: &str) -> io::Result<()> {
        if !self.settings.should_render_text_only && !self.settings.should_render_attributes {
            self.buffer.push_str(
                self.colorizer
                    .colorize(format!("<?{} {}>", target.trim(), data.trim()), Color::Blue)
                    .as_ref(),
            );
        }
        Ok(())
    }
}

fn tagname(name: &QualName) -> LocalName {
    match name.ns {
        ns!(html) | ns!(mathml) | ns!(svg) => (),
        _ => {}
    }

    name.local.clone()
}

// This struct allows to enable/disable the color
// the library only expose an environment variable to do so
#[derive(Clone, Copy)]
pub struct Colorizer {
    is_enabled: bool,
}

impl Colorizer {
    fn new(is_enabled: bool) -> Self {
        Colorizer { is_enabled }
    }

    fn colorize<T: AsRef<str>>(self, s: T, color: Color) -> String {
        if self.is_enabled {
            s.as_ref().color(color).to_string()
        } else {
            s.as_ref().to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use std::{env, fs};

    use crate::filter::filter;
    use crate::renderer;
    use crate::renderer::SNode;
    use crate::{parser::CssSelector, renderer::SerializeSettingsBuilder};

    #[test]
    fn serialize_nodes() {
        struct Scenario {
            filename: &'static str,
            selector: &'static str,
            settings: SerializeSettingsBuilder,
            test: Box<dyn Fn(String, String)>,
        }

        let scenarios: Vec<Scenario> = vec![
            Scenario {
                // Remove all extra spaces
                filename: "document_with_space",
                selector: "html",
                settings: (|| SerializeSettingsBuilder::new())(),
                test: Box::new(|actual, expected| {
                    assert_eq!(actual, expected);
                }),
            },
            Scenario {
                // Extract one element per line
                filename: "extract_markup",
                selector: "div",
                settings: (|| SerializeSettingsBuilder::new())(),
                test: Box::new(|actual, expected| {
                    assert_eq!(actual, expected);
                }),
            },
            Scenario {
                // Enable colors
                filename: "enable_colors",
                selector: "div",
                settings: (|| {
                    let mut s = SerializeSettingsBuilder::new();
                    s.enable_color();
                    s
                })(),
                test: Box::new(|actual, expected| {
                    assert_eq!(actual, expected);
                }),
            },
            Scenario {
                // Text only
                filename: "text_only",
                selector: "html",
                settings: (|| {
                    let mut s = SerializeSettingsBuilder::new();
                    s.should_render_text_only();
                    s
                })(),
                test: Box::new(|actual, expected| {
                    assert_eq!(actual, expected);
                }),
            },
            Scenario {
                // Render attributes
                filename: "render_attributes",
                selector: "div",
                settings: (|| {
                    let mut s = SerializeSettingsBuilder::new();
                    s.should_render_attributes(vec!["class".to_string(), "data-value".to_string()]);
                    s
                })(),
                test: Box::new(|actual, expected| {
                    assert_eq!(actual, expected);
                }),
            },
            Scenario {
                // Render a whole html document in JSON
                filename: "render_whole_json",
                selector: "html",
                settings: (|| {
                    let mut s = SerializeSettingsBuilder::new();
                    s.render_json();
                    s
                })(),
                test: Box::new(|actual, expected| {
                    let a: Vec<SNode> = serde_json::from_str(actual.as_str()).unwrap();
                    let e: Vec<SNode> = serde_json::from_str(expected.as_str()).unwrap();
                    assert_eq!(a, e);
                }),
            },
            Scenario {
                // Render part of html tree in a single JSON
                filename: "render_partial_json",
                selector: "div",
                settings: (|| {
                    let mut s = SerializeSettingsBuilder::new();
                    s.render_json();
                    s
                })(),
                test: Box::new(|actual, expected| {
                    let a: Vec<SNode> = serde_json::from_str(actual.as_str()).unwrap();
                    let e: Vec<SNode> = serde_json::from_str(expected.as_str()).unwrap();
                    assert_eq!(a, e);
                }),
            },
        ];

        for s in scenarios {
            let given_html =
                fs::read(env::var("CARGO_MANIFEST_DIR").unwrap() + "/src/renderer/" + s.filename)
                    .unwrap();

            let expected = fs::read_to_string(
                env::var("CARGO_MANIFEST_DIR").unwrap()
                    + "/src/renderer/"
                    + s.filename
                    + "_expected",
            )
            .unwrap();

            let nodes = filter(
                given_html,
                &vec![CssSelector {
                    name: Some(s.selector.to_string()),
                    attributes: vec![],
                }],
            );

            let actual = renderer::serialize_nodes(s.settings, nodes)
                .unwrap()
                .replace("\u{1b}", "\\u{1b}");

            (s.test)(actual, expected);
        }
    }
}
