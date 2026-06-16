# D3-V: dola 検証・Cue × 脆弱性

- status: completed
- commit: fix(D3): 検証網羅性・Cue 時刻境界・panic 経路の点検に基づく NOTE/SAFETY 注記と特性化テスト16件を追加

## findings

### 点検対象

`crates/dola/src/validate/`（mod.rs / rules.rs）、`crates/dola/src/cue/`（command.rs / schedule.rs / sheet.rs / mod.rs）、`crates/dola/src/{document,lib}.rs`。点検観点: バリデーション網羅性の欠落（不正文書の素通り）・スケジュール時刻の数値境界・panic 経路。既知提案 P14/P19/P20/P21（DolaDocument 数値検証）および P22/P23/P24（D3-T/S 由来）との重複を排除し、新規ギャップのみ提案化した。

### 1. バリデーション網羅性（不正文書の素通り）

| # | 項目 | 解析結果 | 対応 |
|---|------|---------|------|
| 1 | **loop_count の文書レベル検証欠如（新規所見）** | スキーマ仕様（storyboard.rs doc:「0以下 = エラー、-1 = 無限」）に反する loop_count（0・-2 等）は validate()・compile のいずれも検査せず素通りし、facade の start/calculate_end_time 時の `InvalidLoopCount` で初めて拒否される（後置検出）。ランタイム検査が存在するため実害（panic・黙った縮退）はないが、load_document 時点で不正文書と判明しない網羅性ギャップ | 検証追加は挙動変更のため **P26 提案記録**。特性化テスト 1 件（`tests/validation/schema_test.rs::invalid_loop_count_passes_document_validation` — 0/-2/-1/3 の 4 値）|
| 2 | NaN loop_offset の V14-V16 素通り | `Scalar(NaN)`・`Range{min:5,max:NaN}`・`Range{min:NaN,max:3}` はいずれも比較（`< 0.0` / `>`）が false 化して合格し、範囲の逆転・退化が未検出のままランタイムのループ遅延サンプリングへ流入する。P14（指示書全数値フィールドの有限性検証）の suggestion が包含する具体例のため新規提案は不要 | **NOTE コメント（rules.rs validate_loop_offset）+ 特性化テスト 3 件**（`tests/runtime/loop_offset_test.rs::nan_boundary_tests`）|
| 3 | NaN variable initial/min/max・transition from/to の V12 素通り | initial=NaN（値域 [0,1] でも合格）、min/max=NaN（退化値域の未検出）、from/to=Scalar(NaN)（値域比較バイパス）の 3 経路を確認。同じく P14 包含 | **NOTE コメント（rules.rs validate_variable_ranges）+ 特性化テスト 3 件**（`tests/validation/transition_test.rs::v12_nan_tests`）|
| 4 | 重複ストーリーボード名・空 entry・トリガー自己参照 | 重複 SB 名は BTreeMap キーのため文書型レベルで表現不能（JSON 重複キーの後勝ちはパーサ責務であり dola 内に deserialize 経路なし — D2-V 所見 1-1 と整合）。空 entry の SB は合法（facade 側で `empty_entry_doc` テスト群が挙動固定済み）。自己参照は V14t、対象不在は V18t が既存検出 | 新規ギャップなし（対応不要）|
| 5 | trigger_start_offset の符号検証欠如 | 負値は子 SB の start_time を過去方向へずらすが、一律拒否は意図的用法（先行開始）の設計判断を要する。P20（delay/duration の負値棚卸し）の suggestion が対応する同型所見 | 既存 P20 へ委譲（新規提案なし）|

### 2. スケジュール時刻の数値境界（cue/）

| # | 項目 | 解析結果 | 対応 |
|---|------|---------|------|
| 1 | **Cue パイプラインに検証層が存在しない（新規所見）** | CueSheet → compile_sheet → TimedSchedule の経路には DolaDocument の validate() に相当する検証がなく、NaN/inf の start_time・オフセットが無検証で流入する | 検証導入は挙動変更のため **P25 提案記録** |
| 2 | `TimedSchedule::insert/extend` の NaN オフセット | `NaN >= 0.0` が false のため debug ビルドでは非負 debug_assert が発火（負値前提のメッセージで報告）。release では素通りし、`partition_point` の前提（述語の単調分割）を破って以後の挿入位置が不定化 → 配信順が黙って崩れ得る（panic なしの整合性侵害） | **NOTE コメント 2 箇所 + debug 発火の特性化テスト 2 件**（`#[cfg(debug_assertions)]` + `#[should_panic]`）|
| 3 | `tick(NaN)`（current_time または start_time が NaN） | offset=NaN により負値ガード・冪等性ガード・`entry_offset > offset` 比較がすべて false 化 → 最初のバリアまでの全ペイロードが即時配信される（演出時系列の崩壊）。バリアでは停止し、解除後の正常時刻 tick で復帰する（current_offset=NaN からの回復を確認） | **NOTE コメント + 特性化テスト 2 件**（全件即時配信・バリア停止と復帰）|
| 4 | `compile_sheet` の非有限 start_time 正規化 | (a) NaN は `f64::min` の NaN 無視特性で最小値計算から黙って脱落し当該 Cue のみ NaN オフセット化、(b) 全 Cue が +inf → min=+inf → inf−inf=NaN オフセット、(c) 一部 +inf → inf オフセットとなり TimedSchedule 上で永遠に配信されず `is_completed()` が false のまま（liveness 喪失、f64::MAX まで進めても残置を実証） | **NOTE コメント（sheet.rs compile_sheet / CueSheet::new）+ 特性化テスト 4 件** |
| 5 | u64/i64 境界（`CueCommand::EntityRef(u64)`） | serde_json は u64 全域を直接扱うため i64::MAX 超（Entity generation ビット等）も欠損なくラウンドトリップする（u64::MAX 含む 4 境界値で実証）。TOML 整数は i64 のため i64::MAX 超は表現不能だが、現行ワークスペースに CueCommand の TOML 直列化経路はない（EntityRef ↔ Entity::to_bits の往復整合は W8 境界） | **特性化テスト 1 件**（command.rs in-source）|
| 6 | バリアタイムアウトの加算（barrier_offset + dur） | f64 加算は全入力で panic せず、巨大値は +inf へ飽和（永続バリア化は P25 の有限性検証で包含） | 対応不要 |

### 3. panic 経路（unwrap / unreachable / 添字 / 再帰）

| # | 経路 | 解析結果 | 対応 |
|---|------|---------|------|
| 1 | `validate/rules.rs::dfs_detect_cycle` の `position(..).unwrap()` | in_stack への挿入/除去は path への push/pop と常に対で行われ両者は同一の要素集合を保持するため、in_stack に含まれる next は必ず path 内に存在 → 発火不能 | **SAFETY コメント** |
| 2 | `validate/rules.rs` V12 の `unreachable!()` | 外側 match アームが Float \| Integer のみを通すため到達不能 | **SAFETY コメント** |
| 3 | `cue/schedule.rs::tick` の `pop().unwrap()` | `while let Some(..) = entries.last()` ガード直後のため非空 → 発火不能 | **SAFETY コメント** |
| 4 | **`dfs_detect_cycle` の再帰 DFS（D2-V 申し送りの確認）** | トリガー連鎖の深さに比例してコールスタックを消費し、数万 SB 規模の細工文書でスタック枯渇（abort）の理論的可能性がある。**P23 の rationale が本ハザード（「極端に深い trigger_storyboard 連鎖を持つ外部指示書でスタックオーバーフローの理論的可能性」）と深鎖非クラッシュテスト（10^5 段）を明記済みのため重複提案せず P23 を参照**。200 段チェーンの動作は D3-T の `v15t_long_chain_200_storyboards_validates_ok` でピン留め済み | **NOTE コメント（関数定義位置 — D2-V は compile/mod.rs 側にのみ NOTE を残していたため定義サイトへ補完）** |
| 5 | `cue/sheet.rs` のソート比較（`partial_cmp().unwrap_or(Equal)`）× 2 | NaN は Equal 扱いで panic しない（順序は規定されない）。`next_routing` の `remove(0)` は is_empty ガード済み、`document.rs`/`lib.rs`/`validate/mod.rs`/`cue/{command,mod}.rs` に unwrap/expect/添字アクセスなし（grep 実証） | NOTE（CueSheet::new）のみ |

結論: 境界内に外部入力から到達可能な panic 経路は、(a) debug ビルド限定の TimedSchedule 非負 debug_assert（NaN で発火 — 特性化済み、release では素通り）、(b) P23 記録済みの再帰 DFS スタック枯渇、の 2 点を除き存在しない。

### 投入した挙動非破壊対策（R2.3/R5.1）

1. **`validate/rules.rs`**: validate_variable_ranges / validate_loop_offset の NaN 素通り NOTE（P14 参照）、V12 `unreachable!()` の SAFETY、dfs_detect_cycle の再帰スタックハザード NOTE（P23 参照）と `position().unwrap()` の SAFETY（+27 行、コメントのみ）。
2. **`cue/schedule.rs`**: insert/extend の NaN オフセット NOTE（debug_assert 発火 / release の partition_point 前提破壊、P25 参照）、tick の NaN 時刻 NOTE、`pop().unwrap()` の SAFETY（+21 行、コメントのみ）。
3. **`cue/sheet.rs`**: compile_sheet の非有限 start_time 縮退 3 系統の NOTE、CueSheet::new の NaN ソート NOTE（+15 行、コメントのみ）。
4. **特性化テスト 16 件追加**: `tests/cue/schedule_test.rs` 4（NaN tick 全件配信・バリア停止と復帰・insert/extend の debug_assert 発火 2）、`tests/cue/sheet_test.rs` 4（NaN ソート非 panic・NaN min 脱落・全 inf NaN 化・部分 inf liveness 喪失）、`src/cue/command.rs` in-source 1（EntityRef u64 境界 4 値）、`tests/validation/transition_test.rs` 3（V12 NaN 素通り）、`tests/runtime/loop_offset_test.rs` 3（V14-V16 NaN 素通り）、`tests/validation/schema_test.rs` 1（loop_count 後置検出）。

既存テストの変更・削除 0、既存製品コード行の変更・削除 0（src 側 diff はコメント挿入と in-source テスト追記のみ）。debug_assert の新規追加なし（既存 debug_assert の NaN 発火を特性化したのみ）。

### 検証（S2）

- BEFORE（HEAD b32d004、ワーキングツリー clean）: `cargo build --workspace` 成功 / `cargo test --workspace` 全スイートグリーン（exit 0、親指示ベースライン 1160 passed / 0 failed と整合）
- AFTER: `cargo build --workspace` 成功 / `cargo test --workspace --no-fail-fast` 全スイートグリーン（1176 passed / 0 failed / 32 ignored、+16 はすべて本セル追加分）

## flaky

- BEFORE / AFTER とも wintf tests/ecs の既知フレーキー（`cue_performance_test::bench_pop_ready_empty_queue`）は失敗せず、隔離再実行は不要だった（パススルー）。

## proposals

- **P25**（report/proposals.md へ追記）: Cue パイプラインの時刻入力検証の欠如 — NaN オフセットの partition_point 前提破壊（release での黙った配信順崩壊）・tick(NaN) の全件即時配信・inf オフセットの liveness 喪失（P14 と検証方針を揃えた統合実装を推奨）
- **P26**（report/proposals.md へ追記）: loop_count の文書レベルバリデーション追加 — 現状は validate() を素通りし facade の start 時に後置検出される（P8/P14 の文書数値フィールド検証仕様群への統合を推奨）
- P23 参照: dfs_detect_cycle の再帰スタック枯渇ハザードは P23（D3-S 記録）の rationale が脆弱性側面（スタックオーバーフローの理論的可能性）と深鎖非クラッシュテストを明記済みのため、重複提案せず定義サイトへの NOTE 補完のみ実施
