#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""areka-P0-bindoption-exclusivity 実機サインオフ判定スクリプト（J1 / J2）

design.md §Testing Strategy → Real Machine Sign-off（emo2・R5）— 5.1-5.6 の
判定式 J1・J2 を、保全ログに対して再実行可能な形で機械判定する。

- J1（要件 5.2・共存痕跡の不在）
    保全ログ（debug 込み）を時刻順 1 パスで走査し、各まばたき発火
    `animation_id=140x`（時刻 t）について、t 以前の直近のまばたきカテゴリ
    適用痕跡（Changed / StateOnly / Unchanged のいずれも可）の id が
    発火 id と一致すること。適用痕跡ゼロでの発火は即赤。

- J2（要件 5.4・飽和パターンの不在）
    実走全体で |Changed 回数(まばたき) − Changed 回数(目)| <= 2 かつ
    最後のまばたき Changed が最後の目 Changed の 120 秒以内にあること。
    ※ この判定式は汎用則ではなく emo2 fixture 固有の較正値である。

- J3（要件 5.3）は目視であり本スクリプトの対象外（開発者サインオフ）。

使い方:
    python signoff-scan.py <ログファイルのパス>

終了コード:
    0 … J1・J2 とも PASS
    1 … いずれかが FAIL
    2 … 観測がゼロ等で判定不能（沈黙を PASS と誤判定しないための非ゼロ）
    3 … 引数不正・ファイル読み取り不能

標準ライブラリのみを使用する（外部パッケージ依存なし）。
"""

from __future__ import annotations

import re
import sys
from datetime import datetime, timedelta

# --------------------------------------------------------------------------
# emo2 fixture 固有の較正値（他ゴーストへ流用しないこと）
# --------------------------------------------------------------------------
# emo2 shell/master/descript.txt の bindgroup1400..1403 が「まばたき」カテゴリ。
# design.md の J1 は発火 id を `140x` と書いており、その範囲を採る。
BLINK_ID_MIN = 1400
BLINK_ID_MAX = 1409
# 適用痕跡のカテゴリ名（ログの `category=` フィールドに素の日本語で出る）。
BLINK_CATEGORY = "まばたき"
EYE_CATEGORY = "目"
# J2 の許容値（emo2 固有較正）。
J2_COUNT_DIFF_MAX = 2
J2_TAIL_GAP_MAX_SEC = 120.0

# --------------------------------------------------------------------------
# ログ行の実形（2026-08-11 実走 `RUST_LOG=info,areka_seriko=debug` で確認済み）
#
# 適用痕跡（Changed・info・実機 grep マーカー）:
#   2026-08-11T01:19:34.139004Z  INFO actor{actor=seriko}: areka_seriko::actor:
#     seriko: bind 適用 scope=0 category=まばたき part=通常 id=1400 on=true
#
# 適用痕跡（StateOnly / Unchanged・debug）:
#   2026-08-11T01:19:38.087602Z DEBUG actor{actor=seriko}: areka_seriko::actor:
#     seriko: bind 集合を更新（非表示/未知 scope または同値ゆえ発行なし）
#     scope=0 category=まばたき part=通常 id=1400 on=true
#
# まばたき発火（looper・info）:
#   2026-08-11T01:19:38.087688Z  INFO actor{actor=seriko}: areka_seriko::looper:
#     seriko: loop 抽選発火（再生開始・先頭コマから・要件 2.1/2.2）
#     scope="0" slot=Shell animation_id=1400 k=4
#
# 注: 適用痕跡の scope は素の数値（scope=0）、発火の scope は引用符付き
#     （scope="0"）で出る。両者を突き合わせるため正規化する。
# --------------------------------------------------------------------------

TS_RE = re.compile(r"^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?)Z")

MARK_CHANGED = "seriko: bind 適用 "
MARK_NOT_EMITTED = "seriko: bind 集合を更新"
MARK_FIRE = "seriko: loop 抽選発火"

TRACE_FIELDS_RE = re.compile(
    r"scope=(?P<scope>\d+)\s+category=(?P<cat>\S+)\s+part=(?P<part>.*?)\s+"
    r"id=(?P<id>\d+)\s+on=(?P<on>true|false)"
)
FIRE_FIELDS_RE = re.compile(
    r'scope="(?P<scope>\d+)"\s+slot=(?P<slot>\S+)\s+animation_id=(?P<id>\d+)'
)


def parse_ts(text: str):
    """先頭のタイムスタンプを datetime に変換する。失敗したら None。"""
    m = TS_RE.match(text)
    if not m:
        return None
    s = m.group(1)
    if "." in s:
        head, frac = s.split(".", 1)
        frac = (frac + "000000")[:6]  # ナノ秒精度を切り捨ててマイクロ秒へ
        s = head + "." + frac
        fmt = "%Y-%m-%dT%H:%M:%S.%f"
    else:
        fmt = "%Y-%m-%dT%H:%M:%S"
    try:
        return datetime.strptime(s, fmt)
    except ValueError:
        return None


class Trace:
    """まばたき/目カテゴリの適用痕跡 1 件。"""

    __slots__ = ("ts", "lineno", "scope", "cat", "part", "id", "on", "changed")

    def __init__(self, ts, lineno, scope, cat, part, id_, on, changed):
        self.ts = ts
        self.lineno = lineno
        self.scope = scope
        self.cat = cat
        self.part = part
        self.id = id_
        self.on = on
        self.changed = changed


class Fire:
    """looper の抽選発火 1 件。"""

    __slots__ = ("ts", "lineno", "scope", "slot", "id")

    def __init__(self, ts, lineno, scope, slot, id_):
        self.ts = ts
        self.lineno = lineno
        self.scope = scope
        self.slot = slot
        self.id = id_


class Scan:
    def __init__(self):
        self.events = []          # (ts, lineno, kind, obj) の時刻順 1 パス用
        self.traces = []          # 全カテゴリの適用痕跡
        self.fires = []           # 全 slot の抽選発火
        self.total_lines = 0
        self.unparsed_lines = 0   # 先頭タイムスタンプを持たない行（他コンポーネント等）


def is_blink_id(i: int) -> bool:
    return BLINK_ID_MIN <= i <= BLINK_ID_MAX


def scan_log(path: str) -> Scan:
    sc = Scan()
    with open(path, "r", encoding="utf-8", errors="replace") as fh:
        for lineno, raw in enumerate(fh, start=1):
            sc.total_lines += 1
            line = raw.rstrip("\n").rstrip("\r")
            ts = parse_ts(line)
            if ts is None:
                # 継続行・他コンポーネントの非標準行。落ちずに読み飛ばす。
                sc.unparsed_lines += 1
                continue
            if MARK_CHANGED in line or MARK_NOT_EMITTED in line:
                m = TRACE_FIELDS_RE.search(line)
                if not m:
                    sc.unparsed_lines += 1
                    continue
                t = Trace(
                    ts,
                    lineno,
                    m.group("scope"),
                    m.group("cat"),
                    m.group("part"),
                    int(m.group("id")),
                    m.group("on") == "true",
                    MARK_CHANGED in line,
                )
                sc.traces.append(t)
                sc.events.append((ts, lineno, "trace", t))
            elif MARK_FIRE in line:
                m = FIRE_FIELDS_RE.search(line)
                if not m:
                    sc.unparsed_lines += 1
                    continue
                f = Fire(
                    ts,
                    lineno,
                    m.group("scope"),
                    m.group("slot"),
                    int(m.group("id")),
                )
                sc.fires.append(f)
                sc.events.append((ts, lineno, "fire", f))
    # 時刻順 1 パス（同一時刻は出現順＝行番号順で安定化）。
    sc.events.sort(key=lambda e: (e[0], e[1]))
    return sc


# --------------------------------------------------------------------------
# J1
# --------------------------------------------------------------------------
def judge_j1(sc: Scan, out):
    blink_traces = [t for t in sc.traces if t.cat == BLINK_CATEGORY]
    blink_fires = [f for f in sc.fires if is_blink_id(f.id)]

    out.append("")
    out.append("== J1（要件 5.2・共存痕跡の不在）==")
    out.append(
        "  判定式: 各まばたき発火 animation_id=140x（時刻 t）について、"
        "t 以前の直近のまばたきカテゴリ適用痕跡（Changed/StateOnly/Unchanged 可）"
        "の id が発火 id と一致すること。痕跡ゼロでの発火は即赤。"
    )
    out.append(
        "  実測: まばたき適用痕跡 %d 件（Changed %d / 非発行 %d）、"
        "まばたき発火 %d 件"
        % (
            len(blink_traces),
            sum(1 for t in blink_traces if t.changed),
            sum(1 for t in blink_traces if not t.changed),
            len(blink_fires),
        )
    )
    by_fire_id = {}
    for f in blink_fires:
        by_fire_id[f.id] = by_fire_id.get(f.id, 0) + 1
    if by_fire_id:
        out.append(
            "  発火 id 内訳: "
            + " / ".join("%d×%d" % (k, v) for k, v in sorted(by_fire_id.items()))
        )
    by_trace_id = {}
    for t in blink_traces:
        key = (t.id, t.on)
        by_trace_id[key] = by_trace_id.get(key, 0) + 1
    if by_trace_id:
        out.append(
            "  痕跡 id 内訳: "
            + " / ".join(
                "id=%d on=%s ×%d" % (k[0], "true" if k[1] else "false", v)
                for k, v in sorted(by_trace_id.items())
            )
        )

    if not blink_fires:
        # 発火が 1 件も無ければ「共存が無い」ことを観測できていない。沈黙は PASS ではない。
        out.append("  → 判定不能: まばたき発火が 0 件（沈黙は PASS ではない）")
        return "INCONCLUSIVE"
    # 適用痕跡が 0 件でも INCONCLUSIVE にはしない。design.md の J1 は
    # 「適用痕跡ゼロでの発火は即赤」と定めており、走査がそのまま赤を出す。

    last_blink_trace = {}  # scope -> Trace
    violations = []
    examined = 0
    for ts, lineno, kind, obj in sc.events:
        if kind == "trace":
            if obj.cat == BLINK_CATEGORY:
                last_blink_trace[obj.scope] = obj
            continue
        # kind == "fire"
        if not is_blink_id(obj.id):
            continue
        examined += 1
        prev = last_blink_trace.get(obj.scope)
        if prev is None:
            violations.append(
                (
                    obj,
                    None,
                    "適用痕跡ゼロでの発火（追跡外 bind・emo2 静的既定集合に 14xx は無い）",
                )
            )
        elif prev.id != obj.id:
            violations.append(
                (
                    obj,
                    prev,
                    "直近のまばたき適用痕跡 id=%d と発火 id=%d が不一致（同一時間帯に複数まばたきが共存）"
                    % (prev.id, obj.id),
                )
            )

    out.append("  走査した発火: %d 件 / 違反: %d 件" % (examined, len(violations)))
    if violations:
        out.append("  違反の実測（先頭 10 件）:")
        for v in violations[:10]:
            f, prev, why = v
            if prev is None:
                out.append(
                    "    L%-6d %sZ scope=%s animation_id=%d  → %s"
                    % (f.lineno, f.ts.isoformat(timespec="microseconds"), f.scope, f.id, why)
                )
            else:
                out.append(
                    "    L%-6d %sZ scope=%s animation_id=%d  ← 直近痕跡 L%d %sZ "
                    "category=%s part=%s id=%d on=%s（%s）  → %s"
                    % (
                        f.lineno,
                        f.ts.isoformat(timespec="microseconds"),
                        f.scope,
                        f.id,
                        prev.lineno,
                        prev.ts.isoformat(timespec="microseconds"),
                        prev.cat,
                        prev.part,
                        prev.id,
                        "true" if prev.on else "false",
                        "Changed" if prev.changed else "非発行",
                        why,
                    )
                )
        if len(violations) > 10:
            out.append("    …ほか %d 件" % (len(violations) - 10))
        # 共存した id の要約（是正前の 1400/1402 並行発火のような形を明示する）。
        mismatched_ids = sorted({f.id for f, _p, _w in violations})
        out.append(
            "  不一致で発火した animation_id: "
            + ", ".join(str(i) for i in mismatched_ids)
        )
        out.append("  → J1: FAIL")
        return "FAIL"

    out.append("  → J1: PASS（全まばたき発火が直近の適用痕跡 id と一致）")
    return "PASS"


# --------------------------------------------------------------------------
# J2
# --------------------------------------------------------------------------
def judge_j2(sc: Scan, out):
    blink_changed = [t for t in sc.traces if t.cat == BLINK_CATEGORY and t.changed]
    eye_changed = [t for t in sc.traces if t.cat == EYE_CATEGORY and t.changed]

    out.append("")
    out.append("== J2（要件 5.4・飽和パターンの不在）==")
    out.append(
        "  判定式: |Changed 回数(まばたき) − Changed 回数(目)| <= %d かつ "
        "最後のまばたき Changed が最後の目 Changed の %.0f 秒以内。"
        % (J2_COUNT_DIFF_MAX, J2_TAIL_GAP_MAX_SEC)
    )
    out.append(
        "  ※ 汎用則ではなく emo2 fixture 固有の較正値（emo2 が毎回の表情変更で"
        "目とまばたきをペア送信することは 2026-08-11 実走で直接観測済み）。"
    )
    out.append(
        "  実測: Changed 回数 まばたき=%d / 目=%d（差 %d）"
        % (
            len(blink_changed),
            len(eye_changed),
            abs(len(blink_changed) - len(eye_changed)),
        )
    )

    if not blink_changed and not eye_changed:
        # 双方沈黙は差 0 で条件Aを素通りしてしまう。観測ゼロは PASS ではない。
        out.append("  → 判定不能: まばたき・目とも Changed が 0 件（観測ゼロは PASS ではない）")
        return "INCONCLUSIVE"
    if not blink_changed or not eye_changed:
        out.append(
            "  → J2: FAIL（%s の Changed が 0 件——片側が全区間で沈黙）"
            % (BLINK_CATEGORY if not blink_changed else EYE_CATEGORY)
        )
        return "FAIL"

    last_blink = blink_changed[-1]
    last_eye = eye_changed[-1]
    gap = abs((last_eye.ts - last_blink.ts).total_seconds())
    out.append(
        "  最後のまばたき Changed: L%d %sZ category=%s part=%s id=%d"
        % (
            last_blink.lineno,
            last_blink.ts.isoformat(timespec="microseconds"),
            last_blink.cat,
            last_blink.part,
            last_blink.id,
        )
    )
    out.append(
        "  最後の目     Changed: L%d %sZ category=%s part=%s id=%d"
        % (
            last_eye.lineno,
            last_eye.ts.isoformat(timespec="microseconds"),
            last_eye.cat,
            last_eye.part,
            last_eye.id,
        )
    )
    out.append("  末尾時刻差: %.3f 秒" % gap)

    ok_count = abs(len(blink_changed) - len(eye_changed)) <= J2_COUNT_DIFF_MAX
    ok_gap = gap <= J2_TAIL_GAP_MAX_SEC
    out.append(
        "  条件A（回数差 <= %d）: %s / 条件B（末尾 %.0f 秒以内）: %s"
        % (
            J2_COUNT_DIFF_MAX,
            "PASS" if ok_count else "FAIL",
            J2_TAIL_GAP_MAX_SEC,
            "PASS" if ok_gap else "FAIL",
        )
    )
    if ok_count and ok_gap:
        out.append("  → J2: PASS")
        return "PASS"
    out.append("  → J2: FAIL（まばたき適用が途中から恒久沈黙する飽和パターン）")
    return "FAIL"


def main(argv):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    if len(argv) != 2:
        sys.stdout.write("使い方: python signoff-scan.py <ログファイルのパス>\n")
        return 3
    path = argv[1]
    try:
        sc = scan_log(path)
    except OSError as exc:
        sys.stdout.write("ログを読めない: %s\n" % exc)
        return 3

    out = []
    out.append("areka-P0-bindoption-exclusivity 実機サインオフ判定（J1/J2）")
    out.append("ログ: %s" % path)
    out.append(
        "走査行数: %d（うち先頭タイムスタンプ無し等で読み飛ばした行 %d）"
        % (sc.total_lines, sc.unparsed_lines)
    )
    out.append(
        "抽出: 適用痕跡 %d 件（Changed %d / 非発行 %d）・抽選発火 %d 件"
        % (
            len(sc.traces),
            sum(1 for t in sc.traces if t.changed),
            sum(1 for t in sc.traces if not t.changed),
            len(sc.fires),
        )
    )
    if sc.events:
        out.append(
            "観測区間: %sZ 〜 %sZ"
            % (
                sc.events[0][0].isoformat(timespec="microseconds"),
                sc.events[-1][0].isoformat(timespec="microseconds"),
            )
        )
    if not sc.events:
        out.append("")
        out.append("→ 判定不能: seriko の適用痕跡・発火が 1 件も無い"
                   "（RUST_LOG=info,areka_seriko=debug で採取したか確認すること）")
        sys.stdout.write("\n".join(out) + "\n")
        sys.stdout.write("\n総合: INCONCLUSIVE\n")
        return 2

    r1 = judge_j1(sc, out)
    r2 = judge_j2(sc, out)

    out.append("")
    out.append("== 総合 ==")
    out.append("  J1=%s  J2=%s" % (r1, r2))
    out.append("  J3（ジト目切替後の表示更新）は開発者の目視サインオフ・本スクリプト対象外")
    sys.stdout.write("\n".join(out) + "\n")

    if "FAIL" in (r1, r2):
        return 1
    if "INCONCLUSIVE" in (r1, r2):
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
