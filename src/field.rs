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
                let selected = self
                    .html
                    .select(&parsed)
                    .nth(*n as usize)
                    .expect("Element not found");
                self.value = Some(extract(&selected, &self.identifier.destination_location));
            },
            ElementSelection::All(delimiter) => {
                let mut selected = self.html.select(&parsed);
                let mut value = extract(&selected.next().expect("Element not found"), &self.identifier.destination_location);
                for element in selected.skip(0) {
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

#[cfg(test)]
mod tests {
    use super::{ FieldPopulator, FieldIdentity, Destination, DestinationLocation, ElementSelection};
    use scraper::Html;
    use std::cell::RefCell;

    #[test]
    fn test_find_non_nested_single_first_text() {
        let html_string = r#"<div>find me</div>"#;
        let html = Html::parse_fragment(&html_string);
        let html = RefCell::new(&html);
        let identity = FieldIdentity{destination: Destination::new("div", ElementSelection::first()), destination_location: DestinationLocation::Text};
        let mut field_populator = FieldPopulator::new(html.borrow(), &identity);

        field_populator.find_field();

        assert_eq!(field_populator.value.unwrap(), "find me");
    }

    #[test]
    fn test_find_nested_single_first_text() {
        let html_string = r#"<div><p>find me<p></div>"#;
        let html = Html::parse_fragment(&html_string);
        let html = RefCell::new(&html);
        let identity = FieldIdentity{destination: Destination::new("p", ElementSelection::first()), destination_location: DestinationLocation::Text};
        let mut field_populator = FieldPopulator::new(html.borrow(), &identity);

        field_populator.find_field();

        assert_eq!(field_populator.value.unwrap(), "find me");
    }

    #[test]
    fn test_find_non_nested_all_text() {
        let html_string = r#"<div>find me</div> <div>as well</div>"#;
        let html = Html::parse_fragment(&html_string);
        let html = RefCell::new(&html);
        let identity = FieldIdentity{destination: Destination::new("div", ElementSelection::All(String::from(" "))), destination_location: DestinationLocation::Text};
        let mut field_populator = FieldPopulator::new(html.borrow(), &identity);

        field_populator.find_field();

        assert_eq!(field_populator.value.unwrap(), "find me as well");
    }

    #[test]
    fn test_find_nested_all_text() {
        let html_string = r#"<p><div>find me</div> <div>as well</div></p>"#;
        let html = Html::parse_fragment(&html_string);
        let html = RefCell::new(&html);
        let identity = FieldIdentity{destination: Destination::new("div", ElementSelection::All(String::from(" "))), destination_location: DestinationLocation::Text};
        let mut field_populator = FieldPopulator::new(html.borrow(), &identity);

        field_populator.find_field();

        assert_eq!(field_populator.value.unwrap(), "find me as well");
    }

    #[test]
    #[should_panic(expected = "Element not found")]
    fn test_panic_on_invalid_element() {
        let html_string = r#"<div>find me</div> <div>as well</div>"#;
        let html = Html::parse_fragment(&html_string);
        let html = RefCell::new(&html);
        let identity = FieldIdentity{destination: Destination::new("a", ElementSelection::All(String::from(" "))), destination_location: DestinationLocation::Text};
        let mut field_populator = FieldPopulator::new(html.borrow(), &identity);

        field_populator.find_field();
    }
}
