#!/bin/bash
# 부팅 프롬프트 대기 → 마운트/드라이버 → enter-exit + hello-native 실행
# 사용: boot_and_run.sh <라벨접두사> [enter-exit회수] [hello-native회수]
set -u
LOG=/home/jara/Desktop/RISC-V/Vyond-fpga/uart-console.log
SER=/dev/ttyUSB6
RUNAPP="$(dirname "$(readlink -f "$0")")/runapp.sh"
PREFIX=${1:-x}; NEE=${2:-3}; NHN=${3:-9}

# SKIP_BOOT=1 이면 이미 부팅/프롬프트 상태로 보고 부팅 대기를 건너뛴다.
if [ "${SKIP_BOOT:-0}" = "1" ]; then
  echo "[boot] SKIP_BOOT=1 — 부팅 대기 생략"
else
n0=$(grep -ac "Please press Enter" "$LOG")
echo "[boot] baseline prompt count=$n0, waiting..."
for i in $(seq 1 120); do
  n=$(grep -ac "Please press Enter" "$LOG")
  if [ "$n" -gt "$n0" ]; then echo "[boot] prompt reached after $((i*5))s"; break; fi
  sleep 5
done
if [ "$(grep -ac 'Please press Enter' "$LOG")" -le "$n0" ]; then
  echo "[boot] TIMEOUT — 부팅 실패"; exit 1
fi
fi

sleep 5
printf '\n' > "$SER"; sleep 3

# 마운트/드라이버도 에코 검증 방식으로 (runapp.sh와 동일한 안전 규칙)
off=$(stat -c%s "$LOG")
cmd='mkdir -p /mnt/sd; mount /dev/mmcblk0p2 /mnt/sd; insmod /apps/keystone-driver.ko; echo RD"Y"'
for ((i=0; i<${#cmd}; i++)); do printf '%s' "${cmd:$i:1}" > "$SER"; sleep 0.02; done
sleep 1.5
if tail -c +"$off" "$LOG" | tr -d '\r\n' | grep -qF -- "$cmd"; then
  printf '\n' > "$SER"
else
  echo "[boot] !! 마운트 명령 에코 검증 실패 → 중단"; printf '\003' > "$SER"; exit 1
fi
ok=0
for i in $(seq 1 24); do
  if tail -c +"$off" "$LOG" | tr -d '\r' | grep -q '^RDY'; then echo "[boot] mount+driver ok"; ok=1; break; fi
  sleep 5
done
[ "$ok" = 1 ] || { echo "[boot] !! 마운트/드라이버 실패 → 중단"; exit 1; }

echo "=== enter-exit-test x$NEE ==="
"$RUNAPP" enter-exit-test.ke "${PREFIX}ee" "$NEE" 300 || echo "(enter-exit 시퀀스 중단됨)"
echo "=== hello-native x$NHN ==="
"$RUNAPP" hello-native-fpga.ke "${PREFIX}hn" "$NHN" 300 || echo "(hello-native 시퀀스 중단됨)"
echo "ALL_SEQUENCE_DONE"
