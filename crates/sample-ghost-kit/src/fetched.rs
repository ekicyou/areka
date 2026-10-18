//! URL の行の書庫の置き場と照合（spec: `areka-P0-sample-url-fetch` 要件 1.6・3.1・3.3〜3.6・
//! 3.9・4.1・4.2）。
//!
//! 置き場の棚の名前 `fetched` を持つ唯一の所。窓口はこの棚に書かず（書くのは取り寄せの段と
//! 開発者だけ）、`devroot` の掃除（`work\`）と回収（`cache\`）もこの棚を走査しない。通信の
//! 部品も子のプロセスも持たない（要件 3.3）。

use std::path::{Path, PathBuf};

use crate::SampleError;
use crate::sha256::sha256_hex;

/// 名前空間の下の置き場の棚の名前。
const FETCHED: &str = "fetched";

/// ハッシュのファイルの後ろの綴り。公開の `<url>.sha256` と、置き場の `<name>.nar.sha256` の
/// 両方に使う唯一の所（実装の段の裁定 2）。
pub(crate) const CHECKSUM_SUFFIX: &str = ".sha256";

/// `<namespace>\fetched\<name>.nar`。
pub(crate) fn fetched_path_in(namespace: &Path, name: &str) -> PathBuf {
    namespace.join(FETCHED).join(format!("{name}.nar"))
}

/// `<namespace>\fetched\<name>.nar.sha256`。
pub(crate) fn fetched_checksum_path_in(namespace: &Path, name: &str) -> PathBuf {
    namespace
        .join(FETCHED)
        .join(format!("{name}.nar{CHECKSUM_SUFFIX}"))
}

/// `<url>.sha256`（`--fetch-list` の 3 列目）。
// `--fetch-list` が使うのはタスク 5。それまでテストの外に呼び手が無い。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn checksum_url(url: &str) -> String {
    format!("{url}{CHECKSUM_SUFFIX}")
}

/// sha256sum の形の文を読み、最初の語が 16 進 64 桁ならそれを小文字にして返す。違えば `None`。
///
/// 2 つ目の語のファイル名は見ない（手で置くときに名前を書き違えても照合は値だけで決まる）。
/// BOM と前後の空白は許す。取り寄せの段も同じ決まりで読む（決まりの正本はここ）。
pub(crate) fn parse_checksum(text: &str) -> Option<String> {
    let word = text
        .trim_start_matches('\u{FEFF}')
        .split_whitespace()
        .next()?;
    (word.len() == 64 && word.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| word.to_ascii_lowercase())
}

/// 置き場のファイルを読む。無ければ [`SampleError::FetchedNarMissing`]（そのパス）。
fn read_shelf(
    name: &'static str,
    path: PathBuf,
    what: &'static str,
) -> Result<(Vec<u8>, PathBuf), SampleError> {
    match std::fs::read(&path) {
        Ok(bytes) => Ok((bytes, path)),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            Err(SampleError::FetchedNarMissing { sample: name, path })
        }
        Err(source) => Err(SampleError::Io { what, path, source }),
    }
}

/// 置き場のハッシュのファイルと書庫を読み、書庫の SHA-256 がハッシュのファイルと合えば
/// 書庫のパスを返す（要件 3.4・3.9）。大きさは見ない（登記に無い）。
///
/// # Errors
///
/// どちらかが無い（NotFound）→ [`SampleError::FetchedNarMissing`]（無かった方のパス・
/// ハッシュのファイルから見る）。形が違う → [`SampleError::FetchedChecksumInvalid`]。
/// 合わない → [`SampleError::FetchedNarMismatch`]。ほかの読み取りの失敗 → [`SampleError::Io`]。
// 窓口の 3 つの入口がこれを通るのはタスク 3.4。それまでテストの外に呼び手が無い。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn verified_nar_in(
    namespace: &Path,
    name: &'static str,
) -> Result<PathBuf, SampleError> {
    let (text, checksum_path) = read_shelf(
        name,
        fetched_checksum_path_in(namespace, name),
        "取り寄せたハッシュのファイルの読み取り",
    )?;
    let Some(expected_sha256) = std::str::from_utf8(&text).ok().and_then(parse_checksum) else {
        return Err(SampleError::FetchedChecksumInvalid {
            sample: name,
            path: checksum_path,
        });
    };
    let (bytes, path) = read_shelf(
        name,
        fetched_path_in(namespace, name),
        "取り寄せた書庫の読み取り",
    )?;
    let actual_sha256 = sha256_hex(&bytes);
    if actual_sha256 != expected_sha256 {
        return Err(SampleError::FetchedNarMismatch {
            sample: name,
            path,
            expected_sha256,
            actual_sha256,
        });
    }
    Ok(path)
}

/// 取り寄せの段のコマンドの文（絶対パス・どのディレクトリからでも動く形）。
pub(crate) fn fetch_command() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    // 絶対パスの `..` を畳むだけなので失敗しない。失敗しても畳まないパスで動く。
    let root = std::path::absolute(&root).unwrap_or(root);
    format!(
        "pwsh -NoProfile -File \"{}\"",
        root.join("tools").join("sample-fetch.ps1").display()
    )
}

/// ASCII の外の字と制御文字を `\u{XXXX}` に写す。
pub(crate) fn ascii(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii() && !c.is_ascii_control() {
            out.push(c);
        } else {
            out.push_str(&format!("\\u{{{:04X}}}", u32::from(c)));
        }
    }
    out
}

/// 手で置く口の案内。
const README: &str = "vendors/sample_ghost/README.md";

/// [`SampleError::FetchedNarMissing`] の表示（英語の ASCII）。
pub(crate) fn missing_text(sample: &str, path: &Path) -> String {
    format!(
        "fetched sample \"{}\": file not found at {}; to fix: run {}, or place both {}.nar and {}.nar{CHECKSUM_SUFFIX} in that folder by hand (see {README})",
        ascii(sample),
        ascii(&path.to_string_lossy()),
        ascii(&fetch_command()),
        ascii(sample),
        ascii(sample),
    )
}

/// [`SampleError::FetchedChecksumInvalid`] の表示（英語の ASCII）。
pub(crate) fn invalid_text(sample: &str, path: &Path) -> String {
    format!(
        "fetched sample \"{}\": checksum file at {} is not in sha256sum form (the first word must be 64 hex digits); the archive is not used; to fix: re-fetch with {}, or rewrite the checksum file by hand (see {README})",
        ascii(sample),
        ascii(&path.to_string_lossy()),
        ascii(&fetch_command()),
    )
}

/// [`SampleError::FetchedNarMismatch`] の表示（英語の ASCII）。
pub(crate) fn mismatch_text(
    sample: &str,
    path: &Path,
    expected_sha256: &str,
    actual_sha256: &str,
) -> String {
    format!(
        "fetched sample \"{}\": archive at {} does not match its checksum file: sha256 {} (checksum file {}); the archive is not used; to fix: re-fetch with {}, or place both files again by hand (see {README})",
        ascii(sample),
        ascii(&path.to_string_lossy()),
        ascii(actual_sha256),
        ascii(expected_sha256),
        ascii(&fetch_command()),
    )
}

/// URL の行の欄の形（要件 1.5）。`https://` で始まり空白と制御文字を含まない URL・
/// `--` で始まらない名前。
///
/// # Errors
///
/// 最初に見つけた形の誤りを文で返す。
#[cfg(test)]
pub(crate) fn fetched_row_form(name: &str, url: &str) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err(format!("{name}: url must start with https://: {url:?}"));
    }
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(format!(
            "{name}: url contains whitespace or control: {url:?}"
        ));
    }
    if name.starts_with("--") {
        return Err(format!("{name}: name must not start with --"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "fetched_tests.rs"]
mod tests;
