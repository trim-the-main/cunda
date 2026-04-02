use core::sync::atomic;

use maitake_sync::{Mutex, RwLock, WaitMap, wait_map::WakeOutcome};
use postcard_rpc::{Endpoint, Key, Topic, header, host_client::RpcFrame};
use postcard_schema::Schema;
use serde::{Serialize, de::DeserializeOwned};

use crate::accumulator::Accumulator;

#[derive(Debug)]
pub struct WireError {}

// These are the traits to juggle around the FRB limitation of not supporting
// generics
pub trait ClientEndpointInterface {
    fn call_rpc_endpoint<E: Endpoint>(
        &self,
        req: E::Request,
    ) -> impl core::future::Future<Output = Result<E::Response, WireError>> + Send
    where
        E::Request: Serialize + Schema + Send,
        E::Response: DeserializeOwned;
}

pub trait ClientTopicInterface {
    fn subscribe<T: Topic>(
        &self,
        sink: Box<dyn TopicSink>,
    ) -> impl std::future::Future<Output = Result<(), WireError>> + Send
    where
        T::Message: DeserializeOwned;
    fn unsubscribe<T: Topic>(
        &self,
    ) -> impl std::future::Future<Output = Result<(), WireError>> + Send
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
    fn parse_and_add(&self, msg: &[u8]) -> Result<(), WireError>;
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

pub struct Client {
    tx_sink: Option<Box<TxDataCallback>>,
    rx_accumulator: Mutex<Accumulator<1024>>,

    rx_endpoint_response_futures: WaitMap<header::VarHeader, (header::VarHeader, Vec<u8>)>,
    topics: RwLock<Vec<(Key, Box<dyn TopicSink>)>>,
    seq_no: atomic::AtomicU32,
    // Normally I expect this to be part of the protocol crate but for some reason it's defined in the server
    // so what we do is to wait for the first response of the microcontroller and find the value there, update
    // our copy and keep going from there.
    key_kind: RwLock<header::VarKeyKind>,
}

impl Client {
    pub const fn new() -> Self {
        Self {
            tx_sink: None,
            rx_accumulator: Mutex::new(Accumulator::new()),
            rx_endpoint_response_futures: WaitMap::new(),
            topics: RwLock::new(Vec::new()),
            seq_no: atomic::AtomicU32::new(0),
            key_kind: RwLock::new(header::VarKeyKind::Key8),
        }
    }

    pub fn init(&mut self, sink: Box<TxDataCallback>) {
        self.tx_sink = Some(sink);
    }

    pub fn deinit(&mut self) {
        if let Some(sink) = self.tx_sink.take() {
            drop(sink);
        }

        self.topics.get_mut().clear();
        self.rx_endpoint_response_futures.close();
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub async fn rx_callback(&self, data: &[u8]) -> Result<(), WireError> {
        log::debug!("Rust side received data {:?}", data);
        let mut acc = self.rx_accumulator.lock().await;
        acc.feed(data).unwrap();

        // Handle the complete frames in the buffer in a loop by waking up tasks
        // that were waiting on them

        // endpoints and topics must be handled separately here:
        // endpoint callers will wait both on response type and the sequence number so
        // that waitmap keys on the complete VarHeader. topic handlers will wait on
        // the key alone. It's okay to lose some topic messages. In fact here, if we receive
        // multiple frames
        while let Ok(frame) = acc.yield_frame() {
            let Some((hdr, body)) = header::VarHeader::take_from_slice(frame) else {
                log::warn!("Problem with decoding an Rpc buffer");
                continue; // TODO!
            };

            // The frame may be a response to an endpoint request or a topic message
            // if it's an endpoint response we need to find the appropriate endpoint
            // waiter and wake it up else we wake up the topic handler

            // TODO: Evaluate if we should peek into header to see if it's an endpoint
            // response key

            // Handle the frame by waking up whoever was waiting for this,
            // otherwise we toss it away
            match self
                .rx_endpoint_response_futures
                .wake(&hdr, (hdr, body.to_vec()))
            {
                WakeOutcome::Woke => continue,
                WakeOutcome::NoMatch(_) => {} // The header does not match a response that we were waiting for
                WakeOutcome::Closed(_) => {} // TODO: This should probably error, why is the waker closed?
            }

            let topics = self.topics.read().await;
            for (key, topic_cb) in (*topics).iter() {
                if hdr.key == header::VarKey::Key8(*key) {
                    topic_cb.parse_and_add(body)?; // should own a StreamSink, decode the message appropriately and put it in the stream
                }
            }
        }
        Ok(())
    }

    // Send a frame using COBS encoding
    pub async fn send(&self, frame: postcard_rpc::host_client::RpcFrame) -> Result<(), WireError> {
        log::debug!(
            "Sending rpc frame using COBS encoding {:?}",
            frame.to_bytes()
        );
        let mut frame = cobs::encode_vec(&frame.to_bytes());
        frame.push(0); // COBS delimiter
        log::debug!("Encoded version {:?}", frame);
        if let Some(sink) = &self.tx_sink {
            sink(frame);
        }
        Ok(())
    }
}

impl ClientEndpointInterface for Client {
    async fn call_rpc_endpoint<E: postcard_rpc::Endpoint>(
        &self,
        req: E::Request,
    ) -> Result<E::Response, WireError>
    where
        E::Request: serde::Serialize + postcard_schema::Schema,
        E::Response: serde::de::DeserializeOwned,
    {
        log::debug!("Calling rpc endpoint {}", E::PATH);
        let start = std::time::Instant::now();
        let seq = self
            .seq_no
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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

        // We are ready with the package to call send, however first we need to start waiting for the response,
        // otherwise there's a window where the response arrives but nobody is waiting for it.

        // compose the response header that we will wait on:
        // TOOD: errors
        // TODO: is it okay that we are expecting a different size key for example?
        let key_kind = header::VarKeyKind::Key8;
        let mut resp_key = header::VarKey::Key8(E::RESP_KEY);
        resp_key.shrink_to(key_kind);
        let ok_resp_header = header::VarHeader {
            key: resp_key,
            seq_no: frame.header.seq_no,
        };

        let response_future = self.rx_endpoint_response_futures.wait(ok_resp_header);
        let mut response_future = Box::pin(response_future);
        response_future.as_mut().subscribe().await.unwrap();

        self.send(frame).await?;

        // TODO; Handle timeout + connection closed
        let Ok((_hdr, response_body)) = response_future.await else {
            log::warn!("Error waiting for response from microcontroller");
            return Err(WireError {});
        };
        let call_duration = std::time::Instant::now() - start;
        log::debug!("{} took {:?}", E::PATH, call_duration);
        postcard::from_bytes(&response_body).map_err(|_err| WireError {})
    }
}

impl ClientTopicInterface for Client {
    async fn subscribe<T: Topic>(&self, sink: Box<dyn TopicSink>) -> Result<(), WireError>
    where
        T::Message: DeserializeOwned,
    {
        let mut topics = self.topics.write().await;
        if topics.iter().any(|(k, _)| *k == T::TOPIC_KEY) {
            return Err(WireError {});
        }
        topics.push((T::TOPIC_KEY, sink));
        Ok(())
    }

    async fn unsubscribe<T: Topic>(&self) -> Result<(), WireError>
    where
        T::Message: DeserializeOwned,
    {
        let mut topics = self.topics.write().await;
        if let Some(index) = topics.iter().position(|(k, _)| *k == T::TOPIC_KEY) {
            topics.remove(index);
            return Ok(());
        }
        Err(WireError {})
    }
}
