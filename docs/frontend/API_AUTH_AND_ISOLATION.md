# 🔐 API Authentication & SaaS Isolation (Frontend)

## 📋 Overview

The application implements a strict **Multi-tenant SaaS** model. Every request must be authenticated and is automatically scoped to a `tenant_id`.

## 🔑 Authentication

All protected API endpoints require a Bearer Token.

### API Base URLs
- **Development:** `http://localhost:8080`
- **Production (Koyeb):** `https://ministerial-yetta-fodi999-c58d8823.koyeb.app`

### Obtaining a Token

1.  **Register:** `POST /api/auth/register` (returns tokens and user info)
2.  **Login:** `POST /api/auth/login` (returns tokens and user info)
3.  **Refresh:** `POST /api/auth/refresh` (uses refresh token to get a new access token)

### Token Payload

The Access Token (JWT) contains:
- `sub`: User ID
- `tenant_id`: The ID of the restaurant/organization
- `exp`: Expiration timestamp

**Frontend Responsibility:** You do NOT need to send `X-Tenant-Id` or `X-User-Id` headers. The backend extracts these automatically from the JWT.

---

## 🌍 Language & I18n

The backend handles translations automatically. 

### How it works:
1.  **User Preference:** Every user has a `language` setting in their profile.
2.  **Automatic Fallback:** If a translation for the requested language is missing, the backend automatically falls back to **English**.
3.  **URL Overrides:** For some endpoints (like AI Insights), you can specify the language in the URL: `/api/recipes/v2/{id}/insights/{lang}`.

---

## 🏢 Tenant Isolation

Data is strictly isolated. A user from `Tenant A` can never see or modify data from `Tenant B`.

### Identifying your Tenant
The login/register response includes the `tenant_id`. You can use this for client-side routing or state, but the backend will always verify it against the JWT.

```json
{
  "access_token": "...",
  "tenant_id": "550e8400-e29b-41d4-a716-446655440000",
  "user_id": "..."
}
```

---

## 🛠 Error Handling

The API returns consistent error structures:

```json
{
  "error": "Resource not found",
  "code": "NOT_FOUND",
  "details": "Recipe with ID ... does not exist in your tenant"
}
```

### Common Status Codes:
- `401 Unauthorized`: Token missing, expired, or invalid.
- `403 Forbidden`: Authenticated but lacks permissions (e.g., trying to access another tenant's data).
- `422 Unprocessable Entity`: Validation errors (check `details` for field-specific errors).
- `500 Internal Server Error`: Unexpected backend failure.
