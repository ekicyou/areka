# Requirements Document

## Project Description (Input)

開発者は、正準のキャリアの合図（`\![name,tokens...]`＝`CueCommand::command_carrier` を載せた `TalkCue`・発火時刻 0・長さ 0）を組むテスト用の本文を 1 か所でだけ直せるようにしたい。今は、同じ働きの写しが 3 つの crate に 5 つある。

- `crates/areka-ghost/src/prop_sink.rs:118` の私有の `carrier_cue(actor, name, tokens)`（同じファイルのテストが呼ぶ）
- `crates/areka-ghost/src/prop_get_sink_tests.rs:18` の `carrier(name, tokens)`（演者 `"0"`）
- `crates/areka-seriko/src/actor_test_support.rs:124` の `pub(super) fn carrier_cue(scope, name, tokens)`（呼び手 5 ファイル）
- `crates/areka-runtime/src/consumer_ledger_test_support.rs:126` の `carrier_cue(name, args)`（演者 `"0"` の既定）と `actor_carrier_cue(actor, name, args)`（呼び手は runtime の 12 ファイルと本体の 5 ファイル）
- `crates/areka-runtime/src/frame_window_link_drain_tests.rs:31` の `carrier(actor, name, tokens)`

写しが増えるたびに、正準形（発火時刻・長さ・演者）の食い違いが入り込みうる。合図の型 `TalkCue`・`CueCommand` と `CueCommand::command_carrier` は `dola`（`crates/dola/src/cue/command.rs` の 183・288 行）にあり、`dola` は crates.io へ出す crate で、`[features]` は今 `toml` だけ。

直した後の姿: 正準のキャリアの合図を組む本文が `dola` に 1 つだけ在り、5 つの写しはそれを通す。正準形を変えるときに直す所が 1 つになる。

出どころは `test-crash-report-merge` の完了の棚卸（同 spec の要件の Out of scope・決定 5）で、優先度の区分は A（見直しの指摘の取り残し・開発者「指摘事項は全部直す」）。ウェーブ C25 で `session-crate-split`・`test-name-work-tags` と並んで走る。

## Introduction

この spec は、テストだけが使う「正準のキャリアの合図の組み立て」を `dola` の側に 1 つ置き、3 つの crate（areka-ghost・areka-seriko・areka-runtime）にある 5 つの写しの本文を消して、それを通す形にする。合図の型そのもの・正準形の判定（`as_command_carrier`）・本番の振る舞いは変えない。

ウェーブ C25 の約束（`roadmap.md` の「ウェーブ編成」の C25 の行の ⑵・⑸）を守る。

- 呼び手の名（runtime の `carrier_cue`・`actor_carrier_cue`、seriko の `carrier_cue`）は、`dola` の組み立てを呼ぶだけの 1 行として残し、呼び手の 22 ファイル（本体 5・runtime 12・seriko 5）に触らない。
- 新しい crate を足す形（brief の ⑶）は取らない。`Cargo.lock` の席は `session-crate-split` が使う（この spec が変えてよいのは `Cargo.toml` の機能の行だけ）。

調べた時点は main `00877299`（brief の最後の再測定 `d4db9eb0` の後に入ったのは roadmap の文書だけ）。`dola` には今 `impl TalkCue` の組み立てが 1 つも無く、本番のコードは発火時刻 0・長さ 0 の `TalkCue` を直に組まない（本番の合図は `CueSheet` の正準の変換で組まれる）。

**仮定のしるし**: 【仮定 A1】〜【仮定 A4】は brief と roadmap で決まらず、要件の討議で開発者の裁定を仰ぐ点。仮定のとおりに書いた。

## Boundary Context

- **In scope**:
  - `dola` に置く正準のキャリアの合図の組み立て 1 つ（置き場と名・`crates/dola/src/cue/command.rs` か `crates/dola/src/cue/` の中の新しいファイル）と、その働きを固定する `dola` のテスト。
  - `dola` の `Cargo.toml` の機能の行と、`dola` の公開の文書（README と機能の行の説明）。【仮定 A1】
  - 5 つの写しの本文を消し、`dola` の組み立てを通す形にすること。
  - 使い手 3 crate の `Cargo.toml` の、`dola` の機能を有効にする行。
- **Out of scope**:
  - 合図の型そのもの（`TalkCue` のフィールド・`CueCommand` の変種）と、`command_carrier`・`as_command_carrier` の意味を変えること。
  - 正準でない合図の組み立て（seriko の `noncanonical_custom_cue`・runtime の `broken_custom_cue`）と、`TalkCue` の他の組み立ての整理（`crates/areka-ghost/src/sink.rs` の `sample_cue`・`crates/areka-runtime/src/shell_carry.rs` の `cue` ほか）。
  - 本体の session のテスト 3 本（`readonly_run_tests.rs:94`・`readonly_sites_tests.rs:118`・`write_failure_sites_tests.rs:139`）の `prop_set_cue`。【仮定 A3】
  - 名のある組み立ての関数を通さず `TalkCue { at: 0.0, … }` を直に書くテスト（`crates/areka-ghost/tests/ghost/inproc_e2e_test.rs` ほか）。【仮定 A4】
  - 呼び手の 22 ファイルの付け替え（名を残すので触らない）。
  - 新しい crate（brief の ⑶）・ワークスペースの外の新しい依存・`Cargo.lock` の変更。
  - `crates/areka-sakura/src/compile_sheet_tests.rs`（テストの名に語が入るだけで写しではない）。
- **Adjacent expectations**:
  - `session-crate-split` は `crates/dola/`・`crates/areka-ghost/`・`crates/areka-seriko/` に触らず、`crates/areka-runtime/` にはパスを名指すコメント（`talk_clock.rs`・`user_break_cue.rs`・`placement_tests/transition_judge_offset_signoff_tests.rs`）のほか触らない（C25 の約束 ⑶）。この spec が runtime で触る `consumer_ledger_test_support.rs`・`frame_window_link_drain_tests.rs`・`Cargo.toml` とは重ならない。本体の `crates/areka/src/session/` の移しとも、この spec は本体に触らないので重ならない。
  - `test-name-work-tags` が runtime で触るテスト 8 ファイル（`window_state_cue_tests.rs`・`zorder_cue_tests.rs`・`zorder_wiring_tests.rs`・`frame/zorder_*_tests.rs`・`frame_schedule_tests.rs`）は、名を残すので、この spec は触らない。
  - 本体のテスト（`install/install_cue_tests.rs` ほか 5 ファイル）は `areka_runtime::consumer_ledger::test_support::carrier_cue` を runtime の `test-support` 機能を通して使う。この spec の後も、本体の `Cargo.toml` を変えずにその名が引ける。
  - C27 の `dola-interrupt-ticks` は、この spec の着地を待つ（`crates/dola/` の全体と使い手のテストに触る）。

## Requirements

### Requirement 1: 正準のキャリアの合図の組み立てを `dola` に 1 つ置く

**Objective:** As a 開発者, I want 正準のキャリアの合図を組む本文をワークスペースに 1 つだけ持ちたい, so that 正準形を変えるときに直す所が 1 つになり、写しの直し忘れで食い違わない

#### Acceptance Criteria

1. The `dola` shall 正準のキャリアの合図（発火時刻 0・長さ 0・演者は呼び手が渡したもの・コマンドは `CueCommand::command_carrier` で組んだもの）を組む組み立てを 1 つだけ持つ
2. When 組み立てが名とトークンの並びを受け取ったとき, the `dola` shall トークンを並びの順のまま変えずに載せ、空のトークン（省略の枠）と `--key=value` のトークンもそのまま保つ
3. When 組み立てで組んだ合図を `as_command_carrier` で開いたとき, the `dola` shall 渡した名と同じ名と、渡したトークンの並びと同じ並びを返す
4. The `dola` shall 組み立ての演者を呼び手に必ず渡させ、演者の既定（`"0"` など）を組み立ての側に持たない【仮定 A2】
5. The `dola` shall 1〜3 の働き（発火時刻 0・長さ 0・演者・トークンの順と空のトークン・開いたときの往復）を `dola` 自身のテストで固定する

### Requirement 2: `dola` の公開の面を増やさない

**Objective:** As a `dola` の利用者（crates.io の使い手）, I want テスト専用の組み立てが既定の公開の面に出ないでほしい, so that 公開の API が本番で使わない口のぶんだけ増えない

#### Acceptance Criteria

1. Where `dola` の `test-support` 機能を有効にしたとき, the `dola` shall 組み立てを使い手の crate から呼べるようにする【仮定 A1】
2. While `test-support` 機能が無効のとき（既定）, the `dola` shall 組み立てを公開の面に出さず、既定の公開の面を今と同じに保つ【仮定 A1】
3. The 使い手の crate（areka-ghost・areka-seriko・areka-runtime） shall `dola` の `test-support` 機能を、テストのビルドのとき（runtime は自身の `test-support` 機能のときも）だけ有効にし、本番のビルド（本体 `areka` の配布物）に組み立てを入れない
4. When 公開前の確認（`tools/crates-io.ps1` の梱包の確かめ）を回したとき, the `dola` shall 通る
5. The `dola` shall `test-support` 機能の働き（テストだけが使う組み立てを開けること・本番では付けないこと）を README に書く（`cargo package` は `Cargo.toml` のコメントを落とすので、crates.io の利用者への説明は README が担う）
6. The `dola` shall ワークスペースの外の crate を新しく足さず、`Cargo.lock` を変えない

### Requirement 3: 5 つの写しの本文を消す

**Objective:** As a 開発者, I want 3 つの crate の写しが自分の本文を持たないでほしい, so that 正準形が 1 か所から決まり、呼び手のテストはそのまま動く

#### Acceptance Criteria

1. The 写しの 5 ファイル（`crates/areka-ghost/src/prop_sink.rs`・`crates/areka-ghost/src/prop_get_sink_tests.rs`・`crates/areka-seriko/src/actor_test_support.rs`・`crates/areka-runtime/src/consumer_ledger_test_support.rs`・`crates/areka-runtime/src/frame_window_link_drain_tests.rs`） shall 正準のキャリアの合図を自分の本文（発火時刻・長さ・`command_carrier` の呼び出し）で組まず、`dola` の組み立てを通す
2. The runtime の `carrier_cue`・`actor_carrier_cue` と seriko の `carrier_cue` shall 名と引数の形を今のまま残し、`dola` の組み立てを呼ぶだけの 1 行になる
3. When runtime の `carrier_cue(name, args)` が呼ばれたとき, the runtime の `carrier_cue` shall 今と同じく演者 `"0"` の合図を返す
4. When areka-ghost の `prop_get_sink_tests.rs` の `carrier(name, tokens)` が置き換わったとき, the 同じファイルのテスト shall 今と同じく演者 `"0"` の合図を受け取る
5. The 呼び手の 22 ファイル（本体 5・runtime 12・seriko 5） shall この spec の差分に入らない
6. The 写しのうち同じファイルの中だけで呼ばれるもの（areka-ghost の `prop_sink.rs` の `carrier_cue`・`prop_get_sink_tests.rs` の `carrier`、runtime の `frame_window_link_drain_tests.rs` の `carrier`） shall 本文を持たない 1 行の名として残すか、同じファイルの呼び手が `dola` の組み立てを直に呼ぶ形にするかのどちらかになる（どちらかは設計で決める）

### Requirement 4: 振る舞いを変えない

**Objective:** As a 開発者, I want 付け替えの前と後でテストの振る舞いと合図の型が変わらないでほしい, so that この spec がテストの支えの整理だけに留まる

#### Acceptance Criteria

1. The 合図の型 shall `TalkCue` のフィールド・`CueCommand` の変種・`command_carrier` と `as_command_carrier` の意味を変えない
2. When 付け替えの後に areka-ghost・areka-seriko・areka-runtime・`dola` と本体のテストを回したとき, the テスト shall 付け替えの前と同じテストが全部緑になり、消えたテストも新しく無視されたテストも無い（足すのは要件 1.5 の `dola` のテストだけ）
3. The 正準でない合図の組み立て（seriko の `noncanonical_custom_cue`・runtime の `broken_custom_cue`） shall 変わらない

### Requirement 5: 範囲の境界とウェーブ C25 の約束を守る

**Objective:** As a ウェーブ C25 の並走のセッション, I want この spec が約束したファイルの外に触らないでほしい, so that `session-crate-split`・`test-name-work-tags` と触るファイルが重ならず、同じウェーブで着地できる

#### Acceptance Criteria

1. The この spec の差分 shall `crates/dola/src/cue/` の中（組み立てとそのテスト）・`crates/dola/Cargo.toml`・`crates/dola/README.md`・写しの 5 ファイル・使い手 3 crate の `Cargo.toml` の `dola` の行と機能の行だけに留まる
2. The この spec の差分 shall 本体 `crates/areka/` のファイル・根の `Cargo.toml`・`Cargo.lock` に入らない
3. The この spec shall 新しい crate を足さない
4. The この spec shall 本体の session のテスト 3 本の `prop_set_cue` を寄せない（`session-crate-split` が本体の `session/` を移し、`audit-areka-app` が本体のテストの残りを受け持つ）【仮定 A3】
5. The この spec shall 名のある組み立ての関数を通さず `TalkCue { at: 0.0, … }` を直に書くテストを付け替えない【仮定 A4】
6. The この spec の差分のコードのコメント shall `.kiro` の spec・要件の番号・タスクの番号を名指さない（`CLAUDE.md` の `## Comments`）
