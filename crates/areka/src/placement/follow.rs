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
    balloon_offset_entries, char_pos_entries, char_pos_to_origin_x, persist_entries,
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
/// 左上基準 offset は基準変換せずそのまま [`balloon_offset_entries`]→[`persist_entries`]
/// で Ghost 永続スコープへ即時 write-through する（fire-and-forget・非ブロッキング）。
/// 保存基準がランタイム基準（char 左上）と同一である理由は
/// [`balloon_offset_entries`] を参照。
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

            // 保存基準＝ランタイム基準（char 左上）ゆえ基準変換なし（balloon_offset_entries
            // の基準記述）→ BalloonOffset entries を Ghost 永続スコープへ即時
            // write-through（fire-and-forget・7.1）。
            let persist = offset_tl;
            // 保存の計測ログ（実機診断・保存↔復元の座標突合）: balloon_pos＝バルーン最終位置、
            // char_pos＝追従元 char の最終位置、offset_tl＝左上基準差分、persist＝保存値
            // （＝offset_tl・BalloonOffset entries として書かれる値）、char_size＝診断用の現寸。
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
///
/// # バルーン追従はリサイズで補正しない（**全アンカー共通**・2026-07-31 実機裁定）
///
/// 本関数は `BalloonFollow.offset` を**一切書き換えない**。以前は Bottom に限って
/// 「原点＝下端中央からの相対を保つ」ため offset に `((w'/2−w/2), (h'−h))` を加算して
/// おり、上記の恒等式記述と矛盾していた（Bottom だけ窓相対でない＝内部分裂）。
/// 受理オラクルは参照実装 SSP の実測——SSP のバルーンは**観測時つねに現在表示中の**
/// キャラ窓に対して窓相対 (−168,−161) にある（実 DPI 120・むらさき 478×684 表示時）。
/// 一方 補正ありの areka は boot 採寸窓（543×859）を基準に置いたきり据え置き、
/// 切替後に 336px 上空へ浮かせていた。補正を撤去して
/// `areka-P0-surface-resize-resnap` Req2.6 の窓相対契約を復元し、矛盾を解消した。
/// 檻: `resize_window_to_bottom_keeps_ssp_window_relative_balloon_offset`。
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

    // 6. 随伴バルーン維持（Req2.6）: **リサイズで `BalloonFollow.offset` を補正しない**。
    //    受理オラクルは参照実装 SSP の実測——SSP のバルーンは観測時つねに現在表示中の
    //    キャラ窓に対して窓相対にある（2026-07-31 実機裁定）。これは
    //    `areka-P0-surface-resize-resnap` Req2.6 の「追従 offset を維持」という窓相対契約
    //    そのものであり、全アンカーで恒等式 `balloon_pos − char_pos ≡ offset` が成立する。
    // 確定後キャラ窓座標＋（不変の）offset で追従（offset 恒等式維持）。
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
#[path = "follow_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "follow_anchor_tests.rs"]
mod anchor_tests;
#[cfg(test)]
#[path = "follow_drag_tests.rs"]
mod drag_tests;
#[cfg(test)]
#[path = "follow_balloon_drag_tests.rs"]
mod balloon_drag_tests;
#[cfg(test)]
#[path = "follow_drag_end_persist_tests.rs"]
mod drag_end_persist_tests;
#[cfg(test)]
#[path = "follow_tests.rs"]
mod tests;
