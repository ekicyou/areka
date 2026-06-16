# W5a-T: wintf テキスト描画 × テスト網羅性

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点
- セルID: W5a-T（領域 W5a「wintf テキスト描画」 × 観点 T「テスト網羅性」）
- 性質: 既存挙動の特性化テスト追加（挙動変更なし。R2.1, R5.1）。W5a 領域の最初のセル（先行 W5a 断片なし）。
- requirements: 2.1, 2.5, 2.7, 2.8, 4.1, 5.1
- design: プロジェクト・プロファイル S2/S9、レビュー観点列 T、CellExecutor 観点別規則（T）、セル断片様式、提案記録様式

## 対象ファイル一覧（W5a = `crates/wintf/src/ecs/widget/text/`）

- `mod.rs`（re-export のみ、18 LOC）
- `label.rs`（Label / TextDirection / TextLayoutResource、119 LOC）
- `draw_labels.rs`（draw_labels システム、233 LOC）
- `typewriter.rs`（Typewriter / TypewriterTalk / TypewriterLayoutCache、352 LOC）
- `typewriter_ir.rs`（Stage1/Stage2 IR 型、210 LOC）
- `typewriter_layout.rs`（init/invalidate システム + convert_to_timeline、242 LOC）
- `typewriter_draw.rs`（update/draw/draw_backgrounds システム、378 LOC）

合計 約1,550 LOC（design.md W5a 概算 1,370 と整合。境界 = widget/text/ のみ。他 widget サブモジュールには一切触れていない）。

## モジュール×テスト対応表（改善前 → 改善後）

| モジュール | 対象（主要 関数/型） | デバイス依存 | 既存テスト所在・件数 | 追加 | 所見 |
|------------|------|------|-----------|------|------|
| `mod.rs` | re-export のみ | なし | — | 0件 | テスト対象なし |
| `label.rs` | `TextDirection`（enum/Default）、`Label::default`、`Label.direction`、`TextLayoutResource::{empty,get}`、フック `on_label_add`（Visual 自動挿入）/`on_label_remove`/`on_text_layout_remove` | 一部（`TextLayoutResource::new` は実 IDWriteTextLayout 必須） | `tests/widget/vertical_text_layout_test.rs` 2件（TextDirection enum/Label.direction）+ `tests/visual/widget_visual_auto_insert_test.rs` 4件（on_label_add の Visual 自動挿入・既存 Visual 非上書き・複数ウィジェット） | **2件** | 空白: `Label::default()` のフィールド既定値（メイリオ/16pt/空文字/横書きLTR）と `TextLayoutResource::empty()→get()=None` が未固定だった。`on_text_layout_remove`（trace ログのみ）は観測価値なし |
| `draw_labels.rs` | `draw_labels` システム（TextFormat/TextLayout 生成・方向設定・CommandList 記録・metrics 補正） | **全面 DirectWrite/D2D** | なし（実デバイス統合経路のみ） | 0件 | 純粋ロジックの抽出可能箇所なし。所見1 |
| `typewriter.rs` | `Typewriter::default`、`TypewriterState`（enum/Default）、**`TypewriterTalk`（new/pause/resume/skip/update/各 getter）**、`TypewriterLayoutCache`（COM 保持）、各フック | `TypewriterTalk` は**全面デバイス非依存**（純粋状態マシン）。`TypewriterLayoutCache` は実 IDWriteTextLayout 必須 | in-source 3件（Typewriter::default・TypewriterState default/transitions のみ） | **12件** | **最大の空白**: `TypewriterTalk` の pause/resume の時刻計算・skip・`update` のタイムライン消費（Glyph 表示・Wait ゲート・FireEvent 発火・progress・Completed 遷移・非Playing no-op・zero-cluster 即完了・再発火防止）が**完全に未テストだった** |
| `typewriter_ir.rs` | `TypewriterToken`（Text/Wait/FireEvent）、`TypewriterEvent`（Default/From）、`TypewriterEventKind`、`TimelineItem`（Glyph/Wait/FireEvent）、`TypewriterTimeline::empty` | なし（plain data） | in-source 7件（Token Text/Wait・Event default・EventKind 3種変換・Timeline empty・TimelineItem Glyph/Wait） | **2件** | 空白: `TypewriterToken::FireEvent` と `TimelineItem::FireEvent`（いずれも Entity + イベント種別を保持するバリアント）の構築・分解が未固定だった |
| `typewriter_layout.rs` | `convert_to_timeline`（Stage1→Stage2 変換）、`init_typewriter_layout`/`invalidate_typewriter_layout_on_arrangement_change` システム | **DirectWrite**（`get_cluster_metrics`・TextFormat/TextLayout 生成） | なし | 0件 | `convert_to_timeline` の本体ロジックは純粋だがシグネチャが `&IDWriteTextLayout` を要求し COM 呼び出しに密結合 → 単体不能。所見2 → **P54** |
| `typewriter_draw.rs` | `update_typewriters`（`talk.update` 委譲）、`draw_typewriters`/`draw_typewriter_backgrounds` システム | `draw_*` は**全面 DirectWrite/D2D**。`update_typewriters` のコアは `TypewriterTalk::update`（本セルで直接特性化済み） | なし | 0件 | `update_typewriters` の状態遷移ロジックは `TypewriterTalk::update`（12件で固定）と等価。描画 2 システムは純粋ロジックなし。所見3 |

追加テスト合計 **16件**（label 2・typewriter 12・typewriter_ir 2）。**プロダクションコードの変更なし**（R5.1 充足）。新規テストファイルなし（既存 in-source `mod tests` 2ファイル + 既存統合テスト1ファイルへの追記）。

## 追加したテスト一覧（ファイル・テスト名・狙い）

**`crates/wintf/src/ecs/widget/text/typewriter.rs`（in-source `mod tests`, 12件）**
ヘルパ `make_glyph_timeline(glyph_count, step)` を追加（`convert_to_timeline` を経由せず `TypewriterTimeline` を直接手組み＝DirectWrite 非依存）。
- `test_typewriter_talk_new_initial_state` — `new()` の初期状態（Playing/start_time/visible=0/progress=0/tokens）
- `test_typewriter_talk_pause_records_elapsed_and_changes_state` — pause が paused_elapsed を記録し、resume が `start_time = now - paused_elapsed` で復元
- `test_typewriter_talk_pause_is_noop_when_not_playing` — 既に Paused のとき pause 再呼び出しは状態・start_time 不変
- `test_typewriter_talk_resume_is_noop_when_not_paused` — Playing 状態での resume は no-op
- `test_typewriter_talk_skip_forces_complete` — skip(N) で visible=N/progress=1.0/Completed
- `test_typewriter_talk_update_returns_empty_when_not_playing` — 非 Playing 中の update は早期 return（空・無進行）
- `test_typewriter_talk_update_reveals_glyphs_up_to_elapsed` — elapsed に応じた Glyph 段階表示と progress 計算
- `test_typewriter_talk_update_completes_when_all_glyphs_visible` — 全 Glyph 表示で Completed 遷移
- `test_typewriter_talk_update_zero_clusters_completes_immediately` — total_cluster_count=0 の退化ケース（progress=1.0・即 Completed）
- `test_typewriter_talk_update_wait_gates_following_glyph` — Wait（start_at+duration）通過まで後続 Glyph をブロック
- `test_typewriter_talk_update_fires_event_at_threshold` — FireEvent が fire_at 到達で発火、未達 Glyph で break
- `test_typewriter_talk_update_does_not_refire_event_on_second_call` — next_item_index 進行 + Completed により再発火しない

**`crates/wintf/src/ecs/widget/text/typewriter_ir.rs`（in-source `mod tests`, 2件）**
- `test_typewriter_token_fire_event` — `TypewriterToken::FireEvent { target, event }` の構築・分解
- `test_timeline_item_fire_event` — `TimelineItem::FireEvent { target, event, fire_at }` の構築・分解

**`crates/wintf/tests/widget/vertical_text_layout_test.rs`（統合, 2件）**
- `test_label_default_values` — `Label::default()` の既定値（text=""/font=メイリオ/16.0/HorizontalLeftToRight）
- `test_text_layout_resource_empty_returns_none` — `TextLayoutResource::empty().get()` が None（COM 不要経路）

## 除外したテスト

なし。widget/text/ 配下の既存 in-source テスト（typewriter.rs 3件・typewriter_ir.rs 7件）および統合テスト（vertical_text_layout 2件・widget_visual_auto_insert 4件）には重複・死テスト（到達不能・常に真・対象消失）は検出されなかった。既存テストはいずれも異なるバリアント/既定値を固定しており冗長ではない。過不足整理の結論: **不足のみ存在（16件で充足）、過剰なし**。

## テスト不能箇所・深掘り所見（R2.8）

1. **`draw_labels` / `draw_typewriters` / `draw_typewriter_backgrounds` は全面 DirectWrite/D2D 依存** — これら描画システムは (a) `IDWriteFactory::CreateTextFormat`/`CreateTextLayout`、(b) `ID2D1DeviceContext::CreateCommandList`/`BeginDraw`/`EndDraw`/`DrawTextLayout`/`FillRectangle`、(c) `IDWriteTextLayout::GetMetrics`/`GetClusterMetrics`/`SetDrawingEffect` を直接呼ぶ。これらは実 GraphicsCore（D3D11+D2D+DWrite デバイス、`graphics_core.device_context()`/`dwrite_factory()`）を要し、各システムは「リソース取得→描画記録→CommandList 挿入」の手続きで、抽出可能な純粋計算ブロックを持たない（origin 補正 `-metrics.left`/`-metrics.top` 等の座標計算も DirectWrite が返す metrics に依存）。`draw_typewriters` の描画範囲計算（visible_count→visible_text_length の `cluster_metrics.iter().take(..).map(length).sum()`）も `get_cluster_metrics()` の戻り値が前提。**ユニット到達不能は環境制約**であり、コード側の改善余地ではないため提案化しない（既存の実デバイス統合テスト群＝tests/graphics の前例が最終的な回帰検知器）。

2. **`convert_to_timeline` の純粋ロジックが DirectWrite に密結合（→ P54）** — `typewriter_layout.rs::convert_to_timeline` は Stage1 IR→Stage2 IR 変換の**本体が純粋計算**（Text→Glyph 累積時刻生成・Wait の start_at/current_time 更新・FireEvent の fire_at 記録・`cluster_index < total_cluster_count` 打ち切り）だが、シグネチャが `text_layout: &IDWriteTextLayout` を受け取り冒頭 `get_cluster_metrics()?` で `total_cluster_count` を得るため関数全体が単体不能。`total_cluster_count` を引数化する純粋内側関数の抽出で単体到達可能になる（観測挙動非破壊の見込み）が、device 依存システムファイル内の構造変更であり「判断に迷う構造変更は proposals へ」（タスク指示）に従い本セルでは見送り、P54 に記録した。W5a-T では消費側（`TypewriterTalk::update` の timeline 走査）を12件で固定済みのため、P54 実施時は生成側と消費側が両端から保護される。

3. **`update_typewriters` の状態遷移ロジックは `TypewriterTalk::update` と等価** — `typewriter_draw.rs::update_typewriters` は FrameTime から現在時刻を取り `talk.update(current_time, layout_cache.timeline())` を呼び、返ったイベントを `commands.entity(target).insert(TypewriterEvent::from(kind))` するだけのシステム。中核の状態遷移・イベント発火判定は `TypewriterTalk::update`（本セルで12件特性化）と同一であり、システム層での再テストは (a) FrameTime リソース + (b) TypewriterLayoutCache（実 IDWriteTextLayout）を要し過剰。`TypewriterEvent::from` 変換は typewriter_ir.rs の既存3件で固定済み。よってシステム層の追加は不要と判断（重複回避）。

4. **`TypewriterTalk` の `update` における Wait/Glyph/FireEvent の混在順序依存** — `update` は `next_item_index` を単調前進させ、各フレームで「閾値未達の最初の項目」で break する。Wait が後続 Glyph をブロックする挙動（所見の `test_..._wait_gates_following_glyph`）と、FireEvent 発火後に未達 Glyph で停止する挙動（`test_..._fires_event_at_threshold`）を固定したことで、3 種 TimelineItem の境界・順序消費が特性化された。深掘りの結果、`update` のロジックにバグ・前提誤りは検出されなかった（全16件が初回実行で導出どおり一致）。

5. **`TypewriterLayoutCache` の `unsafe impl Send/Sync`** — typewriter.rs:291-292 で IDWriteTextLayout（!Send/!Sync）を保持する `TypewriterLayoutCache` に手動 Send/Sync を付与している。これは V 観点（unsafe 境界の妥当性: TextLayout が単一スレッドからのみアクセスされる不変条件の確認）の検討対象。T セルでは構築・getter を直接テストするには実 IDWriteTextLayout が必要で不能のため、Send/Sync 健全性の点検は W5a-V へ申し送る（本 T セルからの提案化はしない）。

## proposals へ回した候補

- **P54**: `convert_to_timeline` の純粋ロジック分離（DirectWrite 非依存なタイムライン構築の単体到達）

W5a-V への申し送り（所見5）: `TypewriterLayoutCache` の手動 `unsafe impl Send/Sync` の不変条件点検。

## verification (S2)

- BEFORE: 親のベースライン（1451 passed / 0 failed・クリーンワークツリー）を信頼して流用（design のフェーズ0 ベースライン規定 + 親指示「BEFORE S2 は省略可」に従う）。
- AFTER: `cargo build --workspace` 成功、`cargo test --workspace` **1467 passed / 0 failed**（全テストバイナリで failed=0）。
  - 触れたバイナリの内訳確認（追加テスト数と完全一致）:
    - `wintf` lib（in-source）: **248 → 262（+14 = typewriter 12 + typewriter_ir 2）**
    - `tests/widget`: 14 → **16**（+2 = label 2）
  - 全16件が初回実行で合格（特性化テスト = GREEN by construction。後述 RED 代替を参照）。
  - グローバル合計は 1451 → 1467（**+16**）。内訳: wintf lib +14（typewriter 12 + typewriter_ir 2）、tests/widget +2（label 2）。変更退避（git stash）で同一集計法によりベースライン 1451 を実測、既存テストの欠落なし（lib テスト名の差分は追加14件のみ・削除0件）、failed=0。触れたバイナリは追加数ぴったり増加。
  - 反復検証: `cargo test -p wintf --lib text::` で text モジュール 24 passed / 0 failed（既存10 + 追加14）、`cargo test -p wintf --test widget` で 16 passed / 0 failed。

## flaky

- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue`（W5a 境界外 `tests/ecs`）: `cargo test --workspace` の全量実行で `tests/ecs` は 79 passed / 0 failed と合格（隔離再実行不要）。本セルの追加テストとは無関係。

## clippy（S3・記録のみ・非ブロッカー）

- `cargo clippy -p wintf --tests` は既存警告約188件を出力。**いずれも本セルの追加テスト由来ではない**（`typewriter`/`text/`/`label`/`vertical_text_layout` を含む警告は0件＝追加コードによる新規警告の導入なし）。本セルはテスト追加のみでプロダクションコード未変更のため新規警告なし。S3 規定によりブロッカーとせず記録に留める（簡素化は S 観点の担当）。

## RED フェーズ代替の検証

追加16件はすべて既存挙動の characterization のため RED は N/A（GREEN by construction）。期待値は実装と独立に各ソース仕様から導出した: `TypewriterTalk` の pause/resume の `paused_elapsed` 経由 start_time 復元、skip の強制完了、`update` の `elapsed >= show_at`/`elapsed >= start_at+duration`/`elapsed >= fire_at` 各閾値判定と `next_item_index` 単調前進・`visible_cluster_count >= total_cluster_count` での Completed 遷移・zero-cluster 即完了・非Playing 早期 return、`Label::default`/`TextLayoutResource::empty` の既定値、FireEvent バリアントのフィールド保持。初回実行で16件全件が導出どおり一致し、バグ・前提誤りは検出されなかった（深掘りを要する初回失敗なし）。
