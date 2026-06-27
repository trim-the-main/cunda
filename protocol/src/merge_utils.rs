/// This file defines two macros: `merge_endpoint_lists`, `merge_topic_lists`.
/// This allows us to define common set of endpoints and topics in their own
/// lists (and traits, hence abstract classes in flutter side). For the firmware
/// we export the merged list so that the firmware can calculate the minimum key
/// length required.
// TODO: Example
use postcard_rpc::uniques::merge_nty_lists;

const fn str_eq(a: &str, b: &str) -> bool {
    let mut i = 0;
    if a.len() != b.len() {
        return false;
    }
    let a_by = a.as_bytes();
    let b_by = b.as_bytes();
    while i < a.len() {
        if a_by[i] != b_by[i] {
            return false;
        }
        i += 1;
    }
    true
}

pub(crate) trait ConstTimeListMerger {
    type Item: Copy;
}

macro_rules! define_const_time_list_merger {
    (
        out_ty = $out_t:ident;
        item_ty = $tyname:ty;
        init_val = $init_val:expr;
        eq_f = $eq_f:path;
    ) => {
        pub(crate) struct $out_t;
        impl ConstTimeListMerger for $out_t {
            type Item = $tyname;
        }
        impl $out_t {
            const INIT_VAL: $tyname = $init_val;

            const fn eq_f(a: &$tyname, b: &$tyname) -> bool {
                $eq_f(a, b)
            }

            const fn contains(list: &[$tyname], item: &$tyname) -> bool {
                let mut i = 0;
                while i < list.len() {
                    if Self::eq_f(&list[i], &item) {
                        return true;
                    }
                    i += 1;
                }
                false
            }

            pub(crate) const fn merged_len(a: &[$tyname], b: &[$tyname]) -> usize {
                let mut count = a.len();
                let mut i = 0;
                while i < b.len() {
                    if !Self::contains(a, &b[i]) {
                        count += 1;
                    }
                    i += 1;
                }
                count
            }

            pub(crate) const fn dedup_merge<const N: usize>(
                a: &[$tyname],
                b: &[$tyname],
            ) -> [$tyname; N] {
                let mut out = [Self::INIT_VAL; N];
                let mut len = 0;

                let mut i = 0;
                while i < a.len() {
                    out[len] = a[i];
                    len += 1;
                    i += 1;
                }

                let mut i = 0;
                while i < b.len() {
                    let v = &b[i];
                    if !Self::contains(a, v) {
                        out[len] = *v;
                        len += 1;
                    }
                    i += 1;
                }

                assert!(len == N, "N does not match the deduplicated length");
                out
            }
        }
    };
}

const fn cmp_nty(
    a: &&'static postcard_schema::schema::NamedType,
    b: &&'static postcard_schema::schema::NamedType,
) -> bool {
    let uniques = merge_nty_lists::<2>(&[&[a], &[b]]).1;
    assert!(uniques == 1 || uniques == 2);
    uniques == 1
}

const fn cmp_ep(
    a: &(&'static str, postcard_rpc::Key, postcard_rpc::Key),
    b: &(&'static str, postcard_rpc::Key, postcard_rpc::Key),
) -> bool {
    str_eq(a.0, b.0) && a.1.const_cmp(&b.1) && a.2.const_cmp(&b.2)
}

const fn cmp_tp(
    a: &(&'static str, postcard_rpc::Key),
    b: &(&'static str, postcard_rpc::Key),
) -> bool {
    str_eq(a.0, b.0) && a.1.const_cmp(&b.1)
}

define_const_time_list_merger! {
    out_ty = NamedTypeListMerger;
    item_ty = &'static ::postcard_schema::schema::NamedType;
    init_val = &::postcard_schema::schema::NamedType {
        name: "",
        ty: &postcard_schema::schema::DataModelType::Unit,
    };
    eq_f = cmp_nty;
}

define_const_time_list_merger! {
    out_ty = EndpointListMerger;
    item_ty = (&'static str, postcard_rpc::Key, postcard_rpc::Key);
    init_val = unsafe { ("", postcard_rpc::Key::from_bytes([0u8; 8]), postcard_rpc::Key::from_bytes([0u8; 8])) };
    eq_f = cmp_ep;
}

define_const_time_list_merger! {
    out_ty = TopicListMerger;
    item_ty = (&'static str, postcard_rpc::Key);
    init_val = unsafe { ("", postcard_rpc::Key::from_bytes([0u8; 8])) };
    eq_f = cmp_tp;
}

#[doc(hidden)]
#[macro_export]
macro_rules! _define_merged_const_list {
    ($out:ident, $merger:ty, $l1:expr, $l2:expr) => {
        const $out: &'static [<$merger as $crate::merge_utils::ConstTimeListMerger>::Item] =
            { <$merger>::dedup_merge::<{ <$merger>::merged_len($l1, $l2) }>($l1, $l2).as_slice() };
    };
}

#[macro_export]
macro_rules! merge_endpoint_lists {
    ($l1:expr, $l2:expr $(,)?) => {
        const {
            $crate::_define_merged_const_list!(
                TYPES_MERGED,
                $crate::merge_utils::NamedTypeListMerger,
                $l1.types,
                $l2.types
            );

            $crate::_define_merged_const_list!(
                ENDPOINTS_MERGED,
                $crate::merge_utils::EndpointListMerger,
                $l1.endpoints,
                $l2.endpoints
            );

            ::postcard_rpc::EndpointMap {
                types: TYPES_MERGED,
                endpoints: ENDPOINTS_MERGED,
            }
        }
    };
    ($l1:expr, $l2:expr, $l3:expr $(,)?) => {
        const {
            const __FIRST_2: ::postcard_rpc::EndpointMap = $crate::merge_endpoint_lists!($l1, $l2);
            $crate::merge_endpoint_lists!(__FIRST_2, $l3)
        }
    };
    ($l1:expr, $l2:expr, $l3:expr, $l4:expr $(,)?) => {
        const {
            const __FIRST_3: ::postcard_rpc::EndpointMap =
                $crate::merge_endpoint_lists!($l1, $l2, $l3);
            $crate::merge_endpoint_lists!(__FIRST_3, $l4)
        }
    };
    ($l1:expr, $l2:expr, $l3:expr, $l4:expr , $l5:expr$(,)?) => {
        const {
            const __FIRST_4: ::postcard_rpc::EndpointMap =
                $crate::merge_endpoint_lists!($l1, $l2, $l3, $l4);
            $crate::merge_endpoint_lists!(__FIRST_4, $l5)
        }
    };
}

#[macro_export]
macro_rules! merge_topic_lists {
    ($l1:expr, $l2:expr $(,)?) => {
        const {
            const TP_DIRECTION_SAME: bool = match ($l1.direction, $l2.direction) {
                (
                    ::postcard_rpc::TopicDirection::ToServer,
                    ::postcard_rpc::TopicDirection::ToServer,
                ) => true,
                (
                    ::postcard_rpc::TopicDirection::ToClient,
                    ::postcard_rpc::TopicDirection::ToClient,
                ) => true,
                _ => false,
            };

            assert!(
                TP_DIRECTION_SAME,
                "Cannot merge topic lists if the directions are not the same."
            );
            const TP_DIRECTION: ::postcard_rpc::TopicDirection = $l1.direction;

            $crate::_define_merged_const_list!(
                TYPES_MERGED,
                $crate::merge_utils::NamedTypeListMerger,
                $l1.types,
                $l2.types
            );

            $crate::_define_merged_const_list!(
                TOPICS_MERGED,
                $crate::merge_utils::TopicListMerger,
                $l1.topics,
                $l2.topics
            );

            ::postcard_rpc::TopicMap {
                direction: TP_DIRECTION,
                types: TYPES_MERGED,
                topics: TOPICS_MERGED,
            }
        }
    };
    ($l1:expr, $l2:expr, $l3:expr $(,)?) => {
        const {
            const __FIRST_2: ::postcard_rpc::TopicMap = $crate::merge_endpoint_lists!($l1, $l2);
            $crate::merge_topic_lists!(__FIRST_2, $l3)
        }
    };
    ($l1:expr, $l2:expr, $l3:expr, $l4:expr $(,)?) => {
        const {
            const __FIRST_3: ::postcard_rpc::TopicMap = $crate::merge_topic_lists!($l1, $l2, $l3);
            $crate::merge_topic_lists!(__FIRST_3, $l4)
        }
    };
    ($l1:expr, $l2:expr, $l3:expr, $l4:expr , $l5:expr$(,)?) => {
        const {
            const __FIRST_4: ::postcard_rpc::TopicMap =
                $crate::merge_topic_lists!($l1, $l2, $l3, $l4);
            $crate::merge_topic_lists!(__FIRST_4, $l5)
        }
    };
}
///

#[cfg(test)]
mod test {
    use postcard_schema::Schema;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Schema)]
    pub struct Arg1;
    #[derive(Serialize, Deserialize, Schema)]
    pub struct Arg2(u8);
    #[derive(Serialize, Deserialize, Schema)]
    pub struct Arg3(Arg1);

    #[derive(Serialize, Deserialize, Schema)]
    pub struct Res1;
    #[derive(Serialize, Deserialize, Schema)]
    pub struct Res2(f32);
    #[derive(Serialize, Deserialize, Schema)]
    pub struct Res3 {
        name: [u8; 12],
        val: u64,
    }

    ::postcard_rpc::endpoints! {
        list = ENDPOINT_LIST_1;
        | EndpointTy | RequestTy | ResponseTy | Path        |
        | ---------- | --------- | ---------- | ----        |
        | Endpoint1  | Arg1      | Res1       | "endpoint1" |
        | Endpoint2  | Arg2      | Res2       | "endpoint2" |
    }
    ::postcard_rpc::endpoints! {
        list = ENDPOINT_LIST_2;
        | EndpointTy | RequestTy | ResponseTy | Path        |
        | ---------- | --------- | ---------- | ----        |
        | DupeEndpoi | Arg1      | Res1       | "endpoint1" |
        | Endpoint3  | Arg3      | Res3       | "endpoint3" |
    }

    ::postcard_rpc::endpoints! {
        list = ENDPOINT_LIST_3;
        | EndpointTy | RequestTy | ResponseTy | Path        |
        | ---------- | --------- | ---------- | ----        |
        | Endpoint4  | Arg3      | Res3       | "endpoint4" |
    }

    #[test]
    fn merged_endpoints() {
        const ENDPOINT_LIST: postcard_rpc::EndpointMap =
            merge_endpoint_lists!(ENDPOINT_LIST_1, ENDPOINT_LIST_2);
        assert_eq!(ENDPOINT_LIST_1.types.len(), 5);
        println!("{:?}", ENDPOINT_LIST_1.types);
        println!("{:?}", ENDPOINT_LIST_2.types);
        assert_eq!(ENDPOINT_LIST_2.types.len(), 6);
        println!("{:?}", ENDPOINT_LIST.types);
        assert_eq!(ENDPOINT_LIST.types.len(), 8);
        assert_eq!(ENDPOINT_LIST_1.endpoints.len(), 4);
        assert_eq!(ENDPOINT_LIST_2.endpoints.len(), 4);
        assert_eq!(ENDPOINT_LIST.endpoints.len(), 5);
    }

    #[test]
    fn merge_multiple_lists() {
        ::postcard_rpc::endpoints! {
            list = ENDPOINT_LIST_4;
            | EndpointTy | RequestTy | ResponseTy | Path        |
            | ---------- | --------- | ---------- | ----        |
            | Endpoint4  | Arg3      | Res3       | "endpoint4" |
        }
        const ENDPOINT_LIST_MERGED_3: postcard_rpc::EndpointMap =
            merge_endpoint_lists!(ENDPOINT_LIST_1, ENDPOINT_LIST_2, ENDPOINT_LIST_3);
        assert_eq!(ENDPOINT_LIST_MERGED_3.endpoints.len(), 6);

        const ENDPOINT_LIST_MERGED_4: postcard_rpc::EndpointMap = merge_endpoint_lists!(
            ENDPOINT_LIST_1,
            ENDPOINT_LIST_2,
            ENDPOINT_LIST_3,
            ENDPOINT_LIST_4
        );
        assert_eq!(ENDPOINT_LIST_MERGED_4.endpoints.len(), 6);
    }
}
