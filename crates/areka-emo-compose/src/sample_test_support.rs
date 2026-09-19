//! emo2 検体を本 crate のテストへ配る共有の受け口（spec: areka-P0-nar-install 要件 1.6）。
//!
//! 検体は窓口 `sample-ghost-kit` から取得し、**プロセス寿命で保持**する。段 ③ で `SampleRoot` の
//! `Drop` が展開した複製を消すため、関数内の一時値にすると借用の元がその場で消える。保持を
//! 1 か所に束ねてあるので、段 ③ で複製を作るのもテストバイナリあたり 1 回で済む
//! （各ファイルが自前で保持すると、その数だけ木が複製される）。

use std::path::PathBuf;
use std::sync::LazyLock;

use sample_ghost_kit::SampleRoot;

static EMO2: LazyLock<SampleRoot> =
    LazyLock::new(|| SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体"));

/// emo2 検体のゴーストフォルダ。
pub(crate) fn emo2_root() -> PathBuf {
    EMO2.folder().to_path_buf()
}
