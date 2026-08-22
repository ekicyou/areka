//! ドラッグ＋バルーン追従（[`BalloonFollow`]・[`on_char_drag`]・[`follow_balloon`]・[`on_balloon_drag`] ほか）。

use bevy_ecs::prelude::*;
use tracing::{debug, info, warn};
use windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT;
use wintf::ecs::drag::{DragEndEvent, DragEvent, DraggingState};
use wintf::ecs::pointer::Phase;
use wintf::ecs::{Point, WindowPos};

use super::{
    Anchor, Anchored, BALLOON_LIMIT_CLAMP_TAG, BALLOON_LIMIT_RELEASE_CONTEXT,
    BALLOON_LIMIT_UNRESOLVED_TAG, BalloonKeywordBase, BalloonLimit, BalloonWindowMarker,
    CharWindowMarker, DESPAWNED_SKIP_TAG, MonitorSnapshot, PlacementRoute, PointPx, SizePx,
    VISIBILITY_UNRESOLVED_TAG, balloon_offset_entries, char_pos_entries, char_pos_to_origin_x,
    enqueue_window_set_pos, evaluate_visibility_guard, limit_correction, persist_entries,
    project_anchor, rect_at, route_applies_visibility_guard, work_area_for_window,
};

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
                    let Some(pos) = world.get::<WindowPos>(entity).and_then(|wp| wp.position)
                    else {
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
                    let Some(pos) = world.get::<WindowPos>(entity).and_then(|wp| wp.position)
                    else {
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
                    let char_size =
                        world
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
        x: (ds.initial_inset.0 as i32).saturating_add(cursor.x.saturating_sub(ds.drag_start_pos.x)),
        y: (ds.initial_inset.1 as i32).saturating_add(cursor.y.saturating_sub(ds.drag_start_pos.y)),
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
pub(super) enum BalloonFollowTrigger {
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
/// **後**に、キャラ窓とまったく同一の純関数（[`guard_visibility`]）・同一の遷移規則を
/// バルーン矩形へ適用する（[`guard_balloon_position`]）。発火可否は書込自身の route では
/// なく `trigger`（随伴の引き金）が決める。
pub(super) fn follow_balloon(
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
/// # 解放時の limit 補正と**順序の固定**（areka-P0-windowposition-limit C8・要件 2.5/5.4）
///
/// バルーン単独ドラッグは wndproc が窓を直接動かす ECS 経路**外**の移動であり、
/// [`enqueue_window_set_pos`] の runtime 関門を一度も通らない。よって解放の瞬間だけが
/// 「はみ出したまま静止した位置」を捕まえられる唯一の観測点であり、ここに 3 つ目の
/// 関門（[`apply_release_limit_correction`]）を置く。ドラッグ**中**（[`on_balloon_drag`]）
/// へは一切介入しない——カーソル追随の自由は要件 2.5 前段が明示的に守るものである。
///
/// 本ハンドラの処理順は**固定**である（DD6・5.4）:
///
/// 1. 解放位置の取得（`WindowPos.position`＝wndproc の最終確定位置）
/// 2. 生 offset（`balloon_pos − char_pos`）の導出と永続 write-through
/// 3. キーワード素材の退役（[`retire_keyword_base_on_save`]・要件 4.7）
/// 4. `BalloonLimit(true)` かつ補正が要るときだけ表示位置を補正
///
/// **2 と 4 を入れ替えてはならない。** 先に補正すると、4 が書いた補正後位置を 2 が
/// `balloon_pos` として読み、補正値が保存値（`BalloonOffset`）へ焼き付く——以後キャラ窓が
/// 作業領域の余裕ある位置へ戻っても、作者指定・保存の相対位置へ復帰できなくなる
/// （DD6「補正を相対位置へ焼き付けない」）。同じ理由で 4 は `BalloonFollow.offset`
/// （in-session 表現）にも触れない。次のバルーンドラッグ開始時の offset は、補正済みの
/// 表示位置から [`on_balloon_drag`] が自然に再導出する。
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
            let Some(balloon_pos) = world.get::<WindowPos>(entity).and_then(|wp| wp.position)
            else {
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
            let mut chars = world.query::<(Entity, &BalloonFollow, &WindowPos, &Anchored)>();
            let mut found: Option<(Entity, Point, Anchor, SizePx)> = None;
            for (char_window, follow, char_wp, anchored) in chars.iter(world) {
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
                    char_window,
                    char_pos,
                    anchored.0,
                    SizePx {
                        w: size.width,
                        h: size.height,
                    },
                ));
                break;
            }
            let Some((char_window, char_pos, anchor, char_size)) = found else {
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

            // ③ キーワード素材の退役（要件 4.7・task 4.6）。**②と同じ観測点**であること
            // 自体が要点なので、②の直後に置く（詳細は [`retire_keyword_base_on_save`]）。
            retire_keyword_base_on_save(world, char_window, entity, scope);

            // ④ 解放時補正（C8・要件 2.5/5.4）。**②の永続 write-through より後**である
            // ことが順序の要点であり、この 1 行が上へ動いた瞬間に補正値が保存値へ
            // 焼き付く（本関数 doc「順序の固定」）。
            apply_release_limit_correction(world, entity, scope, balloon_pos, char_pos, char_size);
            false
        }
    }
}

/// 保存された相対位置が生まれた瞬間に、キーワード由来の基本位置の素材を退役させる
/// （areka-P0-windowposition-limit 要件 4.7・task 4.6）。
///
/// # 何を直しているのか
///
/// 要件 4.7 は While 節（状態）である——「永続化されたバルーン相対位置が**存在する間**は
/// 永続値を優先し、キーワード指定の適用は初期既定位置の供給にとどめる」。task 4.5 は
/// この規則を `persist::merge_scope`（＝**起動時**に読み込んだ保存値の有無）にだけ置いた。
/// しかし保存された相対位置はセッション中にも生まれる——それがまさに本ハンドラの手順②
/// である。対応する退役点が無いと、次にキャラ窓の寸が変わった書込で
/// [`super::keyword_base::rederive_keyword_balloon_offset`] が発火し、利用者がドラッグで
/// 決めた相対位置をキーワード既定へ**上書きする**（保存値優先の順位が静かに反転する）。
///
/// これは隅の症状ではない。実表示寸確定の再解決は同寸の書込を skip するため、採寸寸と
/// 実表示寸が一致する**ふつうの**ゴーストでは素材が起動時に一度も消費されず、装填された
/// まま残り続ける——面替え・再吸着・DPI 移動のどれかが来た瞬間に発火する。
///
/// # なぜ退役点が [`on_balloon_drag`]（連続）ではなく DragEnd（確定）なのか
///
/// 要件 4.7 の条件は「**永続化された**相対位置が存在する間」であり、その状態が生まれる
/// 唯一の観測点が DragEnd の write-through である（1 ドラッグ＝1 書込・発火規律）。
/// 「同じ事象が保存値を作り、同じ事象が素材を退役させる」——起動時に
/// `persist::merge_scope` が保存値の存在をもって素材を落とすのと**同一の規則**であり、
/// 二つの退役点が別々の条件で動く形にしない。
///
/// [`on_balloon_drag`] にも置かない理由はもう一つある。ドラッグ**中**に寸法変化が挟まって
/// 再導出が走った場合でも、被害は残らない——利用者はまだバルーンを掴んでおり、次の
/// カーソル移動で [`on_balloon_drag`] が `balloon_pos − char_pos` から offset を引き直し、
/// 続く DragEnd がその値を保存する。すなわち連続側の穴は自己修復するのに対し、確定側の
/// 穴（本欠陥）はセッションが終わるまで残る。加えて連続イベントごとに構造変更
/// （Component 除去）を試みるのは、`BalloonLimit` と同じ「持たない窓は 1 bit も触らない」
/// 流儀にも反する。
///
/// # 縮退を持たない理由（要件 6.3 の適用範囲）
///
/// 追従元キャラ窓が解決できない場合は、呼出元が**手前で**既に `debug!`＋skip している
/// （保存値の導出に同じ entity が要る）。ここへ到達した時点で対象は確定しており、解決に
/// 失敗する余地が無い。素材を持たない窓（`Side`・保存値が効いた scope・消費済み）は縮退
/// ではなく**正常な no-op** ゆえ警告も出さない——出すと `Side` のバルーンをドラッグする
/// たびに良性の警告が積み上がり、本物の異常を埋める。
fn retire_keyword_base_on_save(
    world: &mut World,
    char_window: Entity,
    balloon: Entity,
    scope: usize,
) {
    // 実際に装填されていたときだけ記録する（no-op を毎回書かない）。
    if world.get::<BalloonKeywordBase>(char_window).is_none() {
        return;
    }
    world.entity_mut(char_window).remove::<BalloonKeywordBase>();
    debug!(
        scope,
        entity = ?char_window,
        ?balloon,
        "バルーン相対位置が保存されたのでキーワード基本位置の素材を退役させた（要件 4.7・以後は保存値優先）"
    );
}

/// バルーン単独ドラッグの解放位置を作業領域内へ引き戻す（design「C8: 解放時補正」・
/// 要件 2.5／5.4／6.1／6.3）。**[`on_balloon_drag_end`] の永続 write-through より後**に
/// 呼ばれることが契約である（呼出点の doc 参照）。
///
/// # 発動条件はデータ駆動（DD1・runtime 関門と同一）
///
/// 対象が [`BalloonLimit`]`(true)` を持つときだけ働く。持たない窓と `false` の窓は
/// 1 bit も触らずに戻る——縮退ではないので警告も出さない（要件 2.7/2.8）。
///
/// # 基準領域はキャラ窓の帰属モニタ（要件 5.5）
///
/// 呼出元が保存値の導出で既に読み終えている追従元キャラ窓の位置・寸をそのまま受け取り、
/// [`work_area_for_window`] の既存の帰属規則（窓中心 half-open ＋最近傍フォールバック）へ
/// 渡すだけである。帰属規則そのものは 1 bit も変えない。runtime 関門のように
/// `GhostWindows` を引き直さないのは、解放時点の追従元は保存値の導出で**既に**逆引き
/// 済みであり、同じ World を二度引くと保存に使った char と補正の基準 char が乖離し得る
/// からである（同一 DragEnd 内では同じ char を見る）。
///
/// # 書き込む位置と route（DD7）
///
/// 補正後位置を [`enqueue_window_set_pos`] へ [`PlacementRoute::BalloonLimitRelease`] で
/// 渡す。この書込はドラッグ随伴でも位置据置きリサイズでもない**独立した新規書込**ゆえ
/// 専用 route を名乗る（関門内の補正が元書込の route を保つのとは逆の扱い）。単一ライター
/// 側の runtime 関門は同じ位置をもう一度クランプするが、クランプは**冪等**ゆえ二重適用は
/// 無害であり、[`limit_correction`] が `None` を返して二重のログも出ない。
///
/// # 何に触れないか（DD6）
///
/// `BalloonFollow.offset` と永続値は解放時の**生値**のまま。本関数は表示位置しか書かない。
///
/// # 縮退（log-first・要件 6.3）
///
/// 判定寸・基準作業領域のいずれかが解決できないときは
/// [`BALLOON_LIMIT_UNRESOLVED_TAG`] の `warn!` を残して補正を見送る（無ログの縮退経路を
/// 作らない）。補正が実際に位置を動かしたときだけ [`BALLOON_LIMIT_CLAMP_TAG`] の `info!`
/// を出す——[`limit_correction`] の `None` は「クランプしても位置が動かない」の意であって
/// 「内包されている」ではない（要件 6.1 は「実際に動かしたとき」の記録を求める）。
fn apply_release_limit_correction(
    world: &mut World,
    balloon: Entity,
    scope: usize,
    release_pos: Point,
    char_pos: Point,
    char_size: SizePx,
) {
    // 発動条件（DD1）: limit 無効・Component 非保持は 1 bit も触らず戻る。
    if !matches!(world.get::<BalloonLimit>(balloon), Some(BalloonLimit(true))) {
        return;
    }

    let pos = PointPx {
        x: release_pos.x,
        y: release_pos.y,
    };

    // 判定寸＝解放時点のバルーン現寸。未確定表現は `Option::None` **だけではない**
    // ——`WindowPos::default()` は `CW_USEDEFAULT`（`i32::MIN` センチネル）を寸に持つ
    // （`guard_balloon_position`／runtime 関門と同型のガード）。
    let balloon_size = world
        .get::<WindowPos>(balloon)
        .and_then(|wp| wp.size)
        .filter(|s| s.width > 0 && s.height > 0)
        .map(|s| SizePx {
            w: s.width,
            h: s.height,
        });
    let Some(balloon_size) = balloon_size else {
        warn!(
            entity = ?balloon,
            scope,
            context = BALLOON_LIMIT_RELEASE_CONTEXT,
            route = ?PlacementRoute::BalloonLimitRelease,
            ?pos,
            "{BALLOON_LIMIT_UNRESOLVED_TAG} 解放時補正: バルーンの寸が未確定（CW_USEDEFAULT センチネル含む）のため内包判定できない → 補正を見送る"
        );
        return;
    };

    // 基準はキャラ窓の帰属モニタ（要件 5.5）。位置のセンチネル・非正寸は矩形を組めない。
    let char_rect = (char_pos.x != CW_USEDEFAULT
        && char_pos.y != CW_USEDEFAULT
        && char_size.w > 0
        && char_size.h > 0)
        .then(|| {
            rect_at(
                PointPx {
                    x: char_pos.x,
                    y: char_pos.y,
                },
                char_size,
            )
        });
    let area = char_rect.and_then(|r| {
        world
            .get_resource::<MonitorSnapshot>()
            .and_then(|snapshot| work_area_for_window(snapshot, r))
    });
    let Some(area) = area else {
        warn!(
            entity = ?balloon,
            scope,
            context = BALLOON_LIMIT_RELEASE_CONTEXT,
            route = ?PlacementRoute::BalloonLimitRelease,
            ?pos,
            ?char_rect,
            "{BALLOON_LIMIT_UNRESOLVED_TAG} 解放時補正: キャラ窓矩形が組めないか MonitorSnapshot 未挿入／モニタ 0 台で基準作業領域を決められない → 補正を見送る"
        );
        return;
    };

    let Some(corrected) = limit_correction(pos, balloon_size, area) else {
        return;
    };

    info!(
        scope,
        context = BALLOON_LIMIT_RELEASE_CONTEXT,
        entity = ?balloon,
        route = ?PlacementRoute::BalloonLimitRelease,
        from = ?pos,
        to = ?corrected,
        balloon_size = ?balloon_size,
        work_area = ?area,
        "{BALLOON_LIMIT_CLAMP_TAG} 解放時補正: バルーンの解放位置を作業領域内へ補正した（保存済みの相対位置は生値のまま）"
    );
    enqueue_window_set_pos(
        world,
        balloon,
        corrected.x,
        corrected.y,
        None,
        Some(PlacementRoute::BalloonLimitRelease),
    );
}
