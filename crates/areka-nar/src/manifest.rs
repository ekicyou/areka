//! 最上位の `install.txt` を見つけ、[`InstallManifest`] へ写す（要件 3.1〜3.16）。
//!
//! 製品側のインストーラが書くのは判断（`accept` の照合・イベント送出）だけで
//! 済むように、`install.txt` の解釈はここ 1 か所に閉じる。
//!
//! # 既存層をそのまま使う
//!
//! 文字コードの判定（`charset` 行の先読みと ANSI 既定）は
//! [`areka_parsers::charset::decode`]、行の分割（最初のカンマ・前後空白の除去・
//! 同一キーは後勝ち・カンマの無い行は無視）は [`areka_parsers::kv::parse_kv`] が
//! 持っている。`descript.txt` 等と共有する層なので、ここからは変えない。
//!
//! 既存層はキーの大小を保つので、引く前に **ASCII 小文字化した写しを作り直し**、
//! 以降はその写しだけを引く（要件 3.4）。同じキーが大小違いで 2 度あれば、
//! 既存層の並び（キーのバイト順）で後に来たほうが勝つ。
//!
//! # 拒否と警告の分かれ目
//!
//! 「本体を置けない」ものだけが拒否（[`RefuseReason`]）になる。種別・`name`・
//! `directory`・読む対象のフォルダ名がこれに当たる。読まないと決めたもの
//! （扱わない同時インストールの種別・ゴースト以外に書かれた同時インストール・
//! 知らないキー・使えないマスクの要素）は、本体の展開を止めずに
//! [`ManifestWarning`] として**必ず 1 件ずつ**記録する。黙って落とす道は無い。
//!
//! # 警告の並び
//!
//! 同じ `install.txt` からは必ず同じ列が出る。⑴ 本体の再インストール →
//! ⑵ キーの名前順の読み飛ばし → ⑶ 同時インストールの名前順の再インストール、
//! の 3 段。1 件のキーからは最大 1 件の警告しか出さない。
//!
//! 本モジュールは宛先を受け取らず、ファイルシステムを変える呼び出しを 1 つも
//! 持たない（兄弟テストが字面で見張る）。

use crate::error::{ManifestWarning, RefuseReason};
use crate::names::{EntryName, is_valid_one_level_name};
use areka_parsers::charset::{DefaultEncoding, decode};
use areka_parsers::kv::parse_kv;
use std::collections::{BTreeMap, BTreeSet};

/// 最上位に置かれているはずのマニフェストの名前（ASCII 大小は区別しない）。
const INSTALL_TXT: &str = "install.txt";

/// 同時インストールとして読む接頭辞。この後ろは数字列だけを許す。
const BALLOON_PREFIX: &str = "balloon";

/// 正典にはあるが areka が扱わない同時インストールの接頭辞（要件 3.13）。
///
/// それぞれ後ろに数字列が付いた形（`headline0` など）も同じ扱い。
const UNSUPPORTED_COMPANION_PREFIXES: &[&str] =
    &["headline", "plugin", "calendar.skin", "calendar.plugin"];

/// 同時インストールのキーの後半（`<接頭辞>.<これ>`）。
///
/// 並びは前から順に試す。`source.directory` は `directory` で先に一致し得るが、
/// そのときの接頭辞（`balloon.source`）はどの種別でもないので次へ進む。
const COMPANION_SUFFIXES: &[&str] = &[
    "directory",
    "source.directory",
    "refresh",
    "refreshundeletemask",
];

/// 単独で意味を持つキー。これでも同時インストールの形でもなければ読み飛ばす。
const KNOWN_KEYS: &[&str] = &[
    "charset",
    "type",
    "name",
    "directory",
    "accept",
    "refresh",
    "refreshundeletemask",
];

/// アーカイブの種別（要件 3.5）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallKind {
    Ghost,
    Shell,
    Supplement,
    Balloon,
}

/// 既に同じ宛先が在るときの扱い（要件 3.15・Requirement 6 へ渡す）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExistingPolicy {
    /// `refresh` が `1` でない: 既存を残し、同名だけ上書きする。
    Overlay,
    /// `refresh,1`: 全消去してから入れ替える。`keep` の名前は全階層で残す。
    Replace { keep: Vec<String> },
}

/// 同時にインストールするバルーン 1 件（要件 3.12）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Companion {
    /// `balloon` / `balloon0` … の接頭辞そのもの。
    pub key: String,
    /// 宛先 `<根>/balloon/<directory>/`。
    pub directory: String,
    /// アーカイブ内の取り出し元フォルダ名。
    pub source_directory: String,
    pub existing: ExistingPolicy,
}

/// `install.txt` の解釈の結果（要件 3.5〜3.16）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallManifest {
    pub kind: InstallKind,
    /// `OnInstallComplete` の Reference1 に使う表示名。
    pub name: String,
    pub directory: String,
    /// `accept` の値。照合はしない（`ghost-install` の責務）。空値は `None`。
    pub accept: Option<String>,
    pub existing: ExistingPolicy,
    /// `kind` が `Ghost`／`Shell` のときだけ非空。接頭辞の名前順に並ぶ。
    pub companions: Vec<Companion>,
    pub warnings: Vec<ManifestWarning>,
    /// ファイルに書かれていた `charset` の値（実際に使われた文字コードではない）。
    pub charset_declared: Option<String>,
}

/// 最上位の `install.txt` を 1 つ選ぶ（要件 3.1・3.2）。
///
/// 最上位＝階層が 1 つのファイルのエントリ。[`EntryName::path`] は区切りを正規化
/// してあるので、**丸ごと `install.txt` に等しい**という一言が「最上位にある」と
/// 「名前が `install.txt` である」の両方を言っている（`emo2/install.txt` は等しく
/// ならない）。階層の数を別に数えても同じ判定にしかならないので、二重には書かない。
/// 名前の ASCII 大小は区別しない（Windows のファイルシステムの意味論に合わせる）。
/// 大小だけが違う二重の名前は [`crate::names::validate_entry_names`] が先に撥ねて
/// いるので、ここで複数当たることはない。
///
/// 見つからなければ、1 段の包みフォルダを黙って剥がさずに拒否し、理由に最上位の
/// 名前の一覧を入れる。
pub(crate) fn locate_install_txt(entries: &[EntryName]) -> Result<&EntryName, RefuseReason> {
    entries
        .iter()
        .find(|entry| !entry.is_dir && entry.path.eq_ignore_ascii_case(INSTALL_TXT))
        .ok_or_else(|| RefuseReason::MissingInstallTxt {
            top_level: top_level_names(entries),
        })
}

/// 最上位に見えている名前を名前順に並べる。フォルダは `名前/` の形。
///
/// フォルダのエントリを 1 つも持たない書庫（実在の配布物 `hello-pasta.nar` が
/// その形）でも包みフォルダの名前が出るよう、深いエントリからは**先頭の要素**を
/// 取る。エントリ名をそのまま並べるだけだと一覧が空になり、利用者に
/// 「何が入っているのか」を返せない。
fn top_level_names(entries: &[EntryName]) -> Vec<String> {
    let mut names = BTreeSet::new();
    for entry in entries {
        if entry.components.len() == 1 && !entry.is_dir {
            names.insert(entry.path.clone());
        } else {
            names.insert(format!("{}/", entry.components[0]));
        }
    }
    names.into_iter().collect()
}

/// `install.txt` の中身を解釈する（要件 3.3〜3.16）。
pub(crate) fn parse_manifest(bytes: &[u8]) -> Result<InstallManifest, RefuseReason> {
    // 文字コードの判定は既存層に任せる。`charset` 行が無ければ ANSI 既定（要件 3.3）。
    let keys = lowercased_keys(&decode(bytes, DefaultEncoding::Ansi));
    let mut warnings = Vec::new();

    // 拒否は「本体を置けない」ものだけ。種別 → `name` → `directory` の順で、
    // 最初に当たった理由で止まる。
    let kind = parse_kind(&keys)?;
    let name = required(&keys, "name")?;
    let directory = required(&keys, "directory")?;
    check_one_level("directory", &directory)?;

    let existing = body_existing(kind, &keys, &mut warnings);
    let companions = collect_companions(kind, &keys, &mut warnings)?;

    Ok(InstallManifest {
        kind,
        name,
        directory,
        accept: non_empty(&keys, "accept"),
        existing,
        companions,
        warnings,
        charset_declared: non_empty(&keys, "charset"),
    })
}

/// 既存層の結果をキーだけ ASCII 小文字化して写し直す（要件 3.4）。
fn lowercased_keys(text: &str) -> BTreeMap<String, String> {
    parse_kv(text)
        .into_iter()
        .map(|(key, value)| (key.to_ascii_lowercase(), value))
        .collect()
}

/// 空でない値だけを取り出す。空の行は行が無いのと同じ扱い。
fn non_empty(keys: &BTreeMap<String, String>, key: &str) -> Option<String> {
    keys.get(key).filter(|value| !value.is_empty()).cloned()
}

/// `type` を [`InstallKind`] にする（要件 3.5・3.6）。
///
/// 大小を区別しないのはキーだけで、**値はそのまま比べる**（要件 3.4）。
/// 値が空の行は行が無いのと同じ「指定なし」として理由に載せる。
fn parse_kind(keys: &BTreeMap<String, String>) -> Result<InstallKind, RefuseReason> {
    match non_empty(keys, "type").as_deref() {
        Some("ghost") => Ok(InstallKind::Ghost),
        Some("shell") => Ok(InstallKind::Shell),
        Some("supplement") => Ok(InstallKind::Supplement),
        Some("balloon") => Ok(InstallKind::Balloon),
        // `calendar` は `calendar skin` の旧綴り。どちらも扱わない。
        other => Err(RefuseReason::UnsupportedType {
            found: other.map(str::to_owned),
        }),
    }
}

/// 省略できないキーを取り出す。無い・空はどちらも拒否（要件 3.7・3.8）。
fn required(keys: &BTreeMap<String, String>, key: &'static str) -> Result<String, RefuseReason> {
    non_empty(keys, key).ok_or(RefuseReason::MissingRequiredKey { key })
}

/// フォルダ名が 1 階層の名前であることを確かめる（要件 3.9）。
fn check_one_level(key: &str, value: &str) -> Result<(), RefuseReason> {
    if is_valid_one_level_name(value) {
        Ok(())
    } else {
        Err(RefuseReason::InvalidDirectoryName {
            key: key.to_owned(),
            value: value.to_owned(),
        })
    }
}

/// 本体の再インストールの扱いを決める（要件 3.15）。
///
/// `supplement` だけは `refresh` を読まない。重ね置き先がゴースト本体なので、
/// 全消去は本体ごと消しかねない。正典はこの組み合わせに沈黙しているので、
/// 安全側（重ね置き）へ倒す——**本設計の決定**であり、正典に反する実装ではない。
/// 読まなかったことは、`refresh` とマスクのどちらが書かれていても記録に残す。
fn body_existing(
    kind: InstallKind,
    keys: &BTreeMap<String, String>,
    warnings: &mut Vec<ManifestWarning>,
) -> ExistingPolicy {
    if kind == InstallKind::Supplement {
        if keys.contains_key("refresh") || keys.contains_key("refreshundeletemask") {
            warnings.push(ManifestWarning::RefreshIgnoredForSupplement);
        }
        return ExistingPolicy::Overlay;
    }
    existing_policy(keys, "", warnings)
}

/// `<prefix>refresh` と `<prefix>refreshundeletemask` から扱いを決める（要件 3.15）。
///
/// `prefix` は本体なら空、同時インストールなら `balloon0.` のような接頭辞。
/// 値が `1` のときだけ全消去にする。`1` でなければマスクは読まない——使わない値の
/// 不備で警告を出しても、利用者の作業は変わらない。
fn existing_policy(
    keys: &BTreeMap<String, String>,
    prefix: &str,
    warnings: &mut Vec<ManifestWarning>,
) -> ExistingPolicy {
    if keys.get(&format!("{prefix}refresh")).map(String::as_str) != Some("1") {
        return ExistingPolicy::Overlay;
    }
    let mask_key = format!("{prefix}refreshundeletemask");
    let keep = keys
        .get(&mask_key)
        .map_or_else(Vec::new, |mask| parse_mask(&mask_key, mask, warnings));
    ExistingPolicy::Replace { keep }
}

/// 除外マスクをコロンで分ける（要件 3.15）。
///
/// 前後の空白を落とし、空の要素は捨て、1 階層のファイル名でない要素は
/// 記録して読み飛ばす（マスクはパス指定ができない）。
fn parse_mask(key: &str, value: &str, warnings: &mut Vec<ManifestWarning>) -> Vec<String> {
    let mut keep = Vec::new();
    for element in value.split(':') {
        let element = element.trim();
        if element.is_empty() {
            continue;
        }
        if is_valid_one_level_name(element) {
            keep.push(element.to_owned());
        } else {
            warnings.push(ManifestWarning::InvalidMaskEntry {
                key: key.to_owned(),
                value: element.to_owned(),
            });
        }
    }
    keep
}

/// 1 つのキーが何であるか。
enum KeyRole<'a> {
    /// 単独で意味を持つキー。ここでは何もしない（読むのは各担当）。
    Known,
    /// 同時インストールの形。`supported` が偽なら areka が扱わない種別。
    Companion {
        prefix: &'a str,
        suffix: &'a str,
        supported: bool,
    },
    /// 知らないキー。
    Unknown,
}

/// キーを 3 つに分ける。
fn classify(key: &str) -> KeyRole<'_> {
    if KNOWN_KEYS.contains(&key) {
        return KeyRole::Known;
    }
    for suffix in COMPANION_SUFFIXES {
        // `balloon.source.directory` は `directory` でも一致するが、そのときの
        // 接頭辞 `balloon.source` はどの種別でもないので次の綴りへ進む。
        let Some(prefix) = key
            .strip_suffix(suffix)
            .and_then(|head| head.strip_suffix('.'))
        else {
            continue;
        };
        if numbered(prefix, BALLOON_PREFIX) {
            return KeyRole::Companion {
                prefix,
                suffix,
                supported: true,
            };
        }
        if UNSUPPORTED_COMPANION_PREFIXES
            .iter()
            .any(|kind| numbered(prefix, kind))
        {
            return KeyRole::Companion {
                prefix,
                suffix,
                supported: false,
            };
        }
    }
    KeyRole::Unknown
}

/// `prefix` が `base` か、`base` の直後に数字列だけが続く形か。
///
/// `balloon`・`balloon0`・`balloon10` は真、`balloons`・`balloon-1` は偽。
fn numbered(prefix: &str, base: &str) -> bool {
    prefix
        .strip_prefix(base)
        .is_some_and(|rest| rest.chars().all(|ch| ch.is_ascii_digit()))
}

/// 同時インストールのバルーンを組み、読み飛ばした指定を記録する（要件 3.12〜3.14・3.16）。
fn collect_companions(
    kind: InstallKind,
    keys: &BTreeMap<String, String>,
    warnings: &mut Vec<ManifestWarning>,
) -> Result<Vec<Companion>, RefuseReason> {
    let handles_companions = matches!(kind, InstallKind::Ghost | InstallKind::Shell);

    // 先に宛先（`<接頭辞>.directory`）だけを集める。宛先の決まっていない接頭辞の
    // 断片（`balloon.source.directory` だけ、など）は、どこへ入れるのか決まらない。
    let mut directories: BTreeMap<String, String> = BTreeMap::new();
    if handles_companions {
        for (key, value) in keys {
            if let KeyRole::Companion {
                prefix,
                suffix: "directory",
                supported: true,
            } = classify(key)
            {
                directories.insert(prefix.to_owned(), value.clone());
            }
        }
    }

    // 読み飛ばしの記録。キーの名前順に 1 件のキーへ 1 件だけ出す。
    for key in keys.keys() {
        match classify(key) {
            KeyRole::Known => {}
            KeyRole::Unknown => warnings.push(ManifestWarning::IgnoredKey { key: key.clone() }),
            KeyRole::Companion {
                prefix, supported, ..
            } => {
                if !handles_companions {
                    // 種別の判定が先に効く。扱わない種別でも、ここでは
                    // 「ゴースト以外に書かれている」として 1 件だけ記録する。
                    warnings.push(ManifestWarning::CompanionOnNonGhost { key: key.clone() });
                } else if !supported {
                    warnings.push(ManifestWarning::UnsupportedCompanionKind { key: key.clone() });
                } else if !directories.contains_key(prefix) {
                    warnings.push(ManifestWarning::IgnoredKey { key: key.clone() });
                }
            }
        }
    }

    let mut companions = Vec::with_capacity(directories.len());
    for (prefix, directory) in &directories {
        check_one_level(&format!("{prefix}.directory"), directory)?;
        let source_key = format!("{prefix}.source.directory");
        // `*.source.directory` が無い（または空）なら宛先と同じ名前を取り出し元にする。
        let source_directory = non_empty(keys, &source_key).unwrap_or_else(|| directory.clone());
        check_one_level(&source_key, &source_directory)?;
        companions.push(Companion {
            key: prefix.clone(),
            directory: directory.clone(),
            source_directory,
            existing: existing_policy(keys, &format!("{prefix}."), warnings),
        });
    }
    Ok(companions)
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod tests;
