pub struct OaepPadding<R: Rng + CryptoRng> {
    rng: R,
}

impl<R: Rng + CryptoRng> OaepPadding<R> {
    const HASH_LEN: usize = 20;

    pub fn new(rng: R) -> Self {
        Self { rng }
    }

    fn mgf1(seed: &[u8], len: usize) -> Result<Vec<u8>, ()> {
        if len > Self::HASH_LEN << 32 {
            return Err(());
        }

        let mut output = vec![0u8; len];
        let mut hash_source = vec![0u8; seed.len() + 4];
        hash_source[0..seed.len()].copy_from_slice(seed);

        for i in 0..(len/Self::HASH_LEN) {
            hash_source[seed.len()..].copy_from_slice(&(i as u32).to_be_bytes());
            let pos = i * Self::HASH_LEN;
            output[pos..pos+Self::HASH_LEN].copy_from_slice(&Sha1::digest(&hash_source));
        }

        let remaining = len % Self::HASH_LEN;
        if remaining > 0 {
            hash_source[seed.len()..].copy_from_slice(&((len/Self::HASH_LEN) as u32).to_be_bytes());
            output[len-remaining..].copy_from_slice(&Sha1::digest(&hash_source)[..remaining]);
        }
        Ok(output)
    }

    /// Pads data according to RFC 8017.
    /// 
    /// Returns an error if data is too long.
    /// 
    ///  ```text
    ///                                                  msg_len
    ///                           ┣━━━━━━━ filling_len ━━┻━━━━━━━━━━━━━┫
    ///                           ┣━━━━━━━━┻━━━━━━━━━━━━━━━━━━━┫
    ///                     seed  hash([]) || 0x00..0x00 || 0x01 || data
    ///                     ━┳━━  ━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━
    ///                      ┃              ┃
    ///                      ┣━━━> MGF ━━> xor
    ///                      ┃             ━┳━
    ///                     xor <━━ MGF <━━━┫
    ///                     ━┳━             ┃
    ///                      ┃              ┃
    /// padded = 0x00 || masked seed || masked msg
    /// ```
    pub fn pad(&mut self, data: &[u8], n: usize) -> Result<Vec<u8>, ()> {
        let seed_len = Self::HASH_LEN;
        if n < 1 + seed_len + Self::HASH_LEN + 2 + 1 + data.len() {
            return Err(());
        }

        let msg_len = n - seed_len - 1;
        let filling_len = msg_len - data.len();

        let mut padded = vec![0u8; n];
        let (seed, msg) = padded[1..].split_at_mut(seed_len);
        {
            for byte in seed.iter_mut() {
                *byte = self.rng.gen();
            }
            let (filling, msg_data) = msg.split_at_mut(filling_len);
            filling[0..Self::HASH_LEN].copy_from_slice(&Sha1::digest([]));
            filling[filling_len-1] = 0x01;
            msg_data.copy_from_slice(data);
        }

        let msg_mask = Self::mgf1(&seed, msg.len())?;
        for i in 0..msg.len() {
            msg[i] ^= msg_mask[i];
        }

        let seed_mask = Self::mgf1(&msg, seed_len)?;
        for i in 0..seed_len {
            seed[i] ^= seed_mask[i];
        }

        Ok(padded)
    }
}
