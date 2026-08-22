//! UI→kanade のマウス入力配信配線（areka-P0-input-events）。
//!
//! キャラ窓のポインタイベントを捉え、当たり判定名を collision-geometry の resolver で解決し、
//! 送出間引き（[`throttle`]）を通して kanade へマウス入力メッセージとして配信する薄い配線層。
//!
//! 本モジュールは現状 [`throttle`]（送出間引きの純粋・決定的判定・task 2.4）のみを収める。
//! per-scope 間引き状態を `HashMap` で保持する `MouseWiring` とポインタハンドラ結線は
//! task 2.6／2.7 で本 mod へ増設される。

pub(crate) mod balloon;
pub(crate) mod choice_drain;
pub(crate) mod throttle;

use std::collections::HashMap;
use std::sync::mpsc::Sender;

use areka_emo_present::EmoPresenter;
use areka_kanade::{KanadeMsg, MouseButton, MouseEventKind, MouseInput};
use bevy_ecs::prelude::*;
use wintf::ecs::pointer::{DoubleClick, OnPointerMoved, OnPointerPressed, Phase, PointerState};

use crate::emo2_boot::frame::Emo2Wiring;
use crate::emo2_boot::hit_region::{HitRegion, resolve_hit_region};
use crate::placement::spawn::{CharWindowMarker, GhostWindowMarker};
use throttle::{MouseMoveThrottle, plan_mouse_move};

/// UI スレッド所有のマウス入力配信資源（NonSend・DD-IE-9）。
///
/// kanade への投函端（[`Sender`] クローン・1.4）・per-scope 間引き状態・当たり判定 resolver の
/// 供給源（実／mock の差し替えシーム・[`RegionSource`]・1.5）・注入可能な時刻源（`now_ms`）を
/// 1 資源に束ねる。[`Sender`] 単体は `Send` だが、resolver（presenter 読み）と間引き状態が
/// UI スレッド所有ゆえ NonSend 1 個に束ねる（`Emo2Wiring` 前例と同型・順序依存なし self-gating）。
///
/// 本 struct と送出ヘルパは task 2.6 の範囲。ポインタハンドラ（`on_char_pointer_moved` /
/// `on_char_pointer_pressed`）と暫定退避（Ctrl+左ダブルクリック）は task 2.7。`wire_mouse_input`
/// による World 挿入（main.rs の boot 成功後呼出）は task 3.1 で結線済み＝`new` は本番から到達可能。
/// 送出ヘルパ群はポインタハンドラ経由でのみ参照される。ハンドラのキャラ窓登録は本モジュールの
/// [`attach_char_pointer_handlers`]（依存方向 input_events→placement。stand-in `on_ghost_pressed`
/// を退役して差し替え・main.rs が spawn 直後に呼ぶ）で完了済み＝本番消費者が到達したため
/// dead_code 抑止は不要になった。
pub(crate) struct MouseWiring {
    /// `GhostRuntime::kanade()` クローン（1.4・std mpsc）。
    sender: Sender<KanadeMsg>,
    /// per-scope 間引き状態（scope→状態）。
    throttle: HashMap<u32, MouseMoveThrottle>,
    /// 実／mock の差し替えシーム（1.5）。
    region_source: RegionSource,
    /// 注入可能 clock（既定: 起動からの経過 ms・単調）。
    now_ms: Box<dyn FnMut() -> u64>,
}

/// 当たり判定名の供給源シーム（実／mock）。
///
/// `Presenter` は `wire_mouse_input`（task 3.1）が本番構築する。`Mock` は決定論檻専用。
pub(crate) enum RegionSource {
    /// 実運用: presenter で `resolve_hit_region` を呼ぶ（1.3）。
    Presenter,
    /// 決定論檻: 固定写像で `HitRegion` を返す（1.5）。
    ///
    /// `#[allow(dead_code)]`: mock seam ゆえ本番からは構築されない（テスト専用・恒久）。
    #[allow(dead_code)]
    Mock(fn(u32, i64, i64) -> HitRegion),
}

impl MouseWiring {
    /// 実運用の構築子（既定 clock＝構築時に捕捉した [`Instant`] からの経過 ms）。
    ///
    /// NOTE: 既定 clock の構築は純粋テスト経路の外に置く（テストは [`with_clock`] で決定的 clock を
    /// 注入する）。`Instant::now()` を読むためユニット檻からは使わない。
    ///
    /// [`Instant`]: std::time::Instant
    /// [`with_clock`]: MouseWiring::with_clock
    pub(crate) fn new(sender: Sender<KanadeMsg>, region_source: RegionSource) -> Self {
        let start = std::time::Instant::now();
        Self {
            sender,
            throttle: HashMap::new(),
            region_source,
            now_ms: Box::new(move || start.elapsed().as_millis() as u64),
        }
    }

    /// テスト用の構築子（決定的 clock を注入する・純粋檻用）。
    #[cfg(test)]
    fn with_clock(
        sender: Sender<KanadeMsg>,
        region_source: RegionSource,
        now_ms: Box<dyn FnMut() -> u64>,
    ) -> Self {
        Self {
            sender,
            throttle: HashMap::new(),
            region_source,
            now_ms,
        }
    }

    /// (scope, 窓 client 物理 px) → [`HitRegion`]（当たり判定名＋配信空間の座標・DD-IE-10 改訂）。
    ///
    /// - [`RegionSource::Mock`] → `f(scope, x, y)`（presenter を無視・1.5）。
    /// - [`RegionSource::Presenter`] → `Some(p)` なら `resolve_hit_region(p, scope, x, y)`（1.3）。
    ///   `presenter` 不在（`Emo2Wiring` 未挿入＝boot 前／失敗時）は `region: None` へ正常縮退する
    ///   （collision-geometry design の消費想定どおり・trace）。このとき `surface_point` は**無変換の
    ///   入力値**とする——presenter が居なければ実適用 k を知る術が無く、等倍相当（縮約は恒等）が
    ///   唯一整合する縮退規約である（presenter 側の k 不在縮退＝要件 1.6 と同じ扱い。座標空間の
    ///   正準契約は `emo2_boot::hit_region` の冒頭 doc）。
    ///
    /// # 座標空間（DD-IE-10 改訂・areka-P0-collision-dpi-hittest）
    ///
    /// 旧 DD-IE-10 の「座標は素通し＝DPI 変換なし・k=1.0 限定契約」は**解除済み**。現契約は次のとおりで、
    /// 全体像の正本は [`crate::emo2_boot::hit_region`] の冒頭 doc（受領空間・吸収点・配信空間・
    /// shell 限定の旨）である。本 doc はそこへの参照＋本層の差分のみを述べる。
    ///
    /// - 受領する `x`/`y` は当該 shell 窓の client 物理 px であり、**そのまま** resolver へ渡す
    ///   （呼び手側で ÷k しない＝前処理は二重縮約になる）。
    /// - ÷k は resolver の先の presenter（`hit_region_client`）が吸収する。本層は縮約の式を持たない。
    /// - 返る [`HitRegion::surface_point`] が SHIORI へ配信する座標（縮約後サーフェス px）であり、
    ///   throttle の位置比較だけは縮約前の client px を使い続ける（[`plan_and_send_move`] 参照・6.8）。
    ///
    /// [`plan_and_send_move`]: MouseWiring::plan_and_send_move
    fn resolve_region(
        &self,
        presenter: Option<&EmoPresenter>,
        scope: u32,
        x: i64,
        y: i64,
    ) -> HitRegion {
        match self.region_source {
            RegionSource::Mock(f) => f(scope, x, y),
            RegionSource::Presenter => match presenter {
                Some(p) => resolve_hit_region(p, scope, x, y),
                None => {
                    tracing::trace!(
                        event = "mouse_region_degrade",
                        scope,
                        "Emo2Wiring 不在（boot 前／失敗時）: region None へ正常縮退"
                    );
                    // k 不明ゆえ等倍相当（縮約は恒等）＝受領した client 物理 px をそのまま配信空間の値とする。
                    HitRegion {
                        scope,
                        region: None,
                        surface_point: (x, y),
                    }
                }
            },
        }
    }

    /// per-scope 間引き判定を通し、送出条件成立時のみ `OnMouseMove` 相当を送出する（5.1・DD-IE-5）。
    ///
    /// per-scope の [`MouseMoveThrottle`] を引き（無ければ既定生成）、[`plan_mouse_move`] で
    /// (次状態, 送出可否) を求めて次状態を保存し、送出可否が true のときだけ
    /// `KanadeMsg::Mouse(MouseInput { .., kind: Move })` を [`Sender`] へ送る。
    ///
    /// # 2 つの座標空間（DD-IE-10 改訂・DD-4）
    ///
    /// - `client_pos`: 窓 client 物理 px。**throttle の位置比較専用**であり、縮約前の空間を保持し
    ///   続ける唯一の分岐である（縮約すると移動検出の実効粒度が k 倍粗くなる・6.8）。
    /// - `surface_pos`: resolver が返した縮約後サーフェス px。**配信する `MouseInput{x,y}` の値**
    ///   であり、`region` と同一空間に揃う（1.8）。k=1.0 では両者が一致し従前の配信値と同一（1.9）。
    ///
    /// 座標契約の全体像は [`crate::emo2_boot::hit_region`] の冒頭 doc（正本）を参照。
    /// 返り値は実際に送出したか。
    /// 送出失敗（kanade 停止後の [`Sender`] エラー）は warn＋no-op（false 返し・log-first）。
    fn plan_and_send_move(
        &mut self,
        scope: u32,
        client_pos: (i64, i64),
        surface_pos: (i64, i64),
        region: Option<String>,
    ) -> bool {
        let now = (self.now_ms)();
        let state = self.throttle.entry(scope).or_default();
        // 位置比較は縮約前 client px のまま（throttle.rs は無変更・6.8）。
        let (next, send) = plan_mouse_move(state, client_pos, &region, now);
        *state = next;

        if !send {
            return false;
        }

        let (x, y) = surface_pos;
        let msg = KanadeMsg::Mouse(MouseInput {
            scope,
            x,
            y,
            region,
            kind: MouseEventKind::Move,
        });
        if self.sender.send(msg).is_err() {
            tracing::warn!(
                event = "mouse_send_failed",
                scope,
                kind = "move",
                "kanade Sender 送出失敗（actor 停止後）: no-op で継続"
            );
            return false;
        }
        true
    }

    /// `OnMouseDoubleClick` 相当を無条件送出する（間引きなし・1.2/3.x）。
    ///
    /// クリックは間引き対象外ゆえ throttle を通さず即送出する。`surface_pos` は resolver が返した
    /// 縮約後サーフェス px＝配信空間の値（1.8・DD-IE-10 改訂）。throttle を経ないため本経路に
    /// client px は現れない。座標契約の正本は [`crate::emo2_boot::hit_region`] の冒頭 doc。
    /// 送出失敗は warn＋no-op（log-first）。
    fn send_double_click(
        &mut self,
        scope: u32,
        surface_pos: (i64, i64),
        region: Option<String>,
        button: MouseButton,
    ) {
        let (x, y) = surface_pos;
        let msg = KanadeMsg::Mouse(MouseInput {
            scope,
            x,
            y,
            region,
            kind: MouseEventKind::DoubleClick { button },
        });
        if self.sender.send(msg).is_err() {
            tracing::warn!(
                event = "mouse_send_failed",
                scope,
                kind = "double_click",
                "kanade Sender 送出失敗（actor 停止後）: no-op で継続"
            );
        }
    }
}

/// boot 成功後に main から呼ぶ（task 3.1・DD-IE-9）。
///
/// kanade Sender クローンで `MouseWiring`（NonSend・`RegionSource::Presenter`）を World へ挿入する。
/// `Emo2Wiring` 挿入と同型（emo2_boot/mod.rs:341-345）・self-gating＝窓 spawn と挿入の順序に
/// 依存しない（click-through 登録と同型）。窓へのポインタハンドラ登録は task 3.2（spawn.rs）。
///
/// `wire_emo2_boot` 成功時（`wired=true`）に呼ばれる前提で `Presenter` を選ぶ。boot 成功時は
/// `Emo2Wiring` 挿入済みゆえ presenter 経由の region 解決が成立する（万一 presenter 不在でも
/// `resolve_region` が region None へ正常縮退する・DD-IE-9）。
pub(crate) fn wire_mouse_input(world: &mut World, sender: Sender<KanadeMsg>) {
    world.insert_non_send(MouseWiring::new(sender, RegionSource::Presenter));
}

/// キャラ窓へポインタハンドラ（[`on_char_pointer_moved`]／[`on_char_pointer_pressed`]）を装着する。
///
/// 全 [`CharWindowMarker`] 窓に `OnPointerMoved`＋`OnPointerPressed` を挿入する
/// （バルーン窓には付けない＝M1 はバルーンにマウス送出なし・DD-IE-12）。
///
/// # 依存方向（regression 修正）
///
/// この結線は本来 `placement::spawn` が担うのが素直だが、placement 本体は
/// `crate::` パスを持てない（example が `#[path]` で私有 include するため・
/// `window-placement.rs`／`collision-probe.rs`）。設計の依存方向も
/// `input_events → placement` ゆえ、ハンドラ装着は placement に依存できる
/// **本モジュール側**が所有する（stand-in 即終了 `on_ghost_pressed` は退役）。
///
/// # タイミング契約
///
/// `spawn_ghost_windows` の**直後**に同一 `&mut World` クロージャ内で呼ぶこと
/// （キャラ窓が既に存在する状態・main.rs の `open_startup_window` 結線）。同一
/// World-mutation 内で同期実行するため async race はない。
pub(crate) fn attach_char_pointer_handlers(world: &mut World) {
    // `&mut World` を借用中にクエリで別の可変借用を取れないため、まず対象 entity を
    // 収集してから 1 件ずつ挿入する。
    let char_windows: Vec<Entity> = world
        .query_filtered::<Entity, With<CharWindowMarker>>()
        .iter(world)
        .collect();
    for e in char_windows {
        world.entity_mut(e).insert((
            OnPointerMoved(on_char_pointer_moved),
            OnPointerPressed(on_char_pointer_pressed),
        ));
    }
}

// ---------------------------------------------------------------------------
// ポインタハンドラ（task 2.7・wintf `PointerEventHandler` 署名）
//
// 署名は `fn(&mut World, sender: Entity, entity: Entity, ev: &Phase<PointerState>) -> bool`
// （`wintf::ecs::pointer::PointerEventHandler`）。Bubble 相のみ処理し Tunnel は no-op false
// （伝播続行）。キャラ窓への登録は本モジュールの `attach_char_pointer_handlers`（依存方向
// input_events→placement・main.rs が spawn 直後に呼ぶ）で行う＝stand-in `on_ghost_pressed`
// を退役して `OnPointerMoved`／`OnPointerPressed` へ差し替え。
// ---------------------------------------------------------------------------

/// キャラ窓 `CharWindowMarker.scope`（usize→u32）を取り出す（M1 実値 {0,1} を `debug_assert`）。
///
/// マーカー不在（本来キャラ窓には常在）は `None` を返し、呼び手は no-op で縮退する（panic しない）。
fn char_scope(world: &World, entity: Entity) -> Option<u32> {
    let scope = world.get::<CharWindowMarker>(entity)?.scope;
    debug_assert!(scope <= 1, "M1 の scope は {{0,1}} を想定: {scope}");
    Some(scope as u32)
}

/// (scope, 窓 client 物理 px) → [`HitRegion`] を owned で解決する（DD-IE-9 の借用規律）。
///
/// presenter 借用（`&Emo2Wiring`）と間引き状態（`&mut MouseWiring`）が同じ `&mut World` 上の別
/// NonSend 資源であるため、**presenter を含む解決は共有借用のみで完結させ結果を owned で取り
/// 出してから** `&mut MouseWiring` を取る（送出は呼び手が行う）。`Emo2Wiring` 不在（boot 前／失敗時）
/// は presenter=None ゆえ `resolve_region` が `region: None` へ正常縮退する（RegionSource::Mock は
/// presenter を無視）。呼び手は事前に `MouseWiring` 在を確認済み（self-gating）。
///
/// 返り値には `region` に加えて配信空間の座標 [`HitRegion::surface_point`] が載る（DD-4）。
/// 呼び手はこれを `MouseInput{x,y}` へ載せ、throttle へは受領した client px を渡す（6.8）。
fn resolve_hit_owned(world: &World, scope: u32, x: i64, y: i64) -> HitRegion {
    let wiring = world
        .get_non_send::<MouseWiring>()
        .expect("MouseWiring は呼び手が存在確認済み（self-gating）");
    // 共有借用同士（&MouseWiring と &Emo2Wiring）ゆえ両立する。presenter は Emo2Wiring 不在で None。
    let presenter = world
        .get_non_send::<Emo2Wiring>()
        .map(Emo2Wiring::presenter);
    wiring.resolve_region(presenter, scope, x, y)
}

/// キャラ窓のポインタ移動ハンドラ（Bubble のみ処理・1.1/1.3・5.x）。
///
/// `CharWindowMarker.scope` と `PointerState.client_point`（窓 client 物理 px・前処理せず resolver へ
/// 渡す）を取り、当たり判定を解決し [`plan_mouse_move`] の間引き判定を通して送出条件成立時のみ
/// `KanadeMsg::Mouse(Move)` を送出する。`MouseWiring` 不在（wiring 前）は self-gating no-op（false・
/// trace）。Tunnel 相は伝播続行のため常に false。
///
/// 座標（DD-IE-10 改訂）: 間引きの位置比較は受領した client px のまま（6.8）、配信する
/// `MouseInput{x,y}` は resolver が返した `surface_point`（縮約後サーフェス px・1.8）。
pub(crate) fn on_char_pointer_moved(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    let state = match ev {
        Phase::Tunnel(_) => return false,
        Phase::Bubble(s) => s,
    };

    // self-gating: MouseWiring 不在（wiring 前）は no-op（trace）。
    if world.get_non_send::<MouseWiring>().is_none() {
        tracing::trace!(event = "mouse_moved_no_wiring", "MouseWiring 不在: no-op");
        return false;
    }
    let Some(scope) = char_scope(world, entity) else {
        return false;
    };
    let x = state.client_point.x as i64;
    let y = state.client_point.y as i64;

    // presenter 借用を解いて解決結果を owned で取り出してから &mut MouseWiring を取る（DD-IE-9）。
    let hit = resolve_hit_owned(world, scope, x, y);
    let mut wiring = world
        .get_non_send_mut::<MouseWiring>()
        .expect("MouseWiring は直上で存在確認済み");
    // throttle 比較＝client px（縮約前）／配信＝surface_point（縮約後）。
    wiring.plan_and_send_move(scope, (x, y), hit.surface_point, hit.region)
}

/// キャラ窓のポインタ押下ハンドラ（Bubble のみ処理・1.2/3.3・6.2/6.3・7.1/7.3/7.4）。
///
/// - **Ctrl+左ダブルクリック → 暫定退避**（DD-IE-7・`MouseWiring` 非依存＝ghost boot 失敗時も脱出
///   可能）: 全 `GhostWindowMarker` 窓を despawn し、wintf の window-close funnel（`run()` 復帰→main
///   shutdown→`ForceQuit` 系列）へ委ねる（stand-in 直接経路を新設しない）。true。
///   **暫定退避 — M-dialogue の `\-` メニュー終了完成で退役**。
/// - **左／右ダブルクリック（Ctrl なし）** → 当たり判定を解決し `KanadeMsg::Mouse(DoubleClick{button})`
///   を送出（Left→`MouseButton::Left`・Right→`Right`）。配信座標は resolver が返した `surface_point`
///   （縮約後サーフェス px・1.8・DD-IE-10 改訂）。true。
/// - **中／拡張ボタンのダブルクリック** → 送出しない（OnMouseDoubleClickEx は M2・7.1）。false。
/// - **単発クリック**（`DoubleClick::None`）→ 送出しない（7.3）。false。
/// - The Hand／collisionex／owner-draw 右クリックメニューは実装しない（7.4）。
///
/// Tunnel 相は伝播続行のため常に false。送出系は `MouseWiring` 不在時 self-gating no-op（暫定退避は
/// 上流で処理済みゆえ wiring 非依存）。
pub(crate) fn on_char_pointer_pressed(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    let state = match ev {
        Phase::Tunnel(_) => return false,
        Phase::Bubble(s) => s,
    };

    // 暫定退避（Ctrl+左ダブルクリック・DD-IE-7）は MouseWiring 非依存。既存 stand-in
    // （spawn.rs on_ghost_pressed）と同じ機構で全 GhostWindowMarker 窓を despawn する。
    // 暫定退避 — M-dialogue の `\-` メニュー終了完成で退役。
    if state.ctrl_down && state.double_click == DoubleClick::Left {
        tracing::info!(
            event = "mouse_escape_close",
            "Ctrl+左ダブルクリック（暫定退避）: 全ゴースト窓を閉じる"
        );
        let targets: Vec<Entity> = world
            .query_filtered::<Entity, With<GhostWindowMarker>>()
            .iter(world)
            .collect();
        for e in targets {
            world.despawn(e);
        }
        return true;
    }

    // 送出対象は左／右ダブルクリックのみ。中／拡張ボタン・単発クリックは送出しない（7.1/7.3）。
    let button = match state.double_click {
        DoubleClick::Left => MouseButton::Left,
        DoubleClick::Right => MouseButton::Right,
        // Middle/XButton1/XButton2（M2）・None（単発）→ 送出しない。
        DoubleClick::Middle | DoubleClick::XButton1 | DoubleClick::XButton2 | DoubleClick::None => {
            return false;
        }
    };

    // self-gating: MouseWiring 不在（wiring 前）は no-op（trace）。暫定退避は上で処理済み。
    if world.get_non_send::<MouseWiring>().is_none() {
        tracing::trace!(event = "mouse_pressed_no_wiring", "MouseWiring 不在: no-op");
        return false;
    }
    let Some(scope) = char_scope(world, entity) else {
        return false;
    };
    let x = state.client_point.x as i64;
    let y = state.client_point.y as i64;

    let hit = resolve_hit_owned(world, scope, x, y);
    let mut wiring = world
        .get_non_send_mut::<MouseWiring>()
        .expect("MouseWiring は直上で存在確認済み");
    // 配信座標は surface_point（縮約後サーフェス px・1.8）。クリックは throttle を通らない。
    wiring.send_double_click(scope, hit.surface_point, hit.region, button);
    true
}

#[cfg(test)]
#[path = "input_events_tests.rs"]
mod tests;
