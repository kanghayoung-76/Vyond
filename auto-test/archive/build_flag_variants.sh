#!/bin/bash
# 요구사항 1(진입/이탈 캐시 플러시 제거) × 요구사항 2(고정주소 스태시 제거) 3조합 fw 빌드
#   T1: flush 제거      + 스태시 유지   (FLUSH=false, STASH=true)
#   T2: flush 유지      + 스태시 제거   (FLUSH=true,  STASH=false)
#   T3: 둘 다 적용      (FLUSH=false, STASH=false)
set -e
ROOT=/home/jara/Desktop/RISC-V/Vyond-fpga
EN=$ROOT/tee/monitor/src/enclave.rs
OUT=/home/jara/.claude/jobs/67140f88/tmp/fw
IMG=/home/jara/Desktop/RISC-V/Vyond-main/build-riscv64/arch/riscv/boot/Image
mkdir -p "$OUT"

set_flag() {  # set_flag <const이름> <true|false>
  sed -i "s/^const $1: bool = \(true\|false\);/const $1: bool = $2;/" "$EN"
  grep -q "^const $1: bool = $2;" "$EN" || { echo "!! $1 설정 실패"; exit 1; }
}

build() {  # build <태그> <FLUSH> <STASH>
  local tag=$1 flush=$2 stash=$3
  set_flag ENTRY_EXIT_CACHE_FLUSH "$flush"
  set_flag BOOT_PARAM_STASH "$stash"
  echo "=== [$tag] ENTRY_EXIT_CACHE_FLUSH=$flush  BOOT_PARAM_STASH=$stash"
  (cd "$ROOT/tee/monitor" && touch src/enclave.rs && cargo build 2>&1 | tail -1)
  (cd "$ROOT/tee/sbi" && rm -rf opensbi/build && ./build.sh "$IMG" > /dev/null 2>&1)
  cp "$ROOT/tee/sbi/opensbi/build/platform/generic/firmware/fw_payload.bin" "$OUT/fw_$tag.bin"
  echo "    -> $OUT/fw_$tag.bin  sha=$(sha256sum "$OUT/fw_$tag.bin" | cut -c1-16)"
}

build T1 false true
build T2 true  false
build T3 false false

# 소스는 T3 상태로 남긴다(둘 다 적용). 필요 시 스크립트로 재설정 가능.
echo "=== 완료"; ls -l --time-style=+%H:%M:%S "$OUT"
