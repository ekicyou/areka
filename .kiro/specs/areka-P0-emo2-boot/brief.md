# Brief: areka-P0-emo2-boot

> **種別**: 本坑（main）。⓪ ghost 帰属・**M-boot マイルストーンの完成ユニット（統合結線）**＝ロードマップの M-boot 節がマイルストーン名として最初から掲げてきた spec（`M-boot ＝ areka-P0-emo2-boot`）を、実体のある最終ユニットとして立てる。
> **調査日**: 2026-07-09（全エンジン完了後の再入精査③——「M-boot 統合」の無所属を検出して新設）。
> **前提依存（順序ゲート・2ユニット待ち）**:
> ```
> _Depends: areka-P0-window-placement（本物ゴースト窓の生成・配置）＋ areka-P0-emo-text-layer（TextSink 実装・文字描画）
> ```
> **⚠️ 本ユニットは並走フロントではない**——上記2本の完了後に着手（M-boot 残3の最終・逐次）。

## Problem

M-boot の 5 トラックは全て完了 or 残 2（window-placement／emo-text-layer）だが、**「emo2 が起動して喋る」を実際に観測する結線が誰の所有でもない**。ghost-setup ✅ は「表示結線はスコープ外＝sink は録音実装・**M-boot 統合が実物を挿す**」と明示的に申し送って完了しており、現 main.rs は:

- `GhostBootOptions` の sink が **両方 `LogSink`**（ログのみ・表示なし）
- 窓は **`spawn_dummy_window`（検証用ダミー）** のまま
- `EmoPresenter`・`SerikoSink` を main は一切参照しない
- seriko の `SurfaceOutput` 終端は **mock のみ**（emo-present への橋渡し実装が存在しない）

つまり**各エンジンは単体で完成しているが、束ねて「動くアプリ」にする最後の一結線が空白**。ここを埋めて M-boot（emo2 起動→OnBoot トークがバルーンに流れる→close 握手→終了）を完成させる。

## Current State（2026-07-09 実シンボル・全て確認済み）

- **差込口（ghost-setup ✅）**: `areka_ghost::boot(GhostBootOptions { ghost_root, default_encoding, shiori: ShioriWiring, surface_sink: S, text_sink: T, ticker })`——sink は**構築時注入・setter なし**が正本契約。`ShioriWiring::Helper { helper_exe }` で実 i686 helper 起動。`GhostRuntime::shutdown(CloseReason)` の close 握手・`into_parts`/`GhostParts` も実装済み。
- **surface 側の部品（seriko ✅）**: `spawn_seriko(resolver: SurfaceResolver, static_binds: BindSet, out: O) -> (SerikoSink, ActorHandle)` where `O: SurfaceOutput`——`SerikoSink` は `SurfaceSink` 実装済み＝そのまま `surface_sink` に挿せる。**空白は `O`（`SurfaceOutput` の本番実装）**: `DisplayCommand::Show { scope, surface_id, binds } / Hide { scope }` → emo-present `PresentCommand::ShowSurface { target, surface_id, binds, reply } / Hide` への**アダプタ**（scope=`ActorKey`→`TargetId` 写像を含む）。
- **text 側の部品（emo-text-layer・依存待ち）**: TextSink 実装 sink → `text_sink` に挿す。
- **表示側の部品（emo-present ✅）**: `EmoPresenter`（**`!Send`＝UI スレッド常駐**）・`attach_target(world, target, window: Entity, emo_world, atlas)`・`apply(world, PresentCommand)`・`build_balloon_target(balloon_dir, decoder)`。UI スレッドへの指令配送は `spawn_ui`/`UiSender` ✅（actor-foundation）。
- **窓側の部品（window-placement・依存待ち）**: **キャラ窓（スコープ別）＋バルーン窓**の Window entity 写像を公開 API から取得（あちらの brief に契約記載済み・07-09 最終精査でバルーン窓を写像に含むことを両 brief で対に確定）。`open_startup_window` シームの中身（**ダミー窓→本物窓の差し替え＝あちらの領分**）も完了済みの想定＝本ユニットはダミー窓に関与しない（残渣が見つかった場合の後始末のみ）。
- **shell 側の構築入力（全て ✅）**: `MountModel`（package-mount）→ `parse`（shell-parse）→ `EmoWorld::build`＋`bake`（emo-atlas/compose）→ `SurfaceResolver`／`build_static_bindset`（seriko・bindgroup default 由来）——example `emo-present.rs` に組立の実績コードあり（donor）。

## Desired Outcome

`areka.exe <emo2 path>` の一発起動で: **emo2 の実 surface が既定位置に表示され、実 pasta.dll の OnBoot 応答スクリプトがバルーンに typewriter 進行で流れ、close で OnClose 握手→再生完了→全エンジン正常終了**する。＝ M-boot マイルストーン充足・M1 の「最初の可視結果」。

**✔ 観測（単一 pass/fail・二段）**:
- (a) **決定論 spine（CI 常設）**: `ScriptedShioriBackend`（ghost-setup ✅ の資産）＋**実 sink 経路**（SerikoSink→アダプタ→PresentCommand 記録・text sink→状態記録）で boot→talk 配送→close 握手の全経路を sleep 不使用で実行テスト化（表示は headless 記録・注入 Tick のみ）。
- (b) **実走（env-gate＋手動サインオフ）**: 実 pasta.dll・実表示・実 DPI（≠96）で起動→喋る→ドラッグ→close の目視確認（M-boot の性質上、最終確認は人間判断）。

## Approach

1. **SurfaceOutput→PresentCommand アダプタ（本ユニットの新規実装の本体）**: `DisplayCommand` → `UiSender` 経由で UI スレッドの `EmoPresenter::apply` へ。scope（`ActorKey`）→`TargetId` の写像・バルーン target の割当（shell=scope 別 target・balloon=独立 target）をここで確定。**アダプタは薄く**（変換と配送のみ・状態を持たない）。
2. **main.rs の結線差し替え**: `LogSink`×2 → `SerikoSink`＋text-layer sink。構築順序＝mount→shell parse→bake/EmoWorld→resolver/bindset→spawn_seriko→boot(GhostBootOptions)（`WinApp::new` との順序・UI スレッド初期化との整合は design で確定——現行は boot が WinApp 前）。
3. **窓とプレゼンタの装着**: window-placement の窓写像（**キャラ窓＋バルーン窓**）から `attach_target`（UI スレッド上）。balloon target は `build_balloon_target`＋**バルーン窓 entity**へ装着し、**text-layer の装着 API（あちらが emo-present へ増設する公開経路）でバルーン窓の text_slot へ文字層を接続**——TextSink の actor spawn・UiSender 結線も本ユニットが main で行う（text-layer は sink 型と装着 API を提供するまで＝あちらの brief と対）。
4. **終了経路の総仕上げ**: 窓 close→`shutdown(CloseReason)`→OnClose 応答の**再生完了待ち**（kanade ✅ の close 握手＝sakura TalkDone 突合）→全 join。smoke ゲートは本物経路で存続（`AREKA_APP_SMOKE_EXIT_MS`＝CI smoke 継続。ダミー窓の退役は **window-placement の領分・完了済み想定**——残渣の後始末のみ本ユニット）。
5. **新規機構は作らない**: 全部品は完成済み——本ユニットは**アダプタ1個＋結線＋観測**に徹する（フレームワーク化禁止・記憶 areka-concurrency-model）。

## クロスユニット契約（2026-07-09）

- **消費する正本（再定義しない）**: talk 契約=`areka-talk` ✅／再生出力契約 `TalkCue`/`cue_target_of`=sakura ✅／表示指令 `DisplayCommand`=seriko ✅／`PresentCommand`/`TargetId`=emo-present ✅／死活語彙 `LifecycleReport`=host32-lifecycle ✅／sink 注入契約 `GhostBootOptions`=ghost-setup ✅。**本ユニットが新たに正本を立てるのは「scope→TargetId 写像」のみ**（アダプタ内・二人立ち M-dual が将来消費）。
- **上流2本への申し送り（依存が先に完了する前提の確認点・07-09 最終精査で補完）**: window-placement は「**キャラ窓（スコープ別）＋バルーン窓**の Window entity 写像の公開 API」（あちらの brief 記載済み・バルーン窓の同梱が必達——無いと balloon target が装着不能で統合が詰む）・emo-text-layer は「`TextSink + Clone + Send + 'static` を満たす sink 型」＋「**text_slot への装着 API**（emo-present へ増設する公開経路）」（あちらの brief 記載済み）——この形なら本ユニットは結線のみで済む。
- **M-e2e（emo2-conformance-e2e）との境界**: 本ユニット＝**boot→talk→close の一本道**（M-boot 完成）。touch/メニュー/選択肢を含む一周適合は M-e2e（M-life/M-dialogue 後）——アプリ組み上げ三段の第三段はあちらのまま。

## ukadoc 必読（design 着手時）

- **`list_shiori_event` の boot/close 節**（OnInitialize→204 フォールスルー→OnBoot→basewareversion NOTIFY／OnClose→再生完了待ち→OnCloseAll）——**実装は kanade ✅ 済み**・本ユニットは「実 pasta 相手に正しく発火するか」の観測側＝イベント引数（Reference0=shell 名等）の実値を fixture で突合。
- **`descript_ghost` の `balloon`/`sakura.balloon.defaultsurface`**——バルーン所在解決（M-boot は fixture 直指定で可・app-shell の構成入力に balloon path 済み）と既定バルーン面番号の適用を design で1判断。

## Scope

- **In**: SurfaceOutput→PresentCommand アダプタ（scope→TargetId 写像含む）／main.rs の実 sink 結線・構築順序確定／窓写像（キャラ＋バルーン）→attach_target 装着／balloon target＋text-layer 装着 API の結線（TextSink actor spawn・UiSender 含む）／実 pasta helper での boot→talk→close 実走（env-gate）／決定論 spine の統合テスト／退役残渣の後始末（ダミー窓本体の差し替えは placement 済み想定）。
- **Out**: 窓の生成・配置・ドラッグ（**window-placement**）／文字描画（**emo-text-layer**）／touch・メニュー・選択肢（**M-life/M-dialogue**）／一周適合証明（**emo2-conformance-e2e**）／二人立ち target 割当の本格化（**M-dual**・写像シームまで）。

## Boundary Candidates

- アダプタ（DisplayCommand→PresentCommand・純粋変換＋配送）／結線（main.rs 構築順序）／観測（spine 統合テスト＋env-gate 実走）の三片。

## Out of Boundary

- 各エンジンの内部改変（全て完成済み・変更が必要と判明したら該当エンジンへ増分 issue として申し送り）。

## Upstream / Downstream

- **Upstream**: **`areka-P0-window-placement`（未・ゲート）**・**`areka-P0-emo-text-layer`（未・ゲート）**・他は全 ✅（ghost-setup/kanade/sakura/seriko/emo 直列3/shiori 全/parsers 全/actor-foundation/app-shell）。
- **Downstream**: M-boot 後の全増分（M-life/M-dialogue/M-dual）・`areka-P0-emo2-conformance-e2e`（M-e2e）——**M-boot 完成が全増分の解禁条件**。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-P0-ghost-setup`（sink 注入契約の実行）・`completed/areka-P0-app-shell`（骨格 main の完成形へ）。
- **Adjacent**: `areka-P0-window-placement`／`areka-P0-emo-text-layer`（両ゲート・完了待ち）。

## Constraints

- Rust 2024・tokio 禁止・新規依存なし。`EmoPresenter` は UI スレッド固定（`!Send`）・sink/アダプタは worker→`UiSender` 配送（並行モデル正本）。
- **決定論テスト網羅**（記憶 deterministic-test-coverage-mandate）: spine は sleep 不使用・注入 Tick のみ。実 pasta は env-gate 追験（DoD 前提にしない＝記憶 prefer-x64-fake-boundary-tests-not-x86）。
- 最終サインオフは実走の人間判断（M-boot の性質上）——AI 単独で milestone 完了を宣言しない。
