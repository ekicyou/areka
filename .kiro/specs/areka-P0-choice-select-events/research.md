# ギャップ分析 — areka-P0-choice-select-events

> 実施: 2026-07-31（`/kiro-validate-gap`）／対象: 確定済み `requirements.md`（Req1〜9）＋ `brief.md` ＋ `.kiro/steering/`
> 方針: **情報提供であって決定ではない**。複数案とトレードオフを提示し、裁定は要件ディスカッション／design に委ねる。
> 実測は本ワークツリー（branch `claude/areka-p0-choice-select-events-914e0c`・main 相当）の HEAD に対して行った。行番号は実測値。

---

## 1. 分析サマリ

- **上流は完成・下流は空**。選択確定の発生源（`ChoiceSelection` 発行＋`ChoiceSelectionInbox`）、talk 側の barrier 解決口（`SakuraMsg::ResolveChoice`→`CuePlayer::resolve_choice`）、`Status` 語彙（`ExecutionState::Choosing`）はいずれも**実物**として着地済み。欠けているのは**それらを結ぶ駆動側 1 本**であり、本 spec の編集面は「新規ロジック」より「**既存の型の壁を additive に開ける**」作業が支配的。
- **最大の構造壁は 3 本の型シグネチャ**: ①`ShioriCall::{Get,Notify}.id: &'static str`（任意名イベント不可）②kanade→talk の唯一の outbound が `Sender<StartTalk>`（解決を運べない）③`DispatcherMsg` に `ResolveChoice` アーム無し（active talk inbox へ到達不能）。いずれも additive 拡張で開くが、`origin: &'static str`（`schedule/mod.rs:55-58`・`steady.rs:191` の origin リテラル match）に波及するため**波及範囲の裁定が design 冒頭の主題**になる。
- **「選択待ちである」という事実が kanade へ届く経路が存在しない**。Req6（`choosing` 導出・自発トーク抑止）と Req7（タイムアウト起点＝トーク表示完了）と Req1.4（候補集合との照合）は、いずれも kanade が知り得ない情報を要求している。到達経路の新設（talk アクター→dispatcher→kanade の通知 1 本 or UI 層からの通知）が**本 spec 最大の新規設計**。
- **タイムアウトは型の入口だけが在り、値が流れない**。`BarrierKind::WaitForChoice { timeout: Option<f64> }` は実在するが、compile は `timeout: None`（M1 無期限）を**ハードコード**（`compile.rs:207-222`）。`None` の意味が「未指定（＝既定値へ）」と「無効化（`\*`/0/-1）」で衝突するため、Req7.6/7.7/7.8 の既定値の住処と `None` 意味論の裁定が必須。なお `\![set,choicetimeout]`／`\*` の**解釈**は追跡 spec `areka-P0-sakura-time-directives`（M1 外）が正本で、本 spec の Out of scope と整合。
- **正典は字面が揺れており、要件の採用値は「揺れの一方」**。ukadoc `\q[タイトル,ID,r2,r3...]` は「Ex に**続いて** OnChoiceSelect も発生する（無意味だが）」と書き、`\*` の記述例は「Ex（**トークがなければ** 無印）」と書く。ローカル正本 `doc/shiori/fragments/events/14.choice.toml:78` の `OnAnchorSelectEx` は「**SHIORI が何も返さなかった場合のみ**続けて OnAnchorSelect」と 204 ゲートを明記。Req2.3/2.4 は 204 ゲート説を採っており、この根拠と反証の両方を Req2.8 の対応表へ記録する必要がある（証跡は §5 に収集済み）。

---

## 2. 現状調査（実測アンカー）

### 2.1 上流＝選択確定通知の発生源（`areka-P0-choice-interact` 完了成果・**実物**）

| 資産 | 位置 | 実測内容 |
|---|---|---|
| `ChoiceSelection` | `crates/areka/src/input_events/balloon.rs:43`（フィールド :45-51） | `pub(crate) struct { id: String, label: String, scope: usize, references: Vec<String> }`。`ordinal` 非漏洩。要件の「選択肢 ID・表示ラベル・発生元 scope・付随参照列」と 1:1。 |
| `BalloonWiring::send_selection` | 同 `:95-106` | mpsc `Sender<ChoiceSelection>` へ高々 1 send。失敗は `warn!(event="choice_selection_send_failed")`＋`false`。 |
| `ChoiceSelectionInbox` | 同 `:130` | `pub(crate) struct ChoiceSelectionInbox(pub(crate) Receiver<ChoiceSelection>)`。**受信処理は皆無**（doc `:123-129` が「W6 が受信処理へ置換する seam」と明記。番号 W6 は旧ウェーブ番号で、現行の宛先は本 spec）。 |
| 押下ハンドラ | 同 `:452-580` | 現行 rows のみ読む純関数 `click_selection`（`:242`）でヒット確定→`send_selection`→`info!(event="choice_selected", scope, id, label, references_len)`。**上流側で既に「表示中でない／非ヒットなら発行しない」を保証**（Req 前提「Adjacent expectations」に一致）。 |
| 結線点 | `wire_balloon_choice`（同 `:782-787`）／`main.rs:363` | mpsc を作り `BalloonWiring` と `ChoiceSelectionInbox` を **NonSend** 挿入（UI スレッド所有）。`main.rs:687-688` でバルーン窓へハンドラ装着。 |
| 表示層の照会口 | `TextLayerRuntime::choice_active(&ActorKey)` / `choice_hit_rows(&ActorKey)`（`balloon.rs:359-360,516-517` で消費） | 「選択肢が表示中か」を UI スレッドから同期照会できる（`Emo2Wiring::runtime()` 経由の `Rc<RefCell<..>>`）。 |

**含意**: 本 spec の areka bin 側編集面は `input_events/balloon.rs`（drain 追加）か新規 drain モジュール。W5 同居 3 本（`dpi-window-vanish` / `collision-dpi-hittest` / `kero-balloon`）とはファイル集合が互いに素。ただし **W6.5 `test-cage-determinism` が同ファイルの毒化呼出と衝突**するため、先着の本 spec が正・後着 cage が rebase（memory 記載どおり）。

### 2.2 kanade＝SHIORI 発火・運行表（`completed/areka-P0-kanade` ＋ `completed/areka-P0-input-events`）

| 資産 | 位置 | 実測内容 |
|---|---|---|
| 入力境界 | `msg.rs:84-102`（`KanadeMsg`） | `Boot/Tick/TalkDone/CloseRequest/ForceQuit/ShioriDown/Mouse/Close`。**選択系 variant 無し**（真）。`Mouse(MouseInput)` が additive 増分の前例（`:99`）。 |
| 純粋状態機械の入力 | `schedule/mod.rs:40-59`（`Input`） | `Mouse(MouseInput)` `:49`／`ShioriReply { outcome, origin: &'static str }` `:55-58`。 |
| Phase | `schedule/mod.rs:63-80` | `Steady { talk: Option<ActiveTalk> }` `:75`。`ActiveTalk { talk_id, origin: &'static str }` `:83-86`（**script は保持しない**）。 |
| State 帳簿 | `schedule/mod.rs:89-97` | `phase / last_now / next_talk_id / pending_close`。Phase 外の帳簿はここに置く規律（`pending_close` が前例）。 |
| Action | `schedule/mod.rs:124-139` | `ShioriRequest / ShioriUnload / StartTalk / ResourceOutcome / StopSelf`。**talk への解決指示を運ぶ variant 無し**。 |
| Reference 表 | `schedule/events.rs`（全体） | 純粋関数群。`on_mouse_move` `:205`／`on_mouse_double_click` `:239` が「座標＋不透明 region 転写」の前例。 |
| 許可 ID 表 | `events.rs:59-68` | `ALLOWED_EVENT_IDS: &[&str]`（8 件・固定表）。**`:54-58` の SEAM コメントが本 spec を名指し**し「本表への ID 追加ではなく**受理規則へのカテゴリ追加（additive）**で行う」「`OnTalk`/`OnHour` 恒久禁止は不変」「任意名カテゴリ × Req3.2 恒久禁止の交差は choice-select-events の要件フェーズで決着」と申し送り済み。 |
| egress チョークポイント | `actor.rs:180-206`（`round_trip_request`） | SHIORI へ出る唯一点。`is_allowed_event_id(id) ‖ is_allowed_resource_id(id)` の OR で判定。違反は送出せず `Failed(Internal(..))`。 |
| Steady 応答政策 | `steady.rs:159-217` | `Steady{None}`+`Value`→`TalkId` 採番＋`Action::StartTalk`（`:166-176`）。`Steady{Some}`+`Value` は **origin リテラル match**（`:190-212`）——`"OnMouseMove" \| "OnMouseDoubleClick"` は slot 置換、それ以外は `warn!(event="steady_value_during_talk")` で**破棄**（`:208-211`）。設計コメントが「wildcard にしない＝第 3 の origin 追加時にレビューで政策判断を要求する」と明記。 |
| Status 語彙 | `status.rs:26`（`Choosing`）／`:60`（`"choosing"`） | 語彙は実在。`derive`（`:159-181`）の導出表 `:170` が「1. choosing ← SEAM(Req2.5): 源 **areka-P0-choice-select-events**。M1 非アクティブ確定」と本 spec を名指し。`ExecutionSnapshot`（`:203-220`）は `talk_active: bool` の 1 本のみで、`:209-219` の NOTE が「源が Phase の外にある状態はシグネチャ拡張がシームに含まれる」と明記。 |
| 送出契約 | `status.rs:185-196`（`render`） | 正典順ソート＋カンマ連結・空集合→`None`（ヘッダ行省略）。`talking,choosing,...` の順序は `canonical_index`（`:41-54`）が保証済み＝**Req6.3 は既存資産で充足**（新書式定義不要）。 |
| アクターシェル | `actor.rs:57-62`（`spawn_kanade`） | `shiori: Sender<ShioriMsg>` と **`sakura: Sender<StartTalk>`**。`Action::StartTalk` はこの channel へ `StartTalk` を素で送る（`:114-123`）。 |
| origin 転記 | `actor.rs:126-129` | `origin` は送出 call の `id`（`&'static str`）をそのまま転記。 |
| 決定論檻 | `tests/kanade/common/mod.rs`（`Fixture` `:173`／`MockShiori` `:322`／`MockSakura` `:685`／`Harness` `:1044`／`spawn_harness` `:1062`） | **mock SHIORI（イベント ID 別の応答 fixture）＋StartTalk 記録＋注入 Tick＋有界 join** が完備。`mouse_test.rs` が「入力注入→GET 期待列 assert→StartTalk 観測」の直近前例。**Req9.1/9.2 の器はほぼ既存**。 |

### 2.3 talk 再生側（dola / sakura / ghost dispatcher）

| 資産 | 位置 | 実測内容 |
|---|---|---|
| バリア状態 | `dola/src/cue/runtime.rs:69-78` | `CuePlayerState::WaitingForChoice` `:75`。到達は `tick` 内 `:242-248`。 |
| 候補集合 | 同 `:57-62`（`PendingChoice{id,text}`）／getter `:367` | `WaitForChoice` 手前の Choice cue を蓄積した**照合専用バッグ**。`references` は**積まれない**（`tick` `:217-222` が `id`/`text` のみ写す）。 |
| 解決 | 同 `:291-305`（`resolve_choice`） | 非待機なら `None`／id 不一致なら `None`（状態不変）／一致で先積みクリア＋`Playing` 復帰＋`Some(id)`。**Req2.8「選択解決後に選択肢集合を維持するか破棄するか」は現行実装＝破棄**。 |
| 強制解除 | 同 `:311-321`（`skip_barrier`）／`:342-347`（`stop`） | `skip_barrier` は先積みクリア＋バリア解除。**`SakuraMsg` に到達口が無い**（外部から呼べない）。 |
| talk 終了時刻 | `dola/src/cue/sheet.rs:85-93`（`absolute_end_time`） | `absolute_start_time + max(start_time + duration)`。**`CuePlayer` は本値を公開していない**（`CueSheet` は `TalkDriver::on_tick` `drive.rs:~270` で消費後 drop）。`CuePlayer::is_completed` `:362` は「entry 枯渇 **かつ** horizon 到達」。 |
| 型付き解決口 | `areka-sakura/src/contract.rs:38` | `SakuraMsg::ResolveChoice { id: String }`（enum は `#[non_exhaustive]` `:23`）。doc が「投函は W5（`areka-P0-choice-select-events`）の領分（本 spec は口の定義と檻のみ）」と明記。 |
| 受領ハンドラ | `areka-sakura/src/drive.rs:167`（dispatch）／`:350-419`（`on_resolve_choice`） | `Driving`→`player.resolve_choice`。`Some` なら**その場で** `settle_after_tick`（追加 Tick 不要で `TalkDone` 到達＝menu ケース・R2.4/9.8）。`None` は `debug!` `:382` で状態不変。`Armed`/`Idle` 誤投函は `warn!` `:401`/`:414`。**Req5.1/5.2/5.5 は本ハンドラで既に充足済み**——本 spec は**投函するだけ**。 |
| dispatcher inbox | `areka-ghost/src/dispatcher.rs:31-40` | `Start / Done / Tick / Close` の 4 アーム。**`ResolveChoice` 無し**＝`crates/areka` からも kanade からも active talk へ到達する経路が存在しない。 |
| active slot | 同 `:64-72`（`ActiveTalk{talk_id, handle: TalkHandle, base_now}`）／`on_start` `:101-129` | Close-then-spawn で単一 slot 差替。`handle.inbox: Sender<SakuraMsg>` を保持（＝**投函先はここに在る**）。 |
| stale 棄却 | 同 `:133-150` | `talk_id` 不一致の `TalkDone` は `info!` して破棄。**Req1.3 の「トーク切替により消滅」に相当する既存規律の前例**。 |
| 結線口 | `areka-ghost/src/runtime.rs:218`（`kanade()`）／`:223`（`dispatcher()`） | `main.rs:345` が `runtime.kanade().clone()` を UI へ渡す前例（`wire_mouse_input`）。`dispatcher()` も同型で公開済み。 |
| Tick 供給 | `areka-ghost/src/ticker.rs:229` | ticker が `KanadeMsg::Tick { now: MonotonicMs }` と dispatcher Tick を供給。dispatcher が talk へ経過秒 f64 へ換算中継（`dispatcher.rs:158-172`）。 |
| barrier 発行 | `areka-sakura/src/compile.rs:207-222` | choice cue が 1 個以上なら **`BarrierKind::WaitForChoice { timeout: None }`（M1 無期限）を末尾に 1 個** append。`timeout` は型としては `Option<f64>`（`dola/src/cue/command.rs:95`）だが**値は常に None**。 |
| Choice cue の形 | `dola/src/cue/command.rs:143-148` | `Choice { id, text, references }`。compile は parser の `Choice{disp, target, references}` を素直に写す（`compile.rs:119-130`）。**`\q[t,ID1,ID2,ID3]`（複数 ID 形）は `target=ID1` ＋ `references=[ID2,ID3]` へ潰れる**＝ワイヤ形からは Ex 形（r2,r3…）と区別不能（§5-c 参照）。 |

### 2.4 互換記録の住処（Req8）

- 訂正対象は実在: **`doc/emo2-conformance-scope.md:24`** = 「`OnChoiceSelectEx` — メニュー選択肢確定（`\q[title,id]` の id を Reference0）」→ 正典は **Ref0=ラベル・Ref1=ID**。`.kiro/specs/areka-P0-emo2-conformance-e2e/brief.md:103` が「§1 の訂正は choice-select-events design が実施」と明記。
- ローカル正典ミラー: `doc/shiori/fragments/events/14.choice.toml`（`areka-P0-shiori-protocol-split` 生成物・`provenance` 付き＝`ukadoc` / `ssp_secondary` を区別可能）。**Req8.2「正典に根拠がある挙動と areka 裁量を区別可能に記録」は本 provenance 語彙と同型で表現できる**（対応表の書式候補）。
- 台帳 spec `areka-P0-status-execution-states` が未着手で存在（`Status` 実行状態の台帳）。`choosing` は本 spec が源（`status.rs:170`）で、台帳とは別担当。

---

## 3. 要件 → 資産マップ（gap タグ）

| 要件 | 既存資産 | ギャップ | タグ |
|---|---|---|---|
| R1.1/1.2 受領・順序・一回性 | `ChoiceSelectionInbox`（mpsc・FIFO） | drain（受信ループ／排他システム）が皆無 | **Missing** |
| R1.3 終了済み選択待ちへの遅延通知を棄却 | dispatcher の stale `talk_id` 棄却が前例 | 「選択待ちが終了済み」を配送側が知る手段が無い（§4-G4） | **Missing** |
| R1.4 候補集合との不一致を棄却 | `CuePlayer::pending_choices`（talk アクター内） | kanade／UI 層に候補集合が届かない。`resolve_choice` の `None` も呼び手へ返らない | **Unknown（裁定要）** |
| R1.5 ラベル・参照列は不透明転写 | `ChoiceSelection` が値ごと搬送 | 無し（充足） | — |
| R1.6 無記録打切り禁止 | `logging.md` 規律＋既存 warn/error 前例 | 新規経路の網羅のみ | Constraint |
| R2.1/2.6 任意名イベント発火 | — | `ShioriCall.id: &'static str` ＋ `ALLOWED_EVENT_IDS` 固定表（§4-G1） | **Missing（型壁）** |
| R2.2/2.3/2.4 カスケード（Ex→無印・204 で次段） | `drive`（`actor.rs:103-`）の execute-batch/reinject-last が多段往復の器 | 段の保持先（State/Phase）が無い（§4-G2） | **Missing** |
| R2.5 純関数判定 | `events.rs` の純粋関数群が前例 | 新規（容易） | Missing（低難度） |
| R2.7 `script:` 前置の縮退 | — | 判定＋warn＋解決だけ実行（実装容易） | Missing（低難度） |
| R2.8 対応表 | `14.choice.toml` の provenance 語彙 | 対応表の住処（doc 新設 or 既存追記）の裁定 | **Unknown（裁定要）** |
| R3.1/3.2/3.3 Reference 割付 | `events.rs` の Reference 表様式 | 構築関数 3 本の新設（容易・ただし id 型に依存） | Missing（低難度） |
| R3.4 `OnChoiceTimeout` Ref0＝スクリプト | — | kanade は talk の script を保持しない（`ActiveTalk` に無し） | **Missing** |
| R3.5 空参照列は位置を作らない | `on_mouse_move` の `None→""` は**逆**の前例（位置保持） | 「空なら Reference を付けない」の実装（容易・前例と非対称なので明記要） | Missing（低難度） |
| R3.6 共通ヘッダ欠落禁止 | `ShioriCall` が `status` を必須フィールド化済み（構築点が忘れられない構造） | 無し（充足） | — |
| R3.7 scope は Reference に載せない | `ChoiceSelection.scope` | 用途限定（解決対象特定）だが、現状 talk は単一 slot ＝ scope の使い道が実質無い | Constraint（要裁定） |
| R4.1/4.2/4.3 既存トーク起動棚への合流 | `steady.rs:159-217`（採番・StartTalk・slot 置換） | origin リテラル match（`:191`）に choice 由来を**明示追加**しなければ `:208-211` の warn 破棄へ落ちる | **Missing（既知の落とし穴）** |
| R4.4 additive・既存観測資産不変 | 既存檻（`steady_test.rs` ほか） | 制約（設計制約として遵守） | Constraint |
| R4.5 送出失敗＝204 相当 | `Failed(Internal/Ipc)` は現状**横断的に Unloading{Fault}**（`mod.rs:317-323`） | **要件と既存挙動が衝突**（選択失敗で終了系列へ倒れる）。choice 由来 Failed の例外扱いが必要（prefetch `:313-315` が前例） | **Constraint（衝突）** |
| R4.6 1 選択＝高々 1 StartTalk | カスケードの段設計に従属 | §4-G2 と一体 | Missing |
| R5.1/5.2/5.5/5.6 バリア解除 | `SakuraMsg::ResolveChoice`＋`on_resolve_choice`（完備） | **投函経路が無い**（`Sender<StartTalk>` 壁＋`DispatcherMsg` 欠落・§4-G3） | **Missing（型壁）** |
| R5.3 204/失敗でも解決は取りやめない | — | カスケードと解決の順序・独立性の裁定（§8-DD4） | Unknown |
| R5.4 解決は高々 1 回 | `on_resolve_choice` が id 不一致で状態不変＝二重解決は構造的に無害 | 配送側の一回性（§4-G2 と共通） | Missing |
| R6.1/6.2 `choosing` 導出 | `ExecutionState::Choosing`＋`derive` 導出表の空行 | **「選択待ち中」という事実が kanade へ届かない**（§4-G4） | **Missing（新規経路）** |
| R6.3 連結順序・省略 | `status.rs:41-54,185-196` | 無し（充足） | — |
| R6.4 選択待ち中も再生中扱い＝NOTIFY・Ref3="0" | `Steady{Some}` は `TalkDone` 到達まで維持＝**選択待ち中は自動的に talk_active=true**（`snapshot_of` `:205-213`）→ `on_second_change` が NOTIFY・Ref3="0"（`events.rs:139-163`） | **既に構造的に充足**（`choosing` 追加のみ） | — |
| R6.5 抑止は areka 側調停のみで成立 | R6.4 の帰結（NOTIFY ＝応答スクリプトを運べない型・`msg.rs:17-19`） | 無し（充足）——消費側 `status=="talking"` 完全一致比較に依存しない | — |
| R7.1/7.2 起点＝トーク表示完了（duration 権威） | `CueSheet::absolute_end_time`（dola 内）／`CuePlayer::is_completed` | kanade は horizon を知らない／`CuePlayer` は horizon を公開しない（§4-G5） | **Missing** |
| R7.3/7.4 `OnChoiceTimeout` 発行・応答再生 | 発行は events.rs 様式で容易 | 起点・計測の住処に従属 | Missing |
| R7.5 204 で選択解除＋トーク終了＋以降棄却 | `skip_barrier` は在るが外部到達口無し／`Close` は `Interrupted` 終端 | 「解除して終了」の正規経路の裁定（`canonical-not-minimal-lifecycle` 規律） | **Unknown（裁定要）** |
| R7.6 0/-1＝無期限 | `BarrierKind::WaitForChoice{timeout: Option<f64>}` | `None` の意味衝突（未指定 vs 無効化）・compile は常に `None`（§4-G5） | **Constraint（衝突）** |
| R7.7 単一の入口値 | 同上 | 入口の物理位置（dola barrier / kanade config / 両方）の裁定 | Unknown |
| R7.8 既定値を単一の値として定める | ukadoc は数値を規定していない（§5-e で実測確認） | 値の裁定＋対応表記録 | **Unknown（裁定要）** |
| R8.1/8.2 互換記録 | `14.choice.toml` provenance | 対応表の住処・書式 | Unknown |
| R8.3 scope doc 訂正 | `doc/emo2-conformance-scope.md:24` | 1 行訂正（容易・下流 e2e brief が本 spec を担当と明記） | Missing（低難度） |
| R9.1/9.2 決定論観測 | kanade 檻一式（mock SHIORI/sakura・注入 Tick・有界 join） | 選択確定の注入口・`choosing` の観測・タイムアウトの注入時刻観測の追加 | Missing（器は既存） |
| R9.3/9.4 実機サインオフ | `areka-real-machine-signoff-bounded-auto-exit` 規律・`AREKA_APP_SMOKE_EXIT_MS`＋`RUST_LOG` grep・`choice_selected` info ログが既に在る | 手順の明文化のみ | Constraint |

---

## 4. 構造的ギャップ 6 本（詳細）

### G1. 任意名イベント — `&'static str` の壁（R2.1/2.6/R3.3）

`ShioriCall::{Get,Notify}` の `id` は `&'static str`（`msg.rs:125-136`）。`\q` の ID は実行時 `String` ゆえ**そのままでは載らない**。連鎖する影響点:

1. `Input::ShioriReply { origin: &'static str }`（`schedule/mod.rs:55-58`）と `actor.rs:126-129`（origin＝call の id 転記）。
2. `steady.rs:190-212` の origin リテラル match（`"OnMouseMove" | "OnMouseDoubleClick"`）。
3. `ALLOWED_EVENT_IDS`（固定表）と `round_trip_request` の egress ガード（`actor.rs:194-206`）。
4. `ActiveTalk.origin: &'static str`（`schedule/mod.rs:83-86`）。

**選択肢**:
- **G1-a: `id` を `Cow<'static, str>` へ**。固定 8 ID は `Borrowed` のまま（既存構築関数・既存檻は無改変に近い）、任意名だけ `Owned`。origin も同様に `Cow` 化するか、**origin を「イベント名」から「出所カテゴリ」へ意味変更**（下記 G1-c）。
- **G1-b: `ShioriCall` に新 variant（`GetDynamic { id: String, .. }`）を additive 追加**。既存 2 variant は無傷だが、`match` 網羅点（`actor.rs:126-129,182-188`・`events.rs` テストヘルパ）が全て増える。
- **G1-c: `origin` を専用 enum（例 `Origin::{Pump, Mouse(..), Choice{stage}, Unload}`）へ**。`steady.rs` の政策 match が文字列比較から型 match へ昇格し、「第 3 の origin 追加時に政策判断を強制する」既存設計意図（`steady.rs:189` コメント）を**型で**保てる。既存 origin リテラル檻の更新が要る。
- 許可規則は SEAM コメント（`events.rs:54-58`）の申し送りどおり **表への ID 追加ではなくカテゴリ追加**——例: `is_allowed_event_id(id) ‖ is_allowed_resource_id(id) ‖ is_allowed_choice_event(id)`。`is_allowed_choice_event` の中身（`On` 始まり ∧ **`OnTalk`/`OnHour` を除く**）が Req3.2 恒久禁止との交差の裁定点（§8-DD2）。

### G2. カスケードの段保持（R2.2/2.3/2.4/4.6/1.1）

`drive`（`actor.rs:103-168`）は「Action バッチ実行 → 最後の SHIORI 応答だけ `ShioriReply` として再投入」を Actions が尽きるまで反復する **execute-batch / reinject-last** ループで、**多段カスケードの器としてはそのまま使える**。足りないのは「今どの段か」を持つ場所:

- 現行 `Phase::Steady{talk}` は段を持てない。`State`（`:89-97`）は Phase 外帳簿の置き場（`pending_close` が前例）。
- **選択肢**: (i) `State` に `pending_cascade: Option<CascadeState>` を足す（`pending_close` と同型・Phase 不変＝既存 Phase 檻に無影響）。(ii) `Phase::Steady` に段フィールドを足す（Phase の網羅 match 全点が波及）。(iii) 段を持たず、**カスケードを 1 step 内で完結**させる（`step` が純関数＝SHIORI 往復できないため**不可**）。
- 一回性（R1.1/4.6/5.4）は「`pending_cascade` が `Some` の間は新規選択確定を棄却」or「drain 側で 1 選択＝1 メッセージ」の二重防御で表現できる（要件の「二重防御」方針と整合）。

### G3. kanade → talk の解決経路（R5.1/5.6）

現状の到達不能を 2 段で確認した:
- kanade の outbound は `sakura: Sender<StartTalk>`（`actor.rs:57-62`）＝**`StartTalk` 以外を運べない**。
- `DispatcherMsg`（`dispatcher.rs:31-40`）に `ResolveChoice` が無い＝仮に運べても active talk の `handle.inbox`（`Sender<SakuraMsg>`）へ橋渡しされない。

**選択肢**:
- **G3-a（kanade 経由・単一調停）**: `Action::ResolveChoice{id}` を追加 → kanade の sakura channel を `Sender<StartTalk>` から新 enum（例 `TalkCommand::{Start(StartTalk), ResolveChoice{id}}`）へ差替 → ghost の start-relay（`relay.rs` の `From` 変換）と `DispatcherMsg` に `ResolveChoice` アームを追加 → dispatcher が `active.handle.inbox` へ `SakuraMsg::ResolveChoice` を中継。
  - ✅ 「1 選択の全帰結（カスケード・解決・タイムアウト・`choosing`）が kanade 1 箇所で調停される」＝Req1.1/4.6/5.4/6.5 が構造的に成立。
  - ❌ 波及が広い（kanade 公開 API・`MockSakura`（`tests/kanade/common/mod.rs:685-740`）・ghost 結線・dispatcher）。
- **G3-b（UI 直行・二経路）**: drain が `KanadeMsg::ChoiceSelected`（カスケード用）と `DispatcherMsg::ResolveChoice`（解決用）を**別々に**投函。
  - ✅ kanade の outbound 契約を触らない（波及最小）。
  - ❌ 1 選択が 2 経路へ分岐＝一回性・順序・失敗時整合（R5.3「204 でも解決は取りやめない」／R4.5）を**2 箇所で守る**必要。`deferral-requires-verified-owner` 的な単一真実源の劣化。
- **G3-c（dispatcher 集約）**: drain は dispatcher にだけ送り、dispatcher が kanade へ転送＋talk へ解決。dispatcher の責務（非対称吸収）を超え、`DispatcherMsg` が入力層の語彙を持つことになる＝structure.md の責務分割から逸脱。

### G4. 「選択待ち中」という事実の到達経路（R6.1/6.2/1.3/1.4/7.1）

**本 spec 最大の新規設計**。現状 kanade が知る talk の状態は `Steady{Some}`（再生中）／`TalkDone`（完了）の 2 値のみで、「バリアで停止している」は誰も伝えない。

**選択肢**:
- **G4-a（talk→kanade 通知の新設）**: `TalkDriver` が `WaitingForChoice` 遷移時に通知を送る。`spawn_talk` の done 端は `Sender<D>` where `D: From<TalkDone>`（`drive.rs:71-79`）ゆえ、境界拡張は `D: From<TalkDone> + From<ChoiceWaiting>` か、`TalkDone` と同居する新 enum の導入。dispatcher（`DispatcherMsg`）→ kanade（`KanadeMsg`）へ 1 本足す。
  - ✅ 真実源が再生層（`CuePlayer` の状態）＝Req7.2「独自の時間基準を導入しない」と自然に整合。通知に **horizon（`absolute_end_time`）と候補 id 集合**を同梱すれば G4/G5/R1.4 が一度に解ける。
  - ❌ areka-sakura／dola／ghost の 3 クレートに触る（`CuePlayer` に horizon getter or `pending_choices` の公開消費が要る）。
- **G4-b（UI 層から通知）**: drain が `TextLayerRuntime::choice_active` の遷移を監視して `KanadeMsg` を投函。
  - ✅ 追加クレートに触らない（areka bin 内で完結）。
  - ❌ brief 4 の明示方針「kanade は**表示状態でなく自分の配送状態**として保持＝単一真実源」に反する。表示層の遷移は per-frame ポーリングで、決定論檻が UI ランタイムを要求する（Req9.1 の「mock SHIORI・注入通知・注入時刻のみ」に反しやすい）。
- **G4-c（kanade 自前推定）**: 「Value に `\q` が含まれるか」を kanade が覗く。→ kanade がさくらスクリプトを解釈しない規律（不透明転写）に反するので**却下候補**として記録すべき。

### G5. タイムアウト（R7.1〜7.8）

- 型の入口は在る（`BarrierKind::WaitForChoice { timeout: Option<f64> }`・`command.rs:95`）。しかし `compile.rs:207-222` が **常に `None`** を書く。かつ `runtime.rs:242-248` は `WaitForChoice` の `timeout` を**読んでいない**（`BarrierReached::Choice` へ潰す）＝**現行は完全無期限**。
- `None` の意味衝突: 「未指定＝既定値を適用」と「`\*`／0／-1＝無効化」が同一表現。Req7.6/7.7 を満たすには `Option<f64>` の意味論を明文化するか、`enum ChoiceTimeout { Default, Disabled, Ms(u64) }` 的な語彙化が要る（`defer-canon-with-full-vocabulary-and-tracking-spec` 規律＝完全語彙＋縮退シーム）。
- 起点（表示完了）の観測: `CuePlayer` は `is_completed()` を「entry 枯渇 **かつ** horizon 到達」で判定するが、`WaitingForChoice` で停止中は `tick` が早期 return（`runtime.rs:183-187`）するため `is_completed` は false のまま。**「表示は終わったがバリアで待っている」を表す照会が無い**（`schedule.remaining()`／`absolute_end_time` から導けるが未公開）。
- 計測の住処の選択肢: **(i) talk アクター側で計測**（duration 権威に最も近い・注入 Tick で決定論）→ タイムアウト到達を kanade へ通知 → kanade が `OnChoiceTimeout` GET。**(ii) kanade 側で計測**（`MonotonicMs` の注入 Tick で計測・G4 通知に deadline を同梱）→ 時間基準が 2 系統（f64 秒 / ms）になるので換算点の裁定要。
- Ref0＝「タイムアウトしたスクリプト」（R3.4）: kanade は `StartTalk::new(talk_id, script)` を作った側なので、`ActiveTalk` に script を保持すれば供給可能（additive・`schedule/mod.rs:83-86`）。あるいは通知へ同梱。

### G6. 互換記録と正典整合（R8）

- 訂正先 `doc/emo2-conformance-scope.md:24` は 1 行。
- 対応表の住処候補: (i) `doc/` 配下に新規 md（例 `doc/choice-cascade-compat.md`）、(ii) `doc/COMPAT_ARCHITECTURE.md` へ節追加、(iii) `doc/shiori/fragments/` は**生成物（do not hand-edit）**ゆえ追記不可（ファイル冒頭に明記）、(iv) design.md 内の表（spec 完了で `completed/` へ移動するため参照性が落ちる）。
- 区別語彙は `14.choice.toml` の `provenance = "ukadoc" | "ssp_secondary"` が既存前例＝**areka 裁量は第 3 の値**（例 `areka_discretion`）で表現できる。

---

## 5. 正典（ukadoc）実測 — 証跡と揺れ

MCP `ukadoc` で再取得した一次記述（2026-07-31 実測）:

- **(a) `\q` は 6 形**（`list_sakura_script`）: `\q[タイトル,ID]` / `\q[タイトル,ID,r2,r3...]` / `\q[タイトル,ID1,ID2,ID3...]` / **`\q[タイトル,OnID,r0,r1,...]`** / `\q[タイトル,script:実行内容]` / `\q[ID][タイトル]`（旧仕様）。
- **(b) OnID 形の正典文**: 「ID が "On" で始まっている場合は、選択後、SHIORI イベント **OnID(書いた通りのイベント)** が開始される。それパラメータは r0,r1,... の順番に **Reference0 以降**に格納される。」→ **Req2.1/3.3 は正典に直接根拠あり**。**Ex/無印が先行するか否かは本記述が沈黙**（Req2.8 の対象そのもの）。
- **(c) 字面の揺れ（要記録）**:
  - `\q[タイトル,ID,r2,r3...]`: 「OnChoiceSelectEx が開始される。… **OnChoiceSelectEx に続いて OnChoiceSelect も発生するが**、Reference1 以降に何も入らずこの書き方では無意味。」＝**無条件で後続**と読める。
  - `\*`: 「選択時は通常通り、OnChoiceSelectEx イベント(**トークがなければ** OnChoiceSelect イベント)が発生する。」＝**204 ゲート**と読める。
  - ローカル正本 `doc/shiori/fragments/events/14.choice.toml:78`（`OnAnchorSelectEx`）: 「SHIORI が**何も返さなかった場合のみ**続けて OnAnchorSelect が発生する」＝アンカー系は 204 ゲートを明記。
  - → **Req2.3/2.4 は 204 ゲート説を採用**。アンカー系の明文＋`\*` 記述が根拠、`\q[…,r2…]` の記述が反証。**両方を対応表へ**（Req2.8）。
- **(d) 複数 ID 形の潰れ**: `\q[タイトル,ID1,ID2,ID3...]` は「ID* が Reference* に格納」＝OnChoiceSelect の Ref1 以降に**追加 ID**が載る形（`14.choice.toml:14` も `extra_choice_id`・provenance=`ssp_secondary`・「CROW のみ」と記載）。一方 areka の `ChoiceSelection`／`CueCommand::Choice` は `id` ＋ `references` の 2 分割ゆえ、**Ex 形（r2,r3…）と複数 ID 形をワイヤ形で区別できない**。Req3.2（OnChoiceSelect は Ref0＝ID のみ）は複数 ID 形を**構造的に非対応へ縮退**させる裁定にあたる → 対応表に「CROW 複数 ID 形は M1 非対応（縮退）」として記録するのが筋（要件本文には無い項目）。
- **(e) タイムアウト**: `\![set,choicetimeout,時間]`＝「単位はミリ秒。時間のカウントは**トークの表示が全て終わってから**開始。そのスクリプト中のみ有効。選択肢より後ろに書いても有効。タイムアウト時 OnChoiceTimeout。**時間指定を省略：デフォルト値に戻す** / **0 か -1：タイムアウトしない**」。**既定値の数値は ukadoc に記載なし**＝Req7.8 の「正典が数値を規定していない旨」は実測で確認済み。
- **(f) Reference 割付**: `OnChoiceSelectEx`＝Ref0 ラベル／Ref1 ID／Ref2+ 拡張（`\q` 3 番目以降）・「OnChoiceSelect よりも先に開始」。`OnChoiceSelect`＝Ref0 ID。`OnChoiceTimeout`＝Ref0 タイムアウトしたスクリプト。→ **Req3.1/3.2/3.4 は正典に直接根拠あり**。

---

## 6. 実装アプローチ（A / B / C）

### Option A: 既存コンポーネント拡張・kanade 単一調停（G1-a/c ＋ G2-i ＋ G3-a ＋ G4-a ＋ G5-i）

- **触る面**: `areka-kanade`（msg/schedule/events/steady/actor/status）・`areka-sakura`（contract の `SakuraMsg` は既存・`drive` に待機通知の送出）・`dola`（`CuePlayer` に horizon／待機照会の getter）・`areka-ghost`（`DispatcherMsg` ＋ relay）・`crates/areka`（drain）。
- ✅ 1 選択の全帰結が kanade で調停され、Req1.1/4.6/5.3/5.4/6.5 が**構造的に**成立。決定論檻が既存 kanade ハーネス上で完結（mock SHIORI＋MockSakura＋注入 Tick）＝Req9.1/9.2 に最短。
- ✅ 既存 SEAM コメント（`events.rs:54-58`・`status.rs:170`・`contract.rs:38`・`balloon.rs:123-129`）が全てこの形を前提に書かれている＝設計意図と一致。
- ❌ 5 クレート横断。`Sender<StartTalk>` の差替は kanade 公開 API と `MockSakura` を巻き込む。
- ❌ `steady.rs` の origin 政策・`Failed`→Fault 横断アーム（R4.5 衝突）に手が入る＝既存檻の再確認が要る。

### Option B: 新規コンポーネント・UI 側 drain が二経路投函（G3-b ＋ G4-b）

- **触る面**: `crates/areka`（新 drain モジュール＋選択状態の保持）＋ `areka-ghost`（`DispatcherMsg::ResolveChoice`）＋ `areka-kanade`（任意名イベント／`choosing` は依然必要）。
- ✅ kanade の outbound 契約・`MockSakura` に触らない。着手が速い。
- ❌ 一回性・順序・`choosing` の真実源が UI スレッドへ移る＝brief 方針および `areka-concurrency-model`（責務三分）に反する。
- ❌ 決定論檻が UI ランタイム（`Emo2Wiring`／`TextLayerRuntime`）を要求しがち＝Req9.1「mock SHIORI・注入通知・注入時刻のみ」を満たしにくい。
- ❌ Req6.5「消費側の比較に依存せず areka 側の調停のみで成立」の担保が弱い。

### Option C: ハイブリッド（段階実装）

- **Phase 1（カスケードのみ）**: 任意名イベント（G1）＋カスケード段（G2）＋既存 StartTalk 棚合流（R4）＋`script:` 縮退（R2.7）＋対応表・scope doc 訂正（R8）。**バリア解決は kanade の新 Action 経由（G3-a）まで含める**——ここを落とすと「メニューが一周」しない。
- **Phase 2（`choosing` ＋ 候補照合）**: talk→kanade の待機通知（G4-a）を新設し、`ExecutionSnapshot` に `choice_waiting` を追加。R1.3/1.4/6.1/6.2 が閉じる。
- **Phase 3（タイムアウト）**: 通知に horizon／deadline を同梱し `OnChoiceTimeout`（G5）。emo2 は本経路を使わない（ハンドラ無し＝204 経路のみ）ため**実機一周の可否には影響しない**＝最後尾に置ける。
- ✅ 「メニュー一周」（本 spec の到達目標）が Phase 1 完了時点で観測可能になり、リスクの高い新規経路（G4/G5）を後段へ隔離できる。
- ✅ Phase 2/3 が難航した場合の縮退（完全語彙＋シーム＋追跡 spec）を宣言しやすい。
- ❌ Phase 境界で `ExecutionSnapshot`／通知型に 2 度触る（Phase 1 で席だけ予約する設計にすれば緩和可能——`status.rs:209-219` の NOTE が既にその作法を指示している）。

---

## 7. 工数・リスク

| 区分 | 工数 | リスク | 根拠 |
|---|---|---|---|
| G1 任意名イベント＋許可カテゴリ | **S** | Low | 型変更は機械的。既存 8 ID は `Borrowed` で無改変。檻は `events.rs` テストの追補のみ。 |
| G2 カスケード段（`State` 追加＋純関数判定） | **S〜M** | Low-Medium | `drive` の reinject ループが既に多段の器。`pending_close` と同型の帳簿追加。 |
| G3 解決経路（channel 型差替＋dispatcher アーム） | **M** | **Medium** | kanade 公開 API・`MockSakura`・ghost 結線・dispatcher の 4 点同時変更。既存 boot/close 檻の再確認が要る。 |
| G4 待機通知（talk→dispatcher→kanade） | **M〜L** | **High** | 新規のクロスクレート経路。`spawn_talk` の `D: From<TalkDone>` 境界拡張・`CuePlayer` の照会追加。決定論檻の設計（注入時刻のみで待機遷移を作る）が最も難しい。 |
| G5 タイムアウト | **M** | **High** | 起点の住処・`Option<f64>` 意味衝突・既定値の裁定・204 時の「解除して終了」の正規経路（`skip_barrier` の到達口）が全て未決。emo2 未使用ゆえ実機で検証できない＝檻のみが根拠。 |
| G6 互換記録・doc 訂正 | **S** | Low | 1 行訂正＋表 1 枚。 |
| 決定論檻（Req9.1/9.2 の 5 項目） | **M** | Medium | 器は既存（`Fixture`/`MockShiori`/`MockSakura`/`Harness`）。(c) `choosing` と (d) タイムアウトは G4/G5 に従属。 |
| **合計** | **L（1〜2 週間）** | **Medium-High** | 単体は小粒だが 5 クレート横断＋新規経路 2 本（G4/G5）がリスクを押し上げる。 |

**追加リスク**:
- **R4.5 と既存横断アームの衝突**: `mod.rs:317-323` は応答待ち中の `Failed` を**無条件で** `Unloading{Fault}` へ倒す。要件は「選択由来の失敗は 204 相当で継続」。prefetch が `mod.rs:313-315` で同種の例外を先行させている前例があるので、同型の先行アームで解けるが、**既存の failure 檻（`failure_test.rs`）との意味衝突を確認する必要**。
- **`steady.rs:208-211` の warn 破棄**: choice 由来 origin を `:191` の明示列挙へ加え忘れると、選択の応答が**沈黙のうちに破棄**される（warn は出るが talk は起動しない）。設計コメントが「wildcard にしない」と明記しているのは正にこの罠の予防＝**必ず檻で固定する**。
- **W6.5 `test-cage-determinism` との同ファイル衝突**: `input_events/balloon.rs`（毒化 18 呼出の対象）。先着の本 spec が正・後着 cage が rebase（roadmap 追記(52) 裁定）。
- **並走 brief の陳腐化**: design 前に `origin/main` へ rebase して再突合すること（[[parallel-worktree-brief-staleness-rebase-before-design]]）。

---

## 8. Research Needed（design フェーズへ持ち越す調査項目）

1. **SSP 実挙動**: OnID 形で `OnChoiceSelectEx`/`OnChoiceSelect` が先行発火するか（正典沈黙・§5-b）。ukadoc だけでは決まらないため、`\*` 記述の反対解釈・アンカー系の明文（§5-c）・SSP ソース／実挙動報告のいずれかで裏取りするか、**areka 裁量として対応表に記録**して閉じる。
2. **`OnChoiceTimeout` の既定値**: ukadoc に数値記載なし（実測確認済み）。SSP の de-facto 値（他ベースウェア実装・ゴースト作者ドキュメント）を探すか、areka 裁量値（例 30_000ms）を宣言して記録する。
3. **カスケード最終段が 204 を返した場合**（Req2.8 の明示対象）: 選択肢集合はどうなるか。areka 現行実装は `resolve_choice` 成功で**破棄**（`runtime.rs:295-301`）＝実装が先に答えを持っている。正典に根拠があるかの確認。
4. **`\q[タイトル,ID1,ID2,ID3...]`（CROW 複数 ID 形）**: areka のワイヤ形では Ex 形と区別不能（§5-d）。M1 非対応の明示縮退として対応表へ載せるかの裁定（要件本文に無い項目＝ディスカッション議題候補）。
5. **`CuePlayer` の照会面**: 「表示は完了したがバリア待ち」を表す照会（horizon 到達 ∧ `WaitingForChoice`）を dola 側に足すか、`absolute_end_time` を talk アクター経由で外へ出すか。dola の Non-Goals（pause/resume を持ち込まない等）との整合確認。
6. **`skip_barrier` の外部到達口**（R7.5「解除して終了」）: `SakuraMsg` へ additive アームを足すのが正規か、`Close`（`Interrupted` 終端）で足りるか。`canonical-not-minimal-lifecycle` 規律に照らした裁定。

---

## 9. design 判断項目（要件ディスカッションへの申し送り・番号付き）

> いずれも本ギャップ分析では**決定しない**。選択肢と根拠のみを提示する。

1. **DD-1 任意名イベントの型表現**: `ShioriCall.id` を `Cow<'static, str>` にするか、`GetDynamic` variant を additive 追加するか。あわせて `origin` を `Cow` 化するか **`Origin` enum へ昇格**するか（`steady.rs:189` の「wildcard にしない」設計意図を型で保てるのは後者）。
2. **DD-2 許可規則のカテゴリ追加と恒久禁止の交差**: `is_allowed_choice_event(id)` の定義。`\q[x,OnTalk]` / `\q[x,OnHour]` を書くゴーストの扱い——(a) 恒久禁止が勝つ（送出せず warn・`events.rs:54-58` の申し送りが想定する形）／(b) 任意名カテゴリが勝つ（正典どおり発火）／(c) 禁止＋対応表記録。**`events.rs` の SEAM コメントが本 spec の要件フェーズでの決着を明示的に要求している**。
3. **DD-3 カスケード段の保持場所**: `State.pending_cascade`（Phase 不変・`pending_close` と同型）か `Phase::Steady` の拡張か。Req4.4「既存の決定的状態機械の観測資産を変更しない」に最も忠実なのは前者。
4. **DD-4 カスケードと barrier 解決の順序・独立性**: (a) 解決を先に投函してからカスケード開始／(b) カスケード完了（Value か最終 204）を待って解決／(c) 同一 step 内で両 Action を並べる。Req5.3（204/失敗でも解決を取りやめない）と Req4.6（高々 1 StartTalk）と Req5.4（解決高々 1 回）が同時に成立する順序はどれか。`on_resolve_choice` が **その場で `TalkDone` を送り得る**（`drive.rs:370-375`）ため、解決を先行させると `Steady{Some}`→`Steady{None}` 遷移がカスケード応答より先に届き得る（＝`steady.rs:166-176` の talk 起動アームが変わる）点に注意。
5. **DD-5 kanade→talk の到達経路**: G3-a（`Action::ResolveChoice` ＋ sakura channel の enum 化 ＋ `DispatcherMsg::ResolveChoice`）か G3-b（UI drain が dispatcher へ直行）。単一調停 vs 波及最小のトレードオフ。
6. **DD-6 「選択待ち中」の真実源**: G4-a（talk アクター発の通知）か G4-b（UI 表示層の監視）か。brief 4 の方針（kanade の配送状態＝単一真実源）と Req9.1（mock のみで決定論）は G4-a を示唆する。
7. **DD-7 候補集合照合（R1.4）の担当**: (a) 通知に候補 id 集合を同梱して kanade が照合／(b) 照合を talk アクターに委ね `resolve_choice` の `None` を ack で返す（新たな往復）／(c) 上流保証に委ね kanade は棄却しない（要件と衝突）。現行 `PendingChoice` は `references` を積んでいない（`runtime.rs:217-222`）ため、(a) を採るなら id のみで足りるかの確認も要る。
8. **DD-8 タイムアウト値の語彙と入口**: `Option<f64>` の意味衝突（未指定 vs 無効化）を `enum ChoiceTimeout{Default,Disabled,Ms}` 等で解くか、`None`＝既定・`Some(0.0)`＝無効の規約で押し切るか。入口の物理位置は dola `BarrierKind`（compile が既定を焼く）か kanade `KanadeConfig`（`close_talk_deadline_ms` が前例）か。**compile 側の変更は `areka-P0-sakura-time-directives`（M1 外・追跡 spec）の領分に触れる**ため境界の明示が要る。
9. **DD-9 タイムアウト計測の住処**: talk アクター（f64 秒・duration 権威直結）か kanade（`MonotonicMs`・注入 Tick）か。前者は Req7.2 に忠実、後者は SHIORI 発火と同一アクターで一回性を守りやすい。
10. **DD-10 `OnChoiceTimeout` Ref0 の供給**: `ActiveTalk` に script を保持（additive）か、待機通知に同梱か。前者は kanade が既に script を作っている事実（`steady.rs:175`）を利用できる。
11. **DD-11 R7.5 の「解除して終了」の正規経路**: `skip_barrier` の外部到達口を新設するか、`SakuraMsg::Close`（`Interrupted` 終端）で代替するか。後者は `TalkDone{Interrupted}` が kanade の防御アーム（`mod.rs:275-279`）へ落ちる点の確認が要る。
12. **DD-12 選択由来 SHIORI 失敗の扱い（R4.5 vs 既存横断アーム）**: `mod.rs:317-323` の `Failed`→`Unloading{Fault}` を choice origin で例外化するか（prefetch の `mod.rs:313-315` が前例）。既存 `failure_test.rs` との意味衝突の有無。
13. **DD-13 `scope` の用途（R3.7）**: 現行 talk は単一 slot（dispatcher の `Option<ActiveTalk>`）＝scope で解決対象を特定する余地が実質無い。`ChoiceSelection.scope` を (a) 検証にのみ使う／(b) 将来 per-scope 化のシームとして保持しログにのみ載せる／(c) 完全に無視して warn しない、のいずれか。
14. **DD-14 対応表（Req2.8/8.1/8.2）の住処と書式**: `doc/` 新規 md か `COMPAT_ARCHITECTURE.md` 追記か。`doc/shiori/fragments/` は生成物ゆえ**手編集不可**（ファイル冒頭に明記）。区別語彙は `provenance = ukadoc | ssp_secondary | areka_discretion` を採るか。
15. **DD-15 CROW 複数 ID 形の縮退明示**: `\q[t,ID1,ID2,ID3]` はワイヤ形で Ex 形と区別不能（§5-d）。M1 非対応として対応表へ記録するか、要件本文に無いため design 裁量とするか。
16. **DD-16 段階実装（Option C）の採否と縮退宣言**: G4/G5 を後段へ隔離する場合、`defer-canon-with-full-vocabulary-and-tracking-spec` の 4 点セット（完全語彙＋縮退シーム＋追跡 spec＋roadmap 明記）をどこまで前倒しで用意するか。特に「タイムアウトは実装するが emo2 では 204 経路のみ通る」＝実機で検証できない領域の受け入れ条件（Req9.3 は「メニュー一周」のみを実機に課しており、タイムアウトは檻のみが根拠）。
