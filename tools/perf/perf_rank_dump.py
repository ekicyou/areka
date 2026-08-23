#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""順位表（`perf-rank.py`）の段③——CPU サンプリング dump の一切と、共有する土台。

`perf-rank.py` は 1 ファイル 1,000 行以下の目安（要件 6.8）に収まらないので、
**段③まわりを丸ごとこちらへ寄せた**。ここに置くのは次の 3 種類である:

* dump そのもの（列名行での列の引き方・記号の綴りの揃え方・数え方・段③の順位表）
* 段③が要る土台のうち、他の段も使うもの（失敗の運び方・固定幅の描画・数の書式）
* 上の 2 つが共有する語彙（`areka.exe`・`Image!Function` など）

呼び出す側は `perf-rank.py` ただ 1 つで、こちらは実行しない（`__main__` を持たない）。
ファイル名にハイフンを使わないのは、`perf-rank.py` から `import` できるようにするため。
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path

# =============================================================================
# 失敗の運び方（4 段すべてが使う）
# =============================================================================

EXIT_OK, EXIT_FAIL, EXIT_BAD_INPUT, EXIT_MEASURE_FAILED = 0, 1, 3, 4


class RankError(Exception):
    """終了コードつきの失敗。文言は必ず添える（黙って落ちない）。"""

    def __init__(self, code: int, message: str, details: list[str] | None = None):
        super().__init__(message)
        self.code = code
        self.message = message
        self.details = details or []


def bad_input(message: str, details: list[str] | None = None) -> RankError:
    """引数不正・読取不能（exit 3）。"""
    return RankError(EXIT_BAD_INPUT, message, details)


def measure_failed(message: str, details: list[str] | None = None) -> RankError:
    """計測失敗（exit 4）——道具か採取が壊れていて、順位表を信じてはいけない状態。"""
    return RankError(EXIT_MEASURE_FAILED, message, details)


# =============================================================================
# 語彙
# =============================================================================

#: 値が取れなかったところに書く印（空欄を出さない）。
PLACEHOLDER = "-"

#: dump の中で読む 2 つのイベントと、そこから引く列名。列は必ず**列名行**で引く
#: （xperf の版で並びが変わるため・設計 C9 の Risks）。
DUMP_EVENTS = ("SampledProfile", "Stack")
DUMP_COLUMN_IMAGE = "Image!Function"
DUMP_COLUMN_TID = "ThreadID"
DUMP_COLUMN_TIMESTAMP = "TimeStamp"

#: 列名行に必ず在るべき既知の列（`SampledProfile`／`Stack` のどちらにも在る）。
#: 1 つでも別名になっていたら列を引き当てられないので、その場で止める（exit 4）。
DUMP_KNOWN_COLUMNS = (DUMP_COLUMN_TIMESTAMP, DUMP_COLUMN_TID, DUMP_COLUMN_IMAGE)

#: 段③の関門が数える対象モジュール（`invoke-cpu-sample.ps1` の `TARGET_MODULE` と同じ）。
TARGET_MODULE = "areka.exe"


# =============================================================================
# 数の書式
# =============================================================================


def fmt(value: float | int | None, decimals: int) -> str:
    """数を固定小数で。無いものは空欄ではなく印を出す。"""
    if value is None:
        return PLACEHOLDER
    return f"{value:.{decimals}f}"


def share_pct(part: float, whole: float) -> str:
    """割合（%）。分母が 0 なら印を出す——0 と「測れない」を混同しない。"""
    if whole <= 0:
        return PLACEHOLDER
    return f"{part / whole * 100.0:.2f}"


# =============================================================================
# 固定幅の描画
# =============================================================================


@dataclass
class Column:
    """表の 1 列。`width=0` は最終列の自由幅（右側を埋めない）。"""

    title: str
    width: int
    right: bool = False


def render_table(indent: int, columns: list[Column], rows: list[list[str]]) -> list[str]:
    """見出し行と本文を同じ幅で組む（幅を 1 箇所に持つので、ずれようがない）。"""

    def compose(cells: list[str]) -> str:
        parts: list[str] = []
        for i, (column, cell) in enumerate(zip(columns, cells)):
            last = i == len(columns) - 1
            if column.width <= 0 or (last and not column.right):
                parts.append(cell)
            elif column.right:
                parts.append(cell.rjust(column.width))
            else:
                parts.append(cell.ljust(column.width))
        return ((" " * indent) + "  ".join(parts)).rstrip()

    lines = [compose([c.title for c in columns])]
    lines.extend(compose(row) for row in rows)
    return lines


def scalar(key: str, value: str) -> str:
    """段の頭に並ぶ 1 値。鍵は左詰め・値は右詰めの固定幅。"""
    return f"  {key:<26}{value:>14}"


def note(text: str) -> str:
    return f"  注記: {text}"


def truncated(total: int, shown: int) -> list[str]:
    """上位 N で切ったときに、切った件数を必ず書く（黙って隠さない）。"""
    rest = total - shown
    return [f"    … 他 {rest} 件"] if rest > 0 else []


def tid_sort_key(tid: str) -> tuple[int, int, str]:
    """TID の並び順（数として読めるものは数の昇順・読めないものは後ろへ字面順）。"""
    return (0, int(tid), "") if tid.isdigit() else (1, 0, tid)


# =============================================================================
# 記号の綴りを 1 つに揃える
# =============================================================================

#: 旧式マングリングのハッシュ（`::h` に続く 16 桁前後の 16 進）。
_RUST_HASH_SUFFIX_RE = re.compile(r"::h[0-9a-f]{8,32}$")

#: `_ZN` … `E` に包まれた旧式マングリング。
_LEGACY_MANGLED_RE = re.compile(r"^_ZN(.+)E$")

#: 旧式マングリングが `$…$` で逃がす記号（rustc-demangle の表の必要分）。
_LEGACY_ESCAPES = (
    ("$LT$", "<"),
    ("$GT$", ">"),
    ("$LP$", "("),
    ("$RP$", ")"),
    ("$C$", ","),
    ("$RF$", "&"),
    ("$BP$", "*"),
    ("$u20$", " "),
    ("$u27$", "'"),
    ("$u5b$", "["),
    ("$u5d$", "]"),
    ("$u7b$", "{"),
    ("$u7d$", "}"),
)


def _decode_legacy_component(component: str) -> str:
    """旧式マングリングの 1 部品を人の読める形へ戻す。"""
    text = component
    if text.startswith("_$"):
        text = text[1:]
    for encoded, decoded in _LEGACY_ESCAPES:
        text = text.replace(encoded, decoded)
    return text.replace("..", "::")


def demangle(func: str) -> str:
    """関数名を 1 つの綴りへ揃える（旧式マングリングは展開・ハッシュは除去）。

    同じ関数が「素の名前」「`::h<16 進>` つき」「`_ZN…E` 包み」の 3 通りで出てくると、
    順位表が同じ関数を 3 行に分けて数えてしまい、上位が実際より軽く見える（設計 C9）。
    解けない綴りは**そのまま返す**——読めない名前を勝手に作り替えない。
    """
    m = _LEGACY_MANGLED_RE.match(func)
    if m:
        body = m.group(1)
        components: list[str] = []
        pos = 0
        while pos < len(body):
            digits = 0
            while pos + digits < len(body) and body[pos + digits].isdigit():
                digits += 1
            if digits == 0:
                components = []
                break
            length = int(body[pos: pos + digits])
            pos += digits
            if length <= 0 or pos + length > len(body):
                components = []
                break
            components.append(body[pos: pos + length])
            pos += length
        if components:
            if len(components) > 1 and re.fullmatch(r"h[0-9a-f]+", components[-1]):
                components.pop()
            return "::".join(_decode_legacy_component(c) for c in components)
    return _RUST_HASH_SUFFIX_RE.sub("", func)


# =============================================================================
# dump の解析
# =============================================================================


@dataclass
class DumpScan:
    """dump 1 本から読み取った中身（フレームは 2 種の行にまたがって数える）。"""

    #: `SampledProfile` 行＝1 つの CPU サンプル。(TimeStamp, ThreadID, module!function)
    samples: list[tuple[str, str, str]] = field(default_factory=list)
    #: `Stack` 行＝サンプルの呼出スタックの 1 段。(TimeStamp, ThreadID, module!function)
    stack_rows: list[tuple[str, str, str]] = field(default_factory=list)
    areka_frames: int = 0
    resolved_frames: int = 0
    unresolved_frames: int = 0
    tids: list[str] = field(default_factory=list)


def parse_dump(text: str, path: Path) -> DumpScan:
    """dump を読む。列は**列名行**で引き、既知列が無ければ exit 4（設計 C9 の Risks）。

    数え方は `invoke-cpu-sample.ps1` の `Measure-ArekaFrames` と同じにしてある——
    同じ断片を 2 つの道具が違う数に数えたら、どちらかが壊れている（fixture
    `sample_ok_counts` がそれを毎回確かめる）。とりわけ `ThreadStartImage!Function`
    列にも `areka.exe!` が入るので、素朴に字面を数えると倍に見える。
    """
    if not text.strip():
        raise measure_failed(f"dump が空です: {path}")

    headers: dict[str, dict[str, object]] = {}
    scan = DumpScan()
    seen_tids: list[str] = []

    for raw in text.replace("\r\n", "\n").split("\n"):
        line = raw.strip()
        if not line or line in ("BeginHeader", "EndHeader"):
            continue
        if line.startswith("//") or line.startswith("#") or "," not in line:
            continue

        cells = line.split(",")
        event = cells[0].strip()
        if event not in DUMP_EVENTS:
            continue
        trimmed = [cell.strip() for cell in cells]

        # 列名行の判別: 2 列目が TimeStamp という語そのもの（データ行はここが数値）。
        if len(trimmed) >= 2 and trimmed[1] == DUMP_COLUMN_TIMESTAMP:
            # 列名行が在っても、既知の列が別名になっていたら**データ行を 1 本も読む前に**
            # 止める（設計 C9 の Risks「既知列が無ければ exit 4 と文言」）。素通りさせると
            # 列を引き当てられないまま読み進め、「空の TID 1 本に全サンプル」のような
            # もっともらしい嘘の順位表が出る——しかも段によって出方が違う。
            missing_columns = [c for c in DUMP_KNOWN_COLUMNS if c not in trimmed]
            if missing_columns:
                raise measure_failed(
                    f"dump の列名行に既知の列がありません: {event} の "
                    f"{'／'.join(missing_columns)}",
                    [
                        f"dump={path}",
                        f"読めた列名: {', '.join(trimmed)}",
                        "xperf の版で列の並びが変わった可能性がある。列は必ず列名行で引くこと。",
                    ],
                )
            # `ThreadStartImage!Function` は別の綴りなので厳密一致では拾わないが、
            # 念のため後ろから探して最後の一致を採る（列が増えても取り違えない）。
            image_index = max(i for i, name in enumerate(trimmed) if name == DUMP_COLUMN_IMAGE)
            headers[event] = {
                "names": trimmed,
                "image": image_index,
                "tid": trimmed.index(DUMP_COLUMN_TID),
                "ts": trimmed.index(DUMP_COLUMN_TIMESTAMP),
            }
            continue

        head = headers.get(event)
        if head is None:
            continue  # 列名行の無いイベントは読まない（欠落は下でまとめて咎める）

        names, image_index = head["names"], head["image"]
        tid_index, ts_index = head["tid"], head["ts"]
        assert isinstance(names, list) and isinstance(image_index, int)
        assert isinstance(tid_index, int) and isinstance(ts_index, int)

        # Rust の記号は総称型の中にカンマを含み得るので、余った分は Image!Function へ寄せる。
        # 列が足りない行（途中で切れた dump）は先に弾く——そうしておけば、以降の列の
        # 引き当ては必ず範囲内に収まる（既知の列が在ることは上の関門で確かめてある）。
        extra = len(cells) - len(names)
        if extra < 0:
            continue

        timestamp = cells[ts_index].strip()
        tid = cells[tid_index].strip()
        if tid and tid not in seen_tids:
            seen_tids.append(tid)

        value = ",".join(cells[image_index: image_index + extra + 1]).strip()
        if not value:
            continue

        bang = value.find("!")
        if bang < 0:
            continue
        module = value[:bang].strip()
        func = value[bang + 1:].strip()
        if re.fullmatch(r"0x[0-9a-fA-F]+", func) or module in ("Unknown", "??"):
            scan.unresolved_frames += 1
        else:
            scan.resolved_frames += 1
        if module == TARGET_MODULE:
            scan.areka_frames += 1

        entry = (timestamp, tid, f"{module}!{demangle(func)}")
        if event == "SampledProfile":
            scan.samples.append(entry)
        else:
            scan.stack_rows.append(entry)

    missing = [event for event in DUMP_EVENTS if event not in headers]
    if missing:
        raise measure_failed(
            f"dump に既知の列名行がありません: {'／'.join(missing)}",
            [
                f"dump={path}",
                "xperf の版で列の並びが変わった可能性がある。列は必ず列名行で引くこと。",
            ],
        )
    scan.tids = sorted(seen_tids, key=tid_sort_key)
    return scan


# =============================================================================
# 段③ 関数
# =============================================================================


def rank_rows(counts: dict[str, int], whole: float, top: int, head: str) -> list[str]:
    """関数の順位表 1 枚（同数は名前の昇順＝Unicode 符号位置順で並べる）。"""
    items = sorted(counts.items(), key=lambda kv: (-kv[1], kv[0]))
    lines = render_table(
        4,
        [
            Column("rank", 4, True),
            Column(head, 6, True),
            Column("share_pct", 10, True),
            Column("module!function", 0),
        ],
        [
            [str(i), str(count), share_pct(float(count), whole), name]
            for i, (name, count) in enumerate(items[:top], start=1)
        ],
    )
    lines.extend(truncated(len(items), min(top, len(items))))
    return lines


def stage_function(dump: DumpScan | None, reason: str | None, top: int) -> list[str]:
    """dump から自己時間・包含時間・記号解決率（unit=samples）。"""
    if dump is None:
        return [
            f"[3] 関数 UNAVAILABLE reason={reason}",
            note(
                "段③（CPU サンプリング）の成果物が無いので関数別の帰属は出せない。"
                "昇格した PowerShell から回すと採れる。"
            ),
        ]

    samples_total = len(dump.samples)
    frames_total = dump.resolved_frames + dump.unresolved_frames

    # 段③が利用可で走ったのに 1 フレームも areka.exe へ解けないのは、記号が付いて
    # いないか列を取り違えているかであって、「areka が軽い」ではない（要件 2.11）。
    if dump.areka_frames == 0:
        raise measure_failed(
            f"段③は利用可なのに {TARGET_MODULE}! の解決フレームが 0 です（記号解決の失敗）。",
            [
                f"サンプル {samples_total} 本・スタック行 {len(dump.stack_rows)} 本・"
                f"解決済み {dump.resolved_frames}・未解決 {dump.unresolved_frames}",
                "CARGO_PROFILE_RELEASE_DEBUG=line-tables-only を付けてビルドしたか、"
                "Image!Function 列を列名行から引いているかを確かめること。",
            ],
        )

    self_counts: dict[str, int] = {}
    for _ts, _tid, name in dump.samples:
        self_counts[name] = self_counts.get(name, 0) + 1

    # 包含時間は「1 サンプルにその関数が現れたか」で数える（同じスタックに 2 度出ても 1）。
    # 呼出スタックが採れていないサンプルは、最上位フレームだけが現れたものとして扱う。
    stacks: dict[tuple[str, str], set[str]] = {}
    for ts, tid, name in dump.samples:
        stacks.setdefault((ts, tid), set()).add(name)
    for ts, tid, name in dump.stack_rows:
        if (ts, tid) in stacks:
            stacks[(ts, tid)].add(name)
    incl_counts: dict[str, int] = {}
    for names in stacks.values():
        for name in names:
            incl_counts[name] = incl_counts.get(name, 0) + 1

    lines = [
        "[3] 関数  unit=samples  出所=dump.txt（CPU サンプリング）",
        scalar("samples_total", str(samples_total)),
        scalar("stack_rows", str(len(dump.stack_rows))),
        scalar("frames_total", str(frames_total)),
        scalar("areka_resolved_frames", str(dump.areka_frames)),
        scalar("resolved_frames", str(dump.resolved_frames)),
        scalar("unresolved_frames", str(dump.unresolved_frames)),
        scalar(
            "resolution_rate_pct",
            share_pct(float(dump.resolved_frames), float(frames_total)),
        ),
        f"  自己時間（最上位フレーム・上位 {top}）",
    ]
    lines.extend(rank_rows(self_counts, float(samples_total), top, "self"))
    lines.append(f"  包含時間（スタックに含まれる・上位 {top}）")
    lines.extend(rank_rows(incl_counts, float(samples_total), top, "incl"))

    lines.append(f"  スレッド別 自己時間（上位 {top}）")
    per_tid: dict[str, dict[str, int]] = {}
    for _ts, tid, name in dump.samples:
        counts = per_tid.setdefault(tid, {})
        counts[name] = counts.get(name, 0) + 1
    for tid in sorted(per_tid, key=tid_sort_key):
        counts = per_tid[tid]
        total = sum(counts.values())
        lines.append(f"    tid={tid} samples={total}")
        lines.extend(rank_rows(counts, float(total), top, "self"))
    return lines
