#!/bin/bash
# Build the WGRocket8 VCU118 bitstream FROM the Vyond-fpga tree (with our ported
# WGRocket8Config + L2 debug counters + WGC/RTL changes). Full log ->
# bitstream-build.log; milestones -> eapp-deploy.log (tmux tab-2 monitor).
# NOTE: no `set -u` — the conda activate.d/activate-riscv-tools.sh references
# $RISCV before setting it, which trips `set -u`.
FPGA=/home/jara/Desktop/RISC-V/Vyond-fpga
CY=$FPGA/chipyard-1.11.0
BLOG=$FPGA/bitstream-build.log
MLOG=$FPGA/eapp-deploy.log
CONFIG=WGRocket8VCU118Config

mile(){ echo "[$(date +%H:%M:%S)] $*" | tee -a "$MLOG"; }

: > "$BLOG"
mile "===== fpga 트리 비트스트림 빌드 시작: SUB_PROJECT=wgvcu118 CONFIG=$CONFIG ====="
mile "전체 로그: $BLOG"

# environment
source /home/jara/Desktop/RISC-V/miniforge3/etc/profile.d/conda.sh
conda activate "$CY/.conda-env" || { mile "!! conda activate 실패"; exit 1; }
source /tools/Xilinx/Vivado/2019.1/settings64.sh 2>/dev/null
mile "env: CONDA_PREFIX=$CONDA_PREFIX ; vivado=$(which vivado) ; firtool=$(which firtool)"

cd "$CY/fpga" || { mile "!! fpga 디렉토리 없음"; exit 1; }

# build (elaboration -> FIRRTL -> Verilog -> Vivado synth/P&R -> bitstream)
mile "make 시작 (elaboration은 초반, Vivado 합성/P&R은 수 시간)..."
stdbuf -oL -eL make SUB_PROJECT=wgvcu118 CONFIG="$CONFIG" bitstream >>"$BLOG" 2>&1
rc=$?

BIT="$CY/fpga/generated-src/chipyard.fpga.vcu118.VCU118FPGATestHarness.$CONFIG/obj/VCU118FPGATestHarness.bit"
if [ $rc -eq 0 ] && [ -f "$BIT" ]; then
  mile "===== 비트스트림 빌드 성공 rc=0 ====="
  mile "BIT: $BIT ($(stat -c%s "$BIT") B)"
else
  mile "===== 비트스트림 빌드 실패 rc=$rc ====="
  mile "마지막 오류 라인:"; grep -iE "error|failed|exception|not found" "$BLOG" | tail -8 | tee -a "$MLOG"
fi
exit $rc
