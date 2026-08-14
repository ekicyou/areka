#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""`tools/perf/fixtures/` の既知ログを生成し直す道具（judge-perf.py `--selftest` の素材）。

このスクリプトは **判定には一切関わらない**。`--selftest` は生成済みのファイルを読むだけで、
このファイルを import も実行もしない。ここに置いてあるのは、fixture が手で細工した数字では
なく「決めた条件から機械的に組み立てたもの」であることを、後から誰でも確かめられるように
するためである。

    python generate.py            # fixtures/ を作り直す（差分が出なければ登録内容と一致）

作り直したあと `git status` が汚れなければ、コミットされている fixture は下の定義どおりで
ある。汚れたら、どちらかが変わっている（生成条件を変えたのか、fixture を手で触ったのか）。

各 fixture ディレクトリには次を置く:

    run.log        実走ログを模した既知ログ（実物と同じ行の形）
    cpu.csv        CPU 時系列（実物と同じヘッダ）
    run-meta.txt   実行条件（judge-perf.py は必要ログ種として要求する）
    case.txt       この fixture が何を固定するか＋期待終了コード（人も機械も読む唯一の台帳）

`case.txt` が `--selftest` の期待値表そのものである。期待値をスクリプト側の表に持たせると
fixture と表が別々に腐るので、置き場所を 1 つにしてある。

【この corpus が塞いでいる穴（task 2.3 の事後調査より）】
判定式⑴ が「系列ごとに除外し、target ごとにまとめて判定する」不整合を持っていたとき、
3 回のレビューが誰も気づかなかった。理由は corpus に **「同一 target の下に判定対象の系列が
2 本ある」形が 1 つも無かった** ことである。速い系列の裾に遅い系列が丸ごと吸収されて合格に
化ける経路を、赤ケース（H1b・H13・H14）と **同格の緑ケース（H11・H12）** の両方で押さえる。
緑だけでも赤だけでも足りない。
"""
from __future__ import annotations

import re
import shutil
from datetime import datetime, timedelta, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent

#: 全 fixture の時刻の起点。実時刻に依存させない（同じ入力からは必ず同じ出力）。
T0 = datetime(2026, 8, 14, 4, 0, 0, tzinfo=timezone.utc)

#: 起動過渡の適用が始まる位置（秒）。judge-perf.py はここを定常状態の基準時刻に採る。
WARMUP_FIRST_APPLY_SEC = 1.0

#: 定常状態の系列を置き始める位置（秒）。judge-perf.py の WARMUP_EXCLUDE_SEC=60 秒より後。
STEADY_START_SEC = 70.0


# =============================================================================
# 行の形（実物の逐語。tasks.md「2.2 で確定した実ログの形」と同じ）
# =============================================================================


def ts(at: datetime) -> str:
    return at.strftime("%Y-%m-%dT%H:%M:%S.%f") + "Z"


def perf_line(at: datetime, target: str, surface: str, hit: bool = True,
              allocs: tuple[int, int, int, int] = (0, 0, 0, 0),
              key_hash: str = "0x9f3a2b1c4d5e6f70") -> str:
    a, b, c, d = allocs
    return (
        f'{ts(at)} DEBUG actor{{actor="emo-text"}}: '
        f"areka_emo_present::presenter::timing: perf(apply_show): 段階別計時 "
        f"target_id={target} surface_id={surface} cache_hit={'true' if hit else 'false'} "
        f"t_cache_us=80 t_compose_us=0 t_resample_us=0 t_mask_us=0 t_upload_us=40 "
        f"t_total_us=120 alloc_compose_dst={a} alloc_resample_dst={b} alloc_xmap={c} "
        f"alloc_mask={d} key_hash={key_hash}"
    )


def show_line(at: datetime, target: str = "TargetId(0)", surface: str = "1000") -> str:
    return (
        f"{ts(at)} INFO areka_emo_present::presenter::show: "
        f"apply(ShowSurface): 表示・マスクを更新 target_id={target} surface_id={surface} "
        f"author_dpi=96 window_dpi=96 k=1 k_ratio=1/1 "
        f"native_w=478 native_h=684 scaled_w=478 scaled_h=684"
    )


def boot_line(at: datetime) -> str:
    return (
        f"{ts(at)} INFO areka::placement: 起動時 DPI 構成 primary_dpi=96 "
        f"shell_author_dpi=96 balloon_author_dpi=96 k_shell=1 k_balloon=1"
    )


# =============================================================================
# 色付け（ANSI エスケープ）— 実物の run.log はこの形で出る
# =============================================================================
#
# tracing-subscriber 0.3.23 は出力先が端末かどうかを見ず、環境変数 NO_COLOR が未設定なら
# 色を付ける（fmt_layer.rs:739-755）。手で起動した走行・NO_COLOR を置く前に採った走行の
# run.log にはエスケープが入っている。judge-perf.py はそれを落としてから解析するので、
# その経路を踏む fixture を 1 本必ず持つ。
#
# ここで足すのはエスケープだけであり、文字は 1 つも足さない・削らない。
# ゆえに「エスケープを落とした結果」は色なしの双子と **バイト等価** になる。
# 生成時にその等価性を検査する（下の build_colored）。

_ANSI_RE = re.compile(r"\x1b\[[0-9;?]*[ -/]*[@-~]")
_TS_RE = re.compile(r"^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{6}Z)")
_LEVEL_RE = re.compile(r"\b(DEBUG|INFO|WARN|ERROR)\b")
_FIELD_RE = re.compile(r"(?<![A-Za-z0-9_])([A-Za-z_][A-Za-z0-9_]*)=")

_DIM = "\x1b[2m"
_ITALIC = "\x1b[3m"
_RESET = "\x1b[0m"
_LEVEL_COLOR = {"DEBUG": "\x1b[34m", "INFO": "\x1b[32m", "WARN": "\x1b[33m",
                "ERROR": "\x1b[31m"}


def colorize(line: str) -> str:
    """色なしの 1 行へ ANSI エスケープを被せる（文字は増減させない）。"""
    line = _TS_RE.sub(lambda m: f"{_DIM}{m.group(1)}{_RESET}", line, count=1)
    line = _LEVEL_RE.sub(
        lambda m: f"{_LEVEL_COLOR[m.group(1)]}{m.group(1)}{_RESET}", line, count=1
    )
    line = _FIELD_RE.sub(
        lambda m: f"{_ITALIC}{m.group(1)}{_RESET}{_DIM}={_RESET}", line
    )
    return line


def strip_ansi(text: str) -> str:
    return _ANSI_RE.sub("", text)


# =============================================================================
# fixture の組み立て
# =============================================================================


class Fixture:
    def __init__(self, name: str):
        self.name = name
        self.dir = HERE / name
        self.rows: list[tuple[datetime, int, str]] = []
        self.cpu: list[tuple[datetime, float]] = []
        self.meta: list[str] = []
        self.case: list[str] = []
        self.colored = False
        self.drop_cpu_csv = False
        self.drop_meta = False

    # --- ログ ---
    def _add(self, at: datetime, text: str) -> None:
        self.rows.append((at, len(self.rows), text))

    def boot(self, warmup_applies: int = 5) -> "Fixture":
        """起動時 DPI ログ＋起動過渡の適用。最初の perf 行が定常状態の基準時刻になる。"""
        self._add(T0, boot_line(T0))
        for i in range(warmup_applies):
            at = T0 + timedelta(seconds=WARMUP_FIRST_APPLY_SEC + i)
            self._add(at, perf_line(at, "TargetId(0)", "1000"))
            self._add(at, show_line(at))
        return self

    def series(self, target: str, surface: str, period_ms: float, count: int,
               start_sec: float = STEADY_START_SEC, offset_ms: float = 0.0) -> "Fixture":
        """定常状態のコマ適用列を 1 本置く（間隔は count-1 本になる）。"""
        base = T0 + timedelta(seconds=start_sec, microseconds=round(offset_ms * 1000))
        for i in range(count):
            at = base + timedelta(microseconds=round(period_ms * i * 1000))
            self._add(at, perf_line(at, target, surface))
        return self

    def show_only(self, count: int = 3, start_sec: float = STEADY_START_SEC) -> "Fixture":
        for i in range(count):
            at = T0 + timedelta(seconds=start_sec + i)
            self._add(at, show_line(at))
        return self

    def drop_show_lines(self) -> "Fixture":
        self.rows = [r for r in self.rows if "表示・マスクを更新" not in r[2]]
        return self

    # --- CPU ---
    def cpu_series(self, start_sec: float = 62.0, count: int = 24,
                   step_sec: float = 15.0, value: float = 1.50) -> "Fixture":
        for i in range(count):
            at = T0 + timedelta(seconds=start_sec + step_sec * i)
            self.cpu.append((at, value))
        return self

    # --- 実行条件 ---
    def run_meta(self, build: str = "release", profile: str = "short") -> "Fixture":
        self.meta = [
            "# fixture の実行条件（実物は invoke-perf-run.ps1 が書き出す）",
            "[実行]",
            f"build = {build}",
            f"profile = {profile}",
            "[マシン]",
            "hostname = FIXTURE",
            "logical_processors = 8",
            "os_caption = Windows 11 Pro",
            "cpu_model = fixture（実機ではない）",
        ]
        return self

    # --- 台帳 ---
    def declare(self, mode: str, exit_code: int, title: str, notes: list[str],
                build: str | None = None, twin: str | None = None) -> "Fixture":
        lines = [
            "# judge-perf.py --selftest の期待値台帳（人も機械もこの 1 ファイルを読む）",
            f"title = {title}",
            f"mode = {mode}",
        ]
        if build is not None:
            lines.append(f"build = {build}")
        lines.append(f"exit = {exit_code}")
        if twin is not None:
            lines.append(f"twin = {twin}")
        lines.append("")
        lines += [f"note = {n}" for n in notes]
        self.case = lines
        return self

    # --- 書き出し ---
    def write(self) -> Path:
        if self.dir.exists():
            shutil.rmtree(self.dir)
        self.dir.mkdir(parents=True)

        self.rows.sort(key=lambda r: (r[0], r[1]))
        text = "".join(line + "\n" for _at, _i, line in self.rows)
        if self.colored:
            colored = "".join(colorize(line) + "\n" for _at, _i, line in self.rows)
            if strip_ansi(colored) != text:
                raise SystemExit(
                    f"{self.name}: 色付けが文字を変えている（色を落としても双子と一致しない）"
                )
            if "\x1b[" not in colored:
                raise SystemExit(f"{self.name}: 色付き fixture にエスケープが 1 つも入っていない")
            text = colored
        (self.dir / "run.log").write_text(text, encoding="utf-8", newline="\n")

        if not self.drop_cpu_csv:
            rows = ["timestamp,cpu_percent_1core"]
            rows += [f"{ts(at)},{value:.2f}" for at, value in sorted(self.cpu)]
            (self.dir / "cpu.csv").write_text(
                "\n".join(rows) + "\n", encoding="utf-8", newline="\n"
            )
        if not self.drop_meta:
            (self.dir / "run-meta.txt").write_text(
                "\n".join(self.meta) + "\n", encoding="utf-8", newline="\n"
            )
        (self.dir / "case.txt").write_text(
            "\n".join(self.case) + "\n", encoding="utf-8", newline="\n"
        )
        return self.dir


# =============================================================================
# 登録簿（この 1 箇所が corpus の全内容）
# =============================================================================
#
# 【寸法の決まり方】judge-perf.py の較正値がそのまま fixture の下限を決めている:
#   * WARMUP_EXCLUDE_SEC=60         … 定常状態は最初の perf 行の 60 秒後から
#   * VERDICT_MIN_STEADY_APPLIES=30 … 定常状態の適用が 30 回以上
#   * VERDICT_MIN_STEADY_SEC=60     … 定常状態の時間幅が 60 秒以上
#                                     （＝定常開始から最後の適用まで。合格を出す fixture は
#                                       どうしても 300 行ほどの列を 1 本持つことになる）
#   * FRAME_INTERVAL_MIN_SAMPLES=30 … 判定対象になる系列は間隔 30 本以上
#   * CONVERGENCE_MIN_SAMPLES_PER_WINDOW=3 x 窓 4 … CPU 採取点が 12 点以上

#: 172.0ms x 359 本で定常状態の時間幅 60 秒をまたぐ列の本数。
MAIN_COUNT = 300
MAIN_PERIOD_MS = 172.0

#: 合格の上限ちょうど（172.0 x 1.15 = 197.8ms）。丸め桁は 3。
BOUNDARY_MS = 197.800
#: 上限を 1/1000 ms だけ超える値。run.log の時刻はマイクロ秒分解能なので表現できる。
OVER_BOUNDARY_MS = 197.801


def case_H11() -> Fixture:
    return (
        Fixture("H11_healthy_two_series")
        .boot()
        .series("TargetId(0)", "1000", MAIN_PERIOD_MS, MAIN_COUNT)
        .series("TargetId(0)", "1001", 180.0, 31)
        .cpu_series()
        .run_meta()
        .declare(
            "verdict", 0, "健全な複数系列（合格の陽性対照）", build="release",
            notes=[
                "同じ対象（TargetId(0)）の下に判定対象の系列が 2 本あり、どちらも上限の内側",
                "にある走行。judge-perf.py はこれを合格（終了コード 0）にしなければならない。",
                "これは赤ケースと同格に重要である。判定式⑴ が「系列ごとに判定する」形に",
                "なっているかは、赤（H1b・H13）だけでは確かめられない——判定を壊して",
                "何もかも不合格にすれば赤は出るからである。合格が出ることまで見て初めて、",
                "判定が生きていると言える。",
                "この形の走行が corpus に 1 つも無かったために、実在した偽合格の欠陥を",
                "レビュー 3 回が見落とした（task 2.3 の事後調査）。",
                "内訳: surface_id=1000 が 172.0ms x 300 回・surface_id=1001 が 180.0ms x 31 回。",
            ],
        )
    )


def case_H12() -> Fixture:
    return (
        Fixture("H12_second_series_at_boundary")
        .boot()
        .series("TargetId(0)", "1000", MAIN_PERIOD_MS, MAIN_COUNT)
        .series("TargetId(0)", "1001", BOUNDARY_MS, 31)
        .cpu_series()
        .run_meta()
        .declare(
            "verdict", 0, "上限ちょうど（等号は合格側）", build="release",
            notes=[
                "2 本目の系列の間隔が上限ちょうど 197.800ms（＝172.0ms x 1.15）の走行。",
                "判定は「以下」なので合格（終了コード 0）である。",
                "H13 とはこの系列の間隔が 1/1000 ミリ秒だけ違うだけで、他は完全に同じである。",
                "2 つを並べて置いてあるのは、境界のどちら側に等号が付いているかを",
                "実際に走らせて確かめるためである。",
                "上限は 172.0 x 1.15 という掛け算で出るが、その計算結果は計算機の中では",
                "197.79999999999998 になる。丸めずに比べると、ちょうど 197.800ms の走行が",
                "「197.8 は 197.8 より大きい」という辻褄の合わない理由で不合格になる。",
                "この fixture はその丸め（小数点以下 3 桁）が効いていることを固定している。",
            ],
        )
    )


def case_H12_colored() -> Fixture:
    fixture = (
        Fixture("H12c_second_series_at_boundary_colored")
        .boot()
        .series("TargetId(0)", "1000", MAIN_PERIOD_MS, MAIN_COUNT)
        .series("TargetId(0)", "1001", BOUNDARY_MS, 31)
        .cpu_series()
        .run_meta()
    )
    fixture.declare(
        "verdict", 0, "色付きログ（色を落とす経路の較正）", build="release",
        twin="H12_second_series_at_boundary",
        notes=[
            "H12 と中身が同じで、run.log にだけ色（画面を彩る制御文字）が入っている走行。",
            "本物の run.log はこの形で出る——ログの出力先がファイルであっても、色を止める",
            "設定が無いかぎり色が付くためである。judge-perf.py は読み込み時に色を落として",
            "から解析する。その経路を踏む fixture がここにしかない。",
            "期待終了コードは H12 と同じ 0 である。色を落とす処理を外すと、行頭の時刻が",
            "読めなくなって判定不能（2）へ落ちるので、この 1 本が赤になる。",
            "さらに --selftest は、この fixture のレポートが色なしの双子（H12）の",
            "レポートと（入力パスの違いを除いて）一字一句同じであることまで確かめる。",
            "終了コードが同じでも中身がずれていたら、それも捕まえるためである。",
        ],
    )
    fixture.colored = True
    return fixture


def case_H13() -> Fixture:
    return (
        Fixture("H13_second_series_over_boundary")
        .boot()
        .series("TargetId(0)", "1000", MAIN_PERIOD_MS, MAIN_COUNT)
        .series("TargetId(0)", "1001", OVER_BOUNDARY_MS, 31)
        .cpu_series()
        .run_meta()
        .declare(
            "verdict", 1, "上限を 1/1000 ミリ秒だけ超過（不合格）", build="release",
            notes=[
                "H12 と完全に同じで、2 本目の系列の間隔だけが 197.801ms（上限 +0.001ms）。",
                "不合格（終了コード 1）でなければならない。",
                "H12 と対で置いてあり、2 つの終了コードが違うことが「境界がここにある」",
                "という主張そのものである。片方だけでは境界を測ったことにならない。",
            ],
        )
    )


def case_H1b() -> Fixture:
    return (
        Fixture("H1b_pool_dilution_300ms")
        .boot()
        .series("TargetId(0)", "1000", MAIN_PERIOD_MS, 621)
        .series("TargetId(0)", "1005", 300.0, 31)
        .cpu_series()
        .run_meta()
        .declare(
            "verdict", 1, "速い系列の陰に隠れた遅い系列（不合格）", build="release",
            notes=[
                "同じ対象の下に、速い系列（172.0ms x 621 回）と遅い系列（300.0ms x 31 回）が",
                "並んでいる走行。遅いほうは上限 197.8ms を大きく超えているので、",
                "不合格（終了コード 1）でなければならない。",
                "これは実在した欠陥の再現である。2 本を 1 つにまとめて上位 5% を見る形だと、",
                "遅い 30 本が速い 620 本の裾へ丸ごと吸い込まれて、まとめた値は 172ms のまま",
                "＝合格に見えた。判定を系列ごとに行っていれば捕まる。",
                "本数はその吸収が起きる下限に合わせてある（遅い側 30 本が全体の 5% に",
                "収まる本数）。速い側を減らすと吸収が起きなくなり、この fixture は",
                "欠陥を再現しなくなる。",
            ],
        )
    )


def case_H14() -> Fixture:
    return (
        Fixture("H14_dilution_plus_excluded")
        .boot()
        .series("TargetId(0)", "1000", MAIN_PERIOD_MS, 621)
        .series("TargetId(0)", "1005", 300.0, 31)
        .series("TargetId(0)", "1009", MAIN_PERIOD_MS, 10)
        .cpu_series()
        .run_meta()
        .declare(
            "verdict", 1, "不合格と判定不能が同時に立つとき（不合格が勝つ）", build="release",
            notes=[
                "H1b に、判定対象から外れる短い系列（間隔が 9 本しかない surface_id=1009）を",
                "足した走行。外れた系列があると判定式⑴ は本来「判定不能」へ倒れるが、",
                "確定的な不合格が別にあるので、総合は不合格（終了コード 1）でなければならない。",
                "不合格が判定不能に埋もれると、直すべき欠陥が「測れなかった」の山に隠れる。",
                "その優先順位を固定している。",
            ],
        )
    )


def case_P12() -> Fixture:
    fixture = Fixture("P12_same_key_two_animations").boot()
    fixture.series("TargetId(0)", "1000", MAIN_PERIOD_MS, MAIN_COUNT)
    # 同じ (target_id, surface_id) の上でもう 1 本のアニメが同時に走っている状態。
    # 適用は同じ鍵の列に混ざるため、混ざった列の隣り合う差はどちらのアニメの間隔でもなくなる。
    fixture.series("TargetId(0)", "1000", 700.0, 74, offset_ms=60.0)
    return (
        fixture.cpu_series()
        .run_meta()
        .declare(
            "verdict", 0, "同じ鍵の上で 2 本のアニメが互いを隠す（既知の穴・現状は合格）",
            build="release",
            notes=[
                "1 つの対象・1 つのサーフェスの上で 2 本のアニメが同時に走っている走行。",
                "適用の記録は同じ鍵の 1 本の列に混ざるので、隣り合う記録の差は",
                "「どちらかのアニメのコマ間隔」ではなくなり、実際より短く見える。",
                "遅いほうのアニメ（700ms 間隔）が速いほうの陰に隠れて、現状は合格",
                "（終了コード 0）になる。",
                "【これは⑴ の欠陥ではない】判定の窓をどこで区切るかがまだ決まっていない",
                "ことの帰結であり、窓の境界を決める作業（task 3.1 の第 1 段ベースライン）が",
                "担当する。ここでは「現状はこうなる」という事実を記録として固定しておき、",
                "窓の定義を変えたときにこの fixture の期待値も一緒に見直させる。",
                "期待値が 0 のままなのは意図であって、見落としではない。",
            ],
        )
    )


def case_E1() -> Fixture:
    return (
        Fixture("E1_steady_state_empty")
        .boot()
        .cpu_series(count=13)
        .run_meta()
        .declare(
            "verdict", 2, "定常状態に観測がない（判定不能・沈黙を合格にしない）",
            build="release",
            notes=[
                "起動直後の適用しかなく、定常状態（起動から 60 秒より後）には適用が",
                "1 回も無い走行。判定不能（終了コード 2）でなければならない。",
                "ここを合格にしてはいけない理由がこの corpus の背骨である——",
                "「定常状態での取りこぼし 0 件」「定常状態での新規確保 0 件」という 2 つの",
                "判定は、観測が 1 つも無ければ「0 件だから合格」と自動的に成立してしまう。",
                "何も測っていない走行と、測って問題が無かった走行を、終了コードで",
                "区別できなければ、この道具は沈黙を合格として売ることになる。",
            ],
        )
    )


def case_E2() -> Fixture:
    return (
        Fixture("E2_show_kind_missing")
        .boot()
        .series("TargetId(0)", "1000", MAIN_PERIOD_MS, 40)
        .drop_show_lines()
        .cpu_series(count=13)
        .run_meta()
        .declare(
            "verdict", 2, "必要なログの種類が欠けている（判定不能）", build="release",
            notes=[
                "表示が成立したことを示すログが 1 本も無い走行。",
                "判定不能（終了コード 2）でなければならず、途中まで集計した数字を",
                "出してはいけない。",
                "部分的な集計を出すと、読み手はそれを完全な結果だと思って受け取る。",
            ],
        )
    )


def case_E3() -> Fixture:
    fixture = (
        Fixture("E3_run_meta_missing")
        .boot()
        .series("TargetId(0)", "1000", MAIN_PERIOD_MS, 40)
        .cpu_series(count=13)
        .run_meta()
        .declare(
            "verdict", 2, "実行条件のファイルが無い（判定不能）", build="release",
            notes=[
                "run-meta.txt（どのマシンでどう走らせたかの記録）が無い走行。",
                "判定不能（終了コード 2）でなければならない。",
                "測定マシンの条件を書けない集計は、同じ条件で測り直せない＝比較にならない。",
                "judge-perf.py が run-meta.txt を必要なログの種類に数えているのは",
                "この理由による（design.md が挙げる 3 種より 1 つ多い。README に登記済み）。",
            ],
        )
    )
    fixture.drop_meta = True
    return fixture


def case_X1() -> Fixture:
    fixture = (
        Fixture("X1_cpu_csv_missing")
        .boot()
        .series("TargetId(0)", "1000", MAIN_PERIOD_MS, 40)
        .cpu_series(count=13)
        .run_meta()
        .declare(
            "verdict", 3, "指定されたファイルが無い（引数の誤り）", build="release",
            notes=[
                "cpu.csv が置かれていないディレクトリ。終了コードは 3（引数の誤り・",
                "読み取り不能）でなければならない。",
                "「測ったが判定できない（2）」と「そもそも指定が間違っている（3）」は",
                "直し方が違うので、終了コードで分けてある。",
            ],
        )
    )
    fixture.drop_cpu_csv = True
    return fixture


def case_X2() -> Fixture:
    return (
        Fixture("X2_verdict_without_build")
        .boot()
        .series("TargetId(0)", "1000", MAIN_PERIOD_MS, 40)
        .cpu_series(count=13)
        .run_meta()
        .declare(
            "verdict", 3, "合否判定なのにビルド種別の指定がない（引数の誤り）",
            notes=[
                "ファイルは 3 つとも揃っているのに、合否判定に必須の --build を渡していない",
                "呼び出し。終了コードは 3 でなければならない。",
                "どの判定式を当てるかがビルド種別で変わる（CPU の数値目標は release だけ）",
                "ため、推測で補ってはいけない。取り違えたまま黙って通ることを防いでいる。",
            ],
        )
    )


def case_B1() -> Fixture:
    return (
        Fixture("B1_baseline_ok")
        .boot()
        .series("TargetId(0)", "1000", MAIN_PERIOD_MS, 40)
        .cpu_series(count=13)
        .run_meta()
        .declare(
            "baseline", 0, "集計だけを行うモードは合否を出さない",
            notes=[
                "定常状態が薄く、合否判定なら判定不能になる走行を、集計だけのモードで",
                "読ませたもの。集計モードは合否を出さないので、警告を添えて終了コード 0 で",
                "終わるのが正しい。",
                "同じ入力でも、集計モード（0）と合否判定モード（2）で終了コードが違う。",
                "その違い自体をここで固定している。",
            ],
        )
    )


CASES = (
    case_H11, case_H12, case_H12_colored, case_H13, case_H1b, case_H14, case_P12,
    case_E1, case_E2, case_E3, case_X1, case_X2, case_B1,
)


def main() -> int:
    for factory in CASES:
        path = factory().write()
        size = sum(p.stat().st_size for p in path.iterdir())
        print(f"  {path.name:44s} {size:>9,d} バイト")
    print(f"{len(CASES)} 件の fixture を生成しました: {HERE}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
