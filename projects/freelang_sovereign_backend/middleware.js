/**
 * 🔧 Middleware Pipeline
 * 요청 처리 파이프라인 (로깅, CORS, 에러 처리)
 *
 * Phase C: Production Hardening
 */

class Middleware {
  constructor() {
    this.requestLog = [];
    this.errorLog = [];
  }

  logRequest(req) {
    const logEntry = {
      timestamp: new Date().toISOString(),
      method: req.method,
      url: req.url,
      headers: {
        host: req.headers.host,
        userAgent: req.headers['user-agent'],
        contentType: req.headers['content-type']
      }
    };
    this.requestLog.push(logEntry);
    // Keep only last 1000 logs
    if (this.requestLog.length > 1000) {
      this.requestLog.shift();
    }
  }

  logError(error, context) {
    const errorEntry = {
      timestamp: new Date().toISOString(),
      error: error.message,
      stack: error.stack,
      context
    };
    this.errorLog.push(errorEntry);
    // Keep only last 100 errors
    if (this.errorLog.length > 100) {
      this.errorLog.shift();
    }
    console.error(`❌ Error: ${error.message}`, context);
  }

  // 요청 검증
  validateRequest(req) {
    const validMethods = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'OPTIONS'];
    if (!validMethods.includes(req.method)) {
      return {
        valid: false,
        statusCode: 405,
        error: 'Method Not Allowed'
      };
    }

    // Content-Type 검증 (POST/PUT/PATCH)
    if (['POST', 'PUT', 'PATCH'].includes(req.method)) {
      const contentType = req.headers['content-type'];
      if (contentType && !contentType.includes('application/json')) {
        return {
          valid: false,
          statusCode: 400,
          error: 'Invalid Content-Type (expected application/json)'
        };
      }
    }

    return { valid: true };
  }

  // 요청 크기 제한 (413)
  checkContentLength(req) {
    const maxSize = 1024 * 1024; // 1MB
    const contentLength = parseInt(req.headers['content-length'] || 0, 10);
    if (contentLength > maxSize) {
      return {
        tooLarge: true,
        statusCode: 413,
        error: `Payload Too Large (max: ${maxSize} bytes)`
      };
    }
    return { tooLarge: false };
  }

  // CORS 헤더 설정
  setCORSHeaders(res) {
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Methods', 'GET, POST, PUT, DELETE, PATCH, OPTIONS');
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization');
    res.setHeader('Access-Control-Max-Age', '3600');
  }

  // Keep-Alive 설정
  setKeepAliveHeaders(res) {
    res.setHeader('Connection', 'keep-alive');
    res.setHeader('Keep-Alive', 'timeout=5, max=100');
  }

  // 응답 헤더 설정
  setResponseHeaders(res, contentType = 'application/json') {
    this.setCORSHeaders(res);
    this.setKeepAliveHeaders(res);
    res.setHeader('Content-Type', contentType);
    res.setHeader('X-Powered-By', 'Sovereign-Backend/1.0');
    res.setHeader('Date', new Date().toUTCString());
  }

  // 에러 응답 생성
  sendError(res, statusCode, error) {
    this.setResponseHeaders(res);
    res.writeHead(statusCode);
    res.end(JSON.stringify({
      error,
      statusCode,
      timestamp: new Date().toISOString()
    }));
  }

  // 성공 응답 생성
  sendSuccess(res, data, statusCode = 200) {
    this.setResponseHeaders(res);
    res.writeHead(statusCode);
    res.end(JSON.stringify(data));
  }

  // 요청 로그 조회
  getRequestLog(limit = 100) {
    return this.requestLog.slice(-limit);
  }

  // 에러 로그 조회
  getErrorLog(limit = 50) {
    return this.errorLog.slice(-limit);
  }

  // 요청 통계
  getStatistics() {
    const methods = {};
    const paths = {};

    this.requestLog.forEach(log => {
      methods[log.method] = (methods[log.method] || 0) + 1;
      const path = log.url.split('?')[0];
      paths[path] = (paths[path] || 0) + 1;
    });

    return {
      totalRequests: this.requestLog.length,
      methods,
      paths,
      totalErrors: this.errorLog.length
    };
  }
}

module.exports = { Middleware };
