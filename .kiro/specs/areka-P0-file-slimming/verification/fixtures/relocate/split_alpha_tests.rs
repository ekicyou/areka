use super::super::*;
use super::test_support::{make_pair, LIMIT};

#[test]
fn add_beta_case() {
    assert_eq!(add(LIMIT, 1), 8);
}

#[test]
fn add_alpha_case() {
    let (a, b) = make_pair();
    assert_eq!(add(a, b), 7);
}
