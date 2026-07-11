# Brief: areka-P0-emo2-boot

> **種別**: 本坑（main）。⓪ ghost 帰属・**M-boot マイルストーンの完成ユニット（統合結線）**＝ロードマップの M-boot 節がマイルストーン名として最初から掲げてきた spec（`M-boot ＝ areka-P0-emo2-boot`）を、実体のある最終ユニットとして立てる。
> **調査日**: 2026-07-09（全エンジン完了後の再入精査③——「M-boot 統合」の無所属を検出して新設）／**2026-07-11 再調整**（再入精査④＝window-placement✅・emo-text-layer✅ の実装完了を受けた実シンボル突合）。
> **前提依存（順序ゲート）**:
> ```
> _Depends: areka-P0-window-placement ✅（2026-07-11 完了）＋ areka-P0-emo-text-layer ✅（2026-07-11 完了）
> ```
> **✅ 両ゲート解消済み＝即着手可**（M-boot 残1の最終ユニット・旧「並走フロントではない」注記は失効）。

## Problem

M-boot の 5 トラックは全て完了 or 残 2（window-placement／emo-text-layer）だが、**「emo2 が起動して喋る」を実際に観測する結線が誰の所有でもない**。ghost-setup ✅ は「表示結線はスコープ外＝sink は録音実装・**M-boot 統合が実物を挿す**」と明示的に申し送って完了しており、現 main.rs は:

- `GhostBootOptions` の sink が **両方 `LogSink`**（ログのみ・表示なし）
- 窓は **本物のゴースト窓が生成されるようになった**（window-placement ✅ が `open_startup_window` シームを差し替え済み）が、**surface 装着前＝WUC 合成で描画内容なし＝不可視**（main.rs:422 コメントが「emo2-boot が装着するまで不可視が正しい状態」と明記）
- `EmoPresenter`・`SerikoSink`・`EmoTextSink` を main は一切参照しない
- seriko の `SurfaceOutput` 終端は **mock のみ**（emo-present への橋渡し実装が存在しない）
- emo-text の `present_frame`（毎フレーム UI 駆動）を回す者がいない

つまり**各エンジンは単体で完成しているが、束ねて「動くアプリ」にする最後の一結線が空白**。ここを埋めて M-boot（emo2 起動→OnBoot トークがバルーンに流れる→close 握手→終了）を完成させる。

## Current State（2026-07-11 実シンボル・全て実装確認済み）

- **差込口（ghost-setup ✅）**: `areka_ghost::boot(GhostBootOptions { ghost_root, default_encoding, shiori: ShioriWiring, surface_sink: S, text_sink: T, ticker })` where `S: SurfaceSink + Clone + Send + 'static, T: TextSink + Clone + Send + 'static`——sink は**構築時注入・setter なし**が正本契約。`ShioriWiring::Helper { helper_exe }` で実 i686 helper 起動。`GhostRuntime::shutdown(CloseReason)` の close 握手も実装済み。main.rs:244 に boot 結線あり（**両 sink `LogSink`・非致命 boot・⚠️ 現行は `WinApp::new` より前**＝実 sink は UI 基盤を要するため構築順序の再編が必要）。
- **surface 側の部品（seriko ✅）**: `spawn_seriko(resolver: SurfaceResolver, static_binds: BindSet, out: O) -> (SerikoSink, ActorHandle)` where `O: SurfaceOutput + Send + 'static`——`SerikoSink` は `SurfaceSink` 実装済み＝そのまま `surface_sink` に挿せる。**空白は `O`（`SurfaceOutput` の本番実装）**: `DisplayCommand::Show { scope, surface_id, binds } / Hide { scope }`（output.rs:21）→ emo-present `PresentCommand::ShowSurface { target, surface_id, binds, reply } / Hide` への**アダプタ**（scope=`ActorKey`→`TargetId` 写像を含む）。
- **text 側の部品（emo-text-layer ✅・2026-07-11 完了）**: `EmoTextSink`（`Clone`・`impl TextSink`・sink.rs:41）＝そのまま `text_sink` に挿せる。取得は `spawn_emo_text(runtime: Rc<RefCell<TextLayerRuntime>>) -> (EmoTextSink, JoinHandle)`（actor.rs:254・`spawn_ui` ベース＝**UI スレッド常駐タスク**）。文字層の装着口は `TextLayerRuntime::register_actor_view(actor: ActorKey, view: &TextSlotView, model: &BalloonModel)`（actor.rs:195）。**⚠️ 新義務: `present_frame(runtime, world, talk_time: f64)`（actor.rs:288）の毎フレーム駆動は本ユニットが所有**（design L363「example/emo2-boot が駆動」）——駆動場所（wintf frame schedule への載せ方）と `talk_time` の時刻源を design で確定。
- **表示側の部品（emo-present ✅）**: `EmoPresenter`（**`!Send`＝UI スレッド常駐**）・`attach_target(&mut self, world, target: TargetId, window: Entity, emo_world: EmoWorld, atlas: AtlasTable)`（presenter.rs:149・skeleton 登録のみ）・`apply(world, PresentCommand)`・`build_balloon_target(balloon_dir, decoder)`（balloon.rs:120）・`text_slot_view(target) -> Option<TextSlotView>`（presenter.rs:404）。**⚠️ 装着順序の罠: `text_slot_view` は初回 `ShowSurface` まで `None`**（供給面/text_slot は mount 遅延生成）＝文字層の結線は「`attach_target`→当該 target へ初回 `ShowSurface`（バルーン枠表示）→`text_slot_view`→`register_actor_view`」の順序が必須。UI スレッドへの指令配送は `spawn_ui`/`UiSender` ✅（actor-foundation）。
- **窓側の部品（window-placement ✅・2026-07-11 完了）**: `placement::prepare_ghost_windows(ghost_root, balloon_root) -> PreparedPlacement`→`spawn_ghost_windows(world, &placements, &titles) -> GhostWindows`（spawn.rs:149）——**キャラ窓＋バルーン窓の Window entity 写像**はアクセサ `char_window(scope) -> Option<Entity>`／`balloon_window`／`scopes()` で取得（契約どおり両窓同梱 ✅）。main.rs `open_startup_window`（main.rs:430）は**既に本物窓を spawn 済み**（clickthrough 登録・`MonitorSnapshot` 挿入込み・`CommandSender` 経路）。**ダミー窓は良性失敗時の意図的フォールバックとして残置**（退役残渣ではない——本ユニットは触らない）。smoke ゲート `despawn_smoke_targets` は `GhostWindowMarker` を既にカバー。placement は `EmoPresenter` を import しない設計境界（design L310）＝**装着は本ユニットの領分**。
- **shell 側の構築入力（全て ✅）**: `MountModel`（package-mount）→ `parse`（shell-parse）→ `EmoWorld::build`＋`bake`（emo-atlas/compose）→ `SurfaceResolver`／`build_static_bindset`（seriko・bindgroup default 由来）——example `emo-present.rs` に組立の実績コードあり（donor）。窓採寸用に placement 側も同資産を消費済み（measure.rs＝採寸後破棄・**所有・装着は本ユニット**＝design L477）。

## Desired Outcome

`areka.exe <emo2 path>` の一発起動で: **emo2 の実 surface が既定位置に表示され、実 pasta.dll の OnBoot 応答スクリプトがバルーンに typewriter 進行で流れ、close で OnClose 握手→再生完了→全エンジン正常終了**する。＝ M-boot マイルストーン充足・M1 の「最初の可視結果」。

**✔ 観測（単一 pass/fail・二段）**:
- (a) **決定論 spine（CI 常設）**: `ScriptedShioriBackend`（ghost-setup ✅ の資産）＋**実 sink 経路**（SerikoSink→アダプタ→PresentCommand 記録・text sink→状態記録）で boot→talk 配送→close 握手の全経路を sleep 不使用で実行テスト化（表示は headless 記録・注入 Tick のみ）。
- (b) **実走（env-gate＋手動サインオフ）**: 実 pasta.dll・実表示・実 DPI（≠96）で起動→喋る→ドラッグ→close の目視確認（M-boot の性質上、最終確認は人間判断）。

## Approach

1. **SurfaceOutput→PresentCommand アダプタ（本ユニットの新規実装の本体）**: `DisplayCommand` → `UiSender` 経由で UI スレッドの `EmoPresenter::apply` へ。scope（`ActorKey`）→`TargetId` の写像・バルーン target の割当（shell=scope 別 target・balloon=独立 target）をここで確定。**アダプタは薄く**（変換と配送のみ・状態を持たない）。
2. **main.rs の結線差し替え**: `LogSink`×2 → `SerikoSink`＋`EmoTextSink`。**構築順序の再編が本丸**——実 sink は UI 基盤（`spawn_ui`・`EmoPresenter`・`TextLayerRuntime`）を要するため、現行の「boot が `WinApp::new` 前」を再編（例: WinApp→UI 側部品 spawn→実 sink 取得→boot。mount→shell parse→bake/EmoWorld→resolver/bindset→spawn_seriko の組立順は example `emo-present.rs` donor どおり）。非致命 boot（`is_benign_boot_error`）の意味論は維持。
3. **窓とプレゼンタの装着**: `GhostWindows`（`char_window(scope)`／`balloon_window`）から `attach_target`（UI スレッド上）。**装着順序の罠に従う**: balloon target は `build_balloon_target`＋バルーン窓 entity へ `attach_target`→**初回 `ShowSurface`（バルーン枠表示）→`text_slot_view(target)` が `Some` になってから→`register_actor_view`** で文字層を接続。`spawn_emo_text` の actor spawn・`present_frame` の毎フレーム駆動（＋`talk_time` 時刻源）も本ユニットが main で結線。
4. **終了経路の総仕上げ**: 窓 close→`shutdown(CloseReason)`→OnClose 応答の**再生完了待ち**（kanade ✅ の close 握手＝sakura TalkDone 突合）→全 join。smoke ゲートは本物経路で存続（`AREKA_APP_SMOKE_EXIT_MS`・`GhostWindowMarker` カバー済み ✅）。ダミー窓フォールバックは**意図的残置＝触らない**（window-placement の良性失敗設計）。
5. **新規機構は作らない**: 全部品は完成済み——本ユニットは**アダプタ1個＋結線＋観測**に徹する（フレームワーク化禁止・記憶 areka-concurrency-model）。

## クロスユニット契約（2026-07-09）

- **消費する正本（再定義しない）**: talk 契約=`areka-talk` ✅／再生出力契約 `TalkCue`/`cue_target_of`=sakura ✅／表示指令 `DisplayCommand`=seriko ✅／`PresentCommand`/`TargetId`/`TextSlotView`=emo-present ✅／文字層装着 `register_actor_view`・sink `EmoTextSink`=emo-text-layer ✅／窓写像 `GhostWindows`=window-placement ✅／死活語彙 `LifecycleReport`=host32-lifecycle ✅／sink 注入契約 `GhostBootOptions`=ghost-setup ✅。**本ユニットが新たに正本を立てるのは「scope→TargetId 写像」のみ**（アダプタ内・二人立ち M-dual が将来消費）。
- **上流2本の申し送りは充足確認済み（2026-07-11 実シンボル突合）**: window-placement は `GhostWindows`（キャラ窓＋バルーン窓同梱 ✅）を納品・emo-text-layer は `EmoTextSink`（`TalkCue` を `TextSink` で受ける・`Clone + Send` ✅）＋`register_actor_view`（`TextSlotView` 消費の装着 API ✅）を納品——**本ユニットは結線のみで済む形が成立**。唯一の追加義務は `present_frame` 毎フレーム駆動（上記 Current State）。
- **`\b[ID]`（バルーン面切替 cue）の裁定義務**: emo-text-layer design L849 が「emo2-boot/バルーン切替ユニットで裁定」と申し送り——**M-boot 裁定案: 既定バルーン面のみ・`\b` は未消費（受けたら warn ログの no-op）**＝バルーン面切替は増分ユニットへ（design で1判断・確定させること）。
- **M-e2e（emo2-conformance-e2e）との境界**: 本ユニット＝**boot→talk→close の一本道**（M-boot 完成）。touch/メニュー/選択肢を含む一周適合は M-e2e（M-life/M-dialogue 後）——アプリ組み上げ三段の第三段はあちらのまま。

## ukadoc 必読（design 着手時）

- **`list_shiori_event` の boot/close 節**（OnInitialize→204 フォールスルー→OnBoot→basewareversion NOTIFY／OnClose→再生完了待ち→OnCloseAll）——**実装は kanade ✅ 済み**・本ユニットは「実 pasta 相手に正しく発火するか」の観測側＝イベント引数（Reference0=shell 名等）の実値を fixture で突合。
- **`descript_ghost` の `balloon`/`sakura.balloon.defaultsurface`**——バルーン所在解決（M-boot は fixture 直指定で可・app-shell の構成入力に balloon path 済み）と既定バルーン面番号の適用を design で1判断。

## Scope

- **In**: SurfaceOutput→PresentCommand アダプタ（scope→TargetId 写像含む）／main.rs の実 sink 結線・構築順序再編（boot を UI 基盤の後へ）／窓写像（キャラ＋バルーン）→attach_target 装着（初回 ShowSurface→register_actor_view の順序遵守）／`spawn_emo_text`＋`present_frame` 毎フレーム駆動の結線（talk_time 時刻源確定含む）／`\b` cue の M-boot 裁定（no-op warn 案）／実 pasta helper での boot→talk→close 実走（env-gate）／決定論 spine の統合テスト。
- **Out**: 窓の生成・配置・ドラッグ（**window-placement**）／文字描画（**emo-text-layer**）／touch・メニュー・選択肢（**M-life/M-dialogue**）／一周適合証明（**emo2-conformance-e2e**）／二人立ち target 割当の本格化（**M-dual**・写像シームまで）。

## Boundary Candidates

- アダプタ（DisplayCommand→PresentCommand・純粋変換＋配送）／結線（main.rs 構築順序）／観測（spine 統合テスト＋env-gate 実走）の三片。

## Out of Boundary

- 各エンジンの内部改変（全て完成済み・変更が必要と判明したら該当エンジンへ増分 issue として申し送り）。

## Upstream / Downstream

- **Upstream**: **全 ✅**（window-placement ✅ 2026-07-11／emo-text-layer ✅ 2026-07-11／ghost-setup/kanade/sakura/seriko/emo 直列3/shiori 全/parsers 全/actor-foundation/app-shell）。
- **Downstream**: M-boot 後の全増分（M-life/M-dialogue/M-dual）・`areka-P0-emo2-conformance-e2e`（M-e2e）——**M-boot 完成が全増分の解禁条件**。バルーン面切替（`\b` 実消費）の増分ユニットも本ユニットの裁定を消費。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-P0-ghost-setup`（sink 注入契約の実行）・`completed/areka-P0-app-shell`（骨格 main の完成形へ）。
- **Adjacent**: `completed/areka-P0-window-placement`✅／`completed/areka-P0-emo-text-layer`✅（両ゲート解消済み・実シンボル消費）／`areka-P0-emo-text-viewbox`（並走候補・areka-emo-text の描画実行側のみ改変＝本ユニットの消費面〔sink/装着 API/present_frame〕とは非交差・pixel 等価 golden が並走安全を担保）。

## Constraints

- Rust 2024・tokio 禁止・新規依存なし。`EmoPresenter` は UI スレッド固定（`!Send`）・sink/アダプタは worker→`UiSender` 配送（並行モデル正本）。
- **決定論テスト網羅**（記憶 deterministic-test-coverage-mandate）: spine は sleep 不使用・注入 Tick のみ。実 pasta は env-gate 追験（DoD 前提にしない＝記憶 prefer-x64-fake-boundary-tests-not-x86）。
- 最終サインオフは実走の人間判断（M-boot の性質上）——AI 単独で milestone 完了を宣言しない。
