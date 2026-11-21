//! 関連で繋がった id の連結成分と、構成 id だけから決まる束 id。
//!
//! ここは純粋層で、ファイルにもスナップショットにも触らない。関連の対を
//! 受け取り、値を返す。
//!
//! # 受け取るのは関連の対だけ
//!
//! [`bundles`] は台帳を受け取らない。**どの関連を渡すかは呼ぶ側が決める**。
//! ドメイン別報告はその台帳の `links` を全部渡し、返った束のうち構成 id が全部その台帳に
//! あるものだけを残す（外へ伸びた束は丸ごと落ちる）。他の台帳からこの台帳の id へ入って
//! くる関連は渡されないので見えず、そうした束は全体報告ではより大きな跨ぐ束の一部として、
//! 別の束 id の下に載ることがある。全体報告はドメインを跨ぐ対も渡す。ここはその選り分けを
//! 知らない。
//!
//! 関連の向きも見ない。`alias_of` と `supersedes` のように互いに逆向きの種別があり、
//! どちらの向きで書いても同じ束になるべきだからである。
//!
//! 束の頂点は**対に現れた id だけ**である。関連を 1 つも持たない項目は束に現れない
//! （繋がっていない項目の一覧は状態の分布が受け持つ）。
//!
//! # 束 id は構成 id の最小値
//!
//! 統合担当が人手で書く `doc/ukadoc-coverage/linkage.md` は束 id を引用して束に名前を
//! 付ける。報告を作り直すたびに束 id が動くと、その引用がすべて外れる。
//! だから束 id は**構成 id の集合だけ**から決める——入力の並びにも、どの対を先に
//! 見たかにも、内部の畳み方にも依存させない。具体的には構成 id のうち byte 順で最小の
//! ものを使う。
//!
//! 同じ理由で、構成 id の並びも byte 昇順に固定し、束の並びも束 id の昇順に固定する。

use std::collections::BTreeMap;

use crate::ledger::Ledger;
use crate::model::EntryId;

/// 関連で繋がった id の 1 束（連結成分）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundle {
    /// 束 id。構成 id のうち byte 順で最小のもの。
    pub id: EntryId,
    /// 構成 id。byte 昇順・重複無し。
    pub members: Vec<EntryId>,
}

/// 関連の対から連結成分を作り、束 id の昇順で返す。
///
/// 同じ対を 2 度書いても、向きを裏返して書いても、答えは変わらない。自分を指す対
/// （`a` と `a`）は構成 id 1 つの束になる——黙って落とすと「関連を書いたのに束に
/// 出てこない」が説明の付かない形で起きるからである。
pub fn bundles(links: &[(EntryId, EntryId)]) -> Vec<Bundle> {
    // 採番は対に現れた順。この順が答えに残ると入力を並べ替えたときに答えが動く。
    let mut vertices: Vec<&EntryId> = Vec::new();
    let mut index_of: BTreeMap<&str, usize> = BTreeMap::new();
    let mut parent: Vec<usize> = Vec::new();

    for (from, to) in links {
        let left = intern(from, &mut vertices, &mut index_of, &mut parent);
        let right = intern(to, &mut vertices, &mut index_of, &mut parent);
        unite(&mut parent, left, right);
    }

    let roots: Vec<usize> = (0..vertices.len())
        .map(|index| find(&mut parent, index))
        .collect();
    let mut groups: BTreeMap<usize, Vec<EntryId>> = BTreeMap::new();
    for (root, vertex) in roots.into_iter().zip(vertices.iter()) {
        groups.entry(root).or_default().push((*vertex).clone());
    }

    let mut result: Vec<Bundle> = Vec::new();
    for members in groups.into_values() {
        let mut members = members;
        members.sort();
        if let Some(id) = members.first().cloned() {
            result.push(Bundle { id, members });
        }
    }
    result.sort_by(|left, right| left.id.cmp(&right.id));
    result
}

/// 頂点に番号を振る。すでに現れた id なら前の番号を返す。
///
/// 引き当ては id の綴りで行う（[`EntryId`] は綴りをそのまま持つ）。
fn intern<'a>(
    id: &'a EntryId,
    vertices: &mut Vec<&'a EntryId>,
    index_of: &mut BTreeMap<&'a str, usize>,
    parent: &mut Vec<usize>,
) -> usize {
    if let Some(found) = index_of.get(id.as_str()) {
        return *found;
    }
    let index = vertices.len();
    vertices.push(id);
    index_of.insert(id.as_str(), index);
    parent.push(index);
    index
}

/// 頂点の属する群の根を引く（経路を短くしながら辿る）。
///
/// 番号は [`intern`] が `vertices.len()` から振ったものだけなので、`parent` の範囲を
/// 外れることはない。
fn find(parent: &mut [usize], node: usize) -> usize {
    let mut node = node;
    while parent[node] != node {
        parent[node] = parent[parent[node]];
        node = parent[node];
    }
    node
}

/// 2 つの頂点を同じ群に入れる。
fn unite(parent: &mut [usize], left: usize, right: usize) {
    let left_root = find(parent, left);
    let right_root = find(parent, right);
    if left_root != right_root {
        parent[right_root] = left_root;
    }
}

/// 台帳の `links` を辺の列にする（ドメイン別報告は 1 本、全体報告は 4 本を渡す）。
///
/// **1 つの id から何本でも辺が出る**（関連は種別違いで何本でも書ける）。
/// 元 id を鍵にした表で集めると 2 本目以降が黙って落ち、繋がるはずの束が割れる。
///
/// `alias_of`・`supersedes` の欄は辺にしない（`links` の kind としてなら辺になる）。
/// 束は `links` だけで作る。
pub(super) fn all_links(ledgers: &[&Ledger]) -> Vec<(EntryId, EntryId)> {
    let mut edges: Vec<(EntryId, EntryId)> = Vec::new();
    for ledger in ledgers {
        for entry in ledger.entries.values() {
            for link in &entry.links {
                edges.push((entry.id.clone(), link.to.clone()));
            }
        }
    }
    edges
}

#[cfg(test)]
#[path = "bundle_tests.rs"]
mod tests;
