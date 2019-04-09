//! Example of reading the first comment of every entry on Hacker News
//!
extern crate debris;
use debris::declare::Destination;
use debris::declare::DestinationLocation;
use debris::declare::ElementSelection;
use debris::declare::FieldIdentity;
use debris::declare::PathBuilder;
use debris::declare::SearchDetail;
use debris::population::MultiplePopulator;
use std::collections::HashMap;

fn main() {
    let link_path = PathBuilder::new()
        .start(Destination::new(r#"tbody"#, ElementSelection::first()))
        .find_all("a", "", DestinationLocation::Attr(String::from("href")))
        .build();

    let mut search = SearchDetail::new();
    let mut comment_fields = HashMap::new();
    comment_fields.insert(
        String::from("first"),
        FieldIdentity {
            destination: Destination::new("p", ElementSelection::All(String::from(" "))),
            destination_location: DestinationLocation::Text,
        },
    );
    let mut initial = HashMap::new();
    initial.insert(
        String::from("initial"),
        FieldIdentity {
            destination: Destination::new(
                r#"span[class="commtext c00"]"#,
                ElementSelection::first(),
            ),
            destination_location: DestinationLocation::Text,
        },
    );
    let comment_path = PathBuilder::new()
        .start(Destination::new(
            r#"div[class="comment"]"#,
            ElementSelection::first(),
        ))
        .populate(initial)
        .descend(r#"span[class="commtext c00"]"#, 0)
        .populate(comment_fields)
        .build();

    let link_converter = |link: String| -> Option<String> {
        if link.starts_with("item") {
            Some(String::from("https://news.ycombinator.com/") + &link)
        } else {
            None
        }
    };

    search.insert_path(comment_path);
    let mut populator = MultiplePopulator::new(
        "https://news.ycombinator.com/",
        link_path,
        Some(&link_converter),
        search,
    );
    populator.populate().expect("Link extracting failed");

    for maps in populator.populated_links {
        for (k, v) in maps {
            println!("{} : {}", k, v);
        }
    }
}
