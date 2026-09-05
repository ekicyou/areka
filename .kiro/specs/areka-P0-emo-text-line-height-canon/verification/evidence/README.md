# 根拠画像と読み取り値（Requirement 1.3・2026-09-05）

> 開発者方針「SSP 実測主義は取らない。意味論は輸入する」（2026-09-05）に従い、SSP の画素実測は行わない。本ディレクトリは裁定の根拠として用いた表示画像 2 枚と、その目視読み取り値を保存する場所である。

## 画像

| ファイル | 内容 | 状態 |
|---|---|---|
| `ssp-emo2-200pct-2026-09-05.png` | SSP 2.8.83・ghost `emo`・balloon `emo2-kakukaku`・表示 200%（192 DPI の面）・本体側「昼間っから呼んでくれるん？／嬉しいわぁ！」・相方側「暇なだけだと／思うよ。」 | **未着**——開発者がチャットへ貼った画像。同名で本ディレクトリへ保存すること |
| `areka-emo2-200pct-2026-09-05.png` | areka（本ブランチ HEAD `36d1c323` 時点の `target\debug\areka.exe`・2026-09-05 15:49 ビルド）・同 fixture・192 DPI の面・本体側「こんばんはー！夜やけど／元気やでー！」・相方側「その元気、どこから／湧くの。」 | **未着**——同上 |

等倍の確認: どちらの画像もバルーン画像（本体側 400 image px 幅）が画面上 ≈ 800px で写っており、物理 px 等倍（k = 2）である。

## 読み取り値（目視・±5 物理 px）

| 量 | SSP | areka（改訂前） | image px 換算（÷2） |
|---|---|---|---|
| 1 文字の送り（本体側の 1 行の幅 ÷ 文字数） | 「昼間っから呼んでくれるん？」13 文字 ≈ 585px → ≈ 45／字 | 「こんばんはー！夜やけど」11 文字 ≈ 505px → ≈ 46／字 | ≈ 22.5〜23（Yu Gothic UI は仮名の送りが em より狭い） |
| 字のインクの丈 | 「昼」≈ 45px | 「夜」≈ 44px | ≈ 22 |
| 行送り（1 行目→2 行目の字の上端） | ≈ 58〜60px | ≈ 72px | SSP **29〜30**・areka **36**（実装値 35） |
| 相方側 1 文字の送り | 「暇なだけだと」6 文字 ≈ 275px → ≈ 46／字 | 「その元気、どこから」9 文字 ≈ 400px → ≈ 44／字 | ≈ 22〜23 |

## 読み取りから導いた裁定（Requirement 1.1／1.2）

- 字の大きさ（送り・インク丈）は SSP と areka で一致 → `font.height` は **em**（areka の現行解釈のまま）。候補 α（セル丈＝em を 28 ÷ 1.33 へ縮める）は不採用。
- 行送りだけが 30 対 35 で異なる → 行送り ＝ `font.height + 行間`・行間の既定は **定数 2 image px**（里々 wiki「既定フォント 12 で 1 行 14px」とも一致）。
- インク丈 ≈ 22 image px < 行送り 30 なので隣接行のインクは重ならない。brief の「係数 1.0 はインクが重なる」は行ボックス（37.24）とインクの取り違えであった。

## 参照

- ukadoc `font.height`: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#font.height_2c_6570_5024:1
- ukadoc `\_l[x,y]`（`XXem`／`XXlh`「1lh＝1em＋行間」）: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_l_5bx_2cy_5d:1
- ukadoc `\f[height,数値]`（「スタイルシートのサイズ指定も可能」）: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5cf_5bheight_2c_6570_5024_5d:1
- 里々 wiki「選択肢 › 2 段組メニュー」（「戻りたい行数×14」）: https://soliton.sub.jp/satori/?%E9%81%B8%E6%8A%9E%E8%82%A2#dc10653c
