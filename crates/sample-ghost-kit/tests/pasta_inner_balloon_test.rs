//! 取り寄せたぱすたの書庫の中身の形を見張る（spec: `areka-P0-pasta-in-windows-verification`
//! 要件 2.1〜2.7）。
//!
//! 見るのは 2 点だけ: 縦書きのバルーンがゴーストのフォルダの直下に在り名前を名乗ること、
//! 根の `balloon/` に何も入っていないこと。書庫の大きさ・数・ハッシュは書かない（2.3）。
//! 窓口は置き場の書庫しか読まないので通信しない（2.5）。取り寄せの段が回っていない机では
//! 窓口の失敗の文（取り寄せのコマンドを含む）で落ちる（2.4）。

use std::path::Path;

use areka_nar::{InstallRequest, NarArchive, SingleRoot};
use sample_ghost_kit::{NarBuilder, SampleRoot, WorkDir, install_txt};

const INNER_BALLOON_DIR: &str = "pasta_tategaki";
const INNER_BALLOON_NAME_LINE: &str = "name,窓際のぱすたさん縦書き";

/// ゴーストのフォルダの直下の縦書きのバルーンと、根の balloon/ が空であることを確かめる。
/// Err の文は、何が無かったか（どのパス・どの行）を書く。
fn check_pasta_layout(root: &Path, ghost_folder: &Path) -> Result<(), String> {
    let descript = ghost_folder.join(INNER_BALLOON_DIR).join("descript.txt");
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

    // 根の直下で名前が balloon の子を探す（パスを継ぎ足さない）。
    let entries =
        std::fs::read_dir(root).map_err(|err| format!("{} を読めない: {err}", root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("{} を読めない: {err}", root.display()))?;
        if entry.file_name() != "balloon" {
            continue;
        }
        let path = entry.path();
        let inside: Vec<String> = std::fs::read_dir(&path)
            .map_err(|err| format!("{} を読めない: {err}", path.display()))?
            .map(|child| {
                child
                    .map(|c| c.file_name().to_string_lossy().into_owned())
                    .map_err(|err| format!("{} を読めない: {err}", path.display()))
            })
            .collect::<Result<_, _>>()?;
        if !inside.is_empty() {
            return Err(format!(
                "{} は 0 件のはずが {inside:?} を持つ",
                path.display()
            ));
        }
    }
    Ok(())
}

#[test]
fn fetched_pasta_puts_the_vertical_balloon_inside_its_ghost_folder() {
    let pasta = SampleRoot::acquire("pasta-in-windows").unwrap_or_else(|err| panic!("{err}"));
    check_pasta_layout(pasta.root(), pasta.folder()).unwrap_or_else(|err| panic!("{err}"));
}

/// 較正（2.7）: 縦書きのバルーンをゴーストのフォルダの外（`ghost/master/` の下）へ置いた
/// 書庫を入れると、判定は赤を返す。木のどこかを探すだけの判定ならここを通してしまう。
#[test]
fn a_vertical_balloon_outside_the_ghost_folder_is_rejected() {
    let ghost = "pasta-in-windows";
    let work = WorkDir::new().expect("書庫を書く場所は取れるはず");
    let nar = work.path().join("outside.nar");
    let name_line = format!("{INNER_BALLOON_NAME_LINE}\r\n");
    NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "charset,UTF-8",
                "type,ghost",
                "name,outside",
                &format!("directory,{ghost}"),
            ]),
        )
        .done()
        .file(
            "ghost/master/descript.txt",
            b"charset,UTF-8\r\ntype,ghost\r\n",
        )
        .done()
        .file(
            format!("ghost/master/{INNER_BALLOON_DIR}/descript.txt"),
            format!("charset,UTF-8\r\ntype,balloon\r\n{name_line}").as_bytes(),
        )
        .done()
        .write_to(&nar)
        .expect("固定入力の書庫を書けるはず");

    let root = WorkDir::new().expect("空の根は取れるはず");
    NarArchive::open(&nar)
        .expect("固定入力の書庫は受理されるはず")
        .install(&InstallRequest {
            roots: &SingleRoot(root.path()),
            target_ghost: None,
        })
        .expect("固定入力は空の根へ入るはず");

    let folder = root.path().join("ghost").join(ghost);
    assert!(
        folder.is_dir(),
        "固定入力のゴーストのフォルダが在ること（判定が空振りしない前提）: {}",
        folder.display()
    );
    let result = check_pasta_layout(root.path(), &folder);
    assert!(
        result.is_err(),
        "ゴーストのフォルダの外の縦書きのバルーンは赤のはず: {result:?}"
    );
}
