//! 入れ子の同梱（ゴーストの中のバルーン）の組み上げと確定の兄弟テスト
//! （本仕様 要件 7.7・8.1・10.3）。
//!
//! 助手と固定入力は [`super`]（`install_commit_tests.rs`）から借りる。分けたのは
//! 1 ファイル 1,000 行の上限に収めるため。

use super::*;

// ---- 入れ子の同梱（ゴーストの中のバルーン＝要件 7.7・8.1・10.3） ----

/// 同梱を `<ゴースト>\balloon\test-balloon` へ入れ子に置く形へ組み替え、その宛先を返す。
///
/// 計画も同じ形に組むが、ここでは外す綴りの大小を変えて上書きし、比べ方が ASCII の
/// 大小を無視することも同時に踏む。
fn nest(prepared: &mut Prepared, ghost: &Path) -> PathBuf {
    let inner = ghost.join("balloon").join("test-balloon");
    prepared.plan[0].carve_out = vec![r"Balloon\Test-Balloon".to_owned()];
    prepared.plan[1].destination = inner.clone();
    prepared.plan[1].within_body = true;
    inner
}

/// 古いゴースト（記憶と古い同梱）を根に置く。
fn plant_old_ghost(ghost: &Path) {
    make_tree(
        ghost,
        &[
            ("profile/ghost.dat", b"ghost state"),
            ("balloon/test-balloon/descript.txt", b"old balloon"),
            ("balloon/test-balloon/old.png", b"old"),
        ],
    );
}

/// 新しい同梱の木（書庫のバイト列そのもの）。`old` を足すと古いファイルが重なった形。
fn companion_tree(old: bool) -> BTreeMap<String, Vec<u8>> {
    let mut entries: Vec<(&str, &[u8])> = vec![
        ("balloon/", b""),
        ("balloon/s0.png", b"s0"),
        ("descript.txt", b"balloon descript"),
        ("install.txt", b"type,balloon"),
    ];
    if old {
        entries.push(("old.png", b"old"));
    }
    expect_tree(&entries)
}

/// 本体 `refresh,1`＋古い同梱在りでも、同梱は巻き戻らずに入る（要件 7.7・8.1）。
///
/// 本体の確定で古い同梱は退避ごと宛先から消え、同梱の宛先は空く。組み上げのときの
/// 別（`Overlaid`）のまま退避の 2 手を踏むと、無い宛先の退避で失敗して全体が戻る。
/// 除外マスク（`descript.txt`）が古い同梱の同名に当たっても、外す場所の下は写さない。
#[test]
fn a_nested_companion_survives_a_refreshing_body() {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    let ghost = work.path().join("ghost").join("test-ghost");
    plant_old_ghost(&ghost);
    let request = InstallRequest {
        roots: &SingleRoot(work.path()),
        target_ghost: None,
    };
    let mut prepared = prepare(
        ghost_archive(&["refresh,1", "refreshundeletemask,descript.txt"]),
        &request,
    );
    let inner = nest(&mut prepared, &ghost);

    let area = WorkArea::create(work.path()).expect("作業フォルダを作れる");
    let states = stage_into(&area, &prepared);
    assert!(
        !area.stage(0).join("balloon").join("test-balloon").exists(),
        "本体の下敷きに古い同梱を写さない"
    );
    let outcome = commit_all(
        std::slice::from_ref(&area),
        &prepared.manifest,
        &prepared.plan,
        Vec::new(),
        &states,
    )
    .expect("確定できる");

    assert_eq!(
        outcome
            .installed
            .iter()
            .map(|element| element.existing)
            .collect::<Vec<_>>(),
        vec![ExistingState::Refreshed, ExistingState::Overlaid],
        "組み上げのときの別をそのまま報告する"
    );
    assert_eq!(outcome.installed[1].path, inner, "同梱はゴーストの中");
    assert_eq!(
        tree(&inner),
        companion_tree(true),
        "同梱に balloon.refresh が無いので古いファイルが残る"
    );
    assert!(
        !ghost.join("profile").exists(),
        "本体は refresh,1 で作り直される"
    );
}

/// 本体 `Overlay`＋同梱 `balloon.refresh,1` なら、同梱だけ作り直される（要件 8.1）。
#[test]
fn a_refreshing_companion_inside_an_overlaid_body_is_rebuilt_alone() {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    let ghost = work.path().join("ghost").join("test-ghost");
    plant_old_ghost(&ghost);
    let request = InstallRequest {
        roots: &SingleRoot(work.path()),
        target_ghost: None,
    };
    let mut prepared = prepare(ghost_archive(&["balloon.refresh,1"]), &request);
    let inner = nest(&mut prepared, &ghost);

    let (_, outcome) = install(work.path(), &prepared);
    let outcome = outcome.expect("確定できる");

    assert_eq!(outcome.installed[0].existing, ExistingState::Overlaid);
    assert_eq!(outcome.installed[1].existing, ExistingState::Refreshed);
    assert_eq!(tree(&inner), companion_tree(false), "同梱は新しいものだけ");
    assert_eq!(
        fs::read(ghost.join("profile").join("ghost.dat")).expect("記憶が残る"),
        b"ghost state",
        "本体は重ね置き"
    );
}

/// 同梱に `balloon.refresh` が無ければ、古いファイルが残り新しいものが重なる（要件 8.1）。
#[test]
fn a_companion_without_its_own_refresh_overlays_the_old_files() {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    let ghost = work.path().join("ghost").join("test-ghost");
    plant_old_ghost(&ghost);
    let request = InstallRequest {
        roots: &SingleRoot(work.path()),
        target_ghost: None,
    };
    let mut prepared = prepare(ghost_archive(&[]), &request);
    let inner = nest(&mut prepared, &ghost);

    let (_, outcome) = install(work.path(), &prepared);
    let outcome = outcome.expect("確定できる");

    assert_eq!(outcome.installed[1].existing, ExistingState::Overlaid);
    assert_eq!(
        tree(&inner),
        companion_tree(true),
        "古いファイルに新しいものが重なる"
    );
    assert!(
        ghost.join("profile").join("ghost.dat").is_file(),
        "本体は重ね置き"
    );
}

/// 初めてのゴーストでも、同梱はゴーストの中に入る（要件 7.7）。
#[test]
fn a_first_time_ghost_gets_its_nested_companion() {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    let ghost = work.path().join("ghost").join("test-ghost");
    let request = InstallRequest {
        roots: &SingleRoot(work.path()),
        target_ghost: None,
    };
    let mut prepared = prepare(ghost_archive(&[]), &request);
    let inner = nest(&mut prepared, &ghost);

    let (_, outcome) = install(work.path(), &prepared);
    let outcome = outcome.expect("確定できる");

    assert_eq!(
        outcome.installed,
        vec![
            element(
                ElementKind::Ghost,
                "ためしゴースト",
                ghost.clone(),
                None,
                ExistingState::New
            ),
            element(
                ElementKind::Balloon,
                "test-balloon",
                inner.clone(),
                None,
                ExistingState::New
            ),
        ]
    );
    assert_eq!(tree(&inner), companion_tree(false));
    assert!(
        !work.path().join("balloon").exists(),
        "根の置き場へは何も入らない"
    );
}

/// 同梱の確定が失敗したら全体が戻り、古い同梱は元のまま（要件 7.7・10.3）。
///
/// 組み上げた同梱の木を掴んでおき、同梱の `rename` だけを失敗させる。戻しは今の
/// 逆順（同梱 → 本体）で、本体の退避ごと古い同梱がバイト列まで元へ戻る。
#[test]
fn a_failing_nested_companion_rolls_the_whole_ghost_back() {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    let ghost = work.path().join("ghost").join("test-ghost");
    plant_old_ghost(&ghost);
    let before = tree(&ghost);
    let request = InstallRequest {
        roots: &SingleRoot(work.path()),
        target_ghost: None,
    };
    let mut prepared = prepare(ghost_archive(&[]), &request);
    let inner = nest(&mut prepared, &ghost);

    let area = WorkArea::create(work.path()).expect("作業フォルダを作れる");
    let states = stage_into(&area, &prepared);
    assert!(
        !area.stage(0).join("balloon").join("test-balloon").exists(),
        "重ね置きの下敷きにも古い同梱を写さない"
    );
    let handle = hold(&area.stage(1).join("descript.txt"));
    let failure = commit_all(
        std::slice::from_ref(&area),
        &prepared.manifest,
        &prepared.plan,
        Vec::new(),
        &states,
    )
    .expect_err("同梱の確定が失敗する");

    assert_eq!(failure.phase, IoPhase::Commit, "確定の段で失敗した");
    assert_eq!(failure.path, inner, "失敗したのは同梱の宛先");
    assert!(failure.rolled_back, "本体を元へ戻せた");
    assert_eq!(
        failure.committed,
        vec![element(
            ElementKind::Ghost,
            "ためしゴースト",
            ghost.clone(),
            None,
            ExistingState::Overlaid
        )],
        "確定まで進んだのは本体だけ"
    );
    assert_eq!(tree(&ghost), before, "古い同梱ごと呼ぶ前のバイト列に戻る");
    assert!(
        !work.path().join("balloon").exists(),
        "根の置き場へ入れ直さない"
    );
    drop(handle);
}

/// 置いた後の同梱より後で失敗したら、置いた同梱を消してから本体を戻す（要件 7.7・10.3）。
///
/// 同梱を 2 つ入れ子にし、2 つ目の `rename` だけを失敗させる。1 つ目の同梱は本体の
/// 宛先の下に置かれているので、戻しが本体を先に戻すと、その後の同梱の消しが戻した
/// 古い同梱を消してしまう。逆順なら古い同梱がバイト列まで元へ戻る。
#[test]
fn a_later_failure_removes_the_nested_companion_before_restoring_the_body() {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    let ghost = work.path().join("ghost").join("test-ghost");
    plant_old_ghost(&ghost);
    let before = tree(&ghost);
    let request = InstallRequest {
        roots: &SingleRoot(work.path()),
        target_ghost: None,
    };
    let mut prepared = prepare(ghost_archive(&[]), &request);
    nest(&mut prepared, &ghost);
    let second = ghost.join("balloon").join("second");
    let first = &prepared.plan[1];
    let third = Placement {
        kind: first.kind.clone(),
        name: first.name.clone(),
        root: first.root.clone(),
        destination: second.clone(),
        target_ghost: first.target_ghost.clone(),
        existing: first.existing.clone(),
        skip_top_level_install_txt: first.skip_top_level_install_txt,
        files: first.files.clone(),
        dirs: first.dirs.clone(),
        carve_out: Vec::new(),
        within_body: true,
    };
    prepared.plan.push(third);
    prepared.plan[0]
        .carve_out
        .push(r"balloon\second".to_owned());

    let area = WorkArea::create(work.path()).expect("作業フォルダを作れる");
    let states = stage_into(&area, &prepared);
    let handle = hold(&area.stage(2).join("descript.txt"));
    let failure = commit_all(
        std::slice::from_ref(&area),
        &prepared.manifest,
        &prepared.plan,
        Vec::new(),
        &states,
    )
    .expect_err("2 つ目の同梱の確定が失敗する");

    assert_eq!(failure.phase, IoPhase::Commit, "確定の段で失敗した");
    assert_eq!(failure.path, second, "失敗したのは 2 つ目の同梱の宛先");
    assert!(failure.rolled_back, "置いた同梱を消してから本体を戻せた");
    assert_eq!(
        failure.committed.len(),
        2,
        "本体と 1 つ目の同梱まで確定した"
    );
    assert_eq!(tree(&ghost), before, "古い同梱ごと呼ぶ前のバイト列に戻る");
    assert!(
        !work.path().join("balloon").exists(),
        "根の置き場へ入れ直さない"
    );
    drop(handle);
}
