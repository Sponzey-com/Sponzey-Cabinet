# Sponzey Cabinet

Sponzey Cabinet은 개인 사용자가 자신의 PC에 설치해 문서, 파일, 링크, Graph와 Canvas를 직접 소유하고 관리하는 로컬 우선 지식 관리 앱이다. 현재 제품 및 릴리스 검증 범위는 macOS 단일 사용자 데스크톱 앱이다. 멀티 사용자, 서버 호스팅, SaaS와 실시간 협업은 사용자의 명시적 요구 전까지 개발 범위에 포함하지 않는다.

## Core Document Experience

- React와 CodeMirror 기반 Markdown 작성 및 preview
- Markdown 첫 번째 물리적 줄에서 파생되는 문서 제목
- 문서 화면에서 파일 첨부, 문서별 첨부 목록, 미리보기/열기와 연결 해제
- 현재 문서 대 과거 version 및 과거 version 간 줄 단위 diff
- 복원 전 diff preview와 version conflict 방지
- 과거 snapshot으로 새 version을 생성하는 비파괴 복원
- Wikilink, Markdown link, backlink, 검색과 Graph projection
- 문서, 첨부와 자유 배치 노드를 연결하는 Canvas
- 문서 current/history, Asset과 Canvas를 포함하는 로컬 backup/restore

첨부 원본은 Markdown과 분리된 content-addressed asset store에 저장한다. 문서는 stable asset identity만 참조하며, 한 문서에서 연결을 해제해도 다른 문서나 Canvas의 참조가 남아 있으면 원본을 삭제하지 않는다.

복원은 기존 이력을 덮어쓰지 않는다. 사용자가 preview한 current version이 그대로일 때 대상 snapshot을 내용으로 하는 새 version을 만들고 current를 전환하므로, 복원 직전 상태도 계속 비교하거나 다시 복원할 수 있다. 내부 version ID, 파일 경로와 Git 용어는 일반 사용자 UI에 노출하지 않는다.

## Run

macOS Tauri 개발 앱을 실행한다.

```sh
scripts/run_desktop_app.sh
```

개발용 Web preview를 실행한다.

```sh
scripts/run_web_app.sh
```

`scripts/run_desktop_shell.sh`는 GUI가 아니라 native command boundary smoke이므로 실제 화면 확인에는 사용하지 않는다.

## Project Documents

- [PROJECT.md](PROJECT.md): 최종 제품 목표와 기능 계약
- [AGENTS.md](AGENTS.md): 아키텍처, TDD, 설정, 로그와 상태머신 규칙
- [ROADMAP.md](ROADMAP.md): 단계별 범위, 산출물과 완료 조건
- [MVP_RELEASE.md](MVP_RELEASE.md): 현재 macOS 로컬 릴리스 범위와 검증 방법
- [RESEARCH.md](RESEARCH.md): 참고 제품과 OSS 조사 기록
