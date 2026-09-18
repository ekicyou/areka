//! エントリ名の復号と安全性の判定（要件 2.2・2.3・2.4・2.7・4.1〜4.7）。
//!
//! 生バイトの名前を「名前は UTF-8」の印（汎用目的ビット 11）に従って復号し、
//! 展開先の外へ出ない・Windows で作れる名前だけを通す。通った名前は
//! [`EntryName`] になり、以降の層は生バイトも印も見ない。
//!
//! # 置換文字で黙って通さない（要件 2.4）
//!
//! 復号に損失があれば（UTF-8 として読めない／Shift_JIS の復号器が誤りを報せた）
//! その場で拒否し、理由にエントリ番号と**生バイトの 16 進**を入れる。復号後の
//! 文字列には U+FFFD しか残らないので、後段でいくら調べても元のバイトは復元
//! できない。診断に足る情報を持てるのはここだけ。
//!
//! # 検査の順序（最初に当たった理由で拒否）
//!
//! 復号 → NUL → `\` → 絶対の形 → `..` → 空の要素 → Windows で作れない名前 →
//! シンボリックリンク → 大小の衝突、の順。1 つの名前が複数の違反を持ち得るので、
//! どの理由を返すかは順序で決まる（`..\CON.txt` は `\` で拒否される）。順序は
//! 兄弟テストが複数違反の検体で測っている。
//!
//! 全エントリを検査し終えてから拒否をまとめる形は採らず、**最初の拒否で全体を
//! 拒否**する。どちらでも書き込みの前に止まり（要件 4.1）、製品側は 1 つの理由を
//! `OnInstallFailure` に写すので 1 件で足りる。
//!
//! 本モジュールは宛先を受け取らず、ファイルシステムを変える呼び出しを 1 つも
//! 持たない（兄弟テストが字面で見張る）。

use crate::container::RawEntry;
use crate::error::{RefuseReason, UnsafeWhy};
use std::collections::BTreeMap;

/// Windows のファイル名に使えない文字。
const WINDOWS_FORBIDDEN_CHARS: &[char] = &['<', '>', ':', '"', '|', '?', '*'];

/// 拡張子を除いた語がこれらのいずれかなら、Windows の予約名（ASCII 大小無視）。
///
/// `COM0`／`LPT0` は入らない。`CONSOLE` のように語が長いものも予約ではないので、
/// 前方一致ではなく**完全一致**で見る。
const RESERVED_STEMS: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Unix の外部属性のうち、種別を取り出す覆い。
const S_IFMT: u32 = 0o170000;

/// Unix の外部属性の種別のうち、シンボリックリンク。
const S_IFLNK: u32 = 0o120000;

/// 検証を通ったエントリ名。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EntryName {
    /// 中央ディレクトリに並ぶ順。0 始まり（[`RawEntry::index`] のまま）。
    pub index: usize,
    /// `/` 区切りの正規化済みのパス。フォルダのエントリでも末尾の `/` は持たない。
    pub path: String,
    /// `path` を `/` で分けた要素。空の要素は無い。
    pub components: Vec<String>,
    /// フォルダのエントリ（生の名前が `/` で終わっていた）。
    pub is_dir: bool,
}

/// 生バイトを小文字の 16 進に写す（区切り無し）。拒否の理由に入れる。
fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// `UnsafePath` の拒否を組む。
fn unsafe_path(index: usize, name: &str, why: UnsafeWhy) -> RefuseReason {
    RefuseReason::UnsafePath {
        index,
        name: name.to_owned(),
        why,
    }
}

/// エントリ名を印に従って復号する。損失があれば拒否する（要件 2.2・2.3・2.4）。
pub(crate) fn decode_entry_name(entry: &RawEntry) -> Result<String, RefuseReason> {
    let undecodable = |encoding| RefuseReason::NameUndecodable {
        index: entry.index,
        raw_hex: to_hex(&entry.name_raw),
        encoding,
    };
    if entry.utf8_flag {
        std::str::from_utf8(&entry.name_raw)
            .map(str::to_owned)
            .map_err(|_| undecodable("UTF-8"))
    } else {
        // `had_errors` を見ないと、読めないバイトが U+FFFD に化けたまま成功に見える。
        let (text, had_errors) =
            encoding_rs::SHIFT_JIS.decode_without_bom_handling(&entry.name_raw);
        if had_errors {
            Err(undecodable("Shift_JIS"))
        } else {
            Ok(text.into_owned())
        }
    }
}

/// `component` が Windows のファイルシステムで作れる名前か（要件 4.6）。
///
/// 空の要素はここへ来る前に撥ねてある。
fn is_usable_windows_name(component: &str) -> bool {
    if component
        .chars()
        .any(|ch| WINDOWS_FORBIDDEN_CHARS.contains(&ch) || ch < '\u{20}')
    {
        return false;
    }
    if component.ends_with('.') || component.ends_with(' ') {
        return false;
    }
    // 「拡張子を除いた語」＝最初のドットより前。`CON.txt` も `CON.a.b` も予約。
    let stem = component.split('.').next().unwrap_or(component);
    !RESERVED_STEMS
        .iter()
        .any(|reserved| stem.eq_ignore_ascii_case(reserved))
}

/// `name` が「木を掘らない 1 つの名前」として使えるか（要件 3.9・3.15）。
///
/// `install.txt` の `directory`・`*.directory`・`*.source.directory` と、
/// `refreshundeletemask` の各要素が満たすべき規則。エントリ名と同じ
/// [`is_usable_windows_name`] を土台にして、そこに含まれない 3 つ——空・`/`・`\`
/// ——だけを足す。`..` は末尾がドット、`C:` は禁止文字の `:`、NUL は制御文字と
/// して、いずれも土台の側で撥ねられる（二重に書くと片方だけ直る）。
pub(crate) fn is_valid_one_level_name(name: &str) -> bool {
    !name.is_empty() && !name.contains('/') && !name.contains('\\') && is_usable_windows_name(name)
}

/// 名前が `X:` の形（ドライブレター）で始まるか。
fn starts_with_drive_letter(name: &str) -> bool {
    let mut chars = name.chars();
    matches!((chars.next(), chars.next()), (Some(first), Some(':')) if first.is_ascii_alphabetic())
}

/// 外部属性の上位 16 bit の種別がシンボリックリンクか（要件 2.7）。
///
/// 作成側 OS の種別（`version made by`）は見ない。Unix 以外を名乗る書庫が
/// `S_IFLNK` を持っていたら、それは受け入れてよい理由にならず安全側へ倒す。
fn is_symlink(external_attrs: u32) -> bool {
    (external_attrs >> 16) & S_IFMT == S_IFLNK
}

/// エントリ 1 件を復号し、安全性を判定する。順序は本モジュールの説明のとおり。
fn validate_one(entry: &RawEntry) -> Result<EntryName, RefuseReason> {
    let name = decode_entry_name(entry)?;
    let refuse = |why| unsafe_path(entry.index, &name, why);

    if name.contains('\0') {
        return Err(refuse(UnsafeWhy::Nul));
    }
    if name.contains('\\') {
        return Err(refuse(UnsafeWhy::Backslash));
    }
    // `/` 始まりは `//`（UNC の `/` 綴り）も兼ねる。
    if name.starts_with('/') || starts_with_drive_letter(&name) {
        return Err(refuse(UnsafeWhy::Absolute));
    }

    // フォルダのエントリの末尾の `/` だけを落とす。`a//` は空の要素として残る。
    let trimmed = name.strip_suffix('/').unwrap_or(&name);
    let components: Vec<String> = trimmed.split('/').map(str::to_owned).collect();

    if components.iter().any(|component| component == "..") {
        return Err(refuse(UnsafeWhy::DotDot));
    }
    if components.iter().any(|component| component.is_empty()) {
        // `a//b` と空の名前。絶対パスと同じく「そのままでは宛先を組めない」形。
        return Err(refuse(UnsafeWhy::Absolute));
    }
    if let Some(bad) = components
        .iter()
        .find(|component| !is_usable_windows_name(component))
    {
        return Err(refuse(UnsafeWhy::InvalidWindowsName(bad.clone())));
    }
    if is_symlink(entry.external_attrs) {
        return Err(RefuseReason::SymlinkEntry {
            index: entry.index,
            name,
        });
    }

    Ok(EntryName {
        index: entry.index,
        path: components.join("/"),
        components,
        is_dir: entry.is_dir,
    })
}

/// 全エントリの名前を復号し、安全性と大小の衝突を判定する（要件 4.1）。
///
/// 衝突の判定は復号後の**完全なパス**を Unicode の規則で小文字化して比べる。
/// 同じパスの重複も同じ変種で拒否する（Windows では後勝ちで一方が消えるので、
/// 大小違いと区別する意味が無い）。フォルダのエントリとファイルのエントリは
/// 末尾の `/` を落とした後で比べるので、`a/` と `a` も衝突として撥ねる。
pub(crate) fn validate_entry_names(entries: &[RawEntry]) -> Result<Vec<EntryName>, RefuseReason> {
    let mut names = Vec::with_capacity(entries.len());
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    for entry in entries {
        let name = validate_one(entry)?;
        if let Some(first) = seen.insert(name.path.to_lowercase(), name.path.clone()) {
            return Err(RefuseReason::CaseCollision {
                a: first,
                b: name.path,
            });
        }
        names.push(name);
    }
    Ok(names)
}

#[cfg(test)]
#[path = "names_tests.rs"]
mod tests;
