#[macro_export]
macro_rules! topic_dispatcher_trait {
    (
        trait_name = $trait_name:ident;
        sink_type = $sink_type:ident;
        | TopicTy        | MessageTy                                $(|)?
        | $(-)*          | $(-)*                                    $(|)?
      $(| $tp_name:ty    | $msg_ty:tt                               $(|)?)*
    ) => {
        pastey::paste! {
            pub trait $trait_name : ::frb_prpc_juggle::client_interface::ClientTopicInterface{
                $(
                    async fn [<create_ $tp_name:snake _stream>](
                        &self,
                        sink: $sink_type<$msg_ty>
                    ) -> ::core::result::Result<(), ::frb_prpc_juggle::client_interface::FrbPostcardRpcError>{
                        self.subscribe::<$tp_name>(::std::boxed::Box::new(sink)).await
                    }
                )*
            }
        }
    };
}

// pass-through for the postcard_rpc::topics macro
// just disable the topics to the server
#[macro_export]
macro_rules! topics_for_flutter {
    ($($any_token:tt)*) => {
        ::postcard_rpc::topics!($($any_token)*);
        $crate::make_sure_direction_is_to_client!($($any_token)*);
    };
}

#[macro_export]
macro_rules! make_sure_direction_is_to_client {
    (@parse_line direction = $dir:expr; $($tail:tt)*) => {
        const _: () = {
            const _IS_GOOD_DIRECTION: bool = matches!($dir, ::postcard_rpc::TopicDirection::ToClient);
            const _: () = assert!(_IS_GOOD_DIRECTION, "Only TopicDirection::ToClient is supported");
        };
    };
    (@parse_line $smth:ident = $smth_else:tt; $($tail:tt)*) => {
        $crate::make_sure_direction_is_to_client!(@parse_line $($tail)*);
    };
    (@parse_line $($x:tt)*) => {
        compile_error!("Topic direction TopicDirection::ToServer is not supported");
    };
    ($($x:tt)*) => {
        $crate::make_sure_direction_is_to_client!(@parse_line $($x)*);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! endpoint_handler_trait_for_flutter {
    (
           list = $list_name:ident;
           $(omit_std = $omit:tt;)?
           | EndpointTy     | RequestTy                                | ResponseTy                                  | Path              | $( Cfg           |)?
           | $(-)*          | $(-)*                                    | $(-)*                                       | $(-)*             | $($(-)*          |)?
        $( | $ep_name:ident | $req_ty:tt $(< $($req_lt:lifetime),+ >)? | $resp_ty:tt $(< $($resp_lt:lifetime),+ >)?  | $path_str:literal | $($meta:meta)? $(|)? )*
    ) => {
        pub trait EndpointDispatcher : ::frb_prpc_juggle::client_interface::ClientEndpointInterface {
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

// pass-through for the postcard_rpc::endpoints macro
#[macro_export]
macro_rules! endpoints_for_flutter {
    ($($any_token:tt)*) => {
        postcard_rpc::endpoints!{$($any_token)*}
        $crate::endpoint_handler_trait_for_flutter!{$($any_token)*}
    }
}
