# D3-T: dola 検証・Cue × テスト網羅性

- status: completed
- commit: test(D3): 検証ルール・Cue 配信エンジン・ドキュメント定義のテスト空白に34件のギャップテストを追加

## findings

### モジュール×テスト対応表（改善前 → 改善後）

| モジュール | 対象 | 既存テスト | 追加 | 備考 |
|------------|------|-----------|------|------|
| `validate/mod.rs` (183 LOC) | V1（スキーマ）/ V2（KF重複）/ V3（予約名）/ V4（未定義変数）/ V5（未定義トランジション） | `tests/validation/schema_test.rs` 6件 | — | 既存で十分 |
| 〃 | 複数ルールのエラー蓄積（fail-fast しないこと） | なし（単一ルールずつの検証のみ） | 1件 | V1+V4 が 1 回の validate() で両方報告されることを固定（`validate_collects_errors_from_multiple_rules_in_one_pass`） |
| 〃 | V7（transition→variable 必須）/ V8（at/between 排他）/ V9（純粋KF必須・トリガー許容）/ V11（to/relative_to 排他） | `tests/validation/{keyframe,transition}_test.rs` + `tests/trigger/validation_test.rs` | — | 既存で十分 |
| 〃 | V16t（トリガー排他）/ V18t（トリガー先存在） | `tests/trigger/validation_test.rs` 3件 | — | 既存で十分 |
| `validate/rules.rs` (400 LOC) | V6: KeyframeRef::Single の前方参照・未定義・暗黙KF・"start" 予約 | `tests/validation/keyframe_test.rs` 4件 | — | 既存で十分 |
| 〃 | V6: KeyframeRef::Multiple / WithOffset(Single/Multiple) / between.from・to の未定義検出 | なし（Single バリアントのみ） | 4件 | `collect_keyframe_names_from_ref` の全バリアント経路と between 参照検証を初めて固定 |
| 〃 | V10: Object 型の from 禁止・Scalar to 拒否 | `tests/validation/transition_test.rs` 2件 | — | 既存で十分 |
| 〃 | V10: Object 型の relative_to / easing 禁止 | なし | 2件 | ObjectTransitionViolation の field="relative_to"/"easing" 分岐 |
| 〃 | V12: Float initial の min/max 逸脱・to の値域・Integer initial の max 超過 | `tests/validation/transition_test.rs` 4件 | — | 既存で十分 |
| 〃 | V12: from の値域逸脱 / 境界値ちょうど（==min/==max）は合格 / Integer initial の min 未満 / Integer 変数のトランジション値域（f64 変換経路） | なし | 4件 | 排他的比較（< min / > max）の境界仕様を明文化 |
| 〃 | V13: 数値変数への Dynamic to 拒否 | `tests/validation/transition_test.rs` 1件 | — | 既存で十分 |
| 〃 | V13: 数値変数への Dynamic from 拒否 | なし | 1件 | from 側アーム |
| 〃 | V14-V16（loop_offset 負値・範囲逆転） | `tests/runtime/loop_offset_test.rs` 4件（runtime ドメイン側） | — | ドメイン配置は runtime だが検証対象は validate/rules.rs。重複追加せず |
| 〃 | V14t（自己参照）/ V15t（A→B→A, A→B→C→A, 非循環チェーン） | `tests/trigger/validation_test.rs` 4件 | — | 既存で十分 |
| 〃 | V15t: 200 段チェーン（再帰 DFS の現行動作ピン留め）/ 閉路パスの形式（先頭==末尾・メンバー集合）/ ダイヤモンド合流の非循環判定 / 自己参照が SelfReference+TriggerCycle の両方を報告 | なし | 4件 | D2-V 申し送り（dfs_detect_cycle 再帰のスタック枯渇懸念）に対する T 観点の特性化。`v15t_long_chain_200_storyboards_validates_ok` が中規模長鎖の回帰検知器 |
| `cue/command.rs` (317 LOC) | バリアント網羅・Clone/Debug/PartialEq・ActorKey hash・基本 serde（ActorKey/CueTarget/CueCommand/BarrierKind）・CuePayload From 変換 | in-source `mod tests` 7件 | — | 既存で十分 |
| 〃 | ActorKey::new/Display/From<String> 等価性 / EntityKey の名前空間分離（Spot vs Balloon vs Actor、スロット違い）と serde / RoutingCommand serde / CuePayload・Cue serde | なし | 5件 | in-source `mod tests` へ追記（S9 Inline）。EntityRegistry 名前空間の型分離と配送制御コマンドの永続化往復を初めて固定 |
| `cue/schedule.rs` (260 LOC) | tick/ready 2 フェーズ・冪等性・絶対→相対変換・WaitForInput バリア停止/解除/タイムアウト・Timeout バリア・ルーティング FIFO・clear/remaining/is_completed・extend | `tests/cue/schedule_test.rs` 17件 | — | 既存で十分 |
| 〃 | WaitForChoice バリアの choice_id 付き解除 / タイムアウト付き WaitForInput のジャンプ通過スキップ（continue 経路）/ タイムアウト前の外部解除 / バリア中 clear / Routing→Barrier→Payload の停止位置 / Entry::offset 全バリアント | なし | 6件 | バリア 3 種 × 解除 3 経路（外部・タイムアウト・スキップ）のマトリクスを完成 |
| 〃 | 同時刻エントリの配信順 | なし（contains 検証のみで順序未固定） | 2件 | 特性化: insert()=FIFO / extend()=LIFO の不整合を発見・固定（P22 提案） |
| `cue/sheet.rs` (112 LOC) | 昇順ソート・empty/len・actor フィルタ・0 ベース正規化・into_entry 3 種・TimedSchedule 統合・serde | `tests/cue/sheet_test.rs` 11件 | — | 既存で十分 |
| 〃 | 安定ソート（同時刻 Cue の記述順保持）/ 負の start_time の 0 基準正規化 / compile_sheet のバリア・ルーティング種別保存 | なし | 3件 | 負時刻正規化は TimedSchedule::insert の非負 debug_assert に対する安全境界の固定 |
| `document.rs` (24 LOC) | フルラウンドトリップ serde | `tests/general/core_types_test.rs` 2件 | — | 既存で十分 |
| 〃 | `#[serde(default)]` の省略フィールド空コレクション化 / schema_version 必須（省略でエラー） | なし | 2件 | in-source `mod tests`（S9 Inline）。最小 JSON `{"schema_version":"1.0"}` の受理仕様を固定 |
| `lib.rs` (38 LOC) | 再エクスポートのみ | （コンパイル時検証） | — | テスト不要と判断 |

追加テスト合計 34 件（統合 27 件: `tests/validation/` 12, `tests/trigger/validation_test.rs` 4, `tests/cue/` 11 / in-source 7 件: `cue/command.rs` 5, `document.rs` 2）。配置は S9 準拠（統合テストは既存ドメインサブディレクトリの対応ファイルへ追記、ユニットテストは Inline 方式 `mod tests`）。テスト入口ファイル（`tests/{validation,trigger,cue}.rs`）の変更は不要（新規ファイルなし）。

### 除外テスト

0 件。重複候補として精査した点: (1) V14-V16（loop_offset）は `tests/runtime/loop_offset_test.rs` が validate 経由で網羅済みのため validation ドメインへ重複追加しなかった。(2) `cue/command.rs` の in-source serde テストと `tests/cue/sheet_test.rs::cue_sheet_serde_roundtrip` は対象型が異なり（ドメイン型単体 vs CueSheet 全体）重複ではない。(3) trigger ドメインのバリデーションテストは validate/ を検証するが、検証軸（トリガー V14t-V18t）が validation ドメイン（V1-V13）と分割されており重複ではない。

### テスト不能箇所・深掘り所見

1. **`dfs_detect_cycle` の再帰実装（D2-V 申し送りの確認）** — `validate/rules.rs:369` の DFS はチェーン長に比例する再帰深度を持ち、超長鎖（数万 SB）でスタック枯渇の可能性がある。T 観点としては `v15t_long_chain_200_storyboards_validates_ok` で「中規模長鎖が動作する」現行挙動をピン留めした。反復化（明示スタック）は挙動非破壊で実施可能なため D3-S/D3-V へ申し送り。
2. **TimedSchedule の同時刻エントリ配信順の不整合（新規発見）** — `insert()` は `partition_point` により同一オフセットで FIFO、`extend()` は安定降順ソート＋末尾 pop により LIFO となり、投入 API の選択で観測順が逆転する。順序統一は配信順という外部観測可能な挙動の変更を伴うため P22 として提案記録（特性化テスト 2 件で現行挙動を固定済み）。
3. **`notify_barrier_resolved` の choice_id 未検査** — `_choice_id` は無視され、WaitForChoice バリアも任意の引数（None 含む）で解除される。選択 ID の照合は将来の意味論変更となり得るが、現行は「無条件解除」が仕様（`wait_for_choice_barrier_resolved_with_choice_id` で特性化）。
4. **`tick()` の冪等性ガードは `ready_buffer` 非空が条件** — 直前 tick の収集が 0 件だった場合、同一（または過去）時刻での再 tick がガードを素通りし `current_offset` が巻き戻る。観測可能な悪影響（再配信・スキップ）は構造上発生しない（entries は消費済み pop のため）と確認したが、時刻巻き戻りの扱いは P9/P15（時刻入力検証）と同根のため新規提案は追加せず既存提案へ委ねる。
5. **`document.rs` 冒頭の `// TODO: Implement DolaDocument` は陳腐化**（実装済み）。コメント除去は挙動非破壊だが T 観点（テスト追加のみ）の範囲外のため D3-S へ申し送り（D1a/D2 の同種所見と同パターン）。
6. **RED フェーズ代替の検証** — 追加テストは既存挙動の特性化のため RED は N/A。代わりに順序依存の 3 件（insert FIFO / extend LIFO / CueSheet 安定ソート）について、期待値を実装読解（`partition_point` の挿入位置・`sort_by` の安定性・末尾 pop の消費方向）から導出してから記述し、実行で一致を確認した。

### 検証（S2）

- BEFORE: HEAD 9d7b203 で `cargo build --workspace` 成功 / `cargo test --workspace` は wintf ecs スイートの既知フレーキー 1 件のみ fail（78 passed / 1 failed）→ 隔離再実行（`cargo test -p wintf --test ecs`）で 79 passed / 0 failed の安定合格を確認（パススルー）。dola を含む他の全スイートはグリーン（親指示のベースライン 1126 passed / 0 failed と整合）
- AFTER: `cargo build --workspace` 成功 / `cargo test --workspace` 1160 passed / 0 failed（+34 はすべて追加分。既存テストの変更・削除なし）
- 変更はテスト追加のみ（src 側変更は `cue/command.rs` / `document.rs` の `#[cfg(test)]` モジュール追記のみ）で、外部観測可能な挙動の変更なし（R5.1 充足）

## flaky

- BEFORE 実行で既知の wintf tests/ecs スイート（`cue_performance_test::bench_pop_ready_empty_queue` 系）が 1 件 fail（78 passed / 1 failed）。境界外（wintf）であり本セルの変更とは無関係。プロトコルに従い隔離再実行 1 回で安定合格（79 passed / 0 failed）を確認し、パススルーと判定。

## proposals

- P22（report/proposals.md へ追記済み）: TimedSchedule の同時刻エントリ配信順の不整合（insert = FIFO / extend = LIFO）
