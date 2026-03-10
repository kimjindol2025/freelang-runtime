# Project Sovereign-Mesh: Fallback Strategy (중단 시 대안)

## 🛡️ 자동화 중단 시 즉시 대응 가이드

파이프라인이 중간에 막힐 수 있는 상황별 대안을 정리했습니다.

---

## 🚨 상황별 대응 매트릭스

### Scenario 1: Webhook Server 작동 안 함

**증상**:
- GOGS 푸시했지만 배포 안 됨
- Webhook 서버 오류

**즉시 대안**:

```bash
# 대안 1-A: 수동 배포 (즉시)
ssh -p 22253 kimjin@123.212.111.26 << 'EOF'
  cd /home/kimjin/sovereign-mesh-docker
  git pull origin master
  docker build -t freelang-sovereign-mesh:latest .
  docker restart sovereign-mesh-live
