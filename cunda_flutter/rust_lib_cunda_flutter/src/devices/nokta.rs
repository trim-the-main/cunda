use crate::cunda_device_base::{impl_protocol_client, CundaDeviceBase};
use protocol::devices::nokta;

// Name the client type
pub struct NoktaV1Client(CundaDeviceBase);
// this is a "protocol" client
impl_protocol_client!(NoktaV1Client for (nokta::DEVICE_TYPE, nokta::v1::RPC_PROTOCOL_VERSION));

// Capabilities:
// Endpoints it implements
impl protocol::cunda_common::v1::endpoints::CundaSysE for NoktaV1Client {}
impl protocol::devices::nokta::v1::endpoints::NoktaEndpoints for NoktaV1Client {}

// Topics
impl crate::rpc::CundaSysT for NoktaV1Client {}
protocol::devices::nokta::v1::topics::define_topic_trait!(NoktaTopics with StreamSink);
impl NoktaTopics for NoktaV1Client {}
