# 設計バリデーションレポート: areka-P0-windowposition-limit

> 実施: 2026-08-14（kiro-validate-design・非対話）。対象: design.md（確定版）・requirements.md（承認済）・research.md §9/§10・steering。
> 検証方法: 設計の file:line 主張をコード実測で抜き取り突合（下記「検証済みアンカー」）。

## Review Summary

「式 1 本・関門 3 点」（純関数 `clamp_rect_to_work_area` ＋ 起動時関門／runtime 単一ライター内関門／ドラッグ解放時補正）は、要件 2.1 の常時不変量を経路個別の規律ではなく構造で保証する設計であり、scg で露見した「新経路の素通し」と同型のリスクを正面から塞いでいる。全 33 の受入基準がトレーサビリティ表で C1〜C12 へ結線され、檻反転インベントリ・COMPAT 記録・atom⇄wpl 台帳再判定トリガまで具体化済み。設計の主要な file:line 主張を抜き取り検証した結果、全点で実コードと一致した——実装可能性は高い。

### 検証済みアンカー（抜き取り）

- `enqueue_window_set_pos`（`follow/window_move.rs:452`・route は観測語彙のみ）— 一致。経路③（`move_window_to:54-61` のバルーン随伴）・経路⑦（`resize_window_keep_position:693-700`）が実際にここへ集約されていることを確認。
- `on_balloon_drag_end`（`drag_follow.rs:581`）・`restore_merged_placements`（`main.rs:596`）— 一致。スナップショット構築（`main.rs:701`）→ 復元マージ（`:727`）の順序ゆえ、起動時関門は `MonitorSnapshot` を確実に受け取れる。`ScopePlacement` は `balloon_size` を保持しており矩形クランプの入力が揃う。
- `route_applies_visibility_guard`（`visibility.rs:178-192`）は現行 9 variant の網羅 match — 「9→10」の追随計画と一致。
- DD6（焼き付けない）の成立条件: spawn は `BalloonFollow.offset` を `p.balloon_offset` から転写する（`spawn.rs:358-361`）——差分再計算ではないため、`apply_balloon_limit` が `balloon_offset` を不変に保てば生値が runtime へ生き残る。設計の不変量宣言と整合。

## Critical Issues

🔴 **Critical Issue 1**: runtime 関門の縮退列挙にバルーン自身の寸法不明ケースが無い
**Concern**: 関門の判定矩形は「`size=None` は対象の現在 `WindowPos` 寸」とするが、窓生成直後は `WindowPos.size` が `None` であり得る（`resize_window_keep_position:681-682` が現に「現寸不明＝窓生成直後」を扱う）。C7 の解決不能列挙（snapshot／GhostWindows／キャラ窓 `WindowPos` 不在）に**バルーン自身の寸不明**が含まれていない。
**Impact**: 列挙外の縮退はログ無し経路（6.3 違反）または未定義挙動になり得る。
**Suggestion**: C7 の Unresolved 列挙へ「対象バルーンの現寸不明」を追加し、warn＋素通し（同型縮退）と明記する。タスク生成時の 1 行追記で足りる。
**Traceability**: 2.2, 6.3　**Evidence**: design.md「C7: runtime 関門」判定矩形・解決不能の節

🔴 **Critical Issue 2**: Unresolved 素通し後の可視化で 2.6 が破れる残余窓
**Concern**: 2.6 は「次に可視となった時点で 2.1 を成立させる」と定めるが、設計の被覆根拠は「関門は可視性へ非依存＝全書込補正」のみ。非表示中の書込が Unresolved（warn＋素通し）で終わり、その後**書込なしに**可視へ遷移した場合、補正契機が無い。
**Impact**: 縮退経路限定の狭い窓だが、2.6 の保証が warn 頼みになる。
**Suggestion**: 設計の残余リスク節へ「Unresolved 素通し＋無書込可視化」を明示登記する（モニタ構成変更と同格の既知制約として）。可視化時の再検査シームを足すなら別途だが、warn 観測可能ゆえ登記のみでも受容可能。
**Traceability**: 2.6, 2.1　**Evidence**: design.md トレーサビリティ表 2.6 行・「Error Handling」表

🔴 **Critical Issue 3**: atom⇄wpl 台帳再判定が設計承認の前提として未消化
**Concern**: 要件 Adjacent expectations は「設計で接触ファイル集合が確定した時点で台帳再判定を仰ぐ」と定め、設計は接触集合（`window_move.rs`・`drag_follow.rs`・`diag.rs`・`visibility.rs`）を確定させた。再判定は本レビュー時点で未実施。
**Impact**: 実装着手後の再判定は wpl⇄atom の rebase コストを増やす（並走 brief 陳腐化の既知パターン）。
**Suggestion**: 設計ディスカッション（本レビューの次工程）で開発者へ台帳再判定（wpl 先着・atom rebase の既定路線確認）を提示し、承認と同時に確定させる。設計文書自体の修正は不要。
**Traceability**: Boundary Context「Adjacent expectations」　**Evidence**: design.md「Revalidation Triggers」・「残余リスク」atom⇄wpl 行

## Design Strengths

1. **不変量の構造化**: 分岐を route ではなく対象窓の `BalloonLimit` Component（データ駆動）に置いたことで、`enqueue_window_set_pos` の「配管」契約と route の観測語彙純粋性を保ったまま、将来の ECS 書込経路まで自動被覆する。旧 A-2 の反対理由を裁定の変化（常時不変量）で正しく再評価しており、判断の系譜が research.md §10 で追跡できる。
2. **回帰境界の型保証**: `WindowPosition` 完全無改変（sibling raw struct）・「`x_num.is_some()` なら常に `Numeric`」の型不変量・`Side` 分岐 bit 同一——5.1/5.2 の回帰要件が規律でなく構造で守られ、檻反転インベントリ（7.3）も 4 件全て file:line で確定済み。

## Final Assessment

**Decision: GO**

**Rationale**: 3 件の指摘はいずれも小規模な明文化・登記・プロセス実行で解消でき、アーキテクチャの根幹（式 1 本・関門 3 点・データ駆動分岐・焼き付けない分離）に欠陥は見つからなかった。設計の実コード突合も全点一致で、実装経路は明確。

**Next Steps**:
1. 設計ディスカッションで Issue 1（縮退列挙 1 行）・Issue 2（残余リスク登記）を design.md へ反映するか裁定する（いずれも軽微・タスク生成時吸収も可）。
2. Issue 3 の atom⇄wpl 台帳再判定を開発者へ提示し確定させる。
3. `/kiro-spec-tasks areka-P0-windowposition-limit` へ進む。
