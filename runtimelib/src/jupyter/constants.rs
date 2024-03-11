use bytes::Bytes;
use lazy_static::lazy_static;
use serde_json::{self, Value};

lazy_static! {
    pub static ref EMPTY_DICT_BYTES: Bytes = {
        let empty_dict: Value = serde_json::json!({});
        let empty_dict_bytes = serde_json::to_vec(&empty_dict).unwrap();
        Bytes::from(empty_dict_bytes)
    };
}
