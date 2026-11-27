//! バルーン単独ドラッグ（[`on_balloon_drag`]・[`on_balloon_drag_end`] ほか）。`drag_follow` から分けた塊。

use bevy_ecs::prelude::*;
use tracing::{debug, info, warn};
use windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT;
use wintf::ecs::drag::{DragEndEvent, DragEvent};
use wintf::ecs::pointer::Phase;
use wintf::ecs::{DPI, Point, WindowPos};

use super::super::balloon_lock::{balloon_drag_held, end_balloon_drag};
use super::drag_follow::known_position;
use super::{
    Anchor, Anchored, BALLOON_LIMIT_CLAMP_TAG, BALLOON_LIMIT_RELEASE_CONTEXT,
    BALLOON_LIMIT_UNRESOLVED_TAG, BalloonFollow, BalloonKeywordBase, BalloonLimit,
    BalloonWindowMarker, OffsetBase, PlacementRoute, PointPx, SizePx, balloon_offset_entries,
    enqueue_window_set_pos, limit_correction, limit_work_area_in, persist_entries, rect_at,
    reset_applied,
};

/// バルーン単独ドラッグで確定した相対位置を、キャラ窓の表示 DPI を読めないまま
/// **未係留**の基準として確立したときの記録タグ。
///
/// `[balloon-drag] ` 接頭辞＋弁別語 1 語という形は
/// [`BALLOON_LIMIT_UNRESOLVED_TAG`]・`[balloon-keyword] ` 系と同じ規律であり、
/// 決定論テストがこの定数を逐語で固定する（生リテラルを書かない）。
pub(super) const BALLOON_DRAG_BASE_UNPINNED_TAG: &str = "[balloon-drag] BaseUnpinned";

/// `OnDrag` ハンドラ: バルーン窓単独ドラッグの相対位置記憶。
///
/// バルーン窓自体は `DragConfig { move_window: true }` により wndproc レベルで
/// 移動済み（`WindowPos.position` 更新済み・物理 px）。キャラ窓は不動で、
/// bottom 吸着の対象外＝バルーンドラッグでキャラ窓の Y 釘付けは発火しない。
///
/// 本ハンドラは `BalloonFollow.balloon == ドラッグ中バルーン` のキャラ窓を
/// query 走査で逆引きし（窓は高々数個・全走査で十分）、
/// `BalloonFollow.offset = balloon_pos − char_pos`（物理 px・再スケールなし）
/// へ更新するだけ。既存 consumer（[`on_char_drag`]／[`move_window_to`]）は
/// 無改変で調整後 offset を読む（`balloon_pos − char_pos ≡ offset` は更新後も不変）。
/// ghost.dat への保存は離したときの `on_balloon_drag_end` が行う。
///
/// イベントは消費しない（常に `false`＝伝播続行。[`on_char_drag`] と同じ規約）。
pub fn on_balloon_drag(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<DragEvent>,
) -> bool {
    match ev {
        Phase::Tunnel(_) => false,
        Phase::Bubble(ev) => {
            // 他 entity 宛イベントには何もしない
            if ev.target != entity {
                return false;
            }
            // 止めたドラッグは追従のずれを引き直さない
            if balloon_drag_held(world, entity) {
                return false;
            }
            let Some(wp) = world.get::<WindowPos>(entity) else {
                return false;
            };
            let Some(balloon_pos) = wp.position else {
                return false;
            };

            reestablish_balloon_follow(world, entity, balloon_pos);
            false
        }
    }
}

/// `BalloonFollow.balloon == balloon` のキャラ窓を逆引きし、追従の基準を `balloon_pos − char_pos`
/// から確立し直す。ドラッグ中（[`on_balloon_drag`]）と、離したときの保存の直後が呼ぶ。
fn reestablish_balloon_follow(world: &mut World, balloon: Entity, balloon_pos: Point) {
    // 読取だけで閉じる（可変借用は次の段で 1 窓ずつ取る）。
    let mut chars = world.query::<(Entity, &BalloonFollow, &WindowPos)>();
    let targets: Vec<(Entity, PointPx)> = chars
        .iter(world)
        .filter(|(_, follow, _)| follow.balloon == balloon)
        .filter_map(|(char_window, _, char_wp)| {
            let Some(char_pos) = known_position(char_wp) else {
                warn!(
                    entity = ?char_window,
                    ?balloon,
                    position = ?char_wp.position,
                    "追従元キャラ窓の位置が無い・未確定（CW_USEDEFAULT の印）ため追従の基準を確立し直さない"
                );
                return None;
            };
            // 両者とも仮想スクリーン座標範囲のため、減算が i32 を溢れることはない
            // （溢れは入力源の異常）
            debug_assert!(
                balloon_pos.x.checked_sub(char_pos.x).is_some()
                    && balloon_pos.y.checked_sub(char_pos.y).is_some(),
                "window positions out of virtual-screen range: {balloon_pos:?} - {char_pos:?}"
            );
            Some((
                char_window,
                PointPx {
                    x: balloon_pos.x - char_pos.x,
                    y: balloon_pos.y - char_pos.y,
                },
            ))
        })
        .collect();

    for (char_window, new_offset) in targets {
        super::super::face_windowposition::mark_user_moved(world, char_window);
        // 同時借用を避けるため表示 DPI を先に読んでから追従 Component を可変で借りる。
        let current_dpi = world.get::<DPI>(char_window).copied();
        let Some(mut follow) = world.get_mut::<BalloonFollow>(char_window) else {
            continue;
        };
        match current_dpi {
            // ドラッグで決まった相対位置を新しい**基準**として焼き直す。
            // 作者指定と同一の追随規則がここから効く。
            Some(dpi) => follow.reestablish(new_offset, dpi),
            None => {
                // 表示 DPI を読めない窓（DPI component 未付与）。値は確定して
                // いるが「どの表示 DPI に属するか分からない」ので、永続値の腕と
                // 同じ**未係留**の基準として確立する——基準 DPI を発明すると
                // 次の遷移で二重に拡大する。最初の観測で値を変えずに係留される。
                *follow = BalloonFollow::new(balloon, OffsetBase::unpinned(new_offset));
                warn!(
                    entity = ?char_window,
                    balloon = ?balloon,
                    offset = ?new_offset,
                    "{BALLOON_DRAG_BASE_UNPINNED_TAG} キャラ窓の表示 DPI を読めないため追従オフセットの基準を未係留で確立した（最初の観測で係留される）"
                );
            }
        }
    }
}

/// `OnDragEnd` ハンドラ: バルーン単独ドラッグ確定 offset の永続 write-through。
///
/// [`on_balloon_drag`] が**連続**イベント（ドラッグ中の in-session offset 更新）で
/// あるのに対し、本ハンドラは DragEnd の**確定**観測点でのみ発火する保存トリガである
/// （1 ドラッグ＝1 書込）。永続へバルーン offset を書くのはこの観測点のみ。
///
/// # 最終確定位置の源（`on_char_drag_end` との差）
///
/// バルーン窓は `DragConfig { move_window: true }` ゆえ wndproc が実窓位置を
/// `WindowPos.position` へ更新済み——DragEnd 時点の**最終確定位置**はこの
/// `WindowPos.position` で読める（[`on_balloon_drag`] と同源）。したがって
/// [`on_char_drag_end`] のように `DraggingState`＋`ev.position`（カーソル）から生座標を
/// 再構成する必要はなく、`ev.position` は使わない。
///
/// # 保存値の導出
///
/// 追従元キャラ窓（`BalloonFollow.balloon == 自バルーン`）を逆引きし、その最終
/// `char_pos`（`WindowPos.position`）・`anchor`（[`Anchored`]）・`char_size`
/// （`WindowPos.size`）を読む。offset は [`on_balloon_drag`] と**同式**
/// `offset_tl = balloon_pos − char_pos`（左上基準・物理 px・再スケールなし）で
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
/// （その `WindowPos.position`・`WindowPos.size` のいずれか）不在は「位置を保存しなかった」の
/// `warn!`＋skip する（[`Anchored`] は記録だけが使うので無くても保存する）。
/// `BalloonFollow.offset`（in-session 表現）は保存の直後に最終確定位置から
/// 出し直す（補正の前の位置・[`reestablish_balloon_follow`]）。最後の OnDrag の値のまま残さない。
///
/// # 解放時の limit 補正と**順序の固定**
///
/// バルーン単独ドラッグは wndproc が窓を直接動かす ECS 経路**外**の移動であり、
/// [`enqueue_window_set_pos`] の runtime 関門を一度も通らない。よって解放の瞬間だけが
/// 「はみ出したまま静止した位置」を捕まえられる唯一の観測点であり、ここに 3 つ目の
/// 関門（[`apply_release_limit_correction`]）を置く。ドラッグ**中**（[`on_balloon_drag`]）
/// へは一切介入しない——カーソル追随の自由を守る。
///
/// 本ハンドラの処理順は**固定**である:
///
/// 1. 解放位置の取得（`WindowPos.position`＝wndproc の最終確定位置）
/// 2. 生 offset（`balloon_pos − char_pos`）の導出と永続 write-through
///    2′. 同じ最終確定位置から追従の基準を出し直す（[`reestablish_balloon_follow`]）
/// 3. キーワード素材の退役（[`retire_keyword_base_on_save`]）
/// 4. `BalloonLimit(true)` かつ補正が要るときだけ表示位置を補正
///
/// **2 と 4 を入れ替えてはならない。** 先に補正すると、4 が書いた補正後位置を 2 が
/// `balloon_pos` として読み、補正値が保存値（`BalloonOffset`）へ焼き付く——以後キャラ窓が
/// 作業領域の余裕ある位置へ戻っても、作者指定・保存の相対位置へ復帰できなくなる。
/// 同じ理由で 4 は `BalloonFollow.offset`（in-session 表現）にも触れない。次のバルーンドラッグ開始時の
/// offset は、補正済みの表示位置から [`on_balloon_drag`] が自然に再導出する。
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
            // 他 entity 宛イベントには何もしない
            if ev.target != entity {
                return false;
            }
            // 止めたドラッグは記憶・素材・補正を動かさず記録 1 件だけ
            if end_balloon_drag(world, entity) {
                return false;
            }
            // 面の move のずれは 0 へ（ドラッグで作った相対の位置へ二重に足さない）。
            reset_applied(world, entity);

            let Some(balloon_pos) = world.get::<WindowPos>(entity).and_then(|wp| wp.position)
            else {
                warn!(
                    ?entity,
                    "バルーン窓の WindowPos.position が無いため位置を保存しなかった"
                );
                return false;
            };

            let Some(scope) = world.get::<BalloonWindowMarker>(entity).map(|m| m.scope) else {
                warn!(
                    ?entity,
                    "BalloonWindowMarker が無いためバルーンの位置を保存しなかった"
                );
                return false;
            };

            // anchor は記録だけが使うので任意（無くても保存する）。
            let mut chars =
                world.query::<(Entity, &BalloonFollow, &WindowPos, Option<&Anchored>)>();
            let mut found: Option<(Entity, Point, Option<Anchor>, SizePx)> = None;
            for (char_window, follow, char_wp, anchored) in chars.iter(world) {
                if follow.balloon != entity {
                    continue;
                }
                let Some(char_pos) = known_position(char_wp) else {
                    continue;
                };
                let Some(size) = char_wp.size else {
                    continue;
                };
                found = Some((
                    char_window,
                    char_pos,
                    anchored.map(|a| a.0),
                    SizePx {
                        w: size.width,
                        h: size.height,
                    },
                ));
                break;
            }
            let Some((char_window, char_pos, anchor, char_size)) = found else {
                warn!(
                    ?entity,
                    "追従元キャラ窓（位置/寸法）が無い・位置が未確定のためバルーンの位置を保存しなかった"
                );
                return false;
            };

            // 両者とも仮想スクリーン座標範囲のため減算は i32 を溢れない（溢れは入力源の異常）。
            debug_assert!(
                balloon_pos.x.checked_sub(char_pos.x).is_some()
                    && balloon_pos.y.checked_sub(char_pos.y).is_some(),
                "window positions out of virtual-screen range: {balloon_pos:?} - {char_pos:?}"
            );
            let offset_tl = PointPx {
                x: balloon_pos.x - char_pos.x,
                y: balloon_pos.y - char_pos.y,
            };

            // 保存基準＝ランタイム基準（char 左上）ゆえ基準変換なし（balloon_offset_entries の基準記述）。
            let persist = offset_tl;
            // 保存↔復元の座標突合に使う実機診断ログ。char_size は診断用の現寸。
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

            // 2′: 追従の基準を最終確定位置から出し直す（補正の前・最後の OnDrag の値を残さない）。
            reestablish_balloon_follow(world, entity, balloon_pos);

            // 3: キーワード素材の退役。2 と同じ観測点であること自体が要点なので、2 の直後に置く
            // （詳細は [`retire_keyword_base_on_save`]）。
            retire_keyword_base_on_save(world, char_window, entity, scope);

            // 4: 解放時補正。2 の永続 write-through より後であることが順序の要点であり、
            // この 1 行が上へ動いた瞬間に補正値が保存値へ焼き付く（本関数 doc「順序の固定」）。
            apply_release_limit_correction(
                world,
                entity,
                scope,
                balloon_pos,
                char_window,
                char_pos,
                char_size,
            );
            false
        }
    }
}

/// 保存された相対位置が生まれた瞬間に、キーワード由来の基本位置の素材を退役させる。
///
/// # 何を直しているのか
///
/// 永続化されたバルーン相対位置が**存在する間**は永続値を優先し、キーワード指定の適用は
/// 初期既定位置の供給にとどめる。この規則は `persist::merge_scope`（＝**起動時**に読み込んだ
/// 保存値の有無）にあるが、保存された相対位置はセッション中にも生まれる——それがまさに
/// [`on_balloon_drag_end`] の手順 2 である。対応する退役点が無いと、次にキャラ窓の寸が変わった書込で
/// [`super::keyword_base::rederive_keyword_balloon_offset`] が発火し、利用者がドラッグで
/// 決めた相対位置をキーワード既定へ**上書きする**（保存値優先の順位が静かに反転する）。
///
/// 実表示寸確定の再解決は同寸の書込を skip するため、採寸寸と実表示寸が一致する
/// **ふつうの**ゴーストでは素材が起動時に一度も消費されず、装填されたまま残り続ける
/// ——面替え・再吸着・DPI 移動のどれかが来た瞬間に発火する。
///
/// # なぜ退役点が [`on_balloon_drag`]（連続）ではなく DragEnd（確定）なのか
///
/// 「永続化された」状態が生まれる唯一の観測点が DragEnd の write-through である。
/// 同じ事象が保存値を作り、同じ事象が素材を退役させる——起動時に
/// `persist::merge_scope` が保存値の存在をもって素材を落とすのと**同一の規則**であり、
/// 二つの退役点が別々の条件で動く形にしない。
///
/// ドラッグ**中**に再導出が走っても被害は残らない（次のカーソル移動で [`on_balloon_drag`] が
/// offset を引き直し、続く DragEnd がその値を保存する）。確定側の穴だけがセッションが終わるまで
/// 残る。加えて連続イベントごとに構造変更（Component 除去）を試みるのは、`BalloonLimit` と同じ
/// 「持たない窓は 1 bit も触らない」流儀にも反する。
///
/// # 縮退を持たない理由
///
/// 追従元キャラ窓が解決できない場合は、呼出元が**手前で**既に `warn!`＋skip している。
/// ここへ到達した時点で対象は確定している。素材を持たない窓（`Side`・保存値が効いた scope・
/// 消費済み）は縮退ではなく**正常な no-op** ゆえ警告も出さない——出すと `Side` のバルーンを
/// ドラッグするたびに良性の警告が積み上がり、本物の異常を埋める。
fn retire_keyword_base_on_save(
    world: &mut World,
    char_window: Entity,
    balloon: Entity,
    scope: usize,
) {
    // 実際に装填されていたときだけ記録する。
    if world.get::<BalloonKeywordBase>(char_window).is_none() {
        return;
    }
    world.entity_mut(char_window).remove::<BalloonKeywordBase>();
    debug!(
        scope,
        entity = ?char_window,
        ?balloon,
        "バルーン相対位置が保存されたのでキーワード基本位置の素材を退役させた（areka-P0-windowposition-limit 要件 4.7・以後は保存値優先）"
    );
}

/// バルーン単独ドラッグの解放位置を作業領域内へ引き戻す。**[`on_balloon_drag_end`] の永続
/// write-through より後**に呼ばれることが契約である（呼出点の doc 参照）。
///
/// # 発動条件はデータ駆動（runtime 関門と同一）
///
/// 対象が [`BalloonLimit`]`(true)` を持つときだけ働く。持たない窓と `false` の窓は
/// 1 bit も触らずに戻る——縮退ではないので警告も出さない。
///
/// # 基準領域は runtime 関門と同じ選び方
///
/// 呼出元が保存値の導出で既に読み終えている追従元キャラ窓の位置・寸と、解放位置×バルーンの寸を
/// [`limit_work_area_in`] へ渡す（台本の値の無いスコープはキャラ窓の帰属モニタ、あるスコープは
/// 解放位置の矩形の帰属モニタ）。帰属規則そのものは 1 bit も変えない。runtime 関門のように
/// `GhostWindows` を引き直さないのは、解放時点の追従元は保存値の導出で**既に**逆引き
/// 済みであり、同じ World を二度引くと保存に使った char と補正の基準 char が乖離し得る
/// からである（同一 DragEnd 内では同じ char を見る）。
///
/// # 書き込む位置と route
///
/// 補正後位置を [`enqueue_window_set_pos`] へ [`PlacementRoute::BalloonLimitRelease`] で
/// 渡す。この書込はドラッグ随伴でも位置据置きリサイズでもない**独立した新規書込**ゆえ
/// 専用 route を名乗る（関門内の補正が元書込の route を保つのとは逆の扱い）。単一ライター
/// 側の runtime 関門は同じ位置をもう一度クランプするが、クランプは**冪等**ゆえ二重適用は
/// 無害であり、[`limit_correction`] が `None` を返して二重のログも出ない。
///
/// # 何に触れないか
///
/// `BalloonFollow.offset` と永続値は解放時の**生値**のまま。本関数は表示位置しか書かない。
///
/// # 縮退（log-first）
///
/// 判定寸・基準作業領域のいずれかが解決できないときは
/// [`BALLOON_LIMIT_UNRESOLVED_TAG`] の `warn!` を残して補正を見送る（無ログの縮退経路を
/// 作らない）。補正が実際に位置を動かしたときだけ [`BALLOON_LIMIT_CLAMP_TAG`] の `info!`
/// を出す——[`limit_correction`] の `None` は「クランプしても位置が動かない」の意であって
/// 「内包されている」ではない。
fn apply_release_limit_correction(
    world: &mut World,
    balloon: Entity,
    scope: usize,
    release_pos: Point,
    char_window: Entity,
    char_pos: Point,
    char_size: SizePx,
) {
    // limit 無効・Component 非保持は 1 bit も触らず戻る。
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

    // 位置のセンチネル・非正寸は矩形を組めない。
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
    let picked =
        char_rect.and_then(|r| limit_work_area_in(world, char_window, r, pos, balloon_size));
    let Some((area, monitor, script)) = picked else {
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
        monitor = monitor.as_str(),
        script,
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
