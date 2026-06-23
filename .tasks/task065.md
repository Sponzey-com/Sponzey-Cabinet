# Task 065. MVP Release Gate Script and Documentation

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 8의 `MVP-113 Documentation update`와 release gate 정리를 완료하는 것이다.
- [x] MVP release gate를 한 번에 실행할 수 있는 개발자 스크립트를 제공한다.
- [x] MVP 사용/개발 문서에 local data location, backup/export 기본 정책, known limitations, developer gate 실행 방법을 명시한다.

## 2. Current Context

- [x] Phase 8의 p95 benchmark, clean install smoke, data preservation smoke, MVP end-to-end smoke가 통과했다.
- [x] 아직 release gate를 한 번에 실행하는 스크립트가 없다.
- [x] local data location, backup/export, known limitations, developer gate가 별도 문서로 정리되어 있지 않다.

## 3. Scope

### Included

- [x] `scripts/mvp_release_gate.sh`를 추가한다.
- [x] `MVP_RELEASE.md`를 추가한다.
- [x] `scripts/check_mvp_release_docs.sh`를 추가해 문서와 release gate 필수 항목을 검증한다.
- [x] Phase 8 gate와 requirement trace를 최종 상태로 갱신한다.

### Excluded

- [x] 실제 packaging artifact 생성은 이번 태스크에서 다루지 않는다.
- [x] UI 자동화 도구 도입은 이번 태스크에서 다루지 않는다.
- [x] iOS/Android/SaaS 구현 문서는 MVP release 문서 범위에서 제외한다.

## 4. Functional Units

### Functional Unit 1

- [x] docs check script는 release 문서와 release gate script의 필수 항목 존재를 검증한다.
- [x] 실패 조건은 필수 파일 또는 필수 문구 누락이다.

### Functional Unit 2

- [x] release gate script는 Rust format/test, architecture boundary, Git CLI absence, frontend/UI/platform boundary smoke를 실행한다.
- [x] 성공 조건은 모든 명령이 0 exit code를 반환하는 것이다.

### Functional Unit 3

- [x] MVP release 문서는 local data location, backup/export policy, known limitations, developer gate, performance/reliability evidence를 포함한다.
- [x] 성공 조건은 docs check script가 통과하는 것이다.

## 5. Architecture Notes

- [x] release gate는 구현 기술보다 사용자-facing 동작과 architecture boundary를 검증한다.
- [x] 문서는 사용자에게 Git, commit, branch, repository 개념을 노출하지 않는다.
- [x] developer gate는 end-user local runtime 요구사항과 구분한다.

## 6. Configuration Rules

- [x] release gate는 runtime 중간 env 변경을 수행하지 않는다.
- [x] 문서는 로컬 기본 실행이 수동 env/config 편집을 요구하지 않는다고 명시한다.
- [x] developer command는 개발 검증용이며 end-user 설치 요구사항이 아니라고 명시한다.

## 7. Logging Requirements

- [x] 문서는 Product Log, Field Debug Log, Development Log 기준을 MVP evidence와 연결한다.
- [x] release gate는 로그 민감정보 검증이 smoke에 포함되어 있음을 명시한다.

## 8. State Machine Requirements

- [x] 문서는 first-run, migration, restore가 명시적 상태 결과로 검증된다고 명시한다.
- [x] release gate는 상태머신을 새로 구현하지 않는다.

## 9. TDD Plan

- [x] 실패하는 docs check script를 먼저 작성하고 실행한다.
- [x] `MVP_RELEASE.md`와 `scripts/mvp_release_gate.sh`를 추가한다.
- [x] docs check script를 통과시킨다.
- [x] release gate script와 전체 품질 게이트를 실행한다.

## 10. Implementation Checklist

- [x] `scripts/check_mvp_release_docs.sh`를 추가한다.
- [x] 실패하는 docs check 결과를 확인한다.
- [x] `scripts/mvp_release_gate.sh`를 추가한다.
- [x] `MVP_RELEASE.md`를 추가한다.
- [x] `.tasks/phase-gates.md`를 최종 evidence로 갱신한다.
- [x] `.tasks/task065.md` completion report를 갱신한다.

## 11. Validation Checklist

- [x] `sh scripts/check_mvp_release_docs.sh`가 통과한다.
- [x] `sh scripts/mvp_release_gate.sh`가 통과한다.
- [x] `cargo fmt --all --check`가 통과한다.
- [x] `cargo test --workspace --quiet`가 통과한다.
- [x] `sh scripts/check_architecture_boundaries.sh`가 통과한다.
- [x] `sh scripts/check_no_git_cli_dependency.sh`가 통과한다.
- [x] `node scripts/check_frontend_boundaries.mjs`가 통과한다.
- [x] `node scripts/check_platform_adapter_smoke.mjs`가 통과한다.
- [x] `sh scripts/check_desktop_shell_boundaries.sh`가 통과한다.

## 12. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - MVP release gate script를 추가해 Rust, architecture, runtime config, logging, domain, UI/editor/platform boundary checks를 한 번에 실행한다.
  - MVP release 문서를 추가해 local data location, backup/export policy, known limitations, developer gate, performance/reliability evidence, 로그 정책, 상태머신 evidence를 정리했다.
  - release docs check script를 추가해 문서와 release gate script의 필수 항목을 검증한다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `MVP_RELEASE.md`
  - `scripts/mvp_release_gate.sh`
  - `scripts/check_mvp_release_docs.sh`
  - `.tasks/task063.md`
  - `.tasks/task064.md`
  - `.tasks/task065.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `sh scripts/check_mvp_release_docs.sh`: pass
  - `sh scripts/mvp_release_gate.sh`: pass
  - release gate 내부 `cargo fmt --all --check`: pass
  - release gate 내부 `cargo test --workspace --quiet`: pass
  - release gate 내부 architecture/runtime/logging/domain/UI/platform checks: pass
- [x] 남은 위험 요소를 기록한다.
  - 실제 packaging artifact 생성과 UI 자동화는 MVP release gate 바깥이다.
  - iOS/Android/SaaS/협업/AI/플러그인/CRM/Canvas는 MVP 범위 밖이다.
- [x] 후속 태스크에서 이어받아야 할 내용을 기록한다.
  - MVP 목표는 현재 release gate 기준으로 완료되었다. 다음 단계 작업은 별도 phase decision 이후 시작한다.

## 13. Next Task Decision Hook

- [x] `plan.md`의 최종 목표에 도달했는지 확인한다.
- [x] 도달했다면 추가 태스크를 생성하지 않는다.
- [x] 도달하지 못했다면 남은 목표를 정리한다.
- [x] 다음 우선순위 작업을 선택한다.
- [x] 다음 태스크가 기능 2~3개 단위를 넘지 않도록 범위를 제한한다.
- [x] 다음 태스크를 `taskXXX.md`로 생성한다.
  - MVP 목표에 도달했으므로 후속 태스크를 생성하지 않는다.
- [x] 다음 태스크를 즉시 실행한다.
  - MVP 목표에 도달했으므로 실행할 다음 태스크가 없다.

## 14. Stop Conditions

- [x] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.
