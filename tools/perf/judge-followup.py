#!/usr/bin/env python3
"""見た目の追随チェックの判定（`run.log` ＋ `probe.log` → 検査ごとの合否）。

spec: areka-P0-draw-load-parity（要件 1.5 / 4.7・design.md「計測の道具（tools/perf/）
      → C13 追随チェック」）

使い方: ``python judge-followup.py <出力ディレクトリ>`` ／ ``python judge-followup.py
--selftest``。`<出力ディレクトリ>` には `invoke-followup-checks.ps1` が書いた `run.log`
（実走の標準出力）と `probe.log`（操作の記録）が入っている。本スクリプトはその 2 つだけを
読み、4 つの検査（`clickthrough` / `drag` / `dpi` / `balloon_follow`）へ
`PASS` / `FAIL` / `INCONCLUSIVE` を付け、総合判定を出す。

終了コードは `judge-perf.py` と同じ体系（0＝総合 PASS ／ 1＝1 つでも FAIL ／
2＝FAIL は無いが判定不能あり ／ 3＝引数不正・読取不能）。**判定不能は採用しない**
（安全側）——改善ループは総合 PASS のときだけ変更を採る。「操作を注入できなかったので
確かめられなかった」を「確かめたら大丈夫だった」と取り違えないための区別であり、
2 と 0 を混ぜてはならない。標準出力の末尾には必ず
``FOLLOWUP VERDICT overall=<…> clickthrough=… drag=… dpi=… balloon_follow=… code=<n>``
の 1 行を出す（背景実行でも会話へ届く形・要件 1.9）。

判定の材料は OS 側の実状態とログの両方が要る（design C13）。実状態だけだと本来の経路を
通らず偶然そう見えている場合を、ログだけだと経路は走ったがウィンドウに届いていない場合を、
それぞれ見分けられない。

「証跡が無い」は 2 種類あり、混ぜてはならない:

* **操作の記録（`probe.log`）が足りない** … 道具の側が動けなかった＝判定不能
  （`INCONCLUSIVE`・理由 `missing_probe_evidence`）。本番の実装を赤にしてはならない。
* **操作は成功したのに本番のログ（`run.log`）に反応が無い** … 本番の欠陥＝`FAIL`。
  ここを判定不能に倒すと、追随が壊れた変更が「確かめられなかった」の札で通ってしまう。

較正値・調整値（変更する場合はここだけを書き換える）: `SCRIPT_VERSION`（版）・
`CHECK_ALL`（検査の固定語彙・判定表の並び順）・`DRAG_DX_PX`／`DRAG_TOL_PX`（ドラッグの
距離と許容差）・`WRITE_POS_TOL_PX`（窓書込ログと OS の実位置の許容差）・
`BALLOON_REL_TOL_PX`（バルーンのキャラ窓相対位置の許容差）・`SELFTEST_*`（自己較正）。

`--selftest` のハーネスは兄弟モジュール `judge_followup_selftest.py` に在る（本ファイルを
1,000 行の目安に収めるため＝要件 6.8）。入口は変わらず `judge-followup.py --selftest` で、
ケースの置き場も `fixtures-loop/followup/<名前>/` のままである。
"""

from __future__ import annotations

import argparse
import contextlib
import re
import sys
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path

# 兄弟モジュール（`judge_followup_selftest.py`）は本ファイルと同じディレクトリに在る。
# 依存は常に「本体 → 兄弟」の一方向で、兄弟へは起動時に `SelftestEnv` で本体を渡す
# （本ファイルの名前にはハイフンがあり、兄弟からは import できない＝循環しない）。
sys.path.insert(0, str(Path(__file__).resolve().parent))

import judge_followup_selftest  # noqa: E402

# =============================================================================
# 較正値
# =============================================================================

SCRIPT_VERSION = "0.1.0"

#: 検査の固定語彙。判定表の並び順でもある。
CHECK_CLICKTHROUGH = "clickthrough"
CHECK_DRAG = "drag"
CHECK_DPI = "dpi"
CHECK_BALLOON_FOLLOW = "balloon_follow"
CHECK_ALL = (CHECK_CLICKTHROUGH, CHECK_DRAG, CHECK_DPI, CHECK_BALLOON_FOLLOW)

#: 許容差（px）。DRAG_DX_PX は `invoke-followup-checks.ps1` の同名の較正値と対。
#: DRAG_TOL_PX / BALLOON_REL_TOL_PX は design C13 の「± 2px」、WRITE_POS_TOL_PX は
#: 窓書込ログの新位置と OS の実位置を突き合わせるときの許容差。
DRAG_DX_PX = 80
DRAG_TOL_PX = 2
WRITE_POS_TOL_PX = 2
BALLOON_REL_TOL_PX = 2

#: 判定語と終了コード。
PASS = "PASS"
FAIL = "FAIL"
INCONCLUSIVE = "INCONCLUSIVE"
EXIT_OK = 0
EXIT_FAIL = 1
EXIT_INCONCLUSIVE = 2
EXIT_BAD_INPUT = 3

#: 入出力のファイル名（`invoke-followup-checks.ps1` が書く 2 つと、判定の書き出し先）。
RUN_LOG_NAME = "run.log"
PROBE_LOG_NAME = "probe.log"
VERDICT_NAME = "followup-verdict.txt"

#: 自己較正。「赤」を再現するケースが 1 件も無い自己較正は較正になっていない（D7）。
SELFTEST_FIXTURES_DIRNAME = "fixtures-loop"
SELFTEST_SUBDIR = "followup"
SELFTEST_CASE_FILENAME = "case.txt"
SELFTEST_EXPECTED_STDOUT = "expected_stdout.txt"
SELFTEST_REQUIRED_RED_EXITS = (EXIT_FAIL, EXIT_INCONCLUSIVE)

# =============================================================================
# 本番ログの語彙（Rust 側の単一定義元を写したもの）
# =============================================================================
#
# 下は本番コードの字面である。変更するときは必ず出所を確かめること。
#   MSG_TOGGLE       crates/wintf/src/ecs/clickthrough/controller.rs:212
#   MSG_SHOW         crates/areka-emo-present/src/presenter/show.rs:462
#   TRANSITION_TAG   crates/wintf/src/ecs/window/transition_diag.rs:57
#   MSG_WINDOWPOSCHANGED / MSG_DPICHANGED
#                    crates/wintf/src/ecs/window/transition_diag.rs:104,102
#   DESIRED_*        crates/wintf/src/ecs/clickthrough/registry.rs:18-23（DesiredState）
#   WIN_KIND_*       crates/areka/src/placement/diag.rs:338-343（WindowKind::as_str）

MSG_TOGGLE = "clickthrough: ex-style トグル適用"
MSG_SHOW = "apply(ShowSurface): 表示・マスクを更新"
TRANSITION_TAG = "[transition]"
DESIRED_TRANSPARENT = "Transparent"
DESIRED_OPAQUE = "Opaque"
MSG_WINDOWPOSCHANGED = "WM_WINDOWPOSCHANGED"
MSG_DPICHANGED = "WM_DPICHANGED"
WIN_KIND_CHAR = "char"
WIN_KIND_BALLOON = "balloon"

# =============================================================================
# 行の解析（`judge-perf.py::parse_fields` と同じ規則）
# =============================================================================

#: 順に、ANSI エスケープ（CSI シーケンス・読み込み時に落とす）／行頭の時刻
#: （`tracing-subscriber` 既定の SystemTime タイマは RFC3339・UTC・小数 6 桁）／
#: `probe.log` のフィールド値としての時刻／`名前=` の出現位置（値は次の `名前=` の
#: 直前までとする＝空白を含んでよい）。
_ANSI_RE = re.compile(r"\x1b\[[0-9;?]*[ -/]*[@-~]")
_TIMESTAMP_RE = re.compile(r"^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?)Z\s")
_BARE_TIMESTAMP_RE = re.compile(r"^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?)Z$")
_FIELD_KEY_RE = re.compile(r"(?:^|\s)([A-Za-z_][A-Za-z0-9_]*)=")


class JudgeError(Exception):
    """終了コードを伴う失敗。"""

    def __init__(self, code: int, message: str, details: list[str] | None = None) -> None:
        super().__init__(message)
        self.code = code
        self.message = message
        self.details = details or []


def bad_input(message: str, details: list[str] | None = None) -> JudgeError:
    return JudgeError(EXIT_BAD_INPUT, message, details)


def strip_ansi(text: str) -> str:
    return _ANSI_RE.sub("", text)


def _to_datetime(raw: str) -> datetime | None:
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
    m = _TIMESTAMP_RE.match(line)
    return _to_datetime(m.group(1)) if m else None


def parse_stamp_value(value: str | None) -> datetime | None:
    """`probe.log` のフィールド値としての時刻を返す。読めなければ None。"""
    if not value:
        return None
    m = _BARE_TIMESTAMP_RE.match(value.strip())
    return _to_datetime(m.group(1)) if m else None


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
    return None if idx < 0 else line[idx + len(message):]


def read_text_lines(path: Path, what: str) -> list[str]:
    """テキストを行に割って返す。ANSI を落とし、行末の CR も落とす。"""
    try:
        raw = path.read_bytes()
    except OSError as exc:
        raise bad_input(f"{what} を読めません: {path}", [str(exc)]) from exc
    text = raw.decode("utf-8", errors="replace")
    if text.startswith("﻿"):
        text = text[1:]
    return [strip_ansi(line.rstrip("\r")) for line in text.split("\n")]


def to_int(value: str | None) -> int | None:
    """10 進・16 進のどちらでも読む。読めなければ None（値を捏造しない）。"""
    if value is None:
        return None
    text = value.strip()
    try:
        return int(text, 16) if text.lower().startswith("0x") else int(text)
    except ValueError:
        return None


def to_float(value: str | None) -> float | None:
    if value is None:
        return None
    try:
        return float(value.strip())
    except ValueError:
        return None


def norm_hwnd(value: str | None) -> str | None:
    """ハンドルの字面を `0xABCD`（大文字・0x 付き）へ正規化する。"""
    number = to_int(value)
    return None if number is None else f"0x{number:X}"


# --- 入力の器 ----------------------------------------------------------------

@dataclass
class ProbeRecord:
    """`probe.log` の 1 行。"""

    check: str
    step: str
    t: datetime | None
    fields: dict[str, str]

    def get(self, name: str, default: str | None = None) -> str | None:
        return self.fields.get(name, default)


@dataclass
class RunRecord:
    """`run.log` の 1 行（時刻と本文）。"""

    t: datetime | None
    line: str


@dataclass
class Rect:
    """窓 1 つの矩形（物理 px・スクリーン座標）。"""

    hwnd: str
    win_kind: str
    x: int
    y: int
    w: int
    h: int


@dataclass
class Verdict:
    """検査 1 つの判定。"""

    check: str
    verdict: str
    reason: str
    details: list[str] = field(default_factory=list)


@dataclass
class Observations:
    """判定に使う観測をまとめた器。"""

    probe: list[ProbeRecord]
    run: list[RunRecord]
    required: list[str] = field(default_factory=list)
    char_hwnd: str | None = None
    balloon_hwnd: str | None = None
    scope: str | None = None
    injection: str | None = None
    injection_detail: str = "-"

    def probe_rows(self, check: str, step: str) -> list[ProbeRecord]:
        return [r for r in self.probe if r.check == check and r.step == step]

    def probe_row(self, check: str, step: str) -> ProbeRecord | None:
        rows = self.probe_rows(check, step)
        return rows[-1] if rows else None

    def run_between(self, begin: datetime, end: datetime) -> list[RunRecord]:
        return [r for r in self.run if r.t is not None and begin <= r.t <= end]

    def run_before(self, begin: datetime) -> list[RunRecord]:
        return [r for r in self.run if r.t is not None and r.t < begin]


# =============================================================================
# 読み込み
# =============================================================================


def load_probe(path: Path) -> list[ProbeRecord]:
    records: list[ProbeRecord] = []
    for line in read_text_lines(path, "操作の記録（probe.log）"):
        idx = line.find("probe: ")
        if idx < 0:
            continue
        fields = parse_fields(line[idx + len("probe: "):])
        check = fields.get("check")
        step = fields.get("step")
        if not check or not step:
            continue
        records.append(ProbeRecord(check, step, parse_stamp_value(fields.get("t")), fields))
    return records


def load_run(path: Path) -> list[RunRecord]:
    records: list[RunRecord] = []
    last: datetime | None = None
    for line in read_text_lines(path, "実走のログ（run.log）"):
        if not line.strip():
            continue
        stamp = parse_timestamp(line)
        if stamp is not None:
            last = stamp
        # 複数行メッセージの続きは直前の行の時刻を引き継ぐ（時刻無しで捨てない）。
        records.append(RunRecord(stamp or last, line))
    return records


def load_observations(directory: Path) -> Observations:
    if not directory.is_dir():
        raise bad_input(
            f"出力ディレクトリがありません: {directory}",
            ["invoke-followup-checks.ps1 の -OutDir に渡したディレクトリを指定してください。"],
        )
    probe_path = directory / PROBE_LOG_NAME
    run_path = directory / RUN_LOG_NAME
    for path, what in ((probe_path, PROBE_LOG_NAME), (run_path, RUN_LOG_NAME)):
        if not path.is_file():
            raise bad_input(
                f"{what} がありません: {path}",
                [
                    "判定は run.log（本番のログ）と probe.log（操作の記録）の両方を要ります。"
                    "片方だけでは「操作したのに反応が無い」と「操作できなかった」を区別できません。",
                ],
            )

    obs = Observations(probe=load_probe(probe_path), run=load_run(run_path))
    if not obs.probe:
        raise bad_input(
            f"{probe_path} に probe: 行が 1 行もありません",
            ["操作を 1 つも記録していない probe.log からは、何も確かめられません。"],
        )

    session = obs.probe_row("session", "begin")
    if session is None or not session.get("required"):
        raise bad_input(
            f"{probe_path} に check=session step=begin の required= がありません",
            ["どの検査を必須とするかは probe.log の session 行が唯一の定義元です。"],
        )
    obs.required = [name.strip() for name in session.get("required", "").split(",") if name.strip()]
    unknown = [name for name in obs.required if name not in CHECK_ALL]
    if unknown:
        raise bad_input(
            f"required= に未知の検査名があります: {'・'.join(unknown)}",
            [f"使えるのは {'・'.join(CHECK_ALL)} です。"],
        )
    if not obs.required:
        raise bad_input(
            "required= が空です",
            ["必須検査が 0 件の判定は、何も確かめずに PASS を返します。"],
        )

    target = obs.probe_row("windows", "target")
    if target is not None:
        obs.char_hwnd = norm_hwnd(target.get("char_hwnd"))
        obs.balloon_hwnd = norm_hwnd(target.get("balloon_hwnd"))
        obs.scope = target.get("scope")

    env = obs.probe_row("env", "injection")
    if env is not None:
        obs.injection = env.get("injection")
        obs.injection_detail = (
            f"SetCursorPos={env.get('setcursorpos_ret')} "
            f"cursor_moved={env.get('cursor_moved')} "
            f"SendInput={env.get('sendinput_sent')} "
            f"lasterr={env.get('sendinput_lasterr')}"
        )
    return obs


# --- 観測の取り出し（検査ごとの時間帯・種別ごとの行・矩形）--------------------

def check_window(obs: Observations, check: str) -> tuple[datetime, datetime] | None:
    """検査の観測窓（`step=window` の `begin_t`〜`end_t`）。読めなければ None。"""
    row = obs.probe_row(check, "window")
    if row is None:
        return None
    begin = parse_stamp_value(row.get("begin_t"))
    end = parse_stamp_value(row.get("end_t"))
    return None if begin is None or end is None else (begin, end)


def check_status(obs: Observations, check: str) -> ProbeRecord | None:
    return obs.probe_row(check, "status")


def transition_rows(records: list[RunRecord], kind: str) -> list[dict[str, str]]:
    """`[transition] … kind=<kind>` 行のフィールド辞書を集める。"""
    rows: list[dict[str, str]] = []
    for record in records:
        tail = split_after_message(record.line, TRANSITION_TAG)
        if tail is None:
            continue
        fields = parse_fields(tail)
        if fields.get("kind") == kind:
            rows.append(fields)
    return rows


def show_k_values(records: list[RunRecord]) -> list[float]:
    """表示成立点の `k=` を出現順に集める。"""
    values: list[float] = []
    for record in records:
        tail = split_after_message(record.line, MSG_SHOW)
        if tail is None:
            continue
        value = to_float(parse_fields(tail).get("k"))
        if value is not None:
            values.append(value)
    return values


def toggle_directions(records: list[RunRecord]) -> dict[str, int]:
    """クリック透過のトグル適用ログを向き別に数える。"""
    counts = {DESIRED_TRANSPARENT: 0, DESIRED_OPAQUE: 0}
    for record in records:
        tail = split_after_message(record.line, MSG_TOGGLE)
        if tail is None:
            continue
        desired = parse_fields(tail).get("desired")
        if desired in counts:
            counts[desired] += 1
    return counts


def rects_by_phase(obs: Observations, check: str, phase: str) -> dict[str, Rect]:
    """`step=rect phase=<phase>` の矩形を `win_kind` で引ける形にする。"""
    found: dict[str, Rect] = {}
    for row in obs.probe_rows(check, "rect"):
        if row.get("phase") != phase:
            continue
        hwnd = norm_hwnd(row.get("hwnd"))
        win_kind = row.get("win_kind")
        values = [to_int(row.get(name)) for name in ("x", "y", "w", "h")]
        if hwnd is None or win_kind is None or any(v is None for v in values):
            continue
        found[win_kind] = Rect(hwnd, win_kind, *values)  # type: ignore[arg-type]
    return found


def relative_offset(rects: dict[str, Rect]) -> tuple[int, int] | None:
    """バルーンのキャラ窓相対位置（バルーン左上 − キャラ左上）。"""
    char = rects.get(WIN_KIND_CHAR)
    balloon = rects.get(WIN_KIND_BALLOON)
    if char is None or balloon is None:
        return None
    return (balloon.x - char.x, balloon.y - char.y)


# =============================================================================
# 検査ごとの判定
# =============================================================================


def _unavailable(check: str, status: ProbeRecord | None) -> Verdict | None:
    """probe が「やれなかった」と言っている検査を判定不能にする（共通の前段）。"""
    if status is None:
        return Verdict(
            check,
            INCONCLUSIVE,
            "missing_probe_evidence",
            [f"    ! probe.log に check={check} step=status の行がありません（操作が記録されていない）。"],
        )
    state = status.get("status")
    if state == "done":
        return None
    reason = status.get("reason") or "unspecified"
    if state in ("unavailable", "skipped"):
        return Verdict(
            check,
            INCONCLUSIVE,
            reason,
            [f"    probe.log の status={state}: 操作そのものを実施できていません（本番の合否ではない）。"],
        )
    return Verdict(
        check,
        INCONCLUSIVE,
        "missing_probe_evidence",
        [f"    ! probe.log の status={state!r} は未知の値です（done / unavailable / skipped のいずれか）。"],
    )


def judge_clickthrough(obs: Observations) -> Verdict:
    check = CHECK_CLICKTHROUGH
    early = _unavailable(check, check_status(obs, check))
    if early is not None:
        return early

    window = check_window(obs, check)
    if window is None:
        return Verdict(check, INCONCLUSIVE, "missing_probe_evidence",
                       ["    ! probe.log に check=clickthrough step=window（観測窓）がありません。"])

    details: list[str] = []
    mismatches: list[str] = []
    inconclusive: list[str] = []
    expectations = {"transparent": True, "opaque": False}
    for point, expected in expectations.items():
        row = next((r for r in obs.probe_rows(check, "read") if r.get("point") == point), None)
        if row is None:
            inconclusive.append(f"{point} 点の ex-style 読み戻しが probe.log にありません")
            continue
        observed_text = (row.get("transparent") or "").lower()
        expected_text = (row.get("expected") or "").lower()
        if observed_text not in ("true", "false") or expected_text not in ("true", "false"):
            inconclusive.append(f"{point} 点の transparent= / expected= を読めません")
            continue
        observed = observed_text == "true"
        declared = expected_text == "true"
        agree = observed == declared
        details.append(
            f"    {point:<11s} hwnd={row.get('hwnd')} ex={row.get('ex')} "
            f"WS_EX_TRANSPARENT={'あり' if observed else 'なし'} "
            f"期待={'あり' if declared else 'なし'} → {'一致' if agree else '食い違い'}"
        )
        if declared != expected:
            # probe が期待そのものを取り違えている（道具の壊れ）。
            inconclusive.append(f"{point} 点の expected= が {declared} で、判定側の期待 {expected} と違います")
            continue
        # probe 自身の result= と判定側の再計算が食い違うなら道具が壊れている。
        declared_result = row.get("result")
        if declared_result in ("match", "mismatch") and (declared_result == "match") != agree:
            inconclusive.append(f"{point} 点の probe の result={declared_result} が読み戻しと矛盾します")
            continue
        if not agree:
            mismatches.append(f"{point} 点で WS_EX_TRANSPARENT が期待と逆")

    counts = toggle_directions(obs.run_between(*window))
    details.append(
        f"    トグル記録  「{MSG_TOGGLE}」 {DESIRED_TRANSPARENT}={counts[DESIRED_TRANSPARENT]} 行 / "
        f"{DESIRED_OPAQUE}={counts[DESIRED_OPAQUE]} 行（観測窓 {window[0].isoformat()} 〜 {window[1].isoformat()}）"
    )
    missing_log = [name for name, count in counts.items() if count == 0]

    if inconclusive:
        return Verdict(check, INCONCLUSIVE, "missing_probe_evidence", details + [f"    ! {m}" for m in inconclusive])
    if mismatches or missing_log:
        reason = "exstyle_mismatch" if mismatches else "toggle_log_missing"
        notes = [f"    ! {m}" for m in mismatches]
        notes += [f"    ! 観測窓に desired={name} のトグル記録が 1 行もありません" for name in missing_log]
        return Verdict(check, FAIL, reason, details + notes)
    return Verdict(check, PASS, "-", details)


def judge_drag(obs: Observations) -> Verdict:
    check = CHECK_DRAG
    early = _unavailable(check, check_status(obs, check))
    if early is not None:
        return early

    window = check_window(obs, check)
    before = rects_by_phase(obs, check, "before")
    after = rects_by_phase(obs, check, "after")
    if window is None or not before or not after:
        return Verdict(check, INCONCLUSIVE, "missing_probe_evidence",
                       ["    ! probe.log の check=drag に観測窓または前後の矩形がありません。"])

    details: list[str] = []
    failures: list[str] = []
    records = obs.run_between(*window)
    writes = transition_rows(records, "write")
    msgs = transition_rows(records, "msg")

    for win_kind in (WIN_KIND_CHAR, WIN_KIND_BALLOON):
        old, new = before.get(win_kind), after.get(win_kind)
        if old is None or new is None:
            return Verdict(check, INCONCLUSIVE, "missing_probe_evidence",
                           details + [f"    ! {win_kind} の前後どちらかの矩形がありません"])
        dx, dy = new.x - old.x, new.y - old.y
        ok = abs(dx - DRAG_DX_PX) <= DRAG_TOL_PX and abs(dy) <= DRAG_TOL_PX
        details.append(
            f"    {win_kind:<8s} OS の実位置 ({old.x},{old.y}) → ({new.x},{new.y}) "
            f"Δ=({dx:+d},{dy:+d}) 期待 ({DRAG_DX_PX:+d},+0) ± {DRAG_TOL_PX} → {'一致' if ok else '食い違い'}"
        )
        if not ok:
            failures.append(f"{win_kind} の移動量が {DRAG_DX_PX}±{DRAG_TOL_PX}px から外れています")

        logged = [
            row for row in writes
            if norm_hwnd(row.get("hwnd")) == new.hwnd
            and to_int(row.get("ax")) is not None
            and abs((to_int(row.get("ax")) or 0) - new.x) <= WRITE_POS_TOL_PX
        ]
        details.append(
            f"    {win_kind:<8s} 窓書込ログ kind=write hwnd={new.hwnd} "
            f"ax={new.x} ± {WRITE_POS_TOL_PX} … {len(logged)} 行"
        )
        if not logged:
            failures.append(f"{win_kind} の新位置を書いた kind=write 行が観測窓にありません")

    char_hwnd = (after.get(WIN_KIND_CHAR) or before[WIN_KIND_CHAR]).hwnd
    poschanged = [
        row for row in msgs
        if row.get("msg") == MSG_WINDOWPOSCHANGED and norm_hwnd(row.get("hwnd")) == char_hwnd
    ]
    details.append(f"    受理記録    kind=msg msg={MSG_WINDOWPOSCHANGED} hwnd={char_hwnd} … {len(poschanged)} 行")
    if not poschanged:
        failures.append(f"キャラ窓の {MSG_WINDOWPOSCHANGED} 受理が観測窓にありません")

    if failures:
        return Verdict(check, FAIL, "drag_not_followed", details + [f"    ! {m}" for m in failures])
    return Verdict(check, PASS, "-", details)


def judge_dpi(obs: Observations) -> Verdict:
    check = CHECK_DPI
    early = _unavailable(check, check_status(obs, check))
    if early is not None:
        return early

    monitors = obs.probe_rows(check, "monitors")
    dpis = sorted({value for value in (to_int(row.get("dpi")) for row in monitors) if value is not None})
    window = check_window(obs, check)
    if window is None:
        return Verdict(check, INCONCLUSIVE, "missing_probe_evidence",
                       ["    ! probe.log に check=dpi step=window（観測窓）がありません。"])

    details = [f"    モニタ表    {len(monitors)} 面・DPI の種類 {dpis}"]
    if len(dpis) < 2:
        return Verdict(check, INCONCLUSIVE, "single_dpi",
                       details + ["    ! DPI の異なるモニタが 2 面以上ないと DPI 遷移を起こせません"])

    failures: list[str] = []
    records = obs.run_between(*window)
    char_hwnd = obs.char_hwnd
    dpi_msgs = [
        row for row in transition_rows(records, "msg")
        if row.get("msg") == MSG_DPICHANGED
        and (char_hwnd is None or norm_hwnd(row.get("hwnd")) == char_hwnd)
    ]
    details.append(f"    受理記録    kind=msg msg={MSG_DPICHANGED} hwnd={char_hwnd} … {len(dpi_msgs)} 行")
    if not dpi_msgs:
        failures.append(f"キャラ窓の {MSG_DPICHANGED} 受理が観測窓にありません")

    before_k = show_k_values(obs.run_before(window[0]))
    inside_k = show_k_values(records)
    baseline = before_k[-1] if before_k else None
    changed = [value for value in inside_k if baseline is None or value != baseline]
    details.append(
        f"    表示成立点  観測窓の直前 k={baseline} → 観測窓 k={inside_k}"
        f"（k の変わった成立点 {len(changed)} 件）"
    )
    if baseline is None:
        failures.append("観測窓より前に表示成立点が 1 件も無く、k の変化を比べられません")
    elif not changed:
        failures.append("観測窓の表示成立点で k= が 1 度も変わっていません")

    moves = {row.get("phase"): row for row in obs.probe_rows(check, "move")}
    for phase in ("out", "back"):
        row = moves.get(phase)
        if row is None:
            failures.append(f"モニタ移動（phase={phase}）が probe.log にありません")
            continue
        details.append(
            f"    モニタ移動  phase={phase} ({row.get('x')},{row.get('y')}) "
            f"{row.get('from_dpi')}→{row.get('to_dpi')} result={row.get('result')}"
        )
        if row.get("result") != "ok":
            failures.append(f"モニタ移動（phase={phase}）が成功していません")

    if failures:
        return Verdict(check, FAIL, "dpi_not_followed", details + [f"    ! {m}" for m in failures])
    return Verdict(check, PASS, "-", details)


def judge_balloon_follow(obs: Observations, others: dict[str, Verdict]) -> Verdict:
    check = CHECK_BALLOON_FOLLOW
    early = _unavailable(check, check_status(obs, check))
    if early is not None:
        return early

    # 相対位置は drag / dpi が実際に動かした前後で測る。動かせていなければ測る対象が無い。
    # 見るのは **必須の**（`required=` に在る）動かし手だけ。外された検査は道具が操作そのものを
    # 行わない（invoke-followup-checks.ps1 は `-Checks` に無い検査を回さない）ので、一律に
    # 見ると「dpi を外したのに balloon_follow が dpi 待ちで判定不能」となり、必須集合を狭めた
    # 意味が消える——DPI の違うモニタが 1 面しか無い機械で 1 周も採れなくなる。
    sources = [name for name in (CHECK_DRAG, CHECK_DPI) if name in obs.required]
    if not sources:
        return Verdict(
            check,
            INCONCLUSIVE,
            "no_required_mover",
            ["    required= に drag も dpi も無いため、前後で比べる操作そのものがありません。"],
        )
    depends = [name for name in sources if others.get(name) and others[name].verdict != PASS]
    if depends:
        return Verdict(
            check,
            INCONCLUSIVE,
            "depends_on_drag_dpi",
            [f"    {'・'.join(depends)} が PASS でないため、前後で比べる操作そのものが成立していません。"],
        )

    details: list[str] = []
    failures: list[str] = []
    labels = {CHECK_DRAG: "ドラッグ", CHECK_DPI: "DPI 往復"}
    for source in sources:
        label = labels[source]
        before = relative_offset(rects_by_phase(obs, source, "before"))
        after = relative_offset(rects_by_phase(obs, source, "after"))
        if before is None or after is None:
            return Verdict(check, INCONCLUSIVE, "missing_probe_evidence",
                           details + [f"    ! {label}の前後にキャラ窓とバルーン窓の矩形が揃っていません"])
        dx, dy = after[0] - before[0], after[1] - before[1]
        ok = abs(dx) <= BALLOON_REL_TOL_PX and abs(dy) <= BALLOON_REL_TOL_PX
        details.append(
            f"    {label:<8s} キャラ窓相対 {before} → {after} Δ=({dx:+d},{dy:+d}) "
            f"許容 ± {BALLOON_REL_TOL_PX} → {'一致' if ok else '食い違い'}"
        )
        if not ok:
            failures.append(f"{label}の前後でバルーンのキャラ窓相対位置が {BALLOON_REL_TOL_PX}px を超えて動きました")

    if failures:
        return Verdict(check, FAIL, "balloon_drifted", details + [f"    ! {m}" for m in failures])
    return Verdict(check, PASS, "-", details)


def judge_all(obs: Observations) -> dict[str, Verdict]:
    verdicts: dict[str, Verdict] = {}
    verdicts[CHECK_CLICKTHROUGH] = judge_clickthrough(obs)
    verdicts[CHECK_DRAG] = judge_drag(obs)
    verdicts[CHECK_DPI] = judge_dpi(obs)
    verdicts[CHECK_BALLOON_FOLLOW] = judge_balloon_follow(obs, verdicts)
    return verdicts


def overall_of(verdicts: dict[str, Verdict], required: list[str]) -> tuple[str, int]:
    """総合判定と終了コード。全 PASS のときだけ PASS（判定不能は採用しない）。"""
    states = [verdicts[name].verdict for name in required]
    if FAIL in states:
        return FAIL, EXIT_FAIL
    if INCONCLUSIVE in states:
        return INCONCLUSIVE, EXIT_INCONCLUSIVE
    return PASS, EXIT_OK


# =============================================================================
# レポート
# =============================================================================


class Report:
    """標準出力と判定ファイルへ同じ本文を出す器。"""

    def __init__(self) -> None:
        self.lines: list[str] = []

    def line(self, text: str = "") -> None:
        self.lines.append(text)

    def sub(self, title: str) -> None:
        self.line()
        self.line(f"-- {title} --")

    def text(self) -> str:
        return "\n".join(self.lines) + "\n"


def render(obs: Observations, verdicts: dict[str, Verdict], overall: str, code: int) -> Report:
    report = Report()
    report.line("=" * 78)
    report.line("judge-followup.py 見た目の追随チェックの判定（要件 1.5 / 4.7・design C13）")
    report.line("=" * 78)
    report.line(f"  スクリプト版 : {SCRIPT_VERSION}")
    report.line(f"  必須検査     : {'・'.join(obs.required)}")
    report.line("  終了コード   : 0=総合 PASS / 1=FAIL あり / 2=FAIL 無しで判定不能あり / 3=読取不能")

    report.sub("観測の前提")
    report.line(f"  対象キャラ窓   : hwnd={obs.char_hwnd} scope={obs.scope}")
    report.line(f"  対象バルーン窓 : hwnd={obs.balloon_hwnd}")
    report.line(f"  操作の注入     : {obs.injection}（{obs.injection_detail}）")
    report.line(f"  probe.log 行数 : {len(obs.probe)}  /  run.log 行数 : {len(obs.run)}")

    report.sub("検査ごとの判定")
    for name in CHECK_ALL:
        verdict = verdicts[name]
        mark = "" if name in obs.required else "  ※ 必須ではない（required= に無い）"
        report.line(f"check={verdict.check} verdict={verdict.verdict} reason={verdict.reason}{mark}")
        for detail in verdict.details:
            report.line(detail)

    report.sub("総合")
    if overall == PASS:
        report.line("  必須検査がすべて PASS。この変更は見た目の追随を劣化させていません。")
    elif overall == FAIL:
        report.line("  追随が壊れています。採用してはいけません（本番の欠陥）。")
    else:
        report.line("  判定不能があります。**判定不能は採用しない**（安全側・design C13）。")
        report.line("  確かめられなかった検査は、操作を注入できる対話セッションで実施してください。")

    summary = " ".join(f"{name}={verdicts[name].verdict}" for name in CHECK_ALL)
    report.line()
    report.line(f"FOLLOWUP VERDICT overall={overall} {summary} code={code}")
    return report


# =============================================================================
# 自己較正（--selftest）
# =============================================================================
#
# 緑は道具が壊れていても出る。ゆえに合格を再現するケースと同格に、不合格・判定不能を
# 再現するケースを置き、そのどちらも実際に走って期待どおりになったことを出力に出す。
# 経路は実運用と同じ `main(argv)`（判定関数を直に叩く別経路は作らない）。
#
# ハーネスそのものは兄弟モジュール judge_followup_selftest.py に在る（本ファイルを
# 1,000 行の目安に収めるため＝要件 6.8）。本体は道具立て（SelftestEnv）を渡すだけで、
# 依存は常に「本体 → 兄弟」の一方向である。入口は変わらず --selftest。


def run_selftest() -> int:
    return judge_followup_selftest.run_selftest(
        judge_followup_selftest.SelftestEnv(
            fixtures_root=Path(__file__).resolve().parent / SELFTEST_FIXTURES_DIRNAME / SELFTEST_SUBDIR,
            invoke=main,
            bad_input=bad_input,
            required_red_exits=SELFTEST_REQUIRED_RED_EXITS,
            case_filename=SELFTEST_CASE_FILENAME,
            expected_stdout_filename=SELFTEST_EXPECTED_STDOUT,
            ok_exit=EXIT_OK,
            fail_exit=EXIT_FAIL,
            bad_input_exit=EXIT_BAD_INPUT,
        )
    )


# =============================================================================
# 入口
# =============================================================================


class Parser(argparse.ArgumentParser):
    """引数不正の終了コードを 3 に揃える（argparse の既定 2 は使わない）。"""

    def error(self, message: str):  # noqa: D102
        self.print_usage(sys.stderr)
        print(f"judge-followup.py: 引数が不正です: {message}", file=sys.stderr)
        raise SystemExit(EXIT_BAD_INPUT)


def build_parser() -> Parser:
    parser = Parser(
        prog="judge-followup.py",
        description="見た目の追随チェック（クリック透過・ドラッグ・DPI・バルーン追従）を判定する。",
        epilog="終了コード: 0=総合 PASS / 1=FAIL あり / 2=FAIL 無しで判定不能あり / 3=引数不正・読取不能",
    )
    parser.add_argument(
        "directory",
        nargs="?",
        help="invoke-followup-checks.ps1 の出力ディレクトリ（run.log と probe.log がある）",
    )
    parser.add_argument(
        "--selftest",
        action="store_true",
        help="fixtures-loop/followup/ の既知ケースで判定そのものを較正する",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    # Windows の既定コンソール（cp932）では日本語のレポートが例外で落ちる。判定結果が
    # 出せないと会話へ届かないため、入口で UTF-8 へ寄せる（自己較正の StringIO は
    # reconfigure を持たないので AttributeError を握り潰す）。
    for stream in (sys.stdout, sys.stderr):
        with contextlib.suppress(AttributeError, OSError):
            stream.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[union-attr]

    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        if args.selftest:
            return run_selftest()
        if not args.directory:
            raise bad_input(
                "出力ディレクトリを指定してください",
                ["使い方: python judge-followup.py <出力ディレクトリ>"],
            )
        directory = Path(args.directory)
        obs = load_observations(directory)
        verdicts = judge_all(obs)
        overall, code = overall_of(verdicts, obs.required)
        report = render(obs, verdicts, overall, code)
        text = report.text()
        sys.stdout.write(text)
        try:
            (directory / VERDICT_NAME).write_text(text, encoding="utf-8")
        except OSError as exc:
            print(f"judge-followup.py: {VERDICT_NAME} を書けません: {exc}", file=sys.stderr)
        return code
    except JudgeError as exc:
        print(f"judge-followup.py: {exc.message}", file=sys.stderr)
        for detail in exc.details:
            print(f"  {detail}", file=sys.stderr)
        return exc.code


if __name__ == "__main__":
    raise SystemExit(main())
