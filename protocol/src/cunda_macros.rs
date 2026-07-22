#[macro_export]
macro_rules! endpoints_for_cunda {
    (
           list = $list_name:ident;
           trait_name = $trait_name:ident;
           | EndpointTy     | RequestTy                                | ResponseTy                                  | Path              | $( Cfg           |)?
           | $(-)*          | $(-)*                                    | $(-)*                                       | $(-)*             | $($(-)*          |)?
        $( | $ep_name:ident | $req_ty:tt $(< $($req_lt:lifetime),+ >)? | $resp_ty:tt $(< $($resp_lt:lifetime),+ >)?  | $path_str:literal | $($meta:meta)? $(|)? )*

    ) => {
        ::postcard_rpc::endpoints! {
           list = $list_name;
           | EndpointTy | RequestTy | ResponseTy | Path | Cfg |
           | ---------- | --------- | ---------- | ---- | --- |
        $( | $ep_name | $req_ty $(< $($req_lt),+ >)? | $resp_ty $(< $($resp_lt),+ >)?  | $path_str | $($meta |)? )*
        }

        // define the trait
        $crate::_endpoints_to_trait! {
            trait_name = $trait_name;
            $( | $ep_name | $req_ty $(< $($req_lt),+ >)? | $resp_ty $(< $($resp_lt),+ >)?  | $path_str | $($meta |)? )*
        }
    };
}

#[macro_export]
macro_rules! topics_for_cunda {
    (
        list = $list_name:ident;
        trait_name = $trait_name:ident;
        path = $p:path;
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
        }

        $crate::_create_topic_definition_macro! {
            trait_name = $trait_name;
            types_path = $p;
            $(| $tp_name | $msg_ty $(< $($msg_lt),+ >)? | $path_str | $($meta |)? )*
        }
    };
}

// Macro that generates another macro...
#[cfg(feature = "flutter")]
#[doc(hidden)]
#[macro_export]
macro_rules! _create_topic_definition_macro {
    (
        trait_name = $trait_name:ident;
        types_path = $p:path;
        $(| $tp_name:ident | $msg_ty:tt $(< $($msg_lt:lifetime),+ >)? | $path_str:literal | $($meta:meta)? $(|)?)*
    ) => {
        _create_topic_definition_macro!(@internal
            trait_name = $trait_name;
            types_path = $p;
            ($);
            $(| $tp_name | $msg_ty $(< $($msg_lt),+ >)? | $($meta)? |)*
        );
    };

    // The internal rule captures the literal '$' as $d
    (@internal
        trait_name = $trait_name:ident;
        types_path = $p:path;
        ($d:tt);
        $(| $tp_name:ty | $msg_ty:ty | $($meta:meta)? $(|)?)*
    ) => {
        ::pastey::paste! {
            #[doc(hidden)]
            #[macro_export]
            macro_rules! [<_define_topic_trait_imp_ $trait_name>] {
                ($trait_name with $d sink_type:ident) => {
                    #[::flutter_rust_bridge::frb]
                    pub trait $trait_name : ::frb_prpc_juggle::client_interface::ClientTopicInterface {
                        fn [<__force_translation_of_ $trait_name:snake>]() {}
                        $(
                            $(#[$meta])?
                            async fn [<create_ $tp_name:snake _stream>](
                                &self,
                                sink: $d sink_type<$d crate ::$p::$msg_ty>
                            ) -> ::core::result::Result<(), ::frb_prpc_juggle::client_interface::FrbPostcardRpcError> {
                                self.subscribe::< $d crate ::$p::$tp_name >(::std::boxed::Box::new(sink)).await
                            }
                        )*
                    }
                };
            }
            pub use [<_define_topic_trait_imp_ $trait_name>] as define_topic_trait;
        }
    };
}

#[cfg(feature = "flutter")]
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
#[cfg(not(feature = "flutter"))]
#[doc(hidden)]
#[macro_export]
macro_rules! _endpoints_to_trait {
    ($($any_token:tt)*) => {};
}

#[cfg(not(feature = "flutter"))]
#[doc(hidden)]
#[macro_export]
macro_rules! _create_topic_definition_macro {
    ($($any_token:tt)*) => {};
}
