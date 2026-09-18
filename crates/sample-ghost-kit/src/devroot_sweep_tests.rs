//! 複製の配布と、札による保護・掃除の兄弟テスト（要件 6.5・7.5・7.7・7.9・7.10）。
//!
//! # 「消えていない」も「増えていない」も空振りしやすい
//!
//! 掃除の主張は「何も作らなければ緑」、使用量の主張は「2 回測れば緑」になる形をして
//! いる。そこで本ファイルは必ず**較正の対**を置く——残骸は消す前に実在を主張し、
//! 使用量は「複製を持っている間は増える」ことを先に主張してから「周回では増えない」
//! ことを主張する。
//!
//! # 「他者の掃除」を本物にする
//!
//! 掃除は札だけを見て持ち主の生死を分ける。だから同じプロセスの別のスレッドから呼んで
//! も、別のプロセスから呼ぶのと同じ経路を通る（自分の札であっても、削除を共有しない
//! 開き方をしているので同じプロセスからは消せない）。下の 3 本は ⑴ 2 度目の**取得**が
//! 走らせる掃除、⑵ 走りっぱなしの掃除のスレッド、⑶ 8 スレッドが同じ検体を同時に取りに
//! 行く並走のいずれでも、生きている複製が消えないことを見る。

use super::cache_tests::{
    RACE_SAMPLE, contents, private_namespace, shelf, tiny_ghost_nar, whole_ghost_tree,
};
use super::*;

/// 使用量の主張を始める前に落ち着かせる周回数。
const SETTLE: usize = 3;

/// 使用量の主張に使う周回数。2 回では「増え続けない」の証明にならない。
const CYCLES: usize = 40;

/// 掃除を並走させる周回数。
const RACED_ROUNDS: usize = 60;

/// 残骸を 1 組置く。`with_lease` が真なら**閉じた**札も置く（＝持ち主の居ない組）。
fn leftover(namespace: &Path, stem: &str, with_lease: bool) -> PathBuf {
    let shelf = namespace.join(WORK);
    let tree = shelf.join(stem);
    std::fs::create_dir_all(&tree).expect("残骸の木を置けるはず");
    std::fs::write(tree.join("a.txt"), b"a").expect("残骸の中身");
    std::fs::write(tree.join("b.txt"), b"b").expect("残骸の中身");
    if with_lease {
        std::fs::write(shelf.join(format!("{stem}{LOCK_SUFFIX}")), b"")
            .expect("閉じた札を置けるはず");
    }
    tree
}

/// 名前空間の下の（要素数, バイト数）。「増え続けない」を数で判定するための物差し。
fn usage(root: &Path) -> (usize, u64) {
    let mut entries = 0;
    let mut bytes = 0;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(children) = std::fs::read_dir(&dir) else {
            continue;
        };
        for child in children.flatten() {
            let Ok(meta) = child.metadata() else { continue };
            entries += 1;
            if meta.is_dir() {
                stack.push(child.path());
            } else {
                bytes += meta.len();
            }
        }
    }
    (entries, bytes)
}

// ---- 生死の見分け（掃除の心臓） ----

/// 札を**消せたときだけ**持ち主が居ないと判定する（要件 7.5・7.7）。
///
/// `Ok` と共有違反は下の取得の検査が本物の経路で通す。`NotFound` は「走査してから
/// 消すまでの間に別のプロセスが札を消した」瞬間にしか起きず、時刻に頼らずに作れない
/// ので、判断そのものをここで判定する。同じファイルの [`report_cleanup`] は
/// `NotFound` を成功として通すので、この取り違えは机上の話ではない——`NotFound` を
/// 「持ち主が居ない」と読むと、回収中の相手の木を横から消してしまう。
#[test]
fn only_a_removed_lease_means_the_owner_is_gone() {
    assert!(
        owner_is_gone(&Ok(())),
        "消せた札の相方は消してよいこと（持ち主が居ない）"
    );
    // 32 = ERROR_SHARING_VIOLATION。生きている利用者が札を握っている。
    assert!(
        !owner_is_gone(&Err(std::io::Error::from_raw_os_error(32))),
        "共有違反の札は生きている利用者の物なので触らないこと"
    );
    assert!(
        !owner_is_gone(&Err(std::io::Error::from(std::io::ErrorKind::NotFound))),
        "札が既に無いのは別のプロセスが同時に回収中という意味なので触らないこと（「持ち主が居ない」と読んではならない）"
    );
}

// ---- 掃除（要件 7.7・7.9） ----

/// 取得のたびの掃除が、持ち主の居ない組と札の無い残骸を消す（要件 7.7・7.9）。
#[test]
fn an_acquire_sweeps_away_a_dead_pair_and_a_lockless_leftover() {
    let namespace = private_namespace();
    let nar = tiny_ghost_nar(namespace.path(), "probe-sweep", b"one");
    let dead = leftover(namespace.path(), "999999-0", true);
    let dead_lock = namespace
        .path()
        .join(WORK)
        .join(format!("999999-0{LOCK_SUFFIX}"));
    let lockless = leftover(namespace.path(), "888888-7", false);
    let half_gc = leftover(namespace.path(), "gc-777777-1", false);

    // 陽性側を先に。これが無いと後の「消えている」は何もしなくても緑になる。
    for path in [&dead, &lockless, &half_gc] {
        assert!(
            path.is_dir(),
            "残骸を先に置けていること: {}",
            path.display()
        );
    }
    assert!(dead_lock.is_file(), "持ち主の居ない札も置けていること");

    let copy = fresh_root_in(namespace.path(), "probe-sweep", &nar).expect("取得できること");

    assert!(
        !dead_lock.exists(),
        "持ち主の居ない札が消えること: {}",
        dead_lock.display()
    );
    assert!(
        !dead.exists(),
        "持ち主の居ない組の木が消えること: {}",
        dead.display()
    );
    assert!(
        !lockless.exists(),
        "札の無い残骸が消えること: {}",
        lockless.display()
    );
    assert!(
        !half_gc.exists(),
        "回収の途中で落ちた `gc-` の木が消えること: {}",
        half_gc.display()
    );
    assert_eq!(
        contents(copy.path()),
        whole_ghost_tree("probe-sweep", b"one"),
        "掃除を通しても配られる複製は完全であること"
    );
}

/// 退けられない残骸は 1 バイトも欠けずに残り、手が離れれば次の取得で片付く
/// （要件 7.7）。
///
/// 残骸の中のファイルを開いておく。**Windows は子孫が開かれているフォルダの `rename`
/// を拒む**ので、先に `gc-…` へ退けてから消す実装は最初の一歩で失敗し、残骸は無傷で
/// 残る。いきなり `remove_dir_all` する実装は隣のファイルを消してしまう。
#[test]
fn a_leftover_that_cannot_be_moved_aside_is_left_whole() {
    let namespace = private_namespace();
    let nar = tiny_ghost_nar(namespace.path(), "probe-stuck", b"one");
    let stuck = leftover(namespace.path(), "666666-3", false);
    let held = std::fs::File::open(stuck.join("a.txt")).expect("残骸の中のファイルは開けるはず");

    drop(fresh_root_in(namespace.path(), "probe-stuck", &nar).expect("片付かなくても取得は成功"));

    assert_eq!(
        contents(&stuck),
        std::collections::BTreeMap::from([
            ("a.txt".to_owned(), b"a".to_vec()),
            ("b.txt".to_owned(), b"b".to_vec()),
        ]),
        "退けられなかった残骸は 1 バイトも欠けないこと（先に `rename` してから消す）"
    );

    // 較正兼、次の取得での片付け。手を離せば同じ呼びが残骸を消す。
    drop(held);
    drop(fresh_root_in(namespace.path(), "probe-stuck", &nar).expect("次の取得"));
    assert!(
        !stuck.exists(),
        "手が離れれば次の取得で片付くこと: {}",
        stuck.display()
    );
}

// ---- 複製（要件 6.5・7.4・7.5） ----

/// 生きている複製は他者の取得が走らせる掃除で消えず、複製どうしは混ざらない
/// （要件 7.5）。
#[test]
fn a_live_copy_survives_another_acquires_sweep_and_the_two_copies_stay_separate() {
    let namespace = private_namespace();
    let nar = tiny_ghost_nar(namespace.path(), "probe-live", b"one");
    let first = fresh_root_in(namespace.path(), "probe-live", &nar).expect("1 つ目");
    std::fs::write(first.path().join("boot.log"), b"first").expect("走行の記録を書けるはず");
    let tree = first.path().to_path_buf();
    let lock = first.lock_path().to_path_buf();
    assert!(
        tree.is_dir() && lock.is_file(),
        "先に木と札の実在を確かめる: {} / {}",
        tree.display(),
        lock.display()
    );

    // 2 つ目の**取得**が掃除を走らせる＝「他者の掃除」を本物の経路で通す。
    let second = fresh_root_in(namespace.path(), "probe-live", &nar).expect("2 つ目");

    assert!(
        tree.is_dir(),
        "生きている複製の木が他者の掃除で消えないこと: {}",
        tree.display()
    );
    assert!(
        lock.is_file(),
        "生きている複製の札が他者の掃除で消えないこと: {}",
        lock.display()
    );
    assert_eq!(
        std::fs::read(tree.join("boot.log")).expect("記録は読めるはず"),
        b"first",
        "生きている複製の中身も無傷であること"
    );
    assert_ne!(
        second.path(),
        first.path(),
        "利用者ごとに別の複製を配ること"
    );
    assert!(
        !second.path().join("boot.log").exists(),
        "1 つ目の走行が書いた記録が 2 つ目の複製に現れないこと（要件 7.5）"
    );
    assert_eq!(
        contents(second.path()),
        whole_ghost_tree("probe-live", b"one"),
        "2 つ目の複製は原本どおりであること"
    );

    drop(first);
    assert!(
        !tree.exists(),
        "破棄で複製の木が消えること: {}",
        tree.display()
    );
    assert!(!lock.exists(), "破棄で札も消えること: {}", lock.display());
}

/// 複製は**原本の複写**であり、`.nar` から展開し直さない（要件 6.5）。
///
/// 原本にだけ在って `.nar` には無いファイルを置いてから取得する。複写なら現れ、
/// 再インストールする実装なら現れない。
#[test]
fn a_copy_is_taken_from_the_master_copy_and_never_re_installed_from_the_nar() {
    let namespace = private_namespace();
    let nar = tiny_ghost_nar(namespace.path(), "probe-copy", b"one");
    drop(fresh_root_in(namespace.path(), "probe-copy", &nar).expect("原本を作る"));

    let bytes = std::fs::read(&nar).expect("組んだ .nar は読める");
    let master = namespace.path().join(CACHE).join(format!(
        "probe-copy-{}-{:08x}",
        bytes.len(),
        areka_nar::crc32(&bytes)
    ));
    assert!(master.is_dir(), "原本が在ること: {}", master.display());
    std::fs::write(master.join("only-in-the-master.txt"), b"m").expect("原本へ印を置けるはず");

    let copy = fresh_root_in(namespace.path(), "probe-copy", &nar).expect("2 度目の取得");

    let mut expected = whole_ghost_tree("probe-copy", b"one");
    expected.insert("only-in-the-master.txt".to_owned(), b"m".to_vec());
    assert_eq!(
        contents(copy.path()),
        expected,
        "複製は原本と 1 ファイルも違わないこと（`.nar` からの展開を通らない＝要件 6.5）"
    );
}

/// 登記済みの検体を実物の `.nar` から複製して配り、破棄で消える（要件 7.4・7.8）。
#[test]
fn the_public_entry_hands_out_a_disposable_copy_of_a_registered_sample() {
    let copy = fresh_root(RACE_SAMPLE).expect("登記済みの検体は取れること");
    let descript = copy
        .path()
        .join("balloon")
        .join(RACE_SAMPLE)
        .join("descript.txt");
    assert!(
        descript.is_file(),
        "配られた複製が完全であること: {}",
        descript.display()
    );

    let kept = copy.path().to_path_buf();
    let lock = copy.lock_path().to_path_buf();
    drop(copy);
    assert!(!kept.exists(), "破棄で複製が消えること: {}", kept.display());
    assert!(!lock.exists(), "破棄で札が消えること: {}", lock.display());
}

// ---- 使用量（要件 7.9） ----

/// 取得と破棄を繰り返しても名前空間の使用量が増え続けない（要件 7.9）。
#[test]
fn the_namespace_usage_does_not_grow_across_many_acquire_and_drop_cycles() {
    let namespace = private_namespace();
    let nar = tiny_ghost_nar(namespace.path(), "probe-growth", b"one");
    let mut settled = (0usize, 0u64);
    let mut while_held = (0usize, 0u64);

    for round in 0..CYCLES {
        let copy = fresh_root_in(namespace.path(), "probe-growth", &nar)
            .unwrap_or_else(|err| panic!("{round} 周目の取得: {err}"));
        assert_eq!(
            contents(copy.path()),
            whole_ghost_tree("probe-growth", b"one"),
            "{round} 周目の複製が完全であること"
        );
        if round == SETTLE {
            while_held = usage(namespace.path());
        }
        drop(copy);
        if round == SETTLE {
            settled = usage(namespace.path());
        }
    }

    let after = usage(namespace.path());
    assert!(
        settled.1 > 0,
        "測っている物が実在すること（較正）: {settled:?}"
    );
    assert!(
        while_held.0 > settled.0 && while_held.1 > settled.1,
        "複製を持っている間は使用量が増えること（物差しが複製を見ている＝較正）: 持っている間 {while_held:?} / 破棄の後 {settled:?}"
    );
    assert_eq!(
        after, settled,
        "{CYCLES} 周しても使用量が増えないこと（要件 7.9）: {after:?} / {settled:?}"
    );
    assert!(
        shelf(namespace.path(), WORK).is_empty(),
        "最後に作業の棚が空であること: {:?}",
        shelf(namespace.path(), WORK)
    );
}

// ---- 走りっぱなしの掃除との並走（要件 7.5・7.7） ----

/// 掃除が走りっぱなしでも、組み上げ中の作業フォルダと生きている複製は消えない
/// （要件 7.5・7.7）。
///
/// これが札を**フォルダより先に**作ること、原本の組み上げの**間ずっと**札を開いた
/// ままにしていることの檻である。どちらかを外すと、掃除の走査が札の無い木を見つけて
/// 退けてしまう窓が開く。周回ごとに刻印を変えるので、1 周につき組み上げと複製の
/// 両方が走る。
#[test]
fn a_sweeper_running_alongside_never_disturbs_a_staging_tree_or_a_live_copy() {
    let namespace = private_namespace();
    let home = namespace.path().to_path_buf();
    std::fs::create_dir_all(home.join(WORK)).expect("作業の棚を先に作れるはず");

    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let sweeper = {
        let home = home.clone();
        let stop = std::sync::Arc::clone(&stop);
        std::thread::spawn(move || {
            let mut passes = 0u64;
            while !stop.load(Ordering::Relaxed) {
                sweep(&home);
                passes += 1;
            }
            passes
        })
    };

    for round in 0..RACED_ROUNDS {
        let mark = format!("{round:04}");
        let nar = tiny_ghost_nar(&home, "probe-raced", mark.as_bytes());
        let copy = fresh_root_in(&home, "probe-raced", &nar)
            .unwrap_or_else(|err| panic!("{round} 周目の取得（掃除が並走）: {err}"));
        let expected = whole_ghost_tree("probe-raced", mark.as_bytes());
        assert_eq!(
            contents(copy.path()),
            expected,
            "{round} 周目に配られた複製が完全であること（組み上げ中の木が消されていない）"
        );
        // 掃除が生きている複製を通り過ぎる機会を作ってから、もう一度見る。
        for _ in 0..3 {
            std::thread::yield_now();
        }
        assert_eq!(
            contents(copy.path()),
            expected,
            "{round} 周目の生きている複製が他者の掃除で消えないこと"
        );
    }

    stop.store(true, Ordering::Relaxed);
    let passes = sweeper.join().expect("掃除の担当が落ちていないこと");
    assert!(
        passes >= RACED_ROUNDS as u64,
        "掃除が周回より多く走ったこと（この検査の較正）: {passes}"
    );
}

// ---- 手動用の根（要件 1.9） ----

/// 手動用の根は**呼ぶたびに作り直す**ので、前の走行が書いた起動記録は残らない
/// （要件 1.9・9.7）。
///
/// 「2 回呼べる」だけでは何も証明しない（作り直さない実装でも緑になる）。そこで 1 回目に
/// 配られた根へ**起動記録の形をしたファイル**を置き、実在を較正してから 2 回目を呼ぶ。
#[test]
fn the_manual_root_is_rebuilt_from_scratch_on_every_call() {
    let home = private_namespace();
    let home = home.path();
    let nar = tiny_ghost_nar(home, "probe-manual", b"0001");

    let first = manual_root_in(home, "probe-manual", &nar).expect("1 回目の手動用の根");
    assert_eq!(
        contents(&first),
        whole_ghost_tree("probe-manual", b"0001"),
        "手動用の根も完全な木であること"
    );

    let profile = first
        .join("ghost")
        .join("probe-manual")
        .join("profile")
        .join("areka");
    std::fs::create_dir_all(&profile).expect("起動記録の置き場を作れるはず");
    let record = profile.join("sylphya.toml");
    std::fs::write(&record, b"boot_count = 1\n").expect("起動記録を置けるはず");
    assert!(
        record.is_file(),
        "較正: 置いた起動記録が実在すること: {}",
        record.display()
    );

    let second = manual_root_in(home, "probe-manual", &nar).expect("2 回目の手動用の根");
    assert_eq!(second, first, "手動用の根の場所は呼ぶたびに変わらないこと");
    assert!(
        !record.exists(),
        "2 回目に配られた根に前回の起動記録が残っている: {}",
        record.display()
    );
    assert_eq!(
        contents(&second),
        whole_ghost_tree("probe-manual", b"0001"),
        "2 回目も起動記録の無い新品であること"
    );
}

/// 手動用の根は**掃除の対象にしない**（設計 `devroot`「回収の対象にしない」・要件 1.9）。
///
/// プロセスが終わった後に人が使う木なので、札を持つ者が誰も居ない。掃除が `work/` の外を
/// 見れば、印字した直後に消える根を配ることになる。較正として、同じ掃除の走行が持ち主の
/// 居ない残骸を実際に消すことを隣で主張する（「何も消さない掃除」で緑にならない）。
#[test]
fn the_sweeper_never_touches_the_manual_root() {
    let home = private_namespace();
    let home = home.path();
    let nar = tiny_ghost_nar(home, "probe-manual-sweep", b"0002");
    let manual = manual_root_in(home, "probe-manual-sweep", &nar).expect("手動用の根");
    let orphan = leftover(home, "999999-0", true);

    sweep(home);

    assert!(
        !orphan.exists(),
        "較正: 持ち主の居ない残骸は同じ掃除の走行が消すこと: {}",
        orphan.display()
    );
    assert_eq!(
        contents(&manual),
        whole_ghost_tree("probe-manual-sweep", b"0002"),
        "手動用の根は掃除で欠けないこと: {}",
        manual.display()
    );
}

// ---- 並走（同一プロセスの多スレッド・要件 7.5・9.5） ----

/// 同じ検体を同時に取りに行くスレッドの本数（設計の「8 スレッド」）。
const RACING_THREADS: usize = 8;

/// 各スレッドが自分の根の直下へ置く走行の記録。**綴りは全員同じで中身だけが自分の物**
/// なので、他人の根へ書けてしまえば中身の違いとして現れる。
const RACE_MARK: &str = "boot.log";

/// 同じ検体を 8 スレッドが同時に取っても、⑴ 一方の書き込みが他方の根に現れず、
/// ⑵ 利用中の木が差し替えられない（要件 7.5・9.5）。
///
/// 全員が**窓口**（[`crate::SampleRoot::acquire`]）を通るので、通る経路は共有の名前
/// 空間を使う本物である。段は 3 つで、関門を 2 回くぐる。
///
/// 1. 8 本が同時に取得し、自分の根へ自分だけの記録を書き、木の姿を控える。
/// 2. 関門①。その後、**1 本目を握ったまま**もう 1 度取得する——取得は掃除を走らせる
///    ので、これで「8 本の生きている複製が在る状態で掃除が 8 回走る」が必ず起きる。
///    段 1 の取得が走らせる掃除は 8 本とも木が出来る前に済んでしまうことがあり、
///    それだけでは生死の見分けを壊しても捕まえられない（この段を置かずに
///    [`owner_is_gone`] を「常に持ち主が居ない」へ変えると緑のまま通ってしまった）。
/// 3. 関門②の後、控えた姿と今の姿を突き合わせる——差し替えも削除も、この等値を破る。
///
/// 取得に失敗しても**必ず両方の関門に着く**書き方にしてある（1 本が早々に落ちて残り
/// 7 本が関門で止まる、を避ける）。壁時計の期限は置かない（この repo は並走する cargo
/// に飢餓させられて赤くなる期限で何度も痛い目に遭っている）。
#[test]
fn eight_threads_acquiring_the_same_sample_keep_their_own_tree_and_never_see_the_others_writes() {
    let gate = std::sync::Arc::new(std::sync::Barrier::new(RACING_THREADS));
    let racers: Vec<_> = (0..RACING_THREADS)
        .map(|index| {
            let gate = std::sync::Arc::clone(&gate);
            std::thread::spawn(move || -> Result<Racer, String> {
                let mark = format!("thread-{index}").into_bytes();
                let staged = crate::SampleRoot::acquire(RACE_SAMPLE)
                    .map_err(|err| format!("{index} 本目の取得: {err}"))
                    .and_then(|acquired| {
                        std::fs::write(acquired.root().join(RACE_MARK), &mark)
                            .map_err(|err| format!("{index} 本目の記録: {err}"))?;
                        let snapshot = contents(acquired.root());
                        Ok((acquired, snapshot))
                    });

                // 関門①——全員が取得と書き込みを終えるまで待つ。
                gate.wait();

                // 8 本の複製が生きている状態で掃除を走らせる（取得がそれを行う）。
                let sweeper = crate::SampleRoot::acquire(RACE_SAMPLE)
                    .map_err(|err| format!("{index} 本目の掃除役の取得: {err}"));

                // 関門②——全員の掃除が済むまで待つ。
                gate.wait();

                drop(sweeper?); // 掃除役はもう用済み。
                let (acquired, snapshot) = staged?;
                let root = acquired.root().to_path_buf();
                let after = contents(&root);
                if after != snapshot {
                    return Err(format!(
                        "{index} 本目の利用中の木が差し替えられた（掃除 8 回の後）: {}",
                        root.display()
                    ));
                }
                Ok(Racer {
                    root,
                    mark,
                    tree: snapshot,
                })
            })
        })
        .collect();

    let taken: Vec<Racer> = racers
        .into_iter()
        .map(|racer| {
            racer
                .join()
                .expect("取りに行ったスレッドが落ちていないこと")
                .unwrap_or_else(|reason| panic!("{reason}"))
        })
        .collect();

    let places: std::collections::BTreeSet<&Path> =
        taken.iter().map(|racer| racer.root.as_path()).collect();
    assert_eq!(
        places.len(),
        RACING_THREADS,
        "利用者ごとに別の複製が配られること: {places:?}"
    );

    for racer in &taken {
        assert_eq!(
            racer.tree.get(RACE_MARK),
            Some(&racer.mark),
            "自分の根に在るのは自分が書いた記録だけであること（要件 7.5）: {}",
            racer.root.display()
        );
    }

    // 記録を除いた木は全員同じで、しかも空ではない（＝中身のある木を比べている）。
    let descript = format!("balloon/{RACE_SAMPLE}/descript.txt");
    let bare = |racer: &Racer| {
        let mut tree = racer.tree.clone();
        tree.remove(RACE_MARK);
        tree
    };
    let first = bare(&taken[0]);
    assert!(
        first.contains_key(&descript),
        "較正: 比べている木が中身のある完全な木であること: {:?}",
        first.keys().collect::<Vec<_>>()
    );
    for racer in &taken[1..] {
        assert_eq!(
            bare(racer),
            first,
            "記録を除けば全員が同じ完全な木を受け取ること: {}",
            racer.root.display()
        );
    }
}

/// 上の検査が 1 スレッドから持ち帰る物——配られた根・自分が書いた記録・その木の全内容。
struct Racer {
    root: PathBuf,
    mark: Vec<u8>,
    tree: std::collections::BTreeMap<String, Vec<u8>>,
}
