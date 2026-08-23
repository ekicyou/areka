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

    case.txt              `exit`（期待終了コード・必須）と `title`（説明・任意）
    expected_stdout.txt   標準出力と逐語一致する期待本文（必須）
    probe.log / run.log   判定の材料（本体が読む 2 つ）

判定は出力ディレクトリへ `followup-verdict.txt` を書くため、ケースは一時ディレクトリへ
複製してから走らせる（fixture を汚さない）。
"""

from __future__ import annotations

import contextlib
import io
import shutil
import tempfile
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path


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
    #: `main` が SystemExit を非整数で投げたときに読み替える終了コード（本体の引数不正）。
    bad_input_exit: int


@dataclass
class SelftestCase:
    name: str
    directory: Path
    title: str
    expected: int


def _read_case(env: SelftestEnv, directory: Path) -> SelftestCase:
    path = directory / env.case_filename
    if not path.is_file():
        raise env.bad_input(
            f"ケースに {env.case_filename} がありません: {directory}",
            ["期待終了コードの書いていないケースは、走らせても合否を言えません。"],
        )
    fields: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#") or "=" not in stripped:
            continue
        key, _, value = stripped.partition("=")
        fields[key.strip()] = value.strip()
    if "exit" not in fields:
        raise env.bad_input(f"{path} に exit がありません", ["必須項目: exit（任意: title）"])
    try:
        expected = int(fields["exit"])
    except ValueError as exc:
        raise env.bad_input(f"{path} の exit が整数ではありません（{fields['exit']!r}）") from exc
    return SelftestCase(directory.name, directory, fields.get("title", "(説明なし)"), expected)


def _invoke(env: SelftestEnv, argv: list[str]) -> tuple[int, str]:
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        try:
            code = env.invoke(argv)
        except SystemExit as exc:
            code = exc.code if isinstance(exc.code, int) else env.bad_input_exit
    return code, out.getvalue()


def _first_difference(want: str, got: str) -> str:
    want_lines, got_lines = want.splitlines(), got.splitlines()
    for index in range(max(len(want_lines), len(got_lines))):
        a = want_lines[index] if index < len(want_lines) else "(行が無い)"
        b = got_lines[index] if index < len(got_lines) else "(行が無い)"
        if a != b:
            return f"標準出力 {index + 1} 行目: 期待 {a!r} 実際 {b!r}"
    return "差はありません"


def run_selftest(env: SelftestEnv) -> int:
    root = env.fixtures_root
    if not root.is_dir():
        raise env.bad_input(
            f"自己較正のケース置き場がありません: {root}",
            ["判定スクリプトと同じ場所の fixtures-loop/followup/ を見ます（相対位置のみ）。"],
        )
    cases = [
        _read_case(env, child)
        for child in sorted(root.iterdir())
        if child.is_dir() and not child.name.startswith((".", "_"))
    ]
    if not cases:
        raise env.bad_input(
            f"自己較正のケースが 1 件もありません: {root}",
            ["ケースの無い自己較正は、何も確かめずに緑を返します。"],
        )

    ok_count = ng_count = 0
    reproduced: set[int] = set()
    with tempfile.TemporaryDirectory(prefix="judge-followup-selftest-") as temporary:
        for case in cases:
            work = Path(temporary) / case.name
            shutil.copytree(case.directory, work)
            code, stdout = _invoke(env, [str(work)])
            reasons: list[str] = []
            if code == case.expected:
                reproduced.add(code)
            else:
                reasons.append(f"終了コード: 期待 {case.expected} 実際 {code}")
            expected_stdout = case.directory / env.expected_stdout_filename
            if expected_stdout.is_file():
                want = expected_stdout.read_text(encoding="utf-8").replace("\r\n", "\n")
                got = stdout.replace("\r\n", "\n")
                if want != got:
                    reasons.append(_first_difference(want, got))
            else:
                reasons.append(
                    f"{env.expected_stdout_filename} がありません（本文の逐語再現を確かめられません）"
                )
            if reasons:
                ng_count += 1
                print(f"[selftest] {case.name} NG  {case.title}")
                for reason in reasons:
                    print(f"           - {reason}")
            else:
                ok_count += 1
                print(f"[selftest] {case.name} ok  {case.title}")

    for code in env.required_red_exits:
        if code not in reproduced:
            ng_count += 1
            print(f"[selftest] (赤の較正) NG  終了コード {code} を再現したケースが 1 件もありません")

    print(f"SELFTEST RESULT ok={ok_count} ng={ng_count}")
    return env.ok_exit if ng_count == 0 else env.fail_exit
