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


CASES = (case_N1, case_U1)
