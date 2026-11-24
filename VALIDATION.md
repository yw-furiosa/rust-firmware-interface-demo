# 검증 기록

2026-11-24에 **Ubuntu 22.04.5 LTS, x86_64 Linux**, Rust 1.94.1에서 확인했다. 호스트와 기본 실험 타깃은 `x86_64-unknown-linux-gnu`다.

## Cargo 빌드와 실행

| 실행 위치와 명령 | 결과 |
| --- | --- |
| 루트: `cargo test --workspace --locked --offline` | 종료 코드 0; 서비스 상태 검사 1개와 compile-fail doctest 4개 통과 |
| 루트: `cargo run --locked --offline -p firmware-sim` | 종료 코드 0; 두 번의 호출에서 receipt 1, 2와 카운터·checksum 확인 |
| 독립 application: `cargo build --locked --target x86_64-unknown-linux-gnu` | 종료 코드 0 |
| 독립 application: `cargo elf-object` | 종료 코드 0; x86_64 ELF64 ET_REL 생성 |
| 독립 application: `cargo rustc --lib --locked --target riscv64imac-unknown-none-elf -- --emit=obj` | 종료 코드 0; RISC-V ELF64 ET_REL 생성 |
| 루트와 독립 application의 Cargo 형식 검사 | 종료 코드 0 |

Application 검증은 `application/` 폴더만 펌웨어 소스 트리 밖의 새 디렉터리로 복사한 뒤 수행했다. 빌드 과정에서 Python이나 `build.rs`를 사용하지 않았다. 공유 API는 Git 의존성으로 가져왔고, `cargo metadata`로 다음 조건을 확인했다.

- Application의 의존성은 `fw-api` 하나이며 `firmware-services`는 포함되지 않는다.
- API는 고정한 Git revision `8276cfb5b544997b1bdf6d9e390f3bfbd84fc8d8`에서 가져온다.
- Git checkout의 API 소스와 루트의 API 소스는 SHA-256이 같다. 두 빌드의 `.rmeta` 파일이 같다는 뜻은 아니다.

## 서비스 코드와 상태의 위치

`nm -C --defined-only`로 독립 application object와 펌웨어 실행 파일을 확인했다.

| 심볼 | Application object | Firmware 실행 파일 |
| --- | --- | --- |
| `plugin_entry` | 있음 | 있음: 호스트 시뮬레이션용으로 정적 링크 |
| `firmware_services::new_buffer_impl` | 없음 | 있음 |
| `firmware_services::bytes_impl` | 없음 | 있음 |
| `firmware_services::submit_impl` | 없음 | 있음 |
| `firmware_services::CREATED`, `SUBMITTED`, `LAST_CHECKSUM` | 없음 | 있음 |

공유 API에 서비스 구현이나 상태를 두지 않고, application이 테이블의 함수 포인터로 호출하는 구조를 확인했다. Application의 작은 Rust 지원 코드나 generic 코드까지 두 바이너리 사이에서 중복되지 않는다는 보장은 아니다.

## 추가 메타데이터 실험과 범위

루트의 `python3 scripts/verify.py`도 종료 코드 0으로 통과했다. 이 추가 실험은 `.rmeta`만 지정한 컴파일을 확인하고, lifetime 연장·move 후 사용·borrow 중 move·제공자 함수 타입 불일치를 각각 종료 코드 1과 예상 진단으로 확인한다. 재실행 시 전체 명령과 로그는 `artifacts/verification.json` 및 `artifacts/logs/`에 저장된다.

호스트 시뮬레이터는 application의 같은 Rust 소스를 정적으로 링크한다. 별도로 생성한 ELF를 로드하지 않는다. 실제 보드 실행, 런타임 ELF 로딩, relocation 적용, SDK 식별자 검증은 수행하지 않았다.
