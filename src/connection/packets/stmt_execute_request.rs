use {
    crate::{
        bitflags::{CursorTypeFlags, StmtExecuteParamFlags, StmtExecuteParamsFlags},
        connection::{types::NullBitmap, MAX_PAYLOAD_LEN},
        types::{SimpleValue, Value},
        Command, Serialize,
    },
    bytes::BufMut,
};

#[derive(Debug)]
pub struct StmtExecuteRequest<'a, V: SimpleValue> {
    stmt_id: u32,
    flags: CursorTypeFlags,
    iteration_count: u32,
    bitmap: Vec<u8>,
    params_flags: StmtExecuteParamsFlags,
    params: &'a [V],
    as_long_data: bool,
}

impl<'a, V: SimpleValue> StmtExecuteRequest<'a, V> {
    pub fn new(id: u32, params: &'a [V]) -> Self {
        let mut bitmap = NullBitmap::<true, Vec<u8>>::new(params.len());
        let meta_len = params.len() * 2;

        let mut data_len = 0;
        for (i, param) in params.iter().enumerate() {
            match param.value().bin_len() as usize {
                0 => bitmap.set(i, true),
                x => data_len += x,
            }
        }

        let total_len = 10 + bitmap.len() + 1 + meta_len + data_len;
        let as_long_data = total_len > MAX_PAYLOAD_LEN;

        Self {
            stmt_id: id,
            flags: CursorTypeFlags::NO_CURSOR,
            iteration_count: 1,
            params_flags: StmtExecuteParamsFlags::NEW_PARAMS_BOUND,
            bitmap: bitmap.into_bytes(),
            params,
            as_long_data,
        }
    }

    pub fn as_long_data(&self) -> bool {
        self.as_long_data
    }
}

impl<V: SimpleValue> Serialize for StmtExecuteRequest<'_, V> {
    fn serialize(&self, buf: &mut Vec<u8>) {
        (Command::StmtExecute as u8).serialize(&mut *buf);
        self.stmt_id.serialize(&mut *buf);
        self.flags.serialize(&mut *buf);
        self.iteration_count.serialize(&mut *buf);

        if !self.params.is_empty() {
            buf.put_slice(&self.bitmap);
            self.params_flags.serialize(&mut *buf);
        }

        for param in self.params {
            let column_type = param.value().column_type();
            let flags = if param.value().is_unsigned() {
                StmtExecuteParamFlags::UNSIGNED
            } else {
                StmtExecuteParamFlags::empty()
            };

            buf.put_slice(&[column_type as u8, flags.bits()]);
        }

        for param in self.params {
            match *param.value() {
                Value::Bytes(_) if !self.as_long_data => param.value().serialize(buf),
                Value::Bytes(_) | Value::Null => {}
                _ => param.value().serialize(buf),
            }
        }
    }
}
