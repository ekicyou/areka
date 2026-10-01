# Brief: areka-P0-balloon-reappear-short-talk

## Problem
隠れたバルーン（時間切れ・利用者の中断・バルーンの切替の後）は、次の台詞が 1 文字だけだと現れない。ゴーストが「ん。」のような 1 文字の台詞を話すと、利用者には何も見えない（台詞は流れているのに吹き出しが出ない）。2 文字以上の台詞なら現れるので、気付きにくい。

## Current State
- 表示の判定は `crates/areka/src/emo2_boot/balloon_visibility.rs` にある。隠れたバルーンを出すのは「見える文字の数が前のフレームより増えた、かつ今は見えていない」とき（`decide_content` と、スコープごとの文字の数の記録 `last_glyphs`）。完了 spec `areka-P0-balloon-visibility` の規則。
- 台詞の始まりの全消去と 1 文字目が**同じフレーム**に届くと、そのフレームの文字の数は前のフレームと同じ（例: 前の台詞の残り 1 文字 → 新しい台詞の 1 文字）か少なく見え、増加に数えられない。次のフレームで文字が増えなければ（台詞が 1 文字で終われば）、バルーンは出ないまま終わる。
- 2026-10-01 に `areka-P0-shell-balloon-switch` のタスク 11.2（バルーンの往復の統合テスト）で見つかった。そのテストは台詞を 2 文字以上にして避け、理由を `crates/areka/src/shell_balloon_switch_session_balloon_tests.rs` の doc に書いた。開発者の指示で、同 spec の完了の棚卸で起票した。

## Desired Outcome
- 隠れたバルーンは、次の台詞が 1 文字でも、その文字が出たフレームで現れる（時間切れ・利用者の中断・バルーンの切替の後のどれでも）。
- 台詞の外では現れない、という今の規則は保つ。特に `areka-P0-shell-balloon-switch` 要件 3.3（バルーンの切替の後、`BalloonVisibilityState::forget_scope` で記録を忘れたスコープが、表示済みの文字を増加と取り違えて台詞の外で現れない）を崩さない。
- 直す前に赤になる決定論のテストがある。

## Approach
根本の判定を直す。候補は「同じフレームに全消去（台詞の始まり）が在れば、そのフレームの文字の数を 0 からの増加として扱う」。全消去の印を可視性の判定がどう受け取るか（文字の層の供給・cue の流れ）は、要件・設計の段で今のデータの流れを読んで決める。表示の時期をずらす（1 フレーム遅らせる）解は取らない。

## Scope
- **In**: 隠れたバルーンを出す判定の修正と、その決定論のテスト（1 文字の台詞を、時間切れ・中断・切替の後の各場面で）。完了 spec `balloon-visibility` の規則の記述（COMPAT §8 など）が変わるなら、その追随。
- **Out**: 時間切れの長さ・隠す側の規則・バルーンの見た目の変更。シェル・バルーンの切替の振る舞い（`shell-balloon-switch` の範囲）。

## Boundary Candidates
- 可視性の判定（`balloon_visibility.rs` の表示の判定）
- 台詞の始まり（全消去）が判定へ届く経路（文字の層の供給・cue）

## Out of Boundary
- バルーンの切替・装着の置き換え（`shell-balloon-switch`）
- バルーンの時間切れの既定値・`balloontimeout` の解釈

## Upstream / Downstream
- **Upstream**: 完了 `areka-P0-balloon-visibility`（判定の規則）、完了 `areka-P0-shell-balloon-switch`（`forget_scope` と要件 3.3）
- **Downstream**: 無し（独立のバグ修正）

## Existing Spec Touchpoints
- **Extends**: 完了 `areka-P0-balloon-visibility` の規則の穴を塞ぐ（完了 spec は書き換えず、本 spec で直す）
- **Adjacent**: `areka-P0-shell-balloon-switch`（`forget_scope`）

## Constraints
- 1 フレーム遅らせる解は取らない（状態の持ち方を変えて 0 フレームで解く）。
- 決定論のテストで赤を立ててから直す。テストは本番ファイルの兄弟ファイルへ置き、どのファイルも 1,000 行以下。
- 規模は S の見込み（判定の分岐 1 か所と、全消去の印の受け渡し）。
