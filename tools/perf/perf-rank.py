#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""1 走行の成果物から「どこが重いか」の 4 段の順位表（rank.txt）を出す道具。

自走改善ループの ⒝ 解析はこの 1 ファイルだけを読んで次の仮説を選ぶ（要件 2.1）。
だから順位表は、人の勘の入る余地が無いほど機械的で、同じ入力なら 1 バイト違わず
同じ文面でなければならない（要件 2.10）。

4 段は「プロセス全体 → スレッド別 → 関数・コード領域別 → フレーム駆動の相別」で、
上へ行くほど広く、下へ行くほど細かい。段が下るほど採取の条件が厳しくなるので
（段③は昇格した PowerShell が要る）、**採れなかった段は空欄ではなく理由を書く**
——黙って続けないための決まりである（要件 2.11）。

    [1] プロセス  cpu.csv（1 コア換算 %）から定常平均・p50・p95・最大・発話中の頂
    [2] スレッド  perf(thread)／perf(process) の 2 スナップショットの差から役割別の CPU 秒
    [3] 関数      dump.txt（CPU サンプリング）から自己時間・包含時間・記号解決率
    [4] 相        [tick] kind=window 行から tick/秒・省略率・13 本のスケジュール別

**壁時計と CPU 時間は混ぜない**（要件 2.6）。段①②は CPU 時間、段④は壁時計、
段③はサンプル数である。どれがどれかは各段の見出しの `unit=` が名乗る。

    python tools/perf/perf-rank.py <run-dir>            # 4 段すべて・上位 10
    python tools/perf/perf-rank.py <run-dir> --stage function --top 20
    python tools/perf/perf-rank.py --selftest           # 自己較正

入力は `<run-dir>` の中の `run.log`（必須）・`cpu.csv`（必須・ヘッダは judge-perf.py と
同じ `timestamp,cpu_percent_1core`）・`run-meta.txt`（任意）・`dump.txt`（任意・あれば段③）。

終了コード（judge-perf.py・perf-ledger.py と同じ体系）: 0＝順位表を書けた（段が
UNAVAILABLE でも、理由を書けているなら 0）／1＝`--selftest` の食い違い＝この道具自身が
壊れている／3＝引数不正・走行ディレクトリが無い・読めない／4＝計測失敗（run.log か
cpu.csv が無いか空・cpu.csv のヘッダが契約違い・観測行が壊れている・dump が在るのに空・dump の既知の
列名行が無い・段③が利用可なのに `areka.exe!` の解決フレームが 0）。**2（判定不能）は
返さない**——順位表は合否を出さないので、判定できるかどうかという状態が無い。

判定スクリプトとの関係: `judge-perf.py` は import しない（同じ周に別の担当が触っている
可能性があり、輸入すると 2 つの道具が同時に倒れる）。行の読み方は judge-perf と共有の
`perf_lines.py` を読む（judge-perf 本体は読まない）。較正値（`WARMUP_EXCLUDE_SEC`・SSP 参考値）も
同じ理由で写しであり、値を動かすときは両方を同時に動かすこと。

段③（dump の解析）と、順位表が共有する土台（失敗の運び方・固定幅の描画・数の書式）は
`perf_rank_dump.py` に置いてある——1 ファイル 1,000 行以下の目安（要件 6.8）のため。
"""

from __future__ import annotations

import argparse
import re
import shutil
import sys
from dataclasses import dataclass, field, replace
from datetime import datetime, timedelta
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from perf_lines import (  # noqa: E402
    CpuSample,
    parse_fields,
    parse_timestamp,
    percentile,
    read_cpu_csv as _read_cpu_csv,
    read_text_lines as _read_text_lines,
)

import perf_selftest_common as selftest_common  # noqa: E402

from perf_rank_dump import (  # noqa: E402
    EXIT_BAD_INPUT,
    EXIT_FAIL,
    EXIT_MEASURE_FAILED,
    EXIT_OK,
    PLACEHOLDER,
    Column,
    DumpScan,
    RankError,
    bad_input,
    fmt,
    measure_failed,
    note,
    parse_dump,
    render_table,
    scalar,
    share_pct,
    stage_function,
    tid_sort_key,
    truncated,
)

# --- 定数（較正値・語彙）-----------------------------------------------------

#: 順位表の版。書式か集計規則を変えたら上げる（fixture の期待出力も一緒に変わる）。
SCRIPT_VERSION = "0.1.0"

STDERR_PREFIX = "perf-rank:"

#: 起動過渡として定常状態から外す秒数。`judge-perf.py` の `WARMUP_EXCLUDE_SEC` の写し
#: （task 3.2 の実測で 60 秒を据え置き）。片方だけ動かすと 2 つの道具が別の定常を語る。
WARMUP_EXCLUDE_SEC = 60.0

#: 「発話中」とみなす表示成立点の前後の秒数（設計 C9 の「前後 10 秒」）。
TALK_PEAK_WINDOW_SEC = 10.0

#: 段②が差を取る相手を選ぶときの目安（設計 C9 の「60 秒前」）。
PERF_THREAD_WINDOW_SEC = 60.0

#: SSP 参考値（2026-08-15 実測・要件 5.2／5.4）。**合否には使わない**。
SSP_REFERENCE_IDLE_PCT = 3.05
SSP_REFERENCE_TALK_PEAK_PCT = 4.64
SSP_REFERENCE_DATE = "2026-08-15"

DEFAULT_TOP = 10

#: cpu.csv のヘッダ（`judge-perf.py` の `J_CPU_CSV_HEADER` の写し）。
CPU_CSV_HEADER = ("timestamp", "cpu_percent_1core")

#: 行を見つけるための固定文言。実行体側の書式の権威は次のとおり:
#:   perf(thread)／perf(process) … crates/areka/src/perf_thread_report.rs
#:   [tick] kind=window           … crates/wintf/src/ecs/world/tick_diag.rs
#:   apply(ShowSurface)／perf(apply_show) … judge-perf.py の同名定数
THREAD_LINE_MESSAGE = "perf(thread): スレッド別 CPU"
PROCESS_LINE_MESSAGE = "perf(process): プロセス CPU"
TICK_WINDOW_MARKER = "[tick] kind=window"
SHOW_LINE_MESSAGE = "apply(ShowSurface): 表示・マスクを更新"
APPLY_SHOW_MESSAGE = "perf(apply_show): 段階別計時"

#: 名簿外の残りを表す役割名（`perf_thread_report.rs` の `ROLE_UNREGISTERED_REST`）。
#: この 1 本だけは「測ったスレッド」ではなく「プロセス CPU − 名簿の合計」である。
ROLE_UNREGISTERED_REST = "unregistered_rest"

#: 13 本のスケジュール（`tick_diag.rs` の `SCHEDULE_NAMES` と同じ綴り・同じ順）。
SCHEDULE_NAMES = (
    "input", "update", "prelayout", "layout", "postlayout", "uisetup",
    "graphicssetup", "draw", "prerendersurface", "rendersurface", "composition",
    "commitcomposition", "framefinalize",
)

STAGE_ALL = ("process", "thread", "function", "phase")

#: 自己較正の置き場と、そこで必ず再現していなければならない赤。
SELFTEST_FIXTURES_DIRNAME = "fixtures-loop"
SELFTEST_RANK_SUBDIR = "rank"
SELFTEST_CASE_FILENAME = "case.txt"
SELFTEST_EXPECTED_FILENAME = "expected_rank.txt"
SELFTEST_REQUIRED_RED_EXITS = (EXIT_MEASURE_FAILED,)

OUTPUT_FILENAME = "rank.txt"


# --- 行の解析（perf_lines.py に共有。bad_input は読めないファイル用）------

def read_text_lines(path: Path, what: str) -> list[str]:
    """テキストを行に割って返す。読めなければ exit 3。"""
    return _read_text_lines(path, what, bad_input)


def iso_z(at: datetime) -> str:
    """注記に載せる時刻（秒まで・UTC）。注記は目印であって観測ではないので小数は出さない。"""
    return at.strftime("%Y-%m-%dT%H:%M:%SZ")


# --- run.log の走査 ----------------------------------------------------------


@dataclass
class ThreadRow:
    """`perf(thread)` 行 1 本（値は**累積**・区間の量ではない）。"""

    snap: int
    t_s: int
    tid: int
    name: str
    role: str
    cpu_us: int
    kernel_us: int
    user_us: int


@dataclass
class ProcessRow:
    """`perf(process)` 行 1 本（使う欄だけ読む・`cpu_us` は累積の CPU 時間）。"""

    snap: int
    cpu_us: int


@dataclass
class TickRow:
    """`[tick] kind=window` 行 1 本（1 秒窓の合計・13 本は壁時計 µs）。"""

    t_ms: int
    ticks: int
    skipped: int
    heartbeat: int
    wall_us: int
    max_us: int
    ui_cpu_us: int
    per_schedule_us: dict[str, int]


@dataclass
class LogScan:
    first_at: datetime | None = None
    first_apply_show_at: datetime | None = None
    show_times: list[datetime] = field(default_factory=list)
    threads: list[ThreadRow] = field(default_factory=list)
    processes: list[ProcessRow] = field(default_factory=list)
    ticks: list[TickRow] = field(default_factory=list)


def _need_int(fields: dict[str, str], key: str, line_no: int, broken: list[str]) -> int:
    """観測行から整数を 1 つ取る。取れなければ理由を積んで 0 を返す（後で exit 4）。"""
    raw = fields.get(key)
    if raw is None:
        broken.append(f"{line_no} 行目: フィールド {key} がありません")
        return 0
    try:
        return int(raw)
    except ValueError:
        broken.append(f"{line_no} 行目: フィールド {key} が整数ではありません（{raw!r}）")
        return 0


def scan_run_log(path: Path) -> LogScan:
    """run.log を 1 度だけ舐めて、4 段が要る行をすべて拾う。

    観測行の書式が壊れていたら（フィールドが欠けている・数でない）**部分集計は出さない**
    ——順位表は上位から順に読まれるので、1 行の欠けが順位を入れ替えてしまう。
    """
    scan = LogScan()
    broken: list[str] = []

    for line_no, line in enumerate(read_text_lines(path, "走行ログ（run.log）"), start=1):
        if not line.strip():
            continue
        at = parse_timestamp(line)
        if at is not None and scan.first_at is None:
            scan.first_at = at

        if APPLY_SHOW_MESSAGE in line:
            if at is not None and scan.first_apply_show_at is None:
                scan.first_apply_show_at = at
            continue

        if SHOW_LINE_MESSAGE in line:
            if at is not None:
                scan.show_times.append(at)
            continue

        index = line.find(THREAD_LINE_MESSAGE)
        if index >= 0:
            f = parse_fields(line[index + len(THREAD_LINE_MESSAGE):])
            need = lambda key: _need_int(f, key, line_no, broken)  # noqa: E731
            scan.threads.append(ThreadRow(
                snap=need("snap"), t_s=need("t_s"), tid=need("tid"),
                name=f.get("name", PLACEHOLDER), role=f.get("role", PLACEHOLDER),
                cpu_us=need("cpu_us"), kernel_us=need("kernel_us"), user_us=need("user_us"),
            ))
            continue

        index = line.find(PROCESS_LINE_MESSAGE)
        if index >= 0:
            f = parse_fields(line[index + len(PROCESS_LINE_MESSAGE):])
            need = lambda key: _need_int(f, key, line_no, broken)  # noqa: E731
            scan.processes.append(ProcessRow(snap=need("snap"), cpu_us=need("cpu_us")))
            continue

        index = line.find(TICK_WINDOW_MARKER)
        if index >= 0:
            f = parse_fields(line[index + len(TICK_WINDOW_MARKER):])
            need = lambda key: _need_int(f, key, line_no, broken)  # noqa: E731
            scan.ticks.append(TickRow(
                t_ms=need("t_ms"), ticks=need("ticks"), skipped=need("skipped"),
                heartbeat=need("heartbeat"), wall_us=need("wall_us"),
                max_us=need("max_us"), ui_cpu_us=need("ui_cpu_us"),
                per_schedule_us={name: need(f"{name}_us") for name in SCHEDULE_NAMES},
            ))
            continue

    if broken:
        raise measure_failed(
            f"走行ログの観測行が {len(broken)} 箇所読めません。部分的な順位表は出しません。",
            broken[:20],
        )
    return scan


# --- cpu.csv・run-meta.txt ---------------------------------------------------


def read_cpu_csv(path: Path) -> list[CpuSample]:
    """CPU 時系列を読む。ファイルを読めなければ exit 3・中身の不備は exit 4。"""
    return _read_cpu_csv(path, CPU_CSV_HEADER, measure_failed, read_fail=bad_input)


def read_run_meta(path: Path) -> dict[str, str]:
    """`[見出し]` と `名前 = 値` を平らな辞書にする（順位表は見出しを使わない）。"""
    meta: dict[str, str] = {}
    for line in read_text_lines(path, "実行条件（run-meta.txt）"):
        stripped = line.strip()
        if not stripped or stripped.startswith("#") or stripped.startswith("["):
            continue
        if "=" in stripped:
            name, _, value = stripped.partition("=")
            meta[name.strip()] = value.strip()
    return meta


# --- 段① プロセス -----------------------------------------------------------


def stage_process(scan: LogScan, cpu: list[CpuSample], meta: dict[str, str] | None) -> list[str]:
    """cpu.csv から定常平均・分位点・最大と、発話中の頂を出す（unit=cpu）。"""
    # 定常の起点は judge-perf.py と同じ「最初の perf 行」。順位付けの走行では
    # perf(apply_show) を点けないこともあるので、無ければ最初の時刻つき行へ倒す。
    if scan.first_apply_show_at is not None:
        base, base_src = scan.first_apply_show_at, "perf_apply_show"
    elif scan.first_at is not None:
        base, base_src = scan.first_at, "first_log_line"
    else:
        base, base_src = cpu[0].at, "first_cpu_sample"
    steady_from = base + timedelta(seconds=WARMUP_EXCLUDE_SEC)

    steady = [s for s in cpu if s.at >= steady_from]
    basis = "steady" if steady else "all"
    values = [s.percent for s in (steady if steady else cpu)]

    peak: float | None = None
    window = timedelta(seconds=TALK_PEAK_WINDOW_SEC)
    for show_at in scan.show_times:
        for sample in cpu:
            if show_at - window <= sample.at <= show_at + window:
                peak = sample.percent if peak is None else max(peak, sample.percent)

    lines = [
        f"[1] プロセス  unit=cpu  1 コア換算 %  出所=cpu.csv  基準={basis}",
        scalar("steady_mean_pct", fmt(sum(values) / len(values), 2)),
        scalar("p50_pct", fmt(percentile(values, 0.50), 2)),
        scalar("p95_pct", fmt(percentile(values, 0.95), 2)),
        scalar("max_pct", fmt(max(values), 2)),
        scalar("talk_peak_pct", fmt(peak, 2)),
        scalar("samples_total", str(len(cpu))),
        scalar("samples_steady", str(len(steady))),
        note(
            f"定常＝起点から {WARMUP_EXCLUDE_SEC:.1f} 秒（WARMUP_EXCLUDE_SEC）を除く。"
            f"起点={base_src} {iso_z(base)} ／ 定常の始まり {iso_z(steady_from)}"
        ),
    ]
    if not steady:
        lines.append(note("定常状態に採取点が 1 つも無いので、全採取点で代用した（basis=all）"))
    lines.append(note(
        f"発話中の頂＝apply(ShowSurface) の前後 {TALK_PEAK_WINDOW_SEC:.1f} 秒に"
        "重なる採取点の最大（合否外）"
    ))
    lines.append(note(
        f"SSP 参考値 idle={SSP_REFERENCE_IDLE_PCT:.2f} "
        f"talk_peak={SSP_REFERENCE_TALK_PEAK_PCT:.2f}"
        f"（{SSP_REFERENCE_DATE} 実測・参考値・合否外）"
    ))
    if meta is None:
        lines.append(note("実行条件 run-meta.txt がありません"))
    else:
        lines.append(note(
            f"実行条件 build={meta.get('build', PLACEHOLDER)} "
            f"logical_processors={meta.get('logical_processors', PLACEHOLDER)}"
        ))
    return lines


# --- 段② スレッド -----------------------------------------------------------


def thread_delta(prev: list[ThreadRow], cur: list[ThreadRow]) -> list[ThreadRow]:
    """2 つのスナップショット（累積）の差を取る。

    `crates/areka/src/perf_thread_report.rs` の `delta` と同じ意味論の写し——
    突き合わせは TID で行い、前回に無い TID は値をそのまま持ち（その区間で生まれた
    スレッド）、巻き戻って見えたら 0 で止める。あちらの決定論テストが規則の権威で、
    ここが食い違えば順位表が静かに嘘をつく。
    """
    before = {row.tid: row for row in prev}
    out: list[ThreadRow] = []
    for row in cur:
        old = before.get(row.tid)
        kernel = max(row.kernel_us - (old.kernel_us if old else 0), 0)
        user = max(row.user_us - (old.user_us if old else 0), 0)
        out.append(ThreadRow(
            snap=row.snap, t_s=row.t_s, tid=row.tid, name=row.name, role=row.role,
            cpu_us=kernel + user, kernel_us=kernel, user_us=user,
        ))
    return out


def stage_thread(scan: LogScan, dump: DumpScan | None, top: int) -> list[str]:
    """2 スナップショットの差から役割別・スレッド別の CPU 秒と占有率（unit=cpu）。"""
    if not scan.threads:
        return [
            "[2] スレッド UNAVAILABLE reason=no_perf_thread_lines",
            note("perf(thread) 行が 1 本も無い。順位付けの走行では areka::perf=debug を点けること。"),
        ]

    by_snap: dict[int, list[ThreadRow]] = {}
    for row in scan.threads:
        by_snap.setdefault(row.snap, []).append(row)
    snaps = sorted(by_snap)
    last_snap = snaps[-1]
    last_rows = by_snap[last_snap]
    last_t_s = max(row.t_s for row in last_rows)

    # 差を取る相手は「最後のスナップショットの 60 秒前」（設計 C9）。それが無ければ
    # 最初のスナップショット。1 枚しか無ければ差が取れないので累積をそのまま使う。
    prev_snap: int | None = None
    for snap in snaps[:-1]:
        if last_t_s - max(row.t_s for row in by_snap[snap]) >= PERF_THREAD_WINDOW_SEC:
            prev_snap = snap
    if prev_snap is None and len(snaps) >= 2:
        prev_snap = snaps[0]

    if prev_snap is None:
        basis, rows, window_s = "cumulative", last_rows, float(last_t_s)
    else:
        basis = "delta"
        rows = thread_delta(by_snap[prev_snap], last_rows)
        window_s = float(last_t_s - max(r.t_s for r in by_snap[prev_snap]))

    processes = {row.snap: row for row in scan.processes}
    process_cpu_us: int | None = None
    if last_snap in processes:
        if prev_snap is None:
            process_cpu_us = processes[last_snap].cpu_us
        elif prev_snap in processes:
            process_cpu_us = max(processes[last_snap].cpu_us - processes[prev_snap].cpu_us, 0)
    whole = float(process_cpu_us) if process_cpu_us is not None else 0.0

    # 名簿外の残りは行どうしの差でなくプロセスの差から引く（D7・#4）。使い回された TID の
    # 新しいスレッドは delta で 0 に止まり、行の差のままだと残りが幻の CPU で膨らむ。
    if basis == "delta" and process_cpu_us is not None:
        named_us = sum(r.cpu_us for r in rows if r.role != ROLE_UNREGISTERED_REST)
        rows = [replace(r, cpu_us=max(process_cpu_us - named_us, 0))
                if r.role == ROLE_UNREGISTERED_REST else r for r in rows]

    # 名簿由来と名簿外の残りは分けて出す。`perf(thread)` 行には `unregistered_rest` も
    # 1 本混じっており、全部足すと必ずプロセス CPU に一致してしまう（C14: 残り＝
    # プロセス − 名簿）。1 つの数にすると「名簿がプロセス全部を説明できている」と
    # 読めてしまい、名簿の穴（bevy の TaskPool など）が見えなくなる。
    rest_us = sum(r.cpu_us for r in rows if r.role == ROLE_UNREGISTERED_REST)
    registry_us = sum(r.cpu_us for r in rows if r.role != ROLE_UNREGISTERED_REST)

    lines = [
        f"[2] スレッド  unit=cpu  出所=perf(thread)／perf(process)  基準={basis}",
        scalar("window_s", fmt(window_s, 1)),
        scalar("process_cpu_s",
               fmt(process_cpu_us / 1e6, 3) if process_cpu_us is not None else PLACEHOLDER),
        scalar("registry_cpu_s", fmt(registry_us / 1e6, 3)),
        scalar("unregistered_rest_cpu_s", fmt(rest_us / 1e6, 3)),
        scalar("snapshots", str(len(snaps))),
    ]
    if basis == "cumulative":
        lines.append(note(
            "スナップショットが 1 枚しかないので差が取れない。"
            "累積値をそのまま使った（basis=cumulative）"
        ))
    if process_cpu_us is None:
        lines.append(note("perf(process) 行が無いので占有率を出せない（分母が無い）"))

    by_role: dict[str, int] = {}
    for row in rows:
        # トークのスレッドは 1 本ずつ役割が違う（sakura-talk-{id}）ので役割別でだけ畳む（D8・#22）。
        role = re.sub(r"^(actor:.+)-\d+$", r"\1-*", row.role)
        by_role[role] = by_role.get(role, 0) + row.cpu_us
    role_items = sorted(by_role.items(), key=lambda kv: (-kv[1], kv[0]))
    lines.append(f"  役割別（上位 {top}）")
    lines.extend(render_table(
        4,
        [Column("rank", 4, True), Column("role", 26), Column("cpu_s", 10, True),
         Column("share_pct", 11, True)],
        [[str(i), role, fmt(us / 1e6, 3), share_pct(float(us), whole)]
         for i, (role, us) in enumerate(role_items[:top], start=1)],
    ))
    lines.extend(truncated(len(role_items), min(top, len(role_items))))

    thread_items = sorted(rows, key=lambda r: (-r.cpu_us, r.tid))
    lines.append(f"  スレッド別（上位 {top}）")
    lines.extend(render_table(
        4,
        [Column("rank", 4, True), Column("tid", 8, True), Column("name", 24),
         Column("role", 24), Column("cpu_s", 10, True), Column("share_pct", 11, True)],
        [[str(i), str(r.tid), r.name, r.role, fmt(r.cpu_us / 1e6, 3),
          share_pct(float(r.cpu_us), whole)]
         for i, r in enumerate(thread_items[:top], start=1)],
    ))
    lines.extend(truncated(len(thread_items), min(top, len(thread_items))))

    # 段③が利用可なら、名簿由来とは別系統の「サンプリング由来」を併記する（設計 C14）。
    # 名簿に載らない TaskPool の各本は、こちらにしか現れない。
    if dump is not None and dump.samples:
        counts: dict[str, int] = {}
        for _ts, tid, _func in dump.samples:
            counts[tid] = counts.get(tid, 0) + 1
        total = float(len(dump.samples))
        items = sorted(counts.items(), key=lambda kv: (-kv[1], tid_sort_key(kv[0])))
        lines.append("  [2b] スレッド（サンプリング由来・unit=samples・出所=dump.txt）")
        lines.extend(render_table(
            4,
            [Column("rank", 4, True), Column("tid", 10, True), Column("samples", 9, True),
             Column("share_pct", 11, True)],
            [[str(i), tid, str(count), share_pct(float(count), total)]
             for i, (tid, count) in enumerate(items[:top], start=1)],
        ))
        lines.extend(truncated(len(items), min(top, len(items))))
    return lines


# --- 段④ 相 -----------------------------------------------------------------


def stage_phase(scan: LogScan, top: int) -> list[str]:
    """[tick] kind=window 行から tick/秒・省略率・13 本のスケジュール別（unit=wall）。"""
    if not scan.ticks:
        return [
            "[4] 相 UNAVAILABLE reason=no_tick_lines",
            note("[tick] kind=window 行が 1 本も無い。順位付けの走行では wintf::tick=debug を点けること。"),
        ]

    ticks = sum(t.ticks for t in scan.ticks)
    skipped = sum(t.skipped for t in scan.ticks)
    heartbeat = sum(t.heartbeat for t in scan.ticks)
    wall_us = sum(t.wall_us for t in scan.ticks)
    ui_cpu_us = sum(t.ui_cpu_us for t in scan.ticks)
    # 窓の実長は 1 秒ちょうどではない（`t_ms=1006` のように振れる）。読み手が
    # ticks_per_s を検算できるよう 3 桁まで出す——丸めた窓長で割ると数が合わない。
    window_s = sum(t.t_ms for t in scan.ticks) / 1000.0

    lines = [
        "[4] 相  unit=wall  出所=[tick] kind=window",
        scalar("windows", str(len(scan.ticks))),
        scalar("ticks_total", str(ticks)),
        scalar("skipped_total", str(skipped)),
        scalar("heartbeat_total", str(heartbeat)),
        scalar("window_s", fmt(window_s, 3)),
        scalar("ticks_per_s", fmt(ticks / window_s, 2) if window_s > 0 else PLACEHOLDER),
        scalar("skip_ratio_pct", share_pct(float(skipped), float(ticks + skipped))),
        scalar("heartbeat_ratio_pct", share_pct(float(heartbeat), float(ticks))),
        scalar("avg_tick_wall_us", fmt(wall_us / ticks, 1) if ticks else PLACEHOLDER),
        scalar("max_tick_wall_us", str(max(t.max_us for t in scan.ticks))),
        scalar("ui_cpu_us_per_s", fmt(ui_cpu_us / window_s, 1) if window_s > 0 else PLACEHOLDER),
        scalar("wall_us_total", str(wall_us)),
        f"  13 本のスケジュール（上位 {top}）",
    ]
    sums = {name: sum(t.per_schedule_us[name] for t in scan.ticks) for name in SCHEDULE_NAMES}
    items = sorted(sums.items(), key=lambda kv: (-kv[1], kv[0]))
    lines.extend(render_table(
        4,
        [Column("rank", 4, True), Column("name", 18), Column("sum_us", 10, True),
         Column("share_pct", 10, True), Column("avg_us", 10, True)],
        [[str(i), name, str(us), share_pct(float(us), float(wall_us)),
          fmt(us / ticks, 1) if ticks else PLACEHOLDER]
         for i, (name, us) in enumerate(items[:top], start=1)],
    ))
    lines.extend(truncated(len(items), min(top, len(items))))
    # 13 本は tick 1 回の中を刻んだものなので、合計は壁時計を超えないはずである。
    # 超えていたら観測行が壊れており、share_pct の合計も 100% を超える。黙らない。
    schedules_us = sum(sums.values())
    if schedules_us > wall_us:
        lines.append(note(
            f"13 本の合計 {schedules_us}µs が wall_us_total {wall_us}µs を超えている"
            "——share_pct の合計は 100% を超える（観測行が壊れている疑い）"
        ))
    lines.append(note("13 本は壁時計 µs（GPU 待ちを含む）。ui_cpu_us だけが UI スレッドの CPU 時間。"))
    return lines


# --- 組み立て ----------------------------------------------------------------


def build_report(
    run_dir: Path,
    stages: tuple[str, ...],
    top: int,
    sampling: str,
    unavailable_reason: str | None,
) -> list[str]:
    run_log, cpu_csv = run_dir / "run.log", run_dir / "cpu.csv"
    meta_path, dump_path = run_dir / "run-meta.txt", run_dir / "dump.txt"

    for path, what in ((run_log, "走行ログ（run.log）"), (cpu_csv, "CPU 時系列（cpu.csv）")):
        if not path.is_file():
            raise measure_failed(f"{what} がありません: {path}")
        if path.stat().st_size == 0:
            raise measure_failed(f"{what} が空です: {path}")

    scan = scan_run_log(run_log)
    cpu = read_cpu_csv(cpu_csv)
    meta = read_run_meta(meta_path) if meta_path.is_file() else None

    # 0 バイトの dump は「在るが空」＝採取の失敗として parse_dump が止める（#62）。
    dump_present = dump_path.is_file()
    if sampling == "true":
        if not dump_present:
            raise measure_failed(
                f"段③は利用可と指定されたのに dump がありません: {dump_path}",
                ["--sampling-available true は採取が成立したという申告なので、"
                 "成果物が無いのは道具の不具合として扱う。"],
            )
        available = True
    elif sampling == "false":
        available = False
    else:
        available = dump_present

    dump: DumpScan | None = None
    reason = unavailable_reason
    if available:
        dump = parse_dump(dump_path)
    elif reason is None:
        reason = "no_dump" if sampling != "false" else "declared_unavailable"

    lines = [
        f"perf-rank {SCRIPT_VERSION} run={run_dir.resolve().name} stages={','.join(stages)} top={top}"
    ]
    for stage in stages:
        lines.append("")
        if stage == "process":
            lines.extend(stage_process(scan, cpu, meta))
        elif stage == "thread":
            lines.extend(stage_thread(scan, dump, top))
        elif stage == "function":
            lines.extend(stage_function(dump, reason, top))
        else:
            lines.extend(stage_phase(scan, top))
    return lines


# --- 自己較正 ----------------------------------------------------------------


@dataclass
class SelftestCase:
    name: str
    directory: Path
    title: str
    expected_exit: int
    argv: list[str]
    dump_path: str | None
    stderr_substrings: list[str]


def script_dir() -> Path:
    return Path(__file__).resolve().parent


def _read_case(directory: Path) -> SelftestCase:
    """ケース 1 件の台帳（case.txt）を読む。知らない項目は拒む（黙って無視しない）。"""
    case_path = directory / SELFTEST_CASE_FILENAME
    values, multi = selftest_common.read_case_fields(
        case_path, frozenset({"title", "exit", "argv", "dump_path", "note"}),
        frozenset({"stderr_substr"}), bad_input,
    )
    if "exit" not in values:
        raise bad_input(
            f"{case_path}: exit がありません（期待終了コードの無いケースは較正になりません）"
        )
    try:
        expected_exit = int(values["exit"])
    except ValueError as exc:
        raise bad_input(f"{case_path}: exit が整数ではありません（{values['exit']!r}）") from exc
    return SelftestCase(
        directory.name, directory, values.get("title") or "(説明なし)", expected_exit,
        values.get("argv", "").split(), values.get("dump_path"), multi["stderr_substr"],
    )


def _run_case(case: SelftestCase, workspace: Path) -> tuple[list[str], set[int]]:
    work = workspace / case.name
    shutil.copytree(case.directory, work)
    if case.dump_path:
        source = (case.directory / case.dump_path).resolve()
        if not source.is_file():
            return ([f"dump_path が指す断片がありません: {source}"], set())
        shutil.copyfile(source, work / "dump.txt")

    reasons: list[str] = []
    reproduced: set[int] = set()
    code, _out, err = selftest_common.invoke(main, [str(work)] + case.argv, EXIT_BAD_INPUT)
    if code == case.expected_exit:
        reproduced.add(code)
    else:
        reasons.append(f"終了コード: 期待 {case.expected_exit} 実際 {code}")

    expected_path = case.directory / SELFTEST_EXPECTED_FILENAME
    produced = work / OUTPUT_FILENAME
    if case.expected_exit == EXIT_OK:
        if not expected_path.is_file():
            reasons.append(
                f"{SELFTEST_EXPECTED_FILENAME} がありません（期待出力の無い合格側は較正になりません）"
            )
        elif not produced.is_file():
            reasons.append(f"{OUTPUT_FILENAME} が作られていません")
        else:
            want = selftest_common.read_normalized(expected_path)
            got = selftest_common.read_normalized(produced)
            if want != got:
                reasons.append(selftest_common.first_difference(OUTPUT_FILENAME, want, got))
    elif produced.is_file():
        reasons.append(f"失敗したのに {OUTPUT_FILENAME} を書いています（途中結果を残さないこと）")

    for substring in case.stderr_substrings:
        if substring not in err:
            reasons.append(f"標準エラーに {substring!r} が出ていません")
    return reasons, reproduced


def run_selftest() -> int:
    """fixtures-loop/rank/ の既知ケースを逐語再現する（要件 6.7・7.5）。"""
    root = script_dir() / SELFTEST_FIXTURES_DIRNAME / SELFTEST_RANK_SUBDIR
    directories, materials = selftest_common.list_case_dirs(
        root, SELFTEST_CASE_FILENAME, bad_input, materials_allowed=True
    )
    cases = [_read_case(child) for child in directories]

    results: list[tuple[str, str, list[str]]] = []
    reproduced_all: set[int] = set()
    with selftest_common.work_dir("perf-rank-selftest-") as temporary:
        for case in cases:
            reasons, reproduced = _run_case(case, temporary)
            reproduced_all |= reproduced
            results.append((case.name, case.title, reasons))

    # 木全体の見張り（⑶ stderr の ASCII・⑸ README の例は、その直しのタスクで真にする）。
    results.append(("(check_tree)", "木全体の見張り",
                    selftest_common.check_tree(ascii_stderr=False, readme_examples=False)))
    ng_count = selftest_common.finish(
        results, materials, SELFTEST_CASE_FILENAME, reproduced_all, SELFTEST_REQUIRED_RED_EXITS
    )
    return EXIT_OK if ng_count == 0 else EXIT_FAIL


# --- 入口 --------------------------------------------------------------------


class Parser(argparse.ArgumentParser):
    """引数不正の終了コードを 3 に揃える（argparse の既定 2 は使わない）。"""

    def error(self, message: str):  # type: ignore[override]
        sys.stderr.write(f"{STDERR_PREFIX} 引数が不正です: {message}\n")
        sys.stderr.write(self.format_usage())
        raise SystemExit(EXIT_BAD_INPUT)


def build_parser() -> Parser:
    parser = Parser(
        prog="perf-rank.py",
        description="1 走行の成果物から 4 段の順位表を出す（要件 2.1・2.6・2.10・5.4・6.7）",
    )
    parser.add_argument("run_dir", nargs="?", help="走行ディレクトリ（run.log・cpu.csv がある場所）")
    parser.add_argument("--stage", default="all", choices=("all",) + STAGE_ALL,
                        help="出す段（既定 all）")
    parser.add_argument("--top", type=int, default=DEFAULT_TOP,
                        help=f"各段の上位件数（既定 {DEFAULT_TOP}）")
    parser.add_argument("--out", default=OUTPUT_FILENAME,
                        help=f"出力先（相対なら走行ディレクトリの中・既定 {OUTPUT_FILENAME}）")
    parser.add_argument("--sampling-available", default="auto", choices=("auto", "true", "false"),
                        help="段③が採れたか（既定 auto＝dump.txt の有無で決める）")
    parser.add_argument("--unavailable-reason", default=None,
                        help="段③が採れなかった理由（採取側から渡す）")
    parser.add_argument("--selftest", action="store_true", help="自己較正（fixtures-loop/rank/）")
    return parser


def main(argv: list[str]) -> int:
    args = build_parser().parse_args(argv)

    try:
        if args.selftest:
            return run_selftest()

        if args.run_dir is None:
            raise bad_input("走行ディレクトリを指定してください（--selftest を除いて必須です）")
        if args.top < 1:
            raise bad_input(f"--top は 1 以上です（{args.top}）")
        run_dir = Path(args.run_dir)
        if not run_dir.is_dir():
            raise bad_input(f"走行ディレクトリがありません: {run_dir}")

        stages = STAGE_ALL if args.stage == "all" else (args.stage,)
        lines = build_report(
            run_dir, stages, args.top, args.sampling_available, args.unavailable_reason
        )

        out_path = Path(args.out)
        if not out_path.is_absolute():
            out_path = run_dir / out_path
        try:
            out_path.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
        except OSError as exc:
            raise bad_input(f"順位表を書けません: {out_path}", [str(exc)]) from exc
        print(f"{STDERR_PREFIX} 順位表を書きました: {out_path}")
        return EXIT_OK
    except RankError as exc:
        label = {EXIT_BAD_INPUT: "入力不正", EXIT_MEASURE_FAILED: "計測失敗"}.get(exc.code, "失敗")
        sys.stderr.write(f"{STDERR_PREFIX} {label}: {exc.message}\n")
        for detail in exc.details:
            sys.stderr.write(f"  - {detail}\n")
        return exc.code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
