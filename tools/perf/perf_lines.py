"""perf の道具が共有する、ログの行と `cpu.csv` の読み方（標準ライブラリのみ）。

判定式・較正値・終了コード・道具に固有の文はここに置かない。読みの失敗は、呼び手が
渡す作り手 `fail(message, details) -> Exception` で起こす（呼び手の bad_input・
inconclusive・measure_failed など）。ここの振る舞い（`parse_fields`・`read_cpu_csv`・
`percentile`）を変えるときは、judge-perf の較正値を変えたのと同じに扱い、judge-perf の
`SCRIPT_VERSION` を上げる。`parse_fields` の名前と切り方は Rust 側の写しと同じ規則。
"""

from __future__ import annotations

import csv
import math
import re
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Callable, Sequence

#: 呼び手の失敗の作り手。例外を返す（投げない）。
Fail = Callable[[str, list[str]], Exception]

#: ANSI エスケープ（CSI シーケンス）。読み込み時に落とす。
ANSI_RE = re.compile(r"\x1b\[[0-9;?]*[ -/]*[@-~]")

#: 行頭の時刻。`tracing-subscriber` 既定の SystemTime タイマは RFC3339（UTC・小数 6 桁）。
TIMESTAMP_RE = re.compile(r"^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?)Z\s")

#: `名前=` の出現位置。値は次の `名前=` の直前までとする。
#: `window_dpi=Some((120, 120))` のように値に空白が入る Debug 出力があるため、
#: 空白で切ってはいけない。
FIELD_KEY_RE = re.compile(r"(?:^|\s)([A-Za-z_][A-Za-z0-9_]*)=")


def strip_ansi(text: str) -> str:
    """ANSI エスケープを落とす。"""
    return ANSI_RE.sub("", text)


def to_utc_datetime(raw: str) -> datetime | None:
    """`YYYY-MM-DDTHH:MM:SS[.frac]` を UTC の時刻にする（小数は 6 桁へ）。読めなければ None。"""
    if "." in raw:
        head, frac = raw.split(".", 1)
        raw = f"{head}.{(frac + '000000')[:6]}"
    else:
        raw = raw + ".000000"
    try:
        return datetime.strptime(raw, "%Y-%m-%dT%H:%M:%S.%f").replace(tzinfo=timezone.utc)
    except ValueError:
        return None


def parse_timestamp(line: str) -> datetime | None:
    """行頭の時刻を返す。時刻で始まらない行（複数行メッセージの続き）は None。"""
    m = TIMESTAMP_RE.match(line)
    return to_utc_datetime(m.group(1)) if m else None


def parse_fields(tail: str) -> dict[str, str]:
    """`名前=値` の並びを辞書にする。値は次の名前の直前まで（空白を含んでよい・同じ名前は後勝ち）。"""
    keys = list(FIELD_KEY_RE.finditer(tail))
    fields: dict[str, str] = {}
    for i, m in enumerate(keys):
        start = m.end()
        end = keys[i + 1].start() if i + 1 < len(keys) else len(tail)
        fields[m.group(1)] = tail[start:end].strip()
    return fields


def split_after_message(line: str, message: str) -> str | None:
    """固定文言より後ろ（＝フィールドの並び）を返す。文言が無ければ None。"""
    idx = line.find(message)
    return None if idx < 0 else line[idx + len(message):]


def read_text_lines(path: Path, what: str, fail: Fail) -> list[str]:
    """テキストを行に割って返す。BOM・行末の CR・ANSI を落とす。

    読めないファイルは `fail` で起こす。文字化けは行を捨てずに置換で通す——
    数値もフィールド名も ASCII であり、化けるのは日本語のメッセージ部分だけである
    （途中で切れた最終行が壊れていても、それ以前の観測を捨てない）。
    """
    try:
        raw = path.read_bytes()
    except OSError as exc:
        raise fail(f"cannot read {what}: {path}", [ascii(str(exc))]) from exc
    text = raw.decode("utf-8-sig", errors="replace")
    return [strip_ansi(line.rstrip("\r")) for line in text.split("\n")]


@dataclass(frozen=True)
class CpuSample:
    at: datetime
    percent: float


def read_cpu_csv(
    path: Path,
    header: tuple[str, ...],
    fail: Fail,
    *,
    broken_note: str = "",
    read_fail: Fail | None = None,
) -> list[CpuSample]:
    """CPU 時系列を読む。ヘッダは契約と厳密一致（版ずれを黙って通さない）。

    行番号は空行も含めて 1 から数える。有限でない CPU 値（NaN・inf）は壊れた行。
    ファイル自体を読めないときは `read_fail`（省けば `fail`）で起こす。
    """
    lines = read_text_lines(path, "CPU series (cpu.csv)", read_fail or fail)
    rows = [(no, row) for no, row in enumerate(csv.reader(lines), start=1) if row]
    if not rows:
        raise fail(f"cpu.csv is empty: {path}", [])

    got = tuple(cell.strip() for cell in rows[0][1])
    if got != header:
        raise fail(
            "cpu.csv header differs from the contract (capture script version mismatch?)",
            [f"expected: {','.join(header)}", f"actual: {ascii(','.join(got))}"],
        )

    samples: list[CpuSample] = []
    broken: list[str] = []
    for line_no, row in rows[1:]:
        if len(row) < 2:
            broken.append(f"line {line_no}: too few columns ({ascii(row)})")
            continue
        at = parse_timestamp(row[0].strip() + " ")
        if at is None:
            broken.append(f"line {line_no}: unreadable timestamp ({ascii(row[0])})")
            continue
        try:
            percent = float(row[1].strip())
        except ValueError:
            broken.append(f"line {line_no}: CPU value is not a number ({ascii(row[1])})")
            continue
        if not math.isfinite(percent):
            broken.append(f"line {line_no}: CPU value is not finite ({ascii(row[1])})")
            continue
        samples.append(CpuSample(at=at, percent=percent))

    if broken:
        raise fail(f"cpu.csv has {len(broken)} unreadable rows{broken_note}", broken[:20])
    if not samples:
        raise fail(f"cpu.csv has no sample: {path}", [])
    return samples


def percentile(values: Sequence[float], q: float) -> float:
    """nearest-rank の分位点（ceil(q·n)＝judge-perf の PERCENTILE_METHOD の実装）。空列では呼ばない。"""
    ordered = sorted(values)
    rank = max(1, math.ceil(q * len(ordered)))
    return ordered[min(rank, len(ordered)) - 1]
