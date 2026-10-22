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

#[cfg(test)]
#[path = "sticky_group_ledger_tests.rs"]
mod tests;
