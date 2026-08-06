pub mod defs {
    use embedded_hal_async::i2c;
    pub type I2CType = esp_hal::i2c::master::I2c<'static, esp_hal::Async>;

    pub struct SharedI2C<'a, BUS> {
        bus: &'a maitake_sync::Mutex<BUS>,
    }

    impl<'a, BUS> SharedI2C<'a, BUS> {
        /// Create a new `I2cDevice`.
        pub fn new(bus: &'a maitake_sync::Mutex<BUS>) -> Self {
            Self { bus }
        }
    }

    impl<BUS> Clone for SharedI2C<'_, BUS> {
        fn clone(&self) -> Self {
            Self { bus: self.bus }
        }
    }

    impl<BUS> i2c::ErrorType for SharedI2C<'_, BUS>
    where
        BUS: i2c::ErrorType,
    {
        type Error = <BUS as i2c::ErrorType>::Error;
    }

    impl<BUS> i2c::I2c for SharedI2C<'_, BUS>
    where
        BUS: i2c::I2c,
    {
        async fn transaction(
            &mut self,
            address: u8,
            operations: &mut [i2c::Operation<'_>],
        ) -> Result<(), Self::Error> {
            let mut bus = self.bus.lock().await;
            bus.transaction(address, operations).await
        }
    }
}
