#!/usr/bin/env python3
"""Build dummy workspace, run gg, render terminal-style PNG screenshots for the docs.

Does not use the developer's real repositories.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "docs" / "src" / "images"
GG = ROOT / "target" / "release" / "gg"


def run(cmd: list[str], env: dict, cwd: Path) -> str:
    p = subprocess.run(
        cmd,
        cwd=cwd,
        env=env,
        text=True,
        capture_output=True,
        check=False,
    )
    # Prefer stdout; keep stderr lines that are useful (selection summary / warnings)
    parts = []
    if p.stderr.strip():
        parts.append(p.stderr.rstrip())
    if p.stdout.strip():
        parts.append(p.stdout.rstrip())
    if p.returncode != 0 and not parts:
        parts.append(f"(exit {p.returncode})")
    return "\n".join(parts) + "\n"


def git(cwd: Path, *args: str) -> None:
    env = os.environ.copy()
    env.update(
        {
            "GIT_AUTHOR_NAME": "Demo",
            "GIT_AUTHOR_EMAIL": "demo@example.com",
            "GIT_COMMITTER_NAME": "Demo",
            "GIT_COMMITTER_EMAIL": "demo@example.com",
        }
    )
    subprocess.run(
        ["git", *args],
        cwd=cwd,
        env=env,
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def init_repo(path: Path) -> None:
    path.mkdir(parents=True, exist_ok=True)
    git(path, "init", "-b", "main")
    (path / "README.md").write_text(f"# {path.name}\n", encoding="utf-8")
    git(path, "add", "README.md")
    git(path, "commit", "-m", f"init {path.name}")


def render_png(text: str, dest: Path, title: str) -> None:
    try:
        from PIL import Image, ImageDraw, ImageFont
    except ImportError as e:
        raise SystemExit(
            "Pillow is required: python3 -m pip install pillow"
        ) from e

    lines = text.rstrip("\n").split("\n") or [""]
    # Prefer a monospace font; fall back to default.
    font = None
    for candidate in (
        "/System/Library/Fonts/Menlo.ttc",
        "/System/Library/Fonts/Monaco.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
    ):
        if Path(candidate).exists():
            try:
                font = ImageFont.truetype(candidate, 15)
                break
            except OSError:
                continue
    if font is None:
        font = ImageFont.load_default()

    pad_x, pad_y = 18, 16
    line_h = 20
    title_h = 28
    width = max(720, max(len(l) for l in lines) * 9 + pad_x * 2)
    height = title_h + pad_y * 2 + line_h * len(lines) + 8

    img = Image.new("RGB", (width, height), "#0f1419")
    draw = ImageDraw.Draw(img)
    draw.rectangle((0, 0, width, title_h), fill="#1a2332")
    draw.text((pad_x, 6), title, fill="#8b9cb3", font=font)

    y = title_h + pad_y
    for line in lines:
        # Soft syntax coloring for demo readability
        color = "#e6edf3"
        stripped = line.lstrip()
        if stripped.startswith("selection:") or stripped.startswith("git-gist:"):
            color = "#7ee787"
        elif stripped.startswith("[warn]") or "stale" in line:
            color = "#ffa657"
        elif stripped.startswith("[info]") or stripped.startswith("added"):
            color = "#79c0ff"
        elif "\t" in line and not stripped.startswith("┌"):
            color = "#c9d1d9"
        draw.text((pad_x, y), line, fill=color, font=font)
        y += line_h

    dest.parent.mkdir(parents=True, exist_ok=True)
    img.save(dest)
    print(f"wrote {dest.relative_to(ROOT)}")


def main() -> None:
    if not GG.exists():
        print("Building release gg…")
        subprocess.run(
            ["cargo", "build", "--release", "-q"],
            cwd=ROOT,
            check=True,
        )

    OUT.mkdir(parents=True, exist_ok=True)
    tmp = Path(tempfile.mkdtemp(prefix="gg-docs-"))
    home = tmp / "home"
    work = tmp / "workspace"
    home.mkdir()
    work.mkdir()

    # Dummy polyrepo layout
    for rel in (
        "oss/payments-api",
        "oss/payments-web",
        "oss/shared-lib",
        "learning/rustlings",
        "learning/tokio-lab",
        "work/client-portal",
    ):
        init_repo(work / rel)

    # Dirty one repo for overview variety
    (work / "oss/payments-api" / "TODO").write_text("ship it\n", encoding="utf-8")

    cfg_dir = home / ".git-gist"
    cfg_dir.mkdir()
    cfg = f'''schema_version = 1
root = "{work}"
depth = 6
theme = "vivid"
show_path = true

[aliases]
payments-api = "{work}/oss/payments-api"
payments-web = "{work}/oss/payments-web"
shared-lib = "{work}/oss/shared-lib"
rustlings = "{work}/learning/rustlings"
tokio-lab = "{work}/learning/tokio-lab"
client-portal = "{work}/work/client-portal"

[groups]
oss = ["payments-api", "payments-web", "shared-lib"]
learning = ["rustlings", "tokio-lab"]

[tags]
backend = ["payments-api", "shared-lib"]

[[auto_enroll]]
path = "{work}"
path_prefix = "oss/"
depth = 6
groups = ["oss"]
'''
    (cfg_dir / "config.toml").write_text(cfg, encoding="utf-8")

    env = os.environ.copy()
    env.update(
        {
            "HOME": str(home),
            "NO_COLOR": "0",
            "TERM": "xterm-256color",
            "PATH": f"{GG.parent}:{env.get('PATH', '')}",
        }
    )
    # Force color off for stable screenshots (we colorize in the renderer)
    env["NO_COLOR"] = "1"

    shots = [
        (
            "overview-oss.png",
            "gg -g oss ov",
            [str(GG), "-g", "oss", "ov"],
        ),
        (
            "list-learning.png",
            "gg -g learning list",
            [str(GG), "-g", "learning", "list"],
        ),
        (
            "doctor-config.png",
            "gg doctor --config",
            [str(GG), "doctor", "--config"],
        ),
        (
            "alias-list.png",
            "gg alias list",
            [str(GG), "alias", "list"],
        ),
        (
            "update-dry-run.png",
            "gg update --dry-run",
            [str(GG), "update", "--dry-run"],
        ),
        (
            "config-enroll-list.png",
            "gg config enroll list",
            [str(GG), "config", "enroll", "list"],
        ),
        (
            "help-config.png",
            "gg config --help",
            [str(GG), "config", "--help"],
        ),
    ]

    for filename, title, cmd in shots:
        text = f"$ {' '.join(cmd[1:])}\n" + run(cmd, env, work)
        render_png(text, OUT / filename, title)

    # TUI frame via library helper
    preview = subprocess.run(
        [
            "cargo",
            "run",
            "-q",
            "--release",
            "--example",
            "tui_preview",
            "--",
            str(cfg_dir / "config.toml"),
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    if preview.returncode == 0 and preview.stdout.strip():
        render_png(preview.stdout, OUT / "config-ui-aliases.png", "gg config ui (Aliases)")
    else:
        print("tui_preview example failed; skipping TUI shot:")
        print(preview.stderr)

    # Wizard transcript (static, mirrors inquire prompts — not a live TTY record)
    wizard = """$ gg config wizard
What would you like to manage?
> Aliases
  Groups
  Tags
  Remotes
  Auto-enroll rules
  Settings
  Prune stale aliases
  Preview & save
  Quit

Aliases
> Add
  Remove
  Prune stale
  Done

> Alias name: payments-api
> Path: /tmp/demo/workspace/oss/payments-api
saved /home/demo/.git-gist/config.toml
"""
    render_png(wizard, OUT / "config-wizard.png", "gg config wizard")

    shutil.rmtree(tmp, ignore_errors=True)
    print("done")


if __name__ == "__main__":
    main()
