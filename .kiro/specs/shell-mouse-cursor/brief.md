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

## 本 spec からの申し送り（2026-10-23・`areka-P0-pasta-in-windows-verification` のタスク 6）

- 出どころ: `areka-P0-pasta-in-windows-verification` の検証の記録 `verification.md`（5 節の崩れた所の表。件の番号は表の番号）と、SSP の測りの記録 `ssp-measurement.md`（SSP 2.9.23・書庫の sha256 `ad473d5d…a43a6`＝`pasta.toml` 1.0.0）。撮った物と記録は git の外の `<w>\target\pasta-verify\`（本走行 run1。`logs\`・`shots\run1\`・SSP の撮った物は `ssp\`）・`<w>\target\pasta-verify-alt\`（補いの走り alt2。縦書きのバルーンを根の `balloon\` へ写した置き方）・`<w>\target\pasta-verify-rerun\`（直しの後の回し直し rerun1。alt2 と同じ置き方）に在る（`<w>` は `C:\home\maz\git\areka\.claude\worktrees\areka-p0-pasta-windows-d4ed7a`）。記録の時刻は世界標準時（`Z`）のまま。日本時間は 9 時間足す。
- 開発者の決め（2026-10-23）「原則として、すべての申し送りが0.1.0に解決するのであれば承認する。申し送り漏れはダメですよ。」。この spec は v0.1.0 なので、下の件はこの spec の中で片づける。
- 着地した後の確かめは、ぱすたの手順書 `doc/pasta-real-run.md`（5 節の 7 段の表）で回す。
- **件 16（当たり判定ごとのマウスの形）**: 当たり判定ごとのマウスの形（ぱすたの身体は `system:hand`）を、areka は出さない見込み。走行ではマウスの形を読み取っていない（`shots\run1\notes.md` の段 3）。run1 では身体が映らなかった（`mouse-click-wheel-events` の件 12）ので、身体の上の形は、その spec の着地の後でないと確かめられない。
  - 原因: シェルの `surfaces.txt` の `sakura.cursor` ブレス（`mouseup*`）を読む所が無い（網羅台帳で absent・この brief の Current State）。
- **SSP の基準・測りの答え**（`ssp-measurement.md` 2 節の段 3・3 節の 6 行目）: カーソルの形の読み取り（`ssp\stage3-cursor.txt` の 5 点）で、×（-84,258）とボール（-615,805）は `HAND`（Windows の標準の指さしの形・ハンドル `0x1001F`）、身体（-410,450 と -300,640）は `custom`（同じハンドル `0x46160A0D`）、窓の白地（-700,1000）は `ARROW`。シェルの `sakura.cursor` ブレスには `mouseup0,顔,system:hand` から `mouseup19,胸左,system:hand` までの 20 行があり、身体の当たり判定 20 個すべてに `system:hand` を当てている。`閉じる`（×）と `セキュリティボール` の行は無く、`kero.cursor` などのブレスも無い。読み方: 身体の `custom` は、シェルが指定した `system:hand` を SSP が自分の手の形のカーソルとして出したもの。× とボールは指定が無いので ukadoc の既定の `system:finger` になり、標準の指さしの形（`HAND`）として読めた。形そのものの絵は撮っていない。
- 網羅台帳の宛先: 2026-10-23 に 2 行の `owner` をこの spec にした（`verification.md` 10 節の「直したことと残り」）。
- **着地で変わるはずの所**（手順書の段 3）: 当たり判定ごとにマウスの形が替わる（身体は手の形・× とボールは指さしの形・当たりの外は矢印）。

## 2026-10-24 棚卸㉚の再測定（main 9318cdb0）
- **前提の変化**: `mouse-click-wheel-events`（#380）が着地し、窓ごとの当たり判定の追跡を作った＝`crates/areka/src/input_events/hover_region.rs`（102 行・状態 `HoverState`〔スコープ・位置・当たり判定の名前〕と、出入りを決める純関数 `plan_hover`）。状態の置き場所は `MouseWiring.hover`（`input_events/mod.rs`・鍵は窓の entity）。読む口はまだ無い（あちらの design「読み手が居ないので作らない」）＝本 spec が足す。押下の控え（`click.rs`・ボタンの区別つき）とホイール（`wheel.rs`）も同じ見張りの系に在り、`mousedown*`・`mouserightdown*`・`mousewheel*` の材料に使える。
- **追跡は毎 tick の系**（`char_watch.rs` 234 行・`Input` の段で `dispatch_pointer_events` の前）で、マウスの移動のたびではない。`WM_SETCURSOR` は移動の直後にその場で来るので、`HoverState` の名前だけで形を選ぶと、当たり判定をまたいだ直後に前の名前の形が 1 tick 残る＝Constraints の「1 フレーム遅らせない」とぶつかる。`WM_SETCURSOR` の中で当たり判定をその場で引くか、名前が変わった tick で `SetCursor` を直に呼ぶかを要件・設計の段で決める。
- **触るファイル**: 新しい転記 `crates/areka-parsers/src/shell/cursor.rs`〔新〕と `shell/mod.rs`（72）・`crates/areka-emo-present/src/shell_target.rs`（855）・読む口 `input_events/hover_region.rs`（102）か `char_watch.rs`（234）・wintf の `crates/wintf/src/ecs/window_proc/mouse_move.rs`（632・`WM_SETCURSOR` は 30 行目で今も診断の十字だけ）と新しいカーソルの口のファイル〔新〕・ゴーストの descript の読み取り・台帳 `assets.toml`（持ち主が本 spec の行は 7204 行）。`input_events/mod.rs`（611）は新しい mod の 1 行を足すときだけ。
- **規模**: 6〜9 のまま（追跡が出来た分は減るが、`WM_SETCURSOR` との時の合わせ方が足される）。
- **先に要るもの**: 働きの上で無し（待っていた `mouse-click-wheel-events` は着地）。同じウェーブに置けない相手（v0.1.0）: `balloon-scroll-arrows`（どちらも `input_events/mod.rs` に mod の行を足す）＝本 spec が新しいコードを `char_watch.rs`／`hover_region.rs` の子（`#[path]`）に置いて `mod.rs` に触らないと約束すれば 0。`seriko-pattern-calls` とは、向こうが `areka-parsers/src/shell/` に新しいファイルを足すときだけ（`shell/mod.rs`）。`placement-measure-bake-once` が `shell_target.rs` の読み込みの口を変える案を選んだとき。1 本だけの席・依存の追加は無い。実機の確かめ（カーソルの形を読む）は画面のロックを外した机。既定の形と止まったときの形は SSP で測る。
- **優先度の区分**: A のまま（開発者の指示＝v0.1.0 のゴール・申し送りの件 16）。
- **要件定義のモデル**: Opus のまま（先例＝wintf の `WM_SETCURSOR` の診断の口・`shell/boxes.rs` の 2 つ目の転記・`HoverState` の読み口）。tick と `WM_SETCURSOR` の合わせ方は、今の窓の手続きと tick の上での選び方で、新しい並行の約束ではない。
- **分割の案**: なし。
- **見つけた穴・古くなった記述**: ⑴ Current State の「どの当たり判定の上に居るかを窓ごとに覚えて、変わった時を知る所は無い」と Approach の「覚える所を 1 つ作る」は古い（`hover_region.rs` が在る・本 spec は読むだけ）。⑵ Constraints の行数（575・602）は古い（611・632）。⑶ 版の決まっていない `mouse-hover-event`（10-24 起票・`OnMouseHover`）も「しばらく動かさない」を測る＝本 spec の `mousehover*` と同じ時間を使う見込み。後に来る側が先の側の判定を使う。⑷ 後の版の `shell-hover-animation` も追跡を使う（あちらの design にも書かれた）。
