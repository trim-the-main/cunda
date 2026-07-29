#![cfg_attr(feature = "embassy-time", no_std)]

pub mod datetime_wrappers;
pub mod display;
mod helpers;
pub mod position;
pub mod pvt;
pub mod velocity;

pub use nmea::Error as NmeaParserError;
// #[cfg(test)]
// mod tests {
// use super::*;
//
// #[test]
// fn gga_updates_position_and_altitude() {
// let mut fix = GpsFix::default();
// let f = [
// "092750.000",
// "4717.11399",
// "N",
// "00833.91590",
// "E",
// "1",
// "08",
// "1.01",
// "499.6",
// "M",
// "48.0",
// "M",
// "",
// "",
// ];
// fix.update_from_sentence("GNGGA", &f);
// assert_eq!(fix.fix_quality, FixQuality::GpsFix);
// assert_eq!(fix.num_satellites_used, Some(8));
// assert!((fix.latitude_deg.unwrap() - 47.28523).abs() < 1e-4);
// assert!((fix.longitude_deg.unwrap() - 8.56526).abs() < 1e-4);
// assert_eq!(fix.height_msl_mm, Some(499.6));
// }
//
// #[test]
// fn gsa_updates_fix_type_and_satellites() {
// let mut fix = GpsFix::default();
// let f = [
// "A", "3", "23", "19", "18", "22", "", "", "", "", "", "", "", "1.94", "1.01", "1.65",
// ];
// fix.update_from_sentence("GNGSA", &f);
// assert_eq!(fix.fix_type, FixType::Fix3D);
// assert_eq!(fix.satellites_used, vec![23, 19, 18, 22]);
// assert_eq!(fix.pdop, Some(1.94));
// }
// }
//
