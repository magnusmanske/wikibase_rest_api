use crate::RestApiError;

#[derive(Debug, Clone, PartialEq, Default, Copy)]
pub enum DataType {
    #[default]
    String,
    Item,
    Property,
    Url,
    Time,
    GlobeCoordinate,
    Quantity,
    Monolingualtext,
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
}

impl DataType {
    /// Constructs a new `DataType` object from a (valid) string.
    pub fn from_str<S: Into<String>>(s: S) -> Result<Self, RestApiError> {
        match s.into().as_str() {
            "wikibase-item" => Ok(DataType::WikibaseItem),
            "external-id" => Ok(DataType::ExternalId),
            "url" => Ok(DataType::Url),
            "commonsMedia" => Ok(DataType::CommonsMedia),
            "monolingualtext" => Ok(DataType::Monolingualtext),
            "quantity" => Ok(DataType::Quantity),
            "string" => Ok(DataType::String),
            "time" => Ok(DataType::Time),
            "globe-coordinate" => Ok(DataType::GlobeCoordinate),
            "wikibase-property" => Ok(DataType::WikibaseProperty),
            "wikibase-lexeme" => Ok(DataType::Lexeme),
            "wikibase-form" => Ok(DataType::Form),
            "wikibase-sense" => Ok(DataType::Sense),
            "geo-shape" => Ok(DataType::GeoShape),
            "tabular-data" => Ok(DataType::TabularData),
            "math" => Ok(DataType::Math),
            "item" => Ok(DataType::Item),
            "property" => Ok(DataType::Property),
            "musical-notation" => Ok(DataType::MusicalNotation),
            "entity-schema" => Ok(DataType::EntitySchema),
            other => Err(RestApiError::UnknownDataType(other.into())),
        }
    }

    /// Returns the string representation of the data type.
    pub fn as_str(&self) -> &str {
        match self {
            DataType::WikibaseItem => "wikibase-item",
            DataType::ExternalId => "external-id",
            DataType::Url => "url",
            DataType::CommonsMedia => "commonsMedia",
            DataType::Monolingualtext => "monolingualtext",
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
            DataType::Item => "item",
            DataType::Property => "property",
            DataType::MusicalNotation => "musical-notation",
            DataType::EntitySchema => "entity-schema",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use crate::RestApi;
    use super::*;

    #[tokio::test]
    async fn test_data_type_from_str() {
        // Useful to have this query the live API, and fast enough.
        let api = RestApi::builder().api("https://www.wikidata.org/w/rest.php").build().unwrap();
        let request = api.wikibase_request_builder("/property-data-types", HashMap::new(), reqwest::Method::GET).await.unwrap().build().unwrap();
        let h: HashMap<String,String> = api.execute(request).await.unwrap().error_for_status().unwrap().json().await.unwrap();
        for (k,_v) in h {
            let dt = DataType::from_str(&k).unwrap();
            assert_eq!(dt.as_str(), k);
        }
        assert!(DataType::from_str("not-a-data-type").is_err());
    }

    #[test]
    fn test_as_str() {
        assert_eq!(DataType::WikibaseItem.as_str(), "wikibase-item");
        assert_eq!(DataType::ExternalId.as_str(), "external-id");
        assert_eq!(DataType::Url.as_str(), "url");
        assert_eq!(DataType::CommonsMedia.as_str(), "commonsMedia");
        assert_eq!(DataType::Monolingualtext.as_str(), "monolingualtext");
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
        assert_eq!(DataType::Item.as_str(), "item");
        assert_eq!(DataType::Property.as_str(), "property");
    }
}