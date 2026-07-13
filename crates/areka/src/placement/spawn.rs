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
//!   ＋`HitTest::none()`（全面ヒットで透過を殺さない）＋`DragConfig`（全面ドラッグ・
//!   4.1。`move_window` は BottomSnap キャラ窓のみ false＝on_char_drag 単一ライター・
//!   DD15 v2／4.7、Free は true＝wndproc 委譲）＋`OnDrag(on_char_drag)`＋`BalloonFollow`
//!   ＋`OnPointerPressed(on_ghost_pressed)`（ダブルクリックで全 `GhostWindowMarker`
//!   despawn→`run()` 正常復帰）。BottomSnap キャラ窓はさらに `BottomSnap` marker＋
//!   `OnDragEnd(on_char_drag_end)`（最終カーソル位置への同写像適用・DD15 v2 (3)）
//! - バルーン窓: 同型（marker は `BalloonWindowMarker{scope}`・`DragConfig::default()`
//!   は付与＝バルーン単独ドラッグ可・4.5。`OnDrag(on_balloon_drag)` で単独ドラッグの
//!   相対位置記憶（4.8・DD16・task 8.3）・`BalloonFollow` なし）
//!
//! # clickthrough 登録（task 5.2）
//!
//! [`register_ghost_windows_click_through`] が `Added<WindowHandle>` で
//! [`GhostWindowMarker`] 窓を αマスク clickthrough 機構
//! （wintf `ClickThroughRegistryHandle`・消費のみ）へ登録する
//! （emo-present donor `register_click_through_windows` の一般化・6.1）。

use std::collections::BTreeMap;

use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::{debug, info};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
};
use wintf::ecs::clickthrough::ClickThroughRegistryHandle;
use wintf::ecs::drag::{DragConfig, OnDrag, OnDragEnd};
use wintf::ecs::layout::HitTest;
use wintf::ecs::pointer::{DoubleClick, OnPointerPressed, Phase, PointerState};
use wintf::ecs::{Point, SizeI, Window, WindowHandle, WindowPos, WindowStyle};

use super::follow::{on_balloon_drag, on_char_drag, on_char_drag_end, BalloonFollow};
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
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GhostWindowMarker;

/// bottom 吸着スコープのキャラ窓 marker（4.7・DD15 v2 基盤・task 8.1/8.2R）。
///
/// `ScopePlacement.anchor` が非 Free（`!anchor.is_free()`）のスコープの転写。
/// この marker が bottom 吸着の単一の真実源: spawn は `DragConfig.move_window=false`
/// と `OnDragEnd` の付与を連動させ、`on_char_drag`／`on_char_drag_end` は marker の
/// 有無で `BottomSnapPolicy`（トレイト実装）を静的に引く（policy を trait object と
/// して entity に持たせる案は marker との二重管理・非 Clone boxed Component の扱い
/// 難と引き換えのため見送り——実装が増えたら component 化を再検討）。吸着対象は
/// キャラ窓のみ＝バルーン窓には anchor 値によらず付けない（DD15・4.8）。
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BottomSnap;

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
        // （4.5）・OnDrag(on_balloon_drag) で単独ドラッグの相対位置記憶
        // （4.8・DD16・task 8.3）・BalloonFollow なし）
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
                OnDrag(on_balloon_drag),
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
                // 非 Free アンカー（Bottom/Top/Left/Right）のキャラ窓は
                // move_window=false＝wndproc は窓を動かさず on_char_drag が単一ライター
                // （DD15 v2・4.7・task 8.2R）。Free は従来どおり wndproc 直接移動（4.1）。
                // 二値吸着フラグは持たず anchor 単一値から導出する（Req1.6）。
                // threshold 等は既定を保つ。
                DragConfig {
                    move_window: p.anchor.is_free(),
                    ..Default::default()
                },
                OnDrag(on_char_drag),
                BalloonFollow {
                    balloon: balloon_window,
                    offset: p.balloon_offset,
                },
                OnPointerPressed(on_ghost_pressed),
            ))
            .id();

        // bottom 吸着の情報伝搬（4.7・task 8.1/8.2R）: 非 Free アンカー
        // （Bottom/Top/Left/Right）スコープのキャラ窓のみ BottomSnap marker
        // （on_char_drag の単一ライター経路の標的）と OnDragEnd（最終 DragEvent 欠落の
        // 穴埋め・DD15 v2 (3)）を付ける。判定は anchor 単一値から導出（Req1.6）。
        if !p.anchor.is_free() {
            world
                .entity_mut(char_window)
                .insert((BottomSnap, OnDragEnd(on_char_drag_end)));
        }

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

// ---------------------------------------------------------------------------
// clickthrough 登録 system（task 5.2・6.1）
// ---------------------------------------------------------------------------

/// clickthrough 登録面の偽装境界（fake boundary）シーム。
///
/// 実体は wintf の [`ClickThroughRegistryHandle`]（NonSend・`WinApp::run` の
/// 結線で挿入）だが、その constructor は wintf 内部（pub(crate)）で headless
/// テストから構築できない。登録呼び出しの決定論的観測のため、登録面をこの
/// trait で抽象し、テストは偽 registrar（呼び出し記録）を NonSend として
/// 挿し込む（本 repo の偽装境界パターン）。
trait ClickThroughRegistrar: 'static {
    /// 監視対象窓（window Entity ＋ HWND）を登録する。
    fn register_window(&self, window: Entity, hwnd: HWND);
}

impl ClickThroughRegistrar for ClickThroughRegistryHandle {
    fn register_window(&self, window: Entity, hwnd: HWND) {
        self.register(window, hwnd);
    }
}

/// `Added<WindowHandle>` で [`GhostWindowMarker`] 窓を αマスク clickthrough
/// 機構へ登録する system（design「placement::spawn」正本 signature・6.1。
/// emo-present donor `register_click_through_windows` の一般化）。
///
/// WUC 化により ULW の自動 α ヒットテストが失われるため、機構が α を評価
/// できるよう placement 生成窓（キャラ窓・バルーン窓）を明示登録する。
/// `WindowHandle` は wintf の窓生成が HWND 生成後に付与するため
/// `Added<WindowHandle>` で「HWND が付いた瞬間」を捉え、各窓を厳密に 1 回
/// 登録する（`register` は同一 Entity 再登録を dedupe するため冪等でもある）。
/// `ClickThroughRegistryHandle` は `WinApp::run` の結線で NonSend リソース
/// として挿入される。ごく初期の tick で未挿入の可能性へ `Option` で防御する
/// （headless でも no-op で安全）。schedule への結線は main.rs シーム
/// `open_startup_window`（task 6.2）が `FrameFinalize` へ行う。
pub fn register_ghost_windows_click_through(
    new_windows: Query<(Entity, &WindowHandle), (With<GhostWindowMarker>, Added<WindowHandle>)>,
    handle: Option<NonSend<ClickThroughRegistryHandle>>,
) {
    register_ghost_windows_via(new_windows, handle);
}

/// [`register_ghost_windows_click_through`] の汎用実装（偽装境界）。
///
/// query filter（`GhostWindowMarker` × `Added<WindowHandle>`）ごとこの system
/// が production 経路の正体であり、公開 system は実 registrar 型
/// （[`ClickThroughRegistryHandle`]）を束縛した thin wrapper。型が一致しない
/// と wrapper が compile できないため、filter の乖離は型システムが防ぐ。
fn register_ghost_windows_via<R: ClickThroughRegistrar>(
    new_windows: Query<(Entity, &WindowHandle), (With<GhostWindowMarker>, Added<WindowHandle>)>,
    handle: Option<NonSend<R>>,
) {
    let Some(handle) = handle else {
        return;
    };
    for (entity, wh) in new_windows.iter() {
        handle.register_window(entity, wh.hwnd);
        debug!(?entity, "placement: クリック透過機構へゴースト窓を登録");
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
    use wintf::ecs::drag::{DragConfig, DragConstraint, OnDrag, OnDragEnd};
    use wintf::ecs::layout::{BoxStyle, HitTest};
    use wintf::ecs::pointer::{DoubleClick, OnPointerPressed, Phase, PointerState};
    use wintf::ecs::{Point, SizeI, Window, WindowPos, WindowStyle};

    use super::{
        BalloonWindowMarker, BottomSnap, CharWindowMarker, GhostWindowMarker, GhostWindows,
        spawn_ghost_windows,
    };
    use crate::placement::follow::BalloonFollow;
    use crate::placement::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
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
                anchor: Anchor::Bottom, // emo2＝alignmenttodesktop,bottom
            },
            ScopePlacement {
                scope: 1,
                char_pos: PointPx { x: 1049, y: 1063 },
                char_size: SizePx { w: 278, h: 357 },
                balloon_pos: PointPx { x: 1334, y: 1044 },
                balloon_size: SizePx { w: 223, h: 158 },
                balloon_offset: PointPx { x: 285, y: -19 },
                anchor: Anchor::Bottom, // emo2＝alignmenttodesktop,bottom
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
    /// `OnDrag`（追従）を持ち、バルーン窓は `OnDrag`（相対位置記憶・4.8/DD16）を
    /// 持つが `BalloonFollow` は持たない（4.2 結線・design 同型 bullet。
    /// バルーン側ハンドラの実挙動は
    /// `t_i4_char_move_follows_adjusted_offset_after_balloon_solo_drag` が檻）。
    #[test]
    fn t_i1_char_window_has_follow_and_on_drag_balloon_has_on_drag_only() {
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

            // バルーン窓: 相対位置記憶ハンドラあり（4.8）・BalloonFollow なし
            assert!(world.get::<BalloonFollow>(balloon_e).is_none());
            assert!(world.get::<OnDrag>(balloon_e).is_some());
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
    // anchor 伝搬（4.2・DD15 基盤・task 8.1）
    // -------------------------------------------------------------------------

    /// 8.1: `placement.anchor` が非 Free のスコープの**キャラ窓にのみ** `BottomSnap`
    /// marker が付き、`Anchor::Free` のキャラ窓・バルーン窓（anchor 不問）には付かない
    /// （吸着対象はキャラ窓のみ・DD15）。
    #[test]
    fn bottom_snap_marker_attached_to_snapping_char_windows_only() {
        let mut world = World::new();
        let mut placements = two_scope_placements(); // 両方 Anchor::Bottom（emo2＝bottom）
        placements[1].anchor = Anchor::Free; // scope1 を非吸着（Free）へ

        let gw = spawn_ghost_windows(&mut world, &placements, &titles());

        let char0 = gw.char_window(0).unwrap();
        let char1 = gw.char_window(1).unwrap();
        assert!(
            world.get::<BottomSnap>(char0).is_some(),
            "非 Free アンカーのキャラ窓には BottomSnap が付く"
        );
        assert!(
            world.get::<BottomSnap>(char1).is_none(),
            "Anchor::Free のキャラ窓には付かない"
        );
        for scope in [0usize, 1] {
            assert!(
                world
                    .get::<BottomSnap>(gw.balloon_window(scope).unwrap())
                    .is_none(),
                "scope{scope}: バルーン窓には anchor 値によらず付かない"
            );
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

    /// T-I3（8.2R 改訂）: 窓 entity に `BoxStyle` 不在（U2・論理 DIP を持ち込まない）・
    /// `DragConstraint` 不在（DD8・全モニタドラッグ可・4.5）。`DragConfig.move_window`
    /// は「BottomSnap キャラ窓のみ false（単一ライター・DD15 v2・4.7）、Free キャラ窓
    /// とバルーン窓は true（wndproc 委譲・4.1/4.5）」。DragEnd 最終適用ハンドラ
    /// （`OnDragEnd`）は BottomSnap キャラ窓にのみ付く。
    #[test]
    fn t_i3_no_box_style_no_drag_constraint_and_move_window_contract() {
        let mut world = World::new();
        let mut placements = two_scope_placements();
        placements[1].anchor = Anchor::Free; // scope1 を Free へ（両変種を 1 テストで檻化）

        let gw = spawn_ghost_windows(&mut world, &placements, &titles());

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
            assert_eq!(drag.threshold, 5, "threshold は既定値を保つ");
            assert!(drag.enabled, "ドラッグは有効");
        }

        let snap_char = gw.char_window(0).unwrap();
        let free_char = gw.char_window(1).unwrap();
        assert!(
            !world.get::<DragConfig>(snap_char).unwrap().move_window,
            "BottomSnap キャラ窓は move_window=false（単一ライター・DD15 v2・4.7）"
        );
        assert!(
            world.get::<OnDragEnd>(snap_char).is_some(),
            "BottomSnap キャラ窓に DragEnd 最終適用ハンドラが付く（DD15 v2 (3)）"
        );
        assert!(
            world.get::<DragConfig>(free_char).unwrap().move_window,
            "Free キャラ窓は move_window=true（wndproc 委譲・4.1）"
        );
        assert!(
            world.get::<OnDragEnd>(free_char).is_none(),
            "Free キャラ窓に OnDragEnd は付けない"
        );
        for scope in [0usize, 1] {
            let balloon = gw.balloon_window(scope).unwrap();
            assert!(
                world.get::<DragConfig>(balloon).unwrap().move_window,
                "scope{scope}: バルーン窓は move_window=true（単独ドラッグ・4.5）"
            );
            assert!(
                world.get::<OnDragEnd>(balloon).is_none(),
                "scope{scope}: バルーン窓に OnDragEnd は付けない"
            );
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

    // -------------------------------------------------------------------------
    // T-I4: clickthrough 登録 system（6.1・task 5.2）
    //
    // 実 `ClickThroughRegistryHandle` は wintf 内部（`new` は pub(crate)）でしか
    // 構築できないため、headless の「登録呼び出しが発生する」観測は偽装境界
    // （`ClickThroughRegistrar` を `FakeRegistrar` へ差し替え）で行う。汎用実装
    // `register_ghost_windows_via` が本体の query filter（GhostWindowMarker ×
    // Added<WindowHandle>）ごと system として走る＝production 経路そのもの。
    // -------------------------------------------------------------------------

    use std::cell::RefCell;
    use windows::Win32::Foundation::{HINSTANCE, HWND};
    use wintf::ecs::WindowHandle;

    use super::{
        ClickThroughRegistrar, register_ghost_windows_click_through, register_ghost_windows_via,
    };

    /// 登録呼び出しを記録する偽 registrar（NonSend リソースとして挿入）。
    #[derive(Default)]
    struct FakeRegistrar {
        calls: RefCell<Vec<(Entity, isize)>>,
    }

    impl ClickThroughRegistrar for FakeRegistrar {
        fn register_window(&self, window: Entity, hwnd: HWND) {
            self.calls.borrow_mut().push((window, hwnd.0 as isize));
        }
    }

    /// 偽 HWND を持つ `WindowHandle`（4.2 と同じ fake WindowHandle パターン）。
    fn fake_window_handle(raw: isize) -> WindowHandle {
        WindowHandle {
            hwnd: HWND(raw as *mut core::ffi::c_void),
            instance: HINSTANCE::default(),
        }
    }

    fn registrar_calls(world: &World) -> Vec<(Entity, isize)> {
        world
            .non_send_resource::<FakeRegistrar>()
            .calls
            .borrow()
            .clone()
    }

    fn register_schedule() -> Schedule {
        let mut schedule = Schedule::default();
        schedule.add_systems(register_ghost_windows_via::<FakeRegistrar>);
        schedule
    }

    /// T-I4: `GhostWindowMarker` 窓に `WindowHandle` が付いた瞬間（Added）だけ
    /// 登録呼び出しが発生し、(Entity, HWND) が正値・再実行で重複登録しない・
    /// 後から HWND が付いた窓も追加で 1 回だけ登録される。
    #[test]
    fn t_i4_register_system_registers_ghost_windows_on_added_window_handle_once() {
        let mut world = World::new();
        let placements = two_scope_placements();
        let gw = spawn_ghost_windows(&mut world, &placements, &titles());
        world.insert_non_send_resource(FakeRegistrar::default());
        let mut schedule = register_schedule();

        // spawn 直後は WindowHandle 不在 → 登録は起きない
        schedule.run(&mut world);
        assert!(registrar_calls(&world).is_empty());

        // scope0 の 2 窓へ HWND 付与（wintf create_windows が付ける状況の模擬）
        let char0 = gw.char_window(0).unwrap();
        let balloon0 = gw.balloon_window(0).unwrap();
        world.entity_mut(char0).insert(fake_window_handle(0x10));
        world.entity_mut(balloon0).insert(fake_window_handle(0x20));

        schedule.run(&mut world);
        let mut calls = registrar_calls(&world);
        calls.sort_by_key(|(_, hwnd)| *hwnd);
        assert_eq!(calls, vec![(char0, 0x10), (balloon0, 0x20)]);

        // 再実行しても重複登録しない（Added は厳密 1 回）
        schedule.run(&mut world);
        assert_eq!(registrar_calls(&world).len(), 2);

        // 後から HWND が付いた scope1 キャラ窓も追加で 1 回だけ登録される
        let char1 = gw.char_window(1).unwrap();
        world.entity_mut(char1).insert(fake_window_handle(0x30));
        schedule.run(&mut world);
        let calls = registrar_calls(&world);
        assert_eq!(calls.len(), 3);
        assert!(calls.contains(&(char1, 0x30)));
    }

    /// T-I4 補: `GhostWindowMarker` を持たない窓は `WindowHandle` が付いても
    /// 登録されない（標的は placement 生成窓のみ・6.1）。
    #[test]
    fn t_i4_register_system_ignores_non_ghost_windows() {
        let mut world = World::new();
        world.insert_non_send_resource(FakeRegistrar::default());
        let mut schedule = register_schedule();

        world.spawn((Window::default(), fake_window_handle(0x40)));

        schedule.run(&mut world);
        assert!(registrar_calls(&world).is_empty());
    }

    /// T-I4 補: 実 system（design 正本 signature）は
    /// `ClickThroughRegistryHandle` 未挿入の headless World で no-op（panic
    /// しない・ごく初期 tick の未挿入への Option 防御＝donor と同じ作法）。
    #[test]
    fn t_i4_real_register_system_is_noop_without_registry_resource() {
        let mut world = World::new();
        let placements = two_scope_placements();
        let gw = spawn_ghost_windows(&mut world, &placements, &titles());
        let char0 = gw.char_window(0).unwrap();
        world.entity_mut(char0).insert(fake_window_handle(0x50));

        let mut schedule = Schedule::default();
        schedule.add_systems(register_ghost_windows_click_through);
        schedule.run(&mut world);

        // no-op で完走（窓はそのまま）
        assert_eq!(ghost_window_entities(&mut world).len(), 4);
    }

    // -------------------------------------------------------------------------
    // T-I4: follow 幾何（task 5.3・design Testing Strategy 4・要件 4.2）
    //
    // 実パイプライン統合: `build_placement_config`（KV 実経路）→
    // `resolve_placement`（非 96 倍数の合成 work_area・原点非 (0,0)）→
    // `spawn_ghost_windows` → 偽 WindowHandle 付与 → `move_window_to`。
    // 期待値は resolver 出力（`ScopePlacement.balloon_offset`）から導出し、
    // 手書き offset のコピー照合にしない（T-I1 との差分＝実パイプライン消費）。
    //
    // 置き場の判断: spawn は resolver 出力と follow API の両方を消費する合成根で
    // あり、兄弟モジュールのテストは自ファイル内という repo 慣行に従いここに置く。
    // -------------------------------------------------------------------------

    use std::collections::BTreeMap;
    use std::time::Instant;

    use wintf::ecs::drag::DragEvent;

    use crate::placement::config::build_placement_config;
    use crate::placement::follow::move_window_to;
    use crate::placement::resolver::{resolve_placement, RectPx, ScopeInput};

    /// ドラッグイベント（wndproc 移動済み後の Bubble 配送を模す・follow.rs と同型）。
    fn drag_event(target: Entity) -> DragEvent {
        DragEvent {
            target,
            start_position: Point::new(0, 0),
            position: Point::new(10, 10),
            is_primary: true,
            timestamp: Instant::now(),
        }
    }

    /// `(key, value)` ペア列 → `parse_kv` 出力相当の `BTreeMap`（config テストと同じ流儀）。
    fn kv_map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    /// 実パイプライン（KV → config → resolver → spawn）で組んだ World。
    ///
    /// - work_area は原点非 (0,0)・全辺 96 の倍数を避けた合成値（隠れた `dpi/96`
    ///   再スケールがあれば完全一致が崩れる檻・repo 慣行）
    /// - scope0＝バルーン left（既定）＋ `balloon.offsetx/offsety` 加算、
    ///   scope1＝バルーン right — 左右両変種を 1 パイプラインで通す
    /// - scope0 のキャラ寸は emo2 実寸（434×687）・他は 96 非倍数の合成値
    fn real_pipeline_world() -> (World, GhostWindows, Vec<ScopePlacement>) {
        let ghost_kv = kv_map(&[("sakura.name", "むらさき"), ("kero.name", "エモ")]);
        let shell_kv = kv_map(&[
            ("seriko.alignmenttodesktop", "bottom"),
            ("sakura.defaultx", "52"),
            ("sakura.balloon.offsetx", "29"),
            ("sakura.balloon.offsety", "-41"),
            ("kero.defaultx", "36"),
            ("kero.balloon.alignment", "right"),
        ]);
        let cfg = build_placement_config(&ghost_kv, &shell_kv);

        let work_area = RectPx {
            left: 31,
            top: 17,
            right: 2574,
            bottom: 1444,
        };
        let scopes = [
            ScopeInput {
                scope: 0,
                char_size: SizePx { w: 434, h: 687 },
                balloon_size: SizePx { w: 401, h: 223 },
            },
            ScopeInput {
                scope: 1,
                char_size: SizePx { w: 278, h: 357 },
                balloon_size: SizePx { w: 227, h: 159 },
            },
        ];
        let placements = resolve_placement(&cfg, work_area, &scopes);

        let mut world = World::new();
        let gw = spawn_ghost_windows(&mut world, &placements, &titles());
        (world, gw, placements)
    }

    /// 全 4 窓へ偽 WindowHandle を付与する（4.2 の偽装境界パターン）。
    fn attach_fake_handles(world: &mut World, gw: &GhostWindows) {
        let mut raw = 0x100isize;
        for scope in gw.scopes().collect::<Vec<_>>() {
            for e in [
                gw.char_window(scope).unwrap(),
                gw.balloon_window(scope).unwrap(),
            ] {
                world.entity_mut(e).insert(fake_window_handle(raw));
                raw += 0x10;
            }
        }
    }

    /// entity の WindowPos.position を読む（未設定は panic で検出）。
    fn window_position(world: &World, e: Entity) -> Point {
        world
            .get::<WindowPos>(e)
            .expect("WindowPos")
            .position
            .expect("position")
    }

    /// T-I4: `spawn_ghost_windows` の `BalloonFollow.offset` が `resolve_placement`
    /// の `balloon_offset` と一致する（実パイプライン・左右両変種・4.2）。
    #[test]
    fn t_i4_follow_offset_matches_resolver_output_through_real_pipeline() {
        let (world, gw, placements) = real_pipeline_world();

        assert_eq!(placements.len(), 2, "空虚一致封じ: 2 スコープが解決されること");

        // config 由来の左右両変種が実際に効いている檻（resolver 幾何の正値を固定し、
        // 「resolver 出力のコピー同士の照合」への退化を防ぐ）
        assert_eq!(
            placements[0].balloon_offset,
            PointPx { x: -401 + 29, y: -41 },
            "scope0: left＝キャラ左隣（−balloon_w）＋balloon.offsetx/y 加算"
        );
        assert_eq!(
            placements[1].balloon_offset,
            PointPx { x: 278, y: 0 },
            "scope1: right＝キャラ右隣（＋char_w）・上端揃え"
        );

        for p in &placements {
            let char_e = gw.char_window(p.scope).expect("char window");
            let follow = world
                .get::<BalloonFollow>(char_e)
                .expect("char window BalloonFollow");
            assert_eq!(
                follow.offset, p.balloon_offset,
                "scope{}: BalloonFollow.offset は resolver 出力の転写",
                p.scope
            );
            assert_eq!(
                follow.balloon,
                gw.balloon_window(p.scope).expect("balloon window"),
                "scope{}: 追従先は自スコープのバルーン窓",
                p.scope
            );

            // 恒等式 balloon_offset ≡ balloon_pos − char_pos（design Postconditions）が
            // spawn 転記後の WindowPos 上でも観測できる
            let char_pos = window_position(&world, char_e);
            let balloon_pos = window_position(&world, follow.balloon);
            assert_eq!(
                PointPx {
                    x: balloon_pos.x - char_pos.x,
                    y: balloon_pos.y - char_pos.y
                },
                p.balloon_offset,
                "scope{}: 初期 WindowPos も恒等式を満たす",
                p.scope
            );
        }
    }

    /// T-I4: spawn 済み entity への `move_window_to` で、バルーンが resolver 由来
    /// offset を保って追従する（複数回移動でも offset 静的・他スコープ不干渉・4.2）。
    #[test]
    fn t_i4_move_window_to_keeps_balloon_offset_across_multiple_moves() {
        let (mut world, gw, placements) = real_pipeline_world();
        attach_fake_handles(&mut world, &gw);

        let p1 = &placements[1];
        let char1 = gw.char_window(1).unwrap();
        let balloon1 = gw.balloon_window(1).unwrap();
        let scope1_initial = (
            window_position(&world, char1),
            window_position(&world, balloon1),
        );

        for p in &placements {
            let char_e = gw.char_window(p.scope).unwrap();
            let balloon_e = gw.balloon_window(p.scope).unwrap();
            // 96 の倍数を避けた移動先を複数回（offset は配置時確定で静的・4.4）
            let targets = [
                (1237 + p.scope as i32, 941),
                (533, 1189 + p.scope as i32),
            ];
            for (x, y) in targets {
                assert!(move_window_to(&mut world, char_e, x, y));
                assert_eq!(
                    window_position(&world, char_e),
                    Point { x, y },
                    "scope{}: 対象自身は指定座標へ（物理 px 素通し）",
                    p.scope
                );
                assert_eq!(
                    window_position(&world, balloon_e),
                    Point {
                        x: x + p.balloon_offset.x,
                        y: y + p.balloon_offset.y
                    },
                    "scope{}: バルーンは resolver 由来 offset を保って追従",
                    p.scope
                );
            }

            // scope0 の移動が scope1 の窓を動かしていない（追従は自スコープのみ）
            if p.scope == 0 {
                assert_eq!(window_position(&world, char1), scope1_initial.0);
                assert_eq!(window_position(&world, balloon1), scope1_initial.1);
                assert_eq!(
                    scope1_initial.1,
                    Point {
                        x: p1.char_pos.x + p1.balloon_offset.x,
                        y: p1.char_pos.y + p1.balloon_offset.y
                    },
                    "前提: scope1 初期位置は resolver 解決値"
                );
            }
        }
    }

    /// T-I4 補: バルーン単独ドラッグの相対位置記憶（4.8・DD16・task 8.3）。
    ///
    /// 仕様退役: 2026-07-11 要件 4.8 —— 本テストの旧版
    /// `t_i4_char_move_restores_initial_offset_after_balloon_solo_move` が檻に
    /// していた「次のキャラ窓移動で初期 offset へスナップバック」は仕様として
    /// 退役し、調整後 offset の記憶・追従が正となった（記憶挙動の檻へ書き換え）。
    ///
    /// 実パイプライン（KV → config → resolver → spawn）で組んだ World 上で、
    /// spawn が付けた**実際の** `OnDrag` ハンドラ（バルーン窓の
    /// `on_balloon_drag`）を呼んで検証する＝結線の檻を兼ねる。
    #[test]
    fn t_i4_char_move_follows_adjusted_offset_after_balloon_solo_drag() {
        let (mut world, gw, placements) = real_pipeline_world();
        attach_fake_handles(&mut world, &gw);
        let p = &placements[0];
        let char_e = gw.char_window(0).unwrap();
        let balloon_e = gw.balloon_window(0).unwrap();

        // バルーン単独ドラッグ: wndproc がバルーンを (613, 407) へ移動済みの状態を
        // 模し、spawn が付けた実 OnDrag ハンドラを Bubble で呼ぶ
        world
            .get_mut::<WindowPos>(balloon_e)
            .unwrap()
            .position = Some(Point { x: 613, y: 407 });
        let handler = world.get::<OnDrag>(balloon_e).expect("balloon OnDrag").0;
        let ev = Phase::Bubble(drag_event(balloon_e));
        assert!(!handler(&mut world, balloon_e, balloon_e, &ev));

        // キャラ窓は不動（4.8: バルーンのみ移動）
        assert_eq!(
            window_position(&world, char_e),
            Point {
                x: p.char_pos.x,
                y: p.char_pos.y
            },
            "バルーンドラッグでキャラ窓は動かない"
        );

        // 調整後 offset = balloon_pos − char_pos が記憶される
        let adjusted = PointPx {
            x: 613 - p.char_pos.x,
            y: 407 - p.char_pos.y,
        };
        assert_ne!(
            adjusted, p.balloon_offset,
            "檻の前提: 調整後 offset は resolver 由来の初期 offset と異なる"
        );
        assert_eq!(
            world.get::<BalloonFollow>(char_e).unwrap().offset,
            adjusted,
            "バルーン単独ドラッグで offset が記憶更新される（4.8）"
        );

        // 次のキャラ窓移動は**調整後** offset で追従（初期 offset へ戻らない）
        assert!(move_window_to(&mut world, char_e, 1751, 893));
        assert_eq!(
            window_position(&world, balloon_e),
            Point {
                x: 1751 + adjusted.x,
                y: 893 + adjusted.y
            },
            "キャラ窓移動でバルーンは調整後 offset 位置へ追従する"
        );

        // 他スコープ（scope1）の offset は不干渉
        let char1 = gw.char_window(1).unwrap();
        assert_eq!(
            world.get::<BalloonFollow>(char1).unwrap().offset,
            placements[1].balloon_offset,
            "scope0 バルーンのドラッグは scope1 の offset を変えない"
        );
    }
}
