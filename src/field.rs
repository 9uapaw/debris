use scraper::ElementRef;
use scraper::Html;
use scraper::Selector;
use std::cell::Ref;
use std::collections::HashMap;

#[derive(Clone)]
/// Contains information about the HTML element to be extracted in the process.
pub struct FieldIdentity {
    pub destination: Destination,
    pub destination_location: DestinationLocation,
}

#[derive(Clone)]
/// A simple tuple, with a selector string and a selection number.
pub struct Destination(pub String, pub ElementSelection);

impl<'a> Destination {
    pub fn new(selector: &'a str, selection: ElementSelection) -> Destination {
        Destination(String::from(selector), selection)
    }
}

#[derive(Clone)]
/// Provides a way to select an exact HTML element, or all the elements.
pub enum ElementSelection {
    /// Select the nth element.
    Single(i32),
    /// Select all the elements.
    /// > Might not be usable on every occasion! (eg. on [`PathStep::Descend`])
    All(String),
}

impl ElementSelection {
    pub fn first() -> ElementSelection {
        ElementSelection::Single(0)
    }
}

#[derive(Clone)]
/// Selects what to extract from the HTML element.
pub enum DestinationLocation {
    /// Extract the element's attribute. (eg. to extract date from `<div itemprop=date></div>`, the
    /// argument is itemprop).
    Attr(String),
    /// Extract the element's id.
    Id(String),
    /// Extract the element's class.
    Class(String),
    /// Extract text. (eg. to extract HELLO from `<div>HELLO</div>`)
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
                self.value = Some(find_single(
                    &self.html.root_element(),
                    self.identifier,
                    *n as usize,
                ));
            }
            ElementSelection::All(delimiter) => {
                self.value = Some(concatenate_all(
                    &self.html.root_element(),
                    self.identifier,
                    delimiter,
                ));
            }
        }
    }
}

pub fn concatenate_all(
    element: &ElementRef,
    identifier: &FieldIdentity,
    delimiter: &str,
) -> String {
    let selector = Selector::parse(&identifier.destination.0).unwrap();
    let mut selection = element.select(&selector);

    let mut value = String::new();
    for selected_element in selection {
        value +=
            &(extract(&selected_element, &identifier.destination_location) + &delimiter.clone());
    }

    String::from(value.trim())
}

pub fn find_all(element: &ElementRef, identifier: &FieldIdentity) -> Vec<String> {
    let selector = Selector::parse(&identifier.destination.0).unwrap();
    let mut selection = element.select(&selector);
    let mut values = Vec::new();

    for child_element in selection {
        values.push(extract(&child_element, &identifier.destination_location));
    }

    values
}

pub fn find_single(
    element: &ElementRef,
    identifier: &FieldIdentity,
    selection_number: usize,
) -> String {
    let selector = Selector::parse(&identifier.destination.0).unwrap();
    let mut selection = element.select(&selector);

    match selection.nth(selection_number) {
        Some(e) => extract(&e, &identifier.destination_location),
        None => String::from(""),
    }
}

pub fn extract(element: &ElementRef, location: &DestinationLocation) -> String {
    match location {
        DestinationLocation::Text => element.text().collect::<Vec<_>>().join(" "),
        DestinationLocation::Attr(attr) => element.value().attr(&attr).unwrap_or("").to_string(),
        _ => "".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Destination, DestinationLocation, ElementSelection, FieldIdentity, FieldPopulator,
    };
    use scraper::Html;
    use std::cell::RefCell;

    #[test]
    fn test_find_non_nested_single_first_text() {
        let html_string = r#"<div>find me</div>"#;
        let html = Html::parse_fragment(&html_string);
        let html = RefCell::new(&html);
        let identity = FieldIdentity {
            destination: Destination::new("div", ElementSelection::first()),
            destination_location: DestinationLocation::Text,
        };
        let mut field_populator = FieldPopulator::new(html.borrow(), &identity);

        field_populator.find_field();

        assert_eq!(field_populator.value.unwrap(), "find me");
    }

    #[test]
    fn test_find_nested_single_first_text() {
        let html_string = r#"<div><p>find me<p></div>"#;
        let html = Html::parse_fragment(&html_string);
        let html = RefCell::new(&html);
        let identity = FieldIdentity {
            destination: Destination::new("p", ElementSelection::first()),
            destination_location: DestinationLocation::Text,
        };
        let mut field_populator = FieldPopulator::new(html.borrow(), &identity);

        field_populator.find_field();

        assert_eq!(field_populator.value.unwrap(), "find me");
    }

    #[test]
    fn test_find_non_nested_all_text() {
        let html_string = r#"<div>find me</div> <div>as well</div>"#;
        let html = Html::parse_fragment(&html_string);
        let html = RefCell::new(&html);
        let identity = FieldIdentity {
            destination: Destination::new("div", ElementSelection::All(String::from(" "))),
            destination_location: DestinationLocation::Text,
        };
        let mut field_populator = FieldPopulator::new(html.borrow(), &identity);

        field_populator.find_field();

        assert_eq!(field_populator.value.unwrap(), "find me as well");
    }

    #[test]
    fn test_find_nested_all_text() {
        let html_string = r#"<p><div>find me</div> <div>as well</div></p>"#;
        let html = Html::parse_fragment(&html_string);
        let html = RefCell::new(&html);
        let identity = FieldIdentity {
            destination: Destination::new("div", ElementSelection::All(String::from(" "))),
            destination_location: DestinationLocation::Text,
        };
        let mut field_populator = FieldPopulator::new(html.borrow(), &identity);

        field_populator.find_field();

        assert_eq!(field_populator.value.unwrap(), "find me as well");
    }

    #[test]
    fn test_empty_on_invalid_element() {
        let html_string = r#"<div>find me</div> <div>as well</div>"#;
        let html = Html::parse_fragment(&html_string);
        let html = RefCell::new(&html);
        let identity = FieldIdentity {
            destination: Destination::new("a", ElementSelection::All(String::from(" "))),
            destination_location: DestinationLocation::Text,
        };
        let mut field_populator = FieldPopulator::new(html.borrow(), &identity);

        field_populator.find_field();

        assert_eq!(field_populator.value.unwrap(), "");
    }
}
