//! 文字の層向けの `\s` の答え（`SurfaceTarget::for_text`）の決定論テスト。

use super::*;
use areka_emo_compose::SurfaceKeyOutcome;

fn ids() -> BTreeSet<u32> {
    BTreeSet::from([0, 2100])
}

#[test]
fn id_in_table_is_shown() {
    assert_eq!(
        SurfaceTarget::Show(2100).for_text(&ids()),
        SurfaceKeyOutcome::Show(2100)
    );
}

#[test]
fn id_absent_from_table_is_unresolved() {
    assert_eq!(
        SurfaceTarget::Show(9999).for_text(&ids()),
        SurfaceKeyOutcome::Unresolved
    );
}

#[test]
fn hide_is_hide_even_with_empty_table() {
    assert_eq!(
        SurfaceTarget::Hide.for_text(&BTreeSet::new()),
        SurfaceKeyOutcome::Hide
    );
}

#[test]
fn unresolved_stays_unresolved() {
    assert_eq!(
        SurfaceTarget::Unresolved.for_text(&ids()),
        SurfaceKeyOutcome::Unresolved
    );
}
