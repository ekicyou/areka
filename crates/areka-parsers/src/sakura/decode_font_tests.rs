//! `\f[...]`（文字装飾）解読の兄弟テスト（text-decoration-canon 要件 2.1/2.2/2.6/2.7/2.8/15.1）。
//!
//! 検証対象は「意味を読まずに転記する」ことだけである:
//! - ukadoc が定める 43 形すべてが `Instruction::Font` として受理され `Raw` へ落ちない。
//! - 引数は記述順のまま保持され、空のトークンも潰されない。
//! - 引数の形は 4 通り（`\f[]`＝0 個・裸の `\f`＝0 個・`\f[""]`＝空文字列 1 個・
//!   途中が空なら空文字列を含む列）。
//! - 別語で始まるタグ（`\foo[...]` 等）は従来どおり `Raw` へ素通しする。
//! - `\f` を含まない台本の解読結果は 1 バイトも変わらない。
//!
//! 較正（要件 15.1）: 「空のトークンを潰す」誤りと「先頭が `f` の別タグまで拾う」誤りを
//! 再現すると赤になる述語を含む（`empty_tokens_are_not_squashed`・
//! `other_words_starting_with_f_stay_raw`）。

use super::super::lexer::lex;
use super::super::model::{Choice, Instruction, NewLineRatio, SurfaceArg};
use super::decode;

/// 文字列をレックスしてデコードするヘルパ（題材は実スクリプト断片）。
fn dec(input: &str) -> Vec<Instruction> {
    decode(lex(input))
}

/// 単一命令へ解読されることを前提に、`Font` の引数列を取り出す。
///
/// `Raw` へ落ちた場合はその中身を添えて落とす（`\f` が受理されていない退行を
/// 一目で読めるようにするため）。
fn font_args(input: &str) -> Vec<String> {
    let out = dec(input);
    assert_eq!(out.len(), 1, "{input}: 単一命令へ解読されるはず: {out:?}");
    match &out[0] {
        Instruction::Font { args } => args.clone(),
        other => panic!("{input}: Font として受理されていない: {other:?}"),
    }
}

/// ukadoc `list_sakura_script` が `\f[...]` として定める 43 形（2026-09-12 に ukadoc MCP で
/// 引き直した見出しの全数）と、そのとき保持されるべき引数列。
///
/// 引数の題材は各項目の ukadoc 記述例から採っている。表そのものが「43 形」の母数であり、
/// 件数と重複の無いことは `canon_table_covers_all_43_forms` が数え上げで固定する。
const CANON_FORMS: &[(&str, &[&str])] = &[
    (r"\f[align,center]", &["align", "center"]),
    (
        r"\f[anchor.font.color,50%,90%,20%]",
        &["anchor.font.color", "50%", "90%", "20%"],
    ),
    (r"\f[anchorcolor,red]", &["anchorcolor", "red"]),
    (
        r"\f[anchorfontcolor,default.cursor]",
        &["anchorfontcolor", "default.cursor"],
    ),
    (r"\f[anchormethod,copypen]", &["anchormethod", "copypen"]),
    (
        r"\f[anchornotselectcolor,red]",
        &["anchornotselectcolor", "red"],
    ),
    (
        r"\f[anchornotselectfontcolor,default.cursor]",
        &["anchornotselectfontcolor", "default.cursor"],
    ),
    (
        r"\f[anchornotselectmethod,copypen]",
        &["anchornotselectmethod", "copypen"],
    ),
    (
        r"\f[anchornotselectpencolor,0,255,0]",
        &["anchornotselectpencolor", "0", "255", "0"],
    ),
    (
        r"\f[anchornotselectstyle,square]",
        &["anchornotselectstyle", "square"],
    ),
    (
        r"\f[anchorpencolor,0,255,0]",
        &["anchorpencolor", "0", "255", "0"],
    ),
    (r"\f[anchorstyle,square]", &["anchorstyle", "square"]),
    (
        r"\f[anchorvisitedcolor,red]",
        &["anchorvisitedcolor", "red"],
    ),
    (
        r"\f[anchorvisitedfontcolor,default.cursor]",
        &["anchorvisitedfontcolor", "default.cursor"],
    ),
    (
        r"\f[anchorvisitedmethod,copypen]",
        &["anchorvisitedmethod", "copypen"],
    ),
    (
        r"\f[anchorvisitedpencolor,0,255,0]",
        &["anchorvisitedpencolor", "0", "255", "0"],
    ),
    (
        r"\f[anchorvisitedstyle,square]",
        &["anchorvisitedstyle", "square"],
    ),
    (r"\f[bold,1]", &["bold", "1"]),
    (r"\f[color,red]", &["color", "red"]),
    (r"\f[cursorcolor,red]", &["cursorcolor", "red"]),
    (
        r"\f[cursorfontcolor,default.anchor]",
        &["cursorfontcolor", "default.anchor"],
    ),
    (r"\f[cursormethod,copypen]", &["cursormethod", "copypen"]),
    (
        r"\f[cursornotselectcolor,red]",
        &["cursornotselectcolor", "red"],
    ),
    (
        r"\f[cursornotselectfontcolor,default.anchor]",
        &["cursornotselectfontcolor", "default.anchor"],
    ),
    (
        r"\f[cursornotselectmethod,copypen]",
        &["cursornotselectmethod", "copypen"],
    ),
    (
        r"\f[cursornotselectpencolor,0,255,0]",
        &["cursornotselectpencolor", "0", "255", "0"],
    ),
    (
        r"\f[cursornotselectstyle,underline]",
        &["cursornotselectstyle", "underline"],
    ),
    (
        r"\f[cursorpencolor,0,255,0]",
        &["cursorpencolor", "0", "255", "0"],
    ),
    (r"\f[cursorstyle,underline]", &["cursorstyle", "underline"]),
    (r"\f[default]", &["default"]),
    (r"\f[disable]", &["disable"]),
    (r"\f[height,15]", &["height", "15"]),
    (r"\f[italic,1]", &["italic", "1"]),
    (
        r"\f[name,メイリオ,meiryo.ttf]",
        &["name", "メイリオ", "meiryo.ttf"],
    ),
    (r"\f[outline,1]", &["outline", "1"]),
    (r"\f[shadowcolor,#ffff00]", &["shadowcolor", "#ffff00"]),
    (r"\f[shadowcolor,none]", &["shadowcolor", "none"]),
    (r"\f[shadowstyle,outline]", &["shadowstyle", "outline"]),
    (r"\f[strike,1]", &["strike", "1"]),
    (r"\f[sub,1]", &["sub", "1"]),
    (r"\f[sup,1]", &["sup", "1"]),
    (r"\f[underline,1]", &["underline", "1"]),
    (r"\f[valign,center]", &["valign", "center"]),
];

/// 正典が「A もしくは B」と並べる 5 項目の**別綴り**（同じ受け皿へ落ちる）。
///
/// `CANON_FORMS` は各項目の先頭の綴りだけを載せているので、並べられた側もここで
/// 受理を固定する（43 形の母数は増えない——正典の見出しは 1 件だから）。
const ALTERNATE_SPELLINGS: &[(&str, &[&str])] = &[
    (r"\f[anchorbrushcolor,red]", &["anchorbrushcolor", "red"]),
    (
        r"\f[anchornotselectbrushcolor,red]",
        &["anchornotselectbrushcolor", "red"],
    ),
    (
        r"\f[anchorvisitedbrushcolor,red]",
        &["anchorvisitedbrushcolor", "red"],
    ),
    (r"\f[cursorbrushcolor,red]", &["cursorbrushcolor", "red"]),
    (
        r"\f[cursornotselectbrushcolor,red]",
        &["cursornotselectbrushcolor", "red"],
    ),
];

// ── 43 形の受理（要件 2.1/2.2）──────────────────────────────────────

/// 表の母数が正典どおり 43 形で、綴りに重複が無いことを数え上げで固定する。
///
/// キーの異なり数は 42（`shadowcolor` だけが `\f[shadowcolor,色指定]` と
/// `\f[shadowcolor,none]` の 2 見出しに現れるため）。この 2 つの零でない差を明示して
/// おかないと、表の増減が「43 形すべて」の主張を黙って壊す。
#[test]
fn canon_table_covers_all_43_forms() {
    assert_eq!(CANON_FORMS.len(), 43, "正典の `\\f[...]` 見出しは 43 形");

    let mut scripts: Vec<&str> = CANON_FORMS.iter().map(|&(s, _)| s).collect();
    scripts.sort_unstable();
    let distinct = scripts.len();
    scripts.dedup();
    assert_eq!(scripts.len(), distinct, "表に同じ綴りが 2 度載っている");

    let mut keys: Vec<&str> = CANON_FORMS.iter().map(|&(_, a)| a[0]).collect();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(
        keys.len(),
        42,
        "キーの異なり数（`shadowcolor` のみ 2 見出し）"
    );
}

/// 43 形すべてが `Instruction::Font` として受理され、引数が記述順のまま保持される。
#[test]
fn all_43_canon_forms_decode_to_font_with_args_in_order() {
    for &(script, expected) in CANON_FORMS {
        assert_eq!(font_args(script), expected, "{script}");
    }
}

/// 「A もしくは B」の別綴り 5 件も同じ受け皿へ落ちる。
#[test]
fn alternate_spellings_decode_to_font_as_well() {
    // 表が空になると以下のループは恒真で緑になるので、母数を先に固定する。
    assert_eq!(ALTERNATE_SPELLINGS.len(), 5, "別綴りの母数");
    for &(script, expected) in ALTERNATE_SPELLINGS {
        assert_eq!(font_args(script), expected, "{script}");
    }
}

// ── 引数の形 4 通り（要件 2.2/2.6）───────────────────────────────────

/// 角括弧が空のとき（`\f[]`）は引数 0 個。
#[test]
fn empty_bracket_yields_zero_args() {
    let expected: Vec<String> = Vec::new();
    assert_eq!(font_args(r"\f[]"), expected);
}

/// 引数なしの裸の `\f` も引数 0 個（字句解析で bare 形になり `decode_bare` を通る）。
///
/// `\f[]` と同じ 0 個だが、通る経路が別（`decode_tag` ではなく `decode_bare`）なので
/// 独立した断言として置く。
#[test]
fn bare_tag_yields_zero_args() {
    let expected: Vec<String> = Vec::new();
    assert_eq!(font_args(r"\f"), expected);
}

/// 引用符付きの空（`\f[""]`）だけが空文字列 1 個。
#[test]
fn quoted_empty_yields_single_empty_string() {
    assert_eq!(font_args(r#"\f[""]"#), vec![String::new()]);
}

/// 途中が空のときは空文字列を含む列になる（潰さない）。
///
/// 較正（要件 15.1）: 空のトークンを潰す実装（`filter(|s| !s.is_empty())` 等）を
/// 入れると、この 3 つの断言がいずれも赤になる。
#[test]
fn empty_tokens_are_not_squashed() {
    assert_eq!(
        font_args(r"\f[bold,]"),
        vec!["bold".to_string(), String::new()]
    );
    assert_eq!(
        font_args(r"\f[color,,default]"),
        vec!["color".to_string(), String::new(), "default".to_string()]
    );
    assert_eq!(font_args(r"\f[,1]"), vec![String::new(), "1".to_string()]);
}

// ── 別語の素通し（要件 2.8）─────────────────────────────────────────

/// 先頭が `f` でも別語の角括弧付きタグは従来どおり `Raw` へ落ちる。
///
/// 較正（要件 15.1）: 腕の判定を `word.starts_with('f')` のように広げると赤になる。
#[test]
fn other_words_starting_with_f_stay_raw() {
    for script in [r"\foo[a,b]", r"\fo[x]", r"\font[bold,1]", r"\f2[1]"] {
        assert_eq!(
            dec(script),
            vec![Instruction::Raw(script.to_string())],
            "{script}"
        );
    }
}

/// 角括弧を伴わない `\foo` は、字句解析の固定規律（`_` 以外で始まる綴りは 1 文字）に
/// より綴り `f` ＋ 本文 `oo` に割れる。本タスクはこの割り方を変えない——変わるのは
/// 綴り `f` の行き先だけ（`Raw("\\f")` → `Font { args: [] }`）。
#[test]
fn bare_f_consumes_exactly_one_character() {
    assert_eq!(
        dec(r"\fooテキスト"),
        vec![
            Instruction::Font { args: Vec::new() },
            Instruction::Text("ooテキスト".to_string()),
        ]
    );
}

// ── 並び順と、`\f` を含まない台本の不変（要件 2.2/2.8）──────────────

/// `\f` は前後の命令の並び順を保ったまま列へ入る（隣を飲み込まない）。
#[test]
fn font_keeps_neighbouring_instructions_in_order() {
    assert_eq!(
        dec(r"\0こん\f[bold,1]にちは\e"),
        vec![
            Instruction::SpeakerScope { n: 0 },
            Instruction::Text("こん".to_string()),
            Instruction::Font {
                args: vec!["bold".to_string(), "1".to_string()],
            },
            Instruction::Text("にちは".to_string()),
            Instruction::End,
        ]
    );
}

/// `\f` を含まない台本の解読結果は変わらない（要件 2.8）。
///
/// subset の代表と、`f` を含む本文・`f` を含む引数を混ぜて、腕の追加が
/// 本文やほかのタグの引数へ滲み出していないことを固定する。
#[test]
fn scripts_without_font_tag_decode_unchanged() {
    assert_eq!(
        dec(r"\p[0]\s[10]face\n[150]\q[のこり,OnRest]\_w[250]\i[f]\e"),
        vec![
            Instruction::SpeakerScope { n: 0 },
            Instruction::Surface(SurfaceArg::new("10".to_string())),
            Instruction::Text("face".to_string()),
            Instruction::NewLine(NewLineRatio::new(1.5)),
            Instruction::Choice(Choice {
                disp: "のこり".to_string(),
                target: "OnRest".to_string(),
                references: Vec::new(),
            }),
            Instruction::Wait(std::time::Duration::from_millis(250)),
            Instruction::Raw(r"\i[f]".to_string()),
            Instruction::End,
        ]
    );
}
