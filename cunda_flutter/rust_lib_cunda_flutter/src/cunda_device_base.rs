use crate::{
    defmt_log_translation::{DefmtLogEntry, LogDecoderDefmt, LogDecodingError},
    frb_generated::StreamSink,
    log_decoder::LogDecoder,
    rpc::FlutterWire,
};
use frb_prpc_juggle::client_interface::PrpcClient;

/// Cunda device handle for flutter to interact with
/// It implements the log decoder trait giving the mechanism to provide a defmt
/// table to decode bytes to string. It also implements the ClientEndpointInterface
/// and ClientTopicInterface, can be used with any cunda device. We still need to
/// create an "RPC Client" type for each (device, protocol) pair and specify what
/// exact endpoint and topic traits that the rpc client implements.
/// That's why this is a "Base" type, it provides all the necessary plumbing for the
/// protocol clients of different devices. The only RPC endpoint the Base class
/// supports is the get_device_id endpoint, that's the `CundaDevice` trait.
pub struct CundaDeviceBase {
    inner: PrpcClient,
    log_decoder: Option<LogDecoderDefmt>,
}

impl protocol::cunda_common::CundaDevice for CundaDeviceBase {}

impl FlutterWire for CundaDeviceBase {
    #[flutter_rust_bridge::frb(sync)]
    fn init(&mut self, sink: StreamSink<Vec<u8>>) {
        self.inner.init(Box::new(move |data| {
            log::trace!("Sending data from rust to flutter {:?}", data);
            if let Err(e) = sink.add(data) {
                todo!("Error sending data to flutter: {:?}", e);
            }
        }))
    }

    async fn rx_callback(
        &self,
        data: &[u8],
    ) -> Result<(), ::frb_prpc_juggle::client_interface::FrbPostcardRpcError> {
        self.inner.rx_callback(data).await
    }
}

impl LogDecoder for CundaDeviceBase {
    fn init_log_decoder(
        &mut self,
        table_bytes: &[u8],
        loc_bytes: &[u8],
    ) -> Result<(), LogDecodingError> {
        let mut decoder = LogDecoderDefmt::load(table_bytes, Some(loc_bytes));
        // If for some reason we cannot parse the location bytes, carry on for now.
        if let Err(LogDecodingError::LocationDeserFailed) = decoder {
            decoder = LogDecoderDefmt::load(table_bytes, None);
        }
        decoder.map(|d| self.log_decoder = Some(d))
    }

    #[flutter_rust_bridge::frb(sync)]
    fn decode_log(&self, bytes: &[u8]) -> Result<Vec<DefmtLogEntry>, LogDecodingError> {
        match self.log_decoder {
            Some(ref decoder) => decoder.defmt_decode(bytes),
            None => Err(LogDecodingError::NoTableData),
        }
    }
}

/// Boilerplate macro to make a wrapper type of CundaDeviceBase into a
/// proper protocol client. Still need to implement the protocol traits
/// to explicitly opt-in to what endpoints/topics the rpc client supports
macro_rules! impl_protocol_client {
    ($name:ident for ($device_type_path:path, $rpc_version_path:path)) => {
        impl $name {
            #[flutter_rust_bridge::frb(sync)]
            pub fn new(base: $crate::cunda_device_base::CundaDeviceBase) -> Self {
                Self(base)
            }

            #[flutter_rust_bridge::frb(sync)]
            pub fn cunda_device_type() -> String {
                String::from($device_type_path)
            }

            #[flutter_rust_bridge::frb(sync)]
            pub fn cunda_rpc_protocol() -> u32 {
                $rpc_version_path
            }
        }

        impl AsRef<::frb_prpc_juggle::client_interface::PrpcClient> for $name {
            fn as_ref(&self) -> &::frb_prpc_juggle::client_interface::PrpcClient {
                &self.0.as_ref()
            }
        }

        impl AsMut<$crate::cunda_device_base::CundaDeviceBase> for $name {
            fn as_mut(&mut self) -> &mut $crate::cunda_device_base::CundaDeviceBase {
                &mut self.0
            }
        }

        impl AsRef<$crate::cunda_device_base::CundaDeviceBase> for $name {
            fn as_ref(&self) -> &$crate::cunda_device_base::CundaDeviceBase {
                &self.0
            }
        }

        impl $crate::rpc::FlutterWire for $name {
            fn init(&mut self, _sink: $crate::frb_generated::StreamSink<Vec<u8>>) {
                unreachable!("Base device should be initialized already");
            }

            async fn rx_callback(
                &self,
                data: &[u8],
            ) -> Result<(), frb_prpc_juggle::client_interface::FrbPostcardRpcError> {
                <Self as AsRef<$crate::cunda_device_base::CundaDeviceBase>>::as_ref(self)
                    .rx_callback(data)
                    .await
            }
        }

        impl $crate::log_decoder::LogDecoder for $name {
            fn init_log_decoder(
                &mut self,
                table_bytes: &[u8],
                loc_bytes: &[u8],
            ) -> Result<(), $crate::defmt_log_translation::LogDecodingError> {
                self.as_mut().init_log_decoder(table_bytes, loc_bytes)
            }

            #[flutter_rust_bridge::frb(sync)]
            fn decode_log(
                &self,
                bytes: &[u8],
            ) -> Result<
                Vec<$crate::defmt_log_translation::DefmtLogEntry>,
                $crate::defmt_log_translation::LogDecodingError,
            > {
                <Self as AsRef<$crate::cunda_device_base::CundaDeviceBase>>::as_ref(self)
                    .decode_log(bytes)
            }
        }
    };
}

pub(crate) use impl_protocol_client;

// This is how we implement `ClientEndpointInterface` and `ClientTopicInterface`
impl AsRef<PrpcClient> for CundaDeviceBase {
    fn as_ref(&self) -> &PrpcClient {
        &self.inner
    }
}

impl CundaDeviceBase {
    #[flutter_rust_bridge::frb(sync)]
    pub fn new() -> Self {
        Self {
            inner: PrpcClient::new(),
            log_decoder: None,
        }
    }
}
