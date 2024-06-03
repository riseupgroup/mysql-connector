use {
    super::{Column, NullBitmap},
    crate::{error::ProtocolError, types::Value, Deserialize, ParseBuf},
};

pub trait Protocol: Send + Sync + 'static {
    fn read_result_set_row(packet: &[u8], columns: &[Column]) -> Result<Vec<Value>, ProtocolError>;
}

#[derive(Debug)]
pub struct TextProtocol;

impl Protocol for TextProtocol {
    fn read_result_set_row(packet: &[u8], columns: &[Column]) -> Result<Vec<Value>, ProtocolError> {
        let mut buf = ParseBuf(packet);
        let mut values = Vec::with_capacity(columns.len());

        for column in columns {
            values.push(Value::deserialize_text(
                column.r#type(),
                column.flags(),
                &mut buf,
            )?);
        }

        Ok(values)
    }
}

#[derive(Debug)]
pub struct BinaryProtocol;

impl Protocol for BinaryProtocol {
    fn read_result_set_row(packet: &[u8], columns: &[Column]) -> Result<Vec<Value>, ProtocolError> {
        let mut buf = ParseBuf(packet);
        buf.skip(1);

        let bitmap = NullBitmap::<false, Vec<u8>>::deserialize(&mut buf, columns.len())?;
        let mut values = Vec::with_capacity(columns.len());

        for (i, column) in columns.iter().enumerate() {
            if bitmap.is_null(i) {
                values.push(Value::Null);
            } else {
                values.push(Value::deserialize_bin(
                    column.r#type(),
                    column.flags(),
                    &mut buf,
                )?);
            }
        }

        Ok(values)
    }
}
