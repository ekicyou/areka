# Brief: dola-cue-test-builder

> 2026-11-21 `test-crash-report-merge` の完了の棚卸（`/kiro-complete`）で起票。出どころ＝同 spec の要件の Out of scope（決定 5）「`dola` にテスト用の組み立てを置く件として起票する」。その場で直さなかった訳＝直し方が複数あり、公開する crate（`dola`）の公開の形の裁定が要る。

## Problem

正準のキャリアの合図（`\![name,tokens...]`＝`CueCommand::command_carrier` を載せた `TalkCue`・at 0・長さ 0）を組むテスト用の関数が、同じ働きのまま 3 つの crate に写しで置かれている。写しが増えるたびに正準形（at・長さ・演者の既定）の食い違いが入り込みうる。

## Current State

- `crates/areka-ghost/src/prop_sink.rs:118` の私有の `fn carrier_cue(actor, name, tokens) -> TalkCue`（呼び手 5 か所・同じファイルのテスト）
- `crates/areka-seriko/src/actor_test_support.rs:124` の `pub(super) fn carrier_cue(scope, name, tokens) -> TalkCue`（同じファイルの `cue(0.0, scope, …, 0.0)` を通す）
- `crates/areka-runtime/src/consumer_ledger_test_support.rs:126` の `pub fn carrier_cue(name, args)`（演者 `"0"` の既定）と、その下の `pub fn actor_carrier_cue(actor, name, args)`（上の 2 つと同じ本文）。runtime の呼び手は 10 ファイルほど（`move_cue_move_sink_tests.rs`・`talk_lifecycle_signals_tests.rs` ほか）
- 合図の型 `TalkCue`・`CueCommand` と `CueCommand::command_carrier` は `dola`（`crates/dola/src/cue/command.rs` 99・288 行）にある。`dola` の `[features]` は今 `toml` だけで、テスト用の機能は無い。`dola` は crates.io へ出す crate（`tools/test-all.ps1` の公開前の確認の段に入る）

## Desired Outcome

正準のキャリアの合図を組む本文が 1 つになり、3 つの crate のテストがそれを使う。正準形を変えるときに直す所が 1 つになる。

## Approach

`dola` の側に組み立てを 1 つ置き、3 つの写しを消して呼び手を付け替える。置き方は議題（要件の段で開発者の裁定を仰ぐ）:
- ⑴ `TalkCue` の公開の組み立て（例 `TalkCue::carrier(actor, name, tokens)`）として常に置く＝テスト専用でなく本番からも使える
- ⑵ `dola` に `test-support` の機能を足し、その下にだけ置く（ワークスペースの他の crate の `test-support` と同じ形）＝公開の API を増やさない
- ⑶ `dola` を引くワークスペースの中の小さな leaf（テスト専用）に置く

runtime の `carrier_cue(name, args)`（演者 `"0"` の既定）を残すか、呼び手で演者を書くかも議題。

## Scope
- **In**: 組み立ての置き場と形・3 つの写しの削除と呼び手の付け替え・`dola` の公開の文書（機能を足すならその説明）
- **Out**: 合図の型そのものの変更・`noncanonical_custom_cue` など正準でない合図の組み立て・`TalkCue` の他の組み立ての整理

## Boundary Candidates
- `dola` の公開の面（組み立ての置き場と名）
- 使い手 3 crate のテストの付け替え

## Out of Boundary
- `CueCommand` の意味や正準形の判定（`as_command_carrier`）を変えること

## Upstream / Downstream
- **Upstream**: `dola` の `cue` モジュール
- **Downstream**: areka-ghost・areka-seriko・areka-runtime のテスト

## Existing Spec Touchpoints
- **Extends**: 無い
- **Adjacent**: `dola` を触る v0.2.0 の spec（wintf・dola の基幹の整備）と、`crates/dola/src/cue/` を触る spec。着手のときに roadmap で重なりを確かめる

## Constraints
- 外部の crate はフォークせずそのまま使う。新しい外の依存は足さない。
- `dola` の公開の形を変えるなら、公開前の確認（`cargo package`）が通ること。

## 見立て
- **触るファイル**: `crates/dola/src/cue/command.rs`（または置き場の新しいファイル）・`crates/dola/Cargo.toml`（⑵ なら）・上の 3 ファイルと runtime の呼び手のテスト 10 ファイルほど・使い手の `Cargo.toml`（⑵・⑶ なら）。
- **依存**: ⑶ なら新しい crate で `Cargo.lock` の席を使う。⑴・⑵ は使わない（⑵ は機能の名だけ）。
- **規模**: S（3〜5 タスク）。
- **先に要るもの**: 無い。
- **優先度の区分**: A（見直しの指摘の取り残し・開発者「指摘事項は全部直す」）。
- **要件定義のモデル**: Opus（置き場の裁定 1 つと機械的な付け替え）。
- **議題**: 置き場（⑴〜⑶）・演者の既定を残すか。
- **分割の案**: 無し。
