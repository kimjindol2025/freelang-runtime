# FreeLang-C v1.0 Deployment Guide

**Purpose**: Complete step-by-step guide for deploying FreeLang-C in production environments
**Version**: 1.0
**Last Updated**: 2026-03-06

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [Local Development](#local-development)
3. [Docker Deployment](#docker-deployment)
4. [Kubernetes Deployment](#kubernetes-deployment)
5. [Cloud Deployment](#cloud-deployment)
6. [Monitoring & Logging](#monitoring--logging)
7. [Troubleshooting](#troubleshooting)
8. [Performance Tuning](#performance-tuning)

---

## Quick Start

### Minimum Requirements

```
CPU:     2 cores minimum
Memory:  512 MB minimum (2GB recommended)
Storage: 100 MB for installation
OS:      Linux (Ubuntu 20.04+), macOS, or Windows with Docker
```

### 30-Second Setup

```bash
# Clone repository
git clone https://gogs.dclub.kr/kim/freelang-c.git
cd freelang-c

# Build and test
docker build -t freelang-c:latest .
docker run freelang-c:latest

# Expected output: ✅ ALL TESTS PASSED
```

---

## Local Development

### Prerequisites

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    cmake \
    git \
    clang \
    lldb

# macOS
brew install cmake git clang-format

# Verify installation
cmake --version
gcc --version
```

### Build from Source

```bash
# Clone repository
git clone https://gogs.dclub.kr/kim/freelang-c.git
cd freelang-c

# Create build directory
mkdir -p build
cd build

# Configure with CMake
cmake -DCMAKE_BUILD_TYPE=Release \
      -DCMAKE_C_COMPILER=gcc \
      ..

# Build optimized binary
make -j$(nproc)

# Run tests
ctest --output-on-failure

# Build size check
ls -lh bin/freelang-c
```

### Development Workflow

```bash
# Edit code
vim src/main.c

# Rebuild
cd build
make -j$(nproc)

# Test changes
./bin/freelang-c --integration-tests

# Run benchmarks
./bin/freelang-c --benchmark

# Cleanup build artifacts
make clean
```

---

## Docker Deployment

### Building Docker Image

```bash
# Standard build
docker build -t freelang-c:1.0 .

# Build with metadata
docker build \
  --build-arg BUILD_DATE=$(date -u +'%Y-%m-%dT%H:%M:%SZ') \
  --build-arg VERSION=1.0 \
  --build-arg VCS_REF=$(git rev-parse --short HEAD) \
  -t freelang-c:latest \
  -t freelang-c:1.0 \
  .

# Multi-platform build (requires buildx)
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t freelang-c:latest .
```

### Running Docker Container

```bash
# Basic run
docker run freelang-c:1.0

# Interactive mode with volume mount
docker run -it \
  -v $(pwd)/data:/app/data \
  freelang-c:1.0

# Background daemon mode
docker run -d \
  --name freelang-c-prod \
  --restart=always \
  --health-cmd="./freelang-c --health" \
  --health-interval=30s \
  --health-timeout=10s \
  --health-retries=3 \
  -p 8080:8080 \
  freelang-c:1.0

# Check container status
docker ps
docker logs freelang-c-prod

# Stop container
docker stop freelang-c-prod
docker rm freelang-c-prod
```

### Docker Compose

```yaml
# docker-compose.yml
version: '3.8'

services:
  freelang-c:
    build: .
    image: freelang-c:1.0
    container_name: freelang-c-prod
    restart: unless-stopped
    ports:
      - "8080:8080"
    volumes:
      - ./data:/app/data
      - ./logs:/app/logs
    environment:
      - LOG_LEVEL=INFO
      - MAX_WORKERS=4
    healthcheck:
      test: ["CMD", "./freelang-c", "--health"]
      interval: 30s
      timeout: 10s
      retries: 3
    networks:
      - app-network

networks:
  app-network:
    driver: bridge
```

```bash
# Deploy with docker-compose
docker-compose up -d

# View logs
docker-compose logs -f freelang-c

# Shutdown
docker-compose down
```

---

## Kubernetes Deployment

### Prerequisites

```bash
# Install kubectl
curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl"
chmod +x kubectl
sudo mv kubectl /usr/local/bin/

# Verify
kubectl version --client
```

### Namespace Setup

```bash
# Create namespace
kubectl create namespace freelang

# Set default namespace
kubectl config set-context --current --namespace=freelang
```

### Deployment Manifest

```yaml
# freelang-c-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: freelang-c
  namespace: freelang
  labels:
    app: freelang-c
    version: 1.0
spec:
  replicas: 3
  selector:
    matchLabels:
      app: freelang-c
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  template:
    metadata:
      labels:
        app: freelang-c
    spec:
      containers:
      - name: freelang-c
        image: ghcr.io/kim/freelang-c:1.0
        imagePullPolicy: Always
        ports:
        - containerPort: 8080
          name: http
          protocol: TCP
        env:
        - name: LOG_LEVEL
          value: "INFO"
        - name: MAX_WORKERS
          valueFrom:
            resourceFieldRef:
              containerResource: limits.cpu
              divisor: "1"
        resources:
          requests:
            memory: "64Mi"
            cpu: "250m"
          limits:
            memory: "256Mi"
            cpu: "500m"
        livenessProbe:
          exec:
            command:
            - ./freelang-c
            - --health
          initialDelaySeconds: 30
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 3
        readinessProbe:
          exec:
            command:
            - ./freelang-c
            - --ready
          initialDelaySeconds: 10
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 2
        securityContext:
          runAsNonRoot: true
          runAsUser: 1000
          allowPrivilegeEscalation: false
          readOnlyRootFilesystem: true
          capabilities:
            drop:
              - ALL
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
          - weight: 100
            podAffinityTerm:
              labelSelector:
                matchExpressions:
                - key: app
                  operator: In
                  values:
                  - freelang-c
              topologyKey: kubernetes.io/hostname
---
apiVersion: v1
kind: Service
metadata:
  name: freelang-c-service
  namespace: freelang
spec:
  selector:
    app: freelang-c
  type: LoadBalancer
  ports:
  - port: 8080
    targetPort: 8080
    protocol: TCP
    name: http
---
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: freelang-c-hpa
  namespace: freelang
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: freelang-c
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

### Deploy to Kubernetes

```bash
# Apply manifests
kubectl apply -f freelang-c-deployment.yaml

# Check deployment status
kubectl get deployments -n freelang
kubectl get pods -n freelang
kubectl get services -n freelang

# View logs
kubectl logs -f deployment/freelang-c -n freelang

# Port forward for local testing
kubectl port-forward svc/freelang-c-service 8080:8080 -n freelang

# Scale deployment
kubectl scale deployment freelang-c --replicas=5 -n freelang

# Update image
kubectl set image deployment/freelang-c \
  freelang-c=ghcr.io/kim/freelang-c:1.1 \
  -n freelang

# Rollback if needed
kubectl rollout undo deployment/freelang-c -n freelang

# Delete deployment
kubectl delete deployment freelang-c -n freelang
```

---

## Cloud Deployment

### AWS Elastic Container Service (ECS)

```bash
# Create ECR repository
aws ecr create-repository --repository-name freelang-c

# Tag and push image
docker tag freelang-c:1.0 ACCOUNT.dkr.ecr.REGION.amazonaws.com/freelang-c:1.0
aws ecr get-login-password --region REGION | \
  docker login --username AWS --password-stdin ACCOUNT.dkr.ecr.REGION.amazonaws.com
docker push ACCOUNT.dkr.ecr.REGION.amazonaws.com/freelang-c:1.0

# Deploy via CloudFormation or AWS CLI
aws ecs create-service \
  --cluster freelang-cluster \
  --service-name freelang-c \
  --task-definition freelang-c:1 \
  --desired-count 3
```

### Google Cloud Run

```bash
# Build and push to Google Container Registry
gcloud builds submit --tag gcr.io/PROJECT_ID/freelang-c:1.0

# Deploy to Cloud Run
gcloud run deploy freelang-c \
  --image gcr.io/PROJECT_ID/freelang-c:1.0 \
  --platform managed \
  --region us-central1 \
  --memory 256Mi \
  --cpu 1 \
  --allow-unauthenticated
```

### Azure Container Instances

```bash
# Push to Azure Container Registry
az acr build -r REGISTRY_NAME --image freelang-c:1.0 .

# Deploy container
az container create \
  --resource-group myResourceGroup \
  --name freelang-c \
  --image REGISTRY_NAME.azurecr.io/freelang-c:1.0 \
  --cpu 1 \
  --memory 1 \
  --ports 8080
```

---

## Monitoring & Logging

### Prometheus Metrics

```bash
# Metrics endpoint
curl http://localhost:8080/metrics

# Expected output:
# freelang_c_requests_total{method="GET",status="200"} 1234
# freelang_c_request_duration_seconds 0.0234
# freelang_c_memory_usage_bytes 34272256
```

### ELK Stack Integration

```yaml
# docker-compose-elk.yml
version: '3.8'

services:
  elasticsearch:
    image: docker.elastic.co/elasticsearch/elasticsearch:8.0.0
    environment:
      - discovery.type=single-node
      - xpack.security.enabled=false
    ports:
      - "9200:9200"

  kibana:
    image: docker.elastic.co/kibana/kibana:8.0.0
    ports:
      - "5601:5601"
    environment:
      - ELASTICSEARCH_HOSTS=http://elasticsearch:9200

  freelang-c:
    build: .
    environment:
      - LOG_FORMAT=json
      - ELK_ENDPOINT=http://elasticsearch:9200
    depends_on:
      - elasticsearch
```

### Log Analysis

```bash
# View container logs
docker logs freelang-c-prod | tail -100

# Search logs for errors
docker logs freelang-c-prod | grep -i error

# Export logs to file
docker logs freelang-c-prod > application.log 2>&1

# Monitor in real-time
docker logs -f freelang-c-prod
```

---

## Troubleshooting

### Common Issues

#### Issue 1: Container fails to start

```bash
# Check container logs
docker logs freelang-c-prod

# Inspect image
docker inspect freelang-c:1.0

# Rebuild image
docker build -t freelang-c:1.0 --no-cache .
```

#### Issue 2: High memory usage

```bash
# Monitor memory in real-time
docker stats freelang-c-prod

# Set memory limits
docker run --memory 512m freelang-c:1.0

# Debug memory leaks
docker run -e MEM_DEBUG=1 freelang-c:1.0
```

#### Issue 3: Performance degradation

```bash
# Run performance tests
docker exec freelang-c-prod ./freelang-c --benchmark

# Check CPU usage
docker stats freelang-c-prod

# Profile application
docker run -e PROFILING=1 freelang-c:1.0
```

### Health Check

```bash
# Manual health check
docker run freelang-c:1.0 ./freelang-c --health

# Expected output:
# ✅ Health check PASSED
# - Uptime: 3600s
# - Memory: 34MB/256MB
# - CPU: 15%
# - Requests: 1234
```

---

## Performance Tuning

### Compiler Optimization Flags

```makefile
CFLAGS = -O3 -march=native -flto
CXXFLAGS = $(CFLAGS)
LDFLAGS = -Wl,-O2 -Wl,--as-needed
```

### Runtime Environment Variables

```bash
# Maximum worker threads
export FREELANG_MAX_WORKERS=8

# Memory cache size (MB)
export FREELANG_CACHE_SIZE=512

# Log level (DEBUG, INFO, WARN, ERROR)
export FREELANG_LOG_LEVEL=INFO

# Enable performance monitoring
export FREELANG_MONITOR=1
```

### Benchmarking

```bash
# Run performance tests
docker run freelang-c:1.0 ./freelang-c --benchmark

# Generate benchmark report
docker run freelang-c:1.0 ./freelang-c --benchmark > report.json

# Compare with C standard
docker run freelang-c:1.0 ./freelang-c --compare-c
```

---

## Maintenance

### Updates

```bash
# Check for updates
curl https://gogs.dclub.kr/api/v1/repos/kim/freelang-c/releases

# Pull new version
docker pull ghcr.io/kim/freelang-c:latest

# Redeploy with new image
docker-compose up -d --pull always
```

### Backup

```bash
# Backup configuration
docker cp freelang-c-prod:/app/config ./backup/config

# Backup data
docker cp freelang-c-prod:/app/data ./backup/data

# Create image backup
docker save freelang-c:1.0 | gzip > freelang-c-1.0.tar.gz
```

### Cleanup

```bash
# Remove unused images
docker image prune -a

# Remove unused containers
docker container prune

# Remove unused networks
docker network prune

# Full cleanup
docker system prune -a
```

---

## Support & Contact

- **Documentation**: https://gogs.dclub.kr/kim/freelang-c
- **Issue Tracker**: https://gogs.dclub.kr/kim/freelang-c/issues
- **Email**: dev@freelang.io

---

**Version**: 1.0
**Last Updated**: 2026-03-06
**Status**: Production Ready ✅
