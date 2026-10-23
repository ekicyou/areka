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

#[test]
fn reader_asks_registry_only_when_mirror_lacks_name() {
    use crate::asker::{AskerContext, AskerId};
    use crate::mirror::{MirrorImage, SharedMirror};
    use crate::reader::SylphyaReader;
    use crate::value::DottedResolution;

    let a = AskerId::new("ghost/a");
    let mut img = MirrorImage::empty();
    img.dotted_per_asker
        .entry(a.clone())
        .or_default()
        .insert("x.s(0)".into(), "MIRROR".into());
    let reader = SylphyaReader::new(SharedMirror::new(Arc::new(img)));
    // 複製へ登録しても元の読み口から引ける（表は共有）。
    reader
        .clone()
        .register_pull(PullShape::parse("x.s(*)").unwrap(), Arc::new(Fixed("PULL")));
    let ctx = AskerContext { asker: a };

    // 写しに在る名前は写しの値（読み手を引かない）。
    assert_eq!(
        reader.resolve_dotted_str(&ctx, "x.s(0)"),
        DottedResolution::Value("MIRROR".into())
    );
    // 写しに無い名前は読み手の答え（文字・読んだ名前の両方の入口）。
    assert_eq!(
        reader.resolve_dotted_str(&ctx, "x.s(1)"),
        DottedResolution::Value("PULL".into())
    );
    assert_eq!(
        reader.resolve_dotted(&ctx, &parse_dotted("x.s(2)").unwrap()),
        DottedResolution::Value("PULL".into())
    );
    // 形の合わない名前は今までどおり NotFound。
    assert_eq!(
        reader.resolve_dotted_str(&ctx, "x.t(1)"),
        DottedResolution::NotFound
    );
}
