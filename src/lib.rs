//! The geocoding service by PDOK.
//! Used to generate references to the desired lots and buildings.
//!
//! See [the service documentation](https://www.pdok.nl/introductie/-/article/pdok-locatieserver)
//! for more information on its capabilities.

pub mod bag;
pub mod brk;
pub mod locatieserver;

#[derive(Debug)]
pub enum Error {
    /// Something went wrong with the request (invalid url, no connection, etc)
    NetworkProblem(reqwest::Error),
    /// Data was received, but could not be decoded
    JsonProblem(reqwest::Error),
}
