# Testing Guide

## Manual Testing Steps

### Prerequisites

1. Start PostgreSQL:
```bash
docker-compose up -d postgres
```

2. Wait for PostgreSQL to be ready (check health):
```bash
docker-compose ps
```

3. Start the service:
```bash
cargo run
```

The service should start on `http://localhost:3000`

---

## Test Scenarios

### 1. Health Check

```bash
curl http://localhost:3000/health
```

**Expected Response:**
```json
{
  "status": "healthy",
  "version": "0.1.0"
}
```

---

### 2. Create First Account

```bash
curl -X POST http://localhost:3000/api/v1/accounts \
  -H "Content-Type: application/json" \
  -d '{
    "business_name": "Test Company A",
    "email": "company-a@example.com",
    "currency": "USD"
  }'
```

**Expected Response:** Status 201
```json
{
  "account": {
    "id": "...",
    "business_name": "Test Company A",
    "email": "company-a@example.com",
    "balance": 0,
    "currency": "USD",
    "status": "active",
    "created_at": "...",
    "updated_at": "..."
  },
  "api_key": "sk_..."
}
```

**Save the API key!**

---

### 3. Create Second Account

```bash
curl -X POST http://localhost:3000/api/v1/accounts \
  -H "Content-Type: application/json" \
  -d '{
    "business_name": "Test Company B",
    "email": "company-b@example.com",
    "currency": "USD"
  }'
```

**Save this API key too!**

---

### 4. Test Authentication

```bash
# Replace with your actual API key
export API_KEY_A="sk_your_first_api_key"

curl http://localhost:3000/api/v1/accounts/me \
  -H "Authorization: Bearer $API_KEY_A"
```

**Expected:** Should return account details

---

### 5. Check Initial Balance

```bash
curl http://localhost:3000/api/v1/accounts/me/balance \
  -H "Authorization: Bearer $API_KEY_A"
```

**Expected Response:**
```json
{
  "account_id": "...",
  "balance": 0,
  "currency": "USD",
  "available_balance": 0
}
```

---

### 6. Credit Transaction

```bash
curl -X POST http://localhost:3000/api/v1/transactions/credit \
  -H "Authorization: Bearer $API_KEY_A" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: $(uuidgen)" \
  -d '{
    "account_id": "YOUR_ACCOUNT_ID",
    "amount": 100000,
    "currency": "USD",
    "description": "Initial deposit"
  }'
```

**Expected:** Status 201, transaction completed

---

### 7. Verify Balance After Credit

```bash
curl http://localhost:3000/api/v1/accounts/me/balance \
  -H "Authorization: Bearer $API_KEY_A"
```

**Expected:**
```json
{
  "balance": 100000,
  ...
}
```

---

### 8. Test Idempotency

Repeat the same credit transaction with the same idempotency key:

```bash
curl -X POST http://localhost:3000/api/v1/transactions/credit \
  -H "Authorization: Bearer $API_KEY_A" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: test-idempotency-123" \
  -d '{
    "account_id": "YOUR_ACCOUNT_ID",
    "amount": 50000,
    "currency": "USD",
    "description": "Test idempotency"
  }'
```

Run it twice - should return same transaction ID both times.

---

### 9. Debit Transaction

```bash
curl -X POST http://localhost:3000/api/v1/transactions/debit \
  -H "Authorization: Bearer $API_KEY_A" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: $(uuidgen)" \
  -d '{
    "account_id": "YOUR_ACCOUNT_ID",
    "amount": 30000,
    "currency": "USD",
    "description": "Service fee"
  }'
```

**Expected:** Status 201, balance should decrease

---

### 10. Test Insufficient Funds

```bash
curl -X POST http://localhost:3000/api/v1/transactions/debit \
  -H "Authorization: Bearer $API_KEY_A" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: $(uuidgen)" \
  -d '{
    "account_id": "YOUR_ACCOUNT_ID",
    "amount": 999999999,
    "currency": "USD",
    "description": "Should fail"
  }'
```

**Expected:** Status 400
```json
{
  "error": {
    "code": "INSUFFICIENT_FUNDS",
    "message": "Account does not have sufficient balance",
    "details": {
      "required": 999999999,
      "available": 70000
    }
  }
}
```

---

### 11. Transfer Between Accounts

```bash
export API_KEY_B="sk_your_second_api_key"
export ACCOUNT_A_ID="first_account_id"
export ACCOUNT_B_ID="second_account_id"

# Transfer from A to B
curl -X POST http://localhost:3000/api/v1/transactions/transfer \
  -H "Authorization: Bearer $API_KEY_A" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: $(uuidgen)" \
  -d '{
    "from_account_id": "'$ACCOUNT_A_ID'",
    "to_account_id": "'$ACCOUNT_B_ID'",
    "amount": 25000,
    "currency": "USD",
    "description": "Payment to Company B"
  }'
```

**Expected:** Status 201, both balances updated atomically

---

### 12. Verify Transfer

Check both account balances:

```bash
# Account A balance
curl http://localhost:3000/api/v1/accounts/me/balance \
  -H "Authorization: Bearer $API_KEY_A"

# Account B balance
curl http://localhost:3000/api/v1/accounts/me/balance \
  -H "Authorization: Bearer $API_KEY_B"
```

**Expected:** A should have 25000 less, B should have 25000 more

---

### 13. List Transactions

```bash
curl "http://localhost:3000/api/v1/transactions?limit=10&offset=0" \
  -H "Authorization: Bearer $API_KEY_A"
```

**Expected:** List of all transactions for account A

---

### 14. Get Specific Transaction

```bash
export TRANSACTION_ID="transaction_id_from_previous_response"

curl "http://localhost:3000/api/v1/transactions/$TRANSACTION_ID" \
  -H "Authorization: Bearer $API_KEY_A"
```

**Expected:** Transaction details

---

### 15. Create Webhook

```bash
curl -X POST http://localhost:3000/api/v1/webhooks \
  -H "Authorization: Bearer $API_KEY_A" \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://webhook.site/unique-url",
    "secret": "my-secure-secret-key-123456",
    "events": ["transaction.completed", "transaction.failed"]
  }'
```

**Expected:** Status 201, webhook created

**Note:** Use https://webhook.site to get a test webhook URL

---

### 16. List Webhooks

```bash
curl http://localhost:3000/api/v1/webhooks \
  -H "Authorization: Bearer $API_KEY_A"
```

**Expected:** List of webhooks

---

### 17. Delete Webhook

```bash
export WEBHOOK_ID="webhook_id_from_previous_response"

curl -X DELETE "http://localhost:3000/api/v1/webhooks/$WEBHOOK_ID" \
  -H "Authorization: Bearer $API_KEY_A"
```

**Expected:** Status 204

---

### 18. Test Unauthorized Access

```bash
# Try to access without API key
curl http://localhost:3000/api/v1/accounts/me

# Try with invalid API key
curl http://localhost:3000/api/v1/accounts/me \
  -H "Authorization: Bearer sk_invalid_key"
```

**Expected:** Status 401 for both

---

### 19. Test Cross-Account Access

```bash
# Try to credit another account
curl -X POST http://localhost:3000/api/v1/transactions/credit \
  -H "Authorization: Bearer $API_KEY_A" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: $(uuidgen)" \
  -d '{
    "account_id": "'$ACCOUNT_B_ID'",
    "amount": 10000,
    "currency": "USD"
  }'
```

**Expected:** Status 401 or 403 (Cannot credit another account)

---

### 20. Test Duplicate Email

```bash
curl -X POST http://localhost:3000/api/v1/accounts \
  -H "Content-Type: application/json" \
  -d '{
    "business_name": "Duplicate Test",
    "email": "company-a@example.com",
    "currency": "USD"
  }'
```

**Expected:** Status 400 (Email already exists)

---

## Database Verification

Connect to PostgreSQL to verify data:

```bash
docker-compose exec postgres psql -U postgres -d transaction_service
```

Run queries:

```sql
-- View all accounts
SELECT id, business_name, email, balance, currency, status FROM accounts;

-- View all transactions
SELECT id, transaction_type, amount, status, created_at FROM transactions ORDER BY created_at DESC;

-- View API keys (hashed)
SELECT id, account_id, name, is_active FROM api_keys;

-- View webhooks
SELECT id, account_id, url, events, is_active FROM webhooks;
```

---

## Load Testing

Use Apache Bench or similar tools:

```bash
# Install apache bench
# On Mac: already installed
# On Linux: apt-get install apache2-utils

# Test health endpoint (should handle easily)
ab -n 1000 -c 10 http://localhost:3000/health

# Test authenticated endpoint
ab -n 100 -c 5 \
  -H "Authorization: Bearer $API_KEY_A" \
  http://localhost:3000/api/v1/accounts/me/balance
```

---

## Performance Benchmarks

Expected performance:
- Health check: < 5ms
- Account lookup: < 50ms
- Transaction creation: < 100ms
- Database connection pool: 20 connections
- Concurrent requests: 100+ req/sec

---

## Common Issues

### Database Connection Failed
```
Solution: Ensure PostgreSQL is running
docker-compose ps
docker-compose up -d postgres
```

### Migrations Failed
```
Solution: Check database URL and permissions
Check logs: cargo run
```

### API Key Not Working
```
Solution: Ensure you're using the full key including 'sk_' prefix
Check Authorization header format
```

### Transaction Failed
```
Solution: Check account balance
Verify account ID matches authenticated account
Ensure amount is positive integer
```

---

## Cleanup

Stop and remove all data:

```bash
# Stop services
docker-compose down

# Remove volumes (WARNING: deletes all data)
docker-compose down -v

# Remove built images
docker-compose down --rmi all
```

---

## Automated Test Suite

To run automated tests (when implemented):

```bash
cargo test
cargo test --test integration_tests
cargo test -- --nocapture  # Show output
```

---

## Summary Checklist

- [ ] Health check works
- [ ] Can create accounts
- [ ] API authentication works
- [ ] Can check balances
- [ ] Credit transactions work
- [ ] Debit transactions work
- [ ] Transfer transactions work
- [ ] Idempotency prevents duplicates
- [ ] Insufficient funds is rejected
- [ ] Cross-account access is blocked
- [ ] Can list transactions
- [ ] Can create webhooks
- [ ] Can list/delete webhooks
- [ ] Unauthorized requests are rejected
- [ ] Database constraints work
- [ ] Performance meets targets

---

## Next Steps

1. Implement actual webhook delivery
2. Add rate limiting enforcement
3. Complete OpenTelemetry integration
4. Add integration test suite
5. Set up CI/CD pipeline
6. Deploy to staging environment
