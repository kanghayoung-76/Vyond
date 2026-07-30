#!/bin/bash
# Orchestrate the FPGA e-app run end-to-end, streaming everything to the
# vyond tmux tab-2 monitor (which tails eapp-deploy.log).
#
# Flow:
#   1. Wait for the SD card to be removed from THIS host (/dev/sdb gone) —
#      that's the cue the user has moved it to the FPGA.
#   2. Poll until the board is powered + JTAG-reachable, then reprogram the
#      WGRocket8 bitstream (hard reset -> fresh boot from SD).
#   3. Drive the booted board over serial and run all 9 e-apps once.
set -u
FPGA=/home/jara/Desktop/RISC-V/Vyond-fpga
MAIN=/home/jara/Desktop/RISC-V/Vyond-main
LOG=$FPGA/eapp-deploy.log
BIT="$MAIN/chipyard-1.11.0/fpga/generated-src/chipyard.fpga.vcu118.VCU118FPGATestHarness.WGRocket8VCU118Config/obj/VCU118FPGATestHarness.bit"
TCL="$MAIN/auto-test/program_vcu118.tcl"
SER=/dev/ttyUSB5
PROG_LOG=$FPGA/board_prog.log

say(){ echo "[$(date +%H:%M:%S)] $*" | tee -a "$LOG"; }

say "===== 오케스트레이션 시작: SD 제거 대기 ====="
say "bit=$BIT"
[ -f "$BIT" ] || { say "!! .bit 없음 — 중단"; exit 1; }

# 1) wait for SD removal
if [ -b /dev/sdb ]; then
  say "[대기] SD를 호스트에서 뽑아 FPGA에 꽂고 전원을 켜세요..."
  while [ -b /dev/sdb ]; do sleep 2; done
fi
say "[감지] SD 호스트에서 제거됨."

# 2) program bitstream, retry until board is up + JTAG reachable
source /home/jara/Desktop/RISC-V/miniforge3/etc/profile.d/conda.sh 2>/dev/null
source /tools/Xilinx/Vivado/2019.1/settings64.sh 2>/dev/null
fuser -k "$SER" 2>/dev/null; sleep 1
ok=0
for attempt in $(seq 1 12); do
  say "[재프로그램] 시도 $attempt/12 (보드 전원/JTAG 대기 포함)..."
  timeout 240 vivado -mode batch -source "$TCL" -tclargs "$BIT" > "$PROG_LOG" 2>&1
  if grep -q PROGRAM_DONE "$PROG_LOG"; then ok=1; say "[재프로그램 완료] PROGRAM_DONE"; break; fi
  say "[재프로그램] 아직 안 됨 (보드 미준비?) — 15s 후 재시도. (마지막 로그: $(grep -iE 'ERROR|not found' "$PROG_LOG" | tail -1))"
  sleep 15
done
[ "$ok" = 1 ] || { say "!! 12회 시도 후에도 프로그램 실패. board 전원/JTAG 케이블 확인. 중단."; exit 2; }

# 3) drive serial, run all e-apps
say "===== e-app 순차 실행 시작 (시리얼 $SER) ====="
python3 -u "$FPGA/auto-test/run_all_eapps.py" --port "$SER" 2>&1 | tee -a "$LOG"
say "===== e-app 실행 종료 ====="
