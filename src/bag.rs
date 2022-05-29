use std::{cmp::Ordering, time::Duration};

use crate::Error::{self, *};

use reqwest::Client;
use serde::{Deserialize, Serialize};

use geo::Polygon;
use geojson::Geometry;

pub struct BagClient {
    client: Client,
}

impl BagClient {
    const BAG_URL: &'static str = "https://api.bag.kadaster.nl/lvbag/individuelebevragingen/v2";
    const CONN_TIMEOUT_SECS: u64 = 5;
    const REQ_TIMEOUT__SECS: u64 = 20;

    pub fn new(user_agent: &str, api_key: &str) -> Self {
        use reqwest::header::{HeaderMap, HeaderValue};

        let mut headers = HeaderMap::new();

        headers.insert("X-Api-Key", HeaderValue::from_str(api_key).unwrap());

        // Gewenste coördinatenstelsel (CRS) van de coördinaten in de response.
        headers.insert(
            "Accept-Crs",
            HeaderValue::from_static(CoordinateSpace::Rijksdriehoek.as_str()),
        );

        headers.insert(
            "transfer-encoding",
            HeaderValue::from_str("chunked").unwrap(),
        );

        let client = reqwest::ClientBuilder::new()
            .user_agent(user_agent)
            .default_headers(headers)
            .connect_timeout(Duration::from_secs(BagClient::CONN_TIMEOUT_SECS))
            .timeout(Duration::new(BagClient::REQ_TIMEOUT__SECS, 0))
            .build()
            .unwrap();

        Self { client }
    }

    ///
    /// Fetch embedded links from a BAG call
    ///
    async fn get_link(&self, url: &str) -> Result<Building, Error> {
        let client_response = self.client.get(url).send().await.map_err(NetworkProblem)?;
        let response: Building = client_response.json().await.map_err(JsonProblem)?;

        Ok(response)
    }

    ///
    /// Fetch all ids for panden, associated with the given addresseerbaarobject
    ///
    pub async fn get_panden(&self, object_id: &str) -> Result<Vec<Pand>, Error> {
        let url = format!("{}/verblijfsobjecten/{}", BagClient::BAG_URL, object_id);

        let client_response = self
            .client
            .get(url.as_str())
            .header("Accept-Crs", "epsg:28992".to_string())
            .send()
            .await;

        match client_response {
            Ok(response) => Ok(self.decode_verblijfsobjecten(response).await?),
            Err(_) => Ok(vec![]),
        }
    }

    ///
    /// Get bag status by fetch info about a random pand.
    ///
    pub async fn get_bag_status(&self) -> Result<bool, Error> {
        let tg_office_verblijfsobject = "0268010000084126";
        let panden = self.get_panden(tg_office_verblijfsobject).await?;

        match panden.len() {
            1 => Ok(true),
            _ => panic!(),
        }
    }

    async fn decode_verblijfsobjecten(
        &self,
        response: reqwest::Response,
    ) -> Result<Vec<Pand>, Error> {
        #[derive(Deserialize, Debug, Clone)]
        struct VerblijfsObjectResponse {
            verblijfsobject: VerblijfsObject,
            #[serde(rename = "_links")]
            links: Links,
        }

        #[derive(Deserialize, Debug, Clone)]
        struct Links {
            #[serde(rename = "maaktDeelUitVan")]
            maakt_deel_uit_van: Vec<Link>,
        }

        #[derive(Deserialize, Debug, Clone)]
        struct Link {
            href: String,
        }

        #[derive(Deserialize, Debug, Clone)]
        struct VerblijfsObject {
            #[serde(default)]
            status: String,
            #[serde(default)]
            oppervlakte: i64,
            gebruiksdoelen: Vec<String>,
        }

        let decoded = response
            .json::<VerblijfsObjectResponse>()
            .await
            .map_err(JsonProblem)?;

        let VerblijfsObjectResponse {
            verblijfsobject,
            links,
        } = decoded;

        let objectstatus = verblijfsobject.status;
        let vloeroppervlak = verblijfsobject.oppervlakte;
        let gebruiksdoelen = verblijfsobject.gebruiksdoelen;

        let gebruiksdoel = gebruiksdoelen.join(", ");

        let panden = links.maakt_deel_uit_van;

        let mut results = Vec::with_capacity(panden.len());

        use geo::algorithm::area::Area;
        for pand in panden {
            let building = self.get_link(&pand.href).await?;
            let geometry_json_value = &building.pand.geometry.value;
            let polygon: Polygon<f64> = geojson_value_to_polygon(geometry_json_value).unwrap();

            let pand = Pand {
                identificatiecode: building.pand.identificatie,
                geometry: building.pand.geometry,
                pandvlak: Area::unsigned_area(&polygon).round().to_string(),
                vloeroppervlak: vloeroppervlak.to_string(),
                bouwjaar: building.pand.bouwjaar.to_string(),
                pandstatus: building.pand.pandstatus,
                objectstatus: objectstatus.clone(),
                gebruiksdoel: gebruiksdoel.clone(),
            };

            results.push(pand);
        }

        Ok(results)
    }
}

/// Coordinate space that the BAG returns
/// (currently only Rijksdriehoek is supported)
enum CoordinateSpace {
    Rijksdriehoek,
}

impl CoordinateSpace {
    fn as_str(&self) -> &'static str {
        match self {
            CoordinateSpace::Rijksdriehoek => {
                // see https://epsg.io/28992
                "epsg:28992"
            }
        }
    }
}

#[derive(Serialize)]
pub struct BagRequest {
    query: String,
}

#[derive(Serialize)]
pub struct BagResponse {
    data: String,
}

#[derive(Serialize, Deserialize)]
pub struct Building {
    #[serde(rename = "pand")]
    pand: BuildingEmbedded,
}

#[derive(Serialize, Deserialize)]
pub struct BuildingEmbedded {
    identificatie: String,
    #[serde(rename = "geometrie")]
    geometry: Geometry,
    #[serde(rename = "oorspronkelijkBouwjaar")]
    bouwjaar: String,
    #[serde(rename = "status")]
    pandstatus: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Pand {
    pub identificatiecode: String,
    pub pandvlak: String,
    pub vloeroppervlak: String,
    pub bouwjaar: String,
    pub pandstatus: String,
    pub objectstatus: String,
    pub gebruiksdoel: String,
    pub geometry: Geometry,
}

impl PartialEq for Pand {
    fn eq(&self, other: &Self) -> bool {
        self.identificatiecode == other.identificatiecode
    }
}

impl Eq for Pand {}

impl PartialOrd for Pand {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Pand {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.eq(other) {
            Ordering::Equal
        } else if self.identificatiecode < other.identificatiecode {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}

fn linestring_help(value: &[geojson::Position]) -> geo::LineString<f64> {
    let mut points = Vec::with_capacity(value.len());

    for position in value {
        match position[..] {
            [x, y] | [x, y, _] => {
                points.push((x, y));
            }
            _ => panic!("invalid positions for a polygon"),
        }
    }

    geo::LineString::from(points)
}

fn geojson_value_to_polygon(value: &geojson::Value) -> Option<Polygon<f64>> {
    use geojson::Value::*;

    match value {
        Polygon(rings) => match rings.split_first() {
            None => unreachable!(),
            Some((outer_positions, inner_positions)) => {
                let outer = linestring_help(outer_positions);

                let inners: Vec<_> = inner_positions.iter().map(|x| linestring_help(x)).collect();

                Some(geo::Polygon::new(outer, inners))
            }
        },
        _ => None,
    }
}

#[cfg(test)]
mod test {

    use super::*;

    macro_rules! aw {
        ($e:expr) => {
            tokio_test::block_on($e)
        };
    }

    pub fn get_bag_key() -> String {
        std::env::var("BAG_API_KEY").expect("Environment variable missing: BAG_API_KEY")
    }

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    #[test]
    fn test_get_building_year() {
        let ua = format!("PECT lot render service {}", VERSION);
        let bag_client = BagClient::new(&ua, &get_bag_key());

        let object_id = "0268010000084126";
        let buildings = aw!(bag_client.get_panden(object_id));
        let year = buildings.unwrap().first().unwrap().bouwjaar.clone();

        assert_eq!(year, String::from("2008"));
    }
}
