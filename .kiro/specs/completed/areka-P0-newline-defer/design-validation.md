# 設計バリデーションレポート: areka-P0-newline-defer

- 実施日: 2026-07-18
- 対象: `.kiro/specs/areka-P0-newline-defer/design.md`（requirements.md / research.md / brief.md / steering 突合せ）
- 実施形態: 非対話（kiro-validate-design REVIEW プロセス: Analysis → Critical Issues → Strengths → GO/NO-GO）
- 検証方針: 設計が既存コードについて行う主張を `crates/areka-emo-text/src/` の実ソースで独立照合（設計の自己申告は鵜呑みにしない）

---

## Review Summary

設計は「`LayoutEngine::layout` 走査ループ内のローカル `pending: Option<f32>`」という最小侵襲の一点改訂に収束しており、要件 8 本（R1–R8）を DD-1〜DD-8 で漏れなく決着し、Requirements Traceability も全 AC を実現要素・檻へ写像している。既存コードに関する主張（走査ループ構造・檻の棚卸し・下流非改変・ログ marker）を実ソースで全数照合した結果、**すべて一致**した。実装準備は十分であり、残る懸念は運用手順（R8 実機 grep）の頑健性と「非影響見込み」檻の残余リスクという軽微な 2 点のみ。

## 対コード検証結果（設計主張の実測照合）

| 設計の主張 | 実ソース照合 | 判定 |
|---|---|---|
| 行送りの本番分岐は `LineBreak` アーム唯一・即時行送り（バグの所在） | `layout.rs:211-224`（`lines.push(finish_line…)` → `block_pos += block_dir * pitch * ratio`）・可視 prefix 打切り `placed == visible_count` は Glyph アームのみ（`layout.rs:184`） | ✅ 一致 |
| `opened` フラグの存在と撤去可能性（DD-4） | `layout.rs:178/187/213/227` に存在。遅延化後は行確定がグリフ配置に隣接するため `!current.is_empty()` 等価の論証は妥当 | ✅ 一致 |
| 更新檻 4 本の棚卸し（DD-7） | `line_break_within_visible_prefix_opens_empty_line`＝layout.rs:714・`trailing_line_break_opens_empty_line`＝layout.rs:750・`trailing_empty_line_participates_in_overflow`＝layout.rs:1019・`empty_lines_are_preserved_as_empty_glyph_residents`＝canvas.rs:468。いずれも即時意味論（trailing 空行・あふれ参加）を檻化しており更新対象で正しい | ✅ 一致 |
| draw.rs 檻は内部改行のみで非影響（DD-7・gap 分析 §4-6 の見込みを否定） | draw.rs の `LineBreak` は 1627/1726/1730 の 3 箇所のみ。全て後続グリフあり＋全可視（1627: `[あい,\n,う]` visible=3・1726/1730: `[■■■,(\n,■)×n]` 全可視）＝遅延化後も同一出力 | ✅ 一致 |
| 回帰檻（緑のまま）は内部改行のみ使用 | `explicit_line_break_ratio_scales_line_feed`（layout.rs:576・`[あ,\n,あ,\n(0.5),あ]`）・`vertical_line_break_feeds_column_axis`（605）・`fractional_ratio_feed_scrolls…`（990）・`broken_lines` ヘルパ（817）・`same_input_yields_identical_output`（1061）——全て改行の直後にグリフあり | ✅ 一致 |
| `viewbox_draw.rs:2074` の「幽霊空行」コメントのみ陳腐化 | 当該コメント現存（2074-2075「幽霊空行（未リビール NewLine による）」）。oracle=viewbox byte 等価檻は両側が同一 layout 出力を消費する構造で意味論非依存の論証も妥当 | ✅ 一致 |
| state.rs 非改変の土台（NewLine 追記・Clear/ClearAll 全消去・reveal はグリフのみ） | state.rs:199-222（`items.push(LineBreak)`・`ActorTextState::default()` 全消去）・改行は reveal 枠を消費しない檻（state.rs:755） | ✅ 一致 |
| DD-8 grep marker の実在 | `あふれ発火`＝layout.rs:292-297（`tracing::debug!`・visible_window 内）・`NewLine cue 適用`＋`ratio` フィールド＝state.rs:200。いずれも target `areka_emo_text` ゆえ `RUST_LOG=info,areka_emo_text=debug` で有効 | ✅ 一致 |

## Critical Issues（≤3・いずれも GO を覆さない軽微〜中程度）

🔴 **Critical Issue 1**: R8 実機 grep の縮退判定に actor 帰属の限界＋ANSI 頑健性が未記載
**Concern**: `あふれ発火` marker（layout.rs:292-297）は `mode`/`first_visible_line`/`total_lines` のみで **actor フィールドを持たず**、かつあふれ状態が続く限り毎フレーム発火する。DD-8 手順 3 の縮退判定（「ratio=1.5 適用後・同一 actor 宛て後続 Text cue が無い区間で `あふれ発火` が無いこと」）は、ログ単体ではどの actor のバルーンの発火かを帰属できない。またプロジェクト既知の落とし穴（ANSI 色コードで event 直結 regex が空振り＝実機サインオフ定石メモリに記録済み）への言及がない。
**Impact**: 正当なあふれ（長文トーク）が同一実走に混入した場合、縮退判定が機械的に決定できず再実行の判断が恣意化する。grep パターン設計を誤ると偽陰性（空振り）で手順が空転する。
**Suggestion**: tasks フェーズで (a) 縮退判定は「主判定＝全体 0 件、混入時は再実行または人間目視＋R7 決定論檻を最終根拠とする」へ簡素化を明記、(b) grep は ANSI 耐性のある素朴トークン分割（`あふれ発火` と `ratio=1.5` を別 grep）で行うことを手順に焼き込む。本番コードへの actor フィールド追加は不要（visible_window は actor を知らない純関数であり、決定論担保は R7 檻が担う設計自体は正しい）。
**Traceability**: R8.1 / R8.3
**Evidence**: design.md「設計決定 DD-8」「Testing Strategy > 実機サインオフ」手順 3

🔴 **Critical Issue 2**: viewbox_draw live-diff 檻の「アサーション非影響」は檻構造からの推定であり、シナリオは幽霊空行由来のスクロール発火を意図的に含む
**Concern**: DD-7 は viewbox_draw の檻を「oracle=viewbox byte 等価＝意味論非依存で非影響」と分類するが、live-diff 5 チェックポイント（あふれ前→スクロール発火直後→…）のシナリオ（viewbox_draw.rs:1546-1552/1827-1835 ほか）は `NewLine`→`Text` の at 分散により**幽霊空行の発生タイミングを実機一致させる**ことを設計意図としている（2074-2075 コメントが明記）。遅延化後は幽霊空行が消えスクロール発火時刻が後ろへずれるため、byte 等価アサーション自体は両側同時に変化して緑のままでも、チェックポイントの状態前提（発火「直後」の切り取り）が変質する。
**Impact**: 全スイート再走で予期せず落ちた場合の切り分けコストと、「緑のまま通ったがチェックポイントが意図した状態を検証しなくなる」静かな検証力低下の両リスク。
**Suggestion**: 設計は既に「実変更後に全スイートを回し落ちた檻を個別判定」とヘッジ済みで方針として十分。tasks で「viewbox_draw live-diff 系は挙動シフト（発火時刻後退）が**予期される**檻」と明記し、落ちた場合の判定基準（意味更新 vs 陳腐化）とチェックポイントコメントの整合更新を作業項目化すること。
**Traceability**: R7.2 / R3.2
**Evidence**: design.md「設計決定 DD-7」「Testing Strategy > 更新檻 > 非影響の確認」

（3 件目なし——上記以外に成功を左右する懸念は実測照合で検出されなかった）

## Design Strengths

1. **檻棚卸しが実測で正確（DD-7）**: 設計が挙げた更新 4 檻・非影響檻・回帰檻・ログ marker・コメント位置を本レビューで全数独立照合し、file:line・檻の中身・「内部改行のみ」の分類まで**一件の齟齬もなく一致**した。gap 分析の「draw.rs 檻更新見込み」を実測で否定した点も含め、実装フェーズで手戻りを生まない精度に達している。
2. **構造で正しさを成立させる最小設計（Option A＋DD-3）**: 新規型・新規状態・新規ファイル・フレーム跨ぎ状態ゼロのまま、「layout が空行を出力しなければ下流（visible_window/canvas/draw/viewbox）は本番非改変で自動的に正しい」ことを実データフロー（行列入力・1:1 写像・first_visible_line 消費のみ）に立脚して論証している。R4.2 の最大リスク（break→flush 順序）を順序契約＋専用檻として明文固定した点、蒸発（R5）を「無操作で成立」させた点は steering の「檻対象＝判断分岐のみ」「実装第一」に完全整合する。

## Final Assessment

**Decision: GO**

**Rationale**: 要件 8 本の全 AC が設計要素・檻へ追跡可能に写像され、既存コードに関する設計主張は実ソース照合で全一致。変更は純粋層 1 関数に閉じ、失敗リスクの最大点（順序契約）にも専用檻が計画済み。指摘 2 件はいずれも運用手順の頑健化・tasks への注記で吸収可能であり、設計の再作成を要しない。

**Next Steps**:
1. 設計ディスカッションで Critical Issues 1・2 の tasks への焼き込み方針を確認（design.md 本文の改訂は必須ではない——手順・作業項目レベルの補強で足る）。
2. `/kiro-spec-tasks areka-P0-newline-defer` でタスク生成へ進む（DD-3 順序契約・DD-7 棚卸し・DD-8 手順の 3 点をタスク本文へ転写すること）。
