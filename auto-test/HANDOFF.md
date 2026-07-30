# 인수인계 — WorldGuard FPGA enclave 디버깅

**작성 2026-07-30.** 브랜치 `worldguard-fpga` (origin에 푸시 완료). 이 문서 기준 커밋:

| 커밋 | 내용 |
|---|---|
| `85247828d` | gitignore: Vivado 잔여물·보드 로그·SD 스테이징·eapp 빌드 트리 (untracked 124→0) |
| `6b8c9dab2` | SM: `scrub_epm()`을 3인자 `flush_epm_to_memory`에 맞춤 |
| `b4b75b6ed` | build.sh: 커널 Image를 인자로 |
| `b1b8d6503` | tooling: 안전한 시리얼 스크립트, 이 문서, 실험 아카이브 |
| `9083a6fef` | bitstream: SD SPI 저속 클럭, 8/16/32-world VCU118 config |
| `adaa0c819` | examples: host runner `-static` 복구, `enter-exit-test` 추가 |
| `09e020517` | runtime/SDK: 부트 파라미터를 로더·eyrie·SDK 전 구간에 추적 출력 |
| `bc6d63fcd` | **SM: WGC 슬롯 매핑 버그 2건 수정, 고정주소 스태시 제거, 디버그 계측 제거** |

원격에 있던 3개 커밋(`beebs`/`paper_eval`) 위로 rebase했다. 충돌 해소 2건:
`isolator.rs` OS catch-all은 **all-perm @ HW slot 7**(우리 쪽) 채택 — 원격의 `OS+TRUSTED`만 주는 변형은
enclave가 hang한다는 실측 기록이 있고 오늘 측정도 전자 구성이다. `examples/CMakeLists.txt`는 양쪽 다 유지.

---

## 1. 환경 (그대로 재현하면 됨)

| 항목 | 값 |
|---|---|
| 보드 | VCU118, UART=`/dev/ttyUSB6` (115200), JTAG=FT232H |
| 비트스트림 | `chipyard-1.11.0/fpga/generated-src/chipyard.fpga.vcu118.VCU118FPGATestHarness.WGRocket8VCU118Config/obj/VCU118FPGATestHarness.bit` (07-27 15:09 빌드) · nWorlds 8 / nSlots 8 / 50 MHz |
| 커널 | `Vyond-main/build-riscv64/arch/riscv/boot/Image` (Linux 6.9.0) — **`prebuilt/Image` 쓰면 조용히 hang** |
| SD | p1 = raw fw_payload, p2 = ext2 "rootfs" (BusyBox + `/apps/keystone-driver.ko` + `*.ke`) |
| SM fw 해시 | 현재 소스 = **`a994c1438f847ae7`** / 아래 3절 측정에 쓴 fw = `a1c3eb794ed93f07` (**서로 다름**, 5절 참고) |
| UART 로그 | `uart-console.log` (`dd if=/dev/ttyUSB6`가 상시 append, tmux 옆 pane이 `tail -F`) |

### 빌드

```bash
# fw (SM + OpenSBI + kernel payload) — 1~2분
cd tee/monitor && cargo build
cd ../sbi && rm -rf opensbi/build && \
  ./build.sh /home/jara/Desktop/RISC-V/Vyond-main/build-riscv64/arch/riscv/boot/Image
# → tee/sbi/opensbi/build/platform/generic/firmware/fw_payload.bin (~29MB)

# .ke 앱 — 환경변수 4개 필수 (FW_BIN 없으면 attestation 이 configure 를 중단시킴)
cd tee/examples && mkdir -p build_fresh && cd build_fresh
export KEYSTONE_SDK_DIR=/home/jara/Desktop/RISC-V/Vyond-fpga/tee/sdk/install
export KEYSTONE_RUNTIME=/home/jara/Desktop/RISC-V/Vyond-fpga/tee/runtime
export PATH=/home/jara/Desktop/RISC-V/toolchains/bin:$PATH
export FW_BIN=/home/jara/Desktop/RISC-V/Vyond-fpga/tee/sbi/opensbi/build/platform/generic/firmware/fw_payload.bin
cmake .. && make enter-exit-test-package hello-native-package ros-package -j4

# 비트스트림 (~50분, RTL/sd.c 변경 시에만)
source chipyard-1.11.0/env.sh && source /tools/Xilinx/Vivado/2019.1/settings64.sh
make -C chipyard-1.11.0/fpga SUB_PROJECT=wgvcu118 CONFIG=WGRocket8VCU118Config bitstream
```

### 배포 (sudo 불필요 — jara가 plugdev)

```bash
dd if=<fw_payload.bin> of=/dev/sdb1 bs=1M oflag=sync && sync   # 항상 sha256 대조
e2cp -p <app>.ke /dev/sdb2:/<name>.ke                          # -p 로 +x 보존 (없으면 permission denied)
e2ls -l /dev/sdb2:/
source /tools/Xilinx/Vivado/2019.1/settings64.sh
vivado -mode batch -source ../Vyond-main/auto-test/program_vcu118.tcl -tclargs <bit>   # 재프로그램 = 하드리셋
```

### 실행 — 반드시 이 스크립트로

```bash
auto-test/serial/boot_and_run.sh <접두사> [ee회수] [hn회수]     # 부팅 대기 → 마운트/드라이버 → 실행
auto-test/serial/run_apps.sh     <접두사> [앱당회수]            # ee-new/hn-new/ros-new 순차, SKIP_BOOT=1 지원
auto-test/serial/runapp.sh       <ke> <라벨> <횟수> [타임아웃]
```

`printf ... > /dev/ttyUSB6` 로 직접 명령을 쏘지 말 것. 2026-07-30에 보드가 폴트 메시지를 뿜는 동안
전송한 명령에서 문자가 유실돼 `rm -rf /apps/xxx` → **`rm -rf /`** 가 만들어져 타겟 입력 버퍼에 대기했다
(`/mnt/sd` 마운트 상태 = SD의 `.ke` 삭제 직전). JTAG 리셋으로 막았다. 위 스크립트는
**에코를 검증한 뒤에만 개행을 전송**하고, 타임아웃 시 다음 명령을 보내지 않는다.

---

## 2. 확정된 사실

- **WGChecker 권한은 원인이 아니다.** 8슬롯 전면 개방(`perm=0xffff…ff`) ↔ 격리 복원에서 실패율이 같고,
  40여 회 실행 내내 4개 체커 `errcause=0`. 로그의 `ACCESS FAULT ... not in region`은 체커 거부가 아니라
  enclave가 DRAM 밖 저주소를 건드려 생긴 버스 에러다.
- **WID 분리/통일도 원인이 아니다.** `ENCLAVE_AS_OS_WID`(단일 WID)에서도 같은 증상.
- **실패의 큰 부분은 바이너리 신선도였다.** SD의 `.ke`가 07-24 빌드, 소스는 07-28+. 재빌드하니 로더가
  부트 파라미터 7개를 정상 수신하고 로더/eyrie 실패 계열(베이스 0 구조체 읽기, `FATAL: failed to load`,
  eyrie `pc=0x0` 무한 루프, PC 저주소 이탈)이 사라졌다.
- HW 사실: perm 쓰기는 `data | (3 << mwid*2)`라 **WID7 비트가 항상 강제**되고(EPM에 `0x300c`를 써도
  `0xf00c`로 읽힘), 미프로그램 슬롯의 `perm=0xc000`은 리셋값, `cfg`의 A필드가 0인 슬롯은 절대 매치되지 않으며,
  매치가 없으면 기본은 거부(M-mode WID만 허용)라서 catch-all 슬롯이 필수다.

## 3. 최종 측정 (fw `a1c3eb79` + 이 커밋들로 빌드한 `.ke`, 클린 부팅)

> **주의**: 이 측정은 `scrub_epm()`이 **없는** fw(`a1c3eb79`)에서 나왔다. rebase로 원격의
> `scrub_epm`(destroy 시 EPM flush→0-clear→flush)이 들어와 현재 소스는 `a994c143`을 만든다.
> 아래 수치를 현재 소스의 성능으로 인용하지 말 것 — 5절 0번 항목이 재측정이다.

| 앱 | 완주 | 상세 |
|---|---|---|
| `hello-native` | **3/3** (오늘 누적 **8/8**) | `Enclave said: "Hello World"` 정상 출력, OCALL 왕복·destroy 완결 |
| `enter-exit-test` | 1/3 | 실패 2회가 동일 시그니처 (아래 4-1) |
| `ros` | 0/1 | `MAP_SHM`에서 300초 타임아웃 (아래 4-2) |

`hello-native` 8/8은 **"안정성 보장"이 아니다** — 95% 신뢰구간이 63~100%이고, 같은 부팅에서 다른 앱이
호스트 메모리 손상으로 죽으므로 근본 결함은 남아 있다. 수치가 필요하면 20회 이상 연속 측정 권장.

## 4. 남은 문제

### 4-1. 호스트가 NULL 함수 포인터를 호출 (exit 139)

`enter-exit-runner`가 `Keystone::Enclave::run()` 안에서 죽는다. 실패 2회 모두 동일:
`ra=0x1236a`, `epc=0x0`, `cause=0xc`, `unhandled signal 11 at 0x0`.

```asm
1234e: ld a5,-56(s0)   ; this
12352: ld a5,40(a5)    ; this->pDevice
12354: ld a5,0(a5)     ; pDevice->vptr
12356: addi a5,a5,72   ; vtable+72 = resume()
1235a: ld a5,0(a5)     ; a5 = 0   ← 런타임에 0으로 읽힌다
12368: jalr a5         ; → epc=0
```

- 바로 앞 `vtable+64`(`run()`)는 **같은 실행에서 정상**. 8바이트 차이로 하나는 정상, 하나는 0.
- 두 러너 바이너리 모두 **정적 데이터의 +64/+72는 유효 주소**임을 확인했다 → 링크 문제가 아니라 런타임 현상.
- `resume()`은 `run()`의 루프(`EdgeCallHost`/`EnclaveInterrupted`/`EnclaveWaitingForDevice`)에서만 호출된다.
  그래서 **enclave가 인터럽트/OCALL로 빠져나온 회차에서만** 죽는다. 성공한 회차는 `EE4 run returned err=0`이
  찍히며 루프에 진입하지 않았다 — 이것이 회차별 비결정성의 정체다.
- **모순이자 실마리**: `hello-native`는 OCALL을 하므로 같은 루프를 타고 `resume()`를 호출하는데 8/8 완주한다.
  같은 SDK 코드가 러너 바이너리에 따라 갈린다.
- SM 쪽 감사 완료: enclave 영역 밖으로 나가는 스토어가 **하나도 없다**(L2 flush MMIO / CLINT / WGChecker MMIO뿐).
  vtable은 호스트의 읽기 전용 페이지다. 남는 설명은 (a) 스테일 dirty 라인이 나중에 축출되며 그 물리 페이지를
  덮음, (b) WG-aware L2가 호스트 읽기에 잘못된 데이터 반환. 값이 항상 정확히 0이고 회차마다 다른 위치가
  깨지는 점(libgcc 언와인더 첫 명령 SIGILL, `badaddr=0x50`, vtable 슬롯 0)은 (a)와 부합한다.

### 4-2. ros가 `MAP_SHM`에서 교착

```
[TRACE][RT] syscall n=1001 (OCALL) → ocall id=8, shm=0xffffffff80000000/262144
[HOST] loan_shm ...          ← 호스트 응답 시작, 출력이 중간에 끊김
[TRACE][RT] syscall n=1005 (RUNTIME_SYSCALL_MAP_SHM) a0=0x80000030 a1=0x3ffffeb0
                             ← 이후 300초 무응답
```
로더·eyrie 부팅은 통과. 여러 fw 구성에서 **글자 단위로 같은 지점**. `a0=0x80000030`이 SM 영역 주소라
인자로 부자연스럽다. 단일 하트에서 `wait_shm`/`notify_shm`/`wait_and_resume_for_shm` 상태 기계 교착 의심
(SM에 WFI는 없다). syscall 번호는 `tee/sdk/include/shared/eyrie_call.h`.

### 4-3. 미수정 견고성 결함

`tee/sbi/plat/generic/vyond.c` `sv39_translate()`가 경계 검사 없이 M-mode에서 PTE를 역참조한다 →
페이지테이블이 쓰레기면 **폴트 핸들러 안에서 폴트** → `_trap_exit`가 `a0=0`을 역참조 → `sbi_hart_hang()`
→ **보드 완전 정지** (07-29에 1회, `mepc=0x800051f8`). 각 레벨 `pt_pa`가 DRAM 범위인지 확인해 아니면
0(변환 실패)을 반환하도록 고치면 해당 실행만 실패하고 보드는 살아 반복 측정이 가능해진다.

### 4-4. 부수 개선거리

- 드라이버가 **UTM을 0으로 지우지 않는다** (`keystone-page.c:131`; EPM은 `:91`에서 `memset(0)`). 미해결.
- ~~destroy 시 EPM 미소거~~ → **origin 쪽에 `scrub_epm()`이 이미 있었고 rebase로 합쳐졌다**
  (`enclave.rs:460`, destroy의 region 해제 직전 호출). 단 07-22 실험(`REWRITE_SAME_ON_DESTROY`)에서
  "값 무관, cross-WID 접근이 트리거"로 결론났으므로 4-1이 이것으로 해결된다고 기대하지는 말 것 —
  그래도 5절 0번에서 실측으로 확인할 것.

## 5. 다음 작업 순서 (권장)

0. **재측정 먼저** — 현재 소스 fw(`a994c143`, `scrub_epm` 포함)로 `ee-new`/`hn-new`/`ros-new`를 다시 돌려
   3절 수치와 비교한다. `scrub_epm`은 destroy 때 EPM 전체를 M-mode(WID7)로 0-clear 하고 앞뒤로 flush하므로
   4-1의 "스테일 라인이 호스트 페이지를 덮는다" 가설에 직접 영향을 줄 수 있다(좋아질 수도, 나빠질 수도 있다).
   → SD 왕복 1회 + 보드 ~15분

1. **4-1 판별** — 러너에 "크래시 직전 그 vtable 슬롯 재읽기" 코드를 넣어 재읽기에도 0인지 확인.
   0이면 **메모리 내용 손상**, 정상으로 돌아오면 **캐시 반환 오류**. 여기에 "왜 hello-native는 안 걸리나"
   비교(두 러너의 해당 페이지 배치)를 붙이면 원인이 빨리 좁혀진다. → 보드 1회
2. **4-3 수정** — `sv39_translate` 경계 검사. 보드 정지 없이 반복 측정 가능해진다. → fw 1회
3. `hello-native` 20회 연속으로 잔여 실패율 상한 확정. → 보드 시간만
4. 4-4 개선(UTM `memset`, destroy 시 EPM 소거) — 기밀성 관점에서 해야 하는 일.

## 6. 참고 자료

- 발표자료(정리본): https://claude.ai/code/artifact/9dbc051c-b87c-4e68-a7a2-699cf83ff654
- 실험 코드 아카이브: `auto-test/archive/sm_uncommitted_0730_1300.patch` — 정리 전 상태의 실험 코드
  (스태시, 슬롯 전면 개방, WID 통일 플래그, 디버그 마커) 전체. 커밋 `d315e86d6` 이후에는 컨텍스트가
  달라져 `git apply`가 깨끗하게 안 될 수 있으니 **참고용 기록**으로 볼 것.
- 플래그 조합 fw 3종 빌드: `auto-test/archive/build_flag_variants.sh` (해당 플래그가 소스에 있을 때만 동작)
