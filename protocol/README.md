# protocol

Shared Rust crate that defines the full RPC contract between the ESP32 firmware
and the Flutter app. Both sides depend on this crate — firmware directly,
Flutter via code generation through `frb_prpc_juggle`.

There are certain limitations of `flutter_rust_bridge` that causes this crate to
be a bit weird. One such example is that `flutter_rust_bridge` does not handle
unit structs and the unit type. That's why we have `NoArg` or `EmptyRes` types.
Both of them are equivalent to the unit type in the wire but we have to type
them as such.

## `flutter` feature flag

The crate is `no_std` by default (for firmware). When compiled with `features
= ["flutter"]`, it switches to `std::String` / `std::Vec` instead of
`heapless::String` / `heapless::Vec`. This flag is set automatically when the
Flutter native library depends on this crate. The `ProtocolStringType!` and
`ProtocolVecType!` macros abstract over this difference so the type definitions
stay identical in both targets. For the heapless types we need a max length.
