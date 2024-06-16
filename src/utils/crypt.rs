use {
    num::BigUint,
    rand::{CryptoRng, Rng},
    sha1::{Digest, Sha1},
};

#[derive(Debug)]
pub enum Error {
    InvalidPem,
    MessageTooLong,
}

mod der {
    use {
        super::{Error, PublicKey},
        base64::{engine::general_purpose::STANDARD, Engine as _},
        num::BigUint,
    };

    fn eat_len(der: &mut &[u8]) -> Result<usize, Error> {
        if der[0] & 0x80 == 0x80 {
            const BITS: usize = (usize::BITS / 8) as usize;
            let len = (der[0] & (!0x80)) as usize;
            if len > BITS {
                return Err(Error::InvalidPem);
            }
            let mut bytes = [0u8; BITS];
            bytes[BITS - len..].copy_from_slice(&der[1..=len]);
            *der = &der[len + 1..];
            Ok(usize::from_be_bytes(bytes))
        } else {
            let len = der[0] as usize;
            *der = &der[1..];
            Ok(len)
        }
    }

    fn eat_uint(der: &mut &[u8]) -> Result<BigUint, Error> {
        if der[0] != 0x02 {
            return Err(Error::InvalidPem);
        }
        *der = &der[1..];
        let len = eat_len(der)?;
        let uint = BigUint::from_bytes_be(&der[..len]);
        *der = &der[len..];
        Ok(uint)
    }

    fn eat_sequence<'a>(der: &mut &'a [u8]) -> Result<&'a [u8], Error> {
        if der[0] != 0x30 {
            return Err(Error::InvalidPem);
        }
        *der = &der[1..];
        let len = eat_len(der)?;
        let sequence = &der[..len];
        *der = &der[len..];
        Ok(sequence)
    }

    fn eat_bit_string<'a>(der: &mut &'a [u8]) -> Result<(u8, &'a [u8]), Error> {
        if der[0] != 0x03 {
            return Err(Error::InvalidPem);
        }
        *der = &der[1..];
        let len = eat_len(der)?;
        let unused_bits = der[0];
        let bit_string = &der[1..len];
        *der = &der[len..];
        Ok((unused_bits, bit_string))
    }

    impl PublicKey {
        pub fn try_from_pkcs1(mut der: &[u8]) -> Result<Self, Error> {
            let mut pub_key = eat_sequence(&mut der)?;
            let modulus = eat_uint(&mut pub_key)?;
            let exponent = eat_uint(&mut pub_key)?;
            Ok(Self { modulus, exponent })
        }

        pub fn try_from_pkcs8(mut der: &[u8]) -> Result<Self, Error> {
            let mut seq_data = eat_sequence(&mut der)?;
            eat_sequence(&mut seq_data)?;
            let (unused_bits, pub_key) = eat_bit_string(&mut seq_data)?;
            if unused_bits != 0 {
                return Err(Error::InvalidPem);
            }
            Self::try_from_pkcs1(pub_key)
        }

        pub fn try_from_pem(pem: &[u8]) -> Result<Self, Error> {
            const PKCS1: (&[u8], &[u8]) =
                (b"-----BEGINRSAPUBLICKEY-----", b"-----ENDRSAPUBLICKEY-----");
            const PKCS8: (&[u8], &[u8]) = (b"-----BEGINPUBLICKEY-----", b"-----ENDPUBLICKEY-----");

            let pem: Vec<u8> = pem
                .iter()
                .filter(|x| !b" \n\t\r\x0b\x0c".contains(x))
                .cloned()
                .collect();

            let (body, is_pkcs_1) = if pem.starts_with(PKCS1.0) && pem.ends_with(PKCS1.1) {
                (&pem[PKCS1.0.len()..pem.len() - PKCS1.1.len()], true)
            } else if pem.starts_with(PKCS8.0) && pem.ends_with(PKCS8.1) {
                (&pem[PKCS8.0.len()..pem.len() - PKCS8.1.len()], false)
            } else {
                return Err(Error::InvalidPem);
            };

            let body = STANDARD.decode(body).map_err(|_| Error::InvalidPem)?;
            match is_pkcs_1 {
                true => Self::try_from_pkcs1(&body),
                false => Self::try_from_pkcs8(&body),
            }
        }
    }
}

#[derive(Debug)]
pub struct PublicKey {
    modulus: BigUint,
    exponent: BigUint,
}

impl PublicKey {
    pub const fn new(modulus: BigUint, exponent: BigUint) -> Self {
        Self { modulus, exponent }
    }

    pub fn num_octets(&self) -> usize {
        (self.modulus().bits() as usize + 6) >> 3
    }

    pub fn modulus(&self) -> &BigUint {
        &self.modulus
    }

    pub fn exponent(&self) -> &BigUint {
        &self.exponent
    }

    pub fn encrypt_padded<R: Rng + CryptoRng>(
        &self,
        data: &[u8],
        mut padding: OaepPadding<R>,
    ) -> Result<Vec<u8>, Error> {
        let octets = self.num_octets();
        let padded = BigUint::from_bytes_be(&padding.pad(data, octets)?);
        let mut encrypted = padded.modpow(self.exponent(), self.modulus()).to_bytes_be();

        let fill = octets - encrypted.len();
        if fill > 0 {
            let mut encrypted_new = vec![0u8; octets];
            encrypted_new[fill..].copy_from_slice(&encrypted);
            encrypted = encrypted_new;
        }
        Ok(encrypted)
    }
}

pub struct OaepPadding<R: Rng + CryptoRng> {
    rng: R,
}

impl<R: Rng + CryptoRng> OaepPadding<R> {
    const HASH_LEN: usize = 20;

    pub fn new(rng: R) -> Self {
        Self { rng }
    }

    fn mgf1(seed: &[u8], len: usize) -> Result<Vec<u8>, Error> {
        #[cfg(target_pointer_width = "64")]
        if len > Self::HASH_LEN << 32 {
            return Err(Error::MessageTooLong);
        }

        let mut output = vec![0u8; len];
        let mut hash_source = vec![0u8; seed.len() + 4];
        hash_source[0..seed.len()].copy_from_slice(seed);

        for i in 0..(len / Self::HASH_LEN) {
            hash_source[seed.len()..].copy_from_slice(&(i as u32).to_be_bytes());
            let pos = i * Self::HASH_LEN;
            output[pos..pos + Self::HASH_LEN].copy_from_slice(&Sha1::digest(&hash_source));
        }

        let remaining = len % Self::HASH_LEN;
        if remaining > 0 {
            hash_source[seed.len()..]
                .copy_from_slice(&((len / Self::HASH_LEN) as u32).to_be_bytes());
            output[len - remaining..].copy_from_slice(&Sha1::digest(&hash_source)[..remaining]);
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
    pub fn pad(&mut self, data: &[u8], n: usize) -> Result<Vec<u8>, Error> {
        let seed_len = Self::HASH_LEN;
        if n < 1 + seed_len + Self::HASH_LEN + 2 + 1 + data.len() {
            return Err(Error::MessageTooLong);
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
            filling[filling_len - 1] = 0x01;
            msg_data.copy_from_slice(data);
        }

        let msg_mask = Self::mgf1(seed, msg.len())?;
        for i in 0..msg.len() {
            msg[i] ^= msg_mask[i];
        }

        let seed_mask = Self::mgf1(msg, seed_len)?;
        for i in 0..seed_len {
            seed[i] ^= seed_mask[i];
        }

        Ok(padded)
    }
}
