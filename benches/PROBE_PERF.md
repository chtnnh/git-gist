# Probe performance — before / after

- Root: `/Users/chtnnh/Desktop/tech` (54 repos)
- Before: `gg 1.1.0` (`benches/baseline-pre-1.2.0.json`)
- After: `gg 1.1.0` (`benches/after-probe-opt.json`)
- Method: 5 timed runs + 1 warmup each; wall-clock `time.perf_counter`

| Case | Before (s) | After (s) | Δ (s) | Speedup |
|------|-----------:|----------:|------:|--------:|
| `--only-dirty list` | 0.9447 | 0.1843 | -0.7604 | 5.13× |
| `--refresh list` | 0.0287 | 0.0189 | -0.0098 | 1.52× |
| `-g oss --only-dirty list` | 0.1320 | 0.0346 | -0.0974 | 3.82× |
| `-g oss ov` | 0.1298 | 0.0614 | -0.0684 | 2.11× |
| `list` | 0.0055 | 0.0053 | -0.0002 | 1.04× |
| `ov` | 0.9401 | 0.4236 | -0.5165 | 2.22× |
| `status -sb` | 0.2034 | 0.1744 | -0.0290 | 1.17× |
| `version` | 0.0043 | 0.0048 | +0.0005 | 0.90× |

## What changed

- Full `probe_status`: ~8 git spawns → 3 (`status --porcelain=v2 --branch`, `stash list`, combined `log`) + FS in-progress check
- `--only-*` filters use `ProbeOpts` so dirty/clean/ahead/behind/detached need only the porcelain call; stash-only skips status
- `stale` / `doctor` use partial probes and rayon parallelism
- Discovery avoids a second cache read on hit

