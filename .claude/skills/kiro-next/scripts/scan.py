"""Step 0-3 groundwork for /kiro-next: the facts an inventory needs, in a few lines.

Usage (run at the worktree root; the scratch directory must exist):
  python -I .claude/skills/kiro-next/scripts/scan.py --out <scratch> [--base <sha>]
  python -I .claude/skills/kiro-next/scripts/scan.py --self-test

Prints: the previous inventory (number, date, base) and the current base, the specs that landed in
between, branches that may be in flight, the current version (the first "版の段" row **vX.Y.Z**
whose tag is not on origin) and its unfinished specs, the ledger-vs-folder check, counts, the
roadmap headings with line ranges, the specs to re-measure (every current-version spec) and the
version-free fillers the diff moved, with the reason for each. Writes
<out>/diff-files.md (files changed since the previous inventory) and <out>/facts-<spec>.md for each
picked spec (the files its latest measurement names: tracked or not, lines, changed since then).
Exit 1 when the roadmap has no previous inventory heading.
"""
import os
import re
import subprocess
import sys

ROADMAP = ".kiro/steering/roadmap.md"
SPECS = ".kiro/specs"
INVENTORY = re.compile(r"^## 棚卸(\S+?)の裁定（(\d{4}-\d{2}-\d{2})・main `([0-9a-f]{7,40})`")
VERSION_ROW = re.compile(r"^\|\s*\*\*v(\d+\.\d+\.\d+)\*\*\s*\|")
SPEC_NAME = re.compile(r"^[a-z0-9]+(-[a-z0-9]+)+$")
REMEASURE = re.compile(r"棚卸\S*の再測定")
PREFIX = "areka-P0-"  # old names of specs filed before 2026-10-23
OPEN, CLOSE = "（〔「", "）〕」"
# Backticked tokens that look like a path: a slash, or a file extension
PATH_TOKEN = re.compile(r"`([^`\s]*(?:/|\.(?:rs|md|toml|yml|yaml|ps1|psm1|js|json|txt|html|py|lock|hlsl|wgsl))[^`\s]*)`")


def git(*args):
    r = subprocess.run(["git", *args], capture_output=True, text=True, encoding="utf-8")
    if r.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)}: {r.stderr.strip()}")
    return r.stdout


def key(name):
    return name[len(PREFIX):] if name.startswith(PREFIX) else name


def cells(row):
    return [c.strip() for c in row.strip().strip("|").split("|")]


def top_names(cell):
    """Backticked spec names outside every bracket (（）〔〕「」) of a cell, in order."""
    out, depth, i = [], 0, 0
    while i < len(cell):
        ch = cell[i]
        if ch in OPEN:
            depth += 1
        elif ch in CLOSE:
            depth = max(depth - 1, 0)
        elif ch == "`":
            j = cell.find("`", i + 1)
            if j < 0:
                break
            if depth == 0 and SPEC_NAME.match(key(cell[i + 1:j])):
                out.append(key(cell[i + 1:j]))
            i = j
        i += 1
    return out


def prev_inventory(lines):
    """(number, date, sha) of the newest 棚卸NNの裁定 heading."""
    for l in lines:
        m = INVENTORY.match(l)
        if m:
            return m.groups()
    return None


def headings(lines):
    """(start, end, text) for every ## and ### heading; end is the last line of its section (1-based)."""
    idx = [(i, l) for i, l in enumerate(lines) if l.startswith("## ") or l.startswith("### ")]
    out = []
    for k, (i, l) in enumerate(idx):
        level = l.split(" ")[0]
        end = next((j for j, m in idx[k + 1:] if len(m.split(" ")[0]) <= len(level)), len(lines))
        out.append((i + 1, end, l))
    return out


def section(lines, prefix):
    start = next((i for i, l in enumerate(lines) if l.startswith(prefix)), None)
    if start is None:
        return []
    end = next((i for i in range(start + 1, len(lines)) if lines[i].startswith("## ")), len(lines))
    return lines[start + 1:end]


def version_rows(lines):
    """[(version, [spec names])] for the **vX.Y.Z** rows of the 版の段 table, in table order."""
    out = []
    for l in section(lines, "## 版の段"):
        m = VERSION_ROW.match(l)
        if m:
            c = cells(l)
            out.append((m.group(1), top_names(c[2]) if len(c) > 2 else []))
    return out


def free_rows(lines):
    """Spec names of the 版の段 rows marked 版に縛らない (the only fillers the version gate admits)."""
    out = []
    for l in section(lines, "## 版の段"):
        c = cells(l) if l.startswith("|") else []
        if len(c) > 2 and "版に縛らない" in c[0]:
            out += top_names(c[2])
    return out


def ledger(lines):
    """{name: deps} from the spec 台帳 table (deps = the 前提 column's top-level names)."""
    rows, col = {}, None
    for l in section(lines, "## spec 台帳"):
        if not l.startswith("|"):
            continue
        c = cells(l)
        if c[0].startswith("spec"):
            col = next((k for k, h in enumerate(c) if h.startswith("前提")), None)
            continue
        names = top_names(c[0])
        if names:
            rows.setdefault(names[0], top_names(c[col]) if col is not None and col < len(c) else [])
    return rows


def latest_remeasure(text):
    """The newest 棚卸NNの再測定 section of a brief (the last one), else the whole brief."""
    parts = re.split(r"(?m)^(?=## )", text.replace("\r\n", "\n"))
    hits = [p for p in parts if REMEASURE.search(p.split("\n", 1)[0])]
    return hits[-1] if hits else text


def path_tokens(text):
    seen = []
    for t in PATH_TOKEN.findall(text):
        t = t.removeprefix("./").rstrip("、。,")
        # a slash command, or a bare top-level folder (`tools/`), names nothing worth matching
        if t and t not in seen and not t.startswith(("http", "/")) and not re.fullmatch(r"[^/]+/", t):
            seen.append(t)
    return seen


# Changed files that never make a spec worth re-measuring: steering, specs and the workflow skills,
# and the generated files (allowed overlaps)
QUIET = re.compile(r"^(\.kiro/|\.claude/skills/kiro-|THIRD-PARTY-NOTICES\.md$|doc/ukadoc-coverage/report/)")


def touches(token, path):
    """Does a brief's path token name this repository path (suffix match, or a folder prefix)?"""
    if token.endswith("/"):
        return f"/{token}" in f"/{path}"
    return path == token or path.endswith("/" + token)


def pick(goal_specs, candidates, briefs, diff, landed, rows):
    """[(spec, reason)] to re-measure: every current-version spec, then the other candidates (the
    version-free fillers) whose latest measurement names a changed file, or whose functional
    predecessor landed. Later-version specs wait until their version is the current one."""
    out = [(s, "version") for s in goal_specs]
    diff = [p for p in diff if not QUIET.match(p)]
    for s in sorted(candidates):
        if s in goal_specs:
            continue
        hit = next((p for t in path_tokens(latest_remeasure(briefs.get(s, ""))) for p in diff if touches(t, p)), None)
        dep = next((d for d in rows.get(s, []) if d in landed), None)
        if hit:
            out.append((s, f"diff: {hit}"))
        elif dep:
            out.append((s, f"dep landed: {dep}"))
    return out


def facts(tokens, tracked, diff):
    """One line per token: the tracked files it names, their lines, and whether they changed."""
    lines, seen = [], set()
    for t in tokens[:40]:
        files = [p for p in tracked if touches(t, p) and p not in seen and (t.startswith(".kiro") or not p.startswith(".kiro/"))]
        seen.update(files)
        if not files and any(touches(t, p) for p in tracked):
            continue
        if not files:
            lines.append(f"- `{t}`: not tracked (new file, or a stale name)")
        elif len(files) > 3:
            ch = sum(1 for p in files if p in diff)
            lines.append(f"- `{t}`: {len(files)} files, {ch} changed")
        else:
            for p in files:
                try:
                    n = sum(1 for _ in open(p, encoding="utf-8", errors="replace"))
                except OSError:
                    n = "?"
                lines.append(f"- `{p}`: {n} lines{', changed' if p in diff else ''}")
    return lines


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    if "--self-test" in sys.argv:
        return self_test()
    out = sys.argv[sys.argv.index("--out") + 1]
    base = sys.argv[sys.argv.index("--base") + 1] if "--base" in sys.argv else git("rev-parse", "HEAD").strip()
    lines = open(ROADMAP, encoding="utf-8").read().splitlines()
    prev = prev_inventory(lines)
    if prev is None:
        print("FAIL no previous inventory heading (## 棚卸NNの裁定（…・main `…`）)", file=sys.stderr)
        return 1
    num, date, prev_sha = prev
    landed = sorted({key(p.split("/")[3]) for p in git("diff", "--name-only", "--diff-filter=A", prev_sha, base, "--", f"{SPECS}/completed/").split()})
    diff = git("diff", "--name-only", prev_sha, base).split()
    tracked = git("ls-files").split()
    folders = {key(d): d for d in os.listdir(SPECS) if d != "completed" and os.path.isdir(f"{SPECS}/{d}")}
    unfinished = set(folders)
    rows = ledger(lines)
    briefs = {s: open(f"{SPECS}/{d}/brief.md", encoding="utf-8").read() for s, d in folders.items() if os.path.exists(f"{SPECS}/{d}/brief.md")}

    tags = set(re.findall(r"refs/tags/(v\d+\.\d+\.\d+)$", git("ls-remote", "--tags", "origin", "refs/tags/v*"), re.M))
    goal = next(((v, names) for v, names in version_rows(lines) if f"v{v}" not in tags), None)
    goal_specs = [n for n in goal[1] if n in unfinished] if goal else []

    me = git("rev-parse", "--abbrev-ref", "HEAD").strip()
    branches = [b.strip() for b in git("branch", "-r", "--format=%(refname:short)").split() if b.startswith("origin/claude/")]
    branches += [l.split("refs/heads/")[1] for l in git("worktree", "list", "--porcelain").splitlines() if l.startswith("branch refs/heads/")]
    merged = set(git("branch", "-r", "--merged", base, "--format=%(refname:short)").split())
    inflight = sorted({(s, b) for b in branches if me not in b and b not in merged for s in unfinished if s in b})

    print(f"prev 棚卸{num} {date} {prev_sha} -> base {base[:8]}")
    print(f"landed ({len(landed)}): {', '.join(landed) or 'none'}")
    print(f"diff: {len(diff)} files -> {out}/diff-files.md")
    print("in flight? " + ("; ".join(f"{s} ({b})" for s, b in inflight) or "none") + "  (name match only: check each)")
    print(f"current version: {'v' + goal[0] + ' (no tag)' if goal else 'none'}; unfinished in it ({len(goal_specs)}): {', '.join(goal_specs) or 'none'}")
    print(f"ledger: {len(rows)} rows / {len(unfinished)} folders"
          f"; rows without folder: {', '.join(sorted(set(rows) - unfinished)) or 'none'}"
          f"; folders without row: {', '.join(sorted(unfinished - set(rows))) or 'none'}")
    print(f"counts: completed {len(os.listdir(SPECS + '/completed'))}, briefs {len(briefs)}, folders {len(unfinished)}")
    print("roadmap headings (line ranges):")
    for s, e, h in headings(lines):
        print(f"  L{s}-{e} {h[:80]}")
    free = [n for n in free_rows(lines) if n in unfinished]
    print(f"version-free fillers ({len(free)}); later-version specs carried forward: {len(unfinished) - len(goal_specs) - len(free)}")
    picks = pick(goal_specs, free if goal else unfinished, briefs, diff, set(landed), rows)
    for title, group in (("re-measure", [x for x in picks if x[1] == "version"]),
                         ("fillers the diff moved (re-measure only those a spare seat may take)", [x for x in picks if x[1] != "version"])):
        print(f"{title} ({len(group)}):")
        for s, why in group:
            print(f"  {s} -- {why}")
    for s, why in picks:
        sheet = facts(path_tokens(latest_remeasure(briefs.get(s, ""))), tracked, set(diff))
        with open(f"{out}/facts-{s}.md", "w", encoding="utf-8") as f:
            f.write(f"# {s} -- files its latest measurement names (base {base[:8]}, changed = since {prev_sha})\n" + "\n".join(sheet) + "\n")
    with open(f"{out}/diff-files.md", "w", encoding="utf-8") as f:
        f.write("\n".join(diff) + "\n")
    return 0


SAMPLE = """# Roadmap
## 版の段（見本）
| 版 | 目標 | spec（並びは依存の順） |
|---|---|---|
| **v0.1.0** | 目標 | 道具＝✅ `done-one`（`not-me` の後） → `next-one`／`areka-P0-old-one` |
| **v0.1.0 の余力の枠** | 数えない | `spare-one` |
| **v0.2.0** | 基幹 | `far-one` |
| **版に縛らない（道具）** | 空いた席 | `free-one`・✅ `free-done` |
## ウェーブ編成（着手順の正本・2026-01-02 棚卸㉙）
### 直列の列
## spec 台帳（brief を持つ 3 本）
| spec（旧い `areka-P0-` は省略） | 段 | 何をするか | 規模 | 並び | 前提（先に着地） | Fable |
|---|---|---|---|---|---|---|
| `next-one`（**01-01 起票**・`x-y` から） | 優先 | 次 | 5 | 1 | `done-one`（`not-dep` の注） | − |
| `areka-P0-old-one` | 優先 | 古い | 5 | 2 | なし | − |
| `far-one` | 優先 | 遠い | 5 | 3 | `next-one`・`other-x` | − |
## 棚卸㉙の裁定（2026-01-02・main `abc1234`・開発者指示）
## 棚卸㉘の裁定（2026-01-01・main `0000000`）
"""

BRIEF = """# Brief: far-one\r
古い `crates/a/old.rs`\r
## 2026-01-01 棚卸㉘の再測定（main `aaa`）\r
- 触るファイル: `crates/a/old.rs`\r
## 2026-01-02 棚卸㉙の再測定（main `bbb`）\r
- 触るファイル: `crates/b/new.rs`、`doc/x/`、`tools/`、`/kiro-next`、https://example.com/a.md は無視\r
"""


def self_test():
    lines = SAMPLE.splitlines()
    assert prev_inventory(lines) == ("㉙", "2026-01-02", "abc1234"), prev_inventory(lines)
    assert version_rows(lines) == [("0.1.0", ["done-one", "next-one", "old-one"]), ("0.2.0", ["far-one"])], version_rows(lines)
    rows = ledger(lines)
    assert rows == {"next-one": ["done-one"], "old-one": [], "far-one": ["next-one", "other-x"]}, rows
    h = headings(lines)
    assert free_rows(lines) == ["free-one", "free-done"], free_rows(lines)
    assert h[0][2].startswith("## 版の段") and any(x[2].startswith("### 直列") and x[1] == 10 for x in h), h
    sec = latest_remeasure(BRIEF)
    assert "new.rs" in sec and "old.rs" not in sec
    assert path_tokens(sec) == ["crates/b/new.rs", "doc/x/"], path_tokens(sec)
    assert touches("b/new.rs", "crates/b/new.rs") and not touches("new.rs", "crates/b/renew.rs")
    assert touches("doc/x/", "doc/x/y.md") and not touches("x/", "doc/xx/y.md")
    unfinished = {"next-one", "far-one", "idle-one"}
    briefs = {"far-one": BRIEF, "idle-one": "`crates/a/old.rs`"}
    got = pick(["next-one"], unfinished, briefs, ["crates/b/new.rs"], set(), rows)
    assert got == [("next-one", "version"), ("far-one", "diff: crates/b/new.rs")], got
    assert pick([], unfinished, briefs, [".kiro/steering/x/a.md", ".claude/skills/kiro-next/old.rs", "THIRD-PARTY-NOTICES.md"], set(), rows) == []
    got = pick([], unfinished, {}, [], {"next-one"}, rows)
    assert got == [("far-one", "dep landed: next-one")], got
    assert facts(["zz/none.rs"], ["crates/a.rs"], set()) == ["- `zz/none.rs`: not tracked (new file, or a stale name)"]
    got = facts(["a.rs", "crates/a.rs", "tech.md"], ["crates/a.rs", ".kiro/x/tech.md"], set())
    assert got == ["- `crates/a.rs`: ? lines"], got
    print("selftest: all passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
