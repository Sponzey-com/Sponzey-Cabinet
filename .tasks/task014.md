# Task 014. Document Body Domain Model

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 3의 `MVP-012 Document body model`을 구현하는 것이다.
- [x] 이 태스크는 editor buffer나 storage file이 아닌 normalized document body value object를 정의한다.
- [x] 이 태스크 완료 후 프로젝트는 line ending normalization, size limit, Unicode preservation 테스트를 가진다.

## 2. Current Context

- [x] 현재 코드베이스는 workspace와 document metadata domain model을 가진다.
- [x] 이전 태스크 Task 013에서 document id/title/path/slug/metadata를 완료했다.
- [x] 이번 태스크는 document body의 최소 domain 정책을 정의했다.
- [x] 현재 확인된 제약 사항은 document body가 editor buffer, rendered HTML, storage file path와 섞이면 안 된다는 점이다.

## 3. Scope

### Included

- [x] `DocumentBody` value object를 추가한다.
- [x] line ending normalization을 추가한다.
- [x] explicit `DocumentBodyPolicy` 기반 size limit을 추가한다.

### Excluded

- [x] markdown parsing은 후속 parser adapter 태스크로 넘긴다.
- [x] editor state/selection/cursor model은 이번 태스크에서 구현하지 않는다.
- [x] version snapshot model은 후속 version domain 태스크로 넘긴다.

## 4. Functional Units

이번 태스크는 기능 3개 단위로만 구성했다.

### Functional Unit 1

- [x] 구현한 기능: document body normalization을 만들었다.
- [x] 입력: raw markdown text.
- [x] 출력: normalized `DocumentBody`.
- [x] 성공 조건: CRLF와 CR line ending은 LF로 정규화된다.
- [x] 실패 조건: normalization이 adapter나 UI에 흩어진다.

### Functional Unit 2

- [x] 구현한 기능: body size policy를 만들었다.
- [x] 입력: max bytes policy와 raw text.
- [x] 출력: valid body 또는 `BodyTooLarge`.
- [x] 성공 조건: policy는 config/global이 아니라 명시적 value object로 전달된다.
- [x] 실패 조건: domain이 runtime config나 environment를 읽는다.

### Functional Unit 3

- [x] 구현한 기능: Unicode preservation을 검증했다.
- [x] 입력: Korean text, emoji, mixed markdown text.
- [x] 출력: same Unicode scalar sequence after line normalization.
- [x] 성공 조건: Unicode 내용이 손상되지 않는다.
- [x] 실패 조건: ASCII-only normalization이나 lossy conversion이 발생한다.

## 5. Architecture Notes

- [x] 변경되는 계층은 pure domain이다.
- [x] domain은 filesystem, editor, parser, renderer, DB, network, env, logger에 의존하지 않는다.
- [x] document body는 document metadata와 분리된다.
- [x] document body는 asset 원본을 포함하지 않는다.
- [x] body policy는 명시적 인자로 전달한다.

## 6. Configuration Rules

- [x] domain은 config object를 직접 읽지 않는다.
- [x] process environment를 읽지 않는다.
- [x] size limit은 `DocumentBodyPolicy`로 명시적으로 전달한다.
- [x] runtime 중간 설정 변경 API를 만들지 않는다.
- [x] 테스트는 외부 파일이나 환경 값 없이 실행한다.

## 7. Logging Requirements

- [x] domain은 어떤 logger도 호출하지 않는다.
- [x] document body 원문을 로그 payload로 만들지 않는다.
- [x] validation detail은 테스트 assertion으로만 검증한다.

## 8. State Machine Requirements

- [x] 이번 태스크는 상태머신을 추가하지 않는다.
- [x] document lifecycle state machine은 후속 태스크로 넘긴다.
- [x] boolean flag 조합으로 document 절차를 관리하지 않는다.

## 9. TDD Plan

- [x] 실패하는 line ending normalization test를 먼저 작성했다.
- [x] 실패하는 body size limit test를 먼저 작성했다.
- [x] 실패하는 Unicode preservation test를 먼저 작성했다.
- [x] 테스트를 통과하는 최소 구현만 작성했다.
- [x] 외부 의존성은 추가하지 않았다.

## 10. Implementation Checklist

- [x] domain tests를 먼저 작성했다.
- [x] 구현 전 테스트 실패를 확인했다.
- [x] document body type을 작성했다.
- [x] domain boundary check가 계속 통과하는지 확인했다.
- [x] 모든 관련 검증 명령을 실행했다.

## 11. Validation Checklist

- [x] 기능 요구사항이 충족되었다.
- [x] 테스트가 모두 통과한다.
- [x] 실패 테스트가 먼저 작성되었다.
- [x] 도메인 계층이 외부 프레임워크에 의존하지 않는다.
- [x] body policy가 명시적 입력으로 전달된다.
- [x] 외부 환경 값이 런타임 중간에 재조회되지 않는다.
- [x] document body 원문이 로그 모델과 섞이지 않는다.

## 12. Completion Report

태스크 완료 결과는 다음이다.

- [x] 수행한 변경 사항:
  - `DocumentBody`와 `DocumentBodyPolicy`를 추가했다.
  - CRLF/CR to LF normalization을 추가했다.
  - explicit max bytes policy와 `BodyTooLarge`, `InvalidBodyPolicy` error를 추가했다.
  - Unicode preservation tests를 추가했다.
- [x] 생성하거나 수정한 파일:
  - `crates/cabinet-domain/src/document.rs`
  - `crates/cabinet-domain/tests/document_body_tests.rs`
  - `.tasks/task013.md`
  - `.tasks/task014.md`
- [x] 실행한 테스트 명령과 결과:
  - `cargo test -p cabinet-domain document_body`: 최초 실행은 `DocumentBody`, `DocumentBodyPolicy`, body error variant 없음으로 실패했고, 구현 후 4개 body 테스트가 통과했다.
  - `sh scripts/check_domain_boundaries.sh`: 통과.
  - `cargo test --workspace`: 통과.
  - `cargo fmt --all --check`: 통과.
  - `sh scripts/check_architecture_boundaries.sh`: 통과.
- [x] 검증한 항목:
  - CRLF와 CR line ending은 LF로 정규화된다.
  - body size limit은 명시적 `DocumentBodyPolicy`로 전달된다.
  - 초과 body는 stable domain error로 실패한다.
  - Korean text와 emoji가 normalization 이후 보존된다.
  - domain source에는 filesystem/env/network/logger/framework 접근이 없다.
- [x] 남은 위험 요소:
  - markdown parsing과 link extraction은 아직 없다.
  - document lifecycle state machine은 아직 없다.
  - body와 asset reference 분리는 asset/link domain에서 추가 검증해야 한다.
- [x] 후속 태스크에서 이어받아야 할 내용:
  - 다음 태스크는 `MVP-013 Document lifecycle state machine`을 시작한다.

## 13. Next Task Decision Hook

이 태스크 완료 후 판단 결과는 다음이다.

- [x] `plan.md`의 최종 목표에 도달했는지 확인했다.
- [x] 최종 목표에는 도달하지 못했다.
- [x] 남은 목표는 document lifecycle, asset/version/link domain, adapters, usecases, UI, release gate다.
- [x] 남은 목표 중 가장 우선순위가 높은 작업은 `MVP-013 Document lifecycle state machine`이다.
- [x] 다음 태스크는 기능 2~3개 단위를 넘지 않도록 lifecycle states/events, valid transitions, invalid transitions로 제한한다.
- [x] 다음 태스크가 테스트와 검증을 포함하도록 정의한다.
- [x] 다음 태스크가 `AGENTS.md` 원칙과 충돌하지 않는지 확인했다.
- [x] 다음 태스크 파일명은 `.tasks/task015.md`다.
- [x] 다음 태스크를 `taskXXX.md`로 생성했다.
- [x] 다음 태스크 생성을 완료한 뒤 즉시 실행을 시작한다.

## 14. Stop Conditions

다음 조건을 확인했다.

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.
- [ ] 외부 정보, 권한, 비밀값, 접근 권한이 없어 진행할 수 없다.
- [ ] `AGENTS.md` 원칙과 충돌하는 요구사항이 발견되었다.
- [ ] 테스트 또는 검증 환경이 없어 완료 여부를 판단할 수 없다.
- [ ] 코드베이스 구조가 계획과 크게 달라 태스크 재설계가 필요하다.
- [ ] 사용자 결정이 필요한 아키텍처 선택지가 발생했다.
