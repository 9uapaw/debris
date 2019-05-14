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

## Communication protocol
The main goal of this library is to provide an easy to use crawling service. In order to ease the deployment, it is possible
to define a config to drive the population, allowing the library to run based on a JSON input. The format
of the protocol is the following:

```json
{
  "meta":
          {
            "populator": ["single", "multiple"],
            "link_path": {"type": "path"} // Only if multiple
            "base_url": {"type": "string"},
            "paging": {"type": "pattern"}, // Optional
            "extend_links": {"type": "string"} // Optional
          },
  "paths":
          {
            [{"type": "path"}]
          },
  "fields":
          {
            [{"field_name": "field_identity"}]
          }
}
```

### Path
Path is a string with a specific format. Every path must start with START command.
```text
START(SELECTOR: STR, SELECT: NUM) -> DESCEND(SELECTOR: STR, SELECT: NUM) -> FIND(NAME: STR, SELECTOR: STR, SELECT: [ALL(STR), NUM], LOC: [ATTR, TEXT])
```
The following commands are available:
1. `START`: Starting point
    - `SELECTOR`: Selector string
    - `SELECT`: Which element to start with _// OPTIONAL: IF NOT SPECIFIED, THE FIRST ELEMENT IS IMPLICITLY USED_
2. `DESCEND`: Step one level down the HTML hierarchy tree
    - `SELECTOR`: Selector string
    - `SELECT`: Which element to continue with _// OPTIONAL: IF NOT SPECIFIED, THE FIRST ELEMENT IS IMPLICITLY USED_
3. `FIND`: Find a value without identifying it
    - `SELECTOR`: Selector string
    - `SELECT`: Find the nth element, or concatenate all result with a delimiter
    - `LOC`: Position of the value in a HTML element

### Field
If a value could be extracted from a HTML tree unambigously (eg. `<div reallyUniqueAttr="unique"></div>`)
a field is an easy way to get started with the following syntax:
```text
FIELD(NAME: STR, SELECTOR: STR, SELECT: [ALL(STR), NUM], LOC: [ATTR, TEXT])
```