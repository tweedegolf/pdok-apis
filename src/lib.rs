//! The geocoding service by PDOK.
//! Used to generate references to the desired lots and buildings.
//!
//! See [the service documentation](https://www.pdok.nl/introductie/-/article/pdok-locatieserver)
//! for more information on its capabilities.

pub mod bag;
pub mod brk;
pub mod lookup;

#[derive(Debug)]
pub enum Error {
    /// Something went wrong with the request (invalid url, no connection, etc)
    NetworkProblem(reqwest::Error),
    /// Data was received, but could not be decoded
    JsonProblem(reqwest::Error),
    /// Data was decoded, but no items were found
    EmptyResponse,
}

/// Supported coordinate spaces
pub enum CoordinateSpace {
    Rijksdriehoek,
    Gps,
}

impl CoordinateSpace {
    fn as_str(&self) -> &'static str {
        match self {
            CoordinateSpace::Rijksdriehoek => {
                // see https://epsg.io/28992
                "epsg:28992"
            }
            CoordinateSpace::Gps => {
                // see https://epsg.io/4258
                "epsg:4258"
            }
        }
    }
}
