# Brief: areka-P0-pasta-in-windows-verification

> 2026-10-18 棚卸㉗で起票（開発者の指示「もう一つの目標は、ゴースト「ぱすた」がSSPと同じレベルで動くこと、です。検体の相談は「ぱすた開発」セッションに問い合わせてください。」＝v0.1.0 のゴール）。検体は「ぱすた開発」セッションの答え（2026-10-18）で「窓際のぱすたさん」（`pasta-in-windows`・`hello-pasta` ではない）に決まった。証拠はこの棚卸の調べ（ぱすたの抜けの調べ・ぱすた開発の答え）と、ゴースト本体（読むだけ・`C:\home\maz\git\ghost_dev\release\pasta-in-windows\pasta-in-windows\`・`C:\home\maz\git\ghost_dev\project\pasta-in-windows\`）。下敷きは完了 `areka-P0-emily-ghost-verification`（要件 8 本・タスク 13〜18 本）。

## Problem

- **開発者**: 自分で作っているゴースト「窓際のぱすたさん」は、SSP 2.9.18 だけで作り、確かめている。areka では一度も動かしたことがない（ぱすた開発「areka では動かしていない・確かめは全部 SSP 2.9.18」）。v0.1.0 のゴールは「ぱすたが SSP と同じレベルで動くこと」。
- **ぱすたは SSP の新しめの機能を多く使う**。縦書きのバルーン（SSP 2.8.80 から）・`\_l[x,y]` で列と枠を並べる台本・バルーン込みの重なりの指定 `\![set,zorder,s0,b0,s1]`（SSP 2.7.34 から）・2 つの窓を重ねて一緒に動かす `seriko.sticky-window,0,1`・着せ替えの多用・element に埋め込んだ動く WebP・当たり判定ごとのマウスの形・自前の色の右クリックメニュー。どれが areka で崩れるかの全部は、まだ誰も見ていない。
- **検体の置き方が決まっていない**。シェルの絵は TH さまからこのゴースト専用にいただいたもので、改変はよいが、別のリポジトリへ置き直してよいかは分からない（ぱすた開発の答え 5）。今の検体の置き場（`vendors/sample_ghost/` に書庫そのものを入れる形）は使えない。

## Current State

- **ゴースト**（2026-10-18・ghost_dev のコミット `e1e0559`）: SHIORI は `pasta.dll` v0.4.0（emo2 と同じ 32 ビットの物）。本体側＝ぱすた・相方側＝「窓」（窓枠の絵を `\1` の面 10 として別の窓に描く）。縦書きのバルーン `pasta_tategaki`（名前 `窓際のぱすたさん縦書き`）をゴーストのフォルダの直下に抱え、`install.txt` に `balloon.directory` は無い。ゴーストの descript は `balloon,窓際のぱすたさん縦書き`・`sakura.seriko.defaultsurface,1000`・`balloon.dontmove,true`・`balloon.syncscale,true`・`homeurl`（GitHub Pages）。
- **書庫**: `ghost_dev` の git に入っている（大きいファイルの別置きではない）。
  - コミット: `e1e05590dbed0a4a4b1afadf55ca896f36d82c14`（2026-10-18 19:50 +0900「起動の挨拶・時報・まばたきの直し・倍率の連動を載せた版」）
  - 場所: `release/pasta-in-windows/pasta-in-windows.nar`（git の中身の印 `175f3c36d3aea85625a48e724b686fda921daf53`）
  - 大きさ: 3,025,198 バイト（68 ファイル）
  - sha256: `04a8e1c19b534b38e39d24f93ac7735c74e186249965b3a0876a893cb41fe35b`
  - コミットを固定した取り寄せ先: `https://raw.githubusercontent.com/ekicyou/ghost_dev/e1e05590dbed0a4a4b1afadf55ca896f36d82c14/release/pasta-in-windows/pasta-in-windows.nar`
  - 書庫は作り直すたびに変わる（ぱすた開発「NAR は変わり続ける」）。Pages の置き場（`homeurl`）は版ごとに替わるので、検体の取り寄せ先にはしない。
- **areka で動く見込みのもの**（この棚卸の調べ）: 縦書きのバルーン（完了 `balloon-vertical-canon`）・`\_l`・`\n[180]`・`\c`・`\_w`・縦書きの選択肢と `OnChoiceSelectEx`・着せ替えの `\![bind]`・`bindgroup`・`mustselect`・`bind+random`・動く WebP の element・数字だけの `\![set,zorder,0,1]` と `\![reset,zorder]`・名前付きの `\![move]`（完了 `move-canon`）・`sakura.balloon.offsetx`／`offsety`・`seriko.alignmenttodesktop,free`・`OnBoot`／`OnFirstBoot`／`OnSecondChange`（Reference3）／`OnMouseMove`／`OnMouseDoubleClick`／`OnMouseDragEnd`／`OnClose`・`Status` の欄。32 ビットの SHIORI は本番では補助のプログラムを通る（emo2 と同じ道）。
- **areka で崩れる見込みのもの**と、引き取る spec（この棚卸で起票・幅を足した）:
  - `OnMouseClick`（× の印で閉じる・ボールのトーク）・`OnMouseEnter`／`OnMouseLeave`（× を赤くする）→ `mouse-click-wheel-events`
  - バルーン込みの重なりで、ぱすたをバルーンより手前に置けない → `zorder-scope-inversion`
  - `\1\v`（常に手前）→ `stay-on-top-directives`
  - `\_u[0x3000]`・`\![bind-noevent]`・選択肢の前の `\![*]` → `script-small-tags`
  - `sakura.seriko.defaultsurface,1000`・`balloon.dontmove,true` → `descript-window-keys`
  - ゴーストのフォルダの中の縦書きのバルーンが名前で見つからない（見込みでいちばん大きく見える崩れ）→ `ghost-inner-balloon`
  - `seriko.sticky-window,0,1` と `--option=ignore-sticky-window` → `window-link-directives`
  - 拡大率 200% と 150% の画面をまたぐドラッグ → `dpi-realign-remembered-chain`
  - 当たり判定ごとの手の形のカーソル（`sakura.cursor` ブレス）→ `shell-mouse-cursor`
  - 右クリックメニューの色と絵（`menu.*`）・着せ替えのメニュー（`sakura.menu,auto`）→ `popup-menu-shell-look`
  - 書庫を URL とハッシュで登記する → `sample-url-fetch`
- **α が小さい画素の当たり**: ぱすたは「α が 8 以下の画素は描かれず、当たらない」を前提に作ってある（ぱすた開発の答え 3）。areka の当たりは α が 128 以上の画素だけ（`crates/wintf/src/ecs/widget/bitmap_source/alpha_mask.rs` の固定のしきい値 `ALPHA_THRESHOLD`）＝α が 9〜127 の画素は areka では通り抜け、SSP では当たる見込み。ukadoc は当たりのしきい値を書いていない。
- **検体の登記表**: `crates/sample-ghost-kit/src/lib.rs` の `SAMPLES`（8 本・行は「名前・種別・同梱のバルーン」だけ）。書庫は `vendors/sample_ghost/<名>.nar` から読む。URL の行は無い。検体のパスの綴りの番人（`crates/log-capture-kit/tests/sample_path_guard_test.rs`）は見張る語を登記表から毎回組むので、行を足せば自動で見張る。

## Desired Outcome

- ぱすたが areka で、SSP 2.9.18 と同じ所まで動く。起動（着せ替えの面 1000・奥に窓の枠・ゴーストのフォルダの中の縦書きのバルーン）→ 縦書きの会話（列と枠・選択肢）→ 撫で・× の印に乗る（赤くなる）・× を押す（終わる）・ボールを押す（ボールのトーク）→ 2 つの窓を一緒にドラッグ・落とした後の付け直し → 拡大率の違う画面への移動 → 右クリックメニュー（着せ替えを含む）→ 終了。
- 重なりは「窓（奥）→ バルーン → ぱすた（手前）」で、SSP と同じに見える。
- 崩れた所が 1 件ずつ、症状・根拠・原因・扱い（直した／方針で受けない／引き取る spec）で記録に残る。範囲の外の崩れも全部、引き取る spec の名前がつく。
- ぱすたが検体の登記表に URL とハッシュの行で載り、書庫そのものはこのリポジトリに入らない。

## Approach

- **最初の段は SSP で測ること**（ぱすたにまたがる問いを、着手の前に SSP 2.9.18 の MCP で測り、答えを兄弟の spec へ渡す）。開発者の機械の SSP を MCP で動かすのは要件の段の作業。測る項目は「要件の段の議題」の 1。
- 登記は `sample-url-fetch` が作る URL の行を使い、ghost_dev のコミットを固定した取り寄せ先と sha256 と大きさを書く。書庫はワークツリーの `target\` の下へ取り寄せるだけで、`vendors/` にも配布物にも入れない。
- 実機の確かめは完了 `emily-ghost-verification` と同じ形（debug 版・`target\` の下の根・絶対パス・32 ビットの補助のプログラムの確かめ・判定の分岐の記録の水準・MCP の撮影と記録の取得・操作は AI、目視の判定は開発者）。
- 崩れの直しは、兄弟の spec が持つものはそちらで直す。この spec の中で直すのは、持ち主の居ない小さいもの（α のしきい値の 1 行など）だけ。
- **「SSP と同じレベル」の最後の確かめ**は、兄弟の spec が着地した後に、この spec が残す実機の手順をもう一度回して行う（完了 `emily-ghost-verification` の棚卸㉓の分け方と同じ）。この spec 自身は、登記・測り・最初の走行・崩れの一覧・自分の持ち場の直しで閉じる。

## Scope

- **In**:
  - SSP 2.9.18 での測り（ぱすたにまたがる問い）と、その記録。答えを兄弟の spec の brief へ申し送る。
  - 検体 `pasta-in-windows` の登記（URL・sha256・大きさの行）と、取り寄せた書庫が登記の物だけを入れる（ゴースト 1 つ・同梱のバルーン 0・縦書きのバルーンはゴーストのフォルダの中）ことの自動テスト。
  - 実機の検証（上の Desired Outcome の流れ）と記録・崩れの一覧。
  - α が小さい画素の当たりのしきい値の直し（SSP で測ってから・全部のゴーストに効くので開発者の決めを得てから）。
  - 何度でも回せる実機の手順（最後の確かめで回し直す物）。
  - 網羅台帳の、ぱすたが使う項目の行を記録と食い違わない形にする。
- **Out**:
  - 兄弟の spec が引き取る崩れの直し（上の一覧）。
  - ぱすたの辞書・書庫の書き換え（直すのは areka の側だけ）。
  - ネットワーク更新でぱすたを新しい版へ替えること（`homeurl` の道は既存の機能。テストの中では走らせない）。
  - 倍率の連動 `balloon.syncscale` と `OnShellScaling`／`OnBalloonScaling`（利用者が倍率を変える道が v0.1.0 に無い＝`shell-balloon-scale-setting`〔v0.4.0〕）。
  - スクロールの矢印 `arrow0`／`arrow1`・`communicatebox`（`balloon-markers`）・`\![reload,shiori]`（`mcp-reload`）・禁則と縦中横（`text-typesetting`）。
  - areka の配布物へのぱすたの同梱。

## Boundary Candidates

- 検体の登記（`sample-ghost-kit` の 1 行と数のテスト・検体の README）
- SSP での測りと記録（この spec のフォルダの中の文書だけ）
- 実機の検証の手順と記録
- α のしきい値（wintf の当たりの 1 か所）

## Out of Boundary

- 兄弟の spec の持ち場のコード
- SSP の作りの癖を写すこと（例: SSP ではバルーンが一度出るまで重なりの指定が効かない・シェルの読み直しの後に `\1\v` で並べ直す必要がある＝ぱすたの辞書のコメント。areka は ukadoc の意味で動かし、癖は写さない）

## Upstream / Downstream

- **Upstream**: `sample-url-fetch`（URL とハッシュの行・取り寄せの段）。最後の確かめは兄弟の 10 本（`mouse-click-wheel-events`・`zorder-scope-inversion`・`stay-on-top-directives`・`script-small-tags`・`descript-window-keys`・`ghost-inner-balloon`・`window-link-directives`・`dpi-realign-remembered-chain`・`shell-mouse-cursor`・`popup-menu-shell-look`）の着地の後。完了 `emily-ghost-verification`（下敷き）・完了 `move-canon`・完了 `balloon-vertical-canon`。
- **Downstream**: 兄弟の spec の要件の段（この spec の SSP の測りの答えを使う）。v0.1.0 の版の門（最後の確かめ）。

## Existing Spec Touchpoints

- **Extends**: なし（完了 spec は直さない）。
- **Adjacent**: 兄弟の 10 本と `sample-url-fetch`。実機の走行の道具を作る `real-run-mcp-kit`・起動の作法を作る `startup-options-core`（どちらもこの棚卸の新しい brief。着地していれば実機の手順が軽くなる）。`popup-menu-residue` の件 12（右クリックメニューが見えずに閉じる＝実機でメニューを押す手順に効く）。

## Constraints

- 実機の根・取り寄せた書庫・一時のフォルダはワークツリーの `target\` の下だけ。areka は絶対パスで起こす。`C:\home\maz\git\ghost_dev` には書かない（読むだけ）。
- 32 ビットの SHIORI なので、debug 版の `areka.exe` の隣に i686 の補助のプログラムが要る（全体テストの後は x64 版に替わっている罠がある）。
- 開発の機械の主の画面は既定で拡大率 200%。拡大率の違う画面のある机で回す。
- 実機の画面の操作は、机（排他テストの机・`impl-watch-test-desk` が着地していれば）をタスクの区切りの中だけで持つ。areka の窓はクリックを通すので、右クリックなどは開発者の許可を得て SendInput で送る（許可は走行のあいだだけ）。
- 失敗はゴーストの台詞で伝える（メッセージボックスにしない）。記録の無い失敗の道を作らない。
- `crates/areka/src/main.rs` は 1,000 行ちょうど＝新しいコードは新しいファイルへ。テストは兄弟のファイル（`*_tests.rs`）。

## 2026-10-18 棚卸㉗の見立て（main 2d86dbff）

- 触るファイル:
  - 検体の登記: `crates/sample-ghost-kit/src/lib.rs`（548 行・登記表に 1 行）・`lib_tests.rs`（648 行・数の直書きを 8 から 9 へ）・`vendors/sample_ghost/README.md`（行を足す・書庫は置かない）。URL の行の形は `sample-url-fetch` が作る。
  - α のしきい値: `crates/wintf/src/ecs/widget/bitmap_source/alpha_mask.rs`（448 行・定数 1 つ）と、その兄弟のテスト（新しいファイル）。
  - 記録: この spec のフォルダの中の測りの記録・検証の記録（新しい文書）。
  - 網羅台帳 `doc/ukadoc-coverage/ledger/*.toml` の、ぱすたが使う行。
  - 実機の手順の道具を置くなら `tools/` か spec のフォルダの中（新しいファイル）。
- 規模: 12〜16 タスク（測り 2〜3・登記 2・α 1〜2・実機の最初の走行 2・崩れの仕分けと申し送り 2・手順と台帳 2〜3）。直しを兄弟へ回すので 20 は超えない見込み。
- 先に要るもの: `sample-url-fetch`（URL の行）。測りの段だけは何も待たずに始められる。
- 優先度の区分: A（開発者の指示＝v0.1.0 のゴール「ゴースト「ぱすた」がSSPと同じレベルで動くこと」）。
- 版: v0.1.0。
- 要件定義のモデル: Opus（先例＝完了 `emily-ghost-verification`。登記の形は `sample-url-fetch` が決め、直しは兄弟へ回すので、先例の無い境目・並行や時刻の新しい保証・骨格の分かれ目には当たらない）。
- 要件の段の議題（答えで作業が変わるもの）:
  1. **SSP で測る**（着手の最初・MCP の `get_property`・`raise_event`・`sakurascript`・`dump_surface`・`get_log` で。ゴーストが複数立つので `ghost_name` を必ず渡す）:
     - ゴーストのフォルダの中の縦書きのバルーンを SSP がどう見つけるか（使っているバルーンの名前と置き場のプロパティ・バルーンのメニューに出るか・ほかのゴーストから見えるか）。ukadoc は「バルーンは全ゴーストで共有される」と書き、ゴーストの中のバルーンを書いていない。
     - α が 8 以下・9〜127 の画素が描かれるか・当たるか。
     - `\1\v` がどの窓を常に手前にするか（相方の窓だけか、ゴーストの全部の窓か）と、いつまで効くか。
     - 選択肢の前の `\![*]` が、マーカーの画像を持たない縦書きのバルーンで何を描くか。
     - 拡大率の違う画面へドラッグしたときに `OnShellScaling` が来るか（来るなら v0.1.0 の `dpi-realign-remembered-chain` の範囲に効く）。
     - `balloon.dontmove,true` でバルーンをドラッグできないか。
  2. **シェルの絵の置き直し**（開発者の決め）: 書庫をこのリポジトリに入れず、ghost_dev のコミットを固定した URL から `target\` の下へ取り寄せるだけにしてよいか。ghost_dev の履歴を書き換えない（`e1e0559` を消さない）約束でよいか。取り寄せた書庫を検体の控えの置き場（`sample-url-fetch` の議題 1）へ写すかどうかも、写すならシェルの絵の置き直しに当たるので開発者が決める。
  3. **固定するコミットの替え方**: ぱすたの書庫は変わり続ける。検体の行を新しいコミットへ替えるのは、spec か明示のコミットでだけにするか（テストが黙って別の版で走らないため）。
  4. **α のしきい値**: 測った結果が「SSP は α が 9 以上なら当たる」なら、areka の 128 を SSP に合わせて下げてよいか。全部のゴーストの当たりが変わる（えみりの要件 6.3 の「全部のゴーストに効く直しは先に聞く」と同じ扱い）。描く側の「α が 8 以下は描かない」まで写すかも決める。
  5. **「SSP と同じレベル」の線**: 見た目の違いのうち、どこまでを不合格にするか。例: ぱすたの右クリックメニューは「あえて普通の Windows のメニューに見せる」色を `menu.*` で書いている＝areka の OS 標準のメニューのままでもほぼ同じに見える（ダークテーマのときだけ違う）。
  6. **最後の確かめの置き場**: 兄弟の 10 本が着地した後の回し直しを、v0.1.0 の版の門の手順に入れるか、小さい spec を後で起こすか。
