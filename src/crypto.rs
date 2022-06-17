use aes_siv::{siv::Aes128Siv as Siv, Key as SivKey};

pub mod cert;

const KEY_LEN: usize = 256 / 8; // 32 bytes
const NONCE_LEN: usize = 32;

pub type Nonce = [u8; NONCE_LEN];
pub type Key = [u8; KEY_LEN];

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Incorrect key length, expected 32 byte key")]
    IncorrectKeyLength,
    #[error("Encryption failed")]
    Encrypt,
    #[error("Decryption failed")]
    Decrypt,
}

pub fn edd25519_keys(secret: &bip32::XPrv) -> (Key, Key) {
    let secret = clone_into_key(&secret.private_key().to_bytes());
    let secret = x25519_dalek::StaticSecret::from(secret);
    let public = x25519_dalek::PublicKey::from(&secret);
    (secret.to_bytes(), public.to_bytes())
}

pub fn encrypt(
    secret: &Key,
    public: &Key,
    peer: &Key,
    plaintext: &[u8],
) -> Result<(Nonce, Vec<u8>), CryptoError> {
    let nonce = generate_nonce();

    let shared_secret = encryption_key(secret, peer, &nonce)?;

    let mut cipher = Siv::new(SivKey::from(shared_secret));

    let ciphertext = cipher
        .encrypt(&[[]], plaintext)
        .map_err(|_| CryptoError::Encrypt)?;

    let ciphertext = [nonce.as_slice(), public.as_slice(), &ciphertext].concat();

    Ok((nonce, ciphertext))
}

pub fn decrypt(
    secret: &Key,
    peer: &Key,
    nonce: &Nonce,
    ciphertext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let shared_secret = encryption_key(secret, peer, &nonce)?;

    let mut cipher = Siv::new(SivKey::from(shared_secret));

    let plaintext = cipher
        .decrypt(&[[]], ciphertext)
        .map_err(|_| CryptoError::Decrypt)?;

    Ok(plaintext)
}

#[derive(Debug, Clone, Copy)]
pub struct Decrypter {
    secret: Key,
    peer: Key,
    nonce: Nonce,
}

impl Decrypter {
    pub fn new(secret: Key, peer: Key, nonce: Nonce) -> Self {
        Decrypter {
            secret,
            peer,
            nonce,
        }
    }

    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        decrypt(&self.secret, &self.peer, &self.nonce, ciphertext)
    }
}

fn generate_nonce() -> Nonce {
    use nanorand::rand::Rng;
    let mut nonce = [0; NONCE_LEN];
    let mut rng = nanorand::rand::ChaCha8::new();
    rng.fill_bytes(&mut nonce);
    nonce
}

fn encryption_key(secret: &Key, public: &Key, nonce: &Nonce) -> Result<Key, CryptoError> {
    let secret = x25519_dalek::StaticSecret::from(*secret);

    let public = x25519_dalek::PublicKey::from(*public);

    let shared = secret.diffie_hellman(&public);

    let ikm = &[shared.as_bytes(), nonce.as_slice()].concat();

    let mut key = [0u8; KEY_LEN];
    hkdf::Hkdf::<sha2::Sha256>::new(Some(&HKDF_SALT), ikm)
        .expand(&[], &mut key)
        .map_err(|_| CryptoError::IncorrectKeyLength)?;

    Ok(key)
}

pub fn clone_into_key(slice: &[u8]) -> Key {
    let mut key = Default::default();
    Key::as_mut(&mut key).clone_from_slice(slice);
    key
}

static HKDF_SALT: [u8; 32] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x4b, 0xea, 0xd8, 0xdf, 0x69, 0x99,
    0x08, 0x52, 0xc2, 0x02, 0xdb, 0x0e, 0x00, 0x97, 0xc1, 0xa1, 0x2e, 0xa6, 0x37, 0xd7, 0xe9, 0x6d,
];

#[cfg(test)]
mod test {
    use crate::account::Account;

    use super::*;

    fn gen_seed() -> [u8; 64] {
        use nanorand::rand::Rng;
        let mut seed = [0; 64];
        let mut rng = nanorand::rand::ChaCha8::new();
        rng.fill_bytes(&mut seed);
        seed
    }

    fn gen_keypair() -> (Key, Key) {
        let acc = Account::from_seed(gen_seed());
        acc.prv_pub_bytes()
    }

    fn gen_bip32_key() -> bip32::XPrv {
        bip32::XPrv::new(gen_seed()).unwrap()
    }

    #[test]
    fn x25519_works() {
        let my_key = gen_bip32_key();
        let peer_key = gen_bip32_key();

        let my_priv = clone_into_key(&my_key.private_key().to_bytes());
        let my_priv = x25519_dalek::StaticSecret::from(my_priv);
        let my_pub = x25519_dalek::PublicKey::from(&my_priv);

        let peer_priv = clone_into_key(&peer_key.private_key().to_bytes());
        let peer_priv = x25519_dalek::StaticSecret::from(peer_priv);
        let peer_pub = x25519_dalek::PublicKey::from(&peer_priv);

        let my_shared = my_priv.diffie_hellman(&peer_pub);
        let peer_shared = peer_priv.diffie_hellman(&my_pub);

        assert_eq!(my_shared.to_bytes(), peer_shared.to_bytes());
    }

    #[test]
    fn shared_encryption_key() {
        let (my_priv, my_pub) = gen_keypair();
        let (peer_priv, peer_pub) = gen_keypair();
        let nonce = generate_nonce();
        let my_shared_key = encryption_key(&my_priv, &peer_pub, &nonce).unwrap();
        let peer_shared_key = encryption_key(&peer_priv, &my_pub, &nonce).unwrap();
        assert_eq!(my_shared_key, peer_shared_key)
    }
}
