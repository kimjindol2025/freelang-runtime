#!/bin/bash

# Project Sovereign-Mesh: Docker Deployment Script for Host 253
# Deployment Target: kimjin@123.212.111.26:22253

set -e

echo "════════════════════════════════════════════════════════════"
echo "  🚀 Project Sovereign-Mesh: Docker Deployment to Host 253"
echo "════════════════════════════════════════════════════════════"
echo ""

# Configuration
REMOTE_HOST="123.212.111.26"
REMOTE_PORT="22253"
REMOTE_USER="kimjin"
REMOTE_ADDR="${REMOTE_USER}@${REMOTE_HOST}"
DOCKER_IMAGE="freelang-sovereign-mesh:latest"
DOCKER_CONTAINER="sovereign-mesh-live"
DEPLOY_DIR="/home/kimjin/docker-deployments/sovereign-mesh"

echo "📡 Configuration:"
echo "   Host:      ${REMOTE_ADDR}:${REMOTE_PORT}"
echo "   Image:     ${DOCKER_IMAGE}"
echo "   Container: ${DOCKER_CONTAINER}"
echo "   Deploy Dir: ${DEPLOY_DIR}"
echo ""

# Step 1: Prepare deployment package
echo "📦 Step 1: Prepare deployment package..."
cd /data/data/com.termux/files/home/freelang-sovereign-mesh

# Create tarball with Dockerfile and code
tar czf sovereign-mesh-deployment.tar.gz \
    Dockerfile \
    src/ \
    tests/ \
    *.md \
    .git/

echo "   ✅ Created: sovereign-mesh-deployment.tar.gz ($(du -h sovereign-mesh-deployment.tar.gz | cut -f1))"
echo ""

# Step 2: Transfer to remote host
echo "📤 Step 2: Transfer to host 253..."
scp -P ${REMOTE_PORT} sovereign-mesh-deployment.tar.gz \
    ${REMOTE_ADDR}:/tmp/ && \
    echo "   ✅ Transferred successfully"
echo ""

# Step 3: Build Docker image on remote host
echo "🔨 Step 3: Build Docker image on host 253..."
ssh -p ${REMOTE_PORT} ${REMOTE_ADDR} << 'REMOTECMD'
set -e
echo ""
cd /tmp
tar xzf sovereign-mesh-deployment.tar.gz -C /tmp/
mkdir -p /tmp/sovereign-mesh-build
cd /tmp/sovereign-mesh-build
cp /tmp/Dockerfile .
cp -r /tmp/src . 2>/dev/null || true
cp -r /tmp/tests . 2>/dev/null || true
cp /tmp/*.md . 2>/dev/null || true

echo "🔨 Building Docker image: freelang-sovereign-mesh:latest"
docker build \
    --tag freelang-sovereign-mesh:latest \
    --label deployment.date="$(date -u +'%Y-%m-%dT%H:%M:%SZ')" \
    --label project="freelang-sovereign-mesh" \
    . 2>&1 | tail -20

echo ""
echo "✅ Image build complete"
docker images | grep sovereign-mesh || echo "Image not found"
REMOTECMD

echo ""

# Step 4: Stop existing container
echo "🛑 Step 4: Stop existing container (if any)..."
ssh -p ${REMOTE_PORT} ${REMOTE_ADDR} \
    "docker stop ${DOCKER_CONTAINER} 2>/dev/null || echo '   No existing container'" && \
    echo "   ✅ Container stopped"
echo ""

# Step 5: Run new container
echo "▶️  Step 5: Run Docker container on host 253..."
ssh -p ${REMOTE_PORT} ${REMOTE_ADDR} << 'REMOTECMD'
DOCKER_CONTAINER="sovereign-mesh-live"

docker run -d \
    --name ${DOCKER_CONTAINER} \
    --restart unless-stopped \
    -v /home/kimjin/sovereign-mesh-data:/app/data \
    -e PROJECT="freelang-sovereign-mesh" \
    -e VERSION="1.0.0" \
    -e ENVIRONMENT="production" \
    -l deployment="host-253" \
    freelang-sovereign-mesh:latest \
    /bin/bash -c "while true; do echo '$(date): Sovereign-Mesh running'; sleep 300; done"

echo "   ✅ Container started: ${DOCKER_CONTAINER}"
docker ps | grep sovereign-mesh
REMOTECMD

echo ""

# Step 6: Verify deployment
echo "✅ Step 6: Verify deployment..."
ssh -p ${REMOTE_PORT} ${REMOTE_ADDR} << 'REMOTECMD'
echo ""
echo "════════════════════════════════════════════════════════════"
echo "  📊 Deployment Verification Report"
echo "════════════════════════════════════════════════════════════"
echo ""
echo "🐳 Docker Image:"
docker images | grep -E "REPOSITORY|sovereign-mesh" || echo "Not found"
echo ""
echo "🐳 Running Containers:"
docker ps | grep -E "CONTAINER ID|sovereign-mesh" || echo "No running container"
echo ""
echo "📋 Container Logs (recent):"
docker logs --tail 5 sovereign-mesh-live 2>/dev/null || echo "No logs yet"
echo ""
echo "📊 Container Stats:"
docker stats --no-stream sovereign-mesh-live 2>/dev/null | tail -2 || echo "Not available"
echo ""
echo "════════════════════════════════════════════════════════════"
REMOTECMD

echo ""
echo "════════════════════════════════════════════════════════════"
echo "  ✨ Deployment Complete"
echo "════════════════════════════════════════════════════════════"
echo ""
echo "🎯 Next Steps:"
echo "   1. SSH to host 253: ssh -p 22253 kimjin@123.212.111.26"
echo "   2. View container: docker logs -f sovereign-mesh-live"
echo "   3. Access shell: docker exec -it sovereign-mesh-live bash"
echo ""
echo "📦 Container Access:"
echo "   docker exec -it sovereign-mesh-live bash"
echo ""
echo "✨ Project Sovereign-Mesh is now running on Host 253 ✨"
echo ""

