// Do not remove any item, this list is append only for version compatibility
// reasons. A newer firmware should be able to deserialize a u32 to this type.
// An older firmware should also be able to deserialize a u32, mapping any new
// variant to UnknownType variant.
// Implements sequential_storage::Key
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum DType {
    SysConfig = 0,
    ApplicationConfig,
    PanicMessage,

    // Any u32 bigger than or equal to this is unknown to us. We may find such
    // values in case of a version downgrade. This is the only field whose value
    // is okay to move. It has to be at the bottom of the list.
    UnknownType,
}

pub mod app_config;
pub mod sys_config;
