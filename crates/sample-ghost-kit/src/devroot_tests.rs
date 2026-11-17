//! `devroot` の兄弟テスト。要件の番号は spec `areka-P0-nar-install` のもの。
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
fn the_work_dir_is_absolute_and_named_after_the_process_with_its_lease_beside_it() {
    let work = WorkDir::new().expect("ビルド成果物の置き場は見つかるはず");
    let path = work.path();

    assert!(
        path.is_absolute(),
        "借り手に渡すのは絶対パス: {}",
        path.display()
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
    //（この拒否が、掃除で「生きている利用者」と「死んだ残骸」を分ける）。
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
        Some(r"D:\elsewhere\tgt"),
        Path::new(r"C:\x\target\debug\deps\t.exe"),
    )
    .expect("設定値があればそれを使う");
    assert_eq!(
        chosen,
        Path::new(r"D:\elsewhere\tgt"),
        "絶対の指定は祖先探索より優先され、そのまま使うこと"
    );

    let found = find_target_dir(None, Path::new(r"C:\x\target\debug\deps\t.exe"))
        .expect("祖先に target があれば見つかる");
    assert_eq!(found, Path::new(r"C:\x\target"));
}

/// 相対の設定値は、実行体の祖先のうちその値で終わる最も近いものに解く（audit-test-kits 決め D4）。
///
/// cargo は起こした場所の上で解き、テストの実行体はパッケージの場所で走るので、
/// 作業フォルダの上で解くと別の場所（ソースの木の中）を指してしまう。
#[test]
fn a_relative_target_dir_is_resolved_against_the_exe_ancestors() {
    let exe = Path::new(r"C:\tgt\repo\tgt\debug\deps\t.exe");
    assert_eq!(
        find_target_dir(Some("tgt"), exe).expect("祖先に在る"),
        Path::new(r"C:\tgt\repo\tgt"),
        "その値で終わる祖先のうち実行体に最も近いもの"
    );
    assert_eq!(
        find_target_dir(Some(r"out\tgt"), Path::new(r"C:\repo\out\tgt\debug\t.exe"))
            .expect("2 段の値も祖先に在る"),
        Path::new(r"C:\repo\out\tgt"),
        "複数の段の値も段ごとに照らす"
    );
    for spelled in [r"..\tgt", r".\tgt"] {
        assert_eq!(
            find_target_dir(Some(spelled), exe).expect("頭の . と .. を外して祖先に在る"),
            Path::new(r"C:\tgt\repo\tgt"),
            "頭の . と .. を外した名前の段で照らす: {spelled}"
        );
    }

    // 祖先に無ければ今までどおり作業フォルダの上で絶対にする。
    let fallback = find_target_dir(Some("not-an-ancestor"), exe).expect("絶対にできる");
    assert!(
        fallback.is_absolute() && fallback.ends_with("not-an-ancestor"),
        "祖先に無い値は作業フォルダの上で解く: {}",
        fallback.display()
    );
}

/// 死んだプロセスが残した同じ名前の札やフォルダに当たっても、次の連番で取り直す（audit-test-kits 指摘 7）。
///
/// 残りの札は他人の物かもしれないので消さない（片付けるのは掃除の仕事）。
#[test]
fn a_leftover_lease_or_folder_with_the_same_name_is_skipped() {
    let namespace = WorkDir::new().expect("私有の名前空間");
    let work = namespace.path().join(WORK);
    let pid = std::process::id();
    std::fs::create_dir_all(work.join(format!("{pid}-1"))).expect("残りのフォルダを置ける");
    let left_lease = work.join(format!("{pid}-0{LOCK_SUFFIX}"));
    std::fs::write(&left_lease, b"").expect("残りの札を置ける");

    let mut serials = 0..;
    let got = WorkDir::in_namespace_with(namespace.path(), || serials.next().expect("連番"))
        .expect("残りを飛ばして取れること");

    assert_eq!(
        got.path(),
        work.join(format!("{pid}-2")),
        "空いた連番まで進む"
    );
    assert_eq!(exists(&got), (true, true));
    assert!(left_lease.is_file(), "残りの札は消さない");
    assert!(
        !work.join(format!("{pid}-1{LOCK_SUFFIX}")).exists(),
        "フォルダが先客で取れなかった連番の、自分の札は畳む"
    );
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

/// 設定値が空・空白だけなら無いものとして祖先の探しへ戻る（audit-test-kits 指摘 69）。
#[test]
fn a_blank_target_dir_falls_back_to_the_ancestor_search() {
    let exe = Path::new(r"C:\x\target\debug\deps\t.exe");
    for blank in ["", "   "] {
        assert_eq!(
            find_target_dir(Some(blank), exe).expect("祖先に target がある"),
            Path::new(r"C:\x\target"),
            "{blank:?}"
        );
    }
}

/// 手動用の根の中を削除を拒む形で掴まれていると、黙って重ねず `Io` で断る（audit-test-kits 指摘 23）。
#[test]
fn manual_root_refuses_a_tree_it_cannot_remove() {
    let namespace = WorkDir::new().expect("私有の名前空間");
    let nar = || {
        Ok(crate::NarBuilder::new()
            .file(
                "install.txt",
                &crate::install_txt(&[
                    "charset,UTF-8",
                    "type,ghost",
                    "name,held",
                    "directory,held",
                ]),
            )
            .done()
            .file("mark.txt", b"held")
            .done()
            .bytes())
    };
    let manual = manual_root_in(namespace.path(), "held", nar).expect("1 度目は作れる");
    let held_file = manual.join("ghost").join("held").join("mark.txt");
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(&held_file)
        .expect("中のファイルを開けるはず");

    let refused = manual_root_in(namespace.path(), "held", nar);
    match &refused {
        Err(SampleError::Io { what, path, .. }) => {
            assert_eq!(*what, "remove the manual root before rebuilding it");
            assert_eq!(*path, manual);
        }
        other => panic!("掴まれた木は Io で断ること: {other:?}"),
    }
    assert!(held_file.is_file(), "掴まれたファイルは消えていない");
    drop(held);
}

/// 後始末の失敗は書き先へ `sample-ghost-kit: cleanup failed (` で始まる 1 行を書き、
/// 既に無い（`NotFound`）ときは何も書かない（`tools/load-flake.ps1` がこの頭と
/// `os error N` を字面で読む）。
#[test]
fn a_cleanup_failure_is_written_to_the_given_sink_and_not_found_writes_nothing() {
    let path = Path::new("held");

    let mut failed = Vec::new();
    write_cleanup(
        &mut failed,
        "remove the work folder",
        Err(std::io::Error::from_raw_os_error(5)),
        path,
    );
    let failed = String::from_utf8(failed).expect("ASCII で書く");
    assert!(
        failed.starts_with("sample-ghost-kit: cleanup failed (remove the work folder): held ("),
        "頭の字面: {failed}"
    );
    assert!(failed.contains("os error 5"), "os error N を保つ: {failed}");

    let mut not_found = Vec::new();
    write_cleanup(
        &mut not_found,
        "remove the work folder",
        Err(std::io::ErrorKind::NotFound.into()),
        path,
    );
    assert!(not_found.is_empty(), "既に無いのは黙って通す");
}
