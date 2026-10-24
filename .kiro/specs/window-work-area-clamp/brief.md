# Brief: window-work-area-clamp

> 2026-10-23 起票（`/kiro-discovery`）。出どころは `window-link-directives` の完了時の棚卸（`.kiro/specs/completed/areka-P0-window-link-directives/signoff.md` の「申し送り」ⅰ〜ⅲ と、`tasks.md` の Implementation Notes の 9.2）。3 件とも「起動のときに窓が作業領域の外へ出る（または出るかもしれない）」で、触る場所が同じ（起動の並べ直し・記憶から戻す所・窓の大きさが変わったときの位置）なので 1 本にまとめた。

## Problem

- **利用者**: 起動の直後に、キャラクターの窓が画面からはみ出したり、隣のモニタへ渡ったりする。ぱすた（`pasta-in-windows`）の実機の走りで 2 つ見えた。
  - **件 1: 起動の並べ直しが窓を作業領域の外へ置く**。`window-link-directives` の実機の 2 回目の走りで、起動の並べ直しが窓を x=−1108 へ置き、窓は左隣の 150% のモニタへ渡った（そこで −831 へ詰め直された）。このときの誤った「動かされていない」の読みは `window-link-directives` が印 `ScriptPlaced` で直したが、並べ直した位置を作業領域の中へ寄せる働きは今も無い。並べ直しの持ち主の `scope-chain-gap` は完了済み。
  - **件 2: ぱすたの窓が作業領域の下へはみ出す**（y 864〜2232・作業領域の下端は 1704）。起動の配置は面 0 の大きさ（320×420）で解かれ、その後に 554×684 の面へ切り替わったとき、吸着の無い（`free`）窓は左上を保つので、下へ伸びた分がはみ出す。
  - **件 3: 記憶から 2 つの窓を戻すときの寄せ方を確かめていない**（`window-link-directives` の research 1.7 の調べ 5）。記憶から戻した位置を作業領域へ寄せ直す処理が、組の 2 つの窓を別々に寄せて離さないか。ぱすたの辞書の「窓あわせ」が起動のたびに窓を重ね直すので、実機ではこの振る舞いが隠れて見えない。

## Current State

- **正典（ukadoc・2026-10-23 に MCP で引き直した）**: 起動のときに窓を作業領域の中へ寄せるかどうかは書いていない。`seriko.alignmenttodesktop` の既定は `bottom`、`free` は「吸着なし」。SSP のヘルプの「よくある質問」は、シェルが画面外へ行ったときは「シェル位置初期化」で戻せるが、初回の起動の時点で位置がおかしい場合はゴーストの作者にかけあうよう書いている＝SSP がいつも寄せるわけではない見込み。どこまで寄せるかは SSP で測る。
- **件 1（areka）**: 起動の並べ直しの判定は `crates/areka/src/placement/chain_finalize.rs`（329 行）の `finalize_chain`、呼び手は `crates/areka/src/emo2_boot/frame/drain_resnap.rs`（576 行）の `finalize_chain_once_with`。判定は隣に詰めた位置を返すだけで、作業領域を見ない。
- **件 2（areka）**: 起動の配置は `crates/areka/src/placement/resolver.rs`（491 行）の `resolve_placement`。窓の大きさを測る面は決め打ち（本体 0・相方 10・`placement/measure.rs` と `emo2_boot/assets.rs`）。ぱすたの descript は `sakura.seriko.defaultsurface,1000` で、`descript-window-keys` がこれを読むようになれば、起動の測りは最初から面 1000 の大きさになる見込み（554×684 が面 1000 かは確かめていない）。それでも、`free` の窓が後で大きい面へ切り替わったときに左上を保ってはみ出す決まりは残る。吸着のある窓は `placement/follow/anchor.rs` の `project_anchor` が辺へ映し直す。
- **件 3（areka）**: 記憶から戻す所は `crates/areka/src/placement/persist.rs` の `apply_restored_placements`（説明の注記に「本関数自身はクランプしない」）。バルーンだけは `placement/balloon_limit.rs` の `apply_balloon_limit` が作業領域へ寄せる。

## Desired Outcome

- 起動の並べ直しの後に、キャラクターの窓が作業領域の外（隣のモニタを含む）へ置かれない。寄せたときに並びの順（本体の隣に相方）を保つかどうかを、SSP と比べて決めてある。
- `free` の窓が大きい面へ切り替わって作業領域からはみ出すときの扱いを、SSP と比べて決めてある（寄せる・寄せない・起動の直後だけ寄せる）。
- 記憶から組の 2 つの窓を戻したとき、作業領域へ寄せても 2 つが離れない（離れるなら直す）。窓あわせの無いゴーストで実機を回して確かめてある。
- 決定論のテストで「並べ直した位置が作業領域の外に出る並び」「`free` の窓の面が大きくなる」「記憶の位置が作業領域の外にある組」を固定する。

## Approach

- **最初の段は SSP で測ること**（開発者の機械の SSP の MCP）: ⑴ 2 人並べると本体の隣が作業領域の外になるとき、SSP はどこへ置くか（どのモニタか・並びの順を保つか） ⑵ `free` の窓が大きい面へ切り替わってはみ出すとき、SSP は寄せるか ⑶ 記憶の位置が作業領域の外にあるとき、SSP は寄せるか、組の 2 つをどう寄せるか。答えで寄せ方を決める（要件の段の裁定）。
- 寄せる働きは、配置の判定と同じく値を受ける純関数に置き、`MonitorSnapshot`（`placement/follow/work_area.rs`）の作業領域を渡す。

## Scope

- **In**: 件 1〜3。SSP での測り・寄せ方の裁定・起動の並べ直しと記憶から戻す所の寄せ・`free` の窓の面の切り替えの扱い・記録・決定論のテスト・実機の確かめ（窓あわせの無いゴーストでの記憶からの戻し）・`doc/COMPAT_ARCHITECTURE.md` §8 の行。
- **Out**: 起動の測りの面を descript の既定の面にすること（`descript-window-keys`）・拡大率が変わったあとの詰め直し（`dpi-realign-remembered-chain`）・`seriko.alignmenttodesktop` の `top`／`left`／`right` の起動の位置と、作業領域が変わったときの沿わせ直し（`window-snap` の件 1）・並べ終えた時点の保存が台本の移動を記憶へ書く件（`window-link-directives` の申し送り ⅴ）。

## Boundary Candidates

- 起動の並べ直しの判定と呼び手（`placement/chain_finalize.rs`・`emo2_boot/frame/drain_resnap.rs`）
- 記憶から戻す所（`placement/persist.rs`）
- 面の大きさが変わったときの `free` の窓の位置（`placement/follow/` の大きさの変化の扱い）

## Out of Boundary

- 表情の差し替えで絵の大きさが変わったときにキャラクターが横へ動かない約束（`scope-chain-gap` 7.4・`dpi-transition-atomicity` 要件 6.6）は変えない。
- 利用者がドラッグで画面の外へ置いた窓を勝手に戻すこと（決めるなら要件の段で）。

## Upstream / Downstream

- **Upstream**: 完了 `scope-chain-gap`（起動の並べ直し）・完了 `position-persist`（記憶から戻す）・完了 `window-link-directives`（`ScriptPlaced` の印・組）。
- **Downstream**: `pasta-in-windows-verification`（最後の通しでぱすたの窓が画面に収まる）。

## Existing Spec Touchpoints

- **Extends**: なし（完了 spec の決まりを足す。足した扱いは `doc/COMPAT_ARCHITECTURE.md` §8 に残す）。
- **Adjacent**: `dpi-realign-remembered-chain`（`chain_finalize.rs`・`drain_resnap.rs`・`persist.rs` を分け合う＝同じウェーブに置かない）・`descript-window-keys`（件 2 の測りの面・`measure.rs`）・`window-snap`（件 1＝`resolver.rs`・作業領域が変わったときの沿わせ直し）・`placement-measure-bake-once`（`measure.rs`）・`extra-character-windows`・`multi-ghost-runtime`（`placement/`）。

## Constraints

- 優先度の区分: B（バグ＝窓が画面の外や隣のモニタへ出る）。
- 版: v0.1.0（ゴール「ゴースト「窓際のぱすたさん」が SSP と同じに動く」で、ぱすたの窓がはみ出すため）。
- 1 フレーム遅らせて寄せる形は取らない（置く時点で作業領域の中に解く）。
- 寄せ方は ukadoc が黙っているので、SSP で測ってから決める（測る前に areka の独自の決まりを作らない）。
- 実機の根・一時フォルダはワークツリーの `target\` の下だけ。
- 規模の見立て: S〜M（6〜9。SSP の測り 1・件 1 が 2・件 2 が 1〜2・件 3 が 1〜2・記録と実機 1）。
- 要件定義のモデル: Opus（先例＝`scope-chain-gap` の並べ直しと `balloon_limit` の作業領域への寄せ）。

## 2026-10-24 `vertical-choice-hover-band` の完了の棚卸で足した件

- **件 4: バルーンだけが作業領域へ寄せられ、ぱすたの「×」と横線が縦書きの 1 列目にかぶる**。`vertical-choice-hover-band` の実機の撮り直し（2026-10-24・拡大率 200%・`.kiro/specs/completed/vertical-choice-hover-band/evidence/areka-choice-*.png`）で、1 列目の「今日の天気は？」の「今日の」の下に横線と × が重なり、続く字が隠れた。SSP の同じ台本の撮った物（同じフォルダの `ssp-choice-*.jpg`）では 1 列目は × の下から始まり、何も重ならない。
  - **× と横線の出どころ**: ぱすたの本体のシェル（sakura・面 1000）の `window/canvas.png`（516×656 の透明なキャンバス）が、× のボタン（`collision20,430,22,476,54,閉じる`）の範囲だけ、窓の絵の × と上の線を不透明に写して持っている（`surfaces.txt` の注記・SSP で × がホバーで赤くなるための当たり板）。重なりの順はゴーストの作りで「窓＝kero → 縦書きバルーン → ぱすた＝sakura」なので、× はいつもバルーンより手前に出る。バルーンの絵 `balloons0.png` は白の不透明で、× の範囲に字が来なければ見えない。
  - **ずれの出どころ（記録から）**: バルーンの置き場はシェルの `sakura.balloon.offsetx,476`・`offsety,58`（`balloon.alignment,left`）で、窓のキャンバスの (40,58) に重ねる作り（バルーンの descript の注記）。実機の記録 `target\vcband\rr\logs\rr3.stdout.log`（`vertical-choice-hover-band` の作業木・消えうる）では、`[balloon-limit] Clamp 起動時関門` と `Clamp runtime 関門` がバルーン（scope 0・872×900 物理画素）を y 980 から 804 へ動かした（作業領域の下端 1704）。差の 176 物理画素＝88 論理画素だけ、バルーンがシェルに対して上へずれ、× の範囲（キャンバスの y 22〜54）がバルーンの上から 52〜84 論理画素の所＝1 列目の 3 字目のあたりに来た。撮った物で数えた横線（バルーンの上から約 52 論理画素）と × の中心（約 66）とも合い、左右はバルーンの左がキャンバスの x 40 で SSP の作りどおり。シェルの窓は寄せられないので、件 2 と同じ「ぱすたの窓が作業領域の下へはみ出す」形から、バルーンだけが寄せられて組がずれた。
  - **見立て（確かめていない）**: 件 2 の窓のはみ出しが解ければ、バルーンは寄せられず × は 1 列目にかからない見込み。SSP が同じ場面でバルーンを作業領域へ寄せるか（寄せるならシェルも一緒に動かすか）は測っていない。この走りは `descript-window-keys`（10-24 着地＝起動の測りを descript の既定の面 1000 で行う）を取り込む前の枝で回したので、取り込んだ後の起動ではシェルの置き場が変わっている見込みがある＝測り直してから扱いを決める。
  - **望む形**: ぱすたの縦書きのバルーンの 1 列目に × と横線がかからない（SSP と同じ）。シェルとバルーンの組の相対の位置を、バルーンだけを作業領域へ寄せることで崩さないか、崩すなら SSP と同じ崩し方にする。
  - **触る見込みの所**: `crates/areka/src/placement/balloon_limit.rs` の `apply_balloon_limit`（起動時の関門）と、`placement/follow/window_move.rs` の runtime の関門（1,221 行・足す分は兄弟のファイルへ）。件 2 の直しで済むなら足し分は無い。
- 起票でなくここへ足した訳: 触る場所（作業領域への寄せ・件 2 のぱすたの窓のはみ出し）が同じで（kiro-complete 冒頭ステップの手順 3 の 2 つ目）、確かめるのに実機（倍率 200%・ぱすた）と SSP の測りを何度も回す必要がある（同 4 つ目）。区分 B・版 v0.1.0（ぱすたのゴール）はこの spec のまま。

## 2026-10-24 追記（`dpi-realign-remembered-chain` のタスク 6.3 の実機の確かめ・`/kiro-discovery`）

> 開発者「ぱすたを起動したときの初期表示位置が画面に収まっていない。SSPは画面内に収まっていた。原因を調査し、適切な位置の初期表示を実現して欲しい。なお、SSPと同じルールになる必要はなく、画面内に収まっていればよいものとする。」
> 開発者（同じ確かめの中で）「初回起動トークの時にデザイン調整してくれないようで、レイアウト崩れの調整が初回起動トークの後にずれる。ただこの現象はSSPでは発生しない。イベントの発生順序が違うかもしれない。」

- **件 2 の再確認と裁定**: 2026-10-24 の実機（主の画面 2880×1800・200%、左の画面 150%・記憶の空の初回起動・ぱすた）でも同じ崩れが出た。記録 `target\dpi-verify\logs\run4.stdout.log`（`dpi-realign-remembered-chain` の作業木）では、本体の既定の位置が面 0 の寸（k=2 で 640×840）で下端に揃えて y=864 に置かれ、台本の `\s[1000]` で 1108×1368 に育ち、`anchor=Free` で左上を保って下端 2232 になった（`\![move]` の基準 `base_bottom=2232`）。相方は `\![move,--base=0,…left.top]` で本体の左上へ寄せられて一緒にはみ出す。この走りの areka は `descript-window-keys`（起動の測りを descript の既定の面にする）の取り込み前の版。**開発者の裁定: 起動の表示位置は画面（作業領域）の中に収まっていればよく、SSP と同じ規則である必要はない**＝件 2 は Approach の SSP の測り ⑵ を待たずに、収める規則を areka で決めてよい（測るのは任意）。
- **件 5（新）: 初回起動のトークの最中にぱすたの窓あわせ（見た目の調整）が効かず、トークの後にずれる**。SSP では起きない。同じ記録では、初回起動で kanade が `OnBoot` を飛ばして `OnFirstBoot` の台本を流し（`boot_type_script`）、台本の頭が `\0\s[1000]\1\s[10]\![move,…]\![reset,zorder]\![set,zorder,0,1]\0\![bind,…]…`。窓あわせがどのイベント・どの指令で流れるか、SSP が初回起動で送る要求の並びと時機（`ghost\master\ssp_shiori_log.txt` の差分）と比べて原因を決めるのが最初の段。触る場所は起動の配置（件 2 と同じ所）か、起動のイベントの順（kanade）かのどちらかで、測ってから決める。件 2 と同じく起動の直後のぱすたの見た目の崩れなので、この spec に束ねた（小さいバグは触る場所が同じものを 1 本に・測った結果で kanade の持ち物と分かったら、要件の段で切り出すか決める）。
- **規模の見立ての更新**: S〜M（7〜11。件 5 の SSP との比べと直しで 1〜2 増える）。
- **件 3 への足し（`dpi-realign-remembered-chain` の完了の棚卸）**: 記憶の位置が作業領域の外で、起動の寄せ直しで引き寄せて表示しているとき、`dpi-realign-remembered-chain` が印の無い古い記憶へ書き足す隣り合いの印（`[window-adjacent]`）は、表示している矩形から決まる（同 spec の設計どおり）。外した画面を付け直して本体が元の位置へ戻る場合だけ、記憶の並びと食い違いうる。寄せ方を決めるとき、印をどの矩形から決めるか（表示・記憶の位置・決めない）も一緒に決める（直し方が複数ある＝裁定が要るため起票側へ回した）。

## 2026-10-24 棚卸㉚の再測定（main 9318cdb0）
- **前提の変化**: ✅ `dpi-realign-remembered-chain`（10-24）で件 1 の場所が変わった。起動の並べ直しの本番の道は `crates/areka/src/emo2_boot/frame/drain_resnap.rs`（685）の `finalize_chain_once_with`（420 行目）が台本の印・組・覚えた隣り合いの印を読み、`placement/chain_finalize.rs`（613）の `boot_pair_decision`（157 行目）を呼ぶ形になった。本文の `finalize_chain` は試験の口だけ（`#[cfg(test)]`）。どちらも作業領域を見ないまま。拡大率の後の詰め直しは `placement/chain_realign.rs`（423）の `realign_chain_once_with`。記憶から戻す所 `placement/persist.rs`（815）の `apply_restored_placements` は 347 行目、注記「本関数自身はクランプしない」は 343 行目。✅ `descript-window-keys`（10-24）が起動の測りを descript の既定の面にした＝件 2 の前半（面 0 で解いて後で 1000 へ育つ）が変わる。件 2・件 4 は取り込んだ後の版で測り直してから扱いを決める。同じ spec がバルーンの動かせない指定を `placement/balloon_lock.rs` に足した（件 4 の寄せの所とは別）。
- **申し送りの漏れ（範囲が広がる）**: `dpi-realign-remembered-chain` の要件（「作業領域への寄せ」の節・2026-10-23 確定）は、「詰め直した位置は寄せない。寄せる働きは `window-work-area-clamp` が作り、詰め直しの位置にも同じ寄せを当てるのは向こうの範囲」と決め、着地のときにこの brief へ申し送ると書いたが、この brief に無かった。同じ spec の設計（`design.md` 22・60 行目）は、起動で隣り合わせた位置にも寄せを当てるのをこの spec の持ち分とする。＝本文 Scope の Out「拡大率が変わったあとの詰め直し（`dpi-realign-remembered-chain`）」のうち寄せだけは In へ移り、触るファイルに `chain_realign.rs` が加わる。
- **触るファイル**: `drain_resnap.rs`（685）・`chain_finalize.rs`（613）・`chain_realign.rs`（423）・`persist.rs`（815）と兄弟のテスト・寄せる純関数（新。`placement/` の直下に置くと `placement/mod.rs`〔969・上限まで 31〕に `mod` の 1 行が入る＝作業領域の写し `placement/follow/work_area.rs`（323）の隣に置けば避けられる）・件 2 の `placement/follow/window_move.rs`（1,221・行数の番人の例外の表に載る＝足す分は兄弟の新しいファイルへ）・件 4 の `placement/balloon_limit.rs`（231）・件 5 は測ってから（起動の配置か、kanade の起動のイベントの順）・`doc/COMPAT_ARCHITECTURE.md` §8。
- **規模**: 8〜12 タスク（7〜11 に、詰め直しへの寄せの 1 を足した）。上限 20 の内。件 5 が kanade の持ち物と分かったら、要件の段で切り出すかを決める。
- **先に要るもの**: 働きの上ではなし（上流は全部着地済み）。v0.1.0 の相手: `placement-measure-bake-once`（案 A は `placement/mod.rs`＝寄せる関数を `placement/` の直下に置くときだけ重なる）。`balloon_limit.rs` を挙げる `real-run-input-guard-observations`。`stay-on-top-directives` とは重ならない（あちらは `spawn.rs`）。依存を足さない・emo-text の席を使わない。件 2 は開発者の裁定（2026-10-24）で SSP の測りは任意。件 5 は SSP の `ssp_shiori_log.txt` の差分と比べるのが最初の段（開発者の機械）。実機は排他テストの机（倍率 200%・ぱすた・窓あわせの無いゴーストでの記憶からの戻し）。静かな机・前後の全体テストの比べは要らない。
- **優先度の区分**: B（バグ＝窓が画面の外や隣の画面へ出る。件 2 は開発者の指示「画面内に収まっていればよい」も出どころ）。
- **要件定義のモデル**: Opus（先例＝起動の並べ直しと、バルーンを作業領域へ寄せる `placement/balloon_limit.rs` の `apply_balloon_limit`・記憶の位置を寄せる `persist.rs` の射影）。
- **分割の案**: なし（件 5 だけ上の条件つき）。
- **見つけた穴・古くなった記述**:
  - 本文 Current State の「`finalize_chain`」は試験の口だけ＝本番は `boot_pair_decision`。行数「`chain_finalize.rs`（329 行）」「`drain_resnap.rs`（576 行）」は 613・685。
  - 本文 Scope の Out「並べ終えた時点の保存が台本の移動を記憶へ書く件（`window-link-directives` の申し送り ⅴ）」は、同じ `window-link-directives` の中で直っていた（`persist.rs` が印 `ScriptPlaced` を読んで外す）＝Out から消してよい。
  - 本文 Downstream の `pasta-in-windows-verification` は 10-23 に着地した。着地した後の確かめは、ぱすたの手順書 `doc/pasta-real-run.md` で回す。
