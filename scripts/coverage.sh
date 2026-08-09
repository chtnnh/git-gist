#!/usr/bin/env bash
# Run tests with llvm-cov and fail if line coverage is below 95%.
set -euo pipefail
cd "$(dirname "$0")/.."

THRESHOLD="${COVERAGE_THRESHOLD:-95}"

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
  echo "Installing cargo-llvm-cov..."
  cargo install cargo-llvm-cov --locked
fi

rustup component add llvm-tools-preview >/dev/null

# Interactive shells are prompt loops; mutation logic is covered via config_ops /
# auto_enroll / CLI. interactive.rs is the thin dispatch shim (kept in report).
IGNORE='(tests/|/cargo/registry/|/tui/|/wizard/|main\.rs)'

cargo llvm-cov clean --workspace
cargo llvm-cov --workspace --lcov --output-path target/lcov.info \
  --ignore-filename-regex "$IGNORE"

cargo llvm-cov report --summary-only --ignore-filename-regex "$IGNORE" \
  | tee target/coverage-summary.txt

python3 - "$THRESHOLD" <<'PY'
import re, sys
threshold = float(sys.argv[1])
text = open("target/coverage-summary.txt").read()
for line in text.splitlines():
    if not line.startswith("TOTAL"):
        continue
    # Columns: Regions Missed Cover | Functions Missed Executed | Lines Missed Cover | ...
    # Percent fields appear as regions%, functions-executed%, lines%.
    percents = re.findall(r"(\d+\.\d+)%", line)
    if len(percents) < 3:
        print(f"FAIL: expected ≥3 percent fields in TOTAL, got {percents}", file=sys.stderr)
        sys.exit(1)
    line_pct = float(percents[2])  # Lines Cover
    rounded = round(line_pct, 1)
    print(f"Line coverage: {line_pct:.2f}% → {rounded:.1f}% (threshold {threshold:.0f}%)")
    if rounded + 1e-9 < threshold:
        print(f"FAIL: coverage {rounded:.1f}% is below {threshold:.0f}%", file=sys.stderr)
        sys.exit(1)
    print("PASS: coverage threshold met")
    sys.exit(0)
print("FAIL: could not parse TOTAL coverage line", file=sys.stderr)
sys.exit(1)
PY

echo "Coverage report: target/lcov.info"
