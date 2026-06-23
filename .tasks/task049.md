# Task 049. HTML/PDF Export Boundary

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 6의 `MVP-072 HTML/PDF export foundation`을 구현하는 것이다.
- [x] 이 태스크는 Markdown export 이후 HTML rendering boundary를 제공하고 PDF export는 MVP에서 unsupported로 명시한다.

## 2. Scope

- [x] `HtmlRenderer` port를 추가한다.
- [x] `HtmlDocument` output model을 정의한다.
- [x] local simple HTML renderer adapter를 추가한다.
- [x] renderer는 heading과 paragraph를 최소 지원한다.
- [x] renderer는 HTML special character를 escape한다.
- [x] `ExportPdfUsecase`는 unsupported result를 반환한다.

## 3. TDD Plan

- [x] 실패하는 html renderer escapes heading and paragraph test를 먼저 작성한다.
- [x] 실패하는 html renderer contract rejects empty body test를 먼저 작성한다.
- [x] 실패하는 pdf export returns unsupported test를 먼저 작성한다.

## 4. Architecture Rules

- [x] HTML renderer port는 domain document body만 참조한다.
- [x] local renderer adapter는 filesystem, env, browser, network를 사용하지 않는다.
- [x] PDF export는 hidden dependency나 external binary 호출 없이 unsupported로 반환한다.
- [x] renderer output은 file write가 아니라 HTML content model이다.
- [x] 사용자에게 Git, commit, PR 개념을 노출하지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `HtmlRenderer` port와 `HtmlDocument` output model을 추가했다.
  - `LocalHtmlRenderer` adapter를 추가해 heading/paragraph rendering과 HTML escaping을 구현했다.
  - `ExportPdfUsecase`는 MVP에서 `Unsupported`를 반환한다.
  - PDF export는 외부 binary, browser, filesystem 호출을 수행하지 않는다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-ports/src/html_renderer.rs`
  - `crates/cabinet-ports/src/lib.rs`
  - `crates/cabinet-ports/tests/html_renderer_contract_tests.rs`
  - `crates/cabinet-adapters/src/local_html_renderer.rs`
  - `crates/cabinet-adapters/src/lib.rs`
  - `crates/cabinet-adapters/tests/local_html_renderer_tests.rs`
  - `crates/cabinet-usecases/src/export.rs`
  - `crates/cabinet-usecases/tests/export_pdf_tests.rs`
  - `.tasks/task049.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-ports --test html_renderer_contract_tests --quiet` 통과
  - `cargo test -p cabinet-adapters --test local_html_renderer_tests --quiet` 통과
  - `cargo test -p cabinet-usecases --test export_pdf_tests --quiet` 통과
  - `cargo fmt --all --check` 통과
  - `cargo test --workspace --quiet` 통과
  - `sh scripts/check_architecture_boundaries.sh` 통과
  - `sh scripts/check_no_git_cli_dependency.sh` 통과
- [x] 다음 태스크를 결정한다.
  - Phase 6을 완료 처리하고 Phase 7 UI/platform integration으로 진입한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.
