//! resolve_bind — shell descript.txt からの着せ替えの読み（bindgroup の転記）。
//!
//! `read_bindgroup_defaults` は 2 点マウントの解決 `resolve_shell::resolve_with_shell`
//! から、選んだシェルのディレクトリを渡されて呼ばれる。
//! charset 判定は foundation へ委譲する（Req 1.5）。KV 分割は行の番号を添えるため
//! `kv` の `parse_kv_with_line` を使う。

use std::path::Path;

use crate::charset::{DefaultEncoding, decode};
use crate::kv::parse_kv_with_line;

use super::model::{BindGroupDefaults, BindGroupName};

/// スコープを決めるキーの頭（`char` は後ろに 2 以上の数と `.` が続く）。
const SAKURA_SCOPE_PREFIX: &str = "sakura.";
const KERO_SCOPE_PREFIX: &str = "kero.";
const CHAR_SCOPE_PREFIX: &str = "char";
/// スコープの頭を除いた bindgroup キーの接頭辞（`bindgroupNNNN.default` の先頭）。
const BINDGROUP_PREFIX: &str = "bindgroup";
/// bindgroup 番号キーの接尾辞（`sakura.bindgroupNNNN.default` の末尾）。
const BINDGROUP_DEFAULT_SUFFIX: &str = ".default";
/// bindgroup 名前宣言キーの接尾辞（`sakura.bindgroupNNNN.name` の末尾・task 1.2）。
const BINDGROUP_NAME_SUFFIX: &str = ".name";
/// スコープの頭を除いた bindoption 宣言キーの接頭辞（`bindoptionN.group` の先頭・task 10.1）。
const BINDOPTION_PREFIX: &str = "bindoption";
/// bindoption 宣言キーの接尾辞（`sakura.bindoptionN.group` の末尾・task 10.1）。
const BINDOPTION_GROUP_SUFFIX: &str = ".group";
/// ちょうど 1 個（解除不可）を宣言するオプション語（オプション欄の 1 語・bindopt 1.2）。
const MUSTSELECT_OPTION: &str = "mustselect";
/// 複数可を宣言するオプション語（オプション欄の 1 語・bindopt 1.1）。
///
/// ukadoc 正典の 3 値は `mustselect`（ちょうど 1 個・解除不可）／非宣言（既定＝高々 1 個・
/// 解除可）／`multiple`（複数可）。旧実装は `multiple` を破棄していたため下流に「明示
/// multiple」と「非宣言」を区別する情報が無かった。
const MULTIPLE_OPTION: &str = "multiple";
/// shell 定義ファイル名（ghost/master と同名だが所在が異なる）。
const SHELL_DESCRIPT_FILE: &str = "descript.txt";

/// 解決済み shell ディレクトリの descript.txt から bindgroup の default と name を転記する。
///
/// 同一走査で 2 種のサフィックスを転記する（task 1.2）:
/// - `<prefix>NNNN.default`: **値が `1`**（= 起動時オン）のものについて `NNNN` を u32
///   としてスコープ別に収集する（R1.6）。`0`／`1` でない値は off のまま鍵・行・値つきで `warn!`（指摘 14）。
/// - `<prefix>NNNN.name`: 値 `カテゴリ名,パーツ名,サムネイル名` を `splitn(3, ',')` で
///   再分割・各フィールド trim し、`BindGroupName`（決めた行の番号つき）としてスコープ別に転記する。
///   カテゴリ名かパーツ名の欠落/空の行は `warn!` の上転記対象外（正典上不完全・R1.5）。
/// - 番号 `NNNN` は数字だけ・先頭の 0 は `0` のときだけ。外れた鍵は `warn!` して読まない（指摘 47）。
///
/// **転記のみ・展開しない**（範囲展開・surface 解決はしない・parsers 転写層原則）。
/// スコープは `sakura.`（0）・`kero.`（1）・`char{N}.`（N は 2 以上）の 3 種類。
/// 走査は `parse_kv_with_line` の `BTreeMap` 反復順（キー昇順）で決定的。名前は転記順（キー昇順）に
/// push され、重複 (カテゴリ, パーツ) は下流 seriko が後勝ちで引く（D2）。shell descript が不在・読取不能なら空を返す（致命でない・不在のほかは `warn!`）。
pub(super) fn read_bindgroup_defaults(
    shell_dir: &Path,
    default_encoding: DefaultEncoding,
) -> BindGroupDefaults {
    let descript = shell_dir.join(SHELL_DESCRIPT_FILE);
    // shell descript は存在確定していない。読めなければ空（bindgroup 定義なしと同義）。
    // 無いだけなら黙って空・それ以外の失敗（共有違反・権限など）は記録して空（指摘 9）。
    let bytes = match std::fs::read(&descript) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return BindGroupDefaults::default();
        }
        Err(e) => {
            tracing::warn!(
                path = %descript.display(),
                error = %e,
                "シェルの descript.txt が読めないため着せ替えの定義を読まない"
            );
            return BindGroupDefaults::default();
        }
    };

    let text = decode(&bytes, default_encoding);
    let map = parse_kv_with_line(&text);

    let mut defaults = BindGroupDefaults::default();
    for (key, (value, line)) in &map {
        let Some((scope, rest)) = split_scope(key) else {
            continue;
        };
        // --- .default 経路（値 "1" のみ on・"0" は off・ほかは off のまま記録する＝指摘 14）---
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#sakura.bindgroup_2a.default_2c_6570_5024:1
        if let Some(id) = parse_bindgroup_id(rest, BINDGROUP_PREFIX, BINDGROUP_DEFAULT_SUFFIX) {
            let Some(id) = checked_id(id, key, *line) else {
                continue;
            };
            match value.as_str() {
                "1" => defaults.entry(scope).or_default().default_on.push(id),
                "0" => {}
                _ => tracing::warn!(
                    key = %key,
                    line = *line,
                    value = %value,
                    "bindgroup .default の値が 0/1 でないため off として読む"
                ),
            }
            continue;
        }
        // --- .name 経路（task 1.2・値はカテゴリ,パーツ[,サムネ] 文字列）---
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#sakura.bindgroup_2a.name_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_30b5_30e0_30cd_30a4_30eb_540d:1
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#kero.bindgroup_2a.name_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_30b5_30e0_30cd_30a4_30eb_540d:1
        if let Some(id) = parse_bindgroup_id(rest, BINDGROUP_PREFIX, BINDGROUP_NAME_SUFFIX) {
            if let Some(id) = checked_id(id, key, *line)
                && let Some(name) = parse_bindgroup_name(id, key, value, *line)
            {
                defaults.entry(scope).or_default().names.push(name);
            }
            continue;
        }
        // --- bindoption.group 経路（値はカテゴリ,オプション[+オプション...] 文字列）---
        // `<scope>.bindoptionN.group,カテゴリ名,オプション` のオプション欄を `+` で分解し、
        // 認識できた語（`mustselect`／`multiple`）の**所属**をスコープ別に忠実転記する。
        // 併記（`mustselect+multiple`）は両方の集合へ push し、併記の情報を落とさない
        // （優先則は下流 seriko の解釈・bindopt D4）。どちらの集合にも入らないカテゴリが
        // 正典の既定（高々 1 個・解除可）を表す——ここで既定を捏造しない（bindopt 1.1）。
        // 「排他か」の判定語彙は parsers に持ち込まない（転写層原則）。オプション
        // インデックス `N`（`checked_id` が検証する）は M1 不使用＝キー形の妥当性
        // 検査にのみ用いる（転記のみ・展開しない）。
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#sakura.bindoption_2a.group_2c_30ab_30c6_30b4_30ea_540d_2c_30aa_30d7_30b7_30e7_30f3:1
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#kero.bindoption_2a.group_2c_30ab_30c6_30b4_30ea_540d_2c_30aa_30d7_30b7_30e7_30f3:1
        if let Some(index) = parse_bindgroup_id(rest, BINDOPTION_PREFIX, BINDOPTION_GROUP_SUFFIX) {
            if checked_id(index, key, *line).is_none() {
                continue;
            }
            // 崩れた宣言（カテゴリ空・認識語ゼロ）は寛容に読み流すが記録は残す（指摘 14）。
            let Some(decl) = parse_bindoption_options(value) else {
                tracing::warn!(
                    key = %key,
                    line = *line,
                    value = %value,
                    "bindoption .group の宣言が読めないため読み流す"
                );
                continue;
            };
            let binds = defaults.entry(scope).or_default();
            if decl.mustselect {
                binds.mustselect.push(decl.category.clone());
            }
            if decl.multiple {
                binds.multiple.push(decl.category);
            }
        }
    }
    defaults
}

/// キーの頭（`sakura.`＝0・`kero.`＝1・`char{N}.`＝N は 2 以上の数字だけ）からスコープ番号を決め、
/// 残りを返す。どれでもないキー（`char0.`・`char1.` を含む）は `None`（読まない）。
fn split_scope(key: &str) -> Option<(u32, &str)> {
    if let Some(rest) = key.strip_prefix(SAKURA_SCOPE_PREFIX) {
        return Some((0, rest));
    }
    if let Some(rest) = key.strip_prefix(KERO_SCOPE_PREFIX) {
        return Some((1, rest));
    }
    let (number, rest) = key.strip_prefix(CHAR_SCOPE_PREFIX)?.split_once('.')?;
    let n = decimal_u32(number).filter(|n| *n >= 2)?;
    Some((n, rest))
}

/// `bindoption*.group` の値 1 行を分解した宣言（カテゴリ名と、認識できたオプション語の所属）。
///
/// 「排他か」の**解釈**は持たない——認識できた語の所属を写すだけの転写値（転写層原則）。
struct BindOptionDecl {
    /// 第 1 フィールド（カテゴリ名・trim 済み・非空）。
    category: String,
    /// オプション欄に `mustselect` が現れたか（bindopt 1.2）。
    mustselect: bool,
    /// オプション欄に `multiple` が現れたか（bindopt 1.1）。
    multiple: bool,
}

/// `bindoption*.group` の値 `カテゴリ名,オプション[+オプション...]` を分解し、認識できた
/// オプション語の所属を返す（ukadoc 正典「オプションは `+` 区切りで複数指定可」・bindopt 1.3）。
///
/// `splitn(2, ',')` でカテゴリ名とオプション欄に分け、オプション欄を `'+'` で split して各語
/// を trim し、`mustselect`／`multiple` との完全一致で認識する。認識できない語は捏造せず
/// 読み流す（寛容パース・bindopt 1.4）。次のいずれかなら `None`＝収録対象外（bindopt 1.5）:
/// カテゴリ名が空／オプション欄が欠落（`,` なし）／認識語ゼロ（空欄・未知語のみ）。
/// どちらの集合にも入らないカテゴリは正典の既定（高々 1 個・解除可）として下流が扱うため、
/// ここで既定を捏造しない。
///
/// 純関数（副作用なし・同一入力同一出力・bindopt 1.7）。
fn parse_bindoption_options(value: &str) -> Option<BindOptionDecl> {
    let mut fields = value.splitn(2, ',');
    let category = fields.next().unwrap_or("").trim();
    // オプション欄欠落（`,` なし）は空欄と同じく認識語ゼロへ落ちる。
    let options = fields.next().unwrap_or("");
    if category.is_empty() {
        return None;
    }

    let mut mustselect = false;
    let mut multiple = false;
    for option in options.split('+') {
        match option.trim() {
            MUSTSELECT_OPTION => mustselect = true,
            MULTIPLE_OPTION => multiple = true,
            // 未知オプション語は読み流す（捏造しない・bindopt 1.4）。
            _ => {}
        }
    }

    if mustselect || multiple {
        Some(BindOptionDecl {
            category: category.to_owned(),
            mustselect,
            multiple,
        })
    } else {
        None
    }
}

/// `<prefix>NNNN<suffix>` 形のキーから番号の欄 `NNNN`（数の形は確かめない）を取り出す。
/// 形が一致しないキーは `None`（転記対象外）。
fn parse_bindgroup_id<'a>(key: &'a str, prefix: &str, suffix: &str) -> Option<&'a str> {
    key.strip_prefix(prefix)?.strip_suffix(suffix)
}

/// 番号の欄を [`decimal_u32`] で読み、外れたら鍵と行つきで `warn!` して `None`（指摘 47）。
fn checked_id(number: &str, key: &str, line: usize) -> Option<u32> {
    let id = decimal_u32(number);
    if id.is_none() {
        tracing::warn!(
            key = %key,
            line,
            "bindgroup・bindoption の番号が数字だけの形でないため読まない"
        );
    }
    id
}

/// 数字だけで、先頭の 0 は `0` そのもののときだけの u32（`+10`・`010`・空は `None`）。
fn decimal_u32(number: &str) -> Option<u32> {
    let digits_only = !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit());
    if !digits_only || (number.len() > 1 && number.starts_with('0')) {
        return None;
    }
    number.parse().ok()
}

/// `.name` の値 `カテゴリ名,パーツ名,サムネイル名` を `BindGroupName` へ忠実転記する。
///
/// `splitn(3, ',')` で最大 3 分割し各フィールドを trim する（第 3 フィールドは残余全部＝
/// サムネイル名として不透明保持）。第 1 フィールド（カテゴリ名）か第 2 フィールド（パーツ名）が
/// 欠落／trim 後に空なら **転記対象外**（鍵と行つきの `warn!` を残し `None` を返す・
/// 捏造しない・R1.5・指摘 48・49）。第 3 フィールドが
/// 欠落／空なら `thumbnail == None`。展開・alias 解決はしない（転写層原則）。
fn parse_bindgroup_name(id: u32, key: &str, value: &str, line: usize) -> Option<BindGroupName> {
    let mut fields = value.splitn(3, ',');
    let category = fields.next().unwrap_or("").trim();
    let part = fields.next().map(str::trim).unwrap_or("");
    let thumbnail_raw = fields.next().map(str::trim).unwrap_or("");

    // カテゴリ名かパーツ名の無い宣言は正典上不完全＝転記しない（着せ替え ID を捏造しない・R1.5）。
    if category.is_empty() || part.is_empty() {
        tracing::warn!(
            key = %key,
            line,
            value = %value,
            "bindgroup .name 宣言にカテゴリ名かパーツ名が無い（正典上不完全）ため転記対象外"
        );
        return None;
    }

    let thumbnail = if thumbnail_raw.is_empty() {
        None
    } else {
        Some(thumbnail_raw.to_owned())
    };

    Some(BindGroupName {
        id,
        category: category.to_owned(),
        part: part.to_owned(),
        thumbnail,
        line,
    })
}
