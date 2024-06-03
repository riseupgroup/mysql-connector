use {
    super::{types::BinaryProtocol, Command, Connection, ParseBuf, ResultSet, Socket},
    crate::{
        error::{ProtocolError, RuntimeError},
        model::FromQueryResult,
        packets::{ErrPacket, OkPacket, Stmt, StmtExecuteRequest},
        types::SimpleValue,
        Deserialize, Error,
    },
};

#[derive(Debug)]
pub struct PreparedStatement<'a, T: Socket> {
    id: u32,
    conn: &'a mut Connection<T>,
    params: usize,
}

impl<'a, T: Socket> PreparedStatement<'a, T> {
    pub async fn query<V: SimpleValue, R: FromQueryResult>(
        &mut self,
        values: &[V],
    ) -> Result<ResultSet<'_, T, BinaryProtocol, R>, Error> {
        if values.len() != self.params {
            return Err(RuntimeError::ParameterCountMismatch.into());
        }

        self.conn
            .query_prepared_statement_unchecked(self.id, values)
            .await
    }

    pub async fn execute<V: SimpleValue>(&mut self, values: &[V]) -> Result<OkPacket, Error> {
        if values.len() != self.params {
            return Err(RuntimeError::ParameterCountMismatch.into());
        }

        self.conn
            .execute_prepared_statement_unchecked(self.id, values)
            .await
    }
}

impl<T: Socket> Connection<T> {
    pub async fn prepare_statement(&mut self, stmt: &str) -> Result<PreparedStatement<T>, Error> {
        self.execute_command(Command::StmtPrepare, stmt).await?;
        let packet = self.read_packet().await?;
        let stmt = match packet.first() {
            Some(0x00) => Stmt::deserialize(&mut ParseBuf(&packet), ())?,
            Some(0xFF) => {
                return Err(
                    ErrPacket::deserialize(&mut ParseBuf(&packet), self.data.capabilities)?.into(),
                )
            }
            _ => return Err(ProtocolError::unexpected_packet(Vec::clone(&packet), None).into()),
        };

        for _ in 0..(stmt.params_len as usize + stmt.columns_len as usize) {
            self.read_packet().await?;
        }

        Ok(PreparedStatement {
            id: stmt.id,
            conn: self,
            params: stmt.params_len as usize,
        })
    }

    async fn query_prepared_statement_unchecked<V: SimpleValue, R: FromQueryResult>(
        &mut self,
        id: u32,
        params: &[V],
    ) -> Result<ResultSet<'_, T, BinaryProtocol, R>, Error> {
        let request = StmtExecuteRequest::new(id, params);

        if request.as_long_data() {
            self.send_long_data(id, params.iter()).await?;
        }

        self.write_command(&request).await?;
        ResultSet::read(self).await
    }

    async fn execute_prepared_statement_unchecked<V: SimpleValue>(
        &mut self,
        id: u32,
        params: &[V],
    ) -> Result<OkPacket, Error> {
        let request = StmtExecuteRequest::new(id, params);

        if request.as_long_data() {
            self.send_long_data(id, params.iter()).await?;
        }

        self.write_command(&request).await?;
        self.read_response().await?.map_err(Into::into)
    }
}
