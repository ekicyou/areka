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

// ---- 読み口を通した檻（タスク 1.3・要件 1.9・3.6・5.2・6.3・6.4・6.6・7.4・7.5） ----

mod via_reader {
    use super::*;
    use crate::asker::{AskerContext, AskerId};
    use crate::mirror::{MirrorImage, SharedMirror};
    use crate::reader::SylphyaReader;
    use crate::value::DottedResolution;
    use std::sync::mpsc;
    use std::time::Duration;

    const RECT: &str = "currentghost.scope(*).rect";

    fn ctx() -> AskerContext {
        AskerContext {
            asker: AskerId::new("ghost/a"),
        }
    }

    fn val(s: &str) -> DottedResolution {
        DottedResolution::Value(s.into())
    }

    /// 聞かれた名前の正準形をそのまま答える読み手（どの名前が届いたかを見る）。
    struct Echo;

    impl PullSource for Echo {
        fn read(&self, path: &PropPath) -> Option<String> {
            Some(path.to_canonical_string())
        }
    }

    struct Nothing;

    impl PullSource for Nothing {
        fn read(&self, _path: &PropPath) -> Option<String> {
            None
        }
    }

    /// 読み手の中から同じ読み口で別の名前を 2 つ読む（写しに在る名前と、別の形の読み手の名前）。
    struct Reentrant(SylphyaReader);

    impl PullSource for Reentrant {
        fn read(&self, _path: &PropPath) -> Option<String> {
            let a = self.0.resolve_dotted_str(&ctx(), "inner.mirror");
            let b = self.0.resolve_dotted_str(&ctx(), "inner.pull(0)");
            Some(format!("{a:?}/{b:?}"))
        }
    }

    #[test]
    fn shape_variants_reach_the_same_reader_and_others_do_not() {
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(MirrorImage::empty())));
        reader.register_pull(PullShape::parse(RECT).unwrap(), Arc::new(Echo));
        for name in [
            "currentghost.scope(0).rect",
            "currentghost.scope(00).rect",
            "CURRENTGHOST.SCOPE(0).RECT",
        ] {
            assert!(
                matches!(
                    reader.resolve_dotted_str(&ctx(), name),
                    DottedResolution::Value(_)
                ),
                "{name}"
            );
        }
        for name in ["currentghost.scope.rect", "currentghost.scope(0).rect.x"] {
            assert_eq!(
                reader.resolve_dotted_str(&ctx(), name),
                DottedResolution::NotFound,
                "{name}"
            );
        }
    }

    #[test]
    fn global_mirror_wins_and_registration_changes_nothing_else() {
        let mut img = MirrorImage::empty();
        img.dotted_global
            .insert("currentghost.scope(0).rect".into(), "GLOBAL".into());
        img.dotted_global
            .insert("other.name".into(), "OTHER".into());
        img.flat_global.insert("username".into(), "U".into());
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(img)));

        let snap_before = reader.talk_snapshot(&ctx());
        let other_before = reader.resolve_dotted_str(&ctx(), "other.name");
        let missing_before = reader.resolve_dotted_str(&ctx(), "no.such(0)");

        reader.register_pull(PullShape::parse(RECT).unwrap(), Arc::new(Fixed("PULL")));

        // 写しの大域の区画に在る名前は読み手を引かない。
        assert_eq!(
            reader.resolve_dotted_str(&ctx(), "currentghost.scope(0).rect"),
            val("GLOBAL")
        );
        // 写しに無い同じ形の名前だけが読み手に届く。
        assert_eq!(
            reader.resolve_dotted_str(&ctx(), "currentghost.scope(1).rect"),
            val("PULL")
        );
        assert_eq!(
            reader.resolve_dotted_str(&ctx(), "other.name"),
            other_before
        );
        assert_eq!(
            reader.resolve_dotted_str(&ctx(), "no.such(0)"),
            missing_before
        );
        assert_eq!(reader.talk_snapshot(&ctx()), snap_before);
    }

    #[test]
    fn clone_made_before_registration_answers_and_reregister_replaces() {
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(MirrorImage::empty())));
        let early = reader.clone();
        let name = "currentghost.scope(0).rect";
        assert_eq!(
            early.resolve_dotted_str(&ctx(), name),
            DottedResolution::NotFound
        );

        reader.register_pull(PullShape::parse(RECT).unwrap(), Arc::new(Fixed("first")));
        assert_eq!(early.resolve_dotted_str(&ctx(), name), val("first"));

        // 実行系の作り直しで同じ形に登録し直すと、どの複製でも新しい読み手が答える。
        reader.register_pull(PullShape::parse(RECT).unwrap(), Arc::new(Fixed("second")));
        assert_eq!(early.resolve_dotted_str(&ctx(), name), val("second"));
        assert_eq!(reader.resolve_dotted_str(&ctx(), name), val("second"));
    }

    #[test]
    fn reader_returning_nothing_is_not_found() {
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(MirrorImage::empty())));
        reader.register_pull(PullShape::parse(RECT).unwrap(), Arc::new(Nothing));
        assert_eq!(
            reader.resolve_dotted_str(&ctx(), "currentghost.scope(0).rect"),
            DottedResolution::NotFound
        );
    }

    #[test]
    fn reentrant_read_does_not_stall_and_does_not_store() {
        let mut img = MirrorImage::empty();
        img.dotted_global.insert("inner.mirror".into(), "M".into());
        let shared = SharedMirror::new(Arc::new(img));
        let reader = SylphyaReader::new(shared.clone());
        reader.register_pull(
            PullShape::parse("inner.pull(*)").unwrap(),
            Arc::new(Fixed("P")),
        );
        reader.register_pull(
            PullShape::parse(RECT).unwrap(),
            Arc::new(Reentrant(reader.clone())),
        );
        let before = shared.load();

        // 止まったらテストの走りごと固まらないよう、別のスレッドで読んで期限つきで待つ。
        let (tx, rx) = mpsc::channel();
        let r = reader.clone();
        std::thread::spawn(move || {
            let _ = tx.send(r.resolve_dotted_str(&ctx(), "currentghost.scope(0).rect"));
        });
        let got = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("読み手の中からの読み直しで止まった");
        assert_eq!(got, val(r#"Value("M")/Value("P")"#));

        // 読み手の答えは写しに書かない（世代も中身も読む前のまま）。
        let after = shared.load();
        assert_eq!(after.epoch, before.epoch);
        assert!(Arc::ptr_eq(&after, &before));
        assert!(
            !after
                .dotted_global
                .contains_key("currentghost.scope(0).rect")
        );
    }
}
