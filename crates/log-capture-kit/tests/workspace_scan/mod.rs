//! ワークスペース走査の部品（要件 8.3・8.4・10.3）。
//!
//! 見張りテスト（要件 8 の迂回検知・要件 10 の 1,000 行番人）が共通で使う道具箱で、
//! 判定そのものは持たない。判定に使う語や例外表は消費側の見張りが `const` で持つ。
//!
//! # 部品の分け方
//!
//! ファイルシステムに触るのは [`walk_workspace_sources`]・[`read_source`]・
//! [`measure_workspace_sources`] の 3 本だけで、残り（[`strip_comments`]・[`scan_tokens`]・
//! [`line_count`]・[`over_limit`]）は**入出力だけで完結する純粋な関数**である。
//! 純粋な側は文字列の見本で自己較正できるので、「違反 0 件だから緑」という形の見張りが
//! 道具の故障で緑になっていないことを別立てで示せる（較正は `tests/workspace_scan_test.rs`）。
//!
//! # 語の一致規則
//!
//! [`scan_tokens`] は語の**左端をアンカーする**（直前の文字が `[A-Za-z0-9_]` なら不一致）。
//! アンカーが無いと `fn test_offset_default` の開き括弧までの形（実在:
//! `crates/wintf/src/ecs/types.rs`）が走査語 `set_default` ＋開き括弧に部分一致して
//! 偽陽性になる。
//!
//! **本ファイルは走査語を開き括弧まで含めた形で 1 度も書かない。** 走査の対象そのものなので、
//! 逐語で置くと ⑴ 迂回検知の見張り（要件 8）が自分自身を拾い ⑵ 着手前インベントリの
//! `rg -l` による捕捉サイト計数の母数が動く。以下の doc も同じ約束で書く。
//!
//! # 行数の定義
//!
//! [`line_count`] は**改行の個数**を返す。着手前インベントリ
//!（`.kiro/specs/areka-P0-test-cage-determinism/verification/remeasure.md` §6）が
//! 同じ定義で 1,000 行超 11 件を採っているので、番人の例外表と数え方が一致する。

// 本 module は 3 つの試験対象（較正・迂回検知の見張り・行数の見張り）から共有され、
// どの試験対象も道具箱の一部しか使わない。未使用の警告は共有の副作用なので黙らせる。
#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// 1 ファイルの行数の上限（`structure.md:176` の目安）。
pub const LINE_LIMIT: usize = 1000;

/// 列挙から外すディレクトリ名。生成物と外部取り込み。
const EXCLUDED_DIRS: &[&str] = &["target", "vendors", ".git"];

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
/// ファイル（`<stem>_*.rs`）をすべて含み、生成物（`target/`）と外部取り込み（`vendors/`）は
/// 除く。戻り値はワークスペース根からの相対パスで、区切りは `/`、**昇順・重複無し**。
/// 順序を固定するのは、見張りが落ちたときの出力を再現可能にするため。
pub fn walk_workspace_sources() -> Vec<String> {
    let root = workspace_root();
    let mut found = Vec::new();
    collect_rs_files(&root.join("crates"), &root, &mut found);
    found.sort();
    found.dedup();
    found
}

fn collect_rs_files(dir: &Path, root: &Path, out: &mut Vec<String>) {
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
pub fn read_source(rel_path: &str) -> String {
    let full = workspace_root().join(rel_path);
    let bytes = std::fs::read(&full)
        .unwrap_or_else(|err| panic!("ソースを読めない: {} ({err})", full.display()));
    String::from_utf8_lossy(&bytes).into_owned()
}

/// 列挙した全ソースの行数を測る。
pub fn measure_workspace_sources() -> Vec<FileLines> {
    walk_workspace_sources()
        .into_iter()
        .map(|path| {
            let lines = line_count(&read_source(&path));
            FileLines::new(path, lines)
        })
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
/// **現時点の本番の語集合（`with_default`・`set_global_default`・`set_default` の 3 つに
/// 開き括弧を付けた形）はどの 2 語も重ならない**ので、この 2 規則は今は 1 件も動かしていない。語の一覧を持つのは
/// 見張り側（要件 8）なので、語が増えたときに黙って壊れないよう規則として固定してある。
/// 規則が生きていることは較正テストが縛る（`workspace_scan_test.rs` の
/// `scan_tokens_returns_the_longer_of_two_tokens_that_start_at_the_same_place` と
/// `scan_tokens_does_not_count_one_call_twice_when_a_longer_token_contains_a_shorter_one`）。
pub fn scan_tokens(src: &str, tokens: &[&str]) -> Vec<(usize, String)> {
    let mut order: Vec<&str> = tokens.to_vec();
    order.sort_by_key(|t| std::cmp::Reverse(t.len()));

    let stripped = strip_comments(src);
    let mut hits = Vec::new();
    for (index, line) in stripped.lines().enumerate() {
        let mut prev_is_ident = false;
        let mut skip_until = 0usize;
        for (at, ch) in line.char_indices() {
            if at >= skip_until && !prev_is_ident {
                for token in &order {
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
