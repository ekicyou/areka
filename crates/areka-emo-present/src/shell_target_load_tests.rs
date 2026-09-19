//! シェルの読み込みの権威の核（[`load_shell_target`]・[`build_shell_target`]・[`ShellTarget`]）の檻。
//!
//! 見ているのは 3 つの判断である——⑴ 焼く絵の一覧へ「使う画像」を足す綴りが、面の表の層 0 の
//! パスと完全一致すること（索引表は文字列の完全一致で引くので、ずれれば絵は黙って出ない）
//! ⑵ `emo2` が今日どおり「使う画像 0 件・使わない画像 2 件」で読めること ⑶ 3 つの失敗
//! （一覧・読取・面 0 個）がそれぞれの枝で返ること。
//!
//! 記録（ログ）の檻はここに無い。記録を出すのはタスク 4.2 で、その檻は
//! `shell_target_emo2_tests.rs`（タスク 4.4）が持つ。
//!
//! # 受け口の置き場所（タスク 4.3 への申し送り）
//!
//! 本ファイル末尾の検体の受け口と COM 初期化は、タスク 4.3 が新設する
//! `shell_target_test_support.rs` へそのまま移すためにここへ最小形で置いてある
//! （`balloon_test_support.rs` は `pub(super)` がバルーンのモジュール境界で閉じており外から引けない）。
//! 一時フォルダは移す必要が無い——共通窓口 [`temp_path_kit::TempPath`] をその場で呼べばよい。

use super::*;

use std::path::PathBuf;
use std::sync::LazyLock;

use areka_emo_atlas::{MemoryDecoder, SetId, WicDecoderArm};
use areka_parsers::shell::parse;
use sample_ghost_kit::SampleRoot;
use temp_path_kit::TempPath;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

/// 不透明 1×1 PBGRA スペック（bake が placement を必ず産む＝非退化）。
fn opaque_1x1() -> (u32, u32, u32, Vec<u8>, bool) {
    (1, 1, 4, vec![10u8, 20, 30, 255], true)
}

/// 面の表の層 0 の画像パスを引く（層 0 が無ければ `None`）。
fn layer0_path(world: &EmoWorld, id: u32) -> Option<String> {
    world
        .surface(id)?
        .elements
        .iter()
        .find(|e| e.layer == 0)
        .map(|e| e.path.as_str().to_string())
}

/// 核の結線（表イ）: 層 0 の空いた面へ足した画像が、**同じ綴りで**索引表にも載る。
///
/// これが本タスクの要所である——焼く絵の一覧へ足す合成 `Surface` の `element0` のパスと、
/// 面の表の層 0 の [`ElementPath`] がずれると、`AtlasTable::resolve`（文字列の完全一致）が
/// 引けず、絵は記録も無しに描かれない。fs には触れず、メモリ上の復号器で核だけを通す。
#[test]
fn used_image_is_baked_under_the_same_spelling_as_layer_zero() {
    // 面 0 は `element1` だけを持つ（層 0 が空いている＝表イ）。
    let shell = parse("surface0\n{\nelement1,overlay,parts.png,0,0\n}\n");
    let selection = select_surface_images(&["surface0000.png", "parts.png"]);
    assert_eq!(
        selection.images.get(&0).map(String::as_str),
        Some("surface0000.png"),
        "前提: 面 0 の画像として `surface0000.png` が採られている"
    );

    let shell_dir = PathBuf::from(r"C:\areka-test\shell-target-core");
    let mut dec = MemoryDecoder::new();
    let (w, h, stride, bytes, has_alpha) = opaque_1x1();
    for name in ["surface0000.png", "parts.png"] {
        dec.insert(shell_dir.join(name), w, h, stride, bytes.clone(), has_alpha);
    }

    let target = build_shell_target(shell, selection, &shell_dir, &dec);

    assert!(
        target.bake_errors().is_empty(),
        "実在する 2 枚だけを焼くので脱落は 0 件: {:?}",
        target.bake_errors()
    );
    assert!(
        target
            .atlas()
            .resolve(SetId(0), "surface0000.png")
            .is_some(),
        "使う画像が焼く絵の一覧へ載っていない（合成 Surface が渡っていない）"
    );

    let world = target.build_world();
    assert_eq!(
        layer0_path(&world, 0).as_deref(),
        Some("surface0000.png"),
        "面の表の層 0 が面の画像になっていない"
    );
    assert_eq!(
        world.base_images().used.get(&0).map(String::as_str),
        Some("surface0000.png"),
        "使った画像として数えられていない"
    );

    // 同じ `ShellTarget` から何度組んでも同じ面の表になる（design Postconditions）。
    let again = target.build_world();
    assert_eq!(
        world.surface_ids().collect::<Vec<_>>(),
        again.surface_ids().collect::<Vec<_>>(),
        "組み直すたびに面の顔ぶれが変わっている"
    );
    assert_eq!(
        layer0_path(&again, 0),
        layer0_path(&world, 0),
        "組み直すたびに層 0 が変わっている"
    );
}

/// 観測可能な完了（要件 3.6・5.1）: `emo2` の実シェルを権威で読むと `Ok` で、焼く段の
/// 脱落は 0 件・面の画像は 2 枚とも `element0` に隠れて（`shadowed`）使われない。
///
/// `emo2` の直下の面画像は `surface0.png`（面 0）と `surface10.png`（面 10）の 2 枚で、
/// どちらの面も `element0` を持つ（表ウ）。ゆえに `used` は 0 件でなければならず、
/// 1 枚でも使われていれば `emo2` の見た目が変わったことを意味する。
#[test]
fn emo2_shell_loads_with_zero_bake_errors_and_two_shadowed_images() {
    with_com_initialized(|| {
        let dec = WicDecoderArm::new().expect("COM 初期化下で WIC ファクトリが作れる");
        let shell_dir = emo2_shell_dir();

        let target = load_shell_target(&shell_dir, &dec).expect("emo2 のシェルは読める");

        assert!(
            target.bake_errors().is_empty(),
            "emo2 のシェルは 1 枚も落とさずに焼ける: {:?}",
            target.bake_errors()
        );

        let world = target.build_world();
        let report = world.base_images();
        assert!(
            report.used.is_empty(),
            "emo2 は面 0・面 10 とも `element0` を持つので使う画像は 0 件: {:?}",
            report.used
        );
        assert_eq!(
            report
                .shadowed
                .iter()
                .map(|(id, file)| (*id, file.as_str()))
                .collect::<Vec<_>>(),
            vec![(0, "surface0.png"), (10, "surface10.png")],
            "直下の面画像 2 枚が `element0` に隠れて使われないこと"
        );
    });
}

/// 要件 1.3: 一覧はフォルダ**直下**の**ファイルだけ**。フォルダは名前が慣習に合っていても
/// 面の画像と認めず、サブフォルダの中身も数えない。
///
/// 名前の判定（[`select_surface_images`]）は渡された名前の列しか見ないので、フォルダを除くのは
/// 一覧を採る側（`list_file_names`）にしかない判断である。ここを外すと、`surface5.png` という
/// 名前のフォルダが面 5 を産み、サブフォルダの `surface0010.png` が面 10 を産む。
#[test]
fn directories_are_not_taken_as_surface_images() {
    let dir = TempPath::new("shell-target-dirs-are-not-images");
    std::fs::write(
        dir.child("surfaces.txt"),
        "charset,UTF-8\nsurface0\n{\nelement1,overlay,parts.png,0,0\n}\n",
    )
    .expect("記述ファイル作成");
    // 面 0 の画像（ファイル）と、面の画像の名前をしたフォルダ、その中の面 10 の名前のファイル。
    std::fs::File::create(dir.child("surface0000.png")).expect("プレースホルダ作成");
    std::fs::File::create(dir.child("parts.png")).expect("プレースホルダ作成");
    std::fs::create_dir_all(dir.child("surface5.png")).expect("フォルダ作成");
    std::fs::File::create(dir.child("surface5.png").join("surface0010.png"))
        .expect("プレースホルダ作成");

    let mut dec = MemoryDecoder::new();
    let (w, h, stride, bytes, has_alpha) = opaque_1x1();
    for name in ["surface0000.png", "parts.png"] {
        dec.insert(dir.child(name), w, h, stride, bytes.clone(), has_alpha);
    }

    let target = load_shell_target(dir.path(), &dec).expect("面 1 個のシェルは読める");
    let world = target.build_world();

    assert_eq!(
        world.surface_ids().collect::<Vec<_>>(),
        vec![0],
        "面はファイルから来た面 0 だけ（フォルダ由来の面 5・面 10 が居てはならない）"
    );
    assert_eq!(
        world.base_images().used.get(&0).map(String::as_str),
        Some("surface0000.png"),
        "直下のファイルは今までどおり面の画像として採られる（本檻が空振りしていない対照）"
    );
}

/// 要件 1.7: シェルのフォルダの一覧が取れなければ「画像 0 件」として先へ進まず失敗を返す。
#[test]
fn missing_shell_dir_yields_list_error() {
    let dir = PathBuf::from(r"C:\areka-test\shell-target-does-not-exist");
    let dec = MemoryDecoder::new();

    match load_shell_target(&dir, &dec) {
        Err(ShellLoadError::List { path, .. }) => assert_eq!(path, dir),
        other => panic!("一覧の失敗は List でなければならない: {other:?}"),
    }
}

/// `surfaces.txt` が読めない（既存の失敗・変更 0）。フォルダの一覧は通る。
#[test]
fn missing_surfaces_txt_yields_read_error() {
    let dir = TempPath::new("shell-target-no-surfaces-txt");
    let dec = MemoryDecoder::new();

    match load_shell_target(dir.path(), &dec) {
        Err(ShellLoadError::Read { path, .. }) => {
            assert_eq!(path, dir.path().join("surfaces.txt"))
        }
        other => panic!("読取の失敗は Read でなければならない: {other:?}"),
    }
}

/// `surfaces.txt` が面を 1 つも産まない（既存の失敗・変更 0）。
#[test]
fn surfaces_txt_without_any_surface_yields_empty_error() {
    let dir = TempPath::new("shell-target-empty-surfaces-txt");
    // 面を 1 つも産まない `surfaces.txt` と、面の画像 1 枚（一覧は通るが面は 0 個）。
    std::fs::write(dir.child("surfaces.txt"), "charset,UTF-8\n").expect("記述ファイル作成");
    std::fs::File::create(dir.child("surface0000.png")).expect("プレースホルダ作成");
    let dec = MemoryDecoder::new();

    match load_shell_target(dir.path(), &dec) {
        Err(ShellLoadError::Empty { path }) => assert_eq!(path, dir.path().join("surfaces.txt")),
        other => panic!("面 0 個の失敗は Empty でなければならない: {other:?}"),
    }
}

// ── 受け口（タスク 4.3 が `shell_target_test_support.rs` へ移す）───────────────────

/// COM を初期化して `f` を走らせる（`WicDecoderArm` の前提・`display_gpu_tests.rs` と同型）。
fn with_com_initialized<F: FnOnce()>(f: F) {
    unsafe {
        // 既に初期化済みでも `RPC_E_CHANGED_MODE` を許容して続行する。
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    f();
    unsafe {
        CoUninitialize();
    }
}

/// emo2 検体。段 ③ で `Drop` が複製を消すため、一時値にせずプロセス寿命で保持する。
static EMO2: LazyLock<SampleRoot> =
    LazyLock::new(|| SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体"));

/// emo2 のシェル（`shell/master/`）のフォルダを窓口から得る（検体の直パスを綴らない・要件 7.11）。
fn emo2_shell_dir() -> PathBuf {
    EMO2.folder().join("shell").join("master")
}
