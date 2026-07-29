macro_rules! impl_float_to_int {
    ($float:ty, $integer: ty) => {
        pastey::paste! {
            pub(crate) const fn [<try_ $float:snake _to_ $integer:snake>](f: $float) -> Option<$integer> {
                if f.is_nan() || f.is_infinite() {
                    return None;
                }
                if f >= ($integer::MIN as $float) && f <= ($integer::MAX as $float) {
                    Some(f as $integer)
                } else {
                    None
                }
            }
        }
    };
}

impl_float_to_int!(f64, i32);
impl_float_to_int!(f32, i32);
impl_float_to_int!(f32, u16);
