# Brief: areka-P0-mcp-scope-rect-property

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
