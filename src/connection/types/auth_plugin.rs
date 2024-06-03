use {
    crate::{
        error::{ProtocolError, RuntimeError},
        ConnectionOptions, Deserialize, Error, ParseBuf, Serialize,
    },
    bytes::BufMut,
    std::sync::Arc,
};

const MYSQL_NATIVE_PASSWORD_PLUGIN_NAME: &[u8] = b"mysql_native_password";
const CACHING_SHA2_PASSWORD_PLUGIN_NAME: &[u8] = b"caching_sha2_password";
const MYSQL_CLEAR_PASSWORD_PLUGIN_NAME: &[u8] = b"mysql_clear_password";

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum AuthPlugin {
    /// `mysql_clear_password`
    Clear,
    /// `mysql_native_password`
    Native,
    /// `caching_sha2_password`
    ///
    /// Default since MySql v8.0.4
    Sha2,
}

impl<'de> Deserialize<'de> for AuthPlugin {
    const SIZE: Option<usize> = None;
    type Ctx = ();

    fn deserialize(buf: &mut ParseBuf<'de>, _ctx: Self::Ctx) -> Result<AuthPlugin, ProtocolError> {
        Self::from_bytes(buf.eat_null_slice())
    }
}

impl Serialize for AuthPlugin {
    fn serialize(&self, buf: &mut Vec<u8>) {
        buf.put_slice(self.as_bytes());
        buf.put_u8(0);
    }
}

impl AuthPlugin {
    pub fn from_bytes(name: &[u8]) -> Result<AuthPlugin, ProtocolError> {
        let name = match name {
            [head @ .., 0] => head,
            // missing trailing `0` is a known bug in mysql
            all => all,
        };
        match name {
            MYSQL_CLEAR_PASSWORD_PLUGIN_NAME => Ok(AuthPlugin::Clear),
            MYSQL_NATIVE_PASSWORD_PLUGIN_NAME => Ok(AuthPlugin::Native),
            CACHING_SHA2_PASSWORD_PLUGIN_NAME => Ok(AuthPlugin::Sha2),
            _ => Err(ProtocolError::UnknownAuthPlugin(name.to_vec())),
        }
    }

    pub const fn as_bytes(&self) -> &[u8] {
        match self {
            AuthPlugin::Clear => MYSQL_CLEAR_PASSWORD_PLUGIN_NAME,
            AuthPlugin::Native => MYSQL_NATIVE_PASSWORD_PLUGIN_NAME,
            AuthPlugin::Sha2 => CACHING_SHA2_PASSWORD_PLUGIN_NAME,
        }
    }

    /// Generates auth plugin data for this plugin.
    ///
    /// It'll generate `None` if password is empty.
    ///
    /// Note that you should trim terminating null character from the `nonce`.
    pub fn gen_data(
        &self,
        pass: &str,
        nonce: &[u8],
        options: &Arc<ConnectionOptions>,
    ) -> Result<Option<AuthPluginData>, Error> {
        use crate::utils::scramble_native;

        Ok(match self {
            AuthPlugin::Clear => {
                if !options.allow_cleartext_password || options.secure_auth {
                    return Err(RuntimeError::InsecureAuth.into());
                }
                Some(AuthPluginData::Clear(pass.as_bytes().to_vec()))
            }
            AuthPlugin::Native => {
                if options.secure_auth {
                    return Err(RuntimeError::InsecureAuth.into());
                }
                scramble_native(nonce, pass.as_bytes()).map(AuthPluginData::Native)
            }
            AuthPlugin::Sha2 => unimplemented!(concat!(
                "caching_sha2_password_auth is not yet implemented.\n",
                "You can change the auth method of an user by running `alter user \"user\"@\"host\" identified with mysql_native_password by \"password\"`.\n",
                "To list all users, you can run `select user, host from mysql.user`.\n"
            )),
        })
    }
}

#[derive(Debug, Clone)]
pub enum AuthPluginData {
    /// Auth data for `mysql_old_password` plugin.
    Old([u8; 8]),
    /// Clear password for `mysql_clear_password` plugin.
    Clear(Vec<u8>),
    /// Auth data for `mysql_native_password` plugin.
    Native([u8; 20]),
    /// Auth data for `sha2_password` and `caching_sha2_password` plugins.
    Sha2([u8; 32]),
}

impl std::ops::Deref for AuthPluginData {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Old(x) => &x[..],
            Self::Clear(x) => &x[..],
            Self::Native(x) => &x[..],
            Self::Sha2(x) => &x[..],
        }
    }
}

impl Serialize for AuthPluginData {
    fn serialize(&self, buf: &mut Vec<u8>) {
        match self {
            Self::Old(x) => {
                buf.put_slice(x);
                buf.push(0);
            }
            Self::Clear(x) => {
                buf.put_slice(x);
                buf.push(0);
            }
            Self::Native(x) => buf.put_slice(x),
            Self::Sha2(x) => buf.put_slice(x),
        }
    }
}
