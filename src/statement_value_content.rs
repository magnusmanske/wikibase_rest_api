use crate::RestApiError;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_json::{json, Value};

/// Represents the Gregorian calendar model.
pub const GREGORIAN_CALENDAR: &str = "http://www.wikidata.org/entity/Q1985727";

/// Represents the Julian calendar model.
pub const JULIAN_CALENDAR: &str = "http://www.wikidata.org/entity/Q11184";

/// Represents the precision of a time value.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TimePrecision {
    BillionYears = 0,
    HundredMillionYears = 1,
    TenMillionYears = 2,
    MillionYears = 3,
    HundredMillennia = 4,
    TenMillennia = 5,
    Millennia = 6,
    Century = 7,
    Decade = 8,
    Year = 9,
    Month = 10,
    Day = 11,
    Hour = 12,
    Minute = 13,
    Second = 14,
}

impl TryFrom<u8> for TimePrecision {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(TimePrecision::BillionYears),
            1 => Ok(TimePrecision::HundredMillionYears),
            2 => Ok(TimePrecision::TenMillionYears),
            3 => Ok(TimePrecision::MillionYears),
            4 => Ok(TimePrecision::HundredMillennia),
            5 => Ok(TimePrecision::TenMillennia),
            6 => Ok(TimePrecision::Millennia),
            7 => Ok(TimePrecision::Century),
            8 => Ok(TimePrecision::Decade),
            9 => Ok(TimePrecision::Year),
            10 => Ok(TimePrecision::Month),
            11 => Ok(TimePrecision::Day),
            12 => Ok(TimePrecision::Hour),
            13 => Ok(TimePrecision::Minute),
            14 => Ok(TimePrecision::Second),
            _ => Err("Invalid TimePrecision value"),
        }
    }
}

impl From<TimePrecision> for u8 {
    fn from(precision: TimePrecision) -> Self {
        precision as u8
    }
}

impl TryFrom<u64> for TimePrecision {
    type Error = &'static str;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if value > u64::from(u8::MAX) {
            return Err("Value too large for TimePrecision");
        }
        (value as u8).try_into()
    }
}

impl From<TimePrecision> for u64 {
    fn from(precision: TimePrecision) -> Self {
        u64::from(u8::from(precision))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatementValueContent {
    String(String),
    Time {
        time: String,
        precision: TimePrecision,
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
                precision: precision
                    .try_into()
                    .map_err(|_| RestApiError::InvalidPrecision)?,
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

    pub fn new_monolingual_text<S1: Into<String>, S2: Into<String>>(
        language: S1,
        text: S2,
    ) -> Self {
        Self::MonolingualText {
            language: language.into(),
            text: text.into(),
        }
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
    precision: &TimePrecision,
    calendarmodel: &String,
) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
where
    S: Serializer,
{
    let precision = *precision as u8;
    let mut s = serializer.serialize_struct("StatementValueContent", 3)?;
    s.serialize_field("time", time)?;
    s.serialize_field("precision", &precision)?;
    s.serialize_field("calendarmodel", calendarmodel)?;
    s.end()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_u8_conversion() {
        assert_eq!(
            TimePrecision::try_from(0_u8),
            Ok(TimePrecision::BillionYears)
        );
        assert_eq!(
            TimePrecision::try_from(1_u8),
            Ok(TimePrecision::HundredMillionYears)
        );
        assert_eq!(
            TimePrecision::try_from(2_u8),
            Ok(TimePrecision::TenMillionYears)
        );
        assert_eq!(
            TimePrecision::try_from(3_u8),
            Ok(TimePrecision::MillionYears)
        );
        assert_eq!(
            TimePrecision::try_from(4_u8),
            Ok(TimePrecision::HundredMillennia)
        );
        assert_eq!(
            TimePrecision::try_from(5_u8),
            Ok(TimePrecision::TenMillennia)
        );
        assert_eq!(TimePrecision::try_from(6_u8), Ok(TimePrecision::Millennia));
        assert_eq!(TimePrecision::try_from(7_u8), Ok(TimePrecision::Century));
        assert_eq!(TimePrecision::try_from(8_u8), Ok(TimePrecision::Decade));
        assert_eq!(TimePrecision::try_from(9_u8), Ok(TimePrecision::Year));
        assert_eq!(TimePrecision::try_from(10_u8), Ok(TimePrecision::Month));
        assert_eq!(TimePrecision::try_from(11_u8), Ok(TimePrecision::Day));
        assert_eq!(TimePrecision::try_from(12_u8), Ok(TimePrecision::Hour));
        assert_eq!(TimePrecision::try_from(13_u8), Ok(TimePrecision::Minute));
        assert_eq!(TimePrecision::try_from(14_u8), Ok(TimePrecision::Second));
        assert!(TimePrecision::try_from(15_u8).is_err());
    }

    #[test]
    fn test_to_u8_conversion() {
        assert_eq!(u8::from(TimePrecision::BillionYears), 0);
        assert_eq!(u8::from(TimePrecision::HundredMillionYears), 1);
        assert_eq!(u8::from(TimePrecision::TenMillionYears), 2);
        assert_eq!(u8::from(TimePrecision::MillionYears), 3);
        assert_eq!(u8::from(TimePrecision::HundredMillennia), 4);
        assert_eq!(u8::from(TimePrecision::TenMillennia), 5);
        assert_eq!(u8::from(TimePrecision::Millennia), 6);
        assert_eq!(u8::from(TimePrecision::Century), 7);
        assert_eq!(u8::from(TimePrecision::Decade), 8);
        assert_eq!(u8::from(TimePrecision::Year), 9);
        assert_eq!(u8::from(TimePrecision::Month), 10);
        assert_eq!(u8::from(TimePrecision::Day), 11);
        assert_eq!(u8::from(TimePrecision::Hour), 12);
        assert_eq!(u8::from(TimePrecision::Minute), 13);
        assert_eq!(u8::from(TimePrecision::Second), 14);
    }

    #[test]
    fn test_from_u64_conversion() {
        assert_eq!(
            TimePrecision::try_from(0_u64),
            Ok(TimePrecision::BillionYears)
        );
        assert_eq!(
            TimePrecision::try_from(1_u64),
            Ok(TimePrecision::HundredMillionYears)
        );
        assert_eq!(
            TimePrecision::try_from(2_u64),
            Ok(TimePrecision::TenMillionYears)
        );
        assert_eq!(
            TimePrecision::try_from(3_u64),
            Ok(TimePrecision::MillionYears)
        );
        assert_eq!(
            TimePrecision::try_from(4_u64),
            Ok(TimePrecision::HundredMillennia)
        );
        assert_eq!(
            TimePrecision::try_from(5_u64),
            Ok(TimePrecision::TenMillennia)
        );
        assert_eq!(TimePrecision::try_from(6_u64), Ok(TimePrecision::Millennia));
        assert_eq!(TimePrecision::try_from(7_u64), Ok(TimePrecision::Century));
        assert_eq!(TimePrecision::try_from(8_u64), Ok(TimePrecision::Decade));
        assert_eq!(TimePrecision::try_from(9_u64), Ok(TimePrecision::Year));
        assert_eq!(TimePrecision::try_from(10_u64), Ok(TimePrecision::Month));
        assert_eq!(TimePrecision::try_from(11_u64), Ok(TimePrecision::Day));
        assert_eq!(TimePrecision::try_from(12_u64), Ok(TimePrecision::Hour));
        assert_eq!(TimePrecision::try_from(13_u64), Ok(TimePrecision::Minute));
        assert_eq!(TimePrecision::try_from(14_u64), Ok(TimePrecision::Second));
        assert!(TimePrecision::try_from(15_u64).is_err());
    }

    #[test]
    fn test_to_u64_conversion() {
        assert_eq!(u64::from(TimePrecision::BillionYears), 0);
        assert_eq!(u64::from(TimePrecision::HundredMillionYears), 1);
        assert_eq!(u64::from(TimePrecision::TenMillionYears), 2);
        assert_eq!(u64::from(TimePrecision::MillionYears), 3);
        assert_eq!(u64::from(TimePrecision::HundredMillennia), 4);
        assert_eq!(u64::from(TimePrecision::TenMillennia), 5);
        assert_eq!(u64::from(TimePrecision::Millennia), 6);
        assert_eq!(u64::from(TimePrecision::Century), 7);
        assert_eq!(u64::from(TimePrecision::Decade), 8);
        assert_eq!(u64::from(TimePrecision::Year), 9);
        assert_eq!(u64::from(TimePrecision::Month), 10);
        assert_eq!(u64::from(TimePrecision::Day), 11);
        assert_eq!(u64::from(TimePrecision::Hour), 12);
        assert_eq!(u64::from(TimePrecision::Minute), 13);
        assert_eq!(u64::from(TimePrecision::Second), 14);
    }
}
