# D1a-V: dola ランタイム中核 × 脆弱性

- status: completed
- commit: fix(D1a): 脆弱性点検に基づく不変条件の debug_assert・安全コメント・特性化テストを追加

## findings

### 点検対象

`crates/dola/src/runtime/{facade,loop_controller}.rs`、`crates/dola/src/runtime/{timeline_manager,subscription_manager}/`、`crates/dola/src/playback.rs`。点検観点: unwrap/expect 多数域の panic 経路（DoS）・整数変換の切り捨て・オーバーフロー・時刻計算の境界条件（inf/NaN・巨大時刻ジャンプ）。

### 1. panic 経路（unwrap / expect / 添字アクセス）

| # | 経路 | 解析結果 | 対応 |
|---|------|---------|------|
| 1 | `timeline_manager/mod.rs:271` `segments.last().unwrap()` | 直前の `segments.is_empty()` 早期 return により非空が保証され発火不能 | **SAFETY コメント投入** |
| 2 | `facade.rs::process_triggers` Step 1 `&triggers[ts.trigger_index]` | `trigger_states` は `create_instance` で `0..compiled.triggers.len()` の添字から構築（`instance_manager/mod.rs:88-90`）、`trigger_store` には同一の `compiled.triggers` が同じ group_id で格納（`start_internal` ステップ 4 / 6.5）。両者は conclude/cancel で同時削除・group_id 非再利用のため `ts.trigger_index < triggers.len()` が常に成立 | **debug_assert + SAFETY コメント投入**（release ではコンパイル除去、挙動不変） |
| 3 | `facade.rs::process_triggers` Step 2 `inst.trigger_states[p.trigger_index]` | `p.trigger_index` は Step 1 で同一インスタンスを enumerate した添字で、Step 1〜2 間に `trigger_states` への変更なし → 範囲外不能 | **不変条件コメント投入** |
| 4 | facade / loop_controller / subscription_manager 製品コードの unwrap/expect | grep 実証で上記以外に 0 件（テストコード内のみ） | 対応不要 |

結論: 外部入力から到達可能な panic 経路（DoS ベクタ）は存在しない。

### 2. 整数変換の切り捨て・オーバーフロー

| # | 箇所 | 解析結果 | 対応 |
|---|------|---------|------|
| 1 | `loop_controller.rs::should_continue_loop` の `loop_count as u64`（i64 → u64） | 負値が wrap すると約 1.8e19（実質無限ループ化）の危険があるが、`facade::compile_and_validate` が「-1 または正値」のみ許可（`loop_count <= 0 && != -1` は `InvalidLoopCount`）し、-1 は直前の早期 return で処理済み → 常に正値の無損失変換 | **debug_assert（`loop_count >= 1`）+ 安全性コメント投入** |
| 2 | `subscription_manager` の `next_id: i64` 単調増加カウンタ | `+= 1` のみで縮小変換なし。オーバーフローには 9.2e18 回の subscribe が必要で実質到達不能 | 対応不要 |
| 3 | その他の `as` 縮小変換 | 境界内製品コードに 0 件（テスト内の `n as f64` のみ） | 対応不要 |

### 3. 時刻計算の境界条件

| # | 箇所 | 解析結果 | 対応 |
|---|------|---------|------|
| 1 | `facade.rs::compile_and_validate` の `loop_duration = total_base_duration / time_scale` | `time_scale` はスキーマ（serde default 1.0）・バリデーション・コンパイルのいずれでも正値検証されない。`time_scale == 0.0` で loop_duration が +inf（duration > 0）または NaN（0/0）となり、`== 0.0` / `< MIN_LOOP_DURATION` 比較を素通りして start が成功。end_time が inf/NaN のインスタンスは自然終了・ループ・トリガー発火が一切起きずリソースリーク（インスタンス・タイムテーブル・トリガーストア解放漏れ）。負値は end_time が過去になり初回 update で即 Conclude | 入力検証の追加は外部観測可能な挙動変更のため **P8 提案記録**（R2.4/R5.2）。現行挙動は **特性化テスト 5 件**（`facade_test.rs::time_scale_boundary`）で固定し、**NOTE コメント投入** |
| 2 | `facade.rs::process_triggers` の `wall_fire_time` 除算 | 同じ `time_scale` 除算で inf/NaN 化しトリガー不発火（上記 #1 と同根） | **NOTE コメント投入**（P8 参照） |
| 3 | `loop_controller.rs::process_loops` の while キャッチアップ | 反復回数は時刻ジャンプ幅 ÷ 周回長に比例。無限ループの周回長下限は MIN_LOOP_DURATION = 0.1s のみで、壁時計の大幅補正（例: 1e9 秒ジャンプ）で最大 1e10 回規模の反復 → update() が長時間ブロックする quasi-hang（DoS）になり得る。停止性自体は保証される | 反復上限キャップ・剰余スキップは loops_completed / loop_start_time / 乱数消費という外部観測可能な挙動を変えるため **P9 提案記録**（R2.4/R5.2）。**ハザードコメント投入** |

### 4. subscription_manager / playback のギャップ点検

- `playback.rs`（25 行）: serde derive 付きのデータ型（`PlaybackState` enum / `ScheduleRequest` struct）のみでロジックなし。panic 経路・整数変換・時刻計算のいずれも非該当 → 所見なし。
- `subscription_manager/mod.rs`（190 行）: unwrap/expect/添字アクセス 0 件。全マップアクセスが `get`/`remove` の Option/bool ハンドリングで panic 不能。`diff_and_update` の凍結値フォールバックも `or_else` + `continue` で安全。整数は上記 2-#2 の単調カウンタのみ。時刻計算なし → 所見なし（投入対策なし）。

### 投入した挙動非破壊対策（R2.3/R5.1）

1. **`facade.rs`**: time_scale inf/NaN ハザードの NOTE コメント（compile_and_validate / process_triggers）、trigger_states/trigger_store 添字不変条件の debug_assert + SAFETY コメント、Step 2 添字安全性コメント（+25 行）。
2. **`loop_controller.rs`**: `loop_count as u64` 変換安全性の debug_assert + コメント、process_loops キャッチアップのハザードコメント（+16 行）。
3. **`timeline_manager/mod.rs`**: `segments.last().unwrap()` の SAFETY コメント（+2 行）。
4. **特性化テスト追加（additive）**: `tests/runtime/facade_test.rs::time_scale_boundary` 5 件 — time_scale=0 での start 成功・+inf end_time・自然終了しない生存・calculate_end_time の +inf・0/0 → NaN end_time・負値での即 Conclude を現行挙動として固定（+137 行）。

debug_assert は上記解析の通りいずれも正規の現行挙動下で発火不能（release ではコンパイル除去）。既存行の変更・削除は 0（全ファイル insertions のみ、計 193 行）。

### 検証（S2）

- HEAD ベースライン: 1032 passed / 0 failed
- AFTER: `cargo build --workspace` 成功 / `cargo test --workspace` 全グリーン（1037 passed / 0 failed）。差分は time_scale_boundary 追加テスト 5 件のみで既存テストの失敗・変更 0

## flaky

なし（wintf cue_performance_test は全実行で安定パス。隔離再実行は不要だった）

## proposals

- P8（report/proposals.md へ追記）: dola ストーリーボード time_scale の入力検証追加（0・負値・非有限値の拒否）— kind: 挙動変更を伴う脆弱性対策
- P9（report/proposals.md へ追記）: dola process_loops の周回キャッチアップ反復上限（時刻ジャンプ DoS 耐性）— kind: 挙動変更を伴う脆弱性対策
