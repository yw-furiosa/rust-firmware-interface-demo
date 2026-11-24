# 검증 기록

2026-11-24에 **Ubuntu 22.04.5 LTS, x86_64 Linux**에서 확인했다. Rust 1.94.1을 사용했으며, 호스트와 기본 실험 타깃은 `x86_64-unknown-linux-gnu`다.

| 명령 또는 검사 | 결과 |
| --- | --- |
| `python3 scripts/verify.py` | 종료 코드 0; 전체 검증 성공 |
| Workspace 형식 검사·빌드·호스트 실행 | 각각 종료 코드 0 |
| `.rmeta`를 지정한 플러그인 object 컴파일 | 종료 코드 0 |
| Lifetime 연장, move 후 사용, borrow 중 move, 제공자 타입 불일치 | 각각 종료 코드 1과 예상 진단 확인 |
| `python3 scripts/build_sdk.py --target x86_64-unknown-linux-gnu` | 종료 코드 0 |
| x86_64 object의 `readelf -h`, `readelf -r`, `nm -u` | 각각 종료 코드 0; ELF64, ET_REL, x86-64 확인 |
| `python3 scripts/build_sdk.py --target riscv64imac-unknown-none-elf` | 종료 코드 0 |
| RISC-V object의 ELF 헤더 검사 | ELF64, ET_REL, RISC-V 확인 |

호스트 실행에서는 펌웨어가 만든 `Buffer`를 플러그인이 받아서 빌려 읽고, 다시 펌웨어에 move했다. 생성·제출 횟수와 반환 결과를 assertion으로 확인했다. 호스트 프로그램은 정적으로 링크되어 있다.

x86_64 ELF object는 Linux 호스트에서 생성했고, RISC-V ELF object도 같은 Linux 호스트에서 cross-compile했다. Object 검증은 생성과 헤더·relocation 정보 확인까지다. 실제 보드 실행, 런타임 ELF 로딩, relocation 적용, SDK 식별자 검증은 수행하지 않았다.

검증을 다시 실행하면 전체 명령과 stdout/stderr가 `artifacts/verification.json` 및 `artifacts/logs/`에 저장된다. 공유 ZIP에는 소스와 문서를 담았으며, 로컬 빌드 산출물은 제외했다.
