use crate::field::concatenate_all;
use crate::field::extract;
use crate::field::find_all;
use crate::field::find_single;
use crate::field::Destination;
use crate::field::DestinationLocation;
use crate::field::ElementSelection;
use crate::field::FieldIdentity;
use scraper::ElementRef;
use scraper::Html;
use scraper::Selector;
use std::cell::Ref;
use std::collections::HashMap;

#[derive(Clone)]
/// Building blocks of a path.
pub enum PathStep {
    /// Lower the hierarchy level by 1.
    Descend(Destination),
    /// Every path should be started with this.
    Start(Destination),
    /// Populate fields on current level.
    Populate(HashMap<String, FieldIdentity>),
    /// Find values on current level without field names.
    Find(FieldIdentity),
}

pub struct PathFinder<'a, 'b> {
    html: Ref<'a, &'b Html>,
    pub map: HashMap<String, String>,
    pub values: Vec<String>,
    path: &'a Vec<PathStep>,
}

impl<'a, 'b> PathFinder<'a, 'b> {
    pub fn new(path: &'a Vec<PathStep>, html: Ref<'a, &'b Html>) -> PathFinder<'a, 'b> {
        let map = HashMap::<String, String>::new();
        let values = Vec::<String>::new();
        PathFinder {
            html,
            map,
            values,
            path,
        }
    }

    pub fn search_path(&mut self) {
        let start = match &self.path.get(0) {
            Some(start) => {
                if let PathStep::Start(first) = start {
                    first
                } else {
                    panic!("First element of path should be a Start!");
                }
            }
            _ => panic!("First element of path should be a Start!"),
        };

        let n = match start.1 {
            ElementSelection::Single(n) => n,
            _ => panic!("Can not descend on all element"),
        };
        let parsed = Selector::parse(&start.0).unwrap();
        let mut selected = match self.html.select(&parsed).nth(n as usize) {
            Some(s) => s,
            None => return,
        };

        self.resolve_path(&selected, 1);
    }

    fn resolve_path(&mut self, element: &ElementRef, level: usize) {
        if level == self.path.len() {
            return;
        }

        match self.path.get(level) {
            Some(step) => match step {
                PathStep::Descend(destination) => {
                    let selection = Selector::parse(&destination.0).unwrap();
                    let n = match destination.1 {
                        ElementSelection::Single(n) => n,
                        _ => panic!("Can not descend on all element"),
                    };

                    let child_element = match element.select(&selection).nth(n as usize) {
                        Some(e) => e,
                        None => return,
                    };

                    self.resolve_path(&child_element, level + 1);
                }

                PathStep::Populate(field_map) => {
                    for (field_name, identifier) in field_map {
                        match &identifier.destination.1 {
                            ElementSelection::Single(n) => {
                                self.map.insert(
                                    field_name.clone(),
                                    find_single(element, identifier, *n as usize),
                                );
                            }
                            ElementSelection::All(delimiter) => {
                                self.map.insert(
                                    field_name.clone(),
                                    concatenate_all(element, identifier, delimiter),
                                );
                            }
                        }
                    }
                }

                PathStep::Find(field_identifier) => match &field_identifier.destination.1 {
                    ElementSelection::Single(n) => {
                        self.values
                            .push(find_single(element, field_identifier, *n as usize));
                    }
                    ElementSelection::All(_) => {
                        self.values.extend(find_all(element, field_identifier));
                    }
                },
                _ => panic!("Invalid path!"),
            },
            None => (),
        };

        self.resolve_path(element, level + 1);
    }
}

/// A convenient helper to build up a path.
pub struct PathBuilder {
    path: Vec<PathStep>,
}

impl<'a> PathBuilder {
    pub fn new() -> PathBuilder {
        let path = Vec::new();
        PathBuilder { path }
    }

    pub fn start(&mut self, destination: Destination) -> &mut Self {
        self.path.push(PathStep::Start(destination));
        return self;
    }

    pub fn descend(&mut self, selector: &'a str, number_of_element: i32) -> &mut Self {
        self.path.push(PathStep::Descend(Destination::new(
            selector,
            ElementSelection::Single(number_of_element),
        )));
        return self;
    }

    pub fn populate(&mut self, population_map: HashMap<String, FieldIdentity>) -> &mut Self {
        self.path.push(PathStep::Populate(population_map));
        return self;
    }

    pub fn populate_one(
        &mut self,
        field_name: &'a str,
        selector: &'a str,
        selection: ElementSelection,
        location: DestinationLocation,
    ) -> &mut Self {
        let mut population = HashMap::new();
        population.insert(
            String::from(field_name),
            FieldIdentity {
                destination: Destination::new(selector, selection),
                destination_location: location,
            },
        );
        self.path.push(PathStep::Populate(population));
        return self;
    }

    pub fn find_one(
        &mut self,
        selector: &'a str,
        number_of_element: i32,
        location: DestinationLocation,
    ) -> &mut Self {
        self.path.push(PathStep::Find(FieldIdentity {
            destination: Destination(
                String::from(selector),
                ElementSelection::Single(number_of_element),
            ),
            destination_location: location,
        }));
        return self;
    }

    pub fn find_all(
        &mut self,
        selector: &'a str,
        delimiter: &'a str,
        location: DestinationLocation,
    ) -> &mut Self {
        self.path.push(PathStep::Find(FieldIdentity {
            destination: Destination(
                String::from(selector),
                ElementSelection::All(String::from(delimiter)),
            ),
            destination_location: location,
        }));
        return self;
    }

    /// Returns the constructed path
    pub fn build(&self) -> Vec<PathStep> {
        self.path.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn test_find_single_element_without_descent_by_path() {
        let html_string = r#"<div><a>NOT THIS</a> <p>find me</p></div>"#;
        let html = Html::parse_fragment(&html_string);
        let html = RefCell::new(&html);
        let path = PathBuilder::new()
            .start(Destination::new("div", ElementSelection::first()))
            .find_one("p", 0, DestinationLocation::Text)
            .build();
        let mut path_finder = PathFinder::new(&path, html.borrow());

        path_finder.search_path();

        assert_eq!(path_finder.values.get(0).unwrap(), "find me");
    }

    #[test]
    fn test_find_single_element_with_descent_by_path() {
        let html_string = r#"<div><a><i>NOT THIS</i> <p>find me</p></a> <p>find me</p></div>"#;
        let html = Html::parse_fragment(&html_string);
        let html = RefCell::new(&html);
        let path = PathBuilder::new()
            .start(Destination::new("div", ElementSelection::first()))
            .descend("a", 0)
            .find_one("p", 0, DestinationLocation::Text)
            .build();
        let mut path_finder = PathFinder::new(&path, html.borrow());

        path_finder.search_path();

        assert_eq!(path_finder.values.get(0).unwrap(), "find me");
    }

    #[test]
    fn test_find_all_element_with_descent_by_path() {
        let html_string =
            r#"<div><a><i>NOT THIS</i> <p>find me</p> <p>as well</p></a> <p>find me</p></div>"#;
        let html = Html::parse_fragment(&html_string);
        let html = RefCell::new(&html);
        let path = PathBuilder::new()
            .start(Destination::new("div", ElementSelection::first()))
            .descend("a", 0)
            .find_all("p", "", DestinationLocation::Text)
            .build();
        let mut path_finder = PathFinder::new(&path, html.borrow());

        path_finder.search_path();

        assert_eq!(path_finder.values.get(0).unwrap(), "find me");
        assert_eq!(path_finder.values.get(1).unwrap(), "as well");
    }

    #[test]
    fn test_populate_element_by_path() {
        let html_string = r#"<div class="first"><span itemprop="first">find me</span>
        <span itemprop="second">as well</span></div>"#;
        let html = Html::parse_fragment(&html_string);
        let html = RefCell::new(&html);
        let mut population = HashMap::new();
        population.insert(
            String::from("first"),
            FieldIdentity {
                destination: Destination::new(
                    r#"span[itemprop="first"]"#,
                    ElementSelection::first(),
                ),
                destination_location: DestinationLocation::Text,
            },
        );
        population.insert(
            String::from("second"),
            FieldIdentity {
                destination: Destination::new(
                    r#"span[itemprop="second"]"#,
                    ElementSelection::first(),
                ),
                destination_location: DestinationLocation::Text,
            },
        );
        let path = PathBuilder::new()
            .start(Destination::new(
                r#"div[class="first"]"#,
                ElementSelection::first(),
            ))
            .populate(population)
            .build();
        let mut path_finder = PathFinder::new(&path, html.borrow());

        path_finder.search_path();

        assert_eq!(path_finder.map.get("first").unwrap(), "find me");
        assert_eq!(path_finder.map.get("second").unwrap(), "as well");
    }

    #[test]
    fn test_find_all_with_different_parents_by_path() {
        let html_string = r#"<div><a><i>find me</i></a><a><i>as well</i></a></div>"#;

        let html = Html::parse_fragment(&html_string);
        let html = RefCell::new(&html);
        let path = PathBuilder::new()
            .start(Destination::new(r#"div"#, ElementSelection::first()))
            .find_all("i", "", DestinationLocation::Text)
            .build();
        let mut path_finder = PathFinder::new(&path, html.borrow());

        path_finder.search_path();

        assert_eq!(path_finder.values.get(0).unwrap(), "find me");
        assert_eq!(path_finder.values.get(1).unwrap(), "as well");
    }

    #[test]
    fn test_populate_multiple_time_by_path() {
        let html_string = r#"<div class="first"><span itemprop="first">find me</span>
        <span itemprop="second">as well</span></div>"#;
        let html = Html::parse_fragment(&html_string);
        let html = RefCell::new(&html);
        let mut population = HashMap::new();
        let mut second_population = HashMap::new();
        population.insert(
            String::from("first"),
            FieldIdentity {
                destination: Destination::new(
                    r#"span[itemprop="first"]"#,
                    ElementSelection::first(),
                ),
                destination_location: DestinationLocation::Text,
            },
        );
        population.insert(
            String::from("second"),
            FieldIdentity {
                destination: Destination::new(
                    r#"span[itemprop="second"]"#,
                    ElementSelection::first(),
                ),
                destination_location: DestinationLocation::Text,
            },
        );
        second_population.insert(
            String::from("third"),
            FieldIdentity {
                destination: Destination::new(
                    r#"span[itemprop="first"]"#,
                    ElementSelection::first(),
                ),
                destination_location: DestinationLocation::Text,
            },
        );
        let path = PathBuilder::new()
            .start(Destination::new(
                r#"div[class="first"]"#,
                ElementSelection::first(),
            ))
            .populate(population)
            .populate(second_population)
            .build();
        let mut path_finder = PathFinder::new(&path, html.borrow());

        path_finder.search_path();

        assert_eq!(path_finder.map.get("first").unwrap(), "find me");
        assert_eq!(path_finder.map.get("second").unwrap(), "as well");
        assert_eq!(path_finder.map.get("third").unwrap(), "find me");
    }

    #[test]
    fn test_populate_one_field_by_path() {
        let html_string = r#"<div class="first"><span itemprop="first">find me</span>
        <span itemprop="second">as well</span></div>"#;
        let html = Html::parse_fragment(&html_string);
        let html = RefCell::new(&html);

        let path = PathBuilder::new()
            .start(Destination::new(
                r#"div[class="first"]"#,
                ElementSelection::first(),
            ))
            .populate_one(
                "first",
                r#"span[itemprop="first"]"#,
                ElementSelection::first(),
                DestinationLocation::Text,
            )
            .build();
        let mut path_finder = PathFinder::new(&path, html.borrow());

        path_finder.search_path();

        assert_eq!(path_finder.map.get("first").unwrap(), "find me");
    }

}
