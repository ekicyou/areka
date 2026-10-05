//! 当たり判定の入口が、子から持ち込んだ領域の列で判定することの檻
//! （spec: areka-P0-surface-element-nesting・タスク 6.2・要件 2.8・4.1・4.2）。
//!
//! 入れ子の検体（`areka-emo-compose/tests/fixtures/surface-nesting/`）の親 B（surface1）は自分の
//! 領域を持たず、子 10 を (90,50) に置く。子 10 の Head（0,0〜20,10）と Eye（5,0〜15,10・後ろ＝手前）は
//! 親の (90,50)〜(110,60) と (95,50)〜(105,60) へ持ち込まれる。転記のままの列（`SurfaceMaster.collisions`）
//! で判定していれば、親 B ではどの点も `None` になる。
//!
//! 親 A（surface0）は Eye が親の Head の下に隠れるので点の確かめには使わない（tasks.md 5.2 の覚え書き）。
//! GPU に触れない（`attach_target` は登録だけ・表示中の面と実適用 k は私有状態を直に書く）。

use super::super::test_support::{force_applied, force_current_surface, spawn_window_with_dpi};
use super::super::{AtlasTable, EmoPresenter, EmoWorld, ScaleRatio, TargetId, World};

use areka_emo_atlas::{MemoryDecoder, PackConfig, bake};
use areka_parsers::shell::parse;

const FIXTURE: &str =
    include_str!("../../areka-emo-compose/tests/fixtures/surface-nesting/surfaces.txt");

/// 検体の親 B（surface1）を表示中にし、実適用 k を与えた presenter。
fn presenter_showing_parent_b(k: ScaleRatio) -> EmoPresenter {
    let mut world = World::new();
    let window = spawn_window_with_dpi(&mut world, 96);
    // 判定は画素を見ないので atlas は空でよい（画像の読み込みも GPU も要らない）。
    let atlas: AtlasTable = bake(&[], &MemoryDecoder::new(), PackConfig::default()).table;
    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(
            &mut world,
            TargetId(0),
            window,
            EmoWorld::build(&parse(FIXTURE)),
            atlas,
            96,
        )
        .expect("attach_target 失敗");
    force_current_surface(&mut presenter, TargetId(0), 1);
    force_applied(&mut presenter, TargetId(0), Some(k));
    presenter
}

/// 拡大率 1: 窓の座標＝サーフェスの座標。子の Eye・Head が子を置いた位置で引ける。
/// 画像の座標の入口（`hit_region`）も同じ列で判定する。
#[test]
fn client_hit_returns_child_region_names_at_scale_1() {
    let presenter = presenter_showing_parent_b(ScaleRatio::ONE);
    for (x, y, want) in [
        (100, 55, Some("Eye")), // Eye と Head の重なり（Eye が後ろ＝手前）
        (92, 55, Some("Head")), // Head だけ
        (5, 5, None),           // 親 B は自分の領域を持たない
    ] {
        let hit = presenter.hit_region_client(TargetId(0), x, y);
        assert_eq!(hit.region, want, "client=({x},{y})");
        assert_eq!(hit.surface_point, (x, y), "k=1 は素通し");
        assert_eq!(
            presenter.hit_region(TargetId(0), x, y),
            want,
            "native=({x},{y})"
        );
    }
}

/// 拡大率 2: 窓の座標を ÷2 してから持ち込み済みの列で判定する。
/// 期待値は定数（縮約は画素中心逆写像 `floor((2v+1)/4)`: 200→100・110→55・184→92・10→5）。
#[test]
fn client_hit_returns_child_region_names_at_scale_2() {
    let k2 = ScaleRatio::new(2, 1).expect("2/1 は構築可能");
    let presenter = presenter_showing_parent_b(k2);
    for (cx, cy, want, point) in [
        (200, 110, Some("Eye"), (100, 55)),
        (184, 110, Some("Head"), (92, 55)),
        (10, 10, None, (5, 5)),
    ] {
        let hit = presenter.hit_region_client(TargetId(0), cx, cy);
        assert_eq!(hit.region, want, "client=({cx},{cy})");
        assert_eq!(hit.surface_point, point, "client=({cx},{cy})");
    }
}
