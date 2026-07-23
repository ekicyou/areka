# 設計バリデーションレポート — areka-P0-mayuna-compose

> 実施日: 2026-07-22／対象: design.md（FINALIZED・phase=design-generated）／レビュー種別: 非対話・GO/NO-GO 判定
> 手法: design-review.md の 4 基準（アーキ整合・一貫性・保守性・型/IF 設計）＋HEAD 実ソースへの file:line 実測突合（Grep/Read・読取専用）

## Review Summary

上流 4 層の実測（W1 キャリア・dola 権威表・seriko 消費テンプレート・emo-present キャッシュ）に基づく consumer-side additive 設計として完成度が高い。設計中の load-bearing な file:line 主張を HEAD で全数照合した結果、**全件一致**（下記証跡）。要件 R1–R8 全 38 AC が Traceability に存在し、制約（Rust 2024・新規依存なし・tokio なし・新 CueCommand variant なし・sleep/Instant 不使用の決定論・純関数全網羅）もすべて設計に織り込まれている。残る指摘は severity 規律の precedent 引用ズレ 1 件と per-scope 初期値の意味論注記 1 件のみで、いずれも実装フェーズで吸収可能な軽微事項。

## 検証証跡（HEAD 実測・全件一致）

| 設計の主張 | 実測結果 |
|---|---|
| dola `command_target_of` は `"move"→Window` のみ（sink.rs:99-104）・`cue_target_of(Custom)=None` は委譲（:41-44 doc） | ✅ 一致（`crates/dola/src/cue/sink.rs:54-76, 99-104`） |
| dola 既存檻 `command_target_of_maps_move_and_rejects_unknown_names`（sink_test.rs:250-282）が「bind は未知名」「Some は move のみ」を assert＝D10 の意図 FAIL→意味ある更新 | ✅ 一致（`crates/dola/tests/cue/sink_test.rs:250-282`・unknown リストに `"bind"` 実在・partition assert `vec!["move"]`） |
| sakura compile `GenericCommand`→`command_carrier`（compile.rs:171-178）＝転写側無改変で bind が台本へ載る | ✅ 一致（`crates/areka-sakura/src/compile.rs:171-178`） |
| emo-text は `Custom{..}` を明示列挙の良性 debug skip 済み（state.rs:260-266）＝R2.3 無改変充足 | ✅ 一致（`crates/areka-emo-text/src/state.rs:260-266`） |
| seriko `ScopeStates` の `static_binds` 置き場差替予約（state.rs:44-47）・`apply` の Show は `static_binds.clone()` 固定 | ✅ 一致（`crates/areka-seriko/src/state.rs:44-47, 104-108`）＝D5 の差替（bind 不在時同値）は非退行 |
| `handle_message` の `cue_target_of==None` 枝（Custom/Wait の良性 debug skip）＝D1 挿入点 | ✅ 一致（`crates/areka-seriko/src/actor.rs:212-229`・balloon 早期分岐 :234-263 も同型実在） |
| `spawn_seriko(resolver, static_binds, out)` 現行署名・呼出点 mod.rs:287／spine.rs:444・seriko tests 3 本追随要 | ✅ 一致（actor.rs:157-174・`crates/areka/src/emo2_boot/mod.rs:287`・`spine.rs:444`・tests/{regression,cue_sequence,balloon_face_e2e}.rs 実在） |
| parsers `BindGroupDefaults`（`#[non_exhaustive]`・default のみ・.name 未読取）・`read_bindgroup_defaults` 同一走査拡張点 | ✅ 一致（`crates/areka-parsers/src/package/model.rs:47-54`・`resolve.rs:107-153`） |
| `MoveCueSink` の 2 条件名前ゲート前例（move_cue.rs:467-484） | ✅ 一致（`crates/areka/src/emo2_boot/move_cue.rs:467-490`・所在は areka 側） |
| emo-present `different_binds_on_same_surface_must_miss` 既存＝R6 は test-only で足る | ✅ 一致（`crates/areka-emo-present/src/cache.rs:268`） |
| static_binds 供給源の二重化（`default_bind_ids` shell KV 直読・sakura 限定）を既知負債として登記 | ✅ 一致（`crates/areka/src/emo2_boot/assets.rs:48, 200-211`・kero 除外テスト :462 実在） |

制約適合: 新 crate なし・新規 crates.io 依存なし・tokio なし・`CueCommand` 新 variant なし（権威表 1 行のみ）・テストは注入 Tick＋同期 handler＋log 捕捉（sleep/Instant なし）・名前解決（`resolve_name`/`BindResolver::resolve`）と on/off 積算（`accumulate`）は純関数として GPU 不要全網羅（R5.4）＝steering（test-only-decision-branches／deterministic-test-coverage-mandate／areka-log-first）と整合。R4 の再番号（数値直指定形の削除・DD-6 解決済み）も design/research 双方に正しく反映されている。

## Critical Issues（2 件・いずれも軽微＝GO を妨げない）

🟡 **Issue 1**: D8④「非正準 params＝`debug!`」の precedent 引用が実コードと不整合
**Concern**: 設計は「非正準 params（`as_command_carrier()==None`）→ `debug!` 良性 skip（MoveCueSink 前例）」とし檻で WARN/ERROR=0 を assert するが、実際の `MoveCueSink`（move_cue.rs:467-479）は **Custom なのに非正準 params の場合は `warn!`**（非キャリア＝非 Custom のみ `debug!`）。seriko の bind 分岐 step 1 が扱うのはまさに「Custom で開封失敗」のケースであり、引用された前例の severity は warn である。
**Impact**: 実装者が前例コードを踏襲すると設計の檻（WARN=0）と矛盾し、テスト/実装の齟齬で手戻りする。severity 規律（破損 wire 形は warn 以上）との一貫性も揺らぐ。
**Suggestion**: tasks 生成時にどちらかへ確定する——(a) MoveCueSink 前例に合わせ `warn!`＋檻を WARN=1 へ、または (b) `debug!` を維持し「前例からの意図的逸脱」と設計注記を修正。挙動影響は皆無（いずれも skip）なので 1 行の規律確定で足りる。
**Traceability**: R2.5／D8④・Error Handling 表 4 行目
**Evidence**: design.md「Design Decisions D8」「Error Handling」 vs `crates/areka/src/emo2_boot/move_cue.rs:467-479`

🟡 **Issue 2**: per-scope 初期値の fallback が kero スコープへ sakura defaults を継承する点が暗黙
**Concern**: `current_binds(scope)` は `dynamic_binds` 不在時に単一の `static_binds` へ fallback するが、その供給源（`default_bind_ids`）は **sakura 限定抽出**（assets.rs kero 除外檻あり）。よって scope "1"（kero）の Show には sakura defaults が載る（現行 `apply` の既存挙動の踏襲であり退行ではない）。R3.1 の「per-scope の既定 on/off 初期値」を厳密に読むと kero は kero defaults であるべきで、R1.2（kero 区別取込）との将来接続が暗黙のまま。
**Impact**: M1 実害ゼロ（emo2 に kero bindgroup なし・kero 実挙動は M-dual 範囲外）だが、M-dual が D7 写像表を拡張する際に「初期値も per-namespace 化する」宿題が設計に明記されていないと見落とす。
**Suggestion**: research.md の既知負債（供給源二重化）に「kero 初期集合の per-namespace 化は M-dual の宿題」と一言添える（tasks フェーズの注記で可・design 本文の改変不要）。
**Traceability**: R3.1／R1.2／R4.3
**Evidence**: design.md「seriko — 動的 bind 状態」「areka — 起動配線」＋ `crates/areka/src/emo2_boot/assets.rs:48-72, 462`

## Design Strengths

1. **全 file:line 主張が HEAD 実測で裏取り済みの「実在アンカー駆動」設計**: D10（既存檻の意図 FAIL 予告と更新方針）に代表される、変更が波及する既存テストまで事前特定した additive 計画は手戻りリスクを大きく下げる。brief 陳腐化補正（compile catch-all 失効の検出等）も research に明記され、並走 worktree の staleness 対策が効いている。
2. **縮退境界の型化と非空虚化**: `BindDirective` 4 類別（Apply/Toggle/CategoryWide/Malformed）＋`BindApplyOutcome` 3 値（Changed/StateOnly/Unchanged）が「実導出/縮退/冪等/保留」を型で峻別し、全縮退枝に正カウント assert を義務付ける（優しい縮退の非空虚化）。kero を「空表の同一機構」で自然縮退させ人工的無効化コードを書かない D7 も、defer-canon 原則（完全語彙＋縮退シーム）の模範解。

## Final Assessment

**Decision: GO**

**Rationale**: アーキテクチャ整合（broadcast＋演者側 relevance＋単一権威表への consumer-side additive）に矛盾なし、全 38 AC がトレース済み、load-bearing な実装アンカーは全件 HEAD 実測一致。指摘 2 件はいずれも severity 文言確定と注記追加であり、tasks/実装フェーズで吸収可能（設計の骨格に影響しない）。

**Next Steps**:
1. 設計ディスカッション（kiro-design-discussion）で Issue 1 の severity 確定（warn 前例踏襲 or 意図的 debug の注記）と Issue 2 の M-dual 宿題注記を扱う
2. 承認後 `/kiro-spec-tasks areka-P0-mayuna-compose` でタスク生成へ
