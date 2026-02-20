use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, SecretString};
use sha2::Sha512;

type HmacSha512 = Hmac<Sha512>;

/// Verify a Paystack webhook signature.
///
/// Paystack signs every webhook payload with HMAC-SHA512 using your secret key
/// and sends the result in the `x-paystack-signature` header.
///
/// Returns `true` if the signature matches, `false` otherwise.
pub fn verify_paystack_signature(
    webhook_secret: &SecretString,
    payload: &[u8],
    signature_header: &str,
) -> bool {
    let mut mac = match HmacSha512::new_from_slice(webhook_secret.expose_secret().as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };

    mac.update(payload);
    let computed = hex::encode(mac.finalize().into_bytes());

    // Use a constant-time comparison to prevent timing attacks
    constant_time_eq(computed.as_bytes(), signature_header.as_bytes())
}

/// Simple constant-time byte comparison to avoid timing oracle attacks
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}
