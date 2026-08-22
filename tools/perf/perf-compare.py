#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""A→B→A→B の 4 本の走行から差とばらつきを出し、変更の採否を返す道具（設計 C10）。

自走改善ループの ⒢ 採否はこの 1 ファイルの出す `verdict` だけで決まる（要件 1.7・3.6）。
人の目視・その場の相談・「たぶん速くなった」は入らない。だから判定は 4 つの数だけで書ける
形にしてあり、その 4 つ（A の平均・B の平均・差・ばらつき）は `compare.txt` の表から
誰でも手で引き直せる。

    python tools/perf/perf-compare.py --a <A1> <A2> --b <B1> <B2> --goal <名前>
    python tools/perf/perf-compare.py --a <A1> <A2> --b <B1> <B2> --goal-file <toml>
    python tools/perf/perf-compare.py --selftest          # 自己較正

`<A1>`… は走行ディレクトリ（`run.log`・`cpu.csv`・`run-meta.txt` がある場所）である。
A＝変更前・B＝変更後で、`A1 → B1 → A2 → B2` の順に採る（`[levels].ab_sequence`）。
同じ形を 2 回はさむのは、走行と走行のあいだに起きるマシン側の揺れを、変更の効果と
取り違えないためである。

判定（設計 C10・要件 1.7）
--------------------------
主指標は `[primary_metric].name`（既定 `steady_idle_cpu_mean_pct`＝定常アイドル CPU の
平均・**小さいほど良い**）。

    ばらつき noise = max(|A1 − A2|, [primary_metric].noise_floor_pct)
    差       delta = mean(B1, B2) − mean(A1, A2)

    delta ≤ −noise  かつ 副指標に悪化が無い        → ADOPTED     （採用・コミットする）
    |delta| < noise かつ 副指標に悪化が無い        → NO_DIFF     （差なし・元へ戻す）
    delta ≥ noise  または 副指標に悪化がある       → WORSE       （悪化・元へ戻す）
    いずれかの走行が判定不能                       → MEASURE_FAILED（計測失敗）

**副指標の悪化は主指標の改善に優先する**。CPU がどれだけ下がっても、コマ適用が遅く
なったり catch-up が増えたりした変更は採らない（要件 4.6＝「軽くなった代わりに反応が
鈍くなった」を持ち込まない）。設計 C10 の文面は `|delta| < noise → NO_DIFF` と
`delta ≥ noise または副指標悪化 → WORSE` を並べて書いているが、「差なしの帯の中で
副指標だけが悪化した」場合はどちらとも読める。ここでは **安全側に倒して WORSE** と
する——差が無いのに副指標が悪化した変更を残す理由が無いためである。

副指標の規則（`[secondary_metrics].must_not_regress`）
------------------------------------------------------
名前の末尾で規則を選ぶ。目標定義ファイルが別の指標を並べても同じ規則で読める。

    `_ms` / `_pct` で終わる  … 率で見る。mean(B) > mean(A) × (1 + 0.05) なら悪化
    それ以外（`_count` ほか）… 増減で見る。mean(B) > mean(A) なら悪化（許容率は無い）

件数に許容率を与えないのは、判定式⑵⑶ が定常状態の catch-up と新規確保を **0 件**
と定めているからである（0 件に「5% 以内の増加」は書けない）。

**片側でも `-`（測っていない）なら、その副指標は `NA` として扱い、採用を止めない**。
設計は `-` の扱いを書いていないので、ここで決めて表と `compare.json` の両方に残す。
理由は `-` と `0` の違いにある——`-` は「測っていない」であって「悪化していない」では
ないから悪化と断ずることはできず、かといって「観測が無いこと」を悪化として扱うと、
既定 OFF の観測（設計 C14・C15）を点けずに採った走行が必ず WORSE になり、採否が
計測の点灯忘れで決まってしまう。ゆえに `NA` は判定に効かせず、代わりに表と
`worse_secondaries` の外側に必ず名前を出して、見落としを防ぐ。

判定スクリプトとの関係（別プロセスで呼ぶ・import しない）
----------------------------------------------------------
各走行の集計は `judge-perf.py --mode baseline --emit-metrics` を **subprocess** で呼び、
標準出力の `metric=<名前> value=<値>` 行だけを読む（設計 C10）。import しないのは、
同じ周に別の担当が判定スクリプトを触っている可能性があり、輸入すると 2 つの道具が
同時に倒れるためである（`perf-rank.py` と同じ理由）。行の読み方は `judge-perf.py` の
`parse_fields` の規則を写してあり、写した箇所には出典を注記してある。

`--build` は `[levels].iteration_build` から渡す。`judge-perf.py` は `--build` と
`run-meta.txt` の `build` が食い違えば止まるので、A/B の 4 本が採否の走行ビルドで
採られたことがここで確かめられる（別ビルドの走行を混ぜた比較は成立しない）。

`judge-perf.py` の終了コードの読み方（設計「Error Handling」）
--------------------------------------------------------------
    0  正常終了            → 数値を使う
    1  不合格（判定モード）→ **判定不能ではない**。「合否が付いた＝数値は採れている」
                             という意味なので、比較の入力としては使える
    2  判定不能            → その走行を判定不能とし、総合を MEASURE_FAILED にする
    3  引数不正・読取不能  → 同上（数値が 1 つも無い）

**終了コード 1 の腕には fixture を置いていない（意図的）**。`judge-perf.py` の `EXIT_FAIL`
は「合否判定モード専用。集計モードは返さない」と定数の注記そのものが宣言しており
（`judge-perf.py:715`）、`perf-compare.py` は常に `--mode baseline` で呼ぶので、この腕は
**現在の判定スクリプトからは到達しない**。到達しない腕を fixture で固定することはできない
ので、代わりに「1 を判定不能に数えない」という決めをここに書き、判定スクリプトが集計
モードでも 1 を返すようになったときに、この段落が食い違いとして目に入るようにしてある。

主指標が行に無い、または `-` である走行も判定不能である。`[goal].judge_version` が
走行の `script_version` と違う場合も判定不能とする——版が違えば判定式か較正値が違い、
前後で意味の違う数を引き算することになるからである（設計 C1）。

出力
----
`--out-dir`（既定はカレント）へ 2 ファイルを書く。

    compare.txt   人が読む表。**基底名しか書かない**ので、同じ入力からは同じ文面になる
                  （走行の絶対パスは compare.json 側にある）
    compare.json  台帳の周の記録（設計 C11）と **同じ綴りの鍵**を 6 つ持つ:
                  `before_idle_cpu_pct` `after_idle_cpu_pct` `delta_pct` `noise_pct`
                  `secondary` `verdict`

`compare.json` を `perf-ledger.py append --from-json` へ**そのまま**渡すことはできない
（`load_entry_json` が `ENTRY_KEYS` に無い鍵を拒む・`perf-ledger.py:772-781`）。周の記録は
`hypothesis`・`files_changed`・`tests`・`followup`・`commit` など別の相の値も要るので、
スキル（設計 C2 の `RECORD`）がこの 6 つを抜き出して他の値と合わせ、1 つの JSON にする。
綴りを揃えてあるのはその写し替えを機械的にするためである。null は台帳側で `-` になる
（`_to_ledger_value`・`perf-ledger.py:456-458`）。

`MEASURE_FAILED` でも 2 ファイルとも書く。計測失敗は台帳に記録する「採否の 1 つ」で
あって、途中で投げ出した状態ではないためである（`perf-rank.py` が失敗時に順位表を
書かないのとは、ここが違う）。ただし **判定不能を含む側の平均と差は空（null）にする**
——測れた事実は残し、測れていない事実を数字で埋めない。

数の丸め
--------
主指標まわりの数は、比べる前に小数 2 桁へ丸める（台帳の `<x.xx>` と同じ精度）。
表に出た数から手で引き直した結果と、道具の出した採否が食い違わないようにするためで
ある——生の浮動小数で比べると「表は差 -0.30・ばらつき 0.30 なのに採用ではない」が
起こり得る。

終了コード（`judge-perf.py`・`perf-rank.py`・`perf-ledger.py` と同じ体系）
--------------------------------------------------------------------------
    0  判定できた（ADOPTED / NO_DIFF / WORSE のいずれか。改善したという意味ではない）
    1  `--selftest` の食い違い＝この道具自身が壊れている
    3  引数不正・走行ディレクトリが無い・目標定義ファイルが読めない
    4  計測失敗（MEASURE_FAILED）

`2`（判定不能）は返さない——判定不能な走行があれば、それは `4`（計測失敗）である。

ファイルの分かれ方（1 ファイル 1,000 行以下・要件 6.8）
------------------------------------------------------
* `perf-compare.py`（本ファイル）——採否そのもの（目標定義の読取・判定スクリプトの呼び出し・
  判定・書式・入口）。
* `perf_compare_selftest.py`——`--selftest` のハーネス。本体を import せず、本体から
  `SelftestEnv` を受け取る（依存は「本体 → 兄弟」の一方向＝循環しない）。
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tomllib
import unicodedata
from dataclasses import dataclass, field
from pathlib import Path

# 兄弟モジュール（`perf_compare_selftest.py`）は本ファイルと同じディレクトリに在る。
# 依存は常に「本体 → 兄弟」の一方向で、兄弟へは起動時に `SelftestEnv` で本体を渡す
# （本ファイルの名前にはハイフンがあり、兄弟からは import できない＝循環しない）。
sys.path.insert(0, str(Path(__file__).resolve().parent))

import perf_compare_selftest  # noqa: E402

# --- 定数（較正値・語彙）-----------------------------------------------------

#: 採否の版。判定規則か書式を変えたら上げる（fixture の期待出力も一緒に変わる）。
SCRIPT_VERSION = "0.1.0"

STDERR_PREFIX = "perf-compare:"

EXIT_OK = 0
EXIT_FAIL = 1
EXIT_BAD_INPUT = 3
EXIT_MEASURE_FAILED = 4

#: 採否の語彙（設計 C10・`perf-ledger.py` の VERDICTS と同じ綴り）。
VERDICT_ADOPTED = "ADOPTED"
VERDICT_NO_DIFF = "NO_DIFF"
VERDICT_WORSE = "WORSE"
VERDICT_MEASURE_FAILED = "MEASURE_FAILED"

#: 副指標を率で見るときの許容（設計 C10「p95 は +5% 以内」）。
SECONDARY_RATIO_TOLERANCE = 0.05
#: 率で見る指標の末尾（それ以外は増減で見る＝許容率なし）。
SECONDARY_RATIO_SUFFIXES = ("_ms", "_pct")
#: 表の `rule` 列の綴り（`limit` 列がその規則で出した上限である）。
SECONDARY_RULE_LABELS = {"ratio": "<=+5%", "count": "no-increase"}

#: 主指標まわりの小数桁（台帳の `<x.xx>` と同じ）。
PCT_DECIMALS = 2
#: 丸めたあとの比較に使う遊び。桁を落としてもなお残る 2 進の誤差だけを吸う。
COMPARE_EPS = 1e-9

#: 判定スクリプトが「値を出せない」ときに書く綴り（judge-perf.py の METRIC_UNAVAILABLE）。
METRIC_UNAVAILABLE = "-"
#: 判定スクリプトの版を名乗る指標（`[goal].judge_version` と突き合わせる）。
JUDGE_VERSION_METRIC = "script_version"
#: 合否には載せないが必ず並べて記録する指標（要件 5.4）。
TALK_PEAK_METRIC = "talk_peak_cpu_pct"

#: 表と `secondary` 行で使う短い名前。知らない指標は名前をそのまま使う。
METRIC_LABELS = {
    "steady_idle_cpu_mean_pct": "idle_cpu",
    "frame_interval_p95_ms": "p95_ms",
    "catchup_count": "catchup",
    "alloc_count": "allocs",
    TALK_PEAK_METRIC: "talk_peak",
}

#: 走行の並び（`[levels].ab_sequence` と同じ順で採る）。
RUN_NAMES = ("a1", "a2", "b1", "b2")
A_RUNS = ("a1", "a2")
B_RUNS = ("b1", "b2")

#: 判定スクリプトの既定の場所（`[goal].judge_script` が無いときに使う）。
DEFAULT_JUDGE_SCRIPT = "tools/perf/judge-perf.py"

#: 走行ディレクトリの中の成果物。
RUN_LOG_FILENAME = "run.log"
CPU_CSV_FILENAME = "cpu.csv"
RUN_META_FILENAME = "run-meta.txt"

OUTPUT_TEXT_FILENAME = "compare.txt"
OUTPUT_JSON_FILENAME = "compare.json"

#: 自己較正の置き場（`perf-rank.py` と同じ並び）。
SELFTEST_FIXTURES_DIRNAME = "fixtures-loop"
SELFTEST_COMPARE_SUBDIR = "compare"
#: 「緑しか作れない自己較正」を禁じるための赤の下限（1 件も無ければ NG）。
SELFTEST_REQUIRED_RED_EXITS = (EXIT_MEASURE_FAILED,)

#: 結果の 1 行（背景タスクの終了で会話へ届く行・要件 1.9）。
RESULT_LINE_PREFIX = "PERF-COMPARE RESULT"


# --- 失敗の運び方 ------------------------------------------------------------


class CompareError(Exception):
    """文言つきの終了コードで止める（黙って続けない・要件 2.11）。"""

    def __init__(self, code: int, message: str, details: list[str] | None = None):
        super().__init__(message)
        self.code = code
        self.message = message
        self.details = details or []


def bad_input(message: str, details: list[str] | None = None) -> CompareError:
    return CompareError(EXIT_BAD_INPUT, message, details)


# --- 目標定義ファイル --------------------------------------------------------


@dataclass
class Goal:
    """`perf-compare.py` が読む節だけを取り出した目標定義（設計 C1）。"""

    name: str
    path: Path
    primary_metric: str
    noise_floor_pct: float
    must_not_regress: list[str]
    iteration_build: str | None
    judge_version: str | None
    judge_script: str


def script_dir() -> Path:
    return Path(__file__).resolve().parent


def default_repo_root() -> Path:
    """`tools/perf/` の 2 つ上がリポジトリ根（`judge_script` の相対の基点）。"""
    return script_dir().parent.parent


def load_goal(path: Path) -> Goal:
    try:
        raw = tomllib.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise bad_input(f"目標定義ファイルを読めません: {path}", [str(exc)]) from exc
    except tomllib.TOMLDecodeError as exc:
        raise bad_input(f"目標定義ファイルの書式が壊れています: {path}", [str(exc)]) from exc

    primary = raw.get("primary_metric") or {}
    secondary = raw.get("secondary_metrics") or {}
    levels = raw.get("levels") or {}
    goal = raw.get("goal") or {}

    name = primary.get("name")
    if not isinstance(name, str) or not name:
        raise bad_input(
            f"{path}: [primary_metric].name がありません",
            ["主指標の名前が無ければ、何を比べているのか決まりません。"],
        )
    floor = primary.get("noise_floor_pct")
    if not isinstance(floor, (int, float)):
        raise bad_input(
            f"{path}: [primary_metric].noise_floor_pct がありません",
            ["ばらつきの床が無いと、実測のばらつきが 0 のとき何もかも採用になります。"],
        )
    must = secondary.get("must_not_regress", [])
    if not isinstance(must, list) or any(not isinstance(m, str) for m in must):
        raise bad_input(f"{path}: [secondary_metrics].must_not_regress は文字列の配列です")

    return Goal(
        name=str(goal.get("name") or path.stem),
        path=path,
        primary_metric=name,
        noise_floor_pct=float(floor),
        must_not_regress=list(must),
        iteration_build=levels.get("iteration_build"),
        judge_version=goal.get("judge_version"),
        judge_script=str(goal.get("judge_script") or DEFAULT_JUDGE_SCRIPT),
    )


# --- 判定スクリプトの呼び出しと読み取り --------------------------------------

#: `名前=` を見つける正規表現（judge-perf.py `_FIELD_KEY_RE` の写し・出典を残す）。
_FIELD_KEY_RE = re.compile(r"(?:^|\s)([A-Za-z_][A-Za-z0-9_]*)=")


def parse_fields(tail: str) -> dict[str, str]:
    """`名前=値` の並びを辞書にする（judge-perf.py `parse_fields` の写し）。

    値は次の名前の直前まで（空白を含んでよい）。同じ名前が 2 度出たら後勝ちである。
    写しであるから、judge-perf.py 側の読み口を変えたときはここも一緒に変えること。
    """
    keys = list(_FIELD_KEY_RE.finditer(tail))
    fields: dict[str, str] = {}
    for i, m in enumerate(keys):
        start = m.end()
        end = keys[i + 1].start() if i + 1 < len(keys) else len(tail)
        fields[m.group(1)] = tail[start:end].strip()
    return fields


def parse_metric_lines(stdout: str) -> dict[str, str]:
    """`metric=<名前> value=<値>` の行だけを拾う（`--emit-metrics` の読み口）。

    行頭が `metric=` のものしか見ない。レポート本文にも `名前=値` の並びは沢山あるので、
    行全体を無選別に読むと集計の途中の数が指標に化ける。
    """
    metrics: dict[str, str] = {}
    for raw in stdout.splitlines():
        line = raw.strip()
        if not line.startswith("metric="):
            continue
        fields = parse_fields(line)
        name, value = fields.get("metric"), fields.get("value")
        if name:
            metrics[name] = value if value is not None else METRIC_UNAVAILABLE
    return metrics


@dataclass
class RunResult:
    """走行 1 本の集計結果（判定不能ならその理由）。"""

    name: str
    directory: Path
    judge_exit: int | None = None
    metrics: dict[str, str] = field(default_factory=dict)
    undecidable_reason: str | None = None

    @property
    def decidable(self) -> bool:
        return self.undecidable_reason is None

    def raw(self, metric: str) -> str:
        return self.metrics.get(metric, METRIC_UNAVAILABLE)

    def number(self, metric: str) -> float | None:
        return to_number(self.raw(metric))


def to_number(value: str | None) -> float | None:
    """指標の値を数にする。`-`（測っていない）と読めない綴りは None。"""
    if value is None or value == METRIC_UNAVAILABLE:
        return None
    try:
        return float(value)
    except ValueError:
        return None


def measure_run(name: str, directory: Path, goal: Goal, judge: Path,
                build: str | None, timeout_sec: float) -> RunResult:
    """走行 1 本を `judge-perf.py --mode baseline --emit-metrics` で集計する。"""
    result = RunResult(name=name, directory=directory)

    run_log = directory / RUN_LOG_FILENAME
    cpu_csv = directory / CPU_CSV_FILENAME
    missing = [p.name for p in (run_log, cpu_csv) if not p.is_file()]
    if missing:
        result.undecidable_reason = f"走行の成果物がありません: {'・'.join(missing)}"
        return result

    argv = [sys.executable, str(judge), str(run_log), str(cpu_csv),
            "--mode", "baseline", "--emit-metrics"]
    if build:
        argv += ["--build", build]
    meta = directory / RUN_META_FILENAME
    if meta.is_file():
        argv += ["--meta", str(meta)]

    try:
        completed = subprocess.run(
            argv, capture_output=True, text=True, encoding="utf-8", errors="replace",
            timeout=timeout_sec, check=False,
        )
    except OSError as exc:
        result.undecidable_reason = f"判定スクリプトを起動できません: {exc}"
        return result
    except subprocess.TimeoutExpired:
        result.undecidable_reason = f"判定スクリプトが {timeout_sec:.0f} 秒で終わりません"
        return result

    result.judge_exit = completed.returncode
    result.metrics = parse_metric_lines(completed.stdout)

    # 終了コード 1（不合格）は判定不能ではない——合否が付いたなら数値は採れている。
    if completed.returncode in (2, 3):
        word = "判定不能" if completed.returncode == 2 else "引数不正・読取不能"
        result.undecidable_reason = (
            f"判定スクリプトが終了コード {completed.returncode}（{word}）で終わりました"
        )
        return result

    if not result.metrics:
        result.undecidable_reason = (
            "主要指標の行が 1 本も出ていません"
            "（判定スクリプトが --emit-metrics に対応していない可能性）"
        )
        return result

    if goal.judge_version:
        found = result.raw(JUDGE_VERSION_METRIC)
        if found != goal.judge_version:
            result.undecidable_reason = (
                f"判定スクリプトの版が違います: 目標定義 {goal.judge_version} ／ 走行 {found}"
            )
            return result

    if result.number(goal.primary_metric) is None:
        result.undecidable_reason = (
            f"主指標 {goal.primary_metric} の値が {result.raw(goal.primary_metric)!r} です"
            "（測れていません）"
        )
    return result


# --- 判定 --------------------------------------------------------------------


def round_pct(value: float) -> float:
    return round(value, PCT_DECIMALS)


def mean(values: list[float]) -> float:
    return sum(values) / len(values)


def side_mean(runs: dict[str, RunResult], names: tuple[str, ...],
              metric: str) -> float | None:
    """片側（A または B）の平均。1 本でも値が無ければ None（0 で埋めない）。"""
    values = [runs[n].number(metric) for n in names]
    if any(v is None for v in values):
        return None
    return mean([v for v in values if v is not None])


@dataclass
class SecondaryResult:
    name: str
    label: str
    rule: str            # "ratio" / "count"
    mean_a: float | None
    mean_b: float | None
    status: str          # "同等" / "改善" / "悪化" / "NA"
    threshold: float | None


def label_of(metric: str) -> str:
    return METRIC_LABELS.get(metric, metric)


def secondary_rule(metric: str) -> str:
    return "ratio" if metric.endswith(SECONDARY_RATIO_SUFFIXES) else "count"


def judge_secondary(metric: str, mean_a: float | None,
                    mean_b: float | None) -> SecondaryResult:
    """副指標 1 つの前後を比べる。片側でも `-` なら NA（採用を止めない）。"""
    rule = secondary_rule(metric)
    if mean_a is None or mean_b is None:
        return SecondaryResult(metric, label_of(metric), rule, mean_a, mean_b, "NA", None)
    if rule == "ratio":
        threshold = mean_a * (1.0 + SECONDARY_RATIO_TOLERANCE)
        worse = mean_b > threshold + COMPARE_EPS
    else:
        threshold = mean_a
        worse = mean_b > threshold + COMPARE_EPS
    if worse:
        status = "悪化"
    elif mean_b < mean_a - COMPARE_EPS:
        status = "改善"
    else:
        status = "同等"
    return SecondaryResult(metric, label_of(metric), rule, mean_a, mean_b, status, threshold)


def primary_of(run: RunResult, goal: Goal) -> float:
    """判定できる走行の主指標を数で返す。None なら道具の側の食い違いである。

    `or 0.0` で埋めてはならない——主指標が空の走行は `measure_run` が必ず判定不能に
    しているので、ここへ None が来るのは「判定不能でないのに主指標が無い」という
    あり得ない状態である。0.0 で埋めると、それが **最良の CPU** として採用側の
    数字に化ける（測っていない走行が「改善した走行」になる）。黙って埋めずに止める。
    """
    value = run.number(goal.primary_metric)
    if value is None:
        raise CompareError(
            EXIT_MEASURE_FAILED,
            f"判定できるはずの走行 {run.name} に主指標 {goal.primary_metric} がありません",
            ["道具の側の食い違いです（0 で埋めずに止めます）。", str(run.directory)],
        )
    return value


@dataclass
class Decision:
    verdict: str
    mean_a: float | None
    mean_b: float | None
    delta: float | None
    noise: float | None
    #: A1 と A2 の主指標の差（`|A1−A2|`）。ばらつきの由来を表に書くために持つ。
    spread: float | None
    secondaries: list[SecondaryResult]
    worse_secondaries: list[str]
    undecidable: list[tuple[str, str]]
    reason: str

    @property
    def exit_code(self) -> int:
        return EXIT_MEASURE_FAILED if self.verdict == VERDICT_MEASURE_FAILED else EXIT_OK


def decide(runs: dict[str, RunResult], goal: Goal) -> Decision:
    """設計 C10 の判定をそのまま書き下す（順序が判定そのものである）。"""
    undecidable = [
        (name, runs[name].undecidable_reason or "")
        for name in RUN_NAMES if not runs[name].decidable
    ]

    a_ok = all(runs[n].decidable for n in A_RUNS)
    b_ok = all(runs[n].decidable for n in B_RUNS)

    mean_a = round_pct(mean([primary_of(runs[n], goal) for n in A_RUNS])) if a_ok else None
    mean_b = round_pct(mean([primary_of(runs[n], goal) for n in B_RUNS])) if b_ok else None

    noise: float | None = None
    spread: float | None = None
    if a_ok:
        spread = abs(primary_of(runs["a1"], goal) - primary_of(runs["a2"], goal))
        noise = round_pct(max(spread, goal.noise_floor_pct))
    delta = round_pct(mean_b - mean_a) if (mean_a is not None and mean_b is not None) else None

    secondaries = [
        judge_secondary(
            metric,
            side_mean(runs, A_RUNS, metric) if a_ok else None,
            side_mean(runs, B_RUNS, metric) if b_ok else None,
        )
        for metric in goal.must_not_regress
    ]
    worse = [s.name for s in secondaries if s.status == "悪化"]

    if undecidable:
        names = "・".join(name for name, _ in undecidable)
        return Decision(
            VERDICT_MEASURE_FAILED, mean_a, mean_b, delta, noise, spread, secondaries,
            worse, undecidable, f"判定不能の走行があります（{names}）。3 本で決めない",
        )

    assert delta is not None and noise is not None  # 判定不能が無ければ 4 本とも数がある
    if worse:
        reason = "副指標が悪化した（" + "・".join(label_of(n) for n in worse) + "）"
        verdict = VERDICT_WORSE
    elif delta <= -noise + COMPARE_EPS:
        verdict = VERDICT_ADOPTED
        reason = f"差 {delta:+.2f} ≤ −ばらつき {noise:.2f} で、副指標に悪化が無い"
    elif delta < noise - COMPARE_EPS:
        verdict = VERDICT_NO_DIFF
        reason = f"差 {delta:+.2f} はばらつき {noise:.2f} の内側"
    else:
        verdict = VERDICT_WORSE
        reason = f"差 {delta:+.2f} ≥ ばらつき {noise:.2f}"
    return Decision(verdict, mean_a, mean_b, delta, noise, spread, secondaries, worse,
                    [], reason)


# --- 書式 --------------------------------------------------------------------


def fmt_pct(value: float | None) -> str:
    return METRIC_UNAVAILABLE if value is None else f"{value:.{PCT_DECIMALS}f}"


#: 測れていない数の書き方。`-` だけだと「0 だった」と読み違えられる。
UNMEASURED_CELL = f"{METRIC_UNAVAILABLE}（測れていない）"


def fmt_pct_unit(value: float | None) -> str:
    return UNMEASURED_CELL if value is None else f"{value:.{PCT_DECIMALS}f} %"


def fmt_secondary_mean(metric: str, value: float | None) -> str:
    """副指標の平均。件数は整数なら整数で（0.0 と書くと 0 件が薄く見える）。"""
    if value is None:
        return METRIC_UNAVAILABLE
    if secondary_rule(metric) == "count" and float(value).is_integer():
        return str(int(value))
    return f"{value:.3f}"


def secondary_ledger_line(secondaries: list[SecondaryResult]) -> str:
    """台帳の `secondary:` に載る 1 行（設計 C11 の見本と同じ形）。"""
    return ", ".join(
        f"{s.label}={fmt_secondary_mean(s.name, s.mean_a)}"
        f"/{fmt_secondary_mean(s.name, s.mean_b)}"
        for s in secondaries
    )


def disp_width(text: str) -> int:
    """等幅で表示したときの桁数。全角は 2 桁と数える。

    `len()` で桁を数えると、日本語の入った列だけが左へずれる（「増えない」は 4 文字だが
    8 桁を占める）。表は人が縦に目で追うためのものなので、桁は表示幅で数える。
    """
    return sum(2 if unicodedata.east_asian_width(ch) in ("W", "F") else 1 for ch in text)


def pad(text: str, width: int, left: bool) -> str:
    fill = " " * max(0, width - disp_width(text))
    return text + fill if left else fill + text


def section(title: str) -> str:
    """節の見出し。全体で 78 桁になるまで罫線を伸ばす。"""
    head = f"-- {title} "
    return head + "-" * max(3, 78 - disp_width(head))


def render_table(headers: list[str], rows: list[list[str]],
                 align_left: set[int]) -> list[str]:
    """固定幅の表（桁は表示幅で数える）。"""
    widths = [disp_width(h) for h in headers]
    for row in rows:
        for i, cell in enumerate(row):
            widths[i] = max(widths[i], disp_width(cell))

    def line(cells: list[str]) -> str:
        parts = [pad(cells[i], widths[i], i in align_left) for i in range(len(cells))]
        return "  " + "  ".join(parts).rstrip()

    return [line(headers), line(["-" * w for w in widths])] + [line(r) for r in rows]


def render_report(runs: dict[str, RunResult], goal: Goal, decision: Decision,
                  judge_version: str) -> list[str]:
    lines = [
        "=" * 78,
        f"採否 perf-compare.py {SCRIPT_VERSION}（設計 C10・要件 1.7／3.6／4.6）",
        "=" * 78,
        f"  目標            : {goal.name}",
        f"  目標定義        : {goal.path.name}",
        f"  主指標          : {goal.primary_metric}（小さいほど良い・単位 %）",
        f"  ばらつきの床    : {goal.noise_floor_pct:.2f} %",
        f"  判定スクリプト  : judge-perf.py {judge_version}"
        "（--mode baseline を別プロセスで 4 回）",
        f"  採否の走行ビルド: {goal.iteration_build or '(指定なし)'}",
        "  走行の並び      : A1 → B1 → A2 → B2"
        "（この表は基底名だけを書く。絶対パスは compare.json にある）",
        "",
        section("走行ごとの数値"),
    ]

    secondary_metrics = [s.name for s in decision.secondaries]
    # 発話の頂は必ず並べるが、目標定義が副指標に挙げていれば既に列が在る。
    # 同じ名前の列を 2 度出さない（要件 2.12 と同じ規律——名前が重なった表は読めない）。
    talk_peak_judged = TALK_PEAK_METRIC in secondary_metrics
    headers = ["run", "dir", label_of(goal.primary_metric)]
    headers += [label_of(m) for m in secondary_metrics]
    if not talk_peak_judged:
        headers.append(label_of(TALK_PEAK_METRIC))
    headers.append("judge")
    rows = []
    for name in RUN_NAMES:
        run = runs[name]
        row = [name.upper(), run.directory.name, run.raw(goal.primary_metric)]
        row += [run.raw(m) for m in secondary_metrics]
        if not talk_peak_judged:
            row.append(run.raw(TALK_PEAK_METRIC))
        row.append(METRIC_UNAVAILABLE if run.judge_exit is None else str(run.judge_exit))
        rows.append(row)
    lines += render_table(headers, rows, align_left={0, 1})
    lines.append("")
    lines.append(
        f"  {label_of(TALK_PEAK_METRIC)} はこの目標定義では副指標なので、合否に効く"
        "（要件 5.4 の「並べるだけ」ではない）。"
        if talk_peak_judged else
        f"  {label_of(TALK_PEAK_METRIC)} は並べて記録するだけで、合否には載せない"
        "（要件 5.4）。"
    )

    lines += ["", section("差とばらつき")]
    if decision.noise is not None and decision.spread is not None:
        lines.append(
            f"  ばらつき noise  : {fmt_pct_unit(decision.noise)}"
            f"（max(|A1−A2|={decision.spread:.2f}, 床 {goal.noise_floor_pct:.2f})）"
        )
    else:
        lines.append(f"  ばらつき noise  : {UNMEASURED_CELL}（A 側に判定不能の走行がある）")
    lines += [
        f"  平均 A（変更前）: {fmt_pct_unit(decision.mean_a)}",
        f"  平均 B（変更後）: {fmt_pct_unit(decision.mean_b)}",
        f"  差 delta        : {UNMEASURED_CELL}（B − A・負が改善）"
        if decision.delta is None
        else f"  差 delta        : {decision.delta:+.2f} %（B − A・負が改善）",
    ]

    lines += ["", section("副指標（悪化があれば主指標の改善より優先する）")]
    if decision.secondaries:
        sec_headers = ["metric", "A", "B", "rule", "limit", "verdict"]
        sec_rows = []
        for s in decision.secondaries:
            limit = (
                METRIC_UNAVAILABLE if s.threshold is None
                else fmt_secondary_mean(s.name, s.threshold)
            )
            rule = SECONDARY_RULE_LABELS[s.rule]
            sec_rows.append([
                s.name,
                fmt_secondary_mean(s.name, s.mean_a),
                fmt_secondary_mean(s.name, s.mean_b),
                rule, limit, s.status,
            ])
        lines += render_table(sec_headers, sec_rows, align_left={0, 3, 5})
        if any(s.status == "NA" for s in decision.secondaries):
            lines.append(
                "  NA は片側でも測っていない指標。悪化と断じない代わりに、"
                "見落とさないようここに名前を出す。"
            )
    else:
        lines.append("  （目標定義に must_not_regress が 1 つも無い）")

    lines += ["", section("採否"), f"  verdict = {decision.verdict}",
              f"  理由    : {decision.reason}"]
    for name, reason in decision.undecidable:
        lines.append(f"  判定不能: {name} — {reason}")
    lines.append("")
    return lines


def build_json(runs: dict[str, RunResult], goal: Goal, decision: Decision,
               judge: Path, judge_version: str) -> dict:
    payload: dict = {
        "script_version": SCRIPT_VERSION,
        "verdict": decision.verdict,
        "exit_code": decision.exit_code,
        "reason": decision.reason,
        # --- 台帳（perf-ledger.py append）がそのまま読む鍵（設計 C11）---
        "before_idle_cpu_pct": decision.mean_a,
        "after_idle_cpu_pct": decision.mean_b,
        "delta_pct": decision.delta,
        "noise_pct": decision.noise,
        "secondary": secondary_ledger_line(decision.secondaries),
        # --- 判定の材料 ---
        "noise_floor_pct": goal.noise_floor_pct,
        "primary_metric": goal.primary_metric,
        "must_not_regress": list(goal.must_not_regress),
        "worse_secondaries": list(decision.worse_secondaries),
        "secondary_detail": [
            {
                "name": s.name, "label": s.label, "rule": s.rule,
                "mean_a": s.mean_a, "mean_b": s.mean_b,
                "threshold": s.threshold, "status": s.status,
            }
            for s in decision.secondaries
        ],
        "undecidable": [{"run": name, "reason": reason}
                        for name, reason in decision.undecidable],
        "goal_name": goal.name,
        "goal_file": str(goal.path),
        "iteration_build": goal.iteration_build,
        "judge_script": str(judge),
        "judge_version": judge_version,
    }
    for name in RUN_NAMES:
        run = runs[name]
        payload[name] = {
            "dir": str(run.directory),
            "judge_exit": run.judge_exit,
            "primary": run.number(goal.primary_metric),
            "secondaries": {m: run.raw(m) for m in goal.must_not_regress},
            "talk_peak": run.raw(TALK_PEAK_METRIC),
            "undecidable_reason": run.undecidable_reason,
        }
    return payload


# --- 本体 --------------------------------------------------------------------


def resolve_goal_path(args: argparse.Namespace, repo_root: Path) -> Path:
    if args.goal_file:
        path = Path(args.goal_file)
    elif args.goal:
        path = repo_root / "tools" / "perf" / "goals" / f"{args.goal}.toml"
    else:
        raise bad_input("--goal（名前）か --goal-file（パス）のどちらかが要ります")
    if not path.is_file():
        raise bad_input(f"目標定義ファイルがありません: {path}")
    return path.resolve()


def resolve_judge(args: argparse.Namespace, goal: Goal, repo_root: Path) -> Path:
    path = (Path(args.judge) if args.judge else repo_root / goal.judge_script).resolve()
    if not path.is_file():
        raise bad_input(
            f"判定スクリプトがありません: {path}",
            ["--judge <パス> で場所を渡せます。"],
        )
    return path


def compare(args: argparse.Namespace) -> int:
    repo_root = Path(args.repo_root).resolve() if args.repo_root else default_repo_root()
    goal = load_goal(resolve_goal_path(args, repo_root))
    judge = resolve_judge(args, goal, repo_root)
    build = args.build or goal.iteration_build

    directories = dict(
        zip(RUN_NAMES, [Path(p).resolve() for p in (list(args.a) + list(args.b))])
    )
    for name, directory in directories.items():
        if not directory.is_dir():
            raise bad_input(f"走行ディレクトリがありません（{name}）: {directory}")

    runs = {
        name: measure_run(name, directory, goal, judge, build, args.timeout)
        for name, directory in directories.items()
    }
    decision = decide(runs, goal)

    versions = {r.raw(JUDGE_VERSION_METRIC) for r in runs.values()} - {METRIC_UNAVAILABLE}
    judge_version = versions.pop() if len(versions) == 1 else (
        goal.judge_version or METRIC_UNAVAILABLE
    )

    out_dir = Path(args.out_dir) if args.out_dir else Path.cwd()
    try:
        out_dir.mkdir(parents=True, exist_ok=True)
        text_path = out_dir / OUTPUT_TEXT_FILENAME
        json_path = out_dir / OUTPUT_JSON_FILENAME
        text_path.write_text(
            "\n".join(render_report(runs, goal, decision, judge_version)),
            encoding="utf-8", newline="\n",
        )
        json_path.write_text(
            json.dumps(build_json(runs, goal, decision, judge, judge_version),
                       ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8", newline="\n",
        )
    except OSError as exc:
        raise bad_input(f"結果を書けません: {out_dir}", [str(exc)]) from exc

    if decision.verdict == VERDICT_MEASURE_FAILED:
        sys.stderr.write(f"{STDERR_PREFIX} 計測失敗: {decision.reason}\n")
        for name, reason in decision.undecidable:
            sys.stderr.write(f"  - {name}: {reason}\n")

    delta = METRIC_UNAVAILABLE if decision.delta is None else f"{decision.delta:+.2f}"
    print(
        f"{RESULT_LINE_PREFIX} verdict={decision.verdict} delta={delta} "
        f"noise={fmt_pct(decision.noise)} code={decision.exit_code} dir={out_dir}"
    )
    return decision.exit_code


# --- 自己較正（ハーネスは perf_compare_selftest.py・本体は道具立てを渡すだけ）----


def run_selftest() -> int:
    return perf_compare_selftest.run_selftest(
        perf_compare_selftest.SelftestEnv(
            fixtures_root=script_dir() / SELFTEST_FIXTURES_DIRNAME / SELFTEST_COMPARE_SUBDIR,
            invoke=main,
            bad_input=bad_input,
            required_red_exits=SELFTEST_REQUIRED_RED_EXITS,
            text_filename=OUTPUT_TEXT_FILENAME,
            json_filename=OUTPUT_JSON_FILENAME,
            unavailable=METRIC_UNAVAILABLE,
            pct_decimals=PCT_DECIMALS,
            ok_exit=EXIT_OK,
            fail_exit=EXIT_FAIL,
        )
    )


# --- 入口 --------------------------------------------------------------------


class Parser(argparse.ArgumentParser):
    """引数不正の終了コードを 3 に揃える（argparse の既定 2 は使わない）。"""

    def error(self, message: str):  # type: ignore[override]
        sys.stderr.write(f"{STDERR_PREFIX} 引数が不正です: {message}\n")
        sys.stderr.write(self.format_usage())
        raise SystemExit(EXIT_BAD_INPUT)


def build_parser() -> Parser:
    parser = Parser(
        prog="perf-compare.py",
        description="A→B→A→B の 4 本から差とばらつきを出し採否を返す（要件 1.7・3.6・4.6・6.7）",
    )
    parser.add_argument("--a", nargs=2, metavar=("A1", "A2"),
                        help="変更前の走行ディレクトリ 2 本")
    parser.add_argument("--b", nargs=2, metavar=("B1", "B2"),
                        help="変更後の走行ディレクトリ 2 本")
    parser.add_argument("--goal", help="目標の名前（tools/perf/goals/<名前>.toml を読む）")
    parser.add_argument("--goal-file", help="目標定義ファイルのパス（--goal の代わり）")
    parser.add_argument("--build", choices=("dev", "release"),
                        help="判定スクリプトへ渡すビルド種別（既定は [levels].iteration_build）")
    parser.add_argument("--judge", help="判定スクリプトのパス（既定は [goal].judge_script）")
    parser.add_argument("--repo-root", help="リポジトリ根（既定は tools/perf の 2 つ上）")
    parser.add_argument("--out-dir",
                        help=f"{OUTPUT_TEXT_FILENAME}／{OUTPUT_JSON_FILENAME} の書き先"
                        "（既定はカレント）")
    parser.add_argument("--timeout", type=float, default=300.0,
                        help="判定スクリプト 1 本あたりの上限秒（既定 300）")
    parser.add_argument("--selftest", action="store_true",
                        help=f"自己較正（{SELFTEST_FIXTURES_DIRNAME}/{SELFTEST_COMPARE_SUBDIR}/）")
    return parser


def main(argv: list[str]) -> int:
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[union-attr]
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[union-attr]
    except (AttributeError, OSError):
        pass

    args = build_parser().parse_args(argv)
    try:
        if args.selftest:
            return run_selftest()
        if not args.a or not args.b:
            raise bad_input(
                "--a <A1> <A2> と --b <B1> <B2> の両方が要ります",
                ["同じ形を 2 回はさむ並び（A1 → B1 → A2 → B2）が判定の前提です。"],
            )
        return compare(args)
    except CompareError as exc:
        label = {EXIT_BAD_INPUT: "入力不正", EXIT_MEASURE_FAILED: "計測失敗"}.get(exc.code, "失敗")
        sys.stderr.write(f"{STDERR_PREFIX} {label}: {exc.message}\n")
        for detail in exc.details:
            sys.stderr.write(f"  - {detail}\n")
        return exc.code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
