//! Ed25519 node identity.

use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};

pub const SIGNATURE_LENGTH: usize = 64;
const KEY_LENGTH: usize = 32;
/// Versioned text format so the file can evolve (e.g. to encrypted keys).
const KEY_FILE_PREFIX: &str = "palveni-ed25519-secret-v1:";

#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("system random source failed: {0}")]
    Random(String),
    #[error("cannot access key file {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("key file {0} is not a Palveni identity")]
    Malformed(PathBuf),
    #[error("`{0}` is not a valid node ID")]
    InvalidNodeId(String),
    #[error("the paired server at {0} has a different node ID")]
    PinMismatch(PathBuf),
}

/// Public identity of a node: its Ed25519 verifying key.
///
/// Textual form: 64 lowercase hex characters.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId([u8; KEY_LENGTH]);

impl NodeId {
    /// Validates that `bytes` is a usable Ed25519 public key.
    pub fn from_bytes(bytes: [u8; KEY_LENGTH]) -> Result<NodeId, IdentityError> {
        VerifyingKey::from_bytes(&bytes)
            .map(|_| NodeId(bytes))
            .map_err(|_| IdentityError::InvalidNodeId(hex::encode(bytes)))
    }

    /// Remembers the authenticated server ID without replacing an earlier pairing.
    pub fn pin(&self, path: &Path) -> Result<(), IdentityError> {
        let matches = |stored| {
            if stored == *self {
                Ok(())
            } else {
                Err(IdentityError::PinMismatch(path.to_owned()))
            }
        };
        match Self::load_pin(path) {
            Ok(stored) => return matches(stored),
            Err(IdentityError::Io { source, .. })
                if source.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }

        let io_err = |source| IdentityError::Io {
            path: path.to_owned(),
            source,
        };
        if let Some(dir) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(dir).map_err(io_err)?;
        }
        let mut nonce = [0u8; 16];
        getrandom::fill(&mut nonce).map_err(|e| IdentityError::Random(e.to_string()))?;
        let temporary = path.with_extension(format!("{}.pin-tmp", hex::encode(nonce)));
        let mut file = fs::File::create_new(&temporary).map_err(io_err)?;
        let write = writeln!(file, "{self}").and_then(|()| file.sync_all());
        drop(file);
        // Publish a fully written pin only if another login has not already pinned a server.
        let publish = write.and_then(|()| fs::hard_link(&temporary, path));
        let _ = fs::remove_file(temporary);
        match publish {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                matches(Self::load_pin(path)?)
            }
            Err(error) => Err(io_err(error)),
        }
    }

    pub fn load_pin(path: &Path) -> Result<Self, IdentityError> {
        let value = fs::read_to_string(path).map_err(|source| IdentityError::Io {
            path: path.to_owned(),
            source,
        })?;
        value
            .trim()
            .parse()
            .map_err(|_| IdentityError::Malformed(path.to_owned()))
    }

    pub fn as_bytes(&self) -> &[u8; KEY_LENGTH] {
        &self.0
    }

    /// Verifies `signature` over `message` made by this node.
    pub fn verify(&self, message: &[u8], signature: &[u8; SIGNATURE_LENGTH]) -> bool {
        let Ok(key) = VerifyingKey::from_bytes(&self.0) else {
            return false;
        };
        let Ok(signature) = Signature::try_from(&signature[..]) else {
            return false;
        };
        key.verify_strict(message, &signature).is_ok()
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex::encode(self.0))
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId({self})")
    }
}

impl FromStr for NodeId {
    type Err = IdentityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let text = s.trim();
        let mut bytes = [0u8; KEY_LENGTH];
        hex::decode_to_slice(text, &mut bytes)
            .map_err(|_| IdentityError::InvalidNodeId(text.to_owned()))?;
        NodeId::from_bytes(bytes)
    }
}

/// A node's private key. Never printed; zeroised on drop by ed25519-dalek.
pub struct NodeIdentity {
    signing: SigningKey,
}

impl NodeIdentity {
    pub fn generate() -> Result<NodeIdentity, IdentityError> {
        let mut secret = [0u8; KEY_LENGTH];
        getrandom::fill(&mut secret).map_err(|e| IdentityError::Random(e.to_string()))?;
        let identity = NodeIdentity::from_secret_bytes(&secret);
        secret.fill(0);
        Ok(identity)
    }

    pub fn from_secret_bytes(secret: &[u8; KEY_LENGTH]) -> NodeIdentity {
        NodeIdentity {
            signing: SigningKey::from_bytes(secret),
        }
    }

    #[cfg(feature = "quic")]
    pub(crate) fn iroh_secret(&self) -> iroh::SecretKey {
        iroh::SecretKey::from_bytes(self.signing.as_bytes())
    }

    pub fn node_id(&self) -> NodeId {
        NodeId(self.signing.verifying_key().to_bytes())
    }

    pub fn sign(&self, message: &[u8]) -> [u8; SIGNATURE_LENGTH] {
        let signature: Signature = Signer::sign(&self.signing, message);
        signature.to_bytes()
    }

    /// Loads the identity at `path`, creating and persisting a new one if
    /// the file does not exist. Returns whether it was created.
    pub fn load_or_generate(path: &Path) -> Result<(NodeIdentity, bool), IdentityError> {
        match fs::metadata(path) {
            Ok(_) => Ok((NodeIdentity::load(path)?, false)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                let identity = NodeIdentity::generate()?;
                identity.save(path)?;
                Ok((identity, true))
            }
            Err(source) => Err(IdentityError::Io {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    pub fn load(path: &Path) -> Result<NodeIdentity, IdentityError> {
        let io_err = |source| IdentityError::Io {
            path: path.to_path_buf(),
            source,
        };
        let text = fs::read_to_string(path).map_err(io_err)?;
        restrict_permissions(path);
        let encoded = text
            .trim()
            .strip_prefix(KEY_FILE_PREFIX)
            .ok_or_else(|| IdentityError::Malformed(path.to_path_buf()))?;
        let mut secret = [0u8; KEY_LENGTH];
        hex::decode_to_slice(encoded, &mut secret)
            .map_err(|_| IdentityError::Malformed(path.to_path_buf()))?;
        let identity = NodeIdentity::from_secret_bytes(&secret);
        secret.fill(0);
        Ok(identity)
    }

    /// Writes the key atomically (temp file + rename) with owner-only
    /// permissions on Unix.
    pub fn save(&self, path: &Path) -> Result<(), IdentityError> {
        let io_err = |source| IdentityError::Io {
            path: path.to_path_buf(),
            source,
        };
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            fs::create_dir_all(dir).map_err(io_err)?;
        }
        let mut tmp_name = path.as_os_str().to_owned();
        tmp_name.push(".tmp");
        let tmp = PathBuf::from(tmp_name);
        // A stale temp file from an interrupted save would block create_new.
        let _ignored = fs::remove_file(&tmp);

        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let contents = format!(
            "{KEY_FILE_PREFIX}{}\n",
            hex::encode(self.signing.to_bytes())
        );
        let mut file = options.open(&tmp).map_err(io_err)?;
        file.write_all(contents.as_bytes()).map_err(io_err)?;
        file.sync_all().map_err(io_err)?;
        drop(file);
        fs::rename(&tmp, path).map_err(io_err)?;
        Ok(())
    }
}

impl fmt::Debug for NodeIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeIdentity")
            .field("node_id", &self.node_id())
            .finish_non_exhaustive()
    }
}

/// Tightens a key file that is readable by group or others (e.g. after
/// being copied around). Best effort: some NAS filesystems ignore modes.
#[cfg(unix)]
fn restrict_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = fs::metadata(path)
        && meta.permissions().mode() & 0o077 != 0
    {
        let _ignored = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
}

/// Windows protects the file through the data directory's ACL.
#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify() {
        let identity = NodeIdentity::generate().unwrap();
        let id = identity.node_id();
        let signature = identity.sign(b"hello");
        assert!(id.verify(b"hello", &signature));
        assert!(!id.verify(b"hellO", &signature));
        let other = NodeIdentity::generate().unwrap().node_id();
        assert!(!other.verify(b"hello", &signature));
    }

    #[test]
    fn node_id_text_round_trip() {
        let id = NodeIdentity::generate().unwrap().node_id();
        let text = id.to_string();
        assert_eq!(text.len(), 64);
        assert!(
            text.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
        assert_eq!(text.parse::<NodeId>().unwrap(), id);
        assert!("zz".parse::<NodeId>().is_err());
        assert!("00".repeat(31).parse::<NodeId>().is_err());
    }

    #[test]
    fn remembered_server_survives_restart_and_cannot_be_silently_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("paired").join("server");
        let server = NodeIdentity::generate().unwrap().node_id();
        server.pin(&file).unwrap();
        server.pin(&file).unwrap();
        let other = NodeIdentity::generate().unwrap().node_id();
        assert!(matches!(
            other.pin(&file),
            Err(IdentityError::PinMismatch(_))
        ));
        assert_eq!(NodeId::load_pin(&file).unwrap(), server);
    }

    #[test]
    fn deterministic_from_secret() {
        let a = NodeIdentity::from_secret_bytes(&[7u8; 32]);
        let b = NodeIdentity::from_secret_bytes(&[7u8; 32]);
        assert_eq!(a.node_id(), b.node_id());
    }

    #[test]
    fn persist_and_reload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keys").join("node.key");
        let (created, was_new) = NodeIdentity::load_or_generate(&path).unwrap();
        assert!(was_new);
        let (loaded, was_new) = NodeIdentity::load_or_generate(&path).unwrap();
        assert!(!was_new);
        assert_eq!(created.node_id(), loaded.node_id());
    }

    #[cfg(unix)]
    #[test]
    fn key_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("node.key");
        NodeIdentity::generate().unwrap().save(&path).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn rejects_foreign_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("node.key");
        fs::write(&path, "-----BEGIN PRIVATE KEY-----\n").unwrap();
        assert!(matches!(
            NodeIdentity::load(&path),
            Err(IdentityError::Malformed(_))
        ));
    }

    #[test]
    fn debug_does_not_leak_the_secret() {
        let identity = NodeIdentity::from_secret_bytes(&[9u8; 32]);
        let debug = format!("{identity:?}");
        assert!(!debug.contains(&hex::encode([9u8; 32])));
    }
}
