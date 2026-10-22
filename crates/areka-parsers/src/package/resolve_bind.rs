//! resolve_bind — shell descript.txt からの着せ替えの読み（bindgroup の転記）。
//!
//! `resolve.rs` から移した（振る舞いは変えない）。`read_bindgroup_defaults` は
//! シェル名つきの解決 `resolve_shell::resolve_with_shell` から呼ばれる。
//! charset 判定は foundation へ委譲する（Req 1.5）。KV 分割は行の番号を添えるため
//! `parse_kv` と同じ切り方の `parse_kv_with_line` をここだけで持つ。

use std::collections::BTreeMap;
use std::path::Path;

use crate::charset::{DefaultEncoding, decode};

use super::model::{BindGroupDefaults, BindGroupName, BindScope};

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
///   としてスコープ別に収集する（従来挙動・無改変・R1.6）。
/// - `<prefix>NNNN.name`: 値 `カテゴリ名,パーツ名,サムネイル名` を `splitn(3, ',')` で
///   再分割・各フィールド trim し、`BindGroupName`（決めた行の番号つき）としてスコープ別に転記する。
///   パーツ名（第 2 フィールド）欠落/空の行は `warn!` の上転記対象外（正典上不完全・R1.5）。
///
/// **転記のみ・展開しない**（範囲展開・surface 解決はしない・parsers 転写層原則）。
/// スコープは `sakura.`（0）・`kero.`（1）・`char{N}.`（N は 2 以上）の 3 種類。
/// 走査は `parse_kv_with_line` の `BTreeMap` 反復順（キー昇順）で決定的。名前は転記順（キー昇順）に
/// push され、重複 (カテゴリ, パーツ) はアクセサ側の後勝ち（`resolve_name` の逆順走査・D2）
/// と整合する。shell descript が不在・読取不能なら空を返す（致命でない）。
pub(super) fn read_bindgroup_defaults(
    shell_dir: &Path,
    default_encoding: DefaultEncoding,
) -> BindGroupDefaults {
    let descript = shell_dir.join(SHELL_DESCRIPT_FILE);
    // shell descript は存在確定していない。読めなければ空（bindgroup 定義なしと同義）。
    let Ok(bytes) = std::fs::read(&descript) else {
        return BindGroupDefaults::default();
    };

    let text = decode(&bytes, default_encoding);
    let map = parse_kv_with_line(&text);

    let mut defaults = BindGroupDefaults::default();
    for (key, (value, line)) in &map {
        let Some((scope, rest)) = split_scope(key) else {
            continue;
        };
        // --- .default 経路（従来挙動・値 "1" のみ・無改変・R1.6）---
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#sakura.bindgroup_2a.default_2c_6570_5024:1
        if value == "1"
            && let Some(id) = parse_bindgroup_id(rest, BINDGROUP_PREFIX, BINDGROUP_DEFAULT_SUFFIX)
        {
            scope_vec(
                scope,
                &mut defaults.sakura_default_on,
                &mut defaults.kero_default_on,
                &mut defaults.char_default_on,
            )
            .push(id);
            continue;
        }
        // --- .name 経路（task 1.2・値はカテゴリ,パーツ[,サムネ] 文字列）---
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#sakura.bindgroup_2a.name_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_30b5_30e0_30cd_30a4_30eb_540d:1
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#kero.bindgroup_2a.name_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_30b5_30e0_30cd_30a4_30eb_540d:1
        if let Some(id) = parse_bindgroup_id(rest, BINDGROUP_PREFIX, BINDGROUP_NAME_SUFFIX) {
            if let Some(name) = parse_bindgroup_name(id, value, *line) {
                scope_vec(
                    scope,
                    &mut defaults.sakura_names,
                    &mut defaults.kero_names,
                    &mut defaults.char_names,
                )
                .push(name);
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
        // インデックス `N`（`parse_bindgroup_id` が検証する）は M1 不使用＝キー形の妥当性
        // 検査にのみ用いる（転記のみ・展開しない）。
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#sakura.bindoption_2a.group_2c_30ab_30c6_30b4_30ea_540d_2c_30aa_30d7_30b7_30e7_30f3:1
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#kero.bindoption_2a.group_2c_30ab_30c6_30b4_30ea_540d_2c_30aa_30d7_30b7_30e7_30f3:1
        if parse_bindgroup_id(rest, BINDOPTION_PREFIX, BINDOPTION_GROUP_SUFFIX).is_some()
            && let Some(decl) = parse_bindoption_options(value)
        {
            if decl.mustselect {
                scope_vec(
                    scope,
                    &mut defaults.sakura_mustselect,
                    &mut defaults.kero_mustselect,
                    &mut defaults.char_mustselect,
                )
                .push(decl.category.clone());
            }
            if decl.multiple {
                scope_vec(
                    scope,
                    &mut defaults.sakura_multiple,
                    &mut defaults.kero_multiple,
                    &mut defaults.char_multiple,
                )
                .push(decl.category);
            }
        }
    }
    defaults
}

/// キーの頭（`sakura.`・`kero.`・`char{N}.`＝N は 2 以上の数字だけ）からスコープを決め、残りを返す。
/// どれでもないキー（`char0.`・`char1.` を含む）は `None`（読まない）。
fn split_scope(key: &str) -> Option<(BindScope, &str)> {
    if let Some(rest) = key.strip_prefix(SAKURA_SCOPE_PREFIX) {
        return Some((BindScope::Sakura, rest));
    }
    if let Some(rest) = key.strip_prefix(KERO_SCOPE_PREFIX) {
        return Some((BindScope::Kero, rest));
    }
    let (number, rest) = key.strip_prefix(CHAR_SCOPE_PREFIX)?.split_once('.')?;
    if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let n = number.parse::<u32>().ok().filter(|n| *n >= 2)?;
    Some((BindScope::Char(n), rest))
}

/// スコープの欄（`char*` は N ごとの表に作る）を書き込み用に引く。
fn scope_vec<'a, T>(
    scope: BindScope,
    sakura: &'a mut Vec<T>,
    kero: &'a mut Vec<T>,
    chars: &'a mut BTreeMap<u32, Vec<T>>,
) -> &'a mut Vec<T> {
    match scope {
        BindScope::Sakura => sakura,
        BindScope::Kero => kero,
        BindScope::Char(n) => chars.entry(n).or_default(),
    }
}

/// 行の番号つきの key-value の読み（着せ替えの読みの中だけで使う）。
///
/// 切り方は `parse_kv` と同じ（`lines()`・最初の `,` で分ける・キーと値の前後の空白を
/// 取る・キーが空の行とカンマの無い行を飛ばす）で、値に 1 始まりの行の番号を添える。
/// 同じキーは後勝ちで、行の番号も後の行になる。`parse_kv` はほかの読み手すべてに効く
/// ので変えない。
pub(super) fn parse_kv_with_line(text: &str) -> BTreeMap<String, (String, usize)> {
    let mut map = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        let Some((raw_key, raw_value)) = line.split_once(',') else {
            continue;
        };
        let key = raw_key.trim();
        if key.is_empty() {
            continue;
        }
        map.insert(key.to_owned(), (raw_value.trim().to_owned(), index + 1));
    }
    map
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

/// `<prefix>NNNN<suffix>` 形のキーから bindgroup 番号 `NNNN`（u32）を取り出す。
/// 形が一致しない／番号が u32 でパースできないキーは `None`（転記対象外）。
fn parse_bindgroup_id(key: &str, prefix: &str, suffix: &str) -> Option<u32> {
    let rest = key.strip_prefix(prefix)?;
    let number = rest.strip_suffix(suffix)?;
    number.parse::<u32>().ok()
}

/// `.name` の値 `カテゴリ名,パーツ名,サムネイル名` を `BindGroupName` へ忠実転記する。
///
/// `splitn(3, ',')` で最大 3 分割し各フィールドを trim する（第 3 フィールドは残余全部＝
/// サムネイル名として不透明保持）。第 2 フィールド（パーツ名）が欠落／trim 後に空なら
/// **転記対象外**（`warn!` を残し `None` を返す・捏造しない・R1.5）。第 3 フィールドが
/// 欠落／空なら `thumbnail == None`。展開・alias 解決はしない（転写層原則）。
fn parse_bindgroup_name(id: u32, value: &str, line: usize) -> Option<BindGroupName> {
    let mut fields = value.splitn(3, ',');
    let category = fields.next().unwrap_or("").trim();
    let part = fields.next().map(str::trim).unwrap_or("");
    let thumbnail_raw = fields.next().map(str::trim).unwrap_or("");

    // パーツ名なしの宣言は正典上不完全＝転記しない（着せ替え ID を捏造しない・R1.5）。
    if part.is_empty() {
        tracing::warn!(
            bindgroup_id = id,
            value = %value,
            "bindgroup .name 宣言にパーツ名が無い（正典上不完全）ため転記対象外"
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
