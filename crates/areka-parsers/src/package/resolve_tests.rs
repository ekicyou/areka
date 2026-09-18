//! resolve_tests — `package::resolve` 正常系（happy path）＋失敗系マトリクスの証明テスト。
//!
//! 正常系（3.1）に加え、失敗系（3.2）の 3 経路
//! （`StartPointMissing` / `StartPointUnreadable` / `ShellDirMissing`）を検証する。
//! この 3 経路が **区別できる**（それぞれ固有の variant を返す）ことこそが本題で、
//! `sakura` の `Result` 無し寛容パースとは意図的に非対称である
//! （マウントは物理不在という現実の失敗を持つ・Req 5.1、詳細は `mod.rs` 冒頭）。
//!
//! 一時ツリーは共通窓口 `temp-path-kit` 経由で組む。名前にプロセス識別子と連番が
//! 入るので**プロセス間でも一意**で、破棄で中身ごと消える。区切り文字は `Path::join`
//! に委ね、クロスプラットフォームで決定的に振る舞う。

use std::fs;

use temp_path_kit::TempPath;

use crate::charset::DefaultEncoding;

use super::{MountError, MountModel, resolve};

/// このテスト専用の一時ディレクトリを返す（共通窓口 `temp-path-kit` 経由）。
///
/// 名前にプロセス識別子と連番が入るので**プロセス間でも一意**になる。返り値が生き
/// ている間だけ実体が存在し、破棄で中身ごと消える（後始末を呼び忘れる余地が無い）。
fn unique_temp_dir(tag: &str) -> TempPath {
    // 札は `-` と英数字だけ（窓口の約束）。呼出側の tag は関数名由来で `_` を含む。
    TempPath::new(&format!(
        "parsers-package-resolve-{}",
        tag.replace('_', "-")
    ))
}

#[test]
fn resolve_happy_path_builds_mount_model() {
    // --- Arrange: 正常なゴーストツリーを一時ディレクトリに構築 ---
    let temp = unique_temp_dir("happy_path_builds_mount_model");
    let root = temp.path().to_path_buf();

    let ghost_master = root.join("ghost").join("master");
    fs::create_dir_all(&ghost_master).expect("create ghost/master");

    // seriko.defaultsurfacedirectoryname を明示指定し、shell/<名> を実在させる。
    let shell_name = "master";
    let shell_dir = root.join("shell").join(shell_name);
    fs::create_dir_all(&shell_dir).expect("create shell/<name>");

    let descript = ghost_master.join("descript.txt");
    let contents = "charset,UTF-8\n\
         type,ghost\n\
         name,テスト\n\
         sakura.name,さくら\n\
         kero.name,けろ\n\
         shiori,pasta.dll\n\
         seriko.defaultsurfacedirectoryname,master\n";
    fs::write(&descript, contents.as_bytes()).expect("write descript.txt");

    // --- Act ---
    let result = resolve(&root, DefaultEncoding::Utf8);

    // --- Assert ---
    let model = result.expect("正常ツリーは Ok(MountModel) を返す");

    // SHIORI マウント: dir = root/ghost/master、file = Some("pasta.dll")（推測なし）。
    assert_eq!(model.shiori.dir, ghost_master);
    assert_eq!(model.shiori.file, Some("pasta.dll".to_string()));

    // shell マウント: dir = root/shell/master（存在確認済み）。
    assert_eq!(model.shell.dir, shell_dir);
    assert!(model.shell.dir.is_dir(), "解決した shell dir は実在する");

    // 名前情報（欠落なし）。
    assert_eq!(model.names.name, Some("テスト".to_string()));
    assert_eq!(model.names.sakura_name, Some("さくら".to_string()));
    assert_eq!(model.names.kero_name, Some("けろ".to_string()));
}

// ---------------------------------------------------------------------------
// 失敗系マトリクス（3.2）— 3 経路が区別できることを証明する。
//
// `sakura` は `Result` を持たず寛容パースだが、`resolve` は物理不在という
// 現実の致命失敗を観測可能な `Err` として早期 return する（意図的な非対称・
// Req 1.6/3.3/5.1、根拠は `mod.rs` 冒頭のモジュールコメント）。各テストは
// 該当する **固有の** variant を assert し、3 経路の識別可能性を担保する。
// ---------------------------------------------------------------------------

/// 起点 `ghost/master/descript.txt` が不在 → `StartPointMissing`。
///
/// descript が存在しないと `std::fs::read` は `NotFound` を返し、これは
/// 読取失敗（`StartPointUnreadable`）とは区別されねばならない。
#[test]
fn resolve_missing_descript_yields_start_point_missing() {
    // --- Arrange: descript.txt を含まないツリー（ghost/master ディレクトリすら作らない）---
    let temp = unique_temp_dir("missing_descript_yields_start_point_missing");
    let root = temp.path().to_path_buf();
    fs::create_dir_all(&root).expect("create root");

    // --- Act ---
    let result = resolve(&root, DefaultEncoding::Utf8);

    // --- Assert: 固有 variant（StartPointMissing）を返す ---
    match result {
        Err(MountError::StartPointMissing { expected }) => {
            // expected は起点 descript.txt のパスを指す（黙って空を返さない）。
            assert_eq!(
                expected,
                root.join("ghost").join("master").join("descript.txt")
            );
        }
        other => panic!("StartPointMissing を期待したが {other:?} が返った"),
    }
}

/// 起点 descript.txt は所在するが読取に失敗 → `StartPointUnreadable`。
///
/// `ghost/master/descript.txt` を **ディレクトリ** として作ると、`std::fs::read`
/// は Windows / Unix いずれでも I/O エラー（`NotFound` 以外）で失敗する。これにより
/// 「所在するが読めない」経路を権限トリック無しでクロスプラットフォームに再現でき、
/// `StartPointMissing`（不在）とは区別される。
#[test]
fn resolve_unreadable_descript_yields_start_point_unreadable() {
    // --- Arrange: descript.txt を「ディレクトリ」として作る（read が失敗する）---
    let temp = unique_temp_dir("unreadable_descript_yields_start_point_unreadable");
    let root = temp.path().to_path_buf();
    let descript_as_dir = root.join("ghost").join("master").join("descript.txt");
    fs::create_dir_all(&descript_as_dir).expect("create descript.txt as a directory");

    // --- Act ---
    let result = resolve(&root, DefaultEncoding::Utf8);

    // --- Assert: 固有 variant（StartPointUnreadable）を返す ---
    match result {
        Err(MountError::StartPointUnreadable { path, kind }) => {
            assert_eq!(path, descript_as_dir);
            // ディレクトリ読取は NotFound ではない I/O エラー（不在と区別される）。
            assert_ne!(kind, std::io::ErrorKind::NotFound);
        }
        other => panic!("StartPointUnreadable を期待したが {other:?} が返った"),
    }
}

/// 解決した shell ディレクトリが不在 → `ShellDirMissing`。
///
/// 起点 descript.txt は正常な file として存在し `seriko.defaultsurfacedirectoryname`
/// で `nope` を指すが、`shell/nope` を実在させないため shell 存在確認で失敗する。
/// 起点系の失敗（不在・読取不能）とは区別される。
#[test]
fn resolve_missing_shell_dir_yields_shell_dir_missing() {
    // --- Arrange: 正常な descript.txt（file）だが shell/<名> は作らない ---
    let temp = unique_temp_dir("missing_shell_dir_yields_shell_dir_missing");
    let root = temp.path().to_path_buf();

    let ghost_master = root.join("ghost").join("master");
    fs::create_dir_all(&ghost_master).expect("create ghost/master");

    let descript = ghost_master.join("descript.txt");
    fs::write(
        &descript,
        b"charset,UTF-8\nseriko.defaultsurfacedirectoryname,nope\n",
    )
    .expect("write descript.txt");
    // shell/nope はあえて作らない。

    // --- Act ---
    let result = resolve(&root, DefaultEncoding::Utf8);

    // --- Assert: 固有 variant（ShellDirMissing）を返す ---
    match result {
        Err(MountError::ShellDirMissing { expected }) => {
            assert_eq!(expected, root.join("shell").join("nope"));
        }
        other => panic!("ShellDirMissing を期待したが {other:?} が返った"),
    }
}

// ---------------------------------------------------------------------------
// 正常・境界系マトリクス（4.1）— 「推測しない」寛容受理を証明する。
//
// 失敗系 3 経路（不在・読取不能・shell 不在）は上記 3.2 で検証済み。ここでは
// 起点が正常に読める場合の 3 つの境界を assert する：`shiori` 未指定なら
// 既定へ推測せず `None`（Req 2.3）／`seriko.defaultsurfacedirectoryname`
// 未指定なら既定 `master` へフォールバック（Req 3.1）／`type` 欠落でも所在
// ベースで受理し `Ok`（Req 1.2/1.3）。これで 6 シナリオ全マトリクスが揃う。
// ---------------------------------------------------------------------------

/// `shiori` 未指定 → `shiori.file == None`（既定 `"shiori.dll"` へ推測しない・Req 2.3）。
///
/// descript に `shiori,` 行を含めず、shell dir は実在させる。`resolve` は `Ok` を
/// 返し、SHIORI ファイル名は **推測されず** `None` であることを assert する。
#[test]
fn resolve_missing_shiori_yields_file_none() {
    // --- Arrange: shiori 行を持たない正常な descript.txt ＋ 実在 shell dir ---
    let temp = unique_temp_dir("missing_shiori_yields_file_none");
    let root = temp.path().to_path_buf();

    let ghost_master = root.join("ghost").join("master");
    fs::create_dir_all(&ghost_master).expect("create ghost/master");

    let shell_dir = root.join("shell").join("master");
    fs::create_dir_all(&shell_dir).expect("create shell/master");

    let descript = ghost_master.join("descript.txt");
    // shiori 行を **あえて含めない**。
    fs::write(
        &descript,
        "charset,UTF-8\n\
         type,ghost\n\
         name,テスト\n\
         seriko.defaultsurfacedirectoryname,master\n"
            .as_bytes(),
    )
    .expect("write descript.txt");

    // --- Act ---
    let result = resolve(&root, DefaultEncoding::Utf8);

    // --- Assert: Ok かつ shiori.file は推測されず None ---
    let model = result.expect("shiori 未指定でも Ok（マウント点は所在で決まる）");
    assert!(
        model.shiori.file.is_none(),
        "shiori 未指定は None であるべき（\"shiori.dll\" 等へ推測しない）: {:?}",
        model.shiori.file
    );
    // dir 自体は起点の親として確定している。
    assert_eq!(model.shiori.dir, ghost_master);
}

/// `seriko.defaultsurfacedirectoryname` 未指定 かつ `shell/master` 実在
/// → 既定 `master` フォールバック（Req 3.1）。
///
/// shell 名の指定行を含めず、`shell/master` を実在させる。`resolve` は既定
/// `master` へフォールバックし、`model.shell.dir == root/shell/master` を assert する。
#[test]
fn resolve_missing_shell_name_falls_back_to_master() {
    // --- Arrange: shell 名指定なし ＋ 実在する shell/master ---
    let temp = unique_temp_dir("missing_shell_name_falls_back_to_master");
    let root = temp.path().to_path_buf();

    let ghost_master = root.join("ghost").join("master");
    fs::create_dir_all(&ghost_master).expect("create ghost/master");

    let shell_master = root.join("shell").join("master");
    fs::create_dir_all(&shell_master).expect("create shell/master");

    let descript = ghost_master.join("descript.txt");
    // seriko.defaultsurfacedirectoryname を **あえて含めない**。
    fs::write(&descript, b"charset,UTF-8\ntype,ghost\nname,fallback\n")
        .expect("write descript.txt");

    // --- Act ---
    let result = resolve(&root, DefaultEncoding::Utf8);

    // --- Assert: Ok かつ既定 master へフォールバック ---
    let model = result.expect("shell 名未指定でも実在 master へフォールバックし Ok");
    assert_eq!(
        model.shell.dir, shell_master,
        "shell 名未指定は既定 master へフォールバックする"
    );
    assert!(
        model.shell.dir.is_dir(),
        "フォールバックした shell dir は実在する"
    );
}

/// `type,ghost` 欠落でも所在ベースで受理 → `Ok`（Req 1.2/1.3）。
///
/// descript に `type` 行を一切含めず（`charset` と `name` のみ）、shell dir は実在
/// させる。`type` 欠落は失敗ではなく、マウントは物理所在で識別されるため `resolve`
/// は `Ok` を返すことを assert する。
#[test]
fn resolve_missing_type_is_accepted() {
    // --- Arrange: type 行を持たない descript.txt ＋ 実在 shell dir ---
    let temp = unique_temp_dir("missing_type_is_accepted");
    let root = temp.path().to_path_buf();

    let ghost_master = root.join("ghost").join("master");
    fs::create_dir_all(&ghost_master).expect("create ghost/master");

    let shell_dir = root.join("shell").join("master");
    fs::create_dir_all(&shell_dir).expect("create shell/master");

    let descript = ghost_master.join("descript.txt");
    // type 行を **あえて含めない**（charset と name のみ）。
    fs::write(
        &descript,
        "charset,UTF-8\n\
         name,no-type\n\
         seriko.defaultsurfacedirectoryname,master\n"
            .as_bytes(),
    )
    .expect("write descript.txt");

    // --- Act ---
    let result = resolve(&root, DefaultEncoding::Utf8);

    // --- Assert: type 欠落は失敗ではない（所在ベース識別）→ Ok ---
    let model = result.expect("type 欠落でも所在ベースで受理し Ok を返す");
    // 名前情報は読めており、マウントも所在で確定する。
    assert_eq!(model.names.name, Some("no-type".to_string()));
    assert_eq!(model.shell.dir, shell_dir);
}

// ---------------------------------------------------------------------------
// descript の文字コード 2 キーの転記（2.5 / 8.5・areka-P0-charset-canon 3.1）
//
// `shiori.encoding` / `shiori.forceencoding` は **生ラベル**として転記するだけで、
// この層は解決も整形もしない（ラベルの解釈は通信層の `Charset::for_label` が行う）。
// ゆえにここでの主張は「`parse_kv` が返した値と 1 バイト違わない」ことであり、
// 綴りの正規化（大小文字・別名）や値の切り詰めが起きないことを固定する。
// ---------------------------------------------------------------------------

/// 2 キーを持つ最小のゴーストツリーを組み、`resolve` の結果を返す。
///
/// 2 キーの転記だけを見るテスト群の共通の組み立て。転記そのものを検査するので、
/// `ShioriMount` を struct リテラルで組まず必ず `resolve` を通す。
fn resolve_with_descript(tag: &str, descript_body: &str) -> (TempPath, MountModel) {
    let temp = unique_temp_dir(tag);
    let root = temp.path().to_path_buf();

    let ghost_master = root.join("ghost").join("master");
    fs::create_dir_all(&ghost_master).expect("create ghost/master");
    fs::create_dir_all(root.join("shell").join("master")).expect("create shell/master");

    fs::write(ghost_master.join("descript.txt"), descript_body.as_bytes())
        .expect("write descript.txt");

    let model = resolve(&root, DefaultEncoding::Utf8).expect("正常ツリーは Ok(MountModel)");
    (temp, model)
}

/// 2 キーとも無い descript では両フィールドが `None`（推測しない・2.5）。
#[test]
fn shiori_encoding_keys_absent_are_none() {
    let (_temp, model) = resolve_with_descript(
        "encoding_keys_absent",
        "charset,UTF-8\n\
         name,テスト\n\
         shiori,pasta.dll\n\
         seriko.defaultsurfacedirectoryname,master\n",
    );

    assert_eq!(
        model.shiori.encoding, None,
        "shiori.encoding は未宣言なら None"
    );
    assert_eq!(
        model.shiori.force_encoding, None,
        "shiori.forceencoding は未宣言なら None"
    );
}

/// 2 キーが在るときは生ラベルをそのまま持つ（解決も大小文字の正規化もしない・2.5）。
///
/// `sHiFt_jis` と `euc-JP` はどちらも `Charset::for_label` が解決できる綴りだが、
/// **この層は解決しない**ので綴りは 1 文字も変わらない。ここで正規名（`Shift_JIS`）
/// が返ってきたら転記層が意味を持ってしまっている証拠になる。
#[test]
fn shiori_encoding_keys_are_transcribed_verbatim() {
    let (_temp, model) = resolve_with_descript(
        "encoding_keys_verbatim",
        "charset,UTF-8\n\
         name,テスト\n\
         shiori,pasta.dll\n\
         shiori.encoding,sHiFt_jis\n\
         shiori.forceencoding,euc-JP\n\
         seriko.defaultsurfacedirectoryname,master\n",
    );

    assert_eq!(model.shiori.encoding.as_deref(), Some("sHiFt_jis"));
    assert_eq!(model.shiori.force_encoding.as_deref(), Some("euc-JP"));
}

/// 値の中の空白とカンマが落ちない（転記層は `parse_kv` の値へ手を加えない・2.5）。
///
/// `parse_kv` は行全体の最初のカンマ 1 個だけで分割し、値の**前後**空白のみを落とす
/// （kv 層 R4.1/R4.4）。よって値の中に現れる空白・カンマは生き残る。転記層が独自に
/// trim・分割・整形を足していれば `Shift_JIS ,x` は縮んでこのテストが赤くなる。
#[test]
fn shiori_encoding_value_keeps_inner_whitespace_and_comma() {
    let (_temp, model) = resolve_with_descript(
        "encoding_value_raw",
        "charset,UTF-8\n\
         name,テスト\n\
         shiori,pasta.dll\n\
         shiori.encoding,  Shift_JIS ,x  \n\
         shiori.forceencoding,\tEUC-JP\t\n\
         seriko.defaultsurfacedirectoryname,master\n",
    );

    // 前後の空白は kv 層（R4.4）が落とし、値の中の空白とカンマはそのまま残る。
    assert_eq!(model.shiori.encoding.as_deref(), Some("Shift_JIS ,x"));
    assert_eq!(model.shiori.force_encoding.as_deref(), Some("EUC-JP"));
}

/// 片方だけの宣言も独立に転記される（`shiori.encoding` と `shiori.forceencoding`
/// を取り違えていないことの固定・2.5）。
#[test]
fn shiori_force_encoding_alone_does_not_fill_encoding() {
    let (_temp, model) = resolve_with_descript(
        "force_encoding_alone",
        "charset,UTF-8\n\
         name,テスト\n\
         shiori,pasta.dll\n\
         shiori.forceencoding,ISO-2022-JP\n\
         seriko.defaultsurfacedirectoryname,master\n",
    );

    assert_eq!(model.shiori.encoding, None);
    assert_eq!(model.shiori.force_encoding.as_deref(), Some("ISO-2022-JP"));
}

/// descript の `charset` キー（ファイル自身の文字コード）は SHIORI 通信の初期値へ
/// 流れ込まない（2.5 の「用いる箇所 0」をこの層で固定する）。
///
/// `charset,Shift_JIS` を宣言しつつ 2 キーを書かない descript で、両フィールドが
/// `None` のままであることを見る。`charset` を初期値の供給源にする実装なら
/// `Some("Shift_JIS")` が現れて赤くなる。
#[test]
fn descript_charset_key_does_not_feed_shiori_encoding() {
    let temp = unique_temp_dir("charset_key_not_shiori_encoding");
    let root = temp.path().to_path_buf();

    let ghost_master = root.join("ghost").join("master");
    fs::create_dir_all(&ghost_master).expect("create ghost/master");
    fs::create_dir_all(root.join("shell").join("master")).expect("create shell/master");

    // Shift_JIS 宣言つきの descript を **Shift_JIS のバイト列**で書く。
    let body = "charset,Shift_JIS\n\
                name,テスト\n\
                shiori,pasta.dll\n\
                seriko.defaultsurfacedirectoryname,master\n";
    let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode(body);
    fs::write(ghost_master.join("descript.txt"), &bytes[..]).expect("write descript.txt");

    let model = resolve(&root, DefaultEncoding::Utf8).expect("正常ツリーは Ok(MountModel)");

    // ファイル自身の文字コード宣言は読めている（デコードが効いている裏づけ）。
    assert_eq!(model.names.name.as_deref(), Some("テスト"));
    // が、SHIORI 通信の 2 キーへは流れ込まない。
    assert_eq!(model.shiori.encoding, None);
    assert_eq!(model.shiori.force_encoding, None);
}

// ---------------------------------------------------------------------------
// readme キーの転記（popup-menu-minimal task 1.3・要件 4.1/9.4/10.4）
//
// `readme,<ファイル名>` は他の転記キーと同じ 1 行の転記のみ（存在確認も既定値の
// 補いもしない・resolve 層の原則）。
// ---------------------------------------------------------------------------

/// `readme,ファイル名` が在るときはそのまま転記する（推測しない・4.1/9.4）。
#[test]
fn readme_key_present_is_transcribed() {
    let (_temp, model) = resolve_with_descript(
        "readme_key_present",
        "charset,UTF-8\n\
         name,テスト\n\
         shiori,pasta.dll\n\
         readme,manual.txt\n\
         seriko.defaultsurfacedirectoryname,master\n",
    );

    assert_eq!(model.readme.as_deref(), Some("manual.txt"));
}

/// `readme` キーが無いときは `None`（存在確認も既定値の補いもしない・4.1/9.4）。
#[test]
fn readme_key_absent_is_none() {
    let (_temp, model) = resolve_with_descript(
        "readme_key_absent",
        "charset,UTF-8\n\
         name,テスト\n\
         shiori,pasta.dll\n\
         seriko.defaultsurfacedirectoryname,master\n",
    );

    assert_eq!(model.readme, None);
}
