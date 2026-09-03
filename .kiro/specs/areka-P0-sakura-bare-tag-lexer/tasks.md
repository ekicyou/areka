# Implementation Plan

> 対象: `areka-P0-sakura-bare-tag-lexer`（角括弧なし `\_` タグの切り分け是正）。
> 設計は `design.md`、受入基準は `requirements.md` を正とする。本書は「何を作るか」だけを書き、ファイル名・関数名・型名の詳細は設計側に委ねる。
> `(P)` は直前の同階層タスクと同時並行に着手してよい印。

- [x] 1. 基盤: 角括弧なしタグの「綴り」を載せられる形へ内部トークンを広げる（挙動不変）
  - 角括弧を伴わないタグを表す内部トークンの載荷を 1 文字から綴り（文字列）へ広げ、意味層がその綴りで既知タグを引き直すようにする
  - 既知タグの集合（`e` `c` `-` `n` `0` `h` `1` `u` の 8 語）と各々の写像先は 1 つも変えない。未知の綴りは既存の素通し形式へ、先頭の `\` を戻した文字列として落とす
  - 入力末尾に裸の `\` が来た場合の扱いを現行と同じ結果に保つ
  - 既存の字句テストの期待値を新しい載荷型へ機械的に追随させる（意味は変えない）
  - 観測: この段階では欠陥はまだ残っている（`\_a本文` は依然として本文へ `a` を漏らす）が、`cargo test -p areka-parsers -p areka-sakura` が緑で、表示結果は 1 文字も変わっていない
  - _Requirements: 3.2, 3.4, 4.1, 7.3_

- [x] 2. 中核: 消費長の固定規律を新設し、字句層の角括弧なし経路をそれへ差し替える
  - 消費長を決める純粋な判断を 1 か所に新設する。判断は綴りの先頭 1〜2 文字だけを見て決める: 先頭が `_` でなければ 1 文字、`_` なら 2 文字、2 文字目も `_` なら 3 文字。いずれも走査済みワードの実長で頭打ちにする
  - ワード走査の結果の長さを消費長に使ってはならないことを、この判断の説明として残す（走査は本文でも空白でも止まらないため、長さを使うと台詞を飲み込む）
  - 大文字と小文字を同一視しない。`_` が 3 個以上続く形を新しいタグ形として扱わない
  - 決めた消費長ぶんの文字を綴りとして切り出してトークンに載せ、次の読み取り位置を同じ長さだけ進める
  - 角括弧が続く場合の経路・短縮形の判定・ワード走査の停止条件・エスケープ処理には一切触れない
  - 観測: `\_a本文` が「綴り `_a`」＋「本文」に、`\__q本文` が「綴り `__q`」＋「本文」に分かれ、`\_a[Hint]` は従来どおり角括弧付きタグとして切り出される。`\_` や `\__` が入力末尾に来ても添字が範囲外にならず、`cargo test -p areka-parsers -p areka-sakura` が緑
  - _Requirements: 1.1, 1.1a, 1.1b, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.7a, 1.8, 2.4, 4.1, 4.2, 4.3, 7.2_

- [ ] 3. 決定論テストの新設と、2 方向の変異による実効性の確認
- [x] 3.1 (P) 字句層のテストを新設し、本番ファイルの末尾から接続する
  - 設計の T1〜T15 を実装する: 2 文字形 8 個と 3 文字形 4 個の全件・大小文字の区別・正典に無い組み合わせ・直後に本文／`\`／`%` が続く 3 境界を 2 形それぞれで・入力末尾の `\_` と `\__`・`_` 3 個以上の形・角括弧優先・全角半角混在・既知 1 文字タグ直後の本文・短縮形／エスケープ／クォート／未閉じ／引数分割の不変・末尾裸 `\`
  - テストファイルの命名と接続は steering の規約に従い、本番ファイル末尾にパス属性つきの宣言を置く（モジュール一覧ファイルは編集しない）
  - 観測: 新設テストが全件緑で通り、既存の字句テストも緑のまま
  - _Requirements: 1.1, 1.1a, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.7a, 1.8, 2.4, 2.5, 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 5.1, 5.3, 5.3a, 5.4, 5.7, 5.8_
  - _Boundary: 字句層テスト_
- [x] 3.2 (P) 通しのテストを新設し、本番ファイルの末尾から接続する
  - 設計の P1〜P10 を実装する: 開始形と終了形の対 4 組で表示本文が期待どおりであること・12 タグ各々が素通しの断片 1 個になり待ちにも表示にもならないこと・角括弧なしの `\_w`／`\_l` が待ちやカーソル移動にならないこと・未知の綴りと末尾の `\_`／`\__` で中断も例外も起きないこと・適合対象フィクスチャが使う角括弧付き形の意味が不変であること・既知 1 文字タグの意味が不変であること・全角半角混在の本文が逐語で残ること
  - 接続は 3.1 と同じ規約に従う
  - 観測: 新設テストが全件緑で、`\_q文字を瞬間表示する。\_q` の表示本文が「文字を瞬間表示する。」だけになる
  - _Requirements: 2.1, 2.2, 2.3, 2.5, 2.6, 3.1, 3.2, 3.3, 3.4, 3.5, 4.1, 4.6, 4.7, 5.1, 5.2, 5.3, 5.7, 5.8_
  - _Boundary: 通しテスト_
- [x] 3.3 2 方向の変異を実際に適用し、赤になったテストを実測して記録する
  - 変異 ⑴ は消費長の判断の本体を「常に 1」へ置き換える（元の欠陥の再現）。変異 ⑵ は同じ本体を「ワード全体の長さ」へ置き換える（読みすぎの再現）
  - どちらも設計が列挙した赤／緑の期待と実測を突き合わせ、食い違えばテスト側を補強する。変異はどちらも元へ戻し、再び全緑を確認する
  - 観測: 2 方向それぞれで赤になったテスト名の実測一覧が記録として残り、変異を戻した後の全テストが緑
  - _Depends: 3.1, 3.2_
  - _Requirements: 5.5, 5.6_

- [ ] 4. 説明注記の訂正と、互換記録・隣接仕様への登記
- [x] 4.1 触れたファイルの説明注記を、実装の実際の振る舞いに合わせる
  - ワード走査が「空白でも止まる」という実装と食い違う記述を、実際の停止条件（角括弧・バックスラッシュ・システム変数の開始）に直し、この走査長を消費長に使わない旨を添える
  - 「1 文字タグ」を前提にした説明を綴り単位の表現へ改め、短縮形の対象語の例を実装の 3 語と一致させる
  - 正典に合致していることを示す出典の注記や参照 URL は置かない（別仕様の担当）
  - 観測: 触れたファイルの中に、実装の振る舞いと食い違う説明が 1 か所も残っていない
  - _Requirements: 6.5, 7.5_
- [x] 4.2 (P) 互換記録の裁量表へ、この裁量を 4 欄 1 行で登記する
  - 項目・裁量・根拠・出典 spec の 4 欄で、消費規律と「意味を与えず素通しする」現在の状態を記す
  - 意味付けの所有は名指しの 2 件のみ引用し、残るタグは所有先未定であること・裁定は網羅ロードマップの無所有一覧に委ねることを明記する
  - 観測: 裁量表に 1 行だけ増え、既存行は 1 文字も変わっていない
  - _Requirements: 6.1, 6.4, 6.4a_
  - _Boundary: 互換記録_
- [x] 4.3 (P) 発見元仕様の記録へ、消化済みであることを取り込み番号付きで登記する
  - 末尾に 1 行だけ追記し、PR 番号の位置には完了時に記入する旨の仮置きを入れる（採番は完了手続きの段で行う 2 段登記）
  - 追記した 1 行以外は 1 文字も変えない。アンカーとしての意味付けの所有は発見元仕様に残したままにする
  - 観測: 発見元仕様の記録の差分が末尾 1 行の追加のみで、他の行に変更が無い
  - _Requirements: 6.2, 6.3, 6.4_
  - _Boundary: 隣接仕様の記録_

- [ ] 5. 非回帰の確認と分量規律の通過
  - 対象 crate（字句・意味の crate と、その下流で素通し断片を無視する crate）のテストを通し、適合対象フィクスチャの表示結果と時間軸が変わっていないことを確かめる
  - 行数上限の機械検査を通す（上限の例外表は書き換えない）
  - ワークスペース全体のテストを通す。事前に 32bit ヘルパーの成果物をビルドしてから実行する
  - 触れたファイルが設計の一覧の範囲に収まっていることを差分で確かめる
  - 出力を末尾抜き取りへ流し込まず、終了コードそのものを判定に使う
  - 観測: 対象範囲・行数検査・ワークスペース全体のいずれも成功で終わり、差分に範囲外のファイルが 1 つも含まれていない
  - _Requirements: 4.7, 5.9, 7.1, 7.4_

## Implementation Notes

- 字句層の `Token` は `pub(crate)` なので `crates/areka-parsers/tests/` の統合テストからは見えない。字句層の検証は `src/sakura/` 内の `#[cfg(test)]` モジュールから行う（設計の接続規約どおり本番ファイル末尾の `#[path]` 宣言経由）。
- `crates/areka-parsers/src/sakura/*.rs` は CRLF。`cargo fmt` は CRLF を LF へ落とすので、実行したら `file` コマンドで行末を確認し、崩れていれば CRLF へ戻したうえで `git diff --stat` の行数が小さいままか確かめる。
- タスク 2 の変異実測（設計の変異手順 ⑴⑵ を暫定ハーネスで先行確認）: 本体 `1`（読み足りない）と本体をワード全体の文字数にした形（読みすぎ）のどちらでも角括弧なし経路が赤になることを確認済み。恒久テストでの正式な実測はタスク 3.3。

### タスク 3.3 変異実測の記録（2026-09-03）

計測前の基準: 作業ツリーは無変更。`cargo test -p areka-parsers -p areka-sakura` は成功（areka-parsers 415 件・areka-sakura 88 件）。`cargo test -p areka-parsers bare_tag_tests` は 30 件すべて成功。

変異はどちらも `bare_tag_len` の**本体だけ**を置き換え、呼び出し側の式には触れていない（設計の変異手順 4 段目の但し書きに従う）。

#### 変異 ⑴ ＝ 元の欠陥の再現（本体を「常に 1」へ）

置き換えた本体:

```rust
fn bare_tag_len(word: &str) -> usize {
    let _ = word;
    1
}
```

実測: `cargo test -p areka-parsers bare_tag_tests` ＝ 10 件成功・20 件失敗。

赤 20 件（`sakura::lexer::bare_tag_tests::` と `sakura::parse::bare_tag_tests::` を省略して関数名のみ）:

| # | 関数名 |
|---|---|
| T1 | `two_char_underscore_tags_consume_as_single_unit` |
| T2 | `three_char_underscore_tags_consume_as_single_unit` |
| T3 | `underscore_tag_spelling_is_case_sensitive` |
| T4 | `underscore_tag_applies_to_non_canonical_followers` |
| T5 | `underscore_tag_does_not_swallow_following_body_text` |
| T6 | `underscore_tag_terminates_before_next_tag` |
| T7 | `underscore_tag_terminates_before_sysvar` |
| T8 | `underscore_tag_at_end_of_input_consumes_what_is_there` |
| T9 | `three_or_more_underscores_are_not_a_new_tag_form` |
| T11 | `underscore_tag_preserves_mixed_width_body_text` |
| T12 | `anchor_open_and_close_split_without_losing_body` |
| P1 | `quick_section_pair_shows_only_the_body` |
| P2 | `anchor_pair_shows_only_the_body` |
| P3 | `choice_range_pair_shows_only_the_body` |
| P4 | `voice_range_pair_shows_only_the_body` |
| P5 | `each_canonical_bracketless_tag_yields_exactly_one_raw` |
| P5 | `bracketless_underscore_w_and_l_never_become_wait_or_cursor` |
| P6 | `unknown_and_truncated_bracketless_tags_stay_raw` |
| P7 | `input_of_bracketless_tags_only_yields_no_text` |
| P10 | `mixed_width_body_survives_verbatim` |

緑 10 件と、緑のままが正しい理由:

| # | 関数名 | 緑が正しい理由 |
|---|---|---|
| T10 | `bracket_form_takes_precedence_over_bare_underscore_tag` | 角括弧経路は `bare_tag_len` を通らない |
| T13 | `known_one_char_tags_do_not_swallow_following_body_text` | 既知 1 文字タグの正しい消費長がちょうど 1 で、変異と一致する |
| T14 | `shorthand_rules_are_unchanged` | 短縮形判定は角括弧なし経路の手前で確定する |
| T14 | `escapes_are_unchanged` | エスケープは `lex` 側の処理で `bare_tag_len` を通らない |
| T14 | `quoted_args_are_unchanged` | 引数のクォートは角括弧経路の処理 |
| T14 | `unclosed_brackets_are_absorbed_as_raw_unchanged` | 未閉じ吸収は角括弧経路の処理 |
| T14 | `bracket_arg_splitting_is_unchanged` | 引数分割は角括弧経路の処理 |
| T15 | `trailing_lone_backslash_is_bare_spelling` | 末尾裸 `\` は `bare_tag_len` を呼ばない別経路 |
| P8 | `emo2_bracket_forms_keep_their_meaning` | `\_l[…]`／`\_w[…]` は角括弧経路 |
| P9 | `known_one_char_bare_tags_keep_their_meaning` | 既知 1 文字タグの消費長が 1 で変異と一致する |

#### 変異 ⑵ ＝ 読みすぎの再現（本体をワード全体の文字数へ）

置き換えた本体:

```rust
fn bare_tag_len(word: &str) -> usize {
    word.chars().count()
}
```

実測: `cargo test -p areka-parsers bare_tag_tests` ＝ 21 件成功・9 件失敗。

赤 9 件:

| # | 関数名 |
|---|---|
| T5 | `underscore_tag_does_not_swallow_following_body_text` |
| T9 | `three_or_more_underscores_are_not_a_new_tag_form` |
| T11 | `underscore_tag_preserves_mixed_width_body_text` |
| T12 | `anchor_open_and_close_split_without_losing_body` |
| T13 | `known_one_char_tags_do_not_swallow_following_body_text` |
| P1 | `quick_section_pair_shows_only_the_body` |
| P2 | `anchor_pair_shows_only_the_body` |
| P3 | `choice_range_pair_shows_only_the_body` |
| P10 | `mixed_width_body_survives_verbatim` |

緑 21 件（T1・T2・T3・T4・T6・T7・T8・T10・T14 の 5 関数・T15・P4・P5 の 2 関数・P6・P7・P8・P9）と理由: タグの直後が入力末尾か `\` か `%` の形は、ワード走査がそこで止まるためワード全体の長さと正しい固定長が一致し、偶然通る。角括弧経路（T10・T14 の 5 関数・P8）と末尾裸 `\`（T15）はそもそも `bare_tag_len` を通らない。P4 は終了形が入力末尾、P6 は綴りだけの入力、P7 は角括弧なしタグだけを並べた入力なので、いずれも本文が続かない。だから本文が続く形（T5・T11・T12・T13・P1・P2・P3・P10）を対に含めることが要る。

#### 設計の予測との差

いずれの変異でも、**設計が赤と予測した項目はすべて実測でも赤**だった。予測が赤なのに緑だった項目（テストの穴）は 1 件も無い。差はすべて逆向き、つまり「設計が緑と予測したが実測は赤」＝テストが予測より強い方向で、テストを弱めた箇所は 1 か所も無い。

変異 ⑴ の差 4 件:

- **T8**（`\__` が入力末尾）: 変異下では `\__` が `Bare("_")` ＋ `Text("_")` に割れる。3 文字形の末尾は実際に壊れるので、緑という予測は設計側の書き誤り。
- **T3**（大小文字の区別）: 設計の理由書き（「別綴りであることは 1 文字でも成立」）は、2 つの綴りが違うことだけを主張する弱い T3 を想定していた。実装した T3 は設計自身が求める「トークン列を丸ごと固定する」流儀に従っているので赤になる。
- **P7**（角括弧なしタグだけの入力から本文が生じない）: 変異下では切り詰められたタグの残りが `Text` へこぼれる。P7 はそれを禁じているので赤になるのが正しい。
- **P10**（全角半角混在の本文が逐語で残る）: 字句層の対である T11 は設計自身が赤と予測しており、P10 だけ緑としたのは予測表の内部矛盾。

変異 ⑵ の差 1 件:

- **P1**（`\_q…\_q` の表示本文）: 開始側の `\_q` の後ろに本文が続くため、ワード全体を消費すると本文が `Raw` に飲み込まれて消える。設計は P1 を「タグが末尾または `\` の直前にある形」と見なしたが、開始側は本文が続く形であり、緑という予測は誤り。

#### 復旧の確認

変異はどちらも元へ戻した。`crates/areka-parsers/src/sakura/lexer.rs` は sha256 `13ce0cbb…4d78` で基準と一致し、`git diff -- crates/areka-parsers/src/sakura/lexer.rs` は空。`cargo test -p areka-parsers -p areka-sakura`（415 件・88 件）・`cargo test -p areka-parsers bare_tag_tests`（30 件）・`cargo test -p log-capture-kit` はいずれも成功（終了コード 0）。
