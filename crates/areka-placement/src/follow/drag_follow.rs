//! ドラッグ＋バルーン追従（[`BalloonFollow`]・[`on_char_drag`]・[`follow_balloon`] ほか。バルーン単独のドラッグは `balloon_drag`）。

use bevy_ecs::prelude::*;
use tracing::{debug, warn};
use windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT;
use wintf::ecs::drag::{DragEndEvent, DragEvent, DraggingState};
use wintf::ecs::pointer::Phase;
use wintf::ecs::{Point, WindowPos};

use super::{
    Anchor, BalloonFollow, CharWindowMarker, DESPAWNED_SKIP_TAG, MonitorSnapshot, PlacementRoute,
    PointPx, SizePx, VISIBILITY_UNRESOLVED_TAG, balloon_shift, char_pos_entries,
    char_pos_to_origin_x, enqueue_window_set_pos, evaluate_visibility_guard, persist_entries,
    project_anchor, rect_at, reset_applied, route_applies_visibility_guard,
};
// 組の連れ動き（areka-P0-window-link-directives・設計の決め 4・6・12）。
use super::super::persist::saved_form_anchor;
use super::sticky_follow::{
    CharDragSession, drag_anchor, drag_wintf_moves, finish_char_drag, follow_partners_on_drag,
};

/// `OnDrag` ハンドラ: ドラッグ中のキャラ窓の移動とバルーン追従（4.2/4.3/4.7）。
///
/// 冒頭で `ev.target == entity` を検査し、他 entity 宛イベントには何もしない
/// （ハードニング・DD15 v2 (6)。wintf のドラッグ対象は `DragConfig` を持つ
/// 窓 entity 自身なので、実 flow では常に一致する）。
///
/// # areka が書く窓（吸着した窓・組に入った窓・4.7/1.6・DD15 v2・単一ライター）
///
/// 縛りの向きは [`drag_anchor`]（ドラッグの始めに控えた向き・2.4）、書き手は
/// [`drag_wintf_moves`]（始めに控えた `DragConfig.move_window`）で決める。吸着した窓と
/// 組に入った窓（吸着なしでも・設計の決め 12）は `move_window: false` で、wndproc は窓を
/// 動かさない。本ハンドラが唯一のライターとして、[`DraggingState`]＋
/// DragEvent のカーソル座標から生ドラッグ座標を復元し（[`policy_mapped_position`]）、
/// [`project_anchor`] 適用済み（`Bottom` は下端吸着）の座標を
/// **一度だけ**書く——反映段階で既に正しい座標が確定しているため、v1 の
/// 「wndproc 移動→事後再釘付け」の毎サイクル振動は原理的に起きない。ドラッグと
/// リサイズが同一 [`project_anchor`] を通ることで座標系変換を二重化しない（Req1.6）。
/// モニタ跨ぎ再吸着は射影の live 算出が担う。組に入った Free の窓は射影が恒等。
///
/// # wintf が書く窓（組に無い Free・向き不在）
///
/// `move_window: true` の窓（および安全側フォールバックとして向き不在）は wndproc
/// レベルで移動済み。本ハンドラは `WindowPos.position`（物理 px）を読むだけで窓を書かない。
///
/// どちらの経路でも、**確定後のキャラ窓座標**へバルーンを追従させ（再スケールなし・U4）、
/// その後に組の相手を [`follow_partners_on_drag`] で同じ量だけ動かす（4.1〜4.4）。
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

            // 縛りの向きはドラッグの始めの向き、分かれ目はこのドラッグの書き手（始めに控えた値）。
            // areka が書く（吸着か組に入った窓）→ project_anchor 単一ライター（Free は恒等）／
            // wintf が書く・向き不在 → wndproc 委譲（不在は旧「marker 無し＝Free」意味論の保存・安全側）。
            let anchor = drag_anchor(world, entity);
            let pos = match anchor {
                Some(anchor) if !drag_wintf_moves(world, entity) => {
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
                    let Some(pos) = world.get::<WindowPos>(entity).and_then(|wp| wp.position)
                    else {
                        return false;
                    };
                    pos
                }
            };

            // 引き金はユーザーの明示的なドラッグ＝遷移ガード適用外（task 6.2・Req 3.1）。
            follow_balloon(world, entity, pos, BalloonFollowTrigger::Drag);
            follow_partners_on_drag(world, entity, PointPx { x: pos.x, y: pos.y });
            false
        }
    }
}

/// `OnDragEnd` ハンドラ: **全アンカー種別**のキャラ窓の最終カーソル位置へ同写像を
/// 適用し、確定位置を永続へ write-through する（4.7/1.6・1.1/1.9・DD15 v2 (3)・
/// design C2）。分岐は [`drag_anchor`]（ドラッグの始めに控えた向き）で判定する。
///
/// wintf の accumulator は LBUTTONUP で `current_dragging_entity` を先にクリア
/// するため、最終カーソル位置の DragEvent は配送されない（debug 調査 2026-07-11）。
/// この穴を、`dispatch_drag_events` の Ended 分岐が配送する DragEndEvent
/// （最終カーソル位置持ち）で埋める。[`DraggingState`] はハンドラ配送**後**に
/// remove されるため、ここではまだ読める（実 flow 準拠）。
///
/// cancel（ESC 等・`ev.cancelled=true`）も同写像で確定する。wintf は取り消しの
/// `DragEndEvent.position` に押した位置を載せるので、ここで窓の位置を引き直すと
/// 結果として開始の位置へ戻る（`input_events/drag_tests.rs` の
/// `cancelled_drag_end_reports_the_start_coordinates` が固定）。
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
/// # 組の相手（4.5・2.4）
///
/// 窓を書いた後に [`finish_char_drag`] が最終の量で相手を書いて記憶へ書き、控えを外し、
/// ドラッグ中に向きが変わっていれば今の向きで当て直す。
///
/// # 保存フック（1.1/1.9/7.1・design C2）
///
/// `mapped` 確定・`enqueue_window_set_pos`・`follow_balloon` の**後**に、当該窓の
/// [`CharWindowMarker`]`.scope`（`usize`→`u32`）で `mapped` を [`saved_form_anchor`]
/// （descript の向き＝次の起動の読み戻しの向き・4.8）の形にして [`char_pos_entries`]→
/// [`persist_entries`] へ渡し、Ghost 永続スコープへ即時 write-through する（fire-and-forget・
/// 非ブロッキング）。marker 不在（防御）は「位置を保存しなかった」の `warn!`＋skip（panic しない）。キャラ窓の位置を記憶に書く時機は、
/// このドラッグの確定と、起動の最後に並べ終えた時点（[`crate::persist::persist_unremembered_char_positions`]）の
/// 2 つ——`on_char_drag`（ドラッグ中）・[`move_window_to`]・[`resize_window_to`]・復元時再射影は書かない。
pub fn on_char_drag_end(
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
            let Some(anchor) = drag_anchor(world, entity) else {
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
                    let Some(pos) = world.get::<WindowPos>(entity).and_then(|wp| wp.position)
                    else {
                        warn!(
                            ?entity,
                            "DraggingState も WindowPos.position も無いため位置を保存しなかった"
                        );
                        world.remove_resource::<CharDragSession>(); // 控えを残さない
                        return false;
                    };
                    pos
                }
            };
            if !enqueue_window_set_pos(world, entity, mapped.x, mapped.y, None, None) {
                world.remove_resource::<CharDragSession>();
                return false;
            }
            // 面の move のずれは、利用者が置いた所を元の位置として 0 から（task 6.2）。
            reset_applied(world, entity);
            // 引き金はユーザーの明示的なドラッグ＝遷移ガード適用外（task 6.2・Req 3.1）。
            follow_balloon(world, entity, mapped, BalloonFollowTrigger::Drag);
            finish_char_drag(
                world,
                entity,
                PointPx {
                    x: mapped.x,
                    y: mapped.y,
                },
            );

            // 保存フック（1.1/1.9/7.1・design C2）: mapped 確定後に当該スコープの
            // WindowPos entries を Ghost 永続スコープへ即時 write-through 投函する。
            // スコープは CharWindowMarker から逆引き（usize→u32）。marker 不在（防御）は
            // warn＋skip（panic しない）。ドラッグ中は書かない（書く時機は上の説明の 2 つ）。
            match world.get::<CharWindowMarker>(entity).map(|m| m.scope) {
                Some(scope) => {
                    // 原点（下端中央）基準へ移してから保存する。左上 x のまま保存すると、
                    // サーフェス寸が変わったとき「同じ左上」が別の中央を指し、復元で
                    // キャラ・バルーンが横へずれる（実機: むらさき 382 で保存→434 で復元）。
                    // 現寸が読めないときは左上のまま（防御・従来挙動）。
                    let char_size = world
                        .get::<WindowPos>(entity)
                        .and_then(|wp| wp.size)
                        .filter(|s| s.width > 0 && s.height > 0)
                        .map(|s| SizePx {
                            w: s.width,
                            h: s.height,
                        });
                    // 形は descript の向き（次の起動はそれで読み戻す・4.8）。
                    let form = saved_form_anchor(world, entity).unwrap_or(anchor);
                    let saved = match char_size {
                        Some(size) => char_pos_to_origin_x(
                            form,
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
                    warn!(?entity, "CharWindowMarker が無いため位置を保存しなかった");
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
/// 復元した生座標へ [`project_anchor`] を適用する——リサイズ（[`resize_window_to`]）と同一の
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
    // 始めの位置が分からない（`None`）なら復元できないので写さない（振る舞いは 164 の追随で直す）
    let origin = ds.initial_window_pos?;
    let raw = PointPx {
        // 実カーソル・窓座標の範囲で溢れないが、極端入力でも panic しない契約
        x: (origin.0 as i32).saturating_add(cursor.x.saturating_sub(ds.drag_start_pos.x)),
        y: (origin.1 as i32).saturating_add(cursor.y.saturating_sub(ds.drag_start_pos.y)),
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
    // 二重化しない（Req1.6）。bottom は project_anchor 内で下端吸着へ委譲。
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
pub enum BalloonFollowTrigger {
    /// ユーザーの明示的なドラッグ（[`on_char_drag`]／[`on_char_drag_end`]）。
    /// 引き戻しは明示操作の否定ゆえ**ガード適用外**（requirements.md Boundary Context）。
    Drag,
    /// 配置系の書込（[`resize_window_to`]）に随伴した。内包する route は**キャラ窓を
    /// 動かした経路**であり、適用可否はキャラ窓と同じ表（D13 帰結⑴⑵）で決まる。
    Placement(PlacementRoute),
}

impl BalloonFollowTrigger {
    /// この引き金の随伴でバルーン矩形へ遷移ガードを適用するか。
    pub(super) fn applies_visibility_guard(self) -> bool {
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
/// **後**に、キャラ窓とまったく同一の純関数（`guard_visibility`）・同一の遷移規則を
/// バルーン矩形へ適用する（[`guard_balloon_position`]）。発火可否は書込自身の route では
/// なく `trigger`（随伴の引き金）が決める。
pub fn follow_balloon(
    world: &mut World,
    entity: Entity,
    pos: Point,
    trigger: BalloonFollowTrigger,
) {
    let Some(follow) = world.get::<BalloonFollow>(entity).copied() else {
        return;
    };
    // 不変条件: pos は仮想スクリーン座標範囲・offset は配置時確定の
    // 有限値のため、加算が i32 を溢れることはない（溢れは入力源の異常）。
    debug_assert!(
        pos.x.checked_add(follow.offset().x).is_some()
            && pos.y.checked_add(follow.offset().y).is_some(),
        "char window position out of virtual-screen range: {pos:?} + {:?}",
        follow.offset()
    );
    // 相対位置の恒等式（4.4: `balloon_pos − char_pos ≡ offset`）に面の `move` のずれを足した提案位置。
    let shift = balloon_shift(world, follow.balloon);
    let proposed = PointPx {
        x: pos
            .x
            .saturating_add(follow.offset().x)
            .saturating_add(shift.x),
        y: pos
            .y
            .saturating_add(follow.offset().y)
            .saturating_add(shift.y),
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
///   決める基準が存在しない（別の矩形から引き直すと `guard_visibility` の事後条件が崩れる）。
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
/// 本 spec の対象外（α 後）であり、**`ClampX` の `warn!` がその先送りの縮退シーム**である
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
    // （wintf 正典 `window_pos.rs` の `apply_window_pos_changes`／`monitor_systems.rs` の
    // `window_center` と同型——正典が見るのは `position.x` で、ここは位置の両軸を見る点だけが異なる。
    // 素通しすると `WindowPos::default()` 由来の窓が「もともと画面外に留置」と誤判定され安全側の腕が死ぬ）。
    let old_rect = window_pos
        .and_then(known_position)
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

/// 窓の位置。`CW_USEDEFAULT` の印（窓生成前・位置未確定）は無いのと同じに扱う。
pub(super) fn known_position(wp: &WindowPos) -> Option<Point> {
    wp.position
        .filter(|p| p.x != CW_USEDEFAULT && p.y != CW_USEDEFAULT)
}
