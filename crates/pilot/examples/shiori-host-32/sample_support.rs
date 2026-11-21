//! emo2 検体を本 example のターゲットへ配る共有の受け口。
//!
//! 検体は窓口 `sample-ghost-kit` から取得し、プロセス寿命で保持する。`SampleRoot` の `Drop` が
//! 展開した複製を消すので、関数内の一時値にすると返したパスの先がその場で消える。
//!
//! 窓口は純 Rust なので、i686 の helper ターゲットでも組める。

use std::path::PathBuf;
use std::sync::LazyLock;

use sample_ghost_kit::SampleRoot;

static EMO2: LazyLock<SampleRoot> =
    LazyLock::new(|| SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体"));

/// emo2 検体の `ghost/master`（`pasta.dll` の在処＝SHIORI `load` 対象）の絶対パス。
pub fn emo2_ghost_master() -> PathBuf {
    EMO2.folder().join("ghost").join("master")
}
