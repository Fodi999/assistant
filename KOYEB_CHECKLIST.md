# ✅ Koyeb Deployment Checklist

Copy this into Koyeb **"Edit multiple variables"** field:

```bash
DATABASE_URL=postgresql://YOUR_USER:YOUR_PASSWORD@YOUR_HOST:5432/restaurant?sslmode=require
HOST=0.0.0.0
PORT=8080
JWT_SECRET=YOUR_OPENSSL_GENERATED_SECRET_HERE
JWT_ISSUER=restaurant-backend
ACCESS_TOKEN_TTL_MINUTES=15
REFRESH_TOKEN_TTL_DAYS=30
RUST_LOG=info
CORS_ALLOWED_ORIGINS=https://your-frontend-domain.com
```

---

## 📝 Before You Deploy

### Step 1: Generate JWT_SECRET

```bash
openssl rand -base64 64
```

**Copy the output** and replace `YOUR_OPENSSL_GENERATED_SECRET_HERE` above.

### Step 2: Get Database URL

**Option A - Koyeb PostgreSQL:**
- Koyeb → Create Service → PostgreSQL
- Copy connection string

**Option B - Neon (Free):**
- https://neon.tech → Create Project
- Copy connection string
- **Must include:** `?sslmode=require`

Example:
```
postgresql://user:pass@ep-xxx.eu-central-1.aws.neon.tech/db?sslmode=require
```

### Step 3: Frontend Domain (Optional)

If you have a frontend, replace:
```bash
CORS_ALLOWED_ORIGINS=https://app.mydomain.com
```

If API-only (no browser access needed), you can omit this or set:
```bash
CORS_ALLOWED_ORIGINS=
```

---

## 🚀 Deployment Steps

1. ✅ Open Koyeb Dashboard: https://app.koyeb.com
2. ✅ Create Service → Deploy from GitHub
3. ✅ Select: `Fodi999/assistant` repo
4. ✅ Branch: `main`
5. ✅ Builder: **Docker**
6. ✅ Dockerfile path: `Dockerfile`
7. ✅ Port: **8080**
8. ✅ Click **"Environment Variables"**
9. ✅ Click **"Edit multiple variables"**
10. ✅ **Paste the config from top** (with your real values!)
11. ✅ Click **"Deploy"**

---

## ⏱️ Wait for Build

- Build time: ~5-10 minutes (Rust compilation)
- Watch logs: "Building..." → "Deploying..." → "Healthy"
- Look for: `Server listening on 0.0.0.0:8080` ✅

---

## 🧪 Test API

Get your app URL from Koyeb (e.g., `https://your-app-name.koyeb.app`)

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

---

## 🎉 Success!

Your Restaurant Backend is live on Koyeb with:

✅ PostgreSQL database (with migrations applied)
✅ Secure JWT authentication
✅ Menu Engineering + ABC Analysis
✅ Inventory management
✅ Recipe costing
✅ Sales tracking

---

## 📊 Next Steps

1. **Connect Frontend:** Use the Koyeb URL as API base
2. **Add Custom Domain:** api.yourdomain.com
3. **Enable Auto-Deploy:** Push to GitHub → Auto-rebuild
4. **Monitor Logs:** Check for errors/performance
5. **Set up Alerts:** Koyeb can notify on downtime

---

## 🔒 Security Reminder

⚠️ **NEVER share:**
- Your `.env` file
- `JWT_SECRET` value
- Database password
- Koyeb environment variables

✅ **These are SECRET** - treat like passwords!

---

## 🆘 Troubleshooting

**Error: "Database connection failed"**
→ Check `DATABASE_URL` has `?sslmode=require`

**Error: "Invalid JWT secret"**
→ Regenerate: `openssl rand -base64 64`

**Error: "CORS blocked"**
→ Add your frontend domain to `CORS_ALLOWED_ORIGINS`

**Build timeout**
→ Retry or upgrade Koyeb plan

---

## 📞 Need Help?

- Koyeb Docs: https://www.koyeb.com/docs
- GitHub Issues: https://github.com/Fodi999/assistant/issues
