#!/bin/bash
# 타겟 시리얼로 앱을 N회 실행하고 각 회차 결과를 요약한다.
# 사용: runapp.sh <ke파일> <라벨> <반복수> [회차당 타임아웃초]
#
# 안전 규칙 (2026-07-29 사고 후 도입 — 흐름제어 없는 115200 UART에서 입력 문자가 유실되어
# `rm -rf /apps/xxx`가 `rm -rf /`로 변형돼 타겟 입력 버퍼에 대기하던 사건):
#   1) 명령은 개행 없이 먼저 보내고, **에코를 검증한 뒤에만** 개행을 보낸다.
#      → 변형된 명령은 절대 실행되지 않는다.
#   2) 보내기 전에 프롬프트 응답(sentinel)으로 타겟이 idle인지 확인한다. 응답이 없으면 중단.
#   3) 문자당 20ms 지연으로 천천히 보낸다(타겟 UART RX FIFO 오버런 방지).
#   4) 한 회차라도 타임아웃/검증 실패면 **다음 회차를 보내지 않고 즉시 중단**한다.
#   5) rm -rf 를 쓰지 않는다. 회차마다 고유 타겟 디렉터리를 쓰므로 삭제가 필요 없다.
set -u
LOG=/home/jara/Desktop/RISC-V/Vyond-fpga/uart-console.log
SER=/dev/ttyUSB6
KE=$1; LABEL=$2; N=${3:-1}; TMO=${4:-300}

seg_since() { tail -c +"$1" "$LOG" | tr -d '\r'; }
# 에코 비교용: 타겟 tty가 80칼럼에서 줄바꿈을 넣기 때문에 개행을 모두 제거하고 비교한다.
seg_flat()  { tail -c +"$1" "$LOG" | tr -d '\r\n'; }

# 개행 없이 한 글자씩 천천히 전송
send_slow() {
  local s="$1" i
  for ((i=0; i<${#s}; i++)); do
    printf '%s' "${s:$i:1}" > "$SER"
    sleep 0.02
  done
}

# 명령 전송: 에코 검증 통과 시에만 개행. 실패하면 ^C로 라인 취소하고 1 반환.
send_verified() {
  local cmd="$1" off
  off=$(stat -c%s "$LOG")
  send_slow "$cmd"
  sleep 1.5
  if seg_flat "$off" | grep -qF -- "$cmd"; then
    printf '\n' > "$SER"
    return 0
  fi
  echo "!! 에코 검증 실패 — 명령이 유실/변형됨. 개행을 보내지 않고 라인을 취소한다."
  echo "   보낸 것 : $cmd"
  echo "   받은 것 : $(seg_since "$off" | tail -2 | tr -d '\n')"
  printf '\003' > "$SER"   # ^C : 편집 중인 라인 취소(실행 안 됨)
  return 1
}

wait_for() {  # wait_for <offset> <정규식> <타임아웃초>
  local off=$1 pat=$2 tmo=$3 i
  for ((i=0; i<tmo/5; i++)); do
    if seg_since "$off" | grep -qE "$pat"; then return 0; fi
    sleep 5
  done
  return 1
}

# 타겟이 프롬프트에서 대기 중인지 확인 (sentinel 왕복)
ready_check() {
  local off
  off=$(stat -c%s "$LOG")
  send_verified 'echo RE"ADY"_OK' || return 1
  wait_for "$off" 'READY_OK' 30 || { echo "!! 타겟 무응답 — 프롬프트 아님(멈춤/실행중)"; return 1; }
  return 0
}

for i in $(seq 1 "$N"); do
  tag="${LABEL}${i}"
  if ! ready_check; then echo "RUN $tag: 전송 전 준비 확인 실패 → 시퀀스 중단"; exit 1; fi

  off=$(stat -c%s "$LOG")
  # rm 없음: 회차별 고유 디렉터리(/apps/$tag)라 삭제 불필요.
  if ! send_verified "cd /mnt/sd && ./$KE --target /apps/$tag; echo XDO\"NE\"_${tag}_\$?"; then
    echo "RUN $tag: 명령 전송 검증 실패 → 시퀀스 중단"; exit 1
  fi

  if ! wait_for "$off" "XDONE_${tag}_[0-9]" "$TMO"; then
    echo "RUN $tag: TIMEOUT (${TMO}s) → 시퀀스 중단 (추가 명령 전송하지 않음)"
    exit 1
  fi

  seg=$(seg_since "$off")
  code=$(printf '%s' "$seg" | grep -aoE "XDONE_${tag}_[0-9]+" | head -1 | sed "s/XDONE_${tag}_//")
  sig=$(printf '%s' "$seg" | grep -aoE "unhandled signal [0-9]+|Segmentation fault|Illegal instruction|Bus error" | head -1)
  fault=$(printf '%s' "$seg" | grep -ac "ACCESS FAULT")
  rcu=$(printf '%s' "$seg" | grep -ac "rcu_sched detected stalls")
  wid=$(printf '%s' "$seg" | grep -aoE "enclave entry wid= 0x0*[0-9a-f]+" | head -1 | grep -oE "0x0*[0-9a-f]+$")
  pfull=$(printf '%s' "$seg" | grep -ac "perm= 0xffffffffffffffff")
  pother=$(printf '%s' "$seg" | grep -a "perm= 0x" | grep -acv "0xffffffffffffffff")
  echo "RUN $tag: exit=$code wid=$wid sig='${sig:-none}' smACCESSFAULT=$fault rcuStall=$rcu permFull=$pfull permOther=$pother"
done
echo "ALL_RUNS_DONE"
