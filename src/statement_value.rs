use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_json::{json, Value};

use crate::RestApiError;

#[derive(Debug, Clone, PartialEq)]
pub enum StatementValueContent {
    String(String),
    Time {
        time: String,
        precision: u8,
        calendarmodel: String,
    },
    Location {
        latitude: f64,
        longitude: f64,
        precision: f64,
        globe: String,
    },
    Quantity {
        amount: String,
        unit: String,
    },
    MonolingualText {
        language: String,
        text: String,
    },
}

impl StatementValueContent {
    /// Creates a new `StatementValueContent` object from a JSON object.
    pub fn from_json(j: &Value) -> Result<Self, RestApiError> {
        // #lizard forgives the complexity
        if let Some(s) = j.as_str() {
            return Ok(StatementValueContent::String(s.to_string()));
        }
        if let (Some(time), Some(precision), Some(calendarmodel)) = (
            j["time"].as_str(),
            j["precision"].as_u64(),
            j["calendarmodel"].as_str(),
        ) {
            return Ok(StatementValueContent::Time {
                time: time.to_string(),
                precision: precision as u8,
                calendarmodel: calendarmodel.to_string(),
            });
        }
        if let (Some(latitude), Some(longitude), Some(precision), Some(globe)) = (
            j["latitude"].as_f64(),
            j["longitude"].as_f64(),
            j["precision"].as_f64(),
            j["globe"].as_str(),
        ) {
            return Ok(StatementValueContent::Location {
                latitude,
                longitude,
                precision,
                globe: globe.to_string(),
            });
        }
        if let (Some(amount), Some(unit)) = (j["amount"].as_str(), j["unit"].as_str()) {
            return Ok(StatementValueContent::Quantity {
                amount: amount.to_string(),
                unit: unit.to_string(),
            });
        }
        if let (Some(language), Some(text)) = (j["language"].as_str(), j["text"].as_str()) {
            return Ok(StatementValueContent::MonolingualText {
                language: language.to_string(),
                text: text.to_string(),
            });
        }
        Err(RestApiError::UnknownValue(format!("{j:?}")))
    }
}

#[cfg(not(tarpaulin_include))] // tarpaulin can't handle the Serialize trait
impl Serialize for StatementValueContent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self {
            StatementValueContent::String(text) => serialize_text(text, serializer),
            StatementValueContent::Time {
                time,
                precision,
                calendarmodel,
            } => serailize_time(serializer, time, precision, calendarmodel),
            StatementValueContent::Location {
                latitude,
                longitude,
                precision,
                globe,
            } => serialize_location(serializer, latitude, longitude, precision, globe),
            StatementValueContent::Quantity { amount, unit } => {
                serialize_quantity(serializer, amount, unit)
            }
            StatementValueContent::MonolingualText { language, text } => {
                serialize_monolingual_text(serializer, language, text)
            }
        }
    }
}

fn serialize_text<S>(
    text: &String,
    serializer: S,
) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
where
    S: Serializer,
{
    json!(text).serialize(serializer)
}

fn serialize_monolingual_text<S>(
    serializer: S,
    language: &String,
    text: &String,
) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
where
    S: Serializer,
{
    let mut s = serializer.serialize_struct("StatementValueContent", 2)?;
    s.serialize_field("language", language)?;
    s.serialize_field("text", text)?;
    s.end()
}

fn serialize_quantity<S>(
    serializer: S,
    amount: &String,
    unit: &String,
) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
where
    S: Serializer,
{
    let mut s = serializer.serialize_struct("StatementValueContent", 2)?;
    s.serialize_field("amount", amount)?;
    s.serialize_field("unit", unit)?;
    s.end()
}

fn serialize_location<S>(
    serializer: S,
    latitude: &f64,
    longitude: &f64,
    precision: &f64,
    globe: &String,
) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
where
    S: Serializer,
{
    let mut s = serializer.serialize_struct("StatementValueContent", 4)?;
    s.serialize_field("latitude", latitude)?;
    s.serialize_field("longitude", longitude)?;
    s.serialize_field("precision", precision)?;
    s.serialize_field("globe", globe)?;
    s.end()
}

fn serailize_time<S>(
    serializer: S,
    time: &String,
    precision: &u8,
    calendarmodel: &String,
) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
where
    S: Serializer,
{
    let mut s = serializer.serialize_struct("StatementValueContent", 3)?;
    s.serialize_field("time", time)?;
    s.serialize_field("precision", precision)?;
    s.serialize_field("calendarmodel", calendarmodel)?;
    s.end()
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum StatementValue {
    Value(StatementValueContent),
    SomeValue,
    #[default]
    NoValue,
}

impl StatementValue {
    /// Creates a new `StatementValue` object from a JSON object.
    pub fn from_json(j: &Value) -> Result<Self, RestApiError> {
        let value_type = j["type"]
            .as_str()
            .ok_or_else(|| RestApiError::MissingOrInvalidField {
                field: "type".into(),
                j: j.to_owned(),
            })?;
        match value_type {
            "value" => Ok(Self::Value(StatementValueContent::from_json(
                &j["content"],
            )?)),
            "somevalue" => Ok(Self::SomeValue),
            "novalue" => Ok(Self::NoValue),
            _ => Err(RestApiError::UnknownValue(value_type.into())),
        }
    }

    /// Creates a new `StatementValue` object from a string, as a String value.
    pub fn new_string<S: Into<String>>(text: S) -> Self {
        StatementValue::Value(StatementValueContent::String(text.into()))
    }

    // TODO more convenience functions
}

#[cfg(not(tarpaulin_include))] // tarpaulin can't handle the Serialize trait
impl Serialize for StatementValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // #lizard forgives the complexity
        match self {
            StatementValue::Value(content) => {
                let mut s = serializer.serialize_struct("StatementValue", 2)?;
                s.serialize_field("type", "value")?;
                s.serialize_field("content", content)?;
                s.end()
            }
            StatementValue::SomeValue => {
                let mut s = serializer.serialize_struct("StatementValue", 1)?;
                s.serialize_field("type", "somevalue")?;
                s.end()
            }
            StatementValue::NoValue => {
                let mut s = serializer.serialize_struct("StatementValue", 1)?;
                s.serialize_field("type", "novalue")?;
                s.end()
            }
        }
    }
}

/// Implement the From trait for `StatementValueContent` to `StatementValue`, for convenience assignments.
impl From<StatementValueContent> for StatementValue {
    fn from(content: StatementValueContent) -> Self {
        StatementValue::Value(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RestApi;
    use crate::{entity::Entity, EntityId, Item};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_somevalue() {
        let v = std::fs::read_to_string("test_data/Q42.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();
        let id = v["id"].as_str().unwrap();

        let mock_path = format!("/w/rest.php/wikibase/v0/entities/items/{id}");
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(&mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder()
            .api(&(mock_server.uri() + "/w/rest.php"))
            .build()
            .unwrap();

        let item = Item::get(EntityId::item(id), &api).await.unwrap();
        let prop = item.statements().property("P2021")[0].to_owned();
        let qual = &prop.qualifiers()[0];
        assert_eq!(qual.value(), &StatementValue::SomeValue);
    }

    #[tokio::test]
    async fn test_novalue() {
        let v = std::fs::read_to_string("test_data/Q255.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();
        let id = v["id"].as_str().unwrap();

        let mock_path = format!("/w/rest.php/wikibase/v0/entities/items/{id}");
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(&mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder()
            .api(&(mock_server.uri() + "/w/rest.php"))
            .build()
            .unwrap();

        let item = Item::get(EntityId::item(id), &api).await.unwrap();
        let prop = item.statements().property("P40")[0];
        assert_eq!(prop.value(), &StatementValue::NoValue);
    }

    #[test]
    fn test_serialize_string() {
        let s = StatementValue::Value(StatementValueContent::String("foo".to_string()));
        let j: Value = json!(s);
        assert_eq!(j, json!({"type": "value", "content": "foo"}));
    }

    #[test]
    fn test_serialize_time() {
        let s = StatementValue::Value(StatementValueContent::Time {
            time: "+2021-01-01T00:00:00Z".to_string(),
            precision: 11,
            calendarmodel: "http://www.wikidata.org/entity/Q1985727".to_string(),
        });
        let j: Value = json!(s);
        assert_eq!(
            j,
            json!({"type": "value", "content": {"time": "+2021-01-01T00:00:00Z", "precision": 11, "calendarmodel": "http://www.wikidata.org/entity/Q1985727"}})
        );
    }

    #[test]
    fn test_serialize_location() {
        let s = StatementValue::Value(StatementValueContent::Location {
            latitude: 37.786971,
            longitude: -122.399677,
            precision: 0.0001,
            globe: "http://www.wikidata.org/entity/Q2".to_string(),
        });
        let j: Value = json!(s);
        assert_eq!(
            j,
            json!({"type": "value", "content": {"latitude": 37.786971, "longitude": -122.399677, "precision": 0.0001, "globe": "http://www.wikidata.org/entity/Q2"}})
        );
    }

    #[test]
    fn test_serialize_quantity() {
        let s = StatementValue::Value(StatementValueContent::Quantity {
            amount: "42".to_string(),
            unit: "http://www.wikidata.org/entity/Q11573".to_string(),
        });
        let j: Value = json!(s);
        assert_eq!(
            j,
            json!({"type": "value", "content": {"amount": "42", "unit": "http://www.wikidata.org/entity/Q11573"}})
        );
    }

    #[test]
    fn test_serialize_monolingual_text() {
        let s = StatementValue::Value(StatementValueContent::MonolingualText {
            language: "en".to_string(),
            text: "foo".to_string(),
        });
        let j: Value = json!(s);
        assert_eq!(
            j,
            json!({"type": "value", "content": {"language": "en", "text": "foo"}})
        );
    }

    #[test]
    fn test_serialize_somevalue() {
        let s = StatementValue::SomeValue;
        let j: Value = json!(s);
        assert_eq!(j, json!({"type": "somevalue"}));
    }

    #[test]
    fn test_serialize_novalue() {
        let s = StatementValue::NoValue;
        let j: Value = json!(s);
        assert_eq!(j, json!({"type": "novalue"}));
    }

    #[test]
    fn test_from_string() {
        let s = StatementValue::new_string("foo");
        assert_eq!(
            s,
            StatementValue::Value(StatementValueContent::String("foo".to_string()))
        );
    }

    #[test]
    fn test_from_time() {
        let s = StatementValue::Value(StatementValueContent::Time {
            time: "+2021-01-01T00:00:00Z".to_string(),
            precision: 11,
            calendarmodel: "http://www.wikidata.org/entity/Q1985727".to_string(),
        });
        assert_eq!(
            s,
            StatementValue::Value(StatementValueContent::Time {
                time: "+2021-01-01T00:00:00Z".to_string(),
                precision: 11,
                calendarmodel: "http://www.wikidata.org/entity/Q1985727".to_string()
            })
        );
    }

    #[test]
    fn test_from_location() {
        let s = StatementValue::Value(StatementValueContent::Location {
            latitude: 37.786971,
            longitude: -122.399677,
            precision: 0.0001,
            globe: "http://www.wikidata.org/entity/Q2".to_string(),
        });
        assert_eq!(
            s,
            StatementValue::Value(StatementValueContent::Location {
                latitude: 37.786971,
                longitude: -122.399677,
                precision: 0.0001,
                globe: "http://www.wikidata.org/entity/Q2".to_string()
            })
        );
    }

    #[test]
    fn test_from_quantity() {
        let s = StatementValue::Value(StatementValueContent::Quantity {
            amount: "42".to_string(),
            unit: "http://www.wikidata.org/entity/Q11573".to_string(),
        });
        assert_eq!(
            s,
            StatementValue::Value(StatementValueContent::Quantity {
                amount: "42".to_string(),
                unit: "http://www.wikidata.org/entity/Q11573".to_string()
            })
        );
    }

    #[test]
    fn test_from_monolingual_text() {
        let s = StatementValue::Value(StatementValueContent::MonolingualText {
            language: "en".to_string(),
            text: "foo".to_string(),
        });
        assert_eq!(
            s,
            StatementValue::Value(StatementValueContent::MonolingualText {
                language: "en".to_string(),
                text: "foo".to_string()
            })
        );
    }

    #[test]
    fn test_from_somevalue() {
        let s = StatementValue::SomeValue;
        assert_eq!(s, StatementValue::SomeValue);
    }

    #[test]
    fn test_from_novalue() {
        let s = StatementValue::NoValue;
        assert_eq!(s, StatementValue::NoValue);
    }

    #[test]
    fn test_from_json_string() {
        let j = json!("foo");
        let s = StatementValueContent::from_json(&j).unwrap();
        assert_eq!(s, StatementValueContent::String("foo".to_string()));
    }

    #[test]
    fn test_from_json_time() {
        let j = json!({"time": "+2021-01-01T00:00:00Z", "precision": 11, "calendarmodel": "http://www.wikidata.org/entity/Q1985727"});
        let s = StatementValueContent::from_json(&j).unwrap();
        assert_eq!(
            s,
            StatementValueContent::Time {
                time: "+2021-01-01T00:00:00Z".to_string(),
                precision: 11,
                calendarmodel: "http://www.wikidata.org/entity/Q1985727".to_string()
            }
        );
    }

    #[test]
    fn test_from_json_location() {
        let j = json!({"latitude": 37.786971, "longitude": -122.399677, "precision": 0.0001, "globe": "http://www.wikidata.org/entity/Q2"});
        let s = StatementValueContent::from_json(&j).unwrap();
        assert_eq!(
            s,
            StatementValueContent::Location {
                latitude: 37.786971,
                longitude: -122.399677,
                precision: 0.0001,
                globe: "http://www.wikidata.org/entity/Q2".to_string()
            }
        );
    }

    #[test]
    fn test_from_json_quantity() {
        let j = json!({"amount": "42", "unit": "http://www.wikidata.org/entity/Q11573"});
        let s = StatementValueContent::from_json(&j).unwrap();
        assert_eq!(
            s,
            StatementValueContent::Quantity {
                amount: "42".to_string(),
                unit: "http://www.wikidata.org/entity/Q11573".to_string()
            }
        );
    }

    #[test]
    fn test_from_json_monolingual_text() {
        let j = json!({"language": "en", "text": "foo"});
        let s = StatementValueContent::from_json(&j).unwrap();
        assert_eq!(
            s,
            StatementValueContent::MonolingualText {
                language: "en".to_string(),
                text: "foo".to_string()
            }
        );
    }

    #[test]
    fn test_from_json_error() {
        let j = json!({"foo": "bar"});
        let s = StatementValueContent::from_json(&j);
        assert!(s.is_err());
    }

    #[test]
    fn test_statement_value_contents_serialize_string() {
        // #lizard forgives the complexity
        let svc = StatementValueContent::String("foo".to_string());
        let j: Value = serde_json::to_value(&svc).unwrap();
        assert_eq!(j, json!("foo"));
    }

    #[test]
    fn test_statement_value_contents_serialize_time() {
        let svc = StatementValueContent::Time {
            time: "+2021-01-01T00:00:00Z".to_string(),
            precision: 11,
            calendarmodel: "http://www.wikidata.org/entity/Q1985727".to_string(),
        };
        let j: Value = serde_json::to_value(&svc).unwrap();
        assert_eq!(
            j,
            json!({"time": "+2021-01-01T00:00:00Z", "precision": 11, "calendarmodel": "http://www.wikidata.org/entity/Q1985727"})
        );
    }

    #[test]
    fn test_statement_value_contents_serialize_location() {
        let svc = StatementValueContent::Location {
            latitude: 37.786971,
            longitude: -122.399677,
            precision: 0.0001,
            globe: "http://www.wikidata.org/entity/Q2".to_string(),
        };
        let j: Value = serde_json::to_value(&svc).unwrap();
        assert_eq!(
            j,
            json!({"latitude": 37.786971, "longitude": -122.399677, "precision": 0.0001, "globe": "http://www.wikidata.org/entity/Q2"})
        );
    }

    #[test]
    fn test_statement_value_contents_serialize_quantity() {
        let svc = StatementValueContent::Quantity {
            amount: "42".to_string(),
            unit: "http://www.wikidata.org/entity/Q11573".to_string(),
        };
        let j: Value = serde_json::to_value(&svc).unwrap();
        assert_eq!(
            j,
            json!({"amount": "42", "unit": "http://www.wikidata.org/entity/Q11573"})
        );
    }

    #[test]
    fn test_statement_value_contents_serialize_monolingual_text() {
        let svc = StatementValueContent::MonolingualText {
            language: "en".to_string(),
            text: "foo".to_string(),
        };
        let j: Value = serde_json::to_value(&svc).unwrap();
        assert_eq!(j, json!({"language": "en", "text": "foo"}));
    }
}
