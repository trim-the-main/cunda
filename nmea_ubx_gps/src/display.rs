use core::fmt::Display;

#[cfg(feature = "embassy-time")]
use num_traits::float::FloatCore;

pub struct Latitude(pub f64);
pub struct Longitude(pub f64);
pub struct LatitudeLongitude(pub f64, pub f64);

impl Display for Latitude {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let lat_degree = self.0.abs() as u8;
        let lat_mins = self.0.abs().fract() * 60.0;
        let hem_ns = if self.0 > 0.0 { 'N' } else { 'S' };

        write!(f, "{: >2}° {: >3.3}' {}", lat_degree, lat_mins, hem_ns,)
    }
}

impl Display for Longitude {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let lon_degree = self.0.abs() as u8;
        let lon_mins = self.0.abs().fract() * 60.0;
        let hem_ew = if self.0 > 0.0 { 'E' } else { 'W' };

        write!(f, "{: >3}° {: >3.3}' {} ", lon_degree, lon_mins, hem_ew)
    }
}

impl Display for LatitudeLongitude {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} {}", Latitude(self.0), Longitude(self.1))?;
        Ok(())
    }
}
