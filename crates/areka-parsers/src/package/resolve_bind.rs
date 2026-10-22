//! resolve_bind — shell descript.txt からの着せ替えの読み（bindgroup の転記）。
//!
//! `resolve.rs` から移した（振る舞いは変えない）。`read_bindgroup_defaults` は
//! シェル名つきの解決 `resolve_shell::resolve_with_shell` から呼ばれる。
//! charset 判定・KV 分割は foundation へ委譲し再実装しない（Req 1.5）。

use std::path::Path;

use crate::charset::{DefaultEncoding, decode};
use crate::kv::parse_kv;

use super::model::{BindGroupDefaults, BindGroupName};

/// bindgroup 番号を含むキーが `default,1` のときに番号を取り出す接頭辞集合。
const SAKURA_BINDGROUP_PREFIX: &str = "sakura.bindgroup";
const KERO_BINDGROUP_PREFIX: &str = "kero.bindgroup";
/// bindgroup 番号キーの接尾辞（`sakura.bindgroupNNNN.default` の末尾）。
const BINDGROUP_DEFAULT_SUFFIX: &str = ".default";
/// bindgroup 名前宣言キーの接尾辞（`sakura.bindgroupNNNN.name` の末尾・task 1.2）。
const BINDGROUP_NAME_SUFFIX: &str = ".name";
/// bindoption 宣言キーの接頭辞（`sakura.bindoptionN.group` の先頭・task 10.1）。
const SAKURA_BINDOPTION_PREFIX: &str = "sakura.bindoption";
const KERO_BINDOPTION_PREFIX: &str = "kero.bindoption";
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
///   として本体／相方スコープ別に収集する（従来挙動・無改変・R1.6）。
/// - `<prefix>NNNN.name`: 値 `カテゴリ名,パーツ名,サムネイル名` を `splitn(3, ',')` で
///   再分割・各フィールド trim し、`BindGroupName` として本体／相方スコープ別に転記する。
///   パーツ名（第 2 フィールド）欠落/空の行は `warn!` の上転記対象外（正典上不完全・R1.5）。
///
/// **転記のみ・展開しない**（範囲展開・surface 解決はしない・parsers 転写層原則）。
/// 走査は `parse_kv` の `BTreeMap` 反復順（キー昇順）で決定的。名前は転記順（キー昇順）に
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
    let map = parse_kv(&text);

    let mut defaults = BindGroupDefaults::default();
    for (key, value) in &map {
        // --- .default 経路（従来挙動・値 "1" のみ・無改変・R1.6）---
        if value == "1" {
            if let Some(id) =
                // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#sakura.bindgroup_2a.default_2c_6570_5024:1
                parse_bindgroup_id(key, SAKURA_BINDGROUP_PREFIX, BINDGROUP_DEFAULT_SUFFIX)
            {
                defaults.sakura_default_on.push(id);
                continue;
            } else if let Some(id) =
                parse_bindgroup_id(key, KERO_BINDGROUP_PREFIX, BINDGROUP_DEFAULT_SUFFIX)
            {
                defaults.kero_default_on.push(id);
                continue;
            }
        }
        // --- .name 経路（task 1.2・値はカテゴリ,パーツ[,サムネ] 文字列）---
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#sakura.bindgroup_2a.name_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_30b5_30e0_30cd_30a4_30eb_540d:1
        if let Some(id) = parse_bindgroup_id(key, SAKURA_BINDGROUP_PREFIX, BINDGROUP_NAME_SUFFIX)
            && let Some(name) = parse_bindgroup_name(id, value)
        {
            defaults.sakura_names.push(name);
            continue;
        } else if let Some(id) =
            // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#kero.bindgroup_2a.name_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_30b5_30e0_30cd_30a4_30eb_540d:1
            parse_bindgroup_id(key, KERO_BINDGROUP_PREFIX, BINDGROUP_NAME_SUFFIX)
            && let Some(name) = parse_bindgroup_name(id, value)
        {
            defaults.kero_names.push(name);
            continue;
        }
        // --- bindoption.group 経路（値はカテゴリ,オプション[+オプション...] 文字列）---
        // `sakura/kero.bindoptionN.group,カテゴリ名,オプション` のオプション欄を `+` で分解し、
        // 認識できた語（`mustselect`／`multiple`）の**所属**をスコープ別に忠実転記する。
        // 併記（`mustselect+multiple`）は両方の集合へ push し、併記の情報を落とさない
        // （優先則は下流 seriko の解釈・bindopt D4）。どちらの集合にも入らないカテゴリが
        // 正典の既定（高々 1 個・解除可）を表す——ここで既定を捏造しない（bindopt 1.1）。
        // 「排他か」の判定語彙は parsers に持ち込まない（転写層原則）。オプション
        // インデックス `N`（`parse_bindgroup_id` が検証する）は M1 不使用＝キー形の妥当性
        // 検査にのみ用いる（転記のみ・展開しない）。
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#sakura.bindoption_2a.group_2c_30ab_30c6_30b4_30ea_540d_2c_30aa_30d7_30b7_30e7_30f3:1
        if parse_bindgroup_id(key, SAKURA_BINDOPTION_PREFIX, BINDOPTION_GROUP_SUFFIX).is_some()
            && let Some(decl) = parse_bindoption_options(value)
        {
            if decl.mustselect {
                defaults.sakura_mustselect.push(decl.category.clone());
            }
            if decl.multiple {
                defaults.sakura_multiple.push(decl.category);
            }
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#kero.bindoption_2a.group_2c_30ab_30c6_30b4_30ea_540d_2c_30aa_30d7_30b7_30e7_30f3:1
        } else if parse_bindgroup_id(key, KERO_BINDOPTION_PREFIX, BINDOPTION_GROUP_SUFFIX).is_some()
            && let Some(decl) = parse_bindoption_options(value)
        {
            if decl.mustselect {
                defaults.kero_mustselect.push(decl.category.clone());
            }
            if decl.multiple {
                defaults.kero_multiple.push(decl.category);
            }
        }
    }
    defaults
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
fn parse_bindgroup_name(id: u32, value: &str) -> Option<BindGroupName> {
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
    })
}
