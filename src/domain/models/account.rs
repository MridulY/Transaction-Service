use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::types::AccountStatus;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Account {
    pub id: Uuid,
    pub business_name: String,
    pub email: String,
    pub balance: i64,
    pub currency: String,
    pub status: AccountStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Account {
    pub fn new(business_name: String, email: String, currency: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            business_name,
            email,
            balance: 0,
            currency,
            status: AccountStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn can_debit(&self, amount: i64) -> bool {
        self.status == AccountStatus::Active && self.balance >= amount
    }

    pub fn is_active(&self) -> bool {
        self.status == AccountStatus::Active
    }
}
