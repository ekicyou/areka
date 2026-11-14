"""新しい判定の検体の組み立て（generate.py の main() が実行時に読み込む）。

generate.py 側は着手時の 1,244 行を越えない決まり（D11）があるので、増やす検体はここに置く。
generate を import するので、generate からは main() の中でだけ読むこと。
"""
from __future__ import annotations

from pathlib import Path

import generate as g


def _healthy(name: str) -> g.Fixture:
    """H11 の芯（合格する走行）。CPU 点は呼び手が足す。"""
    return (
        g.Fixture(name)
        .boot()
        .cycle_series(g.JUDGED_TARGET, g.JUDGED_SURFACE, g.MAIN_WAITS_MS, g.MAIN_CYCLES)
        .run_meta()
    )


def _at(sec: float):
    return g.T0 + g.timedelta(seconds=sec)


def _edit_perf_row(f: g.Fixture, sec: float, edit) -> g.Fixture:
    """`sec` 秒以降で最初の perf 行（定常状態の中）を 1 本だけ `edit` で書き換える。"""
    i = next(i for i, (at, _n, text) in enumerate(f.rows) if at >= _at(sec) and "perf(apply_show)" in text)
    at, n, text = f.rows[i]
    f.rows[i] = (at, n, edit(text))
    return f


def case_N1() -> g.Fixture:
    f = _healthy("N1_cpu_nan").cpu_series(count=12).cpu_point(250.0, float("nan"))
    return f.declare(
        "verdict", 2, "有限でない CPU の値は壊れた行（判定不能）", build="release",
        notes=[
            "cpu.csv に nan の値が 1 行ある。有限でない値を数として読むと、比べが全部",
            "偽になって平均や傾きが黙って狂う。壊れた行として数え、判定不能（2）にする。",
        ],
    )


class _BomFixture(g.Fixture):
    def write(self) -> Path:
        path = super().write()
        # run.log の先頭行は検索で読まれ BOM に鈍いので、先頭が厳密一致の cpu.csv にも付ける。
        for name in ("run.log", "cpu.csv"):
            f = path / name
            f.write_bytes(b"\xef\xbb\xbf" + f.read_bytes())
        return path


def case_U1() -> g.Fixture:
    f = _BomFixture("U1_bom_run_log")
    f.boot().cycle_series(g.JUDGED_TARGET, g.JUDGED_SURFACE, g.MAIN_WAITS_MS, g.MAIN_CYCLES)
    return f.cpu_series().run_meta().declare(
        "verdict", 0, "BOM 付きの run.log・cpu.csv も同じに読める", build="release",
        notes=[
            "run.log と cpu.csv の頭に UTF-8 の BOM（EF BB BF）を付けた、H11 の芯と同じ健全な走行。",
            "run.log は探して読むので BOM に左右されないが、BOM を字のまま残すと cpu.csv の",
            "見出しが一致しなくなり判定不能（2）になる。読むときに BOM を外して合格（0）に",
            "なることを縛る。",
        ],
    )


def _long(name: str, count: int, values=None) -> g.Fixture:
    f = _healthy(name).cpu_series(count=count)
    if values:
        f.cpu = []
        for i, v in enumerate(values):
            f.cpu_point(62.0 + 15.0 * i, v)
    return f.run_meta(profile="long")


def case_L1() -> g.Fixture:
    return _long("L1_long_flat_pass", 89).declare(
        "verdict", 0, "長い走行（20 分超）で平坦な CPU は合格", build="release",
        notes=[
            "profile = long・CPU 89 点（15 秒おき・1,320 秒・1.50%）。20 分の下限を越え、",
            "平均も傾きも基準の内側なので、判定式⑷b まで適用されて合格（0）になる。",
        ],
    )


def case_L2() -> g.Fixture:
    vals = [1.0 + 1.8 * i / 88 for i in range(89)]
    return _long("L2_long_rising_fail", 89, vals).declare(
        "verdict", 1, "長い走行で CPU が上り続けると⑷b だけ不合格", build="release",
        notes=[
            "89 点を 1.0 から 2.8% へ直線で上げた。平均は 3.0% の内側だが傾きは",
            "0.08 %/分で、収束の基準 0.05 %/分を越える。⑷b だけが不合格（1）になる。",
        ],
    )


def case_L3() -> g.Fixture:
    return _long("L3_long_too_short", 25).declare(
        "verdict", 2, "長い走行を名乗るのに 20 分に届かない走行は判定不能", build="release",
        notes=[
            "profile = long なのに CPU は 25 点（360 秒）しかない。⑷b を適用する条件の",
            "20 分に届かないので、黙って通さず判定不能（2）にする。",
        ],
    )


def case_L4() -> g.Fixture:
    vals = [1.0] * 44 + [1.5] * 45
    return _long("L4_long_window_diff_fail", 89, vals).declare(
        "verdict", 1, "長い走行で末尾窓と中間窓の差だけが上限を越えると不合格", build="release",
        notes=[
            "89 点のうち窓 1・2 の 44 点を 1.0%、窓 3・4 の 45 点を 1.5% にした。後半窓の中は",
            "平らで傾きは 0（上限 0.05 %/分の内側）、末尾窓 − 中間窓は 0.50%（上限 0.3% 越え）。",
            "⑷b の 2 つの条件のうち窓の差だけで不合格（1）になる（L2 は両方を越える）。",
        ],
    )


def case_L5() -> g.Fixture:
    vals = [1.5] * 44 + [1.0 + 1.0 * k / 44 for k in range(45)]
    return _long("L5_long_slope_fail", 89, vals).declare(
        "verdict", 1, "長い走行で後半窓の傾きだけが上限を越えると不合格", build="release",
        notes=[
            "窓 1・2 の 44 点を 1.5%、窓 3・4 の 45 点を 1.0% から 2.0% へ直線で上げた。",
            "後半窓の傾きは約 0.09 %/分（上限 0.05 越え）、末尾窓 − 中間窓は約 0.25%（上限 0.3% の内側）。",
            "⑷b の 2 つの条件のうち傾きだけで不合格（1）になる（傾きの符号を逆に読むと合格に化ける）。",
        ],
    )


def case_V1() -> g.Fixture:
    return _healthy("V1_dev_not_applicable").cpu_series(count=12, value=4.0).run_meta(build="dev").declare(
        "verdict", 0, "dev ビルドでは CPU の数値目標は適用外", build="dev",
        notes=[
            "build = dev。CPU 4.0% は release なら ⑷a で不合格（1）になる値。",
            "⑷a・⑷b は release だけの判定なので dev では適用外とし、ほかが健全なら合格（0）になる。",
        ],
    )


def _explicit(name: str, wait_ms: float, cycles: int) -> g.Fixture:
    # 周期 4 秒にすると 20 本で定常が 60 秒を越える（1 秒周期だと届かない）。
    return (g.Fixture(name).boot()
            .cycle_series(g.JUDGED_TARGET, g.JUDGED_SURFACE, (wait_ms,), cycles,
                          cycle_period_ms=4000.0)
            .cpu_series().run_meta())


def case_S1() -> g.Fixture:
    return _explicit("S1_explicit_19_intervals", 150.0, 19).declare(
        "verdict", 2, "明示指定の系列の間隔が 19 本では判定不能", build="release",
        notes=[
            "判定対象の系列の窓の間隔が 19 本。下限の 20 本に 1 本足りないので、",
            "合否を出さず判定不能（2）にする。S2 と対で境目を固定する。",
        ],
    )


def case_S2() -> g.Fixture:
    return _explicit("S2_explicit_20_intervals", 150.0, 20).declare(
        "verdict", 0, "明示指定の系列の間隔がちょうど 20 本なら判定できる", build="release",
        notes=["S1 と同じ形で間隔が 20 本。下限ちょうどなので判定でき、合格（0）になる。"],
    )


def case_S3() -> g.Fixture:
    return _explicit("S3_explicit_over_limit", 200.0, 19).declare(
        "verdict", 1, "明示指定の系列が 20 本に届かなくても上限を越えたら不合格", build="release",
        notes=[
            "S1 と同じ 19 本で、待ちを 200 ms（上限 172.5 ms 越え）にした。間隔が下限の 20 本に",
            "届かない薄さは合格だけを止める。上限越えは判定不能より先に不合格（1）になる。",
        ],
    )


def case_B2() -> g.Fixture:
    f = (g.Fixture("B2_thin_steady_verdict").boot()
         .series(g.JUDGED_TARGET, g.JUDGED_SURFACE, 150.0, 40)
         .cpu_series(count=13).run_meta())
    return f.declare(
        "verdict", 2, "定常状態が薄い走行は合否判定では判定不能", build="release",
        notes=[
            "B1 と同じ入力を合否判定で読む。定常の時間幅が下限の 60 秒に届かないので、",
            "判定不能（2）になり、理由に薄い定常の文が出る。",
        ],
        contains=("秒しかありません（下限 VERDICT_MIN_STEADY_SEC=60 秒）",),
    )


def case_X3() -> g.Fixture:
    # 実行条件も debug にして、引数の choices だけが 3 の理由になるようにする。
    return _healthy("X3_build_debug_arg").cpu_series(count=12).run_meta(build="debug").declare(
        "verdict", 3, "build = debug は引数の誤り", build="debug",
        notes=["判定の build に debug は無い（argparse の choices が拒む）。終了コード 3。"],
    )


def case_K1() -> g.Fixture:
    f = _healthy("K1_boot_talk_excluded").cpu_series(count=12).cpu_point(12.0, 9.0).cpu_point(92.5, 4.2)
    for sec, event in ((2.0, "boot_talk"), (8.0, "boot_talk_done")):
        f._add(_at(sec), f'{g.ts(_at(sec))}  INFO kanade: 起動挨拶 event="{event}" talk_id=1')
    f.talk_span(90.0, 95.0)
    return f.declare(
        "baseline", 0, "起動の挨拶（boot_talk）は boot_talk_done で閉じ、その後の CPU の山を発話中の頂に巻き込まない", build="release",
        emit_metrics=True,
        notes=[
            "boot_talk（2 秒）〜boot_talk_done（8 秒）で区間が閉じ、12 秒の CPU 9.00 は区間の外（起動の挨拶の後）。",
            "boot_talk_done が終わりの組に無いと区間が次の終わり（95 秒の steady_talk_done）まで開き、",
            "後の発話の頂 4.20 が 9.00 に化ける。",
        ],
        contains=("metric=talk_peak_cpu_pct value=4.20",),
    )


def case_K2() -> g.Fixture:
    f = _healthy("K2_talk_spans_closed_and_open").cpu_series(count=12)
    f.talk_span(80.0, 85.0).cpu_point(82.0, 3.0).cpu_point(87.0, 9.0).cpu_point(91.0, 6.0)
    f._add(_at(90.0), g.talk_line(_at(90.0), "steady_talk"))
    f._add(_at(92.0), g.talk_line(_at(92.0), "steady_talk").replace('"steady_talk"', '"steady_talk_replace"'))
    return f.declare(
        "baseline", 0, "発話区間は終わりで閉じ、入れ子の開始は畳み、終わりの無い発話は走行の最後で閉じる", build="release",
        emit_metrics=True,
        notes=[
            "steady_talk（80 秒）〜steady_talk_done（85 秒）の中に CPU 3.00、その後（87 秒）に 9.00。",
            "90 秒の steady_talk の中で 92 秒に steady_talk_replace（再生中の差し替え）があり、",
            "終わりの記録が無いまま走行が終わる。91 秒の 6.00 が発話中の頂になる。",
            "steady_talk_done で閉じないと 9.00、差し替えで区間を開き直すと 3.00、",
            "最後に閉じずに捨てると 3.00 に化ける。",
        ],
        contains=("metric=talk_peak_cpu_pct value=6.00",),
    )


def _profile(name: str, profile: str) -> g.Fixture:
    return _healthy(name).cpu_series(count=12).run_meta(profile=profile)


def case_M1() -> g.Fixture:
    return _profile("M1_profile_empty", "").declare(
        "verdict", 2, "run-meta.txt の profile が空なら判定不能", build="release",
        notes=[
            "profile = （空）。short か long か分からないので判定式⑷b の適用条件を決められない。",
            "空を「書いてある」と読むと適用外へ落ちて合格（0）になるので、判定不能（2）にする。",
        ],
    )


def case_M2() -> g.Fixture:
    return _profile("M2_profile_unknown", "medium").declare(
        "verdict", 2, "run-meta.txt の profile が short・long 以外なら判定不能", build="release",
        notes=["profile = medium。M1 と同じ理由で、知らない値も判定不能（2）にする。"],
    )


def _undated_catchup(name: str) -> g.Fixture:
    f = _healthy(name).cpu_series(count=12)
    f._add(_at(90.0), g.catchup_line(_at(90.0), "kanade").split("  ", 1)[1])  # 行頭の時刻を外す
    # 起動過渡に、欄が target だった頃の綴りの行と、欄の無い行を 1 本ずつ置く。
    for sec, field in ((30.0, 'target="dispatcher"'), (31.0, "")):
        f._add(_at(sec), g.catchup_line(_at(sec), "dispatcher").replace('lane="dispatcher"', field).rstrip())
    return f


def case_C3() -> g.Fixture:
    return _undated_catchup("C3_undated_catchup_metric").declare(
        "baseline", 0, "時刻の読めない catch-up があれば catchup_count は - を出す", build="release",
        emit_metrics=True,
        notes=[
            "行頭に時刻の無い catch-up 行が 1 本ある。定常の中か外か分からないので、",
            "catchup_count は「測っていない」の - を出す（0 は「数えて 0」で、A/B の比べを欺く）。",
            "起動過渡の 2 本は、古い綴り target=\"dispatcher\" を dispatcher と読むことと、",
            "欄の無い行を既知の系統へ寄せず (lane 欄なし) と名指しすることを縛る。",
        ],
        contains=(
            "metric=catchup_count value=-",
            "metric=catchup_dispatcher value=1",
            "target=(lane 欄なし): 1 件",
        ),
    )


def case_C4() -> g.Fixture:
    return _undated_catchup("C4_undated_catchup_verdict").declare(
        "verdict", 2, "時刻の読めない catch-up がある走行は合否判定では判定不能", build="release",
        notes=[
            "C3 と同じ入力を合否判定で読む。定常状態に catch-up は 0 件だが、時刻の無い 1 本が",
            "定常の中か外か分からないので、判定式⑵ は「0 件」と言い切らず判定不能（2）にする。",
        ],
        contains=("時刻不明 1 件",),
    )


def case_E4() -> g.Fixture:
    f = _healthy("E4_run_meta_without_values").cpu_series(count=12)
    f.meta = ["# 名前 = 値 の行が 1 つも無い実行条件", "[実行]"]
    return f.declare(
        "baseline", 2, "run-meta.txt に名前 = 値 の行が無ければ判定不能",
        notes=[
            "run-meta.txt はあるが見出しだけで、マシン条件が 1 つも書いていない。レポートに",
            "マシン条件を載せられない（要件 4.5）ので、集計モードでも判定不能（2）にする。",
        ],
    )


def case_A1() -> g.Fixture:
    f = _edit_perf_row(_healthy("A1_steady_alloc_fail").cpu_series(count=12), 100.0,
                       lambda t: t.replace("alloc_mask=0", "alloc_mask=1"))
    return f.declare(
        "verdict", 1, "定常状態で表示用バッファの確保が 1 回でもあれば不合格", build="release",
        notes=[
            "H11 の芯の定常状態の perf 行 1 本だけ alloc_mask=1 にした。判定式⑶ は定常状態の",
            "alloc_* 合計 0 を求めるので不合格（1）になり、確保の起きた適用を名指しする。",
        ],
        contains=("確保が起きた適用: 1 回",),
    )


def _bad_perf(name: str, edit, what: str) -> g.Fixture:
    f = _edit_perf_row(_healthy(name).cpu_series(count=12), 100.0, edit)
    return f.declare(
        "verdict", 2, f"perf 行の{what}は判定不能", build="release",
        notes=[
            f"H11 の芯の定常状態の perf 行 1 本を、{what}形にした（要件 2.5）。",
            "その行を黙って捨てると残りの行で合格（0）に化けるので、判定不能（2）にする。",
        ],
    )


def case_R1() -> g.Fixture:
    return _bad_perf("R1_perf_line_missing_field", lambda t: t.replace(" t_mask_us=0", ""),
                     "必須の欄 t_mask_us が欠けた")


def case_R2() -> g.Fixture:
    return _bad_perf("R2_perf_line_undated", lambda t: t.split(" ", 1)[1], "行頭に時刻が無い")


def case_R3() -> g.Fixture:
    return _bad_perf("R3_perf_line_cache_hit_not_bool",
                     lambda t: t.replace("cache_hit=true", "cache_hit=yes"), "cache_hit が true／false でない")


def _seriko(at, text: str) -> str:
    return f"{g.ts(at)} INFO actor{{actor=seriko}}: areka_seriko::looper: {text}"


def case_P15() -> g.Fixture:
    f = _healthy("P15_other_stop_messages").cpu_series(count=12)
    for sec, anim, message in (
        (64.0, "1401", "seriko: loop 末尾残留（最終コマ保持・再抽選対象へ・要件 4.4/9.4）"),
        (66.0, "1402", "seriko: loop bind から外れた ID の再生を停止（保持コマ除去・bindopt 7.3）"),
    ):
        f._add(_at(sec), g.fire_line(_at(sec), animation_id=anim))
        f._add(_at(sec + 1.0), _seriko(_at(sec + 1.0), f'{message} scope="0" slot=Shell animation_id={anim}'))
    return f.declare(
        "verdict", 0, "末尾残留と bind 外れの停止もアニメの終わりとして数える", build="release",
        notes=[
            "定常の系列の前に、同じ scope=\"0\" で 1401 を発火→末尾残留、1402 を発火→bind 外れの停止で",
            "終わらせた。どちらかを停止と読まないと、そのアニメが走り続けて活性 2 本になり、",
            "窓 C の条件 4 が判定対象の間隔を全部落として判定不能（2）に化ける。正しくは合格（0）。",
        ],
    )


CASES = (case_N1, case_U1, case_L1, case_L2, case_L3, case_L4, case_L5, case_V1,
         case_S1, case_S2, case_S3, case_B2, case_X3, case_K1, case_K2,
         case_M1, case_M2, case_C3, case_C4, case_E4, case_A1,
         case_R1, case_R2, case_R3, case_P15)
