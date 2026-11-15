//! 見本のゴーストの入口（配置のテストだけ・`sample-ghost-kit` はここからだけ引く）。
//!
//! 本体の側の同じ働きの入口は本体の `runtime/sample_test_support.rs`（crate ごとに 1 つ・
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

/// emo2 の実寸（物理 px・PNG を実際に読んだ値）。配置のテストはこの 1 組だけを使う。
/// 本体側の初期の面（surface0 ← surface0.png）。
pub(crate) const SCOPE0_W: i32 = 434;
pub(crate) const SCOPE0_H: i32 = 687;
/// 相方側の初期の面（surface10 ← CityPop/surface0010.png）。
pub(crate) const SCOPE1_W: i32 = 336;
pub(crate) const SCOPE1_H: i32 = 400;
/// 本体側のバルーンの面 0（`balloons0.png`＝本体側の系列）。
pub(crate) const BALLOON0_W: i32 = 400;
pub(crate) const BALLOON0_H: i32 = 224;
/// 相方側のバルーンの面 0（`balloonk0.png`＝相方側の系列）。本体側と**異なる**寸であることが
/// この検体の判別力の源である。
pub(crate) const BALLOON1_W: i32 = 288;
pub(crate) const BALLOON1_H: i32 = 203;

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
