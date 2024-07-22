use {
    super::{
        types::{Column, Protocol},
        Connection, MAX_PAYLOAD_LEN,
    },
    crate::{
        bitflags::CapabilityFlags,
        model::{FromQueryResult, FromQueryResultMapping},
        packets::{ErrPacket, OkPacket},
        Deserialize, Error, ParseBuf,
    },
    std::marker::PhantomData,
};

pub struct ResultSet<'a, P, R>
where
    P: Protocol,
    R: FromQueryResult,
{
    __phantom_data: PhantomData<P>,
    columns: Vec<Column>,
    mapping: R::Mapping,
    ok_packet: Option<OkPacket>,
    conn: &'a mut Connection,
}

impl<'a, P, R> ResultSet<'a, P, R>
where
    P: Protocol,
    R: FromQueryResult,
{
    pub(super) async fn read(conn: &'a mut Connection) -> Result<Self, Error> {
        let packet = conn.read_packet().await?;
        match packet.first() {
            Some(0x00) => {
                let mut res = ResultSet::new(Vec::new(), conn);
                res.ok_packet = Some(res.conn.decode_response(&packet).await??);
                Ok(res)
            }
            Some(0xFB) => unimplemented!("local infile"),
            Some(0xFF) => Err(ErrPacket::deserialize(
                &mut ParseBuf(&packet),
                conn.data().capabilities(),
            )?
            .into()),
            _ => {
                conn.pending_result = true;
                let columns_len = ParseBuf(&packet).checked_eat_lenenc_int()?;
                let columns = conn.read_column_defs(columns_len as usize).await?;
                Ok(ResultSet::new(columns, conn))
            }
        }
    }

    fn new(columns: Vec<Column>, conn: &'a mut Connection) -> Self {
        let mapping = R::Mapping::from_columns(&columns);
        Self {
            __phantom_data: PhantomData,
            columns,
            mapping,
            ok_packet: None,
            conn,
        }
    }

    pub async fn next(&mut self) -> Result<Option<R>, Error> {
        if self.ok_packet.is_some() {
            return Ok(None);
        }
        let packet = self.conn.read_packet().await?;
        let is_last_result_set_packet = if self
            .conn
            .data
            .capabilities
            .contains(CapabilityFlags::DEPRECATE_EOF)
        {
            packet[0] == 0xFE && packet.len() < MAX_PAYLOAD_LEN
        } else {
            packet[0] == 0xFE && packet.len() < 8
        };
        if is_last_result_set_packet {
            self.ok_packet = Some(OkPacket::read_eof(packet, self.conn.data.capabilities)?);
            self.conn.pending_result = false;
            Ok(None)
        } else {
            let mut row = P::read_result_set_row(&packet, &self.columns)?;
            Ok(Some(R::from_mapping_and_row(&self.mapping, &mut row)?))
        }
    }

    pub async fn collect(&mut self) -> Result<Vec<R>, Error> {
        let mut rows = Vec::new();
        while let Some(row) = self.next().await? {
            rows.push(row);
        }
        Ok(rows)
    }

    pub async fn one(&mut self) -> Result<Option<R>, Error> {
        let res = self.next().await;
        while self.next().await?.is_some() {}
        res
    }

    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    pub fn into_columns(self) -> Vec<Column> {
        self.columns
    }

    pub fn mapping(&self) -> &R::Mapping {
        &self.mapping
    }

    pub fn into_mapping(self) -> R::Mapping {
        self.mapping
    }

    pub fn into_inner(self) -> (Vec<Column>, R::Mapping) {
        (self.columns, self.mapping)
    }

    pub async fn finish(mut self) -> Result<OkPacket, Error> {
        match self.ok_packet {
            Some(x) => Ok(x),
            None => {
                while self.next().await?.is_some() {}
                // Safety: `self.next()` only returns `None` if ok packet was read
                Ok(self.ok_packet.unwrap())
            }
        }
    }

    pub async fn finish_into_inner(mut self) -> Result<(OkPacket, Vec<Column>, R::Mapping), Error> {
        match self.ok_packet {
            Some(x) => Ok((x, self.columns, self.mapping)),
            None => {
                while self.next().await?.is_some() {}
                // Safety: `self.next() only returns `None` if ok packet was read
                Ok((self.ok_packet.unwrap(), self.columns, self.mapping))
            }
        }
    }
}

impl Connection {
    pub async fn cleanup(&mut self) -> Result<Option<OkPacket>, Error> {
        if self.pending_result {
            loop {
                let packet = self.read_packet().await?;
                let is_last_result_set_packet = if self
                    .data
                    .capabilities
                    .contains(CapabilityFlags::DEPRECATE_EOF)
                {
                    packet[0] == 0xFE && packet.len() < MAX_PAYLOAD_LEN
                } else {
                    packet[0] == 0xFE && packet.len() < 8
                };
                if is_last_result_set_packet {
                    self.pending_result = false;
                    return Ok(Some(OkPacket::read_eof(packet, self.data.capabilities)?));
                }
            }
        } else {
            Ok(None)
        }
    }
}
