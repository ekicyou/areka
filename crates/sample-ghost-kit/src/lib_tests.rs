//! 窓口 [`SampleRoot`](super::SampleRoot) と登記表 [`SAMPLES`](super::SAMPLES) の自己テスト
//! （要件 1.1・1.2・1.3・1.4・1.5・7.4）。
//!
//! 窓口は `vendors/sample_ghost/<名>.nar` を展開した原本から**使い捨ての複製**を作って配る。
//! だから主張は 5 本——登記した検体が登記どおりの位置（`<根>/ghost/<名>`・
//! `<根>/balloon/<名>`）に実在し、値を捨てると複製ごと消える／未登録名は既知の 4 つを
//! 含む失敗になる／同梱していないバルーン名は既知の一覧を含む失敗になる／展開結果が
//! 登記と食い違えば理由付きの失敗になる／種別と同梱バルーンが登記されている。
//!
//! 「借用しか返さないので取得した値を捨てるとコンパイルできない」ことは、コンパイル自体を
//! 判定に使うので rustdoc の `compile_fail` 例（[`SampleRoot::root`](super::SampleRoot::root)・
//! [`SampleRoot::folder`](super::SampleRoot::folder)・
//! [`SampleRoot::balloon`](super::SampleRoot::balloon) の doc）が固定する。その対として、
//! 2 行だけ違う「値を持ったまま使う」形が**実際にコンパイルできる**例を隣に置いてあるので、
//! `compile_fail` が別の理由（import 違い・署名違い）で通る恒真ではないと分かる。
//!
//! # 食い違いの固定入力は自分で組む
//!
//! 登記と食い違う展開結果は、実物の検体では作れない（登記どおりの `.nar` しか置いていない）。
//! そこで [`crate::NarBuilder`] で最小の `.nar` を組み、私有の名前空間で複製まで作ってから
//! 窓口の照合へ渡す。通る経路は実物と同じ（[`SampleRoot::from_copy`](super::SampleRoot)）で、
//! 違うのは入力だけである。

use super::*;

/// 食い違いの検査で使う登記の行（同梱バルーンを 1 つ持つゴースト）。
const GHOST_WITH_BALLOON: Sample = Sample {
    name: "probe-window",
    kind: SampleKind::Ghost,
    balloons: &[PROBE_BALLOON],
};

/// 食い違いの検査で使う登記の行（同梱バルーンを持たないゴースト）。
const GHOST_ALONE: Sample = Sample {
    name: "probe-window",
    kind: SampleKind::Ghost,
    balloons: &[],
};

/// 固定入力の同梱バルーンの `directory`。
const PROBE_BALLOON: &str = "probe-window-balloon";

/// 他のテストと混ざらない私有の場所。破棄で丸ごと消える。
fn private_dir() -> WorkDir {
    WorkDir::new().expect("ビルド成果物の置き場の下は取れるはず")
}

/// 登記と突き合わせる最小の `.nar` を組んで置く。
///
/// `directory`・種別・同梱バルーンを引数で決められるので、展開結果が登記と食い違う形を
/// 作れる（フォルダ名違い・種別違い・同梱の欠け・同梱の余り）。
fn probe_nar(
    dir: &Path,
    tag: &str,
    kind: &str,
    directory: &str,
    companion: Option<&str>,
) -> PathBuf {
    let nar = dir.join(format!("{tag}.nar"));
    let mut lines = vec![
        "charset,UTF-8".to_owned(),
        format!("type,{kind}"),
        format!("name,{directory}"),
        format!("directory,{directory}"),
    ];
    if let Some(balloon) = companion {
        lines.push(format!("balloon.directory,{balloon}"));
        lines.push(format!("balloon.source.directory,{balloon}"));
    }
    let spelled: Vec<&str> = lines.iter().map(String::as_str).collect();
    let mut builder = NarBuilder::new()
        .file("install.txt", &install_txt(&spelled))
        .done()
        .file("ghost/master/descript.txt", b"charset,UTF-8\r\n")
        .done();
    if let Some(balloon) = companion {
        builder = builder
            .file(format!("{balloon}/descript.txt"), b"charset,UTF-8\r\n")
            .done();
    }
    builder
        .write_to(&nar)
        .expect("固定入力の .nar を置けるはず");
    nar
}

/// 固定入力を組み、複製まで作ってから窓口の照合へ渡す。返すのは `(表示, 期待, 実際)`。
fn refused(
    home: &Path,
    fixtures: &Path,
    tag: &str,
    kind: &str,
    directory: &str,
    companion: Option<&str>,
    registry: &'static Sample,
) -> (String, String, Vec<String>) {
    let nar = probe_nar(fixtures, tag, kind, directory, companion);
    let copy = devroot::fresh_root_in(home, tag, &nar).expect("固定入力の複製は取れるはず");
    let err = SampleRoot::from_copy(registry, copy)
        .err()
        .unwrap_or_else(|| panic!("登記と食い違う展開結果（{tag}）は失敗になるはず"));
    let shown = err.to_string();
    let SampleError::RegistryMismatch {
        sample,
        expected,
        installed,
    } = err
    else {
        panic!("食い違いは RegistryMismatch であるはず: {shown}");
    };
    assert_eq!(sample, registry.name, "失敗に載る検体名");
    (shown, expected, installed)
}

/// 登記した 4 つの検体は、登記の行から導いた位置に実在し、値を捨てると複製ごと消える
/// （要件 1.1・1.2・1.3・7.4）。
///
/// 位置は `<根>/<格納先>/<名>` の等式で判定する。検体ごとの第 2 の表や専用関数があれば
/// この等式は破れる（要件 1.5）。
#[test]
fn every_registered_sample_lands_where_its_registry_row_says() {
    assert_eq!(SAMPLES.len(), 4, "登記は検体 4 つ");
    for sample in SAMPLES {
        let acquired = SampleRoot::acquire(sample.name).expect("登記済みの名前は取得できるはず");

        let root = acquired.root().to_path_buf();
        assert!(
            root.is_absolute(),
            "窓口は絶対パスを返す: {}",
            root.display()
        );
        assert!(root.is_dir(), "根が実在しない: {}", root.display());

        let store = match sample.kind {
            SampleKind::Ghost => "ghost",
            SampleKind::Balloon => "balloon",
        };
        assert_eq!(
            acquired.folder(),
            root.join(store).join(sample.name),
            "検体 {} の位置が登記の行から導かれていない",
            sample.name
        );
        assert!(
            acquired.folder().is_dir(),
            "検体 {} のフォルダが実在しない: {}",
            sample.name,
            acquired.folder().display()
        );

        for balloon in sample.balloons {
            let path = acquired
                .balloon(balloon)
                .expect("登記済みの同梱バルーンは引けるはず");
            assert_eq!(
                path,
                root.join("balloon").join(balloon),
                "同梱バルーン {balloon} は `<根>/balloon/<名>`（要件 1.3）"
            );
            assert!(
                path.is_dir(),
                "同梱バルーン {balloon} のフォルダが実在しない: {}",
                path.display()
            );
        }

        drop(acquired);
        assert!(
            !root.exists(),
            "値を捨てたのに複製が残っている: {}",
            root.display()
        );
    }
}

/// 未登録の検体名は、既知の名前 4 つを含む理由付きの失敗になる（要件 1.4）。
#[test]
fn unknown_sample_fails_with_all_four_known_names() {
    let err = SampleRoot::acquire("no-such-ghost").expect_err("未登録名は失敗するはず");
    let SampleError::UnknownSample { requested, known } = &err else {
        panic!("未登録名の失敗は UnknownSample であるはず: {err:?}");
    };
    assert_eq!(requested, "no-such-ghost");
    assert_eq!(
        known,
        &[
            "emo2",
            "R_POST_and_KOMAINU",
            "emo2-kakukaku-offsetdpi",
            "emo2-kakukaku-wplimit",
        ]
    );
    let shown = err.to_string();
    for name in [
        "emo2",
        "R_POST_and_KOMAINU",
        "emo2-kakukaku-offsetdpi",
        "emo2-kakukaku-wplimit",
    ] {
        assert!(shown.contains(name), "失敗の表示に {name} が無い: {shown}");
    }
}

/// 同時にインストールしていないバルーン名は、既知の一覧を含む失敗になる（要件 1.4）。
///
/// 同梱バルーンを持たない検体では既知の一覧が**空**になることも明示して確かめる
/// （引き算で導ける 0 は沈黙と同じなので書く）。同梱 0 の側は登記の中で `.nar` が一番
/// 小さい検体を使う（主張は種別に依らないので、複製の小さい方を選ぶ）。
#[test]
fn unknown_balloon_fails_with_the_known_balloon_list() {
    let emo2 = SampleRoot::acquire("emo2").expect("emo2 は登記済み");
    let err = emo2
        .balloon("emo2-kakukaku-nope")
        .expect_err("同梱していない名前は失敗するはず");
    let SampleError::UnknownBalloon {
        sample,
        requested,
        known,
    } = &err
    else {
        panic!("同梱外の失敗は UnknownBalloon であるはず: {err:?}");
    };
    assert_eq!(*sample, "emo2");
    assert_eq!(requested, "emo2-kakukaku-nope");
    assert_eq!(known, &["emo2-kakukaku"]);
    assert!(
        err.to_string().contains("emo2-kakukaku"),
        "失敗の表示に既知の名前が無い"
    );

    let plain = SampleRoot::acquire("emo2-kakukaku-wplimit").expect("登記済み");
    let err = plain
        .balloon("emo2-kakukaku")
        .expect_err("同梱 0 の検体では何も引けない");
    let SampleError::UnknownBalloon { known, .. } = &err else {
        panic!("UnknownBalloon であるはず: {err:?}");
    };
    assert!(
        known.is_empty(),
        "同梱バルーンを持たない検体の既知一覧は空: {known:?}"
    );
}

/// 展開結果の要素が登記と食い違えば、理由付きの失敗になる（要件 1.1・1.3）。
///
/// 食い違いは 4 通り——本体のフォルダ名・種別・同梱の欠け・同梱の余り。対照として
/// 登記どおりの `.nar` が通ることも見る（照合が「常に赤」ではないことの較正）。
#[test]
fn a_tree_that_disagrees_with_the_registry_row_is_refused() {
    let home = private_dir();
    let fixtures = private_dir();

    // 対照——登記どおりに展開されるなら通り、位置も登記の行から導かれる。
    let good = probe_nar(
        fixtures.path(),
        "probe-agrees",
        "ghost",
        GHOST_WITH_BALLOON.name,
        Some(PROBE_BALLOON),
    );
    let copy = devroot::fresh_root_in(home.path(), "probe-agrees", &good).expect("複製");
    let acquired =
        SampleRoot::from_copy(&GHOST_WITH_BALLOON, copy).expect("登記どおりなら通るはず");
    assert!(acquired.folder().is_dir(), "本体のフォルダが実在する");
    assert!(
        acquired.balloon(PROBE_BALLOON).expect("同梱").is_dir(),
        "同梱バルーンのフォルダが実在する"
    );
    drop(acquired);

    // ⑴ 本体のフォルダ名が違う。
    let (shown, expected, installed) = refused(
        home.path(),
        fixtures.path(),
        "probe-other-folder",
        "ghost",
        "probe-other",
        Some(PROBE_BALLOON),
        &GHOST_WITH_BALLOON,
    );
    assert_eq!(
        installed,
        ["balloon/probe-window-balloon", "ghost/probe-other"]
    );
    assert_eq!(expected, "balloon/probe-window-balloon, ghost/probe-window");
    assert!(
        shown.contains("probe-other") && shown.contains("probe-window"),
        "失敗の表示に両側の綴りが無い: {shown}"
    );

    // ⑵ 種別が違う（バルーンとして置かれた）。
    let (_, expected, installed) = refused(
        home.path(),
        fixtures.path(),
        "probe-other-kind",
        "balloon",
        GHOST_ALONE.name,
        None,
        &GHOST_ALONE,
    );
    assert_eq!(installed, ["balloon/probe-window"]);
    assert_eq!(expected, "ghost/probe-window");

    // ⑶ 同梱バルーンが置かれていない。
    let (_, expected, installed) = refused(
        home.path(),
        fixtures.path(),
        "probe-missing-companion",
        "ghost",
        GHOST_WITH_BALLOON.name,
        None,
        &GHOST_WITH_BALLOON,
    );
    assert_eq!(installed, ["ghost/probe-window"]);
    assert_eq!(expected, "balloon/probe-window-balloon, ghost/probe-window");

    // ⑷ 登記していない同梱バルーンが置かれた。
    let (_, expected, installed) = refused(
        home.path(),
        fixtures.path(),
        "probe-extra-companion",
        "ghost",
        GHOST_ALONE.name,
        Some(PROBE_BALLOON),
        &GHOST_ALONE,
    );
    assert_eq!(
        installed,
        ["balloon/probe-window-balloon", "ghost/probe-window"]
    );
    assert_eq!(expected, "ghost/probe-window");
}

/// 種別と同梱バルーン名が登記されている（要件 1.3・位置を組み立てる元）。
#[test]
fn registry_records_kind_and_bundled_balloons() {
    let by_name = |name: &str| SAMPLES.iter().find(|s| s.name == name).expect("登記済み");
    assert_eq!(by_name("emo2").kind, SampleKind::Ghost);
    assert_eq!(by_name("emo2").balloons, &["emo2-kakukaku"]);
    assert_eq!(by_name("R_POST_and_KOMAINU").kind, SampleKind::Ghost);
    assert!(
        by_name("R_POST_and_KOMAINU").balloons.is_empty(),
        "R_POST_and_KOMAINU は同梱バルーン 0"
    );
    for name in ["emo2-kakukaku-offsetdpi", "emo2-kakukaku-wplimit"] {
        assert_eq!(by_name(name).kind, SampleKind::Balloon);
        assert!(by_name(name).balloons.is_empty(), "{name} は同梱バルーン 0");
    }
}
