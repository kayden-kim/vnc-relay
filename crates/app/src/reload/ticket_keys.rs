//! Reads the ticket public key files of enabled roles (config-v4 §1.11).
//! Startup and reload share this step.
//! Design: docs/architectures/relay-access-ticket.md §4.1, §4.4.

use vnc_relay_domain::entry_point::Role;
use vnc_relay_domain::ticket::{TicketKeyError, TicketKeyList};

use crate::reload::ports::{ReloadFileStore, ReloadTicketKeyStore};
use crate::settings::{ConfigDiagnostic, ConfigError, Settings, TicketSettings};

/// Fills `keys` of each enabled ticket role and empties it for disabled roles.
///
/// Every failing role gets one `ticket_keys_invalid` diagnostic on its
/// `public_keys_file` key. When both files load but the roles share a public
/// key (config-v4 §1.6), the diagnostic goes on the Server key.
pub fn load(
    settings: &mut Settings,
    files: &dyn ReloadFileStore,
    store: &dyn ReloadTicketKeyStore,
) -> Result<(), Vec<ConfigDiagnostic>> {
    let mut diagnostics = Vec::new();
    for role in [Role::Viewer, Role::Server] {
        let ticket = settings.relay.ticket.role_mut(role);
        ticket.keys = TicketKeyList::default();
        if !ticket.enabled {
            continue;
        }
        // The config loader requires the path of an enabled role.
        let Some(path) = ticket.public_keys_file.as_deref() else {
            continue;
        };
        let loaded = files
            .read(path)
            .map_err(|_| TicketKeyError::Unreadable)
            .and_then(|bytes| store.parse(&bytes));
        match loaded {
            Ok(keys) => ticket.keys = keys,
            Err(error) => diagnostics.push(diagnostic(role, error)),
        }
    }
    let ticket = &settings.relay.ticket;
    if diagnostics.is_empty()
        && ticket.viewer.enabled
        && ticket.server.enabled
        && ticket.viewer.keys.shares_key_with(&ticket.server.keys)
    {
        diagnostics.push(diagnostic(Role::Server, TicketKeyError::SharedAcrossRoles));
    }
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(diagnostics)
    }
}

/// The config key of a role's key file, used as `key` and `config_key`.
pub fn file_key(role: Role) -> String {
    format!("{}.public_keys_file", TicketSettings::table(role))
}

fn diagnostic(role: Role, error: TicketKeyError) -> ConfigDiagnostic {
    ConfigDiagnostic {
        error: ConfigError::TicketKeysInvalid,
        key: file_key(role),
        message: error.detail().to_owned(),
    }
}

/// Test key store: each line `<kid> <byte>` makes a key whose coordinates
/// repeat `<byte>`; a file containing `invalid` fails with `Format`.
#[cfg(test)]
pub(crate) struct FakeTicketKeys;

#[cfg(test)]
impl ReloadTicketKeyStore for FakeTicketKeys {
    fn parse(&self, bytes: &[u8]) -> Result<TicketKeyList, TicketKeyError> {
        let text = std::str::from_utf8(bytes).map_err(|_| TicketKeyError::Format)?;
        let keys = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let (kid, byte) = line.split_once(' ').ok_or(TicketKeyError::Format)?;
                let byte = byte
                    .trim()
                    .parse::<u8>()
                    .map_err(|_| TicketKeyError::Format)?;
                vnc_relay_domain::ticket::TicketKey::new(kid.to_owned(), [byte; 32], [byte; 32])
            })
            .collect::<Result<Vec<_>, _>>()?;
        TicketKeyList::new(keys)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    use super::*;
    use crate::relay::test_support::settings;

    #[derive(Default)]
    struct Files(Mutex<HashMap<PathBuf, Vec<u8>>>);

    impl ReloadFileStore for Files {
        fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
            self.0
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .get(path)
                .cloned()
                .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
        }
    }

    fn enable(settings: &mut Settings, role: Role, path: &str) {
        let ticket = settings.relay.ticket.role_mut(role);
        ticket.enabled = true;
        ticket.issuer = Some("https://issuer.example".to_owned());
        ticket.relay_host = Some("relay.example".to_owned());
        ticket.public_keys_file = Some(PathBuf::from(path));
    }

    fn files(entries: &[(&str, &str)]) -> Files {
        let files = Files::default();
        for (path, text) in entries {
            files
                .0
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .insert(PathBuf::from(path), text.as_bytes().to_vec());
        }
        files
    }

    #[test]
    fn fills_enabled_roles_and_clears_disabled_ones() {
        let mut settings = settings();
        enable(&mut settings, Role::Viewer, "viewer.jwks");
        settings.relay.ticket.server.public_keys_file = Some(PathBuf::from("server.jwks"));
        let store = files(&[("viewer.jwks", "a 1\nb 2"), ("server.jwks", "c 3")]);
        assert_eq!(load(&mut settings, &store, &FakeTicketKeys), Ok(()));
        assert_eq!(settings.relay.ticket.viewer.keys.len(), 2);
        assert!(settings.relay.ticket.server.keys.is_empty());
    }

    #[test]
    fn reports_each_failing_role_on_its_file_key() {
        let mut settings = settings();
        enable(&mut settings, Role::Viewer, "viewer.jwks");
        enable(&mut settings, Role::Server, "server.jwks");
        let store = files(&[("viewer.jwks", "invalid")]);
        let diagnostics = load(&mut settings, &store, &FakeTicketKeys)
            .err()
            .unwrap_or_else(|| panic!("both roles fail"));
        let keys: Vec<_> = diagnostics.iter().map(|d| d.key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "relay.ticket.viewer.public_keys_file",
                "relay.ticket.server.public_keys_file"
            ]
        );
        assert!(
            diagnostics
                .iter()
                .all(|d| d.error == ConfigError::TicketKeysInvalid)
        );
        assert_eq!(diagnostics[0].message, TicketKeyError::Format.detail());
        assert_eq!(diagnostics[1].message, TicketKeyError::Unreadable.detail());
    }

    #[test]
    fn rejects_a_public_key_shared_by_both_roles() {
        let mut settings = settings();
        enable(&mut settings, Role::Viewer, "viewer.jwks");
        enable(&mut settings, Role::Server, "server.jwks");
        let store = files(&[("viewer.jwks", "a 1"), ("server.jwks", "other 1")]);
        let diagnostics = load(&mut settings, &store, &FakeTicketKeys)
            .err()
            .unwrap_or_else(|| panic!("shared key"));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].key, "relay.ticket.server.public_keys_file");
        assert_eq!(
            diagnostics[0].message,
            TicketKeyError::SharedAcrossRoles.detail()
        );
    }
}
