//! 原本（`cache/<検体>-<刻印>/`）の兄弟テスト——刻印・初回の展開・古い原本の回収・
//! 多重プロセスの競合（要件 7.2・7.3・7.6・7.7・7.8）。
//!
//! # 私有の名前空間で測る
//!
//! 判定は全て [`WorkDir`] の中に掘った**私有の名前空間**の上で行う。共有の
//! `target/nar-samples/` を使うと、「名前空間を丸ごと消す」（要件 7.8）や「原本は 1 つ」
//! の主張が、同時に走っている他のテストの原本や作業フォルダを巻き込む／巻き込まれる。
//! 私有にすれば主張は全数で書けるし、破棄で丸ごと消えるので後始末も要らない。
//!
//! # 固定入力は自分で組む
//!
//! 検体の実物（6.6 MB）ではなく [`crate::NarBuilder`] で組んだ最小のゴーストを使う。
//! **中身の長さを変えずに**目印だけ書き換えられるので、刻印が長さだけでなく CRC でも
//! 決まっていることを判定できる。実物の `.nar` と `nar_dir()` の綴りは、下の多重
//! プロセスの検査が [`cached_root`] 経由で通す。
//!
//! # 「半端な原本の自己修復」を測らない理由
//!
//! `cache/<検体>-<刻印>/` は「名前が合えば完全」という約束で運用する（作業フォルダで
//! 組んでから `rename` で入れ、消すときは先に `rename` で出す）。だから手で半分消した
//! 原本が完全とみなされるのは**設計どおり**であり、そこを検査にすると設計と逆の要求を
//! 固定してしまう。代わりに「据え付け前の木は答えにならない」ことを測る。

use super::*;

/// 多重プロセスの検査に使う検体。登記の中で `.nar` が一番小さいもの。
pub(super) const RACE_SAMPLE: &str = "emo2-kakukaku-wplimit";

/// 子として起こされたことを伝える環境変数（値は子の番号）。
const CHILD_VAR: &str = "SAMPLE_GHOST_KIT_RACE_CHILD";

/// 子に走らせるテストの完全な名前。
const CHILD_TEST: &str = "devroot::cache_tests::the_multiprocess_child_acquires_the_master_copy";

/// 他のテストと混ざらない私有の名前空間。破棄で丸ごと消える。
pub(super) fn private_namespace() -> WorkDir {
    WorkDir::new().expect("私有の名前空間は取れるはず")
}

/// 最小のゴースト 1 体だけを含む `.nar` を組んで置く。
///
/// `mark` は中身の目印で、長さを変えずに中身だけ変えられる。
pub(super) fn tiny_ghost_nar(dir: &Path, name: &str, mark: &[u8]) -> PathBuf {
    let nar = dir.join(format!("{name}.nar"));
    crate::NarBuilder::new()
        .file("install.txt", &manifest_of(name))
        .done()
        .file("ghost/master/descript.txt", b"charset,UTF-8\r\n")
        .done()
        .file("mark.txt", mark)
        .done()
        .write_to(&nar)
        .expect("固定入力の .nar を置けるはず");
    nar
}

/// [`tiny_ghost_nar`] が書く `install.txt`。期待値の木と綴りを共有する。
fn manifest_of(name: &str) -> Vec<u8> {
    crate::install_txt(&[
        "charset,UTF-8",
        "type,ghost",
        &format!("name,{name}"),
        &format!("directory,{name}"),
    ])
}

/// [`tiny_ghost_nar`] の検体が完全に展開されたときの木の**全内容**。
///
/// 「完全な原本」を 1 ファイルずつ確かめる形は、隣のファイルが落ちていても緑になる。
/// 完了状態の 2 本はこの写像との等値で judge する。
pub(super) fn whole_ghost_tree(
    name: &str,
    mark: &[u8],
) -> std::collections::BTreeMap<String, Vec<u8>> {
    std::collections::BTreeMap::from([
        (format!("ghost/{name}/install.txt"), manifest_of(name)),
        (
            format!("ghost/{name}/ghost/master/descript.txt"),
            b"charset,UTF-8\r\n".to_vec(),
        ),
        (format!("ghost/{name}/mark.txt"), mark.to_vec()),
    ])
}

/// 棚の直下の名前を並べる（棚が無ければ空）。
pub(super) fn shelf(namespace: &Path, name: &str) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(namespace.join(name)) else {
        return Vec::new();
    };
    let mut out: Vec<String> = entries
        .map(|entry| {
            entry
                .expect("棚は辿れる")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    out.sort();
    out
}

/// 原本の名前（`cache/` の中の 1 件の綴り）。
fn leaf(path: &Path) -> String {
    path.file_name()
        .expect("原本には名前がある")
        .to_string_lossy()
        .into_owned()
}

/// 木の中の全ファイルの相対パスとバイト列。木が丸ごと無傷かを 1 つの値で比べる。
pub(super) fn contents(root: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    let mut out = std::collections::BTreeMap::new();
    let mut stack = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, prefix)) = stack.pop() {
        for child in std::fs::read_dir(&dir).expect("木は辿れる") {
            let child = child.expect("要素は読める");
            let name = format!("{prefix}{}", child.file_name().to_string_lossy());
            if child.file_type().expect("種別は読める").is_dir() {
                stack.push((child.path(), format!("{name}/")));
            } else {
                out.insert(name, std::fs::read(child.path()).expect("中身は読める"));
            }
        }
    }
    out
}

/// 原本の中の目印を読む。読めれば原本は完全に展開されている。
fn mark_of(master: &Path, name: &str) -> Vec<u8> {
    let path = master.join("ghost").join(name).join("mark.txt");
    std::fs::read(&path)
        .unwrap_or_else(|err| panic!("原本が完全なら目印が読めるはず: {} ({err})", path.display()))
}

// ---- 刻印 ----

/// 刻印の割り戻しが、名前が名前の接頭辞になっている検体を取り違えない。
///
/// `emo2` の古い原本を探すときに `emo2-kakukaku-wplimit` の原本へ当たると、生きている
/// 別の検体の原本を回収してしまう。刻印は必ず末尾の 2 つなので後ろから剥がす。
#[test]
fn the_stamp_is_split_off_from_the_end_so_a_name_prefix_is_not_confused_for_the_name() {
    assert_eq!(
        split_stamp("emo2-6631325-1a2b3c4d"),
        Some(("emo2", "6631325-1a2b3c4d"))
    );
    assert_eq!(
        split_stamp("emo2-kakukaku-wplimit-33906-deadbeef"),
        Some(("emo2-kakukaku-wplimit", "33906-deadbeef")),
        "名前に含まれる `-` を刻印の区切りと取り違えないこと"
    );
    // 刻印の形をしていないものは「この仕様が置いた原本ではない」＝触らない。
    for other in [
        "emo2",
        "emo2-6631325",
        "emo2-6631325-1a2b3c4",   // 16 進が 7 桁
        "emo2-6631325-1a2b3c4de", // 16 進が 9 桁
        "emo2-66x1325-1a2b3c4d",  // 長さが 10 進でない
        "-6631325-1a2b3c4d",      // 名前が空
        "gc-1234-5",
    ] {
        assert_eq!(split_stamp(other), None, "刻印の形をしていない: {other}");
    }
}

// ---- 初回の展開と刻印の変化（完了状態の 2 本） ----

/// 原本が無い状態からの取得が、刻印を名前に含む**完全な原本 1 つ**に落ち着く（要件 7.2）。
#[test]
fn an_acquire_with_no_master_copy_installs_exactly_one_complete_master_copy() {
    let namespace = private_namespace();
    let nar = tiny_ghost_nar(namespace.path(), "probe-fresh", b"one");
    let bytes = std::fs::read(&nar).expect("組んだ .nar は読める");

    // 陽性側を先に。これが無いと後の「1 つ」は何も作らなくても緑になる。
    assert!(
        shelf(namespace.path(), CACHE).is_empty(),
        "始めは原本が 1 つも無いこと"
    );

    let master = cached_root_in(namespace.path(), "probe-fresh", &nar).expect("原本を作れること");

    assert_eq!(
        leaf(&master),
        format!(
            "probe-fresh-{}-{:08x}",
            bytes.len(),
            areka_nar::crc32(&bytes)
        ),
        "原本の名前は `<検体>-<長さ>-<crc32 8 桁>`"
    );
    assert_eq!(
        shelf(namespace.path(), CACHE),
        vec![leaf(&master)],
        "落ち着く先は原本 1 つ"
    );
    assert_eq!(
        contents(&master),
        whole_ghost_tree("probe-fresh", b"one"),
        "原本が完全であること（1 枚ずつではなく全ファイルで見る）"
    );
    assert!(
        shelf(namespace.path(), WORK).is_empty(),
        "据え付けの後の作業フォルダに木も札も残らないこと: {:?}",
        shelf(namespace.path(), WORK)
    );

    // 刻印が同じなら展開し直さない。据え付けは必ず作業フォルダを 1 つ取るので、
    // 棚の場所にファイルを置いて「組み直せない」状態にしてから呼ぶ。
    // cache-hit なら棚に触れずに答えが返り、展開し直す実装なら必ず失敗する。
    //
    // 原本へ目印を置いて「残っているか」を見る形では判定にならない——展開し直す実装は
    // 据え付けの `rename` に負けるだけで、原本は目印ごと無傷のまま残るからである
    // （要件 7.2 の「無いときだけ展開し直す」が丸ごと無検査になる）。
    std::fs::remove_dir_all(namespace.path().join(WORK)).ok();
    std::fs::write(namespace.path().join(WORK), b"no shelf here").expect("棚の場所を塞げる");

    let again = cached_root_in(namespace.path(), "probe-fresh", &nar)
        .expect("cache-hit は作業フォルダを要らないこと（＝展開し直していない）");
    assert_eq!(again, master, "刻印が同じなら同じ原本");
}

/// 刻印を変えた `.nar` での取得が、新しい原本 1 つに落ち着き、古い原本は回収される
/// （要件 7.3）。
///
/// 目印の長さを変えずに書き換えるので、刻印が長さだけで決まっていれば赤になる。
#[test]
fn a_changed_nar_installs_a_new_master_copy_and_the_old_one_is_reclaimed() {
    let namespace = private_namespace();
    let nar = tiny_ghost_nar(namespace.path(), "probe-restamp", b"one");
    let old = cached_root_in(namespace.path(), "probe-restamp", &nar).expect("1 度目");
    assert_eq!(mark_of(&old, "probe-restamp"), b"one");

    let nar = tiny_ghost_nar(namespace.path(), "probe-restamp", b"two");
    let new = cached_root_in(namespace.path(), "probe-restamp", &nar).expect("2 度目");

    assert_ne!(new, old, "刻印が変われば別の原本になること");
    assert_eq!(
        contents(&new),
        whole_ghost_tree("probe-restamp", b"two"),
        "新しい原本が完全であること（1 枚ずつではなく全ファイルで見る）"
    );
    assert_eq!(
        shelf(namespace.path(), CACHE),
        vec![leaf(&new)],
        "落ち着く先は原本 1 つ（古い原本が回収されている）"
    );
    assert!(!old.exists(), "古い原本が消えていること: {}", old.display());
}

/// 別の検体の原本は、名前が接頭辞になっていても回収されない（要件 7.3 の範囲）。
#[test]
fn reclaiming_one_sample_leaves_another_samples_master_copy_alone() {
    let namespace = private_namespace();
    let other = tiny_ghost_nar(namespace.path(), "probe-twin-extra", b"one");
    let kept = cached_root_in(namespace.path(), "probe-twin-extra", &other).expect("隣の原本");

    let nar = tiny_ghost_nar(namespace.path(), "probe-twin", b"one");
    cached_root_in(namespace.path(), "probe-twin", &nar).expect("1 度目");
    let nar = tiny_ghost_nar(namespace.path(), "probe-twin", b"two");
    let new = cached_root_in(namespace.path(), "probe-twin", &nar).expect("2 度目");

    assert!(
        kept.is_dir(),
        "名前が接頭辞になっている隣の原本を巻き込まないこと: {}",
        kept.display()
    );
    assert_eq!(
        shelf(namespace.path(), CACHE),
        {
            let mut both = vec![leaf(&kept), leaf(&new)];
            both.sort();
            both
        },
        "残るのは隣の原本と新しい原本の 2 つだけ"
    );
}

/// 退避（`cache/` から出す `rename`）に失敗しても取得は成功し、次の取得で回収し直す
/// （要件 7.3・7.7）。
///
/// 古い原本の中のファイルを開いたまま取得する。**Windows は子孫が開かれている
/// フォルダの `rename` を拒む**（下の較正で実測する。共有モードに削除を足しても同じ）
/// ので、回収は最初の一歩で失敗する。それでも取得は止まらず、古い原本は `cache/` に
/// **無傷のまま**残る（半端に消しかけた木にはならない）。
#[test]
fn a_reclaim_that_cannot_move_the_stale_copy_keeps_going_and_retries_on_the_next_acquire() {
    let namespace = private_namespace();
    let nar = tiny_ghost_nar(namespace.path(), "probe-held", b"one");
    let old = cached_root_in(namespace.path(), "probe-held", &nar).expect("1 度目");
    let inside = old.join("ghost").join("probe-held").join("mark.txt");
    let before = contents(&old);
    assert!(
        before.len() >= 3,
        "古い原本には複数のファイルが在る: {before:?}"
    );
    let held = std::fs::File::open(&inside).expect("原本の中のファイルは開けるはず");

    let nar = tiny_ghost_nar(namespace.path(), "probe-held", b"two");
    let new = cached_root_in(namespace.path(), "probe-held", &nar)
        .expect("回収が失敗しても取得は成功すること");

    assert_eq!(mark_of(&new, "probe-held"), b"two", "新しい原本は完全");
    // 無傷＝**全ファイル**が元のまま。掴まれているファイル 1 つだけを見ると、先に消して
    // から退避する実装（＝順序を入れ替えた実装）が隣のファイルを消していても緑になる。
    assert_eq!(
        contents(&old),
        before,
        "退避に失敗した古い原本は 1 バイトも欠けないこと（`cache/` に半端な木を作らない）"
    );
    assert_eq!(
        shelf(namespace.path(), CACHE),
        {
            let mut both = vec![leaf(&old), leaf(&new)];
            both.sort();
            both
        },
        "退避に失敗した分だけ原本が 2 つ残る"
    );

    // 較正兼、次の取得での回収。手を離せば同じ呼びが古い原本を片付ける。
    drop(held);
    let again = cached_root_in(namespace.path(), "probe-held", &nar).expect("3 度目");
    assert_eq!(again, new, "刻印が同じなら同じ原本");
    assert_eq!(
        shelf(namespace.path(), CACHE),
        vec![leaf(&new)],
        "次の取得で回収し直すこと"
    );
}

/// 新しい `.nar` が拒否されたときは、古い原本を回収しない（要件 7.3・7.7）。
///
/// 回収を据え付けより**先**に行うと、展開に失敗した瞬間に「使える原本が 1 つも無い」
/// 状態になる。順序が守られていれば、拒否されても手元には前の原本が残る。
#[test]
fn a_refused_nar_leaves_the_previous_master_copy_in_place() {
    let namespace = private_namespace();
    let nar = tiny_ghost_nar(namespace.path(), "probe-refused", b"one");
    let old = cached_root_in(namespace.path(), "probe-refused", &nar).expect("1 度目");

    // 終端記録を落とした書庫＝読取層が受理しない別の刻印の入力。
    let broken = namespace.path().join("probe-refused-broken.nar");
    crate::NarBuilder::new()
        .file(
            "install.txt",
            &crate::install_txt(&[
                "type,ghost",
                "name,probe-refused",
                "directory,probe-refused",
            ]),
        )
        .done()
        .damage(crate::Damage::NoEocd)
        .write_to(&broken)
        .expect("壊した .nar を置ける");

    let err = cached_root_in(namespace.path(), "probe-refused", &broken)
        .expect_err("受理されない .nar は失敗になること");
    assert!(
        matches!(err, SampleError::Nar(_)),
        "拒否は理由の付いた失敗で返ること: {err:?}"
    );
    assert_eq!(
        shelf(namespace.path(), CACHE),
        vec![leaf(&old)],
        "展開に失敗しても前の原本は残ること（回収は据え付けの後）"
    );
    assert_eq!(
        mark_of(&old, "probe-refused"),
        b"one",
        "前の原本は完全なまま"
    );
}

// ---- 作り直し（要件 7.7・7.8） ----

/// 名前空間を丸ごと消しても、次の取得で作り直す（要件 7.8）。
#[test]
fn deleting_the_whole_namespace_rebuilds_the_master_copy_on_the_next_acquire() {
    // `.nar` は名前空間の外に置く（消す対象に入れない）。
    let fixtures = private_namespace();
    let namespace = private_namespace();
    let nar = tiny_ghost_nar(fixtures.path(), "probe-wipe", b"one");

    let first = cached_root_in(namespace.path(), "probe-wipe", &nar).expect("1 度目");
    assert_eq!(mark_of(&first, "probe-wipe"), b"one");

    std::fs::remove_dir_all(namespace.path()).expect("名前空間を丸ごと消せること");
    assert!(!first.exists(), "消えたことを先に確かめる");

    let second =
        cached_root_in(namespace.path(), "probe-wipe", &nar).expect("消えていても作り直すこと");
    assert_eq!(second, first, "刻印が同じなら同じ名前へ戻ること");
    assert_eq!(
        mark_of(&second, "probe-wipe"),
        b"one",
        "作り直した原本が完全であること"
    );
}

/// 作業フォルダに半端な木が在っても cache-hit にならない（要件 7.7）。
///
/// 「名前が合う原本は完全」は `cache/` の中でだけ成り立つ約束なので、据え付け前の木を
/// 原本と同じ名前で `work/` に置いても答えにしてはならない。
#[test]
fn a_half_built_tree_in_the_work_shelf_is_never_taken_as_the_master_copy() {
    let namespace = private_namespace();
    let nar = tiny_ghost_nar(namespace.path(), "probe-half", b"one");
    let bytes = std::fs::read(&nar).expect("読める");
    let stamped = format!(
        "probe-half-{}-{:08x}",
        bytes.len(),
        areka_nar::crc32(&bytes)
    );

    // 据え付け前の半端な木を、原本と同じ名前で作業の棚に置く。
    let decoy = namespace.path().join(WORK).join(&stamped);
    std::fs::create_dir_all(&decoy).expect("囮を置ける");
    std::fs::write(decoy.join("half.txt"), b"half").expect("囮の中身");

    let master = cached_root_in(namespace.path(), "probe-half", &nar).expect("取得できること");

    assert_eq!(
        master,
        namespace.path().join(CACHE).join(&stamped),
        "答えは `cache/` の下"
    );
    assert_eq!(mark_of(&master, "probe-half"), b"one", "答えは完全な木");
    assert!(
        decoy.join("half.txt").is_file(),
        "囮は答えにも据え付けの材料にもされないこと（片付けるのは取得の入口が走らせる掃除で、原本を用意するこの関数ではない）"
    );
}

// ---- 多重プロセスの初回展開（要件 7.6） ----

/// 据え付けに負けた側は、自分の作業を捨てて勝者の木を使う（要件 7.6）。
///
/// 勝者を先に置いてから**据え付けだけ**をもう一度走らせる。宛先が空でない木なので
/// `rename` は必ず失敗する＝負けの経路を時刻に頼らず必ず通る。
#[test]
fn a_lost_race_discards_its_own_stage_and_keeps_the_winner() {
    let namespace = private_namespace();
    let nar = tiny_ghost_nar(namespace.path(), "probe-race", b"one");
    let winner = cached_root_in(namespace.path(), "probe-race", &nar).expect("勝者を置く");
    std::fs::write(winner.join("winner.txt"), b"w").expect("勝者の目印");

    stage_into_cache(
        namespace.path(),
        &namespace.path().join(CACHE),
        &nar,
        &winner,
    )
    .expect("負けた側も失敗にはならないこと");

    assert!(
        winner.join("winner.txt").is_file(),
        "勝者の木が上書きされていないこと"
    );
    assert_eq!(
        shelf(namespace.path(), CACHE),
        vec![leaf(&winner)],
        "原本は 1 つのまま"
    );
    assert!(
        shelf(namespace.path(), WORK).is_empty(),
        "負けた側の作業フォルダも札も残らないこと: {:?}",
        shelf(namespace.path(), WORK)
    );
}

/// 空の名前空間へ 4 つのプロセスが同時に取得を始めても、全員が完全な木を受け取り、
/// `cache/` に半端な木が残らない（要件 7.6・9.5）。
///
/// 子は**自分自身のテストバイナリ**で、`--exact` で下の子側のテストだけを走らせる。
/// 子の名前空間は `CARGO_TARGET_DIR` で私有の場所へ振るので、この検査の外で走っている
/// テストの原本にも作業フォルダにも触らない。実物の `.nar` をこの経路だけが通るので、
/// `nar_dir()` の綴りもここで確かめられる。
///
/// 子の待ちに**期限を置かないのは意図**である。この repo は壁時計の期限で何度も痛い目に
/// 遭っている（他の cargo が並走すると飢餓して赤くなる）。止まったプロセスは原因を追える
/// が、時々赤くなる期限は追えない。「timeout を足す」改善は入れないこと。
#[test]
fn four_processes_racing_the_first_install_all_get_a_complete_tree() {
    if std::env::var(CHILD_VAR).is_ok() {
        return; // 子として起こされたときに孫を作らない。
    }
    let home = private_namespace();
    let exe = std::env::current_exe().expect("テストバイナリの場所");

    let children: Vec<_> = (0..4)
        .map(|index| {
            std::process::Command::new(&exe)
                .args(["--exact", CHILD_TEST, "--test-threads", "1"])
                .env("CARGO_TARGET_DIR", home.path())
                .env(CHILD_VAR, index.to_string())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .expect("子を起こせること")
        })
        .collect();

    for (index, child) in children.into_iter().enumerate() {
        let out = child.wait_with_output().expect("子の終了を待てること");
        assert!(
            out.status.success(),
            "子 {index} が完全な木を得られなかった: {}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    let namespace = home.path().join(NAMESPACE);
    let masters = shelf(&namespace, CACHE);
    assert_eq!(masters.len(), 1, "原本は 1 つに落ち着くこと: {masters:?}");
    let descript = namespace
        .join(CACHE)
        .join(&masters[0])
        .join("balloon")
        .join(RACE_SAMPLE)
        .join("descript.txt");
    assert!(
        descript.is_file(),
        "`cache/` に残るのは完全な木であること: {}",
        descript.display()
    );
    let leftovers = shelf(&namespace, WORK);
    assert!(
        leftovers.is_empty(),
        "負けた側の作業フォルダも札も残らないこと: {leftovers:?}"
    );
}

/// 上の検査が起こす子。親の走行では環境変数が無いので何もしない。
///
/// 「完全な木」の判定は 2 段。⑴ 目印の 1 ファイルが在って空でないこと——競争が
/// 生むのは**半端な木**なので、これだけでも半分は捕まる。⑵ 同じ `.nar` を自分用の
/// 空の根へ展開し直した木と**1 ファイルも違わない**こと——目印以外が欠けた木は⑴を
/// すり抜けるので、こちらが本体である。
#[test]
fn the_multiprocess_child_acquires_the_master_copy() {
    let Ok(index) = std::env::var(CHILD_VAR) else {
        return;
    };
    let master = cached_root(RACE_SAMPLE)
        .unwrap_or_else(|err| panic!("子 {index} が原本を得られない: {err}"));
    let descript = master
        .join("balloon")
        .join(RACE_SAMPLE)
        .join("descript.txt");
    assert!(
        descript.is_file(),
        "子 {index} が受け取った木が完全でない: {}",
        descript.display()
    );
    assert!(
        !std::fs::read(&descript).expect("読める").is_empty(),
        "子 {index} が受け取った木の中身が空"
    );

    // 同じ `.nar` を自分だけの空の根へ展開し直して、原本と丸ごと突き合わせる。
    let reference = WorkDir::new().unwrap_or_else(|err| panic!("子 {index} の空の根: {err}"));
    let nar = nar_dir().join(format!("{RACE_SAMPLE}.nar"));
    let archive =
        NarArchive::open(&nar).unwrap_or_else(|err| panic!("子 {index} の .nar の読み取り: {err}"));
    archive
        .install(&InstallRequest {
            root: reference.path(),
            target_ghost: None,
        })
        .unwrap_or_else(|err| panic!("子 {index} の突き合わせ用の展開: {err}"));
    assert_eq!(
        contents(&master),
        contents(reference.path()),
        "子 {index} が受け取った原本が、展開し直した木と食い違う（半端な木）"
    );
}
