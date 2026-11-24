# Rust 펌웨어와 런타임 플러그인의 타입 공유 예제

펌웨어가 실행 중 로드한 Rust 플러그인에 function table을 전달할 때, **Rust 함수 포인터 타입으로 borrow, lifetime, 소유권 이전 계약을 표현할 수 있다.** 양쪽이 같은 공유 crate를 사용하면 각 컴파일러가 그 계약을 검사한다. API마다 C wrapper를 만들 필요는 없다.

이 프로젝트는 `no_std` API와 플러그인, 호스트에서 실행하는 펌웨어 시뮬레이터로 그 동작을 재현한다. `.rmeta`만 읽어 플러그인을 별도로 컴파일하고, 잘못된 lifetime과 move를 컴파일러가 거부하는지도 확인한다.

**실제 ELF 로더는 포함하지 않는다.** 호스트 시뮬레이터는 별도 crate들을 정적으로 링크한다. `.rmeta`를 사용하는 별도 빌드는 object 파일까지 생성하며, 런타임 relocation과 entrypoint 주소 변환은 수행하지 않는다.

## 실행하기

이 문서는 **x86_64 Linux 개발 환경**을 기준으로 한다. 기본 실험 타깃은 `x86_64-unknown-linux-gnu`다. Rustup, 호스트용 C toolchain과 링커(`cc`), Python 3.9 이상, object 확인용 binutils(`readelf`, `nm`)가 필요하다.

`rust-toolchain.toml`이 Rust 1.94.1을 지정하며, 해당 toolchain이 없으면 Rustup이 처음 실행할 때 내려받는다. 외부 Cargo 의존성은 없다.

```bash
cargo run --locked --offline -p firmware-sim
```

정상 출력:

```text
PASS: firmware -> plugin -> firmware ownership transfer
PASS: borrowed slice returned through a Rust function pointer
PluginReport { length: 16, first_byte: 7, receipt: 1 }
Scope: host simulation; the plugin is statically linked.
```

전체 검증:

```bash
python3 scripts/verify.py
```

검증 스크립트는 형식 검사, workspace 빌드, 호스트 실행, SDK 생성, `.rmeta`만 사용한 플러그인 컴파일, 네 가지 컴파일 실패 검사를 수행한다. 실패 검사는 종료 코드뿐 아니라 예상한 Rust 진단도 확인한다. 컴파일러를 찾지 못하는 등의 다른 실패를 성공으로 처리하지 않는다.

전체 명령·종료 코드·로그 경로는 `artifacts/verification.json`에, 실행별 stdout/stderr는 `artifacts/logs/`에 저장된다.

Linux에서 확인한 호스트 실행과 ELF object 생성 결과는 [검증 기록](VALIDATION.md)에 정리했다.

## 프로젝트 구성

| 경로 | 역할 |
| --- | --- |
| [crates/fw-api](crates/fw-api/src/lib.rs) | `Buffer`, `FirmwareApi`, `PluginEntry` 등 양쪽이 공유하는 계약 |
| [crates/plugin](crates/plugin/src/lib.rs) | 전달받은 테이블로 펌웨어 함수를 호출하는 `no_std` 코드 |
| [crates/firmware-sim](crates/firmware-sim/src/main.rs) | 서비스 구현과 상태를 소유하는 호스트 시뮬레이터 |
| [scripts/build_sdk.py](scripts/build_sdk.py) | `.rmeta`, `.rlib`, 플러그인 object, 빌드 manifest 생성 |
| [tests/compile-fail](tests/compile-fail) | lifetime·move·제공자 함수 타입 위반 예제 |
| [DESIGN.md](DESIGN.md) | 컴파일과 런타임의 역할, 실제 펌웨어에 적용할 때의 설계 범위 |

## function table에 lifetime을 표현하기

공유 crate의 핵심 정의는 다음과 같다.

```rust
pub struct FirmwareApi {
    pub new_buffer: fn(u8) -> Buffer,
    pub bytes: for<'a> fn(&'a Buffer) -> &'a [u8],
    pub submit: fn(Buffer) -> usize,
}

pub type PluginEntry = for<'fw> fn(&'fw FirmwareApi) -> PluginReport;
```

`for<'a> fn(&'a Buffer) -> &'a [u8]`는 호출자가 선택한 borrow의 lifetime에 맞춰 동작하는 함수 포인터다. 반환된 slice를 사용할 수 있는 기간은 입력 `Buffer`의 borrow에 묶인다. 함수 포인터를 통해 호출해도 이 관계를 컴파일러가 검사한다.

실행 시 전달하는 테이블에는 함수 포인터가 들어간다. lifetime을 나타내는 값이나 실행 중 borrow 검사기는 들어가지 않는다. 펌웨어 쪽 컴파일러는 구현을 테이블에 넣을 때 타입을 검사하고, 플러그인 쪽 컴파일러는 그 타입에 맞춰 호출과 반환값 사용을 검사한다.

```text
컴파일
  fw-api crate ── 타입·시그니처·lifetime 계약 ──┬─ 펌웨어 컴파일러
                                             └─ 플러그인 컴파일러

실행
  펌웨어 ── &FirmwareApi ──> plugin_entry
           함수 포인터 <─── 플러그인이 호출
```

플러그인은 다음 순서로 객체를 사용한다.

```rust
let buffer = (api.new_buffer)(7); // 펌웨어 → 플러그인: 소유권 이전
let data = (api.bytes)(&buffer);  // buffer를 빌린 slice
let length = data.len();
let receipt = (api.submit)(buffer); // 플러그인 → 펌웨어: 소유권 이전
```

`Buffer`는 `Copy`가 아니다. 따라서 `submit` 이후 `buffer`를 다시 사용할 수 없다. `submit` 뒤에서 `data`를 사용하려고 해도, 살아 있는 borrow와 move가 충돌하므로 컴파일 오류다.

| 잘못된 코드 | 거부되는 이유 |
| --- | --- |
| 반환된 slice를 `'static`으로 반환 | 입력보다 긴 lifetime을 만들 수 없음 |
| `submit(buffer)` 뒤에서 `buffer` 사용 | 이미 소유권을 이전함; E0382 |
| `submit(buffer)` 뒤에서 기존 `data` 사용 | borrow가 살아 있는 동안 move; E0505 |
| `'static` 입력만 받는 함수를 `bytes`에 대입 | 임의의 `'a`를 받는 계약을 만족하지 않음; E0308 |

## `.rmeta`를 만들고 공유하기

x86_64 Linux용 SDK와 object를 생성하려면:

```bash
python3 scripts/build_sdk.py --target x86_64-unknown-linux-gnu
```

결과는 `artifacts/sdk/x86_64-unknown-linux-gnu/`에 저장된다. 다른 타깃을 지정하면 해당 target 이름의 디렉터리를 사용한다.

| 산출물 | 용도 |
| --- | --- |
| `libfw_api.rmeta` | 다른 Rust crate를 컴파일할 때 읽는 메타데이터 |
| `libfw_api.rlib` | 메타데이터와 라이브러리 코드를 포함하는 Rust 라이브러리 |
| `plugin.o` | `.rmeta`로 타입을 검사한 뒤 생성한 플러그인 object |
| `manifest.json` | toolchain, target, 옵션, 산출물 해시를 기록한 빌드 정보 |

메타데이터를 만드는 핵심 옵션은 `--emit=metadata`다. 다음은 같은 x86_64 Linux 타깃을 명시한 최소 명령이다.

```bash
mkdir -p artifacts/manual

rustc crates/fw-api/src/lib.rs \
  --edition=2024 --crate-name=fw_api --crate-type=rlib \
  --target=x86_64-unknown-linux-gnu \
  -C panic=abort \
  --emit=metadata=artifacts/manual/libfw_api.rmeta,link=artifacts/manual/libfw_api.rlib

rustc crates/plugin/src/lib.rs \
  --edition=2024 --crate-name=plugin --crate-type=rlib \
  --target=x86_64-unknown-linux-gnu \
  -C panic=abort -C codegen-units=1 \
  --extern fw_api=artifacts/manual/libfw_api.rmeta \
  -L dependency=artifacts/manual \
  --emit=obj=artifacts/manual/plugin.o
```

두 번째 명령의 `--extern`은 `.rlib`이 아닌 `.rmeta`를 직접 지정한다. `rustc`는 펌웨어 함수의 구현을 다시 정의하지 않고도 공유 타입과 계약을 알아낸다. `.rmeta` 자체에는 최종 링크에 필요한 기계어가 없으므로, object 생성 이후의 링크·패키징은 별도 단계다.

일반 Rust 빌드에는 `.rlib`을 공유해도 된다. `.rlib` 안에도 메타데이터가 있다. **공유 API crate에는 타입과 계약을 두고, 펌웨어 서비스 구현과 상태는 펌웨어에 둔다.** 이 구조에서는 양쪽에 API crate를 링크해도 펌웨어 서비스 상태를 복제하지 않는다.

SDK는 코드 생성에 필요한 정보를 포함하도록 실제 빌드에서 생성한다. `cargo check`가 생성한 메타데이터를 범용 SDK 산출물로 복사하지 않는다. generic과 inline 함수에 필요한 MIR이 생략될 수 있다.

## 펌웨어 타깃 object 생성

x86_64 Linux에서 먼저 기본 실험용 ELF object를 만들고 헤더와 relocation을 확인한다.

```bash
python3 scripts/build_sdk.py --target x86_64-unknown-linux-gnu
readelf -h artifacts/sdk/x86_64-unknown-linux-gnu/plugin.o
readelf -r artifacts/sdk/x86_64-unknown-linux-gnu/plugin.o
nm -u artifacts/sdk/x86_64-unknown-linux-gnu/plugin.o
```

헤더의 `Class`는 `ELF64`, `Type`은 `REL (Relocatable file)`, `Machine`은 `Advanced Micro Devices X86-64`다. `plugin.o`는 최종 실행 파일이 아니라 relocation이 남아 있는 ET_REL object다. `nm -u`는 최종 링크 또는 로더에서 해결해야 할 외부 심볼을 보여준다.

**빌드를 실행하는 Linux 호스트와 실제 펌웨어 타깃은 별개다.** `x86_64-unknown-linux-gnu`는 이 예제의 Linux 실험용 타깃이며, bare-metal 펌웨어에 그대로 적용할 타깃을 의미하지 않는다. 실제 펌웨어가 다른 Rust target이나 custom target을 사용한다면 API와 플러그인도 그 target과 설정에 맞춰 빌드해야 한다.

예를 들어 Linux 호스트에서 RISC-V 펌웨어용 object를 cross-compile하려면:

```bash
rustup target add --toolchain 1.94.1 riscv64imac-unknown-none-elf
python3 scripts/build_sdk.py --target riscv64imac-unknown-none-elf
```

이 결과는 `artifacts/sdk/riscv64imac-unknown-none-elf/plugin.o`에 생성된다. 실제 보드에서 실행하려면 펌웨어에 맞는 최종 링크와 로더가 추가로 필요하다.

`--relocation-model static` 또는 `--relocation-model pic`도 선택할 수 있다. 어떤 모델이 적합한지는 타깃과 로더가 처리할 relocation에 따라 결정한다. function table은 펌웨어 서비스 주소를 전달할 뿐, ELF 내부 relocation이나 `core` 등에서 생기는 추가 심볼 의존성을 자동으로 해결하지 않는다.

## 적용 조건과 확인한 범위

이 방식은 펌웨어와 플러그인이 같은 주소 공간에서 동작하고, 참조하는 메모리와 코드가 사용 중에 유효하다는 전제다. 양쪽은 같은 Rust toolchain, target, 공유 API artifact 및 호환되는 빌드 설정을 사용해야 한다. Rust ABI는 버전 간 안정성을 보장하지 않는다.

공유 타입과 lifetime은 올바른 Rust 코드를 컴파일하는 데 도움을 준다. ELF의 entrypoint 주소가 실제로 `PluginEntry` 타입의 함수인지, 로더가 relocation을 올바르게 적용했는지는 `.rmeta`가 검사하지 않는다. 실제 로더는 이 조건을 충족한 뒤 typed entrypoint를 호출해야 한다.

예제는 heap allocator, DMA, 인터럽트, 모듈 언로드를 다루지 않는다. `Buffer`가 inline 배열을 가진 이유도 allocator 계약을 별도의 문제로 남기기 위해서다. 실제 펌웨어 적용 시의 선택과 책임은 [설계 설명](DESIGN.md)에 정리했다.

## 근거

- [Rust 함수 포인터](https://doc.rust-lang.org/reference/types/function-pointer.html): 함수 포인터에도 lifetime 계약을 표현할 수 있다.
- [Higher-ranked lifetime bounds](https://doc.rust-lang.org/reference/trait-bounds.html#higher-ranked-trait-bounds): `for<'a>`의 의미.
- [Rust 라이브러리와 메타데이터](https://rustc-dev-guide.rust-lang.org/backend/libs-and-metadata.html): `.rmeta`, `.rlib`, compiler 버전, MIR.
- [rustc `--emit`와 `--extern`](https://doc.rust-lang.org/rustc/command-line-arguments.html): 생성 및 소비 명령.
- [Rust ABI](https://doc.rust-lang.org/reference/items/external-blocks.html#abi): Rust ABI의 안정성 범위.
- [Linux의 Rust 라이브러리 빌드](https://github.com/torvalds/linux/blob/master/rust/Makefile): 코드와 메타데이터를 생성하는 실제 사용 사례. 이 예제는 Linux 커널에 의존하지 않는다.
