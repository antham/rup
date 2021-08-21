use clap::{App, Arg};
use std::io::{stdin, Read};

use crate::renderer::SerializeSettingsBuilder;

mod filter;
mod parser;
mod renderer;

fn main() {
    let matches = App::new("rup")
        .version("0.0.1")
        .author("Anthony Hamon <hamon.anth@gmail.com>")
        .about("jq for html")
        .arg(Arg::new("no-color").long("no-color").short('c').about("Disable colors, NO_COLOR environment variable could be used as well see https://no-color.org/"))
        .arg(Arg::new("keep-text-only").long("keep-text-only").short('t').about("Extract the text from every end matched node, if a node has children, the text of every node is extracted and concatened with a space as separator"))
        .arg(Arg::new("keep-attributes-values").long("keep-attributes-values").short('a').conflicts_with("keep-text-only").takes_value(true) .multiple_values(true).about("Extract provided node attributes from every end matched node using the attribute key, if several attributes are provided or if an attribute is found more than once for a given node, values are extracted and concatened with a space as separator"))
        .arg(Arg::new("json").long("json").short('j').conflicts_with("keep-text-only").conflicts_with("keep-attributes-values").about("Render html nodes as a JSON document. When a node property does not contain any data it is set to null. A type property separate comment, regular markup, doctype and processor instructions"))
        .arg(Arg::new("selectors").multiple_values(true).about(r#"Css selectors, it is possible to provide several selectors by separating them with a space, pay attention to the fact that "div" "span" is different than "div span", the first one select all div nodes and all span nodes the second one select span nodes children of a div node"#))
        .get_matches();

    let mut buffer: Vec<u8> = Vec::new();
    stdin().read_to_end(&mut buffer).unwrap();

    match matches.values_of("selectors") {
        Some(selector_chains) => {
            selector_chains.for_each(|selector_chain| {
                let mut settings_builder = SerializeSettingsBuilder::new();
                if !matches.is_present("no-color") {
                    settings_builder.enable_color();
                }
                if matches.is_present("keep-text-only") {
                    settings_builder.should_render_text_only();
                }
                if matches.is_present("json") {
                    settings_builder.render_json();
                }
                match matches.values_of("keep-attributes-values") {
                    Some(attributes) => settings_builder.should_render_attributes(
                        attributes.map(|v| v.to_string()).collect::<Vec<String>>(),
                    ),
                    None => (),
                }

                let css_selector = &parser::parse(selector_chain.to_string());
                let nodes = filter::filter(buffer.clone(), css_selector);
                println!(
                    "{}",
                    renderer::serialize_nodes(settings_builder, nodes).unwrap()
                );
            });
            ()
        }
        None => (),
    };
}
