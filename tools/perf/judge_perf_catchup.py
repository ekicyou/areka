#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""`judge-perf.py` 0.4.0 の §9 追補——catch-up の系統別・時刻突合と仮説の数値化（要件 2.9）。

【なぜ別ファイルか】`judge-perf.py` は既に 3,500 行を超えている（本 spec 以前からの状態）。
0.4.0 で足す「表を組んで印字する」部分をそこへ積み増すと読めなくなるので、ここへ分けた。

【ここに無いもの】判定式は 1 つも無い。合否に関わる計算は `judge-perf.py` 側にしかなく、
このモジュールがするのは **集計と表示だけ** である。較正値も持たない——閾値（比の下限・
直前何秒を数えるか）は呼び出し側から渡される（値の所在は `judge-perf.py` 冒頭のバナー 1 箇所）。

【何を出すか（要件 2.9）】
  * catch-up の各発生について、時刻・発行元の系統・直前の表示成立点との差（秒）・
    直前 10 秒の表示成立点数（発話再生中かどうかの代理）・同時刻の `[tick]` 窓の
    壁時計合計と省略数を 1 行ずつ
  * 「フレーム駆動の負荷が CPU 競合でティッカーの起床を遅らせる」という **仮説** の成否を
    数値で。catch-up が起きた `[tick]` 窓の壁時計平均 ÷ 全窓の壁時計平均を比として出し、
    比が下限以上なら 成立・下回れば 不成立・`[tick]` 行が無ければ 判定不能 と機械が語を付ける

【判定不能を 不成立 と書かないこと】`[tick]` を点灯せずに採った走行では、この仮説は
「調べたが違った」のではなく「調べていない」のである。不成立 と書くと後者が前者に化ける。
"""

from __future__ import annotations

import bisect
from dataclasses import dataclass, field
from datetime import datetime


@dataclass(frozen=True)
class TickWindow:
    """`[tick] kind=window` 1 行（＝フレーム駆動の 1 秒窓）。

    `at` は **窓が閉じた時刻**（行の時刻）であり、窓が覆う区間は `[at − span_sec, at]` である。
    `span_sec` は `t_ms` 由来。読めない・欠けている値は `None` にして、無いものを 0 と
    書かない（0 と「測れていない」は違う）。
    """

    at: datetime
    span_sec: float | None
    wall_us: int | None
    max_us: int | None
    ticks: int | None
    skipped: int | None


@dataclass(frozen=True)
class CatchupRow:
    """catch-up 1 件と、その時刻の周辺状況。"""

    at: datetime
    target: str
    in_steady: bool
    #: 直前の表示成立点からの秒数。直前に 1 つも無ければ None。
    since_last_show_s: float | None
    #: 直前 `show_window_sec` 秒の表示成立点数（発話再生中かどうかの代理）。
    shows_in_window: int
    #: この時刻を覆う `[tick]` 窓。無ければ None。
    tick: TickWindow | None


@dataclass
class CatchupAnalysis:
    """§9 追補が印字する材料の全部。"""

    rows: list[CatchupRow] = field(default_factory=list)
    #: 時刻を読めなかった catch-up 行の数（表には出せないが、黙って消さない）。
    undated: int = 0
    #: 系統別の件数（全区間・定常状態）。鍵は target の値。
    total_by_target: dict[str, int] = field(default_factory=dict)
    steady_by_target: dict[str, int] = field(default_factory=dict)
    #: `[tick]` 窓の総数と、壁時計合計を読めた窓の数。
    tick_windows: int = 0
    tick_windows_with_wall: int = 0
    #: catch-up を覆った窓の数（同じ窓に複数の catch-up が入れば 1 と数える）。
    matched_windows: int = 0
    mean_wall_all_us: float | None = None
    mean_wall_catchup_us: float | None = None
    ratio: float | None = None
    #: 機械が付ける語。成立／不成立／判定不能 のいずれか。
    verdict_word: str = "判定不能"
    #: 語をそう付けた理由（1 行）。
    verdict_reason: str = ""
    #: 省略率（全窓の skipped 合計 ÷（ticks + skipped）合計）。読めなければ None。
    skip_ratio: float | None = None


def _covering_window(
    windows: list[TickWindow], closes: list[datetime], at: datetime, fallback_sec: float
) -> TickWindow | None:
    """時刻 `at` を覆う `[tick]` 窓を返す。

    窓は閉じた時刻で並んでいるので、`at` 以上で最も早く閉じた窓が唯一の候補である。
    その窓の開き際（`閉じた時刻 − span_sec`）が `at` 以下なら覆っている。
    `t_ms` を読めなかった窓は `fallback_sec` 以内に閉じたかどうかで代用する
    （窓幅が分からないまま「覆っている」と言い切らないための上限）。
    """
    index = bisect.bisect_left(closes, at)
    if index >= len(windows):
        return None
    window = windows[index]
    gap = (window.at - at).total_seconds()
    limit = window.span_sec if window.span_sec is not None else fallback_sec
    return window if 0.0 <= gap <= limit else None


def analyze(
    *,
    events: list[tuple[datetime | None, str]],
    show_times: list[datetime],
    tick_windows: list[TickWindow],
    steady_from: datetime,
    show_window_sec: float,
    ratio_min: float,
    tick_match_max_sec: float,
) -> CatchupAnalysis:
    """catch-up の各発生を周辺状況と突き合わせ、仮説の比まで出す。

    `events` は `(時刻, 系統)` の並び（時刻は読めなければ None）。`show_times` は
    表示成立点の時刻（昇順であること）。`tick_windows` は閉じた時刻の昇順であること。
    """
    analysis = CatchupAnalysis()
    windows = sorted(tick_windows, key=lambda w: w.at)
    closes = [w.at for w in windows]
    shows = sorted(show_times)

    for at, target in events:
        analysis.total_by_target[target] = analysis.total_by_target.get(target, 0) + 1
        if at is None:
            analysis.undated += 1
            continue
        in_steady = at >= steady_from
        if in_steady:
            analysis.steady_by_target[target] = analysis.steady_by_target.get(target, 0) + 1

        # 直前の表示成立点（at と同時刻のものは「直前」に含める）。
        prev_index = bisect.bisect_right(shows, at) - 1
        since = (at - shows[prev_index]).total_seconds() if prev_index >= 0 else None
        # 直前 show_window_sec 秒に成立した表示の数（発話再生中かどうかの代理）。
        window_from = at - _timedelta_seconds(show_window_sec)
        in_window = prev_index + 1 - bisect.bisect_left(shows, window_from)

        analysis.rows.append(
            CatchupRow(
                at=at,
                target=target,
                in_steady=in_steady,
                since_last_show_s=since,
                shows_in_window=max(0, in_window),
                tick=_covering_window(windows, closes, at, tick_match_max_sec),
            )
        )

    analysis.rows.sort(key=lambda r: (r.at, r.target))

    # --- 仮説の比 ---------------------------------------------------------
    analysis.tick_windows = len(windows)
    walls = [w.wall_us for w in windows if w.wall_us is not None]
    analysis.tick_windows_with_wall = len(walls)

    ticks_total = sum(w.ticks for w in windows if w.ticks is not None)
    skipped_total = sum(w.skipped for w in windows if w.skipped is not None)
    if windows and (ticks_total + skipped_total) > 0:
        analysis.skip_ratio = skipped_total / (ticks_total + skipped_total)

    matched: dict[datetime, int] = {}
    for row in analysis.rows:
        if row.tick is not None and row.tick.wall_us is not None:
            matched[row.tick.at] = row.tick.wall_us
    analysis.matched_windows = len(matched)

    if not windows:
        analysis.verdict_word = "判定不能"
        analysis.verdict_reason = (
            "[tick] kind=window の行が 1 本も無い（点灯せずに採った走行では"
            "「調べたが違った」と「調べていない」を区別できない）"
        )
        return analysis
    if not walls:
        analysis.verdict_word = "判定不能"
        analysis.verdict_reason = "[tick] 行はあるが wall_us を 1 本も読めない"
        return analysis

    analysis.mean_wall_all_us = sum(walls) / len(walls)
    if not matched:
        analysis.verdict_word = "判定不能"
        analysis.verdict_reason = (
            "catch-up の時刻を覆う [tick] 窓が 1 つも無い"
            "（catch-up と [tick] の点灯区間が重なっていない）"
        )
        return analysis
    if analysis.mean_wall_all_us <= 0.0:
        analysis.verdict_word = "判定不能"
        analysis.verdict_reason = "全窓の wall_us 平均が 0 で比を取れない"
        return analysis

    analysis.mean_wall_catchup_us = sum(matched.values()) / len(matched)
    analysis.ratio = analysis.mean_wall_catchup_us / analysis.mean_wall_all_us
    if analysis.ratio >= ratio_min:
        analysis.verdict_word = "成立"
        analysis.verdict_reason = f"比 {analysis.ratio:.3f} >= 下限 {ratio_min:.3f}"
    else:
        analysis.verdict_word = "不成立"
        analysis.verdict_reason = f"比 {analysis.ratio:.3f} < 下限 {ratio_min:.3f}"
    return analysis


def _timedelta_seconds(seconds: float):
    from datetime import timedelta

    return timedelta(seconds=seconds)


def _fmt_int(value: int | None) -> str:
    return "-" if value is None else str(value)


def render(
    emit,
    analysis: CatchupAnalysis,
    *,
    targets: tuple[str, ...],
    show_window_sec: float,
    ratio_min: float,
    max_rows: int,
) -> None:
    """§9 の追補を印字する。`emit` は 1 行を受け取る関数（`Report.line`）。"""
    emit("")
    emit(f"-- 系統別・各発生の突合（要件 2.9・0.4.0 で追加） " + "-" * 30)
    emit("  系統は行末の target フィールド（tracing のフィールドであってメタデータの")
    emit("  target 指示子ではない）で分ける。dispatcher と kanade は文言が同一なので、")
    emit("  文言では分けられない（ticker.rs:203-206, 223-226, 305-308）。")
    for target in targets:
        total = analysis.total_by_target.get(target, 0)
        steady = analysis.steady_by_target.get(target, 0)
        emit(f"    系統 target={target}: 全区間 {total} 件（うち定常状態 {steady} 件）")
    others = sorted(set(analysis.total_by_target) - set(targets))
    for target in others:
        total = analysis.total_by_target.get(target, 0)
        steady = analysis.steady_by_target.get(target, 0)
        emit(
            f"    系統 target={target}: 全区間 {total} 件（うち定常状態 {steady} 件）"
            "   ← 既知の 3 系統に無い値（ticker.rs と本スクリプトの語彙がずれた疑い）"
        )
    if analysis.undated:
        emit(f"    時刻を読めなかった行: {analysis.undated} 件（表には出せない）")

    emit("")
    emit(
        f"  各発生の周辺状況（前表示差＝直前の表示成立点からの秒数／"
        f"{show_window_sec:.0f}s表示＝直前 {show_window_sec:.0f} 秒の表示成立点数＝"
        "発話再生中かどうかの代理）:"
    )
    emit(
        "    時刻              区間  系統            前表示差   "
        f"{show_window_sec:.0f}s表示  tick.wall_us  tick.skipped"
    )
    if not analysis.rows:
        emit("    （時刻を読めた catch-up がありません）")
    for row in analysis.rows[:max_rows]:
        since = "     -" if row.since_last_show_s is None else f"{row.since_last_show_s:6.3f}"
        wall = _fmt_int(row.tick.wall_us) if row.tick else "-"
        skipped = _fmt_int(row.tick.skipped) if row.tick else "-"
        emit(
            f"    {row.at.strftime('%H:%M:%S.%f')}  "
            f"{'定常' if row.in_steady else '過渡'}  {row.target:<14s} "
            f"{since}s  {row.shows_in_window:>7d}  {wall:>12s}  {skipped:>12s}"
        )
    if len(analysis.rows) > max_rows:
        emit(f"    …ほか {len(analysis.rows) - max_rows} 件（表示上限 {max_rows} 行）")

    emit("")
    emit("  仮説「フレーム駆動の負荷が CPU 競合でティッカーの起床を遅らせる」（要件 2.9）:")
    emit(
        f"    規則: catch-up を覆った [tick] 窓の wall_us 平均 ÷ 全 [tick] 窓の wall_us 平均。"
    )
    emit(
        f"          比 >= {ratio_min:.3f} なら 成立・下回れば 不成立・"
        "[tick] 行が無ければ 判定不能。"
    )
    emit(
        f"    [tick] 窓 {analysis.tick_windows} 個"
        f"（wall_us を読めた窓 {analysis.tick_windows_with_wall} 個・"
        f"catch-up を覆った窓 {analysis.matched_windows} 個）"
    )
    if analysis.mean_wall_all_us is not None:
        emit(f"    全窓の wall_us 平均       = {analysis.mean_wall_all_us:.1f}us")
    if analysis.mean_wall_catchup_us is not None:
        emit(f"    catch-up 窓の wall_us 平均 = {analysis.mean_wall_catchup_us:.1f}us")
    if analysis.ratio is not None:
        emit(f"    比                        = {analysis.ratio:.3f}")
    emit(f"    仮説: {analysis.verdict_word}（{analysis.verdict_reason}）")
    emit("    この仮説は因果の主張ではない。比が大きいことは「重い窓と catch-up が同じ秒に")
    emit("    居合わせた」ことしか言わない（要件 2.9 が「仮説」と書いているのはこのためである）。")
