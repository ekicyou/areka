//! [`apply_named_bind`] の 4 つの結果の試験（spec: areka-P0-offline-surface-dump 要件 5.1・5.3・5.4・14.2）。

use super::*;
use crate::resolve::SurfaceTarget;
use crate::state::{BindApplyOutcome, ScopeStates};

/// 本体側に「目」（既定・1301/1303）と「服」（mustselect・1500/1501）を持つ表。
fn resolver() -> BindResolver {
    let mut sakura = BTreeMap::new();
    sakura.insert(("目".to_string(), "開き".to_string()), 1301);
    sakura.insert(("目".to_string(), "閉じ".to_string()), 1303);
    sakura.insert(("服".to_string(), "制服".to_string()), 1500);
    sakura.insert(("服".to_string(), "私服".to_string()), 1501);
    let options = BindOptionDecls {
        sakura_mustselect: BTreeSet::from(["服".to_string()]),
        ..BindOptionDecls::default()
    };
    BindResolver::new(sakura, BTreeMap::new(), options)
}

fn shown_states(ids: &[u32]) -> (ScopeStates, ActorKey) {
    let mut states = ScopeStates::new(DefaultBinds::uniform(build_static_bindset(ids)));
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(10));
    (states, scope)
}

/// 当てた: 既定のカテゴリの着衣は排他（同じカテゴリの 1301 が外れる）。
#[test]
fn applied_exclusive_on() {
    let (mut states, scope) = shown_states(&[1301]);
    let result = apply_named_bind(&mut states, &resolver(), &scope, "目", "閉じ", true);
    assert!(
        matches!(
            result,
            NamedBindResult::Applied {
                id: 1303,
                outcome: BindApplyOutcome::Changed(_)
            }
        ),
        "{result:?}"
    );
    assert_eq!(states.current_binds(&scope).ids(), &[1303]);
}

/// 当てた: mustselect でない OFF は外す。
#[test]
fn applied_off_removes() {
    let (mut states, scope) = shown_states(&[1301]);
    let result = apply_named_bind(&mut states, &resolver(), &scope, "目", "開き", false);
    assert!(
        matches!(result, NamedBindResult::Applied { id: 1301, .. }),
        "{result:?}"
    );
    assert_eq!(states.current_binds(&scope).ids(), &[] as &[u32]);
}

/// 名前の空間が無い: 数でないスコープ。状態は変わらない。
#[test]
fn no_namespace_for_non_numeric_scope() {
    let (mut states, _) = shown_states(&[1301]);
    let scope = ActorKey::from("x");
    let result = apply_named_bind(&mut states, &resolver(), &scope, "目", "閉じ", true);
    assert_eq!(result, NamedBindResult::NoNamespace);
    assert_eq!(states.current_binds(&ActorKey::from("0")).ids(), &[1301]);
}

/// 見つからない: 宣言の無いパーツ。状態は変わらない。
#[test]
fn not_found_for_undeclared_part() {
    let (mut states, scope) = shown_states(&[1301]);
    let result = apply_named_bind(&mut states, &resolver(), &scope, "目", "半目", true);
    assert_eq!(result, NamedBindResult::NotFound);
    assert_eq!(states.current_binds(&scope).ids(), &[1301]);
}

/// mustselect で外さなかった: mustselect の OFF は集合を変えない。
#[test]
fn mustselect_off_is_kept() {
    let (mut states, scope) = shown_states(&[1500]);
    let result = apply_named_bind(&mut states, &resolver(), &scope, "服", "制服", false);
    assert_eq!(result, NamedBindResult::MustSelectKept { id: 1500 });
    assert_eq!(states.current_binds(&scope).ids(), &[1500]);
}
