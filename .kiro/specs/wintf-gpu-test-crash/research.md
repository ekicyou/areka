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

---

# 設計フェーズ追記（kiro-spec-design・2026-07-24）

## Summary
- **Feature**: `wintf-gpu-test-crash`
- **Discovery Scope**: Extension（既存 WUC ライフサイクルの調査・是正）＋ Complex Investigation（根因未確定のため調査フェーズ先行の phase-gate 設計）
- **Key Findings**:
  - 多重 WUC 生成バイナリの全数インベントリを確定（下記）。gap analysis の候補（wintf graphics／areka bin／areka-emo-text tests）に加え、**`wintf --lib`（in-source テスト）・`wintf --test visual`・`areka-emo-text --lib`（`src/actor.rs` の in-source テスト 5 箇所）** も同一プロセス多重 WUC 生成構造を持つことを grep 実測で確認。
  - `DQTYPE_THREAD_CURRENT`（`com/wuc.rs:126-135`）により DispatcherQueue は**生成スレッド束縛**。libtest は `#[test]` ごとに別スレッドを spawn するため、Path B（共有 fixture）は素朴な `OnceLock` 共有ではスレッド親和性違反になる——**専用オーナースレッド＋クロージャ marshal 型**（B1）が安全形。
  - `wuc_spike.rs:162-191` に `ShutdownQueueAsync` 発行→`Status()` ポーリングドレイン→controller 最後 drop の実証済みパターンが存在。Path A の実装テンプレートとして流用可能。
  - `WucGraphicsResourceInner`（`wuc_resource.rs:27-34`）は宣言順 drop のみで明示 shutdown なし——gap analysis 所見を再確認。

## Research Log（設計フェーズ）

### 多重 WUC 生成テストバイナリの全数インベントリ（Requirement 2 AC3 の検証対象母集団）
- **Context**: R2 AC3/AC4 は「同一プロセス内 2 個以上の WUC/GPU スタック生成構造を持つ全テストバイナリ」の実測検証と緑化を要求する。
- **Sources Consulted**: `WucGraphicsResource::new(` の全 crate grep（2026-07-24・本 worktree）。
- **Findings**（バイナリ単位・呼出箇所つき）:
  1. `wintf --test graphics` — 8 ファイルが WUC 生成（`clip_sync_system_test.rs:26`・`components_test.rs:29`・`window_pos_systems_test.rs:103`・`reinit_unit_test.rs:58,95`・`dcomp_integration_test.rs:24`・`surface_systems_test.rs:37`・`surface_pixel_equivalence_test.rs:303`）＝クラッシュ確認済み（brief）
  2. `areka` bin（in-crate テスト）— `src/emo2_boot/spine.rs:257`（`make_world_with_gpu()`）＝クラッシュ確認済み（brief・`spine_e2e_kero_blink_one_cycle_golden`）
  3. `wintf --test visual` — `tests/visual/common/mod.rs:45`（共有ヘルパー）＋`graphics_auto_creation_test.rs:31`＝**未実測**
  4. `wintf --lib` — in-source テスト: `src/ecs/graphics/wuc_resource.rs:160,189`・`src/ecs/graphics/tests.rs:52,89`・`src/com/wuc.rs`（tests 内で `create_dispatcher_queue_controller`＋`Compositor` を 2 テスト）＝**未実測**
  5. `areka-emo-text --lib` — `src/actor.rs:1181,1268,1365,1539,2233`（in-source テスト 5 箇所）＝**未実測**
  6. `areka-emo-text` 統合テスト各バイナリ — `draw_readback_test.rs:101`（ヘルパー・`#[test]` 2 個が呼ぶ）・`emo2_fixture_e2e_test.rs:241`・`attach_wiring_test.rs:52`・`choice_fixture_test.rs:162`・`viewbox_scroll_test.rs:98`＝**未実測**（バイナリごとに WUC 生成 `#[test]` 数の選別要）
  7. `areka-emo-present --lib` — `src/presenter.rs:531`＝**未実測**（1 箇所・多重かは `#[test]` 数次第）
- **Implications**: 検証マトリクス（design.md C6）はこの 7 グループを母集団とし、`cargo test --workspace`（R2 AC1）で総括判定する。未実測バイナリが現状緑なら「なぜ graphics だけ落ちるか」の差分自体が根因ヒントになる（gap analysis §4 の指摘どおり）。

### Path B の実現形とスレッド親和性
- **Context**: 共有 fixture 化（Approach B）が libtest のテスト毎スレッド分離と両立するかの構造確認。
- **Findings**: `DQTYPE_THREAD_CURRENT` により DispatcherQueue／Compositor は生成スレッドに束縛。`OnceLock<World>` の素朴共有は「fixture 生成スレッド ≠ 利用テストスレッド」となり WUC スレッド親和規約（memory `areka-wuc-runs-on-mta-thread`・`unsafe impl Send/Sync` の SAFETY 条件「同時呼び出しなし」）を破る恐れがある。
- **Implications**: B 採用時は **B1: 専用オーナースレッド常駐＋チャネルでクロージャを marshal して実行**（GPU 操作は常に同一スレッド）を正とする。副次効果として GPU テストの直列化（並列実行での多重生成も構造的に消える）。

### 回帰檻（R5）と Path B の緊張関係
- **Context**: R5 AC1/AC2 は「同一プロセス内 WUC スタック 2 回以上連続生成」する回帰テストが**安定成功**することを要求。B は生成回避策であり、素の再生成が依然クラッシュするなら檻が立たない。
- **Findings**: 切り分け実験 (c)（明示 `ShutdownQueueAsync` ドレイン挿入で 2 個目が通るか）が緑なら、「処方された安全再生成プロトコル」を檻テスト内で常用することで B 採用下でも R5 を充足できる。実験 (c) を含む全緩和が不成立（環境が再生成を全面禁止）の場合、R5 AC1+AC2 は**いかなる設計でも充足不能**＝要件エスカレーション事由。
- **Implications**: 判定ゲート G1 に「安全再生成プロトコル成立/不成立」を明示出力として組み込み、不成立時は設計内で糊塗せずエスカレーションする（design.md C2）。

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Phase-gate（調査先行→判定→条件付き是正） | R3 の bisect・cdb・切り分け実験を必須先行させ、判定ゲート G1 で A/B/C を選択 | R3/R4 の要件構造（根因確定が修正選択の前提）に 1:1 で適合・憶測実装を構造的に排除 | 調査結果次第で実装範囲が変動（File Structure Plan が条件分岐） | **採用**。requirements discussion で root-cause-first が明示裁定済み |
| 即時 Path B 先行（緑化優先→根因調査後追い） | 共有 fixture でまず workspace 緑へ | 最速で DoD 閉塞解除 | R3 AC1/AC2・R4 AC1 の宣言なしに修正を選ぶことになり要件違反・根因を隠す | 棄却（brief・requirements とも root-cause-first を明記） |
| 即時 Path C（プロセス分離） | バイナリ分割で前提条件を消す | 確実に緑 | R3/R5 未充足・`structure.md` テスト入口固定化規約から逸脱 | 最終避難路としてのみ保持 |

## Design Decisions（設計フェーズ）

### Decision: Phase-gate 構造（調査 3 フェーズ→判定ゲート G1→条件付き是正）
- **Context**: 根因が未確定（H-env vs H-code・クラッシュ点未特定）のまま修正経路 A/B/C を静的に確定できない。要件（R3→R4→R1/R2/R5）が「調査→宣言→是正」の順序を規定している。
- **Alternatives Considered**: 上記 Architecture Pattern Evaluation の 3 案。
- **Selected Approach**: Phase 0（環境記録）→ Phase 1（bisect `68bd2e3e~1`）→ Phase 2（cdb/WinDbg 実スタック）→ Phase 3（切り分け実験 a/b/c）→ G1（原因分類宣言＋経路選択＋安全再生成プロトコル確定）→ Phase 4（是正実装 A/B）→ Phase 5（検証マトリクス＋回帰檻＋×5 安定確認）。
- **Rationale**: R3 AC1（bisect）・AC2（スタック証拠）は G1 の入力そのもの。R4 AC2/AC3 の分岐は G1 の出力。設計はこのワークフローと両経路のコンポーネント設計を確定し、経路選択のみ実装時証拠に委ねる。
- **Trade-offs**: tasks.md は条件付きタスク（経路別）を含むことになるが、判定基準を設計で客観化することで実装時の裁量を排除。
- **Follow-up**: G1 の判定証拠は本 research.md「根本原因記録」節（実装フェーズで新設）へ記録。

### Decision: Path A の介入点＝`WucGraphicsResourceInner::drop` への有界ドレイン内蔵＋明示 `shutdown_blocking()` 併設
- **Context**: gap analysis が特定した唯一の実装済み安全パターン（`wuc_spike.rs`）をどこへ載せるか。呼び出し側 91+ テストへの個別 teardown 追加は漏れが必然。
- **Alternatives Considered**:
  1. 全 `setup_world()` 系ヘルパーへ明示 teardown 追加 — 14+ 箇所・漏れリスク大
  2. `Inner::drop` に自動ドレイン内蔵 — 全経路（テスト・本番・invalidate）を無改修でカバー
- **Selected Approach**: 2 を主とし、観測可能な明示 API `shutdown_blocking(timeout)` を併設（回帰檻・本番終了経路・実験 (c) が使用）。ドレインは有界タイムアウト＋log-first（無音失敗禁止）・panic 中 drop でも panic-free。
- **Rationale**: drop は生成スレッド上で起きる（テスト＝テストスレッド・本番＝UI スレッド）ため `DQTYPE_THREAD_CURRENT` 束縛と整合。既存呼び出し側は無改修で恩恵を受ける＝R2 の「wintf 根因修正の波及で緑化」期待経路と一致。
- **Trade-offs**: drop 内メッセージポンプは実行時間を数 ms 増やす（91 テストで最大数百 ms・許容）。タイムアウト保険でハング不能を保証。
- **Follow-up**: 実験 (c) の結果が「ドレインでも 2 個目死亡」なら本 Decision は棄却され G1 で再判定。

### Decision: 回帰檻の正準形＝「檻専用ファイルに 2 独立 `#[test]` ＋ 単一テスト内再生成」の 2 形態併設
- **Context**: R5 AC1「単一テストプロセス内で 2 回以上連続生成」・AC3「再発時に必ず失敗」。libtest 並列実行下では「プロセス内 2 個目」がどのテストに当たるか非決定。
- **Selected Approach**: `crates/wintf/tests/graphics/wuc_restart_regression_test.rs` に (i) 各々フルスタック生成→最小合成操作→teardown する独立 `#[test]` 2 本（どちらが 2 個目になっても法則を踏む）、(ii) 同一 `#[test]` 内で 生成→teardown→再生成→最小操作 の決定論形、の 2 形態を置く。B 採用時もこのファイルは共有 fixture を**意図的にバイパス**する（檻の意味を保つ）。
- **Rationale**: (i) は実際のクラッシュ様式（別テスト実行単位・別スレッド）の忠実な再現、(ii) は単独実行でも決定論的に再生成を検証。既存 in-source `wuc_graphics_resource_lifecycle`（同一スレッド内）では捕捉できない領域を覆う（gap analysis §2 R5 行）。
- **Follow-up**: 檻内の teardown 手順は G1 で確定する「安全再生成プロトコル」を常用する。

## Risks & Mitigations（設計フェーズ追記）
- 環境が WUC 再生成を全面禁止（全緩和不成立）— G1 でエスカレーション条項を発動し R5 の要件再交渉へ（設計内で糊塗しない）
- Path B1 の直列化で graphics スイート実行時間が増加 — 91 テストの GPU 部分は元々 UI スレッド固定相当の逐次性・許容範囲を Phase 5 で実測確認
- `bisect` 用旧リビジョンのビルドが現環境で成立しない（依存ドリフト）— lockfile 固定（`Cargo.lock` はリポジトリ管理下）で再現・失敗時は brief のタイムライン証拠（コミットメッセージの ×5 緑記録）を代替証拠として記録
- 未実測バイナリ（`wintf --lib`／`--test visual`／`areka-emo-text` 系）で新たな赤が発覚 — スコープ拡大は裁定済み（テストコード是正は全クレート許容）・検証マトリクスで早期に全数実測

## References
- `crates/wintf/examples/wuc_spike.rs` — `ShutdownQueueAsync` ドレインの実証済みパターン（Path A テンプレート）
- `crates/wintf/src/com/wuc.rs:126-135` — `DQTYPE_THREAD_CURRENT`（スレッド束縛の根拠）
- `.kiro/specs/wintf-gpu-test-crash/brief.md` — 診断マトリクス・タイムライン・仮説の正本
- memory `areka-wuc-runs-on-mta-thread`／`areka-no-ci-gpu-tests-in-cargo-test`／`areka-log-first-no-silent-failure` — 制約の出典

---

# 根本原因記録（実装フェーズ・調査 Phase 0–3 と判定ゲート G1・2026-07-24）

> 本節が Requirement 3.1–3.3 / 4.1 の証拠・宣言の**単一の正本**。以下すべて本実装 worktree（`claude/kiro-gpu-test-crash-6a952a`・バイナリ `graphics-56cbb62983609269.exe`）で実測。cdb/WinDbg は本開発機に未インストールのため、クラッシュ点特定は Requirement 3.2 が明示許容する**切り分け実験結果**を裏付け証拠として用いた。

## 結論（要約）

**根本原因＝H-env（環境要因）確定。クラッシュ点＝2 個目の `Compositor::new()`（生成時）。トリガ条件＝「同一プロセス内で、別スレッド上に 2 個目の MTA 束縛 `Compositor`（`DQTAT_COM_NONE`）を生成すること」。同一スレッド上の逐次再生成（生成→drop→再生成）は安全。1 個目スタックの teardown 有無・明示ドレイン有無とは無関係。**

**G1 宣言＝(b) テストハーネス固有。選択経路＝Path B（SharedGpuFixture・専用オーナースレッド）。** 保守側既定則の適用はなし（証拠は分類可能・非競合）。

## Phase 0: 環境記録と修正前ベースライン

### 環境情報（H-env 裏取り）
| 項目 | 値 |
|---|---|
| GPU | Intel(R) Arc(TM) Graphics |
| GPU ドライバ | 32.0.101.6314（DriverDate 2024-11-26） |
| OS | Windows 11 26200.8894（25H2・UBR 8894） |
| 直近 Update（Get-HotFix 可視） | KB5121767(2026-07-19)・KB5100998(07-15)・KB5120102(Security・07-14)・KB5054156(2025-10-18) |

**所見**: brief のタイムライン（2026-07-23 ×5 連続緑 → 07-24 100% AV）の回帰窓の間に **Get-HotFix 可視の Windows Update は 1 件も無い**（最新は 07-19）。GPU ドライバも 2024-11-26 据え置き。H-env の「トリガ」は Get-HotFix 非可視のコンポーネント更新／ドライバランタイム状態のドリフト、または WUC/composition ランタイムの累積状態と推定される（本 spec は根因の是正であり、環境ドリフトの発生源特定までは範囲外）。

### 修正前ベースライン実測表（C6 マトリクス母集団・7 グループ）
| # | 対象バイナリ | 実測コマンド | 修正前結果 |
|---|---|---|---|
| 1 | wintf graphics | `cargo test -p wintf --test graphics -- --test-threads=1 --exact <最小ペア>` | 💥 AV（0xc0000005・2 個目 `clip_sync_clears_clip_when_clip_is_none` 実行中）|
| 2 | areka bin | （brief 実測・`spine_e2e_kero_blink_one_cycle_golden`）| 💥 AV（brief 正本・本節では未再実測）|
| 3 | wintf visual | `cargo test -p wintf --test visual` | ✅ 52 passed, exit 0 |
| 4 | wintf lib | `cargo test -p wintf --lib`（`--test-threads=1` でも）| ✅ 517 passed, exit 0 |
| 5 | areka-emo-text lib | `cargo test -p areka-emo-text --lib` | ✅ 368 passed / 2 ignored, exit 0 |
| 6 | areka-emo-text draw_readback | `cargo test -p areka-emo-text --test draw_readback_test` | ✅ 2 passed, exit 0（後述の重要所見あり）|
| 7 | areka-emo-present | （未実測・Phase 4 検証マトリクスで実測予定）| — |

**差分の意味**: 落ちるのは graphics と areka bin のみ。他は現状緑。緑バイナリの共通点は「同一プロセスで**別スレッド上の 2 個目 MTA/NONE Compositor** が発生しない」こと（後述の全数分析で確認）。この差分自体が「別スレッド 2 個目」がトリガであることの傍証。

## Phase 1: bisect 一発判定（Requirement 3.1）

- 別 worktree に `68bd2e3e~1`（=`429a4ff2c9791c00cc7ecd9fbfb6ea977136eebd`・MTA 常駐 `CoIncrementMTAUsage` 導入の**直前**リビジョン）を checkout してビルドし、最小再現ペアを実走。
- **結果: 同一 AV（0xc0000005）を再現**。1 個目 `clip_sync_applies_all_clip_shape_variants` ok → 2 個目 `clip_sync_clears_clip_when_clip_is_none` 実行中に AV。
- **判定: H-env 確定**。`68bd2e3e`（MTA 常駐）は犯人ではない（H-code 棄却）。gap analysis のコード物証（graphics バイナリは `WicCore`／`wic_core`／MTA 常駐経路を一切参照しない）と完全整合。

## Phase 2: クラッシュ点の特定と分類（Requirement 3.2）

cdb/WinDbg 未インストールのため、切り分け実験（Phase 3）の行動論的証拠でクラッシュ点を分類した（Requirement 3.2 は「デバッガのスタックトレース**または**切り分け実験結果」を許容）。

**クラッシュ点＝「WUC スタック生成時」、具体的には 2 個目の `Compositor::new()`**。分類根拠（`eprintln` プローブによる到達点特定）:
- 2 個目テストで `GraphicsCore::new()`（D3D11/D2D/DWrite）は**成功**（`after GraphicsCore::new` 到達）。→ D3D/D2D コアは 2 個目でも健全。
- `WucGraphicsResource::new()` 内で: `create_dispatcher_queue_controller(DQTAT_COM_NONE)` は**成功**（`before Compositor::new` 到達）→ `Compositor::new()` で**クラッシュ**（`before create_graphics_device` 未到達）。
- したがってクラッシュ点は「前 world の teardown 由来」でも「schedule 実行中」でもなく、**2 個目 `Compositor::new()` の生成時**。

## Phase 3: 切り分け実験（a）（b）（c）＋統制スレッド実験

すべて一時変更（最終ツリーへは未コミット・`git restore` 済み）。

| 実験 | 内容 | 結果 | 含意 |
|---|---|---|---|
| (a) 生成のみ | 2 個目テストを `setup_world()` のみに縮小（schedule/spawn 無し）| 💥 AV（`after setup_world` 未到達）| クラッシュは**生成時**（操作時でない）|
| (b) teardown 抑止 | 1 個目 world を `std::mem::forget`（drop 完全抑止・Compositor 生存維持）| 💥 AV（2 個目なお死亡）| **teardown 犯人説を棄却**。1 個目の生死は無関係 |
| (c) 明示ドレイン | 1 個目 `Inner::drop` に `ShutdownQueueAsync` 発行＋ポンプドレインを内蔵（`wuc_spike` パターン）| 💥 AV（ドレインは `drain done` まで完走・なお 2 個目死亡）| **明示ドレイン（Path A の中核前提）は無効**。design.md C3 の「ドレインで治る」前提が崩壊 |
| 統制: 同一スレッド 2 個 | 1 スレッドで Compositor 生成→drop→再生成 | ✅ PASS | **同一スレッド逐次再生成は安全** |
| 統制: 別スレッド 2 個 | スレッド A 生成+drop(join)→スレッド B 生成 | 💥 AV（B の `Compositor::new` で死）| **別スレッド 2 個目 Compositor が真のトリガ**（A を完全 join 後でも死ぬ）|

**補強証拠（in-source テスト）**: `wuc_graphics_resource_lifecycle`（`wuc_resource.rs`・`--lib`）は**同一スレッド上で** `WucGraphicsResource::new` を 2 回呼ぶ（invalidate/drop を挟む）が緑。`--lib` に MTA/NONE Compositor テストは実質これ 1 本のみ（他は ASTA 別アパートメント or 非 WUC）ゆえ「別スレッド 2 個目」が発生せず緑を保つ——Phase 0 ベースラインの差分を機序で説明する。

**特記所見（emo-text draw_readback の緑）**: draw_readback は `make_world_with_gpu`（graphics `setup_world` と同一パターン）を 2 本の `#[test]` が別スレッドで呼ぶが緑。ところが**同一バイナリに素の「別スレッド 2 個 Compositor」プローブを注入すると AV する**。差分は実テストが `MessageLoop`／`pump_until_idle` で**メッセージポンプを回す**こと。DispatcherQueue の静止化が別スレッド 2 個目の生死を左右する示唆だが、素のプローブが同バイナリで死ぬ以上この緑は**偶発的・脆弱**であり、是正の基盤には据えない（Phase 4 検証マトリクスで全数の緑を担保する）。

## 判定ゲート G1（Requirement 3.3, 4.1）

### 宣言: **(b) テストハーネス固有**
- **根拠**: 本番 `WinApp` は**単一 UI スレッド上に単一 Compositor**を持つ（memory `areka-wuc-runs-on-mta-thread`・`areka-concurrency-model`）。本番は「別スレッド上の 2 個目 Compositor」を生成しない。多重生成は libtest が `#[test]` ごとに別スレッドを spawn する**テストハーネス構造に固有**。同一スレッド逐次再生成（本番のゴースト再ロードが踏む経路）は実測で安全。
- **保守側既定則（分類不能時の宣言 (a) フォールバック）は不適用**: 証拠は分類可能かつ非競合（H-env 確定・クラッシュ点特定・トリガ=別スレッド 2 個目・同一スレッド安全）。design.md G1 表の既定則行の発動条件（分類不能・相互競合）に該当しない。

### 選択経路: **Path B（SharedGpuFixture・専用オーナースレッド）**
- **Path A を選ばない理由**: design.md C3 の Path A（`Inner::drop` 有界ドレイン＋`shutdown_blocking`）は実験 (c) で**無効と実証**。teardown/ドレインは根因（別スレッド 2 個目生成）に作用しない。宣言 (b) では Path A 縮小適用または Path B を選択可（Requirement 4.3・design.md G1 表）であり、実効性のある Path B を選ぶ。
- **安全再生成プロトコル（C7 回帰檻・C6 の正準手順）**: 「WUC スタックの生成・操作・破棄はすべて**単一のオーナースレッド上**で行う」。別スレッドでの 2 個目 Compositor 生成を構造的に排除する。同一スレッド上での逐次再生成（生成→drop→再生成）は安全に許容される。

### エスカレーション判定: **不発動（通常経路で続行）**
- design.md/tasks.md 2.4 のエスカレーション条項（実験 (a)(b)(c) すべて不成立＝プロセス内再生成が環境的に全面不能）には**該当しない**。同一スレッド再生成は成立する（緩和経路が存在する）ため、Requirement 5.1+5.2 は Path B のオーナースレッド上での再生成で充足可能。
- **ただし Requirement 5（回帰檻）の形態調整が必要**（design.md C7 との差分・実装フェーズで反映）: design.md C7 の「独立 2 `#[test]`（別スレッド）」形態(i)は、素で実行すれば別スレッド 2 個目＝**必ず AV** となり緑にできない。回帰檻は**同一オーナースレッド上での逐次再生成**（design.md C7 形態(ii)相当）を正準形とし、別スレッド生成の再発は「共有 fixture をバイパスした WUC 生成 → プロセス AV → バイナリ失敗」という構造で検出保証する（Requirement 5.3 はサイレント成功が構造的に不可能な点で充足）。

## 本番設計への含意（Requirement 3.3, 4.1・将来のゴースト再ロード／シェル切替）

- **同一 UI スレッド上での WUC スタック再生成は本環境で安全**（実測: 同一スレッド 2 個 PASS・`wuc_graphics_resource_lifecycle` 緑）。ゆえに**単一 UI スレッド構成を維持する限り、ゴースト再ロード・シェル切替が WUC スタックを作り直しても本クラッシュは踏まない**。
- **危険なのは「別スレッド上での 2 個目 Compositor 生成」のみ**。本番がこの構成を採る（例: 複数 UI スレッド／ワーカースレッドでの Compositor 生成）場合は本環境で AV する。**設計不変条件として「プロセス内の WUC Compositor は単一 UI スレッド所有・別スレッドでの Compositor 生成を禁止」を維持すべき**。
- 以上より、**現時点で本番実在リスクは無い（宣言 b）**。将来の再生成機能は上記不変条件を前提に設計すること。

## 波及（brief 追記㊹エスケープ条項の発動）

Path B 採用により、本 spec は `crates/wintf/tests/graphics/*` のハーネス構造（共有 fixture）を**単独所有**する。後続 spec（**W4 emo-dpi-scaling**〔graphics テスト増設面〕・**W5 kero-balloon**〔areka `emo2_boot/spine.rs` 檻域〕）は本 spec 完了後に本構造へ **rebase 必須**（design.md Revalidation Triggers・brief 追記㊹）。
