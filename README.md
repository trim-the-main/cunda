# Cunda
This is a sample ESP32 to mobile communication over BLE application
using [postcard-rpc](https://github.com/jamesmunns/postcard-rpc).
On the mobile side it uses [flutter](https://flutter.dev/) and
[flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge). The goal
is to be able to define the communication protocol in rust at one place and be
able to call remote functions on esp32 from the flutter app using native dart
functions.

Beware, there's a lot of code generation to make all these translations work. It
comes with limitations. But if you can work around the limitations, it is nice
to be able to call an async function from your mobile app and get the result
or the side effect from the micro-controller.

In addition to the communication, the firmware and the driving mobile
application implements basic persistent storage on the flash. I think almost
every project that would need this communication needs a way of changing the
firmware behavior from the mobile app (like changing the BLE advertisement
name).

I would like to use this project as a starting point for running more tasks
on the ESP32 chip and I would like to be able to update the firmware without
the USB connection. That's why there is an OTA module.

## Structure

|  Folder            | Description                                             |
|--------------------|---------------------------------------------------------|
| `firmware/`        | ESP32 no_std Rust firmware — BLE peripheral, RPC server |
| `protocol/`        | Shared Rust crate defining endpoints, topics etc.       |
| `frb_prpc_juggle/` | Glue code for flutter_rust_bridge and postcard_rpc      |
| `cunda_flutter/`   | Flutter mobile app — BLE client, UI                     |

