# Transaction Service - Design Specification

## Table of Contents
1. [System Overview](#system-overview)
2. [Architecture](#architecture)
3. [Database Schema](#database-schema)
4. [API Design](#api-design)
5. [Security Considerations](#security-considerations)
6. [Webhook System](#webhook-system)
7. [Operational Considerations](#operational-considerations)
8. [Trade-offs and Design Decisions](#trade-offs-and-design-decisions)

## System Overview

The Transaction Service is a production-grade payment processing system that enables businesses to manage accounts, process transactions (credit, debit, transfer), and receive real-time webhook notifications. The system is built with Rust using the Axum web framework, focusing on reliability, security, and scalability.

### Core Features
- **API Authentication**: Secure API key-based authentication
- **Account Management**: Create and manage business accounts with balance tracking
- **Transaction Processing**: Atomic credit, debit, and transfer operations
- **Webhook System**: Reliable webhook delivery with retry logic and signature verification
- **Idempotency**: Prevent duplicate transactions using idempotency keys
- **Rate Limiting**: Per-API-key rate limiting to prevent abuse
- **Observability**: OpenTelemetry integration for logs, traces, and metrics

## Architecture

### High-Level Architecture

```
┌─────────────────┐
│   API Gateway   │
│    (Axum)       │
└────────┬────────┘
         │
    ┌────┴────────────────────────┐
    │                              │
┌───┴──────┐              ┌───────┴────────┐
│Middleware│              │   Application   │
│  Layer   │              │      Layer      │
├──────────┤              ├─────────────────┤
│- Auth    │              │- Account Svc    │
│- RateLimit│             │- Transaction    │
│- Idempotency│           │  Svc            │
│- RequestID│             │- Webhook Svc    │
└─────┬────┘              └────────┬────────┘
      │                            │
      └──────────┬─────────────────┘
                 │
         ┌───────┴─────────┐
         │   Infrastructure│
         ├─────────────────┤
         │- PostgreSQL     │
         │- Redis (cache)  │
         │- HTTP Client    │
         │- Telemetry      │
         └─────────────────┘
```

### Layer Architecture

1. **API Layer** (`src/api/`)
   - HTTP handlers for all endpoints
   - Request/response DTOs
   - Route definitions

2. **Domain Layer** (`src/domain/`)
   - Business models and logic
   - Service interfaces
   - Repository traits
   - Domain-specific validations

3. **Infrastructure Layer** (`src/infrastructure/`)
   - Database implementations
   - External service integrations
   - Webhook dispatcher
   - Telemetry setup

4. **Middleware Layer** (`src/middleware/`)
   - Authentication
   - Rate limiting
   - Idempotency checking
   - Request ID generation

## Database Schema

### Tables

#### `accounts`
Stores business account information.

```sql
CREATE TABLE accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    business_name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    balance BIGINT NOT NULL DEFAULT 0, -- Stored in cents to avoid floating point issues
    currency VARCHAR(3) NOT NULL DEFAULT 'USD',
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT balance_non_negative CHECK (balance >= 0),
    CONSTRAINT valid_currency CHECK (currency IN ('USD', 'EUR', 'GBP'))
);

CREATE INDEX idx_accounts_email ON accounts(email);
CREATE INDEX idx_accounts_status ON accounts(status);
```

#### `api_keys`
Stores API keys for authentication.

```sql
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    key_hash VARCHAR(255) NOT NULL UNIQUE, -- Bcrypt hash of the API key
    name VARCHAR(100) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,

    CONSTRAINT valid_expiry CHECK (expires_at IS NULL OR expires_at > created_at)
);

CREATE INDEX idx_api_keys_key_hash ON api_keys(key_hash);
CREATE INDEX idx_api_keys_account_id ON api_keys(account_id);
```

#### `transactions`
Records all financial transactions.

```sql
CREATE TABLE transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    idempotency_key VARCHAR(255) UNIQUE,
    transaction_type VARCHAR(20) NOT NULL, -- 'credit', 'debit', 'transfer'
    from_account_id UUID REFERENCES accounts(id),
    to_account_id UUID REFERENCES accounts(id),
    amount BIGINT NOT NULL,
    currency VARCHAR(3) NOT NULL DEFAULT 'USD',
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    description TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,

    CONSTRAINT amount_positive CHECK (amount > 0),
    CONSTRAINT valid_transaction_type CHECK (transaction_type IN ('credit', 'debit', 'transfer')),
    CONSTRAINT valid_status CHECK (status IN ('pending', 'completed', 'failed', 'reversed')),
    CONSTRAINT valid_accounts CHECK (
        (transaction_type = 'credit' AND to_account_id IS NOT NULL AND from_account_id IS NULL) OR
        (transaction_type = 'debit' AND from_account_id IS NOT NULL AND to_account_id IS NULL) OR
        (transaction_type = 'transfer' AND from_account_id IS NOT NULL AND to_account_id IS NOT NULL AND from_account_id != to_account_id)
    )
);

CREATE INDEX idx_transactions_from_account ON transactions(from_account_id);
CREATE INDEX idx_transactions_to_account ON transactions(to_account_id);
CREATE INDEX idx_transactions_status ON transactions(status);
CREATE INDEX idx_transactions_created_at ON transactions(created_at DESC);
CREATE INDEX idx_transactions_idempotency_key ON transactions(idempotency_key) WHERE idempotency_key IS NOT NULL;
```

#### `webhooks`
Stores webhook endpoint configurations.

```sql
CREATE TABLE webhooks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    url VARCHAR(2048) NOT NULL,
    secret VARCHAR(255) NOT NULL, -- Used for HMAC signature
    events TEXT[] NOT NULL, -- Array of event types to subscribe to
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_webhooks_account_id ON webhooks(account_id);
CREATE INDEX idx_webhooks_is_active ON webhooks(is_active);
```

#### `webhook_deliveries`
Tracks webhook delivery attempts.

```sql
CREATE TABLE webhook_deliveries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    webhook_id UUID NOT NULL REFERENCES webhooks(id) ON DELETE CASCADE,
    transaction_id UUID NOT NULL REFERENCES transactions(id),
    event_type VARCHAR(50) NOT NULL,
    payload JSONB NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    attempt_count INT NOT NULL DEFAULT 0,
    last_attempt_at TIMESTAMPTZ,
    next_retry_at TIMESTAMPTZ,
    response_code INT,
    response_body TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_delivery_status CHECK (status IN ('pending', 'delivered', 'failed', 'exhausted'))
);

CREATE INDEX idx_webhook_deliveries_webhook_id ON webhook_deliveries(webhook_id);
CREATE INDEX idx_webhook_deliveries_transaction_id ON webhook_deliveries(transaction_id);
CREATE INDEX idx_webhook_deliveries_status ON webhook_deliveries(status);
CREATE INDEX idx_webhook_deliveries_next_retry ON webhook_deliveries(next_retry_at) WHERE status = 'pending';
```

#### `idempotency_keys`
Tracks idempotency keys to prevent duplicate operations.

```sql
CREATE TABLE idempotency_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key VARCHAR(255) NOT NULL UNIQUE,
    account_id UUID NOT NULL REFERENCES accounts(id),
    request_hash VARCHAR(64) NOT NULL, -- SHA-256 hash of request body
    response_code INT,
    response_body JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_idempotency_keys_key ON idempotency_keys(key);
CREATE INDEX idx_idempotency_keys_expires_at ON idempotency_keys(expires_at);
```

#### `rate_limits`
Tracks API request counts for rate limiting.

```sql
CREATE TABLE rate_limits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    api_key_id UUID NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    window_start TIMESTAMPTZ NOT NULL,
    request_count INT NOT NULL DEFAULT 0,

    UNIQUE(api_key_id, window_start)
);

CREATE INDEX idx_rate_limits_api_key_window ON rate_limits(api_key_id, window_start);
```

### Design Decisions

1. **Money Storage**: Amounts are stored as `BIGINT` (cents) to avoid floating-point precision issues
2. **Soft Deletes**: Not implemented initially for simplicity; accounts can be deactivated via status
3. **Audit Trail**: Complete transaction history maintained; no deletions allowed
4. **Idempotency**: Separate table for better performance and easier cleanup
5. **Webhook Reliability**: Dedicated delivery tracking table with retry state

## API Design

### Authentication

All API requests (except health check) require an API key in the header:

```
Authorization: Bearer <api_key>
```

### Base URL

```
http://localhost:3000/api/v1
```

### Endpoints

#### Health Check
```
GET /health
```

#### Account Management

**Create Account**
```
POST /accounts
Content-Type: application/json

{
  "business_name": "Acme Corp",
  "email": "finance@acme.com",
  "currency": "USD"
}

Response: 201 Created
{
  "id": "uuid",
  "business_name": "Acme Corp",
  "email": "finance@acme.com",
  "balance": 0,
  "currency": "USD",
  "status": "active",
  "created_at": "timestamp"
}
```

**Get Account**
```
GET /accounts/{account_id}

Response: 200 OK
{
  "id": "uuid",
  "business_name": "Acme Corp",
  "balance": 100000,
  "currency": "USD",
  "status": "active"
}
```

**Get Account Balance**
```
GET /accounts/{account_id}/balance

Response: 200 OK
{
  "account_id": "uuid",
  "balance": 100000,
  "currency": "USD",
  "available_balance": 100000
}
```

#### Transactions

**Create Credit Transaction**
```
POST /transactions/credit
Content-Type: application/json
Idempotency-Key: unique-key-123

{
  "account_id": "uuid",
  "amount": 50000,
  "currency": "USD",
  "description": "Payment received"
}

Response: 201 Created
{
  "id": "uuid",
  "type": "credit",
  "account_id": "uuid",
  "amount": 50000,
  "currency": "USD",
  "status": "completed",
  "description": "Payment received",
  "created_at": "timestamp"
}
```

**Create Debit Transaction**
```
POST /transactions/debit
Content-Type: application/json
Idempotency-Key: unique-key-124

{
  "account_id": "uuid",
  "amount": 30000,
  "currency": "USD",
  "description": "Service fee"
}

Response: 201 Created
```

**Create Transfer Transaction**
```
POST /transactions/transfer
Content-Type: application/json
Idempotency-Key: unique-key-125

{
  "from_account_id": "uuid1",
  "to_account_id": "uuid2",
  "amount": 25000,
  "currency": "USD",
  "description": "Transfer to vendor"
}

Response: 201 Created
```

**Get Transaction**
```
GET /transactions/{transaction_id}

Response: 200 OK
```

**List Transactions**
```
GET /transactions?account_id=uuid&limit=50&offset=0

Response: 200 OK
{
  "transactions": [...],
  "total": 150,
  "limit": 50,
  "offset": 0
}
```

#### Webhooks

**Register Webhook**
```
POST /webhooks
Content-Type: application/json

{
  "url": "https://example.com/webhook",
  "events": ["transaction.completed", "transaction.failed"],
  "secret": "your-webhook-secret"
}

Response: 201 Created
{
  "id": "uuid",
  "account_id": "uuid",
  "url": "https://example.com/webhook",
  "events": ["transaction.completed", "transaction.failed"],
  "is_active": true,
  "created_at": "timestamp"
}
```

**List Webhooks**
```
GET /webhooks

Response: 200 OK
```

**Delete Webhook**
```
DELETE /webhooks/{webhook_id}

Response: 204 No Content
```

### Error Responses

All errors follow a consistent format:

```json
{
  "error": {
    "code": "INSUFFICIENT_FUNDS",
    "message": "Account does not have sufficient balance",
    "details": {
      "required": 50000,
      "available": 30000
    }
  }
}
```

Common error codes:
- `INVALID_REQUEST`: Malformed request
- `UNAUTHORIZED`: Invalid or missing API key
- `RATE_LIMIT_EXCEEDED`: Too many requests
- `INSUFFICIENT_FUNDS`: Not enough balance
- `DUPLICATE_IDEMPOTENCY_KEY`: Idempotency key reused with different request
- `RESOURCE_NOT_FOUND`: Requested resource doesn't exist

## Security Considerations

### API Key Management

1. **Storage**: API keys are hashed using bcrypt before storage (never stored in plaintext)
2. **Generation**: 256-bit random keys encoded in base64
3. **Transmission**: Keys are only shown once during creation
4. **Rotation**: Support for multiple active keys per account for zero-downtime rotation

### Webhook Security

1. **HMAC Signatures**: All webhook payloads signed with HMAC-SHA256
   ```
   X-Webhook-Signature: sha256=<signature>
   X-Webhook-Timestamp: <unix-timestamp>
   ```

2. **Replay Protection**: Timestamp verification (reject if > 5 minutes old)

3. **TLS Required**: Only HTTPS webhook URLs accepted in production

### Transaction Security

1. **Atomic Operations**: All balance updates use database transactions with row-level locking
2. **Idempotency**: Required for all state-changing operations
3. **Balance Validation**: Database constraints prevent negative balances
4. **Audit Trail**: All transactions logged and immutable

### Rate Limiting

- 100 requests per minute per API key (configurable)
- Sliding window implementation
- Returns `429 Too Many Requests` with `Retry-After` header

## Webhook System

### Delivery Mechanism

1. **Immediate Dispatch**: Webhooks triggered immediately on transaction completion
2. **Async Processing**: Webhook delivery happens asynchronously to not block transaction processing
3. **Retry Logic**: Exponential backoff with jitter
   - Attempt 1: Immediate
   - Attempt 2: 1 minute
   - Attempt 3: 10 minutes
   - Attempt 4: 1 hour
   - Attempt 5: 6 hours
   - Maximum attempts: 5

4. **Timeout**: 30-second timeout per delivery attempt
5. **Success Criteria**: HTTP 2xx response code

### Webhook Payload Format

```json
{
  "id": "webhook-delivery-uuid",
  "event": "transaction.completed",
  "created_at": "2025-12-16T10:30:00Z",
  "data": {
    "transaction": {
      "id": "transaction-uuid",
      "type": "transfer",
      "from_account_id": "uuid1",
      "to_account_id": "uuid2",
      "amount": 50000,
      "currency": "USD",
      "status": "completed",
      "description": "Payment",
      "created_at": "timestamp",
      "completed_at": "timestamp"
    }
  }
}
```

### Event Types

- `transaction.completed`: Transaction successfully processed
- `transaction.failed`: Transaction failed
- `transaction.reversed`: Transaction was reversed

## Operational Considerations

### Observability

1. **Logging**
   - Structured JSON logs
   - Log levels: ERROR, WARN, INFO, DEBUG, TRACE
   - Request ID tracking across all operations

2. **Metrics** (Prometheus format)
   - `http_requests_total`: Total HTTP requests by method, endpoint, status
   - `http_request_duration_seconds`: Request latency histogram
   - `transaction_total`: Transactions by type and status
   - `webhook_delivery_attempts_total`: Webhook delivery attempts by status
   - `db_connection_pool_size`: Database connection pool metrics
   - `rate_limit_exceeded_total`: Rate limit violations

3. **Tracing**
   - OpenTelemetry integration
   - Distributed tracing across all operations
   - Trace IDs in logs for correlation

### Health Checks

```
GET /health
```

Returns:
- Application status
- Database connectivity
- Response time

### Database Considerations

1. **Connection Pooling**: PgPool with configurable size (default: 20)
2. **Migrations**: Automated using SQLx migrations
3. **Indexes**: Optimized for common query patterns
4. **Backup Strategy**: Regular automated backups recommended (implementation outside scope)

### Deployment

1. **Container-based**: Docker + Docker Compose
2. **Environment Configuration**: 12-factor app principles
3. **Graceful Shutdown**: Handles SIGTERM/SIGINT properly
4. **Health Checks**: K8s-compatible liveness/readiness probes

### Performance Characteristics

- **Transaction Processing**: Target < 100ms p99
- **API Response Time**: Target < 50ms p99 (excluding transaction processing)
- **Webhook Delivery**: Best-effort async, doesn't block transactions
- **Database Load**: Optimized indexes for read-heavy workloads
