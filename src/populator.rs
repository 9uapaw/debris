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
use scraper::Html;
use std::cell::RefCell;
use std::collections::HashMap;
use std::prelude::v1::Vec;
use std::string::ToString;
use std::sync::Arc;
use std::sync::Mutex;
use threadpool::ThreadPool;

pub type Fields<'a> = HashMap<&'a str, FieldIdentity>;
pub type Path = Vec<PathStep>;
pub type Paths = Vec<Path>;

pub enum Paging {
    Disabled,
    Enabled(PagingOptions),
}

pub struct PagingOptions {
    pub extension: String,
    pub range: PagingRange,
}

pub enum PagingRange {
    Indefinite,
    Page(i32),
}
/// HTML extractor on a single HTML structure
pub struct SinglePopulator<'a> {
    url: String,
    search_detail: SearchDetail<'a>,
    /// A map, that contains populated field_names
    pub map: HashMap<String, String>,
    /// Values, that are populated without specifying the field_name (eg. extracting links etc..)
    pub values: Vec<String>,
}

impl<'a> SinglePopulator<'a> {
    pub fn new(url:&'a str, search: SearchDetail<'a>) -> SinglePopulator<'a> {
        let map = HashMap::new();
        let values = Vec::new();
        SinglePopulator {
            url: String::from(url),
            search_detail: search,
            map,
            values,
        }
    }

    pub fn populate(&mut self) {
        let html = get_html(&self.url);
        let html = RefCell::new(&html);

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
pub struct MultiplePopulator {
    url: String,
    /// Multiple populated map
    pub populated_links: Vec<HashMap<String, String>>,
    links_path: Path,
    /// A simple converter function that takes the link as an argument. Use it when the HTML structure
    /// only contains a relative path instead of an absolute url.
    link_prefix: Option<String>,
    search_detail: SearchDetail<'static>,
    paging: Paging,
    multi_thread: bool,
}

impl<'a> MultiplePopulator {
    pub fn new(
        url: &str,
        links_path: Path,
        link_converter: Option<String>,
        search: SearchDetail<'static>,
        multi_thread: bool,
    ) -> MultiplePopulator {
        let populated_links = Vec::<HashMap<String, String>>::new();
        MultiplePopulator {
            url: String::from(url),
            populated_links,
            links_path,
            link_prefix: link_converter,
            search_detail: search,
            paging: Paging::Disabled,
            multi_thread,
        }
    }

    pub fn new_with_paging(
        url: &str,
        links_path: Path,
        link_converter: Option<String>,
        search: SearchDetail<'static>,
        multi_thread: bool,
        paging_option: PagingOptions,
    ) -> MultiplePopulator {
        let mut html_string = reqwest::get(url).expect("Can't connect to url!");
        let html = Html::parse_fragment(&html_string.text().unwrap_or("".to_string()));
        let populated_links = Vec::<HashMap<String, String>>::new();
        MultiplePopulator {
            url: String::from(url),
            populated_links,
            links_path,
            link_prefix: link_converter,
            search_detail: search,
            paging: Paging::Enabled(paging_option),
            multi_thread,
        }
    }

    pub fn run(&mut self) -> Result<&Vec<HashMap<String, String>>, String> {
        match &self.paging {
            Paging::Enabled(options) => match options.range {
                PagingRange::Indefinite => {
                    let mut page = 0;
                    loop {
                        let link = format!("{}{}", self.url, options.extension)
                            .replace("{}", &page.to_string());
                        let html = get_html(&link);
                        let mut result = self.populate(html)?;
                        if result.len() == 0 {
                            break;
                        }
                        self.populated_links.append(&mut result);
                        page += 1;
                    }
                }
                PagingRange::Page(n) => {
                    for i in 0..n {
                        let link = format!("{}{}", self.url, options.extension)
                            .replace("{}", (&i.to_string()));
                        let html = get_html(&link);
                        let mut result = self.populate(html)?;
                        self.populated_links.append(&mut result);
                    }
                }
            },
            Paging::Disabled => {
                let html = get_html(&self.url);
                let mut result = self.populate(html)?;
                self.populated_links.append(&mut result);
            }
        }
        Ok(&self.populated_links)
    }

    fn populate(&self, html: Html) -> Result<Vec<HashMap<String, String>>, String> {
        match self.multi_thread {
            true => self.par_populate(html),
            false => self.single_populate(html),
        }
    }

    /// Start single threaded population based on the link path.
    fn single_populate(&self, html: Html) -> Result<Vec<HashMap<String, String>>, String> {
        let html = RefCell::new(&html);

        let mut populated_links: Vec<HashMap<String, String>> = Vec::new();
        let mut path_finder = PathFinder::new(&self.links_path, html.borrow());
        path_finder.search_path();

        for link in path_finder.values {
            let link = match &self.link_prefix {
                Some(prefix) => prefix.clone() + &link,
                None => link,
            };

//            let link_html = match reqwest::get(&link) {
//                Ok(mut response) => match response.text() {
//                    Ok(html) => html,
//                    Err(_) => {
//                        return Err(String::from("Unable to extract html from link"));
//                    }
//                },
//                Err(_) => {
//                    return Err(String::from("Link not found"));
//                }
//            };

            let mut populator = SinglePopulator::new(&link, self.search_detail.clone());
            populator.populate();
            populated_links.push(populator.map);
        }
        Ok(populated_links)
    }

    /// Start multithreaded population based on the link path.
    fn par_populate(&self, html: Html) -> Result<Vec<HashMap<String, String>>, String> {
        let html = RefCell::new(&html);
        let mut path_finder = PathFinder::new(&self.links_path, html.borrow());
        let all_results: Vec<HashMap<String, String>> = Vec::new();
        let paralell_populated_links = Arc::new(Mutex::new(all_results));
        path_finder.search_path();

        if path_finder.values.len() == 0 {
            return Err(String::from("No link found"));
        }

        let pool = ThreadPool::new(path_finder.values.len());

        for link in path_finder.values {
            let results = paralell_populated_links.clone();

            let link = match &self.link_prefix {
                Some(prefix) => prefix.clone() + &link,
                None => link,
            };

            let search = self.search_detail.clone();
            pool.execute(move || {
                let mut populator = SinglePopulator::new(&link, search);
                populator.populate();
                results.lock().unwrap().push(populator.map);
            });
        }

        pool.join();
        let result = paralell_populated_links.lock().unwrap().clone();
        Ok(result)
    }

}

fn get_html(url: &str) -> Html {
    let mut html_string = reqwest::get(url).expect("URL not found");
    Html::parse_fragment(&html_string.text().unwrap_or("".to_string()))
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
