//! The Basisregistratie Kadaster (BRK) service by PDOK.
//! Used to lookup lots.
//!
//! See [the service documentation](https://www.pdok.nl/introductie/-/article/basisregistratie-kadaster-brk-)
//! for more information on its capabilities.
use std::cmp::Ordering;

use crate::Error::{self};
pub use crate::CoordinateSpace;

use geojson::Geometry;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct BrkClient {
    client: Client,
}

impl BrkClient {
    const BRK_BASISREGISTRATIES_OVERHEID_NL: &'static str =
        "https://brk.basisregistraties.overheid.nl";
    const CONN_TIMEOUT_SECS: u64 = 5;
    const REQ_TIMEOUT__SECS: u64 = 20;

    pub fn new(user_agent: &str, accept_crs: CoordinateSpace) -> Self {
        use reqwest::header::{HeaderMap, HeaderValue};

        let mut headers = HeaderMap::new();

        // Gewenste coördinatenstelsel (CRS) van de coördinaten in de response.
        headers.insert("Accept-Crs", HeaderValue::from_static(accept_crs.as_str()));

        headers.insert(
            "transfer-encoding",
            HeaderValue::from_str("chunked").unwrap(),
        );

        let client = reqwest::ClientBuilder::new()
            .user_agent(user_agent)
            .default_headers(headers)
            .connect_timeout(Duration::from_secs(BrkClient::CONN_TIMEOUT_SECS))
            .timeout(Duration::new(BrkClient::REQ_TIMEOUT__SECS, 0))
            .build()
            .unwrap();

        Self { client }
    }

    /// Fetch a singular lot according to its uid,
    /// which is comprised of gemeentecode, sectie and perceelnummer.
    pub async fn get_lot(
        &self,
        gemeentecode: &str,
        sectie: &str,
        perceelnummer: &str,
    ) -> Result<Vec<Lot>, Error> {
        let u = url::Url::parse_with_params(
            &format!(
                "{}/api/v1/percelen",
                BrkClient::BRK_BASISREGISTRATIES_OVERHEID_NL
            ),
            &[
                ("kadastraleGemeentecode", gemeentecode),
                ("sectie", sectie),
                ("perceelnummer", perceelnummer),
            ],
        )
        .unwrap();

        // println!("u: {}", u);

        let res_client_response = self.client.get(u.as_str()).send().await;

        match res_client_response {
            Err(e) => Err(Error::NetworkProblem(e)),
            Ok(client_response) => match client_response.json().await {
                Err(e) => Err(Error::JsonProblem(e)),
                Ok(response) => {
                    let response: Response = response;
                    let lots = response.embedded.results;

                    // println!("lots: {:?}", lots);

                    if lots.is_empty() {
                        Err(Error::EmptyResponse)
                    } else {
                        Ok(lots)
                    }
                }
            },
        }
    }

    ///
    /// Check if API is up by lookup up the TG office
    ///
    pub async fn get_brk_status(&self) -> Result<Vec<Lot>, Error> {
        self.get_lot("HTT02", "M", "5038").await
    }
}

/// A singular lot along with its geometry and size.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Lot {
    pub id: String,
    #[serde(rename = "kadastraleGemeentenaam")]
    pub gemeentenaam: Option<String>,
    #[serde(rename = "kadastraleGemeentecode")]
    pub kadastralegemeentecode: Option<String>,
    #[serde(rename = "kadastraleGrootte")]
    pub grootte: Option<f64>,
    pub sectie: Option<String>,
    pub perceelnummer: Option<u64>,
    pub geometry: Geometry,
}

impl PartialEq for Lot {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Lot {}

impl PartialOrd for Lot {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Lot {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.eq(other) {
            Ordering::Equal
        } else if self.id < other.id {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}

#[derive(Deserialize, Debug)]
struct PerceelEmbedded {
    results: Vec<Lot>,
}

#[derive(Deserialize, Debug)]
struct Response {
    #[serde(rename = "_embedded")]
    embedded: PerceelEmbedded,
}

#[cfg(test)]
mod test {

    use super::*;

    macro_rules! aw {
        ($e:expr) => {
            tokio_test::block_on($e)
        };
    }

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    #[test]
    fn test_get_lot() {
        let ua = format!("pdok-apis brk {}", VERSION);
        let brk_client = BrkClient::new(&ua, CoordinateSpace::Rijksdriehoek);

        let result = aw!(brk_client.get_lot("HTT02", "M", "5038"));

        assert_eq!(result.is_ok(), true);
    }
}
