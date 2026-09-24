//! emo2 検体を `emo2_boot` 配下のテストへ配る共有の受け口（spec: areka-P0-nar-install 要件 1.6）。
//!
//! 検体は窓口 `sample-ghost-kit` から取得する。取得のたびに**起動記録の無い新品の複製**が
//! 配られ、得た値を捨てた時点でその複製は消える（要件 7.4）。配り方は用途で 2 通りある。
//!
//! - **ゴーストを起動する**テストは [`acquire_emo2`] を呼び、得た値を起動したものと同じ
//!   寿命で保持する。共有の複製を使い回すと、同じプロセスの 2 回目の起動が 1 回目の
//!   `[boot] count` を読んで「2 回目起動」と判定し、`OnFirstBoot` を発行しなくなる。
//! - **読むだけ**のテストは [`emo2_root`]／[`emo2_balloon_root`] を使う。こちらは
//!   プロセス寿命の複製 1 つを共有するので、テストバイナリあたり木は 1 つで済む
//!   （各ファイルが自前で取得すると、その数だけ木が複製される）。
//!
//! どちらも `SampleRoot` の `Drop` が複製を消すため、関数内の一時値にすると借用の元が
//! その場で消える。保持の口をこの 1 ファイルに束ねてある。

use std::path::PathBuf;
use std::sync::LazyLock;

use sample_ghost_kit::SampleRoot;

/// 読むだけのテストが共有する複製（プロセス寿命）。
static EMO2: LazyLock<SampleRoot> = LazyLock::new(acquire_emo2);

/// emo2 検体の**使い捨ての複製**を 1 つ取得する（要件 7.4・起動を伴うテスト向け）。
pub(crate) fn acquire_emo2() -> SampleRoot {
    SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体")
}

/// emo2 検体のゴーストフォルダ（読むだけのテスト向けの共有複製）。
pub(super) fn emo2_root() -> PathBuf {
    EMO2.folder().to_path_buf()
}

/// emo2 検体が同時にインストールするバルーンのフォルダ（自分でパスを継ぎ足さない）。
pub(super) fn emo2_balloon_root() -> PathBuf {
    EMO2.balloon("emo2-kakukaku")
        .expect("emo2 の同梱バルーン")
        .to_path_buf()
}
