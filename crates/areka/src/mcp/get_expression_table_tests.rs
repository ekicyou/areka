//! `get_expression_table` の決定論テスト。
//!
//! 規則（`render`）・読み取りと記録（`load`）・配線（`handle`）。
//! 空の World で `handle` を呼ぶと、シェルのフォルダが引けないので `warn!` を 1 件出し、
//! 既定の 15 件の全文を素の値（isError: false）で答える（要件 1.7・3.5・5.9）。

use std::path::PathBuf;

use areka_mcp::ToolContent;
use areka_mcp::tools::{ToolCall, ToolRequest};

use super::*;

#[test]
fn an_empty_world_answers_the_15_defaults_with_one_warning() {
    let ghost = ActiveGhost {
        name: Some("Emily/Phase4.5".to_string()),
        sakura_name: None,
        root: PathBuf::from(r"C:\ssp\ghost\emily4"),
    };
    let args = Args {
        ghost_name: Some("Emily/Phase4.5".to_string()),
    };
    let (req, pending) = ToolRequest::new(ToolCall::GetExpressionTable(args.clone()));

    let ((), events) = capture(|| handle(&mut World::new(), &ghost, args, req.reply));

    let answer = pending.try_answer().ok().flatten().expect("その場で答える");
    assert_eq!(
        answer.outcome.content,
        vec![ToolContent::Text(crlf(DEFAULTS_ONLY_ANSWER))]
    );
    assert!(!answer.outcome.is_error);
    let warns = warns(&events);
    assert_eq!(warns.len(), 1, "{events:?}");
    assert!(
        warns[0].message().starts_with("[get_expression_table]"),
        "{events:?}"
    );
}

// ---- 規則（render(&parse_surfacetable(..))・期待値は ssp-measurements.md の写し） ----

/// 期待値の各行を `\r\n` で終えて 1 本にする（行の終わりは survey.md の `\r\n`・要件 1.3）。
fn crlf(lines: &str) -> String {
    lines.lines().map(|line| format!("{line}\r\n")).collect()
}

/// SSP の実測 14 の検体（ssp-measurements.md の検体 1・字下げはタブ）。
const SAMPLE_1: &str = "charset,UTF-8
version,1


group,0
{
\tscope,0

\t0,素
\t1,照れ
\t2,驚き
\t3,不安
\t4,落胆
\t5,笑顔
\t6,静観
\t7,怒り
\t8,冷笑
\t9,照れ怒り
\t20,ぴえん
\t21,ジト
}

group,1
{
\tscope,1

\t2100,素(1)
\t2101,照れ(1)
\t2102,驚き(1)
\t2103,不安(1)
\t2104,落胆(1)
\t2105,笑顔(1)
\t2106,静観(1)
\t2107,怒り(1)
\t2108,冷笑(1)
\t2109,照れ怒り(1)
\t2110,( ﾟдﾟ)(1)

\t2200,素(2)
\t2201,照れ(2)
\t2202,驚き(2)
\t2203,不安(2)
\t2204,落胆(2)
\t2205,笑顔(2)
\t2206,静観(2)
\t2207,怒り(2)
\t2208,冷笑(2)
\t2209,照れ怒り(2)
\t2210,( ﾟдﾟ)(2)
}
";

/// 検体 1 に対する SSP の答えの全文。
const SAMPLE_1_ANSWER: &str = r"|scope \0,\1,\p[2]...|character name|description|surface number : \s[]|
|-----|-----|-----|-----|
|\0|0|素|\s[0]|
|\0|0|照れ|\s[1]|
|\0|0|驚き|\s[2]|
|\0|0|不安|\s[3]|
|\0|0|落胆|\s[4]|
|\0|0|笑顔|\s[5]|
|\0|0|静観|\s[6]|
|\0|0|怒り|\s[7]|
|\0|0|冷笑|\s[8]|
|\0|0|照れ怒り|\s[9]|
|\0||\1-素|\s[10]|
|\0||\1-刮目|\s[11]|
|\0||\1-歌|\s[19]|
|\0|0|ぴえん|\s[20]|
|\0|0|ジト|\s[21]|
|\0||歌|\s[25]|
|\1|1|素(1)|\s[2100]|
|\1|1|照れ(1)|\s[2101]|
|\1|1|驚き(1)|\s[2102]|
|\1|1|不安(1)|\s[2103]|
|\1|1|落胆(1)|\s[2104]|
|\1|1|笑顔(1)|\s[2105]|
|\1|1|静観(1)|\s[2106]|
|\1|1|怒り(1)|\s[2107]|
|\1|1|冷笑(1)|\s[2108]|
|\1|1|照れ怒り(1)|\s[2109]|
|\1|1|( ﾟдﾟ)(1)|\s[2110]|
|\1|1|素(2)|\s[2200]|
|\1|1|照れ(2)|\s[2201]|
|\1|1|驚き(2)|\s[2202]|
|\1|1|不安(2)|\s[2203]|
|\1|1|落胆(2)|\s[2204]|
|\1|1|笑顔(2)|\s[2205]|
|\1|1|静観(2)|\s[2206]|
|\1|1|怒り(2)|\s[2207]|
|\1|1|冷笑(2)|\s[2208]|
|\1|1|照れ怒り(2)|\s[2209]|
|\1|1|( ﾟдﾟ)(2)|\s[2210]|
";

/// `surfacetable.txt` の無いシェルに対する SSP の答えの全文（検体 2・既定の 15 件だけ）。
const DEFAULTS_ONLY_ANSWER: &str = r"|scope \0,\1,\p[2]...|character name|description|surface number : \s[]|
|-----|-----|-----|-----|
|\0||素|\s[0]|
|\0||照れ|\s[1]|
|\0||驚き|\s[2]|
|\0||不安|\s[3]|
|\0||落ち込み|\s[4]|
|\0||微笑み|\s[5]|
|\0||目閉じ|\s[6]|
|\0||怒り|\s[7]|
|\0||冷笑|\s[8]|
|\0||照れ怒り|\s[9]|
|\0||\1-素|\s[10]|
|\0||\1-刮目|\s[11]|
|\0||\1-歌|\s[19]|
|\0||立て看板|\s[20]|
|\0||歌|\s[25]|
";

/// 実測 14 の検体の全文が、見出し行から最後の `\r\n` まで SSP の答えと一致する（要件 7.1・4.1〜4.3）。
#[test]
fn sample_1_matches_the_ssp_answer_in_full() {
    assert_eq!(render(&parse_surfacetable(SAMPLE_1)), crlf(SAMPLE_1_ANSWER));
}

/// 空の転記は既定の 15 件だけの表（検体 2・要件 3.1・3.5）。
#[test]
fn an_empty_table_renders_the_15_defaults_only() {
    assert_eq!(render(&SurfaceTable::default()), crlf(DEFAULTS_ONLY_ANSWER));
}

/// スコープ 2 以上は `\p[n]`（要件 1.4）。`__disabled` と `__parts` の行は載らないが、
/// その ID の既定は消える（要件 2.8・2.9・3.4）。
#[test]
fn scope_two_is_written_as_p_and_disabled_or_parts_rows_hide_defaults() {
    let text = "group,__disabled\n{\n10,__parts\n}\ngroup,x\n{\nscope,2\n11,__parts\n200,y\n}\n";
    let rendered = render(&parse_surfacetable(text));
    assert!(rendered.ends_with(&crlf(
        r"|\0||歌|\s[25]|
|\p[2]|x|y|\s[200]|"
    )));
    assert!(!rendered.contains(r"\s[10]|"));
    assert!(!rendered.contains(r"\s[11]|"));
    assert!(!rendered.contains("__"));
}

// ---- 実測ごとの行と既定の重ね合わせの境目（ssp-measurements.md の検体 3〜9） ----

/// 見出し行と区切り行を前に付け、各行を `\r\n` で終えて 1 本にする（要件 1.1・1.3）。
fn answer(rows: &str) -> String {
    crlf(&format!(
        "|scope \\0,\\1,\\p[2]...|character name|description|surface number : \\s[]|\n\
         |-----|-----|-----|-----|\n{rows}"
    ))
}

/// 検体 3（`group` の無い平たいファイル）: `group` の外の行はスコープ `\0`・キャラクタ名は空で載り、
/// 既定の 20 はシェルの行に替わる。既定にも行にも無い ID（21〜24 など）は載らない（要件 2.5・2.11）。
#[test]
fn sample_3_rows_outside_any_group_are_scope_0_with_no_name() {
    let text = "charset,Shift_JIS\n\n0,素\n20,パーカー素\n30,両手素\n40,片手素\n50,チラ見せ\n";
    assert_eq!(
        render(&parse_surfacetable(text)),
        answer(
            r"|\0||素|\s[0]|
|\0||照れ|\s[1]|
|\0||驚き|\s[2]|
|\0||不安|\s[3]|
|\0||落ち込み|\s[4]|
|\0||微笑み|\s[5]|
|\0||目閉じ|\s[6]|
|\0||怒り|\s[7]|
|\0||冷笑|\s[8]|
|\0||照れ怒り|\s[9]|
|\0||\1-素|\s[10]|
|\0||\1-刮目|\s[11]|
|\0||\1-歌|\s[19]|
|\0||パーカー素|\s[20]|
|\0||歌|\s[25]|
|\0||両手素|\s[30]|
|\0||片手素|\s[40]|
|\0||チラ見せ|\s[50]|"
        )
    );
}

/// 検体 4（名前の省略と閉じていない `{`）: 名前を省略した行は説明の列を空にして載り、既定の 0 を消す。
/// 閉じないまま終わっても読めた分で組む（要件 1.5・2.6・3.4・5.5）。
#[test]
fn sample_4_an_omitted_name_is_an_empty_description_even_in_an_unclosed_group() {
    let text = "charset,Shift_JIS\nversion,1\ngroup,本体基本\n{\n0,\n";
    assert_eq!(
        render(&parse_surfacetable(text)),
        answer(
            r"|\0|本体基本||\s[0]|
|\0||照れ|\s[1]|
|\0||驚き|\s[2]|
|\0||不安|\s[3]|
|\0||落ち込み|\s[4]|
|\0||微笑み|\s[5]|
|\0||目閉じ|\s[6]|
|\0||怒り|\s[7]|
|\0||冷笑|\s[8]|
|\0||照れ怒り|\s[9]|
|\0||\1-素|\s[10]|
|\0||\1-刮目|\s[11]|
|\0||\1-歌|\s[19]|
|\0||立て看板|\s[20]|
|\0||歌|\s[25]|"
        )
    );
}

/// 検体 5（`__disabled` が既定を消す）: `__disabled` の行は載らず、その中の 10 は既定の 10 も消す
/// （要件 2.8・2.9・3.3・3.4）。
#[test]
fn sample_5_disabled_rows_are_not_shown_and_hide_the_default() {
    let text = "charset,Shift_JIS
version,1

group,￥０名
{
scope,0

0,素
1,照れ
2,驚き
3,不安
4,落ち込み
5,微笑み
6,目閉じ
7,怒り
8,冷笑
9,照れ怒り
}

//表示されないためのもの
group,__disabled
{
10,__parts
100,__parts
101,__parts
}
";
    assert_eq!(
        render(&parse_surfacetable(text)),
        answer(
            r"|\0|￥０名|素|\s[0]|
|\0|￥０名|照れ|\s[1]|
|\0|￥０名|驚き|\s[2]|
|\0|￥０名|不安|\s[3]|
|\0|￥０名|落ち込み|\s[4]|
|\0|￥０名|微笑み|\s[5]|
|\0|￥０名|目閉じ|\s[6]|
|\0|￥０名|怒り|\s[7]|
|\0|￥０名|冷笑|\s[8]|
|\0|￥０名|照れ怒り|\s[9]|
|\0||\1-刮目|\s[11]|
|\0||\1-歌|\s[19]|
|\0||立て看板|\s[20]|
|\0||歌|\s[25]|"
        )
    );
}

/// 検体 6 と同じ形（グループ名が空・`scope` の行なし・字下げはタブ）: 行はスコープ `\0`・
/// キャラクタ名は空で載り、書かれていない 11・20 の既定が ID の順の位置に入る（要件 2.3・2.4・4.3）。
#[test]
fn sample_6_shape_an_empty_group_name_without_scope_is_scope_0_with_no_name() {
    let text = "group,\n{\n\t10,星\n\t15,にっこり\n\t16,お祈り\n\t18,照れ手組口開け\n\
                \t19,照れ手組口開け半目\n\t25,どや顔\n\t100,素\n}\n";
    assert_eq!(
        render(&parse_surfacetable(text)),
        answer(
            r"|\0||素|\s[0]|
|\0||照れ|\s[1]|
|\0||驚き|\s[2]|
|\0||不安|\s[3]|
|\0||落ち込み|\s[4]|
|\0||微笑み|\s[5]|
|\0||目閉じ|\s[6]|
|\0||怒り|\s[7]|
|\0||冷笑|\s[8]|
|\0||照れ怒り|\s[9]|
|\0||星|\s[10]|
|\0||\1-刮目|\s[11]|
|\0||にっこり|\s[15]|
|\0||お祈り|\s[16]|
|\0||照れ手組口開け|\s[18]|
|\0||照れ手組口開け半目|\s[19]|
|\0||立て看板|\s[20]|
|\0||どや顔|\s[25]|
|\0||素|\s[100]|"
        )
    );
}

/// 検体 7 と同じ形: スコープ 2 は `\p[2]`、同じスコープ 2 の `group` が 2 つあると `group` ごとに
/// まとめず ID の順に混ざる（ここではファイルに `その他` を先に書く）。スコープ 1 に書かれた
/// 10・11・19 は `\0` の既定を消し、25 は消さない（要件 1.4・2.7・3.4・4.1・4.4）。
#[test]
fn sample_7_shape_scope_2_groups_mix_by_id_and_scope_1_ids_hide_defaults() {
    let text = "charset,Shift_JIS
group,__disabled
{
\t1000,__parts
}
group,エミリ
{
\tscope,0
\t24,魔法発動
\t27,疑問
}
group,テディ
{
\tscope,1
\t10,素
\t11,呆れ
\t12,驚き
\t14,コゲコゲ
\t17,怒
\t19,何かいやなことを…
\t1100,TB<NEW!>
}
group,その他
{
\tscope,2
\t500,謎ペット
\t600,誰？
\t700,黒板
}
group,エミリオ
{
\tscope,2
\t200,素
\t240,コゲコゲ
}
";
    assert_eq!(
        render(&parse_surfacetable(text)),
        answer(
            r"|\0||素|\s[0]|
|\0||照れ|\s[1]|
|\0||驚き|\s[2]|
|\0||不安|\s[3]|
|\0||落ち込み|\s[4]|
|\0||微笑み|\s[5]|
|\0||目閉じ|\s[6]|
|\0||怒り|\s[7]|
|\0||冷笑|\s[8]|
|\0||照れ怒り|\s[9]|
|\0||立て看板|\s[20]|
|\0|エミリ|魔法発動|\s[24]|
|\0||歌|\s[25]|
|\0|エミリ|疑問|\s[27]|
|\1|テディ|素|\s[10]|
|\1|テディ|呆れ|\s[11]|
|\1|テディ|驚き|\s[12]|
|\1|テディ|コゲコゲ|\s[14]|
|\1|テディ|怒|\s[17]|
|\1|テディ|何かいやなことを…|\s[19]|
|\1|テディ|TB<NEW!>|\s[1100]|
|\p[2]|エミリオ|素|\s[200]|
|\p[2]|エミリオ|コゲコゲ|\s[240]|
|\p[2]|その他|謎ペット|\s[500]|
|\p[2]|その他|誰？|\s[600]|
|\p[2]|その他|黒板|\s[700]|"
        )
    );
}

/// 検体 8（`option,DisableNoDefineSurfaces`）の写しにある行だけ（0〜8 と `__disabled` の行の名前は
/// 写しに無い）。グループ名 `\0` はそのまま載る。
const SAMPLE_8_ROWS: &str = r"group,\0
{
scope,0
9,笑顔
11,しゅん
12,真顔
13,んー
14,目閉じ微笑
15,困り微笑
100,ヤダ
}
group,__disabled
{
10500,__parts
}
";

/// 検体 8 の答え（0〜8 は書いていないので既定）。
const SAMPLE_8_ANSWER: &str = r"|\0||素|\s[0]|
|\0||照れ|\s[1]|
|\0||驚き|\s[2]|
|\0||不安|\s[3]|
|\0||落ち込み|\s[4]|
|\0||微笑み|\s[5]|
|\0||目閉じ|\s[6]|
|\0||怒り|\s[7]|
|\0||冷笑|\s[8]|
|\0|\0|笑顔|\s[9]|
|\0||\1-素|\s[10]|
|\0|\0|しゅん|\s[11]|
|\0|\0|真顔|\s[12]|
|\0|\0|んー|\s[13]|
|\0|\0|目閉じ微笑|\s[14]|
|\0|\0|困り微笑|\s[15]|
|\0||\1-歌|\s[19]|
|\0||立て看板|\s[20]|
|\0||歌|\s[25]|
|\0|\0|ヤダ|\s[100]|";

/// 検体 8: `option,DisableNoDefineSurfaces` の有無で同じ文字列になり、定義の無い既定の 19・20・25 も
/// 載る（要件 3.7・3.8・7.5）。
#[test]
fn sample_8_disable_no_define_surfaces_does_not_change_the_table() {
    let with = render(&parse_surfacetable(&format!(
        "charset,UTF-8\noption,DisableNoDefineSurfaces\n{SAMPLE_8_ROWS}"
    )));
    let without = render(&parse_surfacetable(&format!(
        "charset,UTF-8\n{SAMPLE_8_ROWS}"
    )));
    assert_eq!(with, answer(SAMPLE_8_ANSWER));
    assert_eq!(without, with);
}

/// 検体 9（行の終わりの `}`・ファイルはそこで終わる）: `}` は名前の一部として載る（要件 5.6）。
#[test]
fn sample_9_a_trailing_brace_is_part_of_the_name() {
    let text = "group,ほつれ銀糸\n{\n21,赤瞳孔_狂\n100,黒塗り}";
    assert_eq!(
        render(&parse_surfacetable(text)),
        answer(
            r"|\0||素|\s[0]|
|\0||照れ|\s[1]|
|\0||驚き|\s[2]|
|\0||不安|\s[3]|
|\0||落ち込み|\s[4]|
|\0||微笑み|\s[5]|
|\0||目閉じ|\s[6]|
|\0||怒り|\s[7]|
|\0||冷笑|\s[8]|
|\0||照れ怒り|\s[9]|
|\0||\1-素|\s[10]|
|\0||\1-刮目|\s[11]|
|\0||\1-歌|\s[19]|
|\0||立て看板|\s[20]|
|\0|ほつれ銀糸|赤瞳孔_狂|\s[21]|
|\0||歌|\s[25]|
|\0|ほつれ銀糸|黒塗り}|\s[100]|"
        )
    );
}

/// 既定が消える 3 つ（スコープ 1 に書かれた 10・`__disabled` の中に `__parts` でない名前で書かれた 11・
/// 名前を省略した 0）と、消えない 1 つ（どこにも書かれていない 1〜9・20・25）。`__disabled` の外の
/// `__parts`（19）も載らずに既定を消す（要件 2.8・2.9・3.2〜3.4・7.3）。
#[test]
fn defaults_are_hidden_by_any_written_id_and_kept_otherwise() {
    let text = "group,テディ\n{\nscope,1\n10,素\n19,__parts\n}\ngroup,__disabled\n{\n11,腕\n}\n\
                group,本体基本\n{\n0,\n}\n";
    assert_eq!(
        render(&parse_surfacetable(text)),
        answer(
            r"|\0|本体基本||\s[0]|
|\0||照れ|\s[1]|
|\0||驚き|\s[2]|
|\0||不安|\s[3]|
|\0||落ち込み|\s[4]|
|\0||微笑み|\s[5]|
|\0||目閉じ|\s[6]|
|\0||怒り|\s[7]|
|\0||冷笑|\s[8]|
|\0||照れ怒り|\s[9]|
|\0||立て看板|\s[20]|
|\0||歌|\s[25]|
|\1|テディ|素|\s[10]|"
        )
    );
}

/// 同じ（スコープ, ID）が 2 度書かれたら、両方が書かれた順に載る（グループ名の順ではない）。
/// 表全体は失敗しない（要件 5.9・5.10）。
#[test]
fn the_same_id_written_twice_is_shown_twice_in_written_order() {
    let text = "group,b\n{\n30,二\n}\ngroup,a\n{\n30,一\n}\n";
    assert_eq!(
        render(&parse_surfacetable(text)),
        format!(
            "{}{}",
            crlf(DEFAULTS_ONLY_ANSWER),
            crlf("|\\0|b|二|\\s[30]|\n|\\0|a|一|\\s[30]|")
        )
    );
}

/// 同じスコープの 2 つの `group` の ID は互い違いでも ID の順に混ざり、別のスコープに書かれた
/// 同じ ID はそれぞれのスコープに載る（要件 4.4・2.7）。
#[test]
fn ids_interleave_across_groups_and_the_same_id_appears_in_each_scope() {
    let text =
        "group,b\n{\n31,b一\n33,b三\n}\ngroup,a\n{\n32,a二\n}\ngroup,c\n{\nscope,1\n31,c一\n}\n";
    assert_eq!(
        render(&parse_surfacetable(text)),
        format!(
            "{}{}",
            crlf(DEFAULTS_ONLY_ANSWER),
            crlf(
                r"|\0|b|b一|\s[31]|
|\0|a|a二|\s[32]|
|\0|b|b三|\s[33]|
|\1|c|c一|\s[31]|"
            )
        )
    );
}

// ---- 読み取りと記録（load・一時フォルダは temp-path-kit・記録は log-capture-kit） ----

use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;

/// `本体` と `表情` の Shift_JIS（`表` の 2 バイト目は `\` と同じ 0x5C）。
const SJIS_HONTAI: [u8; 4] = [0x96, 0x7B, 0x91, 0xCC];
const SJIS_HYOUJOU: [u8; 4] = [0x95, 0x5C, 0x8F, 0xEE];

/// `group,本体`・`scope,0`・`100,表情` を読めたときの表（既定の 15 件の後に 100 が続く）。
fn hontai_answer() -> String {
    format!(
        "{}{}",
        crlf(DEFAULTS_ONLY_ANSWER),
        crlf(r"|\0|本体|表情|\s[100]|")
    )
}

/// `group,本体` の 1 グループを、先頭に `head` を付けた UTF-8 のバイト列にする。
fn utf8_hontai(head: &str) -> Vec<u8> {
    format!("{head}group,本体\r\n{{\r\n\tscope,0\r\n\t100,表情\r\n}}\r\n").into_bytes()
}

/// `group,本体` の 1 グループを、先頭に `head` を付けた Shift_JIS のバイト列にする。
fn sjis_hontai(head: &str) -> Vec<u8> {
    [
        head.as_bytes(),
        b"group,",
        &SJIS_HONTAI,
        b"\r\n{\r\n\tscope,0\r\n\t100,",
        &SJIS_HYOUJOU,
        b"\r\n}\r\n",
    ]
    .concat()
}

/// 一時フォルダをシェルのフォルダに見立て、`surfacetable.txt` に `bytes` を書く。
fn shell_dir_with(bytes: &[u8]) -> TempPath {
    let dir = TempPath::new("expression-table");
    std::fs::write(dir.child("surfacetable.txt"), bytes).expect("surfacetable.txt を書く");
    dir
}

fn warns(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|ev| ev.level == tracing::Level::WARN)
        .collect()
}

/// Shift_JIS（`charset` なし・`charset,Shift_JIS`）と UTF-8（`Charset,UTF-8`・BOM 付き・BOM なし
/// `charset,UTF-8`）のそれぞれで、キャラクタ名と説明が元の字のとおりの表になる。どれも記録を出さない
/// （要件 7.4・1.6・5.1・5.2）。
#[test]
fn each_charset_and_bom_reads_names_as_written() {
    let cases: [(&str, Vec<u8>); 5] = [
        ("Shift_JIS・charset なし", sjis_hontai("")),
        ("charset,Shift_JIS", sjis_hontai("charset,Shift_JIS\r\n")),
        ("Charset,UTF-8", utf8_hontai("Charset,UTF-8\r\n")),
        ("BOM 付き UTF-8", utf8_hontai("\u{feff}charset,UTF-8\r\n")),
        ("BOM なし charset,UTF-8", utf8_hontai("charset,UTF-8\r\n")),
    ];
    for (label, bytes) in cases {
        let dir = shell_dir_with(&bytes);
        let (rendered, events) = capture(|| render(&load(dir.path())));
        assert_eq!(rendered, hontai_answer(), "{label}");
        assert!(warns(&events).is_empty(), "{label}: {events:?}");
    }
}

/// `surfacetable.txt` が無い → 既定の 15 件だけ・`warn!` は出さない（要件 3.5）。
#[test]
fn a_missing_file_is_defaults_only_without_a_warning() {
    let dir = TempPath::new("expression-table");
    let (rendered, events) = capture(|| render(&load(dir.path())));
    assert_eq!(rendered, crlf(DEFAULTS_ONLY_ANSWER));
    assert!(warns(&events).is_empty(), "{events:?}");
}

/// `surfacetable.txt` という名前のフォルダ（開けない）→ 既定の 15 件だけ・`warn!` が 1 件（要件 5.8）。
#[test]
fn an_unopenable_file_is_defaults_only_with_one_warning() {
    let dir = TempPath::new("expression-table");
    std::fs::create_dir(dir.child("surfacetable.txt")).expect("同名のフォルダを作る");
    let (rendered, events) = capture(|| render(&load(dir.path())));
    assert_eq!(rendered, crlf(DEFAULTS_ONLY_ANSWER));
    let warns = warns(&events);
    assert_eq!(warns.len(), 1, "{events:?}");
    assert!(
        warns[0].message().starts_with("[get_expression_table]"),
        "{events:?}"
    );
    assert!(warns[0].field("path").is_some(), "{events:?}");
    assert!(warns[0].field("error").is_some(), "{events:?}");
}

/// 読めない行が 2 つあっても、読めた分の表が返り、`warn!` は 1 件だけで行番号の列を持つ
/// （要件 7.6・5.7）。
#[test]
fn unreadable_lines_keep_the_readable_rows_and_warn_once_with_line_numbers() {
    let dir = shell_dir_with(
        "charset,UTF-8\r\ngroup,本体\r\n{\r\n\tscope,0\r\n\t100,表情\r\n\tabc,壊れ\r\n\t壊れ\r\n}\r\n"
            .as_bytes(),
    );
    let (rendered, events) = capture(|| render(&load(dir.path())));
    assert_eq!(rendered, hontai_answer());
    let warns = warns(&events);
    assert_eq!(warns.len(), 1, "{events:?}");
    assert!(
        warns[0].message().starts_with("[get_expression_table]"),
        "{events:?}"
    );
    assert_eq!(warns[0].field("lines"), Some("[6, 7]"), "{events:?}");
}

/// 同じフォルダの `surfaces.txt` の `surface.alias`ブレスと surface*ブレスの `name` は表を変えず、
/// `surfaces.txt` にだけ定義された ID も載らない（要件 2.10・2.12）。
#[test]
fn surfaces_txt_in_the_same_folder_does_not_change_the_table() {
    let dir = shell_dir_with(&utf8_hontai("charset,UTF-8\r\n"));
    let before = render(&load(dir.path()));
    std::fs::write(
        dir.child("surfaces.txt"),
        "charset,UTF-8\r\n\
         surface.alias\r\n{\r\n50,[0]\r\n}\r\n\
         surface0\r\n{\r\nname,別名\r\n}\r\n\
         surface77\r\n{\r\nname,定義だけ\r\nelement0,overlay,a.png,0,0\r\n}\r\n",
    )
    .expect("surfaces.txt を書く");

    let after = render(&load(dir.path()));
    assert_eq!(after, before);
    assert_eq!(after, hontai_answer());
    assert!(!after.contains(r"\s[77]"), "{after}");
    assert!(!after.contains(r"\s[50]"), "{after}");
}

// ---- 配線（実行系つきの単位・同時の呼び出し・副作用なし・切替の後） ----

/// 置き場のゴーストの今のシェルのフォルダ（実行系が無ければ `None`）。
fn current_shell_dir(world: &World) -> Option<PathBuf> {
    world
        .get_non_send::<GhostSlot>()?
        .0
        .as_ref()?
        .runtime()
        .map(|runtime| runtime.mount().shell.dir.clone())
}

/// kanade へ先に送られた依頼の処理が済むまで待つ（受信は順に 1 件ずつ・空のリソース照会は SHIORI を
/// 呼ばずにその場で答える）。これで「呼出の記録が増えない」が kanade の遅れで素通りしない。
fn settle_kanade(world: &World) {
    let kanade = world
        .non_send::<GhostSlot>()
        .0
        .as_ref()
        .and_then(|session| session.kanade())
        .expect("起動したゴーストには kanade の送り口がある")
        .clone();
    let (reply, rx) = areka_actor::reply_channel();
    kanade
        .send(areka_kanade::KanadeMsg::ResourceQuery {
            ids: Vec::new(),
            reply,
        })
        .expect("kanade に届く");
    rx.recv_timeout(std::time::Duration::from_secs(20))
        .expect("照会の返事が届く");
}

/// 偽の SHIORI で実行系つきの単位を起こし、受け口を置いて定常に落ち着かせる。今のシェルのフォルダへ
/// `surfacetable.txt` を置き、2 件続けて送って `Input` の段を 1 回回すと 2 件とも同じ表で答え、
/// SHIORI の呼出の記録は増えない。`set_shell_dir` でリグの根の下の別のシェルのフォルダへ替えて呼ぶと、
/// 替えた後の表で答える（要件 1.7・6.1・6.2・6.4）。
#[test]
fn real_unit_answers_from_the_current_shell_each_call_without_side_effects() {
    use std::sync::mpsc;

    use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};

    let mut rig = SwitchRig::new(vec![(
        "A",
        FakeShiori::Scripted(Box::new(|| standard_script(r"\0A\e"))),
    )]);
    rig.boot("A");
    let (tx, rx) = mpsc::channel();
    crate::mcp::install(&mut rig.world, rx);
    let steady = rig.wait_steady();
    let shell = current_shell_dir(&rig.world).expect("起動したゴーストには実行系がある");
    std::fs::write(
        shell.join("surfacetable.txt"),
        utf8_hontai("charset,UTF-8\r\n"),
    )
    .expect("今のシェルへ surfacetable.txt を書く");
    settle_kanade(&rig.world);
    let calls_before = rig.calls("A");

    let ask = || {
        let (request, pending) = ToolRequest::new(ToolCall::GetExpressionTable(Args {
            ghost_name: Some("A".to_owned()),
        }));
        assert!(tx.send(request).is_ok(), "受け口は生きている");
        pending
    };
    let (first, second) = (ask(), ask());
    rig.world.run_schedule(wintf::ecs::Input);
    let both = [&first, &second].map(|p| p.try_answer().ok().flatten().map(|a| a.outcome));
    settle_kanade(&rig.world);
    let calls_after = rig.calls("A");

    // 別のシェルのフォルダ（リグの根の下）へ替えて呼ぶ。
    let other = rig
        .root
        .ghost_dir("A")
        .join("shell")
        .join("expression-table-other");
    std::fs::create_dir_all(&other).expect("別のシェルのフォルダを作る");
    std::fs::write(
        other.join("surfacetable.txt"),
        "charset,UTF-8\r\ngroup,別\r\n{\r\n\tscope,1\r\n\t200,横顔\r\n}\r\n",
    )
    .expect("別のシェルへ surfacetable.txt を書く");
    let switched = rig
        .world
        .get_non_send_mut::<GhostSlot>()
        .and_then(|mut slot| slot.0.as_mut().map(|s| s.set_shell_dir(other.clone())));
    let third = ask();
    rig.world.run_schedule(wintf::ecs::Input);
    let after_switch = third.try_answer().ok().flatten().map(|a| a.outcome);
    let down = rig.shutdown();

    let table = Some(outcome::value(hontai_answer()));
    assert_eq!(
        (steady, both, switched, down),
        (true, [table.clone(), table], Some(true), true),
        "（定常に着いた・2 件の答え・替えられた・降ろせた）"
    );
    assert_eq!(calls_after, calls_before, "呼出の記録は増えない");
    assert_eq!(
        after_switch,
        Some(outcome::value(format!(
            "{}{}",
            crlf(DEFAULTS_ONLY_ANSWER),
            crlf(r"|\1|別|横顔|\s[200]|")
        ))),
        "替えた後の表"
    );
}
