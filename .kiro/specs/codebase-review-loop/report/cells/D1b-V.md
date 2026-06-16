# D1b-V: dola 補間・状態 × 脆弱性

- status: completed
- commit: fix(D1b): 補間・状態の数値境界と不変条件に NOTE/SAFETY コメント・debug_assert・特性化テスト14件を追加

## findings

### 点検対象

`crates/dola/src/runtime/{conflict_resolver,document_store,types,clock,instance_state}.rs`、`crates/dola/src/runtime/{interpolator,instance_manager}/`、`crates/dola/src/{storyboard,transition,easing,value,variable}.rs`。点検観点: unwrap/expect 多数域の panic 経路・補間計算の数値境界（NaN・無限大・ゼロ除算）・状態遷移の不変条件。

### 1. panic 経路（unwrap / expect / unreachable! / 添字アクセス）

| # | 経路 | 解析結果 | 対応 |
|---|------|---------|------|
| 1 | `conflict_resolver.rs` `unreachable!("Never policy...")` | 手前の Never チェックループが同一の `instance_manager.get` で conflicting 全件を走査し早期 return。チェックと match の間に instance_manager への変更なし（`remove` はエラー経路のみ）→ 発火不能 | **SAFETY コメント投入** |
| 2 | `conflict_resolver.rs` `from_policy(policy).expect(...)` | `from_policy` の Some は Never 以外の 4 ポリシーで、Never は直前 match で除外済み。さらに不変条件「Some は全て is_terminal」を `from_policy` doc に明文化し、`terminate_instance` に **debug_assert（terminal_state.is_terminal()）** を投入（正規経路で発火不能、release ではコンパイル除去） | **doc 明文化 + debug_assert + 不変条件テスト**（`core_types_test.rs::from_policy_results_are_terminal`） |
| 3 | `instance_manager/mod.rs:116` `insert` 直後の `get(&group_id).unwrap()` | 直前の insert により必ず Some → 発火不能。group_id は facade の単調増加カウンタで採番され衝突しない（衝突時の上書き挙動は D1b-T の `create_instance_with_same_group_id_overwrites` で特性化済みのため debug_assert は投入不可と判断） | **SAFETY/NOTE コメント投入** |
| 4 | `interpolator` の easing 計算（interpolation crate 0.3.0 ソース精査） | 全 30 named easing は多項式・sqrt・sin・powf・定数除算のみで入力依存の除算なし。`quad_bez`/`cub_bez` は lerp（乗算・加減算）の合成で除算なし。クレート内部 clamp は NaN を素通り（比較 false）させるが panic しない | **NOTE コメント投入 + 全 31 名前 × NaN/±inf の panic-free テスト** |
| 5 | その他境界ファイル（document_store / types / clock / storyboard / transition / easing / value / variable） | 製品コードに unwrap/expect/添字アクセス 0 件（grep 実証、テストコード内のみ） | 対応不要 |

結論: 外部入力から到達可能な panic 経路は存在しない。

### 2. 補間計算の数値境界（NaN・無限大・ゼロ除算・飽和）

| # | 箇所 | 解析結果 | 対応 |
|---|------|---------|------|
| 1 | `interpolator/mod.rs` `progress_t.clamp(0.0, 1.0)` | min<=max が定数のため panic 不能。NaN はクランプされず素通り → Object は `t >= 1.0` が false で from_value、Float は NaN が結果へ伝播、Integer は 0 へ飽和。±inf は 1.0/0.0 にクランプ | **NOTE コメント + 特性化テスト 5 件**（NaN×3型・±inf・NaN 端点） |
| 2 | Integer 補間 `result.round() as i64` | `as i64` は飽和キャスト（Rust 1.45+）で panic 不能: NaN→0、1e300/+inf→i64::MAX、-1e300/-inf→i64::MIN。`round()` も全 f64 で panic しない | **NOTE コメント + 飽和テスト**（`integer_interpolation_saturates_at_i64_bounds`） |
| 3 | ベジェ制御点（`ParametricEasing` x0..x3）は指示書由来の任意 f64 | 除算なしで panic 不能だが NaN は結果へ伝播。また出力は [0,1] にクランプされず制御点次第で from/to を超える外挿（オーバーシュート、Back/Elastic と同様の許容挙動） | **NOTE コメント + 特性化テスト 2 件**（NaN 制御点伝播・オーバーシュート外挿） |
| 4 | `scalar_value` の非数値 Dynamic → 0.0 フォールバック | V13 バリデーションにより正規経路で到達しない防御パス（D1b-T で特性化済み） | **NOTE コメント投入** |
| 5 | 指示書数値フィールド全般の有限性検証の欠如 | バリデーションの数値検査は loop_offset 負値（V14/V15）のみ。TOML の `nan`/`inf` リテラル経由で NaN が流入し得る。panic はしないが静かな縮退 3 系統: (a) `EvaluatedValue::PartialEq` の NaN 自己不等で変更通知が毎フレームスパム化、(b) `detect_overlaps` が NaN 時刻セグメントを競合検出しない、(c) `check_finish_deadlines` が NaN deadline を発火させない | 検証追加は挙動変更のため **P14 提案記録**（R2.4/R5.2）。types.rs / conflict_resolver.rs / instance_manager の各所に **NOTE コメント** + **特性化テスト 3 件**（`float_nan_is_never_equal_to_itself`・`nan_finish_deadline_never_fires`・`infinite_finish_deadline_fires_only_at_infinity`） |
| 6 | `clock.rs::now()` の `counter / frequency` 除算 | QPC/QPF は Windows XP 以降失敗せず frequency は 0 にならない（Microsoft docs）。仮に 0 でも f64 除算は panic せず inf/NaN を返すのみ | **SAFETY コメント投入**（Result 破棄の根拠を含む） |

### 3. 状態遷移の不変条件

| # | 箇所 | 解析結果 | 対応 |
|---|------|---------|------|
| 1 | `try_transition`/`from_policy`/`is_terminal` の整合性 | 遷移グラフは Created→Playing⇄Paused→終了4種で閉じており、終了状態からの脱出・Created からの直接終了は全拒否（D1b-T/既存テストで全遷移網羅済み）。`from_policy` の Some が全て terminal である不変条件（conflict_resolver の expect の前提）は未明文化だった | **doc 明文化 + 不変条件テスト追加** |
| 2 | 「instances マップに終了状態は存在しない」不変条件 | `transition` は終了遷移と同時に自動削除し、製品コードで `state` を直接書くのは try_transition 経由の 2 箇所のみ（grep 実証、instances_mut の利用は facade の Playing 判定のみ）。`set_finish_deadline`/`check_finish_deadlines` の is_terminal チェックはこの不変条件下で到達不能の防御（P6 と同型） | **transition doc に不変条件を明文化 + 防御チェックに NOTE コメント** |
| 3 | `resume` の `pause_duration = current_time - pause_start` | 非単調な時刻入力（current_time < pause_start）で pause_duration が負となり pause_accumulated / end_time が過去方向へ補正 → 意図しない早期終了。API は任意 f64 を受け付けるため debug_assert は正規入力で発火し得ず投入不可 | 検証/クランプは挙動変更のため **P15 提案記録**。**NOTE コメント + 特性化テスト**（`resume_with_time_before_pause_start_shrinks_end_time`: pause_accumulated=-2.0, end_time=0.0 を固定） |
| 4 | `transition`/`resume` の不正遷移エラーが `InvalidGroupId` に混同 | 実在する group_id でも「ID 不在」と同一エラーで報告され原因の区別不能。`try_transition` の `Err(current_state)` 情報は破棄されている（D1b-T で特性化済み） | 専用バリアント追加は公開エラー型の挙動変更のため **P16 提案記録**。**NOTE コメント投入** |

### 投入した挙動非破壊対策（R2.3/R5.1）

1. **`interpolator/mod.rs`**: clamp の NaN 素通り・飽和キャスト・easing の panic-free 性・ベジェ外挿・防御的フォールバックの NOTE コメント 5 箇所（+32 行、既存行の変更なし）。
2. **`conflict_resolver.rs`**: unreachable! の SAFETY コメント、terminate_instance の debug_assert（不変条件表明）、detect_overlaps の NaN 素通り NOTE（+17 行）。
3. **`instance_manager/mod.rs`**: insert 直後 unwrap の SAFETY、group_id 衝突時の上書き NOTE、transition の不変条件 doc + エラー混同 NOTE、resume の非単調時刻 NOTE、set_finish_deadline/check_finish_deadlines の防御チェック NOTE（+30 行）。
4. **`instance_state.rs`**: from_policy の terminal 不変条件 doc（+4 行）。
5. **`clock.rs`**: QPC/QPF の Result 破棄と除算安全性の SAFETY コメント（+5 行）。
6. **`types.rs`**: EvaluatedValue::PartialEq の NaN 自己不等ハザード NOTE（+5 行）。
7. **特性化テスト 14 件追加（additive）**: interpolator 9 件（NaN progress×3型・±inf クランプ・NaN 端点・i64 飽和・全 31 easing×非有限 t の panic-free・NaN 制御点・ベジェオーバーシュート）、instance_manager 3 件（非単調 resume・NaN deadline 不発火・inf deadline）、core_types 2 件（from_policy terminal 不変条件・Float(NaN) 自己不等）。

debug_assert は解析の通り正規の現行挙動下で発火不能（release ではコンパイル除去）。既存テストの変更・削除 0、既存製品コード行の変更・削除 0（全ファイル insertions のみ）。

### 検証（S2）

- BEFORE（HEAD 8bd4718）: `cargo build --workspace` 成功 / `cargo test --workspace` 1073 passed / 0 failed
- AFTER: `cargo build --workspace` 成功 / `cargo test --workspace` 1087 passed / 0 failed。差分は本セル追加の 14 件のみで既存テストの失敗・変更 0

## flaky

なし（wintf cue_performance_test::bench_pop_ready_empty_queue は BEFORE/AFTER とも安定パス。隔離再実行は不要だった）

## proposals

- **P14**: 指示書数値フィールドの有限性検証の欠如（NaN/inf の素通り。変更通知スパム・競合不検出・deadline 不発火の静かな縮退 3 系統。P8 と同根で統合実装推奨）
- **P15**: resume() の非単調時刻入力による pause_accumulated / end_time の過去方向補正（時刻単調性の未検証）
- **P16**: instance_manager::transition の不正状態遷移エラーの InvalidGroupId への混同（専用バリアント追加）
