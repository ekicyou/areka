#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""`perf-compare.py --selftest` の中身（自己較正のハーネス）。

**なぜ別ファイルか**: 本体 `perf-compare.py` が 1,000 行の目安（要件 6.8）に達したため、
採否を決める経路（本番）と、道具そのものを疑う経路（自己較正）を分けた。入口は変わらず
`python tools/perf/perf-compare.py --selftest` である。

**なぜ本体を import しないか**: 本体のファイル名にはハイフンがあり、Python のモジュール名
として import できない。そこで本体が起動時に `run_selftest(env)` へ `SelftestEnv` を渡す
（＝逆向きの import を作らない＝循環しない）。`env.invoke` は本体の `main(argv)` そのもの
であり、**自己較正だけの近道は作らない**（`perf-rank.py`／`perf-ledger.py` と同じ規律）。

1 ケース＝1 ディレクトリ（`fixtures-loop/compare/<名前>/`）:

    case.txt              期待値台帳（下記）。この 1 ファイルを人も機械も読む
    expected_compare.txt  `compare.txt` と逐語一致する期待出力（必須）

`case.txt` に書ける項目:

    title                 ケースの説明（NG 行に出る）
    goal                  目標定義ファイル（ケースからの相対・既定 `goal.toml`）
    a1 a2 b1 b2           走行ディレクトリ（ケースからの相対・必須）
    exit                  期待終了コード（必須）
    verdict               期待採否（必須）
    before_idle_cpu_pct   `compare.json` の同名の鍵と突き合わせる（任意）
    after_idle_cpu_pct    同上
    delta_pct             同上
    noise_pct             同上
    secondary             同上（台帳の `secondary:` に載る 1 行）
    argv                  追加の引数（空白区切り・任意）
    stderr_substr         標準エラーに必ず現れる文字列（何行でも書ける）
    note                  覚え書き（判定には使わない）

数の 4 項目は **書かなければ「null（測れていない）であること」を期待する**——書き忘れと
「空であってほしい」を取り違えないためである。judge-perf.py の fixture と同じく、`case.txt`
に無い項目は黙って通さない（知らない項目は拒む）。

`case.txt` の無いディレクトリは **素材置き場**として飛ばす（`runs/` がそれで、複数のケースが
同じ走行を共有している）。「緑だけの自己較正は較正になっていない」ため、
`env.required_red_exits` に挙げた終了コードを 1 件も再現していなければ自己較正そのものを
不合格にする。
"""

from __future__ import annotations

import json
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

import perf_selftest_common as common

# =============================================================================
# 定数（自己較正だけの較正値。採否の判断には一切関与しない）
# =============================================================================

SELFTEST_CASE_FILENAME = "case.txt"
SELFTEST_EXPECTED_FILENAME = "expected_compare.txt"

#: `case.txt` で使える 1 対 1 の項目（これ以外は書き間違いとして拒む）。
CASE_SINGLE_KEYS = (
    "title", "goal", "exit", "verdict", "a1", "a2", "b1", "b2", "argv",
    "before_idle_cpu_pct", "after_idle_cpu_pct", "delta_pct", "noise_pct", "secondary",
)
#: 無ければ較正にならない項目。
CASE_REQUIRED_KEYS = ("exit", "verdict", "a1", "a2", "b1", "b2")
#: `compare.json` の同名の鍵と突き合わせる項目。
CASE_JSON_KEYS = (
    "verdict", "before_idle_cpu_pct", "after_idle_cpu_pct",
    "delta_pct", "noise_pct", "secondary",
)
#: 書かれていなければ null であることを期待する数の項目。
CASE_NULLABLE_NUMBER_KEYS = (
    "before_idle_cpu_pct", "after_idle_cpu_pct", "delta_pct", "noise_pct",
)
DEFAULT_GOAL_FILENAME = "goal.toml"


@dataclass(frozen=True)
class SelftestEnv:
    """本体から渡される道具立て（本体を import しないための唯一の受け口）。"""

    fixtures_root: Path
    #: 実運用と同じ入口（本体の `main(argv)`）。
    invoke: Callable[[list[str]], int]
    #: 期待の書けていないケースを拒むときに投げる例外を作る（本体の `bad_input`）。
    bad_input: Callable[..., Exception]
    #: 1 件も再現していなければ自己較正を不合格にする終了コード。
    required_red_exits: tuple[int, ...]
    text_filename: str
    json_filename: str
    #: 「測っていない」の綴り（本体の `METRIC_UNAVAILABLE`）。
    unavailable: str
    #: 数を突き合わせる桁（本体の `PCT_DECIMALS`）。
    pct_decimals: int
    ok_exit: int
    fail_exit: int


@dataclass
class SelftestCase:
    name: str
    directory: Path
    title: str
    expected_exit: int
    values: dict[str, str]
    argv: list[str]
    stderr_substrings: list[str]


def _read_case(env: SelftestEnv, directory: Path) -> SelftestCase:
    """ケース 1 件の台帳（case.txt）を読む。知らない項目は拒む（黙って無視しない）。"""
    case_path = directory / SELFTEST_CASE_FILENAME
    values, multi = common.read_case_fields(
        case_path, frozenset(CASE_SINGLE_KEYS) | {"note"}, frozenset({"stderr_substr"}),
        env.bad_input,
    )
    values.pop("note", None)
    for required in CASE_REQUIRED_KEYS:
        if required not in values:
            raise env.bad_input(f"{case_path}: {required} がありません")
    try:
        expected_exit = int(values["exit"])
    except ValueError as exc:
        raise env.bad_input(
            f"{case_path}: exit が整数ではありません（{values['exit']!r}）"
        ) from exc
    return SelftestCase(
        directory.name, directory, values.get("title") or "(説明なし)", expected_exit,
        values, values.get("argv", "").split(), multi["stderr_substr"],
    )


def _matches(env: SelftestEnv, expected: str, actual) -> bool:
    """`case.txt` の綴りと `compare.json` の値を突き合わせる（数は桁を揃えて比べる）。"""
    if actual is None:
        return expected == env.unavailable
    if isinstance(actual, (int, float)) and not isinstance(actual, bool):
        try:
            return abs(float(expected) - float(actual)) < 10 ** -env.pct_decimals / 2
        except ValueError:
            return False
    return expected == str(actual)


def _check_json(env: SelftestEnv, case: SelftestCase, work: Path) -> list[str]:
    json_path = work / env.json_filename
    if not json_path.is_file():
        return [f"{env.json_filename} が作られていません"]
    payload = json.loads(json_path.read_text(encoding="utf-8"))
    reasons: list[str] = []
    for key in CASE_JSON_KEYS:
        if key in case.values and not _matches(env, case.values[key], payload.get(key)):
            reasons.append(
                f"{env.json_filename} の {key}: "
                f"期待 {case.values[key]!r} 実際 {payload.get(key)!r}"
            )
    for key in CASE_NULLABLE_NUMBER_KEYS:
        if key not in case.values and payload.get(key) is not None:
            reasons.append(
                f"{env.json_filename} の {key}: 期待 null（測れていない側）"
                f" 実際 {payload.get(key)!r}"
            )
    return reasons


def _run_case(env: SelftestEnv, case: SelftestCase,
              workspace: Path) -> tuple[list[str], set[int]]:
    work = workspace / case.name
    work.mkdir(parents=True)
    reasons: list[str] = []
    reproduced: set[int] = set()

    def resolve(name: str) -> str:
        return str((case.directory / case.values[name]).resolve())

    goal = case.values.get("goal", DEFAULT_GOAL_FILENAME)
    argv = [
        "--a", resolve("a1"), resolve("a2"),
        "--b", resolve("b1"), resolve("b2"),
        "--goal-file", str((case.directory / goal).resolve()),
        "--out-dir", str(work),
    ] + case.argv

    code, _out, err = common.invoke(env.invoke, argv, env.fail_exit)
    if code == case.expected_exit:
        reproduced.add(code)
    else:
        reasons.append(f"終了コード: 期待 {case.expected_exit} 実際 {code}")

    reasons += _check_json(env, case, work)

    expected_path = case.directory / SELFTEST_EXPECTED_FILENAME
    produced = work / env.text_filename
    if not expected_path.is_file():
        reasons.append(
            f"{SELFTEST_EXPECTED_FILENAME} がありません"
            "（期待出力の無いケースは較正になりません）"
        )
    elif not produced.is_file():
        reasons.append(f"{env.text_filename} が作られていません")
    else:
        want = common.read_normalized(expected_path)
        got = common.read_normalized(produced)
        if want != got:
            reasons.append(common.first_difference(env.text_filename, want, got))

    for substring in case.stderr_substrings:
        if substring not in err:
            reasons.append(f"標準エラーに {substring!r} が出ていません")
    return reasons, reproduced


def run_selftest(env: SelftestEnv) -> int:
    """fixtures-loop/compare/ の既知ケースを逐語再現する（要件 6.7・3.6）。"""
    directories, materials = common.list_case_dirs(
        env.fixtures_root, SELFTEST_CASE_FILENAME, env.bad_input, materials_allowed=True
    )
    cases = [_read_case(env, child) for child in directories]

    results: list[tuple[str, str, list[str]]] = []
    reproduced_all: set[int] = set()
    with common.work_dir("perf-compare-selftest-") as temporary:
        for case in cases:
            reasons, reproduced = _run_case(env, case, temporary)
            reproduced_all |= reproduced
            results.append((case.name, case.title, reasons))

    ng_count = common.finish(
        results, materials, SELFTEST_CASE_FILENAME, reproduced_all, env.required_red_exits
    )
    return env.ok_exit if ng_count == 0 else env.fail_exit
