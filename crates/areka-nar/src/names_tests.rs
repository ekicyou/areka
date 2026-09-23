//! `names` の兄弟テスト（要件 2.2・2.3・2.4・2.7・4.1〜4.7）。
//!
//! 固定入力は `sample_ghost_kit::NarBuilder` が組み、`container` の読み手を通して
//! `RawEntry` にしてから渡す。生バイトの名前も「名前は UTF-8」の印も外部属性も、
//! 実際に zip の欄を往復した値になるので、印を読み違えたまま緑になることが無い。
//!
//! 期待値の置き方は 3 つ。⑴ 受理の側は**復号後の文字列そのもの**を突き合わせる
//! （「復号できた」ではなく「何になったか」を見る）。⑵ 拒否の側は理由の変種と
//! 中身（どの要素が悪いか・生バイトの 16 進）まで見る。⑶ 検査の**順序**は、
//! 複数の違反を同時に持つ名前を渡して「どの理由が勝つか」で測る。単独違反の
//! 名前だけを並べても順序は証明できない。

use super::*;
use crate::container::read_central_directory;
use sample_ghost_kit::NarBuilder;

// ---- 固定入力 ----

/// Shift_JIS の「さくら/ゆめ.txt」。印なしの名前の材料。
///
/// 手で書いた値を使い、正しさは下の較正テストが符号化器で確かめる。読み手と
/// 同じ道具で作った値を期待値にすると、読み手の取り違えごと緑になってしまう。
const SJIS_NAME: &[u8] = b"\x82\xb3\x82\xad\x82\xe7/\x82\xe4\x82\xdf.txt";

/// 上の生バイトが表しているはずの文字列。
const SJIS_NAME_TEXT: &str = "さくら/ゆめ.txt";

/// Shift_JIS として復号できない名前。`0x81` は 2 バイト文字の先頭だが、
/// 続く `0x20` は後続バイトの範囲（`0x40`〜`0x7e`・`0x80`〜`0xfc`）の外。
const SJIS_BROKEN_NAME: &[u8] = b"\x81\x20.txt";

/// UTF-8 として復号できない名前。`0xff` はどの符号単位にも現れない。
///
/// `0x01` を混ぜてあるのは、16 進の桁揃えを見張るため。1 桁で書く実装だと
/// `ff` と `01` が `ff1` に潰れ、どこで区切れるのか読めなくなる。
const UTF8_BROKEN_NAME: &[u8] = b"\xff\x01.txt";

/// ファイルのエントリ 1 件だけの書庫。印あり（＝UTF-8 の名前）。
fn one_file(name: impl Into<Vec<u8>>) -> NarBuilder {
    NarBuilder::new().file(name, b"x").done()
}

/// 書庫を読み、エントリ名の検証まで通す。
fn validated(builder: NarBuilder) -> Result<Vec<EntryName>, RefuseReason> {
    let bytes = builder.bytes();
    let entries = read_central_directory(&bytes).expect("書庫そのものは無傷");
    validate_entry_names(&entries)
}

/// 受理を取り出す。拒否されたら「受理されるはず」の主張ごと落とす。
fn accepted(builder: NarBuilder) -> Vec<EntryName> {
    validated(builder).expect("安全な名前は受理される")
}

/// 拒否だけを取り出す。受理されたら「拒否されるはず」の主張ごと落とす。
fn refusal(builder: NarBuilder) -> RefuseReason {
    match validated(builder) {
        Ok(names) => {
            let paths: Vec<&str> = names.iter().map(|name| name.path.as_str()).collect();
            panic!("拒否されるはずの名前が受理された: {paths:?}")
        }
        Err(reason) => reason,
    }
}

/// 1 件だけの書庫の拒否理由。
fn refusal_of(name: impl Into<Vec<u8>>) -> RefuseReason {
    refusal(one_file(name))
}

/// `UnsafePath` の期待値を組む（先頭のエントリ）。
fn unsafe_of(name: &str, why: UnsafeWhy) -> RefuseReason {
    RefuseReason::UnsafePath {
        index: 0,
        name: name.to_owned(),
        why,
    }
}

// ---- 固定入力の較正 ----
//
// 下の 2 本が無いと「復号できない列」のつもりで置いたバイト列が実は復号できて
// しまい、別の理由で拒否されているのに気付けない。

/// `SJIS_NAME` が本当に「さくら/ゆめ.txt」の Shift_JIS 表現で、かつ UTF-8 としては
/// 読めない（＝印の有無で結果が変わる材料になっている）。
// 走査器が「この 文字列 は UTF-8 として読めない」と警告してくるが、それこそが
// ここで主張したいこと。実行時に確かめる形は残す（固定入力を差し替えたときに
// 警告が消えるだけで黙って通る、という穴を作らない）。
#[allow(invalid_from_utf8)]
#[test]
fn the_shift_jis_fixture_really_is_that_text_and_is_not_utf8() {
    let (encoded, _, had_unmappable) = encoding_rs::SHIFT_JIS.encode(SJIS_NAME_TEXT);
    assert!(!had_unmappable, "この文字列は Shift_JIS で表せる");
    assert_eq!(
        &*encoded, SJIS_NAME,
        "手で書いた生バイトが符号化器と食い違う"
    );
    assert!(
        std::str::from_utf8(SJIS_NAME).is_err(),
        "UTF-8 としても読めてしまうなら、印の有無を取り違えても緑になる"
    );
}

/// 「復号できない」つもりで置いた 2 つの列が、本当にそれぞれの文字コードで
/// 復号できない。
#[allow(invalid_from_utf8)]
#[test]
fn the_undecodable_fixtures_really_are_undecodable() {
    let (_, had_errors) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(SJIS_BROKEN_NAME);
    assert!(
        had_errors,
        "Shift_JIS として読めてしまう列では損失を測れない"
    );
    assert!(
        std::str::from_utf8(UTF8_BROKEN_NAME).is_err(),
        "UTF-8 として読めてしまう列では損失を測れない"
    );
}

// ---- 復号（要件 2.2・2.3） ----

/// 印ありの名前は UTF-8 として復号される。日本語のフォルダ名とファイル名が
/// そのまま戻る。
#[test]
fn a_flagged_name_is_decoded_as_utf8() {
    let names = accepted(one_file("ゴースト/さくら/descript.txt".as_bytes().to_vec()));

    assert_eq!(names.len(), 1);
    assert_eq!(names[0].path, "ゴースト/さくら/descript.txt");
    assert_eq!(
        names[0].components,
        vec!["ゴースト", "さくら", "descript.txt"]
    );
    assert!(!names[0].is_dir);
}

/// 印なしの名前は Shift_JIS として復号される。日本語のフォルダ名とファイル名が
/// そのまま戻る（段 ③ の検体はこの経路だけを通る）。
#[test]
fn an_unflagged_name_is_decoded_as_shift_jis() {
    let names = accepted(
        NarBuilder::new()
            .file(SJIS_NAME.to_vec(), b"x")
            .utf8_flag(false)
            .done(),
    );

    assert_eq!(names.len(), 1);
    assert_eq!(names[0].path, SJIS_NAME_TEXT);
    assert_eq!(names[0].components, vec!["さくら", "ゆめ.txt"]);
}

/// 同じ生バイトでも印が立っていれば UTF-8 として読もうとする。Shift_JIS の列は
/// UTF-8 では読めないので拒否になる＝印を見ていることの対照。
#[test]
fn the_same_bytes_are_refused_when_the_flag_claims_utf8() {
    let reason = refusal_of(SJIS_NAME.to_vec());

    assert_eq!(
        reason,
        RefuseReason::NameUndecodable {
            index: 0,
            raw_hex: "82b382ad82e72f82e482df2e747874".to_owned(),
            encoding: "UTF-8",
        }
    );
}

// ---- 復号できない名前（要件 2.4） ----

/// 印なしで Shift_JIS として読めない列は、エントリ番号と生バイトの 16 進つきで
/// 拒否する（置換文字で黙って通さない）。
#[test]
fn an_undecodable_shift_jis_name_is_refused_with_its_raw_bytes() {
    let reason = refusal(
        NarBuilder::new()
            .file(SJIS_BROKEN_NAME.to_vec(), b"x")
            .utf8_flag(false)
            .done(),
    );

    assert_eq!(
        reason,
        RefuseReason::NameUndecodable {
            index: 0,
            raw_hex: "81202e747874".to_owned(),
            encoding: "Shift_JIS",
        }
    );
}

/// 印ありで UTF-8 として読めない列も同じ形で拒否する。文字コード名だけが変わる。
#[test]
fn an_undecodable_utf8_name_is_refused_with_its_raw_bytes() {
    let reason = refusal_of(UTF8_BROKEN_NAME.to_vec());

    assert_eq!(
        reason,
        RefuseReason::NameUndecodable {
            index: 0,
            raw_hex: "ff012e747874".to_owned(),
            encoding: "UTF-8",
        }
    );
}

/// 拒否の表示にも 16 進が載る（診断はこの 1 行しか手掛かりが無い＝要件 9.1）。
#[test]
fn the_refusal_message_carries_the_raw_hex() {
    let shown = refusal_of(UTF8_BROKEN_NAME.to_vec()).to_string();

    assert!(
        shown.contains("ff012e747874"),
        "表示に生バイトの 16 進が無い: {shown}"
    );
}

/// 2 件目で復号に失敗したら、エントリ番号は 2 件目のものになる。
#[test]
fn the_refusal_names_the_offending_entry_number() {
    let reason = refusal(
        NarBuilder::new()
            .file("install.txt", b"x")
            .done()
            .file(UTF8_BROKEN_NAME.to_vec(), b"x")
            .done(),
    );

    assert!(
        matches!(reason, RefuseReason::NameUndecodable { index: 1, .. }),
        "2 件目の失敗なのに番号が違う: {reason:?}"
    );
}

// ---- 危険な名前（要件 4.2〜4.6） ----

/// 書き込みの前に撥ねるべき名前と、それぞれに対応する理由。
///
/// 1 つの名前が 1 つの違反だけを持つように選んである（順序は下の節で別に測る）。
#[test]
fn dangerous_names_are_refused_each_with_its_own_reason() {
    let cases: &[(&str, UnsafeWhy)] = &[
        // NUL（4.4）
        ("ghost/\u{0}.txt", UnsafeWhy::Nul),
        // 区切りの `\`（4.5）
        ("ghost\\master.txt", UnsafeWhy::Backslash),
        // UNC（4.2）。`\` を含むので区切りの検査が先に当たる。
        ("\\\\server\\share\\a.txt", UnsafeWhy::Backslash),
        // 絶対パス（4.2）
        ("/etc/passwd", UnsafeWhy::Absolute),
        ("//server/share/a.txt", UnsafeWhy::Absolute),
        ("C:/windows/a.txt", UnsafeWhy::Absolute),
        ("c:a.txt", UnsafeWhy::Absolute),
        // 空の要素（絶対と同じ拒否）
        ("ghost//master.txt", UnsafeWhy::Absolute),
        // 親へ遡る（4.3）
        ("../outside.txt", UnsafeWhy::DotDot),
        ("ghost/../../outside.txt", UnsafeWhy::DotDot),
        // Windows の予約名（4.6）
        (
            "ghost/CON.txt",
            UnsafeWhy::InvalidWindowsName("CON.txt".to_owned()),
        ),
        ("ghost/con", UnsafeWhy::InvalidWindowsName("con".to_owned())),
        (
            "ghost/COM1.txt",
            UnsafeWhy::InvalidWindowsName("COM1.txt".to_owned()),
        ),
        (
            "ghost/LPT9",
            UnsafeWhy::InvalidWindowsName("LPT9".to_owned()),
        ),
        (
            "ghost/nul.dat",
            UnsafeWhy::InvalidWindowsName("nul.dat".to_owned()),
        ),
        // 末尾のドット・空白（4.6）
        (
            "ghost/master.",
            UnsafeWhy::InvalidWindowsName("master.".to_owned()),
        ),
        (
            "ghost/master ",
            UnsafeWhy::InvalidWindowsName("master ".to_owned()),
        ),
        // 禁止文字（4.6）
        (
            "ghost/a<b>.txt",
            UnsafeWhy::InvalidWindowsName("a<b>.txt".to_owned()),
        ),
        (
            "ghost/a?b*c.txt",
            UnsafeWhy::InvalidWindowsName("a?b*c.txt".to_owned()),
        ),
        (
            "ghost/a|b\"c.txt",
            UnsafeWhy::InvalidWindowsName("a|b\"c.txt".to_owned()),
        ),
        // ドライブレターの形ではないコロン（4.6）
        (
            "ghost/a:b.txt",
            UnsafeWhy::InvalidWindowsName("a:b.txt".to_owned()),
        ),
        // 制御文字（4.6）
        (
            "ghost/a\u{1}b.txt",
            UnsafeWhy::InvalidWindowsName("a\u{1}b.txt".to_owned()),
        ),
    ];

    assert!(cases.len() >= 10, "危険な名前の検体が足りない");
    for (name, why) in cases {
        assert_eq!(
            refusal_of(name.as_bytes().to_vec()),
            unsafe_of(name, why.clone()),
            "名前 {name:?} の拒否理由が違う"
        );
    }
}

/// 危険な名前が 1 件でもあれば、無害なエントリが先に並んでいても書庫全体を拒否する
/// （要件 4.1。ここは判定だけを行う層なので、書き込みが起きないことは構造で担保する）。
#[test]
fn one_dangerous_name_refuses_the_whole_archive() {
    let reason = refusal(
        NarBuilder::new()
            .file("install.txt", b"x")
            .done()
            .file("ghost/master/descript.txt", b"x")
            .done()
            .file("../outside.txt", b"x")
            .done(),
    );

    assert_eq!(
        reason,
        RefuseReason::UnsafePath {
            index: 2,
            name: "../outside.txt".to_owned(),
            why: UnsafeWhy::DotDot,
        }
    );
}

// ---- 検査の順序 ----
//
// 複数の違反を同時に持つ名前を渡し、**どの理由が勝つか**で順序を測る。単独違反の
// 検体をいくら並べても、検査を入れ替えたときに赤くならない。

/// NUL → `\` → 絶対 → `..` → 空の要素 → Windows 名、の順で最初に当たった理由を返す。
#[test]
fn the_checks_run_in_a_fixed_order_and_the_first_hit_wins() {
    let cases: &[(&str, UnsafeWhy)] = &[
        // NUL は `\`・`..`・予約名より先。
        ("\u{0}..\\CON.txt", UnsafeWhy::Nul),
        // `\` は `..`・予約名より先。
        ("..\\CON.txt", UnsafeWhy::Backslash),
        // 絶対は `..` より先。
        ("C:/a/../b.txt", UnsafeWhy::Absolute),
        ("/a/../b.txt", UnsafeWhy::Absolute),
        // `..` は空の要素より先。
        ("ghost/..//b.txt", UnsafeWhy::DotDot),
        // `..` は予約名より先。
        ("ghost/../CON.txt", UnsafeWhy::DotDot),
        // 空の要素は予約名より先。
        ("ghost//CON.txt", UnsafeWhy::Absolute),
    ];

    for (name, why) in cases {
        assert_eq!(
            refusal_of(name.as_bytes().to_vec()),
            unsafe_of(name, why.clone()),
            "名前 {name:?} で勝つ理由が違う（検査の順序が入れ替わっている）"
        );
    }
}

/// 復号は安全性の検査より先。読めない名前は「どの要素が悪いか」を言えないので、
/// 先に復号の失敗として返す。
#[test]
fn decoding_runs_before_the_safety_checks() {
    // `0xff` は UTF-8 に現れない。続く `/../a` は `..` を含む。
    let reason = refusal_of(b"\xff/../a.txt".to_vec());

    assert!(
        matches!(reason, RefuseReason::NameUndecodable { .. }),
        "復号より先に安全性の検査が走っている: {reason:?}"
    );
}

// ---- 長さの上限（要件 1.1〜1.7） ----

/// 長さは復号の直後・NUL より先。NUL・`\`・`..`・予約名を同時に持つ 200 単位超の
/// 名前でも「長すぎる」が勝ち、理由は先頭 32 単位と測った長さだけを持つ。
#[test]
fn an_overlong_name_is_refused_for_its_length_before_any_other_reason() {
    let prefix = "\u{0}..\\CON.txt";
    let name = format!("{prefix}{}", "a".repeat(200));

    assert_eq!(
        refusal_of(name.as_bytes().to_vec()),
        RefuseReason::PathTooLong {
            index: 0,
            length: 211,
            limit: 200,
            head: format!("{prefix}{}", "a".repeat(21)),
        },
        "長さの検査が NUL より後ろにあるか、外れている"
    );
}

/// 1 階層の名前の判定は UTF-16 の単位で 200 まで真・201 で偽。BMP 外の文字は
/// 2 単位で数える（文字数でも UTF-8 のバイト数でもない）。
#[test]
fn a_one_level_name_is_valid_up_to_200_utf16_units() {
    assert!(is_valid_one_level_name(&"a".repeat(200)));
    assert!(!is_valid_one_level_name(&"a".repeat(201)));

    let wide = "\u{1F600}".repeat(100); // 100 文字・200 単位・400 バイト
    assert!(is_valid_one_level_name(&wide));
    assert!(!is_valid_one_level_name(&format!("{wide}a")));
}

/// 先頭は 32 単位を超えず、BMP 外の文字を途中で切らない。有界の値は上限の内側なら
/// 全体、超えていれば全体を持たず長さと上限を綴る。
#[test]
fn the_head_never_splits_a_surrogate_pair_and_bounded_values_hide_the_whole() {
    assert_eq!(
        head_utf16(&format!("{}\u{1F600}b", "a".repeat(31))),
        "a".repeat(31)
    );
    assert_eq!(
        head_utf16(&format!("{}\u{1F600}b", "a".repeat(30))),
        format!("{}\u{1F600}", "a".repeat(30))
    );

    assert_eq!(bounded_value(&"a".repeat(200)), "a".repeat(200));
    let long = format!("{}z", "a".repeat(300));
    let shown = bounded_value(&long);
    assert!(shown.starts_with(&"a".repeat(32)), "{shown}");
    assert!(shown.contains("301") && shown.contains("200"), "{shown}");
    assert!(
        !shown.contains('z') && shown.len() < 80,
        "全体が載っている: {shown}"
    );
}

// ---- シンボリックリンク（要件 2.7） ----

/// 外部属性の上位 16 bit が `S_IFLNK` のエントリは拒否する。
#[test]
fn a_symlink_entry_is_refused() {
    let reason = refusal(
        NarBuilder::new()
            .file("ghost/master/link.txt", b"../../outside")
            .symlink()
            .done(),
    );

    assert_eq!(
        reason,
        RefuseReason::SymlinkEntry {
            index: 0,
            name: "ghost/master/link.txt".to_owned(),
        }
    );
}

/// 名前が安全でもリンクなら拒否する＝中身ではなく種別で見ている。
#[test]
fn a_symlink_with_a_harmless_name_is_still_refused() {
    let reason = refusal(NarBuilder::new().file("install.txt", b"x").symlink().done());

    assert!(
        matches!(reason, RefuseReason::SymlinkEntry { index: 0, .. }),
        "無害な名前のリンクが通っている: {reason:?}"
    );
}

/// リンクの判定は種別欄だけを見る。実行ビットや所有者が立っていても普通の
/// ファイルとして受理する（要件 4.9「それ以外の外部属性は無視する」）。
#[test]
fn executable_bits_are_not_mistaken_for_a_symlink() {
    // `S_IFREG | 0755` を上位 16 bit に置いた値。
    let names = accepted(
        NarBuilder::new()
            .file("ghost/master/run.sh", b"x")
            .external_attrs(0x81ED_0000)
            .done(),
    );

    assert_eq!(names[0].path, "ghost/master/run.sh");
}

/// 名前の安全性の検査はリンクの判定より先に当たる（順序の主張）。
#[test]
fn an_unsafe_name_wins_over_the_symlink_check() {
    let reason = refusal(NarBuilder::new().file("../outside", b"x").symlink().done());

    assert_eq!(
        reason,
        unsafe_of("../outside", UnsafeWhy::DotDot),
        "リンクの判定が名前の検査より先に当たっている"
    );
}

// ---- 大文字小文字の衝突（要件 4.7） ----

/// 大小だけが違う 2 つの名前は、Windows では同じファイルに書かれるので拒否する。
#[test]
fn names_differing_only_in_case_collide() {
    let reason = refusal(
        NarBuilder::new()
            .file("ghost/master/A.txt", b"x")
            .done()
            .file("ghost/master/a.txt", b"y")
            .done(),
    );

    assert_eq!(
        reason,
        RefuseReason::CaseCollision {
            a: "ghost/master/A.txt".to_owned(),
            b: "ghost/master/a.txt".to_owned(),
        }
    );
}

/// 同じ名前が 2 回あるときも同じ変種で拒否する（後勝ちで一方が消えるのは同じ）。
#[test]
fn an_exact_duplicate_collides_too() {
    let reason = refusal(
        NarBuilder::new()
            .file("a.txt", b"x")
            .done()
            .file("a.txt", b"y")
            .done(),
    );

    assert_eq!(
        reason,
        RefuseReason::CaseCollision {
            a: "a.txt".to_owned(),
            b: "a.txt".to_owned(),
        }
    );
}

/// フォルダのエントリ `a/` とファイルのエントリ `a` も衝突する。末尾の `/` を
/// 落とした後の宛先が同じで、同じ場所にフォルダとファイルの両方は作れない。
#[test]
fn a_directory_and_a_file_with_the_same_name_collide() {
    let reason = refusal(NarBuilder::new().dir("ghost").file("ghost", b"x").done());

    assert_eq!(
        reason,
        RefuseReason::CaseCollision {
            a: "ghost".to_owned(),
            b: "ghost".to_owned(),
        }
    );
}

/// 大小の畳み込みは Unicode の規則で行う（ASCII だけではない）。
#[test]
fn the_case_fold_is_unicode_not_ascii_only() {
    let reason = refusal(
        NarBuilder::new()
            .file("ＡＢＣ.txt", b"x")
            .done()
            .file("ａｂｃ.txt", b"y")
            .done(),
    );

    assert!(
        matches!(reason, RefuseReason::CaseCollision { .. }),
        "全角の大小が畳み込まれていない: {reason:?}"
    );
}

/// 違う場所にある同じ名前は衝突しない（畳み込むのは完全なパス）。
#[test]
fn the_same_leaf_in_different_folders_does_not_collide() {
    let names = accepted(
        NarBuilder::new()
            .file("ghost/master/descript.txt", b"x")
            .done()
            .file("shell/master/descript.txt", b"y")
            .done(),
    );

    assert_eq!(names.len(), 2);
}

// ---- 受理される名前 ----

/// 予約名の境界。`CON` は予約だが `CONSOLE` は違う。番号は 1〜9 だけで `0` は
/// 予約ではない。片側だけ試すと、広すぎる規則も狭すぎる規則も通ってしまう。
#[test]
fn only_the_listed_reserved_words_are_refused() {
    let accepted_names = [
        "ghost/CONSOLE.txt",
        "ghost/COM0.txt",
        "ghost/LPT0.txt",
        "ghost/COM10.txt",
        "ghost/AUXILIARY.dat",
        "ghost/PRNS",
        "ghost/CONTENT/a.txt",
    ];
    for name in accepted_names {
        let names = accepted(one_file(name.as_bytes().to_vec()));
        assert_eq!(names[0].path, name, "予約名でないのに撥ねられた: {name}");
    }

    let refused_names = ["ghost/PRN", "ghost/aux.txt", "ghost/Com9.dat", "ghost/LPT1"];
    for name in refused_names {
        let reason = refusal_of(name.as_bytes().to_vec());
        assert!(
            matches!(
                reason,
                RefuseReason::UnsafePath {
                    why: UnsafeWhy::InvalidWindowsName(_),
                    ..
                }
            ),
            "予約名なのに通った: {name}（{reason:?}）"
        );
    }
}

/// ドットを含む普通の名前は通る。先頭のドットも中のドットも禁じていない。
#[test]
fn ordinary_names_with_dots_are_accepted() {
    for name in [
        "ghost/a.b.txt",
        "ghost/.hidden",
        "ghost/..hidden",
        "ghost/a",
    ] {
        let names = accepted(one_file(name.as_bytes().to_vec()));
        assert_eq!(names[0].path, name);
    }
}

/// フォルダのエントリは末尾の `/` を落とした形になり、`is_dir` が立つ。
#[test]
fn a_directory_entry_is_normalised_without_its_trailing_slash() {
    let names = accepted(NarBuilder::new().dir("ghost/master"));

    assert_eq!(names.len(), 1);
    assert_eq!(names[0].path, "ghost/master");
    assert_eq!(names[0].components, vec!["ghost", "master"]);
    assert!(names[0].is_dir);
    assert_eq!(names[0].index, 0);
}

/// 番号は中央ディレクトリの並び順のまま 0 始まりで振られる。
#[test]
fn the_index_follows_the_central_directory_order() {
    let names = accepted(
        NarBuilder::new()
            .file("install.txt", b"x")
            .done()
            .dir("ghost")
            .file("ghost/master/descript.txt", b"y")
            .done(),
    );

    let observed: Vec<(usize, &str)> = names
        .iter()
        .map(|name| (name.index, name.path.as_str()))
        .collect();
    assert_eq!(
        observed,
        vec![
            (0, "install.txt"),
            (1, "ghost"),
            (2, "ghost/master/descript.txt"),
        ]
    );
}

// ---- 「1 バイトも書かない」（要件 4.1） ----

/// `names` の本体に決して現れてはならない綴り。
///
/// `container` の見張りと同じ形だが、走査語が 1 つ違う。`container` は `Path` を
/// 禁じていて、こちらは拒否語彙に `UnsafePath` が出てくるのでその語では測れない。
/// 代わりに `path` の**道の綴り**を見る。`&Path` を書くには `use` でも直書きでも
/// `path::` が要るので、道具立てとしては同じ強さになる。
const FORBIDDEN_SPELLINGS: &[&str] = &["fs::", "path::", "process::"];

/// `source` に現れる禁じ手の綴りを拾う。
fn forbidden_spellings_in(source: &str) -> Vec<&'static str> {
    FORBIDDEN_SPELLINGS
        .iter()
        .copied()
        .filter(|needle| source.contains(needle))
        .collect()
}

/// `names` の本体が、ファイルシステムにも別プロセスにも触る綴りを 1 つも持たない。
///
/// 「観測しなかった」ではなく「触る手段を持たない」を主張する。入口は `&[RawEntry]`
/// しか受け取らず宛先を知らないので、書くとすれば `fs::`・道としての `path::`・
/// `process::` のいずれかが本体に要る。
#[test]
fn names_source_contains_no_filesystem_mutation() {
    let found = forbidden_spellings_in(include_str!("names.rs"));
    assert!(
        found.is_empty(),
        "names が禁じ手の綴りを持っている: {found:?}"
    );
}

/// 上の走査の較正。走査語が空でないこと、各語を足した写しが必ず赤になること、
/// 拒否語彙の `UnsafePath` では赤にならないことを、この 1 本で確かめる。
///
/// **最初の主張が要る**——走査語が空だと、上のテストは `found.is_empty()` で緑、
/// 下の繰り返しは 0 周で緑になり、二本とも恒真のまま「見張っている」ように見える。
#[test]
fn the_mutation_scan_catches_every_spelling_it_claims_to_watch() {
    assert_eq!(
        FORBIDDEN_SPELLINGS.len(),
        3,
        "走査語が空なら二本とも恒真で緑になる"
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
        forbidden_spellings_in("RefuseReason::UnsafePath { index, name, why }").is_empty(),
        "拒否語彙の変種名で赤になるなら、走査は本体を書けなくするだけで何も守らない"
    );
}
