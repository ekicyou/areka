# Brief: descript-window-keys

> 2026-10-18 棚卸㉗で起票（開発者の指示「もう一つの目標は、ゴースト「ぱすた」がSSPと同じレベルで動くこと、です。」＝v0.1.0 のゴール）。証拠はこの棚卸のぱすたの抜けの調べと、ゴースト「窓際のぱすたさん」のゴーストの descript（読むだけ・`sakura.seriko.defaultsurface,1000`・`balloon.dontmove,true`）。正典は ukadoc MCP で 2026-10-18 に引き直した。

## Problem

- **ゴーストの作者**: descript に書いた窓の決まり 2 つを areka が読まない。
  - **最初に出す面**（`sakura.seriko.defaultsurface` ほか）: ぱすたは着せ替えの面 1000 を既定にしている（ぱすたの descript のコメント: 既定の 0 だと、起動の直後とリロードの直後に辞書が `\s[1000]` を出すまで、ずれた小さい絵が出て窓とも重ならない）。areka は面 0 を出す＝起動の一瞬がずれて見える。
  - **バルーンを動かさせない**（`balloon.dontmove,true`）: ぱすたの縦書きのバルーンは窓の枠の絵にぴったり重ねてある。利用者がバルーンをドラッグできると、絵からずれる。areka はバルーンのドラッグを止めない。

## Current State

- **正典（ukadoc・要旨）**:
  - ゴーストの descript `sakura.seriko.defaultsurface,数値`: 本体側の既定の面（既定 0）。`kero.seriko.defaultsurface`: 相方側（既定 10）。`char*.seriko.defaultsurface`: `\p[*]` の既定の面（既定は 10 と書かれている）。プロパティ `currentghost.scope(ID).seriko.defaultsurface`（設定も可能）。
  - ゴーストの descript `balloon.dontmove,true`: バルーン位置調整の標準設定を OFF にする（既定 false）。
  - シェルの descript `sakura.balloon.dontmove`・`kero.balloon.dontmove`・`char*.balloon.dontmove`（SSP 2.4.96）: そのスコープのバルーンを利用者が動かすことを禁止する。1 か true で禁止。既定はゴーストの descript の `balloon.dontmove`。
  - `\![lock,balloonmove]`（SSP 2.4.96）: `\![unlock,balloonmove]` が呼ばれるまで、利用者のドラッグでのバルーンの移動を止める。descript の `sakura.balloon.dontmove` で恒久的にも指定できる。
- **areka**:
  - 最初に出す面は決め打ち: スコープ 0 は面 0、それ以外は面 10（`crates/areka/src/emo2_boot/assets.rs` の 411 行あたり・定数 `KERO_INITIAL_SURFACE_ID`）。窓の大きさを測る所にも同じ決め打ちがある（`crates/areka/src/placement/measure.rs` の 41・190〜205 行あたり）。descript から最初の面を読む所は 0（完了 `emily-ghost-verification` の棚卸㉓の申し送り）。
  - バルーンのドラッグ: `crates/areka/src/placement/follow/drag_follow.rs`（937 行）の `on_balloon_drag`・`on_balloon_drag_end`。止める決まりは無い。
  - descript の読み取り: 配置の設定は、ゴーストとシェルの両方の descript を受け取って組む（`crates/areka/src/placement/config.rs` の `build_placement_config`・687 行）。
  - 網羅台帳 `assets.toml`: `sakura.`・`kero.`・`char*.seriko.defaultsurface`・`balloon.dontmove`（ゴースト）・`sakura.`・`kero.`・`char*.balloon.dontmove`（シェル）は absent・持ち主なし。`sakura-script.toml` の `\![lock,balloonmove]`・`\![unlock,balloonmove]` も absent・持ち主なし。
  - `extra-character-windows`（いつか）の brief は、最初の面の決め打ちを「見つけた穴」として書いていた＝ここで引き取る。

## Desired Outcome

- 起動・ゴーストの切り替え・シェルの切り替え・リロードの後に、各スコープの窓が descript の既定の面で出る。窓の大きさを測るときも同じ面で測る。書かれていなければ今と同じ（本体 0・相方 10）。
- `balloon.dontmove,true`（ゴースト）とシェルの `*.balloon.dontmove`、`\![lock,balloonmove]` の間は、利用者がバルーンをドラッグしても動かない。`\![unlock,balloonmove]` で戻る。台本の `\![move]` や窓の追従でバルーンが動くのは止めない。
- 網羅台帳の行が実物と合う。

## Approach

- 既定の面は配置の設定と同じ所で descript から読み、起動の資産（`assets.rs`）と大きさの測り（`measure.rs`）の決め打ちを、その値へ置き換える。値が面として無いときの扱い（今の決め打ちへ落とすか）を要件で決める。
- ドラッグを止める決まりは、スコープごとの「動かせない」の値（descript の段の優先＋台本の lock）を 1 か所で持ち、バルーンのドラッグの入口で見る。

## Scope

- **In**: `sakura.`・`kero.`・`char*.seriko.defaultsurface`（読み取りはスコープ 0・1。`char*` は窓が無い今は読むだけ）・起動・切り替え・リロードの最初の面と大きさの測り・`balloon.dontmove`（ゴースト）・`sakura.`・`kero.`・`char*.balloon.dontmove`（シェル）・`\![lock,balloonmove]`・`\![unlock,balloonmove]`・記録・決定論のテスト・網羅台帳。
- **Out**: プロパティ `currentghost.scope(ID).seriko.defaultsurface` の読み書き（`currentghost-property-tree`）・3 人目からの窓（`extra-character-windows`）・バルーンの既定の面 `*.balloon.defaultsurface`（`balloon-canon-residue`）・右クリックメニューの「バルーンの移動」の項目（`enableballoonmovebutton.caption`）。

## Boundary Candidates

- descript の読み取り（配置の設定）
- 最初の面と大きさの測り（`emo2_boot/assets.rs`・`placement/measure.rs`・切り替えの資産）
- バルーンのドラッグの入口と、台本の lock の受け手

## Out of Boundary

- 面の合成とアニメーション
- 台本で動かすバルーンの位置（`\![move]`・`balloonoffset`）

## Upstream / Downstream

- **Upstream**: なし。
- **Downstream**: `pasta-in-windows-verification`・`extra-character-windows`（3 人目からの窓の最初の面に使う）・`mcp-reload`（リロードの後の最初の面）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `placement-measure-bake-once`（`placement/measure.rs`・`emo2_boot/assets.rs`＝同じウェーブに置かない）・`extra-character-windows`（同じ 2 ファイル・最初の面の穴をここへ渡した）・`window-link-directives`（`placement/config.rs`・`drag_follow.rs` の近く）・`balloon-canon-residue`（`placement/config.rs`）・`mcp-reload`（リロードの道）。

## Constraints

- `placement/mod.rs`（915 行）・`emo2_boot/mod.rs`（916 行）・`drag_follow.rs`（937 行）は 1,000 行に近い＝足す分は兄弟の新しいファイルへ。
- 1 フレーム遅らせて面を差し替える解は取らない（最初から既定の面で組む）。
- 読めない値（数でない・面が無い）は記録を残す。

## 2026-10-18 棚卸㉗の見立て（main 2d86dbff）

- 触るファイル: `crates/areka/src/placement/config.rs`（687）・`crates/areka/src/emo2_boot/assets.rs`（547）・`placement/measure.rs`（450）・`emo2_boot/switch_assets.rs`（253・シェルの切り替え）・`placement/follow/drag_follow.rs`（937・入口の数行）・`emo2_boot/consumer_ledger.rs`（960・lock の受け手の 1 行）・新しい受け手と決まりのファイル（印: 新）・網羅台帳 `assets.toml`・`sakura-script.toml`・`doc/COMPAT_ARCHITECTURE.md` §8。
- 規模: 6〜9 タスク（既定の面 3〜4・動かさせない 3〜4・記録 1）。
- 先に要るもの: 働きの上では無し。ファイルの上では `placement-measure-bake-once`（v0.1.0・`measure.rs`・`assets.rs`）と同じウェーブに置かない。
- 優先度の区分: A（開発者の指示＝v0.1.0 のゴール・ぱすた）。
- 版: v0.1.0。
- 要件定義のモデル: Opus（先例＝配置の設定の descript の段の優先・バルーンのドラッグの入口）。
- 要件の段の議題:
  1. **ゴーストの `balloon.dontmove` の意味**: ukadoc はゴーストの側を「バルーン位置調整の標準設定を OFF にする」と書き、シェルの側の既定がこれを指す。ドラッグを禁止する値と読んでよいか。SSP で測る（ぱすたのバルーンを SSP でドラッグして動かないか）。
  2. **既定の面が無いとき**: descript の番号の面がシェルに無いときに、今の決め打ち（0・10）へ落とすか、その番号のまま空の窓にするか。SSP で測る。
  3. **`char*.seriko.defaultsurface` の既定**: ukadoc は 10 と書く（相方と同じ）。3 人目からの窓が無い今は読むだけ。`extra-character-windows` へ値を渡す形だけ作るか。

## 2026-10-22 棚卸㉘の再測定（main 52b014de）
- 前提の変化: ✅ `mcp-reload`（10-22）の着地で、最初の面の値の使われ方が分かった。シェルは最初の `\s` まで窓に出さない決まり（`crates/areka/src/emo2_boot/frame/attach.rs` の「defect #5」の注記）なので、起動の直後に面 0 の絵が見えることは今は無い。最初の面の値（`emo2_boot/assets.rs` 411 行目の決め打ち）が効くのは 2 つ＝⑴ 窓の大きさの測り（`placement/measure.rs` 43・190〜205 行目）＝面 0 の大きさで窓が置かれる ⑵ ゴーストの読み直しで控えた面が読み直したシェルに無いときの戻り先（`emo2_boot/shell_carry.rs` の `decide`・`frame/attach.rs` 285 行目）。読み直しの後は前の面が持ち越される（`mcp-reload` 要件 3.8）。受け手の表 `emo2_boot/consumer_ledger.rs` は 472 行、`emo2_boot/mod.rs` は 787 行に減った。
- 触るファイル: `crates/areka/src/placement/config.rs`（687）・`emo2_boot/assets.rs`（547）・`placement/measure.rs`（450）・`emo2_boot/switch_assets.rs`（253）・`placement/follow/drag_follow.rs`（937・入口の数行・足す分は兄弟へ）・`consumer_ledger.rs`（472・lock の 1 行）・新しい受け手と決まりのファイル（新）・網羅台帳 `assets.toml`・`sakura-script.toml`・`doc/COMPAT_ARCHITECTURE.md` §8。
- 規模: 6〜9 タスクのまま。
- 先に要るもの: 働きの上ではなし。同じウェーブに置けない v0.1.0 の相手: `window-link-directives`（`config.rs`・`drag_follow.rs`）・`placement-measure-bake-once`（`measure.rs`。案 A なら `assets.rs` も）・受け手の表に行を足す `stay-on-top-directives`・`script-small-tags`・`mcp-strict-errors`（案 2）・`script-security-level`（1 本ずつ）。後の版: `extra-character-windows`・`balloon-canon-residue`（`config.rs`）・`window-snap`（`drag_follow.rs`）・`multi-ghost-runtime`・`shell-companion-balloon`（`switch_assets.rs`）。依存を足さない。静かな机・前後の全体テストの比べは要らない。議題 1・2 の測りは開発者の機械の SSP、実機のドラッグは排他テストの机。
- 優先度の区分: A（開発者の指示＝v0.1.0 のゴール・ぱすた）。
- 要件定義のモデル: Opus（先例＝配置の設定の descript の段の優先・バルーンのドラッグの入口）。
- 分割の案: なし。
- 見つけた穴・古くなった記述:
  - 本文 Problem の「areka は面 0 を出す＝起動の一瞬がずれて見える」は古い（上）。崩れの本体は、窓の大きさと置き場所が面 0 の絵で決まること（ぱすたの面 0 は「ずれた小さい絵」）。
  - 本文 Desired Outcome の「リロードの後に既定の面で出る」は、ゴーストの読み直しでは前の面の持ち越しが先で、既定の面は戻り先だけ、と書き直す。本文 Downstream の `mcp-reload` は着地済み＝この spec は戻り先の値を descript から渡すだけになった。
  - 本文 Constraints の `emo2_boot/mod.rs`（916 行）は 787 行、`consumer_ledger.rs`（960 行）は 472 行。
