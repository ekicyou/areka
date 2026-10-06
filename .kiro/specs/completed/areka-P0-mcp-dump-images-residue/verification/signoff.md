# 実機確認の記録（タスク 5.1・要件 1.1・1.4・4.3・7.2・7.3）

- **実施日**: 2026-10-06（記録の時刻は UTC で 2026-10-05 19:38〜19:39）
- **判定に使った版**: コミット `03bf2486`（タスク 4.2 の後）。配布物の `BUILD-INFO.txt` は `commit=03bf248`・`dirty=0`
- **前提の検査**: `cargo test -p areka --bin areka` 2,687 件 緑（無視 2）・`cargo test -p areka-emo-text` 緑・`cargo fmt --all -- --check` 緑。clippy（`-D warnings`）は既存の指摘で赤のまま（起票済みの `clippy-199-lints`）。本 spec で変えたファイルに当たる指摘は 0 件
- **ビルド**: `pwsh -NoProfile -File tools/package.ps1`（全段 緑）→ `target\package\areka-0.0.1-x64.zip`（配布用のビルド）
- **置き場（要件 7.3）**: どれもこのワークツリーの `target\` の下（絶対パス）
  - 展開先: `C:\home\maz\git\areka\.claude\worktrees\areka-p0-mcp-dump-images-7ad1fb\target\signoff-residue\a\`
  - 記録: `...\target\signoff-residue\logs\`（`run1.out.txt`・`run1.err.txt`）
  - 応答と PNG: `...\target\signoff-residue\out\`（`b1.json`〜`b6.json`・`b1.png`〜`b6.png`）
- **画面**: 内蔵の画面（主の画面）・表示の拡大率 200%（`AppliedDPI` 192）
- **ゴースト**: 既定ゴースト（emo2・バルーン `emo2-kakukaku`）
- **起動のしかた**: `AREKA_*`／`WINTF_*` を全部外し、`AREKA_APP_SMOKE_EXIT_MS=1200000`・`NO_COLOR=1`・`RUST_LOG=info,areka::mcp=debug,areka_mcp=debug` を付けて `Start-Process -PassThru` で展開先の `areka.exe` を起こした（pid 34224）。止めるときは、その pid にだけ `taskkill`（強制なし）を送った。「きれいに終わったので起動中の印を消しました」と「待受を閉じた」の記録が出ている
- **呼び出し**: PowerShell の `Invoke-WebRequest` で `initialize` の後に `tools/call`（`dump_balloon`・引数なし）を 2.5 秒おきに 6 回送った。待受は `http://127.0.0.1:9801/api/mcp/v1`

## 判定

| # | 確かめたこと | 結果 |
|---|---|---|
| 7.2 ⑴ | 台詞を出した後に `dump_balloon` を 5 回以上（最初の 1 回を含む）呼び、成功の記録の `ui_us` がすべて 2 ms 以内 | **合**（6 回。最初の 1 回が最大で 939 µs、ほかは 277〜429 µs。撮ったバルーンは 400×224 物理 px） |
| 7.2 ⑵ | 返った絵に背景と文字が入り、原寸で返る | **合**（6 回とも 400×224・左上の角のアルファ 0（透過）。バルーンの画像 `balloon\emo2-kakukaku\balloons0.png` の原寸 400×224 と同じ。拡大率 200% の画面でも原寸。開いて背景の枠と字が入っていることを確かめた） |
| 7.2 ⑶ | 呼んでいる間もゴーストの描画と会話が止まらず、ERROR の記録が 0 件 | **合**（呼んでいる 13 秒の間も台詞が進み、返った絵が b1→b4 で 4 通りに変わった。表情の切り替え（`apply(ShowSurface)`・`seriko: bind 適用`）も続いた。ERROR は走行の全体で 0 件（標準出力・標準エラー出力の両方）） |
| 4.3 | 件 5 の確かめの結果を、定義の名前で再び確かめられる形で残す | **合**（下の「件 5 の確かめ」） |

## 7.2 ⑴⑵ 呼び出しごとの記録

成功の記録（`debug!`）:

```
19:39:01.598538Z DEBUG areka::mcp::dump_balloon: [mcp] 絵を返す tool="dump_balloon" scope=0 width=400 height=224 base64_len=19256 ui_us=939 encode_us=8712
19:39:04.221863Z DEBUG areka::mcp::dump_balloon: [mcp] 絵を返す tool="dump_balloon" scope=0 width=400 height=224 base64_len=14196 ui_us=379 encode_us=7161
19:39:06.764415Z DEBUG areka::mcp::dump_balloon: [mcp] 絵を返す tool="dump_balloon" scope=0 width=400 height=224 base64_len=16212 ui_us=429 encode_us=7705
19:39:09.313361Z DEBUG areka::mcp::dump_balloon: [mcp] 絵を返す tool="dump_balloon" scope=0 width=400 height=224 base64_len=16328 ui_us=277 encode_us=7158
19:39:11.854735Z DEBUG areka::mcp::dump_balloon: [mcp] 絵を返す tool="dump_balloon" scope=0 width=400 height=224 base64_len=16328 ui_us=284 encode_us=6807
19:39:14.413380Z DEBUG areka::mcp::dump_balloon: [mcp] 絵を返す tool="dump_balloon" scope=0 width=400 height=224 base64_len=16328 ui_us=398 encode_us=6815
```

| 呼び出し | 要求を受けた（UTC） | 答えた（UTC） | 要求→答え | `ui_us` | `encode_us` | 返った PNG | SHA256 の頭 16 字 | 絵の字 |
|---|---|---|---|---|---|---|---|---|
| b1 | 19:39:01.575 | 19:39:01.606 | 30.7 ms | 939 | 8,712 | 400×224 | `E4B708F537C97624` | 「はじめましてや！！うちは、むらさき。」「そこ自分でいう・・・・。」 |
| b2 | 19:39:04.202 | 19:39:04.223 | 20.8 ms | 379 | 7,161 | 400×224 | `A286DCC393DD8442` | （b1 と違う絵） |
| b3 | 19:39:06.744 | 19:39:06.773 | 28.7 ms | 429 | 7,705 | 400×224 | `C81E115265430D6F` | （b2 と違う絵） |
| b4 | 19:39:09.291 | 19:39:09.315 | 23.4 ms | 277 | 7,158 | 400×224 | `597A247F6750F8B8` | 「OK？」「まあ、これから、よろしゅうに！」 |
| b5 | 19:39:11.836 | 19:39:11.857 | 21.2 ms | 284 | 6,807 | 400×224 | `597A247F6750F8B8` | b4 と同じ（台詞が出終わって次を待っている間） |
| b6 | 19:39:14.389 | 19:39:14.415 | 25.9 ms | 398 | 6,815 | 400×224 | `597A247F6750F8B8` | b4 と同じ |

- 本文はどれも `OK:balloon of scope 0 as last drawn (before scaling and transparency; kept even if the balloon is hidden now)`・`isError=false`。
- `ui_us` は 1 フレームで UI スレッドを塞いだ時間の最大（符号化のスレッドを起こす時間を含む）。最大は最初の 1 回の 939 µs で、線（2,000 µs）の内。前の spec の実機確認（走行 3）では、同じ `dump_balloon` の最初の 1 回が 14,233 µs（文字の面を GPU から読み戻す間 UI スレッドが待っていた）だった。
- 「要求を受けた」は MCP のスレッドの `Service initialized as server` の記録、「答えた」は `MCP: 要求に応えた method=tools/call` の記録の時刻。要求から答えまでの 20.8〜30.7 ms には、表示のスレッドへ渡して汲まれるまでの待ち・読み出しを待つフレーム・符号化のスレッドの時間（`encode_us` 6.8〜8.7 ms）が入る。答えが届くまでの遅れは 1〜2 フレームの見込み（design の Performance の節）と合う。

## 7.2 ⑶ 描画と会話

- 呼び出しの間（19:39:01〜19:39:15）も、表示の層の `apply(ShowSurface)` と `seriko: bind 適用` の記録が途切れずに出ていた（例: 19:39:01.785〜01.800、19:39:03.641〜03.652、19:39:04.341〜04.356）。返った絵も b1→b2→b3→b4 で台詞の進みどおりに変わった。
- `areka_ghost::ticker` の `loop ticker catch-up: skipped multiple boundaries`（INFO）が 19:38:57・19:39:05・19:39:31 に出ているが、呼び出しの前（19:38:57）と呼び出しを終えた後（19:39:31）にも出ており、呼び出しとは関わらない。
- 記録の段の数（走行の全体・起動から終了まで）: ERROR 0 件。WARN 3 件はどれも起動のときの検体のゴースト・バルーンの作りに由来するもの（`bake: element が全透明（α=0）でトリム後 0 寸です` 2 件・`折返し基準が描画範囲の外に解決された` 1 件）で、呼び出しの間には 0 件。

## 件 5 の確かめ（要件 4.3）

**結果: 本 spec の前の areka では起きうる道が 1 本あった。本 spec の後は、その道では `NG:Specified ghost is not active` で答える。** 道筋の調べは research.md の「4. 件 5（預けている間にゴーストが替わる）の確かめ」、設計の扱いは design.md の「件 5 の確かめ（4.3）」。

道筋（ファイルと定義の名前。2026-10-06 に定義が在ることを grep で確かめた）:

1. ゴースト A から B へ切り替える。`crates/areka/src/emo2_boot/ghost_switch.rs` の `switch_to` が A を降ろし、B を起こす。
2. B の窓は `crates/areka/src/ghost_session.rs` の `commit_ghost_windows` が後から作るので、B の `Emo2Wiring::attached`（`crates/areka/src/emo2_boot/frame/wiring.rs`）は少なくとも 1 フレーム偽のまま。
3. その間に届いた `dump_surface`／`dump_balloon` は、`crates/areka/src/mcp/dump_surface.rs` の `start` が覗く関数の状態 `Wait`（段 `Stage::Attaching`）にして `mcp::later` へ預ける。
4. B の SHIORI が装着の前に止まると、`crates/areka/src/emo2_boot/frame.rs` の `run_ghost_quit_phase` → `ghost_switch.rs` の `on_ghost_stopped` → `switch_to_default` で既定のゴーストへ戻る。
5. 本 spec の前は、預けた覗く関数が毎フレーム結線状態を読み直すので、既定のゴーストの絵を B の名前を添えて返していた。本 spec の後は、`Wait::attach`（`dump_surface.rs`）が覗くたびに `resolve::active` と呼び出しを解決したゴースト（`ActiveGhost`＝名前とルートフォルダ）を比べるので、B が去った時点で `NG:Specified ghost is not active`（`debug!`・ERROR なし）で答える（要件 4.2）。写した後の段（`Stage::Reading`・`Stage::Encoding`）は写した時点の絵を返す（要件 4.5）。

決定論テストでの固定（要件 4.4・実機では起こさない）: `crates/areka/src/mcp/dump_surface_tests.rs` の `switch_to_another_ghost_answers_not_active_without_mcp_errors`（台本の `\![change,ghost,B]` で本物の切り替えを通す）と `switch_to_the_same_ghost_keeps_waiting_for_attachment`（同じゴーストの起こし直しは同じと見なす）。

## 結論

要件 7.2 ⑴⑵⑶・7.3・4.3 はすべて**合**。`ui_us` は 6 回とも 2 ms 以内（最大 939 µs）で、design.md の実機確認の節が定める「2 ms を超えたら否・起票」には当たらない。
