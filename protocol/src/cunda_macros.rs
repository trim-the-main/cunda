// #[doc(hidden)]
// #[macro_export]
// macro_rules! _merge_endpoint_lists {
//     // accumulation is done
//     ( [] [$finisher_macro:path $($finisher_args:tt)*] [$($acc:tt)*] ) => {
//         $finisher_macro ! { $($finisher_args)* $($acc)* }
//     };
//     // pop the next source macro and hand it the rest of the work
//     ( [$appender_macro:path $($other_macros:tt)*] [$finisher_macro:path $($finisher_args:tt)*] [$($acc:tt)*] ) => {
//         $appender_macro ! { @append [$($other_macros)*] [$finisher_macro $($finisher_args)*] [$($acc)*] }
//     };
// }

#[doc(hidden)]
#[macro_export]
macro_rules! cunda_sys_endpoints {
    (@append [$($others:tt)*] [$($finish:tt)*] [$($acc:tt)*]) => {
        $crate::_merge_endpoint_lists! { [$($others)*] [$($finish)*]
            [
            $($acc)*
           // SYSTEM
           | GetDeviceId         | NoArg       | DeviceId               | "get_device_id"         |
           | PingEndpoint        | NoArg       | EmptyRes               | "sys_ping"              |
           | GetSysSettings      | NoArg       | SysSettings            | "get_sys_settings"      |
           | SetSysSettings      | SysSettings | EmptyRes               | "set_sys_settings"      |
           | StartSysStatsTopic  | NoArg       | EmptyRes               | "start_sys_stats_topic" |
           | StopSysStatsTopic   | NoArg       | EmptyRes               | "stop_sys_stats_topic"  |
           | GetMtu              | NoArg       | u16                    | "get_mtu"               |
           | StartSysLogsTopic   | NoArg       | EmptyRes               | "start_sys_logs_topic"  |
           | StopSysLogsTopic    | NoArg       | EmptyRes               | "stop_sys_logs_topic"   |

            ]}
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! cunda_ota_endpoints {
    (@append [$($others:tt)*] [$($finish:tt)*] [$($acc:tt)*]) => {
        $crate::_merge_endpoint_lists! { [$($others)*] [$($finish)*]
            [
            $($acc)*
           // OTA related
           | PrepareOta          | OtaMData    | OtaResult              | "prepare_ota"              |
           | TransferOtaBytes    | OtaBytes    | OtaResult              | "transfer_ota_bytes"       |
           | FinalizeOta         | NoArg       | OtaResult              | "finalize_ota"             |
           | ApproveFirmware     | NoArg       | OtaResult              | "approve_firmware_version" |
           | FactoryReset        | NoArg       | OtaResult              | "factory_reset"            |
            ]}
    };}

#[doc(hidden)]
#[macro_export]
macro_rules! _endpoints_to_trait {
    (
        trait_name = $trait_name:ident;
        $( | $ep_name:ident | $req_ty:tt $(< $($req_lt:lifetime),+ >)? | $resp_ty:tt $(< $($resp_lt:lifetime),+ >)?  | $path_str:literal | $($meta:meta)? $(|)? )*
    ) => {
        #[allow(async_fn_in_trait)]
        pub trait $trait_name: ::frb_prpc_juggle::client_interface::ClientEndpointInterface {
            pastey::paste! {
                $(
                    async fn [<$ep_name:snake>](&self, req: $req_ty) -> ::core::result::Result<$resp_ty, ::frb_prpc_juggle::client_interface::FrbPostcardRpcError> {
                        self.call_rpc_endpoint::<$ep_name>(req).await
                    }
                )*
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! _blanket_impl_for_cunda_endpoints {
    (
        $( | $ep_name:ident | $req_ty:tt $(< $($req_lt:lifetime),+ >)? | $resp_ty:tt $(< $($resp_lt:lifetime),+ >)?  | $path_str:literal | $($meta:meta)? $(|)? )*
    ) => {
        impl<T> $crate::cunda_defaults::CundaEndpoints for T
        where T: ::frb_prpc_juggle::client_interface::ClientEndpointInterface {
            pastey::paste! {
                $(
                    async fn [<$ep_name:snake>](&self, req: $req_ty) -> ::core::result::Result<$resp_ty, ::frb_prpc_juggle::client_interface::FrbPostcardRpcError> {
                        self.call_rpc_endpoint::<$ep_name>(req).await
                    }
                )*
            }
        }
    };
}

#[cfg(feature = "flutter")]
#[macro_export]
macro_rules! endpoints_with_cunda_defaults {
    (
           list = $list_name:ident;
           appl_trait_name = $trait_name:ident;
           | EndpointTy     | RequestTy                                | ResponseTy                                  | Path              | $( Cfg           |)?
           | $(-)*          | $(-)*                                    | $(-)*                                       | $(-)*             | $($(-)*          |)?
        $( | $ep_name:ident | $req_ty:tt $(< $($req_lt:lifetime),+ >)? | $resp_ty:tt $(< $($resp_lt:lifetime),+ >)?  | $path_str:literal | $($meta:meta)? $(|)? )*

    ) => {
        ::postcard_rpc::endpoints! {
           list = $list_name;
           | EndpointTy | RequestTy | ResponseTy | Path | Cfg |
           | ---------- | --------- | ---------- | ---- | --- |
        $( | $ep_name | $req_ty $(< $($req_lt),+ >)? | $resp_ty $(< $($resp_lt),+ >)?  | $path_str | $($meta |)? )*
           // SYSTEM
           | GetDeviceId         | NoArg       | DeviceId               | "get_device_id"         |
           | PingEndpoint        | NoArg       | EmptyRes               | "sys_ping"              |
           | GetSysSettings      | NoArg       | SysSettings            | "get_sys_settings"      |
           | SetSysSettings      | SysSettings | EmptyRes               | "set_sys_settings"      |
           | StartSysStatsTopic  | NoArg       | EmptyRes               | "start_sys_stats_topic" |
           | StopSysStatsTopic   | NoArg       | EmptyRes               | "stop_sys_stats_topic"  |
           | GetMtu              | NoArg       | u16                    | "get_mtu"               |
           | StartSysLogsTopic   | NoArg       | EmptyRes               | "start_sys_logs_topic"  |
           | StopSysLogsTopic    | NoArg       | EmptyRes               | "stop_sys_logs_topic"   |

           // OTA related
           | PrepareOta          | OtaMData    | OtaResult              | "prepare_ota"              |
           | TransferOtaBytes    | OtaBytes    | OtaResult              | "transfer_ota_bytes"       |
           | FinalizeOta         | NoArg       | OtaResult              | "finalize_ota"             |
           | ApproveFirmware     | NoArg       | OtaResult              | "approve_firmware_version" |
           | FactoryReset        | NoArg       | OtaResult              | "factory_reset"            |
        }

        $crate::_endpoints_to_trait! {
            trait_name = $trait_name;
            $( | $ep_name | $req_ty $(< $($req_lt),+ >)? | $resp_ty $(< $($resp_lt),+ >)?  | $path_str | $($meta |)? )*
        }

        $crate::_endpoints_to_trait! {
            trait_name = CundaEndpoints;
           // SYSTEM
           | GetDeviceId         | NoArg       | DeviceId               | "get_device_id"         |
           | PingEndpoint        | NoArg       | EmptyRes               | "sys_ping"              |
           | GetSysSettings      | NoArg       | SysSettings            | "get_sys_settings"      |
           | SetSysSettings      | SysSettings | EmptyRes               | "set_sys_settings"      |
           | StartSysStatsTopic  | NoArg       | EmptyRes               | "start_sys_stats_topic" |
           | StopSysStatsTopic   | NoArg       | EmptyRes               | "stop_sys_stats_topic"  |
           | GetMtu              | NoArg       | u16                    | "get_mtu"               |
           | StartSysLogsTopic   | NoArg       | EmptyRes               | "start_sys_logs_topic"  |
           | StopSysLogsTopic    | NoArg       | EmptyRes               | "stop_sys_logs_topic"   |

           // OTA related
           | PrepareOta          | OtaMData    | OtaResult              | "prepare_ota"              |
           | TransferOtaBytes    | OtaBytes    | OtaResult              | "transfer_ota_bytes"       |
           | FinalizeOta         | NoArg       | OtaResult              | "finalize_ota"             |
           | ApproveFirmware     | NoArg       | OtaResult              | "approve_firmware_version" |
           | FactoryReset        | NoArg       | OtaResult              | "factory_reset"            |
        }
    };
}

#[cfg(not(feature = "flutter"))]
#[macro_export]
macro_rules! endpoints_with_cunda_defaults {
    (
           list = $list_name:ident;
           appl_trait_name = $trait_name:ident;
           | EndpointTy     | RequestTy                                | ResponseTy                                  | Path              | $( Cfg           |)?
           | $(-)*          | $(-)*                                    | $(-)*                                       | $(-)*             | $($(-)*          |)?
        $( | $ep_name:ident | $req_ty:tt $(< $($req_lt:lifetime),+ >)? | $resp_ty:tt $(< $($resp_lt:lifetime),+ >)?  | $path_str:literal | $($meta:meta)? $(|)? )*
    ) => {
        ::postcard_rpc::endpoints! {
            list = $list_name;
           | EndpointTy | RequestTy | ResponseTy | Path | Cfg |
           | ---------- | --------- | ---------- | ---- | --- |
        $( | $ep_name | $req_ty $(< $($req_lt),+ >)? | $resp_ty $(< $($resp_lt),+ >)?  | $path_str | $($meta |)? )*
           // SYSTEM
           | GetDeviceId         | NoArg       | DeviceId               | "get_device_id"         |
           | PingEndpoint        | NoArg       | EmptyRes               | "sys_ping"              |
           | GetSysSettings      | NoArg       | SysSettings            | "get_sys_settings"      |
           | SetSysSettings      | SysSettings | EmptyRes               | "set_sys_settings"      |
           | StartSysStatsTopic  | NoArg       | EmptyRes               | "start_sys_stats_topic" |
           | StopSysStatsTopic   | NoArg       | EmptyRes               | "stop_sys_stats_topic"  |
           | GetMtu              | NoArg       | u16                    | "get_mtu"               |
           | StartSysLogsTopic   | NoArg       | EmptyRes               | "start_sys_logs_topic"  |
           | StopSysLogsTopic    | NoArg       | EmptyRes               | "stop_sys_logs_topic"   |

           // OTA related
           | PrepareOta          | OtaMData    | OtaResult              | "prepare_ota"              |
           | TransferOtaBytes    | OtaBytes    | OtaResult              | "transfer_ota_bytes"       |
           | FinalizeOta         | NoArg       | OtaResult              | "finalize_ota"             |
           | ApproveFirmware     | NoArg       | OtaResult              | "approve_firmware_version" |
           | FactoryReset        | NoArg       | OtaResult              | "factory_reset"            |
        }
    };
}

#[cfg(not(feature = "flutter"))]
#[macro_export]
macro_rules! topics_with_cunda_defaults {
    (
        list = $list_name:ident;
        | TopicTy        | MessageTy                                | Path              | $( Cfg           |)?
        | $(-)*          | $(-)*                                    | $(-)*             | $($(-)*          |)?
      $(| $tp_name:ident | $msg_ty:tt $(< $($msg_lt:lifetime),+ >)? | $path_str:literal | $($meta:meta)? $(|)?)*
    ) => {
        ::postcard_rpc::topics! {
            list = $list_name;
            direction = ::postcard_rpc::TopicDirection::ToClient;
            | TopicTy       | MessageTy     | Path               | Cfg |
            | -------       | ---------     | ----               | --- |
      $(| $tp_name | $msg_ty $(< $($msg_lt),+ >)? | $path_str | $($meta |)? )*
            // System topics
            | SysStatsTopic | SysStats      | "sys_stats_stream" |     |
            | SysLogsTopic  | LogMessage    | "sys_logs_stream"  |     |
        }
    };
}

#[cfg(feature = "flutter")]
#[macro_export]
macro_rules! topics_with_cunda_defaults {
    (
        list = $list_name:ident;
        | TopicTy        | MessageTy                                | Path              | $( Cfg           |)?
        | $(-)*          | $(-)*                                    | $(-)*             | $($(-)*          |)?
      $(| $tp_name:ident | $msg_ty:tt $(< $($msg_lt:lifetime),+ >)? | $path_str:literal | $($meta:meta)? $(|)?)*
    ) => {
        ::frb_prpc_juggle::topics_for_flutter! {
            list = $list_name;
            direction = ::postcard_rpc::TopicDirection::ToClient;
            | TopicTy       | MessageTy     | Path               | Cfg |
            | -------       | ---------     | ----               | --- |
      $(| $tp_name | $msg_ty $(< $($msg_lt),+ >)? | $path_str | $($meta |)? )*
            // System topics
            | SysStatsTopic | SysStats      | "sys_stats_stream" |     |
            | SysLogsTopic  | LogMessage    | "sys_logs_stream"  |     |
        }
    };
}
