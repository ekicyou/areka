//! 共有機構を迂回する捕捉の新設を検知する見張り（要件 1.3・1.6・2.6・8.1・8.2・8.3・11.5）。
//!
//! # 何を見張るか（4 つ）
//!
//! 1. **捕捉先を直接差す呼出**（3 語）が、共有 crate の定義（`crates/log-capture-kit/src/`）と
//!    例外表 [`ALLOWED_DIRECT_CALLS`] の外に 1 件も無いこと（要件 2.6・8.1）。
//! 2. **全スレッド捕捉の窓口**の利用が、別表 [`ALLOWED_GLOBAL_CAPTURE`] に載ったファイルだけで
//!    あること（要件 1.6。既定の捕捉 API と混同させないための明示表）。
//! 3. 各 crate の**製品側依存**（`[dependencies]`・`[build-dependencies]`・`[target.*.dependencies]`）に
//!    共有 crate が現れないこと＝dev-dependency 限定（要件 1.3・11.5）。
//! 4. `env-filter` フィーチャを宣言する crate が **`wintf` だけ**であること。
//!    フィーチャはワークスペースで統合されるので、他 crate が宣言すると 10 crate すべてに
//!    `capture_under_filter` が届き、「有効にするのは wintf のみ」という設計の宣言が崩れる
//!    （コンパイラは強制しない＝見張りが唯一の担保。タスク 2.1 の申し送り）。
//!
//! # 走査語を逐語で書かない約束
//!
//! 本ファイルは `crates/` の下にあるので、上の ⑴⑵ の走査対象そのものでもある。走査語を
//! 開き括弧まで含めた形で書くと ⒜ 見張りが自分自身を違反として拾い ⒝ 着手前インベントリの
//! `rg -l` による捕捉サイト計数の母数が動く。そこで走査語は `concat!` で 2 片から組み立て、
//! ファイルの字面には 1 度も現れないようにする（`workspace_scan/mod.rs:21-23` と同じ約束）。
//! 自分自身を違反として拾っていないことは
//! [`the_guard_file_itself_is_not_a_hit_because_the_tokens_are_never_spelled_out`] が、
//! **約束そのもの**（コメントの中にも逐語で置かないこと）は
//! [`the_guard_files_never_spell_the_tokens_out_not_even_inside_comments`] が縛る。
//! 前者だけでは足りない——[`scan_tokens`] はコメントを除去するので、コメントへ逐語で
//! 植えても 1 件も拾わないまま `rg -l` の母数だけが動く（レビューの実測で確認済み）。
//!
//! # 例外表が暗黙に増えない形（要件 8.2）
//!
//! 例外表は `const` なのでソースの編集でしか増えないが、それだけでは「ついでに 1 行足す」を
//! 止められない。そこで次の 3 つを併せて要求する。
//!
//! - 件数を別の定数（[`ALLOWED_DIRECT_CALLS_COUNT`]・[`ALLOWED_GLOBAL_CAPTURE_COUNT`]）に
//!   逐語で持ち、表と一致しなければ赤にする。項目の追加は**表と件数の 2 箇所**の編集になる。
//! - 各項目に理由（空でない文字列）を要求し、ファイル 1 件を逐語で指すことを要求する。
//! - 表に載っているのに実際には当たりが無い項目（陳腐化した例外）を赤にする。
//!   例外は「今そこにある事情」だけを表すので、事情が消えたら表からも消える。
//!
//! # 「0 件なら緑」への較正（要件 8.4）
//!
//! ⑴⑵⑶ はいずれも「違反 0 件なら緑」の形で、**道具が壊れていても緑**になる。よって
//! 判定に使う関数はすべて純関数として切り出し、見本で両側（当たる／当たらない）を固定する。
//! 加えて実データ側にも陽性を要求する——例外表の全項目に実際の当たりがあること、
//! 除外した共有 crate の定義には確かに 3 ファイル分の当たりがあること、
//! 10 crate が確かに dev-dependency として共有 crate を引いていること。

mod workspace_scan;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use workspace_scan::{read_source, scan_tokens, walk_workspace_sources, workspace_root};

// ---------------------------------------------------------------------------
// 走査語（逐語で置かないため 2 片に割る。module doc を参照）
// ---------------------------------------------------------------------------

/// 捕捉先をスレッドへ差す素の呼出。
const TOKEN_WITH_DEFAULT: &str = concat!("with_", "default(");
/// 同上（`tracing::dispatcher` 側の綴り）。
const TOKEN_SET_DEFAULT: &str = concat!("set_", "default(");
/// プロセス全体へ捕捉先を据える呼出。
const TOKEN_SET_GLOBAL_DEFAULT: &str = concat!("set_global_", "default(");
/// 共有 crate の全スレッド捕捉の窓口。
const TOKEN_INSTALL_GLOBAL_CAPTURE: &str = concat!("install_global_capture_", "all(");

/// ⑴ の走査語。
fn direct_call_tokens() -> Vec<&'static str> {
    vec![
        TOKEN_WITH_DEFAULT,
        TOKEN_SET_GLOBAL_DEFAULT,
        TOKEN_SET_DEFAULT,
    ]
}

/// ⑵ の走査語。
fn global_capture_tokens() -> Vec<&'static str> {
    vec![TOKEN_INSTALL_GLOBAL_CAPTURE]
}

// ---------------------------------------------------------------------------
// 例外表
// ---------------------------------------------------------------------------

/// 共有機構の**定義**が置かれた領域。⑴⑵ の走査から外す唯一の領域。
///
/// 外すのは `src/` だけで、共有 crate の `tests/` は走査する（`tests/` には較正が
/// 意図的な素の呼出を 1 件持っており、それは例外表の項目として扱うのが正しい）。
const KIT_DEFINITION_PREFIX: &str = "crates/log-capture-kit/src/";

/// ⑴ の例外表（相対パス・理由）。
///
/// **初期値は空ではない**。移行（タスク 3.7）の実測で 3 件が原理的に移行不能と判明し、
/// 較正（タスク 2.7）の 1 件と合わせて 4 件で始まる。起草時の「例外表は既定で空」は
/// 着手前の見積りで、実測と食い違っていた（design.md はタスク 3.7 で、requirements.md 8.1 と
/// tasks.md はタスク 6.2 で、いずれも実測側へ訂正済み）。
const ALLOWED_DIRECT_CALLS: &[(&str, &str)] = &[
    (
        "crates/areka/src/placement/diag_tests.rs",
        "実濾過（EnvFilter）の観測が要るが capture_under_filter は共有 crate の env-filter \
         フィーチャ下にあり、areka は当該フィーチャを有効にしない（有効にしてよいのは wintf のみ）\
         ため移行不能。窓の直前で ensure_interest_probes() を呼んでおり硬化は保たれている——\
         この呼出を「未使用」として外すと窓が静かに硬化を失う",
    ),
    (
        "crates/areka/src/placement/follow_transition_diag_tests.rs",
        "同上（実濾過が要る・窓の直前の ensure_interest_probes() で硬化を保っている）",
    ),
    (
        "crates/areka/src/placement/follow_window_move_diag_tests.rs",
        "同上（実濾過が要る・窓の直前の ensure_interest_probes() で硬化を保っている）",
    ),
    (
        "crates/log-capture-kit/tests/capture_calibration_test.rs",
        "硬化なしの捕捉が取りこぼすことを別プロセスで示す較正（要件 3.4-b）の、意図的な素の呼出。\
         共有機構へ寄せると較正が空振りになり、本仕様の中心的な主張が無証跡に戻る",
    ),
];

/// [`ALLOWED_DIRECT_CALLS`] の件数（逐語）。表を増やすときはここも編集する（要件 8.2）。
const ALLOWED_DIRECT_CALLS_COUNT: usize = 4;

/// ⑵ の別表（相対パス・理由）。
const ALLOWED_GLOBAL_CAPTURE: &[(&str, &str)] = &[
    (
        "crates/areka-ghost/tests/ghost/spine_e2e_test_global_log_probe.rs",
        "観測したいログが別スレッド（areka-actor のアクター）で発火するため、スレッド局所の\
         既定 API では原理的に捕まらない。要件 1.6 が求める「明示的に区別された別の API」の利用",
    ),
    (
        "crates/areka-seriko/tests/loop_integration.rs",
        "同上（seriko のループがアクタースレッドで回るため全スレッド捕捉が要る）",
    ),
];

/// [`ALLOWED_GLOBAL_CAPTURE`] の件数（逐語）。
const ALLOWED_GLOBAL_CAPTURE_COUNT: usize = 2;

// ---------------------------------------------------------------------------
// 走査（⑴⑵）
// ---------------------------------------------------------------------------

/// 走査で当たった 1 件。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Hit {
    /// ワークスペース根からの相対パス（区切りは `/`）。
    path: String,
    /// 1 始まりの行番号。
    line: usize,
    /// 当たった語。
    token: String,
}

impl Hit {
    fn new(path: impl Into<String>, line: usize, token: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            line,
            token: token.into(),
        }
    }
}

/// 見張りの対象（共有 crate の定義を除いた全ソース）。
fn watched_sources() -> Vec<String> {
    walk_workspace_sources()
        .into_iter()
        .filter(|p| !p.starts_with(KIT_DEFINITION_PREFIX))
        .collect()
}

/// 共有 crate の定義（走査から外した領域）。
fn kit_definition_sources() -> Vec<String> {
    walk_workspace_sources()
        .into_iter()
        .filter(|p| p.starts_with(KIT_DEFINITION_PREFIX))
        .collect()
}

/// 与えられたファイル群を走査する。
fn scan_sources(paths: &[String], tokens: &[&str]) -> Vec<Hit> {
    let mut hits = Vec::new();
    for path in paths {
        for (line, token) in scan_tokens(&read_source(path), tokens) {
            hits.push(Hit::new(path.clone(), line, token));
        }
    }
    hits
}

/// 例外表に載っていない当たり（＝違反）を返す（純関数）。
fn unlisted(hits: &[Hit], allow: &[(&str, &str)]) -> Vec<Hit> {
    hits.iter()
        .filter(|h| !allow.iter().any(|(path, _)| *path == h.path))
        .cloned()
        .collect()
}

/// 例外表にあるのに当たりが 1 件も無い項目（＝陳腐化した例外）を返す（純関数）。
fn stale_entries(hits: &[Hit], allow: &[(&str, &str)]) -> Vec<String> {
    allow
        .iter()
        .filter(|(path, _)| !hits.iter().any(|h| h.path == *path))
        .map(|(path, _)| (*path).to_string())
        .collect()
}

/// 違反の一覧を人が読める形にする。
fn render(hits: &[Hit]) -> String {
    hits.iter()
        .map(|h| format!("  {}:{} ({})", h.path, h.line, h.token))
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// manifest（⑶⑷）
// ---------------------------------------------------------------------------

/// 共有 crate のパッケージ名。
const KIT_PACKAGE: &str = "log-capture-kit";

/// フィーチャ名（`Cargo.toml` に現れる引用符つきの形）。
const FEATURE_ENV_FILTER: &str = "\"env-filter\"";

/// `env-filter` を宣言してよい唯一の crate。
const ENV_FILTER_OWNER: &str = "wintf";

/// `Cargo.toml` の 1 行（コメント除去済み・所属セクションつき）。
#[derive(Debug, Clone, PartialEq, Eq)]
struct ManifestLine {
    /// 1 始まりの行番号。
    line: usize,
    /// 直近のセクション見出し（角括弧を外した中身）。
    section: String,
    /// コメントを除き前後の空白を落とした行。
    text: String,
    /// この行自身がセクション見出しか。
    is_header: bool,
}

/// TOML の行末コメントを落とす。引用符の内側の `#` はコメントではない。
fn strip_toml_comment(line: &str) -> &str {
    let mut in_string = false;
    for (at, ch) in line.char_indices() {
        match ch {
            '"' | '\'' => in_string = !in_string,
            '#' if !in_string => return &line[..at],
            _ => {}
        }
    }
    line
}

/// `Cargo.toml` を「コメントを除いた非空行 ＋ 所属セクション」へ分解する（純関数）。
fn manifest_lines(src: &str) -> Vec<ManifestLine> {
    let mut out = Vec::new();
    let mut section = String::new();
    for (index, raw) in src.lines().enumerate() {
        let text = strip_toml_comment(raw).trim().to_string();
        if text.is_empty() {
            continue;
        }
        let is_header = text.starts_with('[') && text.ends_with(']');
        if is_header {
            section = text
                .trim_matches(|c| c == '[' || c == ']')
                .trim()
                .to_string();
        }
        out.push(ManifestLine {
            line: index + 1,
            section: section.clone(),
            text,
            is_header,
        });
    }
    out
}

/// 製品側の依存表か（`dev-dependencies` 系は含まない）。
fn is_production_dependency_section(section: &str) -> bool {
    section.contains("dependencies") && !section.contains("dev-dependencies")
}

/// 開発側の依存表か。
fn is_dev_dependency_section(section: &str) -> bool {
    section.contains("dev-dependencies")
}

/// 共有 crate を名指ししているか（`-` 表記と `_` 表記の両方）。
fn mentions_kit(text: &str) -> bool {
    text.contains(KIT_PACKAGE) || text.contains("log_capture_kit")
}

/// 製品側依存に共有 crate が現れている行を返す（純関数・要件 1.3／11.5）。
///
/// `[dependencies]` の中の 1 行という形と、`[dependencies.log-capture-kit]` という
/// 下位表の形の両方を拾う。下位表は**見出し 1 件**として報告する（見出しが名前を持つ表では
/// 中身の行も `path = "../log-capture-kit"` のように名前を含みがちで、1 つの依存が
/// 複数件に膨らむ）。
fn production_kit_dependencies(src: &str) -> Vec<ManifestLine> {
    manifest_lines(src)
        .into_iter()
        .filter(|l| is_production_dependency_section(&l.section))
        .filter(|l| {
            if l.is_header {
                mentions_kit(&l.section)
            } else {
                !mentions_kit(&l.section) && mentions_kit(&l.text)
            }
        })
        .collect()
}

/// 開発側依存に共有 crate が現れているか（純関数・較正の陽性側）。
fn has_dev_kit_dependency(src: &str) -> bool {
    manifest_lines(src).iter().any(|l| {
        is_dev_dependency_section(&l.section)
            && if l.is_header {
                mentions_kit(&l.section)
            } else {
                mentions_kit(&l.text)
            }
    })
}

/// 共有 crate の依存宣言で `env-filter` フィーチャを有効にしているか（純関数）。
fn declares_env_filter_feature(src: &str) -> bool {
    manifest_lines(src).iter().any(|l| {
        !l.is_header
            && (mentions_kit(&l.text) || mentions_kit(&l.section))
            && l.text.contains(FEATURE_ENV_FILTER)
    })
}

/// 列挙から外すディレクトリ名（生成物と外部取り込み）。
const EXCLUDED_DIRS: &[&str] = &["target", "vendors", ".git"];

/// `crates/**/Cargo.toml` を列挙して `(crate ディレクトリ名, 中身)` を返す。
///
/// 共有 crate 自身の manifest は除く（自分の名前を `[package]` に持つので比較の対象外）。
fn workspace_manifests() -> Vec<(String, String)> {
    let root = workspace_root();
    let mut found = Vec::new();
    collect_manifests(&root.join("crates"), &mut found);
    let mut out: Vec<(String, String)> = found
        .into_iter()
        .filter_map(|path| {
            let name = path
                .parent()
                .and_then(Path::file_name)
                .expect("Cargo.toml には親ディレクトリがあるはず")
                .to_string_lossy()
                .into_owned();
            if name == KIT_PACKAGE {
                return None;
            }
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("manifest を読めない: {} ({err})", path.display()));
            Some((name, text))
        })
        .collect();
    out.sort();
    out
}

fn collect_manifests(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => panic!("列挙できないディレクトリがある: {} ({err})", dir.display()),
    };
    for entry in entries {
        let entry = entry.expect("ディレクトリ項目の読み取りに失敗した");
        let path = entry.path();
        let file_type = entry.file_type().expect("種別の判定に失敗した");
        let name = entry.file_name().to_string_lossy().into_owned();
        if file_type.is_dir() {
            if EXCLUDED_DIRS.contains(&name.as_str()) {
                continue;
            }
            collect_manifests(&path, out);
        } else if file_type.is_file() && name == "Cargo.toml" {
            out.push(path);
        }
    }
}

// ---------------------------------------------------------------------------
// ⑴ 捕捉先を直接差す呼出
// ---------------------------------------------------------------------------

#[test]
fn no_direct_capture_call_lives_outside_the_shared_crate_and_the_allow_table() {
    let hits = scan_sources(&watched_sources(), &direct_call_tokens());
    let violations = unlisted(&hits, ALLOWED_DIRECT_CALLS);
    assert!(
        violations.is_empty(),
        "共有機構を迂回して捕捉先を直接差している箇所がある。\
         共有 crate（log_capture_kit）の窓口を使うか、やむを得ない場合は理由付きで \
         ALLOWED_DIRECT_CALLS へ明示的に追加すること:\n{}",
        render(&violations)
    );
}

#[test]
fn every_direct_call_exception_still_has_a_real_hit() {
    // 「違反 0 件」は道具が壊れていても成立する。表の全項目に実際の当たりがあることを
    // 要求すると、走査が空振りしていれば必ず赤になる（＝上の検査の非空虚性の担保）。
    let hits = scan_sources(&watched_sources(), &direct_call_tokens());
    let stale = stale_entries(&hits, ALLOWED_DIRECT_CALLS);
    assert!(
        stale.is_empty(),
        "例外表に載っているのに当たりが 1 件も無い（走査が空振りしているか、事情が消えた）: {stale:?}"
    );
}

#[test]
fn the_direct_call_allow_table_declares_its_own_size_and_reasons() {
    assert_eq!(
        ALLOWED_DIRECT_CALLS.len(),
        ALLOWED_DIRECT_CALLS_COUNT,
        "例外表の件数が宣言と食い違う。項目の追加は表と件数の 2 箇所を明示的に編集すること（要件 8.2）"
    );
    let sources = walk_workspace_sources();
    for (path, reason) in ALLOWED_DIRECT_CALLS {
        assert!(
            !reason.trim().is_empty(),
            "例外には理由が要る（要件 8.1）: {path}"
        );
        assert!(
            !path.contains('*') && path.ends_with(".rs"),
            "例外はファイル 1 件を逐語で指すこと（総括的な指定は暗黙の増加を許す）: {path}"
        );
        assert!(
            sources.contains(&(*path).to_string()),
            "例外表が実在しないファイルを指している: {path}"
        );
    }
}

#[test]
fn the_shared_crate_definition_is_the_only_excluded_region_and_it_really_holds_the_calls() {
    // 除外が「効いている」ことの陽性側。除外領域には確かに直接呼出があり、
    // 除外を外せば上の見張りは赤になる（＝除外が飾りでない）。
    let hits = scan_sources(&kit_definition_sources(), &direct_call_tokens());
    let files: BTreeSet<String> = hits.iter().map(|h| h.path.clone()).collect();
    assert_eq!(
        files,
        BTreeSet::from([
            "crates/log-capture-kit/src/capture.rs".to_string(),
            "crates/log-capture-kit/src/filter.rs".to_string(),
            "crates/log-capture-kit/src/global.rs".to_string(),
        ]),
        "共有機構の定義箇所が動いている。要件 1.1 は定義を 1 箇所に保つことを求めている"
    );
}

#[test]
fn the_guard_file_itself_is_not_a_hit_because_the_tokens_are_never_spelled_out() {
    // 本ファイルが走査語を逐語で持つと、見張りが自分自身を拾って例外表が膨らむ。
    let me = "crates/log-capture-kit/tests/with_default_guard_test.rs";
    let mut tokens = direct_call_tokens();
    tokens.extend(global_capture_tokens());
    assert_eq!(
        scan_sources(&[me.to_string()], &tokens),
        Vec::new(),
        "見張り自身が走査語を逐語で持ってしまっている（concat! で割ること）"
    );
}

/// 走査語を逐語で持ってはいけないファイル群（見張りと走査器）。
const FILES_THAT_MUST_NOT_SPELL_THE_TOKENS: &[&str] = &[
    "crates/log-capture-kit/tests/with_default_guard_test.rs",
    "crates/log-capture-kit/tests/workspace_scan/mod.rs",
    "crates/log-capture-kit/tests/workspace_scan_test.rs",
];

/// コメントを**除かずに**生テキストから走査語を探す（純関数）。
///
/// 着手前インベントリの `rg -l` と同じ見方（アンカーもコメント除去もしない素の部分一致）に
/// そろえてある。**母数が動く条件をそのまま写すのが目的なので、ここを賢くしてはいけない。**
fn raw_occurrences(src: &str, tokens: &[&str]) -> Vec<(usize, String)> {
    let mut hits = Vec::new();
    for (index, line) in src.lines().enumerate() {
        for token in tokens {
            if line.contains(token) {
                hits.push((index + 1, (*token).to_string()));
            }
        }
    }
    hits
}

#[test]
fn the_guard_files_never_spell_the_tokens_out_not_even_inside_comments() {
    let mut tokens = direct_call_tokens();
    tokens.extend(global_capture_tokens());

    // 較正: 生テキスト走査はコメントを除かない。除いてしまうと本検査は恒真になる
    //（コメント除去を通す上の検査は、コメントへ植えた走査語を 1 件も拾わない）。
    let planted = format!(
        "// 説明: かつては tracing::subscriber::{TOKEN_WITH_DEFAULT}sub, || {{}}); と書いていた"
    );
    assert_eq!(
        raw_occurrences(&planted, &tokens),
        vec![(1usize, TOKEN_WITH_DEFAULT.to_string())],
        "コメントの中でも生テキスト走査は当たらねばならない（当たらないなら本検査は恒真）"
    );

    let mut violations = Vec::new();
    for path in FILES_THAT_MUST_NOT_SPELL_THE_TOKENS {
        for (line, token) in raw_occurrences(&read_source(path), &tokens) {
            violations.push(format!("  {path}:{line} ({token})"));
        }
    }
    assert!(
        violations.is_empty(),
        "走査語はコメントの中も含めて逐語で書いてはいけない。         ⒜ 見張りが自分自身を違反として拾い ⒝ 着手前インベントリの `rg -l` による         捕捉サイト計数の母数が黙って動く（コメントは見張りが除去するが `rg` は除去しない）。         concat! で 2 片に割ること:
{}",
        violations.join("
")
    );
}

// ---------------------------------------------------------------------------
// ⑵ 全スレッド捕捉
// ---------------------------------------------------------------------------

#[test]
fn every_use_of_the_all_thread_capture_is_listed_with_a_reason() {
    let hits = scan_sources(&watched_sources(), &global_capture_tokens());
    let violations = unlisted(&hits, ALLOWED_GLOBAL_CAPTURE);
    assert!(
        violations.is_empty(),
        "全スレッド捕捉は既定の捕捉 API と意味論が違う（窓の外・他スレッドのイベントも入る）。\
         利用するファイルは理由付きで ALLOWED_GLOBAL_CAPTURE へ明示的に追加すること（要件 1.6）:\n{}",
        render(&violations)
    );
}

#[test]
fn every_all_thread_capture_exception_still_has_a_real_hit() {
    let hits = scan_sources(&watched_sources(), &global_capture_tokens());
    let stale = stale_entries(&hits, ALLOWED_GLOBAL_CAPTURE);
    assert!(
        stale.is_empty(),
        "別表に載っているのに当たりが 1 件も無い（走査が空振りしているか、利用が消えた）: {stale:?}"
    );
}

#[test]
fn the_all_thread_capture_table_declares_its_own_size_and_reasons() {
    assert_eq!(
        ALLOWED_GLOBAL_CAPTURE.len(),
        ALLOWED_GLOBAL_CAPTURE_COUNT,
        "別表の件数が宣言と食い違う。項目の追加は表と件数の 2 箇所を明示的に編集すること（要件 8.2）"
    );
    let sources = walk_workspace_sources();
    for (path, reason) in ALLOWED_GLOBAL_CAPTURE {
        assert!(!reason.trim().is_empty(), "別表には理由が要る: {path}");
        assert!(
            !path.contains('*') && path.ends_with(".rs"),
            "別表はファイル 1 件を逐語で指すこと: {path}"
        );
        assert!(
            sources.contains(&(*path).to_string()),
            "別表が実在しないファイルを指している: {path}"
        );
    }
}

// ---------------------------------------------------------------------------
// ⑶ 依存方向
// ---------------------------------------------------------------------------

/// 共有 crate を dev-dependency として引いている crate（実測の陽性側・較正）。
const KNOWN_DEV_DEPENDENTS: &[&str] = &[
    "areka",
    "areka-emo-atlas",
    "areka-emo-compose",
    "areka-emo-present",
    "areka-emo-text",
    "areka-ghost",
    "areka-kanade",
    "areka-seriko",
    "areka-sylphya",
    "wintf",
];

#[test]
fn the_shared_crate_never_appears_in_a_production_dependency_table() {
    let mut violations: Vec<String> = Vec::new();
    for (name, text) in workspace_manifests() {
        for line in production_kit_dependencies(&text) {
            violations.push(format!(
                "  crates/{name}/Cargo.toml:{} [{}] {}",
                line.line, line.section, line.text
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "共有 crate はテスト専用で、製品側の依存に現れてはいけない（要件 1.3・11.5）:\n{}",
        violations.join("\n")
    );
}

#[test]
fn the_shared_crate_is_actually_pulled_in_as_a_dev_dependency() {
    // 較正: 上の検査が「manifest を 1 つも読めていないから空」で通っていないことを示す。
    let manifests = workspace_manifests();
    assert!(
        manifests.len() > 15,
        "manifest の列挙が極端に少ない＝走査が空振りしている疑い: {} 件",
        manifests.len()
    );
    let dev: BTreeMap<String, bool> = manifests
        .iter()
        .map(|(name, text)| (name.clone(), has_dev_kit_dependency(text)))
        .collect();
    for known in KNOWN_DEV_DEPENDENTS {
        assert_eq!(
            dev.get(*known),
            Some(&true),
            "共有 crate を dev-dependency として引いているはずの crate で検出できない: {known}"
        );
    }
}

// ---------------------------------------------------------------------------
// ⑷ env-filter フィーチャ
// ---------------------------------------------------------------------------

#[test]
fn only_one_crate_declares_the_env_filter_feature() {
    let owners: BTreeSet<String> = workspace_manifests()
        .into_iter()
        .filter(|(_, text)| declares_env_filter_feature(text))
        .map(|(name, _)| name)
        .collect();
    assert_eq!(
        owners,
        BTreeSet::from([ENV_FILTER_OWNER.to_string()]),
        "フィーチャはワークスペースで統合されるので、1 crate が env-filter を宣言すると \
         全 crate から capture_under_filter が届く。宣言してよいのは wintf だけ"
    );
}

// ---------------------------------------------------------------------------
// 純関数の較正（要件 8.4）
// ---------------------------------------------------------------------------

fn sample_hits() -> Vec<Hit> {
    vec![
        Hit::new("crates/demo/src/a_tests.rs", 12, TOKEN_WITH_DEFAULT),
        Hit::new("crates/demo/src/b_tests.rs", 34, TOKEN_SET_DEFAULT),
        Hit::new("crates/demo/tests/c_test.rs", 56, TOKEN_SET_GLOBAL_DEFAULT),
    ]
}

#[test]
fn unlisted_is_empty_only_while_every_hit_is_in_the_allow_table() {
    let full = [
        ("crates/demo/src/a_tests.rs", "理由 A"),
        ("crates/demo/src/b_tests.rs", "理由 B"),
        ("crates/demo/tests/c_test.rs", "理由 C"),
    ];
    assert_eq!(
        unlisted(&sample_hits(), &full),
        Vec::new(),
        "全件が表に載っていれば違反は 0 件"
    );
}

#[test]
fn unlisted_returns_the_hit_whose_exception_was_dropped() {
    // 較正: 「0 件なら緑」の検査は道具が壊れていても緑になるので、
    // 例外を 1 件外したときに確かにその 1 件が返ることを固定する。
    let short = [
        ("crates/demo/src/a_tests.rs", "理由 A"),
        ("crates/demo/tests/c_test.rs", "理由 C"),
    ];
    assert_eq!(
        unlisted(&sample_hits(), &short),
        vec![Hit::new(
            "crates/demo/src/b_tests.rs",
            34,
            TOKEN_SET_DEFAULT
        )],
        "例外表から外した 1 件だけが返るはず"
    );
}

#[test]
fn stale_entries_names_the_exception_that_no_longer_has_a_hit() {
    let with_ghost = [
        ("crates/demo/src/a_tests.rs", "理由 A"),
        ("crates/demo/src/b_tests.rs", "理由 B"),
        ("crates/demo/tests/c_test.rs", "理由 C"),
        ("crates/demo/src/gone.rs", "もう当たりが無い"),
    ];
    assert_eq!(
        stale_entries(&sample_hits(), &with_ghost),
        vec!["crates/demo/src/gone.rs".to_string()],
        "当たりを失った例外だけが返るはず"
    );
    assert_eq!(
        stale_entries(&sample_hits(), &with_ghost[..3]),
        Vec::<String>::new(),
        "全項目に当たりがあれば陳腐化は 0 件"
    );
}

#[test]
fn scanning_finds_the_token_only_when_it_is_real_code() {
    // 走査そのものの両側。`scan_tokens` の較正は workspace_scan_test.rs にもあるが、
    // 本見張りが渡す語集合でも成立することをここで固定する。
    let live = format!("    tracing::subscriber::{TOKEN_WITH_DEFAULT}sub, || {{}});");
    assert_eq!(
        scan_tokens(&live, &direct_call_tokens()),
        vec![(1usize, TOKEN_WITH_DEFAULT.to_string())],
        "素の直接呼出は 1 件として出るはず"
    );

    let commented =
        format!("// かつては tracing::subscriber::{TOKEN_WITH_DEFAULT}sub, || {{}}); と書いていた");
    assert_eq!(
        scan_tokens(&commented, &direct_call_tokens()),
        Vec::new(),
        "コメントの中の走査語は違反ではない"
    );

    let global = format!("    let sink = {TOKEN_INSTALL_GLOBAL_CAPTURE});");
    assert_eq!(
        scan_tokens(&global, &global_capture_tokens()),
        vec![(1usize, TOKEN_INSTALL_GLOBAL_CAPTURE.to_string())],
        "全スレッド捕捉の窓口も 1 件として出るはず"
    );
    assert_eq!(
        scan_tokens(&global, &direct_call_tokens()),
        Vec::new(),
        "⑴ の語集合は全スレッド捕捉の窓口に当たってはいけない（表を取り違える）"
    );
}

/// manifest の見本。依存行だけを差し替えて両側を作る（依存行は必ず 6 行目）。
fn manifest_fixture(section: &str, line: &str) -> String {
    format!("[package]\nname = \"demo\"\n\n[{section}]\ntracing = {{ workspace = true }}\n{line}\n")
}

/// 見本の依存行（共有 crate を引く素の形）。
const KIT_DEP_LINE: &str = "log-capture-kit = { path = \"../log-capture-kit\" }";

#[test]
fn a_production_dependency_on_the_shared_crate_is_a_violation() {
    for section in ["dependencies", "build-dependencies"] {
        let src = manifest_fixture(section, KIT_DEP_LINE);
        let found = production_kit_dependencies(&src);
        assert_eq!(
            found.len(),
            1,
            "[{section}] の共有 crate 依存は違反として出るはず: {found:?}"
        );
        assert_eq!(found[0].line, 6, "行番号は元の manifest のもの");
        assert_eq!(found[0].section, section);
    }
}

#[test]
fn a_dev_dependency_on_the_shared_crate_is_not_a_violation() {
    for section in ["dev-dependencies", "target.'cfg(windows)'.dev-dependencies"] {
        let src = manifest_fixture(section, KIT_DEP_LINE);
        assert_eq!(
            production_kit_dependencies(&src),
            Vec::new(),
            "[{section}] は開発側なので違反ではない"
        );
        assert!(
            has_dev_kit_dependency(&src),
            "[{section}] は開発側の依存として検出されるはず"
        );
    }
}

#[test]
fn a_target_specific_production_dependency_is_still_a_violation() {
    let src = manifest_fixture("target.'cfg(windows)'.dependencies", KIT_DEP_LINE);
    assert_eq!(
        production_kit_dependencies(&src).len(),
        1,
        "プラットフォーム別の製品側依存も違反"
    );
}

#[test]
fn a_sub_table_production_dependency_is_still_a_violation() {
    // `[dependencies.log-capture-kit]` の形。見出し自身が名前を持つので行の中身には現れない。
    let src = "[package]\nname = \"demo\"\n\n[dependencies.log-capture-kit]\npath = \"../log-capture-kit\"\n";
    let found = production_kit_dependencies(src);
    assert_eq!(found.len(), 1, "下位表の形も拾うはず: {found:?}");
    assert!(found[0].is_header, "見出し行 1 件として報告されるはず");
}

#[test]
fn a_commented_out_dependency_is_not_a_violation() {
    let src = manifest_fixture(
        "dependencies",
        "# log-capture-kit = { path = \"../log-capture-kit\" }",
    );
    assert_eq!(
        production_kit_dependencies(&src),
        Vec::new(),
        "コメントアウトされた依存は違反ではない"
    );

    let trailing = manifest_fixture(
        "dependencies",
        "tracing-subscriber = { workspace = true } # log-capture-kit ではない",
    );
    assert_eq!(
        production_kit_dependencies(&trailing),
        Vec::new(),
        "行末コメントの中の名前を拾ってはいけない"
    );
}

#[test]
fn a_hash_inside_a_quoted_value_is_not_a_comment() {
    // 較正: コメント除去が乱暴だと引用符の中で行が切れ、依存行が消えて違反を見落とす。
    let src = "[dependencies]\ndemo = { git = \"https://example.invalid/x#tag\", package = \"log-capture-kit\" }\n";
    assert_eq!(
        production_kit_dependencies(src).len(),
        1,
        "引用符の内側の # で行を切ってはいけない"
    );
}

#[test]
fn the_env_filter_feature_is_detected_only_when_it_is_actually_declared() {
    let on = manifest_fixture(
        "dev-dependencies",
        "log-capture-kit = { path = \"../log-capture-kit\", features = [\"env-filter\"] }",
    );
    assert!(declares_env_filter_feature(&on), "宣言を検出できていない");

    let off = manifest_fixture("dev-dependencies", KIT_DEP_LINE);
    assert!(
        !declares_env_filter_feature(&off),
        "フィーチャ指定の無い依存を宣言と誤読している"
    );

    let commented = manifest_fixture(
        "dev-dependencies",
        "# log-capture-kit = { path = \"../log-capture-kit\", features = [\"env-filter\"] }",
    );
    assert!(
        !declares_env_filter_feature(&commented),
        "コメントアウトされた宣言を拾ってはいけない"
    );

    let other_crate = manifest_fixture(
        "dev-dependencies",
        "tracing-subscriber = { workspace = true, features = [\"env-filter\"] }",
    );
    assert!(
        !declares_env_filter_feature(&other_crate),
        "共有 crate 以外の依存のフィーチャを拾ってはいけない"
    );
}

#[test]
fn manifest_lines_track_the_section_and_drop_comments() {
    let src = "# 先頭コメント\n[package]\nname = \"demo\"\n\n[dev-dependencies]\n# 説明\nlog-capture-kit = { path = \"../log-capture-kit\" } # 行末\n";
    let lines = manifest_lines(src);
    let last = lines.last().expect("最終行があるはず");
    assert_eq!(last.line, 7, "行番号は元のソースのもの");
    assert_eq!(last.section, "dev-dependencies");
    assert!(
        !last.text.contains("行末"),
        "行末コメントが残っている: {last:?}"
    );
    assert!(
        !lines.iter().any(|l| l.text.contains("説明")),
        "コメント専用行が残っている"
    );
}
