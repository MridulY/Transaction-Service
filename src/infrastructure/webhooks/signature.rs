use hex;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn generate_signature(secret: &str, payload: &str, timestamp: i64) -> Result<String, String> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| format!("Invalid secret key: {}", e))?;

    let message = format!("{}.{}", timestamp, payload);
    mac.update(message.as_bytes());

    let result = mac.finalize();
    let signature = hex::encode(result.into_bytes());

    Ok(signature)
}

pub fn verify_signature(
    secret: &str,
    payload: &str,
    timestamp: i64,
    signature: &str,
) -> Result<bool, String> {
    let expected_signature = generate_signature(secret, payload, timestamp)?;
    Ok(constant_time_compare(&expected_signature, signature))
}

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

        let mut tampered = signature.clone();
        if let Some(first_char) = tampered.chars().next() {
            let replacement = if first_char == 'a' { 'b' } else { 'a' };
            tampered.replace_range(0..1, &replacement.to_string());
        }
        let is_valid = verify_signature(secret, payload, timestamp, &tampered).unwrap();

        assert!(!is_valid);
    }

    #[test]
    fn test_signature_different_timestamps() {
        let secret = "test_secret_key_12345";
        let payload = r#"{"event":"transaction.completed"}"#;

        let sig1 = generate_signature(secret, payload, 1640000000).unwrap();
        let sig2 = generate_signature(secret, payload, 1640000001).unwrap();

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
