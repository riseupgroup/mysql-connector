use {
    super::{types::AuthPlugin, Connection, ParseBuf, Socket},
    crate::{
        error::ProtocolError,
        packets::{AuthSwitchRequest, ErrPacket},
        Deserialize, Error,
    },
    std::{future::Future, pin::Pin},
};

impl<T: Socket> Connection<T> {
    pub(super) fn continue_auth(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + '_>> {
        match self.options.auth_plugin.unwrap_or(self.data.auth_plugin) {
            AuthPlugin::Sha2 => {
                unimplemented!(concat!(
                    "caching_sha2_password_auth is not yet implemented.\n",
                    "You can change the auth method of an user by running `alter user \"user\"@\"host\" identified with mysql_native_password by \"password\"`.\n",
                    "To list all users, you can run `select user, host from mysql.user`.\n"
                ));
            }
            AuthPlugin::Native | AuthPlugin::Clear => {
                Box::pin(self.continue_mysql_native_password_auth())
            }
        }
    }

    async fn continue_mysql_native_password_auth(&mut self) -> Result<(), Error> {
        let packet = self.read_packet().await?;
        match packet.first() {
            Some(0x00) => Ok(()),
            Some(0xfe) if !self.data.auth_switched => {
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
