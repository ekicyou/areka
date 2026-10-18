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

/// `<namespace>\fetched\<name>.nar`。
pub(crate) fn fetched_path_in(namespace: &Path, name: &str) -> PathBuf {
    namespace.join(FETCHED).join(format!("{name}.nar"))
}

/// 置き場の書庫を読み、大きさと SHA-256 が登記と合えばそのパスを返す（要件 3.4・3.9）。
///
/// 大きさ → SHA-256 の順に比べるが、大きさが違っても SHA-256 は計算して失敗に載せる
/// （要件 3.6 が両方の SHA-256 を求める）。
///
/// # Errors
///
/// 無い（NotFound）→ [`SampleError::FetchedNarMissing`]。合わない →
/// [`SampleError::FetchedNarMismatch`]。ほかの読み取りの失敗 → [`SampleError::Io`]。
// 窓口の 3 つの入口がこれを通るのはタスク 3.3。それまでテストの外に呼び手が無い。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn verified_nar_in(
    namespace: &Path,
    name: &'static str,
    sha256: &'static str,
    size: u64,
) -> Result<PathBuf, SampleError> {
    let path = fetched_path_in(namespace, name);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Err(SampleError::FetchedNarMissing { sample: name, path });
        }
        Err(source) => {
            return Err(SampleError::Io {
                what: "取り寄せた書庫の読み取り",
                path,
                source,
            });
        }
    };
    let actual_size = bytes.len() as u64;
    let actual_sha256 = sha256_hex(&bytes);
    if actual_size != size || actual_sha256 != sha256 {
        return Err(SampleError::FetchedNarMismatch {
            sample: name,
            path,
            expected_size: size,
            actual_size,
            expected_sha256: sha256,
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
        "fetched sample \"{}\" archive not found at {}; to fix: run {}, or place the archive there by hand (see {README})",
        ascii(sample),
        ascii(&path.to_string_lossy()),
        ascii(&fetch_command()),
    )
}

/// [`SampleError::FetchedNarMismatch`] の表示（英語の ASCII）。
pub(crate) fn mismatch_text(
    sample: &str,
    path: &Path,
    expected_size: u64,
    actual_size: u64,
    expected_sha256: &str,
    actual_sha256: &str,
) -> String {
    format!(
        "fetched sample \"{}\" archive at {} does not match the registry: size {actual_size} (registry {expected_size}), sha256 {} (registry {}); the archive is not used; to fix: re-fetch with {}, or replace it with the registered archive (see {README})",
        ascii(sample),
        ascii(&path.to_string_lossy()),
        ascii(actual_sha256),
        ascii(expected_sha256),
        ascii(&fetch_command()),
    )
}

/// URL の行の欄の形（要件 1.5）。`https://` で始まり空白と制御文字を含まない URL・
/// 小文字 16 進 64 桁の SHA-256・0 より大きい大きさ・`--` で始まらない名前。
///
/// # Errors
///
/// 最初に見つけた形の誤りを文で返す。
#[cfg(test)]
pub(crate) fn fetched_row_form(
    name: &str,
    url: &str,
    sha256: &str,
    size: u64,
) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err(format!("{name}: url must start with https://: {url}"));
    }
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(format!(
            "{name}: url contains whitespace or control: {url:?}"
        ));
    }
    if sha256.len() != 64
        || !sha256
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err(format!(
            "{name}: sha256 must be 64 lowercase hex digits: {sha256}"
        ));
    }
    if size == 0 {
        return Err(format!("{name}: size must be greater than 0"));
    }
    if name.starts_with("--") {
        return Err(format!("{name}: name must not start with --"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "fetched_tests.rs"]
mod tests;
