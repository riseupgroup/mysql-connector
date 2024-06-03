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
            hasher.update(bytes.as_ref());
        }
        hasher.finalize().into()
    }

    Some(xor(
        sha1(&[password]),
        sha1(&[nonce, &sha1(&[sha1(&[password])])]),
    ))
}
