/// Define the device id metadata for mayna package manager
///
/// Use [`get_mayna_metadata!`] to read individual fields.
#[macro_export]
macro_rules! define_mayna_metadata {
    (device_type: $dt:expr, protocol_version: $pv:expr $(,)?) => {
        const __MAYNA_META_DEVICE_TYPE: &str = $dt;
        const __MAYNA_META_FIRMWARE_VERSION: &str = env!("CARGO_PKG_VERSION");
        const __MAYNA_META_PROTOCOL_VERSION: u32 = $pv;
        const __MAYNA_META_GIT_REV: Option<$crate::types::GitRevSha> =
            $crate::types::GitRevSha::from_hex_str(env!("GIT_HASH"));

        const _: () = {
            const JSON_BASE: &str = ::const_format::concatcp!(
                r#"{"device_type":""#,
                $dt,
                r#"","firmware_version":""#,
                env!("CARGO_PKG_VERSION"),
                r#"","protocol_version":"#,
                $pv
            );

            const JSON: &str = if __MAYNA_META_GIT_REV.is_none() {
                ::const_format::concatcp!(JSON_BASE, "}")
            } else {
                ::const_format::concatcp!(JSON_BASE, r#","git_hash":""#, env!("GIT_HASH"), r#""}"#)
            };

            #[unsafe(link_section = ".mayna_meta")]
            #[used]
            static __MAYNA_META: [u8; JSON.len()] = {
                let bytes = JSON.as_bytes();
                let mut arr = [0u8; JSON.len()];
                let mut i = 0;
                while i < arr.len() {
                    arr[i] = bytes[i];
                    i += 1;
                }
                arr
            };
        };
    };
}

/// Read a field from the metadata defined by [`define_mayna_metadata!`].
///
/// ```ignore
/// get_mayna_metadata!(device_type)        // -> &str
/// get_mayna_metadata!(firmware_version)   // -> &str
/// get_mayna_metadata!(protocol_version)   // -> u32
/// get_mayna_metadata!(git_hash)           // -> Option<GitRevSha>
/// ```
#[macro_export]
macro_rules! get_mayna_metadata {
    (device_type) => {
        __MAYNA_META_DEVICE_TYPE
    };
    (firmware_version) => {
        __MAYNA_META_FIRMWARE_VERSION
    };
    (protocol_version) => {
        __MAYNA_META_PROTOCOL_VERSION
    };
    (git_hash) => {
        __MAYNA_META_GIT_REV
    };
}

/// Construct a [`DeviceId`](crate::types::DeviceId) from mayna metadata.
///
/// Reads `device_type`, `firmware_version`, `protocol_version`, and `git_hash`
/// from the constants defined by [`define_mayna_metadata!`]. Only `hardware_revision`
/// and `serial_number` must be supplied (read from device flash).
#[macro_export]
macro_rules! new_device_id_from_metadata {
    (hardware_revision: $hardware_revision:expr, serial_number: $serial_number:expr $(,)?) => {
        $crate::types::DeviceId::new(
            $crate::get_mayna_metadata!(device_type),
            $hardware_revision,
            $serial_number,
            $crate::get_mayna_metadata!(firmware_version),
            $crate::get_mayna_metadata!(protocol_version),
            $crate::get_mayna_metadata!(git_hash),
        )
    };
}
