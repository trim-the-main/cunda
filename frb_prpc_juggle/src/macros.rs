/// This macro generates a struct that holds the stream sinks for the topics
/// that are defined in the protocol crate. The struct is named ProtocolStreams
/// and the fields are named after the topic types.
#[macro_export]
macro_rules! any_topic_message_enum_for_frb {
    (
        list = $list_name:ident;
        direction = TopicDirection::ToClient;
        $(omit_std = $omit:tt;)?
        | TopicTy        | MessageTy                                | Path              | $( Cfg           |)?
        | $(-)*          | $(-)*                                    | $(-)*             | $($(-)*          |)?
      $(| $tp_name:ident | $msg_ty:tt                               | $path_str:literal | $($meta:meta)? $(|)?)*
    ) => {
        // struct definitions and trait impls
        pub enum AnyTopicMessage {
            $(
                $tp_name($msg_ty),
            )*
        }

        $(
            impl From<$msg_ty> for AnyTopicMessage {
                fn from(msg: $msg_ty) -> Self {
                    AnyTopicMessage::$tp_name(msg)
                }

            }
        )*

        impl AnyTopicMessage {
            pub fn topic_name(&self) -> &'static str {
                match self {
                    $(
                        AnyTopicMessage::$tp_name(_) => $path_str,
                    )*
                }
            }
            pub fn from_key(key: ::postcard_rpc::header::VarKey, body: &[u8]) -> Option<Self> {
                $(
                    if key == ::postcard_rpc::header::VarKey::Key8(<$tp_name as ::postcard_rpc::Topic>::TOPIC_KEY) {
                        if let Ok(msg) = ::postcard::from_bytes::<$msg_ty>(body){
                            return Some(AnyTopicMessage::$tp_name(msg));
                        } else {
                            return None;
                        }
                    }
                )*
                return None;

            }
        }
    };
    ($($x:tt)*) => {}
}

#[macro_export]
macro_rules! topic_dispatcher_trait {
    (
        trait_name = $trait_name:ident;
        sink_type = $sink_type:ident;
        version = $version:literal;
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
                    ) -> ::core::result::Result<(), ::frb_prpc_juggle::client_interface::WireError>{
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
        make_sure_direction_is_to_client!($($any_token)*);
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
        make_sure_direction_is_to_client!(@parse_line $($tail)*);
    };
    (@parse_line $($x:tt)*) => {
        compile_error!("Topic direction TopicDirection::ToServer is not supported");
    };
    ($($x:tt)*) => {
        make_sure_direction_is_to_client!(@parse_line $($x)*);
    };
}

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
                    async fn [<$ep_name:snake>](&self, req: $req_ty) -> ::core::result::Result<$resp_ty, ::frb_prpc_juggle::client_interface::WireError> {
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
        endpoint_handler_trait_for_flutter!{$($any_token)*}
    }
}

#[macro_export]
macro_rules! define_topics_type {
    (
        topics_struct = $topics_struct_name:ident
        sink_type = $sink_ty:ident
        version = $version:literal
          | TopicTy        | MessageTy                                | Name              |
          | $(-)*          | $(-)*                                    | $(-)*             |
        $(| $tp_name:ty    | $msg_ty:ty                               | $name:ident       |)*

    ) => {
        #[flutter_rust_bridge::frb(opaque)]
        pub struct $topics_struct_name {
            $(
                $name: Option<$sink_ty<$msg_ty>>,
            )*
        }

        pastey::paste! {
            impl $topics_struct_name {
                pub fn new() -> Self {
                    Self {
                        $(
                            $name: None,
                        )*
                    }
                }
                pub fn version() -> &'static str {
                    $version
                }

                $(
                    pub fn [<create_ $name _stream>](&mut self, sink: $sink_ty<$msg_ty>) -> ::core::result::Result<(), ::frb_prpc_juggle::client_interface::WireError> {
                        if self.$tp_name.is_some() {
                            return Err(::frb_prpc_juggle::WireError {});
                        }
                        self.$tp_name = Some(sink);

                        Ok(())
                    }
                )*
            }
        }
    }
}
