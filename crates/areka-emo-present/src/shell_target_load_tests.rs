//! シェルの読み込みの権威の核（[`load_shell_target`]・[`build_shell_target`]・[`ShellTarget`]）の檻。
//!
//! 見ているのは 3 つの判断である——⑴ 焼く絵の一覧へ「使う画像」を足す綴りが、面の表の層 0 の
//! パスと完全一致すること（索引表は文字列の完全一致で引くので、ずれれば絵は黙って出ない）
//! ⑵ `emo2` が今日どおり「使う画像 0 件・使わない画像 2 件」で読めること ⑶ 3 つの失敗
//! （一覧・読取・面 0 個）がそれぞれの枝で返ること。
//!
//! 併せて ⑷ 権威が出す記録（要件 6）——6 種の記録が読み込み 1 回につきそれぞれ 1 度だけ出て、
//! `build_world` は新しい記録を 0 本出すこと、3 つの失敗がどれも `error!` を伴うこと——を
//! 本ファイル後半で判定する。`emo2` の絵の不変（A／B の全画素の一致）はタスク 4.4 が
//! `shell_target_emo2_tests.rs` で持つ。
//!
//! # 受け口の置き場所
//!
//! 検体の受け口・COM 初期化・ログの捕捉窓は、`shell_target` の檻が共有する
//! `shell_target_test_support.rs`（[`super::test_support`]）に在る。一時フォルダは共有窓口
//! [`temp_path_kit::TempPath`] をその場で呼ぶ。

use super::*;

use std::path::PathBuf;

use areka_emo_atlas::{MemoryDecoder, SetId, WicDecoderArm};
use areka_parsers::shell::parse;
use temp_path_kit::TempPath;

use super::test_support::{CapturedEvent, capture_events, emo2_shell_dir, with_com_initialized};

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

// ── 記録（要件 1.5・1.6・3.5・6.1・6.2・6.4・6.5）──────────────────────────────

/// 本モジュールが出す記録の宛先（既定の target＝モジュールパス・design「Monitoring」）。
const SHELL_TARGET: &str = "areka_emo_present::shell_target";

/// 宛先が権威のものである記録だけを数える。
fn count_from_shell_target(events: &[CapturedEvent], level: tracing::Level) -> usize {
    events
        .iter()
        .filter(|e| e.target == SHELL_TARGET && e.level == level)
        .count()
}

/// 宛先が権威のもので、本文に `needle` を含む記録を 1 件だけ取り出す。
fn only_from_shell_target<'a>(
    events: &'a [CapturedEvent],
    level: tracing::Level,
    needle: &str,
) -> &'a CapturedEvent {
    let hits: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.target == SHELL_TARGET && e.level == level && e.message().contains(needle))
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "`{needle}` を含む {level} の記録は 1 件でなければならない: {hits:?}"
    );
    hits[0]
}

/// 6 種の記録が、**読み込み 1 回につきそれぞれ 1 度だけ**出る（要件 6.1・6.2・6.4）。
///
/// 1 つのシェルに 6 つの事象を同居させてある——面 0 は同じ番号の画像が 2 枚（重複）で
/// `element0` を持つ（使わない）、面 1 は画像を土台に使い、相手の居ないコマ（面 777）を持ち、
/// `element1` の絵は復号器が知らない（焼く段で脱落）、そして桁溢れの名前が 1 つ在る。
#[test]
fn every_record_is_emitted_once_per_load() {
    let dir = TempPath::new("shell-target-records");
    std::fs::write(
        dir.child("surfaces.txt"),
        concat!(
            "charset,UTF-8\n",
            "surface0\n{\nelement0,overlay,base0.png,0,0\n}\n",
            "surface1\n{\nelement1,overlay,parts1.png,0,0\n",
            "animation0.interval,random,4\n",
            "animation0.pattern0,overlay,777,0,0,0\n}\n",
        ),
    )
    .expect("記述ファイル作成");
    for name in [
        "surface0.png",
        "surface0000.png",
        "surface1.png",
        "surface99999999999.png",
        "base0.png",
        "parts1.png",
    ] {
        std::fs::File::create(dir.child(name)).expect("プレースホルダ作成");
    }

    // 焼けるのは 2 枚だけ——`parts1.png` を入れないので、その 1 枚が焼く段で落ちる。
    let mut dec = MemoryDecoder::new();
    let (w, h, stride, bytes, has_alpha) = opaque_1x1();
    for name in ["base0.png", "surface1.png"] {
        dec.insert(dir.child(name), w, h, stride, bytes.clone(), has_alpha);
    }

    let (target, events) =
        capture_events(|| load_shell_target(dir.path(), &dec).expect("シェルは読める"));

    // 6.1: 一覧の結果は 1 行だけ。
    let summary = only_from_shell_target(&events, tracing::Level::INFO, "一覧");
    assert_eq!(summary.field("recognized"), Some("2"), "認めた画像は 2 枚");
    assert_eq!(
        summary.field("used"),
        Some("1"),
        "土台に使ったのは面 1 の 1 枚"
    );
    assert_eq!(
        summary.field("shadowed"),
        Some("1"),
        "`element0` が在って使わなかったのは面 0 の 1 枚"
    );
    assert_eq!(
        summary.field("shell_dir"),
        Some(dir.path().display().to_string().as_str()),
        "どのシェルの結果かが読み取れる"
    );
    assert_eq!(
        count_from_shell_target(&events, tracing::Level::INFO),
        1,
        "読み込み 1 回につき `info!` は 1 行だけ"
    );

    // 6.2: 使わなかった画像は面ごとに 1 行。
    let shadowed = only_from_shell_target(&events, tracing::Level::DEBUG, "element0");
    assert_eq!(shadowed.field("surface_id"), Some("0"));
    assert_eq!(shadowed.field_str("file"), Some("surface0.png"));

    // 1.6: 桁溢れは名前ごとに 1 行。
    let overflow = only_from_shell_target(&events, tracing::Level::DEBUG, "大きすぎる");
    assert_eq!(overflow.field_str("file"), Some("surface99999999999.png"));

    assert_eq!(
        count_from_shell_target(&events, tracing::Level::DEBUG),
        2,
        "`debug!` は使わなかった画像 1 行＋桁溢れ 1 行だけ"
    );

    // 1.5: 同じ番号の重複は番号ごとに 1 行。
    let duplicate = only_from_shell_target(&events, tracing::Level::WARN, "同じ番号");
    assert_eq!(duplicate.field("surface_id"), Some("0"));
    assert_eq!(duplicate.field_str("adopted"), Some("surface0.png"));
    assert_eq!(
        duplicate.field("dropped"),
        Some("[\"surface0000.png\"]"),
        "捨てた名前が読み取れる"
    );

    // 3.5: 相手の無いコマは組ごとに 1 行。
    let dangling = only_from_shell_target(&events, tracing::Level::WARN, "コマ");
    assert_eq!(dangling.field("surface_id"), Some("1"));
    assert_eq!(dangling.field("target"), Some("777"));

    // 4.9 ほか: 焼く段で落ちた絵は絵ごとに 1 行（実機確認が数える語を含む）。
    let bake = only_from_shell_target(
        &events,
        tracing::Level::WARN,
        "shell bake で脱落した element",
    );
    assert!(
        bake.field("error")
            .is_some_and(|e| e.contains("parts1.png")),
        "どの絵が落ちたかが読み取れる: {:?}",
        bake.field("error")
    );
    assert_eq!(target.bake_errors().len(), 1, "落ちた絵は 1 枚（前提）");

    assert_eq!(
        count_from_shell_target(&events, tracing::Level::WARN),
        3,
        "`warn!` は重複・相手の無いコマ・脱落の 3 行だけ"
    );

    // `build_world` は新しい記録を 0 本出す（design「Monitoring」）。
    let (world, again) = capture_events(|| target.build_world());
    assert_eq!(
        count_from_shell_target(&again, tracing::Level::WARN)
            + count_from_shell_target(&again, tracing::Level::INFO)
            + count_from_shell_target(&again, tracing::Level::DEBUG)
            + count_from_shell_target(&again, tracing::Level::ERROR),
        0,
        "面の表を組み直しても権威は記録を出さない: {again:?}"
    );
    // 対照: この窓で面の表の構築そのものは動いていた（焼けなかった `parts1.png` の装着が
    // 既存の記録を出す）。これが 0 件なら上の主張は空振りである。
    assert!(
        again
            .iter()
            .any(|e| e.target.starts_with("areka_emo_compose")),
        "面の表の構築の既存の記録が 1 本も無い（窓が素通りしている）: {again:?}"
    );
    assert!(world.surface(1).is_some(), "対照の面の表は実際に組めている");
}

/// 要件 6.4・1.7: 一覧が取れない失敗は `error!` を伴う（記録の無い失敗経路を持たない）。
#[test]
fn list_failure_is_recorded_before_the_error() {
    let dir = PathBuf::from(r"C:\areka-test\shell-target-records-no-dir");
    let dec = MemoryDecoder::new();

    let (result, events) = capture_events(|| load_shell_target(&dir, &dec));

    assert!(matches!(result, Err(ShellLoadError::List { .. })));
    let hit = only_from_shell_target(&events, tracing::Level::ERROR, "一覧");
    assert_eq!(
        hit.field("shell_dir"),
        Some(dir.display().to_string().as_str())
    );
    assert!(hit.field("error").is_some(), "OS の理由が載っている");
}

/// 要件 6.4: `surfaces.txt` が読めない失敗は `error!` を伴う。
#[test]
fn read_failure_is_recorded_before_the_error() {
    let dir = TempPath::new("shell-target-records-no-surfaces-txt");
    let dec = MemoryDecoder::new();

    let (result, events) = capture_events(|| load_shell_target(dir.path(), &dec));

    assert!(matches!(result, Err(ShellLoadError::Read { .. })));
    let hit = only_from_shell_target(&events, tracing::Level::ERROR, "読み取り");
    assert!(hit.field("error").is_some(), "OS の理由が載っている");
}

/// 要件 6.4: 面が 0 個の失敗は `error!` を伴う。
#[test]
fn empty_failure_is_recorded_before_the_error() {
    let dir = TempPath::new("shell-target-records-empty-surfaces-txt");
    std::fs::write(dir.child("surfaces.txt"), "charset,UTF-8\n").expect("記述ファイル作成");
    let dec = MemoryDecoder::new();

    let (result, events) = capture_events(|| load_shell_target(dir.path(), &dec));

    assert!(matches!(result, Err(ShellLoadError::Empty { .. })));
    let hit = only_from_shell_target(&events, tracing::Level::ERROR, "1 つも産まなかった");
    assert_eq!(
        hit.field("path"),
        Some(
            dir.path()
                .join("surfaces.txt")
                .display()
                .to_string()
                .as_str()
        )
    );
}

/// 観測可能な完了（タスク 4.2）: `emo2` のシェルで `recognized=2 used=0 shadowed=2` の
/// `info!` が 1 行、使わなかった画像の `debug!` が 2 行、`warn!` が 0 行になる。
#[test]
fn emo2_shell_records_two_shadowed_images_and_no_warnings() {
    with_com_initialized(|| {
        let dec = WicDecoderArm::new().expect("COM 初期化下で WIC ファクトリが作れる");
        let shell_dir = emo2_shell_dir();

        let (target, events) =
            capture_events(|| load_shell_target(&shell_dir, &dec).expect("emo2 のシェルは読める"));
        assert!(target.bake_errors().is_empty(), "前提: 脱落は 0 件");

        let summary = only_from_shell_target(&events, tracing::Level::INFO, "一覧");
        assert_eq!(summary.field("recognized"), Some("2"));
        assert_eq!(summary.field("used"), Some("0"));
        assert_eq!(summary.field("shadowed"), Some("2"));

        let shadowed: Vec<(Option<&str>, Option<&str>)> = events
            .iter()
            .filter(|e| {
                e.target == SHELL_TARGET
                    && e.level == tracing::Level::DEBUG
                    && e.message().contains("element0")
            })
            .map(|e| (e.field("surface_id"), e.field_str("file")))
            .collect();
        assert_eq!(
            shadowed,
            vec![
                (Some("0"), Some("surface0.png")),
                (Some("10"), Some("surface10.png")),
            ],
            "使わなかった画像は面 0・面 10 の 2 行"
        );

        // 重複・相手の無いコマ・脱落はどれも 0 件。上の `info!`／`debug!` が同じ走行で
        // 実在することが、この 0 件が空振りでないことの対照である。
        assert_eq!(count_from_shell_target(&events, tracing::Level::WARN), 0);
    });
}
