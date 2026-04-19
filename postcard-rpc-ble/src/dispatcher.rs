use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Receiver;
use postcard_rpc::header::VarHeader;
use postcard_rpc::server::{self, AsWireTxErrorKind, Dispatch, WireTxErrorKind};

use cobs_accumulator::{Accumulator, DecodeError};
use trouble_host::gatt::WriteEvent;
use trouble_host::prelude::DefaultPacketPool;

/// Runs the dispatcher loop. Handed to the dispatcher task.
pub struct DispatcherRunner<'storage, 'stack, 'server, const CH_SIZE: usize> {
    rx: Receiver<'storage, NoopRawMutex, WriteEvent<'stack, 'server, DefaultPacketPool>, CH_SIZE>,
}

impl<'storage, 'stack, 'server, const CH_SIZE: usize>
    DispatcherRunner<'storage, 'stack, 'server, CH_SIZE>
{
    pub(crate) const fn new(
        rx: Receiver<
            'storage,
            NoopRawMutex,
            WriteEvent<'stack, 'server, DefaultPacketPool>,
            CH_SIZE,
        >,
    ) -> Self {
        Self { rx }
    }
    /// Run the dispatcher loop. Generic over the `Dispatch` implementation
    /// and the accumulator buffer size (set `RX_BUF_SIZE` to accommodate
    /// your largest message, e.g. 4352 for 4096-byte OTA transfers).
    pub async fn run<D: Dispatch, const RX_BUF_SIZE: usize>(
        &self,
        d: &mut D,
        tx: server::Sender<D::Tx>,
    ) {
        let mut acc = Accumulator::<RX_BUF_SIZE>::new();

        loop {
            defmt::debug!("Dispatcher waiting for write event");

            let w_event = self.rx.receive().await;
            if acc.feed(w_event.data()).is_err() {
                acc.reset();
            }

            match w_event.payload().incoming() {
                trouble_host::att::AttClient::Request(_att_req) => {
                    defmt::debug!("Write request received {}", last(w_event.data()));
                    match w_event.accept() {
                        Ok(reply) => {
                            reply.send().await;
                        }
                        Err(err) => {
                            defmt::warn!(
                                "Error sending acknowledgement on the received package {}",
                                err
                            );
                        }
                    }
                }
                trouble_host::att::AttClient::Command(_) => {}
                trouble_host::att::AttClient::Confirmation(_) => {}
            }

            loop {
                match acc.yield_frame() {
                    Ok(buf) => {
                        defmt::debug!("Acc yielded a frame");

                        let Some((hdr, body)) = VarHeader::take_from_slice(buf) else {
                            defmt::warn!("Bad message of size {}. Discarding...", buf.len());
                            continue;
                        };
                        let fut = d.handle(&tx, &hdr, body);
                        defmt::debug!("Dispatcher task waiting on the handler");
                        if let Err(e) = fut.await {
                            let kind = e.as_kind();
                            defmt::error!("Endpoint request resulted in error: {}", kind);
                            match kind {
                                WireTxErrorKind::ConnectionClosed => break,
                                WireTxErrorKind::Other => continue,
                                WireTxErrorKind::Timeout => continue,
                                _ => continue,
                            }
                        }
                        defmt::debug!("Dispatcher task returned ok");
                    }
                    Err(DecodeError::DecodingError) => continue,
                    Err(DecodeError::Incomplete) | Err(DecodeError::NoData) => break,
                }
            }
        }
    }
}
fn last<T: Copy>(s: &[T]) -> Option<T> {
    if s.len() < 2 {
        None
    } else {
        Some(s[s.len() - 2])
    }
}
