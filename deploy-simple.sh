#!/bin/bash

# Project Sovereign-Mesh: Simplified Docker Deployment for Host 253

set -e

echo "════════════════════════════════════════════════════════════"
echo "  🚀 Project Sovereign-Mesh: Docker Deployment to Host 253"
echo "════════════════════════════════════════════════════════════"
echo ""

REMOTE_HOST="123.212.111.26"
REMOTE_PORT="22253"
REMOTE_USER="kimjin"
REMOTE_ADDR="${REMOTE_USER}@${REMOTE_HOST}"

echo "📡 Target: ${REMOTE_ADDR}:${REMOTE_PORT}"
echo ""

# Deploy directly from GOGS
echo "📤 Deploying from GOGS: https://gogs.dclub.kr/kim/freelang-sovereign-mesh.git"
echo ""

ssh -p ${REMOTE_PORT} ${REMOTE_ADDR} << 'REMOTECMD'
#!/bin/bash
set -e

echo "🔧 Step 1: Setup deployment directory..."
mkdir -p /home/kimjin/sovereign-mesh-docker
cd /home/kimjin/sovereign-mesh-docker

echo "📥 Step 2: Clone from GOGS..."
if [ -d ".git" ]; then
    echo "   Updating existing repository..."
    git pull origin master
else
    echo "   Cloning new repository..."
    git clone https://gogs.dclub.kr/kim/freelang-sovereign-mesh.git .
fi

echo ""
echo "✅ Repository prepared"
git log --oneline -3

echo ""
echo "🔨 Step 3: Create Dockerfile..."
cat > Dockerfile << 'DOCKERFILE'
FROM ubuntu:22.04

LABEL maintainer="kim@freelang.dev"
LABEL project="freelang-sovereign-mesh"
LABEL version="1.0.0"

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates git curl build-essential && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY . /app/

ENV PROJECT="freelang-sovereign-mesh"
ENV ARCHITECTURE="L0-L1-L2-L3"
ENV TESTS="18 unforgiving tests"
ENV RULES="4/4 achieved"

HEALTHCHECK --interval=30s --timeout=10s --retries=3 \
    CMD git log --oneline -1 > /dev/null 2>&1 || exit 1

CMD ["/bin/bash"]
DOCKERFILE

echo "✅ Dockerfile created"

echo ""
echo "🐳 Step 4: Build Docker image..."
docker build \
    --tag freelang-sovereign-mesh:latest \
    --label "built=$(date -u +'%Y-%m-%dT%H:%M:%SZ')" \
    --label "commit=$(git rev-parse --short HEAD)" \
    . 2>&1 | tail -15

echo ""
echo "✅ Image build complete"
docker images | grep sovereign-mesh | head -1

echo ""
echo "▶️  Step 5: Deploy container..."
CONTAINER="sovereign-mesh-live"

# Stop existing container
docker stop ${CONTAINER} 2>/dev/null || true
docker rm ${CONTAINER} 2>/dev/null || true

# Run new container
docker run -d \
    --name ${CONTAINER} \
    --restart unless-stopped \
    -v /home/kimjin/sovereign-mesh-data:/app/data \
    -e PROJECT="freelang-sovereign-mesh" \
    -e VERSION="1.0.0" \
    freelang-sovereign-mesh:latest \
    /bin/bash -c "echo '🎯 Project Sovereign-Mesh running on Host 253'; exec /bin/bash"

echo "✅ Container deployed: ${CONTAINER}"

echo ""
echo "════════════════════════════════════════════════════════════"
echo "  📊 Deployment Verification"
echo "════════════════════════════════════════════════════════════"
echo ""

echo "🐳 Docker Image:"
docker images | grep sovereign-mesh | head -1

echo ""
echo "🐳 Running Containers:"
docker ps | grep sovereign-mesh

echo ""
echo "📋 Project Status inside container:"
docker exec ${CONTAINER} bash -c 'cd /app && echo "Files:" && ls -1 && echo "" && echo "Commits:" && git log --oneline -3' || echo "Container not ready yet"

echo ""
echo "════════════════════════════════════════════════════════════"
echo "  ✨ Deployment Complete!"
echo "════════════════════════════════════════════════════════════"
echo ""
echo "📦 Access container:"
echo "   docker exec -it sovereign-mesh-live bash"
echo ""
echo "📊 View logs:"
echo "   docker logs -f sovereign-mesh-live"
echo ""
echo "🎯 Project location in container: /app"
echo ""

REMOTECMD

echo ""
echo "✅ Deployment to Host 253 completed!"

