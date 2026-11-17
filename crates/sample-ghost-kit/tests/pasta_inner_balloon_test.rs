//! 取り寄せたぱすたの書庫の中身の形を見張る（spec: `areka-P0-pasta-in-windows-verification`
//! 要件 2.1〜2.7）。
//!
//! 見るのは縦書きのバルーンがゴーストのフォルダの中の `balloon/` に在り名前を名乗ること。
//! 根の `balloon/` が空なことは窓口の登記の照合（`check_registry`）が断る。書庫の大きさ・
//! 数・ハッシュは書かない（2.3）。
//! 窓口は置き場の書庫しか読まないので通信しない（2.5）。取り寄せの段が回っていない机では
//! 窓口の失敗の文（取り寄せのコマンドを含む）で落ちる（2.4）。

use std::path::Path;

use sample_ghost_kit::{SampleRoot, WorkDir};

const INNER_BALLOON_DIR: &str = "pasta_tategaki";
const INNER_BALLOON_NAME_LINE: &str = "name,窓際のぱすたさん縦書き";

/// ゴーストのフォルダの中の `balloon/` の縦書きのバルーンを確かめる。
/// Err の文は、何が無かったか（どのパス・どの行）を書く。
fn check_pasta_layout(ghost_folder: &Path) -> Result<(), String> {
    let descript = ghost_folder
        .join("balloon")
        .join(INNER_BALLOON_DIR)
        .join("descript.txt");
    let text = std::fs::read_to_string(&descript)
        .map_err(|err| format!("{} を読めない: {err}", descript.display()))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    if !text
        .lines()
        .any(|line| line.trim() == INNER_BALLOON_NAME_LINE)
    {
        return Err(format!(
            "{} に行 `{INNER_BALLOON_NAME_LINE}` が無い",
            descript.display()
        ));
    }
    Ok(())
}

#[test]
fn fetched_pasta_puts_the_vertical_balloon_inside_its_ghost_folder() {
    let pasta = SampleRoot::acquire("pasta-in-windows").unwrap_or_else(|err| panic!("{err}"));
    check_pasta_layout(pasta.folder()).unwrap_or_else(|err| panic!("{err}"));
}

/// ゴーストのフォルダの形を直に書く（書庫を介さない）。`relative` はフォルダからの相対の段。
fn ghost_folder_with(work: &WorkDir, relative: &[&str], descript: &str) -> std::path::PathBuf {
    let folder = work.path().join("ghost").join("pasta-in-windows");
    let file = relative
        .iter()
        .fold(folder.clone(), |path, part| path.join(part))
        .join("descript.txt");
    std::fs::create_dir_all(file.parent().expect("親がある")).expect("フォルダを作れるはず");
    std::fs::write(&file, descript).expect("descript.txt を書けるはず");
    folder
}

/// 較正（2.7）: 判定は正しい形を通し、外に置いた形と名前の行の無い形を赤にする。
/// 木のどこかを探すだけの判定は外の形を、ファイルの在る無しだけの判定は名前の行の無い
/// 形を通してしまう。
#[test]
fn the_layout_check_passes_only_a_named_balloon_inside_the_ghost_folder() {
    let named = format!("charset,UTF-8\r\ntype,balloon\r\n{INNER_BALLOON_NAME_LINE}\r\n");

    let work = WorkDir::new().expect("場所は取れるはず");
    let inside = ghost_folder_with(&work, &["balloon", INNER_BALLOON_DIR], &named);
    assert_eq!(check_pasta_layout(&inside), Ok(()), "正しい形は通る");

    let work = WorkDir::new().expect("場所は取れるはず");
    let outside = ghost_folder_with(&work, &["ghost", "master", INNER_BALLOON_DIR], &named);
    let result = check_pasta_layout(&outside);
    assert!(
        result.is_err(),
        "ゴーストのフォルダの外の縦書きのバルーンは赤のはず: {result:?}"
    );

    let work = WorkDir::new().expect("場所は取れるはず");
    let unnamed = ghost_folder_with(
        &work,
        &["balloon", INNER_BALLOON_DIR],
        "charset,UTF-8\r\ntype,balloon\r\n",
    );
    let result = check_pasta_layout(&unnamed);
    assert!(
        result.is_err(),
        "名前の行の無いバルーンは赤のはず: {result:?}"
    );
}
