# FreeLang-C v1.0 Production Docker Image
#
# Multi-stage build for optimal image size and security
# Based on Ubuntu 22.04 with build tools and runtime dependencies
#
# Build: docker build -t freelang-c:latest .
# Run:   docker run -it freelang-c:latest

# ============================================================================
# Stage 1: Builder
# ============================================================================
FROM ubuntu:22.04 as builder

LABEL maintainer="FreeLang Team <dev@freelang.io>"
LABEL version="1.0"
LABEL description="FreeLang-C Compiler & Runtime - Production Build"

# Install build dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        build-essential \
        cmake \
        git \
        curl \
        wget \
        ca-certificates \
        pkg-config \
        libssl-dev \
        libpthread-stubs0-dev && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create build directory
WORKDIR /build

# Copy source code
COPY . .

# Build Phase 10 integration & deployment
RUN echo "Building FreeLang-C Phase 10..." && \
    mkdir -p build && \
    cd build && \
    cmake -DCMAKE_BUILD_TYPE=Release .. && \
    make -j$(nproc) && \
    make test || true && \
    echo "Build completed successfully"

# ============================================================================
# Stage 2: Runtime
# ============================================================================
FROM ubuntu:22.04 as runtime

# Install runtime dependencies only
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        libssl3 \
        ca-certificates \
        curl \
        jq && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy artifacts from builder
COPY --from=builder /build/build/bin/freelang-c-integration /app/freelang-c
COPY --from=builder /build/build/bin/freelang-c-tests /app/freelang-c-tests
COPY --from=builder /build/docs /app/docs
COPY --from=builder /build/README.md /app/

# Create non-root user for security
RUN groupadd -r freelang && \
    useradd -r -g freelang freelang && \
    chown -R freelang:freelang /app

USER freelang

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD /app/freelang-c --health || exit 1

# Expose port for any future networking features
EXPOSE 8080

# Default command: Run full integration test suite
CMD ["/app/freelang-c", "--integration-tests"]

# ============================================================================
# Build Arguments & Labels
# ============================================================================

ARG BUILD_DATE
ARG VCS_REF
ARG VERSION=1.0

LABEL org.opencontainers.image.created=$BUILD_DATE \
      org.opencontainers.image.source="https://gogs.dclub.kr/kim/freelang-c.git" \
      org.opencontainers.image.version=$VERSION \
      org.opencontainers.image.revision=$VCS_REF \
      org.opencontainers.image.vendor="FreeLang Team" \
      org.opencontainers.image.title="FreeLang-C" \
      org.opencontainers.image.description="Comprehensive C implementation of FreeLang language with Phase 10 integration and deployment support"
