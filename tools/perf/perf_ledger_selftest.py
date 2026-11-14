#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""`perf-ledger.py --selftest` の中身（自己較正のハーネス）。

**なぜ別ファイルか**: 本体 `perf-ledger.py` が 1,000 行の目安に達したため、台帳の読み書き
（本番の経路）と自己較正（道具を疑う経路）を分けた。入口は変わらず
`python tools/perf/perf-ledger.py --selftest` である。

**なぜ本体を import しないか**: 本体のファイル名にはハイフンがあり、Python の
モジュール名として import できない。そこで本体が起動時に `bind(<本体モジュール>)` で
自分を渡す（`perf_ledger_goal` と同じ・逆向きの import を作らない＝循環しない）。走らせる
入口は本体の `main(argv)` そのものであり、**自己較正だけの近道は作らない**。

1 ケース＝1 ディレクトリ（`fixtures-loop/ledger/<名前>/`）:

    case.txt   title=… / stderr_substr=… / note=…（期待の説明と標準エラーの部分一致）
    steps.txt  `<期待終了コード> :: <引数…>` を 1 行 1 手順（引数は空白区切り）
    expected_stdout.txt   全手順の標準出力を連結したものと逐語一致（任意）
    expected_ledger.md    走り終えた台帳と逐語一致（任意）
    ledger.md             最初から置いておく台帳（任意。無ければ `init` で作る）

`@LEDGER@` は複製先の `ledger.md`、`@DIR@` は複製先ディレクトリ、`@SHARED@` はケースが分け合う
素材の置き場 `fixtures-loop/ledger/_shared/`（`_` で始まるのでケースとしては読まない）に置き換える。
「緑だけの自己較正は較正になっていない」ため、本体の `SELFTEST_REQUIRED_RED_EXITS` に挙げた
終了コードを 1 件も再現していなければ自己較正そのものを不合格にする。
"""

from __future__ import annotations

import shutil
from dataclasses import dataclass
from pathlib import Path

import perf_ledger_goal
import perf_selftest_common as common

#: 本体モジュール（`bind()` が入れる）。
CORE = None


def bind(core) -> None:
    """本体モジュールを受け取る（`perf-ledger.py` が起動時に 1 度だけ呼ぶ）。"""
    global CORE
    CORE = core


# =============================================================================
# 定数（自己較正だけの較正値。台帳の値の判断には一切関与しない）
# =============================================================================

SELFTEST_CASE_FILENAME = "case.txt"
SELFTEST_STEPS_FILENAME = "steps.txt"
SELFTEST_LEDGER_FILENAME = "ledger.md"
SELFTEST_EXPECTED_LEDGER = "expected_ledger.md"
SELFTEST_EXPECTED_STDOUT = "expected_stdout.txt"
SELFTEST_STEP_SEPARATOR = " :: "
SELFTEST_LEDGER_PLACEHOLDER = "@LEDGER@"
SELFTEST_DIR_PLACEHOLDER = "@DIR@"
SELFTEST_SHARED_PLACEHOLDER = "@SHARED@"
SELFTEST_SHARED_DIRNAME = "_shared"
CRLF, LF = "\r\n", "\n"


@dataclass
class SelftestCase:
    """1 ケース＝1 ディレクトリ。`steps.txt` の各行を実運用と同じ入口へ流す。"""

    name: str
    directory: Path
    title: str
    stderr_substrings: list[str]
    steps: list[tuple[int, list[str]]]


def _read_case(directory: Path) -> SelftestCase:
    case_path = directory / SELFTEST_CASE_FILENAME
    steps_path = directory / SELFTEST_STEPS_FILENAME
    for path in (case_path, steps_path):
        if not path.is_file():
            raise CORE.bad_input(
                f"自己較正のケースに {path.name} がありません: {directory}",
                ["期待の書いていないケースは、走らせても合否を言えません。"],
            )
    fields, multi = common.read_case_fields(
        case_path, frozenset({"title", "note"}), frozenset({"stderr_substr"}), CORE.bad_input
    )
    title = fields.get("title", "")
    substrings = multi["stderr_substr"]

    steps: list[tuple[int, list[str]]] = []
    for lineno, raw in enumerate(steps_path.read_text(encoding="utf-8").splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        head, separator, rest = line.partition(SELFTEST_STEP_SEPARATOR)
        if not separator or not rest.split():
            raise CORE.bad_input(
                f"{steps_path}:{lineno}: 手順の書き方が違います",
                [f"`<期待終了コード>{SELFTEST_STEP_SEPARATOR}<引数…>` の形で書きます（引数は空白区切り）。"],
            )
        try:
            expected = int(head.strip())
        except ValueError as exc:
            raise CORE.bad_input(f"{steps_path}:{lineno}: 期待終了コードが整数ではありません（{head!r}）") from exc
        steps.append((expected, rest.split()))
    if not steps:
        raise CORE.bad_input(f"{steps_path} に手順が 1 つもありません", ["何も走らせないケースは較正になりません。"])
    return SelftestCase(directory.name, directory, title or "(説明なし)", substrings, steps)


def _entry_region(path: Path, prefix: str) -> str:
    """台帳のうち「周の記録」の部分だけを取り出す（状態ブロック書き換えの検査用）。"""
    if not path.is_file():
        return ""
    text = common.read_normalized(path)
    index = text.find("\n" + prefix)
    return "" if index < 0 else text[index:]


def _run_case(case: SelftestCase, workspace: Path) -> tuple[list[str], set[int]]:
    """ケースを 1 つ走らせ、食い違いの一覧と「実際に再現した終了コード」を返す。"""
    work = workspace / case.name
    shutil.copytree(case.directory, work)
    ledger = work / SELFTEST_LEDGER_FILENAME

    # 周の記録の見出しの先頭（`## 周 `）。状態ブロック書き換えの不変条件検査に使う。
    prefix = CORE.ENTRY_HEADING_FORMAT.split("{")[0]
    reasons: list[str] = []
    reproduced: set[int] = set()
    stdout_all = stderr_all = ""
    for expected, raw_argv in case.steps:
        argv = [
            token.replace(SELFTEST_LEDGER_PLACEHOLDER, str(ledger))
            .replace(SELFTEST_DIR_PLACEHOLDER, str(work))
            .replace(SELFTEST_SHARED_PLACEHOLDER, str(case.directory.parent / SELFTEST_SHARED_DIRNAME))
            for token in raw_argv
        ]
        before = _entry_region(ledger, prefix)
        code, out, err = common.invoke(CORE.main, argv, CORE.EXIT_FAIL)
        stdout_all += out
        stderr_all += err
        if code == expected:
            reproduced.add(code)
        else:
            reasons.append(f"{argv[0]}: exit code: want {expected} got {code}")
        if argv[0] == "set-phase" and _entry_region(ledger, prefix) != before:
            reasons.append("set-phase rewrote an iteration entry (it may rewrite only the state block)")

    expected_stdout = case.directory / SELFTEST_EXPECTED_STDOUT
    if expected_stdout.is_file():
        want = common.read_normalized(expected_stdout)
        got = stdout_all.replace(CRLF, LF)
        if want != got:
            reasons.append(common.first_difference("stdout", want, got))

    expected_ledger = case.directory / SELFTEST_EXPECTED_LEDGER
    if expected_ledger.is_file():
        want = common.read_normalized(expected_ledger)
        got = common.read_normalized(ledger) if ledger.is_file() else ""
        if want != got:
            reasons.append(common.first_difference("ledger", want, got))

    for substring in case.stderr_substrings:
        if substring not in stderr_all:
            reasons.append(f"stderr does not contain {ascii(substring)}")
    return reasons, reproduced


def _goal_text_check_reasons() -> list[str]:
    """goal-text の検査（`check_goal_text`）を直に呼ぶ: 終端の前置きで始まる行と 4,000 字の境目。"""
    limit = perf_ledger_goal.GOAL_TEXT_MAX_CHARS
    probes = (
        ("line starting with the FINAL prefix", f"目標\n{CORE.FINAL_PREFIX}GOAL_MET run=12345678 が出たら達成\n", False),
        ("line starting with the STATUS prefix", f"{CORE.STATUS_PREFIX}iter=<n> を返す\n", False),
        (f"exactly {limit} chars", "a" * limit, True),
        (f"{limit + 1} chars", "a" * (limit + 1), False),
    )
    reasons: list[str] = []
    for label, text, should_pass in probes:
        try:
            perf_ledger_goal.check_goal_text(text)
            passed = True
        except CORE.LedgerError:
            passed = False
        if passed != should_pass:
            reasons.append(f"{label}: want {'pass' if should_pass else 'reject'} got {'passed' if passed else 'rejected'}")
    return reasons


def run_selftest() -> int:
    """`fixtures-loop/ledger/` の既知ケースを逐語再現する（要件 6.7）。"""
    directories, _ = common.list_case_dirs(
        CORE.script_dir() / CORE.SELFTEST_FIXTURES_DIRNAME / CORE.SELFTEST_LEDGER_SUBDIR, SELFTEST_CASE_FILENAME, CORE.bad_input, materials_allowed=False
    )
    cases = [_read_case(child) for child in directories]

    results: list[tuple[str, str, list[str]]] = []
    reproduced_all: set[int] = set()
    with common.work_dir("perf-ledger-selftest-") as temporary:
        for case in cases:
            reasons, reproduced = _run_case(case, temporary)
            reproduced_all |= reproduced
            results.append((case.name, case.title, reasons))

    results.append(("(goal_text_check)", "goal-text の検査は終端の前置きで始まる行を拒み、4,000 字ちょうどは通す",
                    _goal_text_check_reasons()))
    # 木全体の見張り（⑸ README の例は有効・⑶ stderr の ASCII は D9 で台帳の失敗の文が日本語のままなので無効）。
    results.append(("(check_tree)", "木全体の見張り",
                    common.check_tree(ascii_stderr=False, readme_examples=True)))
    ng_count = common.finish(
        results, [], SELFTEST_CASE_FILENAME, reproduced_all, CORE.SELFTEST_REQUIRED_RED_EXITS
    )
    return CORE.EXIT_OK if ng_count == 0 else CORE.EXIT_FAIL
