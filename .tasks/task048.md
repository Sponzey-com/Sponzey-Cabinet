# Task 048. Markdown Export Usecase

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 6의 `MVP-071 Markdown export` usecase를 구현하는 것이다.
- [x] 이 태스크는 current document를 filesystem write 없이 export file plan으로 변환한다.

## 2. Scope

- [x] `ExportMarkdownInput/Output`을 추가한다.
- [x] export state enum을 정의한다.
- [x] document id 목록을 명시적 input으로 받는다.
- [x] current document repository에서 문서를 조회한다.
- [x] found document는 path/content 형태의 `ExportedMarkdownFile`로 반환한다.
- [x] missing document는 failed item으로 기록하고 나머지 export를 계속한다.
- [x] 모든 요청 실패 시 `Failed`, 일부 실패 시 `PartiallyFailed`, 모두 성공 시 `Completed`를 반환한다.

## 3. TDD Plan

- [x] 실패하는 export success returns markdown files test를 먼저 작성한다.
- [x] 실패하는 export preserves asset reference text test를 먼저 작성한다.
- [x] 실패하는 missing document produces partial failure test를 먼저 작성한다.

## 4. Architecture Rules

- [x] usecase는 `DocumentRepository` port만 받는다.
- [x] usecase는 filesystem, env, concrete adapter, UI export picker를 import하지 않는다.
- [x] export output은 write command가 아니라 file plan이다.
- [x] export는 version history를 scan하지 않는다.
- [x] 사용자에게 Git, commit, PR 개념을 노출하지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `ExportMarkdownUsecase`를 추가했다.
  - usecase는 current document repository에서 문서를 조회해 path/content export file plan을 반환한다.
  - asset reference text는 Markdown content 안에 그대로 보존된다.
  - missing document는 failed item으로 수집하고 나머지 export를 계속한다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-usecases/src/export.rs`
  - `crates/cabinet-usecases/src/lib.rs`
  - `crates/cabinet-usecases/tests/export_markdown_tests.rs`
  - `.tasks/task048.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-usecases --test export_markdown_tests --quiet` 통과
  - `cargo fmt --all --check` 통과
  - `cargo test --workspace --quiet` 통과
  - `sh scripts/check_architecture_boundaries.sh` 통과
  - `sh scripts/check_no_git_cli_dependency.sh` 통과
- [x] 다음 태스크를 결정한다.
  - Task 049는 HTML/PDF export boundary를 구현한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.
