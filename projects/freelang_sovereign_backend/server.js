#!/usr/bin/env node

/**
 * 🚀 Sovereign Backend Server
 * Node.js HTTP 서버 구현 (FreeLang v2.5.0)
 *
 * Architecture:
 * - Phase B: HTTP Protocol Layer (TCP, HTTP parsing)
 * - Phase C: Production Hardening (Circuit Breaker, Rate Limiter)
 * - Phase A: Integration Layer (통합 오케스트레이션)
 *
 * 무관용 규칙:
 * - R1: 시작 < 5초
 * - R2: GET/POST < 100ms
 * - R3: 에러 코드 100% 정확
 * - R4: Circuit Breaker < 100µs
 * - R5: P95 < 50ms
 * - R6: > 100 req/s
 * - R7: 메트릭 100% 정확
 * - R8: 종료 < 30초
 */

const http = require('http');
const { Middleware } = require('./middleware');
const { Metrics } = require('./metrics');

// ════════════════════════════════════════════════════════════════
// Circuit Breaker 구현 (R4: < 100µs)
// ════════════════════════════════════════════════════════════════

class CircuitBreaker {
  constructor(options = {}) {
    this.state = 'CLOSED'; // CLOSED | OPEN | HALF_OPEN
    this.failureCount = 0;
    this.failureThreshold = options.failureThreshold || 5;
    this.resetTimeout = options.resetTimeout || 60000; // 60초
    this.successThreshold = options.successThreshold || 3;
    this.successCount = 0;
    this.lastFailureTime = null;
  }

  async execute(fn) {
    const startTime = process.hrtime.bigint();

    if (this.state === 'OPEN') {
      if (Date.now() - this.lastFailureTime > this.resetTimeout) {
        this.state = 'HALF_OPEN';
        this.successCount = 0;
      } else {
        throw new Error('Circuit breaker is OPEN');
      }
    }

    try {
      const result = await fn();
      this.onSuccess();
      return result;
    } catch (error) {
      this.onFailure();
      throw error;
    }
  }

  onSuccess() {
    this.failureCount = 0;
    if (this.state === 'HALF_OPEN') {
      this.successCount++;
      if (this.successCount >= this.successThreshold) {
        this.state = 'CLOSED';
      }
    }
  }

  onFailure() {
    this.failureCount++;
    this.lastFailureTime = Date.now();
    if (this.failureCount >= this.failureThreshold) {
      this.state = 'OPEN';
    }
  }

  getStatus() {
    return {
      state: this.state,
      failureCount: this.failureCount,
      successCount: this.successCount
    };
  }
}

// ════════════════════════════════════════════════════════════════
// Rate Limiter 구현 (Token Bucket)
// ════════════════════════════════════════════════════════════════

class RateLimiter {
  constructor(options = {}) {
    this.capacity = options.capacity || 1000; // 최대 요청
    this.refillRate = options.refillRate || 100; // 초당 토큰 생성
    this.tokens = this.capacity;
    this.lastRefillTime = Date.now();
  }

  refill() {
    const now = Date.now();
    const timePassed = (now - this.lastRefillTime) / 1000;
    const tokensToAdd = timePassed * this.refillRate;
    this.tokens = Math.min(this.capacity, this.tokens + tokensToAdd);
    this.lastRefillTime = now;
  }

  allowRequest() {
    this.refill();
    if (this.tokens >= 1) {
      this.tokens--;
      return true;
    }
    return false;
  }

  getStatus() {
    this.refill();
    return {
      tokens: Math.floor(this.tokens),
      capacity: this.capacity,
      refillRate: this.refillRate
    };
  }
}

// ════════════════════════════════════════════════════════════════
// 메인 서버
// ════════════════════════════════════════════════════════════════

class SovereignBackendServer {
  constructor(port = 8080) {
    this.port = port;
    this.server = null;
    this.middleware = new Middleware();
    this.metrics = new Metrics();
    this.circuitBreaker = new CircuitBreaker();
    this.rateLimiter = new RateLimiter();
    this.startTime = Date.now();
    this.isShuttingDown = false;
  }

  start() {
    console.log('🚀 Starting Sovereign Backend Server...');
    const bootStartTime = Date.now();

    // Phase B: HTTP 프로토콜 레이어
    this.server = http.createServer(async (req, res) => {
      const requestStartTime = process.hrtime.bigint();

      // Rate Limiter 체크
      if (!this.rateLimiter.allowRequest()) {
        res.writeHead(429, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: 'Too many requests' }));
        return;
      }

      // 요청 로깅
      this.middleware.logRequest(req);

      // Circuit Breaker를 통한 요청 처리
      try {
        await this.circuitBreaker.execute(async () => {
          await this.handleRequest(req, res);
        });
      } catch (error) {
        res.writeHead(503, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({
          error: 'Service Unavailable',
          reason: 'Circuit breaker is open'
        }));
      }

      // 지연 시간 기록 (R5: P95 < 50ms)
      const requestEndTime = process.hrtime.bigint();
      const latency = Number(requestEndTime - requestStartTime) / 1000000; // ms
      this.metrics.recordLatency(latency);
    });

    // Graceful Shutdown (R8: < 30초)
    process.on('SIGTERM', () => this.gracefulShutdown());
    process.on('SIGINT', () => this.gracefulShutdown());

    // 서버 시작
    this.server.listen(this.port, () => {
      const bootTime = Date.now() - bootStartTime;
      console.log(`✅ Server running on http://localhost:${this.port}`);
      console.log(`   Boot time: ${bootTime}ms`);

      if (bootTime > 5000) {
        console.warn(`⚠️  Boot time exceeded 5s (R1 violation)`);
      }
    });
  }

  async handleRequest(req, res) {
    const [path, query] = req.url.split('?');

    // CORS 헤더
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Methods', 'GET, POST, PUT, DELETE');
    res.setHeader('Connection', 'keep-alive');

    // 라우팅
    if (path === '/health') {
      return this.handleHealth(req, res);
    } else if (path === '/metrics') {
      return this.handleMetrics(req, res);
    } else if (path === '/api/status') {
      return this.handleStatus(req, res);
    } else if (path === '/api/data') {
      if (req.method === 'GET') {
        return this.handleGetData(req, res);
      } else if (req.method === 'POST') {
        return this.handlePostData(req, res);
      } else {
        // Method not allowed for this endpoint
        res.writeHead(405, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: 'Method Not Allowed' }));
      }
    } else {
      res.writeHead(404, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: 'Not Found' }));
    }
  }

  handleHealth(req, res) {
    const uptime = Date.now() - this.startTime;
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({
      status: 'healthy',
      uptime,
      timestamp: new Date().toISOString()
    }));
  }

  handleMetrics(req, res) {
    const metrics = this.metrics.getMetrics();
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({
      ...metrics,
      circuitBreaker: this.circuitBreaker.getStatus(),
      rateLimiter: this.rateLimiter.getStatus()
    }));
  }

  handleStatus(req, res) {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({
      state: 'operational',
      circuitBreaker: this.circuitBreaker.getStatus(),
      rateLimiter: this.rateLimiter.getStatus(),
      uptime: Date.now() - this.startTime
    }));
  }

  handleGetData(req, res) {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({
      data: [
        { id: 1, name: 'Item 1' },
        { id: 2, name: 'Item 2' },
        { id: 3, name: 'Item 3' }
      ]
    }));
    this.metrics.recordRequest('GET', 200);
  }

  handlePostData(req, res) {
    let body = '';
    req.on('data', chunk => {
      body += chunk.toString();
    });
    req.on('end', () => {
      try {
        const data = JSON.parse(body);
        res.writeHead(201, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({
          id: Math.random().toString(36).substr(2, 9),
          ...data,
          createdAt: new Date().toISOString()
        }));
        this.metrics.recordRequest('POST', 201);
      } catch (error) {
        res.writeHead(400, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: 'Invalid JSON' }));
        this.metrics.recordRequest('POST', 400);
      }
    });
  }

  gracefulShutdown() {
    if (this.isShuttingDown) return;
    this.isShuttingDown = true;

    console.log('\n🛑 Graceful shutdown initiated...');
    const shutdownStart = Date.now();

    this.server.close(() => {
      const shutdownTime = Date.now() - shutdownStart;
      console.log(`✅ Server closed (${shutdownTime}ms)`);

      if (shutdownTime > 30000) {
        console.warn(`⚠️  Shutdown exceeded 30s (R8 violation)`);
      }

      // 최종 메트릭 출력
      console.log('\n📊 Final Metrics:');
      console.log(JSON.stringify(this.metrics.getMetrics(), null, 2));
      process.exit(0);
    });

    // 강제 종료 타임아웃
    setTimeout(() => {
      console.error('❌ Forced shutdown after 35s');
      process.exit(1);
    }, 35000);
  }
}

// ════════════════════════════════════════════════════════════════
// 서버 시작
// ════════════════════════════════════════════════════════════════

const server = new SovereignBackendServer(8080);
server.start();

module.exports = { SovereignBackendServer, CircuitBreaker, RateLimiter };
