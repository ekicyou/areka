# Brief: areka-P0-zorder-scope-inversion

> 2026-10-18 棚卸㉗で起票（開発者の指示「もう一つの目標は、ゴースト「ぱすた」がSSPと同じレベルで動くこと、です。」＝v0.1.0 のゴール）。証拠はこの棚卸のぱすたの抜けの調べと、ゴースト「窓際のぱすたさん」の辞書（読むだけ・`ghost_dev` の `release/pasta-in-windows/pasta-in-windows/ghost/master/dic/tategaki.pasta`）。**完了 spec の要件を覆す**ので、覆すかどうかは要件の段で開発者が決める。

## Problem

- **ゴーストの作者**: 台本で「キャラクターを自分のバルーンより手前に置く」と書いても、areka はそうしない。ぱすたは `\![set,zorder,s0,b0,s1]`（手前から ぱすた → 縦書きのバルーン → 窓）で、窓の枠の絵の上にバルーンの文字を置き、その手前にぱすたを立たせる。areka ではバルーンがぱすたより手前に来て、ぱすたが文字の裏に回る。
- 正典はこの並べ方を認めている。ukadoc の `\![set,zorder,…]` は、`balloon1,surface1,balloon0,surface0` の例を挙げて「この順に厳密に制御される」と書く（SSP 2.7.34 からバルーン込みの記法）。並べる順に「バルーンは自分のキャラクターより手前」という縛りは書いていない。

## Current State

- **完了 spec の決め**（`.kiro/specs/completed/areka-P0-scope-zorder-pinning/requirements.md`）:
  - **要件 2.4**: 明示の記法で、あるスコープのキャラクターの窓をそのスコープのバルーンの窓より手前に置く指定は、「同じスコープの中でバルーンの窓がキャラクターの窓の直上にある」という既存の決まりを優先し、そのスコープの中の逆転を採らず、採らなかったことを記録する。
  - **要件 6.3**: 同じスコープの中でバルーンの窓がキャラクターの窓の直上にある重なりを、グループの指定の有無にかかわらず保つ。
  - 要件 2.6（片方の窓だけを名指ししたら相棒の窓を隣に補う）と要件 12.2（互換の記録に「明示の指定が同じスコープの中の隣り合いと食い違うときは隣り合いを優先する」を裁量として書く）も同じ決まりの上に立つ。
- **そう決めた理由**（同 spec の要件・研究・設計から）:
  1. 守る決まりの出どころが、さらに前の完了 spec `areka-P0-ghost-window-zorder` の要件 1（1.1「バルーンの窓はキャラクターの窓より手前」・1.4「キャラクターの窓がバルーンの窓より手前になる状態を起こさない」）。目的は「会話が他の窓に埋もれて読めなくなることがない」。
  2. 作りの上で、バルーンの窓はキャラクターの窓に**持ち主として結ばれている**（wintf の組の宣言 `KeepDirectlyAbove` を OS の持ち主の関係へ落とす `crates/wintf/src/ecs/window/zorder_pair_establish.rs`）。OS は「持たれる窓は持ち主より手前」を強いるので、キャラクターをバルーンより手前に出すことは、この結び目を外さない限り OS が許さない（同 spec の研究 R2「そもそも OS が許さない」）。
  3. 設計は変える所を小さくするため、組の結び目に触らず、スコープをまたぐ結び目だけを足す形を採った（同 spec の設計 DD-2「ペア edge は既存機構のまま」）。研究の段では、要件 2.4 は「記録だけが仕事の可能性」と見られていた。
  - つまり当時、逆転を求めるゴーストは無く、正典の例も「バルーンがキャラクターの手前」の順だけだったので、既存の決まりを守る側に倒した。
- **今の作り**:
  - 指定の読み取りと「隣り合いへ寄せる」決まり: `crates/areka/src/placement/zorder_group_ledger.rs`（633 行・冒頭の説明の「同一スコープの 2 窓は隣接ブロックへ寄せる」）。数字だけの記法は `[バルーン, キャラクター]` の 2 枚へ広げる。寄せたことは `Normalization.reordered` で記録へ渡す。
  - 並びから持ち主の鎖を組む: `crates/areka/src/placement/zorder_chain_compose.rs`（202 行・前提「各スコープが `[Balloon, Char]` の隣接ブロック」）。
  - wintf の鎖: `crates/wintf/src/ecs/window/zorder_chain.rs`（450）・`zorder_chain_apply.rs`（799）。組の結び目: `zorder_pair.rs`（884）・`zorder_pair_establish.rs`（197）・`zorder_pair_maintain.rs`（531）。
  - ぱすたの `\![set,zorder,s0,b0,s1]` は、areka では `b0,s0`（寄せる）＋`b1,s1`（補う）の順になる＝バルーンがぱすたより手前。

## Desired Outcome

- 明示の記法で書いた順のとおりに、キャラクターの窓とバルーンの窓が並ぶ。`s0,b0,s1` なら手前から ぱすた → バルーン → 窓。
- 指定が無いときと、数字だけの記法のときの重なりは今のまま（バルーンはキャラクターの直上）。
- `\![reset,zorder]` で、組の結び目（バルーンはキャラクターの直上）に戻る。
- 互換の記録（`doc/COMPAT_ARCHITECTURE.md` §8）の裁量の行と、覆した完了 spec の要件の番号が書かれている。

## Approach

- 鎖は今の「持たれる窓が手前」の 1 本の鎖のまま使う。明示の記法で同じスコープのキャラクターがバルーンより手前のときは、そのスコープの組の結び目（バルーン → キャラクター）を外し、鎖の中で書かれた順に結び直す。ぱすたの辞書のコメントにある SSP の作り（「手前の窓を奥の窓の所有ウィンドウにして重ねている」）と同じ形。
- グループが解けたら、組の結び目を張り直す（今の `zorder_pair_establish.rs` の手順）。
- 間に他のスコープが挟まる指定（非隣接・研究 R6）も、書かれた順のまま採るかを要件で決める。
- 「片方だけ名指し」の補い（要件 2.6）は残す見込み（ぱすたは `s1` だけを書き、`b1` は補われる）。

## Scope

- **In**: 明示の記法の書かれた順のままの並び（逆転・非隣接）・そのときの組の結び目の外し方と張り直し・窓の出入り（バルーンが後から出る・消える）での鎖の繋ぎ直し・記録・決定論のテスト・実機の確かめ（ぱすたの重なり）・互換の記録と網羅台帳の行。
- **Out**: 数字だけの記法の意味（変えない）・指定が無いときの重なり（変えない）・`currentghost.seriko.zorder` のプロパティ（`zorder-property`）・常に手前（`stay-on-top-directives`）・SSP の癖（バルーンが一度出るまで指定が効かない・読み直しの後に並べ直しが要る）を写すこと。

## Boundary Candidates

- 指定の読み取りの決まり（`zorder_group_ledger.rs`）
- 並びから鎖を組む所（`zorder_chain_compose.rs`）
- wintf の組の結び目と鎖の繋ぎ目（`zorder_pair*`・`zorder_chain*`）

## Out of Boundary

- 窓の位置と大きさ（変えない）
- バルーンの見える・見えないの寿命

## Upstream / Downstream

- **Upstream**: 完了 `scope-zorder-pinning`・完了 `ghost-window-zorder`（覆す要件の出どころ）。
- **Downstream**: `pasta-in-windows-verification`（重なりの確かめ）・`zorder-property`（プロパティの読み書きは逆転した並びも返す）。

## Existing Spec Touchpoints

- **Extends**: なし（完了 spec は直さず、この spec が新しい決まりを持つ）。
- **Adjacent**: `stay-on-top-directives`（同じ鎖を常に手前の帯へ上げる＝wintf の `zorder_chain_apply.rs`・`zorder_pair.rs` が重なる）・`zorder-property`（`zorder_group_ledger.rs`）・`zorder-chain-residue`（据え置き・wintf の鎖のテスト）・`check-script-arg-checks`（`zorder_group_ledger.rs` を挙げる）。

## Constraints

- 毎巡の観測と是正で並びを保つ形へ戻さない（完了 spec の要件 14＝持ち主の鎖で OS に保たせる）。
- 1 フレーム遅らせて並べ直す解は取らない。
- 失敗の道は全部記録を残す（`error!` か `warn!`＋`Err`）。
- wintf と areka のテストのファイルは大きい（`zorder_group_ledger_tests.rs` 824 行・`zorder_chain_compose_tests.rs` 811 行・`zorder_pair_tests.rs` 809 行・`zorder_chain_order_tests.rs` 977 行）＝足すテストは新しい兄弟のファイルへ。

## 2026-10-18 棚卸㉗の見立て（main 2d86dbff）

- 触るファイル: `crates/areka/src/placement/zorder_group_ledger.rs`（633）・`zorder_chain_compose.rs`（202）・`crates/wintf/src/ecs/window/zorder_pair.rs`（884）・`zorder_pair_establish.rs`（197）・`zorder_pair_maintain.rs`（531）・`zorder_chain.rs`（450）・`zorder_chain_apply.rs`（799）・新しい兄弟のテストのファイル（印: 新）・`doc/COMPAT_ARCHITECTURE.md` §8・網羅台帳 `sakura-script.toml` の `\![set,zorder]` の行。
- 規模: 8〜12 タスク（読み取りの決まり 2・鎖の組み方 2・組の結び目の外しと張り直し 2〜3・窓の出入り 1〜2・実機と記録 1〜2）。
- 先に要るもの: 働きの上では無し。ファイルの上では `stay-on-top-directives`（wintf の鎖と組）と同じウェーブに置かない。
- 優先度の区分: A（開発者の指示＝v0.1.0 のゴール・ぱすたが SSP と同じレベルで動く）。
- 版: v0.1.0。
- 要件定義のモデル: Opus（先例＝完了 `scope-zorder-pinning` の持ち主の鎖と、ぱすたの辞書が書く SSP の作り。覆すかどうかは開発者の決めで、作りの骨格は今の鎖のまま）。要件の段で「組の結び目を wintf の公開の宣言 `KeepDirectlyAbove` ごと作り替える」形を選ぶなら、wintf の公開の約束を先例なしに変える＝Fable に上げる。
- 要件の段の議題:
  1. **完了 spec の要件を覆すか**（開発者の決め）。覆すのは `scope-zorder-pinning` の要件 2.4・6.3（と 12.2 の裁量の行）。さらに出どころの `ghost-window-zorder` の要件 1.1・1.4 も「明示の記法で逆転を書いたスコープでは」覆すことになる。推し: 覆す（正典の「厳密に制御される」・SSP 2.9.18 の実際・ぱすたが要る）。覆さないなら、ぱすたは文字の裏に回ったまま（v0.1.0 のゴールの線の外）。
  2. **非隣接の指定**（`b1,s0,s1,b0` のように同じスコープの 2 枚の間に他が挟まる）も書かれた順のまま採るか。推し: 採る（正典は縛りを書かない）。SSP で測る。
  3. **キャラクターの窓を押したとき**: 今はキャラクターを押すとバルーンをすぐ手前へ連れて上がる（`ghost-window-zorder` 要件 1.2）。逆転しているスコープでは何を連れて上がるか。推し: 鎖ごと上がる（鎖の中の順は変わらない）。
  4. **補いの残し方**: `s1` だけを書いたときに `b1` をすぐ手前へ補う決まり（要件 2.6・`b1` だけなら `s1` をすぐ奥へ）を、逆転を許した後も残すか。名指しされなかった窓を SSP がどこに置くかを SSP で測る。
