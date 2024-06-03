use {super::Deserialize, crate::error::ProtocolError, std::io};

#[derive(Debug, Clone)]
pub(crate) struct ParseBuf<'a>(pub(crate) &'a [u8]);

impl io::Read for ParseBuf<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let count = self.0.len().min(buf.len());
        (buf[..count]).copy_from_slice(&self.0[..count]);
        self.0 = &self.0[count..];
        Ok(count)
    }
}

impl<'a> ParseBuf<'a> {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn skip(&mut self, cnt: usize) {
        self.0 = &self.0[usize::min(cnt, self.0.len())..];
    }

    #[inline]
    pub fn check_len(&self, len: usize) -> Result<(), ProtocolError> {
        if self.len() < len {
            Err(ProtocolError::eof())
        } else {
            Ok(())
        }
    }

    /// Eats n bytes
    ///
    /// # Panic
    ///
    /// Will panic if `n > self.len()`
    #[inline]
    pub fn eat(&mut self, n: usize) -> &'a [u8] {
        let (left, right) = self.0.split_at(n);
        self.0 = right;
        left
    }

    #[inline]
    pub fn checked_eat(&mut self, n: usize) -> Result<&'a [u8], ProtocolError> {
        if self.len() >= n {
            Ok(self.eat(n))
        } else {
            Err(ProtocolError::eof())
        }
    }

    #[inline]
    pub fn eat_all(&mut self) -> &'a [u8] {
        let value = self.0;
        self.0 = &[];
        value
    }

    #[inline]
    pub fn parse_unchecked<T>(&mut self, ctx: T::Ctx) -> Result<T, ProtocolError>
    where
        T: Deserialize<'a>,
    {
        T::deserialize(self, ctx)
    }

    #[inline]
    pub fn parse<T>(&mut self, ctx: T::Ctx) -> Result<T, ProtocolError>
    where
        T: Deserialize<'a>,
    {
        if let Some(size) = T::SIZE {
            if self.len() < size {
                return Err(ProtocolError::eof());
            }
        }
        self.parse_unchecked(ctx)
    }
}

macro_rules! parse_num {
    ($t:ty) => {
        paste::paste! {
            #[allow(dead_code)]
            pub fn [< eat_ $t >](&mut self) -> $t {
                const SIZE: usize = std::mem::size_of::<$t>();
                let bytes = self.eat(SIZE);
                unsafe { <$t>::from_le_bytes(*(bytes as *const _ as *const [_; SIZE])) }
            }

            #[allow(dead_code)]
            pub fn [< checked_eat_ $t >](&mut self) -> Result<$t, ProtocolError> {
                const SIZE: usize = std::mem::size_of::<$t>();
                let bytes = self.checked_eat(SIZE)?;
                Ok(unsafe { <$t>::from_le_bytes(*(bytes as *const _ as *const [_; SIZE])) })
            }
        }
    };
    ($t:ty, $name:ident, $size:literal) => {
        paste::paste! {
            #[allow(dead_code)]
            pub fn [< eat_ $name >](&mut self) -> $t {
                let mut bytes = [0u8; std::mem::size_of::<$t>()];
                for (i, b) in self.eat($size).iter().enumerate() {
                    bytes[i] = *b;
                }
                <$t>::from_le_bytes(bytes)
            }

            #[allow(dead_code)]
            pub fn [< checked_eat_ $name >](&mut self) -> Result<$t, ProtocolError> {
                let mut bytes = [0u8; std::mem::size_of::<$t>()];
                for (i, b) in self.checked_eat($size)?.iter().enumerate() {
                    bytes[i] = *b;
                }
                Ok(<$t>::from_le_bytes(bytes))
            }
        }
    };
    ($($($t:ty)? $({$t2:ty, $n:ident, $s:literal})?),* $(,)?) => {
        $(
            $(parse_num!($t);)?
            $(parse_num!($t2, $n, $s);)?
        )*
    };
}

impl ParseBuf<'_> {
    parse_num!(u8, u16, {u32,u24,3}, u32, {u64,u40,5}, {u64,u48,6}, {u64,u56,7}, u64, u128);
    parse_num!(i8, i16, {i32,i24,3}, i32, {i64,i40,5}, {i64,i48,6}, {i64,i56,7}, i64, i128);
    parse_num!(f32, f64);
}

#[allow(dead_code)]
impl<'a> ParseBuf<'a> {
    /// Consumes MySql length-encoded integer.
    ///
    /// Returns `0` if integer is malformed (starts with 0xff or 0xfb).
    pub fn eat_lenenc_int(&mut self) -> u64 {
        match self.eat_u8() {
            x @ 0..=0xfa => x as u64,
            0xfc => self.eat_u16() as u64,
            0xfd => self.eat_u24() as u64,
            0xfe => self.eat_u64(),
            0xfb | 0xff => 0,
        }
    }

    pub fn checked_eat_lenenc_int(&mut self) -> Result<u64, ProtocolError> {
        match self.checked_eat_u8()? {
            x @ 0..=0xfa => Ok(x as u64),
            0xfc => self.checked_eat_u16().map(|x| x as u64),
            0xfd => self.checked_eat_u24().map(|x| x as u64),
            0xfe => self.checked_eat_u64(),
            0xfb | 0xff => Ok(0),
        }
    }

    /// Returns an empty slice if length is malformed (starts with 0xff or 0xfb).
    pub fn eat_lenenc_slice(&mut self) -> &'a [u8] {
        let len: u64 = self.eat_lenenc_int();
        self.eat(len as usize)
    }

    /// Returns an empty string if length is malformed (starts with 0xff or 0xfb).
    pub fn eat_lenenc_str(&mut self) -> Result<&'a str, ProtocolError> {
        std::str::from_utf8(self.eat_lenenc_slice()).map_err(Into::into)
    }

    pub fn checked_eat_lenenc_slice(&mut self) -> Result<&'a [u8], ProtocolError> {
        let len = self.checked_eat_lenenc_int()?;
        self.checked_eat(len as usize)
    }

    pub fn checked_eat_lenenc_str(&mut self) -> Result<&'a str, ProtocolError> {
        std::str::from_utf8(self.checked_eat_lenenc_slice()?).map_err(Into::into)
    }

    pub fn eat_u8_slice(&mut self) -> &'a [u8] {
        let len = self.eat_u8();
        self.eat(len as usize)
    }

    pub fn eat_u8_str(&mut self) -> Result<&'a str, ProtocolError> {
        std::str::from_utf8(self.eat_u8_slice()).map_err(Into::into)
    }

    pub fn checked_eat_u8_slice(&mut self) -> Result<&'a [u8], ProtocolError> {
        let len = self.checked_eat_u8()?;
        self.checked_eat(len as usize)
    }

    pub fn checked_eat_u8_str(&mut self) -> Result<&'a str, ProtocolError> {
        std::str::from_utf8(self.checked_eat_u8_slice()?).map_err(Into::into)
    }

    /// Consumes whole buffer if there is no `0`-byte.
    pub fn eat_null_slice(&mut self) -> &'a [u8] {
        let pos = self
            .0
            .iter()
            .position(|x| *x == 0)
            .map(|x| x + 1)
            .unwrap_or_else(|| self.len());
        match self.eat(pos) {
            [head @ .., 0_u8] => head,
            x => x,
        }
    }

    pub fn eat_null_str(&mut self) -> Result<&'a str, ProtocolError> {
        std::str::from_utf8(self.eat_null_slice()).map_err(Into::into)
    }
}
