# Design Validation Report: areka-P0-input-events

> 実施日: 2026-07-19／レビュー種別: kiro-validate-design（非対話・レポート永続化）
> 対象: design.md（FINALIZED）／requirements.md（Req1〜8・確定済み）／research.md §1〜§7
> 検証方法: design-review.md の 4 基準（アーキテクチャ整合・一貫性・保守性・型/契約）＋実コードスポットチェック

## Review Summary

本設計は「マウス入力→kanade→GET→StartTalk」の背骨を、確立済みの kanade additive パターン＋薄い UI 結線モジュールの Hybrid で実現しており、要件 8 本・全 AC への対応表（Requirements Traceability）が完備している。design 送り事項 10 項目＋追加 2 項目（DD-IE-1〜12）は全て実 SSP wire 捕獲・実 fixture・ukadoc 正典で根拠づけられ、本レビューのコード実測でも行アンカー（hit_region.rs:42/:69・events.rs:48・actor.rs:155/:166・steady.rs:104 の origin 固定・spawn.rs:167/:205/:321・`ShioriCall.id: &'static str`＝DD-IE-3 の転記が自明に健全）が全件一致した。実装準備は十分に整っている。

## Analysis（基準別・要点のみ）

- **既存アーキテクチャ整合**: kanade 三層（純粋状態機械＋アクターシェル＋境界差し替え）への additive 増分・dispatcher の Close-then-spawn 再利用・NonSend self-gating（Emo2Wiring 前例）・actor の drive() が「1 メッセージ＝1 入力＝往復完結」であることを実読し、DD-IE-3（origin 転記）と「GET 発行時フェーズ＝reply 到着時フェーズ」の前提が実装と一致することを確認。
- **一貫性・規約**: log-first（panic 新設なし・正常入力は trace）・`expected_call` 檻共有・events.rs 単一列挙点＋ALLOWED_EVENT_IDS チョークポイントの既存流儀に完全準拠。
- **保守性・拡張性**: OnChoiceSelectEx への背骨再利用（MouseInput の形）・Ref2/Ref6 の increment シーム・Revalidation Triggers 明記・暫定退避の退役条件記録（canonical-not-minimal-lifecycle 整合）。
- **型・契約**: kanade は `HitRegion` を知らず不透明 `Option<String>` で受ける（resolver 契約の再定義なし＝Req1.3）。座標契約（client 物理 px 三者一致・k=1.0）は hit_region.rs:54-56 の実装コメントと一致。

## Critical Issues（2 件・いずれも GO を妨げない討議事項）

🔴 **Critical Issue 1**: 置換調停（DD-IE-2）は実機で構造的に発火せず、DD-6 の意味縮小と対で檻を固定する必要がある
**Concern**: マウス origin の Value-during-talk 置換は、実 pasta が `status=="talking"` で nil 自衛（204 相当）するため実機では発火しない＝mock 檻のみが検証手段。同時に DD-6 が「防御専用」から「OnSecondChange origin 限定の防御」へ意味縮小し、既存檻 `steady_value_during_talk` の更新を伴う。
**Impact**: 自衛しないゴーストが talk 中に Value を返す将来ケースの実挙動が M1 で一度も実機観測されない。檻更新を誤ると idle-talk の防御保証が静かに消える。
**Suggestion**: 実装時、置換檻（mouse origin→置換）と DD-6 保存檻（OnSecondChange origin→warn＋破棄）を同一テスト群で対に配置し、origin の match を wildcard にしない（第 3 の origin 追加時にレビューで気づける形）。E2E-5（talking 付き GET＋pasta 自衛のログ確認）は維持。
**Traceability**: Req 4.3・8.1(c)　**Evidence**: design.md「steady マウスアーム」Risks／research §7.5

🔴 **Critical Issue 2**: SHIORI 応答遅延時のマウス GET 滞留と鮮度低下が実機で未実測
**Concern**: UI 間引きは送出を 10Hz×最大 2 scope に絞るが、actor は同期往復（in-flight ≤1）ゆえ実 32bit helper の往復遅延が 100ms を超えると持続撫で中に kanade inbox へ Mouse が滞留し、処理時点で座標・region が古い GET が続く。design は「無限成長しない」と有界性のみ論証し、遅延蓄積の実機挙動は未検証。
**Impact**: 撫で反応の遅延・古い座標での発火（touch_detect の 2 秒規律には概ね無害だが、体感品質と menu dblclick 応答の即時性に影響し得る）。
**Suggestion**: 実機サインオフ（Testing Strategy E2E）に「持続撫で中の talk 起動遅延・ログ上の GET 送出時刻と応答時刻の差」を観測項目として一行追加する。`MOUSE_MOVE_MIN_INTERVAL_MS` が定数シームとして既にあるため、実測次第で値調整のみで対処可能（構造変更不要）。
**Traceability**: Req 5.1・8.3　**Evidence**: design.md「Performance & Scalability」

## Design Strengths

1. **実測駆動の設計判断**: DD-IE-1〜12 の全てが実 SSP wire 捕獲（shiori-sample.log・OnMouse 系 121 件全 GET）・実 fixture（touch.pasta:19 の空 region 期待・touch_detect.lua の 2 秒規律）・ukadoc 正典で裏付けられ、事前予想（NOTIFY 化案）を実測で覆して確定した過程が research §7 に透明に記録されている。本レビューのスポットチェックでも行アンカーの齟齬ゼロ。
2. **additive の徹底と檻の所在一致**: 新調停・新スレッド流儀・新規依存ゼロ。判断分岐（間引き・Ref 組立・reply 政策）のみを檻化し、間引き檻を判定の所在（areka in-crate）に置く方針は [[test-only-decision-branches-not-proven-wiring]]・[[areka-bin-crate-internal-tests-in-crate]] と完全整合。

## Final Assessment

**Decision: GO**

**Rationale**: アーキテクチャ不整合・要件ギャップ・過剰複雑性のいずれも無し。全 AC がコンポーネント・フロー・檻へ追跡可能で、設計判断は正典・実測で裏付け済み。指摘 2 件はいずれも実装時のテスト配置・実機観測項目の微修正で吸収でき、設計改稿を要しない。

**Next Steps**:
1. 設計ディスカッション（kiro-design-discussion）で上記 2 件の吸収方針を確認
2. `/kiro-spec-tasks areka-P0-input-events` で実装タスク生成（Issue 1/2 の Suggestion をタスク注記へ転記）
3. 実装は mock resolver で並走可・実機サインオフは撫でクラスタ合流（実 resolver・実 DPI≠96）で 1 回実施（Req8.3 確定済み）
