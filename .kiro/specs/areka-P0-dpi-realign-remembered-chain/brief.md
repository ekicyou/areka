# Brief: areka-P0-dpi-realign-remembered-chain

> 2026-10-05 起票（`/kiro-discovery`）。出どころは spec `areka-P0-char-position-save-on-exit` の設計の検証（`.kiro/specs/completed/areka-P0-char-position-save-on-exit/design-validation.md` 3 章 議題 1）と設計ディスカッション 議題 1。開発者の裁定は「案イ＝受け入れず、別の spec として起票」。

## Problem

- **利用者**: Windows の拡大率を変える（例: 200% → 100%・拡大率の違うモニターへ移る）と、キャラクターの絵の幅が変わり、隣り合っていた本体と相方のあいだに隙間が開く（実測 359px）。隙間が開いたまま戻らない。
- `char-position-save-on-exit` が入る前は、一度もドラッグしていないキャラクターは拡大率が変わったあと本体の隣へ詰め直されていた。同 spec が入ると、2 回目の起動からは全員が「位置を覚えているキャラクター」になり、誰も詰め直されなくなる。直っていた動きが後退する。

## Current State

- 拡大率が変わったあとの詰め直しは `crates/areka/src/placement/chain_realign.rs`（完了 spec `areka-P0-dpi-transition-atomicity` 要件 6.1〜6.3・6.6）。対象の決め方は起動の最後の並べ直しと同じ判定 `finalize_chain`（`placement/chain_finalize.rs`）をそのまま使い、「既定の位置」`GhostWindows::default_char_pos` が `None` のスコープ（＝記憶から戻したスコープ）を必ず外す。
- `char-position-save-on-exit` は、起動の最後に並べ終えた時点で記憶に位置が無いキャラクターの位置を書く。次の起動からそのスコープは `None` になる。
- 両方のキャラクターをドラッグ済みの利用者は、今でも詰め直されない（同じ隙間が開く）。
- `char-position-save-on-exit` の実装のときに足した 1 件（要件 1.7 の是正）: 並べ終える前に台本が縦にだけ動かした相方は、並べ直しのあとも既定の y が元のまま残り「動かされた」と扱われるので、同じ起動の中の詰め直しの対象からも外れる（並べ終えた後に台本で動かした窓と同じ扱い。`emo2_boot/frame/drain_resnap.rs` の `finalize_chain_once_with` の既定の位置を揃える所）。対象の決め方を見直すときに合わせて扱う。

## Desired Outcome

- 拡大率が変わる前に隣り合っていたキャラクターは、位置を覚えていても、拡大率が変わったあとも隣り合ったままに見える。
- 利用者が離して置いたキャラクターどうしは、勝手に寄せない。
- 詰め直した位置を記憶に書くか書かないかを決める（`position-persist` の要件 1.9・5.4 と `char-position-save-on-exit` の要件 3.2「画面の変化では書かない」と突き合わせる）。
- 決定論のテストで「全員が記憶ありの状態で拡大率を変える → 隙間が開かない」を固定し、実機（emo2・拡大率の切り替え）で確かめる。

## Approach

要件の段で決める。候補:

- **隣り合いを見て詰め直す**: 拡大率が変わる直前に「隣と接していた（隙間が一定以下）」組を、記憶の有無に関わらず詰め直しの対象にする。
- **位置を拡大率に合わせて運ぶ**: 詰め直しではなく、覚えている位置そのものを拡大率の比で運ぶ（DPI 追従の基本設計に沿うかを確かめる）。
- **受け入れて記録だけ残す**（開発者が 2026-10-05 に採らなかった案。再検討はしない）。

## Scope

- **In**: 拡大率が変わったあとの詰め直しの対象の決め方・詰め直した位置の記憶の扱い・決定論のテスト・実機の確認。
- **Out**: 起動の最後の並べ直し（`finalize_chain` の起動時の使い方）・位置を記憶に書く時機そのもの（`char-position-save-on-exit` の持ち分）・拡大率の切り替えの途中の見え方（`dpi-transition-two-tick-bounce`）。

## Boundary Candidates

- 詰め直しの対象の判定（`placement/chain_realign.rs`・`chain_finalize.rs` の判定の共有のしかた）
- 位置の記憶との関係（`placement/persist.rs`）

## Out of Boundary

- 表情の差し替えで絵の大きさが変わったときにキャラクターが横へ動かない約束（`scope-chain-gap` 7.4・`dpi-transition-atomicity` 要件 6.6）は変えない。

## Upstream / Downstream

- **Upstream**: `areka-P0-char-position-save-on-exit`（これが入って初めて後退が表に出る）・完了 `areka-P0-dpi-transition-atomicity`・完了 `areka-P0-position-persist`。
- **Downstream**: `extra-character-windows`（3 体目以降の隣り合い）。

## Existing Spec Touchpoints

- **Extends**: なし（完了 spec `dpi-transition-atomicity` の詰め直しの対象の決め方を改める。改めた記録は `doc/COMPAT_ARCHITECTURE.md` に残す）。
- **Adjacent**: `placement/` を触る spec と同時に走らせない。`dpi-transition-two-tick-bounce` と `chain_realign.rs` の周辺で接する。

## Constraints

- 段は**バグ**（`char-position-save-on-exit` の着地で、直っていた動きが 2 回目の起動から後退する）。着手は `char-position-save-on-exit` の着地の後。
- 1 フレーム遅らせて解く形は取らない。DPI 追従は基本設計。

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**:
  - **前提の `char-position-save-on-exit` が着地した（✅ 10-08・PR#264）＝後退はもう main に出ている**。起動の最後に並べ終えた時点で、記憶に位置が無いキャラクター窓の位置を書く（`crates/areka/src/placement/persist.rs` の `persist_unremembered_char_positions`・呼び手は `crates/areka/src/emo2_boot/frame/drain_resnap.rs` の起動の並べ直しの 1 か所）。位置を書く時機は「ドラッグの確定」と「並べ終えた時点」の 2 つで、終了の時には書かない（spec の名前と中身が違う）。
  - **「覚えている」の今の意味**: 窓の一式 `GhostWindows` の既定の位置が無い（`default_char_pos` が `None`）スコープ＝記憶から戻したスコープ（`crates/areka/src/placement/spawn.rs`）。記憶が空の初回の起動は全員に既定の位置があるので詰め直される。2 回目の起動からは全員 `None` で、詰め直しの判定 `finalize_chain`（`placement/chain_finalize.rs`＝既定の x が今の x と同じスコープだけ動かす）が全員を外す。
  - Current State の記述はどれも main と合う（`placement/chain_realign.rs` 245 行。拡大率の相で武装する所は `emo2_boot/frame/dpi.rs`、解く所は `frame/drain_resnap.rs` の `realign_chain_once`）。
- **触るファイル**: `crates/areka/src/placement/chain_realign.rs`（245）・`chain_finalize.rs`（320・判定を分けるなら）・`spawn.rs`（775・「隣り合っていた」を覚える欄を足すなら）・`persist.rs`（642・詰め直した位置を書くなら）・`crates/areka/src/emo2_boot/frame/{dpi.rs, drain_resnap.rs}`（508・533）・`emo2_boot/frame.rs`（568・新しいテストの宣言）・兄弟のテスト（`emo2_boot/frame_chain_realign_tests.rs` 593・`frame_chain_realign_arm_tests.rs` 347・`placement/chain_finalize_tests.rs` 505・`placement/persist_restore_tests.rs` **939**・土台 `emo2_boot/frame_test_support.rs` **902**＝足すなら新しいファイルへ）・`doc/COMPAT_ARCHITECTURE.md` §8。**触らずに済ませる**: `placement/follow/window_move.rs`（1,221・行数の番人の例外）・`placement/transition_judge.rs`（**996**）とそのテスト 2 本（1,039・1,037・例外）＝新しい書き込みの経路を足さず、今ある「詰め直し」の経路を使えば拡大率の切替の判定器に触れない。
- **規模**: S〜M（6〜9。roadmap の S〔4〜8〕から少し上げた＝下の穴を範囲に入れる分）。**分割の案**: なし。
- **先に要るもの**: なし（前提は着地済み）＝今すぐ着手できる。
- **ファイルの重なり**: 着手の候補（バグ・優先の段）とは 0。`placement/` を触る未完了は `extra-character-windows`（`spawn.rs`・`persist.rs`・その他の段・本 spec の下流）・`clippy-199-lints` の 2 段目（`spawn.rs`・`persist.rs` の注記）・`placement-measure-bake-once`（`mod.rs`・`measure.rs`＝別のファイル）・`zorder-property`（`zorder_group_ledger.rs`＝別）・`balloon-canon-residue`（`config.rs`・`resolver.rs`＝別）・据え置きの `dpi-transition-two-tick-bounce`（`frame/dpi.rs`）。`emo2_boot` の結線の列のファイル（`mod.rs`・`ghost_switch.rs`・`frame/{attach,switch,wiring}.rs`）には触れない。
- **優先度の区分**: B（バグ＝直っていた動きの後退・開発者が「受け入れず、別の spec として起票」と裁定）。
- **要件定義のモデル**: Fable（決め方の分かれ目が 2 案・拡大率への追従は基本設計・位置の記憶の決まり 3 つと突き合わせる）。
- **見つけた穴・古くなった記述**:
  - **同じ隙間は、起動と起動のあいだに拡大率が変わったときにも出る見込み**（コードの読みだけ・実機では未確認）。覚えている位置は「下端の中央の x」を物理ピクセルで保存し（`placement/persist.rs` の `char_pos_to_origin_x`）、起動の最後の並べ直しも覚えているスコープを外すので、拡大率を変えてから起動すると幅だけが変わって隙間が開く。Scope の Out「起動の最後の並べ直し」と当たる＝要件で範囲に入れるかを決める。絵の幅が違うシェルへ切り替えた後の起動も同じ形になりうる（確かめていない）。
  - `placement/chain_realign.rs` の冒頭の説明「誰も触っていないスコープは対象に残る」は、2 回目の起動からは成り立たない＝本 spec が判定と一緒に書き直す。
