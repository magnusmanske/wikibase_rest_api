use serde::{Serialize, Serializer};

/// A Wikibase property data type, e.g. `wikibase-item` or `external-id`.
///
/// Data types this crate doesn't know about (e.g. ones added by a Wikibase extension)
/// are kept verbatim as [`DataType::Other`], so they survive a read/serialize round trip.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum DataType {
    #[default]
    String,
    Url,
    Time,
    GlobeCoordinate,
    Quantity,
    MonolingualText,
    CommonsMedia,
    GeoShape,
    TabularData,
    Math,
    MusicalNotation,
    ExternalId,
    WikibaseItem,
    WikibaseProperty,
    Lexeme,
    Form,
    Sense,
    EntitySchema,
    /// A data type not known to this crate, with its raw name.
    Other(String),
}

impl DataType {
    /// Constructs a `DataType` from its Wikibase name. Unknown names become [`DataType::Other`].
    pub fn new<S: Into<String>>(s: S) -> Self {
        let s = s.into();
        match s.as_str() {
            "wikibase-item" => DataType::WikibaseItem,
            "external-id" => DataType::ExternalId,
            "url" => DataType::Url,
            "commonsMedia" => DataType::CommonsMedia,
            "monolingualtext" => DataType::MonolingualText,
            "quantity" => DataType::Quantity,
            "string" => DataType::String,
            "time" => DataType::Time,
            "globe-coordinate" => DataType::GlobeCoordinate,
            "wikibase-property" => DataType::WikibaseProperty,
            "wikibase-lexeme" => DataType::Lexeme,
            "wikibase-form" => DataType::Form,
            "wikibase-sense" => DataType::Sense,
            "geo-shape" => DataType::GeoShape,
            "tabular-data" => DataType::TabularData,
            "math" => DataType::Math,
            "musical-notation" => DataType::MusicalNotation,
            "entity-schema" => DataType::EntitySchema,
            _ => DataType::Other(s),
        }
    }

    /// Returns the string representation of the data type.
    pub fn as_str(&self) -> &str {
        match self {
            DataType::WikibaseItem => "wikibase-item",
            DataType::ExternalId => "external-id",
            DataType::Url => "url",
            DataType::CommonsMedia => "commonsMedia",
            DataType::MonolingualText => "monolingualtext",
            DataType::Quantity => "quantity",
            DataType::String => "string",
            DataType::Time => "time",
            DataType::GlobeCoordinate => "globe-coordinate",
            DataType::WikibaseProperty => "wikibase-property",
            DataType::Lexeme => "wikibase-lexeme",
            DataType::Form => "wikibase-form",
            DataType::Sense => "wikibase-sense",
            DataType::GeoShape => "geo-shape",
            DataType::TabularData => "tabular-data",
            DataType::Math => "math",
            DataType::MusicalNotation => "musical-notation",
            DataType::EntitySchema => "entity-schema",
            DataType::Other(s) => s,
        }
    }
}

impl Serialize for DataType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RestApi;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_data_type_from_str() {
        let body = serde_json::json!({
            "wikibase-item": "wikibase-entityid",
            "external-id": "string",
            "time": "time",
        });
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v1/property-data-types"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();

        let types = api.get_property_data_types().await.unwrap();
        for k in types.into_keys() {
            let dt = DataType::new(&k);
            assert!(!matches!(dt, DataType::Other(_)), "{k} should be known");
            assert_eq!(dt.as_str(), k);
        }
    }

    #[test]
    fn test_as_str() {
        assert_eq!(DataType::WikibaseItem.as_str(), "wikibase-item");
        assert_eq!(DataType::ExternalId.as_str(), "external-id");
        assert_eq!(DataType::Url.as_str(), "url");
        assert_eq!(DataType::CommonsMedia.as_str(), "commonsMedia");
        assert_eq!(DataType::MonolingualText.as_str(), "monolingualtext");
        assert_eq!(DataType::Quantity.as_str(), "quantity");
        assert_eq!(DataType::String.as_str(), "string");
        assert_eq!(DataType::Time.as_str(), "time");
        assert_eq!(DataType::GlobeCoordinate.as_str(), "globe-coordinate");
        assert_eq!(DataType::WikibaseProperty.as_str(), "wikibase-property");
        assert_eq!(DataType::Lexeme.as_str(), "wikibase-lexeme");
        assert_eq!(DataType::Form.as_str(), "wikibase-form");
        assert_eq!(DataType::Sense.as_str(), "wikibase-sense");
        assert_eq!(DataType::GeoShape.as_str(), "geo-shape");
        assert_eq!(DataType::TabularData.as_str(), "tabular-data");
        assert_eq!(DataType::Math.as_str(), "math");
        assert_eq!(DataType::MusicalNotation.as_str(), "musical-notation");
        assert_eq!(DataType::EntitySchema.as_str(), "entity-schema");
    }

    #[test]
    fn test_unknown_data_type_round_trips() {
        let dt = DataType::new("edtf");
        assert_eq!(dt, DataType::Other("edtf".to_string()));
        assert_eq!(dt.as_str(), "edtf");
        assert_eq!(serde_json::to_value(&dt).unwrap(), "edtf");
        assert_eq!(
            serde_json::to_value(DataType::WikibaseItem).unwrap(),
            "wikibase-item"
        );
    }
}
