# 設計バリデーションレポート — areka-P0-sakura-bare-tag-lexer

> 実施日: 2026-09-02 ／ 実施形態: 非対話（kiro-validate-design subagent・レポートはディスクへ永続化）
> 対象: ワークツリー `claude/sakura-bare-tag-lexer-bbdf8c`・HEAD `0bc2f472`（design 生成コミット）
> 入力: spec.json（language=ja・phase=design-generated）／requirements.md（確定）／design.md（確定）／research.md（§4 実装案評価・§9 要件ディスカッション裁定・§10 設計フェーズ記録）／steering（product・tech・structure・roadmap・logging・workflow）
> 検証方法: design-review プロセス（Analysis → Critical Issues → Strengths → GO/NO-GO）＋**設計が引用する file:line を実ツリーで全件突合**（design.md の記述を鵜呑みにせず、lexer.rs／decode.rs／mod.rs／lexer_tests.rs／parse.rs／compile.rs／COMPAT_ARCHITECTURE.md／anchor-tag-canon brief／file_length_guard_test.rs／emo2 フィクスチャを直接読んだ）

---

## 1. レビューサマリ

本設計は、字句層の角括弧なし経路の消費長を「`\_` ＋（`_` 0〜1 個）＋ 1 文字」の固定規律に置き換え（純粋関数 `bare_tag_len`）、`Token::Bare` の載荷を `char` → `String` に広げ、意味層は既知 8 語の写像を綴りで引き直すだけで未知語を既存の `Instruction::Raw` へ素通しする、という小さく閉じた是正である。**設計が引用する file:line は全件が実ファイルと一致し、`Token::Bare` の構築／照合箇所（本番 3 行＋テスト 11 行）と decode 経路（`decode_bare`・`decode_passthrough_bare`）の列挙に漏れはない。** 消費規律の決定表は依頼された全境界（`\_` 末尾・`\__` 末尾・`\___x`・直後に本文／`\`／`%`・角括弧形不変・既知 1 文字タグ不変・短縮形不変・末尾裸 `\`）を手作業でトレースして正しいことを確認した。ブロッキングとなる欠陥は無く、設計ディスカッションで詰めるべき整合点が 3 件（うち 1 件は steering との食い違い）ある。

## 2. 実ツリー突合（全項目）

| 設計の主張 | 実ツリー確認結果 | 判定 |
|---|---|---|
| `lexer.rs`（285 行）`:35` `Bare(char)`・`:47-48` `Raw` の説明・`:128-130` 末尾裸 `\` → `Token::Bare('\\')`・`:132` 「空白」に言及する注記・`:142-149` 短縮形判定・`:152-157` ワード走査（停止は `[`／`\`／`%` のみ）・`:159` `word` 確定・`:162-171` 角括弧経路・`:172-177` 角括弧なし経路（`let bare = first; (Token::Bare(bare), word_start + 1)`） | 全て逐語一致。ワード走査に空白判定は無い（`:153` の条件は 3 文字のみ）＝`:132` の注記は確かに実装と食い違う | 一致 |
| `decode.rs`（348 行）`:156` `Token::Bare(c) => decode_bare(c)`・`:163` 「角括弧なし 1 文字」・`:174-186` `decode_bare(c: char)`（既知語 `e c - n 0 h 1 u`・`:184` `other`）・`:196` `"_w"` → `Wait`・`:212` `"_l"` → `Cursor`・`:219` catch-all・`:328-333` `decode_passthrough_bare(c: char)` → `Raw(format!("\\{c}"))`・`:337-339`・`:342-348` `reconstruct_tag` | 全て一致。`is_choice_marker`／`is_legacy_q_head`（`:77-115`）は `Token::Tag` しか見ないので `Bare` の型変更に無関係（設計 §10.3 ⑶ のとおり） | 一致 |
| `Token::Bare` の全構築／照合箇所 | grep 実測: 本番は `lexer.rs:35`（型）`:129`（末尾裸 `\`）`:176`（角括弧なし経路）＋ `decode.rs:156`（照合）の 4 行、テストは `lexer_tests.rs` の **11 行**（`:92` `:93` `:94` `:100` `:161` `:163` `:174` `:229` `:323` `:436` `:454`）。`decode_tests.rs`・`parse_tests.rs`・`validation_tests.rs` に直接構築は無い（`decode_tests.rs:154` は doc コメントでの言及のみ） | 設計の列挙と完全一致・漏れなし |
| `mod.rs`（36 行）`:17-20`・`:27-33`、`parse.rs:13-14`（失敗しない契約）・`:28-30` | 一致 | 一致 |
| `compile.rs`（329 行）`:61-65` `Wait` → 実 cue・`:137-149` `Cursor` → 実 cue・`:202-203` catch-all（`Raw` を `tracing::debug!` で無視） | 一致。加えて **`Instruction::Raw` を照合する非テストコードは他クレートに存在しない**（`areka-ghost/src/sink.rs`・`areka-sakura/src/drive.rs`・`contract.rs` は `Instruction` の variant を照合しない）＝要件 3.3「表示にも待ちにも変換しない」の成立条件は compile.rs の catch-all 1 か所で閉じる | 一致 |
| 案 B 却下の根拠（角括弧なし `\_w`／`\_l` を `Tag{word, args: []}` で流すと `Wait(0ms)`／`Cursor{"",""}` の実 cue が出る） | `decode.rs:196`・`:212` → `compile.rs:61-65`・`:137-149` で実証可能。設計 P5 が「`\_w`・`\_l` の角括弧なし形が `Raw` になる」ことを固定するのは、この副作用が入っていない証拠として適切 | 一致 |
| `doc/COMPAT_ARCHITECTURE.md`（216 行）`:122` §8 見出し・`:126-127` 表ヘッダ・`:207` 表の最終行 | `:207` はパイプ 5 本（4 欄）の表行、`:208` は空行、`:209` から性能目標の小節。`:207` 直後への 1 行挿入は表を壊さない。設計の登記行本文にエスケープを要する `\|` は無い | 一致 |
| `.kiro/specs/areka-P0-anchor-tag-canon/brief.md`（62 行）`:59-62` の棚卸⑫ブロック・`:60` が登記位置と文言を自ら指定 | 一致（`:60` の指定文言は「lexer 修正は消化済み（**PR#**）」＝§3 Issue 3 参照） | 一致 |
| `crates/log-capture-kit/tests/file_length_guard_test.rs` `:61-109` 例外表 11 件・`OVER_LIMIT_ALLOWED_COUNT = 11`・`areka-parsers` は 0 件 | 一致。新設 2 ファイル（見積 250〜350／200〜300 行）と変更後の `lexer.rs`（約 300 行）は上限に遠く、例外表を触らずに済む | 一致 |
| emo2 フィクスチャは角括弧なし `\_` を使わない（要件 4.7） | `dic/*.pasta` の `\_` 出現は **4 行・全て角括弧付き**: `menu.pasta:15` `:33` `:62` の `\_l[5em,2lh]`、`update.pasta:44` の `\_w[600]`。⚠ research.md §10.2 は「`:62` は該当なし・`\_l` のみ」と書くが実測では `:62` も存在し `\_w[600]` もある——**いずれも角括弧付きなので結論（4.7 は構造的に成立）は変わらない**。research の記録訂正を推奨（設計本文には影響しない） | 結論一致（research の記述に軽微な誤り） |

### 消費規律の決定表トレース（`bare_tag_len`・設計 Components「Service Interface」）

| 入力 | ワード走査結果 `word`（`n`） | 設計の返り値 | 生成トークン列 | 判定 |
|---|---|---|---|---|
| `\_`（末尾） | `_`（1） | `min(2,1)=1` | `[Bare("_")]` → `Raw("\\_")`（現行と同じ） | 正 |
| `\__`（末尾） | `__`（2） | `min(3,2)=2` | `[Bare("__")]` → `Raw("\\__")` | 正 |
| `\___x` | `___x`（4） | `3` | `[Bare("___"), Text("x")]`（要件 1.1b） | 正 |
| `\_a本文` | `_a本文`（4） | `2` | `[Bare("_a"), Text("本文")]`——**ワード長 4 を使わない**ことが要件 2 の要 | 正 |
| `\_a\e`／`\_a%username` | `_a`（2・`\`／`%` で停止） | `2` | `[Bare("_a"), Bare("e")]`／`[Bare("_a"), SysVar("username")]` | 正 |
| `\_\e`／`\__\e` | `_`（1）／`__`（2） | `1`／`2` | `[Bare("_"), Bare("e")]`／`[Bare("__"), Bare("e")]` | 正 |
| `\_a[Hint]`・`\__q[OnTest]`・`\_[x]` | 直後が `[` → 角括弧経路（`:162`）で確定済み・本経路に来ない | — | `Tag{"_a",["Hint"]}` 等（不変・要件 1.8） | 正 |
| `\eあ`・`\iい`（既知／未知 1 文字） | `eあ`（2） | 先頭が `_` でない → `1` | `[Bare("e"), Text("あ")]`（要件 4.1／4.2） | 正 |
| `\w2`・`\b3`・`\p1` | 短縮形判定（`:142-149`）が本経路の手前で確定 | — | `Shorthand`（不変・要件 4.3） | 正 |
| `\`（末尾裸） | `chars.get(i+1)` が `None` → `:129` | — | `Bare("\\")` → `decode_bare("\\")` は既知 8 語に無いので `Raw(format!("\\{word}"))` = `\\`（2 文字）＝現行 `Raw(format!("\\{c}"))` と逐語同一 | 正（挙動不変） |

`take <= n` が決定表で保証されるため `chars[word_start..word_start + take]` は常に範囲内（`word` 自体が `chars[word_start..j]` で `n = j - word_start`）。`take >= 1` で `lex` は必ず前進する（要件 7.2）。

## 3. Critical Issues

ブロッキング（NO-GO 相当）の issue は**なし**。設計ディスカッションで確定すべき整合点を 3 件登記する。

### 🔴 Critical Issue 1（重要度: 中）— 新設テストモジュールの接続形が steering の規約と食い違う

- **Concern**: design.md「File Structure Plan」「Modified Files」は新設 2 ファイルを `mod.rs` に `#[cfg(test)] mod lexer_bare_tag_tests;`／`#[cfg(test)] mod parse_bare_tag_tests;` で接続するとし、脚注で「同ディレクトリの既存兄弟（`lexer_tests` 等）と同じく `mod.rs` で接続」と述べる。しかし `.kiro/steering/structure.md`「Unit Tests」節は**新規のテストモジュール**について「本番ファイル側にパス属性つきの接続宣言だけを残す」形——`#[cfg(test)] #[path = "<stem>_<モジュール名>.rs"] mod <モジュール名>;` を本番ファイル末尾に置く——を正本と定め、歴史的形式は「既存のものはそのまま維持するが、新規には使わない」としている。research.md §5.1 が根拠に挙げる structure.md `:167` は「歴史的形式のテストファイルも stem 候補に含める」という逆引き規則であり、新規接続に歴史的形式を使ってよいとは書いていない（誤読）。
- **Impact**: ファイル名（`lexer_bare_tag_tests.rs`／`parse_bare_tag_tests.rs`）は規約どおりで変える必要が無いが、接続宣言の置き場と `mod` 名が規約から外れる。レビュアーが steering 違反として差し戻す形になり、実装後の手戻りになる。
- **Suggestion**: 接続を `lexer.rs` 末尾に `#[cfg(test)] #[path = "lexer_bare_tag_tests.rs"] mod bare_tag_tests;`、`parse.rs` 末尾に `#[cfg(test)] #[path = "parse_bare_tag_tests.rs"] mod bare_tag_tests;` と置く形へ改め、`mod.rs` を Modified Files から外す（`lexer.rs` は約 300 行のまま上限に影響しない）。テスト側の `use super::…` は `super` が `lexer`／`parse` になるので `use super::{Token, lex};`／`use super::parse;`＋`use super::super::model::Instruction;` 相当へ読み替える（設計の import 記述もこれに合わせて直す）。歴史的形式を意図的に踏襲するなら、その裁定を設計に明記して開発者の了承を取る。
- **Traceability**: 要件 5.1〜5.8（テスト新設）・7.4（範囲外ファイルを編集しない＝`mod.rs` の要否）
- **Evidence**: design.md「File Structure Plan」ツリー・「Modified Files」`mod.rs` 行・「New Files」脚注・research.md §5.1／§10.4

### 🔴 Critical Issue 2（重要度: 低〜中）— 変異手順 ⑵ の定義と「赤になるべきテスト」の一覧が実装者の裁量に残る

- **Concern**: 「変異手順」は ⑵ を「消費長を `word.chars().count()`（ワード全体）にして実行。期待: T5・T11・T12・P2・P3・P10 が赤」とする。しかし `bare_tag_len` を丸ごと `word.chars().count()` に置き換えると、先頭が `_` でない経路も巻き込まれ **T13**（`\eあ` → `Bare("eあ")`）と **T9**（`\___x` → `Bare("___x")`）も赤になる。一方 `_` 分岐だけを変異させれば T13 は緑のままである。どちらの変異を指すかで「期待される赤」が変わり、実装者は自分の結果に合わせて一覧を後付けできてしまう（較正にならない）。⑴ についても T3・T8・P6 の一部（`\_` 単独）は変異後も緑になる点が明記されていない。
- **Impact**: 要件 5.5／5.6 の「戻すと落ちる対」の証跡が、事前に固定された期待ではなく事後の観測になる。記憶「検証の道具そのものが壊れる・較正せよ」「摂動は平行移動でなく経路から外す」の趣旨に反し、赤が出なかったときに欠陥を見逃す。
- **Suggestion**: 変異を関数本体の置換として一意に定義する——⑴ `bare_tag_len` の本体を `1` に、⑵ 本体を `word.chars().count()` に。期待される赤を**完全列挙**で書き直す（⑴: T1・T2・T4・T5・T6・T7・T9・T11・T12・P1〜P6 が赤、T3・T8・T10・T13〜T15・P7〜P10 は緑／⑵: T5・T9・T11・T12・T13・P2・P3・P10 が赤、T1・T2・T4・T8・P1・P5・P6 は「タグが末尾にあるため偶然通る」と注記）。「一覧に無い赤／一覧にある緑」が出たら実装記録に理由を残す規律も添える。
- **Traceability**: 要件 5.5・5.6・5.7
- **Evidence**: design.md「Testing Strategy」→「変異手順（要件 5.5／5.6・実装タスクで実施し結果を記録する）」手順 2・3

### 🔴 Critical Issue 3（重要度: 低）— 隣接仕様への登記行の「取り込み番号」の解釈と根拠づけ

- **Concern**: 要件 6.2 は「取り込み番号付きで登記」を求め、発見元 brief `:60` は「本 brief に『lexer 修正は消化済み（**PR#**）』を登記」と PR 番号を想定している。設計は取り込み番号を「brief 内の消化済み通し番号（消化済み①）」と読み替え、PR 番号は「完了 PR（squash）で確定」と書くのみで後追い記入をしないと決めた。決定自体は妥当（squash 前に PR 番号は存在しない・spec 名で追跡できる）だが、根拠に挙げた「要件 6.3 により再編集を避ける」は正しくない——6.3 が禁じるのは**自らの登記行以外**の書き換えであり、自分の登記行の追記は禁じていない。
- **Impact**: 要件文の解釈を設計が黙って決めている状態。開発者が PR 番号の記入を期待していると、完了後に「登記が不完全」と受け取られる。
- **Suggestion**: 設計ディスカッションで開発者に明示的に選ばせる——(a) 設計案どおり spec 名を追跡キーにし PR 番号は書かない（根拠は「squash 前に採番されない」に改める）、(b) `/kiro-complete` の最終コミット前に PR 番号を自らの登記行へ追記する運用を採る（6.3 とは矛盾しない）。いずれでも要件本文は動かない。
- **Traceability**: 要件 6.2・6.3
- **Evidence**: design.md「登記」→「隣接仕様登記」第 1 項・research.md §10.5・anchor-tag-canon brief `:60`

### 申し送り（非ブロッキング・裁定不要）

- 要件 7.5「触れるファイルの中に実装と食い違う説明注記を残さない」は**触れるファイル全体**が射程である。設計の訂正一覧（`lexer.rs:8` `:34` `:125` `:132` `:173-174`・`decode.rs:163` `:328`）に加え、`lexer.rs:124`・`:133` が短縮対象語を「`\wN`/`\bN`」「`\w`/`\b`」とだけ書き `\p`（`SHORTHAND_WORDS` の 3 語目）を落としている。是正対象に含めるか、実装タスクで「触れるファイルを 7.5 の観点で一巡する」手順を置くと安全。
- research.md §10.2 の emo2 フィクスチャ行（`menu.pasta:15` `:33` のみ・`:62` 該当なし）は実測と異なる（`:62` も `\_l[5em,2lh]`・`update.pasta:44` に `\_w[600]`）。結論は変わらないが記録を直しておくと後続 spec が引用したときに迷わない。
- `lexer_tests.rs`「行数不変」は、置換後の最長行（`:174`・70 → 約 83 文字）が rustfmt 既定幅 100 に収まるため成立する見込み。

## 4. Strengths

1. **消費長の判断が純粋関数 1 個に閉じ、決定表が両向きの失敗を塞いでいる。** 「ワード長を消費長に使ってはならない」を契約に書き、`\_a[Hint]アンカー\_aをクリックする。`（T12／P2）を「戻すと落ちる・行き過ぎても落ちる」対の要として置いた設計は、要件 2 の逆向きの退行（台詞が消える）を構造とテストの両方で防いでいる。`min(want, n)` 1 式で `\_` 末尾・`\__` 末尾・直後の `\`／`%` を同時に満たす点も無駄が無い。
2. **境界の主張が実コードで裏付けられ、範囲が 1 バイトも膨らんでいない。** 案 B 却下は `decode.rs:196`／`:212` → `compile.rs:61-65`／`:137-149` の実経路で示され、`Instruction` に variant を足さず・`decode_bare` の既知語集合を動かさず・`Raw` の無視経路（`compile.rs:202-203`・他クレートに消費者なし）へ落とす、という 3 点で要件 3・7 の「意味を付けない」が下流まで通っている。引用 file:line は全件一致し、実装者が推測で埋める箇所は Issue 1〜3 以外に無い。

## 5. 最終判定

### GO

- **根拠**: 既存アーキテクチャ（`model ← lexer ← decode ← parse`・`pub(crate)` 閉包・`Instruction::Raw` 素通し・`parse` の失敗しない契約）との不整合は無く、要件 1〜7 の全受入基準が Traceability 表で具体コンポーネントとテストへ写像されている。設計が依拠する file:line と `Token::Bare` の影響箇所は実ツリーで全件一致し、消費規律の決定表は依頼された全境界で正しい。残る 3 件は接続宣言の形・変異手順の一意化・登記番号の解釈という**設計の骨格を変えない整合点**であり、設計ディスカッションで確定すれば実装に進める。
- **次のステップ**:
  1. `/kiro-design-discussion areka-P0-sakura-bare-tag-lexer` で Issue 1（接続宣言を `#[path]` 形へ）・Issue 2（変異の定義と赤の完全列挙）・Issue 3（取り込み番号の扱い）を裁定し、design.md の該当節を更新する。
  2. 申し送り 3 点（`lexer.rs:124`/`:133` の 7.5 対象化・research §10.2 の記録訂正・行数見込み）は同じ更新で拾う。
  3. 裁定反映後に `/kiro-spec-tasks areka-P0-sakura-bare-tag-lexer` へ進む。
