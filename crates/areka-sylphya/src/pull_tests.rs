//! 形と登録の表の単体の檻（spec mcp-scope-rect-property タスク 1.1・要件 7.1・7.4）。

use super::*;
use crate::key::parse_dotted;

struct Fixed(&'static str);

impl PullSource for Fixed {
    fn read(&self, _path: &PropPath) -> Option<String> {
        Some(self.0.to_string())
    }
}

#[test]
fn selector_folds_to_star_and_case_folds() {
    // 選択子の中身・英字の大小が違っても同じ形になる。
    let want = "currentghost.scope(*).rect";
    for name in [
        "currentghost.scope(0).rect",
        "currentghost.scope(*).rect",
        "CURRENTGHOST.SCOPE(00).RECT",
    ] {
        assert_eq!(PullShape::parse(name).unwrap().as_str(), want, "{name}");
    }
    let path = parse_dotted("currentghost.scope(1).balloon.rect").unwrap();
    assert_eq!(
        PullShape::of(&path).as_str(),
        "currentghost.scope(*).balloon.rect"
    );
}

#[test]
fn selectorless_segment_is_a_different_shape() {
    assert_eq!(
        PullShape::parse("currentghost.scope.rect")
            .unwrap()
            .as_str(),
        "currentghost.scope.rect"
    );
    assert_ne!(
        PullShape::parse("currentghost.scope.rect").unwrap(),
        PullShape::parse("currentghost.scope(0).rect").unwrap()
    );
}

#[test]
fn parse_error_passes_through() {
    assert_eq!(PullShape::parse(""), Err(KeyParseError::Empty));
    assert_eq!(
        PullShape::parse("a.scope(0"),
        Err(KeyParseError::UnclosedParen { at: 1 })
    );
}

#[test]
fn registry_is_shared_by_clones_and_reregister_replaces() {
    let reg = PullRegistry::default();
    let clone = reg.clone();
    let path = parse_dotted("currentghost.scope(0).rect").unwrap();
    assert!(clone.find(&path).is_none());

    let shape = PullShape::parse("currentghost.scope(*).rect").unwrap();
    reg.register(shape.clone(), Arc::new(Fixed("first")));
    assert_eq!(
        clone.find(&path).unwrap().read(&path).as_deref(),
        Some("first")
    );

    clone.register(shape, Arc::new(Fixed("second")));
    assert_eq!(
        reg.find(&path).unwrap().read(&path).as_deref(),
        Some("second")
    );

    // 選択子の無い名前は別の形なので引けない。
    assert!(
        reg.find(&parse_dotted("currentghost.scope.rect").unwrap())
            .is_none()
    );
}

#[test]
fn registry_survives_poison() {
    let reg = PullRegistry::default();
    let poisoner = reg.clone();
    let _ = std::thread::spawn(move || {
        let _guard = poisoner.inner.write().unwrap();
        panic!("poison the lock");
    })
    .join();
    assert!(reg.inner.is_poisoned());
    let shape = PullShape::parse("a.b(*)").unwrap();
    reg.register(shape, Arc::new(Fixed("v")));
    let path = parse_dotted("a.b(3)").unwrap();
    assert_eq!(reg.find(&path).unwrap().read(&path).as_deref(), Some("v"));
}
