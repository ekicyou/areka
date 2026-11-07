//! 窓の組（`\![set,sticky-window,...]`・descript の `seriko.sticky-window`）の帳簿と、
//! ID の並びの読み（純・Win32 非依存）。
//!
//! 拒否は値（[`StickyReject`]）で返し、記録は World の側の薄い関数が出す
//! （design「StickyGroupLedger」）。台本の指令と descript の値は同じ読み
//! （[`parse_sticky_tokens`]）を通る（要件 3.8）。
//!
//! 不変条件: どの ID も高々 1 つの組に入る。組は 2 つ以上の ID を持つ。descript の組は
//! 高々 1 つ（descript のキーは 1 つ）。

use std::collections::BTreeSet;

use bevy_ecs::prelude::*;
use tracing::{info, warn};

use super::follow::sync_drag_writer;
use super::spawn::CharWindowMarker;
use super::zorder_group_ledger::parse_scope_id;

/// 組の出どころ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StickySource {
    /// 台本の `\![set,sticky-window,...]`。
    Script,
    /// シェルの descript の `seriko.sticky-window`。
    Descript,
}

/// 窓の組 1 つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StickyGroup {
    /// 並び順に意味を持たせない（要件 3.2）。
    pub members: BTreeSet<u32>,
    pub source: StickySource,
}

/// 組を作らずに捨てた理由（要件 3.4・3.5・3.9・6.5）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StickyReject {
    /// 数として読めない ID（受け取ったままの綴り）。
    UnparsableToken { token: String },
    /// 同じ ID の重ね書き。
    DuplicateScope { scope: u32 },
    /// 異なる ID が 2 つ未満。
    TooFewScopes { count: usize },
    /// 既にほかの組（descript の組を含む）に入っている ID（昇順・要件 3.4）。
    AlreadyGrouped { scopes: Vec<u32> },
}

/// ID の並びを読む。各 ID は前後の空白を除いて `parse_scope_id`（ASCII の数字だけ・あふれは
/// 拒否）で読む。検査の順: 数でない → 同じ ID の重ね書き → 異なる ID が 2 つ未満（要件 3.5）。
/// 上限は置かない（設計の決め 11）。
pub fn parse_sticky_tokens<S: AsRef<str>>(tokens: &[S]) -> Result<BTreeSet<u32>, StickyReject> {
    let mut scopes = Vec::with_capacity(tokens.len());
    for token in tokens {
        let token = token.as_ref();
        let scope = parse_scope_id(token.trim()).ok_or_else(|| StickyReject::UnparsableToken {
            token: token.to_string(),
        })?;
        scopes.push(scope);
    }
    let mut members = BTreeSet::new();
    for scope in scopes {
        if !members.insert(scope) {
            return Err(StickyReject::DuplicateScope { scope });
        }
    }
    if members.len() < 2 {
        return Err(StickyReject::TooFewScopes {
            count: members.len(),
        });
    }
    Ok(members)
}

/// 窓の組の帳簿（World の資源・設計の決め 1）。
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct StickyGroupLedger {
    /// 作った順。
    groups: Vec<StickyGroup>,
}

impl StickyGroupLedger {
    /// 台本の組を足す。重なりがあれば何も変えずに拒否（要件 3.1・3.4）。
    pub fn try_add_script_group(&mut self, members: BTreeSet<u32>) -> Result<(), StickyReject> {
        self.reject_overlap(&members, |_| true)?;
        self.groups.push(StickyGroup {
            members,
            source: StickySource::Script,
        });
        Ok(())
    }

    /// 台本の組をすべて解く。解いた組の数を返す（要件 3.7）。
    pub fn reset_to_descript(&mut self) -> usize {
        let before = self.groups.len();
        self.groups
            .retain(|group| group.source == StickySource::Descript);
        before - self.groups.len()
    }

    /// descript の組を取り替える（起動は空の帳簿へ、シェルの切り替えは今の帳簿へ）。値が無い・
    /// 空なら組なし。台本の組と重なれば作らず拒否を返す（要件 3.8・3.9・6.5）。台本の組は触らない。
    /// 拒否のときも前の descript の組は外れたままにする（新しいシェルの値に取り替えたので）。
    pub fn replace_descript(&mut self, raw: Option<&str>) -> Result<(), StickyReject> {
        self.groups
            .retain(|group| group.source == StickySource::Script);
        let Some(raw) = raw.filter(|raw| !raw.trim().is_empty()) else {
            return Ok(());
        };
        let tokens: Vec<&str> = raw.split(',').collect();
        let members = parse_sticky_tokens(&tokens)?;
        self.reject_overlap(&members, |group| group.source == StickySource::Script)?;
        self.groups.push(StickyGroup {
            members,
            source: StickySource::Descript,
        });
        Ok(())
    }

    pub fn groups(&self) -> &[StickyGroup] {
        &self.groups
    }

    /// 他の機能が「この窓はどの組か」を確かめる口（裁定 7）。
    pub fn group_of(&self, scope: u32) -> Option<&StickyGroup> {
        self.groups
            .iter()
            .find(|group| group.members.contains(&scope))
    }

    /// 組の相手（自分を除く・昇順）。組に無ければ空。
    pub fn partners_of(&self, scope: u32) -> Vec<u32> {
        self.group_of(scope)
            .map(|group| {
                group
                    .members
                    .iter()
                    .copied()
                    .filter(|&member| member != scope)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// `considered` に当たる組と `members` が重なれば、重なった ID（昇順）で拒否する。
    fn reject_overlap(
        &self,
        members: &BTreeSet<u32>,
        considered: impl Fn(&StickyGroup) -> bool,
    ) -> Result<(), StickyReject> {
        let scopes: BTreeSet<u32> = self
            .groups
            .iter()
            .filter(|group| considered(group))
            .flat_map(|group| group.members.intersection(members).copied())
            .collect();
        if scopes.is_empty() {
            Ok(())
        } else {
            Err(StickyReject::AlreadyGrouped {
                scopes: scopes.into_iter().collect(),
            })
        }
    }
}

// ---- World の側の薄い関数（記録を出す・設計「StickyGroupLedger」） ----
//
// 記録はここだけで出す（帳簿は拒否を値で返す）。どの関数も最後に `DragConfig.move_window` を
// 付け直す（設計の決め 12）。

/// 組を変えた／変えようとした後の共通の後始末: その時点のすべての組を記録し、書き手を付け直す（9.2）。
fn log_groups_and_resync(world: &mut World, action: &'static str) {
    if let Some(ledger) = world.get_resource::<StickyGroupLedger>() {
        info!(
            event = "sticky_groups",
            action,
            groups = ?ledger.groups(),
            "sticky: 窓の組の一覧"
        );
    }
    sync_drag_writer(world);
}

/// 拒否を記録する（3.4・3.5・3.9・6.5・9.2）。
fn warn_rejected<S: AsRef<str> + std::fmt::Debug>(
    source: &'static str,
    tokens: &[S],
    reason: &StickyReject,
) {
    warn!(
        event = "sticky_group_rejected",
        source,
        tokens = ?tokens,
        reason = ?reason,
        "sticky: 窓の組を作らずに捨てた"
    );
}

/// 受け付けた組のうち窓の無いスコープを記録する（受け付けたまま・3.6）。
fn warn_absent_members(world: &mut World, members: &BTreeSet<u32>, when: &'static str) {
    let present: BTreeSet<u32> = world
        .query::<&CharWindowMarker>()
        .iter(world)
        .filter_map(|marker| u32::try_from(marker.scope).ok())
        .collect();
    for &scope in members.difference(&present) {
        warn!(
            event = "sticky_member_absent",
            scope, when, "sticky: 組に窓の無いスコープがある（受け付けたまま）"
        );
    }
}

/// 帳簿が無いときの指令を捨てる記録（取り出しの相は窓が無いと読まないので、起きないはず）。
fn warn_no_ledger(directive: &'static str) {
    warn!(
        event = "sticky_skipped",
        directive,
        reason = "no_ledger",
        "sticky: 窓の組の帳簿が無いので捨てた"
    );
}

/// 台本の `\![set,sticky-window,ID,ID,...]` を当てる。組を作れば真（3.1・3.4〜3.6）。
pub fn apply_set_sticky(world: &mut World, tokens: &[String]) -> bool {
    let Some(mut ledger) = world.get_resource_mut::<StickyGroupLedger>() else {
        warn_no_ledger("set");
        return false;
    };
    let added = parse_sticky_tokens(tokens).and_then(|members| {
        ledger.try_add_script_group(members.clone())?;
        Ok(members)
    });
    let ok = match added {
        Ok(members) => {
            warn_absent_members(world, &members, "set");
            true
        }
        Err(reason) => {
            warn_rejected("script", tokens, &reason);
            false
        }
    };
    log_groups_and_resync(world, "set");
    ok
}

/// 台本の `\![reset,sticky-window]` を当てる（台本の組だけを解く・3.7）。帳簿があれば真。
pub fn apply_reset_sticky(world: &mut World) -> bool {
    let Some(mut ledger) = world.get_resource_mut::<StickyGroupLedger>() else {
        warn_no_ledger("reset");
        return false;
    };
    ledger.reset_to_descript();
    log_groups_and_resync(world, "reset");
    true
}

/// 帳簿の descript の組を取り替え、拒否と窓の無いスコープを記録する（3.6・3.8・3.9・6.5）。
fn apply_descript(world: &mut World, raw: Option<&str>, action: &'static str) {
    let result = world
        .resource_mut::<StickyGroupLedger>()
        .replace_descript(raw);
    match result {
        Ok(()) => {
            let members = world
                .resource::<StickyGroupLedger>()
                .groups()
                .iter()
                .find(|group| group.source == StickySource::Descript)
                .map(|group| group.members.clone());
            if let Some(members) = members {
                warn_absent_members(world, &members, "seed");
            }
        }
        Err(reason) => {
            let tokens: Vec<&str> = raw.unwrap_or_default().split(',').collect();
            warn_rejected("descript", &tokens, &reason);
        }
    }
    log_groups_and_resync(world, action);
}

/// 起動で帳簿を据え、descript の `seriko.sticky-window` の組を入れる（3.8・3.9）。
pub fn seed_sticky_descript(world: &mut World, raw: Option<&str>) {
    world.insert_resource(StickyGroupLedger::default());
    apply_descript(world, raw, "descript_seed");
}

/// シェルの切り替えで descript の組だけを取り替える（台本の組は保つ・6.5）。
pub fn replace_sticky_descript(world: &mut World, raw: Option<&str>) {
    if !world.contains_resource::<StickyGroupLedger>() {
        warn_no_ledger("descript_replace");
        return;
    }
    apply_descript(world, raw, "descript_replace");
}

#[cfg(test)]
#[path = "sticky_group_ledger_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "sticky_group_ledger_world_tests.rs"]
mod world_tests;
