# Brief: areka-P0-choice-script-prefix

> 2026-10-05 `/kiro-discovery` で起票（開発者「さくらスクリプトで、リンク（選択肢）をクリックしたらファイルを開いたりできるか？ バルーンからアプリを開く用途は結構あると思う。SHIORI でもできなくはないけど、あったらうれしい機能」）。「バルーンのリンクと OS の連携」の 7 本の 1 本（roadmap の同名の節が分け方の正本）。

## Problem

- **利用者**: 選択肢からその場で台本を走らせたいゴースト作者。正典では `\q[タイトル,script:さくらスクリプト]` と書けば、SHIORI を通さずに選んだ瞬間に台本が動く（ukadoc `\q[タイトル,script:実行内容]`「選択後、script:以下の内容をさくらスクリプトとして実行する」・記述例 `\q[バルーンを閉じる,script:\e]`・入れ子 `\q[その１,"script:\q[その２,script:その３はない]"]`）。
- 「メモ帳を開く」「公式サイトを開く」のような選択肢は、`script:` と開く系のタグ（`open-external-tags`）の組み合わせで書くのが正典の作法＝`\q[メモ帳を開く,script:\![open,file,notepad.exe]]`。
- areka では `script:` の選択肢を選んでも**何も実行されない**。記録は警告 1 行だけで、ゴースト作者は動かない理由に気付きにくい。

## Current State

- 読み込み: `crates/areka-parsers/src/sakura/decode.rs` の `decode_choice` が `Instruction::Choice{disp,target,references}` を作る。`script:` の ID もそのまま `target` に入る。
- 振り分け: `crates/areka-kanade/src/schedule/choice.rs` の `plan_cascade` が `script:` で始まる ID を `Unsupported` にする（テストは同ファイルの `script:` の組）。
- 実行: `crates/areka-kanade/src/schedule/steady.rs` の `on_choice`（起票時 313〜326 行）が `warn event="choice_unsupported_category"` を出し、SHIORI へは何も送らず、`ResolveChoice`（理由 `"unsupported"`）で選択肢の待ちを閉じるだけ。
- `steady.rs` は起票時 947 行＝1,000 行の上限の近く（roadmap の kanade の列の注記）。足す分は新しいファイルへ置く。
- 網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml` の `\q[タイトル,script:実行内容]` の行の状態は要件の段で確かめて直す。

## Desired Outcome

- `\q[…,script:…]` を選ぶと、`script:` の後ろが新しいトークとして実行される。`\e` で閉じる・`\q` の入れ子・開く系のタグ（入っていれば）が正典どおりに動く。
- 実行した台本の出どころは、選択肢を含んでいた元の台本と同じ扱い（`script-security-level` が出どころを運ぶようになったら、それを引き継ぐ）。
- 実行したことを記録に残す。読めない・空の `script:` も記録を出して閉じる（黙って捨てない）。

## Approach

- `plan_cascade` の `Unsupported` の枝を「台本を実行する」結論に替え、`on_choice` の実行は新しいファイル（例: `schedule/choice_script.rs`）に置く。
- 実行する台本は、トークの入口（SHIORI の応答の台本を流すのと同じ道）へ渡す。新しい再生の仕組みは作らない。
- 引数の `"…"` 括り（入れ子の例）の読み方が `decode.rs` で正典どおりかを要件の段で確かめる。

## Scope

- **In**: `\q[…,script:…]` の実行・出どころの扱い・記録・正典の記述例 2 つの決定論テスト・台帳の行の更新。
- **Out**:
  - `\__q[script:…]`（`range-choice-tag` が同じ道を使う）。
  - 開く系のタグそのもの（`open-external-tags`）。
  - おすすめサイトの `script:`（SHIORI リソース `sakura.recommendsites` などの URL 欄）＝メニューの側の話。

## Boundary Candidates

- 振り分けの結論（`plan_cascade`・純粋）。
- 台本の実行の配線（kanade の `on_choice` の新しいファイル）。

## Out of Boundary

- `OnChoiceSelect(Ex)` の通常の連鎖（完了済み `choice-select-events`）。
- 影響の段・同意の窓（`script-impact-tiers`）。

## Upstream / Downstream

- **Upstream**: 完了済み `choice-select-events`・`choice-interact`・`choice-timeout-directive`。
- **Downstream**: `range-choice-tag`（`\__q` の `script:`）・`link-context-copy`（`script:` の中の行き先を読む）。

## Existing Spec Touchpoints

- **Extends**: なし（完了済み `choice-select-events` の `Unsupported` の枝を埋める）。
- **Adjacent**: `balloon-lifecycle-events`・`sakura-time-critical`（kanade の列で同じ `schedule/` を触る）。

## Constraints

- kanade の列（roadmap「直列の列」）。`steady.rs` を 1,000 行未満に保つ。
- 段: 優先（バルーン関係）。規模の見込み S（5〜8）。
