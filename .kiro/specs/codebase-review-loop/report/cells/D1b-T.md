# D1b-T: dola 補間・状態 × テスト網羅性

- status: completed
- commit: test(D1b): 補間・状態・競合解決のテスト空白に36件のギャップテストを追加

## findings

### モジュール×テスト対応表（改善前 → 改善後）

| モジュール | 対象 | 既存テスト | 追加 | 備考 |
|------------|------|-----------|------|------|
| `runtime/conflict_resolver.rs` (288 LOC) | 検出基本（変数別/初回/同一変数/自然終了後）・4終了戦略・Never 3系・混合ポリシー・副作用なし | `tests/runtime/conflict_resolution_test.rs` 14件 | — | 既存で十分 |
| 〃 | Paused インスタンスの競合対象性（detect_overlaps は Playing/Paused 両方） | なし | 1件 | `paused_instance_still_conflicts` |
| 〃 | 時間シフト開始の競合判定（座標系の特性化） | なし | 1件 | `time_shifted_start_on_same_variable_still_conflicts` — 壁時計上は非重複でも競合する現行挙動を固定（P11） |
| 〃 | 複数インスタンス同時競合時の affected 全列挙 | mixed_policies は逐次1件ずつのみ | 1件 | `overlap_with_multiple_instances_affects_all`（HashSet 由来の順序非決定性は sort で吸収） |
| 〃 | 競合終了した親のトリガー不発火（stale trigger_store 領域の特性化） | なし | 1件 | `conflict_terminated_parent_trigger_never_fires`（D1a-V 申し送り、P12） |
| 〃 | `resolve_conflicts_excluding` の skip_group_ids 経路（トリガー親除外） | 間接のみ（trigger テストは時間が隣接し重複未発生） | 1件 | `triggered_child_excludes_parent_from_conflict` — 子が親と同一変数で実際に重複する構成で除外を固定 |
| `runtime/document_store.rs` (170 LOC) | 初期 None・有効 store・無効時の既存保持・SB 検索・差し替え | in-source 6件 | 1件 | 空ストアへの無効 store 失敗時に None 維持（`store_invalid_document_on_empty_store_keeps_none`） |
| `runtime/types.rs` (217 LOC) | Conflict 構造/Display・StartResult | in-source 3件 + `tests/runtime/core_types_test.rs` | 5件 | Display 未固定 4 バリアント（InvalidLoopCount / TooShortDuration / CompileError / InvalidVariableId）+ EvaluatedValue 異種バリアント不等（`cross_variant_never_equal`） |
| `runtime/clock.rs` (116 LOC) | 単調増加・正有限値・ms 精度・呼出コスト | in-source 4件 | — | 既存で十分（QPC 依存のため unit はこれが上限） |
| `runtime/instance_state.rs` (70 LOC) | 許可遷移10・拒否遷移8・terminal 判定・from_policy 全対応 | `tests/runtime/core_types_test.rs` 22件 | 3件 | 自己遷移（Created/Playing/Paused → 同一状態）拒否の固定 |
| `runtime/interpolator/` (178+208 LOC) | 線形/明示 Linear/境界 t/クランプ/Integer 丸め/Object 切替/QuadraticIn/ベジェ2種/全30バリアント | `interpolator/tests.rs` 14件 | 4件 | `ObjectInternPool` 直接テスト空白を解消（同値→同一 Rc、異値→別 Rc、pool 有無での ptr_eq/PartialEq 差） |
| 〃 | `scalar_value`/`transition_value_to_dynamic` の変換分岐（Dynamic Float/Integer、非数値→0.0、Scalar→DynamicValue::Float） | なし | 3件 | 私有関数の全分岐を公開経路（interpolate）経由でカバー |
| 〃 | Integer 補間の負値丸め方向（away from zero）・イージング併用 | 正値のみ | 2件 | `f64::round` 準拠（-2.5 → -3）を固定 |
| `runtime/instance_manager/` (262+187 LOC) | 生成初期値・基本遷移・terminal 自動削除（4種）・pause/resume・deadline・複数独立 | `instance_manager/tests.rs` 15件 | 10件 | 下記 |
| 〃 | 無効 group_id への全操作（get_mut/transition/pause/set_pause_start/resume/set_finish_deadline） | get のみ | 1件 | 全6操作の InvalidGroupId 一括固定 |
| 〃 | 不正遷移エラーの特性化・失敗時の状態保持 | なし | 3件 | 既存 ID でも `InvalidGroupId` を返すエラー型の縮退を固定（内部 API のため提案化せず所見のみ） |
| 〃 | pause_start 未設定 resume の防御パス・get_mut 可変性・remove no-op・同一 gid 上書き・trigger_states 初期化・deadline 未設定の除外 | なし | 6件 | |
| `storyboard.rs` (166 LOC) | Storyboard/Entry/KeyframeRef/InterruptionPolicy serde + LoopOffset 全形式 | `tests/general/core_types_test.rs` 10件 + `tests/runtime/loop_offset_test.rs` 10件 + `tests/trigger/serde_test.rs` | — | 既存で十分 |
| `transition.rs` (57 LOC) | TransitionValue/Def/Ref serde・delay デフォルト | `tests/general/core_types_test.rs` 7件 | — | 既存で十分 |
| `easing.rs` (60 LOC) | 全31名称 serde・パラメトリック2種・untagged 判別 | `tests/general/core_types_test.rs` 5件 | — | 既存で十分 |
| `value.rs` (54 LOC) | 全7バリアント serde・BTreeMap 決定的順序 | `tests/general/core_types_test.rs` 9件 | 3件 | 手書き Hash/Eq 実装が未テストだった（ObjectInternPool の HashMap キーとして使用）。同値同ハッシュ・0.0/-0.0 の Hash/Eq 契約違反の特性化（P10）・NaN 自己不等の固定 |
| `variable.rs` (34 LOC) | 3型 serde・typewriter | `tests/general/core_types_test.rs` 4件 | — | 既存で十分 |

追加テスト合計 36 件（統合 16 件: `tests/runtime/conflict_resolution_test.rs` 5, `tests/runtime/core_types_test.rs` 8, `tests/general/core_types_test.rs` 3 / in-source 20 件: `interpolator/tests.rs` 9, `instance_manager/tests.rs` 10, `document_store.rs` 1）。配置は S9 準拠（統合テストはドメインサブディレクトリの既存ファイルへ追記、ユニットテストは既存の Separated 方式 `{module}/tests.rs` または Inline `mod tests` を踏襲）。

### 除外テスト

0 件。重複候補を精査したが、`interpolator/tests.rs` の `all_30_easing_names_mapping`（実行時マッピング）と `tests/general/core_types_test.rs` の `all_31_easing_names_json_roundtrip`（serde 表現）は検証対象が異なり重複ではない。`instance_manager/tests.rs` と `tests/runtime/core_types_test.rs` の遷移テストも層が異なる（マネージャ経由の自動削除 vs `InstanceState::try_transition` 純関数）ため温存。

### テスト不能箇所・深掘り所見

1. **競合検出はストーリーボードローカル座標で行われ wall-clock を無視する（P11）** — facade は全 SB を `compile_and_validate(name, 0.0)` で base 0.0 にコンパイルし、`detect_overlaps` は `_start_time` 引数（アンダースコア付き＝意図的未使用）を使わず compile 時座標どうしを比較する。壁時計上重ならないスケジュール（[0,2] 終了直後の t=2.0 開始）でも常に競合し先行インスタンスが終了する。当初「隣接セグメントは厳密不等号で非競合」と仮説を立てたテストが RED になったことで発見し、現行挙動の特性化テストへ転換した。wall-clock 換算判定は挙動変更のため P11 として提案記録。
2. **競合解決の終了経路で trigger_store エントリが残置される（P12、D1a-V 申し送りの確認）** — conflict_resolver の 4 終了経路は instance_manager からのみ削除し、facade 保有の trigger_store は触れない。`process_triggers` がインスタンス起点走査のため残置エントリは読まれず、外部挙動は「競合終了した親のトリガーは発火しない・panic しない」に留まることを `conflict_terminated_parent_trigger_never_fires` で固定した（将来修正の回帰検知器）。修正は facade（D1a 境界）とのシグネチャ変更をまたぐため P12 として提案記録。
3. **`DynamicValue` の Hash/Eq 契約違反エッジ（P10）** — `Float` は `to_bits()` ハッシュのため `0.0 == -0.0`（PartialEq 真）なのにハッシュが異なる。`ObjectInternPool` で別エントリとして intern される（メモリ安全だが規約違反）。決定的に検証するため HashMap 経由ではなく `DefaultHasher` 直接比較で特性化した。
4. **`instance_manager::transition` のエラー縮退** — 不正遷移も ID 不在も同じ `InvalidGroupId(gid)` を返す。pub(crate) 内部 API で外部観測不能のため提案化せず、特性化テスト（`invalid_transition_on_existing_instance_reports_invalid_group_id`）で現行挙動と失敗時の状態保持のみ固定した。
5. **`clock.rs` は QPC 直結のためこれ以上の unit 分解は不可** — 既存4件（単調性・有限性・精度・コスト）が実用上の上限。注入可能な時刻源への抽象化はシグネチャ変更（テスト容易性リファクタの範囲超）のため見送り。
6. **`conflict_resolver::resolve_conflicts`（非 excluding 版）は `#[allow(dead_code)]` の未使用ラッパー** — 全呼び出しは excluding 版経由（facade が空 HashSet を渡す）。削除は D1b-S（シンプル化観点）の判断材料として申し送り（公開範囲 pub(crate) のため挙動非破壊で除去可能の見込み）。

### 検証（S2）

- BEFORE: `cargo build --workspace` 成功 / `cargo test --workspace` 全グリーン（1037 passed / 0 failed）
- AFTER: `cargo build --workspace` 成功 / `cargo test --workspace` 全グリーン（1073 passed / 0 failed、+36 はすべて追加分。既存スイートはベースラインと同一結果）
- RED フェーズ: 特性化テスト中心のため原則 N/A だが、`time_shifted_start_on_same_variable_still_conflicts` と `overlap_with_multiple_instances_affects_all` は仮説（壁時計座標での重複判定）で一度 RED となり、実挙動（ローカル座標判定）の発見と特性化への転換という形で RED→GREEN を経由した
- wintf `cue_performance` フレーキーは本実行では発生せず

## proposals

- P10（report/proposals.md へ追記）: DynamicValue の Hash/Eq 契約違反（Float 0.0 / -0.0）の解消
- P11（〃）: 競合検出の wall-clock 座標非対応（時間シフトした非重複スケジュールの誤競合）
- P12（〃）: 競合解決の終了経路における trigger_store エントリ残置（リソースリーク）の解消
