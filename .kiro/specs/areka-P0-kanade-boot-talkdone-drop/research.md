# ギャップ分析（kiro-validate-gap）— areka-P0-kanade-boot-talkdone-drop

- 作成: 2026-09-11（要件確定後・設計前）
- 入力: `requirements.md`（確定）・`brief.md`・steering（`product.md`／`tech.md`／`structure.md`／`logging.md`／`roadmap.md` W13）
- 調査対象: `crates/areka-kanade/src/schedule/{mod.rs, boot.rs, steady.rs, close.rs}`・兄弟テスト・`crates/areka-kanade/src/actor.rs`・`crates/areka/src/emo2_boot/spine_conformance_*`・完了仕様 `areka-P0-idle-talk`（DD-IT-12）／`areka-P0-emo2-conformance-e2e`（記録 §13.2 行 4・手順書 §5.7）
- 本文書は情報と選択肢を並べるものであり、最終判断は要件ディスカッションと設計に委ねる。

---

## 1. 現状調査（コードの事実）

### 1.1 遷移の入口と委譲の形

| 要素 | 場所 | 事実 |
|---|---|---|
| 唯一の遷移入口 `step` | `schedule/mod.rs` の `pub(crate) fn step` | 「横断遷移を先に判定 → 該当しなければフェーズ分岐」。`Input::TalkDone` は無条件に `on_talk_done` へ |
| 横断突合 `on_talk_done` | `mod.rs` の `fn on_talk_done` | `current_talk_id(&state.phase)` と `done.talk_id` が一致したときだけ `choice_prev_talk = None` → 理由で 3 分岐。`Quit` → `Unloading{Quit}`＋`ShioriUnload`（横断で完結）。`Interrupted` → info `talk_done_interrupted_as_non_quit` の後 `dispatch_phase`。`Ended` → `dispatch_phase`。不一致は `talk_done_stale_choice`（info）か `unknown_talk_done`（error）で終端し委譲へ来ない |
| 突合対象 `current_talk_id` | `mod.rs` の `fn current_talk_id` | `Steady{Some}`・**`BootVersion{Some}`**・`CloseTalkWait` の 3 つ。コメントに「TalkDone が BootVersion 中に届いた場合の防御・DD-IT-12」と逐語で書いてある |
| 相ごとの委譲 `dispatch_phase` | `mod.rs` の `fn dispatch_phase` | `Idle｜BootInit｜BootPrefetch｜BootType｜BootMain｜BootVersion{..}` → `boot::step`／`Steady` → `steady::step`／`ClosePending｜CloseTalkWait` → `close::step`／`Unloading｜Stopped` → warn `input_after_terminate` |
| 起動系列の分岐 `boot::step` | `schedule/boot.rs` の `pub(crate) fn step` | 腕は `Boot`・`ShioriReply`・`CloseRequest` の 3 本＋ワイルドカード `_ =>` が `warn!(target: "kanade", event = "boot_input_ignored", "boot 系列に無関係な入力を無視")` で捨てる。**`TalkDone` の腕が無い**＝欠陥の実体 |
| 定常運転の受理 `steady::on_talk_done` | `schedule/steady.rs` の `fn on_talk_done` | `Steady{Some}` のみ受け、`clear_choice_ledger(.., "steady_talk_done")` → `pending_close` があれば info `steady_talk_done_close`＋`begin_close`（`OnClose` GET・`ClosePending`）、無ければ info `steady_talk_done`＋`Steady{None}`。**副作用指示なし** |
| 終了系列の受理 `close::on_close_talk_wait` | `schedule/close.rs` | `TalkDone` → info `close_refused`＋`Steady{None}`。同じく副作用なし |
| 起動完了 | `boot.rs` の `on_reply` の `Phase::BootVersion` 腕 | `Notified` → info `boot_complete` → `Steady{talk}`（`talk` をそのまま引き継ぐ・DD-IT-12） |
| 起動中の終了指示 | `boot.rs` の `record_pending_close` | info `boot_pending_close`＋`pending_close = Some`。相は変えない |
| 保留の消化 | `steady.rs` の Tick 処理（`Phase::Steady{talk: None}` 腕） | `pending_close.take()` → `begin_close`。`Steady{Some}` の Tick は NOTIFY のみで保留を消化しない |

要件が求める「受理 → 枠を空にする → 待ち点は維持 → 副作用 0 → 握手は定常運転に入ってから」は、`steady::on_talk_done` の `pending_close` 無しの枝（`Steady{None}` へ復帰・副作用なし）と同型であり、既存の語彙・規律をそのまま写せる。

### 1.2 ログ語彙と捕捉基盤

- `boot_input_ignored` の発行点は `boot.rs` のワイルドカード腕 1 か所のみ。コード側の消費者は `schedule_log_firing_tests.rs` の `warn_boot_input_ignored_logs`（`BootInit`＋Tick を `step` で駆動し WARN の存在を表明）だけ。文書側の消費者は完了仕様 `areka-P0-emo2-conformance-e2e` の手順書 §5.7（点灯語の表・`boot.rs:34` を指す）と記録 §7／§13.2 行 4。
- 隣接語彙の型: 受理系は `info!(target: "kanade", event = "steady_talk_done", ..)`／`"close_refused"`（`talk_id` 付き）／起動系は `"boot_talk"`（`talk_id` 付き）・`"boot_complete"`・`"boot_pending_close"`。要件 3.3 の「info 以上・`event`・`talk_id` 付き・ちょうど 1 行」はこの型に収まる。
- 捕捉基盤 `schedule/log_capture.rs`（cfg(test)）: `capture(|| ..) -> Vec<CapturedEvent>`・`assert_logged(&ev, Level, "event")`・**`assert_not_logged(&ev, "event")`**・`assert_no_error_logs`・`logged_once`。要件 5.1 ⑵「`boot_input_ignored` 0 行」は `assert_not_logged` で、3.3「ちょうど 1 行」は `logged_once` でそのまま表明できる。

### 1.3 既存テストの配置と型

| ファイル | 行数 | 本仕様との関係 |
|---|---|---|
| `boot.rs` | 307 | 本番。腕 1 本＋ヘルパ 1 本を足しても 350 行前後 |
| `boot_sequence_tests.rs` | 584 | `boot::step` の兄弟テスト（`#[path]` 接続）。`boot_greeting_talkdone_correlates_without_unknown_error`（DD-IT-12 の檻）が **`Idle → … → BootVersion{Some(id=1)} → Steady{Some}` を `step` で駆動し、`TalkDone{1, Ended}` を `capture` 付きで投入して相と不在ログを表明する**——本仕様の RED テストはこの系列の「`Notified` を入れる前に `TalkDone` を入れる」写しで書ける。`close_request_during_boot_records_pending_only`／`pending_close_survives_boot_completion` は要件 5.3（保留経路）の雛形 |
| `boot_reply_branch_tests.rs` | 549 | prefetch／初回ゲート／epilogue-only 起動記録トーク（`first_boot_204_204_with_epilogue_emits_epilogue_only_tracked_talk`＝要件 4.4 の経路） |
| `boot_test_support.rs` | 46 | `config()`・`initial()`・`assert_get`・`assert_notify` |
| `schedule_log_firing_tests.rs` | 626 | `warn_boot_input_ignored_logs`（要件 5.4 の既存檻・不変で足りる） |
| `steady_flow_tests.rs` | 924 | `steady_talk_done_*` 3 本（要件 2.4 の既存檻・不変）。**1,000 行に近いので本仕様のテストは置かない** |
| `schedule_tests.rs` | 935 | 同上、置かない |

steering `structure.md`: 新規テストは本番ファイルに書かず兄弟ファイルへ／1 ファイル 1,000 行以下は本番・テストの双方／番人 `file_length_guard_test.rs` が機械で見張る。本仕様の追加先は `boot_sequence_tests.rs`（＋必要なら `boot_test_support.rs` へ「`BootVersion{Some}` まで駆動する」ヘルパを 1 本）が自然。

### 1.4 上流・下流の配線（変更しない範囲の確認）

- `areka-ghost/src/dispatcher.rs` の `on_done` が `KanadeMsg::TalkDone` を kanade へ転送（slot 不一致は stale 破棄）。`actor.rs` の `spawn_kanade_with_stop_sink` が `KanadeMsg::TalkDone(td) => Input::TalkDone(td)` で `drive` へ渡す。本仕様はこの配線に触れない。
- `crates/areka/src/emo2_boot/spine_conformance_*`（決定論一周）は `areka-ghost` の runtime 経由で**本物のアクターシェル**（`spawn_kanade_with_stop_sink`）を使う。`spine_conformance_support_tests.rs` の `kanade_probe_raises_no_shiori_call_and_observes_the_close` の doc が本欠陥を「残る危険（本檻では直せない）」として逐語登記している。

---

## 2. 前提の同一性の検査（⚠ 要件の前提に食い違いが 1 件）

**アクターシェル経由では `BootVersion{..}` 滞在中に `Input::TalkDone` が届く経路が構造上存在しない。**

根拠（`crates/areka-kanade/src/actor.rs`）:
- `drive` は DD-2「execute-batch/reinject-last」: `step` が返した Action 列を `execute_actions` で先頭から全実行し、列中の SHIORI 往復（`round_trip_request`＝`reply_rx.recv()` で**同期ブロック**）の応答を `Input::ShioriReply` として**同じ `drive` 呼出の中で**再投入し、往復の無い Action 列が返るまで反復する。
- 起動系列は `OnInitialize` → username 照会 → `OnFirstBoot`／`OnBoot` → `basewareversion` のすべてが往復を含むので、**`drive(Input::Boot)` 1 回の中で `Idle` から `Steady{talk}` まで一気に進む**。受信箱（`run_inbox`）が次のメッセージを取り出すのは `drive` が返った後＝相はもう `Steady`。
- `to_baseware_version` の Action 列 `[StartTalk, ShioriRequest(basewareversion)]` でも、`StartTalk` を sakura へ送った直後に `basewareversion` の往復でブロックし、応答を受けて `Steady{talk}` へ入ってから受信箱へ戻る。起動記録トーク（空 script）の即時完了通知は受信箱に**積まれる**が、取り出されるときの相は `Steady{Some}` であり、既存の `steady::on_talk_done` が正しく受理する。
- 往復失敗（`Failed`）は `on_shiori_reply` の横断腕で `Unloading{Fault}` へ倒れ、`BootVersion` に留まらない。

帰結:
1. 完了仕様 e2e の記録 §13.2 行 4／タスク注記の「窓は狭い（4 hop 対 2 hop の競争）」は、現行シェルでは「**窓は閉じている**」がより正確。実機一周の `boot_input_ignored` 0 行はタイミングの幸運ではなく構造の帰結である（完了仕様の記録は本仕様では書き換えない——設計で「補足」として言及するかは裁定事項）。
2. 要件の Project Description「決定論の検証環境では起動記録トークが空になり得るため再現できる」は、**純粋状態機械 `step` を直接駆動する in-crate テスト**（`boot_sequence_tests.rs` の型）についてのみ真であり、`crates/areka` の `spine_conformance_*`（アクターシェル経由）では**直す前でも赤にならない**。要件 5.5 の `cargo test -p areka --bin areka` は非回帰の検出器としては働く（直しの前後で緑）が、欠陥の検出器ではない。
3. それでも直す価値はある: `step` は crate の契約上「唯一の遷移入口」であり、`current_talk_id` が `BootVersion{Some}` を突合対象に含めて防御した通知を委譲先が捨てる不整合は、シェルの再投入規律（DD-2）が変わった瞬間に発現する潜在欠陥である（W15 `translate-pipeline`・W17 `lifecycle-events` が `kanade/schedule/*` に触れる予定・roadmap 輻輳点）。「あるべき姿」（純粋機械の自己完結した正しさ）として直す位置づけが整合する。

→ **議題 1**（要件ディスカッションへ）: Project Description の「再現できる」の主語を「純粋状態機械の決定論テスト」に限定して読むことの確認、および設計で上記の構造的事実を明記するか。答えで変わるもの＝要件 5.5 の位置づけ（欠陥検出器か非回帰検出器か）と、設計文書に「アクター経由では到達不能」を書くか。

---

## 3. 要件 → 資産マップ（欠落の種別付き）

| 要件 | 必要な能力 | 既存資産 | 欠落／制約 |
|---|---|---|---|
| 1.1 `BootVersion{Some}` で一致 `Ended` を受理・`talk: None`・待ち点維持 | `boot::step` に `TalkDone` の受理腕 | 突合は `on_talk_done`／`current_talk_id` で成立済み。`steady::on_talk_done` の `Steady{None}` 復帰枝が同型 | **Missing**: `boot.rs` に腕が無い（ワイルドカードが捨てる） |
| 1.2 `Interrupted` を `Ended` と同一に | 理由に依らない腕 | `on_talk_done` が両者を同じ委譲へ流す（3 値写像） | なし（腕を `Input::TalkDone(_)` で書けば自動的に満たす） |
| 1.3 受理後の `Notified` で `Steady{None}` | 既存の `boot_complete` 腕 | `Phase::BootVersion{talk} → Steady{talk}` の引き継ぎがそのまま `None` を運ぶ | なし |
| 1.4 `Quit` は従来どおり | 横断腕 | `on_talk_done` の `Quit` 腕（委譲へ来ない） | なし |
| 1.5 副作用 0 | 腕が空 Vec を返す | `steady_talk_done` 枝と同じ | なし |
| 1.6 `basewareversion` の再送・修正なし | 腕が Action を積まない | 同上。`snapshot_of` は送出時点で撮り済み | なし（受理後 `BootVersion{None}` の `snapshot_of` は非アクティブになるが、誰も再送しない） |
| 2.1 `Steady{None}` 後の終了指示で即 `OnClose` GET | 既存 `steady::on_close_request` の `Steady{None}` 枝 | `steady_none_close_request_begins_handshake_now` が檻 | なし |
| 2.2 保留＋受理→`Steady{None}`→次 Tick で `OnClose` | 既存 Tick 処理の `pending_close.take()` | `pending_close_survives_boot_completion` が同型（挨拶なし版） | なし（テスト追加のみ） |
| 2.3 受理時に握手を始めない | 腕が `pending_close` を触らない | `record_pending_close` の規律 | **Constraint**: `steady::on_talk_done` を写すときに `pending_close.take()` の枝を**写さない**こと |
| 2.4 `Steady{Some}` での完了は従来どおり | 既存 | `steady_talk_done_*` 3 本 | なし |
| 3.1 受理時に `boot_input_ignored` を書かない | 腕がワイルドカードより先に一致 | — | **Missing**（1.1 と同じ 1 か所） |
| 3.2 Tick などは従来どおり warn | ワイルドカード腕を残す | `warn_boot_input_ignored_logs` | なし |
| 3.3 受理を info 以上・`event`・`talk_id` 付き・ちょうど 1 行 | 新語彙 1 つ | 型は `steady_talk_done`／`boot_talk` | **Missing**: 語の命名（議題 3）。`Interrupted` 経路では横断腕の `talk_done_interrupted_as_non_quit` が**別に** 1 行先行する（既存・不変）→「ちょうど 1 行」は受理の語についての条件と読むのが自然（議題 4） |
| 3.4 不一致の扱い不変 | — | `on_talk_done` の `Some(_)｜None` 腕 | なし |
| 4.1〜4.5 起動系列不変 | 既存腕に触れない | `boot_sequence_tests.rs`・`boot_reply_branch_tests.rs` | **Constraint**: 追加は腕 1 本に限定 |
| 5.1／5.3 決定論テスト | `step` 駆動＋`capture` | DD-IT-12 檻・`pending_close_survives_boot_completion` が雛形 | **Missing**: テスト 2〜3 本 |
| 5.2 RED 先行の記録 | 実装の記録 | 直す前は相が `BootVersion{Some}` のまま＋WARN が出るので必ず赤 | なし（記録は実装フェーズ） |
| 5.4 Tick の warn 固定 | 既存檻 | `warn_boot_input_ignored_logs` | なし（「`BootVersion{Some}`＋Tick でも warn」を 1 本足すかは任意・議題 5） |
| 5.5 `cargo test -p areka-kanade`／`-p areka --bin areka` 緑 | — | — | **Constraint**: 後者は §2 のとおり非回帰のみ |
| 5.6 兄弟配置・1,000 行以下 | — | `boot_sequence_tests.rs` 584 行 | なし |

**Unknown（Research Needed）**: なし。外部依存の追加も無い。

---

## 4. 実装アプローチの選択肢

brief は 2 案を挙げる。精読の結果、brief の (b)「`dispatch_phase` が `TalkDone` を相に依らず先に `on_talk_done` へ回す」は**現状既にそうなっている**（`step` が `TalkDone` を無条件に `on_talk_done` へ回し、`on_talk_done` が一致後に `dispatch_phase` へ委譲する）。欠陥は「委譲の順序」ではなく「委譲先に腕が無い」ことにある。よって現実の選択肢は「受理の腕を**どこに**置くか」である。

### 案 A: `boot::step` に `TalkDone` の腕を足す（相固有の遷移を相のモジュールが持つ）

```text
boot::step:
  Input::Boot            => boot_start
  Input::ShioriReply     => on_reply
  Input::CloseRequest    => record_pending_close
  Input::TalkDone(done)  => on_talk_done(state, done)      // 追加
  _                      => warn boot_input_ignored（不変）

boot::on_talk_done:
  Phase::BootVersion{talk: Some(_)} =>
      info!(target:"kanade", event="<新語>", talk_id=done.talk_id.0, ..)
      state.phase = BootVersion{talk: None}
      (state, vec![])
  _ => warn boot_input_ignored と同じ扱い（構造上到達しない防御）
```

- 変更点: `boot.rs` に腕 1 本＋関数 1 本（15〜25 行）。`mod.rs` は無変更。
- 既存規律との整合: `mod.rs` ヘッダ「フェーズ固有の遷移は `boot`／`steady`／`close` へ委譲」に忠実。`steady::step`・`close::step` が自分の `TalkDone` 腕を持つのと**対称**になる。DD-IT-12「アクティブな talk を運ぶ相」＝`BootVersion{Some}` の枠の書き換えを、その相を所有する `boot.rs` が行う。`on_talk_done` の「横断 → 委譲」順序は不変。
- ✅ 最小差分・対称・順序規律不変。❌ `boot.rs` の `_` 腕コメント（「Tick・TalkDone など」）の更新が要る（1 行）。

### 案 B: `mod.rs` の `on_talk_done` が `BootVersion{Some}` を横断的に受理する（委譲しない）

```text
on_talk_done（一致・非 quit）:
  if let Phase::BootVersion{talk: Some(_)} = state.phase {
      info!(..); state.phase = BootVersion{talk: None}; return (state, vec![])
  }
  dispatch_phase(..)
```

- 変更点: `mod.rs` に 8〜12 行。`boot.rs` は無変更（ワイルドカードは残る）。
- 既存規律との整合: 相固有の状態書き換えを横断層が持つことになり、`mod.rs` ヘッダの責務分割（横断＝Quit／ForceQuit／ShioriDown／Failed・相固有＝各サブモジュール）に**反する**。`current_talk_id` の防御コメントと同じ場所に処理が集まる利点はあるが、`Steady` の受理が `steady.rs` にあるのとの非対称が残る。
- ✅ `boot.rs` に触れない。❌ 責務境界を崩す・将来 `boot.rs` を読む人が受理経路を見つけられない。

### 案 C: `dispatch_phase` を `TalkDone` 専用の相別分配に分ける（`on_talk_done_by_phase`）

- `on_talk_done` の委譲先を `dispatch_phase` から新設の `dispatch_talk_done`（`BootVersion` → `boot::on_talk_done`／`Steady` → `steady::on_talk_done`／`CloseTalkWait` → `close::…`）へ差し替える。
- 変更点: `mod.rs` に関数 1 本＋`boot.rs`／`steady.rs`／`close.rs` の `on_talk_done` を `pub(super)` 化。
- ✅ `TalkDone` の到達先が 1 表で見える。❌ 差分が 3 ファイルに広がり、要件 4（起動系列以外を変えない）と Out of scope（`steady.rs`／`close.rs` 非接触）に触れる。本仕様の規模（S）に対して過剰。

### 比較

| 観点 | 案 A | 案 B | 案 C |
|---|---|---|---|
| 触るファイル | `boot.rs`＋兄弟テスト | `mod.rs`＋兄弟テスト | `mod.rs`・`boot.rs`・`steady.rs`・`close.rs`＋テスト |
| DD-IT-12／`mod.rs` の責務分割との整合 | 整合（対称） | 崩す | 整合だが再編 |
| Out of scope（`steady.rs`／`close.rs` 非接触）| 守る | 守る | 破る |
| 要件 3.2（ワイルドカードの warn 不変） | 守る | 守る | 守る |
| 差分の大きさ | 最小 | 最小に近い | 中 |

ギャップ分析としての所見: 案 A が既存の順序規律（横断を先に・相固有は相のモジュール）に最も素直に乗る。案 B は「`current_talk_id` の防御と受理を同じ場所に置きたい」動機があるときのみ検討に値する。案 C は本仕様の範囲を超える。最終選択は設計フェーズ。

---

## 5. 決定論テストの設計材料

追加先: `boot_sequence_tests.rs`（`boot::step` の兄弟・584 行）。必要なら `boot_test_support.rs` に「`Idle → BootVersion{Some(id=1)}` まで駆動する」ヘルパを 1 本（DD-IT-12 檻と本仕様の 2〜3 本で共有）。

| # | 系列 | 表明 | 直す前 |
|---|---|---|---|
| T1（要件 5.1） | `Boot` → `Notified` → `NoContent`(prefetch) → `NoContent`(OnFirstBoot) → `Value("greeting")`(OnBoot) → `BootVersion{Some(1)}`。ここで `capture` 付きで `TalkDone{1, Ended}` | ⑴ 相が `BootVersion{talk: None}`・Action 空 ⑵ `assert_not_logged(.., "boot_input_ignored")`・`logged_once(INFO, <新語>)`・`assert_no_error_logs` ⑶ `Notified` → `Steady{None}` ⑷ `CloseRequest` → `assert_get(actions[0], &events::on_close(reason, &INACTIVE))`＋`ClosePending` | 赤（相が `Some` のまま・WARN あり・⑷で `steady_close_pending` になり GET が出ない） |
| T2（要件 5.3） | T1 の `BootMain` 到達後に `CloseRequest` を入れて `pending_close` を作る → `Value` → `BootVersion{Some}` → `TalkDone{Ended}` → `Notified` → `Steady{None}` → `Tick` | `OnClose` GET＋`ClosePending`。`TalkDone` 受理時点では Action 空（要件 2.3） | 赤 |
| T3（要件 1.2・任意） | T1 と同じで理由を `Interrupted` | T1 ⑴⑵ と同じ（先行 info `talk_done_interrupted_as_non_quit` は既存） | 赤 |
| T4（要件 5.4・任意） | `BootVersion{Some}` に `Tick` | `boot_input_ignored` WARN あり・相不変 | 緑（受理腕が広すぎたら赤になる較正） |

既存 `warn_boot_input_ignored_logs`（`BootInit`＋Tick）は不変で要件 5.4 を満たす。`spine_conformance_*` は §2 のとおり前後とも緑（非回帰のみ）。

---

## 6. 規模・リスク

- **Effort: S**（半日〜1 日）。腕 1 本＋テスト 2〜4 本、既存パターンの写し。
- **Risk: Low**。純粋関数の分岐追加・外部依存なし・配線非接触。残るリスクは「`steady::on_talk_done` を写すときに `pending_close` 消化まで写す」誤り（要件 2.3 違反）で、T2 が検出する。

---

## 7. 設計フェーズへの申し送り（Design-decision items）

1. ~~**要件の前提の同一性**（§2）~~ — **要件ディスカッションで解決済み（2026-09-13・開発者裁定）**。裁定は「予定どおり今直す」。実在の証拠は使い捨ての決定論テストで実測した（`BootVersion{talk: Some(id=1)}` に `TalkDone{1, Ended}` を投入 → 相が変わらず `boot_input_ignored` が WARN で点灯 → 枠が `Steady{talk: Some}` へ漏れ → 終了指示で `OnClose` GET が **0 件**。完了通知を 1 手後ろにずらした既存テスト `boot_greeting_talkdone_correlates_without_unknown_error` は緑）。この裁定に伴い requirements.md を改訂した: Project Description の発現条件をタイミング論から構造論（`actor.rs` の `drive` の同期再投入）へ・Introduction に「到達可能性」の段を追加・Out of scope にシェルの待ち方の変更を明記・要件 4.4 の「決定論の検証環境」を「純粋状態機械の決定論テスト」へ・**要件 5.5 に「`-p areka-kanade` が欠陥の検出器／`-p areka --bin areka` は非回帰の検出器」を明記**。設計では「到達不能の根拠」を `actor.rs` の `drive` を引いて記すこと。完了仕様 e2e の文書は書き換えない。
2. ~~**受理腕の置き場所**（§4）: 案 A（`boot.rs`）／案 B（`mod.rs` 横断）／案 C（`TalkDone` 専用分配）。既存の順序規律との整合は案 A が最も素直。~~ — **設計で解決（2026-09-17・§9 D1）**: 案 A。`boot::step` にガード付きの腕 `Input::TalkDone(done) if matches!(state.phase, Phase::BootVersion { talk: Some(_) })` を 1 本足し、新設 `boot::on_talk_done` へ委譲する。
3. ~~**受理ログの語**（要件 3.3）: 候補 `boot_talk_done`（`steady_talk_done`／`close_refused` と並ぶ命名）／`boot_talk_done_early`（起動完了前であることを語に込める）。level は info（`steady_talk_done` と同じ）。フィールドは `talk_id` 必須・`origin`（`"boot"`）は任意。~~ — **設計で解決（2026-09-17・§9 D2）**: `boot_talk_done`・info・target `kanade`・フィールドは `talk_id` のみ（`origin` は付けない）。
4. ~~**「ちょうど 1 行」の読み**（要件 3.3）~~ — **要件ディスカッションで解決済み（2026-09-11・カテゴリ A）**。要件 3.3 の本文に「数えるのは受理の語の行だけ・横断遷移の既存ログ（`talk_done_interrupted_as_non_quit` など）は数えない」と明記した。設計での再裁定は不要。
5. ~~**較正テストの追加**（要件 5.4）: `BootVersion{Some}`＋Tick で `boot_input_ignored` が**出続ける**ことを 1 本足すか（受理腕が広すぎる退行の検出器）。既存 `warn_boot_input_ignored_logs`（`BootInit`＋Tick）だけで足りるとするか。~~ — **設計で解決（2026-09-17・§9 D4）**: 足す（T4・`schedule_log_firing_tests.rs` の既存 `warn_boot_input_ignored_logs` の直後）。受理が入る相そのもので Tick が従来どおり捨てられることを固定する。
6. ~~**選択帳簿の掃除の対称性**: `steady::on_talk_done` は `clear_choice_ledger` を呼ぶが、起動中は `ChoiceWaiting` が非 Steady で棄却されるため帳簿は構造上 `None`。受理腕で呼ばない（最小）か、対称性のために呼ぶ（trace のみ・害なし）か。~~ — **設計で解決（2026-09-17・§9 D3）**: 呼ばない。構造上 `None` の帳簿に対する呼出はテストで区別できない空振りであり、握手開始時の `begin_close` が別途掃除する。
7. ~~**`boot.rs` ワイルドカード腕のコメント**: 「Tick・TalkDone など」の文言から `TalkDone` を外す（1 行）。綴り `boot_input_ignored` とメッセージは不変。~~ — **設計で解決（2026-09-17・§9 D5）**: 「上記以外（Tick など）」へ改める。綴り・メッセージ・レベル・発行点は不変。

## 8. Research Needed

- なし（外部依存・未知技術ともに無し）。§2 の構造的事実は `actor.rs` の `drive`／`execute_actions`／`round_trip` の精読で確認済み。設計での再確認点は「`run_inbox` が 1 メッセージ 1 `drive` であること」（`crates/areka-actor/src/spawn.rs` の `run_inbox`）のみ。→ **§9.1 で確認済み（2026-09-17）**。

---

## 9. 設計フェーズの追記（kiro-spec-design・2026-09-17）

### Summary
- **Feature**: `areka-P0-kanade-boot-talkdone-drop`
- **Discovery Scope**: Extension（既存状態機械への腕 1 本の追加）→ light discovery をメインコンテキストで実施（サブエージェント派遣なし・外部調査なし）
- **Key Findings**:
  - §2 の到達不能の根拠を再確認した（§9.1）。`run_inbox` は 1 メッセージ 1 ハンドラ呼出で、ハンドラは 1 回の `drive` を呼ぶ。
  - `boot::step` のワイルドカード腕にガード付きの `TalkDone` 腕を前置すれば、`boot_input_ignored` の発行点を 1 か所に保ったまま、到達しない `TalkDone` の防御を既存のワイルドカードに委ねられる（第 2 の防御腕も第 2 の発行点も要らない）。
  - 既存テストの基線: `cargo test -p areka-kanade --lib boot_` は 23 本すべて緑（2026-09-17 実測・`boot_greeting_talkdone_correlates_without_unknown_error` を含む）。
  - 直る欠陥を「残る危険（本檻では直せない）」と逐語で書いたコード内 doc が `crates/areka/src/emo2_boot/spine_conformance_support_tests.rs` に残る（§9.2）。

### 9.1 Research Log

#### 到達可能性の再確認（要件の前提）
- **Context**: 要件ディスカッションの裁定（§7 項目 1）で「設計に到達不能の根拠を `actor.rs` の `drive` を引いて記す」とされた。
- **Sources Consulted**: `crates/areka-kanade/src/actor.rs` の `drive`（DD-2 同期往復ループ）・`execute_actions`・`spawn_kanade_with_stop_sink` の受信ハンドラ／`crates/areka-actor/src/spawn.rs` の `run_inbox`／`crates/areka-kanade/src/schedule/boot.rs` の `to_baseware_version`。
- **Findings**: `drive` は `step` の指示列を全実行し、往復応答を同じ呼出の中で `Input::ShioriReply` として再投入し、往復の無い指示列が返るまで反復する。`to_baseware_version` は必ず `basewareversion` の往復を積む。`run_inbox` は `recv` ごとにハンドラを 1 回呼び、ハンドラは `KanadeMsg` を `Input` へ写して `drive` を 1 回呼ぶ。
- **Implications**: 受信箱が `TalkDone` を取り出す時点で相は `Steady`。`spine_conformance_*` は非回帰の検出器（design Overview「到達可能性」）。

#### 受理腕の形（ガード付き腕 vs 受理関数内の防御腕）
- **Context**: 案 A で `boot::on_talk_done` を新設すると、相が `BootVersion{Some}` でない場合の腕をどう書くかが残る。
- **Findings**: 受理関数の中で `_ => warn boot_input_ignored` を書くと点灯語の発行点が 2 か所になる（完了仕様 e2e の手順書 §5.7 は 1 か所を指す）。沈黙の `_ => (state, vec![])` は log-first に反する。`step` の腕にガード `if matches!(state.phase, Phase::BootVersion { talk: Some(_) })` を付ければ、外れる `TalkDone` は既存のワイルドカード腕へ落ちて従来どおり warn になり、受理関数は前提が確定した状態だけを受ける。
- **Implications**: design「boot::step 受理腕」「boot::on_talk_done」の契約。`matches!` は `Some(_)` を束縛しないので `state` を動かさず、腕本体で `state` を move できる。

#### テストの表明手段
- **Sources Consulted**: `schedule/log_capture.rs`（`capture`／`assert_logged`／`assert_not_logged`／`assert_no_error_logs`／`logged_once`・`CapturedEvent.fields` に `talk_id` の文字列値）・`boot_test_support.rs`・`schedule_log_firing_tests.rs` の `run_step`／`state_in`。
- **Findings**: `Phase`／`State`／`ActiveTalk` は `Debug` を持たない（`mod.rs` の定義に derive なし）。`config()` は `KanadeConfig::new("master", "1.0.0")` で `first_boot: true`・`first_boot_epilogue` 空。`BootVersion{Some(id=1)}` へは `Boot` → `Notified` → `NoContent` → `Value("greeting")` の 4 入力で到達する（`OnFirstBoot` にスクリプトが返る経路＝`boot_type_script`・`OnBoot` を飛ばす）。
- **Implications**: design Testing Strategy（`matches!` で表明・`logged_once(..).fields["talk_id"] == "1"`・T4 は `run_step` で 1 行）。

### 9.2 隣接文書の陳腐化（境界の保守）
- `crates/areka/src/emo2_boot/spine_conformance_support_tests.rs` の `kanade_probe_raises_no_shiori_call_and_observes_the_close` の doc に「残る危険（本檻では直せない）——…`schedule/boot.rs:32-36` の防御アームが**それを捨てる**」の段落がある。本仕様の着地でこの主張は反対の意味になる（参照の綴りは残り中身だけが反転する型・`ukadoc-survey-sakura-script` の申し送りと同型）。設計はこの段落だけをコメントとして追随させる（File Structure Plan・コードと表明は不変）。同 doc の他の段落にある `boot.rs:31`／`:285-288` の行番号は本変更で後者がずれるが、意味は変わらないので触らない（既知の行番号ドリフト・「何の定義か」で指す規律の適用は別途）。
- 完了仕様 e2e の文書（手順書 §5.7 の表・記録 §7／§13.2・design D9）が指す `boot.rs:34`／`:33-36` は、腕の追加でワイルドカード腕が下へずれる。要件ディスカッションの裁定により書き換えない。実体は「`boot::step` のワイルドカード腕」で不変。

### 9.3 Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| 案 A（採用） | `boot::step` にガード付き `TalkDone` 腕＋`boot::on_talk_done` | 最小差分・`steady`／`close` と対称・`mod.rs` の順序規律不変・発行点 1 か所 | ワイルドカード腕の注記を 1 行直す | §4 案 A＋§9.1 のガード形 |
| 案 B | `mod.rs` の `on_talk_done` が `BootVersion{Some}` を横断受理 | `boot.rs` に触れない | 相固有の状態書き換えを横断層が持つ＝責務分割に反する・非対称 | 不採用 |
| 案 C | `TalkDone` 専用の相別分配を新設 | 到達先が 1 表で見える | 3 ファイルに差分・Out of scope（`steady.rs`／`close.rs` 非接触）に触れる | 不採用 |

### 9.4 Design Decisions

#### D1: 受理腕の置き場所＝案 A（`boot.rs`・ガード付き腕）
- **Context**: §7 項目 2。
- **Alternatives Considered**: 案 A／案 B／案 C（§9.3）。
- **Selected Approach**: `boot::step` の `CloseRequest` 腕の直後に `Input::TalkDone(done) if matches!(state.phase, Phase::BootVersion { talk: Some(_) }) => on_talk_done(state, done)` を置く。
- **Rationale**: 相固有の遷移は相のモジュールが持つ（`mod.rs` ヘッダの責務分割）。ガード形により防御は既存のワイルドカード腕に一本化される。
- **Trade-offs**: 受理関数の前提（相の確定）が呼出側のガードに依存する——私的関数で呼出点は 1 か所なので doc に契約を書く。
- **Follow-up**: レビューで「ガード付きの腕 1 本・`boot_input_ignored` の発行点 1 か所」を確認する。

#### D2: 受理ログの語＝`boot_talk_done`（info・`talk_id` のみ）
- **Context**: §7 項目 3。
- **Alternatives Considered**: `boot_talk_done_early`／`origin` フィールド付き。
- **Selected Approach**: `info!(target: "kanade", event = "boot_talk_done", talk_id = done.talk_id.0, "起動挨拶 talk 完了——basewareversion 応答待ちを維持しつつ枠を空にする")`。
- **Rationale**: `steady_talk_done`／`close_refused` と並ぶ `<相>_talk_done` の命名。「起動完了前」は相名が表す。`BootVersion` の追跡は常に `origin="boot"` なので `origin` は情報を増やさない。
- **Follow-up**: T1 の `logged_once` で `talk_id` の値まで突合する。

#### D3: `clear_choice_ledger` は呼ばない
- **Context**: §7 項目 6。
- **Selected Approach**: 受理関数は `state.choice`／`choice_prev_talk` に触れない。
- **Rationale**: 起動中は `ChoiceWaiting` が `choice_waiting_stale` で棄却され帳簿は構造上 `None`。呼んでも trace の空振りにしかならず、テストで区別できない呼出は置かない（到達する経路だけを書く）。握手開始時の `begin_close` が別途掃除する。
- **Trade-offs**: `steady::on_talk_done` との字面の対称は崩れる——doc に理由を書く。

#### D4: 較正テスト T4 を足す
- **Context**: §7 項目 5。
- **Selected Approach**: `schedule_log_firing_tests.rs` の `warn_boot_input_ignored_logs` の直後に、`BootVersion{Some}`＋Tick で `boot_input_ignored` が出続け `boot_talk_done` が出ないことを 1 本足す。
- **Rationale**: 受理が入る相そのもので「無関係な入力は従来どおり捨てる」を固定する（要件 3.2・5.4）。既存の `BootInit`＋Tick だけでは受理腕が Tick まで広がる退行を見ない。

#### D5: ワイルドカード腕の注記
- **Context**: §7 項目 7。
- **Selected Approach**: 「上記以外（Tick・TalkDone など）」→「上記以外（Tick など）」。綴り・メッセージ・レベル・発行点は不変。

#### D6: 隣接 doc コメントの追随（`spine_conformance_support_tests.rs`・コメントのみ）
- **Context**: §9.2。
- **Alternatives Considered**: 触らない（Out of scope の「areka 側の配線」に含めて読む）／段落を追随させる。
- **Selected Approach**: 「残る危険（本檻では直せない）」の段落だけを、本仕様で `boot::on_talk_done` が受理するようになった旨へ改める。テスト本体・表明・他の段落は不変。
- **Rationale**: 直った欠陥を「残る危険」と書き続けるのは、参照が残って中身だけ反転する既知の罠。配線（`spine.rs`）には触れないので Out of scope と衝突しない。

#### D7: テストの配置と共有ヘルパ
- **Selected Approach**: T1〜T3 は `boot_sequence_tests.rs`（584 → 720 行前後）、T4 は `schedule_log_firing_tests.rs`（626 → 645 行前後）、ヘルパ `boot_until_version_with_greeting(cfg, pending)` は `boot_test_support.rs`。既存 `boot_greeting_talkdone_correlates_without_unknown_error` は書き換えない。
- **Rationale**: `structure.md` の兄弟配置・1,000 行以下・共有ヘルパは `<stem>_test_support.rs` へ集約。`steady_flow_tests.rs`（924 行）・`schedule_tests.rs`（935 行）には置かない。

### 9.5 Synthesis（generalization／build-vs-adopt／simplification）
- Generalization: 要件 1.1／1.2／4.4 は「`BootVersion{Some}` に届いた一致・非 quit の通知」という 1 つの経路であり、理由と追跡トークの由来（挨拶／起動記録）で分岐しない腕 1 本で満たす。
- Build vs Adopt: 既存の `steady::on_talk_done` の保留なしの枝を写す。新規の機構・依存なし。
- Simplification: 受理関数内の防御腕・`origin` フィールド・帳簿掃除・`mod.rs` の変更を落とした。残るのは腕 1 本・関数 1 本・注記 1 行・テスト 4 本・ヘルパ 1 本・doc 1 段落。

### 9.6 Risks & Mitigations
- `pending_close` の消化まで写す誤り（要件 2.3 違反）— T2 が赤になる。
- 名前指定の `cargo test` を `--lib` 無しで走らせて母数 0 の緑を「赤の記録」と取り違える — design 検証手順に `--lib` を明記。
- 完了仕様 e2e の文書の行番号（`boot.rs:34`）がずれる — 書き換えない裁定・実体は「ワイルドカード腕」で不変（§9.2）。
- 壁時計デッドラインを持つ `crates/areka` のテストは他の cargo と並走させると赤になる — 非回帰の全走は単独で行う。

### 9.7 設計レビューゲート（2026-09-17）
- 機械検査: 要件 ID 25 件（1.1〜5.6）すべてが design.md の traceability 表に存在・Boundary 4 節と File Structure Plan は具体・全コンポーネントがファイルへ対応。
- 判断検査: 要件の曖昧・矛盾なし。修正 1 回（受理腕の Risks の文言を明確化）でゲート通過。

### 9.8 References
- `crates/areka-kanade/src/actor.rs` の `drive`／`spawn_kanade_with_stop_sink`・`crates/areka-actor/src/spawn.rs` の `run_inbox`・`crates/areka-kanade/src/schedule/{mod.rs, boot.rs, steady.rs, close.rs, log_capture.rs}`。
- 完了仕様 `areka-P0-idle-talk`（DD-IT-12）・`areka-P0-emo2-conformance-e2e`（手順書 §5.7・記録 §7／§13.2 行 4・design D9）。
- steering: `structure.md`（兄弟テスト・1,000 行）・`logging.md`（構造化フィールド）・`roadmap.md` W13 ②。
