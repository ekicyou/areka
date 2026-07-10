//! ECS 組立: キャラ窓／バルーン窓 entity spawn と公開データ構造（task 5.1）。
//!
//! 解決済み配置（[`ScopePlacement`]）から窓 entity を組み立て、識別子
//! （markers・[`GhostWindows`]）を後続（emo2-boot）へ公開する
//! （design「placement::spawn」・要件 1.1/1.2/1.5/4.1/4.5/5.1/6.1/6.2/6.3/7.2）。
//!
//! # 座標単位契約（design U1/U2）
//!
//! 位置・寸法は **`ScopePlacement` 由来の物理 px のみ**を `WindowPos` へ転記する。
//! `BoxStyle`（論理 DIP）と `DragConstraint` は一切付けない（U2・DD8。
//! 2026-07-05 の単位混在・単一モニタ誤釘付けの欠陥面そのものを消す）。
//! デモ由来の座標リテラル（`(400,200)`／`(335,0)` 等）はこのモジュールに
//! 存在しない（1.5・design「座標定数の禁止」）。
//!
//! # 窓 entity 構成（design「placement::spawn」の正本 bullet）
//!
//! - キャラ窓: `Name`＋`CharWindowMarker{scope}`＋`GhostWindowMarker`＋`Window{title}`
//!   ＋`WindowStyle { style: WS_POPUP|WS_VISIBLE, ex_style: WS_EX_LAYERED|WS_EX_TOOLWINDOW }`
//!   （**`WS_EX_TOPMOST` なし**・5.1／DD13）＋`WindowPos { position, size }`（物理 px）
//!   ＋`HitTest::none()`（全面ヒットで透過を殺さない）＋`DragConfig::default()`
//!   （move_window=true・全面ドラッグ・4.1）＋`OnDrag(on_char_drag)`＋`BalloonFollow`
//!   ＋`OnPointerPressed(on_ghost_pressed)`（ダブルクリックで全 `GhostWindowMarker`
//!   despawn→`run()` 正常復帰）
//! - バルーン窓: 同型（marker は `BalloonWindowMarker{scope}`・`DragConfig::default()`
//!   は付与＝バルーン単独ドラッグ可・4.5。`OnDrag` 追従ハンドラなし・`BalloonFollow` なし）
//!
//! clickthrough 登録 system（`register_ghost_windows_click_through`）は task 5.2 の領分。

use std::collections::BTreeMap;

use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::info;
use windows::Win32::UI::WindowsAndMessaging::{
    WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
};
use wintf::ecs::drag::{DragConfig, OnDrag};
use wintf::ecs::layout::HitTest;
use wintf::ecs::pointer::{DoubleClick, OnPointerPressed, Phase, PointerState};
use wintf::ecs::{Point, SizeI, Window, WindowPos, WindowStyle};

use super::follow::{on_char_drag, BalloonFollow};
use super::resolver::ScopePlacement;
use super::source::GhostTitles;

// ---------------------------------------------------------------------------
// 識別 markers（6.2）
// ---------------------------------------------------------------------------

/// スコープ別キャラ窓の識別 marker（6.2・補助的な逆引き。正本は [`GhostWindows`]）。
#[allow(dead_code)] // 結線（main.rs シーム）は task 6.2
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharWindowMarker {
    /// スコープ番号（0=本体・1=相方・…）。
    pub scope: usize,
}

/// スコープ別バルーン窓の識別 marker（6.2・補助的な逆引き。正本は [`GhostWindows`]）。
#[allow(dead_code)] // 結線（main.rs シーム）は task 6.2
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BalloonWindowMarker {
    /// スコープ番号（対応するキャラ窓と同じ番号）。
    pub scope: usize,
}

/// placement 生成窓の共通標識（smoke close・一括 despawn・clickthrough 登録の標的）。
#[allow(dead_code)] // 結線（main.rs シーム／task 5.2 clickthrough 登録）は後続タスク
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GhostWindowMarker;

// ---------------------------------------------------------------------------
// GhostWindows（後続 emo2-boot への引き渡し正本・6.1/6.2）
// ---------------------------------------------------------------------------

/// スコープ 1 体ぶんの窓 entity 対（キャラ窓＋バルーン窓）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScopeWindows {
    /// キャラ窓 entity。
    pub char_window: Entity,
    /// バルーン窓 entity。
    pub balloon_window: Entity,
}

/// 後続（emo2-boot）への引き渡し正本（6.1/6.2）。
///
/// 「スコープ×種別 → Entity」の唯一の正本（markers は補助的な逆引き）。
/// spawn 完了時に Resource 挿入＋戻り値の両方で公開される。窓 despawn 後の
/// Entity 無効化は M1 では追跡しない（emo2-boot は起動直後に読む前提・design
/// Revalidation Trigger）。
#[derive(Resource, Clone, Debug)]
pub struct GhostWindows {
    /// スコープ番号 → 窓 entity 対（非公開・アクセサ経由）。
    windows: BTreeMap<usize, ScopeWindows>,
}

#[allow(dead_code)] // 消費側（emo2-boot／main.rs シーム task 6.2）は後続
impl GhostWindows {
    /// スコープのキャラ窓 entity を返す（未知スコープは `None`・panic しない）。
    pub fn char_window(&self, scope: usize) -> Option<Entity> {
        self.windows.get(&scope).map(|w| w.char_window)
    }

    /// スコープのバルーン窓 entity を返す（未知スコープは `None`・panic しない）。
    pub fn balloon_window(&self, scope: usize) -> Option<Entity> {
        self.windows.get(&scope).map(|w| w.balloon_window)
    }

    /// 生成済みスコープ番号を昇順で列挙する（`BTreeMap` キー順）。
    pub fn scopes(&self) -> impl Iterator<Item = usize> + '_ {
        self.windows.keys().copied()
    }
}

// ---------------------------------------------------------------------------
// spawn_ghost_windows（bare World で動く組立・headless テスト可）
// ---------------------------------------------------------------------------

/// 解決済み配置からキャラ窓・バルーン窓 entity を組み立てる（design「placement::spawn」）。
///
/// bare `World` だけで動く（`spawn_dummy_window` と同型・headless テスト可）。
/// 位置・寸法は **`placements`（[`ScopePlacement`]・物理 px）由来のみ**を
/// `WindowPos` へ転記し、座標リテラルを一切持たない（1.5・U1）。
/// スコープごとにバルーン窓を先に spawn し（`BalloonFollow.balloon` が entity を
/// 要するため）、次にそのバルーンを参照するキャラ窓を spawn する。
///
/// 完了時に [`GhostWindows`] を Resource として挿入し、同じ内容を戻り値でも
/// 返す（6.1・Resource 挿入＋戻り値の両方で公開）。
#[allow(dead_code)] // 結線（main.rs シーム）は task 6.2
pub fn spawn_ghost_windows(
    world: &mut World,
    placements: &[ScopePlacement],
    titles: &GhostTitles,
) -> GhostWindows {
    let mut windows = BTreeMap::new();

    for p in placements {
        let title = titles.title(p.scope);

        // バルーン窓（design「窓 entity 構成（バルーン窓）」: キャラ窓と同型・
        // marker は BalloonWindowMarker・DragConfig::default() 付与＝単独ドラッグ可
        // （4.5）・OnDrag 追従ハンドラなし・BalloonFollow なし）
        let balloon_window = world
            .spawn((
                Name::new(format!("Ghost-Balloon-Window-{}", p.scope)),
                BalloonWindowMarker { scope: p.scope },
                GhostWindowMarker,
                Window {
                    title: title.to_string(),
                    ..Default::default()
                },
                window_style(),
                window_pos(p.balloon_pos.x, p.balloon_pos.y, p.balloon_size.w, p.balloon_size.h),
                HitTest::none(),
                DragConfig::default(),
                OnPointerPressed(on_ghost_pressed),
            ))
            .id();

        // キャラ窓（design「窓 entity 構成（キャラ窓）」: OnDrag(on_char_drag) で
        // バルーン追従（4.2）・BalloonFollow は ScopePlacement.balloon_offset の転写
        // （配置時 1 回だけ確定・4.4））
        let char_window = world
            .spawn((
                Name::new(format!("Ghost-Char-Window-{}", p.scope)),
                CharWindowMarker { scope: p.scope },
                GhostWindowMarker,
                Window {
                    title: title.to_string(),
                    ..Default::default()
                },
                window_style(),
                window_pos(p.char_pos.x, p.char_pos.y, p.char_size.w, p.char_size.h),
                HitTest::none(),
                DragConfig::default(),
                OnDrag(on_char_drag),
                BalloonFollow {
                    balloon: balloon_window,
                    offset: p.balloon_offset,
                },
                OnPointerPressed(on_ghost_pressed),
            ))
            .id();

        windows.insert(
            p.scope,
            ScopeWindows {
                char_window,
                balloon_window,
            },
        );
    }

    let ghost_windows = GhostWindows { windows };
    world.insert_resource(ghost_windows.clone());
    ghost_windows
}

/// 全ゴースト窓共通の `WindowStyle`（DD13: `WS_EX_TOPMOST` を含めない＝既定
/// z-order 非 topmost・5.1。`WS_EX_LAYERED` は clickthrough トグルの同伴フラグ）。
fn window_style() -> WindowStyle {
    WindowStyle {
        style: WS_POPUP | WS_VISIBLE,
        ex_style: WS_EX_LAYERED | WS_EX_TOOLWINDOW,
    }
}

/// `ScopePlacement` 由来の位置・寸法（物理 px）だけを転記した `WindowPos`（U1）。
fn window_pos(x: i32, y: i32, w: i32, h: i32) -> WindowPos {
    WindowPos {
        position: Some(Point { x, y }),
        size: Some(SizeI::new(w, h)),
        ..Default::default()
    }
}

/// `OnPointerPressed` ハンドラ: ダブルクリック（左）で全 [`GhostWindowMarker`]
/// 窓を despawn する（design main.rs seam: despawn → wintf の
/// `on_window_handle_remove` → `WM_CLOSE` → `DestroyWindow` → `WindowRegistry`
/// 空遷移 → `run()` 正常復帰。`spawn_dummy_window` の `on_dummy_pressed` と同じ作法）。
///
/// `Phase::Bubble` の `DoubleClick::Left` のみ処理して true を返す。それ以外
/// （他ボタン・`Phase::Tunnel`）は false（伝播続行）。
fn on_ghost_pressed(
    world: &mut World,
    _sender: Entity,
    _entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    match ev {
        Phase::Tunnel(_) => false,
        Phase::Bubble(state) => {
            if state.double_click == DoubleClick::Left {
                info!("ゴースト窓ダブルクリック検出 — 全ゴースト窓を閉じます");
                let targets: Vec<Entity> = world
                    .query_filtered::<Entity, With<GhostWindowMarker>>()
                    .iter(world)
                    .collect();
                for e in targets {
                    world.despawn(e);
                }
                return true;
            }
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::prelude::*;
    use windows::Win32::UI::WindowsAndMessaging::{
        WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
    };
    use wintf::ecs::drag::{DragConfig, DragConstraint, OnDrag};
    use wintf::ecs::layout::{BoxStyle, HitTest};
    use wintf::ecs::pointer::{DoubleClick, OnPointerPressed, Phase, PointerState};
    use wintf::ecs::{Point, SizeI, Window, WindowPos, WindowStyle};

    use super::{
        BalloonWindowMarker, CharWindowMarker, GhostWindowMarker, GhostWindows,
        spawn_ghost_windows,
    };
    use crate::placement::follow::BalloonFollow;
    use crate::placement::resolver::{PointPx, ScopePlacement, SizePx};
    use crate::placement::source::GhostTitles;

    // -------------------------------------------------------------------------
    // テストヘルパ（bare World・emo2 相当 2 スコープ。値は resolver 出力を模した
    // 合成値で、96 の倍数を避けて隠れた dpi 再スケールがあれば一致が崩れる檻とする。
    // 恒等式 balloon_offset ≡ balloon_pos − char_pos を満たすように構築する）
    // -------------------------------------------------------------------------

    /// scope0/scope1 の 2 スコープぶんの解決済み配置（emo2 相当の形）。
    fn two_scope_placements() -> Vec<ScopePlacement> {
        vec![
            ScopePlacement {
                scope: 0,
                char_pos: PointPx { x: 1483, y: 733 },
                char_size: SizePx { w: 434, h: 687 },
                balloon_pos: PointPx { x: 1071, y: 708 },
                balloon_size: SizePx { w: 223, h: 158 },
                balloon_offset: PointPx { x: -412, y: -25 },
            },
            ScopePlacement {
                scope: 1,
                char_pos: PointPx { x: 1049, y: 1063 },
                char_size: SizePx { w: 278, h: 357 },
                balloon_pos: PointPx { x: 1334, y: 1044 },
                balloon_size: SizePx { w: 223, h: 158 },
                balloon_offset: PointPx { x: 285, y: -19 },
            },
        ]
    }

    fn titles() -> GhostTitles {
        GhostTitles::from_scope_titles([(0, "むらさき".to_string()), (1, "エモ".to_string())])
    }

    /// 全 GhostWindowMarker 窓 entity を収集する。
    fn ghost_window_entities(world: &mut World) -> Vec<Entity> {
        world
            .query_filtered::<Entity, With<GhostWindowMarker>>()
            .iter(world)
            .collect()
    }

    fn pressed_event(double_click: DoubleClick) -> PointerState {
        PointerState {
            double_click,
            ..Default::default()
        }
    }

    // -------------------------------------------------------------------------
    // T-I1: spawn 組立（6.1/6.2・1.1/1.2）
    // -------------------------------------------------------------------------

    /// T-I1: bare World で 2 スコープぶん spawn → 窓 4 entity・markers 正値・
    /// `GhostWindows` の scope×種別引き当てが `ScopePlacement` と一致する。
    #[test]
    fn t_i1_spawn_assembles_four_windows_with_markers_and_ghost_windows() {
        let mut world = World::new();
        let placements = two_scope_placements();

        let gw = spawn_ghost_windows(&mut world, &placements, &titles());

        // 窓 4 entity（2 スコープ × キャラ/バルーン・1.1/1.2）
        assert_eq!(ghost_window_entities(&mut world).len(), 4);

        // scopes() はスコープ番号を昇順で列挙する
        assert_eq!(gw.scopes().collect::<Vec<_>>(), vec![0, 1]);

        for p in &placements {
            // GhostWindows の scope×種別引き当て（6.1/6.2）
            let char_e = gw.char_window(p.scope).expect("char window entity");
            let balloon_e = gw.balloon_window(p.scope).expect("balloon window entity");
            assert_ne!(char_e, balloon_e);

            // markers 正値（補助的な逆引き・6.2）
            assert_eq!(
                world.get::<CharWindowMarker>(char_e).map(|m| m.scope),
                Some(p.scope)
            );
            assert!(world.get::<BalloonWindowMarker>(char_e).is_none());
            assert_eq!(
                world.get::<BalloonWindowMarker>(balloon_e).map(|m| m.scope),
                Some(p.scope)
            );
            assert!(world.get::<CharWindowMarker>(balloon_e).is_none());
            assert!(world.get::<GhostWindowMarker>(char_e).is_some());
            assert!(world.get::<GhostWindowMarker>(balloon_e).is_some());

            // WindowPos が ScopePlacement と一致（物理 px 転記・U1）
            let char_pos = world.get::<WindowPos>(char_e).expect("char WindowPos");
            assert_eq!(
                char_pos.position,
                Some(Point {
                    x: p.char_pos.x,
                    y: p.char_pos.y
                })
            );
            assert_eq!(
                char_pos.size,
                Some(SizeI::new(p.char_size.w, p.char_size.h))
            );
            let balloon_pos = world.get::<WindowPos>(balloon_e).expect("balloon WindowPos");
            assert_eq!(
                balloon_pos.position,
                Some(Point {
                    x: p.balloon_pos.x,
                    y: p.balloon_pos.y
                })
            );
            assert_eq!(
                balloon_pos.size,
                Some(SizeI::new(p.balloon_size.w, p.balloon_size.h))
            );
        }

        // 未知スコープは None（panic しない）
        assert_eq!(gw.char_window(99), None);
        assert_eq!(gw.balloon_window(99), None);
    }

    /// T-I1 補: `GhostWindows` は Resource としても挿入され、戻り値と同じ
    /// 引き当てを返す（design「Resource 挿入＋戻り値の両方で公開」）。
    #[test]
    fn t_i1_ghost_windows_resource_matches_return_value() {
        let mut world = World::new();
        let placements = two_scope_placements();

        let returned = spawn_ghost_windows(&mut world, &placements, &titles());

        let resource = world
            .get_resource::<GhostWindows>()
            .expect("GhostWindows Resource が挿入されているはず");
        assert_eq!(
            resource.scopes().collect::<Vec<_>>(),
            returned.scopes().collect::<Vec<_>>()
        );
        for scope in returned.scopes() {
            assert_eq!(resource.char_window(scope), returned.char_window(scope));
            assert_eq!(resource.balloon_window(scope), returned.balloon_window(scope));
        }
    }

    /// T-I1 補: 窓タイトルは `GhostTitles` 由来（欠落スコープは既定 "areka"）。
    #[test]
    fn t_i1_window_titles_come_from_ghost_titles() {
        let mut world = World::new();
        let placements = two_scope_placements();

        let gw = spawn_ghost_windows(&mut world, &placements, &titles());

        let char0 = gw.char_window(0).unwrap();
        let balloon1 = gw.balloon_window(1).unwrap();
        assert_eq!(world.get::<Window>(char0).unwrap().title, "むらさき");
        assert_eq!(world.get::<Window>(balloon1).unwrap().title, "エモ");

        // タイトル欠落スコープは既定文字列（GhostTitles::title の契約）
        let mut world2 = World::new();
        let gw2 = spawn_ghost_windows(
            &mut world2,
            &placements,
            &GhostTitles::from_scope_titles([]),
        );
        let char0 = gw2.char_window(0).unwrap();
        assert_eq!(world2.get::<Window>(char0).unwrap().title, "areka");
    }

    /// T-I1 補: キャラ窓は `BalloonFollow`（balloon 引き当て＋offset 転写）と
    /// `OnDrag` を持ち、バルーン窓はどちらも持たない（4.2 結線・design 同型 bullet）。
    #[test]
    fn t_i1_char_window_has_follow_and_on_drag_balloon_has_neither() {
        let mut world = World::new();
        let placements = two_scope_placements();

        let gw = spawn_ghost_windows(&mut world, &placements, &titles());

        for p in &placements {
            let char_e = gw.char_window(p.scope).unwrap();
            let balloon_e = gw.balloon_window(p.scope).unwrap();

            // BalloonFollow: 対応バルーン entity＋ScopePlacement.balloon_offset の転写
            let follow = world
                .get::<BalloonFollow>(char_e)
                .expect("char window BalloonFollow");
            assert_eq!(follow.balloon, balloon_e);
            assert_eq!(follow.offset, p.balloon_offset);
            assert!(world.get::<OnDrag>(char_e).is_some());

            // バルーン窓には追従ハンドラも BalloonFollow も付けない
            assert!(world.get::<BalloonFollow>(balloon_e).is_none());
            assert!(world.get::<OnDrag>(balloon_e).is_none());
        }
    }

    /// T-I1 補: 全窓が `HitTest::none()`（全面ヒットで透過を殺さない）と
    /// `OnPointerPressed`（ダブルクリック close）を持つ。
    #[test]
    fn t_i1_all_windows_have_hit_test_none_and_pointer_pressed() {
        let mut world = World::new();
        let placements = two_scope_placements();

        spawn_ghost_windows(&mut world, &placements, &titles());

        for e in ghost_window_entities(&mut world) {
            assert_eq!(world.get::<HitTest>(e).copied(), Some(HitTest::none()));
            assert!(world.get::<OnPointerPressed>(e).is_some());
        }
    }

    // -------------------------------------------------------------------------
    // T-I2: z-order 既定（5.1・DD13）
    // -------------------------------------------------------------------------

    /// T-I2: 全窓の `WindowStyle.ex_style` に `WS_EX_TOPMOST` が含まれない
    /// （既定 z-order 非 topmost・5.1／DD13。style/ex_style の正値も固定する）。
    #[test]
    fn t_i2_no_window_has_ws_ex_topmost() {
        let mut world = World::new();
        let placements = two_scope_placements();

        spawn_ghost_windows(&mut world, &placements, &titles());

        let entities = ghost_window_entities(&mut world);
        assert_eq!(entities.len(), 4);
        for e in entities {
            let style = world.get::<WindowStyle>(e).expect("WindowStyle");
            assert!(
                !style.ex_style.contains(WS_EX_TOPMOST),
                "WS_EX_TOPMOST が含まれてはならない（5.1／DD13）: {:?}",
                style.ex_style
            );
            assert_eq!(style.ex_style, WS_EX_LAYERED | WS_EX_TOOLWINDOW);
            assert_eq!(style.style, WS_POPUP | WS_VISIBLE);
        }
    }

    // -------------------------------------------------------------------------
    // T-I3: 単位契約（U2・DD8・4.1/4.5）
    // -------------------------------------------------------------------------

    /// T-I3: 窓 entity に `BoxStyle` 不在（U2・論理 DIP を持ち込まない）・
    /// `DragConstraint` 不在（DD8・全モニタドラッグ可・4.5）・
    /// `DragConfig.move_window=true`（全面ドラッグ・4.1）。
    #[test]
    fn t_i3_no_box_style_no_drag_constraint_and_move_window_true() {
        let mut world = World::new();
        let placements = two_scope_placements();

        spawn_ghost_windows(&mut world, &placements, &titles());

        let entities = ghost_window_entities(&mut world);
        assert_eq!(entities.len(), 4);
        for e in entities {
            assert!(
                world.get::<BoxStyle>(e).is_none(),
                "窓 entity に BoxStyle を付けてはならない（U2）"
            );
            assert!(
                world.get::<DragConstraint>(e).is_none(),
                "窓 entity に DragConstraint を付けてはならない（DD8・4.5）"
            );
            let drag = world.get::<DragConfig>(e).expect("DragConfig");
            assert!(drag.move_window, "DragConfig.move_window=true（4.1/4.5）");
        }
    }

    // -------------------------------------------------------------------------
    // OnPointerPressed close（design main.rs seam: ダブルクリック → 全
    // GhostWindowMarker despawn → run() 正常復帰）
    // -------------------------------------------------------------------------

    /// ダブルクリック（左）の Bubble で全 `GhostWindowMarker` 窓を despawn し
    /// true を返す。マーカーを持たない entity は残す。
    #[test]
    fn double_click_left_despawns_all_ghost_windows() {
        let mut world = World::new();
        let placements = two_scope_placements();
        let gw = spawn_ghost_windows(&mut world, &placements, &titles());
        let other = world.spawn_empty().id();

        let char0 = gw.char_window(0).unwrap();
        let handler = world
            .get::<OnPointerPressed>(char0)
            .expect("OnPointerPressed")
            .0;

        let ev = Phase::Bubble(pressed_event(DoubleClick::Left));
        assert!(handler(&mut world, char0, char0, &ev));

        assert!(ghost_window_entities(&mut world).is_empty());
        assert!(world.get_entity(other).is_ok());
    }

    /// 左以外のダブルクリック・Tunnel フェーズでは despawn しない（false）。
    #[test]
    fn non_left_double_click_and_tunnel_do_not_despawn_ghost_windows() {
        let mut world = World::new();
        let placements = two_scope_placements();
        let gw = spawn_ghost_windows(&mut world, &placements, &titles());

        let balloon1 = gw.balloon_window(1).unwrap();
        let handler = world
            .get::<OnPointerPressed>(balloon1)
            .expect("OnPointerPressed")
            .0;

        for dc in [DoubleClick::None, DoubleClick::Right, DoubleClick::Middle] {
            let ev = Phase::Bubble(pressed_event(dc));
            assert!(!handler(&mut world, balloon1, balloon1, &ev));
        }
        let ev = Phase::Tunnel(pressed_event(DoubleClick::Left));
        assert!(!handler(&mut world, balloon1, balloon1, &ev));

        assert_eq!(ghost_window_entities(&mut world).len(), 4);
    }
}
