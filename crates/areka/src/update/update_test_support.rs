//! `update/` のテストが分け合う小道具（`#[test]` を持たない）。

use std::path::Path;

use areka_runtime::spine::RecordedCall;
use log_capture_kit::CapturedEvent;

/// `dir` に `descript.txt`（`charset,UTF-8` の頭＋`body`＋末尾の改行・CRLF）を置く。
pub(crate) fn write_descript(dir: &Path, body: &str) {
    std::fs::create_dir_all(dir).expect("検体のフォルダを作る");
    std::fs::write(
        dir.join("descript.txt"),
        format!("charset,UTF-8\r\n{body}\r\n"),
    )
    .expect("descript.txt を書く");
}

/// `event` の名が `name` の記録。
pub(crate) fn named<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

/// `id` の GET の Reference（最初の 1 件）。
pub(crate) fn refs_of(calls: &[RecordedCall], id: &str) -> Option<Vec<String>> {
    calls.iter().find_map(|call| match call {
        RecordedCall::Get {
            id: got,
            references,
        } if got == id => Some(references.clone()),
        _ => None,
    })
}
