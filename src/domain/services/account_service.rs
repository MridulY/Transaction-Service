use std::sync::Arc;
use uuid::Uuid;

use crate::domain::models::{generate_api_key, hash_api_key, Account, ApiKey};
use crate::domain::repositories::{AccountRepository, ApiKeyRepository};
use crate::utils::{AppError, AppResult};

pub struct AccountService {
    account_repo: Arc<dyn AccountRepository>,
    api_key_repo: Arc<dyn ApiKeyRepository>,
}

impl AccountService {
    pub fn new(
        account_repo: Arc<dyn AccountRepository>,
        api_key_repo: Arc<dyn ApiKeyRepository>,
    ) -> Self {
        Self {
            account_repo,
            api_key_repo,
        }
    }

    pub async fn create_account(
        &self,
        business_name: String,
        email: String,
        currency: String,
    ) -> AppResult<(Account, String)> {
        if self.account_repo.find_by_email(&email).await?.is_some() {
            return Err(AppError::BadRequest(
                "Account with this email already exists".to_string(),
            ));
        }

        let account = Account::new(business_name, email, currency);
        let created_account = self.account_repo.create(&account).await?;

        let api_key_str = generate_api_key();
        let key_hash = hash_api_key(&api_key_str)
            .map_err(|e| AppError::Internal(format!("Failed to hash API key: {}", e)))?;

        let api_key = ApiKey::new(created_account.id, key_hash, "Default".to_string());
        self.api_key_repo.create(&api_key).await?;

        Ok((created_account, api_key_str))
    }

    pub async fn get_account(&self, account_id: Uuid) -> AppResult<Account> {
        self.account_repo
            .find_by_id(account_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Account not found".to_string()))
    }

    pub async fn get_account_balance(&self, account_id: Uuid) -> AppResult<i64> {
        let account = self.get_account(account_id).await?;
        Ok(account.balance)
    }
}
