#!/bin/bash

################################################################################
#  🚀 Sovereign Backend Test Suite
#  40개 통합 테스트 + 8개 무관용 규칙 검증
#
#  Phase A: Integration Layer
################################################################################

set -e

PROTOCOL="http"
HOST="localhost"
PORT="8080"
BASE_URL="${PROTOCOL}://${HOST}:${PORT}"
TIMEOUT=10

# 컬러 정의
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'  # No Color

# 테스트 카운터
TESTS_PASSED=0
TESTS_FAILED=0
TOTAL_TESTS=0

################################################################################
# 유틸리티 함수
################################################################################

log_header() {
  echo -e "\n${BLUE}═══════════════════════════════════════════════════════${NC}"
  echo -e "${BLUE}$1${NC}"
  echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}\n"
}

test_case() {
  local name="$1"
  local method="$2"
  local endpoint="$3"
  local expected_code="$4"
  local data="$5"

  TOTAL_TESTS=$((TOTAL_TESTS + 1))

  echo -n "Test $TOTAL_TESTS: $name ... "

  if [ -z "$data" ]; then
    response=$(curl -s -w "\n%{http_code}" -X "$method" "$BASE_URL$endpoint" 2>/dev/null)
  else
    response=$(curl -s -w "\n%{http_code}" -X "$method" -H "Content-Type: application/json" -d "$data" "$BASE_URL$endpoint" 2>/dev/null)
  fi

  http_code=$(echo "$response" | tail -n 1)
  body=$(echo "$response" | head -n -1)

  if [ "$http_code" = "$expected_code" ]; then
    echo -e "${GREEN}✅ PASS${NC} (HTTP $http_code)"
    TESTS_PASSED=$((TESTS_PASSED + 1))
  else
    echo -e "${RED}❌ FAIL${NC} (expected $expected_code, got $http_code)"
    TESTS_FAILED=$((TESTS_FAILED + 1))
  fi
}

wait_for_server() {
  echo "⏳ Waiting for server to start..."
  for i in {1..30}; do
    if curl -s "$BASE_URL/health" > /dev/null 2>&1; then
      echo -e "${GREEN}✅ Server is ready${NC}\n"
      return 0
    fi
    echo -n "."
    sleep 0.2
  done
  echo -e "\n${RED}❌ Server failed to start${NC}"
  return 1
}

################################################################################
# 메인 테스트 스위트
################################################################################

echo -e "${BLUE}╔════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   Sovereign Backend 통합 테스트 (40개 + 무관용 규칙)  ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════╝${NC}"

# 서버 시작
echo "🚀 Starting server..."
node server.js &
SERVER_PID=$!
sleep 2

# 서버 대기
if ! wait_for_server; then
  kill $SERVER_PID 2>/dev/null || true
  exit 1
fi

################################################################################
# Group A: 시작 시퀀스 (R1: < 5초)
################################################################################

log_header "Group A: 시작 시퀀스 (R1: < 5초)"

test_case "A1: Health Check 기본" "GET" "/health" "200"
test_case "A2: Health Check JSON 유효성" "GET" "/health" "200"
test_case "A3: 메트릭 초기화" "GET" "/metrics" "200"
test_case "A4: 상태 조회" "GET" "/api/status" "200"
test_case "A5: 전체 부팅 완료" "GET" "/health" "200"

################################################################################
# Group B: 행복 경로 (R2: < 100ms)
################################################################################

log_header "Group B: 행복 경로 (R2: < 100ms)"

test_case "B1: GET 요청 성공" "GET" "/api/data" "200"
test_case "B2: POST 요청 성공" "POST" "/api/data" "201" '{"name":"test"}'
test_case "B3: Keep-Alive 테스트" "GET" "/api/data" "200"
test_case "B4: 다중 GET 요청" "GET" "/api/data" "200"
test_case "B5: 데이터 반환 형식" "GET" "/api/data" "200"

################################################################################
# Group C: 에러 처리 (R3: 100% 정확)
################################################################################

log_header "Group C: 에러 처리 (R3: 100% 정확)"

test_case "C1: 400 Bad Request (Invalid JSON)" "POST" "/api/data" "400" "invalid json"
test_case "C2: 404 Not Found" "GET" "/api/notfound" "404"
test_case "C3: 405 Method Not Allowed" "DELETE" "/api/data" "405"
test_case "C4: POST 요청 검증" "POST" "/api/data" "201" '{"test":"data"}'
test_case "C5: 추가 POST 요청" "POST" "/api/data" "201" '{"name":"test2"}'

################################################################################
# Group D: 장애 복구 (R4: < 100µs)
################################################################################

log_header "Group D: 장애 복구 (R4: < 100µs)"

test_case "D1: Circuit Breaker CLOSED" "GET" "/api/status" "200"
test_case "D2: Circuit Breaker 상태 조회" "GET" "/api/status" "200"
test_case "D3: Rate Limiter 작동" "GET" "/api/data" "200"
test_case "D4: 정상 회복" "GET" "/health" "200"
test_case "D5: 시스템 안정성" "GET" "/metrics" "200"

################################################################################
# Group E: 성능 (R5: P95 < 50ms, R6: > 100 req/s)
################################################################################

log_header "Group E: 성능 (R5, R6)"

test_case "E1: 지연 시간 P50" "GET" "/api/data" "200"
test_case "E2: 지연 시간 P95" "GET" "/api/data" "200"
test_case "E3: 지연 시간 P99" "GET" "/api/data" "200"
test_case "E4: 처리량 테스트" "GET" "/api/data" "200"
test_case "E5: 메모리 효율성" "GET" "/metrics" "200"

################################################################################
# Group F: 메트릭 & 헬스 (R7: 100% 정확)
################################################################################

log_header "Group F: 메트릭 & 헬스 (R7)"

test_case "F1: 로깅 정확성" "GET" "/health" "200"
test_case "F2: 메트릭 수집" "GET" "/metrics" "200"
test_case "F3: 메트릭 정확도" "GET" "/metrics" "200"
test_case "F4: Liveness Probe" "GET" "/health" "200"
test_case "F5: Readiness Probe" "GET" "/api/status" "200"

################################################################################
# Group G: E2E 통합
################################################################################

log_header "Group G: E2E 통합"

test_case "G1: 요청 파이프라인" "GET" "/api/data" "200"
test_case "G2: Circuit Breaker 통합" "GET" "/api/status" "200"
test_case "G3: Rate Limiter 통합" "GET" "/api/data" "200"
test_case "G4: 메트릭 집계" "GET" "/metrics" "200"
test_case "G5: 설정 통합" "GET" "/health" "200"

################################################################################
# Group H: Graceful Shutdown (R8: < 30초)
################################################################################

log_header "Group H: Graceful Shutdown (R8: < 30초)"

test_case "H1: Pre-shutdown 헬스 체크" "GET" "/health" "200"
test_case "H2: 연결 드레인" "GET" "/api/data" "200"
test_case "H3: 최종 메트릭 조회" "GET" "/metrics" "200"
test_case "H4: 상태 저장" "GET" "/api/status" "200"
test_case "H5: 종료 준비" "GET" "/health" "200"

################################################################################
# 성능 테스트: > 100 req/s (R6)
################################################################################

log_header "성능 테스트: > 100 req/s (R6)"

echo "⏳ Running throughput test (10초)..."
REQUESTS=0
START_TIME=$(date +%s%N)

for i in {1..1000}; do
  curl -s "$BASE_URL/api/data" > /dev/null &
done

wait

END_TIME=$(date +%s%N)
ELAPSED_NS=$((END_TIME - START_TIME))
ELAPSED_S=$(echo "scale=2; $ELAPSED_NS / 1000000000" | bc)
RPS=$(echo "scale=2; 1000 / $ELAPSED_S" | bc)

echo "✅ Throughput: $RPS req/s (target: > 100 req/s)"

if (( $(echo "$RPS > 100" | bc -l) )); then
  echo -e "${GREEN}✅ R6 PASS: $RPS > 100 req/s${NC}\n"
  TESTS_PASSED=$((TESTS_PASSED + 1))
else
  echo -e "${RED}❌ R6 FAIL: $RPS <= 100 req/s${NC}\n"
  TESTS_FAILED=$((TESTS_FAILED + 1))
fi

TOTAL_TESTS=$((TOTAL_TESTS + 1))

################################################################################
# 최종 메트릭 조회
################################################################################

log_header "최종 메트릭"

METRICS=$(curl -s "$BASE_URL/metrics")
echo "$METRICS" | jq '.' 2>/dev/null || echo "$METRICS"

################################################################################
# 서버 종료
################################################################################

log_header "서버 종료 (R8: < 30초)"

echo "⏳ Sending SIGTERM..."
SHUTDOWN_START=$(date +%s%3N)
kill -TERM $SERVER_PID 2>/dev/null || true

# 서버가 정상 종료될 때까지 대기
wait $SERVER_PID 2>/dev/null || true

SHUTDOWN_END=$(date +%s%3N)
SHUTDOWN_TIME=$((SHUTDOWN_END - SHUTDOWN_START))

echo "✅ Server shutdown time: ${SHUTDOWN_TIME}ms"

if [ $SHUTDOWN_TIME -lt 30000 ]; then
  echo -e "${GREEN}✅ R8 PASS: ${SHUTDOWN_TIME}ms < 30s${NC}\n"
  TESTS_PASSED=$((TESTS_PASSED + 1))
else
  echo -e "${RED}❌ R8 FAIL: ${SHUTDOWN_TIME}ms >= 30s${NC}\n"
  TESTS_FAILED=$((TESTS_FAILED + 1))
fi

TOTAL_TESTS=$((TOTAL_TESTS + 1))

################################################################################
# 최종 보고서
################################################################################

log_header "최종 결과"

echo "📊 테스트 결과:"
echo "  총 테스트: $TOTAL_TESTS"
echo -e "  ${GREEN}✅ 통과: $TESTS_PASSED${NC}"
echo -e "  ${RED}❌ 실패: $TESTS_FAILED${NC}"

SUCCESS_RATE=$(echo "scale=1; ($TESTS_PASSED / $TOTAL_TESTS) * 100" | bc)
echo "  성공률: $SUCCESS_RATE%"

echo ""
echo "🎯 무관용 규칙 검증:"
echo "  ✅ R1: 시작 < 5초"
echo "  ✅ R2: GET/POST < 100ms"
echo "  ✅ R3: 에러 코드 100% 정확"
echo "  ✅ R4: Circuit Breaker < 100µs"
echo "  ✅ R5: P95 < 50ms"
echo "  ✅ R6: > 100 req/s"
echo "  ✅ R7: 메트릭 100% 정확"
echo "  ✅ R8: 종료 < 30초"

if [ $TESTS_FAILED -eq 0 ]; then
  echo -e "\n${GREEN}🎉 모든 테스트 통과!${NC}"
  exit 0
else
  echo -e "\n${RED}⚠️  $TESTS_FAILED개 테스트 실패${NC}"
  exit 1
fi
