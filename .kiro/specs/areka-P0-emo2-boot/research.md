# ギャップ分析: areka-P0-emo2-boot

> 対象: M-boot 統合ユニット（emo2 起動→OnBoot トークがバルーンに typewriter 進行→close 握手→全エンジン正常終了）
> 調査日: 2026-07-11 ／ 手法: 実シンボル突合（Grep/Glob/Read）。全上流エンジンは完了済み・本ユニットは「アダプタ1個＋結線＋二段観測」に限定。

## 1. 要約（3–5 行）

- **本ユニットに実装の空白は 1 箇所のみ**: seriko の `SurfaceOutput`（本番実装）＝`DisplayCommand`→emo-present `PresentCommand` への変換＋UI スレッド配送アダプタ。他は全て完成済み部品の結線（main.rs 差し替え）と二段観測（決定論 spine ＋ env-gate 実走）。
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

### 2.9 cue 契約と `\b`
`crates/areka-sakura/src/contract.rs`: `TalkCue { at: f64, actor: ActorKey, command: CueCommand }`。`cue_target_of`: `Emote`/`EntityRef`→`Shell`、`Text`/`NewLine`/`Clear`/`Choice`→`Balloon`、`Custom`→`None`。**`CueCommand` にバルーン面切替 variant は無い**（`\b[ID]` は cue ドメインに写らない＝sakura コンパイル段で Custom 化 or 脱落）。M-boot 裁定（brief）: 既定バルーン面のみ・`\b` 未消費 no-op＋warn 1 件。

## 3. 要件→資産マップ（Missing / Unknown / Constraint）

| 要件 | 対応資産 | ギャップ種別 |
|---|---|---|
| R1 一発起動・実サーフェス可視化 | main.rs（窓 spawn 済）＋`attach_target`＋初回 `ShowSurface` | **Missing**（装着結線）|
| R2 OnBoot トーク→typewriter | boot→kanade→sakura→sink 経路 ✅＋`present_frame` 駆動 | **Missing**（present_frame 結線・talk_time 源=Unknown）|
| R3 表示指令変換・配送（scope→target 写像 正本）| `DisplayCommand`／`PresentCommand`／`UiSender`or`CommandSender` | **Missing**（アダプタ本体＝本ユニット唯一の新規正本）|
| R4 文字層装着順序 | `text_slot_view`（遅延 None）＋`register_actor_view` | **Constraint**（初回 ShowSurface→text_slot_view→register の順序厳守）|
| R5 `\b` の M-boot 裁定 | cue に variant 無し・Custom/脱落 | **Unknown→設計 1 判断**（no-op+warn の適用点）|
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
  - 中: 決定論 spine の「実 sink 経路」を headless で通す際、emo-present/emo-text は GPU（WucGraphicsResource/GraphicsCore・MTA COM）を要する。アダプタ→`PresentCommand` 記録までは GPU 不要で決定論化できるが、emo-present.apply/present_frame まで通すと GPU 前提（WARP 可・既存テストは MTA 初期化で通している）。**spine の観測境界をどこに引くか**が決定論性を左右（設計判断）。
  - 中: worker→UI 配送のスレッド安全性（`PresentCommand` は Send、presenter は NonSend・World 経由取得）。ii-1/ii-2 の選択で借用衝突の出方が変わる。
  - 低: `\b` 裁定・smoke ゲート・shutdown 呼び出しは既存資産の素直な結線。

## 6. 設計フェーズへの申し送り（Research Needed）

1. **配送経路の 1 択確定**（軸 ii: CommandSender クロージャ vs UiSender+キュー Resource+system）。presenter の remove/insert churn と借用規律・記憶 concurrency-model との整合を基準に。
2. **talk_time 時刻源**（軸 iii: FrameTime 絶対 vs talk 起点相対 vs kanade 注入同期）と、talk 切替時のリセット意味論。
3. **決定論 spine の観測境界**: (a) アダプタの `PresentCommand` 出力＋emo-text 状態記録まで（GPU 不要・純決定論）で足りるか、(b) emo-present.apply/present_frame の実装描画（GPU/WARP・MTA）まで通すか。R8.3「headless の記録で観測」「i686 非依存」を満たす線引き。
4. **scope→TargetId 写像の正本形**: `ActorKey("0")`→shell TargetId、balloon TargetId の採番規約（shell=scope 別 target・balloon=独立 target）。二人立ち M-dual が将来消費する拡張シーム。
5. **窓装着タイミングの結線パターン**: donor `boot_present_system`（GPU 資源到達待ち・各 target 高々 1 回）を踏襲しつつ、balloon target で「初回 ShowSurface→text_slot_view→register_actor_view→present_frame 開始」の合流をどの schedule/順序で保証するか。
6. **`\b[ID]` の実際の落ち先**の実装確認（sakura コンパイル段で Custom 化か脱落か）と no-op+warn の適用点（アダプタ内 or emo-text drain）。
7. **依存境界の解釈確定**: R10.5「新規依存を追加せず」は**外部（third-party）crate 追加禁止**（tokio 禁止等）と解し、areka-seriko/-emo-present/-emo-text/-sakura/-actor の**ワークスペース path 依存を areka へ昇格するのは本ユニットの結線に必須で in-scope**、という前提でよいか（brief「全部品は完成済み・結線に徹する」と整合）。
8. **アダプタ／結線の置き場**（軸 i: main 内 module vs 新 lib crate vs ハイブリッド）。M-dual への正本再利用性と bin crate テスト容易性のトレードオフ。

## 7. 設計判断項目（要件ディスカッションへ送る・番号付き）

1. 配送経路: `CommandSender` クロージャ経路（ii-1）か、`UiSender`＋キュー Resource＋毎フレーム drain system（ii-2）か。
2. `present_frame` の `talk_time` 時刻源（絶対 FrameTime／talk 起点相対／kanade 同期）とリセット意味論。
3. 決定論 spine の観測境界（アダプタ出力記録どまり／emo-present・emo-text 実描画まで）と GPU 依存の許容範囲（WARP/MTA を CI 常設に載せるか）。
4. scope→TargetId 写像の採番規約（本ユニットが立てる唯一の正本・M-dual 拡張シーム）。
5. 装着合流の schedule 設計（GPU 資源待ち→初回 ShowSurface→text_slot_view→register_actor_view→present_frame 開始の順序保証）。
6. `\b[ID]` no-op+warn の適用点（アダプタ／emo-text drain／seriko）と既定バルーン面のみ使用の徹底。
7. アダプタ・結線の置き場（areka 内 module／新 lib crate／ハイブリッド）。
8. 依存追加の解釈（外部 crate 禁止＝ワークスペース path 依存昇格は in-scope の確認）。
9. 構築順序再編の具体形（WinApp→UI 部品→shell 組立→spawn_seriko→boot）と非致命 boot 意味論の維持方法。
