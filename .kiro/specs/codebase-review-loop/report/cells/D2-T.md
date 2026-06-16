# D2-T: dola コンパイル・DSL × テスト網羅性

- status: completed
- commit: test(D2): コンパイル・Builder・エラー型のテスト空白に28件のギャップテストを追加

## findings

### モジュール×テスト対応表（改善前 → 改善後）

| モジュール | 対象 | 既存テスト | 追加 | 備考 |
|------------|------|-----------|------|------|
| `compile/mod.rs` (297 LOC) | validate 失敗パススルー / SB未定義 / KF循環 / between delay超過 / セグメント重複 | `tests/compile/error_test.rs` 5件 | — | 既存で十分 |
| 〃 | 複数エラーの収集（1回のコンパイルで2件以上） | なし | 1件 | between 失敗→下流 at 解決不能の連鎖で `errs.len()==2` を固定（`multiple_errors_accumulated_with_unresolvable_downstream_keyframe`） |
| 〃 | メタ情報伝播（time_scale / loop_count / interruption_policy / type hint / min・max / base_duration / total_base_duration） | `tests/compile/metadata_test.rs` 8件 | — | 既存で十分 |
| 〃 | `loop_offset` の伝播（Scalar / Range / 省略時 None） | なし | 3件 | CompiledStoryboard の唯一未検証だったメタフィールド |
| 〃 | セグメントの start_time 昇順ソート（記述順と逆の at 配置） | なし | 1件 | Step 5 のソート処理を直接固定 |
| 〃 | トリガーエントリの時刻解決（at なし→直前エントリ継承 / トリガー keyframe の at アンカー利用） | `tests/trigger/compile_test.rs` 6件（at 起点・先頭フォールバック・オフセット・ソート・total_base_duration 非寄与） | 2件 | `tests/compile/integration_test.rs` へ追加。トリガー KF が 0 秒完了として登録され後続 at から参照可能なことを初めて固定 |
| 〃 | 空SB / serde / Builder→compile フロー / 複合配置 / 全変数型混在 | `tests/compile/integration_test.rs` 7件 | — | 既存で十分 |
| `compile/resolve.rs` (438 LOC) | sequential / at / at+offset / Multiple / between / 即時遷移 / start_time オフセット / 初回 sequential 基準 | `tests/compile/time_resolution_test.rs` 8件 | — | 既存で十分 |
| 〃 | 純粋KF（at なし）: 直前エントリ時刻の継承・先頭エントリの start フォールバック | なし（at 付き純粋KFのみ `integration_test.rs` にあり） | 2件 | `resolve_pure_keyframe_time` の else 分岐2経路 |
| 〃 | `KeyframeRef::WithOffset{Multiple}` / 負オフセット | なし | 2件 | KeyframeRef 全バリアント×オフセット符号を網羅 |
| 〃 | between: 疑似KF "start" 起点 / 区間内に収まる有効 delay | 境界超過（エラー）のみ | 2件 | 正常系 delay と "start" 疑似KFの between 利用を固定 |
| 〃 | at 配置後の同一変数 sequential 継続（var_last_end_time 更新） | なし | 1件 | at セグメント終端からの継続と from 値引き継ぎ |
| 〃 | from 推論（前セグメント / Float initial / Object initial）・relative_to・easing 保持 | `tests/compile/transition_test.rs` 6件 | — | 既存で十分 |
| 〃 | Integer 変数 initial の f64 変換 from 推論 | なし | 1件 | `resolve_from_value` の Integer アーム |
| `compile/types.rs` (104 LOC) | CompiledStoryboard / VariableTypeHint / CompiledSegment の serde | `tests/compile/serde_test.rs` 3件 | — | 既存で十分 |
| 〃 | Optional フィールドの省略（loop_offset / triggers / min・max / easing 同時）・loop_offset＋triggers 込みフルラウンドトリップ（start_offset の Some/None 両方） | easing 単独の skip のみ | 2件 | `skip_serializing_if` ＋ `serde(default)` の往復整合を全フィールドで固定 |
| `builder.rs` (127 LOC) | new / 全要素構築 / バリデーションエラー / SB デフォルト値 / entry 追加 / メタ設定 | `tests/general/builder_test.rs` 6件 | — | 既存で十分（general ドメイン） |
| 〃 | `loop_offset()` 設定・省略時 None / `Default` 実装 / 同名要素の後勝ち上書き / スキーマ不一致での build 失敗 | なし | 5件 | in-source `mod tests`（S9 Inline）。境界内の src 側へ配置し general ドメインのテストファイルには触れない |
| `error.rs` (296 LOC) | `Display` 実装（全20バリアント）・`std::error::Error` 実装 | 直接テストなし（matches! によるバリアント判別のみ） | 6件 | in-source `mod tests`（S9 Inline）。診断メッセージの全文を完全一致で固定し、フォーマット崩れの回帰検知器とした |

追加テスト合計 28 件（統合 17 件: `tests/compile/` 配下5ファイル / in-source 11 件: `builder.rs` 5, `error.rs` 6）。配置は S9 準拠（統合テストは既存ドメインサブディレクトリ `tests/compile/` の対応ファイルへ追記、ユニットテストは Inline 方式 `mod tests`）。`tests/compile.rs` エントリポイントの変更は不要（新規ファイルなし）。

### 除外テスト

0 件。重複候補として精査した `tests/compile/integration_test.rs` ローカルの `make_doc` は `tests/compile/common/mod.rs::make_doc_with_storyboard` と同一実装の重複だが、テストヘルパの統合は T 観点の範囲外（テスト削除ではない構造整理）のため D2-S へ申し送り。`tests/trigger/compile_test.rs` のトリガーコンパイル6件は compile ドメインと対象が重なるが、検証軸（fire_time 計算・ソート・total_base_duration 非寄与）が本セル追加分（KF 登録・継承経路）と相補的であり重複ではない。

### テスト不能箇所・深掘り所見

1. **validate() 前提の到達不能な防御分岐が compile 内に5系統** — `resolve_transition` の Named 未定義／transition 欠落エラー（V5/V7-9 が先に検出）、`var_def` 取得失敗の continue（V4）、Object 型への easing 強制 None（V10 が easing 指定自体を拒否するため常に no-op）、`build_variable_type_hint` の None→Float フォールバック、`resolve_to_value` の非 Scalar from の relative_to スキップ（V10/V13）。公開 API（`compile_storyboard`）経由ではいずれも到達不能であることをバリデーションルールとの突き合わせで確認した。整理は P18 として提案記録。
2. **セグメント重複エラーの entry_index は常に 0（コード内 "approximate" 明記）、循環検出は閉路外の下流エントリも過大包含** — 利用者がエラー位置を特定できない診断精度の問題。エラー内容の変更を伴うため P17 として提案記録（現行挙動は既存テストが reason 文字列・バリアント種別で特性化済み）。
3. **`topological_sort` 内の死コード** — in_degree を2回計算しており（1回目のループは直後に「Recompute in_degree properly」で全面上書き）、`*in_degree.entry(*idx).or_insert(0) += 0;` という no-op 文も残存。挙動非破壊で除去可能なため D2-S へ申し送り。
4. **`find_previous_entry_in_sort_order` は名前に反して sorted_indices 未使用**（`_sorted_indices`）で「元配列の index - 1」を返す。仕様（配列直前エントリ）どおりの挙動だが名前が誤解を招く。リネームは挙動非破壊のため D2-S へ申し送り。
5. **`builder.rs` / `error.rs` 冒頭の `// TODO: Implement ...` は陳腐化**（いずれも実装済み）。コメント除去は D2-S へ申し送り（D1a の同種所見と同パターン）。
6. **RED フェーズ代替の検証** — 追加テストは既存挙動の特性化のため RED は N/A。代わりに代表 4 件（純粋KF継承・複数エラー収集・loop_offset 伝播・Display 全文一致）について、期待値が実装から導出した値と一致することをソース読解（resolve.rs の時刻式・Display フォーマット文字列）で照合してから記述した。

### 検証（S2）

- BEFORE: HEAD 2b02459 で `cargo build --workspace` 成功 / `cargo test --workspace` 全グリーン（1087 passed / 0 failed / 32 ignored）
- AFTER: `cargo build --workspace` 成功 / `cargo test --workspace` 1115 passed / 0 failed（+28 はすべて追加分。既存スイートはベースラインと同一結果）
- フレーキー: AFTER 初回実行で wintf の1スイートが 78 passed / 1 failed（境界外・既知の `cue_performance_test::bench_pop_ready_empty_queue` 系）。プロトコルに従い再実行で安定合格を確認（フレーキー・パススルー、詳細は下記 flaky 節）

## flaky

- `wintf` tests/ecs スイート（既知: `cue_performance_test::bench_pop_ready_empty_queue`）が AFTER 初回の `cargo test --workspace` で 1 件 fail（78 passed / 1 failed）。本セルの変更は dola の compile/builder/error のみで wintf には不干渉。`--no-fail-fast` での再実行および最終クリーン実行で全グリーン（安定合格）を確認し、既知フレーキーのパススルーと判定。

## proposals

- P17（report/proposals.md へ追記済み）: compile エラー診断の精度改善（overlap の entry_index 固定0・循環報告の過大包含）
- P18（report/proposals.md へ追記済み）: compile 内の到達不能な防御分岐の整理（validate() 前提の二重防御）
