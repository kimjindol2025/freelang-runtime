# Project Sovereign-Mesh: 5-Layer Mesh Network Architecture
# Docker: Multi-stage build for FreeLang v2.2.0

FROM ubuntu:22.04 AS builder

LABEL maintainer="kim@freelang.dev"
LABEL project="freelang-sovereign-mesh"
LABEL description="Decentralized mesh networking system (L0-L3 complete stack)"

# Install dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    git \
    curl \
    wget \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Clone Project Sovereign-Mesh from GOGS
RUN git clone https://gogs.dclub.kr/kim/freelang-sovereign-mesh.git . && \
    git log --oneline -5

# Final stage
FROM ubuntu:22.04

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    git \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy from builder
COPY --from=builder /app /app

# Metadata
ENV PROJECT="freelang-sovereign-mesh"
ENV VERSION="1.0.0"
ENV ARCHITECTURE="L0-L1-L2-L3"
ENV CHALLENGES="C16+C17+L0"
ENV TESTS="18 (100% unforgiving)"
ENV RULES="4/4 (100% achieved)"

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD git -C /app log --oneline -1 || exit 1

# Default command
CMD ["/bin/bash"]

# Build metadata
ONBUILD LABEL build.timestamp="$(date -u +'%Y-%m-%dT%H:%M:%SZ')"
ONBUILD LABEL build.git.commit="$(git rev-parse --short HEAD)"

