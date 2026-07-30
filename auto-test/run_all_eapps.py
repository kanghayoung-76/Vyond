#!/usr/bin/env python3
"""Drive an already-booted WGRocket8 VCU118 board over serial to run every
fpga e-app once. No FPGA reprogram here (board must already be up at the
BusyBox '# ' prompt). Output streams to stdout — run piped through
`tee -a eapp-deploy.log` so the vyond tmux tab-2 monitor shows it live.

Prereq: picocom/minicom on the serial port must be CLOSED (frees the port).
jara is in plugdev -> no sudo needed.

On-target model (confirmed from main/auto-test/run_eapps.py):
  - SD partition 2  -> /dev/mmcblk0p2 mounted at /mnt/sd  (holds the .ke files)
  - keystone driver -> /apps/keystone-driver.ko in the initramfs
  - run one app     -> cd /mnt/sd && ./<app>.ke --target /apps/<name>
"""
import sys, time, re, argparse, serial

BAUD = 115200
PROMPT = re.compile(rb"# ")

# (label, on-SD filename, per-run timeout seconds). Single-binary apps.
EAPPS = [
    ("ros",            "ros.ke",            150),  # inter-enclave pub/sub SHM (2 enclaves)
    ("attestor",       "attestor.ke",       120),  # attestation + crypto
    ("shm-ocall-test", "shm-ocall-test.ke", 120),  # SHM + OCALL
    ("wg-slot-test",   "wg-slot-test.ke",   120),  # WGC slot virtualization / lazy load
    ("dev-irq-test",   "dev-irq-test.ke",   120),  # device IRQ
    ("tdds-test",      "tdds-test.ke",      120),  # DDS
    ("dds-tdds-test",  "dds-tdds-test.ke",  120),  # DDS (tdds variant)
]
# split-host-test is two cooperating hosts (bridge + sub); run bridge in the
# background then sub in the foreground. Handled separately at the end.
SPLIT = ("split-host-test", "split-bridge-host.ke", "split-sub-host.ke", 150)


def drain_to_prompt(ser, timeout, nudge_every=6):
    t0 = time.time(); buf = b""; nudge = 0
    ser.reset_input_buffer()
    while time.time() - t0 < timeout:
        c = ser.read(4096)
        if c:
            sys.stdout.buffer.write(c); sys.stdout.flush(); buf += c
            if PROMPT.search(buf[-80:]):
                return True, buf
        now = time.time()
        if now - nudge > nudge_every:
            ser.write(b"\r\n"); ser.flush(); nudge = now
    return False, buf


def send_and_wait(ser, cmd, sentinel, timeout, silence=45):
    """Send cmd, echo everything, return when sentinel regex hit or silence."""
    ser.write(cmd + b"\n"); ser.flush()
    buf = b""; last = time.time(); t0 = time.time()
    sent = re.compile(sentinel)
    while time.time() - t0 < timeout:
        c = ser.read(4096)
        now = time.time()
        if c:
            sys.stdout.buffer.write(c); sys.stdout.flush(); buf += c; last = now
            if sent.search(buf):
                return buf
        elif now - last > silence:
            return buf + b"\n<<SILENCE_TIMEOUT>>\n"
    return buf + b"\n<<HARD_TIMEOUT>>\n"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", default="/dev/ttyUSB6")
    ap.add_argument("--boot-timeout", type=int, default=200)
    ap.add_argument("--only", default="", help="comma-separated labels to run (default: all)")
    args = ap.parse_args()

    only = set(f for f in args.only.split(",") if f)

    try:
        ser = serial.Serial(args.port, BAUD, timeout=2)
    except serial.SerialException as e:
        print(f"!! cannot open {args.port}: {e}\n!! Close picocom/minicom first.")
        sys.exit(2)

    results = {}
    with ser:
        print(f"=== 1) shell prompt 대기 (최대 {args.boot_timeout}s) — board가 부팅돼 있어야 함 ===")
        ok, _ = drain_to_prompt(ser, args.boot_timeout)
        if not ok:
            print("!! no shell prompt. board가 부팅 후 BusyBox '# '에 있는지 확인.")
            sys.exit(3)

        print("\n=== 2) SD p2 mount + keystone 드라이버 insmod + .ke 목록 ===")
        out = send_and_wait(ser,
            b"mkdir -p /mnt/sd; mount /dev/mmcblk0p2 /mnt/sd 2>/dev/null; "
            b"insmod /apps/keystone-driver.ko 2>/dev/null; "
            b"ls -l /dev/keystone; ls /mnt/sd/*.ke; echo READY_$?",
            rb"READY_\d+", timeout=40)
        if b"/dev/keystone" not in out:
            print("!! /dev/keystone 없음 — 드라이버 로드 실패 가능. 계속 시도.")

        for label, fname, tmo in EAPPS:
            if only and label not in only:
                continue
            print(f"\n=== [run] {label}  ({fname}) ===")
            sent = f"DONE_{label}_".encode()
            out = send_and_wait(ser,
                b"rm -rf /apps/%b; cd /mnt/sd && ./%b --target /apps/%b 2>&1; echo DONE_%b_$?"
                % (label.encode(), fname.encode(), label.encode(), label.encode()),
                re.escape(sent) + rb"\d+", timeout=tmo)
            m = re.search(re.escape(sent) + rb"(\d+)", out)
            results[label] = ("exit " + m.group(1).decode()) if m else "TIMEOUT/NO-EXIT"

        # split-host-test: bridge in background, then sub in foreground
        slabel, bridge, sub, stmo = SPLIT
        if not only or slabel in only:
            print(f"\n=== [run] {slabel}  (bridge bg + sub fg) ===")
            out = send_and_wait(ser,
                b"cd /mnt/sd; rm -rf /apps/splitb /apps/splits; "
                b"(./%b --target /apps/splitb >/tmp/bridge.log 2>&1 &) ; sleep 2; "
                b"./%b --target /apps/splits 2>&1; echo DONE_split_$?; "
                b"echo '--- bridge.log ---'; cat /tmp/bridge.log"
                % (bridge.encode(), sub.encode()),
                rb"DONE_split_\d+", timeout=stmo)
            m = re.search(rb"DONE_split_(\d+)", out)
            results[slabel] = ("exit " + m.group(1).decode()) if m else "TIMEOUT/NO-EXIT"

        print("\n=== 요약 ===")
        for k, v in results.items():
            print(f"  {k:18s} : {v}")
        print("=== done ===")


if __name__ == "__main__":
    main()
