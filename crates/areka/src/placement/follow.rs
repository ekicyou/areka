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
//!   その bottom 吸着実装（[`project_anchor`] の `Bottom` 腕が委譲）。非 Free
//!   アンカーのキャラ窓は `DragConfig{move_window:false}` で wndproc 移動を止め、
//!   [`on_char_drag`]／[`on_char_drag_end`] が [`Anchored`] を読んで [`project_anchor`]
//!   適用済み座標を**単一ライター**として書く（v1 の事後再釘付けは wndproc と
//!   競合し振動→撤去）
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
use bevy_ecs::system::SystemState;
use tracing::{debug, warn};
use windows::Win32::UI::WindowsAndMessaging::{
    CW_USEDEFAULT, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER,
};
use wintf::ecs::drag::{DragEndEvent, DragEvent, DraggingState};
use wintf::ecs::layout::{Arrangement, Offset};
use wintf::ecs::pointer::Phase;
use wintf::ecs::window::monitor::Monitor;
use wintf::ecs::{DPI, Point, SetWindowPosCommand, SizeI, WindowHandle, WindowPos};

use super::diag::{self, DESPAWNED_SKIP_TAG, PlacementRoute, WindowKind, WindowMoveRecord};
use super::persist::{
    balloon_offset_entries, balloon_offset_to_persist, char_pos_entries, char_pos_to_origin_x,
    persist_entries,
};
use super::resolver::{Anchor, PointPx, RectPx, SizePx};
use super::spawn::{BalloonWindowMarker, CharWindowMarker};

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
#[allow(dead_code)] // spawn 付与（task 3.1）は後続 task の領分——構築が付くまで dead_code 警告を抑える
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
/// # 非 Free アンカーのキャラ窓（4.7/1.6・DD15 v2・単一ライター）
///
/// 分岐は [`Anchored`]（drag／resize が読む単一真実源）で判定する。非 `Free`
/// （`Bottom` 等）のキャラ窓は `DragConfig { move_window: false }` で spawn され、
/// wndproc は窓を動かさない。本ハンドラが唯一のライターとして、[`DraggingState`]＋
/// DragEvent のカーソル座標から生ドラッグ座標を復元し（[`policy_mapped_position`]）、
/// [`project_anchor`] 適用済み（`Bottom` は [`BottomSnapPolicy`] へ委譲）の座標を
/// **一度だけ**書く——反映段階で既に正しい座標が確定しているため、v1 の
/// 「wndproc 移動→事後再釘付け」の毎サイクル振動は原理的に起きない。ドラッグと
/// リサイズが同一 [`project_anchor`] を通ることで座標系変換を二重化しない（Req1.6）。
/// モニタ跨ぎ再吸着は射影の live 算出が担う。
///
/// # Free キャラ窓／`Anchored` 不在（挙動不変・wndproc 委譲）
///
/// `Anchored(Free)`（および安全側フォールバックとして `Anchored` 不在）のキャラ窓は
/// `DragConfig { move_window: true }` のまま wndproc レベルで移動済み。本ハンドラは
/// wndproc が更新した `WindowPos.position`（物理 px）を読むだけで窓を書かない
/// （wndproc の領分）。
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

            // 分岐は Anchored（drag／resize が読む単一真実源）で判定する（Req1.6）。
            // 非 Free → project_anchor 単一ライター／Free・Anchored 不在 → wndproc 委譲
            // （不在も Free と同じく wndproc へ倒す＝旧「marker 無し＝Free」意味論の保存・安全側）。
            let anchor = world.get::<Anchored>(entity).map(|a| a.0);
            let pos = match anchor {
                Some(anchor) if anchor != Anchor::Free => {
                    // 単一ライター経路: 生ドラッグ座標→アンカー射影 T 適用済み座標を書く
                    let Some(mapped) = policy_mapped_position(world, entity, anchor, ev.position)
                    else {
                        return false;
                    };
                    if !enqueue_window_set_pos(world, entity, mapped.x, mapped.y, None, None) {
                        return false;
                    }
                    mapped
                }
                _ => {
                    // Free・Anchored 不在: wndproc（move_window=true）が移動済みの位置を読むだけ
                    let Some(pos) = world.get::<WindowPos>(entity).and_then(|wp| wp.position) else {
                        return false;
                    };
                    pos
                }
            };

            // 引き金はユーザーの明示的なドラッグ＝遷移ガード適用外（task 6.2・Req 3.1）。
            follow_balloon(world, entity, pos, BalloonFollowTrigger::Drag);
            false
        }
    }
}

/// `OnDragEnd` ハンドラ: **全アンカー種別**のキャラ窓の最終カーソル位置へ同写像を
/// 適用し、確定位置を永続へ write-through する（4.7/1.6・1.1/1.9・DD15 v2 (3)・
/// design C2）。分岐は [`Anchored`] で判定する。
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
/// # 全アンカー結線と Free の保存専用アーム（1.1・design C2）
///
/// spawn は Free 含む**全**キャラ窓へ本ハンドラを結線する（吸着はドラッグ中の制約で
/// あって保存条件ではない・1.1）。非 Free は [`project_anchor`] でアンカー辺へ最終
/// 再固定し、Free は射影が identity ゆえ `mapped` = wndproc が動かし切った確定位置と
/// なり（`enqueue_window_set_pos`／`follow_balloon` は identity 再釘付けで無害通過）、
/// 本ハンドラがそのまま**保存専用アーム**として働く。[`Anchored`] 不在（防御）は生
/// ドラッグ座標を復元する基準アンカーが無いため skip する（Req1.6）。
///
/// # 保存フック（1.1/1.9/7.1・design C2）
///
/// `mapped` 確定・`enqueue_window_set_pos`・`follow_balloon` の**後**に、当該窓の
/// [`CharWindowMarker`]`.scope`（`usize`→`u32`）で `mapped` を [`char_pos_entries`]→
/// [`persist_entries`] へ渡し、Ghost 永続スコープへ即時 write-through する（fire-and-forget・
/// 非ブロッキング）。marker 不在（防御）は `debug!`＋skip（panic しない）。永続の
/// 窓位置を書くのはこの DragEnd 観測点のみ——`on_char_drag`（ドラッグ中）・
/// [`move_window_to`]・[`resize_window_to`]・復元時再射影は書かない（発火規律・Req1.9）。
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
            // 全アンカー種別のキャラ窓が最終位置を確定する（1.1: 吸着はドラッグ中の
            // 制約であって保存条件ではない）。非 Free は project_anchor でアンカー辺へ
            // 再固定し、Free は identity 射影ゆえ mapped＝wndproc 確定位置を素通しする
            // （保存専用アーム・射影段は無害通過）。Anchored 不在（防御）は生座標を
            // 復元する基準アンカーが無いため skip する（Req1.6）。
            let Some(anchor) = world.get::<Anchored>(entity).map(|a| a.0) else {
                return false;
            };
            // 最終位置の第一義は DraggingState からの生座標再導出（最終 DragEvent 欠落の穴埋め・
            // ev.position が真の最終カーソル）。ただし保存を DraggingState 依存にすると、dispatch が
            // DragEnd 前に DraggingState を落とした場合（多窓時に observed・実 flow の穴）に、連続
            // on_char_drag が既に最終位置へ動かした char の位置保存が丸ごと落ちる——一方 balloon 側は
            // char の WindowPos.position を読んで offset を保存するため、balloon-offset だけが残り
            // window が欠落する（実機 sylphya.toml の [window.0] 欠落／[balloon-offset.0] 残存）。
            // Req1.6（位置の単一真実源はキャラ窓）を守るには「ドラッグした char は必ず保存する」が
            // 要る。よって DraggingState 不在時は再導出済みの最終位置＝現 WindowPos.position（非 Free は
            // on_char_drag が project_anchor 適用済み・Free は wndproc 確定位置）へ縮退して保存する。
            let mapped = match policy_mapped_position(world, entity, anchor, ev.position) {
                Some(mapped) => mapped,
                None => {
                    let Some(pos) = world.get::<WindowPos>(entity).and_then(|wp| wp.position) else {
                        debug!(
                            ?entity,
                            "DraggingState も WindowPos.position も無いため位置保存を skip（防御・no-op）"
                        );
                        return false;
                    };
                    pos
                }
            };
            if !enqueue_window_set_pos(world, entity, mapped.x, mapped.y, None, None) {
                return false;
            }
            // 引き金はユーザーの明示的なドラッグ＝遷移ガード適用外（task 6.2・Req 3.1）。
            follow_balloon(world, entity, mapped, BalloonFollowTrigger::Drag);

            // 保存フック（1.1/1.9/7.1・design C2）: mapped 確定後に当該スコープの
            // WindowPos entries を Ghost 永続スコープへ即時 write-through 投函する。
            // スコープは CharWindowMarker から逆引き（usize→u32）。marker 不在（防御）は
            // debug＋skip（panic しない）。発火はこの DragEnd 観測点のみ（Req1.9）。
            match world.get::<CharWindowMarker>(entity).map(|m| m.scope) {
                Some(scope) => {
                    // 原点（下端中央）基準へ移してから保存する。左上 x のまま保存すると、
                    // サーフェス寸が変わったとき「同じ左上」が別の中央を指し、復元で
                    // キャラ・バルーンが横へずれる（実機: むらさき 382 で保存→434 で復元）。
                    // 現寸が読めないときは左上のまま（防御・従来挙動）。
                    let char_size = world
                        .get::<WindowPos>(entity)
                        .and_then(|wp| wp.size)
                        .map(|s| SizePx {
                            w: s.width,
                            h: s.height,
                        });
                    let saved = match char_size {
                        Some(size) => char_pos_to_origin_x(
                            anchor,
                            PointPx {
                                x: mapped.x,
                                y: mapped.y,
                            },
                            size,
                        ),
                        None => PointPx {
                            x: mapped.x,
                            y: mapped.y,
                        },
                    };
                    // 保存の計測ログ（実機診断・保存↔復元の座標突合）: char_x/y は左上、
                    // saved_x は実際に永続へ書く原点（下端中央）基準の x。
                    tracing::info!(
                        target: "areka::persist::save",
                        scope,
                        char_x = mapped.x, char_y = mapped.y,
                        saved_x = saved.x, saved_y = saved.y,
                        char_w = ?char_size.map(|s| s.w),
                        ?anchor,
                        "char DragEnd 保存"
                    );
                    let entries = char_pos_entries(scope as u32, saved);
                    persist_entries(world, entries);
                }
                None => {
                    debug!(
                        ?entity,
                        "CharWindowMarker 不在のため位置保存を skip（防御・no-op）"
                    );
                }
            }
            false
        }
    }
}

/// 非 Free アンカーのキャラ窓の「カーソル座標→アンカー射影 T 適用済み窓位置」
/// （DD15 v2・Req1.6）。`anchor` は呼び出し側が [`Anchored`] から読んだ現在アンカー。
///
/// 生ドラッグ座標（＝move_window=true なら wndproc が書いたであろう位置）を
/// wndproc と同じ式で復元する: `initial_window_pos + (cursor − drag_start)`。
/// [`DraggingState`] の `initial_inset` は wintf dispatch が「ドラッグ開始時の
/// 窓位置」を転記したもの（フィールド名は歴史的経緯・dispatch.rs 参照）。復元した
/// 生座標へ [`project_anchor`] を適用する——リサイズ（[`resize_window_to`]）と同一の
/// 射影 T を通し、座標系変換を二重化しない（Req1.6）。
///
/// `None` は「[`DraggingState`] 不在で生座標を復元できない」場合のみ（実 flow では
/// dispatch が DragEvent より先に挿入するため起きない・`debug!` の上で no-op）。
/// 寸法不明・snapshot 不在は [`project_anchor`] が identity へ縮退する。
fn policy_mapped_position(
    world: &World,
    entity: Entity,
    anchor: Anchor,
    cursor: Point,
) -> Option<Point> {
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
        // 不在は非正寸法（＝寸法不明）として project_anchor の identity 縮退へ委ねる
        .unwrap_or(SizePx { w: 0, h: 0 });
    // drag と resize（resize_window_to）が同一 project_anchor を通ることで座標系変換を
    // 二重化しない（Req1.6）。bottom は project_anchor 内で BottomSnapPolicy へ委譲。
    let mapped = project_anchor(anchor, raw, size, world.get_resource::<MonitorSnapshot>());
    Some(Point {
        x: mapped.x,
        y: mapped.y,
    })
}

/// 随伴バルーンの**引き金**（task 6.2・S3′ 是正・Req 3.4）。
///
/// # なぜ「書込自身の route」では決められないのか
///
/// [`follow_balloon`] の書込は定義上つねに [`PlacementRoute::BalloonFollow`] であり、
/// その語からは「なぜバルーンが動いたのか」——ユーザーがキャラをドラッグしたのか、
/// 配置系（DPI 再射影・再スナップ等）が勝手に動かしたのか——を復元できない。しかし
/// 遷移ガードの発火可否はまさにそこで反転する（Req 3.1 の「ユーザーの明示的なドラッグ
/// 以外の要因」）。ゆえに引き金は**呼出元しか知らない情報**であり、引数で配管するほか
/// ない（[`route_applies_visibility_guard`] を `BalloonFollow` に当てると常に偽＝
/// バルーンのガードが恒久的に無効になり、逆に無条件適用するとドラッグ随伴でバルーンが
/// 引き戻されて明示操作の尊重が壊れる）。
///
/// # 網羅 `match` で書く理由
///
/// [`route_applies_visibility_guard`] と同じ流儀。引き金の種類が増えたとき、既定腕が
/// あると新しい引き金が黙って片側へ倒れる（D14 帰結⑵）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BalloonFollowTrigger {
    /// ユーザーの明示的なドラッグ（[`on_char_drag`]／[`on_char_drag_end`]）。
    /// 引き戻しは明示操作の否定ゆえ**ガード適用外**（requirements.md Boundary Context）。
    Drag,
    /// 配置系の書込（[`resize_window_to`]）に随伴した。内包する route は**キャラ窓を
    /// 動かした経路**であり、適用可否はキャラ窓と同じ表（D13 帰結⑴⑵）で決まる。
    Placement(PlacementRoute),
}

impl BalloonFollowTrigger {
    /// この引き金の随伴でバルーン矩形へ遷移ガードを適用するか。
    fn applies_visibility_guard(self) -> bool {
        match self {
            BalloonFollowTrigger::Drag => false,
            BalloonFollowTrigger::Placement(route) => route_applies_visibility_guard(route),
        }
    }
}

/// 確定済みキャラ窓座標 `pos` を基準に随伴バルーンを追従させる（4.2・U4）。
///
/// [`BalloonFollow`] が無ければ no-op。[`on_char_drag`]／[`on_char_drag_end`]／
/// [`resize_window_to`] の共通後段。
///
/// 経路タグ（Req 1.2・task 1.4）は定義上つねに [`PlacementRoute::BalloonFollow`]
/// （経路語彙が関数名そのもの）ゆえ引数で受けない。呼出元がドラッグ経路
/// （route 語彙なし）であっても**随伴バルーンの書込は BalloonFollow として記録される**
/// ——これは Req 2.5（バルーン消失がキャラ追従の随伴か）の判別材料そのものである。
///
/// # 可視性の遷移ガード（S3′ 是正・task 6.2・Req 3.4）
///
/// offset 恒等式（`pos + offset`）が出した提案位置は、キャラ窓が可視のままでも
/// offset ぶん外側のバルーンだけを全 work area 非交差へ落とし得る——「キャラは見えて
/// いるのに会話が読めない」状態であり、Req 3.4 が名指しで防ぐものである。よって恒等式の
/// **後**に、キャラ窓とまったく同一の純関数（[`guard_visibility`]）・同一の遷移規則を
/// バルーン矩形へ適用する（[`guard_balloon_position`]）。発火可否は書込自身の route では
/// なく `trigger`（随伴の引き金）が決める。
fn follow_balloon(world: &mut World, entity: Entity, pos: Point, trigger: BalloonFollowTrigger) {
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
    // 相対位置の恒等式（4.4: `balloon_pos − char_pos ≡ offset`）が出す提案位置。
    let proposed = PointPx {
        x: pos.x + follow.offset.x,
        y: pos.y + follow.offset.y,
    };
    // 遷移ガード（S3′ 是正）。適用外の引き金では 1 bit も触らない。
    let decided = if trigger.applies_visibility_guard() {
        guard_balloon_position(world, follow.balloon, proposed)
    } else {
        proposed
    };
    enqueue_window_set_pos(
        world,
        follow.balloon,
        decided.x,
        decided.y,
        None,
        Some(PlacementRoute::BalloonFollow),
    );
}

/// バルーン矩形へ可視性の遷移ガードを適用する（task 6.2・S3′ 是正・Req 3.4）。
///
/// キャラ窓側（[`resize_window_to`] 手順 3a／3c）と**完全に同一の規則・同一の純関数**で
/// あり、違うのは矩形の組み方だけである——バルーンには射影 T が無く、位置は offset 恒等式
/// が決める従属量ゆえ:
///
/// - **旧矩形** = 現在位置（`WindowPos.position`）× 現寸。[`follow_balloon`] は移動専用
///   （`SWP_NOSIZE`）で寸を変えないため、新旧で同じ寸を使うのが正しい。
/// - **`raw`（clamp 先の引き元）** = 提案位置そのもの。キャラ窓では「射影 T が Y に用いた
///   矩形」を貫通させる必要があるが、バルーンには射影段が無く、提案位置以外に clamp 先を
///   決める基準が存在しない（別の矩形から引き直すと [`guard_visibility`] の事後条件が崩れる）。
///
/// # 縮退（log-first・Req 3.3／6.2/6.3）
///
/// - **バルーン entity 破棄済み**: 終了処理でゴースト窓が破棄された後のフレームでも随伴は
///   走り得る＝**正常終了系**ゆえ [`DESPAWNED_SKIP_TAG`] の `debug!` で打ち切り、提案位置を
///   そのまま返す（task 3.2 と同じ区別。ここを `warn!` にすると良性ノイズが本物の異常を埋める）。
/// - **寸が未確定**: 矩形を組めず交差判定が成立しないため、位置には**一切手を入れず**
///   `warn!` を残す。未確定は `Option::None` **だけではない**——`WindowPos::default()` は
///   寸を `Some(SizeI { CW_USEDEFAULT, .. })`（`i32::MIN` センチネル）で持ち、素の矩形として
///   交差判定へ入れると `saturating_add` で逆転矩形になって判定が丸ごと意味を失う
///   （[[4.6 の教訓]]・キャラ窓側の手順 3a と同型のガード）。
///
/// # 縮退シーム（美観配置政策の先送り・Req 3.4 の範囲）
///
/// 本ガードが持つのは「完全不可視への遷移を防ぐ安全網」までである。clamp によりバルーンが
/// キャラと部分的に重なり得ることは**許容する**（*見えない会話*より*重なった会話*を優先する
/// 裁定・design「バルーン適用（S3′ 是正）」）。画面端での左右反転など SSP 互換の美観配置政策は
/// 本 spec の対象外（M2）であり、**`ClampX` の `warn!` がその先送りの縮退シーム**である
/// ——「安全網が働いた＝本来なら美観政策が要る局面」を実機ログに残す
/// （diagnosis-report.md §1.4「縮退シームの明示」）。
fn guard_balloon_position(world: &World, balloon: Entity, proposed: PointPx) -> PointPx {
    if world.get_entity(balloon).is_err() {
        debug!(
            entity = ?balloon,
            "{DESPAWNED_SKIP_TAG} 随伴バルーンは既に破棄済み（despawn）→ 可視性の遷移ガードを正常系として打ち切り"
        );
        return proposed;
    }

    let window_pos = world.get::<WindowPos>(balloon);
    // 非正寸（`None` と `CW_USEDEFAULT` センチネル＝`i32::MIN` の双方）は
    // 「寸が未確定」＝矩形を組めない＝判定不能。
    let Some(size) = window_pos
        .and_then(|wp| wp.size)
        .filter(|s| s.width > 0 && s.height > 0)
        .map(|s| SizePx {
            w: s.width,
            h: s.height,
        })
    else {
        // `route` は [`evaluate_visibility_guard`] が出す同タグ行と**同じフィールド名**で
        // 載せる（実機では 3 語を接頭辞で一括 grep してから `route=` で窓種別へ振り分ける
        // ＝`diagnosis-procedure.md` §3.1「件数の読み方」）。落とすと当該行だけが
        // 「route を持たない行」になり、手順書の振り分け規則が静かに嘘になる。
        // **`proposed` の有無が本行（良性の判定不能）と装置異常（`MonitorSnapshot`
        // 不在・モニタ 0 台）を分ける唯一の判別子**であり、フィールド集合は檻
        // `balloon_undetermined_size_*`／`missing_monitor_snapshot_*` が固定している。
        warn!(
            entity = ?balloon,
            route = ?PlacementRoute::BalloonFollow,
            ?proposed,
            "{VISIBILITY_UNRESOLVED_TAG} バルーン窓の寸が未確定（窓生成前／CW_USEDEFAULT センチネル）のため可視性を判定できない → 提案位置を変更しない"
        );
        return proposed;
    };

    // 旧矩形＝**現在位置** × 現寸。位置が未確定なら旧矩形不明＝安全側 clamp。
    // 負座標は正当（左モニタは `-1920..0`）ゆえ、位置の未確定は符号ではなく
    // wintf 正典のセンチネル `CW_USEDEFAULT` そのもので判定する
    // （`crates/wintf/src/ecs/graphics/systems/window_pos.rs:41`
    //   ／`crates/wintf/src/ecs/layout/systems/monitor_systems.rs:408`
    // と同じ式。素通しすると `WindowPos::default()` 由来の窓が
    // 「もともと画面外に留置されていた」と誤判定され安全側の腕が死ぬ）。
    let old_rect = window_pos
        .and_then(|wp| wp.position)
        .filter(|p| p.x != CW_USEDEFAULT && p.y != CW_USEDEFAULT)
        .map(|p| rect_at(PointPx { x: p.x, y: p.y }, size));

    evaluate_visibility_guard(
        balloon,
        PlacementRoute::BalloonFollow,
        world.get_resource::<MonitorSnapshot>(),
        old_rect,
        // バルーンには射影段が無く、clamp 先は提案位置からしか引けない（doc 参照）。
        proposed,
        proposed,
        size,
    )
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

/// `OnDragEnd` ハンドラ: バルーン単独ドラッグ確定 offset の永続 write-through
/// （2.1・8.1・design C2/C3・task 2.3）。
///
/// [`on_balloon_drag`] が**連続**イベント（ドラッグ中の in-session offset 更新）で
/// あるのに対し、本ハンドラは DragEnd の**確定**観測点でのみ発火する保存トリガである
/// （1 ドラッグ＝1 書込・発火規律）。永続へバルーン offset を書くのはこの観測点のみ。
///
/// # 最終確定位置の源（`on_char_drag_end` との差）
///
/// バルーン窓は `DragConfig { move_window: true }` ゆえ wndproc が実窓位置を
/// `WindowPos.position` へ更新済み——DragEnd 時点の**最終確定位置**はこの
/// `WindowPos.position` で読める（[`on_balloon_drag`] と同源）。したがって
/// [`on_char_drag_end`] のように `DraggingState`＋`ev.position`（カーソル）から生座標を
/// 再構成する必要はなく、`ev.position` は使わない（char 窓は move_window=false で
/// wndproc 移動が無いため cursor 再構成が要るが、balloon は move_window=true で窓自身が
/// 動くため WindowPos が確定位置＝「最終確定位置」意味論は両者で一致する）。
///
/// # 保存値の導出（in-session offset を使わない・design 検証 Issue 1 対応）
///
/// 追従元キャラ窓（`BalloonFollow.balloon == 自バルーン`）を逆引きし、その最終
/// `char_pos`（`WindowPos.position`）・`anchor`（[`Anchored`]）・`char_size`
/// （`WindowPos.size`）を読む。offset は [`on_balloon_drag`] と**同式**
/// `offset_tl = balloon_pos − char_pos`（左上基準・物理 px・再スケールなし・U4）で
/// **最終確定位置から再導出**する——in-session の `BalloonFollow.offset`（連続ドラッグ中の
/// 表現）は**流用しない**（最後の OnDrag 配信と最終確定位置はずれ得るため）。導出した
/// 左上基準 offset を [`balloon_offset_to_persist`]`(anchor, offset_tl, char_size)` で
/// サーフェス寸不変なアンカー辺基準へ移し、[`balloon_offset_entries`]→[`persist_entries`]
/// で Ghost 永続スコープへ即時 write-through する（fire-and-forget・非ブロッキング）。
/// scope はバルーン窓自身の [`BalloonWindowMarker`]（追従元 char の
/// [`CharWindowMarker`] と同番号）。
///
/// # 縮退（panic しない・log-first・no-op）
///
/// バルーンの `WindowPos.position` 不在／[`BalloonWindowMarker`] 不在／追従元キャラ窓
/// （その `WindowPos.position`・[`Anchored`]・`WindowPos.size` のいずれか）不在は
/// `debug!`＋skip する。`BalloonFollow.offset`（in-session 表現）は本ハンドラでは
/// 変異させない（連続ドラッグ側 [`on_balloon_drag`] が所有）。
///
/// イベントは消費しない（常に `false`＝伝播続行。[`on_balloon_drag`] と同じ規約）。
pub(crate) fn on_balloon_drag_end(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<DragEndEvent>,
) -> bool {
    match ev {
        Phase::Tunnel(_) => false,
        Phase::Bubble(ev) => {
            // 他 entity 宛イベントには何もしない（ハードニング・on_balloon_drag と同じ規約）
            if ev.target != entity {
                return false;
            }

            // 最終確定位置＝バルーン窓の WindowPos.position（wndproc が move_window=true で
            // 更新済み・on_balloon_drag と同源。ev.position(cursor) は使わない）。
            let Some(balloon_pos) = world.get::<WindowPos>(entity).and_then(|wp| wp.position) else {
                debug!(
                    ?entity,
                    "バルーン窓の WindowPos.position 不在のため offset 保存を skip（防御・no-op）"
                );
                return false;
            };

            // scope はバルーン窓自身の BalloonWindowMarker（追従元 char と同番号）。
            let Some(scope) = world.get::<BalloonWindowMarker>(entity).map(|m| m.scope) else {
                debug!(
                    ?entity,
                    "BalloonWindowMarker 不在のためバルーン offset 保存を skip（防御・no-op）"
                );
                return false;
            };

            // BalloonFollow.balloon == 自バルーンのキャラ窓を逆引きし、最終 char_pos・anchor・
            // char_size を読む（in-session の BalloonFollow.offset は SAVE に使わない）。
            let mut chars = world.query::<(&BalloonFollow, &WindowPos, &Anchored)>();
            let mut found: Option<(Point, Anchor, SizePx)> = None;
            for (follow, char_wp, anchored) in chars.iter(world) {
                if follow.balloon != entity {
                    continue;
                }
                let Some(char_pos) = char_wp.position else {
                    continue;
                };
                let Some(size) = char_wp.size else {
                    continue;
                };
                found = Some((
                    char_pos,
                    anchored.0,
                    SizePx {
                        w: size.width,
                        h: size.height,
                    },
                ));
                break;
            }
            let Some((char_pos, anchor, char_size)) = found else {
                debug!(
                    ?entity,
                    "追従元キャラ窓（位置/anchor/寸法）不在のためバルーン offset 保存を skip（防御・no-op）"
                );
                return false;
            };

            // offset_tl = balloon_final_pos − char_pos（左上基準・on_balloon_drag と同式）。
            // 不変条件: 両者とも仮想スクリーン座標範囲のため減算は i32 を溢れない
            // （溢れは入力源の異常・on_balloon_drag と同じ流儀）。
            debug_assert!(
                balloon_pos.x.checked_sub(char_pos.x).is_some()
                    && balloon_pos.y.checked_sub(char_pos.y).is_some(),
                "window positions out of virtual-screen range: {balloon_pos:?} - {char_pos:?}"
            );
            let offset_tl = PointPx {
                x: balloon_pos.x - char_pos.x,
                y: balloon_pos.y - char_pos.y,
            };

            // アンカー辺基準へ変換（保存方向・サーフェス寸不変）→ BalloonOffset entries を
            // Ghost 永続スコープへ即時 write-through（fire-and-forget・7.1）。
            let persist = balloon_offset_to_persist(anchor, offset_tl, char_size);
            // 保存の計測ログ（実機診断・保存↔復元の座標突合）: balloon_pos＝バルーン最終位置、
            // char_pos＝追従元 char の最終位置、offset_tl＝左上基準差分、persist＝アンカー辺基準
            // （BalloonOffset entries として書かれる値）、char_size＝offset 逆変換で使う寸。
            tracing::info!(
                target: "areka::persist::save",
                scope,
                balloon_x = balloon_pos.x, balloon_y = balloon_pos.y,
                char_x = char_pos.x, char_y = char_pos.y,
                offset_tl_x = offset_tl.x, offset_tl_y = offset_tl.y,
                persist_x = persist.x, persist_y = persist.y,
                char_w = char_size.w, char_h = char_size.h,
                ?anchor,
                "balloon DragEnd 保存"
            );
            let entries = balloon_offset_entries(scope as u32, persist);
            persist_entries(world, entries);
            false
        }
    }
}

/// R7 公開 API: UI スレッド上で呼ばれる窓移動関数（物理 px・スクリーン座標直渡し・7.1）。
///
/// - 移動は `SetWindowPosCommand`（`SWP_NOSIZE|SWP_NOZORDER|SWP_NOACTIVATE`）経由。
///   座標は物理 px 素通し（U4・再スケールなし）
/// - 対象が [`BalloonFollow`] を持つ場合はバルーン窓も offset 維持で随伴移動する
/// - `WindowHandle` 未付与（窓生成前）は `warn!` して `false` を返す（silent no-op に
///   しない）。対象が既に破棄済み（entity 不在）なら**正常終了系**ゆえ `debug!` で
///   打ち切って `false`（task 7.3・Req 6.2）。いずれも随伴バルーンは動かさない
/// - 随伴バルーン側の `WindowHandle` 未付与は `warn!` のみ（対象自身の移動は成立
///   しているため戻り値は `true`）
///
/// 消費者: `emo2_boot::move_cue::apply_move_directive`（`\![move]` の UI スレッド適用・
/// task 7.4）が唯一の位置ライターとして本 API を呼ぶ。
///
/// # 経路タグ（Req 1.2／2.4・D13・task 1.4）
///
/// 対象窓の書込は [`PlacementRoute::MoveCue`]・随伴バルーンは
/// [`PlacementRoute::BalloonFollow`] として記録する。唯一の消費者が `\![move]` cue である
/// ため route は引数で受けない（呼出側が渡せる値が 1 つしか無く、取り違えの余地だけを
/// 増やす＝`resize_window_keep_position` と同じ判断）。**スクリプトの明示操作**ゆえ
/// 遷移ガードは適用しない（ドラッグ・`Restore` と同族・D13 帰結⑵）。
pub fn move_window_to(world: &mut World, window: Entity, x: i32, y: i32) -> bool {
    let follow = world.get::<BalloonFollow>(window).copied();

    if !enqueue_window_set_pos(world, window, x, y, None, Some(PlacementRoute::MoveCue)) {
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
        enqueue_window_set_pos(
            world,
            follow.balloon,
            x + follow.offset.x,
            y + follow.offset.y,
            None,
            Some(PlacementRoute::BalloonFollow),
        );
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
/// - **対象 entity 不在（despawn 済み）**: 終了処理でゴースト窓が破棄された後のフレーム
///   ＝**正常終了系**ゆえ `debug!`（[`diag::DESPAWNED_SKIP_TAG`]）＋`false`（要件 6.2/6.3）。
///   直下の [`Anchored`] 欠落 `warn!` と**必ず区別する**——混ぜると終了時ログが良性ノイズで
///   埋まり、本物の結線バグ（実在窓の `Anchored` 欠落）が読めなくなる。
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
///
/// # `route` 引数（Req 1.2／2.4・design「PlacementRoute 配管＋guard_visibility >
/// Integration」・task 1.4）
///
/// 本関数は複数の上流（[`anchor_changed_system`]＝[`PlacementRoute::AnchorChange`]・
/// frame の毎フレーム再スナップ＝[`PlacementRoute::Resnap`]・frame の DPI 相＝
/// [`PlacementRoute::DpiReproject`]・frame の drain 相（寸法報告回収・`Changed<DPI>` 非依存）＝
/// [`PlacementRoute::ReportedSizeReconcile`]・D13）から呼ばれる**同一の反映口**であり、どの経路が
/// 書いたかは呼出側しか知らない。ゆえに経路は引数で受け、[`enqueue_window_set_pos`] の
/// 窓移動レコードへ透過させる（D11: ラッパ関数を乱立させない）。
///
/// **task 6.1 以後、route は観測語彙であると同時に挙動の入力でもある**——可視性の遷移
/// ガード（手順 3c・[`apply_visibility_guard`]）は非ドラッグの配置系 4 経路
/// （[`AnchorChange`](PlacementRoute::AnchorChange)／[`Resnap`](PlacementRoute::Resnap)／
/// [`DpiReproject`](PlacementRoute::DpiReproject)／
/// [`ReportedSizeReconcile`](PlacementRoute::ReportedSizeReconcile)）でのみ発火する
/// （D13 帰結⑴。同じ幾何でも由来が明示操作なら引き戻さない＝Req 3.1 の「ユーザーの
/// 明示的なドラッグ以外の要因」）。
///
/// **task 6.2 以後、route は随伴バルーンの発火条件でもある**——手順 7 の
/// [`follow_balloon`] へ [`BalloonFollowTrigger::Placement`]`(route)` として渡され、
/// バルーン矩形への遷移ガード（S3′・Req 3.4）が同じ 4 経路でのみ発火する。
#[allow(dead_code)] // 呼び出し側（anchor_changed_system task 2.6・frame resnap シーム）は後続 task の領分
pub fn resize_window_to(
    world: &mut World,
    char_window: Entity,
    new_size: SizePx,
    route: PlacementRoute,
) -> bool {
    // 0. 存在確認（要件 6.2/6.3・design D8 消費側）: 対象が既に despawn 済みなら
    //    **正常終了系**として debug で打ち切る。終了処理でゴースト窓が破棄された後の
    //    フレームでも寸法の再導出は走り得るため、ここを素通りさせると下の
    //    「Anchored 未付与」warn が破棄済み窓ぶんだけ鳴り、良性ノイズが本物の異常を
    //    埋める。区別すべきは水準であって、打ち切ること自体ではない——
    //    **実在する** entity の `Anchored` 欠落は下で従来どおり warn のままにする。
    if world.get_entity(char_window).is_err() {
        debug!(
            entity = ?char_window,
            ?route,
            "{DESPAWNED_SKIP_TAG} 対象 entity は既に破棄済み（despawn）→ アンカー保存リサイズを正常系として打ち切り"
        );
        return false;
    }

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
        // wintf の未確定表現は `Option::None` **だけではない**（D15・task 6.3）:
        // `WindowPos::default()` は position を `CW_USEDEFAULT`（`i32::MIN` センチネル）で
        // 持ち、`on_window_add` フックがそれを実際に挿す。素通しすると
        //   ① 手順 3a の `old_rect` が `i32::MIN` 近傍の全 work area 非交差矩形になり、
        //      `guard_visibility` が「もともと画面外に留置されていた」と誤読して `Keep`
        //      ＝**6.1 が敷いた安全側 clamp の腕が黙って死ぬ**
        //   ② 手順 3b の中央付替えと射影 T の入力（raw）も同時に汚染され、位置権威の
        //      無い窓へ clamp 由来の任意 X を書く＝位置権威の僭称
        // 位置未確定は「保存すべき接地点が存在しない」状態ゆえ、上の `Option::None` と
        // 同じ腕へ合流させて打ち切る（Req 3.3「必要な入力が取得できない場合は現状維持＋
        // 警告」）。**寸センチネルとの非対称は意図的**（D15 帰結⑴）——寸未確定は接地点が
        // 実在するので resize に意味があり、手順 3a の `old_rect` 不明＝安全側 clamp で扱う。
        //
        // 判定は**センチネル一致**で行う（wintf 正典 `window_pos.rs:41`／
        // `monitor_systems.rs:408` と同型——正典が見るのは `position.x` と `size.width`
        // で、ここは位置の両軸を見る点だけが異なる）。負座標そのものは正当（実機の左隣
        // モニタは負の X を持つ）ゆえ、符号や大きさの閾値で判定してはならない。
        if pos.x == CW_USEDEFAULT || pos.y == CW_USEDEFAULT {
            warn!(
                entity = ?char_window,
                position = ?pos,
                "WindowPos.position がセンチネル（位置未確定）＝窓生成前のため raw を導出できず resize しない"
            );
            return false;
        }
        (PointPx { x: pos.x, y: pos.y }, wp.size)
    };

    // 3a. 旧矩形（書込**前**の窓矩形）＝遷移ガードが「もともと見えていたか」を判定する入力
    //     （task 6.1・S3 是正）。手順 3b の付替えより**前**の生位置で組むこと——3b 後の
    //     値は「これから書こうとしている位置」であって旧矩形ではない。
    //
    //     寸が未確定のときは `None`＝**旧矩形不明**として扱い、ガードを安全側 clamp へ
    //     倒す。ここで注意すべきは、wintf の未確定表現が `Option::None` **だけではない**
    //     ことである——`WindowPos::default()` は `Some(SizeI { CW_USEDEFAULT, .. })`
    //     （`i32::MIN` センチネル）を持つ（`wintf::ecs::window::window_pos::WindowPos`
    //     の `Default`）。素の矩形として交差判定へ入れると `saturating_add` で逆転矩形に
    //     なり、「もともと画面外に留置されていた」と誤判定して尊重側（Keep）へ倒れる
    //     ＝安全側の腕が丸ごと死ぬ（4.6 で同型の見落としが本番 panic を新設した教訓）。
    let old_rect = match current_size {
        Some(s) if s.width > 0 && s.height > 0 => Some(rect_at(
            raw,
            SizePx {
                w: s.width,
                h: s.height,
            },
        )),
        _ => None,
    };

    // 3b. 原点＝**下端中央**の保存（伺かの立ち絵は足元中央が接地点・寸法変動で原点は動かない）。
    //     旧寸の中央 x を求め、新寸でも同じ中央になる左上 x へ付け替えてから射影へ渡す。
    //     これをしないと左上 x が据え置かれ、幅が変わるたびキャラの見た目の中心が横へ動き
    //     （実機: むらさきが surface0 434 → surface1000 382 で中心が 26px ずれる）、
    //     随伴バルーンも一緒に引きずられる。旧寸不明（窓生成直後等）は付け替えない。
    //     対象は**下端吸着（Bottom）のみ**——Free は「位置を一切動かさない」契約、
    //     Top/Left/Right は各アンカー辺（上端・左端・右端）が原点であって中央ではない。
    let raw = match (anchor, current_size) {
        (Anchor::Bottom, Some(old)) if old.width > 0 && new_size.w > 0 => {
            let center_x = raw.x.saturating_add(old.width / 2);
            PointPx {
                x: center_x.saturating_sub(new_size.w / 2),
                y: raw.y,
            }
        }
        _ => raw,
    };

    // 新位置 = アンカー射影 T（bottom は wa.bottom−h' 再計算・snapshot 不在は
    // project_anchor が identity 縮退）。drag と同一 T を呼び二重化しない（Req1.6）。
    // 下端は T が再導出し、中央 x は上の付け替えで維持される＝原点（下端中央）が不動。
    let snapshot = world.get_resource::<MonitorSnapshot>();
    let new_pos = project_anchor(anchor, raw, new_size, snapshot);

    // 3c. 可視性の遷移ガード（S3 是正・task 6.1・D5/D6・Req 3.1/3.2/3.3）: 射影 T の
    //     **下流・外側**で、非ドラッグの配置系 route のときだけ「可視 → 全 work area
    //     非交差」の遷移を X の clamp で阻止する（Y は射影の所有ゆえ不変）。射影関数
    //     自体の契約は変えない——`project_anchor` は幾何しか知らず、発火条件である
    //     route（＝書込の由来）を持たないためここでしか判定できない。
    //     べき等 skip より**前**に置くのは、clamp 結果が現在値と一致した走行で冗長な
    //     書込を出さないため（design Integration の手順どおり）。
    let new_pos = apply_visibility_guard(
        char_window,
        route,
        snapshot,
        old_rect,
        raw,
        new_pos,
        new_size,
    );

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
    if !enqueue_window_set_pos(
        world,
        char_window,
        new_pos.x,
        new_pos.y,
        Some(new_size),
        Some(route),
    ) {
        return false;
    }

    // 6. 随伴バルーン維持（Req2.6）＋**原点（下端中央）基準での相対維持**:
    //    セッション内 `BalloonFollow.offset` は左上基準表現ゆえ、寸法が変わると
    //    「左上からの距離」を保ったままバルーンが動いてしまう（実機: むらさきの
    //    surface0 434x687 → surface1000 382x547 で高さ差 140px・幅差 52px ぶん
    //    バルーンが引きずられた）。原点＝下端中央は寸法変動で動かないのだから、
    //    バルーンの相対位置も下端中央基準で不変であるべき。旧寸・新寸から原点差
    //    （Δ = 新原点 − 旧原点、左上基準の差分）を求め、offset をその逆方向へ
    //    付け替えて「下端中央からの相対位置」を保存する。旧寸不明なら従来どおり。
    //    対象は下端吸着（Bottom）のみ（他アンカーは原点が中央でないため従来どおり）。
    if let (Anchor::Bottom, Some(old)) = (anchor, current_size)
        && old.width > 0
        && old.height > 0
        && let Some(mut follow) = world.get_mut::<BalloonFollow>(char_window)
    {
        // offset は「char 左上からの差分」表現。原点（下端中央）からの相対を不変に保つには、
        //   旧原点相対 = offset_old + 旧左上 − 旧原点 = offset_old − (old.w/2, old.h)
        //   新 offset   = 旧原点相対 + 新原点 − 新左上 = 旧原点相対 + (new.w/2, new.h)
        // ゆえに offset += ((new.w/2 − old.w/2), (new.h − old.h)) が正しい変換
        // （原点が左上から見て遠ざかった分だけ、左上基準 offset は増える）。
        let d_origin_x = (new_size.w / 2) - (old.width / 2);
        let d_origin_y = new_size.h - old.height;
        if d_origin_x != 0 || d_origin_y != 0 {
            follow.offset = PointPx {
                x: follow.offset.x.saturating_add(d_origin_x),
                y: follow.offset.y.saturating_add(d_origin_y),
            };
        }
    }
    // 7. 確定後キャラ窓座標＋（補正済み）offset で追従（offset 恒等式維持）。
    //    引き金はキャラ窓を動かした route そのもの——バルーン矩形への遷移ガード
    //    （task 6.2・S3′）はこれで発火可否が決まる（書込自身の `BalloonFollow` では
    //    決められない・[`BalloonFollowTrigger`] の doc 参照）。
    follow_balloon(
        world,
        char_window,
        Point {
            x: new_pos.x,
            y: new_pos.y,
        },
        BalloonFollowTrigger::Placement(route),
    );

    true
}

/// アンカー変化トリガ（Req1.4・consumer 契約のみ・producer=seriko は非所有）。
///
/// `Changed<Anchored>` の char 窓それぞれについて、**現在の表示寸法**
/// （`WindowPos.size`＝新寸ではなく今まさに表示している寸法）を読み、
/// [`resize_window_to`] を呼んで**新しいアンカー**（変化後の [`Anchored`]）に対応する
/// 射影 T を再適用する——新しいアンカー辺を work area の対応辺へ合わせる。
/// アンカー値の解決（`seriko.alignmenttodesktop` 優先度チェーン）と
/// `\![set,alignmenttodesktop]` の cue routing は上流（parsers／seriko）の領分＝
/// 本 spec は非所有。本 system は `Anchored` の変化に反応する **consumer** に徹する
/// （design「System Flows > アンカー変化トリガ」・Req4.2）。schedule への登録（結線）は
/// main.rs／runtime 側の領分ゆえ、本 task は system の**定義のみ**を持つ
/// （`create_windows` が window_system.rs で定義のみ・登録は runtime、という repo 慣行）。
///
/// # 変更検知の永続性（毎フレーム全マッチしない・Req1.4 の要）
///
/// [`SystemState`] を [`Local`] に保持して `last_run` tick を run を跨いで引き継ぐ。
/// 毎 run で新規 `QueryState` を作ると `last_run` が過去 0 のまま `Changed` が全窓へ
/// 誤マッチするため不可。[`SystemState::get`] は fetch 後に `last_run` を進めるので、
/// 次 run 以降は「前回 run 以後に `Anchored` が変わった窓」だけがマッチする。
/// 初回 run は `SystemState::new` が `last_run` を過去へ置く仕様で全 char 窓が
/// マッチし得るが、[`resize_window_to`] が同寸・同位置でべき等 skip して吸収する
/// （design Implementation Notes）。
///
/// # 縮退（panic しない・log-first）
///
/// `WindowPos.size` 不在／未生成（窓生成前）の窓は skip する（現寸を導出できない）。
/// アンカー欠落・非正寸・`WindowHandle` 未付与など残りの縮退は [`resize_window_to`]
/// 側が warn＋no-op で吸収する（Req3.3/3.4・二重に弾かない）。
///
/// # Concurrency
///
/// UI スレッド・World 排他（`&mut World`）。他 actor は触れない（design State Management）。
#[allow(dead_code)] // schedule 登録（結線）は main.rs／runtime 側の領分（本 task は定義のみ）
pub fn anchor_changed_system(
    world: &mut World,
    mut state: Local<Option<SystemState<Query<'static, 'static, Entity, Changed<Anchored>>>>>,
) {
    // Changed<Anchored> 検知の永続シーム: SystemState を跨 run で使い回し last_run を保つ。
    let state = state.get_or_insert_with(|| SystemState::new(world));
    // 変更窓を collect して borrow を即解放してから &mut World ループへ（先例
    // create_windows の collect→release→&mut World ループを Changed 対応にしたもの）。
    let changed: Vec<Entity> = state.get(world).iter().collect();
    for entity in changed {
        // 現在の表示寸法（新寸ではない）を読む。size 不在／未生成は skip（panic しない）。
        let Some(size) = world.get::<WindowPos>(entity).and_then(|wp| wp.size) else {
            continue;
        };
        // 現寸で resize_window_to → 現在アンカー（変化後）で project_anchor を再適用。
        // 経路タグは AnchorChange（Req 1.2 の「変化を引き起こした経路」・task 1.4）。
        resize_window_to(
            world,
            entity,
            SizePx {
                w: size.width,
                h: size.height,
            },
            PlacementRoute::AnchorChange,
        );
    }
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
///
/// # `route` 引数（Req 1.2／2.4・design D11「enum 引数配管」・task 1.4）
///
/// 位置を書いた**経路**（＝要件 2.4 の「最終位置を書き込んだ主体」の名指し語彙）を
/// 呼出側から受け取り、**書込成功時に**窓移動レコード 1 行を専用 target
/// （[`diag::DIAG_TARGET`]）へ出す。ラッパ関数を増やさず引数で配管するのは D11 の裁定で、
/// route は後続タスクで遷移ガードの発火条件・warn 水準分岐の第一級入力にもなる。
///
/// `None` は「本 target が観測を**所有しない**書込」であり、該当するのは
/// **ドラッグ経路のキャラ窓書込のみ**である（[`on_char_drag`]／[`on_char_drag_end`]）——
/// design「placement::diag > Risks」の裁定どおり、ドラッグ中の観測は wintf の `[drag]`
/// target が所有し本 target を通らない（Req 2.4 の結論語彙も「[`PlacementRoute`] 名
/// ＋ wintf `[drag]`／提案位置書込の 2 語」と規定されている）。ドラッグに随伴する
/// バルーン側の書込は [`PlacementRoute::BalloonFollow`] を持つ（Req 2.5 の判別材料）。
///
/// `\![move]` cue（[`move_window_to`] の対象窓）は **`None` ではない**——D13 で
/// [`PlacementRoute::MoveCue`] を新設し、スクリプト明示移動を名指しできるようにした
/// （無記録のままだと Q3「ドラッグ以外の経路での消失」の観測に穴が残る）。
///
/// `None` でも**挙動は完全に同一**であり、変わるのはレコードを出すか否かだけである。
fn enqueue_window_set_pos(
    world: &mut World,
    window: Entity,
    x: i32,
    y: i32,
    size: Option<SizePx>,
    route: Option<PlacementRoute>,
) -> bool {
    // 「entity 不在＝破棄済み」と「実在するが `WindowHandle` 未付与＝窓生成前」を混ぜない
    // （task 7.3・Req 6.2/6.3・task 3.2 が消費側 4 入口へ敷いたのと同じ区別）。前者は終了
    // 処理の**正常終了系**——終了処理でゴースト窓が破棄された後も随伴書込（`follow_balloon`）
    // は走り得るため、`warn!` のままにすると終了時ログが良性ノイズで埋まって本物の異常が
    // 読めなくなる（6.2 → 7.3 の申し送り）。後者は結線の異常ゆえ `warn!` を保つ。
    if world.get_entity(window).is_err() {
        debug!(
            entity = ?window,
            x, y,
            "{DESPAWNED_SKIP_TAG} 移動対象窓は既に破棄済み（despawn）→ 窓移動を正常系として打ち切り"
        );
        return false;
    }
    let Some(handle) = world.get::<WindowHandle>(window).copied() else {
        warn!(
            entity = ?window,
            x, y,
            "移動対象窓の WindowHandle 未付与（窓生成前）のため移動しない"
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

    // 窓移動レコード（Req 1.2）: **書込成功時のみ** 1 レコードを専用 target へ出す。
    // 経路語彙を持たない書込（ドラッグ）は route=None ＝無記録（doc 参照）。
    // `\![move]` は D13 で `MoveCue` を得たので記録される（:753 で `Some` を渡す）。
    if let Some(route) = route {
        log_window_move(world, window, route, x, y, size);
    }

    true
}

/// 窓移動レコード（Req 1.2）を World から転写して組み、専用 target へ出す
/// （[`enqueue_window_set_pos`] の書込成功時専用・design「placement::diag > Invariants」）。
///
/// `diag` は placement の最下流で `World`・wintf 型に依存しない契約ゆえ、
/// **窓種別（[`CharWindowMarker`]／[`BalloonWindowMarker`]）・scope・DPI（`DPI` component）の
/// 読み出しは呼出側である本モジュールの仕事**である。`entity` は wintf 側ログ
/// （`entity = ?e`・scope を持たない）との**結合キー**として必ず入れる——Req 1.9 の
/// scope 別 DPI 受理計数は、この結合による 2 段 grep で機械化される。
///
/// 種別 marker が無い窓（placement が生成したゴースト窓ではない）は、種別・scope を
/// 発明せずレコードを出さない。ただし「出さなかった事実」自体は同 target へ残す
/// （silent skip を作らない・log-first）。
fn log_window_move(
    world: &World,
    window: Entity,
    route: PlacementRoute,
    x: i32,
    y: i32,
    size: Option<SizePx>,
) {
    let identity = world
        .get::<CharWindowMarker>(window)
        .map(|m| (WindowKind::Char, m.scope))
        .or_else(|| {
            world
                .get::<BalloonWindowMarker>(window)
                .map(|m| (WindowKind::Balloon, m.scope))
        });
    let Some((kind, scope)) = identity else {
        debug!(
            target: diag::DIAG_TARGET,
            entity = ?window,
            route = route.as_str(),
            "窓種別 marker 不在（placement 生成のゴースト窓ではない）ゆえ窓移動レコードを出さない"
        );
        return;
    };

    diag::log_window_move(&WindowMoveRecord {
        route,
        entity: window,
        kind,
        scope,
        x,
        y,
        // 寸を伴う経路（`Some`）は実寸を必ず詰める。`None` は移動専用（`SWP_NOSIZE`）の
        // 書込に限り、レコード側は番兵 `-` でフィールドを落とさない（grep 語の不変）。
        size: size.map(|s| (s.w, s.h)),
        dpi: world.get::<DPI>(window).map(|d| d.dpi_x as u32),
    });
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
///
/// # 判別付き版への委譲（task 2.2・D6）
///
/// 本関数は [`work_area_for_window_with_origin`] の**戻り値から判別を落とすだけ**の
/// 薄いラッパである（契約不変＝既存呼出元の挙動は 1 bit も変わらない。等価性は
/// `work_area_for_window_delegates_to_with_origin` が檻で固定する）。
pub fn work_area_for_window(snapshot: &MonitorSnapshot, window: RectPx) -> Option<RectPx> {
    work_area_for_window_with_origin(snapshot, window).map(|(wa, _)| wa)
}

// =============================================================================
// 可視性の遷移ガード＋work area 解決の判別（task 2.2・D6/S3′・Req 3.1/3.2/5.3）
// =============================================================================

/// [`work_area_for_window_with_origin`] が work area を決めた**規則**（D6・Req 3.2）。
///
/// 最近傍フォールバックは「どのモニタにも属さない」＝モニタ構成情報と実画面の
/// 食い違い／窓が可視領域外という**異常の兆候**でありながら、[`work_area_for_window`]
/// の戻り値だけでは正常な帰属と区別できない（S3 の後半＝「最近傍フォールバックが
/// 異常を無観測で吸収する」）。本 enum はその区別を呼出側へ返すためだけに在る。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkAreaResolution {
    /// 窓中心がその work area に帰属した（half-open 判定・正常）。
    Contains,
    /// どのモニタにも属さず、最近傍フォールバックで選ばれた（Req 3.2 の観測点）。
    NearestFallback,
}

/// [`work_area_for_window`] の判別付き版（D6・Req 3.2）。
///
/// 解決規則そのものは [`work_area_for_window`] の doc が定める決定論規則と**完全に
/// 同一**（中心の半開帰属・昇順 index 先勝ち・最近傍は clamp 点自乗距離最小・空
/// snapshot は `None`）。違いは「どちらの規則で決まったか」を
/// [`WorkAreaResolution`] として併せて返す点だけである。
///
/// 消費側（task 6.1 で配線）は `NearestFallback` を非ドラッグ経路でのみ `warn!` へ
/// 昇格させる（ドラッグ経路は毎イベント発火ゆえ従来 `debug!` 水準を維持・Req 3.3）。
pub fn work_area_for_window_with_origin(
    snapshot: &MonitorSnapshot,
    window: RectPx,
) -> Option<(RectPx, WorkAreaResolution)> {
    let cx = (window.left as i64 + window.right as i64) / 2;
    let cy = (window.top as i64 + window.bottom as i64) / 2;

    // 帰属（half-open）・昇順 index 先勝ち
    if let Some(wa) = snapshot.work_areas.iter().find(|wa| {
        (wa.left as i64) <= cx
            && cx < (wa.right as i64)
            && (wa.top as i64) <= cy
            && cy < (wa.bottom as i64)
    }) {
        return Some((*wa, WorkAreaResolution::Contains));
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
        .map(|wa| (wa, WorkAreaResolution::NearestFallback))
}

/// [`guard_visibility`] の判定（D6・S3/S3′）。
///
/// いずれの腕も**最終位置そのもの**を持つ（呼出側が「clamp されたか」を見て warn
/// 水準を分岐しつつ、位置は腕を問わず [`VisibilityVerdict::position`] で取れる）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibilityVerdict {
    /// 提案位置をそのまま採る（交差維持・またはユーザーの明示留置の尊重）。
    Keep(PointPx),
    /// 交差→非交差の遷移を検出し、X のみ `clamp_wa` の水平範囲へ引き戻した。
    ClampX(PointPx),
}

impl VisibilityVerdict {
    /// 判定によらず最終位置を取り出す。
    pub fn position(self) -> PointPx {
        match self {
            VisibilityVerdict::Keep(p) | VisibilityVerdict::ClampX(p) => p,
        }
    }
}

/// 可視性の**遷移**ガード（純関数・非ドラッグ経路専用・D5/D6・Req 3.1/3.2/3.4）。
///
/// S3／S3′ が登記する欠陥は「キャラ窓・バルーン窓の水平方向に可視性の不変条件が
/// 存在しない」ことである。本関数はその不変条件を**遷移**として定義する——静的な
/// 「常に可視領域内」ではない。ユーザーが自ら画面外へ運んだ窓を引き戻すのは
/// 明示操作の否定であり本 spec の Out of scope だからである。
///
/// # 判定規則（4 分岐・すべて交差の有無で表現＝絶対 px の閾値を持たない・Req 5.6）
///
/// | 提案矩形が work area 集合と交差 | 旧矩形 | 判定 |
/// | --- | --- | --- |
/// | する | 問わない | [`Keep`](VisibilityVerdict::Keep)（素通し） |
/// | しない | 交差していた | [`ClampX`](VisibilityVerdict::ClampX)（可視→不可視の遷移を阻止） |
/// | しない | 交差していなかった | [`Keep`](VisibilityVerdict::Keep)（ユーザーの明示留置を尊重） |
/// | しない | `None`（不明） | [`ClampX`](VisibilityVerdict::ClampX)（安全側） |
///
/// # 引数
///
/// - `old_rect`: 書込**前**の窓矩形（現 `WindowPos` の position＋size）。窓生成直後等で
///   読めない場合は `None`＝安全側 clamp。
/// - `proposed_pos`／`size`: 射影 T が出した提案位置と、その位置に置く窓の寸。
/// - `clamp_wa`: clamp 先の work area。**射影が Y に用いたのと同じ矩形**を呼出側が
///   貫通させる（task 6.1 が [`work_area_for_window_with_origin`] の戻り値を渡す）。
///   ガード内で引き直さないのは、Y と X が別モニタを基準にする不整合を作らないため。
/// - `snapshot`: 交差判定に用いる全 work area 集合。
///
/// # 事後条件・不変条件
///
/// - **Y は一切変更しない**（Y は射影 T の所有・D6）。`Keep`／`ClampX` のいずれでも
///   `verdict.position().y == proposed_pos.y`。
/// - `ClampX` の X は `clamp_wa.left ..= clamp_wa.right − size.w` の範囲へ入る
///   （`saturating` 演算・逆転区間でも panic しない `min`/`max` 流儀）。窓幅が
///   work area より広い場合は `left` が勝つ＝左端合わせで**必ず水平に重なる**。
/// - 正寸かつ `proposed_pos` の Y 範囲が `clamp_wa` と重なるとき（＝射影 T が Y を
///   決めた正常系）、`ClampX` 後の矩形は `clamp_wa` と交差する＝完全不可視が消える。
/// - World 非依存・副作用なし・panic しない。ログは出さない——`ClampX`／
///   `NearestFallback` の `warn!` は route（経路タグ）で水準が変わる呼出側の責務
///   （Req 3.3・ドラッグ経路 spam 回避の水準分岐は route を持つ層でしか書けない）。
///
/// # 縮退
///
/// 空 snapshot では何も交差しないため、`old_rect` が `Some`（＝同じく非交差）なら
/// `Keep`＝現状維持。架空の可視領域を発明しない（resolver／`work_area_for_window`
/// と同方針）。
pub fn guard_visibility(
    old_rect: Option<RectPx>,
    proposed_pos: PointPx,
    size: SizePx,
    clamp_wa: RectPx,
    snapshot: &MonitorSnapshot,
) -> VisibilityVerdict {
    // 1. 提案矩形がどれかの work area と交差していれば可視性は失われていない。
    if intersects_any_work_area(snapshot, rect_at(proposed_pos, size)) {
        return VisibilityVerdict::Keep(proposed_pos);
    }

    // 2. 旧矩形も非交差だった＝ユーザーが自ら画面外へ留置した窓（Out of scope）。
    //    旧矩形不明（`None`）はここに含めない＝安全側で clamp する。
    let was_already_off_screen = match old_rect {
        Some(old) => !intersects_any_work_area(snapshot, old),
        None => false,
    };
    if was_already_off_screen {
        return VisibilityVerdict::Keep(proposed_pos);
    }

    // 3. 交差→非交差の遷移（または旧矩形不明）＝X のみ引き戻す。Y は射影の所有。
    VisibilityVerdict::ClampX(PointPx {
        x: clamp_x_into(proposed_pos.x, size.w, clamp_wa),
        y: proposed_pos.y,
    })
}

/// 位置＋寸から窓矩形を作る（`right`/`bottom` は排他側・`saturating` で溢れない）。
fn rect_at(pos: PointPx, size: SizePx) -> RectPx {
    RectPx {
        left: pos.x,
        top: pos.y,
        right: pos.x.saturating_add(size.w),
        bottom: pos.y.saturating_add(size.h),
    }
}

/// 2 矩形が**面積を持って**重なるか（半開区間・接触のみは交差としない）。
fn rects_intersect(a: RectPx, b: RectPx) -> bool {
    (a.left as i64) < (b.right as i64)
        && (b.left as i64) < (a.right as i64)
        && (a.top as i64) < (b.bottom as i64)
        && (b.top as i64) < (a.bottom as i64)
}

/// いずれかの work area と交差するか（空 snapshot は常に `false`）。
fn intersects_any_work_area(snapshot: &MonitorSnapshot, window: RectPx) -> bool {
    snapshot
        .work_areas
        .iter()
        .any(|wa| rects_intersect(window, *wa))
}

/// X を `wa.left ..= wa.right − w` へ引き戻す（`i32::clamp` は逆転区間で panic する
/// ため min/max 流儀・`work_area_for_window` の最近傍 clamp と同型の防波堤）。
fn clamp_x_into(x: i32, w: i32, wa: RectPx) -> i32 {
    x.min(wa.right.saturating_sub(w)).max(wa.left)
}

// =============================================================================
// 遷移ガードの配線（task 6.1・S3 是正・D5/D6/D13・Req 3.1/3.2/3.3）
// =============================================================================

/// 遷移ガードが X を引き戻したことを表す判定語（`diagnosis-procedure.md` §3.3）。
const VISIBILITY_CLAMP_TAG: &str = "[visibility-guard] ClampX";

/// work area 解決が最近傍フォールバックへ落ちたことを表す判定語（同上・Req 3.2）。
const VISIBILITY_NEAREST_FALLBACK_TAG: &str = "[visibility-guard] NearestFallback";

/// work area が解決できずガードを評価できなかったことを表す判定語（同上・Req 3.3）。
const VISIBILITY_UNRESOLVED_TAG: &str = "[visibility-guard] WorkAreaUnresolved";

/// この `route` の書込が**非ドラッグの自動配置**か（＝遷移ガードの発火対象・D13 帰結⑴）。
///
/// # なぜ route が第一級の入力なのか
///
/// S3 が防ぐのは「**ユーザーが意図せず**窓を見失う」経路だけである（requirements.md
/// Boundary Context「ユーザーが自らドラッグして運んだ結果の不可視化」は Out of scope）。
/// 明示操作（ドラッグ・`\![move]`）とスクリプト／永続化が決めた位置を引き戻すのは
/// その否定であり、**同じ矩形・同じ幾何でも判定が反転する**。ゆえに発火条件は幾何では
/// 表現できず、書込の由来＝route を見るしかない。
///
/// # 網羅 `match` で書く理由
///
/// 既定腕（`_ => false` 等）を置くと、[`PlacementRoute`] へ語彙が増えたとき新経路が
/// 黙って片側へ倒れる。網羅 `match` ならコンパイラが判断を要求する（D14 帰結⑵と同じ流儀）。
///
/// # 適用外の内訳
///
/// - [`SpawnInitial`](PlacementRoute::SpawnInitial)／[`Restore`](PlacementRoute::Restore):
///   復元時の可視化保証は `areka-P0-position-persist` の所有（design Boundary）。
/// - [`MoveCue`](PlacementRoute::MoveCue): `\![move]` はスクリプトの明示操作（D13 帰結⑵）。
/// - [`KeepPositionResize`](PlacementRoute::KeepPositionResize)／
///   [`BalloonFollow`](PlacementRoute::BalloonFollow): バルーン窓側の書込。
///   **本述語をそのままバルーン適用（task 6.2）の発火条件に流用しないこと**——
///   バルーンの適用可否は「随伴の**引き金**がドラッグだったか配置系だったか」で決まり、
///   `follow_balloon` の呼出元が持つ情報である（本述語の入力は書込自身の route）。
///   task 6.2 は [`BalloonFollowTrigger`] を新設して**引き金**を配管し、その
///   [`Placement`](BalloonFollowTrigger::Placement) 腕が**引き金の route** に対して
///   本述語を引く形にした（本述語へ `BalloonFollow` を渡す形にはしていない）。
fn route_applies_visibility_guard(route: PlacementRoute) -> bool {
    match route {
        // 非ドラッグの自動配置（S3 の保護対象・D13 帰結⑴）
        PlacementRoute::AnchorChange
        | PlacementRoute::Resnap
        | PlacementRoute::DpiReproject
        | PlacementRoute::ReportedSizeReconcile => true,
        // 明示操作・別 spec 所有・バルーン窓側（上記 doc の内訳）
        PlacementRoute::SpawnInitial
        | PlacementRoute::Restore
        | PlacementRoute::KeepPositionResize
        | PlacementRoute::BalloonFollow
        | PlacementRoute::MoveCue => false,
    }
}

/// 射影 T の**下流・外側**で可視性の遷移ガードを適用する（D5: `project_anchor` の
/// 内部は変更しない）。
///
/// # 引数
///
/// - `route`: 発火条件（[`route_applies_visibility_guard`]）。適用外なら `proposed` を素通す。
/// - `snapshot`／`raw`: 射影 T が work area を選んだのと**同一の入力**。
/// - `old_rect`: 書込**前**の窓矩形。`None`＝不明で安全側 clamp（[`guard_visibility`]）。
///
/// # 2 つの解決を引き分ける（同じ純関数を 2 回引くのは意図的）
///
/// - **clamp 先**（`clamp_wa`）は射影 T が Y に用いたのと同じ矩形（`raw` × `size`）から
///   引く。ここを別の矩形で引き直すと Y と X が別モニタを基準にして
///   [`guard_visibility`] の事後条件（clamp 後に `clamp_wa` と交差する）が崩れる
///   （design Risks・[`guard_visibility`] doc の `clamp_wa` 項）。
/// - **食い違いの観測**（Req 3.2）は**射影 T が決めた位置**（`proposed` × `size`）の帰属で
///   判定する。要件が言う「窓位置を**決めた**とき」の位置がこれであり、射影の入力 `raw`
///   は下端吸着より前の一時状態にすぎない——`raw` で判定すると「射影が正しく接地させて
///   可視域へ収めた窓」まで食い違いとして報告する偽陽性になる（下端吸着では
///   `raw` の中心が work area 下端より下にあることは珍しくない）。
///
/// # ログ（Req 3.1/3.2/3.3・[[2.2 → 6.1 の申し送り]]）
///
/// [`guard_visibility`] は**意図的に無ログ**の純関数で、水準の分岐（非ドラッグ経路は
/// `warn!`／ドラッグ経路は従来 `debug!` のまま）は route を持つ本層でしか書けない。
/// ゆえに観測は本関数の責務である——ここで出さなければ Req 3.1/3.2 の観測が丸ごと欠落する。
///
/// # 縮退（Req 3.3）
///
/// `MonitorSnapshot` 不在／空 snapshot では work area が 1 つも無く、clamp 先を決められない。
/// このとき**位置には一切手を入れず** `warn!` を残す（架空の可視領域を発明しない＝
/// `work_area_for_window` と同方針）。この場合の `proposed` は射影 T 自身が同じ入力欠落で
/// identity へ縮退した値＝現在位置であり、「現状維持」がそのまま成立する。
fn apply_visibility_guard(
    entity: Entity,
    route: PlacementRoute,
    snapshot: Option<&MonitorSnapshot>,
    old_rect: Option<RectPx>,
    raw: PointPx,
    proposed: PointPx,
    size: SizePx,
) -> PointPx {
    if !route_applies_visibility_guard(route) {
        return proposed;
    }
    evaluate_visibility_guard(entity, route, snapshot, old_rect, raw, proposed, size)
}

/// 発火可否の判定が**済んだ後**の本体（評価＋観測）。
///
/// キャラ窓（[`apply_visibility_guard`]＝書込自身の route で発火判定）とバルーン窓
/// （[`guard_balloon_position`]＝随伴の**引き金**で発火判定・task 6.2）が共有する。
/// 発火判定だけを外へ出したのは、両者で判定の**入力が違う**（書込の route ⇔ 引き金の
/// route）一方、評価規則・clamp 先の引き方・3 語の観測は**完全に同一**だからである
/// （design「バルーン適用（S3′ 是正）」＝新規機構ゼロ）。
///
/// `route` は**ログに載る経路名**であり、発火判定には用いない——バルーン随伴の書込は
/// [`PlacementRoute::BalloonFollow`] として記録される（[`enqueue_window_set_pos`] が出す
/// `[diag.window_move]` レコードと同じ語）ので、警告行とレコード行が同じ route 名で
/// 突合できる。
fn evaluate_visibility_guard(
    entity: Entity,
    route: PlacementRoute,
    snapshot: Option<&MonitorSnapshot>,
    old_rect: Option<RectPx>,
    raw: PointPx,
    proposed: PointPx,
    size: SizePx,
) -> PointPx {
    let Some(snapshot) = snapshot else {
        warn!(
            entity = ?entity,
            ?route,
            "{VISIBILITY_UNRESOLVED_TAG} MonitorSnapshot 未挿入のため可視性を判定できない → 位置は現状維持"
        );
        return proposed;
    };
    // 射影が Y に用いたのと同じ矩形（raw × 新寸）から引き直す＝clamp 先の貫通。
    let Some((clamp_wa, _)) = work_area_for_window_with_origin(snapshot, rect_at(raw, size)) else {
        warn!(
            entity = ?entity,
            ?route,
            "{VISIBILITY_UNRESOLVED_TAG} モニタ 0 台（空 snapshot）のため可視性を判定できない → 位置は現状維持"
        );
        return proposed;
    };

    // 最近傍フォールバック＝**決めた位置**の窓中心がどのモニタにも属さない＝モニタ構成
    // 情報と実画面の食い違い、あるいは窓が既に可視領域外という異常の兆候（Req 3.2・
    // S3 後段「最近傍フォールバックが異常を無観測で吸収する」）。ドラッグ経路は毎イベント
    // 発火ゆえ従来 `debug!` のまま（本関数を通らない＝水準分岐が route で成立する）。
    let decided = work_area_for_window_with_origin(snapshot, rect_at(proposed, size));
    if matches!(decided, Some((_, WorkAreaResolution::NearestFallback))) {
        warn!(
            entity = ?entity,
            ?route,
            ?proposed,
            ?size,
            ?clamp_wa,
            "{VISIBILITY_NEAREST_FALLBACK_TAG} 決めた位置の窓中心がどの work area にも属さず最近傍で解決した（モニタ構成情報と実画面の食い違いの兆候）"
        );
    }

    // 判定は「腕を見て warn 水準を分岐する」ためだけに使い、位置は腕を問わず
    // [`VisibilityVerdict::position`] で取る（同 enum の doc が定める消費の形）。
    let verdict = guard_visibility(old_rect, proposed, size, clamp_wa, snapshot);
    if let VisibilityVerdict::ClampX(clamped) = verdict {
        warn!(
            entity = ?entity,
            ?route,
            ?old_rect,
            ?proposed,
            clamped = ?clamped,
            ?size,
            ?clamp_wa,
            "{VISIBILITY_CLAMP_TAG} 全 work area 非交差への遷移を検出し X を引き戻した（Y は射影の所有ゆえ不変）"
        );
    }
    verdict.position()
}

// =============================================================================
// resize_window_keep_position（areka-P0-emo-dpi-scaling task 2.2・R3.1/R4.2）
// =============================================================================

/// 現在位置を維持して窓寸のみ更新する（balloon 窓の DPI 追従用・R3.1/R4.2）。
///
/// 私有単一ライター経路 [`enqueue_window_set_pos`]`(.., Some(new_size))` の**薄い公開
/// ラッパ**——DPI 変化フェーズ（design D8・Flow 2）が balloon 窓の寸を k 追従させる
/// ための唯一の正規手段であり、単一ライター規律（`SetWindowPosCommand` 発行＋
/// `WindowPos` bypass ミラー＋`Arrangement.offset` 同期）を迂回する第二の書込経路を
/// 新設しない（Req1.5 の継承）。
///
/// [`resize_window_to`] との違いは**位置の決め方**だけである。あちらは
/// [`Anchored`] を読んで [`project_anchor`] で位置を再導出する（キャラ窓＝接地点が
/// アンカー辺に釘付けされる）が、こちらは `WindowPos.position` の**現在値をそのまま
/// 据え置き**、寸法だけを差し替える。balloon 窓の位置は [`follow_balloon`] が
/// キャラ窓＋`BalloonFollow.offset` から決める従属量であり、寸法変更の場面で
/// 独自にアンカー射影をかけると同フレーム内で二重に位置が動くため。
///
/// # 縮退・失敗経路（log-first・silent failure を作らない）
///
/// - **対象 entity 不在（despawn 済み）**: 正常終了系ゆえ `debug!`
///   （[`diag::DESPAWNED_SKIP_TAG`]）＋`false`（要件 6.2/6.3・[`resize_window_to`] と同一流儀）。
/// - 非正寸（w≤0 or h≤0）: 何も書かず `warn!`＋`false`（[`resize_window_to`] の
///   非正寸縮退と同一流儀）。
/// - `WindowPos` 不在（窓生成前の異常系）: 現在位置を読めないため `warn!`＋`false`。
/// - `WindowPos.position` 不在（窓生成前）: 同上 `warn!`＋`false`（panic しない）。
/// - べき等 skip（R4.2）: 現 `WindowPos.size` が新寸と同一なら**書込を一切行わず**
///   `false`（k 不変・同寸で窓が振動しないための檻・正常系ゆえ `debug!`）。
/// - `WindowHandle` 未付与/対象不在: [`enqueue_window_set_pos`] が `warn!`＋`false`
///   （判定を二重化せず委譲する）。
///
/// 消費者: `emo2_boot` の DPI 追従フェーズ（`run_dpi_phase`／`reconcile_reported_sizes`・結線済み）。
///
/// # 経路タグ（Req 1.2・task 1.4）
///
/// 本関数の書込は定義上つねに [`PlacementRoute::KeepPositionResize`]（経路語彙が関数名
/// そのもの）ゆえ、[`resize_window_to`] と違い route を引数で受けない——受けても
/// 呼出側が渡せる値は 1 つしか無く、取り違えの余地だけを増やすため。
#[allow(dead_code)] // examples が #[path] include するため、本体未使用ビルドでも必要
pub fn resize_window_keep_position(world: &mut World, window: Entity, new_size: SizePx) -> bool {
    // 0. 存在確認（要件 6.2/6.3・design D8 消費側）: 破棄済みバルーン窓は正常終了系として
    //    debug で打ち切る（下の `WindowPos` 未付与 warn は**実在する**窓の異常に取っておく）。
    if world.get_entity(window).is_err() {
        debug!(
            entity = ?window,
            "{DESPAWNED_SKIP_TAG} 対象 entity は既に破棄済み（despawn）→ 位置据置きリサイズを正常系として打ち切り"
        );
        return false;
    }

    // 1. 非正寸ガード（R3.1）: 窓寸として成立しない値は書かない。
    if new_size.w <= 0 || new_size.h <= 0 {
        warn!(
            entity = ?window,
            ?new_size,
            "新しい窓寸法が非正のためリサイズせず現状保持"
        );
        return false;
    }

    // 2. 現在位置（維持する値）と現寸（べき等判定の材料）を読む。
    //    WindowPos／position 不在は窓生成前の異常系＝log-first で no-op（panic しない）。
    let (pos, current_size) = {
        let Some(wp) = world.get::<WindowPos>(window) else {
            warn!(
                entity = ?window,
                "WindowPos 未付与（窓生成前）のため現在位置を読めずリサイズしない"
            );
            return false;
        };
        let Some(pos) = wp.position else {
            warn!(
                entity = ?window,
                "WindowPos.position 不在（窓生成前）のため現在位置を読めずリサイズしない"
            );
            return false;
        };
        (pos, wp.size)
    };

    // 3. べき等 skip（R4.2・D8）: 同寸なら書込ゼロ。位置は据え置きゆえ寸だけを見れば
    //    十分（resize_window_to の (position, size) 一致判定の位置維持版）。
    //    現寸不明（None＝窓生成直後）は判定が成立しないので書込へ進む。
    if current_size == Some(SizeI::new(new_size.w, new_size.h)) {
        debug!(
            entity = ?window,
            ?new_size,
            "窓寸が現在値と同一のため書込をスキップ（べき等・振動防止）"
        );
        return false;
    }

    // 4. 一度書き: 現在位置＋新寸を単一ライター経路で 1 コマンド発行する。
    //    WindowHandle 未付与/対象不在は enqueue が warn!＋false（二重に弾かない）。
    enqueue_window_set_pos(
        world,
        window,
        pos.x,
        pos.y,
        Some(new_size),
        Some(PlacementRoute::KeepPositionResize),
    )
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
    // work_area_for_window_with_origin ／ guard_visibility
    // （task 2.2・D6/S3・S3′・Req 3.1/3.2/5.1/5.3/5.6）
    //
    // 共通規約: 判定は絶対 px の固定値ではなく**交差・不変条件**で書く（Req 5.6）。
    // 座標は 96/120/192 の各水準へスケールした合成レイアウト上で構築し、96 の
    // 自己整合（k=1 で恒等写像に退化して欠陥を隠す性質・Req 5.1）に依存しない。
    // -------------------------------------------------------------------------

    use super::{
        VisibilityVerdict, WorkAreaResolution, guard_visibility, work_area_for_window_with_origin,
    };

    /// DPI 水準（Req 5.1: 96 のほかに 120・192 を必ず含む）。
    const DPIS: [i32; 3] = [96, 120, 192];

    /// 論理基準値 → 各 DPI の物理 px（整数演算のみ・厳密整除を強制。
    /// `resolver.rs` の `px()` が donor・Req 5.6）。
    fn px(logical: i32, dpi: i32) -> i32 {
        assert_eq!(
            (logical * dpi) % 96,
            0,
            "テスト入力は厳密整除になる論理値（4 の倍数）で構築する"
        );
        logical * dpi / 96
    }

    /// 混在 DPI マルチモニタの合成レイアウト（Req 5.1/5.3）。
    ///
    /// - index 0: 96 水準の左モニタ。**負座標**（`-1920..0`）・上端 40px の
    ///   非対称 work area（`top = -40`）
    /// - index 1: `dpi` 水準の右モニタ。左端に 64 論理 px のタスクバー＝
    ///   **非対称 work area**（`left = px(64)`）。192 では右端 3840＝**3200 超座標**
    ///
    /// 2 面のあいだ（`0 ..= px(64)`）はどの work area にも属さない帯であり、
    /// 最近傍フォールバックの発火面として使う。
    fn mixed_layout(dpi: i32) -> MonitorSnapshot {
        MonitorSnapshot {
            work_areas: vec![left_wa(), right_wa(dpi)],
        }
    }

    /// 左モニタ（96 水準・負座標）の work area。
    fn left_wa() -> RectPx {
        rect(-1920, -40, 0, 1000)
    }

    /// 右モニタ（`dpi` 水準・非対称）の work area。192 で right=3840（>3200）。
    fn right_wa(dpi: i32) -> RectPx {
        rect(px(64, dpi), 0, px(1920, dpi), px(1040, dpi))
    }

    /// キャラ窓の寸（論理 300x400）。
    fn char_size(dpi: i32) -> SizePx {
        SizePx {
            w: px(300, dpi),
            h: px(400, dpi),
        }
    }

    /// バルーン窓の寸（論理 500x300）。
    fn balloon_size(dpi: i32) -> SizePx {
        SizePx {
            w: px(500, dpi),
            h: px(300, dpi),
        }
    }

    fn point(x: i32, y: i32) -> PointPx {
        PointPx { x, y }
    }

    /// 位置＋寸 → 窓矩形（テスト側の独立実装＝実装の `rect_at` を再利用しない）。
    fn win(pos: PointPx, size: SizePx) -> RectPx {
        rect(pos.x, pos.y, pos.x + size.w, pos.y + size.h)
    }

    /// 面積を持つ重なりの独立実装（実装の `rects_intersect` とは別式で書く）。
    fn overlaps(a: RectPx, b: RectPx) -> bool {
        a.left.max(b.left) < a.right.min(b.right) && a.top.max(b.top) < a.bottom.min(b.bottom)
    }

    /// キャラ窓の Bottom 接地位置（射影 T が出す Y＝`wa.bottom − h`）。
    fn grounded_y(wa: RectPx, size: SizePx) -> i32 {
        wa.bottom - size.h
    }

    // --- work_area_for_window_with_origin -------------------------------------

    /// 中心が帰属するときは `Contains` を返す（左右どちらのモニタでも・全水準）。
    #[test]
    fn with_origin_reports_contains_when_center_belongs() {
        for dpi in DPIS {
            let snapshot = mixed_layout(dpi);
            let size = char_size(dpi);

            // 右モニタの中央付近
            let pos = point(px(800, dpi), grounded_y(right_wa(dpi), size));
            assert_eq!(
                work_area_for_window_with_origin(&snapshot, win(pos, size)),
                Some((right_wa(dpi), WorkAreaResolution::Contains)),
                "dpi={dpi}: 右モニタ内の窓は Contains"
            );

            // 左モニタ（負座標）の中央付近
            let pos = point(-1200, grounded_y(left_wa(), size));
            assert_eq!(
                work_area_for_window_with_origin(&snapshot, win(pos, size)),
                Some((left_wa(), WorkAreaResolution::Contains)),
                "dpi={dpi}: 左モニタ（負座標）内の窓は Contains"
            );
        }
    }

    /// どのモニタにも属さない中心は `NearestFallback` として判別される
    /// （S3 後半＝最近傍フォールバックが異常を無観測で吸収する性質の是正・Req 3.2）。
    #[test]
    fn with_origin_reports_nearest_fallback_when_center_belongs_nowhere() {
        for dpi in DPIS {
            let snapshot = mixed_layout(dpi);
            let size = char_size(dpi);

            // ① 右モニタの右外（192 では 3200 超座標）
            let far_right = point(
                px(1920, dpi) + px(400, dpi),
                grounded_y(right_wa(dpi), size),
            );
            let (wa, origin) = work_area_for_window_with_origin(&snapshot, win(far_right, size))
                .expect("非空 snapshot ゆえ Some");
            assert_eq!(
                origin,
                WorkAreaResolution::NearestFallback,
                "dpi={dpi}: 右外の窓は最近傍フォールバック"
            );
            assert_eq!(wa, right_wa(dpi), "dpi={dpi}: 最近傍は右モニタ");

            // ② 左モニタの左外（負座標側）
            let far_left = point(-4000, 400);
            let (wa, origin) = work_area_for_window_with_origin(&snapshot, win(far_left, size))
                .expect("非空 snapshot ゆえ Some");
            assert_eq!(
                origin,
                WorkAreaResolution::NearestFallback,
                "dpi={dpi}: 左外の窓は最近傍フォールバック"
            );
            assert_eq!(wa, left_wa(), "dpi={dpi}: 最近傍は左モニタ");

            // ③ 2 面のあいだの帯（右モニタのタスクバー上・非対称 work area 由来）
            //    幅 px(60) の窓を帯へ完全に収め、中心を帯の中へ落とす
            let strip_size = SizePx {
                w: px(40, dpi),
                h: px(40, dpi),
            };
            let strip = point(px(12, dpi), px(400, dpi));
            let (_, origin) = work_area_for_window_with_origin(&snapshot, win(strip, strip_size))
                .expect("非空 snapshot ゆえ Some");
            assert_eq!(
                origin,
                WorkAreaResolution::NearestFallback,
                "dpi={dpi}: 非対称 work area の帯（タスクバー上）は帰属なし"
            );
        }
    }

    /// 空 snapshot は判別付き版でも `None`（架空の既定矩形を発明しない）。
    #[test]
    fn with_origin_empty_snapshot_is_none() {
        let snapshot = MonitorSnapshot { work_areas: vec![] };
        assert_eq!(
            work_area_for_window_with_origin(&snapshot, rect(0, 0, 100, 100)),
            None
        );
    }

    /// **委譲の等価性**（task 2.2 完了条件）: 既存 `work_area_for_window` の戻り値は
    /// 判別付き版の第 1 要素と常に一致する＝既存呼出元の挙動が 1 bit も変わらない。
    ///
    /// 帰属・最近傍・境界・重複・空 snapshot の全経路を同一の probe 集合で走らせる。
    #[test]
    fn work_area_for_window_delegates_to_with_origin() {
        for dpi in DPIS {
            let size = char_size(dpi);
            let snapshots = [
                mixed_layout(dpi),
                // 重複（先勝ち）と共有辺（half-open）を含む合成
                MonitorSnapshot {
                    work_areas: vec![
                        rect(0, 0, px(1920, dpi), px(1040, dpi)),
                        rect(px(1920, dpi), 0, px(3840, dpi), px(1040, dpi)),
                        rect(-40, -40, px(2000, dpi), px(1100, dpi)),
                    ],
                },
                MonitorSnapshot { work_areas: vec![] },
            ];
            let probes = [
                point(px(800, dpi), grounded_y(right_wa(dpi), size)),
                point(-1200, 400),
                point(px(1920, dpi) + px(400, dpi), 100),
                point(-4000, 2000),
                point(px(12, dpi), px(400, dpi)),
                // 共有辺ちょうどに中心が来る位置（half-open の分岐点）
                point(px(1920, dpi) - size.w / 2, px(500, dpi)),
            ];
            for snapshot in &snapshots {
                for pos in probes {
                    let window = win(pos, size);
                    assert_eq!(
                        work_area_for_window(snapshot, window),
                        work_area_for_window_with_origin(snapshot, window).map(|(wa, _)| wa),
                        "dpi={dpi}: 委譲の等価性が崩れた（pos={pos:?}）"
                    );
                }
            }
        }
    }

    // --- guard_visibility: キャラ矩形 -----------------------------------------

    /// 提案矩形がいずれかの work area と交差していれば素通し（`Keep`）。
    /// clamp 先 work area の水平範囲外であっても、交差している限り触らない。
    #[test]
    fn guard_keeps_position_while_still_intersecting() {
        for dpi in DPIS {
            let snapshot = mixed_layout(dpi);
            let size = char_size(dpi);
            let wa = right_wa(dpi);
            let old = win(point(px(800, dpi), grounded_y(wa, size)), size);

            // 右モニタ内の別位置（交差維持）
            let proposed = point(px(1200, dpi), grounded_y(wa, size));
            assert_eq!(
                guard_visibility(Some(old), proposed, size, wa, &snapshot),
                VisibilityVerdict::Keep(proposed),
                "dpi={dpi}: 交差維持は素通し"
            );

            // 右端から半分はみ出した位置（部分可視＝交差あり）でも素通し
            let half_out = point(wa.right - size.w / 2, grounded_y(wa, size));
            assert!(overlaps(win(half_out, size), wa), "前提: 部分可視である");
            assert_eq!(
                guard_visibility(Some(old), half_out, size, wa, &snapshot),
                VisibilityVerdict::Keep(half_out),
                "dpi={dpi}: 部分可視は clamp しない（美観政策は本 spec 非所有）"
            );
        }
    }

    /// 交差→非交差の**遷移**は X のみ clamp（Y は射影の所有＝不変）。
    /// clamp 後は clamp 先 work area と交差する＝完全不可視が消える。
    #[test]
    fn guard_clamps_x_on_transition_to_invisible() {
        for dpi in DPIS {
            let snapshot = mixed_layout(dpi);
            let size = char_size(dpi);
            let wa = right_wa(dpi);
            let y = grounded_y(wa, size);
            let old = win(point(px(800, dpi), y), size);
            assert!(overlaps(old, wa), "前提: 旧矩形は可視だった");

            // ① 右外へ吹き飛んだ提案（192 では 4000 超＝3200 超座標）
            let proposed = point(wa.right + px(600, dpi), y);
            assert!(
                !overlaps(win(proposed, size), wa) && !overlaps(win(proposed, size), left_wa()),
                "前提: 提案矩形はどの work area とも交差しない"
            );
            let verdict = guard_visibility(Some(old), proposed, size, wa, &snapshot);
            let VisibilityVerdict::ClampX(got) = verdict else {
                panic!("dpi={dpi}: 交差→非交差の遷移は ClampX（got {verdict:?}）");
            };
            assert_eq!(got.y, proposed.y, "dpi={dpi}: Y は一切変更しない");
            assert!(
                got.x >= wa.left && got.x <= wa.right - size.w,
                "dpi={dpi}: X は clamp_wa の水平範囲内（got.x={}）",
                got.x
            );
            assert!(
                overlaps(win(got, size), wa),
                "dpi={dpi}: clamp 後は clamp 先 work area と交差する"
            );

            // ② 左外（負座標側）へ吹き飛んだ提案でも同じ規則
            let proposed = point(left_wa().left - px(2000, dpi), y);
            assert!(
                !overlaps(win(proposed, size), wa) && !overlaps(win(proposed, size), left_wa()),
                "前提: 提案矩形はどの work area とも交差しない"
            );
            let verdict = guard_visibility(Some(old), proposed, size, wa, &snapshot);
            let VisibilityVerdict::ClampX(got) = verdict else {
                panic!("dpi={dpi}: 左外への遷移も ClampX（got {verdict:?}）");
            };
            assert_eq!(got.y, proposed.y, "dpi={dpi}: Y は一切変更しない");
            assert_eq!(
                got.x, wa.left,
                "dpi={dpi}: 左方向の逸脱は clamp_wa.left へ引き戻す"
            );
            assert!(overlaps(win(got, size), wa), "dpi={dpi}: 交差が回復する");
        }
    }

    /// 旧矩形も非交差だった（ユーザーが自ら画面外へ留置した窓）＝尊重して素通し。
    /// 本 spec の Out of scope「明示ドラッグでの画面外運搬」を型で守る腕。
    #[test]
    fn guard_respects_window_already_parked_off_screen() {
        for dpi in DPIS {
            let snapshot = mixed_layout(dpi);
            let size = char_size(dpi);
            let wa = right_wa(dpi);
            let y = grounded_y(wa, size);

            let old = win(point(wa.right + px(400, dpi), y), size);
            assert!(
                !overlaps(old, wa) && !overlaps(old, left_wa()),
                "前提: 旧矩形は既に全 work area と非交差（ユーザー留置）"
            );
            let proposed = point(wa.right + px(800, dpi), y);
            assert_eq!(
                guard_visibility(Some(old), proposed, size, wa, &snapshot),
                VisibilityVerdict::Keep(proposed),
                "dpi={dpi}: 既に非交差なら引き戻さない"
            );
        }
    }

    /// 旧矩形が不明（`None`＝窓生成直後等）は安全側で clamp する。
    #[test]
    fn guard_clamps_when_old_rect_is_unknown() {
        for dpi in DPIS {
            let snapshot = mixed_layout(dpi);
            let size = char_size(dpi);
            let wa = right_wa(dpi);
            let y = grounded_y(wa, size);
            let proposed = point(wa.right + px(600, dpi), y);

            let verdict = guard_visibility(None, proposed, size, wa, &snapshot);
            let VisibilityVerdict::ClampX(got) = verdict else {
                panic!("dpi={dpi}: 旧矩形不明は安全側 clamp（got {verdict:?}）");
            };
            assert_eq!(got.y, proposed.y, "dpi={dpi}: Y は一切変更しない");
            assert!(
                overlaps(win(got, size), wa),
                "dpi={dpi}: clamp 後は clamp 先 work area と交差する"
            );

            // 旧矩形不明でも、提案が交差しているなら素通し（clamp は遷移時のみ）
            let inside = point(px(800, dpi), y);
            assert_eq!(
                guard_visibility(None, inside, size, wa, &snapshot),
                VisibilityVerdict::Keep(inside),
                "dpi={dpi}: 交差している提案は old 不明でも素通し"
            );
        }
    }

    /// 窓幅が clamp 先 work area より広い退化ケース: 左端合わせで必ず水平に重なる
    /// （`i32::clamp` の逆転区間 panic を踏まない・非 panic 契約）。
    #[test]
    fn guard_clamp_handles_window_wider_than_work_area() {
        for dpi in DPIS {
            let snapshot = mixed_layout(dpi);
            let wa = right_wa(dpi);
            let size = SizePx {
                w: (wa.right - wa.left) + px(400, dpi),
                h: px(400, dpi),
            };
            let y = grounded_y(wa, size);
            let old = win(point(wa.left, y), size);
            let proposed = point(wa.right + px(1200, dpi), y);

            let verdict = guard_visibility(Some(old), proposed, size, wa, &snapshot);
            let VisibilityVerdict::ClampX(got) = verdict else {
                panic!("dpi={dpi}: 遷移は ClampX（got {verdict:?}）");
            };
            assert_eq!(got.x, wa.left, "dpi={dpi}: 幅超過は left 合わせ");
            assert!(overlaps(win(got, size), wa), "dpi={dpi}: 交差が回復する");
        }
    }

    /// 空 snapshot（縮退）: 何も交差しないため、旧矩形が読めるなら現状維持。
    /// 架空の可視領域を発明しない。
    #[test]
    fn guard_empty_snapshot_keeps_position() {
        for dpi in DPIS {
            let snapshot = MonitorSnapshot { work_areas: vec![] };
            let size = char_size(dpi);
            let wa = right_wa(dpi);
            let proposed = point(px(800, dpi), px(600, dpi));
            let old = win(point(px(700, dpi), px(600, dpi)), size);
            assert_eq!(
                guard_visibility(Some(old), proposed, size, wa, &snapshot),
                VisibilityVerdict::Keep(proposed),
                "dpi={dpi}: 空 snapshot は現状維持"
            );
        }
    }

    // --- guard_visibility: バルーン矩形（S3′・Req 3.4） -----------------------
    //
    // バルーンは**別規則を持たない**——キャラ窓とまったく同一の純関数・同一の
    // 遷移規則へ、バルーン矩形（`char_pos + offset` と バルーン寸）を渡すだけ。

    /// キャラ窓が右端で clamp された合成で、offset 恒等式が出したバルーン提案位置
    /// だけが全 work area と非交差になるケース → バルーン矩形も ClampX で救われる。
    #[test]
    fn guard_clamps_balloon_rect_that_alone_becomes_invisible() {
        for dpi in DPIS {
            let snapshot = mixed_layout(dpi);
            let wa = right_wa(dpi);
            let c_size = char_size(dpi);
            let b_size = balloon_size(dpi);

            // キャラ窓は右端ぎりぎりに clamp 済み（可視）
            let char_pos = point(wa.right - c_size.w, grounded_y(wa, c_size));
            assert!(overlaps(win(char_pos, c_size), wa), "前提: キャラは可視");

            // offset 恒等式（キャラの右上へ出す）が work area の外を指す
            let offset = point(px(320, dpi), -px(200, dpi));
            let proposed = point(char_pos.x + offset.x, char_pos.y + offset.y);
            let old_balloon = win(point(px(800, dpi), proposed.y), b_size);
            assert!(overlaps(old_balloon, wa), "前提: 旧バルーンは可視だった");
            assert!(
                !overlaps(win(proposed, b_size), wa) && !overlaps(win(proposed, b_size), left_wa()),
                "前提: 提案バルーン矩形はどの work area とも交差しない"
            );

            let verdict = guard_visibility(Some(old_balloon), proposed, b_size, wa, &snapshot);
            let VisibilityVerdict::ClampX(got) = verdict else {
                panic!("dpi={dpi}: バルーンも同一規則で ClampX（got {verdict:?}）");
            };
            assert_eq!(got.y, proposed.y, "dpi={dpi}: バルーンの Y も変更しない");
            assert!(
                got.x >= wa.left && got.x <= wa.right - b_size.w,
                "dpi={dpi}: バルーン X も clamp_wa の水平範囲内"
            );
            assert!(
                overlaps(win(got, b_size), wa),
                "dpi={dpi}: clamp 後のバルーン矩形は work area と交差する（Req 3.4）"
            );
            // clamp によりキャラと部分的に重なり得る＝許容（見えない会話より重なった会話）
        }
    }

    /// バルーンが交差を保っているあいだは素通し（キャラと同一規則）。
    #[test]
    fn guard_keeps_balloon_rect_while_intersecting() {
        for dpi in DPIS {
            let snapshot = mixed_layout(dpi);
            let wa = right_wa(dpi);
            let b_size = balloon_size(dpi);
            let proposed = point(px(600, dpi), px(200, dpi));
            let old = win(point(px(500, dpi), px(200, dpi)), b_size);
            assert_eq!(
                guard_visibility(Some(old), proposed, b_size, wa, &snapshot),
                VisibilityVerdict::Keep(proposed),
                "dpi={dpi}: 交差維持のバルーンは素通し"
            );
        }
    }

    /// ユーザーが画面外へ留置したバルーンは引き戻さない（キャラと同一規則）。
    #[test]
    fn guard_respects_balloon_parked_off_screen() {
        for dpi in DPIS {
            let snapshot = mixed_layout(dpi);
            let wa = right_wa(dpi);
            let b_size = balloon_size(dpi);
            let old = win(point(wa.right + px(200, dpi), px(200, dpi)), b_size);
            assert!(
                !overlaps(old, wa) && !overlaps(old, left_wa()),
                "前提: 旧バルーンは既に非交差（ユーザー留置）"
            );
            let proposed = point(wa.right + px(600, dpi), px(200, dpi));
            assert_eq!(
                guard_visibility(Some(old), proposed, b_size, wa, &snapshot),
                VisibilityVerdict::Keep(proposed),
                "dpi={dpi}: 留置バルーンは尊重する"
            );
        }
    }

    /// Y 不変の横断檻: 全分岐・キャラ／バルーン両寸で `position().y == proposed.y`
    /// （Y は射影 T の所有・D6）。分岐の識別（Keep か ClampX か）も同時に固定する。
    ///
    /// # 檻の非空虚性の要（レビュー #1・2026-07-31 の指摘に対する是正）
    ///
    /// 提案 Y に射影 T 由来の接地値（`wa.bottom − h`）だけを与えると、その Y は
    /// **work area の Y clamp の不動点**であるため「ガードが Y も clamp する」という
    /// 実在しやすい退行（`y: proposed.y.min(wa.bottom − h).max(wa.top)`）と正しい実装が
    /// 区別できず、檻が空虚になる。よって各分岐へ
    /// `[clamp_wa.top, clamp_wa.bottom − h]` の**範囲外**の Y を必ず通す。
    ///
    /// 範囲外 Y の投入は契約上も正当である——`guard_visibility` の前提条件は正寸のみ
    /// であり（design.md:425）、Y の値域は射影 T の関心であってガードの前提ではない。
    #[test]
    fn guard_never_modifies_y_in_any_branch() {
        for dpi in DPIS {
            let snapshot = mixed_layout(dpi);
            let wa = right_wa(dpi);
            for size in [char_size(dpi), balloon_size(dpi)] {
                // Y clamp の**不動点**（射影 T が出す接地 Y）＝従来の網羅を維持する側
                let y_fixed = grounded_y(wa, size);
                // Y clamp の不動点**ではない** Y ＝ clamp が入れば必ず動く側
                let y_above = wa.top - px(300, dpi); // 上端より上
                let y_below = wa.bottom + px(200, dpi); // 下端より下
                let y_partial = wa.top - size.h / 2; // 上端を跨ぐ（水平内なら交差は保つ）
                for y in [y_above, y_below, y_partial] {
                    assert!(
                        y < wa.top || y > wa.bottom - size.h,
                        "前提: {y} は work area Y clamp の不動点であってはならない\
                         （dpi={dpi} size={size:?}）"
                    );
                }

                let x_in = px(800, dpi);
                let x_far = wa.right + px(900, dpi);
                let old_visible = Some(win(point(px(700, dpi), y_fixed), size));
                let old_parked = Some(win(point(wa.right + px(500, dpi), y_fixed), size));
                let in_partial = point(x_in, y_partial);
                let far_above = point(x_far, y_above);
                let far_below = point(x_far, y_below);
                let in_fixed = point(x_in, y_fixed);
                let far_fixed = point(x_far, y_fixed);

                for (label, old, proposed, expect_clamped) in [
                    // --- 範囲外 Y（Y clamp 退行を必ず捕まえる側）---
                    ("Keep 交差維持", old_visible, in_partial, false),
                    ("ClampX 遷移", old_visible, far_above, true),
                    ("Keep 留置尊重", old_parked, far_below, false),
                    ("ClampX 安全側", None, far_below, true),
                    // --- 不動点 Y（射影 T の実出力に相当する正常系）---
                    ("Keep 交差維持@接地Y", old_visible, in_fixed, false),
                    ("ClampX 遷移@接地Y", old_visible, far_fixed, true),
                    ("Keep 留置尊重@接地Y", old_parked, far_fixed, false),
                    ("ClampX 安全側@接地Y", None, far_fixed, true),
                ] {
                    let verdict = guard_visibility(old, proposed, size, wa, &snapshot);
                    assert_eq!(
                        matches!(verdict, VisibilityVerdict::ClampX(_)),
                        expect_clamped,
                        "dpi={dpi} {label}: 分岐の識別が想定と違う\
                         （size={size:?} proposed={proposed:?} verdict={verdict:?}）"
                    );
                    assert_eq!(
                        verdict.position().y,
                        proposed.y,
                        "dpi={dpi} {label}: Y は全分岐で不変\
                         （size={size:?} proposed={proposed:?}）"
                    );
                }
            }
        }
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
                Anchored(Anchor::Bottom),
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

    /// (b') 非 Bottom アンカーの drag 配線存在チェック（Req1.6・design Integration
    /// Tests #8 末尾・[[test-only-decision-branches-not-proven-wiring]] の「一度」）:
    /// `Anchored(Left)` 窓のドラッグで X=`wa.left` 固定・Y 保持（縦自由）になる。
    ///
    /// これは `on_char_drag` の drag 配線が**実 `Anchored.0`（Left）を `project_anchor`
    /// へ転送している**証拠であり、`Anchor::Bottom` をハードコードしていないことを
    /// 弁別する檻——もし Bottom 決め打ちなら X=raw.x（≠wa.left）・Y=wa.bottom−h
    /// （≠raw.y）となって落ちる。期待 `(wa.left, raw.y)` と Bottom 誤配線の
    /// `(raw.x, wa.bottom−h)` が両軸とも全く異なる座標になるよう値を選ぶ。Top/Right
    /// の drag は同一配線の再確認ゆえ足さない（proven-wiring 過剰檻の回避）。
    #[test]
    fn on_char_drag_left_anchor_pins_left_edge_and_keeps_y() {
        let mut world = World::new();
        // 96 非倍数の left=53・bottom=1043・非零原点（dpi/96 再スケール混入の檻）
        world.insert_resource(odd_edge_snapshot()); // rect(53, 37, 1877, 1043)
        let start = (1400, 600);
        // 初期窓位置＋カーソル差分で生ドラッグ座標 raw を復元（policy_mapped_position と同式）:
        // raw.x = 700 + (1500−1400) = 800／raw.y = 300 + (917−600) = 617
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(700, 300, 434, 687),
                Anchored(Anchor::Left),
                dragging_state((700, 300), start),
            ))
            .id();

        let ev = Phase::Bubble(drag_event_at(window, start, (1500, 917)));
        // donor 同様イベントは消費しない（伝播続行＝false）
        assert!(!on_char_drag(&mut world, window, window, &ev));

        // Left（左端固定・縦自由）: X=wa.left=53・Y=raw.y=617。もし配線が Bottom を
        // ハードコードしていたら (raw.x=800, wa.bottom−h=1043−687=356) となり、両軸とも
        // 全く異なる座標で落ちる（wa.left 53 ≠ raw.x 800／wa.bottom−h 356 ≠ raw.y 617）。
        assert_eq!(
            position_of(&world, window),
            Point { x: 53, y: 617 },
            "実 Anchored.0=Left を転送: X=wa.left 固定・Y=raw.y 保持（Bottom 決め打ちなら落ちる）"
        );
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
                Anchored(Anchor::Bottom),
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

    /// (d) Free 窓（`Anchored(Free)`＝move_window=true）は wndproc 委譲のまま:
    /// ハンドラはキャラ窓を書かず、DraggingState があってもポリシー写像を使わない
    /// （wndproc 更新済み WindowPos 基準でバルーン追従のみ・挙動不変・4.7/Req1.6）。
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
                Anchored(Anchor::Free),
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

    /// (d') `Anchored` 不在（安全側フォールバック・task 2.7 の新規判断分岐）: marker が
    /// 一切無い窓は Free と同じく wndproc 委譲へ倒す——DraggingState が居ても単一ライター
    /// 写像を走らせず、キャラ窓を書かない（旧「marker 無し＝Free」意味論の保存・Req1.6）。
    #[test]
    fn on_char_drag_without_anchored_stays_wndproc_delegated() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot());
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
            .id();
        let offset = PointPx { x: 498, y: -37 };
        let start = (1400, 600);
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(1207, 217, 434, 687), // wndproc がドラッグ中に更新した位置
                // Anchored は付けない（None）——DraggingState は実 flow 同様に挿入される
                BalloonFollow { balloon, offset },
                dragging_state((999, 888), start),
            ))
            .id();

        let ev = Phase::Bubble(drag_event_at(window, start, (1601, 113)));
        assert!(!on_char_drag(&mut world, window, window, &ev));

        // 単一ライター写像は走らず、キャラ窓は wndproc 更新位置のまま不動
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
                Anchored(Anchor::Bottom),
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
                Anchored(Anchor::Bottom),
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
                Anchored(Anchor::Bottom),
                dragging_state((1207, 356), start),
            ))
            .id();

        let ev = Phase::Tunnel(drag_end_event_at(window, (1601, 113)));
        assert!(!on_char_drag_end(&mut world, window, window, &ev));
        assert_eq!(position_of(&world, window), Point { x: 1250, y: 356 });
    }

    /// Task 2.2 保存フック（Req1.1/1.9・design C2）: 非 Free アンカーのキャラ窓の
    /// DragEnd で、確定位置 `mapped` が当該スコープの `WindowPos` entries として Ghost
    /// 永続スコープへ write-through される。`barrier` 後に別ハンドルの `load_scope` で
    /// 読み戻し、保存 x/y が `mapped` 位置に等しいことを固定する（persist.rs の
    /// `persist_entries_with_wiring_write_through_to_ghost_scope` と同流儀の実 publisher 檻）。
    #[test]
    fn on_char_drag_end_persists_char_pos_for_scope() {
        use std::path::{Path, PathBuf};
        use std::sync::Arc;

        use areka_sylphya::persist::{FakePersistIo, PersistIo};
        use areka_sylphya::{
            Axis, PersistKey, PersistScope, ScopeRoots, SylphyaInit, load_scope, spawn_sylphya,
        };

        use super::super::persist::PersistWiring;
        use crate::placement::spawn::CharWindowMarker;

        // 共有 fake IO（アクター Box 移送用と観測用で同一ストアを指す・persist.rs と同流儀）。
        struct SharedFakeIo(Arc<FakePersistIo>);
        impl PersistIo for SharedFakeIo {
            fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
                self.0.read(path)
            }
            fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
                self.0.commit(path, content)
            }
        }

        let shared = Arc::new(FakePersistIo::new());
        let roots = ScopeRoots {
            ghost: Some(PathBuf::from("/g")),
            ..ScopeRoots::default()
        };
        let parts = spawn_sylphya(SylphyaInit {
            roots: roots.clone(),
            io: Box::new(SharedFakeIo(shared.clone())),
            runtime_sink: None,
        });

        let mut world = World::new();
        // UI スレッド常駐の保存投函口を挿入（persist_entries が引く NonSend リソース）。
        world.insert_non_send_resource(PersistWiring {
            publisher: parts.publisher.clone(),
        });
        world.insert_resource(single_monitor_snapshot()); // 下端 1043・釘付け Y=1043−687=356

        let start = (1400, 600);
        // scope=1 の非 Free（Bottom）キャラ窓。値は (f) 檻と同一＝mapped=(1408, 356) が既知。
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(1250, 356, 434, 687),
                Anchored(Anchor::Bottom),
                CharWindowMarker { scope: 1 },
                dragging_state((1207, 356), start),
            ))
            .id();

        // 最終カーソル (1601, 113) → raw=(1408, −131) → 適用後 mapped=(1408, 356)
        let ev = Phase::Bubble(drag_end_event_at(window, (1601, 113)));
        assert!(!on_char_drag_end(&mut world, window, window, &ev));

        // 確定点: mapped が WindowPos へ反映されている
        assert_eq!(position_of(&world, window), Point { x: 1408, y: 356 });

        // barrier 復帰＝上記 put の write-through 保存まで完了（同一送信端 FIFO）。
        parts
            .publisher
            .barrier()
            .expect("barrier should resolve while actor is alive");

        // 別ハンドルの load_scope で scope1 の WindowPos を観測（実 IO 通過＝投函の証明）。
        // 保存 x は**原点＝下端中央**基準（左上 1408 ＋ w/2=217 → 1625）。
        let loaded = load_scope(PersistScope::Ghost, &roots, &SharedFakeIo(shared.clone()));
        assert!(
            loaded.contains(&(
                PersistKey::WindowPos {
                    scope: 1,
                    axis: Axis::X
                },
                "1625".to_string()
            )),
            "DragEnd 確定位置 X=1408 が scope1 の WindowPos として保存されていない: {loaded:?}"
        );
        assert!(
            loaded.contains(&(
                PersistKey::WindowPos {
                    scope: 1,
                    axis: Axis::Y
                },
                "356".to_string()
            )),
            "DragEnd 確定位置 Y=356 が scope1 の WindowPos として保存されていない: {loaded:?}"
        );

        // 正典終了（アクター join）——テスト後始末（リーク回避・非本質）。
        parts.publisher.close();
        let _ = parts.handle.join();
    }

    /// Task 8.1 偽 Free アンカー DragEnd→保存値等価の檻（Req1.1・design C2/C3・
    /// Testing Strategy Unit §7）: `Anchored(Anchor::Free)` のキャラ窓を headless World に
    /// 合成し、DragEnd 駆動→`project_anchor` の Free identity 腕を素通しした確定位置が、
    /// **アンカー種別を問わず**そのまま `WindowPos` entries 化されて Ghost 永続スコープへ
    /// write-through されることを決定論固定する。
    ///
    /// なぜこの檻が正本か: 保存はドラッグ中の吸着制約（Bottom 等）ではなく DragEnd の
    /// 確定位置を書く（Req1.1）。Free は wndproc（move_window=true）が動かし切った位置を
    /// `project_anchor` が identity で無害通過させ、本ハンドラが**保存専用アーム**として
    /// 働く——実 emo2 は全スコープ Bottom（実機で Free の保存経路を一度も踏まない）ゆえ、
    /// この偽 Free 檻だけがその等価性の source of truth となる。
    ///
    /// 檻の噛み方（射影が Free 位置を改変したら落ちる）: snapshot を挿入し、確定 raw の
    /// Y=883 は同モニタの bottom 吸着値（1043−687=356）と**異なる**値を選ぶ。もし Free が
    /// identity でなく（誤って Bottom 等へ）射影されれば mapped.y と保存 Y が 356 へ変わり、
    /// position_of・load_scope の双方が落ちる。座標は 96 の非倍数（1531・883）で隠れた
    /// dpi/96 再スケールの檻も兼ね、既定値（0・96 系）と重ならない。
    #[test]
    fn on_char_drag_end_persists_free_anchor_raw_position_for_scope() {
        use std::path::{Path, PathBuf};
        use std::sync::Arc;

        use areka_sylphya::persist::{FakePersistIo, PersistIo};
        use areka_sylphya::{
            Axis, PersistKey, PersistScope, ScopeRoots, SylphyaInit, load_scope, spawn_sylphya,
        };

        use super::super::persist::PersistWiring;
        use crate::placement::spawn::CharWindowMarker;

        // 共有 fake IO（アクター Box 移送用と観測用で同一ストアを指す・上の Bottom 檻と同流儀）。
        struct SharedFakeIo(Arc<FakePersistIo>);
        impl PersistIo for SharedFakeIo {
            fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
                self.0.read(path)
            }
            fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
                self.0.commit(path, content)
            }
        }

        let shared = Arc::new(FakePersistIo::new());
        let roots = ScopeRoots {
            ghost: Some(PathBuf::from("/g")),
            ..ScopeRoots::default()
        };
        let parts = spawn_sylphya(SylphyaInit {
            roots: roots.clone(),
            io: Box::new(SharedFakeIo(shared.clone())),
            runtime_sink: None,
        });

        let mut world = World::new();
        world.insert_non_send_resource(PersistWiring {
            publisher: parts.publisher.clone(),
        });
        // snapshot 挿入（bottom=1043）。Free identity なら未使用だが、誤射影時に
        // Bottom 吸着 Y=1043−687=356 が現れる差分検出のため意図的に居させる。
        world.insert_resource(single_monitor_snapshot());

        // scope=2 の Free キャラ窓。DraggingState は実 flow（dispatch_drag_events 挿入）を模す。
        // initial_inset=(1250,356)＝ドラッグ開始時窓位置・drag_start=(1400,600)＝開始カーソル。
        let start = (1400, 600);
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(1250, 356, 434, 687),
                Anchored(Anchor::Free),
                CharWindowMarker { scope: 2 },
                dragging_state((1250, 356), start),
            ))
            .id();

        // 最終カーソル (1681, 1127) → raw = (1250+(1681−1400), 356+(1127−600)) = (1531, 883)。
        // Free identity ゆえ mapped = raw = wndproc 確定位置（射影で改変されない）。
        let ev = Phase::Bubble(drag_end_event_at(window, (1681, 1127)));
        assert!(!on_char_drag_end(&mut world, window, window, &ev));

        // 確定点: mapped=(1531,883) が WindowPos へ反映（Bottom 吸着 356 ではなく生確定 883）。
        assert_eq!(
            position_of(&world, window),
            Point { x: 1531, y: 883 },
            "Free は identity 射影＝確定 raw をそのまま反映（Bottom 誤射影なら Y=356 で落ちる）"
        );

        // barrier 復帰＝上記 put の write-through 保存まで完了（同一送信端 FIFO）。
        parts
            .publisher
            .barrier()
            .expect("barrier should resolve while actor is alive");

        // 別ハンドルの load_scope で scope2 の WindowPos を読み戻す（実 IO 通過＝投函の証明）。
        // Free アンカーでも保存値は確定 raw と value-equal（アンカー種別を問わない・Req1.1）。
        let loaded = load_scope(PersistScope::Ghost, &roots, &SharedFakeIo(shared.clone()));
        assert!(
            loaded.contains(&(
                PersistKey::WindowPos {
                    scope: 2,
                    axis: Axis::X
                },
                "1531".to_string()
            )),
            "Free DragEnd 確定 X=1531 が scope2 の WindowPos として保存されていない: {loaded:?}"
        );
        assert!(
            loaded.contains(&(
                PersistKey::WindowPos {
                    scope: 2,
                    axis: Axis::Y
                },
                "883".to_string()
            )),
            "Free DragEnd 確定 Y=883 が scope2 の WindowPos として保存されていない\
             （Bottom 誤射影なら 356・保存脱落なら空）: {loaded:?}"
        );

        // 正典終了（アクター join）——テスト後始末（リーク回避・非本質）。
        parts.publisher.close();
        let _ = parts.handle.join();
    }

    /// Task 8.3 発火規律の統合檻（Req1.9・8.4・design C2/C3・Testing Strategy Integration §2）:
    /// 永続の窓位置・バルーン相対オフセットを書くのは **DragEnd の観測点のみ**であり、
    /// 自動再射影（`resize_window_to`）・`\![move]` 消費経路（`move_window_to`）・復元時
    /// 再射影（`apply_restored_placements`・純関数）・**連続ドラッグ**（`on_char_drag`）は
    /// 永続ストアを一切書き換えないことを、ストア内容のバイト等価で決定論固定する。
    ///
    /// # 檻の噛み方（意味のある不変チェックにするための seed）
    ///
    /// まず 1 回の正当な書込（char の `on_char_drag_end`）で **ストアに内容を与えて**から
    /// スナップショットを捕捉する（空ストア同士の比較では「何も書かない」ことが自明に
    /// 成立してしまい檻が噛まないため）。その後に非 DragEnd 操作群を駆動し、`barrier` を
    /// 挟んで（保留 put があれば flush される）ストア内容を再捕捉し、seed 時点と **完全一致**
    /// することを assert する。`load_scope` は決定論順（sylphya 契約）ゆえ Vec を直接比較できる。
    ///
    /// もし駆動した操作のいずれかが `persist_put` を投函すれば、scope1 の WindowPos／
    /// BalloonOffset entries が変化して seed スナップショットと乖離し、本 assert が落ちる
    /// （RED 検証: 駆動ブロックへ一時的に `persist_entries` を差し込むと本檻が実際に落ちることを
    /// 確認済み——発火規律が破れれば必ず検出する）。emo2 は全スコープ Bottom＝実機で Free の
    /// 保存経路を踏まないのと同様、自動再射影が永続へ漏れないことは決定論檻でのみ観測できる。
    #[test]
    fn non_dragend_operations_leave_persist_store_byte_invariant() {
        use std::path::{Path, PathBuf};
        use std::sync::Arc;

        use areka_sylphya::persist::{FakePersistIo, PersistIo};
        use areka_sylphya::{PersistScope, ScopeRoots, SylphyaInit, load_scope, spawn_sylphya};

        use super::super::persist::{PersistWiring, apply_restored_placements};
        use crate::placement::resolver::ScopePlacement;
        use crate::placement::spawn::{BalloonWindowMarker, CharWindowMarker};

        // 共有 fake IO（アクター Box 移送用と観測用で同一ストアを指す・上の DragEnd 檻と同流儀）。
        struct SharedFakeIo(Arc<FakePersistIo>);
        impl PersistIo for SharedFakeIo {
            fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
                self.0.read(path)
            }
            fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
                self.0.commit(path, content)
            }
        }

        let shared = Arc::new(FakePersistIo::new());
        let roots = ScopeRoots {
            ghost: Some(PathBuf::from("/g")),
            ..ScopeRoots::default()
        };
        let parts = spawn_sylphya(SylphyaInit {
            roots: roots.clone(),
            io: Box::new(SharedFakeIo(shared.clone())),
            runtime_sink: None,
        });

        let mut world = World::new();
        world.insert_non_send_resource(PersistWiring {
            publisher: parts.publisher.clone(),
        });
        world.insert_resource(single_monitor_snapshot()); // 下端 1043・釘付け Y=1043−687=356

        // char 窓 scope=1（Bottom）＋ balloon 窓 scope=1（BalloonFollow で連結）。
        let balloon = world
            .spawn((
                fake_handle(0x2000),
                window_pos_at(701, 383),
                BalloonWindowMarker { scope: 1 },
            ))
            .id();
        let start = (1400, 600);
        let char_window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(1250, 356, 434, 687),
                Anchored(Anchor::Bottom),
                CharWindowMarker { scope: 1 },
                BalloonFollow {
                    balloon,
                    offset: PointPx { x: -549, y: 27 },
                },
                dragging_state((1207, 356), start),
            ))
            .id();

        // --- SEED: 1 回の正当な書込（char DragEnd）でストアに内容を与える（不変チェックを
        //     意味あるものにするため）。最終カーソル (1601,113) → mapped=(1408, 356)。
        let ev = Phase::Bubble(drag_end_event_at(char_window, (1601, 113)));
        assert!(!on_char_drag_end(&mut world, char_window, char_window, &ev));
        parts
            .publisher
            .barrier()
            .expect("seed barrier should resolve while actor is alive");

        // seed 書込後のストア内容を正準スナップショットとして捕捉（load_scope は決定論順）。
        let before = load_scope(PersistScope::Ghost, &roots, &SharedFakeIo(shared.clone()));
        assert!(
            !before.is_empty(),
            "seed の DragEnd 書込がストアを満たしていない＝不変チェックが無意味になる: {before:?}"
        );

        // --- DRIVE: 書いてはならない非 DragEnd 操作群を駆動する ---------------------------
        // 1) `\![move]` 消費経路（apply_move_directive が唯一呼ぶ位置ライター）。
        assert!(
            move_window_to(&mut world, char_window, 999, 777),
            "move_window_to は成立するはず（char に WindowHandle あり）"
        );
        // 2) 自動再射影（re-snap）経路。Bottom → project_anchor で y=1043−700=343 へ再固定。
        assert!(
            resize_window_to(
                &mut world,
                char_window,
                SizePx { w: 500, h: 700 },
                PlacementRoute::Resnap
            ),
            "resize_window_to は成立するはず（Anchored/正寸/WindowHandle あり）"
        );
        // 3) 復元時再射影（純関数・World も永続も触れない・返り値は捨てる）。
        let snap = single_monitor_snapshot();
        let placements = vec![ScopePlacement {
            scope: 1,
            char_pos: PointPx { x: 1250, y: 356 },
            char_size: SizePx { w: 434, h: 687 },
            balloon_pos: PointPx { x: 701, y: 383 },
            balloon_size: SizePx { w: 200, h: 300 },
            balloon_offset: PointPx { x: -549, y: 27 },
            anchor: Anchor::Bottom,
        }];
        let _restored = apply_restored_placements(placements, &before, &snap);
        // 4) 連続ドラッグ（DragEnd ではない・書込トリガにしない確定点規律）。
        let drag = Phase::Bubble(drag_event_at(char_window, start, (1450, 350)));
        assert!(!on_char_drag(&mut world, char_window, char_window, &drag));

        // 保留 put（存在しないはず）があれば flush する越境フェンス。
        parts
            .publisher
            .barrier()
            .expect("post-drive barrier should resolve while actor is alive");

        // --- ASSERT: ストア内容が seed 時点と完全一致（非 DragEnd 操作は何も書いていない）---
        let after = load_scope(PersistScope::Ghost, &roots, &SharedFakeIo(shared.clone()));
        assert_eq!(
            before, after,
            "非 DragEnd 操作（move_window_to / resize_window_to / apply_restored_placements / \
             連続 on_char_drag）が永続ストアを書き換えた（Req1.9/8.4 発火規律違反）: \
             before={before:?} after={after:?}"
        );

        // 正典終了（アクター join）——テスト後始末（リーク回避・非本質）。
        parts.publisher.close();
        let _ = parts.handle.join();
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
                Anchored(Anchor::Bottom),
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
                Anchored(Anchor::Bottom),
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
                Anchored(Anchor::Bottom),
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
                Anchored(Anchor::Bottom),
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
                Anchored(Anchor::Bottom),
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
                Anchored(Anchor::Bottom),
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
    // on_balloon_drag_end: バルーン単独ドラッグ確定 offset の永続 write-through
    // （task 2.3・Req2.1・8.1・design C2/C3）
    //
    // バルーン窓は move_window=true ゆえ wndproc が実窓位置を WindowPos.position へ
    // 更新済み——DragEnd 時点の最終確定位置はこの WindowPos.position で読める
    // （on_balloon_drag と同源）。on_balloon_drag_end は最終確定位置から
    // offset = balloon_pos − char_pos を**再導出**（in-session BalloonFollow.offset は
    // 使わない）し、balloon_offset_to_persist でアンカー辺基準へ変換して
    // BalloonOffset entries を Ghost 永続スコープへ即時 write-through する。
    // 実 publisher（spawn_sylphya + SharedFakeIo）で barrier→load_scope し、保存値が
    // 最終確定位置由来の persist 値に一致することを固定する（Issue 1 対応・2.1/8.1）。
    // -------------------------------------------------------------------------

    /// Task 2.3 保存フック（Req2.1/8.1・design C3）: バルーン窓の DragEnd で、最終確定
    /// 位置から**再導出**した相対 offset がアンカー辺基準へ変換され scope の
    /// BalloonOffset として Ghost 永続スコープへ write-through される。**in-session の
    /// BalloonFollow.offset は SAVE に使わない**（DragEnd 最終確定位置から再導出）——
    /// stale な offset を仕込んで弁別する。
    #[test]
    fn on_balloon_drag_end_persists_balloon_offset_for_scope() {
        use std::path::{Path, PathBuf};
        use std::sync::Arc;

        use areka_sylphya::persist::{FakePersistIo, PersistIo};
        use areka_sylphya::{
            Axis, PersistKey, PersistScope, ScopeRoots, SylphyaInit, load_scope, spawn_sylphya,
        };

        use super::super::persist::{PersistWiring, balloon_offset_to_persist};
        use super::on_balloon_drag_end;
        use crate::placement::spawn::BalloonWindowMarker;

        // 共有 fake IO（アクター Box 移送用と観測用で同一ストアを指す・persist.rs と同流儀）。
        struct SharedFakeIo(Arc<FakePersistIo>);
        impl PersistIo for SharedFakeIo {
            fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
                self.0.read(path)
            }
            fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
                self.0.commit(path, content)
            }
        }

        let shared = Arc::new(FakePersistIo::new());
        let roots = ScopeRoots {
            ghost: Some(PathBuf::from("/g")),
            ..ScopeRoots::default()
        };
        let parts = spawn_sylphya(SylphyaInit {
            roots: roots.clone(),
            io: Box::new(SharedFakeIo(shared.clone())),
            runtime_sink: None,
        });

        let mut world = World::new();
        // UI スレッド常駐の保存投函口を挿入（persist_entries が引く NonSend リソース）。
        world.insert_non_send_resource(PersistWiring {
            publisher: parts.publisher.clone(),
        });

        // char 窓（Bottom・emo2 実寸）と、単独ドラッグで wndproc が最終確定位置へ移した
        // balloon 窓。値はいずれも 96 の倍数を避け、隠れた dpi/96 再スケールの檻とする。
        let char_size = SizePx { w: 434, h: 687 };
        let char_pos = Point { x: 1483, y: 733 };
        let final_balloon_pos = Point { x: 1071, y: 708 }; // wndproc の最終確定位置
        let anchor = Anchor::Bottom;

        // stale な in-session offset（SAVE に誤用したら弁別で落ちる檻の値）。
        let stale_offset = PointPx { x: 999, y: 888 };

        let balloon = world
            .spawn((
                fake_handle(0x2000),
                window_pos_at(final_balloon_pos.x, final_balloon_pos.y),
                BalloonWindowMarker { scope: 1 },
            ))
            .id();
        let char_w = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(char_pos.x, char_pos.y, char_size.w, char_size.h),
                Anchored(anchor),
                BalloonFollow {
                    balloon,
                    offset: stale_offset,
                },
            ))
            .id();

        // 期待 persist 値 = 最終確定位置から再導出（in-session offset ではない）。
        let offset_tl = PointPx {
            x: final_balloon_pos.x - char_pos.x,
            y: final_balloon_pos.y - char_pos.y,
        };
        let expected = balloon_offset_to_persist(anchor, offset_tl, char_size);
        assert_ne!(
            expected, stale_offset,
            "檻の前提: 最終確定 offset は stale な in-session offset と異なる"
        );

        // DragEnd をバルーン窓へ配送（cursor 値は無関係＝最終確定位置は balloon 窓の
        // WindowPos.position を読む・move_window=true）。
        let ev = Phase::Bubble(drag_end_event_at(balloon, (0, 0)));
        assert!(!on_balloon_drag_end(&mut world, balloon, balloon, &ev));

        // キャラ窓は不動・BalloonFollow.offset（in-session 表現）も on_balloon_drag_end では
        // 変えない（保存は最終確定位置から独立に導出する）。
        assert_eq!(position_of(&world, char_w), char_pos);
        assert_eq!(
            world.get::<BalloonFollow>(char_w).unwrap().offset,
            stale_offset,
            "on_balloon_drag_end は in-session offset を変異させない（保存専用）"
        );

        // barrier 復帰＝上記 put の write-through 保存まで完了（同一送信端 FIFO）。
        parts
            .publisher
            .barrier()
            .expect("barrier should resolve while actor is alive");

        // 別ハンドルの load_scope で scope1 の BalloonOffset を観測（実 IO 通過＝投函の証明）。
        let loaded = load_scope(PersistScope::Ghost, &roots, &SharedFakeIo(shared.clone()));
        assert!(
            loaded.contains(&(
                PersistKey::BalloonOffset {
                    scope: 1,
                    axis: Axis::X
                },
                expected.x.to_string()
            )),
            "バルーン DragEnd の最終確定 offset X={} が scope1 の BalloonOffset として保存されていない: {loaded:?}",
            expected.x
        );
        assert!(
            loaded.contains(&(
                PersistKey::BalloonOffset {
                    scope: 1,
                    axis: Axis::Y
                },
                expected.y.to_string()
            )),
            "バルーン DragEnd の最終確定 offset Y={} が scope1 の BalloonOffset として保存されていない: {loaded:?}",
            expected.y
        );

        // 正典終了（アクター join）——テスト後始末（リーク回避・非本質）。
        parts.publisher.close();
        let _ = parts.handle.join();
    }

    /// Task 8.2 保存→復元 往復値等価の END-TO-END 統合檻（Req8.1・Req7.2・design
    /// Testing Strategy Integration §1）。
    ///
    /// 実 `FsPersistIo`＋temp dir に置いた**最小解決可能ゴースト**へ、save 側（DragEnd 観測点
    /// →`PersistWiring`→実アクター→`sylphya.toml`）と restore 側（`load_restored_state`
    /// →`apply_restored_placements`）を実ファイルシステム越しに結線し、キャラ位置・バルーン
    /// オフセットが値等価で往復すること、および同居する無関係 key（`BootCount`）が save で
    /// 破壊されないことを決定論固定する。
    ///
    /// これまでの follow.rs 檻は `FakePersistIo`（インメモリ）で「投函→load_scope」の
    /// 送信端 FIFO を証明したが、本檻は**実 FS 書込＋mount 解決経由の実読出**で往復全体
    /// （save→file→resolve→load→merge）を一本の檻に収める（design §1 が実 `FsPersistIo`
    /// を要求する所以）。
    ///
    /// 檻の噛み方（往復が壊れたら落ちる）:
    /// - char: save 側の bottom 吸着確定位置（1427, 513）が実ファイルへ書かれ、restore 側で
    ///   同一 work area の `project_restore` が恒等（既に下端一致・x 域内）ゆえ merge 後の
    ///   `char_pos` が確定位置と値等価に戻る。既定 char_pos(100,100) が漏れれば落ちる。
    /// - balloon: DragEnd 最終確定位置から再導出した左上基準 offset(-412,-43) が下端基準
    ///   (-412,-730) へ変換されてファイルへ、restore 側で現 char_size で左上基準へ足し戻り、
    ///   `balloon_pos` が balloon 最終確定位置(1015, 470)へ戻る。
    /// - 7.2: 事前に `persist_put` した無関係 key `BootCount="1"` が、char/balloon の DragEnd
    ///   save 後も `load_restored_state` に不変で残る（read-modify-write の無関係 key 温存）。
    ///
    /// 座標は 96 の非倍数（1427・1015・470…）で隠れた dpi/96 再スケールの檻を兼ね、既定値
    /// （100・0・96 系）と重ならない。
    #[test]
    fn round_trip_save_restore_value_equivalence_over_real_fs() {
        use std::path::PathBuf;

        use areka_ghost::sylphya_wiring::profile_areka_root;
        use areka_parsers::charset::DefaultEncoding;
        use areka_sylphya::persist::FsPersistIo;
        use areka_sylphya::{
            Axis, PersistKey, PersistScope, ScopeRoots, SylphyaInit, load_scope, spawn_sylphya,
        };

        use super::super::persist::{
            PersistWiring, apply_restored_placements, balloon_offset_from_persist,
            balloon_offset_to_persist, load_restored_state,
        };
        use super::on_balloon_drag_end;
        use crate::placement::resolver::ScopePlacement;
        use crate::placement::spawn::{BalloonWindowMarker, CharWindowMarker};

        // panic をまたいで temp dir を確実に片付ける Drop ガード。
        struct TempGhostDir(PathBuf);
        impl Drop for TempGhostDir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }

        // --- fixture: 最小解決可能ゴースト（persist.rs plant_minimal_ghost 同型）---------
        let mut root = std::env::temp_dir();
        root.push("areka_follow_round_trip_e2e_8_2");
        let _ = std::fs::remove_dir_all(&root);
        let _guard = TempGhostDir(root.clone());
        let ghost_master = root.join("ghost").join("master");
        std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
        std::fs::write(
            ghost_master.join("descript.txt"),
            "charset,UTF-8\nname,テスト\nsakura.name,さくら\n".as_bytes(),
        )
        .expect("write ghost descript");
        std::fs::create_dir_all(root.join("shell").join("master")).expect("create shell/master");

        // 永続ファイルは load_restored_state が読む場所と同一＝profile_areka_root(shiori.dir)。
        // shiori.dir は resolve が root/ghost/master へ解決する（persist.rs load 檻が証明）。
        // FsPersistIo::commit は親ディレクトリを作らないため、書込先を先に用意する
        // （本番 boot 経路は profile/areka を別途用意する・ここでは檻の前提を満たす）。
        let profile_root = profile_areka_root(&ghost_master);
        std::fs::create_dir_all(&profile_root).expect("create profile/areka");

        // --- save 側 sylphya（実 FsPersistIo・実 FS 往復）------------------------------
        let roots = ScopeRoots {
            ghost: Some(profile_root.clone()),
            ..ScopeRoots::default()
        };
        let parts = spawn_sylphya(SylphyaInit {
            roots: roots.clone(),
            io: Box::new(FsPersistIo),
            runtime_sink: None,
        });

        // 7.2 の無関係 key を DragEnd save に**先立って**植える（read-modify-write の温存対象）。
        parts.publisher.persist_put(
            PersistScope::Ghost,
            vec![(PersistKey::BootCount, "1".to_string())],
        );

        // --- headless World（char + balloon + PersistWiring）--------------------------
        let char_size = SizePx { w: 434, h: 687 };
        // work area 下端 1200 → bottom 吸着 y = 1200 − 687 = 513（save/restore 双方で同一）。
        let snapshot = MonitorSnapshot {
            work_areas: vec![rect(0, 0, 1920, 1200)],
        };

        let mut world = World::new();
        world.insert_non_send_resource(PersistWiring {
            publisher: parts.publisher.clone(),
        });
        world.insert_resource(MonitorSnapshot {
            work_areas: snapshot.work_areas.clone(),
        });

        // balloon 窓（scope1）: 単独ドラッグ確定後の最終位置は後段で明示設定する。
        let balloon = world
            .spawn((
                fake_handle(0x2000),
                window_pos_at(500, 500),
                BalloonWindowMarker { scope: 1 },
            ))
            .id();
        // char 窓（scope1・Bottom）: DraggingState + cursor から mapped=(1427, 513) が確定。
        let stale_offset = PointPx { x: 999, y: 888 };
        let char_w = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(1200, 600, char_size.w, char_size.h),
                Anchored(Anchor::Bottom),
                CharWindowMarker { scope: 1 },
                BalloonFollow {
                    balloon,
                    offset: stale_offset,
                },
                dragging_state((1250, 500), (1300, 550)),
            ))
            .id();

        // --- char DragEnd（保存）: cursor(1477,313) → raw(1427,263) → Bottom mapped(1427,513) ---
        let char_ev = Phase::Bubble(drag_end_event_at(char_w, (1477, 313)));
        assert!(!on_char_drag_end(&mut world, char_w, char_w, &char_ev));
        let char_final = position_of(&world, char_w); // Point（WindowPos 通貨）
        assert_eq!(
            char_final,
            Point { x: 1427, y: 513 },
            "char DragEnd 確定位置（bottom 吸着・96 非倍数）"
        );
        // ScopePlacement は PointPx 通貨ゆえ比較用に写す（値は同一）。
        let char_final_px = PointPx {
            x: char_final.x,
            y: char_final.y,
        };

        // --- balloon 単独ドラッグ確定位置を wndproc が置いたものとして明示設定 -----------
        // （on_char_drag_end の follow_balloon が stale offset で動かした後の、ユーザーの
        //   独立バルーンドラッグの最終確定位置。on_balloon_drag_end は WindowPos.position を読む。）
        let balloon_final = Point { x: 1015, y: 470 };
        world.get_mut::<WindowPos>(balloon).unwrap().position = Some(balloon_final);
        let balloon_final_px = PointPx {
            x: balloon_final.x,
            y: balloon_final.y,
        };

        // --- balloon DragEnd（保存）: 最終確定位置から左上基準 offset を再導出→下端基準で保存 ---
        let balloon_ev = Phase::Bubble(drag_end_event_at(balloon, (0, 0)));
        assert!(!on_balloon_drag_end(&mut world, balloon, balloon, &balloon_ev));

        // 期待 persist（下端基準）と復元 offset（左上基準）を同じ純関数で先に押さえる。
        let expected_offset_tl = PointPx {
            x: balloon_final.x - char_final.x, // 1015−1427 = −412
            y: balloon_final.y - char_final.y, // 470−513  = −43
        };
        let expected_persist =
            balloon_offset_to_persist(Anchor::Bottom, expected_offset_tl, char_size); // (−412,−730)
        assert_ne!(
            expected_offset_tl, expected_persist,
            "檻の前提: 下端基準変換が左上基準と別値（Bottom は h ぶんずれる）"
        );

        // --- barrier: 上記 3 件の put（BootCount／WindowPos／BalloonOffset）が実 FS へ確定 ---
        parts
            .publisher
            .barrier()
            .expect("barrier should resolve while actor is alive");

        // 実アクターと同一 roots・実 FsPersistIo で読み戻し、保存 entries を直接確認（往復の中間証拠）。
        // 保存 x は**原点＝下端中央**基準（左上 1427 ＋ w/2=217 → 1644）。
        let loaded = load_scope(PersistScope::Ghost, &roots, &FsPersistIo);
        assert!(
            loaded.contains(&(
                PersistKey::WindowPos {
                    scope: 1,
                    axis: Axis::X
                },
                "1644".to_string()
            )) && loaded.contains(&(
                PersistKey::WindowPos {
                    scope: 1,
                    axis: Axis::Y
                },
                "513".to_string()
            )),
            "char 確定位置が実 FS へ書かれていない: {loaded:?}"
        );
        assert!(
            loaded.contains(&(
                PersistKey::BalloonOffset {
                    scope: 1,
                    axis: Axis::X
                },
                expected_persist.x.to_string()
            )) && loaded.contains(&(
                PersistKey::BalloonOffset {
                    scope: 1,
                    axis: Axis::Y
                },
                expected_persist.y.to_string()
            )),
            "balloon 下端基準 offset が実 FS へ書かれていない: {loaded:?}"
        );

        // --- restore 側: mount 解決経由で実ファイルを読み、merge へ流す ------------------
        let entries = load_restored_state(&root, DefaultEncoding::Ansi);

        // 7.2: 無関係 key BootCount が DragEnd save 後も不変で残る（read-modify-write 温存）。
        assert!(
            entries.contains(&(PersistKey::BootCount, "1".to_string())),
            "同居する無関係 key BootCount が DragEnd save で破壊された（7.2）: {entries:?}"
        );

        // resolver 出力を模す合成 placement（既定は saved と別位置＝復元優先の証明）。
        let default_char_pos = PointPx { x: 100, y: 100 };
        let default_balloon_offset = PointPx { x: 7, y: 7 };
        let synthetic = ScopePlacement {
            scope: 1,
            char_pos: default_char_pos,
            char_size,
            balloon_pos: PointPx {
                x: default_char_pos.x + default_balloon_offset.x,
                y: default_char_pos.y + default_balloon_offset.y,
            },
            balloon_size: SizePx { w: 200, h: 300 },
            balloon_offset: default_balloon_offset,
            anchor: Anchor::Bottom,
        };
        // saved 位置を覆う work area ゆえ project_restore は恒等（既に下端一致・x 域内）。
        let out = apply_restored_placements(vec![synthetic], &entries, &snapshot);

        assert_eq!(out.len(), 1);
        // (8.1) 復元 char_pos が DragEnd 確定位置と値等価（既定を上書き）。
        assert_eq!(
            out[0].char_pos, char_final_px,
            "復元 char_pos が DragEnd 確定位置と値等価でない（1.4/8.1）"
        );
        assert_ne!(
            out[0].char_pos, default_char_pos,
            "復元が既定位置を漏らしている"
        );
        // (8.1) 復元 balloon offset（左上基準）が DragEnd 由来 offset と値等価。
        let expected_restored_offset =
            balloon_offset_from_persist(Anchor::Bottom, expected_persist, char_size);
        assert_eq!(
            expected_restored_offset, expected_offset_tl,
            "檻の前提: 下端基準⇄左上基準が現 char_size で往復恒等"
        );
        assert_eq!(
            out[0].balloon_offset, expected_offset_tl,
            "復元 balloon offset が DragEnd 由来 offset と値等価でない（2.2/2.3/8.1）"
        );
        // (8.1) 復元 balloon_pos が balloon DragEnd 最終確定位置と値等価。
        assert_eq!(
            out[0].balloon_pos, balloon_final_px,
            "復元 balloon_pos が balloon DragEnd 最終確定位置と値等価でない（2.3/8.1）"
        );
        // 事後条件（design C1）: 寸法・anchor は不変。
        assert_eq!(out[0].char_size, char_size);
        assert_eq!(out[0].anchor, Anchor::Bottom);

        // 正典終了（アクター join）——temp dir は _guard の Drop が片付ける。
        parts.publisher.close();
        let _ = parts.handle.join();
    }

    /// 実機サインオフ再現檻（多窓・2 スコープ・Bottom）: DragEnd 時に `DraggingState` を
    /// 失った char が位置を保存できず、相対追従のバルーンが復元でずれる欠陥を決定論再現する。
    ///
    /// # 背景（実機 emo2 `sylphya.toml` の観測異常）
    ///
    /// 4 窓（scope0/1 の char+balloon）を各々ドラッグしたにもかかわらず `[window.1]` のみ保存され
    /// `[window.0]` が欠落（一方 `[balloon-offset.0/1]` は両方保存）。復元時 scope0(むらさき) char が
    /// resolver 既定へスナップし、相対追従のバルーン（Req1.6: 位置の単一真実源はキャラ窓）が既定
    /// char へ引きずられて位置がずれた。
    ///
    /// # 再現する根本経路（root cause）
    ///
    /// `on_char_drag_end` は保存位置を `policy_mapped_position`（＝`DraggingState` からの生座標
    /// 再導出）に依存させており、`DraggingState` 不在なら `None` で**早期 return＝保存 skip** する。
    /// しかし非 Free char は連続 `on_char_drag` が既に最終位置へ動かし済みで `WindowPos.position` が
    /// 最終確定位置を保持している。dispatch が DragEnd 前に `DraggingState` を落とすと（多窓時に
    /// observed・実 flow の穴）、char は動いたのに位置が保存されない——一方 `on_balloon_drag_end` は
    /// char の `WindowPos.position` を読んで offset を保存するため balloon-offset だけが残り、実機の
    /// 観測状態（`[window.0]` 欠落・`[balloon-offset.0]` 残存）に一致する。
    ///
    /// # 檻の噛み方
    ///
    /// - scope0 char: `DraggingState` **無し**・`WindowPos.position` は連続ドラッグが置いた最終位置。
    ///   修正前は `on_char_drag_end` が保存 skip → `[window.0]` 欠落 → 復元で char が既定へ落ち、
    ///   balloon が既定 char へ追従してずれる（RED）。修正後は `WindowPos.position` を最終位置として
    ///   保存 → 復元で char/balloon とも最終確定位置へ戻る（GREEN）。
    /// - scope1 char: `DraggingState` **有り**（正常経路の対照）。修正前後で常に保存・復元される。
    ///
    /// 座標は 96 の非倍数を用い（隠れた dpi/96 再スケールの副次檻）、scope 間・既定値と重ねない。
    #[test]
    fn dragged_char_persists_even_without_dragging_state_at_dragend() {
        use std::path::PathBuf;

        use areka_ghost::sylphya_wiring::profile_areka_root;
        use areka_parsers::charset::DefaultEncoding;
        use areka_sylphya::persist::FsPersistIo;
        use areka_sylphya::{
            Axis, PersistKey, PersistScope, ScopeRoots, SylphyaInit, load_scope, spawn_sylphya,
        };

        use super::super::persist::{
            PersistWiring, apply_restored_placements, balloon_offset_from_persist,
            balloon_offset_to_persist, load_restored_state,
        };
        use super::on_balloon_drag_end;
        use crate::placement::resolver::ScopePlacement;
        use crate::placement::spawn::{BalloonWindowMarker, CharWindowMarker};

        struct TempGhostDir(PathBuf);
        impl Drop for TempGhostDir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }

        // --- fixture: 最小解決可能ゴースト（round_trip 8.2 と同型）------------------------
        let mut root = std::env::temp_dir();
        root.push("areka_follow_dragend_no_dragging_state_repro");
        let _ = std::fs::remove_dir_all(&root);
        let _guard = TempGhostDir(root.clone());
        let ghost_master = root.join("ghost").join("master");
        std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
        std::fs::write(
            ghost_master.join("descript.txt"),
            "charset,UTF-8\nname,テスト\nsakura.name,さくら\n".as_bytes(),
        )
        .expect("write ghost descript");
        std::fs::create_dir_all(root.join("shell").join("master")).expect("create shell/master");
        let profile_root = profile_areka_root(&ghost_master);
        std::fs::create_dir_all(&profile_root).expect("create profile/areka");

        // --- save 側 sylphya（実 FsPersistIo・実 FS 往復）------------------------------
        let roots = ScopeRoots {
            ghost: Some(profile_root.clone()),
            ..ScopeRoots::default()
        };
        let parts = spawn_sylphya(SylphyaInit {
            roots: roots.clone(),
            io: Box::new(FsPersistIo),
            runtime_sink: None,
        });

        // work area 下端 1200・単一モニタ。両スコープの Bottom 吸着 y を確定する。
        let snapshot = MonitorSnapshot {
            work_areas: vec![rect(0, 0, 1920, 1200)],
        };
        let mut world = World::new();
        world.insert_non_send_resource(PersistWiring {
            publisher: parts.publisher.clone(),
        });
        world.insert_resource(MonitorSnapshot {
            work_areas: snapshot.work_areas.clone(),
        });

        // scope0（むらさき）: char_size(434,687)→ bottom 吸着 y = 1200−687 = 513。
        let s0_size = SizePx { w: 434, h: 687 };
        let s0_char_final = Point { x: 1427, y: 513 };
        let s0_balloon_final = Point { x: 1289, y: 529 };
        // scope1（エモ）: char_size(400,600)→ bottom 吸着 y = 1200−600 = 600。
        let s1_size = SizePx { w: 400, h: 600 };
        let s1_char_final = Point { x: 811, y: 600 };
        let s1_balloon_final = Point { x: 985, y: 727 };

        // scope0 char: DraggingState **無し**（DragEnd 前に dispatch が落とした穴）。連続ドラッグが
        // 既に最終位置へ動かし済みとして WindowPos.position を最終確定位置で spawn する。
        let s0_balloon = world
            .spawn((
                fake_handle(0x2000),
                window_pos_at(500, 500),
                BalloonWindowMarker { scope: 0 },
            ))
            .id();
        let s0_char = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(s0_char_final.x, s0_char_final.y, s0_size.w, s0_size.h),
                Anchored(Anchor::Bottom),
                CharWindowMarker { scope: 0 },
                BalloonFollow {
                    balloon: s0_balloon,
                    offset: PointPx { x: 111, y: 222 },
                },
                // ここに dragging_state を**付けない**のが本檻の肝。
            ))
            .id();
        // scope1 char: DraggingState **有り**（正常経路の対照）。raw.x=811（cursor==drag_start）。
        let s1_balloon = world
            .spawn((
                fake_handle(0x4000),
                window_pos_at(700, 700),
                BalloonWindowMarker { scope: 1 },
            ))
            .id();
        let s1_char = world
            .spawn((
                fake_handle(0x3000),
                window_pos_sized(800, 650, s1_size.w, s1_size.h),
                Anchored(Anchor::Bottom),
                CharWindowMarker { scope: 1 },
                BalloonFollow {
                    balloon: s1_balloon,
                    offset: PointPx { x: 333, y: 444 },
                },
                dragging_state((s1_char_final.x, 650), (1000, 1000)),
            ))
            .id();

        // --- 実機と同じ順序で 4 回ドラッグ確定（char0 → char1 → balloon0 → balloon1）---
        // char0 DragEnd: DraggingState 不在。修正後は WindowPos.position(1427,513) を最終位置に採る。
        assert!(!on_char_drag_end(
            &mut world,
            s0_char,
            s0_char,
            &Phase::Bubble(drag_end_event_at(s0_char, (1000, 1000))),
        ));
        assert_eq!(
            position_of(&world, s0_char),
            s0_char_final,
            "scope0 char は連続ドラッグ最終位置を保持（DragEnd は位置を変えない）"
        );
        // char1 DragEnd: raw(811,650)→Bottom mapped(811,600)。
        assert!(!on_char_drag_end(
            &mut world,
            s1_char,
            s1_char,
            &Phase::Bubble(drag_end_event_at(s1_char, (1000, 1000))),
        ));
        assert_eq!(
            position_of(&world, s1_char),
            s1_char_final,
            "scope1 char DragEnd 確定位置（bottom 吸着・DraggingState 経路）"
        );

        // balloon の最終確定位置を wndproc が置いたものとして明示設定（move_window=true 相当）。
        world.get_mut::<WindowPos>(s0_balloon).unwrap().position = Some(s0_balloon_final);
        world.get_mut::<WindowPos>(s1_balloon).unwrap().position = Some(s1_balloon_final);
        // balloon0 DragEnd（保存）: char0 の WindowPos.position(1427,513) 基準に offset を保存。
        assert!(!on_balloon_drag_end(
            &mut world,
            s0_balloon,
            s0_balloon,
            &Phase::Bubble(drag_end_event_at(s0_balloon, (0, 0))),
        ));
        // balloon1 DragEnd（保存）。
        assert!(!on_balloon_drag_end(
            &mut world,
            s1_balloon,
            s1_balloon,
            &Phase::Bubble(drag_end_event_at(s1_balloon, (0, 0))),
        ));

        // --- barrier: put が実 FS へ確定 ---------------------------------------------
        parts
            .publisher
            .barrier()
            .expect("barrier should resolve while actor is alive");

        // 実 FS を直接読み、両スコープの WindowPos が保存されていることを中間確認する
        // （実機で欠落した [window.0] がここで存在すべき＝修正前はここで RED）。
        // 保存 x は**原点＝下端中央**基準（左上 ＋ char_w/2）。
        let loaded = load_scope(PersistScope::Ghost, &roots, &FsPersistIo);
        for (scope, cf, cw) in [(0u32, s0_char_final, 434), (1u32, s1_char_final, 400)] {
            assert!(
                loaded.contains(&(
                    PersistKey::WindowPos {
                        scope,
                        axis: Axis::X
                    },
                    (cf.x + cw / 2).to_string()
                )) && loaded.contains(&(
                    PersistKey::WindowPos {
                        scope,
                        axis: Axis::Y
                    },
                    cf.y.to_string()
                )),
                "scope{scope} の char 位置 ({},{}) が実 FS へ保存されていない（実機 [window.{scope}] 欠落再現）: {loaded:?}",
                cf.x,
                cf.y
            );
        }

        // --- restore: mount 解決経由で読み、両スコープの合成 placement を merge ----------
        let entries = load_restored_state(&root, DefaultEncoding::Ansi);
        let synth = |scope: usize, size: SizePx| ScopePlacement {
            scope,
            char_pos: PointPx { x: 100, y: 100 }, // 既定（saved と別位置＝復元優先の証明）
            char_size: size,
            balloon_pos: PointPx { x: 107, y: 107 },
            balloon_size: SizePx { w: 200, h: 300 },
            balloon_offset: PointPx { x: 7, y: 7 },
            anchor: Anchor::Bottom,
        };
        let out = apply_restored_placements(
            vec![synth(0, s0_size), synth(1, s1_size)],
            &entries,
            &snapshot,
        );
        assert_eq!(out.len(), 2);

        // 期待復元値（両スコープとも saved char + balloon 最終確定位置へ戻る）。
        for (p, cf, bf, size) in [
            (&out[0], s0_char_final, s0_balloon_final, s0_size),
            (&out[1], s1_char_final, s1_balloon_final, s1_size),
        ] {
            let cf_px = PointPx { x: cf.x, y: cf.y };
            let bf_px = PointPx { x: bf.x, y: bf.y };
            assert_eq!(
                p.char_pos, cf_px,
                "復元 char_pos が DragEnd 確定位置と値等価でない（scope{}）",
                p.scope
            );
            assert_ne!(
                p.char_pos,
                PointPx { x: 100, y: 100 },
                "復元 char_pos が既定へ落ちている（scope{} の window 保存欠落）",
                p.scope
            );
            // balloon 往復健全性（純関数側は 8.5 で証明済み・ここは結線の確認）。
            let offset_tl = PointPx {
                x: bf.x - cf.x,
                y: bf.y - cf.y,
            };
            let expected_persist = balloon_offset_to_persist(Anchor::Bottom, offset_tl, size);
            assert_eq!(
                balloon_offset_from_persist(Anchor::Bottom, expected_persist, size),
                offset_tl,
                "檻の前提: 下端基準⇄左上基準が現 char_size で往復恒等（scope{}）",
                p.scope
            );
            assert_eq!(
                p.balloon_pos, bf_px,
                "復元 balloon_pos が balloon DragEnd 最終確定位置と値等価でない（scope{}）",
                p.scope
            );
        }

        parts.publisher.close();
        let _ = parts.handle.join();
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
                Anchored(Anchor::Bottom),
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

        assert!(enqueue_window_set_pos(
            &mut world, window, 1531, 883, None, None
        ));
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
            None,
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
            None,
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

    use super::{PlacementRoute, resize_window_to};

    /// #1 一度書き＋re-snap（Req1.1/1.3/1.7/2.1）: `Anchored(Bottom)` の char 窓を
    /// 新寸へ resize すると、`WindowPos.size` が新寸・`position.y` が `wa.bottom − h'`
    /// へ更新され `true`。**原点＝下端中央**ゆえ x は「中央を保つ」よう付け替わる
    /// （伺かの立ち絵は足元中央が接地点＝寸法が変わっても原点は動かない）。
    /// 下端・寸法とも 96 非倍数で dpi/96 再スケール混入の檻。
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

        // 新寸 (517×823・いずれも 96 非倍数): Y=1043−823=220。
        // X は下端中央保持: 旧中央 731+434/2=948 → 新 x = 948−517/2 = 690。
        assert!(resize_window_to(
            &mut world,
            window,
            SizePx { w: 517, h: 823 },
            PlacementRoute::Resnap
        ));
        assert_eq!(
            position_of(&world, window),
            Point {
                x: 690,
                y: 1043 - 823
            },
            "下端中央保持（旧中央 948 を維持）・Y=wa.bottom−h'（bottom 再計算）"
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
        assert!(!resize_window_to(
            &mut world,
            window,
            SizePx { w: 434, h: 687 },
            PlacementRoute::Resnap
        ));
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
                !resize_window_to(&mut world, window, bad, PlacementRoute::Resnap),
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

        assert!(!resize_window_to(
            &mut world,
            window,
            SizePx { w: 517, h: 823 },
            PlacementRoute::Resnap
        ));
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

        assert!(!resize_window_to(
            &mut world,
            window,
            SizePx { w: 517, h: 823 },
            PlacementRoute::Resnap
        ));
        assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
        assert_eq!(size_of(&world, window), SizeI::new(434, 687));
    }

    /// #1 随伴バルーン維持（Req2.6）＋**原点（下端中央）基準の相対位置不変**:
    /// `BalloonFollow` 付き Bottom char 窓を resize すると、バルーンは「キャラの
    /// 下端中央からの相対位置」を保ったまま随伴する（左上基準 offset は原点移動ぶん
    /// 補正される）。伺かの立ち絵は足元中央が接地点＝寸法が変わっても原点は動かないので、
    /// バルーンも引きずられてはならない（実機回帰: むらさきが surface0 434x687 →
    /// surface1000 382x547 でバルーンが 140px 引きずられた欠陥の恒久檻）。
    #[test]
    fn resize_window_to_keeps_balloon_relative_to_bottom_center_origin() {
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

        // 旧原点（下端中央）: x=731+434/2=948・y=356+687=1043。
        // バルーンの旧絶対位置: (731−412, 356−25)=(319, 331)。
        // 旧原点からの相対: (319−948, 331−1043)=(-629, -712)。
        let old_origin = (731 + 434 / 2, 356 + 687);
        let old_balloon = (731 + offset.x, 356 + offset.y);
        let rel_to_origin = (old_balloon.0 - old_origin.0, old_balloon.1 - old_origin.1);

        // 新寸 (517×823): char は下端中央保持で x=948−517/2=690・y=1043−823=220。
        assert!(resize_window_to(
            &mut world,
            window,
            SizePx { w: 517, h: 823 },
            PlacementRoute::Resnap
        ));
        let char_pos = position_of(&world, window);
        let balloon_pos = position_of(&world, balloon);
        assert_eq!(char_pos, Point { x: 690, y: 1043 - 823 });

        // 新原点（下端中央）= (690+517/2, 220+823) = (948, 1043)＝**旧原点と同一**。
        let new_origin = (char_pos.x + 517 / 2, char_pos.y + 823);
        assert_eq!(
            new_origin, old_origin,
            "原点（下端中央）は寸法変動で動かない"
        );
        // バルーンは原点からの相対位置を保つ＝絶対位置も不変。
        assert_eq!(
            (balloon_pos.x - new_origin.0, balloon_pos.y - new_origin.1),
            rel_to_origin,
            "バルーンは下端中央原点からの相対位置を保つ（引きずられない）"
        );
        assert_eq!(
            balloon_pos,
            Point {
                x: old_balloon.0,
                y: old_balloon.1
            },
            "原点が動かない以上、バルーンの絶対位置も動かない"
        );
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
        assert!(resize_window_to(
            &mut world,
            window,
            SizePx { w: 517, h: 823 },
            PlacementRoute::Resnap
        ));
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
        assert!(resize_window_to(
            &mut world,
            window,
            SizePx { w: 517, h: 823 },
            PlacementRoute::Resnap
        ));
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
        assert!(resize_window_to(
            &mut world,
            window,
            SizePx { w: 517, h: 823 },
            PlacementRoute::Resnap
        ));
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
        assert!(resize_window_to(
            &mut world,
            window,
            SizePx { w: 517, h: 823 },
            PlacementRoute::Resnap
        ));
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
        assert!(resize_window_to(
            &mut world,
            window,
            SizePx { w: 517, h: 823 },
            PlacementRoute::Resnap
        ));
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
        assert!(!resize_window_to(
            &mut world,
            window,
            SizePx { w: 517, h: 823 },
            PlacementRoute::Resnap
        ));
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
                !resize_window_to(&mut world, with_handle, bad, PlacementRoute::Resnap),
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
        assert!(!resize_window_to(
            &mut world,
            no_handle,
            SizePx { w: 517, h: 823 },
            PlacementRoute::Resnap
        ));
        assert_eq!(position_of(&world, no_handle), Point { x: 731, y: 500 });
        assert_eq!(size_of(&world, no_handle), SizeI::new(434, 687));
    }

    // -------------------------------------------------------------------------
    // anchor_changed_system（アンカー変化トリガ・task 2.6・Req1.4・
    // design「Anchored（Component）/ anchor_changed_system」「System Flows >
    // アンカー変化トリガ」「File Structure Plan > follow.rs」）
    //
    // producer（seriko の `\![set,alignmenttodesktop]` routing）は本 spec 非所有＝
    // 本群は `Changed<Anchored>` に反応する **consumer** のみを固定し、テストは
    // `Anchored` を直接書き換えて駆動する。change tick を正しく管理するため system は
    // `Schedule` に登録して run し（同一 Schedule インスタンスを使い回すことで
    // 永続 `SystemState` の `last_run` を run 跨ぎで効かせる）、初回 run の全マッチは
    // resize_window_to のべき等 skip で吸収する。全辺 96 非倍数の odd_edge_snapshot
    // （rect(53,37,1877,1043)）で dpi/96 再スケール混入の檻とする。
    // -------------------------------------------------------------------------

    use super::anchor_changed_system;

    /// #1 アンカー変化で再射影（Req1.4 の核）: `Anchored(Bottom)` の釘付け済み char 窓を
    /// spawn し、初回 run はべき等 skip（初回 Changed 付与を resize が同寸・同位置で吸収
    /// ＝位置不変）。次に `Anchored` を Top へ**直接書換**→再 run で「現在の表示寸法の
    /// まま」新アンカー辺（y=wa.top）へ再配置され、X 保持・size 不変（新寸を与えない
    /// ので size は変わらない）。
    #[test]
    fn anchor_changed_system_reprojects_to_new_anchor_edge_at_current_size() {
        let mut world = World::new();
        world.insert_resource(odd_edge_snapshot()); // rect(53, 37, 1877, 1043)
        // Bottom 釘付け済み: y = wa.bottom − h = 1043 − 687 = 356・x=731（96 非倍数）
        let e = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(731, 356, 434, 687),
                Anchored(Anchor::Bottom),
            ))
            .id();

        let mut schedule = Schedule::default();
        schedule.add_systems(anchor_changed_system);

        // 初回 run: 初回 Changed 付与で発火し得るが、Bottom は現寸で y=356 のまま
        // ＝べき等 skip で吸収（位置・寸法不変）。
        schedule.run(&mut world);
        assert_eq!(
            position_of(&world, e),
            Point { x: 731, y: 356 },
            "初回 run はべき等 skip（位置不変）"
        );
        assert_eq!(size_of(&world, e), SizeI::new(434, 687), "初回 run: size 不変");

        // Anchored を Top へ直接書換（producer=seriko の代替＝consumer 駆動の檻）。
        world.get_mut::<Anchored>(e).unwrap().0 = Anchor::Top;

        // 再 run: 現在の表示寸法(434×687)のまま新アンカー辺 y=wa.top=37 へ再射影。
        schedule.run(&mut world);
        assert_eq!(
            position_of(&world, e),
            Point { x: 731, y: 37 },
            "新アンカー辺 y=wa.top へ再配置・X=731 保持（Bottom のままなら y=356 で落ちる）"
        );
        assert_eq!(
            size_of(&world, e),
            SizeI::new(434, 687),
            "現在の表示寸法のまま（新寸を与えないので size は不変）"
        );
    }

    /// #2 Anchored 未変化では発火しない（変更検知の正しさの檻・最重要）: 初回 run で
    /// 初回 Changed を消費した後、`Anchored` を触らずに `WindowPos.position` を故意に
    /// アンカー辺から外して再 run しても**再スナップされない**（system は `Anchored`
    /// 変化にのみ反応し `WindowPos` 変化には反応しない）。毎 run 全マッチ実装
    /// （fresh QueryState の last_run=0）ならここで y=356 へ戻り落ちる。
    #[test]
    fn anchor_changed_system_does_not_fire_when_anchor_unchanged() {
        let mut world = World::new();
        world.insert_resource(odd_edge_snapshot()); // 下端 1043
        let e = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(731, 356, 434, 687), // Bottom 釘付け済み（y=1043−687）
                Anchored(Anchor::Bottom),
            ))
            .id();

        let mut schedule = Schedule::default();
        schedule.add_systems(anchor_changed_system);

        // 初回 run で初回 Changed<Anchored> を消費（べき等 skip・位置不変）。
        schedule.run(&mut world);
        assert_eq!(position_of(&world, e), Point { x: 731, y: 356 });

        // Anchored は触らず、WindowPos.position をアンカー辺から外れた位置へ手動移動。
        world.get_mut::<WindowPos>(e).unwrap().position = Some(Point { x: 731, y: 900 });

        // 再 run: Anchored 未変化ゆえ Changed にマッチせず再スナップしない。
        schedule.run(&mut world);
        assert_eq!(
            position_of(&world, e),
            Point { x: 731, y: 900 },
            "Anchored 未変化では再スナップしない（毎 run 全マッチ実装ならここで y=356 へ戻り落ちる）"
        );
    }

    /// #3 別遷移（Bottom→Left）: `Anchored` を Left へ直接書換すると、現在の表示寸法の
    /// まま左端固定（x=wa.left=53）へ再射影され Y 保持（Top 以外の辺でも配線が
    /// `Anchored.0` を正しく転送していることの補強）。
    #[test]
    fn anchor_changed_system_reprojects_bottom_to_left() {
        let mut world = World::new();
        world.insert_resource(odd_edge_snapshot()); // 左端 53・下端 1043
        let e = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(731, 356, 434, 687), // Bottom 釘付け済み
                Anchored(Anchor::Bottom),
            ))
            .id();

        let mut schedule = Schedule::default();
        schedule.add_systems(anchor_changed_system);
        schedule.run(&mut world); // 初回 Changed 消費（べき等・位置不変）
        assert_eq!(position_of(&world, e), Point { x: 731, y: 356 });

        world.get_mut::<Anchored>(e).unwrap().0 = Anchor::Left;
        schedule.run(&mut world);
        // Left: x=wa.left=53・Y=356 保持・size 不変
        assert_eq!(
            position_of(&world, e),
            Point { x: 53, y: 356 },
            "x=wa.left=53（左端固定）・Y=356 保持"
        );
        assert_eq!(size_of(&world, e), SizeI::new(434, 687));
    }

    // -------------------------------------------------------------------------
    // resize_window_keep_position（balloon 窓の位置維持リサイズ・
    // areka-P0-emo-dpi-scaling task 2.2・R3.1/R4.2・
    // design「areka / placement > follow.rs（additive・balloon 窓の k 追従）」・D8）
    //
    // 「書込ゼロ」の観測境界について: `SetWindowPosCommand` の TLS キューは
    // wintf 私有（`WINDOW_POS_COMMANDS`）で件数を覗く公開 API が無く、`flush()` は
    // 偽 HWND に対し実 `SetWindowPos` を撃ってしまうため使えない（既存
    // enqueue_window_set_pos 群と同じ制約）。代わりに **`Arrangement.offset` 同期**
    // を witness に使う——この同期は `enqueue_window_set_pos` 内で enqueue と
    // 不可分に対で走るため、「stale な sentinel offset が据え置かれたまま」＝
    // 単一ライター経路を一度も通っていない＝enqueue 件数 0 の決定論的証拠になる
    // （逆に通れば offset は必ず `WindowPos.position` の `as f32` 転写になる）。
    // 寸法・座標は 96 の非倍数を使い、隠れた dpi/96 再スケールの檻とする。
    // -------------------------------------------------------------------------

    use super::resize_window_keep_position;

    /// 単一ライター経路を通ったか否かの witness 用 sentinel（実位置と重ならない値）。
    const WRITER_WITNESS: Offset = Offset { x: -1.0, y: -1.0 };

    /// 経路を通っていない＝書込ゼロ（sentinel が据え置かれている）。
    fn assert_no_write(world: &World, entity: Entity) {
        assert_eq!(
            arrangement_offset_of(world, entity),
            WRITER_WITNESS,
            "単一ライター経路を通った痕跡がある（書込ゼロのはず）"
        );
    }

    /// べき等 skip（R4.2・D8「同寸なら書込ゼロで振動しない」）: 現寸と同じ寸を
    /// 渡すと単一ライター経路を**一度も通らず** `false` を返し、位置・寸法とも不変。
    #[test]
    fn resize_window_keep_position_same_size_writes_nothing() {
        let mut world = World::new();
        let window = world
            .spawn((
                fake_handle(0x3000),
                window_pos_sized(731, 356, 434, 687),
                arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y),
            ))
            .id();

        assert!(
            !resize_window_keep_position(&mut world, window, SizePx { w: 434, h: 687 }),
            "同寸はべき等 skip ゆえ false"
        );
        assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
        assert_eq!(size_of(&world, window), SizeI::new(434, 687));
        assert_no_write(&world, window);
    }

    /// 異寸（R3.1/R4.2）: 位置は**現在位置のまま**・寸法だけが新寸へ更新され `true`。
    /// `resize_window_to` と違いアンカー射影 T を再適用しない（balloon は char 窓
    /// 追従で位置が決まるため、DPI 追従では寸だけを差し替える）。
    #[test]
    fn resize_window_keep_position_new_size_keeps_position_and_writes_once() {
        let mut world = World::new();
        let window = world
            .spawn((
                fake_handle(0x3000),
                window_pos_sized(731, 356, 434, 687),
                arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y),
            ))
            .id();

        assert!(resize_window_keep_position(
            &mut world,
            window,
            SizePx { w: 517, h: 823 }
        ));
        assert_eq!(
            position_of(&world, window),
            Point { x: 731, y: 356 },
            "位置は維持される（再射影しない）"
        );
        assert_eq!(size_of(&world, window), SizeI::new(517, 823));
        // 単一ライター経路を通った証拠＝Arrangement.offset が現在位置の as f32 転写
        assert_eq!(
            arrangement_offset_of(&world, window),
            Offset { x: 731.0, y: 356.0 }
        );
    }

    /// 現寸不明（`WindowPos.size` が `None`＝窓生成直後）はべき等判定が成立しない
    /// ため書込へ進む（位置維持・新寸反映）。
    #[test]
    fn resize_window_keep_position_with_unknown_current_size_writes() {
        let mut world = World::new();
        let window = world
            .spawn((
                fake_handle(0x3000),
                window_pos_at(731, 356),
                arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y),
            ))
            .id();

        assert!(resize_window_keep_position(
            &mut world,
            window,
            SizePx { w: 517, h: 823 }
        ));
        assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
        assert_eq!(size_of(&world, window), SizeI::new(517, 823));
    }

    /// `WindowPos` 未付与（窓生成前の異常系）: warn＋`false`＋書込ゼロ
    /// （silent no-op にしない）。
    #[test]
    fn resize_window_keep_position_without_window_pos_returns_false() {
        let mut world = World::new();
        let window = world
            .spawn((
                fake_handle(0x3000),
                arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y),
            ))
            .id();

        assert!(!resize_window_keep_position(
            &mut world,
            window,
            SizePx { w: 517, h: 823 }
        ));
        assert_no_write(&world, window);
    }

    /// `WindowPos.position` 不在（窓生成前）: 現在位置を読めないため warn＋`false`＋
    /// 書込ゼロ。`size` も書き換えない。
    #[test]
    fn resize_window_keep_position_without_position_returns_false() {
        let mut world = World::new();
        let window = world
            .spawn((
                fake_handle(0x3000),
                WindowPos {
                    position: None,
                    size: Some(SizeI::new(434, 687)),
                    ..Default::default()
                },
                arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y),
            ))
            .id();

        assert!(!resize_window_keep_position(
            &mut world,
            window,
            SizePx { w: 517, h: 823 }
        ));
        assert!(
            world
                .get::<WindowPos>(window)
                .expect("WindowPos があるはず")
                .position
                .is_none(),
            "position は復活しない"
        );
        assert_eq!(size_of(&world, window), SizeI::new(434, 687));
        assert_no_write(&world, window);
    }

    /// 非正寸（0・負）: warn＋`false`＋書込ゼロ（`resize_window_to` の非正寸縮退と
    /// 同一流儀・`wa.right−w` 系の暴走を先に弾く）。
    #[test]
    fn resize_window_keep_position_nonpositive_size_holds_state() {
        for bad in [
            SizePx { w: 0, h: 687 },
            SizePx { w: 434, h: 0 },
            SizePx { w: 0, h: 0 },
            SizePx { w: -517, h: 823 },
            SizePx { w: 517, h: -823 },
        ] {
            let mut world = World::new();
            let window = world
                .spawn((
                    fake_handle(0x3000),
                    window_pos_sized(731, 356, 434, 687),
                    arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y),
                ))
                .id();

            assert!(
                !resize_window_keep_position(&mut world, window, bad),
                "非正寸 {bad:?} は false"
            );
            assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
            assert_eq!(size_of(&world, window), SizeI::new(434, 687));
            assert_no_write(&world, window);
        }
    }

    /// `WindowHandle` 未付与（窓生成前）: 判定を二重化せず `enqueue_window_set_pos`
    /// の既存 warn 経路へ委譲し `false`＋状態不変（単一ライター規律の継承）。
    #[test]
    fn resize_window_keep_position_without_handle_returns_false() {
        let mut world = World::new();
        let window = world
            .spawn((
                window_pos_sized(731, 356, 434, 687),
                arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y),
            ))
            .id();

        assert!(!resize_window_keep_position(
            &mut world,
            window,
            SizePx { w: 517, h: 823 }
        ));
        assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
        assert_eq!(size_of(&world, window), SizeI::new(434, 687));
        assert_no_write(&world, window);
    }

    /// despawn 済み（対象不在）でも panic せず `false`。
    #[test]
    fn resize_window_keep_position_on_despawned_entity_returns_false() {
        let mut world = World::new();
        let window = world
            .spawn((fake_handle(0x3000), window_pos_sized(731, 356, 434, 687)))
            .id();
        world.despawn(window);

        assert!(!resize_window_keep_position(
            &mut world,
            window,
            SizePx { w: 517, h: 823 }
        ));
    }

    // -------------------------------------------------------------------------
    // task 3.2: 消費側の存在確認と警告水準の区別（Req 6.2/6.3・design D8 消費側・
    // design「guard_visibility > Implementation Notes > 消費側の区別」）
    //
    // 追従層の消費入口（[`resize_window_to`]／[`resize_window_keep_position`]）は
    // **2 つの事象を混ぜてはならない**:
    //   (a) entity 不在（既に despawn 済み）＝終了処理の正常系 → `debug!` で打ち切り
    //   (b) entity は実在するが接地点規約の component（`Anchored`）が欠落＝真の異常 → `warn!`
    // (a) を warn のままにすると終了時ログが良性ノイズで埋まり（Req 6.2 違反）、(b) を
    // debug へ落とすと本物の結線バグが観測から消える。**同じ檻の中で両方**を見る。
    // -------------------------------------------------------------------------

    /// Req 6.2/6.3（追従層・キャラ窓入口）: despawn 済み entity への resize は正常終了系
    /// として `debug!` 1 行で打ち切られ、**warn 以上を 1 行も出さない**。
    #[test]
    fn resize_window_to_on_despawned_entity_is_debug_only_normal_termination() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot());
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(731, 356, 434, 687),
                Anchored(Anchor::Bottom),
            ))
            .id();
        world.despawn(window);

        let (ok, events) = capture_logs(|| {
            resize_window_to(
                &mut world,
                window,
                SizePx { w: 517, h: 823 },
                PlacementRoute::Resnap,
            )
        });

        assert!(!ok, "破棄済み窓へは書けない（false・panic しない）");
        // `tracing::Level` の Ord は ERROR < WARN < INFO < DEBUG < TRACE ゆえ
        // 「INFO より verbose」＝ debug/trace のみ、が静穏性の表現になる（spawn.rs T-V1 と同型）。
        assert!(
            events.iter().all(|e| e.level > tracing::Level::INFO),
            "破棄済み窓に対して警告以上のログが出ている（Req 6.2 違反）: {events:?}"
        );
        let skipped = expect_one(&events, DESPAWNED_SKIP_TAG);
        assert_eq!(
            skipped.level,
            tracing::Level::DEBUG,
            "破棄済みの打ち切りは debug 水準（正常終了系）"
        );
    }

    /// Req 6.2 の裏面（真の異常を殺さない）: **生存している** entity の接地点規約 component
    /// （`Anchored`）欠落は従来どおり `warn!`。存在確認の導入でこちらまで静穏化してはならない。
    #[test]
    fn resize_window_to_missing_anchored_on_living_entity_still_warns() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot());
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(731, 356, 434, 687),
                // Anchored なし（entity は実在する）
            ))
            .id();

        let (ok, events) = capture_logs(|| {
            resize_window_to(
                &mut world,
                window,
                SizePx { w: 517, h: 823 },
                PlacementRoute::Resnap,
            )
        });

        assert!(!ok, "Anchored 欠落は書かない（false）");
        let warned = expect_one(&events, "Anchored 未付与");
        assert_eq!(
            warned.level,
            tracing::Level::WARN,
            "実在 entity の Anchored 欠落は真の異常＝warn のまま（Req 6.2 の区別）"
        );
        assert!(
            !events.iter().any(|e| e.message().contains(DESPAWNED_SKIP_TAG)),
            "実在 entity を『破棄済み』と誤判定している: {events:?}"
        );
    }

    /// Req 6.2/6.3（追従層・バルーン窓入口）: despawn 済み entity への位置据置きリサイズも
    /// 正常終了系（`debug!`）として打ち切られ、warn 以上を出さない。
    #[test]
    fn resize_window_keep_position_on_despawned_entity_is_debug_only_normal_termination() {
        let mut world = World::new();
        let window = world
            .spawn((fake_handle(0x3000), window_pos_sized(731, 356, 434, 687)))
            .id();
        world.despawn(window);

        let (ok, events) =
            capture_logs(|| resize_window_keep_position(&mut world, window, SizePx { w: 517, h: 823 }));

        assert!(!ok, "破棄済み窓へは書けない（false・panic しない）");
        assert!(
            events.iter().all(|e| e.level > tracing::Level::INFO),
            "破棄済み窓に対して警告以上のログが出ている（Req 6.2 違反）: {events:?}"
        );
        let skipped = expect_one(&events, DESPAWNED_SKIP_TAG);
        assert_eq!(skipped.level, tracing::Level::DEBUG);
    }

    /// Req 6.2 の裏面（バルーン窓入口）: **生存している** entity の `WindowPos` 欠落
    /// （窓生成前の異常系）は従来どおり `warn!`。
    #[test]
    fn resize_window_keep_position_missing_window_pos_on_living_entity_still_warns() {
        let mut world = World::new();
        let window = world.spawn(fake_handle(0x3000)).id(); // WindowPos なし・entity は実在

        let (ok, events) =
            capture_logs(|| resize_window_keep_position(&mut world, window, SizePx { w: 517, h: 823 }));

        assert!(!ok);
        let warned = expect_one(&events, "WindowPos 未付与");
        assert_eq!(
            warned.level,
            tracing::Level::WARN,
            "実在 entity の WindowPos 欠落は真の異常＝warn のまま"
        );
    }

    // -------------------------------------------------------------------------
    // 窓移動レコード（Req 1.2／2.4・task 1.4・design「placement::diag > Invariants」
    // ＋「PlacementRoute 配管＋guard_visibility > Integration」・D11）
    //
    // 単一ライター `enqueue_window_set_pos` の**書込成功時**に 1 レコードを専用 target
    // （`areka::placement::diag`）へ出す。檻の要点:
    //   (1) 経路名が呼出点と 1:1（route を取り違えたら赤）
    //   (2) route・entity・種別・scope・位置・寸・DPI の**全フィールド**が揃う
    //       （entity は wintf 側ログとの結合キーゆえ必ず入る＝Req 1.9 の 2 段 grep 条件）
    //   (3) 書込が起きない経路（べき等 skip・`WindowHandle` 未付与）ではレコードが出ない
    //   (4) 既定 `RUST_LOG=info` では 1 行も出ない（Req 1.7）
    //
    // 観測境界は tracing イベント本体（`test_support::capture_logs`）——本レコードは
    // `WindowPos` ミラーと違い「書込が起きた事実」そのものの証跡だからである。
    // 座標・寸・DPI は 96 の非倍数／非既定値を使い、取り違えを差で炙り出す。
    // -------------------------------------------------------------------------

    use std::sync::{Arc, Mutex};

    use tracing_subscriber::EnvFilter;
    use wintf::ecs::DPI;

    use super::super::diag::{DESPAWNED_SKIP_TAG, WINDOW_MOVE_RECORD_TAG};
    use super::super::spawn::{BalloonWindowMarker, CharWindowMarker};
    use super::super::test_support::{LogEvent, capture_logs, ensure_interest_probes, expect_one};

    /// 捕捉イベントから窓移動レコード行だけを抜く（他の debug ログは無視）。
    fn window_move_lines(events: &[LogEvent]) -> Vec<String> {
        events
            .iter()
            .map(|e| e.message().to_string())
            .filter(|m| m.starts_with(WINDOW_MOVE_RECORD_TAG))
            .collect()
    }

    /// ちょうど 1 行の窓移動レコードを取り出す（0 件・複数件は落とす）。
    fn only_window_move_line(events: &[LogEvent]) -> String {
        let lines = window_move_lines(events);
        assert_eq!(
            lines.len(),
            1,
            "窓移動レコードがちょうど 1 行ではない: {lines:?} / all={events:?}"
        );
        lines.into_iter().next().expect("1 件あることは検査済み")
    }

    /// 釘付け済みキャラ窓（marker/DPI 付き）1 枚だけの World。
    ///
    /// `DPI` は **`WindowHandle` 付与の後**に入れる——wintf の `WindowHandle` on_add フックが
    /// `GetDpiForWindow` を引き（偽 HWND では失敗＝96）`DPI` を上書きするため、同一 spawn の
    /// タプルへ混ぜると意図した DPI が 96 に潰れる（混在 DPI の檻が自己整合で無力化する罠）。
    fn char_window_world(scope: usize, dpi: u16) -> (World, Entity) {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot()); // 下端 1043
        let e = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(731, 356, 434, 687),
                Anchored(Anchor::Bottom),
                CharWindowMarker { scope },
            ))
            .id();
        world.entity_mut(e).insert(DPI::from_dpi(dpi, dpi));
        (world, e)
    }

    /// (2) 全フィールドの檻: 書込成功で**ちょうど 1 行**、route・entity・kind・scope・
    /// 物理位置・物理寸・DPI が揃う（1 つでも落ちたら赤）。
    #[test]
    fn window_move_record_carries_route_entity_kind_scope_position_size_and_dpi() {
        let (mut world, e) = char_window_world(1, 192);

        let (ok, events) = capture_logs(|| {
            resize_window_to(
                &mut world,
                e,
                SizePx { w: 517, h: 823 },
                PlacementRoute::DpiReproject,
            )
        });
        assert!(ok, "前提: 書込は成立する");

        // 期待値は resize_window_to の既存檻と同一の導出（下端中央保持 x=690・Y=1043−823）。
        assert_eq!(
            only_window_move_line(&events),
            format!(
                "[diag.window_move] route=DpiReproject entity={e:?} kind=char scope=1 \
                 x=690 y=220 w=517 h=823 dpi=192"
            )
        );
    }

    /// (2) 結合キーの檻: entity は wintf 側ログ（`entity = ?e`＝`Debug` 表現・scope を
    /// 持たない）と同一表現で出る——Req 1.9 の scope 別計数（2 段 grep）の成立条件。
    #[test]
    fn window_move_record_entity_matches_wintf_debug_rendering() {
        let (mut world, e) = char_window_world(0, 120);

        let (_, events) = capture_logs(|| {
            resize_window_to(
                &mut world,
                e,
                SizePx { w: 517, h: 823 },
                PlacementRoute::Resnap,
            )
        });
        let line = only_window_move_line(&events);
        assert!(
            line.contains(&format!("entity={e:?}")),
            "wintf 側ログと結合できる Debug 表現になっていない: {line}"
        );
        assert!(line.contains("scope=0") && line.contains("kind=char"));
    }

    /// (1) 経路名は**呼出側が渡した route と 1:1**（`resize_window_to` は 3 経路の共通
    /// 反映口ゆえ、ここを取り違えると書き手の名指し＝Req 2.4 が丸ごと嘘になる）。
    #[test]
    fn window_move_record_route_follows_the_argument_of_the_shared_resize_entry() {
        for route in [
            PlacementRoute::AnchorChange,
            PlacementRoute::Resnap,
            PlacementRoute::DpiReproject,
        ] {
            let (mut world, e) = char_window_world(0, 96);
            let (ok, events) =
                capture_logs(|| resize_window_to(&mut world, e, SizePx { w: 517, h: 823 }, route));
            assert!(ok);
            let line = only_window_move_line(&events);
            assert!(
                line.contains(&format!("route={}", route.as_str())),
                "route={route} を渡したのにレコードが一致しない: {line}"
            );
            // 他 8 経路の語が混ざらない（取り違えの檻）。
            for other in PlacementRoute::ALL {
                if other == route {
                    continue;
                }
                assert!(
                    !line.contains(&format!("route={}", other.as_str())),
                    "route={other} が混入: {line}"
                );
            }
        }
    }

    /// (1) 呼出点割当の檻: アンカー変化トリガ（`anchor_changed_system`）は
    /// `AnchorChange` を渡す（system 側の割当ミスを検出する）。
    #[test]
    fn anchor_changed_system_records_the_anchor_change_route() {
        let mut world = World::new();
        world.insert_resource(odd_edge_snapshot()); // rect(53, 37, 1877, 1043)
        let e = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(731, 356, 434, 687),
                Anchored(Anchor::Bottom),
                CharWindowMarker { scope: 1 },
            ))
            .id();
        world.entity_mut(e).insert(DPI::from_dpi(120, 120)); // on_add フックの後に入れる
        let mut schedule = Schedule::default();
        schedule.add_systems(anchor_changed_system);
        // 初回 run はべき等 skip（＝レコードも出ない＝(3) の裏取りも兼ねる）。
        let (_, first) = capture_logs(|| schedule.run(&mut world));
        assert!(
            window_move_lines(&first).is_empty(),
            "べき等 skip でレコードが出た: {first:?}"
        );

        world.get_mut::<Anchored>(e).unwrap().0 = Anchor::Top;
        let (_, second) = capture_logs(|| schedule.run(&mut world));
        let line = only_window_move_line(&second);
        assert!(
            line.contains("route=AnchorChange"),
            "アンカー変化の書込が AnchorChange として記録されない: {line}"
        );
        assert!(line.contains("y=37") && line.contains("dpi=120"), "{line}");
    }

    /// (1) 呼出点割当の檻: バルーン窓の位置据置きリサイズは `KeepPositionResize`。
    /// 種別・scope はバルーン marker から読む（キャラと取り違えない）。
    #[test]
    fn resize_window_keep_position_records_the_keep_position_route() {
        let mut world = World::new();
        let window = world
            .spawn((
                fake_handle(0x3000),
                window_pos_sized(731, 356, 434, 687),
                BalloonWindowMarker { scope: 1 },
            ))
            .id();
        world.entity_mut(window).insert(DPI::from_dpi(192, 192)); // on_add フックの後に入れる

        let (ok, events) = capture_logs(|| {
            resize_window_keep_position(&mut world, window, SizePx { w: 517, h: 823 })
        });
        assert!(ok);
        assert_eq!(
            only_window_move_line(&events),
            format!(
                "[diag.window_move] route=KeepPositionResize entity={window:?} kind=balloon \
                 scope=1 x=731 y=356 w=517 h=823 dpi=192"
            )
        );
    }

    /// (1)(2) `\![move]` cue（[`move_window_to`]）は**対象窓を `MoveCue`**・**随伴バルーンを
    /// `BalloonFollow`** として記録する（D13: スクリプト明示移動は固有の経路語を持つ＝Q3
    /// 「ドラッグ以外の経路での消失」の観測穴を塞ぐ）。移動専用ゆえ寸は番兵（`w=-`／`h=-`）で
    /// 欠落させない（フィールド語彙は経路によらず不変）。
    #[test]
    fn move_cue_write_is_recorded_as_move_cue_with_a_balloon_follow_companion() {
        let mut world = World::new();
        let balloon = world
            .spawn((
                fake_handle(0x2000),
                window_pos_at(180, 383),
                BalloonWindowMarker { scope: 0 },
            ))
            .id();
        // `DPI` 未付与の窓（component 欠落の防御経路）を作る——`WindowHandle` on_add フックが
        // 常に `DPI` を挿すため、番兵 `dpi=-` を単一ライター越しに固定するには外す必要がある。
        world.entity_mut(balloon).remove::<DPI>();
        let char_window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(731, 356, 434, 687),
                CharWindowMarker { scope: 0 },
                BalloonFollow {
                    balloon,
                    offset: PointPx { x: -551, y: 27 },
                },
            ))
            .id();
        // 96 非倍数の DPI を明示付与（on_add フックの後に入れる＝96 へ潰されない）。
        world
            .entity_mut(char_window)
            .insert(DPI::from_dpi(120, 120));

        let (ok, events) = capture_logs(|| move_window_to(&mut world, char_window, 999, 777));
        assert!(ok);
        // 対象窓＝MoveCue／随伴バルーン＝BalloonFollow の 2 行（発行順＝書込順）。
        assert_eq!(
            window_move_lines(&events),
            vec![
                format!(
                    "[diag.window_move] route=MoveCue entity={char_window:?} kind=char scope=0 \
                     x=999 y=777 w=- h=- dpi=120"
                ),
                format!(
                    "[diag.window_move] route=BalloonFollow entity={balloon:?} kind=balloon scope=0 \
                     x=448 y=804 w=- h=- dpi=-"
                ),
            ]
        );
        // 位置自体は従来どおり両方書かれている（挙動不変の裏取り）。
        assert_eq!(position_of(&world, char_window), Point { x: 999, y: 777 });
        assert_eq!(position_of(&world, balloon), Point { x: 448, y: 804 });
    }

    /// (1) ドラッグ経路（連続イベント）はキャラ窓の書込を記録しない一方、随伴バルーンは
    /// `BalloonFollow` として記録される（Req 2.5「バルーン消失は追従の随伴か」の判別材料）。
    #[test]
    fn drag_path_records_only_the_balloon_follow_write() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot()); // 下端 1043
        let balloon = world
            .spawn((
                fake_handle(0x2000),
                window_pos_at(180, 383),
                BalloonWindowMarker { scope: 0 },
            ))
            .id();
        let char_window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(1207, 356, 434, 687),
                Anchored(Anchor::Bottom),
                CharWindowMarker { scope: 0 },
                BalloonFollow {
                    balloon,
                    offset: PointPx { x: -551, y: 27 },
                },
                dragging_state((1207, 356), (1300, 500)),
            ))
            .id();

        let ev = Phase::Bubble(drag_event_at(char_window, (1300, 500), (1450, 520)));
        let (_, events) = capture_logs(|| on_char_drag(&mut world, char_window, char_window, &ev));

        let lines = window_move_lines(&events);
        assert_eq!(
            lines.len(),
            1,
            "ドラッグ 1 イベントの記録は随伴 1 行: {lines:?}"
        );
        assert!(
            lines[0].contains("route=BalloonFollow")
                && lines[0].contains(&format!("entity={balloon:?}")),
            "{lines:?}"
        );
        assert!(
            !lines[0].contains(&format!("entity={char_window:?}")),
            "ドラッグ経路のキャラ窓書込は本 target を通らない（wintf `[drag]` の所有）: {lines:?}"
        );
    }

    /// (3) 書込が起きなければレコードも出ない: べき等 skip（同寸・同位置）と
    /// `WindowHandle` 未付与（失敗）の双方で 0 行。
    #[test]
    fn no_window_move_record_when_nothing_is_written() {
        // べき等 skip（Req3.1）
        let (mut world, e) = char_window_world(0, 120);
        let (wrote, events) = capture_logs(|| {
            resize_window_to(
                &mut world,
                e,
                SizePx { w: 434, h: 687 },
                PlacementRoute::Resnap,
            )
        });
        assert!(!wrote, "前提: 同寸・同位置はべき等 skip");
        assert!(
            window_move_lines(&events).is_empty(),
            "書込ゼロなのにレコードが出た: {events:?}"
        );

        // WindowHandle 未付与（Req3.3・enqueue が warn＋false）
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot());
        let no_handle = world
            .spawn((
                window_pos_sized(731, 356, 434, 687),
                Anchored(Anchor::Bottom),
                CharWindowMarker { scope: 0 },
            ))
            .id();
        let (wrote, events) = capture_logs(|| {
            resize_window_to(
                &mut world,
                no_handle,
                SizePx { w: 517, h: 823 },
                PlacementRoute::Resnap,
            )
        });
        assert!(!wrote);
        assert!(
            window_move_lines(&events).is_empty(),
            "失敗経路でレコードが出た: {events:?}"
        );
    }

    /// 与えた `RUST_LOG` 相当 directive で実際に濾した出力を集める（diag.rs の
    /// `emit_all_under_filter` と同型——こちらは**単一ライター経由**で点灯を確かめる）。
    fn window_move_output_under_filter(directives: &str) -> String {
        ensure_interest_probes();

        #[derive(Clone)]
        struct VecWriter(Arc<Mutex<Vec<u8>>>);
        impl std::io::Write for VecWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0
                    .lock()
                    .expect("捕捉バッファの毒化なし")
                    .extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
        let sink = buf.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::new(directives))
            .with_ansi(false)
            .with_writer(move || VecWriter(sink.clone()))
            .finish();

        let (mut world, e) = char_window_world(1, 192);
        tracing::subscriber::with_default(subscriber, || {
            tracing::callsite::rebuild_interest_cache();
            assert!(resize_window_to(
                &mut world,
                e,
                SizePx { w: 517, h: 823 },
                PlacementRoute::DpiReproject
            ));
        });

        String::from_utf8(buf.lock().expect("捕捉バッファの毒化なし").clone()).expect("UTF-8")
    }

    /// (4) 既定 `RUST_LOG=info`（`main.rs` のフォールバック）では窓移動レコードが
    /// **1 行も出ない**（Req 1.7・恒久計装の既定 OFF）。
    #[test]
    fn window_move_records_are_silent_under_default_info_filter() {
        let out = window_move_output_under_filter("info");
        assert!(
            !out.contains(WINDOW_MOVE_RECORD_TAG),
            "既定 RUST_LOG=info で窓移動レコードが漏れている（Req 1.7 違反）: {out}"
        );
    }

    /// (4) 手順書の directive（`areka::placement::diag=debug`）で点灯する
    /// ＝単一ライター経由でも target が手順書と 1:1 で結ばれている（Req 1.5/1.7）。
    #[test]
    fn window_move_records_light_up_under_the_procedure_directive() {
        let out = window_move_output_under_filter("info,areka::placement::diag=debug");
        assert!(
            out.contains(WINDOW_MOVE_RECORD_TAG) && out.contains("route=DpiReproject"),
            "手順書の RUST_LOG で単一ライターのレコードが点灯しない: {out}"
        );
    }

    // -------------------------------------------------------------------------
    // 遷移ガードの**配線**（task 6.1・S3 是正・Req 3.1/3.2/3.3・D5/D6/D13）
    //
    // task 2.2 は `guard_visibility`／`work_area_for_window_with_origin` を純関数として
    // 用意したが**本番呼出はゼロ**だった（diagnosis-report.md §1.3「純関数が在ることは
    // S3 の充足ではない」）。本節が檻に入れるのは純関数の判定規則ではなく、
    // **`resize_window_to` の中でそれが実際に走るか・どの route で走るか**である。
    //
    // 檻の要点（空虚化を避けるための自己検査を各檻が持つ）:
    //   (1) 探針の自己検査——ガード**無し**の提案が本当に全 work area 非交差であること
    //       （交差する探針では ClampX 腕へ一度も入らず「緑」が何も意味しない・[[2.2 の教訓]]）
    //   (2) 位置の不変条件——clamp 後の矩形がいずれかの work area と交差する（Req 3.1）
    //   (3) route による発火条件——適用外 route（`MoveCue`／`Restore` 等）とドラッグ経路
    //       では**位置が素の射影と 1 bit も違わない**こと。ログ側の否定 assert だけに
    //       依存しない（[[5.2 の教訓＝空虚性 6 例目]]: 不変量がログ側にしか無いと
    //       別ファイルの水準変更で守りが消える）
    //   (4) 判定語のリテラル——手順書 §3.3 の grep 語を檻側にも literal で持つ
    //       （[[5.1 → 7.2 の申し送り]]「判定語に使っているのに檻が無い」型の再発防止）
    //
    // 座標はすべて論理値 × DPI（96/120/192）で構築し、絶対 px の固定値を持たない（Req 5.6）。
    // -------------------------------------------------------------------------

    use super::route_applies_visibility_guard;

    /// 手順書 §3.3 の grep 判定語（**本体の定数とは独立にここへ literal で置く**）。
    const CLAMP_TAG: &str = "[visibility-guard] ClampX";
    /// 同上（最近傍フォールバックの非ドラッグ経路 warn 昇格・Req 3.2）。
    const NEAREST_TAG: &str = "[visibility-guard] NearestFallback";
    /// 同上（work area を解決できず判定不能・Req 3.3）。
    const UNRESOLVED_TAG: &str = "[visibility-guard] WorkAreaUnresolved";
    /// 3 語に共通の接頭辞（「ガードが何かを言った」ことの一括検出）。
    const GUARD_TAG_PREFIX: &str = "[visibility-guard]";

    /// 幅広のキャラ窓寸（論理 320×400）。論理 320／32 はいずれも 8 の倍数ゆえ、
    /// 96/120/192 のどの水準でも物理 px が偶数＝手順 3b の `w/2` が切り捨てで狂わない。
    fn wide_char_size(dpi: i32) -> SizePx {
        SizePx {
            w: px(320, dpi),
            h: px(400, dpi),
        }
    }

    /// 「どの work area にも属さない帯」（`0 ..= px(64)`）より**狭い**新寸。
    fn narrow_char_size(dpi: i32) -> SizePx {
        SizePx {
            w: px(32, dpi),
            h: px(400, dpi),
        }
    }

    /// 帯の中で**右モニタが一意に最近傍になる**中心 x（帯の中点 `px(32)` は左右等距離で
    /// 先勝ちに依存するため使わない）。
    fn gap_center_x(dpi: i32) -> i32 {
        px(40, dpi)
    }

    /// 「旧矩形は可視・新提案は全 work area 非交差」へ落ちるキャラ窓 World を組む。
    ///
    /// 旧寸 [`wide_char_size`] の窓を、下端中央付替え（`resize_window_to` 手順 3b）後の
    /// 中心が帯へ落ちる位置に置く。新寸 [`narrow_char_size`] は帯より狭いので、射影 T が
    /// 出す提案矩形は帯へ収まり **どの work area とも交差しない**——S3 が言う
    /// 「非ドラッグ要因で不可視へ遷移する」状態そのものを合成する。
    fn gap_bound_char_world(dpi: i32) -> (World, Entity, PointPx) {
        let old = wide_char_size(dpi);
        let old_pos = PointPx {
            x: gap_center_x(dpi) - old.w / 2,
            y: left_wa().bottom - old.h,
        };
        let mut world = World::new();
        world.insert_resource(mixed_layout(dpi));
        let e = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(old_pos.x, old_pos.y, old.w, old.h),
                Anchored(Anchor::Bottom),
            ))
            .id();
        (world, e, old_pos)
    }

    /// ガードを通さない**素の**射影結果（＝本タスク以前の挙動）。手順 3b と
    /// [`project_anchor`] を檻側で独立に再現し、本体の実装を呼び直さない。
    fn unguarded_projection(dpi: i32, old_pos: PointPx, new: SizePx) -> PointPx {
        let old = wide_char_size(dpi);
        let raw = PointPx {
            x: old_pos.x + old.w / 2 - new.w / 2,
            y: old_pos.y,
        };
        project_anchor(Anchor::Bottom, raw, new, Some(&mixed_layout(dpi)))
    }

    /// 窓矩形がいずれかの work area と交差するか（檻側の独立実装 [`overlaps`] で判定）。
    fn visible_in(layout: &MonitorSnapshot, pos: PointPx, size: SizePx) -> bool {
        layout
            .work_areas
            .iter()
            .any(|wa| overlaps(win(pos, size), *wa))
    }

    /// 現在位置を [`PointPx`] で読む（檻の比較単位を射影の単位へ揃える）。
    fn point_of(world: &World, entity: Entity) -> PointPx {
        let p = position_of(world, entity);
        PointPx { x: p.x, y: p.y }
    }

    /// `[visibility-guard]` を名乗るイベントだけを抜く。
    fn guard_events<'a>(events: &'a [LogEvent], needle: &str) -> Vec<&'a LogEvent> {
        events
            .iter()
            .filter(|e| e.message().contains(needle))
            .collect()
    }

    /// 発火条件の**表そのもの**を固定する（D13 帰結⑴⑵）。挙動側の檻（下 2 件）と
    /// 二段構えにしてあるのは、語彙が 9 種あるのに `resize_window_to` を実際に通るのは
    /// 現状 4 種だけで、残り 5 種の判定が挙動檻だけでは**合成でしか**検査できないため。
    /// [`PlacementRoute::ALL`] を回すので、語彙が増えたら本檻も落ちる。
    #[test]
    fn visibility_guard_route_table_matches_the_d13_decision() {
        for route in PlacementRoute::ALL {
            let expected = matches!(
                route,
                PlacementRoute::AnchorChange
                    | PlacementRoute::Resnap
                    | PlacementRoute::DpiReproject
                    | PlacementRoute::ReportedSizeReconcile
            );
            assert_eq!(
                route_applies_visibility_guard(route),
                expected,
                "route={route} の発火判定が D13 帰結⑴⑵ と食い違う"
            );
        }
        // 表が「全部真」「全部偽」へ潰れていないこと（自明な述語への退化の検出）。
        let fired = PlacementRoute::ALL
            .into_iter()
            .filter(|r| route_applies_visibility_guard(*r))
            .count();
        assert_eq!(fired, 4, "発火 route が 4 種でない（表が潰れている）");
    }

    /// **Req 3.1 の本体**: 非ドラッグの配置系 4 経路（D13 帰結⑴）では、全 work area
    /// 非交差への遷移が X の clamp で阻止され、`warn!` が 1 行残る。
    ///
    /// Y は射影 T の所有ゆえ 1 bit も動かない（[`guard_visibility`] の事後条件）。
    #[test]
    fn visibility_guard_clamps_x_on_non_drag_placement_routes() {
        for dpi in DPIS {
            let layout = mixed_layout(dpi);
            let new = narrow_char_size(dpi);
            for route in [
                PlacementRoute::AnchorChange,
                PlacementRoute::Resnap,
                PlacementRoute::DpiReproject,
                PlacementRoute::ReportedSizeReconcile,
            ] {
                let (mut world, e, old_pos) = gap_bound_char_world(dpi);

                // (1) 探針の自己検査: 素の射影は**本当に**不可視へ落ちる／旧矩形は可視。
                //     どちらかが崩れると ClampX 腕に入らず、この檻は空虚になる。
                let bare = unguarded_projection(dpi, old_pos, new);
                assert!(
                    !visible_in(&layout, bare, new),
                    "dpi={dpi}: 探針が不動点——ガード無しの提案 {bare:?} が既に可視で ClampX 腕へ入らない"
                );
                assert!(
                    visible_in(&layout, old_pos, wide_char_size(dpi)),
                    "dpi={dpi}: 旧矩形が非交差では『遷移』でなく留置＝Keep が正解になってしまう"
                );

                let (ok, events) = capture_logs(|| resize_window_to(&mut world, e, new, route));
                assert!(ok, "dpi={dpi} route={route}: 書込は成立する前提");

                // (2) 位置の不変条件（Req 3.1）: 書かれた矩形はどこかの work area と交差する。
                let pos = point_of(&world, e);
                assert!(
                    visible_in(&layout, pos, new),
                    "dpi={dpi} route={route}: Req 3.1 違反——{pos:?} は全 work area と非交差"
                );
                assert_eq!(
                    pos.y, bare.y,
                    "dpi={dpi} route={route}: Y は射影 T の所有＝ガードが触ってはならない"
                );
                assert_ne!(
                    pos.x, bare.x,
                    "dpi={dpi} route={route}: X が引き戻されていない（ガード未発火）"
                );
                // clamp 先は射影が Y に用いた work area（右モニタ）の水平範囲内。
                let wa = right_wa(dpi);
                assert!(
                    wa.left <= pos.x && pos.x <= wa.right - new.w,
                    "dpi={dpi} route={route}: clamp 先が射影の work area {wa:?} の外: {pos:?}"
                );

                // (4) 判定語: ClampX の warn が 1 行・水準は WARN（Req 3.1/3.2 の観測）。
                let clamped = expect_one(&events, CLAMP_TAG);
                assert_eq!(
                    clamped.level,
                    tracing::Level::WARN,
                    "dpi={dpi} route={route}: clamp の記録が warn 水準でない"
                );
            }
        }
    }

    /// **Req 3.1 の裏面（D13 帰結⑵）**: 明示操作系・非配置系の route では、位置が素の
    /// 射影と 1 bit も違わず、ガードのログも 1 行も出ない。
    ///
    /// `MoveCue`（`\![move]`）と `Restore`（位置復元）を引き戻すのは、スクリプト／
    /// 永続化が決めた位置の否定であり本 spec の Out of scope である。**ここが緑のまま
    /// 「常に発火」へ変異させられると S3 是正が明示操作の尊重を壊す**ため、位置側の
    /// assert（ログではなく挙動）を第一の守りに置く。
    #[test]
    fn visibility_guard_does_not_fire_on_explicit_or_non_placement_routes() {
        for dpi in DPIS {
            let layout = mixed_layout(dpi);
            let new = narrow_char_size(dpi);
            for route in [
                PlacementRoute::SpawnInitial,
                PlacementRoute::Restore,
                PlacementRoute::KeepPositionResize,
                PlacementRoute::BalloonFollow,
                PlacementRoute::MoveCue,
            ] {
                let (mut world, e, old_pos) = gap_bound_char_world(dpi);
                let bare = unguarded_projection(dpi, old_pos, new);
                // 探針の自己検査: ガードが**発火する条件は揃っている**（route だけが違う）。
                assert!(
                    !visible_in(&layout, bare, new),
                    "dpi={dpi}: 探針が不動点——発火条件が揃っていない"
                );

                let (ok, events) = capture_logs(|| resize_window_to(&mut world, e, new, route));
                assert!(ok, "dpi={dpi} route={route}: 書込は成立する前提");

                assert_eq!(
                    point_of(&world, e),
                    bare,
                    "dpi={dpi} route={route}: 適用外 route で位置が動いた（明示操作の尊重が壊れている）"
                );
                assert!(
                    guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
                    "dpi={dpi} route={route}: 適用外 route でガードが喋っている: {events:?}"
                );
            }
        }
    }

    /// **ドラッグ経路は従来の水準のまま**（Req 3.3 の水準分岐・D5）: ユーザーが自分で
    /// 帯へ運んだ窓は引き戻されず、毎イベント発火する経路に `warn!` を増やさない。
    #[test]
    fn drag_path_neither_clamps_nor_warns_when_leaving_every_work_area() {
        for dpi in DPIS {
            let layout = mixed_layout(dpi);
            let size = narrow_char_size(dpi);
            // 開始位置は右モニタ上（可視）・接地済み。
            let start_pos = PointPx {
                x: px(200, dpi),
                y: right_wa(dpi).bottom - size.h,
            };
            assert!(
                visible_in(&layout, start_pos, size),
                "dpi={dpi}: 前提——ドラッグ開始位置は可視"
            );

            let mut world = World::new();
            world.insert_resource(mixed_layout(dpi));
            let cursor = (px(800, dpi), px(400, dpi));
            let window = world
                .spawn((
                    fake_handle(0x1000),
                    window_pos_sized(start_pos.x, start_pos.y, size.w, size.h),
                    Anchored(Anchor::Bottom),
                    dragging_state((start_pos.x, start_pos.y), cursor),
                ))
                .id();

            // カーソルを帯へ運ぶ: 生ドラッグ x = px(24) ＝ 帯の内側。
            let moved = (cursor.0 - (px(200, dpi) - px(24, dpi)), cursor.1);
            let ev = Phase::Bubble(drag_event_at(window, cursor, moved));
            let (consumed, events) = capture_logs(|| on_char_drag(&mut world, window, window, &ev));
            assert!(!consumed);

            let pos = point_of(&world, window);
            // 自己検査: ドラッグは**実際に**窓を全 work area の外へ運んだ（＝ガードが
            // 配線されていれば必ず clamp する状況である）。
            assert!(
                !visible_in(&layout, pos, size),
                "dpi={dpi}: 探針が不動点——ドラッグ先が可視のままでは『引き戻さない』を検査していない"
            );
            assert_eq!(
                pos.x,
                px(24, dpi),
                "dpi={dpi}: ドラッグの X は素通し（明示操作の尊重）"
            );
            assert!(
                guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
                "dpi={dpi}: ドラッグ経路でガードが喋っている（spam・水準分岐の破壊）: {events:?}"
            );
        }
    }

    /// **Req 3.2**: 最近傍フォールバック（窓中心がどのモニタにも属さない＝モニタ構成
    /// 情報と実画面の食い違いの兆候）は、非ドラッグ経路で `warn!` へ昇格する。
    ///
    /// この探針は **clamp を伴わない**（提案矩形は work area と交差したまま）——
    /// `NearestFallback` の観測が `ClampX` の副産物ではなく独立に成立することを示す。
    #[test]
    fn nearest_fallback_warns_on_non_drag_route_even_without_clamping() {
        for dpi in DPIS {
            let layout = mixed_layout(dpi);
            let old = wide_char_size(dpi);
            // 幅は据置き・高さだけ変える＝手順 3b で x は動かず、中心は帯に留まる。
            let new = SizePx {
                w: old.w,
                h: px(200, dpi),
            };
            let (mut world, e, old_pos) = gap_bound_char_world(dpi);

            // 探針の自己検査: **決めた位置**の work area 解決が本当に最近傍へ落ちる
            // （`Contains` なら昇格の腕へ入らず空虚になる）。かつ提案矩形は交差したまま
            // ＝clamp しない（`NearestFallback` が `ClampX` の副産物でないことの担保）。
            let bare = unguarded_projection(dpi, old_pos, new);
            let (_, resolution) = work_area_for_window_with_origin(&layout, win(bare, new))
                .expect("合成レイアウトは空でない");
            assert_eq!(
                resolution,
                WorkAreaResolution::NearestFallback,
                "dpi={dpi}: 探針が `Contains` に落ちている＝昇格の腕を検査していない"
            );
            assert!(
                visible_in(&layout, bare, new),
                "dpi={dpi}: 探針が clamp を伴っている＝`NearestFallback` 単独の檻になっていない"
            );

            let (ok, events) =
                capture_logs(|| resize_window_to(&mut world, e, new, PlacementRoute::Resnap));
            assert!(ok);
            assert_eq!(
                point_of(&world, e),
                bare,
                "dpi={dpi}: Keep 腕で位置が動いた"
            );

            let warned = expect_one(&events, NEAREST_TAG);
            assert_eq!(
                warned.level,
                tracing::Level::WARN,
                "dpi={dpi}: 最近傍フォールバックが非ドラッグ経路で warn へ昇格していない"
            );
            assert!(
                guard_events(&events, CLAMP_TAG).is_empty(),
                "dpi={dpi}: clamp していないのに ClampX が出ている: {events:?}"
            );
        }
    }

    /// **Req 3.3**: 位置決定に必要な入力（モニタ work area）が取得できない場合は、
    /// 位置を変更せず現状のまま `warn!` を残す（架空の可視領域を発明しない）。
    ///
    /// `MonitorSnapshot` 不在／空 snapshot のいずれでも、射影 T は identity へ縮退
    /// 済みである＝ガードが位置へ手を入れないことが「現状維持」の内容になる。
    #[test]
    fn missing_work_area_holds_position_and_warns_on_non_drag_route() {
        for dpi in DPIS {
            for (label, snapshot) in [
                ("resource 不在", None),
                ("空 snapshot", Some(MonitorSnapshot { work_areas: vec![] })),
            ] {
                let new = narrow_char_size(dpi);
                let (mut world, e, old_pos) = gap_bound_char_world(dpi);
                world.remove_resource::<MonitorSnapshot>();
                if let Some(s) = snapshot {
                    world.insert_resource(s);
                }
                // work area が無いときの射影は identity ＝ 手順 3b 後の raw そのもの。
                let old = wide_char_size(dpi);
                let identity = PointPx {
                    x: old_pos.x + old.w / 2 - new.w / 2,
                    y: old_pos.y,
                };

                let (ok, events) =
                    capture_logs(|| resize_window_to(&mut world, e, new, PlacementRoute::Resnap));
                assert!(ok, "dpi={dpi} {label}: 寸の反映自体は従来どおり成立する");
                assert_eq!(
                    point_of(&world, e),
                    identity,
                    "dpi={dpi} {label}: ガードが位置を動かした（現状維持の違反）"
                );

                let warned = expect_one(&events, UNRESOLVED_TAG);
                assert_eq!(
                    warned.level,
                    tracing::Level::WARN,
                    "dpi={dpi} {label}: 入力欠落が warn として残っていない（Req 3.3）"
                );
                assert!(
                    guard_events(&events, CLAMP_TAG).is_empty(),
                    "dpi={dpi} {label}: work area 不明なのに clamp している: {events:?}"
                );
            }
        }
    }

    /// 適用外 route では、work area 不明であってもガードは 1 行も喋らない
    /// （警告の出所が route 条件の**内側**にあることの檻）。
    #[test]
    fn missing_work_area_stays_silent_on_guard_exempt_routes() {
        for dpi in DPIS {
            let (mut world, e, _) = gap_bound_char_world(dpi);
            world.remove_resource::<MonitorSnapshot>();
            let (_, events) = capture_logs(|| {
                resize_window_to(
                    &mut world,
                    e,
                    narrow_char_size(dpi),
                    PlacementRoute::MoveCue,
                )
            });
            assert!(
                guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
                "dpi={dpi}: 適用外 route でガードが喋っている: {events:?}"
            );
        }
    }

    /// **旧矩形『不明』は `Option::None` だけではない**（[[4.6 の教訓]]）: wintf の
    /// [`WindowPos::default`] は寸を `Some(CW_USEDEFAULT)`（＝`i32::MIN` センチネル）で
    /// 持つ。これを素の矩形として交差判定へ入れると退化矩形が「もともと画面外に
    /// 留置されていた」と誤判定され、**安全側 clamp の腕が丸ごと死ぬ**。
    #[test]
    fn undetermined_old_size_is_treated_as_unknown_rect_and_clamps() {
        for dpi in DPIS {
            let layout = mixed_layout(dpi);
            let new = narrow_char_size(dpi);
            // 手順 3b は旧寸が非正のとき付替えを行わない＝raw は現在位置そのもの。
            let raw = PointPx {
                x: gap_center_x(dpi) - new.w / 2,
                y: left_wa().bottom - new.h,
            };
            let mut world = World::new();
            world.insert_resource(mixed_layout(dpi));
            let e = world
                .spawn((
                    fake_handle(0x1000),
                    // 寸は `CW_USEDEFAULT` センチネルのまま（窓生成直後の実表現）。
                    WindowPos {
                        position: Some(Point { x: raw.x, y: raw.y }),
                        ..Default::default()
                    },
                    Anchored(Anchor::Bottom),
                ))
                .id();

            // 探針の自己検査: 素の射影は不可視へ落ちる（＝安全側 clamp が要る状況）。
            let bare = project_anchor(Anchor::Bottom, raw, new, Some(&layout));
            assert!(
                !visible_in(&layout, bare, new),
                "dpi={dpi}: 探針が不動点——素の射影が既に可視"
            );

            let (ok, events) =
                capture_logs(|| resize_window_to(&mut world, e, new, PlacementRoute::Resnap));
            assert!(ok);
            assert!(
                visible_in(&layout, point_of(&world, e), new),
                "dpi={dpi}: 寸未確定（センチネル）を『留置』と誤読して clamp を見送っている"
            );
            expect_one(&events, CLAMP_TAG);
        }
    }

    // -------------------------------------------------------------------------
    // バルーン矩形への遷移ガード配線（task 6.2・S3′ 是正・Req 3.4・D6）
    //
    // task 2.2 は `guard_visibility` のバルーン矩形ケース（純関数）を、task 6.1 は
    // キャラ窓経路の配線を固めた。本節が檻に入れるのは**バルーン随伴で実際に走るか・
    // どの引き金で走るか**である（diagnosis-report.md §1.4「純関数が在ることは S3′ の
    // 充足ではない」）。
    //
    // 檻の要点（空虚化を避けるための自己検査を各檻が持つ）:
    //   (1) 探針の自己検査——ガード**無し**のバルーン提案が本当に全 work area 非交差で
    //       あること／旧バルーン矩形は可視であること（どちらかが崩れると ClampX 腕へ
    //       入らず「緑」が何も意味しない・[[2.2 の教訓]]）
    //   (2) **キャラ窓は clamp されない**こと——キャラ側のガードが動かした結果を
    //       バルーンの成果と読み違えない（S3 と S3′ の分離）
    //   (3) 引き金による発火条件——**ドラッグ随伴では位置が素の恒等式と 1 bit も違わない**。
    //       ログ側の否定 assert だけに依存しない（[[5.2 の教訓＝空虚性 6 例目]]:
    //       不変量がログ側にしか無いと別ファイルの水準変更で守りが消える）
    //   (4) 判定語のリテラル——`CLAMP_TAG`／`NEAREST_TAG`／`UNRESOLVED_TAG` を檻側にも持つ
    //
    // 座標はすべて論理値 × DPI（96/120/192）で構築し、絶対 px の固定値を持たない（Req 5.6）。
    // -------------------------------------------------------------------------

    use super::BalloonFollowTrigger;

    /// キャラ窓の初期位置（**接地していない** Y）。同寸の [`resize_window_to`] でも
    /// 射影 T が Y を `wa.bottom − h` へ動かす＝手順 4 のべき等 skip に落ちない。
    fn char_start_pos(dpi: i32) -> PointPx {
        point(px(1500, dpi), px(100, dpi))
    }

    /// 射影 T 適用後のキャラ窓確定位置（右モニタへ接地・**可視のまま**）。
    fn char_settled_pos(dpi: i32) -> PointPx {
        point(px(1500, dpi), grounded_y(right_wa(dpi), char_size(dpi)))
    }

    /// 全 work area の外を指す追従 offset（キャラの右上へ px(500)／−px(400)）。
    ///
    /// キャラ窓（右端 `px(1800)`）は右モニタ内に留まる一方、バルーン（幅 `px(500)`）は
    /// `px(2000)` 以降＝`right_wa.right = px(1920)` の外側へ丸ごと出る。左モニタは負座標
    /// ゆえ交差し得ない＝**バルーンだけが完全不可視**になる S3′ の合成そのもの。
    fn far_out_offset(dpi: i32) -> PointPx {
        point(px(500, dpi), -px(400, dpi))
    }

    /// 旧バルーン位置（右モニタ内＝**可視**。ゆえに「可視→不可視の遷移」になる）。
    fn visible_balloon_pos(dpi: i32) -> PointPx {
        point(px(800, dpi), px(240, dpi))
    }

    /// 「キャラ窓は可視のまま・offset 恒等式の提案位置だけが全 work area 非交差」へ
    /// 落ちる合成 World を組む（S3′＝*キャラは見えているのに会話が読めない*）。
    fn char_with_far_balloon_world(
        dpi: i32,
        balloon_pos: PointPx,
        offset: PointPx,
    ) -> (World, Entity, Entity) {
        let c = char_size(dpi);
        let b = balloon_size(dpi);
        let start = char_start_pos(dpi);
        let mut world = World::new();
        world.insert_resource(mixed_layout(dpi));
        let balloon = world
            .spawn((
                fake_handle(0x2000),
                window_pos_sized(balloon_pos.x, balloon_pos.y, b.w, b.h),
            ))
            .id();
        let char_window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(start.x, start.y, c.w, c.h),
                Anchored(Anchor::Bottom),
                BalloonFollow { balloon, offset },
            ))
            .id();
        (world, char_window, balloon)
    }

    /// 引き金の表（D13 帰結⑴⑵ の**キャラ窓と同一の表**）を固定する。
    ///
    /// バルーンは別規則を持たない——違うのは「何を入力に引くか」だけで、引くのは
    /// キャラ窓と同じ [`route_applies_visibility_guard`] である。ドラッグ腕が真へ倒れる
    /// 変異（＝明示操作の尊重の破壊）は挙動檻
    /// [`balloon_drag_trigger_neither_clamps_nor_warns`] が第一の守りとして捕まえる。
    #[test]
    fn balloon_follow_trigger_table_mirrors_the_char_window_table() {
        assert!(
            !BalloonFollowTrigger::Drag.applies_visibility_guard(),
            "ドラッグ随伴でガードが発火する（明示操作の尊重が壊れている・Req 3.1）"
        );
        for route in PlacementRoute::ALL {
            assert_eq!(
                BalloonFollowTrigger::Placement(route).applies_visibility_guard(),
                route_applies_visibility_guard(route),
                "route={route} の引き金判定がキャラ窓の表と食い違う"
            );
        }
        // 表が「全部真」「全部偽」へ潰れていないこと（自明な述語への退化の検出）。
        let fired = PlacementRoute::ALL
            .into_iter()
            .filter(|r| BalloonFollowTrigger::Placement(*r).applies_visibility_guard())
            .count();
        assert_eq!(fired, 4, "発火する引き金が 4 種でない（表が潰れている）");
    }

    /// **Req 3.4 の本体**: 非ドラッグの配置系 4 経路が引き金のとき、offset 恒等式が出した
    /// バルーン提案位置が全 work area 非交差へ落ちるなら、X の clamp で救われる。
    ///
    /// キャラ窓は終始可視（clamp されない）＝救われたのは**バルーンだけ**である。
    #[test]
    fn balloon_visibility_guard_clamps_x_on_non_drag_placement_triggers() {
        for dpi in DPIS {
            let layout = mixed_layout(dpi);
            let b_size = balloon_size(dpi);
            let offset = far_out_offset(dpi);
            let old_pos = visible_balloon_pos(dpi);
            for route in [
                PlacementRoute::AnchorChange,
                PlacementRoute::Resnap,
                PlacementRoute::DpiReproject,
                PlacementRoute::ReportedSizeReconcile,
            ] {
                let (mut world, char_window, balloon) =
                    char_with_far_balloon_world(dpi, old_pos, offset);

                // (1) 探針の自己検査: 恒等式の素の提案は**本当に**全 work area 非交差／
                //     旧バルーン矩形は可視。どちらかが崩れると ClampX 腕へ入らず空虚になる。
                let settled = char_settled_pos(dpi);
                let bare = point(settled.x + offset.x, settled.y + offset.y);
                assert!(
                    !visible_in(&layout, bare, b_size),
                    "dpi={dpi}: 探針が不動点——素のバルーン提案 {bare:?} が既に可視"
                );
                assert!(
                    visible_in(&layout, old_pos, b_size),
                    "dpi={dpi}: 旧バルーンが非交差では『遷移』でなく留置＝Keep が正解になる"
                );

                let (ok, events) = capture_logs(|| {
                    resize_window_to(&mut world, char_window, char_size(dpi), route)
                });
                assert!(ok, "dpi={dpi} route={route}: 書込は成立する前提");

                // (2) キャラ窓は clamp されていない＝救われたのはバルーンだけである。
                assert_eq!(
                    point_of(&world, char_window),
                    settled,
                    "dpi={dpi} route={route}: キャラ窓が動いた＝S3′ ではなく S3 の檻になっている"
                );

                // Req 3.4: 書かれたバルーン矩形はいずれかの work area と交差する。
                let pos = point_of(&world, balloon);
                assert!(
                    visible_in(&layout, pos, b_size),
                    "dpi={dpi} route={route}: Req 3.4 違反——バルーン {pos:?} が全 work area と非交差"
                );
                assert_eq!(
                    pos.y, bare.y,
                    "dpi={dpi} route={route}: バルーンの Y は恒等式の所有＝ガードが触ってはならない"
                );
                assert_ne!(
                    pos.x, bare.x,
                    "dpi={dpi} route={route}: バルーンの X が引き戻されていない（ガード未発火）"
                );
                let wa = right_wa(dpi);
                assert!(
                    wa.left <= pos.x && pos.x <= wa.right - b_size.w,
                    "dpi={dpi} route={route}: clamp 先が work area {wa:?} の外: {pos:?}"
                );

                // (4) 判定語: ClampX の warn が 1 行・水準は WARN（縮退シームの記録）。
                let clamped = expect_one(&events, CLAMP_TAG);
                assert_eq!(
                    clamped.level,
                    tracing::Level::WARN,
                    "dpi={dpi} route={route}: バルーンの clamp が warn 水準でない"
                );
                // 提案位置の中心はどの work area にも属さない＝食い違いの兆候も 1 行残る。
                assert_eq!(
                    expect_one(&events, NEAREST_TAG).level,
                    tracing::Level::WARN,
                    "dpi={dpi} route={route}: 最近傍フォールバックが warn へ昇格していない"
                );
            }
        }
    }

    /// **Req 3.1 の裏面**: 明示操作系・非配置系の引き金では、バルーン位置が素の offset
    /// 恒等式と 1 bit も違わず、ガードのログも 1 行も出ない。
    #[test]
    fn balloon_visibility_guard_does_not_fire_on_explicit_or_non_placement_triggers() {
        for dpi in DPIS {
            let layout = mixed_layout(dpi);
            let b_size = balloon_size(dpi);
            let offset = far_out_offset(dpi);
            let old_pos = visible_balloon_pos(dpi);
            for route in [
                PlacementRoute::SpawnInitial,
                PlacementRoute::Restore,
                PlacementRoute::KeepPositionResize,
                PlacementRoute::BalloonFollow,
                PlacementRoute::MoveCue,
            ] {
                let (mut world, char_window, balloon) =
                    char_with_far_balloon_world(dpi, old_pos, offset);
                let settled = char_settled_pos(dpi);
                let bare = point(settled.x + offset.x, settled.y + offset.y);
                // 探針の自己検査: 発火条件は揃っている（引き金だけが違う）。
                assert!(
                    !visible_in(&layout, bare, b_size),
                    "dpi={dpi}: 探針が不動点——発火条件が揃っていない"
                );

                let (ok, events) = capture_logs(|| {
                    resize_window_to(&mut world, char_window, char_size(dpi), route)
                });
                assert!(ok, "dpi={dpi} route={route}: 書込は成立する前提");

                assert_eq!(
                    point_of(&world, balloon),
                    bare,
                    "dpi={dpi} route={route}: 適用外の引き金でバルーンが動いた（明示操作の尊重が壊れている）"
                );
                assert!(
                    guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
                    "dpi={dpi} route={route}: 適用外の引き金でガードが喋っている: {events:?}"
                );
            }
        }
    }

    /// **本タスクの中核の守り（[[6.1 → 6.2 の申し送り]]）**: ドラッグ随伴では発火しない。
    ///
    /// `follow_balloon` は配置系（[`resize_window_to`]）とドラッグ
    /// （[`on_char_drag`]／[`on_char_drag_end`]）の**双方**から呼ばれる。無条件適用すると
    /// ユーザーがキャラを画面端へ運んだときにバルーンだけが引き戻され、Req 3.1 の
    /// 「明示操作の尊重」が壊れる——その変異を**位置 assert**で捕まえる（ログ側の否定
    /// assert だけに依存しない・[[5.2 の教訓]]）。
    #[test]
    fn balloon_drag_trigger_neither_clamps_nor_warns() {
        for dpi in DPIS {
            let layout = mixed_layout(dpi);
            let c_size = char_size(dpi);
            let b_size = balloon_size(dpi);
            let offset = far_out_offset(dpi);
            let old_pos = visible_balloon_pos(dpi);
            let start = char_start_pos(dpi);
            let cursor = (px(800, dpi), px(400, dpi));
            // カーソルを右へ px(100) 動かす＝生ドラッグ x は px(1600)。
            let moved = (cursor.0 + px(100, dpi), cursor.1);
            // 射影 T 適用後のキャラ確定位置（下端接地・X は素通し）。
            let settled = point(px(1600, dpi), grounded_y(right_wa(dpi), c_size));
            let bare = point(settled.x + offset.x, settled.y + offset.y);

            // 探針の自己検査: ドラッグ随伴の提案は**本当に**全 work area 非交差
            //（＝ガードが配線されていれば必ず clamp する状況である）。旧矩形は可視。
            assert!(
                !visible_in(&layout, bare, b_size),
                "dpi={dpi}: 探針が不動点——ドラッグ随伴の提案 {bare:?} が可視のまま"
            );
            assert!(
                visible_in(&layout, old_pos, b_size),
                "dpi={dpi}: 旧バルーンが非交差では『留置の尊重』と区別が付かない"
            );

            for entry in ["on_char_drag", "on_char_drag_end"] {
                let (mut world, char_window, balloon) =
                    char_with_far_balloon_world(dpi, old_pos, offset);
                world
                    .entity_mut(char_window)
                    .insert(dragging_state((start.x, start.y), cursor));

                let (_, events) = capture_logs(|| match entry {
                    "on_char_drag" => {
                        let ev = Phase::Bubble(drag_event_at(char_window, cursor, moved));
                        on_char_drag(&mut world, char_window, char_window, &ev)
                    }
                    _ => {
                        let ev = Phase::Bubble(drag_end_event_at(char_window, moved));
                        on_char_drag_end(&mut world, char_window, char_window, &ev)
                    }
                });

                assert_eq!(
                    point_of(&world, char_window),
                    settled,
                    "dpi={dpi} {entry}: 前提——ドラッグの確定位置が想定と違う"
                );
                assert_eq!(
                    point_of(&world, balloon),
                    bare,
                    "dpi={dpi} {entry}: ドラッグ随伴でバルーンが引き戻された（Req 3.1 違反）"
                );
                assert!(
                    guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
                    "dpi={dpi} {entry}: ドラッグ随伴でガードが喋っている（spam・水準分岐の破壊）: {events:?}"
                );
            }
        }
    }

    /// ユーザーが画面外へ留置したバルーンは、配置系の引き金でも引き戻さない
    /// （キャラ窓と完全に同一の規則＝`Keep` 腕・Req 3.1 の「明示操作の尊重」）。
    #[test]
    fn balloon_parked_off_screen_is_respected_on_placement_trigger() {
        for dpi in DPIS {
            let layout = mixed_layout(dpi);
            let b_size = balloon_size(dpi);
            let offset = far_out_offset(dpi);
            // 旧バルーンは既に全 work area の外（ユーザー留置）。
            let parked = point(px(2400, dpi), px(240, dpi));
            assert!(
                !visible_in(&layout, parked, b_size),
                "dpi={dpi}: 前提——旧バルーンは既に非交差（留置）"
            );

            let (mut world, char_window, balloon) =
                char_with_far_balloon_world(dpi, parked, offset);
            let settled = char_settled_pos(dpi);
            let bare = point(settled.x + offset.x, settled.y + offset.y);
            assert!(
                !visible_in(&layout, bare, b_size),
                "dpi={dpi}: 前提——提案も非交差（`Keep` 腕を通る条件）"
            );

            let (ok, events) = capture_logs(|| {
                resize_window_to(
                    &mut world,
                    char_window,
                    char_size(dpi),
                    PlacementRoute::DpiReproject,
                )
            });
            assert!(ok);
            assert_eq!(
                point_of(&world, balloon),
                bare,
                "dpi={dpi}: 留置バルーンが引き戻された（Keep 腕が効いていない）"
            );
            assert!(
                guard_events(&events, CLAMP_TAG).is_empty(),
                "dpi={dpi}: 留置バルーンに ClampX が出ている: {events:?}"
            );
        }
    }

    /// 任意の `WindowPos` を持つバルーンで [`char_with_far_balloon_world`] 相当を組む
    /// （未確定表現の探針用）。
    fn char_with_balloon_window_pos(
        dpi: i32,
        balloon_pos: WindowPos,
        offset: PointPx,
    ) -> (World, Entity, Entity) {
        let c = char_size(dpi);
        let start = char_start_pos(dpi);
        let mut world = World::new();
        world.insert_resource(mixed_layout(dpi));
        let balloon = world.spawn((fake_handle(0x2000), balloon_pos)).id();
        let char_window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(start.x, start.y, c.w, c.h),
                Anchored(Anchor::Bottom),
                BalloonFollow { balloon, offset },
            ))
            .id();
        (world, char_window, balloon)
    }

    /// **バルーン寸の未確定は `Option::None` だけではない**（[[4.6 の教訓]]・6.1 の
    /// `old_rect` 導出と同型の罠）: `WindowPos::default()` は position・size の**両方**を
    /// `CW_USEDEFAULT`（`i32::MIN` センチネル）で持つ。
    ///
    /// センチネルを素の矩形として交差判定へ入れると `saturating_add` で逆転矩形になり、
    /// 判定そのものが意味を失う。是正版は**寸が未確定なら位置に一切手を入れず** `warn!` を残す。
    ///
    /// # 檻の非空虚性（[[5.2 の教訓]]＝ログ側だけの守りにしない）
    ///
    /// 寸フィルタを外す変異では、位置センチネルが `old_rect = None`（不明）へ落ちるため
    /// 安全側 `ClampX` が走り、`clamp_x_into(x, i32::MIN, wa)` が `wa.left` を返す
    /// ＝**提案位置と違う座標が書かれる**。提案 X を `left_wa().left` より左へ置いてあるのは
    /// そのためで、位置 assert が第一の守りになる。
    #[test]
    fn balloon_undetermined_size_holds_proposed_position_and_warns() {
        for dpi in DPIS {
            // 提案 X は左モニタ work area の左端よりさらに左（センチネル素通し変異で
            // 必ず `left_wa().left` へ引き戻される位置）。
            let offset = point(-px(4500, dpi), -px(400, dpi));
            let settled = char_settled_pos(dpi);
            let bare = point(settled.x + offset.x, settled.y + offset.y);
            assert!(
                bare.x < left_wa().left,
                "dpi={dpi}: 探針が不動点——センチネル素通し変異でも X が動かない配置になっている"
            );

            // 窓生成直後の実表現（position・size ともに CW_USEDEFAULT センチネル）。
            let (mut world, char_window, balloon) =
                char_with_balloon_window_pos(dpi, WindowPos::default(), offset);

            let (ok, events) = capture_logs(|| {
                resize_window_to(
                    &mut world,
                    char_window,
                    char_size(dpi),
                    PlacementRoute::ReportedSizeReconcile,
                )
            });
            assert!(ok);
            assert_eq!(
                point_of(&world, balloon),
                bare,
                "dpi={dpi}: 寸未確定（センチネル）なのに位置へ手が入った"
            );
            let warned = expect_one(&events, UNRESOLVED_TAG);
            assert_eq!(
                warned.level,
                tracing::Level::WARN,
                "dpi={dpi}: 判定不能が warn として残っていない（Req 3.3）"
            );
            // **フィールド集合の固定**（`diagnosis-procedure.md` §3.1／§6.3 の振り分け規則が
            // これに依存する）: `route=BalloonFollow` で窓種別が引け、**`proposed` の有無**が
            // 本行（良性の判定不能）と装置異常（`MonitorSnapshot` 不在・モニタ 0 台）を分ける。
            // どちらを落としても実機判定が反転するので、literal で固定する
            // （[[5.1 → 7.2 の申し送り＝判定語に使っているのに檻が無い型]] の再発防止）。
            assert_eq!(
                warned.field("route"),
                "BalloonFollow",
                "dpi={dpi}: 判定不能行が窓種別を名乗っていない（§3.1 の振り分けが成立しない）"
            );
            assert_eq!(
                warned.field("proposed"),
                format!("{bare:?}"),
                "dpi={dpi}: 判定不能行の `proposed` が提案位置と違う（§6.3 の判別子）"
            );
            assert!(
                guard_events(&events, CLAMP_TAG).is_empty(),
                "dpi={dpi}: 寸が読めないのに clamp している: {events:?}"
            );
        }
    }

    /// **§6.3 の判別子の裏面**: 真の観測装置異常（`MonitorSnapshot` 不在）はキャラ窓・
    /// バルーン窓の**双方**から `WorkAreaUnresolved` を出すが、いずれも **`proposed` を
    /// 持たない**。
    ///
    /// 手順書はこの 1 点で「良性の判定不能（バルーン寸未確定）」と「セッション全体を
    /// 無効にする装置異常」を分ける。`route=` だけでは分けられない——装置異常も
    /// バルーン随伴で起きれば `route=BalloonFollow` を名乗るからである。
    #[test]
    fn missing_monitor_snapshot_warns_for_both_windows_without_the_proposed_field() {
        for dpi in DPIS {
            let (mut world, char_window, _balloon) = char_with_far_balloon_world(
                dpi,
                visible_balloon_pos(dpi),
                far_out_offset(dpi),
            );
            world.remove_resource::<MonitorSnapshot>();
            // 射影が identity へ縮退しても書込が起きるよう、寸を変える（高さのみ＝
            // 手順 3b の x 付替えを避ける）。同寸だとべき等 skip で随伴まで届かない。
            let new = SizePx {
                w: char_size(dpi).w,
                h: px(200, dpi),
            };

            let (ok, events) =
                capture_logs(|| resize_window_to(&mut world, char_window, new, PlacementRoute::Resnap));
            assert!(ok, "dpi={dpi}: 寸の反映自体は従来どおり成立する");

            let warned = guard_events(&events, UNRESOLVED_TAG);
            assert_eq!(
                warned.len(),
                2,
                "dpi={dpi}: 装置異常はキャラ窓とバルーン窓の双方から出るはず: {events:?}"
            );
            let routes: Vec<&str> = warned.iter().map(|e| e.field("route")).collect();
            assert!(
                routes.contains(&"Resnap") && routes.contains(&"BalloonFollow"),
                "dpi={dpi}: 2 行の route が {routes:?}（キャラ窓＋バルーン窓の対になっていない）"
            );
            for e in &warned {
                assert_eq!(e.level, tracing::Level::WARN, "dpi={dpi}: 水準が warn でない");
                assert!(
                    !e.fields.contains_key("proposed"),
                    "dpi={dpi}: 装置異常の行が `proposed` を持っている＝§6.3 の判別子が壊れる: {:?}",
                    e.fields
                );
            }
            assert!(
                guard_events(&events, CLAMP_TAG).is_empty(),
                "dpi={dpi}: work area 不明なのに clamp している: {events:?}"
            );
        }
    }

    /// **旧位置の未確定も `Option::None` だけではない**: 寸だけ確定して位置が
    /// `CW_USEDEFAULT` のままの窓は、素通しすると矩形が `i32::MIN` 近傍へ落ちて
    /// 「もともと画面外に留置されていた」と誤判定され、**安全側 clamp の腕が丸ごと死ぬ**
    /// （6.1 が寸について踏んだのと同型の罠を、位置について踏まないための檻）。
    ///
    /// 負座標そのものは正当（左モニタは `-1920..0`）ゆえ、判定は符号ではなく
    /// wintf 正典のセンチネル一致で行う。
    #[test]
    fn balloon_undetermined_position_is_treated_as_unknown_rect_and_clamps() {
        for dpi in DPIS {
            let layout = mixed_layout(dpi);
            let b_size = balloon_size(dpi);
            let offset = far_out_offset(dpi);
            let settled = char_settled_pos(dpi);
            let bare = point(settled.x + offset.x, settled.y + offset.y);
            assert!(
                !visible_in(&layout, bare, b_size),
                "dpi={dpi}: 探針が不動点——提案が既に可視で安全側 clamp の腕へ入らない"
            );

            // 寸は確定済み・位置だけ CW_USEDEFAULT（wintf 正典の未確定表現）。
            let window_pos = WindowPos {
                size: Some(SizeI::new(b_size.w, b_size.h)),
                ..Default::default()
            };
            let (mut world, char_window, balloon) =
                char_with_balloon_window_pos(dpi, window_pos, offset);

            let (ok, events) = capture_logs(|| {
                resize_window_to(
                    &mut world,
                    char_window,
                    char_size(dpi),
                    PlacementRoute::DpiReproject,
                )
            });
            assert!(ok);
            assert!(
                visible_in(&layout, point_of(&world, balloon), b_size),
                "dpi={dpi}: 位置未確定（センチネル）を『留置』と誤読して clamp を見送っている"
            );
            expect_one(&events, CLAMP_TAG);
        }
    }

    /// 破棄済みバルーンへの随伴は**正常終了系**として `debug!` で打ち切る（Req 6.2/6.3・
    /// task 3.2 と同じ区別）。ここを `warn!` にすると終了時ログが良性ノイズで埋まり、
    /// 本物の異常（実在窓の寸未確定）が読めなくなる。
    #[test]
    fn balloon_despawned_skips_guard_without_warning() {
        for dpi in DPIS {
            let (mut world, char_window, balloon) =
                char_with_far_balloon_world(dpi, visible_balloon_pos(dpi), far_out_offset(dpi));
            world.despawn(balloon);

            let (_, events) = capture_logs(|| {
                resize_window_to(
                    &mut world,
                    char_window,
                    char_size(dpi),
                    PlacementRoute::Resnap,
                )
            });

            assert!(
                guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
                "dpi={dpi}: 破棄済みバルーンに対してガードが喋っている（Req 6.2 違反）: {events:?}"
            );
            // **task 7.3 で強化**: 6.2 が固定していたのは「ガードが喋らない」だけで、
            // 随伴書込そのもの（`enqueue_window_set_pos`）が破棄済みバルーンに対して
            // `warn!` を出していた（6.2 → 7.3 の申し送り）。終了時静穏（Req 6.2）は
            // **経路全体**の主張ゆえ、ここで警告以上ゼロを丸ごと見る。
            assert!(
                events.iter().all(|e| e.level > tracing::Level::INFO),
                "dpi={dpi}: 破棄済みバルーンに対して警告以上のログが出ている（Req 6.2 違反）: {events:?}"
            );
            // **相ごとに数える**——総数で数えると、片方の打ち切りを外しても他方が同じ
            // 判定語を出して総数が偶然一致し、檻が空虚になる（3.2 の教訓と同型）。
            let skips = despawn_skip_lines(&events);
            assert!(
                skips.iter().all(|e| e.level == tracing::Level::DEBUG),
                "dpi={dpi}: 破棄済み打ち切りが debug 水準でない: {skips:?}"
            );
            assert_eq!(
                skips
                    .iter()
                    .filter(|e| e.message().contains("可視性の遷移ガード"))
                    .count(),
                1,
                "dpi={dpi}: 遷移ガード相の打ち切りが 1 行でない: {events:?}"
            );
            assert_eq!(
                skips
                    .iter()
                    .filter(|e| e.message().contains("窓移動"))
                    .count(),
                1,
                "dpi={dpi}: 随伴書込相の打ち切りが 1 行でない: {events:?}"
            );
        }
    }

    /// 破棄済み判定語（[`DESPAWNED_SKIP_TAG`]）を含む行を抜く（相ごとの計数用）。
    fn despawn_skip_lines(events: &[LogEvent]) -> Vec<&LogEvent> {
        events
            .iter()
            .filter(|e| e.message().contains(DESPAWNED_SKIP_TAG))
            .collect()
    }

    /// Req 6.2 の裏面（真の異常を殺さない・随伴書込相）: **生存している** entity の
    /// `WindowHandle` 欠落（窓生成前）は従来どおり `warn!`。存在確認の導入でこちらまで
    /// 静穏化してはならない——「窓がまだ無い」は結線の異常であって終了系ではない。
    #[test]
    fn balloon_without_handle_on_living_entity_still_warns_on_follow_write() {
        let dpi = 96;
        let (mut world, char_window, balloon) =
            char_with_far_balloon_world(dpi, visible_balloon_pos(dpi), far_out_offset(dpi));
        // entity は実在させたまま `WindowHandle` だけを剥がす（窓生成前と同じ状態）。
        world.entity_mut(balloon).remove::<WindowHandle>();

        let (_, events) = capture_logs(|| {
            resize_window_to(
                &mut world,
                char_window,
                char_size(dpi),
                PlacementRoute::Resnap,
            )
        });

        let warned = expect_one(&events, "WindowHandle 未付与");
        assert_eq!(
            warned.level,
            tracing::Level::WARN,
            "実在 entity の WindowHandle 欠落は真の異常＝warn のまま（Req 6.2 の区別）"
        );
        assert!(
            !despawn_skip_lines(&events)
                .iter()
                .any(|e| e.message().contains("窓移動")),
            "実在 entity を『破棄済み』と誤判定している: {events:?}"
        );
    }

    // -------------------------------------------------------------------------
    // 位置の未確定表現（`CW_USEDEFAULT`）をキャラ窓経路でも打ち切る
    // （task 6.3・S3 補・D15・Req 3.1/3.3）
    //
    // `resize_window_to` 手順 3 は `WindowPos.position` の `Option::None` しか縮退させて
    // おらず、wintf 正典の**もう一つの未確定表現**（`CW_USEDEFAULT` ＝ `i32::MIN`・
    // `WindowPos::default()` が position に持つ）を素通ししていた。素通しすると
    //   ① 手順 3a の `old_rect` が `i32::MIN` 近傍の全 work area 非交差矩形になり、
    //      `guard_visibility` が「もともと留置されていた」と誤読して `Keep` へ落ちる
    //      ＝**6.1 が敷いた安全側 clamp の腕が黙って死ぬ**
    //   ② 手順 3b の中央付替えと射影 T の入力（raw）も同時に汚染される
    // D15 は (b) **resize 打ち切り**を採る——位置未確定は「保存すべき接地点が存在しない」
    // ゆえ、`Option::None` と同じ腕（`warn!`＋`false`）へ合流させて①②を一括で断つ。
    //
    // 檻の要点（空虚化を避けるための自己検査を各檻が持つ）:
    //   (1) 打ち切り檻の自己検査——**位置だけを実値に替えた対照窓**が同じ route・同じ寸で
    //       確実に書込まで進むこと（進まないなら「打ち切れた」は何も意味しない）
    //   (2) 書込ゼロの直接観測——`WindowPos` が呼出前後で**完全一致**（`PartialEq`）
    //   (3) `warn!` ちょうど 1 件——ログ側の守りを位置 assert と二段構えにする
    //       （[[5.2 の教訓＝空虚性 6 例目]]／[[6.2 の教訓＝檻の空虚性]]）
    //   (4) **符号判定への変異の検出**——左モニタは `-1920..0` ＝負座標そのものは正当。
    //       実在する負座標の窓が打ち切られないことを独立の檻で固定する
    //
    // なお寸センチネルとの**非対称は意図的**（D15 帰結⑴）: 寸未確定は接地点（位置）が
    // 実在するので resize に意味があり、`old_rect` 不明の安全側 clamp で扱う
    // （既存檻 `undetermined_old_size_is_treated_as_unknown_rect_and_clamps` が無改変で
    // 緑のまま＝その非対称の檻を兼ねる）。
    // -------------------------------------------------------------------------

    /// wintf 正典の未確定センチネル（`== i32::MIN`）。**本体の import とは独立に**
    /// 定義元から直接引き、判定式が正典と同式であることを檻側でも固定する
    /// （`window_pos.rs:41`／`monitor_systems.rs:408` と同じ値）。
    use windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT as SENTINEL;

    /// 手順 3 の位置センチネル打ち切りが名乗る語（**本体の文言とは独立に literal で置く**）。
    const POSITION_SENTINEL_TAG: &str = "センチネル（位置未確定）";

    /// 位置・寸を明示した単独キャラ窓の World（混在 DPI 合成レイアウト付き）。
    fn char_world_with_window_pos(dpi: i32, position: Point, size: Option<SizeI>) -> (World, Entity) {
        let mut world = World::new();
        world.insert_resource(mixed_layout(dpi));
        let e = world
            .spawn((
                fake_handle(0x1000),
                WindowPos {
                    position: Some(position),
                    size,
                    ..Default::default()
                },
                Anchored(Anchor::Bottom),
            ))
            .id();
        (world, e)
    }

    /// 旧寸（[`wide_char_size`] の `SizeI` 表現）。
    fn old_size_i(dpi: i32) -> SizeI {
        let s = wide_char_size(dpi);
        SizeI::new(s.w, s.h)
    }

    /// 左モニタ（**負座標** `-1920..0`）内の**実在する**接地位置。
    ///
    /// 符号（`x < 0`）や大きさの閾値で未確定判定をすると、この正当な位置が巻き添えで
    /// 打ち切られる＝檻 [`negative_real_position_is_not_aborted_and_still_resizes`] の被検体。
    fn negative_real_pos(dpi: i32) -> Point {
        Point {
            x: left_wa().left / 2,
            y: left_wa().bottom - old_size_i(dpi).height,
        }
    }

    /// **探針の自己検査**: 位置**だけ**を実値に替えた対照窓は、同じ route・同じ新寸で
    /// 必ず書込まで進む。これが崩れていると打ち切り檻の「何も起きなかった」は
    /// センチネルの成果ではなく入力の不備になる（不動点の検出）。
    fn assert_control_position_writes(dpi: i32, new: SizePx) {
        let (mut world, e) =
            char_world_with_window_pos(dpi, negative_real_pos(dpi), Some(old_size_i(dpi)));
        let before = *world.get::<WindowPos>(e).expect("WindowPos があるはず");
        assert!(
            resize_window_to(&mut world, e, new, PlacementRoute::Resnap),
            "dpi={dpi}: 探針が不動点——位置が実値の対照でも resize が成立しない"
        );
        assert_ne!(
            *world.get::<WindowPos>(e).expect("WindowPos があるはず"),
            before,
            "dpi={dpi}: 探針が不動点——対照でも WindowPos が 1 bit も変わらない"
        );
    }

    /// **位置がセンチネルの窓は log-first で打ち切る**（D15 採用案 (b)）: 戻り値 `false`・
    /// `WindowPos` 書込ゼロ・`warn!` ちょうど 1 件。
    ///
    /// 是正前はここで安全側 `ClampX` が走り、`clamp_x_into(i32::MIN, .., wa)` が返す
    /// `wa.left` が**位置権威の無い窓へ書き込まれて**いた（＝位置権威の僭称）。
    #[test]
    fn undetermined_position_aborts_resize_without_writing() {
        for dpi in DPIS {
            let new = narrow_char_size(dpi);
            assert_control_position_writes(dpi, new);

            for (label, size) in [
                // `on_window_add` が挿す実表現そのもの（位置・寸とも未確定）。
                ("窓生成直後（位置・寸ともセンチネル）", None),
                // 寸だけ確定した窓＝汚染されるのは位置の側だけ、という切り分け。
                ("寸のみ確定・位置センチネル", Some(old_size_i(dpi))),
            ] {
                let position = Point {
                    x: SENTINEL,
                    y: SENTINEL,
                };
                let (mut world, e) = char_world_with_window_pos(dpi, position, size);
                // 探針の前提: 被検体が本当にセンチネルを持っている。
                assert_eq!(
                    world
                        .get::<WindowPos>(e)
                        .expect("WindowPos があるはず")
                        .position,
                    Some(position),
                    "dpi={dpi} {label}: 探針がセンチネルを持っていない"
                );
                let before = *world.get::<WindowPos>(e).expect("WindowPos があるはず");

                let (ok, events) =
                    capture_logs(|| resize_window_to(&mut world, e, new, PlacementRoute::Resnap));

                assert!(
                    !ok,
                    "dpi={dpi} {label}: 位置未確定（センチネル）なのに resize が成立している"
                );
                assert_eq!(
                    *world.get::<WindowPos>(e).expect("WindowPos があるはず"),
                    before,
                    "dpi={dpi} {label}: 打ち切りのはずが WindowPos へ書き込まれている（Req 3.3 の現状維持違反）"
                );
                let warned = expect_one(&events, POSITION_SENTINEL_TAG);
                assert_eq!(
                    warned.level,
                    tracing::Level::WARN,
                    "dpi={dpi} {label}: 打ち切りが warn として残っていない（log-first 違反）"
                );
                assert_eq!(
                    warned.field("entity"),
                    format!("{e:?}"),
                    "dpi={dpi} {label}: 警告行が対象 entity を名乗っていない"
                );
                assert_eq!(
                    warned.field("position"),
                    format!("{position:?}"),
                    "dpi={dpi} {label}: 警告行が問題の位置を載せていない"
                );
                assert!(
                    guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
                    "dpi={dpi} {label}: 打ち切ったのにガードが喋っている（射影 T の入力が汚染されている）: {events:?}"
                );
            }
        }
    }

    /// **負座標そのものは正当**（合成レイアウトの左モニタは `-1920..0`）。
    ///
    /// 判定を符号（`x < 0`）や大きさの閾値へ変異させると、この実在位置の窓まで打ち切られる。
    /// ゆえに本檻は「打ち切られない」ことを**位置の実値**で固定する（従来経路の非退行）。
    #[test]
    fn negative_real_position_is_not_aborted_and_still_resizes() {
        for dpi in DPIS {
            let start = negative_real_pos(dpi);
            let new = narrow_char_size(dpi);
            let layout = mixed_layout(dpi);
            // 探針の自己検査: ①本当に負座標であり ②センチネルではなく
            // ③旧矩形が実際に可視（＝「もともと留置」腕へ落ちない通常経路の入力）。
            assert!(start.x < 0, "dpi={dpi}: 探針が負座標になっていない");
            assert_ne!(start.x, SENTINEL, "dpi={dpi}: 探針がセンチネルと衝突している");
            assert!(
                visible_in(
                    &layout,
                    PointPx {
                        x: start.x,
                        y: start.y
                    },
                    wide_char_size(dpi)
                ),
                "dpi={dpi}: 探針の旧矩形が既に不可視——通常経路を通らない"
            );

            let (mut world, e) = char_world_with_window_pos(dpi, start, Some(old_size_i(dpi)));
            let (ok, events) =
                capture_logs(|| resize_window_to(&mut world, e, new, PlacementRoute::Resnap));

            assert!(
                ok,
                "dpi={dpi}: 正当な負座標が打ち切られた（符号での未確定判定＝D15 が禁じた式）"
            );
            assert_eq!(
                point_of(&world, e),
                unguarded_projection(
                    dpi,
                    PointPx {
                        x: start.x,
                        y: start.y
                    },
                    new
                ),
                "dpi={dpi}: 負座標の従来経路（手順 3b＋射影 T）が退行している"
            );
            assert!(
                guard_events(&events, POSITION_SENTINEL_TAG).is_empty(),
                "dpi={dpi}: 正当な負座標に対してセンチネル警告が出ている: {events:?}"
            );
            assert!(
                guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
                "dpi={dpi}: 可視 → 可視の遷移でガードが喋っている: {events:?}"
            );
        }
    }

    /// **片軸だけ**のセンチネルも打ち切る（`pos.x == SENTINEL || pos.y == SENTINEL`）。
    ///
    /// `&&` への変異（両軸そろったときだけ打ち切る）を検出する。y のみのセンチネルは
    /// wintf 正典の `window_center` が見ていない軸であり、`||` にしてある理由が
    /// 「接地点（下端中央）は x・y の**両方**が揃って初めて意味を持つ」ことである。
    #[test]
    fn single_axis_position_sentinel_also_aborts() {
        for dpi in DPIS {
            let new = narrow_char_size(dpi);
            let real = negative_real_pos(dpi);
            assert_control_position_writes(dpi, new);

            for (label, position) in [
                (
                    "x のみセンチネル",
                    Point {
                        x: SENTINEL,
                        y: real.y,
                    },
                ),
                (
                    "y のみセンチネル",
                    Point {
                        x: real.x,
                        y: SENTINEL,
                    },
                ),
            ] {
                let (mut world, e) =
                    char_world_with_window_pos(dpi, position, Some(old_size_i(dpi)));
                let before = *world.get::<WindowPos>(e).expect("WindowPos があるはず");

                let (ok, events) =
                    capture_logs(|| resize_window_to(&mut world, e, new, PlacementRoute::Resnap));

                assert!(
                    !ok,
                    "dpi={dpi} {label}: 片軸センチネルが打ち切られていない"
                );
                assert_eq!(
                    *world.get::<WindowPos>(e).expect("WindowPos があるはず"),
                    before,
                    "dpi={dpi} {label}: 打ち切りのはずが WindowPos へ書き込まれている"
                );
                let warned = expect_one(&events, POSITION_SENTINEL_TAG);
                assert_eq!(
                    warned.level,
                    tracing::Level::WARN,
                    "dpi={dpi} {label}: 打ち切りが warn として残っていない"
                );
                assert!(
                    guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
                    "dpi={dpi} {label}: 打ち切ったのにガードが喋っている: {events:?}"
                );
            }
        }
    }

    // -------------------------------------------------------------------------
    // 混在 DPI・複数モニタ回帰檻の拡充（task 7.2・Req 3.4/4.4/5.1/5.2/5.3/5.6）
    //
    // task 6.1 は**キャラ窓だけ**が不可視へ落ちる合成を、task 6.2 は**バルーンだけ**が
    // 落ちる合成（キャラは終始可視だと明示的に assert する）を固めた。どちらの檻も
    // 「もう一方の窓は自明に安全」な世界で 1 つの連言肢を証明しており、Req 3.4 が
    // 要求する **連言**——「キャラ窓とバルーン窓の *どちらも* 不可視状態に遷移させない」
    // ——を 1 回の書込の中で見た檻は存在しない。本節が足すのはその連言と、
    // 2 つのガードが**互いの結果に依存する**接続点である。
    //
    //   (A) 1 回の [`resize_window_to`] で**両窓が同時に**全 work area 非交差へ落ちる
    //       合成。しかも救出先の work area が**別々のモニタ**になる配置で組むので、
    //       clamp 先の解決が窓ごとに独立であること（キャラの clamp_wa を流用していない
    //       こと）まで座標で固定される。
    //   (B) バルーンが追従するのは **ガード適用後**のキャラ位置であること。手順 7 が
    //       `new_pos` ではなく素の射影（`raw`／ガード前）を渡す変異は、6.2 の檻では
    //       **不動点**になる（あちらはキャラが clamp されない合成ゆえ両者が同値）。
    //       ここでは clamp 前後で px(40) ずれるので、恒等式の主張が実際に効く。
    //
    // 座標はすべて論理値 × DPI（96/120/192）で構築し、絶対 px の固定値を持たない
    // （Req 5.6）。実 GPU・実高 DPI モニタを要さず決定論（Req 5.2）。
    // -------------------------------------------------------------------------

    /// 下端中央原点の移動量（左上基準 offset の付替え量・[`resize_window_to`] 手順 6 の
    /// **檻側の独立実装**）。本体の式を呼び直さない。
    fn origin_delta(old: SizePx, new: SizePx) -> PointPx {
        point(new.w / 2 - old.w / 2, new.h - old.h)
    }

    /// [`gap_bound_char_world`] に随伴バルーンを足した World。
    ///
    /// `offset` は **spawn 時点**の左上基準 offset。手順 6 が原点移動ぶんを付け替えるため、
    /// 追従に実際に使われるのは `offset + origin_delta(wide, narrow)` である。
    fn gap_bound_char_world_with_balloon(
        dpi: i32,
        balloon_size: SizePx,
        balloon_pos: PointPx,
        offset: PointPx,
    ) -> (World, Entity, Entity, PointPx) {
        let old = wide_char_size(dpi);
        let old_pos = PointPx {
            x: gap_center_x(dpi) - old.w / 2,
            y: left_wa().bottom - old.h,
        };
        let mut world = World::new();
        world.insert_resource(mixed_layout(dpi));
        let balloon = world
            .spawn((
                fake_handle(0x2000),
                window_pos_sized(
                    balloon_pos.x,
                    balloon_pos.y,
                    balloon_size.w,
                    balloon_size.h,
                ),
            ))
            .id();
        let char_window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(old_pos.x, old_pos.y, old.w, old.h),
                Anchored(Anchor::Bottom),
                BalloonFollow { balloon, offset },
            ))
            .id();
        (world, char_window, balloon, old_pos)
    }

    /// **Req 3.4／5.3 の連言**: 1 回の非ドラッグ配置書込で、キャラ窓とバルーン窓の
    /// **どちらも**全 work area 非交差にならない。しかも救出先は**別々のモニタ**である。
    ///
    /// 合成の骨格（混在 DPI・複数モニタ・負座標・192 で 3200 超座標）:
    /// - キャラ窓は帯（`0 ..= px(64)`＝どの work area にも属さない）へ落ちる幅の新寸を
    ///   受け取り、**右モニタ**へ引き戻される（[`gap_bound_char_world`] と同じ機序）。
    /// - 随伴 offset は救出後のキャラ位置から見て遥か左（`-px(2600)`）を指すので、
    ///   バルーン提案矩形は**左モニタよりさらに左**の完全不可視域へ出る。最近傍は
    ///   左モニタゆえ **`left_wa().left` へ**引き戻される。
    ///
    /// ゆえに 2 つの clamp 先が別モニタになる——キャラの `clamp_wa` を流用する実装は
    /// バルーンを右モニタへ引き戻してしまい、`balloon.x == left_wa().left` の assert が
    /// 落ちる。6.1／6.2 の単窓檻はどちらもこの取り違えに対して不動点である
    /// （両窓の clamp 先が同じ右モニタになる合成しか持っていない）。
    #[test]
    fn both_windows_survive_a_single_write_onto_different_monitors() {
        for dpi in DPIS {
            let layout = mixed_layout(dpi);
            let new = narrow_char_size(dpi);
            let b_size = balloon_size(dpi);
            // 追従に実際に使われる offset（手順 6 の付替え後）と、spawn 時点の offset。
            let applied_offset = point(-px(2600, dpi), -px(600, dpi));
            let d_origin = origin_delta(wide_char_size(dpi), new);
            let spawn_offset = point(
                applied_offset.x - d_origin.x,
                applied_offset.y - d_origin.y,
            );
            // 旧バルーンは**左モニタ内**で可視（＝「遷移」であって留置ではない）。
            // 座標は左モニタ左端からの論理オフセット×DPI で組む（絶対 px を置かない・Req 5.6）。
            let old_balloon = point(left_wa().left + px(360, dpi), px(200, dpi));

            for route in [
                PlacementRoute::AnchorChange,
                PlacementRoute::Resnap,
                PlacementRoute::DpiReproject,
                PlacementRoute::ReportedSizeReconcile,
            ] {
                let (mut world, char_window, balloon, old_pos) =
                    gap_bound_char_world_with_balloon(dpi, b_size, old_balloon, spawn_offset);

                // --- (1) 探針の自己検査（[[2.2 の教訓]]）---
                let char_bare = unguarded_projection(dpi, old_pos, new);
                let char_saved = point(right_wa(dpi).left, char_bare.y);
                let balloon_bare = point(
                    char_saved.x + applied_offset.x,
                    char_saved.y + applied_offset.y,
                );
                assert!(
                    visible_in(&layout, old_pos, wide_char_size(dpi)),
                    "dpi={dpi}: 旧キャラ矩形が非交差では『遷移』にならない"
                );
                assert!(
                    visible_in(&layout, old_balloon, b_size),
                    "dpi={dpi}: 旧バルーン矩形が非交差では『遷移』にならない"
                );
                assert!(
                    !visible_in(&layout, char_bare, new),
                    "dpi={dpi}: 探針が不動点——ガード無しのキャラ提案 {char_bare:?} が既に可視"
                );
                assert!(
                    !visible_in(&layout, balloon_bare, b_size),
                    "dpi={dpi}: 探針が不動点——ガード無しのバルーン提案 {balloon_bare:?} が既に可視"
                );

                let (ok, events) = capture_logs(|| {
                    resize_window_to(&mut world, char_window, new, route)
                });
                assert!(ok, "dpi={dpi} route={route}: 書込は成立する前提");

                let char_pos = point_of(&world, char_window);
                let balloon_pos = point_of(&world, balloon);

                // --- (2) 連言そのもの（Req 3.4）: どちらも全 work area 非交差ではない ---
                assert!(
                    visible_in(&layout, char_pos, new),
                    "dpi={dpi} route={route}: キャラ窓 {char_pos:?} が全 work area と非交差"
                );
                assert!(
                    visible_in(&layout, balloon_pos, b_size),
                    "dpi={dpi} route={route}: バルーン窓 {balloon_pos:?} が全 work area と非交差"
                );

                // --- (3) 救出先は**別々のモニタ**（clamp 先の解決が窓ごとに独立）---
                assert_eq!(
                    char_pos, char_saved,
                    "dpi={dpi} route={route}: キャラは右モニタ左端へ引き戻されるはず"
                );
                assert_eq!(
                    balloon_pos.x,
                    left_wa().left,
                    "dpi={dpi} route={route}: バルーンの clamp 先が左モニタでない\
                     （キャラの clamp_wa を流用している疑い）: {balloon_pos:?}"
                );

                // --- (4) Y は両窓とも射影／恒等式の所有＝ガードは触らない ---
                assert_eq!(
                    char_pos.y, char_bare.y,
                    "dpi={dpi} route={route}: キャラの Y が動いた"
                );
                assert_eq!(
                    balloon_pos.y, balloon_bare.y,
                    "dpi={dpi} route={route}: バルーンの Y が動いた"
                );

                // --- (5) 判定語: ClampX が**ちょうど 2 行**（両窓ぶん）・水準は WARN ---
                let clamps = guard_events(&events, CLAMP_TAG);
                assert_eq!(
                    clamps.len(),
                    2,
                    "dpi={dpi} route={route}: ClampX が両窓ぶん 2 行でない: {events:?}"
                );
                for ev in clamps {
                    assert_eq!(
                        ev.level,
                        tracing::Level::WARN,
                        "dpi={dpi} route={route}: clamp の記録が warn 水準でない"
                    );
                }
            }
        }
    }

    /// **Req 4.4 の恒等式は「ガード適用後のキャラ位置」に対して成立する**。
    ///
    /// [`resize_window_to`] 手順 7 は確定位置（`new_pos`＝遷移ガード適用**後**）で
    /// [`follow_balloon`] を呼ぶ。ここを素の射影（ガード前）へ差し替える変異は、
    /// 6.2 の檻ではキャラが clamp されない合成ゆえ**不動点**になる。
    ///
    /// 本檻はキャラだけが clamp される合成（clamp 前後で X が `px(40)` ずれる）を組み、
    /// バルーンの追従先が**ずれた後**の位置であることを座標で固定する。バルーン自身は
    /// clamp されない（＝救われたのはキャラだけ・`ClampX` はちょうど 1 行）ので、
    /// 「バルーンが偶然どこかへ clamp されて結果が一致した」逃げ道も塞がる。
    #[test]
    fn balloon_follows_the_guarded_char_position_not_the_raw_projection() {
        for dpi in DPIS {
            let layout = mixed_layout(dpi);
            let new = narrow_char_size(dpi);
            // 帯（`0 ..= px(64)`）より**狭い**バルーン＝帯の中へ丸ごと収まり得る。
            let b_size = SizePx {
                w: px(48, dpi),
                h: px(300, dpi),
            };
            let applied_offset = point(-px(12, dpi), -px(600, dpi));
            let d_origin = origin_delta(wide_char_size(dpi), new);
            let spawn_offset = point(
                applied_offset.x - d_origin.x,
                applied_offset.y - d_origin.y,
            );
            let old_balloon = visible_balloon_pos(dpi);

            for route in [
                PlacementRoute::AnchorChange,
                PlacementRoute::Resnap,
                PlacementRoute::DpiReproject,
                PlacementRoute::ReportedSizeReconcile,
            ] {
                let (mut world, char_window, balloon, old_pos) =
                    gap_bound_char_world_with_balloon(dpi, b_size, old_balloon, spawn_offset);

                let char_bare = unguarded_projection(dpi, old_pos, new);
                let char_saved = point(right_wa(dpi).left, char_bare.y);
                let follows_guarded = point(
                    char_saved.x + applied_offset.x,
                    char_saved.y + applied_offset.y,
                );
                let follows_raw = point(
                    char_bare.x + applied_offset.x,
                    char_bare.y + applied_offset.y,
                );

                // --- 探針の自己検査: 2 つの追従先が**区別できる**こと ---
                assert_ne!(
                    follows_guarded.x, follows_raw.x,
                    "dpi={dpi}: 探針が不動点——ガード前後でキャラ X が動いていない"
                );
                assert!(
                    !visible_in(&layout, char_bare, new),
                    "dpi={dpi}: 探針が不動点——ガード無しのキャラ提案が既に可視"
                );
                assert!(
                    visible_in(&layout, follows_guarded, b_size),
                    "dpi={dpi}: 救出後のキャラに追従したバルーンは可視のはず（clamp 不要）"
                );
                assert!(
                    !visible_in(&layout, follows_raw, b_size),
                    "dpi={dpi}: 素の射影に追従したバルーン {follows_raw:?} が可視では変異を区別できない"
                );

                let (ok, events) = capture_logs(|| {
                    resize_window_to(&mut world, char_window, new, route)
                });
                assert!(ok, "dpi={dpi} route={route}: 書込は成立する前提");

                assert_eq!(
                    point_of(&world, char_window),
                    char_saved,
                    "dpi={dpi} route={route}: キャラが右モニタ左端へ救出されていない"
                );
                assert_eq!(
                    point_of(&world, balloon),
                    follows_guarded,
                    "dpi={dpi} route={route}: バルーンが**ガード適用後**のキャラ位置に追従していない\
                     （素の射影に追従した場合は {follows_raw:?}）"
                );
                assert!(
                    visible_in(&layout, point_of(&world, balloon), b_size),
                    "dpi={dpi} route={route}: 追従先のバルーンが全 work area と非交差"
                );

                // 恒等式（Req 4.4）: `balloon − char ≡ BalloonFollow.offset`（付替え後）。
                let offset = world
                    .get::<BalloonFollow>(char_window)
                    .expect("char 窓は BalloonFollow を持つ")
                    .offset;
                assert_eq!(
                    offset, applied_offset,
                    "dpi={dpi} route={route}: 原点（下端中央）基準の offset 付替えが崩れている"
                );
                let c = point_of(&world, char_window);
                let b = point_of(&world, balloon);
                assert_eq!(
                    point(b.x - c.x, b.y - c.y),
                    offset,
                    "dpi={dpi} route={route}: 追従恒等式が崩れている"
                );

                // 救われたのは**キャラだけ**＝`ClampX` はちょうど 1 行。
                let clamps = guard_events(&events, CLAMP_TAG);
                assert_eq!(
                    clamps.len(),
                    1,
                    "dpi={dpi} route={route}: ClampX がキャラぶん 1 行でない\
                     （バルーンまで clamp されているなら追従先が偶然一致しただけ）: {events:?}"
                );
                assert_eq!(
                    clamps[0].level,
                    tracing::Level::WARN,
                    "dpi={dpi} route={route}: clamp の記録が warn 水準でない"
                );
            }
        }
    }
}
