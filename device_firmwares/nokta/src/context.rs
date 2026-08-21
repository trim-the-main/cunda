use embassy_sync::pubsub::PubSubChannel;

pub mod nmea_data_bus {
    use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, pubsub};
    const CAPACITY: usize = 130;
    const PUBLISHERS: usize = 4;
    const SUBSCRIBERS: usize = 4;

    pub type Channel = pubsub::PubSubChannel<
        CriticalSectionRawMutex,
        alloc::vec::Vec<u8>,
        CAPACITY,
        SUBSCRIBERS,
        PUBLISHERS,
    >;
    pub type Subscriber<'a> = pubsub::Subscriber<
        'a,
        CriticalSectionRawMutex,
        alloc::vec::Vec<u8>,
        CAPACITY,
        SUBSCRIBERS,
        PUBLISHERS,
    >;
    pub type Publisher<'a> = pubsub::Publisher<
        'a,
        CriticalSectionRawMutex,
        alloc::vec::Vec<u8>,
        CAPACITY,
        SUBSCRIBERS,
        PUBLISHERS,
    >;
}

pub mod gps_data_bus {
    use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, pubsub};
    use nmea_ubx_gps::pvt::GpsData;

    const CAPACITY: usize = 5;
    const PUBLISHERS: usize = 5;
    const SUBSCRIBERS: usize = 5;

    pub type Channel =
        pubsub::PubSubChannel<CriticalSectionRawMutex, GpsData, CAPACITY, SUBSCRIBERS, PUBLISHERS>;
    pub type Subscriber<'a> =
        pubsub::Subscriber<'a, CriticalSectionRawMutex, GpsData, CAPACITY, SUBSCRIBERS, PUBLISHERS>;
    pub type Publisher<'a> =
        pubsub::Publisher<'a, CriticalSectionRawMutex, GpsData, CAPACITY, SUBSCRIBERS, PUBLISHERS>;
}

// pub mod temperature_data_bus {
//     use crate::temperature_service::TemperatureSensorTable;
//     use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch};

//     pub type BillBoard = watch::Watch<CriticalSectionRawMutex, TemperatureSensorTable, 4>;
//     pub type Announcer<'a> = watch::Sender<'a, CriticalSectionRawMutex, TemperatureSensorTable, 4>;
//     pub type Follower<'a> = watch::Receiver<'a, CriticalSectionRawMutex, TemperatureSensorTable, 4>;
// }

pub struct RuntimeContext {
    // Put all the state that requires to be passed around or slept on here:
    pub gps_broadcast_channel: gps_data_bus::Channel,
    // pub screen_page: Watch<CriticalSectionRawMutex, ScreenPages, 3>,
    pub nmea_bcast_channel: nmea_data_bus::Channel,
    // pub temperature_billboard: temperature_data_bus::BillBoard,
}

impl RuntimeContext {
    pub fn new() -> Self {
        Self {
            gps_broadcast_channel: PubSubChannel::new(),
            // screen_page: Watch::new_with(ScreenPages::VersionPage),
            nmea_bcast_channel: nmea_data_bus::Channel::new(),
            // temperature_billboard: temperature_data_bus::BillBoard::new(),
        }
    }
}

impl Default for RuntimeContext {
    fn default() -> Self {
        Self::new()
    }
}
