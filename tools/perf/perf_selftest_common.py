#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""自己較正の支え（perf-rank・perf-compare・judge-followup・perf-ledger の 4 組が使う）。

4 組の自己較正に同じ形で写されていた部分（ケースのフォルダの並べ方・ケースの鍵の読み・
本体の `main` を同じプロセスで呼ぶ `invoke`・最初の違い・赤の下限と結果の行・一時フォルダ）
をここに置く（#41）。道具ごとの違い（鍵の一覧・ケースの回し方・期待出力の突き合わせ・
赤の値）は道具の側に残す。judge-perf の自己較正は形が違うので寄せない。

一時フォルダはリポジトリの `target\\perf-loop-tmp\\` の下に作る（#1・D1）。OS の一時
フォルダは使わない。このモジュールは道具を読まない（依存は「自己較正 → 支え」の一方向）。
"""

from __future__ import annotations

import contextlib
import io
import re
import shutil
import sys
import uuid
from collections.abc import Callable, Iterator
from pathlib import Path

#: tools/perf/perf_selftest_common.py の 3 つ上（[0]=tools/perf・[1]=tools・[2]=根）。
REPO_ROOT = Path(__file__).resolve().parents[2]
if not (REPO_ROOT / "Cargo.toml").is_file():
    # 根を取り違えたまま target の下に作らない（.gitignore の target はどの深さにも当たり、git では見えない）。
    sys.stderr.write(f"[perf-selftest] repository root not found: no Cargo.toml in {REPO_ROOT}\n")
    raise SystemExit(3)
WORK_ROOT = REPO_ROOT / "target" / "perf-loop-tmp"

MISSING_LINE = "(no line)"

Fail = Callable[..., Exception]

#: `work_dir` が作った子と `invoke` が受けた stderr（`check_tree` の ⑴ と ⑶ が見る）。
_children_made: list[Path] = []
_stderr_seen: list[str] = []


@contextlib.contextmanager
def work_dir(prefix: str) -> Iterator[Path]:
    """`WORK_ROOT/<prefix><uuid4 の 16 進>` を作って渡し、抜けるときその子だけを消す。

    消せなかったら理由を stderr に 1 行出し、自己較正の結果は変えない（D1）。
    """
    WORK_ROOT.mkdir(parents=True, exist_ok=True)
    child = WORK_ROOT / f"{prefix}{uuid.uuid4().hex}"
    child.mkdir()
    _children_made.append(child)
    try:
        yield child
    finally:
        try:
            shutil.rmtree(child)
        except OSError as exc:
            sys.stderr.write(f"[perf-selftest] could not remove work dir {child}: {exc}\n")


def list_case_dirs(root: Path, case_file: str, fail: Fail, *,
                   materials_allowed: bool) -> tuple[list[Path], list[str]]:
    """ケースのフォルダを名前順に並べる。`.` と `_` で始まるフォルダは飛ばす。

    `materials_allowed` が真なら `case_file` の無いフォルダを素材置き場として名だけ返す。
    偽ならどのフォルダもケースとして返す（`case_file` の無いことは道具の読みが拒む）。
    """
    if not root.is_dir():
        raise fail(f"selftest case folder not found: {root}")
    cases: list[Path] = []
    materials: list[str] = []
    for child in sorted(root.iterdir()):
        if not child.is_dir() or child.name.startswith((".", "_")):
            continue
        if materials_allowed and not (child / case_file).is_file():
            materials.append(child.name)
        else:
            cases.append(child)
    if not cases:
        raise fail(
            f"selftest has no cases: {root}",
            ["a selftest without cases checks nothing and returns green."],
        )
    return cases, materials


def read_case_fields(path: Path, allowed: frozenset[str], multi: frozenset[str],
                     fail: Fail) -> tuple[dict[str, str], dict[str, list[str]]]:
    """`名前 = 値` の行を読む。`#` の行と空行は飛ばす。知らない鍵は `fail`（道具の入力不正）。

    `multi` の鍵は何行でも書け、出てきた順に並ぶ。それ以外の鍵は最後の値が残る。
    """
    single: dict[str, str] = {}
    multiple: dict[str, list[str]] = {key: [] for key in multi}
    for lineno, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        key, separator, value = line.partition("=")
        if not separator:
            raise fail(f"{path}:{lineno}: write `name = value` (got {line!a})")
        key, value = key.strip(), value.strip()
        if key in multi:
            multiple[key].append(value)
        elif key in allowed:
            single[key] = value
        else:
            names = ", ".join(sorted(allowed | multi))
            raise fail(f"{path}:{lineno}: unknown key: {key!a}", [f"allowed keys: {names}"])
    return single, multiple


def invoke(main: Callable[[list[str]], int], argv: list[str],
           exit_fallback: int) -> tuple[int, str, str]:
    """実運用と同じ入口 `main(argv)` を同じプロセスで呼ぶ（自己較正だけの近道は作らない）。"""
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        try:
            code = main(argv)
        except SystemExit as exc:
            code = exc.code if isinstance(exc.code, int) else exit_fallback
    _stderr_seen.append(err.getvalue())
    return code, out.getvalue(), err.getvalue()


def first_difference(label: str, want: str, got: str) -> str:
    """最初に違う行を出す。枠の語も英語で、違う行は `ascii()` で出す（端末へは ASCII だけ・#25）。"""
    want_lines, got_lines = want.splitlines(), got.splitlines()
    for index in range(max(len(want_lines), len(got_lines))):
        a = want_lines[index] if index < len(want_lines) else MISSING_LINE
        b = got_lines[index] if index < len(got_lines) else MISSING_LINE
        if a != b:
            return f"{label} line {index + 1}: want {ascii(a)} got {ascii(b)}"
    return f"{label}: no difference"


def ascii_outside_root(text: str, root: Path = REPO_ROOT) -> bool:
    """根の道筋を伏せたうえで ASCII か。根は利用者が選ぶ場所で、日本語のフォルダでも道具の誤りではない。"""
    return text.replace(str(root), "<root>").isascii()


def read_normalized(path: Path) -> str:
    """utf-8 で読み、CRLF を LF にそろえる。"""
    return path.read_text(encoding="utf-8").replace("\r\n", "\n")


def finish(results: list[tuple[str, str, list[str]]], materials: list[str], case_file: str,
           reproduced: set[int], required_red: tuple[int, ...]) -> int:
    """ケースごとの行・素材置き場の行・赤の下限の行・`SELFTEST RESULT` の行を出し、NG の数を返す。

    `results` は（ケース名・説明・食い違いの理由）。理由が空なら ok。
    """
    ok_count = ng_count = 0
    for name, title, reasons in results:
        if reasons:
            ng_count += 1
            print(f"[selftest] {name} NG  {title}")
            for reason in reasons:
                print(f"           - {reason}")
        else:
            ok_count += 1
            print(f"[selftest] {name} ok  {title}")

    for name in materials:
        print(f"[selftest] {name} --  {case_file} が無いので素材置き場として飛ばす")

    # 緑だけの自己較正は較正になっていない（道具が壊れていても緑は出る）。
    for code in required_red:
        if code not in reproduced:
            ng_count += 1
            print(f"[selftest] (赤の較正) NG  終了コード {code} を再現したケースが 1 件もありません")

    print(f"SELFTEST RESULT ok={ok_count} ng={ng_count}")
    return ng_count


# --- 木全体の見張り（T1・T13・T25・T26・T27） --------------------------------

#: `import tempfile`・`import os, tempfile`・`from tempfile import …` の行。
_TEMPFILE_IMPORT = re.compile(r"^[ \t]*(?:import[ \t][^#\n]*\btempfile\b|from[ \t]+tempfile\b)", re.M)
#: U+FEFF の字そのものか、その逃がし書き（この行は字を書かずに組む）。
_BOM_TEXTS = (chr(0xFEFF), "\\" + "ufeff", "\\" + "uFEFF", "\\" + "N{BYTE ORDER MARK}",
              "\\" + "N{ZERO WIDTH NO-BREAK SPACE}")
#: 字の書き手の付いた絶対の道筋（`C:\…`）。README の例は置き換え札か変数で書く（#27）。
_DRIVE_PATH = re.compile(r"(?<![\w<])[A-Za-z]:[\\/]")


def _find_root() -> Path | None:
    """自分の位置から上へ、`Cargo.toml` と `tools/perf/` のあるフォルダを探す（`REPO_ROOT` は使わない）。"""
    for folder in Path(__file__).resolve().parents:
        if (folder / "Cargo.toml").is_file() and (folder / "tools" / "perf").is_dir():
            return folder
    return None


def _readme_example_paths(readme: Path) -> list[str]:
    """README のコードの囲みの中で、字の書き手の付いた道筋を書いた行を返す。"""
    reasons: list[str] = []
    inside = False
    for lineno, line in enumerate(readme.read_text(encoding="utf-8").splitlines(), 1):
        if line.lstrip().startswith("```"):
            inside = not inside
        elif inside and _DRIVE_PATH.search(line):
            reasons.append(f"(5) {readme.name}:{lineno}: example path is not a placeholder"
                           f" (<worktree>\\target\\... or a variable): {ascii(line.strip())}")
    return reasons


def _helper_failure_texts() -> list[str]:
    """支えが自分で組む失敗の文（道具の stderr へそのまま出る）を、わざと 4 通り失敗させて集める。"""
    def fail(message: str, details: list[str] | tuple[str, ...] = ()) -> Exception:
        return ValueError(" | ".join([message, *details]))

    texts: list[str] = []
    with work_dir("perf-selftest-check-") as folder:
        bad = folder / "case.txt"
        attempts = [
            lambda: list_case_dirs(folder / "missing", "case.txt", fail, materials_allowed=False),
            lambda: list_case_dirs(folder, "case.txt", fail, materials_allowed=False),
        ]
        for body in ("no separator", "unknown = 1"):
            attempts.append(lambda body=body: (bad.write_text(body, encoding="utf-8"),
                                               read_case_fields(bad, frozenset({"title"}),
                                                                frozenset(), fail)))
        for attempt in attempts:
            try:
                attempt()
            except ValueError as exc:
                texts.append(str(exc))
            else:
                texts.append("(no failure raised)")
    return texts


def check_tree(*, ascii_stderr: bool, readme_examples: bool) -> list[str]:
    """木全体の見張り。食い違いの理由（ASCII）を返す。空なら合格。

    ⑴ tools/perf の .py が tempfile を読まない・`work_dir` の子が別に求めた根の target\\perf-loop-tmp の下
    ⑵ tools/perf の .py に U+FEFF の字が無い ⑷ `first_difference` が ASCII を出す
    ⑹ 支え自身の失敗の文（ケース置き場・case.txt の読み）が ASCII ⑺ ASCII の見張りが根の道筋を伏せる
    （⑶ ⑹ と道具の標準出力の見張りは `ascii_outside_root` で、日本語の根で赤にならない）。
    ⑶（`invoke` が受けた stderr の全行が ASCII）と ⑸（README の例の道筋）は道具の側から真にする。
    """
    reasons: list[str] = []
    root = _find_root()
    if root is None:
        return [f"(1) repository root not found above {Path(__file__).resolve()}"]
    work_root = root / "target" / "perf-loop-tmp"
    for path in sorted((root / "tools" / "perf").rglob("*.py")):
        text = path.read_text(encoding="utf-8")
        name = path.relative_to(root).as_posix()
        if _TEMPFILE_IMPORT.search(text):
            reasons.append(f"(1) {name} imports tempfile (use perf_selftest_common.work_dir)")
        if any(bom in text for bom in _BOM_TEXTS):
            reasons.append(f"(2) {name} has a U+FEFF character (decode with utf-8-sig)")
    for text in _helper_failure_texts():
        if not ascii_outside_root(text) or text == "(no failure raised)":
            reasons.append(f"(6) helper failure message is not ASCII (or not raised): {ascii(text)}")
    for child in _children_made:
        if child.resolve().parent != work_root:
            reasons.append(f"(1) work dir {ascii(str(child))} is not under {ascii(str(work_root))}")
    if ascii_stderr:
        for text in _stderr_seen:
            for line in text.splitlines():
                if not ascii_outside_root(line):
                    reasons.append(f"(3) stderr line is not ASCII: {ascii(line)}")
    fake_root = REPO_ROOT / "\u3042"
    if not ascii_outside_root(f"x: {fake_root / 'case.txt'}", fake_root) or ascii_outside_root("\u3042"):
        reasons.append("(7) ascii_outside_root does not mask a non-ASCII repository root")
    sample = first_difference("x", "\u00b5", "\u2014")
    if not sample.isascii():
        reasons.append(f"(4) first_difference is not ASCII: {ascii(sample)}")
    if readme_examples:
        reasons += _readme_example_paths(root / "tools" / "perf" / "README.md")
    return reasons
