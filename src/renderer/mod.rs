use std::{io, rc::Rc};

use colored::*;
use html5ever::{
    serialize::{AttrRef, Serialize, SerializeOpts, Serializer, TraversalScope},
    QualName,
};
use markup5ever::LocalName;
use markup5ever::{local_name, namespace_url, ns};
use markup5ever_rcdom::{Node, SerializableHandle};
use regex::RegexBuilder;

// Output a node list as a list of html markup strings
pub fn serialize_nodes(is_color_enabled: bool, nodes: Vec<Rc<Node>>) -> io::Result<String> {
    nodes.iter().fold(Ok(String::new()), |acc, node| {
        let mut buffer = String::new();
        let serializer = SerializableHandle::from(node.to_owned());
        let opts = SerializeOpts {
            scripting_enabled: true,
            traversal_scope: TraversalScope::IncludeNode,
            create_missing_parent: false,
        };

        let mut ser: HtmlSerializer = HtmlSerializer::new(
            Colorizer::new(is_color_enabled),
            &mut buffer,
            opts.to_owned(),
        );
        serializer.serialize(&mut ser, opts.traversal_scope)?;

        match acc {
            // Every extra whitespaces is removed and a new line is added to separate every node
            // to be processed easily in a pipe
            Ok(s) => Ok((if s.is_empty() { s } else { s + "\n" })
                + RegexBuilder::new(r">\s+<")
                    .build()
                    .unwrap()
                    .replace_all(buffer.as_str(), "><")
                    .replace("\n", "")
                    .as_ref()),
            e => e,
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
    opts: SerializeOpts,
    stack: Vec<ElemInfo>,
    buffer: &'a mut String,
}

impl<'a> HtmlSerializer<'a> {
    pub fn new(colorizer: Colorizer, buffer: &'a mut String, opts: SerializeOpts) -> Self {
        let html_name = match opts.traversal_scope {
            TraversalScope::IncludeNode | TraversalScope::ChildrenOnly(None) => None,
            TraversalScope::ChildrenOnly(Some(ref n)) => Some(tagname(n)),
        };
        HtmlSerializer {
            colorizer,
            opts,
            stack: vec![ElemInfo {
                html_name,
                ignore_children: false,
            }],
            buffer,
        }
    }

    fn parent(&mut self) -> &mut ElemInfo {
        if self.stack.len() == 0 {
            if self.opts.create_missing_parent {
                self.stack.push(Default::default());
            } else {
                panic!("no parent ElemInfo")
            }
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
        self.buffer.push_str(
            self.colorizer
                .colorize(format!("<{}", tagname(&name).to_string()), Color::Magenta)
                .as_str(),
        );
        for (name, value) in attrs {
            self.buffer.push_str(" ");
            match name.ns {
                ns!() => (),
                ns!(xml) => self
                    .buffer
                    .push_str(self.colorizer.colorize("xml:", Color::Magenta).as_ref()),
                ns!(xmlns) => {
                    if name.local != local_name!("xmlns") {
                        self.buffer
                            .push_str(self.colorizer.colorize("xmlns:", Color::Magenta).as_ref());
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
                    .colorize(name.local.to_string(), Color::Yellow)
                    .as_str(),
            );
            self.buffer
                .push_str(self.colorizer.colorize("=\"", Color::Magenta).as_ref());
            let v = self
                .colorizer
                .colorize(self.write_escaped(value, true), Color::Green);
            self.buffer.push_str(v.to_string().as_ref());
            self.buffer
                .push_str(self.colorizer.colorize("\"", Color::Magenta).as_ref());
        }
        self.buffer
            .push_str(self.colorizer.colorize(">", Color::Magenta).as_ref());

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
            None if self.opts.create_missing_parent => Default::default(),
            _ => panic!("no ElemInfo"),
        };
        if info.ignore_children {
            return Ok(());
        }

        self.buffer.push_str(
            self.colorizer
                .colorize(format!("</{}>", tagname(&name).to_string()), Color::Magenta)
                .as_ref(),
        );
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
            | Some(local_name!("plaintext")) => false,

            Some(local_name!("noscript")) => !self.opts.scripting_enabled,

            _ => true,
        };

        if escape {
            let v = self.write_escaped(text, false);
            self.buffer.push_str(v.to_string().trim());
        } else {
            self.buffer.push_str(text.to_string().trim());
        }

        Ok(())
    }

    fn write_comment(&mut self, text: &str) -> io::Result<()> {
        self.buffer.push_str(
            self.colorizer
                .colorize(format!("<!--{}-->", text), Color::Blue)
                .as_ref(),
        );
        Ok(())
    }

    fn write_doctype(&mut self, name: &str) -> io::Result<()> {
        self.buffer.push_str(
            self.colorizer
                .colorize(format!("<!DOCTYPE {}>", name), Color::Blue)
                .as_ref(),
        );
        Ok(())
    }

    fn write_processing_instruction(&mut self, target: &str, data: &str) -> io::Result<()> {
        self.buffer.push_str(
            self.colorizer
                .colorize(format!("<?{} {}>", target, data), Color::Blue)
                .as_ref(),
        );
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
    use std::{env, fs};

    use crate::filter::filter;
    use crate::parser::CssSelector;
    use crate::renderer;

    #[test]
    fn serialize_nodes() {
        let scenarios = vec![
            (
                // Remove all extra spaces
                "document_with_space",
                "html",
                false,
            ),
            (
                // Extract one element per line
                "extract_markup",
                "div",
                false,
            ),
            (
                // Enable colors
                "enable_colors",
                "div",
                true,
            ),
        ];

        for s in scenarios {
            let given_html =
                fs::read(env::var("CARGO_MANIFEST_DIR").unwrap() + "/src/renderer/" + s.0).unwrap();

            let expected_html = fs::read_to_string(
                env::var("CARGO_MANIFEST_DIR").unwrap() + "/src/renderer/" + s.0 + "_expected",
            )
            .unwrap();

            let nodes = filter(
                given_html,
                &vec![CssSelector {
                    name: Some(s.1.to_string()),
                    attributes: vec![],
                }],
            );

            debug_assert_eq!(
                renderer::serialize_nodes(s.2, nodes)
                    .unwrap()
                    .replace("\u{1b}", "\\u{1b}"),
                expected_html
            );
        }
    }
}
