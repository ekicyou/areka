# Research & Design Decisions

## Summary

- **Feature**: `areka-P0-farewell-talk-status`
- **Discovery Scope**: Extension（既存の kanade の運行の判定 1 か所を直す・軽い調査）
- **Key Findings**:
  - 「再生中か」の判定 `talk_active_of` と、再生中のトークの番号を引く `current_talk_id`（どちらも `crates/areka-kanade/src/schedule/mod.rs`）は、同じ問いに別々の表で答えている。後者は既にお別れの台詞の 3 つの相を知っている。前者を後者へ委ねれば、足すのは 0 行で、2 つの表が食い違う余地が無くなる。
  - お別れの台詞の再生中に `State::snapshot` から作られて実際に外へ出る値は 2 つだけ: 状態の問い合わせの答え（`actor.rs` の `answer_status`）と、お別れの台詞そのものの翻訳（`schedule/translate.rs` の `capture`）の `Status`。brief が挙げた `\![raise,…]` とバルーンのイベントは、定常でなければ送る前に断られるので該当しない。
  - 新しい振る舞いで赤になる既存のテストは 3 か所（お別れの台詞の翻訳の `Status` を「会話なし」または「行なし」と固定している所。設計の時点では 2 か所と読み、3 か所目は実装で見つけた）。ほかは期待を `State::snapshot` から導いているので自動で追随する。

## Research Log

### 判定を読む所の洗い出し

- **Context**: brief の Approach「この判定を使うすべての所への波及を洗い出す」。
- **Sources Consulted**: `crates/` 全域の `talk_active_of`・`snapshot()`・`snapshot_with_choice`・`snapshot_without_talk`・`talk_active` の検索と、各所の入口の門（相の判定）の読み。
- **Findings**:
  - `talk_active_of` の呼び手は `State::snapshot_with_choice` の 1 か所（`State::snapshot` はここを通る）。`talking` と `nouserbreak`（`talk_active && 旗の写し`）がここで決まる。
  - `State::snapshot`／`snapshot_with_choice` の本番の呼び手は 12 か所。お別れの 3 つの相で届くのは `actor.rs` の `answer_status` と `schedule/translate.rs` の `capture` だけ。
  - `schedule/change.rs` の `on_raise_event` は `Phase::Steady` でなければ `raise_event_not_steady` を残して捨てる。`schedule/talk_gap.rs` の `begin` は「定常でない」で返す。`schedule/balloon_events.rs` の 3 つの入口は `steady_idle`／`Phase::Steady` の門で `not_steady` を残して送らない。`actor_resources.rs` の `answer` は `queryable`（`Phase::Steady` だけ）が偽なら SHIORI へ送らない。
  - `schedule/translate.rs` の `capture` は、腕が相を決めた後（＝お別れの相へ移った後）に `ExecutionStatus::derive(&state.snapshot())` を取る。普段の会話の台詞の翻訳は今も `talking` 付き（`schedule/translate_tests.rs` が「捕まえた時点の相は再生中」を固定）。
  - `events::on_second_change` の Reference3（`!snapshot.talk_active`）は `OnSecondChange` が定常の相でしか送られないので変わらない。
- **Implications**: 要件 2.2（お別れの台詞の翻訳の `Status` に `talking`）が、判定の直しから自動で出る。要件 3.3（増える要求は 0）は門を触らないことで保たれる。

### 既存のテストへの影響

- **Context**: 要件 4.9（既存のテストを緑に保つ・古い前提のテストは書き換える）。
- **Sources Consulted**: `expected_translate(` の全呼び手・`ExecutionSnapshot::INACTIVE` を使う終了／切り替えのテスト・`crates/areka` の進行状態の記録（`status_calls()`）の読み手。
- **Findings**:
  - 赤になる 1: `crates/areka-kanade/tests/kanade/close_test_handshake_tests.rs` の、`OnClose` の後の 1 件目（別れの台詞の `OnTranslate`）を `ExecutionSnapshot::INACTIVE` で期待している所（文言「終了の相は会話なしの状態」）。
  - 赤になる 2: `crates/areka/src/emo2_boot/spine_conformance_script.rs` の `expected_statuses` の最後の行 `status(TRANSLATE, None)`（注記「終了の挨拶の `OnTranslate` も終了の相の状態＝ヘッダ行なし」）。読むのは `spine_conformance_lap_tests.rs` の一周のテスト。
  - 赤になる 3（設計の時点の見落とし・実装で `cargo test -p areka-kanade` を全部回して見つけた）: `crates/areka-kanade/tests/kanade/external_status_test.rs` の、通信中の旗を落とした後の要求を「どれも `Status` の行なし」と見る表明。区間に終了の挨拶の `OnTranslate` が入る。
  - 追随する: `schedule/translate_path_tests.rs` の `assert_path`（期待を `d.s.snapshot()` から導く）・`schedule/schedule_tests.rs` の `state_snapshot_preserves_the_talk_axis_of_phase`（期待を `talk_active_of` から導く）。
  - 影響なし: `crates/areka-ghost` の結合テストの記録は `Status` を持たない。`tests/kanade/translate_test.rs` の終了・切り替えの行は `OnClose`／`OnGhostChanging` が 204 の筋書きで、お別れの台詞を再生しない。
  - 握手の要求が会話なしで送られることは `schedule/external_state_tests.rs` が既に固定している（`OnGhostChanging`・その 204 の後の `OnClose`・定常からの `OnClose`・普段の会話の途中の強制終了）。足りないのは「普段の会話の完了を待ってから送る `OnClose`」と「お別れの台詞の途中の強制終了」。
- **Implications**: 書き換える既存のテストは 3 か所（上の 1〜3）。握手の固定は 2 場面を足せば要件 4.5 の 4 つがそろう。

### テストの置き場

- **Context**: brief は「新しい兄弟のテストファイル」と `tests/kanade/status_query_test.rs` を候補に挙げた。
- **Findings**:
  - `schedule/external_state_tests.rs`（402 行）は「`State::snapshot` の `nouserbreak` の門」と「会話なしの作り方を使う各リクエスト」を既に扱い、要る道具（満たした写し `full_copy`・`WITHOUT_TALK`・最後のリクエストを取る `last_request`・翻訳を素通しする `pass_translate` の使い方）がそろっている。ここへ足せば `schedule/mod.rs` にテストの接続宣言を足さずに済む。
  - `tests/kanade/status_query_test.rs` の土台（`Fixture`）は `OnGhostChanging` に台詞を返せない（未知の GET は 204）。`tests/kanade/translate_test.rs`（637 行）の偽物（`spawn_rig`／`WireSpec`）は、任意の GET の ID に台詞を返せ、再生側が完了を返さない（台詞はテストが完了を送るまで再生中）ので、3 つの場面すべてを手を入れずに通せる。記録に `Status` も残る。
- **Implications**: 新しいファイルは 0。段のテストは `external_state_tests.rs`、殻を通すテストは `translate_test.rs` に足す。

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| A: `matches!` に 3 つの相を足す | `talk_active_of` の表に `CloseTalkWait`・`ChangeTalkWait`・`ChangeCloseTalkWait` を足す | brief の Approach のまま | 同じ問いの表が 2 つ残り、相を足すたびに両方を直す。`schedule/mod.rs` が増える（955 行） | 不採用 |
| B: `current_talk_id` へ委ねる | `talk_active_of(phase)` を「`current_talk_id(phase)` が番号を返すか」にする | 表が 1 つになり、要件 1.7 が作りで保たれる。`schedule/mod.rs` は数行減る | 将来「番号は在るが話していない」相が出たら分け直す | **採用** |
| C: 判定を相のメソッドへ移す | `Phase::active_talk_id()` を作って両者を寄せる | 読みやすい | 呼び手の書き換えが広がる。`mcp-kanade-tools` が `schedule/mod.rs` を割る予定と重なる | 不採用（範囲の外） |

## Design Decisions

### Decision: 判定を `current_talk_id` へ委ねる（案 B）

- **Context**: 要件 1.1〜1.3・1.7。
- **Alternatives Considered**: 上の表の A・C。
- **Selected Approach**: `talk_active_of` の本体を `current_talk_id(phase).is_some()` にする。署名は変えない。
- **Rationale**: 利用者の中断・完了の突き合わせ・印の台詞の追跡が既に `current_talk_id` を「再生中のトーク」の定義として使っている。`talking` だけ別の表を持つ理由が無い。
- **Trade-offs**: 2 つの関数が同じになるので、要件 4.6 の一致のテストは今は自明になる（将来だれかが片方だけを書き換えたときの見張りとして残す）。
- **Follow-up**: 判定の全場面の表のテスト（要件 4.1）は、期待を手書きの表で持つ（`talk_active_of` から導かない）。

### Decision: テストは既存の 2 ファイルへ足し、新しいファイルを作らない

- **Context**: 要件 4.1〜4.6。`schedule/mod.rs` は 955 行。
- **Alternatives Considered**: 1. 新しい兄弟のテストファイル＋`schedule/mod.rs` の接続宣言 2. `status_query_test.rs`＋`Fixture` の拡張
- **Selected Approach**: 段のテストは `schedule/external_state_tests.rs`、殻を通すテストは `tests/kanade/translate_test.rs`。
- **Rationale**: 要る道具が両方にそろっている。`schedule/mod.rs` を触る行を判定の本体と注記だけに絞れ、同じファイルを触る他の spec との重なりが最小になる。
- **Trade-offs**: `external_state_tests.rs` は 600 行前後、`translate_test.rs` は 750 行前後になる（どちらも上限 1,000 の内）。

### Decision: 実機の確認は課さない

- **Context**: 要件の暫定の裁定 5。
- **Selected Approach**: 殻を通す決定論テスト（本物の kanade の殻に偽の SHIORI と再生側をつなぐ）で、問い合わせの答えと `OnTranslate` の `Status` を固定する。
- **Rationale**: MCP の側は変えない。`get_status` の実機の確認は完了 `mcp-get-status` が済ませている。

## Risks & Mitigations

- `crates/areka` の一周のテスト（`spine_conformance_lap_tests.rs`）は GPU の足場を使う重いテスト — 期待の書き換えの後、そのテストだけを名前で絞って 1 回回す。全体は完了時の `tools/test-all.ps1` に任せる。
- 見落とした既存のテストが赤になる — `cargo test -p areka-kanade` を全部回して拾う（軽い）。`crates/areka` は `Status` を読む所が進行状態の記録の 1 系統だけなので、上の 1 か所で尽きる。
- 同じ時期に `schedule/mod.rs` を触る spec と競合する — 触るのは `talk_active_of` の本体と注記の数行だけ。先に着地させる。

## References

- ukadoc `Status [SSP拡張]`: <https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html#request> — `talking`＝「喋っている途中」。
- 完了 spec `areka-P0-mcp-get-status`（`.kiro/specs/completed/areka-P0-mcp-get-status/requirements.md`）— 暫定の裁定 7 が本 spec の起票元。要件 1.2（`get_status` の本文は SHIORI へ送る `Status` と同じ）。
- `doc/ssp-mcp/get-status-diff-areka.md` — 本 spec が直す差の一覧。
