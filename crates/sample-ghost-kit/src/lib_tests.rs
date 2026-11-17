//! 窓口 [`SampleRoot`](super::SampleRoot) と登記表 [`SAMPLES`](super::SAMPLES) の自己テスト
//! （要件 1.1・1.2・1.3・1.4・1.5・7.4・8.4・9.5。登記の往復の `emily4` の枝は spec:
//! `areka-P0-emily-ghost-verification` 要件 1.10・2.3・7.3、`dot_sakura` の枝は spec:
//! `areka-P0-dot-sakura-specimen` 要件 2.4・5.7）。spec の名を添えていない番号は spec
//! `areka-P0-nar-install` のもの（ほかの spec の番号はその場で spec の名を添える）。
//!
//! 窓口は検体の書庫（`vendors/sample_ghost/<名>.nar` か、URL の行なら取り寄せの置き場）を
//! 展開した原本から**使い捨ての複製**を作って配る。
//! だから主張は次のとおり——登記した検体が登記どおりの位置（`<根>/ghost/<名>`・
//! `<根>/balloon/<名>`・同梱バルーンはゴーストの中の `<根>/ghost/<名>/balloon/<名>`）に実在し、値を捨てると複製ごと消える／未登録名は既知の名前を
//! 含む失敗になる／同梱していないバルーン名は既知の一覧を含む失敗になる／展開結果が
//! 登記と食い違えば理由付きの失敗になる／登記した各 `.nar` を空の根へ展開すると要素が
//! 登記の 1 行とちょうど一致する（登記の往復。取り出し元の無い同梱を名乗る `emily4` は
//! 同梱を飛ばしてゴーストだけを入れ、`dot_sakura` は警告も飛ばした同梱も無くゴーストと
//! 同梱を入れて最上位の普通のファイルを本体の直下に置く）／取得した根へ書いた起動記録が
//! 次に取得した根に現れない／URL の行はぱすたの 1 行だけで、書庫は Vendored の行だけが
//! リポジトリに置く（spec `areka-P0-sample-url-fetch`）／2 つ目のシェルを写すと写した先の
//! `name` だけが変わり、元は残る（spec `areka-P0-shell-balloon-switch`）／失敗の表示は
//! ASCII だけで書く。
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
    source: SampleSource::Vendored,
};

/// 食い違いの検査で使う登記の行（同梱バルーンを持たないゴースト）。
const GHOST_ALONE: Sample = Sample {
    name: "probe-window",
    kind: SampleKind::Ghost,
    balloons: &[],
    source: SampleSource::Vendored,
};

/// 固定入力の同梱バルーンの `directory`。
const PROBE_BALLOON: &str = "probe-window-balloon";

/// 他のテストと混ざらない私有の場所。破棄で丸ごと消える。
fn private_dir() -> WorkDir {
    WorkDir::new().expect("ビルド成果物の置き場の下は取れるはず")
}

/// 登記と突き合わせる最小の `.nar` のバイト列を組む（ファイルを介さない）。
///
/// `directory`・種別・同梱バルーンを引数で決められるので、展開結果が登記と食い違う形を
/// 作れる（フォルダ名違い・種別違い・同梱の欠け・同梱の余り）。
fn probe_nar(kind: &str, directory: &str, companion: Option<&str>) -> Vec<u8> {
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
    builder.bytes()
}

/// 固定入力を組み、複製まで作ってから窓口の照合へ渡す。返すのは `(表示, 期待, 実際)`。
fn refused(
    home: &Path,
    tag: &str,
    kind: &str,
    directory: &str,
    companion: Option<&str>,
    registry: &'static Sample,
) -> (String, String, Vec<String>) {
    let nar = probe_nar(kind, directory, companion);
    let copy =
        devroot::fresh_root_in(home, tag, || Ok(nar.clone())).expect("固定入力の複製は取れるはず");
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

/// 登記表の URL の行の `(名前, URL)`。
fn fetched_rows() -> Vec<(&'static str, &'static str)> {
    SAMPLES
        .iter()
        .filter_map(|s| match s.source {
            SampleSource::Fetched { url } => Some((s.name, url)),
            SampleSource::Vendored => None,
        })
        .collect()
}

/// URL の行はぱすた（`pasta-in-windows`）の 1 行だけで、URL は公開の置き場を指す
/// （spec `areka-P0-sample-url-fetch` 要件 1.1・1.4・5.2）。書庫をリポジトリに置く行が
/// `Vendored` であることは `only_vendored_rows_have_a_nar_in_the_repository` が持つ。
#[test]
fn the_only_url_row_is_pasta_in_windows() {
    assert_eq!(
        fetched_rows(),
        [(
            "pasta-in-windows",
            "https://ekicyou.github.io/ghost_dev/pasta-in-windows/pasta-in-windows.nar"
        )],
        "URL の行はぱすただけ"
    );
}

/// 登記表の全部の URL の行が欄の形の判定を通る（spec `areka-P0-sample-url-fetch` 要件 1.5・5.2）。
/// 母数 0 で緑にしない較正: URL の行が 1 つ以上在る。
#[test]
fn every_fetched_row_has_a_well_formed_url_and_name() {
    let rows = fetched_rows();
    assert!(!rows.is_empty(), "較正: URL の行が 1 つ以上在る");
    for (name, url) in rows {
        fetched::fetched_row_form(name, url).unwrap_or_else(|err| panic!("{err}"));
    }
}

/// URL の行の書庫はリポジトリに置かず、書庫をリポジトリに置く行（Vendored）の書庫は置いてある（spec `areka-P0-sample-url-fetch` 要件 1.3・1.4・5.3）。
#[test]
fn only_vendored_rows_have_a_nar_in_the_repository() {
    let rows = fetched_rows();
    assert!(!rows.is_empty(), "較正: URL の行が 1 つ以上在る");
    for (name, _) in rows {
        let nar = devroot::nar_dir().join(format!("{name}.nar"));
        assert!(
            !nar.exists(),
            "URL の行の書庫がリポジトリに在る: {}",
            nar.display()
        );
    }
    for sample in SAMPLES
        .iter()
        .filter(|s| s.source == SampleSource::Vendored)
    {
        let nar = devroot::nar_dir().join(format!("{}.nar", sample.name));
        assert!(
            nar.is_file(),
            "Vendored の行の書庫が無い: {}",
            nar.display()
        );
    }
}

/// 登記した検体は、登記の行から導いた位置に実在し、値を捨てると複製ごと消える
/// （要件 1.1・1.2・1.3・7.4。URL の行は spec `areka-P0-sample-url-fetch` 要件 5.4・5.6）。
///
/// 位置は `<根>/<格納先>/<名>` の等式で判定する。検体ごとの第 2 の表や専用関数があれば
/// この等式は破れる（要件 1.5）。置き場に URL の行の書庫が無ければ、その失敗の文
/// （名前・探した場所・取り寄せの段のコマンド）で落ちる。
#[test]
fn every_registered_sample_lands_where_its_registry_row_says() {
    assert!(!SAMPLES.is_empty(), "較正: 登記が空でないこと");
    for sample in SAMPLES {
        let acquired = SampleRoot::acquire(sample.name).unwrap_or_else(|err| panic!("{err}"));

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
                acquired.folder().join("balloon").join(balloon),
                "同梱バルーン {balloon} は `<根>/ghost/<名>/balloon/<名>`（要件 1.3・spec: `areka-P0-ghost-inner-balloon` 要件 7.1）"
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

/// 未登録の検体名は、既知の名前をすべて含む理由付きの失敗になる（要件 1.4）。
#[test]
fn unknown_sample_fails_with_every_known_name() {
    let names: Vec<&str> = SAMPLES.iter().map(|sample| sample.name).collect();
    let err = SampleRoot::acquire("no-such-ghost").expect_err("未登録名は失敗するはず");
    let SampleError::UnknownSample { requested, known } = &err else {
        panic!("未登録名の失敗は UnknownSample であるはず: {err:?}");
    };
    assert_eq!(requested, "no-such-ghost");
    assert_eq!(known, &names);
    let shown = err.to_string();
    for name in &names {
        assert!(shown.contains(name), "失敗の表示に {name} が無い: {shown}");
    }
}

/// 同時にインストールしていないバルーン名は、既知の一覧を含む失敗になる（要件 1.4）。
///
/// 同梱バルーンを持たない検体では既知の一覧が**空**になることも明示して確かめる
/// （引き算で導ける 0 は沈黙と同じなので書く）。主張は検体に依らないので、複製の小さい
/// 検体（`dot_sakura`・同梱 0 の側は `emo2-kakukaku-wplimit`）を使う。
#[test]
fn unknown_balloon_fails_with_the_known_balloon_list() {
    let dot_sakura = SampleRoot::acquire("dot_sakura").expect("dot_sakura は登記済み");
    let err = dot_sakura
        .balloon("bottle-nope")
        .expect_err("同梱していない名前は失敗するはず");
    let SampleError::UnknownBalloon {
        sample,
        requested,
        known,
    } = &err
    else {
        panic!("同梱外の失敗は UnknownBalloon であるはず: {err:?}");
    };
    assert_eq!(*sample, "dot_sakura");
    assert_eq!(requested, "bottle-nope");
    assert_eq!(known, &["bottle"]);
    assert!(
        err.to_string().contains("bottle"),
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
/// 食い違いは本体のフォルダ名・種別・同梱の欠け・同梱の余り。対照として
/// 登記どおりの `.nar` が通ることも見る（照合が「常に赤」ではないことの較正）。
#[test]
fn a_tree_that_disagrees_with_the_registry_row_is_refused() {
    let home = private_dir();

    // 対照——登記どおりに展開されるなら通り、位置も登記の行から導かれる。
    let good = probe_nar("ghost", GHOST_WITH_BALLOON.name, Some(PROBE_BALLOON));
    let copy =
        devroot::fresh_root_in(home.path(), "probe-agrees", || Ok(good.clone())).expect("複製");
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
        "probe-other-folder",
        "ghost",
        "probe-other",
        Some(PROBE_BALLOON),
        &GHOST_WITH_BALLOON,
    );
    assert_eq!(
        installed,
        [
            "ghost/probe-other",
            "ghost/probe-other/balloon/probe-window-balloon"
        ]
    );
    assert_eq!(
        expected,
        "ghost/probe-window, ghost/probe-window/balloon/probe-window-balloon"
    );
    assert!(
        shown.contains("probe-other") && shown.contains("probe-window"),
        "失敗の表示に両側の綴りが無い: {shown}"
    );

    // ⑵ 種別が違う（バルーンとして置かれた）。
    let (_, expected, installed) = refused(
        home.path(),
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
        "probe-missing-companion",
        "ghost",
        GHOST_WITH_BALLOON.name,
        None,
        &GHOST_WITH_BALLOON,
    );
    assert_eq!(installed, ["ghost/probe-window"]);
    assert_eq!(
        expected,
        "ghost/probe-window, ghost/probe-window/balloon/probe-window-balloon"
    );

    // ⑷ 登記していない同梱バルーンが置かれた。
    let (_, expected, installed) = refused(
        home.path(),
        "probe-extra-companion",
        "ghost",
        GHOST_ALONE.name,
        Some(PROBE_BALLOON),
        &GHOST_ALONE,
    );
    assert_eq!(
        installed,
        [
            "ghost/probe-window",
            "ghost/probe-window/balloon/probe-window-balloon"
        ]
    );
    assert_eq!(expected, "ghost/probe-window");
}

// ---- 登記の往復と新品（要件 7.4・8.4・9.5） ----

/// 起動記録の置き場。ゴーストのフォルダの中で、前の走行が残す物が置かれる所。
const BOOT_RECORD: [&str; 5] = ["ghost", "master", "profile", "areka", "x.toml"];

/// 新品の検査に使うゴーストの検体。登記されたゴーストのうち `.nar` が小さい方。
const FRESH_SAMPLE: &str = "R_POST_and_KOMAINU";

/// 登記の 1 行が主張する要素を、展開結果と同じ `<種別> <格納先>/<名前>` の形で並べる。
///
/// 種別は展開器が返す [`ElementKind`](areka_nar::ElementKind) の表示と同じ綴りにする。
/// 本体が 1 つ、同梱バルーンが 0 個以上。
fn declared_elements_with_kind(sample: &Sample) -> Vec<String> {
    let kind = match sample.kind {
        SampleKind::Ghost => "Ghost",
        SampleKind::Balloon => "Balloon",
    };
    let mut declared = vec![format!(
        "{kind} {}/{}",
        match sample.kind {
            SampleKind::Ghost => "ghost",
            SampleKind::Balloon => "balloon",
        },
        sample.name
    )];
    declared.extend(
        sample
            .balloons
            .iter()
            .map(|balloon| format!("Balloon ghost/{}/balloon/{balloon}", sample.name)),
    );
    declared.sort();
    declared
}

/// 登記の往復——登記した各 `.nar` を空の根へ展開すると、置かれた要素が登記の 1 行
/// （種別・フォルダ名・同梱バルーン）とちょうど一致する（要件 8.4 の恒常版・9.5）。
///
/// 見るのは**展開器が返した要素**であって、根を走査して拾った名前ではない。種別は
/// `install.txt` の `type` から展開器が決めた物なので、`.nar` が登記と違う種別を
/// 名乗っていれば（置き場が偶然一致していても）ここで赤になる。
///
/// 段 ① の 8.4 は畳む前の追跡ツリーとの突き合わせを 1 度だけ行う手順だった。ツリーを
/// 消した後は突き合わせる相手が登記になるので、この形が常設の姿である。`.nar` を
/// 中身の違う物に差し替えれば、`SAMPLES` の行と食い違った瞬間に赤になる。
#[test]
fn every_sample_nar_installs_exactly_the_elements_its_registry_row_declares() {
    assert!(
        !SAMPLES.is_empty(),
        "較正: 登記が空でないこと（母数 0 で緑にならない）"
    );
    for sample in SAMPLES {
        let root = WorkDir::new().expect("空の根は取れるはず");
        // URL の行は置き場の照合を通った書庫。落ちたら失敗の文（取り寄せの段のコマンド）で止まる。
        let (nar, _) = devroot::sample_nar(sample.name).unwrap_or_else(|err| panic!("{err}"));

        let archive = areka_nar::NarArchive::open(&nar)
            .unwrap_or_else(|err| panic!("検体 {} の .nar は受理されるはず: {err}", sample.name));
        let outcome = archive
            .install(&areka_nar::InstallRequest {
                roots: &areka_nar::SingleRoot(root.path()),
                target_ghost: None,
            })
            .unwrap_or_else(|err| panic!("検体 {} は空の根へ展開できるはず: {err}", sample.name));

        let mut installed: Vec<String> = outcome
            .installed
            .iter()
            .map(|element| {
                let place = element
                    .path
                    .strip_prefix(root.path())
                    .unwrap_or_else(|_| {
                        panic!(
                            "検体 {} の要素が根の外に置かれた: {}",
                            sample.name,
                            element.path.display()
                        )
                    })
                    .to_string_lossy()
                    .replace('\\', "/");
                assert!(
                    element.path.is_dir(),
                    "検体 {} の要素のフォルダが実在しない: {}",
                    sample.name,
                    element.path.display()
                );
                format!("{:?} {place}", element.kind)
            })
            .collect();
        installed.sort();

        assert_eq!(
            installed,
            declared_elements_with_kind(sample),
            "検体 {} の展開結果が登記の 1 行と食い違う",
            sample.name
        );
        assert_eq!(
            outcome.warnings.len(),
            0,
            "検体 {} の警告は 0 件",
            sample.name
        );
        assert_eq!(
            outcome.leftovers.len(),
            0,
            "検体 {} の残った作業フォルダは 0 件",
            sample.name
        );
        // `emily4.nar` は `balloon.source.directory,balloon` を名乗るのに書庫に `balloon/` が
        // 無い。入るのはゴーストだけで、飛ばした同梱が 1 件だけ載る（spec:
        // `areka-P0-emily-ghost-verification` 要件 1.10・7.3）。ほかの検体は 0 件。
        let skipped_expected = if sample.name == "emily4" {
            vec![areka_nar::SkippedCompanion {
                directory: "emily4".to_owned(),
                key: "balloon.source.directory".to_owned(),
                source: "balloon".to_owned(),
            }]
        } else {
            Vec::new()
        };
        assert_eq!(
            outcome.skipped, skipped_expected,
            "検体 {} の飛ばした同梱",
            sample.name
        );
        // `dot_sakura.nar` は取り出し元が宛先と同じ名前のフォルダで書庫の最上位に在る。最上位の
        // `delete.txt`・`updates.txt` は本体の直下に普通のファイルとして置かれ、取り出し元の
        // フォルダは残らず、同梱はゴーストの中の balloon へ入る（spec: areka-P0-dot-sakura-specimen
        // 要件 2.4・5.7、areka-P0-ghost-inner-balloon 要件 7.1）。
        if sample.name == "dot_sakura" {
            let balloon = sample.balloons[0];
            let body = root.path().join("ghost").join("dot_sakura");
            for top_level_file in ["delete.txt", "updates.txt"] {
                assert!(
                    body.join(top_level_file).is_file(),
                    "最上位の {top_level_file} は本体の直下に普通のファイルとして在るはず"
                );
            }
            assert!(
                !body.join(balloon).exists(),
                "取り出し元のフォルダは本体の直下に残らないはず"
            );
            assert!(
                body.join("balloon")
                    .join(balloon)
                    .join("descript.txt")
                    .is_file(),
                "同梱の descript.txt がゴーストの中に在るはず"
            );
            assert!(
                !root.path().join("balloon").exists(),
                "根の balloon には何も入らないはず"
            );
        }
    }
}

/// 窓口から取得した根へ起動記録を書いて捨てても、次に取得した根には現れない
/// （要件 7.4・9.5）。
///
/// 「2 回取得できる」だけでは何も証明しない（同じ木を配り続ける実装でも緑になる）。
/// そこで 1 回目に書いた起動記録の実在を先に主張し、2 回目の根が**完全**である
/// ことも主張してから「無い」を言う（空の木を配る実装で恒真にならない）。
#[test]
fn a_boot_record_written_into_an_acquired_root_never_appears_in_the_next_one() {
    let record_of = |acquired: &SampleRoot| {
        BOOT_RECORD
            .iter()
            .fold(acquired.folder().to_path_buf(), |path, part| {
                path.join(part)
            })
    };

    let first = SampleRoot::acquire(FRESH_SAMPLE).expect("登記済みの名前は取得できるはず");
    let record = record_of(&first);
    std::fs::create_dir_all(record.parent().expect("起動記録には親が在る"))
        .expect("起動記録の置き場を作れるはず");
    std::fs::write(&record, b"boot,1\r\n").expect("起動記録を書けるはず");
    assert!(
        record.is_file(),
        "較正: 1 回目の根に起動記録が実在すること: {}",
        record.display()
    );
    let first_root = first.root().to_path_buf();
    drop(first);
    assert!(
        !first_root.exists(),
        "破棄で 1 回目の複製が消えること: {}",
        first_root.display()
    );

    let second = SampleRoot::acquire(FRESH_SAMPLE).expect("2 回目も取得できるはず");
    assert!(
        second
            .folder()
            .join("ghost")
            .join("master")
            .join("descript.txt")
            .is_file(),
        "較正: 2 回目の根が完全であること（空の木では「起動記録が無い」が恒真になる）"
    );
    let again = record_of(&second);
    assert!(
        !again.exists(),
        "2 回目の根に前の走行の起動記録が現れないこと（要件 7.4）: {}",
        again.display()
    );
    assert!(
        !again
            .parent()
            .and_then(std::path::Path::parent)
            .expect("`profile/` には親が在る")
            .exists(),
        "前の走行が作った `profile/` ごと現れないこと（要件 7.4）"
    );
    assert_ne!(
        second.root(),
        first_root,
        "2 回目は別の複製であること（同じ木を配り直していない）"
    );
}

/// 2 つ目のシェルを写すと、写した先の `name` だけが変わり、元のシェルはバイト単位で
/// そのまま残る（spec: `areka-P0-shell-balloon-switch` 要件 9.1・9.3）。
///
/// 検体は `dot_sakura`（`charset,Shift_JIS`・改行 CRLF・`name,master` を持つ）。
/// 名前以外の行がバイト単位で残ることも見るので、文字コードを読み直す実装は赤になる。
/// （`R_POST_and_KOMAINU` は 2026-10-04 の版から UTF-8 になり、この較正に使えなくなった）
#[test]
fn a_copied_shell_gets_the_new_name_and_the_original_stays_byte_for_byte() {
    let sakura = SampleRoot::acquire("dot_sakura").expect("登記済みの検体");
    let shell = sakura.folder().join("shell");
    let original = std::fs::read(shell.join("master").join("descript.txt"))
        .expect("元のシェルの descript.txt を読めるはず");
    assert!(
        original.windows(8).any(|w| w == b"Shift_JI"),
        "較正: 元のシェルが Shift_JIS を宣言していること"
    );

    sakura
        .add_shell_copy("master", "second", "second shell")
        .expect("2 つ目のシェルを写せるはず");

    let copied = std::fs::read(shell.join("second").join("descript.txt"))
        .expect("写した先の descript.txt を読めるはず");
    assert_eq!(
        std::fs::read(shell.join("master").join("descript.txt")).expect("読めるはず"),
        original,
        "元のシェルの descript.txt はバイト単位で不変であること"
    );
    let lines = |bytes: &[u8]| -> Vec<Vec<u8>> {
        bytes.split(|b| *b == b'\n').map(<[u8]>::to_vec).collect()
    };
    let (before, after) = (lines(&original), lines(&copied));
    assert_eq!(before.len(), after.len(), "行の数は変わらないこと");
    for (old, new) in before.iter().zip(&after) {
        if old.starts_with(b"name,") {
            assert_eq!(
                new.as_slice(),
                b"name,second shell\r",
                "name の行だけが替わること"
            );
        } else {
            assert_eq!(old, new, "name 以外の行はバイト単位で残ること");
        }
    }
    assert!(
        shell.join("second").join("surface0.png").is_file(),
        "シェルの中身が再帰で写ること"
    );
    assert!(
        sakura.add_shell_copy("master", "second", "x").is_err(),
        "既に在る先へは写さない（黙って上書きしない）"
    );
}

/// `descript.txt` の `name` の行は置き換え、無ければ足す（改行の形は元に合わせる・spec: `areka-P0-shell-balloon-switch` 要件 9.1）。
#[test]
fn named_descript_replaces_the_name_line_or_appends_one() {
    assert_eq!(
        named_descript(b"charset,UTF-8\r\ntype,shell\r\n", "x"),
        b"charset,UTF-8\r\ntype,shell\r\nname,x\r\n"
    );
    assert_eq!(
        named_descript(b"charset,UTF-8\ntype,shell", "x"),
        b"charset,UTF-8\ntype,shell\nname,x\n"
    );
    assert_eq!(
        named_descript(b"charset,UTF-8\nname,old\n", "x"),
        b"charset,UTF-8\nname,x\n"
    );
    // 1 行目の頭の BOM は外して照らし、置き換えても残す（かけ直し 4 回目 S7）。
    assert_eq!(
        named_descript(b"\xEF\xBB\xBFname,old\r\ntype,shell\r\n", "x"),
        b"\xEF\xBB\xBFname,x\r\ntype,shell\r\n"
    );
}

/// 元のシェルが無ければ、写す先のフォルダを作らずに `Io`（`NotFound`）を返す（audit-test-kits 指摘 16）。
#[test]
fn copying_a_missing_shell_leaves_no_empty_folder() {
    let sakura = SampleRoot::acquire("dot_sakura").expect("登記済みの検体");
    let to = sakura.folder().join("shell").join("second");

    let refused = sakura.add_shell_copy("no-such-shell", "second", "x");

    match refused {
        Err(SampleError::Io { source, .. }) => {
            assert_eq!(source.kind(), std::io::ErrorKind::NotFound, "{source}");
        }
        other => panic!("無い元のシェルは Io（NotFound）で断ること: {other:?}"),
    }
    assert!(!to.exists(), "空の写す先を残さないこと: {}", to.display());
}

/// `Io` の表示は、パスと元の失敗（日本語の OS の文）を ASCII に写す。`os error N` は残る
/// （audit-test-kits 指摘 32）。
#[test]
fn the_io_error_is_shown_in_ascii_only() {
    let source = std::io::Error::new(
        std::io::Error::from_raw_os_error(5).kind(),
        format!(
            "アクセスが拒否されました。 {}",
            std::io::Error::from_raw_os_error(5)
        ),
    );
    let refused = SampleError::Io {
        what: "create the work folder",
        path: Path::new("C:").join("作業").join("x"),
        source,
    };

    let text = refused.to_string();

    assert!(text.is_ascii(), "{text}");
    assert!(text.contains("os error 5"), "{text}");
    assert!(text.contains("create the work folder"), "{text}");
    assert!(text.contains("\\u{"), "日本語は逃がして残る: {text}");
}

/// `Io` 以外の表示も、名前・パス・展開器の日本語の文を ASCII に写す（`nar-sample-path` が
/// 標準エラーへ出す文・audit-test-kits のかけ直し 1 回目 S1）。
#[test]
fn the_other_errors_are_shown_in_ascii_only() {
    let shown = [
        SampleError::Nar(areka_nar::NarError::Refused {
            archive: Path::new("C:").join("検体.nar"),
            reason: areka_nar::RefuseReason::CorruptArchive {
                detail: "壊れた".to_owned(),
            },
        }),
        SampleError::UnknownSample {
            requested: "知らない".to_owned(),
            known: vec!["emo2"],
        },
        SampleError::UnknownBalloon {
            sample: "emo2",
            requested: "知らない".to_owned(),
            known: &["既知"],
        },
        SampleError::RegistryMismatch {
            sample: "emo2",
            expected: "ghost/名前".to_owned(),
            installed: vec!["ghost/別".to_owned()],
        },
        SampleError::TargetDirNotFound {
            started_from: Path::new("C:").join("作業").join("x.exe"),
        },
    ]
    .map(|refused| refused.to_string());

    for text in shown {
        assert!(text.is_ascii(), "{text}");
        // 日本語を落とさず写したこと（`is_ascii` だけでは捨てても緑になる）。
        assert!(text.contains("\\u{"), "{text}");
    }
}
