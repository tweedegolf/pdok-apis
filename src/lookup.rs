//! The geocoding service by PDOK.
//! Used to generate references to the desired lots and buildings.
//!
//! See [the service documentation](https://www.pdok.nl/introductie/-/article/pdok-locatieserver)
//! for more information on its capabilities.
//!
use crate::{
    ClientBuilder,
    Error::{self, *},
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, time::Duration};

pub struct LookupClient {
    client: Client,
}

pub struct LookupClientBuilder<'a> {
    connection_timeout_secs: u64,
    request_timeout_secs: u64,
    user_agent: &'a str,
}

impl<'a> ClientBuilder<'a> for LookupClientBuilder<'a> {
    type OutputType = LookupClient;

    fn connection_timeout_secs(&mut self, connection_timeout_secs: u64) -> &mut Self {
        self.connection_timeout_secs = connection_timeout_secs;
        self
    }

    fn request_timeout_secs(&mut self, request_timeout_secs: u64) -> &mut Self {
        self.request_timeout_secs = request_timeout_secs;
        self
    }

    fn build(&self) -> Self::OutputType {
        let client = reqwest::ClientBuilder::new()
            .user_agent(self.user_agent)
            .connect_timeout(Duration::from_secs(self.connection_timeout_secs))
            .timeout(Duration::new(self.request_timeout_secs, 0))
            .build()
            .unwrap();

        LookupClient { client }
    }
}

impl<'a> LookupClientBuilder<'a> {
    pub fn new(user_agent: &'a str) -> Self {
        Self {
            user_agent,
            connection_timeout_secs: 10,
            request_timeout_secs: 30,
        }
    }
}

impl LookupClient {
    const GEODATA_NATIONAALGEOREGISTER_NL: &'static str = "https://api.pdok.nl/bzk";

    /// Perform a Geocoding lookup based on postal code and housenumber.
    /// Yields a list of possible matches.
    pub async fn suggest_concrete(
        &self,
        postcode: &str,
        huisnummer: &str,
    ) -> Result<Vec<SuggestDoc>, Error> {
        let params = SuggestParams {
            q: format!("postcode:{} {}", postcode, huisnummer),
        };

        let url = format!(
            "{}/locatieserver/search/v3_1/suggest",
            LookupClient::GEODATA_NATIONAALGEOREGISTER_NL
        );

        let client_response = self
            .client
            .get(&url)
            .query(&params)
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
            "{}/locatieserver/search/v3_1/lookup",
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
            "{}/locatieserver/search/v3_1/free",
            LookupClient::GEODATA_NATIONAALGEOREGISTER_NL
        );
        // Example: https://api.pdok.nl/bzk/locatieserver/search/v3_1/free?q=gekoppeld_perceel:HTT02-M-5038
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

impl PartialEq for LookupDoc {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for LookupDoc {}

impl PartialOrd for LookupDoc {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LookupDoc {
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

// See: https://api.pdok.nl/bzk/locatieserver/search/v3_1/ui/#/Locatieserver/suggest
#[derive(Serialize)]
struct SuggestParams {
    q: String,
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
    fn concrete_address() {
        let postalcode = "6542WZ";
        let housenumber = "222";
        let client = LookupClientBuilder::new("pdok-apis lookup").build();

        let suggest_doc = aw!(client.suggest_concrete(postalcode, housenumber));

        let id = suggest_doc.unwrap().first().unwrap().id.clone();

        assert_eq!(id, "adr-2fe93c94378bb179c424cf9918662375");

        let lookup_doc = aw!(client.lookup(&id));
        let street_name = lookup_doc.unwrap().first().unwrap().straatnaam.clone();

        assert_eq!(street_name, "Oude Nonnendaalseweg");
    }

    #[test]
    fn suggest_address_for_lot() {
        let client = LookupClientBuilder::new("pdok-apis lookup").build();

        // TG office plot
        let result = aw!(client.suggest_addresses_for_lot("HTT02", "M", "5038"));

        // Should return Castellastraat 1
        let id = result.unwrap().first().unwrap().id.clone();
        assert_eq!(id, "adr-03b34aeb91028a913c05006049ed3245");
    }

    #[test]
    fn lookup_id() {
        let client = LookupClientBuilder::new("pdok-apis lookup").build();

        // TG office ID
        let result = aw!(client.lookup_tg_office()).unwrap();

        // Check if the address matches
        let street = result.first().unwrap().straatnaam.clone();
        let number = result.first().unwrap().huis_nlt.clone();
        let postcode = result.first().unwrap().postcode.clone();
        assert_eq!(street, "Castellastraat");
        assert_eq!(number, "26");
        assert_eq!(postcode, "6512EX");
    }
}
