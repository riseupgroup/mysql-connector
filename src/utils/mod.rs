mod buf_mut_ext;
mod scramble;

#[cfg(feature = "caching-sha2-password")]
#[cfg_attr(doc, doc(cfg(feature = "caching-sha2-password")))]
pub(crate) mod crypt;

pub(crate) use {buf_mut_ext::BufMutExt, scramble::*};

#[cfg(feature = "caching-sha2-password")]
#[cfg_attr(doc, doc(cfg(feature = "caching-sha2-password")))]
pub(crate) use crypt::{OaepPadding, PublicKey};

pub fn lenenc_int_len(x: u64) -> u64 {
    if x < 251 {
        1
    } else if x < 65_536 {
        3
    } else if x < 16_777_216 {
        4
    } else {
        9
    }
}

pub fn lenenc_slice_len(s: &[u8]) -> u64 {
    let len = s.len() as u64;
    lenenc_int_len(len) + len
}

pub fn read_u32(value: &[u8]) -> u32 {
    let mut bytes = [0u8; 4];
    for (i, b) in value.iter().enumerate() {
        bytes[i] = *b;
    }
    u32::from_le_bytes(bytes)
}
