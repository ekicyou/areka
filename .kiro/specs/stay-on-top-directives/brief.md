# Brief: stay-on-top-directives

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

## 2026-10-19 zorder-scope-inversion の完了からの申し送り

- wintf の鎖と組の形が変わった。鎖が組の 2 枚（バルーンとキャラクター）を連続させない間は、鎖の適用系がその組の結び目を外し、印 `PairReleased`（値なし）を付ける。解除や鎖の変化で連続に戻れば結び直して印を外す。外すかどうかの判定は `zorder_chain.rs` の `pair_released_by_chain` 1 本だけで、組の系（確立・維持・沈降の観測）は同じフレームに公開済みの計画をこの判定で読み、外している組には触れない（見送りの理由 `ReleasedByChain`）。
- 鎖の適用系の 1 巡の順は「去る窓の切離し → 鎖の撤去 → 組の外し → 組の結び直し → 鎖の付与 → 後押し → 直後の実測」。外し損ねがあった巡だけ、付与と結び直しの直前に持ち主の輪を確かめる（`link-refused`）。帯へ上げる処理をこの 1 巡に足すなら、この順と `PairReleased` の組（結び目の無い組）を前提にすること。持ち主の鎖の根だけを帯へ上げる案では、結び目を外している組の窓が鎖の外れ方によって付いてこない場合がある。
- 触るファイルの行数（完了時）: `zorder_chain_apply.rs` 930・`zorder_pair.rs` 892・`zorder_chain.rs` 631（上限 1,000）。`zorder_chain_apply.rs` は余地が 70 行しか無いので、足すなら先に外へ出す。

## 2026-10-22 棚卸㉘の再測定（main 52b014de）
- 前提の変化: 待っていた ✅ `zorder-scope-inversion`（10-19）が着地した（申し送りは上の節）＝こちら側の待ちは無い。✅ `mcp-reload`（10-22）が受け手の表 `crates/areka/src/emo2_boot/consumer_ledger.rs` の中のテストを兄弟のファイルへ出し、960 行から 472 行になった（受け手の中身を新しいファイルに置くのは変わらない）。ゴーストの読み直しは窓を閉じて作り直す（`emo2_boot/reload.rs`）。
- 触るファイル: `crates/areka-parsers/src/sakura/decode.rs`（502・`decode_bare` に 1 腕）と兄弟のテスト・`consumer_ledger.rs`（472・1 行）・新しい受け手のファイル（新・`emo2_boot/` の下）・wintf の `crates/wintf/src/ecs/window/zorder_pair.rs`（892・帯を避ける判断は 368 行目）・`zorder_pair_maintain.rs`（557・帯の中かを見る所は 488 行目）・`zorder_chain_apply.rs`（930・余地 70 行＝足すなら先に外へ出す）・`window_pos/mod.rs`（454・帯へ上げ下げする指令の口）・後から出る窓の作り `crates/areka/src/placement/spawn.rs`（776）・網羅台帳 `sakura-script.toml` の 3 行（3352・3385 行目ほか）・`doc/COMPAT_ARCHITECTURE.md` §8。
- 規模: 6〜9 タスクのまま（`zorder_chain_apply.rs` を外へ出すなら 1 つ増える）。
- 先に要るもの: 働きの上ではなし。同じウェーブに置けない v0.1.0 の相手: `script-small-tags`（`decode.rs`）・受け手の表に行を足す `window-link-directives`・`script-small-tags`・`descript-window-keys`・`mcp-strict-errors`（案 2）・`script-security-level`（1 本ずつ）・`dpi-realign-remembered-chain`（`placement/spawn.rs`）。後の版: `zorder-chain-residue`・`zorder-property`・`multi-ghost-runtime`・`tasktray-minimize`・`elevated-drop-filter`（`spawn.rs`）。依存を足さない。実機の確かめは画面のロックを外した排他テストの机で、ほかのアプリを前に出して見る。静かな机・前後の全体テストの比べは要らない。議題 1 の測りは開発者の機械の SSP。
- 優先度の区分: A（開発者の指示＝v0.1.0 のゴール・ぱすた）。
- 要件定義のモデル: Opus（`zorder-scope-inversion` が鎖の 1 巡の順を決めた後で、帯へ上げる処理はその順に乗る＝時刻や順番の新しい保証を作らない。先例＝`ghost-window-zorder` の要件 8.2）。
- 分割の案: なし。
- 見つけた穴・古くなった記述:
  - 本文と Constraints の「`consumer_ledger.rs`（960 行）」は古い（472 行）。
  - 本文 Current State の「帯を避ける判断（`zorder_pair.rs` の `measure_window_is_always_on_top`）」: 測る関数は `zorder_pair.rs` に残るが、呼ぶのは組を保つ系の `zorder_pair_maintain.rs`。条件をつけるなら両方を見る。
  - ゴーストの読み直し（`\![reload,ghost]`・MCP の `reload`）で窓が作り直されたとき、「常に手前」を保つか戻すかを決める。正典は「ゴースト終了まで有効」で、読み直しは終了ではない。SSP で測る項目（議題 1）に足す。

## 追記（2026-10-23 `areka-P0-window-link-directives` の完了の棚卸）

- **件 1: 台本の `\![set,zorder]` の組が、シェルの切り替えで捨てられる**（`window-link-directives` 要件 10.6・要件の段の裁定の表の 2 行目）。
  - 正典（ukadoc・2026-10-23 に MCP で引き直した）: `\![set,zorder,スコープID,スコープID,...]` は「ゴースト終了まで有効」。シェルの切り替えはゴーストの終了ではない。
  - areka の今: シェルの切り替えで、重なりの順の帳簿の基底を新しいシェルの `seriko.zorder` へ置き直すとき、台本で作った組も一緒に落ちる（`crates/areka/src/emo2_boot/frame/zorder_descript.rs` の `apply_descript_rebase` が `ZOrderGroupLedger::set_descript_base` を呼ぶ・注記に「タグ由来のグループも落ち」）。完了 `shell-balloon-switch` の要件 2.7 の後始末として入った形。
  - 兄弟の決まり: 完了 `window-link-directives` は、台本で作った窓の組（`\![set,sticky-window]`）と台本で変えた吸着を、シェルの切り替え（とシェルの読み直し）の後も保ち、descript の値に従っていたものだけを新しいシェルの descript へ取り替える（裁定 2）。`\![set,zorder]` の組だけが違う扱いになっている。
  - やること: 台本で作った重なりの組をシェルの切り替えの後も保ち、descript の `seriko.zorder` から来た組だけを新しいシェルの値へ取り替える。ゴーストの読み直しで窓が作り直されたとき（本文の棚卸㉘の穴の 3 つ目）も同じ「ゴースト終了まで有効」の線で決める。SSP で測る項目（議題 1）に「シェルの切り替えの後に `\![set,zorder]` の組が残るか」を足す。
  - 起票の理由: 本 spec は「ゴースト終了まで有効」の指令（`\v`・`stayontop`）の寿命をシェルの切り替えと読み直しで決め、同じ重なりの順の鎖の上で働く。`zorder-property` はプロパティの読み書きだけを持ち、Out に「タグ／descript 入口」を挙げているので合わない。
  - 触るファイル（足す分）: `crates/areka/src/emo2_boot/frame/zorder_descript.rs`（105 行）と兄弟のテスト `zorder_descript_tests.rs`・帳簿 `crates/areka/src/placement/zorder_group_ledger.rs`（基底の置き直しで台本の組を残す口）・`doc/COMPAT_ARCHITECTURE.md` §8 の行。+1〜2 タスク。

## 本 spec からの申し送り（2026-10-23・`areka-P0-pasta-in-windows-verification` のタスク 6）

- 出どころ: `areka-P0-pasta-in-windows-verification` の検証の記録 `verification.md`（5 節の崩れた所の表。件の番号は表の番号）と、SSP の測りの記録 `ssp-measurement.md`（SSP 2.9.23・書庫の sha256 `ad473d5d…a43a6`＝`pasta.toml` 1.0.0）。撮った物と記録は git の外の `<w>\target\pasta-verify\`（本走行 run1。`logs\`・`shots\run1\`・SSP の撮った物は `ssp\`）・`<w>\target\pasta-verify-alt\`（補いの走り alt2。縦書きのバルーンを根の `balloon\` へ写した置き方）・`<w>\target\pasta-verify-rerun\`（直しの後の回し直し rerun1。alt2 と同じ置き方）に在る（`<w>` は `C:\home\maz\git\areka\.claude\worktrees\areka-p0-pasta-windows-d4ed7a`）。記録の時刻は世界標準時（`Z`）のまま。日本時間は 9 時間足す。
- 開発者の決め（2026-10-23）「原則として、すべての申し送りが0.1.0に解決するのであれば承認する。申し送り漏れはダメですよ。」。この spec は v0.1.0 なので、下の件はこの spec の中で片づける。
- 着地した後の確かめは、ぱすたの手順書 `doc/pasta-real-run.md`（5 節の 7 段の表）で回す。
- **件 19（`\![set,windowstate,!stayontop]` に受け手が無い）**: ぱすたの起動の台本の `\![set,windowstate,!stayontop]` に受け手が無い。見た目への効きは走行の記録からは言えない。
  - 根拠: 起動の台本の原文（`logs\run1.stdout.log` の `2026-10-22T23:17:58.394554Z` の `areka::log::script`）。
  - 原因: 受け手の表 `crates/areka/src/emo2_boot/consumer_ledger.rs:27` の決まり（名簿に無い第 1 引数 `windowstate` は担当なし）。
- **SSP の基準**（`ssp-measurement.md` 2 節の段 1・3 節の終わり）: SSP が送った `OnBoot` の台本も `\![set,windowstate,!stayontop]` を含む。起動の重なりは、窓の枠（奥）→ バルーンの字 → ぱすた（手前）。`\1\v` は通しの台本に出てこなかったので、見えた答えは無い。
- 網羅台帳の宛先: 2026-10-23 に 1 行の `owner` をこの spec にした（`verification.md` 10 節の「直したことと残り」）。
- **着地で変わるはずの所**（手順書の段 1）: 窓の枠が奥へ回る（常に手前にしない指定が SSP と同じに効く）。

## 2026-10-23 棚卸㉙の再測定（main aa8da8a7）
- **前提の変化**: 触る所の中身は棚卸㉘から変わらない。✅ `window-link-directives`（10-23）が受け手の表 `crates/areka/src/emo2_boot/consumer_ledger.rs` に 5 行足し、472 行から 496 行になった（登記の数は 36）。窓を作る所 `crates/areka/src/placement/spawn.rs` は 783 行（13 行増えた）。✅ `pasta-in-windows-verification`（10-23）の申し送り（件 19）と、`window-link-directives` の完了の棚卸の追記（件 1・台本の重なりの組がシェルの切り替えで落ちる）が本文の後ろに在る＝どちらもこの spec の中で片づける。網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml` は `\![set,windowstate,!stayontop]` の 1 行だけ持ち主がこの spec で、`\v`・`\![set,windowstate,stayontop]` の 2 行は持ち主が空のまま。
- **触るファイル**: 台本の読み手 `crates/areka-parsers/src/sakura/decode.rs`（502・`decode_bare` は 226 行目）と兄弟のテスト・受け手の表 `consumer_ledger.rs`（496・1 行と見本の表）・新しい受け手のファイル（新・`emo2_boot/` の下）・wintf の組の判断 `crates/wintf/src/ecs/window/zorder_pair.rs`（892・帯を避ける所は 364〜370 行目）・組を保つ系 `zorder_pair_maintain.rs`（557・帯の中かを読む所は 489 行目）・鎖の適用 `zorder_chain_apply.rs`（930・余地 70 行）・窓の指令の口 `window_pos/mod.rs`（454・帯へ上げ下げは 131・137 行目）・`placement/spawn.rs`（783）・件 1 の `emo2_boot/frame/zorder_descript.rs`（105）と帳簿 `placement/zorder_group_ledger.rs`（657）・網羅台帳の 3 行・`doc/COMPAT_ARCHITECTURE.md` §8。**足す分**: 「先送りの言葉」を見張る 2 つのテスト＝wintf の `crates/wintf/src/ecs/window/zorder_pair_deferred_vocabulary_tests.rs`（組と鎖の 8 ファイルに `topmost`・`stayontop`・`windowstate` などの語が入ると赤）と、兄弟の `crates/areka/src/placement/spawn_zorder_pair_deferred_tests.rs`（`spawn.rs`・`zorder_descript.rs`・`zorder_group_ledger.rs` ほか 7 ファイルを見る）。完了 `ghost-window-zorder` の要件 8.3「`\v` を解釈しない」を守る檻なので、この spec が両方を一緒に書き直す（片方だけ直すと守りが片肺になる、と表の注記に在る）。
- **規模**: 8〜12 タスク（本文の 6〜9 に、件 1 の 1〜2 と、見張りの 2 つのテストの書き直し 1 を足した。`zorder_chain_apply.rs` を先に外へ出すならさらに 1）。上限 20 の内。
- **先に要るもの**: 働きの上ではなし。同じウェーブに置けない v0.1.0 の相手＝受け手の表に行を足す `script-small-tags`・`descript-window-keys`・`shell-scale-linkage`・`balloon-communicatebox`・`mcp-strict-errors`（案 2）（1 本ずつ）・`script-small-tags`（`decode.rs` も）・`dpi-realign-remembered-chain`（`spawn.rs`）。依存を足さない。静かな机・前後の全体テストの比べは要らない。実機の確かめは画面のロックを外した排他テストの机で、ほかのアプリを前に出して見る。議題 1 の測り（`\1\v` の効く範囲・シェルの切り替えの後に `\![set,zorder]` の組が残るか）は開発者の機械の SSP。
- **優先度の区分**: A（開発者の指示＝v0.1.0 のゴール「ぱすたが SSP と同じに動く」）。
- **要件定義のモデル**: Opus（帯へ上げる処理は `zorder-scope-inversion` が決めた鎖の 1 巡の順に乗る。先例＝`ghost-window-zorder` の要件 8.2「後から足せる構造」。新しい境目も、並行や順の新しい保証も作らない）。
- **分割の案**: なし。
- **見つけた穴・古くなった記述**:
  - 上の見張りの 2 つのテストが、これまでの見立ての触るファイルに無かった。帯へ上げる処理を組と鎖のファイルに書くと、ここが赤になる。
  - 棚卸㉘の節の「`consumer_ledger.rs`（472）」は 496、「`spawn.rs`（776）」は 783。
  - 受け手の表の決まりの注記（`consumer_ledger.rs` 27 行目「名簿に無い第 1 引数（例: `\![set,windowstate]`）は担当なし」）は、この spec の着地で例が変わる＝一緒に直す。
  - 網羅台帳の `\v` と `stayontop` の 2 行の持ち主が空。着地のときにこの spec を書く。
