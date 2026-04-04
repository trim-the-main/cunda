# frb_prpc_juggle

Bridge crate that connects [Flutter Rust Bridge
(FRB)](https://github.com/fzyzcjy/flutter_rust_bridge) with
[postcard-rpc](https://github.com/jamesmunns/postcard-rpc). It handles the wire
protocol on the Flutter/host side and provides the glue macros that make the
protocol crate's definitions callable as idiomatic Dart methods.

## Why this exists

Short reason: This project deals with a lot of code generation. We need this
crate to make things work.

Longer reason: I want to have one `protocol` crate to define the endpoints
and topics, in Rust, and have the flutter app use these definitions. In
practice, even `postcard_rpc` defines this interface using macros which is just
another form of code generation. To make `flutter_rust_bridge` happy and not
to modify `postcard_rpc` I came up with a solution where the protocol crate
actually depends on this crate gated the "flutter" feature flag.

> [!IMPORTANT]
> `endpoints!` macro is used in `protocol` crate to define the EndpointTrait
> (when the `flutter` feature is set) but the `topics!` table has to go
> into the flutter project (`rust_lib_cunda_flutter`). The reason is that
> in order to have topics be translated into dart streams we need to be able
> to name the type `StreamSink` which is not available in the `protocol` crate.
> That is why there are two copies of the topic definitions (one in the protocol
> crate one in `cunda_flutter/rust_lib_cunda_flutter/src/rpc/mod.rs`)
