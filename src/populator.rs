//! This module provides the main logic of the library
//!
use crate::field::Destination;
use crate::field::DestinationLocation;
use crate::field::ElementSelection;
use crate::field::FieldIdentity;
use crate::field::FieldPopulator;
use crate::path::PathFinder;
use crate::path::PathStep;
use reqwest;
use scraper::element_ref;
use scraper::html;
use scraper::ElementRef;
use scraper::{Html, Selector};
use std::cell::Ref;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::prelude::v1::Vec;
use std::str::FromStr;
use std::string::ToString;
use std::sync::Arc;
use std::sync::Barrier;
use std::sync::Mutex;
use std::sync::RwLock;
use threadpool::ThreadPool;

pub type Fields<'a> = HashMap<&'a str, FieldIdentity>;
pub type Path = Vec<PathStep>;
pub type Paths = Vec<Path>;

/// HTML extractor on a single HTML structure
pub struct SinglePopulator<'a> {
    /// Parsed Html struct
    html: Html,
    search_detail: SearchDetail<'a>,
    /// A map, that contains populated field_names
    pub map: HashMap<String, String>,
    /// Values, that are populated without specifying the field_name (eg. extracting links etc..)
    pub values: Vec<String>,
}

impl<'a> SinglePopulator<'a> {
    pub fn new(html_string: &'a str, search: SearchDetail<'a>) -> SinglePopulator<'a> {
        //        let mut html_string = reqwest::get(url).unwrap();
        let map = HashMap::new();
        let values = Vec::new();
        let html = Html::parse_fragment(html_string);
        SinglePopulator {
            html,
            search_detail: search,
            map,
            values,
        }
    }

    pub fn populate(&mut self) {
        let mut populated_fields: HashMap<String, String> = HashMap::new();
        let mut values = Vec::<String>::new();
        let html = RefCell::new(&self.html);

        for (field_name, field) in &self.search_detail.fields {
            let mut populator = FieldPopulator::new(html.borrow(), &field);
            populator.find_field();
            self.map.insert(
                field_name.to_string(),
                populator.value.unwrap_or_else(|| "".to_string()),
            );
        }

        for path in &self.search_detail.paths {
            let mut path_finder = PathFinder::new(&path, html.borrow());
            path_finder.search_path();
            for (k, v) in path_finder.map {
                self.map.insert(k, v);
            }
            self.values.extend(path_finder.values);
        }
    }
}

type ThreadSafeLinks = Arc<Mutex<Vec<HashMap<String, String>>>>;

/// The populator usable on multiple identical HTML structure (link crawling).
pub struct MultiplePopulator<'a> {
    html: Html,
    /// Multiple populated map
    pub populated_links: Vec<HashMap<String, String>>,
    paralell_populated_links: ThreadSafeLinks,
    links_path: Path,
    /// A simple converter function that takes the link as an argument. Use it when the HTML structure
    /// only contains a relative path instead of an absolute url.
    link_callback: Option<&'a Fn(String) -> Option<String>>,
    search_detail: SearchDetail<'static>,
}

impl<'a> MultiplePopulator<'a> {
    pub fn new(
        url: &str,
        links_path: Path,
        link_converter: Option<&'a Fn(String) -> Option<String>>,
        search: SearchDetail<'static>,
    ) -> MultiplePopulator<'a> {
        let mut html_string = reqwest::get(url).expect("Can't connect to url!");
        let html = Html::parse_fragment(&html_string.text().unwrap_or("".to_string()));
        let populated_links = Vec::<HashMap<String, String>>::new();
        let paralell_populated_links = Arc::new(Mutex::new(Vec::new()));
        MultiplePopulator {
            html,
            populated_links,
            paralell_populated_links,
            links_path,
            link_callback: link_converter,
            search_detail: search,
        }
    }

    /// Start single threaded population based on the link path.
    pub fn populate(&mut self) {
        let html = RefCell::new(&self.html);

        let mut path_finder = PathFinder::new(&self.links_path, html.borrow());
        path_finder.search_path();

        for link in path_finder.values {

            let link = match self.link_callback {
                Some(callback) => match (callback)(link.clone()) {
                    Some(link) => link,
                    None => continue,
                },
                None => link,
            };

            let link_html = reqwest::get(&link)
                .expect("Link not found")
                .text()
                .expect("Unable to extract html from link");

            let mut populator = SinglePopulator::new(&link_html, self.search_detail.clone());
            populator.populate();
            self.populated_links.push(populator.map);
        }
    }

    /// Start multithreaded population based on the link path.
    pub fn par_populate(&mut self) {
        let html = RefCell::new(&self.html);
        let mut path_finder = PathFinder::new(&self.links_path, html.borrow());
        path_finder.search_path();
        let pool = ThreadPool::new(path_finder.values.len());

        for link in path_finder.values {
            let results = self.paralell_populated_links.clone();

            let link = match self.link_callback {
                Some(callback) => match (callback)(link.clone()) {
                    Some(link) => link,
                    None => continue,
                },
                None => link,
            };
            let search = self.search_detail.clone();
            pool.execute(move || {
                let link_html = reqwest::get(&link)
                    .expect("Link not found")
                    .text()
                    .expect("Unable to extract html from link");
                let mut populator = SinglePopulator::new(&link_html, search);
                populator.populate();
                results.lock().unwrap().push(populator.map);
            });
        }

        pool.join();
        self.populated_links = self.paralell_populated_links.lock().unwrap().clone();
    }
}

#[derive(Clone)]
/// A struct holding the search parameters.
pub struct SearchDetail<'a> {
    paths: Paths,
    fields: Fields<'a>,
}

impl<'a> SearchDetail<'a> {
    pub fn new() -> Self {
        let paths = Vec::<Vec<PathStep>>::new();
        let fields = HashMap::<&'a str, FieldIdentity>::new();
        SearchDetail { paths, fields }
    }

    /// Insert a field to be populated in the process. Use this, when the HTML element could be extracted
    /// unambigously
    pub fn insert_field(
        &mut self,
        field_name: &'a str,
        selector: &'a str,
        location: DestinationLocation,
        element_number: ElementSelection,
    ) {
        self.fields.insert(
            field_name,
            FieldIdentity {
                destination: Destination(String::from(selector), element_number),
                destination_location: location,
            },
        );
    }

    /// A specialized form of field population.
    pub fn insert_attr_field(
        &mut self,
        field_name: &'a str,
        selector: &'a str,
        attr_name: &'a str,
        element_number: ElementSelection,
    ) {
        self.fields.insert(
            field_name,
            FieldIdentity {
                destination: Destination(String::from(selector), element_number),
                destination_location: DestinationLocation::Attr(String::from(attr_name)),
            },
        );
    }

    /// When a field can not be distinguished (eg. a simple `<div>` element, that is unlikely to be unique), a path
    /// must be used to extract the element.
    pub fn insert_path(&mut self, path: Path) {
        self.paths.push(path);
    }
}
