use crate::field::extract;
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
pub enum PathStep {
    Descend(Destination),
    Start(Destination),
    Populate(HashMap<String, FieldIdentity>),
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
        let mut selected = self
            .html
            .select(&parsed)
            .nth(n as usize)
            .expect("Path not found!");

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
                    let child_element = element
                        .select(&selection)
                        .nth(n as usize)
                        .expect("Element not found");
                    self.resolve_path(&child_element, level + 1);
                }
                PathStep::Populate(field_map) => {
                    for (field_name, identifier) in field_map {
                        let selector = Selector::parse(&identifier.destination.0).unwrap();
                        let mut selection = element.select(&selector);
                        match &identifier.destination.1 {
                            ElementSelection::Single(n) => {
                                let selected_element =
                                    selection.nth(*n as usize).expect("Element not found");
                                self.map.insert(
                                    field_name.clone(),
                                    extract(&selected_element, &identifier.destination_location),
                                );
                            }
                            ElementSelection::All(delimiter) => {
                                let mut value = String::new();
                                for selected_element in selection {
                                    value += &(extract(
                                        &selected_element,
                                        &identifier.destination_location,
                                    ) + &delimiter.clone());
                                }
                                self.map.insert(field_name.clone(), value);
                            }
                        }
                    }
                }
                PathStep::Find(field_identifier) => self.find(element, field_identifier),

                _ => panic!("Invalid path!"),
            },
            None => (),
        };
    }

    fn find(&mut self, element: &ElementRef, identifier: &FieldIdentity) {
        let selector = Selector::parse(&identifier.destination.0).unwrap();
        let mut selection = element.select(&selector);

        match &identifier.destination.1 {
            ElementSelection::Single(n) => {
                let child_element = selection.nth(*n as usize).expect("Element not found");
                self.values
                    .push(extract(&child_element, &identifier.destination_location));
            }
            ElementSelection::All(_) => {
                for child_element in selection {
                    self.values
                        .push(extract(&child_element, &identifier.destination_location));
                }
            }
        }
    }
}

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

    pub fn find_all(&mut self,
                    selector: &'a str,
                    delimiter: &'a str,
                    location: DestinationLocation) -> &mut Self {
        self.path.push(PathStep::Find(FieldIdentity {
            destination: Destination(
                String::from(selector),
                ElementSelection::All(String::from(delimiter)),
            ),
            destination_location: location,
        }));
        return self;
    }

    pub fn build(&self) -> Vec<PathStep> {
        self.path.to_owned()
    }
}
