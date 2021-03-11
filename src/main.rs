use clap::{App, Arg};

fn main() {
    App::new("rup")
        .version("0.0.1")
        .author("Anthony Hamon <hamon.anth@gmail.com>")
        .about("jq for html, fork of ericchiang/pup")
        .arg(Arg::with_name("selector").index(1))
        .get_matches();
}
