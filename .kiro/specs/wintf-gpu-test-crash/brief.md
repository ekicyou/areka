# Brief: wintf-gpu-test-crash

> **Discovery 実施**: 2026-07-24・sylphya 実装セッション（worktree `claude/areka-p0-sylphya-fc9b81`）にて。
> areka-P0-sylphya の DoD ゲート（Task 10.2 `cargo test --workspace`）で初観測し、境界外・pre-existing と判定して切り出した専用 spec の起票ブリーフ。**実施は別セッション**（本ブリーフが引き継ぎの正本・診断は全て本セッションで実測済み）。

## Problem

`cargo test -p wintf --test graphics` が **STATUS_ACCESS_VIOLATION (0xc0000005) で確定クラッシュ**する（flake ではない・100% 再現）。`cargo test --workspace` はこの 1 点だけで exit 非 0 となり、**全 spec の kiro-complete DoD Test Gate（workspace 緑）が閉塞**している。直近では areka-P0-sylphya の完了が「feature 5 クレート緑・workspace 一括だけ赤」という条件付き判定を強いられた。放置すれば以後のすべての spec 完了が同じ星取り注記を引きずる。

さらに根因法則（下記）が示すとおり、これは単なるテスト基盤問題に留まらない可能性がある——**「同一プロセスで WUC グラフィックススタックを 2 度生成すると死ぬ」**なら、将来のゴースト再ロード／シェル切替（プロセス内 GPU スタック再生成）が本番で同じ AV を踏む。

## Current State（本セッション実測・全て worktree `areka-p0-input-events-ec190f` バイナリ `graphics-75e8573b1af088ab.exe`・2026-07-24）

### 確定した再現法則

**「同一プロセスで 2 個目の WUC スタック（`setup_world()`＝`GraphicsCore::new()`＋`WucGraphicsResource::new()`）を生成するテストは、単一スレッド逐次でも必ず STATUS_ACCESS_VIOLATION」**。1 個目は常に緑・2 個目のテスト実行中に落ちる。

診断マトリクス（すべて `--test-threads=1`・`--exact`）:

| # | 実行内容 | 結果 |
|---|---|---|
| (1) | `clip_sync_clears_clip_when_clip_is_none` **単独** | ✅ ok（0.08s） |
| (4) | `applies_all_clip_shape_variants` → `clears_clip_when_clip_is_none`（WUC 2 連続） | 💥 AV |
| (6) | `applies_all_clip_shape_variants` → `clears_clip_when_size_is_zero`（2 個目を差替） | 💥 AV |
| (8) | `clears_clip_when_clip_is_none` → `clears_clip_when_size_is_zero`（**applies_all 抜き**の WUC 2 連続） | 💥 AV |
| (7) | `brushes_system_test::solid_brushes_kept...`（WUC 非生成）→ `clears_clip_when_clip_is_none` | ✅ ok 2 passed |
| — | フルスイート 91 テスト逐次 | 💥 12 番目（＝プロセス 2 個目の WUC 生成テスト）で AV |
| — | フルスイート並列（既定スレッド数） | 💥 AV（sylphya DoD ゲートでの初観測形） |

- 最小再現ペア: `cargo test -p wintf --test graphics -- --test-threads=1 --exact clip_sync_system_test::clip_sync_applies_all_clip_shape_variants clip_sync_system_test::clip_sync_clears_clip_when_clip_is_none`
- **特定テスト非依存**（(6)(8) で一般化済み）。brushes 系（WUC を作らない）は無害（(7)）。
- クラッシュは 2 個目テストの**実行中**（`test ... ` 出力後・ok 到達前）。panic ではなくプロセス AV（バックトレース無し）。

### 影響範囲の拡大（2026-07-24 追記・sylphya 完了マージ時に観測）

`areka-P0-seriko-loop` が main へ着地（`5eeb9d01`）して **areka bin テストにも同じ AV が波及**した。seriko-loop はまばたき e2e で 2 本目の GPU world 生成 spine テストを追加しており、**`spine_e2e_kero_blink_one_cycle_golden`（プロセス 2 本目の live GPU spine）で `STATUS_ACCESS_VIOLATION`**。直前の `spine_blink_smoke_send_tick_drives_loop_pattern_command`（1 本目）は緑＝**「2 個目の WUC スタックで死ぬ」法則と完全に一致**。

- 実測（`--test-threads=1`）: worktree のマージ結果／**main チェックアウト単体（`5eeb9d01`・マージ抜き）とも同一テストで同一 AV**＝マージ起因ではなく main 既存。
- 結論: 本バグの影響は wintf `graphics` バイナリに閉じず、**GPU world を 2 個以上作るあらゆるテストバイナリ**へ及ぶ。`cargo test --workspace` の赤は今や 2 バイナリ（wintf graphics ＋ areka bin）。
- **優先度の引き上げ根拠**: 新機能が GPU テストを 1 本足すたびに別のバイナリが落ちる＝**DoD ゲートの侵食が進行中**。

### main でも再現＝pre-existing の直接証明

main チェックアウト（`C:\home\maz\git\areka`・ブランチ main・**別バイナリ** `graphics-f34527c40075921b.exe`）で同一最小ペアを実走 → **同一 AV**。かつ sylphya ブランチは wintf を一切変更していない（`git diff --name-only main...HEAD` に wintf 皆無・wintf が使う workspace 依存も無改変）。

### タイムライン（重要・regression 窓）

| 日時 (JST) | 事象 |
|---|---|
| 2026-07-23 16:06 | main へ `68bd2e3e`（areka-P0-log-capture-determinism）マージ。コミットメッセージに「**`cargo test --workspace` × 5 連続で failed 0・0xC0000005 0・3488 passed 完全一致**」の検証記録＝**この時点で graphics 91 テスト（WUC 複数 world 含む）は緑だった** |
| 2026-07-23 16:23 | mayuna-compose PR#81 マージ（DoD 緑前提） |
| 2026-07-24（本セッション） | wintf graphics が**最小 2 テストで 100% AV**。間に wintf を触る repo 変更は存在しない |

→ repo 無変更のまま「×5 連続緑」→「100% AV」へ転じた。**環境ドリフト（Windows Update・GPU ドライバ更新・WARP/コンポジタ側の変化等）が最有力仮説**。

### 仮説と対抗仮説

- **H-env（最有力）**: マシン環境の変化により、WUC スタック（Compositor／DispatcherQueueController・`DQTAT_COM_NONE`）の「生成→drop→再生成」がプロセス内で破綻するようになった。1 個目の world teardown がプロセスグローバル状態（DispatcherQueue／composition DLL 内部）を汚し、2 個目の生成 or 操作が AV。
- **H-code（対抗・要 bisect）**: `68bd2e3e` は wintf 本番 `WicCore` に `CoIncrementMTAUsage`（**プロセス寿命 MTA 常駐**）を導入した——皮肉にも「借り物 MTA 解体 × WIC factory の use-after-free（0xC0000005・~13% flake）」という**同種クラッシュの根治**として。コミット時検証では緑だったが、この常駐 MTA が WUC teardown と環境要因の組合せで新たな teardown 順序問題を作った可能性はゼロではない。**判定法**: `git checkout 68bd2e3e~1`（=`31d5fe71`）で最小ペアを実走。緑なら H-code（68bd2e3e が犯人）・AV なら H-env 確定。
- 参考: テスト側は各 `#[test]` が libtest の**別スレッド**で `CoInitializeEx(MULTITHREADED)`＋フル GPU スタックを新規生成する構造（`clip_sync_system_test.rs` の `setup_world()`。graphics 配下の他ドメインテストも同型が多い）。

### 調査の第一手（新セッション向け・順序どおり）

1. **bisect 一発判定**: `68bd2e3e~1` をビルドし最小ペア実走（H-env vs H-code の分岐点）。
2. **クラッシュ点の特定**: 最小ペアを WinDbg/cdb 配下で実行（`cdb -g -G -o target\debug\deps\graphics-*.exe --test-threads=1 --exact ...`）し AV の実スタックを取得（WUC 生成時か・schedule.run 中か・前 world の teardown 由来か）。または `WerFault` フルダンプ有効化。
3. **切り分け実験**: (a) 2 個目を「生成のみ・schedule.run 無し」に縮めて生成時点で死ぬか確認 (b) 1 個目の world を `std::mem::forget`（drop 抑止）して 2 個目が通るか——teardown 犯人説の直接検証 (c) DispatcherQueueController の明示 Shutdown 有無。
4. 環境情報の記録: GPU/ドライバ版数・直近 Windows Update 履歴（H-env の裏取り）。

## Desired Outcome

1. `cargo test -p wintf --test graphics` が**全 91 テスト・並列既定設定で決定論的に緑**（＝`cargo test --workspace` exit 0 が復活し、全 spec の DoD Test Gate が再び機能する）。
2. 根因が特定・記録され、「プロセス内 WUC スタック再生成」の可否が**本番設計への含意として明文化**される（ゴースト再ロードが再生成を要するなら本番コードの寿命管理も是正、テストだけの都合なら共有 fixture 化等の基盤是正で良い——この分岐は根因判明後に requirements で確定）。
3. 再発を檻に入れる回帰テスト（例: 「WUC world 2 連続生成が緑」という最小ペアそのものを恒久檻化）。

## Approach（候補・最終確定は新セッションの requirements/design で）

- **A. 根因修正（本命・root-cause-first）**: bisect＋実スタックで根因を特定し、wintf の WUC リソース寿命管理（DispatcherQueueController の明示 shutdown・Compositor drop 順序・COM apartment 整合）を是正する。Pros: 本番のゴースト再ロード耐性も獲得・DoD 恒久復旧。Cons: COM/WinRT teardown のデバッグは重い。規模: 中。
- **B. テストハーネス共有 fixture 化（緩和策）**: graphics スイートをプロセス内 1 個の共有 WUC スタック（`OnceLock`）に載せ替え、再生成自体を回避。Pros: 速い・Defender 知見（共有 hardlink fixture）と同系の定石。Cons: **根因を隠すだけ**——本番再生成問題が残る可能性・テスト独立性低下。規模: 小。
- **C. プロセス分離（外形回避）**: 91 テストをドメイン別バイナリへ分割 or 1 テスト 1 プロセス実行化。Pros: 確実に緑。Cons: ビルド時間・規律逸脱（cargo test 定石から外れる）・根因未解明。規模: 小-中。
- **推奨**: **A を主・B を A の知見確定後の設計選択肢として保持**（根因が「本番も踏む寿命バグ」なら A 一択・「テストだけの環境制約」と判明したら B へ縮退可）。C は最後の避難路。

## Scope

- **In**: wintf graphics テストスイートの AV 根因特定・修正・回帰檻・（根因次第で）wintf の WUC リソース寿命管理是正・DoD ゲート（workspace 緑）の復旧確認
- **Out**: areka 側クレートの変更（sylphya/kanade/ghost/parsers/bin は既に緑・無関係）・WUC 以外のレンダリング機能追加・graphics テストの網羅拡張

## Boundary Candidates

- wintf `ecs/graphics/wuc_resource.rs`（WucGraphicsResource 生成/破棄）・`com/wuc.rs`（interop）・`GraphicsCore` 寿命
- `tests/graphics/` ハーネス構造（`setup_world()` パターンの共通化是非）
- （bisect 結果次第）`ecs/widget/bitmap_source/wic_core.rs` の `CoIncrementMTAUsage` 常駐との相互作用

## Out of Boundary

- areka-P0-sylphya の完了処理（本件と独立・10.3 人間サインオフ待ちで別トラック）
- kanade/ghost の協調ループ・Defender 飢餓 flake（既知・別知見で管理済み）
- emo2 実機系（`AREKA_EMO2_REAL_RUN`）・32bit SHIORI 系

## Upstream / Downstream

- **Upstream**: `wintf-dcomp-to-wuc-migration`（completed・現 WUC スタックの導入元・clip_sync テスト最終改変者）／`areka-P0-log-capture-determinism`（completed・`68bd2e3e`・bisect 対象・WicCore MTA 常駐導入）
- **Downstream**: **全 spec の kiro-complete DoD Test Gate**（workspace 緑の回復が直接の便益）／将来のゴースト再ロード・シェル切替機能（プロセス内 GPU スタック再生成の可否確定に依存）

## Existing Spec Touchpoints

- **Extends**: なし（新規境界。wintf 側の独立メンテナンス spec）
- **Adjacent**: `areka-P0-sylphya`（発見元・tasks.md Implementation Notes 10.2 に初観測記録）／`completed/areka-P0-log-capture-determinism`（同種 0xC0000005 根治の前例＝手法参考・かつ bisect 容疑）

## Constraints

- 実 GPU/WUC を要する検証はこの開発機ローカル実行（外部 CI 無し・`cargo test` 定石＝memory `areka-no-ci-gpu-tests-in-cargo-test`）
- WUC はスレッド親和・`DQTAT_COM_NONE`＝MTA で動く（memory `areka-wuc-runs-on-mta-thread`）——修正は本規約と整合させること
- 修正は決定論必達（`×N` 連続緑で判定・flake 許容せず）・log-first（無音失敗禁止）
- **本ブリーフは sylphya worktree ブランチ上で起票**——新セッションが main から分岐する場合、sylphya PR マージ後に見えるようになる（それまでの参照はメモリ `wintf-gpu-test-crash-discovery` とこのパス直読で可能）
