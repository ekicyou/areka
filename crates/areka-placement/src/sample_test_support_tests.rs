//! 検体の受け口が実在するシェルのフォルダを返すことの檻
//! （spec: areka-P0-shell-implicit-surface 要件 7.11・7.12）。
//!
//! 受け口は窓口 `sample_ghost_kit::SampleRoot` 経由でしかパスを組まないので、
//! ここで確かめるのは「返ったフォルダに `surfaces.txt` が実在する」ことだけでよい。
//! 綴り違い・展開先の取り違え・登記漏れは、どれもこの 1 本で赤になる。

use super::{konnoyayame_shell_root, r_post_and_komainu_shell_root};

#[test]
fn konnoyayame_shell_root_holds_surfaces_txt() {
    let shell = konnoyayame_shell_root();
    assert!(
        shell.join("surfaces.txt").is_file(),
        "konnoyayame のシェルに surfaces.txt が無い: {}",
        shell.display()
    );
}

#[test]
fn r_post_and_komainu_shell_root_holds_surfaces_txt() {
    let shell = r_post_and_komainu_shell_root();
    assert!(
        shell.join("surfaces.txt").is_file(),
        "R_POST_and_KOMAINU のシェルに surfaces.txt が無い: {}",
        shell.display()
    );
}
