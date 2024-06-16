use {
    super::{types::AuthPlugin, Connection, ParseBuf, Stream},
    crate::{
        error::ProtocolError,
        packets::{AuthSwitchRequest, ErrPacket},
        Deserialize, Error,
    },
    std::{future::Future, pin::Pin},
};

impl<T: Stream> Connection<T> {
    pub(super) fn continue_auth(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + '_>> {
        match self.options.auth_plugin.unwrap_or(self.data.auth_plugin) {
            #[cfg(feature = "caching-sha2-password")]
            AuthPlugin::Sha2 => Box::pin(self.continue_caching_sha2_password_auth()),
            AuthPlugin::Native | AuthPlugin::Clear => {
                Box::pin(self.continue_mysql_native_password_auth())
            }
        }
    }

    async fn continue_mysql_native_password_auth(&mut self) -> Result<(), Error> {
        let packet = self.read_packet().await?;
        match packet.first() {
            Some(0x00) => Ok(()),
            Some(0xFE) if !self.data.auth_switched => {
                let auth_switch = AuthSwitchRequest::deserialize(&mut ParseBuf(&packet), ())?;
                self.perform_auth_switch(auth_switch).await
            }
            _ => Err(
                match ErrPacket::deserialize(&mut ParseBuf(&packet), self.data.capabilities) {
                    Ok(err) => err.into(),
                    Err(_) => {
                        ProtocolError::unexpected_packet(packet.to_vec(), Some("Ok or auth switch"))
                            .into()
                    }
                },
            ),
        }
    }

    #[cfg(feature = "caching-sha2-password")]
    #[cfg_attr(doc, doc(cfg(feature = "caching-sha2-password")))]
    async fn continue_caching_sha2_password_auth(&mut self) -> Result<(), Error> {
        use {
            crate::{
                error::SerializeError,
                utils::{OaepPadding, PublicKey},
            },
            rand::SeedableRng as _,
        };
        let packet = self.read_packet().await?;
        match packet.first() {
            Some(0x00) => {
                // ok packet for empty password
                Ok(())
            }
            Some(0x01) => match packet.get(1) {
                Some(0x03) => {
                    // auth ok
                    self.read_packet().await?;
                    Ok(())
                }
                Some(0x04) => {
                    let mut pass = super::BUFFER_POOL.get();
                    pass.extend_from_slice(self.options.password.as_bytes());
                    pass.push(0);

                    if T::SECURE {
                        self.write_packet(&pass).await?;
                    } else {
                        let server_key = match &self.data.server_key {
                            Some(key) => key.clone(),
                            None => {
                                self.write_packet(&[0x02]).await?;
                                let packet = self.read_packet().await?;
                                match packet.first() {
                                    Some(0x01) => {
                                        let server_key = std::sync::Arc::new(
                                            PublicKey::try_from_pem(&packet[1..])
                                                .map_err(SerializeError::from)?,
                                        );
                                        self.data.server_key = Some(server_key.clone());
                                        server_key
                                    }
                                    Some(0xFF) => {
                                        return Err(Error::Server(ErrPacket::deserialize(
                                            &mut ParseBuf(&packet),
                                            self.data.capabilities,
                                        )?))
                                    }
                                    _ => {
                                        return Err(Error::Protocol(
                                            ProtocolError::unexpected_packet(
                                                packet.to_vec(),
                                                Some("Server key"),
                                            ),
                                        ))
                                    }
                                }
                            }
                        };
                        for (i, byte) in pass.iter_mut().enumerate() {
                            *byte ^= self.data.nonce[i % self.data.nonce.len()];
                        }
                        let padding = OaepPadding::new(rand::rngs::StdRng::from_entropy());
                        let encrypted_pass = server_key
                            .encrypt_padded(&pass, padding)
                            .map_err(SerializeError::from)?;
                        self.write_packet(&encrypted_pass).await?;
                    }
                    let res = self.read_packet().await?;
                    match res.first() {
                        Some(0x00) => Ok(()),
                        Some(0xFF) => Err(Error::Server(ErrPacket::deserialize(
                            &mut ParseBuf(&res),
                            self.data.capabilities,
                        )?)),
                        _ => Err(Error::Protocol(ProtocolError::unexpected_packet(
                            res.to_vec(),
                            None,
                        ))),
                    }
                }
                _ => Err(ProtocolError::unexpected_packet(packet.to_vec(), None).into()),
            },
            Some(0xFE) if !self.data.auth_switched => {
                let auth_switch_request = ParseBuf(&packet).parse::<AuthSwitchRequest>(()).unwrap();
                self.perform_auth_switch(auth_switch_request).await
            }
            _ => Err(
                match ErrPacket::deserialize(&mut ParseBuf(&packet), self.data.capabilities) {
                    Ok(err) => err.into(),
                    Err(_) => {
                        ProtocolError::unexpected_packet(packet.to_vec(), Some("Ok or auth switch"))
                            .into()
                    }
                },
            ),
        }
    }

    async fn perform_auth_switch(
        &mut self,
        auth_switch_request: AuthSwitchRequest,
    ) -> Result<(), Error> {
        assert!(
            !self.data.auth_switched,
            "auth_switched flag should be checked by caller"
        );

        self.data.auth_switched = true;
        self.data.auth_plugin = auth_switch_request.plugin();
        self.data.nonce = auth_switch_request.into_data();

        let plugin_data = self.data.auth_plugin.gen_data(
            &self.options.password,
            &self.data.nonce,
            &self.options,
        )?;

        if let Some(plugin_data) = plugin_data {
            self.write_struct(&plugin_data).await?;
        } else {
            self.write_packet(&[]).await?;
        }

        self.continue_auth().await
    }
}
