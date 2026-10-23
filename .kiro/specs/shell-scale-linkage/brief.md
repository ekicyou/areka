# Brief: shell-scale-linkage

> 2026-10-23 `areka-P0-pasta-in-windows-verification` のタスク 5.3 で `/kiro-discovery` により起票（本 spec からの申し送り（2026-10-23））。開発者の決め（同日のチャット）「0.1.0までに解決しない場合、独立して起票せよ」「原則として、すべての申し送りが0.1.0に解決するのであれば承認する。申し送り漏れはダメですよ。」。ぱすたが使う倍率の連動のうち、ぱすたが要る分だけを 1 本にまとめた。版は **v0.1.0**（ゴールの 2 つ目「ぱすたが SSP と同じに動く」）。

## Problem

- **ゴーストの作者（ぱすた）**: ぱすたは倍率と DPI を台本で聞き、窓の絵（面 10・11・12）を選び、画面をまたいでドラッグされた後や倍率が変わった後に、窓の位置と重なりの順を出し直す。areka では、聞く台本の `\![get,property,…]` に受け手が無く、倍率の変わりのイベントも送らず、ゴーストの descript の `balloon.syncscale` も読まない。どれも黙って捨てられる。
- 今は areka の DPI 追従でシェルとバルーンの倍率が揃うので見た目は崩れていないが、ぱすたの選ぶ窓の絵と付け直しの合図は SSP と違う道を通る（聞いた答えが返らないので、起動の値 10 のまま）。

## Current State

- **ぱすたの使い方**（取り寄せた書庫を展開した木 `target\pasta-verify\root\ghost\pasta-in-windows`・読むだけ）:
  - `ghost/master/dic/tategaki.pasta:53` `\![get,property,OnPastaShellScale,currentghost.scope(0).scaling,currentghost.scope(0).currentmonitor.dpi]`＝毎秒のイベントで付け直すとき・辞書の読み込みの直後に聞く。答えの `OnPastaShellScale` が「シェルの拡大率 ÷ 画面の倍率」で窓の絵を 10・11・12 から選び、変わったら付け直しの印を付ける。
  - `ghost/master/dic/tategaki.pasta:101` `\![get,property,OnPastaScaleCheck,currentghost.scope(0).scaling,currentghost.balloon.scope(0).scaling]`＝`OnShellScaling`・`OnBalloonScaling` の応答。ずれていれば利用者に揃えてもらう台詞を出す。`OnShellScaling` は付け直しの印も付ける。
  - `ghost/master/descript.txt:13` `balloon.syncscale,true`（縦書きのバルーンは窓の絵に重ねてあるので、バルーンの倍率をシェルに連動させる）。
- **証拠**: `verification.md` 10 節の突き合わせの表の `\![get,property,…]`（absent・`property-query-channels`）・`OnShellScaling`・`OnBalloonScaling`（absent・持ち主なし）・ゴースト `balloon.syncscale`（absent・持ち主なし）の 3 欄。同 10 節「台帳に行の無いもの」のぱすた独自のイベント `OnPastaShellScale`・`OnPastaScaleCheck`。
- **SSP の基準**（`ssp-measurement.md` 2 節の段 5・3 節の表の 3 行目）: 150% の画面から 200% の画面へ移っても、`currentghost.scope(0).scaling` は `150,150`、`currentghost.scope(0).currentmonitor.dpi` は 192、ぱすたの大きさは 831×1026 のまま。`OnShellScaling` は届かなかった（倍率が変わっていないため）。`\![get,property]` の往復と `OnShellScaling` を送る場面は測っていない＝要件の段で SSP の MCP で測る。
- **正典（ukadoc）**:
  - `\![get,property,イベント名,プロパティ名,…]`: 指定したイベントが起き、Reference0 から順に値が入る（`SenderType: property`）。
  - `currentghost.scope(ID).scaling`「現在のゴーストのスコープIDの実効拡大率。」・`currentghost.balloon.scope(ID).scaling`・`currentghost.scope(ID).currentmonitor.dpi`。
  - `OnShellScaling`「シェル倍率が変更された際に発生。」・`OnBalloonScaling`（Reference0／1＝横の後／前・2／3＝縦の後／前の百分率）。
  - `balloon.syncscale,true`「バルーン拡大縮小率のゴーストとの連動を強制的にONにする(trueのみ有効)。」（SSP 2.7 RC1）。シェルの `sakura.`・`kero.`・`char*.balloon.syncscale` も同じ。
- **areka**: `\![get,property]` は汎用の運び手で受け手の表まで届き、誰も拾わない（`crates/areka/src/emo2_boot/consumer_ledger.rs`）。参照つきのイベントを送る口は ✅ `mcp-kanade-tools` が作った。`SenderType` を SHIORI へ運ぶのは `script-security-level`（v0.1.0・C10）。倍率は DPI 追従（倍率＝画面の DPI ÷ 作者の基準の DPI）で、シェルとバルーンは同じ倍率で動く。利用者が倍率を変える道（`shell-balloon-scale-setting`）は v0.4.0。
- **持ち主の今**: `\![get,property]` は `property-query-channels`（いつか）・`scope(ID).scaling` と `currentmonitor.*` は `currentghost-property-others`（いつか）・`balloon.scope(ID).scaling` は `currentghost-property-tree`（v0.3.0）・`balloon.syncscale` は `shell-balloon-scale-setting`（v0.4.0）の棚卸㉗の足し分・`OnShellScaling`／`OnBalloonScaling` は持ち主なし（`shell-balloon-scale-setting` が Out に挙げ、`dpi-realign-remembered-chain` が「要件の段で持ち主を決める」と書いていた）。

## Desired Outcome

- ぱすたの 2 つの `\![get,property,…]` に、指定のイベント（`OnPastaShellScale`・`OnPastaScaleCheck`）が Reference に値を持って返り、ぱすたが SSP と同じ道で窓の絵を選ぶ。
- `currentghost.scope(ID).scaling`・`currentghost.balloon.scope(ID).scaling`・`currentghost.scope(ID).currentmonitor.dpi` が、MCP の `get_property` とゴーストの道の両方で読める。
- シェルとバルーンの倍率が変わったとき（areka では拡大率の違う画面へ移ったとき）に `OnShellScaling`・`OnBalloonScaling` を送るかを SSP の意味に照らして決め、送るなら正典の Reference で送る。ぱすたの付け直しが、ドラッグの終わりと倍率の変わりの両方で走る。
- `balloon.syncscale,true` を読み、バルーンの倍率がシェルに連動していることを値として持つ（今は DPI 追従で揃っている。利用者の倍率の設定との兼ね合いは `shell-balloon-scale-setting`）。

## Approach

- `\![get,property]` の受け手を 1 つ立て、sylphya の読み手で値を引き、参照つきのイベントを送る口（✅ `mcp-kanade-tools`）で返す。出どころ `SenderType: property` は `script-security-level` の運び手に 1 つ足す。受け手は名前に縛らない（載っている値なら何でも返る）。`\![set,property]`・`%property[…]`・`\![embed]` などほかの道は `property-query-channels` に残す。
- 3 つの値は `mcp-scope-rect-property` が作る動く値の口に載せる。
- **areka の倍率の意味を先に決める**: SSP の `scaling` は利用者のシェル倍率で、画面を移っても変わらない。areka は DPI 追従が基本の設計（2026-07-18 開発者・SSP には合わせない）。`scaling` に「DPI の倍率 × 表示倍率」を出すか「表示倍率だけ（今は 100）」を出すかで、ぱすたの選ぶ窓の絵と `OnShellScaling` を送る場面が変わる＝要件の段の議題 1。

## Scope

- **In**: `\![get,property,イベント名,プロパティ名,…]` の受け手（`SenderType: property`）・3 つのプロパティの値・`OnShellScaling`／`OnBalloonScaling` の送る場面と Reference・ゴーストとシェルの descript の `balloon.syncscale`（`sakura.`・`kero.`・`char*.`）の読み取りと値・受け手の表に 1 行と検査の見本・網羅台帳の行（`sakura-script.toml` の `\![get,property]`・`shiori.toml` の 2 イベント・`assets.toml` の syncscale の 4 行・`property.toml` の 3 行）・決定論のテスト・ぱすたの実機の確かめ（200% と 150% の画面をまたぐドラッグ）。
- **Out**: `\![set,property]`・`%property[…]`・`\![embed]`・照会の道の残り（`property-query-channels`）・利用者が倍率を変える道と設定（`shell-balloon-scale-setting`・v0.4.0）・`\![set,scaling]` のタグ・描画メソッド `scaling`・ほかの `currentghost.*`。

## Boundary Candidates

- `\![get,property]` の受け手と参照つきのイベント（kanade の口を使う）
- 倍率と DPI の値（動く値の口に載せる）
- 倍率の変わりのイベント（`emo2_boot/frame/dpi.rs` の DPI の替わりの所）
- descript の `syncscale` の読み取り（配置の設定の近く）

## Out of Boundary

- 画面をまたぐドラッグの置き方（`dpi-realign-remembered-chain`）
- 窓の組の連れ動き（`window-link-directives`）

## Upstream / Downstream

- **Upstream**: `mcp-scope-rect-property`（動く値の口）・`script-security-level`（`SenderType` の運び手）・✅ `mcp-kanade-tools`（参照つきのイベントの口）。
- **Downstream**: `pasta-in-windows-verification` の回し直し・`property-query-channels`（残りの道が同じ受け手の表と口を使う）・`shell-balloon-scale-setting`（表示倍率が変わったときも同じイベントで送る・`syncscale` の強制を利用者の切り替えに効かせる）。

## Existing Spec Touchpoints

- **Extends**: `property-query-channels`（`\![get,property]` を本 spec へ移した＝同 brief に 1 行）・`currentghost-property-tree`（`balloon.scope(ID).scaling`）・`currentghost-property-others`（`scope(ID).scaling`・`scope(ID).currentmonitor.dpi`）・`shell-balloon-scale-setting`（`balloon.syncscale` の読み取りと 2 つのイベント）＝それぞれの brief に 1 行。
- **Adjacent**: `dpi-realign-remembered-chain`（`OnShellScaling` の持ち主を決める議題があった＝本 spec に決まった・同 brief に 1 行。ぱすたの付け直しと詰め直しの順を確かめる相手）・`window-link-directives`・`descript-window-keys`（`placement/config.rs`）。

## Constraints

- DPI 追従の基本の設計は変えない（SSP の「画面が替わっても大きさを変えない」には合わせない・`verification.md` 5 節の 8）。
- 1 フレーム遅らせる解は取らない。イベントは倍率が決まった時点で送る。
- 受け手の表 `consumer_ledger.rs` に行を足す spec は 1 本ずつ（同じウェーブに置かない）。

## 起票時の見立て（2026-10-23・ブランチ `claude/areka-p0-pasta-windows-d4ed7a`）

- 触るファイル: `crates/areka-ghost/src/`（新しい get の受け口）・`crates/areka/src/emo2_boot/{consumer_ledger.rs, frame/dpi.rs}`＋新しいファイル・kanade の参照つきのイベントの口（✅ `mcp-kanade-tools` の形）と `schedule/events.rs` の許可の表（2 つのイベント）・`crates/areka/src/placement/config.rs`（descript の読み取り）・`crates/areka-parsers` の descript の鍵・網羅台帳 4 本。
- 規模: M（10〜14）。
- 優先度の区分: A（開発者の指示＝v0.1.0 のゴール・ぱすた）。段: その他。
- 要件定義のモデル: Opus（口は `mcp-scope-rect-property`、イベントの往復は `mcp-kanade-tools` の先例に乗る）。
- 議題（答えで作業が変わるものだけ）: ⑴ areka の `scaling` に何を出すか（上の Approach）。⑵ SSP の MCP で測る: `\![get,property]` の往復の Reference と `SenderType`・`OnShellScaling` を送る場面（シェルの倍率の設定を変えたとき・画面を移ったとき）。

## 2026-10-23 棚卸㉙の再測定（main aa8da8a7）
- **前提の変化**: 起票（10-23）の枝はそのまま main `aa8da8a7` に入った。上流の ✅ `script-security-level`（10-23）が出どころの語を作り、`SenderType: property` の語はもう在る（`crates/areka-talk/src/origin.rs` の `SenderWord::Property`・72 行目、文字列への写しは 84 行目）＝Approach の「運び手に 1 つ足す」は、語を足すのでなく、この語を受け手から使うだけになった。参照つきのイベントの口（✅ `mcp-kanade-tools`）は kanade の知らせ `crates/areka-kanade/src/msg.rs` の `references` の欄。もう 1 つの上流 `mcp-scope-rect-property`（動く値の口）は未着手。
- **触るファイル**: `\![get,property]` の受け手（新）と受け手の表 `crates/areka/src/emo2_boot/consumer_ledger.rs`（496・1 行と見本の表）・倍率の替わりの所 `crates/areka/src/emo2_boot/frame/dpi.rs`（508）・kanade の送ってよいイベントの表 `crates/areka-kanade/src/schedule/events.rs`（888・表 `ALLOWED_EVENT_IDS` は 115 行目。ぱすたが名前を決める返しのイベント `OnPastaShellScale` などは、名前の自由な選択肢のイベントの判定〔286 行目〕と同じ形の扱いが要る）・descript の `balloon.syncscale` の読み取り（配置の設定 `crates/areka/src/placement/config.rs` 689）・動く値の口に 3 つの値を載せる所（`mcp-scope-rect-property` が作るファイル）・網羅台帳 4 本の行。
- **規模**: M（10〜14 タスク）のまま。上限 20 の内。
- **先に要るもの**: 働きの上で `mcp-scope-rect-property`（動く値の口）の着地。同じウェーブに置けない v0.1.0 の相手＝受け手の表に行を足す `stay-on-top-directives`・`script-small-tags`・`descript-window-keys`・`balloon-communicatebox`・`mcp-strict-errors`（案 2）（1 本ずつ）・`descript-window-keys`（`placement/config.rs` も）・`dpi-realign-remembered-chain`（`frame/dpi.rs`）・`mouse-click-wheel-events`（`schedule/events.rs`）・`mcp-scope-rect-property`（sylphya のファイル）。依存を足さない。議題 2 の測り（`\![get,property]` の往復の参照と出どころ・`OnShellScaling` を送る場面）は開発者の機械の SSP の MCP。実機は拡大率の違う 2 つの画面のある机で、排他テストの机を持って回す。静かな机・前後の全体テストの比べは要らない。
- **優先度の区分**: A（開発者の指示＝v0.1.0 のゴール「ぱすたが SSP と同じに動く」・ぱすたの検証の申し送り）。
- **要件定義のモデル**: Opus（口は `mcp-scope-rect-property`、イベントの往復は `mcp-kanade-tools` の参照つきのイベントの先例に乗る。議題 1 の「倍率に何を出すか」は、2026-07-18 の開発者の決め〔拡大率への追従が基本・SSP の大きさを変えない動きには合わせない〕で枠が決まり、骨格を変える読みではない）。
- **分割の案**: なし。
- **見つけた穴・古くなった記述**:
  - 起票の見立ての触るファイルの「kanade の参照つきのイベントの口と `schedule/events.rs` の許可の表（2 つのイベント）」のうち、返しのイベントの名前はゴーストが決める（ぱすたは `OnPastaShellScale`・`OnPastaScaleCheck`）。固定の表に載せる形は取れない＝名前の自由なイベントの扱いを要件で決める。
  - 起票の見立ての「`crates/areka-parsers` の descript の鍵」は、どのファイルかが書かれていない。今の descript の読み取りは配置の設定 `placement/config.rs` の `build_placement_config`（122 行目）が受け取る形＝ここに足すなら `crates/areka-parsers` には触らずに済む見込み。
  - 網羅台帳 `assets.toml` のシェルの `sakura.`・`kero.`・`char*.balloon.syncscale` の 3 行は持ち主が空（ゴーストの `balloon.syncscale` の行だけこの spec）。着地のときにこの spec を書く。
  - 受け手の表に行を足す v0.1.0 の spec は、この spec を入れて 5 本（`stay-on-top-directives`・`script-small-tags`・`descript-window-keys`・`balloon-communicatebox`。`mcp-strict-errors` の案 2 なら 6 本）。表の登記の数を手で書いて固定するテスト（`crates/areka/src/emo2_boot/consumer_ledger_tests.rs` 294〜299 行目の 36）と見本の表（`consumer_ledger_agreement_tests.rs` の `samples`）を全員が書き換えるので、1 ウェーブに 1 本ずつになる。
