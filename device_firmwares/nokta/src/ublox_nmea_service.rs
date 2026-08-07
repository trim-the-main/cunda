use alloc::{vec, vec::Vec};

use core::result::{Result, Result::Ok};
use embedded_hal_async::i2c::{Error, ErrorKind, I2c as I2cTrait};
extern crate alloc;

const ADDR: u8 = 0x42;

use crate::context::{RuntimeContext, nmea_data_bus};

#[derive(Debug, defmt::Format)]
pub enum UbloxNMEAPublisherError {
    // I2C related errors
    Bus,
    DeviceDidNotRespond,
    Overrun,
    Other,

    InitError,
    Uninitialized,

    // Decoding related errors
    BufferTooSmall,
    Utf8ParseError,
    NmeaParseError,
}

impl<E: Error> From<E> for UbloxNMEAPublisherError {
    fn from(err: E) -> Self {
        match err.kind() {
            ErrorKind::Bus => UbloxNMEAPublisherError::Bus,
            ErrorKind::ArbitrationLoss => UbloxNMEAPublisherError::Bus,
            ErrorKind::NoAcknowledge(_) => UbloxNMEAPublisherError::DeviceDidNotRespond,
            ErrorKind::Overrun => UbloxNMEAPublisherError::Overrun,
            ErrorKind::Other => UbloxNMEAPublisherError::Other,
            _ => unreachable!("An unknown errorkind variant is received"),
        }
    }
}

// UbloxNMEAPublisher is a task that reads data from i2c and publishes NMEA
// sentences as Vec<u8> into the data bus

pub struct UbloxNMEAPublisher<'a, I2C> {
    i2c: I2C,

    publisher: nmea_data_bus::Publisher<'a>,
}
impl<'a, I2C: I2cTrait> UbloxNMEAPublisher<'a, I2C> {
    pub fn new(i2c: I2C, rt_ctxt: &'a RuntimeContext) -> Self {
        let publisher = rt_ctxt.nmea_bcast_channel.publisher().unwrap();

        Self { i2c, publisher }
    }

    async fn read_from_i2c(&mut self, buf: &mut [u8]) -> Result<usize, UbloxNMEAPublisherError> {
        let mut size_buf = [0u8; 2];

        self.i2c.write_read(ADDR, &[0xFD], &mut size_buf).await?;
        // From sparkfun driver, it seems rare
        if size_buf[0] > 127 {
            defmt::error!("MSB error");
            size_buf[0] -= 128;
        }
        let size: usize = ((size_buf[0] as u16) << 8 | (size_buf[1] as u16)).into();
        defmt::trace!("Got size: {}", size);
        //

        let mut remaining_bytes: usize = if size > buf.len() {
            defmt::warn!(
                "i2c buffer size {} is not big enough for ublox i2c message of size {}",
                buf.len(),
                size
            );
            buf.len()
        } else {
            size
        };
        let mut copied = 0;
        const WINDOW_SIZE: usize = 31;
        while remaining_bytes > 0 {
            let bytes_to_read = if remaining_bytes < WINDOW_SIZE {
                remaining_bytes
            } else {
                WINDOW_SIZE
            };
            match self
                .i2c
                .write_read(ADDR, &[0xFF], &mut buf[copied..copied + bytes_to_read])
                .await
            {
                Ok(_) => {
                    remaining_bytes -= bytes_to_read;
                    copied += bytes_to_read;
                }
                Err(err) => {
                    defmt::error!(
                        "Failed to read the payload, remaining bytes {:?}",
                        remaining_bytes
                    );
                    return Err(err.into());
                }
            }
        }
        Ok(copied)
    }

    pub async fn do_work(&mut self) -> Result<bool, UbloxNMEAPublisherError> {
        let mut big_buffer = vec![0u8; 2048];

        let mut nmea_sentence: Option<Vec<u8>> = None;

        let data = match self.read_from_i2c(&mut big_buffer).await {
            Ok(bytes_read) => &big_buffer[..bytes_read],
            Err(UbloxNMEAPublisherError::DeviceDidNotRespond) => {
                return Err(UbloxNMEAPublisherError::DeviceDidNotRespond);
            }
            Err(err) => {
                defmt::error!("ublox task got i2c error: {:?}", err);
                &[] // No data
            }
        };

        if data.is_empty() {
            return Ok(false);
        }
        for char in data {
            // beginning of a new sentence
            if *char == b'$' {
                nmea_sentence = Some(vec![b'$']);
                continue;
            }

            // there's an active sentence so append the bytes
            if let Some(current_sentence) = &mut nmea_sentence {
                current_sentence.push(*char);
                // end of a sentence
                if *char == b'\n' {
                    self.publisher.publish(current_sentence.clone()).await;
                }
            }
        }

        Ok(true)
    }
}
