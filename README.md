# Debris

Declarative HTML scraper

[![Build Status](https://travis-ci.com/9uapaw/debris.svg?branch=master)](https://travis-ci.com/9uapaw/debris)

## Why use it?

Debris is created for automatic content generation in mind. It provides an abstraction over CSS and HTML element selection in a declarative way, that helps structuring your code for scraping scripts.

## Usage

Add this to your `Cargo.toml`

```toml
[dependencies]
debris = { git = "https://github.com/9uapaw/debris" }
```

## Getting started

### Basic example

A simple example of finding an element by using a path and a field finder.

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


