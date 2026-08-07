use postcard_schema::{
    Schema,
    schema::{DataModelType, NamedType, NamedValue},
};
use serde::{Deserialize, Serialize};

/// Wrappers for Date and time that implement postcard-schema

#[cfg_attr(feature = "no-std", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WireDate(pub chrono::NaiveDate);

#[cfg_attr(feature = "no-std", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WireTime(pub chrono::NaiveTime);

impl Schema for WireDate {
    const SCHEMA: &'static NamedType = &NamedType {
        name: "WireDate",
        ty: &DataModelType::TupleStruct(&[&NamedType {
            name: "NaiveDate",
            ty: &DataModelType::Struct(&[&NamedValue {
                name: "yof",
                ty: u32::SCHEMA,
            }]),
        }]),
    };
}

impl Schema for WireTime {
    const SCHEMA: &'static NamedType = &NamedType {
        name: "WireTime",
        ty: &DataModelType::TupleStruct(&[&NamedType {
            name: "NaiveTime",
            ty: &DataModelType::Struct(&[
                &NamedValue {
                    name: "secs",
                    ty: u32::SCHEMA,
                },
                &NamedValue {
                    name: "frac",
                    ty: u32::SCHEMA,
                },
            ]),
        }]),
    };
}

impl From<chrono::NaiveDate> for WireDate {
    fn from(d: chrono::NaiveDate) -> Self {
        Self(d)
    }
}

impl From<chrono::NaiveTime> for WireTime {
    fn from(t: chrono::NaiveTime) -> Self {
        Self(t)
    }
}
