//! 見本のゴーストの入口（配置のテストだけ・`sample-ghost-kit` はここからだけ引く）。
//!
//! 本体の側の同じ働きの入口は本体の `emo2_boot/sample_test_support.rs`（crate ごとに 1 つ・
//! spec: placement-crate-split 要件 3.2）。

use std::path::PathBuf;
use std::sync::LazyLock;

use sample_ghost_kit::SampleRoot;

/// emo2 検体。段 ③ で `Drop` が複製を消すため、一時値にせずプロセス寿命で保持する。
static EMO2: LazyLock<SampleRoot> =
    LazyLock::new(|| SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体"));

pub(crate) fn emo2_root() -> PathBuf {
    EMO2.folder().to_path_buf()
}

pub(crate) fn balloon_root() -> PathBuf {
    EMO2.balloon("emo2-kakukaku")
        .expect("emo2 の同梱バルーン")
        .to_path_buf()
}

/// 里々の標準テンプレート `R_POST_and_KOMAINU` 検体（spec: areka-P0-shell-implicit-surface 要件 7.11）。
static R_POST_AND_KOMAINU: LazyLock<SampleRoot> = LazyLock::new(|| {
    SampleRoot::acquire("R_POST_and_KOMAINU").expect("R_POST_and_KOMAINU は登記済みの検体")
});

/// YAYA の標準テンプレート `konnoyayame` 検体（同上）。
static KONNOYAYAME: LazyLock<SampleRoot> =
    LazyLock::new(|| SampleRoot::acquire("konnoyayame").expect("konnoyayame は登記済みの検体"));

/// `R_POST_and_KOMAINU` 検体のシェルのフォルダ（`shell/master/`）。
pub(crate) fn r_post_and_komainu_shell_root() -> PathBuf {
    R_POST_AND_KOMAINU.folder().join("shell/master")
}

/// `konnoyayame` 検体のシェルのフォルダ（`shell/master/`）。
pub(crate) fn konnoyayame_shell_root() -> PathBuf {
    KONNOYAYAME.folder().join("shell/master")
}

#[cfg(test)]
#[path = "sample_test_support_tests.rs"]
mod tests;
