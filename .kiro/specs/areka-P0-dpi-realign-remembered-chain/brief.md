# Brief: areka-P0-dpi-realign-remembered-chain

> 2026-10-05 起票（`/kiro-discovery`）。出どころは spec `areka-P0-char-position-save-on-exit` の設計の検証（`.kiro/specs/areka-P0-char-position-save-on-exit/design-validation.md` 3 章 議題 1）と設計ディスカッション 議題 1。開発者の裁定は「案イ＝受け入れず、別の spec として起票」。

## Problem

- **利用者**: Windows の拡大率を変える（例: 200% → 100%・拡大率の違うモニターへ移る）と、キャラクターの絵の幅が変わり、隣り合っていた本体と相方のあいだに隙間が開く（実測 359px）。隙間が開いたまま戻らない。
- `char-position-save-on-exit` が入る前は、一度もドラッグしていないキャラクターは拡大率が変わったあと本体の隣へ詰め直されていた。同 spec が入ると、2 回目の起動からは全員が「位置を覚えているキャラクター」になり、誰も詰め直されなくなる。直っていた動きが後退する。

## Current State

- 拡大率が変わったあとの詰め直しは `crates/areka/src/placement/chain_realign.rs`（完了 spec `areka-P0-dpi-transition-atomicity` 要件 6.1〜6.3・6.6）。対象の決め方は起動の最後の並べ直しと同じ判定 `finalize_chain`（`placement/chain_finalize.rs`）をそのまま使い、「既定の位置」`GhostWindows::default_char_pos` が `None` のスコープ（＝記憶から戻したスコープ）を必ず外す。
- `char-position-save-on-exit` は、起動の最後に並べ終えた時点で記憶に位置が無いキャラクターの位置を書く。次の起動からそのスコープは `None` になる。
- 両方のキャラクターをドラッグ済みの利用者は、今でも詰め直されない（同じ隙間が開く）。

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
