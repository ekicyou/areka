#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""areka 自走改善ループの台帳（loop-ledger.md）を書き・読みする道具。

台帳はループの**唯一の記憶**である（要件 1.3・1.10）。ループの手順は会話の記憶に頼らず、
毎ターン台帳から現在地（周番号・相・待っている走行）を読み直して続きを回す。だから台帳は
人が読める Markdown でありながら、機械が一意に解せる厳格な形でなければならない。

台帳の形——先頭に状態ブロック（機械が書き換える固定キー行）、以後に周の記録を追記する:

    ## 状態
    - goal: draw-load-parity
    - iteration: 3
    - phase: REMEASURE
    - pending_run: C:/…/iter-3
    - streak_no_gain: 1
    - best_idle_cpu_pct: 6.12
    - baseline_idle_cpu_pct: 9.31
    - started_at: 2026-08-23T00:00:00Z
    - run: -
    - capabilities: -

    ## 周 1 — 2026-08-23T01:23:45Z
    - hypothesis: 変化が無い tick で 13 本を回さない（tick gate）
    …（以下、固定の順で 16 個の鍵行。値の無い鍵は `-` と書く）

* 鍵の語彙は `STATE_KEYS`／`ENTRY_KEYS` が唯一の所在。語彙にない鍵は読むときも書くときも
  拒む（黙って受け入れると、次に読む者が「書いたはずの値が無い」ことに気づけない）。
* `run`（走行固有の 8 桁トークン）と `capabilities`（PREFLIGHT で測った道具の可否）は
  設計 C1・C2 が状態ブロックに置くと決めた鍵で、`init` は `-` で作る。
* 値に空白・コロン・読点を含んでよい（鍵と値は最初の `: ` だけで分ける）。改行は不可。

サブコマンド:
    init      台帳を新しく作る（既にあれば拒む。作り直すなら --force）
    state     状態ブロックを JSON で出す（`-` は null・数の鍵は数として出す）
    set-phase 状態ブロックだけを書き換える（周の記録は 1 バイトも動かさない）
    append    周の記録を 1 つ追記する（--from-json で値を渡す）
    read      指定した周の記録を JSON で出す（append へそのまま戻せる形）
    entries   全ての周の記録を JSON 配列で出す（周番号と日時つき）
    --selftest 自己較正（fixtures-loop/ledger/ の既知ケースを逐語再現する）

台帳の在り処は次の順で解く: `--ledger <path>` が最優先。無ければ目標定義ファイル
（`--goal-file <toml>`、または `--goal <名前>` → `tools/perf/goals/<名前>.toml`）の
`[goal].spec_dir` と `[goal].ledger` を繋いだ位置（リポジトリ根＝`tools/perf` の親の親・
`--repo-root` で上書き可）。どれも無ければ exit 3。

    python tools/perf/perf-ledger.py state --goal draw-load-parity
    python tools/perf/perf-ledger.py append --goal draw-load-parity --from-json iter.json
    python tools/perf/perf-ledger.py --selftest

終了コード（judge-perf.py と同じ体系）: 0=正常（--selftest は全ケース一致）／
1=自己較正の食い違い＝道具が壊れている／3=引数不正・台帳の破損・語彙にない鍵や相や採否・
目標定義ファイルが無い／4=読み書きの失敗。**2（判定不能）は返さない**——台帳は読めたか
読めなかったかのどちらかで、途中まで読めた台帳を部分的に信じることはしない（要件 2.11）。

task 5.5 で足すもの: `status`（STATUS 行）・`final`（FINAL 行・`run=` トークン込み）・
`next-phase`（相の遷移表・純関数）・`goal-check`（目標定義の検査とトークン生成）・
`goal-text`（`/goal` 条件文）・`summary`（results/summary.md）。呼ばれたら黙らずに
「まだ無い」と述べて exit 3 する（`PLANNED_SUBCOMMANDS`）。足すときは定数節と
`SUBCOMMANDS` に 1 行ずつ加えればよく、読み書き（`parse_ledger`／`replace_state_block`）
には手を入れない。行数が 1,000 行に近づいたら、この節を別ファイルへ分けること。
"""

from __future__ import annotations

import argparse
import contextlib
import io
import json
import os
import re
import shutil
import sys
import tempfile
import tomllib
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path

# =============================================================================
# 定数（値の所在はすべてここ 1 箇所。task 5.5 はこの節へ足す）
# =============================================================================

#: 本スクリプトの版（台帳の書式か語彙を変えたら上げる）。
SCRIPT_VERSION = "0.1.0 (task 5.4 / 状態ブロック・追記・読取・自己較正)"

EXIT_OK, EXIT_FAIL, EXIT_BAD_INPUT, EXIT_IO_ERROR = 0, 1, 3, 4

#: 台帳の書式（見出し・鍵行・空値）。
STATE_HEADING = "## 状態"
ENTRY_HEADING_FORMAT = "## 周 {iteration} — {timestamp}"
ENTRY_HEADING_RE = re.compile(r"^## 周 (\d+) — (.+)$")
KEY_LINE_RE = re.compile(r"^- ([a-z][a-z0-9_]*): (.+)$")
KEY_LINE_FORMAT = "- {key}: {value}"
EMPTY_VALUE = "-"

#: 状態ブロックの鍵（この順で書く。`init` が作るのもこの 10 行）。
#: `run`・`capabilities` は後から書かれる鍵なので、無い台帳も読める（`-` を補う）。
STATE_KEYS = (
    "goal", "iteration", "phase", "pending_run", "streak_no_gain",
    "best_idle_cpu_pct", "baseline_idle_cpu_pct", "started_at", "run", "capabilities",
)
STATE_LATE_KEYS = ("run", "capabilities")
STATE_INT_KEYS = ("iteration", "streak_no_gain")
STATE_FLOAT_KEYS = ("best_idle_cpu_pct", "baseline_idle_cpu_pct")

#: 周の記録の鍵（この順で書く。設計 C11 の見本と同じ順・同じ語）。
ENTRY_KEYS = (
    "hypothesis", "candidate", "files_changed", "runs",
    "before_idle_cpu_pct", "after_idle_cpu_pct", "delta_pct", "noise_pct",
    "secondary", "tests", "followup", "verdict", "commit",
    "skipped_candidates", "duration_min", "reason",
)
ENTRY_INT_KEYS = ("duration_min",)
ENTRY_FLOAT_KEYS = ("before_idle_cpu_pct", "after_idle_cpu_pct", "delta_pct", "noise_pct")
#: 小数は 2 桁に揃えて記録する（STATUS 行の `<x.xx>` と同じ精度）。
FLOAT_FORMAT = "{:.2f}"

#: 相の語彙（設計 C2 の遷移表）。背景コマンドの完了待ちは `WAIT_` を冠して表す。
PHASES = (
    "PREFLIGHT", "BASELINE", "RANK", "SELECT", "IMPLEMENT", "TEST",
    "REMEASURE", "DECIDE", "RECORD", "TOOLFIX", "FINAL",
)
WAIT_PHASE_PREFIX = "WAIT_"
INITIAL_PHASE = "PREFLIGHT"

#: 採否の語彙（設計 C10 の判定・STATUS 行の `verdict=`）。
VERDICTS = ("ADOPTED", "NO_DIFF", "WORSE", "TESTS_RED", "FOLLOWUP_FAIL", "MEASURE_FAILED", "NA")

#: 目標定義ファイル（リポジトリ根からの相対）。
GOALS_RELATIVE_DIR = ("tools", "perf", "goals")
GOAL_TABLE = "goal"
GOAL_REQUIRED_KEYS = ("spec_dir", "ledger")

#: 自己較正の較正値（台帳の値の判断には一切関与しないため、所在はここ 1 箇所）。
SELFTEST_FIXTURES_DIRNAME = "fixtures-loop"
SELFTEST_LEDGER_SUBDIR = "ledger"
SELFTEST_CASE_FILENAME = "case.txt"
SELFTEST_STEPS_FILENAME = "steps.txt"
SELFTEST_LEDGER_FILENAME = "ledger.md"
SELFTEST_EXPECTED_LEDGER = "expected_ledger.md"
SELFTEST_EXPECTED_STDOUT = "expected_stdout.txt"
SELFTEST_STEP_SEPARATOR = " :: "
SELFTEST_LEDGER_PLACEHOLDER = "@LEDGER@"
SELFTEST_DIR_PLACEHOLDER = "@DIR@"
#: 「緑だけの自己較正は較正になっていない」（D7）。ここに挙げた終了コードを期待する
#: ケースが 1 つも再現していなければ、自己較正そのものを不合格にする。
SELFTEST_REQUIRED_RED_EXITS = (EXIT_BAD_INPUT,)

#: 標準出力は機械が読む値（JSON）だけに使い、人向けの報告は標準エラーへ出す。
STDERR_PREFIX = "[perf-ledger]"


# =============================================================================
# エラー（黙って続く経路は作らない・要件 2.11）
# =============================================================================


class LedgerError(Exception):
    def __init__(self, code: int, message: str, details: tuple[str, ...] | list[str] = ()):
        super().__init__(message)
        self.code = code
        self.message = message
        self.details = list(details)


def bad_input(message: str, details: tuple[str, ...] | list[str] = ()) -> LedgerError:
    return LedgerError(EXIT_BAD_INPUT, message, details)


def io_failure(message: str, details: tuple[str, ...] | list[str] = ()) -> LedgerError:
    return LedgerError(EXIT_IO_ERROR, message, details)


# =============================================================================
# 台帳の読み書き
# =============================================================================


@dataclass
class Entry:
    """周の記録 1 つ（値は台帳に書かれたままの文字列。`-` は「値なし」）。"""

    iteration: int
    timestamp: str
    values: dict[str, str] = field(default_factory=dict)


@dataclass
class Ledger:
    """台帳 1 ファイル。`lines` と `state_span` は状態ブロックの差し替えに使う。"""

    path: Path
    state: dict[str, str]
    entries: list[Entry]
    lines: list[str]
    state_span: tuple[int, int]

    def entry(self, iteration: int) -> Entry:
        for item in self.entries:
            if item.iteration == iteration:
                return item
        have = "・".join(str(e.iteration) for e in self.entries) or "（1 件も無し）"
        raise bad_input(f"周 {iteration} の記録が台帳にありません: {self.path}", [f"台帳にあるのは {have} です。"])


def read_text(path: Path) -> str:
    if not path.is_file():
        raise bad_input(f"台帳がありません: {path}", ["先に `init` で作ってください（周 0 の手順）。"])
    try:
        return path.read_text(encoding="utf-8")
    except OSError as exc:
        raise io_failure(f"台帳を読めません: {path}", [str(exc)]) from exc
    except UnicodeDecodeError as exc:
        raise bad_input(f"台帳が UTF-8 ではありません: {path}", [str(exc)]) from exc


def write_text(path: Path, text: str) -> None:
    """一時ファイルへ書いてから差し替える（途中で落ちても台帳を壊さない）。"""
    temporary = path.with_name(path.name + ".tmp")
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        temporary.write_text(text, encoding="utf-8", newline="\n")
        os.replace(temporary, path)
    except OSError as exc:
        with contextlib.suppress(OSError):
            temporary.unlink()
        raise io_failure(f"台帳を書けません: {path}", [str(exc)]) from exc


def _corrupt(path: Path, lineno: int, line: str, why: str) -> LedgerError:
    return bad_input(
        f"台帳に壊れた行があります: {why}",
        [
            f"{path}:{lineno}: {line!r}",
            "台帳の行は `## 状態`・`## 周 <n> — <日時>`・`- <鍵>: <値>`・空行のみです。"
            "読める行だけで続けると、書いたはずの値が黙って消えます（要件 2.11）。",
        ],
    )


def parse_ledger(text: str, path: Path) -> Ledger:
    """台帳を厳格に読む。1 行でも解せなければ exit 3（部分的に信じない）。"""
    lines = text.replace("\r\n", "\n").replace("\r", "\n").split("\n")

    index = 0
    while index < len(lines) and not lines[index].strip():
        index += 1
    if index >= len(lines) or lines[index] != STATE_HEADING:
        head = lines[index] if index < len(lines) else ""
        raise _corrupt(path, index + 1, head, f"台帳の先頭は {STATE_HEADING!r} でなければなりません")

    state_start = index
    index += 1
    state: dict[str, str] = {}
    while index < len(lines):
        match = KEY_LINE_RE.match(lines[index])
        if not match:
            break
        key, value = match.group(1), match.group(2)
        if key not in STATE_KEYS:
            raise _corrupt(path, index + 1, lines[index], f"状態ブロックの知らないキー {key!r} です")
        if key in state:
            raise _corrupt(path, index + 1, lines[index], f"状態ブロックのキー {key!r} が 2 度出ています")
        state[key] = value
        index += 1
    state_end = index

    entries: list[Entry] = []
    current: Entry | None = None
    while index < len(lines):
        line = lines[index]
        index += 1
        if not line.strip():
            continue
        heading = ENTRY_HEADING_RE.match(line)
        if heading:
            iteration = int(heading.group(1))
            if any(e.iteration == iteration for e in entries):
                raise _corrupt(path, index, line, f"周 {iteration} が 2 度出ています")
            current = Entry(iteration=iteration, timestamp=heading.group(2))
            entries.append(current)
            continue
        match = KEY_LINE_RE.match(line)
        if not match:
            raise _corrupt(path, index, line, "見出しでも鍵行でも空行でもありません")
        if current is None:
            raise _corrupt(path, index, line, "状態ブロックの外に鍵行があります")
        key, value = match.group(1), match.group(2)
        if key not in ENTRY_KEYS:
            raise _corrupt(path, index, line, f"周の記録の知らないキー {key!r} です")
        if key in current.values:
            raise _corrupt(path, index, line, f"周 {current.iteration} のキー {key!r} が 2 度出ています")
        current.values[key] = value

    required = [key for key in STATE_KEYS if key not in STATE_LATE_KEYS]
    missing = [key for key in required if key not in state]
    if missing:
        raise bad_input(
            f"状態ブロックに必須のキーがありません: {'・'.join(missing)}",
            [f"{path}", f"必須は {'・'.join(required)} です。"],
        )
    for key in STATE_KEYS:
        state.setdefault(key, EMPTY_VALUE)

    ledger = Ledger(path, state, entries, lines, (state_start, state_end))
    state_to_json(ledger)  # 数として読む鍵が数であることを、読んだ時点で確かめる
    return ledger


def state_lines(state: dict[str, str]) -> list[str]:
    return [STATE_HEADING] + [
        KEY_LINE_FORMAT.format(key=key, value=state.get(key, EMPTY_VALUE)) for key in STATE_KEYS
    ]


def entry_lines(iteration: int, timestamp: str, values: dict[str, str]) -> list[str]:
    return [ENTRY_HEADING_FORMAT.format(iteration=iteration, timestamp=timestamp)] + [
        KEY_LINE_FORMAT.format(key=key, value=values.get(key, EMPTY_VALUE)) for key in ENTRY_KEYS
    ]


def replace_state_block(ledger: Ledger, state: dict[str, str]) -> str:
    """状態ブロックの行だけを差し替える。周の記録は 1 バイトも触らない。"""
    start, end = ledger.state_span
    return "\n".join(ledger.lines[:start] + state_lines(state) + ledger.lines[end:])


def append_entry_text(body: str, iteration: int, timestamp: str, values: dict[str, str]) -> str:
    """末尾へ周の記録を 1 つ足す（既存の本文はそのまま・末尾の空行だけ整える）。"""
    return body.rstrip("\n") + "\n\n" + "\n".join(entry_lines(iteration, timestamp, values)) + "\n"


# =============================================================================
# 値の型（台帳の文字列 ⇄ JSON の値）。型は鍵の名前で決まる
# =============================================================================


def _to_json_value(key: str, raw: str, int_keys: tuple[str, ...], float_keys: tuple[str, ...], path: Path):
    if raw == EMPTY_VALUE:
        return None
    if key in int_keys or key in float_keys:
        caster, what = (int, "整数") if key in int_keys else (float, "数")
        try:
            return caster(raw)
        except ValueError as exc:
            raise bad_input(f"{key} は{what}でなければなりません（{raw!r}）", [str(path)]) from exc
    return raw


def _to_ledger_value(key: str, value, int_keys: tuple[str, ...], float_keys: tuple[str, ...]) -> str:
    if value is None:
        return EMPTY_VALUE
    if isinstance(value, bool):
        raise bad_input(f"{key} に真偽値は書けません（{value!r}）", ["値は文字列か数です。"])
    if key in int_keys or key in float_keys:
        integral = key in int_keys
        try:
            number = int(str(value).strip()) if integral else float(str(value).strip())
        except ValueError as exc:
            what = "整数" if integral else "数"
            raise bad_input(f"{key} は{what}でなければなりません（{value!r}）") from exc
        return str(number) if integral else FLOAT_FORMAT.format(number)
    if not isinstance(value, (str, int, float)):
        raise bad_input(f"{key} に書けない型です（{type(value).__name__}）", ["値は文字列か数です。"])
    text = str(value)
    if "\n" in text or "\r" in text:
        raise bad_input(f"{key} の値に改行は入れられません", ["1 つの値は 1 行で書きます。"])
    return text.strip() or EMPTY_VALUE


def state_to_json(ledger: Ledger) -> dict:
    return {
        key: _to_json_value(key, ledger.state[key], STATE_INT_KEYS, STATE_FLOAT_KEYS, ledger.path)
        for key in STATE_KEYS
    }


def entry_to_json(entry: Entry, path: Path) -> dict:
    return {
        key: _to_json_value(key, entry.values.get(key, EMPTY_VALUE), ENTRY_INT_KEYS, ENTRY_FLOAT_KEYS, path)
        for key in ENTRY_KEYS
    }


def emit_json(payload) -> None:
    print(json.dumps(payload, ensure_ascii=False, indent=2))


def report(message: str) -> None:
    sys.stderr.write(f"{STDERR_PREFIX} {message}\n")


# =============================================================================
# 語彙の検査
# =============================================================================


def validate_phase(phase: str) -> str:
    base = phase[len(WAIT_PHASE_PREFIX):] if phase.startswith(WAIT_PHASE_PREFIX) else phase
    if base not in PHASES:
        raise bad_input(
            f"知らない相です: {phase}",
            [f"使える相は {'・'.join(PHASES)}（背景待ちは {WAIT_PHASE_PREFIX} を冠する）です。"],
        )
    return phase


def validate_verdict(verdict: str) -> str:
    if verdict != EMPTY_VALUE and verdict not in VERDICTS:
        raise bad_input(f"知らない採否です: {verdict}", [f"使える採否は {'・'.join(VERDICTS)} です。"])
    return verdict


def validate_timestamp(value: str) -> str:
    """ISO 8601（末尾 Z 可）であることだけ確かめ、字面はそのまま台帳へ書く。"""
    probe = value[:-1] + "+00:00" if value.endswith("Z") else value
    try:
        datetime.fromisoformat(probe)
    except ValueError as exc:
        raise bad_input(f"日時が ISO 8601 ではありません: {value!r}", ["例: 2026-08-23T01:23:45Z"]) from exc
    return value


def now_iso() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


# =============================================================================
# 台帳の在り処
# =============================================================================


def script_dir() -> Path:
    return Path(__file__).resolve().parent


def repo_root(args) -> Path:
    root = getattr(args, "repo_root", None)
    return Path(root).resolve() if root else script_dir().parent.parent


def load_goal_toml(path: Path) -> dict:
    if not path.is_file():
        raise bad_input(
            f"目標定義ファイルがありません: {path}",
            [
                f"`--goal <名前>` は {'/'.join(GOALS_RELATIVE_DIR)}/<名前>.toml を見ます（task 7.1 が置く）。",
                "位置を直に指すなら `--goal-file <toml>`、台帳を直に指すなら `--ledger <path>` です。",
            ],
        )
    try:
        with path.open("rb") as handle:
            return tomllib.load(handle)
    except OSError as exc:
        raise io_failure(f"目標定義ファイルを読めません: {path}", [str(exc)]) from exc
    except tomllib.TOMLDecodeError as exc:
        raise bad_input(f"目標定義ファイルが TOML として読めません: {path}", [str(exc)]) from exc


def goal_toml_path(args) -> Path | None:
    if getattr(args, "goal_file", None):
        return Path(args.goal_file)
    if getattr(args, "goal", None):
        return repo_root(args).joinpath(*GOALS_RELATIVE_DIR, f"{args.goal}.toml")
    return None


def resolve_ledger(args) -> Path:
    if getattr(args, "ledger", None):
        return Path(args.ledger)
    toml_path = goal_toml_path(args)
    if toml_path is None:
        raise bad_input(
            "台帳の在り処が分かりません",
            ["`--ledger <path>`・`--goal <名前>`・`--goal-file <toml>` のいずれかを指定してください。"],
        )
    table = load_goal_toml(toml_path).get(GOAL_TABLE) or {}
    missing = [key for key in GOAL_REQUIRED_KEYS if not table.get(key)]
    if missing:
        raise bad_input(
            f"目標定義ファイルの [{GOAL_TABLE}] に {'・'.join(missing)} がありません: {toml_path}",
            [f"必須は {'・'.join(GOAL_REQUIRED_KEYS)}（どちらもリポジトリ根からの相対）です。"],
        )
    return repo_root(args) / table["spec_dir"] / table["ledger"]


def resolve_goal_name(args) -> str:
    if getattr(args, "goal_name", None):
        return args.goal_name
    toml_path = goal_toml_path(args)
    if toml_path is not None:
        name = (load_goal_toml(toml_path).get(GOAL_TABLE) or {}).get("name")
        if name:
            return str(name)
    if getattr(args, "goal", None):
        return str(args.goal)
    raise bad_input(
        "目標の名前が分かりません",
        ["`--goal-name <名前>`、または [goal].name のある目標定義ファイルを渡してください。"],
    )


def load_ledger(args) -> Ledger:
    path = resolve_ledger(args)
    return parse_ledger(read_text(path), path)


# =============================================================================
# サブコマンド
# =============================================================================


def cmd_init(args) -> int:
    path = resolve_ledger(args)
    if path.exists() and not args.force:
        raise bad_input(
            f"台帳が既にあります: {path}",
            [
                "黙って上書きすると、それまでの周の記録（＝ループの唯一の記憶）が消えます。",
                "作り直してよいと分かっているときだけ --force を付けてください。",
            ],
        )
    state = {
        "goal": resolve_goal_name(args),
        "iteration": "0",
        "phase": INITIAL_PHASE,
        "pending_run": EMPTY_VALUE,
        "streak_no_gain": "0",
        "best_idle_cpu_pct": EMPTY_VALUE,
        "baseline_idle_cpu_pct": EMPTY_VALUE,
        "started_at": validate_timestamp(args.now) if args.now else now_iso(),
        "run": EMPTY_VALUE,
        "capabilities": EMPTY_VALUE,
    }
    write_text(path, "\n".join(state_lines(state)) + "\n")
    report(f"台帳を作りました: {path}")
    return EXIT_OK


def cmd_state(args) -> int:
    emit_json(state_to_json(load_ledger(args)))
    return EXIT_OK


def cmd_set_phase(args) -> int:
    ledger = load_ledger(args)
    state = dict(ledger.state)
    state["phase"] = validate_phase(args.phase)
    # 値の型は鍵の名前で決まる。`-` を渡したときだけは「値を消す」の意味に採る。
    updates = (
        ("iteration", args.iteration),
        ("streak_no_gain", args.streak),
        ("pending_run", args.pending_run),
        ("best_idle_cpu_pct", args.best),
        ("baseline_idle_cpu_pct", args.baseline),
        ("run", args.run),
        ("capabilities", args.capabilities),
    )
    for key, value in updates:
        if value is None:
            continue
        state[key] = (
            EMPTY_VALUE
            if value == EMPTY_VALUE
            else _to_ledger_value(key, value, STATE_INT_KEYS, STATE_FLOAT_KEYS)
        )
    write_text(ledger.path, replace_state_block(ledger, state))
    report(f"状態を更新しました: phase={state['phase']} iteration={state['iteration']}")
    return EXIT_OK


def load_entry_json(path: Path) -> dict[str, str]:
    if not path.is_file():
        raise bad_input(f"追記する値のファイルがありません: {path}")
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise io_failure(f"{path} を読めません", [str(exc)]) from exc
    except (json.JSONDecodeError, UnicodeDecodeError) as exc:
        raise bad_input(f"{path} を JSON として読めません", [str(exc)]) from exc
    if not isinstance(payload, dict):
        raise bad_input(f"{path} の中身が JSON のオブジェクトではありません", ["鍵と値の対で書きます。"])
    unknown = [key for key in payload if key not in ENTRY_KEYS]
    if unknown:
        raise bad_input(
            f"周の記録に知らないキーがあります: {'・'.join(sorted(unknown))}",
            [
                f"{path}",
                f"使えるキーは {'・'.join(ENTRY_KEYS)} です"
                "（語彙は固定・周番号と日時は引数で渡します）。",
            ],
        )
    values = {
        key: _to_ledger_value(key, payload.get(key), ENTRY_INT_KEYS, ENTRY_FLOAT_KEYS)
        for key in ENTRY_KEYS
    }
    validate_verdict(values["verdict"])
    return values


def cmd_append(args) -> int:
    ledger = load_ledger(args)
    values = load_entry_json(Path(args.from_json))
    timestamp = validate_timestamp(args.now) if args.now else now_iso()
    iteration = args.iteration if args.iteration is not None else int(ledger.state["iteration"]) + 1
    if iteration < 1:
        raise bad_input(f"周番号は 1 以上です（{iteration}）")
    if any(entry.iteration == iteration for entry in ledger.entries):
        raise bad_input(
            f"周 {iteration} は既に台帳にあります: {ledger.path}",
            ["同じ周を 2 度書くと、どちらが本当か後から決められません。"],
        )
    state = dict(ledger.state)
    state["iteration"] = str(iteration)
    body = replace_state_block(ledger, state)
    write_text(ledger.path, append_entry_text(body, iteration, timestamp, values))
    report(f"周 {iteration} を追記しました: {ledger.path}")
    return EXIT_OK


def cmd_read(args) -> int:
    ledger = load_ledger(args)
    emit_json(entry_to_json(ledger.entry(args.iteration), ledger.path))
    return EXIT_OK


def cmd_entries(args) -> int:
    ledger = load_ledger(args)
    emit_json(
        [
            {"iteration": e.iteration, "timestamp": e.timestamp, **entry_to_json(e, ledger.path)}
            for e in ledger.entries
        ]
    )
    return EXIT_OK


def cmd_planned(args) -> int:
    report(f"{args.subcommand} は task 5.5 で実装します（{PLANNED_SUBCOMMANDS[args.subcommand]}）。")
    return EXIT_BAD_INPUT


#: 実装済みのサブコマンド（task 5.5 はここへ 1 行ずつ足す）。
SUBCOMMANDS = {
    "init": cmd_init,
    "state": cmd_state,
    "set-phase": cmd_set_phase,
    "append": cmd_append,
    "read": cmd_read,
    "entries": cmd_entries,
}

#: まだ無いサブコマンド（求められたら黙らずに理由を述べて終わる）。
PLANNED_SUBCOMMANDS = {
    "status": "STATUS 行 1 本",
    "final": "FINAL 行（run= トークン込み）",
    "next-phase": "相の遷移表（純関数）",
    "goal-check": "目標定義ファイルの検査とトークン生成",
    "goal-text": "トークンを埋めた /goal 条件文",
    "summary": "results/summary.md（旧数値との対比表）",
}


# =============================================================================
# 自己較正（--selftest）
# =============================================================================


@dataclass
class SelftestCase:
    """1 ケース＝1 ディレクトリ。`steps.txt` の各行を実運用と同じ入口へ流す。"""

    name: str
    directory: Path
    title: str
    stderr_substrings: list[str]
    steps: list[tuple[int, list[str]]]


def _selftest_read_case(directory: Path) -> SelftestCase:
    case_path = directory / SELFTEST_CASE_FILENAME
    steps_path = directory / SELFTEST_STEPS_FILENAME
    for path in (case_path, steps_path):
        if not path.is_file():
            raise bad_input(
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
        elif key != "note":
            raise bad_input(
                f"{case_path} の知らない項目です: {key!r}", ["使えるのは title・stderr_substr・note です。"]
            )

    steps: list[tuple[int, list[str]]] = []
    for lineno, raw in enumerate(steps_path.read_text(encoding="utf-8").splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        head, separator, rest = line.partition(SELFTEST_STEP_SEPARATOR)
        if not separator or not rest.split():
            raise bad_input(
                f"{steps_path}:{lineno}: 手順の書き方が違います",
                [f"`<期待終了コード>{SELFTEST_STEP_SEPARATOR}<引数…>` の形で書きます（引数は空白区切り）。"],
            )
        try:
            expected = int(head.strip())
        except ValueError as exc:
            raise bad_input(f"{steps_path}:{lineno}: 期待終了コードが整数ではありません（{head!r}）") from exc
        steps.append((expected, rest.split()))
    if not steps:
        raise bad_input(f"{steps_path} に手順が 1 つもありません", ["何も走らせないケースは較正になりません。"])
    return SelftestCase(directory.name, directory, title or "(説明なし)", substrings, steps)


def _selftest_invoke(argv: list[str]) -> tuple[int, str, str]:
    """実運用と同じ入口 `main(argv)` を呼ぶ（自己較正だけの近道は作らない）。"""
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        try:
            code = main(argv)
        except SystemExit as exc:
            code = exc.code if isinstance(exc.code, int) else EXIT_BAD_INPUT
    return code, out.getvalue(), err.getvalue()


def _entry_region(path: Path) -> str:
    """台帳のうち「周の記録」の部分だけを取り出す（状態ブロック書き換えの検査用）。"""
    if not path.is_file():
        return ""
    text = path.read_text(encoding="utf-8").replace("\r\n", "\n")
    index = text.find("\n" + ENTRY_HEADING_FORMAT.split("{")[0])
    return "" if index < 0 else text[index:]


def _first_difference(label: str, want: str, got: str) -> str:
    want_lines, got_lines = want.splitlines(), got.splitlines()
    for index in range(max(len(want_lines), len(got_lines))):
        a = want_lines[index] if index < len(want_lines) else "(行が無い)"
        b = got_lines[index] if index < len(got_lines) else "(行が無い)"
        if a != b:
            return f"{label} {index + 1} 行目: 期待 {a!r} 実際 {b!r}"
    return f"{label}: 差はありません"


def _selftest_run_case(case: SelftestCase, workspace: Path) -> tuple[list[str], set[int]]:
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
        before = _entry_region(ledger)
        code, out, err = _selftest_invoke(argv)
        stdout_all += out
        stderr_all += err
        if code == expected:
            reproduced.add(code)
        else:
            reasons.append(f"{argv[0]}: 期待 {expected} 実際 {code}")
        if argv[0] == "set-phase" and _entry_region(ledger) != before:
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


def run_selftest() -> int:
    """fixtures-loop/ledger/ の既知ケースを逐語再現する（要件 6.7）。"""
    root = script_dir() / SELFTEST_FIXTURES_DIRNAME / SELFTEST_LEDGER_SUBDIR
    if not root.is_dir():
        raise bad_input(f"自己較正のケース置き場がありません: {root}")
    cases = [
        _selftest_read_case(child)
        for child in sorted(root.iterdir())
        if child.is_dir() and not child.name.startswith((".", "_"))
    ]
    if not cases:
        raise bad_input(
            f"自己較正のケースが 1 件もありません: {root}",
            ["ケースの無い自己較正は、何も確かめずに緑を返します。"],
        )

    ok_count = ng_count = 0
    reproduced_all: set[int] = set()
    with tempfile.TemporaryDirectory(prefix="perf-ledger-selftest-") as temporary:
        for case in cases:
            reasons, reproduced = _selftest_run_case(case, Path(temporary))
            reproduced_all |= reproduced
            if reasons:
                ng_count += 1
                print(f"[selftest] {case.name} NG  {case.title}")
                for reason in reasons:
                    print(f"           - {reason}")
            else:
                ok_count += 1
                print(f"[selftest] {case.name} ok  {case.title}")

    for code in SELFTEST_REQUIRED_RED_EXITS:
        if code not in reproduced_all:
            ng_count += 1
            print(f"[selftest] (赤の較正) NG  終了コード {code} を再現したケースが 1 件もありません")

    print(f"SELFTEST RESULT ok={ok_count} ng={ng_count}")
    return EXIT_OK if ng_count == 0 else EXIT_FAIL


# =============================================================================
# 入口
# =============================================================================


class Parser(argparse.ArgumentParser):
    """引数不正の終了コードを 3 に揃える（argparse の既定 2 は使わない）。"""

    def error(self, message: str):  # type: ignore[override]
        sys.stderr.write(f"{STDERR_PREFIX} 引数が不正です: {message}\n")
        sys.stderr.write(self.format_usage())
        raise SystemExit(EXIT_BAD_INPUT)


def build_parser() -> Parser:
    parser = Parser(
        prog="perf-ledger.py",
        description="自走改善ループの台帳を書き・読みする（要件 1.3・1.10・6.7・7.6）",
    )
    parser.add_argument(
        "--selftest",
        action="store_true",
        help=f"自己較正: {SELFTEST_FIXTURES_DIRNAME}/{SELFTEST_LEDGER_SUBDIR}/ の既知ケースを逐語再現する",
    )
    # 台帳の在り処の指定は全サブコマンド共通（親パーサから継がせる）。
    location = argparse.ArgumentParser(add_help=False)
    location.add_argument("--ledger", help="台帳のパス（最優先）")
    location.add_argument("--goal", help=f"目標の名前（{'/'.join(GOALS_RELATIVE_DIR)}/<名前>.toml を読む）")
    location.add_argument("--goal-file", help="目標定義ファイル（TOML）のパス")
    location.add_argument("--repo-root", help="リポジトリ根（既定は tools/perf の親の親）")
    subparsers = parser.add_subparsers(dest="subcommand")

    init = subparsers.add_parser("init", parents=[location], help="台帳を新しく作る")
    init.add_argument("--goal-name", help="目標の名前（省略時は目標定義ファイルの [goal].name）")
    init.add_argument("--now", help="started_at に書く ISO 日時（省略時は現在の UTC）")
    init.add_argument("--force", action="store_true", help="既にある台帳を作り直す")

    subparsers.add_parser("state", parents=[location], help="状態ブロックを JSON で出す")

    set_phase = subparsers.add_parser("set-phase", parents=[location], help="状態ブロックだけを書き換える")
    set_phase.add_argument("phase", help=f"相（{'・'.join(PHASES)}／背景待ちは {WAIT_PHASE_PREFIX} 冠）")
    set_phase.add_argument("--iteration", type=int, help="周番号")
    set_phase.add_argument("--pending-run", help="待っている走行の出力先（消すなら -）")
    set_phase.add_argument("--streak", help="採用に至らなかった連続周数")
    set_phase.add_argument("--best", help="これまでの最良のアイドル CPU（%%）")
    set_phase.add_argument("--baseline", help="ベースラインのアイドル CPU（%%）")
    set_phase.add_argument("--run", help="走行固有のトークン（task 5.5 の goal-check が書く）")
    set_phase.add_argument("--capabilities", help="道具の可否（PREFLIGHT の結果）")

    append = subparsers.add_parser("append", parents=[location], help="周の記録を 1 つ追記する")
    append.add_argument("--from-json", required=True, help="追記する値の JSON ファイル")
    append.add_argument("--now", help="周の日時（省略時は現在の UTC）")
    append.add_argument("--iteration", type=int, help="周番号（省略時は状態の iteration + 1）")

    read = subparsers.add_parser("read", parents=[location], help="指定した周の記録を JSON で出す")
    read.add_argument("--iteration", type=int, required=True, help="周番号")

    subparsers.add_parser("entries", parents=[location], help="全ての周の記録を JSON 配列で出す")

    for name, what in PLANNED_SUBCOMMANDS.items():
        subparsers.add_parser(name, parents=[location], help=f"（task 5.5 で実装）{what}")
    return parser


def main(argv: list[str]) -> int:
    for stream in (sys.stdout, sys.stderr):
        with contextlib.suppress(AttributeError, OSError):
            stream.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[union-attr]

    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        if args.selftest:
            if args.subcommand:
                parser.error("--selftest はサブコマンドと一緒に使えません（ケース側が引数を持っています）")
            return run_selftest()
        if not args.subcommand:
            parser.error(f"サブコマンドがありません（{'・'.join(SUBCOMMANDS)}・--selftest のいずれか）")
        if args.subcommand in PLANNED_SUBCOMMANDS:
            return cmd_planned(args)
        return SUBCOMMANDS[args.subcommand](args)
    except LedgerError as exc:
        label = {EXIT_BAD_INPUT: "入力不正", EXIT_IO_ERROR: "入出力の失敗"}.get(exc.code, "失敗")
        sys.stderr.write(f"{STDERR_PREFIX} {label}: {exc.message}\n")
        for detail in exc.details:
            sys.stderr.write(f"  - {detail}\n")
        sys.stderr.write(f"{STDERR_PREFIX} 台帳は変えていません。終了コード {exc.code}\n")
        return exc.code


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
