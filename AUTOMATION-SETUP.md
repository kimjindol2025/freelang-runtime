# Project Sovereign-Mesh: Auto-Deploy & Test Pipeline

## 📋 Overview

이 문서는 GOGS 저장소에서 코드 변경을 감지하여 자동으로 테스트 및 배포하는 CI/CD 파이프라인을 설명합니다.

---

## 🔄 Pipeline Architecture

```
┌─────────────────┐
│  Commit to GOGS │
└────────┬────────┘
         │
         ▼
┌──────────────────────┐
│ Webhook Triggered    │  (gogs-webhook-receiver.py)
│ Port 8888            │
└────────┬─────────────┘
         │
         ▼
┌────────────────────────────┐
│ SSH to Host 253            │  (123.212.111.26:22253)
└────────┬───────────────────┘
         │
         ▼
┌────────────────────────────┐
│ Auto Test Pipeline         │  (auto-test-pipeline.sh)
│ - Code integrity check     │
│ - Module verification      │
│ - Test count validation    │
│ - Git history check        │
│ - Docker status            │
└────────┬───────────────────┘
         │
         ▼
┌────────────────────────────┐
│ Auto Docker Build          │
│ - Pull latest code         │
│ - Build image              │
│ - Restart container        │
└────────┬───────────────────┘
         │
         ▼
┌────────────────────────────┐
│ Verify Deployment          │
│ - Container running        │
│ - Health check             │
│ - Report generation        │
└────────────────────────────┘
```

---

## 🚀 Installation & Setup

### Option 1: Webhook Server (Recommended)

#### Step 1: Start Webhook Listener on Local Machine

```bash
# Make sure Python 3 is installed
python3 /tmp/gogs-webhook-receiver.py 8888 &

# Or use screen
screen -S gogs-webhook
python3 /tmp/gogs-webhook-receiver.py 8888
```

#### Step 2: Configure GOGS Webhook

1. Go to: https://gogs.dclub.kr/kim/freelang-sovereign-mesh
2. Settings → Webhooks → Add Webhook
3. Fill in:
   - **Payload URL**: `http://your-public-ip:8888/`
   - **Secret**: `sovereign-mesh-webhook-secret-2026`
   - **Events**: Push events
   - **Active**: ✅

4. Click "Add Webhook"
5. Test: Click the webhook → "Test Delivery"

Expected response: `200 OK`

---

### Option 2: Cron Job (Alternative)

```bash
# Add to crontab: crontab -e

# Check GOGS every 5 minutes and deploy if changed
*/5 * * * * cd /home/kimjin/sovereign-mesh-docker && \
  if [ "$(git rev-parse HEAD)" != "$(git rev-parse origin/master)" ]; then \
    git pull origin master && \
    docker build -t freelang-sovereign-mesh:latest . && \
    docker restart sovereign-mesh-live; \
  fi
```

---

### Option 3: Git Hook (Local)

```bash
# In local clone:
cat > .git/hooks/post-commit << 'HOOK'
#!/bin/bash
echo "🚀 Pushing to GOGS..."
git push origin master

echo "📡 Triggering remote deployment..."
ssh -p 22253 kimjin@123.212.111.26 << 'DEPLOY'
  cd /home/kimjin/sovereign-mesh-docker
  git pull origin master
  docker build -t freelang-sovereign-mesh:latest .
  docker restart sovereign-mesh-live
DEPLOY
HOOK

chmod +x .git/hooks/post-commit
```

---

## 📊 Auto-Test Pipeline

### Running Tests Locally

```bash
# Execute test pipeline
bash auto-test-pipeline.sh

# Expected output:
# ✅ Phase 1: Code Integrity Check
# ✅ Phase 2: Module Structure Verification (14/14 modules)
# ✅ Phase 3: Test File Verification (3/3 test files)
# ✅ Phase 4: Git Commit History
# ✅ Phase 5: Docker Deployment Status
```

### Running Tests on Host 253

```bash
ssh -p 22253 kimjin@123.212.111.26 << 'EOF'
cd /home/kimjin/sovereign-mesh-docker
bash auto-test-pipeline.sh
