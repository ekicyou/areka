# ギャップ分析: areka-P0-balloon-visibility

> 実施日 2026-08-11 ／ 対象ツリー: worktree `claude/areka-p0-balloon-visibility-b341d2`（base `fdddef4` ＝ PR#103 `file-slimming` マージ後）
> **brief の file:line は PR#103 のファサード分割で全面的に陳腐化していたため、本書のアンカーは全て現ツリーで再検証した実測値である。**
> 位置づけ: 情報提供（決定ではない）。選択肢と論点を並べ、要件ディスカッションと設計フェーズへ渡す。

---

## 0. アンカー再検証（brief 追記(60) からのドリフト）

`frame.rs`（旧 1,532 行）は `frame.rs`（202 行のファサード）＋ `frame/{attach,dpi,drain_resnap,scale_text,wiring}.rs` へ純移動された。本 spec の主戦場は **`frame/attach.rs`** である。

| brief の記載（追記(60)・2026-08-06） | 現ツリーの実位置（2026-08-11 実測） | 判定 |
|---|---|---|
| 主犯 ShowSurface `frame.rs:541-550` | **`frame/attach.rs:327-336`**（`PresentCommand::ShowSurface{ target: item.balloon_target, surface_id: 0, .. }`） | 移動・内容不変 |
| `run_attach_phase` `frame.rs:369-581` | **`frame/attach.rs:155-367`** | 移動 |
| `connect_balloon_text` `frame.rs:589-608` | **`frame/attach.rs:375-394`**（呼出は `:340-347`） | 移動 |
| `text_slot_view` 取得 `frame.rs:553` | **`frame/attach.rs:339`** | 移動 |
| `emo2_frame_system` `frame.rs:1466-1496`（7 フェーズ） | **`frame.rs:135-165`**（7 フェーズ構成は不変: attach `:141`→dpi `:148`→drain `:149`→move_drain `:153`→resnap `:157`→text_scale `:162`→text `:163`） | 移動・構成不変 |
| `Emo2Wiring` `frame.rs:181` | **`frame/wiring.rs:35`**（`new` `:90`・`runtime()` `:145`・`presenter()` `:127`） | 移動 |
| `run_text_scale_phase` `frame.rs:1045` | **`frame/scale_text.rs:77`** | 移動 |
| `on_talk_done` `steady.rs:827` | **`areka-kanade/src/schedule/steady.rs:827`**（横断アームは `schedule/mod.rs:488`） | 一致 |
| **`TalkDone` は `crates/areka/src` に出現ゼロ** | **2026-08-11 実測でも出現ゼロ**（`areka-talk`／`sakura`／`ghost`／`kanade`／`dola` のみ） | **gap の核は有効** |
| バルーン窓 `HitTest::none()` `spawn.rs:245/:273` | **`placement/spawn.rs:245`（balloon）／`:273`（char）** | 一致 |
| hover donor は「既設消費」へ昇格 | **本番結線を確認**（`main.rs:363` `wire_balloon_choice`／`main.rs:731` `attach_balloon_pointer_handlers`）。ただし `input_events/balloon.rs` の doc コメント群（`:315`/`:472`/`:780`/`:804`/`:821`）は「本番到達者なし（M1 暫定抑止）」のまま**記述が陳腐化**しており、`#[allow(dead_code)]` も残置 | 昇格は真・注記は要更新 |

---

## 1. 現状資産の棚卸（Current State Investigation）

### 1.1 表示の「実行手段」は端まで貫通済み（消費するだけでよい）

```
seriko ScopeStates::apply_balloon   crates/areka-seriko/src/state.rs:243-275（冪等ガード :247/:263）
  → DisplayCommand::ShowBalloon / HideBalloon      crates/areka-seriko/src/output.rs:50
  → adapter map_display_command                    crates/areka/src/emo2_boot/adapter.rs:35-75（Hide 写像 :70-73）
  → PresentBridge（mpsc 非ブロック）                crates/areka/src/emo2_boot/adapter.rs:87-125
  → run_drain_phase（try_iter→presenter.apply）     crates/areka/src/emo2_boot/frame/drain_resnap.rs:34-51
  → EmoPresenter::apply                            crates/areka-emo-present/src/presenter/hub.rs:90-104
      ├ apply_show                                 crates/areka-emo-present/src/presenter/show.rs:23-277
      └ apply_hide                                 crates/areka-emo-present/src/presenter/hub.rs:107-127
  → VisualMount::set_visible                       crates/areka-emo-present/src/mount.rs:193-216
```

`set_visible(false)` は `Visual::set_visible(false)`（→ `visual_sync.rs:264-269` が WUC opacity を 0.0 にする）＋ `HitTest::none()` を同時に行う。**Requirement 1.4（不可視時のクリック透過）は既存 hide 経路が構造的に満たしている**（surface entity に限る。§3.1 の欠落を参照）。

### 1.2 「可視コンテンツの配置」の観測点は既に存在する

| 観測 | 実体 | 位置 |
|---|---|---|
| グリフ追記の唯一の入口 | `TextLayerState::apply_cue` の `Text` 腕／`Choice` 腕 | `areka-emo-text/src/state.rs:325-341` / `:367-407` |
| 非可視 cue（表示契機にしてはならない） | `NewLine`(`:342`)／`Cursor`(`:408`)／`Clear`(`:347`)／`ClearAll`(`:352`)／`Wait`・`Emote`・`BalloonSurface`(`:426-432`) | 同上 |
| **時刻 t での可視グリフ数** | `TextLayerState::visible_glyphs(actor, t)` | `areka-emo-text/src/state.rs:440-444` |
| UI からの到達手段 | `Emo2Wiring::runtime()` → `TextLayerRuntime::state()` | `frame/wiring.rs:145` / `actor.rs:468` |

`visible_glyphs` は **グリフのみを数え、改行・カーソル・待機・消去を数えない**。Requirement 2.3 の「可視コンテンツ」定義と**そのまま一致する**（新しい判定語彙を発明する必要がない）。
`ClearAll` は全 actor の state を初期化する（`state.rs:363-365`）ため、`visible_glyphs == 0` が全 scope で同時成立する ＝ **Requirement 3.1／3.6 が同じ 1 つの観測から導出できる**。

### 1.3 会話終了信号は UI へ届いていない（本 spec の新設面）

- `TalkDone{talk_id, reason}` の物理正本: `crates/areka-talk/src/lib.rs:110-115`（`TalkEndReason::{Ended, Interrupted, ...}` `:100-108`）。
- 経路: sakura talk → dispatcher `on_done`（`areka-ghost/src/dispatcher.rs:314-332`・kanade 転送 `:318`）→ kanade 横断アーム（`schedule/mod.rs:488`）→ `steady::on_talk_done`（`schedule/steady.rs:827`）。**ここで終端**。
- dola 側の占有 horizon 権威: `CuePlayer::is_completed`（`dola/src/cue/runtime.rs:362`）／`occupancy_horizon`（`:377`）。ただし `CuePlayer` は talk スレッド所有で UI から参照できない。
- **UI が既に持っている材料**: 全 cue が broadcast で UI 側 sink（`ClockedTextSink`＝`emo2_boot/talk_clock.rs:85-109`）へ届き、`cue.at`／`cue.duration` を読める。`MoveCueSink`（`emo2_boot/move_cue.rs`）が「UI 向け mpsc を持つ 4 番目の sink」の**既成の先例**である（`wire_emo2_boot` の `sinks: vec![...]` は `emo2_boot/mod.rs:400-404`）。

### 1.4 時刻源

| 用途 | 実体 | 備考 |
|---|---|---|
| フレーム時刻（QPC 秒・フレーム内一貫・**注入可能**） | `wintf::ecs::FrameTime`（`wintf/src/ecs/graphics/core.rs:147`） | Requirement 4.9／9.2 の要求（実時間待機なし）を満たす唯一の適材 |
| talk 起点相対秒 | `TalkClock::talk_time`（`emo2_boot/talk_clock.rs:63-76`） | **タイムアウト計測には不適**（新 talk で epoch が前方リベース・負値 0 clamp） |
| talk_time 解決の純判断 | `resolve_talk_time`（`frame/scale_text.rs:216-227`） | override 注入口が既にある＝檻の donor |
| SERIKO 16ms ticker | `spawn_loop_ticker`（`emo2_boot/mod.rs:451-456`） | seriko 専用・UI 判断には使わない |

### 1.5 抑止条件（Requirement 5）の観測源は全て既設

| 抑止条件 | 既存資産 | 位置 |
|---|---|---|
| バルーンドラッグ中 | `OnDrag(on_balloon_drag)`／`OnDragEnd(on_balloon_drag_end)` がバルーン窓へ本番装着済み。`DraggingState` component も実 flow で挿入される | `placement/spawn.rs:251/:255`・`placement/follow/drag_follow.rs:494/:581` |
| ポインタ滞在 | `on_balloon_pointer_moved`（バルーン窓の Bubble ハンドラ・選択肢の有無に関わらず着火）／離脱は `clear_balloon_hover_on_leave`（`PointerLeave` マーカー） | `input_events/balloon.rs:318`／`:641`。本番結線は `main.rs:731`／`:363` |
| 選択肢表示中 | **`TextLayerRuntime::choice_active(actor)` が既に読める**（新 seam 不要） | `areka-emo-text/src/actor.rs:541` |

### 1.6 「頭脳」だけが不在

起動時にバルーンへ無条件の表示指令を出しているのは attach 相の 1 箇所のみ（`frame/attach.rs:327-336`）。**シェル側には既に同型の「初回表示を出さない」先例がある**（`frame/attach.rs:276-283`・defect #5 の是正コメント）——シェルは `attach_target` で target だけ作り、最初の `\s` cue が運ぶ `ShowSurface` を待つ。本 spec はこれをバルーンへ適用するが、**バルーンには「文字層スロットの確保が初回 ShowSurface に依存する」という追加の縛りがある**（§3.2）。

---

## 2. 要件 → 資産 対応表

タグ: **✅既存**（そのまま消費）／**➕追加**（新規実装）／**⚠Missing**（現状に能力そのものが無い）／**❓Unknown**（要調査）／**🔒Constraint**（既存構造による制約）

| 要件 | 必要な技術要素 | 既存資産 | 判定 |
|---|---|---|---|
| 1.1／1.6 起動時の全 scope 不可視 | attach の無条件 ShowSurface 撤去 | `frame/attach.rs:327-336` を削除するだけ。`plan.items` は全 scope を回る（`:240`）ため部分適用にならない | ✅ 削除で成立 |
| 1.2 一瞬の点滅も禁止 | show→hide の同一フレーム往復を採らない構造 | — | ⚠ §3.2 の判断が必要 |
| 1.3 可視化を伴わない配置先確保 | 「表示せずに mount／chain／applied／native_size を確立する」能力 | `text_slot_view` は mount＋chain＋applied＋native_size の 4 点を要求（`presenter/read.rs:95-111`）。これらを作るのは `apply_show` のみで、同関数は末尾で必ず `set_visible(true)`（`presenter/show.rs:211-213`） | **⚠ Missing（中核）** |
| 1.4 不可視時のクリック透過 | 可視切替と同時の当たり判定停止 | `VisualMount::set_visible` が `HitTest::none()` を同時適用（`mount.rs:203-215`） | ✅ 既存 |
| 1.5 装着失敗の log-first | error!＋既存失敗経路 | attach 相は全分岐 log-first（`frame/attach.rs:246/:273/:290/:306/:322`） | ✅ 既存規律 |
| 2.1／2.2／2.5／2.6 可視コンテンツ駆動の scope 別 show | scope→actor 写像・エッジ検出・冪等 | `ActorKey::from(scope.to_string())`（attach `:345`／再追従 `scale_text.rs:97` と同一写像）・`visible_glyphs` | ➕ 頭脳の新設（判定材料は全て既存） |
| 2.3 可視コンテンツの定義 | 文字・記号を数え改行等を数えない | `visible_glyphs`（`state.rs:440`）が厳密に一致 | ✅ 既存 |
| 2.3「画像等」 | バルーン内画像要素 | `TextItem` は `Glyph`／`LineBreak`／`CursorMove` の 3 種のみ（`areka-emo-text/src/state.rs:71-90`）＝**画像要素は M1 に存在しない** | 🔒 制約（語彙は将来 additive） |
| 2.4 scope 切替時の追加表示（既表示を戻さない） | per-scope 独立状態 | per-scope の `visible_glyphs` は独立 | ➕（設計で per-scope 状態を持てば自明） |
| 2.7 無発話 scope の非表示継続 | 同上 | 同上 | ➕ |
| 3.1／3.2／3.6 会話冒頭の全消去連動 | `ClearAll` 観測 | `ClearAll` は cue.actor に依らず全 actor を初期化（`state.rs:352-366`）＝**先読み不要で全 scope 同時に 0 になる** | ✅ 既存観測から導出 |
| 3.4／3.5 空バルーン可視の禁止・旧内容の残存禁止 | 可視化とテキスト描画の同一フレーム順序 | `emo2_frame_system` の相順（`frame.rs:141-163`）を利用。`present_frame` は text 相（`:163`）で走る | 🔒 相の挿入位置が要件充足の鍵（§4） |
| 4.1 表示終了起点 | 会話終了信号の UI 到達 | **無し**（§1.3） | **⚠ Missing** |
| 4.2 既定 30 秒・単一定義箇所 | 定数 1 箇所 | sylphya に balloon timeout 語彙は**未登録**（`areka-sylphya` に `timeout` の出現ゼロ） | ➕ 定数 or sylphya 語彙追加（❓討議） |
| 4.3／4.7 タイムアウト hide と再表示 | ラッチ状態 | — | ➕（§3.4 の注意） |
| 4.4 hide は可視状態のみ変更 | 内容・会話状態を触らない | `apply_hide` は cache／chain／`native_size` を保持（`hub.rs:107-127`）・`refresh_actor_scale` は純粋状態に触れない（`actor.rs:328-334`） | ✅ 既存契約 |
| 4.5 次会話開始で計測破棄 | ClearAll 観測 | ✅（3.1 と同源） | ➕ |
| 4.6 中断も同一起点 | `TalkEndReason::Interrupted` の扱い | `TalkDone.reason` は 3 値（`areka-talk/src/lib.rs:100-115`）。horizon 観測方式では reason を区別できない | ❓ 方式選択に依存（§4.2） |
| 4.8 信号欠落時は表示保持＋観測 | 縮退規律 | log-first 規律は既存 | ➕ |
| 4.9／9.2／9.3 注入時刻駆動 | フレーム時刻の注入 | `FrameTime`（注入可）＋`resolve_talk_time` の override 先例（`scale_text.rs:216`） | ✅ 先例あり |
| 5.1 ドラッグ中抑止 | ドラッグ状態の観測 | `OnDrag`／`OnDragEnd` 本番装着済み。ただし `DraggingState` のポーリングは**多窓時に DragEnd 前に落ちる既知の穴**あり（`drag_follow.rs:159-174`） | ➕（エッジ追跡を推奨・§6-R3） |
| 5.2 ポインタ滞在抑止 | 滞在の観測 | `on_balloon_pointer_moved`（`balloon.rs:318`）＋`clear_balloon_hover_on_leave`（`:641`）。**不可視時は `HitTest::none()` ゆえ着火しない**＝抑止は可視時のみ働く（正しい） | ➕（既設消費） |
| 5.3 解除後の再計測 | ラッチ | — | ➕ |
| 5.4 選択肢表示中の連携口 | 外部からの状態受領 | `TextLayerRuntime::choice_active`（`actor.rs:541`）が既に読める＝**新 seam を作らずに「受け取る」形にできる** | ✅ 既存（要件文言との整合は §7-D9） |
| 5.5 観測不能時は非抑止側へ | 縮退 | 既存 `try_borrow` 失敗＝error!＋no-op 流儀（`balloon.rs:705-715`） | ➕ |
| 6.1 `\b[-1]` は即時 | 明示指令の優先 | seriko→adapter→`apply_hide` 経路は本 spec 非干渉で通る | ✅／⚠（§3.3 の二重所有） |
| 6.2 `\b[ID]` は可視性を変えない | 「表示せずに面だけ切り替える」能力 | **無し**——`apply_show` は必ず `set_visible(true)`（`show.rs:211`） | **⚠ Missing（1.3 と同一の欠落）** |
| 6.3 `\b` 経路の回帰緑 | 既存檻 | `spine_display_tests.rs:152-260`（Hide→Show の発行順序＋readback 一致） | 🔒 一部は前提が変わる（§3.6） |
| 6.4 scope 別資産の不変 | 触らない | `balloon_models`（`wiring.rs:61`）・`BalloonScopeAssets` | ✅ |
| 6.5 位置・追従・永続の不変 | 触らない | `placement/follow/*`・`persist.rs` | ✅（`placement/follow/visibility.rs` は**窓位置**の可視性ガードであり別物——命名衝突に注意） |
| 6.6 非表示期間中の変化を取りこぼさない | 再表示時の k 追従 | `refresh_scale` は**不可視なら再スケールしない**（`refresh.rs:74-80`）。再表示時に `apply_show` が現 DPI から k を導出し直す（`show.rs:38-43`）＋`run_text_scale_phase` が毎フレーム binding を組み直す（`scale_text.rs:77-119`） | ✅ 構造的に成立見込み（❓窓寸 reconcile の同一フレーム着地＝§4 の相順） |
| 6.7 キャラ窓へ波及しない | target 分離 | `shell_target`／`balloon_target`（`target_map.rs`）が別 target | ✅ |
| 7.1 `\![set,balloontimeout]` 語彙記録 | 汎用キャリア | `CueCommand::Custom` 汎用キャリア（`dola/src/cue/command.rs:163`・構築 `:201`・抽出 `:213`）＝**受理せずとも「消費者ゼロの口が実在する」形が既にある** | ✅ 記録＋注記 |
| 7.2 SHIORI 3 イベントの型シーム予約 | UI→会話進行の口 | 現状 UI→kanade の口は `MouseWiring`（`input_events/mod.rs:243`）／`ChoiceForwarder`（`choice_drain.rs:126`）の 2 先例 | ➕ 型予約のみ |
| 7.3〜7.8 対応表・追跡先 | `doc/COMPAT_ARCHITECTURE.md` §8 沈黙ルール対応表（`:122-`） | 既存の記録先が確定している（`\![move]` 群の行が書式の先例） | ✅ 追記のみ |
| 8.1〜8.6 観測ログ | tracing 規律 | `.kiro/steering/logging.md`＋`apply_show` の `info!`（`show.rs:258-275`）が「実機 grep 前提は info! が契約」の先例 | ➕（水準選択は §7-D10） |
| 9.1 決定論檻 | headless 純判断の切り出し | `resolve_talk_time`（純関数＋override）・`plan_attachments`（純関数）の 2 先例 | ➕ |
| 9.4／9.5 実機サインオフ | 有界 auto-exit＋ログ grep | `AREKA_APP_SMOKE_EXIT_MS`（`main.rs:664/:768/:808/:868`）・`crates/areka/tests/emo2_real_run.rs`（`AREKA_EMO*` 絶対パス） | ✅ 既存定石 |
| 9.6 既存テスト・注記の更新 | 洗い出し | §3.6 に一覧 | ➕ |

---

## 3. 本分析で新たに判明した論点（設計前に必ず読むこと）

### 3.1 【重大】既存 hide 経路は**文字層を隠さない**（構造的欠陥）

`VisualMount::attach`（`mount.rs:94-157`）は 2 つの entity を作るが、**両方とも窓の子（兄弟関係）** である:

- `text_slot`: `ChildOf(window)` で spawn（`mount.rs:129-135`）——描画 z を上にするため surface より**先に**追加
- `surface_entity`: 同じく `ChildOf(window)`（`mount.rs:138-148`）

そして `VisualMount::set_visible`（`mount.rs:193-216`）は **`surface_entity` の `Visual`／`HitTest` しか触らない**。
文字層の実描画面は `TextSurface::attach` が **`binding.slot`（＝`text_slot` entity）自身へ `VisualGraphics` を挿す**（`areka-emo-text/src/surface.rs:239-262`）。WUC の opacity 継承は親子でしか効かない（`visual_sync.rs:264-269` は当該 entity の Visual にのみ opacity を書く）。

**帰結**: `apply_hide`／`\b[-1]` を実行しても **バルーン枠の絵だけが消え、文字はそのまま画面に残る**。
本 spec の Requirement 3.4（内容が空のバルーンが可視である状態を作らない）・4.3（全 scope を非表示）・1.2（可視となる瞬間を作らない）は、この欠落を塞がない限り**実機で成立しない**。

- 既存の `\b[-1]` 檻（`spine_display_tests.rs:152-260`）は emo-present の **供給面 readback** を見ているだけで、文字層の別 visual を観測していないため**この欠陥を構造的に検出できない**。
- 最小の修正点: `VisualMount` は `text_slot` を**所有している**（`mount.rs:82`・`text_slot()` `:224`）。`set_visible` が両 entity を扱う形にすれば、`\b[-1]` 側も同時に是正される（Requirement 6.3 の「非退行」に対しては**挙動改善**）。
- 代替: 頭脳側が `TextSlotView::slot`（`presenter/read.rs:103-110` が返す）から slot entity を引いて自前で隠す。ただし「実行手段側に規則を持ち込まない」原則（Boundary Context）とは逆に、**頭脳が 2 つの entity を知る**ことになる。

> **設計判断 D1（§7）** として上げる。要件 6.3 の「回帰テストを緑のまま維持」との関係も要判断（既存檻は「枠が消える」ことしか見ていないので緑のまま通る）。

### 3.2 【重大】`apply_show` は「可視化」と不可分——Requirement 1.3 と 6.2 は**同一の欠落**を指している

`apply_show`（`show.rs:23-277`）の 1 回の呼び出しが、以下を**全て一括で**行う:

1. k 導出（`:38-43`）
2. 合成／キャッシュ引き当て（`:45-113`）
3. **chain／mount の遅延生成**（`:115-182`）← Requirement 1.3 が欲しいのはここまで
4. upload＋αマスク同期＋**`mount.set_visible(world, true)`**（`:184-213`）← ここが不可分に付いてくる
5. `applied`／`native_size`／`last_show`／`pending_resize` の確定（`:230-245`）← `text_slot_view` が `Some` になる条件（`read.rs:101-102`）

`text_slot_view` は 3 と 5 の両方を要求するため、**「表示せずにスロットだけ確保する」は現 API では表現できない**。
同じ欠落が Requirement 6.2（不可視のまま `\b[ID]` の面切替結果だけ保持）にも現れる——seriko の `ShowBalloon` は必ず `ShowSurface` へ写り（`adapter.rs:58-68`）、`apply_show` は必ず可視化する。

**したがって「可視化と分離した表示状態更新」という 1 つの能力が、1.3 と 6.2 を同時に解く。** 選択肢は §4.1。

### 3.3 【注意】バルーン可視性の所有者が二重化する

seriko `ScopeStates.balloon`（`state.rs:243-275`）は「バルーンが表示中か・どの面か」を**自前で持ち、冪等ガードを掛ける**（同一 `Shown(id)` なら再発行しない `:247`／既に `Hidden` なら再発行しない `:263`）。
新しい頭脳が presenter へ直接 Show/Hide を出すと、seriko の状態と実表示が乖離する。具体的な壊れ方:

- タイムアウトで頭脳が hide → seriko は `Shown(0)` のまま → 次の `\b[0]` が `Unchanged` になり **seriko からの再表示が出ない**（頭脳が再表示するので実害は出ないが、状態は嘘になる）。
- 逆に `\b[-1]` で seriko が `Hidden` → 頭脳が可視コンテンツを見て show → seriko は `Hidden` のまま。

現状でも attach 相が presenter へ直接 `ShowSurface` を出しており（`frame/attach.rs:327-336`）、seriko の `balloon` map を経由していないので**乖離は既に存在する**（良性）。本 spec は乖離を常態化させるため、**「バルーン窓の可視性の唯一の所有者は頭脳である」と宣言し、seriko 側は面 ID の所有に限る**、という境界の明文化が要る（設計判断 D3）。

### 3.4 【注意】「可視コンテンツ ⇒ 可視」は**純述語では書けない**

Requirement 4.4 が「タイムアウト非表示では内容を消さない」と定めるため、タイムアウト後も `visible_glyphs > 0` のままである。
したがって `可視 == (visible_glyphs > 0)` という純関数にすると、**タイムアウトの次フレームで即座に再表示される**。

必要なのは per-scope の小さな状態機械:

- `shown: bool`（今この scope のバルーンを表示させているか）
- `content_seen: usize`（最後に観測した可視グリフ数——**増加エッジ**が show の契機・Requirement 2.5 の「2 つ目以降で再発行しない」もこれで自然に満たす）
- `timed_out: bool`（タイムアウトで消したラッチ。`ClearAll`＝`visible_glyphs` が 0 へ落ちた瞬間、または `content_seen` の増加で解除）
- 会話単位の `display_end_at: Option<f64>`（`FrameTime` 基準の絶対秒）と `suppress_since: Option<f64>`

要件の「単一規則から導出」（2.6）は **show の契機**についての主張であり、hide には時間軸の第 2 規則（4.x）とラッチが要る——設計文書でこの区別を明示しないと「純述語で書けるはず」という誤った単純化に落ちる。

### 3.5 【注意】attach の ShowSurface 撤去が持つ下流波及

| 波及先 | 現在の挙動 | 撤去後 |
|---|---|---|
| `run_text_scale_phase`（`scale_text.rs:98-109`） | 起動直後から `text_slot_view` が `Some`＝warn は鳴らない | 最初の発話まで `None`＝**scope ごとに 1 回 warn! が鳴る**。これは正常系になるので**水準と文言の是正が要る**（Requirement 8.6 の「毎フレーム出さない」は満たすが、warn は誤解を招く） |
| `reconcile_reported_sizes`（`scale_text.rs:140-205`） | attach 初回表示が積んだ k₀ 補正を drain 末尾で消費（`drain_resnap.rs:50`） | 初回表示が後ろへずれる。**頭脳の show を `reconcile_reported_sizes` より前に置かないと窓寸補正が 1 フレーム遅れる**（Requirement 6.6 の「その期間ずっと表示されていた場合と同一」に効く） |
| `connect_balloon_text`（`frame/attach.rs:375`） | attach 相で 1 回登録 | 初回 show まで登録できない。**cue は登録前でも `TextLayerState` に蓄積される**（`actor.rs:287-288`／`present_frame` の未解決 actor 扱い `:609-624`）ので **1 文字も欠落しない**（Requirement 1.3 後段は満たせる） |
| `refresh_scale`（`refresh.rs:74-80`） | 可視ゆえ DPI 変化で再スケール | 不可視の間は再スケールしない。再表示時に `apply_show` が現 DPI から k を導出（`show.rs:38-43`）＋text_scale 相が binding 再構築 → Requirement 6.6 は成立見込み（❓要檻） |

### 3.6 Requirement 9.6 で更新が要る既存テスト・注記（実測一覧）

| 場所 | 現在の前提 |
|---|---|
| `crates/areka/src/emo2_boot/spine_display_tests.rs:48/:59/:99-110` | 「バルーンは attach 初回表示済み（opaque_count>0）」を積極 assert |
| 同 `:163` / `:289-297` | 同上（`\b` 無し台本でもバルーンは非全透明） |
| `crates/areka/src/emo2_boot/frame_attach_tests.rs:301-302` | 「attach 初回 ShowSurface で相方 scope の文字層スロットが成立する」 |
| `crates/areka/src/emo2_boot/spine_text_scale_tests.rs:13/:43-45` | 「attach 初回表示済みの balloon target は適用 k を持つ」 |
| `crates/areka/src/emo2_boot/spine_test_support.rs:35` | 「実 attach で `text_slot_view` が `Some` になっている本番経路」 |
| `crates/areka/src/emo2_boot/frame.rs:6-8`（モジュール doc） | 「attach: …→バルーン初回 `ShowSurface`（面0）→文字層スロット取得」 |
| `crates/areka/src/emo2_boot/frame/attach.rs:139-154` / `:287-288` / `:325-336` / `:337-338` | 同上（doc とコード両方） |
| `crates/areka/src/emo2_boot/frame/scale_text.rs:63-71` | 「`Hide` は `text_slot_view` を `None` にしない／不可視は本縮退経路に落ちない」——起動直後は落ちるようになる |
| `crates/areka/src/input_events/balloon.rs:315/:472/:780/:804/:821` | 「本番結線まで到達者なし（M1 暫定抑止）」＝**既に陳腐**（`main.rs:363/:731` が本番呼出）。本 spec が hover を消費するなら同時に是正するのが自然 |

---

## 4. 実装アプローチの選択肢

### 4.1 「可視化と分離した表示状態更新」の実現方法（§3.2 の中核・1.3＋6.2 を同時に解く）

| 案 | 内容 | ✅ 解けること | ❌ 懸念 |
|---|---|---|---|
| **A-1: `PresentCommand` に可視性フラグ／新 variant** | `ShowSurface{ .., visible: bool }` あるいは `PrepareSurface{ target, surface_id, binds, pattern }` を追加。`apply_show` の末尾 `set_visible` だけを分岐（`show.rs:211-213`） | 1.3・6.2・1.2 を**構造的に**（フレームを跨いだ可視状態を作らない）。`PresentCommand` は `#[non_exhaustive]`（`command.rs:38`）ゆえ下流 `match` は壊れない | 「実行手段側に新たな表示規則を持ち込まない」（Boundary Context）との線引きが要る。`apply_show` は budget／atom／cage が後続で触る**同居ハンク**（roadmap 干渉台帳 `presenter/show.rs`）＝先着として実形を確定させる責任が生じる |
| **A-2: presenter に非指令 API を足す**（`EmoPresenter::materialize_target(world, target, ...)`） | `apply` の指令面を増やさず、UI 結線層からだけ呼べる口を足す | 同上。指令 API（seriko からの経路）を汚さない | 単一漏斗（`apply_show`）の外に第 2 の表示成立点を作ると `applied`／`native_size`／`last_show` の更新点が二重化する——**内部で `apply_show` を呼んで末尾だけ抑制する形**にしないと危険 |
| **B: 同一フレーム内 show→hide** | attach で従来どおり show し、同じフレームのうちに hide する | 新 API 不要 | Requirement 1.2 が「装着時に一度表示してから即座に消す＝一瞬の点滅も禁止」と**明示的に禁じている**。WUC のコミットが frame 末尾なので実際には見えない可能性が高いが、**要件文言に反する**。6.2 は解けない |
| **C: 表示確立を初回発話まで遅延**（新 API なし） | attach では target 登録のみ（シェルと同型）。頭脳が「最初の可視コンテンツ」を検出した同一フレームで show → `text_slot_view` → `register_actor_view` → text 相が描画 | 1.3 を新 API ゼロで解く。cue は蓄積されるので欠落なし（§3.5） | **6.2 は解けない**（不可視状態での `\b[ID]` は依然 `apply_show` が可視化する）。頭脳が attach 相の責務（`connect_balloon_text`・`balloon_models`）を一部引き取る＝attach 相との責務分割が要る |
| **D: 窓レベル hide**（`WindowPos.hide_window`＝`SWP_HIDEWINDOW`・`wintf/src/ecs/window/window_pos/mod.rs:68/:196`） | バルーン HWND ごと隠す | 文字層も枠も確実に消える（§3.1 も同時解決） | placement の follow／persist／z-order（同ウェーブ `ghost-window-zorder`）と真正面から干渉。van の申し送り（窓の despawn 禁止・可視切替で実装）とも接する。**推奨しない** |

> 実務的には **C（1.3 を解く）＋ A-1 or A-2（6.2 を解く）** の合成が最も素直で、A の適用範囲を「面切替を可視化しない」1 点に絞れる。ただし C 単独でも Requirement 6.2 を「不可視中の `\b[ID]` は頭脳が同一フレームで hide し直す」で近似できる（＝1 フレームだけ可視になり得るので 1.2 の精神には反する）。**設計判断 D2。**

### 4.2 会話終了信号（Requirement 4.1）の入手形

| 案 | 実装 | ✅ | ❌ |
|---|---|---|---|
| **α: 4 番目の broadcast sink（UI 側 horizon 観測）** | `GhostBootOptions.sinks`（`emo2_boot/mod.rs:400-404`）へ `BalloonLifecycleSink` を追加し、`max(cue.at + cue.duration)` を mpsc で UI へ。`MoveCueSink` と完全同型 | **上流の署名を一切変えない**（`spawn_dispatcher` の 3 引数・4 呼出箇所に触れない）。`ClockedTextSink` が既に同じ cue 列を見ている実績 | 正典の「スクリプトの表示が終わってから」＝dola 占有 horizon と一致するが、**中断（Requirement 4.6）を区別できない**。選択肢バリアで配送が止まると horizon が古いまま＝Requirement 5.4 の抑止に依存する |
| **β: `TalkDone` の UI 配線** | `dispatcher.rs:314-332`（`on_done`）に UI 向け送出を additive に足す（`spawn_dispatcher` へ第 4 引数、または `GhostBootOptions` へ観測口） | `reason` 3 値が取れる＝4.6／4.8 を素直に満たす。7.2 の「UI→会話進行の口」と対になる双方向の形が見える | ghost/kanade の署名変更（呼出 4 箇所＋テスト）。「会話進行の管理層の責務・既存の通知先は変えない」の解釈判断が要る |
| **γ: 両方**（α を主・β を将来の予約口として型のみ） | — | 縮退耐性 | 二重の真実源になりうる |

> **設計判断 D4。** なお 4.8（信号が来ない＝表示保持）は α でも β でも「`display_end_at` が `None` のままなら消さない」で同型に書ける。

### 4.3 頭脳（コントローラ）の置き場

| 案 | 位置 | 評価 |
|---|---|---|
| **A（brief 推奨・据え置き妥当）** | `crates/areka/src/emo2_boot/` の新モジュール（例 `balloon_visibility.rs`）＋`emo2_frame_system` に相を 1 つ追加 | emo＝UI 層所有の宣言に整合。`Emo2Wiring` から presenter／runtime／clock に全て届く。**推奨** |
| B（棄却継続） | seriko に頭脳 | 時間源・talk 終了・コンテンツ配置・フォーカスの 4 知識を注入することになる（brief の棄却理由は今も有効） |
| C（棄却継続） | kanade に頭脳 | 表示詳細を運行層が持つ責務違反 |

> **要件ディスカッション（議題 3・2026-08-11）で案 A を再確認。** 開発者から「kanade もしくは kanade が所有する非同期処理が持つべきでは」という提起があり、次の 3 点で C（kanade）を改めて棄却した——⑴判断材料 4 つのうち kanade が持つのは「トーク終了」のみで、可視コンテンツの配置・会話冒頭の全消去・ポインタ／ドラッグの 3 つは全て UI 側にある（kanade へ置くと UI→kanade の逆流配線が 3 系統要る）⑵**kanade の非同期タイマーは壁時計駆動になり Requirement 4.9／9.2（フレーム時刻のみで判定・実時間の待機に依存しない）を満たせない**——UI 側なら注入可能な `FrameTime` で駆動できる ⑶「見える・触れるは emo（UI 層）の窓口」の所有宣言。結論: **判断は UI 層、kanade からは会話終了の信号だけを受け取る**（受け取り方＝D4 は設計）。
>
> なお議題 3 が裁定したのは面切替側（seriko）の帳簿の扱いであって、頭脳の置き場ではない。両者は別問題として扱う。

**相の挿入位置（重要）**: `emo2_frame_system`（`frame.rs:135-165`）の並びで、頭脳は

- `run_drain_phase` の **`presenter.apply` ループ（`drain_resnap.rs:41-45`）の後・`reconcile_reported_sizes`（`:50`）の前**に置けると、`\b` 由来の指令を全部見終えた状態で判断でき、かつ show が積む窓寸 reconcile を**同一フレームで**着地できる（§3.5）。
- `run_text_scale_phase`（`frame.rs:162`）と `run_text_phase`（`:163`）の**上流**である必要がある（binding 組み直しと描画が新しい可視状態の後に走る＝Requirement 3.5「旧内容が見える状態を作らない」）。

→ 現行の 7 相へ「drain の内側（reconcile 直前）」または「drain と move_drain の間」に第 8 相を挿す形が候補。`atom`（`frame/dpi.rs`）とはファイルが別だが、**フェーズ列（`frame.rs:135-163`）は共有面**である（roadmap 干渉台帳 `atom⇄vis` の「順序変更時のみ直列注意」がここに該当）。

### 4.4 タイムアウト既定値（Requirement 4.2）の供給元

- **案 1: `const BALLOON_TIMEOUT_SECS: f64 = 30.0;` を頭脳モジュールに 1 箇所**——最小。単一定義箇所の要件は満たす。
- **案 2: sylphya の点付きプロパティ**——`areka-sylphya` に timeout 語彙は現状ゼロ（実測）。`vocab/dotted.rs` へ語彙追加＋publish 経路の結線が要る。正典の「ベースウェア本体設定の喋りタイムアウト」に将来対応する筋は良いが、M1 の消費者は 1 つだけ。
- **設計判断 D6。**

---

## 5. 工数・リスク

| 区分 | 評価 | 根拠 |
|---|---|---|
| **Effort** | **L（1〜2 週）** | 頭脳の新設は中規模（純関数の状態機械＋frame 相 1 つ）だが、⑴emo-present への能力追加（§4.1）⑵文字層 hide の是正（§3.1）⑶会話終了信号の新配線（§4.2）⑷既存檻 9 箇所の更新（§3.6）⑸Requirement 9.1 が列挙する 8 系統の決定論檻 ⑹実機サインオフ 4 点、と面が広い。単なる「1 行削除＋タイマ」ではない |
| **Risk** | **Medium〜High** | High 側の理由: §3.1（文字層が隠れない）は**現時点で誰も観測していない構造欠陥**であり、実機で初めて出る類。§3.2 は完成済みクレート（emo-present）の単一漏斗に手を入れる。§4.3 の相順は 1 フレームのちらつき／窓寸ずれとして実機でしか見えない。Medium 側の根拠: 判定材料（`visible_glyphs`・`choice_active`・pointer/drag 配線・`FrameTime` 注入）が**全て既設**で、新しい機構の発明はほぼ不要 |

**リスク緩和**:
- §3.1 は**独立した小さな是正**として先に切り出せる（`\b[-1]` の既存経路にも効く）。実機ではなく **`mount.rs` の構造檻**（両 entity の `Visual.is_visible` を assert）で固定できる。
- §4.1 の A 案は `presenter/show.rs` の**先着**になる（後続 budget／atom／cage④ が同ファイル）。roadmap の干渉台帳どおり「先着が実形を確定 → 後続が rebase」の規律に乗る。

---

## 6. Research Needed（設計フェーズへ持ち越す調査項目）

- **R1**: §3.1 の実機確認——`\b[-1]` 実行後にバルーン文字が残ることを実 emo2 で目視／readback で確定させる（本分析は静的構造証跡のみ。記憶「静的構造証跡は実機と同格」に照らせば file:line で再検証可能だが、**是正後の見た目**は実機で見る価値がある）。
- **R2**: `apply_show` の可視化分離が `AlphaMaskResource`／`set_bounds`／`pending_resize` の不変条件を壊さないか（不可視のまま bounds とマスクを更新して良いか）。
- **R3**: ドラッグ抑止の観測形——`DraggingState` のポーリングは多窓時に DragEnd 前に落ちる既知の穴がある（`placement/follow/drag_follow.rs:159-174`）。`OnDrag`／`OnDragEnd` のエッジで頭脳側にフラグを持つ形との比較。
- **R4**: `ClearAll` が UI 側 `TextLayerRuntime` へ届くタイミング（`spawn_emo_text` の UI アクター経由＝`actor.rs:562-580`）と frame 相の相対順序。同一フレーム内で「ClearAll 適用 → 頭脳が全 hide → 新 Text 適用 → 頭脳が show」が起こり得るか（＝1 フレームで消えて出るのは要件上どちらでも良いが、ログの契機種別が二重に出る）。
- **R5**: 選択肢バリアで cue 配送が止まる間の horizon 観測（§4.2 α 案）の挙動——`CuePlayer` のバリア seam（`dola/src/cue/runtime.rs:172` 近傍）と Requirement 5.4 の抑止の重なり。
- **R6**: `\e` 後・`TalkDone` 後に SERIKO ループ（16ms ticker）がバルーン target へ `ShowBalloon` を発行し得るか（`SerikoLoopConfig.balloon_tables`・`emo2_boot/mod.rs:340`）。発行するなら**タイムアウト hide をループが即座に打ち消す**恐れがある。※brief 追補 2 が指摘するとおり balloon 側 `AnimationTable` は現 fixture では構造的に常に空だが、空でない場合の経路を設計で塞ぐ必要がある。
- **R7**: 実機サインオフでの時間短縮手段（Requirement 9.5）——env（`AREKA_` 名前空間）で既定 30 秒を上書きする形が定石（`AREKA_CHOICE_HOVER_INJECT` が先例・`emo2_boot/hover_inject.rs:28`）だが、「本番 env 変数」規律との整合を確認。
- **R8**: `doc/COMPAT_ARCHITECTURE.md` §8 対応表への追記形式（`\![move]` 群の行が書式先例）と、追跡 spec 3 本（`balloon-canon-residue`／`choice-select-events`／`status-execution-states`）の brief 実在確認。

---

## 7. 要件ディスカッションへ渡す設計判断項目

> **要件ディスカッションでの処理区分（2026-08-11）**
>
> | 項目 | 区分 | 処理 |
> |---|---|---|
> | D1 文字層 hide | **解決済（要件へ反映）** | 議題 1 で ⒜ 採用＝`VisualMount::set_visible` を両 entity 対応へ是正（実行手段側・`\b[-1]` も同時是正）。Requirement 1.7／1.8 新設・6.3 に非退行の解釈追記・9.1 へ檻追加・Boundary Context に例外を明文化 |
> | D2 可視化の分離 | **一部解決＋設計フェーズ** | 議題 2 で **B 案（同一フレーム show→hide）を明確に排除**＝Requirement 1.2 の厳格さを維持し、可視状態を経由しないことで構造的に保証する。よって A-1／A-2／C の選択（能力の置き場と形）のみが設計判断として残る。1.3 と 6.2 を 1 つの能力で解くことは要件本文へ明記済み |
> | D3 所有者の一元化 | **解決済（要件へ反映）** | 議題 3 で ⒜ 採用＝可視性の所有者は本仕様の制御ただ一つ、面切替側は面 ID の所有に限る。乖離は常態として明文化。Requirement 6.8 新設 |
> | D4 会話終了信号 α/β/γ | **設計フェーズ** | 要件本文が「入手形は設計で確定」と既に委譲済み |
> | D5 相の挿入位置 | **設計フェーズ** | 純粋に実装構造の判断 |
> | D6 既定値の供給元 | **設計フェーズ** | Requirement 4.2 は「単一定義箇所」しか課していない |
> | D7 タイムアウト起点の粒度 | **解決済（要件へ反映）** | 占有区間の終端（待機を含む）＝正典整合。Requirement 4.1 に明記 |
> | D8 抑止の観測形 | **設計フェーズ** | 既知の穴（`DraggingState` 脱落）を含めて設計で選択 |
> | D9 選択肢抑止の連携口 | **解決済（要件へ反映）** | 既存照会の消費で足りる。Requirement 5.4 を改訂 |
> | D10 ログ水準 | **設計フェーズ** | Requirement 8 は粒度のみ規定 |
> | D11 既存注記の是正範囲 | **解決済（要件へ反映）** | 議題 4 で ⒜ 採用＝`input_events/balloon.rs` の陳腐化注記 5 箇所と `#[allow(dead_code)]` を本仕様で是正。Requirement 9.7 新設（従来の 9.7＝全体テスト緑は 9.8 へ繰り下げ） |
> | D12 SERIKO ループとの相互作用 | **解決済（要件へ反映）** | 議題 3 に統合。ループ由来の表示指令は可視性を変えない規約を M1 の要件に含める＝Requirement 6.9 新設・9.1 へ檻追加。実装形（D2 の能力で塞ぐ）は設計 |
>
> 調査項目 R1〜R8（§6）は全て設計フェーズへ持ち越す。

1. **D1（文字層 hide・§3.1）**: バルーン非表示は「枠 surface のみ」か「枠＋文字層の両方」か。後者を採る場合、是正は `VisualMount::set_visible`（実行手段側・`\b[-1]` にも波及＝挙動改善）か、頭脳側で slot entity を隠すか。要件 6.3「既存 `\b` 回帰テストを緑のまま維持」との関係（既存檻はこの差を観測していない）も併せて裁定。
2. **D2（可視化の分離・§3.2／§4.1）**: Requirement 1.3 と 6.2 が要求する「可視化を伴わない表示状態更新」を、⒜`PresentCommand` へ可視性フラグ／新 variant（A-1）⒝presenter の非指令 API（A-2）⒞表示確立を初回発話まで遅延して 6.2 は別扱い（C）のいずれで実現するか。B（同一フレーム show→hide）は Requirement 1.2 が明示的に禁じている解釈で良いか。
3. **D3（所有者の一元化・§3.3）**: 「バルーン窓の可視性の唯一の所有者は本 spec の頭脳」と宣言し、seriko `ScopeStates.balloon` は面 ID の所有に限る、という境界を明文化してよいか（seriko 側の状態が実表示と乖離することを設計として許容するか）。
4. **D4（会話終了信号・§4.2）**: α（4 番目の broadcast sink による horizon 観測・上流無改変）／β（`TalkDone` の UI 配線・reason 3 値が取れる）／γ（両方）。Requirement 4.6（中断も同一起点）と 4.8（信号欠落時は保持）の充足度が方式で変わる。
5. **D5（相の挿入位置・§4.3）**: 頭脳の相を `emo2_frame_system` のどこに置くか。「drain の `apply` ループ後・`reconcile_reported_sizes` 前」に置くと窓寸補正が同一フレームで着地するが、drain 相の内部構造へ手が入る（`drain_resnap.rs` は `atom` の隣接面）。
6. **D6（既定値の供給元・§4.4）**: 30 秒を頭脳モジュールの定数 1 箇所とするか、sylphya の点付きプロパティ（語彙新設）とするか。正典の「ベースウェア本体設定の喋りタイムアウト」への将来対応をどこまで先取りするか。
7. **D7（タイムアウト起点の粒度）**: Requirement 4.1 の「会話の表示が終了したとき」は ⒜台本の占有 horizon 到達 ⒝最後のグリフがリビールされた時刻（`RevealSchedule.times().last()`）のどちらか。両者は末尾 `\w` の有無でずれる。正典は「スクリプトの表示が終わってから」。
8. **D8（抑止の観測形・R3）**: ドラッグ抑止を `DraggingState` のポーリングで見るか、`OnDrag`／`OnDragEnd` のエッジで頭脳がフラグを持つか。後者は既知の「DragEnd 前に `DraggingState` が落ちる」穴を回避できる。
9. **D9（選択肢抑止の「連携口」・要件 5.4）**: 要件文言は「外部から受け取る連携口を通じて判定」だが、実測では `TextLayerRuntime::choice_active(actor)`（`actor.rs:541`）が**既に読める**。新しい口を作らず既存照会を消費する形で 5.4 を満たしたと見なしてよいか（二重所有を作らない意図には合致する）。
10. **D10（ログ水準・要件 8）**: 実機サインオフのログ grep が契約になる（`apply_show` の `info!`＝`show.rs:258-275` が先例）。表示／非表示の遷移は `info!`、計測の開始・破棄・抑止は `debug!`、といった水準割りをどう置くか。Requirement 8.5「1 行から scope と契機が確定できる」を満たす構造化フィールド名（`scope`／`trigger`／`visible`）の確定。
11. **D11（既存注記の是正範囲・§3.6／要件 9.6）**: 更新対象 9 箇所のうち、`input_events/balloon.rs` の「本番到達者なし」注記と `#[allow(dead_code)]`（既に陳腐）を本 spec の範囲で是正するか、別 spec へ送るか。
12. **D12（SERIKO ループとの相互作用・R6）**: バルーン側 SERIKO ループが `ShowBalloon` を出す経路が将来生きたとき、頭脳のタイムアウト hide を打ち消さないための規約（ループ由来の表示指令は可視性を変えない＝D2 と同じ能力で塞げる）を M1 の時点で設計に含めるか。
