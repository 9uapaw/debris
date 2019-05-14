//! Get current weather of Budapest from BBC main site
//!
//!
extern crate debris;

use debris::parse::Config;
use debris::parse::Meta;
use debris::parse::Parser;
use debris::parse::Populator;
use std::collections::HashMap;

fn main() {
    let path = String::from(r#"START(SELECTOR: ul[id="results"]) -> FIND(NAME: daily, SELECTOR: a[class="title"], SELECT: ALL(-), LOC: ATTR(HREF))"#);
    let paths = vec![path];
    let fields = HashMap::<String, String>::new();
    let meta = Meta {
        populator: String::from("single"),
        link_path: None,
        base_url: String::from("https://port.hu/programkereso/szinhaz?q=&interval=today&events_from=2019-05-13&events_until=2019-05-14&dft=i&cityMain=1&city=cityList-3372&area=theater&ageLimitFrom=2&ageLimitTo=10&s=start&onlyFav=0&documentId="),
        paging: None,
        prepend_links: None,
    };
    let config = Config {
        meta,
        paths,
        fields,
    };
    let mut parser = Parser::new(config);
    let populator = parser.build();
    match populator {
        Populator::Single(mut p) => {
            p.populate();
            for (k, v) in p.map {
                println!("{} {}", k, v);
            }
        }
        _ => (),
    }
}
