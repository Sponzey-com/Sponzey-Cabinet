# Decision Records

이 디렉터리는 `.tasks/plan.md`의 Decision Records를 실제 구현 전에 고정하기 위한 공간이다. 필수 decision record는 지정된 deadline 이전에 작성되어야 하며, AGENTS 원칙과 충돌하면 승인하지 않는다.

## Decision Record Template

```text
# Decision XXX. Title

## Context

## Options

## Selected Option

## Rejected Options

## Architecture Impact

## Configuration Impact

## Logging Impact

## State Machine Impact

## Test Strategy

## Rollback or Migration Plan

## Review Owner
```

## Required Decisions

| Decision | Deadline | Status | Required Criteria | Rejection Criteria |
| --- | --- | --- | --- | --- |
| Local metadata store | Phase 4 시작 전 | not-started | 외부 DB 서버 없이 설치 1회로 초기화된다. migration runner와 contract test가 가능하다. current snapshot metadata 조회가 300ms 목표를 방해하지 않는다. | 수동 DB 설치, runtime daemon 필수, 숨겨진 global connection |
| Internal version store | Phase 4 시작 전 | not-started | Git CLI 없이 동작한다. current snapshot과 history 조회가 분리된다. corruption detection과 recovery test가 가능하다. | 사용자-facing Git 개념 노출, history 전체 스캔 기반 current 조회 |
| Current snapshot layout | Phase 4 시작 전 | not-started | latest snapshot direct read가 가능하다. atomic write와 partial recovery가 가능하다. | version history를 매번 순회해야 최신 문서를 알 수 있는 구조 |
| Local search index | Phase 6 시작 전 | not-started | 외부 검색 서버 없이 동작한다. rebuild가 가능하다. p95 300ms benchmark가 가능하다. | 원본 파일 전체 스캔이 기본 조회 경로인 구조 |
| Markdown/MDX parser | Phase 6 시작 전 | not-started | Markdown link, Wikilink, heading, asset reference 추출이 가능하다. parser output을 domain value object로 변환할 수 있다. | parser 내부에 document lifecycle 또는 storage rule을 넣어야 하는 구조 |
| Asset reference syntax | Phase 3 종료 전 | not-started | 문서 본문에 원본 파일을 넣지 않고 asset id/reference만 표현한다. export/import roundtrip이 가능하다. | 파일 경로 원문을 영구 식별자로 사용하는 구조 |
| Import conflict policy | Phase 6 시작 전 | not-started | duplicate path, unsupported file, partial failure, broken link를 명시적으로 처리한다. | 실패 시 workspace를 오염시키는 구조 |
| Export output policy | Phase 6 시작 전 | not-started | Markdown export와 HTML export가 데이터 이동성을 보장한다. PDF는 unsupported 또는 async extension boundary로 명시한다. | export가 내부 storage schema에 종속되는 구조 |
| Product/Field/Development log event naming | Phase 2 시작 전 | not-started | stable event name, error code, masking 기준이 있다. | free text only log, 민감 정보 포함 가능성 |
| Performance fixture shape | Phase 8 시작 전 | not-started | 문서 수, 평균 본문 크기, 링크 수, 첨부 수, 측정 환경이 고정된다. | 측정 환경과 데이터 크기를 기록하지 않는 benchmark |

## Review Rules

- AGENTS 원칙과 충돌하면 결정하지 않는다.
- 설정 정책 완화를 요구하는 선택지는 거부한다.
- Product Log에 원문 데이터를 요구하는 선택지는 거부한다.
- Field Debug Log가 scope와 expiry 없이 활성화되는 선택지는 거부한다.
- 상태가 3개 이상인데 상태머신이 없는 선택지는 보류한다.
- TDD를 어렵게 만드는 선택지는 경계 설계를 수정한 뒤 재검토한다.
- 로컬 기본 실행에 외부 DB, 검색 서버, Git CLI, Node.js, 수동 env, 수동 설정 파일을 요구하는 선택지는 거부한다.
- current document 조회가 version history 전체 스캔에 의존하는 선택지는 거부한다.
- 사용자-facing UI/API에 Git commit, branch, repository 개념을 노출하는 선택지는 거부한다.

## Naming

- 파일명은 `decisionXXX-short-title.md` 형식을 사용한다.
- 번호는 3자리 zero-padding을 사용한다.
- decision record는 이전 번호를 건너뛰지 않는다.
