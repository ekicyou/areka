use std::collections::BTreeMap;

use areka_parsers::shell::{
    AppendTarget, DefRef, Element, ElementPath, Shell, Surface, SurfaceAppend,
};
use bevy_ecs::world::World;

use crate::base_image::BaseImageReport;
use crate::fold::fold_shell;
use crate::method::ComposeMethod;
use crate::normalized::SurfaceMaster;
use crate::world::{AliasMap, EmoWorld, ShellSettings, SurfaceIndex};

/// element 1 本（layer/x/y 指定）。
fn elem(layer: u32, path: &str, x: i64, y: i64) -> Element {
    Element {
        layer,
        path: ElementPath::new(path.to_string()),
        x,
        y,
    }
}

/// ターゲット記述子とボディ element 群から plain surface 定義を組み立てる。
fn surface_def(id: u32, targets: Vec<AppendTarget>, elements: Vec<Element>) -> Surface {
    Surface {
        id,
        targets,
        elements,
        collisions: Vec::new(),
        animations: Vec::new(),
    }
}

/// surfaces と definitions を 1 対 1（登場順）で組んだ `Shell`。
fn shell_of(surfaces: Vec<Surface>) -> Shell {
    let definitions = (0..surfaces.len()).map(DefRef::Surface).collect();
    Shell {
        surfaces,
        appends: Vec::new(),
        aliases: Vec::new(),
        animation_sort: None,
        collision_sort: None,
        definitions,
    }
}

/// 番号 → ファイル名の対応を組み立てる。
fn images(entries: &[(u32, &str)]) -> BTreeMap<u32, String> {
    entries
        .iter()
        .map(|(id, file)| (*id, (*file).to_string()))
        .collect()
}

/// 画像を足す前の面の表（畳み込みだけを通した素の結果）を id 昇順で取り出す。
fn folded_masters(shell: &Shell) -> Vec<SurfaceMaster> {
    let mut world = World::new();
    world.insert_resource(SurfaceIndex::default());
    world.insert_resource(AliasMap::default());
    world.insert_resource(ShellSettings::default());
    fold_shell(&mut world, shell);

    let index = world.resource::<SurfaceIndex>();
    let mut ids: Vec<u32> = index.0.keys().copied().collect();
    ids.sort_unstable();
    let entities: Vec<_> = ids.iter().map(|id| index.0[id]).collect();
    entities
        .into_iter()
        .map(|e| {
            world
                .get::<SurfaceMaster>(e)
                .expect("SurfaceIndex が指す entity は SurfaceMaster を持つ")
                .clone()
        })
        .collect()
}

/// 面の表の全 master を id 昇順で取り出す。
fn masters_of(world: &EmoWorld) -> Vec<SurfaceMaster> {
    world
        .surface_ids()
        .map(|id| world.surface(id).expect("列挙された id は引ける").clone())
        .collect()
}

/// 複数番号の見出し（`surface0,1`）で展開された面は、番号ごとに**自分の**画像を層 0 に受け取る。
///
/// 判定が展開前の見出し単位だと、面 0 と面 1 が同じ 1 枚を共有するか片方が空になる（要件 2.1 のア・2.2）。
#[test]
fn shared_header_gives_each_id_its_own_image() {
    let shell = shell_of(vec![surface_def(
        0,
        vec![AppendTarget::Single(0), AppendTarget::Single(1)],
        Vec::new(),
    )]);
    let world =
        EmoWorld::build_with_images(&shell, &images(&[(0, "surface0.png"), (1, "surface1.png")]));

    for (id, file) in [(0u32, "surface0.png"), (1, "surface1.png")] {
        let master = world.surface(id).expect("展開された面が在る");
        assert_eq!(master.elements.len(), 1, "面 {id} の層は画像 1 枚だけ");
        let base = &master.elements[0];
        assert_eq!(base.layer, 0, "面 {id} の画像は層 0");
        assert_eq!(base.path.as_str(), file, "面 {id} は自分の画像を受け取る");
        assert_eq!(base.transform.offset(), (0, 0), "面 {id} の画像は (0,0)");
        assert_eq!(base.method, ComposeMethod::Overlay);
    }

    let report = world.base_images();
    assert_eq!(
        report.used,
        images(&[(0, "surface0.png"), (1, "surface1.png")])
    );
    assert!(report.shadowed.is_empty());
}

/// 画像 0 件で組んだ面の表は、畳み込みだけを通した結果と同じである（本仕様の適用前と同じ・要件 5.1〜5.3）。
#[test]
fn no_images_leaves_the_folded_table_untouched() {
    let shell = shell_of(vec![
        surface_def(
            0,
            vec![AppendTarget::Single(0)],
            vec![elem(0, "body.png", 3, 4), elem(1, "face.png", 5, 6)],
        ),
        surface_def(
            10,
            vec![AppendTarget::Single(10)],
            vec![elem(2, "hat.png", 0, 0)],
        ),
    ]);

    let world = EmoWorld::build_with_images(&shell, &BTreeMap::new());
    assert_eq!(masters_of(&world), folded_masters(&shell));
    assert_eq!(*world.base_images(), BaseImageReport::default());

    // 既存の呼び手（`build`）も同じ結果であること（呼び手の変更 0 件の裏取り）。
    assert_eq!(masters_of(&EmoWorld::build(&shell)), folded_masters(&shell));
}

/// `used` と `shadowed` はキーが重ならず、和は渡した画像の全体に等しい（design「Invariants」）。
///
/// 併せて表のウ（層 0 が在れば使わない）・イ（層の昇順に並べ直す）・エ（画像の無い番号に触らない）を留める。
#[test]
fn used_and_shadowed_partition_the_images() {
    let shell = shell_of(vec![
        // ウ: 層 0 が在るので画像は使わない。
        surface_def(
            0,
            vec![AppendTarget::Single(0)],
            vec![elem(0, "body.png", 0, 0)],
        ),
        // イ: 層 0 が空いているので画像が最も奥に入る。
        surface_def(
            1,
            vec![AppendTarget::Single(1)],
            vec![elem(1, "face.png", 7, 8)],
        ),
        // エ: 画像が無いので触らない。
        surface_def(
            2,
            vec![AppendTarget::Single(2)],
            vec![elem(3, "hat.png", 0, 0)],
        ),
    ]);
    let given = images(&[
        (0, "surface0.png"),
        (1, "surface1.png"),
        (5, "surface5.png"),
    ]);
    let world = EmoWorld::build_with_images(&shell, &given);

    let report = world.base_images();
    assert!(
        report
            .used
            .keys()
            .all(|id| !report.shadowed.contains_key(id)),
        "used と shadowed のキーは重ならない: used={:?} shadowed={:?}",
        report.used,
        report.shadowed,
    );
    let union: BTreeMap<u32, String> = report
        .used
        .iter()
        .chain(report.shadowed.iter())
        .map(|(id, file)| (*id, file.clone()))
        .collect();
    assert_eq!(union, given, "used と shadowed の和は渡した画像の全体");
    assert_eq!(
        report.shadowed,
        images(&[(0, "surface0.png")]),
        "ウ: 面 0 は使わない"
    );
    assert_eq!(
        report.used,
        images(&[(1, "surface1.png"), (5, "surface5.png")]),
        "イ: 面 1 と ア: 面 5 は使う",
    );

    // ウ: 面 0 の層は `element0` 1 本のまま（画像で二重にならない）。
    let face0 = world.surface(0).expect("面 0 が在る");
    assert_eq!(face0.elements.len(), 1);
    assert_eq!(face0.elements[0].path.as_str(), "body.png");

    // イ: 面 1 は画像が層 0（最も奥）・既存の `element1` がその上。
    let face1 = world.surface(1).expect("面 1 が在る");
    let layers: Vec<(u32, &str)> = face1
        .elements
        .iter()
        .map(|e| (e.layer, e.path.as_str()))
        .collect();
    assert_eq!(layers, vec![(0, "surface1.png"), (1, "face.png")]);

    // エ: 画像の無い面 2 は畳み込みのまま。
    let face2 = world.surface(2).expect("面 2 が在る");
    assert_eq!(face2.elements.len(), 1);
    assert_eq!(face2.elements[0].layer, 3);

    // ア: 宣言の無い面 5 は画像 1 枚を持つ「存在する面」になる（要件 2.2・3.9）。
    let face5 = world.surface(5).expect("画像だけの面 5 が在る");
    assert_eq!(face5.id, 5);
    assert_eq!(face5.elements.len(), 1);
    assert_eq!(face5.elements[0].path.as_str(), "surface5.png");
    assert!(face5.collisions.is_empty());
    assert!(face5.animations.is_empty());
}

/// surfaces と appends を、渡した定義順（`definitions`）どおりに並べた `Shell`。
fn shell_mixed(
    surfaces: Vec<Surface>,
    appends: Vec<SurfaceAppend>,
    definitions: Vec<DefRef>,
) -> Shell {
    Shell {
        surfaces,
        appends,
        aliases: Vec::new(),
        animation_sort: None,
        collision_sort: None,
        definitions,
    }
}

/// `surface.append` の対象群（ターゲット記述子と追記 element）。
fn append_def(targets: Vec<AppendTarget>, elements: Vec<Element>) -> SurfaceAppend {
    SurfaceAppend {
        targets,
        elements,
        collisions: Vec::new(),
        animations: Vec::new(),
    }
}

/// `surface.append` は、波括弧が 1 行も無い「画像だけで存在する面」にも効く（要件 3.7・C6）。
///
/// 追記で層 0 を足した面は画像を使わない（`shadowed`）。層 0 を足さなかった面は今までどおり
/// 画像が層 0 に入る（`used`）。どちらも新設ではないので、重複の `warn!` は 0 件である。
#[test]
fn append_reaches_image_only_faces() {
    let shell = shell_mixed(
        Vec::new(),
        vec![
            // 面 7: 追記が層 0 を足す（画像は使われない＝ウ）。
            append_def(
                vec![AppendTarget::Single(7)],
                vec![elem(0, "add0.png", 0, 0), elem(5, "deco.png", 1, 2)],
            ),
            // 面 8: 追記は層 5 だけ（層 0 は空いたままなので画像が入る＝ア）。
            append_def(
                vec![AppendTarget::Single(8)],
                vec![elem(5, "deco.png", 1, 2)],
            ),
        ],
        vec![DefRef::Append(0), DefRef::Append(1)],
    );
    let given = images(&[(7, "surface7.png"), (8, "surface8.png")]);

    let mut built = None;
    let logs = crate::log_capture::capture_logs(|| {
        built = Some(EmoWorld::build_with_images(&shell, &given));
    });
    let world = built.expect("構築された面の表");

    // 面 7: 画像だけの面へ追記が届き、その層 0 が画像を押しのける。
    let face7 = world.surface(7).expect("画像だけの面 7 へ追記が届く");
    let layers7: Vec<(u32, &str)> = face7
        .elements
        .iter()
        .map(|e| (e.layer, e.path.as_str()))
        .collect();
    assert_eq!(layers7, vec![(0, "add0.png"), (5, "deco.png")]);
    assert!(face7.collisions.is_empty());
    assert!(face7.animations.is_empty());

    // 面 8: 層 0 が空いているので画像が最も奥に入る。
    let face8 = world.surface(8).expect("画像だけの面 8 へ追記が届く");
    let layers8: Vec<(u32, &str)> = face8
        .elements
        .iter()
        .map(|e| (e.layer, e.path.as_str()))
        .collect();
    assert_eq!(layers8, vec![(0, "surface8.png"), (5, "deco.png")]);

    let report = world.base_images();
    assert_eq!(
        report.shadowed,
        images(&[(7, "surface7.png")]),
        "追記で足した層 0 も「在る」に数える",
    );
    assert_eq!(report.used, images(&[(8, "surface8.png")]));

    // 追記のために作った面は「新設」ではないので、重複の警告も未存在の警告も出ない。
    assert!(
        !logs.contains("重複"),
        "画像だけの面への追記で偽の重複警告を出さない: {logs}",
    );
    assert!(
        !logs.contains("未存在"),
        "画像だけの面は「在る」ので未存在の警告を出さない: {logs}",
    );
}

/// 宣言も画像も無い番号への `surface.append` は、今までどおり新設せず `warn!` を出す（要件 3.8・変更 0）。
#[test]
fn append_to_unknown_id_still_warns_and_creates_nothing() {
    let shell = shell_mixed(
        Vec::new(),
        vec![append_def(
            vec![AppendTarget::Single(9)],
            vec![elem(0, "add0.png", 0, 0)],
        )],
        vec![DefRef::Append(0)],
    );

    let mut built = None;
    let logs = crate::log_capture::capture_logs(|| {
        // 画像は面 7 の分だけ渡す（面 9 はどちらにも無い）。
        built = Some(EmoWorld::build_with_images(
            &shell,
            &images(&[(7, "surface7.png")]),
        ));
    });
    let world = built.expect("構築された面の表");

    assert!(world.surface(9).is_none(), "どちらにも無い面 9 はできない");
    assert!(world.surface(7).is_some(), "画像だけの面 7 はできる");
    assert!(logs.contains("level=WARN"), "未存在は WARN: {logs}");
    assert!(
        logs.contains("id=9"),
        "未存在 append 対象 id を載せる: {logs}"
    );
    assert!(logs.contains("未存在"), "未存在メッセージ: {logs}");
}

/// 画像だけの面へ追記した後で同じ番号の波括弧が来たら、既存の全置換の規則と `warn!` のまま。
///
/// 追記で実際に定義ができた後の置換なので、この警告は事実どおりである（design「面の表の構築の中の順序」）。
#[test]
fn brace_after_append_replaces_the_image_only_face() {
    let shell = shell_mixed(
        vec![surface_def(
            7,
            vec![AppendTarget::Single(7)],
            vec![elem(0, "brace0.png", 0, 0)],
        )],
        vec![append_def(
            vec![AppendTarget::Single(7)],
            vec![elem(5, "deco.png", 1, 2)],
        )],
        vec![DefRef::Append(0), DefRef::Surface(0)],
    );

    let mut built = None;
    let logs = crate::log_capture::capture_logs(|| {
        built = Some(EmoWorld::build_with_images(
            &shell,
            &images(&[(7, "surface7.png")]),
        ));
    });
    let world = built.expect("構築された面の表");

    // 波括弧が全置換するので、追記した層 5 は残らない。
    let face7 = world.surface(7).expect("面 7 が在る");
    let layers: Vec<(u32, &str)> = face7
        .elements
        .iter()
        .map(|e| (e.layer, e.path.as_str()))
        .collect();
    assert_eq!(layers, vec![(0, "brace0.png")], "波括弧が全置換する");
    assert_eq!(
        world.base_images().shadowed,
        images(&[(7, "surface7.png")]),
        "ウ: 波括弧の element0 が在るので画像は使わない",
    );

    assert!(logs.contains("level=WARN"), "重複は WARN: {logs}");
    assert!(logs.contains("重複"), "既存の重複メッセージのまま: {logs}");
}
