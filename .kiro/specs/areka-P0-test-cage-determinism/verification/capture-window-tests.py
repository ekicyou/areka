#!/usr/bin/env python3
"""共有捕捉窓を使うテストを**静的に**列挙する（タスク 11.2・要件 13.2 の 2026-08-27 追記）。

なぜ静的な列挙なのか
--------------------
起草時の設計 C9 は「常駐を無効にした側で赤になるテストを観測して除外する」だったが、
タスク 11.1 の実測で**赤の集合が非決定**であることが判明した（8 回の走行で不変に赤なのは
較正テスト 1 本だけ・他は 1/8〜5/8 で出入り・5 本除外すると 6 本目が出る）。無効側の赤は
毒化の競合が起こすもので、並列の巡り合わせに依存する。したがって除外集合は観測ではなく
ソースの走査で決める。

何を列挙するか
--------------
種となる語は 2 群。

  ⑴ 共有 crate `log-capture-kit` の**窓を開く公開 API** 5 本
     （`capture` / `capture_lines` / `count_levels` / `capture_under_filter` /
      `install_global_capture_all`）。`ensure_interest_probes` は窓ではないので入れない。
  ⑵ 共有機構を迂回する素の呼出 3 語（`with_default` / `set_default` / `set_global_default`）。

各 crate は薄い包み（`test_log_capture.rs` の `capture`、`placement/test_support.rs` の
`capture_logs` など）を持つので、**包みの名前を呼出語へ足す不動点**を回す。足すのは

  - `log_capture_kit` を参照するファイルの中の、
  - **テスト関数の外で定義された非テスト関数**で、その本体が窓を開く語を呼ぶもの

の名前だけ。さらにその語は**定義元の crate の中**（`tests/` の下で定義されたものはその
`tests/` の木の中）でしか当てない。この縛りが無いと、`install` のような一般名が別 crate の
無関係な `thread_roles::install()` に当たって偽陽性を出す（実測で 3 ファイル分の偽陽性が出た）。

**供給条件は 2026-08-27 の差し戻しで直した（この道具の 2 度目の穴）。** 初版は供給者を
「`log_capture_kit` を参照し `#[test]` を 1 本も持たないファイル」に限っていた。ところが
この repo では**包みファイルが自己テストを持つのが定石**で、
`crates/areka-emo-atlas/src/log_capture.rs:34` と `crates/areka-emo-compose/src/log_capture.rs:34`
の `capture_logs`（どちらも `capture_lines` を呼ぶ本物の共有窓の包み）は自己テストを 3 本ずつ
抱えているために供給者から外れ、**その呼び手 5 ファイル・96 件が両側で走り続けた**。
ファイル単位ではなく**関数単位**で判定するのが正しい形で、較正 5 がこれを陽性で縛る。

そのうえで、

- 窓を開く語がテスト関数の**外側**（＝そのファイルの補助関数）にも現れるファイルは、
  どのテストがその補助を呼ぶかをここでは解かないので**ファイル内の全テスト**を除外する
  （過剰除外の側に倒す）。
- 窓を開く語がテスト関数の**内側にしか**現れないファイルは、当たったテストだけを除外する。

出力
----
`--out-dir` に 4 つ書く（すべて LF・末尾改行あり）。

- `exclusion-skip-args.txt` … `cargo test -- --skip <値>` に渡す値（1 行 1 個）
- `exclusion-tests.txt`     … 除外される完全なテスト名（1 行 1 個・`--list` と突合済み）
- `exclusion-files.txt`     … 根拠となったファイルと判定（`<判定><TAB><パス><TAB><除外数>/<全テスト数>`）
- `exclusion-report.txt`    … 件数の要約（走査語・ファイル数・テスト数・過剰除外の内訳）

較正（要件 8.4 と同じ規律＝「0 件なら緑」の道具を素通しにしない）
----------------------------------------------------------------
`--calibrate` を付けると、判定に使う純粋な部分を見本で両側から縛り、**既知の答えを逐語で
再現**してから本走査へ進む。再現できなければ非 0 で終了する。

  1. コメント除去と語の一致規則の見本（当たる／当たらない両側）。
  2. `with_default` 系 3 語の当たりファイル集合が、実在する見張り
     `crates/log-capture-kit/tests/with_default_guard_test.rs` の例外表 4 件と
     ちょうど一致すること（＝同じ答えを別の実装で再現する）。
  3. `install_global_capture_all` の当たりファイル集合が同見張りの別表 2 件と一致すること。
  4. `use … as 別名` で窓口を取り込んでいるファイルの集合が既知の 4 件と一致すること。
  5. **自己テストを持つ包み 2 件**（上記 `log_capture.rs`）が供給者になり、
     **その呼び手 5 件が当たること**。⑵⑶⑷ と同じく**陽性を要求する**形で書く
     （「拾われないこと」ではなく「拾われること」を要求する）。
  6. 選んだ `--skip` の値が、`--list` の全テスト名に対して**除外集合とちょうど同じ集合**に
     当たること（`--skip` は部分一致なので、巻き込みがあれば報告へ件数を出す）。

使い方
------
    cargo test --workspace -- --list > list.txt
    python capture-window-tests.py --root <ワークスペース根> --list list.txt \
        --out-dir <出力先> --calibrate
"""

import argparse
import os
import re
import sys

EXCLUDED_DIRS = {"target", "vendors", ".git"}
KIT_SRC_PREFIX = "crates/log-capture-kit/src/"
BACKSLASH = chr(92)

# ⑴ 共有 crate の窓を開く公開 API と ⑵ 迂回する素の呼出。定義元が共有 crate なので
# ワークスペースのどこに現れても当てる（＝適用範囲は空＝無制限）。
SEED_TOKENS = [
    "capture(",
    "capture_lines(",
    "count_levels(",
    "capture_under_filter(",
    "install_global_capture_all(",
    "with_default(",
    "set_default(",
    "set_global_default(",
]

# 較正 2・3 の既知の答え（with_default_guard_test.rs の ALLOWED_DIRECT_CALLS /
# ALLOWED_GLOBAL_CAPTURE を逐語で写したもの。表が動いたらここが赤になる）。
KNOWN_DIRECT_CALL_FILES = [
    "crates/areka/src/placement/diag_tests.rs",
    "crates/areka/src/placement/follow_transition_diag_tests.rs",
    "crates/areka/src/placement/follow_window_move_diag_tests.rs",
    "crates/log-capture-kit/tests/capture_calibration_test.rs",
]
KNOWN_GLOBAL_CAPTURE_FILES = [
    "crates/areka-ghost/tests/ghost/spine_e2e_test_global_log_probe.rs",
    "crates/areka-seriko/tests/loop_integration.rs",
]

# 較正 4 の既知の答え。`use … as 別名` で窓口を取り込んでいるファイル（2026-08-27 実測）。
# 別名を追わない走査はここを 1 件も拾わず、常駐なし側の 3 回目で
# `emo2_boot::frame::diag_route_tests::…` が赤になって初めて露見した。
KNOWN_ALIAS_FILES = [
    "crates/areka/src/emo2_boot/frame_diag_route_tests.rs",
    "crates/areka/src/emo2_boot/frame_dpi_reproject_none_tests.rs",
    "crates/areka/src/emo2_boot/frame_dpi_reproject_tests.rs",
    "crates/areka/src/emo2_boot/frame_harness_tests.rs",
]

# 較正 5 の既知の答え（2026-08-27 の差し戻しで足した・別名の穴と同じ扱い）。
# **自己テストを持つ包みも供給者になれる**ことを陽性で要求する。
# この 2 ファイルの `capture_logs` は `log_capture_kit::capture_lines` を呼ぶ本物の窓の包みで、
# どちらも `#[test]` を 3 本ずつ抱えている。「`#[test]` を 1 本も持たないファイル」だけを
# 供給者にしていた初版は両者を供給者から外し、下の 5 ファイル・96 件が両側で走り続けた。
KNOWN_SELF_TESTED_WRAPPERS = [
    ("crates/areka-emo-atlas/src/log_capture.rs", "capture_logs(", "crates/areka-emo-atlas/"),
    ("crates/areka-emo-compose/src/log_capture.rs", "capture_logs(", "crates/areka-emo-compose/"),
]
# その包みを呼ぶファイル（＝取りこぼしていた側）。当たることを要求する。
KNOWN_SELF_TESTED_WRAPPER_CALLERS = [
    "crates/areka-emo-atlas/src/lib.rs",
    "crates/areka-emo-compose/src/log_firing_tests.rs",
    "crates/areka-emo-compose/src/plan_ops_tests.rs",
    "crates/areka-emo-compose/src/scale_ratio_tests.rs",
    "crates/areka-emo-compose/src/scale_resample_tests.rs",
]

WORD_CHAR = re.compile(r"[A-Za-z0-9_]")
FN_DECL = re.compile(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)")
TEST_ATTR = re.compile(r"#\[\s*(?:tokio::)?test\s*[(\]]")
RAW_STRING_OPEN = re.compile(r"(?:b?r)(#*)\"")
CHAR_LITERAL = re.compile(r"'(?:" + BACKSLASH + r".[^']*|[^" + BACKSLASH + r"'])'")


# ---------------------------------------------------------------------------
# 純粋な部分（見本で較正できる）
# ---------------------------------------------------------------------------
def strip_comments(src):
    """コメントを取り除く。行の構成は変えない（改行を 1 個も増減させない）。

    `crates/log-capture-kit/tests/workspace_scan/mod.rs` の `strip_comments` と同じ規則。
    文字列・raw 文字列・文字リテラルの中身は残す（中の `//` はコメントではない）。
    """
    n = len(src)
    out = []
    i = 0
    while i < n:
        c = src[i]
        if c == "/" and i + 1 < n and src[i + 1] == "/":
            while i < n and src[i] != "\n":
                i += 1
            continue
        if c == "/" and i + 1 < n and src[i + 1] == "*":
            depth = 1
            i += 2
            while i < n and depth > 0:
                if src[i] == "/" and i + 1 < n and src[i + 1] == "*":
                    depth += 1
                    i += 2
                elif src[i] == "*" and i + 1 < n and src[i + 1] == "/":
                    depth -= 1
                    i += 2
                else:
                    if src[i] == "\n":
                        out.append("\n")
                    i += 1
            continue
        m = RAW_STRING_OPEN.match(src, i)
        if m and (i == 0 or not WORD_CHAR.match(src[i - 1])):
            closing = '"' + m.group(1)
            j = src.find(closing, m.end())
            j = n if j < 0 else j + len(closing)
            out.append(src[i:j])
            i = j
            continue
        if c == '"':
            out.append(c)
            i += 1
            while i < n:
                d = src[i]
                out.append(d)
                i += 1
                if d == BACKSLASH:
                    if i < n:
                        out.append(src[i])
                        i += 1
                elif d == '"':
                    break
            continue
        if c == "'":
            m = CHAR_LITERAL.match(src, i)
            if m:
                out.append(m.group(0))
                i = m.end()
                continue
        out.append(c)
        i += 1
    return "".join(out)


def scan_tokens(text, tokens):
    """語の左端をアンカーして当たりの位置を返す（昇順）。

    アンカーが無いと `fn test_offset_default` の開き括弧までの形が `set_default` ＋
    開き括弧に部分一致して偽陽性になる（`workspace_scan/mod.rs` の注記と同じ理由）。
    """
    hits = []
    for token in tokens:
        start = 0
        while True:
            pos = text.find(token, start)
            if pos < 0:
                break
            if pos == 0 or not WORD_CHAR.match(text[pos - 1]):
                hits.append(pos)
            start = pos + 1
    hits.sort()
    return hits


def fn_spans(text):
    """`(名前, 署名開始, 本体開始, 本体終了)` の一覧。入れ子の関数も個別に返す。"""
    spans = []
    for m in FN_DECL.finditer(text):
        i = m.end()
        parens = 0
        body_start = -1
        while i < len(text):
            ch = text[i]
            if ch == "(":
                parens += 1
            elif ch == ")":
                parens -= 1
            elif parens == 0 and ch == ";":
                break
            elif parens == 0 and ch == "{":
                body_start = i
                break
            i += 1
        if body_start < 0:
            continue
        depth = 0
        j = body_start
        while j < len(text):
            if text[j] == "{":
                depth += 1
            elif text[j] == "}":
                depth -= 1
                if depth == 0:
                    break
            j += 1
        spans.append((m.group(1), m.start(), body_start, j))
    return spans


def is_test_fn(text, sig_start):
    """署名の直前に `#[test]`（または `#[tokio::test]`）が付いているか。"""
    head = text[max(0, sig_start - 600) : sig_start]
    tail = head.rsplit("}", 1)[-1].rsplit(";", 1)[-1]
    return bool(TEST_ATTR.search(tail))


def token_scope(rel_path):
    """包みの語を当てる範囲。定義元の crate（`tests/` 由来ならその `tests/` の木）。"""
    parts = rel_path.split("/")
    if len(parts) >= 3 and parts[0] == "crates" and parts[2] == "tests":
        return "/".join(parts[:3]) + "/"
    if len(parts) >= 2 and parts[0] == "crates":
        return "/".join(parts[:2]) + "/"
    return ""


ALIAS_RENAME = re.compile(
    r"\b([A-Za-z_][A-Za-z0-9_]*)\s+as\s+([A-Za-z_][A-Za-z0-9_]*)"
)


def alias_tokens(text, base_tokens):
    """`use … as 別名` で名前を替えた呼出を拾う。

    **この穴は実測で見つかった**——`crates/areka/src/emo2_boot/frame_diag_route_tests.rs:24` が
    `capture_logs as capture_diag_logs` で取り込んでおり、別名を追わない走査は当該ファイルを
    1 件も拾わなかった。除外集合が 1 ファイル分欠けたまま常駐なし側を 3 回走らせて、
    3 回目に当該ファイルのテストが赤になって露見した（走査の空振りは緑で通る）。

    元の名前が窓を開く語であるときだけ別名を採るので、型変換の `x as u32` には当たらない。
    """
    names = {t[:-1] for t in base_tokens}
    found = set()
    for m in ALIAS_RENAME.finditer(text):
        if m.group(1) in names:
            found.add(m.group(2) + "(")
    return sorted(found)


def tokens_for(rel_path, scoped_tokens, text=None):
    """そのファイルへ当ててよい語の一覧（`use … as 別名` の別名を含む）。"""
    usable = [t for (t, scope) in scoped_tokens if not scope or rel_path.startswith(scope)]
    if text is not None:
        usable = usable + alias_tokens(text, usable)
    return usable


# ---------------------------------------------------------------------------
# 走査
# ---------------------------------------------------------------------------
def walk_workspace_sources(root):
    found = []
    for dirpath, dirnames, filenames in os.walk(os.path.join(root, "crates")):
        dirnames[:] = sorted(d for d in dirnames if d not in EXCLUDED_DIRS)
        for name in sorted(filenames):
            if name.endswith(".rs"):
                rel = os.path.relpath(os.path.join(dirpath, name), root)
                found.append(rel.replace(BACKSLASH, "/"))
    found.sort()
    return found


def read_sources(root):
    sources = {}
    for rel in walk_workspace_sources(root):
        with open(os.path.join(root, rel), "rb") as handle:
            raw = handle.read()
        sources[rel] = strip_comments(raw.decode("utf-8", "replace"))
    return sources


def donor_fn_spans(text, spans, tests):
    """供給者になれる関数の span。＝**テスト関数の外で定義された非テスト関数**。

    2026-08-27 の差し戻しで入れた条件。以前は「`#[test]` を 1 本も持たないファイル」だけを
    供給者にしていたが、この repo では**包みファイルが自己テストを持つのが定石**なので、
    `crates/areka-emo-atlas/src/log_capture.rs` と `crates/areka-emo-compose/src/log_capture.rs`
    の `capture_logs`（どちらも `capture_lines` を呼ぶ本物の窓の包み・自己テスト 3 本ずつ）が
    供給者から外れ、**その呼び手 5 ファイル・96 件が両側で走り続けた**。
    ファイル単位ではなく関数単位で判定する。

    テスト関数の**内側**で定義された入れ子の関数は供給者にしない（その名前はそのテストの
    ローカルな道具であって、他ファイルから呼ばれる窓口ではない）。
    """
    test_bodies = [(b, e) for (_n, _s, b, e) in tests]
    out = []
    for name, sig, body_start, body_end in spans:
        if is_test_fn(text, sig):
            continue
        if any(b < sig < e for (b, e) in test_bodies):
            continue
        out.append((name, sig, body_start, body_end))
    return out


def fixpoint_tokens(sources, spans_of, tests_of):
    """包みの語の不動点。`(語, 適用範囲)` の集合を返す。

    語を提供できるのは「`log_capture_kit` を参照する」ファイルの中の、
    **テスト関数の外で定義され、本体が窓を開く語を呼ぶ非テスト関数**。
    ファイルが自己テストを持っていても供給者になれる（2026-08-27 の是正）。
    """
    donors = {
        rel: text
        for rel, text in sources.items()
        if not rel.startswith(KIT_SRC_PREFIX) and "log_capture_kit" in text
    }
    scoped = {(t, "") for t in SEED_TOKENS}
    for _ in range(16):
        added = set()
        for rel, text in donors.items():
            usable = tokens_for(rel, scoped, text)
            hits = scan_tokens(text, usable)
            if not hits:
                continue
            spans = donor_fn_spans(text, spans_of(rel), tests_of(rel))
            for pos in hits:
                for name, sig, body_start, body_end in spans:
                    if body_start < pos < body_end:
                        entry = (name + "(", token_scope(rel))
                        if entry not in scoped:
                            added.add(entry)
        if not added:
            return sorted(scoped), sorted(donors), True
        scoped |= added
    return sorted(scoped), sorted(donors), False


def classify_files(sources, scoped_tokens, spans_of, tests_of):
    """ファイルごとに除外するテスト名を決める。"""
    result = {}
    for rel, text in sorted(sources.items()):
        if rel.startswith(KIT_SRC_PREFIX):
            continue
        hits = scan_tokens(text, tokens_for(rel, scoped_tokens, text))
        if not hits:
            continue
        tests = tests_of(rel)
        all_names = sorted({n for (n, _s, _b, _e) in tests})
        inside = set()
        covered = set()
        for pos in hits:
            for name, _s, body_start, body_end in tests:
                if body_start < pos < body_end:
                    inside.add(name)
                    covered.add(pos)
        outside = [pos for pos in hits if pos not in covered]
        if not all_names:
            mode, chosen = "helper-only", []
        elif outside:
            mode, chosen = "whole-file", all_names
        else:
            mode, chosen = "per-test", sorted(inside)
        result[rel] = {"mode": mode, "excluded": chosen, "all": all_names}
    return result


# ---------------------------------------------------------------------------
# `--list` との突合と `--skip` の値の選定
# ---------------------------------------------------------------------------
def load_list(path):
    names = []
    with open(path, "rb") as handle:
        text = handle.read().decode("utf-8", "replace")
    for line in text.replace("\r\n", "\n").split("\n"):
        if line.endswith(": test"):
            names.append(line[: -len(": test")])
    return names


def choose_skip_values(all_names, excluded):
    """除外集合を覆う `--skip` の値を選ぶ。

    `--skip` は完全なテスト名に対する**部分一致**なので、module 接頭辞（`::` 終わり）で
    まとめられるところはまとめ、残りは完全名で個別に指定する。接頭辞を採るのは
    「その接頭辞で始まる全テストが除外集合に入っている」ときだけ。
    """
    excluded = set(excluded)
    prefixes = {}
    for name in all_names:
        parts = name.split("::")
        for k in range(1, len(parts)):
            prefixes.setdefault("::".join(parts[:k]) + "::", []).append(name)
    usable = [
        (p, members)
        for p, members in prefixes.items()
        if all(m in excluded for m in members)
    ]
    usable.sort(key=lambda item: (-len(item[1]), item[0]))
    chosen = []
    remaining = set(excluded)
    for prefix, members in usable:
        if any(m in remaining for m in members):
            chosen.append(prefix)
            remaining -= set(members)
    chosen.extend(sorted(remaining))
    return sorted(chosen)


def matched_by(values, all_names):
    """libtest の `--skip` と同じ規則（部分一致）で当たる名前の集合。"""
    hit = set()
    for name in all_names:
        for value in values:
            if value in name:
                hit.add(name)
                break
    return hit


# ---------------------------------------------------------------------------
# 較正
# ---------------------------------------------------------------------------
def calibrate_pure():
    """⑴ コメント除去と語の一致規則を見本で両側から縛る。"""
    problems = []

    def check(label, actual, expected):
        if actual != expected:
            problems.append(f"{label}: 期待 {expected!r} 実際 {actual!r}")

    token = "capture("
    check("行コメントの中の語は当たらない", scan_tokens(strip_comments("// capture(x)\n"), [token]), [])
    check("塊コメントの中の語は当たらない", scan_tokens(strip_comments("/* capture(x) */\n"), [token]), [])
    check(
        "入れ子の塊コメントを抜けた後の語は当たる",
        len(scan_tokens(strip_comments("/* /* x */ */ capture(1)\n"), [token])),
        1,
    )
    check(
        "文字列の中の二重斜線で行が消えない",
        len(scan_tokens(strip_comments('let s = "//"; capture(1)\n'), [token])),
        1,
    )
    check(
        "raw 文字列の中の二重斜線で行が消えない",
        len(scan_tokens(strip_comments('let s = r#"//"#; capture(1)\n'), [token])),
        1,
    )
    check("左端のアンカー（識別子の一部には当たらない）", scan_tokens("fn recapture(x)", [token]), [])
    check("素の当たりは 1 件", len(scan_tokens("  capture(1)", [token])), 1)
    check("コメント除去は改行を増減させない", strip_comments("a // x\nb /* y\nz */ c\n").count("\n"), 3)
    src = "#[test]\nfn t() { capture(1); }\nfn helper() { capture(2); }\n"
    stripped = strip_comments(src)
    spans = fn_spans(stripped)
    check("関数は 2 本見つかる", len(spans), 2)
    check("テスト属性の判定", [is_test_fn(stripped, s) for (_n, s, _b, _e) in spans], [True, False])
    check("語の適用範囲（src）", token_scope("crates/areka/src/a/b.rs"), "crates/areka/")
    check(
        "語の適用範囲（tests）",
        token_scope("crates/areka-ghost/tests/ghost/x.rs"),
        "crates/areka-ghost/tests/",
    )
    check(
        "適用範囲の外では当てない",
        tokens_for("crates/areka/src/main.rs", [("install(", "crates/areka-ghost/tests/")]),
        [],
    )
    check(
        "別名の取り込みを拾う",
        alias_tokens("use a::b::{X, capture_logs as capture_diag_logs};", ["capture_logs("]),
        ["capture_diag_logs("],
    )
    check("型変換の as は別名ではない", alias_tokens("let n = x as u32;", ["capture_logs("]), [])
    return problems


def calibrate_known_answers(sources):
    """⑵⑶ 既知の答え（実在する見張りの例外表）を逐語で再現する。"""
    problems = []
    direct = sorted(
        rel
        for rel, text in sources.items()
        if not rel.startswith(KIT_SRC_PREFIX)
        and scan_tokens(text, ["with_default(", "set_default(", "set_global_default("])
    )
    if direct != sorted(KNOWN_DIRECT_CALL_FILES):
        problems.append(
            "素の呼出の当たりファイルが見張りの例外表と一致しない:\n"
            f"  走査 {direct}\n  例外表 {sorted(KNOWN_DIRECT_CALL_FILES)}"
        )
    glob = sorted(
        rel
        for rel, text in sources.items()
        if not rel.startswith(KIT_SRC_PREFIX)
        and scan_tokens(text, ["install_global_capture_all("])
    )
    if glob != sorted(KNOWN_GLOBAL_CAPTURE_FILES):
        problems.append(
            "全スレッド捕捉の当たりファイルが見張りの別表と一致しない:\n"
            f"  走査 {glob}\n  別表 {sorted(KNOWN_GLOBAL_CAPTURE_FILES)}"
        )
    aliased = sorted(
        rel
        for rel, text in sources.items()
        if not rel.startswith(KIT_SRC_PREFIX)
        and alias_tokens(text, ["capture(", "capture_lines(", "count_levels(", "capture_logs(",
                                "capture_events(", "capture_under_filter(",
                                "install_global_capture_all("])
    )
    if aliased != sorted(KNOWN_ALIAS_FILES):
        problems.append(
            "別名で窓口を取り込んでいるファイルの集合が既知の答えと一致しない:"
            f"  走査 {aliased} / 既知 {sorted(KNOWN_ALIAS_FILES)}"
        )
    if not direct or not glob or not aliased:
        problems.append("既知の答えの側が 0 件（走査そのものが空振りしている）")
    return problems


def calibrate_self_tested_wrappers(sources, scoped_tokens, per_file):
    """⑸ 自己テストを持つ包みが供給者になり、その呼び手が当たること（陽性で要求する）。

    2026-08-27 の差し戻しで足した。§4 の「別名の穴」と同じ家族の 2 件目で、
    あちらが 1 ファイルの取りこぼしだったのに対しこちらは**供給条件そのものが構造的に
    外していた**（この repo では包みファイルが自己テストを持つのが定石なので同じ形は再発する）。
    """
    problems = []
    scoped = set(scoped_tokens)
    for rel, token, scope in KNOWN_SELF_TESTED_WRAPPERS:
        if rel not in sources:
            problems.append(f"自己テスト付きの包みが実在しない: {rel}")
            continue
        n_tests = len(TEST_ATTR.findall(sources[rel]))
        if n_tests == 0:
            problems.append(
                f"{rel} が自己テストを持たない（この較正が守っている形が消えている）"
            )
        if (token, scope) not in scoped:
            problems.append(
                f"自己テスト付きの包みが供給者になっていない: {rel} は {token} を "
                f"{scope} へ供給するはず（走査語 {len(scoped)} 語の中に不在）"
            )
    missed = [rel for rel in KNOWN_SELF_TESTED_WRAPPER_CALLERS if rel not in per_file]
    if missed:
        problems.append(
            "自己テスト付きの包みを呼ぶファイルが 1 件も当たっていない: "
            f"  取りこぼし {missed}"
        )
    covered = [rel for rel in KNOWN_SELF_TESTED_WRAPPER_CALLERS if rel in per_file]
    if not covered:
        problems.append("較正 5 の陽性側が 0 件（走査そのものが空振りしている）")
    return problems


# ---------------------------------------------------------------------------
def write_lf(path, lines):
    with open(path, "wb") as handle:
        handle.write(("\n".join(lines) + "\n").encode("utf-8"))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", required=True)
    parser.add_argument("--list", dest="list_path", required=True)
    parser.add_argument("--out-dir", required=True)
    parser.add_argument("--calibrate", action="store_true")
    parser.add_argument(
        "--granularity",
        choices=["file", "test"],
        default="file",
        help=(
            "file（既定）= 当たったファイルの全テストを除外する。"
            "test = 窓を開く語がテスト関数の内側にしか無いファイルでは当たったテストだけを除外する。"
            "test のほうが除外は小さいが `--skip` の値が 319 個・約 28.8KB になり、"
            "Windows のコマンドラインの上限 32,767 文字に迫る（file は 179 個・約 9.9KB）。"
            "既定を file にしてあるのはそのため——過剰除外の側に倒しても、"
            "常駐の代償はワークスペース全体に乗るので残りに差は出る（設計 C9）。"
        ),
    )
    args = parser.parse_args()

    root = os.path.abspath(args.root)
    os.makedirs(args.out_dir, exist_ok=True)

    sources = read_sources(root)

    span_cache = {}

    def spans_of(rel):
        if rel not in span_cache:
            span_cache[rel] = fn_spans(sources[rel])
        return span_cache[rel]

    test_cache = {}

    def tests_of(rel):
        if rel not in test_cache:
            text = sources[rel]
            test_cache[rel] = [
                (n, s, b, e) for (n, s, b, e) in spans_of(rel) if is_test_fn(text, s)
            ]
        return test_cache[rel]

    scoped_tokens, donors, settled = fixpoint_tokens(sources, spans_of, tests_of)
    if not settled:
        print("走査語の不動点が収束しなかった", file=sys.stderr)
        return 2

    per_file = classify_files(sources, scoped_tokens, spans_of, tests_of)

    # 較正は走査の結果まで含めて縛るので、不動点と分類の**後**・書き出しの**前**に置く。
    if args.calibrate:
        problems = (
            calibrate_pure()
            + calibrate_known_answers(sources)
            + calibrate_self_tested_wrappers(sources, scoped_tokens, per_file)
        )
        if problems:
            print("較正に失敗した（道具が壊れている）:", file=sys.stderr)
            for problem in problems:
                print("  - " + problem, file=sys.stderr)
            return 2
        print(
            "較正 OK（純粋な部分の見本 / 既知の答え"
            " 素の呼出 4 件 + 全スレッド捕捉 2 件 + 別名 4 件"
            " + 自己テスト付きの包み 2 件とその呼び手 5 件 の逐語再現）"
        )
    pick = "all" if args.granularity == "file" else "excluded"
    excluded_fn_names = set()
    for info in per_file.values():
        excluded_fn_names.update(info[pick])

    all_names = load_list(args.list_path)
    if not all_names:
        print(f"--list の出力からテスト名を採れなかった: {args.list_path}", file=sys.stderr)
        return 2

    by_leaf = {}
    for name in all_names:
        by_leaf.setdefault(name.split("::")[-1], []).append(name)

    excluded_full = set()
    unmatched = []
    ambiguous = []
    for fn_name in sorted(excluded_fn_names):
        hits = by_leaf.get(fn_name, [])
        if not hits:
            unmatched.append(fn_name)
            continue
        if len(hits) > 1:
            ambiguous.append(fn_name)
        excluded_full.update(hits)

    skip_values = choose_skip_values(all_names, excluded_full)
    matched = matched_by(skip_values, all_names)
    overshoot = sorted(matched - excluded_full)
    undershoot = sorted(excluded_full - matched)
    if undershoot:
        print(f"選んだ --skip が除外集合を覆えていない: {len(undershoot)} 件", file=sys.stderr)
        for n in undershoot[:10]:
            print("  - " + n, file=sys.stderr)
        return 2
    if overshoot:
        # 接頭辞の部分一致で巻き込んだぶんは除外集合へ繰り入れる（両側同じフィルタなので
        # 比較の妥当性は保たれる。何件巻き込んだかは報告へ残す）。
        excluded_full |= set(overshoot)

    write_lf(os.path.join(args.out_dir, "exclusion-skip-args.txt"), skip_values)
    write_lf(os.path.join(args.out_dir, "exclusion-tests.txt"), sorted(excluded_full))
    write_lf(
        os.path.join(args.out_dir, "exclusion-files.txt"),
        [
            "{}\t{}\t{}/{}".format(info["mode"], rel, len(info["excluded"]), len(info["all"]))
            for rel, info in sorted(per_file.items())
        ],
    )
    modes = {}
    for info in per_file.values():
        modes[info["mode"]] = modes.get(info["mode"], 0) + 1
    report = [
        "# 共有捕捉窓を使うテストの静的な列挙（タスク 11.2・要件 13.2）",
        f"除外の粒度: {args.granularity}"
        + "（file = 当たったファイルの全テスト / test = 当たったテストだけ）",
        f"走査したソース: {len(sources)} ファイル（crates/**/*.rs・コメント除去後）",
        f"包みの語を提供しうるファイル: {len(donors)} 件"
        + "（log_capture_kit を参照する・自己テストの有無は問わない）",
        f"走査語（不動点後・適用範囲つき）: {len(scoped_tokens)} 語",
    ]
    for token, scope in scoped_tokens:
        report.append(f"  {token}\t{scope or '（全域＝共有 crate の窓口）'}")
    report += [
        f"当たったファイル: {len(per_file)} 件（"
        + " ".join(f"{k}={v}" for k, v in sorted(modes.items()))
        + "）",
        f"除外するテスト関数名: {len(excluded_fn_names)} 個",
        f"--list の行（test）: {len(all_names)} 行 / 重複を除くと {len(set(all_names))} 個",
        f"除外する完全なテスト名: {len(excluded_full)} 個",
        f"  うち --skip の部分一致で巻き込んだぶん: {len(overshoot)} 個",
        f"  同名のテストが複数 module にある関数名: {len(ambiguous)} 個",
        f"  --list に見つからなかった関数名: {len(unmatched)} 個",
        "  " + (" ".join(unmatched[:30]) if unmatched else "（無し）"),
        f"--skip に渡す値: {len(skip_values)} 個",
        f"残るテスト名: {len(set(all_names)) - len(excluded_full)} 個",
    ]
    write_lf(os.path.join(args.out_dir, "exclusion-report.txt"), report)
    print("\n".join(report))
    return 0


if __name__ == "__main__":
    sys.exit(main())
