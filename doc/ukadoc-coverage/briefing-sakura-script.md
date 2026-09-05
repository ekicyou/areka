# さくらスクリプトのブリーフィング

正典（SSP 公式仕様書）のさくらスクリプトのページにある 342 項目について、areka が今どこまで
実現しているかを調べた結果を、台帳の数字ではなく言葉で読めるようにした文書である。
台帳そのものは `doc/ukadoc-coverage/ledger/sakura-script.toml`、状態の分布は
`doc/ukadoc-coverage/report/sakura-script.md` にある。

全体は 7 つの節でできている。

| 節 | 内容 |
| --- | --- |
| ⑴ | 書式群と新旧の一覧 |
| ⑵ | `\![...]` の消費側の名前の表 |
| ⑶ | 担当の突合表 |
| ⑷ | 担当なし・裁定待ちの一覧 |
| ⑸ | 「書いてあるのに何も起きない」順の未対応一覧 |
| ⑹ | 既存の brief と `doc/COMPAT_ARCHITECTURE.md` §8 への是正候補 |
| ⑺ | カタログとの件数の差 |

この文書に出てくる数は、すべて台帳と作業用の中間の表から引き直せる。
どの表からどう作ったかは節ごとに書いた。

---

## ⑴ 書式群と新旧の一覧

### 名前のまとめ方

同じ機能に複数の書式があるかどうかを見るには、まず「どこまでを 1 つの名前とみなすか」を
決めなければならない。この調査は次の 1 文で決めた。

> 見出しの先頭にある 1 つ目のタグだけを見て、`\![` で始まるものは命令名（第 1 引数）と
> 選択子（第 2 引数）までを名前とし、`%` で始まるものは最初の `[` の直前までを名前とし、
> それ以外のタグは `\` に続く先頭の下線の連なり（多くて 2 つ）と 1 文字を名前とする。
> 丸括弧から後ろは名前に含めない。

補足が 3 つある。

- 「1 つ目のタグだけ」は、見出しが 2 つの綴りを並べている 8 件と、完全形を続けて連ねている
  2 件の両方に効く。
- 角括弧を使わないタグの切れ目は、areka の字句解析が採っている切れ目と同じ場所に来る。
  正典の名前の切れ目と areka の切れ目が食い違わない。
- 丸括弧を落とすので、見出しの `biff(` と `updatebymyself(` は命令名 `biff`・`updatebymyself`
  として数える。

### この規則から出る数

| 量 | 値 |
| --- | ---: |
| 項目の総数 | 342（うちタグでない見出し 1） |
| 異なる名前 | **259**（`\![...]` の組 183 ＋ `%` 28 ＋ それ以外のタグ 48） |
| 2 件以上を持つ名前 | **23 群・105 件** |
| `\![...]` の命令名だけ | 52 名・198 件 |
| `\![...]` の命令名と選択子の組 | 183 組 |

### 2 件以上を持つ 23 群の内訳

23 群は 3 つに割れる。

| 種類 | 群の数 | 項目の数 | 別名にしてよいか |
| --- | ---: | ---: | --- |
| 引数の数だけが違う | 9 | 31 | 短い側を別名にする |
| 括弧なし形と括弧形の対 | 3 | 6 | 括弧なし形を別名にする |
| 引数の値が違うだけの兄弟 | 11 | 68 | 別名にしない（それぞれが自分の状態を持つ） |
| 合計 | 23 | 105 | |

群ごとに並べると次のとおり。

| 名前 | 種類 | 項目の数 | うち別名になった数 |
| --- | --- | ---: | ---: |
| `\f` | 引数の値が違うだけの兄弟 | 43 | 0 |
| `\_b` | 引数の数だけが違う | 6 | 2 |
| `\q` | 引数の数だけが違う | 6 | 2 |
| `\c` | 引数の数だけが違う | 5 | 2 |
| `\![open,dialog]` | 引数の値が違うだけの兄弟 | 4 | 0 |
| `\![execute,http-get]` | 引数の数だけが違う | 3 | 1 |
| `\![set,scaling]` | 引数の数だけが違う | 3 | 1 |
| `\![set,windowstate]` | 引数の値が違うだけの兄弟 | 3 | 0 |
| `\_a` | 引数の値が違うだけの兄弟 | 3 | 1 |
| `\n` | 引数の値が違うだけの兄弟 | 3 | 0 |
| `\![anim,add]` | 引数の値が違うだけの兄弟 | 2 | 0 |
| `\![execute,http-post]` | 引数の数だけが違う | 2 | 0 |
| `\![execute,install]` | 引数の値が違うだけの兄弟 | 2 | 0 |
| `\![reload,descript]` | 引数の数だけが違う | 2 | 1 |
| `\![set,alignmenttodesktop]` | 引数の値が違うだけの兄弟 | 2 | 0 |
| `\![set,autoscroll]` | 引数の値が違うだけの兄弟 | 2 | 0 |
| `\__w` | 引数の値が違うだけの兄弟 | 2 | 0 |
| `\_s` | 引数の数だけが違う | 2 | 1 |
| `\b` | 括弧なし形と括弧形の対 | 2 | 1 |
| `\i` | 引数の値が違うだけの兄弟 | 2 | 0 |
| `\p` | 括弧なし形と括弧形の対 | 2 | 1 |
| `\s` | 括弧なし形と括弧形の対 | 2 | 1 |
| `\x` | 引数の数だけが違う | 2 | 1 |

別名にするかどうかは群ではなく 2 つの項目の対に当てている。だから「引数の値が違うだけの兄弟」
の群にも別名が 1 件だけ現れる（`\_a[ID]` は `\_a[ID,r2,r3...]` の引数を減らした形なので
対の判定が当たる）。逆に「引数の数だけが違う」群でも、置き場所の語が違って先頭から一致しない対は
別名にならない。

### 正典 1 つと別名の対応

別名は **20 件**ある。台帳で `status` が `alias` の項目がそのまま別名で、指す先は必ず
別名でない項目（鎖の根）にしてある。

向きを決めた根拠は 3 つのどれかである。

- **⑴ 本文の注記**……正典の本文が旧い書式であることと置き換え先を書いている
- **⑵ 版番号**……片側にだけ登場した版が付いていて、付いている側が新しいと決まる
- **⑶ 人手の判断**……本文にも版番号にも決め手が無く、引数の広さや正典の並びを見て人が決めた

| 別名の綴り | 別名の項目 id | 正典の綴り | 正典の項目 id | 向きを決めた根拠 | 決まった順 |
| --- | --- | --- | --- | --- | --- |
| `\7` | `ukadoc:list_sakura_script:_5c7:1` | `\![executesntp]` | `ukadoc:list_sakura_script:_5c_21_5bexecutesntp_5d:1` | ⑶ 人手の判断 | 順 6 |
| `\![execute,http-get,URL]` | `ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-get_2cURL_5d:1` | `\![execute,http-get,URL,パラメータ]` | `ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-get_2cURL_2c_30d1_30e9_30e1_30fc_30bf_5d:1` | ⑶ 人手の判断 | 順 5 |
| `\![reload,descript]` | `ukadoc:list_sakura_script:_5c_21_5breload_2cdescript_5d:1` | `\![reload,descript,パラメータ]` | `ukadoc:list_sakura_script:_5c_21_5breload_2cdescript_2c_30d1_30e9_30e1_30fc_30bf_5d:1` | ⑶ 人手の判断 | 順 5 |
| `\![reloadsurface]` | `ukadoc:list_sakura_script:_5c_21_5breloadsurface_5d:1` | `\![reload,shell]` | `ukadoc:list_sakura_script:_5c_21_5breload_2cshell_5d:1` | ⑴ 本文の注記 | 順 1 |
| `\![set,scaling,横倍率,縦倍率]` | `ukadoc:list_sakura_script:_5c_21_5bset_2cscaling_2c_6a2a_500d_7387_2c_7e26_500d_7387_5d:1` | `\![set,scaling,横倍率,縦倍率,オプション]` | `ukadoc:list_sakura_script:_5c_21_5bset_2cscaling_2c_6a2a_500d_7387_2c_7e26_500d_7387_2c_30aa_30d7_30b7_30e7_30f3_5d:1` | ⑶ 人手の判断 | 順 5 |
| `\_V` | `ukadoc:list_sakura_script:_5c_V:1` | `\![sound,wait]` | `ukadoc:list_sakura_script:_5c_21_5bsound_2cwait_5d:1` | ⑶ 人手の判断 | 順 6 |
| `\_a[ID]` | `ukadoc:list_sakura_script:_5c_a_5bID_5d:1` | `\_a[ID,r2,r3...]` | `ukadoc:list_sakura_script:_5c_a_5bID_2cr2_2cr3..._5d:1` | ⑶ 人手の判断 | 順 5 |
| `\_b[ファイルパス,inline]` | `ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cinline_5d:1` | `\_b[ファイルパス,inline,opaque]` | `ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cinline_2copaque_5d:1` | ⑶ 人手の判断 | 順 5 |
| `\_b[ファイルパス,x,y]` | `ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cx_2cy_5d:1` | `\_b[ファイルパス,x,y,opaque]` | `ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cx_2cy_2copaque_5d:1` | ⑶ 人手の判断 | 順 5 |
| `\_s` | `ukadoc:list_sakura_script:_5c_s:1` | `\_s[ID1,ID2,ID3...]` | `ukadoc:list_sakura_script:_5c_s_5bID1_2cID2_2cID3..._5d:1` | ⑶ 人手の判断 | 順 5 |
| `\_v[ファイル名]` | `ukadoc:list_sakura_script:_5c_v_5b_30d5_30a1_30a4_30eb_540d_5d:1` | `\![sound,play,ファイル名,オプション...]` | `ukadoc:list_sakura_script:_5c_21_5bsound_2cplay_2c_30d5_30a1_30a4_30eb_540d_2c_30aa_30d7_30b7_30e7_30f3..._5d:1` | ⑶ 人手の判断 | 順 6 |
| `\bID番号` | `ukadoc:list_sakura_script:_5cbID_756a_53f7:1` | `\b[ID番号]` | `ukadoc:list_sakura_script:_5cb_5bID_756a_53f7_5d:1` | ⑵ 版番号 | 順 4 |
| `\c[char,数値]` | `ukadoc:list_sakura_script:_5cc_5bchar_2c_6570_5024_5d:1` | `\c[char,数値,開始位置]` | `ukadoc:list_sakura_script:_5cc_5bchar_2c_6570_5024_2c_958b_59cb_4f4d_7f6e_5d:1` | ⑶ 人手の判断 | 順 5 |
| `\c[line,数値]` | `ukadoc:list_sakura_script:_5cc_5bline_2c_6570_5024_5d:1` | `\c[line,数値,開始位置]` | `ukadoc:list_sakura_script:_5cc_5bline_2c_6570_5024_2c_958b_59cb_4f4d_7f6e_5d:1` | ⑶ 人手の判断 | 順 5 |
| `\pID番号` | `ukadoc:list_sakura_script:_5cpID_756a_53f7:1` | `\p[ID番号]` | `ukadoc:list_sakura_script:_5cp_5bID_756a_53f7_5d:1` | ⑶ 人手の判断 | 順 4 |
| `\q[ID][タイトル]または\q*[ID][タイトル]` | `ukadoc:list_sakura_script:_5cq_5bID_5d_5b_30bf_30a4_30c8_30eb_5d_307e_305f_306f_5cq_2a_5bID_5d_5b_30bf_30a4_30c8_30eb_5d:1` | `\q[タイトル,ID,r2,r3...]` | `ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cID_2cr2_2cr3..._5d:1` | ⑴ 本文の注記 | 順 1 |
| `\q[タイトル,ID]` | `ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cID_5d:1` | `\q[タイトル,ID,r2,r3...]` | `ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cID_2cr2_2cr3..._5d:1` | ⑶ 人手の判断 | 順 5 |
| `\sID番号` | `ukadoc:list_sakura_script:_5csID_756a_53f7:1` | `\s[ID番号]` | `ukadoc:list_sakura_script:_5cs_5bID_756a_53f7_5d:1` | ⑶ 人手の判断 | 順 4 |
| `\x` | `ukadoc:list_sakura_script:_5cx:1` | `\x[noclear]` | `ukadoc:list_sakura_script:_5cx_5bnoclear_5d:1` | ⑶ 人手の判断 | 順 5 |
| `\z` | `ukadoc:list_sakura_script:_5cz:1` | `\e` | `ukadoc:list_sakura_script:_5ce:1` | ⑴ 本文の注記 | 順 1 |

根拠で数えると **本文の注記 3 件・版番号 1 件・人手の判断 16 件**。
決まった順で数えると **順 1 が 3 件・順 4 が 3 件・順 5 が 11 件・順 6 が 3 件**である。

| 順 | 何で決まるか | 件数 |
| ---: | --- | ---: |
| 1 | 本文の注記が旧い書式だと述べ、置き換え先も本文から 1 つに定まる | 3 |
| 4 | 括弧なし形と括弧形の 3 対 | 3 |
| 5 | 引数の並びが短い側が長い側の先頭と 1 語ずつ一致する | 11 |
| 6 | 本文が「同じ機能」と言い切っていて、向きだけを人が決めた | 3 |
| 合計 | | 20 |

**この件数は 2026-09-05 の見直しで 17 件から 20 件に変わった。** 見直しの前は順 1・順 4・順 5 の
3 つしか数えておらず、本文が「同じ機能」と言い切っている 3 組（下の表）が抜けていた。

| 別名 | 正典 | 向きを決めた根拠 |
| --- | --- | --- |
| `\7` | `\![executesntp]` | ⑶ 人手の判断 |
| `\_V` | `\![sound,wait]` | ⑶ 人手の判断 |
| `\_v[ファイル名]` | `\![sound,play,ファイル名,オプション...]` | ⑶ 人手の判断 |

この 3 組は本文にも版番号にも向きの決め手が無い——どちらの側にも「旧仕様」の札が付いておらず、
カタログの版番号も両側とも空である。そこで正典の文書の並び（角括弧を使わない短い綴りが
`\![...]` の族より前に置かれている）を裏付けにして、短い綴りを旧い側と人が決めた。

### 決めなかったこと

`\![anim,stop,ID]` と `\![anim,clear,ID]` は、本文が同等であることを言い切っている。
それでも**別名にしなかった**。どちらが新しくどちらが旧いかを決める材料が本文にも版番号にも
無く、正典の並びも決め手にならなかったためである。憶測で向きを付けるかわりに、
2 つを「同じ機能」の関連で結んだだけにしてある。**決めなかったことを、決めなかったと書いておく。**

### 見出しが 2 つの綴りを並べている 8 件

正典の見出しには、2 つの綴りを「もしくは」「または」で並べているものがある。
名前のまとめ方は「1 つ目のタグだけ」を見るので、並べられた側の綴りは名前に入らない。
**並べられた側は正典にも独立した項目 id を持たない**ので、台帳にもその行は無い。

| 項目の綴り（見出し） | 並べられたもう 1 つの綴り | もう 1 つの綴りの項目 id |
| --- | --- | --- |
| `\0もしくは\h` | `\h` | 無い |
| `\1もしくは\u` | `\u` | 無い |
| `\q[ID][タイトル]または\q*[ID][タイトル]` | `\q*[ID][タイトル]` | 無い |
| `\f[anchorcolor,色指定]もしくは\f[anchorbrushcolor,色指定]` | `\f[anchorbrushcolor,色指定]` | 無い |
| `\f[anchornotselectcolor,色指定]もしくは\f[anchornotselectbrushcolor,色指定]` | `\f[anchornotselectbrushcolor,色指定]` | 無い |
| `\f[anchorvisitedcolor,色指定]もしくは\f[anchorvisitedbrushcolor,色指定]` | `\f[anchorvisitedbrushcolor,色指定]` | 無い |
| `\f[cursorcolor,色指定]もしくは\f[cursorbrushcolor,色指定]` | `\f[cursorbrushcolor,色指定]` | 無い |
| `\f[cursornotselectcolor,色指定]もしくは\f[cursornotselectbrushcolor,色指定]` | `\f[cursornotselectbrushcolor,色指定]` | 無い |

8 件のうち 5 件は `\f` の色指定である。areka から見ると、並べられた 2 つの綴りは
1 つの項目として扱えばよい、ということになる。

---

## ⑵ `\![...]` の消費側の名前の表

`\![...]` の 198 件は、areka の中では 1 本の運び役に載って流れる。運び役は名前をそのまま持ち、
受け手の側が「自分あての名前か」を見て拾う。だから「どの名前に受け手がいるか」が分かれば、
次に受け手を 1 つ作ったときに何本のタグが一度に動くようになるかが読める。

名前の粒度は 2 段ある。第 1 段は命令名（第 1 引数）だけ、第 2 段は選択子（第 2 引数）まで含めた組
である。受け手は組で選ぶことがあるので、2 つの表を突き合わせて読む。

### 運び役は通るが誰も消費しない名前

| 量 | 全体 | 消費される | **誰も消費しない** |
| --- | ---: | ---: | ---: |
| 命令名 | 52 | 4 | **48** |
| 命令名と選択子の組 | 183 | 4 | **179** |
| 項目 | 198 | 4 | **194** |

消費されている経路は 4 つだけである。

| 消費される組 | 受け手（定義の名前） | 属する項目の数 |
| --- | --- | ---: |
| `bind,カテゴリ名` | `handle_message` | 1 |
| `move` | `MoveCueSink::emit` | 1 |
| `reset,zorder` | `ZOrderCueSink::emit` | 1 |
| `set,zorder` | `ZOrderCueSink::emit` | 1 |

運び役を開ける場所は areka の中に 5 つあるが、5 つ目は areka 自身の内部の名前を選ぶための
もので、さくらスクリプトの名前ではない。だから経路は 4 つと数えている。

### 表 A: 命令名ごと（52 行・全数）

| 命令名 | 属する項目の数 | 選択子の異なり | 消費される組 | 受け手（定義の名前） | 消費されない項目の数 |
| --- | ---: | ---: | --- | --- | ---: |
| `*` | 1 | 1 | — | — | 1 |
| `anim` | 7 | 6 | — | — | 7 |
| `biff` | 1 | 1 | — | — | 1 |
| `bind` | 1 | 1 | `bind,カテゴリ名` | `handle_message` | 0 |
| `bind-noevent` | 1 | 1 | — | — | 1 |
| `call` | 1 | 1 | — | — | 1 |
| `cancel` | 2 | 2 | — | — | 2 |
| `change` | 3 | 3 | — | — | 3 |
| `close` | 5 | 5 | — | — | 5 |
| `create` | 1 | 1 | — | — | 1 |
| `effect` | 1 | 1 | — | — | 1 |
| `effect2` | 1 | 1 | — | — | 1 |
| `embed` | 1 | 1 | — | — | 1 |
| `enter` | 6 | 6 | — | — | 6 |
| `execute` | 26 | 22 | — | — | 26 |
| `executesntp` | 1 | 1 | — | — | 1 |
| `filter` | 2 | 2 | — | — | 2 |
| `get` | 1 | 1 | — | — | 1 |
| `leave` | 6 | 6 | — | — | 6 |
| `load` | 2 | 2 | — | — | 2 |
| `lock` | 3 | 3 | — | — | 3 |
| `move` | 1 | 1 | `move` | `MoveCueSink::emit` | 0 |
| `moveasync` | 1 | 1 | — | — | 1 |
| `notify` | 1 | 1 | — | — | 1 |
| `notifyother` | 1 | 1 | — | — | 1 |
| `notifyplugin` | 1 | 1 | — | — | 1 |
| `open` | 41 | 38 | — | — | 41 |
| `quicksection` | 2 | 2 | — | — | 2 |
| `raise` | 1 | 1 | — | — | 1 |
| `raiseother` | 1 | 1 | — | — | 1 |
| `raiseplugin` | 1 | 1 | — | — | 1 |
| `reload` | 8 | 7 | — | — | 8 |
| `reloadsurface` | 1 | 1 | — | — | 1 |
| `reset` | 4 | 4 | `reset,zorder` | `ZOrderCueSink::emit` | 3 |
| `restore` | 1 | 1 | — | — | 1 |
| `save` | 1 | 1 | — | — | 1 |
| `send` | 2 | 2 | — | — | 2 |
| `set` | 31 | 25 | `set,zorder` | `ZOrderCueSink::emit` | 30 |
| `sound` | 9 | 9 | — | — | 9 |
| `timernotify` | 1 | 1 | — | — | 1 |
| `timernotifyother` | 1 | 1 | — | — | 1 |
| `timernotifyplugin` | 1 | 1 | — | — | 1 |
| `timerraise` | 1 | 1 | — | — | 1 |
| `timerraiseother` | 1 | 1 | — | — | 1 |
| `timerraiseplugin` | 1 | 1 | — | — | 1 |
| `unload` | 2 | 2 | — | — | 2 |
| `unlock` | 3 | 3 | — | — | 3 |
| `update` | 2 | 2 | — | — | 2 |
| `updatebymyself` | 1 | 1 | — | — | 1 |
| `updateother` | 1 | 1 | — | — | 1 |
| `vanishbymyself` | 1 | 1 | — | — | 1 |
| `wait` | 1 | 1 | — | — | 1 |

### 表 B: 命令名と選択子の組（151 行）

表 A のうち**選択子が 2 つ以上ある 20 名だけ**を組へ開いたものである。
選択子が 1 つしかない 32 名は、表 A の行がそのまま組なので開いていない（開いても分かることが増えない）。

| 命令名と選択子 | 属する項目の数 | 消費の有無 | 受け手（定義の名前） |
| --- | ---: | --- | --- |
| `anim,add` | 2 | 消費されない | — |
| `anim,clear` | 1 | 消費されない | — |
| `anim,offset` | 1 | 消費されない | — |
| `anim,pause` | 1 | 消費されない | — |
| `anim,resume` | 1 | 消費されない | — |
| `anim,stop` | 1 | 消費されない | — |
| `cancel,http` | 1 | 消費されない | — |
| `cancel,websocket` | 1 | 消費されない | — |
| `change,balloon` | 1 | 消費されない | — |
| `change,ghost` | 1 | 消費されない | — |
| `change,shell` | 1 | 消費されない | — |
| `close,communicatebox` | 1 | 消費されない | — |
| `close,dialog` | 1 | 消費されない | — |
| `close,inputbox` | 1 | 消費されない | — |
| `close,teachbox` | 1 | 消費されない | — |
| `close,websocket` | 1 | 消費されない | — |
| `enter,collisionmode` | 1 | 消費されない | — |
| `enter,inductionmode` | 1 | 消費されない | — |
| `enter,nouserbreakmode` | 1 | 消費されない | — |
| `enter,onlinemode` | 1 | 消費されない | — |
| `enter,passivemode` | 1 | 消費されない | — |
| `enter,selectmode` | 1 | 消費されない | — |
| `execute,compressarchive` | 1 | 消費されない | — |
| `execute,createnar` | 1 | 消費されない | — |
| `execute,createupdatedata` | 1 | 消費されない | — |
| `execute,dumpsurface` | 1 | 消費されない | — |
| `execute,emptyrecyclebin` | 1 | 消費されない | — |
| `execute,extractarchive` | 1 | 消費されない | — |
| `execute,headline` | 1 | 消費されない | — |
| `execute,http-delete` | 1 | 消費されない | — |
| `execute,http-get` | 3 | 消費されない | — |
| `execute,http-head` | 1 | 消費されない | — |
| `execute,http-options` | 1 | 消費されない | — |
| `execute,http-patch` | 1 | 消費されない | — |
| `execute,http-post` | 2 | 消費されない | — |
| `execute,http-put` | 1 | 消費されない | — |
| `execute,install` | 2 | 消費されない | — |
| `execute,nslookup` | 1 | 消費されない | — |
| `execute,ping` | 1 | 消費されない | — |
| `execute,resetballoonpos` | 1 | 消費されない | — |
| `execute,resetwindowpos` | 1 | 消費されない | — |
| `execute,rss-get` | 1 | 消費されない | — |
| `execute,rss-post` | 1 | 消費されない | — |
| `execute,websocket` | 1 | 消費されない | — |
| `filter` | 1 | 消費されない | — |
| `filter,プラグイン名` | 1 | 消費されない | — |
| `leave,collisionmode` | 1 | 消費されない | — |
| `leave,inductionmode` | 1 | 消費されない | — |
| `leave,nouserbreakmode` | 1 | 消費されない | — |
| `leave,onlinemode` | 1 | 消費されない | — |
| `leave,passivemode` | 1 | 消費されない | — |
| `leave,selectmode` | 1 | 消費されない | — |
| `load,makoto` | 1 | 消費されない | — |
| `load,shiori` | 1 | 消費されない | — |
| `lock,balloonmove` | 1 | 消費されない | — |
| `lock,balloonrepaint` | 1 | 消費されない | — |
| `lock,repaint` | 1 | 消費されない | — |
| `open,addressbar` | 1 | 消費されない | — |
| `open,aigraph` | 1 | 消費されない | — |
| `open,archiveviewer` | 1 | 消費されない | — |
| `open,backlogviewer` | 1 | 消費されない | — |
| `open,balloonexplorer` | 1 | 消費されない | — |
| `open,browser` | 1 | 消費されない | — |
| `open,calendar` | 1 | 消費されない | — |
| `open,communicatebox` | 1 | 消費されない | — |
| `open,configurationdialog` | 1 | 消費されない | — |
| `open,dateinput` | 1 | 消費されない | — |
| `open,developer` | 1 | 消費されない | — |
| `open,dialog` | 4 | 消費されない | — |
| `open,dressupexplorer` | 1 | 消費されない | — |
| `open,editor` | 1 | 消費されない | — |
| `open,errorlog` | 1 | 消費されない | — |
| `open,explorer` | 1 | 消費されない | — |
| `open,file` | 1 | 消費されない | — |
| `open,ghostexplorer` | 1 | 消費されない | — |
| `open,headlinesensorexplorer` | 1 | 消費されない | — |
| `open,help` | 1 | 消費されない | — |
| `open,inputbox` | 1 | 消費されない | — |
| `open,ipinput` | 1 | 消費されない | — |
| `open,mailer` | 1 | 消費されない | — |
| `open,messenger` | 1 | 消費されない | — |
| `open,passwordinput` | 1 | 消費されない | — |
| `open,pictureviewer` | 1 | 消費されない | — |
| `open,pluginexplorer` | 1 | 消費されない | — |
| `open,rateofusegraph` | 1 | 消費されない | — |
| `open,rateofusegraphballoon` | 1 | 消費されない | — |
| `open,rateofusegraphtotal` | 1 | 消費されない | — |
| `open,readme` | 1 | 消費されない | — |
| `open,shellexplorer` | 1 | 消費されない | — |
| `open,shiorirequest` | 1 | 消費されない | — |
| `open,sliderinput` | 1 | 消費されない | — |
| `open,surfacetest` | 1 | 消費されない | — |
| `open,teachbox` | 1 | 消費されない | — |
| `open,terms` | 1 | 消費されない | — |
| `open,timeinput` | 1 | 消費されない | — |
| `quicksection,false` | 1 | 消費されない | — |
| `quicksection,true` | 1 | 消費されない | — |
| `reload,aigraph` | 1 | 消費されない | — |
| `reload,balloon` | 1 | 消費されない | — |
| `reload,descript` | 2 | 消費されない | — |
| `reload,ghost` | 1 | 消費されない | — |
| `reload,makoto` | 1 | 消費されない | — |
| `reload,shell` | 1 | 消費されない | — |
| `reload,shiori` | 1 | 消費されない | — |
| `reset,position` | 1 | 消費されない | — |
| `reset,sticky-window` | 1 | 消費されない | — |
| `reset,syncobject` | 1 | 消費されない | — |
| `reset,zorder` | 1 | 消費される | `ZOrderCueSink::emit` |
| `send,websocket` | 1 | 消費されない | — |
| `send,websocket-binary` | 1 | 消費されない | — |
| `set,alignmentondesktop` | 1 | 消費されない | — |
| `set,alignmenttodesktop` | 2 | 消費されない | — |
| `set,alpha` | 1 | 消費されない | — |
| `set,autoscroll` | 2 | 消費されない | — |
| `set,balloonalign` | 1 | 消費されない | — |
| `set,balloonmarker` | 1 | 消費されない | — |
| `set,balloonnum` | 1 | 消費されない | — |
| `set,balloonoffset` | 1 | 消費されない | — |
| `set,balloontimeout` | 1 | 消費されない | — |
| `set,balloonwait` | 1 | 消費されない | — |
| `set,choicetimeout` | 1 | 消費されない | — |
| `set,otherghosttalk` | 1 | 消費されない | — |
| `set,othersurfacechange` | 1 | 消費されない | — |
| `set,position` | 1 | 消費されない | — |
| `set,property` | 1 | 消費されない | — |
| `set,scaling` | 3 | 消費されない | — |
| `set,serikotalk` | 1 | 消費されない | — |
| `set,shioridebugmode` | 1 | 消費されない | — |
| `set,sticky-window` | 1 | 消費されない | — |
| `set,syncobject` | 1 | 消費されない | — |
| `set,tasktrayicon` | 1 | 消費されない | — |
| `set,trayballoon` | 1 | 消費されない | — |
| `set,wallpaper` | 1 | 消費されない | — |
| `set,windowstate` | 3 | 消費されない | — |
| `set,zorder` | 1 | 消費される | `ZOrderCueSink::emit` |
| `sound,cdplay` | 1 | 消費されない | — |
| `sound,load` | 1 | 消費されない | — |
| `sound,loop` | 1 | 消費されない | — |
| `sound,option` | 1 | 消費されない | — |
| `sound,pause` | 1 | 消費されない | — |
| `sound,play` | 1 | 消費されない | — |
| `sound,resume` | 1 | 消費されない | — |
| `sound,stop` | 1 | 消費されない | — |
| `sound,wait` | 1 | 消費されない | — |
| `unload,makoto` | 1 | 消費されない | — |
| `unload,shiori` | 1 | 消費されない | — |
| `unlock,balloonmove` | 1 | 消費されない | — |
| `unlock,balloonrepaint` | 1 | 消費されない | — |
| `unlock,repaint` | 1 | 消費されない | — |
| `update,platform` | 1 | 消費されない | — |
| `update,更新対象` | 1 | 消費されない | — |

2 つの表を突き合わせると、たとえば `set` は **31 件・25 組**あって、消費されるのは選択子 `zorder` の
1 組 1 件だけだと読める。残る 30 件は名前が運ばれるだけで、受け取る側がいない。

### 既に引数の語彙を持っている場所

`\![...]` の引数の語彙を areka が既に持っている場所が 1 つだけある。窓の重なり順を指定する
`\![set,zorder,...]` の引数を読む場所で、`balloon`・`surface` と省略形の `b`・`s` の 4 語を
解釈している。

この 4 語が正典のどの記述に当たるかというと、`\![set,zorder,...]` の本文のうち、
キャラの窓だけでなくバルーンも混ぜて重なり順を並べる書き方を説明している段である。
その段は `balloonN`・`surfaceN` と書く形と、`bN`・`sN` と省略する形の 2 通りを示している。
本文に添えられた版の表記が 2 つに割れていることから、省略する形が後から足されたと読める。

---

## ⑶ 担当の突合表

台帳の `owner` の欄（この節では「担当」と書く）は、既にある文書がタグの担当を宣言している
ところを 1 件ずつ項目 id へ写したものである。写した元は 2 つある。

- `.kiro/specs/*/brief.md` のうち、さくらスクリプトのタグの担当を宣言している **11 本**
- `doc/COMPAT_ARCHITECTURE.md` の §8 の **21 行**（タグを主題にする 17 行と `%` を主題にする 4 行）

**この一覧の出口はこの文書しかない。** 機械で作り直すドメイン別の報告
（`doc/ukadoc-coverage/report/sakura-script.md`）は状態と世代の集計だけを出す作りで、
担当の列を持たない。担当を読みたい人はこの節と次の ⑷ 節を見るほかない。

### 表 ⑶-1: 11 本の brief が宣言したタグと項目 id の対応（96 行・全数）

brief が書いている綴りを 1 つずつ項目 id へ当てた。当たり方が「その項目の残る作業を引き受ける」
と読める行だけを **所有** とし、壊れ方の実例・フィクスチャの棚卸し・原典の記述例として
綴りを挙げているだけの行は **例示**、タグ語をどこで閉じるかの対象として並べている行は
**字句の境界** と分けてある。**担当になるのは所有の行だけ**である。

| 主張している brief | brief に書かれた綴り | 項目の綴り | 項目 id | 主張の型 | 台帳の担当 |
| --- | --- | --- | --- | --- | --- |
| `areka-P0-anchor-tag-canon` | `\_a[ID,引数]` / `\_a[ID]` | `\_a[ID]` | `ukadoc:list_sakura_script:_5c_a_5bID_5d:1` | 所有 | 空 |
| `areka-P0-anchor-tag-canon` | `\_l[10,20]` | `\_l[x,y]` | `ukadoc:list_sakura_script:_5c_l_5bx_2cy_5d:1` | 例示 | `areka-P0-cursor-tag-canon` |
| `areka-P0-anchor-tag-canon` | `\_n` | `\_n` | `ukadoc:list_sakura_script:_5c_n:1` | 例示 | 空 |
| `areka-P0-anchor-tag-canon` | `\_q` | `\_q` | `ukadoc:list_sakura_script:_5c_q:1` | 例示 | `areka-P0-sakura-time-directives` |
| `areka-P0-anchor-tag-canon` | `\_v` | `\_v[ファイル名]` | `ukadoc:list_sakura_script:_5c_v_5b_30d5_30a1_30a4_30eb_540d_5d:1` | 例示 | 空 |
| `areka-P0-anchor-tag-canon` | `\_w[450]` | `\_w[時間]` | `ukadoc:list_sakura_script:_5c_w_5b_6642_9593_5d:1` | 例示 | 空 |
| `areka-P0-anchor-tag-canon` | `\f[anchor.font.color]` | `\f[anchor.font.color,色指定]` | `ukadoc:list_sakura_script:_5cf_5banchor.font.color_2c_8272_6307_5b9a_5d:1` | 所有 | この brief |
| `areka-P0-anchor-tag-canon` | `\f[anchorbrushcolor]` / `\f[anchorcolor]` | `\f[anchorcolor,色指定]もしくは\f[anchorbrushcolor,色指定]` | `ukadoc:list_sakura_script:_5cf_5banchorcolor_2c_8272_6307_5b9a_5d_3082_3057_304f_306f_5cf_5banchorbrushcolor_2c_8272_6307_5b9a_5d:1` | 所有 | この brief |
| `areka-P0-anchor-tag-canon` | `\f[anchorfontcolor]` | `\f[anchorfontcolor,色指定]` | `ukadoc:list_sakura_script:_5cf_5banchorfontcolor_2c_8272_6307_5b9a_5d:1` | 所有 | この brief |
| `areka-P0-anchor-tag-canon` | `\f[anchormethod]` | `\f[anchormethod,描画方法]` | `ukadoc:list_sakura_script:_5cf_5banchormethod_2c_63cf_753b_65b9_6cd5_5d:1` | 所有 | この brief |
| `areka-P0-anchor-tag-canon` | `\f[anchornotselect*]` | `\f[anchornotselectcolor,色指定]もしくは\f[anchornotselectbrushcolor,色指定]` | `ukadoc:list_sakura_script:_5cf_5banchornotselectcolor_2c_8272_6307_5b9a_5d_3082_3057_304f_306f_5cf_5banchornotselectbrushcolor_2c_8272_6307_5b9a_5:1` | 所有 | この brief |
| `areka-P0-anchor-tag-canon` | `\f[anchornotselect*]` | `\f[anchornotselectfontcolor,色指定]` | `ukadoc:list_sakura_script:_5cf_5banchornotselectfontcolor_2c_8272_6307_5b9a_5d:1` | 所有 | この brief |
| `areka-P0-anchor-tag-canon` | `\f[anchornotselect*]` | `\f[anchornotselectmethod,描画方法]` | `ukadoc:list_sakura_script:_5cf_5banchornotselectmethod_2c_63cf_753b_65b9_6cd5_5d:1` | 所有 | この brief |
| `areka-P0-anchor-tag-canon` | `\f[anchornotselect*]` | `\f[anchornotselectpencolor,色指定]` | `ukadoc:list_sakura_script:_5cf_5banchornotselectpencolor_2c_8272_6307_5b9a_5d:1` | 所有 | この brief |
| `areka-P0-anchor-tag-canon` | `\f[anchornotselect*]` | `\f[anchornotselectstyle,形状]` | `ukadoc:list_sakura_script:_5cf_5banchornotselectstyle_2c_5f62_72b6_5d:1` | 所有 | この brief |
| `areka-P0-anchor-tag-canon` | `\f[anchorpencolor]` | `\f[anchorpencolor,色指定]` | `ukadoc:list_sakura_script:_5cf_5banchorpencolor_2c_8272_6307_5b9a_5d:1` | 所有 | この brief |
| `areka-P0-anchor-tag-canon` | `\f[anchorstyle]` | `\f[anchorstyle,形状]` | `ukadoc:list_sakura_script:_5cf_5banchorstyle_2c_5f62_72b6_5d:1` | 所有 | この brief |
| `areka-P0-anchor-tag-canon` | `\f[anchorvisited*]` | `\f[anchorvisitedcolor,色指定]もしくは\f[anchorvisitedbrushcolor,色指定]` | `ukadoc:list_sakura_script:_5cf_5banchorvisitedcolor_2c_8272_6307_5b9a_5d_3082_3057_304f_306f_5cf_5banchorvisitedbrushcolor_2c_8272_6307_5b9a_5d:1` | 所有 | この brief |
| `areka-P0-anchor-tag-canon` | `\f[anchorvisited*]` | `\f[anchorvisitedfontcolor,色指定]` | `ukadoc:list_sakura_script:_5cf_5banchorvisitedfontcolor_2c_8272_6307_5b9a_5d:1` | 所有 | この brief |
| `areka-P0-anchor-tag-canon` | `\f[anchorvisited*]` | `\f[anchorvisitedmethod,描画方法]` | `ukadoc:list_sakura_script:_5cf_5banchorvisitedmethod_2c_63cf_753b_65b9_6cd5_5d:1` | 所有 | この brief |
| `areka-P0-anchor-tag-canon` | `\f[anchorvisited*]` | `\f[anchorvisitedpencolor,色指定]` | `ukadoc:list_sakura_script:_5cf_5banchorvisitedpencolor_2c_8272_6307_5b9a_5d:1` | 所有 | この brief |
| `areka-P0-anchor-tag-canon` | `\f[anchorvisited*]` | `\f[anchorvisitedstyle,形状]` | `ukadoc:list_sakura_script:_5cf_5banchorvisitedstyle_2c_5f62_72b6_5d:1` | 所有 | この brief |
| `areka-P0-balloon-canon-residue` | `\![reload,balloon]` | `\![reload,balloon]` | `ukadoc:list_sakura_script:_5c_21_5breload_2cballoon_5d:1` | 所有 | この brief |
| `areka-P0-balloon-canon-residue` | `\![set,balloontimeout,時間]` | `\![set,balloontimeout,時間]` | `ukadoc:list_sakura_script:_5c_21_5bset_2cballoontimeout_2c_6642_9593_5d:1` | 所有 | この brief |
| `areka-P0-balloon-canon-residue` | `\0` | `\0もしくは\h` | `ukadoc:list_sakura_script:_5c0_3082_3057_304f_306f_5ch:1` | 例示 | `areka-P0-kero-balloon` |
| `areka-P0-balloon-canon-residue` | `\_l` | `\_l[x,y]` | `ukadoc:list_sakura_script:_5c_l_5bx_2cy_5d:1` | 所有 | `areka-P0-cursor-tag-canon` |
| `areka-P0-balloon-canon-residue` | `\e` | `\e` | `ukadoc:list_sakura_script:_5ce:1` | 例示 | 空 |
| `areka-P0-balloon-canon-residue` | `\x` | `\x` | `ukadoc:list_sakura_script:_5cx:1` | 所有 | 空 |
| `areka-P0-balloon-canon-residue` | `\x[noclear]` | `\x[noclear]` | `ukadoc:list_sakura_script:_5cx_5bnoclear_5d:1` | 所有 | この brief |
| `areka-P0-choice-marker-styling` | `\f[cursor*]` | `\f[cursorcolor,色指定]もしくは\f[cursorbrushcolor,色指定]` | `ukadoc:list_sakura_script:_5cf_5bcursorcolor_2c_8272_6307_5b9a_5d_3082_3057_304f_306f_5cf_5bcursorbrushcolor_2c_8272_6307_5b9a_5d:1` | 所有 | この brief |
| `areka-P0-choice-marker-styling` | `\f[cursor*]` | `\f[cursorfontcolor,色指定]` | `ukadoc:list_sakura_script:_5cf_5bcursorfontcolor_2c_8272_6307_5b9a_5d:1` | 所有 | この brief |
| `areka-P0-choice-marker-styling` | `\f[cursor*]` | `\f[cursormethod,描画方法]` | `ukadoc:list_sakura_script:_5cf_5bcursormethod_2c_63cf_753b_65b9_6cd5_5d:1` | 所有 | この brief |
| `areka-P0-choice-marker-styling` | `\f[cursor*]` | `\f[cursornotselectcolor,色指定]もしくは\f[cursornotselectbrushcolor,色指定]` | `ukadoc:list_sakura_script:_5cf_5bcursornotselectcolor_2c_8272_6307_5b9a_5d_3082_3057_304f_306f_5cf_5bcursornotselectbrushcolor_2c_8272_6307_5b9a_5:1` | 所有 | この brief |
| `areka-P0-choice-marker-styling` | `\f[cursor*]` | `\f[cursornotselectfontcolor,色指定]` | `ukadoc:list_sakura_script:_5cf_5bcursornotselectfontcolor_2c_8272_6307_5b9a_5d:1` | 所有 | この brief |
| `areka-P0-choice-marker-styling` | `\f[cursor*]` | `\f[cursornotselectmethod,描画方法]` | `ukadoc:list_sakura_script:_5cf_5bcursornotselectmethod_2c_63cf_753b_65b9_6cd5_5d:1` | 所有 | この brief |
| `areka-P0-choice-marker-styling` | `\f[cursor*]` | `\f[cursornotselectpencolor,色指定]` | `ukadoc:list_sakura_script:_5cf_5bcursornotselectpencolor_2c_8272_6307_5b9a_5d:1` | 所有 | この brief |
| `areka-P0-choice-marker-styling` | `\f[cursor*]` | `\f[cursornotselectstyle,形状]` | `ukadoc:list_sakura_script:_5cf_5bcursornotselectstyle_2c_5f62_72b6_5d:1` | 所有 | この brief |
| `areka-P0-choice-marker-styling` | `\f[cursor*]` | `\f[cursorpencolor,色指定]` | `ukadoc:list_sakura_script:_5cf_5bcursorpencolor_2c_8272_6307_5b9a_5d:1` | 所有 | この brief |
| `areka-P0-choice-marker-styling` | `\f[cursor*]` | `\f[cursorstyle,形状]` | `ukadoc:list_sakura_script:_5cf_5bcursorstyle_2c_5f62_72b6_5d:1` | 所有 | この brief |
| `areka-P0-cursor-tag-canon` | `\_l` / `\_l[0,0]` / `\_l[x,y]` | `\_l[x,y]` | `ukadoc:list_sakura_script:_5c_l_5bx_2cy_5d:1` | 所有・例示 | この brief |
| `areka-P0-cursor-tag-canon` | `\c` | `\c` | `ukadoc:list_sakura_script:_5cc:1` | 所有 | この brief |
| `areka-P0-cursor-tag-canon` | `\f[align]` | `\f[align,寄せる側]` | `ukadoc:list_sakura_script:_5cf_5balign_2c_5bc4_305b_308b_5074_5d:1` | 例示 | `areka-P0-text-decoration-canon` |
| `areka-P0-makoto-dll-host` | `\![load,makoto]` | `\![load,makoto]` | `ukadoc:list_sakura_script:_5c_21_5bload_2cmakoto_5d:1` | 所有 | この brief |
| `areka-P0-makoto-dll-host` | `\![reload,makoto]` | `\![reload,makoto]` | `ukadoc:list_sakura_script:_5c_21_5breload_2cmakoto_5d:1` | 所有 | この brief |
| `areka-P0-makoto-dll-host` | `\![unload,makoto]` | `\![unload,makoto]` | `ukadoc:list_sakura_script:_5c_21_5bunload_2cmakoto_5d:1` | 所有 | この brief |
| `areka-P0-makoto-dll-host` | `\0` | `\0もしくは\h` | `ukadoc:list_sakura_script:_5c0_3082_3057_304f_306f_5ch:1` | 例示 | `areka-P0-kero-balloon` |
| `areka-P0-makoto-dll-host` | `\e` | `\e` | `ukadoc:list_sakura_script:_5ce:1` | 例示 | 空 |
| `areka-P0-property-query-channels` | `%property[...]` / `%property[x]` / `%property[…]` / `%property[プロパティ名]` | `%property[プロパティ名]` | `ukadoc:list_sakura_script:_25property_5b_30d7_30ed_30d1_30c6_30a3_540d_5d:1` | 所有 | この brief |
| `areka-P0-property-query-channels` | `\![embed,イベント名,r0,...]` / `\![embed]` | `\![embed,イベント名,r0,r1,r2...]` | `ukadoc:list_sakura_script:_5c_21_5bembed_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1` | 所有 | 空 |
| `areka-P0-property-query-channels` | `\![get,*]` / `\![get,property,…]` / `\![get,property,イベント名,プロパティ名,...]` / `\![get,property]` | `\![get,property,イベント名,プロパティ名,プロパティ名,...]` | `ukadoc:list_sakura_script:_5c_21_5bget_2cproperty_2c_30a4_30d9_30f3_30c8_540d_2c_30d7_30ed_30d1_30c6_30a3_540d_2c_30d7_30ed_30d1_30c6_30a3_540d_2c:1` | 所有 | この brief |
| `areka-P0-property-query-channels` | `\![set,property,プロパティ名,値]` / `\![set,property]` | `\![set,property,プロパティ名,値]` | `ukadoc:list_sakura_script:_5c_21_5bset_2cproperty_2c_30d7_30ed_30d1_30c6_30a3_540d_2c_5024_5d:1` | 所有 | この brief |
| `areka-P0-sakura-tag-word-boundary` | `\![move,...]` | `\![move]` | `ukadoc:list_sakura_script:_5c_21_5bmove_5d:1` | 例示 | 空 |
| `areka-P0-sakura-tag-word-boundary` | `\-` | `\-` | `ukadoc:list_sakura_script:_5c-:1` | 字句の境界・例示 | 空 |
| `areka-P0-sakura-tag-word-boundary` | `\0` / `\h` | `\0もしくは\h` | `ukadoc:list_sakura_script:_5c0_3082_3057_304f_306f_5ch:1` | 字句の境界・例示 | `areka-P0-kero-balloon` |
| `areka-P0-sakura-tag-word-boundary` | `\1` / `\u` | `\1もしくは\u` | `ukadoc:list_sakura_script:_5c1_3082_3057_304f_306f_5cu:1` | 字句の境界・例示 | `areka-P0-kero-balloon` |
| `areka-P0-sakura-tag-word-boundary` | `\_a[ID]` | `\_a[ID]` | `ukadoc:list_sakura_script:_5c_a_5bID_5d:1` | 字句の境界 | 空 |
| `areka-P0-sakura-tag-word-boundary` | `\_l[...]` | `\_l[x,y]` | `ukadoc:list_sakura_script:_5c_l_5bx_2cy_5d:1` | 例示 | `areka-P0-cursor-tag-canon` |
| `areka-P0-sakura-tag-word-boundary` | `\_w[600]` | `\_w[時間]` | `ukadoc:list_sakura_script:_5c_w_5b_6642_9593_5d:1` | 例示 | 空 |
| `areka-P0-sakura-tag-word-boundary` | `\c` | `\c` | `ukadoc:list_sakura_script:_5cc:1` | 字句の境界 | `areka-P0-cursor-tag-canon` |
| `areka-P0-sakura-tag-word-boundary` | `\e` | `\e` | `ukadoc:list_sakura_script:_5ce:1` | 字句の境界・例示 | 空 |
| `areka-P0-sakura-tag-word-boundary` | `\n[half]` | `\n[half]` | `ukadoc:list_sakura_script:_5cn_5bhalf_5d:1` | 字句の境界 | 空 |
| `areka-P0-sakura-tag-word-boundary` | `\w` / `\w[2]` / `\w[3000]` | `\w時間` | `ukadoc:list_sakura_script:_5cw_6642_9593:1` | 字句の境界・例示 | 空 |
| `areka-P0-sakura-time-directives` | `\![embed,イベント名,r*]` | `\![embed,イベント名,r0,r1,r2...]` | `ukadoc:list_sakura_script:_5c_21_5bembed_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1` | 所有 | 空 |
| `areka-P0-sakura-time-directives` | `\![move]` | `\![move]` | `ukadoc:list_sakura_script:_5c_21_5bmove_5d:1` | 所有 | 空 |
| `areka-P0-sakura-time-directives` | `\![set,alpha,--time/--wait]` | `\![set,alpha,数値,オプション]` | `ukadoc:list_sakura_script:_5c_21_5bset_2calpha_2c_6570_5024_2c_30aa_30d7_30b7_30e7_30f3_5d:1` | 所有 | この brief |
| `areka-P0-sakura-time-directives` | `\![set,balloontimeout,時間]` / `\![set,balloontimeout]` | `\![set,balloontimeout,時間]` | `ukadoc:list_sakura_script:_5c_21_5bset_2cballoontimeout_2c_6642_9593_5d:1` | 所有 | `areka-P0-balloon-canon-residue` |
| `areka-P0-sakura-time-directives` | `\![set,balloonwait,倍率\|ms指定]` | `\![set,balloonwait,倍率]` | `ukadoc:list_sakura_script:_5c_21_5bset_2cballoonwait_2c_500d_7387_5d:1` | 所有 | この brief |
| `areka-P0-sakura-time-directives` | `\![set,choicetimeout,時間]` | `\![set,choicetimeout,時間]` | `ukadoc:list_sakura_script:_5c_21_5bset_2cchoicetimeout_2c_6642_9593_5d:1` | 所有 | この brief |
| `areka-P0-sakura-time-directives` | `\![sound,wait]` | `\![sound,wait]` | `ukadoc:list_sakura_script:_5c_21_5bsound_2cwait_5d:1` | 所有 | この brief |
| `areka-P0-sakura-time-directives` | `\![wait,syncobject,名前,--timeout=]` | `\![wait,syncobject,同期オブジェクト名,オプション]` | `ukadoc:list_sakura_script:_5c_21_5bwait_2csyncobject_2c_540c_671f_30aa_30d6_30b8_30a7_30af_30c8_540d_2c_30aa_30d7_30b7_30e7_30f3_5d:1` | 所有 | この brief |
| `areka-P0-sakura-time-directives` | `\_V` | `\_V` | `ukadoc:list_sakura_script:_5c_V:1` | 所有 | 空 |
| `areka-P0-sakura-time-directives` | `\_q` | `\_q` | `ukadoc:list_sakura_script:_5c_q:1` | 例示 | この brief |
| `areka-P0-status-execution-states` | `\![enter,inductionmode]` | `\![enter,inductionmode]` | `ukadoc:list_sakura_script:_5c_21_5benter_2cinductionmode_5d:1` | 所有 | この brief |
| `areka-P0-status-execution-states` | `\![enter,passivemode]` | `\![enter,passivemode]` | `ukadoc:list_sakura_script:_5c_21_5benter_2cpassivemode_5d:1` | 所有 | この brief |
| `areka-P0-status-execution-states` | `\t` | `\t` | `ukadoc:list_sakura_script:_5ct:1` | 所有 | この brief |
| `areka-P0-surfaces-basepos` | `\![move,...,base,base]` / `\![move]` | `\![move]` | `ukadoc:list_sakura_script:_5c_21_5bmove_5d:1` | 所有 | 空 |
| `areka-P0-text-decoration-canon` | `\_l` | `\_l[x,y]` | `ukadoc:list_sakura_script:_5c_l_5bx_2cy_5d:1` | 所有 | `areka-P0-cursor-tag-canon` |
| `areka-P0-text-decoration-canon` | `\f[align]` | `\f[align,寄せる側]` | `ukadoc:list_sakura_script:_5cf_5balign_2c_5bc4_305b_308b_5074_5d:1` | 所有 | この brief |
| `areka-P0-text-decoration-canon` | `\f[bold,1]` / `\f[bold]` | `\f[bold,パラメータ]` | `ukadoc:list_sakura_script:_5cf_5bbold_2c_30d1_30e9_30e1_30fc_30bf_5d:1` | 所有 | この brief |
| `areka-P0-text-decoration-canon` | `\f[color]` | `\f[color,色指定]` | `ukadoc:list_sakura_script:_5cf_5bcolor_2c_8272_6307_5b9a_5d:1` | 所有 | この brief |
| `areka-P0-text-decoration-canon` | `\f[default]` | `\f[default]` | `ukadoc:list_sakura_script:_5cf_5bdefault_5d:1` | 所有 | この brief |
| `areka-P0-text-decoration-canon` | `\f[disable]` | `\f[disable]` | `ukadoc:list_sakura_script:_5cf_5bdisable_5d:1` | 所有 | この brief |
| `areka-P0-text-decoration-canon` | `\f[height]` | `\f[height,数値]` | `ukadoc:list_sakura_script:_5cf_5bheight_2c_6570_5024_5d:1` | 所有 | この brief |
| `areka-P0-text-decoration-canon` | `\f[italic]` | `\f[italic,パラメータ]` | `ukadoc:list_sakura_script:_5cf_5bitalic_2c_30d1_30e9_30e1_30fc_30bf_5d:1` | 所有 | この brief |
| `areka-P0-text-decoration-canon` | `\f[name]` | `\f[name,フォント名]` | `ukadoc:list_sakura_script:_5cf_5bname_2c_30d5_30a9_30f3_30c8_540d_5d:1` | 所有 | この brief |
| `areka-P0-text-decoration-canon` | `\f[outline]` | `\f[outline,パラメータ]` | `ukadoc:list_sakura_script:_5cf_5boutline_2c_30d1_30e9_30e1_30fc_30bf_5d:1` | 所有 | この brief |
| `areka-P0-text-decoration-canon` | `\f[shadowcolor,none]` | `\f[shadowcolor,none]` | `ukadoc:list_sakura_script:_5cf_5bshadowcolor_2cnone_5d:1` | 所有 | この brief |
| `areka-P0-text-decoration-canon` | `\f[shadowcolor]` | `\f[shadowcolor,色指定]` | `ukadoc:list_sakura_script:_5cf_5bshadowcolor_2c_8272_6307_5b9a_5d:1` | 所有 | この brief |
| `areka-P0-text-decoration-canon` | `\f[shadowstyle]` | `\f[shadowstyle,形態指定]` | `ukadoc:list_sakura_script:_5cf_5bshadowstyle_2c_5f62_614b_6307_5b9a_5d:1` | 所有 | この brief |
| `areka-P0-text-decoration-canon` | `\f[strike]` | `\f[strike,パラメータ]` | `ukadoc:list_sakura_script:_5cf_5bstrike_2c_30d1_30e9_30e1_30fc_30bf_5d:1` | 所有 | この brief |
| `areka-P0-text-decoration-canon` | `\f[sub]` | `\f[sub,パラメータ]` | `ukadoc:list_sakura_script:_5cf_5bsub_2c_30d1_30e9_30e1_30fc_30bf_5d:1` | 所有 | この brief |
| `areka-P0-text-decoration-canon` | `\f[sup]` | `\f[sup,パラメータ]` | `ukadoc:list_sakura_script:_5cf_5bsup_2c_30d1_30e9_30e1_30fc_30bf_5d:1` | 所有 | この brief |
| `areka-P0-text-decoration-canon` | `\f[underline]` | `\f[underline,パラメータ]` | `ukadoc:list_sakura_script:_5cf_5bunderline_2c_30d1_30e9_30e1_30fc_30bf_5d:1` | 所有 | この brief |
| `areka-P0-text-decoration-canon` | `\f[valign]` | `\f[valign,寄せる側]` | `ukadoc:list_sakura_script:_5cf_5bvalign_2c_5bc4_305b_308b_5074_5d:1` | 所有 | この brief |
| `areka-P0-text-decoration-canon` | `\x` | `\x` | `ukadoc:list_sakura_script:_5cx:1` | 所有 | 空 |
| `areka-P0-text-decoration-canon` | `\x[noclear]` | `\x[noclear]` | `ukadoc:list_sakura_script:_5cx_5bnoclear_5d:1` | 所有 | `areka-P0-balloon-canon-residue` |

型で数えると **所有 74 組・字句の境界 8 組・例示 14 組**（1 つの項目を 2 本の brief が
主張していれば 2 組と数える）。所有の型が付いた項目は **67 件**である。

`areka-P0-sakura-tag-word-boundary` は 11 件の項目を挙げているのに、所有の行を 1 つも持たない。
この brief が引き受けているのは「タグの綴りをどこで閉じるか」という規則であって、
個々のタグの意味ではないからである。要件が数える「主張している 11 本」には入り続けるが、
台帳の担当には 1 度も現れない。

別名になった項目も落とさずに載せてある。別名の行では担当は根の側に付くので、
brief が挙げた件数と台帳の担当の件数はその分だけずれる。

| 別名になった項目 | 根の項目 | 主張している brief |
| --- | --- | --- |
| `\_V`（alias → 根の id: `ukadoc:list_sakura_script:_5c_21_5bsound_2cwait_5d:1`） | `\![sound,wait]` | `areka-P0-sakura-time-directives` |
| `\_a[ID]`（alias → 根の id: `ukadoc:list_sakura_script:_5c_a_5bID_2cr2_2cr3..._5d:1`） | `\_a[ID,r2,r3...]` | `areka-P0-anchor-tag-canon` |
| `\_a[ID]`（alias → 根の id: `ukadoc:list_sakura_script:_5c_a_5bID_2cr2_2cr3..._5d:1`） | `\_a[ID,r2,r3...]` | `areka-P0-sakura-tag-word-boundary` |
| `\_v[ファイル名]`（alias → 根の id: `ukadoc:list_sakura_script:_5c_21_5bsound_2cplay_2c_30d5_30a1_30a4_30eb_540d_2c_30aa_30d7_30b7_30e7_30f3..._5d:1`） | `\![sound,play,ファイル名,オプション...]` | `areka-P0-anchor-tag-canon` |
| `\x`（alias → 根の id: `ukadoc:list_sakura_script:_5cx_5bnoclear_5d:1`） | `\x[noclear]` | `areka-P0-balloon-canon-residue` |
| `\x`（alias → 根の id: `ukadoc:list_sakura_script:_5cx_5bnoclear_5d:1`） | `\x[noclear]` | `areka-P0-text-decoration-canon` |

### `\f` の 43 件が 3 本に過不足なく割れること

`\f` は 1 つの名前に 43 件の項目がぶら下がる、いちばん大きい群である。
この 43 件を 3 本の brief が分け合っていて、3 つの集合は重ならず、和はちょうど 43 になる。

| brief | 件数 | 取り出し方 |
| --- | ---: | --- |
| `areka-P0-text-decoration-canon` | 17 | 文字装飾の核。brief の「本 spec の所有分＝核 17 項目」の行が語を 17 個並べている |
| `areka-P0-anchor-tag-canon` | 16 | アンカーの装飾。brief の「アンカー装飾 16 項目」の行の 9 語のうち 2 語が各 5 件へ広がる |
| `areka-P0-choice-marker-styling` | 10 | 選択肢カーソルの装飾。同じ行の `\f[cursor*]` の前方一致で取れる |
| 合計 | 43 | 正典の `\f` は 43 件。欠け 0・余り 0 |

台帳の担当で数え直しても同じになる——
`areka-P0-anchor-tag-canon` 16 件、`areka-P0-choice-marker-styling` 10 件、`areka-P0-text-decoration-canon` 17 件、合計 43 件。

`\f` の綴りを本文に持つ brief は 3 本ではなく 4 本ある。4 本目の `areka-P0-cursor-tag-canon` は
`\f[align,寄せる側]` を挙げているが、自分の Scope の Out に「実装は別の spec が所有」と
書いているので、所有の主張ではなく例示に落ちる。だから 17＋16＋10 は動かない。

### 表 ⑶-2: 対応表 §8 の 21 行が宣言したタグ（57 行）

`doc/COMPAT_ARCHITECTURE.md` の §8 は、正典が黙っているところで areka がどう決めたかを
登記した表である。そのうちさくらスクリプトの項目を主題にしている 21 行が宣言しているタグを、
1 つずつ項目 id へ当てた。行は行番号ではなく**主題**で指している（行番号は文書が伸び縮みすると
ずれるため）。

| §8 の行の主題 | 行に書かれた綴り | 項目 id | 項目の綴り |
| --- | --- | --- | --- |
| **`\![move]` のオフセット引数を拡大率 k で換算すること**（上行の SSP 自己不整合 (b) に対する areka 側の実… | `\![move]` | `ukadoc:list_sakura_script:_5c_21_5bmove_5d:1` | `\![move]` |
| **`\![reset,zorder,...]` に余分なトークンが付いていたときの扱い**（正典の解除は引数を取らず、余分の可否に沈黙） | `\![reset,zorder,...]` | `ukadoc:list_sakura_script:_5c_21_5breset_2czorder_5d:1` | `\![reset,zorder]` |
| **`\![reset,zorder,...]` に余分なトークンが付いていたときの扱い**（正典の解除は引数を取らず、余分の可否に沈黙） | `\![reset,zorder]` | `ukadoc:list_sakura_script:_5c_21_5breset_2czorder_5d:1` | `\![reset,zorder]` |
| **`\_l` の縦書き座標系の正典写像と、areka の既知非互換**（本表の別行にある `origin` クランプ撤去の行とは別項目であ… | `\_l` | `ukadoc:list_sakura_script:_5c_l_5bx_2cy_5d:1` | `\_l[x,y]` |
| **`\_l` の縦書き座標系の正典写像と、areka の既知非互換**（本表の別行にある `origin` クランプ撤去の行とは別項目であ… | `\_l[0,0]` | `ukadoc:list_sakura_script:_5c_l_5bx_2cy_5d:1` | `\_l[x,y]` |
| **`\_l` の縦書き座標系の正典写像と、areka の既知非互換**（本表の別行にある `origin` クランプ撤去の行とは別項目であ… | `\_l[x,y]` | `ukadoc:list_sakura_script:_5c_l_5bx_2cy_5d:1` | `\_l[x,y]` |
| **`\_l` の縦書き座標系の正典写像と、areka の既知非互換**（本表の別行にある `origin` クランプ撤去の行とは別項目であ… | `\f[align]` | `ukadoc:list_sakura_script:_5cf_5balign_2c_5bc4_305b_308b_5074_5d:1` | `\f[align,寄せる側]` |
| **`\f[align]`／`\f[valign]`／下線の縦書き写像**（正典の 2 ページで `valign` の写像が逆になっている）… | `\f` | 族への言及（1 件に絞れない） | — |
| **`\f[align]`／`\f[valign]`／下線の縦書き写像**（正典の 2 ページで `valign` の写像が逆になっている）… | `\f[align,～]` | `ukadoc:list_sakura_script:_5cf_5balign_2c_5bc4_305b_308b_5074_5d:1` | `\f[align,寄せる側]` |
| **`\f[align]`／`\f[valign]`／下線の縦書き写像**（正典の 2 ページで `valign` の写像が逆になっている）… | `\f[align]` | `ukadoc:list_sakura_script:_5cf_5balign_2c_5bc4_305b_308b_5074_5d:1` | `\f[align,寄せる側]` |
| **`\f[align]`／`\f[valign]`／下線の縦書き写像**（正典の 2 ページで `valign` の写像が逆になっている）… | `\f[underline,true]` | `ukadoc:list_sakura_script:_5cf_5bunderline_2c_30d1_30e9_30e1_30fc_30bf_5d:1` | `\f[underline,パラメータ]` |
| **`\f[align]`／`\f[valign]`／下線の縦書き写像**（正典の 2 ページで `valign` の写像が逆になっている）… | `\f[valign,～]` | `ukadoc:list_sakura_script:_5cf_5bvalign_2c_5bc4_305b_308b_5074_5d:1` | `\f[valign,寄せる側]` |
| **`\f[align]`／`\f[valign]`／下線の縦書き写像**（正典の 2 ページで `valign` の写像が逆になっている）… | `\f[valign]` | `ukadoc:list_sakura_script:_5cf_5bvalign_2c_5bc4_305b_308b_5074_5d:1` | `\f[valign,寄せる側]` |
| **shell 設定の `seriko.zorder` でバルーン込みの明示記法（`balloonN`／`surfaceN`・省略形 `bN… | `\![set,zorder]` | `ukadoc:list_sakura_script:_5c_21_5bset_2czorder_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c..._5d:1` | `\![set,zorder,スコープID,スコープID,...]` |
| **【訂正】shell 設定の `seriko.zorder` を「絵の重ね順（SERIKO のレイヤ順）」とする既存記述** | `\![set,zorder,スコープID,...]` | `ukadoc:list_sakura_script:_5c_21_5bset_2czorder_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c..._5d:1` | `\![set,zorder,スコープID,スコープID,...]` |
| **【訂正】shell 設定の `seriko.zorder` を「絵の重ね順（SERIKO のレイヤ順）」とする既存記述** | `\![set,zorder]` | `ukadoc:list_sakura_script:_5c_21_5bset_2czorder_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c..._5d:1` | `\![set,zorder,スコープID,スコープID,...]` |
| **バルーンの面切替が可視性を変えないこと**（`\b[ID]` および面のアニメーション定義の反復再生由来を含む。正典は両者の関係に沈黙）… | `\b[-1]` | 族への言及（1 件に絞れない） | — |
| **バルーンの面切替が可視性を変えないこと**（`\b[ID]` および面のアニメーション定義の反復再生由来を含む。正典は両者の関係に沈黙）… | `\b[ID]` | 族への言及（1 件に絞れない） | — |
| `%keroname`（descript `kero.name`）が未定義のときの値 | `%keroname` | `ukadoc:list_sakura_script:_25keroname:1` | `%keroname` |
| `%selfname2`（descript `sakura.name2`）が未定義のときの値 | `%selfname2` | `ukadoc:list_sakura_script:_25selfname2:1` | `%selfname2` |
| `%username` の SHIORI Resource `username` GET が 204 No Content／空値を応答した場… | `%username` | `ukadoc:list_sakura_script:_25username:1` | `%username` |
| `%username` 既定値（スナップショット未解決時） | `%username` | `ukadoc:list_sakura_script:_25username:1` | `%username` |
| `\![move]` の名前付き `--key=value` 形（ukadoc 記述例の形式） | `\![move]` | `ukadoc:list_sakura_script:_5c_21_5bmove_5d:1` | `\![move]` |
| `\![move]` の基準 `screen`／`primaryscreen`／`me`／`global` | `\![move]` | `ukadoc:list_sakura_script:_5c_21_5bmove_5d:1` | `\![move]` |
| `\![move]` の基準位置 `base`（basepos）解決 | `\![move]` | `ukadoc:list_sakura_script:_5c_21_5bmove_5d:1` | `\![move]` |
| `\![move]` の時間指定 `time>0`（アニメーション付き移動） | `\![move]` | `ukadoc:list_sakura_script:_5c_21_5bmove_5d:1` | `\![move]` |
| `\![move]` の裸 `base`（ドット無しの基準位置トークン） | `\![move]` | `ukadoc:list_sakura_script:_5c_21_5bmove_5d:1` | `\![move]` |
| `\![set,balloontimeout,時間]`（バルーン表示のタイムアウト時間指定）の**バルーン寿命側の実導出** | `\!` | 族への言及（1 件に絞れない） | — |
| `\![set,balloontimeout,時間]`（バルーン表示のタイムアウト時間指定）の**バルーン寿命側の実導出** | `\![set,balloontimeout,時間]` | `ukadoc:list_sakura_script:_5c_21_5bset_2cballoontimeout_2c_6642_9593_5d:1` | `\![set,balloontimeout,時間]` |
| `\b[ID]` の ID が指す名前空間 | `\b` | 族への言及（1 件に絞れない） | — |
| `\b[ID]` の ID が指す名前空間 | `\b[1]` | 族への言及（1 件に絞れない） | — |
| `\b[ID]` の ID が指す名前空間 | `\b[ID]` | 族への言及（1 件に絞れない） | — |
| `\x` ／ `\x[noclear]`（バルーンをクリック待ちにする） | `\0` | `ukadoc:list_sakura_script:_5c0_3082_3057_304f_306f_5ch:1` | `\0もしくは\h` |
| `\x` ／ `\x[noclear]`（バルーンをクリック待ちにする） | `\e` | `ukadoc:list_sakura_script:_5ce:1` | `\e` |
| `\x` ／ `\x[noclear]`（バルーンをクリック待ちにする） | `\f` | 族への言及（1 件に絞れない） | — |
| `\x` ／ `\x[noclear]`（バルーンをクリック待ちにする） | `\x` | `ukadoc:list_sakura_script:_5cx:1` | `\x` |
| `\x` ／ `\x[noclear]`（バルーンをクリック待ちにする） | `\x[noclear]` | `ukadoc:list_sakura_script:_5cx_5bnoclear_5d:1` | `\x[noclear]` |
| compile 側時間指令 allowlist（`quicksection`／`set,balloonwait`／`set,choiceti… | `\!` | 族への言及（1 件に絞れない） | — |
| compile 側時間指令 allowlist（`quicksection`／`set,balloonwait`／`set,choiceti… | `embed` | `ukadoc:list_sakura_script:_5c_21_5bembed_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1` | `\![embed,イベント名,r0,r1,r2...]` |
| compile 側時間指令 allowlist（`quicksection`／`set,balloonwait`／`set,choiceti… | `move` | `ukadoc:list_sakura_script:_5c_21_5bmove_5d:1` | `\![move]` |
| compile 側時間指令 allowlist（`quicksection`／`set,balloonwait`／`set,choiceti… | `set,balloontimeout` | `ukadoc:list_sakura_script:_5c_21_5bset_2cballoontimeout_2c_6642_9593_5d:1` | `\![set,balloontimeout,時間]` |
| compile 側時間指令 allowlist（`quicksection`／`set,balloonwait`／`set,choiceti… | `set,balloonwait` | `ukadoc:list_sakura_script:_5c_21_5bset_2cballoonwait_2c_500d_7387_5d:1` | `\![set,balloonwait,倍率]` |
| compile 側時間指令 allowlist（`quicksection`／`set,balloonwait`／`set,choiceti… | `set,choicetimeout` | `ukadoc:list_sakura_script:_5c_21_5bset_2cchoicetimeout_2c_6642_9593_5d:1` | `\![set,choicetimeout,時間]` |
| compile 側時間指令 allowlist（`quicksection`／`set,balloonwait`／`set,choiceti… | `sound,wait` | `ukadoc:list_sakura_script:_5c_21_5bsound_2cwait_5d:1` | `\![sound,wait]` |
| compile 側時間指令 allowlist（`quicksection`／`set,balloonwait`／`set,choiceti… | `wait,syncobject` | `ukadoc:list_sakura_script:_5c_21_5bwait_2csyncobject_2c_540c_671f_30aa_30d6_30b8_30a7_30af_30c8_540d_2c_30aa_30d7_30b7_30e7_30f3_5d:1` | `\![wait,syncobject,同期オブジェクト名,オプション]` |
| 角括弧なし `\_` タグ（2 文字形 `\_X`・3 文字形 `\__X`）の字句境界と意味 | `\_!` | `ukadoc:list_sakura_script:_5c__21:1` | `\_!` |
| 角括弧なし `\_` タグ（2 文字形 `\_X`・3 文字形 `\__X`）の字句境界と意味 | `\_+` | `ukadoc:list_sakura_script:_5c__2b:1` | `\_+` |
| 角括弧なし `\_` タグ（2 文字形 `\_X`・3 文字形 `\__X`）の字句境界と意味 | `\_?` | `ukadoc:list_sakura_script:_5c__3f:1` | `\_?` |
| 角括弧なし `\_` タグ（2 文字形 `\_X`・3 文字形 `\__X`）の字句境界と意味 | `\_V` | `ukadoc:list_sakura_script:_5c_V:1` | `\_V` |
| 角括弧なし `\_` タグ（2 文字形 `\_X`・3 文字形 `\__X`）の字句境界と意味 | `\__c` | `ukadoc:list_sakura_script:_5c__c:1` | `\__c` |
| 角括弧なし `\_` タグ（2 文字形 `\_X`・3 文字形 `\__X`）の字句境界と意味 | `\__q` | `ukadoc:list_sakura_script:_5c__q_5bID_2c..._5d:1` | `\__q[ID,...]` |
| 角括弧なし `\_` タグ（2 文字形 `\_X`・3 文字形 `\__X`）の字句境界と意味 | `\__t` | `ukadoc:list_sakura_script:_5c__t:1` | `\__t` |
| 角括弧なし `\_` タグ（2 文字形 `\_X`・3 文字形 `\__X`）の字句境界と意味 | `\__v` | `ukadoc:list_sakura_script:_5c__v_5b_30aa_30d7_30b7_30e7_30f3_5d:1` | `\__v[オプション]` |
| 角括弧なし `\_` タグ（2 文字形 `\_X`・3 文字形 `\__X`）の字句境界と意味 | `\_a` | 族への言及（1 件に絞れない） | — |
| 角括弧なし `\_` タグ（2 文字形 `\_X`・3 文字形 `\__X`）の字句境界と意味 | `\_n` | `ukadoc:list_sakura_script:_5c_n:1` | `\_n` |
| 角括弧なし `\_` タグ（2 文字形 `\_X`・3 文字形 `\__X`）の字句境界と意味 | `\_q` | `ukadoc:list_sakura_script:_5c_q:1` | `\_q` |
| 角括弧なし `\_` タグ（2 文字形 `\_X`・3 文字形 `\__X`）の字句境界と意味 | `\_s` | `ukadoc:list_sakura_script:_5c_s:1` | `\_s` |

この 21 行から項目 id まで届いたのは **31 件**である。上の表 57 行のうち 10 行は、
書かれた綴りが族を指していて 1 件に絞れないので、id を空のままにしてある
（憶測で近い項目に結び付けない）。

§8 の中には、さくらスクリプトの綴りが**根拠として引かれているだけ**の行も 13 行ある。
主題が別の事柄なので、担当の宣言としては数えていない。

### 表 ⑶-3: brief の件数と台帳の担当の件数の差

brief が挙げた件数と、台帳で担当になった件数は一致しない。増える方向と減る方向の両方に
理由があるので、spec ごとに 3 つの数を並べる。

| brief | 主張が届いた項目 | うち所有の型が付いた項目 | 台帳の担当 |
| --- | ---: | ---: | ---: |
| `areka-P0-anchor-tag-canon` | 22 | 17 | 18 |
| `areka-P0-balloon-canon-residue` | 7 | 5 | 3 |
| `areka-P0-choice-marker-styling` | 10 | 10 | 10 |
| `areka-P0-cursor-tag-canon` | 3 | 2 | 2 |
| `areka-P0-kero-balloon` | 0 | 0 | 3 |
| `areka-P0-makoto-dll-host` | 5 | 3 | 3 |
| `areka-P0-property-query-channels` | 4 | 4 | 3 |
| `areka-P0-sakura-dialogue-tags` | 0 | 0 | 1 |
| `areka-P0-sakura-tag-word-boundary` | 11 | 0 | 0 |
| `areka-P0-sakura-time-directives` | 10 | 9 | 10 |
| `areka-P0-scope-zorder-pinning` | 0 | 0 | 2 |
| `areka-P0-status-execution-states` | 3 | 3 | 3 |
| `areka-P0-surfaces-basepos` | 1 | 1 | 0 |
| `areka-P0-sylphya` | 0 | 0 | 2 |
| `areka-P0-text-decoration-canon` | 20 | 20 | 17 |
| 合計 | 96 | 74 | 77 |

差の中身を 1 件ずつ書き出すと次のとおり。**この表を足し引きすれば 3 つの数はつながる。**

| brief | 項目 | 向き | 理由 |
| --- | --- | --- | --- |
| `areka-P0-anchor-tag-canon` | `\_a[ID]` | 減る | 別名になった項目なので、担当は根の項目（`\_a[ID,r2,r3...]`）に付く |
| `areka-P0-anchor-tag-canon` | `\_a[ID,r2,r3...]` | 増える | brief が族の名前だけを挙げていて、族が小さいので中身の項目まで届いた |
| `areka-P0-anchor-tag-canon` | `\_a[OnID,r0,r1...]` | 増える | brief が族の名前だけを挙げていて、族が小さいので中身の項目まで届いた |
| `areka-P0-balloon-canon-residue` | `\_l[x,y]` | 減る | 2 本以上が主張していて分担が合意済み。作業が残っている `areka-P0-cursor-tag-canon` が担当 |
| `areka-P0-balloon-canon-residue` | `\x` | 減る | 別名になった項目なので、担当は根の項目（`\x[noclear]`）に付く |
| `areka-P0-kero-balloon` | `\0もしくは\h` | 増える | §8 の行が担当を名指ししている |
| `areka-P0-kero-balloon` | `\1もしくは\u` | 増える | §8 の別の行——主題はタグそのものではないが、この項目を実装した spec を名指ししている行——から決まった |
| `areka-P0-kero-balloon` | `\b[ID番号]` | 増える | brief が族の名前だけを挙げていて、族が小さいので中身の項目まで届いた |
| `areka-P0-property-query-channels` | `\![embed,イベント名,r0,r1,r2...]` | 減る | 2 本以上が主張していて分担が未確定。担当は空にして裁定を待つ |
| `areka-P0-sakura-dialogue-tags` | `%username` | 増える | §8 の行が担当を名指ししている |
| `areka-P0-sakura-time-directives` | `\![embed,イベント名,r0,r1,r2...]` | 減る | 2 本以上が主張していて分担が未確定。担当は空にして裁定を待つ |
| `areka-P0-sakura-time-directives` | `\![move]` | 減る | 2 本以上が主張していて分担が未確定。担当は空にして裁定を待つ |
| `areka-P0-sakura-time-directives` | `\![set,balloontimeout,時間]` | 減る | 2 本以上が主張していて分担が合意済み。作業が残っている `areka-P0-balloon-canon-residue` が担当 |
| `areka-P0-sakura-time-directives` | `\_V` | 減る | 別名になった項目なので、担当は根の項目（`\![sound,wait]`）に付く |
| `areka-P0-sakura-time-directives` | `\![quicksection,false]` | 増える | brief が族の名前だけを挙げていて、族が小さいので中身の項目まで届いた |
| `areka-P0-sakura-time-directives` | `\![quicksection,true]` | 増える | brief が族の名前だけを挙げていて、族が小さいので中身の項目まで届いた |
| `areka-P0-sakura-time-directives` | `\![set,scaling,倍率]` | 増える | brief が族の名前だけを挙げていて、族が小さいので中身の項目まで届いた |
| `areka-P0-sakura-time-directives` | `\![set,scaling,横倍率,縦倍率,オプション]` | 増える | brief が族の名前だけを挙げていて、族が小さいので中身の項目まで届いた |
| `areka-P0-sakura-time-directives` | `\_q` | 増える | §8 の行が担当を名指ししている |
| `areka-P0-scope-zorder-pinning` | `\![reset,zorder]` | 増える | §8 の行が担当を名指ししている |
| `areka-P0-scope-zorder-pinning` | `\![set,zorder,スコープID,スコープID,...]` | 増える | §8 の行が担当を名指ししている |
| `areka-P0-surfaces-basepos` | `\![move]` | 減る | 2 本以上が主張していて分担が未確定。担当は空にして裁定を待つ |
| `areka-P0-sylphya` | `%keroname` | 増える | §8 の行が担当を名指ししている |
| `areka-P0-sylphya` | `%selfname2` | 増える | §8 の行が担当を名指ししている |
| `areka-P0-text-decoration-canon` | `\_l[x,y]` | 減る | 2 本以上が主張していて分担が合意済み。作業が残っている `areka-P0-cursor-tag-canon` が担当 |
| `areka-P0-text-decoration-canon` | `\x` | 減る | 別名になった項目なので、担当は根の項目（`\x[noclear]`）に付く |
| `areka-P0-text-decoration-canon` | `\x[noclear]` | 減る | 2 本以上が主張していて分担が合意済み。作業が残っている `areka-P0-balloon-canon-residue` が担当 |

台帳で担当が入っているのは **77 件**、空は **265 件**である。
担当が入った 77 件の決まり方は、1 本だけが主張している 66 件・分担が合意済みで作業が残る側 3 件・
実装済みの項目に「誰が実装したか」を記録した 8 件である。

> **2026-09-05 の見直しで 1 件減った。** `\_V` が別名になったので、それまで担当だった
> `areka-P0-sakura-time-directives` の件数が 11 から 10 へ、全体が 78 から 77 へ変わった。
> 担当は根の `\![sound,wait]` の側で読む。

### 表 ⑶-4: どの項目にも当たらない綴り 20 種

brief に書かれている綴りのうち、正典のどの項目にも当たらないものが 20 種ある。
**憶測で近い項目に結び付けていない。** 当たらなかったという事実のまま残してある。

| 綴り | 分類 | 書いている brief | 備考 |
| --- | --- | --- | --- |
| `%APPDATA` | 本ドメインの綴りではないもの（拾い過ぎ） | `areka-P0-ukadoc-survey-toolkit` | 正規表現やパスの断片・環境変数・説明のための代表名として現れたもので、さくらスクリプトの綴りとして書かれたものではない |
| `%LOCALAPPDATA` | 本ドメインの綴りではないもの（拾い過ぎ） | `areka-P0-dpi-transition-atomicity`・`areka-P0-recompose-budget` | 正規表現やパスの断片・環境変数・説明のための代表名として現れたもので、さくらスクリプトの綴りとして書かれたものではない |
| `%keyword` | 本ドメインの綴りではないもの（拾い過ぎ） | `areka-P0-ukadoc-survey-sakura-script` | 正規表現やパスの断片・環境変数・説明のための代表名として現れたもので、さくらスクリプトの綴りとして書かれたものではない |
| `%m` | 本ドメインの綴りではないもの（拾い過ぎ） | `areka-P0-sylphya` | 正規表現やパスの断片・環境変数・説明のための代表名として現れたもので、さくらスクリプトの綴りとして書かれたものではない |
| `\![enter,nouserbreak]` | 本ドメインの綴りのつもりで書かれているが、正典に無い綴り | `areka-P0-status-execution-states` | 正典の綴りは `\![enter,nouserbreakmode]` |
| `\![vanish]` | 本ドメインの綴りのつもりで書かれているが、正典に無い綴り | `areka-P0-position-persist`・`areka-P0-sylphya` | 正典の綴りは `vanishbymyself`（`\![vanishbymyself]` の 1 件）。書いているのは完了済みの 2 本 |
| `\2` | 本ドメインの綴りではないもの（拾い過ぎ） | `areka-P0-dpi-transition-atomicity`・`areka-P0-recompose-budget` | 正規表現やパスの断片・環境変数・説明のための代表名として現れたもので、さくらスクリプトの綴りとして書かれたものではない |
| `\A` | 本ドメインの綴りではないもの（拾い過ぎ） | `areka-P0-draw-load-parity` | 正規表現やパスの断片・環境変数・説明のための代表名として現れたもので、さくらスクリプトの綴りとして書かれたものではない |
| `\L` | 本ドメインの綴りではないもの（拾い過ぎ） | `areka-P0-draw-load-parity` | 正規表現やパスの断片・環境変数・説明のための代表名として現れたもので、さくらスクリプトの綴りとして書かれたものではない |
| `\U` | 本ドメインの綴りではないもの（拾い過ぎ） | `areka-P0-draw-load-parity` | 正規表現やパスの断片・環境変数・説明のための代表名として現れたもので、さくらスクリプトの綴りとして書かれたものではない |
| `\_X` | 綴りではなく「書き方の記法」 | `areka-P0-anchor-tag-canon`・`areka-P0-sakura-bare-tag-lexer`・`areka-P0-sakura-tag-word-boundary` | 角括弧を使わない `\_` タグの 2 文字形を表す書き方であって、正典の項目ではない |
| `\_X[...]` | 綴りではなく「書き方の記法」 | `areka-P0-sakura-bare-tag-lexer` | 角括弧を取る形を表す書き方であって、正典の項目ではない |
| `\__X` | 綴りではなく「書き方の記法」 | `areka-P0-anchor-tag-canon`・`areka-P0-sakura-tag-word-boundary` | 角括弧を使わない `\_` タグの 3 文字形を表す書き方であって、正典の項目ではない |
| `\_a[id]` | 綴りではなく「書き方の記法」 | `areka-P0-anchor-tag-canon`・`areka-P0-sakura-bare-tag-lexer` | `\_a[ID]` の大文字小文字違い。正典は `ID` |
| `\d` | 本ドメインの綴りではないもの（拾い過ぎ） | `areka-P0-dpi-window-vanish`・`areka-P0-kero-balloon`・`areka-P0-ukadoc-survey-toolkit`・`wintf-gpu-test-crash` | 正規表現やパスの断片・環境変数・説明のための代表名として現れたもので、さくらスクリプトの綴りとして書かれたものではない |
| `\g` | 本ドメインの綴りではないもの（拾い過ぎ） | `areka-P0-draw-load-parity`・`areka-P0-log-capture-determinism`・`wintf-gpu-test-crash` | 正規表現やパスの断片・環境変数・説明のための代表名として現れたもので、さくらスクリプトの綴りとして書かれたものではない |
| `\o` | 本ドメインの綴りではないもの（拾い過ぎ） | `areka-P0-dpi-transition-atomicity` | 正規表現やパスの断片・環境変数・説明のための代表名として現れたもので、さくらスクリプトの綴りとして書かれたものではない |
| `\r` | 本ドメインの綴りではないもの（拾い過ぎ） | `areka-P0-makoto-dll-host` | 正規表現やパスの断片・環境変数・説明のための代表名として現れたもので、さくらスクリプトの綴りとして書かれたものではない |
| `quicksession` | 本ドメインの綴りのつもりで書かれているが、正典に無い綴り | `areka-P0-ukadoc-survey-sakura-script` | 正典の綴りは `quicksection`（`\![quicksection,true]`・`\![quicksection,false]` の 2 件） |
| `vanish` | 本ドメインの綴りのつもりで書かれているが、正典に無い綴り | `areka-P0-ukadoc-survey-sakura-script` | 正典の綴りは `vanishbymyself`（`\![vanishbymyself]` の 1 件） |

分類で数えると、正典に無い綴り **4 種**・書き方の記法 **4 種**・拾い過ぎ **12 種**である。
このうち `\![enter,nouserbreak]` は、担当を主張している brief 自身が書いている綴りである。
直し方の候補は ⑹ 節で扱う。

### この節を読むときの断り

3 つある。

1. **表 ⑶-1 の「主張の型」は、機械の第 1 判定をそのまま出したものではない。** 作業用の
   `owner-map.tsv` は「brief の本文のどこかに綴りが出る」だけで主張とみなす粗い判定を持っており、
   その判定では欠陥の実例やフィクスチャの棚卸しまで主張に数えてしまう。表 ⑶-1 の型は、
   行ごとに読み直した `kind.tsv` の型で**上書き**してある。担当になるかどうかはこの型で決まる。
2. **担当の一覧はこの文書にしかない。** 機械で作り直す報告には担当の列が無い。
   台帳を直接読めば `owner` の欄はあるが、誰がどれを持っているかを一覧で読める場所は
   この ⑶ 節と ⑷ 節だけである。
3. **担当の欄は brief の型だけで決まらず、`doc/COMPAT_ARCHITECTURE.md` の §8 の行からも入る。**
   表 ⑶-1 で型が「例示」なのに担当が入っている行があるのはそのためで、内訳は表 ⑶-3 にある。

---

## ⑷ 担当なし・裁定待ちの一覧

この節は続きの作業（タスク 4.2）で書く。

---

## ⑸ 「書いてあるのに何も起きない」順の未対応一覧

この節は続きの作業（タスク 4.2）で書く。

---

## ⑹ 既存の brief と `doc/COMPAT_ARCHITECTURE.md` §8 への是正候補

この節は続きの作業（タスク 4.2）で書く。

---

## ⑺ カタログとの件数の差

台帳の項目とカタログの項目を、id の集合として両方向で突き合わせた。

| 比べたもの | 件数 | 台帳にだけある | カタログにだけある |
| --- | ---: | ---: | ---: |
| 台帳 ⇔ カタログ（`list_sakura_script` のページ） | 342 / 342 | **0 件** | **0 件** |

**差は 0 件である。** 台帳にだけある項目も 0 件、カタログにだけある項目も 0 件で、
342 件がそのまま 342 件に対応する。

この 0 は引き算で導いたものではなく、集合の差を両方向で取って数えた結果である。
片方向だけを見ると「台帳に余分が無い」ことしか言えず、「カタログの取りこぼしが無い」ことは
言えないので、必ず両方向で取っている。

調べ方は、台帳のファイルから `[entry."…"]` の見出しを全部拾って集合にし、
カタログのファイルから `list_sakura_script` のページに属する id を全部拾って集合にし、
2 つの集合の差を両方向で取る、というものである。id の文字列は 1 文字も直さずに比べている。

台帳を作り始める前の検分では、この 2 つに加えて正典のスナップショットからも直接 id を取り出し、
**3 つの集合が両方向とも差 0 で一致する**ことを確かめてある。

| 比べたもの | 件数 | 差 |
| --- | ---: | ---: |
| 台帳 ⇔ カタログ | 342 / 342 | 0 件 |
| 台帳 ⇔ スナップショット | 342 / 342 | 0 件 |
| カタログ ⇔ スナップショット | 342 / 342 | 0 件 |

**したがって「カタログを正として差の中身を書く」場面は起きなかった。**
要件はカタログと食い違った場合にカタログを正とし、食い違いの中身をここへ書くと定めているが、
その食い違いが **0 件**だったので、書くべき中身は無い。
無いことを黙って省くと「まだ調べていない」のと見分けが付かないので、
**0 件であることをここに明示して残す**。

