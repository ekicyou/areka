//! 関連で繋がった id の連結成分と、構成 id だけから決まる束 id（要件 7.9）。
//!
//! ここは純粋層で、ファイルにもスナップショットにも触らない（要件 6.2）。関連の対を
//! 受け取り、値を返す。
//!
//! # 受け取るのは関連の対だけ
//!
//! [`bundles`] は台帳を受け取らない。**どの関連を渡すかは呼ぶ側が決める**。
//! ドメイン別報告（要件 7.1 の 5 つ目・設計 D-11）はその台帳の `links` のうち両端が
//! 自ドメインに属するものだけを渡し、全体報告（要件 7.2）はドメインを跨ぐ対も渡す。
//! ここはその選り分けを知らない。
//!
//! 関連の向きも見ない。`alias_of` と `supersedes` のように互いに逆向きの種別があり
//! （要件 4.3）、どちらの向きで書いても同じ束になるべきだからである。
//!
//! 束の頂点は**対に現れた id だけ**である。関連を 1 つも持たない項目は束に現れない
//! （要件 7.1 が言うのは「関連で繋がった束」であり、繋がっていない項目の一覧では
//! ない。それは状態の分布が受け持つ）。
//!
//! # 束 id は構成 id の最小値
//!
//! 統合担当が人手で書く `doc/ukadoc-coverage/linkage.md` は束 id を引用して束に名前を
//! 付ける（要件 7.9）。報告を作り直すたびに束 id が動くと、その引用がすべて外れる。
//! だから束 id は**構成 id の集合だけ**から決める——入力の並びにも、どの対を先に
//! 見たかにも、内部の畳み方にも依存させない。具体的には構成 id のうち byte 順で最小の
//! ものを使う（設計「`report/<ドメイン>.md`」の項）。
//!
//! 同じ理由で、構成 id の並びも byte 昇順に固定し、束の並びも束 id の昇順に固定する
//! （設計 report 節の事後条件）。

use std::collections::BTreeMap;

use crate::model::EntryId;

/// 関連で繋がった id の 1 束（連結成分）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundle {
    /// 束 id。構成 id のうち byte 順で最小のもの（要件 7.9）。
    pub id: EntryId,
    /// 構成 id。byte 昇順・重複無し。
    pub members: Vec<EntryId>,
}

/// 関連の対から連結成分を作り、束 id の昇順で返す（要件 7.9）。
///
/// 同じ対を 2 度書いても、向きを裏返して書いても、答えは変わらない。自分を指す対
/// （`a` と `a`）は構成 id 1 つの束になる——黙って落とすと「関連を書いたのに束に
/// 出てこない」が説明の付かない形で起きるからである。
pub fn bundles(links: &[(EntryId, EntryId)]) -> Vec<Bundle> {
    // 頂点は「対に現れた順」に採番する。畳み終えたあとで並べ直すので、この順が
    // 答えに残ってはならない（残っていれば入力を並べ替えたときに答えが動く）。
    let mut vertices: Vec<&EntryId> = Vec::new();
    let mut index_of: BTreeMap<&str, usize> = BTreeMap::new();
    let mut parent: Vec<usize> = Vec::new();

    for (from, to) in links {
        let left = intern(from, &mut vertices, &mut index_of, &mut parent);
        let right = intern(to, &mut vertices, &mut index_of, &mut parent);
        unite(&mut parent, left, right);
    }

    // 同じ根に付いた頂点をまとめる。根がどれになるかは対を見た順で変わるが、
    // 束 id も構成 id の並びもこのあと並べ直すので答えには残らない。
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
        // 群は必ず 1 つ以上の構成 id を持つ（空の群は作らない）。
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
    // 最初は自分自身が根。
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

#[cfg(test)]
#[path = "bundle_tests.rs"]
mod tests;
