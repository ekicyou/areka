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

## `mcp-get-property` からの申し送り（2026-10-04・実装の後に書き直し）

> `areka-P0-mcp-get-property` の設計の段で書き（同 spec の要件 5.6）、実装と実機確認の後に実装の事実で書き直した。

1. **`get_property` は名前を手直ししない。** 処理（`crates/areka/src/mcp/get_property.rs` の `handle`）は、渡された `property_name` を受け取ったまま `runtime.sylphya_reader().resolve_dotted_str(&asker, &args.property_name)` へ渡す（`trim`・大小の変換・置き換えの呼び出しは 0）。問い手は宛先のゴースト自身（`AskerContext { asker: ghost_asker_id(&runtime.mount().shiori.dir) }`）、読み手は実行系の読み口 `GhostRuntime::sylphya_reader()`（`crates/areka-ghost/src/runtime.rs`・同 spec が足した）から借りる。答えは `DottedResolution::Value(v)` → `v` をそのまま（`isError: false`）、`NotFound` → `NG:Cannot find such property name.`。読み手の側（`resolve_dotted_str`／`parse_dotted`）で大小を畳めば、`get_property` は何も変えずに追従する。
2. **`get_property` のテストは大小の扱いを固定していない。** 値を読むテストは `crates/areka/src/mcp/get_property_tests.rs` の `reads_values_through_the_ghost_own_asker_on_a_real_runtime` 1 本で、聞く 7 つの名前は `baseware.name`・`test.key`・`test.other_only`・`test.empty`・`currentghost.name`・空の名前・前に空白 1 つの ` baseware.name`。実行系が無いときのテスト `answers_unavailable_and_warns_once_without_a_runtime` も `baseware.name`（小文字）だけを渡す。英字はどれも小文字だけで、大文字を混ぜた名前を期待するテストは 0 本＝本 spec の着地を赤で妨げない。ただし ` baseware.name`（前の空白）は `NG:Cannot find such property name.` を期待する＝前後の空白を削らないことは固定している（SSP 2.9.07 も削らない・survey §7.1）。大小を畳むときに空白の除去まで足すと、このテストが赤になる。
3. **実機確認に大小を混ぜた名前を足す場所。** 同 spec の `verification/signoff.md`（2026-10-04・配布形 `584feaa1`・emo2・待受 9801）は、curl の `tools/call` で ⑴ `baseware.name` → `areka`、⑵ `no.such.thing` → `NG:Cannot find such property name.`、⑶ `ghost_name: "えも？？"` を渡した `baseware.name` → `areka` の順に呼んだ。大小の確認は ⑴ の直後に `BASEWARE.NAME` を 1 行足せばよい（本 spec の着地の前は `NG:Cannot find such property name.`、後は `areka`）。手順（`tools/package.ps1` の配布形を `target\` の下へ展開・`AREKA_APP_SMOKE_EXIT_MS` の有界の自動終了・記録の行「MCP: 待受を始めた url=…」から URL を読む・`MCP-Protocol-Version: 2026-07-28` の無状態版の形）はそのまま使える。

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: S〜M（5〜9）のまま。切る: なし。
- 前提の状態: 前提は無い＝今すぐ着手できる。`mcp-get-property`（✅ 10-04）が着地し、上の申し送りの事実（`crates/areka/src/mcp/get_property.rs` の `handle` は名前を手直しせず `resolve_dotted_str` へ渡す・55 行）は main でもそのまま。
- 崩れた前提／古くなった位置: なし。C3 は `crates/areka-sylphya/src/` に説明文の言い換え（「M2」→「α 後」）しか入れていない。`key.rs` 411（`parse_dotted` の定義行）・`reader.rs` 584（`resolve_dotted`／`resolve_dotted_str` の定義行）・`actor.rs` 754。
- **穴の候補を静的に確かめた**: `actor.rs` の `classify_set` は ⑴ `SET_EFFECTIVE` に大小まで一致 → 運行の値、⑵ `is_canonical_vocab`（根が `DOTTED_ROOTS` に・葉が `GENERIC_PROP_NAMES` に、どちらも `contains` の完全一致）→ 書けない名前、⑶ それ以外 → 自由な名前として保存、の順。`CURRENTGHOST.NAME` は ⑵ をすり抜けて ⑶ の保存へ落ち、`MENU` のような SET 有効の名前も ⑴ を外れて保存へ落ちる。今は台本から SET へ届く道（`\![set,property]`）が無く、`PropSetCueSink` はカウンタの名前の族だけ・SHIORI の `SetProperty` は環境変数で開くデモだけ＝利用者に見える害はまだ無い。`property-query-channels` が書く道を開く**前に**着地すると、穴が開いたまま出ることが無い。
- 触るファイル: `crates/areka-sylphya/src/{key.rs, reader.rs, actor.rs}`（名前を正準の鍵にする所・`classify_set`）と兄弟のテスト・`crates/areka-sylphya/src/persist/`（保存の読み戻し・畳むなら）・`crates/areka/src/mcp/get_property_tests.rs`（大文字の名前を 1 つ足すなら）・`doc/COMPAT_ARCHITECTURE.md` §8。
- 議題（答えで作業が変わるものだけ）: 畳む範囲（括弧の中の名前・自由な名前・保存された名前を含めるか）。前回どおり要件の段で決める。
- 見つけた穴: なし（上の候補は本 spec の範囲で塞ぐ）。並走の照合: プロパティの動く値の列（`currentghost-property-tree` ほか・`actor.rs` を分け合う）と同時に走らせない。kanade・host32・`emo2_boot` には触れない＝それらの列の spec とは並べられる。

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**: 前提は無いまま＝今すぐ着手できる。C4 の 16 本は `crates/areka-sylphya/src/` に 1 行も触れていない（`f26aa1c1` からの変更 0）。穴（書き込みの仕分け `classify_set` が大小まで一致で比べ、`CURRENTGHOST.NAME` が「書けない名前」をすり抜けて保存へ落ちる）は今もそのまま。台本から書く道はまだ無い（`property-query-channels` が未着手）ので、利用者に見える害は出ていない。
- **兄弟の決まりが先に着地した**: `mcp-ghost-name-match`（✅ 10-06）は MCP の宛先の名前を「前後の空白を除く＋半角の英字の大小だけ同じとみなす」で比べる（`crates/areka/src/mcp/resolve.rs` の `resolve`）。本 spec のプロパティの名前は「半角の英字の大小だけ畳む・前後の空白は削らない」が素直（空白を削らないことは `crates/areka/src/mcp/get_property_tests.rs` が固定している・SSP も削らない）。空白の扱いが 2 つの決まりで違うことを要件に 1 行で書く。括弧の中の名前（`ghostlist(名前)`）も畳むなら、宛先の名前と同じ「半角の英字だけ」に揃える。
- **触るファイル**: `crates/areka-sylphya/src/key.rs`（411・点付きの名前の解釈 `parse_dotted` と 1 区切りの解釈。`index(…)` の見分けも完全一致）・`actor.rs`（754・`classify_set`・正準語彙の判定・自由な名前を保存へ回す腕）・`reader.rs`（584・解釈の側で畳めば変更 0 の見込み）・`persist/{mod.rs, format.rs}`（403・533・保存済みの名前の読み戻しを畳むなら）と兄弟のテスト（`actor_tests.rs`・`ledger_key_determinism_tests.rs`・`persist/persist_tests.rs` 786・新しいテスト）・`crates/areka/src/mcp/get_property_tests.rs`（大文字の名前を 1 つ足すなら）・`doc/ssp-mcp/survey.md` 7.3 節・`doc/COMPAT_ARCHITECTURE.md` §8。`mirror.rs`・`vocab/dotted.rs`・`areka-ghost`・kanade・`emo2_boot` には触れない。1,000 行に近いファイルは無い。
- **規模**: S〜M（5〜9）のまま。**分割の案**: なし。
- **先に要るもの**: なし。**本 spec を待つ未完了の spec は 7 本**＝直に 2 本（`currentghost-property-tree`＝同じ `actor.rs` を分け合う／`property-query-channels`＝書く道を開く前に穴をふさぐ、という棚卸㉒の裁定。ファイルは重ならない）と、その先の 5 本（`currentghost-property-others`・`system-property-values`・`property-catalog-lists`・`zorder-property`・`sakura-embed-directive`）。据え置きの `property-ipc-transport` は待たない。
- **ファイルの重なり**: `currentghost-property-tree`（`actor.rs`）・`property-catalog-lists`（`actor.rs`・`key.rs`）・`system-property-values`（`key.rs`）・`zorder-property`（`actor.rs`）。どれも本 spec の後ろに並ぶ同じ列の仲間で、ほかの列（kanade・台本・`emo2_boot`・配置・MCP）の spec とは 0＝どのウェーブにも単独で入れられる。
- **優先度の区分**: C（SSP に合わせる持ち越し・ukadoc は黙っている）。出どころは `mcp-get-property` の議題 1 で開発者が「起票して残す」と決めたことなので、A と読む余地もある。潜在の穴を 1 つ含むが、今は害が出ない。
- **要件定義のモデル**: Opus で足りる（畳む範囲の分かれ目は `mcp-ghost-name-match` の決まりが手本になる）。
- **見つけた穴・古くなった記述**: `doc/ssp-mcp/survey.md` 7.3 節の 6（「areka の宛先の解決は名前を完全一致で比べ、空文字を省略と同じに扱う」）は `mcp-ghost-name-match` の着地で偽になった（7.4 節には 10-06 の追記があるが 7.3 節には無い）。roadmap の列「プロパティの動く値」の重なる場所に `reader.rs`・`persist/` が載っていない。本文 Constraints の列挙にも `key.rs`・`reader.rs` が無い。
