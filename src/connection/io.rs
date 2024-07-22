use {
    super::{
        packets::{ErrPacket, OkPacket},
        Connection, ParseBuf, Serialize, BUFFER_POOL, MAX_PAYLOAD_LEN,
    },
    crate::{
        error::ProtocolError,
        packets::StmtSendLongData,
        pool::PoolItem,
        types::{SimpleValue, Value},
        utils::read_u32,
        Deserialize, Error, StreamRequirements, Timeout, TimeoutFuture,
    },
    bytes::Buf,
    std::time::Duration,
    tokio::io::{AsyncReadExt, AsyncWriteExt},
};

impl Connection {
    pub(super) async fn send_long_data<'a, V, I>(
        &mut self,
        statement_id: u32,
        params: I,
    ) -> Result<(), Error>
    where
        V: SimpleValue + 'a,
        I: Iterator<Item = &'a V>,
    {
        for (i, value) in params.enumerate() {
            if let Value::Bytes(bytes) = value.value() {
                if bytes.is_empty() {
                    self.write_command(&StmtSendLongData::new(statement_id, i as u16, &[]))
                        .await?;
                } else {
                    let chunks = bytes.chunks(MAX_PAYLOAD_LEN - 6);
                    for chunk in chunks {
                        self.write_command(&StmtSendLongData::new(statement_id, i as u16, chunk))
                            .await?;
                    }
                }
            }
        }
        Ok(())
    }

    async fn read_chunk_to_buf(
        stream: &mut dyn StreamRequirements,
        dst: &mut Vec<u8>,
        sleep: &dyn Fn(std::time::Duration) -> TimeoutFuture,
        timeout: Duration,
    ) -> Result<(u8, bool), Error> {
        let mut metadata_buf = [0u8; 4];
        Timeout::new(stream.read_exact(&mut metadata_buf), sleep, timeout).await??;
        let chunk_len = read_u32(&metadata_buf[..3]) as usize;
        let seq_id = metadata_buf[3];

        if chunk_len == 0 {
            return Ok((seq_id, true));
        }

        let start = dst.len();
        dst.resize(start + chunk_len, 0);
        Timeout::new(stream.read_exact(&mut dst[start..]), sleep, timeout).await??;

        if dst.len() % MAX_PAYLOAD_LEN == 0 {
            Ok((seq_id, false))
        } else {
            Ok((seq_id, true))
        }
    }

    pub(super) async fn read_packet_to_buf(
        stream: &mut dyn StreamRequirements,
        seq_id: &mut u8,
        dst: &mut Vec<u8>,
        sleep: &dyn Fn(std::time::Duration) -> TimeoutFuture,
        timeout: Duration,
    ) -> Result<(), Error> {
        loop {
            let (read_seq_id, last_chunk) =
                Self::read_chunk_to_buf(stream, dst, sleep, timeout).await?;
            if *seq_id != read_seq_id {
                return Err(Error::Protocol(ProtocolError::OutOfSync));
            }

            *seq_id = seq_id.wrapping_add(1);

            if last_chunk {
                return Ok(());
            }
        }
    }

    pub(super) async fn read_packet<'b>(&mut self) -> Result<PoolItem<'b, Vec<u8>>, Error> {
        let mut decode_buf = BUFFER_POOL.get();
        Self::read_packet_to_buf(
            &mut self.stream,
            &mut self.seq_id,
            decode_buf.as_mut(),
            self.data.sleep,
            self.options.timeout(),
        )
        .await?;
        Ok(decode_buf)
    }

    pub(super) async fn write_packet(&mut self, mut bytes: &[u8]) -> Result<(), Error> {
        let extra_packet = bytes.remaining() % MAX_PAYLOAD_LEN == 0;

        while bytes.has_remaining() {
            let chunk_len = usize::min(bytes.remaining(), MAX_PAYLOAD_LEN);
            Timeout::new(
                self.stream
                    .write_u32_le(chunk_len as u32 | (u32::from(self.seq_id) << 24)),
                self.data.sleep,
                self.options.timeout(),
            )
            .await??;
            Timeout::new(
                self.stream.write_all(&bytes[..chunk_len]),
                self.data.sleep,
                self.options.timeout(),
            )
            .await??;
            bytes = &bytes[chunk_len..];
            self.seq_id = self.seq_id.wrapping_add(1);
        }

        if extra_packet {
            Timeout::new(
                self.stream.write_u32_le(u32::from(self.seq_id) << 24),
                self.data.sleep,
                self.options.timeout(),
            )
            .await??;
            self.seq_id = self.seq_id.wrapping_add(1);
        }
        Ok(())
    }

    pub(super) async fn write_struct<S: Serialize>(&mut self, x: &S) -> Result<(), Error> {
        let mut buf = BUFFER_POOL.get();
        x.serialize(buf.as_mut());
        self.write_packet(&buf).await
    }

    pub(super) async fn write_command<S: Serialize>(&mut self, cmd: &S) -> Result<(), Error> {
        self.cleanup().await?;
        self.seq_id = 0;
        self.write_struct(cmd).await
    }

    pub(crate) async fn decode_response(
        &mut self,
        packet: &[u8],
    ) -> Result<Result<OkPacket, ErrPacket>, Error> {
        let capabilities = self.data().capabilities();
        if packet.is_empty() {
            return Err(ProtocolError::eof().into());
        }
        match packet[0] {
            0x00 => Ok(Ok(OkPacket::read_ok(packet, capabilities)?)),
            0xFF => Ok(Err(ErrPacket::deserialize(
                &mut ParseBuf(packet),
                capabilities,
            )?)),
            _ => Err(
                ProtocolError::unexpected_packet(packet.to_vec(), Some("Ok or Err Packet")).into(),
            ),
        }
    }

    pub(crate) async fn read_response(&mut self) -> Result<Result<OkPacket, ErrPacket>, Error> {
        let packet = self.read_packet().await?;
        self.decode_response(&packet).await
    }
}
