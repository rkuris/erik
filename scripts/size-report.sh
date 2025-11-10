#!/bin/bash
# Size report: compares unstripped vs stripped ELF and outputs section & total deltas.
# Usage: scripts/size-report.sh <ELF_PATH>
set -e
ELF="$1"
if [ -z "$ELF" ] || [ ! -f "$ELF" ]; then
  echo "Usage: $0 <ELF_PATH>" >&2
  exit 1
fi
TMP_DIR=$(mktemp -d)
STRIPPED="$TMP_DIR/stripped.elf"
cp "$ELF" "$STRIPPED"
# Strip only symbols & debug info to simulate distribution artifact
riscv32-esp-elf-strip -g "$STRIPPED" || true
# Collect size info
unstripped_size=$(stat -c %s "$ELF")
stripped_size=$(stat -c %s "$STRIPPED")
printf "Unstripped ELF: %d bytes\n" "$unstripped_size"
printf "Stripped   ELF: %d bytes\n" "$stripped_size"
python - <<'PY'
import subprocess, json, os
elf=os.environ['ELF']
stripped=os.environ['STRIPPED']
cmd=['riscv32-esp-elf-size','-A',elf]
raw=subprocess.check_output(cmd, text=True)
scmd=['riscv32-esp-elf-size','-A',stripped]
raw_s=subprocess.check_output(scmd, text=True)
def parse(text):
    out={}
    for line in text.splitlines():
        parts=line.split()
        if len(parts)>=4 and parts[0].startswith('.'):
            out[parts[0]]=int(parts[1])
    return out
u=parse(raw)
s=parse(raw_s)
sections=sorted(set(u)|set(s))
print('\nSection Deltas (bytes):')
print(f"{'Section':25} {'Unstripped':>12} {'Stripped':>12} {'Delta':>8}")
for sec in sections:
    print(f"{sec:25} {u.get(sec,0):12} {s.get(sec,0):12} {s.get(sec,0)-u.get(sec,0):8}")
PY
rm -rf "$TMP_DIR"
