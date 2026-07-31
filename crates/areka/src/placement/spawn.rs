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
//!   ＋`HitTest::none()`（全面ヒットで透過を殺さない）＋`Anchored(p.anchor)`（解決済み
//!   アンカーの単一真実源・**全 char 窓へ無条件付与**＝Free 窓も resize の identity 射影で
//!   読む・4.2）＋`DragConfig`（全面ドラッグ・4.1。`move_window` は非 Free アンカーの
//!   キャラ窓のみ false＝on_char_drag 単一ライター・DD15 v2／4.7、Free は true＝wndproc
//!   委譲）＋`OnDrag(on_char_drag)`＋`BalloonFollow`。**全**キャラ窓（Free 含む）は
//!   さらに `OnDragEnd(on_char_drag_end)` を持つ（最終カーソル位置への同写像適用＋
//!   確定位置の永続 write-through・非 Free は最終再固定・Free は保存専用アーム・
//!   DD15 v2 (3)・1.1・task 2.2）。
//!   なおキャラ窓へのポインタハンドラ（`OnPointerMoved`／`OnPointerPressed`）は
//!   **本モジュールでは付けない**——マウス移動／ダブルクリックを kanade へ配信する結線は
//!   `input_events::attach_char_pointer_handlers` が spawn 直後に装着する（依存方向
//!   input_events→placement。placement は `crate::` パスを持たず `super::`／外部 crate のみ
//!   参照する＝example の `#[path]` include で成立させるため。areka-P0-input-events）。
//!   Ctrl+左ダブルクリックは暫定退避（全 `GhostWindowMarker` despawn→window-close funnel→
//!   `run()` 正常復帰）で、これも input_events 側ハンドラ／main.rs の結線が担う（stand-in
//!   即終了 `on_ghost_pressed` は退役）
//! - バルーン窓: 同型（marker は `BalloonWindowMarker{scope}`・`DragConfig::default()`
//!   は付与＝バルーン単独ドラッグ可・4.5。`OnDrag(on_balloon_drag)` で単独ドラッグの
//!   相対位置記憶（4.8・DD16・task 8.3）＋`OnDragEnd(on_balloon_drag_end)` で単独ドラッグ
//!   確定 offset の永続 write-through（2.1・design C3・task 2.3。`on_balloon_drag` は連続
//!   イベントで保存トリガではなく、DragEnd 確定点でのみ 1 ドラッグ 1 書込）・`BalloonFollow`
//!   なし。M1 はマウス送出なし＝ポインタハンドラを付けない・DD-IE-12。バルーン入力は
//!   M-dialogue／choice-render の領分）
//!
//! # clickthrough 登録（task 5.2）
//!
//! [`register_ghost_windows_click_through`] が `Added<WindowHandle>` で
//! [`GhostWindowMarker`] 窓を αマスク clickthrough 機構
//! （wintf `ClickThroughRegistryHandle`・消費のみ）へ登録する
//! （emo-present donor `register_click_through_windows` の一般化・6.1）。

use std::collections::BTreeMap;

use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use bevy_ecs::world::DeferredWorld;
use tracing::debug;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
};
use wintf::ecs::clickthrough::ClickThroughRegistryHandle;
use wintf::ecs::drag::{DragConfig, OnDrag, OnDragEnd};
use wintf::ecs::layout::HitTest;
use wintf::ecs::{Point, SizeI, Window, WindowHandle, WindowPos, WindowStyle};

use super::follow::{
    on_balloon_drag, on_balloon_drag_end, on_char_drag, on_char_drag_end, Anchored, BalloonFollow,
};
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
///
/// # despawn 掃除（areka-P0-dpi-window-vanish 要件 6.1・D8）
///
/// `on_remove` component hook（wintf `Visual::on_add`／`VisualGraphics::on_remove` の先例）で
/// [`GhostWindows`] から当該 scope エントリを落とす。**hook にしているのが要点**で、
/// 「終了処理から掃除関数を呼ぶ」形にすると呼出点結合になり、別経路の despawn
/// （Ctrl+左ダブルクリック退避・将来の個別 close 等）を取りこぼす。marker が消える所＝
/// 窓が消える所であり、そこが唯一の掃除トリガである。
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
#[component(on_remove = on_ghost_window_marker_remove)]
pub struct GhostWindowMarker;

/// [`GhostWindowMarker`] 除去 hook: [`GhostWindows`] から scope エントリを落とす（6.1）。
///
/// # 触ってよいもの／いけないもの（要件 6.4 の構造的保証）
///
/// **Resource（[`GhostWindows`]）だけ**を触る。生存している entity の component は
/// 読みも書きもしない——それゆえ「掃除の前後で生存窓の位置・寸法・追従関係が変化しない」
/// は檻の主張ではなく**構造の帰結**である。`DeferredWorld` は `get::<C>()` で他 entity の
/// component を覗けてしまうが、ここでそれをやってはならない。
///
/// 除去成立も no-op も `debug!` 止まり（正常終了系＝良性ノイズを作らない・要件 6.2 の前提）。
/// Resource 未挿入（ダミー窓フォールバック経路・素の `World` の檻）は静かに no-op。
fn on_ghost_window_marker_remove(mut world: DeferredWorld, hook: HookContext) {
    let entity = hook.entity;
    // Resource 未挿入は no-op（`resource_mut` だと panic するので `get_resource_mut`）。
    let Some(mut registry) = world.get_resource_mut::<GhostWindows>() else {
        return;
    };
    match registry.remove_entry_of(entity) {
        Some((scope, windows)) => debug!(
            scope,
            ?entity,
            char_window = ?windows.char_window,
            balloon_window = ?windows.balloon_window,
            "placement: ゴースト窓レジストリから scope エントリを除去"
        ),
        // 対の後追い despawn（最初の片割れが既に scope ごと落としている）＝正常系。
        None => debug!(
            ?entity,
            "placement: ゴースト窓 despawn だがレジストリに該当 scope なし（除去済み・良性）"
        ),
    }
}

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

    /// `entity` が char/balloon いずれかに一致する scope エントリを**丸ごと**除去し、
    /// 除去した `(scope, ScopeWindows)` を返す（areka-P0-dpi-window-vanish 6.1・D8）。
    ///
    /// 対（char＋balloon）は spawn/despawn とも原子的な生存単位ゆえ、**片割れの entity
    /// 1 個で scope エントリごと**落とす。不一致（既に除去済み・非ゴースト entity・
    /// 空レジストリ）は `None` を返すだけの **no-op**——panic せず、`Err` も出さない。
    /// これが「対の後追い despawn が良性である」ことの構造的な根拠である。
    pub fn remove_entry_of(&mut self, entity: Entity) -> Option<(usize, ScopeWindows)> {
        let scope = self
            .windows
            .iter()
            .find(|(_, w)| w.char_window == entity || w.balloon_window == entity)
            .map(|(scope, _)| *scope)?;
        self.windows.remove(&scope).map(|w| (scope, w))
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
        // （4.8・DD16・task 8.3）・BalloonFollow なし。
        // M1 はバルーンにマウス送出なし＝ポインタハンドラを付けない（DD-IE-12・
        // task 3.2：stand-in `on_ghost_pressed` 登録を撤去。バルーン入力は
        // M-dialogue／choice-render の領分でリゾルバは shell 窓専用）。
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
                // バルーン単独ドラッグ確定 offset の永続 write-through（2.1・8.1・
                // design C3・task 2.3）。on_balloon_drag は連続イベント（in-session offset
                // 更新）で保存トリガではない——DragEnd 確定点でのみ 1 ドラッグ 1 書込する。
                OnDragEnd(on_balloon_drag_end),
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
                // 解決済みアンカーの単一真実源を全 char 窓へ**無条件付与**する（4.2/1.6）。
                // Free 窓も付ける——resize の identity 射影（project_anchor の Free 腕）が
                // Anchored を読むため。二値吸着フラグ（旧 BottomSnap marker）は廃し、
                // ドラッグ／リサイズはこの単一値を読んで射影 T を分岐する（Req1.6・DD15 v2）。
                Anchored(p.anchor),
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
                // マウス入力配線（areka-P0-input-events）: キャラ窓のポインタ移動／押下を
                // kanade へ配信する `OnPointerMoved`／`OnPointerPressed` は**ここでは付けない**。
                // 依存方向は input_events→placement（placement は `crate::` パスを持てない＝
                // example の `#[path]` include で成立させるため）ゆえ、ハンドラ装着は spawn
                // 直後に `input_events::attach_char_pointer_handlers` が行う（stand-in 即終了
                // `on_ghost_pressed` は退役。Ctrl+左ダブルクリック暫定退避もそのハンドラ側の
                // 責務・DD-IE-7）。
            ))
            .id();

        // DragEnd 最終適用＋位置保存の結線（1.1/1.9/4.7/1.6・design C2/C3・task 2.2）:
        // Free 含む**全**キャラ窓へ OnDragEnd を無条件結線する（吸着はドラッグ中の制約で
        // あって保存条件ではない・1.1）。非 Free は on_char_drag_end が Anchored を読んで
        // project_anchor でアンカー辺へ最終再固定し（最終 DragEvent 欠落の穴埋め・DD15 v2 (3)）、
        // Free は射影が identity ゆえ wndproc 確定位置を素通しする保存専用アームとして働く。
        // いずれも on_char_drag_end 末尾で CharWindowMarker.scope を逆引きして位置を
        // Ghost 永続スコープへ write-through する。
        world
            .entity_mut(char_window)
            .insert(OnDragEnd(on_char_drag_end));

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

#[cfg(test)]
mod tests {
    use bevy_ecs::prelude::*;
    use windows::Win32::UI::WindowsAndMessaging::{
        WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
    };
    use wintf::ecs::drag::{DragConfig, DragConstraint, OnDrag, OnDragEnd};
    use wintf::ecs::layout::{BoxStyle, HitTest};
    use wintf::ecs::pointer::{OnPointerMoved, OnPointerPressed, Phase};
    use wintf::ecs::{Point, SizeI, Window, WindowPos, WindowStyle};

    use super::{
        BalloonWindowMarker, CharWindowMarker, GhostWindowMarker, GhostWindows, ScopeWindows,
        spawn_ghost_windows,
    };
    use crate::placement::follow::{Anchored, BalloonFollow};
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

    // -------------------------------------------------------------------------
    // T-V1: despawn 掃除（areka-P0-dpi-window-vanish 要件 6.1/6.4・D8）
    //
    // 檻は「判断が下される場所」＝本ファイル（`remove_entry_of` と on_remove hook）
    // に置く（記憶 test-only-decision-branches-not-proven-wiring・タスク 1.4 の教訓）。
    // -------------------------------------------------------------------------

    /// 生存 scope の「動いていないこと」を判定するための可観測状態の束。
    ///
    /// 位置・寸法（`WindowPos`）と追従関係（`BalloonFollow` の相手 entity と offset）＋
    /// アンカーを 1 個の比較可能な値へまとめる（要件 6.4 の「位置・寸法・追従関係」）。
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct ScopeState {
        char_pos: Option<Point>,
        char_size: Option<SizeI>,
        balloon_pos: Option<Point>,
        balloon_size: Option<SizeI>,
        follow_target: Option<Entity>,
        follow_offset: Option<PointPx>,
        anchor: Option<Anchor>,
    }

    /// `GhostWindows` の引き当てから scope の可観測状態を採取する。
    fn scope_state(world: &World, gw: &GhostWindows, scope: usize) -> ScopeState {
        let char_e = gw.char_window(scope).expect("char window entity");
        let balloon_e = gw.balloon_window(scope).expect("balloon window entity");
        let char_wp = world.get::<WindowPos>(char_e);
        let balloon_wp = world.get::<WindowPos>(balloon_e);
        let follow = world.get::<BalloonFollow>(char_e);
        ScopeState {
            char_pos: char_wp.and_then(|w| w.position),
            char_size: char_wp.and_then(|w| w.size),
            balloon_pos: balloon_wp.and_then(|w| w.position),
            balloon_size: balloon_wp.and_then(|w| w.size),
            follow_target: follow.map(|f| f.balloon),
            follow_offset: follow.map(|f| f.offset),
            anchor: world.get::<Anchored>(char_e).map(|a| a.0),
        }
    }

    /// T-V1: キャラ窓を despawn すると当該 scope が `GhostWindows` から消える（6.1）。
    ///
    /// 呼出点結合なしで拾えること（＝`world.despawn` を素で叩くだけで発火すること）が
    /// 要件の核心ゆえ、掃除関数を明示的に呼ぶ経路は檻に持ち込まない。
    #[test]
    fn t_v1_despawn_char_window_removes_scope_entry() {
        let mut world = World::new();
        let placements = two_scope_placements();
        let gw = spawn_ghost_windows(&mut world, &placements, &titles());
        let char0 = gw.char_window(0).unwrap();

        world.despawn(char0);

        let reg = world.resource::<GhostWindows>();
        assert_eq!(reg.scopes().collect::<Vec<_>>(), vec![1]);
        assert_eq!(reg.char_window(0), None);
        // scope 粒度＝対の相方も同時に引き当て不能になる（D8）
        assert_eq!(reg.balloon_window(0), None);
    }

    /// T-V1: バルーン窓側の despawn でも同じ scope エントリが消える（片割れ対称性・D8）。
    #[test]
    fn t_v1_despawn_balloon_window_removes_same_scope_entry() {
        let mut world = World::new();
        let placements = two_scope_placements();
        let gw = spawn_ghost_windows(&mut world, &placements, &titles());
        let balloon1 = gw.balloon_window(1).unwrap();

        world.despawn(balloon1);

        let reg = world.resource::<GhostWindows>();
        assert_eq!(reg.scopes().collect::<Vec<_>>(), vec![0]);
        assert_eq!(reg.char_window(1), None);
        assert_eq!(reg.balloon_window(1), None);
    }

    /// T-V1: 対の**後追い** despawn は no-op（panic せず・他 scope を巻き込まない・6.1）。
    ///
    /// 終了処理（`despawn_smoke_targets`）は同一 World 変異内で対を一括 despawn する＝
    /// 2 個目の hook は必ず「既に除去済みの scope」を引く。ここが panic すると終了経路が
    /// 落ちるため、良性であることを構造で固定する。
    #[test]
    fn t_v1_paired_followup_despawn_is_noop() {
        let mut world = World::new();
        let placements = two_scope_placements();
        let gw = spawn_ghost_windows(&mut world, &placements, &titles());
        let char0 = gw.char_window(0).unwrap();
        let balloon0 = gw.balloon_window(0).unwrap();

        world.despawn(char0);
        let after_first: Vec<usize> = world.resource::<GhostWindows>().scopes().collect();

        // 後追いの片割れ——panic しない・レジストリは 1 ミリも動かない
        world.despawn(balloon0);

        let reg = world.resource::<GhostWindows>();
        assert_eq!(after_first, vec![1]);
        assert_eq!(reg.scopes().collect::<Vec<_>>(), after_first);
        assert_eq!(reg.char_window(1), gw.char_window(1));
        assert_eq!(reg.balloon_window(1), gw.balloon_window(1));
    }

    /// T-V1: 未登録 entity の除去は no-op（不一致・二重除去・空レジストリ・6.1）。
    ///
    /// `remove_entry_of` の全域性を直接叩いて固定する（hook はこの関数しか呼ばない）。
    #[test]
    fn t_v1_remove_entry_of_unregistered_entity_is_noop() {
        let mut world = World::new();
        let placements = two_scope_placements();
        let gw = spawn_ghost_windows(&mut world, &placements, &titles());
        let stranger = world.spawn_empty().id();

        let mut reg = world.resource_mut::<GhostWindows>();
        // 非ゴースト entity
        assert_eq!(reg.remove_entry_of(stranger), None);
        assert_eq!(reg.scopes().collect::<Vec<_>>(), vec![0, 1]);

        // 一致 → 二重除去は None
        let char1 = gw.char_window(1).unwrap();
        let (scope, removed) = reg.remove_entry_of(char1).expect("初回は除去成立");
        assert_eq!(scope, 1);
        assert_eq!(removed.char_window, char1);
        assert_eq!(removed.balloon_window, gw.balloon_window(1).unwrap());
        assert_eq!(reg.remove_entry_of(char1), None);
        assert_eq!(reg.remove_entry_of(removed.balloon_window), None);

        // 空になっても no-op（panic しない）
        let expected0 = ScopeWindows {
            char_window: gw.char_window(0).unwrap(),
            balloon_window: gw.balloon_window(0).unwrap(),
        };
        assert_eq!(
            reg.remove_entry_of(expected0.balloon_window),
            Some((0, expected0))
        );
        assert_eq!(reg.scopes().collect::<Vec<_>>(), Vec::<usize>::new());
        assert_eq!(reg.remove_entry_of(char1), None);
    }

    /// T-V1: 掃除の前後で**生存 scope** の位置・寸法・追従関係が変化しない（6.4）。
    ///
    /// 空虚性回避（記憶 2.2 の教訓・タスク 2.2 Implementation Notes）:
    /// ①探針は 2 scope で、位置・寸法・offset・追従先がすべて**互いに異なる**ことを
    /// 事前に `assert_ne!` で自己検査する（＝「全部同じ値」なら不変主張が不動点に落ちて
    /// 何も検出できない）。②「レジストリに scope 1 が残っている」も同時に主張する
    /// （＝除去が広すぎる変異——例えば scope 粒度でなく全消し——を捕まえる）。
    #[test]
    fn t_v1_surviving_scope_state_unchanged_by_cleanup() {
        let mut world = World::new();
        let placements = two_scope_placements();
        let gw = spawn_ghost_windows(&mut world, &placements, &titles());

        let before0 = scope_state(&world, &gw, 0);
        let before1 = scope_state(&world, &gw, 1);

        // 探針の自己検査: 2 scope の可観測状態は互いに異なる（不動点でない）。
        assert_ne!(before0.char_pos, before1.char_pos);
        assert_ne!(before0.char_size, before1.char_size);
        assert_ne!(before0.balloon_pos, before1.balloon_pos);
        assert_ne!(before0.follow_offset, before1.follow_offset);
        assert_ne!(before0.follow_target, before1.follow_target);
        assert!(before1.char_pos.is_some() && before1.follow_offset.is_some());

        // scope 0 の対を一括 despawn（終了処理と同じ形）。
        world.despawn(gw.char_window(0).unwrap());
        world.despawn(gw.balloon_window(0).unwrap());

        // 生存 scope 1: 位置・寸法・追従関係・アンカーが 1 ビットも動かない（6.4）。
        assert_eq!(scope_state(&world, &gw, 1), before1);

        // 除去は scope 粒度で止まる（生存 scope の引き当ては健在）。
        let reg = world.resource::<GhostWindows>();
        assert_eq!(reg.scopes().collect::<Vec<_>>(), vec![1]);
        assert_eq!(reg.char_window(1), gw.char_window(1));
        assert_eq!(reg.balloon_window(1), gw.balloon_window(1));
    }

    /// T-V1: `GhostWindows` Resource 未挿入の World で despawn しても no-op（hook が
    /// Resource 不在で panic しない・design「Resource 未挿入は no-op」）。
    #[test]
    fn t_v1_despawn_without_registry_resource_is_noop() {
        let mut world = World::new();
        let e = world.spawn(GhostWindowMarker).id();

        world.despawn(e); // panic しなければ合格

        assert!(world.get_resource::<GhostWindows>().is_none());
    }

    /// T-V1: 掃除は Resource のみを触る＝**生存 entity の component へ一切書かない**
    /// ことをログ側からも固定する（6.4 の構造的保証）。除去成立・no-op のいずれも
    /// `debug!` 止まりで、warn 以上を出さない（要件 6.2 の前提となる静穏性）。
    #[test]
    fn t_v1_cleanup_logs_are_debug_only_and_name_the_scope() {
        let (_, events) = crate::placement::test_support::capture_logs(|| {
            let mut world = World::new();
            let placements = two_scope_placements();
            let gw = spawn_ghost_windows(&mut world, &placements, &titles());
            let char0 = gw.char_window(0).unwrap();
            let balloon0 = gw.balloon_window(0).unwrap();
            world.despawn(char0);
            world.despawn(balloon0);
        });

        // 除去成立 1 件（最初の片割れ）＋ no-op 1 件（後追いの片割れ）
        let removed = crate::placement::test_support::expect_one(
            &events,
            "placement: ゴースト窓レジストリから scope エントリを除去",
        );
        assert_eq!(removed.level, tracing::Level::DEBUG);
        assert_eq!(removed.field("scope"), "0");

        let noop = crate::placement::test_support::expect_one(
            &events,
            "placement: ゴースト窓 despawn だがレジストリに該当 scope なし",
        );
        assert_eq!(noop.level, tracing::Level::DEBUG);

        // info 以上（＝warn/error を含む）は 1 行も出さない（良性ノイズを作らない）。
        // `tracing::Level` の Ord は ERROR < WARN < INFO < DEBUG < TRACE ゆえ
        // 「INFO より verbose」＝ debug/trace のみ、が静穏性の表現になる。
        assert!(
            events.iter().all(|e| e.level > tracing::Level::INFO),
            "掃除経路が info 以上のログを出している: {events:?}"
        );
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

    /// T-I1 補: 全窓が `HitTest::none()`（全面ヒットで透過を殺さない）を持つ。
    /// `spawn_ghost_windows` 自体はポインタハンドラを付けない（依存方向
    /// input_events→placement・placement は `crate::` パスを持てないため）。
    /// spawn 直後に `input_events::attach_char_pointer_handlers` を呼ぶと、キャラ窓は
    /// 正規ポインタハンドラ（`OnPointerMoved`＋`OnPointerPressed`＝stand-in
    /// `on_ghost_pressed` を退役して差し替え）を持ち、バルーン窓はポインタハンドラを
    /// 一切持たない（M1 はバルーンにマウス送出なし・DD-IE-12）。ハンドラ実挙動の檻は
    /// input_events の task 2.7 檻＝proven-wiring ゆえ存在の有無だけ固定する
    /// （[[test-only-decision-branches-not-proven-wiring]]）。本テストは `#[cfg(test)]`
    /// ゆえ `crate::` パス使用可（example の `#[path]` include 不変条件は非テストコード限定）。
    #[test]
    fn t_i1_all_windows_have_hit_test_none_and_char_has_pointer_handlers() {
        let mut world = World::new();
        let placements = two_scope_placements();

        let gw = spawn_ghost_windows(&mut world, &placements, &titles());
        // spawn は crate::-free ゆえハンドラを付けない。装着は input_events が担う。
        crate::input_events::attach_char_pointer_handlers(&mut world);

        for e in ghost_window_entities(&mut world) {
            assert_eq!(world.get::<HitTest>(e).copied(), Some(HitTest::none()));
        }

        for p in &placements {
            let char_e = gw.char_window(p.scope).unwrap();
            let balloon_e = gw.balloon_window(p.scope).unwrap();

            // キャラ窓は正規の移動／押下ハンドラを両方持つ（task 3.2 差し替え）
            assert!(
                world.get::<OnPointerMoved>(char_e).is_some(),
                "scope{}: キャラ窓は OnPointerMoved を持つ",
                p.scope
            );
            assert!(
                world.get::<OnPointerPressed>(char_e).is_some(),
                "scope{}: キャラ窓は OnPointerPressed を持つ",
                p.scope
            );

            // バルーン窓はポインタハンドラを一切持たない（DD-IE-12・stand-in 撤去）
            assert!(
                world.get::<OnPointerMoved>(balloon_e).is_none(),
                "scope{}: バルーン窓に OnPointerMoved を付けない（DD-IE-12）",
                p.scope
            );
            assert!(
                world.get::<OnPointerPressed>(balloon_e).is_none(),
                "scope{}: バルーン窓に OnPointerPressed を付けない（DD-IE-12）",
                p.scope
            );
        }
    }

    // -------------------------------------------------------------------------
    // anchor 伝搬（4.2・DD15 基盤・task 3.1）
    //
    // 旧 `bottom_snap_marker_attached_to_snapping_char_windows_only`（二値吸着 marker
    // の有無検証）を、解決済み 5 値アンカーの entity 表現（`Anchored`）付与検証へ
    // 意味を保って差し替え（marker 有無→アンカー種別付与・単一真実源＝`Anchored`・Req1.6）。
    // -------------------------------------------------------------------------

    /// 3.1（4.2/1.6）: 生成直後の**全 char 窓**が対応する 5 値アンカーの
    /// `Anchored(anchor)` を保持する（非 Free 窓は非 Free アンカー・Free 窓は
    /// `Anchored(Anchor::Free)`＝無条件付与）。バルーン窓には `Anchored` を付けない
    /// （吸着／リサイズ対象はキャラ窓のみ・DD15）。
    #[test]
    fn anchored_attached_to_all_char_windows_with_resolved_anchor() {
        let mut world = World::new();
        let mut placements = two_scope_placements(); // 両方 Anchor::Bottom（emo2＝bottom）
        placements[1].anchor = Anchor::Free; // scope1 を非吸着（Free）へ

        let gw = spawn_ghost_windows(&mut world, &placements, &titles());

        let char0 = gw.char_window(0).unwrap();
        let char1 = gw.char_window(1).unwrap();
        // 各 char 窓は自スコープの解決済みアンカーを Anchored として保持する（4.2）
        assert_eq!(
            world.get::<Anchored>(char0).copied(),
            Some(Anchored(Anchor::Bottom)),
            "非 Free スコープの char 窓は Anchored(Bottom) を保持"
        );
        assert_eq!(
            world.get::<Anchored>(char1).copied(),
            Some(Anchored(Anchor::Free)),
            "Free スコープの char 窓も無条件で Anchored(Free) を保持（resize identity 射影が読む）"
        );
        // 転写元の placement と一致すること（値の取り違え封じ）
        for p in &placements {
            assert_eq!(
                world.get::<Anchored>(gw.char_window(p.scope).unwrap()).copied(),
                Some(Anchored(p.anchor)),
                "scope{}: Anchored は ScopePlacement.anchor の転写",
                p.scope
            );
        }
        // バルーン窓には anchor 値によらず Anchored を付けない（char 窓のみ）
        for scope in [0usize, 1] {
            assert!(
                world
                    .get::<Anchored>(gw.balloon_window(scope).unwrap())
                    .is_none(),
                "scope{scope}: バルーン窓には Anchored を付けない"
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

    /// T-I3（3.1 改訂）: 窓 entity に `BoxStyle` 不在（U2・論理 DIP を持ち込まない）・
    /// `DragConstraint` 不在（DD8・全モニタドラッグ可・4.5）。`DragConfig.move_window`
    /// は「非 Free アンカーのキャラ窓のみ false（単一ライター・DD15 v2・4.7）、Free
    /// キャラ窓とバルーン窓は true（wndproc 委譲・4.1/4.5）」。DragEnd ハンドラ
    /// （`OnDragEnd`）は Free 含む**全**キャラ窓に付く（非 Free は最終再固定・Free は
    /// 保存専用アーム・1.1・task 2.2）。バルーン窓にも `OnDragEnd`（`on_balloon_drag_end`）が
    /// 付く（単独ドラッグ確定 offset の永続 write-through・2.1・task 2.3）。`Anchored` は
    /// char 窓のみ（Free/非 Free とも無条件付与）・バルーン窓には付かない（4.2/1.6）。
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
            "非 Free アンカーのキャラ窓は move_window=false（単一ライター・DD15 v2・4.7）"
        );
        assert!(
            world.get::<OnDragEnd>(snap_char).is_some(),
            "非 Free アンカーのキャラ窓に DragEnd 最終適用ハンドラが付く（DD15 v2 (3)）"
        );
        assert!(
            world.get::<DragConfig>(free_char).unwrap().move_window,
            "Free キャラ窓は move_window=true（wndproc 委譲・4.1）"
        );
        assert!(
            world.get::<OnDragEnd>(free_char).is_some(),
            "Free キャラ窓にも OnDragEnd が付く（全アンカー結線・保存専用アーム・1.1・task 2.2）"
        );
        // Anchored は char 窓のみ（Free/非 Free とも付与）・バルーン窓には付かない（4.2/1.6）
        assert_eq!(
            world.get::<Anchored>(snap_char).copied(),
            Some(Anchored(Anchor::Bottom)),
            "非 Free キャラ窓は Anchored(Bottom) を保持"
        );
        assert_eq!(
            world.get::<Anchored>(free_char).copied(),
            Some(Anchored(Anchor::Free)),
            "Free キャラ窓も Anchored(Free) を保持"
        );
        for scope in [0usize, 1] {
            let balloon = gw.balloon_window(scope).unwrap();
            assert!(
                world.get::<DragConfig>(balloon).unwrap().move_window,
                "scope{scope}: バルーン窓は move_window=true（単独ドラッグ・4.5）"
            );
            assert!(
                world.get::<OnDragEnd>(balloon).is_some(),
                "scope{scope}: バルーン窓に OnDragEnd（on_balloon_drag_end）が付く（単独ドラッグ確定 offset 保存・task 2.3）"
            );
            assert!(
                world.get::<Anchored>(balloon).is_none(),
                "scope{scope}: バルーン窓に Anchored は付けない（char 窓のみ）"
            );
        }
    }

    // -------------------------------------------------------------------------
    // stand-in 即終了（`on_ghost_pressed`）の退役（areka-P0-input-events task 3.2・6.1）
    //
    // 旧テスト `double_click_left_despawns_all_ghost_windows` /
    // `non_left_double_click_and_tunnel_do_not_despawn_ghost_windows` は stand-in
    // 即終了（プレーンなダブルクリックで全窓 despawn）の挙動を檻にしていたが、
    // 本 task で stand-in を退役し正規ハンドラ（`on_char_pointer_pressed`）へ
    // 差し替えたため仕様退役＝除去した（[[obsolete-vs-broken-test-policy]]）。
    // 正規ハンドラの挙動檻（プレーン dblclick は despawn せず DoubleClick 送出／
    // Ctrl+左 dblclick で暫定退避 despawn）は input_events の task 2.7 檻が所有し、
    // task 4.4 で再カバーされる。本 spawn.rs 側は登録の存在
    // （`t_i1_all_windows_have_hit_test_none_and_char_has_pointer_handlers`）で足る。
    // -------------------------------------------------------------------------

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
