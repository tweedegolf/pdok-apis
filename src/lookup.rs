//! The geocoding service by PDOK.
//! Used to generate references to the desired lots and buildings.
//!
//! See [the service documentation](https://www.pdok.nl/introductie/-/article/pdok-locatieserver)
//! for more information on its capabilities.
//! 
use crate::Error::{self, *};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct LookupClient {
    client: Client,
}

impl LookupClient {
    const GEODATA_NATIONAALGEOREGISTER_NL: &'static str = "https://geodata.nationaalgeoregister.nl";
    const CONN_TIMEOUT_SECS: u64 = 10;
    const REQ_TIMEOUT__SECS: u64 = 30;

    pub fn new(user_agent: &str) -> Self {
        let client = reqwest::ClientBuilder::new()
            .user_agent(user_agent)
            .connect_timeout(Duration::from_secs(LookupClient::CONN_TIMEOUT_SECS))
            .timeout(Duration::new(LookupClient::REQ_TIMEOUT__SECS, 0))
            .build()
            .unwrap();

        Self { client }
    }

    /// Perform a Geocoding lookup based on postal code and housenumber.
    /// Yields a list of possible matches.
    pub async fn suggest_concrete(
        &self,
        postcode: &str,
        huisnummer: &str,
    ) -> Result<Vec<SuggestDoc>, Error> {
        let params = SuggestParams {
            query: format!("postcode:{} {}", postcode, huisnummer),
        };

        let url = format!(
            "{}/locatieserver/v3/suggest",
            LookupClient::GEODATA_NATIONAALGEOREGISTER_NL
        );

        let client_response = self
            .client
            .post(&url)
            .json(&params)
            .send()
            .await
            .map_err(NetworkProblem)?;

        let response: SuggestResponse = client_response.json().await.map_err(JsonProblem)?;

        Ok(response.response.docs)
    }

    /// Lookup a specific location id.
    ///
    /// Returns a 1:1 representation of the SolrReponse.
    pub async fn lookup(&self, id: &str) -> Result<Vec<LookupDoc>, Error> {
        let url = format!(
            "{}/locatieserver/v3/lookup",
            LookupClient::GEODATA_NATIONAALGEOREGISTER_NL
        );

        let u = url::Url::parse_with_params(&url, &[("id", id)]).unwrap();

        let client_response = self
            .client
            .get(u.as_str())
            .send()
            .await
            .map_err(NetworkProblem)?;

        let response: LookupResponse = client_response.json().await.map_err(JsonProblem)?;

        Ok(response.response.docs)
    }

    /// Get suggestions on addresses related to a lot
    /// Yields a list of possible matches.
    pub async fn suggest_addresses_for_lot(
        &self,
        lot_code: &str,
        lot_letter: &str,
        lot_number: &str,
    ) -> Result<Vec<SuggestDoc>, Error> {
        let query = format!(
            "gekoppeld_perceel:{}-{}-{}",
            lot_code, lot_letter, lot_number
        );

        let url = format!(
            "{}/locatieserver/v3/free",
            LookupClient::GEODATA_NATIONAALGEOREGISTER_NL
        );

        // Example: https://geodata.nationaalgeoregister.nl/locatieserver/v3/free?q=gekoppeld_perceel:HTT02-M-5763
        let u =
            url::Url::parse_with_params(&url, &[("q", query), ("fq", "type:adres".to_string())])
                .unwrap();

        let client_response = self
            .client
            .get(u.as_str())
            .send()
            .await
            .map_err(NetworkProblem)?;

        let response: SuggestResponse = client_response.json().await.map_err(JsonProblem)?;

        Ok(response.response.docs)
    }

    /// Check if the API is up by looking up our office
    pub async fn lookup_tg_office(&self) -> Result<Vec<LookupDoc>, Error> {
        self.lookup("adr-5826c02550308f6da19e4feb5eb97ec8").await
    }
}

/// A specific location that was looked up.
/// Contains references to the lot, building and address.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LookupDoc {
    pub id: String,
    pub gekoppeld_perceel: Vec<String>,
    pub nummeraanduiding_id: String,
    pub adresseerbaarobject_id: String,
    pub postcode: String,
    pub huis_nlt: String,
    pub straatnaam: String,
    pub woonplaatsnaam: String,
}

#[derive(Serialize)]
struct SuggestParams {
    query: String,
}

/// One element of the set of suggestions as done by the geocoding service.
///
/// Probably only the best result is relevant for our search.
#[derive(Serialize, Deserialize, Debug)]
pub struct SuggestDoc {
    pub id: String,
    #[serde(rename = "type")]
    pub result_type: String,
    pub weergavenaam: String,
    pub score: f64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SolrResponse<T> {
    docs: Vec<T>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SuggestResponse {
    response: SolrResponse<SuggestDoc>,
}

#[derive(Deserialize, Debug)]
struct LookupResponse {
    response: SolrResponse<LookupDoc>,
}

#[cfg(test)]
mod test {

    use super::*;

    macro_rules! aw {
        ($e:expr) => {
            tokio_test::block_on($e)
        };
    }

    #[test]
    fn test_concrete_address() {
        let postalcode = "6542WZ";
        let housenumber = "222";
        let client = LookupClient::new("Your user agent");

        let suggest_doc = aw!(client.suggest_concrete(postalcode, housenumber));
        let id = suggest_doc.unwrap().first().unwrap().id.clone();

        assert_eq!(id, "adr-2fe93c94378bb179c424cf9918662375");

        let lookup_doc = aw!(client.lookup(&id));
        let street_name = lookup_doc.unwrap().first().unwrap().straatnaam.clone();

        assert_eq!(street_name, "Oude Nonnendaalseweg");
    }
}
