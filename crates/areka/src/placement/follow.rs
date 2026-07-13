//! バルーン追従コンポーネントと窓移動の公開 API。
//!
//! - [`BalloonFollow`]: キャラ窓に付与する追従 Component（配置時確定の暫定 offset・
//!   物理 px・4.4。offset は `ScopePlacement.balloon_offset` の転写）
//! - [`on_char_drag`]: `OnDrag` ハンドラ（mock-shell donor `on_shell_drag` の一般化。
//!   マーカー全走査ではなく `BalloonFollow.balloon` の `WindowHandle` を直接引く）
//! - [`on_balloon_drag`]: バルーン窓の `OnDrag` ハンドラ（4.8・DD16・task 8.3）——
//!   バルーン単独ドラッグの相対位置記憶。`BalloonFollow.offset` を
//!   `balloon_pos − char_pos` へ更新する（キャラ窓は不動）
//! - [`move_window_to`]: R7 公開 API（UI スレッド関数・物理 px スクリーン座標直渡し）
//! - [`DragPositionPolicy`]／[`BottomSnapPolicy`]: bottom 吸着ドラッグ（4.7・
//!   DD15 v2・task 8.2R）の核——「生ドラッグ座標→実窓位置」の純粋写像トレイトと
//!   その bottom 吸着実装。`BottomSnap` キャラ窓は `DragConfig{move_window:false}`
//!   で wndproc 移動を止め、[`on_char_drag`]／[`on_char_drag_end`] が適用済み座標を
//!   **単一ライター**として書く（v1 の事後再釘付けは wndproc と競合し振動→撤去）
//! - [`MonitorSnapshot`]／[`work_area_for_window`]: 全モニタ work area 集合の
//!   Resource と窓中心→モニタ解決の純粋ヘルパ（task 8.1・ポリシーの入力）
//!
//! # 座標単位契約（design U1/U4）
//!
//! 本モジュールの座標はすべて**物理 px**。`WindowPos.position` は wndproc が
//! 実ウィンドウ位置から更新する物理 px であり、ここに DPI 再スケール
//! （`dpi/96` 乗除）を一切挟まない（2026-07-05 の二重スケール欠陥の檻）。
//!
//! # UI スレッド契約（7.1/7.2/7.3）
//!
//! 署名は `&mut World` のみで完結し channel／actor 型を持たない。`&mut World` は
//! wintf の UI スレッド tick 内でのみ到達可能なため、窓操作の UI スレッド専有
//! （7.2）を型で担保する。UI 配送ブリッジ（`spawn_ui`／`UiSender`）との結線は
//! 後続の領分（7.3）。

use bevy_ecs::prelude::*;
use tracing::{debug, warn};
use windows::Win32::UI::WindowsAndMessaging::{SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER};
use wintf::ecs::drag::{DragEndEvent, DragEvent, DraggingState};
use wintf::ecs::layout::{Arrangement, Offset};
use wintf::ecs::pointer::Phase;
use wintf::ecs::window::monitor::Monitor;
use wintf::ecs::{Point, SetWindowPosCommand, SizeI, WindowHandle, WindowPos};

use super::resolver::{Anchor, PointPx, RectPx, SizePx};
use super::spawn::BottomSnap;

// =============================================================================
// DragPositionPolicy（task 8.2R・DD15 v2・4.7）
// =============================================================================

/// 生ドラッグ座標→実窓位置の純粋写像トレイト（DD15 v2・開発者指示 2026-07-11）。
///
/// 「ドラッグ座標管理」（wintf の DraggingState/DragEvent＝カーソル差分の復元）と
/// 「実ウィンドウ位置の算出」を分離する。実装は純粋関数であること——
/// `raw`（生ドラッグ座標＝ドラッグ開始時窓位置＋カーソル差分・物理 px）と
/// 窓寸法・モニタ snapshot だけから実窓位置を返し、World に触れない。
/// 反映段階（`enqueue_window_set_pos`）には**適用済み座標のみ**が渡る＝
/// 事後補正が存在しないため、v1 の wndproc 競合振動は原理的に起きない。
pub trait DragPositionPolicy {
    /// 生ドラッグ座標 `raw` に対する実窓位置を返す（物理 px・純粋）。
    ///
    /// - `size`: 窓寸法（物理 px）。非正値は「寸法不明」を意味する
    /// - `snapshot`: モニタ work area 集合。`None`＝フォールバック経路（未挿入）
    fn resolve(&self, raw: PointPx, size: SizePx, snapshot: Option<&MonitorSnapshot>) -> PointPx;
}

/// bottom 吸着ポリシー（4.7・DD15 v2）: X 素通し・Y＝現在モニタ `bottom − h`。
///
/// Y は **`raw` 位置に置いた窓矩形**の中心が属するモニタ（[`work_area_for_window`]）
/// の work area 下端から live 算出する——イベントごとに引き直すため、モニタを
/// 跨いだら跨いだ先の下端へ自然に再吸着する（4.7 後段）。
///
/// graceful degradation（identity＝`raw` 素通し・panic しない・架空矩形を発明しない）:
/// - `snapshot` 不在: main.rs フォールバック経路は挿入しない設計（8.1 note）。
///   ドラッグ移動イベントごとに発火する経路ゆえ `warn!` は spam——`debug!` に留める
/// - 空 snapshot: [`work_area_for_window`] が `None`
/// - 非正寸法: `WindowPos::default()` の size は `CW_USEDEFAULT`（負のセンチネル）で、
///   `bottom − h` が `i32::MAX` 方向へ暴走するため w/h > 0 のみ吸着する
pub struct BottomSnapPolicy;

impl DragPositionPolicy for BottomSnapPolicy {
    fn resolve(&self, raw: PointPx, size: SizePx, snapshot: Option<&MonitorSnapshot>) -> PointPx {
        let Some(snapshot) = snapshot else {
            debug!("MonitorSnapshot 未挿入（フォールバック経路）のため identity 縮退");
            return raw;
        };
        if size.w <= 0 || size.h <= 0 {
            debug!(?size, "窓寸法が不明（非正）のため identity 縮退");
            return raw;
        }
        let window = RectPx {
            left: raw.x,
            top: raw.y,
            right: raw.x.saturating_add(size.w),
            bottom: raw.y.saturating_add(size.h),
        };
        let Some(wa) = work_area_for_window(snapshot, window) else {
            debug!("空 snapshot のため identity 縮退");
            return raw;
        };
        PointPx {
            x: raw.x,
            // 実 work area／窓寸で溢れない範囲だが、極端入力でも panic しない契約
            // （resolver の saturating 演算と同じ防波堤）
            y: wa.bottom.saturating_sub(size.h),
        }
    }
}

/// 変換 T: 解決済みアンカー＋生位置＋新寸から、アンカー辺を work area 対応辺へ
/// 固定した窓左上位置を返す純粋射影（5 アンカー・task 2.1・Req1.1/1.2/2.1-2.5/3.4/5.4）。
///
/// シェル座標系（アンカー辺基準）→ ウィンドウ座標系（サーフェス寸法基準）の変換 T の
/// 恒常維持を担う純粋関数（World 不可視・物理 px 単一通貨・`saturating_*` 演算で
/// panic しない）。ドラッグ（`policy_mapped_position`）とリサイズ（後続 task の
/// `resize_window_to`）の**両者が同一 T を呼ぶ**ことで座標系変換の二重化を避ける
/// （R1.6）。
///
/// # 射影規則（`wa` 取得成功かつ正寸のとき）
///
/// `wa` は「生位置 `raw` に置いた窓矩形の中心が属するモニタの work area」
/// （[`work_area_for_window`]）。モニタ跨ぎは live 算出＝跨いだ先の対応辺へ再吸着する。
/// - `Bottom`: 既存 [`BottomSnapPolicy::resolve`] へ**委譲**（X 保持・`y = wa.bottom − h`）。
///   再定義しない（Req1.2・bottom は T の一事例）。
/// - `Top`: `x = raw.x`（保持）・`y = wa.top`。
/// - `Left`: `x = wa.left`・`y = raw.y`（保持）。
/// - `Right`: `x = wa.right − w`・`y = raw.y`（保持）。
/// - `Free`: `raw` 素通し（identity・position 再計算なし・Req2.5）。
///
/// # graceful degradation（identity＝`raw` 素通し・panic しない・既存 `BottomSnapPolicy` 流儀）
///
/// - `Free` は常に identity（`wa` 不要・寸法・snapshot を問わない）。
/// - 非正寸（w≤0 or h≤0）は identity＋`debug!`。`wa.right − w`／`wa.bottom − h` が
///   `i32::MAX` 方向へ暴走する前に弾く（Req3.4・`BottomSnapPolicy` の CW_USEDEFAULT
///   センチネル縮退と整合）。
/// - `snapshot` 不在（`None`）／空 snapshot（[`work_area_for_window`] が `None`）は
///   identity＋`debug!`。ドラッグ経路 spam 回避で `warn!` でなく `debug!`（既存流儀）。
///
/// # 不変条件（テストで固定）
///
/// 正寸・snapshot 有効時、適用後の窓のアンカー辺 ≡ work area 対応辺。既にアンカー辺
/// 一致の位置に対しては同値を返す（べき等の基礎・R3.1）。
#[allow(dead_code)] // consumer（resize_window_to／on_char_drag 改修）は後続 task 2.2-2.4 の領分
pub fn project_anchor(
    anchor: Anchor,
    raw: PointPx,
    size: SizePx,
    snapshot: Option<&MonitorSnapshot>,
) -> PointPx {
    // Free: アンカー辺なし＝常に identity（wa 不要・寸法・snapshot を問わない・Req2.5）
    if let Anchor::Free = anchor {
        return raw;
    }

    // Bottom: 既存 BottomSnapPolicy へ全面委譲（再定義しない・Req1.2）。縮退規約
    // （snapshot 不在/空・非正寸）も同ポリシーが所有し、T の bottom 事例＝同値になる
    if let Anchor::Bottom = anchor {
        return BottomSnapPolicy.resolve(raw, size, snapshot);
    }

    // 以下 Top/Left/Right（bottom の一般化）。非正寸は wa.right−w／暴走の前に弾く
    if size.w <= 0 || size.h <= 0 {
        debug!(?size, ?anchor, "窓寸法が不明（非正）のため identity 縮退");
        return raw;
    }
    let Some(snapshot) = snapshot else {
        debug!(?anchor, "MonitorSnapshot 未挿入（フォールバック経路）のため identity 縮退");
        return raw;
    };
    // wa＝生位置に置いた窓矩形の中心が属するモニタの work area（live 算出・跨ぎ再吸着）
    let window = RectPx {
        left: raw.x,
        top: raw.y,
        right: raw.x.saturating_add(size.w),
        bottom: raw.y.saturating_add(size.h),
    };
    let Some(wa) = work_area_for_window(snapshot, window) else {
        debug!(?anchor, "空 snapshot のため identity 縮退");
        return raw;
    };

    match anchor {
        // 上端固定・X 保持（Req2.2）
        Anchor::Top => PointPx { x: raw.x, y: wa.top },
        // 左端固定・Y 保持（Req2.3）
        Anchor::Left => PointPx { x: wa.left, y: raw.y },
        // 右端固定（left_X = wa.right − w）・Y 保持（Req2.4）。極端入力でも panic
        // しない契約で saturating_sub（BottomSnapPolicy の bottom−h と同型の防波堤）
        Anchor::Right => PointPx {
            x: wa.right.saturating_sub(size.w),
            y: raw.y,
        },
        // Bottom／Free は冒頭で return 済み（到達不能・網羅性のための恒等既定）
        Anchor::Bottom | Anchor::Free => raw,
    }
}

/// キャラ窓が保持する現在の解決済みアンカー（drag／resize が読む単一の真実源・4.2/1.4）。
///
/// 全 char 窓へ 1 つだけ付与される 5 値アンカー表現（`Anchor` は 5 値ゆえ、二値
/// `BottomSnap` marker を generalize した後継。単一真実源＝二つ目の格納表現を作らない・
/// Req1.6）。値は spawn 時に `config.alignment` 由来（`ScopePlacement.anchor`＝
/// `Anchor::from_alignment` の解決結果）で焼き込まれ（付与は spawn の領分・task 3.1）、
/// runtime は seriko（本 spec 非所有＝`\![set,alignmenttodesktop]` の routing）が
/// 書き換える。`Changed<Anchored>` がアンカー変化での変換 T 再適用トリガとなる
/// （反応 system は後続 task の領分・本 spec は consumer 契約のみ・Req1.4/4.2）。
///
/// ドラッグ（`on_char_drag`）とリサイズ（`resize_window_to`）の**両者がこの値を読んで**
/// 同一射影 T（`project_anchor`）を呼ぶ——`Free` か否かで wndproc 委譲／単一ライターを
/// 分岐する。
#[allow(dead_code)] // consumer（spawn 付与 task 3.1・on_char_drag 改修 task 2.7・anchor_changed_system task 2.6）は後続 task の領分
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anchored(pub Anchor);

/// キャラ窓に付与するバルーン追従 Component（4.2/4.4/4.8）。
///
/// `offset` の初期値は配置時に確定する暫定 offset（物理 px・
/// `ScopePlacement.balloon_offset` の転写＝P5 幾何の暫定規則。正式な配置規則は
/// balloon 表示系の後続が所有する・4.4）。バルーン単独ドラッグでユーザーが
/// ずらすと [`on_balloon_drag`] が `balloon_pos − char_pos` へ**記憶更新**し、
/// 以後のキャラ窓ドラッグ・[`move_window_to`] は調整後 offset で追従する
/// （4.8・セッション内のみ・永続化は M-life の領分。
/// 旧挙動「次のキャラ窓ドラッグで初期 offset へスナップバック」は
/// 2026-07-11 要件 4.8 により仕様退役）。
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BalloonFollow {
    /// 追従して動かすバルーン窓 entity。
    pub balloon: Entity,
    /// キャラ窓左上からバルーン窓左上への相対 offset（物理 px・配置時確定）。
    pub offset: PointPx,
}

/// `OnDrag` ハンドラ: ドラッグ中のキャラ窓の移動とバルーン追従（4.2/4.3/4.7）。
///
/// 冒頭で `ev.target == entity` を検査し、他 entity 宛イベントには何もしない
/// （ハードニング・DD15 v2 (6)。wintf のドラッグ対象は `DragConfig` を持つ
/// 窓 entity 自身なので、実 flow では常に一致する）。
///
/// # BottomSnap キャラ窓（4.7・DD15 v2・task 8.2R——単一ライター）
///
/// `Bottom`/`Seam` スコープのキャラ窓は `DragConfig { move_window: false }` で
/// spawn され、wndproc は窓を動かさない。本ハンドラが唯一のライターとして、
/// [`DraggingState`]＋DragEvent のカーソル座標から生ドラッグ座標を復元し
/// （[`policy_mapped_position`]）、[`BottomSnapPolicy`] 適用済みの座標を
/// **一度だけ**書く——反映段階で既に正しい座標が確定しているため、v1 の
/// 「wndproc 移動→事後再釘付け」の毎サイクル振動は原理的に起きない。
/// モニタ跨ぎ再吸着はポリシーの live 算出が担う。
///
/// # Free キャラ窓（marker なし・挙動不変）
///
/// `DragConfig { move_window: true }` のまま wndproc レベルで移動済み。
/// 本ハンドラは wndproc が更新した `WindowPos.position`（物理 px）を読むだけで
/// 窓を書かない（wndproc の領分）。
///
/// どちらの経路でも、バルーン追従は**確定後のキャラ窓座標**へ
/// `BalloonFollow.offset` を加算して enqueue する（再スケールなし・U4）。
/// イベントは消費しない（常に `false`＝伝播続行。donor on_shell_drag と同じ規約）。
pub(crate) fn on_char_drag(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<DragEvent>,
) -> bool {
    match ev {
        Phase::Tunnel(_) => false,
        Phase::Bubble(ev) => {
            if ev.target != entity {
                return false;
            }

            let pos = if world.get::<BottomSnap>(entity).is_some() {
                // 単一ライター経路: 生ドラッグ座標→ポリシー適用済み座標を書く
                let Some(mapped) = policy_mapped_position(world, entity, ev.position) else {
                    return false;
                };
                if !enqueue_window_set_pos(world, entity, mapped.x, mapped.y, None) {
                    return false;
                }
                mapped
            } else {
                // Free 経路: wndproc（move_window=true）が移動済みの位置を読むだけ
                let Some(pos) = world.get::<WindowPos>(entity).and_then(|wp| wp.position) else {
                    return false;
                };
                pos
            };

            follow_balloon(world, entity, pos);
            false
        }
    }
}

/// `OnDragEnd` ハンドラ: BottomSnap キャラ窓の最終カーソル位置へ同写像を適用する
/// （4.7・DD15 v2 (3)・task 8.2R）。
///
/// wintf の accumulator は LBUTTONUP で `current_dragging_entity` を先にクリア
/// するため、最終カーソル位置の DragEvent は配送されない（debug 調査 2026-07-11）。
/// この穴を、`dispatch_drag_events` の Ended 分岐が配送する DragEndEvent
/// （最終カーソル位置持ち）で埋める。[`DraggingState`] はハンドラ配送**後**に
/// remove されるため、ここではまだ読める（実 flow 準拠）。
///
/// cancel（ESC 等・`ev.cancelled=true`）も同写像で確定する——move_window=false の
/// 窓は wndproc の巻き戻しが存在せず、吸着不変量（Y=下端）を満たす位置で終える
/// のが 4.7 の意図に最も忠実（M1 簡素化・開始位置への復元は将来領分）。
///
/// spawn（task 8.2R）が BottomSnap キャラ窓にのみ結線する。Free 窓・バルーン窓は
/// wndproc が最終位置まで動かし切るため不要。
pub(crate) fn on_char_drag_end(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<DragEndEvent>,
) -> bool {
    match ev {
        Phase::Tunnel(_) => false,
        Phase::Bubble(ev) => {
            if ev.target != entity {
                return false;
            }
            if world.get::<BottomSnap>(entity).is_none() {
                return false;
            }
            let Some(mapped) = policy_mapped_position(world, entity, ev.position) else {
                return false;
            };
            if !enqueue_window_set_pos(world, entity, mapped.x, mapped.y, None) {
                return false;
            }
            follow_balloon(world, entity, mapped);
            false
        }
    }
}

/// BottomSnap キャラ窓の「カーソル座標→ポリシー適用済み窓位置」（DD15 v2・8.2R）。
///
/// 生ドラッグ座標（＝move_window=true なら wndproc が書いたであろう位置）を
/// wndproc と同じ式で復元する: `initial_window_pos + (cursor − drag_start)`。
/// [`DraggingState`] の `initial_inset` は wintf dispatch が「ドラッグ開始時の
/// 窓位置」を転記したもの（フィールド名は歴史的経緯・dispatch.rs 参照）。
///
/// `None` は「[`DraggingState`] 不在で生座標を復元できない」場合のみ（実 flow では
/// dispatch が DragEvent より先に挿入するため起きない・`debug!` の上で no-op）。
/// 寸法不明・snapshot 不在は [`BottomSnapPolicy`] が identity へ縮退する。
fn policy_mapped_position(world: &World, entity: Entity, cursor: Point) -> Option<Point> {
    let Some(ds) = world.get::<DraggingState>(entity) else {
        debug!(
            ?entity,
            "DraggingState 不在のため生ドラッグ座標を復元できない（写像スキップ）"
        );
        return None;
    };
    let raw = PointPx {
        // 実カーソル・窓座標の範囲で溢れないが、極端入力でも panic しない契約
        x: (ds.initial_inset.0 as i32)
            .saturating_add(cursor.x.saturating_sub(ds.drag_start_pos.x)),
        y: (ds.initial_inset.1 as i32)
            .saturating_add(cursor.y.saturating_sub(ds.drag_start_pos.y)),
    };
    let size = world
        .get::<WindowPos>(entity)
        .and_then(|wp| wp.size)
        .map(|s| SizePx {
            w: s.width,
            h: s.height,
        })
        // 不在は非正寸法（＝寸法不明）としてポリシーの identity 縮退へ委ねる
        .unwrap_or(SizePx { w: 0, h: 0 });
    let mapped = BottomSnapPolicy.resolve(raw, size, world.get_resource::<MonitorSnapshot>());
    Some(Point {
        x: mapped.x,
        y: mapped.y,
    })
}

/// 確定済みキャラ窓座標 `pos` を基準に随伴バルーンを追従させる（4.2・U4）。
///
/// [`BalloonFollow`] が無ければ no-op。[`on_char_drag`]／[`on_char_drag_end`] の
/// 共通後段。
fn follow_balloon(world: &mut World, entity: Entity, pos: Point) {
    let Some(follow) = world.get::<BalloonFollow>(entity).copied() else {
        return;
    };
    // 不変条件: pos は仮想スクリーン座標範囲・offset は配置時確定の
    // 有限値のため、加算が i32 を溢れることはない（溢れは入力源の異常）。
    debug_assert!(
        pos.x.checked_add(follow.offset.x).is_some()
            && pos.y.checked_add(follow.offset.y).is_some(),
        "char window position out of virtual-screen range: {pos:?} + {:?}",
        follow.offset
    );
    enqueue_window_set_pos(
        world,
        follow.balloon,
        pos.x + follow.offset.x,
        pos.y + follow.offset.y,
        None,
    );
}

/// `OnDrag` ハンドラ: バルーン窓単独ドラッグの相対位置記憶（4.8・DD16・task 8.3）。
///
/// バルーン窓自体は `DragConfig { move_window: true }` により wndproc レベルで
/// 移動済み（`WindowPos.position` 更新済み・物理 px）。キャラ窓は不動で、
/// bottom 吸着（4.7）の対象外＝バルーンドラッグでキャラ窓の Y 釘付けは発火しない。
///
/// 本ハンドラは `BalloonFollow.balloon == ドラッグ中バルーン` のキャラ窓を
/// query 走査で逆引きし（窓は高々数個・全走査で十分）、
/// `BalloonFollow.offset = balloon_pos − char_pos`（物理 px・再スケールなし・U4）
/// へ更新するだけ。既存 consumer（[`on_char_drag`]／[`move_window_to`]）は
/// 無改変で調整後 offset を読む（DD16・4.4 の恒等式
/// `balloon_pos − char_pos ≡ offset` は更新後も不変）。記憶はセッション内のみ
/// （永続化 ghost.dat は M-life の領分）。
///
/// イベントは消費しない（常に `false`＝伝播続行。[`on_char_drag`] と同じ規約）。
pub(crate) fn on_balloon_drag(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<DragEvent>,
) -> bool {
    match ev {
        Phase::Tunnel(_) => false,
        Phase::Bubble(ev) => {
            // 他 entity 宛イベントには何もしない（ハードニング・DD15 v2 (6)）
            if ev.target != entity {
                return false;
            }
            // バルーン窓の現在位置（wndproc が実窓位置から更新済み・物理 px）
            let Some(wp) = world.get::<WindowPos>(entity) else {
                return false;
            };
            let Some(balloon_pos) = wp.position else {
                return false;
            };

            // BalloonFollow.balloon == 自バルーンのキャラ窓を逆引きし offset 更新
            let mut chars = world.query::<(&mut BalloonFollow, &WindowPos)>();
            for (mut follow, char_wp) in chars.iter_mut(world) {
                if follow.balloon != entity {
                    continue;
                }
                let Some(char_pos) = char_wp.position else {
                    continue;
                };
                // 不変条件: 両者とも仮想スクリーン座標範囲のため、減算が i32 を
                // 溢れることはない（溢れは入力源の異常・on_char_drag と同じ流儀）
                debug_assert!(
                    balloon_pos.x.checked_sub(char_pos.x).is_some()
                        && balloon_pos.y.checked_sub(char_pos.y).is_some(),
                    "window positions out of virtual-screen range: {balloon_pos:?} - {char_pos:?}"
                );
                follow.offset = PointPx {
                    x: balloon_pos.x - char_pos.x,
                    y: balloon_pos.y - char_pos.y,
                };
            }
            false
        }
    }
}

/// R7 公開 API: UI スレッド上で呼ばれる窓移動関数（物理 px・スクリーン座標直渡し・7.1）。
///
/// - 移動は `SetWindowPosCommand`（`SWP_NOSIZE|SWP_NOZORDER|SWP_NOACTIVATE`）経由。
///   座標は物理 px 素通し（U4・再スケールなし）
/// - 対象が [`BalloonFollow`] を持つ場合はバルーン窓も offset 維持で随伴移動する
/// - 対象不在／`WindowHandle` 未付与（窓生成前）は `warn!` して `false` を返す
///   （silent no-op にしない）。このとき随伴バルーンも動かさない
/// - 随伴バルーン側の `WindowHandle` 未付与は `warn!` のみ（対象自身の移動は成立
///   しているため戻り値は `true`）
#[allow(dead_code)] // 呼び出し側（UI 配送ブリッジ結線）は後続 spec の領分（7.3）
pub fn move_window_to(world: &mut World, window: Entity, x: i32, y: i32) -> bool {
    let follow = world.get::<BalloonFollow>(window).copied();

    if !enqueue_window_set_pos(world, window, x, y, None) {
        return false;
    }

    if let Some(follow) = follow {
        debug_assert!(
            x.checked_add(follow.offset.x).is_some() && y.checked_add(follow.offset.y).is_some(),
            "move target out of virtual-screen range: ({x},{y}) + {:?}",
            follow.offset
        );
        // バルーン側の失敗（WindowHandle 未付与等）は enqueue_window_set_pos が
        // warn! 済み。対象自身の移動は成立しているため true のまま返す。
        enqueue_window_set_pos(world, follow.balloon, x + follow.offset.x, y + follow.offset.y, None);
    }

    true
}

/// 単一ライター反映口: 新しい表示サーフェス寸法に対しアンカー射影 T を再適用し、
/// 確定した position＋size を単一ライター経路で**一度だけ**書く（task 2.4・
/// Req1.1/1.3/1.7/3.1/3.4＋2.6/3.3）。
///
/// `char_window` の現在アンカー（[`Anchored`]＝単一真実源）を読み、[`project_anchor`]
/// で新 position を導出する——ドラッグ（[`policy_mapped_position`]）と**同一の T** を
/// 呼び、座標系変換を二重化しない（Req1.6）。`bottom` は `wa.bottom − h'` の再計算、
/// `snapshot` 不在は `project_anchor` が identity 縮退する。
///
/// # 縮退・失敗経路（log-first・silent failure を作らない）
///
/// - [`Anchored`] 欠落: char 窓は spawn で必ず付与される＝異常系ゆえ `warn!`＋`false`。
/// - 非正寸（w≤0 or h≤0）: T を再適用せず現状保持＋`warn!`＋`false`（Req3.4・
///   [`BottomSnapPolicy`] の非正寸縮退と整合）。
/// - `WindowPos`／`WindowPos.position` 不在（窓生成前の異常系）: 生位置 `raw` を
///   導出できないため `warn!`＋`false`（panic しない）。
/// - べき等（Req3.1）: 導出 `(position, size)` が現 `WindowPos` と同一なら書込を行わず
///   `false`（冗長な再配置を避ける・正常系ゆえ `debug!`）。
/// - `WindowHandle` 未付与/対象不在: [`enqueue_window_set_pos`] が `warn!`＋`false`
///   （Req3.3）。このとき随伴バルーンも動かさない（[`move_window_to`] と同じ流儀）。
///
/// # 不変条件（Req1.5/1.7）
///
/// 位置・サイズは [`enqueue_window_set_pos`]（`Some(new_size)`）で 1 コマンドだけ
/// 発行する——`enqueue_window_move` を迂回する新たな bypass 書込を新設せず、単一
/// ライター規律（bypass ミラー＋Arrangement 同期）を継承する。反映段階で既に確定
/// 座標のみを書くため、切替・アンカー変更で窓が振動しない。書込成功後は
/// [`follow_balloon`] が [`BalloonFollow.offset`] を保って随伴させる（Req2.6・
/// 恒等式 `balloon_pos − char_pos ≡ offset` 維持）。
#[allow(dead_code)] // 呼び出し側（anchor_changed_system task 2.6・frame resnap シーム）は後続 task の領分
pub fn resize_window_to(world: &mut World, char_window: Entity, new_size: SizePx) -> bool {
    // 1. Anchored（drag／resize が読む単一真実源）を読む。char 窓は spawn で必ず
    //    付与される＝欠落は異常系ゆえ log-first で no-op（silent failure にしない）。
    let Some(Anchored(anchor)) = world.get::<Anchored>(char_window).copied() else {
        warn!(
            entity = ?char_window,
            "Anchored 未付与（char 窓は spawn で必ず付与）のため resize しない"
        );
        return false;
    };

    // 2. 非正寸ガード（Req3.4）: T を再適用せず現状保持。wa.right−w／wa.bottom−h の
    //    暴走を先に弾く（BottomSnapPolicy の CW_USEDEFAULT センチネル縮退と整合）。
    if new_size.w <= 0 || new_size.h <= 0 {
        warn!(
            entity = ?char_window,
            ?new_size,
            "新しいサーフェス寸法が非正のため T 再適用せず現状保持"
        );
        return false;
    }

    // 3. 現在位置（生位置 raw）と現寸を読む。WindowPos／position 不在（窓生成前の
    //    異常系）は raw を作れないため安全に no-op（panic しない・log-first）。
    let (raw, current_size) = {
        let Some(wp) = world.get::<WindowPos>(char_window) else {
            warn!(
                entity = ?char_window,
                "WindowPos 未付与（窓生成前）のため raw を導出できず resize しない"
            );
            return false;
        };
        let Some(pos) = wp.position else {
            warn!(
                entity = ?char_window,
                "WindowPos.position 不在（窓生成前）のため raw を導出できず resize しない"
            );
            return false;
        };
        (PointPx { x: pos.x, y: pos.y }, wp.size)
    };

    // 新位置 = アンカー射影 T（bottom は wa.bottom−h' 再計算・snapshot 不在は
    // project_anchor が identity 縮退）。drag と同一 T を呼び二重化しない（Req1.6）。
    let snapshot = world.get_resource::<MonitorSnapshot>();
    let new_pos = project_anchor(anchor, raw, new_size, snapshot);

    // 4. べき等 skip（Req3.1）: 導出 (position, size) が現 WindowPos と同一なら書かない
    //    （冗長な再配置を避ける・こちらは正常系ゆえ debug!）。
    if new_pos == raw && current_size == Some(SizeI::new(new_size.w, new_size.h)) {
        debug!(
            entity = ?char_window,
            ?new_pos,
            ?new_size,
            "導出 position/size が現在値と同一のため書込をスキップ（べき等）"
        );
        return false;
    }

    // 5. 一度書き（Req1.5/1.7）: 位置＋サイズを単一ライター経路で 1 コマンド発行。
    //    WindowHandle 未付与/不在は enqueue が warn!＋false（Req3.3）——false なら
    //    随伴バルーンも動かさず false を返す（move_window_to と同じ流儀）。
    if !enqueue_window_set_pos(world, char_window, new_pos.x, new_pos.y, Some(new_size)) {
        return false;
    }

    // 6. 随伴バルーン維持（Req2.6）: 確定後キャラ窓座標＋offset で追従（offset 恒等式維持）。
    follow_balloon(
        world,
        char_window,
        Point {
            x: new_pos.x,
            y: new_pos.y,
        },
    );

    true
}

/// 1 窓ぶんの位置（と任意で寸法）を enqueue する共通経路（物理 px 素通し・
/// 単一ライター・task 2.3 で move 専用から size 対応へ一般化）。
///
/// `WindowHandle` を直接引いて `SetWindowPosCommand` を enqueue し、ECS 側の
/// `WindowPos.position` を `bypass_change_detection()` で先行反映する。
///
/// # size 引数（`None`＝移動専用の後方互換／`Some`＝位置＋寸を一度に反映）
///
/// - `None`: 移動専用。flags は `SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE`・
///   width/height=0 で、`WindowPos.position` のみ bypass ミラーし `WindowPos.size`
///   は**触らない**（`move_window_to`／drag 経路の従来挙動と完全に同一）。
/// - `Some(s)`: flags から `SWP_NOSIZE` を外し（＝`SWP_NOZORDER | SWP_NOACTIVATE`）、
///   `width=s.w`／`height=s.h` を渡す。`WindowPos.position` に加え `WindowPos.size`
///   を `SizeI::new(s.w, s.h)` で bypass ミラーする（`resize_window_to` の反映口）。
///
/// `size.is_some()` で明示分岐して flags/寸法を合成し、`SWP_NOSIZE` の付け外しミスを
/// 防ぐ（Req1.5：本経路を迂回する第二の bypass 書込経路を新設しない）。
///
/// bypass の理由: 実アプリでは flush 後の `SetWindowPos` が同期発火させる
/// `WM_WINDOWPOSCHANGED` echo が同値を（同じく bypass で）再書込するため、
/// ここで `Changed<WindowPos>` を発火させると `apply_window_pos_changes` が
/// 別フラグの `SetWindowPos` を二重発行してしまう。bypass なら発行は本関数の
/// 1 コマンドに閉じ、headless World（echo が来ない）でも `WindowPos` が
/// 期待座標・寸法を示す決定論シームになる。
///
/// # Arrangement.offset の直接同期（task 8.3-fix・4.8 実機ブロッカ）
///
/// bypass 書込は `Changed<WindowPos>` を発火させないため、wintf の
/// `sync_window_arrangement_from_window_pos` は本経路の移動を拾えない。放置すると
/// `GlobalArrangement`（αマスクヒットテストの境界）が spawn 位置に取り残され、
/// 移動後のバルーンがクリック死する（実機で確認された 4.8 ブロッカ）。wintf 自身が
/// ドラッグ対象窓の DragEnd で行う直接同期（drag/dispatch.rs
/// 「[DragEnd] Direct Arrangement.offset sync」＝`WindowPos.position` を `as f32`
/// 転写・同値ガード付き）と同じパターンを、本経路で動かした窓にも適用する。
/// `Changed<Arrangement>` は発火する（GA 再計算に必要）が、ゴースト窓の
/// `GlobalArrangement.bounds` は零寸のため `window_pos_sync_system` の
/// `width <= 0` ガードが skip し、`SetWindowPos` echo ループにはならない
/// （donor の DragEnd 同期と同じ性質）。
fn enqueue_window_set_pos(
    world: &mut World,
    window: Entity,
    x: i32,
    y: i32,
    size: Option<SizePx>,
) -> bool {
    let Some(handle) = world.get::<WindowHandle>(window).copied() else {
        warn!(
            entity = ?window,
            x, y,
            "移動対象窓が不在か WindowHandle 未付与（生成前）のため移動しない"
        );
        return false;
    };

    // size 有無で flags と width/height を明示分岐（SWP_NOSIZE の付け外しミスを防ぐ）。
    // None＝移動専用（SWP_NOSIZE 付・後方互換）／Some＝位置＋寸（SWP_NOSIZE 外し）。
    let (flags, w, h) = match size {
        Some(s) => (SWP_NOZORDER | SWP_NOACTIVATE, s.w, s.h),
        None => (SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE, 0, 0),
    };

    SetWindowPosCommand::enqueue(SetWindowPosCommand::new(
        handle.hwnd,
        x,
        y,
        w,
        h,
        flags,
        None,
    ));

    match world.get_mut::<WindowPos>(window) {
        Some(mut wp) => {
            let wp = wp.bypass_change_detection();
            wp.position = Some(Point { x, y });
            // Some のときのみ寸法もミラー（None は size を触らない＝移動専用の後方互換）
            if let Some(s) = size {
                wp.size = Some(SizeI::new(s.w, s.h));
            }
        }
        None => {
            debug!(
                entity = ?window,
                "WindowPos 未付与のため ECS 側ミラー更新はスキップ（コマンドは enqueue 済み）"
            );
        }
    }

    // wintf DragEnd donor と同型の直接同期（doc コメント参照）。同値なら
    // 書かない（Deref 読みのみ＝Changed<Arrangement> を発火させない）。
    let new_offset = Offset {
        x: x as f32,
        y: y as f32,
    };
    match world.get_mut::<Arrangement>(window) {
        Some(mut arr) => {
            if arr.offset != new_offset {
                arr.offset = new_offset;
            }
        }
        None => {
            debug!(
                entity = ?window,
                "Arrangement 未付与のため GA ヒットテスト境界の同期はスキップ"
            );
        }
    }

    true
}

// =============================================================================
// MonitorSnapshot（task 8.1・DD15 基盤・4.7）
// =============================================================================

/// 全モニタの work area 集合（物理 px・起動時取得のセッション内固定 snapshot・DD15）。
///
/// 起動時に seam（main.rs）／example が [`MonitorSnapshot::from_monitors`] で実モニタ
/// から忠実転写して Resource 挿入し、bottom 吸着ドラッグ（task 8.2）が
/// [`work_area_for_window`] で「窓が現在属するモニタの work area」を引くのに使う。
/// snapshot はセッション内固定＝M1 受容（`WM_DISPLAYCHANGE` 追随は後続・DD15）。
/// 中身は `RectPx` のみの純粋データで、headless テストは合成値を直接構築して
/// 注入する（偽装境界・wintf に触れるのは挿入サイトだけ）。
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct MonitorSnapshot {
    /// モニタ列挙順の work area（物理 px）。
    pub work_areas: Vec<RectPx>,
}

impl MonitorSnapshot {
    /// 実モニタ列挙結果から全 work area（`RECT`・物理 px）を列挙順のまま
    /// **単位変換なしで忠実転写**する（mod.rs `primary_work_area` と同じ U 契約:
    /// どちらも物理 px 通貨）。0 台は空 snapshot（panic しない・消費側が
    /// [`work_area_for_window`] の `None` で防御）。
    pub fn from_monitors(monitors: &[Monitor]) -> Self {
        Self {
            work_areas: monitors
                .iter()
                .map(|m| RectPx {
                    left: m.work_area.left,
                    top: m.work_area.top,
                    right: m.work_area.right,
                    bottom: m.work_area.bottom,
                })
                .collect(),
        }
    }
}

/// 窓矩形の中心が属するモニタの work area を引く純粋ヘルパ（4.7・DD15）。
///
/// 決定論規則（テストで固定）:
/// - 中心は `((left+right)/2, (top+bottom)/2)`（i64 演算・ゼロ方向切り捨て）
/// - 帰属判定は half-open（`left ≤ cx < right`・`top ≤ cy < bottom`）＝共有辺上の
///   中心は右／下側のモニタへ属する。複数矩形が含む（重複）場合は昇順 index 先勝ち
/// - どのモニタにも属さない場合は最近傍（中心→矩形 clamp 点の自乗距離最小・
///   等距離は昇順 index 先勝ち＝`min_by_key` の先頭優先）
/// - 空 snapshot は `None`（架空の既定矩形を発明しない・resolver と同方針）
///
/// 距離は i128 で自乗和を取り、極端な仮想スクリーン座標でも溢れない
/// （panic しない契約・resolver の saturating 演算と同じ防波堤）。
pub fn work_area_for_window(snapshot: &MonitorSnapshot, window: RectPx) -> Option<RectPx> {
    let cx = (window.left as i64 + window.right as i64) / 2;
    let cy = (window.top as i64 + window.bottom as i64) / 2;

    // 帰属（half-open）・昇順 index 先勝ち
    if let Some(wa) = snapshot.work_areas.iter().find(|wa| {
        (wa.left as i64) <= cx
            && cx < (wa.right as i64)
            && (wa.top as i64) <= cy
            && cy < (wa.bottom as i64)
    }) {
        return Some(*wa);
    }

    // どこにも属さない → 最近傍（clamp 点との自乗距離最小・等距離は先勝ち）
    snapshot
        .work_areas
        .iter()
        .min_by_key(|wa| {
            // `i64::clamp` は逆転区間（万一の退化矩形）で panic するため min/max で書く
            // （resolver `clamp_axis` と同じ非 panic 流儀）
            let px = cx.min(wa.right as i64).max(wa.left as i64);
            let py = cy.min(wa.bottom as i64).max(wa.top as i64);
            let dx = (cx - px) as i128;
            let dy = (cy - py) as i128;
            dx * dx + dy * dy
        })
        .copied()
}

// =============================================================================
// Tests（TDD RED: 実装前に振る舞いを固定する）
// =============================================================================

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use bevy_ecs::prelude::*;
    use windows::Win32::Foundation::{HINSTANCE, HWND};
    use wintf::ecs::drag::DragEvent;
    use wintf::ecs::pointer::Phase;
    use wintf::ecs::{Point, WindowHandle, WindowPos};

    use super::{BalloonFollow, move_window_to, on_char_drag};
    use crate::placement::resolver::PointPx;

    // -------------------------------------------------------------------------
    // テストヘルパ（偽装境界: 実 HWND なしの headless World で決定論検証する。
    // SetWindowPosCommand は TLS キューへの enqueue のみで flush しないため、
    // 偽 HWND に対する実 SetWindowPos は一切呼ばれない——wintf 自身の
    // window_pos_systems_test と同じ流儀）
    // -------------------------------------------------------------------------

    /// 偽 HWND の WindowHandle（実窓なし・headless 決定論シーム）。
    fn fake_handle(raw: usize) -> WindowHandle {
        WindowHandle {
            hwnd: HWND(raw as *mut _),
            instance: HINSTANCE::default(),
        }
    }

    /// position 初期値付きの WindowPos。
    fn window_pos_at(x: i32, y: i32) -> WindowPos {
        WindowPos {
            position: Some(Point { x, y }),
            ..Default::default()
        }
    }

    /// entity の WindowPos.position を読む（未設定は panic で検出）。
    fn position_of(world: &World, entity: Entity) -> Point {
        world
            .get::<WindowPos>(entity)
            .expect("WindowPos があるはず")
            .position
            .expect("position があるはず")
    }

    fn drag_event(target: Entity) -> DragEvent {
        DragEvent {
            target,
            start_position: Point::new(0, 0),
            position: Point::new(10, 10),
            is_primary: true,
            timestamp: Instant::now(),
        }
    }

    // -------------------------------------------------------------------------
    // move_window_to（R7 公開 API・7.1/7.2/7.3・U4）
    // -------------------------------------------------------------------------

    /// 観測可能な完了状態: headless World 上で move_window_to を呼ぶと
    /// 対象窓の WindowPos が期待座標へ更新される（物理 px 素通し・U4）。
    /// 座標は 96 の倍数を避けた値を使い、隠れた dpi/96 再スケールがあれば
    /// 完全一致が崩れる檻とする（07-05 欠陥の再発防止・3.2/3.3）。
    #[test]
    fn move_window_to_updates_window_pos_physical_px() {
        let mut world = World::new();
        let window = world
            .spawn((fake_handle(0x1234), window_pos_at(10, 20)))
            .id();

        assert!(move_window_to(&mut world, window, 1531, 883));
        assert_eq!(position_of(&world, window), Point { x: 1531, y: 883 });
    }

    /// WindowHandle 未付与（窓生成前）は false を返し、位置も変更しない。
    #[test]
    fn move_window_to_without_handle_returns_false() {
        let mut world = World::new();
        let window = world.spawn(window_pos_at(10, 20)).id();

        assert!(!move_window_to(&mut world, window, 500, 600));
        assert_eq!(position_of(&world, window), Point { x: 10, y: 20 });
    }

    /// despawn 済み（対象不在）の entity も false（silent no-op にしない・panic しない）。
    #[test]
    fn move_window_to_on_despawned_entity_returns_false() {
        let mut world = World::new();
        let window = world
            .spawn((fake_handle(0x1234), window_pos_at(0, 0)))
            .id();
        world.despawn(window);

        assert!(!move_window_to(&mut world, window, 100, 200));
    }

    /// BalloonFollow を持つ対象の移動はバルーンも offset 維持で随伴移動する
    /// （T-I4: 移動後も balloon_pos − char_pos ≡ offset が保存される）。
    #[test]
    fn move_window_to_moves_balloon_with_offset_preserved() {
        let mut world = World::new();
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
            .id();
        let offset = PointPx { x: -412, y: -25 };
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_at(50, 60),
                BalloonFollow { balloon, offset },
            ))
            .id();

        assert!(move_window_to(&mut world, window, 907, 1201));

        let char_pos = position_of(&world, window);
        let balloon_pos = position_of(&world, balloon);
        assert_eq!(char_pos, Point { x: 907, y: 1201 });
        assert_eq!(
            balloon_pos,
            Point {
                x: 907 + offset.x,
                y: 1201 + offset.y
            }
        );
        // offset 保存則（balloon_pos − char_pos ≡ offset）
        assert_eq!(balloon_pos.x - char_pos.x, offset.x);
        assert_eq!(balloon_pos.y - char_pos.y, offset.y);
    }

    /// 対象自身に WindowHandle が無ければ false で、バルーンも動かさない。
    #[test]
    fn move_window_to_target_without_handle_does_not_move_balloon() {
        let mut world = World::new();
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(70, 80)))
            .id();
        let window = world
            .spawn((
                window_pos_at(50, 60),
                BalloonFollow {
                    balloon,
                    offset: PointPx { x: 11, y: 22 },
                },
            ))
            .id();

        assert!(!move_window_to(&mut world, window, 907, 1201));
        assert_eq!(position_of(&world, window), Point { x: 50, y: 60 });
        assert_eq!(position_of(&world, balloon), Point { x: 70, y: 80 });
    }

    /// バルーン側に WindowHandle が無い場合: 対象の移動自体は成功（true）し、
    /// バルーンは動かない（warn ログ・silent failure ではない）。
    #[test]
    fn move_window_to_balloon_without_handle_still_moves_target() {
        let mut world = World::new();
        let balloon = world.spawn(window_pos_at(70, 80)).id();
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_at(50, 60),
                BalloonFollow {
                    balloon,
                    offset: PointPx { x: 11, y: 22 },
                },
            ))
            .id();

        assert!(move_window_to(&mut world, window, 907, 1201));
        assert_eq!(position_of(&world, window), Point { x: 907, y: 1201 });
        assert_eq!(position_of(&world, balloon), Point { x: 70, y: 80 });
    }

    // -------------------------------------------------------------------------
    // MonitorSnapshot / work_area_for_window（task 8.1・DD15 基盤・4.7）
    // -------------------------------------------------------------------------

    use super::{MonitorSnapshot, work_area_for_window};
    use crate::placement::resolver::RectPx;

    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RectPx {
        RectPx {
            left,
            top,
            right,
            bottom,
        }
    }

    /// 複数モニタ: 窓中心が属するモニタの work area が返る（縦位置・寸法の異なる
    /// 2 面で中心帰属を固定する）。
    #[test]
    fn work_area_for_window_picks_monitor_containing_center() {
        let snapshot = MonitorSnapshot {
            work_areas: vec![
                rect(0, 0, 1920, 1040),       // primary
                rect(1920, -213, 4480, 1227), // 右の高解像度モニタ（負 top）
            ],
        };
        // 中心 (2500, 500) → 右モニタ
        let window = rect(2100, 100, 2900, 900);
        assert_eq!(
            work_area_for_window(&snapshot, window),
            Some(rect(1920, -213, 4480, 1227))
        );
        // 中心 (960, 520) → primary
        let window = rect(660, 220, 1260, 820);
        assert_eq!(
            work_area_for_window(&snapshot, window),
            Some(rect(0, 0, 1920, 1040))
        );
    }

    /// 負座標モニタ（プライマリの左）でも中心帰属が成立する。
    #[test]
    fn work_area_for_window_handles_negative_coords() {
        let snapshot = MonitorSnapshot {
            work_areas: vec![rect(0, 0, 1920, 1040), rect(-1920, -40, 0, 1000)],
        };
        let window = rect(-1500, 100, -700, 700); // 中心 (-1100, 400)
        assert_eq!(
            work_area_for_window(&snapshot, window),
            Some(rect(-1920, -40, 0, 1000))
        );
    }

    /// 境界中心の決定論: 帰属判定は half-open（right/bottom 排他）＝共有辺上の中心は
    /// 右隣モニタへ属する。複数矩形が同一中心を含む（重複）場合は昇順 index 先勝ち。
    #[test]
    fn work_area_for_window_boundary_center_is_half_open_and_first_match_wins() {
        let a = rect(0, 0, 1920, 1040);
        let b = rect(1920, 0, 3840, 1040);
        let snapshot = MonitorSnapshot {
            work_areas: vec![a, b],
        };
        // 中心 x=1920 ちょうど（共有辺）→ a の right は排他ゆえ b
        let window = rect(1520, 220, 2320, 820); // 中心 (1920, 520)
        assert_eq!(work_area_for_window(&snapshot, window), Some(b));

        // 重複 2 面が同一中心を含む → 先勝ち（昇順 index）
        let overlap = MonitorSnapshot {
            work_areas: vec![a, rect(-10, -10, 2000, 1100)],
        };
        let window = rect(700, 300, 1300, 700); // 中心 (1000, 500) は両方に属す
        assert_eq!(work_area_for_window(&overlap, window), Some(a));
    }

    /// どのモニタにも属さない中心 → 最近傍（中心→矩形 clamp 点の自乗距離最小・
    /// 等距離は昇順 index 先勝ち）。
    #[test]
    fn work_area_for_window_off_all_monitors_returns_nearest() {
        let a = rect(0, 0, 1920, 1040);
        let b = rect(1920, 0, 3840, 1040);
        let snapshot = MonitorSnapshot {
            work_areas: vec![a, b],
        };
        // 中心 (4340, 500): b の右外 500px・a の右外 2420px → b
        let window = rect(4040, 200, 4640, 800);
        assert_eq!(work_area_for_window(&snapshot, window), Some(b));
        // 中心 (-1000, 2000): a の clamp 点 (0,1040) が b の (1920,1040) より近い → a
        let window = rect(-1300, 1700, -700, 2300);
        assert_eq!(work_area_for_window(&snapshot, window), Some(a));
        // 等距離: 中心 (1920, 2000) は a clamp (1920,1040)・b clamp (1920,1040) と
        // 同距離 → 先勝ちで a
        let window = rect(1620, 1700, 2220, 2300);
        assert_eq!(work_area_for_window(&snapshot, window), Some(a));
    }

    /// 空 snapshot → `None`（架空の既定矩形を発明しない）。
    #[test]
    fn work_area_for_window_empty_snapshot_is_none() {
        let snapshot = MonitorSnapshot { work_areas: vec![] };
        assert_eq!(work_area_for_window(&snapshot, rect(0, 0, 100, 100)), None);
    }

    // -------------------------------------------------------------------------
    // on_char_drag（4.2/4.3/4.4・U4）
    // -------------------------------------------------------------------------

    /// Tunnel フェーズは無視する（donor on_shell_drag と同じ規約）。
    #[test]
    fn on_char_drag_tunnel_phase_is_ignored() {
        let mut world = World::new();
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(70, 80)))
            .id();
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_at(50, 60),
                BalloonFollow {
                    balloon,
                    offset: PointPx { x: 11, y: 22 },
                },
            ))
            .id();

        let ev = Phase::Tunnel(drag_event(window));
        assert!(!on_char_drag(&mut world, window, window, &ev));
        assert_eq!(position_of(&world, balloon), Point { x: 70, y: 80 });
    }

    /// Bubble フェーズ: キャラ窓の WindowPos（wndproc 更新済み想定・物理 px）に
    /// offset を加算した位置へバルーンが追従する。再スケールなしの檻として
    /// 96 の倍数を避けた座標で完全一致を要求する（U4・3.3）。
    #[test]
    fn on_char_drag_bubble_moves_balloon_by_offset() {
        let mut world = World::new();
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
            .id();
        let offset = PointPx { x: 498, y: -37 };
        // wndproc がドラッグ中に更新した後のキャラ窓位置を模す
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_at(1207, 653),
                BalloonFollow { balloon, offset },
            ))
            .id();

        let ev = Phase::Bubble(drag_event(window));
        // donor 同様、イベントは消費しない（伝播続行＝false）
        assert!(!on_char_drag(&mut world, window, window, &ev));

        assert_eq!(
            position_of(&world, balloon),
            Point {
                x: 1207 + offset.x,
                y: 653 + offset.y
            }
        );
        // キャラ窓自体はハンドラでは動かさない（wndproc の領分）
        assert_eq!(position_of(&world, window), Point { x: 1207, y: 653 });
    }

    /// キャラ窓に WindowPos（position）が無ければ何もしない（false・panic なし）。
    #[test]
    fn on_char_drag_without_window_pos_is_noop() {
        let mut world = World::new();
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(70, 80)))
            .id();
        let window = world
            .spawn((
                fake_handle(0x1000),
                BalloonFollow {
                    balloon,
                    offset: PointPx { x: 11, y: 22 },
                },
            ))
            .id();

        let ev = Phase::Bubble(drag_event(window));
        assert!(!on_char_drag(&mut world, window, window, &ev));
        assert_eq!(position_of(&world, balloon), Point { x: 70, y: 80 });
    }

    /// BalloonFollow の無い entity への Bubble は no-op（false・panic なし）。
    #[test]
    fn on_char_drag_without_balloon_follow_is_noop() {
        let mut world = World::new();
        let window = world
            .spawn((fake_handle(0x1000), window_pos_at(50, 60)))
            .id();

        let ev = Phase::Bubble(drag_event(window));
        assert!(!on_char_drag(&mut world, window, window, &ev));
        assert_eq!(position_of(&world, window), Point { x: 50, y: 60 });
    }

    // -------------------------------------------------------------------------
    // DragPositionPolicy / BottomSnapPolicy（task 8.2R・4.7・DD15 v2）
    // 純粋写像の単体檻: X 素通し・Y 釘付け・モニタ別 live 算出・identity 縮退。
    // -------------------------------------------------------------------------

    use wintf::ecs::SizeI;
    use wintf::ecs::drag::{DragEndEvent, DraggingState};

    use super::{BottomSnapPolicy, DragPositionPolicy, on_char_drag_end};
    use crate::placement::resolver::SizePx;
    use crate::placement::spawn::BottomSnap;

    /// emo2 scope0 実寸のキャラ窓寸法（物理 px）。
    const CHAR_SIZE: SizePx = SizePx { w: 434, h: 687 };

    /// 単一モニタの合成 snapshot（物理 px・96 の倍数を避けた下端で再スケール檻）。
    fn single_monitor_snapshot() -> MonitorSnapshot {
        MonitorSnapshot {
            work_areas: vec![rect(0, 0, 1920, 1043)],
        }
    }

    /// ポリシー単体: X 素通し・Y=work_area.bottom−h（4.7・純粋写像）。
    #[test]
    fn bottom_snap_policy_pins_y_and_passes_x_through() {
        let snapshot = single_monitor_snapshot();
        let mapped =
            BottomSnapPolicy.resolve(PointPx { x: 1207, y: 217 }, CHAR_SIZE, Some(&snapshot));
        assert_eq!(mapped, PointPx { x: 1207, y: 1043 - 687 });
        // 既に下端一致なら不動点（釘付け済み座標は変わらない）
        assert_eq!(
            BottomSnapPolicy.resolve(mapped, CHAR_SIZE, Some(&snapshot)),
            mapped
        );
    }

    /// ポリシー単体: raw 位置の窓中心が属するモニタの下端で live 算出
    /// （モニタごとに異なる下端へ写る＝跨ぎ再吸着の核・4.7）。
    #[test]
    fn bottom_snap_policy_resolves_per_monitor() {
        let snapshot = MonitorSnapshot {
            work_areas: vec![
                rect(0, 0, 1920, 1040),       // primary
                rect(1920, -213, 4480, 1227), // 右の高解像度モニタ（下端が異なる）
            ],
        };
        // 中心 x=2700+217=2917 → 右モニタ → Y=1227−687=540
        assert_eq!(
            BottomSnapPolicy.resolve(PointPx { x: 2700, y: 353 }, CHAR_SIZE, Some(&snapshot)),
            PointPx { x: 2700, y: 540 }
        );
        // 中心 x=1000+217=1217 → primary → Y=1040−687=353
        assert_eq!(
            BottomSnapPolicy.resolve(PointPx { x: 1000, y: 900 }, CHAR_SIZE, Some(&snapshot)),
            PointPx { x: 1000, y: 353 }
        );
    }

    /// ポリシー単体: snapshot 不在／空・非正寸法（CW_USEDEFAULT センチネル含む）は
    /// identity 縮退（graceful・panic しない・架空矩形を発明しない）。
    #[test]
    fn bottom_snap_policy_degrades_to_identity() {
        let raw = PointPx { x: 1207, y: 217 };
        // snapshot 不在（main.rs フォールバック経路）
        assert_eq!(BottomSnapPolicy.resolve(raw, CHAR_SIZE, None), raw);
        // 空 snapshot
        let empty = MonitorSnapshot { work_areas: vec![] };
        assert_eq!(BottomSnapPolicy.resolve(raw, CHAR_SIZE, Some(&empty)), raw);
        // 非正寸法（saturating_sub が i32::MAX へ飛ぶ暴走の檻）
        let snapshot = single_monitor_snapshot();
        for size in [
            SizePx { w: 0, h: 687 },
            SizePx { w: 434, h: 0 },
            SizePx {
                w: i32::MIN,
                h: i32::MIN,
            },
        ] {
            assert_eq!(BottomSnapPolicy.resolve(raw, size, Some(&snapshot)), raw);
        }
    }

    // -------------------------------------------------------------------------
    // project_anchor（変換 T・task 2.1・4.2/DD15・Req1.1/1.2/2.1-2.5/3.1/3.4/5.4）
    // 5 アンカー射影の純粋檻: アンカー辺固定・非アンカー軸保持・Bottom 委譲・
    // Free identity・縮退・モニタ跨ぎ live 算出・べき等の不動点。
    // 座標・work area 辺は 96 の非倍数を含め、隠れた dpi/96 再スケールの檻とする。
    // -------------------------------------------------------------------------

    use super::project_anchor;
    use crate::placement::resolver::Anchor;

    /// 全 4 辺が 96 の非倍数の単一モニタ snapshot（各アンカー辺再計算の再スケール檻）。
    /// left=53・top=37・right=1877・bottom=1043（いずれも 96 で割り切れない・非零原点）。
    fn odd_edge_snapshot() -> MonitorSnapshot {
        MonitorSnapshot {
            work_areas: vec![rect(53, 37, 1877, 1043)],
        }
    }

    /// #1 Bottom: X 保持・Y=wa.bottom−h。既存 `BottomSnapPolicy` へ委譲し再定義しない
    /// ——同一入力で `BottomSnapPolicy.resolve` と**同値**（再利用の証明・Req1.2/2.1）。
    #[test]
    fn project_anchor_bottom_delegates_to_bottom_snap_policy() {
        let snapshot = odd_edge_snapshot();
        // 中心 (700+217, 300+343)=(917, 643) は単一モニタ内
        let raw = PointPx { x: 700, y: 300 };
        let mapped = project_anchor(Anchor::Bottom, raw, CHAR_SIZE, Some(&snapshot));
        assert_eq!(mapped, PointPx { x: 700, y: 1043 - 687 }, "X 保持・Y=下端−h");
        assert_eq!(
            mapped,
            BottomSnapPolicy.resolve(raw, CHAR_SIZE, Some(&snapshot)),
            "Bottom は BottomSnapPolicy と同値（再定義しない）"
        );
    }

    /// #1 Top: X 保持・Y=wa.top（96 非倍数の top で再計算を固定・Req2.2）。
    #[test]
    fn project_anchor_top_pins_top_edge_and_keeps_x() {
        let snapshot = odd_edge_snapshot();
        let raw = PointPx { x: 700, y: 300 };
        assert_eq!(
            project_anchor(Anchor::Top, raw, CHAR_SIZE, Some(&snapshot)),
            PointPx { x: 700, y: 37 }
        );
    }

    /// #1 Left: X=wa.left・Y 保持（96 非倍数の left で再計算を固定・Req2.3）。
    #[test]
    fn project_anchor_left_pins_left_edge_and_keeps_y() {
        let snapshot = odd_edge_snapshot();
        let raw = PointPx { x: 700, y: 300 };
        assert_eq!(
            project_anchor(Anchor::Left, raw, CHAR_SIZE, Some(&snapshot)),
            PointPx { x: 53, y: 300 }
        );
    }

    /// #1 Right: X=wa.right−w・Y 保持（96 非倍数の right で再計算を固定・Req2.4）。
    #[test]
    fn project_anchor_right_pins_right_edge_and_keeps_y() {
        let snapshot = odd_edge_snapshot();
        let raw = PointPx { x: 700, y: 300 };
        assert_eq!(
            project_anchor(Anchor::Right, raw, CHAR_SIZE, Some(&snapshot)),
            PointPx { x: 1877 - 434, y: 300 }
        );
    }

    /// #1/#2 Free: raw 素通し（identity・position 再計算なし・Req2.5）。snapshot 有無・
    /// 寸法（非正含む）を問わず常に identity。
    #[test]
    fn project_anchor_free_is_always_identity() {
        let snapshot = odd_edge_snapshot();
        let raw = PointPx { x: 700, y: 300 };
        assert_eq!(
            project_anchor(Anchor::Free, raw, CHAR_SIZE, Some(&snapshot)),
            raw,
            "snapshot 有・正寸でも Free は identity"
        );
        assert_eq!(
            project_anchor(Anchor::Free, raw, CHAR_SIZE, None),
            raw,
            "snapshot 不在でも identity"
        );
        assert_eq!(
            project_anchor(Anchor::Free, raw, SizePx { w: 0, h: 0 }, Some(&snapshot)),
            raw,
            "非正寸でも Free は identity（寸法を問わない）"
        );
        assert_eq!(
            project_anchor(
                Anchor::Free,
                raw,
                SizePx {
                    w: i32::MIN,
                    h: i32::MIN,
                },
                None,
            ),
            raw,
        );
    }

    /// #2 縮退（Req3.4）: Bottom/Top/Left/Right とも snapshot 不在(None)/空・非正寸
    /// （0・負・i32::MIN）で identity 縮退（`BottomSnapPolicy` の非正寸縮退と整合・
    /// `wa.right−w`／`wa.bottom−h` の暴走を先に弾く檻・panic しない）。
    #[test]
    fn project_anchor_degrades_to_identity_on_missing_snapshot_or_nonpositive_size() {
        let raw = PointPx { x: 700, y: 300 };
        let empty = MonitorSnapshot { work_areas: vec![] };
        let snapshot = odd_edge_snapshot();
        for anchor in [Anchor::Bottom, Anchor::Top, Anchor::Left, Anchor::Right] {
            assert_eq!(
                project_anchor(anchor, raw, CHAR_SIZE, None),
                raw,
                "{anchor:?}: snapshot 不在は identity"
            );
            assert_eq!(
                project_anchor(anchor, raw, CHAR_SIZE, Some(&empty)),
                raw,
                "{anchor:?}: 空 snapshot は identity"
            );
            for size in [
                SizePx { w: 0, h: 687 },
                SizePx { w: 434, h: 0 },
                SizePx { w: -434, h: -687 },
                SizePx {
                    w: i32::MIN,
                    h: i32::MIN,
                },
            ] {
                assert_eq!(
                    project_anchor(anchor, raw, size, Some(&snapshot)),
                    raw,
                    "{anchor:?}: 非正寸 {size:?} は identity"
                );
            }
        }
    }

    /// #3 モニタ跨ぎ（Req1.1/2.4）: Right/Bottom は raw 位置の窓中心が属するモニタの
    /// 対応辺へ live 算出する（跨いだ先の右端／下端へ再吸着）。下端・右端が異なる
    /// 2 面で固定する。
    #[test]
    fn project_anchor_resolves_per_crossed_monitor() {
        let snapshot = MonitorSnapshot {
            work_areas: vec![
                rect(0, 0, 1920, 1040),       // primary（右端 1920・下端 1040）
                rect(1920, -213, 4477, 1227), // 右モニタ（右端 4477・下端 1227・96 非倍数）
            ],
        };
        // 中心 (700+217, 300+343)=(917, 643) → primary
        let raw_primary = PointPx { x: 700, y: 300 };
        // 中心 (2700+217, 300+343)=(2917, 643) → 右モニタ
        let raw_right = PointPx { x: 2700, y: 300 };

        // Right: 属するモニタの右端で live 算出
        assert_eq!(
            project_anchor(Anchor::Right, raw_primary, CHAR_SIZE, Some(&snapshot)),
            PointPx { x: 1920 - 434, y: 300 },
            "primary 帰属 → primary 右端"
        );
        assert_eq!(
            project_anchor(Anchor::Right, raw_right, CHAR_SIZE, Some(&snapshot)),
            PointPx { x: 4477 - 434, y: 300 },
            "右モニタ帰属 → 右モニタ右端（跨ぎ再吸着）"
        );
        // Bottom: 属するモニタの下端で live 算出
        assert_eq!(
            project_anchor(Anchor::Bottom, raw_right, CHAR_SIZE, Some(&snapshot)),
            PointPx { x: 2700, y: 1227 - 687 },
            "右モニタ帰属 → 右モニタ下端"
        );
        assert_eq!(
            project_anchor(Anchor::Bottom, raw_primary, CHAR_SIZE, Some(&snapshot)),
            PointPx { x: 700, y: 1040 - 687 },
            "primary 帰属 → primary 下端"
        );
    }

    /// #5 べき等の不動点（Req3.1）: 既にアンカー辺一致の位置＋同寸で project_anchor が
    /// 同値を返す（drag/resize の再適用が振動を生まない基礎）。加えて T∘T = T
    /// （二重適用同値）を Bottom/Right で固定する。
    #[test]
    fn project_anchor_is_idempotent_at_anchor_aligned_positions() {
        let snapshot = odd_edge_snapshot(); // rect(53, 37, 1877, 1043)
        // 各アンカー辺に既に一致する位置は不動点（中心はいずれも単一モニタ内）
        let bottom_fixed = PointPx { x: 700, y: 1043 - 687 };
        assert_eq!(
            project_anchor(Anchor::Bottom, bottom_fixed, CHAR_SIZE, Some(&snapshot)),
            bottom_fixed,
            "Bottom 不動点"
        );
        let top_fixed = PointPx { x: 700, y: 37 };
        assert_eq!(
            project_anchor(Anchor::Top, top_fixed, CHAR_SIZE, Some(&snapshot)),
            top_fixed,
            "Top 不動点"
        );
        let left_fixed = PointPx { x: 53, y: 300 };
        assert_eq!(
            project_anchor(Anchor::Left, left_fixed, CHAR_SIZE, Some(&snapshot)),
            left_fixed,
            "Left 不動点"
        );
        let right_fixed = PointPx { x: 1877 - 434, y: 300 };
        assert_eq!(
            project_anchor(Anchor::Right, right_fixed, CHAR_SIZE, Some(&snapshot)),
            right_fixed,
            "Right 不動点"
        );

        // T∘T = T: 任意の生位置を一度射影した結果に再射影しても同値
        for anchor in [Anchor::Bottom, Anchor::Right] {
            let once = project_anchor(anchor, PointPx { x: 700, y: 999 }, CHAR_SIZE, Some(&snapshot));
            assert_eq!(
                project_anchor(anchor, once, CHAR_SIZE, Some(&snapshot)),
                once,
                "{anchor:?}: T∘T = T（べき等）"
            );
        }
    }

    // -------------------------------------------------------------------------
    // Anchored（Component・task 2.2・Req4.2/1.4）
    //
    // 解決済みアンカーを窓 entity へ 1 つだけ紐づけ、drag／resize が読む単一の
    // 真実源として付与・読み出しできることを固定する（表現のみ）。spawn 時付与
    // （task 3.1）・`Changed<Anchored>` 反応 system（task 2.6）・`BottomSnap`→
    // `Anchored` 移行（task 2.7）は後続 task の領分ゆえ先取りしない。
    // -------------------------------------------------------------------------

    use super::Anchored;

    /// 観測可能な完了条件（4.2/1.4）: 任意の窓 entity へ 5 値アンカーのうち任意の
    /// 1 つを付与し、`world.get::<Anchored>()` で読み出せる。付け替えると読み出しも
    /// 変わる＝単一値を保持する（drag／resize が読む単一真実源・二重格納しない）。
    #[test]
    fn anchored_component_attaches_and_reads_back_on_window_entity() {
        let mut world = World::new();

        // 5 値のうち任意の 1 つ（Left）を窓 entity へ付与して読み出せる
        let e = world
            .spawn((fake_handle(0x1000), Anchored(Anchor::Left)))
            .id();
        assert_eq!(world.get::<Anchored>(e), Some(&Anchored(Anchor::Left)));

        // 別 anchor（Bottom）でも 1 件確認＝「5 値のうち任意の 1 つを保持できる」
        let e2 = world.spawn(Anchored(Anchor::Bottom)).id();
        assert_eq!(world.get::<Anchored>(e2), Some(&Anchored(Anchor::Bottom)));

        // 付け替えたら読み出しも変わる（単一値の保持・格納は 1 つだけ）
        world.entity_mut(e).insert(Anchored(Anchor::Top));
        assert_eq!(world.get::<Anchored>(e), Some(&Anchored(Anchor::Top)));
    }

    // -------------------------------------------------------------------------
    // bottom 吸着ドラッグ（task 8.2R・4.7・DD15 v2: 単一ライター）
    //
    // BottomSnap キャラ窓は DragConfig{move_window:false}＝wndproc は窓を動かさず、
    // on_char_drag が DraggingState（dispatch_drag_events が挿入）＋DragEvent の
    // カーソル座標から生ドラッグ座標を復元し、ポリシー適用済み座標を一度だけ書く。
    // headless では DraggingState を注入して実 flow を模し、handler を直接呼ぶ。
    // -------------------------------------------------------------------------

    /// DraggingState（dispatch_drag_events 挿入の模擬）。wintf の実セマンティクス:
    /// `initial_inset`＝ドラッグ開始時の**窓位置**（dispatch.rs が initial_window_pos
    /// を転記）・`drag_start_pos`＝開始カーソル（スクリーン物理 px）。
    fn dragging_state(initial_window: (i32, i32), drag_start: (i32, i32)) -> DraggingState {
        DraggingState {
            drag_start_pos: Point::new(drag_start.0, drag_start.1),
            initial_inset: (initial_window.0 as f32, initial_window.1 as f32),
        }
    }

    /// カーソル座標付き DragEvent（start_position は DraggingState と同値・実 flow 準拠）。
    fn drag_event_at(target: Entity, start: (i32, i32), cursor: (i32, i32)) -> DragEvent {
        DragEvent {
            target,
            start_position: Point::new(start.0, start.1),
            position: Point::new(cursor.0, cursor.1),
            is_primary: true,
            timestamp: Instant::now(),
        }
    }

    /// 最終カーソル座標付き DragEndEvent。
    fn drag_end_event_at(target: Entity, cursor: (i32, i32)) -> DragEndEvent {
        DragEndEvent {
            target,
            position: Point::new(cursor.0, cursor.1),
            cancelled: false,
            is_primary: true,
            timestamp: Instant::now(),
        }
    }

    /// position＋size 付きの WindowPos（spawn の `window_pos` と同型）。
    fn window_pos_sized(x: i32, y: i32, w: i32, h: i32) -> WindowPos {
        WindowPos {
            position: Some(Point { x, y }),
            size: Some(SizeI::new(w, h)),
            ..Default::default()
        }
    }

    /// (a)(b) 単一ライター・振動なし: 連続 DragEvent の**各適用直後**に WindowPos が
    /// 「X=生ドラッグ X・Y=釘付け Y」を示し、非釘付け Y が一度も現れない
    /// （v1 の事後補正振動に対する最強の檻——反映段階で既に正しい座標のみが書かれる）。
    /// X はカーソル差分の素通し（物理 px・再スケールなし・4.7）。
    #[test]
    fn on_char_drag_writes_only_policy_applied_positions() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot()); // 下端 1043・釘付け Y=1043−687=356
        let start = (1400, 600);
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(1207, 356, 434, 687), // 釘付け済み初期位置
                BottomSnap,
                dragging_state((1207, 356), start),
            ))
            .id();

        // 上下左右へ振るカーソル列（生 Y はどれも下端から浮く／沈む値になる）
        for cursor in [(1450, 650), (1500, 300), (1290, 900), (1601, 113)] {
            let ev = Phase::Bubble(drag_event_at(window, start, cursor));
            assert!(!on_char_drag(&mut world, window, window, &ev));
            let expected_x = 1207 + (cursor.0 - start.0);
            assert_eq!(
                position_of(&world, window),
                Point {
                    x: expected_x,
                    y: 356
                },
                "cursor={cursor:?}: 反映段階で既に釘付け済みの座標のみが書かれる"
            );
        }
    }

    /// (c) モニタ跨ぎ: 生ドラッグ位置の窓中心が隣モニタへ移ったら、跨いだ先の
    /// work area 下端へ再吸着し、戻れば元モニタの下端へ戻る（live 算出・4.7）。
    #[test]
    fn on_char_drag_resnaps_to_crossed_monitor_bottom() {
        let mut world = World::new();
        world.insert_resource(MonitorSnapshot {
            work_areas: vec![
                rect(0, 0, 1920, 1040),       // primary（下端 1040）
                rect(1920, -213, 4480, 1227), // 右の高解像度モニタ（下端 1227）
            ],
        });
        let start = (1600, 500);
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(1400, 353, 434, 687), // primary の下端に釘付け済み
                BottomSnap,
                dragging_state((1400, 353), start),
            ))
            .id();

        // カーソルを右モニタ方向へ: raw=(2700,353)・中心 x=2917 → 右モニタ帰属
        let ev = Phase::Bubble(drag_event_at(window, start, (2900, 500)));
        assert!(!on_char_drag(&mut world, window, window, &ev));
        assert_eq!(
            position_of(&world, window),
            Point {
                x: 2700,
                y: 1227 - 687
            }
        );

        // 戻す: raw=(1100,353)・中心 x=1317 → primary へ再吸着
        let ev = Phase::Bubble(drag_event_at(window, start, (1300, 500)));
        assert!(!on_char_drag(&mut world, window, window, &ev));
        assert_eq!(
            position_of(&world, window),
            Point {
                x: 1100,
                y: 1040 - 687
            }
        );
    }

    /// (d) Free 窓（BottomSnap なし＝move_window=true）は wndproc 委譲のまま:
    /// ハンドラはキャラ窓を書かず、DraggingState があってもポリシー写像を使わない
    /// （wndproc 更新済み WindowPos 基準でバルーン追従のみ・挙動不変・4.7）。
    #[test]
    fn on_char_drag_free_window_stays_wndproc_delegated() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot());
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
            .id();
        let offset = PointPx { x: 498, y: -37 };
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(1207, 217, 434, 687), // wndproc がドラッグ中に更新した位置
                BalloonFollow { balloon, offset },
                // DraggingState が居ても free 経路は写像を使わない檻（実 flow でも挿入される）
                dragging_state((999, 888), (0, 0)),
            ))
            .id();

        let ev = Phase::Bubble(drag_event_at(window, (0, 0), (10, 10)));
        assert!(!on_char_drag(&mut world, window, window, &ev));

        // キャラ窓は不動（wndproc の領分）・バルーンは WindowPos 基準で追従
        assert_eq!(position_of(&world, window), Point { x: 1207, y: 217 });
        assert_eq!(
            position_of(&world, balloon),
            Point {
                x: 1207 + offset.x,
                y: 217 + offset.y
            }
        );
    }

    /// (e) バルーン追従はポリシー**適用後**座標＋offset 基準
    /// （生ドラッグ座標基準だと Y がずれる檻・4.2/4.7）。
    #[test]
    fn on_char_drag_balloon_follows_policy_applied_position() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot());
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
            .id();
        let offset = PointPx { x: -400, y: 25 };
        let start = (1400, 600);
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(1207, 356, 434, 687),
                BottomSnap,
                BalloonFollow { balloon, offset },
                dragging_state((1207, 356), start),
            ))
            .id();

        // カーソルが上へ 250px: raw Y=106 だが適用後 Y=356
        let ev = Phase::Bubble(drag_event_at(window, start, (1450, 350)));
        assert!(!on_char_drag(&mut world, window, window, &ev));

        let char_pos = position_of(&world, window);
        assert_eq!(char_pos, Point { x: 1257, y: 356 });
        assert_eq!(
            position_of(&world, balloon),
            Point {
                x: char_pos.x + offset.x,
                y: char_pos.y + offset.y
            }
        );
    }

    /// (f) DragEnd: 最終カーソル位置へ同写像を適用する（accumulator の
    /// `current_dragging_entity` 先行クリアで最終 DragEvent が欠落する穴の埋め・
    /// DD15 v2 (3)）。バルーンも適用後座標基準で追従する。
    #[test]
    fn on_char_drag_end_applies_policy_at_final_cursor() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot());
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
            .id();
        let offset = PointPx { x: -400, y: 25 };
        let start = (1400, 600);
        let window = world
            .spawn((
                fake_handle(0x1000),
                // 「最後に配送された DragEvent 時点」の位置を模す（最終位置とはずれている）
                window_pos_sized(1250, 356, 434, 687),
                BottomSnap,
                BalloonFollow { balloon, offset },
                // OnDragEnd 配送時点では DraggingState はまだ生きている（dispatch.rs は
                // ハンドラ配送**後**に remove する）——実 flow 準拠
                dragging_state((1207, 356), start),
            ))
            .id();

        let ev = Phase::Bubble(drag_end_event_at(window, (1601, 113)));
        assert!(!on_char_drag_end(&mut world, window, window, &ev));

        // raw=(1207+201, 356−487)=(1408, −131) → 適用後 (1408, 356)
        assert_eq!(position_of(&world, window), Point { x: 1408, y: 356 });
        assert_eq!(
            position_of(&world, balloon),
            Point {
                x: 1408 + offset.x,
                y: 356 + offset.y
            }
        );
    }

    /// (f) 補: DragEnd の Tunnel フェーズは無視する（他ハンドラと同じ規約）。
    #[test]
    fn on_char_drag_end_tunnel_phase_is_ignored() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot());
        let start = (1400, 600);
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(1250, 356, 434, 687),
                BottomSnap,
                dragging_state((1207, 356), start),
            ))
            .id();

        let ev = Phase::Tunnel(drag_end_event_at(window, (1601, 113)));
        assert!(!on_char_drag_end(&mut world, window, window, &ev));
        assert_eq!(position_of(&world, window), Point { x: 1250, y: 356 });
    }

    /// (g) target==自 entity ガード: 他 entity 宛イベントの Bubble を受けても
    /// on_char_drag／on_char_drag_end／on_balloon_drag はすべて no-op。
    #[test]
    fn drag_handlers_ignore_events_targeting_other_entities() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot());
        let other = world.spawn_empty().id();
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(701, 383)))
            .id();
        let initial = PointPx { x: 11, y: 22 };
        let start = (1400, 600);
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(1207, 356, 434, 687),
                BottomSnap,
                BalloonFollow {
                    balloon,
                    offset: initial,
                },
                dragging_state((1207, 356), start),
            ))
            .id();

        // on_char_drag: target=other → 窓もバルーンも不動
        let ev = Phase::Bubble(drag_event_at(other, start, (1601, 113)));
        assert!(!on_char_drag(&mut world, other, window, &ev));
        assert_eq!(position_of(&world, window), Point { x: 1207, y: 356 });
        assert_eq!(position_of(&world, balloon), Point { x: 701, y: 383 });

        // on_char_drag_end: target=other → 不動
        let ev = Phase::Bubble(drag_end_event_at(other, (1601, 113)));
        assert!(!on_char_drag_end(&mut world, other, window, &ev));
        assert_eq!(position_of(&world, window), Point { x: 1207, y: 356 });

        // on_balloon_drag: target=other → offset 不変
        let ev = Phase::Bubble(drag_event_at(other, start, (10, 10)));
        assert!(!on_balloon_drag(&mut world, other, balloon, &ev));
        assert_eq!(world.get::<BalloonFollow>(window).unwrap().offset, initial);
    }

    /// (+) MonitorSnapshot 不在（main.rs フォールバック経路）: ポリシーは identity
    /// へ縮退し、窓は生ドラッグ座標のまま単一ライターで移動する（move_window=false
    /// でもドラッグ追従が生きる縮退・吸着なし・panic なし）。
    #[test]
    fn on_char_drag_without_snapshot_moves_to_raw_position() {
        let mut world = World::new(); // Resource 未挿入
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
            .id();
        let offset = PointPx { x: 11, y: 22 };
        let start = (1400, 600);
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(1207, 356, 434, 687),
                BottomSnap,
                BalloonFollow { balloon, offset },
                dragging_state((1207, 356), start),
            ))
            .id();

        let ev = Phase::Bubble(drag_event_at(window, start, (1450, 350)));
        assert!(!on_char_drag(&mut world, window, window, &ev));

        // raw=(1257, 106) そのまま・バルーンは raw 基準で追従
        assert_eq!(position_of(&world, window), Point { x: 1257, y: 106 });
        assert_eq!(
            position_of(&world, balloon),
            Point {
                x: 1257 + offset.x,
                y: 106 + offset.y
            }
        );
    }

    /// (+) WindowPos.size 不在／`WindowPos::default()` の CW_USEDEFAULT センチネル:
    /// 非正寸法として identity 縮退＝生ドラッグ座標のまま移動（暴走・panic なし）。
    #[test]
    fn on_char_drag_with_invalid_size_degrades_to_identity() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot());
        let start = (1400, 600);

        // size=None
        let mut wp = window_pos_at(1207, 356);
        wp.size = None;
        let no_size = world
            .spawn((
                fake_handle(0x1000),
                wp,
                BottomSnap,
                dragging_state((1207, 356), start),
            ))
            .id();
        let ev = Phase::Bubble(drag_event_at(no_size, start, (1450, 350)));
        assert!(!on_char_drag(&mut world, no_size, no_size, &ev));
        assert_eq!(position_of(&world, no_size), Point { x: 1257, y: 106 });

        // size=CW_USEDEFAULT センチネル（window_pos_at は ..Default::default()）
        let sentinel = world
            .spawn((
                fake_handle(0x2000),
                window_pos_at(1207, 356),
                BottomSnap,
                dragging_state((1207, 356), start),
            ))
            .id();
        let ev = Phase::Bubble(drag_event_at(sentinel, start, (1450, 350)));
        assert!(!on_char_drag(&mut world, sentinel, sentinel, &ev));
        assert_eq!(position_of(&world, sentinel), Point { x: 1257, y: 106 });
    }

    /// (+) DraggingState 不在の BottomSnap 窓（実 flow では dispatch が DragEvent
    /// より先に挿入する）: 生座標を復元できないため書き込みなし（panic なし・
    /// バルーンも不動）。
    #[test]
    fn on_char_drag_without_dragging_state_is_noop_for_snap_window() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot());
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(70, 80)))
            .id();
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(1207, 356, 434, 687),
                BottomSnap,
                BalloonFollow {
                    balloon,
                    offset: PointPx { x: 11, y: 22 },
                },
            ))
            .id();

        let ev = Phase::Bubble(drag_event_at(window, (1400, 600), (1450, 350)));
        assert!(!on_char_drag(&mut world, window, window, &ev));
        assert_eq!(position_of(&world, window), Point { x: 1207, y: 356 });
        assert_eq!(position_of(&world, balloon), Point { x: 70, y: 80 });
    }

    // -------------------------------------------------------------------------
    // on_balloon_drag: バルーン単独ドラッグの相対位置記憶（task 8.3・4.8・DD16）
    // wndproc がバルーン窓を移動済み（WindowPos 更新済み）の状態を模して呼び、
    // offset の記憶更新・キャラ窓不動・以後の consumer 追従を決定論検証する。
    // 座標は 96 の倍数を避け、x/y で符号・値の異なる offset を使う
    // （符号取り違え・軸取り違えの檻）。
    // -------------------------------------------------------------------------

    use super::on_balloon_drag;

    /// Tunnel フェーズは無視する（on_char_drag と同じ規約・offset 不変）。
    #[test]
    fn on_balloon_drag_tunnel_phase_is_ignored() {
        let mut world = World::new();
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(701, 383)))
            .id();
        let initial = PointPx { x: 11, y: 22 };
        let char_w = world
            .spawn((
                fake_handle(0x1000),
                window_pos_at(1207, 653),
                BalloonFollow {
                    balloon,
                    offset: initial,
                },
            ))
            .id();

        let ev = Phase::Tunnel(drag_event(balloon));
        assert!(!on_balloon_drag(&mut world, balloon, balloon, &ev));
        assert_eq!(
            world.get::<BalloonFollow>(char_w).unwrap().offset,
            initial,
            "Tunnel では offset を更新しない"
        );
    }

    /// (a)(c) バルーン単独ドラッグ: 所有キャラ窓の `BalloonFollow.offset` が
    /// `balloon_pos − char_pos` へ更新され（4.8）、キャラ窓は不動。
    /// x/y の符号が異なる期待値で減算の向き（balloon − char）を固定する檻。
    #[test]
    fn on_balloon_drag_updates_offset_and_char_window_is_unmoved() {
        let mut world = World::new();
        // wndproc がドラッグ中に更新した後のバルーン位置を模す
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(1729, 401)))
            .id();
        let char_w = world
            .spawn((
                fake_handle(0x1000),
                window_pos_at(1207, 653),
                BalloonFollow {
                    balloon,
                    offset: PointPx { x: -412, y: -25 },
                },
            ))
            .id();

        let ev = Phase::Bubble(drag_event(balloon));
        // イベントは消費しない（伝播続行＝false）
        assert!(!on_balloon_drag(&mut world, balloon, balloon, &ev));

        // offset = balloon − char = (1729−1207, 401−653) = (+522, −252)
        // （char − balloon なら (−522, +252)＝符号取り違えの檻）
        assert_eq!(
            world.get::<BalloonFollow>(char_w).unwrap().offset,
            PointPx { x: 522, y: -252 }
        );
        // (c) キャラ窓は不動（4.8: バルーンのみ移動・bottom 吸着の対象外）
        assert_eq!(position_of(&world, char_w), Point { x: 1207, y: 653 });
        // バルーン自身もハンドラでは動かさない（wndproc の領分）
        assert_eq!(position_of(&world, balloon), Point { x: 1729, y: 401 });
    }

    /// (b) バルーン単独ドラッグ後の `move_window_to`: 調整後 offset で追従する
    /// （初期 offset へのスナップバックは仕様退役・4.8）。
    #[test]
    fn move_window_to_after_balloon_drag_follows_adjusted_offset() {
        let mut world = World::new();
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
            .id();
        let initial = PointPx { x: -412, y: -25 };
        let char_w = world
            .spawn((
                fake_handle(0x1000),
                window_pos_at(1207, 653),
                BalloonFollow {
                    balloon,
                    offset: initial,
                },
            ))
            .id();

        // バルーン単独ドラッグ（wndproc がバルーンを (613, 407) へ移動済み）
        world.get_mut::<WindowPos>(balloon).unwrap().position = Some(Point { x: 613, y: 407 });
        let ev = Phase::Bubble(drag_event(balloon));
        assert!(!on_balloon_drag(&mut world, balloon, balloon, &ev));

        let adjusted = PointPx {
            x: 613 - 1207,
            y: 407 - 653,
        };
        assert_ne!(adjusted, initial, "檻の前提: 調整後 offset は初期値と異なる");
        assert_eq!(world.get::<BalloonFollow>(char_w).unwrap().offset, adjusted);

        // 次のキャラ窓移動 API は調整後 offset で追従（consumer 無改変・DD16）
        assert!(move_window_to(&mut world, char_w, 1751, 893));
        assert_eq!(
            position_of(&world, balloon),
            Point {
                x: 1751 + adjusted.x,
                y: 893 + adjusted.y
            }
        );
    }

    /// (b)(c) 8.2＋8.3 の合成: BottomSnap キャラ窓の場合——バルーンドラッグでは
    /// キャラ窓の Y 釘付けは発火せず（不動・4.8「bottom 吸着の対象外」）、
    /// その後のキャラ窓ドラッグは Y 釘付けの**後**に調整後 offset で追従する。
    #[test]
    fn on_char_drag_after_balloon_drag_pins_y_and_follows_adjusted_offset() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot()); // 下端 1043
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(500, 300)))
            .id();
        let initial = PointPx { x: -412, y: -25 };
        // 釘付け済み位置（Y=1043−687=356）から開始する BottomSnap キャラ窓
        let char_w = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(1207, 356, 434, 687),
                BottomSnap,
                BalloonFollow {
                    balloon,
                    offset: initial,
                },
            ))
            .id();

        // バルーン単独ドラッグ（wndproc がバルーンを (831, 149) へ移動済み）
        world.get_mut::<WindowPos>(balloon).unwrap().position = Some(Point { x: 831, y: 149 });
        let ev = Phase::Bubble(drag_event(balloon));
        assert!(!on_balloon_drag(&mut world, balloon, balloon, &ev));

        // キャラ窓は不動（Y 釘付けも発火しない・4.8）
        assert_eq!(position_of(&world, char_w), Point { x: 1207, y: 356 });
        let adjusted = PointPx {
            x: 831 - 1207,
            y: 149 - 356,
        };
        assert_eq!(world.get::<BalloonFollow>(char_w).unwrap().offset, adjusted);

        // 次のキャラ窓ドラッグ（move_window=false 単一ライター・8.2R）:
        // DraggingState を注入し、カーソルが下端から浮く位置まで動いた DragEvent を配送
        let start = (1300, 700);
        world
            .entity_mut(char_w)
            .insert(dragging_state((1207, 356), start));
        let ev = Phase::Bubble(drag_event_at(char_w, start, (996, 555)));
        assert!(!on_char_drag(&mut world, char_w, char_w, &ev));

        // 8.2R: raw=(903, 211) → 適用後 (903, 356)（Y 釘付け・X 素通し）
        assert_eq!(position_of(&world, char_w), Point { x: 903, y: 356 });
        // 8.3: バルーンは釘付け後座標＋**調整後** offset（初期 offset だと不一致）
        assert_eq!(
            position_of(&world, balloon),
            Point {
                x: 903 + adjusted.x,
                y: 356 + adjusted.y
            }
        );
    }

    /// (d) 複数スコープ: ドラッグしたバルーンを所有するキャラ窓の offset だけが
    /// 更新され、他スコープの offset・窓位置は不干渉（誤マッチの檻）。
    #[test]
    fn on_balloon_drag_updates_only_matching_scope_offset() {
        let mut world = World::new();
        let balloon0 = world
            .spawn((fake_handle(0x2000), window_pos_at(701, 383)))
            .id();
        let char0 = world
            .spawn((
                fake_handle(0x1000),
                window_pos_at(1207, 653),
                BalloonFollow {
                    balloon: balloon0,
                    offset: PointPx { x: -412, y: -25 },
                },
            ))
            .id();
        let balloon1 = world
            .spawn((fake_handle(0x4000), window_pos_at(1334, 1044)))
            .id();
        let offset1 = PointPx { x: 285, y: -19 };
        let char1 = world
            .spawn((
                fake_handle(0x3000),
                window_pos_at(1049, 1063),
                BalloonFollow {
                    balloon: balloon1,
                    offset: offset1,
                },
            ))
            .id();

        let ev = Phase::Bubble(drag_event(balloon0));
        assert!(!on_balloon_drag(&mut world, balloon0, balloon0, &ev));

        // scope0 の offset は balloon0 − char0 = (−506, −270) へ更新
        assert_eq!(
            world.get::<BalloonFollow>(char0).unwrap().offset,
            PointPx { x: -506, y: -270 }
        );
        // scope1 の offset・窓位置は不変（誤マッチなし）
        assert_eq!(world.get::<BalloonFollow>(char1).unwrap().offset, offset1);
        assert_eq!(position_of(&world, char1), Point { x: 1049, y: 1063 });
        assert_eq!(position_of(&world, balloon1), Point { x: 1334, y: 1044 });
    }

    /// (+) バルーンの `WindowPos.position` 不在は no-op（false・panic なし・
    /// offset 不変）。所有キャラ窓の position 不在も skip で panic しない。
    #[test]
    fn on_balloon_drag_without_positions_is_graceful() {
        let mut world = World::new();

        // バルーン側 position 不在 → offset 不変
        let mut wp = window_pos_at(0, 0);
        wp.position = None;
        let balloon = world.spawn((fake_handle(0x2000), wp)).id();
        let initial = PointPx { x: 11, y: 22 };
        let char_w = world
            .spawn((
                fake_handle(0x1000),
                window_pos_at(50, 60),
                BalloonFollow {
                    balloon,
                    offset: initial,
                },
            ))
            .id();
        let ev = Phase::Bubble(drag_event(balloon));
        assert!(!on_balloon_drag(&mut world, balloon, balloon, &ev));
        assert_eq!(world.get::<BalloonFollow>(char_w).unwrap().offset, initial);

        // キャラ側 position 不在 → skip（panic なし・offset 不変）
        let balloon2 = world
            .spawn((fake_handle(0x4000), window_pos_at(70, 80)))
            .id();
        let mut char_wp = window_pos_at(0, 0);
        char_wp.position = None;
        let char2 = world
            .spawn((
                fake_handle(0x3000),
                char_wp,
                BalloonFollow {
                    balloon: balloon2,
                    offset: initial,
                },
            ))
            .id();
        let ev = Phase::Bubble(drag_event(balloon2));
        assert!(!on_balloon_drag(&mut world, balloon2, balloon2, &ev));
        assert_eq!(world.get::<BalloonFollow>(char2).unwrap().offset, initial);

        // 所有キャラ窓が 1 つも無いバルーン → no-op（panic なし）
        let orphan = world
            .spawn((fake_handle(0x5000), window_pos_at(10, 20)))
            .id();
        let ev = Phase::Bubble(drag_event(orphan));
        assert!(!on_balloon_drag(&mut world, orphan, orphan, &ev));
    }

    // -------------------------------------------------------------------------
    // Arrangement.offset 同期（task 8.3-fix・4.8 実機ブロッカ）
    //
    // enqueue_window_set_pos は WindowPos を bypass_change_detection() で書くため
    // Changed<WindowPos> が発火せず、wintf の
    // sync_window_arrangement_from_window_pos は走らない。同期を怠ると
    // GlobalArrangement（αマスクヒットテストの境界）が spawn 位置に取り残され、
    // 移動後のバルーンがクリック死する（実機で確認された 4.8 ブロッカ）。
    // 実 pipeline では window entity に Arrangement が付く（Visual::on_add）が、
    // bare World には無いので spawn 時 offset 付きで手動挿入して檻にする。
    // 期待値は wintf DragEnd 直接同期と同じ `as f32` 転写の完全一致。
    // -------------------------------------------------------------------------

    use wintf::ecs::layout::{Arrangement, Offset};

    /// spawn 時 offset 付きの Arrangement（実 pipeline の spawn 位置を模す）。
    fn arrangement_at(x: f32, y: f32) -> Arrangement {
        Arrangement {
            offset: Offset { x, y },
            ..Default::default()
        }
    }

    /// entity の Arrangement.offset を読む（未付与は panic で検出）。
    fn arrangement_offset_of(world: &World, entity: Entity) -> Offset {
        world
            .get::<Arrangement>(entity)
            .expect("Arrangement があるはず")
            .offset
    }

    /// (a) 実 on_char_drag（Bubble DragEvent＋DraggingState・8.2R 単一ライター）:
    /// 移動後、キャラ窓・随伴バルーンとも Arrangement.offset が
    /// WindowPos.position と一致する（GA ヒットテスト境界の追従・4.8）。
    #[test]
    fn on_char_drag_syncs_arrangement_offset_of_char_and_balloon() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot()); // 下端 1043・釘付け Y=356
        let balloon = world
            .spawn((
                fake_handle(0x2000),
                window_pos_at(795, 331),
                arrangement_at(795.0, 331.0),
            ))
            .id();
        let offset = PointPx { x: -412, y: -25 };
        let start = (1400, 600);
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(1207, 356, 434, 687),
                arrangement_at(1207.0, 356.0),
                BottomSnap,
                BalloonFollow { balloon, offset },
                dragging_state((1207, 356), start),
            ))
            .id();

        let ev = Phase::Bubble(drag_event_at(window, start, (1450, 350)));
        assert!(!on_char_drag(&mut world, window, window, &ev));

        // 適用後キャラ窓 (1257, 356)・バルーン (1257−412, 356−25)
        let char_pos = position_of(&world, window);
        assert_eq!(char_pos, Point { x: 1257, y: 356 });
        assert_eq!(
            arrangement_offset_of(&world, window),
            Offset {
                x: char_pos.x as f32,
                y: char_pos.y as f32
            },
            "キャラ窓の Arrangement.offset が WindowPos に追従する"
        );
        let balloon_pos = position_of(&world, balloon);
        assert_eq!(balloon_pos, Point { x: 845, y: 331 });
        assert_eq!(
            arrangement_offset_of(&world, balloon),
            Offset {
                x: balloon_pos.x as f32,
                y: balloon_pos.y as f32
            },
            "バルーンの Arrangement.offset が WindowPos に追従する（クリック死の檻）"
        );
    }

    /// (b) move_window_to: 対象キャラ窓・随伴バルーンとも Arrangement.offset が
    /// 移動後の WindowPos.position と一致する。
    #[test]
    fn move_window_to_syncs_arrangement_offset_of_target_and_balloon() {
        let mut world = World::new();
        let balloon = world
            .spawn((
                fake_handle(0x2000),
                window_pos_at(0, 0),
                arrangement_at(0.0, 0.0),
            ))
            .id();
        let offset = PointPx { x: -412, y: -25 };
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_at(50, 60),
                arrangement_at(50.0, 60.0),
                BalloonFollow { balloon, offset },
            ))
            .id();

        assert!(move_window_to(&mut world, window, 907, 1201));

        assert_eq!(
            arrangement_offset_of(&world, window),
            Offset { x: 907.0, y: 1201.0 }
        );
        assert_eq!(
            arrangement_offset_of(&world, balloon),
            Offset {
                x: (907 + offset.x) as f32,
                y: (1201 + offset.y) as f32
            }
        );
    }

    /// (c) move_window_to（BalloonFollow なしの単独窓）: 自身の Arrangement.offset
    /// が同期される（バルーン単独移動＝enqueue 共通経路の檻）。
    #[test]
    fn move_window_to_syncs_arrangement_offset_of_single_window() {
        let mut world = World::new();
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_at(10, 20),
                arrangement_at(10.0, 20.0),
            ))
            .id();

        assert!(move_window_to(&mut world, window, 1531, 883));
        assert_eq!(
            arrangement_offset_of(&world, window),
            Offset { x: 1531.0, y: 883.0 }
        );
    }

    // -------------------------------------------------------------------------
    // enqueue_window_set_pos（size 対応一般化・task 2.3・Req1.5/3.3・
    // design Testing Strategy > Integration Tests #5）
    //
    // 既存 move 専用発行口の一般化。`None` は移動専用の後方互換（position のみ
    // ミラー・size 不変・SWP_NOSIZE 継続）、`Some` は位置＋寸を一度に反映
    // （WindowPos.size も bypass ミラー）。観測境界は `WindowPos.position`／
    // `WindowPos.size` のミラー——`SetWindowPosCommand` キューは private TLS で
    // flush せず flags/width/height を覗けないため（design Validation の指定）。
    // 座標・寸法は 96 の非倍数を使い、隠れた dpi/96 再スケールの檻とする。
    // -------------------------------------------------------------------------

    use super::enqueue_window_set_pos;

    /// entity の WindowPos.size を読む（未設定は panic で検出）。
    fn size_of(world: &World, entity: Entity) -> SizeI {
        world
            .get::<WindowPos>(entity)
            .expect("WindowPos があるはず")
            .size
            .expect("size があるはず")
    }

    /// `None`（後方互換・移動専用）: position のみ更新し size は触らない
    /// （既存移動専用挙動＝SWP_NOSIZE 継続の観測境界）。
    #[test]
    fn enqueue_window_set_pos_none_updates_position_leaves_size() {
        let mut world = World::new();
        let window = world
            .spawn((fake_handle(0x1234), window_pos_sized(10, 20, 434, 687)))
            .id();

        assert!(enqueue_window_set_pos(&mut world, window, 1531, 883, None));
        assert_eq!(position_of(&world, window), Point { x: 1531, y: 883 });
        // size は不変（移動専用＝寸法を書かない）
        assert_eq!(size_of(&world, window), SizeI::new(434, 687));
    }

    /// `Some`: 位置と寸法の**双方**が更新される（WindowPos.size = SizeI::new(w,h)）。
    #[test]
    fn enqueue_window_set_pos_some_updates_position_and_size() {
        let mut world = World::new();
        let window = world
            .spawn((fake_handle(0x1234), window_pos_sized(10, 20, 434, 687)))
            .id();

        assert!(enqueue_window_set_pos(
            &mut world,
            window,
            907,
            1201,
            Some(SizePx { w: 517, h: 823 }),
        ));
        assert_eq!(position_of(&world, window), Point { x: 907, y: 1201 });
        assert_eq!(size_of(&world, window), SizeI::new(517, 823));
    }

    /// 不在/未付与（Req3.3）: `WindowHandle` 無し entity は `false`＋位置/寸法不変
    /// （warn no-op・`Some` 経路でも既存 warn 経路を継承）。
    #[test]
    fn enqueue_window_set_pos_without_handle_returns_false_and_leaves_state() {
        let mut world = World::new();
        let window = world.spawn(window_pos_sized(10, 20, 434, 687)).id();

        assert!(!enqueue_window_set_pos(
            &mut world,
            window,
            907,
            1201,
            Some(SizePx { w: 517, h: 823 }),
        ));
        assert_eq!(position_of(&world, window), Point { x: 10, y: 20 });
        assert_eq!(size_of(&world, window), SizeI::new(434, 687));
    }

    // -------------------------------------------------------------------------
    // resize_window_to（単一ライター反映口・task 2.4・
    // Req1.1/1.3/1.7/3.1/3.4＋2.6/3.3・design Integration Tests #1・#4 一部）
    //
    // 新しい表示寸法へアンカー射影 T を再適用し、確定 position＋size を単一ライター
    // 経路で一度だけ書く（bottom は wa.bottom−h' 再計算）。観測境界は headless World
    // （偽 HWND）の WindowPos.position／WindowPos.size ミラー——SetWindowPosCommand
    // キューは private TLS で flush せず flags/width/height を覗けないため。縮退
    // （べき等・非正寸・不在・Anchored 欠落）は false＋状態不変で固定する。座標・
    // 寸法は 96 の非倍数を使い、隠れた dpi/96 再スケールの檻とする。
    // -------------------------------------------------------------------------

    use super::resize_window_to;

    /// #1 一度書き＋re-snap（Req1.1/1.3/1.7/2.1）: `Anchored(Bottom)` の char 窓を
    /// 新寸へ resize すると、`WindowPos.size` が新寸・`position.y` が `wa.bottom − h'`
    /// （X 保持）へ更新され `true`。下端・寸法とも 96 非倍数で dpi/96 再スケール混入の檻。
    #[test]
    fn resize_window_to_bottom_resnaps_size_and_position_once() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot()); // 下端 1043
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(731, 356, 434, 687), // 旧寸で下端釘付け済み
                Anchored(Anchor::Bottom),
            ))
            .id();

        // 新寸 (517×823・いずれも 96 非倍数): Y=1043−823=220・X=731 保持
        assert!(resize_window_to(&mut world, window, SizePx { w: 517, h: 823 }));
        assert_eq!(
            position_of(&world, window),
            Point {
                x: 731,
                y: 1043 - 823
            },
            "X 保持・Y=wa.bottom−h'（bottom 再計算）"
        );
        assert_eq!(size_of(&world, window), SizeI::new(517, 823));
    }

    /// #4 べき等 skip（Req3.1）: 既に射影済み位置＋同寸の窓へ同寸 resize すると、
    /// 書込なし・`false`・状態不変（冗長な再配置を避ける）。
    #[test]
    fn resize_window_to_is_idempotent_on_same_size_and_position() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot()); // 下端 1043・Y=1043−687=356
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(731, 356, 434, 687), // 既に bottom 射影済み
                Anchored(Anchor::Bottom),
            ))
            .id();

        // 同寸 → 導出 (731,356)＋(434,687) は現在値と同一 → 書込なし・false
        assert!(!resize_window_to(&mut world, window, SizePx { w: 434, h: 687 }));
        assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
        assert_eq!(size_of(&world, window), SizeI::new(434, 687));
    }

    /// #4 非正寸縮退（Req3.4）: w≤0 or h≤0 は T 再適用せず `false`・位置/寸不変
    /// （warn・`BottomSnapPolicy` の非正寸縮退と整合）。
    #[test]
    fn resize_window_to_nonpositive_size_holds_state() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot());
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(731, 356, 434, 687),
                Anchored(Anchor::Bottom),
            ))
            .id();

        for bad in [
            SizePx { w: 0, h: 823 },
            SizePx { w: 517, h: 0 },
            SizePx { w: -517, h: -823 },
        ] {
            assert!(
                !resize_window_to(&mut world, window, bad),
                "{bad:?}: 非正寸は false"
            );
            assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
            assert_eq!(size_of(&world, window), SizeI::new(434, 687));
        }
    }

    /// #4 不在/未付与（Req3.3）: `WindowHandle` 未付与の char 窓は `false`・状態不変
    /// （`enqueue_window_set_pos` の warn no-op を継承・随伴バルーンも動かさない）。
    #[test]
    fn resize_window_to_without_handle_returns_false_and_leaves_state() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot());
        let window = world
            .spawn((
                // WindowHandle なし（窓生成前）
                window_pos_sized(731, 356, 434, 687),
                Anchored(Anchor::Bottom),
            ))
            .id();

        assert!(!resize_window_to(&mut world, window, SizePx { w: 517, h: 823 }));
        assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
        assert_eq!(size_of(&world, window), SizeI::new(434, 687));
    }

    /// #4 Anchored 欠落: 単一真実源 `Anchored` 未付与の窓は `false`・状態不変
    /// （char 窓は spawn で必ず付与＝異常系・warn no-op）。
    #[test]
    fn resize_window_to_without_anchored_returns_false_and_leaves_state() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot());
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(731, 356, 434, 687),
                // Anchored なし
            ))
            .id();

        assert!(!resize_window_to(&mut world, window, SizePx { w: 517, h: 823 }));
        assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
        assert_eq!(size_of(&world, window), SizeI::new(434, 687));
    }

    /// #1 随伴バルーン維持（Req2.6）: `BalloonFollow` 付き char 窓を resize すると、
    /// バルーンが `new_char_pos + offset` へ随伴し `balloon_pos − char_pos ≡ offset` が
    /// 維持される（offset を破壊しない）。
    #[test]
    fn resize_window_to_preserves_balloon_follow_offset() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot()); // 下端 1043
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
            .id();
        let offset = PointPx { x: -412, y: -25 };
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(731, 356, 434, 687),
                Anchored(Anchor::Bottom),
                BalloonFollow { balloon, offset },
            ))
            .id();

        // 新寸 (517×823) → char (731, 220)・balloon (731−412, 220−25)
        assert!(resize_window_to(&mut world, window, SizePx { w: 517, h: 823 }));
        let char_pos = position_of(&world, window);
        let balloon_pos = position_of(&world, balloon);
        assert_eq!(char_pos, Point { x: 731, y: 1043 - 823 });
        assert_eq!(
            balloon_pos,
            Point {
                x: 731 + offset.x,
                y: (1043 - 823) + offset.y
            }
        );
        // offset 恒等式（balloon_pos − char_pos ≡ offset）の維持
        assert_eq!(balloon_pos.x - char_pos.x, offset.x);
        assert_eq!(balloon_pos.y - char_pos.y, offset.y);
    }

    // -------------------------------------------------------------------------
    // resize_window_to 5 アンカー統合網羅（task 2.5・テスト固定タスク・
    // Req1.1/2.1-2.6/3.1/3.3/3.4・design Integration Tests #2・#3・#4）
    //
    // task 2.4 が Bottom で押さえた「一度書き＋re-snap／べき等／非正寸／不在／
    // Anchored 欠落／随伴バルーン維持」を、残る Top/Left/Right/Free へ拡張する。
    // resize_window_to 本体は 2.4 で完成済み＝本群は「既存配線が 5 アンカーで
    // 正しく `Anchored.0` を転送している（非 Bottom を `Anchor::Bottom` へ
    // ハードコードしていない）」ことを固定する回帰檻（非 Bottom 配線バグ＝
    // 2.4 エスケープの捕捉）。
    //
    // 全辺 96 非倍数の odd_edge_snapshot（rect(53,37,1877,1043)）で各アンカー辺の
    // 再計算を dpi/96 再スケール混入の檻とし、各アンカーで「固定辺の座標」と
    // 「非アンカー軸の保持」を両方 assert する（Top↔Bottom は Y・Left↔Right は X が
    // 合わず落ちる取り違え耐性）。
    // -------------------------------------------------------------------------

    /// #2 Top resize（Req2.2）: `Anchored(Top)` を新寸へ resize すると `WindowPos.size`
    /// 新寸・`position.y = wa.top`（上端固定）・`position.x` 保持で `true`。
    /// Bottom と取り違えれば Y が `wa.bottom−h'` になって落ちる（辺取り違え耐性）。
    #[test]
    fn resize_window_to_top_pins_top_edge_and_keeps_x() {
        let mut world = World::new();
        world.insert_resource(odd_edge_snapshot()); // rect(53, 37, 1877, 1043)
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(731, 500, 434, 687),
                Anchored(Anchor::Top),
            ))
            .id();

        // 新寸 (517×823・いずれも 96 非倍数): Y=wa.top=37・X=731 保持
        assert!(resize_window_to(&mut world, window, SizePx { w: 517, h: 823 }));
        assert_eq!(
            position_of(&world, window),
            Point { x: 731, y: 37 },
            "X 保持・Y=wa.top（上端固定・Bottom と取り違えたら 1043−823 で落ちる）"
        );
        assert_eq!(size_of(&world, window), SizeI::new(517, 823));
    }

    /// #2 Left resize（Req2.3）: `Anchored(Left)` を新寸へ resize すると `WindowPos.size`
    /// 新寸・`position.x = wa.left`（左端固定）・`position.y` 保持で `true`。
    /// Right と取り違えれば X が `wa.right−w'` になって落ちる（辺取り違え耐性）。
    #[test]
    fn resize_window_to_left_pins_left_edge_and_keeps_y() {
        let mut world = World::new();
        world.insert_resource(odd_edge_snapshot()); // rect(53, 37, 1877, 1043)
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(731, 500, 434, 687),
                Anchored(Anchor::Left),
            ))
            .id();

        // 新寸 (517×823): X=wa.left=53・Y=500 保持
        assert!(resize_window_to(&mut world, window, SizePx { w: 517, h: 823 }));
        assert_eq!(
            position_of(&world, window),
            Point { x: 53, y: 500 },
            "X=wa.left（左端固定・Right と取り違えたら 1877−517 で落ちる）・Y 保持"
        );
        assert_eq!(size_of(&world, window), SizeI::new(517, 823));
    }

    /// #2 Right resize（Req2.4）: `Anchored(Right)` を新寸へ resize すると `WindowPos.size`
    /// 新寸・`position.x = wa.right − w'`（右端固定）・`position.y` 保持で `true`。
    /// Left と取り違えれば X が `wa.left` になって落ちる（辺取り違え耐性）。
    #[test]
    fn resize_window_to_right_pins_right_edge_and_keeps_y() {
        let mut world = World::new();
        world.insert_resource(odd_edge_snapshot()); // rect(53, 37, 1877, 1043)
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(731, 500, 434, 687),
                Anchored(Anchor::Right),
            ))
            .id();

        // 新寸 (517×823): X = wa.right − w' = 1877 − 517 = 1360・Y=500 保持
        assert!(resize_window_to(&mut world, window, SizePx { w: 517, h: 823 }));
        assert_eq!(
            position_of(&world, window),
            Point { x: 1877 - 517, y: 500 },
            "X=wa.right−w'（右端固定・Left と取り違えたら 53 で落ちる）・Y 保持"
        );
        assert_eq!(size_of(&world, window), SizeI::new(517, 823));
    }

    /// #2 Free resize（Req2.5）: `Anchored(Free)` はアンカー辺を持たず position を
    /// 保持し、`WindowPos.size` のみ新寸へ反映する。size が変わるので冗長でなく
    /// `true`（書込あり）。Bottom へ取り違えれば position.y が動いて落ちる
    /// （射影なし・寸法反映のみの区別）。
    #[test]
    fn resize_window_to_free_keeps_position_and_updates_size_only() {
        let mut world = World::new();
        world.insert_resource(odd_edge_snapshot());
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(731, 500, 434, 687),
                Anchored(Anchor::Free),
            ))
            .id();

        // Free: 射影なし＝position 不変・size のみ新寸（size 変化ゆえ冗長でなく true）
        assert!(resize_window_to(&mut world, window, SizePx { w: 517, h: 823 }));
        assert_eq!(
            position_of(&world, window),
            Point { x: 731, y: 500 },
            "Free は position 再計算なし（現在位置保持・Bottom 取り違えなら Y が動く）"
        );
        assert_eq!(size_of(&world, window), SizeI::new(517, 823));
    }

    /// #3 随伴バルーン維持（非 Bottom・Req2.6）: `Anchored(Left)`＋`BalloonFollow` の
    /// char 窓を resize すると、char は左端固定（Y 保持）へ移り、バルーンは
    /// `new_char_pos + offset` へ随伴し `balloon_pos − char_pos ≡ offset` を維持する
    /// （task 2.4 の Bottom 版と別アンカーで offset 恒等式を固定）。
    #[test]
    fn resize_window_to_left_preserves_balloon_follow_offset() {
        let mut world = World::new();
        world.insert_resource(odd_edge_snapshot()); // 左端 53
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
            .id();
        let offset = PointPx { x: -412, y: -25 };
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(731, 500, 434, 687),
                Anchored(Anchor::Left),
                BalloonFollow { balloon, offset },
            ))
            .id();

        // 新寸 (517×823) → char 左端固定 (53, 500)・balloon (53−412, 500−25)
        assert!(resize_window_to(&mut world, window, SizePx { w: 517, h: 823 }));
        let char_pos = position_of(&world, window);
        let balloon_pos = position_of(&world, balloon);
        assert_eq!(char_pos, Point { x: 53, y: 500 }, "左端固定・Y 保持");
        assert_eq!(
            balloon_pos,
            Point {
                x: 53 + offset.x,
                y: 500 + offset.y
            }
        );
        // offset 恒等式（balloon_pos − char_pos ≡ offset）の維持
        assert_eq!(balloon_pos.x - char_pos.x, offset.x);
        assert_eq!(balloon_pos.y - char_pos.y, offset.y);
    }

    /// #4 べき等（非 Bottom・Req3.1）: 既に左端一致（x=wa.left）の位置＋同寸へ
    /// `Anchored(Left)` を resize すると、導出 (position, size) が現在値と同一ゆえ
    /// 書込なし・`false`・状態不変（Bottom 版 idempotent の非 Bottom 対応・
    /// 同一寸法/同一アンカーの再適用が窓状態を変更しない＝冗長書込をしない）。
    #[test]
    fn resize_window_to_left_is_idempotent_on_same_size_and_position() {
        let mut world = World::new();
        world.insert_resource(odd_edge_snapshot()); // 左端 53
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(53, 500, 517, 823), // 既に左端射影済み・同寸
                Anchored(Anchor::Left),
            ))
            .id();

        // 同寸・既に左端一致 → 導出 (53,500)＋(517,823) は現在値と同一 → 書込なし・false
        assert!(!resize_window_to(&mut world, window, SizePx { w: 517, h: 823 }));
        assert_eq!(position_of(&world, window), Point { x: 53, y: 500 });
        assert_eq!(size_of(&world, window), SizeI::new(517, 823));
    }

    /// #4 非 Bottom 縮退（Req3.3/3.4）: 縮退経路がアンカー非依存（Bottom 特化でない）
    /// ことを代表として Top で固定する。task 2.4 が Bottom で押さえた縮退を、
    /// 別アンカーでも配線が同一であることの確認（過剰重複を避け 1 件へ集約）。
    /// - 非正寸（w≤0 or h≤0）: project_anchor 前に弾かれ `false`・位置/寸不変。
    /// - `WindowHandle` 未付与: 射影は走るが enqueue が warn no-op＝`false`・位置/寸不変。
    #[test]
    fn resize_window_to_non_bottom_degrades_on_nonpositive_and_missing_handle() {
        let mut world = World::new();
        world.insert_resource(odd_edge_snapshot());

        // (a) Top＋非正寸: project_anchor 前に弾かれ false・状態不変（Bottom と同一縮退）
        let with_handle = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(731, 500, 434, 687),
                Anchored(Anchor::Top),
            ))
            .id();
        for bad in [
            SizePx { w: 0, h: 823 },
            SizePx { w: 517, h: 0 },
            SizePx { w: -517, h: -823 },
        ] {
            assert!(
                !resize_window_to(&mut world, with_handle, bad),
                "{bad:?}: 非正寸は false（Top でも Bottom と同一縮退）"
            );
            assert_eq!(position_of(&world, with_handle), Point { x: 731, y: 500 });
            assert_eq!(size_of(&world, with_handle), SizeI::new(434, 687));
        }

        // (b) Top＋WindowHandle 未付与: 射影は走るが enqueue が warn no-op＝false・状態不変
        let no_handle = world
            .spawn((
                // WindowHandle なし（窓生成前）
                window_pos_sized(731, 500, 434, 687),
                Anchored(Anchor::Top),
            ))
            .id();
        assert!(!resize_window_to(&mut world, no_handle, SizePx { w: 517, h: 823 }));
        assert_eq!(position_of(&world, no_handle), Point { x: 731, y: 500 });
        assert_eq!(size_of(&world, no_handle), SizeI::new(434, 687));
    }
}
