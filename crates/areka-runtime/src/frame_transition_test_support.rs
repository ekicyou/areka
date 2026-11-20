//! 拡大率の遷移と追従オフセットのテストが共有する補助（書込の選り分け・整地・遷移の駆動・
//! 観測行の拾い出し）。`frame_test_support.rs` は 1,000 行の上限に近いので兄弟のファイルに置く。

use log_capture_kit::CapturedEvent;
use wintf::ecs::DPI;
use wintf::ecs::window::SetWindowPosCommand;

use areka_placement::config::BalloonXMode;
use areka_placement::diag::WindowKind;
use areka_placement::follow::{Anchored, BalloonFollow, OffsetBase};
use areka_placement::resolver::{Anchor, PointPx};
use areka_placement::spawn::BalloonKeywordBase;

use super::shared_test_support::{BASE_OFFSETS, HIGH_DPI, LOW_DPI};
use super::test_support::{
    FakeReports, FrameHarness, s2_monitors_with_work_area, s2_work_area_for_dpi,
};

/// キーワード素材の揃え種別（中央上＝バルーン下端がシェル画像上端に接する）。
pub const KEYWORD_MODE: BalloonXMode = BalloonXMode::CenterTop;

/// キーワード素材の調整量（作者指定 0＝揃えの式だけを見る）。
pub const KEYWORD_ADJUST: PointPx = PointPx { x: 0, y: 0 };

/// 当該拡大率水準における物理寸（等倍寸を `dpi/96` 倍する）。
pub fn scaled(size: (u32, u32), dpi: u16) -> (u32, u32) {
    let k = u32::from(dpi);
    (size.0 * k / 96, size.1 * k / 96)
}

/// 指定スコープ・指定種別の窓書込だけを取り出す。
pub fn writes_for(
    writes: &[SetWindowPosCommand],
    scope: u32,
    kind: WindowKind,
) -> Vec<SetWindowPosCommand> {
    writes
        .iter()
        .filter(|cmd| cmd.tag.scope == Some(scope) && cmd.tag.kind == kind.as_str())
        .cloned()
        .collect()
}

/// 捕捉行のうち WARN で、`needle` を本文に含むものの件数。
pub fn warnings_containing(events: &[CapturedEvent], needle: &str) -> usize {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .filter(|e| e.message().contains(needle))
        .count()
}

/// 当該スコープの追従 Component。
pub fn follow_of(harness: &FrameHarness, scope: usize) -> BalloonFollow {
    harness
        .world
        .get::<BalloonFollow>(harness.char_window(scope))
        .copied()
        .expect("キャラ窓に BalloonFollow がある")
}

/// 当該スコープの追従オフセット（現在値）。
pub fn offset_of(harness: &FrameHarness, scope: usize) -> PointPx {
    follow_of(harness, scope).offset()
}

/// `kind=offset` の観測行だけを取り出す。
pub fn offset_lines(logs: &[String]) -> Vec<&String> {
    logs.iter().filter(|l| l.contains("kind=offset")).collect()
}

/// 起動直後の整地——3 つの源と窓の拡大率を `dpi` へ揃え、拡大率の相の初回全窓マッチ
/// （永続 `SystemState` の仕様）を空回しして消費する。窓書込のキューと witness は掃除する。
///
/// 済んだことを自己検査する: 基準対が「fixture の値 × 係留済み `dpi`」であること（係留が
/// 済んでいないと次の遷移は係留の腕へ落ち、追随が起きないまま緑になる）と、下端に
/// アンカーしたキャラ窓が `dpi` の作業領域下端へ接地していること。
pub fn settle_at(harness: &mut FrameHarness, source: &mut FakeReports, dpi: u16) {
    harness.set_monitor_sources_for_dpi(dpi);
    harness.set_monitor_table_for_dpi(dpi);
    harness.set_window_dpi(dpi);
    harness.advance_frame();
    harness.run_placement_phases(source);
    let _priming = harness.drain_writes();
    harness.reset_write_witness();
    for scope in harness.scopes().to_vec() {
        assert_eq!(
            follow_of(harness, scope).base(),
            OffsetBase {
                offset: BASE_OFFSETS[scope],
                dpi: Some(DPI::from_dpi(dpi, dpi)),
            },
            "前提が崩れている: scope={scope} の基準対が「fixture 値 × 係留済み {dpi}」になっていない"
        );
        let anchored = harness
            .world
            .get::<Anchored>(harness.char_window(scope))
            .copied()
            .expect("キャラ窓に Anchored がある");
        if anchored == Anchored(Anchor::Bottom) {
            assert_eq!(
                harness.ground_point(scope).1,
                s2_work_area_for_dpi(dpi).bottom,
                "前提が崩れている: dpi={dpi} scope={scope} のキャラ窓が作業領域下端へ接地していない"
            );
        }
    }
}

/// 表示 DPI を `dpi` へ動かして 1 フレーム回す（モニタ表と窓の拡大率を同時に更新＝
/// 待ち札の付かない通常の遷移）。
pub fn transition_to(harness: &mut FrameHarness, source: &mut FakeReports, dpi: u16) {
    harness.set_monitor_table_for_dpi(dpi);
    harness.set_window_dpi(dpi);
    harness.advance_frame();
    harness.run_placement_phases(source);
}

/// **拡大率だけ**を [`HIGH_DPI`] へ上げる（作業領域は [`LOW_DPI`] のまま＝べき等 skip 腕）。
pub fn raise_scale_without_moving_the_work_area(harness: &mut FrameHarness) {
    harness.set_monitor_table(s2_monitors_with_work_area(
        HIGH_DPI,
        s2_work_area_for_dpi(LOW_DPI),
    ));
    harness.set_window_dpi(HIGH_DPI);
}

/// 全スコープのキャラ窓へキーワード素材を付ける（本番の spawn が `ScopePlacement` から
/// 付けるのと同じ Component をそのまま置く）。
pub fn attach_material(harness: &mut FrameHarness) {
    for scope in harness.scopes().to_vec() {
        let char_window = harness.char_window(scope);
        harness
            .world
            .entity_mut(char_window)
            .insert(BalloonKeywordBase {
                mode: KEYWORD_MODE,
                adjust: KEYWORD_ADJUST,
            });
    }
}
