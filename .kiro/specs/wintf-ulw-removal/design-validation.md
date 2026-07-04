# 設計バリデーションレポート: wintf-ulw-removal

> 対象: `.kiro/specs/wintf-ulw-removal/design.md`（phase=design-generated・言語=ja）
> 参照: requirements.md（確定）・research.md（gap 分析＋D1〜D8 確定記録）・steering `tech.md`
> 実施日: 2026-07-04（コードベース実査で load-bearing 主張を照合）
> モード: 非対話（GO/NO-GO 判定を直接提示）

---

## レビューサマリ

本設計は純粋な削除リファクタ（ULW 撤去→GPU 合成 WUC 単独へ collapse）として実装準備完了レベルにある。File Structure Plan の削除・編集・Preserve 集合は具体的な絶対パス・行番号まで確定し、全 28 要件 ID（1.1〜7.3）がトレーサビリティ表に対応、境界 4 節（Owns/Out/Allowed/Revalidation）も populated 済み。設計の核心的主張（`CommitComposition` 空化と `tick_order_tests` 13 本固定列の非破壊性、areka/examples の呼び出し実在、collapse 方式 Option A の front-run 妥当性）はコードベース実査で裏付けを確認した。指摘は軽微なドキュメント衛生 1 件にとどまり、アーキテクチャ的な不整合や重大ギャップは無い。

---

## 実査による裏付け（GO 判断の根拠）

- `ecs/world/mod.rs`: `Schedule::new(CommitComposition)`（138 行）・`try_run_schedule(CommitComposition)`（532 行）・`EXPECTED_ORDER: [&str; 13]`（612 行、`CommitComposition` を 624 行に含む）を確認。D3（空化して schedule label・生成・tick 呼び出しは残す）は `tick_order_tests` を無改変で緑維持でき、設計主張は正確。
- examples: `ulw_twin_demo.rs`・`ulw_debug_demo.rs`（削除対象）・`multi_backend_demo.rs`（削除対象）の実在を確認。design.md が gap §5 未列挙の 2 本を実査で拾い上げた記述（D5 実査補足）は事実。
- areka `main.rs`: `CompositionMode` import（29 行）・`composition_mode: CompositionMode::DComp`（225・292 行）の実在を確認。Req3.5 の追随対象は design 記載どおり。

---

## クリティカルイシュー（≤3）

🟡 **イシュー 1（軽微・ドキュメント衛生）**: areka `main.rs` の ULW 由来コメントが Req7.2 の整合対象から漏れている
**Concern**: `main.rs` 220-231 行の Window/WindowStyle リテラルには「ex_style は factory が composition_mode から自動計算する（compute_ex_style）」（230 行）「ex_style は factory の compute_ex_style が DComp に応じて…」（222-224 行）など、`composition_mode` フィールド前提のコメントが残る。design の File Structure Plan は `main.rs` を「フィールド＋import 除去」としか記さず、Req7.2 のトレーサビリティは comment 整合を wintf 側（`components`/`lifecycle`/`window_factory`/`world/mod`）に限定しているため、これら areka コメントは明示的な整合対象に含まれない。
**Impact**: フィールド削除後、削除済みシンボル `composition_mode` を指すコメントが残存し、Req7.2（撤去された ULW 経路を前提とする残余記述を含まない）の趣旨に対する小さな漏れになる。ビルド・挙動には影響しない。
**Suggestion**: File Structure Plan の `main.rs` 編集項目に「220-231 行の `composition_mode` 前提コメントを WUC 固定の現況へ整合」を追記（tasks フェーズで拾えば足りる粒度）。
**Traceability**: Req7.2
**Evidence**: design.md「File Structure Plan / 編集」`crates/areka/src/main.rs` 項、Requirements Traceability 7.2 行

（重大イシューは以上 1 件のみ。他に GO を妨げるアーキテクチャ的不整合・要件ギャップは検出されなかった。）

---

## 設計の強み

- **境界と非破壊の実証性**: 「巻き込み禁止」集合（WUC 保全集合・クリックスルー層・13 本 schedule label）を Preserve として明示し、クリックスルー α 源が per-widget `AlphaMask` のみで ULW compositor の staging α に非依存であること（Req5.4）を実査で裏付けた上で設計に反映している。削除リファクタで最も危険な「意図しないブラスト半径」を正面から封じている。
- **D3 の的確なリスク回避**: `CommitComposition` を削除せず空化する判断により、`tick_order_tests` の 13 本固定列を無改変で緑維持し、影響を Req2 の範囲（ULW system 登録解除）に閉じ込めている。schedule label・`Schedule::new`・`try_run_schedule` を消さないという明示的禁止事項まで設計に落としており、実装時の踏み外しを予防している。

---

## 最終判定

**判定: GO**

**Rationale**: 削除対象・追随箇所・Preserve 集合がパス／行番号レベルで確定し、tick 構成不変性・クリックスルー非依存・collapse 方式（Option A front-run）の全ての load-bearing 主張がコードベース実査で裏付けられている。唯一の指摘は areka コメントのドキュメント衛生（非ブロッキング）で、tasks/impl フェーズで吸収可能。

**Next Steps**:
- イシュー 1 を tasks 生成時に File Structure Plan の `main.rs` 編集粒度へ織り込む（design.md の改訂は必須ではない）。
- `/kiro-spec-tasks wintf-ulw-removal` で実装タスクを生成する。
- impl 時の Req1.4 プロセスゲート（撤去対象の事前提示）は本 design の File Structure Plan を確定版として運用する。
