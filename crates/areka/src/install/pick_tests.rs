//! ファイルを選ぶ画面の純粋な部分の決定論テスト（要件 1.3）。画面は出さない。
//!
//! 確かめること: フィルタが「書庫」と「すべてのファイル」の 2 組を NUL で区切り、NUL 2 つで
//! 閉じること・受けの領域から最初の NUL までがパスになること。

use super::*;

/// フィルタは（表示名・パターン）×2 の 4 語と、末尾の空の語（NUL 2 つで閉じる）。
#[test]
fn filter_lists_archives_then_all_files_and_ends_with_two_nuls() {
    let wide = filter_wide();
    let words: Vec<String> = wide
        .split(|&c| c == 0)
        .map(String::from_utf16_lossy)
        .collect();
    assert_eq!(
        words,
        vec![
            "書庫 (*.nar;*.zip)",
            "*.nar;*.zip",
            "すべてのファイル (*.*)",
            "*.*",
            "",
            "",
        ],
        "区切りの NUL 4 つ＋閉じの NUL 2 つ"
    );
    assert_eq!(&wide[wide.len() - 2..], &[0, 0]);
}

/// 受けの領域は最初の NUL までを読む（後ろの残りは読まない）。
#[test]
fn path_is_read_up_to_the_first_nul() {
    let mut buffer: Vec<u16> = r"C:\ダウンロード\hana.nar".encode_utf16().collect();
    buffer.extend([0, u16::from(b'x'), 0]);
    assert_eq!(
        path_from_buffer(&buffer),
        PathBuf::from(r"C:\ダウンロード\hana.nar")
    );
}
