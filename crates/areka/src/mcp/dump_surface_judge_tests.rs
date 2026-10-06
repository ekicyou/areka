//! 判断と成功の本文の決定論テスト（要件 7.2・7.3）。事実は手書きの表で与える。

use super::*;

/// 手書きの事実の表。surface ID はシェル全体で 1 つの集合（スコープを問わない）。
struct Table {
    /// 絵の資産が組まれているスコープ。
    scopes: &'static [u32],
    /// シェルに在る生の surface ID。
    surfaces: &'static [u32],
    /// スコープごとの最後に表示した surface ID。
    last: &'static [(u32, u32)],
}

impl ShellFacts for Table {
    fn scope_exists(&self, scope: u32) -> bool {
        self.scopes.contains(&scope)
    }
    fn surface_exists(&self, _scope: u32, surface_id: u32) -> bool {
        self.surfaces.contains(&surface_id)
    }
    fn last_shown(&self, scope: u32) -> Option<u32> {
        self.last
            .iter()
            .find(|(s, _)| *s == scope)
            .map(|(_, id)| *id)
    }
}

/// ふつうのゴースト: 窓はスコープ 0・1・2 に在るが、絵の資産はスコープ 0・1 だけ（2 は窓だけ）。
/// 生の ID は 0・3・10（10 はスコープ 1 用の絵）。5 は別名（`\s[5]`）にしか無い番号。
/// スコープ 0 は surface 3 を最後に表示した（今は `\s[-1]` で隠していても残る）。スコープ 1 は一度も表示していない。
const GHOST: Table = Table {
    scopes: &[0, 1],
    surfaces: &[0, 3, 10],
    last: &[(0, 3)],
};

/// 何でも在ると答える表（数の範囲の検査だけで断られることを確かめる）。
struct Everything;

impl ShellFacts for Everything {
    fn scope_exists(&self, _scope: u32) -> bool {
        true
    }
    fn surface_exists(&self, _scope: u32, _surface_id: u32) -> bool {
        true
    }
    fn last_shown(&self, _scope: u32) -> Option<u32> {
        Some(0)
    }
}

fn surface(scope: Option<i64>, id: Option<i64>) -> Result<SurfacePlan, Refusal> {
    judge_surface(Some(&GHOST), scope, id)
}

// ---- dump_surface の判断（要件 7.2） ----

#[test]
fn omitted_scope_is_scope_zero() {
    assert_eq!(
        surface(None, None),
        Ok(SurfacePlan::Shown {
            scope: 0,
            surface_id: 3
        })
    );
    assert_eq!(
        surface(None, Some(0)),
        Ok(SurfacePlan::Alone {
            scope: 0,
            surface_id: 0
        })
    );
}

#[test]
fn negative_scope_is_no_such_scope() {
    assert_eq!(surface(Some(-1), None), Err(NO_SUCH_SCOPE));
    assert_eq!(surface(Some(i64::MIN), Some(0)), Err(NO_SUCH_SCOPE));
}

#[test]
fn scope_without_assets_is_no_such_scope() {
    assert_eq!(surface(Some(7), None), Err(NO_SUCH_SCOPE));
    assert_eq!(surface(Some(7), Some(0)), Err(NO_SUCH_SCOPE));
}

#[test]
fn scope_with_window_but_no_assets_is_no_such_scope() {
    // スコープ 2 は窓だけ在って絵の資産が無い。
    assert_eq!(surface(Some(2), None), Err(NO_SUCH_SCOPE));
    assert_eq!(surface(Some(2), Some(0)), Err(NO_SUCH_SCOPE));
}

#[test]
fn scope_above_i32_max_is_no_such_scope_even_if_facts_say_yes() {
    let max = i64::from(i32::MAX);
    assert_eq!(
        judge_surface(Some(&Everything), Some(max), None),
        Ok(SurfacePlan::Shown {
            scope: i32::MAX as u32,
            surface_id: 0
        })
    );
    assert_eq!(
        judge_surface(Some(&Everything), Some(max + 1), None),
        Err(NO_SUCH_SCOPE)
    );
    assert_eq!(
        judge_surface(Some(&Everything), Some(i64::MAX), Some(0)),
        Err(NO_SUCH_SCOPE)
    );
}

#[test]
fn missing_surface_id_is_no_such_surface() {
    assert_eq!(surface(Some(0), Some(4)), Err(NO_SUCH_SURFACE));
}

#[test]
fn negative_surface_id_is_no_such_surface() {
    assert_eq!(surface(Some(0), Some(-1)), Err(NO_SUCH_SURFACE));
    assert_eq!(
        judge_surface(Some(&Everything), Some(0), Some(-1)),
        Err(NO_SUCH_SURFACE)
    );
}

#[test]
fn surface_id_above_u32_max_is_no_such_surface_even_if_facts_say_yes() {
    let max = i64::from(u32::MAX);
    assert_eq!(
        judge_surface(Some(&Everything), Some(0), Some(max)),
        Ok(SurfacePlan::Alone {
            scope: 0,
            surface_id: u32::MAX
        })
    );
    assert_eq!(
        judge_surface(Some(&Everything), Some(0), Some(max + 1)),
        Err(NO_SUCH_SURFACE)
    );
}

#[test]
fn number_only_in_alias_is_no_such_surface() {
    // 5 は `get_expression_table` に出る別名の番号で、生の ID には無い。
    assert_eq!(surface(Some(0), Some(5)), Err(NO_SUCH_SURFACE));
}

#[test]
fn surface_id_for_another_scope_succeeds() {
    // 10 はスコープ 1 用の絵だが、スコープ 0 で指定しても成功（シェル全体から探す）。
    assert_eq!(
        surface(Some(0), Some(10)),
        Ok(SurfacePlan::Alone {
            scope: 0,
            surface_id: 10
        })
    );
}

#[test]
fn hidden_scope_succeeds_with_the_last_shown_surface() {
    // `\s[-1]` で隠しているだけ＝最後に表示した ID が残っている。
    assert_eq!(
        surface(Some(0), None),
        Ok(SurfacePlan::Shown {
            scope: 0,
            surface_id: 3
        })
    );
}

#[test]
fn never_shown_scope_is_not_shown_yet() {
    assert_eq!(surface(Some(1), None), Err(NOT_SHOWN_YET));
}

#[test]
fn never_shown_scope_still_renders_a_given_surface_alone() {
    assert_eq!(
        surface(Some(1), Some(10)),
        Ok(SurfacePlan::Alone {
            scope: 1,
            surface_id: 10
        })
    );
}

#[test]
fn ghost_without_window_is_no_window() {
    assert_eq!(judge_surface(None, None, None), Err(NO_WINDOW));
    assert_eq!(judge_surface(None, Some(0), Some(0)), Err(NO_WINDOW));
}

#[test]
fn first_matching_condition_wins_in_order() {
    // 窓 → スコープ: 窓も無くスコープも負なら窓。
    assert_eq!(judge_surface(None, Some(-1), Some(-1)), Err(NO_WINDOW));
    // スコープ → surface ID: 無いスコープと無い ID なら スコープ。
    assert_eq!(surface(Some(7), Some(4)), Err(NO_SUCH_SCOPE));
    assert_eq!(surface(Some(-1), Some(-1)), Err(NO_SUCH_SCOPE));
    // surface ID → 未表示: 一度も表示していないスコープで無い ID なら surface ID。
    assert_eq!(surface(Some(1), Some(4)), Err(NO_SUCH_SURFACE));
}

// ---- dump_balloon の判断（窓・スコープ・成功） ----

#[test]
fn balloon_ghost_without_window_is_no_window() {
    assert_eq!(judge_balloon(None, Some(-1)), Err(NO_WINDOW));
}

#[test]
fn balloon_missing_scope_is_no_such_scope() {
    assert_eq!(judge_balloon(Some(&GHOST), Some(-1)), Err(NO_SUCH_SCOPE));
    assert_eq!(judge_balloon(Some(&GHOST), Some(2)), Err(NO_SUCH_SCOPE));
    assert_eq!(
        judge_balloon(Some(&Everything), Some(i64::from(i32::MAX) + 1)),
        Err(NO_SUCH_SCOPE)
    );
}

#[test]
fn balloon_succeeds_with_the_scope_number() {
    assert_eq!(judge_balloon(Some(&GHOST), None), Ok(0));
    // 一度も表示していないスコープでも、バルーンの判断はスコープまで。
    assert_eq!(judge_balloon(Some(&GHOST), Some(1)), Ok(1));
}

// ---- 成功の本文（要件 7.3） ----

#[test]
fn shown_text_is_verbatim() {
    assert_eq!(
        shown_text(0, 3),
        "scope 0, surface 3 as currently shown (with running animations and dressups, before scaling and transparency)"
    );
    assert_eq!(
        shown_text(1, 1050),
        "scope 1, surface 1050 as currently shown (with running animations and dressups, before scaling and transparency)"
    );
}

#[test]
fn alone_text_is_verbatim() {
    assert_eq!(
        alone_text(0, 5),
        "scope 0, surface 5 rendered alone in its initial state (not what is on the screen now)"
    );
    assert_eq!(
        alone_text(1, 10),
        "scope 1, surface 10 rendered alone in its initial state (not what is on the screen now)"
    );
}

#[test]
fn balloon_text_is_verbatim() {
    assert_eq!(
        balloon_text(0),
        "balloon of scope 0 as last drawn (before scaling and transparency; kept even if the balloon is hidden now)"
    );
    assert_eq!(
        balloon_text(1),
        "balloon of scope 1 as last drawn (before scaling and transparency; kept even if the balloon is hidden now)"
    );
}
