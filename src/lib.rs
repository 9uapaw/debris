//! A declarative HTML data extractor
//!
//! This library helps structuring scraping scripts in a declarative way. It uses [`reqwest`] and [`scraper`]
//! under the hood.
//!
//! # Organization
//!
//! There are two primary structs that are to be used. [`SinglePopulator`] is used to extract information
//! from one link. However, the general case is that several identical HTML structures need to be extracted simultaneously.
//! The [`MultiplePopulator`] is created exactly for this reason.
//!
mod path;
mod field;
mod populator;

pub mod declare {
    pub use crate::path::PathBuilder;
    pub use crate::path::PathStep;
    pub use crate::field::DestinationLocation;
    pub use crate::field::Destination;
    pub use crate::field::ElementSelection;
    pub use crate::field::FieldIdentity;
    pub use crate::populator::SearchDetail;
    pub use crate::populator::Path;
    pub use crate::populator::Paths;
    pub use crate::populator::Fields;
}

pub mod population {
    pub use crate::populator::SinglePopulator;
    pub use crate::populator::MultiplePopulator;
}

