use postcard_rpc::standard_icd::WireError as RpcWireError;
use postcard_rpc::{Endpoint, Key, Topic, header, host_client::RpcFrame};
use tokio::sync::{Mutex, RwLock, oneshot};

use postcard_schema::Schema;
use serde::{Serialize, de::DeserializeOwned};

use cobs_accumulator::Accumulator;

#[derive(Debug)]
pub enum FrbPostcardRpcError {
    InternalError,
    OnlyOneDartStreamAllowed,
    DeserializationError,
    AlreadySubscribedtoTopic,
    NotSubscribedToTopic,
    RpcError(RpcWireError),
}

// These are the traits to juggle around the FRB limitation of not supporting
// generics
pub trait ClientEndpointInterface {
    fn call_rpc_endpoint<E: Endpoint>(
        &self,
        req: E::Request,
    ) -> impl core::future::Future<Output = Result<E::Response, FrbPostcardRpcError>> + Send
    where
        E::Request: Serialize + Schema + Send,
        E::Response: DeserializeOwned;
}

pub trait ClientTopicInterface {
    fn subscribe<T: Topic>(
        &self,
        sink: Box<dyn TopicSink>,
    ) -> impl std::future::Future<Output = Result<(), FrbPostcardRpcError>> + Send
    where
        T::Message: DeserializeOwned;
    fn unsubscribe<T: Topic>(
        &self,
    ) -> impl std::future::Future<Output = Result<(), FrbPostcardRpcError>> + Send
    where
        T::Message: DeserializeOwned;
}

// This is again a little cumbersome but this is a way to get topics
// working on flutter side as a stream. We are using dynamic dispatch
// for topic message dispatch. StreamSink of flutter_rust_bridge will
// implement this trait so that we can generate code that takes
// frb_generated::StreamSink and puts it into a Box<dyn TopicSink> for
// each topic. Trying to make two generated code to work with one another.
// FRB generates StreamSink and the protocol crate generates (using macro and
// generics) Topic types.
pub trait TopicSink: Send + Sync {
    fn parse_and_add(&self, msg: &[u8]) -> Result<(), FrbPostcardRpcError>;
}

// Send the vector of bytes to flutter to be transmitted over BLE
// We want to use dart stream api to communicate with rust but the
// way that frb handles the stream api is that the streams are
// generated in dart side, split (conseptually) to rx,tx pairs and
// tx is sent to rust side as a StreamSink<T>. Since this StreamSink
// type is defined in generated code, we cannot just have an
// Option<StreamSink<Vec<u8>>>. Here, we instead define a trait object
// that can take care of Vec<u8>. When a BLE connection is established,
// flutter side will construct a stream object and hand over the
// StreamSink so that rust side can construct the closure using this
// sink:
//
// This is most likely something like
//     ```
//         move |data| { sink.add(data);};
//     ```
//
// The idea is that it is constructed after BLE connection is established,
// we discovered the service, found the appropriate characteristics etc.
// The communication in the other direction is handled by a simple callback
// function. frb generates the callback function, dart can call it no
// no problem and we handle the accumulation of frames and deserialization
// dispatch of the responses topic messages etc.
pub type TxDataCallback = dyn Fn(std::vec::Vec<u8>) + Send + Sync;

struct PendingRpcResponseKey {
    ok_header: header::VarHeader,
    err_header: header::VarHeader,
}

impl PendingRpcResponseKey {
    const RPC_ERROR_KEY: header::VarKey =
        header::VarKey::Key8(postcard_rpc::standard_icd::ERROR_KEY);
    const fn new(ok_header: header::VarHeader) -> Self {
        let err_header = header::VarHeader {
            key: Self::RPC_ERROR_KEY,
            seq_no: ok_header.seq_no,
        };
        Self {
            ok_header,
            err_header,
        }
    }
}
type RpcResponseResult = Result<Vec<u8>, Vec<u8>>;

pub struct EndpointCaller {
    tx_sink: Option<Box<TxDataCallback>>,
    seq_no: u32,
}

pub struct EndpointHandle<'a> {
    caller: tokio::sync::MutexGuard<'a, EndpointCaller>,
    key_kind: &'a RwLock<header::VarKeyKind>,
    pending_response:
        &'a Mutex<Option<(PendingRpcResponseKey, oneshot::Sender<RpcResponseResult>)>>,
}

pub struct Client {
    // Serialize RPC calls so only one is on the wire at a time.
    // Don't overwhelm the firmware.
    endpoint: tokio::sync::Mutex<EndpointCaller>,
    // Normally I expect this to be part of the protocol crate but for some reason it's defined in the server
    // so what we do is to wait for the first response of the microcontroller and find the value there, update
    // our copy and keep going from there.
    key_kind: RwLock<header::VarKeyKind>,
    rx_accumulator: Mutex<Accumulator<2048>>,
    pending_response: Mutex<Option<(PendingRpcResponseKey, oneshot::Sender<RpcResponseResult>)>>,
    topics: RwLock<Vec<(Key, Box<dyn TopicSink>)>>,
}

impl Client {
    pub fn new() -> Self {
        Self {
            endpoint: tokio::sync::Mutex::new(EndpointCaller {
                tx_sink: None,
                seq_no: 0u32,
            }),
            key_kind: RwLock::new(header::VarKeyKind::Key8),
            rx_accumulator: Mutex::new(Accumulator::new()),
            pending_response: Mutex::new(None),
            topics: RwLock::new(Vec::new()),
        }
    }

    pub fn init(&mut self, sink: Box<TxDataCallback>) {
        self.endpoint.get_mut().tx_sink = Some(sink);
    }

    pub fn deinit(&mut self) {
        if let Some(sink) = self.endpoint.get_mut().tx_sink.take() {
            drop(sink);
        }

        self.topics.get_mut().clear();
        *self.pending_response.get_mut() = None;
    }

    // This is how we serialize the endpoint calls. Client is locked to make an rpc call but can still
    // receive topic messages.
    pub async fn lock_for_endpoint_call(&self) -> EndpointHandle<'_> {
        EndpointHandle {
            caller: self.endpoint.lock().await,
            key_kind: &self.key_kind,
            pending_response: &self.pending_response,
        }
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub async fn rx_callback(&self, data: &[u8]) -> Result<(), FrbPostcardRpcError> {
        log::trace!("Rust side received data {:?}", data);
        let mut acc = self.rx_accumulator.lock().await;
        acc.feed(data).unwrap();

        // Handle the complete frames in the buffer in a loop by waking up tasks
        // that were waiting on them

        // endpoints and topics must be handled separately here:
        // endpoint callers will wait both on response type and the sequence number so
        // that waitmap keys on the complete VarHeader. topic handlers will wait on
        // the key alone. It's okay to lose some topic messages. In fact here, if we receive
        // multiple frames
        'frame_loop: while let Ok(frame) = acc.yield_frame() {
            let Some((hdr, body)) = header::VarHeader::take_from_slice(frame) else {
                log::warn!("Problem with decoding an Rpc buffer");
                continue; // TODO!
            };

            // The frame may be a response to an endpoint request or a topic message
            // if it's an endpoint response we need to find the appropriate endpoint
            // waiter and wake it up else we wake up the topic handler

            // Handle the frame by waking up whoever was waiting for this,
            // otherwise we toss it away
            {
                let mut pending = self.pending_response.lock().await;
                if let Some((resp_key, _)) = pending.as_ref() {
                    if resp_key.ok_header == hdr || resp_key.err_header == hdr {
                        let (key, tx) = pending.take().unwrap();
                        if key.ok_header == hdr {
                            if hdr.key.kind() != key.ok_header.key.kind() {
                                *self.key_kind.write().await = hdr.key.kind();
                            }
                            let _ = tx.send(Ok(body.to_vec()));
                        } else {
                            let _ = tx.send(Err(body.to_vec()));
                        }
                        continue 'frame_loop;
                    }
                }
            }

            let topics = self.topics.read().await;
            for (key, topic_cb) in (*topics).iter() {
                if hdr.key == header::VarKey::Key8(*key) {
                    topic_cb.parse_and_add(body)?; // should own a StreamSink, decode the message appropriately and put it in the stream
                    continue 'frame_loop;
                }
            }
            // if we came here, we did not do anything with this frame, let's log it at least:
            log::warn!(
                "Did not understand this message: header: {:?} body: {:?}",
                hdr,
                body
            );
        }
        Ok(())
    }
}

impl EndpointHandle<'_> {
    async fn call_rpc_endpoint<E: postcard_rpc::Endpoint>(
        &self,
        req: E::Request,
    ) -> Result<E::Response, FrbPostcardRpcError>
    where
        E::Request: serde::Serialize + postcard_schema::Schema,
        E::Response: serde::de::DeserializeOwned,
    {
        log::debug!("Calling rpc endpoint {}", E::PATH);
        let start = std::time::Instant::now();
        let seq = self.caller.seq_no.wrapping_add(1);
        let mut frame = RpcFrame {
            header: header::VarHeader {
                key: header::VarKey::Key8(E::REQ_KEY),
                seq_no: header::VarSeq::Seq4(seq),
            },
            body: postcard::to_stdvec(&req).expect("Allocation failure"),
        };

        // Shrink the header
        if seq < u8::MAX.into() {
            frame.header.seq_no.resize(header::VarSeqKind::Seq1);
        } else if seq < u16::MAX.into() {
            frame.header.seq_no.resize(header::VarSeqKind::Seq2);
        }

        let key_kind = *self.key_kind.read().await;
        frame.header.key.shrink_to(key_kind);

        // compose the response header that we will wait on:
        let key_kind = header::VarKeyKind::Key8;
        let mut resp_key = header::VarKey::Key8(E::RESP_KEY);
        resp_key.shrink_to(key_kind);
        let ok_resp_header = header::VarHeader {
            key: resp_key,
            seq_no: frame.header.seq_no,
        };

        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending_response.lock().await;
            *pending = Some((PendingRpcResponseKey::new(ok_resp_header), tx));
        }

        // Send using COBS encoding
        log::trace!(
            "Sending rpc frame using COBS encoding {:?}",
            frame.to_bytes()
        );
        let mut encoded = cobs::encode_vec(&frame.to_bytes());
        encoded.push(0); // COBS delimiter
        log::trace!("Encoded version {:?}", encoded);
        if let Some(sink) = &self.caller.tx_sink {
            sink(encoded);
        }

        // TODO: Handle timeout + connection closed
        let result = rx.await.map_err(|_| FrbPostcardRpcError::InternalError)?;

        let call_duration = std::time::Instant::now() - start;
        log::debug!("{} took {:?}", E::PATH, call_duration);

        match result {
            Ok(body) => {
                postcard::from_bytes(&body).map_err(|_| FrbPostcardRpcError::DeserializationError)
            }
            Err(body) => {
                let err = postcard::from_bytes::<postcard_rpc::standard_icd::WireError>(&body)
                    .map_err(|_| FrbPostcardRpcError::DeserializationError)?;
                Err(FrbPostcardRpcError::RpcError(err))
            }
        }
    }
}

impl ClientEndpointInterface for Client {
    async fn call_rpc_endpoint<E: Endpoint>(
        &self,
        req: E::Request,
    ) -> Result<E::Response, FrbPostcardRpcError>
    where
        E::Request: Serialize + Schema + Send,
        E::Response: DeserializeOwned,
    {
        self.lock_for_endpoint_call()
            .await
            .call_rpc_endpoint::<E>(req)
            .await
    }
}

impl ClientTopicInterface for Client {
    async fn subscribe<T: Topic>(&self, sink: Box<dyn TopicSink>) -> Result<(), FrbPostcardRpcError>
    where
        T::Message: DeserializeOwned,
    {
        let mut topics = self.topics.write().await;
        if topics.iter().any(|(k, _)| *k == T::TOPIC_KEY) {
            return Err(FrbPostcardRpcError::AlreadySubscribedtoTopic);
        }
        topics.push((T::TOPIC_KEY, sink));
        Ok(())
    }

    async fn unsubscribe<T: Topic>(&self) -> Result<(), FrbPostcardRpcError>
    where
        T::Message: DeserializeOwned,
    {
        let mut topics = self.topics.write().await;
        if let Some(index) = topics.iter().position(|(k, _)| *k == T::TOPIC_KEY) {
            topics.remove(index);
            return Ok(());
        }
        Err(FrbPostcardRpcError::NotSubscribedToTopic)
    }
}

impl<T> ClientEndpointInterface for T
where
    T: AsRef<Client>,
{
    fn call_rpc_endpoint<E: Endpoint>(
        &self,
        req: E::Request,
    ) -> impl core::future::Future<Output = Result<E::Response, FrbPostcardRpcError>> + Send
    where
        E::Request: Serialize + Schema + Send,
        E::Response: DeserializeOwned,
    {
        self.as_ref().call_rpc_endpoint::<E>(req)
    }
}

impl<C> ClientTopicInterface for C
where
    C: AsRef<Client>,
{
    fn subscribe<T: Topic>(
        &self,
        sink: Box<dyn TopicSink>,
    ) -> impl std::future::Future<Output = Result<(), FrbPostcardRpcError>> + Send
    where
        T::Message: DeserializeOwned,
    {
        self.as_ref().subscribe::<T>(sink)
    }

    fn unsubscribe<T: Topic>(
        &self,
    ) -> impl std::future::Future<Output = Result<(), FrbPostcardRpcError>> + Send
    where
        T::Message: DeserializeOwned,
    {
        self.as_ref().unsubscribe::<T>()
    }
}
