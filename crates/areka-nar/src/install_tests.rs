//! `install`（組み上げ）の兄弟テスト——既存なし・上書き・全消去＋マスク残しの 3 通りと、
//! バイト列の無変換・残骸の片付け（要件 4.9・5.9・6.1・6.2）。
//!
//! 主張の置き方は 3 つ。
//!
//! ⑴ 作業フォルダの中身は [`tree`] で**丸ごと**突き合わせる。写し取るのは相対パスと
//!    **バイト列そのもの**で、長さや個数では代えない。同じ長さの書き換え（改行の変換は
//!    まさにこれ）が見えなくなる道を残さないため（要件 5.9）。空フォルダも末尾 `/` の
//!    鍵として拾うので、要件 4.9 の「空フォルダを作る」が消えても赤になる。
//! ⑵ 宛先はどのテストでも**呼ぶ前と後で同じ**であることを突き合わせる。組み上げは
//!    宛先を読むだけで、入れ替えはタスク 4.3 の仕事（完了条件「宛先はまだ触らない」）。
//! ⑶ 既存の扱いは `install.txt` の綴りから通す。`refresh` や除外マスクの解釈を
//!    テストが手で組むと、マニフェスト側と食い違ったまま両方緑になる。
//!
//! 根は OS の一時フォルダではなく [`WorkDir`] が配る（要件 7.10）。フォルダ名は
//! 登記表の検体とわざと違う綴りにする（検体パスの綴りを見張る常設検査に当たらないため）。

use super::*;
use crate::container::{inflate_entry, read_central_directory};
use crate::manifest::{locate_install_txt, parse_manifest};
use crate::names::validate_entry_names;
use crate::plan::{Placement, build_plan};
use sample_ghost_kit::{NarBuilder, WorkDir, install_txt};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

// ---- 助手 ----

/// 読取 → 名前の検証 → マニフェスト → 計画 → 全エントリの伸長まで通した固定入力。
///
/// 公開面（タスク 4.4）が `open` で行うことと同じ順で通す。手で [`Placement`] を
/// 組まないので、計画側の取り違えが組み上げのテストで緑のまま残らない。
struct Prepared {
    plan: Vec<Placement>,
    /// エントリ番号で引ける伸長済みの中身（`open` が保持するもの）。
    contents: Vec<Vec<u8>>,
}

fn prepare(builder: NarBuilder, request: &InstallRequest<'_>) -> Prepared {
    let bytes = builder.bytes();
    let raw = read_central_directory(&bytes).expect("書庫そのものは無傷");
    let names = validate_entry_names(&raw).expect("名前は安全");
    let index = locate_install_txt(&names)
        .expect("最上位に install.txt がある")
        .index;
    let text = inflate_entry(&bytes, &raw[index]).expect("install.txt を伸長できる");
    let manifest = parse_manifest(&text).expect("マニフェストは受理される");
    let contents = raw
        .iter()
        .map(|entry| inflate_entry(&bytes, entry).expect("エントリを伸長できる"))
        .collect();
    let plan = build_plan(&manifest, &names, request).expect("計画は組める");
    Prepared { plan, contents }
}

/// 木の全内容。ファイルは相対パス → バイト列、フォルダは末尾 `/` の相対パス → 空。
///
/// フォルダが無ければ空の写像を返す（「宛先がまだ無い」と「宛先が空」を同じ形で扱える）。
fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    if root.is_dir() {
        walk(root, "", &mut out);
    }
    out
}

fn walk(dir: &Path, prefix: &str, out: &mut BTreeMap<String, Vec<u8>>) {
    for child in fs::read_dir(dir).expect("木を辿れる") {
        let child = child.expect("要素を読める");
        let name = child.file_name().to_string_lossy().into_owned();
        let relative = format!("{prefix}{name}");
        if child.file_type().expect("種別を読める").is_dir() {
            out.insert(format!("{relative}/"), Vec::new());
            walk(&child.path(), &format!("{relative}/"), out);
        } else {
            out.insert(relative, fs::read(child.path()).expect("中身を読める"));
        }
    }
}

/// 期待する木を綴りから組む（末尾 `/` の鍵は空フォルダ）。
fn expect_tree(entries: &[(&str, &[u8])]) -> BTreeMap<String, Vec<u8>> {
    entries
        .iter()
        .map(|(path, data)| ((*path).to_owned(), data.to_vec()))
        .collect()
}

/// 相対パスとバイト列の対から木を実際に作る（既存の宛先を仕込む口）。
fn make_tree(root: &Path, entries: &[(&str, &[u8])]) {
    for (path, data) in entries {
        if let Some(folder) = path.strip_suffix('/') {
            fs::create_dir_all(root.join(folder)).expect("フォルダを作れる");
        } else {
            let full = root.join(path);
            fs::create_dir_all(full.parent().expect("親がある")).expect("親を作れる");
            fs::write(&full, data).expect("ファイルを書ける");
        }
    }
}

/// 全ての配置を組み上げる。宛先が呼ぶ前と後で変わらないことを、その場で突き合わせる。
fn stage_all(root: &Path, prepared: &Prepared) -> (WorkArea, Vec<ExistingState>) {
    let before: Vec<_> = prepared
        .plan
        .iter()
        .map(|placement| tree(&placement.destination))
        .collect();
    let area = WorkArea::create(root).expect("作業フォルダを作れる");
    let states = prepared
        .plan
        .iter()
        .enumerate()
        .map(|(k, placement)| {
            stage_placement(&area.stage(k), placement, &prepared.contents).expect("組み上げられる")
        })
        .collect();
    for (placement, was) in prepared.plan.iter().zip(before) {
        assert_eq!(
            tree(&placement.destination),
            was,
            "宛先 {} は組み上げでは触らない（入れ替えはタスク 4.3）",
            placement.destination.display()
        );
    }
    (area, states)
}

/// 宛先ゴースト `name` が実在する空の根を 1 つ借りる（要件 7.10）。
fn root_with_ghost(name: &str) -> WorkDir {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    fs::create_dir_all(work.path().join("ghost").join(name)).expect("宛先ゴーストを作れる");
    work
}

// ---- 固定入力 ----

/// 改行・NUL・高位バイトを取り混ぜた、変換されたら必ず変わるバイト列。
///
/// CRLF・裸の CR・裸の LF・NUL・Shift_JIS の「あ」（`82 a0`）・UTF-8 として不正な列。
const RAW_BYTES: &[u8] = b"a\r\nb\rc\nd\x00e\x82\xa0f\xff\xfe";

/// 同梱バルーンつきのゴースト（検体 emo2 と同じ形）。
///
/// `ghost/`・`ghost/master/` はフォルダのエントリを持ち、`shell/master/` は持たない。
/// 同梱バルーン側も `test-balloon/balloon/` のエントリを持たない。要件 4.9 の 2 つの形が
/// 1 つの書庫に同居する。
fn ghost_manifest(extra: &[&str]) -> Vec<u8> {
    let mut lines = vec![
        "charset,UTF-8",
        "type,ghost",
        "name,ためしゴースト",
        "directory,test-ghost",
        "balloon.directory,test-balloon",
        "balloon.source.directory,test-balloon",
    ];
    lines.extend_from_slice(extra);
    install_txt(&lines)
}

fn ghost_archive(extra: &[&str]) -> NarBuilder {
    NarBuilder::new()
        .file("install.txt", &ghost_manifest(extra))
        .done()
        .file("readme.txt", b"readme")
        .done()
        .dir("ghost")
        .dir("ghost/master")
        .file("ghost/master/descript.txt", b"ghost descript")
        .done()
        .file("shell/master/surface0.png", b"png")
        .done()
        .dir("test-balloon")
        .file("test-balloon/install.txt", b"type,balloon")
        .done()
        .file("test-balloon/descript.txt", b"balloon descript")
        .done()
        .file("test-balloon/balloon/s0.png", b"s0")
        .done()
}

/// 単独のバルーンの `install.txt`。`extra` で `refresh` と除外マスクを足す。
fn balloon_manifest(extra: &[&str]) -> Vec<u8> {
    let mut lines = vec![
        "charset,UTF-8",
        "type,balloon",
        "name,ためしバルーン",
        "directory,test-balloon",
    ];
    lines.extend_from_slice(extra);
    install_txt(&lines)
}

fn balloon_archive(extra: &[&str]) -> NarBuilder {
    NarBuilder::new()
        .file("install.txt", &balloon_manifest(extra))
        .done()
        .file("descript.txt", b"new descript")
        .done()
        .file("balloon/s0.png", b"s0")
        .done()
}

// ---- 既存なし ----

/// 宛先が無ければ、書庫の木がそのまま作業フォルダに立ち上がる（要件 4.9）。
///
/// フォルダのエントリは空フォルダとして作られ（`ghost/`）、フォルダのエントリが
/// 無い枝でも親が全段作られる（`shell/`・`shell/master/`）。同梱バルーンの取り出し元は
/// 本体側に残らず、剥がした形でバルーン側に立つ（要件 5.3）。
#[test]
fn stages_the_archive_tree_verbatim_when_the_destination_is_absent() {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    let request = InstallRequest {
        root: work.path(),
        target_ghost: None,
    };
    let prepared = prepare(ghost_archive(&[]), &request);
    let (area, states) = stage_all(work.path(), &prepared);

    assert_eq!(states, vec![ExistingState::New, ExistingState::New]);
    assert_eq!(
        tree(&area.stage(0)),
        expect_tree(&[
            ("ghost/", b""),
            ("ghost/master/", b""),
            ("ghost/master/descript.txt", b"ghost descript"),
            ("install.txt", &ghost_manifest(&[])),
            ("readme.txt", b"readme"),
            ("shell/", b""),
            ("shell/master/", b""),
            ("shell/master/surface0.png", b"png"),
        ]),
        "本体側は取り出し元を含まない"
    );
    assert_eq!(
        tree(&area.stage(1)),
        expect_tree(&[
            ("balloon/", b""),
            ("balloon/s0.png", b"s0"),
            ("descript.txt", b"balloon descript"),
            ("install.txt", b"type,balloon"),
        ]),
        "同梱バルーンは接頭辞を剥がして立つ"
    );
    assert!(
        !work.path().join("ghost").join("test-ghost").exists(),
        "宛先はまだ作られない"
    );
}

// ---- 上書き（`refresh` 無効） ----

/// `refresh` が `1` でなければ既存を丸ごと取り込み、同名だけが書庫の内容で上書きされる
/// （要件 6.1）。
#[test]
fn takes_in_the_whole_existing_tree_and_overwrites_only_same_named_files_when_refresh_is_off() {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    let destination = work.path().join("balloon").join("test-balloon");
    make_tree(
        &destination,
        &[
            ("descript.txt", b"old descript"),
            ("readme.txt", b"old readme"),
            ("profile/state.dat", b"old state"),
            ("empty/", b""),
        ],
    );
    let request = InstallRequest {
        root: work.path(),
        target_ghost: None,
    };
    let prepared = prepare(balloon_archive(&["refresh,0"]), &request);
    let (area, states) = stage_all(work.path(), &prepared);

    assert_eq!(states, vec![ExistingState::Overlaid]);
    assert_eq!(
        tree(&area.stage(0)),
        expect_tree(&[
            ("balloon/", b""),
            ("balloon/s0.png", b"s0"),
            ("descript.txt", b"new descript"),
            ("empty/", b""),
            ("install.txt", &balloon_manifest(&["refresh,0"])),
            ("profile/", b""),
            ("profile/state.dat", b"old state"),
            ("readme.txt", b"old readme"),
        ]),
        "同名は書庫の内容・それ以外は既存のまま・空フォルダも残る"
    );
}

// ---- 全消去＋マスク残し（`refresh,1`） ----

/// `refresh,1` では除外マスクに挙がったファイル名だけが残る（全階層・ASCII 大小無視）
/// （要件 6.2）。
///
/// 3 つの形を同時に見る。⑴ 何段も深いところの同名は残る。⑵ 綴りが大小だけ違う名前も
/// 残る。⑶ マスクに無いファイルと、残す物を 1 つも含まないフォルダは消える。
#[test]
fn refresh_one_keeps_only_the_undelete_mask_names_at_every_level() {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    let destination = work.path().join("balloon").join("test-balloon");
    make_tree(
        &destination,
        &[
            ("readme.txt", b"old readme"),
            ("KEEPME.txt", b"kept by case"),
            ("profile/deep/nest/state.dat", b"kept deep"),
            ("profile/deep/nest/gone.txt", b"gone"),
            ("empty/", b""),
        ],
    );
    let extra = ["refresh,1", "refreshundeletemask,state.dat:keepme.TXT"];
    let request = InstallRequest {
        root: work.path(),
        target_ghost: None,
    };
    let prepared = prepare(balloon_archive(&extra), &request);
    let (area, states) = stage_all(work.path(), &prepared);

    assert_eq!(states, vec![ExistingState::Refreshed]);
    assert_eq!(
        tree(&area.stage(0)),
        expect_tree(&[
            ("KEEPME.txt", b"kept by case"),
            ("balloon/", b""),
            ("balloon/s0.png", b"s0"),
            ("descript.txt", b"new descript"),
            ("install.txt", &balloon_manifest(&extra)),
            ("profile/", b""),
            ("profile/deep/", b""),
            ("profile/deep/nest/", b""),
            ("profile/deep/nest/state.dat", b"kept deep"),
        ]),
        "マスクの名前だけが同じ位置に残り、他は消える"
    );
}

/// 同梱バルーン側の宛先には `*.refresh`・`*.refreshundeletemask` が当たる（要件 6.3）。
///
/// 本体（ゴースト）は `refresh` を書いていないので重ね置きのまま。1 本の書庫の中で
/// 2 つの配置が別々の扱いになることを、同じ 1 回の組み上げで見る。
#[test]
fn a_companion_balloon_follows_its_own_prefixed_refresh_keys() {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    make_tree(
        &work.path().join("ghost").join("test-ghost"),
        &[("profile/ghost.dat", b"ghost state")],
    );
    make_tree(
        &work.path().join("balloon").join("test-balloon"),
        &[("keep.dat", b"kept"), ("gone.dat", b"gone")],
    );
    let request = InstallRequest {
        root: work.path(),
        target_ghost: None,
    };
    let prepared = prepare(
        ghost_archive(&["balloon.refresh,1", "balloon.refreshundeletemask,keep.dat"]),
        &request,
    );
    let (area, states) = stage_all(work.path(), &prepared);

    assert_eq!(
        states,
        vec![ExistingState::Overlaid, ExistingState::Refreshed],
        "本体は重ね置き・同梱バルーンは全消去"
    );
    assert_eq!(
        tree(&area.stage(0))
            .get("profile/ghost.dat")
            .map(Vec::as_slice),
        Some(&b"ghost state"[..]),
        "本体側の既存は残る"
    );
    assert_eq!(
        tree(&area.stage(1)),
        expect_tree(&[
            ("balloon/", b""),
            ("balloon/s0.png", b"s0"),
            ("descript.txt", b"balloon descript"),
            ("install.txt", b"type,balloon"),
            ("keep.dat", b"kept"),
        ]),
        "同梱バルーン側はマスクの名前だけ残る"
    );
}

// ---- サプリメント ----

/// サプリメントは `refresh,1` が書かれていても重ね置きになる（要件 6.2・5.8）。
///
/// 宛先はゴースト本体なので、全消去はゴーストごと消しかねない。既存が丸ごと残ること
/// に加えて、最上位の `install.txt` が**既存のまま**（サプリメントの物に置き換わって
/// いない）ことも見る。
#[test]
fn a_supplement_overlays_even_when_it_declares_refresh_one() {
    let work = root_with_ghost("test-ghost");
    let destination = work.path().join("ghost").join("test-ghost");
    make_tree(
        &destination,
        &[
            ("install.txt", b"type,ghost"),
            ("ghost/master/descript.txt", b"ghost descript"),
            ("profile/ghost.dat", b"ghost state"),
        ],
    );
    let supplement = install_txt(&[
        "charset,UTF-8",
        "type,supplement",
        "name,ついか辞書",
        "directory,test-extra",
        "refresh,1",
        "refreshundeletemask,nothing.txt",
    ]);
    let archive = NarBuilder::new()
        .file("install.txt", &supplement)
        .done()
        .file("ghost/master/dic.txt", b"dic")
        .done();
    let request = InstallRequest {
        root: work.path(),
        target_ghost: Some("test-ghost"),
    };
    let prepared = prepare(archive, &request);
    let (area, states) = stage_all(work.path(), &prepared);

    assert_eq!(states, vec![ExistingState::Overlaid]);
    assert_eq!(
        tree(&area.stage(0)),
        expect_tree(&[
            ("ghost/", b""),
            ("ghost/master/", b""),
            ("ghost/master/descript.txt", b"ghost descript"),
            ("ghost/master/dic.txt", b"dic"),
            ("install.txt", b"type,ghost"),
            ("profile/", b""),
            ("profile/ghost.dat", b"ghost state"),
        ]),
        "既存は丸ごと残り、最上位の install.txt はゴーストの物のまま"
    );
}

/// 置く物が 1 つも無い配置でも、作業フォルダそのものは立つ。
///
/// `install.txt` だけのサプリメントは、その `install.txt` を重ねない（要件 5.8）ので
/// 置くファイルもフォルダも 0 件になる。宛先が空なら下敷きも敷かれない。それでも
/// 入れ替え（タスク 4.3）が `rename` する元は要るので、空のまま作っておく必要がある。
#[test]
fn the_work_folder_is_created_even_for_a_placement_with_nothing_to_place() {
    let work = root_with_ghost("test-ghost");
    let archive = NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "charset,UTF-8",
                "type,supplement",
                "name,空の追加",
                "directory,test-extra",
            ]),
        )
        .done();
    let request = InstallRequest {
        root: work.path(),
        target_ghost: Some("test-ghost"),
    };
    let prepared = prepare(archive, &request);
    assert!(
        prepared.plan[0].files.is_empty() && prepared.plan[0].dirs.is_empty(),
        "置く物が 1 つも無い計画になっている"
    );
    let (area, states) = stage_all(work.path(), &prepared);

    // 宛先（ゴースト本体）は在るが空なので、下敷きの複写も 1 件も起きない。
    assert_eq!(states, vec![ExistingState::Overlaid]);
    assert!(area.stage(0).is_dir(), "空でも作業フォルダは在る");
    assert_eq!(tree(&area.stage(0)), expect_tree(&[]), "中身は空");
}

// ---- バイト列の無変換 ----

/// 書き出す内容は読取層のバイト列そのもので、改行も文字コードも変換しない（要件 5.9）。
///
/// 無圧縮と deflate の両方で同じバイト列を通す。既存から取り込む側も同じバイト列で
/// 確かめる（複写の経路で変換が入っても赤になる）。
#[test]
fn written_content_stays_byte_identical_without_any_conversion() {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    make_tree(
        &work.path().join("balloon").join("test-balloon"),
        &[("existing.bin", RAW_BYTES)],
    );
    let manifest = balloon_manifest(&[]);
    let archive = NarBuilder::new()
        .file("install.txt", &manifest)
        .done()
        .file("stored.bin", RAW_BYTES)
        .done()
        .file("deflated.bin", RAW_BYTES)
        .deflate()
        .done();
    let request = InstallRequest {
        root: work.path(),
        target_ghost: None,
    };
    let prepared = prepare(archive, &request);
    let (area, _) = stage_all(work.path(), &prepared);

    assert_eq!(
        tree(&area.stage(0)),
        expect_tree(&[
            ("deflated.bin", RAW_BYTES),
            ("existing.bin", RAW_BYTES),
            ("install.txt", &manifest),
            ("stored.bin", RAW_BYTES),
        ]),
        "CRLF・裸の CR／LF・NUL・高位バイトが 1 つも変わらない"
    );
}

// ---- 作業フォルダ ----

/// 作業フォルダは `<根>/.nar-work/<プロセス識別子>-<連番>/<配置番号>/` に立つ。
///
/// 入れ替えが `rename` で済むのは根と同じボリュームに在るからで、この形はタスク 4.3 の
/// 前提になる。
#[test]
fn the_work_folder_sits_on_the_shelf_directly_under_the_root() {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    let area = WorkArea::create(work.path()).expect("作業フォルダを作れる");

    assert!(area.path().is_dir(), "作業フォルダが実在する");
    assert_eq!(
        area.path().parent(),
        Some(work.path().join(".nar-work").as_path()),
        "棚は根の直下"
    );
    let stem = area
        .path()
        .file_name()
        .expect("名前がある")
        .to_string_lossy()
        .into_owned();
    assert!(
        stem.starts_with(&format!("{}-", std::process::id())),
        "名前はプロセス識別子と連番: {stem}"
    );
    assert_eq!(
        area.stage(2),
        area.path().join("2"),
        "配置ごとに番号で分かれる"
    );
}

/// 前回の異常終了が残した作業フォルダは、開始時に片付ける。
#[test]
fn stale_work_folders_are_swept_before_staging_starts() {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    let stale = work.path().join(".nar-work").join("999999-0").join("0");
    make_tree(&stale, &[("junk/leftover.txt", b"leftover")]);
    assert!(stale.is_dir(), "残骸を仕込めた");

    let area = WorkArea::create(work.path()).expect("作業フォルダを作れる");

    assert!(!stale.exists(), "残骸は残らない");
    assert_eq!(
        tree(&work.path().join(".nar-work"))
            .keys()
            .filter(|key| key.starts_with("999999-0"))
            .count(),
        0,
        "残骸の枝は 1 つも残らない"
    );
    assert!(area.path().is_dir(), "自分の作業フォルダは在る");
}

/// 残骸を片付けられなければ、黙って続けずに失敗を返す。
///
/// 前回の木が混ざったまま組み上げると、宛先に入るはずの無い物が入る。同じ根への
/// 同時インストールは起こらない前提（設計「Risks」）なので、片付けられないのは
/// 異常であり、そのまま呼び手へ返す。
#[test]
fn fails_when_the_stale_residue_cannot_be_swept() {
    use std::os::windows::fs::OpenOptionsExt;

    let work = WorkDir::new().expect("作業フォルダを取れる");
    let shelf = work.path().join(".nar-work");
    fs::create_dir_all(&shelf).expect("棚を作れる");
    let held = shelf.join("held.bin");
    // 共有なしで開いたままにする（起動中のゴーストの `shiori.dll` と同じ状態）。
    let handle = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .share_mode(0)
        .open(&held)
        .expect("掴んだままのファイルを作れる");

    let failure = WorkArea::create(work.path()).expect_err("片付けられないので失敗する");

    assert_eq!(failure.path, shelf, "失敗したパスが分かる");
    assert!(held.exists(), "掴まれた物は消えていない");
    drop(handle);
}

/// 根が無ければ、根を勝手に作らずに失敗を返す（設計の事前条件）。
#[test]
fn fails_without_creating_the_root_when_the_root_is_absent() {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    let missing = work.path().join("no-such-root");

    let failure = WorkArea::create(&missing).expect_err("根が無いので失敗する");

    assert_eq!(failure.path, missing, "失敗したパスが分かる");
    assert_eq!(failure.source.kind(), std::io::ErrorKind::NotFound);
    assert!(!missing.exists(), "根は作られない");
}
