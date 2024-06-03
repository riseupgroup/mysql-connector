use {super::ParseBuf, crate::error::ProtocolError};

pub(crate) trait Serialize {
    fn serialize(&self, buf: &mut Vec<u8>);
}

pub(crate) trait Deserialize<'de>: Sized {
    /// Size of a serialized value (in bytes), if it's constant
    const SIZE: Option<usize>;
    type Ctx;

    fn deserialize(buf: &mut ParseBuf<'de>, ctx: Self::Ctx) -> Result<Self, ProtocolError>;
}

macro_rules! num_serialization {
    ($($t:ty),* $(,)?) => {
        $(
            const _: () = {
                const MY_SIZE: usize = std::mem::size_of::<$t>();

                impl Serialize for $t {
                    fn serialize(&self, buf: &mut Vec<u8>) {
                        buf.extend_from_slice(&self.to_le_bytes())
                    }
                }

                impl<'de> Deserialize<'de> for $t {
                    const SIZE: Option<usize> = Some(MY_SIZE);
                    type Ctx = ();

                    fn deserialize(buf: &mut ParseBuf<'de>, _ctx: Self::Ctx) -> Result<Self, ProtocolError> {
                        let bytes = buf.eat(MY_SIZE);
                        Ok(unsafe { <$t>::from_le_bytes(*(bytes as *const _ as *const [_; MY_SIZE])) })
                    }
                }
            };
        )*
    };
}

num_serialization!(u8, u16, u32, u64, u128);
num_serialization!(i8, i16, i32, i64, i128);
num_serialization!(f32, f64);

impl<'de, const N: usize> Deserialize<'de> for [u8; N] {
    const SIZE: Option<usize> = Some(N);
    type Ctx = ();

    fn deserialize(buf: &mut ParseBuf<'de>, _ctx: Self::Ctx) -> Result<Self, ProtocolError> {
        Ok(unsafe { *(buf.eat(N) as *const _ as *const [u8; N]) })
    }
}
