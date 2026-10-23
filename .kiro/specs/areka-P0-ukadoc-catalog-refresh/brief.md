# Brief: areka-P0-ukadoc-catalog-refresh

> 起票: 2026-10-22 `/kiro-discovery`（`areka-P0-script-security-level` の要件討議の議題 3・開発者「起票漏れしないか？起票するなら今しましょう」）。

## Problem

網羅台帳（`doc/ukadoc-coverage/ledger/*.toml`）の行の id は、正典の写し `doc/ukadoc-coverage/catalog.toml` に実在しなければならない（無ければ検査が `LedgerIdNotInCatalog` で落ちる）。その写しが古いので、ukadoc に後から入った項目を台帳に載せられない。

- 実例: `\![execute,filewatch]`・`\![execute,schedule-add]`・`\![execute,schedule-delete]`（ukadoc では 2.8.91 の版の印が付く）がカタログに 1 件も無い。`areka-P0-script-security-level` は、この 3 つを「外部扱いの台本では実行しない」タグとして判定の表に載せる。ところが網羅台帳には行を足せないので、ukadoc の URL を出典に書くだけになっている。
- 同じ理由で、写しの後に ukadoc に入った項目はすべて、どの spec の持ち物にもならず、台帳のどこにも現れない。報告（`report/`）の数も古い正典の上で数えている。

## Current State

- `catalog.toml` の頭: `package = "ukagaka-doc-mcp"`・`package_version = "0.2.7"`・`generated_at = "2026-08-24T04:08:57.881Z"`・`ukadoc_entries = 1749`。
- 2026-10-22 の npm の最新は `ukagaka-doc-mcp` 0.10.3。
- 道具はそろっている（`crates/ukadoc-survey`・手引き `doc/ukadoc-coverage/README.md`）:
  - `cargo run -p ukadoc-survey -- diff`: 今のカタログと新しいスナップショットの差を並べる。
  - `catalog`: カタログを作り直す。
  - `ledger-init`: 初期の台帳を作って既存の台帳へ差し込む。
  - `report`・`report-summary`・`check`。
  - スナップショットの場所は `AREKA_UKADOC_SNAPSHOT` で指定でき、無ければ既定の場所を組み立てる。
- `catalog.toml` は「機械だけが書く・手で編集しない」決まり（README）。台帳に手で行を足す道は無い。
- 網羅の見直しの `coverage-roadmap-refresh` は「ukadoc のカタログの更新」を範囲外と明記している＝今の受け皿は無い。

## Desired Outcome

- カタログが今の ukadoc（`ukagaka-doc-mcp` の最新）の写しになり、`cargo test -p ukadoc-survey` と `check` が緑のまま通る。
- 新しく増えた項目が台帳に載る。状態は実装の今の姿に合わせ、持ち主は決まるものだけ書く（例: `\![execute,filewatch]` 系 3 行の注記に、`script-security-level` の判定の表に載っていることを書く）。
- 消えた項目・id が変わった項目（アンカーの付け替え・連番のずれ）を、台帳の行と持ち主を失わずに移す。
- 報告（`report/` の 5 本）を作り直す。

## Approach

既存の道具の手順に乗せる。⑴ 最新のスナップショットを取る → ⑵ `diff` で増えた・消えた・変わった項目を数えて、要件の段で規模を決める → ⑶ `catalog` → ⑷ `ledger-init` で増えた項目を差し込む → ⑸ 消えた・変わった id の台帳の行を手で移す → ⑹ `check`・`report`・`report-summary`。

道具に足りない所（スナップショットの形が 0.2.7 から変わっていて読めない、id の付け替えを機械で追えない、など）が出たら、その直しも範囲に入れる。

## Scope

- **In**:
  - スナップショットの更新。
  - カタログの作り直し。
  - 増えた項目の台帳への差し込み。
  - 消えた・id の変わった項目の台帳の行の移し替え。
  - 報告の作り直し。
  - 写しの更新に要る道具の直し。
  - 手引き（README）のうち写しの版に触れる記述の追随。
- **Out**:
  - 新しく増えた項目の実装（各 spec が持つ）。
  - `roadmap-draft.md`・`briefing.md` の手書きの数の撮り直し（`coverage-roadmap-refresh` の持ち物）。
  - 正典の増減に自動で気付く仕組み（開発規律「買わない」）。
  - 写しの定期的な更新の運用。

## Boundary Candidates

- スナップショットの取得とカタログの作り直し（道具を回す）。
- 台帳の行の差し込みと移し替え（id の差分の扱い）。
- 道具の直し（形の変化が見つかった場合だけ）。

## Out of Boundary

- 項目の状態を「実装済み」へ上げるための実装。
- 束の帰属と優先度の組み直し（`priority-apply` の入力の見直し）。増えた項目の帰属が要るなら、要件の段で範囲を決める。

## Upstream / Downstream

- **Upstream**: なし（道具は完了 `ukadoc-survey-toolkit` ほかで着地済み）。
- **Downstream**:
  - `coverage-roadmap-refresh`（写しの後に数え直すのが安い＝本 spec が先）。
  - `script-security-level`（`\![execute,…]` 3 行の注記。完了がどちらが先でも、後から着地する方が注記を合わせる）。
  - 写しの後の正典の項目を実装する spec すべて。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**:
  - `coverage-roadmap-refresh`（同じ `doc/ukadoc-coverage/` を触る・本 spec の後に置く）。
  - `audit-ukadoc-survey`（同じ `crates/ukadoc-survey` を見直す・道具を直すことになったら照合）。
  - 台帳の行の状態や持ち主を変える spec すべて（`ledger/*.toml` の同じ行の近くを触る）。

## Constraints

- `catalog.toml` は手で書かない。直すのは道具だけ。
- 台帳の決まり: id はカタログに実在すること・id 順に並ぶこと・台帳を触ったら `cargo test -p ukadoc-survey`。
- 版は `ukagaka-doc-mcp` の公開版を使う。版を上げるときは、0.2.7 → 0.10.3 の間でスナップショットの形が変わっていないかを最初に確かめる。
- 席: 台帳の行の状態や持ち主を変える spec と同じウェーブに置かない（`coverage-roadmap-refresh` と同じ条件）。v0.1.0 の 9 本が台帳の行を動かすので、v0.1.0 の後が安い。
- 規模の見立て: 差しだいで S〜M（5〜12 タスク）。`diff` で数えるまで決めない。
- 優先度の区分: C（ukadoc の拾い残し）。
