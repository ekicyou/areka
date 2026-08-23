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
2 本ある」形が 1 つも無かった** ことである。赤ケースと **同格の緑ケース** を置く、という
規律はここから来ている。緑だけでも赤だけでも足りない。

【task 3.2 の裁定を受けた作り直し】判定式⑴ の窓が「発火起点」（窓 C）になり、判定対象系列が
`TargetId(0) / surface_id=1000` の 1 本へ明示指定された。これに伴い corpus の形が変わる:

  * 走行には **seriko の発火ログ**（scope="0"）が要る。発火が無いと窓 C は空になり、
    どの fixture も判定不能へ落ちる。ゆえに定常の系列は `cycle_series()` で組む
  * 「同一 target の下に判定対象の系列が 2 本」は **本番の較正値では作れない**
    （明示指定が 1 本だけなので、2 本目は必ず意図的な対象外になる）。代わりに、
    明示指定がもたらす **新しい穴** を赤で押さえる:
      - 明示指定した系列が定常状態の perf 行を 1 本も出していない（H20）
      - 明示指定した系列が判定窓に間隔を 1 本も残していない（H21）
    どちらも「他の系列が上限内だから合格」に化ける経路であり、判定不能でなければならない
  * 明示指定から漏れた系列は、どれだけ遅くても合格を止めない（H1b）。これは欠陥ではなく
    all-or-nothing の指定がもたらす **既知の代償** であり、記録として固定しておく

【remediation 1 で足したもの（P13）】「同じ鍵に 2 本のアニメが混ざる」形は、混ざり方の位相で
倒れる向きが変わる。P12 は **偽の不合格** の側だけを固定していて、**偽の合格** の側には
陽性対照が 1 つも無かった。これは task 2.3 が三度踏んだ形（陽性対照の無い経路が静かに壊れる）
そのものなので、P13 で合格側の位相を固定した。窓の条件 4
（FRAME_INTERVAL_IDLE_MAX_ACTIVE）を外すと P13 は終了コード 0 を返す。

【remediation 2 で足したもの（P14）】その P13 も P12 も、2 本のアニメは **どちらも
scope="0"**（キャラ）である。つまり corpus には「別の scope のアニメが重なっている」形が
1 つも無く、条件 4 が scope をまたいで数えているという誤りを 3 回のレビューが素通りした。
同じ穴の 4 度目である。P14 はその陽性対照で、上限超過の区間だけを別 scope のアニメが覆う。
数える範囲が全 scope なら合格（0）、判定対象の scope だけなら不合格（1）に分かれるので、
この fixture 1 本で 2 つの数え方を区別できる。
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

#: 発火から次の発火までの間隔（ミリ秒）。サイクルの外側の空白がここに入る。
#: 実機の emo2 はまばたきを数秒に 1 回しか起こさないが、fixture は行数を抑えるため 1 秒。
#: 「発火から最後のコマまで」より十分大きければ窓 C の条件は変わらない。
CYCLE_PERIOD_MS = 1000.0


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


def catchup_line(at: datetime, target: str) -> str:
    """進行境界スキップ（catch-up）の info!（判定式⑵の入力・§9 の系統別の素材）。

    実物の逐語（`loop_ticker`・45 秒の実走ログで確認）:
        2026-08-22T19:18:27.526356Z  INFO actor{actor=loop-ticker}: areka_ghost::ticker:         loop ticker catch-up: skipped multiple boundaries, firing once target="loop_ticker"

    `dispatcher` / `kanade` は `ticker.rs:203-206,223-226` の同じ形で、違うのは
    (a) 文言が短いほう（"ticker catch-up: …"）(b) スパンが `actor{actor=ticker}`
    （`ticker.rs:179` の `spawn_actor("ticker")`）(c) `target` の値、の 3 点だけである。

    【重要】`target` は tracing の **フィールド**（`target = "…"`）であって、
    メタデータの target 指示子（`target: "…"`）ではない。ゆえに行末に
    `target="dispatcher"` の形で出る（値は引用符付き）。3 系統を分けるのはこの値であり、
    文言では dispatcher と kanade を区別できない（どちらも同じ短いほうの文言である）。
    """
    if target == "loop_ticker":
        span = "loop-ticker"
        message = "loop ticker catch-up: skipped multiple boundaries, firing once"
    else:
        span = "ticker"
        message = "ticker catch-up: skipped multiple boundaries, firing once"
    return (
        f"{ts(at)}  INFO actor{{actor={span}}}: areka_ghost::ticker: "
        f'{message} target="{target}"'
    )


#: `[tick] kind=window` の相別フィールド（13 本・design.md C15 の順）。
TICK_PHASE_FIELDS = (
    "input_us", "update_us", "prelayout_us", "layout_us", "postlayout_us",
    "uisetup_us", "graphicssetup_us", "draw_us", "prerendersurface_us",
    "rendersurface_us", "composition_us", "commitcomposition_us", "framefinalize_us",
)

#: 相別の値（合計が wall_us と一致する必要はない——判定に使わない参考量である）。
TICK_PHASE_VALUES = (
    33445, 15447, 122169, 23679, 24679, 41068, 18288, 40755, 6711, 1581, 4315, 569, 874920,
)


def tick_line(at: datetime, frame: int, t_ms: int = 1000, ticks: int = 120,
              skipped: int = 0, heartbeat: int = 0, wall_us: int = 200000,
              max_us: int = 6888, ui_cpu_us: int = 150000) -> str:
    """フレーム駆動の 1 秒窓（`[tick] kind=window`・design.md C15）。任意種である。

    実物の逐語（45 秒の実走ログで確認）:
        2026-08-22T19:18:18.470014Z DEBUG actor{actor=emo-text}: wintf::tick:         [tick] kind=window frame=38 t_ms=1006 ticks=38 skipped=0 heartbeat=0         wall_us=1210594 max_us=680880 ui_cpu_us=1046875 input_us=33445 … framefinalize_us=874920

    行の時刻は **窓が閉じた時刻** である（窓が覆う区間は `[時刻 − t_ms, 時刻]`）。
    """
    phases = " ".join(
        f"{name}={value}" for name, value in zip(TICK_PHASE_FIELDS, TICK_PHASE_VALUES)
    )
    return (
        f"{ts(at)} DEBUG actor{{actor=emo-text}}: wintf::tick: "
        f"[tick] kind=window frame={frame} t_ms={t_ms} ticks={ticks} skipped={skipped} "
        f"heartbeat={heartbeat} wall_us={wall_us} max_us={max_us} ui_cpu_us={ui_cpu_us} "
        f"{phases}"
    )


def thread_report_lines(at: datetime, snap: int, t_s: int) -> list[str]:
    """スレッド別・プロセス全体の CPU 報告（design.md C14）。任意種である。"""
    head = f"{ts(at)}  INFO areka::perf: "
    return [
        head + (
            f"perf(process): プロセス CPU snap={snap} t_s={t_s} wall_ms={t_s * 1000 + 1} "
            f"cpu_us=4546875 kernel_us=781250 user_us=3765625 threads=14"
        ),
        head + (
            f"perf(thread): スレッド別 CPU snap={snap} t_s={t_s} tid=19224 name=main "
            f"role=ui cpu_us=3343750 kernel_us=312500 user_us=3031250"
        ),
        head + (
            f"perf(thread): スレッド別 CPU snap={snap} t_s={t_s} tid=19225 name=- "
            f"role=unregistered_rest cpu_us=203125 kernel_us=46875 user_us=156250"
        ),
    ]


def talk_line(at: datetime, event: str) -> str:
    """発話（talk）の開始・終了 info!（`areka-kanade/src/schedule/steady.rs:763,851`）。

    `target: "kanade"` は指示子なので target 位置に出る。`event` はフィールドなので
    メッセージの後ろに引用符付きで出る。発話中の CPU の頂（要件 5.4）はこの 2 本で挟まれた
    区間の採取点から採る。
    """
    if event == "steady_talk":
        return (
            f"{ts(at)}  INFO kanade: 応答にスクリプト——再生起動 "
            f'event="{event}" talk_id=7 origin="mouse"'
        )
    return f'{ts(at)}  INFO kanade: talk 完了——定常運転へ復帰 event="{event}"'


def boot_line(at: datetime) -> str:
    return (
        f"{ts(at)} INFO areka::placement: 起動時 DPI 構成 primary_dpi=96 "
        f"shell_author_dpi=96 balloon_author_dpi=96 k_shell=1 k_balloon=1"
    )


def fire_line(at: datetime, scope: str = "0", animation_id: str = "1400") -> str:
    """seriko の「loop 抽選発火」info!（判定式⑴の窓 C の起点）。

    実物の逐語（`budget-base-short-release/run.log:140` で確認）:
        …Z  INFO actor{actor=seriko}: areka_seriko::looper: seriko: loop 抽選発火
        （再生開始・先頭コマから・要件 2.1/2.2） scope="0" slot=Shell animation_id=1400 k=4
    scope は引用符付きで出る（judge-perf.py の `unquote_field` が落とす）。
    """
    return (
        f"{ts(at)} INFO actor{{actor=seriko}}: areka_seriko::looper: "
        f"seriko: loop 抽選発火（再生開始・先頭コマから・要件 2.1/2.2） "
        f'scope="{scope}" slot=Shell animation_id={animation_id} k=4'
    )


def stop_line(at: datetime, scope: str = "1", animation_id: str = "0") -> str:
    """seriko の「loop 停止」info!（judge-perf.py の J_SERIKO_STOP_MESSAGES の 1 つ）。

    実物の逐語（`budget-base-short-release/run.log` のケロの停止行で確認）:
        …Z  INFO actor{actor=seriko}: areka_seriko::looper: seriko: loop 停止
        （負 surface でベース復帰・要件 4.3） scope="1" slot=Shell animation_id=0
    窓 C の条件 4 は発火と停止の収支から活性本数を推定するので、
    「別のアニメがこの区間だけ走っていた」形を組むには停止行が要る。
    """
    return (
        f"{ts(at)} INFO actor{{actor=seriko}}: areka_seriko::looper: "
        f"seriko: loop 停止（負 surface でベース復帰・要件 4.3） "
        f'scope="{scope}" slot=Shell animation_id={animation_id}'
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
        """定常状態のコマ適用列を 1 本置く（間隔は count-1 本になる）。

        発火ログを伴わないので、この列は **判定式⑴の窓 C には 1 本も残らない**。
        判定対象の系列を組むときは `cycle_series()` を使うこと。
        """
        base = T0 + timedelta(seconds=start_sec, microseconds=round(offset_ms * 1000))
        for i in range(count):
            at = base + timedelta(microseconds=round(period_ms * i * 1000))
            self._add(at, perf_line(at, target, surface))
        return self

    def cycle_series(self, target: str, surface: str, waits_ms: tuple[float, ...],
                     cycles: int, scope: str = "0", animation_id: str = "1400",
                     cycle_period_ms: float = CYCLE_PERIOD_MS,
                     start_sec: float = STEADY_START_SEC) -> "Fixture":
        """発火 1 回＋そのサイクルのコマ適用を `cycles` 回並べる（判定式⑴の窓 C の素材）。

        1 サイクルは「時刻 f に発火 → f から `waits_ms` を累積した各時刻にコマ適用」。
        `waits_ms=(150.0, 22.0)` なら適用は f+0 / f+150 / f+172 の 3 回になり、
        窓 C に残る間隔は **150ms と 22ms の 2 本**（f+172 から次のサイクルへの間隔は
        区間に次の発火を含むので落ちる）。emo2 の `animation1400` と同じ形である。

        `waits_ms=(X,)` なら適用は f+0 / f+X の 2 回で、窓 C に残る間隔は **X ms 1 本**。
        境界を 1/1000 ミリ秒の精度で置きたいときはこちらを使う。

        【窓の条件を満たすための前提】`cycle_period_ms` は「発火から最後のコマまでの長さ」
        より十分大きく取ること。重なると次の発火が区間に入り、間隔が落ちる。
        """
        for c in range(cycles):
            fired = T0 + timedelta(
                seconds=start_sec, microseconds=round(cycle_period_ms * c * 1000)
            )
            self._add(fired, fire_line(fired, scope=scope, animation_id=animation_id))
            elapsed = 0.0
            self._add(fired, perf_line(fired, target, surface))
            for wait in waits_ms:
                elapsed += wait
                at = fired + timedelta(microseconds=round(elapsed * 1000))
                self._add(at, perf_line(at, target, surface))
        return self

    def other_scope_span(self, at_sec: float, duration_ms: float,
                         scope: str = "1", animation_id: str = "0") -> "Fixture":
        """別 scope のアニメが `at_sec` から `duration_ms` のあいだ走っていた記録を置く。

        発火と停止の 2 行だけを置く（コマ適用の記録は置かない——別 scope のアニメは
        別の対象へコマを出すので、判定対象の系列の列には混ざらないからである）。
        窓 C の条件 4 が「判定対象系列の scope だけを数える」ことを確かめる素材である。
        """
        start = T0 + timedelta(seconds=at_sec)
        stop = start + timedelta(microseconds=round(duration_ms * 1000))
        self._add(start, fire_line(start, scope=scope, animation_id=animation_id))
        self._add(stop, stop_line(stop, scope=scope, animation_id=animation_id))
        return self

    def catchups(self, events: tuple[tuple[float, str], ...]) -> "Fixture":
        """catch-up を `(発生秒, 系統)` の並びで置く（系統は target フィールドの値）。"""
        for at_sec, target in events:
            at = T0 + timedelta(seconds=at_sec)
            self._add(at, catchup_line(at, target))
        return self

    def tick_windows(self, start_sec: float, count: int, wall_us: int = 200000,
                     heavy_secs: tuple[float, ...] = (), heavy_wall_us: int = 900000,
                     ticks: int = 120, skipped: int = 0) -> "Fixture":
        """`[tick] kind=window` を 1 秒刻みで置く。

        i 番目の窓は `[start_sec + i − 0.5, start_sec + i + 0.5]` を覆い、行の時刻は
        その右端（＝窓が閉じた時刻）である。`heavy_secs` に挙げた時刻を覆う窓だけ
        `wall_us` を `heavy_wall_us` にする——「catch-up が起きた窓は重かったか」という
        仮説を、成立する側の入力で確かめるためである。
        """
        for i in range(count):
            close_sec = start_sec + 0.5 + i
            open_sec = close_sec - 1.0
            value = (
                heavy_wall_us
                if any(open_sec <= h <= close_sec for h in heavy_secs)
                else wall_us
            )
            at = T0 + timedelta(seconds=close_sec)
            self._add(
                at,
                tick_line(at, frame=100 + i * ticks, ticks=ticks, skipped=skipped,
                          wall_us=value),
            )
        return self

    def thread_reports(self, at_secs: tuple[float, ...]) -> "Fixture":
        """スレッド別・プロセス全体の CPU 報告を置く（任意種であることの素材）。"""
        for snap, at_sec in enumerate(at_secs, start=1):
            at = T0 + timedelta(seconds=at_sec)
            for text in thread_report_lines(at, snap=snap, t_s=int(at_sec)):
                self._add(at, text)
        return self

    def talk_span(self, start_sec: float, end_sec: float) -> "Fixture":
        """発話（talk）の開始と終了を置く。この区間の CPU 採取点が「発話中の頂」になる。"""
        start = T0 + timedelta(seconds=start_sec)
        end = T0 + timedelta(seconds=end_sec)
        self._add(start, talk_line(start, "steady_talk"))
        self._add(end, talk_line(end, "steady_talk_done"))
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

    def cpu_point(self, at_sec: float, value: float) -> "Fixture":
        """CPU 採取点を 1 点だけ足す（山を作って統計量の違いを見分ける用）。"""
        self.cpu.append((T0 + timedelta(seconds=at_sec), value))
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
                build: str | None = None, twin: str | None = None,
                emit_metrics: bool = False,
                contains: tuple[str, ...] = ()) -> "Fixture":
        """`case.txt` を書く。

        `contains` は「標準出力に必ず現れる文字列」で、何行でも書ける。終了コードだけを
        見る自己較正は「レポートの中身が消えた」壊れ方を捕まえられない——0.4.0 で足した
        §9 の表・仮説の語・`--emit-metrics` の行は、どれも終了コードを動かさないためである。
        `emit_metrics` を立てると、その fixture は `--emit-metrics` 付きで呼ばれる。
        """
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
        if emit_metrics:
            lines.append("emit_metrics = yes")
        lines.append("")
        lines += [f"note = {n}" for n in notes]
        if contains:
            lines.append("")
            lines += [f"contains = {c}" for c in contains]
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
#                                     （＝定常開始から最後の適用まで）
#   * FRAME_INTERVAL_MIN_SAMPLES=20 … 判定対象になる系列は間隔 20 本以上
#                                     （明示指定でもこれ未満なら合格は返らない）
#   * CONVERGENCE_MIN_SAMPLES_PER_WINDOW=3 x 窓 4 … CPU 採取点が 12 点以上

#: 判定対象の系列（judge-perf.py の FRAME_INTERVAL_JUDGED_SERIES と一致させること）。
JUDGED_TARGET = "TargetId(0)"
JUDGED_SURFACE = "1000"

#: 発火の scope（judge-perf.py の FRAME_INTERVAL_SERIES_SCOPE と一致させること）。
JUDGED_SCOPE = "0"

#: サイクル数。1 サイクル 1 秒なので 70 サイクルで定常状態の時間幅 60 秒を超える。
MAIN_CYCLES = 70

#: emo2 の `animation1400` と同じコマ待ち列（発火 → +0 → +150 → +172）。
#: 窓 C に残る間隔は 1 サイクルあたり 150ms と 22ms の 2 本。
MAIN_WAITS_MS = (150.0, 22.0)

#: 合格の上限ちょうど（FRAME_INTERVAL_MAX_WAIT_MS 150.0 x 1.15 = 172.5ms）。丸め桁は 3。
BOUNDARY_MS = 172.500
#: 上限を 1/1000 ms だけ超える値。run.log の時刻はマイクロ秒分解能なので表現できる。
OVER_BOUNDARY_MS = 172.501

# --- P13（同じ鍵に 2 本のアニメ・合格側へ倒れる位相）の寸法 -------------------
#
# P12 は「混ざると偽の不合格になる」位相を固定している。倒れる向きは位相しだいなので、
# **偽の合格へ倒れる位相** も同格に固定する（緑だけ・赤だけでは較正にならない）。
# 速いアニメのサイクルを詰める（発火の周期を 200ms にする）と、遅いアニメの 1 コマは
# 必ずどこかのサイクルの内側へ落ちる。すると本来 700ms の間隔が 2 本の短い間隔へ割られ、
# 遅いアニメの鈍さが判定の窓から完全に消える。

#: 速いアニメ（1400 相当）の発火周期。サイクル全長 172ms より僅かに長いだけにして、
#: サイクル間の空白を詰める（＝遅いアニメのコマがサイクルの内側へ落ちるようにする）。
P13_FAST_PERIOD_MS = 200.0
#: 速いアニメのサイクル数。200ms x 350 = 70 秒 ＞ VERDICT_MIN_STEADY_SEC=60 秒。
P13_FAST_CYCLES = 350
#: 遅いアニメ（1402 相当）の **本当の** コマ間隔。上限 172.5ms の 4 倍以上ある。
P13_SLOW_PERIOD_MS = 700.0
#: 遅いアニメのサイクル数。700ms x 100 = 70 秒（速いほうと同じ時間帯を覆う）。
P13_SLOW_CYCLES = 100

# --- P14（別 scope のアニメが判定対象の区間を覆う）の寸法 ---------------------
#
# 条件 4 が全 scope をまたいで活性本数を数えていると、ケロ（scope="1"）が動いているだけで
# キャラ（scope="0"）の間隔が落ちる。落ちた中に上限超過が入っていれば、残った健全な間隔
# だけで p95 が決まり、**確定的な不合格が合格へ反転する**。
# ここで組む形はその最短の再現である:
#   * 健全なサイクルを P14_HEALTHY_CYCLES 回（1 サイクルにつき 150ms と 22ms の 2 本）
#   * 上限を大きく超えるサイクルを P14_SLOW_CYCLES 回（1 サイクルにつき 1 本）
#   * 別 scope のアニメの発火・停止を、**上限超過の区間だけ** を覆うように置く
# 数え方が全 scope なら上限超過の間隔が全部落ち、残り 140 本の p95 は 150.000ms＝合格。
# 数え方が判定対象の scope だけなら 150 本すべてが残り、p95 は 400.000ms＝不合格。

#: 健全なサイクル数（判定対象系列・emo2 の animation1400 と同じ 150/22 のコマ待ち）。
P14_HEALTHY_CYCLES = 70
#: 上限超過のサイクル数。150 本中 10 本が超過なら p95（nearest-rank・rank=143）は超過側に入る。
P14_SLOW_CYCLES = 10
#: 上限超過のコマ間隔（上限 172.500ms の 2 倍以上）。
P14_SLOW_INTERVAL_MS = 400.0
#: 上限超過のサイクルを置き始める位置（秒）。健全なサイクル 70 本（70〜139 秒）の直後。
P14_SLOW_START_SEC = STEADY_START_SEC + P14_HEALTHY_CYCLES
#: 別 scope のアニメの発火を、上限超過の区間の何ミリ秒前に置くか。
P14_OTHER_SCOPE_LEAD_MS = 10.0
#: 別 scope のアニメが走っている長さ（発火から停止まで）。区間の両端を確実に覆う。
P14_OTHER_SCOPE_SPAN_MS = P14_OTHER_SCOPE_LEAD_MS + P14_SLOW_INTERVAL_MS + 10.0


# --- C1 / C2 / T2（0.4.0・catch-up の系統別と [tick] の有無）------------------
#
# 【何を塞ぐか】判定式⑵ は「定常状態の catch-up が 0 件か」しか見ないので、3 系統
# （dispatcher／kanade／loop_ticker）を取り違えても、時刻の突合が壊れても、仮説の語が
# 消えても、終了コードは 1 つも動かない。ゆえにこの 3 件は `contains` でレポート本文を
# 直接固定する（case.txt の contains 行）。
#
# 【3 系統を分けるのは target フィールドだけである】dispatcher と kanade は文言が
# 同一（"ticker catch-up: …"）で、違うのは `target="…"` の値だけである。文言で数える
# 実装はこの 2 系統を 1 つに潰すので、C1／C2 はどちらも 3 系統を 1 件ずつ含む。

#: catch-up を置く時刻（秒）。C1 は起動過渡（定常状態より前）＝判定式⑵ は合格側。
C1_CATCHUP_SECS = ((10.0, "dispatcher"), (20.0, "kanade"), (30.0, "loop_ticker"))
#: C2 は定常状態（＝判定式⑵ は不合格側）。定常開始は最初の perf 行 +60 秒＝T0+61 秒。
C2_CATCHUP_SECS = ((80.0, "dispatcher"), (90.0, "kanade"), (100.0, "loop_ticker"))

#: `[tick]` の窓を置き始める位置と本数。窓は 1 秒刻みで、i 番目は [start+i-0.5, start+i+0.5]。
C1_TICK_START_SEC, C1_TICK_COUNT = 5.0, 36
C2_TICK_START_SEC, C2_TICK_COUNT = 75.0, 36

#: 平常の窓の壁時計合計と、catch-up が起きた窓の壁時計合計。
#: C1 は 33 窓が 200,000 / 3 窓が 900,000 なので全体平均 258,333.3・比 3.484 ≧ 1.5＝成立。
#: C2 は全窓が 200,000 で比 1.000 ＜ 1.5＝不成立。
TICK_WALL_US_CALM = 200000
TICK_WALL_US_HEAVY = 900000

#: C1 の発話区間（秒）と、その内側に置く CPU の山。発話中の頂（要件 5.4）の素材。
C1_TALK_FROM_SEC, C1_TALK_TO_SEC = 90.0, 95.0
C1_TALK_PEAK_AT_SEC, C1_TALK_PEAK_PCT = 92.5, 4.20


def case_C1() -> Fixture:
    return (
        Fixture("C1_catchup_by_target_pass")
        .boot()
        .cycle_series(JUDGED_TARGET, JUDGED_SURFACE, MAIN_WAITS_MS, MAIN_CYCLES)
        .catchups(C1_CATCHUP_SECS)
        .tick_windows(
            C1_TICK_START_SEC, C1_TICK_COUNT, wall_us=TICK_WALL_US_CALM,
            heavy_secs=tuple(sec for sec, _t in C1_CATCHUP_SECS),
            heavy_wall_us=TICK_WALL_US_HEAVY,
        )
        .thread_reports((30.0,))
        .talk_span(C1_TALK_FROM_SEC, C1_TALK_TO_SEC)
        .cpu_series()
        .cpu_point(C1_TALK_PEAK_AT_SEC, C1_TALK_PEAK_PCT)
        .run_meta()
        .declare(
            "verdict", 0, "catch-up が 3 系統とも起動過渡だけにある（合格側）",
            build="release", emit_metrics=True,
            notes=[
                "H11 と同じ健全な走行に、catch-up を 3 系統（dispatcher・kanade・",
                "loop_ticker）から 1 件ずつ、いずれも起動過渡（定常状態より前）に置いた。",
                "判定式⑵ が数えるのは定常状態だけなので合格（終了コード 0）である。",
                "同時に、この走行には任意種の行（[tick] kind=window・perf(thread)・",
                "perf(process)）と発話区間の印が入っている。任意種は 1 本も無くても",
                "判定が成立しなければならず、有っても判定を動かしてはならない。",
                "その「有る側」がこの fixture である（無い側は既存の 17 件すべて）。",
                "[tick] の窓は catch-up が起きた 3 窓だけ壁時計合計が 4.5 倍あるので、",
                "「フレーム駆動の負荷が起床を遅らせる」仮説は 成立 と印字されなければならない。",
                "終了コードはこれらの中身を 1 つも反映しないため、contains 行で本文を固定する。",
            ],
            contains=(
                "仮説: 成立",
                "target=dispatcher",
                "target=kanade",
                "target=loop_ticker",
                "metric=steady_idle_cpu_mean_pct value=1.61",
                "metric=frame_interval_p95_ms value=150.000",
                "metric=catchup_count value=0",
                "metric=catchup_count_total value=3",
                "metric=catchup_dispatcher value=1",
                "metric=catchup_kanade value=1",
                "metric=catchup_loop_ticker value=1",
                "metric=alloc_count value=0",
                "metric=talk_peak_cpu_pct value=4.20",
                "metric=cpu_max_pct value=4.20",
                "metric=tick_skip_ratio value=0.000",
                "metric=catchup_tick_load_ratio value=3.484",
            ),
        )
    )


def case_C2() -> Fixture:
    return (
        Fixture("C2_catchup_by_target_fail")
        .boot()
        .cycle_series(JUDGED_TARGET, JUDGED_SURFACE, MAIN_WAITS_MS, MAIN_CYCLES)
        .catchups(C2_CATCHUP_SECS)
        .tick_windows(C2_TICK_START_SEC, C2_TICK_COUNT, wall_us=TICK_WALL_US_CALM)
        .cpu_series()
        .run_meta()
        .declare(
            "verdict", 1, "catch-up が 3 系統とも定常状態にある（不合格側）",
            build="release",
            notes=[
                "C1 と同じ走行で、catch-up の 3 件だけを定常状態へ移したもの。",
                "判定式⑵ は定常状態の catch-up 0 件を求めるので不合格（終了コード 1）である。",
                "C1 と対にして置いてあるのは、合格側だけ・不合格側だけでは「数えているのか",
                "数えていないのか」が区別できないためである。",
                "[tick] の窓はどれも同じ壁時計合計なので、catch-up が起きた窓と全体の比は",
                "1.000 であり、仮説は 不成立 と印字されなければならない。",
                "仮説の語は終了コードを動かさないので contains 行で固定する。",
            ],
            contains=(
                "仮説: 不成立",
                "target=dispatcher",
                "target=kanade",
                "target=loop_ticker",
            ),
        )
    )


def case_T2() -> Fixture:
    return (
        Fixture("T2_tick_lines_absent")
        .boot()
        .cycle_series(JUDGED_TARGET, JUDGED_SURFACE, MAIN_WAITS_MS, MAIN_CYCLES)
        .catchups(C1_CATCHUP_SECS)
        .cpu_series()
        .run_meta()
        .declare(
            "baseline", 0, "catch-up はあるが [tick] 行が 1 本も無い（仮説は判定不能）",
            emit_metrics=True,
            notes=[
                "C1 から任意種の行（[tick]・perf(thread)・perf(process)）と発話区間の印を",
                "すべて落とした走行。任意種が無くても集計は成立し、終了コードは 0 である",
                "（任意種は必要ログ種ではない）。",
                "ただし [tick] が無ければ「フレーム駆動の負荷が起床を遅らせる」仮説は",
                "数値で確かめようがない。ここで 不成立 と書くと「調べたが違った」と読まれる",
                "ので、判定不能 でなければならない。無観測を結論にしないという規律である。",
                "同じ理由で、発話区間の印が無い走行の発話中の頂は 0 ではなく - である。",
            ],
            contains=(
                "仮説: 判定不能",
                "metric=talk_peak_cpu_pct value=-",
                "metric=tick_skip_ratio value=-",
                "metric=catchup_tick_load_ratio value=-",
                "metric=catchup_count_total value=3",
            ),
        )
    )


def case_H11() -> Fixture:
    return (
        Fixture("H11_healthy_judged_series")
        .boot()
        .cycle_series(JUDGED_TARGET, JUDGED_SURFACE, MAIN_WAITS_MS, MAIN_CYCLES)
        .series("TargetId(0)", "1001", 180.0, 31)
        .cpu_series()
        .run_meta()
        .declare(
            "verdict", 0, "健全な走行（合格の陽性対照）", build="release",
            notes=[
                "判定対象の系列（TargetId(0) / surface_id=1000）が emo2 の animation1400 と",
                "同じ形で動いている走行。発火のたびに +0 / +150 / +172 ミリ秒で 3 コマを",
                "適用するので、判定の窓には 150ms と 22ms の間隔が並び、上位 5% は 150ms。",
                "上限 172.500ms の内側なので合格（終了コード 0）でなければならない。",
                "同じ対象の下に、判定対象に指定していない系列（surface_id=1001・180ms）も",
                "置いてある。指定外の系列があっても合格を止めないこと——これが「対象を",
                "明示指定すれば除外は意図的になる」という規則の生きている姿である。",
                "これは赤ケースと同格に重要である。判定を壊して何もかも不合格にすれば赤は",
                "出るので、赤だけでは判定が生きていることを確かめられない。",
                "合格が出ることまで見て初めて、判定が生きていると言える。",
            ],
        )
    )


def case_H12() -> Fixture:
    return (
        Fixture("H12_judged_series_at_boundary")
        .boot()
        .cycle_series(JUDGED_TARGET, JUDGED_SURFACE, (BOUNDARY_MS,), MAIN_CYCLES)
        .cpu_series()
        .run_meta()
        .declare(
            "verdict", 0, "上限ちょうど（等号は合格側）", build="release",
            notes=[
                "判定対象の系列のコマ間隔が上限ちょうど 172.500ms（＝150.0ms x 1.15）の走行。",
                "判定は「以下」なので合格（終了コード 0）である。",
                "H13 とはこの間隔が 1/1000 ミリ秒だけ違うだけで、他は完全に同じである。",
                "2 つを並べて置いてあるのは、境界のどちら側に等号が付いているかを",
                "実際に走らせて確かめるためである。",
                "上限は掛け算で出るので、較正値の組によっては計算機の中で割り切れず、",
                "丸めずに比べると「ちょうど上限」の走行が辻褄の合わない理由で不合格になる",
                "（是正前の 172.0 x 1.15 = 197.79999999999998 が実際にその形だった）。",
                "この fixture はその丸め（小数点以下 3 桁）が効いていることを固定している。",
                "【上限が 150ms 側であることの証拠でもある】172.500ms は是正前の上限",
                "197.800ms の内側にある。H12 が緑・H13 が赤になることで、上限がどこにあるかを",
                "1/1000 ミリ秒の精度で示している。上限の土台を是正前の 172.0ms へ戻すと、",
                "H13 の 172.501ms も 197.800ms の内側に入って赤が消える。",
            ],
        )
    )


def case_H12_colored() -> Fixture:
    fixture = (
        Fixture("H12c_judged_series_at_boundary_colored")
        .boot()
        .cycle_series(JUDGED_TARGET, JUDGED_SURFACE, (BOUNDARY_MS,), MAIN_CYCLES)
        .cpu_series()
        .run_meta()
    )
    fixture.declare(
        "verdict", 0, "色付きログ（色を落とす経路の較正）", build="release",
        twin="H12_judged_series_at_boundary",
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
        Fixture("H13_judged_series_over_boundary")
        .boot()
        .cycle_series(JUDGED_TARGET, JUDGED_SURFACE, (OVER_BOUNDARY_MS,), MAIN_CYCLES)
        .cpu_series()
        .run_meta()
        .declare(
            "verdict", 1, "上限を 1/1000 ミリ秒だけ超過（不合格）", build="release",
            notes=[
                "H12 と完全に同じで、コマ間隔だけが 172.501ms（上限 +0.001ms）。",
                "不合格（終了コード 1）でなければならない。",
                "H12 と対で置いてあり、2 つの終了コードが違うことが「境界がここにある」",
                "という主張そのものである。片方だけでは境界を測ったことにならない。",
            ],
        )
    )


def case_H1b() -> Fixture:
    return (
        Fixture("H1b_unpinned_slow_series")
        .boot()
        .cycle_series(JUDGED_TARGET, JUDGED_SURFACE, MAIN_WAITS_MS, MAIN_CYCLES)
        .series("TargetId(0)", "1005", 300.0, 31)
        .cpu_series()
        .run_meta()
        .declare(
            "verdict", 0, "指定外の遅い系列は合格を止めない（明示指定の代償・記録）",
            build="release",
            notes=[
                "判定対象の系列は健全だが、同じ対象の下に 300ms 間隔の遅い系列",
                "（surface_id=1005）が並んでいる走行。judge-perf.py はこれを合格",
                "（終了コード 0）にする。遅い系列は判定対象に指定されていないからである。",
                "【これは欠陥ではなく、明示指定の代償である】判定対象の列挙は全部か無しかで、",
                "書き漏らした系列は黙って対象外になり、どれだけ遅くても合格を止めない。",
                "だから列挙の完全性が合否の意味を支えている。ここでその代償を記録として",
                "固定しておき、対象を増やしたときにこの期待値も一緒に見直させる。",
                "レポートには外した系列が名指しで並ぶので、読み手が見落とすことはない。",
                "期待値が 0 なのは意図であって、見落としではない。",
                "【以前この fixture が固定していたもの】判定対象が自動選別だった頃、この形は",
                "「速い系列の裾に遅い系列が丸ごと吸収されて合格に化ける」欠陥の再現だった。",
                "判定を系列ごとに行う形（FRAME_INTERVAL_JUDGE_GRANULARITY）は今も生きており、",
                "対象を 2 本以上に増やせばこの入力は再び赤になる。",
            ],
        )
    )


def case_H14() -> Fixture:
    return (
        Fixture("H14_fail_beats_inconclusive")
        .boot()
        .cycle_series(JUDGED_TARGET, JUDGED_SURFACE, (OVER_BOUNDARY_MS,), MAIN_CYCLES)
        .cpu_series(count=4)
        .run_meta()
        .declare(
            "verdict", 1, "不合格と判定不能が同時に立つとき（不合格が勝つ）", build="release",
            notes=[
                "H13 と同じくコマ間隔が上限を 1/1000 ミリ秒超えており、さらに CPU の採取点が",
                "4 点しかない走行。CPU の判定は採取点が足りないので判定不能へ倒れるが、",
                "コマ間隔の判定は確定的な不合格である。総合は不合格（終了コード 1）で",
                "なければならない。",
                "不合格が判定不能に埋もれると、直すべき欠陥が「測れなかった」の山に隠れる。",
                "その優先順位を固定している。",
            ],
        )
    )


def case_H20() -> Fixture:
    return (
        Fixture("H20_judged_series_absent")
        .boot()
        .cycle_series("TargetId(0)", "1005", MAIN_WAITS_MS, MAIN_CYCLES)
        .cpu_series()
        .run_meta()
        .declare(
            "verdict", 2, "判定対象に指定した系列が観測されていない（判定不能）",
            build="release",
            notes=[
                "判定対象に指定した系列（TargetId(0) / surface_id=1000）が、定常状態の記録に",
                "1 度も現れない走行。別の系列（surface_id=1005）だけが健全に動いている。",
                "判定不能（終了コード 2）でなければならない。",
                "【なぜ黙って通りやすいか】指定した系列は記録に現れないので、判定対象の",
                "一覧にも除外の一覧にも出てこない。数え直す元の表にすら現れないため、",
                "「残った系列が上限内だった＝合格」に化ける。指定と実測を直接突き合わせる",
                "関門がないと、この形は静かに合格する。",
                "ログの絞り込み設定を誤って片方のゴーストの詳細ログが落ちた走行、",
                "面の呼称や対象の番号が変わった走行が、実際にこの形になる。",
                "指定は全部か無しかなので、指定した系列が観測されていることまで含めて",
                "初めて合格が意味を持つ。",
            ],
        )
    )


def case_H21() -> Fixture:
    return (
        Fixture("H21_judged_series_outside_window")
        .boot()
        .series(JUDGED_TARGET, JUDGED_SURFACE, 150.0, 400)
        .cpu_series()
        .run_meta()
        .declare(
            "verdict", 2, "判定対象の系列が判定の窓に 1 本も残らない（判定不能）",
            build="release",
            notes=[
                "判定対象の系列が定常状態で 400 回も適用されているのに、アニメの発火の記録が",
                "1 本も無い走行。判定の窓は発火を起点に切り出すので、間隔が 1 本も残らない。",
                "判定不能（終了コード 2）でなければならない。",
                "【なぜ黙って通りやすいか】間隔が 1 本も残らない系列は間隔の表に鍵が立たず、",
                "判定対象にも除外にも現れない。しかも間隔が 150ms と健全なので、",
                "窓の外を見れば「問題ない走行」に見える。ここを合格にすると、",
                "「窓の設定が壊れていて何も測れていない」と「測って問題が無かった」が",
                "区別できなくなる。",
                "発火の記録が出ない走行（ログ水準の絞り込み・アニメ定義の変更）と、",
                "適用が発火から離れすぎている走行（＝本 spec が追っている鈍化そのもの）が、",
                "同じ経路でここへ来る。どちらも判定不能である。",
            ],
        )
    )


def case_P12() -> Fixture:
    fixture = Fixture("P12_same_key_two_animations").boot()
    fixture.cycle_series(JUDGED_TARGET, JUDGED_SURFACE, MAIN_WAITS_MS, MAIN_CYCLES)
    # 同じ (target_id, surface_id) の上でもう 1 本のアニメが同時に走っている状態。
    # 適用は同じ鍵の列に混ざるため、混ざった列の隣り合う差はどちらのアニメの間隔でもなくなる。
    # **この 2 本目は発火の記録を出さない**（`series()` は適用だけを置く）。窓 C の条件 4 は
    # 発火・停止の収支から活性本数を推定するので、記録の無いこの形は今も見張れない。
    # 記録を出す形は P13 が固定している（あちらは条件 4 が捕まえて判定不能になる）。
    fixture.series(JUDGED_TARGET, JUDGED_SURFACE, 700.0, 74, offset_ms=60.0)
    return (
        fixture.cpu_series()
        .run_meta()
        .declare(
            "verdict", 1,
            "同じ鍵の上で 2 本のアニメが混ざる（2 本目に発火の記録が無い・残る穴）",
            build="release",
            notes=[
                "1 つの対象・1 つのサーフェスの上で 2 本のアニメが同時に走っている走行。",
                "適用の記録は同じ鍵の 1 本の列に混ざるので、隣り合う記録の差は",
                "「どちらかのアニメのコマ間隔」ではなくなる。",
                "【この fixture が固定している「残る穴」】2 本目のアニメは適用の記録だけを",
                "出し、発火の記録を 1 本も出さない。判定の窓の条件 4 は発火と停止の記録の",
                "収支から「今何本走っているか」を推定するので、記録を出さないこの形は",
                "数えられない＝見張れない。ゆえに混ざった列がそのまま判定に使われる。",
                "現状は不合格（終了コード 1）になる。判定の窓に残る間隔の上位 5% が",
                "588.000ms で、上限 172.500ms を超えるためである（実測値）。",
                "この 588ms は、速いアニメのサイクル最後のコマから、遅いアニメ（700ms 間隔）の",
                "次のコマまでの空きであり、**どちらのアニメのコマ間隔でもない**。",
                "【これは⑴ の欠陥ではない・窓を発火起点にしても残る】窓はアニメの発火を起点に",
                "区間を切るが、切った区間の中に混ざり込んだ別アニメの適用までは分けられない",
                "——記録は対象とサーフェスの組で 1 本の列になっており、どのアニメが出した",
                "適用かを区別する情報がログに無いためである。塞ぐには記録側にアニメの",
                "識別子を足す必要があり、本 spec の範囲外である。",
                "【逆向きの誤り（＝偽の合格）は P13 が固定している】2 本目のコマがサイクルの",
                "内側へ落ちる位相だと、長い間隔が短い間隔 2 本に割られて、遅いアニメが合格に",
                "化ける。混ざり方の位相しだいで偽の不合格にも偽の合格にも倒れる。",
                "2 本目が発火の記録を出していれば条件 4 がその位相を捕まえて判定不能にする",
                "——それを固定しているのが P13_same_key_two_animations_pass_side である。",
                "実機の emo2 でも、キャラの surface1000 には animation1400 と 1402 が",
                "同居する（長時間走行で 1400 が 212 回・1402 が 28 回発火した。どちらも",
                "scope=\"0\" である）。この 2 本はどちらも発火の記録を出すので条件 4 の",
                "見張りが効くが、2026-08-14 の 3 走行では両者が同時に走っていた区間は",
                "1 つも無く、条件 4 が落とした間隔は 0 本だった。",
                "ここでは「現状はこうなる」という事実を記録として固定しておき、",
                "窓の定義を変えたときにこの fixture の期待値も一緒に見直させる。",
                "期待値が 1 なのは意図であって、判定の欠陥ではない。",
            ],
        )
    )


def case_P13() -> Fixture:
    fixture = Fixture("P13_same_key_two_animations_pass_side").boot()
    # 速いアニメ: 200ms ごとに発火し、+0 / +150 / +172 ミリ秒でコマを出す（健全な形）。
    fixture.cycle_series(
        JUDGED_TARGET, JUDGED_SURFACE, MAIN_WAITS_MS, P13_FAST_CYCLES,
        animation_id="1400", cycle_period_ms=P13_FAST_PERIOD_MS,
    )
    # 遅いアニメ: 同じ対象・同じサーフェスの上で、700ms に 1 コマしか出さない。
    # 実機の emo2 と同じく、こちらにも自分の発火の記録がある（scope は系列単位なので同じ "0"）。
    fixture.cycle_series(
        JUDGED_TARGET, JUDGED_SURFACE, (), P13_SLOW_CYCLES,
        animation_id="1402", cycle_period_ms=P13_SLOW_PERIOD_MS,
    )
    return (
        fixture.cpu_series()
        .run_meta()
        .declare(
            "verdict", 2, "同じ鍵の上で 2 本のアニメが混ざる（合格側へ倒れる位相）",
            build="release",
            notes=[
                "P12 と同じ「1 つの対象・1 つのサーフェスの上で 2 本のアニメが同時に走る」形。",
                "違うのは混ざり方の位相だけである。速いほうの発火を 200 ミリ秒ごとに詰めたので、",
                "遅いほう（700 ミリ秒に 1 コマ）のコマは必ずどこかのサイクルの内側へ落ちる。",
                "すると本当は 700 ミリ秒ある間隔が 2 本の短い間隔に割られ、遅いアニメの鈍さが",
                "判定の窓から完全に消える。",
                "【これが赤ケースである理由】この形を放っておくと、判定は合格（終了コード 0）を",
                "返す。上限 172.500 ミリ秒の 4 倍で動いているアニメがあるのに、である。",
                "P12 が固定しているのは偽の不合格の側だけで、偽の合格の側には見張りが",
                "1 つも無かった。倒れる向きは位相しだいなので、両側を同格に置く。",
                "【今は何が止めるか】判定の窓の条件 4（同時に走っているアニメの推定本数が",
                "1 本を超える区間の間隔は使わない）が、この走行の間隔を全部落とす。",
                "落とした結果、判定対象に指定した系列の間隔が 1 本も残らないので、",
                "判定不能（終了コード 2）になる。合格には決してならない。",
                "【条件 4 が見張れる範囲】2 本目のアニメが自分の発火の記録を出していることが",
                "前提である。記録の無い書き換えが同じ鍵に混ざる形は今も見張れない",
                "——それを固定しているのが P12 である（あちらは 2 本目に発火の記録が無い）。",
            ],
        )
    )


def case_P14() -> Fixture:
    fixture = Fixture("P14_other_scope_animation_overlap").boot()
    # 判定対象の系列（scope="0" のキャラ）。まず健全なサイクルを並べる。
    fixture.cycle_series(
        JUDGED_TARGET, JUDGED_SURFACE, MAIN_WAITS_MS, P14_HEALTHY_CYCLES,
        animation_id="1400",
    )
    # 同じアニメが鈍った区間。1 サイクルにつき 400ms の間隔が 1 本だけ残る。
    fixture.cycle_series(
        JUDGED_TARGET, JUDGED_SURFACE, (P14_SLOW_INTERVAL_MS,), P14_SLOW_CYCLES,
        animation_id="1400", start_sec=P14_SLOW_START_SEC,
    )
    # 別 scope（ケロ）のアニメが、**鈍った区間だけ** を覆って走っている。
    # 別の対象へコマを出すので、判定対象の系列の列には 1 行も混ざらない。
    for c in range(P14_SLOW_CYCLES):
        fixture.other_scope_span(
            at_sec=P14_SLOW_START_SEC + c - P14_OTHER_SCOPE_LEAD_MS / 1000.0,
            duration_ms=P14_OTHER_SCOPE_SPAN_MS,
        )
    return (
        fixture.cpu_series()
        .run_meta()
        .declare(
            "verdict", 1, "別 scope のアニメが重なっても判定対象の間隔は落ちない",
            build="release",
            notes=[
                "判定対象の系列（キャラ・scope=\"0\"）に、上限を大きく超えるコマ間隔",
                "400.000ms が 10 本ある走行。上限は 172.500ms なので不合格（終了コード 1）で",
                "なければならない。",
                "その 10 本の区間に重ねて、**別の scope（ケロ・scope=\"1\"）のアニメ** の",
                "発火と停止を置いてある。ケロのアニメは別の対象へコマを出すので、キャラの",
                "記録の列には 1 行も混ざらない。キャラのコマ送りが健全かどうかについて",
                "何も語らない情報である。",
                "【この fixture が押さえている経路】判定の窓の条件 4（同時に走っている",
                "アニメの推定本数が 1 本を超える区間の間隔は使わない）を、scope をまたいで",
                "数えると、ケロが動いているというだけでキャラの間隔が落ちる。",
                "落ちたのが上限超過の 10 本ちょうどなら、残るのは健全な 140 本だけになり、",
                "上位 5% は 150.000ms＝**確定的な不合格が合格（終了コード 0）へ反転する**。",
                "数える範囲を判定対象の系列の scope に限れば 150 本すべてが残り、",
                "上位 5% は 400.000ms＝不合格である。",
                "終了コードが 1 と 0 に分かれるので、この fixture だけで 2 つの数え方を",
                "区別できる。条件 4 の数える範囲を全 scope へ戻すと、ここが赤になる。",
                "【なぜ足したか】この形は実機のベースラインで実際に起きていた。長時間 release で",
                "42 本の間隔が落ちており、その 42 本は 1 本残らず「キャラ 1 本＋ケロ 1 本」で、",
                "キャラの 2 本重なりは 1 件も無かった。落ちた 42 本には上限超過が 1 本",
                "（173.997ms）含まれ、上限超過が 9 件から 8 件へ減っていた。",
                "corpus には scope をまたぐ形が 1 つも無かったので（P12・P13 はどちらも",
                "2 本とも scope=\"0\"）、この誤りは 3 回のレビューを素通りした。",
                "陽性対照の無い経路は静かに壊れる——本 spec が三度踏んだ形である。",
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
        .cycle_series(JUDGED_TARGET, JUDGED_SURFACE, MAIN_WAITS_MS, MAIN_CYCLES)
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
        .cycle_series(JUDGED_TARGET, JUDGED_SURFACE, MAIN_WAITS_MS, MAIN_CYCLES)
        .cpu_series()
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
        .cycle_series(JUDGED_TARGET, JUDGED_SURFACE, MAIN_WAITS_MS, MAIN_CYCLES)
        .cpu_series()
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
        .cycle_series(JUDGED_TARGET, JUDGED_SURFACE, MAIN_WAITS_MS, MAIN_CYCLES)
        .cpu_series()
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
        .series(JUDGED_TARGET, JUDGED_SURFACE, 150.0, 40)
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
    case_H11, case_H12, case_H12_colored, case_H13, case_H1b, case_H14,
    case_H20, case_H21, case_P12, case_P13, case_P14,
    case_C1, case_C2, case_T2,
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
