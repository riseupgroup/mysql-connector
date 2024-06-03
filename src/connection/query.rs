use {
    super::{
        types::{Column, TextProtocol},
        Command, Connection, ParseBuf, ResultSet, Socket, BUFFER_POOL,
    },
    crate::{
        model::FromQueryResult,
        packets::{ColumnDef, OkPacket},
        types::Value,
        Deserialize, Error,
    },
};

impl<T: Socket> Connection<T> {
    pub async fn query<R: FromQueryResult>(
        &mut self,
        query: &str,
    ) -> Result<ResultSet<'_, T, TextProtocol, R>, Error> {
        self.execute_command(Command::Query, query).await?;
        ResultSet::read(self).await
    }

    pub async fn execute_query(&mut self, query: &str) -> Result<OkPacket, Error> {
        self.execute_command(Command::Query, query).await?;
        self.read_response().await?.map_err(Into::into)
    }

    pub(super) async fn execute_command<D>(&mut self, cmd: Command, data: D) -> Result<(), Error>
    where
        D: AsRef<[u8]>,
    {
        let mut buf = BUFFER_POOL.get();
        let body: &mut Vec<u8> = buf.as_mut();
        body.push(cmd as u8);
        body.extend_from_slice(data.as_ref());
        self.cleanup().await?;
        self.seq_id = 0;
        self.write_packet(&buf).await
    }

    pub(super) async fn read_column_defs(&mut self, count: usize) -> Result<Vec<Column>, Error> {
        let mut columns: Vec<Column> = Vec::with_capacity(count);
        for _ in 0..count {
            let packet = self.read_packet().await?;
            let def = ColumnDef::deserialize(&mut ParseBuf(&packet), ())?;
            columns.push(def.try_into()?);
        }
        Ok(columns)
    }

    pub(super) async fn read_settings(&mut self) -> Result<(), Error> {
        if self.options.max_allowed_packet.is_none() || self.options.wait_timeout.is_none() {
            let mut query = String::from("select ");
            if self.options.max_allowed_packet.is_none() {
                query += "@@max_allowed_packet";
            }

            if self.options.wait_timeout.is_none() {
                if self.options.max_allowed_packet.is_none() {
                    query += ", ";
                }
                query += "@@wait_timeout";
            }

            let mut res = self.query::<Vec<Value>>(&query).await?;
            let row = res.next().await?;
            let columns = res.into_columns();

            if let Some(mut row) = row {
                if self.options.max_allowed_packet.is_none() {
                    if let Some(i) = columns
                        .iter()
                        .position(|x| x.name() == "@@max_allowed_packet")
                    {
                        self.data.max_allowed_packet =
                            <Value as TryInto<u64>>::try_into(row[i].take())? as usize;
                    }
                }

                if self.options.wait_timeout.is_none() {
                    if let Some(i) = columns.iter().position(|x| x.name() == "@@wait_timeout") {
                        self.data.wait_timeout =
                            <Value as TryInto<u64>>::try_into(row[i].take())? as usize;
                    }
                }
            }
        }
        Ok(())
    }
}
