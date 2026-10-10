# postcard-rpc-ble

This crate bridges [postcard-rpc] and [trouble] as it is used in `cunda`
project. A subset of the operations in [postcard-rpc] is supported.

Limitations include (these are `cunda` design decisions):
- No support for `TOPIC` messages going to the RPC server (towards the
micro-controller). Use endpoints instead.
- Only one endpoint request at a time in flight. If this is a long running
request, endpoints can be used to spawn embassy tasks which can send topic
messages.
 
### Transport

`postcard-rpc` frames are not assumed to fit in one BLE "read/write" operation.
We have to send them in chunks. We use two characteristics in each direction.
One characteristic for un-acked messages, a second characteristic for acked.
Each `postcard-rpc` message is split up in MTU sized chunks and all but the
last message is sent over the un-acked characteristic. `server` sends the last
chunk on the acked characteristic and the client will acknowledge it.
`postcard-rpc` frames are COBS encoded and `0x00` terminated. That's why losing
a last chunk because it was delivered as an unacked "notification" would corrupt
the next message too. The last chunk including the frame delimiter is always
transmitted as an acked "indication".

### Server loop

Instead of implementing `WireRx` trait and using the `postcard-rpc` default
server loop, I chose to implement the server loop myself. Plugging the BLE
advertisement as `async fn wait_connection(&mut self)` was the first avenue
I tried but marrying `postcard-rpc` abstractions and `trouble` abstractions
turned to be too difficult. It is substantially easier to reason about a task
which handles the received data and the confirmations (acks) of the sent data,
and another task that aggregates the received bytes, deserialing the frames
and dispatching the endpoints. For this I used `embassy-sync` channel and to
avoid copying data I directly pass the BLE packets from the packet pool.

### Concurrency model

Multiple copies of `BleWireTx` exists: each topic task can publish messages and
the endpoint dispatcher task also needs to be able to send responses back. They
access to the `GattServerRpc` and the current `GattConnection` protected by an
`RwLock`. The endpoint dispatcher task is responsible for advertising, creating
a `GattConnection`.

[postcard-rpc]: https://github.com/jamesmunns/postcard-rpc
[trouble]: https://github.com/embassy-rs/trouble
