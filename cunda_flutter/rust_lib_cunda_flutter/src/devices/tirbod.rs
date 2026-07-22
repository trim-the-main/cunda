use crate::cunda_device_base::{impl_protocol_client, CundaDeviceBase};
use protocol::devices::tirbod;

pub struct TirbodV1Client(CundaDeviceBase);
impl_protocol_client!(TirbodV1Client for (tirbod::DEVICE_TYPE, tirbod::v1::RPC_PROTOCOL_VERSION));

impl protocol::cunda_common::v1::endpoints::CundaSysE for TirbodV1Client {}
impl tirbod::v1::endpoints::TirbodEndpoints for TirbodV1Client {}

impl crate::rpc::CundaSysT for TirbodV1Client {}
protocol::devices::tirbod::v1::topics::define_topic_trait!(TirbodTopics with StreamSink);
impl TirbodTopics for TirbodV1Client {}
