# 별도 repository용 Rust application

이 디렉터리는 독립된 Cargo 프로젝트다. 공유 `fw-api`만 Git 의존성으로 가져오고, 펌웨어의 처리 함수나 상태를 포함하는 `firmware-services`에는 의존하지 않는다. 이 디렉터리 전체를 다른 repository로 옮겨도 상위 디렉터리를 참조하지 않고 빌드할 수 있다.

공유 API의 정의와 함수 포인터 계약은 Cargo가 컴파일에 연결한다. 실제 `new_buffer`, `bytes`, `submit` 구현 주소는 펌웨어가 entrypoint의 첫 인자인 `&FirmwareApi`로 전달한다.

## Cargo로 빌드

x86_64 Linux와 Rustup을 기준으로, 이 디렉터리에서 실행한다.

```bash
cargo build --locked --target x86_64-unknown-linux-gnu
cargo elf-object
```

첫 실행은 지정된 Rust toolchain과 Git 의존성을 내려받을 수 있다. Python이나 `build.rs`는 필요하지 않다. `cargo elf-object`는 이 프로젝트의 `.cargo/config.toml`에 정의한 alias로, 다음 Cargo 명령을 실행한다.

```bash
cargo rustc --lib --locked --target x86_64-unknown-linux-gnu -- --emit=obj
```

생성되는 `target/x86_64-unknown-linux-gnu/debug/deps/firmware_app-*.o`는 ELF64 ET_REL object다. 완성된 실행 ELF나 ELF 로더를 제공하는 것은 아니다.

## 처리 코드가 application에 포함되는지 확인

```bash
cargo tree --locked
nm -C --defined-only target/x86_64-unknown-linux-gnu/debug/deps/firmware_app-*.o
```

의존성 트리에는 `fw-api`만 나타나고 `firmware-services`는 나타나지 않는다. Object에는 `plugin_entry`와 필요한 작은 Rust 지원 코드가 들어갈 수 있지만, `firmware_services::submit_impl`이나 펌웨어 카운터 정의는 없다. 공유 API 소스를 빌드한다는 사실이 펌웨어 서비스 로직을 application에 복제하는 것은 아니다.

## API 버전 관리

`Cargo.toml`의 `rev`와 `Cargo.lock`으로 API 소스를 고정하고, 이 프로젝트의 `rust-toolchain.toml` 및 profile 설정을 펌웨어 SDK와 맞춘다. 의존하는 repository의 toolchain과 profile 설정은 자동으로 적용되지 않는다.

이 방식은 펌웨어에서 이미 생성한 `.rmeta`를 다운로드하는 방식이 아니다. Cargo가 고정된 API 소스를 application 빌드에서 다시 컴파일한다. 별도로 빌드된 펌웨어와 ELF의 ABI 호환성은 SDK 정책으로 관리해야 한다.

전체 설계와 펌웨어 시뮬레이터는 [예제 repository](https://github.com/yw-furiosa/rust-firmware-interface-demo)를 참고한다. 시뮬레이터는 이 application의 같은 소스를 정적으로 링크하여 동작을 확인하며, 별도로 빌드한 ELF를 로드하지는 않는다.
