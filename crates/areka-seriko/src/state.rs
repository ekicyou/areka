//! サーフェス状態モジュール（状態層・per-scope の現 surface 状態と静的 bind 集合の所有者）。
//!
//! 話者スコープ（`ActorKey`）ごとに独立した現在の表示状態（表示中 surface / 非表示）を
//! `HashMap` で保持し（`ActorKey` は `Hash+Eq` を持つが `Ord` を持たないため HashMap・design
//! 状態層）、shell descript 由来の静的 bind 集合を per-scope マップと**同居**して保持する
//! （要件 4.4・後続 `mayuna-compose` が置き場のみ差し替えられる）。
//!
//! [`ScopeStates::apply`] は 1 cue = 1 scope の更新をトランザクション境界とし、状態が実際に
//! 変化したときだけ発行すべき [`DisplayCommand`] を [`ApplyOutcome::Changed`] で返す。状態
//! 不変なら [`ApplyOutcome::Unchanged`] を返して再発行を抑止する（冪等ガード・要件 3.4・DD8）。
//! 実際の発行（単一発行点）は後続タスク（アクター層 `emit_display`）の責務。

use std::collections::HashMap;

use areka_emo_compose::BindSet;
use areka_sakura::ActorKey;

use crate::output::DisplayCommand;
use crate::resolve::SurfaceTarget;

/// あるスコープの現 surface 状態（要件 3.1/3.3/3.4）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScopeState {
    /// 表示中の surface id（要件 3.1/3.5）。
    Shown(u32),
    /// 非表示（`\s[-1]` 相当・要件 3.3/3.4）。
    Hidden,
}

/// `apply` の適用結果（状態変化の有無を発行層へ伝える冪等ガード用）。
///
/// - [`ApplyOutcome::Changed`]: 状態が実際に変化した。同梱の [`DisplayCommand`] を発行すべき。
/// - [`ApplyOutcome::Unchanged`]: 状態不変＝再発行不要（要件 3.4・DD8）。
#[derive(Clone, Debug, PartialEq)]
pub enum ApplyOutcome {
    /// 発行すべき指令（状態が変化した・要件 5.1/5.2）。
    Changed(DisplayCommand),
    /// 状態不変＝発行しない（冪等・要件 3.4）。
    Unchanged,
}

/// per-scope surface 状態と静的 bind 集合の所有者（状態層）。
///
/// per-scope マップと静的 `BindSet` を同一構造体に同居させ（要件 4.4）、後続の動的切替
/// ユニット（`mayuna-compose`）が `static_binds` の置き場のみを差し替えられる形にする。
/// 本ユニットは bind の切替 API を持たず、`static_binds` は [`ScopeStates::new`] で一度だけ
/// 設定して以後不変（要件 4.3）。
pub struct ScopeStates {
    /// 話者スコープごとの現 surface 状態（`ActorKey` は `Ord` 非対応ゆえ HashMap・design）。
    scopes: HashMap<ActorKey, ScopeState>,
    /// 静的 bind 集合（起動時に一度だけ解決・以後不変・要件 4.2/4.3/4.4）。
    static_binds: BindSet,
}

impl ScopeStates {
    /// 静的 bind 集合を受けて空のスコープ状態を構築する（要件 4.2）。
    ///
    /// `static_binds` は task 1.2/2.4 で解決される bindgroup default 由来の `BindSet` を想定し、
    /// 本構造体はそれを不変に保持する（切替 API を持たない・要件 4.3）。
    pub fn new(static_binds: BindSet) -> Self {
        Self {
            scopes: HashMap::new(),
            static_binds,
        }
    }

    /// 1 つの解決済み surface 指令を対象スコープへ適用する（1 cue = 1 scope・design）。
    ///
    /// 対象 `scope` のエントリのみ触り、他スコープの状態は一切変更しない（要件 3.1/3.2）。
    /// 戻り値で状態変化の有無を返し、冪等ガード（要件 3.4）を呼び手（発行層）へ委ねる。
    ///
    /// 分岐（design Postconditions）:
    /// - [`SurfaceTarget::Show(id)`]:
    ///   - 現状が既に `Shown(id)`（同一 id）→ [`ApplyOutcome::Unchanged`]（冪等・再発行しない・
    ///     要件 3.4/DD8）。
    ///   - それ以外（未知 scope・`Hidden`・別 id を表示中）→ 状態を `Shown(id)` に設定（未知 scope
    ///     は新規挿入・design「未知 scope への Show は新規挿入」）し、
    ///     [`ApplyOutcome::Changed`]`(DisplayCommand::Show { scope, surface_id: id, binds })` を返す
    ///     （現在の静的 bind 集合を同梱・要件 5.1）。
    /// - [`SurfaceTarget::Hide`]:
    ///   - 現状が既に `Hidden` → [`ApplyOutcome::Unchanged`]（非表示保持・要件 3.4）。
    ///   - それ以外（表示中、または未知 scope）→ 状態を `Hidden` に設定し
    ///     [`ApplyOutcome::Changed`]`(DisplayCommand::Hide { scope })` を返す（要件 5.2）。
    ///     未知 scope への `Hide` は先行 `Hidden` 状態を持たないため「変化」とみなし Hide を発行する
    ///     （通常運用ではスコープは未設定から始まり、明示的な非表示指定は 1 度発行して観測可能に
    ///     する方が下流にとって安全。design「未知 scope への Show は新規挿入」と整合の判断）。
    /// - [`SurfaceTarget::Unresolved`]: 呼び手が apply 前に skip するのが正規経路（design
    ///   「Unresolved は apply に渡さない」）。防御的に、状態を変更せず [`ApplyOutcome::Unchanged`]
    ///   を返す（panic せず・発行もしない）。
    pub fn apply(&mut self, scope: &ActorKey, target: SurfaceTarget) -> ApplyOutcome {
        match target {
            SurfaceTarget::Show(id) => {
                // 冪等: 既に同一 surface を表示中なら再発行しない（要件 3.4/DD8）。
                if self.scopes.get(scope) == Some(&ScopeState::Shown(id)) {
                    return ApplyOutcome::Unchanged;
                }
                // 未知 scope は新規挿入・Hidden/別 id は上書き（要件 3.1/3.5）。
                self.scopes.insert(scope.clone(), ScopeState::Shown(id));
                ApplyOutcome::Changed(DisplayCommand::Show {
                    scope: scope.clone(),
                    surface_id: id,
                    binds: self.static_binds.clone(),
                })
            }
            SurfaceTarget::Hide => {
                // 非表示保持: 既に Hidden なら再発行しない（要件 3.4）。
                if self.scopes.get(scope) == Some(&ScopeState::Hidden) {
                    return ApplyOutcome::Unchanged;
                }
                // 表示中→非表示、または未知 scope→非表示（要件 3.3）。
                self.scopes.insert(scope.clone(), ScopeState::Hidden);
                ApplyOutcome::Changed(DisplayCommand::Hide {
                    scope: scope.clone(),
                })
            }
            // 呼び手が先に skip する（正規経路）。防御的に無変更・無発行（要件 6.1）。
            SurfaceTarget::Unresolved => ApplyOutcome::Unchanged,
        }
    }

    /// 現在の静的 bind 集合を返す（不変・要件 4.2/4.3/4.4）。
    ///
    /// 本ユニットは bind を変更する API を提供しない（動的切替は `mayuna-compose` の領分）。
    pub fn binds(&self) -> &BindSet {
        &self.static_binds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト用 bind 集合（非空・emo2 実測相当の任意 id）。
    fn binds_1100_1207() -> BindSet {
        BindSet::from_ids([1100, 1207])
    }

    fn empty_states() -> ScopeStates {
        ScopeStates::new(binds_1100_1207())
    }

    /// スコープ分離（要件 3.1/3.2）: あるスコープへの適用が他スコープへ波及しない。
    #[test]
    fn scope_isolation_does_not_touch_other_scope() {
        let mut states = empty_states();
        let scope0 = ActorKey::from("0");
        let scope1 = ActorKey::from("1");

        // scope "0" に Show を適用しても scope "1" は未知（未設定）のまま。
        let outcome = states.apply(&scope0, SurfaceTarget::Show(2100));
        assert_eq!(
            outcome,
            ApplyOutcome::Changed(DisplayCommand::Show {
                scope: scope0.clone(),
                surface_id: 2100,
                binds: binds_1100_1207(),
            })
        );
        assert_eq!(states.scopes.get(&scope0), Some(&ScopeState::Shown(2100)));
        assert_eq!(states.scopes.get(&scope1), None, "他スコープは未設定のまま");

        // scope "1" を別 surface で更新しても scope "0" の状態は保たれる。
        let outcome1 = states.apply(&scope1, SurfaceTarget::Show(2200));
        assert_eq!(
            outcome1,
            ApplyOutcome::Changed(DisplayCommand::Show {
                scope: scope1.clone(),
                surface_id: 2200,
                binds: binds_1100_1207(),
            })
        );
        assert_eq!(
            states.scopes.get(&scope0),
            Some(&ScopeState::Shown(2100)),
            "scope 0 は scope 1 の更新で変わらない"
        );
        assert_eq!(states.scopes.get(&scope1), Some(&ScopeState::Shown(2200)));
    }

    /// 非表示遷移（要件 3.3）: 表示中から Hide で Hidden へ遷移し Hide 指令を発行する。
    #[test]
    fn hide_transition_from_shown() {
        let mut states = empty_states();
        let scope = ActorKey::from("0");

        // まず表示中にする。
        assert!(matches!(
            states.apply(&scope, SurfaceTarget::Show(2100)),
            ApplyOutcome::Changed(_)
        ));

        // Hide で Hidden へ遷移し Hide 指令。
        let outcome = states.apply(&scope, SurfaceTarget::Hide);
        assert_eq!(
            outcome,
            ApplyOutcome::Changed(DisplayCommand::Hide {
                scope: scope.clone(),
            })
        );
        assert_eq!(states.scopes.get(&scope), Some(&ScopeState::Hidden));
    }

    /// 非表示保持・冪等（要件 3.4/DD8）: 既に Hidden な状態への Hide は Unchanged（再発行しない）。
    #[test]
    fn hide_when_already_hidden_is_unchanged() {
        let mut states = empty_states();
        let scope = ActorKey::from("0");

        // Show→Hide で Hidden にしておく。
        states.apply(&scope, SurfaceTarget::Show(2100));
        assert!(matches!(
            states.apply(&scope, SurfaceTarget::Hide),
            ApplyOutcome::Changed(_)
        ));

        // もう一度 Hide → Unchanged（状態不変ゆえ再発行不要）。
        let outcome = states.apply(&scope, SurfaceTarget::Hide);
        assert_eq!(outcome, ApplyOutcome::Unchanged);
        assert_eq!(states.scopes.get(&scope), Some(&ScopeState::Hidden));
    }

    /// 冪等 Show（要件 3.4/DD8）: 同一 surface の再指定は 2 回目 Unchanged。
    #[test]
    fn show_same_surface_twice_second_is_unchanged() {
        let mut states = empty_states();
        let scope = ActorKey::from("0");

        // 1 回目: Changed。
        let first = states.apply(&scope, SurfaceTarget::Show(2100));
        assert_eq!(
            first,
            ApplyOutcome::Changed(DisplayCommand::Show {
                scope: scope.clone(),
                surface_id: 2100,
                binds: binds_1100_1207(),
            })
        );

        // 2 回目: 同一 id ゆえ Unchanged。
        let second = states.apply(&scope, SurfaceTarget::Show(2100));
        assert_eq!(second, ApplyOutcome::Unchanged);
        assert_eq!(states.scopes.get(&scope), Some(&ScopeState::Shown(2100)));
    }

    /// 別 surface への切替は Changed（冪等ガードは同一 id 限定であることの反証）。
    #[test]
    fn show_different_surface_is_changed() {
        let mut states = empty_states();
        let scope = ActorKey::from("0");

        states.apply(&scope, SurfaceTarget::Show(2100));
        let outcome = states.apply(&scope, SurfaceTarget::Show(2106));
        assert_eq!(
            outcome,
            ApplyOutcome::Changed(DisplayCommand::Show {
                scope: scope.clone(),
                surface_id: 2106,
                binds: binds_1100_1207(),
            })
        );
        assert_eq!(states.scopes.get(&scope), Some(&ScopeState::Shown(2106)));
    }

    /// 非表示→表示の復帰（要件 3.5）: Hidden から Show(id) で Shown(id) へ遷移し Show 指令。
    #[test]
    fn recovery_from_hidden_to_shown() {
        let mut states = empty_states();
        let scope = ActorKey::from("0");

        // Show→Hide で Hidden にする。
        states.apply(&scope, SurfaceTarget::Show(2100));
        states.apply(&scope, SurfaceTarget::Hide);
        assert_eq!(states.scopes.get(&scope), Some(&ScopeState::Hidden));

        // Hidden から表示 surface 指定で復帰。
        let outcome = states.apply(&scope, SurfaceTarget::Show(2106));
        assert_eq!(
            outcome,
            ApplyOutcome::Changed(DisplayCommand::Show {
                scope: scope.clone(),
                surface_id: 2106,
                binds: binds_1100_1207(),
            })
        );
        assert_eq!(states.scopes.get(&scope), Some(&ScopeState::Shown(2106)));
    }

    /// bind 同梱（要件 4.2/5.1）: Changed(Show) の binds は `new` で渡した静的集合と一致する。
    #[test]
    fn show_command_carries_static_binds() {
        let mut states = ScopeStates::new(BindSet::from_ids([1100, 1207]));
        let scope = ActorKey::from("0");

        let outcome = states.apply(&scope, SurfaceTarget::Show(2100));
        match outcome {
            ApplyOutcome::Changed(DisplayCommand::Show { binds, .. }) => {
                assert_eq!(binds, BindSet::from_ids([1100, 1207]));
            }
            other => panic!("Changed(Show) を期待: {other:?}"),
        }
    }

    /// binds() アクセサ（要件 4.3/4.4）: 静的集合を返し、切替 API を持たない（不変）。
    #[test]
    fn binds_accessor_returns_static_set() {
        let states = ScopeStates::new(BindSet::from_ids([1100, 1207]));
        assert_eq!(states.binds(), &BindSet::from_ids([1100, 1207]));

        // 空 bind でも accessor は静的値を返す。
        let empty = ScopeStates::new(BindSet::from_ids([]));
        assert_eq!(empty.binds(), &BindSet::from_ids([]));
    }

    /// Unresolved は no-op（要件 6.1）: 状態不変・発行なし・panic なし。
    #[test]
    fn unresolved_is_noop() {
        let mut states = empty_states();
        let scope = ActorKey::from("0");

        // 未設定スコープへの Unresolved → Unchanged・未設定のまま。
        let outcome = states.apply(&scope, SurfaceTarget::Unresolved);
        assert_eq!(outcome, ApplyOutcome::Unchanged);
        assert_eq!(states.scopes.get(&scope), None, "状態は生成されない");

        // 既存状態を持つスコープでも Unresolved は状態を変えない。
        states.apply(&scope, SurfaceTarget::Show(2100));
        let outcome2 = states.apply(&scope, SurfaceTarget::Unresolved);
        assert_eq!(outcome2, ApplyOutcome::Unchanged);
        assert_eq!(
            states.scopes.get(&scope),
            Some(&ScopeState::Shown(2100)),
            "既存状態は保たれる"
        );
    }

    /// 未知 scope への Hide は「変化」とみなし Hide を 1 度発行、以後は保持（要件 3.3/3.4）。
    #[test]
    fn hide_on_unknown_scope_emits_once_then_holds() {
        let mut states = empty_states();
        let scope = ActorKey::from("2");

        // 未知 scope への Hide は先行 Hidden を持たないため Changed。
        let first = states.apply(&scope, SurfaceTarget::Hide);
        assert_eq!(
            first,
            ApplyOutcome::Changed(DisplayCommand::Hide {
                scope: scope.clone(),
            })
        );
        assert_eq!(states.scopes.get(&scope), Some(&ScopeState::Hidden));

        // 2 回目は保持ゆえ Unchanged。
        assert_eq!(
            states.apply(&scope, SurfaceTarget::Hide),
            ApplyOutcome::Unchanged
        );
    }
}
