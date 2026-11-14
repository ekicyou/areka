//! Z 順のテストが共に使う道具（台帳・鎖の合成のテスト 6 ファイルの写しを 1 つにまとめた）。
//!
//! 要素の `b`／`s`、要素から決定論的に組む `Entity`、繋ぎ 1 本、本番の解釈器を通したタグの
//! グループを置く。テストのときだけ組み込む。

use bevy_ecs::entity::Entity;
use wintf::ecs::window::{ChainSegment, CrossEdge};

use crate::zorder_group_ledger::{
    GroupElement, GroupSource, GroupWindowKind, ZOrderGroup, parse_zorder_tokens,
};

/// バルーン窓の要素（`balloonN`／`bN`）。
pub(crate) fn b(scope: u32) -> GroupElement {
    GroupElement {
        scope,
        kind: GroupWindowKind::Balloon,
    }
}

/// キャラ窓の要素（`surfaceN`／`sN`）。
pub(crate) fn s(scope: u32) -> GroupElement {
    GroupElement {
        scope,
        kind: GroupWindowKind::Char,
    }
}

/// 要素 1 つに割り当てる決定論的な `Entity`（スコープ×種別で一意）。
pub(crate) fn ent(element: GroupElement) -> Entity {
    let kind = match element.kind {
        GroupWindowKind::Balloon => 0,
        GroupWindowKind::Char => 1,
    };
    Entity::from_raw_u32(element.scope * 2 + kind + 1).expect("テスト用 entity 索引は有効")
}

/// 要素列を `Entity` 列へ。
pub(crate) fn ents(elements: &[GroupElement]) -> Vec<Entity> {
    elements.iter().copied().map(ent).collect()
}

/// 手前側 `front` が奥側 `back` に所有される繋ぎ 1 本（`front ← back`・区間つき）。
///
/// 区間は**手前側の枠が属していたもの**である（合成の規則そのもの）。ここで書き下すのは
/// 「どのグループの繋ぎか」が記録から復元できることの主張であり、値を取り違えると赤くなる。
pub(crate) fn edge(front: GroupElement, back: GroupElement, segment: ChainSegment) -> CrossEdge {
    CrossEdge {
        owned: ent(front),
        owner: ent(back),
        segment,
    }
}

/// タグ由来のグループ 1 本を、本番と同じ解釈器（補い込み）を通して組む。
pub(crate) fn tag_group(id: u32, tokens: &[&str]) -> ZOrderGroup {
    let (members, _normalizations) =
        parse_zorder_tokens(tokens).expect("テストの指定は受理されるもの");
    ZOrderGroup {
        id,
        members,
        source: GroupSource::Tag,
    }
}
