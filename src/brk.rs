//! The Basisregistratie Kadaster (BRK) service by PDOK.
//! Used to lookup lots.
//!
//! See [the service documentation](https://www.pdok.nl/introductie/-/article/basisregistratie-kadaster-brk-)
//! for more information on its capabilities.
use std::cmp::Ordering;

pub use crate::CoordinateSpace;
use crate::Error;

use geojson::{FeatureCollection, Geometry};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct BrkClient {
    client: Client,
}

pub struct BrkClientBuilder<'a> {
    accept_crs: CoordinateSpace,
    connection_timeout_secs: u64,
    request_timeout_secs: u64,
    user_agent: &'a str,
}

impl<'a> BrkClientBuilder<'a> {
    pub fn new(user_agent: &'a str) -> Self {
        Self {
            user_agent,
            accept_crs: CoordinateSpace::Gps,
            connection_timeout_secs: 5,
            request_timeout_secs: 20,
        }
    }

    pub fn accept_crs(&mut self, accept_crs: CoordinateSpace) -> &mut Self {
        self.accept_crs = accept_crs;
        self
    }
}

impl<'a> crate::ClientBuilder<'a> for BrkClientBuilder<'a> {
    type OutputType = BrkClient;

    fn connection_timeout_secs(&mut self, connection_timeout_secs: u64) -> &mut Self {
        self.connection_timeout_secs = connection_timeout_secs;
        self
    }

    fn request_timeout_secs(&mut self, request_timeout_secs: u64) -> &mut Self {
        self.request_timeout_secs = request_timeout_secs;
        self
    }

    fn build(&self) -> BrkClient {
        use reqwest::header::{HeaderMap, HeaderValue};

        let mut headers = HeaderMap::new();

        // Gewenste coördinatenstelsel (CRS) van de coördinaten in de response.
        headers.insert(
            "Accept-Crs",
            HeaderValue::from_static(self.accept_crs.as_str()),
        );

        headers.insert(
            "transfer-encoding",
            HeaderValue::from_str("chunked").unwrap(),
        );

        let client = reqwest::ClientBuilder::new()
            .user_agent(self.user_agent)
            .default_headers(headers)
            .connect_timeout(Duration::from_secs(self.connection_timeout_secs))
            .timeout(Duration::new(self.request_timeout_secs, 0))
            .build()
            .unwrap();

        BrkClient { client }
    }
}

impl BrkClient {
    const BRK_URL: &'static str = "https://service.pdok.nl/kadaster/kadastralekaart/wfs/v5_0";

    /// Fetch a singular lot according to its uid,
    /// which is comprised of gemeentecode, sectie and perceelnummer.
    pub async fn get_lot(
        &self,
        gemeentecode: &str,
        sectie: &str,
        perceelnummer: &str,
    ) -> Result<Vec<Lot>, Error> {
        let filter = format!("<Filter><And><And><PropertyIsEqualTo><PropertyName>sectie</PropertyName><Literal>{sectie}</Literal></PropertyIsEqualTo><PropertyIsEqualTo><PropertyName>perceelnummer</PropertyName><Literal>{perceelnummer}</Literal></PropertyIsEqualTo></And><PropertyIsEqualTo><PropertyName>AKRKadastraleGemeenteCodeWaarde</PropertyName><Literal>{gemeentecode}</Literal></PropertyIsEqualTo></And></Filter>");

        let u = url::Url::parse_with_params(
            &format!("{}", BrkClient::BRK_URL),
            &[
                ("request", "GetFeature"),
                ("service", "WFS"),
                ("version", "2.0.0"),
                ("typenames", "kadastralekaartv5:perceel"),
                ("outputFormat", "application/json"),
                ("filter", &filter),
            ],
        )
        .unwrap();
        let res_client_response = self.client.get(u.as_str()).send().await;
        match res_client_response {
            Err(e) => Err(Error::NetworkProblem(e)),
            Ok(client_response) => match client_response.json().await {
                Err(e) => Err(Error::JsonProblem(e)),
                Ok(response) => {
                    let response: FeatureCollection = response;
                    let lots: Vec<Lot> = response
                        .features
                        .iter()
                        .map(|feature| Lot {
                            id: feature
                                .property("identificatieLokaalID")
                                .unwrap()
                                .to_string(),
                            gemeentenaam: Some(
                                feature
                                    .property("kadastraleGemeenteWaarde")
                                    .unwrap()
                                    .to_string(),
                            ),
                            kadastralegemeentecode: Some(
                                feature
                                    .property("kadastraleGemeenteCode")
                                    .unwrap()
                                    .to_string(),
                            ),
                            grootte: feature
                                .property("kadastraleGrootteWaarde")
                                .unwrap()
                                .as_f64(),
                            sectie: Some(feature.property("sectie").unwrap().to_string()),
                            perceelnummer: Some(
                                feature.property("perceelnummer").unwrap().as_u64().unwrap(),
                            ),
                            geometry: feature.geometry.clone().unwrap(),
                        })
                        .collect();
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
    /// Check if API is up by looking up the TG office
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
    use crate::ClientBuilder;

    macro_rules! aw {
        ($e:expr) => {
            tokio_test::block_on($e)
        };
    }

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    #[test]
    fn test_get_lot() {
        let ua = format!("pdok-apis brk {}", VERSION);
        let brk_client = BrkClientBuilder::new(&ua)
            .accept_crs(CoordinateSpace::Rijksdriehoek)
            .build();

        let result = aw!(brk_client.get_lot("HTT02", "M", "5038"));
        assert_eq!(result.is_ok(), true);
    }
}
