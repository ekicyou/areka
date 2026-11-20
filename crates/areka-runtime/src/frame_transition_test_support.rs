//! 拡大率の遷移と追従オフセットのテストが共有する補助（書込の選り分け・整地・遷移の駆動・
//! 観測行の拾い出し）。`frame_test_support.rs` は 1,000 行の上限に近いので兄弟のファイルに置く。

use bevy_ecs::entity::Entity;
use log_capture_kit::CapturedEvent;
use wintf::ecs::window::SetWindowPosCommand;
use wintf::ecs::window::monitor::Monitor;
use wintf::ecs::{DPI, Point, SizeI, WindowPos};

use areka_placement::config::BalloonXMode;
use areka_placement::diag::WindowKind;
use areka_placement::dpi_sync::DpiSyncHold;
use areka_placement::follow::{Anchored, BalloonFollow, OffsetBase};
use areka_placement::resolver::{Anchor, PointPx};
use areka_placement::spawn::BalloonKeywordBase;

use super::shared_test_support::{BALLOON_SPAWN_SIZE, BASE_OFFSETS, HIGH_DPI, LOW_DPI, SCALE_120};
use super::test_support::{
    FakeReports, FrameHarness, SPAWN_SIZE_0, SPAWN_SIZE_1, s2_monitors, s2_monitors_with_work_area,
    s2_work_area_for_dpi,
};
use super::{balloon_target, shell_target};

/// キーワード素材の揃え種別（中央上＝バルーン下端がシェル画像上端に接する）。
pub const KEYWORD_MODE: BalloonXMode = BalloonXMode::CenterTop;

/// キーワード素材の調整量（作者指定 0＝揃えの式だけを見る）。
pub const KEYWORD_ADJUST: PointPx = PointPx { x: 0, y: 0 };

/// キャラ窓を隣接モニタへ置くときの位置（随伴バルーンも隣接モニタ内へ収まる値）。
pub const CHAR_ON_NEIGHBOR: Point = Point { x: 3200, y: 800 };

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

/// `(u32, u32)` の報告寸を窓寸の通貨（`SizeI`）へ移す。
pub fn as_window_size(size: (u32, u32)) -> SizeI {
    SizeI {
        width: size.0 as i32,
        height: size.1 as i32,
    }
}

/// 隣接モニタ（ゴーストが決して居ない側）だけを当該水準にした実行時のモニタ表。
pub fn neighbor_at(dpi: u16) -> Vec<Monitor> {
    let mut monitors = s2_monitors(LOW_DPI);
    monitors[1].dpi = u32::from(dpi);
    monitors
}

/// 窓の位置を直接置く（ドラッグの結果だけを再現する・書込は経ない）。
pub fn set_position(harness: &mut FrameHarness, window: Entity, position: Point) {
    let mut window_pos = harness
        .world
        .get_mut::<WindowPos>(window)
        .expect("WindowPos がある");
    window_pos.position = Some(position);
}

/// 当該スコープの二体をまとめて指定位置へ移す（随伴の窓相対を保ったまま）。
pub fn move_scope_to(harness: &mut FrameHarness, scope: usize, position: Point) {
    let char_window = harness.char_window(scope);
    let balloon = harness.balloon_window(scope);
    let offset = harness
        .world
        .get::<BalloonFollow>(char_window)
        .expect("char 窓は BalloonFollow を持つ")
        .offset();
    set_position(harness, char_window, position);
    set_position(
        harness,
        balloon,
        Point {
            x: position.x + offset.x,
            y: position.y + offset.y,
        },
    );
}

/// 拡大率通知だけが先に届いた状態を作る（実行時のモニタ表は旧水準のまま据え置き、4 窓
/// すべてへ当該水準の再表示報告を与える）。
pub fn deliver_the_scale_notice_first(
    harness: &mut FrameHarness,
    source: &mut FakeReports,
    dpi: u16,
) {
    harness.set_window_dpi(dpi);
    for (target, size) in [
        (shell_target(0).0, SPAWN_SIZE_0),
        (shell_target(1).0, SPAWN_SIZE_1),
        (balloon_target(0).0, BALLOON_SPAWN_SIZE),
        (balloon_target(1).0, BALLOON_SPAWN_SIZE),
    ] {
        source.refresh.insert(target, scaled(size, dpi));
    }
    harness.advance_frame();
    harness.run_placement_phases(source);
}

/// 当該スコープの 2 窓に整合待ちの札が付いているか（キャラ, バルーン）。
pub fn holds_of(harness: &FrameHarness, scope: usize) -> (bool, bool) {
    (
        harness
            .world
            .get::<DpiSyncHold>(harness.char_window(scope))
            .is_some(),
        harness
            .world
            .get::<DpiSyncHold>(harness.balloon_window(scope))
            .is_some(),
    )
}

/// 遷移のテストが**複数モニタ**の作業領域を注入した状態で走っていることを、源の中身で固定する。
///
/// 単一モニタへ退化すると作業領域の解決は「候補が 1 つしか無いから当たる」になり、帰属を
/// 通っているかどうかが観測できなくなる。それでも遷移のテストは緑のまま通るので、条件の側を
/// 別に固定する。
#[test]
fn the_transition_cases_run_against_a_multi_monitor_work_area_table() {
    for dpi in [LOW_DPI, SCALE_120, HIGH_DPI] {
        let mut harness = FrameHarness::new();
        let mut source = FakeReports::default();
        settle_at(&mut harness, &mut source, dpi);

        let source_areas = &harness
            .work_area_source()
            .expect("作業領域源がある")
            .work_areas;
        assert!(
            source_areas.len() >= 2,
            "dpi={dpi}: 作業領域源が複数モニタになっていない（帰属を通らない退化した観測条件）: {source_areas:?}"
        );
        assert_ne!(
            source_areas[0], source_areas[1],
            "dpi={dpi}: 2 つの作業領域が同一（隣接モニタが候補として効いていない）"
        );
        let table = harness.monitor_dpi_table().expect("モニタ別拡大率表がある");
        assert!(
            table.entries.len() >= 2,
            "dpi={dpi}: モニタ別拡大率表が複数モニタになっていない: {table:?}"
        );
    }
}
