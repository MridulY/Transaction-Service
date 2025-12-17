use validator::ValidationError;

pub fn validate_email(email: &str) -> Result<(), ValidationError> {
    if email.contains('@') && email.len() >= 3 {
        Ok(())
    } else {
        Err(ValidationError::new("Invalid email format"))
    }
}

pub fn validate_amount(amount: i64) -> Result<(), ValidationError> {
    if amount > 0 {
        Ok(())
    } else {
        Err(ValidationError::new("Amount must be positive"))
    }
}

pub fn validate_currency(currency: &str) -> Result<(), ValidationError> {
    match currency {
        "USD" | "EUR" | "GBP" => Ok(()),
        _ => Err(ValidationError::new("Invalid currency code")),
    }
}

pub fn validate_url(url: &str) -> Result<(), ValidationError> {
    if url.starts_with("http://") || url.starts_with("https://") {
        Ok(())
    } else {
        Err(ValidationError::new("URL must start with http:// or https://"))
    }
}
