//! `manifest` の兄弟テスト——最上位の `install.txt` 探し・文字コード・キーの引き方・
//! 種別・`name`／`directory`／`accept`（要件 3.1〜3.11）。
//!
//! 同時インストールと再インストールの規則（要件 3.12〜3.16）は
//! [`companion`] の側に置く。1 ファイルが 1,000 行を超えると読めなくなるので
//! 固定入力を削らずに分けた。助手はこの側に置き、`companion` は `use super::*`
//! で借りる。
//!
//! 期待値の置き方は 3 つ。⑴ 受理の側は組み上がった [`InstallManifest`] の
//! **全ての欄**を突き合わせる（「受理された」ではなく「何になったか」を見る）。
//! ⑵ 拒否の側は理由の変種と中身（どのキーか・読み取った値）まで見る。
//! ⑶ 警告は**順序つきの列**そのものを突き合わせる。「含む」で測ると、
//! 同じ警告が二重に出ても 1 件落ちても気付けない。

use super::*;
use crate::container::read_central_directory;
use crate::names::validate_entry_names;
use sample_ghost_kit::{NarBuilder, install_txt};

// ---- 助手 ----

/// `install.txt` の行を組んで解釈する。受理されなければ主張ごと落とす。
pub(super) fn parsed(lines: &[&str]) -> InstallManifest {
    match parse_manifest(&install_txt(lines)) {
        Ok(manifest) => manifest,
        Err(reason) => panic!("受理されるはずの install.txt が拒否された: {reason}"),
    }
}

/// 拒否だけを取り出す。受理されたら主張ごと落とす。
pub(super) fn refusal(lines: &[&str]) -> RefuseReason {
    match parse_manifest(&install_txt(lines)) {
        Ok(manifest) => panic!("拒否されるはずの install.txt が受理された: {manifest:?}"),
        Err(reason) => reason,
    }
}

/// 種別と必須キーだけを持つ最小のゴースト。足した行の効き目だけを見たいときに使う。
///
/// `charset,UTF-8` を先頭に置くのは、この助手の固定入力がソースと同じ UTF-8 で
/// 書かれているから。宣言を落とすと既存層の ANSI 既定（Shift_JIS）で読まれて
/// 日本語の `name` が化ける。検体 emo2 が `Charset,UTF-8` を 1 行目に置くのと
/// 同じ理由で、実在の配布物もこの形になっている。
pub(super) fn ghost_with(extra: &[&str]) -> Vec<String> {
    let mut lines = vec![
        "charset,UTF-8".to_owned(),
        "type,ghost".to_owned(),
        "name,えも".to_owned(),
        "directory,emo2".to_owned(),
    ];
    lines.extend(extra.iter().map(|line| (*line).to_owned()));
    lines
}

/// [`ghost_with`] の結果をそのまま解釈へ渡す。
pub(super) fn parsed_ghost(extra: &[&str]) -> InstallManifest {
    let lines = ghost_with(extra);
    let borrowed: Vec<&str> = lines.iter().map(String::as_str).collect();
    parsed(&borrowed)
}

/// [`ghost_with`] の結果の拒否を取り出す。
pub(super) fn refused_ghost(extra: &[&str]) -> RefuseReason {
    let lines = ghost_with(extra);
    let borrowed: Vec<&str> = lines.iter().map(String::as_str).collect();
    refusal(&borrowed)
}

/// 書庫を組み、エントリ名の検証まで通してから最上位の `install.txt` を探す。
///
/// 手で [`EntryName`] を作らず本物の zip を往復させる。`components` や `is_dir` を
/// 取り違えたまま緑になることが無い。
fn located(builder: NarBuilder) -> Result<String, RefuseReason> {
    let bytes = builder.bytes();
    let entries = read_central_directory(&bytes).expect("書庫そのものは無傷");
    let names = validate_entry_names(&entries).expect("名前は安全");
    locate_install_txt(&names).map(|entry| entry.path.clone())
}

/// 最上位の一覧つきの拒否だけを取り出す。
fn missing_top_level(builder: NarBuilder) -> Vec<String> {
    match located(builder) {
        Ok(path) => panic!("最上位に無いはずの install.txt が見つかった: {path}"),
        Err(RefuseReason::MissingInstallTxt { top_level }) => top_level,
        Err(other) => panic!("最上位不在とは別の理由で拒否された: {other}"),
    }
}

// ---- 検体の実物と同じ綴り ----
//
// 実ファイルを `include_bytes!` で読まないのは、タスク 9 が展開形の置き場
// （`vendors/sample_ghost/` の下と pilot の fixtures）を消して `.nar` だけに
// するため。消える道を焼き付けると、後のタスクが本タスクのテストごと壊す。
// 代わりにバイト列を書き写し、下の較正が符号化器で写しの正しさを確かめる。

/// `crates/pilot/examples/shiori-host-32/fixtures/emo2/install.txt` の実物。
///
/// `Charset` が大文字・`name` が非 ASCII・同時インストールのバルーンつき。
const EMO2_GHOST: &[u8] = b"Charset,UTF-8\r\ntype,ghost\r\nname,\xe3\x81\x88\xe3\x82\x82\xef\xbc\x9f\xef\xbc\x9f\r\ndirectory,emo2\r\nballoon.directory,emo2-kakukaku\r\nballoon.source.directory,emo2-kakukaku\r\n";

/// 上の `name` が表しているはずの文字列。
const EMO2_GHOST_NAME: &str = "えも？？";

/// `vendors/sample_ghost/R_POST_and_KOMAINU/install.txt` の実物。
///
/// `charset` の値の前に空白があり、本文は Shift_JIS。
const R_POST: &[u8] = b"charset, Shift_JIS\r\ntype,ghost\r\nname,\x82\x71\x83\x7c\x83\x58\x83\x67\x82\xc6\x8d\x9d\x8c\xa2\r\ndirectory,R_POST_and_KOMAINU\r\n";

/// 上の `name` が表しているはずの文字列。
const R_POST_NAME: &str = "Ｒポストと狛犬";

/// `crates/pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku/install.txt` の実物。
const EMO2_BALLOON: &[u8] =
    b"charset,UTF-8\r\ntype,balloon\r\nname,kakukaku for emo2\r\ndirectory,emo2-kakukaku\r\n";

/// `vendors/pasta/crates/pasta_sample_ghost/ghosts/hello-pasta/install.txt` の実物。
///
/// `charset` 行が無く、`accept` が空。
const HELLO_PASTA: &[u8] =
    b"type,ghost\r\nname,hello-pasta\r\ndirectory,hello-pasta\r\naccept,\r\n";

/// 書き写した生バイトの較正。符号化器で組み直した値と一致することを確かめる。
///
/// 読み手（`charset::decode`）で作った期待値を使うと、読み手の取り違えごと
/// 緑になる。ここは書き手（`encoding_rs` の符号化）で測る。
#[test]
fn the_transcribed_sample_bytes_really_spell_the_sample_names() {
    let (utf8, _, had_errors) = encoding_rs::UTF_8.encode(EMO2_GHOST_NAME);
    assert!(
        !had_errors,
        "UTF-8 に符号化できない名前を期待値に置いている"
    );
    assert!(
        EMO2_GHOST
            .windows(utf8.len())
            .any(|window| window == &utf8[..]),
        "emo2 の写しに {EMO2_GHOST_NAME} の UTF-8 バイト列が入っていない"
    );

    let (sjis, _, had_errors) = encoding_rs::SHIFT_JIS.encode(R_POST_NAME);
    assert!(
        !had_errors,
        "Shift_JIS に符号化できない名前を期待値に置いている"
    );
    assert!(
        R_POST.windows(sjis.len()).any(|window| window == &sjis[..]),
        "R_POST の写しに {R_POST_NAME} の Shift_JIS バイト列が入っていない"
    );
}

/// 検体 4 つの実物が、1 件の拒否も 1 件の警告も出さずにそのまま通る（要件 3.1〜3.16）。
///
/// 通らない実物が 1 つでもあれば、この spec の出口が実在の配布物を撥ねる。
#[test]
fn every_real_sample_install_txt_is_accepted_verbatim_without_a_single_warning() {
    let emo2 = parse_manifest(EMO2_GHOST).expect("emo2 の実物が拒否された");
    assert_eq!(emo2.kind, InstallKind::Ghost);
    assert_eq!(emo2.name, EMO2_GHOST_NAME);
    assert_eq!(emo2.directory, "emo2");
    assert_eq!(emo2.charset_declared.as_deref(), Some("UTF-8"));
    assert_eq!(emo2.accept, None);
    assert_eq!(emo2.existing, ExistingPolicy::Overlay);
    assert_eq!(
        emo2.companions,
        vec![Companion {
            key: "balloon".to_owned(),
            directory: "emo2-kakukaku".to_owned(),
            source_directory: "emo2-kakukaku".to_owned(),
            existing: ExistingPolicy::Overlay,
        }]
    );
    assert_eq!(emo2.warnings, vec![]);

    let r_post = parse_manifest(R_POST).expect("R_POST_and_KOMAINU の実物が拒否された");
    assert_eq!(r_post.kind, InstallKind::Ghost);
    assert_eq!(r_post.name, R_POST_NAME);
    assert_eq!(r_post.directory, "R_POST_and_KOMAINU");
    assert_eq!(r_post.charset_declared.as_deref(), Some("Shift_JIS"));
    assert_eq!(r_post.companions, vec![]);
    assert_eq!(r_post.warnings, vec![]);

    let balloon = parse_manifest(EMO2_BALLOON).expect("emo2-kakukaku の実物が拒否された");
    assert_eq!(balloon.kind, InstallKind::Balloon);
    assert_eq!(balloon.name, "kakukaku for emo2");
    assert_eq!(balloon.directory, "emo2-kakukaku");
    assert_eq!(balloon.warnings, vec![]);

    let pasta = parse_manifest(HELLO_PASTA).expect("hello-pasta の実物が拒否された");
    assert_eq!(pasta.kind, InstallKind::Ghost);
    assert_eq!(pasta.name, "hello-pasta");
    assert_eq!(pasta.directory, "hello-pasta");
    assert_eq!(pasta.charset_declared, None);
    assert_eq!(pasta.accept, None, "空の accept は「指定なし」");
    assert_eq!(pasta.warnings, vec![]);
}

// ---- 最上位の `install.txt` 探し（要件 3.1・3.2） ----

/// 最上位のファイルとして置かれた `install.txt` を見つける（要件 3.1）。
#[test]
fn finds_install_txt_at_the_top_level() {
    let archive = NarBuilder::new()
        .file("install.txt", b"type,ghost")
        .done()
        .file("ghost/master/descript.txt", b"x")
        .done();
    assert_eq!(located(archive).expect("最上位にある"), "install.txt");
}

/// ファイル名の ASCII 大小は区別しない（Windows のファイルシステムに合わせる＝要件 3.1）。
#[test]
fn finds_install_txt_whose_name_differs_only_in_case() {
    let archive = NarBuilder::new().file("INSTALL.TXT", b"x").done();
    assert_eq!(located(archive).expect("大小は区別しない"), "INSTALL.TXT");
}

/// 深いところの `install.txt` は最上位ではない。包みフォルダを黙って剥がさない（要件 3.2）。
#[test]
fn refuses_when_install_txt_only_sits_inside_a_wrapper_folder() {
    let archive = NarBuilder::new()
        .dir("emo2/")
        .file("emo2/install.txt", b"x")
        .done();
    assert_eq!(missing_top_level(archive), vec!["emo2/".to_owned()]);
}

/// フォルダのエントリを 1 つも持たない書庫でも、包みフォルダの名前を一覧に出す（要件 3.2）。
///
/// 実在の配布物 `hello-pasta.nar` はフォルダのエントリを持たない。エントリの名前を
/// そのまま並べるだけだと一覧が空になり、「何が入っているのか」を利用者に返せない。
#[test]
fn lists_implied_top_level_folders_when_the_archive_has_no_folder_entries() {
    let archive = NarBuilder::new()
        .file("emo2/install.txt", b"x")
        .done()
        .file("emo2/ghost/master/descript.txt", b"y")
        .done()
        .file("readme.txt", b"z")
        .done();
    assert_eq!(
        missing_top_level(archive),
        vec!["emo2/".to_owned(), "readme.txt".to_owned()],
        "同じ包みフォルダは 1 度だけ・名前順で並ぶ"
    );
}

/// 何も入っていない書庫でも拒否の理由は組める。一覧は空（要件 3.2）。
#[test]
fn refuses_an_empty_archive_with_an_empty_top_level_list() {
    assert_eq!(missing_top_level(NarBuilder::new()), Vec::<String>::new());
}

/// `install.txt` という名前の**フォルダ**は読む対象ではない（要件 3.1）。
#[test]
fn does_not_take_a_top_level_folder_named_install_txt() {
    let archive = NarBuilder::new()
        .dir("install.txt/")
        .file("install.txt/inner.txt", b"x")
        .done();
    assert_eq!(missing_top_level(archive), vec!["install.txt/".to_owned()]);
}

// ---- 文字コード（要件 3.3） ----

/// `Charset` が大文字でも読む。検体 emo2 の綴り（要件 3.3）。
#[test]
fn reads_the_uppercase_charset_key_like_the_emo2_sample() {
    let manifest = parse_manifest(EMO2_GHOST).expect("受理される");
    assert_eq!(manifest.name, EMO2_GHOST_NAME, "UTF-8 として読めている");
    assert_eq!(manifest.charset_declared.as_deref(), Some("UTF-8"));
}

/// 値の前の空白は無視する。検体 R_POST_and_KOMAINU の綴り（要件 3.3）。
#[test]
fn reads_the_charset_value_with_a_leading_space_like_the_r_post_sample() {
    let manifest = parse_manifest(R_POST).expect("受理される");
    assert_eq!(manifest.name, R_POST_NAME, "Shift_JIS として読めている");
    assert_eq!(
        manifest.charset_declared.as_deref(),
        Some("Shift_JIS"),
        "値の前後の空白は落とす"
    );
}

/// `charset` 行が無ければ既存層の ANSI 既定（＝CP932）で読む（要件 3.3）。
#[test]
fn falls_back_to_the_ansi_default_when_no_charset_line_is_present() {
    let (sjis, _, _) = encoding_rs::SHIFT_JIS.encode("狛犬");
    let mut bytes = b"type,ghost\r\nname,".to_vec();
    bytes.extend_from_slice(&sjis);
    bytes.extend_from_slice(b"\r\ndirectory,komainu\r\n");

    let manifest = parse_manifest(&bytes).expect("受理される");
    assert_eq!(manifest.name, "狛犬", "宣言が無ければ Shift_JIS で読む");
    assert_eq!(manifest.charset_declared, None);
}

/// 空の `charset` 行は「宣言なし」として返す（要件 3.3）。
#[test]
fn treats_an_empty_charset_line_as_no_declaration() {
    assert_eq!(parsed_ghost(&["charset,"]).charset_declared, None);
}

/// **既知の盲点**——`charset` 行より前に非 ASCII の行があると、既存層の先読みが
/// そこで打ち切られて宣言に届かない。本文は ANSI 既定で読まれるのに、
/// `charset_declared` にはファイルに書かれた値が載る（要件 3.3 の境界）。
///
/// 検体 5 つはいずれも `charset` を 1 行目に置くのでこの形には当たらない。
/// 直すには既存層（`areka-parsers::charset`）の先読みを変えることになり、
/// 本 spec の境界の外なので、ここでは「そうなる」ことだけを固定する。
#[test]
fn a_charset_line_after_a_non_ascii_line_is_recorded_but_not_used() {
    let (sjis, _, _) = encoding_rs::SHIFT_JIS.encode("狛犬");
    let mut bytes = b"type,ghost\r\nname,".to_vec();
    bytes.extend_from_slice(&sjis);
    bytes.extend_from_slice(b"\r\ncharset,UTF-8\r\ndirectory,komainu\r\n");

    let manifest = parse_manifest(&bytes).expect("受理される");
    assert_eq!(
        manifest.charset_declared.as_deref(),
        Some("UTF-8"),
        "書かれている宣言はそのまま記録する"
    );
    assert_eq!(
        manifest.name, "狛犬",
        "実際に使われたのは ANSI 既定のほう（宣言には届いていない）"
    );
}

// ---- キーの引き方（要件 3.4） ----

/// 全てのキーを ASCII 大小を区別せずに引く（要件 3.4）。
#[test]
fn looks_up_every_key_ignoring_ascii_case() {
    let manifest = parsed(&[
        "Charset,UTF-8",
        "TYPE,ghost",
        "Name,えも",
        "DiReCtOrY,emo2",
        "ACCEPT,pasta",
        "REFRESH,1",
    ]);
    assert_eq!(manifest.kind, InstallKind::Ghost);
    assert_eq!(manifest.name, "えも");
    assert_eq!(manifest.directory, "emo2");
    assert_eq!(manifest.accept.as_deref(), Some("pasta"));
    assert_eq!(manifest.existing, ExistingPolicy::Replace { keep: vec![] });
    assert_eq!(manifest.warnings, vec![], "大小違いは知らないキーではない");
}

/// 大小だけが違う同じキーが 2 度あれば後勝ち（要件 3.4）。
///
/// 「後」は既存層の並び（キーのバイト順）で決まる。`Directory` は `directory` より
/// 前に来るので、小文字のほうが勝つ。
#[test]
fn the_later_of_two_keys_differing_only_in_case_wins() {
    let manifest = parsed(&[
        "type,ghost",
        "name,えも",
        "Directory,first",
        "directory,second",
    ]);
    assert_eq!(manifest.directory, "second");
}

/// カンマの無い行は既存層と同じく読み飛ばす。知らないキーにも数えない（要件 3.4）。
#[test]
fn skips_lines_without_a_comma() {
    let manifest = parsed(&["type,ghost", "name,えも", "directory,emo2", "これは注記"]);
    assert_eq!(manifest.warnings, vec![]);
}

/// 値の中のカンマは最初の 1 つでだけ分ける（要件 3.4）。
#[test]
fn splits_on_the_first_comma_only() {
    assert_eq!(
        parsed_ghost(&["accept,pasta,yaya"]).accept.as_deref(),
        Some("pasta,yaya")
    );
}

// ---- 種別（要件 3.5・3.6） ----

/// 4 種はそのまま受理する（要件 3.5）。
#[test]
fn accepts_the_four_supported_types() {
    for (value, expected) in [
        ("ghost", InstallKind::Ghost),
        ("shell", InstallKind::Shell),
        ("supplement", InstallKind::Supplement),
        ("balloon", InstallKind::Balloon),
    ] {
        let manifest = parsed(&[&format!("type,{value}"), "name,えも", "directory,emo2"]);
        assert_eq!(manifest.kind, expected, "type,{value} が受理されない");
    }
}

/// 扱わない種別は読み取った値つきで拒否する（要件 3.6）。
#[test]
fn refuses_every_unsupported_type_with_the_value_it_read() {
    for value in [
        "plugin",
        "headline",
        "language",
        "calendar skin",
        "calendar plugin",
        "calendar",
        "package",
        "nekomimi",
    ] {
        let reason = refusal(&[&format!("type,{value}"), "name,えも", "directory,emo2"]);
        assert_eq!(
            reason,
            RefuseReason::UnsupportedType {
                found: Some(value.to_owned())
            },
            "type,{value} の拒否の中身が違う"
        );
    }
}

/// `type` 行が無い／値が空なら「指定なし」として拒否する（要件 3.6）。
#[test]
fn refuses_a_missing_or_empty_type_as_unspecified() {
    for lines in [
        vec!["name,えも", "directory,emo2"],
        vec!["type,", "name,えも", "directory,emo2"],
    ] {
        assert_eq!(
            refusal(&lines),
            RefuseReason::UnsupportedType { found: None },
            "{lines:?} が「指定なし」にならない"
        );
    }
}

/// 値の大小は区別する。大小を無視するのはキーだけ（要件 3.4・3.6）。
#[test]
fn refuses_a_type_value_whose_case_differs() {
    assert_eq!(
        refusal(&["Type,Ghost", "name,えも", "directory,emo2"]),
        RefuseReason::UnsupportedType {
            found: Some("Ghost".to_owned())
        }
    );
}

// ---- `name`・`directory`・`accept`（要件 3.7〜3.11） ----

/// `name` が無い／空なら拒否する（要件 3.8）。
#[test]
fn refuses_a_missing_or_empty_name() {
    for lines in [
        vec!["type,ghost", "directory,emo2"],
        vec!["type,ghost", "name,", "directory,emo2"],
    ] {
        assert_eq!(
            refusal(&lines),
            RefuseReason::MissingRequiredKey { key: "name" },
            "{lines:?} が拒否されない"
        );
    }
}

/// `directory` が無い／空なら拒否する（要件 3.7）。
#[test]
fn refuses_a_missing_or_empty_directory() {
    for lines in [
        vec!["type,ghost", "name,えも"],
        vec!["type,ghost", "name,えも", "directory,"],
    ] {
        assert_eq!(
            refusal(&lines),
            RefuseReason::MissingRequiredKey { key: "directory" },
            "{lines:?} が拒否されない"
        );
    }
}

/// 検査の順序——種別・`name`・`directory` の順で、最初に当たった理由で拒否する。
///
/// 3 つとも欠けた検体を渡して「どの理由が勝つか」で測る。単独欠落の検体だけを
/// 並べても順序は証明できない。
#[test]
fn reports_the_type_then_the_name_then_the_directory() {
    assert_eq!(
        refusal(&["accept,pasta"]),
        RefuseReason::UnsupportedType { found: None },
        "種別が最初"
    );
    assert_eq!(
        refusal(&["type,ghost"]),
        RefuseReason::MissingRequiredKey { key: "name" },
        "次が name"
    );
    assert_eq!(
        refusal(&["type,ghost", "name,えも"]),
        RefuseReason::MissingRequiredKey { key: "directory" },
        "最後が directory"
    );
}

/// `name` の値をそのまま返す（要件 3.10）。前後の空白だけは既存層が落とす。
#[test]
fn returns_the_name_verbatim() {
    assert_eq!(
        parsed_ghost(&["name,  Ｒポストと狛犬  "]).name,
        "Ｒポストと狛犬"
    );
}

/// `accept` の値をそのまま返し、照合はしない（要件 3.11）。
#[test]
fn returns_accept_without_matching_it() {
    assert_eq!(
        parsed_ghost(&["accept,pasta"]).accept.as_deref(),
        Some("pasta"),
        "知らないゴースト名でも撥ねない"
    );
    assert_eq!(
        parsed_ghost(&["accept,まったく無関係な名前"])
            .accept
            .as_deref(),
        Some("まったく無関係な名前")
    );
}

/// 空の `accept` は行が無いのと同じ「指定なし」（要件 3.11・検体 hello-pasta）。
#[test]
fn treats_an_empty_accept_like_a_missing_line() {
    assert_eq!(parsed_ghost(&["accept,"]).accept, None);
    assert_eq!(parsed_ghost(&[]).accept, None);
}

/// `directory` は 1 階層の名前でなければならない（要件 3.9）。
///
/// 区切り・`..`・絶対の形・NUL・Windows で使えない名前をひと通り渡す。
#[test]
fn refuses_a_directory_that_is_not_a_one_level_name() {
    for value in [
        "a/b", "a\\b", "..", "/emo2", "C:", "C:\\emo2", "emo2\0", "CON", "emo2.", "em<o>2",
        "emo|2", ".",
    ] {
        assert_eq!(
            refused_ghost(&[&format!("directory,{value}")]),
            RefuseReason::InvalidDirectoryName {
                key: "directory".to_owned(),
                value: value.to_owned(),
            },
            "directory,{value} が拒否されない"
        );
    }
}

/// 1 階層の名前ならドットや空白を含んでいても通す（上の拒否が広すぎないことの対照）。
#[test]
fn accepts_an_ordinary_one_level_directory_name() {
    for value in ["emo2", "emo2-kakukaku", "R_POST_and_KOMAINU", "ゴースト.v2"] {
        assert_eq!(
            parsed_ghost(&[&format!("directory,{value}")]).directory,
            value,
            "directory,{value} が通らない"
        );
    }
}

// ---- 本体が書き込みの手段を持たない（要件 4.1） ----

/// `manifest` の本体に決して現れてはならない綴り。
///
/// `names` の見張りと同じ 3 語。入口は `&[EntryName]` と `&[u8]` しか受け取らず
/// 宛先を知らないので、書くとすれば `fs::`・道としての `path::`・`process::` の
/// いずれかが本体に要る。
const FORBIDDEN_SPELLINGS: &[&str] = &["fs::", "path::", "process::"];

/// `source` に現れる禁じ手の綴りを拾う。
fn forbidden_spellings_in(source: &str) -> Vec<&'static str> {
    FORBIDDEN_SPELLINGS
        .iter()
        .copied()
        .filter(|needle| source.contains(needle))
        .collect()
}

/// `manifest` の本体が、ファイルシステムにも別プロセスにも触る綴りを 1 つも持たない。
#[test]
fn manifest_source_contains_no_filesystem_mutation() {
    let found = forbidden_spellings_in(include_str!("manifest.rs"));
    assert!(
        found.is_empty(),
        "manifest が禁じ手の綴りを持っている: {found:?}"
    );
}

/// 上の走査の較正。走査語が空だと上のテストは恒真で緑になる。
#[test]
fn the_mutation_scan_catches_every_spelling_it_claims_to_watch() {
    assert_eq!(
        FORBIDDEN_SPELLINGS.len(),
        3,
        "走査語が空なら上のテストは恒真で緑になる"
    );
    for needle in FORBIDDEN_SPELLINGS {
        let synthetic = format!("fn f() {{ let _ = {needle}; }}");
        assert_eq!(
            forbidden_spellings_in(&synthetic),
            vec![*needle],
            "綴り {needle} を走査が拾えていない"
        );
    }
    assert!(
        forbidden_spellings_in("RefuseReason::InvalidDirectoryName { key, value }").is_empty(),
        "拒否語彙の変種名で赤になるなら、走査は本体を書けなくするだけで何も守らない"
    );
}

#[path = "manifest_companion_tests.rs"]
mod companion;
