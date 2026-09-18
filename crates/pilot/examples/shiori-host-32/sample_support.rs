//! emo2 検体を本 example のターゲットへ配る共有の受け口（spec: areka-P0-nar-install 要件 1.6）。
//!
//! 検体は窓口 `sample-ghost-kit` から取得し、**プロセス寿命で保持**する。段 ③ で `SampleRoot` の
//! `Drop` が展開した複製を消すため、関数内の一時値にすると借用の元がその場で消える——helper が
//! 引数も `GHOSTDIR` も無いときに落ちる退避経路は、まさにこの寿命に依存する。
//!
//! 窓口は純 Rust（依存は `thiserror` のみ）なので、i686 の helper ターゲットでも組める。
//! 本ファイルは `#[path]` で helper（i686）と親 entry（x64）の両方へ取り込むが、静的変数は
//! ターゲットごとに 1 つずつ＝1 コンパイル単位に 1 つである。

use std::path::PathBuf;
use std::sync::LazyLock;

use sample_ghost_kit::SampleRoot;

static EMO2: LazyLock<SampleRoot> =
    LazyLock::new(|| SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体"));

/// emo2 検体の `ghost/master`（`pasta.dll` の在処＝SHIORI `load` 対象）の絶対パス。
pub fn emo2_ghost_master() -> PathBuf {
    EMO2.folder().join("ghost").join("master")
}
