use aes_siv::{siv::Aes128Siv as Siv, Key};

pub mod cert;

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Incorrect key length, expected 32 byte key")]
    IncorrectKeyLength,
    #[error("Encryption failed")]
    Encrypt,
}

pub fn generate_nonce() -> [u8; 32] {
    use nanorand::rand::Rng;
    let mut nonce = [0; 32];
    let mut rng = nanorand::rand::ChaCha8::new();
    rng.fill_bytes(&mut nonce);
    nonce
}

pub fn encryption_key(secret: &[u8], public: &[u8], nonce: &[u8]) -> Result<Vec<u8>, CryptoError> {
    println!("secret key: {}", hex::encode(secret));
    println!("public key: {}", hex::encode(public));
    // initial key material
    let ikm = x25519_dalek::x25519(
        secret
            .try_into()
            .map_err(|_| CryptoError::IncorrectKeyLength)?,
        public
            .try_into()
            .map_err(|_| CryptoError::IncorrectKeyLength)?,
    );
    println!("ikm key: {}", hex::encode(&ikm));
    println!("nonce: {}", hex::encode(nonce));
    let ikm_nonce = &[&ikm, nonce].concat();
    println!("ikm:nonce: {}", hex::encode(ikm_nonce));
    // perform key expansion
    let mut key = [0u8; 32];
    hkdf::Hkdf::<sha2::Sha256>::new(Some(&HKDF_SALT), ikm_nonce)
        .expand(&[], &mut key)
        .map_err(|_| CryptoError::IncorrectKeyLength)?;
    println!("encryption key: {}", hex::encode(&key));
    Ok(key.to_vec())
}

pub fn encrypt(
    secret: &[u8],
    public: &[u8],
    plaintext: &[u8],
    nonce: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let mut cipher = Siv::new(*Key::from_slice(secret));

    let ciphertext = cipher
        .encrypt(&[[]], plaintext)
        .map_err(|_| CryptoError::Encrypt)?;

    let ciphertext = [nonce, public, &ciphertext].concat();

    Ok(ciphertext)
}

static HKDF_SALT: [u8; 32] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x4b, 0xea, 0xd8, 0xdf, 0x69, 0x99,
    0x08, 0x52, 0xc2, 0x02, 0xdb, 0x0e, 0x00, 0x97, 0xc1, 0xa1, 0x2e, 0xa6, 0x37, 0xd7, 0xe9, 0x6d,
];
