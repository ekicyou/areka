//! 試験の道具（`#[cfg(test)]` だけ・本番からは参照しない）。
//!
//! 常時テストはここの [`FakeFetch`] だけで一周を回し、ネットワークに触れない（9.6）。
//! 作業場所は `sample_ghost_kit::WorkDir`（OS の一時フォルダは使わない）。

use crate::error::FetchError;
use crate::fetch::Fetch;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;

/// 固定表の 1 行の答え。
type Answer = Result<Vec<u8>, FetchError>;
/// 取得のたびの差し込み。
type Hook = Box<dyn Fn(&str)>;

/// URL → バイト列または失敗の固定表（3.2）。呼ばれた URL を記録し、取得のたびに `on_get` を呼ぶ。
#[derive(Default)]
pub(crate) struct FakeFetch {
    table: BTreeMap<String, Answer>,
    calls: RefCell<Vec<String>>,
    on_get: Option<Hook>,
}

impl FakeFetch {
    pub(crate) fn new() -> FakeFetch {
        FakeFetch::default()
    }

    /// `url` にバイト列を返させる。
    pub(crate) fn serve(mut self, url: &str, bytes: &[u8]) -> FakeFetch {
        self.table.insert(url.to_owned(), Ok(bytes.to_vec()));
        self
    }

    /// `url` に失敗を返させる（失敗の注入）。
    pub(crate) fn fail(mut self, url: &str, err: FetchError) -> FakeFetch {
        self.table.insert(url.to_owned(), Err(err));
        self
    }

    /// 取得のたびに、答える前に呼ぶ差し込み（取得の最中に木を書き換える注入の口）。
    pub(crate) fn on_get(mut self, hook: impl Fn(&str) + 'static) -> FakeFetch {
        self.on_get = Some(Box::new(hook));
        self
    }

    /// 呼ばれた URL（呼ばれた順）。
    pub(crate) fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }
}

impl Fetch for FakeFetch {
    /// 表に無い URL は `NotFound`（「無い」）。
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        self.calls.borrow_mut().push(url.to_owned());
        if let Some(hook) = &self.on_get {
            hook(url);
        }
        self.table
            .get(url)
            .cloned()
            .unwrap_or(Err(FetchError::NotFound))
    }
}

/// 相対パス → バイト列（フォルダは末尾 `/`・値は空）。9.4 のバイト単位の比較に使う。
///
/// フォルダが無ければ空（`areka-nar` の `install_tests.rs` の `tree` の写し）。
pub(crate) fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    if root.is_dir() {
        walk(root, "", &mut out);
    }
    out
}

fn walk(dir: &Path, prefix: &str, out: &mut BTreeMap<String, Vec<u8>>) {
    for child in fs::read_dir(dir).expect("木を辿れる") {
        let child = child.expect("要素を読める");
        let rel = format!("{prefix}{}", child.file_name().to_string_lossy());
        if child.file_type().expect("種別を読める").is_dir() {
            out.insert(format!("{rel}/"), Vec::new());
            walk(&child.path(), &format!("{rel}/"), out);
        } else {
            out.insert(rel, fs::read(child.path()).expect("中身を読める"));
        }
    }
}

/// `FILE_SHARE_READ`。
const SHARE_READ: u32 = 1;

/// `share_mode(FILE_SHARE_READ)` で開いたまま持ち、`rename`／`remove` を失敗させる
/// （`install_commit_tests.rs` の `hold` と同じ・読み込まれた DLL と同じく読みは通る）。
pub(crate) fn hold(path: &Path) -> fs::File {
    fs::OpenOptions::new()
        .read(true)
        .share_mode(SHARE_READ)
        .open(path)
        .unwrap_or_else(|err| panic!("{} を掴めるはず: {err}", path.display()))
}

/// `updates2.dau` の固定入力: 欄を `\x01` で、行を CRLF（`crlf`）か LF で終える。
pub(crate) fn dau(lines: &[&[&str]], crlf: bool) -> Vec<u8> {
    let eol = if crlf { "\r\n" } else { "\n" };
    lines
        .iter()
        .map(|fields| format!("{}{eol}", fields.join("\x01")))
        .collect::<String>()
        .into_bytes()
}

/// `updates.txt` の固定入力: 行を CRLF で終える（`file,`・`charset,` は呼び手が綴る）。
pub(crate) fn txt(lines: &[&str]) -> Vec<u8> {
    lines
        .iter()
        .map(|line| format!("{line}\r\n"))
        .collect::<String>()
        .into_bytes()
}

/// Shift_JIS のバイト列。符号化できない文字があれば赤。
pub(crate) fn sjis(s: &str) -> Vec<u8> {
    let (bytes, _, lossy) = encoding_rs::SHIFT_JIS.encode(s);
    assert!(!lossy, "Shift_JIS で符号化できない文字: {s:?}");
    bytes.into_owned()
}

/// `link` に `target` を指すジャンクションを作る（`cmd /c mklink /J`）。作れなければ赤。
pub(crate) fn junction(link: &Path, target: &Path) {
    let out = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .expect("cmd を起こせる");
    assert!(
        out.status.success() && link.exists(),
        "ジャンクションを作れない: {} -> {}: {}{}",
        link.display(),
        target.display(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[path = "testkit_tests.rs"]
mod tests;
