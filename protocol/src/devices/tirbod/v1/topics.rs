pub const TOPICS: postcard_rpc::TopicMap = crate::merge_topic_lists!(
    APPL_TOPICS,
    crate::cunda_common::v1::topics::sys::CUNDA_SYS_TOPICS
);

topics_for_cunda! {
    list = APPL_TOPICS;
    trait_name = TirbodTopics;
    full_mod_path_for_msg_types = devices::tirbod::v1::topics;
    | TopicTy            | MessageTy             | Path                   | Cfg |
    | -------            | ---------             | ----                   | --- |
}

#[cfg(not(feature = "flutter"))]
postcard_rpc::topics! {
   list = EMPTY_TOPICS;
   direction = postcard_rpc::TopicDirection::ToServer;
   | TopicTy        | MessageTy     | Path              |
   | -------        | ---------     | ----              |
}
