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

## `mcp-get-property` からの申し送り（2026-10-04）

> `areka-P0-mcp-get-property` の設計の段で書いた（同 spec の要件 5.6）。同 spec を完了するときに、実装の事実で書き直す。

1. **`get_property` は名前を手直ししない。** 処理（`crates/areka/src/mcp/get_property.rs` の `handle`）は、渡された `property_name` を受け取ったまま読み手 `SylphyaReader::resolve_dotted_str` へ渡す設計（前後の空白の除去・英字の大小の変換は 0）。読み手の側で大小を畳めば、`get_property` は何も変えずに追従する。問い手は宛先のゴースト自身（`ghost_asker_id(&runtime.mount().shiori.dir)`）で、読み手は実行系の読み口 `GhostRuntime::sylphya_reader()`（同 spec が足す）から借りる。
2. **`get_property` のテストは大小の扱いを固定していない。** `crates/areka/src/mcp/get_property_tests.rs` のテストは小文字の名前だけを使う設計（`baseware.name`・`currentghost.name`・自由な名前 `test.key`・`test.other_only`・`test.empty`・空の名前）。大文字を混ぜた名前を「無い名前」と期待するテストも、値が返ると期待するテストも 0 本＝本 spec の着地を赤で妨げない。
3. **実機確認に大小を混ぜた名前を足す場所。** `get_property` の実機確認（同 spec の design「実機確認」の手順 3 の表）は、⑴ `baseware.name` → `areka`、⑵ `no.such.thing` → `NG:Cannot find such property name.`、⑶ `ghost_name` を渡した `baseware.name` の順に呼ぶ。大小の確認は ⑴ の直後に `BASEWARE.NAME` を 1 行足せばよい（本 spec の着地の前は `NG:Cannot find such property name.`、後は `areka`）。同じ手順（配布形を `target\` の下へ展開・記録の行から待受の URL を読む・`curl` の `tools/call`）がそのまま使える。
