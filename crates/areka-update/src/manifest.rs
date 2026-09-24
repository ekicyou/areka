//! `updates2.dau`／`updates.txt` の読み手（純関数・要件 1.5〜1.10）。

use crate::error::{InvalidWhy, UpdateWarning};
use crate::outcome::ManifestName;
use crate::paths::{is_in_work_area, normalize_separators, unsafe_component};
use crate::urlpath;
use std::collections::HashMap;

/// 既定の文字コード（1.9）。
pub(crate) const DEFAULT_CHARSET: &encoding_rs::Encoding = encoding_rs::SHIFT_JIS;

pub(crate) struct Manifest {
    pub name: ManifestName,
    /// 解決した文字コード（`delete.txt` へ引き継ぐ＝6.2）。
    pub charset: &'static encoding_rs::Encoding,
    /// 有効なエントリ（定義の順・重複は後勝ち）。
    pub entries: Vec<Entry>,
}

pub(crate) struct Entry {
    pub line: usize,
    /// ローカルパス。`/` 区切り・復号済み（1.15）。
    pub local: String,
    /// URL のパス部分。符号化済み・`/` は区切りのまま（1.15）。
    pub url_path: String,
    /// 小文字 32 桁（2.2）。
    pub md5: String,
}

/// 定義ファイルのバイト列をエントリ列にする。行番号は 1 始まりの物理行。
pub(crate) fn parse(name: ManifestName, bytes: &[u8]) -> (Manifest, Vec<UpdateWarning>) {
    let mut warnings = Vec::new();
    let charset = resolve_charset(prescan_charset(name, bytes), &mut warnings);
    // BOM があれば BOM を優先（encoding_rs の規則）。実際に使った方を delete.txt へ引き継ぐ。
    let (text, used, _) = charset.decode(bytes);

    // (行番号, `/` に揃えたパス, 小文字の MD5)。
    let mut valid = Vec::new();
    for (i, line) in text.split('\n').enumerate() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        // 0 文字の行は数えない。中身の無い `file,` 行は数える（黙って捨てない＝NoMd5）。
        if line.is_empty() {
            continue;
        }
        let body = match name {
            ManifestName::Updates2Dau => line,
            // `charset,` 行は先読みで済んでいる。どちらでもない行は無視（1.6）。
            ManifestName::UpdatesTxt => match line.strip_prefix("file,") {
                Some(rest) => rest,
                None => continue,
            },
        };
        // 位置 0＝パス・位置 1＝MD5・位置 2 以降の拡張欄は読み飛ばす（1.7）。
        let mut fields = body.split('\x01');
        let path = normalize_separators(fields.next().unwrap_or_default());
        let md5 = fields.next().unwrap_or_default();
        let why = if md5.is_empty() {
            Some(InvalidWhy::NoMd5)
        } else if md5.len() != 32 || !md5.bytes().all(|c| c.is_ascii_hexdigit()) {
            Some(InvalidWhy::BadMd5)
        } else {
            invalid_path(&path, name)
        };
        match why {
            Some(why) => warnings.push(UpdateWarning::InvalidEntry { line: i + 1, why }),
            None => valid.push((i + 1, path, md5.to_ascii_lowercase())),
        }
    }

    // 符号化の判定は有効な全エントリで 1 回（1.15）。
    let all_encoded = valid.iter().all(|(_, path, _)| urlpath::is_encoded(path));
    let mut slots: Vec<Option<Entry>> = Vec::with_capacity(valid.len());
    let mut seen: HashMap<String, usize> = HashMap::new();
    for (line, path, md5) in valid {
        let (local, url_path) = if all_encoded {
            let bytes = urlpath::decode(&path);
            let local = match String::from_utf8(bytes) {
                Ok(s) => s,
                Err(e) => used
                    .decode_without_bom_handling(e.as_bytes())
                    .0
                    .into_owned(),
            };
            // `%2E%2E`・`%5C`・`%00` は復号して初めて見える。復号後も同じ検査を通す。
            let local = normalize_separators(&local);
            if let Some(why) = invalid_path(&local, name) {
                warnings.push(UpdateWarning::InvalidEntry { line, why });
                continue;
            }
            (local, path)
        } else {
            let url_path = urlpath::encode(&path);
            (path, url_path)
        };
        // 重複は小文字の鍵で後勝ち（1.14）。
        if let Some(old) = seen.insert(local.to_lowercase(), slots.len())
            && let Some(old) = slots[old].take()
        {
            warnings.push(UpdateWarning::DuplicateEntry {
                line: old.line,
                path: old.local,
            });
        }
        slots.push(Some(Entry {
            line,
            local,
            url_path,
            md5,
        }));
    }

    let manifest = Manifest {
        name,
        charset: used,
        entries: slots.into_iter().flatten().collect(),
    };
    (manifest, warnings)
}

/// パスの無効（1.11・1.12）。`\` は `/` に揃え済み。MD5 の 2 種の後を、定めた順に検査する。
fn invalid_path(path: &str, name: ManifestName) -> Option<InvalidWhy> {
    let why = if path.contains('\0') {
        InvalidWhy::Nul
    } else if path.starts_with('/') || path.contains(':') {
        // ドライブ文字（`C:`）に加え、それ以外の `:` も NTFS のストリーム指定なので同じ扱い。
        InvalidWhy::Absolute
    } else if path.ends_with('/') {
        InvalidWhy::FolderEntry
    } else if unsafe_component(path) {
        // 空・`.`・末尾の `.`／空白（Win32 が落とす）。`:` は上で済み、`..` は下の `DotDot`。
        InvalidWhy::EmptyComponent
    } else if path.split('/').any(|c| c == "..") {
        InvalidWhy::DotDot
    } else if path.eq_ignore_ascii_case(name.file_name()) {
        InvalidWhy::SelfReference
    } else if is_in_work_area(path) {
        InvalidWhy::InsideWorkArea
    } else {
        return None;
    };
    Some(why)
}

/// 文字コードの指定を生のバイト列から先読みする（1.8）。指定が無ければ `None`。
///
/// 区切り（`\n`・`\x01`）は Shift_JIS の 2 バイト目に現れないので、復号前に分けてよい。
fn prescan_charset(name: ManifestName, bytes: &[u8]) -> Option<&[u8]> {
    let mut lines = bytes
        .split(|&b| b == b'\n')
        .map(|l| l.strip_suffix(b"\r").unwrap_or(l));
    match name {
        // 先頭エントリの最後の欄（位置 2 以降）だけ。
        ManifestName::Updates2Dau => {
            let fields: Vec<&[u8]> = lines.next()?.split(|&b| b == 1).collect();
            if fields.len() < 3 {
                return None;
            }
            fields.last()?.strip_prefix(b"charset=")
        }
        ManifestName::UpdatesTxt => lines.find_map(|l| l.strip_prefix(b"charset,")),
    }
}

/// 名前を解決する。無指定は既定、解決できなければ警告して既定（1.9・1.10）。OS のロケールは読まない。
fn resolve_charset(
    label: Option<&[u8]>,
    warnings: &mut Vec<UpdateWarning>,
) -> &'static encoding_rs::Encoding {
    let Some(label) = label else {
        return DEFAULT_CHARSET;
    };
    encoding_rs::Encoding::for_label(label).unwrap_or_else(|| {
        warnings.push(UpdateWarning::UnknownCharset {
            name: String::from_utf8_lossy(label).into_owned(),
        });
        DEFAULT_CHARSET
    })
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod tests;
