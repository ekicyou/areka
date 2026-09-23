//! `updates2.dau`／`updates.txt` の読み手（純関数・要件 1.5〜1.10）。
// ponytail: 本番の呼び手（run）が付くまでの間だけ。試験でも `url_path` を読むのは
// 符号化の判定（2.5）からなので無条件にしている。run から呼んだら外す。
#![allow(dead_code)]

use crate::error::UpdateWarning;
use crate::outcome::ManifestName;

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

    let mut entries = Vec::new();
    for (i, line) in text.split('\n').enumerate() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let body = match name {
            ManifestName::Updates2Dau => line,
            // `charset,` 行は先読みで済んでいる。どちらでもない行は無視（1.6）。
            ManifestName::UpdatesTxt => match line.strip_prefix("file,") {
                Some(rest) => rest,
                None => continue,
            },
        };
        if body.is_empty() {
            continue;
        }
        // 位置 0＝パス・位置 1＝MD5・位置 2 以降の拡張欄は読み飛ばす（1.7）。
        let mut fields = body.split('\x01');
        let path = fields.next().unwrap_or_default();
        let md5 = fields.next().unwrap_or_default();
        // 無効の検査・符号化の判定・MD5 の小文字化・重複の後勝ちはタスク 2.5 でここに入る。
        entries.push(Entry {
            line: i + 1,
            local: path.to_owned(),
            url_path: path.to_owned(),
            md5: md5.to_owned(),
        });
    }

    let manifest = Manifest {
        name,
        charset: used,
        entries,
    };
    (manifest, warnings)
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
