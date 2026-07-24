# 設計バリデーションレポート: areka-P0-choice-render

> 実施日: 2026-07-23 ／ 対象: design.md（requirements.md 確定済・research.md 追記済）
> 検証手段: 設計・要件・research の全文精査＋実コード突合（state.rs シーム / viewbox.rs `scroll_state` 契約 / canvas.rs `#[non_exhaustive]` / balloon parser cursor.\* 非モデル化を確認済み）

## Review Summary

要件 9 系列すべてが設計要素へ 1:1 でトレースされ、research.md の RN-1〜RN-5（ukadoc 正典確定）と DD-1〜DD-10（設計判断 10 項目の裁定）が設計本文と完全に整合する、実装準備度の高い設計である。純粋層 DAG 配置（`choice.rs` 最下流・循環禁止）、hover を行指紋差分に乗せる ScrollPlanner 無改変のダーティ設計、単一導出による表示/ヒット座標整合の構造保証など、既存アーキテクチャ規律への適合は模範的。指摘は精緻化レベルであり、アーキテクチャ上の不整合は無い。

## Critical Issues

🔴 **Critical Issue 1**: `to_window_physical` の k≠1.0／committed≠0 純関数檻が未列挙
**Concern**: 座標写像式（`(origin + block) × k + committed`）は Supporting References の正本だが、Unit Tests 一覧（choice.rs #2）には `to_window_physical` の k≠1.0・committed≠0 パラメタライズ檻が明示されていない。現行 `ScaleContract` は k=1.0 恒常のため、実機 DPI≠96 サインオフ（8.1）でも k≠1.0 経路は実は行使されない（memory: 「k=1.0 検証は DPI 差を見ていない罠」）。
**Impact**: emo-dpi-scaling（W4）で k が実供給された瞬間に写像式の誤り（×k と +committed の適用順・符号）が顕在化するリスク。Revalidation Trigger 登録済みだが、純関数なら今すぐ GPU 不要で檻に入れられる。
**Suggestion**: tasks フェーズで choice.rs 単体テストに「k≠1.0 × committed≠0 × writing_mode 3 方向」の写像全網羅を明示的に追加する（決定論テスト網羅必達の規律にも合致）。
**Traceability**: 2.2, 3.3, 7.5 ／ **Evidence**: design.md「Supporting References／座標写像式」「Testing Strategy／Unit Tests #2」

🔴 **Critical Issue 2**: `annotate_lines` の挿入点と序数空間の不変条件が暗黙
**Concern**: 既存 present 経路は「layout → visible_window → from_layout」だが、System Flows のシーケンス図は visible_window を省略しており、`annotate_lines` の precondition「グリフ序数が items のグリフ順と 1:1」は部分リビール時（可視グリフ＝prefix のみ layout へ渡る）の序数空間整合（`ChoiceSpan.glyph_range`＝全 items 序数 vs lines 内＝可視 prefix 序数）を明文化していない。
**Impact**: 実装者が annotate を windowed 後の行列へ適用したり、可視 prefix と span 範囲の交差規則を誤ると、部分リビール中のヒット矩形・ハイライトが 1 行ずれる。単体テスト #2 に「部分リビール」があるため檻で捕捉可能だが、設計不変条件として先に固定すべき。
**Suggestion**: tasks で「annotate は from_layout と同一の lines を消費」「リビールは prefix ゆえ span 範囲との交差は `min(range.end, visible_count)` 打ち切り」を choice.rs の Invariants に明記する。
**Traceability**: 3.3, 7.5, 1.2 ／ **Evidence**: design.md「System Flows／選択肢表示」「ChoicePure／Preconditions」

（Critical Issue 3 は無し——`choice_active` のバリア非依存化（DD-6）は R1.3 からの意味変換だが、バリアが sink へ配送されない実測根拠と真実源の分離明記により正当と判断）

## Design Strengths

1. **表示とヒットの単一導出＋提示フレーム同期スナップショット**（DD-5/DD-8）: 3.3/5.2 の「片方だけ古い状態を作らない」を API 規約でなくデータフロー構造で保証し、hover を行指紋（`CommittedLine` additive フィールド）に乗せることで既存 `derive_dirty` 無改変のまま 4.4（差分再描画）を成立させる設計は、既存資産流用と決定論檻の両立として秀逸。
2. **正典確定と縮退規律の完全性**: RN-1〜RN-5 を ukadoc で確定し、縮退表（13 行）が全判断分岐をログ義務・Req 対応付きで正本化。`ResolvedChoiceStyle` 開放 enum＋「塗り色+文字色」正規形への一点写像は「完全語彙＋縮退シーム」規律の模範実装。settled main への行アンカー再突合（2026-07-23）も並走 brief 陳腐化対策として適切。

## Final Assessment

**Decision: GO**

**Rationale**: 全要件がトレース済みで、既存 3 層一方向規律・additive 制約・決定論テスト規律との適合に瑕疵が無い。Critical Issue 2 件はいずれも tasks フェーズのテスト項目追加・不変条件明記で吸収可能な精緻化事項であり、設計の再生成を要しない。

**Next Steps**:
1. 設計ディスカッションで Critical Issue 1/2 の取り込み（tasks への反映方針）を確認
2. `/kiro-spec-tasks areka-P0-choice-render` で実装タスク生成（Issue 1/2 をテスト・不変条件タスクとして織り込む）
