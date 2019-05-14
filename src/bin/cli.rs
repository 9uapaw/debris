extern crate debris;
use clap::{App, Arg};
use debris::parse::Config;
use debris::parse::Meta;
use debris::parse::Parser;
use debris::parse::Populator;
use std::fs::File;
use std::io::Read;

fn main() {
    let matches = App::new("Debris CLI Application")
        .version("0.2.0")
        .author("Quapaw")
        .arg(Arg::with_name("path").short("p").value_name("PATH"))
        .arg(
            Arg::with_name("print")
                .help("Prints result to stdout")
                .long("print"),
        )
        .get_matches();

    let print = matches.is_present("print");

    match matches.value_of("path") {
        Some(path) => process_file(path, print),
        None => return,
    }
}

fn process_file(path: &str, print: bool) {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => panic!("Invalid path"),
    };

    let mut s = String::new();
    file.read_to_string(&mut s);
    let config: Config = match serde_json::from_str(&s) {
        Ok(c) => c,
        Err(error) => panic!(format!("{}", error)),
    };
    let mut parser = Parser::new(config);
    let mut populator = parser.build();
    populator.run();

    if print {
        populator.print();
    }
}
