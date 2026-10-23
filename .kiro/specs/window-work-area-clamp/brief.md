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

## 2026-10-23 棚卸㉙の再測定（main aa8da8a7）
- **前提の変化**: 起票（10-23）の後に、✅ `pasta-in-windows-verification`（10-23）が着地し、起動の並べ直しの呼び手 `crates/areka/src/emo2_boot/frame/drain_resnap.rs` は 576 行から 633 行になった（描く指令の取り出しを足しただけ・並べ直しの判定は変わらない）。並べ直しの判定 `placement/chain_finalize.rs` は 331 行（`finalize_chain` は作業領域を見ないまま）。記憶から戻す所 `placement/persist.rs`（673）の `apply_restored_placements` は 344 行目で、340 行目の注記「本関数自身はクランプしない」も今のまま。件 2 の崩れ（ぱすたの窓 (0,864)-(1108,2232)）は、`descript-window-keys` の申し送りの件 4 と同じ記録を指す。
- **触るファイル**: `placement/chain_finalize.rs`（331）・`emo2_boot/frame/drain_resnap.rs`（633）・`placement/persist.rs`（673）と兄弟のテスト・寄せる純関数のファイル（新・作業領域は `placement/follow/work_area.rs` 323 の写しを受ける）・件 2 の「吸着の無い窓は左上を保つ」決まりの所＝窓の大きさが変わったときの位置の付け替え `placement/follow/window_move.rs`（**1,221**・289〜305 行目。行数の番人 `crates/log-capture-kit/tests/file_length_guard_test.rs` の例外の表に載る＝足す分は兄弟の新しいファイルへ）・`doc/COMPAT_ARCHITECTURE.md` §8。
- **規模**: 6〜9 タスク（起票の見立てのまま）。上限 20 の内。
- **先に要るもの**: 働きの上ではなし（上流は全部着地済み）。ただし件 2 は `descript-window-keys` が起動の測りを descript の既定の面にした後で、残る崩れを測り直すほうが無駄が無い＝`descript-window-keys` の後に置く。同じウェーブに置けない v0.1.0 の相手＝`dpi-realign-remembered-chain`（`chain_finalize.rs`・`drain_resnap.rs`・`persist.rs`）。依存を足さない。最初の段の測りは開発者の機械の SSP（MCP）。実機の確かめ（窓あわせの無いゴーストでの記憶からの戻し・隣の画面）は排他テストの机。静かな机・前後の全体テストの比べは要らない。
- **優先度の区分**: B（バグ＝窓が画面の外や隣の画面へ出る。起票の Constraints）。
- **要件定義のモデル**: Opus（先例＝`scope-chain-gap` の並べ直しと、バルーンを作業領域へ寄せる `placement/balloon_limit.rs` の `apply_balloon_limit`）。
- **分割の案**: なし。
- **見つけた穴・古くなった記述**:
  - 本文 Scope の Out の「並べ終えた時点の保存が台本の移動を記憶へ書く件（`window-link-directives` の申し送り ⅴ）」は、同じ `window-link-directives` の中で直っていた（`placement/persist.rs` の 545 行目の注記と 604 行目で印 `ScriptPlaced` を読んで外す）。Out から消してよい。
  - 本文 Current State の行数「`chain_finalize.rs`（329 行）」は 331、「`drain_resnap.rs`（576 行）」は 633。
  - 本文 Downstream の `pasta-in-windows-verification` は 10-23 に着地した。着地した後の確かめは、ぱすたの手順書 `doc/pasta-real-run.md` で回す形になる。
  - 本文の Boundary Candidates「`placement/follow/` の大きさの変化の扱い」は、上の `window_move.rs` の 289〜305 行目（下端に吸着した窓だけ中央を保ち、ほかは左上を保つ）のこと。1,000 行を超えているファイルなので、ここへは足さない。
  - `dpi-realign-remembered-chain` の詰め直しも、詰めた位置が隣の画面へ渡りうる。寄せる純関数をどちらが持ち、どちらが呼ぶかを、先に着地する側の要件で決める。
