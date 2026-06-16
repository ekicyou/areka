# D2-V: dola コンパイル・DSL × 脆弱性

- status: completed
- commit: fix(D2): デシリアライズ境界・数値境界・名前衝突の SAFETY/NOTE 注記・debug_assert・特性化テスト11件を追加

## findings

### 点検対象

`crates/dola/src/compile/`（mod.rs / resolve.rs / types.rs）、`crates/dola/src/{builder,error}.rs`。点検観点: 外部入力（JSON/TOML/YAML 文書）のデシリアライズ境界・検証欠如・panic 経路・再帰深度・Builder API 経由のコンパイラクラッシュ可能性。

### 1. デシリアライズ境界（compile が deserialize 済みデータに置く前提）

| # | 項目 | 解析結果 | 対応 |
|---|------|---------|------|
| 1 | 取り込み経路の検証ゲート | `compile_storyboard` は冒頭で必ず `doc.validate()` を実行するため、JSON/TOML/YAML・Builder のどの経路でも検証を迂回できない（単一ゲート）。プロダクションコード内に外部文書の deserialize 呼び出しはなく（grep 実証、`from_str` はテストのみ）、フォーマット機能（toml/yaml）は feature ゲートで既定ビルド外 | **NOTE コメント（mod.rs）+ 特性化テスト**（`storyboard_builder_cannot_bypass_compile_validation`） |
| 2 | 数値範囲の前提 | NaN/inf は標準 JSON では注入不能（serde_json がパース時拒否 — `json_rejects_nan_literal_at_deserialization_boundary` で固定）だが TOML（`nan`/`inf`）・YAML（`.nan`/`.inf`）経由で流入可能（P14 既存）。compile 内の帰結: (a) NaN 時刻はソート（`partial_cmp` → Equal）・重複検査（比較常に false）・between 反転検査を素通りし NaN セグメントが出力へ伝播、(b) NaN の base_duration は `fold(0.0, f64::max)` の NaN 無視特性で total_base_duration から黙って脱落、(c) ±inf は inf 時刻として伝播。いずれも panic なし | **NOTE コメント 3 箇所 + 特性化テスト 5 件**（NaN duration/delay・inf delay・between NaN・trigger_start_offset NaN） |
| 3 | **負値の検証欠如（新規所見）** | `delay`/`duration` は符号検証がなく、負 duration で `segment_end < segment_start` の反転セグメントが生成され、事後条件（時刻順・重複なし、`CompiledSegment` の end >= start 含意）に違反したままランタイムへ流出する。単独配置では重複検査に掛からず base_duration も負値となる | 検証追加は挙動変更のため **P20 提案記録**。**NOTE コメント + 特性化テスト 2 件** |
| 4 | 文字列長・マップサイズ | compile の処理量・割り当てはエントリ数に線形比例（`entry_count` 由来の `Vec` 確保は実体化済み入力サイズが上限で、フィールド値からの `with_capacity` 増幅なし — grep 実証）。capacity DoS の増幅経路なし | 対応不要（テスト #11 で線形性を間接実証） |

### 2. panic 経路（添字アクセス / unwrap / 算術）

| # | 経路 | 解析結果 | 対応 |
|---|------|---------|------|
| 1 | `mod.rs` `&sb.entry[entry_idx]` | `sorted_indices` は `topological_sort` 成功時 `0..entry_count` の置換 → 範囲内（発火不能） | **SAFETY コメント + debug_assert 2 件**（長さ・全要素範囲） |
| 2 | `mod.rs` `segments.last()/first().unwrap()` | `is_empty()` ガード直後 → 発火不能 | **SAFETY コメント** |
| 3 | `resolve.rs` `reverse_deps[dep]` 添字 | `graph.deps` のキー・値は enumerate 由来の index（< entry_count）。呼び出し契約（同一 storyboard から構築した graph と entry_count）の下で範囲内 | **SAFETY コメント + debug_assert** |
| 4 | `resolve.rs` `in_degree[dependent] -= 1` | 入次数は HashSet（重複エッジなし）の deps 数に初期化され、エッジごとに正確に 1 回減算 → usize アンダーフロー不能 | **SAFETY コメント + debug_assert（> 0）** |
| 5 | `mod.rs`/`resolve.rs` のソート・時刻計算 | `partial_cmp().unwrap_or(Equal)` は NaN で panic せず、f64 加減算は全入力で panic しない。`*initial as f64`（i64→f64）も panic 不能 | 対応不要（NaN 縮退は所見 1-2 で NOTE 済み） |
| 6 | `error.rs` Display / `types.rs` | 純粋フォーマット・データ定義のみで panic 経路なし（unwrap/expect/添字 0 件） | 対応不要 |

結論: 境界内に外部入力から到達可能な panic 経路は存在しない（D2-S の unwrap 排除後の再点検でも新規検出なし）。

### 3. 再帰深度（スタック枯渇）

- **compile/ は再帰を持たない**: トポロジカルソートは Kahn 法の反復実装（D2-S で BinaryHeap 化済み）、時刻解決・値解決の全関数も非再帰。1000 エントリの依存連鎖コンパイルで実証（`long_dependency_chain_compiles_without_stack_exhaustion`）。
- **ネスト構造のデシリアライズ深度は上流（フォーマットパーサ）の責務**: `DynamicValue`/`KeyframeRef` は再帰型だが、深度制限は serde_json（既定 128）等パーサ側に委譲される（document.rs = D3 境界）。compile 内の `DynamicValue` 再帰操作（clone/eq）はその深度上限を継承するため安全。mod.rs に NOTE で明文化。
- **申し送り（D3-V 境界）**: `compile_storyboard` → `doc.validate()` 経由で到達する `validate/rules.rs::dfs_detect_cycle`（トリガー循環検出）は再帰 DFS であり、ストーリーボード連鎖長に比例したスタックを消費する。巨大な連鎖（数万 SB 規模）を持つ細工文書でスタック枯渇（abort）の可能性がある。validate/ は D3 境界のため本セルでは反復化を実施せず、mod.rs の NOTE に明記して **D3-V へ申し送り**（反復 DFS 化は挙動非破壊で実施可能）。

### 4. Builder API（コンパイラをクラッシュさせる文書を構築できるか）

| # | 項目 | 解析結果 | 対応 |
|---|------|---------|------|
| 1 | 検証迂回 | `DolaDocumentBuilder::build()` は validate() を内包。`StoryboardBuilder::build()` は無検証だが、compile_storyboard 自身が再検証するため、不正文書は panic ではなくエラーで拒否される（迂回不能） | **NOTE コメント（builder.rs）+ 特性化テスト** |
| 2 | NaN/inf/負値の f64 直接注入 | Rust コードからは serde を介さず任意 f64 を注入可能だが、帰結は所見 1-2/1-3 と同一（panic なし、P14/P20 の縮退） | 所見 1 のテストが Builder 経路で実証（テストは全件 Builder API 経由で構築） |
| 3 | **`__implicit_` 名前衝突（新規所見）** | 明示 `keyframe = "__implicit_{n}"` は V2（明示名同士の重複）・V3（`start` のみ予約）を通過し、keyframe 省略エントリの暗黙名と衝突する。`kf_to_entry`/`keyframe_times` の HashMap 後勝ち上書きで明示キーフレームの時刻が黙ってシャドウされ、`at` 参照が誤った時刻へ解決される（panic なしの整合性侵害 — 誤ったタイムラインが正常出力として返る）。外部指示書・Builder の両経路で再現 | プレフィックス予約は挙動変更のため **P21 提案記録**。**NOTE コメント（resolve.rs）+ 特性化テスト**（明示名 1.0 が暗黙名 5.0 にシャドウされる挙動を固定） |

### 投入した挙動非破壊対策（R2.3/R5.1）

1. **`compile/mod.rs`**: デシリアライズ境界の整理 NOTE（検証単一ゲート・深度責務の委譲・D3 申し送り）、sorted_indices 不変条件の SAFETY + debug_assert 2 件、NaN ソート/重複検査素通り NOTE、反転セグメント NOTE、last/first unwrap の SAFETY、f64::max の NaN 脱落 NOTE（+24 行）。
2. **`compile/resolve.rs`**: `entry_keyframe_name` の衝突ハザード NOTE、`reverse_deps` 添字の SAFETY + debug_assert、`in_degree` 減算の SAFETY + debug_assert、Kahn 法の非再帰性 NOTE、`resolve_entry_timing` の未検証数値 NOTE（+29 行）。
3. **`builder.rs`**: StoryboardBuilder の無検証と compile 側ゲートによる迂回不能性の NOTE（+5 行）。
4. **特性化テスト 11 件追加**（`tests/compile/boundary_test.rs` 新設 + `tests/compile.rs` へのモジュール登録 2 行）: 負 duration 反転セグメント・負 delay・NaN duration 伝播・NaN 重複検査素通り・inf delay・between NaN 反転検査素通り・trigger_start_offset NaN 伝播・JSON の NaN 拒否（境界特性）・`__implicit_` 衝突シャドウ・1000 エントリ連鎖（非再帰実証）・Builder 検証迂回不能、の各現行挙動を固定。

debug_assert は解析の通り正規挙動下で発火不能（テストスイート全グリーンで実証、release ではコンパイル除去）。既存テストの変更・削除 0、既存製品コード行の変更・削除 0（diff は全ファイル insertions のみ、ソース計 60 行 + テスト新規 1 ファイル）。`cargo clippy -p dola` で本セルが新規に持ち込む警告 0（適用中に検出した doc_lazy_continuation 1 件は空行挿入で解消済み。残存警告はすべて変更前から存在し本セル変更ファイル外）。

### 検証（S2）

- BEFORE（HEAD d021958）: `cargo build --workspace` 成功 / `cargo test --workspace` 全グリーン（18 スイート、1115 passed / 0 failed、exit 0）
- AFTER: `cargo build --workspace` 成功 / `cargo test --workspace --no-fail-fast` 全グリーン（18 スイート、1126 passed / 0 failed / 32 ignored、exit 0）。差分は本セル追加の 11 件のみで既存テストの失敗・変更 0。最終のコメント微修正後も build + `cargo test -p dola`（530 passed / 0 failed）で再確認

## flaky

- AFTER 初回の `cargo test --workspace` で wintf tests/ecs スイートが 78 passed / 1 failed（既知の `cue_performance_test::bench_pop_ready_empty_queue`、境界外）。プロトコルに従い隔離実行（ok 1/0）＋スイート全体再実行（79/0）＋ワークスペース全体再実行（1126/0、exit 0）で安定合格を確認し、既知フレーキーのパススルーと判定。

## proposals

- **P20**（report/proposals.md へ追記）: delay/duration の負値検証の欠如 — 反転セグメント（end < start）が事後条件違反のままランタイムへ流出（P14 の有限性検証との統合実装を推奨）
- **P21**（report/proposals.md へ追記）: 暗黙キーフレーム名プレフィックス `__implicit_` の予約欠如 — 明示名との HashMap 後勝ち衝突で明示キーフレームが黙ってシャドウされる整合性侵害（V3 拡張で対処、P17 と統合可）
- 申し送り（提案ではなく D3-V セルで実施可能）: `validate/rules.rs::dfs_detect_cycle` の再帰 DFS によるスタック枯渇可能性（反復化は挙動非破壊、D3 境界）
