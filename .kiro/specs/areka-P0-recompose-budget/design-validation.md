# 設計バリデーションレポート: areka-P0-recompose-budget

> 実施日: 2026-08-14（design フェーズ・非対話レビュー）
> 対象: `.kiro/specs/areka-P0-recompose-budget/design.md`（requirements.md・research.md・steering を参照）
> レビュー観点: 既存アーキテクチャ整合・設計一貫性・拡張性/保守性・型/インターフェース設計（design-review.md 準拠）

## Review Summary

要件 30 項目（AC 全数）を Traceability 表で 1:1 に写像し、D1〜D10 の各決定に棄却代替と根拠（research §9）を備えた、実装準備度の高い設計である。file:line アンカーはレビュー時に独立スポットチェックした範囲（show.rs :66-101／:240／:215-235・cache.rs :98-147・scale.rs :395/:423・`Changed<AlphaMaskResource>` 依存 0 件の grep）で全て実コードと一致した。承認済み契約（キャッシュ容量 1・原子対・既存 info! 実機サインオフ契約）を不変に保ち、上流変更を additive に限定し、隣接 spec（exact／atom／cage④）の観測域を明示凍結する境界規律も要件・steering と整合している。

## Critical Issues

いずれも NO-GO 級ではない（実装フェーズ・第 1 段較正で吸収可能な補強点）。

🔴 **Critical Issue 1**: 恒等 k（100% 表示）の定常経路が steady-state 檻の対象外
**Concern**: D2 は恒等 k のとき native scratch と表示バッファを swap で交代させるため、表示バッファの先頭ポインタは 2 本が交互になる。Testing Strategy の統合檻 1 は「2 パターン交互 × k≠1」のみを規定しており、swap 方式固有の経路（100% DPI＝一般的な利用条件）がポインタ/計数 assert で固定されない。
**Impact**: 恒等 k 経路のアロケーション回帰（swap 忘れ・コピー化）が檻をすり抜け、R6.1 の「静かな復活防止」が部分的に破れる。
**Suggestion**: 恒等 k 用の檻を「2 ポインタ集合の不変」（swap 交代を許容する形）＋計数増分 0 で追加する。等価檻は既に k 恒等/非恒等の両方を規定済みなので、steady-state 檻だけ同じ 2 水準へ揃えれば足りる。
**Traceability**: R6.1・R3.1
**Evidence**: design.md「Testing Strategy > Integration Tests #1」「Architecture > D2」

🔴 **Critical Issue 2**: 判定式⑴の間隔期待値が単一較正値（172ms）で、多アニメ重畳時の p95 の意味が未定義
**Concern**: `FRAME_INTERVAL_EXPECTED_MS=172` はまばたき定義由来の単一値だが、実走ログのコマ適用間隔は複数アニメーション・talk 再生・複数 target の apply が混在した系列になる。どの窓・どの粒度（target 別／アイドル区間限定）で p95 を取るかが較正値台帳に現れていない。
**Impact**: 重畳 apply による短間隔や別アニメの長周期が混入すると、判定式⑴が偽陽性/偽陰性のどちらにも振れ、スロー再生解消の機械判定（本 spec の合格条件の中核）が不安定になる。
**Suggestion**: perf 行に既にある `target_id`／`surface_id`／`key_hash` で系列を分離し、「アイドル区間・対象 target 限定の p95」を judge-perf.py の較正値（測定窓の定義）として第 1 段ベースライン時に確定・README に登記する。コード変更は不要（スクリプト側の定義追加のみ）。
**Traceability**: R4.2⑴・R4.5
**Evidence**: design.md「Performance & Scalability 合格判定式⑴」「Data Models > 較正値台帳」

## Design Strengths

1. **実測駆動の裏取り規律**: Research Needed 全件を design フェーズの読み取り実測で解決し（`Changed<AlphaMaskResource>` 0 件・catch-up 実文言の brief 誤り訂正・可視性 `pub(crate)` 制約）、その上に D3（Arc 輪番）等の決定を置いている。本レビューの独立検証でもアンカーの不一致は検出されなかった。doc-claims-need-file-line-verification の steering 規律に完全準拠。
2. **境界規律と既存資産の再利用**: 承認済み契約を 1 つも書き換えず（容量 1・原子対・既存 info!）、上流 API は additive のみ、atom/cage④ の観測帯（:215-235／:227-231）を構造凍結。計測資産は signoff-scan.py の exit 規約・自己較正 fixture・「観測ゼロ＝判定不能」の実証済み規律を踏襲し、檻は emo-compose 予算檻アプローチ (A)・CaptureSubscriber 流儀へ相乗りしており、新規発明を最小化している。

## Final Assessment

**Decision: GO**

**Rationale**: アーキテクチャ整合・要件充足（全 AC トレース）・実装経路の明確さのいずれにも致命的欠陥がなく、指摘 2 件はともに檻の水準追加とスクリプト側較正定義の補強であり、実装フェーズ（第 1 段較正・檻実装時）で吸収できる受容可能リスクである。

**Next Steps**:
- 設計ディスカッション（kiro-design-discussion）で上記 2 件の取り込み可否を裁定する（Issue 1 は Testing Strategy への 1 行追補、Issue 2 は較正値台帳への測定窓定義の追補で足りる）
- 裁定後 `/kiro-spec-tasks areka-P0-recompose-budget` で実装タスクを生成する
