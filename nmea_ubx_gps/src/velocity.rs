use core::num::NonZeroU32;

use crate::helpers::try_f32_to_i32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpsVelocity {
    /// 2d ground speed mm/s
    pub sog: i32,

    /// heading 1e-5 degrees, between [0-360) degrees
    pub cog: i32,

    /// speed accuracy estimate in mm/s
    pub s_acc: Option<NonZeroU32>,

    /// cog accuracy estimate in 1e-5 degrees
    pub c_acc: Option<NonZeroU32>,

    /// 3d velocity in mm/s north-east-down
    pub v_ned: Option<(i32, i32, i32)>,
}

impl GpsVelocity {
    const KNOTS: f32 = 1852000.0 / 3600.0;

    pub(crate) const fn try_new(sog: i32, cog: i32) -> Option<Self> {
        if sog < 0 {
            return None;
        }
        if cog < 0 || cog >= 36_000_000 {
            return None;
        }
        Some(Self {
            sog,
            cog,
            s_acc: None,
            c_acc: None,
            v_ned: None,
        })
    }

    pub(crate) const fn try_from_nmea_units(
        sog_knots: f32,
        course_over_ground: f32,
    ) -> Option<Self> {
        let Some(sog) = try_f32_to_i32(sog_knots * Self::KNOTS) else {
            return None;
        };

        let Some(cog) = try_f32_to_i32(course_over_ground * 1e5) else {
            return None;
        };

        Self::try_new(sog, cog)
    }

    #[allow(dead_code)]
    pub(crate) const fn set_s_acc(&mut self, s_acc: u32) {
        self.s_acc = NonZeroU32::new(s_acc);
    }

    #[allow(dead_code)]
    pub(crate) const fn set_c_acc(&mut self, c_acc: u32) {
        self.c_acc = NonZeroU32::new(c_acc);
    }

    #[allow(dead_code)]
    pub(crate) const fn set_v_ned(&mut self, v_ned: (i32, i32, i32)) {
        self.v_ned = Some(v_ned);
    }
}
