# 設計バリデーションレポート — areka-P0-emo-text-viewbox

> 実施日: 2026-07-12
> 対象: `.kiro/specs/areka-P0-emo-text-viewbox/design.md`（requirements.md 確定済み・research.md §8 決定記録／§9 design discovery 併読）
> レビュー基準: `.claude/skills/kiro-validate-design/rules/design-review.md`（Analysis → Critical Issues → Strengths → GO/NO-GO）
> 実施形態: 非対話（レポートを本ファイルへ永続化・ディスカッションは kiro-design-discussion が別途実施）

---

## Review Summary

要件ディスカッション確定の「ダーティ矩形スクロール（`ScrollDC`/`WM_PAINT` 型・固定 validrect 面＋whole-pixel 面内 blit）」を、emo-text-layer が納品した分離シームの上に正確に写した高品質な設計である。実シンボル突合（`visible_window` layout.rs:255・`ContentCanvas::from_layout` canvas.rs:217・`DrawExecutor::render` draw.rs:523・呼び順 actor.rs:448–457・`line_cache`→`LineLayoutStore` 抽出・`TextSurface` 単一 `source_tex`→2 枚化）を本レビューで独立に再確認し、全て一致した。DD1–DD12 は全件確定・RN1–RN5 は全件処置済み・全 10 要件が Traceability 表で構成要素へ落ちており、純粋層無改変（lib.rs 構造檻の拡張つき）・新規依存ゼロ・log-first/thiserror/UI スレッド規律の steering 整合も取れている。実装可能な状態にあり、下記 2 点はいずれも設計の破棄を要さず、タスク順序と小さな追補で吸収できる。

## Critical Issues（≤3）

### 🔴 Critical Issue 1: byte 等価の唯一の耐荷重仮定（blit＝ラスタ平行移動不変）をタスク先頭で spike 検証すべき

- **Concern**: k=1.0 byte 一致（R6.1）と再描画レス（R3.1）が両立する前提は「整数平行移動の blit 結果 ≡ 新位置で `DrawTextLayout` し直した結果」という DirectWrite/D2D の AA・ClearType 位相不変仮定ただ一点に載っている。design はこれをリスク登録し（viewbox_draw Risks・§9.5）、live-diff を第一檻・全ダーティ縮退を fallback とするが、**縮退した世界では R3（確定 content を再描画しない）の受け入れ基準そのものが満たせない**＝仮定が破れた場合は性能劣化でなく本ユニットの前提崩壊であり、「正しさは保てる」の記述はこの点でやや楽観的。
- **Impact**: 仮定検証が実装後期（live-diff 檻の完成後）に回ると、破れが判明した時点で ScrollPlanner／ダーティ導出の実装済み資産が要件を満たせない構成に化ける（手戻り最大）。
- **Suggestion**: tasks 生成時に「最小 spike＝同一 format/TextLayout で『位置 A に描いて blit で B へ』vs『最初から B に描く』の readback byte 比較（横/縦・数行・k=1.0）」を**タスク 1 相当の先頭**へ置き、GO を確認してから ScrollPlanner 本実装へ進む順序を明記する。ガード余白（`DIRTY_GUARD_IMG_PX`）の実効値もこの spike で早期に確定できる。
- **Traceability**: R3.1/R3.4・R6.1–6.3（両立要件）
- **Evidence**: design.md「ViewboxExecutor — Implementation Notes / Risks」「§9.5 リスク登録」「Testing Strategy Integration #1」

### 🔴 Critical Issue 2: R10.3 の DrawStats を example から読む口（runtime レベルのアクセサ）が未規定

- **Concern**: design は `ViewboxExecutor::stats()`（pub）と example の追加 checkpoint（DrawStats 檻）を定めるが、executor は `TextLayerRuntime` 内部の `ActorRender` に抱えられており、example から actor 別の stats へ届く**runtime レベルの読み口が Components/Interfaces に現れない**。既存 example は `rt.surface(actor)` で readback へ届いており（examples/emo-text-layer.rs:908 実測）、同型のアクセサが 1 本要る。
- **Impact**: 未規定のまま実装に入ると、実装者が場当たりで公開面を増やす（あるいは R10.3 checkpoint がテスト側にしか作れない）——R9.2 の「消費経路の非再定義」との線引きが実装時判断に漏れる。
- **Suggestion**: `TextLayerRuntime::draw_stats(actor: &ActorKey) -> Option<DrawStats>`（`surface(actor)` と同型・additive・R9.2 非抵触）を actor.rs 結線層の契約へ 1 行追補する。設計本文の変更は最小（Interfaces 表と actor 結線の Responsibilities に 1 項目）。
- **Traceability**: R3.5・R10.3（決定論観測を example で成立させる要件）
- **Evidence**: design.md「観測 example」「actor.rs の結線差し替え」（stats 読み口の欠落）／examples/emo-text-layer.rs:908（`rt.surface(actor)` の既存精度）

（Critical Issue は以上 2 件。3 件目に相当する重大欠陥は検出しなかった。）

## Design Strengths

1. **plan/commit 二相＋back 全被覆不変条件による決定論設計**: 純粋な `ScrollPlanner`（windows 非依存・lib.rs 構造檻へ編入）が blit 量・ダーティ導出・量子化（真位置直接丸め＝累積ドリフトの構造的排除・`|committed−pos|≤0.5` 恒真檻）を headless unit テスト可能な形に閉じ、COM 失敗フレームは未 commit 再試行——記憶 deterministic-test-coverage-mandate への模範的応答であり、残像漏れ（blit 写域 ∪ dirty＝面全域）まで不変条件として檻化している。
2. **オラクル保全と経路共有による等価証明の構造化**: 再描画方式 `DrawExecutor` を無改変で `#[cfg(test)]` 独立オラクル化し、`LineLayoutStore` 抽出・format/D2D DC 生成経路・origin 式・`SetTransform(scale(k))` 一点則を両 executor で完全共有（RN5 解決）——「同一プロセス live-diff・k=1.0 byte 一致」を偶然でなく構造で成立させる設計になっている。既存テスト 5 本無改変 green を R2.5/R9 の主担保に据える判断も的確。

## Final Assessment

**Decision: GO**

**Rationale**: 既存アーキテクチャ（分離シーム・純粋層檻・log-first・スケール一点適用・donor 装着契約）との整合が実シンボルレベルで検証済みであり、全要件のトレーサビリティ・エラー戦略・テスト戦略（unit 5／integration 4／E2E＋手動 DoD 申し送り）が実装着手に十分な密度で確定している。Issue 1 は設計変更でなくタスク順序（spike 先頭化）、Issue 2 は additive なアクセサ 1 本の追補であり、いずれも GO を妨げない。

**Next Steps**:
1. kiro-design-discussion で Issue 1（spike 先頭のタスク順序方針）と Issue 2（`draw_stats` アクセサ追補）を確認・design.md へ最小反映。
2. `/kiro-spec-tasks areka-P0-emo-text-viewbox` でタスク生成（Issue 1 の spike をタスク 1 に据える）。
3. 実装フェーズの DoD に非 96 DPI 手動確認（R10.6・design 明記済み）を申し送りとして維持。
