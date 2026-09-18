//! メニューの計画（areka-P0-popup-menu-minimal）。
//!
//! 登記の写し（枠と項目の列）と、照会で決まった項目名から、表示するメニューの構造
//! ——枠の並び順・区切り線・1 回の表示の中で一意な識別子・アクセスキー `&` の写し——
//! を作る純粋な計算を置く。OS の API にも World にも触れない。

use std::collections::HashMap;

use super::captions::CaptionMap;
use super::{Frame, ItemBody, MenuAction, MenuItem};

/// 表示するメニューの 1 要素（要件 2.1・2.4〜2.6）。閉包を持たないので値として比べられる。
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PlanEntry {
    /// 選べる項目。`id` は 1 回の表示の中で一意（要件 2.7）。
    Item {
        id: u32,
        label: String,
        enabled: bool,
        checked: bool,
    },
    /// 子項目を持つ見出し。見出し自体は選べないので識別子を持たない。
    Submenu {
        label: String,
        enabled: bool,
        children: Vec<PlanEntry>,
    },
    /// 群の間の区切り線。
    Separator,
}

/// 表示直前の計画。並びと、識別子から動作への逆引きを持つ。
pub(crate) struct MenuPlan {
    pub entries: Vec<PlanEntry>,
    actions: HashMap<u32, (Frame, MenuAction)>,
}

impl MenuPlan {
    /// 選ばれた識別子から、その項目を登記した枠と動作を引く（要件 2.7）。
    /// `Submenu` の見出しや区切り線、知らない識別子には `None`。
    pub(crate) fn action(&self, id: u32) -> Option<(Frame, MenuAction)> {
        self.actions
            .get(&id)
            .map(|(frame, action)| (*frame, action.clone()))
    }

    /// 選べる項目（[`PlanEntry::Item`]）の総数。サブメニューの子項目は数え、見出しと
    /// 区切り線は数えない。表示を記録する行の `items=`（要件 8.1）に使う。
    pub(crate) fn item_count(&self) -> usize {
        self.actions.len()
    }
}

/// 写しと項目名から計画を組み立てる。`snapshot` は既に [`Frame::ORDER`] 順であること。
pub(crate) fn build(snapshot: Vec<(Frame, MenuItem)>, captions: &CaptionMap) -> MenuPlan {
    let mut plan = MenuPlan {
        entries: Vec::new(),
        actions: HashMap::new(),
    };
    let mut next_id = 1;
    let mut last_group = None;
    for (frame, item) in snapshot {
        let group = group_of(frame);
        if last_group.is_some_and(|last| last != group) {
            plan.entries.push(PlanEntry::Separator);
        }
        last_group = Some(group);
        let entry = convert(frame, item, captions, &mut next_id, &mut plan.actions);
        plan.entries.push(entry);
    }
    plan
}

/// 区切り線のための群。{①②③}{④⑤}{⑥}{⑦} の隣り合う非空の群の間に 1 本入れる
/// （要件 2.6 の設計裁定）。空の群は間にあっても区切りを増やさない。
fn group_of(frame: Frame) -> u8 {
    match frame {
        Frame::Ghost | Frame::Shell | Frame::Balloon => 0,
        Frame::Update | Frame::Install => 1,
        Frame::Readme => 2,
        Frame::Close => 3,
    }
}

/// 登記の 1 項目を計画の 1 要素へ写す。葉には識別子を払い出して動作を控え、サブメニューは
/// 見出しに識別子を与えずに子項目へ降りる（＝識別子は深さ優先の出現順になる）。
fn convert(
    frame: Frame,
    item: MenuItem,
    captions: &CaptionMap,
    next_id: &mut u32,
    actions: &mut HashMap<u32, (Frame, MenuAction)>,
) -> PlanEntry {
    let label = caption_for(&item, captions);
    match item.body {
        ItemBody::Action(action) => {
            let id = *next_id;
            *next_id += 1;
            actions.insert(id, (frame, action));
            PlanEntry::Item {
                id,
                label,
                enabled: item.enabled,
                checked: item.checked == Some(true),
            }
        }
        ItemBody::Submenu(children) => PlanEntry::Submenu {
            label,
            enabled: item.enabled,
            children: children
                .into_iter()
                .map(|child| convert(frame, child, captions, next_id, actions))
                .collect(),
        },
    }
}

/// 項目の文言を決める。リソースの値が非空ならそのまま（`&` はアクセラレータ記法として
/// 素通し・要件 3.5）、それ以外は登記された既定名をアクセラレータ用に写す。子項目も同じ。
fn caption_for(item: &MenuItem, captions: &CaptionMap) -> String {
    item.caption_resource
        .and_then(|id| captions.get(id))
        .filter(|caption| !caption.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| escape_ampersand(&item.label))
}

/// 登記者由来の文言の `&` を OS のアクセラレータ記法から逃がす（`&`→`&&`）。
pub(crate) fn escape_ampersand(label: &str) -> String {
    label.replace('&', "&&")
}

#[cfg(test)]
#[path = "plan_tests.rs"]
mod plan_tests;
