# W6b-S: wintf ドラッグ × シンプル化（デッドストア除去・clippy 簡素化）

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点
- セルID: W6b-S（領域 W6b「wintf ドラッグ」 × 観点 S「シンプル化」）
- 性質: **非挙動変更タスク**（リファクタリング／簡素化。R5.1）。直前の W6b-T（特性化テスト50件）が回帰検知器。簡素化後に S2 全量が緑であることが挙動非破壊の証拠。Feature Flag Protocol 不要。
- requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3（デッドストア除去として 2.9, 2.10）
- design: S 観点手順（CellExecutor「S」規則：S6 基準で簡素化／テスト保護外 unsafe/COM/GUI は構造的整理に限定 R5.5／非推奨コードは利用ゼロ実証で削除 R2.9・できなければ提案 R2.10）、S6 karpathy（churn 回避・自明な重複除去）、領域定義「W6b: wintf ドラッグ / `crates/wintf/src/ecs/drag/`」、セル断片様式、提案記録様式
- 参考: `report/cells/W6b-T.md`（テスト済み範囲・P61 根拠）、`report/cells/W6a-S.md`（実証付き削除の先例）、`report/proposals.md`（P61）
- 境界: `crates/wintf/src/ecs/drag/`。簡素化に伴うテスト追従として境界内 `crates/wintf/tests/drag/dispatch_test.rs` を含む。**境界外ファイルの変更なし**（ecs/mod.rs 等への波及なし＝prev_frame_pos はフィールドであり型 re-export に影響しないことを確認）。

## 調査範囲
W6b-T が作成したモジュール×テスト対応表（mod/state/accumulator/context/dispatch/capture_guard/systems）を基に、状態遷移（state.rs 状態機械）・キャプチャガード（capture_guard.rs）・accumulator・context・dispatch の S6 簡素化候補を検証した。最有力候補は P61（W6b-T 申し送り）の `DraggingState.prev_frame_pos` デッドストア除去。あわせて drag モジュール配下の clippy 簡素化系 lint を棚卸しし、明確に挙動非破壊なもののみ適用した。

## prev_frame_pos デッドストア除去の再実証（必須記録）

### grep 再実証（ワークスペース全域）
- `prev_frame_pos`（全 `*.rs`）の**本番（非テスト）出現**:
  - 定義: `mod.rs:78`（`pub prev_frame_pos: PhysicalPoint`）。
  - 書き込み1: `dispatch.rs:160`（Started 時 `prev_frame_pos: start_pos` の構造体リテラル初期化）。
  - 書き込み2: `dispatch.rs:387`（DragEvent 発火時 `dragging_state.prev_frame_pos = flush_result.current_position`）。
  - 行コメント: `dispatch.rs:382`「DraggingState.prev_frame_posを更新」（コードではない）。
  - **本番読み取り: ゼロ**。`.prev_frame_pos`（フィールドアクセス）の grep でも、本番は `dispatch.rs:387`（代入 `=` の左辺＝書き込み）のみで、右辺・条件・引数として読む箇所は皆無。
- **「フレーム間デルタが次フレームに読まれていないか」の慎重確認**: `dispatch_drag_events` のデルタは `flush_result.delta`（accumulator が wndproc 側で `accumulate_delta` 累積したもの）と `flush_result.current_position - start_pos`（dispatch.rs:364-365 のログ）で算出されており、**`prev_frame_pos` を経由しない**。デルタの真のソースは `DragAccumulator.accumulated_delta`（accumulator.rs:54-57）と `current_position`／`drag_start_pos` であることをコードで確認。`prev_frame_pos` はどのデルタ計算にも参加していない。
- **DraggingState 全アクセスの棚卸し**（`DraggingState` grep）: 本番の読み取りアクセスは `dispatch.rs:345`（`.map(|ds| ds.drag_start_pos)` ＝ `drag_start_pos` のみ）だけ。`initial_inset` は本番では書き込みのみ（dispatch.rs:159）でテストが読む。`prev_frame_pos` を読む本番経路は存在しない。examples（`taffy_flex_demo/drag.rs:116`）は**コメントのみ**で `DraggingState` を構築・参照しない（フィールド除去でビルド破壊なし）。
- テスト参照: `tests/drag/dispatch_test.rs` の構造体リテラル5件（W6b-T が追加）と `dispatch_emits_drag_event_when_delta_nonzero` の更新アサート1件のみ。

### 判定
**本番読み取りゼロを再実証**（コメント「現在は未使用」と一致）。P61 は誤認ではなく正しい（W6a-T のような本番事実誤認なし）。`prev_frame_pos` は「毎ドラッグフレームに書き込まれるが決して読まれないデッドストア」であり、**読まれないフィールドの除去は挙動非破壊**。R2.9/R2.10 適用域のデッドコード除去として本セルで実施。

### 適用した除去
1. `mod.rs:78` フィールド定義削除（+ コメント「前回ECSフレームの位置（デルタ計算用、現在は未使用）」も同時削除）。
2. `dispatch.rs:160` Started 時の初期化 `prev_frame_pos: start_pos` 削除。
3. `dispatch.rs:382-389` の DragEvent 経路更新ブロック（`get_mut::<DraggingState>` → `dragging_state.prev_frame_pos = ...`）を**丸ごと削除**。当該 `get_mut` ブロックは prev_frame_pos 更新の**唯一の目的**で存在し（ブロック内の文は当該代入1つだけ）、フィールド除去で完全にデッド化するため。DragEvent のメッセージ書き込み・ハンドラ配信（dispatch.rs:350-379）は無傷で残存。
4. テスト追従（`tests/drag/dispatch_test.rs`）:
   - `prev_frame_pos` を含む構造体リテラル5件（Ended/no-delta/cleanup×2/delta-nonzero）から当該フィールド行を除去（フィールド消失への機械的追随）。
   - `dispatch_emits_drag_event_when_delta_nonzero`: prev_frame_pos 更新を検証していたアサート（`ds.prev_frame_pos == (140,160)`）と直前の `let ds = world.get::<DraggingState>(target)` 読み取りを除去。**DragEvent 本体の検証（件数1・target・position(140,160)・start_position=drag_start_pos(100,100)）は全て残存**（テスト関数は削除せず、prev_frame_pos 検証部分のみ除去）。doc コメントを「DragEvent が書き込まれ、start_position が DraggingState.drag_start_pos から取得される」へ是正。
   - テスト関数の**追加・削除はなし**（アサート/リテラルの調整のみ）。よって drag テスト件数は不変（in-source 44・統合 19）。

## 適用した簡素化の一覧（各々の挙動非破壊根拠）

| # | 適用箇所 | 内容 | 挙動非破壊根拠 |
|---|----------|------|----------------|
| 1 | `mod.rs`/`dispatch.rs` | `DraggingState.prev_frame_pos` デッドストア除去（上記） | 本番読み取りゼロを grep 再実証。デルタは別ソース（accumulated_delta / current_position − drag_start_pos）で算出。読まれないフィールド・その2書込・専用 get_mut ブロックの除去は観測挙動に影響しない |
| 2 | `state.rs:185` | `RefCell::new(DragState::Idle)` を `const { RefCell::new(DragState::Idle) }` へ（clippy `missing_const_for_thread_local`） | `DragState::Idle` は unit variant・`RefCell::new` は const-stable。thread_local の初期値・型・観測挙動は同一。const 初期化はコンパイラ最適化のみ |
| 3 | `state.rs:195` | `f(&mut *state)` → `f(&mut state)`（clippy `explicit_auto_deref`） | `state` は `RefMut<DragState>`。`&mut state` は自動 deref で `&mut DragState` になり、明示 deref+再参照と同一の参照を渡す。恒等変換 |
| 4 | `state.rs:206` | `f(&*state)` → `f(&state)`（clippy `explicit_auto_deref`） | 同上（`Ref<DragState>` の自動 deref）。恒等変換 |
| 5 | `context.rs:32` | 手書き `impl Default for WindowDragContext` を削除し struct へ `#[derive(Default)]` 付与（clippy `derivable_impls`） | 手書き Default は `hwnd: None, initial_window_pos: None, move_window: false, constraint: None`＝各フィールド型（`Option<_>`／`bool`）の Default と完全一致。derive 生成物と同値。`clear()` メソッドは別途残存（Default ではないため未変更） |
| 6 | `dispatch.rs:111` | `wh.client_to_window_coords(pos.into(), size.into())` → `(pos, size)`（clippy `useless_conversion` ×2） | `WindowPos.position: Option<Point>`→`pos: Point`、`WindowPos.size: Option<SizeI>`→`size: SizeI`。`client_to_window_coords(position: Point, size: SizeI)`（window_handle.rs:137-140）。型一致＝`From<T> for T` の反射 blanket impl による恒等変換（`self` を返すのみ）。GDI パス上だが R5.5 が許す「自明な重複除去」（ロジック変更なし） |
| 7 | `dispatch.rs:121` | `initial_window_pos = pos.into()` → `pos`（clippy `useless_conversion`） | `initial_window_pos: crate::ecs::Point`・`pos: Point`（同型）。恒等変換。WindowHandle 無しフォールバック経路 |

差分: 5 ファイル変更、**+7 / −43 行（net −36）**。
- `crates/wintf/src/ecs/drag/mod.rs`: 0 / −2
- `crates/wintf/src/ecs/drag/dispatch.rs`: +2 / −12
- `crates/wintf/src/ecs/drag/state.rs`: +3 / −3
- `crates/wintf/src/ecs/drag/context.rs`: 0 / −13
- `crates/wintf/tests/drag/dispatch_test.rs`: +2 / −13

## 適用しなかった候補と理由（churn 回避）

1. **`collapsible_if`（dispatch.rs:107・119・172・340、計4件）の let-chain 化**: いずれも `if let Some(x) { if let Some(y)/cond {...} }` を `if let Some(x) && let Some(y) = ... {...}` へ畳む clippy 提案。挙動非破壊だが、(a) 107/119 は実 HWND + `client_to_window_coords`（GDI）のデバイス依存ブロック（W6b-T 所見1・R5.5 の構造的整理限定域）で、ネスト構造の改変は churn かつ「壊れていないものをいじらない」（karpathy）に反する、(b) 172/340 はクリティカルなドラッグ入力ディスパッチ経路の制御フロー構造変更で、本セルの主目的（prev_frame_pos 除去）と直交、(c) 私が導入した lint ではない既存指摘。**先例**: W6a-S は `nchittest_cache.rs:60` の collapsible_if を scope+churn で見送り、W6b-T もプロダクションの collapsible_if を未変更で記録に留めた。同方針で本セルも**見送り**（ロジック変更不要＝proposals 化はせず、本断片に churn 回避の記録に留める）。
2. **`WindowDragContext::clear()` を `*self = Self::default()` へ**: derive Default 化後は技術的に可能だが、clippy 指摘外であり手書き clear は既存パターン。surgical 原則（lint が指摘した箇所のみ触る）で未変更。
3. **`DragState` 状態機械（state.rs）の遷移ロジック統合**: `end_dragging`/`cancel_dragging` の CaptureGuard 抽出 match（state.rs:405-410, 450-455）と `force_idle()`（テスト）が同形だが、(a) 本番2関数の統合はヘルパ抽出＝抽象の純増（2箇所のための抽象、S6 不採用基準）、(b) CaptureGuard の所有権移動・borrow 解放タイミング（RefCell already borrowed パニック回避）の厳密保持が要点で naive な共通化は退行リスク。挙動非破壊の明確な可読性向上が乏しく churn が勝るため**見送り**（ロジック変更を要さないため proposals 化もせず）。
4. **`CaptureGuard::is_released`（capture_guard.rs:47-50）のテスト専用アクセサ整理（P61 併記候補）**: `#[allow(dead_code)]` 付き・本番読み取りゼロだが、in-source テスト2件（`capture_guard_with_null_hwnd`・`mark_released_sets_flag`）が `released` フラグの検証手段として参照する。除去するとテストの検証手段（`mark_released` 後にフラグが立つことの観測）が失われ、代替アクセサ／検証方法の新設が必要になる。これは「テスト保護の弱体化」につながり、prev_frame_pos（本番デッドストア・テストは更新挙動を assert するだけ）とは性質が異なる（is_released はテストの**観測 API** として生きている）。本セルでは**見送り**、P61 の併記候補のまま維持（優先度低）。断片に理由を明記。

## proposals へ回した候補
- **新規 P 採番なし**（P62 以降への追加なし）。本セルで検証した簡素化候補は (a) 挙動非破壊として適用済み（prev_frame_pos・clippy 7件）、または (b) ロジック変更を要さず churn 回避で見送り（collapsible_if・状態機械統合・clear・is_released）であり、いずれも「ロジック変更を要する簡素化」に該当しないため proposals 追記は不要と判断（churn 抑制）。
- **P61（既存・本セルで解消）**: `DraggingState.prev_frame_pos` のデッドストア除去を本セルで実施・解消。`CaptureGuard::is_released` の併記候補は上記「適用しなかった候補4」の理由で本セルでは見送り、P61 の低優先候補として維持。proposals.md の P61 に「W6b-S で prev_frame_pos 除去を実施・解消済み（is_released はテスト観測 API のため見送り）」の解決状況を追記予定（親または集約フェーズで反映）。

## clippy（S3・記録のみ・非ブロッカー）
- drag モジュール配下（`ecs/drag/` の7ソースファイル）を参照する clippy 警告:
  - **BEFORE: 12 箇所** — `context.rs:32`（derivable_impls）、`dispatch.rs:107/119/173/341`（collapsible_if ×4）、`dispatch.rs:111`（useless_conversion ×2）、`dispatch.rs:121`（useless_conversion）、`state.rs:185`（missing_const_for_thread_local）、`state.rs:195/206`（explicit_auto_deref ×2）。
  - **AFTER: 4 箇所** — `dispatch.rs:107/119/172/340`（collapsible_if ×4、意図的に見送り。行番号は prev_frame_pos ブロック除去で 173→172・341→340 へシフト）。
  - **解消した lint: 8件**（derivable_impls 1・useless_conversion 3・missing_const_for_thread_local 1・explicit_auto_deref 2・collapsible_if 1〔dispatch.rs:383、prev_frame_pos ブロック除去に伴い消滅〕）。
  - **新規導入 lint: ゼロ**。`mod.rs`/`state.rs`/`context.rs`/`accumulator.rs`/`systems.rs`/`capture_guard.rs` を参照する警告は AFTER で皆無（残存は dispatch.rs の4件のみ）。
- wintf lib 全体の `cargo clippy -p wintf --lib` 総警告数は AFTER 145（他セル/他モジュールの既存警告を含む全体値であり本セルのスコープ外。本セルの寄与は drag モジュールの 12→4＝**純減8・新規0**）。S3 規定によりブロッカーとせず記録に留める。

## verification (S2)
- BEFORE: 親検証済みベースライン（1566 passed / 0 failed・クリーンワークツリー）を信頼し全量は省略（design フェーズ0 + 親指示「BEFORE S2 は省略可」）。反復検証ベースラインとして `cargo test -p wintf --lib drag::` で **44 passed / 0 failed** を開始時に確認。
- AFTER:
  - `cargo build --workspace` **成功**（`areka` 本体・`wintf` 含む全クレート。prev_frame_pos フィールド除去・derive Default 化・useless_conversion 除去がビルドを壊さないことの確認）。
  - `cargo test --workspace` **1566 passed / 0 failed / 32 ignored**（全 `test result` 行を awk 合算: passed=1566 / failed=0 / ignored=32。`test result: FAILED`/`error[`/`panicked` 行ゼロ）。
  - **増減内訳**: 1566 → 1566（**±0**）。prev_frame_pos 除去はテスト関数を削除せずアサート/リテラルを in-place 調整したため件数不変。clippy 簡素化6種も挙動・件数不変。
  - 反復検証: `cargo test -p wintf --lib drag::` で **44 passed / 0 failed**（既存 capture_guard 3 + W6b-T 追加 41、不変）。`cargo test -p wintf --test drag` で **19 passed / 0 failed**（capture_guard_panic_safety 3 + window_dragging_filter 7 + dispatch_test 9、不変）。
  - **回帰検知器の残存確認**: W6b-T 追加の50件（特に `dispatch_emits_drag_event_when_delta_nonzero` の DragEvent 検証、状態機械21件、accumulator 8件）が全て GREEN。prev_frame_pos 更新挙動を assert していた箇所のみ除去し、DragEvent 配信・状態遷移の検証は無傷。

## flaky
- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue`（W6b 境界外 `tests/ecs`）: `cargo test --workspace` の全量実行で `tests/ecs` バイナリは **155 passed / 0 failed** と合格（隔離再実行不要）。本セルの変更とは無関係。

## 自己レビュー結論
- 除去した `prev_frame_pos` は本番読み取りゼロを grep で再実証したデッドストアに限定（定義1・書込2・読取0、デルタは別ソース）。観測可能な挙動変更なし（R5.1/R5.3）。
- 適用した clippy 簡素化6種（const thread_local・auto-deref ×2・derive Default・useless_conversion ×3）はいずれも恒等変換／同値生成で挙動非破壊。各々の根拠を型・シグネチャで裏取り済み。
- 本番ドラッグ経路（状態機械・accumulator・dispatch の DragStart/Drag/DragEnd 配信）は無傷で残存し、W6b-T の50件が回帰検知器として機能。
- S2 全量 AFTER = 1566 passed / 0 failed（ベースラインと完全一致＝±0、数字は実測）。
- 境界外（ecs/mod.rs 等）への波及なし（prev_frame_pos はフィールドであり型 re-export に無影響）。
- churn を要する候補（collapsible_if 4件・状態機械統合・clear・is_released）は karpathy「壊れていないものをいじらない」と R5.5・先例（W6a-S/W6b-T）に従い見送り。ロジック変更を要さないため proposals 新規採番もなし（P61 を解消、is_released は低優先で維持）。
