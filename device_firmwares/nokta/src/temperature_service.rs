use alloc::{string::String, vec::Vec};
use core::fmt::Write;
use esp_hal_rmt_onewire::{Address, Error, OneWire, Search, SearchError};

use crate::context::{RuntimeContext, temperature_data_bus};

pub const MAX_N_SENSORS: usize = 10;

#[derive(Debug, Clone)]
pub struct Ds18B20Sensor {
    addr: Address,
    pub name: String,

    pub value: f32,
    _max_allowed_temp: f32,
    _min_allowed_temp: f32,
}

impl Ds18B20Sensor {
    pub fn new(
        addr: Address,
        name: String,
        value: f32,
        max_allowed_temp: f32,
        min_allowed_temp: f32,
    ) -> Self {
        Self {
            addr,
            name,
            value,
            _max_allowed_temp: max_allowed_temp,
            _min_allowed_temp: min_allowed_temp,
        }
    }
}

pub trait OWBusTempReader {
    fn reset_bus(&mut self) -> impl Future<Output = Result<(), Error>>;
    fn ask_for_measurement(&mut self) -> impl Future<Output = Result<(), Error>>;
    fn restart_search(&mut self);
    fn search_next_address(&mut self) -> impl Future<Output = Result<Address, SearchError>>;
    fn get_temperature(&mut self, addr: Address) -> impl Future<Output = Result<f32, Error>>;
}

pub struct TempReader<'ch> {
    ow: OneWire<'ch>,
    search: Search,
}

impl<'ch> TempReader<'ch> {
    pub fn new(ow: OneWire<'ch>) -> Self {
        Self {
            ow,
            search: Search::new(),
        }
    }
}

impl<'ch> OWBusTempReader for TempReader<'ch> {
    async fn search_next_address(&mut self) -> Result<Address, SearchError> {
        self.search.next(&mut self.ow).await
    }

    async fn get_temperature(&mut self, addr: Address) -> Result<f32, Error> {
        self.ow.reset().await.unwrap();
        self.ow.send_byte(0x55).await.unwrap();
        self.ow.send_address(addr).await.unwrap();
        self.ow.send_byte(0xBE).await.unwrap();
        let temp_low = self
            .ow
            .exchange_byte(0xFF)
            .await
            .expect("failed to get low byte of temperature");
        let temp_high = self
            .ow
            .exchange_byte(0xFF)
            .await
            .expect("failed to get high byte of temperature");
        let temp = fixed::types::I12F4::from_le_bytes([temp_low, temp_high]);
        Ok(temp.into())
    }

    fn restart_search(&mut self) {
        self.search = Search::new();
    }

    async fn reset_bus(&mut self) -> Result<(), Error> {
        self.ow.reset().await?;
        Ok(())
    }

    async fn ask_for_measurement(&mut self) -> Result<(), Error> {
        for a in [0xCC, 0x44] {
            self.ow.send_byte(a).await?;
        }
        Ok(())
    }
}

pub type TemperatureSensorTable = alloc::vec::Vec<Ds18B20Sensor>;
pub struct TemperatureService<'a, 'ch> {
    reader: TempReader<'ch>,
    sensor_table: TemperatureSensorTable,
    announcer: temperature_data_bus::Announcer<'a>,

    name_id: usize,
}

impl<'a, 'ch> TemperatureService<'a, 'ch> {
    pub fn new(ow: OneWire<'ch>, rt_ctxt: &'a RuntimeContext) -> Self {
        let reader = TempReader::new(ow);
        Self {
            reader,
            sensor_table: Vec::with_capacity(MAX_N_SENSORS),
            announcer: rt_ctxt.temperature_billboard.sender(),
            name_id: 0,
        }
    }

    pub async fn read_sensors_and_announce(&mut self) {
        defmt::info!("starting search");
        self.reader.reset_bus().await.unwrap();
        self.reader.ask_for_measurement().await.unwrap();
        self.reader.restart_search();
        loop {
            match self.reader.search_next_address().await {
                Ok(addr) => {
                    defmt::info!("Found addr {}", addr);
                    let value = match self.reader.get_temperature(addr).await {
                        Ok(val) => val,
                        Err(err) => {
                            defmt::error!("Temperature service encountered error {:?}", err);
                            break;
                        }
                    };
                    match self.sensor_table.iter_mut().find(|s| s.addr == addr) {
                        Some(entry) => {
                            defmt::info!(
                                "Updating {} from value {} to {}",
                                entry.name,
                                entry.value,
                                value
                            );
                            entry.value = value;
                        }
                        None => {
                            if self.sensor_table.len() < MAX_N_SENSORS {
                                self.name_id += 1;
                                let mut name = String::new();
                                write!(&mut name, "TempSensor_{}", self.name_id).unwrap();
                                defmt::info!(
                                    "Adding new sensor {} with name value {} to {}",
                                    addr,
                                    name,
                                    value
                                );
                                let entry = Ds18B20Sensor::new(addr, name, value, 120f32, 0f32);
                                self.sensor_table.push(entry);
                            } else {
                                defmt::warn!(
                                    "Found a new temperature sensor but the number of connected sensors exceeds the maximum number {}, ignoring it",
                                    MAX_N_SENSORS
                                );
                            }
                        }
                    }
                    self.announcer.send(self.sensor_table.clone());
                }
                Err(_) => {
                    defmt::info!("Search finished");
                    break;
                }
            }
        }
    }
}
