# 設計バリデーションレポート: areka-P0-log-capture-determinism

- 実施日: 2026-07-23（kiro-validate-design・非対話モード）
- 対象: design.md（phase=design-generated・requirements 承認済み）
- 検証方法: design.md / requirements.md / research.md / brief.md / steering を読了のうえ、設計が依拠する主張を**実コードと一次ソースで実測検証**した

## 検証サマリ

設計品質は極めて高い。根本原因の機構主張（tracing-core の callsite Interest `Never` 焼き付き）は行番号レベルで一次ソースと**全件一致**を実測確認し、二次修正（R7'）の対象ループ・最終表明・ハーネス定石も実コードと**全件一致**した。要件 8 本・全 AC がトレーサビリティ表で設計要素へ写像されており、実装準備完了と判断する。

## 実測検証の結果（本レビューで独立確認した事実）

### 1. tracing 一次ソースの行番号引用 — 全件一致

`c:\rust\cargo\registry\src\index.crates.io-1949cf8c6b5b557f\` 配下を直接読了:

| 設計の主張 | 実測 |
|---|---|
| `tracing-core-0.1.36/callsite.rs:505` = `unwrap_or_else(Interest::never)`（dispatcher 0 個で Never 焼き付き） | ✅ 505 行に原文一致 |
| `callsite.rs:407-421` max-level hint（Registry は hint 無し→TRACE 仮定・dispatcher 0 個時のみ OFF） | ✅ :408 `LevelFilter::OFF` 初期値・:412 `unwrap_or(LevelFilter::TRACE)`・:421 `set_max` 一致 |
| `callsite.rs:484-488` `register_dispatch` が登録時に全 callsite rebuild（DD-1 の治癒根拠） | ✅ :487 `CALLSITES.rebuild_interest(dispatchers)` 一致 |
| `dispatcher.rs:314-319` `set_global_default` の Arc leak（registrar 永久生存） | ✅ `Arc::into_raw(s)`＋コメント「the global default will never be dropped」原文一致 |
| `subscriber.rs:676-678` `NoSubscriber::register_callsite = Interest::never()` | ✅ 一致 |
| `tracing-subscriber-0.3.23/sharded.rs:222-228` bare registry の `register_callsite = Interest::always()`（per-layer filter 無し時）・`:288` `event` no-op | ✅ 一致（keeper の registry() は layer 無し＝`has_per_layer_filters()=false` 経路で always 確定） |
| `tracing-0.1.44/lib.rs:963-966` `tracing::callsite` 再エクスポート（`rebuild_interest_cache` 到達可） | ✅ 一致（`#[doc(hidden)]` だが公開パス・後述の注意点参照） |

結論: **keeper（`OnceLock`＋`set_global_default(registry())`＋leak 常駐）が `callsite.rs:505` を構造的に到達不能にする**という設計の中核主張は、一次ソースで裏付けられた。

### 2. R7' 対象と最終表明の実在 — 全件一致

- `log_capture.rs`（98 行・`#[cfg(test)]`）: 構造・旧 PITFALL 注記（79-83 行 `Arc::try_unwrap`）とも設計記載どおり。`capture()` 先頭 1 行前置で API 不変が成立する構造を確認
- `steady_test.rs:821` / `close_test.rs:170` / `close_test.rs:806`: 同型 `'drive: for i in N..=500`＋64-yield 内側ループを実確認（N=3/2/2 も設計記載と一致）
- `close_test.rs:57` `wait_until`: 100,000 yield 有界・壁時計なしを実確認
- 中間 assert 削除後に復帰意味論を担う**既存**最終表明の実在: steady (2) `resumed_get_after_active_window`（885 行）・close#1 (c) post-close pump ≥1（236-248 行）・close#7 (b) `resumed_get_after_notify`（859 行）——すべて join 後の最終記録列に対する表明として実在
- `common/mod.rs`: `DEFAULT_TIMEOUT`（5 秒・40 行）・`join_bounded`（969 行）・`wait_until_blocked`（561 行）の Instant deadline 定石が実在＝ヘルパー新設の同居先とパターン整合を確認
- 完了バリアの因果連鎖（復帰→GET→Value→quit:true talk→終了→inbox 切断）: 各テストの fixture（`steady_value_indices`・`QuitPolicy::PerTalk([false,true])`）で終了が必然である構造を実コードで確認。close#1 の `close_talk_deadline_ms = u64::MAX` 前提も設計に明記済み

### 3. 要件カバレッジ

R1〜R8 の全 AC（26 項目）がトレーサビリティ表で設計要素（interest-keeper / drive ヘルパー / 3 テスト置換 / wait_until 置換 / 検証手順）へ漏れなく写像されている。討議#1（park-barrier 棄却→意味論的完了バリア）・討議#2（証拠形式の病別分離）の帰結も設計へ忠実に反映されている。

## Critical Issues

**ブロッキングな critical issue なし。** 以下は実装時に留意すべき非ブロッキングの注意点（NO-GO 事由ではない）:

1. **［注意・低］`tracing::callsite::rebuild_interest_cache` は `#[doc(hidden)]` 再エクスポート経由**: 公開パスであることは実測確認済みだが、doc(hidden) 項目は semver 保証の慣行が弱い。バージョンは Cargo.lock 固定（R6.3）かつ Revalidation Triggers がバージョン変動を捕捉するため設計上は許容範囲。DD-2 のとおり本呼出は理論上冗長な保険であり、万一将来のバージョンで消えても keeper 本体（`set_global_default` 登録時 rebuild・callsite.rs:487）だけで根治が成立する点が耐性になっている。
2. **［注意・低］drive ヘルパーの Tick 供給は打ち切り観測を持たないため、旧ループより多くの Tick が inbox へ滞留しうる**: 設計は「滞留 Tick は切断時に破棄」と自己言及済みで、旧ループにも証左(ii)（send Err まで送り続ける）経路が実在した＝新規挙動ではない。ただし close#7 は直前 Tick が now=3,600,000（1h）で drive Tick が now=2,000 からと**時刻が後退する既存構造**を保存するため、実装時に「既存の開始秒を保存」の意図（挙動不変）をヘルパー呼出コメントで明示すると将来の混乱を防げる。

## Strengths

1. **一次ソース検証の徹底**: 根本原因と修正機構の全主張が tracing-core/tracing-subscriber/tracing の実ソース行番号で裏付けられ（research.md §8 で R-1〜R-4 全件 RESOLVED・本レビューで独立再確認済み）、「内部仕様依存」のリスクが Revalidation Triggers として明文化されている。設計の反証可能性が極めて高い。
2. **病の性質に合わせた証拠形式の分離**（討議#2 帰結）: keeper＝確率再現可能→RED→GREEN ストレス、R7'＝飢餓由来で統計再現不能→構造証明＋回帰緑、と検証戦略を病理別に設計しており、「偽の安心を生む人工再現」を明示的に棄却している。steering（deterministic-test-coverage-mandate・areka-log-first-no-silent-failure・Defender 飢餓の既知病）との整合も一貫。

## 判定: GO

**根拠**: 設計の中核主張（Interest 焼き付き機構と keeper による構造的遮断）は一次ソースで全件検証済み、二次修正の対象・置換構造・保存すべき最終表明は実コードで全件確認済み、要件トレーサビリティは完全、スコープ境界（本番不改変・API 不変・新規依存ゼロ）は明確。残る注意点は 2 件とも非ブロッキングで、設計自体が耐性・追跡機構（Revalidation Triggers）を備えている。

**次工程**: `/kiro-spec-tasks areka-P0-log-capture-determinism` で実装タスクを生成する（設計討議で上記注意点 2 件を共有のうえ、必要ならタスクの実装ノートへ転写）。
