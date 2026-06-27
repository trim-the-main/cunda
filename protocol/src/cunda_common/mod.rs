/// Common cunda functionality. We are trying to handle different devices and
/// different versions but there always has to be one endpoint that is not
/// versioned. Every `CundaDevice` has to respond to it the same. After that
/// we can navigate to the correct
use crate::{type_helpers::hex_nibble, types::NoArg};
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

pub mod v1;

endpoints_for_cunda! {
    list = CUNDA_DEVICE_ENDPOINT;
    trait_name = CundaDevice;
    | EndpointTy          | RequestTy   | ResponseTy             | Path                    |
    | ----------          | ---------   | ----------             | ----                    |
    // All devices must respond to this endpoint. This is not versioned, never will be.
    | GetDeviceId         | NoArg       | DeviceId               | "get_device_id"         |
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Default)]
pub struct DeviceId {
    /// The internal name of the device. The device respond to this is hard coded
    /// On the flutter side we match this with the protocol version to create the
    /// the correct rpc client, and go to the correct page.
    pub device_type: ProtocolStringType!(capacity: 16),

    /// What protocol does this firmware speak? It is less of a version more of an
    /// identifier. A newer version may be speaking an older protocol. This, along
    /// with the device type will give us enough information in what client we will
    /// initialize. Hard coded in the firmware using mayna macros.
    pub protocol_version: u32,

    /// The framework doesn't care much about the hardware or firmware version
    /// except for when it's time to do a firmware upgrade (to determine what version
    /// is compatible with what other). When the framework constructs the RPC
    /// client it only looks at the `device_type` and `protocol_version` and with those
    /// it knows how to serialize and deserialize everything. These versions can be used
    /// by the user of the rpc client though. A newer firmware may change the outcome of
    /// an endpoint call or the frequency of a topic message. These does not change the
    /// protocol version but the firmware version. Same for the hardware revision, some
    /// capabilities may require a newer hardware revision, or some hardware bugs may
    /// require a change. Instead of trying to squeze all this version info in a version
    /// string with major.protocol.minor kind of setup, I chose to have different fields.
    /// Thanks to this, the hardware revision can be hardcoded to the hardware flash
    /// (along with the serial number) and firmware is hardcoded to the firmware binary
    /// that changes between the upgrades.
    pub hardware_revision: u32,
    pub firmware_version: VersionString,

    /// Device serial number. In case a batch is found different than another, this can
    /// be used to trigger different behavior in the client.
    pub serial_number: u32,

    /// Actual commit hash. This is the same to the `firmware_version` what `serial_number`
    /// is to the `hardware_revision`. Automatically set at compile time if the working tree
    /// is clean (the compiled code is the same as HEAD), otherwise it will be None. This
    /// also eases development. If we detect a mayna package with the same `firmware_version`
    /// but a different `git_hash` (very common in development where we don't bump up the
    /// firmware version everytime we make an edit), we offer it as a firmware upgrade.
    pub git_hash: Option<GitRevSha>,
}

pub type VersionString = ProtocolStringType!(capacity: 16);

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GitRevSha(pub [u8; 20]);

impl GitRevSha {
    pub const fn from_hex_str(hex: &str) -> Option<Self> {
        let bytes = hex.as_bytes();
        if bytes.len() != 40 {
            return None;
        }
        // "git hash must be 40 hex characters");

        let mut result = [0u8; 20];
        let mut i = 0;
        while i < 20 {
            result[i] = hex_nibble(bytes[i * 2]) * 16 + hex_nibble(bytes[i * 2 + 1]);
            i += 1;
        }
        Some(Self(result))
    }
}

impl core::fmt::Display for GitRevSha {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

impl DeviceId {
    pub fn new(
        device_type: &str,
        protocol_version: u32,
        hardware_revision: u32,
        firmware_version: &str,
        serial_number: u32,
        git_hash: Option<GitRevSha>,
    ) -> Self {
        use core::str::FromStr;
        Self {
            device_type: FromStr::from_str(device_type).unwrap(),
            protocol_version,
            hardware_revision,
            firmware_version: FromStr::from_str(firmware_version).unwrap(),
            serial_number,
            git_hash,
        }
    }
}
