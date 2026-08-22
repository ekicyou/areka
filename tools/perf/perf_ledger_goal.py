#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""`perf-ledger.py` の**判定面**——STATUS／FINAL 行・相の遷移・目標定義の検査・まとめ。

入口は変わらず `python tools/perf/perf-ledger.py <サブコマンド>` である。ここに置くのは
次の 6 つで、いずれも「台帳と目標定義ファイルだけを読み、人の判断を挟まない」:

    status      STATUS 行を 1 本（`/goal` の判定役は会話に現れた字面しか見ない・要件 1.9）
    final       FINAL 行（走行固有の 8 桁トークン込みでのみ・要件 1.4）
    next-phase  相の遷移表の純関数（設計 C2）。`--table` で表そのものを出す
                （`@PREVIOUS@` の行き先だけは `--previous` か台帳の `previous_phase` から採る）
    goal-check  目標定義の必須キー・判定スクリプトの版・閾値の一致（違えば exit 3）＋トークン生成
    goal-text   トークンを埋めた `/goal` 条件文（4,000 字未満・要件 1.6）
    summary     `results/summary.md`（brief 旧数値との対比表・要件 7.6）

**本体との関係**: 本体 `perf-ledger.py` はファイル名にハイフンがあり import できない。
起動時に `bind(<本体モジュール>)` で受け取り、以後 `CORE.<名前>` で参照する（逆向きの
import は作らないので循環しない）。**行の文法・語彙・遷移表は本体の定数が唯一の所在**で、
ここには「その定数をどう組み立てるか」しか置かない。

**トークンの意味**（設計 C1・設計バリデーション Critical 1）: `/goal` の判定役はテンプレートと
実出力を字面でしか区別できない。そこで周 0 に `goal-check` が 8 桁の乱数を作って台帳の状態
ブロック `run` へ書き、FINAL 行はその値込みでのみ出す。文書側の見本は常に山括弧のまま書く
（`perf-ledger.py --samples` が両者を並べて確かめる）。
"""

from __future__ import annotations

import argparse
import re
import secrets

#: 本体モジュール（`bind()` が入れる）。
CORE = None


def bind(core) -> None:
    """本体モジュールを受け取る（`perf-ledger.py` が起動時に 1 度だけ呼ぶ）。"""
    global CORE
    CORE = core


# =============================================================================
# 定数（目標定義の スキーマ・条件文の雛形・brief の旧数値）
# =============================================================================

#: `/goal` の 1 周を回すプロジェクトスキル（設計 C2）。条件文はこの名前を指す。
LOOP_SKILL_NAME = "perf-loop-iteration"

#: `/goal` 条件文の上限（要件 1.6・公式文書の 4,000 字）。
GOAL_TEXT_MAX_CHARS = 4000

#: 目標定義ファイル（TOML）の必須キー（設計 C1 の表そのもの）。
#: ここに挙げた鍵が 1 つでも欠けていれば `goal-check` は欠けた鍵を挙げて exit 3 にする——
#: 「読めた鍵だけで回す」と、欠けた鍵の既定値が誰の判断か分からなくなる。
GOAL_SCHEMA: dict[str, tuple[str, ...]] = {
    "goal": ("name", "spec_dir", "ledger", "results_dir", "judge_script", "judge_version"),
    "target": ("idle_cpu_release_max_pct", "formulas", "builds_final"),
    "levels": ("short_profile", "long_profile", "ab_sequence", "iteration_build", "release_debug_env"),
    "primary_metric": ("name", "noise_floor_pct"),
    "secondary_metrics": ("must_not_regress",),
    "stop": ("max_no_gain_streak", "max_iterations", "toolfix_retry"),
    "quiet": ("machine_cpu_max_pct", "sample_sec", "heavy_process_names", "retry_max", "retry_wait_sec"),
    "followup": ("required", "exit_ms"),
    "goal_runtime": ("checkin_minutes", "main_model_recommended"),
    "sampling": ("backend",),
}

#: 判定スクリプト（`judge-perf.py`）から**字面で**読む 2 つの値。
#: import しないのは、判定スクリプトが重く Windows 前提の定数を持つため（自己較正は偽物を読む）。
JUDGE_VERSION_RE = re.compile(r"SCRIPT_VERSION\s*=\s*\(?\s*[\"'](\d+\.\d+\.\d+)")
JUDGE_THRESHOLD_RE = re.compile(r"^IDLE_CPU_MAX_RELEASE_PCT\s*=\s*([0-9]+(?:\.[0-9]+)?)", re.MULTILINE)
JUDGE_THRESHOLD_NAME = "IDLE_CPU_MAX_RELEASE_PCT"

#: `/goal` へ貼る条件文の雛形（要旨は設計 C1）。**波括弧は書式指定にしか使わない**。
#: 終端行の字面はどの行も行頭に来ないように書く（この文自体が判定役に終端と誤読されないため）。
GOAL_TEXT_TEMPLATE = """目標: {goal_name} — areka の release アイドル CPU（1 コア換算・定常平均）を {target_pct}% 未満にし、判定式⑴〜⑷b が 25 分の最終判定で全て合格すること。合否は judge-perf.py {judge_version} の出力だけで決め、人の目視や主観を使わない。

毎ターンの手順: プロジェクトスキル `{skill}` を引数 `{goal_name}` で 1 回だけ呼び、その最後に出る `{status_prefix}…` の行を一字も変えずに返答の最後の行として書く。スキルは背景コマンド（計測・最終判定）を起動するところまで相を進めてターンを終える（相の境界ごとに台帳を更新し状態行を印字する）。背景コマンドが走っている間は待つ——check-in が届いたターンは出力の末尾を読み、進行中なら「待つ」と答える。

達成の判定: 会話に `{final_prefix}GOAL_MET run={token}` で始まる行が現れたとき。
不可能の判定: 会話に `{final_prefix}STOPPED run={token} reason=` で始まる行が現れたとき（頭打ち・安全停止・道具の故障・周数上限 {max_iterations} のいずれか）。
注意: 文書・スキル本文・README に載っている書式見本は山括弧つき（例 run=<token>）で、実出力とは一致しない。判定は上の実トークン {token} を含む字面でのみ行う。

制約:
- 開発者へ質問しない・裁定を仰がない。必要な判断はルール化して行い、根拠を台帳へ残す。
- Cargo.toml を変更しない。採用は 1 周 1 コミット。採用しない変更は元へ戻す。
- 判断の記憶は台帳 `{ledger}` だけに持つ（会話の記憶・要約に頼らない。再開時は台帳の phase から続ける）。
- 連続 {max_streak} 周で採用に至る改善が無ければ頭打ちとして止まる。周数の上限は {max_iterations} 周。
- check-in は {checkin_minutes} 分（環境変数 CLAUDE_CODE_GOAL_CHECKIN_MINUTES）。計測 1 本がこれを超えないよう分けて起動する。
- 重い作業（計測・順位付け・実装・差し戻し判定）は役割別のサブエージェントへ委ね、結論だけを受け取る。
"""

#: brief `## Problem` の旧数値（2026-08-15 実測・emo2・拡大率 200%・25 分走行・1 コア換算 %）。
#: `summary` の対比表の左半分。**値の所在はここ 1 箇所**で、brief を書き換えたらここも直す。
BRIEF_SOURCE = ".kiro/specs/areka-P0-draw-load-parity/brief.md `## Problem`（2026-08-15 実測）"
#: (指標, areka の旧値, SSP 参考値, 台帳のどの数と並べるか)
BRIEF_ROWS = (
    ("アイドル CPU 平均（1 コア換算 %）", "10.97", "3.05", "primary"),
    ("CPU の底（アイドル・%）", "3.60", "1.77", None),
    ("CPU の頂（発話中・%）", "20.42", "4.64", None),
    ("Private メモリ（MB）", "163.4", "54.2", None),
    ("スレッド数", "83", "32", None),
    ("定常 catch-up 件数（release・短時間）", "17", "-", None),
    ("定常 catch-up 件数（release・25 分）", "69", "-", None),
)
#: 先行 spec が残した内訳（対比表の下に注として並べる。合否には使わない）。
BRIEF_NOTES = (
    "表示 1 コマの適用経路は先行 spec `areka-P0-recompose-budget` が 22,210µs → 1,240µs（18 分の 1）"
    "まで削ったが、アイドル CPU の 3.3% しか占めておらず中央値は 9.3% のまま動かなかった。",
    "残る主役はフレーム駆動そのもの——ECS の tick が毎秒 120 回・1 回あたり約 578µs で 13 本の"
    "スケジュールを全部回し、その 98% は表示に変化が無い（上位 2 本 FrameFinalize 182µs・Draw 143µs で 56%）。",
    "SSP 列は参考値であり合否には使わない（要件 5.2）。",
)

#: 対比表の空欄（台帳にまだ値が無い列）。
NOT_AVAILABLE = "-"


# =============================================================================
# 台帳から値を取り出す小道具
# =============================================================================


def _latest_entry(ledger):
    """最後の周の記録（周番号が最大のもの）。1 件も無ければ None。"""
    return max(ledger.entries, key=lambda entry: entry.iteration) if ledger.entries else None


def _entry_number(entry, key: str, ledger, signed: bool = False) -> str:
    """周の記録の小数を STATUS 行の精度で返す。記録が無ければ `-`。"""
    if entry is None:
        return CORE.EMPTY_VALUE
    raw = entry.values.get(key, CORE.EMPTY_VALUE)
    return CORE.format_number_text(raw, signed, f"{ledger.path} の周 {entry.iteration} の {key}")


def _goal_table(args) -> dict:
    """目標定義ファイルの中身（指定が無ければ空の辞書＝既定値で回す）。"""
    toml_path = CORE.goal_toml_path(args)
    return CORE.load_goal_toml(toml_path) if toml_path is not None else {}


def _stop_limits(args) -> tuple[int, int]:
    """停止条件（連続無改善の上限・周数の上限）。TOML → 既定 の順で解く。"""
    stop = _goal_table(args).get("stop") or {}
    max_streak = getattr(args, "max_streak", None) or stop.get(
        "max_no_gain_streak", CORE.DEFAULT_MAX_NO_GAIN_STREAK
    )
    max_iterations = getattr(args, "max_iterations", None) or stop.get(
        "max_iterations", CORE.DEFAULT_MAX_ITERATIONS
    )
    try:
        return int(max_streak), int(max_iterations)
    except (TypeError, ValueError) as exc:
        raise CORE.bad_input(
            "目標定義ファイルの [stop] の値が整数ではありません",
            [f"max_no_gain_streak={max_streak!r} max_iterations={max_iterations!r}"],
        ) from exc


def _run_token(ledger) -> str:
    """状態ブロックの走行トークン。無ければ（＝周 0 の goal-check がまだ）exit 3。"""
    token = ledger.state.get("run", CORE.EMPTY_VALUE)
    if token == CORE.EMPTY_VALUE:
        raise CORE.bad_input(
            f"走行トークンが状態ブロックにありません: {ledger.path}",
            [
                "FINAL 行は走行固有のトークン込みでのみ出します（テンプレートと実出力を"
                "字面で区別できるようにするため・設計 C1）。",
                "先に `goal-check --goal <名前>` を走らせてください（周 0 の手順）。",
            ],
        )
    if not CORE.RUN_TOKEN_RE.match(token):
        raise CORE.bad_input(
            f"走行トークンが {CORE.RUN_TOKEN_DIGITS} 桁の数字ではありません: {token!r}",
            [str(ledger.path)],
        )
    return token


def _default_next_phase(phase: str) -> str:
    """STATUS 行の既定の `next=`＝遷移表の `ok` の行き先（無ければ今の相のまま）。"""
    base = _base_phase(phase)
    target = CORE.PHASE_TRANSITIONS.get(base, {}).get("ok")
    return phase if target is None or target == CORE.PREVIOUS_PHASE_MARKER else target


def _base_phase(phase: str) -> str:
    """背景待ち（`WAIT_` 冠）は素の相として扱う（待っていても相の意味は変わらない）。"""
    prefix = CORE.WAIT_PHASE_PREFIX
    return phase[len(prefix):] if phase.startswith(prefix) else phase


def _first_present(*candidates: str | None) -> str:
    """最初に「値のある」候補を返す（無ければ `-`）。空値と未指定を同じに扱う。"""
    for value in candidates:
        if value and value != CORE.EMPTY_VALUE:
            return value
    return CORE.EMPTY_VALUE


def _idle_cpu_text(args, ledger, latest) -> str:
    """今のアイドル CPU——直の指定 → 最後の周の後の値 → これまでの最良、の順。"""
    return _first_present(
        CORE.format_number_text(args.idle_cpu, False, "--idle-cpu") if args.idle_cpu else None,
        _entry_number(latest, "after_idle_cpu_pct", ledger),
        CORE.state_number_text(ledger, "best_idle_cpu_pct"),
    )


def _one_of(value: str, allowed: tuple[str, ...], what: str) -> str:
    if value not in allowed:
        raise CORE.bad_input(f"知らない{what}です: {value}", [f"使えるのは {'・'.join(allowed)} です。"])
    return value


def _emit_checked(line: str, pattern, what: str) -> None:
    """組み立てた行が文法どおりかを出す前に確かめる（崩れた行を会話へ流さない）。"""
    if not pattern.match(line):
        raise CORE.bad_input(
            f"{what}が決まった書式になりませんでした（台帳の値を確かめてください）",
            [line, f"期待する形: {CORE.SAMPLE_LINES_FOR_DOCS[0 if what.startswith('STATUS') else 1]}"],
        )
    print(line)


# =============================================================================
# status / final / next-phase
# =============================================================================


def cmd_status(args) -> int:
    """STATUS 行を 1 本だけ出す（要件 1.9）。値は状態ブロックと最後の周の記録から組む。"""
    ledger = CORE.load_ledger(args)
    max_streak, max_iterations = _stop_limits(args)
    latest = _latest_entry(ledger)

    iteration = CORE.state_int(ledger, "iteration", default=0)
    phase = CORE.validate_phase(args.phase) if args.phase else ledger.state["phase"]
    judge = _one_of(args.judge or "NA", CORE.JUDGE_RESULTS, "判定結果")
    verdict = args.verdict or (latest.values.get("verdict") if latest else None) or "NA"
    _one_of(verdict, CORE.VERDICTS, "採否")

    idle = _idle_cpu_text(args, ledger, latest)
    line = CORE.STATUS_PREFIX + " ".join((
        f"iter={iteration}",
        f"phase={phase}",
        f"judge={judge}",
        f"idle_cpu={idle}",
        f"baseline={CORE.state_number_text(ledger, 'baseline_idle_cpu_pct')}",
        f"delta={_entry_number(latest, 'delta_pct', ledger, signed=True)}",
        f"noise={_entry_number(latest, 'noise_pct', ledger)}",
        f"verdict={verdict}",
        f"streak={CORE.state_int(ledger, 'streak_no_gain', default=0)}/{max_streak}",
        f"iters_left={max(max_iterations - iteration, 0)}",
        f"next={CORE.validate_phase(args.next) if args.next else _default_next_phase(phase)}",
    ))
    _emit_checked(line, CORE.STATUS_RE, "STATUS 行")
    return CORE.EXIT_OK


def cmd_final(args) -> int:
    """FINAL 行を出す（走行トークン込みでのみ・要件 1.4）。"""
    ledger = CORE.load_ledger(args)
    token = _run_token(ledger)
    outcome = _one_of(args.outcome, CORE.FINAL_OUTCOMES, "終端の種類")
    iterations = args.iters if args.iters is not None else CORE.state_int(ledger, "iteration", default=0)
    latest = _latest_entry(ledger)

    if outcome == "GOAL_MET":
        if args.reason:
            raise CORE.bad_input(
                "GOAL_MET に reason は付けません",
                [f"reason= を付けるのは STOPPED のときだけです（{'・'.join(CORE.FINAL_REASONS)}）。"],
            )
        idle = _idle_cpu_text(args, ledger, latest)
        commits = args.commits if args.commits is not None else _adopted_commits(ledger)
        line = CORE.FINAL_PREFIX + (
            f"GOAL_MET run={token} idle_cpu={idle} judge=PASS iters={iterations} commits={commits}"
        )
    else:
        if not args.reason:
            raise CORE.bad_input(
                "STOPPED には reason が要ります",
                [f"使えるのは {'・'.join(CORE.FINAL_REASONS)} です（停止の理由を残さずに止めない）。"],
            )
        _one_of(args.reason, CORE.FINAL_REASONS, "停止の reason")
        best = (
            CORE.format_number_text(args.best_idle_cpu, False, "--best-idle-cpu")
            if args.best_idle_cpu
            else CORE.state_number_text(ledger, "best_idle_cpu_pct")
        )
        top = args.top_remaining or CORE.EMPTY_VALUE
        line = CORE.FINAL_PREFIX + (
            f"STOPPED run={token} reason={args.reason} best_idle_cpu={best} "
            f"top_remaining={top} iters={iterations}"
        )
    _emit_checked(line, CORE.FINAL_RE, "FINAL 行")
    return CORE.EXIT_OK


def _adopted_commits(ledger) -> int:
    """採用した周の数（`--commits` を省いたときの既定＝採用＝1 周 1 コミット・要件 1.8）。"""
    return sum(1 for entry in ledger.entries if entry.values.get("verdict") == "ADOPTED")


def _resolve_previous_phase(args) -> str:
    """`@PREVIOUS@`（TOOLFIX から戻る先）を解く。`--previous` が最優先・無ければ台帳。

    遷移表そのものは相と出来事だけで決まる（純関数）。ここで台帳を読むのは「直前の相」＝
    **呼び出し側の状態**の受け取り口が 1 本増えるだけで、表の中身には触れない。台帳を読む
    経路を足した理由は要件 1.10——渡す側（スキル）は会話の記憶を持たないので、置き場が
    台帳しかない。台帳の在り処が渡されていなければ、これまでどおり `--previous` を要求する。
    """
    if args.previous:
        return CORE.validate_phase(args.previous)
    stored = ""
    if args.ledger or args.goal or args.goal_file:
        ledger = CORE.load_ledger(args)
        raw = ledger.state.get("previous_phase", CORE.EMPTY_VALUE)
        stored = "" if raw == CORE.EMPTY_VALUE else raw
    if not stored:
        raise CORE.bad_input(
            "TOOLFIX の toolfix_ok は直前の相へ戻ります（--previous <相> が要ります）",
            [
                "直前の相は台帳の記録から渡してください（道具は覚えていません）。",
                "台帳（--ledger／--goal／--goal-file）の状態ブロックに `previous_phase` が"
                "書いてあれば、--previous は省けます（`set-phase <相> --previous-phase <相>`）。",
            ],
        )
    return CORE.validate_phase(stored)


def cmd_next_phase(args) -> int:
    """相の遷移表（純関数）。入力は相と出来事だけ。

    唯一の例外が `@PREVIOUS@` の行き先で、これは `--previous` か台帳の `previous_phase`
    から受け取る（`_resolve_previous_phase`）。目標定義は台帳の在り処を解くためだけに読む。
    """
    if args.table:
        for phase in CORE.PHASES:
            row = CORE.PHASE_TRANSITIONS.get(phase, {})
            if not row:
                print(f"{phase} {CORE.TERMINAL_PHASE_NOTE}")
                continue
            for event, target in row.items():
                print(f"{phase} {event} -> {target}")
        return CORE.EXIT_OK

    if not args.phase or not args.event:
        raise CORE.bad_input(
            "next-phase には --phase と --event が要ります",
            ["遷移表そのものを見るなら `next-phase --table` です。"],
        )
    base = _base_phase(CORE.validate_phase(args.phase))
    row = CORE.PHASE_TRANSITIONS.get(base, {})
    if args.event not in row:
        known = "・".join(row) if row else "（無し）"
        raise CORE.bad_input(
            f"相 {base} に出来事 {args.event} の遷移が定義されていません",
            [
                f"{base} で使える出来事: {known}",
                "表に無い組で先へ進めると、どの相からどう来たのかが後から分かりません。",
            ],
        )
    target = row[args.event]
    if target == CORE.PREVIOUS_PHASE_MARKER:
        target = _resolve_previous_phase(args)
    print(target)
    return CORE.EXIT_OK


# =============================================================================
# goal-check / goal-text
# =============================================================================


def _require_goal_file(args):
    toml_path = CORE.goal_toml_path(args)
    if toml_path is None:
        raise CORE.bad_input(
            "目標定義ファイルが指定されていません",
            ["`--goal <名前>` か `--goal-file <toml>` を渡してください。"],
        )
    return toml_path, CORE.load_goal_toml(toml_path)


def _missing_goal_keys(data: dict) -> list[str]:
    missing: list[str] = []
    for section, keys in GOAL_SCHEMA.items():
        table = data.get(section)
        if not isinstance(table, dict):
            missing.append(f"[{section}]")
            continue
        missing.extend(f"{section}.{key}" for key in keys if table.get(key) is None)
    return missing


def _read_judge_constants(path) -> tuple[str, float]:
    """判定スクリプトの版と閾値を**字面で**読む（import しない）。"""
    text = CORE.read_judge_text(path)
    version = JUDGE_VERSION_RE.search(text)
    threshold = JUDGE_THRESHOLD_RE.search(text)
    if not version or not threshold:
        raise CORE.bad_input(
            f"判定スクリプトから版か較正値を読めません: {path}",
            [
                f"`SCRIPT_VERSION = \"<x.y.z> …\"` と `{JUDGE_THRESHOLD_NAME} = <数>` の 2 行を探します。",
            ],
        )
    return version.group(1), float(threshold.group(1))


def cmd_goal_check(args) -> int:
    """目標定義を確かめ、周 0 に走行トークンを作って状態ブロックへ書く（設計 C1）。"""
    toml_path, data = _require_goal_file(args)
    missing = _missing_goal_keys(data)
    if missing:
        raise CORE.bad_input(
            f"目標定義ファイルに必須キーがありません: {'・'.join(missing)}",
            [str(toml_path), f"必須の全体は perf_ledger_goal.GOAL_SCHEMA（{len(GOAL_SCHEMA)} 節）です。"],
        )

    goal_table, target_table = data["goal"], data["target"]
    judge_path = CORE.resolve_judge_script(args, goal_table)
    judge_version, judge_threshold = _read_judge_constants(judge_path)
    if str(goal_table["judge_version"]) != judge_version:
        raise CORE.bad_input(
            "判定スクリプトの版が目標定義と違います",
            [
                f"[goal].judge_version = {goal_table['judge_version']}（{toml_path}）",
                f"SCRIPT_VERSION = {judge_version}（{judge_path}）",
                "版が違えば判定式か較正値が違うので、そのまま回すと合否の意味が変わります。",
            ],
        )
    declared = target_table["idle_cpu_release_max_pct"]
    if abs(float(declared) - judge_threshold) > 1e-9:
        raise CORE.bad_input(
            f"目標の閾値が判定スクリプトの較正値 {JUDGE_THRESHOLD_NAME} と違います",
            [
                f"[target].idle_cpu_release_max_pct = {declared}（{toml_path}）",
                f"{JUDGE_THRESHOLD_NAME} = {judge_threshold}（{judge_path}）",
                "目標と判定式が別の数を見ていると、合格の意味が 2 つになります。",
            ],
        )

    ledger = CORE.load_ledger(args)
    declared_name = str(goal_table["name"])
    if ledger.state["goal"] != declared_name:
        raise CORE.bad_input(
            f"台帳の goal と目標定義の [goal].name が違います: "
            f"{ledger.state['goal']!r} / {declared_name!r}",
            [str(ledger.path), str(toml_path)],
        )

    existing = ledger.state.get("run", CORE.EMPTY_VALUE)
    if existing != CORE.EMPTY_VALUE:
        token = _run_token(ledger)
        if args.token and args.token != token:
            raise CORE.bad_input(
                f"走行トークンは書き換えません（今は {token}・指定は {args.token}）",
                [
                    "トークンを差し替えると、それまでに会話へ出した終端行の字面が意味を失います。",
                    "作り直すなら台帳ごと `init --force` してください。",
                ],
            )
        CORE.report(f"走行トークンは既にあります（変えません）: {token}")
    else:
        token = _new_run_token(args.token)
        state = dict(ledger.state)
        state["run"] = token
        CORE.write_text(ledger.path, CORE.replace_state_block(ledger, state))
        CORE.report(f"走行トークンを状態ブロックへ書きました: {token}")

    print(f"GOAL-CHECK ok judge_version={judge_version} token={token}")
    return CORE.EXIT_OK


def _new_run_token(requested: str | None) -> str:
    """8 桁の走行トークンを作る。`00000000` は文書の見本専用なので作らない。"""
    if requested is not None:
        if not CORE.RUN_TOKEN_RE.match(requested):
            raise CORE.bad_input(
                f"--token は {CORE.RUN_TOKEN_DIGITS} 桁の数字です: {requested!r}",
                ["自己較正・再現のためだけの指定です（普段は省いて乱数に任せます）。"],
            )
        return requested
    while True:
        token = f"{secrets.randbelow(10 ** CORE.RUN_TOKEN_DIGITS):0{CORE.RUN_TOKEN_DIGITS}d}"
        if token != CORE.RESERVED_RUN_TOKEN:
            return token


def cmd_goal_text(args) -> int:
    """走行トークンを埋めた `/goal` 条件文を出す（要件 1.6・4,000 字未満）。"""
    toml_path, data = _require_goal_file(args)
    missing = _missing_goal_keys(data)
    if missing:
        raise CORE.bad_input(
            f"目標定義ファイルに必須キーがありません: {'・'.join(missing)}",
            [str(toml_path), "先に `goal-check` を通してください。"],
        )
    ledger = CORE.load_ledger(args)
    token = _run_token(ledger)
    stop = data["stop"]

    text = GOAL_TEXT_TEMPLATE.format(
        goal_name=data["goal"]["name"],
        target_pct=data["target"]["idle_cpu_release_max_pct"],
        judge_version=data["goal"]["judge_version"],
        skill=LOOP_SKILL_NAME,
        status_prefix=CORE.STATUS_PREFIX,
        final_prefix=CORE.FINAL_PREFIX,
        token=token,
        ledger=data["goal"]["ledger"],
        max_streak=stop["max_no_gain_streak"],
        max_iterations=stop["max_iterations"],
        checkin_minutes=data["goal_runtime"]["checkin_minutes"],
    )
    if len(text) >= GOAL_TEXT_MAX_CHARS:
        raise CORE.bad_input(
            f"/goal 条件文が長すぎます（{len(text)} 文字・上限 {GOAL_TEXT_MAX_CHARS}）",
            ["詳しい手順は tools/perf/README.md へ移し、条件文は判定に要る事実だけにしてください。"],
        )
    for line in text.splitlines():
        if CORE.STATUS_RE.match(line) or CORE.FINAL_RE.match(line):
            raise CORE.bad_input(
                "条件文の中に、そのまま終端と読める行があります",
                [line, "条件文を貼った時点で「達成」と判定されてしまいます（行頭に置かないこと）。"],
            )
    print(text, end="" if text.endswith("\n") else "\n")
    CORE.report(f"/goal 条件文: {len(text)} 文字（上限 {GOAL_TEXT_MAX_CHARS}）・走行トークン {token}")
    return CORE.EXIT_OK


# =============================================================================
# summary（brief 旧数値との対比表・要件 7.6）
# =============================================================================


def build_summary(ledger, goal_name: str) -> str:
    """まとめの Markdown を組む（純関数・時計を読まないので何度走らせても同じ）。"""
    baseline = CORE.state_number_text(ledger, "baseline_idle_cpu_pct")
    best = CORE.state_number_text(ledger, "best_idle_cpu_pct")
    latest = _latest_entry(ledger)
    final = _entry_number(latest, "after_idle_cpu_pct", ledger)

    lines = [
        f"# {goal_name} — 改善ループの結果まとめ",
        "",
        f"- 台帳: `{ledger.path.name}`（周 {CORE.state_int(ledger, 'iteration', default=0)}"
        f"・記録 {len(ledger.entries)} 件）",
        f"- 相: {ledger.state['phase']}",
        f"- 開始: {ledger.state['started_at']}",
        f"- 走行トークン: {ledger.state.get('run', CORE.EMPTY_VALUE)}",
        "",
        "## brief 旧数値との対比",
        "",
        f"出所: {BRIEF_SOURCE}",
        "",
        "| 指標 | brief 旧数値（areka） | 参考（SSP） | ベースライン | 最良 | 最終 |",
        "|---|---:|---:|---:|---:|---:|",
    ]
    for label, old, ssp, kind in BRIEF_ROWS:
        if kind == "primary":
            columns = (baseline, best, final)
        else:
            columns = (NOT_AVAILABLE, NOT_AVAILABLE, NOT_AVAILABLE)
        lines.append(f"| {label} | {old} | {ssp} | {columns[0]} | {columns[1]} | {columns[2]} |")
    lines += ["", "注:"]
    lines += [f"- {note}" for note in BRIEF_NOTES]

    lines += [
        "",
        "## 周ごとの採否",
        "",
        "| 周 | 日時 | 仮説 | 採否 | 前 | 後 | 差 | ばらつき | コミット |",
        "|---:|---|---|---|---:|---:|---:|---:|---|",
    ]
    if not ledger.entries:
        lines.append("| - | - | （まだ 1 周も記録がありません） | - | - | - | - | - | - |")
    for entry in sorted(ledger.entries, key=lambda item: item.iteration):
        values = entry.values
        lines.append(
            f"| {entry.iteration} | {entry.timestamp} | {values.get('hypothesis', CORE.EMPTY_VALUE)} "
            f"| {values.get('verdict', CORE.EMPTY_VALUE)} "
            f"| {_entry_number(entry, 'before_idle_cpu_pct', ledger)} "
            f"| {_entry_number(entry, 'after_idle_cpu_pct', ledger)} "
            f"| {_entry_number(entry, 'delta_pct', ledger, signed=True)} "
            f"| {_entry_number(entry, 'noise_pct', ledger)} "
            f"| {values.get('commit', CORE.EMPTY_VALUE)} |"
        )
    lines.append("")
    return "\n".join(lines)


def cmd_summary(args) -> int:
    """`results/summary.md` を書く（`--out -` なら標準出力へ）。"""
    ledger = CORE.load_ledger(args)
    text = build_summary(ledger, ledger.state["goal"])
    if args.out == "-":
        print(text, end="" if text.endswith("\n") else "\n")
        return CORE.EXIT_OK
    out_path = CORE.resolve_summary_path(args)
    CORE.write_text(out_path, text)
    CORE.report(f"まとめを書きました: {out_path}")
    return CORE.EXIT_OK


# =============================================================================
# --samples（見本と実出力が判定の正規表現の反対側に立つことを確かめる）
# =============================================================================


def print_samples() -> int:
    """書式見本（山括弧）と実出力の見本を並べ、両者が判定の正規表現の反対側に立つか確かめる。

    見本行は文書・スキル本文・README・goal テンプレートへそのまま貼るためのもので、
    **判定の正規表現に一致してはならない**（設計 C1）。逆に実出力の見本は一致しなければ
    ならない——どちらかが崩れたら、その場で理由を述べて exit 3 にする（黙って緑にしない）。
    印字は `sample: `／`real: ` を冠する。行頭が `PERF-LOOP` でなくなるので、この出力自体が
    `/goal` の判定役に終端行と誤読されることもない。
    """
    for line in CORE.SAMPLE_LINES_FOR_DOCS:
        if CORE.STATUS_RE.match(line) or CORE.FINAL_RE.match(line):
            raise CORE.bad_input(
                "文書の書式見本が判定の正規表現に一致しました（見本と実出力は一致させない）",
                [line, "山括弧のプレースホルダを外さないでください（設計 C1）。"],
            )
        print(f"sample: {line}")
    for line in CORE.REAL_EXAMPLE_LINES:
        if not (CORE.STATUS_RE.match(line) or CORE.FINAL_RE.match(line)):
            raise CORE.bad_input(
                "実出力の見本が判定の正規表現に一致しません（文法か見本のどちらかが古い）",
                [line],
            )
        print(f"real: {line}")
    return CORE.EXIT_OK


# =============================================================================
# サブコマンドの登録
# =============================================================================

COMMANDS = {
    "status": cmd_status,
    "final": cmd_final,
    "next-phase": cmd_next_phase,
    "goal-check": cmd_goal_check,
    "goal-text": cmd_goal_text,
    "summary": cmd_summary,
}


def add_parsers(subparsers, location: argparse.ArgumentParser) -> None:
    """判定面の 6 つの受け口を作る（`location` は台帳の在り処の共通引数）。"""
    status = subparsers.add_parser("status", parents=[location], help="STATUS 行を 1 本出す")
    status.add_argument("--judge", help=f"判定結果（{'・'.join(CORE.JUDGE_RESULTS)}・既定 NA）")
    status.add_argument("--verdict", help=f"採否（{'・'.join(CORE.VERDICTS)}・既定は最後の周の値）")
    status.add_argument("--phase", help="相（既定は状態ブロックの phase）")
    status.add_argument("--next", help="次の相（既定は遷移表の ok の行き先）")
    status.add_argument("--idle-cpu", help="今のアイドル CPU（%%・既定は最後の周の後の値）")
    status.add_argument("--max-streak", help="連続無改善の上限（既定は [stop].max_no_gain_streak）")
    status.add_argument("--max-iterations", help="周数の上限（既定は [stop].max_iterations）")

    final = subparsers.add_parser("final", parents=[location], help="FINAL 行を出す")
    final.add_argument("--outcome", required=True, help="・".join(CORE.FINAL_OUTCOMES))
    final.add_argument("--reason", help=f"停止の理由（STOPPED のみ・{'・'.join(CORE.FINAL_REASONS)}）")
    final.add_argument("--idle-cpu", help="到達したアイドル CPU（%%・GOAL_MET）")
    final.add_argument("--best-idle-cpu", help="これまでの最良（%%・STOPPED・既定は状態ブロック）")
    final.add_argument("--top-remaining", help="残る最大項 stage:item:share（STOPPED）")
    final.add_argument("--commits", type=int, help="採用した周の数（既定は台帳の ADOPTED の件数）")
    final.add_argument("--iters", type=int, help="回した周数（既定は状態ブロックの iteration）")

    next_phase = subparsers.add_parser("next-phase", parents=[location], help="相の遷移表（純関数）")
    next_phase.add_argument("--phase", help=f"今の相（{CORE.WAIT_PHASE_PREFIX} 冠も可）")
    next_phase.add_argument("--event", help="起きた出来事（表に無ければ exit 3）")
    next_phase.add_argument(
        "--previous",
        help=f"直前の相（{CORE.PREVIOUS_PHASE_MARKER} の行き先。省くと台帳の previous_phase を読む）",
    )
    next_phase.add_argument("--table", action="store_true", help="遷移表そのものを出す")

    goal_check = subparsers.add_parser("goal-check", parents=[location], help="目標定義の検査とトークン生成")
    goal_check.add_argument("--judge-file", help="判定スクリプト（既定は [goal].judge_script）")
    goal_check.add_argument("--token", help=f"走行トークンを指定する（{CORE.RUN_TOKEN_DIGITS} 桁・再現用）")

    subparsers.add_parser("goal-text", parents=[location], help="トークンを埋めた /goal 条件文")

    summary = subparsers.add_parser("summary", parents=[location], help="brief 旧数値との対比表")
    summary.add_argument("--out", help="書き出し先（`-` で標準出力・既定は <spec_dir>/<results_dir>/summary.md）")
