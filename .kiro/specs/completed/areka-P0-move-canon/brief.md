# Brief: areka-P0-move-canon

> 2026-10-17 `/kiro-discovery` で起票（開発者「新しい move 形式を含む周辺仕様を ukadoc から取り込み、areka の次のバージョン（+0.0.1）として、最優先のロードマップ設定にしてください」→ 版は v0.0.3 に足す〔開発者の選択〕・「とりあえず 1 本、2 本目は 0.1.0 に回す。アニメーションに絡むのは、dola の改造後」）。着手の区分は **A（開発者が起票）**。版は **v0.0.3**（`emily-ghost-verification` と並ぶ v0.0.3 の残り）。

## Problem

検体 emo2 の 2026-10-17 版（ghost_dev `ed87661`・pasta.dll v0.4.0・PR#326 で `vendors/sample_ghost/emo2.nar` を差し替えた）の `ghost/master/dic/menu.pasta` が、メニューの「エモの位置調整」で名前付きの形の `\![move]` を使い始めた。

```
＊Onエモの位置調整選択
　　　エモ：＠通常　\1\![move,--X=0,--Y=0,--base=0,--base-offset=left.bottom,--move-offset=right.bottom]
```

意図はゴーストの注釈どおり「エモの右下をむらさきの左下にそろえる（窓の端どうしで合わせるのでシェル倍率に追従する）」。ところが areka はこの行で**エモを動かさない**。利用者がメニューから位置調整を選んでも何も起きない。emo2 は areka の既定のゴーストで、配布物にも同梱しているので、v0.0.3 の利用者がまず踏む。

## Current State

- `crates/areka/src/emo2_boot/move_cue.rs`（683 行）
  - `parse_move_directive`（:234）は `--` で始まる引数を 1 つでも見つけると `Err(MoveDegradation::NamedForm)` を返す（:240-242）＝**名前付きの形は記録付きで飛ばす**。旧い位置の形（`X,Y,time,base,base-offset,move-offset`）だけを解釈する。
  - **座標の計算 `resolve_move_target_position`（:386-429）は `base_offset`／`move_offset` を一度も読まない**。式は両窓とも常に basepos（幅÷2・下端）を基準にした `x' = base.x + bp(base).x + k·dx − bp(target).x` に固定されている（:409-426）。旧い形の emo2（`\1\![move,-353,,,0,base,base]`）がたまたま `base.base` だったので表に出なかった。名前付きの形を読めるようにするだけでは、emo2 の `left.bottom`／`right.bottom` は**間違った位置へ動く**。
  - 基準の語 `screen`・`primaryscreen`・`me`・`global` は型（`MoveBase`・:106-117）はあるが、解決は数値のスコープだけ（`apply_move_directive`・:585-594 が他は warn して false）。他のゴーストの名前の基準と `--option` には型が無い。
  - time>0 は補間せず最終位置へすぐ置き、`TimedMoveImmediate` を記録する。
  - 窓を動かすのは `placement/follow/window_move.rs` の `move_window_to` だけで、`Anchored` と位置の記憶には触れない（`char-position-save-on-exit` の決まり「台本の移動の指示で動いた窓は書かない」）。dx・dy に拡大率 k を掛けるのは意図した SSP との差（`doc/COMPAT_ARCHITECTURE.md` §8 :155）。
- `doc/COMPAT_ARCHITECTURE.md` §8 :131-135 の `\![move]` の 5 行（裸の `base`・名前付きの形の縮退・`screen` ほか 4 語の縮退・time>0 は即時・basepos は既定の値だけ）は「emo2 は positional 形のみ使用」を根拠にしている＝**前提が崩れた**。
- 網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml`: `move`（:1773）は degraded・持ち主 `areka-P0-sakura-time-directives`（同 brief :116 は「付け替える」と書く）。`set,position`（:3266）・`reset,position`（:2932 前後）・`execute,resetwindowpos`（:1466）は absent・持ち主なし。
- 登録された受け手は `emo2_boot/consumer_ledger.rs`（943 行）:347-356 の `move`・`(set,zorder)`・`(reset,zorder)`（と bind）だけ。`\![set,position]`・`\![reset,position]`・`\![execute,resetwindowpos]` は運ばれるだけで誰も受けない。areka には「シェル位置初期化」のメニューも `OnResetWindowPos` も無い。

### ukadoc の正典（2026-10-17 に読んだ・節「\![move]、\![moveasync]について」は MCP の索引に無いので https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html から）

- `\![move]`（`ukadoc:list_sakura_script:_!_move_`）。新しい形は「新仕様：(SSP 2.3.85以降)」で `\![move,--X=,--Y=,--time=,--base=,--base-offset=,--move-offset=,--option=]`。どのキーも省略でき、順は自由、キー名の大小は区別しない（`--x=` も可）。旧い形（「旧仕様」）は廃止と書かれていない＝両方を受ける。
- X／Y: 「どちらかを省略するか、"fix"と指定した場合は、その座標を保持する」。負の値も可。
- time: ミリ秒。省略か 0 で瞬間移動。
- base: 「省略時screen」。語は `screen`（今キャラ窓がいるモニタの左上）・`primaryscreen`（主モニタの左上）・`global`（システム座標・Windows では primaryscreen と同じ）・スコープの番号・他のゴーストの sakura 名か \0 名とスコープ ID・`me`（動くキャラ自身）。
- base-offset／move-offset: `X.Y`。X は left・right・base・center、Y は top・bottom・base・center。base は surfaces.txt の `point.basepos`。既定は「省略時left.top」（move-offset にも掛かるかは本文から読み切れない＝議題）。
- 組み合わせの制限: 「screen指定の場合はbase、global指定の場合はright,bottom,centerが利用できない」。
- option: 繰り返し書けて、新しい形だけ。値は `ignore-sticky-window` の 1 語（「\![set,sticky-window]での連携設定を無視して単独で動かす」）。
- `\![set,position,x,y,スコープID]`: 「指定スコープをXY座標に固定する。(ドラッグ等を不可能にする)」。解除は `\![reset,position]`。版の注記なし。
- `\![execute,resetwindowpos]`（2.7 RC1）: 「メニューの「シェル位置初期化」と同じ働き」。対のイベント `OnResetWindowPos`（2.7 RC1）は、応答が無いときにこのタグと同じ処理になる。

## Desired Outcome

- emo2 の 2026-10-17 版で「エモの位置調整」を選ぶと、エモ（`\1`）の窓の右下がむらさき（`\0`）の窓の左下にそろう。拡大率（DPI・シェル倍率）が変わってもそろう。
- `\![move]` の名前付きの形と旧い位置の形が、同じ型（`MoveDirective`）へ解釈される。キーの大小・順・省略・`fix`・`--option` の繰り返しが ukadoc どおり。
- 参照点 `base-offset`／`move-offset` の 9 点＋`base` が座標の計算に入る（旧い形にも効く）。旧い emo2 の `\1\![move,-353,,,0,base,base]` の着地は変わらない（決定論テストが固定している）。
- 基準 `screen`・`primaryscreen`・`global`・`me` が解決される。他のゴーストの名前の基準は 1 体しか動かない今は解決先が無い＝記録付きで飛ばす（`multi-ghost-runtime` の後で引き取る）。制限の組み合わせは記録付きで扱う。
- `\![set,position]`／`\![reset,position]` でキャラクターの窓を固定・解除できる（固定の間はドラッグで動かない）。`\![execute,resetwindowpos]` と `OnResetWindowPos` で窓を既定の位置へ戻せる。
- COMPAT §8 の `\![move]` の行と台帳の行（`move`・`set,position`・`reset,position`・`execute,resetwindowpos`）が新しい実装に合う。台帳の `move` 行の持ち主をこの spec へ付け替え、時間の部分は残りとして書く。

## Approach

瞬間の移動の正典化に絞った 1 本。時間とアニメーション（`--time` の補間・`\![moveasync]`・cancel）は dola の改造（v0.2.0）の後へ残す（開発者「アニメーションに絡むのは、dola の改造後」）。窓どうしの連携（`\![set,alignmenttodesktop]`・`\![set/reset,sticky-window]`）は 2 本目の `window-link-directives`（v0.1.0）へ回す。

1. **解釈**: 名前付きの形の読み手を兄弟のファイルへ出し（move_cue.rs は 683 行・1,000 行の決まり）、旧い形と同じ `MoveDirective` へ落とす。`--option` は語彙として持つ（`ignore-sticky-window` は連携がまだ無いので効き目なし＝2 本目が意味を持たせる）。
2. **計算**: `resolve_move_target_position` に参照点を入れる。両窓の矩形から `base-offset` の点（基準）と `move-offset` の点（動く窓の合わせる点）を取り、`move-offset` の点が `基準の点 + k·(dx,dy)` に来るよう置く。`base` の参照点は今の `BaseposResolver`（既定の basepos）を使う。
3. **基準**: `screen`・`primaryscreen`・`global`・`me` の矩形を決める（モニタの矩形か作業領域かは議題）。
4. **固定と初期化**: `\![set,position]`／`\![reset,position]` を受け手に足し、固定の間はドラッグを止める。`\![execute,resetwindowpos]` と `OnResetWindowPos` は「既定の位置へ並べ直す」既存の道（起動の並べ方）を使う。
5. **実機**: emo2 の 2026-10-17 版で位置調整を選び、MCP の `get_log`・窓の位置で確かめる（拡大率 100% と 200%）。

## Scope
- **In**: `\![move]` の名前付きの形の解釈・参照点 9 点＋`base` の計算・基準 `screen`／`primaryscreen`／`global`／`me` の解決・制限の組み合わせと他のゴーストの名前の扱い（記録付き）・`--option` の語彙・`\![set,position]`／`\![reset,position]`・`\![execute,resetwindowpos]`・`OnResetWindowPos`・COMPAT §8 と網羅台帳の行の改め・emo2 の 2026-10-17 版での実機の確かめ
- **Out**: `--time` の補間と台本の時間を進めること・`\![moveasync]`・`\![moveasync,cancel]`（dola の改造〔v0.2.0〕の後・覚え書き「`sakura-time-directives` から戻した C・D 群」の D 群）・`\![set,alignmenttodesktop]`・`\![set/reset,sticky-window]`・`--option=ignore-sticky-window` の効き目（`window-link-directives`・v0.1.0）・宣言された `point.basepos` を面ごとに使うこと（`surfaces-basepos`）・`check_script` の引数の診断（`check-script-arg-checks`）

## Boundary Candidates
- 名前付きの形の読み手（純粋な関数・決定論テスト）
- 参照点と基準の矩形の計算（`resolve_move_target_position`・純粋な関数）
- 受け手の追加（`consumer_ledger.rs` に `set,position`・`reset,position`・`execute,resetwindowpos`）と窓の固定（ドラッグの道）
- `OnResetWindowPos` を SHIORI へ送り、応答が無ければ既定の位置へ並べ直す

## Out of Boundary
- 時間つきの移動と非同期の移動（dola の改造の後）
- 窓どうしの連携と吸着の台本の指令（`window-link-directives`）
- 3 人目以降の窓（`extra-character-windows`）・複数ゴースト（`multi-ghost-runtime`）

## Upstream / Downstream
- **Upstream**: ✅ `sakura-dialogue-tags`（`\![move]` の受け口と `MoveDirective`）・✅ `char-position-save-on-exit`（台本で動いた窓は記憶に書かない決まり）・✅ `scope-zorder-pinning`（`\![set,zorder]` の受け手の型）・PR#326（emo2 の 2026-10-17 版の検体）
- **Downstream**: `window-link-directives`（v0.1.0・sticky-window の組で move を一緒に動かす・`ignore-sticky-window` に意味を持たせる）・`check-script-arg-checks`（`parse_move_directive` を呼ぶ＝名前付きの形が通るようになる）・`surfaces-basepos`（`base` の参照点を面ごとの basepos へ）・`element-animation-easing`（`--time` に `--easing` を添える）・覚え書きの D 群（時間つきの `\![move]`・`\![moveasync]`）

## Existing Spec Touchpoints
- **Extends**: なし（完了 spec は直さない）。台帳の `move` 行の持ち主を `sakura-time-directives` からこの spec へ付け替える（同 brief :116 の申し送りどおり。時間の部分は D 群の残りとして note に書く）
- **Adjacent**: `check-script-arg-checks`・`surfaces-basepos`・`element-animation-easing`・`window-snap`（どれも `move_cue.rs` か窓の位置の道を分け合う＝この spec の後に着手）・`emily-ghost-verification`（同じ v0.0.3・触る場所は `areka-nar`・`install/`・`sample-ghost-kit`＝重なり 0 の見込み）

## Constraints
- 拡大率 k を dx・dy に掛ける今の決まり（COMPAT §8 :155）は保つ。参照点の矩形は物理 px（`WindowPos.size`）から取り、論理 px を経由しない。
- 台本で動いた窓は位置の記憶に書かない（`char-position-save-on-exit`）。`\![set,position]` の固定も記憶には書かない（要件の段で確かめる）。
- `move_cue.rs`（683 行）と `consumer_ledger.rs`（943 行）は 1,000 行の決まりに近い＝新しい中身は兄弟のファイルへ出す。
- 実機の根と検体はワークツリーの `target\` の下だけ。emo2 は絶対パスで起こす。

## 議題の種（要件の段で決める）
1. `move-offset` の既定: ukadoc の「省略時left.top」が `base-offset` だけに掛かるのか両方か（今の areka は両方 left.top）。
2. `screen`・`primaryscreen` の矩形はモニタ全体か作業領域か（ukadoc は「モニタの左上」）。
3. 制限の組み合わせ（`screen` と `base`・`global` と `right`／`bottom`／`center`）を断るか、記録して近い意味で動かすか。
4. `\![execute,resetwindowpos]` の「既定の位置」は起動の並べ方と同じでよいか・位置の記憶を消すか。
