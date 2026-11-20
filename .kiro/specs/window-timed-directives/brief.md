# Brief: window-timed-directives

> 2026-11-19 `/kiro-discovery` で起票。出どころは、ghost_dev の「ぱすた開発」セッションが開発者の発注で書いた台本「席替え」（今の SSP だけで遊べるノベル）。開発者「同じ台本を SSP でも areka でも動かす」「areka 環境に実装の起票投げておいてね」。開発者は 2026-11-19 に、この形（1 本にまとめる・v0.4.0）を承認した（「推奨で起票」）。

## Problem

- **ゴーストの作者**: 窓を「すべらせて移す」「ふっと消して現す」「大きくして戻す」を、正典のタグで書いても areka では動かない。
  - `\![move,…,--time=500]` は、時間をかけずに最終位置へすぐ飛ぶ。
  - `\![set,scaling,…]` と `\![set,alpha,…]` は、時間つきでも時間なしでも、何も起きない。
- **台本「席替え」**（ぱすたが、遊ぶ人のデスクトップを教室に見立てて、窓ごと席を移る）は、この 3 つを全部使う。SSP 2.9.34 では動く（透明度と大きさの `--time` は「ノベルゲーム(pasta)」セッションが実機で確かめた）。
- **持ち主が居ない**: 時間つきの指令は、`sakura-time-directives` が範囲から外して roadmap の覚え書きへ戻した（D 群・「消費する者が現れたら引き取る」）。`element-animation-easing` の議題 1 も「土台を誰が持つか」のまま残っていた。今回、消費する者が現れた。

## Current State

起票のときの実測（main `e382b09b0`）:

- **`\![move]`**: 位置は実装済み（完了 `areka-P0-move-canon`）。名前付きの形と旧い位置の形、基準 `screen`・`primaryscreen`・`me`・`global`・スコープの番号、参照点を解く。**時間だけが残り**で、`time>0` は最終位置へすぐ置いて `warn!`（`move_arg_substituted`）を残す（`crates/areka-runtime/src/move_cue_args.rs` の `TimedMoveImmediate`・網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml` の `\![move]` は `degraded`）。
- **`\![moveasync]`・`\![moveasync,cancel]`**: 受け手が無い（台帳は `absent`・持ち主は空）。
- **`\![set,scaling]`**（3 つの形）・**`\![set,alpha]`**: 受け手が無い（台帳は `absent`）。台帳の持ち主の欄は `sakura-time-directives` のままだが、同 spec は範囲から外している（同 brief の 84・116 行）。
- **倍率の土台は在る**: 表示の倍率は「アプリの倍率 × DPI の倍率」の掛け算で、アプリの倍率の席は 1 倍に固定のまま空いている（`crates/areka-emo-present/src/scale.rs` の `ScalePolicy`）。絵の拡大は GPU が行い、当たり判定は押された点を倍率で割ってから見る（`crates/areka-emo-compose/src/scale.rs`・`hit.rs`）。
- **倍率を知らせる道も在る**: `currentghost.scope(ID).scaling` と `OnShellScaling`（完了 `shell-scale-linkage`）。
- **台本の時刻**: 1 本の台本は、読んだ時に時刻の表に直す。時刻が進むのは、字と待ちのタグのぶんだけ（`crates/areka-sakura/src/compile.rs`）。時間つきの指令のぶんを進める口は無い。
- **正典（ukadoc を 2026-11-19 に引き直した）**:
  - `\![move]`・`\![moveasync]`: 「`--time=移動時間`」「移動時間はミリ秒単位。移動時間を省略、もしくは0を指定した場合は瞬時に移動」。`\![moveasync]` は「指定座標まで非同期移動」「動作途中に `\![moveasync,cancel]` で停止することができる」（`https://ukatech.shillest.net/ukadoc-spec/list_sakura_script.html#bang-moveasync.note`）。
  - `\![set,alpha,数値,オプション]`: 「0～100で透過度を変更する。0で完全透明、100で不透明。100超は100にクランプされる。負値は値変更なしの再描画のみ」「ゴースト終了まで有効」。`--time=ミリ秒`（省略時・負値は 0）・`--wait`（完了まで待つ。省略時は非同期・SSP 2.7.52 以降）（同 `#bang-set-alpha`）。
  - `\![set,scaling,倍率]`・`\![set,scaling,横倍率,縦倍率]`・`\![set,scaling,横倍率,縦倍率,オプション]`: 「単位%。ユーザの設定する倍率を100％とした倍率。負数の指定が可能で、-100でその軸方向に反転する」「ゴースト終了まで有効」。オプションは `--time`・`--wait`（同 `#bang-set-scaling`・`#bang-set-scaling-3`）。
  - どれも、変化の速さの形（イージング）は書いていない。

## Desired Outcome

- `\![move]` の `--time`（と旧い形の時間の欄）で、窓が指定の時間をかけて最終位置へ移る。台本は、移り終わるまで先へ進まない。
- `\![moveasync]` で、台本を止めずに窓が移る。`\![moveasync,cancel]` で途中で止まる。
- `\![set,scaling,…]` で、そのスコープの絵の倍率が変わる（時間なし・`--time`・`--wait`）。倍率は「遊ぶ人の倍率 × タグの倍率」で、`currentghost.scope(ID).scaling` と `OnShellScaling` に出る。
- `\![set,alpha,…]` で、そのスコープの絵の透明度が変わる（時間なし・`--time`・`--wait`）。
- 倍率と透明度は、ゴーストが終わるまで残る（正典どおり）。
- 動き方は等速。形を選ぶ書き方（`--easing`）は `element-animation-easing` が後から添える。
- 台本「席替え」が、SSP と同じ見た目で動く。
- 読めない値は、トークを止めずに既定へ置き換えて記録する（`\![move]` の今の決まりと同じ）。

## Approach

- **1 本にまとめる**（2026-11-19 開発者の承認）。3 つの指令は「窓の単位の値を、時間をかけて変える」という同じ仕事で、触る場所が同じ（小さい起票は触る場所でまとめる決まり）。`element-animation-easing` に入れる案は採らなかった（覚書の段の重い spec に混ざる）。
- **時刻の表に長さを書く**: 待つ形（`\![move]` の時間・`--wait`）は、読んだ時に長さが分かるので、台本の時刻をそのぶん進める（`sakura-time-directives` の D 群の設計: 「envelope duration へ転写し offset を進める・moveasync は 0」）。待たない形は、始めるだけで時刻を進めない。
- **値は台本の時計で動かす**: 途中の値は、始まりの時刻と長さから毎フレーム計算する（1 フレーム遅らせない・時刻は正確に）。途中に次の指令が来たら、そのときの途中の値から続ける。
- **倍率**: 空いている「アプリの倍率」の席に、タグの倍率を掛ける。`shell-balloon-scale-setting`（遊ぶ人の倍率）と同じ席なので、掛け算の順と持ち方を合わせる。
- **段**: v0.4.0（開発者 2026-10-17「アニメーションに絡むのは、dola の改造後」）。置く波は次の棚卸で決める。

## Scope

- **In**: `\![move]` の時間（名前付きの `--time` と旧い形の時間の欄）・`\![moveasync]`・`\![moveasync,cancel]`・`\![set,scaling]` の 3 つの形・`\![set,alpha]`・`--time` と `--wait`・台本の時刻を進める所・途中の値の計算（等速）・倍率と透明度をゴーストの終わりまで持つ所・`currentghost.scope(ID).scaling` と `OnShellScaling` への反映・読めない値の置き換えと記録・`check_script` の表（`consumer_ledger.rs`）への登記・網羅台帳の持ち主の付け替えと `doc/COMPAT_ARCHITECTURE.md` の行・決定論のテスト。
- **Out**: 変化の形（`--easing`＝`element-animation-easing`）・遊ぶ人の倍率の設定（`shell-balloon-scale-setting`）・演劇の舞台（ウィンドウモード）の中のアクターの位置・大きさ・透明度（pasta の取り決め `novel-areka-contract` の `play` の命令と転換＝`shell-multi-compose`）。**舞台を開かない演劇**（デスクトップに浮いたまま 1 行ずつ進む）は、位置・大きさ・透明度を正典のタグで出す、と 2026-11-19 に開発者が決めた（pasta の骨子 `final-model-2026-11-19.md` の 8 章・議題 5）ので、その分はこの spec が受ける（In）。前の文面「演劇では正典の `--time` を使わない」は、議論の途中の案で、決定ではなかった・`\![sound,wait]`・`\![wait,syncobject]`（C 群の残り）・SERIKO のコマ間をつなぐ書き方。

## Boundary Candidates

- 引数の読みと、時刻の表へ長さを書く所（`crates/areka-sakura/src/compile.rs`・`crates/areka-runtime/src/move_cue_args.rs`）。
- 時間をかけた移動（`crates/areka-runtime/src/move_cue*.rs`・`crates/areka-placement/`）。
- 倍率の席と透明度（`crates/areka-emo-present/src/scale.rs`・表示の経路）。
- 倍率の知らせ（`crates/areka-runtime/src/scope_scale.rs`・`crates/areka/src/ghost_session_scope_scale.rs`）。

## Out of Boundary

- 窓の位置の決まり（参照点・基準・吸い付き）は `areka-P0-move-canon` のまま変えない。
- 絵の拡大の式と当たり判定の逆算（`ScaleRatio`）は変えない。

## Upstream / Downstream

- **Upstream**: v0.2.0 の時刻の土台（`dola-interrupt-ticks` → `talk-anchor-interrupt-ticks`）。完了済みの `areka-P0-move-canon`・`shell-scale-linkage`・`areka-P0-window-link-directives`（組にした窓は一緒に動く）。
- **Downstream**: `element-animation-easing`（`--easing` を添える。同 brief の議題 1 は「先に別の spec を立てる」で決着）・`shell-balloon-scale-setting`（表示倍率 × タグの倍率）・`currentghost-property-others`（途中の値を返すかどうか）・台本「席替え」。

## Existing Spec Touchpoints

- **Extends**: なし（完了済みの `areka-P0-move-canon` の「時間の部分」の残りを引き取る）。
- **Adjacent**: `sakura-time-directives`（台本の時刻のほかの指令・同じ `compile.rs`）・`element-animation-easing`・`shell-balloon-scale-setting`・`parallel-visual-script`（並走の側に `\![move]` を書けるかは同 brief の議題）・`talk-fast-forward`（早送りのときの時間つきの指令の扱い）・`scope-currentmonitor-property`（席替えの台本が一緒に使う）。

## Constraints

- 区分 A（開発者が起票を頼んだ）・v0.4.0。
- 規模の見立て: M（12〜16 タスク）。Opus。
- 意味は ukadoc から輸入する。ukadoc で読めない点は、議題にする前に SSP の MCP で測る（下の議題）。
- 1 フレーム遅らせる解は取らない。時刻は正確に扱う。
- 失敗を黙って捨てない（読めない値・効かなかった指令は記録に出す）。メッセージボックスは出さない。
- 台本「席替え」の本文は ghost_dev のリポジトリの物で、areka のリポジトリへ写さない。

## 要件の段で決めること（議題の候補）

1. **負の倍率（反転）**: 正典は「-100 でその軸方向に反転」。絵の裏返しと当たり判定の逆算が要る。この spec に入れるか、追跡の spec に分けるか。
2. **透明度と当たり判定**: 窓を薄くしたとき、クリックの当たり（今は絵の α ≥ 0.5）に透明度を掛けるか。SSP で測る。
3. **待つ `\![move]` の途中のクリック**（早送り・中断）で、移動を最終位置へ飛ばすか、続けるか。
4. **バルーン**: 倍率と透明度をバルーンにも掛けるか（SSP で測る。倍率は `shell-balloon-scale-setting` の「バルーンも連携する」と合わせる）。
5. **途中の値**: 変化の途中に `currentghost.scope(ID).scaling` を読んだら、途中の値か終わりの値か。`OnShellScaling` を途中で送るか。
6. **ゴーストの切り替え・終了・`\![reset]`** のときの、動いている途中の指令と、残っている倍率・透明度の扱い。
