# Research & Design Decisions — dola-runtime-3-facade

## Summary
- **Feature**: `dola-runtime-3-facade`
- **Discovery Scope**: Extension（既存 runtime モジュールへの 5 ファイル追加）
- **Key Findings**:
  - Tier 1 core-types は完全実装済み。`InstanceState`（7 バリアント）、`EvaluatedValue`、`RuntimeError`（4 バリアント）、`StartResult`、`Interpolator`（31 イージング）すべて利用可能
  - `compile_storyboard()` は 753 行の完全実装。内部で `doc.validate()` を呼び出しているため、facade 側の `load_document()` でのバリデーションとは二重実行になるが、安全性を優先
  - `runtime` feature gate が残存しているが、統合指針では clock 仕様（Tier 1）実装時に削除予定。facade はこの feature gate 内で実装を進める

## Research Log

### runtime Feature Gate の現状と影響

- **Context**: `Cargo.toml` に `runtime = ["dep:interpolation"]`、`lib.rs` に `#[cfg(feature = "runtime")] pub mod runtime` が残存
- **Sources Consulted**: `integration-guide.md` Section 5、`gap-analysis.md` Section 2.2
- **Findings**:
  - 統合指針 Section 5.2 では「仕様1 (core-types) 実装時に `interpolation = "0.3.0"` 常時依存化（runtime feature 削除）」と記載されているが、core-types 実装時には削除されなかった
  - 統合指針 Section 5.2 では「仕様2 (clock) 実装時に `windows-clock` feature も削除」と記載
  - facade（仕様3）では追加依存なし。feature gate の有無は facade のコードに影響しない（モジュールの配置場所 `src/runtime/` は同一）
- **Implications**: facade は現行 `runtime` feature gate 内で実装。feature 削除は別仕様（clock または専用リファクタ仕様）の責務。テスト実行時は `--features runtime` が必要

### compile_storyboard() の二重バリデーション

- **Context**: `load_document()` で `doc.validate()` を実行する設計（Req 1 AC1）だが、`compile_storyboard()` も内部で `doc.validate()` を呼んでいる（compile.rs L128）
- **Sources Consulted**: `crates/dola/src/compile.rs` L114-L140、`requirements.md` Req 1
- **Findings**:
  - `compile_storyboard()` の Preconditions コメントに「内部で validate() を実行するため、呼び出し側の事前バリデーションは不要」と記載
  - しかし、Req 1 の AC は「`load_document` 時にバリデーションし、失敗時は既存 document を保持する」と明記
  - `load_document()` でのバリデーションは「不正な指示書を受け入れない」ゲートキーパー機能であり、`compile_storyboard()` のバリデーションは「コンパイル時の安全網」
- **Implications**: 二重バリデーションはパフォーマンスへの影響は軽微（デスクトップマスコット用途で指示書は数十エントリ規模）。安全性を優先し両方維持

### VariableTimeline vs CompiledVariableTimeline の区別

- **Context**: 既存 `compile.rs` には `CompiledVariableTimeline`（コンパイラ出力）が定義されているが、facade は独自の `VariableTimeline` と `TimelineEntry` を定義する
- **Sources Consulted**: `compile.rs` L43-L59、既存 design.md
- **Findings**:
  - `CompiledVariableTimeline` はコンパイラの出力で、1 回の `compile_storyboard()` 呼び出しに対応
  - facade の `VariableTimeline` は複数 group_id のエントリを時系列で管理するコンテナ
  - `TimelineEntry` は 1 つの group_id に属するセグメント群をラップし、`CompiledSegment` を直接参照
- **Implications**: 名前の類似性に注意。`CompiledVariableTimeline` はコンパイラ層、`VariableTimeline` はランタイム層と明確に区別

### Finish Deadline の Pull 型実装

- **Context**: `finish(group_id, offset)` で設定した deadline をタイマーなしで検出する方法（facade は pull 型設計）
- **Sources Consulted**: `gap-analysis.md` Section 4.3、`requirements.md` Req 5.5
- **Findings**:
  - facade はクロックを持たず、`update(subscriber_id, current_time)` で外部から時刻を受け取る
  - deadline チェックのタイミングは `update()` 内の evaluate ループの **前** が適切
  - 理由: deadline 到達済みインスタンスの最終値を確定してから通常の evaluate に進むことで、1 回の `update()` 呼び出しで正しい最終値を配信できる
- **Implications**: `update()` の冒頭で `check_finish_deadlines(current_time)` を実行。対象インスタンスを Conclude 相当で終了させてからメインの evaluate ループに入る

### Conclude/Cancel のタイムテーブル操作順序

- **Context**: Conclude（最終値ジャンプ）と Cancel（凍結）でのタイムテーブル操作手順
- **Sources Consulted**: `gap-analysis.md` Section 4.4、`requirements.md` Req 5.3-5.4
- **Findings**:
  - **Conclude**: (1) 各セグメントの最終値を取得 → (2) SubscriptionManager の `last_values` を更新 → (3) InstanceManager で `Concluded` に遷移 → (4) TimelineManager の該当エントリ削除
  - **Cancel**: (1) 現在時刻での補間値を取得（または取得しない: 次回 update での差分検出に委ねる） → (2) InstanceManager で `Cancelled` に遷移 → (3) TimelineManager の該当エントリ削除
  - Cancel の場合、「現在値で凍結」は SubscriptionManager の `last_values` がそのまま残るので明示的な値書き込みは不要
- **Implications**: Conclude は最終値書き込みが必要だが、Cancel は `last_values` そのままで操作完了。どちらもエントリ削除は操作の最終ステップ

### 指示書差し替え時の再生中インスタンス

- **Context**: `load_document()` で新しい指示書が配信された時、再生中のインスタンスをどう扱うか
- **Sources Consulted**: `requirements.md` Req 2.1-2.4、`gap-analysis.md` Section 6
- **Findings**:
  - Req 2.1: 「旧定義を完全に上書きし、新定義で置換する」— DocumentStore のドキュメントのみの話
  - 再生中インスタンスは `CompiledStoryboard` / `CompiledSegment` を TimelineManager が保持しているため、元の定義が消えても影響しない
  - 差し替え後の新しい `start()` は新ドキュメントから新たにコンパイルする
  - 同名変数の値引き継ぎ（Req 2.2）は SubscriptionManager の `last_values` で自動的に実現される
  - 消失変数の凍結（Req 2.3）は「SubscriptionManager に購読は残るが、TimelineManager に新エントリが追加されない」状態で実現
- **Implications**: `load_document()` は DocumentStore のドキュメントを置換するだけ。再生中のインスタンスへの介入は不要（自然終了を待つ）。これが最もシンプルで安全な設計

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| A: 5 ファイル新規追加 | design.md 準拠の 1:1 モジュール対応 | 責務分離明確、Tier 3 拡張容易、テスト分離可能 | 5 ファイル同時追加（各小規模） | **採用** |
| B: 単一 facade.rs | 全コンポーネントを 1 ファイルに統合 | ファイル数最少 | 800行超、責務混在、Tier 3 拡張困難 | 不採用 |
| C: Trait ベース抽象化 | 各コンポーネントに trait 定義 | テスタビリティ最大 | 過剰設計、`&mut self` trait 合成が煩雑 | 不採用 |

## Design Decisions

### Decision 1: `runtime` Feature Gate — 現行維持

- **Context**: 統合指針では core-types 実装時に削除予定だったが未実施。facade で削除するか現行維持するか
- **Alternatives Considered**:
  1. facade 実装と同時に feature gate 削除（BREAKING CHANGE）
  2. 現行の feature gate 内で facade を実装
- **Selected Approach**: Option 2 — 現行維持
- **Rationale**: feature gate 削除は `interpolation` 依存を常時有効化する BREAKING CHANGE であり、facade の責務を超える。統合指針では clock 仕様が削除タイミングとして記載されている
- **Trade-offs**: テスト時に `--features runtime` が必要（現状維持）。将来的に clock 仕様で一括削除
- **Follow-up**: clock 仕様の tasks.md に feature gate 削除タスクを含める

### Decision 2: Finish Deadline チェック位置 — update() 冒頭

- **Context**: `finish(group_id, offset)` で設定した deadline の検知タイミング
- **Alternatives Considered**:
  1. evaluate ループの前（update 冒頭）
  2. evaluate ループの後（update 末尾）
  3. 各変数の evaluate 内（変数ごとにチェック）
- **Selected Approach**: Option 1 — update() 冒頭
- **Rationale**: deadline 到達インスタンスの最終値を確定してからメインの evaluate に進むことで、1 回の update で正しい最終値を差分配信できる。Option 2 だと deadline 到達が 1 フレーム遅延する。Option 3 は変数ごとの重複チェックで非効率
- **Trade-offs**: update() の処理順が「deadline チェック → evaluate → diff」と 3 ステップになるが、可読性は十分
- **Follow-up**: なし

### Decision 3: Conclude/Cancel 操作順序

- **Context**: 制御コマンド実行時のコンポーネント間の操作順序
- **Alternatives Considered**:
  1. 状態遷移 → 値取得 → エントリ削除
  2. 値取得 → 状態遷移 → エントリ削除
- **Selected Approach**: Option 2 — 値取得 → 状態遷移 → エントリ削除
- **Rationale**: Conclude では最終値を取得してから状態を遷移させる必要がある（遷移後はタイムテーブルのデータにアクセスする理由がない）。Cancel では現在値が SubscriptionManager の last_values に既に保持されているため明示的な値取得は不要だが、同一パターンで統一
- **Trade-offs**: Cancel で不要な値取得が発生する可能性があるが、Cancel は即座にエントリ削除するため影響は軽微
- **Follow-up**: なし

### Decision 4: 指示書差し替え時の再生中インスタンス — 自然終了を待つ

- **Context**: `load_document()` で新指示書を配信した際、再生中インスタンスへの介入方針
- **Alternatives Considered**:
  1. 全再生中インスタンスを即座に Conclude/Cancel
  2. 自然終了を待つ（DocumentStore のみ更新）
  3. 新定義で再コンパイル（hot-reload）
- **Selected Approach**: Option 2 — 自然終了を待つ
- **Rationale**: 再生中インスタンスは CompiledStoryboard を TimelineManager が直接保持しており、元の定義が消えても影響しない。即座に停止すると視覚的な不連続が発生する。hot-reload は複雑性が高く Tier 2 のスコープ外
- **Trade-offs**: 旧定義のインスタンスが新定義と並行して走る可能性があるが、ストーリーボードは数秒〜数十秒で終了するため許容範囲
- **Follow-up**: Tier 3 で競合解決を導入する際に、指示書差し替え時の競合戦略を再検討可能

### Decision 5: 差分検出メカニズムと Object 値の効率的比較（議題 D1）

- **Context**: Conclude で最終値を `force_update_last_values()` で凍結値に反映した後、次回 `update()` で subscriber に最終値を差分配信する必要がある。また、Object 型変数の比較コストを最小化したい（60fps 想定で update は頻繁に呼ばれる）
- **Alternatives Considered**:
  1. `last_values` 単一フィールドで配信値と凍結値を兼用 → Conclude 後の差分検出が失敗（前回も最終値、今回も最終値で差分なし）
  2. `last_values`（凍結値）と `last_sent_values`（前回配信値）を分離 → 差分検出は `last_sent_values` との比較で正しく動作
  3. Object 比較は構造的比較（`PartialEq`）→ O(n) だが正確
  4. Object を `Rc<DynamicValue>` 化 + compile 時に intern → `Rc::ptr_eq()` で O(1) 比較
- **Selected Approach**: Option 2 + Option 4 — `SubscriberState` に `last_sent_values` 追加、`EvaluatedValue::Object` を `Rc<DynamicValue>` 化（Tier 1 変更）
- **Rationale**:
  - **差分検出の正確性**: 凍結値（`last_values`）と前回配信値（`last_sent_values`）を分離することで、Conclude 後も正しく最終値を配信できる
  - **Object 比較の効率**: compile 時に同一内容の Object 値は同一 Rc を共有（intern）するため、内容が同じなら必ずポインタも同じ。`Rc::ptr_eq()` で O(1) 判定可能
  - **用途適合性**: デスクトップマスコット用途では Object 変数は小規模（シェル切り替え、表情番号程度）だが、update は 60fps で頻繁に呼ばれるため、diff 処理の軽量化は重要
- **Trade-offs**:
  - Tier 1 core-types の `EvaluatedValue::Object` 型を変更（BREAKING CHANGE）
  - `Rc` は serde と直接統合できないため、custom serialize impl が必要
  - compile.rs に Object intern pool 追加（~30行の追加コード）
- **Follow-up**: Tier 1 `types.rs` の修正、Tier 1 テストの修正、`compile.rs` への intern pool 追加（本仕様スコープに含む）

## Risks & Mitigations
- **二重バリデーションのパフォーマンス** — デスクトップマスコット用途では問題にならない規模。最適化は計測後に判断
- **f64 精度の蓄積誤差** — アニメーション時間は数十秒程度。f64 の有効桁（15-16桁）で十分
- **Tier 3 拡張ポイントの不足** — コメントベースのフックポイントと構造的分離で対応。trait 抽象化は不要
- **`runtime` feature gate の忘却** — テスト CI で `--features runtime` を維持。clock 仕様で一括削除

## References
- `crates/dola/src/runtime/mod.rs` — 現行 runtime モジュール構成
- `crates/dola/src/compile.rs` — `compile_storyboard()` 実装（753 行）
- `crates/dola/src/validate.rs` — `Validate` trait 実装（403 行）
- `.kiro/specs/dola-runtime-engine/integration-guide.md` — 子仕様統合指針
- `.kiro/specs/dola-runtime-3-facade/gap-analysis.md` — ギャップ分析（4 つの設計判断を解決）
