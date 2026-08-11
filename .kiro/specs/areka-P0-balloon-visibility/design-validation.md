# 設計バリデーションレポート: areka-P0-balloon-visibility

> 実施日: 2026-08-11 ／ 対象: `design.md`（design-generated）・`requirements.md`（承認済）・`research.md` §8（設計決定記録）
> 手法: design-review.md の REVIEW プロセス（Analysis → Critical Issues → Strengths → GO/NO-GO）。設計が依拠する既存コードの file:line アンカーを現ツリーで実測スポット検証した。

## レビューサマリ

可視性の所有権（`VisibilityOwnership`）という単一機構で Requirement 1.2／1.3／6.2／6.9 を同時に解き、会話終了信号は上流無改変の 4 本目 broadcast sink で観測する——という中核判断は、代替案の棄却理由・既存テストへの波及・後続 spec（budget/atom/cage）との共有ハンク責任まで含めて筋が通っている。設計が主張する既存コードの実態（無条件 ShowSurface、`set_visible` の片肺、`clamped_opacity` の `is_visible` 非参照、`HitTest` 既定 Bounds、per-talk `clone_box`、`CueSink::emit(&mut self)`、7 相構成、sinks 3 本、drain 末尾 reconcile）を 10 点スポット検証し、**全点が実コードと一致**した。要件トレーサビリティは 1.1〜9.8 の全 AC を網羅し、縮退方向が常に「表示保持側」へ揃っている。以下 2 点の明確化を条件付けつつ、実装へ進んでよい品質である。

## 検証済みアンカー（実測）

| 設計の主張 | 実測 | 判定 |
|---|---|---|
| 無条件 ShowSurface `frame/attach.rs:327-336` | `PresentCommand::ShowSurface{surface_id:0,..}` 完全一致 | ✅ |
| `set_visible` は surface entity のみ `mount.rs:193-216` | slot 非接触を確認 | ✅ |
| `apply_show` 末尾の可視化 `show.rs:211-213` | `mount.set_visible(world,true)`＋`visible=true` | ✅ |
| `clamped_opacity` は `is_visible` 非参照 `visual.rs:140-142` | opacity clamp のみ | ✅ |
| `HitTest` 不在＝既定 Bounds `hit_test/mod.rs:91` | doc 明記を確認 | ✅ |
| per-talk `clone_box` `dispatcher.rs:290-294` | prototype は emit を受けない構造 | ✅ |
| `CueSink::emit(&mut self)`・broadcast で担当外 cue（`Wait` 含む）も受領 | `dola/src/cue/sink.rs:25-31` | ✅ |
| sinks 3 本（4 本目追加余地）`emo2_boot/mod.rs:400-404` | `surface/clocked_text/move` の 3 本 | ✅ |
| 7 相構成 `frame.rs:135-165`・drain 末尾 reconcile `drain_resnap.rs:50` | 一致（純移動の前提成立） | ✅ |
| `WindowDragging` 挿入/除去 `dispatch.rs:171-177`/`:260-270` | 一致（行番号は ±2 のブロック境界差のみ） | ✅ |

## Critical Issues（2 件）

🔴 **Critical Issue 1**: タイムアウト deadline の初期確立式がフロー・データモデルに未明記
**Concern**: 判断フロー（S9〜S15）は「期限超過?」を判定するが、`deadline` を最初に Some へ確立する箇所と式が図にも Data Models にも明示されていない。5.3 の再計測は `deadline = now + timeout` と明記される一方、初期確立が `display_end + timeout`（正典起点）か「計測 eligible になった最初のフレームの now + timeout」かが読み手に委ねられている。
**Impact**: 後者で実装すると観測遅延（sink 配送・フレーム粒度）の分だけ正典起点「スクリプトの表示が終わってから」からずれ、9.1 の境界檻（deadline±ε）も式の取り違えごと緑になり得る。
**Suggestion**: Data Models の不変条件へ「初期 `deadline = display_end + timeout_secs`。`now` 基準への再設定は 5.3 の抑止解除エッジのみ」と 1 行明記し、境界檻の表駆動入力を display_end 相対で書く。
**Traceability**: Requirement 4.1・4.2・5.3・9.1
**Evidence**: design.md「System Flows／可視性の判断フロー」S9-S15・「Data Models／BalloonVisibilityState」

🔴 **Critical Issue 2**: 文字層スロットの `Visual` を外部挿入が上書きしない保証が未検証
**Concern**: `TextSurface::attach`（`areka-emo-text/src/surface.rs:249-262`・実測）は slot entity へ `VisualGraphics`＋`Arrangement` を挿入し、コメントで「wintf `Visual` フックの既定値上書き」に言及する。`Visual::default()` は可視（research §8.1）であり、mount が不可視で構築した slot の `Visual` がテキスト装着・再装着のフック経由で可視既定へ戻ると、本仕様が是正対象とする「枠だけ消えて文字が残る」欠陥が別経路で再発しうる。
**Impact**: 計画済みの mount 単体檻は構築時のみ、統合檻は `target_visible`（presenter 状態）を観測するため、**text 装着後の slot entity の `Visual` 実値**はどのテストの視野にも入っておらず盲点になる。
**Suggestion**: 起動シーケンス統合テストへ「`connect_balloon_text` 完了後も slot entity の `Visual.is_visible == false`」の assert を 1 本追加し、実装タスクでフック意味論（既存 `Visual` 存在時に上書きしないこと）を file:line で確認する。
**Traceability**: Requirement 1.2・1.7・3.4
**Evidence**: design.md「VisualMount 両 entity 化」・research.md §8.1「`Visual::default()` は可視」

## Design Strengths

1. **アンカーの正確性と単一機構の切れ味**: 実測スポット検証 10 点が全て一致（上表）。所有権方式（D2）は「確立と可視化の分離」1 つで 1.2／1.3／6.2／6.9 を同時に解き、attach の初回 ShowSurface を「不可視のままの確立」として維持することで既存テスト前提（readback・スロット成立・適用 k）を無傷に保つ——波及最小と要件充足を両立する最良の形。
2. **縮退方向の一貫性**: バリア中の horizon 過小（5.4 抑止が吸収）・中断の起点（占有 horizon＝保持側）・観測不能（非抑止側＝5.5）・信号欠落（表示保持＝4.8）・不可視中の hover 固着（非表示遷移で掃除）と、弱点を列挙した上で全て要件の別条項か明示規則で「安全側」へ倒しており、実機でのみ現れる破綻の余地を系統的に狭めている。

## Final Assessment

**Decision: GO**

**Rationale**: アーキテクチャ整合（UI 層所有・判断は純関数・実行は既存漏斗の消費）・要件充足（全 AC トレース済み）・実装経路の明確さ（File Structure Plan とテスト戦略が 9.1 の列挙と対応）のいずれにも致命的欠陥はない。Critical Issue 2 件はいずれも設計の骨格を変えず、明記 1 行＋assert 1 本で閉じられる精密化であり、実装フェーズで吸収可能なリスクである。

**Next Steps**:
1. 設計ディスカッションで Issue 1（deadline 初期式の明記）・Issue 2（slot Visual の統合 assert 追加）を design.md へ反映するか裁定する。
2. `/kiro-spec-tasks areka-P0-balloon-visibility` でタスク生成へ進む。
