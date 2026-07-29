use chrono::{Datelike, Timelike};
use postcard_schema::{
    Schema,
    schema::{DataModelType, NamedType},
};
use serde::{Deserialize, Serialize};

/// Wrappers for Date and time that implement postcard-schema

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WireDate(pub time::Date);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WireTime(pub time::Time);

impl Schema for WireDate {
    const SCHEMA: &'static NamedType = &NamedType {
        name: "WireDate",
        ty: &DataModelType::Tuple(&[
            i32::SCHEMA, // year
            u16::SCHEMA, // ordinal
        ]),
    };
}

impl Schema for WireTime {
    const SCHEMA: &'static NamedType = &NamedType {
        name: "WireTime",
        ty: &DataModelType::Tuple(&[
            u8::SCHEMA,  // hour
            u8::SCHEMA,  // minute
            u8::SCHEMA,  // second
            u32::SCHEMA, // nanosecond
        ]),
    };
}

impl From<chrono::NaiveDate> for WireDate {
    fn from(d: chrono::NaiveDate) -> Self {
        // SAFETY: These unwrap()s are fine since NaiveDate hold valid values
        Self(
            time::Date::from_calendar_date(
                d.year(),
                time::Month::try_from(d.month() as u8).unwrap(),
                d.day().try_into().unwrap(),
            )
            .unwrap(),
        )
    }
}

impl From<chrono::NaiveTime> for WireTime {
    fn from(t: chrono::NaiveTime) -> Self {
        // SAFETY: These unwrap()s are fine since NaiveDate hold valid values
        Self(
            time::Time::from_hms_nano(
                t.hour().try_into().unwrap(),
                t.minute().try_into().unwrap(),
                t.second().try_into().unwrap(),
                t.nanosecond() % 1_000_000_000, // NaiveTime has leap second handling, time::Time doesn't
            )
            .unwrap(),
        )
    }
}
