#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""`perf-ledger.py --selftest` の中身（自己較正のハーネス）。

**なぜ別ファイルか**: 本体 `perf-ledger.py` が 1,000 行の目安に達したため、台帳の読み書き
（本番の経路）と自己較正（道具を疑う経路）を分けた。入口は変わらず
`python tools/perf/perf-ledger.py --selftest` である。

**なぜ本体を import しないか**: 本体のファイル名にはハイフンがあり、Python の
モジュール名として import できない。そこで本体が起動時に `run_selftest(env)` へ
`SelftestEnv` を渡す（＝逆向きの import を作らない＝循環しない）。`env.invoke` は
本体の `main(argv)` そのものであり、**自己較正だけの近道は作らない**。

1 ケース＝1 ディレクトリ（`fixtures-loop/ledger/<名前>/`）:

    case.txt   title=… / stderr_substr=… / note=…（期待の説明と標準エラーの部分一致）
    steps.txt  `<期待終了コード> :: <引数…>` を 1 行 1 手順（引数は空白区切り）
    expected_stdout.txt   全手順の標準出力を連結したものと逐語一致（任意）
    expected_ledger.md    走り終えた台帳と逐語一致（任意）
    ledger.md             最初から置いておく台帳（任意。無ければ `init` で作る）

`@LEDGER@` は複製先の `ledger.md`、`@DIR@` は複製先ディレクトリに置き換える。
「緑だけの自己較正は較正になっていない」ため、`env.required_red_exits` に挙げた
終了コードを 1 件も再現していなければ自己較正そのものを不合格にする。
"""

from __future__ import annotations

import contextlib
import io
import shutil
import tempfile
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

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
#: `case.txt` に書ける項目（これ以外は書き間違いとして拒む）。
SELFTEST_CASE_FIELDS = ("title", "stderr_substr", "note")


@dataclass(frozen=True)
class SelftestEnv:
    """本体から渡される道具立て（本体を import しないための唯一の受け口）。"""

    fixtures_root: Path
    #: 実運用と同じ入口（本体の `main(argv)`）。
    invoke: Callable[[list[str]], int]
    #: 期待の書けていないケースを拒むときに投げる例外を作る（本体の `bad_input`）。
    fail: Callable[..., Exception]
    #: 周の記録の見出しの先頭（`## 周 `）。状態ブロック書き換えの不変条件検査に使う。
    entry_heading_prefix: str
    ok_code: int
    fail_code: int
    #: 1 件も再現しなければ自己較正を不合格にする終了コード。
    required_red_exits: tuple[int, ...]


@dataclass
class SelftestCase:
    """1 ケース＝1 ディレクトリ。`steps.txt` の各行を実運用と同じ入口へ流す。"""

    name: str
    directory: Path
    title: str
    stderr_substrings: list[str]
    steps: list[tuple[int, list[str]]]


def _read_case(directory: Path, env: SelftestEnv) -> SelftestCase:
    case_path = directory / SELFTEST_CASE_FILENAME
    steps_path = directory / SELFTEST_STEPS_FILENAME
    for path in (case_path, steps_path):
        if not path.is_file():
            raise env.fail(
                f"自己較正のケースに {path.name} がありません: {directory}",
                ["期待の書いていないケースは、走らせても合否を言えません。"],
            )
    title = ""
    substrings: list[str] = []
    for raw in case_path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, _, value = line.partition("=")
        if key == "title":
            title = value
        elif key == "stderr_substr":
            substrings.append(value)
        elif key not in SELFTEST_CASE_FIELDS:
            raise env.fail(
                f"{case_path} の知らない項目です: {key!r}",
                [f"使えるのは {'・'.join(SELFTEST_CASE_FIELDS)} です。"],
            )

    steps: list[tuple[int, list[str]]] = []
    for lineno, raw in enumerate(steps_path.read_text(encoding="utf-8").splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        head, separator, rest = line.partition(SELFTEST_STEP_SEPARATOR)
        if not separator or not rest.split():
            raise env.fail(
                f"{steps_path}:{lineno}: 手順の書き方が違います",
                [f"`<期待終了コード>{SELFTEST_STEP_SEPARATOR}<引数…>` の形で書きます（引数は空白区切り）。"],
            )
        try:
            expected = int(head.strip())
        except ValueError as exc:
            raise env.fail(f"{steps_path}:{lineno}: 期待終了コードが整数ではありません（{head!r}）") from exc
        steps.append((expected, rest.split()))
    if not steps:
        raise env.fail(f"{steps_path} に手順が 1 つもありません", ["何も走らせないケースは較正になりません。"])
    return SelftestCase(directory.name, directory, title or "(説明なし)", substrings, steps)


def _invoke(env: SelftestEnv, argv: list[str]) -> tuple[int, str, str]:
    """実運用と同じ入口 `main(argv)` を呼ぶ（自己較正だけの近道は作らない）。"""
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        try:
            code = env.invoke(argv)
        except SystemExit as exc:
            code = exc.code if isinstance(exc.code, int) else env.fail_code
    return code, out.getvalue(), err.getvalue()


def _entry_region(path: Path, prefix: str) -> str:
    """台帳のうち「周の記録」の部分だけを取り出す（状態ブロック書き換えの検査用）。"""
    if not path.is_file():
        return ""
    text = path.read_text(encoding="utf-8").replace("\r\n", "\n")
    index = text.find("\n" + prefix)
    return "" if index < 0 else text[index:]


def _first_difference(label: str, want: str, got: str) -> str:
    want_lines, got_lines = want.splitlines(), got.splitlines()
    for index in range(max(len(want_lines), len(got_lines))):
        a = want_lines[index] if index < len(want_lines) else "(行が無い)"
        b = got_lines[index] if index < len(got_lines) else "(行が無い)"
        if a != b:
            return f"{label} {index + 1} 行目: 期待 {a!r} 実際 {b!r}"
    return f"{label}: 差はありません"


def _run_case(case: SelftestCase, workspace: Path, env: SelftestEnv) -> tuple[list[str], set[int]]:
    """ケースを 1 つ走らせ、食い違いの一覧と「実際に再現した終了コード」を返す。"""
    work = workspace / case.name
    shutil.copytree(case.directory, work)
    ledger = work / SELFTEST_LEDGER_FILENAME

    reasons: list[str] = []
    reproduced: set[int] = set()
    stdout_all = stderr_all = ""
    for expected, raw_argv in case.steps:
        argv = [
            token.replace(SELFTEST_LEDGER_PLACEHOLDER, str(ledger)).replace(
                SELFTEST_DIR_PLACEHOLDER, str(work)
            )
            for token in raw_argv
        ]
        before = _entry_region(ledger, env.entry_heading_prefix)
        code, out, err = _invoke(env, argv)
        stdout_all += out
        stderr_all += err
        if code == expected:
            reproduced.add(code)
        else:
            reasons.append(f"{argv[0]}: 期待 {expected} 実際 {code}")
        if argv[0] == "set-phase" and _entry_region(ledger, env.entry_heading_prefix) != before:
            reasons.append("set-phase が周の記録を書き換えました（状態ブロックだけを書き換えること）")

    expected_stdout = case.directory / SELFTEST_EXPECTED_STDOUT
    if expected_stdout.is_file():
        want = expected_stdout.read_text(encoding="utf-8").replace("\r\n", "\n")
        if want != stdout_all.replace("\r\n", "\n"):
            reasons.append(_first_difference("標準出力", want, stdout_all.replace("\r\n", "\n")))

    expected_ledger = case.directory / SELFTEST_EXPECTED_LEDGER
    if expected_ledger.is_file():
        want = expected_ledger.read_text(encoding="utf-8").replace("\r\n", "\n")
        got = ledger.read_text(encoding="utf-8").replace("\r\n", "\n") if ledger.is_file() else ""
        if want != got:
            reasons.append(_first_difference("台帳", want, got))

    for substring in case.stderr_substrings:
        if substring not in stderr_all:
            reasons.append(f"標準エラーに {substring!r} が出ていません")
    return reasons, reproduced


def run_selftest(env: SelftestEnv) -> int:
    """`fixtures-loop/ledger/` の既知ケースを逐語再現する（要件 6.7）。"""
    root = env.fixtures_root
    if not root.is_dir():
        raise env.fail(f"自己較正のケース置き場がありません: {root}")
    cases = [
        _read_case(child, env)
        for child in sorted(root.iterdir())
        if child.is_dir() and not child.name.startswith((".", "_"))
    ]
    if not cases:
        raise env.fail(
            f"自己較正のケースが 1 件もありません: {root}",
            ["ケースの無い自己較正は、何も確かめずに緑を返します。"],
        )

    ok_count = ng_count = 0
    reproduced_all: set[int] = set()
    with tempfile.TemporaryDirectory(prefix="perf-ledger-selftest-") as temporary:
        for case in cases:
            reasons, reproduced = _run_case(case, Path(temporary), env)
            reproduced_all |= reproduced
            if reasons:
                ng_count += 1
                print(f"[selftest] {case.name} NG  {case.title}")
                for reason in reasons:
                    print(f"           - {reason}")
            else:
                ok_count += 1
                print(f"[selftest] {case.name} ok  {case.title}")

    for code in env.required_red_exits:
        if code not in reproduced_all:
            ng_count += 1
            print(f"[selftest] (赤の較正) NG  終了コード {code} を再現したケースが 1 件もありません")

    print(f"SELFTEST RESULT ok={ok_count} ng={ng_count}")
    return env.ok_code if ng_count == 0 else env.fail_code
