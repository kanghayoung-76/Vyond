#!/bin/bash
# Board is already powered with the SD inserted. Reprogram with the freshly-built
# Vyond-fpga WGRocket8 bitstream (JTAG hard reset -> boot from SD), then drive the
# serial console and run all 9 e-apps. Everything -> eapp-deploy.log (tmux tab 2).
FPGA=/home/jara/Desktop/RISC-V/Vyond-fpga
MAIN=/home/jara/Desktop/RISC-V/Vyond-main
LOG=$FPGA/eapp-deploy.log
BIT="$FPGA/chipyard-1.11.0/fpga/generated-src/chipyard.fpga.vcu118.VCU118FPGATestHarness.WGRocket8VCU118Config/obj/VCU118FPGATestHarness.bit"
TCL="$MAIN/auto-test/program_vcu118.tcl"
SER=/dev/ttyUSB6
PROG_LOG=$FPGA/board_prog2.log

say(){ echo "[$(date +%H:%M:%S)] $*" | tee -a "$LOG"; }

say "===== 새 fpga .bit로 재프로그램 + e-app 실행 ====="
say "bit=$BIT ($(stat -c%s "$BIT" 2>/dev/null) B)"
[ -f "$BIT" ] || { say "!! .bit 없음 — 중단"; exit 1; }

source /tools/Xilinx/Vivado/2019.1/settings64.sh 2>/dev/null
fuser -k "$SER" 2>/dev/null; sleep 1

say "[재프로그램] JTAG로 xcvu9p 프로그램 중..."
timeout 240 vivado -mode batch -source "$TCL" -tclargs "$BIT" > "$PROG_LOG" 2>&1
if grep -q PROGRAM_DONE "$PROG_LOG"; then
  say "[재프로그램 완료] PROGRAM_DONE — 보드 리셋 → SD 부팅"
else
  say "!! 재프로그램 실패. 마지막 로그:"; grep -iE "ERROR|not found" "$PROG_LOG" | tail -5 | tee -a "$LOG"
  exit 2
fi

say "===== 부팅 대기 + e-app 순차 실행 (시리얼 $SER) ====="
python3 -u "$FPGA/auto-test/run_all_eapps.py" --port "$SER" 2>&1 | tee -a "$LOG"
say "===== 완료 ====="
