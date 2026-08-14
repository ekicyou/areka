#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""areka 性能計測 判定スクリプト（集計モード・判定モード）。

`invoke-perf-run.ps1` が採取した実走ログ（run.log）と CPU 時系列（cpu.csv）を読み、

  * 集計モード（`--mode baseline`）: 段階別所要・キャッシュ命中率・コマ適用間隔・確保計数・
    進行境界スキップ件数・CPU 統計・合成キーの異なり数を集計する（要件 2.2・7.2）
  * 判定モード（`--mode verdict`）: 合格判定式⑴〜⑷（要件 4.2）を適用ビルドの別（要件 4.3/4.4）
    に従って判定し、CPU 時系列の収束判定（要件 5.1・5.3）を行う。判定根拠は較正値つきで出す

このスクリプトは Python 標準ライブラリだけで動く。リポジトリのビルドグラフには入らない
（design.md「Boundary Commitments / Allowed Dependencies」）。

使い方
------
    python judge-perf.py <run.log> <cpu.csv> --mode baseline [--build dev|release] [--meta <run-meta.txt>]
    python judge-perf.py <run.log> <cpu.csv> --mode verdict  --build dev|release   [--meta <run-meta.txt>]
    python judge-perf.py --selftest

判定モードでは `--build` の明示が要る（どの判定式を適用するかがビルド種別で変わるため）。

`--selftest`（自己較正）は `fixtures/` の既知ログを **実運用と同じ入口（`main(argv)`）** で
走らせ、各 fixture の `case.txt` に書いてある期待終了コードを逐語再現する（要件 2.4）。
run.log・cpu.csv の指定は要らない。詳しくは「自己較正（--selftest）」の節を参照。

終了コード（design.md「Error Handling」・D7・signoff-scan.py 前例踏襲）
--------------------------------------------------------------------
    0  合格（集計モードは正常終了。判定モードは適用される判定式が全て合格）
    1  不合格（判定モード専用。集計モードは 1 を返さない）
    2  判定不能——必要ログ種の欠落・段フィールドの欠落・観測ゼロ・定常状態が空。
       部分集計は一切出さない（要件 2.5。沈黙を合格として読ませない）
    3  引数不正・ファイル読取不能

`--selftest` も同じ体系に載せる: `0`=全 fixture が期待どおり／`1`=1 つでも食い違い（＝道具が
壊れている＝不合格）／`3`=fixture 置き場そのものが読めない（引数・ファイルの問題）。`2` は
返さない（自己較正に「判定不能」は無い——読めたか読めなかったかのどちらかである）。

**不合格(1) は判定不能(2) より優先する**。1 つの判定式が確定的に不合格で、別の判定式が
判定不能なら、総合は不合格である（design.md「Error Handling」）。

「観測が無い」を合格として読ませないこと（D7）
----------------------------------------------
判定モードは判定式ごとに「支える観測集合が空のときどうなるか」を持ち、レポートに明記する。
とくに **定常状態（ウォームアップ除外後）が空の走行は判定不能** である——空だと判定式⑵
（catch-up 0 件）と⑶（確保 0 件）が「観測が無いから 0」で恒真になるため。集計モードは
全区間の集計＋警告を出して 0 で終わるのが正しく、判定モードだけがこの区別を持つ。

run-meta.txt の所在（要件 4.5）
------------------------------
既定では `cpu.csv` と同じディレクトリの `run-meta.txt` を自動で探す
（`invoke-perf-run.ps1` が run.log・cpu.csv・run-meta.txt を同一の出力先へ並べて置くため）。
`--meta` を渡せばその位置を上書きできる。マシン条件はレポート先頭に必ず併記するので、
run-meta.txt が見つからない走行は「条件を書けない集計」として exit 2 で拒否する。
判定モードはさらに `profile` の記載も必須にする（無ければ exit 2）。判定式⑷b の適用条件が
profile で変わるため、profile が無いと途中で切れた長時間走行が黙って「適用外・exit 0」に
なるからである。`build` は `--build` と突合しているのに profile を見ていなかった非対称を塞ぐ。

ログ行の形（実測で確定した契約面）
--------------------------------
areka は `tracing_subscriber::fmt().with_env_filter(..).init()` で初期化する
（`crates/areka/src/main.rs:260-264`）。この既定書式が出す 1 行は次の形である：

    <時刻>Z <水準> <スパン>: <target>: <メッセージ> <名前>=<値> <名前>=<値> ...

実測サンプル（本スクリプトの開発時に、同一バージョンの tracing 0.1.44 /
tracing-subscriber 0.3.23 を同一の初期化で走らせて採取した逐語）:

    2026-08-14T04:17:25.043998Z DEBUG actor{actor="emo-text"}: \
    areka_emo_present::presenter::timing: perf(apply_show): 段階別計時 \
    target_id=TargetId(0) surface_id=1000 cache_hit=false t_cache_us=83 ...

【重要】色付け（ANSI エスケープ）は既定で **有効** である。
`tracing-subscriber-0.3.23/src/fmt/fmt_layer.rs:739-755` の `Default for Layer` は
「ansi feature が有効で、かつ環境変数 `NO_COLOR` が未設定または空なら色を付ける」と
決めており、出力先が端末かどうかを見ていない。`invoke-perf-run.ps1` は `NO_COLOR` を
設定しないため、ファイルへリダイレクトした run.log にも色コードが入る。
本スクリプトは読み込み時に ANSI エスケープを除去してから解析する（除去後の行が
`NO_COLOR=1` で採取した行と逐語一致することを実測で確認済み）。
"""

from __future__ import annotations

import argparse
import csv
import math
import re
import sys
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from pathlib import Path

# =============================================================================
# 較正値バナー（要件 2.6 / 4.5 / 5.3）
# -----------------------------------------------------------------------------
# 本スクリプトが使う値は **すべてここにある**。README・判定モード・
# task 2.4（自己較正）はこのバナーを唯一の所在として参照する。値を変えるときはここだけを
# 書き換える。
#
# 【他ゴーストへ流用しないこと】末尾に「emo2 固有」と書いた値は、計測 fixture である
# ゴースト emo2 のアニメ定義・実寸から来ている。別のゴーストで測るときは必ず測り直す。
# =============================================================================

#: 本スクリプトの版（レポート先頭に出す。較正値を変えたら上げる）。
SCRIPT_VERSION = (
    "0.2.3 (task 2.4 / 集計モード＋判定モード＋自己較正モード・"
    "判定式と較正値は 0.2.2 から不変)"
)

#: 終了コード表（バナーが唯一の所在であるため、docstring だけでなくここにも置く）。
#: 【優先順位】FAIL(1) は INCONCLUSIVE(2) より優先する（design.md「Error Handling」）。
#: 判定式が 1 つでも確定的に不合格なら、他が判定不能でも総合は不合格である。
EXIT_CODE_TABLE = (
    ("0", "合格（適用される判定式が全て合格。適用外だけで 0 にはしない）"),
    ("1", "不合格（判定式のいずれかが確定的に不合格）"),
    ("2", "判定不能（観測ゼロ・必要ログ種の欠落・定常状態が空・データ不足）"),
    ("3", "引数不正・ファイル読取不能"),
)

#: 自己較正（`--selftest`）の較正値。集計・判定の数値には一切関与しないため、
#: レポート[2] の較正値表ではなく `--selftest` 自身の出力へ印字する（所在はここ 1 箇所）。
SELFTEST_FIXTURES_DIRNAME = "fixtures"
SELFTEST_CASE_FILENAME = "case.txt"

#: 自己較正が「毎回赤も作る」ことの関門（D7・steering「検証の道具そのものが壊れる・較正せよ」）。
#: ここに挙げた終了コードは、それを期待する fixture が **1 つ以上実際に走って再現している**
#: ことを要求する。緑しか走らない自己較正は、道具が壊れていても緑を出すので較正にならない。
#: （1=不合格・2=判定不能。終了コード定数 EXIT_FAIL / EXIT_INCONCLUSIVE より前に置くので
#:   数値で書き、定義後に取り違えが無いことを検査する。）
SELFTEST_REQUIRED_RED_CODES = (1, 2)

#: 総合判定の決め方（要件 4.2 の運用形・D7「沈黙を PASS にしない」）。
VERDICT_PRECEDENCE = (
    "不合格が 1 つでもあれば 不合格(1)／"
    "なければ判定不能が 1 つでもあれば 判定不能(2)／"
    "合格が 1 つ以上あれば 合格(0)／"
    "全て適用外なら 判定不能(2)"
)

# --- コマ適用間隔（要件 4.2⑴ の入力） ---------------------------------------

#: 期待するコマ適用間隔（ミリ秒）。emo2 のまばたきアニメ定義値。**emo2 固有**。
FRAME_INTERVAL_EXPECTED_MS = 172.0

#: 期待値からの許容率。判定は p95 <= 期待値 x (1 + 許容率)（要件 4.2⑴）。
#: 暫定値であり、task 3.1 の第 1 段ベースラインで確定する。
FRAME_INTERVAL_TOLERANCE = 0.15

#: コマ適用間隔の系列の分け方（design.md「較正値台帳」FRAME_INTERVAL_WINDOW）。
#: 同時刻に別対象へ適用が走ると、混ぜた列の差分は「間隔」ではなくなる（重畳による
#: 偽陽性・偽陰性）。ゆえに `target_id` と `surface_id` の組ごとに列を分け、
#: 各列の中で連続する適用の時刻差だけを間隔として数える。
FRAME_INTERVAL_SERIES_KEY = ("target_id", "surface_id")

#: 測定窓の定義。**未確定**であることを明示しておく。
#: design.md の台帳は「アイドル区間（talk 再生・複数アニメ重畳を除外した窓）限定」と
#: 定めているが、窓の境界条件は task 3.1 の第 1 段ベースラインで確定して README へ登記する
#: 手筈になっている。本スクリプトは下の 2 つの窓を **両方** 集計して並べて出す：
#:   "warmup"  : 起動過渡（WARMUP_EXCLUDE_SEC）を除いただけの窓。確実に計算できる
#:   "idle"    : さらに「同時に走っているアニメが FRAME_INTERVAL_IDLE_MAX_ACTIVE 本以下」
#:               の区間だけに絞った窓。seriko の発火・停止 info! の収支から推定する
#: talk 再生の除外は、現在のログ水準（RUST_LOG=info,areka_emo_present=debug）で
#: talk の開始・終了を名指しできる行が無いため **実装していない**。3.1 のベースラインで
#: 目印になる行を決め、FRAME_INTERVAL_IDLE_EXCLUDE_MARKERS へ入れること。
FRAME_INTERVAL_WINDOW = "warmup と idle の 2 通りを併記（idle の境界は task 3.1 で確定）"

#: idle 窓とみなす「同時に走っているアニメの本数」の上限。
#: 1 本（まばたき単独）までをアイドルとみなす暫定値。task 3.1 で確定する。
FRAME_INTERVAL_IDLE_MAX_ACTIVE = 1

#: talk 再生区間を idle 窓から外すための目印（メッセージの一部）。**現在は空**。
#: task 3.1 が第 1 段ベースラインの実走ログを見て確定し、ここへ入れる。
#: 空のままだと talk 中の適用が idle 窓に混ざるため、レポートにその旨を明記する。
FRAME_INTERVAL_IDLE_EXCLUDE_MARKERS: tuple[str, ...] = ()

#: 判定式⑴が使う窓（要件 4.2⑴・design.md 較正値台帳）。台帳は「アイドル区間限定」と定めるので
#: 窓 B を使う。窓 A（起動過渡のみ除外）は参考として集計に併記するだけで判定には使わない。
FRAME_INTERVAL_JUDGE_WINDOW = "窓 B（起動過渡を除外し、さらに推定活性アニメ本数で絞った窓）"

#: 判定式⑴の判定粒度（design.md 較正値台帳・合格判定式⑴）。target_id へ畳んだ 1 本の p95で
#: 判定してはならない——遅い系列がまるごと速い系列の 5% の裾へ吸収されるため。
FRAME_INTERVAL_JUDGE_GRANULARITY = (
    "系列ごと（target_id x surface_id）に判定し、1 系列でも上限超過なら不合格。"
    "同一 target の系列をまとめた p95 は参考表示にとどめる"
    "（速い系列の 5% 裾に遅い系列が丸ごと吸収されるため）"
)

#: 判定式⑴の対象系列を明示指定する場合の一覧（(target_id, surface_id) の並び）。
#: **空なら下の 2 つの閾値で自動選別する**。task 3.1 の第 1 段ベースラインで実測を見て確定する。
FRAME_INTERVAL_JUDGED_SERIES: tuple[tuple[str, str], ...] = ()

#: 自動選別: 判定に足る間隔の本数の下限。これ未満の系列は判定対象から外す（p95 が立たない）。
FRAME_INTERVAL_MIN_SAMPLES = 30

#: 自動選別: 平均間隔が「期待間隔 x この倍率」を超える系列は、アニメのコマ送りではなく
#: 散発的な適用（バルーンの書き換え等）とみなして判定対象から外す。
FRAME_INTERVAL_MAX_MEAN_FACTOR = 10.0

# --- 自動選別で外した系列を合格に読み替えない規則（D7）------------------------
#
# 【この判定の穴と、その塞ぎ方】
# 判定式⑴の計算からまるごと消える系列は 3 通りある。しかも消える条件は、まさに本 spec が
# 直そうとしている「鈍化」そのものである：
#   * 鈍化したアニメは同じ走行時間で出すコマ数が **減る** ため、まず MIN_SAMPLES で落ちる
#     （こちらのほうが先に効く。20 秒/コマなら 7 分走っても 20 本前後にしかならない）
#   * さらに鈍ければ平均間隔が MAX_MEAN_FACTOR 倍を超えて落ちる
#   * **判定窓（活性アニメ本数の上限）で間隔が 1 本も残らなかった系列**は、そもそも間隔の表に
#     鍵が立たない。上の 2 つの閾値にすら触れないので、放っておくと判定対象にも除外一覧にも
#     現れず、誰にも数えられないまま「残った速い系列が上限内＝合格」に化ける。
#     窓内の適用が 1 回以下の系列も同じ経路で消える（適用 2 回なら MIN_SAMPLES で捕まるのに、
#     適用 1 回の **もっと遅い** 系列はすり抜ける、という倒錯が生じる）。
#     この 3 つ目は `vanished_series()` が perf レコードから数え直して除外へ差し戻す。
# ゆえに「速い系列だけが残って合格」という読み替えを **禁止** する。
#
# 規則: FRAME_INTERVAL_JUDGED_SERIES が空（＝自動選別が効いている）あいだは、
#       1 つでも系列を外したら判定式⑴は 合格 を返さない（判定不能にする）。
#       ただし上限超過が 1 つでもあれば、除外の有無によらず **不合格が優先** する
#       （確定的な違反を判定不能に埋もれさせない）。
# 解除: task 3.1 の第 1 段ベースラインが FRAME_INTERVAL_JUDGED_SERIES を実測から確定して
#       明示指定に切り替えれば、除外は「意図した対象外」になるので合格が再び意味を持つ。
#       ただし **明示指定した系列自身が判定窓から消えていた** 場合だけは別で、明示指定でも
#       合格を返さない（対象と決めた系列を測れていないのだから合格ではない）。
FRAME_INTERVAL_EXCLUSION_BLOCKS_PASS = (
    "自動選別（FRAME_INTERVAL_JUDGED_SERIES が空）のあいだは、除外した系列が 1 つでもあれば"
    "判定式⑴は合格を返さない（判定不能）。除外には、閾値で外した系列だけでなく"
    "**判定窓に間隔が 1 本も残らなかった系列**（窓で全部落ちた／窓内の適用が 1 回以下）も含める"
    "（数えられずに消える経路を残さない）。不合格は除外の有無によらず優先する。"
    "task 3.1 が対象系列を明示指定すれば除外は意図的になり、合格が再び意味を持つ。"
    "ただし明示指定した系列自身が判定窓から消えていれば、明示指定でも合格は返さない。"
)

#: 判定式⑴の比較と表示を丸める桁数（ミリ秒の小数点以下）。
#: 【なぜ要るか】上限は FRAME_INTERVAL_EXPECTED_MS x (1 + FRAME_INTERVAL_TOLERANCE) の
#: 浮動小数点の積であり、172.0 x 1.15 は 197.79999999999998 になる。丸めずに比べると
#: 実測 p95 が 197.8ms ちょうどの走行で「p95 197.8ms > 上限 197.8ms（超過）」という、
#: 表示と判定が食い違う根拠が出る（要件 4.5 は比べた閾値の明記を求めている）。
#: run.log の時刻はマイクロ秒分解能なので、間隔として意味を持つ桁は 0.001ms までである。
#: ゆえに実測 p95 と上限の **双方** をこの桁へ丸めてから比べ、同じ桁で表示する。
#: 丸めが動かすのは 0.0005ms 未満の差だけであり、判定の意味（⑴は ≦）は変えない。
FRAME_INTERVAL_COMPARE_DECIMALS = 3

# --- 定常状態の開始境界 -------------------------------------------------------

#: 起動過渡（初回確保・ウォームアップ）として集計から外す秒数。
#: 実走の最初の観測時刻を 0 秒として数える。暫定値であり task 3.1 で確定する。
WARMUP_EXCLUDE_SEC = 60.0

#: 判定モード限定: 定常状態がこの回数未満の適用しか含まなければ判定式⑵⑶を判定不能にする。
#: 【由来】定常状態が空だと⑵（catch-up 0 件）と⑶（確保 0 件）は「観測が無いから 0」で
#: 恒真になり、沈黙が合格として読まれる（2.2→2.3 申し送り）。空だけでなく、実質的に空と
#: 変わらない薄さも同じ穴なので、下限を明示の較正値として置く。
VERDICT_MIN_STEADY_APPLIES = 30

#: 判定モード限定: 定常状態の時間幅の下限（秒）。上と同じ理由。
VERDICT_MIN_STEADY_SEC = 60.0

# --- CPU（要件 4.2⑷ / 5.1） --------------------------------------------------

#: release ビルドのアイドル CPU 上限（1 コア換算パーセント）。
#: 議題 2 裁定（同一ゴーストを表示中の SSP 実測 2.2〜2.8% の同等圏）。
#: 判定は「< 3.0%」の **狭義** の比較である（要件 4.4 の文言が「3% 未満」であるため）。
IDLE_CPU_MAX_RELEASE_PCT = 3.0

#: 判定式⑷の前半が閾値と比べる統計量。定常状態（ウォームアップ除外後）の平均を使う。
#: 【なぜ平均か】要件 4.4 の較正の出所である SSP 実測 2.2〜2.8% は「定常値」として採られており、
#: 分位点ではない。同じ土俵に載せるため平均を判定に使い、p50/p95/最大は根拠として併記する。
IDLE_CPU_STATISTIC = "定常状態（ウォームアップ除外後）の平均（1 コア換算 %）"

#: 判定式⑷の前半に必要な CPU 採取点の下限（定常状態内）。これ未満なら判定不能。
VERDICT_MIN_CPU_SAMPLES = 5

#: 判定式⑷の後半「20 分超で単調上昇しない」が成立を主張できる走行長（秒）。
#: 定常状態の CPU 時系列の時間幅がこれ未満の走行では、後半は **適用外** として扱い、
#: 終了コードに寄与させない（design.md Flow 1: 長時間水準はベースラインと最終判定の 2 回のみ・
#: 中間の是正ループは短時間水準。task 7.1 は短時間水準で⑴〜⑶を判定する）。
#: ただし run-meta.txt の profile が long の走行でこれに届かない場合は、途中で切れた長時間走行と
#: みなして **判定不能** にする（短いのに「長時間の性質を確かめた」と読ませないため）。
LONG_RUN_MIN_SPAN_SEC = 1200.0

#: CPU 時系列の想定刻み（秒）。`invoke-perf-run.ps1` の CPU_SAMPLE_INTERVAL_SEC と対。
#: 【重要】ランナーは次回時刻を絶対時刻で積み上げるためドリフトしないが、マシンが詰まって
#: 刻みを飛ばした場合は印を残さずに欠測する。ゆえに本スクリプトは刻みを均等と仮定せず、
#: 行ごとの timestamp の差から実際の間隔を出す。この値は「欠測らしさ」の報告にのみ使う。
CPU_SAMPLE_INTERVAL_EXPECTED_SEC = 15.0

#: 実測間隔がこの倍率を超えたら「刻みを飛ばした疑い」として件数を報告する。
CPU_SAMPLE_GAP_FACTOR = 1.5

#: 収束判定（要件 5.1・5.3）。時系列を等分する窓の数。
CONVERGENCE_WINDOW_COUNT = 4

#: 収束判定: 後半窓の回帰傾きの上限（パーセント毎分）。task 3.1 で確定する暫定値。
CONVERGENCE_SLOPE_MAX_PCT_PER_MIN = 0.05

#: 収束判定: 末尾窓平均 − 中間窓平均 の上限（パーセント）。task 3.1 で確定する暫定値。
CONVERGENCE_WINDOW_DIFF_MAX_PCT = 0.3

#: 収束判定: 窓の切り方。**採取点の個数**で等分する（時間で等分しない）。
#: 刻みを飛ばした欠測があっても窓ごとの点数が偏らない。集計モードの窓別平均と同一の切り方。
CONVERGENCE_WINDOW_SPLIT = "定常状態の採取点を個数で等分（時間等分ではない）"

#: 収束判定: 回帰傾きを取る窓の範囲（1 始まり）。「後半窓」＝後ろ半分の窓。
CONVERGENCE_SLOPE_WINDOW_FROM = CONVERGENCE_WINDOW_COUNT // 2 + 1

#: 収束判定: 「中間窓」の番号（1 始まり）。末尾窓との間に十分な時間差を取るため後ろ半分の直前を採る。
CONVERGENCE_MIDDLE_WINDOW_INDEX = CONVERGENCE_WINDOW_COUNT // 2

#: 収束判定: 傾きの求め方。
CONVERGENCE_SLOPE_METHOD = "最小二乗（x=経過分・y=1 コア換算 %）"

#: 収束判定: 1 窓あたりに必要な採取点の下限。全体でこれ x 窓数に満たなければ収束判定は判定不能。
CONVERGENCE_MIN_SAMPLES_PER_WINDOW = 3

# --- 分位点の求め方 -----------------------------------------------------------

#: p50/p95 の定義。nearest-rank（昇順に並べ、ceil(q * n) 番目の値をそのまま採る）。
#: 補間しないので、同じ入力からは必ず同じ値が出る（道具の再現性）。
PERCENTILE_METHOD = "nearest-rank（補間なし・ceil(q*n) 番目）"

# --- ログの文言（J_*）--------------------------------------------------------

#: perf サマリ行の固定文言（`presenter/timing.rs:53,206`）。
J_PERF_LINE_MESSAGE = "perf(apply_show): 段階別計時"

#: 表示成立点 info! の固定文言（`presenter/show.rs:359`）。
J_SHOW_LINE_MESSAGE = "apply(ShowSurface): 表示・マスクを更新"

#: 進行境界スキップ（catch-up）の実文言。
#: `ticker.rs:205`（dispatcher）／`ticker.rs:225`（kanade）が前者、
#: `ticker.rs:307`（loop_ticker）が後者。
#: 【重要な落とし穴】後者は前者を部分文字列として **含む**。素朴に「含む行を数える」と
#: loop_ticker の 1 行が両方に数えられて二重計上になる。ゆえに必ず長いほうから先に判定する。
J_CATCHUP_LOOP_MESSAGE = "loop ticker catch-up: skipped multiple boundaries, firing once"
J_CATCHUP_TICKER_MESSAGE = "ticker catch-up: skipped multiple boundaries, firing once"

#: seriko のループ発火・停止 info!（`areka-seriko/src/looper.rs`）。
#: 活性アニメ本数の直接ログは無いため、この収支から推定する（design.md「収束判定」）。
J_SERIKO_FIRE_MESSAGE = "seriko: loop 抽選発火"
J_SERIKO_STOP_MESSAGES = (
    "seriko: loop 末尾残留",  # looper.rs:342 最終コマ保持・再抽選対象へ
    "seriko: loop 停止",  # looper.rs:372 負 surface でベース復帰
    "seriko: loop bind から外れた ID の再生を停止",  # looper.rs:308
)

#: 必要ログ種の一覧（欠けたら部分集計を出さずに exit 2・要件 2.5）。
#: `run.stderr.log` は必要ログ種では **ない**。標準出力と標準エラーは同一ファイルへ流せない
#: ためランナーが別ファイルへ分けているだけであり、欠落扱いにしてはならない。
J_REQUIRED_LOG_KINDS = (
    ("perf サマリ行", J_PERF_LINE_MESSAGE),
    ("表示成立点 info!", J_SHOW_LINE_MESSAGE),
)

#: CPU 時系列 CSV のヘッダ（`invoke-perf-run.ps1` の CSV_HEADER と対）。
J_CPU_CSV_HEADER = ("timestamp", "cpu_percent_1core")

#: perf サマリ行に必ず載るフィールド（design.md「perf サマリ行スキーマ」の 14 個）。
#: 1 つでも欠けた行があれば部分集計を出さずに exit 2（要件 2.5）。
J_PERF_STAGE_FIELDS = (
    "t_cache_us",
    "t_compose_us",
    "t_resample_us",
    "t_mask_us",
    "t_upload_us",
    "t_total_us",
)
J_PERF_ALLOC_FIELDS = (
    "alloc_compose_dst",
    "alloc_resample_dst",
    "alloc_xmap",
    "alloc_mask",
)
J_PERF_IDENT_FIELDS = ("target_id", "surface_id", "cache_hit", "key_hash")
J_PERF_REQUIRED_FIELDS = J_PERF_IDENT_FIELDS + J_PERF_STAGE_FIELDS + J_PERF_ALLOC_FIELDS

#: マシン条件に必ず載せる DPI 構成の出どころ（2.1→2.2 申し送り 2）。
#: run-meta.txt に DPI は無い。run.log の起動時ログ（`areka::placement`）と表示成立点 info!
#: から拾う。合成コストは DPI に比例し、ベースラインは 2 モニタ混在 DPI で採取されるため、
#: DPI を書かないレポートは再現できない。
J_DPI_BOOT_FIELDS = (
    "primary_dpi",
    "shell_author_dpi",
    "balloon_author_dpi",
    "k_shell",
    "k_balloon",
)
J_DPI_SHOW_FIELDS = (
    "author_dpi",
    "window_dpi",
    "k",
    "k_ratio",
    "native_w",
    "native_h",
    "scaled_w",
    "scaled_h",
)

# =============================================================================
# 終了コードと失敗の伝え方
# =============================================================================

EXIT_OK = 0
EXIT_FAIL = 1  # 合否判定モード専用。集計モードは返さない
EXIT_INCONCLUSIVE = 2
EXIT_BAD_INPUT = 3

# バナーの SELFTEST_REQUIRED_RED_CODES は定数定義より前にあるため数値で書いてある。
# ここで取り違えが無いことを確かめる（赤の関門が別のコードを見張っていたら意味が無い）。
assert SELFTEST_REQUIRED_RED_CODES == (EXIT_FAIL, EXIT_INCONCLUSIVE)


class JudgeError(Exception):
    """理由と終了コードを必ず持つ失敗（黙って落ちる経路を作らない）。"""

    def __init__(self, code: int, message: str, details: list[str] | None = None):
        super().__init__(message)
        self.code = code
        self.message = message
        self.details = details or []


def inconclusive(message: str, details: list[str] | None = None) -> JudgeError:
    """判定不能（必要ログ種の欠落・段フィールドの欠落・観測ゼロ）。"""
    return JudgeError(EXIT_INCONCLUSIVE, message, details)


def bad_input(message: str, details: list[str] | None = None) -> JudgeError:
    """引数不正・ファイル読取不能。"""
    return JudgeError(EXIT_BAD_INPUT, message, details)


# =============================================================================
# ログ行の解析
# =============================================================================

#: ANSI エスケープ（CSI シーケンス）。読み込み時に落とす。
_ANSI_RE = re.compile(r"\x1b\[[0-9;?]*[ -/]*[@-~]")

#: 行頭の時刻。`tracing-subscriber` 既定の SystemTime タイマは RFC3339（UTC・小数 6 桁）。
_TIMESTAMP_RE = re.compile(r"^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?)Z\s")

#: `名前=` の出現位置。値は次の `名前=` の直前までとする。
#: `window_dpi=Some((120, 120))` のように値に空白が入る Debug 出力があるため、
#: 空白で切ってはいけない。
_FIELD_KEY_RE = re.compile(r"(?:^|\s)([A-Za-z_][A-Za-z0-9_]*)=")


def strip_ansi(text: str) -> str:
    """ANSI エスケープを落とす。"""
    return _ANSI_RE.sub("", text)


def parse_timestamp(line: str) -> datetime | None:
    """行頭の時刻を返す。時刻で始まらない行（複数行メッセージの続き）は None。"""
    m = _TIMESTAMP_RE.match(line)
    if not m:
        return None
    raw = m.group(1)
    if "." in raw:
        head, frac = raw.split(".", 1)
        frac = (frac + "000000")[:6]
        raw = f"{head}.{frac}"
    else:
        raw = raw + ".000000"
    try:
        return datetime.strptime(raw, "%Y-%m-%dT%H:%M:%S.%f").replace(tzinfo=timezone.utc)
    except ValueError:
        return None


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
    if idx < 0:
        return None
    return line[idx + len(message):]


def read_text_lines(path: Path, what: str) -> list[str]:
    """テキストを行に割って返す。ANSI を落とし、行末の CR も落とす。

    読めないファイルは exit 3（引数不正・読取不能）。文字化けは行を捨てずに置換で通す——
    数値もフィールド名も ASCII であり、化けるのは日本語のメッセージ部分だけである
    （途中で切れた最終行が壊れていても、それ以前の観測を捨てない）。
    """
    try:
        raw = path.read_bytes()
    except OSError as exc:
        raise bad_input(f"{what} を読めません: {path}", [str(exc)]) from exc
    text = raw.decode("utf-8", errors="replace")
    if text.startswith("﻿"):
        text = text[1:]
    return [strip_ansi(line.rstrip("\r")) for line in text.split("\n")]


# =============================================================================
# perf サマリ行
# =============================================================================


@dataclass
class PerfRecord:
    """perf サマリ行 1 本（＝表示 1 コマの適用 1 回）。"""

    line_no: int
    at: datetime
    target_id: str
    surface_id: str
    cache_hit: bool
    key_hash: str
    stages: dict[str, int]
    allocs: dict[str, int]

    @property
    def series_key(self) -> tuple[str, str]:
        return (self.target_id, self.surface_id)


@dataclass
class LogScan:
    """run.log を 1 度なめて集めたもの。"""

    perf: list[PerfRecord] = field(default_factory=list)
    show_count: int = 0
    first_at: datetime | None = None
    last_at: datetime | None = None
    catchup: list[tuple[datetime | None, str, str]] = field(default_factory=list)
    seriko_events: list[tuple[datetime | None, str, str, str]] = field(default_factory=list)
    boot_dpi: dict[str, str] = field(default_factory=dict)
    show_dpi_variants: Counter = field(default_factory=Counter)
    idle_exclusion_hits: int = 0


def _catchup_kind(line: str) -> str | None:
    """catch-up 行の種別。長いほう（loop）から順に見る（部分文字列の罠）。"""
    if J_CATCHUP_LOOP_MESSAGE in line:
        return "loop_ticker"
    if J_CATCHUP_TICKER_MESSAGE in line:
        return "ticker"
    return None


def _seriko_kind(line: str) -> str | None:
    if J_SERIKO_FIRE_MESSAGE in line:
        return "fire"
    for stop in J_SERIKO_STOP_MESSAGES:
        if stop in line:
            return "stop"
    return None


def scan_log(path: Path) -> LogScan:
    """run.log を 1 度だけなめて必要なものを全部集める。"""
    lines = read_text_lines(path, "実走ログ（run.log）")
    scan = LogScan()
    bad_lines: list[str] = []

    for i, line in enumerate(lines, start=1):
        if not line.strip():
            continue
        at = parse_timestamp(line)
        if at is not None:
            if scan.first_at is None:
                scan.first_at = at
            scan.last_at = at

        if J_PERF_LINE_MESSAGE in line:
            record = _parse_perf_line(i, line, at, bad_lines)
            if record is not None:
                scan.perf.append(record)
            continue

        if J_SHOW_LINE_MESSAGE in line:
            scan.show_count += 1
            fields = parse_fields(split_after_message(line, J_SHOW_LINE_MESSAGE) or "")
            variant = tuple(
                f"{name}={fields.get(name, '(無し)')}" for name in J_DPI_SHOW_FIELDS
            )
            scan.show_dpi_variants[variant] += 1
            continue

        kind = _catchup_kind(line)
        if kind is not None:
            fields = parse_fields(line)
            scan.catchup.append((at, kind, fields.get("target", "(target 欄なし)")))
            continue

        skind = _seriko_kind(line)
        if skind is not None:
            fields = parse_fields(line)
            scan.seriko_events.append(
                (at, skind, fields.get("scope", "?"), fields.get("animation_id", "?"))
            )
            continue

        if any(marker in line for marker in FRAME_INTERVAL_IDLE_EXCLUDE_MARKERS):
            scan.idle_exclusion_hits += 1
            continue

        if not scan.boot_dpi and "primary_dpi=" in line:
            fields = parse_fields(line)
            for name in J_DPI_BOOT_FIELDS:
                if name in fields:
                    scan.boot_dpi[name] = fields[name]

    if bad_lines:
        raise inconclusive(
            "perf サマリ行に必要なフィールドが揃っていません。"
            f"欠落のある行が {len(bad_lines)} 本あります。部分集計は出しません（要件 2.5）。",
            bad_lines[:20]
            + ([f"…ほか {len(bad_lines) - 20} 本"] if len(bad_lines) > 20 else []),
        )
    return scan


def _parse_perf_line(
    line_no: int, line: str, at: datetime | None, bad_lines: list[str]
) -> PerfRecord | None:
    """perf サマリ行 1 本を読む。欠落・値異常は bad_lines へ積んで None を返す。"""
    tail = split_after_message(line, J_PERF_LINE_MESSAGE)
    fields = parse_fields(tail or "")

    missing = [name for name in J_PERF_REQUIRED_FIELDS if name not in fields]
    if missing:
        bad_lines.append(f"{line_no} 行目: 欠落フィールド {', '.join(missing)}")
        return None

    if at is None:
        bad_lines.append(f"{line_no} 行目: 行頭の時刻を読めません（コマ適用間隔が出せません）")
        return None

    numbers: dict[str, int] = {}
    for name in J_PERF_STAGE_FIELDS + J_PERF_ALLOC_FIELDS:
        try:
            numbers[name] = int(fields[name])
        except ValueError:
            bad_lines.append(
                f"{line_no} 行目: {name} が整数ではありません（値 {fields[name]!r}）"
            )
            return None

    raw_hit = fields["cache_hit"]
    if raw_hit not in ("true", "false"):
        bad_lines.append(f"{line_no} 行目: cache_hit が true/false ではありません（値 {raw_hit!r}）")
        return None

    return PerfRecord(
        line_no=line_no,
        at=at,
        target_id=fields["target_id"],
        surface_id=fields["surface_id"],
        cache_hit=(raw_hit == "true"),
        key_hash=fields["key_hash"],
        stages={name: numbers[name] for name in J_PERF_STAGE_FIELDS},
        allocs={name: numbers[name] for name in J_PERF_ALLOC_FIELDS},
    )


# =============================================================================
# CPU 時系列 CSV
# =============================================================================


@dataclass
class CpuSample:
    at: datetime
    percent: float


def read_cpu_csv(path: Path) -> list[CpuSample]:
    lines = read_text_lines(path, "CPU 時系列（cpu.csv）")
    rows = [row for row in csv.reader(lines) if row]
    if not rows:
        raise inconclusive(f"CPU 時系列 CSV が空です: {path}")

    header = tuple(cell.strip() for cell in rows[0])
    if header != J_CPU_CSV_HEADER:
        raise inconclusive(
            "CPU 時系列 CSV のヘッダが契約と違います（採取スクリプトの版ずれの疑い）。",
            [f"期待: {','.join(J_CPU_CSV_HEADER)}", f"実際: {','.join(header)}"],
        )

    samples: list[CpuSample] = []
    broken: list[str] = []
    for line_no, row in enumerate(rows[1:], start=2):
        if len(row) < 2:
            broken.append(f"{line_no} 行目: 列が足りません（{row!r}）")
            continue
        at = parse_timestamp(row[0].strip() + " ")
        if at is None:
            broken.append(f"{line_no} 行目: 時刻を読めません（{row[0]!r}）")
            continue
        try:
            percent = float(row[1].strip())
        except ValueError:
            broken.append(f"{line_no} 行目: CPU 値が数値ではありません（{row[1]!r}）")
            continue
        samples.append(CpuSample(at=at, percent=percent))

    if broken:
        raise inconclusive(
            f"CPU 時系列 CSV に読めない行が {len(broken)} 本あります。部分集計は出しません（要件 2.5）。",
            broken[:20],
        )
    if not samples:
        raise inconclusive(f"CPU 時系列 CSV に観測が 1 点もありません: {path}")
    return samples


# =============================================================================
# run-meta.txt（マシン条件・要件 4.5）
# =============================================================================


def read_run_meta(path: Path) -> list[tuple[str, list[tuple[str, str]]]]:
    """`[見出し]` と `名前 = 値` の並びを、順序を保ったまま読む。"""
    lines = read_text_lines(path, "実行条件（run-meta.txt）")
    sections: list[tuple[str, list[tuple[str, str]]]] = []
    current: list[tuple[str, str]] = []
    title = "(見出しなし)"
    for line in lines:
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if stripped.startswith("[") and stripped.endswith("]"):
            if current:
                sections.append((title, current))
            title = stripped[1:-1]
            current = []
            continue
        if "=" in stripped:
            name, _, value = stripped.partition("=")
            current.append((name.strip(), value.strip()))
    if current:
        sections.append((title, current))
    if not sections:
        raise inconclusive(
            f"実行条件（run-meta.txt）から測定マシンの条件を読み取れません: {path}",
            ["要件 4.5 はマシン条件の併記を求めており、条件なしの集計は出しません。"],
        )
    return sections


def meta_lookup(sections, name: str) -> str | None:
    for _title, items in sections:
        for key, value in items:
            if key == name:
                return value
    return None


# =============================================================================
# 統計
# =============================================================================


def percentile(values: list[float], q: float) -> float:
    """nearest-rank の分位点（PERCENTILE_METHOD）。空列では呼ばない。"""
    ordered = sorted(values)
    rank = max(1, math.ceil(q * len(ordered)))
    return ordered[min(rank, len(ordered)) - 1]


def describe(values: list[float]) -> dict[str, float]:
    return {
        "n": float(len(values)),
        "min": min(values),
        "p50": percentile(values, 0.50),
        "p95": percentile(values, 0.95),
        "max": max(values),
        "mean": sum(values) / len(values),
    }


def split_windows(items: list, count: int) -> list[list]:
    """並びを個数で等分する（CONVERGENCE_WINDOW_SPLIT）。集計モードと判定モードで同一の切り方。"""
    if count <= 0 or not items:
        return []
    size = len(items) / count
    return [items[int(w * size): int((w + 1) * size)] for w in range(count)]


def regression_slope_per_min(samples: list["CpuSample"]) -> float | None:
    """最小二乗の傾き（パーセント毎分）。x が 1 点に潰れている場合は None。"""
    if len(samples) < 2:
        return None
    base = samples[0].at
    xs = [(s.at - base).total_seconds() / 60.0 for s in samples]
    ys = [s.percent for s in samples]
    n = len(xs)
    mean_x = sum(xs) / n
    mean_y = sum(ys) / n
    sxx = sum((x - mean_x) ** 2 for x in xs)
    if sxx == 0.0:
        return None
    sxy = sum((x - mean_x) * (y - mean_y) for x, y in zip(xs, ys))
    return sxy / sxx


def fmt_stats(values: list[float], unit: str = "", decimals: int = 1) -> str:
    """要約統計の 1 行表示。

    `decimals` は表示桁数。既定の 1 桁は集計モードの一覧向けである。判定式の根拠行のように
    同じ行の中で閾値と実測を突き合わせる場合は、比較に使う桁（判定式⑴なら
    FRAME_INTERVAL_COMPARE_DECIMALS）を渡して行全体の桁を揃えること。桁が混ざると
    「p95 197.8ms … → p95 197.802ms vs 上限 197.800ms（超過）」のように、同じ行の中で
    同じ値が違う数字に見える（要件 4.5 が求める「比べた閾値の明記」を濁らせる）。
    """
    if not values:
        return "観測なし"
    s = describe(values)
    d = decimals
    return (
        f"n={int(s['n'])}  最小={s['min']:.{d}f}{unit}  p50={s['p50']:.{d}f}{unit}  "
        f"p95={s['p95']:.{d}f}{unit}  最大={s['max']:.{d}f}{unit}  平均={s['mean']:.{d}f}{unit}"
    )


# =============================================================================
# 活性アニメ本数の推定（idle 窓の根拠）
# =============================================================================


def active_animation_timeline(scan: LogScan) -> list[tuple[datetime, int]]:
    """seriko の発火・停止 info! の収支から、時刻ごとの活性アニメ本数を推定する。

    活性集合サイズの直接ログは存在しないため、これは **間接推定** である
    （design.md「収束判定」の但し書きと同じ性質）。停止が発火より先に現れた場合は
    負にせず 0 で止める（起動前から走っていたぶんは観測できない）。
    """
    active: set[tuple[str, str]] = set()
    timeline: list[tuple[datetime, int]] = []
    for at, kind, scope, anim in sorted(
        (e for e in scan.seriko_events if e[0] is not None), key=lambda e: e[0]
    ):
        key = (scope, anim)
        if kind == "fire":
            active.add(key)
        else:
            active.discard(key)
        timeline.append((at, len(active)))
    return timeline


def active_count_at(timeline: list[tuple[datetime, int]], at: datetime) -> int:
    """`at` の時点で走っていたと推定されるアニメ本数（直前の収支）。"""
    lo, hi = 0, len(timeline)
    while lo < hi:
        mid = (lo + hi) // 2
        if timeline[mid][0] <= at:
            lo = mid + 1
        else:
            hi = mid
    return timeline[lo - 1][1] if lo > 0 else 0


# =============================================================================
# 集計
# =============================================================================


@dataclass
class Aggregation:
    scan: LogScan
    cpu: list[CpuSample]
    steady_from: datetime
    steady_perf: list[PerfRecord]
    intervals_warmup: dict[tuple[str, str], list[float]]
    intervals_idle: dict[tuple[str, str], list[float]]


def frame_intervals(
    records: list[PerfRecord],
    timeline: list[tuple[datetime, int]],
    idle_only: bool,
) -> dict[tuple[str, str], list[float]]:
    """コマ適用間隔（ミリ秒）を系列（target_id x surface_id）ごとに出す。

    系列を分けるのは、別対象への適用が同じ時間帯に重なると、混ぜた列の差分が「間隔」で
    なくなるためである（FRAME_INTERVAL_SERIES_KEY）。`idle_only` のときは、推定活性アニメ
    本数が FRAME_INTERVAL_IDLE_MAX_ACTIVE を超える区間の適用を落とす。**両端が残った隣接**
    だけを間隔として数える（落とした点を跨いだ差分は間隔ではない）。
    """
    by_series: dict[tuple[str, str], list[float]] = defaultdict(list)
    grouped: dict[tuple[str, str], list[PerfRecord]] = defaultdict(list)
    for record in records:
        grouped[record.series_key].append(record)

    for key, group in grouped.items():
        group.sort(key=lambda r: r.at)
        previous: PerfRecord | None = None
        for record in group:
            keep = True
            if idle_only:
                keep = active_count_at(timeline, record.at) <= FRAME_INTERVAL_IDLE_MAX_ACTIVE
            if not keep:
                previous = None
                continue
            if previous is not None:
                by_series[key].append(
                    (record.at - previous.at).total_seconds() * 1000.0
                )
            previous = record
    return by_series


def aggregate(scan: LogScan, cpu: list[CpuSample]) -> Aggregation:
    base = scan.perf[0].at
    steady_from = base + timedelta(seconds=WARMUP_EXCLUDE_SEC)
    steady_perf = [r for r in scan.perf if r.at >= steady_from]
    timeline = active_animation_timeline(scan)
    return Aggregation(
        scan=scan,
        cpu=cpu,
        steady_from=steady_from,
        steady_perf=steady_perf,
        intervals_warmup=frame_intervals(steady_perf, timeline, idle_only=False),
        intervals_idle=frame_intervals(steady_perf, timeline, idle_only=True),
    )


# =============================================================================
# レポート
# =============================================================================


class Report:
    def __init__(self) -> None:
        self.lines: list[str] = []

    def head(self, title: str) -> None:
        self.lines.append("")
        self.lines.append("=" * 78)
        self.lines.append(title)
        self.lines.append("=" * 78)

    def sub(self, title: str) -> None:
        self.lines.append("")
        self.lines.append(f"-- {title} " + "-" * max(0, 74 - len(title)))

    def line(self, text: str = "") -> None:
        self.lines.append(text)

    def render(self) -> str:
        return "\n".join(self.lines)


def render_machine_conditions(report: Report, sections, scan: LogScan) -> None:
    """要件 4.5: 測定マシンの条件をレポート先頭に併記する。"""
    report.head("[1] 測定マシンの条件（要件 4.5）")
    for title, items in sections:
        report.sub(f"run-meta.txt [{title}]")
        for name, value in items:
            report.line(f"  {name} = {value}")

    report.sub("DPI 構成（run.log 由来・run-meta.txt には無い）")
    report.line(
        "  合成コストは DPI に比例し、ベースラインは 2 モニタ混在 DPI で採取される。"
    )
    report.line("  DPI を書かないレポートは同一条件で再測できない。")
    if scan.boot_dpi:
        for name in J_DPI_BOOT_FIELDS:
            if name in scan.boot_dpi:
                report.line(f"  {name} = {scan.boot_dpi[name]}")
    else:
        report.line("  起動時 DPI ログ（primary_dpi を含む行）は run.log にありません。")
    if scan.show_dpi_variants:
        report.line("  表示成立点 info! に現れた DPI/寸法の組み合わせ（出現数の多い順）:")
        for variant, count in scan.show_dpi_variants.most_common():
            report.line(f"    [{count} 回] " + "  ".join(variant))
    else:
        report.line("  表示成立点 info! からは DPI を拾えませんでした。")


def render_calibration(report: Report) -> None:
    """要件 2.6 / 5.3: 較正値をレポートに明記する（本文が使う値はすべてバナー由来）。"""
    report.head("[2] 較正値（要件 2.6 / 4.5 / 5.3・所在はスクリプト冒頭のバナー 1 箇所）")
    rows = [
        ("SCRIPT_VERSION", SCRIPT_VERSION, ""),
        ("FRAME_INTERVAL_EXPECTED_MS", FRAME_INTERVAL_EXPECTED_MS, "emo2 固有・流用禁止"),
        ("FRAME_INTERVAL_TOLERANCE", FRAME_INTERVAL_TOLERANCE, "暫定・task 3.1 で確定"),
        ("FRAME_INTERVAL_SERIES_KEY", " x ".join(FRAME_INTERVAL_SERIES_KEY), ""),
        ("FRAME_INTERVAL_WINDOW", FRAME_INTERVAL_WINDOW, ""),
        ("FRAME_INTERVAL_JUDGE_WINDOW", FRAME_INTERVAL_JUDGE_WINDOW, "判定式⑴が使う窓"),
        (
            "FRAME_INTERVAL_JUDGE_GRANULARITY",
            FRAME_INTERVAL_JUDGE_GRANULARITY,
            "判定式⑴の判定粒度",
        ),
        (
            "FRAME_INTERVAL_JUDGED_SERIES",
            [f"{t} / surface_id={s}" for t, s in FRAME_INTERVAL_JUDGED_SERIES]
            or "（空＝下の 2 値で自動選別）",
            "task 3.1 で確定",
        ),
        ("FRAME_INTERVAL_MIN_SAMPLES", FRAME_INTERVAL_MIN_SAMPLES, "判定式⑴の自動選別・暫定"),
        (
            "FRAME_INTERVAL_MAX_MEAN_FACTOR",
            FRAME_INTERVAL_MAX_MEAN_FACTOR,
            "判定式⑴の自動選別・暫定",
        ),
        (
            "FRAME_INTERVAL_EXCLUSION_BLOCKS_PASS",
            FRAME_INTERVAL_EXCLUSION_BLOCKS_PASS,
            "自動選別で外した系列を合格に読み替えない規則（D7）",
        ),
        (
            "FRAME_INTERVAL_COMPARE_DECIMALS",
            FRAME_INTERVAL_COMPARE_DECIMALS,
            "判定式⑴の比較・表示を揃える桁（ms 小数点以下）",
        ),
        (
            "FRAME_INTERVAL_IDLE_MAX_ACTIVE",
            FRAME_INTERVAL_IDLE_MAX_ACTIVE,
            "暫定・task 3.1 で確定",
        ),
        (
            "FRAME_INTERVAL_IDLE_EXCLUDE_MARKERS",
            list(FRAME_INTERVAL_IDLE_EXCLUDE_MARKERS) or "（空＝talk 区間を除外できていない）",
            "task 3.1 で確定",
        ),
        ("WARMUP_EXCLUDE_SEC", WARMUP_EXCLUDE_SEC, "暫定・task 3.1 で確定"),
        ("VERDICT_MIN_STEADY_APPLIES", VERDICT_MIN_STEADY_APPLIES, "判定式⑵⑶の成立条件・暫定"),
        ("VERDICT_MIN_STEADY_SEC", VERDICT_MIN_STEADY_SEC, "判定式⑵⑶の成立条件・暫定"),
        ("IDLE_CPU_MAX_RELEASE_PCT", IDLE_CPU_MAX_RELEASE_PCT, "判定式⑷前半・狭義の < 比較"),
        ("IDLE_CPU_STATISTIC", IDLE_CPU_STATISTIC, "判定式⑷前半が閾値と比べる量"),
        ("VERDICT_MIN_CPU_SAMPLES", VERDICT_MIN_CPU_SAMPLES, "判定式⑷前半の成立条件・暫定"),
        ("LONG_RUN_MIN_SPAN_SEC", LONG_RUN_MIN_SPAN_SEC, "判定式⑷後半の適用条件（20 分）"),
        ("CPU_SAMPLE_INTERVAL_EXPECTED_SEC", CPU_SAMPLE_INTERVAL_EXPECTED_SEC, "欠測報告のみに使用"),
        ("CPU_SAMPLE_GAP_FACTOR", CPU_SAMPLE_GAP_FACTOR, ""),
        ("CONVERGENCE_WINDOW_COUNT", CONVERGENCE_WINDOW_COUNT, "収束判定・要件 5.1/5.3"),
        ("CONVERGENCE_WINDOW_SPLIT", CONVERGENCE_WINDOW_SPLIT, ""),
        (
            "CONVERGENCE_SLOPE_WINDOW_FROM",
            f"窓{CONVERGENCE_SLOPE_WINDOW_FROM} 〜 窓{CONVERGENCE_WINDOW_COUNT}（＝後半窓）",
            "",
        ),
        (
            "CONVERGENCE_MIDDLE_WINDOW_INDEX",
            f"窓{CONVERGENCE_MIDDLE_WINDOW_INDEX}（＝中間窓）",
            "",
        ),
        ("CONVERGENCE_SLOPE_METHOD", CONVERGENCE_SLOPE_METHOD, ""),
        (
            "CONVERGENCE_SLOPE_MAX_PCT_PER_MIN",
            CONVERGENCE_SLOPE_MAX_PCT_PER_MIN,
            "収束判定・暫定・task 3.1 で確定",
        ),
        (
            "CONVERGENCE_WINDOW_DIFF_MAX_PCT",
            CONVERGENCE_WINDOW_DIFF_MAX_PCT,
            "収束判定・暫定・task 3.1 で確定",
        ),
        (
            "CONVERGENCE_MIN_SAMPLES_PER_WINDOW",
            CONVERGENCE_MIN_SAMPLES_PER_WINDOW,
            f"全体で {CONVERGENCE_MIN_SAMPLES_PER_WINDOW * CONVERGENCE_WINDOW_COUNT} 点未満なら判定不能",
        ),
        ("PERCENTILE_METHOD", PERCENTILE_METHOD, ""),
        ("J_PERF_LINE_MESSAGE", J_PERF_LINE_MESSAGE, ""),
        ("J_SHOW_LINE_MESSAGE", J_SHOW_LINE_MESSAGE, ""),
        ("J_CATCHUP_TICKER_MESSAGE", J_CATCHUP_TICKER_MESSAGE, "dispatcher / kanade"),
        ("J_CATCHUP_LOOP_MESSAGE", J_CATCHUP_LOOP_MESSAGE, "loop_ticker"),
        ("J_SERIKO_FIRE_MESSAGE", J_SERIKO_FIRE_MESSAGE, ""),
        ("J_SERIKO_STOP_MESSAGES", list(J_SERIKO_STOP_MESSAGES), ""),
        ("J_CPU_CSV_HEADER", ",".join(J_CPU_CSV_HEADER), ""),
        ("J_PERF_IDENT_FIELDS", list(J_PERF_IDENT_FIELDS), ""),
        ("J_PERF_STAGE_FIELDS", list(J_PERF_STAGE_FIELDS), ""),
        ("J_PERF_ALLOC_FIELDS", list(J_PERF_ALLOC_FIELDS), ""),
        (
            "J_PERF_REQUIRED_FIELDS",
            f"{len(J_PERF_REQUIRED_FIELDS)} 個（上の 3 つの和・1 つでも欠ければ exit 2）",
            "",
        ),
        ("J_DPI_BOOT_FIELDS", list(J_DPI_BOOT_FIELDS), "run.log の起動時ログから"),
        ("J_DPI_SHOW_FIELDS", list(J_DPI_SHOW_FIELDS), "表示成立点 info! から"),
        (
            "J_REQUIRED_LOG_KINDS",
            [name for name, _ in J_REQUIRED_LOG_KINDS] + ["CPU 時系列 CSV", "run-meta.txt"],
            "run.stderr.log は必要ログ種ではない",
        ),
    ]
    for name, value, note in rows:
        suffix = f"   ← {note}" if note else ""
        report.line(f"  {name} = {value}{suffix}")
    report.line("")
    report.line("  終了コード（EXIT_CODE_TABLE）:")
    for code, meaning in EXIT_CODE_TABLE:
        report.line(f"    {code} = {meaning}")
    report.line(f"  総合判定の決め方（VERDICT_PRECEDENCE）: {VERDICT_PRECEDENCE}")
    report.line("")
    report.line("  【他ゴーストへ流用しないこと】「emo2 固有」と付した値は計測 fixture である")
    report.line("  ゴースト emo2 のアニメ定義・実寸に由来する。別のゴーストでは測り直すこと。")


def render_stage_times(report: Report, agg: Aggregation, section: str = "5") -> None:
    report.head(f"[{section}] 段階別所要（要件 2.2）")
    report.line(f"  分位点の求め方: {PERCENTILE_METHOD}")
    for label, records in (
        ("全区間", agg.scan.perf),
        (f"定常状態（開始から {WARMUP_EXCLUDE_SEC:.0f} 秒を除外）", agg.steady_perf),
    ):
        report.sub(label)
        if not records:
            report.line("  観測なし")
            continue
        for name in J_PERF_STAGE_FIELDS:
            values = [float(r.stages[name]) for r in records]
            s = describe(values)
            report.line(
                f"  {name:<16} n={int(s['n']):<6} p50={s['p50']:>10.0f}us "
                f"p95={s['p95']:>10.0f}us  最大={s['max']:>10.0f}us  平均={s['mean']:>10.0f}us"
            )
        report.line("")
        report.line("  補足: 段の帯は名前より広い（1.3→3.2 申し送り）。")
        report.line("    t_cache_us  はターゲット照会と k 導出を含む。")
        report.line("    t_mask_us   は cache.insert 全体（マスク生成＋スロット置換＋旧エントリ解放）を含む。")
        report.line("    t_upload_us は初回適用の chain/mount 遅延生成を含み、命中経路では照会以降の全部を含む。")


def render_cache_hits(report: Report, agg: Aggregation, section: str = "6") -> None:
    report.head(f"[{section}] キャッシュ命中率（要件 2.2）")
    for label, records in (
        ("全区間", agg.scan.perf),
        ("定常状態", agg.steady_perf),
    ):
        if not records:
            report.line(f"  {label}: 観測なし")
            continue
        hits = sum(1 for r in records if r.cache_hit)
        report.line(
            f"  {label}: 適用 {len(records)} 回中 命中 {hits} 回 "
            f"（{hits / len(records) * 100:.1f}%・不命中 {len(records) - hits} 回）"
        )
    report.sub("対象別（target_id x surface_id）")
    per: dict[tuple[str, str], list[PerfRecord]] = defaultdict(list)
    for record in agg.scan.perf:
        per[record.series_key].append(record)
    for key, records in sorted(per.items(), key=lambda kv: -len(kv[1])):
        hits = sum(1 for r in records if r.cache_hit)
        report.line(
            f"  {key[0]} / surface_id={key[1]}: 適用 {len(records)} 回・"
            f"命中 {hits} 回（{hits / len(records) * 100:.1f}%）"
        )


def render_intervals(report: Report, agg: Aggregation, section: str = "7") -> None:
    report.head(f"[{section}] コマ適用間隔（要件 2.2・判定式⑴の入力）")
    report.line(f"  系列の分け方: {' x '.join(FRAME_INTERVAL_SERIES_KEY)}")
    report.line(f"  測定窓: {FRAME_INTERVAL_WINDOW}")
    report.line(
        f"  期待間隔 {FRAME_INTERVAL_EXPECTED_MS:.0f}ms・許容率 {FRAME_INTERVAL_TOLERANCE:.0%} "
        f"→ 判定式⑴の上限は {FRAME_INTERVAL_EXPECTED_MS * (1 + FRAME_INTERVAL_TOLERANCE):.1f}ms"
        "（判定そのものは --mode verdict）"
    )
    if not FRAME_INTERVAL_IDLE_EXCLUDE_MARKERS:
        report.line(
            "  【未確定】talk 再生区間を外す目印が未設定です"
            "（FRAME_INTERVAL_IDLE_EXCLUDE_MARKERS が空）。"
        )
        report.line(
            "  現在の ログ水準では talk の開始・終了を名指しできる行が無く、talk 中の適用が"
        )
        report.line("  idle 窓に混ざります。task 3.1 の第 1 段ベースラインで目印を確定すること。")
    else:
        report.line(
            f"  talk 除外の目印に一致した行: {agg.scan.idle_exclusion_hits} 本"
        )

    for label, table in (
        ("窓 A: 起動過渡のみ除外", agg.intervals_warmup),
        (
            f"窓 B: さらに推定活性アニメ {FRAME_INTERVAL_IDLE_MAX_ACTIVE} 本以下へ限定",
            agg.intervals_idle,
        ),
    ):
        report.sub(label)
        if not table:
            report.line("  間隔を出せる連続適用がありません")
            continue
        for key, values in sorted(table.items(), key=lambda kv: -len(kv[1])):
            report.line(f"  {key[0]} / surface_id={key[1]}: {fmt_stats(values, 'ms')}")
        by_target: dict[str, list[float]] = defaultdict(list)
        for (target_id, _surface), values in table.items():
            by_target[target_id].extend(values)
        report.line("")
        report.line("  対象（target_id）別のまとめ:")
        for target_id, values in sorted(by_target.items()):
            report.line(f"    {target_id}: {fmt_stats(values, 'ms')}")


def render_allocs(report: Report, agg: Aggregation, section: str = "8") -> None:
    report.head(f"[{section}] 表示用バッファの新規確保 計数（要件 2.2・判定式⑶の入力）")
    for label, records in (
        ("全区間", agg.scan.perf),
        ("定常状態", agg.steady_perf),
    ):
        report.sub(label)
        if not records:
            report.line("  観測なし")
            continue
        total = 0
        for name in J_PERF_ALLOC_FIELDS:
            value = sum(r.allocs[name] for r in records)
            total += value
            nonzero = sum(1 for r in records if r.allocs[name] > 0)
            report.line(f"  {name:<20} 合計 {value:>8}  （発生した適用 {nonzero} 回）")
        report.line(f"  {'合計':<20} 合計 {total:>8}")


def render_catchup(report: Report, agg: Aggregation, section: str = "9") -> None:
    report.head(f"[{section}] 進行境界スキップ（catch-up）件数（要件 2.2・判定式⑵の入力）")
    report.line("  数え方: 長い文言（loop_ticker）を先に判定してから短い文言を数える。")
    report.line("  短いほうは長いほうの部分文字列であり、素朴に数えると二重計上になる。")
    if not agg.scan.catchup:
        report.line("  0 件（全区間）")
        return
    for label, events in (
        ("全区間", agg.scan.catchup),
        (
            "定常状態",
            [e for e in agg.scan.catchup if e[0] is not None and e[0] >= agg.steady_from],
        ),
    ):
        counts: Counter = Counter()
        for _at, kind, target in events:
            counts[(kind, target)] += 1
        report.sub(label)
        report.line(f"  合計 {len(events)} 件")
        for (kind, target), count in sorted(counts.items()):
            report.line(f"    文言={kind}  target={target}: {count} 件")


def render_cpu(report: Report, agg: Aggregation, section: str = "10") -> None:
    report.head(f"[{section}] CPU 時系列（要件 2.2 / 2.3・判定式⑷と収束判定の入力）")
    samples = agg.cpu
    first, last = samples[0].at, samples[-1].at
    span_sec = (last - first).total_seconds()
    report.line(f"  採取点 {len(samples)} 点・{first.isoformat()} 〜 {last.isoformat()}（{span_sec:.0f} 秒）")
    report.line("  単位: 1 コアを 100% とする換算値（複数コア使用時は 100 超もあり得る）。")
    report.line("  1 行は約 1 秒幅の瞬時値であり、刻み幅の平均ではない。")

    report.sub("刻み（均等と仮定しない・2.1→2.2 申し送り 1）")
    gaps = [
        (samples[i + 1].at - samples[i].at).total_seconds() for i in range(len(samples) - 1)
    ]
    if not gaps:
        report.line("  採取点が 1 点しかないため刻みを出せません")
    else:
        report.line(f"  実測の刻み（秒）: {fmt_stats(gaps, 's')}")
        limit = CPU_SAMPLE_INTERVAL_EXPECTED_SEC * CPU_SAMPLE_GAP_FACTOR
        skipped = [g for g in gaps if g > limit]
        report.line(
            f"  期待刻み {CPU_SAMPLE_INTERVAL_EXPECTED_SEC:.0f}s x {CPU_SAMPLE_GAP_FACTOR} "
            f"= {limit:.1f}s を超えた刻み: {len(skipped)} 箇所"
            + (f"（最大 {max(skipped):.1f}s）" if skipped else "")
        )
        report.line("  ランナーは絶対時刻へ積み上げるためドリフトしないが、")
        report.line("  マシンが詰まったときの欠測は印を残さない。上の件数がその痕跡である。")

    for label, subset in (
        ("全区間", samples),
        ("定常状態", [s for s in samples if s.at >= agg.steady_from]),
    ):
        report.sub(label)
        if not subset:
            report.line("  観測なし")
            continue
        report.line(f"  CPU: {fmt_stats([s.percent for s in subset], '%')}")

    report.sub(f"窓別平均（{CONVERGENCE_WINDOW_COUNT} 等分・収束判定の入力）")
    steady = [s for s in samples if s.at >= agg.steady_from]
    if len(steady) < CONVERGENCE_WINDOW_COUNT:
        report.line(
            f"  定常状態の採取点が {len(steady)} 点しかなく "
            f"{CONVERGENCE_WINDOW_COUNT} 等分できません（短時間水準では通常の状態）。"
        )
    else:
        for w, chunk in enumerate(split_windows(steady, CONVERGENCE_WINDOW_COUNT)):
            if not chunk:
                continue
            mean = sum(s.percent for s in chunk) / len(chunk)
            report.line(
                f"  窓{w + 1}: n={len(chunk)}  平均={mean:.2f}%  "
                f"（{chunk[0].at.strftime('%H:%M:%S')} 〜 {chunk[-1].at.strftime('%H:%M:%S')}）"
            )


def render_compose_keys(report: Report, agg: Aggregation, section: str = "11") -> None:
    report.head(f"[{section}] 合成キーの異なり数（要件 7.2・キャッシュ容量の裁定材料）")
    report.line("  キャッシュ容量は承認済み要件で 1 である。以下は「容量 1 で足りるのか」を")
    report.line("  実測で示すための材料であり、容量変更は開発者の明示的な裁定なしに行わない。")
    for label, records in (("全区間", agg.scan.perf), ("定常状態", agg.steady_perf)):
        report.sub(label)
        if not records:
            report.line("  観測なし")
            continue
        pairs = Counter((r.key_hash, r.surface_id) for r in records)
        report.line(f"  異なり数（key_hash x surface_id）: {len(pairs)}")
        report.line(f"  異なり数（key_hash のみ）        : {len({r.key_hash for r in records})}")
        report.line(f"  異なり数（surface_id のみ）      : {len({r.surface_id for r in records})}")
        per_target: dict[str, set] = defaultdict(set)
        for record in records:
            per_target[record.target_id].add((record.key_hash, record.surface_id))
        for target_id, keys in sorted(per_target.items()):
            report.line(f"    {target_id}: 異なりキー {len(keys)} 個")
        report.line("  出現の多いキー（上位 10）:")
        for (key_hash, surface_id), count in pairs.most_common(10):
            report.line(f"    key_hash={key_hash} surface_id={surface_id}: {count} 回")


def render_seriko(report: Report, agg: Aggregation, section: str = "12") -> None:
    report.head(f"[{section}] seriko のループ発火・停止（要件 5.1 の傍証・idle 窓の根拠）")
    report.line("  活性アニメ本数を直接出すログは無い。以下は発火・停止 info! の収支からの")
    report.line("  **間接推定** であり、そのつもりで読むこと。")
    fires = sum(1 for e in agg.scan.seriko_events if e[1] == "fire")
    stops = sum(1 for e in agg.scan.seriko_events if e[1] == "stop")
    report.line(f"  発火 {fires} 件・停止 {stops} 件")
    timeline = active_animation_timeline(agg.scan)
    if timeline:
        counts = [c for _at, c in timeline]
        report.line(f"  推定活性本数の推移: {fmt_stats([float(c) for c in counts], ' 本')}")
        report.line(f"  末尾時点の推定活性本数: {counts[-1]} 本")
    else:
        report.line("  発火・停止のログが 1 件もありません（RUST_LOG に seriko が乗っていない疑い）。")


# =============================================================================
# 合否判定（要件 4.2 の判定式⑴〜⑷・4.3・4.4／収束判定 要件 5.1・5.3）
# -----------------------------------------------------------------------------
# 【この節を貫く規律】観測が無いことを合格として読ませない（D7）。
# 判定式ごとに「支える観測集合が空のときどうなるか」を必ず 1 行で持たせ、レポートに出す。
# 恒真（観測が無いから 0 件）は合格ではなく判定不能である。
# =============================================================================

STATUS_PASS = "合格"
STATUS_FAIL = "不合格"
STATUS_INCONCLUSIVE = "判定不能"
STATUS_NOT_APPLICABLE = "適用外"

#: 総合判定に寄与する状態（適用外は寄与しない＝適用外だけで合格にはならない）。
_STATUS_ORDER = (STATUS_FAIL, STATUS_INCONCLUSIVE, STATUS_PASS, STATUS_NOT_APPLICABLE)


@dataclass
class FormulaResult:
    """判定式 1 本の結果。読み手が「何と何を比べて、いくつの観測でそうなったか」を追える形。"""

    tag: str  # ⑴ ⑵ ⑶ ⑷a ⑷b
    title: str
    applies_to: str  # 適用ビルド（要件 4.3 / 4.4）
    status: str
    measured: str  # 実測値
    threshold: str  # 比べた閾値（較正値名つき）
    calibration: tuple[str, ...]  # 使った較正値の名前
    observations: str  # 支える観測集合の大きさ
    empty_behavior: str  # 観測集合が空のときの挙動（D7）
    reasons: list[str] = field(default_factory=list)


def worst_status(statuses) -> str:
    for status in _STATUS_ORDER:
        if status in statuses:
            return status
    return STATUS_INCONCLUSIVE


def derive_exit_code(results: list[FormulaResult]) -> tuple[int, str]:
    """終了コードを決める。**不合格(1) は判定不能(2) より優先**（design.md「Error Handling」）。

    適用外しか無い場合は合格にせず判定不能にする（沈黙を合格として読ませない・D7）。
    """
    statuses = [r.status for r in results]
    if STATUS_FAIL in statuses:
        failed = "・".join(r.tag for r in results if r.status == STATUS_FAIL)
        return EXIT_FAIL, f"不合格（判定式 {failed} が確定的に不合格）"
    if STATUS_INCONCLUSIVE in statuses:
        unknown = "・".join(r.tag for r in results if r.status == STATUS_INCONCLUSIVE)
        return EXIT_INCONCLUSIVE, f"判定不能（判定式 {unknown} を判定できる観測がない）"
    if STATUS_PASS in statuses:
        return EXIT_OK, "合格（適用される判定式が全て合格）"
    return EXIT_INCONCLUSIVE, "判定不能（適用される判定式が 1 つも無い）"


# --- 定常状態の成立（判定式⑵⑶の前提）---------------------------------------


@dataclass
class SteadyState:
    ok: bool
    applies: int
    span_sec: float
    notes: list[str]


def steady_state_status(agg: "Aggregation") -> SteadyState:
    """定常区間が判定を支えられる厚みを持つか。

    【2.2→2.3 申し送りの核心】定常区間が空だと判定式⑵（catch-up 0 件）と⑶（確保 0 件）は
    「観測が無いから 0」で恒真になる。集計モードは全区間の集計＋警告で exit 0 が正しいが、
    判定モードは合格にしてはならない。
    """
    records = agg.steady_perf
    if not records:
        return SteadyState(
            ok=False,
            applies=0,
            span_sec=0.0,
            notes=[
                "定常状態に perf 行が 1 本もありません"
                f"（WARMUP_EXCLUDE_SEC={WARMUP_EXCLUDE_SEC:.0f} 秒が走行長に対して大きすぎるか、走行が短すぎます）。",
                "この状態では判定式⑵と⑶が「観測が無いから 0 件」で恒真になります。"
                "恒真を合格として読ませないため判定不能にします。",
            ],
        )
    span = (max(r.at for r in records) - agg.steady_from).total_seconds()
    notes: list[str] = []
    ok = True
    if len(records) < VERDICT_MIN_STEADY_APPLIES:
        ok = False
        notes.append(
            f"定常状態の適用が {len(records)} 回しかありません"
            f"（下限 VERDICT_MIN_STEADY_APPLIES={VERDICT_MIN_STEADY_APPLIES} 回）。"
        )
    if span < VERDICT_MIN_STEADY_SEC:
        ok = False
        notes.append(
            f"定常状態の時間幅が {span:.1f} 秒しかありません"
            f"（下限 VERDICT_MIN_STEADY_SEC={VERDICT_MIN_STEADY_SEC:.0f} 秒）。"
        )
    if not ok:
        notes.append(
            "薄すぎる定常区間では「0 件」が中身のある観測なのか観測不足なのか区別できません。"
        )
    return SteadyState(ok=ok, applies=len(records), span_sec=span, notes=notes)


# --- 判定式⑴ コマ適用間隔 ----------------------------------------------------


def vanished_series(
    agg: "Aggregation",
    table: dict[tuple[str, str], list[float]],
    limit: float,
) -> list[tuple[tuple[str, str], str]]:
    """判定窓で間隔が 1 本も残らなかった系列を、除外理由の文言にして返す。

    【なぜ要るか（この関数が塞ぐ穴）】
    `frame_intervals` が返す表は「判定窓の中で **隣接が 2 点そろった** 系列」しか鍵を持たない。
    ゆえに次の 2 種類の系列は、判定対象にも除外一覧にも現れない＝**誰にも数えられない**：

      * 判定窓（推定活性アニメ本数 <= FRAME_INTERVAL_IDLE_MAX_ACTIVE）で適用が全部落ちた系列
      * 判定窓の中の適用が 1 回以下しかない系列（間隔は 0 本になる）

    どちらも外れる条件は本 spec が直そうとしている鈍化そのものである。とくに後者は倒錯していて、
    適用 2 回（間隔 1 本）の系列は FRAME_INTERVAL_MIN_SAMPLES で捕まるのに、適用 1 回の
    **もっと遅い** 系列はすり抜ける。遅いほど見逃されるという逆転を残してはならない。
    そこで走行の perf レコードそのものから「判定され得た系列」を数え直し、表に無い鍵を
    除外として名指しする。呼び出し側はこれを `excluded` に足すので、D7 の関門
    （FRAME_INTERVAL_EXCLUSION_BLOCKS_PASS）がそのまま効いて判定不能になる。

    【窓 A の p95 を併記する理由】
    消えた系列が「遅いから消えた（＝本 spec が追っている鈍化）」のか「速いが散発的なだけ
    （バルーンの書き換え等）」なのかは、判定窓の外＝窓 A（起動過渡のみ除外）を見ないと
    分からない。task 3.1 が FRAME_INTERVAL_JUDGED_SERIES を確定するにはこの区別が要る。
    """
    counts: dict[tuple[str, str], int] = defaultdict(int)
    for record in agg.steady_perf:
        counts[record.series_key] += 1

    lines: list[tuple[tuple[str, str], str]] = []
    for key in sorted(counts):
        if key in table:
            continue
        applies = counts[key]
        window_a = agg.intervals_warmup.get(key) or []
        if window_a:
            p95_a = round(percentile(window_a, 0.95), FRAME_INTERVAL_COMPARE_DECIMALS)
            mean_a = sum(window_a) / len(window_a)
            mean_cap = FRAME_INTERVAL_EXPECTED_MS * FRAME_INTERVAL_MAX_MEAN_FACTOR
            note = (
                f"窓 A（起動過渡のみ除外）の p95 {p95_a:.{FRAME_INTERVAL_COMPARE_DECIMALS}f}ms"
                f"（{len(window_a)} 本・平均 {mean_a:.1f}ms）"
            )
            # 「遅くなったアニメ」と「もともと散発的な系列（バルーンの書き換え等）」の読み分けに、
            # 自動選別が使うのと同じ物差し（平均 vs 期待間隔 x MAX_MEAN_FACTOR）を併記する。
            # ただしこの物差しを超えた側では、両者は閾値だけでは区別できない。そこを
            # 断定すると、鈍化を「バルーンだから」と読み捨てる余地を作ってしまう。
            if mean_a > mean_cap:
                note += (
                    f" ＝ 平均が期待間隔 x FRAME_INTERVAL_MAX_MEAN_FACTOR={mean_cap:.0f}ms を"
                    "超える。この水準では「鈍化したアニメ」と「もともと散発的な系列"
                    "（バルーンの書き換え等）」を自動選別の閾値だけでは区別できない"
                    "（どちらとも決めない。task 3.1 が FRAME_INTERVAL_JUDGED_SERIES で決めること）"
                )
            elif p95_a > limit:
                note += (
                    " ＝ コマ送りの水準（平均は期待間隔 x "
                    f"FRAME_INTERVAL_MAX_MEAN_FACTOR={mean_cap:.0f}ms の内側）でありながら"
                    f"上限 {limit:.{FRAME_INTERVAL_COMPARE_DECIMALS}f}ms を超えている"
                    "＝本 spec が追っている鈍化の疑いが濃い"
                )
            else:
                note += (
                    f" ＝ 上限 {limit:.{FRAME_INTERVAL_COMPARE_DECIMALS}f}ms の内側であり、"
                    "判定窓の絞り込みだけで消えたと読める"
                )
        else:
            note = "窓 A（起動過渡のみ除外）にも間隔が無い（定常状態の適用が 1 回だけ）"
        lines.append(
            (
                key,
                f"{key[0]} / surface_id={key[1]}"
                f"（判定窓の間隔 0 本・定常状態の適用 {applies} 回）: "
                "判定窓に間隔が 1 本も残らなかった"
                f"（推定活性アニメ本数が FRAME_INTERVAL_IDLE_MAX_ACTIVE="
                f"{FRAME_INTERVAL_IDLE_MAX_ACTIVE} 本の上限を超えている、"
                f"または窓内の適用が 1 回以下）。{note}",
            )
        )
    return lines


def judge_frame_interval(agg: "Aggregation") -> FormulaResult:
    """⑴ コマ適用間隔の p95 ≦ 期待間隔 x（1＋許容率）。dev・release 両方に適用（要件 4.3）。"""
    # 上限は浮動小数点の積（172.0 x 1.15 = 197.79999999999998）なので、そのまま比べると
    # 実測 p95 197.8ms の走行で「197.8ms > 197.8ms」という表示と判定の食い違いが出る。
    # 実測側も同じ桁へ丸めて比べ、同じ桁で表示する（FRAME_INTERVAL_COMPARE_DECIMALS）。
    limit = round(
        FRAME_INTERVAL_EXPECTED_MS * (1.0 + FRAME_INTERVAL_TOLERANCE),
        FRAME_INTERVAL_COMPARE_DECIMALS,
    )
    fmt = f".{FRAME_INTERVAL_COMPARE_DECIMALS}f"
    calibration = (
        "FRAME_INTERVAL_EXPECTED_MS",
        "FRAME_INTERVAL_TOLERANCE",
        "FRAME_INTERVAL_JUDGE_WINDOW",
        # 判定を系列ごとに行う（target へ畳まない）という粒度そのもの。合否の意味が
        # ここで決まるので、「使った較正値」に必ず出す（要件 4.5）。
        "FRAME_INTERVAL_JUDGE_GRANULARITY",
        # 判定窓の境界そのもの。消えた系列の理由文がこの値を名指しするので、
        # 「使った較正値」にも出しておく（要件 4.5・値の所在は冒頭バナー 1 箇所）。
        "FRAME_INTERVAL_IDLE_MAX_ACTIVE",
        "FRAME_INTERVAL_SERIES_KEY",
        "FRAME_INTERVAL_JUDGED_SERIES",
        "FRAME_INTERVAL_MIN_SAMPLES",
        "FRAME_INTERVAL_MAX_MEAN_FACTOR",
        "FRAME_INTERVAL_EXCLUSION_BLOCKS_PASS",
        "FRAME_INTERVAL_COMPARE_DECIMALS",
        "PERCENTILE_METHOD",
    )
    empty_behavior = (
        "判定対象の系列が 1 つも残らなければ判定不能（間隔が無いことを合格として読ませない）。"
        "判定窓に間隔が 1 本も残らなかった系列も、間隔の表に鍵が立たないだけで「無かったこと」"
        "にはせず、走行の perf レコードから数え直して除外として名指しする（窓 A の p95 併記）。"
        " さらに " + FRAME_INTERVAL_EXCLUSION_BLOCKS_PASS
        + " 鈍化したアニメほど出すコマ数が減って FRAME_INTERVAL_MIN_SAMPLES で落ちるため、"
        "除外を黙って合格に読み替えると本 spec が直そうとしている鈍化そのものを見逃す。"
    )
    threshold = (
        f"p95 <= {limit:{fmt}}ms"
        f"（FRAME_INTERVAL_EXPECTED_MS={FRAME_INTERVAL_EXPECTED_MS:.0f}ms x "
        f"(1 + FRAME_INTERVAL_TOLERANCE={FRAME_INTERVAL_TOLERANCE:.0%})"
        f"・比較と表示は小数点以下 {FRAME_INTERVAL_COMPARE_DECIMALS} 桁へ丸めて揃える）"
    )
    reasons: list[str] = []
    if not FRAME_INTERVAL_IDLE_EXCLUDE_MARKERS:
        reasons.append(
            "【較正が未確定】talk 再生区間を外す目印（FRAME_INTERVAL_IDLE_EXCLUDE_MARKERS）が"
            "空のため、talk 中の適用が判定窓に混ざり得ます（task 3.1 で確定）。"
        )

    table = agg.intervals_idle
    # 判定窓で間隔が 1 本も残らなかった系列（表に鍵が立たない＝誰にも数えられない系列）。
    # 表の鍵の集合ではなく、走行の perf レコードから「判定され得た系列」を数え直す。
    vanished = vanished_series(agg, table, limit)

    if not table:
        reasons.append(
            f"判定窓（{FRAME_INTERVAL_JUDGE_WINDOW}）に間隔が 1 本もありません。"
        )
        if agg.intervals_warmup:
            reasons.append(
                "窓 A（起動過渡のみ除外）には間隔があります。"
                "推定活性アニメ本数が常時 "
                f"FRAME_INTERVAL_IDLE_MAX_ACTIVE={FRAME_INTERVAL_IDLE_MAX_ACTIVE} 本を"
                "超えていた疑いがあります。"
            )
        if vanished:
            reasons.append("判定窓で間隔が消えた系列:")
            reasons.extend(f"  - {text}" for _key, text in vanished)
        return FormulaResult(
            tag="⑴",
            title="コマ適用間隔の p95",
            applies_to="dev・release",
            status=STATUS_INCONCLUSIVE,
            measured="観測なし",
            threshold=threshold,
            calibration=calibration,
            observations="判定窓の間隔 0 本",
            empty_behavior=empty_behavior,
            reasons=reasons,
        )

    judged: dict[tuple[str, str], list[float]] = {}
    excluded: list[str] = []
    for key, values in sorted(table.items(), key=lambda kv: -len(kv[1])):
        mean = sum(values) / len(values)
        label = f"{key[0]} / surface_id={key[1]}（{len(values)} 本・平均 {mean:.1f}ms）"
        if FRAME_INTERVAL_JUDGED_SERIES:
            if key in FRAME_INTERVAL_JUDGED_SERIES:
                judged[key] = values
            else:
                excluded.append(f"{label}: FRAME_INTERVAL_JUDGED_SERIES の明示指定に無い")
            continue
        if len(values) < FRAME_INTERVAL_MIN_SAMPLES:
            excluded.append(
                f"{label}: 間隔が FRAME_INTERVAL_MIN_SAMPLES={FRAME_INTERVAL_MIN_SAMPLES} 本に満たない"
            )
            continue
        if mean > FRAME_INTERVAL_EXPECTED_MS * FRAME_INTERVAL_MAX_MEAN_FACTOR:
            excluded.append(
                f"{label}: 平均が期待間隔 x FRAME_INTERVAL_MAX_MEAN_FACTOR="
                f"{FRAME_INTERVAL_EXPECTED_MS * FRAME_INTERVAL_MAX_MEAN_FACTOR:.0f}ms を超える"
                "（アニメのコマ送りではなく散発的な適用とみなす）"
            )
            continue
        judged[key] = values

    # 判定窓で消えた系列も除外として名指しする（表に鍵が無いので上の走査には現れない）。
    # これで D7 の関門（下の FRAME_INTERVAL_EXCLUSION_BLOCKS_PASS）がそのまま効く。
    pinned_missing: list[str] = []
    for key, text in vanished:
        if FRAME_INTERVAL_JUDGED_SERIES:
            if key in FRAME_INTERVAL_JUDGED_SERIES:
                # 明示指定した系列が判定窓から消えたのなら、それは「意図した対象外」ではない。
                # 明示指定でも合格を返してはならない（下で判定不能にする）。
                pinned_missing.append(text)
                excluded.append(text)
            else:
                excluded.append(
                    f"{key[0]} / surface_id={key[1]}（判定窓の間隔 0 本）: "
                    "FRAME_INTERVAL_JUDGED_SERIES の明示指定に無い"
                )
        else:
            excluded.append(text)

    if excluded:
        reasons.append("判定対象から外した系列:")
        reasons.extend(f"  - {text}" for text in excluded)

    if not judged:
        reasons.append(
            "判定対象の系列が 1 つも残りませんでした。"
            "期待間隔の "
            f"{FRAME_INTERVAL_MAX_MEAN_FACTOR:.0f} 倍を超える鈍化はこの経路（判定不能）に現れます。"
            "黙って合格にはしません。"
        )
        return FormulaResult(
            tag="⑴",
            title="コマ適用間隔の p95",
            applies_to="dev・release",
            status=STATUS_INCONCLUSIVE,
            measured="判定対象なし",
            threshold=threshold,
            calibration=calibration,
            observations=f"判定窓の系列 {len(table)} 本・うち判定対象 0 本",
            empty_behavior=empty_behavior,
            reasons=reasons,
        )

    # 【判定は系列ごと】target_id へ畳んで 1 本の p95 を採ってはならない。同一 target の
    # 複数系列を連結すると、遅い系列がまるごと速い系列の 5% の裾へ吸収され、スロー再生が
    # 合格に化ける（例: 3000 本 172ms ＋ 30 本 300ms → まとめた p95 は 172.0ms で「合格」）。
    # 要件 4.2⑴ が測るのは「あるアニメのコマ適用間隔 vs そのアニメの指定間隔」であり、
    # 人がスロー再生に気づくのは、隣の系列がどれだけ滑らかでも変わらない。
    # perf 行の surface_id は表示要求の基底 surface id なので、1 つの target が表情替えで
    # 複数の系列を正当に出す。「target あたり 1 系列」には縮められない。
    # まとめた p95 は参考表示としてだけ残す（FRAME_INTERVAL_JUDGE_GRANULARITY）。
    by_target: dict[str, list[tuple[str, list[float]]]] = defaultdict(list)
    for (target_id, surface_id), values in judged.items():
        by_target[target_id].append((surface_id, values))

    # 【順序が意味を持つ】確定的な違反（上限超過）を先に確定させる。除外による判定不能より
    # 不合格が優先する（design.md「Error Handling」）。
    measured_parts: list[str] = []
    failures: list[str] = []
    for target_id, entries in sorted(by_target.items()):
        pooled: list[float] = []
        for surface_id, values in sorted(entries, key=lambda entry: entry[0]):
            pooled.extend(values)
            p95 = round(percentile(values, 0.95), FRAME_INTERVAL_COMPARE_DECIMALS)
            mark = "OK" if p95 <= limit else "超過"
            # 判定した系列にも、除外した系列と同じ密度で実測を並べる（要件 4.2⑴ の根拠）。
            # 除外側だけが n・最小・p50・p95・最大・平均を出していた非対称が、
            # まとめた p95 に吸収された系列を「読めば分かる」はずの位置から消していた。
            reasons.append(
                f"系列 {target_id} / surface_id={surface_id}: "
                f"{fmt_stats(values, 'ms', FRAME_INTERVAL_COMPARE_DECIMALS)} → "
                f"p95 {p95:{fmt}}ms vs 上限 {limit:{fmt}}ms（{mark}）"
            )
            measured_parts.append(
                f"{target_id}/surface_id={surface_id}: p95={p95:{fmt}}ms（{len(values)} 本）"
            )
            if p95 > limit:
                failures.append(
                    f"{target_id} / surface_id={surface_id}"
                    f"（p95 {p95:{fmt}}ms > {limit:{fmt}}ms）"
                )
        pooled_p95 = round(percentile(pooled, 0.95), FRAME_INTERVAL_COMPARE_DECIMALS)
        reasons.append(
            f"  （参考・判定には使わない）対象 {target_id} の {len(entries)} 系列を"
            f"まとめた p95 {pooled_p95:{fmt}}ms（{len(pooled)} 本）"
        )

    total = sum(len(v) for v in judged.values())
    observations = (
        f"判定対象 {len(judged)} 系列・間隔 {total} 本（対象 {len(by_target)} 個）"
        "／判定は系列ごと（target 別のまとめは参考表示）"
    )
    if excluded:
        observations += f"／判定対象から外した系列 {len(excluded)} 本: " + " ／ ".join(excluded)

    if failures:
        return FormulaResult(
            tag="⑴",
            title="コマ適用間隔の p95",
            applies_to="dev・release",
            status=STATUS_FAIL,
            measured="  ".join(measured_parts),
            threshold=threshold,
            calibration=calibration,
            observations=observations,
            empty_behavior=empty_behavior,
            reasons=reasons + [f"上限を超えた系列: {'・'.join(failures)}"],
        )

    # 明示指定した系列が判定窓から消えていたら、明示指定でも合格を返さない。
    # 「対象はこれだ」と決めた系列を測れていないのだから、残りが上限内でも走行の合格ではない。
    if pinned_missing:
        reasons.append(
            f"FRAME_INTERVAL_JUDGED_SERIES で明示指定した {len(pinned_missing)} 系列が、"
            "判定窓に間隔を 1 本も残していません（上の一覧）。明示指定した対象を測れていない"
            "ので、残った系列が上限内でも走行の合格としては読ませません（判定不能）。"
        )
        return FormulaResult(
            tag="⑴",
            title="コマ適用間隔の p95",
            applies_to="dev・release",
            status=STATUS_INCONCLUSIVE,
            measured="  ".join(measured_parts)
            + f"（ただし明示指定 {len(pinned_missing)} 系列が判定窓から消失）",
            threshold=threshold,
            calibration=calibration,
            observations=observations,
            empty_behavior=empty_behavior,
            reasons=reasons,
        )

    # 除外した系列があるあいだは合格を返さない（D7・FRAME_INTERVAL_EXCLUSION_BLOCKS_PASS）。
    # 外れる条件（コマ数が少ない・平均間隔が長い）は、本 spec が直そうとしている鈍化そのもの
    # なので、「残った速い系列が上限内だった」を走行全体の合格として読ませてはならない。
    if excluded and not FRAME_INTERVAL_JUDGED_SERIES:
        reasons.append(
            f"残った {len(judged)} 系列はいずれも上限内でしたが、上に挙げた "
            f"{len(excluded)} 系列を自動選別で判定対象から外しています。"
        )
        reasons.append(
            "自動選別で外れる条件（コマ数が下限未満・平均間隔が期待の "
            f"{FRAME_INTERVAL_MAX_MEAN_FACTOR:.0f} 倍超）は、本 spec が直そうとしている"
            "鈍化そのものです。鈍化した系列ほど出すコマ数が減るため、まず "
            f"FRAME_INTERVAL_MIN_SAMPLES={FRAME_INTERVAL_MIN_SAMPLES} 本の下限で落ちます。"
        )
        reasons.append(
            "ゆえに「速い系列だけが残ったので合格」とは読ませません（D7）。判定不能とし、"
            "外した系列を名指しで上に出します。"
        )
        reasons.append(
            "解除の仕方: task 3.1 の第 1 段ベースラインで FRAME_INTERVAL_JUDGED_SERIES を"
            "実測から確定し明示指定に切り替えれば、除外は意図した対象外になり合格が"
            "再び意味を持ちます。"
        )
        return FormulaResult(
            tag="⑴",
            title="コマ適用間隔の p95",
            applies_to="dev・release",
            status=STATUS_INCONCLUSIVE,
            measured="  ".join(measured_parts) + f"（ただし {len(excluded)} 系列を除外）",
            threshold=threshold,
            calibration=calibration,
            observations=observations,
            empty_behavior=empty_behavior,
            reasons=reasons,
        )

    return FormulaResult(
        tag="⑴",
        title="コマ適用間隔の p95",
        applies_to="dev・release",
        status=STATUS_PASS,
        measured="  ".join(measured_parts),
        threshold=threshold,
        calibration=calibration,
        observations=observations,
        empty_behavior=empty_behavior,
        reasons=reasons,
    )


# --- 判定式⑵ 進行境界スキップ（catch-up）------------------------------------


def judge_catchup(agg: "Aggregation", steady: SteadyState) -> FormulaResult:
    """⑵ 定常状態の catch-up 件数 = 0。dev・release 両方に適用（要件 4.3）。"""
    calibration = (
        "WARMUP_EXCLUDE_SEC",
        "VERDICT_MIN_STEADY_APPLIES",
        "VERDICT_MIN_STEADY_SEC",
        "J_CATCHUP_TICKER_MESSAGE",
        "J_CATCHUP_LOOP_MESSAGE",
    )
    empty_behavior = (
        "定常区間が空（または薄すぎる）なら「0 件」が恒真になるため判定不能。合格にしない。"
    )
    threshold = "定常状態の catch-up 件数 = 0 件"

    in_steady = [
        e for e in agg.scan.catchup if e[0] is not None and e[0] >= agg.steady_from
    ]
    undated = [e for e in agg.scan.catchup if e[0] is None]
    reasons = [
        f"全区間の catch-up: {len(agg.scan.catchup)} 件"
        f"（うち定常状態 {len(in_steady)} 件・時刻を読めない行 {len(undated)} 件）",
        "数え方: 長い文言（loop_ticker）を先に判定してから短い文言を数える（二重計上の罠）。",
    ]

    # 確定的な違反が 1 件でもあれば不合格（判定不能より優先）。
    if in_steady:
        counts: Counter = Counter((kind, target) for _at, kind, target in in_steady)
        for (kind, target), count in sorted(counts.items()):
            reasons.append(f"  文言={kind}  target={target}: {count} 件")
        return FormulaResult(
            tag="⑵",
            title="定常状態の進行境界スキップ（catch-up）件数",
            applies_to="dev・release",
            status=STATUS_FAIL,
            measured=f"{len(in_steady)} 件",
            threshold=threshold,
            calibration=calibration,
            observations=f"定常状態の適用 {steady.applies} 回・時間幅 {steady.span_sec:.1f} 秒",
            empty_behavior=empty_behavior,
            reasons=reasons,
        )

    if not steady.ok:
        return FormulaResult(
            tag="⑵",
            title="定常状態の進行境界スキップ（catch-up）件数",
            applies_to="dev・release",
            status=STATUS_INCONCLUSIVE,
            measured="0 件（ただし恒真）",
            threshold=threshold,
            calibration=calibration,
            observations=f"定常状態の適用 {steady.applies} 回・時間幅 {steady.span_sec:.1f} 秒",
            empty_behavior=empty_behavior,
            reasons=reasons + steady.notes,
        )

    if undated:
        return FormulaResult(
            tag="⑵",
            title="定常状態の進行境界スキップ（catch-up）件数",
            applies_to="dev・release",
            status=STATUS_INCONCLUSIVE,
            measured=f"定常状態 0 件・時刻不明 {len(undated)} 件",
            threshold=threshold,
            calibration=calibration,
            observations=f"定常状態の適用 {steady.applies} 回・時間幅 {steady.span_sec:.1f} 秒",
            empty_behavior=empty_behavior,
            reasons=reasons
            + [
                "時刻を読めない catch-up 行があり、定常状態の内か外かを決められません。"
                "定常状態 0 件と言い切れないため判定不能にします。",
            ],
        )

    return FormulaResult(
        tag="⑵",
        title="定常状態の進行境界スキップ（catch-up）件数",
        applies_to="dev・release",
        status=STATUS_PASS,
        measured="0 件",
        threshold=threshold,
        calibration=calibration,
        observations=f"定常状態の適用 {steady.applies} 回・時間幅 {steady.span_sec:.1f} 秒",
        empty_behavior=empty_behavior,
        reasons=reasons,
    )


# --- 判定式⑶ 表示用バッファの新規確保 ----------------------------------------


def judge_allocations(agg: "Aggregation", steady: SteadyState) -> FormulaResult:
    """⑶ 定常状態の alloc_* 合計 = 0。dev・release 両方に適用（要件 4.3）。"""
    calibration = (
        "WARMUP_EXCLUDE_SEC",
        "VERDICT_MIN_STEADY_APPLIES",
        "VERDICT_MIN_STEADY_SEC",
        "J_PERF_ALLOC_FIELDS",
    )
    empty_behavior = (
        "定常区間が空（または薄すぎる）なら「合計 0」が恒真になるため判定不能。合格にしない。"
    )
    threshold = "定常状態の alloc_* 合計 = 0"
    observations = f"定常状態の適用 {steady.applies} 回・時間幅 {steady.span_sec:.1f} 秒"

    per_field = {
        name: sum(r.allocs[name] for r in agg.steady_perf) for name in J_PERF_ALLOC_FIELDS
    }
    total = sum(per_field.values())
    reasons = [
        "内訳（定常状態）: "
        + "  ".join(f"{name}={value}" for name, value in per_field.items()),
    ]

    if total > 0:
        offenders = [
            r for r in agg.steady_perf if any(r.allocs[n] > 0 for n in J_PERF_ALLOC_FIELDS)
        ]
        reasons.append(f"確保が起きた適用: {len(offenders)} 回（最初の 5 件を示す）")
        for record in offenders[:5]:
            detail = " ".join(
                f"{n}={record.allocs[n]}" for n in J_PERF_ALLOC_FIELDS if record.allocs[n] > 0
            )
            reasons.append(
                f"  {record.line_no} 行目 {record.at.isoformat()} "
                f"{record.target_id} / surface_id={record.surface_id}: {detail}"
            )
        return FormulaResult(
            tag="⑶",
            title="定常状態の表示用バッファ新規確保",
            applies_to="dev・release",
            status=STATUS_FAIL,
            measured=f"合計 {total}",
            threshold=threshold,
            calibration=calibration,
            observations=observations,
            empty_behavior=empty_behavior,
            reasons=reasons,
        )

    if not steady.ok:
        return FormulaResult(
            tag="⑶",
            title="定常状態の表示用バッファ新規確保",
            applies_to="dev・release",
            status=STATUS_INCONCLUSIVE,
            measured="合計 0（ただし恒真）",
            threshold=threshold,
            calibration=calibration,
            observations=observations,
            empty_behavior=empty_behavior,
            reasons=reasons + steady.notes,
        )

    return FormulaResult(
        tag="⑶",
        title="定常状態の表示用バッファ新規確保",
        applies_to="dev・release",
        status=STATUS_PASS,
        measured="合計 0",
        threshold=threshold,
        calibration=calibration,
        observations=observations,
        empty_behavior=empty_behavior,
        reasons=reasons,
    )


# --- 収束判定（要件 5.1・5.3）------------------------------------------------


@dataclass
class Convergence:
    """CPU 時系列が頭打ちへ収束したか、単調上昇を続けるか。"""

    verdict: str  # CONVERGENCE_SETTLED / CONVERGENCE_RISING / CONVERGENCE_UNKNOWN のいずれか
    slope: float | None
    window_means: list[tuple[int, int, float, datetime, datetime]]
    diff: float | None
    notes: list[str]


CONVERGENCE_SETTLED = "頭打ち（定常到達）"
CONVERGENCE_RISING = "単調上昇（未知機序候補）"
CONVERGENCE_UNKNOWN = "判定不能"


def judge_convergence(agg: "Aggregation") -> Convergence:
    """design.md「収束判定」の定義をそのまま実装する。

        ウォームアップ除外後の CPU 時系列を窓分割し、
        後半窓の回帰傾き <= 上限 かつ 末尾窓平均 − 中間窓平均 <= 上限 なら「頭打ち」、
        いずれか超過なら「単調上昇（未知機序候補）」。
    """
    steady = [s for s in agg.cpu if s.at >= agg.steady_from]
    need = CONVERGENCE_MIN_SAMPLES_PER_WINDOW * CONVERGENCE_WINDOW_COUNT
    if len(steady) < need:
        return Convergence(
            verdict=CONVERGENCE_UNKNOWN,
            slope=None,
            window_means=[],
            diff=None,
            notes=[
                f"定常状態の CPU 採取点が {len(steady)} 点しかなく、"
                f"{CONVERGENCE_WINDOW_COUNT} 窓 x "
                f"CONVERGENCE_MIN_SAMPLES_PER_WINDOW={CONVERGENCE_MIN_SAMPLES_PER_WINDOW} "
                f"= {need} 点に届きません。",
                "窓を作れない時系列から収束の有無は読めません（観測不足を収束と読ませない）。",
            ],
        )

    windows = split_windows(steady, CONVERGENCE_WINDOW_COUNT)
    means = [
        (
            i + 1,
            len(chunk),
            sum(s.percent for s in chunk) / len(chunk),
            chunk[0].at,
            chunk[-1].at,
        )
        for i, chunk in enumerate(windows)
        if chunk
    ]
    latter: list[CpuSample] = []
    for i, chunk in enumerate(windows):
        if i + 1 >= CONVERGENCE_SLOPE_WINDOW_FROM:
            latter.extend(chunk)
    slope = regression_slope_per_min(latter)
    tail_mean = means[-1][2]
    middle = next(
        (m for m in means if m[0] == CONVERGENCE_MIDDLE_WINDOW_INDEX), None
    )
    notes = [
        f"窓の切り方: {CONVERGENCE_WINDOW_SPLIT}（{CONVERGENCE_WINDOW_COUNT} 窓）",
        f"傾きの求め方: {CONVERGENCE_SLOPE_METHOD}"
        f"・対象は 窓{CONVERGENCE_SLOPE_WINDOW_FROM}〜窓{CONVERGENCE_WINDOW_COUNT}（{len(latter)} 点）",
    ]
    if middle is None or slope is None:
        notes.append(
            "後半窓の回帰または中間窓の平均を出せません（時刻が 1 点に潰れている等）。"
        )
        return Convergence(CONVERGENCE_UNKNOWN, slope, means, None, notes)

    diff = tail_mean - middle[2]
    slope_ok = slope <= CONVERGENCE_SLOPE_MAX_PCT_PER_MIN
    diff_ok = diff <= CONVERGENCE_WINDOW_DIFF_MAX_PCT
    notes.append(
        f"後半窓の回帰傾き {slope:+.4f} %/分 vs 上限 "
        f"CONVERGENCE_SLOPE_MAX_PCT_PER_MIN={CONVERGENCE_SLOPE_MAX_PCT_PER_MIN}"
        f" → {'以内' if slope_ok else '超過'}"
    )
    notes.append(
        f"末尾窓平均 {tail_mean:.2f}% − 中間窓（窓{CONVERGENCE_MIDDLE_WINDOW_INDEX}）平均 "
        f"{middle[2]:.2f}% = {diff:+.2f}% vs 上限 "
        f"CONVERGENCE_WINDOW_DIFF_MAX_PCT={CONVERGENCE_WINDOW_DIFF_MAX_PCT}"
        f" → {'以内' if diff_ok else '超過'}"
    )
    verdict = CONVERGENCE_SETTLED if (slope_ok and diff_ok) else CONVERGENCE_RISING
    return Convergence(verdict, slope, means, diff, notes)


# --- 判定式⑷ アイドル CPU（release のみ・要件 4.3/4.4）-----------------------


def judge_idle_cpu(
    agg: "Aggregation", build: str, convergence: Convergence, meta_profile: str | None
) -> list[FormulaResult]:
    """⑷ アイドル CPU < 3.0% かつ 20 分超で単調上昇しない。**release のみ**（要件 4.4）。

    前半（閾値）と後半（20 分超で単調上昇しない）を別々の結果として返す。両者は成立条件が
    違うためひとまとめにすると「短時間走行なのに長時間の性質を確かめた」と読まれてしまう。
    """
    calibration_a = (
        "IDLE_CPU_MAX_RELEASE_PCT",
        "IDLE_CPU_STATISTIC",
        "WARMUP_EXCLUDE_SEC",
        "VERDICT_MIN_CPU_SAMPLES",
    )
    calibration_b = (
        "LONG_RUN_MIN_SPAN_SEC",
        "CONVERGENCE_WINDOW_COUNT",
        "CONVERGENCE_WINDOW_SPLIT",
        "CONVERGENCE_SLOPE_METHOD",
        "CONVERGENCE_SLOPE_MAX_PCT_PER_MIN",
        "CONVERGENCE_WINDOW_DIFF_MAX_PCT",
        "CONVERGENCE_MIN_SAMPLES_PER_WINDOW",
    )
    empty_a = "定常状態の CPU 採取点が下限に満たなければ判定不能（無観測を合格にしない）。"
    empty_b = (
        "20 分超に満たない走行では適用外（終了コードに寄与しない）。"
        "ただし profile=long の走行が 20 分に届かない場合は途中で切れた走行として判定不能。"
        "この分岐は profile に依存するため、判定モードは run-meta.txt に profile が無い入力を"
        "読み込み時点で判定不能(2) として拒む（適用外へ黙って落とさない）。"
    )

    steady = [s for s in agg.cpu if s.at >= agg.steady_from]
    span = (steady[-1].at - steady[0].at).total_seconds() if len(steady) >= 2 else 0.0

    if build != "release":
        note = (
            "要件 4.4: dev ビルドには CPU の数値目標を課さない"
            "（要件 4.3 の判定式⑴〜⑶のみ適用）。適用外は合格ではなく、終了コードに寄与しない。"
        )
        return [
            FormulaResult(
                tag="⑷a",
                title="アイドル CPU（1 コア換算）",
                applies_to="release のみ",
                status=STATUS_NOT_APPLICABLE,
                measured=(
                    f"参考値 平均 {sum(s.percent for s in steady) / len(steady):.2f}%"
                    if steady
                    else "参考値なし"
                ),
                threshold=f"< {IDLE_CPU_MAX_RELEASE_PCT}%（release のみ）",
                calibration=calibration_a,
                observations=f"定常状態の CPU 採取点 {len(steady)} 点",
                empty_behavior=empty_a,
                reasons=[note, f"本走行のビルド種別: {build}"],
            ),
            FormulaResult(
                tag="⑷b",
                title="20 分超で単調上昇しない",
                applies_to="release のみ",
                status=STATUS_NOT_APPLICABLE,
                measured=f"参考値 収束判定={convergence.verdict}",
                threshold="収束判定が「頭打ち」であること",
                calibration=calibration_b,
                observations=f"定常状態の CPU 採取点 {len(steady)} 点・時間幅 {span:.0f} 秒",
                empty_behavior=empty_b,
                reasons=[note, f"本走行のビルド種別: {build}"],
            ),
        ]

    # ---- ⑷a 閾値の半分 ----
    if len(steady) < VERDICT_MIN_CPU_SAMPLES:
        first = FormulaResult(
            tag="⑷a",
            title="アイドル CPU（1 コア換算）",
            applies_to="release のみ",
            status=STATUS_INCONCLUSIVE,
            measured="観測不足",
            threshold=f"< {IDLE_CPU_MAX_RELEASE_PCT}%",
            calibration=calibration_a,
            observations=f"定常状態の CPU 採取点 {len(steady)} 点",
            empty_behavior=empty_a,
            reasons=[
                f"定常状態の CPU 採取点が {len(steady)} 点しかありません"
                f"（下限 VERDICT_MIN_CPU_SAMPLES={VERDICT_MIN_CPU_SAMPLES} 点）。",
                "起動過渡を除いた区間に観測が無い走行から、アイドル CPU は読めません。",
            ],
        )
    else:
        values = [s.percent for s in steady]
        mean = sum(values) / len(values)
        ok = mean < IDLE_CPU_MAX_RELEASE_PCT
        first = FormulaResult(
            tag="⑷a",
            title="アイドル CPU（1 コア換算）",
            applies_to="release のみ",
            status=STATUS_PASS if ok else STATUS_FAIL,
            measured=f"平均 {mean:.2f}%",
            threshold=(
                f"< {IDLE_CPU_MAX_RELEASE_PCT}%"
                f"（IDLE_CPU_MAX_RELEASE_PCT・比べる量は {IDLE_CPU_STATISTIC}）"
            ),
            calibration=calibration_a,
            observations=f"定常状態の CPU 採取点 {len(steady)} 点・時間幅 {span:.0f} 秒",
            empty_behavior=empty_a,
            reasons=[
                f"定常状態の CPU: {fmt_stats(values, '%')}",
                "1 行は約 1 秒幅の瞬時値であり刻み幅の平均ではない（2.1→2.4 申し送り）。",
                "【task 3.1 への申し送り】平均は 1 回の長い非アイドル区間に引きずられる"
                "（分位点と違って裾に強くない）。"
                "FRAME_INTERVAL_IDLE_EXCLUDE_MARKERS が空のあいだは talk 再生区間が混ざり得るので、"
                "第 1 段ベースラインでは上の p50 と平均を必ず突き合わせること。"
                "両者が離れていたら、比べる量ではなく窓の切り方を疑う。",
            ]
            + (
                []
                if ok
                else [
                    f"平均 {mean:.2f}% が上限 {IDLE_CPU_MAX_RELEASE_PCT}% を下回っていません。"
                    "要件 4.6 に従い残る最大内訳を特定して是正へ戻ること。",
                ]
            ),
        )

    # ---- ⑷b 20 分超で単調上昇しない ----
    long_enough = span >= LONG_RUN_MIN_SPAN_SEC
    observations_b = f"定常状態の CPU 採取点 {len(steady)} 点・時間幅 {span:.0f} 秒"
    if not long_enough:
        declared_long = (meta_profile or "").strip().lower() == "long"
        if declared_long:
            second = FormulaResult(
                tag="⑷b",
                title="20 分超で単調上昇しない",
                applies_to="release のみ",
                status=STATUS_INCONCLUSIVE,
                measured=f"収束判定={convergence.verdict}",
                threshold=f"時間幅 >= LONG_RUN_MIN_SPAN_SEC={LONG_RUN_MIN_SPAN_SEC:.0f} 秒",
                calibration=calibration_b,
                observations=observations_b,
                empty_behavior=empty_b,
                reasons=[
                    "run-meta.txt の profile は long ですが、定常状態の CPU 時系列は "
                    f"{span:.0f} 秒しかありません（必要 {LONG_RUN_MIN_SPAN_SEC:.0f} 秒）。",
                    "途中で切れた長時間走行とみなします。"
                    "短い走行を「長時間の性質を確かめた」と読ませないため判定不能にします。",
                ],
            )
        else:
            second = FormulaResult(
                tag="⑷b",
                title="20 分超で単調上昇しない",
                applies_to="release のみ・長時間水準（20 分超）のみ",
                status=STATUS_NOT_APPLICABLE,
                measured=f"参考値 収束判定={convergence.verdict}",
                threshold=f"時間幅 >= LONG_RUN_MIN_SPAN_SEC={LONG_RUN_MIN_SPAN_SEC:.0f} 秒",
                calibration=calibration_b,
                observations=observations_b,
                empty_behavior=empty_b,
                reasons=[
                    f"本走行の定常状態は {span:.0f} 秒で、20 分超の性質を確かめられません。",
                    "design.md Flow 1: 長時間水準はベースライン（第 1 段）と最終合格判定"
                    "（第 4 段）の 2 回のみで、中間の是正ループは短時間水準で回す。"
                    "したがって短時間走行では本判定は適用外とし、終了コードに寄与させない。",
                    "【未検証であることの明示】この走行は 20 分超で単調上昇しないことを"
                    "確かめていない。長時間水準の判定は task 7.2 が行う。",
                    "上の参考値は収束判定（要件 5.1）の結果であって、⑷後半の合格ではない。",
                ],
            )
    else:
        status = {
            CONVERGENCE_SETTLED: STATUS_PASS,
            CONVERGENCE_RISING: STATUS_FAIL,
            CONVERGENCE_UNKNOWN: STATUS_INCONCLUSIVE,
        }[convergence.verdict]
        reasons = [f"収束判定: {convergence.verdict}"] + convergence.notes
        if status == STATUS_FAIL:
            reasons.append(
                "要件 5.2: 頭打ちに収束しない場合は未知の機序として実測データを添えて記録し、"
                "本 spec の範囲で是正可能かを判定のうえ、範囲外なら独立した課題として起票する。"
            )
        second = FormulaResult(
            tag="⑷b",
            title="20 分超で単調上昇しない",
            applies_to="release のみ・長時間水準（20 分超）のみ",
            status=status,
            measured=f"収束判定={convergence.verdict}",
            threshold=(
                f"傾き <= {CONVERGENCE_SLOPE_MAX_PCT_PER_MIN} %/分 かつ "
                f"末尾窓平均 − 中間窓平均 <= {CONVERGENCE_WINDOW_DIFF_MAX_PCT}%"
            ),
            calibration=calibration_b,
            observations=observations_b,
            empty_behavior=empty_b,
            reasons=reasons,
        )
    return [first, second]


# --- 傍証: seriko の発火・停止（要件 5.1 の但し書き）--------------------------


def render_convergence(report: Report, agg: "Aggregation", convergence: Convergence) -> None:
    report.head("[6] 収束判定（要件 5.1・根拠は較正値つき＝要件 5.3）")
    report.line("  定義（design.md「収束判定」）: ウォームアップ除外後の CPU 時系列を窓分割し、")
    report.line("  後半窓の回帰傾き <= 上限 かつ 末尾窓平均 − 中間窓平均 <= 上限 なら「頭打ち」、")
    report.line("  いずれか超過なら「単調上昇（未知機序候補）」。")
    report.line("")
    report.line(f"  判定: {convergence.verdict}")
    for note in convergence.notes:
        report.line(f"    - {note}")
    if convergence.window_means:
        report.sub("窓別平均")
        tail_index = convergence.window_means[-1][0]
        for index, count, mean, first, last in convergence.window_means:
            mark = []
            if index == CONVERGENCE_MIDDLE_WINDOW_INDEX:
                mark.append("中間窓")
            if index == tail_index:
                mark.append("末尾窓")
            if index >= CONVERGENCE_SLOPE_WINDOW_FROM:
                mark.append("後半窓")
            report.line(
                f"  窓{index}: n={count}  平均={mean:.2f}%  "
                f"（{first.strftime('%H:%M:%S')} 〜 {last.strftime('%H:%M:%S')}）"
                + (f"  ← {'・'.join(mark)}" if mark else "")
            )

    report.sub("傍証: seriko のループ発火・停止の窓別件数")
    report.line("  【重要】活性アニメの集合サイズを直接出すログは存在しない。以下は発火・停止")
    report.line("  info! の件数であり、活性集合の大きさは**間接推定**にとどまる（そのつもりで読むこと）。")
    steady_cpu = [s for s in agg.cpu if s.at >= agg.steady_from]
    windows = split_windows(steady_cpu, CONVERGENCE_WINDOW_COUNT)
    total_fire = sum(1 for _at, kind, _s, _a in agg.scan.seriko_events if kind == "fire")
    total_stop = sum(1 for _at, kind, _s, _a in agg.scan.seriko_events if kind == "stop")
    undated = sum(1 for at, _kind, _s, _a in agg.scan.seriko_events if at is None)
    if not any(windows):
        report.line("  定常状態の CPU 採取点が無いため窓を作れません。")
        report.line(
            f"  （全区間の合計は 発火 {total_fire} 件・停止 {total_stop} 件＝[8.8] と同じ数え方）"
        )
        return
    attributed_fire = 0
    attributed_stop = 0
    for i, chunk in enumerate(windows, start=1):
        if not chunk:
            continue
        start, end = chunk[0].at, chunk[-1].at
        fires = sum(
            1 for at, kind, _s, _a in agg.scan.seriko_events
            if kind == "fire" and at is not None and start <= at <= end
        )
        stops = sum(
            1 for at, kind, _s, _a in agg.scan.seriko_events
            if kind == "stop" and at is not None and start <= at <= end
        )
        attributed_fire += fires
        attributed_stop += stops
        report.line(
            f"  窓{i}（{start.strftime('%H:%M:%S')} 〜 {end.strftime('%H:%M:%S')}）: "
            f"発火 {fires} 件・停止 {stops} 件・収支 {fires - stops:+d}"
        )

    # 【突合を出す】窓の境界は「各窓の最初と最後の CPU 採取点の時刻」なので、窓と窓の隙間や
    # 定常状態の CPU 採取範囲の外に落ちた発火・停止はどの窓にも入らない。突合を書かないと、
    # 上の窓別件数の合計と [8.8] の全体件数が黙って食い違って見える。
    report.line("")
    report.line("  窓別件数と全体件数の突合（窓の境界＝各窓の最初と最後の CPU 採取点の時刻）:")
    report.line(
        f"    発火: 窓に入った {attributed_fire} 件 / 全体 {total_fire} 件 → "
        f"どの窓にも入らなかった {total_fire - attributed_fire} 件"
    )
    report.line(
        f"    停止: 窓に入った {attributed_stop} 件 / 全体 {total_stop} 件 → "
        f"どの窓にも入らなかった {total_stop - attributed_stop} 件"
    )
    report.line(f"    うち時刻を読めなかった行: {undated} 件（時刻が無いのでどの窓にも入らない）")
    report.line("    窓に入らない残りの内訳: 定常状態の最初の採取点より前／最後の採取点より後／")
    report.line(
        f"    窓と窓の隙間（隙間は最大でも CPU 刻み 1 つ分＝約 "
        f"{CPU_SAMPLE_INTERVAL_EXPECTED_SEC:.0f} 秒）。"
    )
    report.line("    窓別件数は傍証であり、全体件数と一致しないのは上の理由による（数え落としではない）。")


def render_verdict(report: Report, results: list[FormulaResult]) -> None:
    report.head("[5] 判定結果（要件 4.2 の判定式⑴〜⑷・適用範囲は要件 4.3 / 4.4）")
    report.line("  判定の語: 合格／不合格／判定不能（観測が無い・足りない）／適用外（この走行の対象外）")
    report.line("  適用外は合格ではない。終了コードにも寄与しない。")
    for result in results:
        report.sub(f"{result.tag} {result.title}  →  {result.status}")
        report.line(f"  適用ビルド : {result.applies_to}")
        report.line(f"  実測値     : {result.measured}")
        report.line(f"  比べた閾値 : {result.threshold}")
        report.line(f"  観測集合   : {result.observations}")
        report.line(f"  空のときは : {result.empty_behavior}")
        report.line(f"  使った較正値: {', '.join(result.calibration)}")
        if result.reasons:
            report.line("  根拠:")
            for reason in result.reasons:
                report.line(f"    {reason}")


# =============================================================================
# 必要ログ種の検査（要件 2.5）
# =============================================================================


def check_required(scan: LogScan, cpu: list[CpuSample], run_log: Path) -> None:
    missing: list[str] = []
    if not scan.perf:
        missing.append(
            f"perf サマリ行（固定文言 {J_PERF_LINE_MESSAGE!r}）が 1 本もありません。"
            " RUST_LOG に areka_emo_present=debug が乗っていない疑いがあります。"
        )
    if scan.show_count == 0:
        missing.append(
            f"表示成立点 info!（固定文言 {J_SHOW_LINE_MESSAGE!r}）が 1 本もありません。"
        )
    if not cpu:
        missing.append("CPU 時系列に観測が 1 点もありません。")
    if missing:
        raise inconclusive(
            f"必要ログ種が欠けています（{run_log}）。部分集計は出しません（要件 2.5）。",
            missing,
        )


# =============================================================================
# 入力の読み込み（集計モードと判定モードで共通）
# =============================================================================


@dataclass
class Inputs:
    run_log: Path
    cpu_csv: Path
    meta_path: Path
    sections: list
    scan: LogScan
    cpu: list[CpuSample]
    agg: "Aggregation"
    build: str | None
    profile: str | None


def load_inputs(args: argparse.Namespace, require_build: bool) -> Inputs:
    """3 つの入力を読み、必要ログ種を検査して集計まで済ませる。

    `require_build` は判定モード用。どの判定式を適用するかがビルド種別で変わる（要件 4.3/4.4）
    ため、判定モードでは推定に頼らず明示を求める。
    """
    run_log = Path(args.run_log)
    cpu_csv = Path(args.cpu_csv)
    meta_given = args.meta is not None
    meta_path = Path(args.meta) if meta_given else cpu_csv.parent / "run-meta.txt"

    if require_build and not args.build:
        raise bad_input(
            "判定モードでは --build dev|release の明示が要ります。",
            [
                "要件 4.3/4.4 により、適用する判定式がビルド種別で変わります"
                "（⑴〜⑶は両ビルド・⑷は release のみ）。",
                "run-meta.txt からの推定で判定範囲を決めると、取り違えが黙って通ります。",
            ],
        )

    for path, what in ((run_log, "実走ログ"), (cpu_csv, "CPU 時系列")):
        if not path.is_file():
            raise bad_input(f"{what}のファイルがありません: {path}")
    if not meta_path.is_file():
        # 明示指定されたパスが無いのは引数の誤り（3）。自動探索で見つからないのは
        # 走行に条件ファイルが無いということ＝欠落（2）。両者は直し方が違うので分ける。
        if meta_given:
            raise bad_input(f"--meta が指すファイルがありません: {meta_path}")
        raise inconclusive(
            f"実行条件（run-meta.txt）が見つかりません: {meta_path}",
            [
                "要件 4.5 は測定マシンの条件をレポートへ併記することを求めており、",
                "条件を書けない集計は出しません。",
                "既定では cpu.csv と同じディレクトリを探します。別の場所にある場合は",
                "--meta <パス> で指定してください。",
            ],
        )

    sections = read_run_meta(meta_path)
    scan = scan_log(run_log)
    cpu = read_cpu_csv(cpu_csv)
    check_required(scan, cpu, run_log)

    meta_build = meta_lookup(sections, "build")
    build = args.build or meta_build
    if args.build and meta_build and args.build != meta_build:
        raise bad_input(
            "--build と run-meta.txt の build が食い違っています（別の走行のログを"
            "組み合わせている疑い）。",
            [f"--build = {args.build}", f"run-meta.txt build = {meta_build}"],
        )

    meta_profile = meta_lookup(sections, "profile")
    if require_build and meta_profile is None:
        # 【黙って適用外へ落とさない】判定式⑷b は「20 分に届かない走行は適用外。ただし
        # profile=long なら途中で切れた長時間走行として判定不能」と決めている。profile が
        # 無いと後者の枝へ入れず、切れた長時間走行が黙って「適用外・終了コード 0」になる。
        # build は上で突合しているのに profile は突合していなかった、という非対称を塞ぐ。
        # 終了コードは 2（判定不能）を選ぶ: run-meta.txt そのものが無い場合を 2 にしている
        # のと同じ理由で、これは「走行条件を書けない＝データ不足」であって引数の誤り(3)では
        # ない（3 は引数不正・ファイル読取不能に限る）。
        raise inconclusive(
            f"run-meta.txt に profile がありません: {meta_path}",
            [
                "判定モードは profile で判定式⑷b の適用条件を決めます"
                f"（LONG_RUN_MIN_SPAN_SEC={LONG_RUN_MIN_SPAN_SEC:.0f} 秒に届かない走行のうち、"
                "profile=long は途中で切れた長時間走行として判定不能にする）。",
                "profile が無いまま判定すると、途中で切れた長時間走行が「適用外」として"
                "黙って終了コード 0 になります（要件 5.1・D7）。",
                "invoke-perf-run.ps1 は profile を必ず書き出します。"
                "手で作った run-meta.txt を使っている場合は profile を足してください。",
            ],
        )

    return Inputs(
        run_log=run_log,
        cpu_csv=cpu_csv,
        meta_path=meta_path,
        sections=sections,
        scan=scan,
        cpu=cpu,
        agg=aggregate(scan, cpu),
        build=build,
        profile=meta_profile,
    )


def render_inputs(report: Report, inputs: Inputs) -> None:
    report.head("[3] 入力")
    report.line(f"  実走ログ     : {inputs.run_log}")
    report.line(f"  CPU 時系列   : {inputs.cpu_csv}")
    report.line(f"  実行条件     : {inputs.meta_path}")
    stderr_log = inputs.run_log.parent / "run.stderr.log"
    report.line(
        f"  標準エラー   : {stderr_log}"
        + (
            f"（{stderr_log.stat().st_size} バイト）"
            if stderr_log.is_file()
            else "（なし）"
        )
    )
    report.line("    ※ 標準出力と標準エラーは同一ファイルへ流せないためランナーが分けている。")
    report.line("       必要ログ種ではないので、無くても集計は成立する。")


def render_required_check(report: Report, inputs: Inputs) -> None:
    scan, cpu, agg = inputs.scan, inputs.cpu, inputs.agg
    report.head("[4] 必要ログ種の検査（要件 2.5）")
    report.line(f"  perf サマリ行      : {len(scan.perf)} 本  → OK")
    report.line(f"  表示成立点 info!   : {scan.show_count} 本  → OK")
    report.line(f"  CPU 時系列         : {len(cpu)} 点  → OK")
    report.line(
        f"  実行条件           : {sum(len(items) for _t, items in inputs.sections)} 項目 → OK"
    )
    report.line(
        f"  perf 行の全 {len(J_PERF_REQUIRED_FIELDS)} フィールドが全行に揃っていることを確認済み。"
    )
    if scan.first_at and scan.last_at:
        report.line(
            f"  ログの時間幅       : {scan.first_at.isoformat()} 〜 {scan.last_at.isoformat()}"
        )
    report.line(
        f"  定常状態の開始     : {agg.steady_from.isoformat()}"
        f"（最初の perf 行から {WARMUP_EXCLUDE_SEC:.0f} 秒後）"
    )


# =============================================================================
# 集計モード
# =============================================================================


def run_baseline(args: argparse.Namespace) -> int:
    inputs = load_inputs(args, require_build=False)
    sections, scan, agg, build = inputs.sections, inputs.scan, inputs.agg, inputs.build

    report = Report()
    report.line("=" * 78)
    report.line("areka 性能計測 集計レポート（judge-perf.py）")
    report.line("=" * 78)
    report.line("  モード       : baseline（集計のみ・合否判定は --mode verdict）")
    report.line(f"  スクリプト版 : {SCRIPT_VERSION}")
    report.line(f"  ビルド種別   : {build or '(不明)'}")
    render_machine_conditions(report, sections, scan)
    render_calibration(report)

    render_inputs(report, inputs)
    render_required_check(report, inputs)
    if not agg.steady_perf:
        report.line(
            "  【注意】定常状態に perf 行が 1 本もありません。"
            "WARMUP_EXCLUDE_SEC が走行長に対して大きすぎます。"
        )

    render_stage_times(report, agg)
    render_cache_hits(report, agg)
    render_intervals(report, agg)
    render_allocs(report, agg)
    render_catchup(report, agg)
    render_cpu(report, agg)
    render_compose_keys(report, agg)
    render_seriko(report, agg)

    report.head("[13] 読むときの注意（較正が未確定の箇所）")
    caveats = [
        "コマ適用間隔の idle 窓の境界は未確定である（task 3.1 の第 1 段ベースラインで確定し"
        " README へ登記する）。窓 A と窓 B の両方を並べてあるのはそのためである。",
        "talk 再生区間を外す目印が未設定のため、talk 中の適用が窓 B に混ざっている。",
        "活性アニメ本数は発火・停止 info! の収支からの間接推定であり、直接の観測ではない。"
        "停止 info! が 1 本落ちると、そのアニメは以後ずっと走っている扱いになり、"
        "窓 B が走行の残り全部で閉じたままになる（判定式⑴で系列が丸ごと消える経路）。"
        "上の窓別収支が 0 に戻らない走行は、まずこれを疑うこと（task 3.1 で確定）。",
        f"WARMUP_EXCLUDE_SEC={WARMUP_EXCLUDE_SEC:.0f} 秒・"
        f"FRAME_INTERVAL_TOLERANCE={FRAME_INTERVAL_TOLERANCE:.0%} は暫定値である。",
    ]
    if not agg.steady_perf:
        caveats.insert(
            0,
            "【重大】定常状態に perf 行が 1 本もない。WARMUP_EXCLUDE_SEC が走行長に対して"
            "大きすぎるので、この走行から定常状態の数値は読めない。",
        )
    if len(agg.cpu) < CONVERGENCE_WINDOW_COUNT * 2:
        caveats.append(
            f"CPU の採取点が {len(agg.cpu)} 点しかなく、収束判定（要件 5.1）に足りない"
            "可能性がある。長時間水準で採り直すこと。"
        )
    for i, text in enumerate(caveats, start=1):
        report.line(f"  ({i}) {text}")

    report.line("")
    report.line("=" * 78)
    report.line("集計は以上。合否判定（要件 4.2 の判定式⑴〜⑷）と収束判定（要件 5.1）は")
    report.line("--mode verdict が行う。")
    report.line("=" * 78)

    print(report.render())
    return EXIT_OK


# =============================================================================
# 判定モード
# =============================================================================


def run_verdict(args: argparse.Namespace) -> int:
    """合否判定（要件 4.2 の判定式⑴〜⑷・4.3・4.4）と収束判定（要件 5.1・5.3）。"""
    inputs = load_inputs(args, require_build=True)
    agg, build = inputs.agg, inputs.build
    assert build is not None  # require_build=True が保証する

    steady = steady_state_status(agg)
    convergence = judge_convergence(agg)
    results = [
        judge_frame_interval(agg),
        judge_catchup(agg, steady),
        judge_allocations(agg, steady),
        *judge_idle_cpu(agg, build, convergence, inputs.profile),
    ]
    code, summary = derive_exit_code(results)

    report = Report()
    report.line("=" * 78)
    report.line("areka 性能計測 合否判定レポート（judge-perf.py）")
    report.line("=" * 78)
    report.line("  モード       : verdict（要件 4.2 の判定式⑴〜⑷・収束判定 要件 5.1）")
    report.line(f"  スクリプト版 : {SCRIPT_VERSION}")
    report.line(f"  ビルド種別   : {build}")
    report.line(f"  水準         : {inputs.profile or '(run-meta.txt に profile 無し)'}")
    render_machine_conditions(report, inputs.sections, inputs.scan)
    render_calibration(report)
    render_inputs(report, inputs)
    render_required_check(report, inputs)

    report.sub("定常状態の成立（判定式⑵⑶の前提）")
    report.line(
        f"  定常状態の適用 {steady.applies} 回・時間幅 {steady.span_sec:.1f} 秒 → "
        + ("判定を支えられる" if steady.ok else "支えられない")
    )
    report.line(
        f"  下限: VERDICT_MIN_STEADY_APPLIES={VERDICT_MIN_STEADY_APPLIES} 回 / "
        f"VERDICT_MIN_STEADY_SEC={VERDICT_MIN_STEADY_SEC:.0f} 秒"
    )
    for note in steady.notes:
        report.line(f"  - {note}")

    render_verdict(report, results)
    render_convergence(report, agg, convergence)

    report.head("[7] 総合判定")
    for result in results:
        report.line(f"  {result.tag} {result.title}: {result.status}")
    report.line("")
    report.line(f"  総合判定の決め方: {VERDICT_PRECEDENCE}")
    report.line(f"  総合           : {summary}")
    report.line(f"  終了コード     : {code}")
    if code == EXIT_FAIL:
        report.line(
            "  要件 4.6: 残る最大内訳を特定して是正（要件 3）へ戻る。"
            "ただし残る最大内訳がキャッシュ不命中による再合成コストなら要件 7 の裁定ゲートへ回す。"
        )
        report.line("  下の参考集計（段階別所要・命中率）が内訳特定の材料である。")
    if code == EXIT_INCONCLUSIVE:
        report.line(
            "  判定不能は合格ではない。上の各判定式の「空のときは」の記述に従い、"
            "観測を採り直すこと。"
        )

    report.head("[8] 参考: 集計（design.md「集計（両モード共通・R2.2）」）")
    report.line("  以下は判定の根拠を追うための集計である。判定そのものは上の [5][6] にある。")
    # 節番号は 1 つのレポートの中で重複させない（task 7.x が節番号で引用するため）。
    # 集計モードでは [5]〜[12]、判定モードでは [8.1]〜[8.8] として出す。
    render_stage_times(report, agg, section="8.1")
    render_cache_hits(report, agg, section="8.2")
    render_intervals(report, agg, section="8.3")
    render_allocs(report, agg, section="8.4")
    render_catchup(report, agg, section="8.5")
    render_cpu(report, agg, section="8.6")
    render_compose_keys(report, agg, section="8.7")
    render_seriko(report, agg, section="8.8")

    report.line("")
    report.line("=" * 78)
    report.line(f"判定は以上。終了コード {code}（{summary}）")
    report.line("=" * 78)

    print(report.render())
    return code


# =============================================================================
# 自己較正（--selftest・要件 2.4 / 2.6・design.md「judge-perf.py → 自己較正」・D7）
# -----------------------------------------------------------------------------
# 【何のためにあるか】判定の道具そのものが壊れていないことを、既知の入力で毎回確かめる。
# 緑は道具が壊れていても出る。ゆえに合格を再現する fixture と同格に、不合格・判定不能を
# 再現する fixture を置き、**そのどちらも実際に走って期待どおりになった** ことを出力に出す。
# 「全部通った」と「1 つも走らなかった」が見分けられない自己較正は、無いのと同じである。
#
# 【実運用と同じ経路を通ること】fixture ごとに `main(argv)` を呼ぶ。判定用の関数を
# 直接叩く別経路を作らない——別経路を作ると、入口（引数の検査・モードの振り分け・
# 例外から終了コードへの変換）が壊れても自己較正は緑のままになる。
#
# 【期待値の所在】各 fixture ディレクトリの `case.txt`。fixture と期待値を同じ場所に
# 置いてあるので、片方だけが更新されて静かにずれることがない。
# =============================================================================


@dataclass
class SelftestCase:
    """fixture 1 件（`case.txt` の中身そのもの）。"""

    name: str
    directory: Path
    title: str
    mode: str
    build: str | None
    expected: int
    twin: str | None

    def argv(self) -> list[str]:
        args = [
            str(self.directory / "run.log"),
            str(self.directory / "cpu.csv"),
            "--mode",
            self.mode,
        ]
        if self.build:
            args += ["--build", self.build]
        return args


def _selftest_read_case(directory: Path) -> SelftestCase:
    """`case.txt` を読む。読めない・欠けているのは fixture 置き場の不備（exit 3）。"""
    path = directory / SELFTEST_CASE_FILENAME
    if not path.is_file():
        raise bad_input(
            f"fixture に {SELFTEST_CASE_FILENAME} がありません: {directory}",
            [
                "期待終了コードの書いていない fixture は、走らせても合否を言えません"
                "（誰にも数えられない fixture を置かないこと）。",
            ],
        )
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        raise bad_input(f"{path} を読めません", [str(exc)]) from exc

    fields: dict[str, str] = {}
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#") or "=" not in stripped:
            continue
        key, _, value = stripped.partition("=")
        key = key.strip()
        if key == "note":  # 人が読む説明。機械は使わない
            continue
        fields[key] = value.strip()

    missing = [name for name in ("mode", "exit") if name not in fields]
    if missing:
        raise bad_input(
            f"{path} に {'・'.join(missing)} がありません",
            [f"必須項目: mode・exit（任意: build・twin・title・note）"],
        )
    if fields["mode"] not in MODES:
        raise bad_input(
            f"{path} の mode が不正です（{fields['mode']!r}）",
            [f"指定できるのは {'・'.join(sorted(MODES))} です。"],
        )
    try:
        expected = int(fields["exit"])
    except ValueError as exc:
        raise bad_input(f"{path} の exit が整数ではありません（{fields['exit']!r}）") from exc

    return SelftestCase(
        name=directory.name,
        directory=directory,
        title=fields.get("title", "(説明なし)"),
        mode=fields["mode"],
        build=fields.get("build") or None,
        expected=expected,
        twin=fields.get("twin") or None,
    )


def _selftest_collect(root: Path) -> list[SelftestCase]:
    if not root.is_dir():
        raise bad_input(
            f"fixture 置き場がありません: {root}",
            [
                "自己較正は判定スクリプトと同じ場所の "
                f"{SELFTEST_FIXTURES_DIRNAME}/ を見ます（相対位置のみ・絶対パスは持ちません）。",
            ],
        )
    cases = [
        _selftest_read_case(child)
        for child in sorted(root.iterdir())
        if child.is_dir() and not child.name.startswith((".", "_"))
    ]
    if not cases:
        raise bad_input(
            f"fixture が 1 件もありません: {root}",
            ["fixture の無い自己較正は、何も確かめずに緑を返します（D7）。"],
        )
    return cases


def _selftest_invoke(case: SelftestCase) -> tuple[int, str]:
    """実運用と同じ入口 `main(argv)` を呼び、終了コードと標準出力を持ち帰る。"""
    import contextlib
    import io

    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        try:
            code = main(case.argv())
        except SystemExit as exc:  # 引数解析が直接終了する経路も終了コードとして拾う
            code = exc.code if isinstance(exc.code, int) else EXIT_BAD_INPUT
    return code, out.getvalue()


def _selftest_normalize(case: SelftestCase, text: str) -> str:
    """双子どうしのレポート比較のため、fixture 名（＝入力パスの違い）だけを伏せる。"""
    return text.replace(case.name, "<FIXTURE>")


def run_selftest() -> int:
    """`fixtures/` の既知ログで終了コードを逐語再現する（要件 2.4）。"""
    root = Path(__file__).resolve().parent / SELFTEST_FIXTURES_DIRNAME
    cases = _selftest_collect(root)

    report = Report()
    report.line("=" * 78)
    report.line("judge-perf.py 自己較正（--selftest・要件 2.4 / 2.6・D7）")
    report.line("=" * 78)
    report.line(f"  スクリプト版   : {SCRIPT_VERSION}")
    report.line(f"  fixture 置き場 : {root}")
    report.line(f"  期待値の所在   : 各 fixture の {SELFTEST_CASE_FILENAME}")
    report.line("  経路           : 実運用と同じ入口 main(argv) を fixture ごとに呼ぶ")
    report.line("  終了コード     : 0=全件一致 / 1=1 件でも食い違い / 3=fixture を読めない")

    outputs: dict[str, str] = {}
    results: list[tuple[SelftestCase, int, bool]] = []
    report.sub("逐語再現（期待終了コードとの突き合わせ）")
    for case in cases:
        code, stdout = _selftest_invoke(case)
        outputs[case.name] = stdout
        ok = code == case.expected
        results.append((case, code, ok))
        build = f"/{case.build}" if case.build else ""
        report.line(
            f"  {'一致  ' if ok else '不一致'} {case.name:<40s} "
            f"期待 {case.expected} 実際 {code}  [{case.mode}{build}]  {case.title}"
        )

    # --- 双子の突き合わせ（色付きログが色なしと同じ結論になること）-----------
    twin_checks: list[tuple[str, str, bool, str]] = []
    for case in cases:
        if not case.twin:
            continue
        partner = next((c for c in cases if c.name == case.twin), None)
        if partner is None:
            twin_checks.append(
                (case.name, case.twin, False, f"双子 {case.twin} が fixture 置き場にありません")
            )
            continue
        mine = _selftest_normalize(case, outputs[case.name])
        theirs = _selftest_normalize(partner, outputs[partner.name])
        if mine == theirs:
            twin_checks.append((case.name, partner.name, True, "レポートが一字一句一致"))
        else:
            first = next(
                (
                    f"{i} 行目: {a!r} ≠ {b!r}"
                    for i, (a, b) in enumerate(
                        zip(mine.splitlines(), theirs.splitlines()), start=1
                    )
                    if a != b
                ),
                "行数が違う",
            )
            twin_checks.append((case.name, partner.name, False, f"レポートが違う（{first}）"))
    if twin_checks:
        report.sub("双子の突き合わせ（色を落とした結果が色なしと同一になること）")
        for name, partner, ok, detail in twin_checks:
            report.line(f"  {'一致  ' if ok else '不一致'} {name} ⇔ {partner}: {detail}")

    # --- 赤の実施状況（D7: 毎回赤も作る）------------------------------------
    report.sub("赤の実施状況（緑だけの自己較正は較正になっていない・D7）")
    labels = {
        EXIT_OK: "0 合格",
        EXIT_FAIL: "1 不合格",
        EXIT_INCONCLUSIVE: "2 判定不能",
        EXIT_BAD_INPUT: "3 引数不正・読取不能",
    }
    red_missing: list[str] = []
    for code in sorted(labels):
        registered = [c for c, _got, _ok in results if c.expected == code]
        reproduced = [c for c, _got, ok in results if c.expected == code and ok]
        report.line(
            f"  期待 {labels[code]:<22s}: 登録 {len(registered)} 件 / "
            f"実際に再現 {len(reproduced)} 件"
            + (f"  （{'・'.join(c.name for c in reproduced)}）" if reproduced else "")
        )
        if code in SELFTEST_REQUIRED_RED_CODES and not reproduced:
            red_missing.append(labels[code])

    mismatches = [(c, got) for c, got, ok in results if not ok]
    twin_failed = [name for name, _p, ok, _d in twin_checks if not ok]

    report.head("自己較正の結果")
    report.line(f"  fixture {len(results)} 件中 {len(results) - len(mismatches)} 件が期待どおり。")
    for case, got in mismatches:
        report.line(
            f"  【食い違い】{case.name}: 期待 {case.expected} に対し実際 {got}"
            f"（{case.mode}{'/' + case.build if case.build else ''}・{case.title}）"
        )
    for name in twin_failed:
        report.line(f"  【食い違い】{name}: 色なしの双子とレポートが一致しません。")
    for label in red_missing:
        report.line(
            f"  【較正になっていない】期待「{label}」を再現した fixture が 1 件もありません。"
            "赤が出ない自己較正は、道具が壊れていても緑を返します（D7）。"
        )

    code = EXIT_OK if not (mismatches or twin_failed or red_missing) else EXIT_FAIL
    if code == EXIT_OK:
        report.line("  合格・不合格・判定不能・引数不正のいずれも期待どおり再現しました。")
    else:
        report.line(
            "  判定スクリプトか fixture のどちらかが変わっています。"
            "実走の判定に使う前に、どちらが正しいかを決めてください。"
        )
    report.line("")
    report.line("=" * 78)
    report.line(f"自己較正は以上。終了コード {code}")
    report.line("=" * 78)
    print(report.render())
    return code


#: モードの登録簿。
#: `--selftest` はこの登録簿ではなく `main(argv)` を入口として使う——
#: fixture ごとに `main([run.log, cpu.csv, "--mode", …, "--build", …])` を呼び、
#: 返り値（終了コード）を期待値と突き合わせるので、実運用と同じ経路を逐語再現できる。
MODES = {
    "baseline": run_baseline,
    "verdict": run_verdict,
}

#: まだ実装していないモード（求められたら黙らずに理由を述べて終了する）。
PLANNED_MODES: dict[str, str] = {}


# =============================================================================
# 入口
# =============================================================================


class Parser(argparse.ArgumentParser):
    """引数不正の終了コードを 3 に揃える（argparse の既定は 2＝判定不能と衝突する）。"""

    def error(self, message: str):  # type: ignore[override]
        sys.stderr.write(f"[judge-perf] 引数が不正です: {message}\n")
        sys.stderr.write(self.format_usage())
        raise SystemExit(EXIT_BAD_INPUT)


def build_parser() -> Parser:
    parser = Parser(
        prog="judge-perf.py",
        description="areka 性能計測の集計・判定（要件 2.2/2.5/2.6/4.5/7.2）",
    )
    # `--selftest` は run.log / cpu.csv / --mode を取らない（fixture 側が全部持っている）。
    # ゆえに必須指定を argparse から外し、下の `_require_run_arguments` で明示的に検査する。
    # 検査に落ちたときの終了コードは従来どおり 3（Parser.error 経由）である。
    parser.add_argument("run_log", nargs="?", help="実走ログ run.log のパス")
    parser.add_argument("cpu_csv", nargs="?", help="CPU 時系列 cpu.csv のパス")
    parser.add_argument(
        "--mode",
        choices=sorted(MODES) + sorted(PLANNED_MODES),
        help="baseline=集計／verdict=合否判定（要件 4.2 の判定式⑴〜⑷・収束判定 要件 5.1）",
    )
    parser.add_argument(
        "--selftest",
        action="store_true",
        help=f"自己較正: {SELFTEST_FIXTURES_DIRNAME}/ の既知ログで期待終了コードを逐語再現する"
        "（要件 2.4・run.log / cpu.csv の指定は不要）",
    )
    parser.add_argument(
        "--build",
        choices=("dev", "release"),
        help="ビルド種別。省略時は run-meta.txt の build を使う",
    )
    parser.add_argument(
        "--meta",
        help="実行条件 run-meta.txt のパス（省略時は cpu.csv と同じディレクトリを探す）",
    )
    return parser


def main(argv: list[str]) -> int:
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[union-attr]
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[union-attr]
    except (AttributeError, OSError):
        pass

    parser = build_parser()
    args = parser.parse_args(argv)

    if args.selftest:
        if args.run_log or args.cpu_csv or args.mode or args.build or args.meta:
            parser.error(
                "--selftest は他の引数と一緒に使えません"
                "（fixture 側が入力もモードもビルド種別も持っています）"
            )
        try:
            return run_selftest()
        except JudgeError as exc:
            sys.stderr.write(f"[judge-perf] 自己較正を開始できません: {exc.message}\n")
            for detail in exc.details:
                sys.stderr.write(f"  - {detail}\n")
            sys.stderr.write(f"[judge-perf] 終了コード {exc.code}\n")
            return exc.code

    # --selftest でないなら、実走ログ・CPU 時系列・モードは必須である
    # （argparse の必須指定から外した分をここで明示的に見る。落ちたら従来どおり exit 3）。
    lacking = [
        name
        for name, value in (
            ("run_log", args.run_log),
            ("cpu_csv", args.cpu_csv),
            ("--mode", args.mode),
        )
        if not value
    ]
    if lacking:
        parser.error(f"次の指定がありません: {'・'.join(lacking)}")

    if args.mode in PLANNED_MODES:
        sys.stderr.write(f"[judge-perf] {PLANNED_MODES[args.mode]}\n")
        return EXIT_BAD_INPUT

    try:
        return MODES[args.mode](args)
    except JudgeError as exc:
        label = {
            EXIT_INCONCLUSIVE: "判定不能",
            EXIT_BAD_INPUT: "入力不正",
        }.get(exc.code, "失敗")
        sys.stderr.write(f"[judge-perf] {label}: {exc.message}\n")
        for detail in exc.details:
            sys.stderr.write(f"  - {detail}\n")
        sys.stderr.write(f"[judge-perf] 部分集計は出していません。終了コード {exc.code}\n")
        return exc.code


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
