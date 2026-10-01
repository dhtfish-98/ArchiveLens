#!/usr/bin/env bash
# Reproducible benchmark: ArchiveLens vs native reference tools on a Mach-O binary.
#
# Measures three axes, each with a fair native competitor:
#   1. Full arm64 disassembly (decode + format all of __text) ... vs llvm-objdump
#   2. Objective-C class dump (metadata recovery) ............... vs rabin2 -c
#   3. Header / load parse (time to first answer) .............. vs rabin2 -I
# Plus decode correctness scored against Capstone (the oracle).
#
# Usage: bench/lens_reference_run.sh <binary> [<binary> ...]
# Requires on PATH: archivelens, lens-bench (cargo build --release), llvm-objdump,
# rabin2 (radare2), python with `capstone`.
set -u

LENS_TOOL="${REIPA:-archivelens}"
LENS_BENCH="${BENCH:-lens-bench}"
LENS_OUT="${OUT:-./bench-out}"
mkdir -p "$LENS_OUT"

lens_timer() { # lens_timer <label> <cmd...> -> prints "label: N.NN s", echoes seconds to stdout var
  local lens_label="$1"; shift
  local lens_s lens_e
  lens_s=$(date +%s.%N); "$@" >/dev/null 2>&1; lens_e=$(date +%s.%N)
  awk -v l="$lens_label" "BEGIN{printf \"  %-28s %8.2f s\n\", l, $lens_e-$lens_s}"
}

for lens_bin in "$@"; do
  lens_name=$(basename "$lens_bin")
  echo "=================================================================="
  echo "BINARY: $lens_name  ($(du -h "$lens_bin" | cut -f1))"
  echo "=================================================================="

  echo "[1] Full arm64 disassembly (all of __text -> text)"
  lens_timer "archivelens full-disasm" "$LENS_BENCH" throughput "$lens_bin" --emit "$LENS_OUT/${lens_name}.archivelens.asm"
  lens_timer "llvm-objdump -d"    llvm-objdump -d --no-show-raw-insn "$lens_bin"

  echo "[2] Objective-C class dump (metadata recovery)"
  lens_timer "archivelens classdump" "$LENS_TOOL" classdump "$lens_bin"
  lens_timer "rabin2 -c"       rabin2 -c "$lens_bin"

  echo "[3] Header / load parse (time to first answer)"
  lens_timer "archivelens info"  "$LENS_TOOL" info "$lens_bin"
  lens_timer "rabin2 -I"   rabin2 -I "$lens_bin"

  echo "[*] Decode correctness vs Capstone oracle"
  "$LENS_BENCH" throughput "$lens_bin" --dump "$LENS_OUT/${lens_name}.archivelens.csv" >/dev/null 2>&1
  python "$(dirname "$0")/lens_capstone_review.py" "$LENS_OUT/${lens_name}.archivelens.csv" \
    | sed 's/^/  /'
  echo
done
