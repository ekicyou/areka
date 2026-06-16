# コードベースレビュー・ループ 改善内容レポート（codebase-review-loop）

本レポートは `report/cells/` 配下の全セル断片（真実源）と `report/proposals.md`（P1〜P75）を集約した最終成果物である。再実行時は全置換される（断片が真実源）。要件 R6.1〜R6.4・design.md「ReportAggregator / レポート完全性」に対応する。

## 1. 概要

### 目的とスコープ

リポジトリ全域（`crates/` 配下の自作クレート areka / dola / wintf と横断的プロジェクト設定）を「レビュー領域 × レビュー観点」のマトリクスで系統的にレビューし、**外部観測可能な挙動を変えずに**テスト網羅性・コードのシンプルさ・脆弱性耐性を改善する完走保証付きループを実行した。

- **マトリクス規模**: 19 レビュー領域 × 3 レビュー観点（T=テスト網羅性 / S=シンプル化 / V=脆弱性）= **57 論理セル**。テスト空白の大領域 W7a / W7b / W8 の T セルは事前分割により各 2 サブセル化され、**60 断片ファイル**が 57 論理セルへ対応する。
- 前後をフェーズ0（環境準備・ベースライン確立）と最終フェーズ（S7 起動テスト・本レポート集約）で挟む。

### 累積メトリクス（断片から導出）

| 指標 | 値 | 根拠 |
|------|----|------|
| 実施論理セル数 | **57 / 57**（断片 60 / 60） | §2 マトリクス網羅性表 |
| テスト件数（ベースライン → 最終） | **963 → 1713 passed / 0 failed**（`cargo test --workspace`） | A1-T BEFORE（963）→ X1-V / final-launch AFTER（1713） |
| 最終（`--all-features`） | **1716 passed / 0 failed**（既定比 +3 = dola toml/yaml 往復テスト） | X1-T / X1-V 実測 |
| 巻き戻し件数 | **0 件** | 全断片に rolled-back ステータスなし（§4） |
| レビュー REJECTED ラウンド | **3 回**（W5a-T / W6a-T / W7b-V、いずれも第2ラウンドで是正済み） | tasks.md Implementation Notes（§4 remediation 記録） |
| フレーキー判定 | `cue_performance_test` 系（負荷依存）→ W8-T1 で決定論化して解消、`tracker_timeout` 容疑は偽陽性確定（§5） | W8-T1・各 V/T 断片 flaky 節 |
| 敵対的レビューが捕捉した事実誤認 | **3 件**（是正済み） | tasks.md Implementation Notes（§4） |
| no-change セル | **3 件**（X1-T / X1-V / final-launch — いずれも点検結果が無変更で正当） | §2・§3 |
| 最終起動テスト（S7） | **PASS**（初期化完了ログ +0.208 秒で出現・stderr 空・panic/error/WARN ゼロ・正常終了） | final-launch.md（§6） |

**ベースライン件数に関する注記（断片 vs tasks.md ヒントの不一致）**: 本レポートはフェーズ最初の実測である A1-T の BEFORE = `963 passed / 0 failed`（`cargo test --workspace`）をベースラインとして採用し、最終 X1-V / final-launch HEAD の `1713 passed / 0 failed` を到達点とする（fragment-derived。差分 **+750**、内訳は各セルの追加テストの累積からデッドコード除去に伴う削除 4 件〔W1-S −1・W4a-S −1・W6a-S −2〕を差し引いたもの）。tasks.md「Implementation Notes」は累積成果を「テスト ~1424→1713（+289 特性化テスト）」と記すが、これは断片の連続的な件数推移（963→…→1713）と一致しないため**不一致として記録する**（指示「断片が真実源」に従い断片の 963→1713 を採用）。「~1424」はおそらく途中時点の再測定値であり、本レポートでは断片の実測連鎖を正とする。

### セル別件数推移（断片の AFTER 実測・連続検証の証跡）

| 領域 | T | S | V | 領域完了時の workspace 件数 |
|------|---|---|---|---|
| A1 | 963→984（areka +21） | ±0 | +1（→994） | 994 |
| D1a | +38（→1032） | ±0 | +5（→1037） | 1037 |
| D1b | +36（→1073） | net −144 行・件数 ±0 | +14（→1087） | 1087 |
| D2 | +28（→1115） | net −76 行・件数 ±0 | +11（→1126） | 1126 |
| D3 | +34（→1160） | net −29 行・件数 ±0 | +16（→1176） | 1176 |
| W1 | +34（→1210） | −1（→1209、削除テスト 1） | +10（→1219） | 1219 |
| W2 | +79（→1298） | ±0 | +1（→1299） | 1299 |
| W3a | +52（→1351） | net −262 行・件数 ±0 | +6（→1357） | 1357 |
| W3b | +32（→1389） | net −36 行・件数 ±0 | +3（→1392） | 1392 |
| W4a | +32（→1424） | −1（→1423、削除テスト 1） | ±0（1423） | 1423 |
| W4b | +23（→1446） | ±0 | +5（→1451） | 1451 |
| W5a | +16（→1467） | ±0 | +1（→1468） | 1468 |
| W5b | +18（→1486） | ±0 | +2（→1488） | 1488 |
| W6a | +28（→1516） | −2（→1514、削除テスト 2） | +2（→1516） | 1516 |
| W6b | +50（→1566） | ±0 | +2（→1568） | 1568 |
| W7a | +43（T1）+14（T2）（→1625） | ±0 | +3（→1628） | 1628 |
| W7b | +18（T1）+21（T2）（→1667） | ±0 | ±0 | 1667 |
| W8 | +33（T1、cue_performance 決定論化含む）+12（T2）（→1712） | ±0 | +1（→1713） | 1713 |
| X1 | no-change | ±0（launch.json 是正のみ） | no-change | 1713 |

最終: **1713 passed / 0 failed / 32 ignored**（`--all-features` で 1716）。S7 起動テスト PASS。

---

## 2. マトリクス網羅性

全 19 領域 × 3 観点 = 57 論理セルが断片へ対応する。下表で各セルの断片の有無を確認する（T セル事前分割の W7a / W7b / W8 は T 列にサブ断片を併記）。**全 60 断片ファイルが存在することを実測確認済み**（`report/cells/` の Glob 列挙で 60 セル断片 + phase0-matrix + final-launch を確認）。

| 領域 | T（テスト網羅性） | S（シンプル化） | V（脆弱性） |
|------|------------------|----------------|------------|
| A1: areka エントリポイント | A1-T ✓ | A1-S ✓ | A1-V ✓ |
| D1a: dola ランタイム中核 | D1a-T ✓ | D1a-S ✓ | D1a-V ✓ |
| D1b: dola 補間・状態 | D1b-T ✓ | D1b-S ✓ | D1b-V ✓ |
| D2: dola コンパイル・DSL | D2-T ✓ | D2-S ✓ | D2-V ✓ |
| D3: dola 検証・Cue | D3-T ✓ | D3-S ✓ | D3-V ✓ |
| W1: wintf レガシー・プロセス | W1-T ✓ | W1-S ✓ | W1-V ✓ |
| W2: wintf COM層 | W2-T ✓ | W2-S ✓ | W2-V ✓ |
| W3a: wintf コンポジタ・描画 | W3a-T ✓ | W3a-S ✓ | W3a-V ✓ |
| W3b: wintf グラフィックス資源 | W3b-T ✓ | W3b-S ✓ | W3b-V ✓ |
| W4a: wintf taffy・配置 | W4a-T ✓ | W4a-S ✓ | W4a-V ✓ |
| W4b: wintf ヒットテスト・計測 | W4b-T ✓ | W4b-S ✓ | W4b-V ✓ |
| W5a: wintf テキスト描画 | W5a-T ✓ | W5a-S ✓ | W5a-V ✓ |
| W5b: wintf 図形・画像・ブラシ | W5b-T ✓ | W5b-S ✓ | W5b-V ✓ |
| W6a: wintf ポインター入力 | W6a-T ✓ | W6a-S ✓ | W6a-V ✓ |
| W6b: wintf ドラッグ | W6b-T ✓ | W6b-S ✓ | W6b-V ✓ |
| W7a: wintf ウィンドウ・メッセージ | W7a-T1 ✓ / W7a-T2 ✓（事前分割） | W7a-S ✓ | W7a-V ✓ |
| W7b: wintf ECS基盤・World | W7b-T1 ✓ / W7b-T2 ✓（事前分割） | W7b-S ✓ | W7b-V ✓ |
| W8: wintf Cue・Dola統合 | W8-T1 ✓ / W8-T2 ✓（事前分割） | W8-S ✓ | W8-V ✓ |
| X1: 横断プロジェクト設定 | X1-T ✓（no-change） | X1-S ✓ | X1-V ✓（no-change） |

**網羅性の結論**: 欠落セルは **ゼロ**。全 57 論理セル（60 断片）が存在し、tasks.md「マトリクス網羅性記録」と完全一致する。no-change として記録されたセルは **X1-T・X1-V・final-launch（タスク21）** の 3 件のみで、いずれも「点検の結果、是正すべき変更が境界内に存在しなかった」正当な無変更（補完記録ではない＝断片が実在する）。補完を要する真の欠落セルは存在しない。フェーズ文脈断片として `phase0-matrix.md`（タスク 1.4 のマトリクス網羅性・実行プロトコル確認、結果 PASS）も存在する。

---

## 3. 領域×観点 実施結果

各領域の T（追加/除外テスト）・S（簡素化・デッドコード除去）・V（脆弱性所見と対応）を要約する。全 19 領域を漏れなく記載する。

### A1: areka エントリポイント（`crates/areka/src/main.rs`、単一 399 行・改善前テスト 0）

- **T**: headless ユニットテスト **21 件**追加（`build_typewriter_tokens` 5・`create_shell/balloon_window` 6・`run_setup` 1・`on_shell_pressed` 3・`on_shell_drag` 5・`SHELL_IMAGE_PATH` 1）。除外 0。バイナリクレートゆえ in-source `#[cfg(test)]` が唯一の選択肢。GUI/COM 依存の `main()` は S7 が回帰検知。
- **S**: 挙動非破壊の簡素化 4 件（中間 `Vec` 割当除去・未使用引数 `_shell_entity` 削除・`Or` 単一クエリ統合・let-else 統合、main.rs 19 ins / 28 del）。`deprecated` は areka 配下に 0 件。
- **V**: panic 経路 6 種を点検し外部入力到達の DoS 経路なしと確定。`on_shell_drag` のオフセット加算に `debug_assert` + 不変条件コメント、`RUST_LOG` フォールバックに安全コメント、境界値テスト 1 件追加（21→22）。提案 **P1〜P4**（キュー検査 API・バルーン位置一元化・`SHELL_IMAGE_PATH` のビルドマシン絶対パス埋め込み・起動経路の無音失敗可観測化）。

### D1a: dola ランタイム中核（facade / loop_controller / timeline_manager / subscription_manager / playback、約1,450 行・unwrap 多数）

- **T**: テスト空白 **38 件**追加（facade エラーパス 18・timeline 内部評価・購読境界 20）。除外 0。
- **S**: `start_internal` / `calculate_end_time` のバリデーション二重実装を私有ヘルパ `compile_and_validate` へ統合、`playback.rs` の陳腐化 TODO 除去（facade 34/41・playback 0/1）。提案 **P6**（到達不能な is_terminal 防御分岐）・**P7**（イージング重複 loop_controller↔interpolator、セル境界跨ぎ）。
- **V**: unwrap/expect の panic 経路に SAFETY コメント + debug_assert、`time_scale` の inf/NaN ハザードと `process_loops` 周回キャッチアップを NOTE 化、特性化テスト 5 件（→1037）。提案 **P5**（playback 旧型整理）・**P8**（time_scale 入力検証）・**P9**（周回反復上限 DoS 耐性）。

### D1b: dola 補間・状態（conflict_resolver / interpolator / instance_manager / storyboard / value 他、約1,830 行・unwrap 多数）

- **T**: **36 件**追加（競合解決・補間・状態遷移・ObjectInternPool・Hash/Eq 契約）。除外 0。`time_shifted_start...` と `overlap_with_multiple...` は仮説 RED→実挙動発見で特性化へ転換。
- **S**: dead code `resolve_conflicts`（非除外ラッパ）削除 + 競合解決 4 終了経路を `terminate_instance` へ統合 + 陳腐化 TODO 5 ファイル除去（8 ファイル・**net −144 行**、65 ins / 209 del）。提案 **P13**（document_store::get_storyboard、テスト専用 dead code）。
- **V**: NaN/inf/ゼロ除算の数値境界に NOTE・SAFETY・debug_assert、特性化テスト **14 件**（→1087）。提案 **P10**（DynamicValue Hash/Eq 契約違反）・**P11**（競合検出の wall-clock 非対応）・**P12**（trigger_store 残置リーク）・**P14**（指示書数値の有限性検証）・**P15**（resume 非単調時刻）・**P16**（InvalidGroupId 混同）。

### D2: dola コンパイル・DSL（compile/ / builder / error、約1,260 行・in-source テストなし）

- **T**: **28 件**追加（複数エラー収集・loop_offset 伝播・純粋KF継承・KeyframeRef 全バリアント・Display 全文一致）。除外 0。
- **S**: `topological_sort` の死コード除去 + `BinaryHeap` 化、誤解を招く関数 `find_previous_entry_in_sort_order` 削除、最遅時刻ロジック重複統合、unwrap 排除、陳腐化 TODO 除去、テストヘルパ重複解消（5 ファイル・**net −76 行**、57 ins / 133 del）。提案 **P17**（診断精度）・**P18**（到達不能防御分岐）・**P19**（純粋KF 暗黙依存がグラフ未反映）。
- **V**: デシリアライズ境界・添字・再帰深度を点検し panic 経路なしと確定、SAFETY + debug_assert + NOTE、特性化テスト **11 件**（→1126）。提案 **P20**（delay/duration 負値検証）・**P21**（`__implicit_` 名前衝突）。

### D3: dola 検証・Cue（validate/ / cue/ / document / lib、約1,360 行・validate/ 未テスト）

- **T**: **34 件**追加（複数ルールのエラー蓄積・V6/V10/V12/V13 補完・トリガー循環・cue 配信順・cue serde）。除外 0。同時刻配信順の insert=FIFO / extend=LIFO 不整合を発見（P22）。
- **S**: トリガー検証 4 ブロックの単一 `if let` 統合 + 空 if（V17t）除去、V10/V12 重複ループ化、tick() の冗長 Timeout ブロック除去、陳腐化 TODO 除去（4 ファイル・**net −29 行**、35 ins / 64 del）。提案 **P23**（dfs 再帰の反復化）・**P24**（変数値域 Float/Integer 統合）。
- **V**: バリデーション網羅性・Cue 時刻境界・panic 経路を点検、SAFETY + NOTE、特性化テスト **16 件**（→1176）。提案 **P25**（Cue パイプライン時刻検証）・**P26**（loop_count 文書レベル検証）。

### W1: wintf レガシー・プロセス（win_state / win_style / process_singleton / api + 非推奨3モジュール、約2,480 行・非推奨 1,838 行含む）

- **T**: 非・非推奨4モジュールに **34 件**追加（win_style 24・win_state 6・api 2・process_singleton 2、すべて in-source）。除外 0。非推奨3モジュールは削除候補のためテスト追加対象外（調査所見のみ）。**重要発見**: `#![deprecated]` を持つのは `win_message_handler.rs` のみで、`winproc.rs` / `win_thread_mgr.rs` は `#![allow(deprecated)]` のみの現役（steering structure.md の記載が不正確 → P29）。
- **S**: **非推奨コードの利用実証調査を実施し削除実施はゼロ**（R2.9 の「利用ゼロ」を実証できる非推奨モジュールが存在しなかった）。残存コードに挙動非破壊の整理 6 件（未使用 private `set_ex2` 削除・no-op `WS_TILED` 削除・doc コメント修正・dead `hidden_window` 削除・winproc 構造整理・ブランケット lint 抑制削減、5 ファイル +11 / −57、削除テスト 1）。提案 **P27**（win_message_handler 削除セット、利用3件の一括移行）・**P28**（winproc::get_boxed_ptr の健全性違反）・**P29**（steering 記載乖離）。
- **V**: 単一実行制御・Win32 ラッパー境界・panic/unsafe を点検、SAFETY + NOTE + 非発火 debug_assert、スレッドライフサイクル特性化テスト **10 件**（→1219）。提案 **P30**（CoUninitialize 欠如・Box リーク）・**P31**（WinProcessSingleton 非冪等初期化）・**P32**（多重生成時の ECS_WORLD 束縛固定）。

### W2: wintf COM層（`crates/wintf/src/com/`、約2,360 行・unsafe 最密集）

- **T**: com ドメイン統合テスト **79 件**追加（command_types 12・command_sink 9・d2d/mod 7・dcomp 17・d3d11 5・dwrite 11・wic 8・animation 7、新規 `tests/com.rs` 束ね役 + 7 ファイル）。除外 0。実デバイス生成の前例（GraphicsCore::new ヘッドレス）に準拠。
- **S**: 構造的整理 4 件（空 dxgi.rs 削除・wic の生ポインタ→参照置換 2 件・dwrite 手書き wcslen→`as_wide` 置換・dcomp の型表記統一）。提案 **P33**（録画モジュール完成/削除・todo! パニック・Clone COM リーク）・**P34**（GetClusterMetrics エラー黙殺）・**P35**（D2D1CommandListExt::open スタブ削除）。
- **V**: unsafe 境界（pitch<stride OOB・draw_text unwrap・COM コールバック生ポインタ）に debug_assert 10 件 + SAFETY 根拠、空 HSTRING 特性化テスト 1 件（→1299）。提案 **P36**（Ref::unwrap の COM ABI 境界越え abort）。

### W3a: wintf コンポジタ・描画（compositor.rs / compositor_systems / systems{init,render,surface,clip_sync} / components、約2,090 行・unsafe 多数）

- **T**: ヘッドレスギャップテスト **52 件**追加（DIB ピクセル直接読み出しで合成結果を画素検証、ClipGuard 3 バリアント等）。除外 0。
- **S**: 廃止済み dead code **3 件削除**（draw_recursive 59 行・sync_surface_from_arrangement 128 行 + create_surface_for_visual 27 行・init_window_visual 20 行）+ ClipGuard レイヤパラメータ共通化 + doc 見出し整理（5 ファイル・**net −262 行**、32 ins / 294 del）。提案 **P39**（render_surface 未使用パラメータ）。
- **V**: unsafe 境界・panic・デバイスロスト・整数変換を点検、SAFETY + NOTE + debug_assert 2 件、特性化テスト 6 件（→1357）。提案 **P37**（赤デバッグ枠常時描画・DIB 全画素スキャン）・**P38**（ClipGuard geometricMask transmute の COM リーク）・**P40**（デバイスロスト検出の欠如）・**P41**（負サイズ入力の i32→u32 ラップ）・**P42**（create_dib_section GDI エラー経路）。

### W3b: wintf グラフィックス資源（visual / visual_manager / clip / core / dcomp_resource / command_list / systems{brushes,visual_sync,window_pos}、約2,010 行・unsafe 多数）

- **T**: **32 件**追加（resolve_inherited_brushes 純粋 ECS ロジック 10・property_sync 5・window_pos systems 7 他）。除外 0。
- **S**: on_visual_add の commands 三重借用を単一ブロック化 + 未使用パラメータ整理 + デバッグ残骸削除（4 ファイル・**net −36 行**、28 ins / 64 del）。提案 **P45**（visual_resource_management_system 未使用 Commands）・**P46**（apply_window_pos_changes 重複 debug ログ）。
- **V**: unsafe 境界・生成/破棄対称性・デバイスロスト再初期化・panic を点検、SAFETY + NOTE、特性化テスト 3 件（→1392）。**command_list.rs の誤コメント是正**（「windows-rs のスマートポインタは Send+Sync」は事実誤認 → 正しい SAFETY 根拠へ書換、§7 横断補正と関連）。提案 **P43**（SetWindowPosCommand 観測 API）・**P44**（resolve_inherited_brushes フィールド単位継承）・**P47**（再ペアレント未検出の孤立 Visual）・**P48**（ChildOf 祖先走査の巡回ガード欠如）。

### W4a: wintf taffy・配置（taffy / arrangement / box_style / dimension / systems{taffy,arrangement} / LayoutRoot、約1,290 行・unwrap あり）

- **T**: **32 件**追加（dimension 変換 13・arrangement アクセサ/フック 10・box_style 7・component_hooks 2）。除外 0。
- **S**: apply_box_size ヘルパ統合・P50 スタブ削除（`From<taffy::Dimension>` 常時 Auto）・未使用クエリ項削減・available_space クロージャ化・SAFETY 注記追加・死コード除去（7 ファイル、削除テスト 1〔P50 スタブの特性化〕→1423）。
- **V**: panic 経路ゼロ・整数算術なし・ゼロ除算なしを確認、`TaffyLayoutResource` の SAFETY 注記強化のみ（件数 ±0、1423 維持）。提案 **P49**（LengthPercentageAuto/LengthPercentage の÷100 正規化欠落）・**P50**（From<taffy::Dimension> スタブ）。

### W4b: wintf ヒットテスト・計測（hit_test / hit_region / metrics / rect / systems{monitor,window_pos}、約1,970 行・テスト比較的厚い）

- **T**: **23 件**追加（hit_test ex 3・hit_region 4・metrics 9・rect 5・monitor 2）。除外 0（過不足整理: 不足のみ）。
- **S**: 重複・崩れたテストバナーコメント整理 2 件（コメントのみ・件数 ±0）。提案 **P52**（hit_test_entity / hit_test_entity_ex の重複統合、NamedRegions 挙動差が前提）。
- **V**: 飽和キャストの安全鎖・ゼロ除算根拠・モニタ境界を点検、index_map 不変条件 debug_assert + 特性化テスト 5 件（→1451）。提案 **P51**（BitmapSourceResource テスト用コンストラクタ）・**P53**（ColorMapData::from_image の u32 乗算オーバーフロー）。

### W5a: wintf テキスト描画（`crates/wintf/src/ecs/widget/text/`、約1,370 行・unsafe あり）

- **T**: **16 件**追加（TypewriterTalk 状態マシン 12・typewriter_ir FireEvent 2・Label/TextLayoutResource 既定値 2）。除外 0。draw 系は全面 DirectWrite 依存でテスト不能。
- **S**: HookContext の修飾正規化 1 件（label.rs +2/−2・件数 ±0）。提案新規なし（P54 参照）。
- **V**: `TypewriterLayoutCache` の手動 `unsafe impl Send/Sync` を点検し**健全かつ冗長**（windows-rs が IDWriteTextLayout に Send+Sync を付与済み）と確定、SAFETY 注記 + 静的特性化テスト 1 件（→1468）。提案 **P54**（convert_to_timeline の純粋ロジック分離、W5a-T 由来）。**横断発見（§7 で集約）**: `graphics/command_list.rs:29-33` の SAFETY コメントが windows-rs の COM 型を一律 !Send/!Sync と誤記している事実を CONCERNS 記録。

### W5b: wintf 図形・画像・ブラシ（shapes / bitmap_source / brushes.rs、約1,680 行・unsafe あり）

- **T**: **18 件**追加（rectangle 4・alpha_mask 5・bitmap_source resolve_path 等 3・brushes 6）。除外 0。`bitmap_source/` の分離テストパターンを応用。
- **S**: `Brush` の手動 Default を derive 化 + AlphaMask の `div_ceil` 化 2 箇所（2 ファイル・net −5 行、4 ins / 9 del）。提案新規なし（P55 参照）。
- **V**: WIC の `unsafe impl Send/Sync` を windows-rs ソース実証で**健全かつ必須**（WIC 型は Send 未生成）と確定、SAFETY 注記 crate 標準化 + is_hit 添字 debug_assert + 特性化テスト 2 件（→1488）。提案 **P55**（generate_alpha_mask_system の u32 乗算オーバーフロー）・**P56**（resolve_path のパストラバーサル検証欠如）。

### W6a: wintf ポインター入力（`crates/wintf/src/ecs/pointer/`、約1,830 行・テスト薄め）

- **T**: **28 件**追加（types 9・buffers 9・systems 5・dispatch 5）。除外 0。**P57 のデッドコードを発見**（process_pointer_buffers がワークスペース全域で未登録）。
- **S**: **デッドコード実証削除**: `process_pointer_buffers` / `process_mouse_buffers`（本番呼び出しゼロを grep 実証）+ `push_mouse_sample`（pub(crate) dead）+ 再エクスポート + 特性化テスト 2 件を削除（4 ファイル・**+3 / −302 行 = net −299**、削除テスト 2 →1514）。境界拡張として ecs/mod.rs の再エクスポート 2 行も削除（タスク明示許可）。提案 **P58**（transfer_buffers_to_world のボタン転送 match 重複）。
- **V**: 本番経路に到達可能な panic ゼロ、velocity 不変条件 debug_assert + 座標キャスト根拠コメント + 特性化テスト 2 件（→1516）。提案 **P59**（WHEEL/DOUBLE_CLICK_BUFFERS 未消費・ホイール未反映ギャップ）・**P60**（thread_local HashMap キー単調増加、現状安全）。

### W6b: wintf ドラッグ（`crates/wintf/src/ecs/drag/`、約1,410 行・テスト薄め）

- **T**: **50 件**追加（mod 6・state 21・accumulator 8・context 6・dispatch/cleanup 統合 9）。除外 0。**P61 のデッドストアを発見**（DraggingState.prev_frame_pos）。
- **S**: **デッドストア実証削除**: `prev_frame_pos`（本番読み取りゼロを grep 再実証、デルタは別ソース算出）の定義・2 書込・専用 get_mut ブロック削除 + clippy 簡素化 6 件（const thread_local・auto-deref 2・derive Default・useless_conversion 3、5 ファイル・net −36 行、件数 ±0）。提案新規なし（P61 を解消）。
- **V**: キャプチャ解放保証（取得二重防止・遷移時 move・終了/キャンセル解放）を実コード裏取りし健全と確定、解放保証 debug_assert 2 + check_threshold ハザードコメント + 特性化テスト 2 件（→1568）。提案 **P62**（check_threshold 本番未使用 + インライン複製 + i32 桁あふれ境界）。

### W7a: wintf ウィンドウ・メッセージ（ecs/window/ + ecs/window_proc/、約2,630 行・window/ 未テスト・unsafe あり。T 事前分割）

- **T（T1=window/ 43 件 + T2=window_proc/ 14 件 = 57 件）**: window/ の dpi/window_pos/command/components/monitor 等に 43 件、window_proc/ の dpi_helpers/find_ancestor_with_drag_config/collect_entities_to_leave に 14 件。除外 0。メッセージパラメータ抽出のインライン重複を所見化（P64）。
- **S**: ZOrder の手書き Default → derive 化 + 同型恒等 `.into()` 除去 8 件（2 ファイル・net −6 行・clippy 9 件解消・件数 ±0）。提案 **P65**（create_windows の CompositionMode→ex_style 分岐の純粋関数抽出）。P64 は維持・見送り。
- **V**: 手動 `unsafe impl Send/Sync` **5 型**（Window/WindowHandle/ZOrder/WindowPos/SendWeak）を windows-rs ソース実証で**健全かつ必須**（HWND/HINSTANCE は `*mut c_void` で Send/Sync 未生成）と確定、SAFETY 注記 crate 標準化 5 箇所 + Send+Sync 静的特性化テスト 3 件、HWND ライフサイクル（生成⇔破棄対称・USERDATA クリアで use-after-free 排除）健全確認（→1628）。提案新規なし（P63〜P65 は参照）。

### W7b: wintf ECS基盤・World（ecs/common/ + ecs/world/ + ecs/app.rs、約2,700 行・world/ 未テスト。T 事前分割）

- **T（T1=common/ 18 件 + T2=world/+app.rs 21 件 = 39 件）**: common/ のジェネリック階層伝播（tree_iter 4 + tree_propagation 統合 14）、world/+app.rs のウィンドウカウント/スケジュールラベル/FrameCount/EcsWorld（app 6 + schedule_labels 7 + world_lifecycle 統合 8）。除外 0。
- **S**: App の手書き Default → derive 化 1 件（app.rs +1/−11・件数 ±0）。type_complexity 3 件は bevy 上流ミラー保護 + 領域全体での受容規約で見送り。提案新規なし。
- **V（REJECTED 1 回・第2ラウンドで是正）**: スケジュール順序（登録順==実行順を実コード照合）・atomic ordering（VSYNC_TICK_COUNT は SPSC・Relaxed 妥当）・FrameCount 消費形態を裏取りし、stale なスケジュール順序コメント是正 + FrameCount 整数境界ハザードコメント + atomic ordering 妥当性コメント（3 件すべてコメントのみ・件数 ±0）。提案 **P66**（FrameCount u32 加算オーバーフロー堅牢化）。

### W8: wintf Cue・Dola統合（ecs/cue/ + ecs/dola/、約1,520 行・in-source テストゼロ・フレーキーテスト所在域。T 事前分割）

- **T（T1=cue/ 32 件 + フレーキー決定論化 + T2=dola/ 12 件 = 44 件）**: cue/ の CueQueue/EntityRegistry/CueSheetTracker に in-source 32 件、dola/ の DolaAnimator 委譲契約・System 配線に in-source 12 件。除外 0。**`cue_performance_test` を決定論化**（§5）。`tracker_timeout` 容疑は偽陽性と確定。T1/T2 とも特性化中に各 1〜3 件が初回失敗→真の挙動発見でテスト前提修正（プロダクション不変・正常収束）。
- **S**: `DolaAnimator` の `unsafe impl Send+Sync` の SAFETY 注記を crate 標準様式へ格上げ 1 件（dola/mod.rs +11/−4・件数 ±0）。collapsible_if 5 件は churn 回避で見送り。提案 **P67**（dispatch_cue_sheet_internal の配送アーム重複統合、未保護分岐ゆえ記録のみ）。
- **V（最終セル）**: `unsafe impl Send+Sync for DolaAnimator` の健全性を**実スケジュール構成で裏取り**（既定マルチスレッドエグゼキュータ・現状未配線で到達不能・write 経路は排他で安全）し、W8-S の SAFETY 注記の「単一スレッド実行」根拠を実構成へ是正、Entity 不正ビット panic を `#[should_panic]` で特性化（→1713）。提案 **P68**（DolaAnimator 配線時の Sync ハザード）・**P69**（resolve_entity_ref の try_from_bits 化）。

### X1: 横断プロジェクト設定（ルート Cargo.toml / 各クレート Cargo.toml / .gitignore / .gitmodules / .vscode/）

- **T（no-change）**: テストエントリポイントの束ね規約（15 エントリ・124 ファイルを comm 突き合わせ→孤児 0 / dangling 0）・feature 組合せ・dev-dependencies 整合を点検し、設定起因のテスト漏れなしと確定。唯一の構成ギャップ（S2 が非既定 feature を実行しない）は P70 へ。CI 欠落・publish=true 上書きを所見記録。
- **S**: 陳腐化 launch.json の是正 2 件（`sample_dcomp.exe` → `areka.exe`・誤配置 `tasks` キー削除、エディタ設定ゆえ挙動非破壊・件数 ±0）。提案 **P71**（publish=true 上書きと要件前提の矛盾）・**P72**（profile.release 最適化見直し）。
- **V（no-change）**: 依存監査 `cargo audit` 0.22.2 を実行（1132 advisories・300 crate 依存スキャン）= **脆弱性 0 件 / 情報的警告 5 件**。プロダクション混入は rand unsoundness（RUSTSEC-2026-0097）のみで発火条件が独立に 2 つ不成立＝到達不能、残り（paste/core2）は dev-only で出荷物非混入。依存固定は `*` ゼロで健全、`.gitignore`/`.gitmodules` に機密漏洩経路なし。変更ゼロ。提案 **P73**（rand パッチ更新）・**P74**（cargo audit の CI 導入）・**P75**（Cargo.lock 追跡方針）。

---

## 4. 巻き戻し記録

### 巻き戻し（rollback）

**巻き戻し 0 件。** 全 60 断片のいずれにも `status: rolled-back` は存在せず、kiro-debug の BLOCK_TASK / 2ラウンド失敗による直近正常コミットへの復元は一度も発生しなかった。全セルが「調査 → 改善 → 自己レビュー → 検証（S2 グリーン）→ コミット」のゲートを通過した（変更なしの no-change セル 3 件も docs コミットとして正常記録）。

### レビュー差し戻し（REJECTED）remediation 記録

自己レビュー（kiro-review）での差し戻しは累計 **3 回**。いずれも第2ラウンドで是正され完了した（出典: tasks.md「Implementation Notes」進捗記録）。

| # | セル | ラウンド | 是正内容（根拠: 各断片の「本番挙動主張の裏取り」節が REJECTED 教訓を反映） |
|---|------|---------|------|
| 1 | W5a-T | 第1→第2 | 第2ラウンドで是正済み。W5a 系は以降「windows-rs ソース直接確認」「コンパイル・プローブで冗長性実証」を徹底（W5a-V がこの裏取り方針を確立し、command_list.rs の事実誤認を CONCERNS として捕捉）。 |
| 2 | W6a-T | 第1→第2 | 第2ラウンドで是正済み。`process_pointer_buffers` の「本番未登録（デッド）」主張を commit 6e7e1ea の実コード（git show + grep で add_systems ゼロ）で裏取りし、`ComputeTaskPool` 初期化済みの事実も確認（W6a-T/S/V がこの裏取りを継続）。 |
| 3 | W7b-V | 第1→第2 | 第2ラウンドで是正済み。スケジュール登録順==実行順・atomic アクセスマップ・FrameCount 消費形態を実コード grep + 精読で全数裏取り（以降の W8-V もこの「未確認本番事実主張による REJECTED 回避」教訓を継承し、bevy_ecs ソース直読 + probe 実測を徹底）。 |

### 敵対的レビューが捕捉した事実誤認（3 件・是正済み）

tasks.md Implementation Notes が記す「事実誤認3件を敵対的レビューが捕捉是正」に対応する。断片から確認できる該当事象:

1. **steering structure.md の非推奨記載乖離**（W1-T/W1-S）: 「3モジュールとも `#[deprecated]`」という記載に対し実態は `win_message_handler.rs` のみ → P29 として是正記録。
2. **command_list.rs の SAFETY コメント誤記**（W5a-V / W3b-V / W7a-V が CONCERNS 記録）: 「windows-rs の COM スマートポインタは自動では Send/Sync にならない」という blanket 主張が型依存で普遍的に正しくない（D2D/DWrite 型は付与済み）→ W3b-V が当該ファイルのコメントを正しい SAFETY 根拠へ是正、§7 で最終是正候補として集約。
3. **W8-S SAFETY 注記の「単一スレッド実行」根拠**（W8-V が是正）: DolaAnimator が単一スレッドで実行されるという根拠が実構成（既定マルチスレッドエグゼキュータ）と不整合 → W8-V が実コード裏取り事実（排他アクセス + 未配線で到達不能）へ是正。

加えて、特性化テスト作成中に「実装の隠れた挙動」を炙り出した正常収束が複数あった（D1b-T の wall-clock 競合判定、W8-T1 の tick 完了後非冪等、W8-T2 の dola 差分初回配信、W8-V の Entity::from_bits panic 条件）。いずれもプロダクション挙動は不変でテスト前提を実挙動へ合わせた。

---

## 5. フレーキー判定記録

| 対象 | 判定 | 判定根拠 | 最終状態 |
|------|------|---------|---------|
| `wintf tests/ecs cue_performance_test::bench_pop_ready_empty_queue`（および同ファイルの実時間ベンチ全 5 件） | **負荷依存フレーキー → W8-T1 で決定論化して解消** | フェーズ0実測で「3回中1回失敗、隔離2回安定合格」＝負荷依存と確定（壁時計しきい値 `elapsed < 1ms` が並列ビルド・CI 負荷でプリエンプションされ偽陽性化）。A1-S/D2-S/D2-V/D3-S/W1-T/W1-S/W2-V/W3a-S/W4b-T/W4b-V/W5b-S 等で「隔離再実行で安定合格・境界外」のパススルー判定を反復記録（design フレーキー判定規則準拠）。 | W8-T1 で実時間しきい値を完全除去し正当性検証（件数整合・全件保持・部分到達残置）へ置換（5→6 件）。隔離 5 回・並列負荷 5 多重・バイナリ 3 回連続で全 `failed=0`。**非フレーキー化完了**。以降の全セル（W8-S/V・X1）で安定合格。 |
| `cue_tracker_lifecycle_test::tracker_timeout`（research.md の容疑） | **偽陽性確定（タイミング非依存）** | W8-T1 が精査: 当該テストは `QueueSnapshot { timed_out: true/false }` という**プレーン bool 入力値**を tracker.update に渡すのみで、`std::time::Instant`・`thread::sleep`・実時間 elapsed を一切使用しない（grep 実証: 時間 API ゼロ）。本番では `check_timeout(current_time)` が算出するがテストは固定 bool を直接注入＝完全決定論。 | 改修不要（既に決定論的）。安定化対象外。 |
| W8-T1 における決定論化（W8-T1 の追加成果物） | 実施・完了 | tasks.md タスク 19.1 の「既知フレーキー安定化を改善対象に含める」を充足。`cue_performance_test.rs` を書き換え（正当性アサーションは維持・強化、実時間しきい値のみ除去）。特性化中に「空キューは最初の tick で Completed 遷移」という真の挙動を発見し state アサーションへ反映。 | 上記のとおり解消済み。 |

**その他のフレーキー判定**: 上記以外に新規フレーキーの導入はゼロ。各セルの追加テストはいずれも純粋ロジック / bevy World 上の決定論的検証（時刻は注入値・実 clock を使わない）であり、タイミング依存なし。

---

## 6. 最終起動テスト（S7）結果

出典: `final-launch.md`（タスク21）。**最終判定: S7 PASS。source code 変更なし（修正不要で合格）。**

全 60 セル完了後（HEAD = `8e4809e` の X1-V まで、クリーンワークツリー）の最終起動ゲート。`RUST_LOG=info` で `target/debug/areka.exe` を起動し、初期化完了ログ `[GraphicsCore] Initialization completed`（`crates/wintf/src/ecs/graphics/core.rs:55` の `info!`）の出現を 60 秒タイムアウトのポーリング（200ms 間隔・ANSI 除去）で監視した。

| 合格要件 | 結果 | 実測根拠 |
|---------|------|---------|
| 初期化完了ログがタイムアウト（60秒）内に出現 | **PASS** | アプリログ時刻 **+0.208 秒**（spawn `03:17:38.310Z` → init-complete `03:17:38.518040Z`）で捕捉。補助ログ「シェルウィンドウとバルーンウィンドウを生成しました」も +0.118 秒で出現。 |
| パニック・error レベルログがない | **PASS** | 結合ログの正規表現スキャン: `panicked`/`panic` ヒット 0、tracing `ERROR` 0、`error` 文字列 0、`WARN` 0。stderr **0 バイト**。全ログ INFO レベルのみ。 |
| 異常終了コードがない | **PASS** | 起動後 +0.2 秒で初期化完了し監視中ずっと正常稼働（自己 panic / 非ゼロ終了なし）。検証完了後に `Stop-Process -Force` で意図的終了（成功）、終了後 PID 消滅・ストレイ areka プロセス 0。 |

3 要件すべて充足。フェーズ0実測（起動約1秒で出現・stderr 空・正常稼働）とも整合。GUI/COM/DirectComposition 領域を含む全 60 セルの改善（テスト追加・デッドコード除去・フレーキー決定論化・SAFETY 注記格上げ等、いずれも挙動非破壊）の最終統合証拠として、areka は panic/error なく約 0.2 秒で初期化完了し正常稼働した。kiro-debug による解消・bisect・巻き戻しは不要（失敗時手順は発動せず）。

---

## 7. 新規仕様提案（優先度付き）

`report/proposals.md` の P1〜P75（全 75 エントリ）を重複統合し、テーマ別・優先度付きで整理する。**全 P 番号の対応関係は各提案の「統合元」に明記し、原 P 番号への追跡性を保持する**（P1〜P75 はいずれかの統合提案に必ず帰属する）。

優先度の基準: **高（P1）= 出荷物に混入する健全性違反 / 外部入力到達の DoS・整合性侵害で、放置すると将来の機能追加・配線時に顕在化するもの**。**中（P2）= 潜在バグ・正確性改善・ビルド再現性 / 検証スコープ・デッドコード整理で、現状実害は限定的だが品質上の負債**。**低（P3）= 診断改善・cosmetic・churn 回避で見送った構造整理**。

各統合提案は「本ループで実装しなかった根拠」（挙動変更を伴うため R2.4/R5.2、または利用実証不能な削除のため R2.10、またはセル境界跨ぎ）を引き継ぐ。

### 優先度 高（P1）

#### NSP-1: 入力検証ハードニング・スイート（dola 数値フィールド + Cue パイプライン + Entity ビット）
- **統合元**: P8（time_scale 正値検証）・P14（指示書数値フィールドの有限性検証）・P20（delay/duration 負値検証）・P25（Cue パイプライン時刻入力検証）・P26（loop_count 文書レベル検証）・P69（resolve_entity_ref の try_from_bits 化）・**P9（process_loops 周回キャッチアップ反復上限＝外部到達の時刻ジャンプ DoS 耐性）**。**P21（`__implicit_` 名前予約）** も検証層の同族として本スイートに含める。
- **種別**: 挙動変更を伴う脆弱性対策 / 検証網羅性改善。
- **根拠**: dola の指示書（TOML/YAML 経由で NaN/inf/負値が流入）と Cue パイプライン（NaN start_time の partition_point 前提破壊・tick(NaN) の全件即時配信・inf オフセットの liveness 喪失）、および外部 CueSheet 由来の Entity ビット（`Entity::from_bits` が下位 index ワード 0 で panic）は、いずれも**外部入力到達可能なリソースリーク・整合性侵害・DoS panic 経路**である。tasks.md の dedup ヒット（P8 time_scale + P14 finiteness + P20 sign は同一仕様ファミリ）に対応。現状は発火経路が未実装ゆえ実害未発現だが、設定/スクリプト由来でドキュメントやウィジェットを構築する構成が入ると顕在化する。全件、現行挙動を特性化テストで固定済み（P8 の time_scale_boundary 5 件、P14 の NaN 系 14 件、P25 の cue 境界 8 件、P69 の should_panic 1 件等）。
- **推奨スコープ**: dola にバリデーション層を一括導入する新規仕様。(a) DolaDocument の validate() に「全数値フィールドは有限・time_scale は正・delay/duration は非負・loop_count は -1 または ≥1」を追加、(b) Cue パイプライン（compile_sheet / TimedSchedule::insert/extend/tick）に有限性検証を追加し DolaDocument と方針を揃える、(c) `resolve_entity_ref` を `try_from_bits` 化して panic→None 縮退（ドキュメント記載の契約に実装を一致）、(d) `__implicit_` プレフィックスを予約名に追加、(e) process_loops の周回キャッチアップに反復上限/剰余スキップを導入（P9・時刻ジャンプ DoS 耐性。loop_offset 乱数消費の決定性を維持する設計判断を含む）。各特性化テストを新仕様のエラー期待へ置き換える。

#### NSP-2: winproc / WinThreadMgr レガシー経路の健全性修正と削除セット
- **統合元**: P27（win_message_handler 削除セット）・P28（get_boxed_ptr のトレイト型混同 + mutable transmute）・P30（CoUninitialize 欠如・create_window 失敗時の Box リーク）・P32（多重生成時の ECS_WORLD 束縛固定・跨スレッド Rc UB）。**P31（WinProcessSingleton 非冪等初期化）** も同経路の堅牢化として含める。
- **種別**: テスト未保護 unsafe のロジック変更を要する健全性修正 + 非推奨コード削除候補（R2.10）。
- **根拠**: `winproc::get_boxed_ptr` は (a) 別トレイトのファットポインタとして読み出す型混同、(b) 共有参照からの可変参照 transmute（UB 領域）を持ち、レガシー `create_window` 利用者（examples/dcomp_demo.rs）では毎メッセージ実行され、同一ウィンドウへの同期送信で `&mut` が 2 つ同時生存し得る。`win_message_handler.rs`（1,400 行・唯一の `#![deprecated]`）は利用 3 件（winproc dispatch・WinThreadMgrInner::create_window・dcomp_demo）の移行を要し削除実証不能だった。P32 は多重 WinThreadMgr 生成で ECS_WORLD 束縛が固定され別インスタンスのメッセージ誤配信・跨スレッド非アトミック Rc upgrade（UB）に至る。
- **推奨スコープ**: 削除セット = { win_message_handler 全体 + winproc のハンドラ dispatch 経路 + WinThreadMgrInner::create_window + dcomp_demo を ECS 経路へ移行 } を一括で扱う新規仕様。P28 は P27 の削除で経路ごと消滅するため P27 を優先。存続させる場合は格納/読出型を統一し可変アクセスを thread_local + RefCell へ。併せて Drop に CoUninitialize 追加・create_window エラー経路の Box 解放・RegisterClassExW の冪等化（ERROR_CLASS_ALREADY_EXISTS 許容 + GetLastError 付きメッセージ）・WinThreadMgr の単一インスタンス契約明示（2個目を Err 拒否）を実施。

#### NSP-3: GPU リソースリーク・デバイスロスト復旧の整備
- **統合元**: P38（ClipGuard geometricMask transmute の COM 参照リーク）・P40（デバイスロスト検出の欠如）・P47（再ペアレント未検出の孤立 Visual）。**P33（録画モジュールの Clone COM リーク・todo! パニック）** も COM リソース健全性として含める。
- **種別**: 挙動変更を伴う脆弱性対策 / 正確性改善。
- **根拠**: P38 は角丸クリップ付き Visual の再合成のたびにジオメトリ COM を 1 個リークし、アニメーションで毎フレーム再合成されると無制限増加。P40 はプロダクションに `GraphicsCore::invalidate()` の発火経路が存在せず、GPU リセット時に ULW/DComp ウィンドウが最終フレームで恒久的に固まる可用性縮退（W3b-V が DComp 側再初期化の不完全性も追加分析）。P47 は再ペアレントで Visual が旧親に残り、旧親再同期時に画面から消失する潜在バグ。P33 の録画モジュール（利用ゼロ 1,016 行）は `commands()` が必ず todo! パニック、Clone が COM 参照を毎回リーク。
- **推奨スコープ**: (a) ClipGuard の transmute を `transmute_copy`（借用コピー）または `ManuallyDrop::into_inner` 回収へ置換、(b) EndDraw/CopyFromBitmap の HRESULT を検査し D2DERR_RECREATE_TARGET / DXGI_ERROR_DEVICE_REMOVED で invalidate() を呼ぶ + DComp 系を invalidate 対象へ再追加するか設計判断、(c) visual_hierarchy_sync_system の検出条件を「parent_visual が現 ChildOf と不一致」へ拡張、(d) 録画モジュールは完成（commands() の Ref 返却・Clone の AddRef/Release 対管理）または削除。

### 優先度 中（P2）

#### NSP-4: ポインター/ドラッグのデッドストレージ整理と機能ギャップ
- **統合元**: P59（WHEEL/DOUBLE_CLICK_BUFFERS 未消費・ホイール未反映）・P60（thread_local キー単調増加）・P61（CaptureGuard::is_released のテスト専用アクセサ、prev_frame_pos は W6b-S で解消済み）。**P57 は W6a-S で解消済み**（process_pointer_buffers 削除）として完了記録。
- **種別**: その他（デッドストレージ削除 + 機能ギャップ）/ 挙動変更を伴う脆弱性対策（P60）。
- **根拠**: `WHEEL_BUFFERS` は書き込まれるが本番で消費されず**マウスホイール入力が PointerState/OnPointer ハンドラに届かない機能ギャップ**（W6a-S 以前からの潜在ギャップで W6a-S は回帰を導入していないと commit 実コードで裏取り済み）。`DOUBLE_CLICK_BUFFERS` は純粋デッドストレージ。thread_local の HashMap キーが distinct Entity ごとに永久蓄積（現状は UI 要素数で実質有界＝安全だが理論的 stale キー残置）。
- **推奨スコープ**: (a) ホイール入力を PointerState へ反映する設計判断を確定し必要なら transfer_buffers_to_world に WHEEL_BUFFERS 消費を追加、(b) 不要なら WHEEL/DOUBLE_CLICK_BUFFERS をデッドコード削除、(c) leave/despawn フック連動で thread_local キーを除去（ButtonBuffer エッジ検出契約との相互作用に注意）、(d) CaptureGuard::is_released の整理。

#### NSP-5: レイアウト/画像の数値オーバーフロー・パストラバーサル
- **統合元**: P49（LengthPercentageAuto/LengthPercentage の÷100 正規化欠落）・P53（ColorMapData::from_image の u32 乗算オーバーフロー）・P55（generate_alpha_mask_system の同型オーバーフロー）・P56（resolve_path のパストラバーサル検証欠如）。**P3（areka SHELL_IMAGE_PATH のビルドマシン絶対パス埋め込み）** も外部アセット解決の同族として含める。
- **種別**: 挙動変更を伴う脆弱性対策。
- **根拠**: P49 は margin/padding/inset に `Percent(50.0)` を指定すると 5000% 解釈でレイアウト破綻（ドキュメントは「÷100 自動」と謳う・現状 Px のみ利用で潜在）。P53/P55 は外部 PNG 寸法の u32 乗算が巨大画像で debug panic（DoS）/release ラップ（過小バッファ）= 同一クラスの 2 箇所。P56 は `resolve_path` が `..` を含む相対パス・絶対パスを無検証で WIC へ渡し情報開示。P3 はビルドマシン絶対パスが配布バイナリへ埋め込まれる情報開示 + 可用性。
- **推奨スコープ**: (a) LengthPercentageAuto/LengthPercentage の Percent を `taffy::percent(v/100.0)` へ正規化、(b) 画像寸法乗算を usize 昇格 / checked_mul + 寸法上限で検証（hit_region 側と alpha_mask 側を統合実施）、(c) resolve_path に基準ディレクトリ jail（`..` 拒否・canonicalize の starts_with・絶対パスポリシー明文化）、(d) areka のアセットパスを実行時解決（current_exe 相対 or include_bytes!）へ移行。P3 と P56 の外部アセット解決方針を揃える。

#### NSP-6: ビルド再現性・公開ポリシー・依存追随
- **統合元**: P71（publish=true 上書きと要件前提の矛盾）・P73（rand パッチ更新 RUSTSEC-2026-0097）・P75（Cargo.lock 追跡方針）。**P72（profile.release 最適化見直し）** も配布方針として含める。
- **種別**: ビルド再現性 / 公開可否（挙動相当の設定）。
- **根拠**: 各クレートが `[workspace.package] publish = false` を `publish = true` で上書きしており、requirements.md「未公開ゆえ後方互換性考慮不要」(R2.9 前提) と矛盾＝本ループの削除判断の土台と食い違う（公開リスクも X1-V が補足）。Cargo.lock が初回コミットから未追跡でバイナリ生成ワークスペースの依存固定・再現性が欠如。rand unsoundness は到達不能だが patched 版（>=0.10.1 / >=0.9.3）への追随が防御的に望ましい（ただし rand@0.9.2 は vendors/pasta が引くため pasta 側の領分）。
- **推奨スコープ**: 配布/公開/サプライチェーン方針の単一の真実源を確定する新規仕様。(a) publish 方針を確定（未公開なら各クレートの `publish = true` を削除しルートを継承、公開予定なら requirements/design を改訂し SemVer 考慮）、(b) Cargo.lock の追跡開始（再現性重視）or 非追跡明文化、(c) rand を patched 版へ更新（乱数列互換性を loop_offset テストで確認）、(d) profile.release の opt-level/lto/codegen-units/strip をベンチで見直し陳腐化コメント整理。design.md Revalidation Triggers（公開ポリシー変更）に該当するため本ループ外で扱う。

#### NSP-7: 検証スコープ・CI 基盤
- **統合元**: P70（S2 が非既定 feature テストを実行しない）・P74（cargo audit の CI 導入）。
- **種別**: その他（検証スコープ是正 + CI 新設・本ループ対象外）。
- **根拠**: 正準 S2（`cargo test --workspace`）は dola の既定 feature のみ実行し、`toml`/`yaml` ゲート付き往復テスト 3 件（1713 vs --all-features 1716、+3 厳密一致）を回帰非保護。CI 自体が不在（`.github/workflows` 等いずれも不在）で feature 全網羅・依存監査・examples ビルドを常時検証する基盤がない。
- **推奨スコープ**: (a) S2 に `cargo test --workspace --all-features`（または dola 限定）の一巡を追加（挙動非破壊・検知範囲拡大のみ）、(b) CI 新設仕様に `--all-features` ジョブ・`cargo audit`（informational/dev-only の扱いを設計判断）・examples ビルドを含める。design.md Revalidation Triggers（S2 変更）に該当。

### 優先度 低（P3）

#### NSP-8: 正確性・診断・ロジック整合の改善
- **統合元**: P2（バルーン位置一元化）・P11（競合検出の wall-clock 非対応）・P12（trigger_store 残置リーク）・P15（resume 非単調時刻）・P16（InvalidGroupId 混同）・P17（compile 診断精度）・P19（純粋KF 暗黙依存）・P22（TimedSchedule 同時刻配信順 FIFO/LIFO）・P34（GetClusterMetrics エラー黙殺）・P44（resolve_inherited_brushes フィールド単位継承）・P48（ChildOf 祖先走査の巡回ガード）・P62（check_threshold 桁あふれ）・P66（FrameCount u32 オーバーフロー）・**P10（DynamicValue の Hash/Eq 契約違反＝Float 0.0/-0.0 のハッシュ正規化）**・**P4（areka 起動経路の無音失敗の可観測化＝不正 RUST_LOG フォールバック・UI 構築コマンド送信失敗時の警告ログ）**。
- **種別**: 正確性改善 / エラー診断改善 / 堅牢化（多くは挙動変更を伴う）。
- **根拠**: いずれも現状実害が限定的（潜在バグ・到達極小・診断品質）で、現行挙動は特性化テストで固定済み。例: P11/P12 は競合解決の早期終了・group_id 単調増加リーク（process 生存中）、P22 は投入 API 選択で同時刻配信順が逆転、P48 は間接巡回 ChildOf（通常 API で生成不能）で UI スレッドハング、P62/P66 は実用座標/828日連続稼働でのみ到達する整数境界。
- **推奨スコープ**: 各々独立の小規模修正として、対応する特性化テストを新仕様の期待へ置き換えつつ実施。多くは挙動変更（エラー応答・配信順・診断内容の変化）を伴うため個別に設計判断を要する。NSP-1 の検証ハードニングと同時実施が効率的なもの（P16）は統合可。

#### NSP-9: コード簡素化・DRY・デッドコード整理（churn 回避で見送った構造変更）
- **統合元**: P5（playback 旧型整理）・P6（到達不能 is_terminal 分岐）・P7（イージング重複 loop_controller↔interpolator）・P13（document_store::get_storyboard）・P18（compile 到達不能防御分岐）・P23（dfs 再帰の反復化）・P24（変数値域 Float/Integer 統合）・P35（D2D1CommandListExt::open スタブ削除）・P36（Ref::unwrap の COM ABI 越え）・P37（赤デバッグ枠・DIB 全画素スキャン）・P39（render_surface 未使用パラメータ）・P41（負サイズ i32→u32 ラップ）・P42（create_dib_section GDI エラー経路）・P43/P63（SetWindowPosCommand テスト観測 API）・P45（visual_resource_management_system 未使用 Commands）・P46（apply_window_pos_changes 重複ログ）・P50（From<taffy::Dimension> スタブ）・P51（BitmapSourceResource テスト用コンストラクタ）・P52（hit_test_entity 重複統合）・P54（convert_to_timeline 純粋分離）・P58（transfer_buffers_to_world match 重複）・P64（window_proc メッセージ抽出ヘルパ）・P65（create_windows ex_style 抽出）・P67（dispatch_cue_sheet_internal 配送アーム統合）・P68（DolaAnimator 配線時の Sync ハザード）。
- **種別**: ロジック変更を要する簡素化 / 非推奨コード削除候補 / テスト容易化 API。
- **根拠**: いずれも (a) セル境界跨ぎ（P7 は D1a↔D1b、P12 は D1a↔D1b、P62 は W6b↔W7a、P52/P64/P65 は GUI システム構造変更）、(b) テスト保護外のロジック変更（R5.5）、(c) 利用実証不能な削除（R2.10）、(d) churn 回避で見送り（既存スタイル維持・壊れていないものを直さない）のいずれかで本ループでは実装しなかった。P43↔P63 は同根（areka 側 P1 / wintf 側 P63 が SetWindowPosCommand のテスト観測 API 欠如を指摘）で統合実装すべき。
- **推奨スコープ**: 個別の小規模リファクタとして、対応する回帰検知器の整備（特に未保護分岐: P67 の Barrier 配送・RouteAdd、P64 の抽出後ユニットテスト）を前提に実施。テスト容易化 API（P43/P63/P51）は `#[cfg(any(test, feature="test-util"))]` で追加し、P68 は DolaAnimator をプロダクション配線する際に必須前提（Arc 化 or SingleThreaded スケジュール固定）として扱う。

### 横断的事実精度の補正（最優先で着手すべき記録是正）

- **command_list.rs:29-33 の SAFETY コメント誤記**: W5a-V が発見し W3b-V が当該ファイルのコメントを是正済み、W7a-V も CONCERNS として再記録。windows-rs 0.62.2 では COM 型の Send/Sync 付与は**型依存**（`ID2D1CommandList`/`IDWriteTextFormat`/`IDWriteTextLayout` は Send+Sync 付与済み＝当該 unsafe impl は冗長、`IWICBitmapSource` 系は非 Send で genuine に必要）であり、「windows-rs の COM スマートポインタは一律 !Send/!Sync」という blanket 主張は普遍的に正しくない。この知見は W2-V / W5a-V / W5b-V / W7a-V / W7b-V / W8-V の SAFETY 注記方針として全域に反映済み。残存する同種の誤記コメント（あれば）の精度是正と、SAFETY 注記の「必須 / 冗長」区別の徹底を、上記提案群と独立に推奨する。

#### P 番号の追跡性確認

P1〜P75 の全 75 エントリがいずれかの統合提案（NSP-1〜9 + 横断補正）に帰属することを確認した。**解消済み**: P57（W6a-S で process_pointer_buffers 削除）・P61 の prev_frame_pos 部分（W6b-S で削除）は本ループ内で実施済みとして完了記録（NSP-4 に注記）。**重複統合の主な対応**: P8↔P14↔P20↔P25↔P26（→NSP-1 検証ファミリ）、P27↔P28（→NSP-2、削除で経路消滅）、P43↔P63（→NSP-9、areka/wintf 同根）、P53↔P55（→NSP-5、同型オーバーフロー2箇所）、P3↔P56（→NSP-5、外部アセット解決）、P70↔P74（→NSP-7、検証/CI）、P71↔P73↔P75（→NSP-6、配布/依存）。**単独帰属の明記**: P4（→NSP-8、起動失敗の可観測化）・P9（→NSP-1、時刻ジャンプ DoS 反復上限）・P10（→NSP-8、Hash/Eq 正規化）・P29（→§4、steering structure.md の非推奨記載の事実誤認として是正記録）。これにより P1〜P75 の全 75 件が NSP-1〜9・§4・横断補正のいずれかに漏れなく帰属する。

---

## 付録: 累計成果サマリ

- **追加テスト**: 963 → 1713（fragment-derived・+750、--all-features 1716）。全領域で in-source / 統合の特性化テストを追加し、テスト空白だった領域（D2 / W2 / W3a/b / W6a/b / W7a/b / W8 / com / dola/cue 各 in-source）をゼロから整備。
- **デッドコード除去（挙動非破壊・利用ゼロ実証）**: W6a-S（process_pointer_buffers 系 +3/−302＝net −299 行）・W3a-S（廃止3関数 net −262 行）・D1b-S（net −144 行）・W6b-S（prev_frame_pos）・D2-S（net −76 行）・W3b-S（net −36 行）他。
- **フレーキー決定論化**: cue_performance_test の実時間ベンチを正当性検証へ置換（W8-T1）。
- **SAFETY 注記の格上げ・是正**: HWND/HINSTANCE（必須）・WIC（必須）・D2D/DWrite（冗長）・DolaAnimator（実構成是正）・command_list.rs（誤記是正）等、windows-rs ソース実証に基づく多数の SAFETY 根拠明文化。
- **巻き戻し 0 件・REJECTED 3 回（全是正）・事実誤認 3 件捕捉是正**。
- **新規仕様提案**: P1〜P75 を 9 つの優先度付き統合提案（NSP-1〜9）+ 横断補正へ整理。
- **最終起動テスト S7**: PASS（初期化 +0.208 秒・panic/error/WARN ゼロ・正常終了）。