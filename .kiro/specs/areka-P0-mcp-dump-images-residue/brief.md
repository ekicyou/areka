# Brief: areka-P0-mcp-dump-images-residue

## Problem
MCP の `dump_surface`／`dump_balloon`（`areka-P0-mcp-dump-images`・2026-10-05 完了）は、実機で要件を満たしたが、完了時の棚卸で後始末が 6 件残った。いちばん重いのは、`dump_balloon` の UI スレッドの側の所要時間が 1 フレームの線に近いこと。エージェントがバルーンを続けて撮ると、ゴーストの描画が 1 フレーム飛ぶおそれがある。

## Current State
- `areka-P0-mcp-dump-images` のタスク 4.3 で、乗算を戻す・重ね合わせ・PNG・base64 を別のスレッド（`mcp-encode`）へ逃がした。UI スレッドに残るのは判断と絵の写しで、バルーンはこれに文字の面の GPU からの読み戻し（`read_back`）が加わる。
- 実機（配布用のビルド・拡大率 200%・`verification/signoff.md` の走行 3）での UI スレッドの側: キャラクター 1 枚で最大 0.31 ms、**バルーンで最大 14.2 ms**（5 回で 1,436〜14,233 µs・最大は最初の 1 回）。design の合否の線は 16 ms（キャラクター 1 枚について決めたもの）。
- 残った 6 件（出どころは `areka-P0-mcp-dump-images` の tasks.md の Implementation Notes と、完了前の `/kiro-validate-impl` の報告）:
  1. `dump_balloon` の文字の面の読み戻しが UI スレッドに残る（上の 14.2 ms）。
  2. `crates/areka/src/mcp/dump_surface.rs` で、`compose_alone` が `None` を返す届かない枝が、判断の文言 `NG:No such scope in this ghost` を `error!` つきで返す。要件 4.6（判断の失敗は ERROR を出さない）の精神と食い違う。想定外の失敗には専用の文言が要る。
  3. 装着の前に `mcp::later` へ預けた組は、`Step::here` で UI スレッドで符号化まで仕上げ、`catch_unwind` が無い。`later` の口が同期で `ToolOutcome` を返す形のため（`crates/areka/src/mcp/mod.rs`）。起きるのは起動の直後に 1 度だけ。
  4. 符号化のスレッドで出る記録は `log_capture_kit` に捕まらない（呼んだスレッドの記録だけを捕まえる）。GPU を通るテストの「ERROR 0 件」は UI スレッドの記録だけを見ている。成功の記録の `ui_us`・`encode_us` の欄を確かめるテストも無い。
  5. design-validation.md の b「`later` に預けている間にゴーストが替わると、別のゴーストの絵を返しうる」が未確認のまま（差の一覧にも実機確認にも無い）。
  6. 要件 7.7 ⑵（`sakurascript` で表情を変えた後に撮ると変わった姿が返る）は、`sakurascript` ツールが未実装のため、開発者の了承のうえキャラクターのダブルクリックで表情を変える代わりの手順で確かめた。

## Desired Outcome
- `dump_balloon` の UI スレッドの側が、文字の面の大きさに依らず線（16 ms）から十分に離れる。
- 想定外の失敗が、判断の失敗と別の文言で答え、`error!` 1 件を出す。
- `later` を通る組も、符号化のスレッドで仕上げるか、少なくとも panic を受けて `error!` と `NG:` で答える。
- 符号化のスレッドの記録をテストで数えられる（または数えられないことを前提にした別の判定がある）。
- 5 が確かめられ、起きるなら塞がれているか差の一覧に載っている。
- `sakurascript` が入った後、7.7 ⑵ を本来の手順で 1 回撮り直した記録がある。

## Approach
着手のときに決める。1 は、読み戻しの写しを非同期の GPU の写し（ステージングの面への写しを UI スレッドで積み、地図の読み出しを次のフレームか別のスレッドで行う）にするのが本筋の見込み。`emo-text` の読み戻しの口を変えるかどうかが議題になる。3 は `mod.rs` の `later` に「別のスレッドで答える」形を足すのがいちばん小さい見込み。

## Scope
- **In**: 上の 1〜6。`crates/areka/src/mcp/dump_surface*.rs`・`dump_balloon*.rs`、必要なら `crates/areka/src/mcp/mod.rs` の `later`、文字の層の読み戻しの口（1 で要るとき）。
- **Out**: 新しいツール。文言の SSP 寄せ（`mcp-strict-errors`）。`sakurascript` そのもの（`mcp-kanade-tools`）。

## Boundary Candidates
- MCP の 2 本のツールのファイル（1・2・3・4）
- 文字の層の読み戻しの口（1）
- `mcp::later` の答え方（3）

## Out of Boundary
- `sakurascript`・`raise_event`・`get_status` の実装（`mcp-kanade-tools`）
- 全体テストの負荷の下の揺れ（`ghost-session-test-load-flake`）

## Upstream / Downstream
- **Upstream**: `areka-P0-mcp-dump-images`（完了）。6 だけは `areka-P0-mcp-kanade-tools` の着地の後。
- **Downstream**: なし

## Existing Spec Touchpoints
- **Extends**: `areka-P0-mcp-dump-images`（完了・`completed/` にある）
- **Adjacent**: `areka-P0-mcp-kanade-tools`（`mod.rs` の `later` を使う見込み）・`areka-P0-mcp-strict-errors`

## Constraints
- 1 フレーム遅らせる解は取らない（状態の持ち方で 0 フレームで解く）。1 の非同期の読み戻しで「答えが 1 フレーム後になる」のは、描画を遅らせるのでなく答えを後にするだけなので可。
- 実機の根・一時ファイルはワークツリーの `target\` の下だけ。
