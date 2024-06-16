use bytes::BufMut;

pub trait BufMutExt: BufMut {
    #[allow(dead_code)]
    fn put_lenenc_int(&mut self, n: u64) {
        if n < 251 {
            self.put_u8(n as u8);
        } else if n < 65_536 {
            self.put_u8(0xFC);
            self.put_uint_le(n, 2);
        } else if n < 16_777_216 {
            self.put_u8(0xFD);
            self.put_uint_le(n, 3);
        } else {
            self.put_u8(0xFE);
            self.put_uint_le(n, 8);
        }
    }

    #[allow(dead_code)]
    fn put_lenenc_slice(&mut self, s: &[u8]) {
        self.put_lenenc_int(s.len() as u64);
        self.put_slice(s);
    }

    /// Writes a string with u8 length prefix. Truncates, if the length is greater that `u8::MAX`.
    #[allow(dead_code)]
    fn put_u8_slice(&mut self, s: &[u8]) {
        let len = std::cmp::min(s.len(), u8::MAX as usize);
        self.put_u8(len as u8);
        self.put_slice(&s[..len]);
    }

    /// Writes a string with u32 length prefix. Truncates, if the length is greater that `u32::MAX`.
    #[allow(dead_code)]
    fn put_u32_slice(&mut self, s: &[u8]) {
        let len = std::cmp::min(s.len(), u32::MAX as usize);
        self.put_u32_le(len as u32);
        self.put_slice(&s[..len]);
    }

    #[allow(dead_code)]
    fn put_null_slice(&mut self, s: &[u8]) {
        self.put_slice(s);
        self.put_u8(0);
    }
}

impl<T: BufMut> BufMutExt for T {}
