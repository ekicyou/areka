# Implementation Tasks — dola-runtime-3-facade

## Major Tasks

- [x] 1. Tier 1 core-types への Object Rc 化の適用 (12) (P)
  - [x] 1.1. EvaluatedValue の Object バリアントを Rc でラップし、PartialEq で ptr_eq 比較を実装 (12)
  - [x] 1.2. EvaluatedValue の serde カスタム実装を追加し、Object の serialize 時に Rc を unwrap (12)

- [x] 2. compile.rs への Object intern pool 追加 (12) (P)
  - [x] 2.1. Object 値の intern pool 構造体を実装し、同一内容の DynamicValue に同一 Rc を返すロジックを追加 (12)
  - [x] 2.2. compile_storyboard 関数に intern pool を統合し、Object 値生成時に intern を適用 (12)

- [x] 3. DocumentStore モジュールの実装 (1, 2) (P)
  - [x] 3.1. DocumentStore 構造体と基本メソッドを実装（指示書保持、バリデーション、ストーリーボード検索） (1, 2)
  - [x]* 3.2. DocumentStore の単体テストを実装（バリデーション成功/失敗、既存保持、差し替え） (1, 2)

- [x] 4. InstanceManager モジュールの実装 (3, 4, 5, 9) (P)
  - [x] 4.1. StoryboardInstance 構造体とインスタンス管理の基本メソッドを実装（作成、参照、状態遷移） (3, 4, 9)
  - [x] 4.2. Pause/Resume 制御ロジックを実装（pause_accumulated 計算、end_time 再計算） (5, 9)
  - [x] 4.3. Finish deadline 機能を実装（deadline 設定、expired インスタンス検出） (5)
  - [x]* 4.4. InstanceManager の単体テストを実装（正常/不正状態遷移、pause/resume、エラー処理） (3, 4, 5, 9)

- [x] 5. TimelineManager モジュールの実装 (7, 8, 10, 11) (P)
  - [x] 5.1. TimelineManager 構造体とタイムテーブルエントリ管理メソッドを実装（挿入、削除、存在確認） (8)
  - [x] 5.2. evaluate メソッドを実装（effective_time 計算、補間、最新 group_id 優先、Pause 固定） (7, 8, 11)
  - [x] 5.3. 終了済みタイムテーブルエントリの自動破棄ロジックを実装 (7, 8)
  - [x] 5.4. collect_final_values メソッドを実装（Conclude 用の全変数最終値取得） (5)
  - [x]* 5.5. TimelineManager の単体テストを実装（評価、Pause 固定、最新優先、エントリ破棄） (7, 8, 10, 11)

- [x] 6. SubscriptionManager モジュールの実装 (6, 7) (P)
  - [x] 6.1. SubscriptionManager 構造体と購読操作メソッドを実装（購読、解除、全解除） (6)
  - [x] 6.2. diff_and_update メソッドを実装（last_values/last_sent_values 分離、差分検出、Object ptr_eq 比較） (7, 12)
  - [x] 6.3. force_update_last_values メソッドを実装（Conclude 用の凍結値強制更新） (5, 7)
  - [x]* 6.4. SubscriptionManager の単体テストを実装（購読、差分検出、凍結変数、指示書受信前購読） (6, 7)

- [x] 7. DolaRuntime facade の実装 (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11)
  - [x] 7.1. DolaRuntime 構造体の基本フィールドとコンストラクタを実装（全内部コンポーネント、next_group_id） (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11)
  - [x] 7.2. load_document メソッドを実装（バリデーション、DocumentStore 委譲） (1, 2)
  - [x] 7.3. start メソッドとフロー統合を実装（compile, ZeroDurationWithLoop チェック, InvalidLoopCount チェック, group_id 採番, Tier 3 hook ポイント） (3, 4)
  - [x] 7.4. 制御コマンドメソッドを実装（pause, resume, conclude, cancel, finish と内部フロー） (5, 9)
  - [x] 7.5. 購読メソッドを実装（subscribe, unsubscribe, unsubscribe_all） (6)
  - [x] 7.6. update メソッドを実装（finish_deadline チェック → 自然終了検知 → evaluate ループ → diff） (7)
  - [x] 7.7. calculate_end_time メソッドを実装（インスタンス非生成、コンパイルのみ） (4)

- [x] 8. Integration テストの実装 (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11)
  - [x]* 8.1. フル再生サイクルテスト（load → start → update → 自然終了 → 差分空） (1, 3, 6, 7, 8, 9)
  - [x]* 8.2. Pause/Resume サイクルテスト（値固定、継続、end_time 再計算） (5, 7, 9)
  - [x]* 8.3. 指示書差し替えテスト（同名変数引き継ぎ、消失変数凍結） (2, 6, 7)
  - [x]* 8.4. 同時再生テスト（異なる変数の並行動作） (10)
  - [x]* 8.5. Conclude/Cancel/Finish フローテスト（最終値配信、凍結、遅延終了） (5, 7)

## Task Summary

- **Total Major Tasks**: 8
- **Total Sub-Tasks**: 28
- **Optional Tests** (marked with *): 9
- **Requirements Coverage**: All 12 requirements (Req 1-12) mapped
- **Parallel Tasks**: 6 major tasks (1, 2, 3, 4, 5, 6) can start in parallel after dependencies resolved
- **Estimated Effort**: 1-3 hours per sub-task (average 2 hours × 28 = 56 hours total, tests excluded)

## Notes

- **Dependencies**: Major 1-2 must complete before Major 3-7 (Object Rc 化と intern pool が前提)
- **Facade Integration**: Major 7 requires Major 3-6 completion (all internal components)
- **Testing Strategy**: Unit tests per module (optional *), integration tests validate end-to-end flows
- **Tier 1 Breaking Change**: Major 1 modifies `EvaluatedValue::Object` type (affects all dola consumers)
- **Code Organization**: All code in `crates/dola/src/runtime/` (5 new files: document_store.rs, instance_manager.rs, timeline_manager.rs, subscription_manager.rs, facade.rs)
- **Task Integration**: Multiple requirements combined where implementation naturally integrates (e.g., Major 7 implements facade for all 11 requirements)
- **Design Updates Included**: 議題D1 (Object Rc化, last_values/last_sent_values 分離), 議題D2 (自然終了検知), loop_count i32 型変更
