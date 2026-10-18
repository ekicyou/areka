# Brief: areka-P0-stay-on-top-directives

> 2026-10-18 棚卸㉗で起票（開発者の指示「もう一つの目標は、ゴースト「ぱすた」がSSPと同じレベルで動くこと、です。」＝v0.1.0 のゴール）。証拠はこの棚卸のぱすたの抜けの調べと、ゴースト「窓際のぱすたさん」の辞書（読むだけ・`dic/tategaki.pasta` の「重ね順」が `\![reset,zorder]\![set,zorder,s0,b0,s1]\1\v\0` を出す）。正典は ukadoc MCP で 2026-10-18 に引き直した。

## Problem

- **ゴーストの作者**: 台本の `\v` と `\![set,windowstate,stayontop]` で「常に手前に表示」にできない。`\![set,windowstate,!stayontop]` で戻すこともできない。どれも黙って捨てられる。
- ぱすたは重なりを指定した直後に `\1\v` を出す（ぱすたの辞書のコメント: SSP ではシェルの読み直しの後などに重なりが並べ直されないことがあり、奥の窓を一度並べ直すと手前の窓が揃う）。SSP ではこの `\v` でゴーストが常に手前になる見込みで、areka では何も起きない＝見え方が SSP と違う。

## Current State

- **正典（ukadoc・要旨）**:
  - `\v`: 手前に表示する。この台本の後は常に最前面に出る。ただしスコープどうし（本体側と相方側のどちらが上か）は利用者の操作しだい。ゴースト終了まで有効。
  - `\![set,windowstate,stayontop]`: 同じく常に最前面。ゴースト終了まで有効。
  - `\![set,windowstate,!stayontop]`: 手前に出さない。この台本の後は他の窓と同じに重なる。ゴースト終了まで有効。
  - SHIORI リソース `alwaysstayontopbutton.caption`（「常に手前に表示」の名称・メニューの項目）。
- **areka**:
  - 台本の読み手: 裸の `\v` は腕が無く、「知らないタグ」として `Raw` のまま運ばれ、台本の組み立てで記録つきで捨てられる（`crates/areka-parsers/src/sakura/decode.rs` の `decode_bare`・`decode_passthrough_bare`）。`\![set,windowstate,…]` は汎用の運び手で届くが、受け手の表 `crates/areka/src/emo2_boot/consumer_ledger.rs`（960 行）に受け手が無い（同ファイルが「担当なし」の例に挙げている）。
  - 完了 `ghost-window-zorder` の要件 4.3「ゴースト窓を常時最前面に固定しない」・8.1「既定は常時最前面ではない通常の窓」・8.2「`\v` 相当を後から足せる構造を保つ」・8.3「`\v` などを本フィーチャーでは解釈しない」。
  - wintf は「常に手前の帯」を避ける作りを持つ: 窓が帯の中に居るかを読む `is_window_always_on_top`（`crates/wintf/src/api.rs`）と、帯の中の窓を挿入位置に選ばない判断（`crates/wintf/src/ecs/window/zorder_pair.rs` の `measure_window_is_always_on_top`）。窓を帯へ上げ下げする指令の口は `crates/wintf/src/ecs/window/window_pos/mod.rs` の `zorder_topmost`・`zorder_notopmost` に在る。
  - 網羅台帳 `sakura-script.toml`: `\v`・`\![set,windowstate,stayontop]`・`\![set,windowstate,!stayontop]` は absent・持ち主なし。`shiori.toml` の `alwaysstayontopbutton.caption` は語の表だけ。
  - `tasktray-minimize`（windows-reactor 待ち）の brief は、この 2 つを Out に挙げている（持ち主が居なかった）。

## Desired Outcome

- `\v` と `\![set,windowstate,stayontop]` の後、ゴーストのキャラクターの窓とバルーンの窓が、他のアプリの窓より常に手前に出る。後から現れたバルーンの窓も同じ。
- `\![set,windowstate,!stayontop]` で普通の重なりに戻る。ゴーストの終了（切り替え）で普通に戻る。
- 常に手前のあいだも、バルーンとキャラクターの重なり・`\![set,zorder]` のグループの並びは崩れない。
- 網羅台帳の 3 行が実物と合う。

## Approach

- 裸の `\v` を `\![set,windowstate,stayontop]` と同じ値へ写す（裸の `\+` を `\![change,ghost,random]` へ写す前例＝`decode_bare`）。受け手は `("set", "windowstate")` に 1 つ置き、第 2 引数で `stayontop`・`!stayontop` を分ける。`minimize` は後で `tasktray-minimize` が同じ受け手へ足す。
- 「常に手前」の状態をゴーストに 1 つ持ち、全部の窓を帯へ上げる。持ち主の鎖（`scope-zorder-pinning`）の上では、持たれる窓は持ち主と一緒に帯へ上がる OS の性質を使えるかを先に測る。
- 帯を避ける今の判断（`zorder_pair.rs`）は、「常に手前」でないときだけ働くよう条件をつける。

## Scope

- **In**: `\v`・`\![set,windowstate,stayontop]`・`\![set,windowstate,!stayontop]` の読み取りと受け手・全部の窓を帯へ上げ下げする働き・後から出る窓と組と鎖での保ち方・ゴーストの終了と切り替えで戻す・記録・決定論のテスト・実機の確かめ・網羅台帳と互換の記録。
- **Out**: 右クリックメニューの「常に手前に表示」の項目と `alwaysstayontopbutton.caption`（議題 3）・`\![set,windowstate,minimize]` と最小化（`tasktray-minimize`）・全画面のアプリを見た自動の最小化・次の起動へ持ち越すこと（正典は「ゴースト終了まで」）。

## Boundary Candidates

- 台本の読み手の `\v` の腕（areka-parsers）
- `windowstate` の受け手（`emo2_boot` の新しいファイル＋受け手の表に 1 行）
- 窓を帯へ上げ下げする働き（wintf の窓の指令・組と鎖の判断）

## Out of Boundary

- スコープどうしの並び（`\![set,zorder]`・`zorder-scope-inversion`）
- 窓の位置と大きさ

## Upstream / Downstream

- **Upstream**: 完了 `ghost-window-zorder`（要件 8.2 が後から足せる構造を約束した）・完了 `scope-zorder-pinning`（持ち主の鎖）。
- **Downstream**: `pasta-in-windows-verification`・`tasktray-minimize`（同じ受け手に `minimize` を足す・戻すときに常に手前の状態も戻す）。

## Existing Spec Touchpoints

- **Extends**: なし（完了 spec は直さない。`ghost-window-zorder` の要件 4.3・8.1 は「台本が常に手前を求めたとき」を除く形に読み替え、互換の記録に書く）。
- **Adjacent**: `zorder-scope-inversion`（同じ wintf の鎖と組＝同じウェーブに置かない）・`tasktray-minimize`・受け手の表 `consumer_ledger.rs` に行を足す spec（`window-link-directives`・`script-small-tags`・`descript-window-keys` ほか多数＝1 本ずつ）。

## Constraints

- `consumer_ledger.rs` は 960 行＝受け手の中身は兄弟の新しいファイルへ。
- 1 フレーム遅らせて帯へ上げる解は取らない。帯への上げ下げは指令への応答として済ませる。
- 失敗（OS が帯へ上げられない）は記録を残し、ゴーストを止めない。
- 実機の確かめは画面のロックを外した机で（他のアプリを前に出して確かめる）。

## 2026-10-18 棚卸㉗の見立て（main 2d86dbff）

- 触るファイル: `crates/areka-parsers/src/sakura/decode.rs`（502・`decode_bare` に 1 腕）と兄弟のテスト・`crates/areka/src/emo2_boot/consumer_ledger.rs`（960・1 行）・新しい受け手のファイル（印: 新・`emo2_boot/` の下）・`crates/wintf/src/ecs/window/zorder_pair.rs`（884）・`zorder_chain_apply.rs`（799）・`window_pos/mod.rs`（帯の指令の口を使う）・後から出る窓の作り（`crates/areka/src/placement/spawn.rs` 776 か wintf の窓の作り）・網羅台帳 `sakura-script.toml` の 3 行・`doc/COMPAT_ARCHITECTURE.md` §8。
- 規模: 6〜9 タスク。
- 先に要るもの: 働きの上では無し。ファイルの上では `zorder-scope-inversion`（wintf の鎖と組）と同じウェーブに置かない。順は `zorder-scope-inversion` が先の見込み（鎖の形が決まってから帯へ上げる）。
- 優先度の区分: A（開発者の指示＝v0.1.0 のゴール・ぱすた）。
- 版: v0.1.0。
- 要件定義のモデル: Opus（先例＝`ghost-window-zorder` の要件 8.2 の約束と、wintf が実測で持つ「帯」の性質の記録。並行や時刻の新しい保証も、先例の無い境目も無い）。
- 要件の段の議題:
  1. **SSP で測る**: `\1\v` は相方の窓だけを常に手前にするのか、ゴーストの全部の窓か。バルーンも上がるか。`!stayontop` の後、ゴーストの切り替え・シェルの読み直しの後はどうなるか。
  2. **鎖と帯**: 持ち主の鎖の根を帯へ上げれば持たれる窓も上がるか（Windows の実測）。上がらないなら窓ごとに上げる。
  3. **メニューの「常に手前に表示」**: SSP のメニューにある切り替えを作るか。推し: 作らない（`alwaysstayontopbutton.caption` は「対応しない」と理由を書く）。作るなら `menu/` に触り、`popup-menu-residue`・`popup-menu-shell-look` と重なる。
