# Gap Analysis: wintf-gpu-test-crash

## Analysis Summary

- **既存資産**: WUC ライフサイクル一式（`GraphicsCore`／`WucGraphicsResource`／`com/wuc.rs` interop）は実装済みで機能自体は健全（単発なら緑）。テスト側は各ドメインが `setup_world()`／`make_world_with_gpu()` という**同型だが非共有**のヘルパーを個別実装しており、共通 fixture は存在しない。
- **不足している能力**: (1) `DispatcherQueueController` の**明示 shutdown**（`ShutdownQueueAsync` ドレイン）を実装しているのは `examples/wuc_spike.rs` のみで、`wuc_resource.rs`／全テストヘルパー／`WucGraphicsResourceInner::drop`（宣言順 drop 任せ）には存在しない。(2) 同一プロセス内 WUC 複数回生成を検証する回帰テストが存在しない（Requirement 5 が要求するものは新規）。(3) bisect・実クラッシュスタック取得（cdb/WinDbg）はまだ未実施（brief の「調査の第一手」未着手）。
- **コード調査による H-code 弱体化の物証**: `68bd2e3e` が導入した `CoIncrementMTAUsage`（`wic_core.rs::ensure_process_mta`）は `WicCore::new()` からのみ呼ばれ、`WicCore::new()` の呼び出しは `tests/widget/bitmap_source_integration_test.rs`（＝`tests/widget.rs` バイナリ）に限定される。`cargo test -p wintf --test graphics`（`tests/graphics.rs` が束ねる 14 ドメインファイル）は `WicCore` を一切参照しない。ゆえに **wintf graphics バイナリのクラッシュに限っては H-code（MTA 常駐×WUC 相互作用）の直接経路が構造的に見当たらない**——bisect で最終確定は必要だが、コード的物証は H-env 側に傾く。ただし areka bin（画像ロードで `WicCore` を経由し得る）側は独立に検証が要る。
- **影響範囲の追加候補（未確認・要検証）**: `crates/areka-emo-text/tests/draw_readback_test.rs` は単一バイナリ内に `make_world_with_gpu()`（`GraphicsCore::new`+`WucGraphicsResource::new`）を呼ぶ `#[test]` が **2 個**ある。`emo2_fixture_e2e_test.rs`・`attach_wiring_test.rs` も同型ヘルパーを含む。これらは brief が確認した wintf graphics／areka bin 以外の「同一プロセス複数 WUC 生成」候補であり、Requirement 2 AC3 の検証対象になり得る。
- **候補アプローチ**: brief の A（根因修正・DispatcherQueueController 明示 shutdown 導入）／B（共有 fixture 化）／C（プロセス分離）を踏襲。コード調査は A の具体的介入点（`WucGraphicsResourceInner::drop` への明示 `ShutdownQueueAsync` 追加、または全 `setup_world()` 系ヘルパーへの共通 teardown 関数抽出）を特定した。

---

## 1. 現状調査（Current State Investigation）

### 主要ファイル・モジュール

| 領域 | パス | 役割 |
|---|---|---|
| WUC ライフサイクル本体 | `crates/wintf/src/ecs/graphics/wuc_resource.rs` | `WucGraphicsResource`（`Compositor`／`CompositionGraphicsDevice`／`DispatcherQueueController` を `Option<Inner>` 遅延初期化・宣言順 drop） |
| WUC interop wrapper | `crates/wintf/src/com/wuc.rs` | `create_dispatcher_queue_controller`（`DQTYPE_THREAD_CURRENT`）・`CompositorInteropExt`・`CompositorDesktopInteropExt`・`DrawingSurfaceInteropExt` |
| D3D/D2D/DWrite 基盤 | `crates/wintf/src/ecs/graphics/core.rs` | `GraphicsCore`（`ID3D11Device`/`ID2D1Device`/`IDWriteFactory2` 等・`unsafe impl Send+Sync` 明記） |
| **既知良好パターン（本番未採用）** | `crates/wintf/examples/wuc_spike.rs` | `dq.ShutdownQueueAsync()` を発行し `Status()` ポーリングで完了ドレインを待ってから `drop`（要件 3.3 相当の意図した最も丁寧な終了シーケンス）。**この明示 shutdown はプロダクションコード・テストヘルパーいずれにも存在しない** |
| クラッシュ最小再現ペアの実体 | `crates/wintf/tests/graphics/clip_sync_system_test.rs` | 各 `#[test]` が独自の `setup_world()`（`CoInitializeEx(MULTITHREADED)`→`GraphicsCore::new`→`WucGraphicsResource::new`）を持つ。**明示 shutdown なし・`World` drop 任せ** |
| areka bin 側の波及箇所 | `crates/areka/src/emo2_boot/spine.rs:250-263` | `make_world_with_gpu()`（wintf 側と同型パターン）。`spine_e2e_kero_blink_one_cycle_golden`（line 1701）が該当 |
| MTA 常駐導入箇所（bisect 対象） | `crates/wintf/src/ecs/widget/bitmap_source/wic_core.rs` | `ensure_process_mta()`（`CoIncrementMTAUsage`・cookie leak・`OnceLock` 一度きり）。`WicCore::new()` からのみ呼ばれる |

### テストハーネス構造の実態

- `tests/graphics.rs` は `#[path]` で 14 ファイルを 1 バイナリへ束ねる（`structure.md` の「テスト入口の固定化」規約どおり）。
- **`setup_world()` は共有されていない**——`clip_sync_system_test.rs`／`surface_systems_test.rs`／`dcomp_integration_test.rs` がそれぞれ独自定義（同型コピー）。`tests/graphics/common/` のような共通モジュールは存在しない（`Glob` で確認済み・0 件）。
- `tests/visual/graphics_auto_creation_test.rs` にも `setup_world_with_graphics() -> Result<World>` という別名の同型ヘルパーがある（`tests/visual.rs` バイナリ）。

### 同一プロセス内複数 WUC 生成の全数調査（`WucGraphicsResource::new(` grep・20 ファイル中）

brief が実測確認したのは wintf graphics バイナリと areka bin のみだが、コード上「同一バイナリ内で複数 `#[test]` が `WucGraphicsResource::new` を呼ぶ」候補は他にも存在する:

- `crates/areka-emo-text/tests/draw_readback_test.rs` — `make_world_with_gpu()` を呼ぶ `#[test]` が2個（L298, L505）。**単一バイナリ内で 2 回 WUC 生成が起きる構造**（brief が未検証の候補）。
- `crates/areka-emo-text/tests/emo2_fixture_e2e_test.rs`・`attach_wiring_test.rs` — GPU world 生成ヘルパーを含み `#[test]` 3 個。全数が GPU 経路を通るかは個別確認要（Research Needed）。
- `crates/wintf/tests/visual.rs`（`component_test.rs`／`graphics_auto_creation_test.rs` 束ね）、`crates/wintf/tests/ecs/lazy_reinit_pattern_test.rs`、`crates/wintf/tests/com/{d3d11,d2d_ext}_test.rs` も `WucGraphicsResource::new` または `GraphicsCore::new` を参照（`GraphicsCore` 単体は WUC を伴わないため対象外の可能性があるが要選別）。

これは Requirement 2 AC3（「wintf graphics および areka bin 以外のテストバイナリが同種構造を持つ場合、検証対象とする」）に直接該当する調査対象であり、設計フェーズでの全数当たりが必要。

### H-code（68bd2e3e／MTA 常駐）の妥当性再検証

`68bd2e3e` の変更ファイル（`git show --stat` で確認済み・7 ファイル）:
`crates/wintf/src/ecs/widget/bitmap_source/wic_core.rs`（MTA 常駐導入）／`crates/wintf/src/ecs/world/mod.rs`（16 行・ログ付与のみ）／areka-kanade 3 ファイル・areka-ghost 1 ファイル（テスト決定論化、WUC 非関連）。

`ensure_process_mta()` の呼び出し元は `WicCore::new()` の1箇所のみで、`WicCore::new()` の呼び出しは `crates/wintf/tests/widget/bitmap_source_integration_test.rs`（`tests/widget.rs` バイナリ）に限定される（grep で確認・他に呼び出し元なし）。`tests/graphics.rs` の 14 モジュールはいずれも `WicCore`／`wic_core`／`bitmap_source` を import していない。

→ **wintf graphics バイナリのクラッシュを H-code（`68bd2e3e` の MTA 常駐）で説明する直接の呼び出し経路はコード上見当たらない**。`ecs/world/mod.rs` の 16 行差分がログ追加のみであれば、この bisect 対象コミットが graphics バイナリの挙動を変える余地はほぼ無い。ただし:
- `CoIncrementMTAUsage` はプロセスグローバルな COM ランタイム状態を変更する API であるため、**別バイナリ（プロセス）には影響しない**が、**同一バイナリ内であっても未使用コードパスであれば無関係**という理解が正しいかは bisect で最終確認すべき（純粋な静的呼び出しグラフの推論であり、リンク時の副作用や static initializer の有無までは検証していない）。
- areka bin（画像ロードで `WicCore` を経由する可能性が高い）は H-code の影響を受け得るため、wintf graphics と areka bin を**同一原因と決めつけず個別に bisect する**選択肢を設計フェーズで検討する価値がある（brief は同一法則と整理しているが、コード動線は両者で異なる）。

この所見は「H-env 最有力」という brief の結論を補強する追加物証だが、**bisect（`68bd2e3e~1` での実走）による直接判定は本 spec の Requirement 3 AC1 が明示的に要求しており、コード調査だけでは代替できない**（Research Needed として維持）。

### DispatcherQueueController のスレッド親和性と shutdown 欠如

`create_dispatcher_queue_controller`（`com/wuc.rs:126`）は `DQTYPE_THREAD_CURRENT` を使う——**呼び出しスレッドに束縛した DispatcherQueue**を生成する。Rust の libtest は既定で `#[test]` ごとに新規 OS スレッドを spawn するため、以下が理論的な懸念点として浮かぶ（bisect・cdb 実測で裏取りが必要な仮説であり、本 gap analysis 段階では確証ではない）:

- `WucGraphicsResourceInner` の drop は宣言順（`compositor` → `graphics_device` → `dq_controller`）に任せるのみで、**`ShutdownQueueAsync` の明示発行・完了ドレイン待ちを一切行わない**（`wuc_resource.rs` 全体・`wuc_spike.rs` にのみ存在する丁寧な手順と対照的）。
- `DispatcherQueueController` の COM ドキュメント上の既知動作として、明示 `ShutdownQueueAsync` を経ない破棄は、キューのバックグラウンドスレッド終了が非同期的に遅延する可能性がある。次のテストが同一プロセス内で新しい `Compositor`／`DispatcherQueueController` を生成する際、前者の非同期な内部終了処理と競合する余地が構造的にある。
- これは brief の「調査の第一手」(b)（1個目の world を `mem::forget` して teardown 犯人説を検証）と直接対応する検証対象であり、design フェーズでの根本原因確定作業そのもの。本 gap analysis はコード上の欠落（明示 shutdown が無い）を特定したに留める。

---

## 2. Requirement-to-Asset Map

| Requirement | 既存資産 | ギャップ分類 | 詳細 |
|---|---|---|---|
| R1: graphics スイート決定論的グリーン化 | `WucGraphicsResource`／`GraphicsCore`（単発は健全）・`setup_world()` 系ヘルパー14+ 箇所 | **Missing**（根本原因の是正コード） | 現状は 100% 決定論的にクラッシュ。明示 shutdown 欠如が候補原因の一つ（未確定） |
| R2: workspace Test Gate 復旧 | `cargo test --workspace` コマンド自体は既存 | **Missing**（areka bin 側の同種修正）／**Unknown**（他バイナリの波及有無） | `spine_e2e_kero_blink_one_cycle_golden` は areka クレート無改変での修正が必須（wintf 側修正のみで解決するかは根因次第）。他バイナリ（`draw_readback_test.rs` 等）の当たりは未調査 |
| R3: 根本原因の特定と記録 | brief の診断マトリクス・タイムライン・仮説（bisect 手順含む） | **Missing**（bisect 実行・cdb/WinDbg スタック取得の両方が未実施） | 本 gap analysis はコード静的解析で H-code を弱める物証を追加したのみ。動的検証（bisect・デバッガ）は design/impl フェーズの必須先行作業 |
| R4: 本番ライフサイクルへの含意の明文化 | `tech.md`／`structure.md` の WUC 節（スレッド親和・DQTAT_COM_NONE 規約） | **Unknown**（根因確定後にしか判定できない） | 「テストハーネス固有」か「本番リスク」かの二択判定はこの spec の中核成果物であり、design フェーズの根因確定を前提とする |
| R5: 再発防止の回帰テスト | `wuc_resource.rs` 内の `#[cfg(test)] wuc_graphics_resource_lifecycle`（同一テスト内で `new`→`invalidate`→再 `new`→drop を検証・**同一スレッド内**） | **Missing**（プロセス内複数回・別テスト実行単位での回帰） | 既存の `wuc_graphics_resource_lifecycle` は同一関数内での再生成であり、**libtest の別スレッド分離を跨ぐ**今回のクラッシュパターン（別 `#[test]` 関数間）を捕捉しない。新規回帰テストは「2 個の独立した `#[test]` が連続して WUC world を生成する」構造を明示的に模す必要がある |

---

## 3. Implementation Approach Options

### Option A: 根因修正（WUC リソース寿命是正・brief の Approach A に対応）

**該当ファイル**: `crates/wintf/src/ecs/graphics/wuc_resource.rs`（`WucGraphicsResourceInner` の Drop 実装追加、または明示 `shutdown()` メソッド新設）／`crates/wintf/src/com/wuc.rs`（`create_dispatcher_queue_controller` 周辺）。

- **是正の具体像（候補）**: `wuc_spike.rs` が実証した `ShutdownQueueAsync` 発行＋メッセージポンプでのドレイン待ちパターンを、`WucGraphicsResource` に明示 `Drop` として実装するか、呼び出し側（テストヘルパー・本番 `App` 終了経路）に明示 `shutdown()` 呼び出しを追加する。
- **適合性**: `tech.md` の「WUC はスレッド親和・`DQTAT_COM_NONE`」規約と整合。本番の `WinApp` 終了経路（`runtime/mod.rs` の `ShutdownPolicy`）にも teardown 拡張点が既にある構造（`shutdown_hook` 等）。
- **Trade-offs**:
  - ✅ 根因が本番リスクだった場合の唯一の正しい解（Requirement 4 AC2 が要求する経路）
  - ✅ `wuc_spike.rs` に実証済みパターンがあり車輪の再発明が不要
  - ❌ COM/WinRT 非同期 teardown のデバッグ・タイミング依存の再現性検証が重い（brief 既述のとおり）
  - ❌ 本番 `App` 側は現状ゴースト再ロード等で WUC 再生成をまだ行っていないため、本番側の呼び出し箇所は新設に近い（design フェーズで洗い出しが必要）

### Option B: テストハーネス共有 fixture 化（brief の Approach B に対応）

**該当ファイル**: `crates/wintf/tests/graphics/`（14 ファイルの `setup_world()` 統合）・新設 `crates/wintf/tests/graphics/common/mod.rs`。

- **該当パターン**: `OnceLock<Mutex<World>>` または similar でプロセス内 1 個の WUC スタックを共有し、テスト間で使い回す。Defender 飢餓対策で採用済みの「共有 hardlink fixture」定石（memory `areka-defender-rescan-starves-cooperative-test-loops`）と同系の手法。
- **Trade-offs**:
  - ✅ 実装が小さく速い（既存 `setup_world()` を共通化するだけ）
  - ✅ Requirement 1/2 の受入基準（グリーン化）は満たせる
  - ❌ **brief 明記のとおり根因を隠すだけ**——本番のゴースト再ロード・シェル切替が同種クラッシュを踏むリスクは未解決のまま残る
  - ❌ Requirement 4 AC2/AC3 の分岐ロジック上、「ハザードが本番の実在リスク」と判明した場合はこの Option 単独では要件を満たせない（B は「テストハーネス固有」と判定された場合のみ許容される縮退経路）
  - ❌ テスト独立性の低下（brief 既述）

### Option C: プロセス分離（brief の Approach C・最終避難路）

**該当ファイル**: `crates/wintf/tests/graphics.rs`（14 ファイルを複数バイナリへ再分割）または `Cargo.toml` の `[[test]]` セクション追加。

- **Trade-offs**:
  - ✅ 確実にグリーン化する（同一プロセス内複数生成という前提条件自体を消す）
  - ❌ 根因未解明のまま終わる＝ Requirement 3/4 を満たさない
  - ❌ ビルド時間増（バイナリ数増加）・`structure.md` の「テスト入口の固定化」規約からの逸脱
  - ❌ brief 明記のとおり最終手段

### 推奨（brief 踏襲・info-over-decision の範囲内での整理）

brief の推奨方針（A を主・B を A 確定後の縮退選択肢として保持・C は最後の避難路）は本コード調査でも覆らない。ただし本調査により **A の具体的介入点が「`WucGraphicsResourceInner` への明示 `ShutdownQueueAsync` ドレイン追加」に絞り込めた**——これは `wuc_spike.rs` に実証済みパターンがあるため、bisect／cdb 実測後の実装コストは brief 想定より低い可能性がある。

---

## 4. Implementation Complexity & Risk

| 観点 | 評価 | 根拠 |
|---|---|---|
| Effort（bisect＋cdb 込みの調査フェーズ） | S〜M（1-4日目安） | 手順は brief に明記済み・実行のみ。`68bd2e3e~1` チェックアウト＋ビルド＋最小ペア実走は数時間規模 |
| Effort（Option A 実装・根因確定後） | S〜M | `wuc_spike.rs` に実証済みパターンがあるため実装自体は小さいが、全 `setup_world()` 系ヘルパー（14+ 箇所・wintf/areka-emo-text 双方）への展開＋回帰テストが対象範囲を広げる |
| Effort（Option B 実装） | S | 共通化のみ・新規ロジックなし |
| Risk（根因特定） | Medium | COM/WinRT 非同期 teardown のデバッグは環境依存性が高い（brief 既述）。bisect で H-env/H-code のどちらかに絞り込めれば Risk は下がる |
| Risk（Option A 是正実装） | Medium | 本番 `WinApp` 終了経路への波及がある場合、既存の 91+ テストへの回帰影響評価が必要 |
| Risk（Option B 実装） | Low | 影響範囲が testonly に閉じる。ただし根因を確定させずに採用すると Requirement 4 の要件充足が未達になるリスク（要件上の high risk） |
| Risk（未調査の追加バイナリ） | Medium | `draw_readback_test.rs` 等が現状グリーンなら「なぜ wintf graphics だけ落ちるか」の差分要因（実行順・スレッド割当・GPU デバイス種別等）自体が根因のヒントになり得る。逆に実際は落ちているが未観測なら Requirement 2 AC3 の追加是正対象が増える |

---

## 5. Research Needed（設計フェーズへの持ち越し）

1. **bisect 一発判定**（Requirement 3 AC1 必須）: `68bd2e3e~1`（`31d5fe71`）をビルドし最小再現ペアを実走。H-env/H-code の確定。
2. **クラッシュ実スタック取得**（Requirement 3 AC2 必須）: cdb/WinDbg 配下で最小ペアを実行し AV 発生箇所（WUC 生成時／schedule.run 中／前 world teardown 由来）を特定。
3. **切り分け実験**: (a) 2個目を「生成のみ」に縮めた場合の生死 (b) 1個目 world を `mem::forget` した場合の生死（teardown 犯人説の直接検証）(c) `DispatcherQueueController::ShutdownQueueAsync` を明示発行した場合の生死（Option A の効果の事前検証）。
4. **他バイナリへの波及有無の実測**（Requirement 2 AC3）: `crates/areka-emo-text/tests/draw_readback_test.rs`（同一プロセス内2回 WUC 生成が構造的に存在）を優先して実走確認。`emo2_fixture_e2e_test.rs`／`attach_wiring_test.rs`／`wintf/tests/visual.rs` も要選別。
5. **areka bin 側の独立 bisect**: wintf graphics と areka bin のクラッシュが本当に同一原因か（`WicCore` 経由の有無で動線が異なる可能性）を個別に確認。
6. **環境情報の記録**（H-env 裏取り）: GPU/ドライバ版数・直近 Windows Update 履歴。

---

## 6. Design-Decision Items（requirements discussion への申し送り）

1. bisect 結果（H-env 確定 vs H-code 確定）に応じて Requirement 4 の分岐（本番リスク是正 vs テストハーネス是正）がどちらへ倒れるか——コード調査は H-env 寄りの物証を追加したが、開発者判断としての最終確定は bisect 実測後。
2. Option A（根因修正）の介入範囲: `WucGraphicsResource` 単体への `Drop`/`shutdown()` 追加に留めるか、`WinApp` 終了経路（`runtime/mod.rs`）まで含めて本番の明示 teardown を新設するか。
3. 回帰テスト（Requirement 5）の実装粒度: 既存 `wuc_graphics_resource_lifecycle`（同一関数内）を拡張するか、`tests/graphics/` に「2つの独立 `#[test]` 相当」を模した新規統合テストを追加するか（libtest のスレッド分離を実際に再現する必要がある点に注意）。
4. ~~`draw_readback_test.rs` 等の追加候補バイナリが実際にクラッシュした場合、Requirement 2 AC3 の是正をこの spec のスコープに含めるか、別 spec へ切り出すか~~ — **✅ 解決済み（requirements discussion 議題1・開発者裁定 2026-07-24）**: 本 spec は全並行開発を閉塞するブロッカー解消 spec であり早期解決が優先、スコープ拡大許容。多重 WUC 生成構造を持つ全テストバイナリ（`areka-emo-text` 含む）の検証＋クラッシュ確認時の緑化までを本 spec 内で完遂する。除外は各クレートの本番ソースコード変更のみで、テストコード（テストハーネス）是正は全クレートで許容（requirements.md Boundary Context / Requirement 2 AC3-AC4 に反映済み）。
