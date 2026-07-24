# Design Validation Report: wintf-gpu-test-crash

- 実施日: 2026-07-24
- 対象: `.kiro/specs/wintf-gpu-test-crash/design.md`（requirements.md / research.md / brief.md / steering 突合）
- レビュー様式: kiro-validate-design（Analysis → Critical Issues → Strengths → GO/NO-GO）

## Review Summary

根因未確定という制約を、調査 3 フェーズ→判定ゲート G1→条件付き是正の phase-gate 構造として設計に昇格させており、要件構造（R3 の証拠確定が R4 の宣言・経路選択の前提）と 1:1 で整合している。コード上の主張（`wuc_resource.rs` の宣言順 drop・`com/wuc.rs:126-135` の `DQTYPE_THREAD_CURRENT` 束縛・`wuc_spike.rs` のドレイン実証パターン・`structure.md` のテスト入口規約）は実コードと突合して正確であることを確認した。トレーサビリティ表は全 AC（1.1–5.3）を覆っており、実装準備度は高い。以下 3 点は GO を覆さない改善事項として設計ディスカッションへ送る。

## Critical Issues（≤3）

🔴 **Critical Issue 1**: Path B 採用時の回帰檻 teardown 手段が経路条件と食い違う
**Concern**: C7 檻は「明示検証には `shutdown_blocking` を使用」とするが、`shutdown_blocking` と `com/wuc.rs` のドレインヘルパは File Structure Plan 上 **Path A 採用時のみ**の成果物。宣言 (b) → Path B 単独採用の世界線では、檻（fixture を意図的にバイパスし素の生成→teardown→再生成を行う）が依拠する明示ドレイン手段の置き場所が未定義になる。
**Impact**: G1 が (b) に倒れた場合に R5 の檻実装が宙に浮き、実装フェーズで場当たり判断（檻内へのドレインコードのインライン複製等）が発生する。
**Suggestion**: ドレインヘルパ（`drain_dispatcher_queue` 相当）を「無条件成果物」へ昇格するか、「Path B 採用時は檻テストファイル内に `wuc_spike.rs` 同型のドレインループを自己完結で持つ」と 1 行明記する。
**Traceability**: 5.1–5.3, 4.3
**Evidence**: design.md「File Structure Plan」（Path A/B 条件分岐）・「C7: RegressionCage」

🔴 **Critical Issue 2**: 未実測バイナリの修正前ベースライン実測が調査フェーズに無い
**Concern**: 検証マトリクス行 3–7（wintf visual / wintf lib / areka-emo-text 系 / areka-emo-present）は「未実測」のまま Phase 4（是正適用後）で初めて実測する計画。research.md 自身が「未実測バイナリが現状緑なら『なぜ graphics だけ落ちるか』の差分自体が根因ヒント」と指摘しているにもかかわらず、C1 調査プロトコル（Phase 0–3）に修正前ベースライン取得が含まれていない。
**Impact**: 是正適用後の実測では「元から緑だったのか・修正の波及で緑化したのか」が区別できず、根本原因記録（3.2）の証拠価値と切り分け材料を不可逆に失う。
**Suggestion**: Phase 0（環境記録）に「多重 WUC バイナリの修正前ベースライン実測（最低限 `draw_readback_test` と wintf visual）」を追加し、結果を根本原因記録へ記載する。
**Traceability**: 2.3, 3.2
**Evidence**: design.md「C6: VerificationMatrix」（既知状態=未実測）・「C1: InvestigationProtocol」Phase 0–3 定義、research.md §4（差分要因の指摘）

🔴 **Critical Issue 3**: G1 判定表に証拠が非定型・混在の場合の既定則が無い
**Concern**: G1 判定表は典型パターン（teardown 残骸／テスト構造固有／実験 (c) 成立等）を列挙するが、cdb スタックがドライバ DLL 内部等で「teardown 由来ともテスト構造固有とも分類しきれない」場合や、bisect と cdb の示唆が食い違う場合の既定宣言が未定義。
**Impact**: 設計が「実装時の裁量を経路選択の証拠判定のみに限定」と宣言しているにもかかわらず、判定不能ケースでまさにその裁量が再侵入し、宣言 (a)/(b) の恣意的選択（4.1 の形骸化）を許す。
**Suggestion**: 判定表末尾に「分類不能・証拠競合時は保守側（宣言 (a)＝本番実在リスク扱い・Path A）を既定とする（または追加切り分け実験を義務化）」の 1 行を追加する。
**Traceability**: 4.1, 3.3
**Evidence**: design.md「System Flows › 判定ゲート G1 — 入力・判定・出力」判定表

## Design Strengths

1. **要件構造への 1:1 適合と誠実なエスカレーション設計**: 即時 Path B/C を要件違反として明示棄却し、全緩和不成立時は「設計内で糊塗せず R5 充足不能を STOP 報告」とするエスカレーション条項を G1 に組み込んでいる。root-cause-first の開発者裁定と steering（無音失敗禁止・あるべき姿検討）に忠実。
2. **両経路の事前設計が実コード検証済みアンカーに立脚**: Path A は `wuc_spike.rs:162-191` の実証済みドレインパターンの本体昇格、Path B は `DQTYPE_THREAD_CURRENT` スレッド束縛の実測確認から素朴 `OnceLock` 共有を禁止し B1（専用オーナースレッド＋クロージャ marshal）を導出。多重 WUC バイナリ 7 グループの全数インベントリ（grep 実測）も母集団の取りこぼしを構造的に防いでいる。

## Final Assessment

**Decision: GO**

**Rationale**: アーキテクチャ上の重大な不整合・要件の未カバー・実装経路の不明瞭さはいずれも無く、根因不確定という本 spec 固有のリスクは phase-gate と G1 判定表で構造的に管理されている。上記 3 issue はいずれも設計骨格を変えない局所補強（成果物の帰属 1 件・調査手順への 1 行追加・判定表の既定則 1 行）であり、設計ディスカッション／tasks 生成時に反映可能。

**Next Steps**:
1. 設計ディスカッション（kiro-design-discussion）で上記 3 issue の採否を裁定し、必要なら design.md へ反映
2. `/kiro-spec-tasks wintf-gpu-test-crash` で phase-gate 構造（調査タスク先行・経路別条件付きタスク）を反映したタスク分解を生成
