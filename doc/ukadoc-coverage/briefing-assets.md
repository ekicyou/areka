# 資産ドメイン調査 ブリーフィング

`doc/ukadoc-coverage/ledger/assets.toml` に載せた判定を、台帳の欄だけでは書ききれない粒度で説明するための文書である。

この調査を支える上流の道具は着地しており、本調査はその道具の上で進めている。同じ台帳から作るドメイン別の報告 `doc/ukadoc-coverage/report/assets.md` は**人手で書かない**——台帳から機械で作り直すものであり、台帳を直したときは同じコミットで作り直して入れる。手で書くのはこの文書と台帳だけである。基準になる 2 つの時点（正典のスナップショットが作られた時点と、ライブを確かめた日）は、この文書の「SERIKO/MAYUNA 世代別対応表」と「ライブ確認の結果」の冒頭に同じ値で書いてあるので、ここでは繰り返さない。

## この文書の構成

節はこの順に置く——冒頭、SERIKO/MAYUNA 世代別対応表、未知の記述の扱い、nar インストールとネットワーク更新の導線、沈黙ルール対応表の一覧、ライブ確認の結果、未収載の候補、隣接 spec の是正候補。本稿はこの 8 節すべてを備えている。

## SERIKO/MAYUNA 世代別対応表

シェルの見た目と動きを決める `surfaces.txt` のページ（`descript_shell_surfaces`）の 137 項目を、1 行ずつ並べた表である。列は「項目 id・見出し・登場した版・areka の状態」の 4 つ。

**この表は手で書いていない。** 作業用のスクリプトが台帳 `doc/ukadoc-coverage/ledger/assets.toml` を読み、見出しを `doc/ukadoc-coverage/catalog.toml` から引いて組んだものを、そのまま貼っている。貼ったものと作り直したものが 1 バイトも違わないこと、台帳のこのページの項目が過不足なく 1 行ずつ載っていること（同じ項目が 2 行に分かれていないこと）は、作業用の検査が毎回見ている。台帳の側を直したら、表も作り直して貼り直すことになる。

### 基準になる 2 つの時点

- 正典のスナップショットが作られた時点: **2026-08-24T04:08:57.881Z**（`doc/ukadoc-coverage/catalog.toml` の `[snapshot]` の `generated_at`）。「登場した版」の欄と「見出し」の欄はこの時点のものである。
- ライブを確かめた日: **2026-09-05**。このページの見出しはその日のライブでも 137 件で、スナップショットと数が合っていた（下の「ライブ確認の結果」の節）。

つまりこの表は、正典についてはスナップショットの時点を、ライブとの照合についてはその 12 日後の 1 日を写している。それより後に正典が動いていれば、その分だけ古い。areka の側の状態は本調査が調べた時点のものである。

**本節の行番号は 2026-09-06 に測ったものである。** 設計と要件が引いている範囲とは端が数行ずれるが、指している定義は同じである。行番号はソースに 1 行足すだけで動くので、以下ではクレート名・ファイル・定義の名前を併せて書いた。

### 表の読み方

「登場した版」は、その項目が正典で最初に現れた版である。カタログがその項目に版を 1 つも記録していないときは `—` と書いた——**66 項目**がこれに当たる。版が記録されているのは **71 項目**で、最も古いものが 2.3.53、最も新しいものが 2.8.52 である。カタログが 2 つ以上の版を記録している項目はこのページに 2 つあり（`element*` と `animation*.pattern*`）、どちらも最も古い版を採った。この選び方は台帳の冒頭に書いてある規則そのままである。

「areka の状態」は台帳の状態をそのまま写したもので、語の意味は報告 `doc/ukadoc-coverage/report/assets.md` と同じである。このページの内訳は 実装済み 4・語彙のみ 57・縮退 4・別名 4・未対応 68 の合わせて 137 件で、**対象外は 0 件、未分類も 0 件**である。実装済みの 4 件は `overlay`・`add`・`random,数値`・`animation-sort,ソート順序` で、この 4 つだけがソース側に正典 URL の 1 行を持っている。

| 項目 id | 見出し | 登場した版 | areka の状態 |
| --- | --- | --- | --- |
| `ukadoc:descript_shell_surfaces:_2a_2c_5b_2a_5d:1` | `*,[*]` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:_5f53_305f_308a_5224_5b9a_540d_2c_8868_793a_5185_5bb9:1` | `当たり判定名,表示内容` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:add:1` | `add` | — | 実装済み |
| `ukadoc:descript_shell_surfaces:alternativestart_2c_28ID1_2cID2..._29:1` | `alternativestart,(ID1,ID2...)` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:alternativestop_2c_28ID1_2cID2..._29:1` | `alternativestop,(ID1,ID2...)` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:always:1` | `always` | — | 語彙のみ |
| `ukadoc:descript_shell_surfaces:animation-sort_2c_30bd_30fc_30c8_9806_5e8f:1` | `animation-sort,ソート順序` | — | 実装済み |
| `ukadoc:descript_shell_surfaces:animation_2a.collision_2a_2c_5f53_305f_308a_5224_5b9a_5b9a_7fa9animation_2a.collisionex_2a_2c_5f53_305f_308a_5224_5b9a_5:1` | `animation*.collision*,当たり判定定義animation*.collisionex*,当たり判定定義(ex)` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:animation_2a.interval_2c_30a4_30f3_30bf_30fc_30d0_30eb:1` | `animation*.interval,インターバル` | — | 縮退 |
| `ukadoc:descript_shell_surfaces:animation_2a.name_2c_5b9a_7fa9_540d:1` | `animation*.name,定義名` | `2.8.24` | 未対応 |
| `ukadoc:descript_shell_surfaces:animation_2a.option_2c_30aa_30d7_30b7_30e7_30f3:1` | `animation*.option,オプション` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:animation_2a.option_2cbackground:1` | `animation*.option,background` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:animation_2a.option_2cexclusive:1` | `animation*.option,exclusive` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:animation_2a.option_2cshared-index:1` | `animation*.option,shared-index` | `2.6.02` | 未対応 |
| `ukadoc:descript_shell_surfaces:animation_2a.pattern_2a_2c_63cf_753b_30e1_30bd_30c3_30c9_2c_30b5_30fc_30d5_30a7_30b9_756a_53f7_2c_30a6_30a7_30a4_30c8_2c:1` | `animation*.pattern*,描画メソッド,サーフェス番号,ウェイト,X座標,Y座標(,オプション...)` | `2.8.25` | 縮退 |
| `ukadoc:descript_shell_surfaces:asis:1` | `asis` | — | 語彙のみ |
| `ukadoc:descript_shell_surfaces:auto:1` | `auto` | `2.8.41` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:balloon.offsetx_2c_5ea7_6a19:1` | `balloon.offsetx,座標` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:balloon.offsety_2c_5ea7_6a19:1` | `balloon.offsety,座標` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:base:1` | `base` | — | 語彙のみ |
| `ukadoc:descript_shell_surfaces:bind:1` | `bind` | — | 語彙のみ |
| `ukadoc:descript_shell_surfaces:bind:2` | `bind` | — | 別名 |
| `ukadoc:descript_shell_surfaces:blend-add-fast:1` | `blend-add-fast` | `2.8.36` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-add-glow-fast:1` | `blend-add-glow-fast` | `2.8.46` | 未対応 |
| `ukadoc:descript_shell_surfaces:blend-add-glow:1` | `blend-add-glow` | `2.8.46` | 未対応 |
| `ukadoc:descript_shell_surfaces:blend-add:1` | `blend-add` | `2.8.36` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-color-burn-fast:1` | `blend-color-burn-fast` | `2.8.40` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-color-burn:1` | `blend-color-burn` | `2.8.40` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-color-dodge-fast:1` | `blend-color-dodge-fast` | `2.8.39` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-color-dodge-glow-fast:1` | `blend-color-dodge-glow-fast` | `2.8.46` | 未対応 |
| `ukadoc:descript_shell_surfaces:blend-color-dodge-glow:1` | `blend-color-dodge-glow` | `2.8.46` | 未対応 |
| `ukadoc:descript_shell_surfaces:blend-color-dodge:1` | `blend-color-dodge` | `2.8.39` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-color-fast:1` | `blend-color-fast` | `2.8.39` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-color:1` | `blend-color` | `2.8.39` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-darken-fast:1` | `blend-darken-fast` | `2.8.40` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-darken:1` | `blend-darken` | `2.8.40` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-darker-color-fast:1` | `blend-darker-color-fast` | `2.8.40` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-darker-color:1` | `blend-darker-color` | `2.8.40` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-difference-fast:1` | `blend-difference-fast` | `2.8.40` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-difference:1` | `blend-difference` | `2.8.40` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-dither:1` | `blend-dither` | `2.8.44` | 未対応 |
| `ukadoc:descript_shell_surfaces:blend-divide-fast:1` | `blend-divide-fast` | `2.8.40` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-divide:1` | `blend-divide` | `2.8.40` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-exclusion-fast:1` | `blend-exclusion-fast` | `2.8.40` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-exclusion:1` | `blend-exclusion` | `2.8.40` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-hard-light-fast:1` | `blend-hard-light-fast` | `2.8.39` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-hard-light:1` | `blend-hard-light` | `2.8.39` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-hard-mix-fast:1` | `blend-hard-mix-fast` | `2.8.40` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-hard-mix:1` | `blend-hard-mix` | `2.8.40` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-hue-fast:1` | `blend-hue-fast` | `2.8.39` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-hue:1` | `blend-hue` | `2.8.39` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-lighten-fast:1` | `blend-lighten-fast` | `2.8.40` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-lighten:1` | `blend-lighten` | `2.8.40` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-lighter-color-fast:1` | `blend-lighter-color-fast` | `2.8.40` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-lighter-color:1` | `blend-lighter-color` | `2.8.40` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-linear-burn-fast:1` | `blend-linear-burn-fast` | `2.8.40` | 未対応 |
| `ukadoc:descript_shell_surfaces:blend-linear-burn:1` | `blend-linear-burn` | `2.8.40` | 未対応 |
| `ukadoc:descript_shell_surfaces:blend-linear-light-fast:1` | `blend-linear-light-fast` | `2.8.40` | 未対応 |
| `ukadoc:descript_shell_surfaces:blend-linear-light:1` | `blend-linear-light` | `2.8.40` | 未対応 |
| `ukadoc:descript_shell_surfaces:blend-luminosity-fast:1` | `blend-luminosity-fast` | `2.8.39` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-luminosity:1` | `blend-luminosity` | `2.8.39` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-multiply-fast:1` | `blend-multiply-fast` | `2.8.36` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-multiply:1` | `blend-multiply` | `2.8.36` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-overlay-fast:1` | `blend-overlay-fast` | `2.8.36` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-overlay:1` | `blend-overlay` | `2.8.36` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-pin-light-fast:1` | `blend-pin-light-fast` | `2.8.40` | 未対応 |
| `ukadoc:descript_shell_surfaces:blend-pin-light:1` | `blend-pin-light` | `2.8.40` | 未対応 |
| `ukadoc:descript_shell_surfaces:blend-saturation-fast:1` | `blend-saturation-fast` | `2.8.39` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-saturation:1` | `blend-saturation` | `2.8.39` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-screen-fast:1` | `blend-screen-fast` | `2.8.36` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-screen:1` | `blend-screen` | `2.8.36` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:blend-soft-light-fast:1` | `blend-soft-light-fast` | `2.8.39` | 未対応 |
| `ukadoc:descript_shell_surfaces:blend-soft-light:1` | `blend-soft-light` | `2.8.39` | 未対応 |
| `ukadoc:descript_shell_surfaces:blend-subtract-fast:1` | `blend-subtract-fast` | `2.8.40` | 未対応 |
| `ukadoc:descript_shell_surfaces:blend-subtract:1` | `blend-subtract` | `2.8.40` | 未対応 |
| `ukadoc:descript_shell_surfaces:blend-vivid-light-fast:1` | `blend-vivid-light-fast` | `2.8.40` | 未対応 |
| `ukadoc:descript_shell_surfaces:blend-vivid-light:1` | `blend-vivid-light` | `2.8.40` | 未対応 |
| `ukadoc:descript_shell_surfaces:charset_2c_6587_5b57_30b3_30fc_30c9:1` | `charset,文字コード` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:collision-sort_2c_30bd_30fc_30c8_9806_5e8f:1` | `collision-sort,ソート順序` | — | 語彙のみ |
| `ukadoc:descript_shell_surfaces:collision_2a_2c_59cb_70b9X_2c_59cb_70b9Y_2c_7d42_70b9X_2c_7d42_70b9Y_2cID:1` | `collision*,始点X,始点Y,終点X,終点Y,ID` | — | 縮退 |
| `ukadoc:descript_shell_surfaces:collisionex_2a_2cID_2c_30bf_30a4_30d7_2c_5ea7_6a191_2c_5ea7_6a192...:1` | `collisionex*,ID,タイプ,座標1,座標2...` | `2.5.19` | 未対応 |
| `ukadoc:descript_shell_surfaces:element_2a_2c_63cf_753b_30e1_30bd_30c3_30c9_2c_30d5_30a1_30a4_30eb_540d_2cX_5ea7_6a19_2cY_5ea7_6a19_28_2c_30aa_30d7_30b7:1` | `element*,描画メソッド,ファイル名,X座標,Y座標(,オプション...)` | `2.3.53` | 縮退 |
| `ukadoc:descript_shell_surfaces:endtalk:1` | `endtalk` | `2.7.26` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:icon.rect_2c_59cb_70b9X_2c_59cb_70b9Y_2c_7d42_70b9X_2c_7d42_70b9Y:1` | `icon.rect,始点X,始点Y,終点X,終点Y` | `2.8.52` | 未対応 |
| `ukadoc:descript_shell_surfaces:import_2c_30d5_30a1_30a4_30eb_540d_2c_30a6_30a8_30a4_30c8msec_2cX_2cY:1` | `import,ファイル名,ウエイトmsec,X,Y` | `2.7.50` | 未対応 |
| `ukadoc:descript_shell_surfaces:insert_2cID:1` | `insert,ID` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:interpolate:1` | `interpolate` | — | 語彙のみ |
| `ukadoc:descript_shell_surfaces:kero.balloon.offsetx_2c_5ea7_6a19:1` | `kero.balloon.offsetx,座標` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:kero.balloon.offsety_2c_5ea7_6a19:1` | `kero.balloon.offsety,座標` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:maxwidth_2c_30d4_30af_30bb_30eb:1` | `maxwidth,ピクセル` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:mousedown_2a_2c_5f53_305f_308a_5224_5b9aID_2c_30d5_30a1_30a4_30eb_540d:1` | `mousedown*,当たり判定ID,ファイル名` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:mousehover_2a_2c_5f53_305f_308a_5224_5b9aID_2c_30d5_30a1_30a4_30eb_540d:1` | `mousehover*,当たり判定ID,ファイル名` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:mouserightdown_2a_2c_5f53_305f_308a_5224_5b9aID_2c_30d5_30a1_30a4_30eb_540d:1` | `mouserightdown*,当たり判定ID,ファイル名` | `2.6.14` | 未対応 |
| `ukadoc:descript_shell_surfaces:mouseup_2a_2c_5f53_305f_308a_5224_5b9aID_2c_30d5_30a1_30a4_30eb_540d:1` | `mouseup*,当たり判定ID,ファイル名` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:mousewheel_2a_2c_5f53_305f_308a_5224_5b9aID_2c_30d5_30a1_30a4_30eb_540d:1` | `mousewheel*,当たり判定ID,ファイル名` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:move:1` | `move` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:name_2c_5b9a_7fa9_540d:1` | `name,定義名` | `2.8.24` | 未対応 |
| `ukadoc:descript_shell_surfaces:never:1` | `never` | — | 語彙のみ |
| `ukadoc:descript_shell_surfaces:overlay-fast:1` | `overlay-fast` | `2.8.36` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:overlay:1` | `overlay` | — | 実装済み |
| `ukadoc:descript_shell_surfaces:overlayfast:1` | `overlayfast` | — | 別名 |
| `ukadoc:descript_shell_surfaces:overlaymultiply:1` | `overlaymultiply` | `2.5.91` | 別名 |
| `ukadoc:descript_shell_surfaces:overlayscreen:1` | `overlayscreen` | `2.8.35` | 別名 |
| `ukadoc:descript_shell_surfaces:parallelstart_2c_28ID1_2cID2..._29:1` | `parallelstart,(ID1,ID2...)` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:parallelstop_2c_28ID1_2cID2..._29:1` | `parallelstop,(ID1,ID2...)` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:periodic_2c_6570_5024:1` | `periodic,数値` | — | 語彙のみ |
| `ukadoc:descript_shell_surfaces:point.basepos.x_2c_5ea7_6a19:1` | `point.basepos.x,座標` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:point.basepos.y_2c_5ea7_6a19:1` | `point.basepos.y,座標` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:point.centerx_2c_5ea7_6a19:1` | `point.centerx,座標` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:point.centery_2c_5ea7_6a19:1` | `point.centery,座標` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:point.kinoko.centerx_2c_5ea7_6a19:1` | `point.kinoko.centerx,座標` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:point.kinoko.centery_2c_5ea7_6a19:1` | `point.kinoko.centery,座標` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:random_2c_6570_5024:1` | `random,数値` | — | 実装済み |
| `ukadoc:descript_shell_surfaces:rarely:1` | `rarely` | — | 語彙のみ |
| `ukadoc:descript_shell_surfaces:reduce:1` | `reduce` | — | 語彙のみ |
| `ukadoc:descript_shell_surfaces:replace:1` | `replace` | — | 語彙のみ |
| `ukadoc:descript_shell_surfaces:runonce:1` | `runonce` | — | 語彙のみ |
| `ukadoc:descript_shell_surfaces:sakura.balloon.offsetx_2c_5ea7_6a19:1` | `sakura.balloon.offsetx,座標` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:sakura.balloon.offsety_2c_5ea7_6a19:1` | `sakura.balloon.offsety,座標` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:scaling:1` | `scaling` | `2.7.28` | 未対応 |
| `ukadoc:descript_shell_surfaces:sometimes:1` | `sometimes` | — | 語彙のみ |
| `ukadoc:descript_shell_surfaces:start_2cID:1` | `start,ID` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:starttalk:1` | `starttalk` | `2.7.26` | 語彙のみ |
| `ukadoc:descript_shell_surfaces:stop_2cID:1` | `stop,ID` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:system_3aarrow:1` | `system:arrow` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:system_3across:1` | `system:cross` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:system_3afinger:1` | `system:finger` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:system_3agrip:1` | `system:grip` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:system_3ahand:1` | `system:hand` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:system_3ahelp:1` | `system:help` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:system_3amove:1` | `system:move` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:system_3ano:1` | `system:no` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:system_3atext:1` | `system:text` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:system_3await:1` | `system:wait` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:talk_2c_6570_5024:1` | `talk,数値` | — | 語彙のみ |
| `ukadoc:descript_shell_surfaces:version_2c_2a:1` | `version,*` | — | 未対応 |
| `ukadoc:descript_shell_surfaces:yen-e:1` | `yen-e` | — | 語彙のみ |

### 表に添える 4 つの注記

**⑴ アニメーションを動かす間隔の語は 2 語だけである。** `animation*.interval` の 2 つめの欄に書く語を、areka は 2 か所で扱う。転記側は `areka-parsers` の `shell::normalize_interval`（`crates/areka-parsers/src/shell/decode.rs:385`）で、`bind`（`:387`）・`random`（`:389`）・`bind+random`（`:392`）の 3 語をそれぞれの値にし、それ以外の語（`sometimes`・`always` など）は綴りを保ったまま持ち上げる（`:396`）。駆動側は `areka-seriko` の `AnimationTable::from_world` の中の振り分け（`crates/areka-seriko/src/table.rs:105`〜`:137`）で、ここで再生を動かすものとして採るのは `random`（`:106`）と `bind+random`（`:107`〜`:109`）の **2 つだけ**である。**`bind` は駆動しない**——採らずに控えめな段の記録を 1 行残して次へ進む（`:110`〜`:117`）。それ以外の語も同じく採らない（`:118`〜`:127`）。控えめな段は既定では見えない（前掲・`crates/areka/src/main.rs:141`）ので、`interval,bind` や `interval,sometimes` と書いた宣言は、利用者から見ると何も起きずに終わる。表で `bind` と、それ以外の間隔語が「語彙のみ」になっているのはこのためである。

**⑵ `bind+random` には正典の項目が無いので、表に行を作っていない。** 駆動する 2 語の一方であるこの綴りは、カタログの ukadoc 1,749 件のどの見出しにも無い（2026-09-06 に数え直して 0 件）。正典の文書を本文まで含めて検索しても当たらない（同日・0 件）。台帳に載る項目の数（542 件）とページ（24 ページ）は正典の側で決まっているので、areka だけが持つ綴りのために新しい行を作ることはしない。代わりに、駆動するもう一方の `random,数値`（`ukadoc:descript_shell_surfaces:random_2c_6570_5024:1`）の備考に、この綴りも同じく駆動する旨を書いた。この注記がもう 1 つの置き場である。

**⑶ 絵の重ね方の語で、実際に画を作るのは `overlay` 1 つだけである。** `areka-emo-compose` の `ComposeMethod::is_implemented`（`crates/areka-emo-compose/src/method.rs:130`〜`:132`）が「実挙動がある」と答えるのは `Overlay` だけである。名前を解く `ComposeMethod::from_name`（同 `:142`）は `overlay`・`add`・`bind` の 3 つの綴りを同じ `Overlay` へ束ねる（`:150`）ので、この 3 つはどれも同じ 1 つの実装で動く。正典自身が後の 2 つを `overlay` と同じ扱いだと書いているためで、台帳もそれに合わせて `overlay` と `add` を実装済み、絵の重ね方の側の `bind` を別名としている。残りの綴りは名前としては受け取るが、そこから画は作られない——旧い書き方の 3 つ（`overlayfast`・`overlaymultiply`・`overlayscreen`）は現在の綴りを指す別名として、名前解決に当たるそれ以外の語は語彙のみとして、どれにも当たらない綴りは未対応として登記した。

**⑷ 当たり判定は矩形だけである。** `areka-parsers` の `decode_collisions`（`crates/areka-parsers/src/shell/decode.rs:229`）は、`collision` に続く部分が数字だけの行しか値にしない（`:234`〜`:236`）。値にするのは始点と終点の 4 つの数、つまり矩形 1 つ分である。円・楕円・多角形を書く `collisionex*` の行はここで**何も記録せずに読み飛ばされる**（上の surfaces.txt の節に書いたのと同じ場所である）。台帳でもそれに合わせて、`collisionex*,ID,タイプ,座標1,座標2...` を未対応、`collision*,始点X,始点Y,終点X,終点Y,ID` を縮退（矩形の分だけ通る）としている。**この縮小は本調査が決めたものではない。** 出所は `doc/emo2-conformance-scope.md:82` で、`areka-P0-seriko-runtime` の範囲を「ukadoc 完全マップ」から「SERIKO/2.0＋MAYUNA bind・overlay のみ・interval 3 種・矩形 collision」へ縮めたと書いてある行である。表のうち当たり判定を名前に持つ項目は 4 つで、内訳は `collision*,始点X,始点Y,終点X,終点Y,ID` が縮退、`collisionex*,ID,タイプ,座標1,座標2...` が未対応、`animation*.collision*`／`animation*.collisionex*` の当たり判定定義が未対応、`collision-sort,ソート順序` が語彙のみである。

### 見出しが `bind` で重なる 2 項目

このページには見出しが `bind` の項目が 2 つある。**表では id で区別して別々の行に載せてあり、1 行にまとめていない。** 名前が同じでも別の機能なので、まとめると片方の状態が消える。

- `ukadoc:descript_shell_surfaces:bind:1` — アニメーションを動かす間隔の語のほう。状態は語彙のみ（注記 ⑴）。
- `ukadoc:descript_shell_surfaces:bind:2` — 絵の重ね方の語のほう。状態は別名で、`overlay` と同じ扱い（注記 ⑶）。

名前で引くと取り違えるので、表でも台帳でも id で引く。同じ理由から、ソース側に置く正典 URL も id ごとに書いている。

## 未知の記述の扱い

定義ファイルに書いてあるのに areka がどこでも引き当てない記述——ここではそれを「未知の記述」と呼ぶ——が、いま何をされているのかを、定義ファイルの種別ごとに 1 節ずつ書く。扱いは「黙って捨てる」「記録を残す」「エラーにする」の 3 つのいずれか 1 つに決める。3 つより細かい分類は作らない。記録の段（`warn!` などの水準）は分類そのものではないので、各節の「記録」の段落に添える。

**この節の行番号は 2026-09-06 に測ったものである。** 行番号はソースに 1 行足すだけで動くので、どの節でもクレート名・ファイル・その中の定義の名前を併せて書いた。行番号が合わなくなったら定義の名前で引き直せる。台帳の備考のほうは流儀として行番号を書かない（動かない指し方だけで同じ判断を書く）ので、行番号が出てくるのはこのブリーフィングだけである。

この節を書くために areka の振る舞いは 1 つも変えていない。記録を増やす変更も、分類を足す変更もしていない。ここに書いてあるのは、いま動いているものを読んで測った結果だけである。

### 記録が残る 3 つの経路と、それ以外の無言

定義ファイルの転記を引き受けるクレート `areka-parsers` の中で、記録を残す場所は**ちょうど 3 つ**である。クレートのソース全体を当たった結果で、これがすべてである。

| 段 | 場所 | 何を記録するか |
|---|---|---|
| 警告段 `warn!` | `crates/areka-parsers/src/package/resolve.rs:306`（`parse_bindgroup_name`） | 着せ替えの名前宣言にパーツ名が無い行を、名前をこしらえずに捨てたこと |
| 控えめな段 `debug!` | `crates/areka-parsers/src/charset/decode.rs:35`（`decode`） | 宣言された文字コードの綴りを解けず、呼ぶ側の決めた既定へ落としたこと |
| 控えめな段 `debug!` | `crates/areka-parsers/src/charset/decode.rs:52`（`decode`） | 読めないバイト列を代わりの文字で吸収したこと |

**エラー段（`error!`）は 0 件である。** 情報段（`info!`）も細かい段（`trace!`）も 0 件で、上の 3 つ以外に記録を残す場所は 1 つも無い。行を名前と値へ割るところ（`crates/areka-parsers/src/kv/parse.rs`）にも、surfaces.txt の読み取り（同 `shell/decode.rs`）にも、バルーンの定義の読み取り（同 `balloon/parse.rs`）にも、記録は 1 行も無い。**未知の記述はこれらの経路を無言で通り抜ける。**

要件と設計は警告段の場所を `resolve.rs:296` と書いているが、そこは記録のしかたを説明した文の行で、記録そのものは `:306` にある。段ごとの数（警告段 1・控えめな段 2・エラー段 0）は測り直しても変わらない。

**控えめな段は既定では見えない。** areka が既定とする記録の水準は `info` である（`crates/areka/src/main.rs:141`。環境変数 `RUST_LOG` が未設定・読めない・書式が壊れているときはここへ落ちる）。`debug!` はこの水準より下なので、利用者が何もしなければ画面にもファイルにも出ない。台帳の項目のうち、記録がこの段だけのものは、分類としては事実どおり「記録を残す」に当たるが、利用者から見えるものは何も無い。だから壊れ方の判定は「黙って壊れる」にしてある（要件 3.5a）。下の 9 節の分類はこれとは別で、その種別の未知の記述そのものが何をされるかで決めている。

**「3 つ」は `areka-parsers` に限った数である。** 読み取った結果を使う下流には、別の記録がある。下流の記録をすべて数えることは本稿の範囲を超えるので、**数える範囲をここで 2 つに切って書く**。⒜ は完全な一覧、⒝ は代表例であって全部ではない。どちらの記録も `areka-parsers` の外にある。

**⒜ ⑴〜⑷ の定義ファイルが読めなかったときの記録（完全な一覧）。** 本番のコードがこの 4 種を開く場所は**ちょうど 10 か所**で、そのうち何も残さず通り抜けるのは **1 か所**だけである。残る 9 か所はいずれも警告段以上の記録へ行き着く。つまり既定の記録の水準で見える。

| 開く場所 | どのファイル | 読めなかったときに残るもの |
|---|---|---|
| `crates/areka-parsers/src/package/resolve.rs:50`（`resolve`） | ⑴ ゴーストの descript | ここでは記録せず、失敗の理由を呼ぶ側へ返す。呼ぶ側は 4 つあり、どれも記録する——`crates/areka-ghost/src/runtime.rs:484`（エラー段）・`crates/areka/src/placement/source.rs:141`（エラー段）・`crates/areka/src/placement/persist.rs:267`（警告段）・`crates/areka/src/emo2_boot/mod.rs:213`（ファイルが無いとき・警告段）と同 `:220`（それ以外・エラー段） |
| `crates/areka-parsers/src/package/resolve.rs:157`（`read_bindgroup_defaults`） | ⑵ シェルの descript | **何も残らない。** 着せ替えの既定を拾うためだけの読みで、読めなければ「着せ替えの宣言が無い」と同じ空として続き、呼ぶ側にも伝わらない。**10 か所のうち無言なのはここだけである。** |
| `crates/areka-ghost/src/config.rs:44`（`resolve_shell_name`） | ⑵ シェルの descript | `:47` の警告段。読めても `name` が無いときは `:61` の警告段。どちらもシェルのフォルダ名で代用して続く |
| `crates/areka/src/placement/source.rs:147`（`read_kv_lenient` を通る） | ⑴ ゴーストの descript | `:183` の警告段。空の表として続く |
| `crates/areka/src/placement/source.rs:151`（`load_descript_source`） | ⑵ シェルの descript | `:154` のエラー段。窓の配置はここで打ち切る |
| `crates/areka/src/placement/source.rs:203`（`load_balloon_author_dpi`。同じ `read_kv_lenient` を通る） | ⑷ バルーンの descript | `:183` の警告段。空の表として続く |
| `crates/areka/src/placement/measure.rs:333`（`build_shell_assets`） | ⑶ surfaces.txt | `:334` のエラー段。読めたのに面を 1 つも産まないときは `:347` のエラー段 |
| `crates/areka/src/emo2_boot/assets.rs:279`（`build_boot_assets`） | ⑶ surfaces.txt | その場では記録せず、`crates/areka/src/emo2_boot/mod.rs:220` のエラー段でまとめて記録する（面を 1 つも産まないときも同じ） |
| `crates/areka/src/emo2_boot/assets.rs:332`（同じ関数） | ⑵ シェルの descript | 同じく `emo2_boot/mod.rs:220` のエラー段 |
| `crates/areka-emo-present/src/balloon.rs:505`（`read_descript_layer`） | ⑷ バルーンの descript | `:429` の警告段。バルーンの既定設定を空にして続く |

**⒝ 書いてある値が語彙や書式の外だったときの記録（代表例。ここに挙げたものが全部ではない）。** 転記のあと、値を受け取った側が「知らない語だ」と気づいて記録することがある。本ドメインの読み取りに近いものを挙げると次のとおりである。**下流は広く、これで尽くしたとは言えない。**

- `crates/areka-emo-compose/src/method.rs:162`（`from_name`）——重ね合わせのしかたの名前を知らなかったときの警告段の記録。
- `crates/areka-seriko/src/table.rs:111`・`:120`・`:129`（`AnimationTable::from_world`）——アニメーションを起こす間隔の語のうち、駆動しない語・知らない語・将来の値だったときの控えめな段の記録。既定の水準では見えない。
- 同 `:142`・`:168`（同じ関数）——間隔の抽選の分母が 0 のときと、コマが 1 枚も無いときの警告段の記録。どちらもそのアニメーションを採らずに続く。
- `crates/areka/src/placement/config.rs:262`（`resolve_scope`）——バルーンを左右どちらに置くかの値が語彙の外だったときの警告段の記録。
- `crates/areka/src/placement/config.rs:310`（`parse_i32`）——窓の既定位置（`defaultx`／`defaultleft`・`defaulty`／`defaulttop`）とバルーンのずらし量（`balloon.offsetx`・`balloon.offsety`）が数として読めなかったときの警告段の記録。
- `crates/areka/src/placement/resolver.rs:266` と同 `:445`——画面のどの縁に寄せるかの値が areka の受け付ける 2 語（`bottom`・`free`）の外だったときの警告段の記録。前者は語彙の外の値を受け取ったことを、後者はそれが `top`・`left`・`right` のいずれでもなく下端寄せとして解釈したことを残す。
- `crates/areka/src/placement/source.rs:230` と同 `:240`（`parse_author_dpi`）——作者が想定した画面の細かさ（`dpi`）の値が 0 だったとき・数として読めなかったときの警告段の記録。シェル側とバルーン側が同じ場所を通る。

**下の 9 節の「記録」の欄は、未知の記述に対する答えである。** 定義ファイルそのものが読めなかったときの話ではない。⒜ のとおり、⑵ シェルの descript も ⑷ バルーンの descript も、読めなければ警告段以上の記録が残って既定の水準で見える（例外は `read_bindgroup_defaults` の 1 か所だけである）。

### 定義ファイル 9 種の扱い

9 種のうち、areka に読む経路があるのは 4 種（ゴーストの descript・シェルの descript・surfaces.txt・バルーンの descript）で、読む経路がまったく無いのが 5 種（install.txt・プラグインの descript・ヘッドラインの descript・surfacetable.txt・更新ファイル）である。**9 種とも扱いは「黙って捨てる」で、「記録を残す」も「エラーにする」も 0 種である。** 読む経路が無い 5 種については、記述を 1 つずつ分けて仕訳けず、ファイル全体が黙って捨てられると書く（要件 3.8。理由は下の「読む経路が無い 5 種をファイル単位で切る理由」に書いた）。

各節の「台帳の内訳」は、その種別を定める正典のページに載っている項目の状態の内訳である。9 種の合計は 527 件で、台帳の 542 件との差 15 件は、ページ 1 枚をまとめて指す粗い粒度の項目である（こちらは定義ファイルの種別に 1 対 1 で対応しないので、この節では数えない）。

#### ⑴ ゴーストの descript（`ghost/master/descript.txt`）

- **分類**: 黙って捨てる。
- **根拠**: 行は `crates/areka-parsers/src/kv/parse.rs:20`（`parse_kv`）で最初の読点だけを使って名前と値に割られ（`:26`）、同じ名前は後に書いたほうで上書きされる（`:39`）。ここに分類も記録も無い。できた表から名前を完全一致で引くのは `crates/areka-parsers/src/package/resolve.rs` の 6 か所（`:69` の `name`・`:71` の `sakura.name`・`:73` の `sakura.name2`・`:75` の `kero.name`・`:82` の `shiori`・`:88` の `seriko.defaultsurfacedirectoryname`）と、文字コードの前走査 `crates/areka-parsers/src/charset/prescan.rs:57`（`prescan_charset`。このクレートで唯一、大文字小文字を区別しない引き方をする）である。窓の置き場所を決める側が同じ表からさらに引く（`crates/areka/src/placement/config.rs` の `resolve_scope`。並べ方 `:228`・バルーンの左右 `:262`・ずらし量 `:273` と `:278`。これらはゴースト側とシェル側の両方の表を順に見る）。**どの引き当てにも当たらない名前は、表に載ったまま誰にも見られずに終わる。**
- **記録**: なし。壊れ方は「黙って壊れる」。例外は文字コードの宣言で、綴りを解けないときだけ `charset/decode.rs:35` の控えめな段の記録が出る。既定の水準では見えないので、壊れ方の判定はやはり「黙って壊れる」である（要件 3.5a）。
- **その記述を読むのは誰か**: 転記層では止まらない。`areka-parsers` の `package::resolve` が SHIORI 本体とシェルの置き場所を決め、`areka` の窓の配置（`placement::config`）が同じ表から自分のぶんを引く。名前は対話の側まで届く。ただし届くのは上に挙げた引き当てに当たった名前だけで、それ以外は転記層より先へ進まない。
- **成立に要る基盤**: 台帳が数えた内訳では、窓の配置と重なりの解決 18 件・ゴーストとシェルの descript の転記層 17 件・資産の素性を保持する場所 13 件が大きく、ほかにマウスの矢印の差し替え 6 件・SHIORI の読み込みと寿命 6 件・好感度の絵柄の画面 4 件などが続く。
- **台帳の内訳**: 74 件（実装済み 7・語彙のみ 1・未対応 66・縮退 0・別名 0・対象外 0）。

#### ⑵ シェルの descript（`shell/master/descript.txt`）

- **分類**: 黙って捨てる。
- **根拠**: 行の割り方は ⑴ と同じ（`kv/parse.rs:20`・`:26`・`:39`）。引くのは `crates/areka-parsers/src/package/resolve.rs` の `read_bindgroup_defaults`（`:151`〜`:228`）で、着せ替えの既定（`:170`・`:175`）・名前（`:183`・`:190`）・選択肢（`:206`・`:216`）の 6 形だけを、`:116`〜`:126` の定数と突き合わせて拾う。ほかに文字コードの前走査 `charset/prescan.rs:57`、シェルの表示名 `crates/areka-ghost/src/config.rs:58`（`resolve_shell_name`・`:39`）、窓の置き場所 `crates/areka/src/placement/config.rs:139`〜`:141`・`:228`・`:262`・`:273`・`:278` が同じ表から引く。**それ以外の名前はどこからも引かれない。**
- **記録**: なし。壊れ方は「黙って壊れる」。文字コードの宣言だけが ⑴ と同じ例外である。なお、このクレートで唯一の警告段（`resolve.rs:306`）はここで働くが、**未知の名前に対するものではない**——着せ替えの名前宣言という既知の名前の、値の形が足りない行に対する記録である。
- **その記述を読むのは誰か**: 転記層では止まらない。着せ替えの既定は `areka-seriko` の面の解決へ、置き場所と重なり順は `areka` の窓の配置へ、名前は画面の表示へ届く。引き当てに当たらない名前は転記層より先へ進まない。
- **成立に要る基盤**: 作り付けのメニュー（自前描画）33 件・窓の配置と重なりの解決 31 件・着せ替え 12 件が大きく、ほかに通知領域とゴースト一覧の画面 9 件・資産の素性を保持する場所 8 件などが続く。
- **台帳の内訳**: 102 件（実装済み 11・語彙のみ 2・未対応 89・縮退 0・別名 0・対象外 0）。

#### ⑶ surfaces.txt（`shell/master/surfaces.txt`）

- **分類**: 黙って捨てる。
- **根拠**: `crates/areka-parsers/src/shell/decode.rs` が塊の見出しで振り分ける（`dispatch_block`・`:115`）。受けるのは 4 語だけで、`descript`（`:118`・中身ごと捨てる）・`kero.surface.alias`（`:122`）・`surface.append*`（`:127`）・`surfaceNNN`（`:132`）である。**どれにも当たらない見出しの塊は `:156` で何も積まずに終わる。** 塊の外の行は、並べ方の 2 語（`animation-sort`・`collision-sort`）だけが値になり、それ以外は `:91`〜`:93` と `:492`（`decode_sort_key`）で何もされない。塊の中でも、重ね合わせの行は第 2 欄が `overlay` のときだけ値になり（`:197`〜`:199`・`decode_elements`）、当たり判定の行は `collision` に続く部分が数字だけのときに限られる（`:234`〜`:236`・`decode_collisions`）——円・楕円・多角形を書く `collisionex` はここで**何も記録せずに読み飛ばされる**。
- **記録**: なし。ただし 2 つ、押さえておくことがある。1 つめ。アニメーションを起こす間隔の語だけは、転記のあと下流で記録が出る（`crates/areka-seriko/src/table.rs:111`・`:120`・`:129`）。段は控えめで既定の水準では見えないので、壊れ方は「黙って壊れる」である（要件 3.5a）。2 つめ。**このファイルの文字コードの宣言は読まれない。** ゴースト・シェル・バルーンの descript が前走査を通るのに対し、surfaces.txt を本番で読む 2 か所（`crates/areka/src/emo2_boot/assets.rs:279` の `build_boot_assets` と `crates/areka/src/placement/measure.rs:333` の `build_shell_assets`）はどちらも標準ライブラリの読み取りで UTF-8 として読む。UTF-8 以外で書かれたシェルは読み取りの段で失敗し、その失敗はエラー段に記録される（`measure.rs:334`・`emo2_boot/mod.rs:220`）。起動はそこで止まらず、シェルを出さないまま続く。宣言そのものは黙って消えるが、その帰結は既定の水準で見える。
- **その記述を読むのは誰か**: 転記層では止まらない。`areka-parsers` の `shell::parse` が組んだものを、`areka-seriko` がアニメーションの表へ、`areka-emo-compose` と `areka-emo-atlas` が絵の重ね合わせへ、`areka` の窓の配置が採寸へ使う。読み飛ばされた行は転記層にも残らない。
- **成立に要る基盤**: emo の合成器 69 件・SERIKO/MAYUNA の再生 26 件・マウスの矢印の差し替え 15 件が大きく、ほかに窓の配置と重なりの解決 12 件・当たり判定の形の拡張 5 件・シェルの定義ファイルの転記層 4 件が続く。
- **台帳の内訳**: 137 件（実装済み 4・語彙のみ 57・縮退 4・別名 4・未対応 68・対象外 0）。

#### ⑷ バルーンの descript（`balloon/<系列>/descript.txt`）

- **分類**: 黙って捨てる。
- **根拠**: `crates/areka-parsers/src/balloon/parse.rs` は完全一致で名前を引くだけで、引く場所は 31 か所・名前は 30 種である（`windowposition.x` だけが 2 か所から引かれる）。**引き当てに無い名前は、引かれないという理由だけで自然に落ちる。** そのことはソースにも書いてある（`:9`・`:39`）。
- **記録**: なし。壊れ方は「黙って壊れる」。文字コードの宣言だけは ⑴・⑵ と同じ例外で、綴りを解けないときに控えめな段の記録が出る（`charset/decode.rs:35`）。
- **その記述を読むのは誰か**: 転記層では止まらない。`areka-emo-present` の `balloon::load_scope_balloon_model`（`crates/areka-emo-present/src/balloon.rs:499`。読み取りは `:513`）が読み取って組んだものを、`areka` の窓の組み立てが持ち、文字を描く層（`areka-emo-text`）が字の枠とフォントに使う。引かれなかった名前は転記層より先へ進まない。
- **成立に要る基盤**: バルーンの中のリンク機能 43 件・選択肢の目印の描画 29 件・外との通信 26 件が大きく、ほかにバルーンの文字描画 24 件・バルーンに付属する画像の族 16 件・資産の素性を保持する場所 10 件が続く。
- **台帳の内訳**: 162 件（実装済み 20・語彙のみ 5・縮退 4・未対応 133・別名 0・対象外 0）。

#### ⑸ install.txt（配布アーカイブの配置指示）

- **分類**: 黙って捨てる（ファイル全体）。
- **根拠**: **読む経路が無い。** 転記層は自分でそう宣言している（`crates/areka-parsers/src/package/resolve.rs:7`〜`:8`。ただしこの宣言文自体が古く、実際には読んでいる `sakura.name2` が並びから漏れている）。結果に影響しないことは試験でも固めてある（`crates/areka-parsers/src/package/validation_tests.rs:113`・`:122`〜`:125`・テストそのものは `:127`）。適合の相手にしている試験用ゴーストは正典どおりの `install.txt` を同梱しているが、開かれない。
- **記録**: なし。ファイルが開かれないので、読み取りの失敗すら起きない。壊れ方は「黙って壊れる」——作者から見ると「置いたのに何も起きず、何も言われない」形になる。
- **その記述を読むのは誰か**: 誰も読まない。転記層にすら届かない。
- **成立に要る基盤**: 配布と更新の仕組み（15 件すべて）。手に入れる・展開する・置く・消す、の一式が先に要る。
- **台帳の内訳**: 15 件（未対応 15・実装済み 0・語彙のみ 0・縮退 0・別名 0・対象外 0）。

#### ⑹ プラグインの descript（`plugin/<名前>/descript.txt`）

- **分類**: 黙って捨てる（ファイル全体）。
- **根拠**: **読む経路が無い。** 解析するコードが 1 行も無い。この綴りが出てくるのはプロパティの名前の予約だけで（`crates/areka-sylphya/src/vocab/dotted.rs:25` の `pluginlist`）、同じファイルの `:101`〜`:105` が「いまは名前の予約に留め、実際に動かすのは先の段階」と書いている。
- **記録**: なし。壊れ方は「黙って壊れる」。
- **その記述を読むのは誰か**: 誰も読まない。
- **成立に要る基盤**: プラグインとヘッドラインの仕組み（13 件すべて）。プラグインとの通信の口が先に要る。
- **台帳の内訳**: 13 件（未対応 13・実装済み 0・語彙のみ 0・縮退 0・別名 0・対象外 0）。

#### ⑺ ヘッドラインの descript（`headline/<名前>/descript.txt`）

- **分類**: 黙って捨てる（ファイル全体）。
- **根拠**: **読む経路が無い。** ⑹ と同じ形で、出てくるのは名前の予約だけである（`crates/areka-sylphya/src/vocab/dotted.rs:24` の `headlinelist`・同 `:101`〜`:105`）。
- **記録**: なし。壊れ方は「黙って壊れる」。
- **その記述を読むのは誰か**: 誰も読まない。
- **成立に要る基盤**: プラグインとヘッドラインの仕組み（9 件すべて）。外の見出しを取りに行く経路が先に要る。
- **台帳の内訳**: 9 件（未対応 9・実装済み 0・語彙のみ 0・縮退 0・別名 0・対象外 0）。

#### ⑻ surfacetable.txt（サーフェスの名前表）

- **分類**: 黙って捨てる（ファイル全体）。
- **根拠**: **読む経路が無い。** シェルの置き場所から areka が開くファイルの名前は定数で 2 つだけ持っており（`crates/areka/src/emo2_boot/assets.rs:82` の `surfaces.txt` と `:84` の `descript.txt`）、この綴りは本番の経路のどこにも現れない。試験用ゴーストはこのファイルを実際に同梱しているが、開かれない。
- **記録**: なし。壊れ方は「黙って壊れる」。
- **その記述を読むのは誰か**: 誰も読まない。
- **成立に要る基盤**: シェルの定義ファイルの転記層（6 件すべて）。まずこのファイルを開いて読む口が要る。
- **台帳の内訳**: 6 件（未対応 6・実装済み 0・語彙のみ 0・縮退 0・別名 0・対象外 0）。

#### ⑼ 更新ファイル（`updates2.dau`・`updates.txt`・`delete.txt`）

- **分類**: 黙って捨てる（ファイル全体）。
- **根拠**: **読む経路が無い。** `crates/` の Rust から `updates2.dau`・`updates.txt`・`.nar` を指す行は 1 つも無く、更新のできごとの名前（`OnUpdate` で始まるもの）を指す行も 1 つも無い。`delete.txt` の綴りが出てくるのは `crates/areka-parsers/src/package/validation_tests.rs:115`・`:123` の 2 行だけで、どちらも「これがあっても結果は変わらない」ことを固める試験の説明の文であり、読み取りではない。ネットワークの出入りも、書庫を展開する仕掛けも無い。
- **記録**: なし。壊れ方は「黙って壊れる」。
- **その記述を読むのは誰か**: 誰も読まない。
- **成立に要る基盤**: 配布と更新の仕組み（9 件すべて）。相手のサーバとやりとりする経路と、ファイルを入れ替える手順が先に要る。
- **台帳の内訳**: 9 件（未対応 9・実装済み 0・語彙のみ 0・縮退 0・別名 0・対象外 0）。

### 読む経路が無い 5 種をファイル単位で切る理由

⑸〜⑼ の 5 種は、書いてある記述の 1 つ 1 つを見るまでもなく、**ファイルが開かれずに終わる**。開かれないので、名前が正典どおりでも綴りを間違えていても結果は同じであり、記述の単位で「これは読まれる／読まれない」と分けても意味を持たない。だから 5 種は種別まるごと 1 つの判断として扱い、記述単位の分類はしない（要件 3.8）。

台帳のほうは正典の項目 1 つに 1 つの行を持つ形なので、この 5 種に属する 52 件（15＋13＋9＋6＋9）にも行がある。その行の状態はすべて「未対応」で、備考にも「ファイル全体が読まれずに終わるので、記述単位の仕訳はしない」と同じ趣旨が書いてある。**行があることと、記述単位で仕訳けたこととは別である。**

### 設計の表から直した 2 か所

設計の同じ主題の表と、測り直した結果とが食い違った箇所が 2 つある。**結論はどちらも動かないが、根拠の書き方が正しくなかった。**

1. **surfacetable.txt が Rust に現れる行数は 0 ではなく 6 である。** 設計の表は「`crates/` に `surfacetable` を含む Rust の行が 0 件」を根拠にしているが、実際には 6 行ある——`crates/ukadoc-survey/src/assignment.rs:48`・同 `assignment_tests.rs:56`・`:246`・`:327`・同 `model_tests.rs:49`・`:50`。**6 行とも、この網羅調査のための道具が正典のページの名前として持っているもので、シェルのファイルを読む経路ではない。** よって「読む経路が無い」という結論は動かない。
2. **「読む経路が無い」という言い方は、ページ 1 枚をまとめて指す項目にまで及ぶように読めるが、そこは「一部だけ読む」である。** `manual_shell`・`manual_ghost`・`manual_directory`・`manual_balloon`・`dev_shell`・`dev_bind` の 6 件は、そのページが並べるファイルの一部に読む経路があり、ページ全体としては満たしていない、という形である。たとえば `manual_shell` が並べるファイルのうち areka が開くのは `crates/areka/src/emo2_boot/assets.rs:82` が指す 1 本の surfaces.txt までで、分割された追加ファイル・別名のファイル・surfacetable.txt・翻訳の DLL・メニュー用の画像には経路が無い。`dev_bind` は起動時の着せ替えの初期集合を組むところまでは動く（`crates/areka/src/emo2_boot/assets.rs:55` の `default_bind_ids`）が、利用者が着せ替えを選ぶ入口である作り付けのメニューが無い。**「読む経路が無い」と言い切ってよいのは、上の ⑸〜⑼ の 5 種だけである。** 台帳の 6 件の備考も同じ書き方をしている。

## nar インストールとネットワーク更新の導線

配られているゴーストを手に入れて、自分の環境に入れて、あとから新しくするまでの道を、6 つの段に分けて並べた節である。段は **入手 → 展開 → 配置 → 起動 → 更新 → 削除** の順に置く。各段について ⑴ その段が成り立つために要る正典の項目、⑵ 最小で何ができれば成り立つか、⑶ areka のいまの状態、の 3 つを対にして書く。

**この節は作り方を決めない。** 書庫を開く部品に何を選ぶか、通信をどう組むか、どの段をどの spec が引き受けるかは、いずれもここでは決めていない。ここで確かめたのは「何が要るか」と「いま何が在るか」の 2 つだけである。

**本節が引く行番号は 2026-09-06 に測ったものである。** 行番号はファイルに 1 行足すだけで動くので、どこでもファイル名と、その中の定義や見出しの名前を併せて書いた。

### 測り直した areka の現状

この節が立つ土台は「いま何も無い」という一連の 0 である。0 は書かなければ沈黙と同じなので、1 つずつ数え方を添えて並べる。数え方が違えば数も違うため、何をどう数えたかまで書いた。値はすべて 2026-09-06 に測り直したものである。

| 測ったもの | 数え方 | 結果 |
|---|---|---|
| `updates2.dau` の綴り | `crates/` 配下の `.rs` を全文検索 | **0 行** |
| `updates.txt` の綴り | 同上 | **0 行** |
| `.nar` の綴り | 同上 | **0 行** |
| `nar` という語（大小文字を問わず・前後が英字でないもの） | 同上 | 9 行。内訳は「触れない」という宣言 2 行（`areka-parsers` の `package::resolve` の前置きと、`package::validation_tests` が無影響の採取元として控えている行）と、この網羅調査の道具 `ukadoc-survey` が正典のページ名 `dev_nar` を持つ 7 行。**読み取りの経路は 1 行も無い** |
| `OnUpdate` の綴り | 同上 | **0 行**。`crates/` 全体まで広げると 5 行あるが、いずれも試験用ゴーストの辞書 `crates/pilot/examples/shiori-host-32/fixtures/emo2/ghost/master/dic/update.pasta` の中で、areka のコードではない |
| `install.txt` の綴り | 同上 | 3 行。`package::resolve` の前置きの「触れない」宣言と、`package::validation_tests` の説明 2 行 |
| `delete.txt` の綴り | 同上 | 2 行。どちらも同じ `package::validation_tests` の説明 |
| 書庫を開く依存 | `Cargo.lock` のパッケージ名に `zip`・`tar`・`archive`・`compress`・`7z`・`lzma` を含むもの | **0 件** |
| 圧縮まわりの依存 | `Cargo.lock` を子から親へ逆にたどる | 5 件（`flate2`・`miniz_oxide` の 2 つの版・`zlib-rs`・`crc32fast`）。親をたどると `flate2` は `png` から、`zlib-rs` と `crc32fast` は `flate2` から、`png` は `image` から、`image` は `wintf` から来ている。すなわち絵を読み込むために付いてきたものである（`miniz_oxide` だけは `flate2` と `png` のほかに `backtrace` からも来ている）。**書庫を開くために入れたものは 1 つも無い** |
| 通信の依存 | `Cargo.lock` のパッケージ名に `reqwest`・`hyper`・`ureq`・`curl`・`tokio`・`rustls`・`native-tls`・`http`・`url` を持つもの | **0 件** |
| 通信の呼び出し | `crates/` 配下の `.rs` に `std::net`・`TcpStream`・`TcpListener`・`UdpSocket`・`WinHttp`・`InternetOpen`・`Networking`・`Winsock`・`Wininet` の綴り | **0 行** |
| 取り違えを見つけるための照合の道具 | `Cargo.lock` のパッケージ名に `md5`・`sha1`・`sha2`・`digest` を含むもの／`.rs` に `md5` の綴り | どちらも **0** |

台帳の側でも同じことが数で出ている。この導線に関わるページの内訳は次のとおりで、**`descript_install` の 15 件と `spec_update_file` の 9 件はすべて未対応**である。実装済み・語彙のみ・縮退・別名はいずれも 0 件で、未分類も 0 件である。ページ 1 枚をまとめて指す `manual_install`・`manual_update`・`manual_directory`・`dev_nar`・`dev_update` の 5 件も、すべて未対応である。

**在るのは 1 段だけである。** 6 段のうち areka の側に実物が立っているのは ④ 起動だけで、①②③⑤⑥ には何も無い。下の各段の「areka のいま」は、この 1 つの例外を除いてすべて同じ結論になる。同じ文が 6 回並ぶのは冗長に見えるが、段ごとに「無い」の中身（読み取りが無いのか、依存が無いのか、そもそも入口が無いのか）が違うので、段ごとに書き分けた。

### 実例に使う 7 本のファイル

読む側の実装は無いが、**正典の書式で書かれた実物は試験用ゴーストの中に 7 本ある**。置き場を数え直したところ、うち 2 本は `fixtures/emo2/` の中ではなくその兄弟にあった。7 本という数のほうは合っていた。各段の実例にはこの 7 本を使う。いずれも `crates/pilot/examples/shiori-host-32/fixtures/` から下の道を書く。

| # | 置き場（`fixtures/` から下） | 何のファイルか | 中身 | 使う段 |
|---|---|---|---|---|
| 1 | `emo2/install.txt` | 配布物を入れる指示（ゴースト本体） | 6 行。文字コードの宣言・種別・名前・入れ先の名前・同梱バルーンの入れ先と取り出し元 | ③ |
| 2 | `emo2/emo2-kakukaku/install.txt` | 同（同梱バルーン） | 4 行。文字コードの宣言・種別・名前・入れ先の名前 | ③ |
| 3 | `emo2-kakukaku-offsetdpi/install.txt` | 同（検証用に増やしたバルーン） | 4 行。同じ 4 つの欄 | ③ |
| 4 | `emo2-kakukaku-wplimit/install.txt` | 同（検証用に増やしたバルーン） | 4 行。同じ 4 つの欄 | ③ |
| 5 | `emo2/updates.txt` | 更新のときに突き合わせる一覧 | 109 行。文字コードの宣言 1 行＋ファイル 1 つにつき 1 行の 108 行。各行は道・取り違えを見つけるための値・大きさ・日付を区切り文字でつないでいる | ⑤ |
| 6 | `emo2/ghost/master/updates.txt` | 同（ゴースト側の置き場に置いた写し） | #5 とバイト単位で同一である（`cmp` で確認） | ⑤ |
| 7 | `emo2/delete.txt` | 更新のときに消す道の指示 | 2 行。文字コードの宣言と、消す道 1 つ | ⑥ |

3 つ気づいたことがある。⑴ **`updates2.dau` は 7 本の中に無い。**試験用ゴーストが持っているのは `updates.txt` の側だけである。⑵ **#1 だけ文字コードの宣言の綴りが `Charset` と大文字で始まる**（他の 6 本は `charset`）。読む側が無いので今は何も起きないが、読む側を作るときに大小文字をどう扱うかがそのまま効く箇所である。⑶ #5 と #6 が同じ中身で 2 か所にあるのは書き間違いではない。正典には `ghost\masterへのコピー` という見出しの項目があり（下の ⑤ の表）、ゴーストの置き場を説明するページも更新用のファイルを並べるものとして挙げている（台帳の `ukadoc:manual_ghost` の備考）。読む側を作るときには、2 か所のどちらを見るかが決めるべきことの 1 つになる。

### ① 入手

- **要る正典の項目**: `ukadoc:manual_install`（インストールのページ全体）・`ukadoc:dev_nar`（配布用の書庫の作り方のページ全体）・`ukadoc:descript_ghost:homeurl_2cURL:1`（配布元の宛先）。
- **最小で何ができれば成り立つか**: 手元のファイルを受け取る口と、宛先から取ってくる口の 2 つ。前者だけでも「手に入れる」は成り立つ（利用者が自分でファイルを落としてきて渡す形）。後者は通信の仕組みが要る。
- **areka のいま**: 手元のファイルを受け取る口も無く、通信の依存も呼び出しも 0 である。areka が受け取れるのは**すでに展開されたフォルダ**だけで、そこへ至る道が無い。宛先の欄（`homeurl`）は台帳では未対応で、読む経路が無い。
- **実例**: この段に対応する実物のファイルは 7 本の中に無い。書庫そのものが試験用ゴーストに含まれていないためである。

### ② 展開

- **要る正典の項目**: `ukadoc:dev_nar`（書庫の作り方）・`ukadoc:manual_directory`（展開したあとの全体の構成）。
- **最小で何ができれば成り立つか**: 書庫を開いて中身を取り出せること、中の道の文字コードを扱えること、取り出す先を一時的に確保できること。
- **areka のいま**: 書庫を開く依存が 1 つも無い。圧縮まわりの依存は 5 件あるが、いずれも絵の読み込みのために付いてきたもので、書庫を開く用ではない。台帳では `dev_nar`・`manual_directory` とも未対応である。
- **実例**: この段に対応する実物のファイルも 7 本の中に無い。試験用ゴーストは**展開済みの形で置かれている**からで、これがそのまま「areka の入口は展開済みのフォルダである」という現状を表している。

### ③ 配置

- **要る正典の項目**: `ukadoc:manual_install` と、`descript_install` の 15 件のうち更新に関わる 4 件を除いた 11 件。

| 見出し | 項目 id |
|---|---|
| `type,種別` | `ukadoc:descript_install:type_2c_7a2e_5225:1` |
| `name,オブジェクト名` | `ukadoc:descript_install:name_2c_30aa_30d6_30b8_30a7_30af_30c8_540d:1` |
| `directory,ディレクトリ名` | `ukadoc:descript_install:directory_2c_30c7_30a3_30ec_30af_30c8_30ea_540d:1` |
| `*.directory,ディレクトリ名` | `ukadoc:descript_install:_2a.directory_2c_30c7_30a3_30ec_30af_30c8_30ea_540d:1` |
| `*.source.directory,ディレクトリ名` | `ukadoc:descript_install:_2a.source.directory_2c_30c7_30a3_30ec_30af_30c8_30ea_540d:1` |
| `accept,本体側名` | `ukadoc:descript_install:accept_2c_672c_4f53_5074_540d:1` |
| `bootghost,ディレクトリ名` | `ukadoc:descript_install:bootghost_2c_30c7_30a3_30ec_30af_30c8_30ea_540d:1` |
| `charset,文字コード` | `ukadoc:descript_install:charset_2c_6587_5b57_30b3_30fc_30c9:1` |
| `相対パス` | `ukadoc:descript_install:_76f8_5bfe_30d1_30b9:1` |
| `相対パス,オプション1,オプション2,...` | `ukadoc:descript_install:_76f8_5bfe_30d1_30b9_2c_30aa_30d7_30b7_30e7_30f31_2c_30aa_30d7_30b7_30e7_30f32_2c...:1` |
| `相対パス,ignore` | `ukadoc:descript_install:_76f8_5bfe_30d1_30b9_2cignore:1` |

- **最小で何ができれば成り立つか**: install.txt を読めること、種別から置き場を決められること、入れ先の名前を決められること、すでに同じものが入っているときの扱いを決められること、入れないファイルの指定を守れること。同梱のバルーンのように 1 つの書庫が 2 つ以上のものを含む形も扱えること。最後の `bootghost` は、入れ終わったあとにどれを起こすかを指す欄で、次の ④ 起動へ引き渡す継ぎ目にあたる。
- **areka のいま**: install.txt を読む経路がファイルごと無い。`areka-parsers` の `package::resolve` は前置き（ファイル冒頭のモジュール説明・8 行目）で「install.txt には触れない」と自ら宣言しており、同じ束の `package::validation_tests` が、試験用ゴーストに置いてあっても解決の結果へ一切漏れないことをテストで固定している（採取元の控えが 113 行目と 115 行目、説明が 122〜125 行目、テストそのものが 127 行目）。台帳では、上の表の 11 件も `ukadoc:manual_install` も未対応である（12 件）。
- **実例**: 上の 7 本のうち #1〜#4。#1 が「ゴースト本体＋同梱バルーン」の形、#2〜#4 がバルーン単体の形である。#1 の同梱バルーンの欄が #2 の入れ先の名前と同じ綴りで対応している。

### ④ 起動

- **要る正典の項目**: `ukadoc:manual_ghost`・`ukadoc:manual_shell`・`ukadoc:manual_balloon`（ゴースト・シェル・バルーンそれぞれの置き場のページ全体）と、ゴーストの descript の 3 件。

| 見出し | 項目 id | 台帳の状態 |
|---|---|---|
| `shiori,ファイル名` | `ukadoc:descript_ghost:shiori_2c_30d5_30a1_30a4_30eb_540d:1` | 実装済み |
| `seriko.defaultsurfacedirectoryname,ディレクトリ名` | `ukadoc:descript_ghost:seriko.defaultsurfacedirectoryname_2c_30c7_30a3_30ec_30af_30c8_30ea_540d:1` | 実装済み |
| `name,ゴースト名` | `ukadoc:descript_ghost:name_2c_30b4_30fc_30b9_30c8_540d:1` | 語彙のみ |

- **最小で何ができれば成り立つか**: 置かれた木からゴーストの定義を読み、頭脳の入れ物を起こし、立ち絵の置き場を決めて絵を出せること。
- **areka のいま**: **6 段のうちここだけが立っている。**`areka-parsers` の `package::resolve` が、展開済みの根から `ghost/master/descript.txt` を起点にして、頭脳のファイル名と立ち絵の置き場を解決する。ページ全体の 3 件（`manual_ghost`・`manual_shell`・`manual_balloon`）はいずれも未対応だが、これはページが並べるファイルのうち読まないものが残っているためで、起動そのものは通る。台帳の内訳では、ゴーストの descript 74 件のうち実装済み 7・語彙のみ 1・未対応 66、シェルの descript 102 件のうち実装済み 11・語彙のみ 2・未対応 89 である。
- **実例**: 試験用ゴースト `fixtures/emo2/` そのもの。①②③ を人手で済ませた状態のものを渡している、というのがいまの形である。

### ⑤ 更新

- **要る正典の項目**: `ukadoc:manual_update`（ネットワークのページ全体）・`ukadoc:dev_update`（ネットワーク更新に対応するための準備のページ全体）・`ukadoc:descript_ghost:homeurl_2cURL:1`（宛先）と、`spec_update_file` の 9 件、および install.txt の更新に関わる欄のうち入れ替えを指す 2 件。

| 見出し | 項目 id |
|---|---|
| `文字コード` | `ukadoc:spec_update_file:_6587_5b57_30b3_30fc_30c9:1` |
| `行フォーマット` | `ukadoc:spec_update_file:_884c_30d5_30a9_30fc_30de_30c3_30c8:1` |
| `行種別` | `ukadoc:spec_update_file:_884c_7a2e_5225:1` |
| `必須フィールド` | `ukadoc:spec_update_file:_5fc5_9808_30d5_30a3_30fc_30eb_30c9:1` |
| `拡張フィールド (位置[2]以降)` | `ukadoc:spec_update_file:_62e1_5f35_30d5_30a3_30fc_30eb_30c9_20_28_4f4d_7f6e_5b2_5d_4ee5_964d_29:1` |
| `ファイル走査` | `ukadoc:spec_update_file:_30d5_30a1_30a4_30eb_8d70_67fb:1` |
| `URLエンコード` | `ukadoc:spec_update_file:URL_30a8_30f3_30b3_30fc_30c9:1` |
| `セキュリティチェック` | `ukadoc:spec_update_file:_30bb_30ad_30e5_30ea_30c6_30a3_30c1_30a7_30c3_30af:1` |
| `ghost\masterへのコピー` | `ukadoc:spec_update_file:ghost_5cmaster_3078_306e_30b3_30d4_30fc:1` |
| `refresh,数値` | `ukadoc:descript_install:refresh_2c_6570_5024:1` |
| `*.refresh,数値` | `ukadoc:descript_install:_2a.refresh_2c_6570_5024:1` |

- **最小で何ができれば成り立つか**: 宛先から一覧を取ってくること、手元の実物と一覧を突き合わせて違っているものを見つけること、違っていたものだけを取り直すこと、取ったものを置き場へ写すこと、道の綴りを安全に扱うこと、受け取ったものが期待どおりかを確かめること。突き合わせには取り違えを見つけるための照合の道具が要る。
- **areka のいま**: 更新ファイルを読む経路がファイルごと無く、通信の依存も呼び出しも 0 で、照合の道具も入っていない。更新を知らせる出来事（`OnUpdate` で始まる名前）を発火する場所も 0 である。台帳では、上の表の 11 件も `ukadoc:manual_update`・`ukadoc:dev_update`・宛先の欄も未対応である（14 件）。
- **実例**: 上の 7 本のうち #5 と #6。突き合わせの一覧が正典の書式でどう並ぶかは、この 108 行で読める。ただし `updates2.dau` の実物は無い。

### ⑥ 削除

- **要る正典の項目**: `ukadoc:manual_update`・`ukadoc:dev_update` と、install.txt の更新に関わる欄のうち消さずに残すものを指す 2 件。

| 見出し | 項目 id |
|---|---|
| `refreshundeletemask,ファイル名1:ファイル名2...` | `ukadoc:descript_install:refreshundeletemask_2c_30d5_30a1_30a4_30eb_540d1_3a_30d5_30a1_30a4_30eb_540d2...:1` |
| `*.refreshundeletemask,ファイル名1:ファイル名2...` | `ukadoc:descript_install:_2a.refreshundeletemask_2c_30d5_30a1_30a4_30eb_540d1_3a_30d5_30a1_30a4_30eb_540d2...:1` |

- **正典の側に細かい項目が無いことについて。**この段だけは、正典に見出しの粒度で対応する項目がほとんど無い。カタログの 1,749 件を見出しの文字で当たったところ、`delete.txt`・`updates2.dau`・`updates.txt` という綴りを見出しに持つ項目は**それぞれ 0 件**である。これらのファイルそのものは、ページ 1 枚をまとめて指す `manual_update` と `dev_update` の中で扱われている。つまりこの段は「項目が足りない」のではなく、**正典がページの説明として書いていて、見出しに切り出していない**のである。台帳に無い行を作ることはできないので、この段の受け皿はページ全体の 2 件と、上の 2 件になる。
- **最小で何ができれば成り立つか**: 消す道の指示を読めること、消してはいけないものの指定を守れること、指示された道が置き場の外を指していないかを確かめられること。
- **areka のいま**: 更新ファイルを読む経路がファイルごと無い。`delete.txt` は試験用ゴーストに実際に置いてあるが、`areka-parsers` の `package::validation_tests` が、置いてあっても解決の結果へ一切漏れないことを固定している。台帳では、上の表の 2 件も `ukadoc:manual_update`・`ukadoc:dev_update` も未対応である（4 件）。
- **実例**: 上の 7 本のうち #7。2 行しかないが、正典の書式で消す道を書いた実物である。

### 本ドメインの外へ伸びる先

導線の入口と出口は本ドメインの外にある。**外の項目は台帳へ写さず、関連の欄で指す。**さくらスクリプトの実行タグは sakura-script 台帳が、更新と設置を知らせる出来事は shiori 台帳が、それぞれ項目そのものを持っている。ここで複製すると同じものが 2 か所で数えられてしまう。

置いたのは 19 本で、置き先はページ 1 枚をまとめて指す 4 件だけである。相手の項目 id はカタログから写した（打ち直していない）。相手が実在するかどうかは、上流の道具の整合検査が毎回見ている。

| 置き先 | 相手の見出し | 相手の項目 id | 種別 |
|---|---|---|---|
| `ukadoc:manual_install` | `\![execute,install,path,ファイル名]` | `ukadoc:list_sakura_script:_5c_21_5bexecute_2cinstall_2cpath_2c_30d5_30a1_30a4_30eb_540d_5d:1` | `same-feature` |
| `ukadoc:manual_install` | `\![execute,install,url,URL,(feed\|nar\|homeurlのいずれか)]` | `ukadoc:list_sakura_script:_5c_21_5bexecute_2cinstall_2curl_2cURL_2c_28feed_7cnar_7chomeurl_306e_3044_305a_308c_304b_29_5d:1` | `same-feature` |
| `ukadoc:manual_install` | `OnInstallComplete` | `ukadoc:list_shiori_event:OnInstallComplete:1` | `same-feature` |
| `ukadoc:manual_install` | `OnInstallCompleteAll` | `ukadoc:list_shiori_event:OnInstallCompleteAll:1` | `same-feature` |
| `ukadoc:manual_install` | `OnInstallRefuse` | `ukadoc:list_shiori_event:OnInstallRefuse:1` | `same-feature` |
| `ukadoc:manual_install` | `OnInstallReroute` | `ukadoc:list_shiori_event:OnInstallReroute:1` | `same-feature` |
| `ukadoc:dev_nar` | `\![execute,createnar]` | `ukadoc:list_sakura_script:_5c_21_5bexecute_2ccreatenar_5d:1` | `same-feature` |
| `ukadoc:dev_nar` | `OnNarCreating` | `ukadoc:list_shiori_event:OnNarCreating:1` | `same-feature` |
| `ukadoc:dev_nar` | `OnNarCreated` | `ukadoc:list_shiori_event:OnNarCreated:1` | `same-feature` |
| `ukadoc:manual_update` | `\![update,更新対象(,オプション,オプション...)]` | `ukadoc:list_sakura_script:_5c_21_5bupdate_2c_66f4_65b0_5bfe_8c61_28_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._29_5d:1` | `same-feature` |
| `ukadoc:manual_update` | `\![updateother,更新対象/オプション群,...]` | `ukadoc:list_sakura_script:_5c_21_5bupdateother_2c_66f4_65b0_5bfe_8c61_2f_30aa_30d7_30b7_30e7_30f3_7fa4_2c..._5d:1` | `same-feature` |
| `ukadoc:manual_update` | `OnUpdateProcessExec` | `ukadoc:list_shiori_event:OnUpdateProcessExec:1` | `same-feature` |
| `ukadoc:manual_update` | `OnUpdateBegin` | `ukadoc:list_shiori_event:OnUpdateBegin:1` | `same-feature` |
| `ukadoc:manual_update` | `OnUpdateReady` | `ukadoc:list_shiori_event:OnUpdateReady:1` | `same-feature` |
| `ukadoc:manual_update` | `OnUpdateComplete` | `ukadoc:list_shiori_event:OnUpdateComplete:1` | `same-feature` |
| `ukadoc:manual_update` | `OnUpdateFailure` | `ukadoc:list_shiori_event:OnUpdateFailure:1` | `same-feature` |
| `ukadoc:dev_update` | `\![execute,createupdatedata]` | `ukadoc:list_sakura_script:_5c_21_5bexecute_2ccreateupdatedata_5d:1` | `same-feature` |
| `ukadoc:dev_update` | `OnUpdatedataCreating` | `ukadoc:list_shiori_event:OnUpdatedataCreating:1` | `same-feature` |
| `ukadoc:dev_update` | `OnUpdatedataCreated` | `ukadoc:list_shiori_event:OnUpdatedataCreated:1` | `same-feature` |

**種別に `same-feature` を選んだ理由。**使える種別は 6 つあり、そのうち「これをすると、あの出来事が飛ぶ」を表すものと「この設定が、あれの動きを決める」を表すものは、片側が操作や設定キーであることを前提にしている。ここで置き先にしている 4 件はいずれも**ページ 1 枚をまとめて指す粗い粒度の項目**で、操作でも設定キーでもないので、その向きには当てはまらない。残る「同じ機能の別の面」＝ `same-feature` が、書式の側と入口の側と知らせの側が 1 つの機能の 3 つの面である、という関係をそのまま表す。向きを持たないので、どちらの側から読んでも同じ意味になる。

**台帳の備考が書いている「関連の向き」の規則との関係。**4 件の備考には、正典の本文が相手のページを名指しで指しているときに、指している側にだけ関連を置く、という規則が書いてある。あれは**本ドメインの中のページどうし**の重複を避けるための規則で、どちらに置くか迷う場面のための決め方である。ここで足した 19 本はドメインを跨ぐもので、迷う余地が無い（相手の項目は別の台帳の持ち物であり、こちら側にしか置けない）。2 つは別の規則であり、先に置かれていた関連は 1 本も動かしていない。

**この 19 本は要件が名指しで求めた最小の集合そのものである。**「導線に関わる外の項目をすべて挙げた」とは言えない——たとえば更新の途中経過を知らせる出来事は上に挙げた以外にも多数あり、この節はそれらを数えていない。ここで保証するのは、要件が最低限として並べた実行タグ 6 種と出来事 13 種が漏れなく指されていることだけである。

**跨いだ関連は、報告の束としては現れない。**相手が別の台帳の持ち物である関連は、ドメイン別の報告に載る束の材料にならない（束は構成する項目の全員が同じ台帳に居るときだけ載る）。この 19 本を目で追えるのは、いまのところ台帳の該当する 4 件と、この表だけである。全体をまとめる側へ申し送るべき事柄として、ここに書き残しておく。

### 既存の判断記録は 1 件も無い

導線に関わる部分について、**これまでに下した判断の記録は 1 件も無い**。

- `doc/COMPAT_ARCHITECTURE.md` の沈黙ルール対応表のデータ行を全数当たった。何をデータ行と数えるかと、その件数は、下の「沈黙ルール対応表の一覧」節の「数え方と、この節の数がいつのものか」に書いた手順（見出し「8. 沈黙ルール対応表」から次の同じ深さの見出しの直前までを切り、縦棒で始まる行のうち区切りの行と見出しの行を除く）による——本節では件数を書かない。表は spec が 1 本着地するたびに伸びるので、同じ数を 2 か所に置くと片方だけが古くなるためである。その全データ行のうち、`install.txt`・`updates2.dau`・`updates.txt`・`delete.txt`・`nar`（大小文字を問わず・前後が英字でないもの）・「ネットワーク更新」・「インストール」・「配布」のいずれかを、どの列でもよいから含む行は **0 行**である（2026-09-06 に測り直した）。すなわちこの導線について areka が何かを決めた記録は、この表には無い。

**既存の言及はいずれも範囲の線を引いた宣言である。**どう振る舞わせるかを決めた記録は 1 つも無く、範囲の内と外を分ける文だけが 5 か所ある。

| 場所 | 何と書いてあるか |
|---|---|
| `doc/emo2-conformance-scope.md` の「5. パッケージローダ／配置規約」の 1 つ目の項目（73 行目） | M1 の置き付けの入口を install.txt の種別に取ると書いている。**ここだけは「外す」とは書いておらず、範囲の内側に線を引いた文である。**ただし実装はそのとおりになっておらず、`areka-parsers` の `package::resolve` は install.txt を読まずに `ghost/master/descript.txt` から始まる（上の測り直し） |
| 同 節の 3 つ目の項目（75 行目） | `delete.txt` は更新のときの旧い道の削除指示であり、M1 の置き付けでは無視してよい |
| 同 節の 4 つ目の項目（76 行目） | NAR の設置の仕組みは M1 の範囲外である（展開済みのフォルダを渡す） |
| 同 「7. 生態系拡張」の本文（92 行目） | 他のゴーストまで広げるには NAR の設置などが順次要る。emo2 の区切りを越えたあとの互換面の拡大として扱う |
| `crates/areka-parsers/src/package/resolve.rs` の前置き（8 行目） | 参照する欄を数えたうえで、install.txt・バルーン系・NAR には触れないと宣言する |

**89 行目について。**設計の「実測の訂正」の表は、以前の要件が挙げていた 89 行目が空行であることを訂正として記録している。測り直したところ、確かに 89 行目は空行だった。いまの要件は既に 73・75・76・92 の 4 か所へ直っているので、この節が引くのはその 4 か所である。要件と設計はこの 4 か所をまとめて「対象外の宣言」と呼んでいる。読み直すと、**3 か所（75・76・92）はそのとおりの「外す」宣言で、73 行目だけは範囲の内側に線を引いた文である**——ただし、そこに書かれた入口を読む実装は無い（上の測り直し）。**5 か所のどれもが、どう振る舞わせるかを決めた記録ではない**という点は共通する。要件 5.7 が確かめたかったこと——判断の記録が 1 件も無く、あるのは範囲についての宣言だけであること——は、この読み直しでも成り立つ。

### 対象外の候補として区別するもの

導線は「利用者が受け取る側」の話である。それに対し、**配る側が使う機能は導線の対象外の候補**として分けておく。区別しておかないと、利用者に届かないものが導線の見積もりに紛れ込む。

| 対象外の候補 | 項目 | 台帳での扱い |
|---|---|---|
| 配布用の書庫を作ること | `ukadoc:dev_nar` | 優先度だけを付け、テーマは付けていない。担当は空 |
| 更新用の一覧を作ること | `ukadoc:dev_update` | 同上 |

これに対応する外の項目——`\![execute,createnar]`・`\![execute,createupdatedata]` と、`OnNarCreating`／`OnNarCreated`／`OnUpdatedataCreating`／`OnUpdatedataCreated` の 4 つの出来事——も同じ側にある。上の関連の表では、これらを `dev_nar` と `dev_update` に寄せて置いた。導線の 6 段に関わる関連（`manual_install` と `manual_update` に置いたもの）とは、置き先で分かれている。

**「対象外」と決めたわけではない。**ここでしているのは候補としての区別だけで、実際に外すかどうかは全体をまとめる側の判断である。台帳の側でも状態を変えていない（4 件とも未対応のままである）。

### 将来 spec を切り出すときの自然な境界 3 つ

この導線と、その周りに残っている未対応の塊を眺めると、切り口が 3 つ見える。**ここでは境界を挙げるだけで、spec は起票しない。**

1. **定義ファイルの解釈**——すでにある転記層を広げる話。ゴースト・シェル・バルーンの descript と surfaces.txt について、いま読んでいない欄を読むようにする。土台は既にあり、増えるのは受け付ける欄の数である。
2. **配布と更新**——新しい土台が要る話。書庫を開くこと、通信すること、突き合わせること、置き換えること。この節の 6 段のうち ④ 起動を除く 5 段がここに入る。上の測り直しが示すとおり、依存も呼び出しも 0 からの出発になる。
3. **surfaces.txt の SERIKO/MAYUNA**——それだけで 1 本になる大きさの話。ページ 1 枚で 137 項目あり、本ドメインの中では最も大きい塊である。1 と重なるように見えるが、動きの組み立てという別の仕組みを伴うので分けた。

この 3 つは互いに重ならないが、**本ドメインの未対応を全部覆うわけでもない**（たとえばプラグインとヘッドライン、オーナードローメニュー、トランスレータはどれにも入らない）。3 つに絞ったのは、この導線から見て自然に切れる線がここだったからである。

## 沈黙ルール対応表の一覧

`doc/COMPAT_ARCHITECTURE.md` の「沈黙ルール対応表」は、ukadoc が何も書いていない箇所や書き方が曖昧な箇所について、areka が自分でどう決めたかを 1 行ずつ残した表である。台帳がある項目を「縮退」（正典どおりではないが、その違いを引き受ける判断が既に書かれている状態）と判定するときは、その判断がこの表の行か、`doc/emo2-conformance-scope.md` の「旧ロードマップ spec への影響」の表の行のどちらかに実在していなければならない。引き受ける行が無ければ、その項目は縮退ではなく未対応として登記する。

この 2 つの表には、どの行がどのドメインの話なのかを示す欄が無い。そこで、本ドメインが受け持つ 24 ページの項目に触れる行を全数読んで選び出し、その一覧をここに置く。行は**行番号ではなく表の第 1 列（項目名）で指す**——行番号は表に 1 行足されるだけで動くが、項目名は動かないためである。台帳の備考も同じ書き方で行を指しているので、台帳の側から逆に引くこともできる。

### 数え方と、この節の数がいつのものか

下の件数は **2026-09-05 に、その日の表に対して**数えたものである。表は spec が 1 本着地するたびに伸びるので、数はいつ古くなってもおかしくない。数え直せるように、切り出しの手順をここに書いておく。

1. `doc/COMPAT_ARCHITECTURE.md` から見出し「8. 沈黙ルール対応表」を探し、そこから**次の同じ深さの見出しの直前まで**を表の範囲とする（行番号では切らない）。
2. その範囲のうち縦棒で始まる行を取り、区切りの行と見出しの行（第 1 列が「項目」）を除いたものがデータ行である。
3. `doc/emo2-conformance-scope.md` の「6. 旧ロードマップ spec への影響」も同じやり方で切り、見出しの行（第 1 列が「旧 spec」）を除く。

| 数えたもの | 2026-09-05 時点 |
|---|---:|
| 沈黙ルール対応表のデータ行 | 81 |
| うち本ドメインの項目に触れる行 | 44 |
| うち縮退の判断が書かれている行 | 11 |
| 見直し表のデータ行 | 7 |

本 spec の要件は、この表を 80 行、縮退の判断が書かれている行を 16 行としている。数え直した結果はそれと 2 か所で食い違う。⑴ データ行は 81 行である——2026-09-03 に角括弧なしのタグについての行が 1 本足されており、要件が固まったのはその前である（増えた 1 行はさくらスクリプトの話なので、触れる 44 行の中身は変わらない）。⑵ 縮退の判断が書かれている行は 11 行である——数え方は下の「縮退の判断が書かれている 11 行」の節に書いた。要件そのものは本 spec では直さず、この食い違いを「隣接 spec の是正候補」の節へ回す。

### どの行を「触れる」と数えたか

**数えた行**——その行が、本ドメインの 24 ページのいずれかが定めるもの、すなわち定義項目のキー、またはそれらのページが定めるファイル名・フォルダ名の決まりについて、areka がどう読むか・どこまで守るかを決めている行。

**数えなかった行**——さくらスクリプトのタグ、SHIORI のイベントや照会、プロパティ、および areka の内部の記録のしかたや表し方だけを決めている行。これらは他のドメインの台帳が受け持つか、正典の項目に対応するものが無い。

判断に迷う行は、**同じ主題の群でそろえた**。たとえばバルーンのファイル名の決まりを扱う行は、決めている事柄がファイル名の探し方であれ内部での表し方であれ、まとめて数えている——群の中で 1 行だけ外すと、後から同じ主題を引くときに取りこぼすためである。群は下の見出しがそのまま対応する。

次の行は、主題が近いが数えていない。⑴ `\_l` の縦書き座標系（さくらスクリプトのタグであり、正典の写像の登記も実装の担当も別ドメインにある）、⑵ 本仕様が引いた正典参照の出所と、正典側の不安定さ（どの版を正典としたかの記録であり、定義項目の扱いを決めていない）、⑶ 重なり順のグループの保ち方・未指定スコープの参加・グループどうしの前後・相棒窓の畳み込み・タグの拒否・解除タグの余分な語・最小化の連動・記録の語彙（いずれも窓の重なりの仕組みそのものを決めており、シェルの descript に書かれた値の読み方を決めていない）。

### 触れる 44 行

**サーフェスの基準位置（surfaces.txt の `point.basepos.x`／`point.basepos.y`）**（2 行）

- `\![move]` の基準位置 `base`（basepos）解決（縮退の判断あり）
- 宣言 `point.basepos` の実導出（サーフェス個別 basepos）（縮退の判断あり）

**ゴーストの descript の名前のキー（`sakura.name2`・`kero.name`）**（2 行）

- `%selfname2`（descript `sakura.name2`）が未定義のときの値
- `%keroname`（descript `kero.name`）が未定義のときの値

**バルーンのファイルとフォルダの名前の決まり（系列名・旧名・面別の上書き・面 ID）**（6 行）

- バルーン系列の正規名 `balloonp0def{ID}` / `balloonp1def{ID}` を scope 0 / 1 の第一候補として先行探索すること
- 同一 scope を指す語彙が二系統ある事実と、内部表現の正準形
- 装飾族に接尾辞なしの旧名がもう一段存在する事実（縮退の判断あり）
- ID 単位フォールバックで後段接頭辞の面を採用したとき、どの面別上書き層を適用するか
- `\b[ID]` の ID が指す名前空間
- バルーン面 ID 判定の厳格化（本 spec 適用前後で字義上唯一の非同一点）

**ゴーストの descript の初期表示面（`balloon.defaultsurface` 系）**（1 行）

- ghost descript の `balloon.defaultsurface` / `kero.balloon.defaultsurface` / `char*.balloon.defaultsurface` による初期表示面宣言（縮退の判断あり）

**バルーンの descript のバルーン位置（`windowposition.x`／`.y`／`.limit`）**（14 行）

- `windowposition.x` のキーワード指定（`center` / `top` / `bottom`）と `windowposition.limit`
- `windowposition.x` の符号規約（正典が x 方向の基本位置に沈黙しているため実機確定した項目）（縮退の判断あり）
- `windowposition` 調整量の k 適用時の丸めが SSP と 1px 食い違うこと（縮退の判断あり）
- `ScaleRatio::scale_len` の「非ゼロ長は最小 1px」規約を `windowposition` 調整量へ継承したこと
- `windowposition.limit=1` をいつ適用するか（正典は適用時点に沈黙）
- 「画面内」の制限領域はどこか（正典はマルチモニタ／作業領域に沈黙）
- ユーザがバルーンをドラッグしている最中の扱い（正典はユーザ操作に沈黙）
- 補正を作者指定・保存の相対位置へ焼き付けるか（正典は沈黙）
- バルーンが作業領域より大きく両端を同時に収められないとき（正典は沈黙）
- `windowposition.x` のキーワードの大小文字（正典はキーワードを小文字で記すのみで沈黙）
- キーワード指定時に `windowposition.y` などの調整量をどう扱うか（正典は「固定」としか言わず沈黙）
- キーワードの水平中央で中点の端数をどう丸めるか（正典は「中央」としか言わず沈黙）
- シェル寸が後から変わったときにキーワードの中央揃えを追従させるか（正典は寸法変動後の扱いに沈黙）
- ゴースト側 `descript.txt` への `windowposition` 系（`.x` / `.y` / `.limit`）の記載の受理（縮退の判断あり）

**ゴーストとシェルの descript の起動位置（`defaultx` 系）**（1 行）

- 複数スコープの既定 X 連鎖規則（正典が二体以上の既定相対配置に沈黙しているため実機確定した項目）

**バルーンの相対位置（シェルの descript の `balloon.offsetx`／`offsety` と、その追従）**（4 行）

- サーフェス寸変動時のバルーン追従基準（正典は resize 時にバルーン相対をどう保つかに沈黙）
- バルーン位置オフセットの単位空間契約（正典は拡大率と単位空間の関係に沈黙）
- 拡大率遷移で追従オフセットをどう変換するか（正典は拡大率遷移そのものに沈黙）
- バルーンオフセットの保存往復で拡大率をどう扱うか（正典は保存形式に沈黙）

**バルーンの descript の文字組み（`vertical`・`origin`・`validrect`・`wordwrappoint`・`font.*`・`arrow0`／`arrow1`）**（8 行）

- areka 拡張キー `writing_mode` の存在・語彙・正典キー `vertical` との優先順位（正典は areka 独自キーの存在にも、両キー併記時の解決にも沈黙）
- バルーン文字の描画開始点——宣言された `origin` の validrect 外クランプ（areka 独自「origin クランプ正準」）の撤去（本表の別行にあるバルーン窓の画面内維持のクランプ〔`placement/balloon_limit.rs::clamp_axis`〕とは別種のクランプであり、そちらは撤去していない）
- フォント縦書き異体の挙動等価（SSP の `@` フォント機構に対する areka 裁量。正典は「指定フォントの `@` 付き縦書き異体を自動使用し、無ければ環境の標準ゴシックの縦書き異体へ自動差し替え」と定める）（縮退の判断あり）
- 会話中の書字方向切替（正典はサーフェス毎の縦横切替を認めるが、切替時に何が起きるかは「崩れる」としか書かない）（縮退の判断あり）
- 縦書きで列が並ぶ範囲の上限（＝列がどこで打ち止めになりスクロールが始まるか。正典に該当キーも該当文も無い）
- `\f[align]`／`\f[valign]`／下線の縦書き写像（正典の 2 ページで `valign` の写像が逆になっている）（縮退の判断あり）
- `arrow0`／`arrow1` の縦書き再解釈（スクロール方向を示す矢印画像の意味）（縮退の判断あり）
- `origin.y` の既定に縦書きの分岐が無いこと（正典は `origin.x` の既定だけを 2 分岐で書き、`origin.y` には分岐を書いていない）

**シェルの descript の重なり順（`seriko.zorder`）**（6 行）

- スコープ間の重なりを「指定が無ければ非強制・指定があればその指定に従って固定」の二状態にすること（正典は明示指定と既定挙動の関係に沈黙）
- shell 設定の `seriko.zorder` でバルーン込みの明示記法（`balloonN`／`surfaceN`・省略形 `bN`／`sN`）を受理すること（正典は descript 版を「タグの descript 版」とだけ記し、書ける記法の範囲に沈黙）
- shell 設定に `seriko.zorder` が複数行現れたときの扱い（正典は重複記載に沈黙）
- 重なり指定の語の大小文字（正典は語を小文字で記すのみで沈黙）
- shell 設定由来の基底を据えるときに、既にタグ由来のグループが載っていた場合の終状態（正典は起動より後に設定が適用される状況を想定していない）
- 【訂正】shell 設定の `seriko.zorder` を「絵の重ね順（SERIKO のレイヤ順）」とする既存記述

### 縮退の判断が書かれている 11 行

上の 44 行のうち、**正典が定めていることを areka がやっていない**か、**正典の書きぶりや参照実装と食い違う**と自分で書いている行を選んだ。正典が何も書いていない箇所について areka が決めただけの行は、正典との食い違いではないので選んでいない（44 行の大半はこちらである）。

正典が定めることを areka がやっていない、と書いている行（9 行）。

- `\![move]` の基準位置 `base`（basepos）解決
- 宣言 `point.basepos` の実導出（サーフェス個別 basepos）
- 装飾族に接尾辞なしの旧名がもう一段存在する事実
- ghost descript の `balloon.defaultsurface` / `kero.balloon.defaultsurface` / `char*.balloon.defaultsurface` による初期表示面宣言
- ゴースト側 `descript.txt` への `windowposition` 系（`.x` / `.y` / `.limit`）の記載の受理
- フォント縦書き異体の挙動等価（SSP の `@` フォント機構に対する areka 裁量。正典は「指定フォントの `@` 付き縦書き異体を自動使用し、無ければ環境の標準ゴシックの縦書き異体へ自動差し替え」と定める）
- 会話中の書字方向切替（正典はサーフェス毎の縦横切替を認めるが、切替時に何が起きるかは「崩れる」としか書かない）
- `\f[align]`／`\f[valign]`／下線の縦書き写像（正典の 2 ページで `valign` の写像が逆になっている）
- `arrow0`／`arrow1` の縦書き再解釈（スクロール方向を示す矢印画像の意味）

正典の書きぶり、または参照実装 SSP の実際の挙動と食い違うと書いている行（2 行）。

- `windowposition.x` の符号規約（正典が x 方向の基本位置に沈黙しているため実機確定した項目）
- `windowposition` 調整量の k 適用時の丸めが SSP と 1px 食い違うこと

この 11 行が、台帳で縮退と判定するときの転記元になる。台帳の備考には、ここに挙げた項目名でその行を指す。

### もう一方の表（見直し表）の 7 行

`doc/emo2-conformance-scope.md` の「旧ロードマップ spec への影響」の表は、古い計画に対して実際に作る範囲をどこまで縮めたかを spec ごとに 1 行で書いたものである。2026-09-05 時点で 7 行あり、第 1 列は spec の名前である。

- `areka-P0-seriko-runtime`——サーフェスの組み立てと動きを、着せ替えと重ね合わせだけ・間隔の語 3 種・矩形の当たり判定へ縮めた行。本ドメインでは、絵の並べ方・当たり判定・動きの定義の項目がこの行を転記元にしている。
- `areka-P0-shiori-host-32`——同居させる別種のモジュールを外した行。本ドメインに対応する項目は無い。
- `areka-P0-sakura-script`——台詞のタグを全数からごく一部へ縮めた行。さくらスクリプトの台帳が受け持つ。
- `wintf-P0-surface-hierarchy`——絵の重ね合わせを汎用の仕組みではなく重ね順と原点合わせだけにした行。縦書きの受け口はここから外れて先に着地した旨も書かれている。本ドメインでは合成のしかたの語と縦書きの受け口が関わる。
- `wintf-P0-animation-system`——動きの再生をまばたきの 2 語だけにした行。本ドメインでは間隔の語の群が関わる。
- `areka-P0-balloon-loader`——バルーンの定義を必須の欄だけにし、通信まわりを後回しにした行。本ドメインではバルーンの descript の通信まわりの項目が関わる。
- `compat-ghost-integration`——適合を確かめる相手のゴーストを差し替えた行。本ドメインに対応する項目は無い。

この表には「必須フィールド」という語が現れるが、本ドメインの更新ファイルのページにも同じ名前の項目がある。別のものなので、取り違えないこと。

## ライブ確認の結果

### 基準になる 2 つの時点

- スナップショットが作られた時点: 2026-08-24T04:08:57.881Z（`doc/ukadoc-coverage/catalog.toml` の `[snapshot]` の `generated_at`）。ここに収まっている ukadoc の項目は 1,749 件で、そのうち本ドメインの台帳に載る項目について記録されている版番号の最大は 2.8.82 である。カタログ全体で見るとこれより新しい値も入っているが、いずれも他ドメインのページに由来する SSP 以外の版番号である（`list_propertysystem` の `system.os.(キー)` に 5.19.0、`list_sakura_script` の `\![close,websocket,URL]` に 7.4.1）。
- ライブを確かめた日: 2026-09-05。

台帳・世代別対応表・報告に出てくる「正典ではこうなっている」という記述は、断りが無ければ前者（スナップショット）の時点のものである。後者は、その間に正典が動いていないかを限られた範囲で見に行った日付である。

### 見に行った範囲

`https://ssp.shillest.net/ukadoc/manual/<ページ名>.html` の 4 ページだけを取得した。4 ページとも応答は正常だった。

| ページ | ライブで見つかった定義項目の見出し |
|---|---|
| `manual_shell` | 0 件（定義項目を持たないページ） |
| `descript_shell_surfaces` | 137 件 |
| `descript_balloon` | 165 件 |
| `descript_shell` | 102 件 |

本ドメインが受け持つ残り 21 ページはライブを見ていない。これは取りこぼしではなく、要件 1.12 が突き合わせの範囲をこの 4 ページに限ると決めているためである。したがって「他の 21 ページに増えた見出しは無い」とは言えない。以下で言えることは、上の表に挙げたページについてだけである。

### ⑴ 2 つの綴りの確認

areka の中で使われている `surface.append` と `kero.surface.alias` の 2 語について、正典に居場所があるか、綴りはどれが正しいかを確かめた。

**`surface.append`——実在する。綴りはこれで正しい。**

- ページ: `descript_shell_surfaces`。見出し: 解説節「surface*ブレスとsurface.append*ブレス」。同じページの解説節「surfaceブレス記述例」にも記述例と説明がある。
- 実際に書くときは後ろに数字が付く形になる。ページの記述例は `surface.append0-9`・`surface.append1-9,20-29`、説明文は「surface(.append)*の*部分に、サーフェスIDとなる数字を指定。複数指定、範囲指定が可能」と書いている。
- ページの説明文に 1 か所だけ `surace.append` という誤記がある（「用記法surace.appendがある」）。`surface` の綴りが崩れたもので、同じページで正しい綴りは 11 か所、この誤記は 1 か所である。areka 側が写すべきなのは `surface.append` のほうである。

**`kero.surface.alias`——実在する。綴りはこれで正しい。**

- ページ: `descript_shell_surfaces`。見出し: 解説節「surface.aliasブレス記述例」。本文が「相方側の設定はkero.surface.alias、char*.surface.alias（char*はSSPのみ）と書き変える」と書いている。
- 同じ族は 3 つある——本体側が `sakura.surface.alias`、相方側が `kero.surface.alias`、SSP だけが持つ形が `char*.surface.alias`。記述例には `sakura.surface.alias` が載っている。

**2 語に共通する事情。**どちらも「定義項目の見出し」ではなく「解説節」に置かれている。ukadoc の定義項目は、書き出しの語とその説明を組にした形で並んでいて、スナップショットが項目として拾うのはこの形のものだけである。2 語はその形をとっていないため、`doc/ukadoc-coverage/catalog.toml` を検索しても 0 件で、対応する項目 id が無い。

このため、台帳に行を作らないという方針は変わらない（台帳の項目はカタログの id と 1 対 1 で対応していなければならない）。ただし理由は「正典に居場所が無い」ではなく、「正典の解説節には確かにあるが、項目として数えられる形をしていない」である。台帳の備考にはこの区別のほうを書く。

**`manual_shell` には 2 語とも 1 か所も出てこない。**このページはシェルのフォルダに置くファイルの一覧を並べたもので、定義項目の見出しを 1 つも持たない。カタログ側もこのページについてはページ全体を指す項目を 1 つ（`UKADOC Project シェル`）持つだけである。

### ⑵ 見出しの突き合わせ

**対象の選び方。**4 ページのうち版番号が新しい 3 ページを対象にした。カタログに記録されている版番号をページごとに集計すると、最新は `descript_shell_surfaces` が 2.8.82、`descript_balloon` が 2.8.80、`descript_shell` が 2.8.53、`manual_shell` が 2.7.38 である。`manual_shell` だけが 1 段古く、しかも定義項目を持たないページなので対象から外れる。これは要件 1.12 が名指ししている 3 ページと一致する。

**やり方。**目視で見比べるのではなく、両側とも機械で一覧にしてから突き合わせた。

- スナップショット側: `doc/ukadoc-coverage/catalog.toml` からページごとに項目名をすべて取り出す。
- ライブ側: 取得した HTML から、定義項目の書き出しにあたる文字列をすべて取り出す。
- 両方を「同じ文字列が何個あるか」まで含めて突き合わせ、片側にしか無いものを両方向で出す。

**結果。**

| ページ | ライブの見出し | スナップショットの項目 | ライブだけにある | スナップショットだけにある |
|---|---|---|---|---|
| `descript_shell_surfaces` | 137 | 137 | 0 | 0 |
| `descript_balloon` | 165 | 162 | 3 | 0 |
| `descript_shell` | 102 | 102 | 0 | 0 |

増えていたのは `descript_balloon` の 3 件だけで、いずれもページ上で「SSP 2.8.83」と記されている。このページを含む本ドメインのスナップショットは 2.8.82 までなので、版が 1 つ進んだ分がそのまま差になっている。3 件の中身は次の節に挙げる。

減っていたものは 1 件も無い。すなわち、この 3 ページについては、台帳に並んでいる項目がライブでも全部そのまま生きている。

**読み取りの確からしさについて。**この突き合わせで言えることと言えないことを分けて書いておく。

- ライブ側は、要約を経由せずに HTML そのものから拾っている。文章に直してから数え直したわけではないので、言い換えによる取りこぼしは起きない。
- 一方で、拾い方の作り込みが甘いと差が作れてしまう。実際に一度、書き出しの印がちょうど「項目」であるものだけを拾ったために 3 件（`overlayfast`・`overlaymultiply`・`overlayscreen`）を取りこぼし、「スナップショットだけにある 3 件」という偽の差が出た。この 3 件は印が「項目（旧称）」になっていたためで、拾い方を直したら差は 0 件に戻った。上の表は、この直しを入れたあとの数である。
- ライブのページ内リンクの名札と、実際の書き出しの文字が食い違う項目がある（例: 書き出しが `charset,文字コード` なのに名札は `charset,文字コード-surfaces`、書き出しが `bind` の 2 項目の名札は `_1_bind` と `_2_bind`）。突き合わせは名札ではなく書き出しの文字で行っている。スナップショットの項目名も書き出しの文字と同じ形なので、これで両側が揃う。名札で突き合わせると、この食い違いが差として大量に出てしまい、意味を持たない。
- `manual_shell` は定義項目を持たないページなので、差の勘定には入れていない。
- 上に書いたとおり、他の 21 ページは見ていない。この節が保証するのは 3 ページ分である。

## 未収載の候補

スナップショットに入っていないが正典には載っている項目を、ページ名と見出しで控えておく場所である。**ここに挙げたものは台帳に書かない。**台帳の項目名はカタログと 1 文字も違わずに一致していなければならず、追加はスナップショットを新しくする側の手続きで入ってくる。ここは、その更新が来るまで見失わないための控えである。

ライブ確認の結果、次の 3 件が見つかった。いずれも `descript_balloon` の項目で、ページ上に「SSP 2.8.83」と記されている。

| ページ | 見出し | ページに記された版 | 何を決める項目か |
|---|---|---|---|
| `descript_balloon` | `number.x,座標 *1` | SSP 2.8.83 | カウンタ数値の X 座標。`number.xr` と同じもので、左を起点として書きたいときのための別の名前 |
| `descript_balloon` | `number.yb,座標 *1` | SSP 2.8.83 | カウンタ数値の Y 座標。下を起点とする。`number.y` と同じもので、縦書きでは下端が基準になることを明示したいときのための別の名前 |
| `descript_balloon` | `sstpmessage.yb,座標 *1` | SSP 2.8.83 | SSTP メッセージの表示終了位置の Y 座標。縦書きのときだけ使われ、横書きでは無視される |

見出しはページの書き出しをそのまま写している。末尾の `*1` はページ内の注記を指す参照記号で、スナップショット側の項目名にも同じ形で入っている（例: 既にカタログにある `number.xr,座標 *1`）。将来スナップショットが更新されたときに同じ形で並ぶよう、削らずに残してある。

3 件のうち 2 件は、既にカタログにある項目の別の名前である（`number.x` は `number.xr` と同じもの、`number.yb` は `number.y` と同じもの）。残る `sstpmessage.yb` だけが、縦書きのときの表示終了位置という新しい役割を持つ。

この一覧はライブを見た 3 ページに限られる。見ていない 21 ページに未収載の項目があるかどうかは、この調査では分からない。

## 隣接 spec の是正候補

調査の途中で、担当が決まっている spec の説明書や、この調査そのものが前提にしていた記述に、いまの実物と合わないものが見つかった。**この調査はそれらを書き換えない。** 他の spec の文書も、食い違いを引き受ける判断を書いた表も、上流が凍結した台帳の決まりも、直すのは持ち主の仕事である。ここには「何が合っていないか」「どうやって測ったか」「誰が引き取るのが筋か」の 3 つを並べて置く。

担当 spec の説明書の**記述が実物と食い違っている**ものは、当該項目の台帳の備考にも「担当 spec の記述が古い」と 1 行書いてある。この節と台帳のどちらから見ても同じ項目にたどり着ける。説明書がその欄について**何も書いていない**だけのものは記述の誤りではないので、備考にその 1 行は書いていない（下の ⑸ がそれである）。

### 担当 spec の説明書と実物が合わないもの

**⑴ `areka-P0-balloon-canon-residue`——初期表示面の綴りが 2 つしか挙がっていない**

- 何が合っていないか: 説明書の番号 3 の項目は、バルーンの初期表示面を宣言する綴りとして、接頭辞の無い形と相方側の形の 2 つを挙げるが、正典の見出しは 4 つある（本体側の綴りと、キャラクタ番号で指す綴りが加わる）。あわせて、受け持ちの範囲を書く行は「6 項目＋追加登記の 7〜10」と書いているが、番号の付いた項目は 12 まである（11 と 12 は後から足され、範囲の行が追随していない）。
- どうやって測ったか: カタログでゴーストの descript のこの名前を持つ見出しを全数拾ったら 4 件だった。説明書は番号の付いた項目を最後まで読んだ。
- 誰が引き取るか: `areka-P0-balloon-canon-residue`。増えた 2 つの綴りの台帳の備考に 1 行ずつ書いてある。

**⑵ `areka-P0-text-decoration-canon`——書体の欄の数が 1 つ足りない**

- 何が合っていないか: 説明書はバルーンの descript の書体の欄を「基底 13 キー」と 6 か所で書いているが、正典の見出しは 14 種ある。
- どうやって測ったか: カタログでバルーンの descript の `font.` で始まる見出しを全数拾ったら 14 件だった。
- 誰が引き取るか: `areka-P0-text-decoration-canon`。14 件の台帳の備考に 1 行ずつ書いてある。

**⑶ `areka-P0-package-mount`——読む欄の数え上げが実物と食い違う**

- 何が合っていないか: 説明書は読む欄として種別の欄（`type`）を挙げているが、着地したゴーストの descript の読み取りはこの名前を 1 度も引かない。逆に、実際に引いている `sakura.name2` が数え上げから漏れている。
- どうやって測ったか: `crates/areka-parsers` 全体でこの 2 つの綴りを当たった。ゴーストの descript を解く経路に種別の欄を引く行は 1 つも無い（この綴りが出てくるのは、割った表がすべての名前を保つことを示す試験の 1 行だけである）。`sakura.name2` は名前をまとめる型の欄として実際に引かれている。なお、同じモジュールの冒頭の説明文も `sakura.name2` を落としている（この点は「未知の記述の扱い」の節にも書いた）。
- 誰が引き取るか: `areka-P0-package-mount`。`type` と `sakura.name2` の 2 件の台帳の備考に 1 行ずつ書いてある。

**⑷ `areka-P0-emo-atlas`——透過の設定は注ぎ込まれておらず決め打ちである**

- 何が合っていないか: 説明書は、透過をどう扱うかの 2 つの設定をシェルの descript 由来の値として注ぎ込む（絵を焼く側が descript を読みに行かない）と書いている。着地した経路は層こそ分かれているが、渡している値は決め打ちである。
- どうやって測ったか: 絵を焼く側と、窓の寸法を測る側の両方が同じ固定値を書いている。もう一方の設定（透過部分を黒で塗るかどうか）の綴りは、リポジトリの Rust に 1 行も現れない。
- 誰が引き取るか: `areka-P0-emo-atlas`。シェル側の 2 件の台帳の備考に 1 行ずつ書いてある。

**⑸ `areka-P0-charset-canon`——いくつかの文字コードの欄に説明書が沈黙している**

- 何が合っていないか: 説明書が範囲外として名前を挙げているのは、配置指示・説明文・サーフェスの名前表の文字コードだけである。ゴースト・シェル・バルーンの descript の `charset` と、ゴーストの descript の逃がし方の宣言（`shiori.escape_unknown`）は、範囲にも範囲外にも挙がっていない。プラグインとヘッドラインの descript の `charset` も同じである。前の 3 つは現状を並べた表に「対応済み」として出てくるが、正典の項目を誰が持つのかは書かれていない。
- どうやって測ったか: 説明書を全文読み、範囲と範囲外の欄に並ぶ名前を 1 つずつ当たった。
- 誰が引き取るか: `areka-P0-charset-canon`（自分の範囲に入れるか、対象外と書くかを決めてほしい）。**これは記述の誤りではなく沈黙なので、台帳の備考には「担当 spec の記述が古い」とは書いていない。** 担当の欄は空のままにしてある。

**⑹ `areka-P0-scope-chain-gap`——「意味論不変」が確かめているのは隣り合う位置関係だけである**

- 何が合っていないか: 判断の表の「複数スコープの既定 X 連鎖規則」の行は、`defaultx` を連鎖の基準からの左向きのずらし量（0 なら基準に密着）として扱うことを、正典の言う意味を変えていない（表の言い方では「意味論不変」）と決めている。その土台は参照実装を相手に取った実機の計測で、隣り合うスコープどうしの位置関係については誤差 0 まで詰められている。しかし正典は、本体側の `defaultx` を「画像を基準にした X 座標」（宣言が無ければ画像の中央）と定め、本体側の `defaultleft` を「画面の上での X 座標」と、**別の基準点**として定めている。areka はこの 2 つを 1 つの欄へ畳んで先に書いてある方を採るので、基準点の違いは残らない。つまり「意味論不変」は**スコープどうしの相対位置については確かめられている**が、**`defaultx` を単独で宣言したときの基準点が正典どおりか**は確かめられていない。この 2 つは別の主張である。
- どうやって測ったか: 判断の表の当該行を全文読み、正典の当該 2 項目の本文を読み直した（2026-09-06）。areka 側は、窓の置き方を解く関数が 2 つの綴りを同じ 1 つの欄へ写していることを読んで確かめた。
- 誰が引き取るか: `areka-P0-scope-chain-gap`。台帳はこの綴りの 6 件（本体側・相方側・キャラクタ番号で指す形が、ゴーストとシェルの両方のページにある）を未対応として登記し、備考で「この行が答えているのは別の問いである」と書き分けてある。行の言い分と台帳の言い分が割れて見える状態なので、実機の計測を「単独で宣言したときの絶対位置」でも取り直すか、当該行に「基準点については未確認」と補うかを決めてほしい。

### 引き受け先が決まっていないもの

担当が決まらない項目の担当の欄は空のままにし、割り当ては統合担当に委ねる決まりである。そのうち、**近いところに居る spec がどれも自分のものだと書いていない**ために宙に浮いている 4 つを、名指しで挙げておく。

- **貼り付き（シェルの descript の `seriko.sticky-window`）**——重なり順を持つ spec は範囲を重なり順の読み口までとし、この欄を範囲にも範囲外にも挙げていない。窓の置き方を建てた spec は「重なり順と貼り付きは受け口だけ」と書くだけで、後を引き受ける先を書いていない。台帳の状態は「語彙のみ」である。
- **バルーンのずらし量の片方だけの宣言**——areka は横と縦の**両方**が書かれているときだけ調整量を作るので、片方だけの宣言は何も言わずに落ちる（窓の置き方を解く関数が 2 つを組にしてから初めて値にする）。正典は横の座標だけでも意味を持つ欄として定めている。判断の表にこのキーを名指しする行は 2 つあるが、どちらも別の問い（キーワードで位置を決めたときに調整量を加え続けるか・作者が書いた値をどの拡大率で物理の画素へ換算するか）に答えている。**この落ち方そのものを引き受ける行は、どちらの表にも無い。**
- **バルーンの左右の寄せ（シェルの descript の `sakura.balloon.alignment`／`kero.balloon.alignment`）**——実装済みだが担当の欄が空である。窓の置き方を建てた spec の説明書はこの欄について「この単位は記録のみ・バルーン配置の後続が持つ」と書き、その後続を名指ししていない。実際に値を読んでいるのは同じ spec の成果物である。
- **`areka-P0-text-decoration-canon` が自分の外に置いた 14 件**——バルーンの descript の通信欄・カウンタ・SSTP メッセージの書体の欄である。同 spec の説明書は「機能ごとに後から本 spec の基盤へ乗る」と書いており、宣言としては筋が通っているので誤りではない。ただし、この 14 件を自分のものと書いている spec はいまのところ 1 本も無い。

### 上流の決まりと本 spec 自身の文書へ回すもの

上流が凍結した台帳の決まりと、本 spec の要件は、この調査では**変えない**。実物と合わないと分かった箇所を、変えずにここへ書き出す。設計は本 spec 自身の文書なので、持ち主が完了までに引き直せばよい。

**⑺ 要件が前提にしている表の行数と内訳が古い。** 数え直した結果と、その数え方の手順は「沈黙ルール対応表の一覧」の節に書いた。要件が言う「縮退の判断が書かれている行」の数だけは、字面で数えても判断で数えても、語を緩めても、どの数え方でも再現できなかった。**その数の出所は分からないままにしてある**（もっともらしい理由を付けない）。

**⑻ 要件と設計が、台帳の備考の書き方で食い違っている。** 要件は当たり判定の読み飛ばしを台帳の備考へ「file:line 付きで」書けと言うが、設計と、同じ日に 4 つの台帳で揃えた流儀は、**台帳の備考にソースの行番号を書かない**（行番号は整理で動き、備考が黙って古びるため）。台帳はクレート名・モジュール名・定義名で指す形で書いた。要件のこの一文は、台帳ではなくこの文書（file:line の根拠を求めているのはこちらである）に当たるものとして読むのが自然である。

**⑼ 要件と設計が挙げるソースの行番号のうち、いくつかは本 spec 自身の書き込みで動いた。** 転記層で唯一の警告段が出る場所として要件と設計が挙げる行は、正典 URL の行をその上に足したぶん下へ動いた。動いた先は「未知の記述の扱い」の節に書いてある。アニメーションの間隔語の転記側の範囲も、同じ理由で終わりが 1 行伸びた。**行番号は、自分が同じファイルへ 1 行足しただけで古くなる。**

**⑽ 間隔語の行の範囲が、要件と設計で違い、どちらも実物と完全には合わない。** 転記側については、要件が挙げる範囲が正典 URL を足す前の実物とちょうど一致し、設計が挙げる範囲は実物と重ならない。駆動側については、振り分けは要件が書く終わりより 1 行先まで続いており、設計が挙げる範囲のほうが実物と一致する。実物の行はこの文書の世代別対応表の注記が挙げている。

**⑾ 設計の「URL を置く先」の表が、実際に置いたものと合わない。** ファイルごとの見込みの数は、表の 7 行のうち 6 行で実際と違う。さらに、実際に 1 行置いたファイル（シェルの表示名を解くところ）が表に載っていない。シェルの着せ替えの欄について「定数の並びの直上にまとめて置く」と書かれた指示も実物と違う——置いたのは値を引く行の直上で、置いた数も指示より少ない。置いた総数は、設計が上限として書いた数には収まっている。

**⑿ 設計の担当の一覧に 4 本足りない。** 台帳で実際に使った担当のうち、`areka-P0-window-placement`・`areka-P0-mayuna-compose`・`areka-P0-emo-atlas`・`areka-P0-ghost-setup` の 4 本が設計の表に無い。いずれも当該 spec の説明書（1 本は設計）が当該項目を自分のものと書いているのを読んで入れた。要件の書き出しは「少なくとも次の担当を取り込む」なので、上限ではない。

**⒀ 設計の「正典に居場所が無い areka の語」の表が、2 語の居場所を取り違えて読める。** 表は追記の記法と相方側の別名の「書く先」を、ファイルの置き場所を並べるページの備考としている。書く先の指定はそのままでよいが、**この 2 語が正典の説明文に現れるのは surfaces.txt のページのほう**であり、置き場所を並べるページには 1 度も現れない。いまの書き方は「置き場所を並べるページに載っている語」と読める。台帳の備考には実際の居場所を書いた。なお「カタログの項目としては見出しにも本文にも無い」という結論そのものは正しく、台帳に行を作らないという判断は動かない。

**⒁ 設計の「未知の記述の扱い」の表の根拠 2 か所が、測り直しと食い違う。** 訂正の中身は「設計の表から直した 2 か所」の節に書いた。結論はどちらも動かないが、根拠の書き方が正しくない。

**⒂ 設計の「機械で決まる件数」の見込みが 2 つ外れている。** ⒜ 絵の重ね方の語を「名前が解ける／解けない」の 2 通りに割ると見込んでいたが、実際の仕訳は 4 通りになった（実装済み・別名・語彙のみ・未対応）。⒝ 窓の置き方を組み立てる関数を「本番の経路から呼ばれていない足場」と書いているが、実際は本番の経路から呼ばれている——起動の入口が窓の準備を呼び、その中でこの関数が呼ばれ、返した値を配置の解決が使う。**ソース側にも古い注記が 2 つ残っている**——この関数と、ゴーストの表示名をまとめる型に付いた「後の作業が結線するまで使われない」という趣旨の注記で、どちらも実際には本番のコードから使われている。この調査が `crates/` に足してよいのは正典 URL の 1 行だけなので、注記は直していない。

**⒃ 優先度の作り方が、コミットした成果物のどこにも書いていない。** 台帳の優先度は「段階 1 文字＋数値」の形で、**数値は本ドメイン全体を通した束の順位（1 から 80）**、**段階はその順位を 5 つに等分したもの（16 ずつ）**である。この作り方は作業用の控えにしか無いので、いまのままでは統合担当が同じ順位を作り直せない。あわせて、上流の説明書は数値を「同じ段階の中での並び」と書いているが、本台帳の数値は段階をまたいだ通し番号である（段階 B の項目の数値は 17 から始まる）。上流の説明書は変えないので、ここに記録する。段階の最終順序を決めないことは台帳の冒頭に書いてある。
