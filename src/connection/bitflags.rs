use {
    crate::{
        error::{InvalidFlags, ProtocolError},
        Deserialize, Serialize,
    },
    std::convert::TryFrom,
};

macro_rules! bitflags {
    (
        $name:ident: $ty:ty {
            $($body:tt)*
        }
    ) => {
        paste::paste! {
            impl TryFrom<$ty> for [< $name Flags >] {
                type Error = InvalidFlags;

                fn try_from(value: $ty) -> std::result::Result<[< $name Flags >], InvalidFlags> {
                    [< $name Flags >]::from_bits(value).ok_or_else(|| InvalidFlags::$name(value))
                }
            }

            impl From<[< $name Flags >]> for $ty {
                fn from(value: [< $name Flags >]) -> Self {
                    value.bits()
                }
            }

            impl Default for [< $name Flags >] {
                fn default() -> [< $name Flags >] {
                    [< $name Flags >]::empty()
                }
            }

            impl Serialize for [< $name Flags >] {
                fn serialize(&self, buf: &mut Vec<u8>) {
                    self.bits().serialize(buf)
                }
            }

            impl<'de> Deserialize<'de> for [< $name Flags >] {
                const SIZE: Option<usize> = <$ty as Deserialize<'de>>::SIZE;
                type Ctx = <$ty as Deserialize<'de>>::Ctx;

                fn deserialize(buf: &mut crate::ParseBuf<'de>, ctx: Self::Ctx) -> Result<Self, ProtocolError> {
                    let val = <$ty>::deserialize(buf, ctx)?;
                    Self::try_from(val).map_err(Into::into)
                }
            }

            bitflags::bitflags! {
                #[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
                pub struct [< $name Flags >]: $ty {
                    $($body)*
                }
            }
        }
    };
}

bitflags! {
    Status: u16 {
        /// Is raised when a multi-statement transaction has been started, either explicitly,
        /// by means of BEGIN or COMMIT AND CHAIN, or implicitly, by the first transactional
        /// statement, when autocommit=off.
        const IN_TRANS             = 0x0001;

        /// Server in auto_commit mode.
        const AUTOCOMMIT           = 0x0002;

        /// Multi query - next query exists.
        const MORE_RESULTS_EXISTS         = 0x0008;

        const NO_GOOD_INDEX_USED   = 0x0010;

        const NO_INDEX_USED        = 0x0020;

        /// The server was able to fulfill the clients request and opened a read-only
        /// non-scrollable cursor for a query. This flag comes in reply to COM_STMT_EXECUTE
        /// and COM_STMT_FETCH commands. Used by Binary Protocol Resultset to signal that
        /// COM_STMT_FETCH must be used to fetch the row-data.
        const CURSOR_EXISTS        = 0x0040;

        /// This flag is sent when a read-only cursor is exhausted, in reply to
        /// COM_STMT_FETCH command.
        const LAST_ROW_SENT        = 0x0080;

        const DB_DROPPED           = 0x0100;

        const NO_BACKSLASH_ESCAPES = 0x0200;

        /// Sent to the client if after a prepared statement reprepare we discovered
        /// that the new statement returns a different number of result set columns.
        const METADATA_CHANGED     = 0x0400;

        const QUERY_WAS_SLOW              = 0x0800;

        /// To mark ResultSet containing output parameter values.
        const PS_OUT_PARAMS               = 0x1000;

        /// Set at the same time as SERVER_STATUS_IN_TRANS if the started multi-statement
        /// transaction is a read-only transaction. Cleared when the transaction commits
        /// or aborts. Since this flag is sent to clients in OK and EOF packets, the flag
        /// indicates the transaction status at the end of command execution.
        const IN_TRANS_READONLY    = 0x2000;

        /// This status flag, when on, implies that one of the state information has
        /// changed on the server because of the execution of the last statement.
        const SESSION_STATE_CHANGED       = 0x4000;

        /// Introduced by mariadb. Contain the information about ANSI_QUOTES SQL_MODE.
        const ANSI_QUOTES          = 0x8000;
    }
}

bitflags! {
    Capability: u32 {
        /// Use the improved version of Old Password Authentication. Assumed to be set since 4.1.1.
        const LONG_PASSWORD                  = 0x0000_0001;

        /// Send found rows instead of affected rows in EOF_Packet.
        const FOUND_ROWS                     = 0x0000_0002;

        /// Get all column flags.
        /// Longer flags in Protocol::ColumnDefinition320.
        ///
        /// ### Server
        /// Supports longer flags.
        ///
        /// ### Client
        /// Expects longer flags.
        const LONG_FLAG                      = 0x0000_0004;

        /// Database (schema) name can be specified on connect in Handshake Response Packet.
        /// ### Server
        /// Supports schema-name in Handshake Response Packet.
        ///
        /// ### Client
        /// Handshake Response Packet contains a schema-name.
        const CONNECT_WITH_DB                = 0x0000_0008;

        /// Don't allow database.table.column.
        const NO_SCHEMA                      = 0x0000_0010;

        /// Compression protocol supported.
        ///
        /// ### Server
        /// Supports compression.
        ///
        /// ### Client
        /// Switches to compressed protocol after successful authentication.
        const COMPRESS                       = 0x0000_0020;

        /// Special handling of ODBC behavior.
        const ODBC                           = 0x0000_0040;

        /// Can use LOAD DATA LOCAL.
        ///
        /// ### Server
        /// Enables the LOCAL INFILE request of LOAD DATA|XML.
        ///
        /// ### Client
        /// Will handle LOCAL INFILE request.
        const LOCAL_FILES                    = 0x0000_0080;

        /// Ignore spaces before '('.
        const IGNORE_SPACE                   = 0x0000_0100;

        const PROTOCOL_41                    = 0x0000_0200;

        /// This is an interactive client.
        const INTERACTIVE                    = 0x0000_0400;

        /// Use SSL encryption for the session.
        ///
        /// ### Server
        /// Supports SSL
        ///
        /// ### Client
        /// Switch to SSL after sending the capability-flags.
        const SSL                            = 0x0000_0800;

        /// Client only flag. Not used.
        ///
        /// ### Client
        /// Do not issue SIGPIPE if network failures occur (libmysqlclient only).
        const IGNORE_SIGPIPE                 = 0x0000_1000;

        /// Client knows about transactions.
        ///
        /// ### Server
        /// Can send status flags in OK_Packet / EOF_Packet.
        ///
        /// ### Client
        /// Expects status flags in OK_Packet / EOF_Packet.
        ///
        /// ### Note
        /// This flag is optional in 3.23, but always set by the server since 4.0.
        const TRANSACTIONS                   = 0x0000_2000;

        const RESERVED                       = 0x0000_4000;

        const SECURE_CONNECTION              = 0x0000_8000;

        /// Enable/disable multi-stmt support.
        /// Also sets MULTI_RESULTS.
        const MULTI_STATEMENTS               = 0x0001_0000;

        /// Enable/disable multi-results.
        ///
        /// ### Server
        /// Can send multiple resultsets for COM_QUERY. Error if the server needs to send
        /// them and client does not support them.
        ///
        /// ### Client
        /// Can handle multiple resultsets for COM_QUERY.
        ///
        /// ### Requires
        /// `PROTOCOL_41`
        const MULTI_RESULTS                  = 0x0002_0000;

        /// Multi-results and OUT parameters in PS-protocol.
        ///
        /// ### Requires
        /// `PROTOCOL_41`
        const PS_MULTI_RESULTS               = 0x0004_0000;

        /// Client supports plugin authentication.
        ///
        /// ### Server
        /// Sends extra data in Initial Handshake Packet and supports the pluggable
        /// authentication protocol.
        ///
        /// ### Client
        /// Supports authentication plugins.
        ///
        /// ### Requires
        /// `PROTOCOL_41`
        const PLUGIN_AUTH                    = 0x0008_0000;

        /// Client supports connection attributes in handshake response.
        const CONNECT_ATTRS                  = 0x0010_0000;

        /// Enable authentication response packet to be larger than 255 bytes.
        /// When the ability to change default plugin require that the initial password
        /// field in the Protocol::HandshakeResponse41 paclet can be of arbitrary size.
        /// However, the 4.1 client-server protocol limits the length of the auth-data-field
        /// sent from client to server to 255 bytes. The solution is to change the type of
        /// the field to a true length encoded string and indicate the protocol change with
        /// this client capability flag.
        ///
        /// ### Server
        /// Understands length-encoded integer for auth response data in
        /// Protocol::HandshakeResponse41.
        ///
        /// ### Client
        /// Length of auth response data in Protocol::HandshakeResponse41 is a
        /// length-encoded integer.
        ///
        /// ### Note
        /// The flag was introduced in 5.6.6, but had the wrong value.
        const PLUGIN_AUTH_LENENC_CLIENT_DATA = 0x0020_0000;

        /// Don't close the connection for a user account with expired password.
        const CAN_HANDLE_EXPIRED_PASSWORDS   = 0x0040_0000;

        /// Capable of handling server state change information.
        /// Its a hint to the server to include the state change information in OK_Packet.
        ///
        /// ### Server
        /// Can set SESSION_STATE_CHANGED in the StatusFlags and send
        /// Session State Information in a OK_Packet.
        ///
        /// ### Client
        /// Expects the server to send Session State Information in a OK_Packet.
        const SESSION_TRACK                  = 0x0080_0000;

        /// Client no longer needs EOF_Packet and will use OK_Packet instead.
        ///
        /// ### Server
        /// Can send OK after a Text Resultset.
        ///
        /// ### Client
        /// Expects an OK_Packet (instead of EOF_Packet) after the resultset
        /// rows of a Text Resultset.
        ///
        /// ### Background
        /// To support SESSION_TRACK, additional information must be sent after all
        /// successful commands. Although the OK_Packet is extensible, the EOF_Packet is
        /// not due to the overlap of its bytes with the content of the Text Resultset Row.
        ///
        /// Therefore, the EOF_Packet in the Text Resultset is replaced with an OK_Packet.
        /// EOF_Packet is deprecated as of MySQL 5.7.5.
        const DEPRECATE_EOF                  = 0x0100_0000;

        /// The client can handle optional metadata information in the resultset.
        const OPTIONAL_RESULTSET_METADATA    = 0x0200_0000;

        /// Compression protocol extended to support zstd compression method.
        ///
        /// This capability flag is used to send zstd compression level between client and server
        /// provided both client and server are enabled with this flag.
        ///
        /// # Server
        ///
        /// Server sets this flag when global variable protocol-compression-algorithms has zstd
        /// in its list of supported values.
        ///
        /// # Client
        ///
        /// Client sets this flag when it is configured to use zstd compression method.
        const ZSTD_COMPRESSION_ALGORITHM     = 0x0400_0000;

        /// Support optional extension for query parameters into the COM_QUERY
        /// and COM_STMT_EXECUTE packets.
        ///
        /// # Server
        ///
        /// Expects an optional part containing the query parameter set(s).
        /// Executes the query for each set of parameters or returns an error if more than 1 set
        /// of parameters is sent and the server can't execute it.
        ///
        /// # Client
        ///
        /// Can send the optional part containing the query parameter set(s).
        const QUERY_ATTRIBUTES               = 0x0800_0000;

        /// Support Multi factor authentication.
        ///
        /// # Server
        ///
        /// Server sends AuthNextFactor packet after every nth factor
        /// authentication method succeeds, except the last factor authentication.
        ///
        /// # Client
        ///
        /// Client reads AuthNextFactor packet sent by server
        /// and initiates next factor authentication method.
        const MULTI_FACTOR_AUTHENTICATION           = 0x1000_0000;

        /// Client or server supports progress reports within error packet.
        const PROGRESS_OBSOLETE              = 0x2000_0000;

        /// Verify server certificate. Client only flag.
        ///
        /// Deprecated in favor of –ssl-mode.
        const SSL_VERIFY_SERVER_CERT         = 0x4000_0000;

        /// Don't reset the options after an unsuccessful connect. Client only flag.
        const REMEMBER_OPTIONS               = 0x8000_0000;
    }
}

bitflags! {
    CursorType: u8 {
        const NO_CURSOR  = 0_u8;
        const READ_ONLY  = 1_u8;
        const FOR_UPDATE = 2_u8;
        const SCROLLABLE = 4_u8;
    }
}

bitflags! {
    StmtExecuteParams: u8 {
        const NEW_PARAMS_BOUND  = 1_u8;
    }
}

bitflags! {
    StmtExecuteParam: u8 {
        const UNSIGNED  = 128_u8;
    }
}

bitflags! {
    Column: u16 {
        /// Field can't be NULL.
        const NOT_NULL_FLAG         = 1u16;

        /// Field is part of a primary key.
        const PRI_KEY_FLAG          = 2u16;

        /// Field is part of a unique key.
        const UNIQUE_KEY_FLAG       = 4u16;

        /// Field is part of a key.
        const MULTIPLE_KEY_FLAG     = 8u16;

        /// Field is a blob.
        const BLOB_FLAG             = 16u16;

        /// Field is unsigned.
        const UNSIGNED_FLAG         = 32u16;

        /// Field is zerofill.
        const ZEROFILL_FLAG         = 64u16;

        /// Field is binary.
        const BINARY_FLAG           = 128u16;

        /// Field is an enum.
        const ENUM_FLAG             = 256u16;

        /// Field is a autoincrement field.
        const AUTO_INCREMENT_FLAG   = 512u16;

        /// Field is a timestamp.
        const TIMESTAMP_FLAG        = 1024u16;

        /// Field is a set.
        const SET_FLAG              = 2048u16;

        /// Field doesn't have default value.
        const NO_DEFAULT_VALUE_FLAG = 4096u16;

        /// Field is set to NOW on UPDATE.
        const ON_UPDATE_NOW_FLAG    = 8192u16;

        /// Intern; Part of some key.
        const PART_KEY_FLAG         = 16384u16;

        /// Field is num (for clients).
        const NUM_FLAG              = 32768u16;
    }
}
