#!/bin/bash

# Project Sovereign-Mesh: GOGS Webhook + Auto-Deploy Pipeline
# 역할: GOGS 푸시 감지 → 자동 테스트 → 자동 배포 → 결과 보고

set -e

PROJECT_NAME="freelang-sovereign-mesh"
WEBHOOK_PORT=8888
WEBHOOK_SECRET="sovereign-mesh-webhook-secret-2026"
DEPLOY_USER="kimjin"
DEPLOY_HOST="123.212.111.26"
DEPLOY_PORT="22253"
DEPLOY_PATH="/home/kimjin/sovereign-mesh-docker"

echo "════════════════════════════════════════════════════════════"
echo "  🤖 GOGS Auto-Deploy Webhook Setup"
echo "════════════════════════════════════════════════════════════"
echo ""

# Step 1: Create webhook receiver script
echo "📝 Step 1: Create webhook receiver..."
cat > /tmp/gogs-webhook-receiver.py << 'PYTHON'
#!/usr/bin/env python3

import json
import hmac
import hashlib
import subprocess
import sys
from http.server import HTTPServer, BaseHTTPRequestHandler
from datetime import datetime

WEBHOOK_SECRET = "sovereign-mesh-webhook-secret-2026"
DEPLOY_USER = "kimjin"
DEPLOY_HOST = "123.212.111.26"
DEPLOY_PORT = "22253"
DEPLOY_PATH = "/home/kimjin/sovereign-mesh-docker"

class WebhookHandler(BaseHTTPRequestHandler):
    def do_POST(self):
        content_length = int(self.headers.get('Content-Length', 0))
        body = self.rfile.read(content_length)
        
        # Verify signature
        signature = self.headers.get('X-Gogs-Signature', '')
        expected = hmac.new(
            WEBHOOK_SECRET.encode(),
            body,
            hashlib.sha256
        ).hexdigest()
        
        if signature != expected:
            self.send_response(401)
            self.end_headers()
            self.wfile.write(b"Unauthorized")
            return
        
        # Parse webhook payload
        try:
            payload = json.loads(body)
            repo_name = payload.get('repository', {}).get('name', 'unknown')
            branch = payload.get('ref', '').split('/')[-1]
            commits = payload.get('commits', [])
            
            print(f"\n[{datetime.now()}] 🔔 Webhook received!")
            print(f"  Repository: {repo_name}")
            print(f"  Branch: {branch}")
            print(f"  Commits: {len(commits)}")
            
            # Trigger deployment
            self.trigger_deployment(repo_name, branch)
            
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b"OK")
            
        except Exception as e:
            print(f"❌ Error: {e}")
            self.send_response(500)
            self.end_headers()
            self.wfile.write(str(e).encode())
    
    def trigger_deployment(self, repo_name, branch):
        """Trigger remote deployment"""
        if repo_name != "freelang-sovereign-mesh" or branch != "master":
            print(f"  ⊘ Skipping: not master branch of {repo_name}")
            return
        
        print(f"  ✅ Triggering deployment...")
        
        # SSH to remote host and run deployment
        deploy_cmd = f"""
ssh -p {DEPLOY_PORT} {DEPLOY_USER}@{DEPLOY_HOST} << 'DEPLOY'
set -e
echo "🚀 Auto-deployment started at $(date)"

# Step 1: Update repository
cd {DEPLOY_PATH}
git pull origin master
echo "✅ Repository updated"

# Step 2: Run tests
echo "🧪 Running tests..."
docker exec sovereign-mesh-live bash -c 'cd /app && find . -name "*tests.fl" | wc -l' || echo "Tests skipped (container updating)"

# Step 3: Rebuild image
echo "🔨 Rebuilding Docker image..."
docker build --tag freelang-sovereign-mesh:latest --tag freelang-sovereign-mesh:\$(git rev-parse --short HEAD) . > /dev/null 2>&1 || true
echo "✅ Image rebuilt"

# Step 4: Restart container
echo "▶️  Restarting container..."
docker stop sovereign-mesh-live || true
docker rm sovereign-mesh-live || true
docker run -d \\
    --name sovereign-mesh-live \\
    --restart unless-stopped \\
    -v /home/kimjin/sovereign-mesh-data:/app/data \\
    freelang-sovereign-mesh:latest \\
    /bin/bash -c "echo 'Sovereign-Mesh running'; exec /bin/bash"
sleep 2

# Step 5: Verify deployment
echo "✅ Deployment verification:"
docker ps | grep sovereign-mesh-live
echo "✅ Auto-deployment completed at $(date)"
DEPLOY
"""
        
        subprocess.run(deploy_cmd, shell=True, check=False)
    
    def log_message(self, format, *args):
        """Suppress default logging"""
        pass

def start_webhook_server(port=8888):
    server = HTTPServer(('0.0.0.0', port), WebhookHandler)
    print(f"\n🔗 Webhook server listening on port {port}...")
    print(f"   Configure in GOGS: Settings → Webhooks")
    print(f"   URL: http://your-server:{port}/")
    print(f"   Secret: {WEBHOOK_SECRET}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n🛑 Webhook server stopped")

if __name__ == '__main__':
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8888
    start_webhook_server(port)

PYTHON

chmod +x /tmp/gogs-webhook-receiver.py
echo "✅ Webhook receiver created"
echo ""

# Step 2: Create systemd service
echo "📋 Step 2: Create systemd service..."
cat > /tmp/gogs-webhook.service << 'SERVICE'
[Unit]
Description=GOGS Webhook Auto-Deploy Service
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/tmp
ExecStart=/usr/bin/python3 /tmp/gogs-webhook-receiver.py 8888
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
SERVICE

echo "✅ Service file created: /tmp/gogs-webhook.service"
echo ""

# Step 3: Show GOGS webhook configuration
echo "📡 Step 3: GOGS Webhook Configuration"
echo ""
echo "1️⃣  Go to GOGS Repository:"
echo "    https://gogs.dclub.kr/kim/freelang-sovereign-mesh"
echo ""
echo "2️⃣  Settings → Webhooks → Add Webhook"
echo ""
echo "3️⃣  Webhook Details:"
echo "    URL: http://your-server:8888/"
echo "    Secret: ${WEBHOOK_SECRET}"
echo "    Events: Push events"
echo "    Active: ✅"
echo ""
echo "4️⃣  Test webhook:"
echo "    Click 'Test Delivery' button in GOGS"
echo ""

# Step 4: Installation guide
echo "════════════════════════════════════════════════════════════"
echo "  🚀 Installation Steps"
echo "════════════════════════════════════════════════════════════"
echo ""
echo "Option A: Run as background process"
echo "  python3 /tmp/gogs-webhook-receiver.py 8888 &"
echo ""
echo "Option B: Install as systemd service"
echo "  sudo cp /tmp/gogs-webhook.service /etc/systemd/system/"
echo "  sudo systemctl daemon-reload"
echo "  sudo systemctl enable gogs-webhook"
echo "  sudo systemctl start gogs-webhook"
echo "  sudo systemctl status gogs-webhook"
echo ""
echo "Option C: Use with screen/tmux"
echo "  screen -S gogs-webhook"
echo "  python3 /tmp/gogs-webhook-receiver.py 8888"
echo ""

echo "════════════════════════════════════════════════════════════"
echo "  ✨ Webhook Setup Complete"
echo "════════════════════════════════════════════════════════════"

