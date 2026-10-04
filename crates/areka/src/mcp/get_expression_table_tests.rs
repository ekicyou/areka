//! `get_expression_table` の決定論テスト。
//!
//! 空の World と作ったゴースト・引数で呼ぶと `NG:not implemented yet`（isError: true）を返す
//! （空の World で答えられる＝ゴーストに何もさせていない・要件 5.1・5.2・5.4）。

use std::path::PathBuf;

use areka_mcp::ToolContent;
use areka_mcp::tools::{ToolCall, ToolRequest};

use super::*;

#[test]
fn answers_not_implemented_yet_with_an_empty_world() {
    let ghost = ActiveGhost {
        name: Some("Emily/Phase4.5".to_string()),
        root: PathBuf::from(r"C:\ssp\ghost\emily4"),
    };
    let args = Args {
        ghost_name: Some("Emily/Phase4.5".to_string()),
    };
    let (req, pending) = ToolRequest::new(ToolCall::GetExpressionTable(args.clone()));

    handle(&mut World::new(), &ghost, args, req.reply);

    let answer = pending.try_answer().ok().flatten().expect("その場で答える");
    assert_eq!(
        answer.outcome.content,
        vec![ToolContent::Text("NG:not implemented yet".to_string())]
    );
    assert!(answer.outcome.is_error);
}

// ---- 規則（render(&parse_surfacetable(..))・期待値は ssp-measurements.md の写し） ----

use areka_parsers::shell::parse_surfacetable;

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
