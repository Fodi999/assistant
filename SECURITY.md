# 🔐 Security Guidelines

## 🚨 Critical Security Settings

### 1️⃣ JWT_SECRET (CRITICAL)

**❌ NEVER use default values in production:**
```bash
JWT_SECRET=your-super-secret-jwt-key  # ❌ INSECURE
```

**✅ Generate a strong secret:**
```bash
openssl rand -base64 64
```

**Example result:**
```bash
JWT_SECRET=R/x7ccoRyHGedn5KuPeOCMVl94V8mlTv6vXYVpxQ7fVTFG8AayG1PId8dy0v5dqaUnLTp1HFP0ySNPLte6j1IA==
```

**⚠️ Consequences of leaked JWT_SECRET:**
- All access tokens become compromised
- Attackers can forge tokens for any user
- Complete authentication bypass
- **Action required:** Immediately rotate secret and invalidate all sessions

---

## 📋 Production Checklist

### Environment Variables

- [ ] `JWT_SECRET` - Generated with `openssl rand -base64 64` (minimum 32 bytes)
- [ ] `DATABASE_URL` - Production database with SSL (`?sslmode=require`)
- [ ] `CORS_ALLOWED_ORIGINS` - Only allow trusted frontend domains
- [ ] `RUST_LOG` - Set to `info` (not `debug` in production)
- [ ] `ACCESS_TOKEN_TTL_MINUTES` - Recommended: 15-60 minutes
- [ ] `REFRESH_TOKEN_TTL_DAYS` - Recommended: 7-30 days

### Database Security

```bash
# ✅ Production (with SSL)
DATABASE_URL=postgresql://user:password@host.neon.tech/db?sslmode=require

# ❌ Local only (no SSL)
DATABASE_URL=postgresql://localhost/restaurant
```

### CORS Configuration

```bash
# ✅ Production (specific domains)
CORS_ALLOWED_ORIGINS=https://app.example.com,https://admin.example.com

# ❌ Development only (localhost)
CORS_ALLOWED_ORIGINS=http://localhost:3000
```

---

## 🛡️ Deployment on Koyeb

### Required Environment Variables

```bash
# Database (use Koyeb PostgreSQL or Neon)
DATABASE_URL=postgresql://user:password@host:5432/restaurant?sslmode=require

# Server
HOST=0.0.0.0
PORT=8080

# JWT (GENERATE YOUR OWN!)
JWT_SECRET=<output-from-openssl-rand-base64-64>
JWT_ISSUER=restaurant-backend
ACCESS_TOKEN_TTL_MINUTES=15
REFRESH_TOKEN_TTL_DAYS=30

# Logging
RUST_LOG=info

# CORS (your frontend domain)
CORS_ALLOWED_ORIGINS=https://your-frontend.com
```

### Steps

1. **Generate JWT_SECRET:**
   ```bash
   openssl rand -base64 64
   ```

2. **Add to Koyeb Environment Variables:**
   - Go to: Service → Settings → Environment Variables
   - Click "Edit multiple variables"
   - Paste all variables
   - Click "Save"

3. **Deploy:**
   - Koyeb will automatically rebuild and deploy
   - Check logs for successful startup

---

## 🔄 Secret Rotation

If JWT_SECRET is compromised:

1. **Generate new secret:**
   ```bash
   openssl rand -base64 64
   ```

2. **Update environment variable** on hosting platform

3. **Redeploy application**

4. **All users will need to re-login** (all existing tokens invalidated)

---

## 📞 Security Contacts

If you discover a security vulnerability:

1. **DO NOT** open a public issue
2. Email: security@example.com
3. Include: description, steps to reproduce, potential impact

---

## 🔍 Security Features

### Current Implementation

✅ **Password Hashing:** Argon2id (OWASP recommended)
✅ **JWT Tokens:** HS256 with secure secret
✅ **Token Expiration:** 15min access, 30d refresh
✅ **SQL Injection:** Protected by SQLx compile-time verification
✅ **CORS:** Configurable allowed origins
✅ **SSL/TLS:** Enforced for production database connections

### Planned Improvements

- [ ] Rate limiting per IP
- [ ] Account lockout after failed login attempts
- [ ] 2FA (TOTP)
- [ ] API key authentication for services
- [ ] Audit logging for sensitive operations

---

## 📚 References

- [OWASP JWT Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/JSON_Web_Token_for_Java_Cheat_Sheet.html)
- [Argon2 Password Hashing](https://github.com/P-H-C/phc-winner-argon2)
- [SQLx Security](https://github.com/launchbadge/sqlx#compile-time-verification)
