"""新しい判定の検体の組み立て（generate.py の main() が実行時に読み込む）。

generate.py 側は 1,000 行の上限（D11）があるので、増やす検体はここに置く。
generate を import するので、generate からは main() の中でだけ読むこと。
"""
from __future__ import annotations

from pathlib import Path

import generate as g


def _healthy() -> g.Fixture:
    """H11 の芯（合格する走行）。CPU 点は呼び手が足す。"""
    return (
        g.Fixture("")
        .boot()
        .cycle_series(g.JUDGED_TARGET, g.JUDGED_SURFACE, g.MAIN_WAITS_MS, g.MAIN_CYCLES)
        .run_meta()
    )


def case_N1() -> g.Fixture:
    f = _healthy().cpu_series(count=12).cpu_point(250.0, float("nan"))
    f.name, f.dir = "N1_cpu_nan", g.HERE / "N1_cpu_nan"
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


def _named(f: g.Fixture, name: str) -> g.Fixture:
    f.name, f.dir = name, g.HERE / name
    return f


def _long(name: str, count: int, values=None) -> g.Fixture:
    f = _healthy().cpu_series(count=count)
    if values:
        f.cpu = []
        for i, v in enumerate(values):
            f.cpu_point(62.0 + 15.0 * i, v)
    f.run_meta(profile="long")
    return _named(f, name)


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


def case_V1() -> g.Fixture:
    f = _healthy().cpu_series(count=12, value=4.0).run_meta(build="dev")
    return _named(f, "V1_dev_not_applicable").declare(
        "verdict", 0, "dev ビルドでは CPU の数値目標は適用外", build="dev",
        notes=[
            "build = dev。CPU 4.0% は release なら ⑷a で不合格（1）になる値。",
            "⑷a・⑷b は release だけの判定なので dev では適用外とし、ほかが健全なら合格（0）になる。",
        ],
    )


def _explicit(name: str, wait_ms: float, cycles: int) -> g.Fixture:
    # 周期 4 秒にすると 20 本で定常が 60 秒を越える（1 秒周期だと届かない）。
    f = (g.Fixture("").boot()
         .cycle_series(g.JUDGED_TARGET, g.JUDGED_SURFACE, (wait_ms,), cycles,
                       cycle_period_ms=4000.0)
         .cpu_series().run_meta())
    return _named(f, name)


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
    return _explicit("S3_explicit_over_limit", 200.0, 20).declare(
        "verdict", 1, "明示指定の系列が上限を越えたら不合格", build="release",
        notes=["S2 の待ちを 200 ms（上限 172.5 ms 越え）にした。不合格（1）になる。"],
    )


def case_B2() -> g.Fixture:
    f = (g.Fixture("").boot().series(g.JUDGED_TARGET, g.JUDGED_SURFACE, 150.0, 40)
         .cpu_series(count=13).run_meta())
    return _named(f, "B2_thin_steady_verdict").declare(
        "verdict", 2, "定常状態が薄い走行は合否判定では判定不能", build="release",
        notes=[
            "B1 と同じ入力を合否判定で読む。定常の時間幅が下限の 60 秒に届かないので、",
            "判定不能（2）になり、理由に薄い定常の文が出る。",
        ],
        contains=("秒しかありません（下限 VERDICT_MIN_STEADY_SEC=60 秒）",),
    )


def case_X3() -> g.Fixture:
    # 実行条件も debug にして、引数の choices だけが 3 の理由になるようにする。
    f = _healthy().cpu_series(count=12).run_meta(build="debug")
    return _named(f, "X3_build_debug_arg").declare(
        "verdict", 3, "build = debug は引数の誤り", build="debug",
        notes=["判定の build に debug は無い（argparse の choices が拒む）。終了コード 3。"],
    )


def case_K1() -> g.Fixture:
    f = _healthy().cpu_series(count=12).cpu_point(12.0, 9.0).cpu_point(92.5, 4.2)
    for sec, event in ((2.0, "boot_talk"), (8.0, "boot_talk_done")):
        at = g.T0 + g.timedelta(seconds=sec)
        f._add(at, f'{g.ts(at)}  INFO kanade: 起動挨拶 event="{event}" talk_id=1')
    f.talk_span(90.0, 95.0)
    return _named(f, "K1_boot_talk_excluded").declare(
        "baseline", 0, "起動の挨拶（boot_talk）は boot_talk_done で閉じ、その後の CPU の山を発話中の頂に巻き込まない", build="release",
        emit_metrics=True,
        notes=[
            "boot_talk（2 秒）〜boot_talk_done（8 秒）で区間が閉じ、12 秒の CPU 9.00 は区間の外（起動の挨拶の後）。",
            "boot_talk_done が終わりの組に無いと区間が最後まで開き、後の発話の頂 4.20 が 9.00 に化ける。",
        ],
        contains=("metric=talk_peak_cpu_pct value=4.20",),
    )


CASES = (case_N1, case_U1, case_L1, case_L2, case_L3, case_V1,
         case_S1, case_S2, case_S3, case_B2, case_X3, case_K1)
