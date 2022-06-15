use super::Key;

#[derive(Debug, thiserror::Error)]
pub enum MalformedError {
    #[error("The DER byte buffer is too short")]
    IncorrectLength,
    #[error("The Netscape Comment is invalid utf8: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("The Netscape Comment has invalid base64 encoding: {0}")]
    Base64(#[from] base64::DecodeError),
}

// https://github.com/scrtlabs/SecretNetwork/blob/19bbd80307b4d6b49f04ad5c62008a3f25ba3f1e/cosmwasm/enclaves/execute/src/registration/cert.rs#L213
fn extract_asn1_value<'a, 'b>(cert: &'a [u8], oid: &'b [u8]) -> Result<&'a [u8], MalformedError> {
    let mut offset = cert
        .windows(oid.len())
        .position(|window| window == oid)
        .ok_or(MalformedError::IncorrectLength)?;

    offset += 12; // 11 + TAG (0x04)

    if offset + 2 >= cert.len() {
        return Err(MalformedError::IncorrectLength);
    }

    // Obtain Netscape Comment length
    let mut len = cert[offset] as usize;
    if len > 0x80 {
        len = (cert[offset + 1] as usize) * 0x100 + (cert[offset + 2] as usize);
        offset += 2;
    }

    // Obtain Netscape Comment
    offset += 1;

    if offset + len >= cert.len() {
        return Err(MalformedError::IncorrectLength);
    }

    Ok(&cert[offset..offset + len])
}

fn extract_netscape_comment(cert_der: &[u8]) -> Result<&[u8], MalformedError> {
    // Search for Netscape Comment OID
    let ns_cmt_oid = &[
        0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x86, 0xF8, 0x42, 0x01, 0x0D,
    ];
    extract_asn1_value(cert_der, ns_cmt_oid)
}

pub(crate) fn consenus_io_pubk(cert_der: &[u8]) -> Result<Key, MalformedError> {
    // localsecret used software SGX so we can just deserialise the payload:
    // https://github.com/scrtlabs/SecretNetwork/blob/19bbd80307b4d6b49f04ad5c62008a3f25ba3f1e/x/registration/remote_attestation/remote_attestation.go#L25
    let buf = extract_netscape_comment(cert_der)?;
    let b64 = std::str::from_utf8(buf)?;
    let pubk = base64::decode(b64)?;

    if pubk.len() < super::KEY_LEN {
        return Err(MalformedError::IncorrectLength);
    }

    let pubk = super::clone_into_key(&pubk);

    Ok(pubk)
}
