# 🚀 Koyeb Deployment - Final Checklist

## ✅ Prerequisites (все готово!)

- [x] Dockerfile в корне репозитория
- [x] Cargo.lock в git (для детерминированной сборки)
- [x] Multi-stage build (rust:1.75 → debian:bookworm-slim)
- [x] PORT 8080 exposed
- [x] Migrations в репозитории
- [x] Environment variables готовы

---

## 📋 Step-by-Step Deployment

### 1️⃣ Create Service on Koyeb

**URL:** https://app.koyeb.com/services/new

**Settings:**
```
┌─────────────────────────────────────┐
│ Source: GitHub                      │
│ Repository: Fodi999/assistant       │
│ Branch: main                        │
│                                     │
│ ⚠️ IMPORTANT:                       │
│ Builder: Docker (not Buildpack!)    │
│ Dockerfile: Dockerfile              │
│ Build context: . (root)             │
│                                     │
│ Port: 8080                          │
│ Protocol: HTTP                      │
│                                     │
│ Region: Europe (eu-west)            │
│ Instance: Free (512 MB) or Eco      │
└─────────────────────────────────────┘
```

### 2️⃣ Environment Variables

Click **"Environment Variables"** → **"Edit multiple variables"**

**Copy-paste this:**

```bash
DATABASE_URL=postgresql://neondb_owner:2pLI4eDQXEdF@ep-orange-bird-a2yh5v07-pooler.eu-central-1.aws.neon.tech/neondb?sslmode=require&application_name=restaurant-backend
HOST=0.0.0.0
PORT=8080
JWT_SECRET=R/x7ccoRyHGedn5KuPeOCMVl94V8mlTv6vXYVpxQ7fVTFG8AayG1PId8dy0v5dqaUnLTp1HFP0ySNPLte6j1IA==
JWT_ISSUER=restaurant-backend
ACCESS_TOKEN_TTL_MINUTES=15
REFRESH_TOKEN_TTL_DAYS=30
RUST_LOG=info
CORS_ALLOWED_ORIGINS=*
```

**⚠️ Important:**
- `DATABASE_URL` - Must include `?sslmode=require`
- `JWT_SECRET` - Your secure generated secret (already set)
- `CORS_ALLOWED_ORIGINS` - Use `*` for testing, specific domain for production

### 3️⃣ Deploy

Click **"Deploy"**

**Expected build time:** 5-10 minutes (Rust compilation)

---

## 📊 Build Process (что будет происходить)

### Stage 1: Docker Build
```
✓ Cloning repository from GitHub
✓ Found Dockerfile ✓
✓ Building multi-stage image
  ├── rust:1.75 (builder stage)
  │   ├── Install pkg-config, libssl-dev
  │   ├── Copy Cargo.toml, Cargo.lock
  │   ├── Cache dependencies (dummy build)
  │   ├── Copy source code
  │   ├── cargo build --release
  │   └── Binary: target/release/restaurant-backend
  │
  └── debian:bookworm-slim (runtime stage)
      ├── Install ca-certificates, libssl3
      ├── Copy binary from builder
      ├── Copy migrations
      └── Expose 8080
```

### Stage 2: Container Start
```
✓ Starting container
✓ Running migrations
✓ Server listening on 0.0.0.0:8080
✓ Health check passed
✓ Service is Healthy ✅
```

---

## 🧪 Testing After Deploy

### Get your app URL
Koyeb will provide: `https://your-app-name.koyeb.app`

### Test 1: Health Check
```bash
curl https://your-app-name.koyeb.app/health
```
Expected: `200 OK`

### Test 2: Register User
```bash
curl -X POST https://your-app-name.koyeb.app/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@test.com",
    "password": "SecurePass123!",
    "display_name": "Admin",
    "restaurant_name": "Test Restaurant"
  }'
```
Expected: `201 Created` with user data

### Test 3: Login
```bash
curl -X POST https://your-app-name.koyeb.app/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@test.com",
    "password": "SecurePass123!"
  }'
```
Expected: Access token + refresh token

### Test 4: Menu Engineering
```bash
TOKEN="your_access_token_from_login"

curl -X GET "https://your-app-name.koyeb.app/api/menu-engineering/analysis?period_days=30&language=en" \
  -H "Authorization: Bearer $TOKEN"
```
Expected: Menu Engineering Matrix with BCG + ABC analysis

---

## 🔍 Troubleshooting

### Error: "Cargo.lock not found"
**Solution:** Already fixed with `COPY Cargo.lock* ./`

If still fails:
1. Go to Koyeb → Settings → Builder
2. Change **Dockerfile path** to: `Dockerfile.koyeb`
3. Redeploy

### Error: "Database connection failed"
**Check:**
- `DATABASE_URL` includes `?sslmode=require`
- Neon database is accessible (not paused)
- Connection string is correct

**Fix:** 
```bash
# Verify in Neon dashboard:
postgresql://user:pass@host.neon.tech/db?sslmode=require
```

### Error: "Build timeout"
**Cause:** Rust compilation takes time on free tier

**Solutions:**
1. Wait and retry (timeout happens randomly)
2. Upgrade to paid tier (more CPU)
3. Use pre-built Docker image (push to GHCR)

### Error: "Container unhealthy"
**Check logs for:**
- Migration errors
- Port binding (must be 0.0.0.0:8080)
- Environment variables missing

---

## 🔄 Auto-Deploy (Optional)

Enable auto-deploy for automatic updates on git push:

1. Koyeb → Your Service → Settings
2. **Git** section
3. Enable **"Auto-deploy on push"**
4. Save

Now every `git push origin main` triggers automatic rebuild! 🚀

---

## 📈 What You've Built

Your deployed API includes:

✅ **Authentication**
- JWT-based (HS256)
- Argon2id password hashing
- Refresh tokens

✅ **Inventory Management**
- Catalog categories & ingredients
- Product tracking with expiration
- Stock warnings

✅ **Recipe Costing**
- Multi-ingredient recipes
- Real-time cost calculation
- Recipe types (ingredient/semi/final)

✅ **Dish Management**
- Recipe-based dishes
- Pricing & margins
- Active/inactive status

✅ **Menu Engineering + ABC Analysis**
- BCG Matrix (Star/Plowhorse/Puzzle/Dog)
- ABC Classification (A/B/C revenue tiers)
- Combined strategies (9 actionable recommendations)
- Sales tracking & analytics

✅ **Multi-language**
- English, Russian, Polish, Ukrainian
- Localized recommendations

---

## 🎉 Success Criteria

Your deployment is successful when:

✓ Build completes without errors
✓ Container status: **Healthy** (green)
✓ `/health` endpoint responds `200 OK`
✓ User registration works
✓ Login returns JWT tokens
✓ Menu Engineering analysis returns data

---

## 📞 Support

**Koyeb Docs:** https://www.koyeb.com/docs/deploy/docker
**GitHub Repo:** https://github.com/Fodi999/assistant
**Issues:** https://github.com/Fodi999/assistant/issues

---

**Last updated:** 2026-02-07
**Commit:** `8ee5275` (latest)
