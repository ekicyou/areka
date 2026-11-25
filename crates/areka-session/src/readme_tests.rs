//! 説明書のファイルの決め方と在否の決定論テスト。
//!
//! 開く処理へは差し込んだ送り先（`Opener::from_sender`）で「何を渡したか」だけを見る。
//! 本物の OS を呼ぶテストは置かない。

use std::sync::mpsc::{self, Receiver, Sender};

use log_capture_kit::{LineFormat, capture_lines};
use temp_path_kit::TempPath;

use super::destination::Target;
use super::opener::{OpenJob, Opener};
use super::*;
use crate::ghost_session::{GhostSession, GhostSlot};

/// 説明書の持ち物を World へ直に入れる（判断分岐だけを見るここでは結線 `wire_readme` を
/// 通さない）。送出端は要求を送るために返す。
fn wired(world: &mut World, path: PathBuf) -> Sender<ReadmeRequest> {
    let (tx, rx) = mpsc::channel::<ReadmeRequest>();
    world.insert_non_send(ReadmeWiring {
        path,
        rx,
        missing_logged: Cell::new(false),
    });
    tx
}

/// 置き場にゴーストを据え、開く処理の送り先を差し込む（受信端で「何を渡したか」を見る）。
fn with_opener(world: &mut World, ghost_dir: PathBuf) -> Receiver<OpenJob> {
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(None, ghost_dir))));
    let (tx, rx) = mpsc::channel::<OpenJob>();
    world.insert_resource(Opener::from_sender(tx));
    rx
}

/// 開く系の行き先（`\j` の URL）。
fn url_request(url: &str) -> ReadmeRequest {
    ReadmeRequest::Open(super::opener_test_support::dest_url(url))
}

/// 指定した出来事の行だけを拾う。
fn lines_of<'a>(lines: &'a [String], event: &str) -> Vec<&'a String> {
    lines.iter().filter(|l| l.contains(event)).collect()
}

/// 入力の段に載っている system の数（段がまだ無ければ 0）。
fn input_systems_len(world: &World) -> usize {
    world
        .resource::<Schedules>()
        .get(Input)
        .map_or(0, |input| input.systems_len())
}

// -------------------------------------------------------------------------
// 登録と結線
// -------------------------------------------------------------------------

/// 登録専用の関数は単独で呼べ、入力の段へ取り出しの system をちょうど 1 つ足す。
#[test]
fn register_readme_drain_alone_adds_one_system_to_the_input_schedule() {
    let mut world = World::new();
    world.init_resource::<Schedules>();

    register_readme_drain(&mut world);

    assert_eq!(input_systems_len(&world), 1, "取り出しの 1 本だけが載る");
    assert!(
        world.get_non_send::<ReadmeWiring>().is_none(),
        "登録は説明書の持ち物を置かない"
    );
    assert!(
        world.get_resource::<Opener>().is_some(),
        "開く専用のスレッドの持ち物が入る"
    );
}

/// 開く専用のスレッドの持ち物が既に在れば起こし直さない（1 度だけ）。
#[test]
fn register_readme_drain_keeps_an_existing_opener() {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    let jobs = with_opener(&mut world, PathBuf::from("C:\\ghosts\\sample"));

    register_readme_drain(&mut world);
    let ReadmeRequest::Open(dest) = url_request("https://example.com/") else {
        unreachable!("url_request は Open を返す");
    };
    opener::submit(&world, dest);

    assert!(
        jobs.try_recv().is_ok(),
        "差し込んだ送り先へ届く＝登録が持ち物を差し替えていない"
    );
}

/// 結線は持ち物を置くだけで、系は登録しない（登録は `ghost_session::register_systems`）。
#[test]
fn wire_readme_inserts_the_wiring_without_registering() {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    let (_tx, rx) = mpsc::channel::<ReadmeRequest>();

    wire_readme(&mut world, PathBuf::from("readme.txt"), rx);

    assert_eq!(input_systems_len(&world), 0, "結線は系を登録しない");
    let wiring = world
        .get_non_send::<ReadmeWiring>()
        .expect("結線は持ち物を置く");
    assert_eq!(wiring.path(), Path::new("readme.txt"), "決めた経路を持つ");
}

// -------------------------------------------------------------------------
// resolve_path
// -------------------------------------------------------------------------

/// `readme` キーがあれば、その値をゴーストのフォルダの根の直下で解決する。
#[test]
fn resolve_path_uses_the_readme_key_when_present() {
    let root = Path::new("C:\\ghosts\\sample");

    assert_eq!(
        resolve_path(root, Some("manual.txt")),
        root.join("manual.txt"),
        "キーの値をそのまま根の直下で解決する"
    );
}

/// キーが無ければ正典の既定名 `readme.txt` を使う。
#[test]
fn resolve_path_falls_back_to_the_canonical_default_name() {
    let root = Path::new("C:\\ghosts\\sample");

    assert_eq!(DEFAULT_README, "readme.txt", "正典の既定名");
    assert_eq!(
        resolve_path(root, None),
        root.join("readme.txt"),
        "キーが無いときは既定名で解決する"
    );
}

// -------------------------------------------------------------------------
// is_available
// -------------------------------------------------------------------------

/// 持ち物が無ければ「無い」扱い（結線前でも判断を止めない）。
#[test]
fn is_available_is_false_without_wiring() {
    let world = World::new();

    assert!(
        !is_available(&world),
        "ReadmeWiring 不在は無効（灰色）で扱う"
    );
}

/// ファイルが無ければ無効で、記録は初回だけ。置けば有効に変わる。
#[test]
fn is_available_follows_the_file_and_records_the_absence_only_once() {
    let dir = TempPath::new("readme-availability");
    let path = dir.child("readme.txt");
    let mut world = World::new();
    let _tx = wired(&mut world, path.clone());

    let ((), missing) = capture_lines(LineFormat::LevelFields, || {
        for _ in 0..3 {
            assert!(!is_available(&world), "ファイルが無い間は無効");
        }
    });
    assert_eq!(
        lines_of(&missing, "readme_missing").len(),
        1,
        "「無い」の記録は初回だけ（3 回見ても 1 行）: {missing:?}"
    );
    assert!(
        missing
            .iter()
            .any(|l| l.contains("readme_missing") && l.contains("level=DEBUG")),
        "「無い」は debug! で記録する: {missing:?}"
    );

    std::fs::write(&path, "説明書").expect("一時ディレクトリへ書けるはず");

    let ((), after) = capture_lines(LineFormat::LevelFields, || {
        assert!(is_available(&world), "ファイルを置けば有効に変わる");
    });
    assert!(
        lines_of(&after, "readme_missing").is_empty(),
        "有効になった後に「無い」の記録は出ない: {after:?}"
    );
}

/// 置き場を差し替えた後に別の無いファイルを指すと、不在の記録がもう 1 件出る。
#[test]
fn set_path_to_another_missing_file_records_the_absence_again() {
    let dir = TempPath::new("readme-set-path-missing");
    let mut world = World::new();
    let _tx = wired(&mut world, dir.child("a.txt"));
    let ((), first) = capture_lines(LineFormat::LevelFields, || assert!(!is_available(&world)));
    assert_eq!(lines_of(&first, "readme_missing").len(), 1, "{first:?}");

    set_path(&mut world, dir.child("b.txt"));

    let ((), after) = capture_lines(LineFormat::LevelFields, || assert!(!is_available(&world)));
    assert_eq!(
        lines_of(&after, "readme_missing").len(),
        1,
        "新しい置き場の不在が記録に残る: {after:?}"
    );
}

// -------------------------------------------------------------------------
// 要求の取り出し
// -------------------------------------------------------------------------

/// 持ち物が無ければ取り出しは 0 件で、黙って済ませない。
#[test]
fn take_pending_is_empty_without_wiring_and_records_it() {
    let world = World::new();

    let ((), lines) = capture_lines(LineFormat::LevelFields, || {
        assert!(take_pending(&world).is_empty(), "ReadmeWiring 不在は 0 件");
    });
    assert_eq!(
        lines_of(&lines, "readme_drain_no_wiring").len(),
        1,
        "不在を記録する（無記録の失敗経路を作らない）: {lines:?}"
    );
}

/// 要求が無ければ 0 件（毎 tick 走る取り出しの no-op 経路）。
#[test]
fn take_pending_is_empty_when_nothing_is_queued() {
    let mut world = World::new();
    let _tx = wired(&mut world, PathBuf::from("readme.txt"));

    assert!(take_pending(&world).is_empty(), "要求が無ければ 0 件");
}

/// 溜まった要求は全件取り出され、受け口に 1 件も残らない。
#[test]
fn take_pending_returns_every_queued_request_and_leaves_none() {
    let mut world = World::new();
    let tx = wired(&mut world, PathBuf::from("readme.txt"));
    for _ in 0..3 {
        tx.send(ReadmeRequest::Readme)
            .expect("受信口は生存している");
    }

    assert_eq!(take_pending(&world).len(), 3, "溜まった 3 件を全件返す");
    assert!(
        take_pending(&world).is_empty(),
        "取り出した要求は残らない（同じ要求で二度開かない）"
    );
}

/// 取り出しは説明書と開く系を届いた順に開く処理へ渡す（説明書は台本の綴り）。
#[test]
fn drain_hands_readme_and_open_requests_over_in_arrival_order() {
    let dir = TempPath::new("readme-drain-order");
    let readme = dir.child("readme.txt");
    std::fs::write(&readme, "説明書").expect("一時ディレクトリへ書けるはず");
    let mut world = World::new();
    let tx = wired(&mut world, readme.clone());
    let jobs = with_opener(&mut world, dir.path().to_path_buf());

    tx.send(ReadmeRequest::Readme)
        .expect("受信口は生存している");
    tx.send(url_request("https://example.com/a"))
        .expect("受信口は生存している");
    tx.send(ReadmeRequest::Readme)
        .expect("受信口は生存している");
    drain_readme_requests(&mut world);

    let got: Vec<(Target, String)> = jobs
        .try_iter()
        .map(|j| (j.destination.target, j.destination.tag))
        .collect();
    let readme_target = Target::Path(
        std::path::absolute(&readme)
            .expect("絶対パスにできる")
            .to_string_lossy()
            .into_owned(),
    );
    assert_eq!(
        got,
        vec![
            (readme_target.clone(), SCRIPT_README_TAG.to_owned()),
            (
                Target::Url("https://example.com/a".to_owned()),
                "\\j[https://example.com/a]".to_owned()
            ),
            (readme_target, SCRIPT_README_TAG.to_owned()),
        ],
        "届いた順に 3 件を渡す"
    );
    assert_eq!(SCRIPT_README_TAG, "\\![open,readme]", "台本の綴り");
}

// -------------------------------------------------------------------------
// open_from_world（持ち物不在）
// -------------------------------------------------------------------------

/// 持ち物が無ければ開かずに `warn!` で記録する（無記録の失敗経路を作らない）。
#[test]
fn open_from_world_warns_without_wiring() {
    let world = World::new();

    let ((), lines) = capture_lines(LineFormat::LevelFields, || {
        open_from_world(&world, MENU_TAG)
    });

    let warned = lines_of(&lines, "readme_open_no_wiring");
    assert_eq!(
        warned.len(),
        1,
        "ReadmeWiring 不在を 1 行記録する: {lines:?}"
    );
    assert!(
        warned[0].contains("level=WARN"),
        "不在は warn! レベル: {warned:?}"
    );
}

/// 説明書が在れば、絶対パスの行き先と綴りを開く処理へ 1 件渡す（OS は直接呼ばない）。
#[test]
fn open_from_world_hands_the_absolute_readme_path_to_the_opener() {
    let dir = TempPath::new("readme-open-present");
    let readme = dir.child("readme.txt");
    std::fs::write(&readme, "説明書").expect("一時ディレクトリへ書けるはず");
    let mut world = World::new();
    let _tx = wired(&mut world, readme.clone());
    let jobs = with_opener(&mut world, dir.path().to_path_buf());

    open_from_world(&world, MENU_TAG);

    let job = jobs.try_recv().expect("1 件渡す");
    let abs = std::path::absolute(&readme)
        .expect("絶対パスにできる")
        .to_string_lossy()
        .into_owned();
    assert_eq!(job.destination.target, Target::Path(abs.clone()));
    assert_eq!(job.destination.written, abs, "書かれた行き先＝説明書のパス");
    assert_eq!(job.destination.tag, MENU_TAG, "メニューの印を綴りに入れる");
    assert!(jobs.try_recv().is_err(), "ちょうど 1 件");
}

/// 説明書のファイルが無ければ開く処理へ渡さず、パスを添えて `warn!` で 1 行記録する。
#[test]
fn open_from_world_skips_a_missing_file_with_a_warning_and_no_submit() {
    let dir = TempPath::new("readme-open-missing");
    let mut world = World::new();
    let _tx = wired(&mut world, dir.child("readme.txt"));
    let jobs = with_opener(&mut world, dir.path().to_path_buf());

    let ((), lines) = capture_lines(LineFormat::LevelFields, || {
        open_from_world(&world, SCRIPT_README_TAG)
    });

    assert!(jobs.try_recv().is_err(), "無いファイルは渡さない");
    let skipped = lines_of(&lines, "readme_open_skipped_missing");
    assert_eq!(skipped.len(), 1, "要求 1 件につき 1 行: {lines:?}");
    assert!(
        skipped[0].contains("level=WARN") && skipped[0].contains("readme.txt"),
        "warn! でパスを添える: {skipped:?}"
    );
}

// -------------------------------------------------------------------------
// set_path（descript の読み直しで替える値）
// -------------------------------------------------------------------------

/// 置き場を差し替えた後の取り出しは、新しい置き場を開く処理へ渡す（受信端はそのまま使う）。
#[test]
fn set_path_makes_the_drain_open_the_new_readme() {
    let dir = TempPath::new("readme-set-path");
    let old = dir.child("readme.txt");
    let new = dir.child("readme-new.txt");
    std::fs::write(&old, "古い説明書").expect("一時ディレクトリへ書けるはず");
    std::fs::write(&new, "新しい説明書").expect("一時ディレクトリへ書けるはず");
    let mut world = World::new();
    let tx = wired(&mut world, old);
    let jobs = with_opener(&mut world, dir.path().to_path_buf());

    set_path(&mut world, new.clone());
    tx.send(ReadmeRequest::Readme)
        .expect("差し替えの後も同じ受信端が生きている");
    drain_readme_requests(&mut world);

    let job = jobs.try_recv().expect("1 件渡す");
    let abs = std::path::absolute(&new)
        .expect("絶対パスにできる")
        .to_string_lossy()
        .into_owned();
    assert_eq!(
        job.destination.target,
        Target::Path(abs),
        "新しい置き場を開く"
    );
    assert!(jobs.try_recv().is_err(), "ちょうど 1 件");
}

/// 持ち物が無ければ差し替えずに `warn!` で記録する（無記録の失敗経路を作らない）。
#[test]
fn set_path_warns_without_wiring() {
    let mut world = World::new();

    let ((), lines) = capture_lines(LineFormat::LevelFields, || {
        set_path(&mut world, PathBuf::from("readme.txt"))
    });

    let warned = lines_of(&lines, "readme_set_path_no_wiring");
    assert_eq!(warned.len(), 1, "不在を 1 行記録する: {lines:?}");
    assert!(warned[0].contains("level=WARN"), "不在は warn!: {warned:?}");
}
