# Brief: areka-P0-char-position-save-on-exit

> 2026-10-05 起票（`/kiro-discovery`）。出どころは spec `areka-P0-mouse-drag-events` の実機 R7 と完了時の棚卸（`.kiro/specs/completed/areka-P0-mouse-drag-events/tasks.md` の Implementation Notes ⑶・`verification/real-machine.md` 3 章 R7・4 章 ⑶）。開発者（2026-10-05）「初回起動時に位置保存されていないということ？それはちょっと不自然」「起票として記憶しておいて」。

## Problem

- **利用者**: 本体だけをドラッグして終了し、起動し直すと、本体は動かした位置に立つが、相方まで動く。前回閉じたときに見えていた並びで立ち上がらない。
- 原因は、一度もドラッグしていないキャラクターの位置が記憶に無いこと。相方は起動のたびに既定の位置から始まり、起動の最後に本体の左隣へ並べ直される。

## Current State

- キャラクター窓の位置を記憶（ゴーストの `profile\areka\sylphya.toml` の `[window.N]`）に書くのは、ドラッグの終了だけ。`crates/areka/src/placement/follow/drag_follow.rs` の `on_char_drag_end` が `char_pos_entries`→`placement::persist::persist_entries` を呼ぶ。doc に「永続の窓位置を書くのはこの DragEnd 観測点のみ」（発火規律・完了 spec `position-persist` の Req1.9）。
- 終了するとき・最初に並べ終えたとき（`emo2_boot/frame/drain_resnap.rs` の `chain_finalize`）・SHIORI の移動の指示（`\![move]` など）で動いたときは書かない。
- 実測（`mouse-drag-events` の R7b）: `merge_scope restore scope=1 … saved_win_x=None` → `chain_finalize: … scope=1 from_x=1548 to_x=503`（本体の 1169 から窓の幅 666 を引いた位置）。SHIORI の台詞に移動の指示は無かった。

## Desired Outcome

- 正常に終了したとき、表示している全部のキャラクター窓の位置が記憶に書かれ、次の起動で同じ並びに立つ。
- 書く時機の候補（終了時・初期配置の確定時・SHIORI の移動の指示の後）を、完了 spec `position-persist` の発火規律（Req1.9＝ドラッグ中や再射影では書かない）と突き合わせて決める。きれいに終わらなかった回（`session-running-mark`）での扱いも決める。
- 決定論のテストで「本体だけ動かして終了 → 再起動で相方も前回の位置」を固定し、実機で確かめる。

## Approach

`position-persist` の design（書く口が DragEnd だけの理由）を読み直し、終了の経路（`app-lifetime-separation` の正規の終了）に全キャラクターの位置を書く口を 1 つ足す案を軸にする。要件の段で書く時機を裁定する。

## Scope

- **In**: 書く時機の追加・終了の経路への結線・発火規律の改訂と記録・決定論のテスト・実機の確認。
- **Out**: 復元の規則（画面の外の補正など）・バルーンの相対位置の保存（今のまま）。

## Boundary Candidates

- 位置の保存（`placement::persist`・`drag_follow.rs`）
- 終了の経路（正規の終了の順序）

## Out of Boundary

- `chain_finalize` の並べ方そのもの（位置が記憶にあれば使われない）。

## Upstream / Downstream

- **Upstream**: 完了 `position-persist`・`app-lifetime-separation`・`session-running-mark`。
- **Downstream**: `extra-character-windows`（3 体目以降も同じ口で書く）。

## Existing Spec Touchpoints

- **Extends**: なし（完了 spec `position-persist` の設計の抜けを埋める）。
- **Adjacent**: `placement/` を触る spec と同時に走らせない。

## Constraints

- 段は**バグ**（再起動で並びが崩れる・開発者「不自然」）。

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: S（4〜8）のまま。切らない。
- 前提の状態: 上流は全部完了・今すぐ着手できる。起票（`1bb449ae`）の後に `crates/areka/src/placement/`・`emo2_boot/frame/drain_resnap.rs`・`session_end*.rs`・`main.rs` を触ったコミットは 0。
- 崩れた前提／古くなった位置: 無し。書く所は今も `placement/follow/drag_follow.rs` の `on_char_drag_end`（キャラクターの位置）と同じファイルのバルーンのドラッグの終わり（バルーンの相対位置）の 2 か所だけ。書く口 `placement/persist.rs` の `persist_entries` は sylphya のアクターへ投げっぱなし（返事を待たない）で、その送り口 `PersistWiring` は `main.rs` の `insert_persist_wiring` と `ghost_session.rs` の切替の結線で**ゴーストごとに差し替わる**。
- 見つけた穴（要件の段で確かめる）:
  1. ゴーストの切替でも同じ問題が起きる見込み（切替で降ろしたゴーストの相方の位置も書かれない）。書く時機を「アプリの終了」だけにすると切替が漏れる。
  2. 投げっぱなしの書き込みが、降ろすときの sylphya のアクターの確定より先に届くことを、順序で保証する必要がある（終了の通知 `KanadeNotice::Stopped` を受けて窓を閉じる `emo2_boot/frame.rs` の `quit_as_today` の時点では、ゴーストの側はもう降りている見込み＝そこで書いても届かない恐れ）。
- 触るファイル: `crates/areka/src/placement/persist.rs`（全キャラクターの位置を集めて書く口）・`placement/follow/drag_follow.rs`（発火規律の doc の改訂）・終了と切替で降ろす前の 1 か所（`emo2_boot/frame.rs` か `emo2_boot/ghost_switch.rs`、または kanade の終了の系列に入る前の areka の側）・`session_end.rs`（OS のセッションの終了も同じ口を通すなら）・兄弟のテスト（`placement/persist_entries_tests.rs` か新規）・完了 `position-persist` の Req1.9 の改訂の記録（`doc/COMPAT_ARCHITECTURE.md` §8）。
- 議題:
  1. 書く時機: ゴーストを降ろす共通の入口（終了・切替・OS のセッションの終了）にするか、アプリの終了だけか。
  2. きれいに終わらなかった回（`session-running-mark`）は書けないまま＝前回の位置で立つのでよいか。
- 同時に走らせない: `placement/` を触る spec（今は列に無い）・`extra-character-windows`（3 体目以降も同じ口）。

### 同じウェーブ C4 の約束（2026-10-05 棚卸㉒・破るなら止めて報告）

- `emo2_boot/mod.rs`・`spine.rs`・`frame/wiring.rs` に触らない（`balloon-lifecycle-events`・`ghost-session-test-load-flake` の持ち物）。終了やゴーストを降ろす前の共通の入口が `spine.rs` にあると分かったら止めて報告。
