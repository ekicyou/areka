//! 記憶への送り口（[`PersistWiring`]・[`persist_entries`]・[`forget_entries`]）と、送る束へ足す
//! 隣り合いの印・並べ終えた時点の保存（[`persist_unremembered_char_positions`]）。
//! `persist.rs` の 1,000 行を越えないように分けた兄弟のファイルで、公開の道は `persist::` のまま。

use std::collections::BTreeSet;

use areka_sylphya::{PersistKey, PersistScope, SylphyaPublisher};
use bevy_ecs::world::World;
use tracing::{debug, info, warn};
use wintf::ecs::WindowPos;

use super::super::chain_finalize::{AdjacencyMark, ChainRect, ScriptPlaced, judge_adjacency};
use super::super::chain_realign::snapshot_char_rects;
use super::super::follow::origin_position;
use super::super::resolver::{PointPx, SizePx};
use super::super::spawn::GhostWindows;
use super::super::sticky_group_ledger::StickyGroupLedger;
use super::{
    RestoredPositionsClamped, adjacency_entry, char_pos_entries, char_pos_to_origin_x,
    saved_form_anchor,
};

// ---------------------------------------------------------------------------
// State Management（PersistWiring）＋保存投函ヘルパ（design C1・
// Req1.1 ドラッグ確定即時書込／1.9 発火規律／6.2 保存失敗縮退／7.1 Ghost スコープ固定）
// ---------------------------------------------------------------------------

/// UI スレッド常駐の保存投函口（**NonSend** リソース・`MouseWiring`／`RuntimeWiring` 先例）。
///
/// [`SylphyaPublisher`] は `Clone + Send`（内部 `std::sync::mpsc::Sender`）だが `Sync` を仮定せず、
/// UI スレッド専有の規律とも一致するため **NonSend** リソースとして World に持たせる（design C1
/// State Management・軸B）。書く呼び手 4 か所（モジュールの doc に並べる。3 か所は [`persist_entries`]、
/// 並べ終えた時点の保存はその本体 `send_entries`）が、この publisher の clone 送信端から保存 entries を
/// 投函する（[`forget_entries`] も同じ）。
pub struct PersistWiring {
    /// sylphya アクターへの変異投函の送信端（`persist_put` の fire-and-forget 投函に用いる）。
    pub publisher: SylphyaPublisher,
}

/// 保存 entries を Ghost 永続スコープへ write-through 投函するフック用ヘルパ（design C1・
/// Req1.1/7.1/6.2）。
///
/// World から [`PersistWiring`] NonSend リソースを引き、存在すれば
/// [`SylphyaPublisher::persist_put`]`(PersistScope::Ghost, entries)` を呼ぶ（**Ghost スコープ固定**＝
/// 当該ゴーストスコープのみ・Req7.1）。`persist_put` は fire-and-forget（reply なし）で**非ブロッキング**
/// ——UI スレッドに同期 IO を持ち込まない（write-through の実 IO 確定は sylphya アクタースレッドの領分・
/// commit 失敗は sylphya が `error!`＋`Degraded` に縮退し旧状態を破壊しない・design「persist_put 先の
/// commit 失敗」）。
///
/// [`PersistWiring`] 不在（例: fallback boot 経路が挿入しなかった場合）は `debug!` ＋ **no-op**で、
/// **panic しない**（6.2 系縮退・無音失敗なし）。`world` は共有参照で足りる（NonSend の読取のみ）——
/// DragEnd フックは `&mut World` を保持するが `&World` へ暗黙 reborrow して渡せる。
pub fn persist_entries(world: &World, entries: Vec<(PersistKey, String)>) {
    send_entries(world, entries, &BTreeSet::new());
}

/// [`persist_entries`] の本体。寄せた起動で古い記憶の印を外したら、外したスコープのうち `logged`
/// （呼び手がもう記録した組）に無いものを 1 行に残す（同じ決めを 2 行にしない）。
fn send_entries(world: &World, entries: Vec<(PersistKey, String)>, logged: &BTreeSet<usize>) {
    let Some(wiring) = world.get_non_send::<PersistWiring>() else {
        // fallback 未挿入等で PersistWiring が無い → debug!＋no-op（起動を止めない・6.2）。
        debug!(
            entry_count = entries.len(),
            "PersistWiring 不在: 保存投函を no-op（fallback boot 経路の縮退・6.2）"
        );
        return;
    };
    // 位置を書く束には触れる組の隣り合いの印を足す（書く時機は足さない・dpi-realign-remembered-chain 6.2）。
    let skip = clamped_old_memory(world, &entries);
    let mut entries = with_adjacency_marks(world, entries);
    let mut dropped = BTreeSet::new();
    entries.retain(|(key, _)| match key {
        PersistKey::WindowAdjacent { scope } if skip.contains(&(*scope as usize)) => {
            dropped.insert(*scope as usize);
            false
        }
        _ => true,
    });
    dropped.retain(|scope| !logged.contains(scope));
    if !dropped.is_empty() {
        info!(
            target: "areka::persist::save",
            scopes = ?dropped,
            "記憶の位置を寄せて表示した起動なので、古い記憶へ隣り合いの印を書き足さない"
        );
    }
    // Ghost スコープ固定（7.1）。fire-and-forget＝UI スレッド非ブロッキング（同期 IO を持ち込まない）。
    wiring.publisher.persist_put(PersistScope::Ghost, entries);
}

/// 記憶の位置を寄せて表示した起動（[`RestoredPositionsClamped`]）で、印の無い古い記憶のスコープの
/// うち `entries` が位置を書かないもの（window-work-area-clamp 要件 4.6）。並びは寄せた矩形なので、
/// [`persist_entries`] はこのスコープの印を束から外す。寄せずに起動した回は空。
// shortcut: 古い記憶かは起動の時点の台帳で決めるので、この起動でドラッグして印を書いたスコープも、
// 後で隣の窓を動かしたときは印を書き直さない。困る例が出たら、書いたスコープを台帳から外す。
fn clamped_old_memory(world: &World, entries: &[(PersistKey, String)]) -> BTreeSet<usize> {
    let clamped = world
        .get_resource::<RestoredPositionsClamped>()
        .is_some_and(|c| c.0);
    let Some(ghost_windows) = world.get_resource::<GhostWindows>().filter(|_| clamped) else {
        return BTreeSet::new();
    };
    ghost_windows
        .scopes()
        .filter(|&scope| {
            is_unmarked_old_memory(ghost_windows, scope)
                && !entries.iter().any(|(key, _)| {
                    matches!(key, PersistKey::WindowPos { scope: s, .. } if *s as usize == scope)
                })
        })
        .collect()
}

/// 記憶から戻したスコープ 1 以上で、覚えた印が無いもの＝印の無い古い記憶（dpi-realign-remembered-chain 6.5）。
fn is_unmarked_old_memory(ghost_windows: &GhostWindows, scope: usize) -> bool {
    scope >= 1
        && ghost_windows.default_char_pos(scope).is_none()
        && ghost_windows.remembered_adjacency(scope).is_none()
}

/// 記憶から `keys` を消す送り口（[`persist_entries`] の兄弟・Ghost スコープ固定・fire-and-forget）。
///
/// [`PersistWiring`] 不在は [`persist_entries`] と同じく `debug!` ＋ no-op。消す鍵に隣り合いの印は
/// 足さない（位置を書かないので印の時機ではない）。
pub fn forget_entries(world: &World, keys: Vec<PersistKey>) {
    let Some(wiring) = world.get_non_send::<PersistWiring>() else {
        debug!(
            key_count = keys.len(),
            "PersistWiring 不在: 鍵を消す投函を no-op（fallback boot 経路の縮退・6.2）"
        );
        return;
    };
    wiring.publisher.persist_remove(PersistScope::Ghost, keys);
}

/// 送る束に窓の位置の鍵（[`PersistKey::WindowPos`]）があれば、触れる組 (k−1, k)・(k, k+1) の
/// 隣り合いの印をスコープ n（組の後ろ側）の鍵で束の後ろへ足す（dpi-realign-remembered-chain
/// 要件 6.2・6.4・design AdjacencyPersist）。束の中身（位置の値を含む）は変えない。
///
/// 印は書く時点の画面の並び（各キャラ窓の [`WindowPos`]）で決める: n が組（[`StickyGroupLedger`]）に
/// 入っていれば `none`、そうでなければ [`judge_adjacency`] の結果。組の相手のスコープが窓の一式に
/// 無ければログなしで飛ばし、矩形の読めない窓に触れる組は印を書かず `warn!` を出す。窓の一式
/// （[`GhostWindows`]）そのものが無いときは印を足さず `debug!` を出す。
/// 束にもう有る印の鍵は足さない（同じ画面の並びから決めるので値は同じ・1 つにまとめる）。
pub(super) fn with_adjacency_marks(
    world: &World,
    mut entries: Vec<(PersistKey, String)>,
) -> Vec<(PersistKey, String)> {
    let touched: BTreeSet<usize> = entries
        .iter()
        .filter_map(|(key, _)| match key {
            PersistKey::WindowPos { scope, .. } => Some(*scope as usize),
            _ => None,
        })
        .flat_map(|k| [k, k + 1])
        .collect();
    push_adjacency_marks(world, &mut entries, touched);
    entries
}

/// 組 (n−1, n) ごとに書く時点の画面の並びから印を決め、スコープ n の鍵で束の後ろへ足し、
/// 足した印を返す（[`with_adjacency_marks`] と古い記憶の書き足しが共有する唯一の決め方）。
/// 決め方と飛ばし方は [`with_adjacency_marks`] の説明のとおり。
fn push_adjacency_marks(
    world: &World,
    entries: &mut Vec<(PersistKey, String)>,
    pairs: BTreeSet<usize>,
) -> Vec<(usize, AdjacencyMark)> {
    let mut added = Vec::new();
    if pairs.is_empty() {
        return added;
    }
    let Some(ghost_windows) = world.get_resource::<GhostWindows>() else {
        debug!(
            target: "areka::persist::save",
            pair_count = pairs.len(),
            "隣り合いの印: 窓の一式（GhostWindows）が無いので印を書かない"
        );
        return added;
    };
    let in_set: BTreeSet<usize> = ghost_windows.scopes().collect();
    let rects = snapshot_char_rects(world);
    let rect = |scope: usize| -> Option<ChainRect> {
        rects.iter().find(|(s, _)| *s == scope).map(|(_, r)| *r)
    };
    let ledger = world.get_resource::<StickyGroupLedger>();
    for n in pairs {
        if n == 0 || !in_set.contains(&(n - 1)) || !in_set.contains(&n) {
            continue;
        }
        if entries
            .iter()
            .any(|(key, _)| *key == PersistKey::WindowAdjacent { scope: n as u32 })
        {
            continue;
        }
        let mark = if ledger.is_some_and(|l| l.group_of(n as u32).is_some()) {
            AdjacencyMark::Separate
        } else {
            let (Some(prev), Some(cur)) = (rect(n - 1), rect(n)) else {
                let unreadable = if rect(n).is_none() { n } else { n - 1 };
                warn!(
                    target: "areka::persist::save",
                    scope = unreadable,
                    pair_scope = n,
                    "隣り合いの印: キャラ窓の位置か大きさが読めないので、この組の印は書かない"
                );
                continue;
            };
            AdjacencyMark::from_adjacency(judge_adjacency(prev, cur))
        };
        entries.push(adjacency_entry(n as u32, mark));
        added.push((n, mark));
    }
    added
}

/// 並べ終えた時点の保存（areka-P0-char-position-save-on-exit 要件 1.1・1.2・1.4・1.5・1.7・
/// 4.1〜4.3・design FinalizeSave）。
///
/// 起動の最後の並べ直しが済んだ直後と、既定の位置への並べ直し（`reset_pos` の
/// `reset_window_positions`）の後に呼ばれ、記憶に位置が無いキャラクター窓の位置を記憶へ送る。[`GhostWindows`] に居る全スコープを昇順に回り（数を決め打ちしない・1.4）、
/// 書くのは次の 3 つを満たすスコープだけ:
///
/// - 既定の位置 [`GhostWindows::default_char_pos`] が `Some`＝記憶に位置が無い。`None` は
///   記憶から戻したスコープで、書き換えない（寄せ直して表示している場合を含む・1.2）。
/// - 今の位置（x・y）が既定の位置と同じ。違うのは並べ終える前にドラッグされた窓（ドラッグの
///   確定が書く）か、SHIORI の移動の指示で動いた窓（その回かぎり・1.7）で、どちらも書かない。
///   台本が縦にだけ動かした窓も見逃さないよう x・y の両方で比べる。システムの置き直しは既定の
///   位置を x・y とも一緒に運ぶので（`follow::window_move` の既定の位置の追随）、ふだんの流れで
///   書き漏れは起きない。
/// - 台本の印（`chain_finalize::ScriptPlaced`）が無い。台本の `\![move]` がちょうど既定の位置へ
///   置いた窓は位置では見分けられないので、並べ直しと同じく印で外す（1.7）。
///
/// 位置は [`WindowPos`] の位置と大きさから読み、ドラッグの確定と同じ [`char_pos_to_origin_x`]
/// で下端の中央の x へ直す。送る鍵は [`PersistKey::WindowPos`] の x・y（1.5）と、[`persist_entries`] が
/// 足す触れる組の隣り合いの印（dpi-realign-remembered-chain 6.2・寄せた起動の古い記憶の鍵は除く）で、
/// 全スコープの分を 1 つの束にして [`persist_entries`] の本体で 1 回送る（待たない。書き込みの失敗は送った先が
/// 記録し、UI の側へは戻らない＝起動は続く・4.1）。読めない窓は `warn!` を出して飛ばし、
/// 残りを書く（4.2）。書いた・書かなかったは target `areka::persist::save` に残す（4.3）。
///
/// World は変えず（`&World`）、panic しない。戻す側の [`apply_restored_placements`] は
/// 純関数で World を持たないので、ここへは届かない（寄せ直した位置を書き戻さない構造を保つ）。
pub fn persist_unremembered_char_positions(world: &World) {
    if world.get_non_send::<PersistWiring>().is_none() {
        // 結線は成り立ったが実行系が無い回。スコープごとの行は出さない。
        info!(
            target: "areka::persist::save",
            "並べ終えた時点の保存: 記憶の送り口が無いので書かない"
        );
        return;
    }
    let Some(ghost_windows) = world.get_resource::<GhostWindows>() else {
        // 並べ直しの直後には起きない（防御）。
        debug!(
            target: "areka::persist::save",
            "並べ終えた時点の保存: GhostWindows が無いので書かない"
        );
        return;
    };

    let mut entries = Vec::new();
    let mut unmarked = BTreeSet::new();
    for scope in ghost_windows.scopes() {
        let Some(default_pos) = ghost_windows.default_char_pos(scope) else {
            if is_unmarked_old_memory(ghost_windows, scope) {
                unmarked.insert(scope);
            }
            info!(
                target: "areka::persist::save",
                scope,
                "並べ終えた時点の保存: 記憶に位置があるので書かない"
            );
            continue;
        };
        let skip = |missing: &str| {
            warn!(
                target: "areka::persist::save",
                scope,
                missing,
                "並べ終えた時点の保存: 読めないので飛ばす"
            );
        };
        let Some(entity) = ghost_windows.char_window(scope) else {
            skip("キャラ窓");
            continue;
        };
        let Some(window_pos) = world.get::<WindowPos>(entity) else {
            skip("WindowPos");
            continue;
        };
        let Some(pos) = window_pos.position else {
            skip("位置");
            continue;
        };
        if world.get::<ScriptPlaced>(entity).is_some() {
            info!(
                target: "areka::persist::save",
                scope,
                "並べ終えた時点の保存: 台本が置いた窓なので書かない"
            );
            continue;
        }
        let current = PointPx { x: pos.x, y: pos.y };
        if current != default_pos {
            info!(
                target: "areka::persist::save",
                scope,
                current_x = current.x, current_y = current.y,
                default_x = default_pos.x, default_y = default_pos.y,
                "並べ終えた時点の保存: 並べ終える前に動かされたので書かない"
            );
            continue;
        }
        let Some(size) = window_pos.size else {
            skip("大きさ");
            continue;
        };
        // 形は次の起動の読み戻しと同じ descript の向きで決める（要件 4.8）。
        let Some(anchor) = saved_form_anchor(world, entity) else {
            skip("揃え方");
            continue;
        };
        let char_size = SizePx {
            w: size.width,
            h: size.height,
        };
        // 既定の位置はずれに付いて動くので今の位置で比べ、書くのはずれを引いた元の位置。
        let saved =
            char_pos_to_origin_x(anchor, origin_position(world, entity, current), char_size);
        info!(
            target: "areka::persist::save",
            scope,
            char_x = current.x, char_y = current.y,
            saved_x = saved.x, saved_y = saved.y,
            char_w = char_size.w,
            ?anchor,
            "並べ終えた時点の保存: 書いた"
        );
        entries.extend(char_pos_entries(scope as u32, saved));
    }
    // 記憶の位置を寄せて表示した起動の並びは記憶の並びと違うので、印を決めない（window-work-area-clamp 4.6）。
    // 隣の窓の位置を書く束が足す印からは、送り口の本体が外す（ここで記録した組は重ねて記録しない）。
    let mut logged = BTreeSet::new();
    if !unmarked.is_empty()
        && world
            .get_resource::<RestoredPositionsClamped>()
            .is_some_and(|c| c.0)
    {
        info!(
            target: "areka::persist::save",
            scopes = ?unmarked,
            "並べ終えた時点の保存: 記憶の位置を寄せて表示した起動なので、古い記憶へ隣り合いの印を書き足さない"
        );
        logged = std::mem::take(&mut unmarked);
    }
    // 古い記憶には位置を書かず、画面の並びから決めた印だけを同じ束へ足す（6.5）。
    for (scope, mark) in push_adjacency_marks(world, &mut entries, unmarked) {
        info!(
            target: "areka::persist::save",
            scope,
            mark = %mark.as_str(),
            "並べ終えた時点の保存: 古い記憶に隣り合いの印だけを書き足した"
        );
    }
    if !entries.is_empty() {
        send_entries(world, entries, &logged);
    }
}
