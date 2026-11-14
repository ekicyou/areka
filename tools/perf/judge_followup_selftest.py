#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""`judge-followup.py --selftest` の中身（自己較正のハーネス）。

**なぜ別ファイルか**: 本体 `judge-followup.py` が 1,000 行の目安（要件 6.8）に達したため、
合否を決める経路（本番）と、道具そのものを疑う経路（自己較正）を分けた。入口は変わらず
`python tools/perf/judge-followup.py --selftest` である。

**なぜ本体を import しないか**: 本体のファイル名にはハイフンがあり、Python のモジュール名
として import できない。そこで本体が起動時に `run_selftest(env)` へ `SelftestEnv` を渡す
（＝逆向きの import を作らない＝循環しない）。`env.invoke` は本体の `main(argv)` そのもの
であり、**自己較正だけの近道は作らない**（`perf-compare.py`／`perf-ledger.py` と同じ規律）。

緑は道具が壊れていても出る。ゆえに合格を再現するケースと同格に、不合格・判定不能を
再現するケースを置き、そのどちらも実際に走って期待どおりになったことを出力に出す
（`env.required_red_exits` の終了コードを 1 件も再現していなければ自己較正そのものを
不合格にする）。

1 ケース＝1 ディレクトリ（`fixtures-loop/followup/<名前>/`）:

    case.txt              `exit`（期待終了コード・必須）と `title`（説明・任意）・`stderr_substr`（stderr に含まれるはずの英語の語・任意）
    expected_stdout.txt   標準出力と逐語一致する期待本文（必須）
    probe.log / run.log   判定の材料（本体が読む 2 つ）

判定は出力ディレクトリへ `followup-verdict.txt` を書くため、ケースは一時ディレクトリ
（`perf_selftest_common.work_dir`）へ複製してから走らせる（fixture を汚さない）。
"""

from __future__ import annotations

import shutil
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

import perf_selftest_common as common

#: `case.txt` に書ける項目（これ以外は書き間違いとして拒む）。
CASE_KEYS = frozenset({"exit", "title", "stderr_substr"})


@dataclass(frozen=True)
class SelftestEnv:
    """本体から渡される道具立て（本体を import しないための唯一の受け口）。"""

    fixtures_root: Path
    #: 実運用と同じ入口（本体の `main(argv)`）。自己較正だけの近道は作らない。
    invoke: Callable[[list[str]], int]
    #: 期待の書けていないケースを拒むときに投げる例外を作る（本体の `bad_input`）。
    bad_input: Callable[..., Exception]
    #: 1 件も再現していなければ自己較正を不合格にする終了コード。
    required_red_exits: tuple[int, ...]
    case_filename: str
    expected_stdout_filename: str
    ok_exit: int
    fail_exit: int
    inconclusive_exit: int
    #: `main` が SystemExit を非整数で投げたときに読み替える終了コード（本体の引数不正）。
    bad_input_exit: int


@dataclass
class SelftestCase:
    name: str
    directory: Path
    title: str
    expected: int
    stderr_substr: str = ""


def _read_case(env: SelftestEnv, directory: Path) -> SelftestCase:
    path = directory / env.case_filename
    if not path.is_file():
        raise env.bad_input(
            f"ケースに {env.case_filename} がありません: {directory}",
            ["期待終了コードの書いていないケースは、走らせても合否を言えません。"],
        )
    fields, _ = common.read_case_fields(path, CASE_KEYS, frozenset(), env.bad_input)
    if "exit" not in fields:
        raise env.bad_input(f"{path} に exit がありません", ["必須項目: exit（任意: title・stderr_substr）"])
    try:
        expected = int(fields["exit"])
    except ValueError as exc:
        raise env.bad_input(f"{path} の exit が整数ではありません（{fields['exit']!r}）") from exc
    return SelftestCase(directory.name, directory, fields.get("title", "(説明なし)"), expected,
                        fields.get("stderr_substr", ""))


def run_selftest(env: SelftestEnv) -> int:
    directories, _ = common.list_case_dirs(
        env.fixtures_root, env.case_filename, env.bad_input, materials_allowed=False
    )
    cases = [_read_case(env, child) for child in directories]

    results: list[tuple[str, str, list[str]]] = []
    reproduced: set[int] = set()
    with common.work_dir("judge-followup-selftest-") as temporary:
        for case in cases:
            work = temporary / case.name
            shutil.copytree(case.directory, work)
            code, stdout, err = common.invoke(env.invoke, [str(work)], env.bad_input_exit)
            reasons: list[str] = []
            if code == case.expected:
                reproduced.add(code)
            else:
                reasons.append(f"終了コード: 期待 {case.expected} 実際 {code}")
            if case.stderr_substr and case.stderr_substr not in err:
                reasons.append(f"stderr does not contain {case.stderr_substr!r}: {ascii(err)}")
            # .ps1 が実際に読むのは標準出力でなく followup-verdict.txt（#31）。
            verdict_file = work / "followup-verdict.txt"
            if code in (env.ok_exit, env.fail_exit, env.inconclusive_exit) and (
                not verdict_file.is_file() or verdict_file.read_text(encoding="utf-8") != stdout
            ):
                reasons.append("followup-verdict.txt is missing or differs from stdout")
            expected_stdout = case.directory / env.expected_stdout_filename
            if expected_stdout.is_file():
                want = common.read_normalized(expected_stdout)
                got = stdout.replace("\r\n", "\n")
                if want != got:
                    reasons.append(common.first_difference("標準出力", want, got))
            else:
                reasons.append(
                    f"{env.expected_stdout_filename} がありません（本文の逐語再現を確かめられません）"
                )
            results.append((case.name, case.title, reasons))

    # 木全体の見張り（⑶ stderr の ASCII・⑸ README の例とも有効）。
    results.append(("(check_tree)", "木全体の見張り",
                    common.check_tree(ascii_stderr=True, readme_examples=True)))
    ng_count = common.finish(
        results, [], env.case_filename, reproduced, env.required_red_exits
    )
    return env.ok_exit if ng_count == 0 else env.fail_exit
