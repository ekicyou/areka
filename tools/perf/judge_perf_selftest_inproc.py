#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""judge-perf の、検体では作れない場面の較正（D12・T6・T8・T24 ほか）。

judge-perf の `--selftest` が本体の加群（`sys.modules[__name__]`）を `run` に渡す。ここは
その関数と定数を `unittest.mock.patch.object` で差し替え、実運用と同じ入口 `main(argv)` を
呼んで終了コードと端末の文を確かめる。差し替えは `with` の中だけ。乱数・時計・一時フォルダは
使わない。このモジュールは judge-perf を読み込まない（本体から渡される一方向）。
"""

from __future__ import annotations

import contextlib
import io
import re
import sys
import types
from datetime import datetime, timedelta, timezone
from pathlib import Path
from unittest import mock

#: 検体の置き場（judge-perf と同じ場所の fixtures/）。
FIXTURES = Path(__file__).resolve().parent / "fixtures"

#: T8 で明示指定に足す、どの検体にも出ない系列。
ABSENT_SERIES = ("TargetId(9)", "9999")


def _main(judge: types.ModuleType, argv: list[str]) -> tuple[int, str, str]:
    """`main` を呼び、終了コード・標準出力・標準エラーを返す。外へ抜けた例外は Python の既定の 1。"""
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        try:
            code = judge.main(argv)
        except SystemExit as exc:
            code = exc.code if isinstance(exc.code, int) else 1
        except Exception:  # noqa: BLE001  （本体が捕まえなかった例外＝インタプリタは 1 で終わる）
            code = 1
    return code, out.getvalue(), err.getvalue()


def _call(
    judge: types.ModuleType, case: str, build: str = "release", mode: str = "verdict", *extra: str
) -> tuple[int, str, str]:
    directory = FIXTURES / case
    argv = [str(directory / "run.log"), str(directory / "cpu.csv"), "--mode", mode, "--build", build]
    return _main(judge, [*argv, *extra])


def _last_line(text: str) -> str:
    """罫線（= や - だけの行）と空行を飛ばした最後の行（perf-loop の LastLine と同じ取り方）。"""
    for line in reversed(text.splitlines()):
        stripped = line.strip()
        if stripped and stripped.strip("=-_* "):
            return stripped
    return ""


def _non_ascii(text: str) -> list[str]:
    return [line for line in text.splitlines() if not line.isascii()]


def _read_case_code(judge: types.ModuleType, read_text) -> int | None:
    """`case.txt` の中身を差し替えて `_selftest_read_case` を呼ぶ。読めれば None・断れば終了コード。"""
    with mock.patch.object(judge.Path, "read_text", read_text):
        try:
            judge._selftest_read_case(FIXTURES / "H11_healthy_judged_series")
        except judge.JudgeError as exc:
            return exc.code
        except Exception:  # noqa: BLE001  （関門をすり抜けた例外＝外へ出れば Python の既定の 1）
            return 1
    return None


def run(judge: types.ModuleType) -> tuple[int, list[str]]:
    """確かめを走らせ、（走った確かめの数, 食い違いの理由）を返す（理由が空なら合格）。

    数を返すのは「1 つも走らなかった」を合格と見分けるため（本体が 0 件を食い違いにする）。
    """
    reasons: list[str] = []
    checks = 0

    # T6: 想定外の例外は道具の故障。不合格(1)と読ませず 3 で終わり、traceback を stderr に残す（#6）。
    def broken_scan(_path):
        raise RuntimeError("injected by judge_perf_selftest_inproc (T6)")

    checks += 1
    with mock.patch.object(judge, "scan_log", broken_scan):
        code, _out, err = _call(judge, "H11_healthy_judged_series")
    if code != 3:
        reasons.append(f"T6: unexpected exception gave exit {code}, expected 3")
    if "Traceback" not in err:
        reasons.append("T6: no Traceback on stderr")
    if _last_line(err) != "JUDGE-PERF RESULT code=3":
        reasons.append(f"T6: last stderr line is {ascii(_last_line(err))}")

    # T6 の自己較正側: 自己較正の中の想定外の例外も 3 で終わり、最後の行を残す。
    def broken_collect(_root):
        raise RuntimeError("injected by judge_perf_selftest_inproc (selftest)")

    checks += 1
    with mock.patch.object(judge, "_selftest_collect", broken_collect):
        code, _out, err = _main(judge, ["--selftest"])
    if code != 3 or "Traceback" not in err or _last_line(err) != "JUDGE-PERF RESULT code=3":
        reasons.append(f"selftest crash: exit {code}, last stderr line {ascii(_last_line(err))}")

    # case.txt の読みの関門: UTF-8 で読めない・`=` の無い行・同じ鍵の 2 度書きは置き場の不備(3)。
    # contains と note は何度でも書ける（対照）。
    def text_of(body: str):
        return lambda _self, *_a, **_k: body

    def undecodable(_self, *_a, **_k):
        raise UnicodeDecodeError("utf-8", b"\xff", 0, 1, "invalid start byte")

    checks += 1
    head = "mode = verdict\nexit = 0\n"
    for label, reader, expected in (
        ("not UTF-8", undecodable, 3),
        ("line without =", text_of(head + "no equals sign\n"), 3),
        ("duplicate key", text_of(head + "exit = 1\n"), 3),
        ("repeated contains/note", text_of(head + "contains = a\ncontains = b\nnote = x\nnote = y\n"), None),
    ):
        got = _read_case_code(judge, reader)
        if got != expected:
            reasons.append(f"case.txt {label}: got {got}, expected {expected}")

    # T8: 明示指定 2 系列で A が上限越え・B が未観測なら不合格(1)。不合格が判定不能より先（#8）。
    # 未観測の系列は名指しされる。catch-up が 0 件の走行でも §9 の仮説の行は出る。
    series = (*judge.FRAME_INTERVAL_JUDGED_SERIES, ABSENT_SERIES)
    checks += 1
    with mock.patch.object(judge, "FRAME_INTERVAL_JUDGED_SERIES", series):
        code, out, _err = _call(judge, "S3_explicit_over_limit")
    if code != 1:
        reasons.append(f"T8: over limit + unobserved series gave exit {code}, expected 1")
    # 較正値の一覧にも系列名は出るので、未観測の理由の行そのものを探す。
    if f"{ABSENT_SERIES[0]} / surface_id={ABSENT_SERIES[1]}: 定常状態の perf 行が 1 本も無い" not in out:
        reasons.append(f"T8: the unobserved series {ABSENT_SERIES[0]} is not named as unobserved")
    if "  仮説: " not in out:
        reasons.append("T8: no hypothesis line in the catch-up section of a run without catch-up")

    # T24: 最後の行は ASCII の決まった形・stderr は全行 ASCII（D9）。判定式で判定不能（E1）と
    # 読みの失敗で判定不能（E2）の両方を見る。
    for case in ("E1_steady_state_empty", "E2_show_kind_missing"):
        checks += 1
        code, out, err = _call(judge, case)
        last = _last_line(err) or _last_line(out)
        if code != 2 or last != "JUDGE-PERF RESULT code=2":
            reasons.append(f"T24: {case} gave exit {code}, last line {ascii(last)}")
        bad = _non_ascii(err)
        if bad:
            reasons.append(f"T24: {case} stderr has non-ASCII lines: {ascii(bad[0])}")

    # T24: --emit-metrics を付けても最後の行は RESULT（指標の行はその前）。両モード。
    for mode in ("baseline", "verdict"):
        checks += 1
        code, out, _err = _call(judge, "C1_catchup_by_target_pass", "release", mode, "--emit-metrics")
        if code != 0 or "metric=" not in out or _last_line(out) != "JUDGE-PERF RESULT code=0":
            reasons.append(f"T24: {mode} --emit-metrics gave exit {code}, last line {ascii(_last_line(out))}")

    # 仮説の窓の数: 覆う窓に wall_us が無くても「覆う窓が無い」とは言わない。
    # 前表示差が無い行は単位を幅の外へ付けない（`     -s` にしない）。
    catchup = judge.judge_perf_catchup
    t0 = datetime(2026, 8, 14, tzinfo=timezone.utc)
    checks += 1
    analysis = catchup.analyze(
        events=[(t0 + timedelta(seconds=0.5), "dispatcher")],
        show_times=[],
        tick_windows=[
            catchup.TickWindow(t0 + timedelta(seconds=1), 1.0, None, None, 10, 0),
            catchup.TickWindow(t0 + timedelta(seconds=5), 1.0, 1000, 500, 10, 0),
        ],
        steady_events=[],
        show_window_sec=10.0,
        ratio_min=1.5,
        tick_match_max_sec=2.0,
    )
    if analysis.matched_windows != 1 or "wall_us を読めない" not in analysis.verdict_reason:
        reasons.append(
            f"catch-up analyze: covering window without wall_us gave {analysis.matched_windows} "
            f"window(s), reason {ascii(analysis.verdict_reason)}"
        )
    printed: list[str] = []
    catchup.render(printed.append, analysis, targets=("dispatcher",), show_window_sec=10.0, ratio_min=1.5, max_rows=5)
    if any("-s " in line for line in printed):
        reasons.append("catch-up render: the unit 's' is glued to the '-' of a missing value")

    # T24: 自己較正そのものの最後の行と、差し替えの較正の件数の行。検体を 1 件に絞り、
    # ここ（run）を決まった返り値にして再帰を断つ。0 件は「較正になっていない」と名指しされる。
    one = judge._selftest_read_case(FIXTURES / "H11_healthy_judged_series")
    for count in (1, 0):
        checks += 1
        out = io.StringIO()
        with mock.patch.object(judge, "_selftest_collect", lambda _root: [one]), \
                mock.patch.object(sys.modules[__name__], "run", lambda _judge, n=count: (n, [])), \
                contextlib.redirect_stdout(out), contextlib.redirect_stderr(io.StringIO()):
            judge.run_selftest()
        text = out.getvalue()
        if not re.fullmatch(r"SELFTEST RESULT ok=\d+ ng=\d+", _last_line(text)):
            reasons.append(f"T24: selftest last line is {ascii(_last_line(text))}")
        if f"差し替えの較正: {count} 件・食い違い 0 件" not in text:
            reasons.append(f"selftest with {count} in-process check(s) printed no count line")
        if (count == 0) != ("【較正になっていない】差し替えの較正" in text):
            reasons.append(f"selftest with {count} in-process check(s): wrong 'not calibrated' line")
    return checks, reasons
