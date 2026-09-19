//! `install`（確定）の兄弟テスト——入れ替えによる確定・巻き戻し・結果の列
//! （要件 5.10・5.11・6.4・6.6）。
//!
//! 助手と固定入力は [`super`]（`install_tests.rs`）から借りる。分けたのは
//! 1 ファイル 1,000 行の上限に収めるためで、固定入力は 1 つも削っていない。
//!
//! 失敗の場合は 4 つあり、どれも**本物の失敗を起こして**踏む。
//!
//! ⑴ 宛先の中のファイルを、読みだけを共有して開いたまま確定する（起動中の
//!    `shiori.dll` の再現＝要件 6.6）。Windows は配下に掴まれたファイルが在るフォルダの
//!    `rename` を拒む（アクセス拒否）ので、1 手目の退避がその場で失敗する。
//! ⑵ 組み上げた木の中のファイルを同じように掴んでおく。1 手目の退避は通り、
//!    2 手目の「作業フォルダ → 宛先」が失敗するので、退避を戻す腕を踏む。
//! ⑶ 2 配置の 2 つ目の宛先を⑴の形で掴む。1 つ目は確定済みなので逆順に戻る。
//! ⑷ 作業フォルダの中に掴んだままのファイルを置く。確定は全て通り、後片付けだけが
//!    失敗して残り物になる。
//!
//! どの失敗でも、宛先が**呼ぶ前と 1 バイトも変わっていない**ことを木ごと突き合わせる
//! （[`tree`] は相対パスとバイト列そのもので写す）。「まだ在る」では何も証明できない。

use super::*;
use crate::error::ElementKind;
use std::os::windows::fs::OpenOptionsExt;

// ---- 助手 ----

/// 読むことだけを共有して開いたまま持つ（起動中のゴーストの `shiori.dll` と同じ状態）。
///
/// 掴んだ handle を返す。落とすまで、そのファイルも**親フォルダ**も
/// `rename`／`remove_dir_all` できない（どちらも os error 5）。
///
/// 設計の試験方針も `share_mode(1)`（`FILE_SHARE_READ`）と書いている。共有をゼロに
/// すると**組み上げ**の段の複写（既存の宛先を作業フォルダへ写す）が os error 32 で
/// 先に失敗し、確定の段まで到達しないためである。
/// 実際に読み込まれている DLL は読みを共有する（だから複写は通り、消せない）ので、
/// 共有を読みだけに絞るほうが再現として正しく、要件 6.6 の「確定ができない」に届く。
const SHARE_READ: u32 = 1;

fn hold(path: &Path) -> fs::File {
    fs::OpenOptions::new()
        .read(true)
        .share_mode(SHARE_READ)
        .open(path)
        .unwrap_or_else(|err| panic!("{} を掴めるはず: {err}", path.display()))
}

/// 全配置を組み上げる（宛先は読むだけ）。
fn stage_into(area: &WorkArea, prepared: &Prepared) -> Vec<ExistingState> {
    prepared
        .plan
        .iter()
        .enumerate()
        .map(|(k, placement)| {
            stage_placement(&area.stage(k), placement, &prepared.contents).expect("組み上げられる")
        })
        .collect()
}

/// 作業フォルダを作り、組み上げてから確定まで通す。
///
/// 仕掛けを挟む口として、作業フォルダのパスも一緒に返す。
fn install(root: &Path, prepared: &Prepared) -> (PathBuf, Result<InstallOutcome, CommitError>) {
    let area = WorkArea::create(root).expect("作業フォルダを作れる");
    let states = stage_into(&area, prepared);
    let work = area.path().to_path_buf();
    let outcome = commit_all(&area, &prepared.manifest, &prepared.plan, &states);
    (work, outcome)
}

/// 木の中の全ファイルの実パスが `root` の配下にあることを確かめる（完了条件）。
///
/// 数えたファイルが 0 件なら赤にする。空の木を歩いて「1 つも外に出ていない」と
/// 言っても何も判定していない。
fn every_file_resolves_under(root: &Path) {
    let real_root = fs::canonicalize(root).expect("宛先を実パスにできる");
    let mut counted = 0;
    for relative in tree(root).keys().filter(|key| !key.ends_with('/')) {
        let real = fs::canonicalize(root.join(relative)).expect("実パスにできる");
        assert!(
            real.starts_with(&real_root),
            "{} が宛先 {} の外へ出ている",
            real.display(),
            real_root.display()
        );
        counted += 1;
    }
    assert!(counted > 0, "数えたファイルが 0 件（母数 0 で緑にしない）");
}

/// 確定に成功したときの要素 1 つぶん。
fn element(
    kind: ElementKind,
    name: &str,
    path: PathBuf,
    target_ghost: Option<&str>,
    existing: ExistingState,
) -> InstalledElement {
    InstalledElement {
        kind,
        name: name.to_owned(),
        path,
        target_ghost: target_ghost.map(str::to_owned),
        existing,
    }
}

// ---- 成功 ----

/// 確定に成功すると、宛先に書庫の木がそのまま立ち、結果の列が返る（要件 5.10）。
///
/// 1 本の `.nar` が 2 つのインストール済みフォルダになる形（ゴースト＋同梱バルーン）で、
/// 種別・名前・絶対パス・宛先ゴースト・既存の別が要素ごとに揃うことを見る。中身は
/// 組み上げのテストと同じ期待値（＝書庫のバイト列そのもの）で突き合わせる。
#[test]
fn committing_moves_every_placement_into_place_and_returns_the_element_list() {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    let request = InstallRequest {
        root: work.path(),
        target_ghost: None,
    };
    // `accept` と読み飛ばしたキーも結果に載る（要件 5.10 後段）ので、両方書いた
    // マニフェストで通す。
    let extra = ["accept,test-accept", "nonsense,1"];
    let prepared = prepare(ghost_archive(&extra), &request);
    let ghost = work.path().join("ghost").join("test-ghost");
    let balloon = work.path().join("balloon").join("test-balloon");

    let (area, outcome) = install(work.path(), &prepared);
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
                balloon.clone(),
                None,
                ExistingState::New
            ),
        ],
        "インストール済みフォルダ 1 つにつき 1 要素"
    );
    assert_eq!(
        outcome.name, "ためしゴースト",
        "OnInstallComplete に写す名前"
    );
    assert_eq!(
        outcome.accept,
        Some("test-accept".to_owned()),
        "書かれた accept がそのまま載る"
    );
    assert_eq!(
        outcome.warnings,
        vec![ManifestWarning::IgnoredKey {
            key: "nonsense".to_owned()
        }],
        "読み飛ばしたキーも載る"
    );
    assert_eq!(outcome.leftovers, Vec::<PathBuf>::new(), "残り物は無い");

    assert_eq!(
        tree(&ghost),
        expect_tree(&[
            ("ghost/", b""),
            ("ghost/master/", b""),
            ("ghost/master/descript.txt", b"ghost descript"),
            ("install.txt", &ghost_manifest(&extra)),
            ("readme.txt", b"readme"),
            ("shell/", b""),
            ("shell/master/", b""),
            ("shell/master/surface0.png", b"png"),
        ]),
        "ゴースト側は書庫のバイト列そのもの"
    );
    assert_eq!(
        tree(&balloon),
        expect_tree(&[
            ("balloon/", b""),
            ("balloon/s0.png", b"s0"),
            ("descript.txt", b"balloon descript"),
            ("install.txt", b"type,balloon"),
        ]),
        "同梱バルーン側も書庫のバイト列そのもの"
    );
    every_file_resolves_under(&ghost);
    every_file_resolves_under(&balloon);
    assert!(
        !area.exists(),
        "確定の後に作業フォルダを片付ける: {}",
        area.display()
    );
}

/// シェルは宛先ゴーストの名前を結果に持つ（要件 5.10）。
///
/// 宛先は `<根>/ghost/<宛先>/shell/<directory>/` で、途中の `shell/` はまだ無い。
/// `rename` は親を作らないので、格納先を掘ってから入れ替えることがここで決まる。
#[test]
fn a_shell_carries_its_target_ghost_into_the_result() {
    let work = root_with_ghost("test-ghost");
    let archive = NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "charset,UTF-8",
                "type,shell",
                "name,ためしシェル",
                "directory,test-shell",
            ]),
        )
        .done()
        .file("surface0.png", b"png")
        .done();
    let request = InstallRequest {
        root: work.path(),
        target_ghost: Some("test-ghost"),
    };
    let prepared = prepare(archive, &request);
    let destination = work
        .path()
        .join("ghost")
        .join("test-ghost")
        .join("shell")
        .join("test-shell");

    let (_, outcome) = install(work.path(), &prepared);
    let outcome = outcome.expect("確定できる");

    assert_eq!(
        outcome.installed,
        vec![element(
            ElementKind::Shell,
            "ためしシェル",
            destination.clone(),
            Some("test-ghost"),
            ExistingState::New
        )],
        "宛先ゴーストの名前まで揃う"
    );
    assert_eq!(
        tree(&destination).get("surface0.png").map(Vec::as_slice),
        Some(&b"png"[..]),
        "まだ無かった shell/ の下に立つ"
    );
}

/// 既存の別は**確定の前**に決まる（要件 5.10）。
///
/// 確定すると宛先は必ず在る状態になるので、後から見ると全部「上書き」に見える。
/// 3 つの値が確定後の結果に揃って残ることを 1 本で見る。全消去の側は、マスクに無い
/// 既存ファイルが宛先から実際に消えていることまで確かめる（値だけ合っていて
/// 中身が動いていない、が起こらない）。
#[test]
fn all_three_existing_states_are_judged_before_the_swap() {
    let fresh = WorkDir::new().expect("作業フォルダを取れる");
    let request = InstallRequest {
        root: fresh.path(),
        target_ghost: None,
    };
    let prepared = prepare(ghost_archive(&[]), &request);
    let (_, outcome) = install(fresh.path(), &prepared);
    let new_states: Vec<_> = outcome
        .expect("確定できる")
        .installed
        .iter()
        .map(|element| element.existing)
        .collect();
    assert_eq!(
        new_states,
        vec![ExistingState::New, ExistingState::New],
        "どちらの宛先も無かった"
    );

    let work = WorkDir::new().expect("作業フォルダを取れる");
    let ghost = work.path().join("ghost").join("test-ghost");
    let balloon = work.path().join("balloon").join("test-balloon");
    make_tree(&ghost, &[("profile/ghost.dat", b"ghost state")]);
    make_tree(&balloon, &[("keep.dat", b"kept"), ("gone.dat", b"gone")]);
    let request = InstallRequest {
        root: work.path(),
        target_ghost: None,
    };
    let prepared = prepare(
        ghost_archive(&["balloon.refresh,1", "balloon.refreshundeletemask,keep.dat"]),
        &request,
    );
    let (_, outcome) = install(work.path(), &prepared);
    let existing: Vec<_> = outcome
        .expect("確定できる")
        .installed
        .iter()
        .map(|element| element.existing)
        .collect();

    assert_eq!(
        existing,
        vec![ExistingState::Overlaid, ExistingState::Refreshed],
        "本体は重ね置き・同梱バルーンは全消去"
    );
    assert_eq!(
        tree(&ghost).get("profile/ghost.dat").map(Vec::as_slice),
        Some(&b"ghost state"[..]),
        "重ね置きの側は既存が残る"
    );
    assert_eq!(
        tree(&balloon),
        expect_tree(&[
            ("balloon/", b""),
            ("balloon/s0.png", b"s0"),
            ("descript.txt", b"balloon descript"),
            ("install.txt", b"type,balloon"),
            ("keep.dat", b"kept"),
        ]),
        "全消去の側はマスクの名前だけ残る"
    );
}

// ---- ⑴ 宛先が使用中（要件 6.6） ----

/// 宛先の中のファイルが他のプロセスに開かれていたら、宛先を呼ぶ前のまま保って失敗する。
///
/// 解放は試みない（要件 6.6・SHIORI のアンロードは呼び出し側の責務）。1 手目の退避が
/// その場で失敗するので、宛先は 1 バイトも動かない。
#[test]
fn a_destination_in_use_fails_the_commit_and_leaves_it_byte_identical() {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    let destination = work.path().join("balloon").join("test-balloon");
    make_tree(
        &destination,
        &[("shiori.dll", b"loaded"), ("descript.txt", b"old descript")],
    );
    let before = tree(&destination);
    let request = InstallRequest {
        root: work.path(),
        target_ghost: None,
    };
    let prepared = prepare(balloon_archive(&[]), &request);
    let handle = hold(&destination.join("shiori.dll"));

    let (_, outcome) = install(work.path(), &prepared);
    let failure = outcome.expect_err("使用中なので確定できない");

    assert_eq!(failure.phase, IoPhase::Commit, "確定の段で失敗した");
    assert_eq!(failure.path, destination, "対象パスが分かる");
    assert!(failure.rolled_back, "宛先は呼ぶ前のまま");
    assert_eq!(failure.committed, vec![], "確定した配置は 1 つも無い");
    assert_eq!(tree(&destination), before, "宛先は 1 バイトも変わらない");
    drop(handle);
}

// ---- ⑵ 2 手目の失敗で退避が戻る ----

/// 「作業フォルダ → 宛先」が失敗したら、退避した木を宛先へ戻す。
///
/// 組み上げた木の中のファイルを掴んでおくと、退避（1 手目）は通り、2 手目だけが
/// 失敗する。宛先が空のまま残る経路が無いことを、呼ぶ前のバイト列で確かめる。
#[test]
fn a_failing_second_rename_puts_the_retired_tree_back() {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    let destination = work.path().join("balloon").join("test-balloon");
    make_tree(&destination, &[("descript.txt", b"old descript")]);
    let before = tree(&destination);
    let request = InstallRequest {
        root: work.path(),
        target_ghost: None,
    };
    let prepared = prepare(balloon_archive(&[]), &request);

    let area = WorkArea::create(work.path()).expect("作業フォルダを作れる");
    let states = stage_into(&area, &prepared);
    // 組み上げた木を掴む＝2 手目の `rename` だけが失敗する。
    let handle = hold(&area.stage(0).join("descript.txt"));
    let failure = commit_all(&area, &prepared.manifest, &prepared.plan, &states)
        .expect_err("2 手目が失敗する");

    assert_eq!(failure.phase, IoPhase::Commit, "確定の段で失敗した");
    assert_eq!(failure.path, destination, "対象パスが分かる");
    assert!(failure.rolled_back, "退避を戻せた");
    assert_eq!(failure.committed, vec![], "確定した配置は 1 つも無い");
    assert_eq!(tree(&destination), before, "宛先は 1 バイトも変わらない");
    drop(handle);
}

// ---- ⑶ 2 配置目の失敗で 1 配置目が戻る ----

/// 後の配置が失敗したら、確定済みの配置を逆順に元へ戻す（要件 5.11・6.4）。
///
/// ゴースト（1 つ目・宛先あり）と同梱バルーン（2 つ目・宛先の中を掴む）で、
/// ゴースト側が呼ぶ前のバイト列に戻ることを見る。`committed` は確定まで進んだ配置を
/// 挙げ、`rolled_back` がそれを戻せたかを言う。
#[test]
fn a_failure_on_the_second_placement_rolls_the_first_one_back() {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    let ghost = work.path().join("ghost").join("test-ghost");
    let balloon = work.path().join("balloon").join("test-balloon");
    make_tree(
        &ghost,
        &[
            ("profile/ghost.dat", b"ghost state"),
            ("readme.txt", b"old readme"),
        ],
    );
    make_tree(&balloon, &[("shiori.dll", b"loaded")]);
    let ghost_before = tree(&ghost);
    let balloon_before = tree(&balloon);
    let request = InstallRequest {
        root: work.path(),
        target_ghost: None,
    };
    let prepared = prepare(ghost_archive(&[]), &request);
    let handle = hold(&balloon.join("shiori.dll"));

    let (_, outcome) = install(work.path(), &prepared);
    let failure = outcome.expect_err("2 つ目の宛先が使用中");

    assert_eq!(failure.phase, IoPhase::Commit, "確定の段で失敗した");
    assert_eq!(failure.path, balloon, "失敗したのは 2 つ目の宛先");
    assert!(failure.rolled_back, "1 つ目を元へ戻せた");
    assert_eq!(
        failure.committed,
        vec![element(
            ElementKind::Ghost,
            "ためしゴースト",
            ghost.clone(),
            None,
            ExistingState::Overlaid
        )],
        "確定まで進んだのはゴーストの 1 配置"
    );
    assert_eq!(tree(&ghost), ghost_before, "1 つ目は呼ぶ前のバイト列に戻る");
    assert_eq!(
        tree(&balloon),
        balloon_before,
        "2 つ目は 1 バイトも変わらない"
    );
    drop(handle);
}

/// 新規に置いた配置は、巻き戻しで**消える**（要件 5.11）。
///
/// 前の場合と同じ形だが、1 つ目の宛先が無い。戻す先が無いので「消す」が正しい振る舞い
/// で、書きかけの木が根に残らないことがここで決まる。
#[test]
fn a_placement_that_was_new_is_removed_again_when_a_later_one_fails() {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    let ghost = work.path().join("ghost").join("test-ghost");
    let balloon = work.path().join("balloon").join("test-balloon");
    make_tree(&balloon, &[("shiori.dll", b"loaded")]);
    let balloon_before = tree(&balloon);
    let request = InstallRequest {
        root: work.path(),
        target_ghost: None,
    };
    let prepared = prepare(ghost_archive(&[]), &request);
    let handle = hold(&balloon.join("shiori.dll"));

    let (_, outcome) = install(work.path(), &prepared);
    let failure = outcome.expect_err("2 つ目の宛先が使用中");

    assert!(failure.rolled_back, "1 つ目を消せた");
    assert_eq!(
        failure.committed,
        vec![element(
            ElementKind::Ghost,
            "ためしゴースト",
            ghost.clone(),
            None,
            ExistingState::New
        )],
        "確定まで進んだのはゴーストの 1 配置"
    );
    assert!(
        !ghost.exists(),
        "新規に置いた木は残らない: {}",
        ghost.display()
    );
    assert_eq!(
        tree(&balloon),
        balloon_before,
        "2 つ目は 1 バイトも変わらない"
    );
    drop(handle);
}

/// 新規のはずの宛先が確定の直前に生えていたら、それを消す予定を立てない。
///
/// 既存の別は**組み上げの時点**で決まるので、そこから確定までの間に他のプロセスが同じ
/// 名前のフォルダを作ると、`New` として来た配置の宛先が既に在る状態になる。Windows の
/// `rename` は中身の在るフォルダを上書きしないので入れ替えは失敗するが、そのとき
/// 「新規に置いた木を消す」手を積んでしまうと、巻き戻しが**自分が作っていない木**を
/// 消してしまう。だから積むのは入れ替えが通った後に限る。
///
/// この競合は 1 本の走行の中では起こせない（他のプロセスの割り込みがいる）ので、
/// 巻き戻しの失敗と同じく、判断を持つ本番関数を直接踏む。
#[test]
fn a_new_placement_whose_destination_appeared_meanwhile_does_not_schedule_it_for_deletion() {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    let request = InstallRequest {
        root: work.path(),
        target_ghost: None,
    };
    let prepared = prepare(balloon_archive(&[]), &request);
    let area = WorkArea::create(work.path()).expect("作業フォルダを作れる");
    let states = stage_into(&area, &prepared);
    assert_eq!(
        states,
        vec![ExistingState::New],
        "組み上げの時点では宛先が無い"
    );

    // ここで他のプロセスが同じ名前のフォルダを作った、という形。
    let destination = prepared.plan[0].destination.clone();
    make_tree(&destination, &[("someone-elses.txt", b"not ours")]);
    let before = tree(&destination);
    let mut undo = Vec::new();

    let failure = commit_one(&area, 0, &prepared.plan[0], ExistingState::New, &mut undo)
        .expect_err("中身の在るフォルダは上書きできない");

    assert_eq!(failure.path, destination, "対象パスが分かる");
    assert!(
        undo.is_empty(),
        "入れ替えが通っていないので、消す手は 1 つも積まれない"
    );
    assert_eq!(
        tree(&destination),
        before,
        "他人の木は 1 バイトも変わらない"
    );
}

// ---- ⑷ 残り物 ----

/// 後片付けが失敗しても確定は取り消さず、片付かなかった場所を残り物として返す。
///
/// 作業フォルダの中に掴んだままのファイルを置くと、全配置の確定が通った後の
/// `remove_dir_all` だけが失敗する。退避した木は作業フォルダの中に居るので、
/// 返す 1 本のパスがそのまま「人が片付ける場所」になる。
#[test]
fn leftovers_name_the_work_folder_when_the_cleanup_cannot_finish() {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    let destination = work.path().join("balloon").join("test-balloon");
    make_tree(&destination, &[("descript.txt", b"old descript")]);
    let request = InstallRequest {
        root: work.path(),
        target_ghost: None,
    };
    let prepared = prepare(balloon_archive(&[]), &request);

    let area = WorkArea::create(work.path()).expect("作業フォルダを作れる");
    let states = stage_into(&area, &prepared);
    let stuck = area.path().join("stuck.bin");
    fs::write(&stuck, b"stuck").expect("仕掛けを置ける");
    let handle = hold(&stuck);
    let outcome =
        commit_all(&area, &prepared.manifest, &prepared.plan, &states).expect("確定そのものは通る");

    assert_eq!(
        outcome.leftovers,
        vec![area.path().to_path_buf()],
        "片付かなかった場所を返す"
    );
    assert_eq!(
        outcome.installed.len(),
        1,
        "後片付けの失敗は確定を取り消さない"
    );
    assert_eq!(
        tree(&destination).get("descript.txt").map(Vec::as_slice),
        Some(&b"new descript"[..]),
        "宛先は新しい木になっている"
    );
    drop(handle);
}

// ---- 巻き戻しそのものが失敗したとき（要件 6.4） ----

/// 元へ戻せなかったら、戻せなかった宛先を名指して「戻っていない」と言う。
///
/// この失敗は、確定を通した後に**他のプロセスが宛先を掴む**ことでしか起きない
/// ——確定を止めるような掴み方は、その手前の入れ替えを先に失敗させるので、確定済みの
/// 手が解けなくなる状況を 1 本の走行の中では作れない。そこで解く手そのものが失敗する形
/// （退避先が消えている）で与え、判断の側を踏む。
///
/// 3 手を積んで、⑴逆順に解くこと⑵最初に躓いた宛先を名指すこと⑶1 つ躓いても残りを
/// 解くことを同時に見る。解く順は「積んだ順の逆」なので、名指されるのは**最後に積んだ**
/// 宛先になる（順が前向きなら別の宛先が名指されて赤になる）。
#[test]
fn a_rollback_that_cannot_finish_reports_the_first_stuck_destination() {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    let removable = work.path().join("ghost").join("test-extra");
    let first_stuck = work.path().join("ghost").join("test-ghost");
    let last_stuck = work.path().join("balloon").join("test-balloon");
    for tree in [&removable, &first_stuck, &last_stuck] {
        make_tree(tree, &[("descript.txt", b"new descript")]);
    }
    let missing_old = work.path().join(".nar-work").join("no-such-old");
    let committed = vec![element(
        ElementKind::Ghost,
        "ためしゴースト",
        first_stuck.clone(),
        None,
        ExistingState::Overlaid,
    )];

    let failure = roll_back(
        vec![
            Undo::Remove(removable.clone()),
            Undo::Restore {
                old: missing_old.clone(),
                dest: first_stuck.clone(),
            },
            Undo::Restore {
                old: missing_old,
                dest: last_stuck.clone(),
            },
        ],
        committed.clone(),
        StageError {
            path: work.path().join("elsewhere"),
            source: io::Error::other("確定を止めた失敗"),
        },
    );

    assert_eq!(failure.phase, IoPhase::Rollback, "巻き戻しの段で失敗した");
    assert_eq!(
        failure.path, last_stuck,
        "逆順に解くので、最後に積んだ宛先が最初に躓く"
    );
    assert!(!failure.rolled_back, "元には戻っていない");
    assert_eq!(
        failure.committed, committed,
        "確定したまま残っている配置が分かる"
    );
    assert!(
        !removable.exists(),
        "1 つ躓いても残りは解く（解ける宛先を巻き添えにしない）"
    );
}

/// 他の走行の置き土産が消せなくても止めず、残り物として結果に載せる。
///
/// この走行の木は自分の番地（`<プロセス識別子>-<連番>/`）の中だけで組み上がるので、
/// 別の番地の残骸は混ざらない。止める理由が無い代わりに、黙って捨てもしない。
#[test]
fn a_stray_work_folder_that_will_not_go_is_reported_instead_of_stopping_the_install() {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    let stray = work.path().join(".nar-work").join("999999-0");
    make_tree(&stray, &[("held.bin", b"held")]);
    let handle = hold(&stray.join("held.bin"));
    let request = InstallRequest {
        root: work.path(),
        target_ghost: None,
    };
    let prepared = prepare(balloon_archive(&[]), &request);

    let area = WorkArea::create(work.path()).expect("残骸が消せなくても作業フォルダは作れる");
    assert_eq!(
        area.residue(),
        std::slice::from_ref(&stray),
        "消せなかった残骸が分かる"
    );
    let states = stage_into(&area, &prepared);
    let outcome = commit_all(&area, &prepared.manifest, &prepared.plan, &states)
        .expect("残骸があっても確定できる");

    assert_eq!(
        outcome.leftovers,
        vec![stray.clone()],
        "残骸は結果の残り物に合流する"
    );
    assert!(
        work.path()
            .join("balloon")
            .join("test-balloon")
            .join("descript.txt")
            .is_file(),
        "インストールそのものは通っている"
    );
    drop(handle);
}
