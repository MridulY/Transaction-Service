use hmac::{Hmac, Mac};
use sha2::Sha256;
use hex;

type HmacSha256 = Hmac<Sha256>;

/// Generates an HMAC-SHA256 signature for webhook payloads
///
/// This ensures webhook authenticity by allowing recipients to verify
/// that the webhook was sent by the transaction service.
///
/// # Arguments
/// * `secret` - The webhook secret key
/// * `payload` - The JSON payload to sign
/// * `timestamp` - Unix timestamp to prevent replay attacks
///
/// # Returns
/// The hex-encoded HMAC signature
pub fn generate_signature(secret: &str, payload: &str, timestamp: i64) -> Result<String, String> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| format!("Invalid secret key: {}", e))?;

    // Include timestamp in signature to prevent replay attacks
    let message = format!("{}.{}", timestamp, payload);
    mac.update(message.as_bytes());

    let result = mac.finalize();
    let signature = hex::encode(result.into_bytes());

    Ok(signature)
}

/// Verifies an HMAC-SHA256 signature
///
/// # Arguments
/// * `secret` - The webhook secret key
/// * `payload` - The JSON payload
/// * `timestamp` - Unix timestamp from the webhook
/// * `signature` - The signature to verify
///
/// # Returns
/// true if signature is valid, false otherwise
pub fn verify_signature(
    secret: &str,
    payload: &str,
    timestamp: i64,
    signature: &str,
) -> Result<bool, String> {
    let expected_signature = generate_signature(secret, payload, timestamp)?;

    // Use constant-time comparison to prevent timing attacks
    Ok(constant_time_compare(&expected_signature, signature))
}

/// Constant-time string comparison to prevent timing attacks
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();

    let mut result = 0u8;
    for i in 0..a_bytes.len() {
        result |= a_bytes[i] ^ b_bytes[i];
    }

    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_generation() {
        let secret = "test_secret_key_12345";
        let payload = r#"{"event":"transaction.completed","data":{"id":"123"}}"#;
        let timestamp = 1640000000;

        let signature = generate_signature(secret, payload, timestamp).unwrap();

        // Signature should be 64 hex characters (32 bytes)
        assert_eq!(signature.len(), 64);
    }

    #[test]
    fn test_signature_verification_success() {
        let secret = "test_secret_key_12345";
        let payload = r#"{"event":"transaction.completed"}"#;
        let timestamp = 1640000000;

        let signature = generate_signature(secret, payload, timestamp).unwrap();
        let is_valid = verify_signature(secret, payload, timestamp, &signature).unwrap();

        assert!(is_valid);
    }

    #[test]
    fn test_signature_verification_failure() {
        let secret = "test_secret_key_12345";
        let payload = r#"{"event":"transaction.completed"}"#;
        let timestamp = 1640000000;

        let signature = generate_signature(secret, payload, timestamp).unwrap();

        // Tamper with signature
        let tampered = signature.replace('a', 'b');
        let is_valid = verify_signature(secret, payload, timestamp, &tampered).unwrap();

        assert!(!is_valid);
    }

    #[test]
    fn test_signature_different_timestamps() {
        let secret = "test_secret_key_12345";
        let payload = r#"{"event":"transaction.completed"}"#;

        let sig1 = generate_signature(secret, payload, 1640000000).unwrap();
        let sig2 = generate_signature(secret, payload, 1640000001).unwrap();

        // Different timestamps should produce different signatures
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("abc123", "abc123"));
        assert!(!constant_time_compare("abc123", "abc124"));
        assert!(!constant_time_compare("abc123", "abc12"));
        assert!(!constant_time_compare("abc", "abc123"));
    }
}
