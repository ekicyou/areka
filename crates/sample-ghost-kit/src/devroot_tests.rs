//! `devroot` の兄弟テスト。
//!
//! # 「残っていない」を空振りにしない
//!
//! 後始末の判定は「何も作らなければ自動的に緑」になる形をしている。そこで本ファイルの
//! 後始末の主張は必ず**対**で書く——生きている間に木と札が**実在すること**を先に主張し、
//! そのうえで破棄の後に両方が消えていることを主張する。
//!
//! # 並走の中で安定させる
//!
//! 判定は常に**自分が取った `<プロセス識別子>-<連番>` の 2 つのパスだけ**を見る。名前空間の
//! 下が空であることは主張しない（同じプロセスの別のテストが同時に別の作業フォルダを
//! 持っているのが正常な状態であり、そこを見ると走らせ方次第で赤くなる）。
//!
//! # OS の一時フォルダを使わないこと（要件 7.10）の担保
//!
//! ここでは置き場の綴りを**持たない**。本ファイルが一時フォルダの入口を綴ると、常設検査
//! `temp_path_guard_test` の例外表に「窓口の迂回」として登録させることになり、迂回を
//! 数えている表が嘘になるからである。代わりに ⑴ 名前空間がビルド成果物の置き場からだけ
//! 導かれること（下の判定）と、⑵ その常設検査が `devroot` に一時フォルダの入口が現れた
//! 瞬間に赤くなることの 2 つで担保する。

use super::*;

/// 削除の共有（`FILE_SHARE_DELETE`）。較正でだけ使うので本体には置かない。
const FILE_SHARE_DELETE: u32 = 4;

/// 木と札の実在を 1 組にして読む。
fn exists(work: &WorkDir) -> (bool, bool) {
    (work.path().is_dir(), work.lock_path().is_file())
}

#[test]
fn the_work_dir_lives_under_the_spec_namespace_and_never_in_the_os_temp_dir() {
    let work = WorkDir::new().expect("ビルド成果物の置き場は見つかるはず");
    let path = work.path();

    assert!(
        path.is_absolute(),
        "借り手に渡すのは絶対パス: {}",
        path.display()
    );
    let namespace = namespace_dir().expect("名前空間は組めるはず");
    assert_eq!(
        namespace.file_name().and_then(|name| name.to_str()),
        Some(NAMESPACE),
        "名前空間はビルド成果物の置き場の直下の専用フォルダ（要件 7.1）: {}",
        namespace.display()
    );
    assert!(
        path.starts_with(&namespace),
        "作業フォルダは本仕様専用の名前空間の下に作ること（要件 7.1）: {} / {}",
        path.display(),
        namespace.display()
    );
    // 名前は `<プロセス識別子>-<連番>`、札はその隣に `.lock` を付けた名前。
    let stem = path
        .file_name()
        .and_then(|n| n.to_str())
        .expect("作業フォルダの名前");
    assert!(
        stem.starts_with(&format!("{}-", std::process::id())),
        "プロセス間の一意性はプロセス識別子で取る: {stem}"
    );
    assert_eq!(
        work.lock_path().file_name().and_then(|n| n.to_str()),
        Some(format!("{stem}.lock").as_str()),
        "札は木と同じ名前に `.lock` を付けた隣の位置"
    );
}

#[test]
fn the_tree_and_the_lease_both_exist_while_held_and_both_are_gone_after_drop() {
    let work = WorkDir::new().expect("作業フォルダは取れるはず");
    let tree = work.path().to_path_buf();
    let lock = work.lock_path().to_path_buf();

    // まず陽性側（ここが緑でなければ後の「消えている」は空振りになる）。
    assert_eq!(
        exists(&work),
        (true, true),
        "生きている間は木も札も実在すること: {} / {}",
        tree.display(),
        lock.display()
    );
    // 借り手が「空の根」として使えること（`areka-nar` のテストがこの形で借りる）。
    assert_eq!(
        std::fs::read_dir(&tree)
            .expect("作業フォルダは読めるはず")
            .count(),
        0,
        "配るのは空の根"
    );
    std::fs::write(tree.join("ghost.txt"), b"x").expect("作業フォルダへは書けるはず");

    drop(work);

    assert!(!tree.exists(), "破棄で木が消えること: {}", tree.display());
    assert!(!lock.exists(), "破棄で札が消えること: {}", lock.display());
}

#[test]
fn two_work_dirs_in_one_process_get_different_places_and_do_not_disturb_each_other() {
    let first = WorkDir::new().expect("1 つ目");
    let second = WorkDir::new().expect("2 つ目");

    assert_ne!(
        first.path(),
        second.path(),
        "同じプロセスの連番で別の場所になること"
    );
    assert_ne!(first.lock_path(), second.lock_path(), "札も別であること");
    assert_eq!(exists(&first), (true, true));
    assert_eq!(exists(&second), (true, true));

    let kept_tree = second.path().to_path_buf();
    let kept_lock = second.lock_path().to_path_buf();
    drop(first);

    assert_eq!(
        exists(&second),
        (true, true),
        "隣の破棄が生きている作業フォルダを巻き込まないこと: {} / {}",
        kept_tree.display(),
        kept_lock.display()
    );
}

#[test]
fn the_lease_refuses_deletion_while_it_is_held() {
    let work = WorkDir::new().expect("作業フォルダは取れるはず");

    // 本命: 札は `FILE_SHARE_READ` だけで開いてあるので、持ち主が居る間は削除できない
    //（この拒否が、後続タスクの掃除で「生きている利用者」と「死んだ残骸」を分ける）。
    let refused = std::fs::remove_file(work.lock_path());
    assert!(
        refused.is_err(),
        "生きている札は削除を拒むこと（共有モードが効いていない）: {}",
        work.lock_path().display()
    );

    // 較正: 同じ開き方で削除の共有だけを足した相方は、開いたままでも削除を受け付ける。
    // これが通ることで、上の拒否が「開いているから」ではなく**共有モードの指定**に
    // よるものだと分かる（指定を外しても赤にならない恒真の判定を避ける）。
    let control = work.path().join("control.lock");
    let handle = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_DELETE)
        .open(&control)
        .expect("較正用の札は開けるはず");
    assert!(
        std::fs::remove_file(&control).is_ok(),
        "削除を共有した札は開いたままでも削除できること（較正が壊れている）"
    );
    drop(handle);

    // 札が消えていないこと＝拒否は本物（ここまでの主張が陽性側の裏付けになる）。
    assert!(
        work.lock_path().is_file(),
        "拒否された削除は札を消していない"
    );
}

#[test]
fn the_configured_target_dir_wins_over_the_ancestor_search() {
    let chosen = find_target_dir(
        Some("some-target"),
        Path::new(r"C:\x\target\debug\deps\t.exe"),
    )
    .expect("設定値があればそれを使う");
    assert!(
        chosen.ends_with("some-target"),
        "環境変数の指定が祖先探索より優先されること: {}",
        chosen.display()
    );
    assert!(
        chosen.is_absolute(),
        "相対の指定でも絶対パスにして返すこと: {}",
        chosen.display()
    );

    let found = find_target_dir(None, Path::new(r"C:\x\target\debug\deps\t.exe"))
        .expect("祖先に target があれば見つかる");
    assert_eq!(found, Path::new(r"C:\x\target"));
}

#[test]
fn a_missing_target_dir_fails_with_the_place_it_searched_from() {
    let started_from = Path::new(r"C:\somewhere\bin\t.exe");
    let err = find_target_dir(None, started_from).expect_err("祖先に target が無い");
    let SampleError::TargetDirNotFound {
        started_from: reported,
    } = &err
    else {
        panic!("理由の付いた失敗を返すこと: {err:?}");
    };
    assert_eq!(
        reported, started_from,
        "どこから探したのかを失敗に載せること"
    );
    assert!(
        err.to_string().contains("t.exe"),
        "失敗の表示に探索の起点が出ること: {err}"
    );
}

/// 原本の据え付けが `rename` の勝敗で決まる（7.6）ことの**前提の実測**。
///
/// 設計は「Windows の `std::fs::rename` は宛先が**空でない**フォルダのときに失敗し、
/// 空フォルダなら置き換える」という規則の上に立っている。この規則が崩れると、負けた側が
/// 勝者の木を上書きしてしまう。前提そのものをここで測る（推測で通さない）。
#[test]
fn a_rename_onto_a_non_empty_folder_fails_and_onto_an_empty_folder_replaces() {
    let work = WorkDir::new().expect("作業フォルダは取れるはず");
    let source = work.path().join("src");
    std::fs::create_dir(&source).expect("移す木");
    std::fs::write(source.join("a.txt"), b"src").expect("中身");

    // 本命: 宛先が空でなければ失敗し、宛先は 1 バイトも変わらない。
    let occupied = work.path().join("occupied");
    std::fs::create_dir(&occupied).expect("宛先");
    std::fs::write(occupied.join("b.txt"), b"dst").expect("先客");
    let refused = std::fs::rename(&source, &occupied);
    assert!(
        refused.is_err(),
        "空でないフォルダへの rename は失敗すること（7.6 の勝敗判定の前提）"
    );
    assert_eq!(
        std::fs::read(occupied.join("b.txt")).expect("先客は無傷"),
        b"dst",
        "負けた側が勝者の木を書き換えていないこと"
    );
    assert!(source.is_dir(), "失敗した rename は元の木を動かさないこと");

    // 較正: 空フォルダなら置き換わる（＝上の失敗は「フォルダだから」ではなく
    // 「空でないから」だと分かる）。原本も宛先も常に空でない木なので、実運用では
    // こちらの経路には入らない。
    let empty = work.path().join("empty");
    std::fs::create_dir(&empty).expect("空の宛先");
    std::fs::rename(&source, &empty).expect("空フォルダへは置き換えられること");
    assert_eq!(
        std::fs::read(empty.join("a.txt")).expect("移った木"),
        b"src"
    );
    assert!(!source.exists(), "移した元は残らないこと");
}
