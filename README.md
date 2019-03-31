# Debris

Declarative HTML scraper

## Why use it?

Debris is created for automatic content generation in mind. It provides an abstraction over CSS and HTML element selection in a declarative way, that helps structuring your code for scraping scripts.

## Getting started

### Basic example

A simple example of finding an unambigous element

```rust
let html = r#"<div class=found>
    <i>HELLO</i>
    <i>WORLD</i>
    <i>!</i>
    </div>"#;
    let path = PathBuilder::new()
        .start(Destination::new(
            r#"div[class="found"]"#,
            ElementSelection::first(),
        ))
        .find_all("i", "", DestinationLocation::Text)
        .build();
    let mut search = SearchDetail::new();
    search.insert_field(
        "test",
        r#"div[class="found"]"#,
        DestinationLocation::Attr(String::from("class")),
        ElementSelection::first(),
    );
    search.insert_path(path);

    let mut populator = SinglePopulator::new(&html, search);
    populator.populate();
    for (k, v) in populator.map {
        println!("KEY: {} - VALUE: {}", k, v);
    }

    for v in populator.values {
        print!("{} ", v);
    }
```
