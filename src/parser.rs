use crate::field::DestinationLocation;
use crate::field::ElementSelection;
use crate::field::{Destination, FieldIdentity};
use crate::path::PathBuilder;
use crate::path::PathStep;
use crate::populator::Path;
use crate::populator::SearchDetail;
use crate::populator::SinglePopulator;
use crate::populator::{MultiplePopulator, Paging, PagingOptions, PagingRange};
use colored::*;
use core::borrow::Borrow;
use matches;
use prettytable::{format, Attr, Cell, Row, Table};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::iter::repeat;
use std::result;

const START: &str = "start";

type Result<T> = result::Result<T, ParseError>;

#[derive(Debug)]
pub struct ParseError {
    step: String,
    error: String,
}

impl ParseError {
    pub fn new(step: &str, message: &str) -> ParseError {
        ParseError {
            step: String::from(step),
            error: String::from(message),
        }
    }
}

impl Error for ParseError {}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter) -> result::Result<(), fmt::Error> {
        let underline: String = repeat("^").take(self.step.len()).collect();
        write!(
            f,
            "{:10}: {:10} \n{:10} \n{:10}",
            "error".red(),
            &self.error,
            &self.step,
            &underline.red(),
        )
    }
}

#[derive(Serialize, Debug, Deserialize)]
pub struct Config {
    pub meta: Meta,
    pub paths: Vec<String>,
    pub fields: HashMap<String, String>,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct Meta {
    pub populator: String,
    pub link_path: Option<String>,
    pub base_url: String,
    pub paging: Option<String>,
    pub prepend_links: Option<String>,
}

pub struct Parser {
    config: Config,
}

pub enum Populator<'a> {
    Single(SinglePopulator<'a>),
    Multiple(MultiplePopulator),
}

impl<'a> Populator<'a> {
    pub fn run(&mut self) {
        match self {
            Populator::Single(ref mut spop) => spop.populate(),
            Populator::Multiple(ref mut mpop) => {
                mpop.run();
            }
        }
    }

    pub fn print(&self) {
        println!("Printing populator result...");

        match self {
            Populator::Single(spop) => {
                self.print_map_table(&[spop.map.clone()]);
                self.print_value_table(&spop.values);
            }
            Populator::Multiple(mpop) => {
                self.print_map_table(mpop.populated_links.as_slice())
            }
        }
    }

    fn print_map_table(&self, maps: &[HashMap<String, String>]) {
        if maps.len() == 0 {
            return;
        }
        println!();
        let mut table = Table::new();
        table.set_titles(Row::new(vec![
            Cell::new("Page").with_style(Attr::Bold),
            Cell::new("Map").with_style(Attr::Bold),
        ]));
        maps.iter().enumerate().for_each(|(i, map)| {
            &table.add_row(Row::new(vec![
                Cell::new(&format!("{}", i)),
                Cell::new(&format!("{:?}", map)),
            ]));
        });
        table.printstd();
    }

    fn print_value_table(&self, values: &Vec<String>) {
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
        table.set_titles(Row::new(vec![Cell::new("Values").with_style(Attr::Bold)]));
        values.iter().for_each(|v| {
            &table.add_row(Row::new(vec![Cell::new(&v)]));
        });
        table.printstd();
    }
}

impl Parser {
    pub fn new(config: Config) -> Parser {
        Parser { config }
    }

    pub fn build(&mut self) -> Populator {
        match self.config.meta.populator.as_str() {
            "single" => Populator::Single(self.build_single()),
            "multiple" => Populator::Multiple(self.build_multiple()),
            _ => panic!("Invalid populator type. Use 'single' or 'multiple'!"),
        }
    }

    fn build_single(&mut self) -> SinglePopulator {
        let mut details = SearchDetail::new();
        for path in &self.config.paths {
            let mut resolver = PathResolver::new(&path);
            details.insert_path(resolver.resolve());
        }
        //        for field in self.config.fields {
        //            details.insert_field()
        //        }
        SinglePopulator::new(&self.config.meta.base_url, details)
    }

    fn build_multiple(&mut self) -> MultiplePopulator {
        if self.config.meta.link_path.is_none() {
            panic!("Link path must be provided for multiple populator.");
        }
        let paging = match &self.config.meta.paging {
            Some(p) => Paging::Enabled(PagingOptions {
                extension: p.clone(),
                range: PagingRange::Indefinite,
            }),
            None => Paging::Disabled,
        };

        let mut link_path = PathResolver::new(self.config.meta.link_path.as_ref().unwrap());

        let mut details = SearchDetail::new();
        for path in &self.config.paths {
            let mut resolver = PathResolver::new(&path);
            details.insert_path(resolver.resolve());
        }
        MultiplePopulator::new(
            &self.config.meta.base_url,
            link_path.resolve(),
            self.config.meta.prepend_links.clone(),
            details,
            true,
        )
    }
}

struct PathResolver {
    path_tokens: Vec<String>,
    path: PathBuilder,
    current: usize,
    map_buffer: Option<HashMap<String, FieldIdentity>>,
    pub errors: Vec<ParseError>,
}

impl PathResolver {
    pub fn new(path_string: &str) -> PathResolver {
        let path_tokens: Vec<String> = path_string
            .replace(" ", "")
            .split("->")
            .map(|p| String::from(p))
            .collect();
        let path = PathBuilder::new();
        let errors = Vec::<ParseError>::new();
        let map_buffer = HashMap::new();
        PathResolver {
            path_tokens,
            current: 0,
            path,
            map_buffer: Some(map_buffer),
            errors,
        }
    }

    pub fn resolve(&mut self) -> Path {
        let first = self.query_current_step();
        if first.to_lowercase().starts_with("start") {
            if let Err(error) = self.resolve_start() {
                self.errors.push(error);
            }
            self.current += 1;
        } else {
            self.errors.push(ParseError::new(
                &first,
                "First path command must be a start!",
            ));
        }

        while self.current + 1 <= self.path_tokens.len() {
            match self.path_tokens.get(self.current) {
                Some(token) => {
                    if token.to_lowercase().starts_with("descend") {
                        if let Err(error) = self.resolve_descend() {
                            self.errors.push(error)
                        }
                    } else if token.to_lowercase().starts_with("find") {
                        if let Err(error) = self.resolve_find() {
                            self.errors.push(error)
                        }
                    } else if token.to_lowercase().starts_with("populate") {
                        if let Err(error) = self.resolve_populate() {
                            self.errors.push(error)
                        }
                    } else {
                        self.errors.push(ParseError::new(&token, "Invalid command"))
                    }
                }
                None => (),
            }
            self.current += 1;
        }

        if self.errors.len() != 0 {
            self.errors.iter().for_each(|error| println!("{}", error));
        }

        if let Some(map) = &self.map_buffer {
            if map.len() != 0 {
                self.path.populate(self.map_buffer.take().unwrap());
            }
        }
        self.path.build()
    }

    fn resolve_start(&mut self) -> Result<()> {
        let args: HashMap<String, String> = self.extract_args()?;
        let selector = self.extract_selector_string(&args)?;
        let select = self.extract_select_number(&args)?;
        self.path.start(Destination::new(
            &selector,
            ElementSelection::Single(select),
        ));
        Ok(())
    }

    fn resolve_descend(&mut self) -> Result<()> {
        let args = self.extract_args()?;
        let selector = self.extract_selector_string(&args)?;
        let select = self.extract_select_number(&args)?;
        self.path.descend(&selector, select);
        Ok(())
    }

    fn resolve_populate(&mut self) -> Result<()> {
        let args = self.extract_args()?;
        let selector = self.extract_selector_string(&args)?;
        let location = self.extract_location(&args)?;
        let field = self.extract_field_name(&args)?;
        let select = match args.get("select") {
            Some(select) => {
                if select.to_lowercase().starts_with("all") {
                    ElementSelection::All(Self::extract_between_brackets(&select)?)
                } else {
                    ElementSelection::Single(self.extract_select_number(&args)?)
                }
            }
            None => ElementSelection::Single(0),
        };

        let identity = FieldIdentity {
            destination: Destination::new(&selector, select),
            destination_location: location,
        };
        if let Some(ref mut map) = self.map_buffer {
            map.insert(field, identity);
        }
        Ok(())
    }

    fn resolve_find(&mut self) -> Result<()> {
        let args = self.extract_args()?;
        let selector = self.extract_selector_string(&args)?;
        let location = self.extract_location(&args)?;
        match args.get("select") {
            Some(select) => {
                if select.to_lowercase().starts_with("all") {
                    self.path.find_all(&selector, location);
                } else {
                    self.path
                        .find_one(&selector, self.extract_select_number(&args)?, location);
                }
            }
            None => (),
        };

        Ok(())
    }

    fn extract_args(&self) -> Result<HashMap<String, String>> {
        let token = self.path_tokens.get(self.current).unwrap();
        let token = Self::extract_between_brackets(&token)?;

        let args: Vec<&str> = token.split(",").collect();
        let args: Vec<Vec<&str>> = args
            .iter()
            .map(|a| a.split(':').collect::<Vec<&str>>())
            .collect();

        let mut arg_map: HashMap<String, String> = HashMap::new();
        args.iter().for_each(|a| {
            &arg_map.insert(
                a.get(0).unwrap().to_lowercase(),
                a.get(1).unwrap().to_lowercase(),
            );
        });
        Ok(arg_map)
    }

    fn extract_field_name(&self, args: &HashMap<String, String>) -> Result<String> {
        let field_name = match args.get("name") {
            Some(name) => name,
            None => {
                return Err(ParseError::new(
                    &self.query_current_step(),
                    "Missing field name",
                ));
            }
        };
        Ok(field_name.clone())
    }
    fn extract_location(&self, args: &HashMap<String, String>) -> Result<DestinationLocation> {
        let location = match args.get("loc") {
            Some(loc) => {
                if loc.to_lowercase().starts_with("text") {
                    DestinationLocation::Text
                } else if loc.to_lowercase().starts_with("attr") {
                    DestinationLocation::Attr(Self::extract_between_brackets(loc)?)
                } else {
                    return Err(ParseError::new(
                        &self.query_current_step(),
                        "Invalid location",
                    ));
                }
            }
            None => {
                return Err(ParseError::new(
                    &self.query_current_step(),
                    "Missing location",
                ));
            }
        };

        Ok(location)
    }

    fn extract_selector_string(&self, args: &HashMap<String, String>) -> Result<String> {
        match args.get("selector") {
            Some(s) => Ok(s.clone()),
            None => {
                return Err(ParseError::new(
                    &self.query_current_step(),
                    "Missing selector string",
                ));
            }
        }
    }

    fn extract_select_number(&self, args: &HashMap<String, String>) -> Result<i32> {
        match args.get("select") {
            Some(s) => match s.parse::<i32>() {
                Ok(n) => Ok(n),
                Err(_) => {
                    return Err(ParseError::new(
                        &self.query_current_step(),
                        "Invalid select element number",
                    ));
                }
            },
            None => Ok(0),
        }
    }

    fn extract_between_brackets(token: &str) -> Result<String> {
        let starting_bracket_index = match token.find("(") {
            Some(i) => i,
            None => {
                return Err(ParseError::new(
                    &token,
                    "Command must start with opening bracket",
                ));
            }
        };

        let closing_bracket_index = match token.rfind(")") {
            Some(i) => i,
            None => return Err(ParseError::new(&token, "Unclosed parentheses")),
        };

        Ok(token[starting_bracket_index + 1..closing_bracket_index].to_string())
    }

    fn query_current_step(&self) -> String {
        self.path_tokens.get(self.current).unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_parse() {
        let path = r#"START(SELECTOR: div, SELECT: 0)"#;
        let mut resolver = PathResolver::new(path);

        let path = resolver.resolve();

        assert_eq!(1, path.len());
        matches::assert_matches!(path.get(0).unwrap(), PathStep::Start(_));
    }

    #[test]
    fn test_missing_selector_start_parse() {
        let path = r#"START(SELECT: 0)"#;
        let mut resolver = PathResolver::new(path);

        let path = resolver.resolve();

        assert_eq!(2, resolver.errors.len());
        assert_eq!(0, path.len());
    }

    #[test]
    fn test_missing_select_start_parse() {
        let path = r#"START(SELECTOR: div)"#;
        let mut resolver = PathResolver::new(path);

        let path = resolver.resolve();

        assert_eq!(1, path.len());
        match path.get(0).unwrap() {
            PathStep::Start(d) => match d.1 {
                ElementSelection::Single(n) => assert_eq!(0, n),
                _ => panic!("Failed"),
            },
            _ => panic!("Failed"),
        }
    }

    #[test]
    fn test_invalid_select_number_start_parse() {
        let path = r#"START(SELECTOR: div, SELECT: a32)"#;
        let mut resolver = PathResolver::new(path);

        let path = resolver.resolve();

        assert_eq!(2, resolver.errors.len());
        assert_eq!(0, path.len());
    }

    #[test]
    fn test_descend_parse() {
        let path = r#"START(SELECTOR: test) -> DESCEND(SELECTOR: div, SELECT: 0)"#;
        let mut resolver = PathResolver::new(path);

        let path = resolver.resolve();

        assert_eq!(2, path.len());
        matches::assert_matches!(path.get(1).unwrap(), PathStep::Descend(_));
    }

    #[test]
    fn test_find_parse() {
        let path = r#"START(SELECTOR: test) -> FIND(SELECTOR: div, SELECT: 0, LOC: ATTR(test))"#;
        let mut resolver = PathResolver::new(path);

        let path = resolver.resolve();

        resolver.errors.iter().for_each(|p| println!("{}", p));
        assert_eq!(2, path.len());
        matches::assert_matches!(path.get(1).unwrap(), PathStep::Find(_));
    }

    #[test]
    fn test_missing_location_find_parse() {
        let path = r#"FIND(SELECTOR: div, SELECT: 0)"#;
        let mut resolver = PathResolver::new(path);

        let path = resolver.resolve();

        resolver.errors.iter().for_each(|p| println!("{}", p));
        assert_eq!(2, resolver.errors.len());
    }

    #[test]
    fn test_invalid_location_find_parse() {
        let path = r#"FIND(SELECTOR: div, SELECT: 0, LOC: INVALID)"#;
        let mut resolver = PathResolver::new(path);

        let path = resolver.resolve();

        resolver.errors.iter().for_each(|p| println!("{}", p));
        assert_eq!(2, resolver.errors.len());
    }

    #[test]
    fn test_invalid_command_parse() {
        let path = r#"NONEXISTINGCOMMAND(NAME: test, SELECTOR: div, SELECT: 0, LOC: TEXT)"#;
        let mut resolver = PathResolver::new(path);

        let path = resolver.resolve();

        resolver.errors.iter().for_each(|p| println!("{}", p));
        assert_eq!(2, resolver.errors.len());
    }

    #[test]
    fn test_first_command_not_start_parse() {
        let path = r#"FIND(SELECTOR: div, SELECT: 0, LOC: TEXT)"#;
        let mut resolver = PathResolver::new(path);

        let path = resolver.resolve();

        resolver.errors.iter().for_each(|p| println!("{}", p));
        assert_eq!(1, resolver.errors.len());
    }

}
