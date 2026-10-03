# ギャップ分析: areka-P0-status-execution-states

- 調査日: 2026-10-03
- 基準: ブランチ `claude/areka-p0-status-execution-bf7596`（main `1ce4c74e` ＋ spec 初期化 `6e792918`）
- 対象: 確定した requirements.md（要件 1〜8）と、いまのコード
- やり方: コードを読んで突き合わせた（外部の依存は増やさない前提なので、外部の調べものは不要だった）。pasta はこのワークツリーに取り出されていないため、本体の取り出し先 `C:\home\maz\git\areka\vendors\pasta`（`48c42fc3`）を読み取りだけで見た。

## 1. 要約

- **組み立ての側はほぼ揃っている**。`crates/areka-kanade/src/status.rs` は 10 状態の語彙・正典順・カンマ連結・空なら行を出さない・`balloon(0=2/1=0)` の書式（`BalloonBindings`／`BalloonBinding`）をすでに持つ。足りないのは「材料」＝`ExecutionSnapshot` が `talk_active` と `choice_active` の 2 本しか持たないことと、導出表の 3 行（nouserbreak・online・balloon）が空のままであること。
- **材料を kanade へ運ぶ道が 3 つとも無い**。中断の無効化の旗は UI スレッドの `UserBreakWiring` にだけあり（読む者がまだ居ない）、バルーンの表示は表示層 `EmoPresenter` にだけあり、ネットワークの状態は更新の窓口 `UpdateDesk`（UI）と URL 取得の使い捨てスレッドに散らばっている。kanade へ UI の状態を届ける `KanadeMsg` は今のところ無い（`Tick` は areka-ghost の ticker スレッドが時刻だけを運ぶ）。
- **nouserbreak の「載せる区間」と「実際に中断を断る区間」は少しずれる**。UI の旗はトークが終わっても下りず、次のトークの最初の指示で下りる。新しいトークの立ち上がり直後のごく短い間、前のトークの旗が残る（要件 3.3 と 3.4 がこの間だけ食い違う）。また areka が断るのはバルーンの左ダブルクリックによる中断だけで、マウスのダブルクリックの応答による差し替えは無効化モード中でも起きる。
- **online は触るファイルが広がりやすい**。更新は UI の窓口の段（`Stage::Running`）で始まりと終わりが分かるが、URL からの取得は失敗したとき何も届かない。さらに online はプロセスに 1 つの状態で、ゴーストの切替（更新の読み直しを含む）をまたいで続くため、新しい kanade へ今の値を渡す手当てが要る。
- **約束したファイルの境界は守れる見込み**。届け元を `input_events/user_break.rs`・`emo2_boot/balloon_visibility_phase.rs`・`update/desk.rs`（＋ URL 取得側）に置けば、`balloon_visibility.rs`・`input_events/balloon.rs`・`emo2_boot/mod.rs`・`input_events/mod.rs`・`frame/drain_resnap.rs` に触らずに済む。ただし「新しいファイルを足す」ときの宣言先が限られる（main.rs・`emo2_boot/mod.rs`・`input_events/mod.rs` には宣言を足せない）。

規模 **M**（6〜8 タスク）・リスク **中**。

---

## 2. いまあるもの

### 2.1 Status の組み立て（kanade）

| もの | 場所 | 現状 |
|---|---|---|
| 語彙 10 状態・正典順の序数・綴り | `status.rs` の `ExecutionState`・`canonical_index`・`render_token` | 揃っている。`Balloon(BalloonBindings)`・`Opening(OpeningKinds)` の内側は `/` 区切り（要件どおり） |
| 連結・重複除去・空なら `None` | `status.rs` の `ExecutionStatus::from_states`・`render` | 揃っている（要件 1.1〜1.3 は今のまま満たす） |
| 導出表 | `status.rs` の `ExecutionStatus::derive` | `talking`・`choosing` だけ。残り 8 行は注記のみ |
| 材料 | `status.rs` の `ExecutionSnapshot`（`Copy`・2 本の真偽値）と `ExecutionSnapshot::INACTIVE` | 3 状態の欄が無い |
| 材料の作り手 | `schedule/mod.rs` の `State::snapshot`・`State::snapshot_with_choice`・`snapshot_of(&Phase)` | kanade の運行状態（`State`）だけを読む |
| 送り口・記録 | `actor.rs` の `round_trip_request`（`trace!(event="shiori_request")` に Status を残す）・`shiori/real.rs`（ヘッダの位置） | 変えなくてよい（要件 1.5・7.2 は今のまま満たす） |

**注意点**:
- `BalloonBindings::new` は並べ替えをしない。要件 4.3（キャラクターID の小さい順）は作り手か導出のどちらかで保証する必要がある。
- 空の `BalloonBindings` を `Balloon` に包むと `balloon()` が出る。要件 4.4 のため、導出で「空なら載せない」を明示する必要がある。
- `ExecutionSnapshot` は `Copy` で、`BalloonBindings`（`Vec` を持つ）を入れると `Copy` が外れる。`INACTIVE` は `Vec::new()` が const なので定数のまま作れる。

### 2.2 スナップショットを作っている所（要件 5.1 に効く）

`INACTIVE`（全部なし）を直接渡している本番の呼出:

| 呼出 | 場所 | 送るイベント |
|---|---|---|
| 起動の各段 | `schedule/boot.rs`（`boot_start`・`on_reply` の BootInit／BootType・`boot_root`・根の無い枝） | OnInitialize・username（照会）・OnFirstBoot／OnGhostChanged／更新の起動の知らせ・OnBoot |
| 切替 | `schedule/change.rs`（受理時・204 の後） | OnGhostChanging・OnClose(system) |
| 終了の握手 | `schedule/steady.rs`（握手の開始） | OnClose |
| 強制終了 | `schedule/mod.rs` の `force_quit`（`snapshot_of` で Unloading 後＝実質 INACTIVE） | OnClose（NOTIFY） |

`state.snapshot()` を使っている所: マウス（`steady::on_mouse`）・選択系（`snapshot_with_choice(true)`）・OnSecondChange・汎用の入口（`change::on_raise_event` → `events::raise`。更新の OnUpdate* はここを通る）・台詞の切れ目（`talk_gap`）・リソース照会（`actor_resources`）。basewareversion は `snapshot_of(&state.phase)`。

→ 「`INACTIVE`＝会話も選択もない」という意味で使われている所に、要件 5.1 は「online・nouserbreak・balloon も同じ規則で載せよ」と求める。起動・終了・切替のイベントにも、そのとき見えているバルーンや走っている通信を載せるのかを決める必要がある（議題 5）。

構造体リテラル `ExecutionSnapshot { .. }` は 47 か所（本番は `schedule/mod.rs` の 2 関数と `resources.rs` のテスト部分程度、大半はテスト）。欄を足すと全部がコンパイルエラーになる。`status.rs` のテスト `inactive_snapshot_has_every_source_false` は、欄が増えたら既定値の判断を迫る目的でわざと網羅のリテラルを使っている。

### 2.3 nouserbreak の出どころ

- talk スレッド: `emo2_boot/user_break_cue.rs` の `NoUserBreakCueSink` が `\![enter,nouserbreakmode]`／`\![leave,nouserbreakmode]` と「このトークの最初の指示」（`TalkStarted`）を 1 本の線で UI へ流す。合図にトークの番号は付いていない（`dola::cue::TalkCue` にも無い）。
- UI スレッド: `input_events/user_break.rs` の `UserBreakWiring.no_user_break`。畳み方は純関数 `fold_no_user_break`（Enter→立てる・Leave→下ろす・TalkStarted→下ろす）、取り出しは `drain_no_user_break_signals`（Input の段・押下の判定より前）。読み口 `no_user_break()` は「読む者がまだ居ない」ため `#[allow(dead_code)]`。
- **`UserBreakWiring` は kanade への送出端 `kanade: Sender<KanadeMsg>` をすでに持っている**（ゴーストごとに `wire_user_break` で作り直される）。旗が変わったときに kanade へ送るのは、このファイルの中だけで書ける。ゴーストごとに作り直されるので、前のゴーストの旗を持ち越さない（要件 3.5）のも作りのまま満たす。
- 判定: `judge_press` の順は ①左ダブルクリックか ②選択の確定に消費されたか ③バルーンが出ているか ④旗が立っているか ⑤受理。kanade 側 `schedule/user_break.rs` の `on_user_break` は旗を見ない（再生中なら止める・二重に止めない・再生中でなければ何もしない）。

### 2.4 balloon の出どころ

- 表示の有無: `EmoPresenter::target_visible(balloon_target(scope))`（`crates/areka-emo-present/src/presenter/read.rs`）。balloon-visibility が「唯一の情報源」と定めたもの。
- バルーンの番号: `EmoPresenter::current_surface_id(balloon_target(scope))`。`\b[N]` の N は `areka-seriko` の `resolve_balloon_key` でそのまま数値になり、`emo2_boot/adapter.rs` の `map_display_command` が `ShowSurface{surface_id}` として転写する（偶奇の付け替えなどは無い）。起動時の装着（`frame/attach.rs`）でバルーンは面 0 を「見えないまま確立」するので、`\b` を書かない台本では 0 になる（要件 4.2 の「選ばれていなければ 0」と合う）。`\b[-1]`・Hide の後は `None`。
- 毎フレームの観測: `emo2_boot/balloon_visibility_phase.rs` の `run_balloon_visibility_phase`（`frame.rs` の相順で drain の直後に呼ばれる）。装着済みバルーンの scope を昇順に並べ、`target_visible` を読む `collect_observations` を持つ。**この相の終わりが「このフレームの最終の見え方」になる**（`\b[-1]` の適用・中断で隠す・タイムアウトで隠す、のどれもこの時点までに済む）。
- このファイルは `balloon_visibility.rs` の子モジュール（`#[path = "balloon_visibility_phase.rs"] mod phase;`）。子の中身を書き換えるだけなら親に触らないが、型を親（`BalloonVisibilityState` など）へ足すことはできない。
- `Emo2Wiring`（`frame/wiring.rs`）はゴーストごとに作り直されるが、kanade への送出端は持っていない。kanade へは World の `GhostSlot` → `GhostSession::kanade()` から届く。

### 2.5 online の出どころ

| 出どころ | 場所 | 始まり | 終わり | 誰が知っているか |
|---|---|---|---|---|
| ネットワーク更新 | `update/desk.rs` の `UpdateDesk.stage`（UI・プロセスに 1 つ） | メニュー: `DeskAsk::Started`（`OnUpdateProcessExec` に応えが無かった後）で `Running`。台本: `hand_over` で即 `Running` | `DeskAsk::OrderDone` で `Idle` | UI の窓口。段の変化は `set_stage` 1 か所に集まっている |
| 〃（背景） | `update/worker.rs`（スレッド `update`） | `run_order` の開始 | `run_order` の終わり→`OrderDone` | 背景スレッド。仕事ごとに頼んだ時点の kanade の送出端 `job.kanade` を持つ |
| URL からのインストール | `install/fetch_url.rs` の `spawn_download` → `fetch_and_send`（使い捨てスレッド `install-fetch`） | スレッドの開始 | 成功: 窓口へ `RawInstallRequest` を送る。**失敗: `error!` を残すだけで何も届かない** | そのスレッドだけ |

- 本番で取得口の型を綴るのはこの 2 ファイルだけ、という字面の検査がある（`update/worker_tests.rs` の `winhttp_is_spelled_only_in_two_production_files`）。online の印をこの 2 ファイルの中に置く分には検査に引っかからない。
- 更新の手続きの最中に「同じゴーストの読み直し」（`DeskAsk::Reload` → `request_ghost_switch`）が起きる。kanade はゴーストごとに作り直されるので、kanade の中だけに online を持つと、読み直しの後の新しい kanade は online を知らない。URL 取得も切替をまたいで続きうる。要件は online の持ち越しを禁じていない（禁じているのは nouserbreak と balloon だけ）ので、新しい kanade へ今の値を渡す手当てが要る。
- 更新の窓口には「定常に着いた」ときの入口 `update::desk::on_steady` がすでにある（`ghost_switch` の定常到達の腕から呼ばれる）。ただし起動の途中の呼出（OnBoot など）はその前に出る。

### 2.6 UI から kanade へ届ける形（前例）

- `KanadeMsg::UserBreak`・`Choice`・`Mouse`・`ChangeGhost`・`RaiseEvent` は、UI の持ち物が持つ送出端から送る（失敗は `error!`。例 `balloon_break_send_failed`）。
- `KanadeMsg::ResourceQuery` は運行表を通さず殻（`actor.rs`）がその場で答える前例。状態を「覚えるだけ」の通知も、殻か運行表のどちらで受けるかを選べる。
- `KanadeMsg` を足すと `msg.rs` のラベル関数 2 つ（`KanadeMsg::Tick { now: _ } => "Tick"` の並び）と `actor.rs` の振り分けに腕が要る。

### 2.7 テスト

- 一周の照合 `emo2_boot/spine_conformance_script.rs` の `expected_statuses()`: basewareversion・会話中の OnSecondChange・撫で・メニュー・選択の 4 呼出がいまは `talking`／`talking,choosing`。バルーンが見えている場面なので、要件 8.3 に従えば `talking,balloon(…)` などへ改まる。会話可の OnSecondChange（いま `None`）も、バルーンのタイムアウト前なら `balloon(0=0)` になりうる。
- 一周の照合の土台 `spine.rs` は本物の `EmoPresenter` をテストスレッドで動かすが、`GhostSlot`・`UserBreakWiring` を据えているかは未確認（議題外の調査 R2）。
- `spine_conformance_support.rs` の `status_ledger_reads_choosing_where_references_cannot` が `ExecutionSnapshot` を手で組む。
- 実 pasta を使う試験は `crates/areka-ghost/tests/ghost/real_pasta_test.rs` だけで、`HOST32_PASTA_DLL` があるときだけ走る（無ければ黙って抜ける）。中身は「起動から終了まで完走したか」だけを見る。

### 2.8 pasta の読み方（要件 8.4 の前提の確認）

`vendors/pasta/crates/pasta_lua/pasta_scripts/pasta/shiori/event/virtual_dispatcher.lua`（`48c42fc3`）の `M.is_blocked` は、`BLOCKED_STATUSES`（talking・choosing・online・opening・passive・induction・timecritical・nouserbreak）のどれかを `string.find(…, 1, true)`（部分一致）で含めば雑談を止める。`balloon(0=2/1=0)` はこのどの語も含まないので止めない。要件の Adjacent expectations の記述と一致する。`STORE.kick_force` が立っていると 1 回だけこの抑止を突破する点は注意（試験で online を確かめるとき、この旗が立たない状況で見る必要がある）。

---

## 3. 要件と資産の対応

凡例: **在る**＝今のまま使える／**欠け**＝作る必要がある／**不明**＝設計の前に調べる／**制約**＝既存の作りや約束から来る縛り

| 要件 | 使える資産 | 状態 |
|---|---|---|
| 1.1〜1.3 連結・行なし・重複なし | `ExecutionStatus::from_states`・`render` | 在る |
| 1.4 talking・choosing を変えない | `State::snapshot`・`snapshot_with_choice` | 在る（欄を足すときに既存の 2 本の作り方を崩さないこと） |
| 1.5 ヘッダの位置 | `shiori/real.rs` | 在る |
| 2.1 更新中の online | `UpdateDesk.stage`／`update/worker.rs` | 欠け（kanade へ届ける道） |
| 2.2 取得中の online | `install/fetch_url.rs` | 欠け（始まりと終わりの印・特に失敗時の終わり） |
| 2.3・2.4 両方の重なりと終わり | — | 欠け（「数」か「2 本の真偽」で持つ） |
| 2.x ゴースト切替をまたぐ online | `update::desk::on_steady` | 欠け・制約（kanade はゴーストごとに作り直し） |
| 3.1・3.2 enter／leave | `UserBreakWiring`・`drain_no_user_break_signals` | 欠け（kanade へ送る 1 手） |
| 3.3 トークが終わったら載せない | kanade の `talk_active` | 欠け・制約（UI の旗はトーク終了では下りない） |
| 3.4 断る区間との一致 | `judge_press` | 制約（下の 4.1 節） |
| 3.5・4.8 持ち越さない | `UserBreakWiring`・`Emo2Wiring`・kanade の `State` はゴーストごとに新品 | ほぼ在る（UI 側に「最後に送った値」を持つならゴーストごとの持ち物に置くこと） |
| 4.1・4.2・4.6 番号つきで載せる | `target_visible`・`current_surface_id`・`BalloonBindings` | 欠け（届ける道） |
| 4.3 昇順 | 観測は scope 昇順（`balloon_visibility_phase.rs`） | 欠け（導出側でも保証するか決める） |
| 4.4 空なら載せない | — | 欠け（導出の分岐） |
| 4.5・4.7 消えたら落とす・画面と一致 | 相の終わりで読む | 欠け・不明（見えているのに番号が `None` の場合） |
| 5.1 すべてのリクエスト | `INACTIVE` を使う起動・終了・切替の呼出 | 欠け・要判断（議題 5） |
| 5.2 1 秒以内 | UI の Input の段・フレームの相は毎フレーム走る（tick の門は既定で無効） | ほぼ在る（門を有効にしたときは不明＝R4） |
| 6.1・6.2 5 状態を載せない・書式を保つ | 導出表の注記・`OpeningKinds` | 在る（注記の書き換えのみ） |
| 6.3 次の持ち主の記録 | roadmap・`doc/ukadoc-coverage/briefing-sakura-script.md` の所有行（`\![enter,inductionmode]`・`\![enter,passivemode]`・`\t` を本 spec が所有） | 欠け・要判断（受け皿の spec が見当たらない・議題 9） |
| 7.1 届けの失敗を error で残す | 前例 `balloon_break_send_failed` | 欠け（新しい送り口ごとに） |
| 7.2 送った値をログで追える | `round_trip_request` の `trace!` | 在る |
| 8.1 組み合わせの決定論テスト | `status.rs` の網羅テストの型 | 欠け（入力空間が広がる） |
| 8.2 送り口を通した観測 | 一周の照合・`ScriptedShiori` の記録の第 2 系統 | 欠け・不明（R2） |
| 8.3 既存の期待値の更新 | `expected_statuses()` ほか | 欠け |
| 8.4 emo2 が雑談しないこと | `real_pasta_test.rs`（env ゲート） | 欠け・要判断（議題 10） |

---

## 4. 重点の 3 点

### 4.1 (a) 「トークが走っている間だけ nouserbreak」は、実際に中断を断る区間と合うか

実際に中断を断っているのは UI の `judge_press` の ④（旗が立っている）だけである。旗の上げ下げは次のとおり。

| 場面 | UI の旗 | 実際に中断を断るか | 「旗 かつ 再生中」で載せた場合 |
|---|---|---|---|
| トーク中に enter の後 | 立つ | 断る（バルーンが見えていれば） | 載る ✔ |
| leave の後 | 下りる | 断らない | 載らない ✔ |
| leave を書かずにトークが終わった後 | **立ったまま** | 再生中のトークが無いので止める物が無い（ただしバルーンが残っていれば、左ダブルクリックで「隠す」も起きない） | 載らない ✔（要件 3.3） |
| 中断（利用者の中断・置き換え）で終わった後 | 立ったまま | 同上 | 載らない ✔ |
| **次のトークの立ち上がり直後**（kanade が StartTalk を出してから、そのトークの最初の指示が UI に取り出されるまで） | **前のトークの旗が残る** | バルーンが残っていれば**断る** | **載る**（要件 3.3 とは食い違い、3.4 とは一致） |
| トーク中・バルーンが見えていない | 立つ | 判定③で先に「バルーンが出ていない」に倒れる（中断は起きようがない） | 載る（中断の手段が無いだけなので食い違いではない、と読める） |

結論:
- トークの外は「再生中」の条件を掛ければ要件 3.3 を満たす。kanade の `talk_active`（`Steady{talk: Some}`／`BootVersion{talk: Some}`）がその条件になる。
- **食い違うのは新しいトークの立ち上がり直後の短い区間だけ**。長さは「talk スレッドが最初の指示を配る → UI の次の Input の段で取り出す → kanade へ届く」の分で、通常は数フレーム。ここで 3.3 と 3.4 のどちらに合わせるかは設計で決める必要がある（議題 1）。合図にトークの番号が無いので、kanade 側だけで「どのトークの旗か」を見分ける手は今は無い。
- 中断の無効化の対象は、いまの areka ではバルーンの左ダブルクリックによる中断だけである。マウスのダブルクリック（`OnMouseDoubleClick`）の応答に台本があれば、再生中のトークは無効化モード中でも置き換わる（`steady.rs` の `value_replaces_active_talk` の腕・旗を見ない）。要件 3.4 の「利用者の操作による中断」を balloon-break の定義どおり「バルーンの左ダブルクリックの中断」と読む前提を、設計で明記しておくのがよい（議題 2）。振る舞いそのものを変えるのは本 spec の外。

### 4.2 (b) online の出どころで、触るファイルがどこまで広がるか

| 案 | 触るファイル（kanade 側を除く） | 良い点 | 弱い点 |
|---|---|---|---|
| O1: UI の窓口から押し込む | `update/desk.rs`（`set_stage` で送る）・`install/fetch_url.rs`（始まりと終わりを窓口へ知らせる）・`install/mod.rs` か `install/desk.rs`（知らせの型と受け）・再送の置き場（`update::desk::on_steady` 等） | 既存の形（UI の持ち物が kanade の送出端へ送る）に揃う | 取得スレッドから UI への知らせが新設になる。切替の後の再送が要る。起動途中の呼出には間に合わない |
| O2: プロセスで 1 つの「通信中の数」を共有し、kanade が読む | `install/fetch_url.rs`・`update/worker.rs`（数を増減する。終わりは関数を抜けるときに必ず戻す形で失敗も拾える）・数を kanade へ渡す道（`KanadeConfig` → `crates/areka-ghost/src/config.rs`・`runtime.rs` → ゴーストの起動の入力。`emo2_boot/mod.rs` を経ずに渡せるかは不明） | 切替をまたいでも新しい kanade がその場で正しい値を読める。起動途中の呼出にも載る。重なり（要件 2.4）も数で自然に扱える | kanade の運行表は「状態と入力だけで決まる」作り。共有の数を読むとその性質が崩れる（殻で読んで入力に添える形なら保てる）。areka-ghost まで広がる |
| O3: 背景スレッドから kanade へ直接 | `update/worker.rs`（`job.kanade` へ前後で送る） | 更新のイベント（OnUpdate*）と同じ線で送るので順序が保証される | URL 取得のスレッドは kanade の送出端を持たない（持たせるには `emo2_boot/mod.rs` の結線＝触らない約束に当たる）。読み直しの後の kanade は古い送出端の外 |
| O4: 混成（数を共有し、UI の系が毎フレーム読んで今の kanade へ変化を押し込む） | `install/fetch_url.rs`・`update/worker.rs`（数の増減）・数の置き場（`update/mod.rs` か `install/mod.rs` の下）・押し込む系の登録（`ghost_session.rs` の `register_systems`）・ゴーストごとの「最後に送った値」 | areka-ghost へ広がらない。失敗の終わりも数で拾える | 起動途中の呼出には最初のフレームまで載らない。押し込みの再送の作り（ゴーストが替わったら送り直す）が要る |

補足:
- 「通信中」の範囲は要件で「更新の開始から終了まで」「ダウンロードしている間」と決まっている。更新を `Stage::Running`（Started〜OrderDone）とみなすか、メニューの答え待ち `AwaitingExec` も含むか、背景スレッドの `run_order` の前後とみなすかで、届け元のファイルが変わる（議題 4）。`AwaitingExec` の間はまだ通信していない。
- 更新のイベント（OnUpdateBegin など）は背景スレッドから kanade へ直接送られる。UI 経由で online を届けると、最初の数件のイベントに online が載らない順序が起こりうる（要件 5.2 の 1 秒以内には収まる）。

### 4.3 (c) 同じウェーブ C1 の約束との突き合わせ

| 約束（触らない） | 本 spec が要りそうか | 回避の道 |
|---|---|---|
| `emo2_boot/balloon_visibility.rs` | `BalloonVisibilityState` に「最後に送った値」を足したくなる | 子の `balloon_visibility_phase.rs` の中で完結させる。値の置き場は `Emo2Wiring`（`frame/wiring.rs`・ゴーストごと）に欄を 1 つ足すか、`balloon_visibility_phase.rs` の中の型で持つ |
| `input_events/balloon.rs` | 要らない（押下の入口 `on_left_press` は `user_break.rs` 側） | — |
| `areka-emo-text/src/{actor,layout,viewbox,viewbox_draw}.rs` | 要らない（読むのは表示層 `areka-emo-present` の照会だけ） | — |
| `emo2_boot/mod.rs` | 新しい線を結線で配りたくなる（取得スレッドに kanade の送出端を持たせる・新しいファイルの宣言） | nouserbreak は `UserBreakWiring` の既存の送出端で足りる。バルーンは `GhostSlot` から引ける。新しい線を作らない案（O1・O4）を選ぶ |
| `input_events/mod.rs` | 新しいファイルを足すと宣言が要る | `input_events/user_break.rs` の中に書く（子モジュールが要るなら `#[path]` でこのファイルから宣言する） |
| `frame/drain_resnap.rs` | 要らない | — |
| `main.rs`（⑥⑦との約束） | 新しい最上位モジュールを足すと宣言が要る | 新しいファイルは `update/`・`install/`・`ghost_session.rs`・kanade の `lib.rs` など、触ってよい親の下に置く |

ほかの C1 との重なり:
- `restart-chain-finalize-stall`（`frame/drain_resnap.rs`）・`drag-click-without-move`（wintf・`placement/`）・`choice-timeout-directive`（`areka-sakura/src/compile.rs`）・`release-package-versioned`（`tools/`・`boot_config.rs`）とは重ならない。
- roadmap の C1 の行には `mcp-server-core`（新しい crate と `main.rs`）が⑦として載っている一方、組み直しの記録（12b）では C1 の席を `animated-image-decode` に譲ったとある。どちらでも本 spec の触るファイルとは重ならない（`animated-image-decode` が `areka-emo-present` の表示層を広く触る場合でも、本 spec は既存の照会 2 本を読むだけ）。
- `emo-text-file-split` が `balloon_visibility.rs` を分けるとき、子の `balloon_visibility_phase.rs` 冒頭の `use super::{BalloonVisibilityState, …}` の綴りが変わる可能性がある。本 spec も同じファイルの同じ辺りを書き換えると、文字の上でぶつかりうる（中身の衝突ではなく併合の手間）。
- C2 の `shell-balloon` は本 spec の後と決まっている。C2 の `translate-pipeline` は `schedule/steady.rs` を分ける予定がある（本 spec の後なので問題は無いが、本 spec が steady.rs に足す行は少ないほどよい）。

---

## 5. 実装の選択肢

### 案 A: 既存を広げる（kanade が 3 状態を覚え、UI の各持ち物が変化を押し込む）

- kanade: `status.rs`（スナップショットに欄・導出表の 3 行）・`msg.rs`（`KanadeMsg` に「状態の知らせ」を 1 つ）・`actor.rs`（振り分け）・`schedule/mod.rs`（`State` に 3 状態・`snapshot` が添える）・起動／終了／切替の `INACTIVE` の扱い（議題 5 しだいで `boot.rs`・`change.rs`・`steady.rs`）。
- nouserbreak: `input_events/user_break.rs` の `drain_no_user_break_signals` で旗が変わったら `wiring.kanade` へ送る。kanade は「旗 かつ 再生中」で載せる。
- balloon: `emo2_boot/balloon_visibility_phase.rs` の相の終わりで、装着済み scope の（見えているか・面番号）を集め、前に送った値と違えば `GhostSlot` の kanade へ送る。
- online: 案 O1（窓口から押し込む）。
- ✅ 既存の形に揃い、新しい部品が少ない。境界の約束を守りやすい。
- ❌ online の「切替後の再送」と「取得の失敗時の終わり」の手当てが散る。起動途中のイベントに online が載らない区間が残る。

### 案 B: 新しい部品（3 状態を 1 つの共有の器に集め、kanade が送るときに読む）

- ゴーストごとの器（nouserbreak・balloon）とプロセスの器（online）を作り、出どころが書き、kanade が読む。
- ✅ 届けの遅れが無い。切替をまたぐ online も自然。
- ❌ 器をゴーストの起動で kanade と UI の両方へ配る必要があり、結線（`emo2_boot/mod.rs`）か areka-ghost の起動の入力まで広がる。kanade の運行表が外の状態を読むことになり、決定論テストの作りが変わる。約束の境界に当たる見込みが高い。

### 案 C: 混成（nouserbreak と balloon は案 A、online だけ共有の数）

- nouserbreak・balloon はゴーストごとの持ち物から押し込む（案 A と同じ）。
- online はプロセスで 1 つの「通信中の数」を `update/worker.rs` と `install/fetch_url.rs` で増減し（関数を抜けるときに必ず戻す形で、成功・失敗・中止を区別せず拾う）、それを kanade へ届けるのは O2（殻で読んで入力に添える）か O4（UI の系が読んで押し込む）のどちらか。
- ✅ 要件 2.3・2.4（重なり・どんな終わり方でも戻る）を 1 つの仕組みで満たせる。online のための UI 側の新しい知らせの線が要らない。
- ❌ O2 を選ぶと areka-ghost の起動の入力まで広がる。O4 を選ぶと起動途中の呼出には載らない区間が残る。

---

## 6. 規模とリスク

- **規模: M（3〜7 日・6〜8 タスク）**。kanade の材料と導出（1〜2）・nouserbreak の届け（1）・balloon の届け（1）・online の印と届け（1〜2）・起動／終了／切替の扱いと既存テストの期待値の更新（1）・送り口を通した観測と emo2 の確かめ（1）・持ち主の記録（ドキュメントのみ）。
- **リスク: 中**。新しい技術は無いが、①スレッドをまたぐ届けの順序（nouserbreak の立ち上がり直後・更新のイベントと online の順）②一周の照合の期待値がフレームと kanade の処理順に依存しないか ③1,000 行の目安に近いファイル（`schedule/steady.rs` 935 行・`msg.rs` 894 行・`schedule/mod.rs` 859 行）への追記、の 3 つに注意が要る。

---

## 7. 設計への申し送り

### 7.1 決めること（要件の議論で扱う候補）

1. **nouserbreak の立ち上がり直後の食い違い**: 新しいトークが始まってから、その最初の指示が UI に届くまでの短い間、前のトークの旗が残る。この間を「載せない」（要件 3.3 優先・kanade がトークの始まりで自分の写しを下ろす）か「載せる」（要件 3.4 優先・UI の旗に従う）か、または合図に区別（トークの番号など）を足して根から消すか。
2. **「中断」の範囲の明記**: 要件 3.4 の照合相手を「バルーンの左ダブルクリックによる中断」（balloon-break の定義）に限ると書くか。マウスのダブルクリックの応答による置き換えは無効化モード中でも起きる（変えるのは本 spec の外）。
3. **online の届け方**: O1（UI の窓口から押し込む）／O2（共有の数を kanade が読む）／O3（背景スレッドから直接）／O4（共有の数を UI が読んで押し込む）。ゴーストの切替（更新の読み直しを含む）をまたいで続く online を、新しい kanade へどう渡すか。
4. **online の区間の端**: 更新は `Stage::Running`（Started〜OrderDone）か、背景の `run_order` の前後か、メニューの答え待ち（`AwaitingExec`）も含めるか。URL 取得は失敗時に何も届かないので、終わりの印の新設が必須。
5. **起動・終了・切替のイベントの扱い**（要件 5.1）: いま `INACTIVE` を渡している OnInitialize・username・OnFirstBoot・OnBoot・OnGhostChanged・更新の起動の知らせ・OnGhostChanging・OnClose にも、そのとき見えているバルーン・走っている通信を載せるか。載せるなら `INACTIVE` を「会話も選択もない、ほかは今の値」の形に改めるか、別の作り方を足すか。
6. **スナップショットの形**: `Copy` を外して `BalloonBindings` を持たせるか、別の持ち方にするか。既存の 47 か所の構造体リテラル（大半はテスト）を `..ExecutionSnapshot::INACTIVE` で吸収するか、作り方の関数を足すか。昇順と「空なら載せない」をどこで保証するか。
7. **balloon の「最後に送った値」の置き場**: ゴーストごとに作り直される持ち物に置く必要がある（要件 4.8）。`Emo2Wiring`（`frame/wiring.rs`）に欄を足すか、`balloon_visibility_phase.rs` の中の型で持つか。`BalloonVisibilityState`（`balloon_visibility.rs`）には足せない。
8. **見えているのに面番号が取れない場合**（`target_visible == Some(true)` かつ `current_surface_id == None`）: 載せずに記録するか、0 とみなすか。作りの上では起きにくいが、要件 4.7 の「食い違わせない」の扱いを決めておく。
9. **5 状態と関連タグの次の持ち主**（要件 6.3）: いま本 spec が `\![enter,inductionmode]`・`\![enter,passivemode]`・`\t` を所有している（`doc/ukadoc-coverage/briefing-sakura-script.md`）。受け皿になりそうな既存 spec は、`\t` なら `sakura-time-directives`、入力ボックスなら `balloon-canon-residue`（項目 5 の入力窓系列）くらいで、最小化・induction・passive には見当たらない。完了した spec は宿題を消化できないので、新しい追跡 spec を `/kiro-discovery` で起こすかを決める必要がある。ukadoc-survey の検査が brief の所有行を読むので、持ち主を移すと他の spec の brief と網羅の表の書き換えが伴う。
10. **要件 8.4 の確かめ方**: 実 pasta を使う試験は `HOST32_PASTA_DLL` があるときだけ走る追験しか無い。追験に「online を載せた OnSecondChange で雑談が始まらない」を足すか、実機の観測で済ませるか。pasta の `kick_force` が立つと 1 回だけ抑止を突破する点に注意。

### 7.2 調べること（設計で確かめる）

- **R1**: ゴーストの切替で、新しい `Emo2Wiring`・`UserBreakWiring` が据わってから `GhostSlot` が新しいゴーストへ替わるまでの間にフレームが走るか（走るなら、新しいゴーストの値を古い kanade へ送る・その逆が起きうる）。`ghost_session.rs` の起動の腕の順序を確認する。
- **R2**: 一周の照合（`spine.rs`・`spine_conformance_*`）の土台が `GhostSlot`・`UserBreakWiring` を据えているか、表示層の照会が `Some` を返すか。返すなら、バルーンの値が載る呼出が「UI のフレームが先か kanade の処理が先か」に依存しないで決まるか（決まらないなら、期待値の比べ方を変える必要がある）。負荷の高い並列実行で揺れないかも見る。
- **R3**: `schedule/steady.rs`（935 行）・`msg.rs`（894 行）・`schedule/mod.rs`（859 行）に追記したとき 1,000 行の見張り（`file_length_guard_test.rs`）を越えないか。越えるなら同じ spec の中でファサードの形で分ける必要がある。
- **R4**: tick の門（`AREKA_TICK_GATE`）を有効にしたとき、入力の段とバルーンの相が毎フレーム走らなくなり、要件 5.2（1 秒以内）を外れないか。既定は無効なので、いまの本番には影響しない。
- 調べなくてよいもの: 実 SSP が `balloon(0=2,1=0)` を送る件は要件で `/` に決着済み。pasta の部分一致が `balloon(…)` に反応しないことは確認済み（2.8 節）。

### 7.3 推し（参考・決定ではない）

触るファイルの境界と既存の形を優先するなら、案 C（nouserbreak・balloon はゴーストごとの持ち物から押し込み、online はプロセスで 1 つの数を必ず戻す形で増減）が筋がよい。online の数を kanade へ渡す道（O2 か O4）と、起動・終了・切替のイベントの扱い（議題 5）は互いに効くので、まとめて決めるのがよい。

---

## 8. 要件の議論での扱い（2026-10-03）

| 7.1 の項目 | 扱い |
|---|---|
| 1. nouserbreak の立ち上がり直後の食い違い | **要件で決着**: 要件 3.4 に「立ち上がり直後も例外としない」を足し、Boundary Context で「中断を断る印の下ろし方を直す」ことを範囲に入れた（1 フレーム遅らせる・食い違いを許す解は取らない）。**直し方**（トークの終わりで印を下ろす／合図にトークの番号を足す等）は設計で決める |
| 2. 「中断」の範囲 | **要件で決着**: 要件 3.4 を「バルーンの左ダブルクリックによる中断（`areka-P0-balloon-break` の定義）」に限定。マウスのダブルクリックの応答による置き換えは Boundary Context の範囲外へ |
| 3. online の届け方 | 設計で決める。ただし要件 2.6（切替・再起動・更新の読み直しをまたいで持ち越す）と要件 5.1（起動途中のイベントにも載せる）を満たすこと |
| 4. online の区間の端 | 更新は要件 2.1 で「実際に始めてから終えるまで・承諾待ち（`AwaitingExec`）は含まない」と決着。`Stage::Running` か `run_order` の前後かと、URL 取得の終わりの印の作りは設計で決める |
| 5. 起動・終了・切替のイベント | **要件で決着**: 要件 5.1 に起動・終了・切替のイベントも含むと明記。`INACTIVE` の改め方は設計で決める |
| 6・7・8・10 | 設計で決める |
| 9. 5 状態と関連タグの次の持ち主 | **議題 1 で決着（開発者裁定）**: `\t`・timecritical → 実在の `sakura-time-directives`、induction・passive → 網羅の計画の候補 `passive-mode-states`、最小化 → 候補 `minimize-state`、opening → 候補 `inputbox-dialog`・`communicate-events`。候補は計画の波で起票（本 spec の完了時には起こさない）。完了時に網羅の整合検査が候補名で赤くならないことを確かめる（要件 6.3） |

---

## 9. 設計の調査と決定（2026-10-03・design.md の裏付け）

### 9.1 要約

- **Feature**: `areka-P0-status-execution-states`
- **Discovery Scope**: Extension（既存の仕組みへの統合・外部の依存なし・調べものはコードの読み合わせだけ）
- **Key Findings**:
  - `GhostSlot` の入れ替えと新しい `Emo2Wiring`・`UserBreakWiring` の据え付けは `ghost_switch.rs` の `boot_into` が同じ関数で同期に行う（`boot_ghost_strict` → `commit_ghost_windows` → `insert_non_send(GhostSlot)`）。間にフレームは走らない。
  - 一周の照合（`spine.rs`・`spine_conformance_*`）は `GhostSlot`・`UserBreakWiring`・`GhostSession` を 1 つも据えていない（`emo2_boot/spine*` を grep して 0 件）。
  - ukadoc 網羅の整合検査（`crates/ukadoc-survey/tests/consistency/spec_checks.rs` の腕 b・c・f）は、台帳の `owner` が `roadmap-draft.md` の `[[spec]]` か `briefing.md` の `[[owner_completed]]` の名前であること、`[[spec]]` の名前のフォルダが実在すること、`owner_count` が台帳の件数と一致することを見る。候補名を `owner` に書くと赤になる。
  - トークごとの受け口の複製は、トークの再生（`areka-sakura/src/drive.rs` の `TalkDriver` → `CuePlayer`）が所有し、終わり（自然終端・`on_close`）で落ちる。`dispatcher.rs` の `Start` は「既存のトークを Close→join してから複製・起動」なので、前のトークの複製の `Drop` は次のトークの最初の `emit` より前に起きる。

### 9.2 調査の記録（7.2 の R1〜R4）

#### R1: 切替の間のフレーム
- **Context**: 新しい `Emo2Wiring` が据わってから `GhostSlot` が替わるまでにフレームが走ると、新しいゴーストの値を古い kanade へ送りうる。
- **Sources Consulted**: `crates/areka/src/emo2_boot/ghost_switch.rs` の `fn take_down`・`fn boot_into`、`crates/areka/src/ghost_session.rs` の `fn boot_ghost_strict`・`fn boot_wired`。
- **Findings**: `boot_into` は同期の関数で、`boot_ghost_strict`（結線の据え付け）→ `commit_ghost_windows` → `world.insert_non_send(GhostSlot(Some(session)))` を続けて行う。`take_down`（置き場を空にして降ろす）から `boot_into` までの間には窓を閉じる待ちがありフレームが走るが、その間は置き場が空で、古い `Emo2Wiring` の相は送る相手を持たない。
- **Implications**: バルーンの届けは `GhostSlot` から kanade を引けばよい。置き場が空のときは送らず「最後に送った値」も更新しない（据わった最初のフレームで送る）。

#### R2: 一周の照合の土台
- **Context**: 一周の照合の期待値（`expected_statuses`）がバルーンの状態で変わるか。
- **Sources Consulted**: `crates/areka/src/emo2_boot/spine.rs`・`spine_conformance_script.rs`・`spine_conformance_support.rs`（grep: `GhostSlot`・`UserBreakWiring`・`GhostSession`）。
- **Findings**: 0 件。一周の照合は表示層を本物で動かすが、置き場も中断の持ち物も据えないので、本 spec の届けは走らない。
- **Implications**: 期待値は変えない（要件 8.3 の対象は構造体リテラルの追随だけ）。送り口を通した観測（要件 8.2）は kanade のハーネス（`tests/kanade`）で行う。一周の照合に置き場を据えて `balloon(0=0)` を観測するのは、フレームと kanade の処理順に期待値が依存するため採らない。

#### R3: 1,000 行の見張り
- **Sources Consulted**: `crates/log-capture-kit/tests/file_length_guard_test.rs`（`LINE_LIMIT`＝1000・例外表は現に超過している 11 件だけ）。
- **Findings**: 触るファイルの今の行数は `steady.rs` 935・`msg.rs` 894・`schedule/mod.rs` 859・`actor.rs` 707・`desk.rs` 684・`balloon_visibility_phase.rs` 605。
- **Implications**: `steady.rs` は `INACTIVE` → `state.snapshot_without_talk()` の置換だけ（追記 0 行）。`msg.rs` は変種 1 つとラベルの腕（+25 行程度）、`schedule/mod.rs` は入力の変種・写しの欄・横断の腕・`snapshot_without_talk`（+40 行程度）で上限に届かない。テストは兄弟ファイルへ置く。

#### R4: tick の門
- **Sources Consulted**: `crates/areka/src/tick_gate_config.rs`（`AREKA_TICK_GATE`・既定は無効）。
- **Findings**: 門が有効だと「見た目が変わらない画面更新」で入力の段とバルーンの相が走らない巡がある。旗の合図やバルーンの差分は tick を起こさない。
- **Implications**: 既定の本番には影響しない。設計の Revalidation Triggers に「門が既定で有効になったら要件 5.2 を見直す」と記す。本 spec では門の起こしを足さない。

### 9.3 形の比較

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| 案 A: 全部押し込み | 3 状態とも UI の持ち物が `KanadeMsg` で kanade へ | 既存の形・新しい部品が少ない | online を切替（読み直し）の後の新しい kanade へ再送する手当てが散り、起動途中のイベントには最初のフレームまで載らない（要件 2.6・5.1 を満たしにくい） | 不採用 |
| 案 B: 共有の器を kanade が読む | 3 状態を器に集め、kanade が送るときに読む | 遅れがない | 器を起動の結線（`emo2_boot/mod.rs`・areka-ghost の起動入力）で配る必要があり C1 の約束に当たる。運行表が外の状態を読む | 不採用 |
| **案 C: 混成** | nouserbreak・balloon は押し込み、online はプロセスの数を殻が読む | online は切替をまたぎ起動の最初のイベントから載る。失敗の終わりも RAII で拾う。運行表は純粋のまま | グローバルの数を持つ。テストの決定論のため持ち手は数を引数で受ける | **採用** |

online を届ける道の比較（研究 4.2 の O1〜O4）: O1（窓口から押し込む）は取得の失敗時の終わりと切替後の再送が新設になる／O2（areka-ghost の起動入力で配る）は `emo2_boot/mod.rs` を通る／O3（背景スレッドから直接）は取得スレッドが kanade の送出端を持たず読み直し後の kanade は古い端の外／O4（UI が毎フレーム読んで押し込む）は起動途中に載らない。**殻が原子的な数を読む**（O2 の「殻で読んで入力に添える」の形）が、要件 2.6・5.1 と C1 の約束を同時に満たす唯一の道だった。

### 9.4 設計の決定（7.1 のうち設計へ送られた項目）

#### 決定 1（7.1 の 1・3.4 の直し方）: 旗はトークの終わりで下ろす
- **Alternatives**: ⑴ kanade がトークの始まりで写しを下ろす（UI が旗を立てている間に「載っていないのに中断できない」が起きる）⑵ 合図にトークの番号を足す（`TalkCue` に番号が無く、dola まで広がる）⑶ 受け口の複製の `Drop` で `TalkEnded` を送る。
- **Selected**: ⑶。複製はトークの再生が所有し終わりで落ちる（`drive.rs`）。`fold_no_user_break` に `TalkEnded => (false, false)` を足す。kanade の写しは UI の旗の鏡で、トークの境界で勝手に下ろさない。`nouserbreak` は `talk_active && 写し` で載せる。
- **Rationale**: 前のトークの旗が次のトークの立ち上がりまで残る区間（研究 4.1 の表の 5 行目）が構造として消える。1 フレーム遅らせる解ではなく、旗の形を変える解である。
- **Trade-offs**: 残るのは UI → kanade の運搬の間だけで、`talking`（`TalkDone` の運搬）と同じ種類の遅れ。取り出しの終わりに 1 回だけ送るので、同じ巡の `Enter`＋`TalkEnded` は差し引き 0 で送らない。
- **Follow-up**: 複製の `Drop` が `TalkStarted(次)` より前に同じ線へ並ぶことを、受け口の兄弟テストと dispatcher の既存の順序で担保する。

#### 決定 2（7.1 の 3）: online はプロセスの数を殻が読む
- **Selected**: `areka_kanade::online::{OnlineCounter, OnlineGuard, PROCESS}`。殻（`actor.rs`）が受け取ったすべてのメッセージの前に数を読み `State.external.online` へ写す。持ち手は `KanadePorts::standard_started`（更新）と `fetch_and_send` の冒頭（取得）。
- **Rationale**: 上の 9.3。
- **Trade-offs**: グローバル。テストの決定論のため、持ち手は `&'static OnlineCounter` を引数で受け、areka の兄弟テストは関数内の `static` を渡す。`PROCESS` を立てるテストは「載っている」だけを見る。
- **Follow-up**: `KanadePorts` に `Cell<Option<OnlineGuard>>` を持たせても `run_order(&dyn UpdatePorts)` の境界が `Sync` を求めないことを実装時に確かめる（求めるなら `Mutex<Option<_>>` へ）。

#### 決定 3（7.1 の 4）: online の区間の端
- **Selected**: 更新は `standard_started`（`OnUpdateProcessExec` の段を抜けた直後・台本の依頼も同じ所を通る）から `run_order` を抜けるまで（閉包が `drop(ports)` してから `OrderDone`）。取得は `fetch_and_send` の冒頭から関数を抜けるまで（取得口を作れない・落とせない・送れない、のどの経路でも戻る）。
- **Rationale**: 要件 2.1（承諾待ちは含まない）と 2.3（どんな終わり方でも戻る）を 1 つの RAII で満たす。

#### 決定 4（7.1 の 5）: `INACTIVE` の改め方
- **Selected**: 本番の直渡しを 0 か所にし、`State::snapshot`（会話・選択あり）と `State::snapshot_without_talk`（会話も選択も無いと決めて送る場面）の 2 系統へ一本化する。`snapshot_of(&Phase)` は `talk_active_of(&Phase) -> bool` へ縮める。`INACTIVE` は const のまま残す（テストの既定値・`..ExecutionSnapshot::INACTIVE`）。
- **Rationale**: 要件 5.1（起動・終了・切替のイベントにも同じ規則）。

#### 決定 5（7.1 の 6）: スナップショットの形
- **Selected**: `Copy` を外し `Clone` を残す。`balloons: Vec<BalloonBinding>`。47 か所の構造体リテラルは `..ExecutionSnapshot::INACTIVE` で吸収し、`inactive_snapshot_has_every_source_false` だけ網羅のリテラルを書き直す。昇順と重複なしは `BalloonBindings::new` が構成時に保証し、空なら載せないは導出表の 9 行目が持つ。
- **Rationale**: 保証を 1 か所に置く（`from_states` と同じ流儀）。

#### 決定 6（7.1 の 7）: 「最後に送った値」の置き場
- **Selected**: `Emo2Wiring.balloon_status: BalloonStatusLedger`（`frame/wiring.rs`・ゴーストごとに新品）。型と配線は新しい `frame/status_report.rs`（`frame.rs` から `mod status_report;`）。`balloon_visibility.rs` には触らない。
- **Rationale**: 相の判断を 1 つも増やさず、差分と送出を切り離す。

#### 決定 7（7.1 の 8）: 見えているのに番号が取れない scope
- **Selected**: `balloon_id = 0` で載せ、scope ごとに 1 回 `warn!(balloon_status_surface_unknown)`。
- **Rationale**: 要件 4.7（見えているバルーンを落とさない）を優先し、記録の無い縮退を作らない。

#### 決定 8（7.1 の 10）: 要件 8.4 の確かめ方
- **Selected**: `real_pasta_test.rs` に env ゲート（`HOST32_PASTA_DLL`）の 1 件を足す。`PROCESS` の guard を持ったまま起動の挨拶の完了を待ち、OnSecondChange を雑談の間隔を超える回数回して `Value` が 0 件であることを見る。`kick_force` が立たない場面（挨拶の後・他の入力なし）で見る。env が無ければ実機の観測（`RUST_LOG=kanade=trace` の `shiori_request`）で代える。

#### 決定 9（7.1 の 9・記録の置き方）
- **Selected**: `\t` の宛先は実在の `areka-P0-sakura-time-directives` へ移す（`owner_count` も合わせる）。候補名はフォルダが無く `owner` に書けないので、`\![enter,inductionmode]`・`\![enter,passivemode]` と `Status [SSP拡張]` の `note`、`roadmap.md` の本 spec の行に書く。`report/` は作り直す。候補の起票はしない。

### 9.5 統合（synthesis）の結果

- **一般化**: nouserbreak と balloon の知らせは 1 つの `KanadeMsg::ExecutionState(ExecutionStateUpdate)` にまとめる（状態ごとに変種を増やさない。online は殻が読むので変種を持たない）。
- **作る／採る**: 新しい依存は無い。原子的な数は `std::sync::atomic`、届けは既存の `mpsc`。
- **簡素化**: kanade 側に「どのトークの旗か」を見分ける仕組みは作らない（写しは鏡・gating は `talk_active`）。バルーンの帳簿は「最後に送った値」だけで、表示の真実源は表示層のまま。

### 9.6 リスクと手当て

- 旗の運搬の遅れが「載っているのに中断できる」として実機で見えるか — 遅れは入力の段 1 巡＋受信箱で、`talking` と同じ種類。実機サインオフで `no_user_break_changed`・`shiori_request` の時刻を並べて確かめる。
- `PROCESS` を立てる本番経路のテストが並列の他のテストを揺らす — 持ち手は数を引数で受け、兄弟テストは自分の `static` を渡す。`PROCESS` を使うテストは「載っている」だけを見る。
- `ExecutionSnapshot` の `Copy` を外して広く赤になる — 47 か所は `..INACTIVE` で機械的に直る。
- ukadoc 網羅の検査（`owner_count`・`report/` の一致）が記録の書き方で赤になる — 設計の「持ち主の記録」の手順どおりに `cargo test -p ukadoc-survey` で確かめる。

### 9.7 参照

- ukadoc `Status [SSP拡張]`: `ukadoc:spec_shiori3:Status_20_5bSSP_62e1_5f35_5d:1`
- ukadoc `\![enter,nouserbreakmode]`: `ukadoc:list_sakura_script:_5c_21_5benter_2cnouserbreakmode_5d:1`
- `crates/areka-kanade/src/status.rs`（語彙・書式・導出表）／`schedule/mod.rs`（`State::snapshot`）／`actor.rs`（殻）
- `crates/areka/src/input_events/user_break.rs`・`emo2_boot/user_break_cue.rs`（旗）／`emo2_boot/balloon_visibility_phase.rs`（相）／`update/worker.rs`・`install/fetch_url.rs`（通信）
- `crates/ukadoc-survey/tests/consistency/spec_checks.rs`（記録の検査の規則）
