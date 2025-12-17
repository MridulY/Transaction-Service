use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApiKey {
    pub id: Uuid,
    pub account_id: Uuid,
    pub key_hash: String,
    pub name: String,
    pub is_active: bool,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl ApiKey {
    pub fn new(account_id: Uuid, key_hash: String, name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            account_id,
            key_hash,
            name,
            is_active: true,
            last_used_at: None,
            created_at: Utc::now(),
            expires_at: None,
        }
    }

    pub fn is_valid(&self) -> bool {
        if !self.is_active {
            return false;
        }

        if let Some(expires_at) = self.expires_at {
            if Utc::now() > expires_at {
                return false;
            }
        }

        true
    }
}

pub fn generate_api_key() -> String {
    use base64::Engine;
    use rand::Rng;
    let random_bytes: Vec<u8> = (0..32).map(|_| rand::thread_rng().gen()).collect();
    format!(
        "sk_{}",
        base64::engine::general_purpose::STANDARD.encode(&random_bytes)
    )
}

pub fn hash_api_key(key: &str) -> Result<String, bcrypt::BcryptError> {
    bcrypt::hash(key, bcrypt::DEFAULT_COST)
}

pub fn verify_api_key(key: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    bcrypt::verify(key, hash)
}
