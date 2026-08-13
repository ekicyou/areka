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
//! キャラ窓・バルーン窓の**両方**が `DpiSuggestedRectPolicy::ExternalAuthority` を持つ
//! （areka-P0-dpi-window-vanish 要件 4.3・D3。[`external_position_authority`] に理由を記載）。
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
use wintf::ecs::{
    DpiSuggestedRectPolicy, Point, SizeI, Window, WindowHandle, WindowPos, WindowStyle,
};

use super::follow::{
    on_balloon_drag, on_balloon_drag_end, on_char_drag, on_char_drag_end, Anchored, BalloonFollow,
};
use super::resolver::{PointPx, ScopePlacement};
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
    /// spawn 時に既定配置（resolver 出力）が置いたキャラ窓位置（物理 px・scg 7.3）。
    ///
    /// 「まだ誰にも動かされていない」ことの判定基準。現在位置がこの値と一致する間は
    /// 既定配置のままであり、ゴースト台本の移動指令や利用者のドラッグで動いた
    /// スコープは一致しなくなる——移動側へフックを足さずに除外できる。
    pub default_char_pos: PointPx,
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

    /// スコープの既定キャラ位置（spawn 時の resolver 出力）を返す（未知スコープは `None`）。
    ///
    /// 現在位置がこの値と一致するかで「既定配置のまま＝まだ誰にも動かされていない」ことを
    /// 判定する（scg 7.3）。
    pub fn default_char_pos(&self, scope: usize) -> Option<PointPx> {
        self.windows.get(&scope).map(|w| w.default_char_pos)
    }

    /// スコープの既定キャラ位置を更新する（実表示寸での連鎖再解決が確定させた値・scg 7.1）。
    ///
    /// 未知スコープは **no-op**（panic せず `false` を返す）。台帳を再解決後の真値へ揃え、
    /// 以後の「既定配置のまま」判定が確定後の位置を基準に働くようにする。
    pub fn set_default_char_pos(&mut self, scope: usize, pos: PointPx) -> bool {
        match self.windows.get_mut(&scope) {
            Some(w) => {
                w.default_char_pos = pos;
                true
            }
            None => false,
        }
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
                // 位置権威の外部宣言（areka-P0-dpi-window-vanish 4.3・D3）。バルーン窓も
                // OS 直書きから外す——キャラ窓だけ外すと、DPI 跨ぎでバルーンだけが OS 提案
                // 位置へ飛び、`balloon_pos − char_pos ≡ offset` の恒等式が構造的に崩れる。
                external_position_authority(),
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
                // 位置権威の外部宣言（areka-P0-dpi-window-vanish 4.3・D3・S1 是正の源断ち）。
                external_position_authority(),
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
                default_char_pos: p.char_pos,
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

/// 全ゴースト窓共通の位置権威宣言（areka-P0-dpi-window-vanish 要件 4.3・D3・task 5.1）。
///
/// # なぜ全ゴースト窓に要るのか
///
/// `WM_DPICHANGED` の OS 提案矩形は「モニタ間の DPI 比で現在位置を素直に拡縮した」
/// 位置であって、接地点規約（下端中央）とは無関係である。wintf 側はこの component が
/// **未付与の窓へは従来どおり提案位置を書き込む**（Per-Monitor v2 の標準応答＝
/// 非ゴースト窓の後方互換）。ゴースト窓の位置を決める権威は areka の配置系
/// （`project_anchor`／`resize_window_to`／DPI 相の再射影）ただ 1 つであり、
/// 二重ライターを許すと OS 由来座標が `WindowPos.position` へ landing して、
/// 以後の射影が「直前に areka が確定した接地点」ではなく OS 提示値を生位置として読む
/// （診断レポート §1.1 の連鎖①〜④＝S1。実機セッション①で `applied=true` が 84/84）。
///
/// 付与の責務が窓の所有者側にあること（wintf は読むだけ）は D3 の裁定である。
fn external_position_authority() -> DpiSuggestedRectPolicy {
    DpiSuggestedRectPolicy::ExternalAuthority
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
#[path = "spawn_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "spawn_assembly_tests.rs"]
mod assembly_tests;
#[cfg(test)]
#[path = "spawn_cleanup_tests.rs"]
mod cleanup_tests;
#[cfg(test)]
#[path = "spawn_clickthrough_tests.rs"]
mod clickthrough_tests;
#[cfg(test)]
#[path = "spawn_follow_pipeline_tests.rs"]
mod follow_pipeline_tests;
