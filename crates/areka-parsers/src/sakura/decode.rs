//! 意味層（Decode）— 構文トークン → 値正規化済み `Instruction`（emo2 subset 限定）。
//!
//! Lexer（`lexer::lex`）が産んだ構文トークン列 `Vec<Token>` を消費し、
//! emo2 subset タグの値正規化を施した `Instruction` 列を産む（依存方向
//! `model ← lexer ← decode`）。下流が再パース不要なのは、この層が値を
//! decode しきるため（要件 1.2）。
//!
//! 本ファイル（タスク 4.1）が担うのは **emo2 subset の値正規化のみ**:
//! - 待ち時間 `\w[n]` / `\wN` / `\_w[ms]` → 単一 `Wait(Duration)`
//!   （`\w[n]`/`\wN` = n × 50ms、`\_w[ms]` = 絶対 ms。要件 3）。
//! - 改行 `\n` / `\n[percent]` / `\n[half]` → `NewLine(NewLineRatio)`
//!   （素の `\n` = 1.0、`percent`/100、`half` = 0.5、負値は符号付き保持。要件 4）。
//! - 選択肢 `\q[disp,target,refs...]` → `Choice`（disp/target 分離・要件 5.1/5.2）。
//! - キャラ移動 `\![move,...]` → `Move(MoveArgs)`（引数区切りのみ・要件 7.1）。
//! - 話者スコープ → `SpeakerScope`: 角括弧付き `\p[n]`、括弧なし `\pN`（0〜9・shorthand）、
//!   および正典 bare 形 `\0`/`\h`（本体側=0）・`\1`/`\u`（相方側=1）を写像（ukadoc・R1.5/R4.4）。
//!   サーフェス `\s[...]` → `Surface`（無加工保持）、
//!   カーソル `\_l[x,y]` → `Cursor`、制御 `\e`/`\c`/`\-` → `End`/`Clear`/`Quit`（要件 2/6）。
//! - システム変数 `%keyword` → `SystemVar`（展開なし・要件 8）、テキスト → `Text`（要件 9）。
//!
//! **スコープ境界（タスク 4.1 / 4.2 シーム）**: 本ファイルは emo2 subset の
//! 値正規化のみを行う。subset 外タグ・`move` 以外の `\!`・`\q` 旧 2 連形・
//! `\![*]` マーカー・不正トークンの吸収（`GenericCommand` / `Raw`）は
//! **タスク 4.2 の領分**であり、ここでは `decode_passthrough` という明示的な
//! シーム関数へ委ねる（現状は最小プレースホルダ。4.2 がここを実装する）。

// `decode` は `parse`（タスク 5）から非テスト経路で結線済み。タスク 4.2 の
// シーム関数（`decode_passthrough_*`）も `decode_token` / `decode_tag` / `decode_bang`
// 経由で到達されるため、モジュール全体の dead_code 抑止は不要（除去済み）。

use super::lexer::Token;
use super::model::{Choice, Instruction, MoveArgs, NewLineRatio, SurfaceArg};
use std::iter::Peekable;
use std::time::Duration;
use std::vec;

/// 1 ウェイト単位（`\w[n]` / `\wN` の n に乗ずる基準。ukadoc 確定: 50ms）。
const WAIT_UNIT_MS: u64 = 50;

/// 構文トークン列を値正規化済みの `Instruction` 列へ写像する（mod 内・`parse` が結線）。
///
/// - 入力 `tokens` は lexer 出力（構文区切り済み）。
/// - 全 `Token` がいずれかの `Instruction` へ写像され、未デコード文字列断片は残さない
///   （要件 1.2）。出力順は入力順（要件 1.3）。
/// - 失敗しない（`Vec` を返す・`Result` でない・要件 10.2）。
///
/// **シーケンス認識（タスク 4.2）**: 大半のトークンは 1:1 写像だが、
/// 一部は隣接トークンを見て局所的に畳む（design L421）:
/// - 旧 2 連 `\q[ID][タイトル]`（`Tag{q|q*,[1]}` ＋ 直後 `Text("[...]")`）→ 単一 `Raw`
///   （Choice 化せず丸ごと吸収・要件 5.3）。
/// - 選択肢マーカー `\![*]`（`Tag{!,["*"]}`）＋ 直後の現行 `\q[...]` → マーカーを
///   Choice へ畳む（要件 5.4）。単独なら `GenericCommand`。
///
/// そのため `Vec<Token>` を peekable に走査する（`map` でなく明示ループ）。
pub(crate) fn decode(tokens: Vec<Token>) -> Vec<Instruction> {
    let mut out = Vec::with_capacity(tokens.len());
    let mut it = tokens.into_iter().peekable();
    while let Some(token) = it.next() {
        match token {
            // 選択肢マーカー `\![*]`: 直後が現行 `\q[...]` なら畳んで Choice、単独なら
            // GenericCommand（要件 5.4・design L425）。
            Token::Tag { word, args } if is_choice_marker(&word, &args) => {
                out.push(fold_choice_marker(&mut it));
            }
            // 旧 2 連 `\q[ID][タイトル]` / `\q*[...]`: 直後 `Text("[...]")` を併せて
            // 単一 `Raw` に吸収（Choice 化しない・要件 5.3）。
            Token::Tag { word, args } if is_legacy_q_head(&word, &args, it.peek()) => {
                out.push(fold_legacy_q(&word, &args, &mut it));
            }
            other => out.push(decode_token(other)),
        }
    }
    out
}

/// `\![*]`（選択肢マーカー）か判定する: word=`!`・引数がちょうど `["*"]`（要件 5.4）。
fn is_choice_marker(word: &str, args: &[String]) -> bool {
    word == "!" && args.len() == 1 && args[0] == "*"
}

/// 選択肢マーカー `\![*]` を解決する。直後が現行 `\q[...]`（`Tag{q,..}`）なら
/// その `\q` を Choice として消化し、マーカーを畳んで未デコード文字列を残さない
/// （要件 5.4）。直後が現行 `\q` でなければ単独マーカー＝`GenericCommand { name:"*" }`。
fn fold_choice_marker(it: &mut Peekable<vec::IntoIter<Token>>) -> Instruction {
    if let Some(Token::Tag { word, .. }) = it.peek()
        && word == "q"
    {
        // 現行 `\q[...]`（単一ブラケット）を消化して Choice 化（マーカーを吸収）。
        if let Some(Token::Tag { args, .. }) = it.next() {
            return decode_choice(args);
        }
    }
    // 単独 `\![*]`: 種別 `*` の汎用コマンド。
    Instruction::GenericCommand {
        name: "*".to_string(),
        raw_args: Vec::new(),
    }
}

/// 旧 2 連 `\q` の先頭タグか判定する。
///
/// lexer は `\q[ID][タイトル]` を `Tag{q,[ID]}` ＋ `Text("[タイトル]")` に分割し、
/// `\q*[ID][タイトル]` を `Tag{q*,[ID]}` ＋ `Text("[タイトル]")` に分割する（実トークン形）。
/// 旧形の弁別条件:
/// - word が `q`（現行形と同綴）または `q*`（旧専用綴り）。
/// - 直後トークンが `Text` で、`[` 始まり `]` 終わり（宙に浮く 2 個目の `[...]`）。
///
/// `q*` は word 自体が現行 `q` と異なるため、後続 Text が無くとも現行 Choice には
/// ならない（`decode_tag` の subset 外 → Raw 経路へ）。本判定が拾うのは「現行 `q` の
/// 綴りだが直後に浮く `[...]` がある」旧形と、「`q*` ＋ 浮く `[...]`」旧形の双方。
fn is_legacy_q_head(word: &str, args: &[String], next: Option<&Token>) -> bool {
    (word == "q" || word == "q*")
        && args.len() == 1
        && matches!(next, Some(Token::Text(t)) if t.starts_with('[') && t.ends_with(']'))
}

/// 旧 2 連 `\q` を単一 `Raw` へ畳む。先頭タグ（`\q[ID]` / `\q*[ID]`）と直後の浮く
/// `Text("[タイトル]")` を結合し、元の見かけを復元した `Raw` を 1 個だけ産む
/// （Choice 化しない・情報を失わない・要件 5.3）。
fn fold_legacy_q(
    word: &str,
    args: &[String],
    it: &mut Peekable<vec::IntoIter<Token>>,
) -> Instruction {
    // 先頭タグの概形（`\q[ID]` 等）を復元。
    let mut raw = reconstruct_tag(word, args);
    // 直後の浮く `Text("[...]")` を取り込む（判定済みなので必ず存在する）。
    if let Some(Token::Text(t)) = it.next() {
        raw.push_str(&t);
    }
    Instruction::Raw(raw)
}

/// 単一の構文トークンを `Instruction` へ写像する。
///
/// emo2 subset（本タスク 4.1）に該当するものはここで値正規化し、それ以外は
/// `decode_passthrough`（タスク 4.2 のシーム）へ委ねる。
fn decode_token(token: Token) -> Instruction {
    match token {
        Token::Text(s) => Instruction::Text(s),
        Token::SysVar(keyword) => Instruction::SystemVar(keyword),
        // 短縮形 `\<word>N` の意味写像は語で分岐する。待ち `\wN` = n × 50ms（既存等価）。
        Token::Shorthand { word: 'w', n } => Instruction::Wait(wait_units(n as u64)),
        // バルーン面短縮 `\bN`（要件 1.2/1.3）: 1 桁の面数字を不透明文字列で保持し
        // `BalloonSurface` へ。`\b[N]` ブラケット形の第 1 引数と対称（数値化しない）。
        Token::Shorthand { word: 'b', n } => {
            Instruction::BalloonSurface(SurfaceArg::new(n.to_string()))
        }
        // 話者スコープ括弧なし短縮 `\pN`（ukadoc `\pID`＝任意番号・0〜9 のみ・R1.5/R4.4）:
        // 単一桁を scope 番号として `SpeakerScope{n}` へ。`\p[n]` ブラケット形（`decode_tag`
        // の `"p"` アーム）と等価。lexer が 1 桁のみ消費するため `\p10` = scope 1 ＋ Text("0")。
        Token::Shorthand { word: 'p', n } => Instruction::SpeakerScope { n: n as u32 },
        // 対象外の短縮語（`SHORTHAND_WORDS` の想定外拡張時）への防御。lexer は現状
        // `'w'`/`'b'`/`'p'` のみ産むため到達不能だが、panic せず生情報を失わない `Raw` に留める。
        Token::Shorthand { word, n } => Instruction::Raw(format!("\\{word}{n}")),
        Token::Bare(word) => decode_bare(&word),
        Token::Tag { word, args } => decode_tag(word, args),
        // タスク 4.2 のシーム: 構文上区切れたが正準でない／不正な生保持。
        Token::Raw(s) => decode_passthrough_raw(s),
    }
}

/// bare タグ（角括弧なし 1 文字）を写像する。
///
/// `\n`（bare）は既定比率 1.0 の改行（要件 4.2）。`\e`/`\c`/`\-` は制御命令（要件 6.2-6.4）。
///
/// **正典スコープタグ（bare 形・R1.5/R4.4）**: ukadoc により `\0`/`\h` = 本体側
/// （scope 0）、`\1`/`\u` = 相方側（scope 1）。これらを `SpeakerScope{n}` へ写像する
/// （角括弧付き `\p[n]` と等価な話者スコープ切替）。これは完了 spec
/// `areka-P0-sakura-parse` req 11.2（bare スコープタグを「M1 未使用 → passthrough」と
/// 分類）の**意図的なスーパーセット更新**であり、非 emo2 ゴースト・bare タグ経路の
/// fixture（`boot.pasta:79` の `\1\![move,...]` 等）が正しく動くようにする（emo2 自身は
/// `\p[n]` を発行するため無影響）。
fn decode_bare(word: &str) -> Instruction {
    match word {
        "e" => Instruction::End,
        "c" => Instruction::Clear,
        "-" => Instruction::Quit,
        "n" => Instruction::NewLine(NewLineRatio::new(1.0)),
        // 正典スコープタグ bare 形（R1.5/R4.4・ukadoc）: `\0`/`\h`=本体側、`\1`/`\u`=相方側。
        "0" | "h" => Instruction::SpeakerScope { n: 0 },
        "1" | "u" => Instruction::SpeakerScope { n: 1 },
        // 上記以外の subset 外 bare タグ（`\i` `\j` 等）はタスク 4.2 のパススルー領分。
        other => decode_passthrough_bare(other),
    }
}

/// 正準タグ `\word[args]` を写像する。
///
/// word ごとに emo2 subset の値正規化を施す。subset 外 word はタスク 4.2 のシームへ。
fn decode_tag(word: String, args: Vec<String>) -> Instruction {
    match word.as_str() {
        // 待ち時間（要件 3.1）: `\w[n]` = n × 50ms。
        "w" => Instruction::Wait(wait_from_arg(args.first())),
        // 待ち時間（要件 3.3）: `\_w[ms]` = 絶対 ms。
        "_w" => Instruction::Wait(wait_absolute_ms(args.first())),
        // 改行（要件 4.1）: `\n[percent]` = percent/100、`\n[half]` = 0.5。
        "n" => Instruction::NewLine(newline_ratio_from_arg(args.first())),
        // 話者スコープ（要件 2.1）: `\p[n]`。
        "p" => Instruction::SpeakerScope {
            n: speaker_scope_n(args.first()),
        },
        // サーフェス（要件 2.2/2.3）: `\s[...]` 中身は不透明文字列で無加工保持。
        "s" => Instruction::Surface(SurfaceArg::new(args.into_iter().next().unwrap_or_default())),
        // バルーン面切替（balloon-face-cue 要件 1.1/1.4/1.5）: `\b[...]` は `\s` と完全対称。
        // **第 1 引数のみ**を不透明保持する（fallback 形 `\b[2,--fallback=4]` は `"2"` に
        // 落ちる＝graceful）。数値化・範囲展開・alias 解決・`-1` 解釈は一切行わない。
        "b" => Instruction::BalloonSurface(SurfaceArg::new(
            args.into_iter().next().unwrap_or_default(),
        )),
        // カーソル絶対位置（要件 6.1）: `\_l[x,y]`（x/y は文字列のまま保持）。
        "_l" => decode_cursor(args),
        // 選択肢（要件 5.1/5.2）: `\q[disp,target,refs...]`。
        "q" => decode_choice(args),
        // `\!` コマンド（要件 7.1）: 第 1 引数が `move` のみ本タスクで Move へ decode。
        // move 以外（要件 7.2/7.3）はタスク 4.2 の GenericCommand 領分。
        "!" => decode_bang(args),
        // subset 外タグ（`\i` `\j` 等）はタスク 4.2 のパススルー領分。
        _ => decode_passthrough_tag(word, args),
    }
}

/// `\_l[x,y]` → カーソル絶対位置。x/y は文字列のまま保持（要件 6.1・design 値定数）。
fn decode_cursor(args: Vec<String>) -> Instruction {
    let mut it = args.into_iter();
    let x = it.next().unwrap_or_default();
    let y = it.next().unwrap_or_default();
    Instruction::Cursor { x, y }
}

/// `\q[disp,target,refs...]` → disp/target 分離 ＋ 追加 references（順序保持・要件 5.1/5.2）。
fn decode_choice(args: Vec<String>) -> Instruction {
    let mut it = args.into_iter();
    let disp = it.next().unwrap_or_default();
    let target = it.next().unwrap_or_default();
    let references: Vec<String> = it.collect();
    Instruction::Choice(Choice {
        disp,
        target,
        references,
    })
}

/// `\![...]` の第 1 引数で分岐する。`move` のみ本タスクで `Move` へ decode（要件 7.1）。
///
/// `move` 以外（および空）はタスク 4.2 のシーム（`GenericCommand`）へ委ねる。
fn decode_bang(args: Vec<String>) -> Instruction {
    match args.first().map(String::as_str) {
        Some("move") => Instruction::Move(MoveArgs {
            args: args.into_iter().skip(1).collect(),
        }),
        // move 以外の `\!`（要件 7.2/7.3）・`\![*]` マーカー（要件 5.4）はタスク 4.2 領分。
        _ => decode_passthrough_bang(args),
    }
}

/// n ウェイト単位を `Duration` へ（`\w[n]` / `\wN` = n × 50ms・要件 3.1/3.2）。
fn wait_units(n: u64) -> Duration {
    Duration::from_millis(n.saturating_mul(WAIT_UNIT_MS))
}

/// `\w[n]` の引数 n（10 進）から待ち時間を求める。引数欠落・非数は 0 ウェイト。
fn wait_from_arg(arg: Option<&String>) -> Duration {
    let n = arg.and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
    wait_units(n)
}

/// `\_w[ms]` の引数（絶対ミリ秒・要件 3.3）から待ち時間を求める。引数欠落・非数は 0ms。
fn wait_absolute_ms(arg: Option<&String>) -> Duration {
    let ms = arg.and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
    Duration::from_millis(ms)
}

/// `\n[percent]` / `\n[half]` から比率を求める（要件 4.1・design 値定数）。
///
/// - `half` → 0.5。
/// - 数値 percent → percent / 100（`150` → 1.5・負値は符号付き保持）。
/// - 引数欠落・非数 → 既定 1.0（素の `\n` と同等）。
fn newline_ratio_from_arg(arg: Option<&String>) -> NewLineRatio {
    match arg.map(String::as_str) {
        Some("half") => NewLineRatio::new(0.5),
        Some(s) => match s.parse::<f32>() {
            Ok(percent) => NewLineRatio::new(percent / 100.0),
            Err(_) => NewLineRatio::new(1.0),
        },
        None => NewLineRatio::new(1.0),
    }
}

/// `\p[n]` の話者スコープ番号（要件 2.1）。引数欠落・非数は 0。
fn speaker_scope_n(arg: Option<&String>) -> u32 {
    arg.and_then(|s| s.parse::<u32>().ok()).unwrap_or(0)
}

// ───────────────────────────────────────────────────────────────────
// タスク 4.2 寛容パススルー規則（要件 10/11.2/13.8）
//
// emo2 subset 外タグ・`move` 以外の `\!`・不正トークンの吸収先。`move` 以外の
// `\!` は `GenericCommand`（種別＋生引数保持）、それ以外（subset 外正準タグ・
// subset 外 bare＝**正典スコープ bare `\0`/`\1`/`\h`/`\u` を除く**・lexer Raw 断片）は
// 生情報を失わない `Raw` へ。いずれもエラーを
// 送出せず解析を中断しない（要件 10.2）。`\q` 旧 2 連形・`\![*]` マーカーは
// シーケンス依存ゆえ `decode` の peekable 走査側で捕捉する（fold_* 群）。
// ───────────────────────────────────────────────────────────────────

/// 【タスク 4.2】subset 外の正準タグ（`\i` `\q*[..]` 等）→ 構文区切りのまま `Raw`
/// 保持（要件 11.2/13.8）。生情報を復元して失わない。
/// なお `\b2[x]`（数字直後が `[` の非短縮ブラケット word `b2`）もここへ落ちる。
fn decode_passthrough_tag(word: String, args: Vec<String>) -> Instruction {
    Instruction::Raw(reconstruct_tag(&word, &args))
}

/// 【タスク 4.2】`move` 以外の `\!` → `GenericCommand { name, raw_args }`（要件 7.2/7.3）。
///
/// 第 1 引数をコマンド種別 `name`、残りを生引数 `raw_args` として保持する
/// （意味解釈・構造化はしない・design L426/L517）。引数が空（`\![]`）の場合は
/// 種別空・生引数空の汎用コマンドとし、情報を捨てずエラーも送出しない（要件 10.2）。
///
/// なお `\![*]` 選択肢マーカー（要件 5.4）は `decode` のシーケンス走査で先に
/// 捕捉されるため、本関数へは到達しない（到達しても種別 `*` の GenericCommand となり破綻しない）。
fn decode_passthrough_bang(args: Vec<String>) -> Instruction {
    let mut it = args.into_iter();
    let name = it.next().unwrap_or_default();
    let raw_args: Vec<String> = it.collect();
    Instruction::GenericCommand { name, raw_args }
}

/// 【タスク 4.2】subset 外の bare タグ（`\i` `\j` 等・**スコープタグを除く**）→ 生情報を
/// 保持して `Raw`（要件 11.2）。正典スコープ bare 形 `\0`/`\1`/`\h`/`\u` は `decode_bare` で
/// `SpeakerScope` へ写像されるため、ここへは到達しない（R1.5/R4.4）。
fn decode_passthrough_bare(word: &str) -> Instruction {
    Instruction::Raw(format!("\\{word}"))
}

/// 【タスク 4.2】lexer が区切れず `Raw` 吸収した不正断片（未閉じ `[`/`"`）→ そのまま
/// `Raw` で保持（要件 10.1/13.8）。decode 側で意味を詐称しない。
fn decode_passthrough_raw(s: String) -> Instruction {
    Instruction::Raw(s)
}

/// `Raw` プレースホルダ用に `\word[args]` を概形再構成する（情報を失わないため）。
fn reconstruct_tag(word: &str, args: &[String]) -> String {
    if args.is_empty() {
        format!("\\{word}")
    } else {
        format!("\\{word}[{}]", args.join(","))
    }
}
