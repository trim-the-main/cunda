// The important list, firmware implements this:
pub const ENDPOINT_LIST: postcard_rpc::EndpointMap = crate::merge_endpoint_lists!(
    APPL_ENDPOINTS,
    crate::cunda_common::CUNDA_DEVICE_ENDPOINT,
    crate::cunda_common::v1::endpoints::CUNDA_SYS_ENDPOINTS,
);

use super::types::*;

endpoints_for_cunda! {
    list = APPL_ENDPOINTS;
    trait_name = NoktaEndpoints;
    | EndpointTy              | RequestTy         | ResponseTy   | Path                        | Cfg |
    | ----------              | ---------         | ----------   | ----                        | --- |
    | GetApplSettings         | NoArg             | ApplSettings |"get_appl_settings"          |     |
    | SetApplSettings         | ApplSettings      | EmptyRes     |"set_appl_settings"          |     |
}
