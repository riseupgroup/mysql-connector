use {
    super::{
        types::AuthPlugin, Command, Connection, ConnectionData, ConnectionOptions, ParseBuf,
        Stream, BUFFER_POOL, DEFAULT_MAX_ALLOWED_PACKET,
    },
    crate::{
        packets::{HandshakePacket, HandshakeResponse},
        Error, Serialize,
    },
    std::sync::Arc,
};

impl<T: Stream> Connection<T> {
    pub async fn connect(options: Arc<ConnectionOptions<T>>) -> Result<Self, Error> {
        let mut stream = T::connect(&options.connection).await?;
        let mut seq_id = 0;

        let data = Self::handle_handshake(&mut stream, &mut seq_id, options.clone()).await?;
        let mut this = Self {
            stream,
            seq_id,
            data,
            options,
            pending_result: false,
        };
        this.do_handshake_response().await?;
        this.continue_auth().await?;
        this.read_settings().await?;
        Ok(this)
    }

    pub async fn disconnect(mut self) -> Result<(), Error> {
        self.execute_command(Command::Quit, &[]).await
    }

    async fn handle_handshake(
        stream: &mut T,
        seq_id: &mut u8,
        options: Arc<ConnectionOptions<T>>,
    ) -> Result<ConnectionData, Error> {
        #[cfg(feature = "time")]
        fn sleep(duration: std::time::Duration) -> crate::TimeoutFuture {
            Box::pin(tokio::time::sleep(duration))
        }
        #[cfg(not(feature = "time"))]
        let sleep = match options.sleep {
            Some(x) => x,
            None => panic!(concat!(
                "No `sleep` function provided.\n",
                "You have to either provide a custom `sleep` function by setting `ConnectionData::sleep` or enable the feature `time`.",
            )),
        };
        let mut packet = BUFFER_POOL.get();
        Self::read_packet_to_buf(stream, seq_id, packet.as_mut(), &sleep, options.timeout).await?;
        let handshake = ParseBuf(&packet).parse::<HandshakePacket>(()).unwrap();

        let (version, is_mariadb) = handshake
            .parse_server_version()
            .unwrap_or(((0, 0, 0), false));
        let auth_plugin = handshake.auth_plugin().unwrap_or(AuthPlugin::Native);

        Ok(ConnectionData {
            id: handshake.connection_id(),
            is_mariadb,
            version,
            capabilities: handshake.capabilities() & options.get_capabilities(),
            nonce: handshake.into_nonce(),
            #[cfg(feature = "caching-sha2-password")]
            server_key: options.server_key.clone(),
            auth_plugin,
            auth_switched: false,
            max_allowed_packet: options
                .max_allowed_packet
                .unwrap_or(DEFAULT_MAX_ALLOWED_PACKET),
            #[cfg(feature = "time")]
            sleep: &sleep,
            #[cfg(not(feature = "time"))]
            sleep,
        })
    }

    async fn do_handshake_response(&mut self) -> Result<(), Error> {
        let auth_plugin = self.options.auth_plugin.unwrap_or(self.data.auth_plugin);
        let auth_data =
            auth_plugin.gen_data(&self.options.password, &self.data.nonce, &self.options)?;

        let handshake_response = HandshakeResponse::new(
            auth_data.as_deref().unwrap_or_default(),
            self.data.version,
            self.options.user.as_bytes(),
            self.options.db_name.as_ref().map(|x| x.as_bytes()),
            Some(auth_plugin),
            self.data.capabilities,
            Default::default(),
            self.data.max_allowed_packet as u32,
        );

        let mut buf = BUFFER_POOL.get();
        handshake_response.serialize(buf.as_mut());
        self.write_packet(&buf).await
    }
}
