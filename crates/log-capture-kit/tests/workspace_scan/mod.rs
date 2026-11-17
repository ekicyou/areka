//! ワークスペース走査の部品（要件 8.3・8.4・10.3）。
//!
//! 要件の番号は、spec の名を添えていないものは spec `areka-P0-test-cage-determinism` のもの。
//!
//! 見張りテスト（要件 8 の迂回検知・要件 10 の 1,000 行番人）が共通で使う道具箱で、
//! 判定そのものは持たない。判定に使う語や例外表は消費側の見張りが `const` で持つ。
//!
//! # 部品の分け方
//!
//! ファイルシステムに触るのは木の読みの置き場（`sources`）・[`read_source`]・
//! [`workspace_manifests`]（と下請けの列挙）だけで、[`walk_workspace_sources`]・
//! [`measure_workspace_sources`] は置き場から読む。残り（[`skip_dir`]・[`strip_comments`]・
//! [`scan_tokens`]・[`line_count`]・[`over_limit`] ほか）は**入出力だけで完結する純粋な関数**である。
//! 純粋な側は文字列の見本で自己較正できるので、「違反 0 件だから緑」という形の見張りが
//! 道具の故障で緑になっていないことを別立てで示せる（較正は `tests/workspace_scan_test.rs`）。
//!
//! # 語の一致規則
//!
//! [`scan_tokens`] は語の**左端をアンカーする**（直前の文字が `[A-Za-z0-9_]` なら不一致）。
//! アンカーが無いと、名前の終わりがたまたま走査語と同じ関数（例: `test_off` を前に置いた
//! `set_default` ＋開き括弧の形）が部分一致して偽陽性になる。
//!
//! アンカーが効くのは**語の 1 文字目が `[A-Za-z0-9_]` のとき**だけである。区切り文字から
//! 始まる語（例: パスの区切りを先頭に置いて「名前を継ぎ足した形」だけを狙う語）にまで
//! アンカーを適用すると、直前が識別子文字である限り**絶対に当たらない**語になり、
//! 見張りが黙って恒真になる。判定は [`needs_left_anchor`] が持ち、較正は
//! `workspace_scan_test.rs` の
//! `scan_tokens_anchors_only_tokens_that_start_with_an_identifier_char` が縛る。
//!
//! **本ファイルは走査語を、開き括弧の有無を問わず 1 度も逐語で書かない。** 走査の対象そのものなので、
//! 逐語で置くと ⑴ 迂回検知の見張り（要件 8）が自分自身を拾い ⑵ 着手前インベントリの
//! `rg -l` による捕捉サイト計数の母数が動く。以下の doc も同じ約束で書く。
//!
//! # 行数の定義
//!
//! [`line_count`] は**改行の個数**を返す。着手前インベントリ
//!（`.kiro/specs/completed/areka-P0-test-cage-determinism/verification/remeasure.md` §6）が
//! 同じ定義で上限越えを採っているので、番人の例外表と数え方が一致する。

// 本 module は見張りの試験対象のどれからも共有され、どの試験対象も道具箱の一部しか
// 使わない。未使用の警告は共有の副作用なので黙らせる。
#![allow(dead_code)]

use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// 1 ファイルの行数の上限（`structure.md` の Unit Tests の節の「1 ファイル 1,000 行以下の目安」）。
pub const LINE_LIMIT: usize = 1000;

/// 列挙で入らないディレクトリか（純関数）。
///
/// 飛ばすのはビルドの出力＝`Cargo.toml` の隣の `target` だけ。モジュールを割った
/// `src/<module>/target/` のようなフォルダは名前が同じでも飛ばさない（飛ばすと割ったファイルが
/// 見張りから黙って消える）。走査は `crates/` から始まるので、根の `vendors` と `.git` は
/// ここで名指ししない。
pub fn skip_dir(name: &str, parent_has_manifest: bool) -> bool {
    parent_has_manifest && name == "target"
}

/// 走査したファイルとその行数の組。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileLines {
    /// ワークスペース根からの相対パス（区切りは `/` に正規化済み）。
    pub path: String,
    /// [`line_count`] の定義による行数。
    pub lines: usize,
}

impl FileLines {
    pub fn new(path: impl Into<String>, lines: usize) -> Self {
        Self {
            path: path.into(),
            lines,
        }
    }
}

/// ワークスペース根の絶対パス。
///
/// 本 crate の manifest は `crates/log-capture-kit/` にあるので 2 段上が根になる。
pub fn workspace_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .expect("crates/<crate>/ の 2 段上がワークスペース根のはず")
        .to_path_buf()
}

/// `crates/**/*.rs` を列挙する。
///
/// 本番（`src/`）・テスト（`tests/`）・実行例（`examples/`）・本番の隣に置いた兄弟テスト
/// ファイル（`<stem>_*.rs`）をすべて含み、ビルドの出力（[`skip_dir`]）は除く。
/// 戻り値はワークスペース根からの相対パスで、区切りは `/`、**昇順・重複無し**。
/// 順序を固定するのは、見張りが落ちたときの出力を再現可能にするため。
pub fn walk_workspace_sources() -> Vec<String> {
    sources().iter().map(|(path, _)| path.clone()).collect()
}

/// 列挙した全ソースの `(相対パス, 本文)`（相対パスの昇順）。
///
/// 見張りはどれも木全体を読むので、テストの実行体ごとに 1 度だけ読んで置いておく（audit-test-kits 決め D7）。
fn sources() -> &'static [(String, String)] {
    static SOURCES: OnceLock<Vec<(String, String)>> = OnceLock::new();
    SOURCES.get_or_init(|| {
        let root = workspace_root();
        let mut found = Vec::new();
        collect_rs_files(&root.join("crates"), &root, &mut found);
        found.sort();
        found.dedup();
        found
            .into_iter()
            .map(|path| {
                let text = read_from_disk(&root, &path);
                (path, text)
            })
            .collect()
    })
}

fn collect_rs_files(dir: &Path, root: &Path, out: &mut Vec<String>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => panic!("列挙できないディレクトリがある: {} ({err})", dir.display()),
    };
    let has_manifest = dir.join("Cargo.toml").is_file();
    for entry in entries {
        let entry = entry.expect("ディレクトリ項目の読み取りに失敗した");
        let path = entry.path();
        let file_type = entry.file_type().expect("種別の判定に失敗した");
        let name = entry.file_name().to_string_lossy().into_owned();
        if file_type.is_dir() {
            if skip_dir(&name, has_manifest) {
                continue;
            }
            collect_rs_files(&path, root, out);
        } else if file_type.is_file() && name.ends_with(".rs") {
            let rel = path
                .strip_prefix(root)
                .expect("列挙したパスはワークスペース根の下にあるはず");
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// ワークスペース根からの相対パスでソースを読む（不正な UTF-8 は置換文字にする）。
///
/// 列挙に入るファイルは置き場から返し、入らないファイルだけをディスクから読む。
pub fn read_source(rel_path: &str) -> String {
    let all = sources();
    match all.binary_search_by(|(path, _)| path.as_str().cmp(rel_path)) {
        Ok(at) => all[at].1.clone(),
        Err(_) => read_from_disk(&workspace_root(), rel_path),
    }
}

/// 列挙した `at` 番目のソースの、コメントを除いた本文。
///
/// 見張りは同じ木を語の組を変えて何度も走査するので、除去はファイルごとに 1 度だけ行って置いておく。
fn stripped_source(at: usize) -> &'static str {
    static STRIPPED: OnceLock<Vec<OnceLock<String>>> = OnceLock::new();
    let all = STRIPPED.get_or_init(|| sources().iter().map(|_| OnceLock::new()).collect());
    all[at].get_or_init(|| strip_comments(&sources()[at].1))
}

fn read_from_disk(root: &Path, rel_path: &str) -> String {
    let full = root.join(rel_path);
    let bytes = std::fs::read(&full)
        .unwrap_or_else(|err| panic!("ソースを読めない: {} ({err})", full.display()));
    String::from_utf8_lossy(&bytes).into_owned()
}

/// 列挙した全ソースの行数を測る。
pub fn measure_workspace_sources() -> Vec<FileLines> {
    sources()
        .iter()
        .map(|(path, text)| FileLines::new(path.clone(), line_count(text)))
        .collect()
}

/// 行数＝改行の個数。末尾に改行が無い最後の行は数えない（着手前インベントリと同じ定義）。
pub fn line_count(src: &str) -> usize {
    src.bytes().filter(|b| *b == b'\n').count()
}

/// 上限を超え、かつ例外表に載っていないファイルを返す。
///
/// 上限**ちょうど**は超過ではない（着手前インベントリの境界と同じ）。
pub fn over_limit(files: &[FileLines], allow: &[&str]) -> Vec<FileLines> {
    files
        .iter()
        .filter(|f| f.lines > LINE_LIMIT && !allow.contains(&f.path.as_str()))
        .cloned()
        .collect()
}

/// コメント（`//`・`//!`・`///`・行末コメント・入れ子を含む `/* */`）を取り除く。
///
/// **行の構成は変えない**——落としたコメントの分だけ空白が減るだけで、改行は 1 個も
/// 増減しない。[`scan_tokens`] が返す行番号が元のソースの行番号と一致するのはこのため。
///
/// 文字列リテラル（素・raw・byte）と文字リテラルの中身は**残す**。中の `//` を
/// コメントと誤認すると、その行の残りが丸ごと消えて違反を見落とす。
pub fn strip_comments(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(src.len());
    let mut i = 0usize;

    while i < n {
        let c = chars[i];

        // 行コメント（`//`・`///`・`//!` を含む）。改行そのものは次の周回で複写される。
        if c == '/' && i + 1 < n && chars[i + 1] == '/' {
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // 塊コメント。Rust の仕様どおり入れ子を数える。改行だけ複写して行番号を保つ。
        if c == '/' && i + 1 < n && chars[i + 1] == '*' {
            let mut depth = 1usize;
            i += 2;
            while i < n && depth > 0 {
                if chars[i] == '/' && i + 1 < n && chars[i + 1] == '*' {
                    depth += 1;
                    i += 2;
                } else if chars[i] == '*' && i + 1 < n && chars[i + 1] == '/' {
                    depth -= 1;
                    i += 2;
                } else {
                    if chars[i] == '\n' {
                        out.push('\n');
                    }
                    i += 1;
                }
            }
            continue;
        }

        // raw 文字列（`r"…"`・`r#"…"#`・`br#"…"#`）。中の `//` はコメントではない。
        if let Some((start, hashes)) = raw_string_opening(&chars, i) {
            copy_raw_string(&chars, &mut out, i, start, hashes, &mut i);
            continue;
        }

        // 素の文字列（byte 文字列 `b"…"` もここを通る）。
        if c == '"' {
            out.push(c);
            i += 1;
            while i < n {
                let d = chars[i];
                out.push(d);
                i += 1;
                if d == '\\' {
                    if i < n {
                        out.push(chars[i]);
                        i += 1;
                    }
                } else if d == '"' {
                    break;
                }
            }
            continue;
        }

        // 文字リテラルとライフタイム注記の区別。`'"'` を素の引用符と読むと以降が
        // 文字列状態に落ちて、行の残りが消える。
        if c == '\'' && is_char_literal_start(&chars, i) {
            out.push(c);
            i += 1;
            while i < n {
                let d = chars[i];
                out.push(d);
                i += 1;
                if d == '\\' {
                    if i < n {
                        out.push(chars[i]);
                        i += 1;
                    }
                } else if d == '\'' {
                    break;
                }
            }
            continue;
        }

        out.push(c);
        i += 1;
    }

    out
}

/// `i` から raw 文字列が始まるなら `(引用符の位置, `#` の個数)` を返す。
fn raw_string_opening(chars: &[char], i: usize) -> Option<(usize, usize)> {
    if i > 0 && is_ident_char(chars[i - 1]) {
        return None;
    }
    let mut cursor = i;
    if chars.get(cursor) == Some(&'b') {
        cursor += 1;
    }
    if chars.get(cursor) != Some(&'r') {
        return None;
    }
    cursor += 1;
    let hash_start = cursor;
    while chars.get(cursor) == Some(&'#') {
        cursor += 1;
    }
    if chars.get(cursor) != Some(&'"') {
        return None;
    }
    Some((cursor, cursor - hash_start))
}

fn copy_raw_string(
    chars: &[char],
    out: &mut String,
    from: usize,
    quote: usize,
    hashes: usize,
    cursor: &mut usize,
) {
    let n = chars.len();
    for &c in &chars[from..=quote] {
        out.push(c);
    }
    let mut i = quote + 1;
    while i < n {
        let c = chars[i];
        out.push(c);
        i += 1;
        if c == '"' {
            let mut seen = 0usize;
            while seen < hashes && chars.get(i + seen) == Some(&'#') {
                seen += 1;
            }
            if seen == hashes {
                for &h in &chars[i..i + hashes] {
                    out.push(h);
                }
                i += hashes;
                break;
            }
        }
    }
    *cursor = i;
}

/// `'` から**文字リテラル**が始まるか（偽ならライフタイム注記）。
fn is_char_literal_start(chars: &[char], i: usize) -> bool {
    match chars.get(i + 1) {
        // `'\n'` `'\''` `'\u{1F600}'` などのエスケープは必ず文字リテラル。
        Some('\\') => true,
        // 1 文字ぶん進んだ先が閉じ引用符なら文字リテラル。`'a>` `'a,` はライフタイム。
        Some(_) => chars.get(i + 2) == Some(&'\''),
        None => false,
    }
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// コメントを除いたソースから走査語を探し、`(1 始まりの行番号, 当たった語)` を返す。
///
/// 語の左端はアンカーされる（module doc を参照）。語同士が重なる場合の規則は 2 つ。
///
/// 1. **同じ位置**に複数の語が当たるとき（＝一方が他方の接頭辞のとき。それ以外の形で
///    同じ位置に 2 語が当たることはない）は、**長い語**を 1 件だけ返す。
/// 2. 当たった語の**内側**からは次の語を探さない。1 つの呼出が、修飾つきの語と裸の語の
///    両方に当たって 2 件に数えられるのを防ぐ。
///
/// 規則 2 は本番で生きている: 一時パスの見張りの修飾つきの綴り（`env::` を前に置いた形）は
/// 後ろ半分で裸の綴りと重なり、規則 2 が無いと 1 つの呼出が 2 件に数えられる。語の一覧を持つのは
/// 見張り側なので、語が増えたときに黙って壊れないよう規則として固定してある。
/// 規則が生きていることは較正テストが縛る（`workspace_scan_test.rs` の
/// `scan_tokens_returns_the_longer_of_two_tokens_that_start_at_the_same_place` と
/// `scan_tokens_does_not_count_one_call_twice_when_a_longer_token_contains_a_shorter_one`）。
pub fn scan_tokens<T: AsRef<str>>(src: &str, tokens: &[T]) -> Vec<(usize, String)> {
    scan_stripped(&strip_comments(src), tokens)
}

/// [`scan_tokens`] の中身。コメントを除いた後の本文を受ける。
fn scan_stripped<T: AsRef<str>>(stripped: &str, tokens: &[T]) -> Vec<(usize, String)> {
    let mut order: Vec<&str> = tokens.iter().map(AsRef::as_ref).collect();
    order.sort_by_key(|t| std::cmp::Reverse(t.len()));
    // 語を頭の文字ごとの組に 1 度だけ分け（組の中は長い順のまま）、文字ごとに二分探索で引く
    // （audit-test-kits 決め D7）。どの語の頭でもない文字では語の輪を回さない。
    let mut groups: Vec<(char, Vec<&str>)> = Vec::new();
    for token in order {
        let Some(first) = token.chars().next() else {
            continue;
        };
        match groups.binary_search_by_key(&first, |(c, _)| *c) {
            Ok(at) => groups[at].1.push(token),
            Err(at) => groups.insert(at, (first, vec![token])),
        }
    }

    let mut hits = Vec::new();
    for (index, line) in stripped.lines().enumerate() {
        let mut prev_is_ident = false;
        let mut skip_until = 0usize;
        for (at, ch) in line.char_indices() {
            let group = if at >= skip_until {
                groups.binary_search_by_key(&ch, |(c, _)| *c).ok()
            } else {
                None
            };
            if let Some(group) = group {
                for token in &groups[group].1 {
                    if prev_is_ident && needs_left_anchor(token) {
                        continue;
                    }
                    if line[at..].starts_with(token) {
                        hits.push((index + 1, (*token).to_string()));
                        skip_until = at + token.len();
                        break;
                    }
                }
            }
            prev_is_ident = is_ident_char(ch);
        }
    }
    hits
}

// ---------------------------------------------------------------------------
// `Cargo.toml` の行分解と本番依存表の抽出（areka-P0-nar-install 要件 10.3）
// ---------------------------------------------------------------------------
//
// `with_default_guard_test.rs` が私有で持っていた部品を、クレート名を引数に取る形へ移した
// もの。共有 crate の見張り（要件 1.3・11.5）と検体 crate の見張り（areka-P0-nar-install 要件 1.7）が同じ部品を
// 呼ぶ。判定と例外表は消費側の見張りが持つ。

/// `Cargo.toml` の 1 行（コメント除去済み・所属セクションつき）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestLine {
    /// 1 始まりの行番号。
    pub line: usize,
    /// 直近のセクション見出し（角括弧を外した中身）。
    pub section: String,
    /// コメントを除き前後の空白を落とした行。
    pub text: String,
    /// この行自身がセクション見出しか。
    pub is_header: bool,
}

/// TOML の行末コメントを落とす。引用符の内側の `#` はコメントではない。
///
/// 開いた引用符の種類を覚える（`"…"` の中の `'` や、`'…'` の中の `"` では閉じない）。
/// 逃がしの `\` は `"…"` の中だけにある（`'…'` は逃がしを持たない）。
fn strip_toml_comment(line: &str) -> &str {
    let mut open: Option<char> = None;
    let mut escaped = false;
    for (at, ch) in line.char_indices() {
        match open {
            Some(quote) => {
                if escaped {
                    escaped = false;
                } else if quote == '"' && ch == '\\' {
                    escaped = true;
                } else if ch == quote {
                    open = None;
                }
            }
            None => match ch {
                '"' | '\'' => open = Some(ch),
                '#' => return &line[..at],
                _ => {}
            },
        }
    }
    line
}

/// `Cargo.toml` を「コメントを除いた非空行 ＋ 所属セクション」へ分解する（純関数）。
pub fn manifest_lines(src: &str) -> Vec<ManifestLine> {
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
pub fn is_production_dependency_section(section: &str) -> bool {
    section.contains("dependencies") && !section.contains("dev-dependencies")
}

/// `package` の crate を名指ししているか（`-` 表記と `_` 表記の両方）。
pub fn mentions_crate(text: &str, package: &str) -> bool {
    text.contains(package) || text.contains(&package.replace('-', "_"))
}

/// 製品側依存に `package` の crate が現れている行を返す（純関数・要件 1.3／11.5・areka-P0-nar-install 要件 10.3）。
///
/// `[dependencies]` の中の 1 行という形と、`[dependencies.<package>]` という
/// 下位表の形の両方を拾う。下位表は**見出し 1 件**として報告する（見出しが名前を持つ表では
/// 中身の行も `path = "../<package>"` のように名前を含みがちで、1 つの依存が
/// 複数件に膨らむ）。
pub fn production_dependencies_on(src: &str, package: &str) -> Vec<ManifestLine> {
    manifest_lines(src)
        .into_iter()
        .filter(|l| is_production_dependency_section(&l.section))
        .filter(|l| {
            if l.is_header {
                mentions_crate(&l.section, package)
            } else {
                !mentions_crate(&l.section, package) && mentions_crate(&l.text, package)
            }
        })
        .collect()
}

/// 語の左端をアンカーすべきか（＝1 文字目が識別子文字か）。
///
/// 区切り文字から始まる語をアンカーすると恒真の走査語になる（module doc を参照）。
pub fn needs_left_anchor(token: &str) -> bool {
    token.chars().next().is_some_and(is_ident_char)
}

// ---------------------------------------------------------------------------
// `crates/**/Cargo.toml` の列挙（areka-P0-nar-install 要件 10.3）
// ---------------------------------------------------------------------------

/// `crates/**/Cargo.toml` を列挙して `(crate ディレクトリ名, 中身)` を返す。
///
/// `skip_crate` と同じディレクトリ名の manifest は除く。見張られる側の crate は自分の名前を
/// `[package]` に持つので、比較の対象に入れると必ず当たってしまう。
pub fn workspace_manifests(skip_crate: &str) -> Vec<(String, String)> {
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
            if name == skip_crate {
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
    let has_manifest = dir.join("Cargo.toml").is_file();
    for entry in entries {
        let entry = entry.expect("ディレクトリ項目の読み取りに失敗した");
        let path = entry.path();
        let file_type = entry.file_type().expect("種別の判定に失敗した");
        let name = entry.file_name().to_string_lossy().into_owned();
        if file_type.is_dir() {
            if skip_dir(&name, has_manifest) {
                continue;
            }
            collect_manifests(&path, out);
        } else if file_type.is_file() && name == "Cargo.toml" {
            out.push(path);
        }
    }
}

// ---------------------------------------------------------------------------
// 見張りが共有する取り回し（判定の語と例外表は各見張りが持つ）
// ---------------------------------------------------------------------------

/// 走査で当たった 1 件。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hit {
    /// ワークスペース根からの相対パス（区切りは `/`）。
    pub path: String,
    /// 1 始まりの行番号。
    pub line: usize,
    /// 当たった語。
    pub token: String,
}

impl Hit {
    pub fn new(path: impl Into<String>, line: usize, token: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            line,
            token: token.into(),
        }
    }
}

impl std::fmt::Display for Hit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{} （語: {}）", self.path, self.line, self.token)
    }
}

impl std::fmt::Display for FileLines {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({} 行)", self.path, self.lines)
    }
}

/// 与えられたファイル群を走査する。
///
/// 列挙に入るファイルは、コメントを除いた本文の置き場（[`stripped_source`]）から読む。
pub fn scan_sources<T: AsRef<str>>(paths: &[String], tokens: &[T]) -> Vec<Hit> {
    let all = sources();
    let mut hits = Vec::new();
    for path in paths {
        let found = match all.binary_search_by(|(known, _)| known.as_str().cmp(path)) {
            Ok(at) => scan_stripped(stripped_source(at), tokens),
            Err(_) => scan_tokens(&read_from_disk(&workspace_root(), path), tokens),
        };
        for (line, token) in found {
            hits.push(Hit::new(path.clone(), line, token));
        }
    }
    hits
}

/// 例外表の 1 項（先頭がパスの組）。見張りごとに表の形が違うので、パスの取り出しだけをそろえる。
pub trait AllowEntry {
    fn path(&self) -> &str;
}

impl AllowEntry for &str {
    fn path(&self) -> &str {
        self
    }
}

impl<A> AllowEntry for (&str, A) {
    fn path(&self) -> &str {
        self.0
    }
}

impl<A, B> AllowEntry for (&str, A, B) {
    fn path(&self) -> &str {
        self.0
    }
}

/// 例外表のパスだけを取り出す（判定に渡す形）。
pub fn allowed_paths<T: AllowEntry>(table: &[T]) -> Vec<&str> {
    table.iter().map(AllowEntry::path).collect()
}

/// 例外表から 1 件だけ落としたパスの一覧を作る（見張りの較正で使う）。
pub fn allowed_paths_without<'a, T: AllowEntry>(table: &'a [T], dropped: &str) -> Vec<&'a str> {
    allowed_paths(table)
        .into_iter()
        .filter(|path| *path != dropped)
        .collect()
}

/// 例外表に載っていない当たり（＝違反）を返す（純関数）。
pub fn unlisted<T: AllowEntry>(hits: &[Hit], allow: &[T]) -> Vec<Hit> {
    hits.iter()
        .filter(|h| !allow.iter().any(|entry| entry.path() == h.path))
        .cloned()
        .collect()
}

/// 例外表にあるのに当たりが 1 件も無い項目（＝陳腐化した例外）のパスを返す（純関数）。
pub fn stale_entries<T: AllowEntry>(hits: &[Hit], allow: &[T]) -> Vec<String> {
    allow
        .iter()
        .map(AllowEntry::path)
        .filter(|path| !hits.iter().any(|h| h.path == *path))
        .map(str::to_string)
        .collect()
}

/// 違反の一覧を人が読める形にする（どのファイルの何行目かが分かること）。
pub fn render<T: Display>(items: &[T]) -> String {
    items
        .iter()
        .map(|item| format!("  {item}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// コメントを**除かずに**生テキストから走査語を探す（純関数）。
///
/// 着手前の実測に使った `rg`／`grep` と同じ見方（アンカーもコメント除去もしない素の
/// 部分一致）にそろえてある。修飾つきの綴りと重なる裸の綴りも別に数える。
/// **母数が動く条件をそのまま写すのが目的なので、ここを賢くしてはいけない。**
pub fn raw_occurrences<T: AsRef<str>>(src: &str, tokens: &[T]) -> Vec<(usize, String)> {
    let mut hits = Vec::new();
    for (index, line) in src.lines().enumerate() {
        for token in tokens {
            if line.contains(token.as_ref()) {
                hits.push((index + 1, token.as_ref().to_string()));
            }
        }
    }
    hits
}

/// 開発側の依存として `package` を引いているか（純関数・較正の陽性側）。
pub fn has_dev_dependency_on(src: &str, package: &str) -> bool {
    manifest_lines(src).iter().any(|l| {
        l.section.contains("dev-dependencies")
            && if l.is_header {
                mentions_crate(&l.section, package)
            } else {
                mentions_crate(&l.text, package)
            }
    })
}

/// manifest の見本。依存行だけを差し替えて両側を作る（依存行は必ず 6 行目）。
pub fn manifest_fixture(section: &str, line: &str) -> String {
    format!("[package]\nname = \"demo\"\n\n[{section}]\ntracing = {{ workspace = true }}\n{line}\n")
}
