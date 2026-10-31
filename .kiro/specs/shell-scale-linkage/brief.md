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

## 2026-10-31 棚卸㉛の再測定（main 4068422f）
- **前提の変化**: ✅ `stay-on-top-directives`（10-31）が受け手の表 `crates/areka/src/emo2_boot/consumer_ledger.rs` に 2 行を足し、529 行・登記 40 件になった（手書きの数 `consumer_ledger_tests.rs` 298 行目も 40・見本 `consumer_ledger_agreement_tests.rs` 417 行）。同じ spec の受け手 `crates/areka/src/emo2_boot/window_state_cue.rs`（新・160 行）が、新しい受け手を `emo2_boot/mod.rs`（828）と `runtime_boot.rs`（237）に 1 行ずつ足す一番新しい先例。✅ `mcp-strict-errors`（10-30）は表に行を足さず、strict の記録（`emo2_boot/strict_judges.rs`）が表の答えを写すようにした＝`(get,property)` の 1 行で strict の「知らない命令」も消える。✅ `window-work-area-clamp`（10-30）は `crates/areka/src/ghost_session.rs`（940・上限まで 60）に記憶から戻した位置の寄せの印を数行足しただけで、拡大率の替わりの所 `emo2_boot/frame/dpi.rs`（587）・descript の鍵の読み `placement/descript_window_keys.rs`（182）・`placement/config.rs`（689）には触れていない。プルの口の登録は今 `ghost_session.rs` 96 行目。`crates/areka-kanade/src/schedule/events.rs` 913・`placement/mod.rs` 970（上限まで 30）。3 つの値・`OnShellScaling`／`OnBalloonScaling`・`syncscale` の読みは今もどこにも無い。
- **値の出し方**: 拡大率と DPI は変わる時点が分かる（拡大率の相）＝写しへ押し込む形が先（開発者の決まり「変わった時点が分かるならプッシュ」）。プルの口に載せると登録の 1 行が `ghost_session.rs` に入り、`placement-measure-bake-once` の案 A と重なる。
- **触るファイル**: `\![get,property]` の受け手（新・`emo2_boot/` の下＝`emo2_boot/mod.rs` と `runtime_boot.rs` に 1 行ずつ）・`consumer_ledger.rs`（529・`(get,property)` の 1 行と担当の種類・手書きの数 40→41・説明の一覧・「11 の受け口」の数）と見本 `consumer_ledger_agreement_tests.rs`・`frame/dpi.rs`・`schedule/events.rs`（送ってよいイベントの表に 2 語）・`placement/descript_window_keys.rs` と、読みの呼び手 `placement/mod.rs`（足すなら 1〜2 行に留める）・3 つの値を載せる所・網羅台帳 4 本の行。
- **規模**: M（10〜14 タスク）のまま。上限 20 の内。
- **先に要るもの**: 働きの上ではなし。受け手の表に行を足す `script-small-tags`・`balloon-communicatebox` と 1 本ずつ（行は避けられない＝`get` は表に無い名前・`mcp-strict-errors` は行を足さずに着地した）。`balloon-communicatebox` とは `emo2_boot/mod.rs`・`runtime_boot.rs`・`schedule/events.rs` でも重なる。`placement-measure-bake-once` は案 A なら `placement/mod.rs`・`emo2_boot/mod.rs` で重なり、案 B なら重なり 0。依存を足さない。議題 2 の測り（`\![get,property]` の往復の参照と出どころ・`OnShellScaling` を送る場面）は開発者の機械の SSP の MCP。実機は拡大率の違う 2 つの画面のある机で、排他テストの机を持って回す。静かな机・前後の全体テストの比べは要らない。
- **優先度の区分**: A（開発者の指示＝v0.1.0 のゴール「ぱすたが SSP と同じに動く」・ぱすたの検証の申し送り）。
- **要件定義のモデル**: Opus（値の口は `mcp-scope-rect-property`、イベントの往復は `mcp-kanade-tools` の参照つきのイベント、descript の読みは `descript-window-keys`、受け手の足し方は `stay-on-top-directives` の形に乗る。議題 1 の「倍率に何を出すか」は 2026-07-18 の開発者の決め〔拡大率への追従が基本〕で枠が決まり、骨格を変える読みではない）。
- **分割の案**: なし。
- **見つけた穴・古くなった記述**:
  - 返しのイベントの名前はゴーストが決める（ぱすたは `OnPastaShellScale`・`OnPastaScaleCheck`）＝固定の表に載せる形は取れない。名前の自由なイベントの扱いを要件で決める（`events.rs` の、名前の自由な選択肢のイベントの判定と同じ形）。
  - 起票の見立ての「`crates/areka-parsers` の descript の鍵」は要らない見込み（上の `descript_window_keys.rs`）。本文 Upstream の `mcp-scope-rect-property` は着地済み。
  - 網羅台帳 `assets.toml` のシェルの `sakura.`・`kero.`・`char*.balloon.syncscale` の 3 行は持ち主が空（ゴーストの `balloon.syncscale` の行だけこの spec）。着地のときにこの spec を書く。
