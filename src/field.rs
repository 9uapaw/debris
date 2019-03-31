use scraper::ElementRef;
use scraper::Html;
use scraper::Selector;
use std::cell::Ref;
use std::collections::HashMap;

#[derive(Clone)]
pub struct FieldIdentity {
    pub destination: Destination,
    pub destination_location: DestinationLocation,
}

#[derive(Clone)]
pub struct Destination(pub String, pub ElementSelection);

impl<'a> Destination {

    pub fn new(selector: &'a str, selection: ElementSelection) -> Destination {
        Destination(String::from(selector), selection)
    }
}

#[derive(Clone)]
pub enum ElementSelection {
    Single(i32),
    All(String),
}

impl ElementSelection {

    pub fn first() -> ElementSelection {
        ElementSelection::Single(0)
    }
}

#[derive(Clone)]
pub enum DestinationLocation {
    Attr(String),
    Id(String),
    Class(String),
    Text,
}

pub struct FieldPopulator<'a, 'b> {
    html: Ref<'a, &'b Html>,
    identifier: &'a FieldIdentity,
    pub value: Option<String>,
}

impl<'a, 'b> FieldPopulator<'a, 'b> {
    pub fn new(html: Ref<'b, &Html>, identifier: &'a FieldIdentity) -> FieldPopulator<'a, 'b> {
        FieldPopulator {
            html,
            identifier,
            value: None,
        }
    }

    pub fn find_field(&mut self) {
        let parsed = Selector::parse(&self.identifier.destination.0).unwrap();
        match &self.identifier.destination.1 {
            ElementSelection::Single(n) => {
                let selected = self
                    .html
                    .select(&parsed)
                    .nth(*n as usize)
                    .expect("Element not found!");
                self.value = Some(extract(&selected, &self.identifier.destination_location));
            },
            ElementSelection::All(delimiter) => {
                let selected = self.html.select(&parsed);
                let mut value = String::new();
                for element in selected {
                    value += &(delimiter.clone() + &extract(&element, &self.identifier.destination_location));
                }
                self.value = Some(value);
            }
        }
    }
}

pub fn extract(element: &ElementRef, location: &DestinationLocation) -> String {
    match location {
        DestinationLocation::Text => element.text().collect::<Vec<_>>().join(" "),
        DestinationLocation::Attr(attr) => element.value().attr(&attr).unwrap_or("").to_string(),
        _ => "".to_string(),
    }
}
