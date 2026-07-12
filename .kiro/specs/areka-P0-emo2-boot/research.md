# ギャップ分析: areka-P0-emo2-boot

> 対象: M-boot 統合ユニット（emo2 起動→OnBoot トークがバルーンに typewriter 進行→close 握手→全エンジン正常終了）
> 調査日: 2026-07-11 ／ 手法: 実シンボル突合（Grep/Glob/Read）。全上流エンジンは完了済み・本ユニットは「アダプタ1個＋結線＋二段観測」に限定。

## 1. 要約（3–5 行）

- **本ユニットに実装の空白は「アダプタ 1 個＋結線」に集約**: seriko の `SurfaceOutput`（本番実装）＝`DisplayCommand`（`Show`/`Hide` に加え、balloon-face-cue 完了で第一級化した `ShowBalloon`/`HideBalloon`）→emo-present `PresentCommand` への変換＋UI スレッド配送アダプタ。**旧ブロッカー（`\b` が cue ドメインに写らない）は `areka-P0-balloon-face-cue`（2026-07-12 完了・main マージ済み）で解消**＝`\b` はもはや no-op 裁定でなく、バルーン表示対象への第一級配送プランビング（同一アダプタへの additive）として in-scope。他は全て完成済み部品の結線（main.rs 差し替え）と二段観測（決定論 spine ＋ env-gate 実走）。
- **最大の本丸は構築順序の再編**: 現 `main.rs` は `boot()` を `WinApp::new()` より前で呼び両 sink `LogSink`。実 sink（`SerikoSink`／`EmoTextSink`）は UI 基盤（`spawn_ui`／`EmoPresenter`／`TextLayerRuntime`）を要するため、順序を「WinApp→UI 部品→実 sink 取得→boot」へ組み替える。
- **装着順序の罠が構造的制約**: `EmoPresenter::text_slot_view` は初回 `ShowSurface`（バルーン枠表示）まで `None`。文字層結線は `attach_target`→初回 `ShowSurface`→`text_slot_view`→`register_actor_view` の順序を厳守する必要がある。
- **新義務は `present_frame` 毎フレーム駆動**: emo-text の `present_frame(runtime, world, talk_time)` を UI フレームスケジュールへ載せる責務が本ユニットにあり、`talk_time` の時刻源決定が未解決。
- **決定論 spine の「実 sink 経路」は既存 ghost spine と別物**: 既存 `spine_e2e_test.rs`（ghost クレート）は `RecordingSink`（TalkCue 記録のみ）。本ユニットは `SerikoSink`→アダプタ→`PresentCommand` 記録・`EmoTextSink`→状態記録の実経路を headless で通す新規テストを要する。

## 2. 現状コードベース調査（既存資産マップ）

### 2.1 差込口（ghost-setup ✅）
- `areka_ghost::boot(GhostBootOptions<S, T>) -> Result<GhostRuntime, GhostBootError>`（`crates/areka-ghost/src/runtime.rs:301`）。境界: `S: SurfaceSink + Clone + Send + 'static`, `T: TextSink + Clone + Send + 'static`。**sink は構築時注入・setter なし**が正本契約（フィールド `surface_sink`／`text_sink`）。
- `GhostRuntime::shutdown(CloseReason) -> Result<(), GhostShutdownError>`（runtime.rs:169）: `ForceQuit`→kanade join→dispatcher Close→join→ticker→shiori→relay×2 join の best-effort 完走。**OnClose 応答の再生完了待ちは kanade の ForceQuit 終了系列内で処理される**（本ユニットは shutdown を呼ぶだけ）。
- `ShioriWiring::Helper { helper_exe }`（本番）／`ShioriWiring::Custom(Box<dyn FnOnce()->Result<Box<dyn ShioriBackend>,String>+Send>)`（spine 注入）。
- `TickerMode::Real(TickerConfig)`（本番）／`Disabled`（決定論・Tick 外部注入）。

### 2.2 現 main.rs（app-shell ✅／window-placement ✅ 結線済み）
`crates/areka/src/main.rs`:
- `main()`（:203）は **`boot()`（:244）を `WinApp::new()`（:266）より前**で呼ぶ。`ghost_boot_options`（:158）は両 sink `LogSink`。boot 失敗は非致命分類（`is_benign_boot_error`:179）。
- `open_startup_window`（:430）は `placement::prepare_ghost_windows`→`spawn_ghost_windows` を `CommandSender` 経由の async タスクで実行（本物ゴースト窓を spawn 済み・:451-465）。clickthrough 登録 system を `FrameFinalize` へ結線（:444）。
- smoke ゲート `AREKA_APP_SMOKE_EXIT_MS`（:496-523）は `despawn_smoke_targets`（:533）で `DummyWindowMarker`＋`GhostWindowMarker` を both カバー済み。
- **ダミー窓フォールバック**（`spawn_dummy_window`:315）は良性失敗時の意図的残置（触らない・R10.7）。
- 依存（`crates/areka/Cargo.toml`）: 通常依存は wintf/shiori-abi/areka-ghost/areka-kanade/areka-parsers/areka-emo-atlas/areka-emo-compose。**areka-seriko／areka-emo-present／areka-emo-text／areka-sakura／areka-actor は通常依存に無い**（emo-present は dev-dependencies のみ）。

### 2.3 surface 側部品（seriko ✅）
- `spawn_seriko(resolver: SurfaceResolver, static_binds: BindSet, out: O) -> (SerikoSink, ActorHandle)` where `O: SurfaceOutput + Send + 'static`（`crates/areka-seriko/src/actor.rs`）。`SerikoSink: SurfaceSink`＝そのまま `surface_sink` に挿せる。
- **空白＝`O`（`SurfaceOutput` の本番実装）**。契約: `SurfaceOutput::send(&mut self, DisplayCommand)`（`output.rs:37`・infallible・FIFO）。`DisplayCommand::Show { scope: ActorKey, surface_id: u32, binds: BindSet }` / `Hide { scope: ActorKey }`（output.rs:20）。現状の実装は観測用 `MockSurfaceOutput` のみ。
- `build_static_bindset`（bind.rs・bindgroup default 由来）／`SurfaceResolver`（resolve.rs）は example `emo-present.rs` の組立経路（parse→bake→EmoWorld→resolver/bindset）で取得可能。

### 2.4 表示側部品（emo-present ✅）
`crates/areka-emo-present/src/presenter.rs`:
- `EmoPresenter`（**`!Send`・PhantomData 強制**・UI スレッド固定）。`new()`／`attach_target(&mut World, TargetId, window: Entity, EmoWorld, AtlasTable)`（:149・skeleton 登録のみ・World 非参照）／`apply(&mut World, PresentCommand)`（:177）／`text_slot_view(TargetId) -> Option<TextSlotView>`（:404）／`read_back`（:419）。
- `PresentCommand::ShowSurface { target, surface_id, binds, reply: Option<ReplySender> }` / `Hide { target, reply }` / `InvalidateCache`（`command.rs:39`・`Send + 'static`・`#[non_exhaustive]`）。`TargetId(pub u32)`（不透明・結線側採番）。
- **装着順序の罠**: `chain`／`mount` は初回 `ShowSurface` で原寸確定後に遅延生成（presenter.rs:256）。`text_slot_view` は `mount`＋`chain` 両方が `Some` になるまで `None`（:404-414・テスト `text_slot_view_is_none_before_display_established` で固定）。
- `build_balloon_target(balloon_dir, decoder) -> Result<(EmoWorld, AtlasTable), _>`（balloon.rs）。
- apply は WucGraphicsResource/GraphicsCore が World に無いと Device エラー（供給面遅延生成）。GPU 資源は wintf が窓生成後に遅延挿入（example の `boot_present_system` は資源到達を待つ）。

### 2.5 text 側部品（emo-text-layer ✅・2026-07-11 完了）
`crates/areka-emo-text/src/{sink,actor}.rs`:
- `EmoTextSink`（`Clone`・`impl TextSink`・`sink.rs:41`）＝そのまま `text_sink` に挿せる（`emo_text_sink_satisfies_injection_contract` で契約固定）。`emit` は `UiSender<TextMsg>` へ非ブロック送出（worker→UI drain）。`close()` で `TextMsg::Close`。
- `spawn_emo_text(runtime: Rc<RefCell<TextLayerRuntime>>) -> Result<(EmoTextSink, JoinHandle<()>), UiSpawnError>`（actor.rs:254・**UI／pump スレッドから呼ぶ**）。
- `TextLayerRuntime::register_actor_view(actor: ActorKey, view: &TextSlotView, model: &BalloonModel)`（actor.rs:195）＝文字層装着口（`TextSlotView` から binding＋layout 入力を導出）。**`text_slot_view` が None の間の cue は蓄積され、登録後の次フレームで装着・描画される**。
- `present_frame(runtime: &mut TextLayerRuntime, world: &mut World, talk_time: f64) -> Result<(), TextLayerError>`（actor.rs:288）＝**毎フレーム UI 駆動。「example/emo2-boot が駆動」と rustdoc 明記＝本ユニット所有**。`talk_time` は「talk 起点相対秒・注入・sleep 不使用」。未解決 actor は skip＋再試行、装着は初回のみ、以降 Present 完結。
- `TextLayerRuntime::new(TextLayerConfig)`／`apply_cue`（純粋状態・World 非参照）。

### 2.6 窓側部品（window-placement ✅・2026-07-11 完了）
`crates/areka/src/placement/spawn.rs`:
- `GhostWindows`（Resource・戻り値両方で公開）: `char_window(scope) -> Option<Entity>` / `balloon_window(scope) -> Option<Entity>` / `scopes()`（:118-133）。**キャラ窓＋バルーン窓同梱 ✅**（契約どおり）。
- `prepare_ghost_windows(ghost_root, balloon_root) -> Result<PreparedPlacement, PlacementError>`→`spawn_ghost_windows(world, &placements, &titles) -> GhostWindows`（:149）。main.rs は既に呼んでいる（本物窓 spawn 済み）。placement は `EmoPresenter` を import しない設計境界＝**装着は本ユニット領分**。

### 2.7 shell 側構築入力（全 ✅・donor コードあり）
`crates/areka/examples/emo-present.rs`（dev-dep で emo-present を引く観測 example）が組立の実績コード（donor）:
- `MountModel`→`areka_parsers::shell::parse`→`EmoWorld::build`＋`bake`（emo-atlas）→`SurfaceResolver`／`build_static_bindset`。
- `boot_present_system`（:591）＝GPU 資源到達フレームで `attach_target`→初回 `apply(ShowSurface surface_id=0)` を各 target 高々 1 回（**装着順序と資源待ちの実装パターン**）。
- `create_shell_window`／`create_balloon_window`（WS_POPUP 透過・surface 原寸を WindowPos.size へ直接・DPI 表示契約）。
- `register_click_through_windows`（`Added<WindowHandle>` で厳密 1 回）。

### 2.8 決定論 spine の既存土台（ghost ✅）
`crates/areka-ghost/tests/ghost/spine_e2e_test.rs`: `ScriptedShioriBackend`（`ShioriBackend` 台本 fake・純 x64・プロセス spawn/i686 不要）＋`RecordingSink`（`Clone`・`SurfaceSink`+`TextSink`）。S1–S6（boot 成功・接続失敗・helper 死活・close 握手 等）を Tick 注入のみ・sleep 不使用で駆動する実績。**ただし観測は `RecordingSink`（TalkCue 記録）で、実 sink 経路（アダプタ／emo-present／emo-text）は通していない**。

### 2.9 cue 契約と `\b`（balloon-face-cue 完了で第一級化・2026-07-12）
`crates/areka-sakura/src/contract.rs`: `TalkCue { at: f64, actor: ActorKey, command: CueCommand }`。`cue_target_of`（contract.rs:67）: `Emote`/`EntityRef`/**`BalloonSurface`**→`Shell`、`Text`/`NewLine`/`Clear`/`Choice`→`Balloon`、`Custom`→`None`。
- **`CueCommand::BalloonSurface { key: String }` は実在する**（`crates/dola/src/cue/command.rs:144`）。`\b` は parser `Instruction::BalloonSurface(SurfaceArg)`（`crates/areka-parsers/src/sakura/model.rs:38`・ブラケット `\b[ID]`＋裸形 `\bN`・本文数字漏れ修正済み）→ sakura compile（`crates/areka-sakura/src/compile.rs:61`）で `CueCommand::BalloonSurface { key }` へ写る（catch-all 破棄されない・固定テストあり）。
- **`cue_target_of(BalloonSurface) → CueTarget::Shell`**（contract.rs:71）＝seriko（SurfaceSink）へ配送。文字状態機械（`CueTarget::Balloon`＝TextSink/emo-text）へは流さない（誤配線でない・`crates/areka-emo-text/src/state.rs:199` でも表示系として明示 skip）。
- seriko は `resolve_balloon_key`（`crates/areka-seriko/src/resolve.rs:103`・**数値のみ・alias 非適用**・`Show(id)`/`Hide`/`NameForm`/`Invalid`）→ `apply_balloon`（`crates/areka-seriko/src/state.rs:149`・シェル `scopes` と独立した per-scope `balloon` map〔state.rs:54〕）→ **新設表示指令** `DisplayCommand::ShowBalloon { scope: ActorKey, surface_id: u32 }`／`HideBalloon { scope: ActorKey }`（`crates/areka-seriko/src/output.rs:39,41`・early branch は actor.rs:193）を発行する。**`ShowBalloon` は `binds` を持たない**——output.rs:19/38 の doc が「adapter は `PresentCommand::ShowSurface{binds: BindSet::default()}` を組む」と明記＝配送は本ユニット領分。
- **本ユニットに残る空白は cue 語彙ではなく配送**: `DisplayCommand::ShowBalloon`/`HideBalloon` → バルーン表示対象への `PresentCommand::ShowSurface{binds 既定}`／hide への変換と、scope→バルーン TargetId 採番（§7 #4 の scope→TargetId 写像の一部）。バルーン面キーの name 形・alias 解決は out of scope（将来増分）。

## 3. 要件→資産マップ（Missing / Unknown / Constraint）

| 要件 | 対応資産 | ギャップ種別 |
|---|---|---|
| R1 一発起動・実サーフェス可視化 | main.rs（窓 spawn 済）＋`attach_target`＋初回 `ShowSurface` | **Missing**（装着結線）|
| R2 OnBoot トーク→typewriter | boot→kanade→sakura→sink 経路 ✅＋`present_frame` 駆動 | **Missing**（present_frame 結線・talk_time 源=Unknown）|
| R3 表示指令変換・配送（scope→target 写像 正本・シェル＋バルーン両対象）| `DisplayCommand`（`Show`/`Hide`/`ShowBalloon`/`HideBalloon`）／`PresentCommand`／`UiSender`or`CommandSender` | **Missing**（アダプタ本体＝本ユニット唯一の新規正本・シェル target＋バルーン target 両配送）|
| R4 文字層装着順序 | `text_slot_view`（遅延 None）＋`register_actor_view` | **Constraint**（初回 ShowSurface→text_slot_view→register の順序厳守）|
| R5 `\b` バルーン面切替 cue の配送 | seriko `ShowBalloon{scope,surface_id}`/`HideBalloon{scope}`（output.rs:39,41）→ balloon target への `PresentCommand::ShowSurface{binds 既定}`／hide | **Missing**（同一アダプタへの additive 配送・数値 key はそのまま消費し alias 非再適用）|
| R6 終了握手・全エンジン正常終了 | `shutdown(CloseReason)`＋kanade close 握手 ✅／smoke ゲート ✅ | 結線のみ（既存 shutdown 呼び足す）|
| R7 構築順序再編・非致命 boot | 現 main.rs（boot が WinApp 前）| **Missing**（順序組み替えが本丸）|
| R8 決定論 spine（実 sink 経路）| `ScriptedShioriBackend`＋アダプタ＋emo-present/text headless | **Missing**（新規統合テスト・GPU 有無=Unknown）|
| R9 実 pasta env-gate 実走 | helper 経路 ✅／smoke ゲート ✅ | 手順整備（DoD 前提にしない）|
| R10 変更境界・非改変 | 各エンジン完成・非 Send/UI スレッド固定 | **Constraint**（tokio 禁止・新規外部依存なし・UI 配送規律）|

## 4. 実装アプローチ選択肢

本ユニットの設計自由度は 3 つの軸に集約される: (i) アダプタ／結線コードの**置き場**、(ii) worker→UI スレッドの**配送経路**、(iii) **present_frame 駆動と talk_time 源**。

### 軸 (i): アダプタ・結線の置き場

#### Option A: areka バイナリ crate 内モジュール（main.rs ＋ 新 module）
- アダプタ（`SurfaceOutput` 実装）と結線を `crates/areka/src/` 配下の新 module（例 `boot_wiring.rs`／`adapter.rs`）に置き、areka の通常依存へ areka-seriko/-emo-present/-emo-text/-sakura/-actor を昇格。
- ✅ 既存 placement/shiori モジュール群と同じ構造・main.rs から直接結線・example emo-present.rs の donor パターンをそのまま移植可能。
- ✅ bin crate でも `#[cfg(test)]` で headless 単体テスト・`tests/` で統合テスト可能（既存 smoke/seam テストの前例）。
- ❌ アダプタの純粋部（`DisplayCommand`→`PresentCommand` 写像・scope→TargetId）を他 crate から再利用したい将来（M-dual）に bin crate 内だと参照不可。

#### Option B: 新規ライブラリ crate（例 `areka-emo2-boot`）
- アダプタ＋結線ヘルパを lib crate 化し、areka bin は薄く呼ぶだけ。
- ✅ scope→TargetId 写像を「本ユニットが立てる唯一の正本」として crate 境界で明示・M-dual が将来消費（brief のクロスユニット契約と整合）・単体テストが自然。
- ✅ emo 系 crate 群（emo-atlas/compose/present/text）と同じ「1 責務 1 crate」慣行に沿う。
- ❌ crate 追加のオーバーヘッド（Cargo.toml・workspace 登録）。ただし brief「新規機構を作らない」は**機構**（フレームワーク化）禁止であり crate 分割自体は禁じていない。

#### Option C: ハイブリッド（純粋写像は小 lib、UI 結線は main）
- scope→TargetId 写像＋`DisplayCommand`→`PresentCommand` 変換（純粋・World/COM 非依存）を最小 lib（or seriko/emo-present 隣接）に置き、UI スレッド配送・窓装着・順序再編は main.rs に置く。
- ✅ 純粋部を決定論テストで檻化しつつ、UI 依存部は薄い結線に留める（layer 分離）。
- ❌ 2 箇所に分かれ追跡性やや低下。

### 軸 (ii): worker→UI 配送経路（アダプタ→`EmoPresenter::apply`）

`EmoPresenter::apply(&mut World, PresentCommand)` は **UI スレッド＋`&mut World`＋NonSend presenter** を要する。アダプタは seriko の worker スレッド上で `DisplayCommand` を受ける。橋渡し候補:

- **Option ii-1: `CommandSender` クロージャ経路**（既存 `open_startup_window` と同型）。アダプタが `PresentCommand`（Send）へ変換し `tx.send(Box<dyn FnOnce(&mut World)+Send>)` で「presenter を World から取り出して apply」するクロージャを投函。Input schedule で適用。✅ 既存前例・追加機構ゼロ。❌ presenter を毎回 World から remove/insert する churn（example のパターン）。
- **Option ii-2: `UiSender`＋受信 Resource キュー＋毎フレーム system**。アダプタが `UiSender<PresentCommand>` で送り、UI ドレインが `Vec<PresentCommand>`（NonSend）へ push、`FrameFinalize` の drain system が presenter.apply を回す。✅ emo-text の `spawn_ui` 規約と統一・presenter churn を 1 フレーム 1 回に集約。❌ キュー＋system の追加結線。
- **Option ii-3: 専用 UI ドレイン（`spawn_ui`）で presenter を Rc 共有**。emo-text と同様に presenter を `Rc<RefCell<>>` 化しドレイン handler が apply。❌ `apply` は `&mut World` を要し spawn_ui handler は World を持たない（emo-text の apply_cue は World 非依存ゆえ成立していた）＝presenter には**不成立**。World アクセスが要る以上 ii-1/ii-2 のいずれかが必要。

→ **ii-1 と ii-2 が現実的候補**。設計で 1 択に確定させる（記憶 areka-concurrency-model「UI 配送経路・Arc 手渡し・フレームワーク化禁止」と整合させること）。

### 軸 (iii): present_frame 駆動と talk_time 源
- 駆動は `FrameFinalize`（or 相当）の毎フレーム system で `present_frame(&mut runtime, &mut world, talk_time)` を呼ぶ（emo-text は Rc 共有ゆえ system 内で borrow）。emo-present の `boot_present_system`/`cycle_present_system` と同じ排他 system パターン。
- **talk_time 源（Unknown・設計判断）**: `present_frame` は「talk 起点相対秒」を期待する。候補: (a) `FrameTime`（wintf・絶対秒）をそのまま渡す＝talk 起点相対ではないが typewriter 進行の単調性は満たす、(b) talk 開始時刻を記録して `FrameTime - talk_start` を渡す（true 相対）、(c) sakura/kanade 側の注入時刻に同期。emo-text のリビール進行は `char_wait` 相対なので絶対時刻でも見た目は動くが、talk 切替時のリセット意味論が (a) と (b) で異なる。設計で確定。

### 4.x 構築順序再編（全 Option 共通・R7 の本丸）
現 `main()`: `boot()`→`WinApp::new()`→`open_startup_window`→`run()`→`shutdown()`。
目標: `WinApp::new()`→（UI 部品構築: `EmoPresenter::new`・`TextLayerRuntime`＋`spawn_emo_text`・窓装着・アダプタ用 UI 経路）→（shell 組立: mount→parse→bake→EmoWorld→resolver/bindset）→`spawn_seriko(out=adapter)` で `SerikoSink` 取得→`boot(surface_sink=SerikoSink, text_sink=EmoTextSink)`→`run()`→`shutdown()`。非致命 boot（`is_benign_boot_error`）の意味論は維持。窓装着（attach_target→初回 ShowSurface→text_slot_view→register_actor_view）は GPU 資源到達フレームで駆動する必要があり、donor `boot_present_system` の資源待ちパターンを踏襲する。

## 5. 工数・リスク

- **工数: L（1–2 週間）**。理由: 新規コードは薄い（アダプタ 1 個＋結線）が、(i) 構築順序の全面組み替え、(ii) 装着順序の罠（初回 ShowSurface→text_slot_view→register の 3 段が GPU 資源到達に依存する非同期タイミング）、(iii) 実 sink 経路の決定論 spine 新設、(iv) present_frame 駆動と talk_time の確定、と結線点が多く、UI スレッド／NonSend／`&mut World` の借用規律に沿った統合が必要。
- **リスク: 中〜高**。
  - 高: 装着順序×GPU 資源遅延挿入×文字層 None 遅延取得の**タイミング合流**（初回 ShowSurface が済むフレームで text_slot_view を取り register_actor_view する結線を取りこぼすと文字が出ない）。donor に近い前例はあるが text 層の合流は新規。
  - ~~中: 決定論 spine の観測境界~~ **✅ 決着（2026-07-12 議題2・案 B）**: 本プロジェクトは**外部 CI を持たない**（`.github` 不在）＝「常設」は `cargo test --workspace`（ローカル DoD ゲート）。emo-present/emo-text の GPU 実描画＋readback は**既に素の `#[test]` で走る定石**（`draw_readback_test.rs`＝env-gate/ignore 無し・テスト内 `CoInitializeEx(COINIT_MULTITHREADED)`＋`GraphicsCore::new()`（WARP 可）＋実 `present_frame`＋`read_back`・注入時刻・sleep 不使用）。ゆえに観測境界は**アダプタ出力どまりでなく実描画→readback まで**（R8.2/8.4/8.5）。GPU/WARP/MTA は脆さコストでなく既存パターンの踏襲。残る非決定要因（seriko worker スレッド合流）は ghost spine（`ScriptedShioriBackend`＋join 単一同期点・sleep 不使用）の既存手法で決定論化（設計 §7 #5 と合流）。
  - 中: worker→UI 配送のスレッド安全性（`PresentCommand` は Send、presenter は NonSend・World 経由取得）。ii-1/ii-2 の選択で借用衝突の出方が変わる。
  - 低: バルーン面配送（`ShowBalloon`/`HideBalloon` → balloon target の `PresentCommand::ShowSurface{binds 既定}`／hide）はシェル写像への素直な additive（同一アダプタ）・smoke ゲート・shutdown 呼び出しは既存資産の素直な結線。

## 6. 設計フェーズへの申し送り（Research Needed）

1. **配送経路の 1 択確定**（軸 ii: CommandSender クロージャ vs UiSender+キュー Resource+system）。presenter の remove/insert churn と借用規律・記憶 concurrency-model との整合を基準に。
2. **talk_time 時刻源**（軸 iii: FrameTime 絶対 vs talk 起点相対 vs kanade 注入同期）と、talk 切替時のリセット意味論。
3. **決定論 spine の観測境界**: (a) アダプタの `PresentCommand` 出力＋emo-text 状態記録まで（GPU 不要・純決定論）で足りるか、(b) emo-present.apply/present_frame の実装描画（GPU/WARP・MTA）まで通すか。R8.3「headless の記録で観測」「i686 非依存」を満たす線引き。
4. **scope→TargetId 写像の正本形**: `ActorKey("0")`→shell TargetId、balloon TargetId の採番規約（shell=scope 別 target・balloon=独立 target）。二人立ち M-dual が将来消費する拡張シーム。
5. **窓装着タイミングの結線パターン**: donor `boot_present_system`（GPU 資源到達待ち・各 target 高々 1 回）を踏襲しつつ、balloon target で「初回 ShowSurface→text_slot_view→register_actor_view→present_frame 開始」の合流をどの schedule/順序で保証するか。
6. **`\b[ID]` の落ち先は確認済み（KNOWN）**: balloon-face-cue 完了により parser `Instruction::BalloonSurface`→ dola `CueCommand::BalloonSurface`→ sakura compile→ `cue_target_of → Shell`→ seriko `resolve_balloon_key`/`apply_balloon`→ `DisplayCommand::ShowBalloon`/`HideBalloon`（output.rs:39,41）まで第一級化。残る確認は「アダプタが `ShowBalloon`/`HideBalloon` を balloon target の `PresentCommand::ShowSurface{binds 既定}`／hide へ配送する結線」のみ（cue 語彙・no-op 裁定は消滅）。
7. **依存境界の解釈確定**: R10.5「新規依存を追加せず」は**外部（third-party）crate 追加禁止**（tokio 禁止等）と解し、areka-seriko/-emo-present/-emo-text/-sakura/-actor の**ワークスペース path 依存を areka へ昇格するのは本ユニットの結線に必須で in-scope**、という前提でよいか（brief「全部品は完成済み・結線に徹する」と整合）。
8. **アダプタ／結線の置き場**（軸 i: main 内 module vs 新 lib crate vs ハイブリッド）。M-dual への正本再利用性と bin crate テスト容易性のトレードオフ。

## 7. 設計判断項目（要件ディスカッションへ送る・番号付き）

1. 配送経路: `CommandSender` クロージャ経路（ii-1）か、`UiSender`＋キュー Resource＋毎フレーム drain system（ii-2）か。
2. `present_frame` の `talk_time` 時刻源（絶対 FrameTime／talk 起点相対／kanade 同期）とリセット意味論。
3. ~~決定論 spine の観測境界（アダプタ出力記録どまり／emo-present・emo-text 実描画まで）と GPU 依存の許容範囲。~~ **✅ 決着（2026-07-12 議題2・案 B）**: 外部 CI 不在ゆえ「常設」＝`cargo test --workspace`。GPU 実描画＋readback は既存 `draw_readback_test`（素の `#[test]`・WARP・MTA・注入時刻）で定石化済み。**観測境界は実描画→readback まで**（アダプタ出力どまりにしない）。要件へ反映済み（R8 を「CI 常設」→「`cargo test --workspace` 常設」へ改稿・R8.2/8.4/8.5 で実描画→readback・WARP/MTA・ピクセル述語を明記）。GPU headless（WARP・オフスクリーン readback）は「実表示非依存」を満たす。
4. scope→TargetId 写像の採番規約（本ユニットが立てる唯一の正本・M-dual 拡張シーム）。
5. 装着合流の schedule 設計（GPU 資源待ち→初回 ShowSurface→text_slot_view→register_actor_view→present_frame 開始の順序保証）。
6. バルーン面切替指令のバルーン target 配送: `DisplayCommand::ShowBalloon`/`HideBalloon` → `PresentCommand::ShowSurface{binds 既定}`／hide の写像を置く場所（アダプタ）と、バルーン TargetId の採番（#4 の scope→TargetId 写像との関係・shell target と独立にバルーン target を割り当てる規約）。
7. アダプタ・結線の置き場（areka 内 module／新 lib crate／ハイブリッド）。
8. ~~依存追加の解釈（外部 crate 禁止＝ワークスペース path 依存昇格は in-scope の確認）。~~ **✅ 決着（2026-07-12 議題1・案 A）**: R10.5「新規依存」＝**外部（crates.io）crate の追加禁止**（tokio 等）と解し、既存 workspace crate（areka-seriko/-emo-present/-emo-text/-sakura/-actor 等）の path 依存を `areka` bin へ昇格するのは統合結線として **in-scope**。要件へ反映済み（R10.5 を「外部依存」へ明確化＋R10.8 新設で昇格許可を明記）。**crate 分割そのもの（案 C＝置き場問題）は §7 #7 の設計判断として別途 design 持ち**（本決着は依存解釈のみ確定）。
9. 構築順序再編の具体形（WinApp→UI 部品→shell 組立→spawn_seriko→boot）と非致命 boot 意味論の維持方法。

## 8. ✅ ブロッカー解消（2026-07-11 議題1 で登記 → 2026-07-12 balloon-face-cue 完了で RESOLVED）

**旧ブロッカーは解消済み（RESOLVED）。以下は歴史的記録＋再開義務の消化状況**。

- **当時の検出（歴史的記録）**: R5（`\b` cue の M-boot 裁定）は前提破綻していた——`\b[ID]` は parser（当時タグ表なし→`Raw` 落ち）→sakura compile（catch-all debug! 破棄）の二段で cue 化されず、`CueCommand` にバルーン面 variant も不在＝統合層に一切届かなかった。R5.1/5.2「受信→no-op＋warn」は検証不能な空手形だった。追加検出: 旧形式 `\bN` の本文数字漏れ（可視破損）／Balloon 分類 cue の TextSink 誤配線。
- **裁定と結末**: no-op 絆創膏でなく cue ドメイン第一級化（`\s` 完全対称）を新設 spec `areka-P0-balloon-face-cue` で先行完遂。**同 spec は 2026-07-12 に完了・main へマージ済み**（`.kiro/specs/completed/areka-P0-balloon-face-cue`）。確認済み実シンボル: `Instruction::BalloonSurface`（parser・model.rs:38）／`CueCommand::BalloonSurface`（dola・command.rs:144）／`cue_target_of → Shell`（sakura・contract.rs:71）／`resolve_balloon_key`＋`apply_balloon`（seriko・resolve.rs:103／state.rs:149）／`DisplayCommand::ShowBalloon`/`HideBalloon`（seriko・output.rs:39,41）。**配送（DisplayCommand → balloon target の PresentCommand・scope→balloon TargetId 採番）は同 spec が意図的に本 spec（emo2-boot）へ残した**（output.rs:19/38 doc「adapter は `PresentCommand::ShowSurface{binds: BindSet::default()}` を組む」）。
- **再開義務の消化状況**:
  - ✅ **義務①（R5 改稿）DONE**: 本回リフレッシュで R5 を「no-op＋warn 空手形」から「実 cue（`ShowBalloon`/`HideBalloon`）が届く→バルーン target へ配送」へ改稿（受入基準 5.1〜5.5・`\b` 含むスクリプトで決定論 spine 観測・OnBoot デモは `\b` 不使用）。
  - ✅ **義務③（R3 拡張）DONE**: R3 の scope→表示対象 写像を、シェル target（Show/Hide）に加えバルーン target（ShowBalloon/HideBalloon）配送まで拡張（受入基準 3.1〜3.7）。§7 設計判断 #6 は「no-op 適用点」から「バルーン target 配送＋バルーン TargetId 採番」へ差し替え済み。
  - ✅ **義務②（要件ディスカッション残議題）DONE（2026-07-12）**: 再開した要件ディスカッションで2議題とも決着。**議題1（旧 議題2）= R10.5 依存解釈 → 案 A**: 「新規依存」＝外部（crates.io）依存の追加禁止と解し、既存 workspace crate の path 依存昇格は in-scope（R10.5 明確化＋R10.8 新設・§7 #8 決着）。**議題2（旧 議題3）= R8 観測境界 → 案 B**: 外部 CI 不在＝「常設」は `cargo test --workspace`。GPU 実描画＋readback は既存 `draw_readback_test`（素の `#[test]`・WARP・MTA・注入時刻）で定石化済みゆえ、観測境界は**実描画→readback まで**（アダプタ出力どまりにしない）。R8 を6基準へ改稿（実描画→readback・WARP/MTA・ピクセル述語明記）＋§7 #3・§5 リスク決着。
- **要件フェーズ完了**: 再開義務①②③すべて消化。要件ディスカッションは全議題決着で締結、design フェーズへ引き継ぐ（残る §7 判断項目 #1/#2/#4/#5/#6/#7/#9 は design 持ち＝要件ではなく設計判断）。

## 9. 設計フェーズ Discovery & Decisions（2026-07-12・design 生成時）

> Discovery Scope: **Extension / Complex Integration**（統合結線・integration-focused discovery）。手法: 実シンボル突合（並行リサーチ 3 系統: UI 配送機構／main・placement 結線面／エンジン公開契約）＋設計判断の確定。§7 の残判断 #1/#2/#4/#5/#6/#7/#9 を本節で全て決着した。

### 9.1 追加調査ログ（design 決定の根拠となった実シンボル）

- **`CommandSender` 経路の制約（軸 ii-1 の棄却根拠）**: `CommandSender = std::sync::mpsc::Sender<Box<dyn FnOnce(&mut World)+Send>>`（`crates/wintf/src/ecs/widget/bitmap_source/task_pool.rs:13-16`）は `WintfTaskPool::spawn(f: FnOnce(CommandSender)->Fut)`（同 :54）経由でしか取得できず、**素の sender 公開アクセサが無い**（`send_command` は `#[cfg(test)]`）。seriko worker 上のアダプタへ構築時に渡すには wintf へ公開口を増設する改変（R10.3 抵触）か async ブートストラップの迂回が要る。drain は `drain_task_pool_commands`（`systems.rs:279`・Input schedule・排他 system）。
- **`spawn_ui`/`UiSender` の制約（軸 ii-3 再確認）**: `areka_actor::spawn_ui` の handler は `M` を値で受け **`&mut World` を持たない**（`crates/areka-actor/src/ui.rs:82-126`）。`EmoPresenter::apply(&mut World, ..)` には不成立（ギャップ分析 ii-3 の確認）。emo-text は「drain=純粋状態のみ／World 作業は `present_frame` の毎フレーム別口」という分離で成立している。
- **presenter 保持の donor パターン**: `EmoPresenter`（`!Send`）は NonSend resource に包み、**排他 system 内で remove→駆動→insert**（`crates/areka/examples/emo-present.rs` `EmoBoot`＋`boot_present_system` L591-679・`FrameFinalize` 登録・GPU 資源 `GraphicsCore`＋`WucGraphicsResource::is_valid()` 到達ゲート・`Option::take` で各 target 高々 1 回）。
- **`FrameTime`**: `#[derive(Resource)] pub struct FrameTime(pub f64)`（`crates/wintf/src/ecs/graphics/core.rs:147`）。**f64 秒・OS boot 起点・`dola::runtime::clock::now()`（QPC・`crates/dola/src/runtime/clock.rs:14` pub）から毎フレーム更新**＝TalkClock と同一クロック源に揃えられる。
- **talk 時刻系の全貌（軸 iii の根拠）**: ghost dispatcher は talk ごとに「初回 Tick を `base_now` として `(now-base)/1000.0` 秒を `SakuraMsg::Tick(f64)` へ中継」（`crates/areka-ghost/src/dispatcher.rs:137-157`）。sakura `TalkDriver::on_tick` は `TimedSchedule(0 起点)` を進め **due になった `TalkCue` をその時点で sink へ emit**（`crates/areka-sakura/src/drive.rs:175-`）。ゆえに **cue は「talk 起点相対の due 時刻 ≒ 実到着時刻」で届き、`cue.at` と到着壁時計の差から talk epoch を推定できる**（量子化誤差 ≤ ticker base_interval 50ms）。emo-text 側は epoch を持たず `talk_time` は常に注入（`state.rs`: リビール時刻 r_i は talk 起点相対・`Clear` が唯一のリセット）。
- **bindgroup default の出所**: 既定オン集合はランタイム導出 API が存在せず、**shell `descript.txt` の `sakura.bindgroup{N}.default,1`** が正典の源（emo2 fixture 実測: 1100/1207/1302/1500/1800 = seriko テストのハードコード値と一致・`crates/pilot/examples/shiori-host-32/fixtures/emo2/shell/master/descript.txt:22,34,44,57,…`）。KV 読取は placement `source.rs` の `load_descript_source` と同じ parsers 経路で可能。
- **`AtlasTable` は `Clone`（Arc 共有・安価）**（`crates/areka-emo-atlas/src/table.rs:94`）・**`EmoWorld` は非 Clone**（target ごとに `EmoWorld::build`＋`bind_atlas` を再実行する＝parse/bake は 1 回で共有可）。
- **窓 close の観測機構**: wintf に close イベント購読は無く、`WM_CLOSE`→entity despawn→`reconcile_window_registry` 空遷移→shutdown Event→`run()` 復帰、が唯一の funnel（`crates/wintf/src/ecs/window_proc/lifecycle.rs:62-73`／`runtime/window_registry.rs:120-139`）。ghost shutdown は run() 復帰後に main が呼ぶ現行構造を維持する。
- **balloon 入力**: `build_balloon_target(balloon_dir, &impl ElementDecoder)`（`crates/areka-emo-present/src/balloon.rs:120`）は `balloons{N}.png` のみ列挙。emo2 バルーン dir は `fixtures/emo2/emo2-kakukaku`（`balloons0.png` のみ＝面 id 0）。`BalloonModel` は `areka_parsers::balloon::parse` 経路から構築。`sakura.balloon.defaultsurface` の消費実装は現存しない（既定面 0 固定が現状唯一の経路）。
- **areka Cargo.toml 現況**: areka-seriko／-emo-present（dev のみ）／-emo-text／-sakura／-actor／dola は通常依存に無い → R10.8 の path 依存昇格対象。

### 9.2 Design Decisions（§7 残判断の決着・design.md へ転記済み）

- **DD-1（§7 #1 配送経路）= 専用 `std::sync::mpsc<PresentCommand>` チャネル＋`FrameFinalize` 単一排他 system の drain**。ii-1（CommandSender）は sender 公開口が無く wintf 改変が要るため棄却、ii-2 の UiSender+キュー Resource 変形は spawn_ui actor＋Resource＋system の 3 部品になるため簡素化で棄却。アダプタ（seriko worker）は `DisplayCommand`→`PresentCommand`（Send）純変換＋非ブロック send のみ。UI 側は毎フレームの排他 system が受信端（NonSend 保持・std Receiver は !Sync）を drain し presenter.apply する。FIFO は単一チャネルで保存・装着完了までは drain しない（チャネルが保留バッファを兼ねる＝早着指令を失わない）。
- **DD-2（§7 #2 talk_time）= TalkClock（到着観測 epoch 推定・単調 max・クロック注入可）**。`ClockedTextSink<T: TextSink>` が emit ごとに `epoch = max(epoch, clock() − cue.at)` を共有 slot（`Arc<Mutex<Option<f64>>>`）へ記録して内側へ転送。毎フレーム `talk_time = FrameTime.0 − epoch`（clamp ≥0・epoch None は present_frame skip）。クロックは `Arc<dyn Fn()->f64 + Send + Sync>` 注入（既定 `dola::runtime::clock::now` = FrameTime と同源）。新 talk は due 時刻配信ゆえ epoch が前方へ跳んで自動リベース（dispatcher per-talk base_now が根拠）。単調 max 方策で talk 内ジッタ（≤50ms）の逆行を抑制。FrameTime 絶対値の直渡しは epoch 不一致で typewriter が即時全表示になるため棄却。kanade 同期はエンジン改変（R10.3）ゆえ棄却。
- **DD-3（§7 #4 scope→TargetId 写像＝本仕様唯一の新正本）**: `shell_target(scope) = TargetId(2*scope)`／`balloon_target(scope) = TargetId(2*scope+1)`。scope は `ActorKey` の数値 parse（`"0"`→0）。非数値 ActorKey は warn!＋当該指令 drop（log-first）。M-dual はこの純関数だけを拡張する（結線側採番という emo-present の契約に整合・donor の TargetId(0)/(1) は scope0 の特殊形として包含）。
- **DD-4（§7 #5 装着合流）= 単一排他 system `emo2_frame_system`（FrameFinalize）3 フェーズ**: ①attach フェーズ（`GhostWindows` Resource＋GPU 資源到達ゲート・`Option::take` で高々 1 回・scope ごとに shell target attach→初回 ShowSurface(初期面・static binds)、balloon target attach→初回 ShowSurface(面 0・binds 既定)→`text_slot_view`→`register_actor_view`。apply は同期ゆえ **同一フレーム内で text_slot_view が Some になる**＝合流タイミング問題を構造的に消す。万一 None なら接続せず次フレーム再試行=R4.2）②drain フェーズ（attach 完了後のみ mpsc を drain→presenter.apply）③text フェーズ（TalkClock→talk_time→`present_frame`）。donor `boot_present_system` の資源待ち・高々 1 回パターンの直系。
- **DD-5（§7 #6 バルーン配送）**: アダプタ純関数 `map_display_command` 内で `ShowBalloon{scope,id}`→`ShowSurface{target: balloon_target(scope), surface_id: id, binds: BindSet::default(), reply: None}`／`HideBalloon{scope}`→`Hide{target: balloon_target(scope)}`。数値 key はそのまま消費（alias 非再適用・R5.3）。
- **DD-6（§7 #7 置き場）= areka bin crate 内モジュール `src/emo2_boot/`**（案 A）。新 lib crate（案 B）は M-dual という仮定要件のための分割＝simplification 原則で棄却（純粋部 lift は将来機械的に可能）。bin 内でも純粋部（target_map/adapter/talk_clock/assets の KV 抽出）は headless 単体テスト可・`tests/` 統合テスト前例あり（smoke_boot_loop_exit）。
- **DD-7（§7 #9 構築順序）**: `WinApp::new()` → `open_startup_window`（既存・不変）→ **`wire_emo2_boot`**（assets 組立→UI 部品〔EmoPresenter/TextLayerRuntime/spawn_emo_text/TalkClock/mpsc〕→`spawn_seriko(out=PresentBridge)`→`boot(SerikoSink, ClockedTextSink<EmoTextSink>)`→NonSend 挿入＋system 登録）→ `run()` → `shutdown(CloseReason::User)`＋seriko `ActorHandle` join。**asset 組立失敗時は現行どおり `LogSink`×2 で boot するフォールバック**（既存の非致命 boot 意味論と smoke テスト前提を完全温存=R7.3/7.4。`GhostRuntime` は非ジェネリックゆえ呼び分け可能）。
- **DD-8（static bindset）**: shell descript KV から `sakura.bindgroup{N}.default`==1 の N を抽出し `build_static_bindset` へ（ukadoc 正典・emo2 実測 [1100,1207,1302,1500,1800]）。kero（scope≥1）別集合の分離は M-dual 増分へ申し送り（M-boot は sakura 系集合を使用）。
- **DD-9（初期表示面）**: scope0=surface 0・scope≥1=surface 10（placement measure と同じ ukadoc 慣行定数）・バルーン=面 0 固定（emo2 は balloons0 のみ）。`sakura.balloon.defaultsurface` の descript 消費は将来増分（brief の ukadoc 必読事項への裁定）。
- **DD-10（終了理由）**: run() 復帰＝全窓 close（対話 close／smoke 自動 close の単一 funnel）を **`CloseReason::User`** で shutdown へ渡す（R6.1 の忠実写像・現行 System から変更）。close 起因の細分（SSTP 等）は M-boot 対象外。wintf への close イベント増設はエンジン改変ゆえ行わない（R10.3）。
- **DD-11（spine テストの scripted backend）**: `ScriptedShioriBackend` は areka-ghost の **テストローカル**実装で lib 公開されていない。areka-ghost への test-support 公開はエンジン改変（R10.3）ゆえ行わず、**areka 側 spine テストに最小 scripted 実装を自前で持つ**（`ShioriWiring::Custom` は pub API・`ShioriBackend` trait は公開面）。

### 9.3 リスク更新

- （高→中）装着×GPU 遅延×文字層 None の合流: DD-4 の「apply 同期＝同一フレーム Some 保証」で構造的に解消。残リスクは GPU 資源が到達しない環境（headless 実行）だが、attach フェーズが毎フレーム再試行するため hang しない（表示なしで動作継続・log-first）。
- （中）reply None 経路の apply 失敗観測: `PresentError` が reply 無しで沈黙する可能性。実装時に presenter 内部ログを確認し、不足なら drain 側で reply チャネルを添えて即 try_recv→error! する（決定論檻は spine のピクセル述語が担保）。
- （低）TalkClock ジッタによる可視数の瞬間逆行: 単調 max 方策＋char_wait(50ms)≒tick 間隔で実害なし。spine は注入クロックで決定論化しジッタ経路自体をテスト対象外に固定。
- （低・既知制約）`Clear` なし連続 talk の可視数逆行: epoch 前方リベースにより talk 跨ぎで既リビール文字が未リビール側へ写り得る（emo-text 契約「Clear が唯一のリセット」の固有性質・実運用は Clear 開始が通常）。design で明記済み＝spine の単調増加述語は単一 talk 内（Clear 起点後）に限定（設計ディスカッション A3・validation Issue 3 決着）。
