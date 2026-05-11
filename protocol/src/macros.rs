/// Define the device id metadata for mayna package manager
///
/// Use [`mayna_meta_value!`] to read individual fields.
#[macro_export]
macro_rules! define_mayna_metadata {
    (device_type: $dt:expr, firmware_version: $fv:expr, protocol_version: $pv:expr $(,)?) => {
        const __MAYNA_META_DEVICE_TYPE: &str = $dt;
        const __MAYNA_META_FIRMWARE_VERSION: &str = $fv;
        const __MAYNA_META_PROTOCOL_VERSION: u32 = $pv;

        const __MAYNA_META_JSON: &str = ::const_format::concatcp!(
            r#"{"device_type":""#,
            $dt,
            r#"","firmware_version":""#,
            $fv,
            r#"","protocol_version":"#,
            $pv,
            "}"
        );

        #[unsafe(link_section = ".mayna_meta")]
        #[used]
        static __MAYNA_META: [u8; __MAYNA_META_JSON.len()] = {
            let bytes = __MAYNA_META_JSON.as_bytes();
            let mut arr = [0u8; __MAYNA_META_JSON.len()];
            let mut i = 0;
            while i < arr.len() {
                arr[i] = bytes[i];
                i += 1;
            }
            arr
        };
    };
}

/// Read a field from the metadata defined by [`mayna_meta!`].
///
/// ```ignore
/// mayna_meta_value!(device_type)        // -> &str
/// mayna_meta_value!(firmware_version)   // -> &str
/// mayna_meta_value!(protocol_version)   // -> u32
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
}
