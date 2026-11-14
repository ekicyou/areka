#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""自己較正の支え（perf-rank・perf-compare・judge-followup・perf-ledger の 4 組が使う）。

4 組の自己較正に同じ形で写されていた部分（ケースのフォルダの並べ方・ケースの鍵の読み・
本体の `main` を同じプロセスで呼ぶ `invoke`・最初の違い・赤の下限と結果の行・一時フォルダ）
をここに置く（#41）。道具ごとの違い（鍵の一覧・ケースの回し方・期待出力の突き合わせ・
赤の値）は道具の側に残す。judge-perf の自己較正は形が違うので寄せない。

一時フォルダはリポジトリの `target\\perf-loop-tmp\\` の下に作る（#1・D1）。OS の一時
フォルダは使わない。このモジュールは道具を読まない（依存は「自己較正 → 支え」の一方向）。
"""

from __future__ import annotations

import contextlib
import io
import shutil
import sys
import uuid
from collections.abc import Callable, Iterator
from pathlib import Path

#: tools/perf/perf_selftest_common.py の 3 つ上（[0]=tools/perf・[1]=tools・[2]=根）。
REPO_ROOT = Path(__file__).resolve().parents[2]
if not (REPO_ROOT / "Cargo.toml").is_file():
    # 根を取り違えたまま target の下に作らない（.gitignore の target はどの深さにも当たり、git では見えない）。
    sys.stderr.write(f"[perf-selftest] repository root not found: no Cargo.toml in {REPO_ROOT}\n")
    raise SystemExit(3)
WORK_ROOT = REPO_ROOT / "target" / "perf-loop-tmp"

MISSING_LINE = "(no line)"

Fail = Callable[..., Exception]


@contextlib.contextmanager
def work_dir(prefix: str) -> Iterator[Path]:
    """`WORK_ROOT/<prefix><uuid4 の 16 進>` を作って渡し、抜けるときその子だけを消す。

    消せなかったら理由を stderr に 1 行出し、自己較正の結果は変えない（D1）。
    """
    WORK_ROOT.mkdir(parents=True, exist_ok=True)
    child = WORK_ROOT / f"{prefix}{uuid.uuid4().hex}"
    child.mkdir()
    try:
        yield child
    finally:
        try:
            shutil.rmtree(child)
        except OSError as exc:
            sys.stderr.write(f"[perf-selftest] could not remove work dir {child}: {exc}\n")


def list_case_dirs(root: Path, case_file: str, fail: Fail, *,
                   materials_allowed: bool) -> tuple[list[Path], list[str]]:
    """ケースのフォルダを名前順に並べる。`.` と `_` で始まるフォルダは飛ばす。

    `materials_allowed` が真なら `case_file` の無いフォルダを素材置き場として名だけ返す。
    偽ならどのフォルダもケースとして返す（`case_file` の無いことは道具の読みが拒む）。
    """
    if not root.is_dir():
        raise fail(f"自己較正のケース置き場がありません: {root}")
    cases: list[Path] = []
    materials: list[str] = []
    for child in sorted(root.iterdir()):
        if not child.is_dir() or child.name.startswith((".", "_")):
            continue
        if materials_allowed and not (child / case_file).is_file():
            materials.append(child.name)
        else:
            cases.append(child)
    if not cases:
        raise fail(
            f"自己較正のケースが 1 件もありません: {root}",
            ["ケースの無い自己較正は、何も確かめずに緑を返します。"],
        )
    return cases, materials


def read_case_fields(path: Path, allowed: frozenset[str], multi: frozenset[str],
                     fail: Fail) -> tuple[dict[str, str], dict[str, list[str]]]:
    """`名前 = 値` の行を読む。`#` の行と空行は飛ばす。知らない鍵は `fail`（道具の入力不正）。

    `multi` の鍵は何行でも書け、出てきた順に並ぶ。それ以外の鍵は最後の値が残る。
    """
    single: dict[str, str] = {}
    multiple: dict[str, list[str]] = {key: [] for key in multi}
    for lineno, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        key, separator, value = line.partition("=")
        if not separator:
            raise fail(f"{path}:{lineno}: `名前 = 値` の形で書きます（{line!r}）")
        key, value = key.strip(), value.strip()
        if key in multi:
            multiple[key].append(value)
        elif key in allowed:
            single[key] = value
        else:
            names = "・".join(sorted(allowed | multi))
            raise fail(f"{path}:{lineno}: 知らない項目です: {key!r}", [f"使えるのは {names} です。"])
    return single, multiple


def invoke(main: Callable[[list[str]], int], argv: list[str],
           exit_fallback: int) -> tuple[int, str, str]:
    """実運用と同じ入口 `main(argv)` を同じプロセスで呼ぶ（自己較正だけの近道は作らない）。"""
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        try:
            code = main(argv)
        except SystemExit as exc:
            code = exc.code if isinstance(exc.code, int) else exit_fallback
    return code, out.getvalue(), err.getvalue()


def first_difference(label: str, want: str, got: str) -> str:
    """最初に違う行を出す。枠の語も英語で、違う行は `ascii()` で出す（端末へは ASCII だけ・#25）。"""
    want_lines, got_lines = want.splitlines(), got.splitlines()
    for index in range(max(len(want_lines), len(got_lines))):
        a = want_lines[index] if index < len(want_lines) else MISSING_LINE
        b = got_lines[index] if index < len(got_lines) else MISSING_LINE
        if a != b:
            return f"{label} line {index + 1}: want {ascii(a)} got {ascii(b)}"
    return f"{label}: no difference"


def read_normalized(path: Path) -> str:
    """utf-8 で読み、CRLF を LF にそろえる。"""
    return path.read_text(encoding="utf-8").replace("\r\n", "\n")


def finish(results: list[tuple[str, str, list[str]]], materials: list[str], case_file: str,
           reproduced: set[int], required_red: tuple[int, ...]) -> int:
    """ケースごとの行・素材置き場の行・赤の下限の行・`SELFTEST RESULT` の行を出し、NG の数を返す。

    `results` は（ケース名・説明・食い違いの理由）。理由が空なら ok。
    """
    ok_count = ng_count = 0
    for name, title, reasons in results:
        if reasons:
            ng_count += 1
            print(f"[selftest] {name} NG  {title}")
            for reason in reasons:
                print(f"           - {reason}")
        else:
            ok_count += 1
            print(f"[selftest] {name} ok  {title}")

    for name in materials:
        print(f"[selftest] {name} --  {case_file} が無いので素材置き場として飛ばす")

    # 緑だけの自己較正は較正になっていない（道具が壊れていても緑は出る）。
    for code in required_red:
        if code not in reproduced:
            ng_count += 1
            print(f"[selftest] (赤の較正) NG  終了コード {code} を再現したケースが 1 件もありません")

    print(f"SELFTEST RESULT ok={ok_count} ng={ng_count}")
    return ng_count
