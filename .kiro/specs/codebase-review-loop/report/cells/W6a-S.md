# W6a-S: wintf ポインター入力 × シンプル化（デッドコードの実証付き削除）

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点
- セルID: W6a-S（領域 W6a「wintf ポインター入力」 × 観点 S「シンプル化」）
- 性質: **非挙動変更タスク**（リファクタリング／簡素化。R5.1）。直前の W6a-T（特性化テスト28件）が回帰検知器。簡素化後に S2 が緑であることが挙動非破壊の証拠。Feature Flag Protocol 不要。
- requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3（デッドコード削除として 2.9, 2.10）
- design: S 観点手順（S2/S3）、S6 karpathy（churn 回避・デッドコード除去）、領域定義「W6a: wintf ポインター入力 / `crates/wintf/src/ecs/pointer/`」、セル断片様式、提案記録様式
- 参考: `report/cells/W6a-T.md`（テスト済み範囲・P57 の根拠）、`report/cells/W1-S.md`（実証付き削除の先例）、`report/proposals.md`（P57）
- 境界: `crates/wintf/src/ecs/pointer/`。**境界拡張**として `crates/wintf/src/ecs/mod.rs` の再エクスポート2行（41,48）の削除を含む（理由は後述「境界拡張」節）。

## 調査範囲
W6a-T が作成したモジュール×テスト対応表（types/buffers/systems/dispatch/nchittest_cache）を基に、イベント収集・配信ロジック（`dispatch/`・`buffers.rs`・`systems.rs`・`types.rs`）の S6 簡素化候補を検証した。最有力候補は P57（W6a-T 申し送り）のデッドコード削除。

## デッドコード削除の実証（必須記録）

### 対象1: `process_pointer_buffers` / `process_mouse_buffers`（P57・最有力）

**実証手順**（ワークスペース全域 grep + build）:
- `process_pointer_buffers|process_mouse_buffers` をリポジトリ全 `*.rs`（`crates/`・`examples/`・`tests/`・`benches/`）で検索。**本番コードでの参照は以下のみ**:
  - 定義: `systems.rs:24`（`process_pointer_buffers`）、`systems.rs:160-162`（`#[deprecated]` エイリアス `process_mouse_buffers`、本体は前者へ委譲）。
  - 再エクスポート: `pointer/mod.rs:27,34`、`ecs/mod.rs:41,48`。
  - 廃止コメント: `world/mod.rs:114`「注: process_pointer_buffersは廃止」（**コードではなくコメント**。実呼び出しなし）。
  - 唯一の呼び出し元: **W6a-T で新規追加した特性化テスト2件のみ**（`test_process_pointer_buffers_*`、`systems.rs` の `add_systems(process_pointer_buffers)`）。
- `add_systems` のワークスペース全域 grep でも、`process_pointer_buffers` を登録する行は本番スケジュール（`world/mod.rs` の Input/UISetup/FrameFinalize 等）に**存在しない**。`world/mod.rs:108-138` の Input スケジュールには `dispatch_pointer_events`・`dispatch_drag_events`・`cleanup_drag_state`・`debug_pointer_state_changes`・`debug_pointer_leave` が登録される一方、`process_pointer_buffers` は登録されない（world/mod.rs:114-116 が明示）。
- 本番の thread_local 消費は `transfer_buffers_to_world`（`buffers.rs`、`try_tick_world` 冒頭 `world/mod.rs:458` から WndProc スレッド上で `try_run_schedule(Input)` の前に同期呼び出し）に一本化済み。

**判定**: 本番呼び出しゼロを実証。デッド/レガシー `pub` 関数として削除（S6/karpathy のデッドコード除去。R2.9/R2.10）。本ワークスペースは publish=false 相当で後方互換考慮不要（W1-S 先例）。削除に伴い唯一の呼び出し元である W6a-T 由来テスト2件（`test_process_pointer_buffers_button_down_priority`、`test_process_pointer_buffers_applies_position_wheel_doubleclick`、`systems.rs`）も対象消失のため除去。

### 対象2: `push_mouse_sample`（隣接デッドコード）

- `buffers.rs:57-62` の `#[allow(dead_code)] #[inline] pub(crate) fn push_mouse_sample`（`push_pointer_sample` への後方互換エイリアス）。
- `push_mouse_sample` をワークスペース全域 grep → **定義1件のみ**（呼び出しゼロ）。`pub(crate)`（クレート内部限定・公開 API 表面ではない）かつ `#[allow(dead_code)]`（コンパイラが未使用と確認済みの証跡）。
- **判定**: 公開 API 表面に一切影響しない純粋な内部デッドコード。削除（S6 デッドコード除去）。`process_*` より明確にデッド（`pub(crate)` ゆえ外部互換義務なし）。

## 適用した簡素化の一覧（各々の挙動非破壊根拠）

| # | 適用箇所 | 内容 | 挙動非破壊根拠 |
|---|----------|------|----------------|
| 1 | `systems.rs` | `process_pointer_buffers`（本体 ~134行）削除 | ワークスペース全域で本番 `add_systems` 登録ゼロ・本番呼び出しゼロを grep 実証（上記対象1）。本番 thread_local 消費は `transfer_buffers_to_world` に一本化済みで機能同値。`cargo build --workspace`（areka 本体含む）成功が「公開シンボル消失でビルドが壊れない＝未使用」の追加実証 |
| 2 | `systems.rs` | `process_mouse_buffers`（`#[deprecated]` エイリアス、3行）削除 | `process_pointer_buffers` へ委譲するだけのレガシー別名。同上 grep で本番呼び出しゼロ |
| 3 | `systems.rs` | 関数2件削除に伴う不要 import の削減（`std::time::Instant`・`super::buffers::{BUTTON_BUFFERS, DOUBLE_CLICK_BUFFERS, MODIFIER_STATE, POINTER_BUFFERS, WHEEL_BUFFERS}` を削除、`super::types::{...}` を残存関数が使う `{DoubleClick, PointerLeave, PointerState, WheelDelta}` のみに縮約） | import の削減のみ。残存関数（`clear_transient_pointer_state`・`debug_*` とそのエイリアス）が使用する型・モジュールは保持。ビルド警告ゼロで未使用なことを確認 |
| 4 | `systems.rs`（tests） | `process_pointer_buffers` を唯一呼んでいた W6a-T 由来テスト2件を除去 + テスト内の冗長 import `use super::super::types::WheelDelta;`（`use super::*` で既に供給される重複）を削除 | テスト対象（`process_pointer_buffers`）の削除に伴う機械的追随。`transfer_buffers_to_world` を検証する buffers.rs の9テスト（本番経路の回帰検知器）は全て残存（実測: buffers 9件 GREEN） |
| 5 | `pointer/mod.rs` | `process_pointer_buffers`（27行目）・`process_mouse_buffers`（34行目）の再エクスポート削除 | 削除した関数への再エクスポート。残りの `clear_transient_*`・`debug_*` の再エクスポートは保持 |
| 6 | `ecs/mod.rs`（境界拡張） | `process_pointer_buffers`（41行目）・`process_mouse_buffers`（48行目）の再エクスポート削除 | 同上。削除しないと `pub use pointer::{... process_pointer_buffers}` が存在しないシンボルを参照しビルド不能になるため、一貫性のため削除（境界拡張の理由は次節） |
| 7 | `buffers.rs` | `push_mouse_sample`（57-62行、`pub(crate)` デッドエイリアス）削除 | 呼び出しゼロを grep 実証・`#[allow(dead_code)]` 付きでコンパイラも未使用と確認済み・`pub(crate)` で公開 API 非該当（上記対象2） |

差分: 4 ファイル変更、**+3 / −302 行（net −299）**。
- `crates/wintf/src/ecs/mod.rs`: +1 / −2
- `crates/wintf/src/ecs/pointer/buffers.rs`: 0 / −7
- `crates/wintf/src/ecs/pointer/mod.rs`: +1 / −5
- `crates/wintf/src/ecs/pointer/systems.rs`: +1 / −288

`world/mod.rs:114-116` の「廃止」コメントは、削除後もコメント内容（廃止済み）とコード状態（関数不在）が一致する方向（コメントは境界外の `world/` のため未変更。コメント文言の更新は本セル境界外であり挙動非破壊性にも影響しない）。

## 境界拡張（ecs/mod.rs 再エクスポート削除）の理由

タスク境界は `ecs/pointer/` だが、`ecs/mod.rs:41,48` は `process_pointer_buffers`/`process_mouse_buffers` を**名指しで** `pointer::` から再エクスポートしている。pointer 側で関数を削除すると `ecs/mod.rs` の `pub use pointer::{... process_pointer_buffers}` / `pub use pointer::{... process_mouse_buffers}` が存在しないシンボルを参照し、`cargo build` がコンパイルエラーになる。タスク指示の明示的許可（「削除して build が通らないなら一貫性のため ecs/mod.rs:41,48 の該当再エクスポート行の削除も本セルの簡素化に含めてよい」）に基づき、当該2シンボルの再エクスポート行のみ削除した（`ecs/mod.rs` のその他の pointer 再エクスポート・他モジュール再エクスポートは未変更）。

## 適用しなかった候補と理由（churn 回避）

1. **`transfer_buffers_to_world` のボタン down/up 転送 match ブロック重複の統合 → P58 へ記録**: `buffers.rs:168-183`（down→各ボタン `=true`）と `buffers.rs:185-201`（up→各ボタン `=false`）のほぼ同形 match を1ブロックへ統合する候補。挙動非破壊の見込みだが、(a)「down も up もない場合は代入しない（既存状態を維持＝エッジ検出）」セマンティクスの厳密保持が必要で naive な常時 bool 代入は退行を生む罠がある、(b) 本番クリティカルな入力反映経路の制御フロー構造変更、のため churn 回避と本番経路保護の観点で適用せず P58 へ。
2. **`nchittest_cache.rs:60` の collapsible_if（clippy 1件）**: `if let Some(entry) { if entry.screen_point == screen_point { ... } }` の入れ子。挙動非破壊で修正可能だが、(a) 本セルのスコープは「イベント収集・配信ロジック（dispatch/・buffers・systems・types）」であり nchittest_cache はヒットテストキャッシュで対象集合外、(b) 私の変更が導入したものではない既存 lint、のため scope 規律と churn 回避で未適用（記録のみ）。
3. **その他の `_mouse_` 非推奨 `pub` エイリアス群の一括削除（`MouseButton`/`MouseState`/`MouseLeave`/`WindowMouseTracking`/`clear_transient_mouse_state`/`debug_mouse_leave`/`debug_mouse_state_changes` と `mouse` モジュール）**: grep の結果これらもワークスペース内利用ゼロ（定義＋再エクスポートのみ）。ただし**生きている関数/型への非推奨互換エイリアス**であり、廃止移行の残骸である `process_*`（基底関数自体が `world/mod.rs` で廃止明記）とは性質が異なる。`pub` API 削除の影響判断は `process_*` より慎重を要し、タスク指示「pub API 削除の影響に少しでも疑義があれば削除せず proposals に記録」に従い、本セルでは適用せず（広域な互換 API 整理は別スコープ）。**本セルの提案化は見送り**（P57 が既にポインター系デッド/レガシー API 整理の文脈を保持しており、広域 deprecated 別名掃除は当該文脈の延長で別途検討するのが適切と判断。新規 P 採番は P58 のロジック整理1件に限定し churn を抑制）。
4. **`debug_pointer_state_changes` の Added/Changed 重複ログ既知限界（systems.rs のコメント）**: デバッグ専用・World 非変更・観測価値低。W6a-T と同様、現行コメントの事実確認に留め未着手。

## proposals へ回した候補

- **P58**（新規）: `transfer_buffers_to_world` のボタン down/up 転送 match ブロック重複の DRY 整理（ロジック変更を要する簡素化。エッジ検出セマンティクス維持が要点のため本ループ見送り）。
- **P57**（既存・本セルで解消）: `process_pointer_buffers`/`process_mouse_buffers` の削除を本セルで実施。proposals.md の P57 に「W6a-S で候補(1)を実施・解消済み」の解決状況を追記。

## clippy（S3・記録のみ・非ブロッカー）

- BEFORE/AFTER とも `cargo clippy -p wintf --lib` の総数は **130 warnings + 20 deny-level error**（数は変化なし）。本セルの削除は純粋に減算的（デッドコード除去）で、新規 warning/error の導入はゼロ。
- ポインターモジュール（`ecs/pointer/`）配下を参照する clippy 診断は **`nchittest_cache.rs:60` の collapsible_if 1件のみ**（本セルで触れていない既存 lint。上記「適用しなかった候補」2参照）。私が変更した `systems.rs`・`buffers.rs`・`pointer/mod.rs`・`ecs/mod.rs` を参照する clippy 警告は AFTER で**ゼロ**（削除により `systems.rs` 由来の潜在 lint 面積はむしろ縮小）。解消した lint・新規 lint ともなし（純減）。

## verification (S2)

- BEFORE: 親検証済みベースライン（1516 passed / 0 failed・クリーンワークツリー）を信頼し省略（design フェーズ0 + 親指示「BEFORE S2 は省略可」）。開始時に `cargo test -p wintf --lib pointer::` で **56 passed / 0 failed** を確認（反復検証用ベースライン）。
- AFTER:
  - `cargo build --workspace` **成功**（`areka` 本体・`wintf` 含む全クレート。削除した `pub` シンボルがビルドを壊さない＝本番未使用の追加実証）。
  - `cargo test --workspace` **1514 passed / 0 failed**（全 `test result` 行を awk 合算: passed=1514 / failed=0 / ignored=32）。
  - **増減内訳**: 1516 → 1514（**−2**）。減少分は削除した `process_pointer_buffers` の特性化テスト2件（`test_process_pointer_buffers_button_down_priority`・`test_process_pointer_buffers_applies_position_wheel_doubleclick`）のみ。それ以外の既存テストの増減なし。
  - wintf lib バイナリ: AFTER **309 passed / 0 failed**（W6a-T の 311 − 2）。
  - 反復検証: `cargo test -p wintf --lib pointer::` で **54 passed / 0 failed**（56 − 2）。内訳 `pointer::types` 22・`pointer::buffers` 9・`pointer::systems` 3（5 − 2）・`pointer::nchittest_cache` 6・`pointer::dispatch` 14。
  - **本番経路の回帰検知器の残存確認**: `transfer_buffers_to_world` を検証する buffers.rs の9テスト（位置/速度・ボタンエッジ検出/reset・全5ボタン写像・修飾キー転送・PointerState 不在スキップ + ヘルパ4種）が全て GREEN（本番ポインター入力反映経路は無傷）。
  - ビルド警告: 削除に伴う unused import / never-used / unreachable 警告の新規導入なし（`cargo test -p wintf --lib` の出力で確認）。

## flaky

- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue`（W6a 境界外 `tests/ecs`）: `cargo test --workspace` の全量実行で `tests/ecs` バイナリは **79 passed / 0 failed** と合格（隔離再実行不要）。本セルの削除とは無関係。

## 自己レビュー結論

- 削除はすべて本番呼び出しゼロを grep で実証したデッド/レガシーコードに限定（`process_pointer_buffers`/`process_mouse_buffers`/`push_mouse_sample`）。観測可能な挙動変更なし（R5.1/R5.3）。
- 本番ポインター入力経路（`transfer_buffers_to_world` + その特性化9件）は無傷で残存し回帰検知器として機能。
- S2 全量 AFTER = 1514 passed / 0 failed（予測どおりの −2 内訳を実測で確認、数字は推測なし）。
- 境界拡張（`ecs/mod.rs:41,48`）はタスク明示許可の範囲内・ビルド整合性のため最小限。
- ロジック変更を要する候補（P58）・広域 deprecated 別名掃除・scope 外 lint は churn 回避と scope 規律により適用せず記録/見送り。
