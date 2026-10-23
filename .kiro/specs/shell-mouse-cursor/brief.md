# Brief: shell-mouse-cursor

> 2026-10-18 棚卸㉗で起票（開発者の指示「もう一つの目標は、ゴースト「ぱすた」がSSPと同じレベルで動くこと、です。」＝v0.1.0 のゴール）。証拠はこの棚卸のぱすたの抜けの調べと、ゴースト「窓際のぱすたさん」の `surfaces.txt`（読むだけ・`sakura.cursor` ブレスで顔・胴など 20 の当たり判定に `system:hand`）。`shell-tooltip` の Out「マウスカーソルの差し替え」を、ツールチップとは別の spec として切り出した。正典は ukadoc MCP で 2026-10-18 に引き直した。

## Problem

- **利用者**: キャラクターの触れる所にマウスを乗せても、カーソルの形が変わらない。ぱすたは体の当たり判定の上で手の形になり、「触れられる」と分かる作り。areka では矢印のまま。
- **シェルの作者**: `surfaces.txt` のカーソルの定義（`sakura.cursor` ブレス）と、ゴーストの descript のカーソル（`cursor`・`mousecursor.*`）が効かない。

## Current State

- **正典（ukadoc・要旨）**:
  - `surfaces.txt` のカーソルの定義（ブレスの中の行）: `mouseup*,当たり判定ID,ファイル名`（当たり判定の上に乗ったとき・既定 `system:finger`、当たり判定名が `bust` のときだけ `system:hand`）・`mousedown*`（押したとき・既定 `system:finger`、`bust` だけ `system:grip`）・`mouserightdown*`（右を押したとき・2.6.14）・`mousehover*`（しばらく動かさないとき・既定 `system:arrow`）・`mousewheel*`（ホイールのとき・既定 `system:arrow`）。ファイル名は `.cur`・`.ani`、または `system:arrow`・`cross`・`finger`・`grip`・`hand`・`help`・`move`・`no`・`text`・`wait` ほか。
  - ゴーストの descript の `cursor`／`mousecursor`（操作できる所・既定 `system:finger`）・`mousecursor.arrow`・`grip`・`hand`・`text`・`wait`（2.5.41〜2.6.11）。
  - プロパティ `currentghost.seriko.cursor.scope(ID).mouse????list(当たり判定名).path` ほか（[SET 有効]・相対パスはシェル → ゴーストのフォルダの順に探す）・`currentghost.mousecursor.*`。
- **areka**:
  - `surfaces.txt` の読み手は、知らない見出しのブレスを記録なしに捨てる（`crates/areka-parsers/src/shell/decode.rs` の `dispatch_block`）。2 つ目の転記の前例は `shell/boxes.rs`・`shell/undrawn.rs`。
  - wintf の窓の手続きは `WM_SETCURSOR` を受けるが、診断の十字（環境変数 `WINTF_CROSSHAIR=1` のときだけ）しか描かない（`crates/wintf/src/ecs/window_proc/mouse_move.rs` の `WM_SETCURSOR`・602 行）。
  - マウスが動いたときに当たり判定の名前を引く道は在る（`OnMouseMove` を送る `crates/areka/src/input_events/mod.rs`・当たり判定を引く `crates/areka/src/emo2_boot/hit_region.rs`）。どの当たり判定の上に居るかを窓ごとに覚えて、変わった時を知る所は無い。
  - 網羅台帳: `assets.toml` の `mouseup*` ほかのカーソルの行・ゴーストの `cursor`・`mousecursor.*` は absent・持ち主なし。`property.toml` のカーソルのプロパティは語の表だけ（持ち主 `currentghost-property-tree`）。

## Desired Outcome

- キャラクターの当たり判定の上に乗ると、`surfaces.txt` の定義のカーソル（`.cur`・`.ani`・`system:*`）になる。押したとき・右を押したときも定義どおり。当たり判定の外では矢印に戻る。
- ゴーストの descript の `mousecursor.*` が既定の形を差し替える。
- 定義の行が読めないときは記録が残る（黙って捨てない）。
- 網羅台帳の行が実物と合う。

## Approach

- カーソルの定義は `surfaces.txt` の 2 つ目の転記として読む（`boxes.rs` と同じ型・`decode.rs` に触らない）。スコープ × 当たり判定名 × 場面（乗る・押す・右を押す・止まる・ホイール）の表にして、シェルの読み込みに乗せて運ぶ。
- **窓ごとの「今どの当たり判定の上に居るか」**をマウスの移動のたびに覚える所を 1 つ作る。カーソルは名前が変わったときに選び直し、wintf の窓に「出すカーソル」を持たせて `WM_SETCURSOR` で出す（十字の診断と同じ口）。
- 同じ「今どの当たり判定の上に居るか」を、`mouse-click-wheel-events`（`OnMouseEnter`／`OnMouseLeave`）と、後の `shell-tooltip` が使う。先に着地した側が作り、後の側は使うだけにする。

## Scope

- **In**: `surfaces.txt` のカーソルの定義（`mouseup*`・`mousedown*`・`mouserightdown*`・`mousehover*`・`mousewheel*`）・`system:*` の形・`.cur`・`.ani`（シェル → ゴーストのフォルダの順に探す）・ゴーストの descript の `cursor`・`mousecursor.*`・窓ごとの当たり判定の追跡・wintf の `WM_SETCURSOR` の口・記録・決定論のテスト・実機の確かめ・網羅台帳。
- **Out**: ツールチップ（`shell-tooltip`）・バルーンの上のカーソル（バルーンの descript の `cursor`・`mousecursor.*`）・プロパティでの差し替え（`currentghost-property-tree`）・`OnMouseEnter`／`OnMouseLeave` の送出（`mouse-click-wheel-events`）。

## Boundary Candidates

- カーソルの定義の転記（areka-parsers の shell）
- 窓ごとの当たり判定の追跡（areka の入力の層）
- カーソルを出す口（wintf の窓の手続き）

## Out of Boundary

- 当たり判定の形（`collisionex-regions`）
- ドラッグの判定

## Upstream / Downstream

- **Upstream**: なし。
- **Downstream**: `shell-tooltip`（窓ごとの当たり判定の追跡を使う）・`pasta-in-windows-verification`。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `mouse-click-wheel-events`（同じ `input_events/` と当たり判定の追跡＝同じウェーブに置かない・先に着地した側が追跡を作る）・`shell-tooltip`（同じ転記の型・`hit_region.rs`）・`collisionex-regions`（`shell/boxes.rs`・`hit_region.rs`）・`hit-id-pass`（`hit_region.rs`）・`instant-to-interrupt-ticks`（wintf の `mouse_move.rs`）。

## Constraints

- クリックを通す窓（`WS_EX_TRANSPARENT`）は `WM_SETCURSOR` を受けない＝当たる所でだけ形が変わるのは正しい。
- カーソルのファイルが読めないときは記録を残し、既定の形へ落とす。
- 1 フレーム遅らせて形を変える解は取らない（`WM_SETCURSOR` はその場で答える）。
- `crates/areka/src/input_events/mod.rs`（575 行）・wintf の `mouse_move.rs`（602 行）に足す分は兄弟の新しいファイルへ。

## 2026-10-18 棚卸㉗の見立て（main 2d86dbff）

- 触るファイル: 新しい転記 `crates/areka-parsers/src/shell/cursor.rs`（印: 新）と `shell/mod.rs`・表を運ぶ `crates/areka-emo-present/src/shell_target.rs`（757）・新しい追跡のファイル `crates/areka/src/input_events/hover_region.rs`（印: 新・`mouse-click-wheel-events` と共有）と `input_events/mod.rs`（575・数行）・`crates/wintf/src/ecs/window_proc/mouse_move.rs`（602・数行）と新しいカーソルの口のファイル（印: 新）・ゴーストの descript の読み取り（配置の設定か新しいファイル）・網羅台帳 `assets.toml`。
- 規模: 6〜9 タスク。
- 先に要るもの: 働きの上では無し。ファイルの上では `mouse-click-wheel-events`（`input_events/` の追跡）と同じウェーブに置かない。順は `mouse-click-wheel-events` が先（ぱすたの × に効く）。
- 優先度の区分: A（開発者の指示＝v0.1.0 のゴール・ぱすた）。
- 版: v0.1.0。
- 要件定義のモデル: Opus（先例＝wintf の `WM_SETCURSOR` の診断の口・`shell/boxes.rs` の 2 つ目の転記・wintf のツールチップの口）。
- 要件の段の議題:
  1. **既定の形を全部のゴーストに効かせるか**: 正典の既定は「当たり判定の上は `system:finger`」。カーソルの定義を書いていないゴーストでも、当たり判定の上で指の形になる＝今の全部のゴーストの見え方が変わる。推し: 正典どおり既定も効かせる。SSP で測る（定義の無いゴーストで指になるか）。
  2. **`mousehover*` と `mousewheel*`**: 「しばらく動かさない」の時間と、ホイールの後いつ戻すか。SSP で測る。ホイールの送出は `mouse-click-wheel-events`。
  3. **ドラッグ中の形**: `bust` の押下の `system:grip` を、ドラッグのあいだ保つか。

## 2026-10-22 棚卸㉘の再測定（main 52b014de）
- 前提の変化: 10-18 から触るファイルは動いていない（wintf の `crates/wintf/src/ecs/window_proc/mouse_move.rs` 602 行〔`WM_SETCURSOR` は今も診断の十字だけ〕・シェルを運ぶ `crates/areka-emo-present/src/shell_target.rs` 757・シェルの読み手 `crates/areka-parsers/src/shell/decode.rs` 597〔知らない見出しのブレスは今も記録なしに捨てる〕・当たり判定を引く `crates/areka/src/emo2_boot/hit_region.rs` 167・`crates/areka/src/input_events/mod.rs` 575）。当たり判定の追跡のファイル `input_events/hover_region.rs` はまだ無い。
- 触るファイル: 棚卸㉗の見立てのとおり（新しい転記 `crates/areka-parsers/src/shell/cursor.rs`〔新〕と `shell/mod.rs` 72・`shell_target.rs`・追跡の `input_events/hover_region.rs`〔新・`mouse-click-wheel-events` と共有〕と `input_events/mod.rs` の数行・`mouse_move.rs` の数行と新しいカーソルの口のファイル〔新〕・ゴーストの descript の読み取り・台帳 `assets.toml`）。
- 規模: 6〜9 のまま。
- 先に要るもの: 働きの上で無し。ファイルの上で `mouse-click-wheel-events` の後（追跡を先に作る側・ぱすたの × に効く）。同じウェーブに置けない相手（v0.1.0）: `mouse-click-wheel-events`（`input_events/`）・`placement-measure-bake-once` が案 B（`shell_target.rs` の読み込みの口を変える）を選んだとき。`offline-surface-dump` は `shell_target.rs` の `load_shell_target` を呼ぶだけ＝0。1 本だけの席は取らない。実機の確かめ（カーソルの形を見る）は画面のロックを外した机で、議題 1・2 は SSP で測る。
- 優先度の区分: A のまま（開発者の指示＝v0.1.0 のゴール・ぱすたの体の当たり判定で手の形）。
- 要件定義のモデル: Opus のまま（先例＝wintf の `WM_SETCURSOR` の診断の口・`shell/boxes.rs` の 2 つ目の転記）。
- 分割の案: なし。
- 見つけた穴・古くなった記述: なし。
