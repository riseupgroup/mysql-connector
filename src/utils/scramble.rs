use sha1::{Digest, Sha1};

fn xor<T, U>(mut left: T, right: U) -> T
where
    T: AsMut<[u8]>,
    U: AsRef<[u8]>,
{
    for (l, r) in left.as_mut().iter_mut().zip(right.as_ref().iter()) {
        *l ^= r;
    }
    left
}

/// SHA1(password) XOR SHA1(nonce, SHA1(SHA1(password)))
pub fn scramble_native(nonce: &[u8], password: &[u8]) -> Option<[u8; 20]> {
    if password.is_empty() {
        return None;
    }

    fn sha1<T: AsRef<[u8]>>(bytes: &[T]) -> [u8; 20] {
        let mut hasher = Sha1::new();
        for bytes in bytes {
            hasher.update(bytes);
        }
        hasher.finalize().into()
    }

    Some(xor(
        sha1(&[password]),
        sha1(&[nonce, &sha1(&[sha1(&[password])])]),
    ))
}

/// SHA256(password) XOR SHA256(SHA256(SHA256(password)), nonce)
#[cfg(feature = "caching-sha2-password")]
#[cfg_attr(doc, doc(cfg(feature = "caching-sha2-password")))]
pub fn scramble_sha256(nonce: &[u8], password: &[u8]) -> Option<[u8; 32]> {
    if password.is_empty() {
        return None;
    }

    fn sha256<T: AsRef<[u8]>>(bytes: &[T]) -> [u8; 32] {
        let mut hasher = sha2::Sha256::new();
        for bytes in bytes {
            hasher.update(bytes);
        }
        hasher.finalize().into()
    }

    Some(xor(
        sha256(&[password]),
        sha256(&[&sha256(&[sha256(&[password])])[..], nonce]),
    ))
}
