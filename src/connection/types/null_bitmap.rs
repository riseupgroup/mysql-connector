use crate::{error::ProtocolError, Deserialize, ParseBuf};

pub struct NullBitmap<const CLIENT_SIDE: bool, T: AsRef<[u8]>>(T);

impl<const CLIENT_SIDE: bool, T: AsRef<[u8]>> NullBitmap<CLIENT_SIDE, T> {
    pub const fn bitmap_len(num_columns: usize) -> usize {
        (num_columns + 7 + Self::offset()) / 8
    }

    const fn offset() -> usize {
        match CLIENT_SIDE {
            true => 0,
            false => 2,
        }
    }

    fn byte_and_bit(&self, column_index: usize) -> (usize, u8) {
        let offset = column_index + Self::offset();
        let byte = offset / 8;
        let bit = 1 << (offset % 8) as u8;

        assert!(byte < self.0.as_ref().len());

        (byte, bit)
    }

    pub fn is_null(&self, column_index: usize) -> bool {
        let (byte, bit) = self.byte_and_bit(column_index);
        self.0.as_ref()[byte] & bit != 0
    }

    pub fn len(&self) -> usize {
        self.0.as_ref().len()
    }

    pub fn from_bytes(bytes: T) -> Self {
        Self(bytes)
    }

    pub fn into_bytes(self) -> T {
        self.0
    }
}

impl<const CLIENT_SIDE: bool> NullBitmap<CLIENT_SIDE, Vec<u8>> {
    pub fn new(num_columns: usize) -> Self {
        Self::from_bytes(vec![0; Self::bitmap_len(num_columns)])
    }
}

impl<const CLIENT_SIDE: bool, T: AsRef<[u8]> + AsMut<[u8]>> NullBitmap<CLIENT_SIDE, T> {
    pub fn set(&mut self, column_index: usize, is_null: bool) {
        let (byte, bit) = self.byte_and_bit(column_index);
        if is_null {
            self.0.as_mut()[byte] |= bit
        } else {
            self.0.as_mut()[byte] &= !bit
        }
    }
}

impl<'de, const CLIENT_SIDE: bool> Deserialize<'de> for NullBitmap<CLIENT_SIDE, Vec<u8>> {
    const SIZE: Option<usize> = None;
    type Ctx = usize;

    fn deserialize(buf: &mut ParseBuf<'de>, num_columns: Self::Ctx) -> Result<Self, ProtocolError> {
        let bitmap_len = Self::bitmap_len(num_columns);
        let bytes = buf.checked_eat(bitmap_len)?;
        Ok(Self::from_bytes(bytes.to_vec()))
    }
}
