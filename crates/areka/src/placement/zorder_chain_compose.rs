//! 望む鎖の合成——台帳のグループと窓の在庫から、「手前から奥へ並んだ窓の列」と
//! 「本 spec が張る繋ぎの列」と「窓が無かった宣言要素」を導く純関数
//! （要件 1.1／1.2／1.4／2.5／3.6／6.1／6.2／8.4／10.1／14.1／14.4／15.1／15.2）。
//!
//! design.md「`zorder_chain_compose`（新設・純関数）」が正本である。
//!
//! # ここに在るのは判断だけである
//!
//! Win32 も World も 1 度も触らない。窓の在庫は
//! `resolve`（要素 1 つ → 実在する窓の `Entity`）という 1 本の関数として受け取り、
//! スコープの一覧は値として受け取る。よって鎖の導出は実機も実ディスプレイも無しに
//! 全分岐を固定できる（要件 10.1）。実際に `GhostWindows` を引くのは呼び出し側
//! （指令消化の相）の担当である。
//!
//! # 鎖は 1 本・繋ぎは連続対の部分集合
//!
//! 正典実装と同じく、前後関係は**分岐の無い一直線の所有の鎖**として書く（要件 14.1）。
//! 並びが決まれば繋ぎは自動的に決まる——鎖の**連続する 2 枚**だけを繋ぐからである。
//! ここから不変条件がそのまま従う（要件 14.4）:
//!
//! 1. ある窓を所有する窓は高々 1 つ（星形にならない）
//! 2. ある窓が所有される回数も高々 1 回（輪にも分岐にもならない）
//! 3. 繋ぎの両端は必ず鎖の要素
//! 4. 同じ窓は鎖に 2 度現れない
//!
//! 4 は「1 枚の窓が並びの 1 箇所にしか現れない」ことに帰着する。グループが名前を
//! 挙げたスコープは後方参加から除き、在庫のスコープは重複を落としてから連ねるので、
//! 同じ窓が 2 度並ぶ経路が無い。
//!
//! # 同一スコープの 2 枚は繋がない（境界）
//!
//! 正規化済みの並びでは、同じスコープの 2 枚は必ず `[バルーン, キャラ窓]` の隣り合う
//! 対になる。この対の所有関係は**既存のスコープ内ペア機構がまさに今張っている**
//! ものであり（要件 6.3）、本 spec は 1 本も張らない。よって連続対を舐めながら、
//! この形だけを除く。除かずに張ると、同じ窓へ 2 つの機構が owner を書き合う。
//!
//! 片割れの窓しか実在しないスコープでは、この除外は起こらない——たとえば
//! `[b0, b1, s1]` の `b0 ← b1` は**スコープをまたぐ**繋ぎなので、本 spec が張る。
//! 相方の居ないバルーンにはペア機構が owner を張れないので、書込先は衝突しない。
//!
//! # 並びの規則は 3 つだけ
//!
//! ⑴グループどうしは**登記の順**で連なる（先に登記されたグループほど手前・shell 設定
//! 由来の基底が最前）。台帳の読み口が既にこの順で貸し出すので、ここでは受け取った順に
//! 連ねるだけでよい（要件 3.6）。
//! ⑵グループ内の並びは台帳が正規化した要素順のまま（要件 1.1／1.2／2.1）。
//! ⑶どのグループにも属さないスコープは、全グループの**後ろ**へ、**スコープ ID の昇順**で、
//! `[バルーン, キャラ窓]` のかたまりとして連なる（要件 15.1／15.2）。
//!
//! ⑶の並びは現況の重なりを 1 度も観測せずに決まる——観測を入れると起動のたびに
//! 並びが変わり、決定論が崩れる。
//!
//! # グループがゼロなら計画そのものを作らない
//!
//! グループが 1 つも無い状態が既定状態であり、そこでは前後関係を固定の規則で決めない
//! （要件 6.1／6.2／6.4）。よって `None` を返す——空の計画を返すと「全部外せ」という
//! 指令に見えてしまい、「1 命令も出さない」と区別が付かなくなる。
//!
//! 一方、グループが在って窓が 1 枚も無い場合は `Some` である。指定は生きており、
//! 窓が無かった宣言要素を記録する材料（要件 8.4）を運ぶ必要があるからである。
//!
//! # 宣言要素だけを「不在」に数える
//!
//! `absent` に載るのは**グループが名前を挙げた要素**だけで、宣言順のまま並ぶ
//! （要件 1.4「取り除かない」・要件 8.4「対応する窓が無いことを記録する」）。
//! 後方参加のスコープは誰も宣言していないので、その窓が欠けても不在要素にはならない。
//!
//! # 段階的実装のための `dead_code` 許可
//!
//! 本 task の時点では本番の呼び手が居ない（指令消化の相の出口の差し替えは後続 task）。
//! 判断の檻は兄弟テストが今すぐ固定するので実装は先に置き、結線が着くまでの間だけ
//! 未使用の警告を伏せる。**出口を差し替える task は、この許可を外せるか必ず確かめること。**

#![allow(dead_code)]

use std::collections::HashSet;

use bevy_ecs::entity::Entity;
use wintf::ecs::window::{ChainPlan, CrossEdge};

use super::zorder_group_ledger::{GroupElement, GroupWindowKind, ZOrderGroup};

/// 1 スコープが鎖へ差し出す窓（手前から奥の順）。
///
/// 正規化済みのグループも、後方参加のスコープも、この並びの `[バルーン, キャラ窓]`
/// ——「バルーン窓がキャラ窓の直上」——に従う（要件 1.2／6.3／15.1）。
const SCOPE_BLOCK: [GroupWindowKind; 2] = [GroupWindowKind::Balloon, GroupWindowKind::Char];

/// 要素 1 つを省略記法（`bN`／`sN`）の字面へ。
///
/// 記録行に載る不在要素の正準表記であり、指令消化の相が受理行の `members=` 欄で
/// 使っている字面と同じものである（行の中で照らし合わせられる）。
pub fn element_text(element: &GroupElement) -> String {
    let prefix = match element.kind {
        GroupWindowKind::Balloon => 'b',
        GroupWindowKind::Char => 's',
    };
    format!("{prefix}{}", element.scope)
}

/// 台帳のグループと窓の在庫から、望む鎖 1 本を組む。
///
/// - `groups`: 台帳の読み口が貸し出すグループ列。**登記の順**（基底が先頭・以降は
///   タグの追加順）に並び、各要素列は正規化済み（各スコープが `[バルーン, キャラ窓]`
///   の隣接ブロック）であること。
/// - `all_scopes`: 在庫にある全スコープ。並びも重複も問わない（本関数が昇順へ整える）。
/// - `resolve`: 要素 1 つを実在する窓へ解決する（実在しなければ `None`）。
///
/// 戻り値が `None` なのはグループが 1 つも無いとき、すなわち既定状態のときだけである
/// （要件 6.1／6.4）。
pub fn compose_chain(
    groups: &[ZOrderGroup],
    all_scopes: &[u32],
    resolve: &dyn Fn(&GroupElement) -> Option<Entity>,
) -> Option<ChainPlan> {
    // 既定状態＝指令ゼロ。空の計画ではなく計画そのものを作らない（要件 6.1／6.2／6.4）。
    if groups.is_empty() {
        return None;
    }

    let mut placed: Vec<(GroupElement, Entity)> = Vec::new();
    let mut absent: Vec<String> = Vec::new();
    let mut named: HashSet<u32> = HashSet::new();

    // ⑴ グループを登記の順で連ねる（要件 3.6）。各グループの中は正規化済みの要素順のまま。
    for group in groups {
        for element in &group.members {
            // 窓がまだ現れていなくてもスコープは「グループに属している」ままである
            // （要件 1.4）。よって後方参加からは常に除く。
            named.insert(element.scope);
            match resolve(element) {
                Some(entity) => placed.push((*element, entity)),
                None => absent.push(element_text(element)),
            }
        }
    }

    // ⑵ どのグループにも属さないスコープを、スコープ ID の昇順で後ろへ連ねる
    //    （要件 15.1／15.2）。重複した在庫は 1 度だけ数える（不変条件④）。
    let mut tail: Vec<u32> = all_scopes
        .iter()
        .copied()
        .filter(|scope| !named.contains(scope))
        .collect();
    tail.sort_unstable();
    tail.dedup();
    for scope in tail {
        for kind in SCOPE_BLOCK {
            let element = GroupElement { scope, kind };
            if let Some(entity) = resolve(&element) {
                placed.push((element, entity));
            }
        }
    }

    let members: Vec<Entity> = placed.iter().map(|(_, entity)| *entity).collect();

    // ⑶ 連続対のうち、同一スコープの（バルーン, キャラ窓）対を除いた残りが本 spec の繋ぎ。
    let cross_edges: Vec<CrossEdge> = placed
        .windows(2)
        .filter(|pair| !is_intra_scope_pair(&pair[0].0, &pair[1].0))
        .map(|pair| CrossEdge {
            owned: pair[0].1,
            owner: pair[1].1,
        })
        .collect();

    Some(ChainPlan {
        members,
        cross_edges,
        absent,
    })
}

/// 連続する 2 枚が「同一スコープのバルーン窓とキャラ窓」か——既存のペア機構が
/// 所有関係を張っている対であり、本 spec は張らない（要件 6.3・境界）。
fn is_intra_scope_pair(front: &GroupElement, back: &GroupElement) -> bool {
    front.scope == back.scope
        && front.kind == GroupWindowKind::Balloon
        && back.kind == GroupWindowKind::Char
}

#[cfg(test)]
#[path = "zorder_chain_compose_tests.rs"]
mod tests;
