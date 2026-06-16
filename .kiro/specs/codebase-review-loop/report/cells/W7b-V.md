# W7b-V: ECS基盤・World（ecs/{common,world}/ ＋ ecs/app.rs） × 脆弱性レビュー

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点・基準・範囲

- 観点: V（脆弱性レビュー）。基準: design.md「Security Considerations」（unsafe 境界・整数変換の切り捨て/オーバーフロー・Win32/COM ハンドルのリーク/二重解放・外部入力の検証欠如・panic 経路 DoS を点検し、**挙動を変えない範囲（内部チェック・debug_assert・不変条件/ordering コメント・安全な型置換）の対策のみ投入**。API シグネチャ/エラー応答を変える対策・整数オーバーフロー堅牢化・スケジュール順序/atomic ordering の変更は proposals.md へ）。CellExecutor V 規則（R2.3/R2.4、design.md:338）、観点順序 T→S→V（R2.7、W7b-T1/T2/S 完了済みの回帰検知器=39件の上で実行）。
- 性質: **非挙動変更タスク**（脆弱性点検＋挙動非破壊な対策のみ）。Feature Flag Protocol 不要。
- requirements（source 番号 = Requirement N の Acceptance Criteria M）: 2.3（脆弱性レビュー＋挙動非破壊対策）・2.4（挙動変更対策→提案記録）・2.5（前後 S2 非破壊）・2.7（列順 T→S→V）・2.8（テスト保護外でも深く解析・安全適用不能は提案記録）・4.1（自己レビュー＋検証）・5.1（外部観測可能挙動を変更しない）・5.2（挙動変更必要時は提案記録）。
- design: Security Considerations（design.md:512-516）、CellExecutor 観点別規則（V、design.md:338）、提案記録様式（design.md:453-461）、セル断片様式（design.md:440-447）。領域定義「W7b: wintf ECS基盤・World / `crates/wintf/src/ecs/{common,world}/`, `ecs/app.rs`」。
- 参考: `report/cells/W7b-T1.md`（common/ モジュール×テスト対応表・tree_system マルチスレッド fan-out 所見1〜3）、`W7b-T2.md`（world/＋app.rs・vsync テスト不能所見1〜3・`has_systems` ガード申し送り）、`W7b-S.md`（App derive Default 化・type_complexity/has_systems churn 回避）、`W6b-V.md`・`W4b-V.md`（V 観点・整数境界・panic 点検・debug_assert/不変条件コメント様式の先例）。
- 境界: `crates/wintf/src/ecs/{common,world}/`, `ecs/app.rs`（tests/ の該当ドメインは境界内）。`win_thread_mgr.rs`（VSYNC_TICK_COUNT/LAST_VSYNC_TICK の定義元・vsync スレッド）・`window_proc/lifecycle.rs`（mark_display_change 呼び出し元）・`layout/systems/monitor_systems.rs`（reset_display_change 呼び出し元・display フラグ消費）・`window/window_system.rs`・`layout/systems/window_pos_systems.rs`・`graphics/visual_manager.rs`・`graphics/systems/{surface,init,render}.rs`（FrameCount.0 消費元。前4者と init.rs/render.rs はログ `frame=`、surface.rs:44 は非ログの「Changed 検出ノンス格納」）には変更を加えていない（**読み取りのみ＝本番挙動の裏取り**: atomic ordering・スケジュール順序・FrameCount 消費形態の確認のため）。
- 起点: W7b-S 適用後のクリーンなワークツリー（境界に未コミット差分なし、HEAD = `055f482`）、ベースライン S2 = 1667 passed / 0 failed。

## 点検手法

境界内 6 ファイル（common: tree_iter.rs / tree_system.rs、world: mod.rs / schedule_labels.rs / vsync.rs、app.rs ＋ in-source tests）を grep（複数パターン）＋全文精読で走査した。

- panic 経路: `unwrap()` / `expect(` / `panic!` / `unreachable!` / `unimplemented!` / `todo!` / 配列・スライス添字 `[i]`
- 整数境界/フレーム時間: `as isize`/`as u32`/`as u64`/`as f64`/`as f32`/`as i32`（切り捨て・符号反転）・整数加算 `+=`/乗算 `*`・除算 `/`（ゼロ除算）・`saturating_`/`checked_`/`wrapping_`
- atomic/スケジュール: `fetch_`/`.load(`/`.store(`/`Ordering::`・`schedules.insert`/`try_run_schedule`/`set_executor_kind`/`add_systems`

本番経路の裏取り（過去セルの未確認主張による REJECTED を回避。**スケジュール順序・atomic ordering・FrameCount 消費形態を実コードで確認**）:
- **スケジュール登録順 == 実行順（実コード照合）**: `EcsWorld::new` の `schedules.insert(Schedule::new(...))`（mod.rs:74-93）と `try_tick_world` の `try_run_schedule(...)`（mod.rs:461-473）を双方 grep で抽出し、**13 ラベルが完全に同一順序**（Input → Update → PreLayout → Layout → PostLayout → UISetup → GraphicsSetup → Draw → PreRenderSurface → RenderSurface → Composition → CommitComposition → FrameFinalize）であることを確認。
- **atomic アクセスマップ（ワークスペース全 grep）**: `VSYNC_TICK_COUNT` は VSync スレッドの `fetch_add(1, Relaxed)`（win_thread_mgr.rs:359、境界外）が唯一の書き込み、メインスレッドの `load(Relaxed)`（mod.rs:498、境界内）が唯一の読み取り = **単一生産者・単一消費者**。`LAST_VSYNC_TICK` は `load`（mod.rs:499）/`store`（mod.rs:508）とも**メインスレッドのみ**（他スレッドからのアクセスはコード上ゼロ）。
- **FrameCount.0 の消費形態（ワークスペース全 grep + 精読・再 grep 実施）**: `FrameCount(pub u32)`（schedule_labels.rs:8）の `frame_count.0` 全消費を `git grep -n "frame_count.0"` で網羅列挙すると、(a) **tracing ログ `frame=` フィールド**: `window/window_system.rs:40`・`layout/systems/window_pos_systems.rs:40,54,72,90,103`・`graphics/visual_manager.rs:98,104,128`・`graphics/systems/init.rs`（多数）・`graphics/systems/render.rs:167`（render.rs:44,163 はコメントアウト済み eprintln!）、(b) **非ログの「Changed 検出ノンス格納」**: `graphics/systems/surface.rs:44` の `dirty.requested_frame = frame_count.0 as u64`（`as u64` で永続コンポーネント `SurfaceGraphicsDirty.requested_frame`（components.rs:243）へ格納）の2形態のみ。**算術上の大小比較 `<`/`>`・厳密比較 `==`・配列添字のいずれにも未使用**であることを精読で確認。surface.rs:44 の格納先 `requested_frame` の本番読み取りは `git grep -n "requested_frame"` で**ゼロ**（読み取りは `graphics/tests.rs:133,140`・`surface_optimization_test.rs:19,61`・`surface_systems_test.rs:106` のテストのみ）。用途は surface.rs:25-26 ドキュメント「フレーム番号更新方式」が示すとおり、前回値と異なることで `Changed<SurfaceGraphicsDirty>` をトリガーする**変化ノンス**であり、`render_surface` は数値ではなく `Changed` フラグに反応する。並行の `dirty.requested_frame = dirty.requested_frame.wrapping_add(1)`（surface.rs:196・`deferred_surface_creation`）も同じ Changed トリガー目的でラップ許容。

## 発見した脆弱性候補と判定

### 1. フレーム時間計算の数値境界

- **`frame_count.0 += 1`（FrameCount=u32、mod.rs:447）— 整数境界ハザードコメント適用＋堅牢化は P66**: 毎 tick +1。~60Hz で 2^32 到達に約828日の連続稼働を要する。到達時 **debug ビルドは加算オーバーフローで panic**（理論的 DoS）、release はラップ。**FrameCount.0 の消費は (1) 多数のログ `frame=` フィールドと (2) surface.rs:44 の `dirty.requested_frame = frame_count.0 as u64`（Changed 検出をトリガーする変化ノンス格納で、値自体は本番で読まれない）のみ**（上記裏取り。算術上の大小比較・厳密比較・配列添字には未使用）。後者はラップ MAX→0 でも「前回値と異なる」性質を満たすため `Changed<SurfaceGraphicsDirty>` 検出は継続する。したがってラップしても観測挙動はログ値が 0 に戻るだけで正当性に影響しない。→ **挙動非破壊の整数境界ハザードコメントを mod.rs:445 に付記**（u32 境界・828日・消費形態（ログ＋変化ノンス）・ラップ無害根拠・P66 参照、コード挙動不変）。`wrapping_add`/`saturating_add` 化は **debug panic 挙動を変える**（R5.1）ため → **P66** へ記録。現行挙動は W7b-T2 の `try_tick_world_increments_frame_count_each_call` が低カウント域で特性化済み。
- **`measure_and_log_framerate` の除算（mod.rs:416-417）— 現状安全（対策不要）**: `fps = self.frame_count as f64 / elapsed.as_secs_f64()` の除数は直前の `if elapsed.as_secs() >= 10` ガードにより **`>= 10.0` で構造的に非ゼロ**。`avg_frame_time = elapsed.as_secs_f64() * 1000.0 / self.frame_count as f64` の除数 `self.frame_count`（EcsWorld 内部 u64・mod.rs:28）は関数冒頭の**無条件 `self.frame_count += 1`（mod.rs:410）後**かつ `last_log_time=Some`（2回目以降の呼び出し）でのみ到達するため **`>= 1` で非ゼロ**。**ゼロ除算なし**。内部 u64 は 10 秒ごとに 0 リセット（mod.rs:425）されるため u64::MAX に到達せずオーバーフローなし。**対策不要**（増分先行・除数非ゼロは自己文書的でコメントも churn 回避で見送り、根拠を本断片に記録）。
- **`hwnd.0 as isize`（app.rs:26）— 現状安全（対策不要）**: `HWND.0`（`*mut c_void`）→ `isize` の provenance 保存キャストで、ポインタと isize は同幅ゆえ**切り捨てなし**。win_thread_mgr.rs:340 の `message_window.0 as isize` と同型の確立パターン。`set_message_window` は格納のみ（Win32 非呼出）。**対策不要**。

### 2. スケジュール順序の不変条件

- **登録順 == 実行順（13 ラベル）— stale な順序ドキュメントを是正（挙動非破壊）**: 上記裏取りで登録順（mod.rs:74-93）と実行順（mod.rs:461-473）が**完全一致**を実コード確認。一方 `schedule_labels.rs:11` の順序ドキュメントコメントは **stale**（"...PostLayout → UISetup → Draw → Render → RenderSurface..." と記載し、**GraphicsSetup・PreRenderSurface・FrameFinalize を欠落**、存在しないラベル "Render" を誤記）。これはスケジュール順序の不変条件を誤って記述しメンテナを誤導しうるドキュメント欠陥。→ **コメントを実行順（= `try_run_schedule` 呼び出し順と一致する不変条件）へ是正**（コメントのみ・コード/挙動不変）。「登録順と実行順が一致する」不変条件と W7b-V で実コード照合した旨を明文化。
- **UISetup のみ SingleThreaded（mod.rs:81-85）— 現状安全（対策不要）**: `set_executor_kind(ExecutorKind::SingleThreaded)` は UISetup（CreateWindowExW 等の UI スレッド固定処理）のみに適用。schedule_labels.rs の各 doc が「マルチスレッド実行可能」と明記する他スケジュールとの使い分けは妥当（Win32 ウィンドウ作成・PostQuitMessage 等メッセージループ影響処理を main 固定）。W7b-T2 の `uisetup_schedule_uses_single_threaded_executor` が特性化済み。**対策不要**。
- **依存関係の整合性 — 現状安全（対策不要）**: 各スケジュール内の `add_systems` は `.after(...)` / `.chain()` で順序制約を宣言（mod.rs:101-320）。bevy_ecs が未充足依存を実行時検出する設計で、本セルでは順序宣言の妥当性に踏み込む変更を要さない（ロジック健全、churn 回避）。

### 3. ディスプレイ構成変更時の境界条件

- **display フラグ（app.rs:30-43）— 現状安全（対策不要）**: `mark_display_change`（=true）/`reset_display_change`（=false）/`display_configuration_changed`（=identity）は**純粋 bool の set/reset/get で境界条件なし**。ライフサイクルを全 grep で追跡: **set** は `window_proc/lifecycle.rs:147`（WM_DISPLAYCHANGE 系ハンドラ・UI スレッド・境界外）、**消費+reset** は `layout/systems/monitor_systems.rs:178`（読み取り）/`:187,:276`（reset・境界外）。App は bevy Resource で `&mut World` 経由の**単一スレッド逐次アクセス**ゆえ bool への競合・原子性問題なし（set も consume も UI スレッドの同一 World 上）。フラグの set/reset/get 契約は W7b-T2 が `monitor_hierarchy_test.rs::test_display_configuration_changed_flag`（mark→true→reset→false）で特性化済み（重複回避）。set-by-WndProc / consume-by-Update の跨りは**境界外（window_proc/・layout/）**の責務。**本境界では対策不要**。
- **ウィンドウカウント整合性（app.rs:46-86）— 現状安全（W7b-T2 で特性化済み）**: `on_window_created`（+1）/`on_window_destroyed`（`saturating_sub(1)`・count==0 で PostMessageW+true）は W7b-T2 の6件（**saturating_sub アンダーフロー防止**含む）が特性化済み。モニタ追加/削除でのウィンドウ生成/破棄は `create_windows`（UISetup・境界外）駆動で、カウント増減ロジック自体は健全。**対策不要**。

### 4. panic 経路

- **app.rs / world/mod.rs / vsync.rs / schedule_labels.rs の本番 panic — ゼロ（対策不要）**:
  - `app.rs`: 本番（非 `#[cfg(test)]`）の `unwrap/expect/panic!` は**ゼロ**（grep の唯一の `expect` は test helper `entity()` の app.rs:95）。
  - `world/mod.rs`: 本番 panic ゼロ。`try_run_schedule` は `Result` を `let _ =` で破棄（mod.rs:461-473、未登録スケジュールでも panic せず）、`get_resource_mut` は `Option` を `if let Some` で処理（mod.rs:446/449）。
  - `world/vsync.rs`: `try_borrow_mut()` を `match Ok/Err`（vsync.rs:71-90）で処理し、**再入時の借用失敗を `false` で安全スキップ**（panic せず）。`unwrap/expect` なし。
  - `world/schedule_labels.rs`: 本番は純粋 derive のみ、panic ゼロ。
  - `common/tree_iter.rs`: `next` の `self.stack.pop()?`（tree_iter.rs:82）は `?` で空スタック時 None を返す**安定終端**（panic せず・W7b-T1 の `test_next_returns_none_after_exhaustion` で特性化）。配列添字なし。
- **`common/tree_system.rs` の本番 panic（assert/unwrap）— 健全な不変条件・到達不能ガード（対策不要、実コードで裏取り）**:
  - `propagate_descendants_unchecked` の `assert_eq!(child_of.parent(), parent)`（tree_system.rs:276）: unsafe な disjoint 並列ミューテーションの健全性を保証する**非循環不変条件アサート**。bevy の `ChildOf`/`Children` 双方向整合が public API（add_children 等）で維持される限り常に成立（W7b-T1 所見3 で実証済み・整合再ペアレントテストで成立側通過）。**意図的防御**。
  - `propagation_worker` の `nodes.get_unchecked(parent).unwrap()`（tree_system.rs:199）と `p_children.unwrap()`（:203）: **「キュー内の全エンティティは子を持つ」不変条件**を実コードで裏取り — outbox への push は `propagate_descendants_unchecked` の `children.map(|children| { ...; child })`（:284-288）内でのみ発生し、push される `child` は**自身の `Children` コンポーネントが `Some` のときに限る**（filter_map の `new_children` が `children.map` 経由でのみ `child` を yield、:270-290 → `outbox.extend` :291）。したがってワーカーが dequeue した親は必ず `Children` を持ち、両 unwrap は健全。これらは **512+ ノードのマルチスレッド fan-out のみ到達**（W7b-T1 所見1）かつ **bevy_transform 上流ミラー**コード。不変条件破綻時の panic は unsafe 健全性の意図的アサートで、`debug_assert` 追加は冗長（unwrap が既に debug/release 両方で発火）・上流ミラー churn（W7b-S が tree_system.rs を type_complexity/bevy ミラー保護で無変更とした方針）に反するため**コード変更せず**、不変条件の検証結果を本断片に記録。

### 5. atomic ordering の妥当性

- **`try_tick_on_vsync` の Ordering::Relaxed（mod.rs:498-508）— ordering 妥当性コメントを適用（挙動非破壊）**: 上記 atomic アクセスマップの裏取りに基づき、Relaxed の妥当性は: (a) `VSYNC_TICK_COUNT` は単一生産者（VSync `fetch_add`）・単一消費者（メイン `load`）のモノトニックカウンタで、tick 要否は「値が前回と異なるか」だけに依存し**カウンタ越しに共有データを渡さない**（実 tick は同一スレッドの `Rc<RefCell>` で逐次化、VSync スレッドとの同期は別途 `PostMessageW` のメッセージキューが担う）→ acquire/release 不要、(b) `LAST_VSYNC_TICK` は**メインスレッドのみ**が load/store するため強い順序付けは観測挙動に無影響。→ **消費側（mod.rs:try_tick_on_vsync、境界内）に ordering 妥当性コメントを付記**（コメントのみ・コード/挙動不変）。定義元 `win_thread_mgr.rs`（境界外）は読み取りのみで未変更。`VSYNC_TICK_COUNT`（u64）の周回は ~60Hz で約 9.7×10^9 年規模＝実機到達不能（堅牢化対象外、コメントに明記）。ordering 変更を要する対策は不要（現行 Relaxed が健全）ゆえ proposals 化もなし。

## 適用した挙動非破壊対策（2 ファイル・3 箇所、すべてコメントのみ＝コード不変）

| ファイル | 箇所 | 対策 | 種別 | 根拠 |
|----------|------|------|------|------|
| `world/schedule_labels.rs` | :11（順序ドキュメントコメント） | stale な実行順コメントを実コード照合済みの正しい 13 ラベル順（=`try_run_schedule` 呼び出し順＝登録順と一致する不変条件）へ是正 | 不変条件/ドキュメント是正 | コメントのみ・コード挙動不変。旧コメントは GraphicsSetup/PreRenderSurface/FrameFinalize 欠落＋存在しない "Render" 誤記でスケジュール順序不変条件を誤記述。登録順(mod.rs:74-93)==実行順(mod.rs:461-473) を実 grep で照合し是正。 |
| `world/mod.rs` | `try_tick_world`（mod.rs:445、FrameCount++ 直前） | FrameCount(u32) のオーバーフロー境界ハザードコメント（828日・debug panic/release wrap・消費はログ `frame=`＋surface.rs:44 の変化ノンス格納（本番読み取りゼロ）でラップ時も Changed 検出継続・正当性不変・P66 参照） | 整数境界ハザードコメント | コメントのみ・コード挙動不変。u32 境界と消費形態（ログ `frame=` ＋ surface.rs:44 の `requested_frame` 変化ノンス）を裏取りし、ラップ MAX→0 でも「前回値と異なる」性質で Changed 検出が継続するため正当性に無影響な根拠と、堅牢化が P66 行きである根拠を明文化。 |
| `world/mod.rs` | `try_tick_on_vsync`（mod.rs:501、atomic load 直前） | Ordering::Relaxed 妥当性コメント（VSYNC_TICK_COUNT は SPSC モノトニックカウンタ・LAST_VSYNC_TICK はメイン専用・u64 周回実機到達不能） | atomic ordering 不変条件コメント | コメントのみ・コード挙動不変。アクセスマップ（生産者/消費者・スレッド帰属）を全 grep で裏取りし、Relaxed が健全な根拠を消費側（境界内）に明記。境界外の win_thread_mgr.rs は未変更。 |

差分（`git diff --numstat` 実測）: **2 ファイル変更、+20 / −2 行（すべてコメント）**。`world/mod.rs` **+16/−1**・`world/schedule_labels.rs` **+4/−1**。`git diff` の `+`/`-` 行は全てコメント行で**実行コード行（`frame_count.0 += 1`・2 つの `load`・struct/関数定義）は不変**を raw diff で確認。`common/`（tree_iter.rs/tree_system.rs）・`app.rs`・`world/vsync.rs` は **±0**（未編集）。

## 追加/除外したテスト

- **追加なし（0件）**。本境界のテスト可能ロジックは W7b-T1（18件）＋ W7b-T2（21件）＝39件で既に網羅済み。V 観点で新たに浮上した境界面は (a) FrameCount u32 増分（W7b-T2 の `try_tick_world_increments_frame_count_each_call` で低カウント域を特性化済み・u32::MAX 境界は 828日で単体非現実的）、(b) vsync atomic 比較（`pub(crate)` プロセスグローバル atomic ゆえ統合テストから操作不能・並列で非決定＝テスト不能、W7b-T2 所見3）、(c) tree_system マルチスレッド fan-out（512+ ノードのみ到達＝決定的結果を変えず単体非現実的、W7b-T1 所見1）のいずれかで、**新規の決定的特性化テストを追加できる余地がなく**、追加すれば churn または挙動影響フックを要するため見送った（W6b-V が新規整数境界 2 件のみ追加した先例と整合 — 本境界の整数境界は既テスト or プロセスグローバル非テスト）。
- **除外なし（0件）**。死テスト・到達不能テストは検出されず。

## proposals.md へ回した候補（P66）

- **P66**: FrameCount(u32) の tick 加算オーバーフロー堅牢化（debug panic 化の回避）。kind: 挙動変更を伴う脆弱性対策。`frame_count.0 += 1` は ~60Hz で 828日連続稼働時に 2^32 到達 → debug panic（理論的 DoS）/release wrap。消費はログ `frame=` ＋ surface.rs:44 の `requested_frame` 変化ノンス格納（本番読み取りゼロ・Changed 検出専用）のみで、ラップ MAX→0 でも「前回値と異なる」性質を保ち Changed 検出が継続するため正当性不変。`wrapping_add`（debug でもラップ統一）/`saturating_add` 化は **debug の panic 挙動を変える**（R5.1）ため本ループでは実装せず、現行挙動を W7b-T2 既存テスト＋本セルのハザードコメントで固定。将来 FrameCount をフレーム識別子として厳密比較/数値添字に使う設計が入る場合の u64 型拡張も suggestion に併記。

既知 proposals の再発見（重複記録なし・参照に留めた）:
- スケジュール順序・atomic ordering の**変更を要する対策は検出せず**（現行の登録順==実行順、UISetup-only SingleThreaded、Relaxed ordering はいずれも健全）ゆえ proposals 新規採番は P66 の1件のみ。
- W7b-T2 申し送りの `has_systems` ガード（`try_tick_world` 戻り値契約の生きたガード・デッドコードでない）は本 V 観点では脆弱性所見なし（戻り値セマンティクスは健全）。除去は挙動変更ゆえ S 観点 W7b-S が既に「無変更」と判定済み（P 採番不要）。

## verification (S2)

- BEFORE: 親検証済みベースライン（W7b-S 直後 = **1667 passed / 0 failed**・クリーンワークツリー、HEAD=`055f482`）を信頼し全量は省略（design フェーズ0 + 親指示「BEFORE S2 は省略可」）。開始時に境界ファイルの未コミット差分ゼロ（`git status --porcelain` で境界パス空）を確認。
- AFTER（必須・全量実施）:
  - `cargo build --workspace` → **成功**（exit 0、wintf/areka 再コンパイル、4.58s）。コメント追加のみで公開シンボル/型シグネチャ不変。
  - `cargo test --workspace` → **1667 passed / 0 failed / 32 ignored**（全 `test result:` 行を awk 合算: passed 合計 1667・failed 合計 0・ignored 32、`test result: FAILED`/`error[`/`panicked` 行ゼロ）。ベースライン 1667 から **±0**（コメントのみ＝テスト増減なし。挙動非破壊を実証）。
  - git diff（境界内のみ、`--numstat` 実測）: `world/mod.rs` **+16/−1**（FrameCount ハザードコメント 7行・atomic ordering コメント 9行で旧 1行コメント置換）・`world/schedule_labels.rs` **+4/−1**（順序コメント是正、旧 1行を 4行へ）。**実行コード行の変更ゼロ**（raw diff で確認、`frame_count.0 += 1`・2 load・全 struct/関数定義は不変）。`common/`・`app.rs`・`vsync.rs` は ±0。
- 全テスト 1667 件がベースラインと完全一致で合格。コメント追加はリリース/デバッグ挙動を一切変えず、S2 全量（1667=1667）で実証。

## clippy（S3・記録のみ・非ブロッカー）

- 境界（common/＋world/＋app.rs）を参照する clippy 警告（`cargo clippy -p wintf --lib`）:
  - **BEFORE/AFTER とも 3 件**: `tree_system.rs:17/55/89`（`type_complexity` ×3、bevy 上流ミラーのジェネリック伝播シグネチャ。W7b-S が churn 回避＋領域全体での type_complexity 受容規約で意図的に見送り記録済み）。
  - **本セル編集ファイル（`world/mod.rs`・`world/schedule_labels.rs`）を参照する診断はゼロ**（ファイル名 grep で該当なしを確認）。本セルの編集（コメント3箇所）は**新規 clippy 警告/error を一切導入していない**。
  - wintf lib 全体の総警告（warning/error 行）数は **156**（W7b-S の AFTER 182 から他セル/他モジュールの変動を含むが、本セル境界由来の増減はゼロ）。S3 規定によりブロッカーとせず記録に留める（簡素化は S 観点の担当）。

## RED フェーズ代替の検証

- テスト追加ゼロのため RED は **N/A**（非挙動変更の点検タスク）。適用した3対策はすべてコメントのみ（コード不変）で、回帰検知器は W7b-T1/T2 の既存39件。S2 全量がベースライン 1667 と完全一致（±0）であることが、コメント追加がリリース/デバッグ挙動を変えない証拠。各コメントの事実主張（登録順==実行順・atomic アクセスマップ・FrameCount 消費形態（ログ `frame=` ＋ surface.rs:44 の変化ノンス格納・`requested_frame` 本番読み取りゼロ））はすべて実コードの grep + 精読で裏取り済み（推測ゼロ）。

## flaky

- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue`（W7b 境界外 `tests/ecs`）: `cargo test --workspace` の全量実行で failed 合計ゼロ（当該バイナリ含め全 `test result` 行が passed のみ、隔離再実行不要）。本セルは変更がコメントのみで当該テストと無関係。

## 自己レビュー

- 実装は本物（モック/スタブ/プレースホルダ追加なし）。本セルの変更はコメント3箇所のみで、新たな unsafe・スタブ・TODO/FIXME・ロジック変更を導入していない。
- 点検は境界内 6 ファイル（+ in-source tests）を grep＋精読で網羅。フレーム時間境界（FrameCount u32 増分・measure_and_log_framerate 除算・hwnd as isize）・スケジュール順序（登録順==実行順・UISetup SingleThreaded・依存宣言）・ディスプレイ境界（display フラグ純 bool・ウィンドウカウント）・panic 経路（本番 unwrap/expect/添字ゼロ・tree_system の健全 unwrap/assert）・atomic ordering（VSYNC_TICK_COUNT/LAST_VSYNC_TICK の Relaxed 妥当性）をすべて判定。
- **本番挙動主張の裏取り（過去 REJECTED 回避）**: (1) スケジュール登録順 == 実行順 = 双方の grep 抽出で 13 ラベル同一順序を確認、(2) atomic アクセスマップ（VSYNC_TICK_COUNT は SPSC・LAST_VSYNC_TICK はメイン専用）= ワークスペース全 grep で生産者/消費者/スレッド帰属を確認、(3) FrameCount.0 消費形態 = `git grep -n "frame_count.0"` の全列挙で (a) ログ `frame=` フィールド（window_system/window_pos_systems/visual_manager/graphics/systems/{init,render}）と (b) 非ログの変化ノンス格納（surface.rs:44 の `dirty.requested_frame = frame_count.0 as u64`）の2形態のみであり算術比較・添字に未使用であること、`git grep -n "requested_frame"` で当該フィールドの本番読み取りがゼロ（テストのみ）で Changed 検出ノンス専用であることを各消費システム精読で確認、(4) tree_system unwrap の「キュー内全エンティティは子持ち」不変条件 = outbox push の filter_map 条件（children.map 経由のみ）を精読で確認。数字（1667→1667・git diff +20/−2・実行コード変更ゼロ）はすべて実測（cargo test / git diff raw）で裏取り、推測なし。
- 件数整合: S2 1667 = 1667（±0、コメントのみ）、git diff 2 ファイル +20/−2（全コメント行）、新規 `#[test]` 0、proposals +1（P66）。すべて相互一致。
- 境界遵守: 変更は `world/mod.rs`・`world/schedule_labels.rs`（W7b 境界内）＋ `proposals.md`（提案台帳）＋本断片のみ。tasks.md 未更新・コミット未作成・`win_thread_mgr.rs`/`window_proc/`/`layout/`/`window/`/`graphics/`（裏取りの読み取りのみ）/他領域/`vendors/`/機能spec文書への変更なし。
- 結論: 本境界は脆弱性耐性が高く、warranted な挙動非破壊対策は (1) stale スケジュール順序コメントの是正、(2) FrameCount 整数境界ハザードコメント、(3) atomic ordering 妥当性コメントの **3 件のコメントのみ**に限られた。本番経路に到達可能な panic・現実的な数値境界破綻・スケジュール順序不整合・atomic ordering 欠陥・ディスプレイ境界不整合は不在（panic は Option/Result で全域グレースフル処理、FrameCount 消費はログ `frame=` ＋ surface.rs:44 の変化ノンス格納のみで算術比較・添字に未使用かつラップ MAX→0 でも Changed 検出が継続するためラップ無害、登録順==実行順、Relaxed は SPSC/メイン専用で健全、display フラグは単一スレッド逐次）。挙動変更を要する 1 件（FrameCount u32 オーバーフロー堅牢化）は R2.4/R5.2 に従い P66 へ記録し、その他は現状安全と判定して churn を回避した。
