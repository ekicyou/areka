# Design Validation Report: areka-P0-sylphya

> 実施日: 2026-07-23／対象: design.md（FINALIZED）×requirements.md×research.md（§8-§12 裁定）×steering×本 worktree 実コード再検証
> 検証者注: 設計中の実コード主張（provider 差替 3 箇所・`ALLOWED_EVENT_IDS` 8 ID・`shiori_host.rs:74` HashMap ストア・toml 未 hoist・sakura.name2 未読取・areka-actor 公開面）を本レビューで独立に grep/Read 再確認し、**すべて実測一致**を確認済み。

## Review Summary

要件討議裁定（§8-§10）を拘束として忠実に具体化した、実装準備度の高い設計である。掲示板モデル＋単一同期アクターは steering 並行モデル・kanade 3 不変量・R7 消費側契約をすべて保存し、依存方向は機械検証可能な形で宣言され、全 34 AC が Traceability 表と Testing Strategy の檻に写像されている。以下の 3 点は実装を止める欠陥ではないが、タスク生成前に明確化すべき残留曖昧点・過小見積である。

## Critical Issues

🔴 **Critical Issue 1**: `PublishShiori` の asker と鏡像フラット区画の写像が未規定
**Concern**: `SylphyaMsg::PublishShiori { asker, .. }` は AskerId を第一級で運ぶが、`MirrorImage` の区画は `flat_global`／`dotted_global`／`dotted_per_asker` のみで、フラット名（username）の per-asker 区画が存在しない。username が flat_global へ落ちるのか（asker は記録のみか）が設計文面から確定しない。
**Impact**: 実装者が M1 で恣意的に解釈すると、R2.6 の「API 形は問い合わせ元第一級」と鏡像モデルの不一致が M2 多重ゴースト時の鏡像再構築（Revalidation Trigger 級の型変更）として跳ね返る。
**Suggestion**: tasks で「M1 規則: フラット区画は global・PublishShiori の asker はログ証跡と将来シームのみに使用（または flat_per_asker 区画を最初から切る）」のいずれかを 1 行で確定し、asker 分岐檻（Unit Test 3）へ含める。
**Traceability**: R2.6, R4.1
**Evidence**: design.md「鏡像＋SylphyaReader」State Management／「sylphya アクター」Service Interface

🔴 **Critical Issue 2**: `default_system_vars()` 退役＋`SystemVarWiring` 型変更の編集面が Modified Files で過小列挙
**Concern**: Modified Files は本番 3 箇所＋runtime.rs のみを挙げるが、実測では in-crate テスト（runtime.rs 5 箇所・dispatcher.rs 5 箇所）と tests/ghost 統合テスト（spine_e2e_test 5・inproc_e2e_test 2・snapshot_capture_test・real_pasta_test ほか）計約 20 箇所が `default_system_vars()`／`system_vars:` 構築子に依存しており、退役＋enum 置換で全箇所がコンパイル不能になる。
**Impact**: DoD ゲート（`cargo test --workspace` green）に直結する編集量がタスク見積から漏れ、実装中の場当たり修正（陳腐化テスト方針の逸脱）を誘発する。
**Suggestion**: tasks 生成時に「テスト呼出面の `Custom` 注入への一括更新」を独立タスクとして全箇所列挙する（設計変更は不要——「テストは Custom で注入」の方針自体は design に明記済み）。
**Traceability**: R7.1, R9.1
**Evidence**: design.md「File Structure Plan > Modified Files」vs 本レビュー grep 実測

🔴 **Critical Issue 3**: `set_property_value` の即時可視性喪失が本番呼出面へ波及しないことの根拠が弱い
**Concern**: sink 統合で Set 系は投函→有界ラグとなる（裁定準拠・意図的変更）。design は「呼出面 shiori_session/reference_brain/e2e は無改変」「現行消費者に該当依存なし（research §1.5）」とするが、§1.5 は使用面の列挙であり「set→直後 Get の順序依存が無い」ことの実測確認ではない。shiori_session 初期化列が set 直後の SHIORI 照会（GetProperty）を前提にしていれば、非決定な取りこぼしが発生する。
**Impact**: 既存 e2e／native 脳デモの間欠的失敗（フレーク）として顕在化し、決定論テスト必達規律に抵触する。
**Suggestion**: tasks で「本番呼出列（shiori_session 系）の set→get 順序依存の実測確認」を統合タスクの検証項目に加え、依存が見つかった場合は充填ラッパ内 barrier（既に公開予定）を初期化列に限り適用する。
**Traceability**: R7.2, R9.1
**Evidence**: design.md「bin（ShioriHostSink 統合）」Risks／research.md §1.5・§12-4

## Design Strengths

1. **裁定拘束の忠実な具体化と不変量保存**: pull 棄却→掲示板モデル、アクター 1 本、TOML 確定、既定値唯一定義点の残置、kanade 3 不変量（ID 檻・&'static str・shiori_tx 専有）の無改変増分——要件討議の全裁定が設計判断へ 1:1 で接続され、逸脱がない。依存方向（sylphya 最下層・kanade は closure seam で sylphya 非依存）が「消費者は backing を知らない」（R2.4）を構造的に保証する点は特に堅牢。
2. **検証可能性の設計**: 全 34 AC の Traceability 表、判断分岐の純関数中核（SylphyaCore／derive_flat_statics）への寄せ、Barrier による boot prefetch 順序の決定論化、R9.3 用の固定ログイベント名の設計時確定——「檻に入れるのは判断分岐のみ」「実機サインオフは有界 auto-exit＋ログ grep」の記憶知見・steering 規律に完全整合。

## Final Assessment

**Decision: GO**

**Rationale**: アーキテクチャ上の不整合・要件ギャップ・過剰複雑性はいずれも認められない。3 つの指摘は (1) 1 行の規則確定、(2) タスク列挙の補完、(3) 検証項目の追加であり、すべて design.md 無改変のまま tasks フェーズで吸収可能な残留リスクである。

**Next Steps**:
1. 設計ディスカッションで Critical Issue 1 の M1 規則（flat 区画×asker）を裁定
2. `/kiro-spec-tasks areka-P0-sylphya` で Issue 2 のテスト呼出面更新・Issue 3 の順序依存実測をタスク化
3. 実装は research §7 の段階化順序（crate 中核→照会座席＋sink 統合）を tasks に反映
