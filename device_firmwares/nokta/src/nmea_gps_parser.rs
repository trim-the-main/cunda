extern crate alloc;

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, pubsub::Publisher};

use embassy_time::{Duration, Instant};
use nmea_ubx_gps::{NmeaParserError, pvt::GpsData};

use crate::context::{RuntimeContext, nmea_data_bus};

pub struct NmeaGpsParserService<'a> {
    sub: nmea_data_bus::Subscriber<'a>,
    gps_notifier: Publisher<'a, CriticalSectionRawMutex, GpsData, 5, 5, 5>,

    data: GpsData,
}

impl<'a> NmeaGpsParserService<'a> {
    const GPS_FIX_TIMEOUT: Duration = Duration::from_secs(20);

    pub fn new(rt_ctxt: &'a RuntimeContext) -> Self {
        Self {
            sub: rt_ctxt.nmea_bcast_channel.subscriber().unwrap(),
            gps_notifier: rt_ctxt.gps_broadcast_channel.publisher().unwrap(),
            data: Default::default(),
        }
    }

    pub fn is_current(&self) -> bool {
        match self.data.updated_at() {
            Some(t) => {
                if (Instant::now() - t) > Self::GPS_FIX_TIMEOUT {
                    false
                } else {
                    true
                }
            }
            None => false,
        }
    }

    pub fn update_from_nmea(&mut self, sentence: &[u8]) {
        match self.data.update_with_sentence(sentence) {
            Ok(sent_ty) => {
                defmt::info!("Success parsing {}", sent_ty);
            }
            Err(err) => match err {
                NmeaParserError::Unsupported(_) => {}
                _ => defmt::error!(
                    "Nmea parse error on sentence: {}",
                    core::str::from_utf8(sentence).unwrap()
                ),
            },
        }
    }

    pub async fn notify_gps_listeners(&self) {
        self.gps_notifier.publish(self.data.clone()).await;
    }
}

pub async fn nmea_consumer_service_run_to_completion(mut srv: NmeaGpsParserService<'_>) {
    loop {
        let msg = srv.sub.next_message().await;
        match msg {
            embassy_sync::pubsub::WaitResult::Lagged(x) => {
                defmt::error!("nmea consumer service lagged {} messages", x)
            }
            embassy_sync::pubsub::WaitResult::Message(sentence_bytes) => {
                srv.update_from_nmea(&sentence_bytes);
            }
        }
        if srv.sub.available() == 0 {
            // update before waiting
            srv.notify_gps_listeners().await;
        }
    }
}
