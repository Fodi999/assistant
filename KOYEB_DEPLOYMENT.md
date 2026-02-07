# 🚀 Deploying to Koyeb

## 📋 Prerequisites

1. **Koyeb Account:** https://app.koyeb.com
2. **PostgreSQL Database:** Use Koyeb PostgreSQL or Neon
3. **Strong JWT Secret:** Generated with `openssl rand -base64 64`

---

## 🔧 Step-by-Step Deployment

### 1️⃣ Create PostgreSQL Database

**Option A: Koyeb PostgreSQL**
- Koyeb → Services → Create Service → PostgreSQL
- Copy the `DATABASE_URL` from connection string

**Option B: Neon (Free Tier)**
- Go to: https://neon.tech
- Create project → Copy connection string
- Example: `postgresql://user:pass@ep-xxx.neon.tech/db?sslmode=require`

### 2️⃣ Generate JWT Secret

```bash
openssl rand -base64 64
```

**Save the output!** You'll need it in the next step.

Example output:
```
R/x7ccoRyHGedn5KuPeOCMVl94V8mlTv6vXYVpxQ7fVTFG8AayG1PId8dy0v5dqaUnLTp1HFP0ySNPLte6j1IA==
```

### 3️⃣ Create Koyeb Service

1. **Go to Koyeb Dashboard:** https://app.koyeb.com
2. **Create Service → Deploy from GitHub**
3. **Select Repository:** `Fodi999/assistant`
4. **Branch:** `main`
5. **⚠️ CRITICAL: Builder → Select "Docker"** (NOT Buildpack!)
6. **Dockerfile path:** Leave empty (Koyeb auto-detects) OR type `Dockerfile`
7. **Build context:** `.` (root directory)
8. **Port:** `8080`
9. **Region:** Europe (eu-west) or closest to you
10. **Instance type:** Free (512 MB RAM) or Eco ($5/month)

**Troubleshooting "Could not detect application":**
- Make sure you selected **"Docker"** builder (not Buildpack)
- If still fails, try using `Dockerfile.koyeb` instead
- Check logs for specific error messages

### 4️⃣ Configure Environment Variables

Click **"Environment Variables"** → **"Edit multiple variables"**

Paste this (replace values with your own):

```bash
DATABASE_URL=postgresql://user:password@host:5432/restaurant?sslmode=require
HOST=0.0.0.0
PORT=8080
JWT_SECRET=YOUR_GENERATED_SECRET_FROM_STEP_2
JWT_ISSUER=restaurant-backend
ACCESS_TOKEN_TTL_MINUTES=15
REFRESH_TOKEN_TTL_DAYS=30
RUST_LOG=info
CORS_ALLOWED_ORIGINS=https://your-frontend.com
```

**⚠️ Important:**
- Replace `DATABASE_URL` with your actual database connection string
- Replace `JWT_SECRET` with output from step 2
- Replace `CORS_ALLOWED_ORIGINS` with your frontend URL (or remove if API-only)

### 5️⃣ Deploy

1. Click **"Deploy"**
2. Wait for build (~5-10 minutes for Rust compilation)
3. Check logs for: `Server listening on 0.0.0.0:8080`

---

## 🧪 Test Your Deployment

### Health Check

```bash
curl https://your-app.koyeb.app/health
```

Expected: `200 OK` (or similar health response)

### Register Test User

```bash
curl -X POST https://your-app.koyeb.app/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "SecurePass123!",
    "display_name": "Test User",
    "restaurant_name": "My Restaurant"
  }'
```

Expected: `201 Created` with user data

### Login

```bash
curl -X POST https://your-app.koyeb.app/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "SecurePass123!"
  }'
```

Expected: Access token and refresh token

---

## 📊 Monitoring

### View Logs

Koyeb Dashboard → Your Service → Logs

Look for:
- ✅ `Starting Restaurant Backend...`
- ✅ `Database connection pool established`
- ✅ `Database migrations completed`
- ✅ `Server listening on 0.0.0.0:8080`

### Common Issues

**Issue:** `Connection refused` or `Database error`
- **Fix:** Check `DATABASE_URL` is correct and includes `?sslmode=require`

**Issue:** `Invalid JWT secret`
- **Fix:** Ensure `JWT_SECRET` is base64-encoded string (minimum 32 chars)

**Issue:** Build timeout
- **Fix:** Koyeb free tier has build limits. Retry or upgrade plan.

---

## 🔄 Updates & Redeployment

### Option 1: Auto-deploy (Recommended)

Enable auto-deploy in Koyeb:
- Service Settings → Git → Enable "Auto-deploy"
- Every push to `main` branch triggers rebuild

### Option 2: Manual Deploy

1. Push code to GitHub
2. Koyeb Dashboard → Your Service → "Redeploy"

---

## 🌍 Custom Domain (Optional)

1. Koyeb → Your Service → Domains
2. Add custom domain: `api.yourdomain.com`
3. Update DNS records as shown
4. Update `CORS_ALLOWED_ORIGINS` if needed

---

## 💰 Cost Estimation

**Koyeb Free Tier:**
- ✅ 1 service
- ✅ 512 MB RAM
- ✅ 100 GB bandwidth/month
- ⚠️ Service sleeps after inactivity

**Paid Tier ($5-10/month):**
- Always-on service
- More RAM (1-2 GB)
- Better performance

**Database:**
- Koyeb PostgreSQL: ~$5/month
- Neon Free Tier: Up to 3 GB storage (good for MVP)

---

## 🔐 Security Reminder

✅ **Always generate new JWT_SECRET** for production
✅ **Never commit `.env` file** to git
✅ **Use SSL** for database (`?sslmode=require`)
✅ **Restrict CORS** to your actual frontend domain
✅ **Review SECURITY.md** for full guidelines

---

## 📞 Support

- **Koyeb Docs:** https://www.koyeb.com/docs
- **GitHub Issues:** https://github.com/Fodi999/assistant/issues
- **Email:** support@example.com
