#!/bin/bash
# 부팅 대기 → 마운트/드라이버 → 새 빌드 3개 앱을 각 1회 실행
# 사용: run3new.sh <라벨접두사> [앱당 반복수]
set -u
LOG=/home/jara/Desktop/RISC-V/Vyond-fpga/uart-console.log
SER=/dev/ttyUSB6
RUNAPP="$(dirname "$(readlink -f "$0")")/runapp.sh"
P=${1:-x}; N=${2:-1}

n0=$(grep -ac "Please press Enter" "$LOG")
echo "[boot] waiting (baseline=$n0)..."
for i in $(seq 1 120); do
  [ "$(grep -ac 'Please press Enter' "$LOG")" -gt "$n0" ] && { echo "[boot] prompt after $((i*5))s"; break; }
  sleep 5
done
[ "$(grep -ac 'Please press Enter' "$LOG")" -gt "$n0" ] || { echo "[boot] TIMEOUT"; exit 1; }

sleep 5; printf '\n' > "$SER"; sleep 3
off=$(stat -c%s "$LOG")
# perf_user_access=2 (SYSCTL_LEGACY): Linux riscv PMU 드라이버가 기본값에서 scounteren=0x2 로
# 덮어써 U-mode rdcycle/rdinstret 이 illegal instruction 이 된다(paper_eval 벤치가 전부 SIGILL).
# 2로 두면 드라이버가 scounteren=0x7 을 쓴다. 부팅마다 초기화되므로 여기서 매번 설정한다.
cmd='mkdir -p /mnt/sd; mount /dev/mmcblk0p2 /mnt/sd; insmod /apps/keystone-driver.ko; echo 2 > /proc/sys/kernel/perf_user_access; echo RD"Y"'
for ((i=0; i<${#cmd}; i++)); do printf '%s' "${cmd:$i:1}" > "$SER"; sleep 0.02; done
sleep 1.5
if tail -c +"$off" "$LOG" | tr -d '\r\n' | grep -qF -- "$cmd"; then printf '\n' > "$SER"; else
  echo "[boot] !! 마운트 명령 에코 검증 실패"; printf '\003' > "$SER"; exit 1; fi
ok=0
for i in $(seq 1 24); do
  tail -c +"$off" "$LOG" | tr -d '\r' | grep -q '^RDY' && { echo "[boot] mount+driver ok"; ok=1; break; }
  sleep 5
done
[ "$ok" = 1 ] || { echo "[boot] !! 마운트 실패"; exit 1; }

for app in ee-new hn-new ros-new; do
  echo "=== $app x$N"
  "$RUNAPP" "$app.ke" "${P}${app%%-*}" "$N" 300 || echo "($app 중단 — 다음 앱으로 계속)"
done
echo "ALL_SEQUENCE_DONE"
