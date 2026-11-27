//! 今の土台の面の 1 か所の答えの檻（状態を持たない・元へ戻る・無い面）。

use crate::bind::BindSet;
use crate::pattern::PatternState;
use crate::plan_base::tests::{base_frame, world};

/// 同じ入力には同じ答え、コマが替われば次の答えはその時点の状態だけで決まる。
#[test]
fn the_answer_follows_the_state_without_memory() {
    let (world, _atlas) = world();
    let none = BindSet::default();
    let mut pattern = PatternState::default();
    assert_eq!(world.current_base_surface(0, &none, &pattern), 0);
    pattern.set(10, base_frame(1, 7, 3));
    assert_eq!(world.current_base_surface(0, &none, &pattern), 1);
    assert_eq!(world.current_base_surface(0, &none, &pattern), 1);
    assert_eq!(
        world.current_base_surface(0, &BindSet::from_ids([20]), &pattern),
        2
    );
    pattern.remove(10);
    assert_eq!(world.current_base_surface(0, &none, &pattern), 0);
}

/// 面の表に無い面は、その面の番号を答える。
#[test]
fn an_unknown_surface_answers_itself() {
    let (world, _atlas) = world();
    let answer = world.current_base_surface(500, &BindSet::default(), &PatternState::default());
    assert_eq!(answer, 500);
}
