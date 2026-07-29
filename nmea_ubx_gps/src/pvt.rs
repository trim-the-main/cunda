#[cfg(feature = "embassy-time")]
use embassy_time::Instant;
#[cfg(not(feature = "embassy-time"))]
use std::time::Instant;

use core::num::NonZeroU64;
use serde::{Deserialize, Serialize};

use postcard_schema::Schema;

use crate::{
    datetime_wrappers::{WireDate, WireTime},
    position::GpsPosition,
    velocity::GpsVelocity,
};

use nmea::{ParseResult, SentenceType};

#[derive(Default, Debug, Clone, PartialEq)]
pub struct GpsData {
    /// in nanoseconds since the boot
    updated_at: Option<NonZeroU64>,

    pub fix_type: FixType,
    pub utc_date: Option<WireDate>,
    pub utc_time: Option<WireTime>,

    pub pos: Option<GpsPosition>,
    pub vel: Option<GpsVelocity>,

    pub mag: Option<MagneticDeclination>,
    pub num_satellites_used: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MagneticDeclination {
    /// magnetic declination 1e-2 degrees
    pub decl: i16,

    /// magnetic declination accuracy, 1e02 degrees
    pub acc: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Schema)]
pub enum FixType {
    #[default]
    NoFix,
    DeadReckoningOnly,
    Fix2D,
    Fix3D,
    GnssPlusDeadReckoning,
    TimeOnlyFix,
}

impl FixType {
    #[allow(dead_code)]
    fn from_ubx(code: u8) -> Self {
        match code {
            1 => FixType::DeadReckoningOnly,
            2 => FixType::Fix2D,
            3 => FixType::Fix3D,
            4 => FixType::GnssPlusDeadReckoning,
            5 => FixType::TimeOnlyFix,
            _ => FixType::NoFix,
        }
    }
}

impl<'a> GpsData {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn updated_at(&self) -> Option<Instant> {
        self.updated_at
            .map(|nanos| Instant::from_nanos(nanos.into()))
    }

    pub fn update_with_sentence(
        &mut self,
        sentence: &'a [u8],
    ) -> Result<SentenceType, nmea::Error<'a>> {
        match nmea::parse_bytes(sentence)? {
            ParseResult::GGA(gga) => {
                self.merge_gga_data(gga);
                Ok(SentenceType::GGA)
            }
            ParseResult::RMC(rmc) => {
                self.merge_rmc_data(rmc);
                Ok(SentenceType::RMC)
            }
            ParseResult::Unsupported(sentence_type) => Err(nmea::Error::Unsupported(sentence_type)),
            // any other implemented sentence which is not part of the `Nmea` parsing is unsupported
            // at this time being
            ref parse_result => Err(nmea::Error::Unsupported(parse_result.into())),
        }
    }

    fn merge_gga_data(&mut self, gga: nmea::sentences::GgaData) {
        if gga.fix_type.is_none() || !gga.fix_type.unwrap().is_valid() {
            return;
        }

        if gga.latitude.is_none() || gga.longitude.is_none() {
            return;
        }
        let latitude = gga.latitude.unwrap();
        let longitude = gga.longitude.unwrap();

        let pos = GpsPosition::try_from_floats(latitude, longitude, gga.altitude, gga.hdop);

        if pos.is_none() {
            return;
        }

        // we have valid data let's update
        self.utc_time = gga.fix_time.map(|t| t.into());
        if gga.fix_satellites.is_some() {
            self.num_satellites_used = gga.fix_satellites.unwrap() as u8;
        }
        self.pos = pos;
        self.fix_type = if self.num_satellites_used >= 4 {
            FixType::Fix3D
        } else {
            FixType::Fix2D
        };
        self.updated_at = NonZeroU64::new(Instant::now().as_nanos());
    }

    fn merge_rmc_data(&mut self, rmc: nmea::sentences::RmcData) {
        match rmc.status_of_fix {
            nmea::sentences::rmc::RmcStatusOfFix::Invalid => return,
            _ => {}
        }
        if rmc.fix_time.is_none() || rmc.lat.is_none() || rmc.lon.is_none() {
            return;
        }

        let latitude = rmc.lat.unwrap();
        let longitude = rmc.lon.unwrap();

        self.utc_time = rmc.fix_time.map(|t| t.into());
        self.utc_date = rmc.fix_date.map(|t| t.into());
        self.updated_at = NonZeroU64::new(Instant::now().as_nanos());

        let Some(pos) = GpsPosition::try_from_floats(latitude, longitude, None, None) else {
            return;
        };
        let vel = if let (Some(speed_over_ground), Some(course_over_ground)) =
            (rmc.speed_over_ground, rmc.true_course)
        {
            GpsVelocity::try_from_nmea_units(speed_over_ground, course_over_ground)
        } else {
            None
        };

        self.vel = vel;
        self.pos = match self.pos {
            Some(old_pos) => Some(old_pos.update(pos)),
            None => Some(pos),
        };
    }
}
