#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""areka 性能計測 判定スクリプト（集計モード）。

`invoke-perf-run.ps1` が採取した実走ログ（run.log）と CPU 時系列（cpu.csv）を読み、
段階別所要・キャッシュ命中率・コマ適用間隔・確保計数・進行境界スキップ件数・CPU 統計・
合成キーの異なり数を集計してレポートを出す（要件 2.2・7.2）。

このスクリプトは Python 標準ライブラリだけで動く。リポジトリのビルドグラフには入らない
（design.md「Boundary Commitments / Allowed Dependencies」）。

使い方
------
    python judge-perf.py <run.log> <cpu.csv> --mode baseline [--build dev|release] [--meta <run-meta.txt>]

`--mode verdict`（合否判定）は task 2.3、`--selftest`（自己較正）は task 2.4 で追加する。
本ファイルは集計モードだけを実装しており、未実装のモードを求められたら黙って別のことを
するのではなく、未実装であることを述べて終了する。

終了コード（design.md「Error Handling」・D7・signoff-scan.py 前例踏襲）
--------------------------------------------------------------------
    0  正常終了（集計モードは常にこれか 2 か 3）
    1  FAIL（合否判定モード専用。集計モードは 1 を返さない）
    2  判定不能——必要ログ種の欠落・段フィールドの欠落・観測ゼロ。
       部分集計は一切出さない（要件 2.5。沈黙を合格として読ませない）
    3  引数不正・ファイル読取不能

run-meta.txt の所在（要件 4.5）
------------------------------
既定では `cpu.csv` と同じディレクトリの `run-meta.txt` を自動で探す
（`invoke-perf-run.ps1` が run.log・cpu.csv・run-meta.txt を同一の出力先へ並べて置くため）。
`--meta` を渡せばその位置を上書きできる。マシン条件はレポート先頭に必ず併記するので、
run-meta.txt が見つからない走行は「条件を書けない集計」として exit 2 で拒否する。

ログ行の形（実測で確定した契約面）
--------------------------------
areka は `tracing_subscriber::fmt().with_env_filter(..).init()` で初期化する
（`crates/areka/src/main.rs:260-264`）。この既定書式が出す 1 行は次の形である：

    <時刻>Z <水準> <スパン>: <target>: <メッセージ> <名前>=<値> <名前>=<値> ...

実測サンプル（本スクリプトの開発時に、同一バージョンの tracing 0.1.44 /
tracing-subscriber 0.3.23 を同一の初期化で走らせて採取した逐語）:

    2026-08-14T04:17:25.043998Z DEBUG actor{actor="emo-text"}: \
    areka_emo_present::presenter::timing: perf(apply_show): 段階別計時 \
    target_id=TargetId(0) surface_id=1000 cache_hit=false t_cache_us=83 ...

【重要】色付け（ANSI エスケープ）は既定で **有効** である。
`tracing-subscriber-0.3.23/src/fmt/fmt_layer.rs:739-755` の `Default for Layer` は
「ansi feature が有効で、かつ環境変数 `NO_COLOR` が未設定または空なら色を付ける」と
決めており、出力先が端末かどうかを見ていない。`invoke-perf-run.ps1` は `NO_COLOR` を
設定しないため、ファイルへリダイレクトした run.log にも色コードが入る。
本スクリプトは読み込み時に ANSI エスケープを除去してから解析する（除去後の行が
`NO_COLOR=1` で採取した行と逐語一致することを実測で確認済み）。
"""

from __future__ import annotations

import argparse
import csv
import math
import re
import sys
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from pathlib import Path

# =============================================================================
# 較正値バナー（要件 2.6 / 4.5 / 5.3）
# -----------------------------------------------------------------------------
# 本スクリプトが使う値は **すべてここにある**。README・task 2.3（合否判定）・
# task 2.4（自己較正）はこのバナーを唯一の所在として参照する。値を変えるときはここだけを
# 書き換える。
#
# 【他ゴーストへ流用しないこと】末尾に「emo2 固有」と書いた値は、計測 fixture である
# ゴースト emo2 のアニメ定義・実寸から来ている。別のゴーストで測るときは必ず測り直す。
# =============================================================================

#: 本スクリプトの版（レポート先頭に出す。較正値を変えたら上げる）。
SCRIPT_VERSION = "0.1.0 (task 2.2 / 集計モードのみ)"

# --- コマ適用間隔（要件 4.2⑴ の入力） ---------------------------------------

#: 期待するコマ適用間隔（ミリ秒）。emo2 のまばたきアニメ定義値。**emo2 固有**。
FRAME_INTERVAL_EXPECTED_MS = 172.0

#: 期待値からの許容率。判定は p95 <= 期待値 x (1 + 許容率)（要件 4.2⑴）。
#: 暫定値であり、task 3.1 の第 1 段ベースラインで確定する。
FRAME_INTERVAL_TOLERANCE = 0.15

#: コマ適用間隔の系列の分け方（design.md「較正値台帳」FRAME_INTERVAL_WINDOW）。
#: 同時刻に別対象へ適用が走ると、混ぜた列の差分は「間隔」ではなくなる（重畳による
#: 偽陽性・偽陰性）。ゆえに `target_id` と `surface_id` の組ごとに列を分け、
#: 各列の中で連続する適用の時刻差だけを間隔として数える。
FRAME_INTERVAL_SERIES_KEY = ("target_id", "surface_id")

#: 測定窓の定義。**未確定**であることを明示しておく。
#: design.md の台帳は「アイドル区間（talk 再生・複数アニメ重畳を除外した窓）限定」と
#: 定めているが、窓の境界条件は task 3.1 の第 1 段ベースラインで確定して README へ登記する
#: 手筈になっている。本スクリプトは下の 2 つの窓を **両方** 集計して並べて出す：
#:   "warmup"  : 起動過渡（WARMUP_EXCLUDE_SEC）を除いただけの窓。確実に計算できる
#:   "idle"    : さらに「同時に走っているアニメが FRAME_INTERVAL_IDLE_MAX_ACTIVE 本以下」
#:               の区間だけに絞った窓。seriko の発火・停止 info! の収支から推定する
#: talk 再生の除外は、現在のログ水準（RUST_LOG=info,areka_emo_present=debug）で
#: talk の開始・終了を名指しできる行が無いため **実装していない**。3.1 のベースラインで
#: 目印になる行を決め、FRAME_INTERVAL_IDLE_EXCLUDE_MARKERS へ入れること。
FRAME_INTERVAL_WINDOW = "warmup と idle の 2 通りを併記（idle の境界は task 3.1 で確定）"

#: idle 窓とみなす「同時に走っているアニメの本数」の上限。
#: 1 本（まばたき単独）までをアイドルとみなす暫定値。task 3.1 で確定する。
FRAME_INTERVAL_IDLE_MAX_ACTIVE = 1

#: talk 再生区間を idle 窓から外すための目印（メッセージの一部）。**現在は空**。
#: task 3.1 が第 1 段ベースラインの実走ログを見て確定し、ここへ入れる。
#: 空のままだと talk 中の適用が idle 窓に混ざるため、レポートにその旨を明記する。
FRAME_INTERVAL_IDLE_EXCLUDE_MARKERS: tuple[str, ...] = ()

# --- 定常状態の開始境界 -------------------------------------------------------

#: 起動過渡（初回確保・ウォームアップ）として集計から外す秒数。
#: 実走の最初の観測時刻を 0 秒として数える。暫定値であり task 3.1 で確定する。
WARMUP_EXCLUDE_SEC = 60.0

# --- CPU（要件 4.2⑷ / 5.1） --------------------------------------------------

#: release ビルドのアイドル CPU 上限（1 コア換算パーセント）。
#: 議題 2 裁定（同一ゴーストを表示中の SSP 実測 2.2〜2.8% の同等圏）。判定は task 2.3。
IDLE_CPU_MAX_RELEASE_PCT = 3.0

#: CPU 時系列の想定刻み（秒）。`invoke-perf-run.ps1` の CPU_SAMPLE_INTERVAL_SEC と対。
#: 【重要】ランナーは次回時刻を絶対時刻で積み上げるためドリフトしないが、マシンが詰まって
#: 刻みを飛ばした場合は印を残さずに欠測する。ゆえに本スクリプトは刻みを均等と仮定せず、
#: 行ごとの timestamp の差から実際の間隔を出す。この値は「欠測らしさ」の報告にのみ使う。
CPU_SAMPLE_INTERVAL_EXPECTED_SEC = 15.0

#: 実測間隔がこの倍率を超えたら「刻みを飛ばした疑い」として件数を報告する。
CPU_SAMPLE_GAP_FACTOR = 1.5

#: 収束判定（要件 5.1／判定は task 2.3）。時系列を等分する窓の数。
CONVERGENCE_WINDOW_COUNT = 4

#: 収束判定: 後半窓の回帰傾きの上限（パーセント毎分）。task 3.1 で確定する暫定値。
CONVERGENCE_SLOPE_MAX_PCT_PER_MIN = 0.05

#: 収束判定: 末尾窓平均 − 中間窓平均 の上限（パーセント）。task 3.1 で確定する暫定値。
CONVERGENCE_WINDOW_DIFF_MAX_PCT = 0.3

# --- 分位点の求め方 -----------------------------------------------------------

#: p50/p95 の定義。nearest-rank（昇順に並べ、ceil(q * n) 番目の値をそのまま採る）。
#: 補間しないので、同じ入力からは必ず同じ値が出る（道具の再現性）。
PERCENTILE_METHOD = "nearest-rank（補間なし・ceil(q*n) 番目）"

# --- ログの文言（J_*）--------------------------------------------------------

#: perf サマリ行の固定文言（`presenter/timing.rs:53,206`）。
J_PERF_LINE_MESSAGE = "perf(apply_show): 段階別計時"

#: 表示成立点 info! の固定文言（`presenter/show.rs:359`）。
J_SHOW_LINE_MESSAGE = "apply(ShowSurface): 表示・マスクを更新"

#: 進行境界スキップ（catch-up）の実文言。
#: `ticker.rs:205`（dispatcher）／`ticker.rs:225`（kanade）が前者、
#: `ticker.rs:307`（loop_ticker）が後者。
#: 【重要な落とし穴】後者は前者を部分文字列として **含む**。素朴に「含む行を数える」と
#: loop_ticker の 1 行が両方に数えられて二重計上になる。ゆえに必ず長いほうから先に判定する。
J_CATCHUP_LOOP_MESSAGE = "loop ticker catch-up: skipped multiple boundaries, firing once"
J_CATCHUP_TICKER_MESSAGE = "ticker catch-up: skipped multiple boundaries, firing once"

#: seriko のループ発火・停止 info!（`areka-seriko/src/looper.rs`）。
#: 活性アニメ本数の直接ログは無いため、この収支から推定する（design.md「収束判定」）。
J_SERIKO_FIRE_MESSAGE = "seriko: loop 抽選発火"
J_SERIKO_STOP_MESSAGES = (
    "seriko: loop 末尾残留",  # looper.rs:342 最終コマ保持・再抽選対象へ
    "seriko: loop 停止",  # looper.rs:372 負 surface でベース復帰
    "seriko: loop bind から外れた ID の再生を停止",  # looper.rs:308
)

#: 必要ログ種の一覧（欠けたら部分集計を出さずに exit 2・要件 2.5）。
#: `run.stderr.log` は必要ログ種では **ない**。標準出力と標準エラーは同一ファイルへ流せない
#: ためランナーが別ファイルへ分けているだけであり、欠落扱いにしてはならない。
J_REQUIRED_LOG_KINDS = (
    ("perf サマリ行", J_PERF_LINE_MESSAGE),
    ("表示成立点 info!", J_SHOW_LINE_MESSAGE),
)

#: CPU 時系列 CSV のヘッダ（`invoke-perf-run.ps1` の CSV_HEADER と対）。
J_CPU_CSV_HEADER = ("timestamp", "cpu_percent_1core")

#: perf サマリ行に必ず載るフィールド（design.md「perf サマリ行スキーマ」の 14 個）。
#: 1 つでも欠けた行があれば部分集計を出さずに exit 2（要件 2.5）。
J_PERF_STAGE_FIELDS = (
    "t_cache_us",
    "t_compose_us",
    "t_resample_us",
    "t_mask_us",
    "t_upload_us",
    "t_total_us",
)
J_PERF_ALLOC_FIELDS = (
    "alloc_compose_dst",
    "alloc_resample_dst",
    "alloc_xmap",
    "alloc_mask",
)
J_PERF_IDENT_FIELDS = ("target_id", "surface_id", "cache_hit", "key_hash")
J_PERF_REQUIRED_FIELDS = J_PERF_IDENT_FIELDS + J_PERF_STAGE_FIELDS + J_PERF_ALLOC_FIELDS

#: マシン条件に必ず載せる DPI 構成の出どころ（2.1→2.2 申し送り 2）。
#: run-meta.txt に DPI は無い。run.log の起動時ログ（`areka::placement`）と表示成立点 info!
#: から拾う。合成コストは DPI に比例し、ベースラインは 2 モニタ混在 DPI で採取されるため、
#: DPI を書かないレポートは再現できない。
J_DPI_BOOT_FIELDS = (
    "primary_dpi",
    "shell_author_dpi",
    "balloon_author_dpi",
    "k_shell",
    "k_balloon",
)
J_DPI_SHOW_FIELDS = (
    "author_dpi",
    "window_dpi",
    "k",
    "k_ratio",
    "native_w",
    "native_h",
    "scaled_w",
    "scaled_h",
)

# =============================================================================
# 終了コードと失敗の伝え方
# =============================================================================

EXIT_OK = 0
EXIT_FAIL = 1  # 合否判定モード専用（task 2.3）。集計モードは返さない
EXIT_INCONCLUSIVE = 2
EXIT_BAD_INPUT = 3


class JudgeError(Exception):
    """理由と終了コードを必ず持つ失敗（黙って落ちる経路を作らない）。"""

    def __init__(self, code: int, message: str, details: list[str] | None = None):
        super().__init__(message)
        self.code = code
        self.message = message
        self.details = details or []


def inconclusive(message: str, details: list[str] | None = None) -> JudgeError:
    """判定不能（必要ログ種の欠落・段フィールドの欠落・観測ゼロ）。"""
    return JudgeError(EXIT_INCONCLUSIVE, message, details)


def bad_input(message: str, details: list[str] | None = None) -> JudgeError:
    """引数不正・ファイル読取不能。"""
    return JudgeError(EXIT_BAD_INPUT, message, details)


# =============================================================================
# ログ行の解析
# =============================================================================

#: ANSI エスケープ（CSI シーケンス）。読み込み時に落とす。
_ANSI_RE = re.compile(r"\x1b\[[0-9;?]*[ -/]*[@-~]")

#: 行頭の時刻。`tracing-subscriber` 既定の SystemTime タイマは RFC3339（UTC・小数 6 桁）。
_TIMESTAMP_RE = re.compile(r"^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?)Z\s")

#: `名前=` の出現位置。値は次の `名前=` の直前までとする。
#: `window_dpi=Some((120, 120))` のように値に空白が入る Debug 出力があるため、
#: 空白で切ってはいけない。
_FIELD_KEY_RE = re.compile(r"(?:^|\s)([A-Za-z_][A-Za-z0-9_]*)=")


def strip_ansi(text: str) -> str:
    """ANSI エスケープを落とす。"""
    return _ANSI_RE.sub("", text)


def parse_timestamp(line: str) -> datetime | None:
    """行頭の時刻を返す。時刻で始まらない行（複数行メッセージの続き）は None。"""
    m = _TIMESTAMP_RE.match(line)
    if not m:
        return None
    raw = m.group(1)
    if "." in raw:
        head, frac = raw.split(".", 1)
        frac = (frac + "000000")[:6]
        raw = f"{head}.{frac}"
    else:
        raw = raw + ".000000"
    try:
        return datetime.strptime(raw, "%Y-%m-%dT%H:%M:%S.%f").replace(tzinfo=timezone.utc)
    except ValueError:
        return None


def parse_fields(tail: str) -> dict[str, str]:
    """`名前=値` の並びを辞書にする。値は次の名前の直前まで（空白を含んでよい）。"""
    keys = list(_FIELD_KEY_RE.finditer(tail))
    fields: dict[str, str] = {}
    for i, m in enumerate(keys):
        start = m.end()
        end = keys[i + 1].start() if i + 1 < len(keys) else len(tail)
        fields[m.group(1)] = tail[start:end].strip()
    return fields


def split_after_message(line: str, message: str) -> str | None:
    """固定文言より後ろ（＝フィールドの並び）を返す。文言が無ければ None。"""
    idx = line.find(message)
    if idx < 0:
        return None
    return line[idx + len(message):]


def read_text_lines(path: Path, what: str) -> list[str]:
    """テキストを行に割って返す。ANSI を落とし、行末の CR も落とす。

    読めないファイルは exit 3（引数不正・読取不能）。文字化けは行を捨てずに置換で通す——
    数値もフィールド名も ASCII であり、化けるのは日本語のメッセージ部分だけである
    （途中で切れた最終行が壊れていても、それ以前の観測を捨てない）。
    """
    try:
        raw = path.read_bytes()
    except OSError as exc:
        raise bad_input(f"{what} を読めません: {path}", [str(exc)]) from exc
    text = raw.decode("utf-8", errors="replace")
    if text.startswith("﻿"):
        text = text[1:]
    return [strip_ansi(line.rstrip("\r")) for line in text.split("\n")]


# =============================================================================
# perf サマリ行
# =============================================================================


@dataclass
class PerfRecord:
    """perf サマリ行 1 本（＝表示 1 コマの適用 1 回）。"""

    line_no: int
    at: datetime
    target_id: str
    surface_id: str
    cache_hit: bool
    key_hash: str
    stages: dict[str, int]
    allocs: dict[str, int]

    @property
    def series_key(self) -> tuple[str, str]:
        return (self.target_id, self.surface_id)


@dataclass
class LogScan:
    """run.log を 1 度なめて集めたもの。"""

    perf: list[PerfRecord] = field(default_factory=list)
    show_count: int = 0
    first_at: datetime | None = None
    last_at: datetime | None = None
    catchup: list[tuple[datetime | None, str, str]] = field(default_factory=list)
    seriko_events: list[tuple[datetime | None, str, str, str]] = field(default_factory=list)
    boot_dpi: dict[str, str] = field(default_factory=dict)
    show_dpi_variants: Counter = field(default_factory=Counter)
    idle_exclusion_hits: int = 0


def _catchup_kind(line: str) -> str | None:
    """catch-up 行の種別。長いほう（loop）から順に見る（部分文字列の罠）。"""
    if J_CATCHUP_LOOP_MESSAGE in line:
        return "loop_ticker"
    if J_CATCHUP_TICKER_MESSAGE in line:
        return "ticker"
    return None


def _seriko_kind(line: str) -> str | None:
    if J_SERIKO_FIRE_MESSAGE in line:
        return "fire"
    for stop in J_SERIKO_STOP_MESSAGES:
        if stop in line:
            return "stop"
    return None


def scan_log(path: Path) -> LogScan:
    """run.log を 1 度だけなめて必要なものを全部集める。"""
    lines = read_text_lines(path, "実走ログ（run.log）")
    scan = LogScan()
    bad_lines: list[str] = []

    for i, line in enumerate(lines, start=1):
        if not line.strip():
            continue
        at = parse_timestamp(line)
        if at is not None:
            if scan.first_at is None:
                scan.first_at = at
            scan.last_at = at

        if J_PERF_LINE_MESSAGE in line:
            record = _parse_perf_line(i, line, at, bad_lines)
            if record is not None:
                scan.perf.append(record)
            continue

        if J_SHOW_LINE_MESSAGE in line:
            scan.show_count += 1
            fields = parse_fields(split_after_message(line, J_SHOW_LINE_MESSAGE) or "")
            variant = tuple(
                f"{name}={fields.get(name, '(無し)')}" for name in J_DPI_SHOW_FIELDS
            )
            scan.show_dpi_variants[variant] += 1
            continue

        kind = _catchup_kind(line)
        if kind is not None:
            fields = parse_fields(line)
            scan.catchup.append((at, kind, fields.get("target", "(target 欄なし)")))
            continue

        skind = _seriko_kind(line)
        if skind is not None:
            fields = parse_fields(line)
            scan.seriko_events.append(
                (at, skind, fields.get("scope", "?"), fields.get("animation_id", "?"))
            )
            continue

        if any(marker in line for marker in FRAME_INTERVAL_IDLE_EXCLUDE_MARKERS):
            scan.idle_exclusion_hits += 1
            continue

        if not scan.boot_dpi and "primary_dpi=" in line:
            fields = parse_fields(line)
            for name in J_DPI_BOOT_FIELDS:
                if name in fields:
                    scan.boot_dpi[name] = fields[name]

    if bad_lines:
        raise inconclusive(
            "perf サマリ行に必要なフィールドが揃っていません。"
            f"欠落のある行が {len(bad_lines)} 本あります。部分集計は出しません（要件 2.5）。",
            bad_lines[:20]
            + ([f"…ほか {len(bad_lines) - 20} 本"] if len(bad_lines) > 20 else []),
        )
    return scan


def _parse_perf_line(
    line_no: int, line: str, at: datetime | None, bad_lines: list[str]
) -> PerfRecord | None:
    """perf サマリ行 1 本を読む。欠落・値異常は bad_lines へ積んで None を返す。"""
    tail = split_after_message(line, J_PERF_LINE_MESSAGE)
    fields = parse_fields(tail or "")

    missing = [name for name in J_PERF_REQUIRED_FIELDS if name not in fields]
    if missing:
        bad_lines.append(f"{line_no} 行目: 欠落フィールド {', '.join(missing)}")
        return None

    if at is None:
        bad_lines.append(f"{line_no} 行目: 行頭の時刻を読めません（コマ適用間隔が出せません）")
        return None

    numbers: dict[str, int] = {}
    for name in J_PERF_STAGE_FIELDS + J_PERF_ALLOC_FIELDS:
        try:
            numbers[name] = int(fields[name])
        except ValueError:
            bad_lines.append(
                f"{line_no} 行目: {name} が整数ではありません（値 {fields[name]!r}）"
            )
            return None

    raw_hit = fields["cache_hit"]
    if raw_hit not in ("true", "false"):
        bad_lines.append(f"{line_no} 行目: cache_hit が true/false ではありません（値 {raw_hit!r}）")
        return None

    return PerfRecord(
        line_no=line_no,
        at=at,
        target_id=fields["target_id"],
        surface_id=fields["surface_id"],
        cache_hit=(raw_hit == "true"),
        key_hash=fields["key_hash"],
        stages={name: numbers[name] for name in J_PERF_STAGE_FIELDS},
        allocs={name: numbers[name] for name in J_PERF_ALLOC_FIELDS},
    )


# =============================================================================
# CPU 時系列 CSV
# =============================================================================


@dataclass
class CpuSample:
    at: datetime
    percent: float


def read_cpu_csv(path: Path) -> list[CpuSample]:
    lines = read_text_lines(path, "CPU 時系列（cpu.csv）")
    rows = [row for row in csv.reader(lines) if row]
    if not rows:
        raise inconclusive(f"CPU 時系列 CSV が空です: {path}")

    header = tuple(cell.strip() for cell in rows[0])
    if header != J_CPU_CSV_HEADER:
        raise inconclusive(
            "CPU 時系列 CSV のヘッダが契約と違います（採取スクリプトの版ずれの疑い）。",
            [f"期待: {','.join(J_CPU_CSV_HEADER)}", f"実際: {','.join(header)}"],
        )

    samples: list[CpuSample] = []
    broken: list[str] = []
    for line_no, row in enumerate(rows[1:], start=2):
        if len(row) < 2:
            broken.append(f"{line_no} 行目: 列が足りません（{row!r}）")
            continue
        at = parse_timestamp(row[0].strip() + " ")
        if at is None:
            broken.append(f"{line_no} 行目: 時刻を読めません（{row[0]!r}）")
            continue
        try:
            percent = float(row[1].strip())
        except ValueError:
            broken.append(f"{line_no} 行目: CPU 値が数値ではありません（{row[1]!r}）")
            continue
        samples.append(CpuSample(at=at, percent=percent))

    if broken:
        raise inconclusive(
            f"CPU 時系列 CSV に読めない行が {len(broken)} 本あります。部分集計は出しません（要件 2.5）。",
            broken[:20],
        )
    if not samples:
        raise inconclusive(f"CPU 時系列 CSV に観測が 1 点もありません: {path}")
    return samples


# =============================================================================
# run-meta.txt（マシン条件・要件 4.5）
# =============================================================================


def read_run_meta(path: Path) -> list[tuple[str, list[tuple[str, str]]]]:
    """`[見出し]` と `名前 = 値` の並びを、順序を保ったまま読む。"""
    lines = read_text_lines(path, "実行条件（run-meta.txt）")
    sections: list[tuple[str, list[tuple[str, str]]]] = []
    current: list[tuple[str, str]] = []
    title = "(見出しなし)"
    for line in lines:
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if stripped.startswith("[") and stripped.endswith("]"):
            if current:
                sections.append((title, current))
            title = stripped[1:-1]
            current = []
            continue
        if "=" in stripped:
            name, _, value = stripped.partition("=")
            current.append((name.strip(), value.strip()))
    if current:
        sections.append((title, current))
    if not sections:
        raise inconclusive(
            f"実行条件（run-meta.txt）から測定マシンの条件を読み取れません: {path}",
            ["要件 4.5 はマシン条件の併記を求めており、条件なしの集計は出しません。"],
        )
    return sections


def meta_lookup(sections, name: str) -> str | None:
    for _title, items in sections:
        for key, value in items:
            if key == name:
                return value
    return None


# =============================================================================
# 統計
# =============================================================================


def percentile(values: list[float], q: float) -> float:
    """nearest-rank の分位点（PERCENTILE_METHOD）。空列では呼ばない。"""
    ordered = sorted(values)
    rank = max(1, math.ceil(q * len(ordered)))
    return ordered[min(rank, len(ordered)) - 1]


def describe(values: list[float]) -> dict[str, float]:
    return {
        "n": float(len(values)),
        "min": min(values),
        "p50": percentile(values, 0.50),
        "p95": percentile(values, 0.95),
        "max": max(values),
        "mean": sum(values) / len(values),
    }


def fmt_stats(values: list[float], unit: str = "") -> str:
    if not values:
        return "観測なし"
    s = describe(values)
    return (
        f"n={int(s['n'])}  最小={s['min']:.1f}{unit}  p50={s['p50']:.1f}{unit}  "
        f"p95={s['p95']:.1f}{unit}  最大={s['max']:.1f}{unit}  平均={s['mean']:.1f}{unit}"
    )


# =============================================================================
# 活性アニメ本数の推定（idle 窓の根拠）
# =============================================================================


def active_animation_timeline(scan: LogScan) -> list[tuple[datetime, int]]:
    """seriko の発火・停止 info! の収支から、時刻ごとの活性アニメ本数を推定する。

    活性集合サイズの直接ログは存在しないため、これは **間接推定** である
    （design.md「収束判定」の但し書きと同じ性質）。停止が発火より先に現れた場合は
    負にせず 0 で止める（起動前から走っていたぶんは観測できない）。
    """
    active: set[tuple[str, str]] = set()
    timeline: list[tuple[datetime, int]] = []
    for at, kind, scope, anim in sorted(
        (e for e in scan.seriko_events if e[0] is not None), key=lambda e: e[0]
    ):
        key = (scope, anim)
        if kind == "fire":
            active.add(key)
        else:
            active.discard(key)
        timeline.append((at, len(active)))
    return timeline


def active_count_at(timeline: list[tuple[datetime, int]], at: datetime) -> int:
    """`at` の時点で走っていたと推定されるアニメ本数（直前の収支）。"""
    lo, hi = 0, len(timeline)
    while lo < hi:
        mid = (lo + hi) // 2
        if timeline[mid][0] <= at:
            lo = mid + 1
        else:
            hi = mid
    return timeline[lo - 1][1] if lo > 0 else 0


# =============================================================================
# 集計
# =============================================================================


@dataclass
class Aggregation:
    scan: LogScan
    cpu: list[CpuSample]
    steady_from: datetime
    steady_perf: list[PerfRecord]
    intervals_warmup: dict[tuple[str, str], list[float]]
    intervals_idle: dict[tuple[str, str], list[float]]


def frame_intervals(
    records: list[PerfRecord],
    timeline: list[tuple[datetime, int]],
    idle_only: bool,
) -> dict[tuple[str, str], list[float]]:
    """コマ適用間隔（ミリ秒）を系列（target_id x surface_id）ごとに出す。

    系列を分けるのは、別対象への適用が同じ時間帯に重なると、混ぜた列の差分が「間隔」で
    なくなるためである（FRAME_INTERVAL_SERIES_KEY）。`idle_only` のときは、推定活性アニメ
    本数が FRAME_INTERVAL_IDLE_MAX_ACTIVE を超える区間の適用を落とす。**両端が残った隣接**
    だけを間隔として数える（落とした点を跨いだ差分は間隔ではない）。
    """
    by_series: dict[tuple[str, str], list[float]] = defaultdict(list)
    grouped: dict[tuple[str, str], list[PerfRecord]] = defaultdict(list)
    for record in records:
        grouped[record.series_key].append(record)

    for key, group in grouped.items():
        group.sort(key=lambda r: r.at)
        previous: PerfRecord | None = None
        for record in group:
            keep = True
            if idle_only:
                keep = active_count_at(timeline, record.at) <= FRAME_INTERVAL_IDLE_MAX_ACTIVE
            if not keep:
                previous = None
                continue
            if previous is not None:
                by_series[key].append(
                    (record.at - previous.at).total_seconds() * 1000.0
                )
            previous = record
    return by_series


def aggregate(scan: LogScan, cpu: list[CpuSample]) -> Aggregation:
    base = scan.perf[0].at
    steady_from = base + timedelta(seconds=WARMUP_EXCLUDE_SEC)
    steady_perf = [r for r in scan.perf if r.at >= steady_from]
    timeline = active_animation_timeline(scan)
    return Aggregation(
        scan=scan,
        cpu=cpu,
        steady_from=steady_from,
        steady_perf=steady_perf,
        intervals_warmup=frame_intervals(steady_perf, timeline, idle_only=False),
        intervals_idle=frame_intervals(steady_perf, timeline, idle_only=True),
    )


# =============================================================================
# レポート
# =============================================================================


class Report:
    def __init__(self) -> None:
        self.lines: list[str] = []

    def head(self, title: str) -> None:
        self.lines.append("")
        self.lines.append("=" * 78)
        self.lines.append(title)
        self.lines.append("=" * 78)

    def sub(self, title: str) -> None:
        self.lines.append("")
        self.lines.append(f"-- {title} " + "-" * max(0, 74 - len(title)))

    def line(self, text: str = "") -> None:
        self.lines.append(text)

    def render(self) -> str:
        return "\n".join(self.lines)


def render_machine_conditions(report: Report, sections, scan: LogScan) -> None:
    """要件 4.5: 測定マシンの条件をレポート先頭に併記する。"""
    report.head("[1] 測定マシンの条件（要件 4.5）")
    for title, items in sections:
        report.sub(f"run-meta.txt [{title}]")
        for name, value in items:
            report.line(f"  {name} = {value}")

    report.sub("DPI 構成（run.log 由来・run-meta.txt には無い）")
    report.line(
        "  合成コストは DPI に比例し、ベースラインは 2 モニタ混在 DPI で採取される。"
    )
    report.line("  DPI を書かないレポートは同一条件で再測できない。")
    if scan.boot_dpi:
        for name in J_DPI_BOOT_FIELDS:
            if name in scan.boot_dpi:
                report.line(f"  {name} = {scan.boot_dpi[name]}")
    else:
        report.line("  起動時 DPI ログ（primary_dpi を含む行）は run.log にありません。")
    if scan.show_dpi_variants:
        report.line("  表示成立点 info! に現れた DPI/寸法の組み合わせ（出現数の多い順）:")
        for variant, count in scan.show_dpi_variants.most_common():
            report.line(f"    [{count} 回] " + "  ".join(variant))
    else:
        report.line("  表示成立点 info! からは DPI を拾えませんでした。")


def render_calibration(report: Report) -> None:
    """要件 2.6: 較正値をレポートに明記する（本文が使う値はすべてバナー由来）。"""
    report.head("[2] 較正値（要件 2.6 / 4.5・所在はスクリプト冒頭のバナー 1 箇所）")
    rows = [
        ("SCRIPT_VERSION", SCRIPT_VERSION, ""),
        ("FRAME_INTERVAL_EXPECTED_MS", FRAME_INTERVAL_EXPECTED_MS, "emo2 固有・流用禁止"),
        ("FRAME_INTERVAL_TOLERANCE", FRAME_INTERVAL_TOLERANCE, "暫定・task 3.1 で確定"),
        ("FRAME_INTERVAL_SERIES_KEY", " x ".join(FRAME_INTERVAL_SERIES_KEY), ""),
        ("FRAME_INTERVAL_WINDOW", FRAME_INTERVAL_WINDOW, ""),
        (
            "FRAME_INTERVAL_IDLE_MAX_ACTIVE",
            FRAME_INTERVAL_IDLE_MAX_ACTIVE,
            "暫定・task 3.1 で確定",
        ),
        (
            "FRAME_INTERVAL_IDLE_EXCLUDE_MARKERS",
            list(FRAME_INTERVAL_IDLE_EXCLUDE_MARKERS) or "（空＝talk 区間を除外できていない）",
            "task 3.1 で確定",
        ),
        ("WARMUP_EXCLUDE_SEC", WARMUP_EXCLUDE_SEC, "暫定・task 3.1 で確定"),
        ("IDLE_CPU_MAX_RELEASE_PCT", IDLE_CPU_MAX_RELEASE_PCT, "判定は task 2.3"),
        ("CPU_SAMPLE_INTERVAL_EXPECTED_SEC", CPU_SAMPLE_INTERVAL_EXPECTED_SEC, "欠測報告のみに使用"),
        ("CPU_SAMPLE_GAP_FACTOR", CPU_SAMPLE_GAP_FACTOR, ""),
        ("CONVERGENCE_WINDOW_COUNT", CONVERGENCE_WINDOW_COUNT, "判定は task 2.3"),
        (
            "CONVERGENCE_SLOPE_MAX_PCT_PER_MIN",
            CONVERGENCE_SLOPE_MAX_PCT_PER_MIN,
            "判定は task 2.3・暫定",
        ),
        (
            "CONVERGENCE_WINDOW_DIFF_MAX_PCT",
            CONVERGENCE_WINDOW_DIFF_MAX_PCT,
            "判定は task 2.3・暫定",
        ),
        ("PERCENTILE_METHOD", PERCENTILE_METHOD, ""),
        ("J_PERF_LINE_MESSAGE", J_PERF_LINE_MESSAGE, ""),
        ("J_SHOW_LINE_MESSAGE", J_SHOW_LINE_MESSAGE, ""),
        ("J_CATCHUP_TICKER_MESSAGE", J_CATCHUP_TICKER_MESSAGE, "dispatcher / kanade"),
        ("J_CATCHUP_LOOP_MESSAGE", J_CATCHUP_LOOP_MESSAGE, "loop_ticker"),
        ("J_SERIKO_FIRE_MESSAGE", J_SERIKO_FIRE_MESSAGE, ""),
        ("J_SERIKO_STOP_MESSAGES", list(J_SERIKO_STOP_MESSAGES), ""),
        ("J_CPU_CSV_HEADER", ",".join(J_CPU_CSV_HEADER), ""),
        ("J_PERF_IDENT_FIELDS", list(J_PERF_IDENT_FIELDS), ""),
        ("J_PERF_STAGE_FIELDS", list(J_PERF_STAGE_FIELDS), ""),
        ("J_PERF_ALLOC_FIELDS", list(J_PERF_ALLOC_FIELDS), ""),
        (
            "J_PERF_REQUIRED_FIELDS",
            f"{len(J_PERF_REQUIRED_FIELDS)} 個（上の 3 つの和・1 つでも欠ければ exit 2）",
            "",
        ),
        ("J_DPI_BOOT_FIELDS", list(J_DPI_BOOT_FIELDS), "run.log の起動時ログから"),
        ("J_DPI_SHOW_FIELDS", list(J_DPI_SHOW_FIELDS), "表示成立点 info! から"),
        (
            "J_REQUIRED_LOG_KINDS",
            [name for name, _ in J_REQUIRED_LOG_KINDS] + ["CPU 時系列 CSV", "run-meta.txt"],
            "run.stderr.log は必要ログ種ではない",
        ),
    ]
    for name, value, note in rows:
        suffix = f"   ← {note}" if note else ""
        report.line(f"  {name} = {value}{suffix}")
    report.line("")
    report.line("  【他ゴーストへ流用しないこと】「emo2 固有」と付した値は計測 fixture である")
    report.line("  ゴースト emo2 のアニメ定義・実寸に由来する。別のゴーストでは測り直すこと。")


def render_stage_times(report: Report, agg: Aggregation) -> None:
    report.head("[5] 段階別所要（要件 2.2）")
    report.line(f"  分位点の求め方: {PERCENTILE_METHOD}")
    for label, records in (
        ("全区間", agg.scan.perf),
        (f"定常状態（開始から {WARMUP_EXCLUDE_SEC:.0f} 秒を除外）", agg.steady_perf),
    ):
        report.sub(label)
        if not records:
            report.line("  観測なし")
            continue
        for name in J_PERF_STAGE_FIELDS:
            values = [float(r.stages[name]) for r in records]
            s = describe(values)
            report.line(
                f"  {name:<16} n={int(s['n']):<6} p50={s['p50']:>10.0f}us "
                f"p95={s['p95']:>10.0f}us  最大={s['max']:>10.0f}us  平均={s['mean']:>10.0f}us"
            )
        report.line("")
        report.line("  補足: 段の帯は名前より広い（1.3→3.2 申し送り）。")
        report.line("    t_cache_us  はターゲット照会と k 導出を含む。")
        report.line("    t_mask_us   は cache.insert 全体（マスク生成＋スロット置換＋旧エントリ解放）を含む。")
        report.line("    t_upload_us は初回適用の chain/mount 遅延生成を含み、命中経路では照会以降の全部を含む。")


def render_cache_hits(report: Report, agg: Aggregation) -> None:
    report.head("[6] キャッシュ命中率（要件 2.2）")
    for label, records in (
        ("全区間", agg.scan.perf),
        ("定常状態", agg.steady_perf),
    ):
        if not records:
            report.line(f"  {label}: 観測なし")
            continue
        hits = sum(1 for r in records if r.cache_hit)
        report.line(
            f"  {label}: 適用 {len(records)} 回中 命中 {hits} 回 "
            f"（{hits / len(records) * 100:.1f}%・不命中 {len(records) - hits} 回）"
        )
    report.sub("対象別（target_id x surface_id）")
    per: dict[tuple[str, str], list[PerfRecord]] = defaultdict(list)
    for record in agg.scan.perf:
        per[record.series_key].append(record)
    for key, records in sorted(per.items(), key=lambda kv: -len(kv[1])):
        hits = sum(1 for r in records if r.cache_hit)
        report.line(
            f"  {key[0]} / surface_id={key[1]}: 適用 {len(records)} 回・"
            f"命中 {hits} 回（{hits / len(records) * 100:.1f}%）"
        )


def render_intervals(report: Report, agg: Aggregation) -> None:
    report.head("[7] コマ適用間隔（要件 2.2・判定式⑴の入力）")
    report.line(f"  系列の分け方: {' x '.join(FRAME_INTERVAL_SERIES_KEY)}")
    report.line(f"  測定窓: {FRAME_INTERVAL_WINDOW}")
    report.line(
        f"  期待間隔 {FRAME_INTERVAL_EXPECTED_MS:.0f}ms・許容率 {FRAME_INTERVAL_TOLERANCE:.0%} "
        f"→ 判定式⑴の上限は {FRAME_INTERVAL_EXPECTED_MS * (1 + FRAME_INTERVAL_TOLERANCE):.1f}ms"
        "（判定そのものは task 2.3）"
    )
    if not FRAME_INTERVAL_IDLE_EXCLUDE_MARKERS:
        report.line(
            "  【未確定】talk 再生区間を外す目印が未設定です"
            "（FRAME_INTERVAL_IDLE_EXCLUDE_MARKERS が空）。"
        )
        report.line(
            "  現在の ログ水準では talk の開始・終了を名指しできる行が無く、talk 中の適用が"
        )
        report.line("  idle 窓に混ざります。task 3.1 の第 1 段ベースラインで目印を確定すること。")
    else:
        report.line(
            f"  talk 除外の目印に一致した行: {agg.scan.idle_exclusion_hits} 本"
        )

    for label, table in (
        ("窓 A: 起動過渡のみ除外", agg.intervals_warmup),
        (
            f"窓 B: さらに推定活性アニメ {FRAME_INTERVAL_IDLE_MAX_ACTIVE} 本以下へ限定",
            agg.intervals_idle,
        ),
    ):
        report.sub(label)
        if not table:
            report.line("  間隔を出せる連続適用がありません")
            continue
        for key, values in sorted(table.items(), key=lambda kv: -len(kv[1])):
            report.line(f"  {key[0]} / surface_id={key[1]}: {fmt_stats(values, 'ms')}")
        by_target: dict[str, list[float]] = defaultdict(list)
        for (target_id, _surface), values in table.items():
            by_target[target_id].extend(values)
        report.line("")
        report.line("  対象（target_id）別のまとめ:")
        for target_id, values in sorted(by_target.items()):
            report.line(f"    {target_id}: {fmt_stats(values, 'ms')}")


def render_allocs(report: Report, agg: Aggregation) -> None:
    report.head("[8] 表示用バッファの新規確保 計数（要件 2.2・判定式⑶の入力）")
    for label, records in (
        ("全区間", agg.scan.perf),
        ("定常状態", agg.steady_perf),
    ):
        report.sub(label)
        if not records:
            report.line("  観測なし")
            continue
        total = 0
        for name in J_PERF_ALLOC_FIELDS:
            value = sum(r.allocs[name] for r in records)
            total += value
            nonzero = sum(1 for r in records if r.allocs[name] > 0)
            report.line(f"  {name:<20} 合計 {value:>8}  （発生した適用 {nonzero} 回）")
        report.line(f"  {'合計':<20} 合計 {total:>8}")


def render_catchup(report: Report, agg: Aggregation) -> None:
    report.head("[9] 進行境界スキップ（catch-up）件数（要件 2.2・判定式⑵の入力）")
    report.line("  数え方: 長い文言（loop_ticker）を先に判定してから短い文言を数える。")
    report.line("  短いほうは長いほうの部分文字列であり、素朴に数えると二重計上になる。")
    if not agg.scan.catchup:
        report.line("  0 件（全区間）")
        return
    for label, events in (
        ("全区間", agg.scan.catchup),
        (
            "定常状態",
            [e for e in agg.scan.catchup if e[0] is not None and e[0] >= agg.steady_from],
        ),
    ):
        counts: Counter = Counter()
        for _at, kind, target in events:
            counts[(kind, target)] += 1
        report.sub(label)
        report.line(f"  合計 {len(events)} 件")
        for (kind, target), count in sorted(counts.items()):
            report.line(f"    文言={kind}  target={target}: {count} 件")


def render_cpu(report: Report, agg: Aggregation) -> None:
    report.head("[10] CPU 時系列（要件 2.2 / 2.3・判定式⑷と収束判定の入力）")
    samples = agg.cpu
    first, last = samples[0].at, samples[-1].at
    span_sec = (last - first).total_seconds()
    report.line(f"  採取点 {len(samples)} 点・{first.isoformat()} 〜 {last.isoformat()}（{span_sec:.0f} 秒）")
    report.line("  単位: 1 コアを 100% とする換算値（複数コア使用時は 100 超もあり得る）。")
    report.line("  1 行は約 1 秒幅の瞬時値であり、刻み幅の平均ではない。")

    report.sub("刻み（均等と仮定しない・2.1→2.2 申し送り 1）")
    gaps = [
        (samples[i + 1].at - samples[i].at).total_seconds() for i in range(len(samples) - 1)
    ]
    if not gaps:
        report.line("  採取点が 1 点しかないため刻みを出せません")
    else:
        report.line(f"  実測の刻み（秒）: {fmt_stats(gaps, 's')}")
        limit = CPU_SAMPLE_INTERVAL_EXPECTED_SEC * CPU_SAMPLE_GAP_FACTOR
        skipped = [g for g in gaps if g > limit]
        report.line(
            f"  期待刻み {CPU_SAMPLE_INTERVAL_EXPECTED_SEC:.0f}s x {CPU_SAMPLE_GAP_FACTOR} "
            f"= {limit:.1f}s を超えた刻み: {len(skipped)} 箇所"
            + (f"（最大 {max(skipped):.1f}s）" if skipped else "")
        )
        report.line("  ランナーは絶対時刻へ積み上げるためドリフトしないが、")
        report.line("  マシンが詰まったときの欠測は印を残さない。上の件数がその痕跡である。")

    for label, subset in (
        ("全区間", samples),
        ("定常状態", [s for s in samples if s.at >= agg.steady_from]),
    ):
        report.sub(label)
        if not subset:
            report.line("  観測なし")
            continue
        report.line(f"  CPU: {fmt_stats([s.percent for s in subset], '%')}")

    report.sub(f"窓別平均（{CONVERGENCE_WINDOW_COUNT} 等分・収束判定 task 2.3 の入力）")
    steady = [s for s in samples if s.at >= agg.steady_from]
    if len(steady) < CONVERGENCE_WINDOW_COUNT:
        report.line(
            f"  定常状態の採取点が {len(steady)} 点しかなく "
            f"{CONVERGENCE_WINDOW_COUNT} 等分できません（短時間水準では通常の状態）。"
        )
    else:
        size = len(steady) / CONVERGENCE_WINDOW_COUNT
        for w in range(CONVERGENCE_WINDOW_COUNT):
            chunk = steady[int(w * size): int((w + 1) * size)]
            if not chunk:
                continue
            mean = sum(s.percent for s in chunk) / len(chunk)
            report.line(
                f"  窓{w + 1}: n={len(chunk)}  平均={mean:.2f}%  "
                f"（{chunk[0].at.strftime('%H:%M:%S')} 〜 {chunk[-1].at.strftime('%H:%M:%S')}）"
            )


def render_compose_keys(report: Report, agg: Aggregation) -> None:
    report.head("[11] 合成キーの異なり数（要件 7.2・キャッシュ容量の裁定材料）")
    report.line("  キャッシュ容量は承認済み要件で 1 である。以下は「容量 1 で足りるのか」を")
    report.line("  実測で示すための材料であり、容量変更は開発者の明示的な裁定なしに行わない。")
    for label, records in (("全区間", agg.scan.perf), ("定常状態", agg.steady_perf)):
        report.sub(label)
        if not records:
            report.line("  観測なし")
            continue
        pairs = Counter((r.key_hash, r.surface_id) for r in records)
        report.line(f"  異なり数（key_hash x surface_id）: {len(pairs)}")
        report.line(f"  異なり数（key_hash のみ）        : {len({r.key_hash for r in records})}")
        report.line(f"  異なり数（surface_id のみ）      : {len({r.surface_id for r in records})}")
        per_target: dict[str, set] = defaultdict(set)
        for record in records:
            per_target[record.target_id].add((record.key_hash, record.surface_id))
        for target_id, keys in sorted(per_target.items()):
            report.line(f"    {target_id}: 異なりキー {len(keys)} 個")
        report.line("  出現の多いキー（上位 10）:")
        for (key_hash, surface_id), count in pairs.most_common(10):
            report.line(f"    key_hash={key_hash} surface_id={surface_id}: {count} 回")


def render_seriko(report: Report, agg: Aggregation) -> None:
    report.head("[12] seriko のループ発火・停止（要件 5.1 の傍証・idle 窓の根拠）")
    report.line("  活性アニメ本数を直接出すログは無い。以下は発火・停止 info! の収支からの")
    report.line("  **間接推定** であり、そのつもりで読むこと。")
    fires = sum(1 for e in agg.scan.seriko_events if e[1] == "fire")
    stops = sum(1 for e in agg.scan.seriko_events if e[1] == "stop")
    report.line(f"  発火 {fires} 件・停止 {stops} 件")
    timeline = active_animation_timeline(agg.scan)
    if timeline:
        counts = [c for _at, c in timeline]
        report.line(f"  推定活性本数の推移: {fmt_stats([float(c) for c in counts], ' 本')}")
        report.line(f"  末尾時点の推定活性本数: {counts[-1]} 本")
    else:
        report.line("  発火・停止のログが 1 件もありません（RUST_LOG に seriko が乗っていない疑い）。")


# =============================================================================
# 必要ログ種の検査（要件 2.5）
# =============================================================================


def check_required(scan: LogScan, cpu: list[CpuSample], run_log: Path) -> None:
    missing: list[str] = []
    if not scan.perf:
        missing.append(
            f"perf サマリ行（固定文言 {J_PERF_LINE_MESSAGE!r}）が 1 本もありません。"
            " RUST_LOG に areka_emo_present=debug が乗っていない疑いがあります。"
        )
    if scan.show_count == 0:
        missing.append(
            f"表示成立点 info!（固定文言 {J_SHOW_LINE_MESSAGE!r}）が 1 本もありません。"
        )
    if not cpu:
        missing.append("CPU 時系列に観測が 1 点もありません。")
    if missing:
        raise inconclusive(
            f"必要ログ種が欠けています（{run_log}）。部分集計は出しません（要件 2.5）。",
            missing,
        )


# =============================================================================
# 集計モード
# =============================================================================


def run_baseline(args: argparse.Namespace) -> int:
    run_log = Path(args.run_log)
    cpu_csv = Path(args.cpu_csv)
    meta_given = args.meta is not None
    meta_path = Path(args.meta) if meta_given else cpu_csv.parent / "run-meta.txt"

    for path, what in ((run_log, "実走ログ"), (cpu_csv, "CPU 時系列")):
        if not path.is_file():
            raise bad_input(f"{what}のファイルがありません: {path}")
    if not meta_path.is_file():
        # 明示指定されたパスが無いのは引数の誤り（3）。自動探索で見つからないのは
        # 走行に条件ファイルが無いということ＝欠落（2）。両者は直し方が違うので分ける。
        if meta_given:
            raise bad_input(f"--meta が指すファイルがありません: {meta_path}")
        raise inconclusive(
            f"実行条件（run-meta.txt）が見つかりません: {meta_path}",
            [
                "要件 4.5 は測定マシンの条件をレポートへ併記することを求めており、",
                "条件を書けない集計は出しません。",
                "既定では cpu.csv と同じディレクトリを探します。別の場所にある場合は",
                "--meta <パス> で指定してください。",
            ],
        )

    sections = read_run_meta(meta_path)
    scan = scan_log(run_log)
    cpu = read_cpu_csv(cpu_csv)
    check_required(scan, cpu, run_log)

    meta_build = meta_lookup(sections, "build")
    build = args.build or meta_build
    if args.build and meta_build and args.build != meta_build:
        raise bad_input(
            "--build と run-meta.txt の build が食い違っています（別の走行のログを"
            "組み合わせている疑い）。",
            [f"--build = {args.build}", f"run-meta.txt build = {meta_build}"],
        )

    agg = aggregate(scan, cpu)

    report = Report()
    report.line("=" * 78)
    report.line("areka 性能計測 集計レポート（judge-perf.py）")
    report.line("=" * 78)
    report.line("  モード       : baseline（集計のみ・合否判定は task 2.3）")
    report.line(f"  スクリプト版 : {SCRIPT_VERSION}")
    report.line(f"  ビルド種別   : {build or '(不明)'}")
    render_machine_conditions(report, sections, scan)
    render_calibration(report)

    report.head("[3] 入力")
    report.line(f"  実走ログ     : {run_log}")
    report.line(f"  CPU 時系列   : {cpu_csv}")
    report.line(f"  実行条件     : {meta_path}")
    stderr_log = run_log.parent / "run.stderr.log"
    report.line(
        f"  標準エラー   : {stderr_log}"
        + (
            f"（{stderr_log.stat().st_size} バイト）"
            if stderr_log.is_file()
            else "（なし）"
        )
    )
    report.line("    ※ 標準出力と標準エラーは同一ファイルへ流せないためランナーが分けている。")
    report.line("       必要ログ種ではないので、無くても集計は成立する。")

    report.head("[4] 必要ログ種の検査（要件 2.5）")
    report.line(f"  perf サマリ行      : {len(scan.perf)} 本  → OK")
    report.line(f"  表示成立点 info!   : {scan.show_count} 本  → OK")
    report.line(f"  CPU 時系列         : {len(cpu)} 点  → OK")
    report.line(f"  実行条件           : {sum(len(items) for _t, items in sections)} 項目 → OK")
    report.line(
        f"  perf 行の全 {len(J_PERF_REQUIRED_FIELDS)} フィールドが全行に揃っていることを確認済み。"
    )
    if scan.first_at and scan.last_at:
        report.line(
            f"  ログの時間幅       : {scan.first_at.isoformat()} 〜 {scan.last_at.isoformat()}"
        )
    report.line(
        f"  定常状態の開始     : {agg.steady_from.isoformat()}"
        f"（最初の perf 行から {WARMUP_EXCLUDE_SEC:.0f} 秒後）"
    )
    if not agg.steady_perf:
        report.line(
            "  【注意】定常状態に perf 行が 1 本もありません。"
            "WARMUP_EXCLUDE_SEC が走行長に対して大きすぎます。"
        )

    render_stage_times(report, agg)
    render_cache_hits(report, agg)
    render_intervals(report, agg)
    render_allocs(report, agg)
    render_catchup(report, agg)
    render_cpu(report, agg)
    render_compose_keys(report, agg)
    render_seriko(report, agg)

    report.head("[13] 読むときの注意（較正が未確定の箇所）")
    caveats = [
        "コマ適用間隔の idle 窓の境界は未確定である（task 3.1 の第 1 段ベースラインで確定し"
        " README へ登記する）。窓 A と窓 B の両方を並べてあるのはそのためである。",
        "talk 再生区間を外す目印が未設定のため、talk 中の適用が窓 B に混ざっている。",
        "活性アニメ本数は発火・停止 info! の収支からの間接推定であり、直接の観測ではない。",
        f"WARMUP_EXCLUDE_SEC={WARMUP_EXCLUDE_SEC:.0f} 秒・"
        f"FRAME_INTERVAL_TOLERANCE={FRAME_INTERVAL_TOLERANCE:.0%} は暫定値である。",
    ]
    if not agg.steady_perf:
        caveats.insert(
            0,
            "【重大】定常状態に perf 行が 1 本もない。WARMUP_EXCLUDE_SEC が走行長に対して"
            "大きすぎるので、この走行から定常状態の数値は読めない。",
        )
    if len(agg.cpu) < CONVERGENCE_WINDOW_COUNT * 2:
        caveats.append(
            f"CPU の採取点が {len(agg.cpu)} 点しかなく、収束判定（要件 5.1）に足りない"
            "可能性がある。長時間水準で採り直すこと。"
        )
    for i, text in enumerate(caveats, start=1):
        report.line(f"  ({i}) {text}")

    report.line("")
    report.line("=" * 78)
    report.line("集計は以上。合否判定（要件 4.2 の判定式⑴〜⑷）と収束判定（要件 5.1）は")
    report.line("--mode verdict（task 2.3）が行う。")
    report.line("=" * 78)

    print(report.render())
    return EXIT_OK


#: モードの登録簿。task 2.3 は "verdict" をここへ足すだけで済む。
MODES = {
    "baseline": run_baseline,
}

#: まだ実装していないモード（求められたら黙らずに理由を述べて終了する）。
PLANNED_MODES = {
    "verdict": "合否判定モードは task 2.3 で実装する（本スクリプトは集計モードのみ）。",
}


# =============================================================================
# 入口
# =============================================================================


class Parser(argparse.ArgumentParser):
    """引数不正の終了コードを 3 に揃える（argparse の既定は 2＝判定不能と衝突する）。"""

    def error(self, message: str):  # type: ignore[override]
        sys.stderr.write(f"[judge-perf] 引数が不正です: {message}\n")
        sys.stderr.write(self.format_usage())
        raise SystemExit(EXIT_BAD_INPUT)


def build_parser() -> Parser:
    parser = Parser(
        prog="judge-perf.py",
        description="areka 性能計測の集計・判定（要件 2.2/2.5/2.6/4.5/7.2）",
    )
    parser.add_argument("run_log", help="実走ログ run.log のパス")
    parser.add_argument("cpu_csv", help="CPU 時系列 cpu.csv のパス")
    parser.add_argument(
        "--mode",
        required=True,
        choices=sorted(MODES) + sorted(PLANNED_MODES),
        help="baseline=集計（本スクリプトが実装済み）／verdict=合否判定（task 2.3）",
    )
    parser.add_argument(
        "--build",
        choices=("dev", "release"),
        help="ビルド種別。省略時は run-meta.txt の build を使う",
    )
    parser.add_argument(
        "--meta",
        help="実行条件 run-meta.txt のパス（省略時は cpu.csv と同じディレクトリを探す）",
    )
    return parser


def main(argv: list[str]) -> int:
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[union-attr]
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[union-attr]
    except (AttributeError, OSError):
        pass

    args = build_parser().parse_args(argv)
    if args.mode in PLANNED_MODES:
        sys.stderr.write(f"[judge-perf] {PLANNED_MODES[args.mode]}\n")
        return EXIT_BAD_INPUT

    try:
        return MODES[args.mode](args)
    except JudgeError as exc:
        label = {
            EXIT_INCONCLUSIVE: "判定不能",
            EXIT_BAD_INPUT: "入力不正",
        }.get(exc.code, "失敗")
        sys.stderr.write(f"[judge-perf] {label}: {exc.message}\n")
        for detail in exc.details:
            sys.stderr.write(f"  - {detail}\n")
        sys.stderr.write(f"[judge-perf] 部分集計は出していません。終了コード {exc.code}\n")
        return exc.code


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
