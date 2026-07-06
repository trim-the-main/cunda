use crate::{
    cunda_device_base::{impl_protocol_client, CundaDeviceBase},
    frb_generated::StreamSink,
};
use protocol::devices::demo_esp32;

// Since topic trait definition includes the StreamSink type, we define it here using the macro
// from the protocol crate:
protocol::devices::demo_esp32::v1::topics::define_topic_trait!(DemoAppTopics with StreamSink);

pub struct DemoV1Client(CundaDeviceBase);
impl_protocol_client!(DemoV1Client for (demo_esp32::DEVICE_TYPE, demo_esp32::v1::RPC_PROTOCOL_VERSION));

// The protocol capabilities:
impl protocol::cunda_common::v1::endpoints::CundaSysE for DemoV1Client {}
impl protocol::devices::demo_esp32::v1::endpoints::DemoAppEndpoints for DemoV1Client {}

impl crate::rpc::CundaSysT for DemoV1Client {}
impl DemoAppTopics for DemoV1Client {}
