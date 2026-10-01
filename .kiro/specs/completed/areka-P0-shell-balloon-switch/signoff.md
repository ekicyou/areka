# 実機確認の記録（areka-P0-shell-balloon-switch・要件 4.4・4.5・9.2・9.3・11.11・11.12）

要件 11.11 の ①〜⑦ を、配布物の形の実行体で 6 回に分けて走らせた。全項目が合格した。操作と目視は開発者が行い、記録の数字は各回の `run<N>.log` から拾った。

## 共通

- 日時: 2026-10-01 16:25〜16:36 JST（記録は UTC の 07:25〜07:36）
- 環境: Windows 11 Pro 10.0.26200・画面の拡大率 200%（窓の DPI は 192 と記録された＝⑥ の条件）
- 実行体: `pwsh -NoProfile -File tools/package-alpha.ps1` で HEAD `1da4808` から組んだ配布物（全段 緑・`areka-alpha-x64-20261001-1da4808.zip`）を、ワークツリーの `target\sbs\root1` へ展開した。根には `ghost\emo2`・`balloon\emo2-kakukaku`・`balloon\StayseeBalloon` と 32bit の補助プロセス `shiori-host32-helper.exe` が在る。`tools/test-all.ps1` は `ae635eef` で全段 緑（その後の `1da4808` は tasks.md だけの変更）
- 環境変数: `NO_COLOR=1`・`RUST_LOG=info,areka=debug,areka_kanade=debug,kanade=debug`（切替の受理・切れ目の見極め・差し替え・中止・記憶の行が出る水準）・`AREKA_APP_SMOKE_EXIT_MS=1200000`（有界の自動終了・安全弁。どの回も開発者がメニューの「終了」で先に閉じた）
- 起動（1〜5 回目）: `Start-Process <根>\areka.exe -ArgumentList "<R_POST のゴーストの絶対パス>" -WorkingDirectory <根>`（標準出力を `<根>\run<N>.log`・標準エラーを `run<N>.err.log` へ）。6 回目は引数なし
- 終了: 6 回とも開発者がメニューの「終了」を選んだ（`app_exit origin=KanadeStopped(Quit)`）

## 2 つ目のシェルの作り方（要件 9.2・9.3）

1. `cargo run -q -p sample-ghost-kit --bin nar-sample-path -- R_POST_and_KOMAINU` で `target\nar-samples\manual\R_POST_and_KOMAINU\` へ展開した
2. その下の `ghost\R_POST_and_KOMAINU\shell\master\` を丸ごと `shell\second\` へ写した
3. 写した `descript.txt` の `name,master` を `name,second` へ置き換えた（バイト単位。`charset,Shift_JIS` と CRLF は保った＝差は 6 バイトだけ）
4. 写した側だけ `surface0000.png` と `surface0001.png` を入れ替えた。ただの写しでは絵が同じで「替わった」ことが目で分からないため（立ち絵の表情が変わる）
5. ③ を確かめるため、`ghost\master\dic02_Event.txt` の `OnShellChanging` の 1 行目の末尾に `\_w[5000]`（ASCII の 9 バイト）を足した。元の台詞は約 0.6 秒で終わり、ダブルクリックが間に合わなかった（1 回目の記録: 要求から切れ目まで 0.64 秒）

どれも `target` の下の展開物で、リポジトリの検体（`.nar`）と `konnoyayame` には触れていない。

## 結果

| 回 | 見たもの | 要求 | 完了 | 中止 | 失敗 | 記憶の書き込み | 切替の後の台詞 | swap_ms |
|---|---|---|---|---|---|---|---|---|
| 1 | ①② シェルの往復×4・④ バルーン 1 回 | 9 | 9 | 0 | 0 | シェル 8・バルーン 1 | `OnShellChanged` 8 | 4.7〜7.3 |
| 2 | ③ 中断×3・バルーン 1 回 | 4 | 1 | 3 | 0 | シェル 0・バルーン 1 | なし | 18.6（バルーン） |
| 3 | ④ バルーンの往復 | 3 | 3 | 0 | 0 | シェル 0・バルーン 3 | なし（R_POST は `OnBalloonChange` を持たない） | 7.3〜9.4 |
| 4 | ① シェル `second`（中断せず） | 1 | 1 | 0 | 0 | シェル 1 | `OnShellChanged` 1 | 5.4 |
| 5 | ⑤ 起動時の記憶 | 0 | 0 | 0 | 0 | — | — | — |
| 6 | ⑦ emo2 のバルーンの往復 | 2 | 2 | 0 | 0 | バルーン 2 | `OnBalloonChange` 2 | 5.2〜5.9 |

- ① 「シェル」枠から `second` を選ぶと、`OnShellChanging` の台詞（`dic02_Event.txt`）が最後まで流れてから絵が替わり、`OnShellChanged` の台詞が出た（`talk_gap_reached marked=Some(Completed)` → `skin_switch_done`）。開発者が目視で確認
- ② `master` へ戻れた（1 回目に往復 4 周）
- ③ `OnShellChanging` の台詞の途中のダブルクリックで、3 回とも `talk_gap_marked_break` → `skin_switch_cancelled`。差し替え・`OnShellChanged`・記憶の書き込みは 0、終了の指示も無く、元の絵のまま。開発者が目視で確認
- ④ `emo2-kakukaku` ⇄ `StayseeBalloon` の切替が通り、次の台詞が新しいバルーンで出た（開発者が目視で確認）。バルーンの切替には「切り替える前」のイベントが無いので、台詞を待たずに差し替わる。R_POST の辞書には `OnBalloonChange` が無いので、切替の直後は喋らない（仕様どおり）
- ⑤ 4 回目に `second` へ替えてから終了し、5 回目は `last_used_recorded shell="second" balloon="emo2-kakukaku"`、`balloon_resolved route=Memory` で起きた。`boot_shell_missing` は 0。開発者が `second` の表情で起きたことを目視で確認。バルーンの記憶は 2〜4 回目の起動でも記憶どおりだった
- ⑥ 拡大率 200% で、6 回を通して絵・当たり判定・窓の大きさの崩れは無く、崩れたフレームも見えなかった（開発者の目視）
- ⑦ emo2（根の唯一のゴースト・同梱バルーン `emo2-kakukaku` で起動）で「バルーン」枠から `StayseeBalloon`、続けて `emo2-kakukaku` を選ぶと、どちらも差し替えの直後に `update.pasta` の `OnBalloonChange` の台詞（「バルーン切り替えです。」「ばるるるるん？」）が新しいバルーンで出た（開発者が目視で確認）

## 差し替えが UI スレッドを止めた時間（要件 4.4）

`swap_ms`（drain の時間＋後始末の時間の上限の値）は 16 回の差し替えのうち 15 回が 4.7〜9.4 ms で、1 フレーム（約 16 ms）の内側だった。2 回目のバルーンの差し替え 1 回だけ 18.6 ms で、1 フレームを少し超えた（目視では崩れは見えなかった）。要件 4.4 は 1 フレームの内側を「目標」とし、決定論テストでは固定しない。測った値をここに残す。

## 既知の制限・観察（判定は変えない）

- **⑦ の台本の経路は未確認**: emo2 の辞書には `\![change,balloon,…]` を出す台本が無い（展開した `ghost\emo2` の下を検索して 0 件）。実機ではメニューからの切替と `OnBalloonChange` の応答だけを確かめた。台本から替わる経路は決定論テスト（`shell_balloon_switch_session_balloon_tests.rs` のバルーンの往復）で固定している
- 1 回目に `ERROR` 4 件・3 回目に 2 件: どれも R_POST の辞書がダブルクリックの台詞で `\s[7]`・`\s[11]` を呼び、シェルにその面が無い（シェルの面は 0〜6・10・100・200）ための合成の失敗（`SurfaceNotFound`）。`master` でも出ており、検体の中身の問題で本仕様の範囲外。表示は適用前のまま続いた
- `WARN` の 3 種（6 回で計 50 件）はどれも本仕様の範囲外の既存のもの: バルーンの面が既定の系列へ縮退した（40 件）、折り返しの基準が描画範囲の外（8 件）、全透明の部品（2 件）
- メッセージボックス: 0
