"""Read the current wave that /kiro-next wrote into roadmap.md.

Usage:
  MSYS_NO_PATHCONV=1 git show origin/main:.kiro/steering/roadmap.md | python -I read-wave.py
  (Git Bash rewrites "origin/main:..." as a Windows path without MSYS_NO_PATHCONV=1.)
  ... | python -I read-wave.py --models <name>...   (requirements model of named specs)
  python -I read-wave.py --self-test

Prints "date <YYYY-MM-DD>" for the wave, then one line per spec: "<model> <name>"
(model is Fable or Opus, name is the full spec name with the areka-P0- prefix).
Warnings go to stderr as "WARN ...". Exit 1 when no wave can be found.
The wave is the first fenced block of the "## ウェーブ編成（…）" section.
A spec is Fable when the block lists it under Fable OR the spec ledger's
"Fable" column says ○ — the stricter of the two wins.
With --models, prints "<Fable|Opus|unknown> <name>" per named spec (with or
without the prefix), looked up in the spec ledger (header "spec…" … "Fable").
"""
import re
import sys

WAVES = "## ウェーブ編成（"
FENCE = "```"
PREFIX = "areka-P0-"


def full_name(name):
    return name if name.startswith(PREFIX) else PREFIX + name


def read_models(text):
    """Requirements model per spec from every table whose first header cell starts
    with "spec" and that has a "Fable" column: ○ → Fable, − → Opus."""
    models, col = {}, None
    for l in text.splitlines():
        if not l.startswith("|"):
            col = None
            continue
        cells = [c.strip() for c in l.strip().strip("|").split("|")]
        if col is None:
            col = cells.index("Fable") if cells[0].startswith("spec") and "Fable" in cells else False
            continue
        if not col or set(cells[0]) <= set("-: ") or len(cells) <= col:
            continue
        m = re.match(r"`([a-z0-9-]+)`", cells[0])
        mark = cells[col]
        kind = "Fable" if mark.startswith("○") else "Opus" if mark.startswith(("−", "-")) else None
        if m and kind and models.get(full_name(m.group(1))) != "Fable":
            models[full_name(m.group(1))] = kind
    return models


def read_wave(text):
    lines = text.splitlines()
    start = next((i for i, l in enumerate(lines) if l.startswith(WAVES)), None)
    if start is None:
        raise ValueError("no wave section (## ウェーブ編成（…）)")
    m = re.search(r"\d{4}-\d{2}-\d{2}", lines[start])
    if m is None:
        raise ValueError("no date in the wave section heading")
    end = next((i for i in range(start + 1, len(lines)) if lines[i].startswith("## ")), len(lines))
    sec = lines[start + 1:end]
    open_ = next((i for i, l in enumerate(sec) if l.strip().startswith(FENCE)), None)
    if open_ is None:
        raise ValueError("no start-command code block in the wave section")
    close = next((i for i in range(open_ + 1, len(sec)) if sec[i].strip().startswith(FENCE)), None)
    if close is None:
        raise ValueError("unterminated code block in the wave section")

    ledger = read_models(text)
    specs, warns, group = [], [], None
    for l in sec[open_ + 1:close]:
        s = l.strip()
        if s in ("Fable", "Opus"):
            group = s
        elif s.startswith("【"):
            group = None
        elif s.startswith("/kiro-start "):
            name = full_name(s.split()[1])
            model = group or "Opus"
            if ledger.get(name) == "Fable" and model != "Fable":
                warns.append(f"{name}: ledger says Fable, block says {model}; treated as Fable")
                model = "Fable"
            specs.append((model, name))
    if not specs:
        raise ValueError("the code block lists no /kiro-start command")
    return m.group(0), specs, warns


SAMPLE = """# Roadmap
## ウェーブ編成（着手順の正本・2026-01-02 棚卸㉙）
| Wave | ユニット | 開始コマンド | 根拠 |
|---|---|---|---|
| **C1** | x | `/kiro-start areka-P0-<名>` | y |

**C1 の開始コマンド**:

```
Wave C1 3本（v0.1.0 2本＋割り込み 1本）
【本来の次の版 v0.1.0】
Fable
/kiro-start areka-P0-b-spec
Opus
/kiro-start areka-P0-a-spec

【割り込み（版に縛らない・v0.1.0 の着地を遅らせない）】
Opus
/kiro-start areka-P0-c-spec
```
### 次
## spec 台帳
| spec（`areka-P0-` 省略） | 段 | 前提 | Fable |
|---|---|---|---|
| `a-spec`（**01-01 起票**） | 優先 | なし | − |
| `b-spec`（**01-01 起票**） | 優先 | なし | ○ |
| `c-spec` | その他 | なし | ○（理由） |
| `d-spec` | その他 | なし | 未定 |
"""


def self_test():
    date, specs, warns = read_wave(SAMPLE)
    assert date == "2026-01-02", date
    assert specs == [("Fable", "areka-P0-b-spec"), ("Opus", "areka-P0-a-spec"), ("Fable", "areka-P0-c-spec")], specs
    assert len(warns) == 1 and warns[0].startswith("areka-P0-c-spec"), warns
    models = read_models(SAMPLE)
    assert models == {"areka-P0-a-spec": "Opus", "areka-P0-b-spec": "Fable", "areka-P0-c-spec": "Fable"}, models
    for bad in ("# none\n", "## ウェーブ編成（2026-01-02）\nno block\n", "## ウェーブ編成（x）\n```\n/kiro-start a\n```\n"):
        try:
            read_wave(bad)
            raise AssertionError("expected ValueError")
        except ValueError:
            pass
    print("selftest: all passed")


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    sys.stderr.reconfigure(encoding="utf-8")
    if "--self-test" in sys.argv:
        self_test()
        return 0
    if "--models" in sys.argv:
        models = read_models(sys.stdin.buffer.read().decode("utf-8"))
        for name in sys.argv[sys.argv.index("--models") + 1:]:
            print(f"{models.get(full_name(name), 'unknown')} {full_name(name)}")
        return 0
    try:
        date, specs, warns = read_wave(sys.stdin.buffer.read().decode("utf-8"))
    except ValueError as e:
        print(f"FAIL {e}", file=sys.stderr)
        return 1
    for w in warns:
        print(f"WARN {w}", file=sys.stderr)
    print(f"date {date}")
    for model, name in specs:
        print(f"{model} {name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
