# 펌웨어와 Rust 플러그인 사이의 계약

펌웨어가 실행 중 로드하는 ELF의 첫 인자로 function table을 전달한다면, 그 테이블을 Rust 타입으로 정의하는 방식이 자연스럽다. 공유 crate가 타입 계약을 제공하고, function table이 실제 펌웨어 구현의 주소를 제공한다. 타입을 공유하는 방식과 주소를 연결하는 방식은 서로 다른 역할을 한다.

이 프로젝트의 선택은 **공유 `fw-api` crate + Rust 함수 포인터 테이블**이다. 각 API를 펌웨어의 export 심볼로 직접 연결할 수도 있지만, lifetime을 검사하기 위해 그 방식으로 전환할 필요는 없다.

공유 API에는 타입과 함수 포인터 정의만 있다. 메서드·`Drop`·`derive` 구현과 서비스 상태는 두지 않는다. `firmware-services`가 처리 함수와 카운터를 소유하며, application은 그 crate를 의존성으로 선언하지 않는다. 따라서 서비스 구현은 펌웨어에만 링크된다. Application에서 발생하는 작은 Rust 지원 코드나 generic 코드까지 중복되지 않는다는 뜻은 아니다.

`application/`은 독립 Cargo root이며 공유 API의 Git revision을 고정한다. 루트의 `plugin` crate는 호스트 시뮬레이터가 application의 같은 Rust 소스를 정적으로 사용할 수 있도록 하는 adapter다. 루트 시뮬레이터는 로컬 API를, 독립 application은 Git API를 사용한다. 두 경로의 API 소스는 현재 같지만, 별도 컴파일의 산출물까지 같다는 보장은 없다.

## 컴파일 시점의 계약

펌웨어와 플러그인은 동일한 공유 crate의 `Buffer`와 `FirmwareApi`를 사용한다. 양쪽 파일에 같은 모양의 구조체를 각각 선언하는 방식이 아니다.

`bytes: for<'a> fn(&'a Buffer) -> &'a [u8]`에는 입력 borrow와 반환값의 lifetime 관계가 들어 있다. 펌웨어 컴파일러는 테이블에 넣는 함수가 이 타입을 만족하는지 검사한다. 플러그인 컴파일러는 반환값의 사용이 입력 borrow의 범위를 벗어나지 않는지 검사한다. 서로의 전체 코드를 한 번에 분석할 필요가 없다.

컴파일 이후에는 `'a`를 나타내는 runtime 값이 없다. 각 컴파일러가 계약을 검사했기 때문에 실행 중 일반 함수 호출을 할 수 있는 것이다. 구현에 `unsafe`가 있다면 그 구현도 Rust 참조·aliasing·소유권 계약을 지켜야 한다.

Rust 메타데이터는 함수 시그니처뿐 아니라 타입, trait 등 외부 crate를 이해하는 데 필요한 정보를 제공한다. `.rlib`에도 메타데이터가 있으므로 `.rmeta` 파일을 반드시 별도로 배포해야 하는 것은 아니다. 반대로 `.rmeta`만 있으면 코드 생성과 최종 링크가 전부 해결되는 것도 아니다.

## 구현 주소를 연결하는 두 방법

| 방식 | 플러그인이 사용하는 호출 | 필요한 연결 |
| --- | --- | --- |
| Function table | `(api.bytes)(&buffer)` | entrypoint에 테이블 전달 |
| 펌웨어 심볼 직접 참조 | `fw_services::bytes(&buffer)` | 펌웨어 심볼 export와 relocation 해결 |

두 방식 모두 같은 Rust 타입 계약을 사용할 수 있다. 현재처럼 entrypoint의 첫 인자로 테이블을 전달한다면 function table을 유지하는 편이 서비스별 심볼 연결 작업을 줄인다. 일반적인 Rust 함수 호출 자체는 C wrapper를 요구하지 않는다.

Function table은 임의의 type-generic 함수를 그대로 담는 수단은 아니다. `fn<T>(T)`를 하나의 일반 함수 포인터 필드로 넣을 수는 없다. 반면 이 예제의 `for<'a>`는 lifetime에 대한 계약이고, 함수 포인터로 표현할 수 있다. 필요한 타입별 함수, 공유 crate의 generic wrapper, trait object 중 무엇을 쓸지는 API 요구에 따라 별도로 결정한다.

## 실제 로더에 남는 역할

현재 호스트 프로그램은 Rust가 확인한 `plugin::plugin_entry`를 `PluginEntry` 변수에 대입한다. 같은 application 소스를 두 번 호출하고 receipt가 1, 2로 증가하는지 확인한다. 독립 application ELF를 로드하는 실험은 아니다. 실제 ELF 로더는 다음 순서를 구현해야 한다.

1. ELF의 section 또는 segment를 배치하고 대상 아키텍처에 필요한 relocation과 실행 준비를 수행한다.
2. ELF와 펌웨어가 같은 SDK 계약을 사용하는지, Rust entrypoint를 해석하기 전에 확인한다.
3. `plugin_entry`의 주소를 찾고, 그 주소가 `PluginEntry` 타입의 호출 대상이라는 조건을 확립한다.
4. 유효한 `&FirmwareApi`를 만들어 entrypoint를 호출한다.

주소를 함수 포인터로 해석하는 단계는 로더가 책임지는 `unsafe` 경계다. 이후 API 호출을 safe Rust로 작성할 수 있어도, 임의의 ELF가 올바른 Rust 계약을 따르는지는 타입 선언만으로 증명되지 않는다.

`#[unsafe(no_mangle)]`은 entrypoint의 심볼 이름을 고정한다. Rust ABI를 C ABI로 바꾸지는 않는다. ELF의 시작 주소에 무조건 점프하는 것과 `PluginEntry` 규약으로 호출하는 것도 같지 않다. 스택과 인자·반환값 전달은 해당 타깃의 호출 규약을 따라야 한다.

ELF 종류는 타깃과 로더 요구에 맞춰 선택한다. 기본 안내는 x86_64 Linux 호스트와 `x86_64-unknown-linux-gnu` 실험용 타깃을 사용한다. 실제 펌웨어가 bare-metal target을 사용한다면 Linux 호스트에서 그 target으로 cross-compile해야 한다.

이 예제가 생성하는 object는 ELF 타깃에서 ET_REL이다. ET_REL을 직접 로드하려면 object의 relocation을 처리해야 한다. ET_EXEC 또는 ET_DYN으로 링크해서 로드하려면 별도 linker script·링크 옵션과 그 파일 형식을 처리하는 로더가 필요하다. 이 프로젝트는 어느 방식의 로더도 구현하지 않는다.

Function table을 사용해도 플러그인 내부 relocation, `core`의 코드, panic 처리 또는 compiler builtins 의존성이 남을 수 있다. 최종 바이너리에서 어떤 코드를 자체 포함하고 어떤 코드를 펌웨어가 제공할지 정해야 한다. Cargo의 일반 `bin`/`cdylib` 빌드로 출력 형식만 바꾸는 것은 이 설계를 대신하지 않는다.

## SDK와 호환성

Rust ABI와 `.rmeta` 포맷은 안정된 배포 ABI가 아니다. 이 예제는 Rust 1.94.1을 고정한다. 실제 운영에서도 펌웨어 SDK별로 정확한 toolchain과 API artifact를 관리해야 한다.

`build_sdk.py`의 manifest는 compiler 식별 정보, target, 명시한 옵션, API source와 `.rmeta`·`.rlib`의 해시를 기록한다. `sdk_build_id`는 이 빌드 정보를 묶은 식별자다. 현재 스크립트는 그 식별자를 ELF에 삽입하지 않으며, 실행 중 비교하지도 않는다.

실제 로더에는 SDK 식별자를 고정된 바이트 형식의 ELF note나 별도 manifest로 기록하고, entrypoint를 Rust 타입으로 해석하기 전에 비교하는 정책을 적용할 수 있다. SDK ID의 의미는 팀의 배포 정책으로 정한다. 이 예제의 해시가 모든 ABI 조건을 자동으로 검증하는 것은 아니다. custom target JSON, CPU 기능, `core`/`alloc` 및 다른 의존성을 별도로 빌드한다면 그 설정과 artifact도 SDK에 포함해야 한다.

일반 Cargo workspace 실행은 같은 API crate를 두 소비자가 의존하도록 구성한다. SDK 스크립트는 한 번 만든 `.rmeta`를 외부 컴파일에 직접 전달하는 경로를 보여준다. 프로젝트를 분리해 배포할 때는 소비자가 API artifact나 feature 구성을 임의로 바꾸지 않도록 관리해야 한다.

독립 application의 일반 빌드에서는 Cargo가 Git 소스를 가져와 `.rmeta`와 `.rlib`을 만들고 컴파일러에 전달한다. Python 스크립트는 필요하지 않다. `application/rust-toolchain.toml`, profile 설정, `Cargo.lock`은 펌웨어 SDK와 함께 관리한다. Git 의존성의 toolchain이나 profile 설정을 상속하는 것은 아니다.

## 메모리 소유권과 실제 리소스

이 예제의 `Buffer`는 inline 배열이다. 펌웨어가 값을 만들고, 플러그인이 잠시 소유하고 빌려 사용한 뒤, 다시 펌웨어로 move한다. move가 기계어 수준의 복사 제거를 보장하는 것은 아니다. 여기서 확인하는 것은 소유권 계약이다.

실제 `Buffer`가 heap 메모리나 장치 리소스를 소유한다면, 소유권을 받은 쪽이 올바른 해제 동작을 수행할 수 있어야 한다. `Box`나 `Vec`를 경계에서 전달하려면 allocator 계약도 맞아야 한다. 펌웨어가 제공하는 allocator를 공유하거나, 펌웨어 전용 해제를 호출하는 소유 타입을 설계하는 방법이 있다. lifetime 정보만으로 allocator 불일치를 해결할 수는 없다.

마찬가지로 DMA, 인터럽트, 비동기 작업이 참조를 보관한다면 실제 작업 완료 시점과 Rust lifetime 사이를 구현이 올바르게 연결해야 한다. 이 예제는 동기 호출만 수행한다. 플러그인이나 참조 중인 코드·데이터를 언로드할 수 있는 시스템이라면 사용 중 언로드를 막는 수명 관리도 별도로 필요하다.

## 확인 가능한 결과

`cargo test --workspace --locked --offline`은 서비스 상태 검사와 네 가지 compile-fail doctest를 실행한다. 독립 application은 Cargo만으로 빌드하며, `cargo tree`와 object의 `nm` 출력으로 펌웨어 서비스 crate와 그 심볼이 포함되지 않는지 확인할 수 있다. 추가 `python3 scripts/verify.py`는 `.rmeta`만 전달한 별도 컴파일과 정확한 오류 코드를 검사한다.

이 결과는 공유 타입과 함수 포인터 계약이 각 컴파일에서 적용되고, 서비스 코드가 application object에 포함되지 않는다는 근거다. 실제 보드에서 실행하거나 런타임 ELF 로딩을 검증했다는 근거는 아니다.

타깃이 정해진 뒤에는 같은 API·플러그인 코드를 유지하면서, 생성된 object의 relocation과 미해결 심볼을 기준으로 로더 및 최종 링크를 구현할 수 있다.
