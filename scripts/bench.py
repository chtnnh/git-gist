#!/usr/bin/env python3
"""Benchmark gg selection/probe-heavy commands. Writes JSON under benches/."""

from __future__ import annotations

import argparse
import json
import os
import statistics
import subprocess
import time
from pathlib import Path


def run_case(cmd: list[str], warmup: int, runs: int) -> dict:
    for _ in range(warmup):
        subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    times: list[float] = []
    last = None
    for _ in range(runs):
        t0 = time.perf_counter()
        last = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        times.append(time.perf_counter() - t0)
    assert last is not None
    return {
        "cmd": " ".join(cmd),
        "n": runs,
        "mean_s": round(statistics.mean(times), 4),
        "stdev_s": round(statistics.stdev(times) if len(times) > 1 else 0.0, 4),
        "min_s": round(min(times), 4),
        "max_s": round(max(times), 4),
        "last_exit": last.returncode,
    }


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("--label", required=True)
    p.add_argument("--gg", default=str(Path.home() / ".cargo/bin/gg"))
    p.add_argument("--root", default=str(Path.home() / "Desktop/tech"))
    p.add_argument("--runs", type=int, default=5)
    p.add_argument("--warmup", type=int, default=1)
    p.add_argument("--out", type=Path, default=None)
    args = p.parse_args()

    gg = args.gg
    root = args.root
    cases = [
        [gg, "version"],
        [gg, "--root", root, "list"],
        [gg, "--root", root, "--refresh", "list"],
        [gg, "--root", root, "ov"],
        [gg, "--root", root, "--only-dirty", "list"],
        [gg, "--root", root, "-g", "oss", "ov"],
        [gg, "--root", root, "-g", "oss", "--only-dirty", "list"],
        [gg, "--root", root, "status", "-sb"],
        [gg, "--root", root, "stale", "--days", "30"],
        [gg, "--root", root, "doctor"],
    ]

    version = subprocess.check_output([gg, "version"], text=True).strip()
    listed = subprocess.check_output([gg, "--root", root, "list"], text=True)
    repo_count = sum(1 for line in listed.splitlines() if "\t" in line)

    results = []
    for cmd in cases:
        print(f"bench: {' '.join(cmd)}", flush=True)
        results.append(run_case(cmd, args.warmup, args.runs))

    payload = {
        "label": args.label,
        "gg_version": version,
        "gg_bin": gg,
        "root": root,
        "repo_count_list": repo_count,
        "runs_per_case": args.runs,
        "warmup": args.warmup,
        "results": results,
    }

    out = args.out or Path("benches") / f"{args.label}.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(payload, indent=2) + "\n")
    print(f"wrote {out}")


if __name__ == "__main__":
    main()
