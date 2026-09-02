# Brief: areka-P0-sakura-bare-tag-lexer

> 起票: 2026-09-02（`/kiro-discovery` 再入・Path C・**S 規模**）。棚卸⑫（追記(88) ②⑴）が「spec を立てず直接修正 1 PR（⓪）」と推奨した挙動バグを、開発者の指摘「spec が無いと開始できない」を受けて S spec として起票した。ロードマップの **⓪ 行＝本 spec**。
> **種別**: 挙動バグ修正（parser の転記忠実性）。意味付け（アンカーや一気表示の実装）は行わない。

## Problem

さくらスクリプトの字句解析 `crates/areka-parsers/src/sakura/lexer.rs:152-177`（2026-09-02 実測）は、`\` の後ろのワードを `[`／`\`／`%` まで読み進めて `word` を作るのに、**角括弧が無い分岐では `first`（1 文字目）だけを `Token::Bare` として消費し、残りを本文へ返す**（`:175-176`＝`let bare = first; (Token::Bare(bare), word_start + 1)`）。その結果、2 文字の bare 形 **`\_a`・`\_q`・`\_n`・`\_s`・`\_v` など `\_X` 系すべて**で `_` だけが消費され、**`X` が本文の文字として漏れる**。

利用者から見える結果: `\_a[id]text\_a` は「text**a**」、`\_qすぐ出す\_q` は「**q**すぐ出す**q**」と表示される。里々／YAYA の辞書は `\_q`（一気表示）と `\_a`（アンカー）を多用するため、**既存ゴーストを入れると台詞に余計な文字が正常な顔で混ざる**＝toolkit 規則 6 の壊れ方で最上位（間違った結果を正常な顔で見せる）。emo2 は `\_X` bare 形を使わないので M1 適合には無害。**テストは 0 本**（bare 形・角括弧形いずれも）。

## Current State（2026-09-02 実測・着手時に再検証）

- 字句: `Token::{Tag{word,args}, Bare(char), Shorthand, SysVar, Text, Raw}`（`lexer.rs:34-59`）。`Bare` は `char` 1 文字型＝2 文字の bare 形を表せない。
- 意味写像: `sakura/decode.rs:174-186` が bare を `\e` `\c` `\-` `\n` `\0`/`\h` `\1`/`\u` へ写し、未知の bare は `Instruction::Raw` へ素通し（`:186`）。角括弧形 `\_a[...]` `\_q[...]` 等は `Tag` として `:220` で `Raw` 素通し。
- 下流: compile は M-boot 外タグを `debug!` で無視（`areka-sakura/src/compile.rs:203`）＝`Raw` は表示に出ない。**したがって字句が正しく 2 文字を 1 単位で切りさえすれば、漏れは消える**（意味付けは不要）。
- 所有宣言: `anchor-tag-canon` brief（`.kiro/specs/areka-P0-anchor-tag-canon/brief.md:60`）が本欠陥を発見し「先に直接修正で着地させる」と記録済み（着地後は同 brief に「消化済み」を登記し L→M）。

## Desired Outcome

1. `\_X` 形（`\_` に 1 文字以上のワードが続き角括弧が無い）を **1 単位のトークンとして全体を消費**し、本文へ文字を漏らさない。角括弧形 `\_X[...]` は現状どおり `Tag`。
2. 未知の bare 形は decode で `Instruction::Raw` へ **タグ全体の文字列**（`\_a` など）として素通し（現在の Raw 素通しの型を保つ）。既知の 1 文字 bare（`\e` `\c` `\-` `\n` `\0` `\1` `\h` `\u`）の挙動は不変。
3. 決定論テストを新設（現在 0 本）: bare 形 5 種（`\_a` `\_q` `\_n` `\_s` `\_v`）・角括弧形（`\_a[id]`・`\_q[..]`）・`\_` 単独で入力末尾・`\_a` 直後に本文／`\`／`%` が続く境界・既知 1 文字 bare の不変（変異＝修正を戻すと赤になる檻）。
4. `cargo test -p areka-parsers -p areka-sakura` 緑・ワークスペース全緑。

## Approach

設計で 2 案から選ぶ（どちらも挙動は同じ・型の変更範囲が違う）:

- ⒜ `Token::Bare(char)` を **`Token::Bare(String)`** へ（bare 分岐で `word` 全体を消費）。decode の bare 腕は 1 文字の既知語だけ意味写像し、他は `Raw(format!("\\{word}"))`。型変更が decode とテストへ波及するが表現が素直。
- ⒝ `Bare(char)` を残し、`word.len() > 1` のときだけ **`Token::Tag{word, args: vec![]}`** を返す。既存型を触らないが「角括弧なしの Tag」という例外が増える。

推奨は ⒜（記憶 canonical-not-minimal-lifecycle＝小細工より正規表現）。いずれも lexer の bare 分岐 1 か所＋decode の bare／passthrough 腕＋テスト新設で完結する。

## Scope

- **In**: `lexer.rs` の bare 分岐・`decode.rs` の bare／passthrough 腕・兄弟テスト新設（1,000 行未満）・`anchor-tag-canon` brief への「消化済み（PR#）」登記・COMPAT §8 に 1 行（「`\_X` bare 形は 1 単位で素通し・意味付けは各 spec」）。
- **Out**: `\_a` のアンカー意味付け（`anchor-tag-canon`）・`\_q` の一気表示（時間指令＝`sakura-time-directives` allowlist）・`\_l`／`\_b`／`\_v` の意味（各所有 spec）・lexer の他の分岐（短縮形・`%`・未閉じ吸収）。

## Boundary Candidates

- 字句（1 単位に切る）と意味（何をするか）の境界で切る。本 spec は字句のみ。

## Out of Boundary

- `decode.rs` の match 腕の追加（新しいタグの意味写像）は行わない。

## Upstream / Downstream

- **Upstream**: なし（現行 main で着手可）。
- **Downstream**: `ukadoc-survey-sakura-script`（`lexer.rs`／`decode.rs` の定義箇所へ ukadoc URL コメントを置く＝**本 spec 着地後に開始**）・`property-query-channels`（W14・`decode.rs`）・`text-decoration-canon`（W13・`decode.rs`）・`anchor-tag-canon`（W15・`"_a"` 腕）。**同じ 2 ファイルを触る後続 4 spec の rebase 源を消すため最初に着地させる**。

## Existing Spec Touchpoints

- **Extends**: なし（`anchor-tag-canon` の登記行を更新するだけ・brief 本文は書き換えない）。
- **Adjacent**: `anchor-tag-canon`（発見者・意味付けの所有）／`sakura-time-directives`（`\_q`）／`ukadoc-survey-sakura-script`（同ファイル・後着）。W12 の e2e／cursor-tag／toolkit とは共有ファイル 0。

## Constraints

- 挙動不変の範囲: 既知 1 文字 bare・短縮形・角括弧形・未閉じ吸収は 1 バイトも変えない（変異テストで担保）。
- `file_length_guard_test.rs` の例外表には触れない。
- 実装済みの証拠（ukadoc URL コメント）は置かない＝survey-sakura-script の仕事（規則 7）。
