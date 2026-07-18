//! `decode` の単体テスト（emo2 subset 値正規化・タスク 4.1 範囲）。
//!
//! 検証対象は構文トークン（`lexer::Token`）→ 値正規化済み `Instruction` の写像:
//! - 待ち時間（`\w[n]` / `\wN` = n×50ms、`\_w[ms]` = 絶対 ms）の境界値。
//! - 改行比率（素の `\n` = 1.0、`\n[percent]` = percent/100、`\n[half]` = 0.5、負値）。
//! - 選択肢 `\q[disp,target,refs...]` の disp/target 分離 ＋ references 順序保持。
//! - `\![move,...]` の Move 化、`\p[n]`/`\s[...]`/`\_l[x,y]`/`\e`/`\c`/`\-`、`%keyword`、テキスト。
//!
//! `move` 以外の `\!`・subset 外タグ・`\q` 旧 2 連形・`\![*]`（タスク 4.2）はここでは検証しない。
//!
//! decode は lexer を消費するため、入力は実スクリプト文字列を `lex` した結果を渡す
//! （構文区切りと意味デコードの結線が正しいことも同時に固定する）。

use super::decode::decode;
use super::lexer::lex;
use super::model::{Choice, Instruction, MoveArgs, NewLineRatio, SurfaceArg};
use std::time::Duration;

/// 文字列をレックスしてデコードするヘルパ（テスト題材は実スクリプト断片）。
fn dec(input: &str) -> Vec<Instruction> {
    decode(lex(input))
}

// ── 待ち時間（要件 3）─────────────────────────────────────────────

/// `\wN` 短縮の境界: `\w1` → 50ms、`\w9` → 450ms（n × 50ms・要件 3.2）。
#[test]
fn wait_shorthand_boundary_values() {
    assert_eq!(dec(r"\w1"), vec![Instruction::Wait(Duration::from_millis(50))]);
    assert_eq!(dec(r"\w9"), vec![Instruction::Wait(Duration::from_millis(450))]);
}

/// `\w[n]` 正準形: `\w[2]` → 100ms、`\w[9]` → 450ms（n × 50ms・要件 3.1）。
#[test]
fn wait_bracket_n_times_50ms() {
    assert_eq!(dec(r"\w[2]"), vec![Instruction::Wait(Duration::from_millis(100))]);
    assert_eq!(dec(r"\w[9]"), vec![Instruction::Wait(Duration::from_millis(450))]);
}

/// `\_w[ms]` 絶対ミリ秒: `\_w[450]` → 450ms、`\_w[950]` → 950ms（要件 3.3）。
#[test]
fn wait_underscore_absolute_ms() {
    assert_eq!(dec(r"\_w[450]"), vec![Instruction::Wait(Duration::from_millis(450))]);
    assert_eq!(dec(r"\_w[950]"), vec![Instruction::Wait(Duration::from_millis(950))]);
}

/// `\w`系/`\_w`系のいずれも同一 `Wait` variant（正規化済み Duration）で表現（要件 3.4）。
#[test]
fn wait_variants_unified_to_same_duration() {
    // \w9 = 450ms と \_w[450] = 450ms は同一 Instruction。
    assert_eq!(dec(r"\w9"), dec(r"\_w[450]"));
}

// ── 改行と割合改行（要件 4）─────────────────────────────────────

/// 素の `\n`（bare）→ 既定比率 1.0（要件 4.2）。
#[test]
fn newline_bare_default_ratio_one() {
    assert_eq!(
        dec(r"\n"),
        vec![Instruction::NewLine(NewLineRatio::new(1.0))],
    );
}

/// `\n[percent]` → percent/100（`\n[150]` → 1.5・要件 4.1）。
#[test]
fn newline_percent_ratio() {
    assert_eq!(
        dec(r"\n[150]"),
        vec![Instruction::NewLine(NewLineRatio::new(1.5))],
    );
    assert_eq!(
        dec(r"\n[100]"),
        vec![Instruction::NewLine(NewLineRatio::new(1.0))],
    );
}

/// `\n[half]` → 0.5（design 値定数）。
#[test]
fn newline_half_ratio() {
    assert_eq!(
        dec(r"\n[half]"),
        vec![Instruction::NewLine(NewLineRatio::new(0.5))],
    );
}

/// 負値の `\n[-100]` → -1.0（戻り・符号付き保持・design 値定数）。
#[test]
fn newline_negative_ratio_back() {
    assert_eq!(
        dec(r"\n[-100]"),
        vec![Instruction::NewLine(NewLineRatio::new(-1.0))],
    );
}

// ── 話者スコープとサーフェス（要件 2）──────────────────────────

/// `\p[0]` / `\p[1]` → 話者スコープ番号（要件 2.1）。
#[test]
fn speaker_scope_number() {
    assert_eq!(dec(r"\p[0]"), vec![Instruction::SpeakerScope { n: 0 }]);
    assert_eq!(dec(r"\p[1]"), vec![Instruction::SpeakerScope { n: 1 }]);
}

/// ブラケット `\p[n]` は**任意番号 u32**を保持し 0/1 へ clamp しない（開発者裁定#3・
/// ukadoc `\p[ID番号]`＝任意番号・R1.5/R4.4）。`speaker_scope_n` の u32 parse が
/// 生値を通すことを固定する（非退行）。
#[test]
fn speaker_scope_bracket_arbitrary_n_not_clamped() {
    assert_eq!(dec(r"\p[2]"), vec![Instruction::SpeakerScope { n: 2 }]);
    assert_eq!(dec(r"\p[5]"), vec![Instruction::SpeakerScope { n: 5 }]);
    assert_eq!(dec(r"\p[42]"), vec![Instruction::SpeakerScope { n: 42 }]);
}

/// 括弧なし短縮 `\pN`（ukadoc `\pID`＝任意番号・0〜9 のみ）→ `SpeakerScope{n}`
/// （lexer shorthand・R1.5/R4.4）。`\p2` → scope 2。ブラケット形 `\p[2]` と等価。
#[test]
fn speaker_scope_shorthand_single_digit() {
    assert_eq!(dec(r"\p2"), vec![Instruction::SpeakerScope { n: 2 }]);
}

/// 短縮 `\pN` は**単一桁**のみ消費する（lexer が 1 桁で終端＝`\wN` と同一規律）。
/// `\p10` → `SpeakerScope{1}` ＋ `Text("0")`（scope 10 ではない・ukadoc「0〜9のみ」）。
#[test]
fn speaker_scope_shorthand_consumes_one_digit_only() {
    assert_eq!(
        dec(r"\p10"),
        vec![
            Instruction::SpeakerScope { n: 1 },
            Instruction::Text("0".to_string()),
        ],
    );
}

/// 数字を伴わない裸 `\p` は shorthand でも正典スコープ bare でもない＝既存 passthrough の
/// まま `Raw("\p")`（`decode_bare` の match に `'p'` アームは無い・要件 11.2）。
/// `\p2`（shorthand）・`\p[2]`（Tag）との弁別を固定する。
#[test]
fn bare_p_without_digit_absorbed_as_raw() {
    assert_eq!(dec(r"\p"), vec![Instruction::Raw(r"\p".to_string())]);
}

/// `\s[...]` の中身は不透明文字列のまま無加工保持（要件 2.2/2.3）。
#[test]
fn surface_opaque_content_preserved() {
    // 数値も日本語エイリアスも改変しない。
    assert_eq!(
        dec(r"\s[1000]"),
        vec![Instruction::Surface(SurfaceArg::new("1000".to_string()))],
    );
    assert_eq!(
        dec(r"\s[通常]"),
        vec![Instruction::Surface(SurfaceArg::new("通常".to_string()))],
    );
}

// ── 選択肢（要件 5）────────────────────────────────────────────

/// `\q[タイトル,ID]` → disp/target 分離（references 空・要件 5.1）。
#[test]
fn choice_disp_target_split() {
    assert_eq!(
        dec(r"\q[はい,OnYes]"),
        vec![Instruction::Choice(Choice {
            disp: "はい".to_string(),
            target: "OnYes".to_string(),
            references: vec![],
        })],
    );
}

/// `\q[タイトル,ID,r0,r1]` → 第 3 引数以降を references に順序保持（要件 5.2）。
#[test]
fn choice_extra_references_order_preserved() {
    assert_eq!(
        dec(r"\q[t,id,r0,r1]"),
        vec![Instruction::Choice(Choice {
            disp: "t".to_string(),
            target: "id".to_string(),
            references: vec!["r0".to_string(), "r1".to_string()],
        })],
    );
}

// ── カーソル位置と制御命令（要件 6）───────────────────────────

/// `\_l[x,y]` → カーソル絶対位置（x/y は文字列のまま・要件 6.1）。
#[test]
fn cursor_absolute_position() {
    assert_eq!(
        dec(r"\_l[10,20]"),
        vec![Instruction::Cursor {
            x: "10".to_string(),
            y: "20".to_string(),
        }],
    );
}

/// `\e` → End、`\c` → Clear、`\-` → Quit（要件 6.2-6.4）。
#[test]
fn control_tags_end_clear_quit() {
    assert_eq!(dec(r"\e"), vec![Instruction::End]);
    assert_eq!(dec(r"\c"), vec![Instruction::Clear]);
    assert_eq!(dec(r"\-"), vec![Instruction::Quit]);
}

// ── キャラ移動（要件 7.1 — move のみ・タスク 4.1）─────────────

/// `\![move,dx,dy]` → Move（move を除いた生引数列を保持・要件 7.1）。
#[test]
fn move_command_decoded_with_args() {
    assert_eq!(
        dec(r"\![move,10,20]"),
        vec![Instruction::Move(MoveArgs {
            args: vec!["10".to_string(), "20".to_string()],
        })],
    );
}

// ── システム変数（要件 8）─────────────────────────────────────

/// `%username` → 展開なし SystemVar トークン（要件 8.1/8.2）。
#[test]
fn system_var_token_unexpanded() {
    assert_eq!(
        dec(r"%username"),
        vec![Instruction::SystemVar("username".to_string())],
    );
}

// ── テキストラン（要件 9）─────────────────────────────────────

/// タグ間プレーンテキスト → Text run（UTF-8 日本語・要件 9.1）。
#[test]
fn text_run_preserved() {
    assert_eq!(
        dec("こんばんわー！"),
        vec![Instruction::Text("こんばんわー！".to_string())],
    );
}

// ── 順序保持 ＋ 未デコード断片を残さない（要件 1.2/1.3）──────

/// 複数 subset タグ＋テキスト混在が入力順を保持し、全トークンが
/// いずれかの `Instruction` へ写像される（未デコード文字列断片を残さない・要件 1.2/1.3）。
#[test]
fn mixed_subset_tags_order_preserved_no_undecoded_fragment() {
    let out = dec(r"\p[0]\s[1000]こんにちは\_w[450]\n[150]\e");
    assert_eq!(
        out,
        vec![
            Instruction::SpeakerScope { n: 0 },
            Instruction::Surface(SurfaceArg::new("1000".to_string())),
            Instruction::Text("こんにちは".to_string()),
            Instruction::Wait(Duration::from_millis(450)),
            Instruction::NewLine(NewLineRatio::new(1.5)),
            Instruction::End,
        ],
    );
}

// ════════════════════════════════════════════════════════════════════
// タスク 4.2: 寛容パススルー・GenericCommand・旧 `\q` 吸収
// ════════════════════════════════════════════════════════════════════

// ── move 以外の `\!` → GenericCommand（要件 7.2/7.3）─────────────────

/// `\![open,sliderinput]` → `GenericCommand { name:"open", raw_args:["sliderinput"] }`
/// （第 1 引数=種別、残り=生引数・design L517・要件 7.2/7.3）。
#[test]
fn non_move_bang_to_generic_command() {
    assert_eq!(
        dec(r"\![open,sliderinput]"),
        vec![Instruction::GenericCommand {
            name: "open".to_string(),
            raw_args: vec!["sliderinput".to_string()],
        }],
    );
}

/// `\![raise,arm]` も同様（種別 raise・残り arm・要件 7.2/7.3）。
#[test]
fn non_move_bang_raise_to_generic_command() {
    assert_eq!(
        dec(r"\![raise,arm]"),
        vec![Instruction::GenericCommand {
            name: "raise".to_string(),
            raw_args: vec!["arm".to_string()],
        }],
    );
}

/// 引数 1 個のみ（生引数なし）の `\![bind]` → `GenericCommand { name:"bind", raw_args:[] }`。
#[test]
fn non_move_bang_single_arg_generic_command() {
    assert_eq!(
        dec(r"\![bind]"),
        vec![Instruction::GenericCommand {
            name: "bind".to_string(),
            raw_args: vec![],
        }],
    );
}

/// 引数空の `\![]` も種別空の GenericCommand（情報を捨てず・エラーにしない・要件 10.2）。
#[test]
fn empty_bang_to_generic_command_empty_name() {
    assert_eq!(
        dec(r"\![]"),
        vec![Instruction::GenericCommand {
            name: String::new(),
            raw_args: vec![],
        }],
    );
}

// ── `\![*]` 選択肢マーカー（要件 5.4）────────────────────────────────

/// `\![*]` が後続の現行 `\q[...]` を伴う場合、マーカーを Choice へ畳み、
/// 未デコードのマーカー文字列を残さない（要件 5.4・design L425）。
#[test]
fn choice_marker_folds_into_following_choice() {
    assert_eq!(
        dec(r"\![*]\q[はい,OnYes]"),
        vec![Instruction::Choice(Choice {
            disp: "はい".to_string(),
            target: "OnYes".to_string(),
            references: vec![],
        })],
    );
}

/// `\![*]` 単独（後続が現行 `\q` でない）→ `GenericCommand { name:"*" }`（design L425）。
#[test]
fn standalone_choice_marker_to_generic_command() {
    assert_eq!(
        dec(r"\![*]"),
        vec![Instruction::GenericCommand {
            name: "*".to_string(),
            raw_args: vec![],
        }],
    );
}

// ── 旧 2 連ブラケット `\q` → Raw（Choice 化しない・要件 5.3）──────────

/// 旧 2 連形 `\q[ID][タイトル]` は現行 Choice 化せず、丸ごと `Raw` で吸収する
/// （要件 5.3・design L425「Choice 化せず…Raw で保持」）。
#[test]
fn legacy_double_bracket_q_to_raw_not_choice() {
    assert_eq!(
        dec(r"\q[ID][タイトル]"),
        vec![Instruction::Raw(r"\q[ID][タイトル]".to_string())],
    );
}

/// 旧 2 連形 `\q*[ID][タイトル]`（`*` 付き）も丸ごと `Raw`（要件 5.3）。
#[test]
fn legacy_double_bracket_q_star_to_raw() {
    assert_eq!(
        dec(r"\q*[ID][タイトル]"),
        vec![Instruction::Raw(r"\q*[ID][タイトル]".to_string())],
    );
}

/// 旧 2 連形は前後の正常命令を破壊しない（隣接命令保持・要件 5.3/10.3）。
#[test]
fn legacy_q_preserves_neighbors() {
    assert_eq!(
        dec(r"\e\q[ID][タイトル]\c"),
        vec![
            Instruction::End,
            Instruction::Raw(r"\q[ID][タイトル]".to_string()),
            Instruction::Clear,
        ],
    );
}

/// 現行単一ブラケット `\q[はい,OnYes]` は引き続き Choice へ decode（4.1 の維持・要件 5.1）。
/// 旧形と現行形の弁別が正しいことを固定。
#[test]
fn modern_single_bracket_q_still_choice() {
    assert_eq!(
        dec(r"\q[はい,OnYes]"),
        vec![Instruction::Choice(Choice {
            disp: "はい".to_string(),
            target: "OnYes".to_string(),
            references: vec![],
        })],
    );
}

// ── subset 外タグ・不正トークン → Raw（要件 10.1/11.2）───────────────

/// emo2 subset 外の正準タグ `\foo[a,b]` → `Raw`（構文区切り＋raw 保持・要件 11.2/13.8）。
#[test]
fn unknown_tag_absorbed_as_raw() {
    assert_eq!(
        dec(r"\foo[a,b]"),
        vec![Instruction::Raw(r"\foo[a,b]".to_string())],
    );
}

/// 正典スコープタグ bare `\0` → `SpeakerScope{n:0}`（本体側・R1.5/R4.4・ukadoc `\0`=本体側）。
/// これは完了 spec `areka-P0-sakura-parse` req 11.2 の passthrough 分類の意図的スーパーセット
/// 更新（emo2 は `\p[n]` を発行するため無影響）。
#[test]
fn scope_bare_zero_maps_to_speaker_scope() {
    assert_eq!(dec(r"\0"), vec![Instruction::SpeakerScope { n: 0 }]);
}

/// 正典スコープタグ bare `\1` → `SpeakerScope{n:1}`（相方側・R1.5/R4.4・ukadoc `\1`=相方側）。
/// `boot.pasta:79` の `\1\![move,...]`（初回起動のエモ位置調整）が正しく scope 1 帰属する経路。
#[test]
fn scope_bare_one_maps_to_speaker_scope() {
    assert_eq!(dec(r"\1"), vec![Instruction::SpeakerScope { n: 1 }]);
}

/// bare スコープ**別名** `\h`（本体側=0）・`\u`（相方側=1）→ `SpeakerScope`（R1.5/R4.4・
/// ukadoc: `\0`/`\h`=本体側、`\1`/`\u`=相方側）。数字形 `\0`/`\1` と同一 scope へ写像する。
#[test]
fn scope_bare_h_u_aliases_map_to_speaker_scope() {
    assert_eq!(dec(r"\h"), vec![Instruction::SpeakerScope { n: 0 }]);
    assert_eq!(dec(r"\u"), vec![Instruction::SpeakerScope { n: 1 }]);
}

/// 真に未知の bare タグ（`\i` 等・スコープタグでない）は引き続き `Raw`（要件 11.2）。
#[test]
fn unknown_bare_tag_absorbed_as_raw() {
    assert_eq!(dec(r"\i"), vec![Instruction::Raw(r"\i".to_string())]);
}

/// lexer が区切れず Raw 吸収した不正断片（未閉じ `[`）は decode でも Raw のまま
/// （要件 10.1/13.8）。前後の正常命令は欠落しない（要件 10.3）。
#[test]
fn unclosed_bracket_raw_preserves_neighbors() {
    assert_eq!(
        dec(r"\e\s[1000"),
        vec![Instruction::End, Instruction::Raw(r"\s[1000".to_string())],
    );
}

/// 未知タグ・旧 `\q`・不正トークンが混在しても解析を中断せず、
/// 周囲の正常命令を保持する（要件 10.1/10.2/10.3 の総合固定）。
#[test]
fn lenient_passthrough_never_aborts_keeps_valid_neighbors() {
    let out = dec(r"\p[0]\foo[x]こんにちは\![open,a]\q[ID][タイトル]\e");
    assert_eq!(
        out,
        vec![
            Instruction::SpeakerScope { n: 0 },
            Instruction::Raw(r"\foo[x]".to_string()),
            Instruction::Text("こんにちは".to_string()),
            Instruction::GenericCommand {
                name: "open".to_string(),
                raw_args: vec!["a".to_string()],
            },
            Instruction::Raw(r"\q[ID][タイトル]".to_string()),
            Instruction::End,
        ],
    );
}

// ════════════════════════════════════════════════════════════════════
// タスク 1.2: バルーン面切替 `\b` の意味 decode（両形・不透明転写）
//
// State Management 表（design.md）の全行を檻に入れる。ブラケット形（数値形／
// 名前形）と裸形短縮の双方が、`\s` の `Surface` と完全対称の `BalloonSurface`
// へ忠実転写される（数値化・範囲展開・alias 解決を一切行わない・R1.1/1.4/1.5）。
// ════════════════════════════════════════════════════════════════════

/// ブラケット数値形 `\b[10]` → `[BalloonSurface("10")]`（第 1 引数を不透明保持・R1.1）。
#[test]
fn balloon_surface_bracket_numeric() {
    assert_eq!(
        dec(r"\b[10]"),
        vec![Instruction::BalloonSurface(SurfaceArg::new("10".to_string()))],
    );
}

/// ブラケット名前形 `\b[バルーン１]` → `[BalloonSurface("バルーン１")]`（無加工・整数へ
/// 限定変換しない・R1.1/R1.5）。
#[test]
fn balloon_surface_bracket_name_form_opaque() {
    assert_eq!(
        dec(r"\b[バルーン１]"),
        vec![Instruction::BalloonSurface(SurfaceArg::new("バルーン１".to_string()))],
    );
}

/// ブラケット非表示指定 `\b[-1]` → `[BalloonSurface("-1")]`（センチネルを文字列のまま
/// 保持し数値化・解決しない・R1.4）。
#[test]
fn balloon_surface_hide_sentinel_opaque() {
    assert_eq!(
        dec(r"\b[-1]"),
        vec![Instruction::BalloonSurface(SurfaceArg::new("-1".to_string()))],
    );
}

/// 裸形短縮 `\b1` → `[BalloonSurface("1")]`（1 桁の面数字を文字列で保持・本文へ
/// 漏らさない・R1.2/R1.3 の decode 面）。
#[test]
fn balloon_surface_shorthand_single_digit() {
    assert_eq!(
        dec(r"\b1"),
        vec![Instruction::BalloonSurface(SurfaceArg::new("1".to_string()))],
    );
}

/// 裸形 `\b12` → `[BalloonSurface("1"), Text("2")]`（面数字は 1 桁のみ消費し、続く
/// `2` は正当なバルーン本文として残す・R1.3）。
#[test]
fn balloon_surface_shorthand_digit_then_body_text() {
    assert_eq!(
        dec(r"\b12"),
        vec![
            Instruction::BalloonSurface(SurfaceArg::new("1".to_string())),
            Instruction::Text("2".to_string()),
        ],
    );
}

/// ukadoc fallback 形 `\b[2,--fallback=4]` は第 1 引数 `"2"` に落ちる（`\s` arm と
/// 完全対称の graceful 動作・Non-Goals 登記）。
#[test]
fn balloon_surface_fallback_form_takes_first_arg() {
    assert_eq!(
        dec(r"\b[2,--fallback=4]"),
        vec![Instruction::BalloonSurface(SurfaceArg::new("2".to_string()))],
    );
}

/// `BalloonSurface` の不透明中身は `SurfaceArg::as_str()` で読み取れる（read-only
/// accessor・無加工保持を直接確認）。
#[test]
fn balloon_surface_inner_readable_via_as_str() {
    let out = dec(r"\b[バルーン１]");
    match &out[..] {
        [Instruction::BalloonSurface(arg)] => assert_eq!(arg.as_str(), "バルーン１"),
        other => panic!("expected single BalloonSurface, got {other:?}"),
    }
}

/// `\b2[x]`（数字の直後が `[`）は短縮形でなく正準タグ `\b2[x]` 扱い＝subset 外 word
/// `b2` → `Raw`（既存 `\w2[x]` と同型・"b" arm 追加後も不変・R1.6 整合）。
#[test]
fn balloon_bracketed_digit_word_stays_raw() {
    assert_eq!(dec(r"\b2[x]"), vec![Instruction::Raw(r"\b2[x]".to_string())]);
}

/// 数字を伴わない裸 `\b` は bare タグ＝既存 passthrough のまま `Raw("\b")`（"b" arm は
/// ブラケット形／短縮形のみを取り込み、数字なし裸形の挙動は変えない・R1.6）。
#[test]
fn balloon_bare_no_digit_stays_raw() {
    assert_eq!(dec(r"\b"), vec![Instruction::Raw(r"\b".to_string())]);
}

/// 非退行（R1.6）: `\b` 追加後も `\s[...]` は引き続き `Surface`（別 variant）へ写像され、
/// バルーン面切替に奪われない。`\b`/`\s` が同一引数で異なる第一級命令になることを固定。
#[test]
fn shell_surface_unchanged_and_distinct_from_balloon() {
    assert_eq!(
        dec(r"\s[10]"),
        vec![Instruction::Surface(SurfaceArg::new("10".to_string()))],
    );
    assert_ne!(dec(r"\b[10]"), dec(r"\s[10]"));
}

/// 非退行（R1.6）: 既存 subset タグ（`\w9`/`\s[0]`/`\n`/`\e`）＋バルーン面切替の混在で、
/// 既存タグの写像が完全不変のまま `\b` だけが `BalloonSurface` として割り込む。
#[test]
fn existing_tags_unchanged_with_balloon_interleaved() {
    let out = dec(r"\s[0]\b[2]\w9\nテキスト\e");
    assert_eq!(
        out,
        vec![
            Instruction::Surface(SurfaceArg::new("0".to_string())),
            Instruction::BalloonSurface(SurfaceArg::new("2".to_string())),
            Instruction::Wait(Duration::from_millis(450)),
            Instruction::NewLine(NewLineRatio::new(1.0)),
            Instruction::Text("テキスト".to_string()),
            Instruction::End,
        ],
    );
}
