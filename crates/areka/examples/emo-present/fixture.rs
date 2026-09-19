use super::PathBuf;
use sample_ghost_kit::SampleRoot;
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// Constants / fixture paths
// ---------------------------------------------------------------------------

/// シェル窓の初期位置（物理 px・スクリーン座標）。
pub(super) const SHELL_INITIAL_X: i32 = 400;
pub(super) const SHELL_INITIAL_Y: i32 = 200;

/// まばたき開閉の周期（秒）。この周期で目開き ⇄ 目閉じ を surface1000 上でトグルする（R6.4 の切替観測）。
pub(super) const CYCLE_INTERVAL_SECS: f64 = 2.5;

/// 本 example が両 target へ与える**作者基準 DPI**（ukadoc 正典既定の 96・D1）。
///
/// 本番アプリは descript の実値（shell `seriko.dpi`／balloon `dpi`）を `attach_target` へ渡すが、
/// 本 example は fixture を直接読むため正典既定を固定で与える。`assert_startup_golden` が golden へ
/// 掛ける k を導出する際の**分母も同じ値**でなければならないため、両者を 1 つの定数に束ねる
/// （`attach_target` 側だけ変えて golden 側が古い分母のまま残る、という食い違いを構造的に潰す）。
pub(super) const AUTHOR_DPI: u16 = 96;

/// emo2 検体。段 ③ で `Drop` が複製を消すため、一時値にせずプロセス寿命で保持する。
static EMO2: LazyLock<SampleRoot> =
    LazyLock::new(|| SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体"));

pub(super) fn emo2(rel: &str) -> PathBuf {
    EMO2.folder().join(rel)
}

pub(super) fn emo2_balloon() -> PathBuf {
    EMO2.balloon("emo2-kakukaku")
        .expect("emo2 の同梱バルーン")
        .to_path_buf()
}
