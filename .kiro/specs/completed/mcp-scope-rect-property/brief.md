# Brief: mcp-scope-rect-property

> 2026-10-23 `areka-P0-pasta-in-windows-verification` のタスク 5.3 で `/kiro-discovery` により起票（本 spec からの申し送り（2026-10-23））。開発者の決め（同日のチャット）「0.1.0までに解決しない場合、独立して起票せよ」「原則として、すべての申し送りが0.1.0に解決するのであれば承認する。申し送り漏れはダメですよ。」。版は **v0.1.0**（ゴールの 1 つ目「SSP 並みの自動試験」）。

## Problem

- **開発者と自動試験の道具**: 外から窓の位置を読む MCP の `get_property` に `currentghost.scope(0).rect`・`currentghost.scope(1).rect`・`currentghost.balloon.scope(0).rect` を聞くと、areka は `NG:Cannot find such property name.` と答える。SSP は値を答える。
- ぱすたの検証では、窓とバルーンの位置を撮影と記録の `bounds=` の行から読み替えるしかなかった。重なり・ドラッグ・画面をまたぐ移動の確かめを自動の試験に載せられない＝v0.1.0 の 1 つ目のゴール（SSP 並みの自動試験）の穴。

## Current State

- **証拠**: `verification.md` 5 節の崩れた所の表の **20**（根拠 `target\pasta-verify\shots\run1\notes.md` の「走行」）・同 5 節「手順の側の件」の 3 つ目。原因の場所 `crates/areka/src/mcp/get_property.rs:48`（sylphya の読み手 `resolve_dotted_str` が値なしを返すと `NotFound` で断る＝値を出す所がまだ無い）。
- **SSP の基準**（`ssp-measurement.md` 2 節）: 段 1 で `scope(0)`・`scope(1)` の rect はどちらも `-782,201,49,1227`、バルーンの rect は `-722,288,-68,963`。段 4 のドラッグの後は `-1482,201,-651,1227`（2 つが揃って動く）。段 5 で 200% の画面へ移った後は `818,261,1649,1287`、バルーンは `878,348,1532,1023`（`target\pasta-verify\ssp\stage5-rects.txt`）。形は「左,上,右,下」の画面の座標。
- **正典（ukadoc）**: `currentghost.scope(ID).rect`「現在のゴーストのスコープIDのウインドウ矩形座標(左,上,右,下)。」・`currentghost.balloon.scope(ID).rect`「現在のバルーンの矩形領域。」（SSP 2.8.39）。
- **sylphya**: 値を出す口は `publish_static`・`publish_shiori`・`persist_put` の 3 つだけ（`crates/areka-sylphya/src/actor.rs`）。窓の位置のように刻々と変わる値（動く値）を出す口が無い。
- **持ち主の今**: `currentghost.balloon.scope(ID).rect` は `currentghost-property-tree`（v0.3.0）の 19 項目、`currentghost.scope(ID).rect` は `currentghost-property-others`（いつか）。動く値の口の持ち主も `currentghost-property-tree`。どちらも v0.1.0 の外。

## Desired Outcome

- MCP の `get_property` と、ゴーストが読むプロパティの道（`sylphya` の読み手）で、`currentghost.scope(ID).rect`（本体・相方・3 人目以降の窓）と `currentghost.balloon.scope(ID).rect` が、今の窓の画面の座標「左,上,右,下」で読める。ドラッグ・`\![move]`・画面をまたぐ移動・面の替わりの後も、読んだ時点の値が返る。
- 窓の無いスコープ・出ていないバルーンは値なし（捏造しない）。SSP が出ていないバルーンに何を答えるかは要件の段で測る。
- ぱすたの検証の手順書（`doc/pasta-real-run.md`）の「撮影と記録で読み替える」所を、`get_property` で読む形へ戻せる。

## Approach

- sylphya に動く値の口を足す（`currentghost-property-tree` が持っていた議題「変わるたびに出す」か「読む時に問い合わせる」かを本 spec が決める）。窓の位置の持ち主（`placement`・wintf の窓）から値を取る。
- 2 つの名前だけを載せる。同じ口に後から `currentghost-property-tree`・`currentghost-property-others`・`shell-scale-linkage` が値を足す。

## Scope

- **In**: 動く値の口（形の決め・sylphya の口・切替と読み直しでの出し直し）・`currentghost.scope(ID).rect`・`currentghost.balloon.scope(ID).rect`・座標の形（物理の画素か論理の画素か・複数の画面の負の座標）を SSP で測って合わせる・決定論のテスト・MCP の `get_property` の実機の確かめ・網羅台帳 `property.toml` の 2 行の持ち主・手順書 `doc/pasta-real-run.md` の読み替えの所の戻し。
- **Out**: `currentghost.*` の残り（`currentghost-property-tree`・`currentghost-property-others` のまま）・倍率と DPI のプロパティ（`shell-scale-linkage`）・`\![get,property]` などゴーストから読む台本の道（`shell-scale-linkage` が `\![get,property]` を、残りは `property-query-channels`）・`currentghost.scope(ID).surface(ID).rect`（面ごとの予測の矩形）。

## Boundary Candidates

- sylphya の動く値の口（最下層・他の値の土台）
- 窓とバルーンの矩形を集める所（areka 本体の側・`placement` と `emo2_boot`）

## Out of Boundary

- 窓の置き方そのもの（`placement` 系 spec）
- プロパティの名前の英字の大小（✅ `property-name-case-fold`）

## Upstream / Downstream

- **Upstream**: ✅ `mcp-get-property`（読む道）・✅ `property-name-case-fold`（同じ `actor.rs`）。
- **Downstream**: `shell-scale-linkage`（同じ口に倍率と DPI を載せる）・`currentghost-property-tree`・`currentghost-property-others`・`system-property-values`・`property-catalog-lists`・`zorder-property`（どれも動く値の口を使う）・`pasta-in-windows-verification` の回し直し・SSP 並みの自動試験。

## Existing Spec Touchpoints

- **Extends**: `currentghost-property-tree`（`balloon.scope(ID).rect` と動く値の口を本 spec へ移した＝同 brief に 1 行）・`currentghost-property-others`（`scope(ID).rect` を本 spec へ移した＝同 brief に 1 行）。
- **Adjacent**: `sylphya-sqlite-kvs`（v0.2.0・sylphya の保存を作り直す＝本 spec の口は保存を通さない形にしておく）・`window-link-directives`・`dpi-realign-remembered-chain`（窓の位置を変える側）。

## Constraints

- 値の捏造禁止（未導出は値なし）・決定論のテストは必達。
- 正典の読みはライブの ukadoc（ukadoc MCP の snapshot のプロパティ節は 2.8.80 の意味で古い）。
- 実機の確かめは排他テストの机を持ってから。

## 起票時の見立て（2026-10-23・ブランチ `claude/areka-p0-pasta-windows-d4ed7a`）

- 触るファイル: `crates/areka-sylphya/src/{actor.rs, mirror.rs, vocab/dotted.rs}`・`crates/areka-ghost/src/{sylphya_wiring.rs, runtime.rs}`・`crates/areka/src/emo2_boot/`（値を集める所は新しいファイル）・`crates/areka/src/placement/`（読むだけ）・`doc/ukadoc-coverage/ledger/property.toml`・`doc/pasta-real-run.md`。
- 規模: M（8〜12）。
- 優先度の区分: A（開発者の指示＝v0.1.0 のゴール）。段: その他。
- 要件定義のモデル: Fable（動く値の口の形で後ろの 6 本の形が決まる・スレッドをまたぐ読み）。
- 議題（答えで作業が変わるものだけ）: ⑴ 動く値を「変わるたびに出す」か「読む時に問い合わせる」か。⑵ 出ていないバルーンの rect に SSP が何を答えるか（測る）。

## 2026-10-23 棚卸㉙の再測定（main aa8da8a7）
- **前提の変化**: 起票（10-23）の枝はそのまま main `aa8da8a7` に入った＝起票の後に触る所は動いていない。上流の ✅ `mcp-get-property`・✅ `property-name-case-fold` は着地済み。MCP の `get_property` の答えを作る所 `crates/areka/src/mcp/get_property.rs`（55）は、値が無いと 48 行目で `NotFound` を返す。MCP の道具の入口 `crates/areka-mcp/src/tools/get_property.rs`（43）は運ぶだけ。
- **触るファイル**: 値の置き場 `crates/areka-sylphya/src/actor.rs`（808・出す口 `publish_static` は 473 行目・`publish_shiori` は 487 行目）・`mirror.rs`（248）・`vocab/dotted.rs`（407）・**読み手 `crates/areka-sylphya/src/reader.rs`（584・`resolve_dotted_str` は 90 行目。起票の見立てに無かった）**・`crates/areka-ghost/src/{sylphya_wiring.rs 423, runtime.rs 815}`・`crates/areka/src/emo2_boot/` の値を集める新しいファイル（新）・`crates/areka/src/placement/`（読むだけ）・要れば `crates/areka/src/mcp/get_property.rs`（55）・網羅台帳 `property.toml` の 2 行・手順書 `doc/pasta-real-run.md`。値の書き手を World に差す所は、今の位置の記憶の差し方（`crates/areka/src/main.rs` 805 行目の `insert_persist_wiring`）の形が先例だが、`main.rs` は 978 行で上限が近い＝差す所は `emo2_boot` の側の新しいファイルに置く。
- **規模**: M（8〜12 タスク）のまま。上限 20 の内。
- **先に要るもの**: 働きの上ではなし（上流は着地済み）。下流の `shell-scale-linkage` はこの spec の口を待つ（同じウェーブに置かない・sylphya のファイルも重なる）。v0.1.0 のほかの spec とのファイルの重なりは見つからない。`mcp-strict-errors`（別の組）とは、向こうが MCP の待受 `crates/areka-mcp/src/handler.rs` を触り、こちらは触らない＝共有のファイル無し（こちらが NG の文言を変えるなら `crates/areka-mcp/src/tools/get_property.rs` が近いが、向こうは挙げていない）。依存を足さない。実機の確かめは排他テストの机。出ていないバルーンに SSP が何を答えるか（議題 2）は開発者の機械の SSP の MCP で測る。静かな机・前後の全体テストの比べは要らない。
- **優先度の区分**: A（開発者の指示＝v0.1.0 のゴール「SSP 並みの自動試験」・ぱすたの検証の申し送り）。
- **要件定義のモデル**: **Opus**（起票の見立ての Fable から改める）。値の置き場 sylphya は「掲示板」の作りで、読みは同じ手番で待たずに済み、書き手が新しい写しを差し替える（`crates/areka-sylphya/src/mirror.rs` の冒頭の説明）。議題 1 の「変わるたびに出す」はこの作りに、`publish_static`・`publish_shiori` と同じ形の口を 1 つ足すだけ＝今ある口に足す形で、新しい境目にも新しい並行の約束にも当たらない。「読む時に問い合わせる」を選ぶと、MCP の手番から画面の手番へ待ち合わせる新しい並行の約束になるので、そのときだけ Fable へ戻す（要件の段で「変わるたびに出す」に倒すのが推し）。
- **分割の案**: なし。
- **見つけた穴・古くなった記述**:
  - 起票の見立ての触るファイルに、読み手 `reader.rs` と MCP の答えを作る `crates/areka/src/mcp/get_property.rs` が無い（上）。
  - 「読んだ時点の値が返る」（Desired Outcome）は、書き手が出した値が読み手に届くまでの間がある作り。ドラッグの直後に読んだときに古い値が返らない約束をどこまで持つか（例: 窓を動かした同じフレームのうちに出す）を要件で書く。
  - 網羅台帳 `property.toml` の 2 行の持ち主は、もうこの spec になっている（`currentghost.scope(ID).rect`・`currentghost.balloon.scope(ID).rect`）。
