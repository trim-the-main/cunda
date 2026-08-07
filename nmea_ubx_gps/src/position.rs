use core::{
    num::{NonZeroU16, NonZeroU32},
    ops::RangeInclusive,
};

use crate::helpers::{try_f32_to_i32, try_f32_to_u16, try_f64_to_i32};

#[cfg_attr(feature = "no-std", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpsPosition {
    /// 1e-7 degrees
    pub lat: i32,
    pub lon: i32,

    /// mean sea level height in mm
    pub h_msl: Option<i32>,

    /// horizontal accuracy 1-sigma in mm
    pub h_acc: Option<NonZeroU32>,

    /// vertical accuracy 1-sigma in mm
    pub v_acc: Option<NonZeroU32>,

    /// position dilution of precision, scale 1e-2
    pub pdop: Option<NonZeroU16>,
}

impl GpsPosition {
    const LAT_RANGE: RangeInclusive<i32> = -900_000_000..=900_000_000;
    const LON_RANGE: RangeInclusive<i32> = -1_800_000_000..=1_800_000_000;

    pub(crate) const fn try_new(lat: i32, lon: i32) -> Option<Self> {
        if lat < *Self::LAT_RANGE.start() || lat > *Self::LAT_RANGE.end() {
            return None;
        }

        if lon < *Self::LON_RANGE.start() || lon > *Self::LON_RANGE.end() {
            return None;
        }

        Some(Self {
            lat,
            lon,
            h_msl: None,
            h_acc: None,
            v_acc: None,
            pdop: None,
        })
    }

    pub(crate) const fn set_pdop(&mut self, pdop: u16) {
        self.pdop = NonZeroU16::new(pdop);
    }

    #[allow(dead_code)]
    pub(crate) const fn set_h_msl(&mut self, h_msl: Option<i32>) {
        self.h_msl = h_msl;
    }

    #[allow(dead_code)]
    pub(crate) const fn with_h_acc(&mut self, h_acc: u32) {
        self.h_acc = NonZeroU32::new(h_acc);
    }

    #[allow(dead_code)]
    pub(crate) const fn with_v_acc(&mut self, v_acc: u32) {
        self.v_acc = NonZeroU32::new(v_acc);
    }

    pub(crate) const fn try_from_floats(
        lat: f64,
        lon: f64,
        h_msl: Option<f32>,
        pdop: Option<f32>,
    ) -> Option<Self> {
        let Some(lat) = try_f64_to_i32(lat * 1e7) else {
            return None;
        };
        let Some(lon) = try_f64_to_i32(lon * 1e7) else {
            return None;
        };

        let Some(mut pos) = Self::try_new(lat, lon) else {
            return None;
        };

        match h_msl {
            Some(height) => pos.h_msl = try_f32_to_i32(height * 1e3),
            None => {}
        }
        match pdop {
            Some(pdop) => match try_f32_to_u16(pdop * 1e2) {
                Some(pdop) => pos.set_pdop(pdop),
                None => {}
            },
            None => {}
        }

        return Some(pos);
    }

    pub(crate) fn update(mut self, new: GpsPosition) -> Self {
        self.lat = new.lat;
        self.lon = new.lon;

        if new.h_msl.is_some() {
            self.h_msl = new.h_msl;
        }
        if new.h_acc.is_some() {
            self.h_acc = new.h_acc;
        }
        if new.v_acc.is_some() {
            self.v_acc = new.v_acc;
        }
        if new.pdop.is_some() {
            self.pdop = new.pdop;
        }
        self
    }
}
