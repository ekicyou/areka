# 設計バリデーションレポート（再検証） — areka-P0-cue-playback-duration

> 生成日: 2026-07-14（再検証）／ 言語: ja（spec.json）／ フェーズ: design-generated
> 入力: requirements.md（R1–R11・討議 #2 反映）・design.md（討議 #2 反映）・research.md（§9/§10）・.kiro/steering/
> 手順: kiro-validate-design。4 視点 × 実コード接地の敵対的 workflow（traceability-coverage／internal-consistency／consolidation-readiness／new-claims-grounding）→ synthesis。指摘の doc 整合修正は本レポート時点で適用済み。

## Design Review Summary

設計討議 #2（最終検証）で 5 論点＋cue 再生エンジンの dola 層統合（Topic 3・スコープ拡張 (A)）を経た、大幅改稿後の再検証。**判定 GO**。4 視点いずれも architecture を覆す blocker を検出せず、全 findings は doc 整合・完全性・具体度の `minor`/`nit`（本レポート時点で全解消）。特に:
- **consolidation-readiness が wintf ecs/cue 撤去の安全性を実コードで実証**: 生きた App は storyboard 系 `tick_dola_animators` のみ登録・live talk 経路（areka-ghost dispatcher）は `spawn_talk`＋sink で ecs/cue を一切使わず・`CueQueue`/`dispatch_pending_cue_sheets`/`pop_ready` の外部参照は wintf 自身のテスト/doc 例のみ。「旧世代・未配線」は正確。
- **new-claims-grounding が新機構 5 つを実装可能と実証**: CueSheet 絶対時刻・horizon 完了・ClearAll 全消去・ingress clamp・CueSink 単一化——いずれも実コードに対し feasible、infeasible/blocker なし。

## 適用済み修正（検証指摘 → 解消）

| # | 指摘（severity） | 解消 |
|---|---|---|
| 1 | Requirements Traceability に R11.1–11.6・R1.8 の行が無い（minor×2・複数視点） | 行 1.8／11.1–11.6 を追加 |
| 2 | `is_completed` 述語が 3 通り表記（entry 枯渇 AND horizon／horizon のみ／entry 枯渇でなく）（minor） | 「entries 枯渇 かつ t≥horizon」の連言へ統一（schedule.rs/drive.rs/D6） |
| 3 | horizon 完了が「tick 源は entry 枯渇後も horizon まで tick 継続」を暗黙前提（minor） | 完了 seam に liveness 前提を明記（drive-level tick 檻で固定） |
| 4 | `absolute_start_time` 刻印場所が矛盾（mermaid/compile コメント=compile vs 規範=dispatch）（minor） | compile edge から除去・dispatch step へ・compile コメントを「相対 start_time のみ」へ |
| 5 | 配送エンベロープが `TalkCue` のまま・row 1.1 で sakura 帰属（dola 移設と矛盾）（minor） | `dola::cue::TalkCue`（移設）へ統一・row 1.1/Data Models/Components 見出し更新 |
| 6 | `CuePlayer`（統合の中核）に interface 契約が無い（minor） | seam メソッド（`tick`/`register_sink`/`is_completed`/`stop`/バリア hook）と port 範囲（seam+Choice+horizon のみ・pause/resume drop）を明記 |
| 7 | CueSheet tuple→named struct の serde 形変更が未言及（minor）／clamp を horizon と envelope 両方へ用いる点が暗黙（nit） | sheet.rs bullet に注記 |
| 8 | requirements R6 見出し/Objective/Boundary が「Clear」のまま（nit） | 「ClearAll」へ統一 |

## アーキテクチャ健全性（再確認）

| 項目 | 判定 | 根拠 |
|---|---|---|
| wintf ecs/cue 撤去は安全 | ✅ | live App 未登録・areka-ghost は spawn_talk 経路・外部参照はテスト/doc のみ（consolidation-readiness 実証） |
| 新機構 5 つ実装可能 | ✅ | CueSheet 絶対時刻/horizon 完了/ClearAll 全消去/ingress clamp/CueSink 単一化 全て feasible（new-claims 実証） |
| 内部整合（統合後） | ✅ | 討議 #2 target model が全節で一貫・before 記述は「Existing Architecture Analysis／現状」で明示区別（internal-consistency 実証） |
| 要件網羅・決定論テスト | ✅ | R1–R11 全 AC が具体 component へ写像・決定論檻と実機サインオフ（R8）を峻別（traceability 実証） |

## Design Strengths

1. **統合の二枝が「移植元」を持つ**: `CuePlayer` = wintf `CueQueue` 状態機械（バリア seam/Choice/state）＋ sakura `drive.on_tick` fan-out の再結合＝green-field でなく既存テスト済みロジックの再配置。storyboard runtime とは別モデルの兄弟として cue モジュールへ置く配置も低リスク。
2. **3 topic が 1 決定で溶ける**: dola 統合（Topic 3）により Topic 1（horizon 完了＝dola runtime 本来責務）・Topic 2（二重ログ/2 sink trait＝CueSink 1 本で消滅）・Topic 3（変換二重化/wintf duration 欠落＝変換 1 本＋撤去で消滅）が同時解消。単一責務・車輪の再発明の根絶。

## Final Assessment

**判定: GO**

**Rationale**: 大幅改稿後の設計は 4 視点の実コード接地検証を通過し、architecture を覆す blocker はゼロ。wintf 撤去の安全性・新機構の実装可能性がコードで実証され、指摘された doc 整合・完全性・具体度の綻び（全 minor/nit）は本レポート時点で全解消。実装準備完了。

**Next Steps**:
1. `/kiro-spec-tasks areka-P0-cue-playback-duration` でタスク生成へ進む。
2. 詳細設計初手として dola `runtime`（storyboard 系）と cue runtime の結線を cue モジュール内新設で確定（storyboard runtime へ混ぜない・精読済）。
