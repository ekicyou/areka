# Brief: areka-P0-dump-balloon-debug-timeout

> 2026-10-07 `ghost-standard-balloon` の完了時の棚卸で起票（`completed/areka-P0-ghost-standard-balloon/tasks.md` の Implementation Notes の 6.2・最終検証 ⑶）。

## Problem

2026-10-06、`ghost-standard-balloon` の実機の確かめ（debug 版の `target\debug\areka.exe`・claudia）で、MCP の `dump_balloon` を呼ぶと呼び出し側の時間切れ（"task cancelled"）になり、答えが返らなかった。バルーンの絵を撮れないと、エージェントがバルーンの見た目を確かめられない。

## Current State

- 起きたのは main `d3000dee` の時点のコード（`mcp-dump-images-residue` の着地の前）。原因は調べていない。
- その後 main に入った `mcp-dump-images-residue`（PR#254・squash `c37d9d61`）は、バルーンの文字の面を UI スレッドで待たずに読み戻すよう直した。この直しで解けている見込みが高いが、**確かめていない**。
- release 版で同じことが起きるかも分からない。

## Desired Outcome

- main の debug 版で `dump_balloon` が、呼び出し側の時間切れより前に答え（絵、または `isError: true` の理由）を返す。
- 再現しない場合は、再現を試みた条件（版・ゴースト・バルーン・呼び方）を記録して閉じる（rejected）。

## Approach

まず再現を確かめる。main の debug 版で claudia（`balloon\claudia`）と emo2 を起こし、MCP の `dump_balloon` を繰り返し呼ぶ。再現したら、待ちがどこで止まっているかを trace の記録で突き止めて根本を直し、決定論テストを 1 本添える。

## Scope
- **In**: `dump_balloon` の答えが返らない原因の特定と修正・再現の記録。
- **Out**: `dump_surface` の変更（同じ形で再現したときだけ範囲に入れる）・MCP の時間切れの値の引き上げ（症状止めはしない）。

## Boundary Candidates
- MCP の受け口（`dump_balloon` の要求を UI スレッドへ渡し、答えを待つ所）
- バルーンの読み戻し（`mcp-dump-images-residue` が直した所）

## Out of Boundary
- バルーンの描き方・装着の待ちの段数の変更

## Upstream / Downstream
- **Upstream**: `mcp-dump-images`（✅ PR#242）・`mcp-dump-images-residue`（✅ PR#254）
- **Downstream**: 実機の確かめで `dump_balloon` を使うすべての spec

## Existing Spec Touchpoints
- **Extends**: なし（完了 spec は直さない）
- **Adjacent**: `ghost-session-test-load-flake`（負荷で待ちが伸びる系の赤）

## Constraints
- 実機の根と検体はワークツリーの `target\` の下だけ。
- 32bit の SHIORI のゴーストを debug 版で起こすときは、`areka.exe` の隣に i686 の helper が要る（`dev-helper-x64-clobber` の罠）。
- 規模の見立て: 再現しなければ XS（記録だけ）、再現すれば S（4〜7）。

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化:
  - 起きたのは main `d3000dee`（10-06 の朝）のコードで、`mcp-dump-images-residue`（`c37d9d61`・10-06 の夜）より前だと git の履歴で確かめた。
  - そのときの `dump_balloon` は、バルーンの文字の面を GPU から読み戻す間、UI スレッドで待っていた（`mcp-dump-images-residue` の要件の件 1）。今の `crates/areka/src/mcp/dump_balloon.rs` は、読み戻しを後のフレームで待たずに覗き、絵の仕上げは別のスレッドで行う。UI スレッドで GPU の終わりを待つ呼び出しは無くなった＝当時の待ちの場所そのものが無い。原因が消えている見込みは高い。
  - ただし、その後の実機の記録（C4 の各 spec の実機確認）に「debug 版で `dump_balloon` が返った」と書いたものは見つからなかった＝確かめは今も済んでいない。
- 触るファイル: 再現しなければ 0（本 spec の記録だけ）。再現したら `crates/areka/src/mcp/dump_balloon.rs` とその兄弟（待ちの段）か、後から答える仕組み（`crates/areka/src/mcp/mod.rs` の `later`）。
- 規模: 再現しなければごく小さい（1〜2 タスク・main の debug 版で claudia と emo2 を起こして 5 回ずつ呼び、条件と結果を記録して閉じる）。再現したら S（4〜7 タスク）。
- 先に要るもの: なし。触るファイルが決まるまでは、どの spec とも重なり 0。
- 優先度の区分: B（バグ・実機で 1 度起きた）。ただし確かめることが本体の spec。
- 要件定義のモデル: Opus。
- 分割の案: なし。単独の席を取らない手もある: 次に実機で MCP を使う spec（`mcp-kanade-tools` は実機で `dump_surface` を呼ぶ申し送りを持つ）の実機確認に「debug 版で `dump_balloon` を 5 回」を相乗りさせ、返れば本 spec を閉じる。
- 見つけた穴・古くなった記述: なし（brief は 10-07 のまま正しい）。
