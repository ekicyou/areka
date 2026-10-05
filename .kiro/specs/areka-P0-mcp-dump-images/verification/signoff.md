# 実機確認の記録（タスク 6.2・要件 6.1・6.2・7.7）

- **実施日**: 2026-10-05
- **判定に使った版**: ⑴ と所要時間はコミット `b222af2e`（符号化を別のスレッドへ逃がしたタスク 4.3 の後）で判定した。配布物の `BUILD-INFO.txt` は `commit=b222af2` `dirty=0`。走行 1・2（`de80f039`）は線を越えた経緯として残す
- **前提の検査**: `b222af2e` で `tools/test-all.ps1` が全段 緑（x64 ワークスペース全テスト 393 秒）。`de80f039` でも全段 緑（同 1658 秒）。clippy（`-D warnings`）は既存の指摘で赤のまま（起票済みの `clippy-199-lints`）。本 spec で変えたファイルに当たる指摘は 0 件
- **ビルド**: `pwsh -NoProfile -File tools/package.ps1`（全段 緑）→ `target\package\areka-0.0.1-x64.zip`（配布用のビルド）
- **展開先**: `target\signoff-dump\b\`（走行 3〜・`b222af2e`）。走行 1・2 は `target\signoff-dump\a\`（どちらもワークツリーの `target\` の下・絶対パス）。記録は `target\signoff-dump\logs\`、返った応答と PNG は `target\signoff-dump\out\`
- **画面**: 内蔵の画面・表示の拡大率 200%
- **起動のしかた**: `AREKA_*`／`WINTF_*` を全部外し、`AREKA_APP_SMOKE_EXIT_MS=1800000`・`NO_COLOR=1` を付けて `Start-Process -PassThru` で展開先の `areka.exe` を起こした。止めるときは、自分で起こした pid にだけ `taskkill`（強制なし）を送った。どの走行も「きれいに終わったので起動中の印を消しました」と「待受を閉じた」の記録が出ている
- **呼び出し**: curl（Git for Windows 同梱）で `initialize` の後に `tools/call` を送った（`target\signoff-dump\mcp.sh`）。待受は `127.0.0.1:9801`

## 判定

| # | 確かめたこと | 結果 |
|---|---|---|
| ⑴ | `dump_surface`（省略）が今の姿の透過 PNG を返し、開いて画面と同じに見える | **合**（走行 3・`b222af2e`。走行 1 の `de80f039` でも合。下記） |
| ⑵ | 表情を変えた後は、変わった姿が返る | **合**（走行 3。`sakurascript` が未実装のため、キャラクターのダブルクリックで表情を変える代わりの手順。下記） |
| ⑶ | `surface` を指定しても画面は変わらず、その surface が返る | 未実施 |
| ⑷ | 台詞の後の `dump_balloon` に背景と文字が入り、消えた後も同じ絵が返る | 未実施 |
| ⑸ | 拡大率が 1 でない画面で ⑴ と ⑷ が原寸で返る | ⑴ の側は **合**（拡大率 200% の画面で 382×547＝原寸。下記）。⑷ の側は未実施 |
| ⑹ | 無いスコープと無い surface ID が要件 4.1・4.2 の文言で返る | 未実施 |
| ⑺ | 撮っている間もゴーストの描画と会話が止まらず、ERROR の記録が増えない | 未実施（走行 1・2 の ERROR は 0 件） |
| 所要時間 | 配布用のビルド・キャラクター 1 枚で、1 回の `handle` が 16 ms 以内 | **合**（走行 3・`b222af2e`。成功の記録の `ui_us` が 31 回で最小 91 µs・中央 123 µs・最大 310 µs。下記）。符号化を逃がす前の走行 1・2（`de80f039`）は線を越えた見込みで、タスク 4.3 を起こした |

## ⑴ 走行 1（pid 31276）

起動して台詞が出終わった後、画面を撮ってから `dump_surface` を引数なしで呼んだ。

```
本文: OK:scope 0, surface 1000 as currently shown (with running animations and dressups, before scaling and transparency)
画像: image/png・base64 の長さ 162972・isError=false
復号した PNG: 382×547・Format32bppArgb・左上の角のアルファ 0（透過）
```

返った PNG は、画面の右のキャラクター（目を閉じて胸の前で手を組んだ姿）と同じ姿だった。画面は拡大率 200% で、PNG の大きさは surface の原寸（382×547）だった。

成功の記録（`debug!`）が 1 件出た:

```
DEBUG actor{actor=emo-text}: areka::mcp::dump_surface: [mcp] 絵を返す tool="dump_surface" scope=0 surface_id=1000 width=382 height=547 base64_len=162972
```

ERROR の段の行は 0 件（標準出力・標準エラー出力の両方）。

## 所要時間（走行 1・2）

`handle` の入口には記録が無いので、次の 2 通りで見た。

1. **要求が届いてから絵を返した記録が出るまでの壁時計**（MCP のスレッドが要求を受けた記録から、表示のスレッドの「絵を返す」の記録まで）。この区間には、MCP のスレッドから表示のスレッドへ渡して次のフレームを待つ時間も入るので、`handle` の所要時間の**上限**として読む。

   | 走行 | 回数 | 最小 | 10% 点 | 中央 | 90% 点 | 最大 |
   |---|---|---|---|---|---|---|
   | 1 | 21 | 19.3 ms | — | 25.9 ms | — | 33.3 ms |
   | 2 | 200 | 17.6 ms | 21.7 ms | 27.1 ms | 32.6 ms | 371.3 ms |

2. **表示のスレッド（`main`・role=ui）の CPU 時間の増え方**（走行 2・`RUST_LOG` に `areka::perf=debug`・`AREKA_PERF_THREAD_REPORT_SEC=10`）。200 回の呼び出し（04:22:29〜04:23:23）を挟む 60 秒（報告の 3 番目から 9 番目）で、`main` の CPU 時間は 10,609 ms 増えた。呼び出しの無い後の 120 秒は 10 秒あたり 594〜938 ms（平均およそ 750 ms）だったので、60 秒ぶんの平常はおよそ 4,500 ms。差のおよそ 6,100 ms を 200 回で割ると、**1 回あたりおよそ 30 ms**。ただし、この差には呼び出しで余計に回ったフレームの描画も入りうる。また、CPU 時間の目盛りは 15.6 ms と粗い。

どちらの見方でも、最小の回を含めて 16 ms を下回った回は無かった。design の合否の線（Performance の節）を越えた見込みが高い。

## 止めた理由と、次の手

タスク 6.2 の決まり（「所要時間が 16 ms を超えたら、そこで止めて開発者へ報告し、符号化を別のスレッドへ逃がす変更を追加のタスクとして起こす」）に従い、⑵ 以降は確かめていない。

## 走行 3（`b222af2e`・pid 30436・展開先 `target\signoff-dump\b\`）

符号化を別のスレッドへ逃がした版（タスク 4.3）で、⑴ と所要時間をやり直した。起動のしかたは走行 1 と同じ（`RUST_LOG=info,areka::mcp=debug,areka_mcp=debug`）。

### ⑴

台詞が出終わった後に画面を撮ってから、`dump_surface` を引数なしで呼んだ。

```
本文: OK:scope 0, surface 1000 as currently shown (with running animations and dressups, before scaling and transparency)
画像: image/png・base64 の長さ 162972・isError=false
復号した PNG: 382×547・左上の角のアルファ 0（透過）
```

返った PNG は、画面の右のキャラクター（目を閉じて胸の前で手を組んだ姿）と同じ姿だった。走行 1（`de80f039`）で返った PNG とバイト列まで一致した（SHA256 の頭 16 字 `189FB6158126CD52`）。拡大率 200% の画面で、大きさは原寸の 382×547（⑸ のキャラクターの側）。

### 所要時間

成功の `debug!` に足した欄で見た。`ui_us` は UI スレッドの側（`handle` に入ってから別のスレッドへ渡すまで）、`encode_us` は別のスレッドの側（乗算を戻す・PNG・base64）。

```
DEBUG areka::mcp::dump_surface: [mcp] 絵を返す tool="dump_surface" scope=0 surface_id=1000 width=382 height=547 base64_len=162972 ui_us=212 encode_us=17631
```

| 欄 | 回数 | 最小 | 中央 | 最大 |
|---|---|---|---|---|
| `ui_us`（UI スレッド） | 31 | 91 µs | 123 µs | 310 µs |
| `encode_us`（別のスレッド） | 31 | 12,048 µs | 15,709 µs | 20,534 µs |

UI スレッドの側は最大でも 0.31 ms で、合否の線（16 ms）を大きく下回った。符号化そのものは中央 15.7 ms かかっており、走行 1・2 ではこれが丸ごと UI スレッドに乗っていた。ERROR の段の行は 0 件（標準出力・標準エラー出力の両方）。

### ⑵（代わりの手順）

要件 7.7 ⑵ は「`sakurascript` で表情を変えた後に撮る」だが、areka の `sakurascript` ツールはまだ中身が無く `NG:not implemented yet` を返す（`crates/areka/src/mcp/sakurascript.rs`・別の spec の受け持ち）。開発者の了承を得て、画面の右のキャラクターの体をダブルクリックし、ゴーストに表情を変えてもらう手順に替えた。

ダブルクリックの 2 秒後に画面を撮った。右のキャラクターは「ダブルクリックしたね？ メニューやで。」と話し、目を開いて青いリボンで手を下ろした姿に変わっていた。その直後に `dump_surface` を引数なしで呼んだ。

```
本文: OK:scope 0, surface 1000 as currently shown (with running animations and dressups, before scaling and transparency)
画像: image/png・base64 の長さ 180616（⑴ の 162972 から変わった）・isError=false
DEBUG areka::mcp::dump_surface: [mcp] 絵を返す tool="dump_surface" scope=0 surface_id=1000 width=382 height=547 base64_len=180616 ui_us=247 encode_us=15794
```

返った PNG は、変わった後の画面と同じ姿（目を開き、青いリボン、手を下ろした姿）だった。surface の ID は 1000 のまま。emo2 は同じ surface の中のアニメーションと着せ替えで表情を変えており、表示の層の記録にも `ShowSurface` の `surface_id=1000` しか出ていない。本文が言う「動いているアニメーションと着せ替えを含む今の見た目」が写っている。
