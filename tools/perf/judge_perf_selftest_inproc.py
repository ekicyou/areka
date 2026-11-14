#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""judge-perf の、検体では作れない場面の較正（D12・T6・T8・T24）。

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
from pathlib import Path
from unittest import mock

#: 検体の置き場（judge-perf と同じ場所の fixtures/）。
FIXTURES = Path(__file__).resolve().parent / "fixtures"

#: T8 で明示指定に足す、どの検体にも出ない系列。
ABSENT_SERIES = ("TargetId(9)", "9999")


def _call(judge: types.ModuleType, case: str, build: str = "release") -> tuple[int, str, str]:
    """`main` を呼び、終了コード・標準出力・標準エラーを返す。外へ抜けた例外は Python の既定の 1。"""
    directory = FIXTURES / case
    argv = [str(directory / "run.log"), str(directory / "cpu.csv"), "--mode", "verdict", "--build", build]
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        try:
            code = judge.main(argv)
        except SystemExit as exc:
            code = exc.code if isinstance(exc.code, int) else 1
        except Exception:  # noqa: BLE001  （本体が捕まえなかった例外＝インタプリタは 1 で終わる）
            code = 1
    return code, out.getvalue(), err.getvalue()


def _last_line(text: str) -> str:
    """罫線（= や - だけの行）と空行を飛ばした最後の行（perf-loop の LastLine と同じ取り方）。"""
    for line in reversed(text.splitlines()):
        stripped = line.strip()
        if stripped and stripped.strip("=-_* "):
            return stripped
    return ""


def _non_ascii(text: str) -> list[str]:
    return [line for line in text.splitlines() if not line.isascii()]


def run(judge: types.ModuleType) -> list[str]:
    """T6・T8・T24 を確かめ、食い違いの理由を返す（空なら合格）。"""
    reasons: list[str] = []

    # T6: 想定外の例外は道具の故障。不合格(1)と読ませず 3 で終わり、traceback を stderr に残す（#6）。
    def broken_scan(_path):
        raise RuntimeError("injected by judge_perf_selftest_inproc (T6)")

    with mock.patch.object(judge, "scan_log", broken_scan):
        code, _out, err = _call(judge, "H11_healthy_judged_series")
    if code != 3:
        reasons.append(f"T6: unexpected exception gave exit {code}, expected 3")
    if "Traceback" not in err:
        reasons.append("T6: no Traceback on stderr")
    if _last_line(err) != "JUDGE-PERF RESULT code=3":
        reasons.append(f"T6: last stderr line is {ascii(_last_line(err))}")

    # T8: 明示指定 2 系列で A が上限越え・B が未観測なら不合格(1)。不合格が判定不能より先（#8）。
    series = (*judge.FRAME_INTERVAL_JUDGED_SERIES, ABSENT_SERIES)
    with mock.patch.object(judge, "FRAME_INTERVAL_JUDGED_SERIES", series):
        code, _out, _err = _call(judge, "S3_explicit_over_limit")
    if code != 1:
        reasons.append(f"T8: over limit + unobserved series gave exit {code}, expected 1")

    # T24: 最後の行は ASCII の決まった形・stderr は全行 ASCII（D9）。判定式で判定不能（E1）と
    # 読みの失敗で判定不能（E2）の両方を見る。
    for case in ("E1_steady_state_empty", "E2_show_kind_missing"):
        code, out, err = _call(judge, case)
        last = _last_line(err) or _last_line(out)
        if code != 2 or last != "JUDGE-PERF RESULT code=2":
            reasons.append(f"T24: {case} gave exit {code}, last line {ascii(last)}")
        bad = _non_ascii(err)
        if bad:
            reasons.append(f"T24: {case} stderr has non-ASCII lines: {ascii(bad[0])}")

    # T24: 自己較正そのものの最後の行。検体を 1 件に絞り、ここ（run）を空にして再帰を断つ。
    one = judge._selftest_read_case(FIXTURES / "H11_healthy_judged_series")
    out = io.StringIO()
    with mock.patch.object(judge, "_selftest_collect", lambda _root: [one]), \
            mock.patch.object(sys.modules[__name__], "run", lambda _judge: []), \
            contextlib.redirect_stdout(out), contextlib.redirect_stderr(io.StringIO()):
        judge.run_selftest()
    if not re.fullmatch(r"SELFTEST RESULT ok=\d+ ng=\d+", _last_line(out.getvalue())):
        reasons.append(f"T24: selftest last line is {ascii(_last_line(out.getvalue()))}")
    return reasons
