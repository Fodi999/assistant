# API Examples

## 1. Register New User and Restaurant

Creates a new tenant (restaurant) and owner user.

```bash
curl -X POST http://localhost:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "owner@restaurant.com",
    "password": "SecurePass123!",
    "restaurant_name": "The Best Restaurant",
    "owner_name": "John Doe"
  }'
```

**Response:**
```json
{
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGc...",
  "refresh_token": "550e8400-e29b-41d4-a716-446655440000",
  "token_type": "Bearer",
  "user_id": "123e4567-e89b-12d3-a456-426614174000",
  "tenant_id": "123e4567-e89b-12d3-a456-426614174001"
}
```

---

## 2. Login

Authenticate with existing credentials.

```bash
curl -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "owner@restaurant.com",
    "password": "SecurePass123!"
  }'
```

**Response:**
```json
{
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGc...",
  "refresh_token": "550e8400-e29b-41d4-a716-446655440000",
  "token_type": "Bearer",
  "user_id": "123e4567-e89b-12d3-a456-426614174000",
  "tenant_id": "123e4567-e89b-12d3-a456-426614174001"
}
```

---

## 3. Refresh Token

Get a new access token using refresh token.

```bash
curl -X POST http://localhost:8080/api/auth/refresh \
  -H "Content-Type: application/json" \
  -d '{
    "refresh_token": "550e8400-e29b-41d4-a716-446655440000"
  }'
```

**Response:**
```json
{
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGc...",
  "refresh_token": "550e8400-e29b-41d4-a716-446655440000",
  "token_type": "Bearer",
  "user_id": "123e4567-e89b-12d3-a456-426614174000",
  "tenant_id": "123e4567-e89b-12d3-a456-426614174001"
}
```

---

## 4. Get Current User (Protected)

Retrieve current authenticated user information.

```bash
curl -X GET http://localhost:8080/api/me \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGc..."
```

**Response:**
```json
{
  "user": {
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "tenant_id": "123e4567-e89b-12d3-a456-426614174001",
    "email": "owner@restaurant.com",
    "display_name": "John Doe",
    "role": "owner",
    "created_at": "2024-01-01T12:00:00.000000000Z"
  },
  "tenant": {
    "id": "123e4567-e89b-12d3-a456-426614174001",
    "name": "The Best Restaurant",
    "created_at": "2024-01-01T12:00:00.000000000Z"
  }
}
```

---

## Error Responses

### Validation Error (400)
```json
{
  "code": "VALIDATION_ERROR",
  "message": "Validation failed",
  "details": "Invalid email format"
}
```

### Authentication Error (401)
```json
{
  "code": "AUTHENTICATION_ERROR",
  "message": "Authentication failed",
  "details": "Invalid email or password"
}
```

### Not Found (404)
```json
{
  "code": "NOT_FOUND",
  "message": "Resource not found",
  "details": "User not found"
}
```

### Conflict (409)
```json
{
  "code": "CONFLICT",
  "message": "Conflict",
  "details": "User with this email already exists"
}
```

### Internal Server Error (500)
```json
{
  "code": "INTERNAL_ERROR",
  "message": "Internal server error"
}
```
