use alloc::vec::Vec;

use core::result::{Result, Result::Ok};
use embedded_hal_async::i2c::{Error, ErrorKind, I2c as I2cTrait};
extern crate alloc;

const ADDR: u8 = 0x42;
const UBX_SYNC_1: u8 = 0xB5;
const UBX_SYNC_2: u8 = 0x62;

use crate::context::{RuntimeContext, gps_data_bus, nmea_data_bus};

#[derive(Debug, defmt::Format)]
pub enum UbloxPublisherError {
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

impl<E: Error> From<E> for UbloxPublisherError {
    fn from(err: E) -> Self {
        match err.kind() {
            ErrorKind::Bus => UbloxPublisherError::Bus,
            ErrorKind::ArbitrationLoss => UbloxPublisherError::Bus,
            ErrorKind::NoAcknowledge(_) => UbloxPublisherError::DeviceDidNotRespond,
            ErrorKind::Overrun => UbloxPublisherError::Overrun,
            ErrorKind::Other => UbloxPublisherError::Other,
            _ => unreachable!("An unknown errorkind variant is received"),
        }
    }
}

pub struct UbloxPublisher<'a, I2C> {
    i2c: I2C,

    // raw nmea messages goes here
    nmea_publisher: nmea_data_bus::Publisher<'a>,

    // parsed GpsData is published here
    gps_publisher: gps_data_bus::Publisher<'a>,

    buf: Vec<u8>,
    write_pos: usize,
}

impl<'a, I2C: I2cTrait> UbloxPublisher<'a, I2C> {
    const BUF_SIZE: usize = 2048;

    pub fn new(i2c: I2C, rt_ctxt: &'a RuntimeContext) -> Self {
        let nmea_publisher = rt_ctxt.nmea_bcast_channel.publisher().unwrap();
        let gps_publisher = rt_ctxt.gps_broadcast_channel.publisher().unwrap();

        let buf = alloc::vec::from_elem(0u8, Self::BUF_SIZE);

        Self {
            i2c,
            nmea_publisher,
            gps_publisher,
            buf,
            write_pos: 0,
        }
    }

    fn remove_processed_bytes(&mut self, processed_bytes: usize) {
        if processed_bytes > 0 {
            if processed_bytes < self.write_pos {
                self.buf.copy_within(processed_bytes..self.write_pos, 0);
                self.write_pos -= processed_bytes;
            } else {
                // processed_bytes == write_pos, nothing to copy
                self.write_pos = 0;
            }
        }
    }

    fn available_buf_room(&self) -> usize {
        Self::BUF_SIZE - self.write_pos
    }

    async fn read_from_i2c(&mut self) -> Result<(), UbloxPublisherError> {
        let mut size_buf = [0u8; 2];

        self.i2c.write_read(ADDR, &[0xFD], &mut size_buf).await?;
        // From sparkfun driver, it seems rare
        if size_buf[0] > 127 {
            defmt::error!("MSB error");
            size_buf[0] -= 128;
        }
        let size: usize = ((size_buf[0] as u16) << 8 | (size_buf[1] as u16)).into();
        defmt::trace!("Got size: {}", size);

        let mut remaining_bytes: usize = if size > self.available_buf_room() {
            defmt::warn!(
                "i2c buffer size {} is not big enough for ublox i2c message of size {}",
                self.available_buf_room(),
                size
            );
            self.available_buf_room()
        } else {
            size
        };
        const WINDOW_SIZE: usize = 31;
        while remaining_bytes > 0 {
            let bytes_to_read = if remaining_bytes < WINDOW_SIZE {
                remaining_bytes
            } else {
                WINDOW_SIZE
            };
            match self
                .i2c
                .write_read(
                    ADDR,
                    &[0xFF],
                    &mut self.buf[self.write_pos..self.write_pos + bytes_to_read],
                )
                .await
            {
                Ok(_) => {
                    remaining_bytes -= bytes_to_read;
                    self.write_pos += bytes_to_read;
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
        Ok(())
    }

    pub async fn configure_ubx(&mut self) {
        let mut config_buf = alloc::vec::Vec::<u8>::new();
        nmea_ubx_gps::ubx::config_sam_m10q_i2c(&mut config_buf);
        defmt::info!("ublox config bytes len {}", config_buf.len());
        defmt::info!("ublox config bytes: {:X}", config_buf.as_slice());
        if let Err(err) = self.i2c.write(ADDR, config_buf.as_slice()).await {
            defmt::error!("Error sending config bytes to ublox {}", err.kind());
        };
    }

    pub async fn do_work(&mut self) -> Result<bool, UbloxPublisherError> {
        let mut nmea_sentence_start: Option<usize> = None;
        let mut ubx_parser = nmea_ubx_gps::ubx::new_parser();

        let data = match self.read_from_i2c().await {
            Ok(_) => &self.buf[..self.write_pos],
            Err(UbloxPublisherError::DeviceDidNotRespond) => {
                return Err(UbloxPublisherError::DeviceDidNotRespond);
            }
            Err(err) => {
                defmt::error!("ublox task got i2c error: {:?}", err);
                &[] // No data
            }
        };

        if data.is_empty() {
            return Ok(false);
        }

        let mut processed_bytes: usize = 0;
        let mut i = 0;
        while i < data.len() {
            let char = data[i];
            match nmea_sentence_start {
                None => {
                    // beginning of ubx message
                    if char == UBX_SYNC_1 {
                        let slice = &data[i..];

                        // Ensure sync byte 2 is present
                        if slice.len() >= 2 && slice[1] == UBX_SYNC_2 {
                            // Need at least 6 header bytes to extract UBX length field:
                            // [0..2] Sync, [2] Class, [3] ID, [4..6] Length (u16, Little-Endian)
                            if slice.len() >= 6 {
                                let payload_len = u16::from_le_bytes([slice[4], slice[5]]) as usize;
                                let total_msg_len = 6 + payload_len + 2; // Header + Payload + 2 Checksum Bytes
                                if slice.len() >= total_msg_len {
                                    processed_bytes += total_msg_len;
                                    ubx_parser.consume_ubx(&slice[..total_msg_len]);
                                    i += total_msg_len;
                                    continue; // bypass i+=1
                                } else {
                                    // we didn't get the entire packet. Leave the bytes in the buffer
                                    // and break the loop. We don't advance `processed_bytes` index
                                    break;
                                }
                            } else {
                                // there's not enough data in the buffer for the whole header. Leave the header
                                // bytes in the buffer.
                                break;
                            }
                        } else if slice.len() >= 2 {
                            // byte 2 did not match, byte 1 is garbage
                            processed_bytes += 1
                        } else {
                            // The UBX_SYNC_1 byte was the last byte
                            break; // we don't need this but easier to read
                        }
                    }
                    // beginning of a new nmea sentence
                    else if char == b'$' {
                        nmea_sentence_start = Some(i);
                    }
                    // we got garbage, none of the start bytes matched
                    else {
                        processed_bytes += 1;
                    }
                }
                Some(nmea_start) => {
                    if char == b'\n' {
                        let mut s = Vec::new();
                        s.extend_from_slice(&data[nmea_start..i + 1]);
                        processed_bytes += s.len();
                        self.nmea_publisher.publish(s).await;
                        nmea_sentence_start = None;
                    }
                }
            }
            i += 1;
        }
        self.remove_processed_bytes(processed_bytes);

        let mut it = ubx_parser.consume_ubx(&[]);
        loop {
            match it.next() {
                Some(Ok(packet)) => {
                    // We've received a &PacketRef, we can handle it
                    // Or we can convert it to an owned structure, so we can move it
                    match packet {
                        ublox::UbxPacket::Proto33(packet_ref) => match packet_ref {
                            ublox::proto33::PacketRef::NavPvt(nav_pvt_ref) => {
                                let mut data = nmea_ubx_gps::pvt::GpsData::new();
                                if let Err(_) = data.update_with_ubx_nav_pvt(nav_pvt_ref) {
                                    defmt::warn!("NAV_PVT parse error");
                                }
                                self.gps_publisher.publish(data).await;
                            }
                            ublox::proto33::PacketRef::AckAck(_) => {
                                defmt::info!("Got ACK response on UBX protocol");
                            }
                            ublox::proto33::PacketRef::AckNak(_) => {
                                defmt::info!("Got NAK response on UBX protocol");
                            }
                            _ => {
                                defmt::info!("Got other UBX message");
                            }
                        },
                    }
                }
                Some(Err(err)) => match err {
                    ublox::ParserError::InvalidChecksum { .. } => {
                        defmt::warn!("UBX parse error: InvalidChecksum");
                    }
                    ublox::ParserError::InvalidField { .. } => {
                        defmt::warn!("UBX parse error: InvalidField")
                    }
                    ublox::ParserError::InvalidPacketLen { .. } => {
                        defmt::warn!("UBX parse error: InvalidPacketLen")
                    }
                    ublox::ParserError::OutOfMemory { .. } => {
                        defmt::warn!("UBX parse error: OutOfMemory")
                    }
                },
                None => {
                    // The internal buffer is now empty
                    break;
                }
            }
        }
        Ok(true)
    }
}
