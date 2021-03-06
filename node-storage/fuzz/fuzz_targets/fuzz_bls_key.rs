#![no_main]
use libfuzzer_sys::fuzz_target;

use elrond_wasm::elrond_codec::*;
use elrond_wasm::elrond_codec::test_util::*;
use node_storage::types::*;

fuzz_target!(|data: &[u8]| {
    if let Ok(decoded) = BLSKey::top_decode(Box::from(data)) {
        let encoded_clean = check_top_encode(&decoded);
        let decoded_again = check_top_decode::<BLSKey>(&encoded_clean[..]);
        let encoded_again = check_top_encode(&decoded_again);
        assert_eq!(encoded_clean, encoded_again);
    }
});
