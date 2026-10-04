# Brief: areka-P0-property-name-case-fold

> 2026-10-04 `/kiro-discovery` で起票（`mcp-get-property` の要件ディスカッション 議題 1 で開発者が「起票して残す」と裁定）。事実の正本は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md) §7（SSP 2.9.07 の実測）。コードは「何の定義か」で指す。着手時に引き直す。

## Problem

SSP はプロパティの名前の英字の大小を区別しない（`BASEWARE.NAME` → `SSP`）。areka のプロパティシステム（sylphya）の読み手は大小を区別するので、同じ聞き方が「無い名前」になる。SSP で動くように書かれたゴーストが大小を混ぜて書いていると、areka では値が取れない。読み手は 3 つの口（SHIORI の `GetProperty`・台本の `%property[]`・MCP の `get_property`）で共有されているので、ずれはプロパティシステム全体のもの。

## Current State

- ukadoc は名前の大小について一般の規則を書いていない。「大文字小文字は区別しない」と明記があるのは `developer.log.ログ種別(名前).count` の括弧の中の名前だけ（ukadoc MCP で「大文字小文字」を引いて 1 件）。＝SSP の実装のふるまいで、ukadoc の正典ではない。開発者は互換を優先して起票を選んだ。
- 点付きの名前の解釈 `parse_dotted`（`crates/areka-sylphya/src/key.rs`）は綴りを保ったまま区切る。読み手 `SylphyaReader::resolve_dotted_canonical`（`crates/areka-sylphya/src/reader.rs`）は正準の綴りを鍵に、ゴーストごとの表 → 全体の表を完全一致で引く。
- 書き込みの検査（`crates/areka-sylphya/src/actor.rs` の正準語彙かどうかの判定＝根が `DOTTED_ROOTS` に入るか・葉が `GENERIC_PROP_NAMES` に入るか）も大小を区別する。**穴の候補**: `\![set,property,CURRENTGHOST.…,値]` のように大文字で書くと、正準語彙と見なされず「書けない名前」（`NotSettable`）を素通りして、自由な名前として書き込まれうる（着手時に確かめる）。
- 括弧の中の名前（`ghostlist(えも2debug).path`）も SSP は大小を区別しない（survey §7.1）。areka の一覧系の値はまだ無い（`property-catalog-lists` の持ち物）。
- 保存された名前（`areka.*`）は綴りのまま永続している。

## Desired Outcome

- 名前の英字の大小だけが違う聞き方に、3 つの口のどれでも同じ値が返る（`BASEWARE.NAME` → `areka`）。
- 書き込みの検査が大小で迂回されない。
- 3 つの口の答えがそろったまま（どれか 1 つだけ畳まない）。

## Approach

畳む場所を読み手の側の 1 か所（名前を正準の鍵にする所）に置き、3 つの口は何も変えずに追従させる。畳む範囲（点で区切った名前だけか、括弧の中の名前もか・自由な名前と保存された名前を含めるか・既存の保存の読み戻しをどう扱うか）は要件の段で決める。

## Scope

- **In**: 点付きの名前の大小を区別しない引き当て・書き込みの検査の大小の迂回をふさぐこと・保存された名前との整合・決定論テスト（3 つの口の代表で同じ答え）。
- **Out**: プロパティの値そのものの追加・`ghost_name`（MCP の宛先の名前）の照合（`resolve.rs`・別の話）・括弧の中の名前で一覧から選ぶ処理そのもの（`property-catalog-lists`）。

## Boundary Candidates

- 名前 → 正準の鍵（`key.rs`・`reader.rs`・`actor.rs` の正準語彙の判定）。
- 保存の読み戻し（`persist/`）。

## Out of Boundary

- MCP の `get_property`・SHIORI の `GetProperty`・`%property[]` の各口のコード（読み手に頼るだけで、変えない）。
- 名前の括弧の読み方（`ghostlist(0)` を番号と読むか名前と読むか＝`property-catalog-lists`）。

## Upstream / Downstream

- **Upstream**: なし（今の sylphya の上に立つ）。
- **Downstream**: `mcp-get-property`（手直しをしないので、本 spec の後は何もせず大小を区別しなくなる）・`property-query-channels`・`currentghost-property-tree` ほかプロパティの動く値の列（値を足すときに大小を気にせず済む）。

## Existing Spec Touchpoints

- **Extends**: なし（sylphya の完了 spec は消化できない＝新しい spec）。
- **Adjacent**: `mcp-get-property`（要件 1.4「名前を手直ししない」＝本 spec の後に追従する前提）・`property-catalog-lists`（括弧の中の名前）・`property-query-channels`（`\![set,property]` の口）。

## Constraints

- 段は「その他」（プロパティ）。プロパティの動く値の列（`crates/areka-sylphya/src/{actor,mirror,vocab/dotted}.rs`）と触るファイルが重なる＝同じ列の spec と同時に走らせない。規模の見立て S〜M（5〜9）。
- 意味は ukadoc から持ち込む方針の例外（ukadoc が黙っている所を SSP に合わせる）であることを、要件の冒頭に書く。
