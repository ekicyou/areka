# 設計検証レポート — areka-P0-balloon-parse

> 生成: kiro-validate-design / 2026-07-01（非対話・レポートをディスクに永続化）
> 入力: spec.json（language=ja）・requirements.md（確定）・design.md（確定）・research.md・steering（product/tech/structure/roadmap）・既存 `crates/areka-parsers/src/sakura/*`・emo2 fixture 実データ。
> レビュー手順: design-review.md（Analysis → Critical Issues → Strengths → GO/NO-GO）。

---

## Design Review Summary

本設計は、確定要件（1.1〜6.4）を全網羅し、既存 `sakura` モジュールの確立規律（`Result` 無し寛容パース・不透明 NewType＋read-only アクセサ・`#[non_exhaustive]`＋最小派生・`tracing` のみ・in-source テスト・過剰実装禁止）を思想として忠実に踏襲しつつ、balloon 固有の「マージ」を独立層に切り出す明快なレイヤ分割（`model ← parse ← merge ← facade`）を採る。設計が固定するテスト期待値（s0s／k0s の確定マージ値・符号含む）は emo2 実 fixture と 1 行単位で一致することを本レビューで確認済みであり、最大リスクである座標符号意味の分類も型名・doc・テストの三重固定で対処されている。実装可能性・境界明確性・アーキ整合性のいずれも高く、実装フェーズへ進む準備は整っている。

## Critical Issues（≤3）

本設計に GO を妨げる重大なアーキ不整合・要件ギャップは検出されなかった。以下は実装フェーズで解像度を上げれば足りる**軽微な明確化項目**であり、いずれも NO-GO 事由ではない。設計ディスカッションで確認しておくと堅い。

### 🟡 明確化 1: 共通値 / サーフェス別値の境界がスケッチ止まり（実装時に確定要）

- **Concern**: `Balloon`（共通既定）と `BalloonSide`（サーフェス別）へのフィールド配置が「emo2 実データ分布に従って確定する」とされ、design 段階では未確定（design.md「State Management（型定義スケッチ）」の脚注）。特に `arrow0`/`arrow1` は s0s(15,90 / 15,-110) と k0s(9,54 / 9,-125) で異なるためサーフェス別、`wordwrappoint` は base(-34) を s0s のみ上書き(-49)・k0s は base 保持という非対称がある。
- **Impact**: 配置を誤ると要件 4.5（sakura/kero を取り違えない）・4.3（overlay 無→base 保持）の充足がテスト実装時にぶれうる。ただし fixture 値は本レビューで実データ一致を確認済みで、期待値自体は正しい。
- **Suggestion**: 実装着手時に「共通=type/use_self_alpha/origin/font/color、サーフェス別=windowposition/validrect/wordwrappoint/arrow」を型定義で先に固定し、merge_tests の (b) base 保持ケース（k0s の wordwrappoint.x=-34）を最初に書いて非対称を pin する。
- **Traceability**: 4.3 / 4.5 / 6.2 / 6.3
- **Evidence**: design.md「## Data Models / Domain Model」および model「State Management（型定義スケッチ）」脚注。

### 🟡 明確化 2: 未使用フィールドの「生保持」範囲が要件と設計でわずかに広め

- **Concern**: 要件 5.1 は未使用フィールド（communicatebox/onlinemarker/... 等）に保持義務を課さず「意味解釈しない」だけを求めるが、設計は診断目的で `RawFields.unknown` へ生保持する（parse「Responsibilities」「未知行保持は診断用途に留め、モデル公開面へは出さない」）。s0s/k0s には実際に number.*・onlinemarker.*・sstpmarker.*・sstpmessage.* が存在する。
- **Impact**: 過剰実装禁止（steering・要件 5）との緊張。ただし非公開の `RawFields` 内に留め公開面へ出さない方針なので実害は小さく、寛容規律（sakura の `Raw` 吸収）とも整合。
- **Suggestion**: 生保持を「無制限コレクション」ではなく診断ログ（`tracing::debug`）中心に寄せ、`RawFields` の未知行保持は境界（emo2 使用フィールド外）を明記。parse_tests で「未知行が認識キーを欠落させない」ことのみ固定し、未知行の内容を公開契約にしない。
- **Traceability**: 5.1 / 5.2 / 5.3
- **Evidence**: design.md「### 構文層 / parse」および「### 中間モデル `RawFields`（非公開）」。

### 🟡 明確化 3: fixture テスト内リテラル直書きと正本 fixture の乖離リスク

- **Concern**: fixture 取り込みは「検証最小抜粋をテスト内リテラルに直書き」（design.md「Fixture 取り込み方式」・研究 §5-8 候補 b）を採用。クレート境界跨ぎ `include_str!` の脆さは回避できるが、実 fixture（`crates/pilot/.../emo2-kakukaku/`）が改訂された際にテスト側リテラルが自動追従しない。
- **Impact**: 将来 fixture が更新されても回帰テストが旧値を pin し続け、実データとの乖離を検知できない可能性（要件 6.1「emo2 fixture を入力に」の精神との緊張）。現時点では本レビューで両者一致を確認済み。
- **Suggestion**: 直書きリテラルの出所（ファイル名・行）をテストコメントで明示し、`validation_tests` に「正本 fixture の該当行から採取」と記す。将来は照合用 helper か doc-test で正本参照を担保する余地を残す（本 spec のスコープ外で可）。
- **Traceability**: 6.1 / 6.4
- **Evidence**: design.md「### Fixture 取り込み方式（研究 §5-8 の how 判断）」。

## Design Strengths

- **要件トレーサビリティと実データ検証の完備**: Requirements Traceability 表が 1.1〜6.4 を Components/Interfaces/Flows へ全マッピングし、Testing Strategy が固定するマージ確定値（s0s/k0s・符号含む）は emo2 実 fixture と 1 行単位で一致することを本レビューで確認できた。「符号意味の分類取り違え」という最大リスクを型名・doc・テストの三重固定で正面から潰しており、実装の検証容易性が高い。
- **既存規律の踏襲と逸脱理由の明示が両立**: sakura の分割思想・寛容規律・NewType・in-source テストを必須踏襲しつつ、balloon 固有の「なぜ独立 lexer を持たず merge を追加するか」を Architecture／File Structure Plan で明文化（研究 §3/§4 の説明責任を吸収）。sakura を import しない独立モジュール・追加依存ゼロ・`lib.rs` 1 行追加という影響最小化も steering（過剰実装禁止・tech.md）に整合。

## Final Assessment

- **Decision**: **GO**
- **Rationale**: 全要件（1.1〜6.4）を網羅し、既存アーキ規律に整合、依存方向・境界・公開面が明確で、テスト期待値が実 fixture と一致確認済み。検出された 3 項目はいずれも実装フェーズで解消可能な明確化であり、重大なアーキ不整合・要件ギャップ・過大な複雑度はない（許容可能リスク）。
- **Next Steps**:
  1. 上記明確化 1〜3 を設計ディスカッション（`/kiro-design-discussion areka-P0-balloon-parse`）で軽く確認（必須ではない）。
  2. 問題なければ `/kiro-spec-tasks areka-P0-balloon-parse` で実装タスクを生成。
  3. 実装時は merge_tests の base 保持ケース（k0s wordwrappoint 非上書き）と符号保持アクセサ往復を最初に固定し、明確化 1 を pin する。
