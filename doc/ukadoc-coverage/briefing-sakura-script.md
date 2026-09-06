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
| `areka-P0-balloon-canon-residue` | `\_l[x,y]` | 減る | 2 本以上が主張していて分担が合意済み。実装はもう済んでいるので、担当は実装した `areka-P0-cursor-tag-canon` の記録である |
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
| `areka-P0-text-decoration-canon` | `\_l[x,y]` | 減る | 2 本以上が主張していて分担が合意済み。実装はもう済んでいるので、担当は実装した `areka-P0-cursor-tag-canon` の記録である |
| `areka-P0-text-decoration-canon` | `\x` | 減る | 別名になった項目なので、担当は根の項目（`\x[noclear]`）に付く |
| `areka-P0-text-decoration-canon` | `\x[noclear]` | 減る | 2 本以上が主張していて分担が合意済み。作業が残っている `areka-P0-balloon-canon-residue` が担当 |

台帳で担当が入っているのは **77 件**、空は **265 件**である。
担当が入った 77 件の決まり方は、1 本だけが主張している 66 件・分担が合意済みで作業が残る側 2 件・
分担が合意済みで実装がもう済んでいる 1 件・
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

台帳で担当の欄が空なのは **265 件**（担当が入っているのは 77 件）。
空になった理由は 1 つではないので、理由別に並べる。
**空の理由が「もう作業が残っていない」ものと「引き受け手がいない」ものは、まったく別である。**

| 空になった理由 | 件数 | 読み方 |
| --- | ---: | --- |
| 順 1: 対象外 | 1 | この調査の対象外。作業は残らない |
| 順 1: 綴りを受ける別名 | 17 | 別名で、areka はその綴りを正典の根と同じに扱う。作業は残らない |
| 順 2: 裁定待ち | 2 | 2 本が主張していて分担が決まっていない。裁定を待つ |
| 順 7: 所有者なし（綴りを受けない別名） | 3 | 別名だが areka はその綴りを受けない。綴りを根へ写す作業が残り、利用者に見える壊れ方もある |
| 順 7: 所有者なし | 242 | どの brief も対応表 §8 も、この項目の担当を宣言していない |
| **合計** | **265** | |

**「未実装なのに、主張しているのが完了済みの spec だけ」という理由で空になった項目は 0 件である。**
決定表はその場合も担当を空にすると定めているが、当たる項目が 1 件も無かった。
0 を黙って省くと「まだ調べていない」のと見分けが付かないので、ここに明示して残す。

### 作業が残らないので空にした 18 件

この 18 件は「引き受け手がいない」ではなく「引き受ける作業が無い」。
別名の項目の作業は、写像先の根の項目の担当が持っている。

| 項目の綴り | 空にした理由 | 写像先の根 | 台帳の項目 id |
| --- | --- | --- | --- |
| `\7` | areka がこの綴りを根と同じに扱う別名 | `\![executesntp]` | `ukadoc:list_sakura_script:_5c7:1` |
| `\![execute,http-get,URL]` | areka がこの綴りを根と同じに扱う別名 | `\![execute,http-get,URL,パラメータ]` | `ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-get_2cURL_5d:1` |
| `\![reload,descript]` | areka がこの綴りを根と同じに扱う別名 | `\![reload,descript,パラメータ]` | `ukadoc:list_sakura_script:_5c_21_5breload_2cdescript_5d:1` |
| `\![reloadsurface]` | areka がこの綴りを根と同じに扱う別名 | `\![reload,shell]` | `ukadoc:list_sakura_script:_5c_21_5breloadsurface_5d:1` |
| `\![set,scaling,横倍率,縦倍率]` | areka がこの綴りを根と同じに扱う別名 | `\![set,scaling,横倍率,縦倍率,オプション]` | `ukadoc:list_sakura_script:_5c_21_5bset_2cscaling_2c_6a2a_500d_7387_2c_7e26_500d_7387_5d:1` |
| `\_V` | areka がこの綴りを根と同じに扱う別名 | `\![sound,wait]` | `ukadoc:list_sakura_script:_5c_V:1` |
| `\_a[ID]` | areka がこの綴りを根と同じに扱う別名 | `\_a[ID,r2,r3...]` | `ukadoc:list_sakura_script:_5c_a_5bID_5d:1` |
| `\_b[ファイルパス,inline]` | areka がこの綴りを根と同じに扱う別名 | `\_b[ファイルパス,inline,opaque]` | `ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cinline_5d:1` |
| `\_b[ファイルパス,x,y]` | areka がこの綴りを根と同じに扱う別名 | `\_b[ファイルパス,x,y,opaque]` | `ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cx_2cy_5d:1` |
| `\_s` | areka がこの綴りを根と同じに扱う別名 | `\_s[ID1,ID2,ID3...]` | `ukadoc:list_sakura_script:_5c_s:1` |
| `\_v[ファイル名]` | areka がこの綴りを根と同じに扱う別名 | `\![sound,play,ファイル名,オプション...]` | `ukadoc:list_sakura_script:_5c_v_5b_30d5_30a1_30a4_30eb_540d_5d:1` |
| `\bID番号` | areka がこの綴りを根と同じに扱う別名 | `\b[ID番号]` | `ukadoc:list_sakura_script:_5cbID_756a_53f7:1` |
| `\c[char,数値]` | areka がこの綴りを根と同じに扱う別名 | `\c[char,数値,開始位置]` | `ukadoc:list_sakura_script:_5cc_5bchar_2c_6570_5024_5d:1` |
| `\c[line,数値]` | areka がこの綴りを根と同じに扱う別名 | `\c[line,数値,開始位置]` | `ukadoc:list_sakura_script:_5cc_5bline_2c_6570_5024_5d:1` |
| `\pID番号` | areka がこの綴りを根と同じに扱う別名 | `\p[ID番号]` | `ukadoc:list_sakura_script:_5cpID_756a_53f7:1` |
| `\q[タイトル,ID]` | areka がこの綴りを根と同じに扱う別名 | `\q[タイトル,ID,r2,r3...]` | `ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cID_5d:1` |
| `\x` | areka がこの綴りを根と同じに扱う別名 | `\x[noclear]` | `ukadoc:list_sakura_script:_5cx:1` |
| `環境変数の記述例` | この調査の対象外 | — | `ukadoc:list_sakura_script:_74b0_5883_5909_6570_306e_8a18_8ff0_4f8b:1` |

### 裁定待ちの 2 件

どちらも 2 本の brief が同じ項目を主張していて、分担が決まっていない。
**下に添える裁定案は案であって決定ではない。** 決めるのは統合担当（`areka-P0-ukadoc-coverage-roadmap`）である。
この 2 件の理由は同じではないので、書き分けてある。

#### `\![embed,イベント名,r0,r1,r2...]`（別のイベントが返す台詞をその場に埋め込む）

台帳の項目 id: `ukadoc:list_sakura_script:_5c_21_5bembed_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1`

| 主張している brief | 主張の中身 |
| --- | --- |
| `areka-P0-property-query-channels` | このタグは呼んだ結果でタグ自身が置き換わる経路なので、照会の配管として自分が持つ |
| `areka-P0-sakura-time-directives` | 台本を読む側の受理一覧にこの名前が入っており、その一覧ごと自分が持つ |

**分担が決まっていない理由**: **名指しは一方向で、分担が決まっていない。** `areka-P0-property-query-channels` の brief は `areka-P0-sakura-time-directives` を隣り合う spec として名指ししているが、その名指しは「層が違うのでぶつからない」と述べるだけで、**どちらがこの項目を持つかを決めていない**。相手側は名指しを返していない。

**裁定案（案であって決定ではない）**: 台本を読む側の受理一覧は `areka-P0-sakura-time-directives` が持ち、返ってきた結果でタグを置き換える実行時の経路は `areka-P0-property-query-channels` が持つ。

#### `\![move]`（窓を動かす）

台帳の項目 id: `ukadoc:list_sakura_script:_5c_21_5bmove_5d:1`

| 主張している brief | 主張の中身 |
| --- | --- |
| `areka-P0-surfaces-basepos` | 基準位置の解決（`base` の引数）を自分が持つ |
| `areka-P0-sakura-time-directives` | 時間の引数の枠を自分が持つ |

**分担が決まっていない理由**: **互いに相手を知らない。** 2 本の brief は互いを 1 度も名指ししておらず（両方向とも 0 件）、対応表 §8 のこのタグの行も、完了済みの spec を担当に書くだけで、2 本の住み分けを登記していない。

**裁定案（案であって決定ではない）**: 引数ごとに分ける。基準位置は `areka-P0-surfaces-basepos`、時間は `areka-P0-sakura-time-directives` が持つ。

### 別名だが areka がその綴りを受けない 3 件

別名の行なのに担当が空だが、理由は「作業が残らない」ではなく「所有者なし」である。
areka はこの 3 件の綴りを受けないので、綴りを根へ写す作業が残っており、利用者に見える壊れ方もある。
壊れ方は ⑸ 節の末尾に群として載せた。

| 項目の綴り | 写像先の根 | 台帳の項目 id |
| --- | --- | --- |
| `\q[ID][タイトル]または\q*[ID][タイトル]` | `\q[タイトル,ID,r2,r3...]` | `ukadoc:list_sakura_script:_5cq_5bID_5d_5b_30bf_30a4_30c8_30eb_5d_307e_305f_306f_5cq_2a_5bID_5d_5b_30bf_30a4_30c8_30eb_5d:1` |
| `\sID番号` | `\s[ID番号]` | `ukadoc:list_sakura_script:_5csID_756a_53f7:1` |
| `\z` | `\e` | `ukadoc:list_sakura_script:_5cz:1` |

### 所有者がいない 242 件

どの brief も対応表 §8 も、この 242 件の担当を宣言していない。
台帳の状態で割ると次のとおりである。

| 台帳の状態 | 件数 |
| --- | ---: |
| 未対応（書いてあるのに何も起きない） | 204 |
| 語彙だけが登記されている | 23 |
| 実装済み | 13 |
| 一部だけが効かない | 2 |
| **合計** | **242** |

全数を項目の id まで並べる。

| 項目の綴り | 台帳の状態 | 群 | 台帳の項目 id |
| --- | --- | --- | --- |
| `%*` | 語彙だけが登記されている | 構文記録 | `ukadoc:list_sakura_script:_25_2a:1` |
| `%day` | 語彙だけが登記されている | 暦時計 | `ukadoc:list_sakura_script:_25day:1` |
| `%dms` | 語彙だけが登記されている | 単語ランダム系 | `ukadoc:list_sakura_script:_25dms:1` |
| `%et` | 語彙だけが登記されている | 画面と OS 起動時間・時刻ネタ | `ukadoc:list_sakura_script:_25et:1` |
| `%exh` | 語彙だけが登記されている | 画面と OS 起動時間・時刻ネタ | `ukadoc:list_sakura_script:_25exh:1` |
| `%hour` | 語彙だけが登記されている | 暦時計 | `ukadoc:list_sakura_script:_25hour:1` |
| `%lastghostname` | 語彙だけが登記されている | インストール文脈 | `ukadoc:list_sakura_script:_25lastghostname:1` |
| `%lastobjectname` | 語彙だけが登記されている | インストール文脈 | `ukadoc:list_sakura_script:_25lastobjectname:1` |
| `%m?` | 語彙だけが登記されている | 単語ランダム系 | `ukadoc:list_sakura_script:_25m_3f:1` |
| `%mc` | 語彙だけが登記されている | 単語ランダム系 | `ukadoc:list_sakura_script:_25mc:1` |
| `%me` | 語彙だけが登記されている | 単語ランダム系 | `ukadoc:list_sakura_script:_25me:1` |
| `%mh` | 語彙だけが登記されている | 単語ランダム系 | `ukadoc:list_sakura_script:_25mh:1` |
| `%minute` | 語彙だけが登記されている | 暦時計 | `ukadoc:list_sakura_script:_25minute:1` |
| `%ml` | 語彙だけが登記されている | 単語ランダム系 | `ukadoc:list_sakura_script:_25ml:1` |
| `%month` | 語彙だけが登記されている | 暦時計 | `ukadoc:list_sakura_script:_25month:1` |
| `%mp` | 語彙だけが登記されている | 単語ランダム系 | `ukadoc:list_sakura_script:_25mp:1` |
| `%ms` | 語彙だけが登記されている | 単語ランダム系 | `ukadoc:list_sakura_script:_25ms:1` |
| `%mt` | 語彙だけが登記されている | 単語ランダム系 | `ukadoc:list_sakura_script:_25mt:1` |
| `%mz` | 語彙だけが登記されている | 単語ランダム系 | `ukadoc:list_sakura_script:_25mz:1` |
| `%screenheight` | 語彙だけが登記されている | 画面と OS 起動時間・時刻ネタ | `ukadoc:list_sakura_script:_25screenheight:1` |
| `%screenwidth` | 語彙だけが登記されている | 画面と OS 起動時間・時刻ネタ | `ukadoc:list_sakura_script:_25screenwidth:1` |
| `%second` | 語彙だけが登記されている | 暦時計 | `ukadoc:list_sakura_script:_25second:1` |
| `%selfname` | 実装済み | — | `ukadoc:list_sakura_script:_25selfname:1` |
| `%wronghour` | 語彙だけが登記されている | 画面と OS 起動時間・時刻ネタ | `ukadoc:list_sakura_script:_25wronghour:1` |
| `\-` | 実装済み | — | `ukadoc:list_sakura_script:_5c-:1` |
| `\4` | 未対応（書いてあるのに何も起きない） | キャラの移動と重なり | `ukadoc:list_sakura_script:_5c4:1` |
| `\5` | 未対応（書いてあるのに何も起きない） | キャラの移動と重なり | `ukadoc:list_sakura_script:_5c5:1` |
| `\6` | 未対応（書いてあるのに何も起きない） | 時計合わせ | `ukadoc:list_sakura_script:_5c6:1` |
| `\8[ファイル名]` | 未対応（書いてあるのに何も起きない） | 音の再生 | `ukadoc:list_sakura_script:_5c8_5b_30d5_30a1_30a4_30eb_540d_5d:1` |
| `\C` | 未対応（書いてあるのに何も起きない） | バルーンの追記と選択肢のタイムアウト抑止 | `ukadoc:list_sakura_script:_5cC:1` |
| `\![*]` | 未対応（書いてあるのに何も起きない） | 選択肢マーカーの表示 | `ukadoc:list_sakura_script:_5c_21_5b_2a_5d:1` |
| `\![anim,add,overlay,ID]` | 未対応（書いてあるのに何も起きない） | サーフェスアニメーション | `ukadoc:list_sakura_script:_5c_21_5banim_2cadd_2coverlay_2cID_5d:1` |
| `\![anim,add,text,x,y,横幅,縦幅,文字列,表示時間,r,g,b,文字サイズ,文字名]` | 未対応（書いてあるのに何も起きない） | サーフェスアニメーション | `ukadoc:list_sakura_script:_5c_21_5banim_2cadd_2ctext_2cx_2cy_2c_6a2a_5e45_2c_7e26_5e45_2c_6587_5b57_5217_2c_8868_793a_6642_9593_2cr_2cg_2cb_2c_658:1` |
| `\![anim,clear,ID]` | 未対応（書いてあるのに何も起きない） | サーフェスアニメーション | `ukadoc:list_sakura_script:_5c_21_5banim_2cclear_2cID_5d:1` |
| `\![anim,offset,ID,x座標,y座標]` | 未対応（書いてあるのに何も起きない） | サーフェスアニメーション | `ukadoc:list_sakura_script:_5c_21_5banim_2coffset_2cID_2cx_5ea7_6a19_2cy_5ea7_6a19_5d:1` |
| `\![anim,pause,ID]` | 未対応（書いてあるのに何も起きない） | サーフェスアニメーション | `ukadoc:list_sakura_script:_5c_21_5banim_2cpause_2cID_5d:1` |
| `\![anim,resume,ID]` | 未対応（書いてあるのに何も起きない） | サーフェスアニメーション | `ukadoc:list_sakura_script:_5c_21_5banim_2cresume_2cID_5d:1` |
| `\![anim,stop,ID]` | 未対応（書いてあるのに何も起きない） | サーフェスアニメーション | `ukadoc:list_sakura_script:_5c_21_5banim_2cstop_2cID_5d:1` |
| `\![biff(,アカウント名)]` | 未対応（書いてあるのに何も起きない） | メールチェック | `ukadoc:list_sakura_script:_5c_21_5bbiff_28_2c_30a2_30ab_30a6_30f3_30c8_540d_29_5d:1` |
| `\![bind-noevent,カテゴリ名,パーツ名,数値]` | 未対応（書いてあるのに何も起きない） | 着せ替え | `ukadoc:list_sakura_script:_5c_21_5bbind-noevent_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_6570_5024_5d:1` |
| `\![bind,カテゴリ名,パーツ名,数値]` | 実装済み | — | `ukadoc:list_sakura_script:_5c_21_5bbind_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_6570_5024_5d:1` |
| `\![call,ghost,ゴースト名(,--option=raise-event)]` | 未対応（書いてあるのに何も起きない） | ゴーストの切り替え | `ukadoc:list_sakura_script:_5c_21_5bcall_2cghost_2c_30b4_30fc_30b9_30c8_540d_28_2c--option_3draise-event_29_5d:1` |
| `\![cancel,http,URL]` | 未対応（書いてあるのに何も起きない） | WebSocket と通信の中止 | `ukadoc:list_sakura_script:_5c_21_5bcancel_2chttp_2cURL_5d:1` |
| `\![cancel,websocket,URL]` | 未対応（書いてあるのに何も起きない） | WebSocket と通信の中止 | `ukadoc:list_sakura_script:_5c_21_5bcancel_2cwebsocket_2cURL_5d:1` |
| `\![change,balloon,バルーン名]` | 未対応（書いてあるのに何も起きない） | バルーンの見た目と切り替え | `ukadoc:list_sakura_script:_5c_21_5bchange_2cballoon_2c_30d0_30eb_30fc_30f3_540d_5d:1` |
| `\![change,ghost,ゴースト名(,--option=raise-event)]` | 未対応（書いてあるのに何も起きない） | ゴーストの切り替え | `ukadoc:list_sakura_script:_5c_21_5bchange_2cghost_2c_30b4_30fc_30b9_30c8_540d_28_2c--option_3draise-event_29_5d:1` |
| `\![change,shell,シェル名(,--option=raise-event)]` | 未対応（書いてあるのに何も起きない） | ゴーストの切り替え | `ukadoc:list_sakura_script:_5c_21_5bchange_2cshell_2c_30b7_30a7_30eb_540d_28_2c--option_3draise-event_29_5d:1` |
| `\![close,communicatebox]` | 未対応（書いてあるのに何も起きない） | 通信箱と教え込み箱 | `ukadoc:list_sakura_script:_5c_21_5bclose_2ccommunicatebox_5d:1` |
| `\![close,dialog,ID]` | 未対応（書いてあるのに何も起きない） | 入力窓とダイアログ | `ukadoc:list_sakura_script:_5c_21_5bclose_2cdialog_2cID_5d:1` |
| `\![close,inputbox,ID]` | 未対応（書いてあるのに何も起きない） | 入力窓とダイアログ | `ukadoc:list_sakura_script:_5c_21_5bclose_2cinputbox_2cID_5d:1` |
| `\![close,teachbox]` | 未対応（書いてあるのに何も起きない） | 通信箱と教え込み箱 | `ukadoc:list_sakura_script:_5c_21_5bclose_2cteachbox_5d:1` |
| `\![close,websocket,URL]` | 未対応（書いてあるのに何も起きない） | ネットワーク通信 | `ukadoc:list_sakura_script:_5c_21_5bclose_2cwebsocket_2cURL_5d:1` |
| `\![create,shortcut]` | 未対応（書いてあるのに何も起きない） | OS まわりの操作 | `ukadoc:list_sakura_script:_5c_21_5bcreate_2cshortcut_5d:1` |
| `\![effect2,追加サーフェスID,プラグイン名,速度倍率,パラメータ]` | 未対応（書いてあるのに何も起きない） | 画面効果のプラグイン | `ukadoc:list_sakura_script:_5c_21_5beffect2_2c_8ffd_52a0_30b5_30fc_30d5_30a7_30b9ID_2c_30d7_30e9_30b0_30a4_30f3_540d_2c_901f_5ea6_500d_7387_2c_30d1:1` |
| `\![effect,プラグイン名,速度倍率,パラメータ]` | 未対応（書いてあるのに何も起きない） | 画面効果のプラグイン | `ukadoc:list_sakura_script:_5c_21_5beffect_2c_30d7_30e9_30b0_30a4_30f3_540d_2c_901f_5ea6_500d_7387_2c_30d1_30e9_30e1_30fc_30bf_5d:1` |
| `\![enter,collisionmode]\![enter,collisionmode,rect]` | 未対応（書いてあるのに何も起きない） | 動作モードの出入り | `ukadoc:list_sakura_script:_5c_21_5benter_2ccollisionmode_5d_5c_21_5benter_2ccollisionmode_2crect_5d:1` |
| `\![enter,nouserbreakmode]` | 未対応（書いてあるのに何も起きない） | 動作モードの出入り | `ukadoc:list_sakura_script:_5c_21_5benter_2cnouserbreakmode_5d:1` |
| `\![enter,onlinemode]` | 未対応（書いてあるのに何も起きない） | 動作モードの出入り | `ukadoc:list_sakura_script:_5c_21_5benter_2conlinemode_5d:1` |
| `\![enter,selectmode,モード(rect),左,上,右,下]\![enter,selectmode,モード(rect),当たり判定名]\![enter,selectmode,モード(rect)]` | 未対応（書いてあるのに何も起きない） | 動作モードの出入り | `ukadoc:list_sakura_script:_5c_21_5benter_2cselectmode_2c_30e2_30fc_30c9_28rect_29_2c_5de6_2c_4e0a_2c_53f3_2c_4e0b_5d_5c_21_5benter_2cselectmode_2c:1` |
| `\![execute,compressarchive,ファイル名,ディレクトリ名,オプション...]` | 未対応（書いてあるのに何も起きない） | 書庫と配布物の作成 | `ukadoc:list_sakura_script:_5c_21_5bexecute_2ccompressarchive_2c_30d5_30a1_30a4_30eb_540d_2c_30c7_30a3_30ec_30af_30c8_30ea_540d_2c_30aa_30d7_30b7_3:1` |
| `\![execute,createnar]` | 未対応（書いてあるのに何も起きない） | 書庫と配布物の作成 | `ukadoc:list_sakura_script:_5c_21_5bexecute_2ccreatenar_5d:1` |
| `\![execute,createupdatedata]` | 未対応（書いてあるのに何も起きない） | 書庫と配布物の作成 | `ukadoc:list_sakura_script:_5c_21_5bexecute_2ccreateupdatedata_5d:1` |
| `\![execute,dumpsurface,ディレクトリ,スコープID,サーフェスリスト,prefix,イベントID,ゼロ位置切り出し]` | 未対応（書いてあるのに何も起きない） | 開発と診断の窓 | `ukadoc:list_sakura_script:_5c_21_5bexecute_2cdumpsurface_2c_30c7_30a3_30ec_30af_30c8_30ea_2c_30b9_30b3_30fc_30d7ID_2c_30b5_30fc_30d5_30a7_30b9_30e:1` |
| `\![execute,emptyrecyclebin]` | 未対応（書いてあるのに何も起きない） | OS まわりの操作 | `ukadoc:list_sakura_script:_5c_21_5bexecute_2cemptyrecyclebin_5d:1` |
| `\![execute,extractarchive,ファイル名,ディレクトリ名,オプション...]` | 未対応（書いてあるのに何も起きない） | 書庫と配布物の作成 | `ukadoc:list_sakura_script:_5c_21_5bexecute_2cextractarchive_2c_30d5_30a1_30a4_30eb_540d_2c_30c7_30a3_30ec_30af_30c8_30ea_540d_2c_30aa_30d7_30b7_30:1` |
| `\![execute,headline,ヘッドライン名]` | 未対応（書いてあるのに何も起きない） | ヘッドラインセンサ | `ukadoc:list_sakura_script:_5c_21_5bexecute_2cheadline_2c_30d8_30c3_30c9_30e9_30a4_30f3_540d_5d:1` |
| `\![execute,http-delete,URL,オプション,オプション,オプション...]` | 未対応（書いてあるのに何も起きない） | HTTP と RSS の通信 | `ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-delete_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3:1` |
| `\![execute,http-get,URL,オプション,オプション,オプション...]` | 未対応（書いてあるのに何も起きない） | HTTP と RSS の通信 | `ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-get_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3...:1` |
| `\![execute,http-get,URL,パラメータ]` | 未対応（書いてあるのに何も起きない） | HTTP と RSS の通信 | `ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-get_2cURL_2c_30d1_30e9_30e1_30fc_30bf_5d:1` |
| `\![execute,http-head,URL,オプション,オプション,オプション...]` | 未対応（書いてあるのに何も起きない） | HTTP と RSS の通信 | `ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-head_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..:1` |
| `\![execute,http-options,URL,オプション,オプション,オプション...]` | 未対応（書いてあるのに何も起きない） | HTTP と RSS の通信 | `ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-options_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f:1` |
| `\![execute,http-patch,URL,オプション,オプション,オプション...]` | 未対応（書いてあるのに何も起きない） | HTTP と RSS の通信 | `ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-patch_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3.:1` |
| `\![execute,http-post,URL,オプション,オプション,オプション...]` | 未対応（書いてあるのに何も起きない） | HTTP と RSS の通信 | `ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-post_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..:1` |
| `\![execute,http-post,URL,パラメータ]` | 未対応（書いてあるのに何も起きない） | HTTP と RSS の通信 | `ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-post_2cURL_2c_30d1_30e9_30e1_30fc_30bf_5d:1` |
| `\![execute,http-put,URL,オプション,オプション,オプション...]` | 未対応（書いてあるのに何も起きない） | HTTP と RSS の通信 | `ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-put_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3...:1` |
| `\![execute,install,path,ファイル名]` | 未対応（書いてあるのに何も起きない） | インストールと更新 | `ukadoc:list_sakura_script:_5c_21_5bexecute_2cinstall_2cpath_2c_30d5_30a1_30a4_30eb_540d_5d:1` |
| `\![execute,install,url,URL,(feed\|nar\|homeurlのいずれか)]` | 未対応（書いてあるのに何も起きない） | インストールと更新 | `ukadoc:list_sakura_script:_5c_21_5bexecute_2cinstall_2curl_2cURL_2c_28feed_7cnar_7chomeurl_306e_3044_305a_308c_304b_29_5d:1` |
| `\![execute,nslookup,パラメータ1,パラメータ2,...]` | 未対応（書いてあるのに何も起きない） | ネットワークの調べもの | `ukadoc:list_sakura_script:_5c_21_5bexecute_2cnslookup_2c_30d1_30e9_30e1_30fc_30bf1_2c_30d1_30e9_30e1_30fc_30bf2_2c..._5d:1` |
| `\![execute,ping,パラメータ1,パラメータ2,...]` | 未対応（書いてあるのに何も起きない） | ネットワークの調べもの | `ukadoc:list_sakura_script:_5c_21_5bexecute_2cping_2c_30d1_30e9_30e1_30fc_30bf1_2c_30d1_30e9_30e1_30fc_30bf2_2c..._5d:1` |
| `\![execute,resetballoonpos]` | 未対応（書いてあるのに何も起きない） | 窓の位置の初期化 | `ukadoc:list_sakura_script:_5c_21_5bexecute_2cresetballoonpos_5d:1` |
| `\![execute,resetwindowpos]` | 未対応（書いてあるのに何も起きない） | 窓の位置の初期化 | `ukadoc:list_sakura_script:_5c_21_5bexecute_2cresetwindowpos_5d:1` |
| `\![execute,rss-get,URL,オプション,オプション,オプション...]` | 未対応（書いてあるのに何も起きない） | HTTP と RSS の通信 | `ukadoc:list_sakura_script:_5c_21_5bexecute_2crss-get_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._:1` |
| `\![execute,rss-post,URL,オプション,オプション,オプション...]` | 未対応（書いてあるのに何も起きない） | HTTP と RSS の通信 | `ukadoc:list_sakura_script:_5c_21_5bexecute_2crss-post_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3...:1` |
| `\![execute,websocket,URL,オプション,オプション,オプション...]` | 未対応（書いてあるのに何も起きない） | WebSocket と通信の中止 | `ukadoc:list_sakura_script:_5c_21_5bexecute_2cwebsocket_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..:1` |
| `\![executesntp]` | 未対応（書いてあるのに何も起きない） | 時計合わせ | `ukadoc:list_sakura_script:_5c_21_5bexecutesntp_5d:1` |
| `\![filter,プラグイン名,起動時間,パラメータ]` | 未対応（書いてあるのに何も起きない） | 画面効果のプラグイン | `ukadoc:list_sakura_script:_5c_21_5bfilter_2c_30d7_30e9_30b0_30a4_30f3_540d_2c_8d77_52d5_6642_9593_2c_30d1_30e9_30e1_30fc_30bf_5d:1` |
| `\![filter]` | 未対応（書いてあるのに何も起きない） | 画面効果のプラグイン | `ukadoc:list_sakura_script:_5c_21_5bfilter_5d:1` |
| `\![leave,collisionmode]` | 未対応（書いてあるのに何も起きない） | 動作モードの出入り | `ukadoc:list_sakura_script:_5c_21_5bleave_2ccollisionmode_5d:1` |
| `\![leave,inductionmode]` | 未対応（書いてあるのに何も起きない） | 動作モードの出入り | `ukadoc:list_sakura_script:_5c_21_5bleave_2cinductionmode_5d:1` |
| `\![leave,nouserbreakmode]` | 未対応（書いてあるのに何も起きない） | 動作モードの出入り | `ukadoc:list_sakura_script:_5c_21_5bleave_2cnouserbreakmode_5d:1` |
| `\![leave,onlinemode]` | 未対応（書いてあるのに何も起きない） | 動作モードの出入り | `ukadoc:list_sakura_script:_5c_21_5bleave_2conlinemode_5d:1` |
| `\![leave,passivemode]` | 未対応（書いてあるのに何も起きない） | 動作モードの出入り | `ukadoc:list_sakura_script:_5c_21_5bleave_2cpassivemode_5d:1` |
| `\![leave,selectmode]` | 未対応（書いてあるのに何も起きない） | 動作モードの出入り | `ukadoc:list_sakura_script:_5c_21_5bleave_2cselectmode_5d:1` |
| `\![load,shiori]` | 未対応（書いてあるのに何も起きない） | 読み込み直しと差し替え | `ukadoc:list_sakura_script:_5c_21_5bload_2cshiori_5d:1` |
| `\![lock,balloonmove]` | 未対応（書いてあるのに何も起きない） | 描き直しと移動の凍結 | `ukadoc:list_sakura_script:_5c_21_5block_2cballoonmove_5d:1` |
| `\![lock,balloonrepaint]` | 未対応（書いてあるのに何も起きない） | 描き直しと移動の凍結 | `ukadoc:list_sakura_script:_5c_21_5block_2cballoonrepaint_5d:1` |
| `\![lock,repaint]` | 未対応（書いてあるのに何も起きない） | 描き直しと移動の凍結 | `ukadoc:list_sakura_script:_5c_21_5block_2crepaint_5d:1` |
| `\![moveasync]` | 未対応（書いてあるのに何も起きない） | キャラの移動と重なり | `ukadoc:list_sakura_script:_5c_21_5bmoveasync_5d:1` |
| `\![notify,イベント名,r0,r1,r2...]` | 未対応（書いてあるのに何も起きない） | イベントを他所へ起こす | `ukadoc:list_sakura_script:_5c_21_5bnotify_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1` |
| `\![notifyother,ゴースト名,イベント名,r0,r1,r2...]` | 未対応（書いてあるのに何も起きない） | イベントを他所へ起こす | `ukadoc:list_sakura_script:_5c_21_5bnotifyother_2c_30b4_30fc_30b9_30c8_540d_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1` |
| `\![notifyplugin,プラグインのIDまたは名前,イベント名,r0,r1,r2...]` | 未対応（書いてあるのに何も起きない） | イベントを他所へ起こす | `ukadoc:list_sakura_script:_5c_21_5bnotifyplugin_2c_30d7_30e9_30b0_30a4_30f3_306eID_307e_305f_306f_540d_524d_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_:1` |
| `\![open,addressbar]` | 未対応（書いてあるのに何も起きない） | SSP の管理窓とビューア | `ukadoc:list_sakura_script:_5c_21_5bopen_2caddressbar_5d:1` |
| `\![open,aigraph]` | 未対応（書いてあるのに何も起きない） | SSP の管理窓とビューア | `ukadoc:list_sakura_script:_5c_21_5bopen_2caigraph_5d:1` |
| `\![open,archiveviewer,(ファイル名)]` | 未対応（書いてあるのに何も起きない） | SSP の管理窓とビューア | `ukadoc:list_sakura_script:_5c_21_5bopen_2carchiveviewer_2c_28_30d5_30a1_30a4_30eb_540d_29_5d:1` |
| `\![open,backlogviewer]` | 未対応（書いてあるのに何も起きない） | SSP の管理窓とビューア | `ukadoc:list_sakura_script:_5c_21_5bopen_2cbacklogviewer_5d:1` |
| `\![open,balloonexplorer]` | 未対応（書いてあるのに何も起きない） | SSP の管理窓とビューア | `ukadoc:list_sakura_script:_5c_21_5bopen_2cballoonexplorer_5d:1` |
| `\![open,browser,パラメータ]` | 未対応（書いてあるのに何も起きない） | 外部のアプリに渡す | `ukadoc:list_sakura_script:_5c_21_5bopen_2cbrowser_2c_30d1_30e9_30e1_30fc_30bf_5d:1` |
| `\![open,calendar]` | 未対応（書いてあるのに何も起きない） | SSP の管理窓とビューア | `ukadoc:list_sakura_script:_5c_21_5bopen_2ccalendar_5d:1` |
| `\![open,communicatebox]` | 未対応（書いてあるのに何も起きない） | 通信箱と教え込み箱 | `ukadoc:list_sakura_script:_5c_21_5bopen_2ccommunicatebox_5d:1` |
| `\![open,configurationdialog,ダイアログID]` | 未対応（書いてあるのに何も起きない） | SSP の管理窓とビューア | `ukadoc:list_sakura_script:_5c_21_5bopen_2cconfigurationdialog_2c_30c0_30a4_30a2_30ed_30b0ID_5d:1` |
| `\![open,dateinput,ID,表示時間,年,月,日,オプション]` | 未対応（書いてあるのに何も起きない） | 入力窓とダイアログ | `ukadoc:list_sakura_script:_5c_21_5bopen_2cdateinput_2cID_2c_8868_793a_6642_9593_2c_5e74_2c_6708_2c_65e5_2c_30aa_30d7_30b7_30e7_30f3_5d:1` |
| `\![open,developer]` | 未対応（書いてあるのに何も起きない） | 開発と診断の窓 | `ukadoc:list_sakura_script:_5c_21_5bopen_2cdeveloper_5d:1` |
| `\![open,dialog,color,パラメータ]` | 未対応（書いてあるのに何も起きない） | 入力窓とダイアログ | `ukadoc:list_sakura_script:_5c_21_5bopen_2cdialog_2ccolor_2c_30d1_30e9_30e1_30fc_30bf_5d:1` |
| `\![open,dialog,folder,パラメータ]` | 未対応（書いてあるのに何も起きない） | 入力窓とダイアログ | `ukadoc:list_sakura_script:_5c_21_5bopen_2cdialog_2cfolder_2c_30d1_30e9_30e1_30fc_30bf_5d:1` |
| `\![open,dialog,open,パラメータ]` | 未対応（書いてあるのに何も起きない） | 入力窓とダイアログ | `ukadoc:list_sakura_script:_5c_21_5bopen_2cdialog_2copen_2c_30d1_30e9_30e1_30fc_30bf_5d:1` |
| `\![open,dialog,save,パラメータ]` | 未対応（書いてあるのに何も起きない） | 入力窓とダイアログ | `ukadoc:list_sakura_script:_5c_21_5bopen_2cdialog_2csave_2c_30d1_30e9_30e1_30fc_30bf_5d:1` |
| `\![open,dressupexplorer]` | 未対応（書いてあるのに何も起きない） | SSP の管理窓とビューア | `ukadoc:list_sakura_script:_5c_21_5bopen_2cdressupexplorer_5d:1` |
| `\![open,editor,ファイル,表示行]` | 未対応（書いてあるのに何も起きない） | 外部のアプリに渡す | `ukadoc:list_sakura_script:_5c_21_5bopen_2ceditor_2c_30d5_30a1_30a4_30eb_2c_8868_793a_884c_5d:1` |
| `\![open,errorlog]` | 未対応（書いてあるのに何も起きない） | 開発と診断の窓 | `ukadoc:list_sakura_script:_5c_21_5bopen_2cerrorlog_5d:1` |
| `\![open,explorer,ファイル]` | 未対応（書いてあるのに何も起きない） | 外部のアプリに渡す | `ukadoc:list_sakura_script:_5c_21_5bopen_2cexplorer_2c_30d5_30a1_30a4_30eb_5d:1` |
| `\![open,file,ファイル名]` | 未対応（書いてあるのに何も起きない） | 外部のアプリに渡す | `ukadoc:list_sakura_script:_5c_21_5bopen_2cfile_2c_30d5_30a1_30a4_30eb_540d_5d:1` |
| `\![open,ghostexplorer]` | 未対応（書いてあるのに何も起きない） | SSP の管理窓とビューア | `ukadoc:list_sakura_script:_5c_21_5bopen_2cghostexplorer_5d:1` |
| `\![open,headlinesensorexplorer]` | 未対応（書いてあるのに何も起きない） | ヘッドラインセンサ | `ukadoc:list_sakura_script:_5c_21_5bopen_2cheadlinesensorexplorer_5d:1` |
| `\![open,help,ダイアログID]` | 未対応（書いてあるのに何も起きない） | SSP の管理窓とビューア | `ukadoc:list_sakura_script:_5c_21_5bopen_2chelp_2c_30c0_30a4_30a2_30ed_30b0ID_5d:1` |
| `\![open,inputbox,ID,表示時間,テキスト,オプション,...]` | 未対応（書いてあるのに何も起きない） | 入力窓とダイアログ | `ukadoc:list_sakura_script:_5c_21_5bopen_2cinputbox_2cID_2c_8868_793a_6642_9593_2c_30c6_30ad_30b9_30c8_2c_30aa_30d7_30b7_30e7_30f3_2c..._5d:1` |
| `\![open,ipinput,ID,表示時間,IP1桁目,IP2桁目,IP3桁目,IP4桁目,オプション]` | 未対応（書いてあるのに何も起きない） | 入力窓とダイアログ | `ukadoc:list_sakura_script:_5c_21_5bopen_2cipinput_2cID_2c_8868_793a_6642_9593_2cIP1_6841_76ee_2cIP2_6841_76ee_2cIP3_6841_76ee_2cIP4_6841_76ee_2c_3:1` |
| `\![open,mailer,パラメータ]` | 未対応（書いてあるのに何も起きない） | 外部のアプリに渡す | `ukadoc:list_sakura_script:_5c_21_5bopen_2cmailer_2c_30d1_30e9_30e1_30fc_30bf_5d:1` |
| `\![open,messenger]` | 未対応（書いてあるのに何も起きない） | SSP の管理窓とビューア | `ukadoc:list_sakura_script:_5c_21_5bopen_2cmessenger_5d:1` |
| `\![open,passwordinput,ID,表示時間,テキスト,オプション]` | 未対応（書いてあるのに何も起きない） | 入力窓とダイアログ | `ukadoc:list_sakura_script:_5c_21_5bopen_2cpasswordinput_2cID_2c_8868_793a_6642_9593_2c_30c6_30ad_30b9_30c8_2c_30aa_30d7_30b7_30e7_30f3_5d:1` |
| `\![open,pictureviewer,(ファイル名)]` | 未対応（書いてあるのに何も起きない） | SSP の管理窓とビューア | `ukadoc:list_sakura_script:_5c_21_5bopen_2cpictureviewer_2c_28_30d5_30a1_30a4_30eb_540d_29_5d:1` |
| `\![open,pluginexplorer]` | 未対応（書いてあるのに何も起きない） | SSP の管理窓とビューア | `ukadoc:list_sakura_script:_5c_21_5bopen_2cpluginexplorer_5d:1` |
| `\![open,rateofusegraph]` | 未対応（書いてあるのに何も起きない） | SSP の管理窓とビューア | `ukadoc:list_sakura_script:_5c_21_5bopen_2crateofusegraph_5d:1` |
| `\![open,rateofusegraphballoon]` | 未対応（書いてあるのに何も起きない） | SSP の管理窓とビューア | `ukadoc:list_sakura_script:_5c_21_5bopen_2crateofusegraphballoon_5d:1` |
| `\![open,rateofusegraphtotal]` | 未対応（書いてあるのに何も起きない） | SSP の管理窓とビューア | `ukadoc:list_sakura_script:_5c_21_5bopen_2crateofusegraphtotal_5d:1` |
| `\![open,readme]` | 未対応（書いてあるのに何も起きない） | SSP の管理窓とビューア | `ukadoc:list_sakura_script:_5c_21_5bopen_2creadme_5d:1` |
| `\![open,shellexplorer]` | 未対応（書いてあるのに何も起きない） | SSP の管理窓とビューア | `ukadoc:list_sakura_script:_5c_21_5bopen_2cshellexplorer_5d:1` |
| `\![open,shiorirequest]` | 未対応（書いてあるのに何も起きない） | 開発と診断の窓 | `ukadoc:list_sakura_script:_5c_21_5bopen_2cshiorirequest_5d:1` |
| `\![open,sliderinput,ID,表示時間,現在値,最小,最大,オプション]` | 未対応（書いてあるのに何も起きない） | 入力窓とダイアログ | `ukadoc:list_sakura_script:_5c_21_5bopen_2csliderinput_2cID_2c_8868_793a_6642_9593_2c_73fe_5728_5024_2c_6700_5c0f_2c_6700_5927_2c_30aa_30d7_30b7_30:1` |
| `\![open,surfacetest]` | 未対応（書いてあるのに何も起きない） | SSP の管理窓とビューア | `ukadoc:list_sakura_script:_5c_21_5bopen_2csurfacetest_5d:1` |
| `\![open,teachbox]` | 未対応（書いてあるのに何も起きない） | 通信箱と教え込み箱 | `ukadoc:list_sakura_script:_5c_21_5bopen_2cteachbox_5d:1` |
| `\![open,terms]` | 未対応（書いてあるのに何も起きない） | SSP の管理窓とビューア | `ukadoc:list_sakura_script:_5c_21_5bopen_2cterms_5d:1` |
| `\![open,timeinput,ID,表示時間,時,分,秒,オプション]` | 未対応（書いてあるのに何も起きない） | 入力窓とダイアログ | `ukadoc:list_sakura_script:_5c_21_5bopen_2ctimeinput_2cID_2c_8868_793a_6642_9593_2c_6642_2c_5206_2c_79d2_2c_30aa_30d7_30b7_30e7_30f3_5d:1` |
| `\![raise,イベント名,r0,r1,r2...]` | 未対応（書いてあるのに何も起きない） | イベントを他所へ起こす | `ukadoc:list_sakura_script:_5c_21_5braise_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1` |
| `\![raiseother,ゴースト名,イベント名,r0,r1,r2...]` | 未対応（書いてあるのに何も起きない） | イベントを他所へ起こす | `ukadoc:list_sakura_script:_5c_21_5braiseother_2c_30b4_30fc_30b9_30c8_540d_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1` |
| `\![raiseplugin,プラグインのIDまたは名前,イベント名,r0,r1,r2...]` | 未対応（書いてあるのに何も起きない） | イベントを他所へ起こす | `ukadoc:list_sakura_script:_5c_21_5braiseplugin_2c_30d7_30e9_30b0_30a4_30f3_306eID_307e_305f_306f_540d_524d_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2:1` |
| `\![reload,aigraph]` | 未対応（書いてあるのに何も起きない） | 開発と診断の窓 | `ukadoc:list_sakura_script:_5c_21_5breload_2caigraph_5d:1` |
| `\![reload,descript,パラメータ]` | 未対応（書いてあるのに何も起きない） | 読み込み直しと差し替え | `ukadoc:list_sakura_script:_5c_21_5breload_2cdescript_2c_30d1_30e9_30e1_30fc_30bf_5d:1` |
| `\![reload,ghost]` | 未対応（書いてあるのに何も起きない） | 読み込み直しと差し替え | `ukadoc:list_sakura_script:_5c_21_5breload_2cghost_5d:1` |
| `\![reload,shell]` | 未対応（書いてあるのに何も起きない） | 読み込み直しと差し替え | `ukadoc:list_sakura_script:_5c_21_5breload_2cshell_5d:1` |
| `\![reload,shiori]` | 未対応（書いてあるのに何も起きない） | 読み込み直しと差し替え | `ukadoc:list_sakura_script:_5c_21_5breload_2cshiori_5d:1` |
| `\![reset,position]` | 未対応（書いてあるのに何も起きない） | 窓の位置と見え方 | `ukadoc:list_sakura_script:_5c_21_5breset_2cposition_5d:1` |
| `\![reset,sticky-window]` | 未対応（書いてあるのに何も起きない） | 窓の位置と見え方 | `ukadoc:list_sakura_script:_5c_21_5breset_2csticky-window_5d:1` |
| `\![reset,syncobject,同期オブジェクト名]` | 未対応（書いてあるのに何も起きない） | 同期オブジェクト | `ukadoc:list_sakura_script:_5c_21_5breset_2csyncobject_2c_540c_671f_30aa_30d6_30b8_30a7_30af_30c8_540d_5d:1` |
| `\![restore,wallpaper]` | 未対応（書いてあるのに何も起きない） | OS まわりの操作 | `ukadoc:list_sakura_script:_5c_21_5brestore_2cwallpaper_5d:1` |
| `\![save,wallpaper]` | 未対応（書いてあるのに何も起きない） | OS まわりの操作 | `ukadoc:list_sakura_script:_5c_21_5bsave_2cwallpaper_5d:1` |
| `\![send,websocket-binary,URL,base64data]` | 未対応（書いてあるのに何も起きない） | WebSocket と通信の中止 | `ukadoc:list_sakura_script:_5c_21_5bsend_2cwebsocket-binary_2cURL_2cbase64data_5d:1` |
| `\![send,websocket,URL,data1,data2,...]` | 未対応（書いてあるのに何も起きない） | WebSocket と通信の中止 | `ukadoc:list_sakura_script:_5c_21_5bsend_2cwebsocket_2cURL_2cdata1_2cdata2_2c..._5d:1` |
| `\![set,alignmentondesktop,bottomまたはtop]` | 未対応（書いてあるのに何も起きない） | 窓の位置と見え方 | `ukadoc:list_sakura_script:_5c_21_5bset_2calignmentondesktop_2cbottom_307e_305f_306ftop_5d:1` |
| `\![set,alignmenttodesktop,方向]` | 未対応（書いてあるのに何も起きない） | 窓の位置と見え方 | `ukadoc:list_sakura_script:_5c_21_5bset_2calignmenttodesktop_2c_65b9_5411_5d:1` |
| `\![set,alignmenttodesktop,free]` | 未対応（書いてあるのに何も起きない） | 窓の位置と見え方 | `ukadoc:list_sakura_script:_5c_21_5bset_2calignmenttodesktop_2cfree_5d:1` |
| `\![set,autoscroll,disable]` | 未対応（書いてあるのに何も起きない） | バルーンの見た目と切り替え | `ukadoc:list_sakura_script:_5c_21_5bset_2cautoscroll_2cdisable_5d:1` |
| `\![set,autoscroll,enable]` | 未対応（書いてあるのに何も起きない） | バルーンの見た目と切り替え | `ukadoc:list_sakura_script:_5c_21_5bset_2cautoscroll_2cenable_5d:1` |
| `\![set,balloonalign,ID]` | 未対応（書いてあるのに何も起きない） | バルーンの見た目と切り替え | `ukadoc:list_sakura_script:_5c_21_5bset_2cballoonalign_2cID_5d:1` |
| `\![set,balloonmarker,マーカー表示文字列]` | 未対応（書いてあるのに何も起きない） | バルーンの見た目と切り替え | `ukadoc:list_sakura_script:_5c_21_5bset_2cballoonmarker_2c_30de_30fc_30ab_30fc_8868_793a_6587_5b57_5217_5d:1` |
| `\![set,balloonnum,ファイル名,現在の数,最大数]` | 未対応（書いてあるのに何も起きない） | バルーンの見た目と切り替え | `ukadoc:list_sakura_script:_5c_21_5bset_2cballoonnum_2c_30d5_30a1_30a4_30eb_540d_2c_73fe_5728_306e_6570_2c_6700_5927_6570_5d:1` |
| `\![set,balloonoffset,x,y]` | 未対応（書いてあるのに何も起きない） | バルーンの見た目と切り替え | `ukadoc:list_sakura_script:_5c_21_5bset_2cballoonoffset_2cx_2cy_5d:1` |
| `\![set,otherghosttalk,true\|false\|before\|after]` | 未対応（書いてあるのに何も起きない） | 他のゴーストとの付き合い方 | `ukadoc:list_sakura_script:_5c_21_5bset_2cotherghosttalk_2ctrue_7cfalse_7cbefore_7cafter_5d:1` |
| `\![set,othersurfacechange,trueかfalse]` | 未対応（書いてあるのに何も起きない） | 他のゴーストとの付き合い方 | `ukadoc:list_sakura_script:_5c_21_5bset_2cothersurfacechange_2ctrue_304bfalse_5d:1` |
| `\![set,position,x,y,スコープID]` | 未対応（書いてあるのに何も起きない） | 窓の位置と見え方 | `ukadoc:list_sakura_script:_5c_21_5bset_2cposition_2cx_2cy_2c_30b9_30b3_30fc_30d7ID_5d:1` |
| `\![set,serikotalk,true/false]` | 未対応（書いてあるのに何も起きない） | 他のゴーストとの付き合い方 | `ukadoc:list_sakura_script:_5c_21_5bset_2cserikotalk_2ctrue_2ffalse_5d:1` |
| `\![set,shioridebugmode,(true/false)]` | 未対応（書いてあるのに何も起きない） | 開発と診断の窓 | `ukadoc:list_sakura_script:_5c_21_5bset_2cshioridebugmode_2c_28true_2ffalse_29_5d:1` |
| `\![set,sticky-window,スコープID,スコープID,...]` | 未対応（書いてあるのに何も起きない） | 窓の位置と見え方 | `ukadoc:list_sakura_script:_5c_21_5bset_2csticky-window_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c..._5d:1` |
| `\![set,syncobject,同期オブジェクト名]` | 未対応（書いてあるのに何も起きない） | 同期オブジェクト | `ukadoc:list_sakura_script:_5c_21_5bset_2csyncobject_2c_540c_671f_30aa_30d6_30b8_30a7_30af_30c8_540d_5d:1` |
| `\![set,tasktrayicon,ファイル名.ico,テキスト(,--duration=待機時間(,--runcount=繰り返し回数))]` | 未対応（書いてあるのに何も起きない） | 通知領域とデスクトップ | `ukadoc:list_sakura_script:_5c_21_5bset_2ctasktrayicon_2c_30d5_30a1_30a4_30eb_540d.ico_2c_30c6_30ad_30b9_30c8_28_2c--duration_3d_5f85_6a5f_6642_959:1` |
| `\![set,trayballoon,オプション,オプション,オプション...]` | 未対応（書いてあるのに何も起きない） | 通知領域とデスクトップ | `ukadoc:list_sakura_script:_5c_21_5bset_2ctrayballoon_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._5d:1` |
| `\![set,wallpaper,ファイル名,オプション]` | 未対応（書いてあるのに何も起きない） | OS まわりの操作 | `ukadoc:list_sakura_script:_5c_21_5bset_2cwallpaper_2c_30d5_30a1_30a4_30eb_540d_2c_30aa_30d7_30b7_30e7_30f3_5d:1` |
| `\![set,windowstate,!stayontop]` | 未対応（書いてあるのに何も起きない） | 窓の位置と見え方 | `ukadoc:list_sakura_script:_5c_21_5bset_2cwindowstate_2c_21stayontop_5d:1` |
| `\![set,windowstate,minimize]` | 未対応（書いてあるのに何も起きない） | 窓の位置と見え方 | `ukadoc:list_sakura_script:_5c_21_5bset_2cwindowstate_2cminimize_5d:1` |
| `\![set,windowstate,stayontop]` | 未対応（書いてあるのに何も起きない） | 窓の位置と見え方 | `ukadoc:list_sakura_script:_5c_21_5bset_2cwindowstate_2cstayontop_5d:1` |
| `\![sound,cdplay,トラックNo.]` | 未対応（書いてあるのに何も起きない） | 音の再生 | `ukadoc:list_sakura_script:_5c_21_5bsound_2ccdplay_2c_30c8_30e9_30c3_30afNo._5d:1` |
| `\![sound,load,ファイル名,オプション...]` | 未対応（書いてあるのに何も起きない） | 音の再生 | `ukadoc:list_sakura_script:_5c_21_5bsound_2cload_2c_30d5_30a1_30a4_30eb_540d_2c_30aa_30d7_30b7_30e7_30f3..._5d:1` |
| `\![sound,loop,ファイル名]` | 未対応（書いてあるのに何も起きない） | 音の再生 | `ukadoc:list_sakura_script:_5c_21_5bsound_2cloop_2c_30d5_30a1_30a4_30eb_540d_5d:1` |
| `\![sound,option,ファイル名,オプション...]` | 未対応（書いてあるのに何も起きない） | 音の再生 | `ukadoc:list_sakura_script:_5c_21_5bsound_2coption_2c_30d5_30a1_30a4_30eb_540d_2c_30aa_30d7_30b7_30e7_30f3..._5d:1` |
| `\![sound,pause,ファイル名]` | 未対応（書いてあるのに何も起きない） | 音の再生 | `ukadoc:list_sakura_script:_5c_21_5bsound_2cpause_2c_30d5_30a1_30a4_30eb_540d_5d:1` |
| `\![sound,play,ファイル名,オプション...]` | 未対応（書いてあるのに何も起きない） | 音の再生 | `ukadoc:list_sakura_script:_5c_21_5bsound_2cplay_2c_30d5_30a1_30a4_30eb_540d_2c_30aa_30d7_30b7_30e7_30f3..._5d:1` |
| `\![sound,resume,ファイル名]` | 未対応（書いてあるのに何も起きない） | 音の再生 | `ukadoc:list_sakura_script:_5c_21_5bsound_2cresume_2c_30d5_30a1_30a4_30eb_540d_5d:1` |
| `\![sound,stop,ファイル名]` | 未対応（書いてあるのに何も起きない） | 音の再生 | `ukadoc:list_sakura_script:_5c_21_5bsound_2cstop_2c_30d5_30a1_30a4_30eb_540d_5d:1` |
| `\![timernotify,時間,繰り返すか否か,イベント名,r0,r1,r2...]` | 未対応（書いてあるのに何も起きない） | 時間差で起こすイベント | `ukadoc:list_sakura_script:_5c_21_5btimernotify_2c_6642_9593_2c_7e70_308a_8fd4_3059_304b_5426_304b_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1` |
| `\![timernotifyother,時間,繰り返すか否か,ゴースト名,イベント名,r0,r1,r2...]` | 未対応（書いてあるのに何も起きない） | 時間差で起こすイベント | `ukadoc:list_sakura_script:_5c_21_5btimernotifyother_2c_6642_9593_2c_7e70_308a_8fd4_3059_304b_5426_304b_2c_30b4_30fc_30b9_30c8_540d_2c_30a4_30d9_30:1` |
| `\![timernotifyplugin,時間,繰り返すか否か,プラグインのIDまたは名前,イベント名,r0,r1,r2...]` | 未対応（書いてあるのに何も起きない） | 時間差で起こすイベント | `ukadoc:list_sakura_script:_5c_21_5btimernotifyplugin_2c_6642_9593_2c_7e70_308a_8fd4_3059_304b_5426_304b_2c_30d7_30e9_30b0_30a4_30f3_306eID_307e_30:1` |
| `\![timerraise,時間,繰り返すか否か,イベント名,r0,r1,r2...]` | 未対応（書いてあるのに何も起きない） | 時間差で起こすイベント | `ukadoc:list_sakura_script:_5c_21_5btimerraise_2c_6642_9593_2c_7e70_308a_8fd4_3059_304b_5426_304b_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1` |
| `\![timerraiseother,時間,繰り返すか否か,ゴースト名,イベント名,r0,r1,r2...]` | 未対応（書いてあるのに何も起きない） | 時間差で起こすイベント | `ukadoc:list_sakura_script:_5c_21_5btimerraiseother_2c_6642_9593_2c_7e70_308a_8fd4_3059_304b_5426_304b_2c_30b4_30fc_30b9_30c8_540d_2c_30a4_30d9_30f:1` |
| `\![timerraiseplugin,時間,繰り返すか否か,プラグインのIDまたは名前,イベント名,r0,r1,r2...]` | 未対応（書いてあるのに何も起きない） | 時間差で起こすイベント | `ukadoc:list_sakura_script:_5c_21_5btimerraiseplugin_2c_6642_9593_2c_7e70_308a_8fd4_3059_304b_5426_304b_2c_30d7_30e9_30b0_30a4_30f3_306eID_307e_305:1` |
| `\![unload,shiori]` | 未対応（書いてあるのに何も起きない） | 読み込み直しと差し替え | `ukadoc:list_sakura_script:_5c_21_5bunload_2cshiori_5d:1` |
| `\![unlock,balloonmove]` | 未対応（書いてあるのに何も起きない） | 描き直しと移動の凍結 | `ukadoc:list_sakura_script:_5c_21_5bunlock_2cballoonmove_5d:1` |
| `\![unlock,balloonrepaint]` | 未対応（書いてあるのに何も起きない） | 描き直しと移動の凍結 | `ukadoc:list_sakura_script:_5c_21_5bunlock_2cballoonrepaint_5d:1` |
| `\![unlock,repaint]` | 未対応（書いてあるのに何も起きない） | 描き直しと移動の凍結 | `ukadoc:list_sakura_script:_5c_21_5bunlock_2crepaint_5d:1` |
| `\![update,更新対象(,オプション,オプション...)]` | 未対応（書いてあるのに何も起きない） | インストールと更新 | `ukadoc:list_sakura_script:_5c_21_5bupdate_2c_66f4_65b0_5bfe_8c61_28_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._29_5d:1` |
| `\![update,platform]` | 未対応（書いてあるのに何も起きない） | インストールと更新 | `ukadoc:list_sakura_script:_5c_21_5bupdate_2cplatform_5d:1` |
| `\![updatebymyself(,オプション,オプション...)]` | 未対応（書いてあるのに何も起きない） | インストールと更新 | `ukadoc:list_sakura_script:_5c_21_5bupdatebymyself_28_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._29_5d:1` |
| `\![updateother,更新対象/オプション群,...]` | 未対応（書いてあるのに何も起きない） | インストールと更新 | `ukadoc:list_sakura_script:_5c_21_5bupdateother_2c_66f4_65b0_5bfe_8c61_2f_30aa_30d7_30b7_30e7_30f3_7fa4_2c..._5d:1` |
| `\![vanishbymyself]` | 未対応（書いてあるのに何も起きない） | ゴーストの終了 | `ukadoc:list_sakura_script:_5c_21_5bvanishbymyself_5d:1` |
| `\&[ID]` | 未対応（書いてあるのに何も起きない） | 文字コードの埋め込みと実体参照 | `ukadoc:list_sakura_script:_5c_26_5bID_5d:1` |
| `\*` | 未対応（書いてあるのに何も起きない） | バルーンの追記と選択肢のタイムアウト抑止 | `ukadoc:list_sakura_script:_5c_2a:1` |
| `\+` | 未対応（書いてあるのに何も起きない） | ゴーストの切り替え | `ukadoc:list_sakura_script:_5c_2b:1` |
| `\_!` | 未対応（書いてあるのに何も起きない） | タグを実行しない区間 | `ukadoc:list_sakura_script:_5c__21:1` |
| `\_+` | 未対応（書いてあるのに何も起きない） | ゴーストの切り替え | `ukadoc:list_sakura_script:_5c__2b:1` |
| `\_?` | 未対応（書いてあるのに何も起きない） | タグを実行しない区間 | `ukadoc:list_sakura_script:_5c__3f:1` |
| `\__c` | 未対応（書いてあるのに何も起きない） | 外部を開く・別窓 | `ukadoc:list_sakura_script:_5c__c:1` |
| `\__q[ID,...]` | 未対応（書いてあるのに何も起きない） | 選択肢の別書式 | `ukadoc:list_sakura_script:_5c__q_5bID_2c..._5d:1` |
| `\__t` | 未対応（書いてあるのに何も起きない） | 外部を開く・別窓 | `ukadoc:list_sakura_script:_5c__t:1` |
| `\__v[オプション]` | 未対応（書いてあるのに何も起きない） | 音声合成 | `ukadoc:list_sakura_script:_5c__v_5b_30aa_30d7_30b7_30e7_30f3_5d:1` |
| `\__w[時間]` | 未対応（書いてあるのに何も起きない） | 表示の速さと区間 | `ukadoc:list_sakura_script:_5c__w_5b_6642_9593_5d:1` |
| `\__w[animation,ID]` | 未対応（書いてあるのに何も起きない） | 表示の速さと区間 | `ukadoc:list_sakura_script:_5c__w_5banimation_2cID_5d:1` |
| `\_b[ファイルパス,inline,オプション,オプション...]` | 未対応（書いてあるのに何も起きない） | バルーンへの画像貼り付け | `ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cinline_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._5d:1` |
| `\_b[ファイルパス,inline,opaque]` | 未対応（書いてあるのに何も起きない） | バルーンへの画像貼り付け | `ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cinline_2copaque_5d:1` |
| `\_b[ファイルパス,x,y,オプション,オプション...]` | 未対応（書いてあるのに何も起きない） | バルーンへの画像貼り付け | `ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cx_2cy_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._5d:1` |
| `\_b[ファイルパス,x,y,opaque]` | 未対応（書いてあるのに何も起きない） | バルーンへの画像貼り付け | `ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cx_2cy_2copaque_5d:1` |
| `\_m[0x00]` | 未対応（書いてあるのに何も起きない） | 文字コードの埋め込みと実体参照 | `ukadoc:list_sakura_script:_5c_m_5b0x00_5d:1` |
| `\_n` | 未対応（書いてあるのに何も起きない） | 表示の速さと区間 | `ukadoc:list_sakura_script:_5c_n:1` |
| `\_s[ID1,ID2,ID3...]` | 未対応（書いてあるのに何も起きない） | 表示の速さと区間 | `ukadoc:list_sakura_script:_5c_s_5bID1_2cID2_2cID3..._5d:1` |
| `\_u[0x0000]` | 未対応（書いてあるのに何も起きない） | 文字コードの埋め込みと実体参照 | `ukadoc:list_sakura_script:_5c_u_5b0x0000_5d:1` |
| `\_w[時間]` | 実装済み | — | `ukadoc:list_sakura_script:_5c_w_5b_6642_9593_5d:1` |
| `\a` | 未対応（書いてあるのに何も起きない） | イベントを起こすだけのタグ | `ukadoc:list_sakura_script:_5ca:1` |
| `\c[char,数値,開始位置]` | 未対応（書いてあるのに何も起きない） | バルーン内の部分消去 | `ukadoc:list_sakura_script:_5cc_5bchar_2c_6570_5024_2c_958b_59cb_4f4d_7f6e_5d:1` |
| `\c[line,数値,開始位置]` | 未対応（書いてあるのに何も起きない） | バルーン内の部分消去 | `ukadoc:list_sakura_script:_5cc_5bline_2c_6570_5024_2c_958b_59cb_4f4d_7f6e_5d:1` |
| `\e` | 実装済み | — | `ukadoc:list_sakura_script:_5ce:1` |
| `\i[ID,wait]` | 未対応（書いてあるのに何も起きない） | サーフェスアニメーション | `ukadoc:list_sakura_script:_5ci_5bID_2cwait_5d:1` |
| `\i[ID番号]` | 未対応（書いてあるのに何も起きない） | サーフェスアニメーション | `ukadoc:list_sakura_script:_5ci_5bID_756a_53f7_5d:1` |
| `\j[ID]` | 未対応（書いてあるのに何も起きない） | 外部を開く・別窓 | `ukadoc:list_sakura_script:_5cj_5bID_5d:1` |
| `\m[umsg,wparam,lparam]` | 未対応（書いてあるのに何も起きない） | 外部を開く・別窓 | `ukadoc:list_sakura_script:_5cm_5bumsg_2cwparam_2clparam_5d:1` |
| `\n` | 実装済み | — | `ukadoc:list_sakura_script:_5cn:1` |
| `\n[パーセント]` | 実装済み | — | `ukadoc:list_sakura_script:_5cn_5b_30d1_30fc_30bb_30f3_30c8_5d:1` |
| `\n[half]` | 実装済み | — | `ukadoc:list_sakura_script:_5cn_5bhalf_5d:1` |
| `\p[ID番号]` | 実装済み | — | `ukadoc:list_sakura_script:_5cp_5bID_756a_53f7_5d:1` |
| `\q[タイトル,ID1,ID2,ID3...]` | 一部だけが効かない | — | `ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cID1_2cID2_2cID3..._5d:1` |
| `\q[タイトル,ID,r2,r3...]` | 実装済み | — | `ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cID_2cr2_2cr3..._5d:1` |
| `\q[タイトル,OnID,r0,r1,...]` | 実装済み | — | `ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cOnID_2cr0_2cr1_2c..._5d:1` |
| `\q[タイトル,script:実行内容]` | 一部だけが効かない | — | `ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cscript_3a_5b9f_884c_5185_5bb9_5d:1` |
| `\s[ID番号]` | 実装済み | — | `ukadoc:list_sakura_script:_5cs_5bID_756a_53f7_5d:1` |
| `\v` | 未対応（書いてあるのに何も起きない） | キャラの移動と重なり | `ukadoc:list_sakura_script:_5cv:1` |
| `\w時間` | 実装済み | — | `ukadoc:list_sakura_script:_5cw_6642_9593:1` |

### 引受先の候補

**候補を担当の欄に書いた項目は 1 件も無い。** 候補は担当の欄ではなく、備考とこの一覧に書く決まりだからである。

**名前を挙げられる根拠が資料にあったのは 8 件だけである。** 対応表 §8 が「意味づけの所有先は未定で、統合担当の無所有一覧で裁定する」と明言している 8 件。字句の切れ目と素通しだけは完了済みの spec が定めている。

| 項目の綴り | 引受先の候補 | 台帳の項目 id |
| --- | --- | --- |
| `\_!` | 統合担当（`areka-P0-ukadoc-coverage-roadmap`）の無所有一覧で裁定 | `ukadoc:list_sakura_script:_5c__21:1` |
| `\_+` | 統合担当（`areka-P0-ukadoc-coverage-roadmap`）の無所有一覧で裁定 | `ukadoc:list_sakura_script:_5c__2b:1` |
| `\_?` | 統合担当（`areka-P0-ukadoc-coverage-roadmap`）の無所有一覧で裁定 | `ukadoc:list_sakura_script:_5c__3f:1` |
| `\__c` | 統合担当（`areka-P0-ukadoc-coverage-roadmap`）の無所有一覧で裁定 | `ukadoc:list_sakura_script:_5c__c:1` |
| `\__q[ID,...]` | 統合担当（`areka-P0-ukadoc-coverage-roadmap`）の無所有一覧で裁定 | `ukadoc:list_sakura_script:_5c__q_5bID_2c..._5d:1` |
| `\__t` | 統合担当（`areka-P0-ukadoc-coverage-roadmap`）の無所有一覧で裁定 | `ukadoc:list_sakura_script:_5c__t:1` |
| `\__v[オプション]` | 統合担当（`areka-P0-ukadoc-coverage-roadmap`）の無所有一覧で裁定 | `ukadoc:list_sakura_script:_5c__v_5b_30aa_30d7_30b7_30e7_30f3_5d:1` |
| `\_n` | 統合担当（`areka-P0-ukadoc-coverage-roadmap`）の無所有一覧で裁定 | `ukadoc:list_sakura_script:_5c_n:1` |

**残る 234 件には引受先の候補を書いていない。** まだ起票されていない spec を引受先として提案してもよい決まりだが、名前を挙げられる根拠が資料に無いものを書けば憶測になる。**書かなかったことを、書けなかった理由と一緒にここへ残す。**

あわせて、**既に動いているのに「誰が作ったか」を資料が言っていない項目が 13 件**ある。
実装済みの項目には、誰が実装したかの記録として完了済みの spec 名を書いてよい決まりだが、それは「書いてよい」であって「書かねばならない」ではないので、資料が言っていない項目には書かなかった。

| 項目の綴り | 台帳の項目 id |
| --- | --- |
| `%selfname` | `ukadoc:list_sakura_script:_25selfname:1` |
| `\-` | `ukadoc:list_sakura_script:_5c-:1` |
| `\![bind,カテゴリ名,パーツ名,数値]` | `ukadoc:list_sakura_script:_5c_21_5bbind_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_6570_5024_5d:1` |
| `\_w[時間]` | `ukadoc:list_sakura_script:_5c_w_5b_6642_9593_5d:1` |
| `\e` | `ukadoc:list_sakura_script:_5ce:1` |
| `\n` | `ukadoc:list_sakura_script:_5cn:1` |
| `\n[パーセント]` | `ukadoc:list_sakura_script:_5cn_5b_30d1_30fc_30bb_30f3_30c8_5d:1` |
| `\n[half]` | `ukadoc:list_sakura_script:_5cn_5bhalf_5d:1` |
| `\p[ID番号]` | `ukadoc:list_sakura_script:_5cp_5bID_756a_53f7_5d:1` |
| `\q[タイトル,ID,r2,r3...]` | `ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cID_2cr2_2cr3..._5d:1` |
| `\q[タイトル,OnID,r0,r1,...]` | `ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cOnID_2cr0_2cr1_2c..._5d:1` |
| `\s[ID番号]` | `ukadoc:list_sakura_script:_5cs_5bID_756a_53f7_5d:1` |
| `\w時間` | `ukadoc:list_sakura_script:_5cw_6642_9593:1` |

### タグ単位で担当を割る形に収まらない brief が 1 本ある

`areka-P0-sakura-tag-word-boundary` は、担当の欄に **1 度も現れない**。
取りこぼしではない。この brief が持っているのは「タグの語をどこで閉じるか」という**規則**であって、個々のタグの意味ではないからである。
規則は 342 件のうち特定の何件かに属するものではなく、タグを読み取る側の語の走査そのものに属する。

この brief は ⑶ 節の突合表には 11 本のうちの 1 本として出てくる（正典の項目 11 件に主張が届いている）が、その主張はすべて「字句の切れ目の対象として並べている」か「壊れ方の実例」であって、所有の宣言ではない。

**この一覧をタグ単位で読むだけでは、この brief の担当範囲は見えない。**
担当を「どのタグを持つか」で割り切れない spec が少なくとも 1 本あることを、順序を決める人に伝えておく必要がある。

---

## ⑸ 「書いてあるのに何も起きない」順の未対応一覧

ここに並ぶのは、**正典に書いてあるとおりに書いても areka では何も起きない項目**である。
台帳で未対応としたのが **259 件**、それに「別名だが areka がその綴りを受けない」**3 件**を足して **262 件**を扱う。

**結論を先に書く。この 262 件はいずれも未対応であり、いつ・どう作るかはここでは決めない。**
この節に設計も工程も見積りも書かない。書いてあるのは「利用者に何が起きるか」「その群を成立させる最小の基盤」「台帳の項目 id」の 3 つだけである。

**「最小の基盤」の読み方**——本調査が実際に辿った道（台本を語に切る → タグを意味へ読み替える → 台本を組み立てる → 名前を運ぶ → 名前で受け取って動かす）のうち、**欠けている環**を書いたものである。その先で必要になる仕組みが、areka の別の場所に既にあるかどうかまでは調べていない。

### 何も起きない道は 3 つある

以下で使う言い方を先に 3 つだけ決めておく。**運び役**＝タグの名前と引数を、実際に動かす側まで運ぶ 1 本の道。**受け口**＝運ばれてきた名前のうち自分の担当を選び取って、実際に動かす側。**素通し**＝どこにも読み替えられないまま、元の綴りのまま通り抜けること。

| 道 | 件数 | 何が起きているか |
| --- | ---: | --- |
| `\![...]` の形 | 182 | 名前は運び役に載って最後まで運ばれるが、その名前で自分を選ぶ受け口が 1 つも居ないので何も起こらない。**素通しになって捨てられるのではない**——組み立ての側の捨て場（どの分岐にも当たらなかったものを捨てる場所）には 1 件も来ない |
| `\f[...]` の形 | 40 | タグを読み替える所に `\f` の分岐が無いので、綴りにかかわらず素通しになり、組み立ての側の捨て場が記録を残して捨てる |
| それ以外のタグ | 37 | タグを読み替える所のどの分岐にも当たらず素通しになり、組み立ての側の捨て場が記録を残して捨てる |
| **合計** | **259** | |

どの道でも、**画面には何も出ず、誤りの表示も欠けた記号も見えない**。
記録には残る（担当外として記録を残す）ので、作者が記録を見れば気付ける。

### テーマが 0 件の 2 つについて

伺からしさのテーマは 8 つあるが、そのうち**「触れ合い」と「記憶」は本ドメインに 1 件も付いていない**。
**取りこぼしではない。** 撫でる・つつくに応える定義は絵の側（当たり判定）にあり、前に会ったことを覚えているのは SHIORI の側（イベントとプロパティ）にある。さくらスクリプトはその結果を画面へ出す言語なので、この 2 つに「無いと利用者が何を失うか」で答えられる項目が本ドメインには無い。
テーマの定義文書が挙げる代表項目も、2 つとも SHIORI のイベントか絵の側の定義であって、さくらスクリプトの項目は 1 つも入っていない。
**4 ドメインをまとめて読む人が、この 0 を取りこぼしと読まないよう明記しておく。**

### 群の一覧（54 群）

群の名前は台帳の備考が持っているものをそのまま使う。
同じ名前の群に複数の書き方が混ざることがある（たとえば「音の再生」には `\8[…]` の形と `\![sound,…]` の形の両方が入る）。

| 群 | 件数 | 書き方の内訳 | 優先度の段 | 付いているテーマ |
| --- | ---: | --- | --- | --- |
| SSP の管理窓とビューア | 20 | `\![...]` 20 | C20・C90 | 装い |
| アンカーの見た目 | 16 | `\f[...]` 16 | C90 | — |
| 入力窓とダイアログ | 12 | `\![...]` 12 | C20・C90 | 交わり |
| 動作モードの出入り | 12 | `\![...]` 12 | C90 | — |
| HTTP と RSS の通信 | 11 | `\![...]` 11 | C90 | — |
| 窓の位置と見え方 | 11 | `\![...]` 11 | C20・C90 | 気配り |
| 選択肢マーカーの見た目 | 10 | `\f[...]` 10 | C90 | — |
| サーフェスアニメーション | 9 | `\![...]` 7／それ以外 2 | C20 | 気配 |
| 読み込み直しと差し替え | 9 | `\![...]` 9 | C90 | — |
| 音の再生 | 9 | `\![...]` 8／それ以外 1 | C90 | — |
| バルーンの見た目と切り替え | 8 | `\![...]` 8 | C20・C90 | 装い |
| イベントを他所へ起こす | 6 | `\![...]` 6 | C20・C90 | 交わり |
| インストールと更新 | 6 | `\![...]` 6 | C20 | 更新 |
| 描き直しと移動の凍結 | 6 | `\![...]` 6 | C90 | — |
| 時間差で起こすイベント | 6 | `\![...]` 6 | C20・C90 | 交わり |
| 表示の速さと区間 | 6 | それ以外 6 | C20・C90 | 掛け合い |
| 開発と診断の窓 | 6 | `\![...]` 6 | C90 | — |
| OS まわりの操作 | 5 | `\![...]` 5 | C90 | — |
| WebSocket と通信の中止 | 5 | `\![...]` 5 | C90 | — |
| ゴーストの切り替え | 5 | `\![...]` 3／それ以外 2 | C20・C90 | 交わり・装い |
| 外部のアプリに渡す | 5 | `\![...]` 5 | C90 | — |
| 文字の色と縁取りと影 | 5 | `\f[...]` 5 | C90 | — |
| キャラの移動と重なり | 4 | `\![...]` 1／それ以外 3 | C20・C90 | 掛け合い |
| バルーンへの画像貼り付け | 4 | それ以外 4 | C90 | — |
| 外部を開く・別窓 | 4 | それ以外 4 | C20・C90 | 交わり |
| 太字・斜体・上下付き | 4 | `\f[...]` 4 | C90 | — |
| 書庫と配布物の作成 | 4 | `\![...]` 4 | C90 | — |
| 画面効果のプラグイン | 4 | `\![...]` 4 | C90 | — |
| 通信箱と教え込み箱 | 4 | `\![...]` 4 | C20 | 交わり |
| 他のゴーストとの付き合い方 | 3 | `\![...]` 3 | C20・C90 | 交わり |
| 文字コードの埋め込みと実体参照 | 3 | それ以外 3 | C90 | — |
| アンカー | 2 | それ以外 2 | C20 | 交わり |
| サーフェスの拡大縮小 | 2 | `\![...]` 2 | C90 | — |
| タグを実行しない区間 | 2 | それ以外 2 | C90 | — |
| ネットワークの調べもの | 2 | `\![...]` 2 | C90 | — |
| バルーンの追記と選択肢のタイムアウト抑止 | 2 | それ以外 2 | C20・C90 | 交わり |
| バルーン内の部分消去 | 2 | それ以外 2 | C90 | — |
| プロパティの照会と設定 | 2 | `\![...]` 2 | C90 | — |
| ヘッドラインセンサ | 2 | `\![...]` 2 | C90 | — |
| 同期オブジェクト | 2 | `\![...]` 2 | C90 | — |
| 文字の書体と大きさ | 2 | `\f[...]` 2 | C90 | — |
| 時計合わせ | 2 | `\![...]` 1／それ以外 1 | C90 | — |
| 窓の位置の初期化 | 2 | `\![...]` 2 | C90 | — |
| 装飾の一括の戻し | 2 | `\f[...]` 2 | C90 | — |
| 通知領域とデスクトップ | 2 | `\![...]` 2 | C90 | — |
| イベントを起こすだけのタグ | 1 | それ以外 1 | C20 | 交わり |
| ゴーストの終了 | 1 | `\![...]` 1 | C90 | — |
| ネットワーク通信 | 1 | `\![...]` 1 | C90 | — |
| メールチェック | 1 | `\![...]` 1 | C90 | — |
| 下線と打ち消し線 | 1 | `\f[...]` 1 | C90 | — |
| 着せ替え | 1 | `\![...]` 1 | C20 | 装い |
| 選択肢の別書式 | 1 | それ以外 1 | C20 | 交わり |
| 選択肢マーカーの表示 | 1 | `\![...]` 1 | C90 | — |
| 音声合成 | 1 | それ以外 1 | C90 | — |
| **合計** | **259** | | | |

### 群ごとの中身

#### SSP の管理窓とビューア（20 件）

- **利用者に何が起きるか**: ゴーストの一覧・シェルの一覧・バルーンの一覧・着せ替えの一覧・説明書・過去の会話・利用条件など、利用者に見せる窓が 1 つも開かない。「そこから選んでね」と促す台詞が空振りになる。
- **その群を成立させる最小の基盤**: 一覧や説明を見せる窓と、それを台本から開く道。
- **台帳の項目 id**:
  - `\![open,addressbar]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2caddressbar_5d:1`
  - `\![open,aigraph]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2caigraph_5d:1`
  - `\![open,archiveviewer,(ファイル名)]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2carchiveviewer_2c_28_30d5_30a1_30a4_30eb_540d_29_5d:1`
  - `\![open,backlogviewer]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cbacklogviewer_5d:1`
  - `\![open,balloonexplorer]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cballoonexplorer_5d:1`
  - `\![open,calendar]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2ccalendar_5d:1`
  - `\![open,configurationdialog,ダイアログID]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cconfigurationdialog_2c_30c0_30a4_30a2_30ed_30b0ID_5d:1`
  - `\![open,dressupexplorer]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cdressupexplorer_5d:1`
  - `\![open,ghostexplorer]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cghostexplorer_5d:1`
  - `\![open,help,ダイアログID]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2chelp_2c_30c0_30a4_30a2_30ed_30b0ID_5d:1`
  - `\![open,messenger]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cmessenger_5d:1`
  - `\![open,pictureviewer,(ファイル名)]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cpictureviewer_2c_28_30d5_30a1_30a4_30eb_540d_29_5d:1`
  - `\![open,pluginexplorer]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cpluginexplorer_5d:1`
  - `\![open,rateofusegraph]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2crateofusegraph_5d:1`
  - `\![open,rateofusegraphballoon]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2crateofusegraphballoon_5d:1`
  - `\![open,rateofusegraphtotal]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2crateofusegraphtotal_5d:1`
  - `\![open,readme]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2creadme_5d:1`
  - `\![open,shellexplorer]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cshellexplorer_5d:1`
  - `\![open,surfacetest]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2csurfacetest_5d:1`
  - `\![open,terms]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cterms_5d:1`

#### アンカーの見た目（16 件）

- **利用者に何が起きるか**: アンカーの色・枠線・形・塗り方が 1 つも指定どおりにならない。アンカーはバルーン設定の見た目のまま出る。選択中・非選択・訪問済みの区別も付かない。
- **その群を成立させる最小の基盤**: タグを読み替える所に `\f` の分岐を置き、装飾の状態を持って文字の描き方へ渡す道。いまは `\f[…]` の綴りが 1 つも読み取られない。
- **台帳の項目 id**:
  - `\f[anchor.font.color,色指定]` — `ukadoc:list_sakura_script:_5cf_5banchor.font.color_2c_8272_6307_5b9a_5d:1`
  - `\f[anchorcolor,色指定]もしくは\f[anchorbrushcolor,色指定]` — `ukadoc:list_sakura_script:_5cf_5banchorcolor_2c_8272_6307_5b9a_5d_3082_3057_304f_306f_5cf_5banchorbrushcolor_2c_8272_6307_5b9a_5d:1`
  - `\f[anchorfontcolor,色指定]` — `ukadoc:list_sakura_script:_5cf_5banchorfontcolor_2c_8272_6307_5b9a_5d:1`
  - `\f[anchormethod,描画方法]` — `ukadoc:list_sakura_script:_5cf_5banchormethod_2c_63cf_753b_65b9_6cd5_5d:1`
  - `\f[anchornotselectcolor,色指定]もしくは\f[anchornotselectbrushcolor,色指定]` — `ukadoc:list_sakura_script:_5cf_5banchornotselectcolor_2c_8272_6307_5b9a_5d_3082_3057_304f_306f_5cf_5banchornotselectbrushcolor_2c_8272_6307_5b9a_5:1`
  - `\f[anchornotselectfontcolor,色指定]` — `ukadoc:list_sakura_script:_5cf_5banchornotselectfontcolor_2c_8272_6307_5b9a_5d:1`
  - `\f[anchornotselectmethod,描画方法]` — `ukadoc:list_sakura_script:_5cf_5banchornotselectmethod_2c_63cf_753b_65b9_6cd5_5d:1`
  - `\f[anchornotselectpencolor,色指定]` — `ukadoc:list_sakura_script:_5cf_5banchornotselectpencolor_2c_8272_6307_5b9a_5d:1`
  - `\f[anchornotselectstyle,形状]` — `ukadoc:list_sakura_script:_5cf_5banchornotselectstyle_2c_5f62_72b6_5d:1`
  - `\f[anchorpencolor,色指定]` — `ukadoc:list_sakura_script:_5cf_5banchorpencolor_2c_8272_6307_5b9a_5d:1`
  - `\f[anchorstyle,形状]` — `ukadoc:list_sakura_script:_5cf_5banchorstyle_2c_5f62_72b6_5d:1`
  - `\f[anchorvisitedcolor,色指定]もしくは\f[anchorvisitedbrushcolor,色指定]` — `ukadoc:list_sakura_script:_5cf_5banchorvisitedcolor_2c_8272_6307_5b9a_5d_3082_3057_304f_306f_5cf_5banchorvisitedbrushcolor_2c_8272_6307_5b9a_5d:1`
  - `\f[anchorvisitedfontcolor,色指定]` — `ukadoc:list_sakura_script:_5cf_5banchorvisitedfontcolor_2c_8272_6307_5b9a_5d:1`
  - `\f[anchorvisitedmethod,描画方法]` — `ukadoc:list_sakura_script:_5cf_5banchorvisitedmethod_2c_63cf_753b_65b9_6cd5_5d:1`
  - `\f[anchorvisitedpencolor,色指定]` — `ukadoc:list_sakura_script:_5cf_5banchorvisitedpencolor_2c_8272_6307_5b9a_5d:1`
  - `\f[anchorvisitedstyle,形状]` — `ukadoc:list_sakura_script:_5cf_5banchorvisitedstyle_2c_5f62_72b6_5d:1`

#### 入力窓とダイアログ（12 件）

- **利用者に何が起きるか**: 利用者に何かを入力させる場面が成立しない。文字・数・時刻・日付・色・ファイル・フォルダのどれを尋ねる窓も開かず、選んだ結果を返すイベントも起きない。
- **その群を成立させる最小の基盤**: 入力を受け取る窓と、入力の結果をイベントとして返す道。
- **台帳の項目 id**:
  - `\![close,dialog,ID]` — `ukadoc:list_sakura_script:_5c_21_5bclose_2cdialog_2cID_5d:1`
  - `\![close,inputbox,ID]` — `ukadoc:list_sakura_script:_5c_21_5bclose_2cinputbox_2cID_5d:1`
  - `\![open,dateinput,ID,表示時間,年,月,日,オプション]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cdateinput_2cID_2c_8868_793a_6642_9593_2c_5e74_2c_6708_2c_65e5_2c_30aa_30d7_30b7_30e7_30f3_5d:1`
  - `\![open,dialog,color,パラメータ]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cdialog_2ccolor_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
  - `\![open,dialog,folder,パラメータ]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cdialog_2cfolder_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
  - `\![open,dialog,open,パラメータ]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cdialog_2copen_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
  - `\![open,dialog,save,パラメータ]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cdialog_2csave_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
  - `\![open,inputbox,ID,表示時間,テキスト,オプション,...]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cinputbox_2cID_2c_8868_793a_6642_9593_2c_30c6_30ad_30b9_30c8_2c_30aa_30d7_30b7_30e7_30f3_2c..._5d:1`
  - `\![open,ipinput,ID,表示時間,IP1桁目,IP2桁目,IP3桁目,IP4桁目,オプション]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cipinput_2cID_2c_8868_793a_6642_9593_2cIP1_6841_76ee_2cIP2_6841_76ee_2cIP3_6841_76ee_2cIP4_6841_76ee_2c_3:1`
  - `\![open,passwordinput,ID,表示時間,テキスト,オプション]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cpasswordinput_2cID_2c_8868_793a_6642_9593_2c_30c6_30ad_30b9_30c8_2c_30aa_30d7_30b7_30e7_30f3_5d:1`
  - `\![open,sliderinput,ID,表示時間,現在値,最小,最大,オプション]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2csliderinput_2cID_2c_8868_793a_6642_9593_2c_73fe_5728_5024_2c_6700_5c0f_2c_6700_5927_2c_30aa_30d7_30b7_30:1`
  - `\![open,timeinput,ID,表示時間,時,分,秒,オプション]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2ctimeinput_2cID_2c_8868_793a_6642_9593_2c_6642_2c_5206_2c_79d2_2c_30aa_30d7_30b7_30e7_30f3_5d:1`

#### 動作モードの出入り（12 件）

- **利用者に何が起きるか**: 受け身・案内・割り込み禁止・通信中・当たり判定の確認・範囲選択という 6 つの状態のどれにも入れず、抜ける側も効かない。とくに割り込み禁止が効かないので、最後まで聞かせたい話を利用者が途中で止められる。
- **その群を成立させる最小の基盤**: 動作の状態を持っておいて、入る・抜けるで切り替える仕組み。入る側と抜ける側が対になっているので、片方だけでは成立しない。
- **台帳の項目 id**:
  - `\![enter,collisionmode]\![enter,collisionmode,rect]` — `ukadoc:list_sakura_script:_5c_21_5benter_2ccollisionmode_5d_5c_21_5benter_2ccollisionmode_2crect_5d:1`
  - `\![enter,inductionmode]` — `ukadoc:list_sakura_script:_5c_21_5benter_2cinductionmode_5d:1`
  - `\![enter,nouserbreakmode]` — `ukadoc:list_sakura_script:_5c_21_5benter_2cnouserbreakmode_5d:1`
  - `\![enter,onlinemode]` — `ukadoc:list_sakura_script:_5c_21_5benter_2conlinemode_5d:1`
  - `\![enter,passivemode]` — `ukadoc:list_sakura_script:_5c_21_5benter_2cpassivemode_5d:1`
  - `\![enter,selectmode,モード(rect),左,上,右,下]\![enter,selectmode,モード(rect),当たり判定名]\![enter,selectmode,モード(rect)]` — `ukadoc:list_sakura_script:_5c_21_5benter_2cselectmode_2c_30e2_30fc_30c9_28rect_29_2c_5de6_2c_4e0a_2c_53f3_2c_4e0b_5d_5c_21_5benter_2cselectmode_2c:1`
  - `\![leave,collisionmode]` — `ukadoc:list_sakura_script:_5c_21_5bleave_2ccollisionmode_5d:1`
  - `\![leave,inductionmode]` — `ukadoc:list_sakura_script:_5c_21_5bleave_2cinductionmode_5d:1`
  - `\![leave,nouserbreakmode]` — `ukadoc:list_sakura_script:_5c_21_5bleave_2cnouserbreakmode_5d:1`
  - `\![leave,onlinemode]` — `ukadoc:list_sakura_script:_5c_21_5bleave_2conlinemode_5d:1`
  - `\![leave,passivemode]` — `ukadoc:list_sakura_script:_5c_21_5bleave_2cpassivemode_5d:1`
  - `\![leave,selectmode]` — `ukadoc:list_sakura_script:_5c_21_5bleave_2cselectmode_5d:1`

#### HTTP と RSS の通信（11 件）

- **利用者に何が起きるか**: 外の網とのやりとりが 1 つも起きない。取ってくるはずの本文も、送ったはずの内容も届かず、結果を知らせるイベントも返らない。
- **その群を成立させる最小の基盤**: HTTP の要求を出して応答を受け取る部分と、その結果を SHIORI のイベントとして返す道。
- **台帳の項目 id**:
  - `\![execute,http-delete,URL,オプション,オプション,オプション...]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-delete_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3:1`
  - `\![execute,http-get,URL,オプション,オプション,オプション...]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-get_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3...:1`
  - `\![execute,http-get,URL,パラメータ]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-get_2cURL_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
  - `\![execute,http-head,URL,オプション,オプション,オプション...]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-head_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..:1`
  - `\![execute,http-options,URL,オプション,オプション,オプション...]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-options_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f:1`
  - `\![execute,http-patch,URL,オプション,オプション,オプション...]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-patch_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3.:1`
  - `\![execute,http-post,URL,オプション,オプション,オプション...]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-post_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..:1`
  - `\![execute,http-post,URL,パラメータ]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-post_2cURL_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
  - `\![execute,http-put,URL,オプション,オプション,オプション...]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-put_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3...:1`
  - `\![execute,rss-get,URL,オプション,オプション,オプション...]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2crss-get_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._:1`
  - `\![execute,rss-post,URL,オプション,オプション,オプション...]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2crss-post_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3...:1`

#### 窓の位置と見え方（11 件）

- **利用者に何が起きるか**: 窓を置く場所・画面の縁への吸い付き・透け具合・最小化・手前への固定・窓どうしのくっつきが、どれも指定どおりにならない。
- **その群を成立させる最小の基盤**: 窓の位置と見え方を台本から動かす道。窓の重なり順だけは既に受け口があり、この群のほかの指示には受け口が無い。
- **台帳の項目 id**:
  - `\![reset,position]` — `ukadoc:list_sakura_script:_5c_21_5breset_2cposition_5d:1`
  - `\![reset,sticky-window]` — `ukadoc:list_sakura_script:_5c_21_5breset_2csticky-window_5d:1`
  - `\![set,alignmentondesktop,bottomまたはtop]` — `ukadoc:list_sakura_script:_5c_21_5bset_2calignmentondesktop_2cbottom_307e_305f_306ftop_5d:1`
  - `\![set,alignmenttodesktop,方向]` — `ukadoc:list_sakura_script:_5c_21_5bset_2calignmenttodesktop_2c_65b9_5411_5d:1`
  - `\![set,alignmenttodesktop,free]` — `ukadoc:list_sakura_script:_5c_21_5bset_2calignmenttodesktop_2cfree_5d:1`
  - `\![set,alpha,数値,オプション]` — `ukadoc:list_sakura_script:_5c_21_5bset_2calpha_2c_6570_5024_2c_30aa_30d7_30b7_30e7_30f3_5d:1`
  - `\![set,position,x,y,スコープID]` — `ukadoc:list_sakura_script:_5c_21_5bset_2cposition_2cx_2cy_2c_30b9_30b3_30fc_30d7ID_5d:1`
  - `\![set,sticky-window,スコープID,スコープID,...]` — `ukadoc:list_sakura_script:_5c_21_5bset_2csticky-window_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c..._5d:1`
  - `\![set,windowstate,!stayontop]` — `ukadoc:list_sakura_script:_5c_21_5bset_2cwindowstate_2c_21stayontop_5d:1`
  - `\![set,windowstate,minimize]` — `ukadoc:list_sakura_script:_5c_21_5bset_2cwindowstate_2cminimize_5d:1`
  - `\![set,windowstate,stayontop]` — `ukadoc:list_sakura_script:_5c_21_5bset_2cwindowstate_2cstayontop_5d:1`

#### 選択肢マーカーの見た目（10 件）

- **利用者に何が起きるか**: 選択肢マーカーの色・枠線・形・塗り方が 1 つも指定どおりにならない。選択中と非選択の区別も付かない。
- **その群を成立させる最小の基盤**: タグを読み替える所に `\f` の分岐を置き、装飾の状態を持って文字の描き方へ渡す道。
- **台帳の項目 id**:
  - `\f[cursorcolor,色指定]もしくは\f[cursorbrushcolor,色指定]` — `ukadoc:list_sakura_script:_5cf_5bcursorcolor_2c_8272_6307_5b9a_5d_3082_3057_304f_306f_5cf_5bcursorbrushcolor_2c_8272_6307_5b9a_5d:1`
  - `\f[cursorfontcolor,色指定]` — `ukadoc:list_sakura_script:_5cf_5bcursorfontcolor_2c_8272_6307_5b9a_5d:1`
  - `\f[cursormethod,描画方法]` — `ukadoc:list_sakura_script:_5cf_5bcursormethod_2c_63cf_753b_65b9_6cd5_5d:1`
  - `\f[cursornotselectcolor,色指定]もしくは\f[cursornotselectbrushcolor,色指定]` — `ukadoc:list_sakura_script:_5cf_5bcursornotselectcolor_2c_8272_6307_5b9a_5d_3082_3057_304f_306f_5cf_5bcursornotselectbrushcolor_2c_8272_6307_5b9a_5:1`
  - `\f[cursornotselectfontcolor,色指定]` — `ukadoc:list_sakura_script:_5cf_5bcursornotselectfontcolor_2c_8272_6307_5b9a_5d:1`
  - `\f[cursornotselectmethod,描画方法]` — `ukadoc:list_sakura_script:_5cf_5bcursornotselectmethod_2c_63cf_753b_65b9_6cd5_5d:1`
  - `\f[cursornotselectpencolor,色指定]` — `ukadoc:list_sakura_script:_5cf_5bcursornotselectpencolor_2c_8272_6307_5b9a_5d:1`
  - `\f[cursornotselectstyle,形状]` — `ukadoc:list_sakura_script:_5cf_5bcursornotselectstyle_2c_5f62_72b6_5d:1`
  - `\f[cursorpencolor,色指定]` — `ukadoc:list_sakura_script:_5cf_5bcursorpencolor_2c_8272_6307_5b9a_5d:1`
  - `\f[cursorstyle,形状]` — `ukadoc:list_sakura_script:_5cf_5bcursorstyle_2c_5f62_72b6_5d:1`

#### サーフェスアニメーション（9 件）

- **利用者に何が起きるか**: 立ち絵が動かない。瞬きも身じろぎも、台本から足す重ね合わせや文字の重ねも起きず、止め絵のまま台詞だけが流れる。動きを止める・ずらす側も同じく効かない。
- **その群を成立させる最小の基盤**: 立ち絵のアニメーションを台本から足す・消す・止める道。`\i[…]` はタグを読み替える所のどの分岐にも当たらず素通しになり、`\![anim,…]` の 7 件は名前が運ばれるだけである。
- **台帳の項目 id**:
  - `\![anim,add,overlay,ID]` — `ukadoc:list_sakura_script:_5c_21_5banim_2cadd_2coverlay_2cID_5d:1`
  - `\![anim,add,text,x,y,横幅,縦幅,文字列,表示時間,r,g,b,文字サイズ,文字名]` — `ukadoc:list_sakura_script:_5c_21_5banim_2cadd_2ctext_2cx_2cy_2c_6a2a_5e45_2c_7e26_5e45_2c_6587_5b57_5217_2c_8868_793a_6642_9593_2cr_2cg_2cb_2c_658:1`
  - `\![anim,clear,ID]` — `ukadoc:list_sakura_script:_5c_21_5banim_2cclear_2cID_5d:1`
  - `\![anim,offset,ID,x座標,y座標]` — `ukadoc:list_sakura_script:_5c_21_5banim_2coffset_2cID_2cx_5ea7_6a19_2cy_5ea7_6a19_5d:1`
  - `\![anim,pause,ID]` — `ukadoc:list_sakura_script:_5c_21_5banim_2cpause_2cID_5d:1`
  - `\![anim,resume,ID]` — `ukadoc:list_sakura_script:_5c_21_5banim_2cresume_2cID_5d:1`
  - `\![anim,stop,ID]` — `ukadoc:list_sakura_script:_5c_21_5banim_2cstop_2cID_5d:1`
  - `\i[ID,wait]` — `ukadoc:list_sakura_script:_5ci_5bID_2cwait_5d:1`
  - `\i[ID番号]` — `ukadoc:list_sakura_script:_5ci_5bID_756a_53f7_5d:1`

#### 読み込み直しと差し替え（9 件）

- **利用者に何が起きるか**: 書き換えた辞書・設定・画像が会話の途中で反映されない。ゴースト一式の読み直しも、言い換えの仕組みの出し入れも起きない。
- **その群を成立させる最小の基盤**: 読み込み済みのものを差し替える仕組みと、その指示を名前で受け取る口。
- **台帳の項目 id**:
  - `\![load,makoto]` — `ukadoc:list_sakura_script:_5c_21_5bload_2cmakoto_5d:1`
  - `\![load,shiori]` — `ukadoc:list_sakura_script:_5c_21_5bload_2cshiori_5d:1`
  - `\![reload,descript,パラメータ]` — `ukadoc:list_sakura_script:_5c_21_5breload_2cdescript_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
  - `\![reload,ghost]` — `ukadoc:list_sakura_script:_5c_21_5breload_2cghost_5d:1`
  - `\![reload,makoto]` — `ukadoc:list_sakura_script:_5c_21_5breload_2cmakoto_5d:1`
  - `\![reload,shell]` — `ukadoc:list_sakura_script:_5c_21_5breload_2cshell_5d:1`
  - `\![reload,shiori]` — `ukadoc:list_sakura_script:_5c_21_5breload_2cshiori_5d:1`
  - `\![unload,makoto]` — `ukadoc:list_sakura_script:_5c_21_5bunload_2cmakoto_5d:1`
  - `\![unload,shiori]` — `ukadoc:list_sakura_script:_5c_21_5bunload_2cshiori_5d:1`

#### 音の再生（9 件）

- **利用者に何が起きるか**: 音が 1 つも鳴らない。効果音も音楽も、鳴らす・止める・一時停止・再開のすべてが無効なので、ゴーストは終始無音で喋る。
- **その群を成立させる最小の基盤**: 音を鳴らす部分と、その再生を名前で受け取る口。`\8[…]` はタグを読み替える所のどの分岐にも当たらず素通しになり、`\![sound,…]` の 8 件は名前が運ばれるだけである。
- **台帳の項目 id**:
  - `\8[ファイル名]` — `ukadoc:list_sakura_script:_5c8_5b_30d5_30a1_30a4_30eb_540d_5d:1`
  - `\![sound,cdplay,トラックNo.]` — `ukadoc:list_sakura_script:_5c_21_5bsound_2ccdplay_2c_30c8_30e9_30c3_30afNo._5d:1`
  - `\![sound,load,ファイル名,オプション...]` — `ukadoc:list_sakura_script:_5c_21_5bsound_2cload_2c_30d5_30a1_30a4_30eb_540d_2c_30aa_30d7_30b7_30e7_30f3..._5d:1`
  - `\![sound,loop,ファイル名]` — `ukadoc:list_sakura_script:_5c_21_5bsound_2cloop_2c_30d5_30a1_30a4_30eb_540d_5d:1`
  - `\![sound,option,ファイル名,オプション...]` — `ukadoc:list_sakura_script:_5c_21_5bsound_2coption_2c_30d5_30a1_30a4_30eb_540d_2c_30aa_30d7_30b7_30e7_30f3..._5d:1`
  - `\![sound,pause,ファイル名]` — `ukadoc:list_sakura_script:_5c_21_5bsound_2cpause_2c_30d5_30a1_30a4_30eb_540d_5d:1`
  - `\![sound,play,ファイル名,オプション...]` — `ukadoc:list_sakura_script:_5c_21_5bsound_2cplay_2c_30d5_30a1_30a4_30eb_540d_2c_30aa_30d7_30b7_30e7_30f3..._5d:1`
  - `\![sound,resume,ファイル名]` — `ukadoc:list_sakura_script:_5c_21_5bsound_2cresume_2c_30d5_30a1_30a4_30eb_540d_5d:1`
  - `\![sound,stop,ファイル名]` — `ukadoc:list_sakura_script:_5c_21_5bsound_2cstop_2c_30d5_30a1_30a4_30eb_540d_5d:1`

#### バルーンの見た目と切り替え（8 件）

- **利用者に何が起きるか**: バルーンの意匠を替えられず、付ける側・並べ方・印・枚数の表示・位置のずらしも効かない。自動送りの入切も届かないので、長い台詞は自分では流れない。
- **その群を成立させる最小の基盤**: バルーンの意匠と表示の設定を、会話の途中で差し替える道。
- **台帳の項目 id**:
  - `\![change,balloon,バルーン名]` — `ukadoc:list_sakura_script:_5c_21_5bchange_2cballoon_2c_30d0_30eb_30fc_30f3_540d_5d:1`
  - `\![reload,balloon]` — `ukadoc:list_sakura_script:_5c_21_5breload_2cballoon_5d:1`
  - `\![set,autoscroll,disable]` — `ukadoc:list_sakura_script:_5c_21_5bset_2cautoscroll_2cdisable_5d:1`
  - `\![set,autoscroll,enable]` — `ukadoc:list_sakura_script:_5c_21_5bset_2cautoscroll_2cenable_5d:1`
  - `\![set,balloonalign,ID]` — `ukadoc:list_sakura_script:_5c_21_5bset_2cballoonalign_2cID_5d:1`
  - `\![set,balloonmarker,マーカー表示文字列]` — `ukadoc:list_sakura_script:_5c_21_5bset_2cballoonmarker_2c_30de_30fc_30ab_30fc_8868_793a_6587_5b57_5217_5d:1`
  - `\![set,balloonnum,ファイル名,現在の数,最大数]` — `ukadoc:list_sakura_script:_5c_21_5bset_2cballoonnum_2c_30d5_30a1_30a4_30eb_540d_2c_73fe_5728_306e_6570_2c_6700_5927_6570_5d:1`
  - `\![set,balloonoffset,x,y]` — `ukadoc:list_sakura_script:_5c_21_5bset_2cballoonoffset_2cx_2cy_5d:1`

#### イベントを他所へ起こす（6 件）

- **利用者に何が起きるか**: 自分の SHIORI へも、別のゴーストへも、プラグインへもイベントが届かない。掛け合いの筋書きも、台本から台本へ渡す作りも成立しない。
- **その群を成立させる最小の基盤**: イベント名と引数を組み立てて相手へ渡す道。相手のイベント名は引数で決まるので、受け口は名前を 1 つに決め打ちできない。
- **台帳の項目 id**:
  - `\![notify,イベント名,r0,r1,r2...]` — `ukadoc:list_sakura_script:_5c_21_5bnotify_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1`
  - `\![notifyother,ゴースト名,イベント名,r0,r1,r2...]` — `ukadoc:list_sakura_script:_5c_21_5bnotifyother_2c_30b4_30fc_30b9_30c8_540d_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1`
  - `\![notifyplugin,プラグインのIDまたは名前,イベント名,r0,r1,r2...]` — `ukadoc:list_sakura_script:_5c_21_5bnotifyplugin_2c_30d7_30e9_30b0_30a4_30f3_306eID_307e_305f_306f_540d_524d_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_:1`
  - `\![raise,イベント名,r0,r1,r2...]` — `ukadoc:list_sakura_script:_5c_21_5braise_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1`
  - `\![raiseother,ゴースト名,イベント名,r0,r1,r2...]` — `ukadoc:list_sakura_script:_5c_21_5braiseother_2c_30b4_30fc_30b9_30c8_540d_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1`
  - `\![raiseplugin,プラグインのIDまたは名前,イベント名,r0,r1,r2...]` — `ukadoc:list_sakura_script:_5c_21_5braiseplugin_2c_30d7_30e9_30b0_30a4_30f3_306eID_307e_305f_306f_540d_524d_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2:1`

#### インストールと更新（6 件）

- **利用者に何が起きるか**: 導入も更新も始まらない。新しい版が出ても取り込めず、「更新しておいたよ」という台詞だけが残る。
- **その群を成立させる最小の基盤**: 配布物を取ってきて入れ替える部分と、その開始を名前で受け取る口。
- **台帳の項目 id**:
  - `\![execute,install,path,ファイル名]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2cinstall_2cpath_2c_30d5_30a1_30a4_30eb_540d_5d:1`
  - `\![execute,install,url,URL,(feed\|nar\|homeurlのいずれか)]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2cinstall_2curl_2cURL_2c_28feed_7cnar_7chomeurl_306e_3044_305a_308c_304b_29_5d:1`
  - `\![update,更新対象(,オプション,オプション...)]` — `ukadoc:list_sakura_script:_5c_21_5bupdate_2c_66f4_65b0_5bfe_8c61_28_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._29_5d:1`
  - `\![update,platform]` — `ukadoc:list_sakura_script:_5c_21_5bupdate_2cplatform_5d:1`
  - `\![updatebymyself(,オプション,オプション...)]` — `ukadoc:list_sakura_script:_5c_21_5bupdatebymyself_28_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._29_5d:1`
  - `\![updateother,更新対象/オプション群,...]` — `ukadoc:list_sakura_script:_5c_21_5bupdateother_2c_66f4_65b0_5bfe_8c61_2f_30aa_30d7_30b7_30e7_30f3_7fa4_2c..._5d:1`

#### 描き直しと移動の凍結（6 件）

- **利用者に何が起きるか**: まとめて描き替えたい場面で描き直しを止められず、ちらつきが残る。バルーンの追随も止められない。止める側も戻す側も効かない。
- **その群を成立させる最小の基盤**: 描き直しと追随を一時的に止めておく仕組みと、その入切を受け取る口。
- **台帳の項目 id**:
  - `\![lock,balloonmove]` — `ukadoc:list_sakura_script:_5c_21_5block_2cballoonmove_5d:1`
  - `\![lock,balloonrepaint]` — `ukadoc:list_sakura_script:_5c_21_5block_2cballoonrepaint_5d:1`
  - `\![lock,repaint]` — `ukadoc:list_sakura_script:_5c_21_5block_2crepaint_5d:1`
  - `\![unlock,balloonmove]` — `ukadoc:list_sakura_script:_5c_21_5bunlock_2cballoonmove_5d:1`
  - `\![unlock,balloonrepaint]` — `ukadoc:list_sakura_script:_5c_21_5bunlock_2cballoonrepaint_5d:1`
  - `\![unlock,repaint]` — `ukadoc:list_sakura_script:_5c_21_5bunlock_2crepaint_5d:1`

#### 時間差で起こすイベント（6 件）

- **利用者に何が起きるか**: 「あとで」「くり返し」の予約が 1 つも積まれない。待っても何も起きない。
- **その群を成立させる最小の基盤**: 時刻を持った予約を積んで、時が来たらイベントを起こす仕組み。相手のイベント名は引数で決まるので、受け口は名前を 1 つに決め打ちできない。
- **台帳の項目 id**:
  - `\![timernotify,時間,繰り返すか否か,イベント名,r0,r1,r2...]` — `ukadoc:list_sakura_script:_5c_21_5btimernotify_2c_6642_9593_2c_7e70_308a_8fd4_3059_304b_5426_304b_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1`
  - `\![timernotifyother,時間,繰り返すか否か,ゴースト名,イベント名,r0,r1,r2...]` — `ukadoc:list_sakura_script:_5c_21_5btimernotifyother_2c_6642_9593_2c_7e70_308a_8fd4_3059_304b_5426_304b_2c_30b4_30fc_30b9_30c8_540d_2c_30a4_30d9_30:1`
  - `\![timernotifyplugin,時間,繰り返すか否か,プラグインのIDまたは名前,イベント名,r0,r1,r2...]` — `ukadoc:list_sakura_script:_5c_21_5btimernotifyplugin_2c_6642_9593_2c_7e70_308a_8fd4_3059_304b_5426_304b_2c_30d7_30e9_30b0_30a4_30f3_306eID_307e_30:1`
  - `\![timerraise,時間,繰り返すか否か,イベント名,r0,r1,r2...]` — `ukadoc:list_sakura_script:_5c_21_5btimerraise_2c_6642_9593_2c_7e70_308a_8fd4_3059_304b_5426_304b_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1`
  - `\![timerraiseother,時間,繰り返すか否か,ゴースト名,イベント名,r0,r1,r2...]` — `ukadoc:list_sakura_script:_5c_21_5btimerraiseother_2c_6642_9593_2c_7e70_308a_8fd4_3059_304b_5426_304b_2c_30b4_30fc_30b9_30c8_540d_2c_30a4_30d9_30f:1`
  - `\![timerraiseplugin,時間,繰り返すか否か,プラグインのIDまたは名前,イベント名,r0,r1,r2...]` — `ukadoc:list_sakura_script:_5c_21_5btimerraiseplugin_2c_6642_9593_2c_7e70_308a_8fd4_3059_304b_5426_304b_2c_30d7_30e9_30b0_30a4_30f3_306eID_307e_305:1`

#### 表示の速さと区間（6 件）

- **利用者に何が起きるか**: 間の取り方が作者の書いたとおりにならない。瞬間表示・累計の待ち・アニメーションの完了待ちが効かず、自動改行の停止や同時発話の指定も落ちる。マウスの反応を止める指定も効かない。
- **その群を成立させる最小の基盤**: 文字を出す速さと待ちを台本から動かす道と、区間の指定を覚えておく仕組み。
- **台帳の項目 id**:
  - `\__w[時間]` — `ukadoc:list_sakura_script:_5c__w_5b_6642_9593_5d:1`
  - `\__w[animation,ID]` — `ukadoc:list_sakura_script:_5c__w_5banimation_2cID_5d:1`
  - `\_n` — `ukadoc:list_sakura_script:_5c_n:1`
  - `\_q` — `ukadoc:list_sakura_script:_5c_q:1`
  - `\_s[ID1,ID2,ID3...]` — `ukadoc:list_sakura_script:_5c_s_5bID1_2cID2_2cID3..._5d:1`
  - `\t` — `ukadoc:list_sakura_script:_5ct:1`

#### 開発と診断の窓（6 件）

- **利用者に何が起きるか**: 作者が中を覗くための窓が開かない。利用者がゴーストとして失うものは無い。
- **その群を成立させる最小の基盤**: 診断用の窓と、その開閉・設定を台本から指示する道。
- **台帳の項目 id**:
  - `\![execute,dumpsurface,ディレクトリ,スコープID,サーフェスリスト,prefix,イベントID,ゼロ位置切り出し]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2cdumpsurface_2c_30c7_30a3_30ec_30af_30c8_30ea_2c_30b9_30b3_30fc_30d7ID_2c_30b5_30fc_30d5_30a7_30b9_30e:1`
  - `\![open,developer]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cdeveloper_5d:1`
  - `\![open,errorlog]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cerrorlog_5d:1`
  - `\![open,shiorirequest]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cshiorirequest_5d:1`
  - `\![reload,aigraph]` — `ukadoc:list_sakura_script:_5c_21_5breload_2caigraph_5d:1`
  - `\![set,shioridebugmode,(true/false)]` — `ukadoc:list_sakura_script:_5c_21_5bset_2cshioridebugmode_2c_28true_2ffalse_29_5d:1`

#### OS まわりの操作（5 件）

- **利用者に何が起きるか**: 壁紙の差し替え・控え・戻し、ごみ箱を空にする、起動用の近道を作る、のどれも起きない。
- **その群を成立させる最小の基盤**: OS の設定へ手を伸ばす部分と、その指示を名前で受け取る口。
- **台帳の項目 id**:
  - `\![create,shortcut]` — `ukadoc:list_sakura_script:_5c_21_5bcreate_2cshortcut_5d:1`
  - `\![execute,emptyrecyclebin]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2cemptyrecyclebin_5d:1`
  - `\![restore,wallpaper]` — `ukadoc:list_sakura_script:_5c_21_5brestore_2cwallpaper_5d:1`
  - `\![save,wallpaper]` — `ukadoc:list_sakura_script:_5c_21_5bsave_2cwallpaper_5d:1`
  - `\![set,wallpaper,ファイル名,オプション]` — `ukadoc:list_sakura_script:_5c_21_5bset_2cwallpaper_2c_30d5_30a1_30a4_30eb_540d_2c_30aa_30d7_30b7_30e7_30f3_5d:1`

#### WebSocket と通信の中止（5 件）

- **利用者に何が起きるか**: 常時つないでおく通信が張れず、走っている通信を取り消すこともできない。通信を使う筋書きが丸ごと成立しない。
- **その群を成立させる最小の基盤**: WebSocket をつなぐ・送る・切る部分と、走っている通信を取り消す道。
- **台帳の項目 id**:
  - `\![cancel,http,URL]` — `ukadoc:list_sakura_script:_5c_21_5bcancel_2chttp_2cURL_5d:1`
  - `\![cancel,websocket,URL]` — `ukadoc:list_sakura_script:_5c_21_5bcancel_2cwebsocket_2cURL_5d:1`
  - `\![execute,websocket,URL,オプション,オプション,オプション...]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2cwebsocket_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..:1`
  - `\![send,websocket-binary,URL,base64data]` — `ukadoc:list_sakura_script:_5c_21_5bsend_2cwebsocket-binary_2cURL_2cbase64data_5d:1`
  - `\![send,websocket,URL,data1,data2,...]` — `ukadoc:list_sakura_script:_5c_21_5bsend_2cwebsocket_2cURL_2cdata1_2cdata2_2c..._5d:1`

#### ゴーストの切り替え（5 件）

- **利用者に何が起きるか**: 別のゴーストを呼ぶ・別のゴーストへ交代する・見た目一式を着替える、のどれも起きない。台詞では相方が来たことになっているのに、画面には誰も現れない。
- **その群を成立させる最小の基盤**: ゴーストとシェルを入れ替える部分と、その指示を受け取る口。`\+`・`\_+` はタグを読み替える所のどの分岐にも当たらず素通しになり、`\![call,ghost,…]` ほか 3 件は名前が運ばれるだけである。
- **台帳の項目 id**:
  - `\![call,ghost,ゴースト名(,--option=raise-event)]` — `ukadoc:list_sakura_script:_5c_21_5bcall_2cghost_2c_30b4_30fc_30b9_30c8_540d_28_2c--option_3draise-event_29_5d:1`
  - `\![change,ghost,ゴースト名(,--option=raise-event)]` — `ukadoc:list_sakura_script:_5c_21_5bchange_2cghost_2c_30b4_30fc_30b9_30c8_540d_28_2c--option_3draise-event_29_5d:1`
  - `\![change,shell,シェル名(,--option=raise-event)]` — `ukadoc:list_sakura_script:_5c_21_5bchange_2cshell_2c_30b7_30a7_30eb_540d_28_2c--option_3draise-event_29_5d:1`
  - `\+` — `ukadoc:list_sakura_script:_5c_2b:1`
  - `\_+` — `ukadoc:list_sakura_script:_5c__2b:1`

#### 外部のアプリに渡す（5 件）

- **利用者に何が起きるか**: 見せたかった頁も、開かせたかったファイルも、宛先を入れたメールの下書きも出ない。
- **その群を成立させる最小の基盤**: 外のアプリケーションへ引き渡す部分と、その指示を名前で受け取る口。
- **台帳の項目 id**:
  - `\![open,browser,パラメータ]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cbrowser_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
  - `\![open,editor,ファイル,表示行]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2ceditor_2c_30d5_30a1_30a4_30eb_2c_8868_793a_884c_5d:1`
  - `\![open,explorer,ファイル]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cexplorer_2c_30d5_30a1_30a4_30eb_5d:1`
  - `\![open,file,ファイル名]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cfile_2c_30d5_30a1_30a4_30eb_540d_5d:1`
  - `\![open,mailer,パラメータ]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cmailer_2c_30d1_30e9_30e1_30fc_30bf_5d:1`

#### 文字の色と縁取りと影（5 件）

- **利用者に何が起きるか**: 文字色・白抜き・影の色と形が効かず、バルーン設定の色のまま出る。
- **その群を成立させる最小の基盤**: タグを読み替える所に `\f` の分岐を置き、装飾の状態を持って文字の描き方へ渡す道。
- **台帳の項目 id**:
  - `\f[color,色指定]` — `ukadoc:list_sakura_script:_5cf_5bcolor_2c_8272_6307_5b9a_5d:1`
  - `\f[outline,パラメータ]` — `ukadoc:list_sakura_script:_5cf_5boutline_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
  - `\f[shadowcolor,色指定]` — `ukadoc:list_sakura_script:_5cf_5bshadowcolor_2c_8272_6307_5b9a_5d:1`
  - `\f[shadowcolor,none]` — `ukadoc:list_sakura_script:_5cf_5bshadowcolor_2cnone_5d:1`
  - `\f[shadowstyle,形態指定]` — `ukadoc:list_sakura_script:_5cf_5bshadowstyle_2c_5f62_614b_6307_5b9a_5d:1`

#### キャラの移動と重なり（4 件）

- **利用者に何が起きるか**: 立ち位置の指示が全部落ちる。離れる・隣り合う・最前面に上げるのどれも起きないので、台詞では動いたことになっているのに、画面では 2 体が同じ場所に立ち続ける。
- **その群を成立させる最小の基盤**: 位置と重なりの指示を台本から受け取る道。`\4`・`\5`・`\v` はタグを読み替える所のどの分岐にも当たらず、そのまま素通しになり、`\![moveasync]` は名前だけが運ばれて受け口が無い。
- **台帳の項目 id**:
  - `\4` — `ukadoc:list_sakura_script:_5c4:1`
  - `\5` — `ukadoc:list_sakura_script:_5c5:1`
  - `\![moveasync]` — `ukadoc:list_sakura_script:_5c_21_5bmoveasync_5d:1`
  - `\v` — `ukadoc:list_sakura_script:_5cv:1`

#### バルーンへの画像貼り付け（4 件）

- **利用者に何が起きるか**: バルーンの中に画像が入らない。行の中へ挟む形も、座標を指定して貼る形も出ない。
- **その群を成立させる最小の基盤**: バルーンの文字層へ画像を置く道と、その引数を読み取る分岐。
- **台帳の項目 id**:
  - `\_b[ファイルパス,inline,オプション,オプション...]` — `ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cinline_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._5d:1`
  - `\_b[ファイルパス,inline,opaque]` — `ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cinline_2copaque_5d:1`
  - `\_b[ファイルパス,x,y,オプション,オプション...]` — `ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cx_2cy_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._5d:1`
  - `\_b[ファイルパス,x,y,opaque]` — `ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cx_2cy_2copaque_5d:1`

#### 外部を開く・別窓（4 件）

- **利用者に何が起きるか**: 飛ばしたかった先が開かない。伝言箱・教え込みの窓も、別のソフトへの受け渡しも起きない。
- **その群を成立させる最小の基盤**: 外部を開く道と、その区間・引数を読み取る分岐。
- **台帳の項目 id**:
  - `\__c` — `ukadoc:list_sakura_script:_5c__c:1`
  - `\__t` — `ukadoc:list_sakura_script:_5c__t:1`
  - `\j[ID]` — `ukadoc:list_sakura_script:_5cj_5bID_5d:1`
  - `\m[umsg,wparam,lparam]` — `ukadoc:list_sakura_script:_5cm_5bumsg_2cwparam_2clparam_5d:1`

#### 太字・斜体・上下付き（4 件）

- **利用者に何が起きるか**: 太字・斜体・上付き・下付きが効かず、地の書体のまま出る。
- **その群を成立させる最小の基盤**: タグを読み替える所に `\f` の分岐を置き、装飾の状態を持って文字の描き方へ渡す道。
- **台帳の項目 id**:
  - `\f[bold,パラメータ]` — `ukadoc:list_sakura_script:_5cf_5bbold_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
  - `\f[italic,パラメータ]` — `ukadoc:list_sakura_script:_5cf_5bitalic_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
  - `\f[sub,パラメータ]` — `ukadoc:list_sakura_script:_5cf_5bsub_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
  - `\f[sup,パラメータ]` — `ukadoc:list_sakura_script:_5cf_5bsup_2c_30d1_30e9_30e1_30fc_30bf_5d:1`

#### 書庫と配布物の作成（4 件）

- **利用者に何が起きるか**: 配布用のファイルや更新用のデータが作られない。書庫の作成と展開も起きない。
- **その群を成立させる最小の基盤**: 書庫を読み書きする部分と、その実行を名前で受け取る口。
- **台帳の項目 id**:
  - `\![execute,compressarchive,ファイル名,ディレクトリ名,オプション...]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2ccompressarchive_2c_30d5_30a1_30a4_30eb_540d_2c_30c7_30a3_30ec_30af_30c8_30ea_540d_2c_30aa_30d7_30b7_3:1`
  - `\![execute,createnar]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2ccreatenar_5d:1`
  - `\![execute,createupdatedata]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2ccreateupdatedata_5d:1`
  - `\![execute,extractarchive,ファイル名,ディレクトリ名,オプション...]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2cextractarchive_2c_30d5_30a1_30a4_30eb_540d_2c_30c7_30a3_30ec_30af_30c8_30ea_540d_2c_30aa_30d7_30b7_30:1`

#### 画面効果のプラグイン（4 件）

- **利用者に何が起きるか**: 立ち絵の切り替えや画面全体に掛ける効果が 1 つも掛からず、そのまま切り替わる。効果を解く側も効かない。
- **その群を成立させる最小の基盤**: 画面効果のプラグインを呼び出す仕組みと、その呼び出しを名前で受け取る口。
- **台帳の項目 id**:
  - `\![effect2,追加サーフェスID,プラグイン名,速度倍率,パラメータ]` — `ukadoc:list_sakura_script:_5c_21_5beffect2_2c_8ffd_52a0_30b5_30fc_30d5_30a7_30b9ID_2c_30d7_30e9_30b0_30a4_30f3_540d_2c_901f_5ea6_500d_7387_2c_30d1:1`
  - `\![effect,プラグイン名,速度倍率,パラメータ]` — `ukadoc:list_sakura_script:_5c_21_5beffect_2c_30d7_30e9_30b0_30a4_30f3_540d_2c_901f_5ea6_500d_7387_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
  - `\![filter,プラグイン名,起動時間,パラメータ]` — `ukadoc:list_sakura_script:_5c_21_5bfilter_2c_30d7_30e9_30b0_30a4_30f3_540d_2c_8d77_52d5_6642_9593_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
  - `\![filter]` — `ukadoc:list_sakura_script:_5c_21_5bfilter_5d:1`

#### 通信箱と教え込み箱（4 件）

- **利用者に何が起きるか**: 他のゴーストへ話しかける窓も、言葉を教え込む窓も開かない。閉じる側も同じく効かないので、開いて閉じる一組がまるごと無い。
- **その群を成立させる最小の基盤**: 話しかけ用・教え込み用の入力窓と、それを台本から開閉する道。
- **台帳の項目 id**:
  - `\![close,communicatebox]` — `ukadoc:list_sakura_script:_5c_21_5bclose_2ccommunicatebox_5d:1`
  - `\![close,teachbox]` — `ukadoc:list_sakura_script:_5c_21_5bclose_2cteachbox_5d:1`
  - `\![open,communicatebox]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2ccommunicatebox_5d:1`
  - `\![open,teachbox]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cteachbox_5d:1`

#### 他のゴーストとの付き合い方（3 件）

- **利用者に何が起きるか**: 他のゴーストの発話や立ち絵の変化を受け取るかどうか、口の動きを台詞に合わせるかどうかを切り替えられず、既定のままになる。
- **その群を成立させる最小の基盤**: 受け取り方の設定を持っておき、台本から切り替える道。
- **台帳の項目 id**:
  - `\![set,otherghosttalk,true\|false\|before\|after]` — `ukadoc:list_sakura_script:_5c_21_5bset_2cotherghosttalk_2ctrue_7cfalse_7cbefore_7cafter_5d:1`
  - `\![set,othersurfacechange,trueかfalse]` — `ukadoc:list_sakura_script:_5c_21_5bset_2cothersurfacechange_2ctrue_304bfalse_5d:1`
  - `\![set,serikotalk,true/false]` — `ukadoc:list_sakura_script:_5c_21_5bset_2cserikotalk_2ctrue_2ffalse_5d:1`

#### 文字コードの埋め込みと実体参照（3 件）

- **利用者に何が起きるか**: 文字そのものを符号で書き入れる 3 通りの書き方が、どれも文字にならない。出したかった 1 文字が画面から消える。
- **その群を成立させる最小の基盤**: 符号から 1 文字を作って本文へ差し込む道。3 件ともタグを読み替える所のどの分岐にも当たらず、そのまま素通しになる。
- **台帳の項目 id**:
  - `\&[ID]` — `ukadoc:list_sakura_script:_5c_26_5bID_5d:1`
  - `\_m[0x00]` — `ukadoc:list_sakura_script:_5c_m_5b0x00_5d:1`
  - `\_u[0x0000]` — `ukadoc:list_sakura_script:_5c_u_5b0x0000_5d:1`

#### アンカー（2 件）

- **利用者に何が起きるか**: 押せる文字にならない。押しても何も起きないので、利用者が言葉を返す道が丸ごと無くなる。
- **その群を成立させる最小の基盤**: 文の範囲をアンカーとして覚え、押されたらイベントを起こす仕組み。
- **台帳の項目 id**:
  - `\_a[ID,r2,r3...]` — `ukadoc:list_sakura_script:_5c_a_5bID_2cr2_2cr3..._5d:1`
  - `\_a[OnID,r0,r1...]` — `ukadoc:list_sakura_script:_5c_a_5bOnID_2cr0_2cr1..._5d:1`

#### サーフェスの拡大縮小（2 件）

- **利用者に何が起きるか**: 立ち絵の大きさが変わらない。倍率の変化にかける時間や完了待ちも指定できない。
- **その群を成立させる最小の基盤**: 立ち絵の拡大縮小を台本から指示する道。
- **台帳の項目 id**:
  - `\![set,scaling,倍率]` — `ukadoc:list_sakura_script:_5c_21_5bset_2cscaling_2c_500d_7387_5d:1`
  - `\![set,scaling,横倍率,縦倍率,オプション]` — `ukadoc:list_sakura_script:_5c_21_5bset_2cscaling_2c_6a2a_500d_7387_2c_7e26_500d_7387_2c_30aa_30d7_30b7_30e7_30f3_5d:1`

#### タグを実行しない区間（2 件）

- **利用者に何が起きるか**: タグをそのまま見せたい区間で、囲んだ中のタグが実行されてしまう。見せるつもりの綴りが別の動きに化ける。
- **その群を成立させる最小の基盤**: 区間の始まりと終わりを覚えておき、その中ではタグを読み替えない仕組み。
- **台帳の項目 id**:
  - `\_!` — `ukadoc:list_sakura_script:_5c__21:1`
  - `\_?` — `ukadoc:list_sakura_script:_5c__3f:1`

#### ネットワークの調べもの（2 件）

- **利用者に何が起きるか**: 名前解決や疎通の確認が走らず、結果を知らせるイベントも返らない。
- **その群を成立させる最小の基盤**: 調べものを実行する部分と、結果をイベントで返す道。
- **台帳の項目 id**:
  - `\![execute,nslookup,パラメータ1,パラメータ2,...]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2cnslookup_2c_30d1_30e9_30e1_30fc_30bf1_2c_30d1_30e9_30e1_30fc_30bf2_2c..._5d:1`
  - `\![execute,ping,パラメータ1,パラメータ2,...]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2cping_2c_30d1_30e9_30e1_30fc_30bf1_2c_30d1_30e9_30e1_30fc_30bf2_2c..._5d:1`

#### バルーンの追記と選択肢のタイムアウト抑止（2 件）

- **利用者に何が起きるか**: 前のトークへ書き足すつもりの文が、バルーンをいったん空にしてから出る。選択肢は待ち時間で勝手に消える。どちらも作者の意図と逆に見える。
- **その群を成立させる最小の基盤**: バルーンを消さずに書き足す道と、選択肢の待ち時間を止める道。2 件ともタグを読み替える所のどの分岐にも当たらず、そのまま素通しになる。
- **台帳の項目 id**:
  - `\C` — `ukadoc:list_sakura_script:_5cC:1`
  - `\*` — `ukadoc:list_sakura_script:_5c_2a:1`

#### バルーン内の部分消去（2 件）

- **利用者に何が起きるか**: 書いた文字の一部を消せず、消したい文字や行が残る。
- **その群を成立させる最小の基盤**: バルーンの文字を位置で数えて部分的に消す道。なお引数なしの全消去は既に動いている。
- **台帳の項目 id**:
  - `\c[char,数値,開始位置]` — `ukadoc:list_sakura_script:_5cc_5bchar_2c_6570_5024_2c_958b_59cb_4f4d_7f6e_5d:1`
  - `\c[line,数値,開始位置]` — `ukadoc:list_sakura_script:_5cc_5bline_2c_6570_5024_2c_958b_59cb_4f4d_7f6e_5d:1`

#### プロパティの照会と設定（2 件）

- **利用者に何が起きるか**: 値を尋ねる要求も、値を書き込む要求も届かない。尋ねた先で値を使う台本が動かない。
- **その群を成立させる最小の基盤**: プロパティを読み書きする経路と、尋ねた結果を名指しのイベントで返す道。
- **台帳の項目 id**:
  - `\![get,property,イベント名,プロパティ名,プロパティ名,...]` — `ukadoc:list_sakura_script:_5c_21_5bget_2cproperty_2c_30a4_30d9_30f3_30c8_540d_2c_30d7_30ed_30d1_30c6_30a3_540d_2c_30d7_30ed_30d1_30c6_30a3_540d_2c:1`
  - `\![set,property,プロパティ名,値]` — `ukadoc:list_sakura_script:_5c_21_5bset_2cproperty_2c_30d7_30ed_30d1_30c6_30a3_540d_2c_5024_5d:1`

#### ヘッドラインセンサ（2 件）

- **利用者に何が起きるか**: 外から見出しを取ってきて読み上げる筋書きが成立しない。取得先を並べた窓も開かない。
- **その群を成立させる最小の基盤**: 見出しを取りに行く部分と、取ってきた結果をイベントで返す道。
- **台帳の項目 id**:
  - `\![execute,headline,ヘッドライン名]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2cheadline_2c_30d8_30c3_30c9_30e9_30a4_30f3_540d_5d:1`
  - `\![open,headlinesensorexplorer]` — `ukadoc:list_sakura_script:_5c_21_5bopen_2cheadlinesensorexplorer_5d:1`

#### 同期オブジェクト（2 件）

- **利用者に何が起きるか**: 2 体の足並みをそろえるための印が立たず、下ろすこともできない。
- **その群を成立させる最小の基盤**: 印を立てて待ち合わせる仕組みと、その入切を受け取る口。
- **台帳の項目 id**:
  - `\![reset,syncobject,同期オブジェクト名]` — `ukadoc:list_sakura_script:_5c_21_5breset_2csyncobject_2c_540c_671f_30aa_30d6_30b8_30a7_30af_30c8_540d_5d:1`
  - `\![set,syncobject,同期オブジェクト名]` — `ukadoc:list_sakura_script:_5c_21_5bset_2csyncobject_2c_540c_671f_30aa_30d6_30b8_30a7_30af_30c8_540d_5d:1`

#### 文字の書体と大きさ（2 件）

- **利用者に何が起きるか**: フォントと文字の大きさが変わらず、バルーン設定のまま出る。
- **その群を成立させる最小の基盤**: タグを読み替える所に `\f` の分岐を置き、装飾の状態を持って文字の描き方へ渡す道。
- **台帳の項目 id**:
  - `\f[height,数値]` — `ukadoc:list_sakura_script:_5cf_5bheight_2c_6570_5024_5d:1`
  - `\f[name,フォント名]` — `ukadoc:list_sakura_script:_5cf_5bname_2c_30d5_30a9_30f3_30c8_540d_5d:1`

#### 時計合わせ（2 件）

- **利用者に何が起きるか**: 時計を合わせるやりとりが、台詞だけで実が伴わない。ゴーストが「直しておいたよ」と言っても時刻は動かない。
- **その群を成立させる最小の基盤**: 外の時刻サーバへ問い合わせる部分と、それを台本から呼ぶ道。`\6` はタグを読み替える所のどの分岐にも当たらず素通しになり、`\![executesntp]` は名前が運ばれるだけである。
- **台帳の項目 id**:
  - `\6` — `ukadoc:list_sakura_script:_5c6:1`
  - `\![executesntp]` — `ukadoc:list_sakura_script:_5c_21_5bexecutesntp_5d:1`

#### 窓の位置の初期化（2 件）

- **利用者に何が起きるか**: 動かしてしまった窓やバルーンを、台本から既定の位置へ戻せない。
- **その群を成立させる最小の基盤**: 窓とバルーンの位置を既定へ戻す道。名前は運ばれるが受け口が無い。
- **台帳の項目 id**:
  - `\![execute,resetballoonpos]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2cresetballoonpos_5d:1`
  - `\![execute,resetwindowpos]` — `ukadoc:list_sakura_script:_5c_21_5bexecute_2cresetwindowpos_5d:1`

#### 装飾の一括の戻し（2 件）

- **利用者に何が起きるか**: 装飾をまとめて標準へ戻す指定と、無効表示へ切り替える指定が落ちる。ただし装飾そのものが 1 つも効いていないので、戻す側だけは画面に差が出ない。
- **その群を成立させる最小の基盤**: 装飾の状態を持つ仕組み。状態が無いので「戻す」対象もまだ無い。
- **台帳の項目 id**:
  - `\f[default]` — `ukadoc:list_sakura_script:_5cf_5bdefault_5d:1`
  - `\f[disable]` — `ukadoc:list_sakura_script:_5cf_5bdisable_5d:1`

#### 通知領域とデスクトップ（2 件）

- **利用者に何が起きるか**: 通知領域のアイコンや吹き出しが出ないので、画面の隅から知らせる形の演出が成立しない。
- **その群を成立させる最小の基盤**: 通知領域へ出す部分と、その指示を名前で受け取る口。
- **台帳の項目 id**:
  - `\![set,tasktrayicon,ファイル名.ico,テキスト(,--duration=待機時間(,--runcount=繰り返し回数))]` — `ukadoc:list_sakura_script:_5c_21_5bset_2ctasktrayicon_2c_30d5_30a1_30a4_30eb_540d.ico_2c_30c6_30ad_30b9_30c8_28_2c--duration_3d_5f85_6a5f_6642_959:1`
  - `\![set,trayballoon,オプション,オプション,オプション...]` — `ukadoc:list_sakura_script:_5c_21_5bset_2ctrayballoon_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._5d:1`

#### イベントを起こすだけのタグ（1 件）

- **利用者に何が起きるか**: 押しても喋り出さない。ランダムトークを促す仕掛けが働かない。
- **その群を成立させる最小の基盤**: このタグを読み取る分岐と、そこからイベントを起こす道。
- **台帳の項目 id**:
  - `\a` — `ukadoc:list_sakura_script:_5ca:1`

#### ゴーストの終了（1 件）

- **利用者に何が起きるか**: 自分から消える処理が始まらず、別れの台詞のあともゴーストは立ったままになる。
- **その群を成立させる最小の基盤**: 自分の終了を始める道。名前は運ばれるが受け口が無い。
- **台帳の項目 id**:
  - `\![vanishbymyself]` — `ukadoc:list_sakura_script:_5c_21_5bvanishbymyself_5d:1`

#### ネットワーク通信（1 件）

- **利用者に何が起きるか**: WebSocket をきちんと切る手続きが行われず、切断後のイベントも起きない。
- **その群を成立させる最小の基盤**: 通信の切断を名前で受け取る口と、切断を知らせるイベントを起こす道。
- **台帳の項目 id**:
  - `\![close,websocket,URL]` — `ukadoc:list_sakura_script:_5c_21_5bclose_2cwebsocket_2cURL_5d:1`

#### メールチェック（1 件）

- **利用者に何が起きるか**: メールの確認が始まらないので、「届いてるよ」と知らせる場面が成立しない。
- **その群を成立させる最小の基盤**: メールの有無を調べる部分と、その開始を名前で受け取る口。
- **台帳の項目 id**:
  - `\![biff(,アカウント名)]` — `ukadoc:list_sakura_script:_5c_21_5bbiff_28_2c_30a2_30ab_30a6_30f3_30c8_540d_29_5d:1`

#### 下線と打ち消し線（1 件）

- **利用者に何が起きるか**: 打ち消し線が引かれない。（同じ群の下線は、正典の語彙として登記だけがある扱いなのでこの一覧に入らない。）
- **その群を成立させる最小の基盤**: タグを読み替える所に `\f` の分岐を置き、装飾の状態を持って文字の描き方へ渡す道。
- **台帳の項目 id**:
  - `\f[strike,パラメータ]` — `ukadoc:list_sakura_script:_5cf_5bstrike_2c_30d1_30e9_30e1_30fc_30bf_5d:1`

#### 着せ替え（1 件）

- **利用者に何が起きるか**: イベントを起こさない形の着せ替えができない。衣装や小物の着脱を静かに切り替える書き方だけが効かない。
- **その群を成立させる最小の基盤**: 着せ替えの着脱をイベント無しで行う道。名前は運ばれるが受け口が無い。
- **台帳の項目 id**:
  - `\![bind-noevent,カテゴリ名,パーツ名,数値]` — `ukadoc:list_sakura_script:_5c_21_5bbind-noevent_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_6570_5024_5d:1`

#### 選択肢の別書式（1 件）

- **利用者に何が起きるか**: 囲んだ文が選択肢にならず、ふつうの台詞として流れる。自動改行も付かない。
- **その群を成立させる最小の基盤**: 囲み形の選択肢を読み取る分岐と、囲んだ範囲を押せる行にする道。
- **台帳の項目 id**:
  - `\__q[ID,...]` — `ukadoc:list_sakura_script:_5c__q_5bID_2c..._5d:1`

#### 選択肢マーカーの表示（1 件）

- **利用者に何が起きるか**: 選択肢の行頭に付くはずの印が描かれない。どこからどこまでが押せる行なのかが目で分かりにくくなる。
- **その群を成立させる最小の基盤**: 選択肢の行に印を描く部分と、その指示を名前で受け取る口。
- **台帳の項目 id**:
  - `\![*]` — `ukadoc:list_sakura_script:_5c_21_5b_2a_5d:1`

#### 音声合成（1 件）

- **利用者に何が起きるか**: 読み上げの抑止も読み替えも効かず、書いたとおりに読み上げられる。
- **その群を成立させる最小の基盤**: 読み上げへ渡す文字列を差し替える道。
- **台帳の項目 id**:
  - `\__v[オプション]` — `ukadoc:list_sakura_script:_5c__v_5b_30aa_30d7_30b7_30e7_30f3_5d:1`

### 別名だが綴りを受けない 3 件（未対応ではないが、利用者には壊れて見える）

この 3 件は台帳では別名である。
別名の多くは areka が正典の根と同じに扱うので作業が残らないが、この 3 件は**綴りそのものを受けない**ので、利用者から見た壊れ方が残る。上の群とは壊れ方の形が違うので、群として分けて載せる。
**3 件とも壊れ方が別々なので、1 件ずつ群にしてある。**

#### `\sID番号`（1 件）

- **利用者に何が起きるか**: 面が変わらないうえに、書いた数字が台詞に混じって画面へ出る。短縮形の語の表に `s` が無いので、綴りだけが 1 単位として切り出され、続く数字が本文へ回る。
- **その群を成立させる最小の基盤**: 短縮形の語の表へこの綴りを足し、角括弧形と同じ面切り替えへ写すこと。
- **台帳の項目 id**:
  - `\sID番号` — `ukadoc:list_sakura_script:_5csID_756a_53f7:1`

#### `\q[ID][タイトル]または\q*[ID][タイトル]`（1 件）

- **利用者に何が起きるか**: 選択肢が出ない。旧い書き方で書かれた選択肢が、台詞としても選択肢としても画面に現れない。
- **その群を成立させる最小の基盤**: 旧書式をいまの選択肢の形へ写すこと。いまは畳む処理がわざと選択肢にせず素通しへ落としている。
- **台帳の項目 id**:
  - `\q[ID][タイトル]または\q*[ID][タイトル]` — `ukadoc:list_sakura_script:_5cq_5bID_5d_5b_30bf_30a4_30c8_30eb_5d_307e_305f_306f_5cq_2a_5bID_5d_5b_30bf_30a4_30c8_30eb_5d:1`

#### `\z`（1 件）

- **利用者に何が起きるか**: トークが終わらない。次の独り言も次の反応も始まらないので、ゴーストが黙ったまま止まって見える。
- **その群を成立させる最小の基盤**: この綴りを、写像先と同じ「トークを終える」分岐へ写すこと。写像先の綴りは正典どおり終端しているので、写す先そのものは既にある。
- **台帳の項目 id**:
  - `\z` — `ukadoc:list_sakura_script:_5cz:1`

### この一覧に入らないもの

「何も起きない」に当たらない項目は、数だけをここに書いて中身は他の節に譲る。

| 台帳の状態 | 件数 | この一覧に入れない理由 |
| --- | ---: | --- |
| 実装済み | 23 | 正典どおり動く |
| 語彙だけが登記されている | 36 | 動かないが、正典の語彙として登記されていることが対応表に書かれている |
| 一部だけが効かない | 3 | 主要な用法は動いていて、書いた引数の一部だけが落ちる |
| 別名 | 20 | うち 17 件は areka が根と同じに扱うので壊れ方が無い。残る 3 件は上に群として載せた |
| 対象外 | 1 | この調査の対象外 |

---

## ⑹ 既存の brief と `doc/COMPAT_ARCHITECTURE.md` §8 への是正候補

**下に並べるのはすべて書き換えの案であって、決定ではない。**
本 spec は対応表 §8 と既存の brief を **1 文字も書き換えていない**。どこをどう直すかは、それぞれの文書の持ち主が決める。

**行を行番号で指していない。** 行番号は自分がその文書へ 1 行足しただけでずれるので、この節は「その行が何を主題にしているか」と「どの定義を指しているか」で指す。
**行番号で指す書き方そのものを改める案**も、下の 1 番に含めてある。

### 是正候補 1: 対応表 §8 の参照先が現在のソースとずれている

§8 の行は、根拠となるソースや文書を「ファイルと行番号」で指している。その参照を機械で全部抜くと **90 件**あり、そのうち**人が 1 件ずつ読んでずれを確かめたのが 14 件**である。

| # | §8 の行の主題 | いま指している先 | その位置に現在あるもの | 書き換えの案（指し先） |
| ---: | --- | --- | --- | --- |
| 1 | `\![set,balloontimeout,時間]` の実導出 | 完了済み `areka-P0-balloon-visibility` の brief の 1 行を「emo2 の辞書に現れず使う人が居ない」ことの根拠として引く | その行は「SSP 本体の設定の存在」を述べる別の話 | 同じ brief の「emo2 の実物（フィクスチャの実測）」の行へ |
| 2 | `OnBalloonClick` が正典に存在しないこと | 同じ brief の同じ行 | 同上 | 同じ brief の「emo2 の実物（フィクスチャの実測）」の行へ |
| 3 | `\x` ／ `\x[noclear]` の実物根拠 | 同じ brief の同じ行 | 同上 | 同じ brief の「emo2 の実物（フィクスチャの実測）」の行へ |
| 4 | `\x` ／ `\x[noclear]`（さくらスクリプトの縮退の行） | 角括弧なしの形が素通しへ落ちる場所を、タグを読み替える所の 1 行として指す | その位置にあるのは改行のタグの分岐 | 角括弧なしの形の**どの分岐にも当たらなかったときの行き先**の名前で指す |
| 5 | 同上 | 素通しにする関数を行番号で指す | その位置は関数の中の 1 行 | 角括弧なしの形の**素通しの関数**の定義で指す |
| 6 | 同上 | 角括弧形が素通しへ落ちる分岐を行番号で指す | その位置はカーソル位置の説明文 | 角括弧形の**どの分岐にも当たらなかったときの行き先**で指す |
| 7 | 同上 | 角括弧形の素通しの関数を行番号で指す | その位置は注釈の途中 | 角括弧形の**素通しの関数**の定義で指す |
| 8 | 縦書きのフォント異体の扱いが等しいこと | 文字の書式を作る関数を行番号で指す | その位置はその関数の説明文 | その**関数の定義**で指す |
| 9 | 同上 | 書字方向のレシピを作る関数を行番号で指す | その位置は欄の説明文 | その**関数の定義**で指す |
| 10 | 同上 | レシピを書式へ当てる 1 行を行番号で指す | その位置は記録の引数 | その**呼び出しの式**で指す |
| 11 | 同上 | 計測も同じ関数を通ることを行番号で指す | その位置は記録の開始行 | その**呼び出し**で指す |
| 12 | バルーンの位置のずらしの単位 | 「バルーン作者の空間で書かれた値だから」の根拠を配置の 1 行として指す | その位置は値の飽和を警告する処理の中 | 同じファイルの**冒頭の説明文**（作者基準の画素で書かれた画面上のずらし、の定義）へ |
| 13 | 同上 | ずらしの 2 段重ねの設定を行番号で指す | その位置は別の設定の分岐の閉じ括弧 | **2 段重ねを組み立てている式**で指す |
| 14 | 遷移の判定の出どころ | 出どころを表す文字列の定義を行番号で指す | その位置は別の定数 | その**文字列の定数の定義**で指す |

**本ドメインに直接効くのは 4 番から 7 番の 4 件**である。この 4 件は `\x` ／ `\x[noclear]` の行に属し、その行は台帳の 2 項目の判定の根拠そのものになっている——`\x[noclear]` を「語彙だけが登記されている」とし、`\x` をその別名とする判定である。

| 直接効く §8 の行 | 台帳の項目の綴り | 台帳の項目 id |
| --- | --- | --- |
| `\x` ／ `\x[noclear]` の行 | `\x` | `ukadoc:list_sakura_script:_5cx:1` |
| `\x` ／ `\x[noclear]` の行 | `\x[noclear]` | `ukadoc:list_sakura_script:_5cx_5bnoclear_5d:1` |

**あわせて、書き方そのものへの案を 1 つ出す。** 上の 14 件はいずれも「行番号で指したせいで、指し先だけが後から動いた」形である。参照を**定義の名前**（関数・定数・見出し）で指す書き方へ改めれば、ソースへ 1 行足しただけで参照が古びることは起きない。

### 是正候補 2: 上の 14 件を「ずれの全部」と読んではならない（道具の申告）

上の 14 件は、機械の判定をそのまま出したものではない。
**機械と人の答えは 6 か所で食い違った。**

| 機械の判定 | 人が読み直した結果 |
| --- | --- |
| 「ずれ」と判定した 14 件 | うち **3 件は誤検出**だった（実際にはずれていない） |
| 別の判定へ落とした行 | そのうち **3 件が実際にはずれ**だった |
| 「人が読む必要がある」と判定した **59 件** | **1 件も確かめていない** |
| 参照先のファイル名が行の中に無い **4 件** | 機械では照合できない |
| 参照先の名前が一意でない **1 件** | 同じ名前のファイルが多数あり、指し先が定まらない |

**したがって「ずれは 14 件だけだ」とは言えない。** 未確認の 59 件が残っている。
参照先のファイル名を行の中に必ず書く、名前が一意になる書き方をする、というのも書き方への案として挙げておく。

### 是正候補 3: brief に書かれた綴りが正典と食い違う

brief に書かれている綴りのうち、正典のどの項目にも当たらないものが **20 種**ある（全数は ⑶ 節の表 ⑶-4）。
3 つに分かれ、**書き換えの案があるのは 1 つ目だけ**である。

| 分類 | 種類 | 書き換えの案 |
| --- | ---: | --- |
| 本ドメインの綴りのつもりで書かれているが、正典に無い綴り | 4 | 下の表のとおり正典の綴りへ |
| 綴りではなく「書き方の記法」 | 4 | 書き換え不要。記法であることが読めればよい |
| 本ドメインの綴りではないもの（拾い過ぎ） | 12 | 書き換え不要。正規表現やパスの断片が走査に当たっただけ |

| brief に書かれた綴り | 正典の綴り | 書いている brief | 書き換え先の項目 id |
| --- | --- | --- | --- |
| `\![enter,nouserbreak]` | `\![enter,nouserbreakmode]` | `areka-P0-status-execution-states` | `ukadoc:list_sakura_script:_5c_21_5benter_2cnouserbreakmode_5d:1` |
| `quicksession` | `\![quicksection,true]`・`\![quicksection,false]` | `areka-P0-ukadoc-survey-sakura-script` | `ukadoc:list_sakura_script:_5c_21_5bquicksection_2ctrue_5d:1`・`ukadoc:list_sakura_script:_5c_21_5bquicksection_2cfalse_5d:1` |
| `vanish` | `\![vanishbymyself]` | `areka-P0-ukadoc-survey-sakura-script` | `ukadoc:list_sakura_script:_5c_21_5bvanishbymyself_5d:1` |
| `\![vanish]` | `\![vanishbymyself]` | `areka-P0-position-persist`・`areka-P0-sylphya`（どちらも完了済み） | `ukadoc:list_sakura_script:_5c_21_5bvanishbymyself_5d:1` |

**そのうえで、`areka-P0-status-execution-states` の 1 件だけは重い。** この綴りは、その項目の担当を主張している brief 自身が書いているものだからである。
あわせて、`\_a[id]` と小文字で書いている brief が 2 本ある（正典は `\_a[ID]`）。読み手には同じに見えるが、機械の照合では別物になる。

### 是正候補 4: 対応表 §8 に登記の無い食い違いがある（新しい行を足す案）

正典が定めているのに areka が別の応え方をしていて、しかもそのことが §8 に 1 行も書かれていない、という食い違いを見つけた。
**台帳の状態は動かしていない**（「一部だけが効かない」と判定する条件に「既に登記されている」が入っているため）。備考に食い違いの中身を書き、ここへ「§8 に新しい行を足す案」として出す。

| 項目の綴り | 台帳の項目 id | 登記の無い食い違い |
| --- | --- | --- |
| `\s[ID番号]` | `ukadoc:list_sakura_script:_5cs_5bID_756a_53f7_5d:1` | 面の別名を、相方側の別名のまとまりからしか読まない（本体側の別名のまとまりは repo に 1 つも無い） |
| `\s[ID番号]` | `ukadoc:list_sakura_script:_5cs_5bID_756a_53f7_5d:1` | 定義名による別名は、面の定義を読む処理にそのキーが無く、一切入らない |
| `\s[ID番号]` | `ukadoc:list_sakura_script:_5cs_5bID_756a_53f7_5d:1` | 同じ名前が複数の面 id を持つとき、先頭を固定で選ぶ（正典は選び方に触れていない） |
| `\![bind,カテゴリ名,パーツ名,数値]` | `ukadoc:list_sakura_script:_5c_21_5bbind_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_6570_5024_5d:1` | パーツ名を空にしたときのカテゴリ単位の動作と、数値を省いたときの入切の繰り返しが、どちらも読み飛ばされる。§8 に `\![bind]` を主題にする行は 1 つも無い |

### 是正候補 5: 本 spec の要件・設計の記述と実測が食い違う

本 spec 自身の文書についても、実測と食い違う記述をここへ出す。**この節では書き換えていない。**

| 対象の文書 | 直したい記述（主題で指す） | 実測 | 書き換えの案 |
| --- | --- | --- | --- |
| 要件（`\![...]` の消費側の項） | 内部の合図の名前が置いてあるファイルの場所 | そのファイルは実在せず、実在するのは別のクレートの同名のファイルである | 実在するファイルの場所へ書き換える。設計の「実測の訂正」の表には既に登記がある |
| 設計（担当の決定表の順 3） | 「2 本が主張し分担が合意済み」に当たるのは 4 件 | **3 件**。挙げられた 4 件のうち 1 件は別名なので、決定表の順 1 が先に当たる | 「4 件」を「3 件」に改め、別名の 1 件は順 1 で止まると書き添える |
| 要件（黙って壊れる形の項） | 黙って壊れる形は 3 通り | **4 つ目の形がある**。一部だけが効かない項目は、タグ全体ではなく**書いた引数の一部だけ**が落ちる（選択肢は出るのに、2 番目以降の指定だけが効かない） | 4 つ目として書き足す |
| 要件（担当の転記の項） | 対応表 §8 のうち本ドメインを主題にする行は「21 行」 | この 21 行は**人が読んで判定した数**である。同じ 21 行に対して「コードの記法で綴りが書かれているか」だけを機械で当てると**17 行**しか当たらない。当たらない 4 行は、タグ名を素の語で並べる行・主題が別ドメインで説明の中に綴りが出る行・記法で書かれた行である | 「21 行」が人の判定であることを書き添え、機械で数え直すときの物差しも一緒に書く。**要件が併記する内訳（タグ 17 行＋`%` 4 行）とは別の分け方**なので、2 つの 17 を取り違えないよう書き分ける |

### 是正候補 6: 上流の道具と、この調査の材料への案

| 対象 | 直したいところ | 書き換えの案 |
| --- | --- | --- |
| 上流の報告を作り直す道具 | 報告を必ず改行 1 文字で書き出すが、この作業ツリーは改行 2 文字である。そのため**台帳が 1 文字も変わっていない他の 3 ドメインの報告まで、改行だけが書き換わって差分に出る**。4 つのドメインの担当が報告を作り直すたびに起きる | 作業ツリーの改行に合わせて書き出す |
| 上流の常設の検査 | **一方向しか見ていない。** 「実装済みなのにソースに正典の URL が無い」は捕まえるが、**「一部だけが効かない」と「語彙だけが登記されている」は証拠を要求されない側**なので、判定の根拠にした登記が上流で取り下げられても検査は全部緑のままになる。この調査は実際に 1 件踏んだ（申し送り ⑹） | **状態の根拠が指す登記の実在を見る検査**を足す。備考が名指す対応表の行が今も在るか、その行が今も食い違いを登記しているかを機械で見る |
| この調査の走査の台本 | 対応表 §8 の行の主題を材料へ写すとき、**70 文字で切っている**。そのため ⑶ 節の表 ⑶-2 の主題も切れている | 切らずに写すよう台本を直す。台本を直さないかぎり、ブリーフィングの側だけでは主題を復元できない |
| 台帳の備考の札 | 語彙だけが登記されている 3 件に、本来は「一部だけが効かない」項目のための札を使っている | 語彙の登記のための書き方に揃える。既に埋まっている項目まで一括で揃えるかどうかは持ち主が決める |
| 台帳の備考の言い回し | 受け口の数を「3 つ」と書いている項目と、数を書かずに済ませている項目がある。**どこまでを受け口と数えるかで数が変わる** | どちらかに寄せるか、数え方を 1 度だけ定義する。いずれにせよ状態の判定には影響しない（どの数え方でも受け手は 0 件） |

### 是正候補 7: この文書自身が読み違えられやすいところ

| どこ | 何が起きるか | 書き換えの案 |
| --- | --- | --- |
| ⑶ 節の表 ⑶-1 | 主張の型が「例示」なのに担当が入っている行が **7 行**ある。この表だけを読むと矛盾に見える | 型は brief だけを見た判定で、担当は対応表 §8 の行からも入る。その断りは既に ⑶ 節に置いてあるが、表のすぐ下にも置くと読み違いが減る |
| 担当の一覧の在りか | 機械で作り直す報告には担当の列が無い。担当を一覧で読める場所は ⑶ 節と ⑷ 節だけである | 報告に担当の列を足すか、この文書が唯一の在りかであることを報告の側にも書く |

### すでに解消していて、是正候補として出さないもの

作業の途中で是正候補に挙がったが、**その後の改訂で既に直っている**ものがある。
統合担当が二重に起票しないよう、落とした理由と一緒に残す。

| 挙がっていた候補 | 落とす理由 |
| --- | --- |
| 設計とタスクが「areka が綴りを受けない別名は 2 件」と書いている | 現在の設計とタスクはいずれも **3 件**に改まっており、3 件目も名指しされている |
| 設計の調べものが、複数の選択肢 id を取る形の縮退を拾い落としている | 設計とタスクを改訂済み。縮退は 3 件・実装済みは 23 件で確定している |
| 設計が「語彙だけの登記」の例を 2 綴りしか挙げていない | 設計を改訂済み。3 綴り目も §8 が逐語で名指ししている |
| 要件と設計が、裁定待ちの 2 件の理由を同じに書いている | 要件と設計はどちらも改訂済みで、「互いに相手を知らない」と「名指しは一方向で分担が未確定」を書き分けている |
| 要件と設計が、本文が 2 桁の版だけを書く項目の列挙に `\![move]` を入れている | どちらも改訂済み。この項目の本文に版の区画は無く、2 桁の数は「2.5 秒かけて移動する」の**所要時間**だと書き添えられている |
| 要件が、本文が 2 桁の版だけを書くのは「5 件」と書いている | 改訂済み。**行の数と項目 id の数を取り違えていた**ことが明記され、id で数え直した **6 件**へ改まっている |

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

---

## 末尾の注記

### 4 つの台帳がそろった状態で確かめたこと

この調査は当初、**他の 3 つの台帳がそろうまで確かめられない検査**を末尾へ注記として残す予定だった。
**その前提はもう成り立たない。** 他の 3 つの台帳も同じ作業ツリーに揃っているので、注記ではなく**実際に回した結果**を書く。

**「カタログのすべての項目が、4 つの台帳のどれか 1 つにちょうど 1 回だけ現れる」**——これが確かめたかった検査である。
上流の常設の検査がこの検査を持っているので、この作業ツリーでそのまま回した。

| 数えたもの | 件数 |
| --- | ---: |
| 台帳 `assets.toml` の項目 | 542 |
| 台帳 `property.toml` の項目 | 188 |
| 台帳 `sakura-script.toml` の項目 | 342 |
| 台帳 `shiori.toml` の項目 | 677 |
| 4 つの台帳の合計 | 1749 |
| カタログの項目 | 1749 |
| どの台帳にも無いカタログの項目 | **0 件** |
| 2 つ以上の台帳に現れる項目 | **0 件** |

**1749 件が 1749 件へ、ちょうど 1 対 1 で対応している。** どの台帳にも無い項目も 0 件、2 つ以上の台帳に現れる項目も 0 件である。
この 0 は引き算で導いたものではなく、4 つの台帳の項目をすべて集めてから数え上げた結果である。

**この数は、担当ドメインの報告を台帳から作り直したあとに取り直したものである。** 作り直す前に採った数と 1 件も違わない。

常設の検査をこの作業ツリーで回した結果は、**所見 0 件**である（終了コード 0・本文の 1 行目が「食い違い 0 件」）。**15 種類の所見がどれも 1 件も出ていない。** 上の表で 0 件と書いた 2 つ——どの台帳にも無いカタログの項目・2 つ以上の台帳に現れる項目——も、この 15 種類に含まれる。

**この緑は「検査が黙っていた」結果ではない。** 調査の途中では同じコマンドが「実装済みとした項目のソースに正典の URL が置かれていない」を 23 件出し続けており、ソースへ正典の URL を置いた段でその 23 件が 0 件へ落ちた。さらに 15 種類それぞれが今日の実データの上に 1 件以上の対象を持つことを、判定を呼ばずに別の道筋から数え直して確かめてある（**対象が 0 件の種類は 0 種**。内訳は下の「検査の結果」の表）。

**1 種類だけ但し書きが要る。** 「台帳の前置きに書いたドメイン名が食い違う」の 1 種は、検査の段では作りようがない——前置きの食い違いは台帳を読み込む段で落ちるので、検査へ渡る時点で既に食い違いが無いからである。この種の「0 件」は、台帳 4 本が読めた事実そのものが証拠になっている。

### 優先度の段階 A〜E は仮の位置づけである

**この調査は段階 A〜E の最終順序を決めていない。** 決めるのは統合担当（`areka-P0-ukadoc-coverage-roadmap`）である。
台帳に書いた優先度は、**段の中の並びだけを作った仮置き**である。

| 優先度 | 件数 | 中身 |
| --- | ---: | --- |
| `C10` | 1 | 書いた引数が台詞へ漏れる |
| `C20` | 57 | 書いてあるのに何も起きず、無いと利用者が何を失うかを言える |
| `C30` | 24 | `%` で始まる語が、綴りのまま台詞に出る |
| `C40` | 3 | 主要な用法は動いていて、書いた一部だけが効かない |
| `C50` | 6 | 語彙としては登記されているが動かない |
| `C90` | 210 | 既定。何も起きないが、無いと何を失うかを 8 つの言葉では言えなかった |
| （空） | 41 | 作業が残らない項目 |

**使った段階は `C` だけである。** A・B・D・E は 1 件も使っていない。段階の文字そのものに意味を持たせず、「同じ段の中でどちらが先か」だけが読めるように 10 刻みの数値を付けてある。
段階を決め直すときに全件を振り直さずに済むよう、番号のあいだは空けてある。

### 使用頻度は弱い代理である

台帳の備考にある使用頻度は、**里々／YAYA のコミュニティ wiki の作例に、その綴りが現れる文書の数**である。
順序を決める 4 つの根拠のうち 3 番目（影響する既存資産の広さ）の目安として置いた。

> 裁定が言う「標準テンプレート辞書が使うタグ」の**弱い代理**にすぎない。この環境の ukadoc の索引に標準テンプレート辞書そのものは入っておらず、あるのは wiki の作例だけだからである。作例は「詰まりやすいところ」を書き残したものなので、日常的に使われる綴りほど記事にならず数が小さく出る。数の大小を「よく使われている／使われていない」と読んではならない。

**0 は「使われていない」ではなく「この検索に当たらない」と読む。** いちばん分かりやすい例が面を切り替える短縮形で、数字を直に続ける綴りで引くと 0 になるが、これは作例が角括弧付きで書くためであって、古い辞書で使われていないという意味ではない。

代理の歪みは 3 種ある。

| 歪み | 中身 | 当たる綴り |
| --- | --- | ---: |
| 大文字と小文字を分けない | 大文字の綴りと小文字の綴りが同じ数になる | 4 |
| 族は前置きで数えた | 同じ前置きで始まる別の命令まで巻き込むので過大に出る | 10 |
| `*` と `?` を文字のまま引いた | 実際の書かれ方に当たらないので過小に出る | 5 |

**この段では、頻度が判断を動かした項目は 0 件である。** 順序の根拠は「壊れ方 → 伺からしさ → 影響する既存資産の広さ → 依存する基盤の共有度」の並びで固定されており、優先度の決定表は先の 2 つだけで当たり切ったためである。
標準テンプレート辞書の場所が示されたら、その辞書だけを走査した出現に置き換え、この数字は補助へ下げる。

### 検査の結果

検査は 15 通り置いた。**15 通りとも緑で、赤は 0 件である。**
**検査した件数も一緒に載せる。** 0 件で緑になったのか、検査が働いて緑になったのかを読者が区別できるようにするためである。
件数の欄が「—」の行は 1 つも無い。
V12 だけは母数がこの文書自身なので、組み上げたあとに数え直した値を入れている。byte 数を母数に使わないのは、同じ中身でも改行を 2 文字で書くか 1 文字で書くかで数が変わり、1 つに定まらないからである。数え方が 1 つに定まる「表の数・表の行数・全体の行数」で母数を示した。

| 番号 | 検査 | 検査した件数 | 合否 |
| --- | --- | --- | --- |
| V1 | 項目の塊がちょうど 342 個 | 342 個（4 台帳を合わせた常設の検査では 1,749 件） | 緑 |
| V2 | 台帳の項目の集合が骨組みの手順で得た集合と一致し、4 つの節への割り振りが合う | 342 件（台帳にしか無い項目 0 件・骨組みにしか無い項目 0 件・担当ページ以外の項目 0 件・節の割り振り 198／28／115／1） | 緑 |
| V3 | 項目が文字順に並び、重複が無い | 隣り合う組 341 組・重複 0 件（常設の検査では 1,749 件） | 緑 |
| V4 | 未分類が 0 件 | 342 件（欄で数えて 0 件・文字列で探しても 0 件） | 緑 |
| V5 | 必須の 7 欄がそろい、状態が空文字でなく、凍結された語彙のいずれかである | 欄の存在 2,394 件・空文字でないこと 342 件・語彙 342 件 | 緑 |
| V6 | 別名の行に写像先があり、その先が台帳に実在し、その状態が別名でない | 342 件（写像先の記入 20 件＝状態が別名の 20 件。相手の実在は常設の検査が 4 台帳で 406 件・連鎖は 23 件） | 緑 |
| V7 | 関連の種別が 6 種のいずれかで、相手がスナップショットに実在する | 関連 182 件・任意のイベント名を取る族 13 件（相手の実在は常設の検査が 406 件。設定キーへ向かう種別は 0 本） | 緑 |
| V8 | テーマ名が 8 つのいずれかで、テーマが付いた行に「失うもの」の 1 文がある | テーマ名 延べ 422 件（4 台帳）・本台帳でテーマが付いた行 71 件 | 緑 |
| V9 | 優先度の形と、空にしてよい条件 | 形 342 件・空でよい項目 41 件・空にしてはいけない別名 3 件・引数が台詞へ漏れる 1 件・上限 342 件 | 緑 |
| V10 | 登場版が本文から取れる版のいずれかで、複数取れる項目は最小が採られている | 版が 2 つ以上取れる項目 4 件・SSP の版でない番号 1 件（記入のある行は常設の検査が 4 台帳で 262 件） | 緑 |
| V11 | ソースの正典 URL が実装済みの集合とちょうど対応する | 収穫した URL の行 47 件・同じ項目が 2 行ないこと 23 件・実装済みでない側に置かれていないこと 23 件（走査した .rs は 1,185 本。常設の検査では URL 51 件・実装済み 46 件） | 緑 |
| V12 | この文書の表が、作り直した結果と 1 バイトも違わない | 48 表・表の行 1,064 行・この文書の全 2,253 行（作り直した結果と改行をそろえて突き合わせ、差 0 バイト） | 緑 |
| V13 | URL を足したファイルの行数と、組み立ての警告が 0 件であること | 5 本（157〜522 行。1,000 行以上は 0 本）・使われない説明文の警告 0 件 | 緑 |
| V14 | 変更したファイルが境界の内側だけであること | 分岐点からの変更対象 16 件（境界の外 0 件）・触れてはいけない文書 25 件（すべて実在を先に確かめ、差分 0） | 緑 |
| V15 | ソースへ URL を置く前後でテストの結果が同じであること | 置く前の赤 16 本／置いた後の赤 2 本（16 本の部分集合・新しい赤 0 本）。その後の裁定で上流の試験の宣言を実測へ合わせ、ワークスペース全体で 7,092 件成功・0 件失敗 | 緑 |

**V14 の「変更対象 16 件」の内訳**——この調査の成果物 3 本（担当の台帳・この文書・担当ドメインの報告）、ソースへ正典の URL を書き足した 5 本、2026-09-06 の裁定で書き換えを許した上流の道具の試験 2 本、この調査自身の仕様書 6 本。**境界の外にあるものは 1 件も無い。**

**触れてはいけない文書は、差分を採る前に 1 つずつ実在を確かめた。** 実在しないパスに差分を求めると必ず空が返り、「触れていない」と「そもそも見ていない」を区別できなくなるためである。25 件すべてが実在し、25 件すべてが差分 0 だった。

⚠ **仕様書が「編集しない」と名指しする文書のうち 1 つは、この木に実在しない**（`doc/ukadoc-coverage/linkage.md`）。この名前のファイルは作業ツリーにも git の管理下にも無い。**実在しないパスに差分を求めれば必ず空が返るので、差分 0 の 25 件には数えていない。** 関連の種別の定義は `doc/ukadoc-coverage/README.md` の側にあり、そちらは差分 0 である。

この文書を最後に作り直したあとに、ワークスペース全体のテストと常設の検査をもう一度回した。テストは **7,092 件成功・0 件失敗**（打ち切らない指定を付けて実行）、常設の検査は **所見 0 件**である。

### 次に読む人への申し送り

この調査を回すあいだに分かった、**この文書に書き残さないと失われる**ことを 6 つ挙げる。
優先度の段階 A〜E が仮の位置づけであること・使用頻度が弱い代理であることの 2 つは、この節の少し上に別立てで書いてあるのでここでは繰り返さない。

#### ⑴ 完成条件の 1 つが、文字どおりには最初から満たせない

仕様書の完成条件のうち 1 つは、ソースへ正典の URL を書き足す前と後で「テストの結果が**同じく緑**であること」と書いてある。
**書き足す前の時点で、テストは既に 16 件失敗していた。** この調査が台帳を埋めたこと自体が、上流の道具の試験が置いていた前提を崩したためである。
文字どおりに読むと、この条件は最初から満たしようがない。

そこで「**書き足した後の失敗が、書き足す前の失敗の一部であること（新しく赤になったものが 0 件であること）**」と読み替えて確かめた。この条件のもとの狙いは「URL のコメントを足しても振る舞いが変わらないこと」の裏取りなので、読み替えても狙いは損なわれない。
**文言をどう直すかは裁定事項である。** この調査は文言を書き換えていない。

#### ⑵ 「156 件」は、数え方を書かないと誰も再現できない

仕様書は「ソースに『ukadoc』の語だけがあって URL を伴わない箇所が **156 件**ある」と書く。
**この字面のとおりに数えても 156 にはならない。** 156 になるのは、⒜ URL を伴わない ⒝ コメントの行である ⒞ 調査の道具自身のソースを除く——の 3 つを**すべて**課したときだけである。

| 数え方 | 件数 |
| --- | ---: |
| ソース全体（`crates` 配下の `.rs`。`target` は除く）で「ukadoc」の語を含む行 | 1,235 |
| ⒜ だけ（＝仕様書の字面どおり） | 1,105〜1,106 |
| ⒝ だけ | 322 |
| ⒞ だけ | 209 |
| ⒜ と ⒝ | 271 |
| ⒜ と ⒞ | 162 |
| ⒝ と ⒞ | 203 |
| **⒜ と ⒝ と ⒞** | **156** |

⒜ だけの数に幅があるのは、URL の目印を何と決めるかで 1 件動くためである（`https://` を目印にすると 1,106、`http` で始まる綴りまで数えると 1,105 になる）。
**8 通りのうち 156 になるのは 1 通りだけで、どの条件を 1 つ落としても 156 にならない。** この 3 条件は数え方そのものであって、後から付けた言い訳ではない。

#### ⑶ 上流の道具の試験が使う「的」は、台帳が育つと痩せる

上流の道具には、カタログからわざと 1 項目を抜いて「所見がちょうど 1 件出る」ことを確かめる試験が 2 本ある。
その的にしていた項目へ、この調査の台帳が関連を張った。所見が 2 件になって試験が赤くなったので、**的を「4 つの台帳のどこからも指されていない項目」へ機械で選び直した**（起動時のイベントから初回起動時のイベントへ移った）。

⚠ **選び直せる候補は、いま 4 件しか残っていない。** 候補の条件は 9 つあり、台帳が育つほど候補は痩せる。とくに**並走している他ドメインの調査が関連を書き足すと、いちばん早く痩せる**。
**候補が 0 件になったら、試験の側だけでは直せない。** 「ちょうど 1 件」という期待そのものの立て方を含めて裁定が要る。

#### ⑷ テストは打ち切らせない指定を付けて回す

ワークスペース全体のテストは、**最初の失敗で打ち切らない指定を必ず付けて**回すこと。
付けないと最初の失敗でそこから先が走らず、**走らなかった群が前後の比較の両側から同じように抜けて、数のつじつまが合ってしまう。** この調査は実際に 1 度これを踏み、比較のやり直しになった。

#### ⑸ 検査が主張する「0 件」のうち 1 つは、はじめから成り立つ

上流の道具は 15 種類の所見それぞれについて「実データでは 0 件」と主張する試験を持つ。
そのうち「**台帳の前置きに書いたドメイン名が食い違う**」の 1 種は、検査の段では作りようがない——前置きの食い違いは台帳を読み込む段で落ちるので、検査へ渡る前に消えているからである。
**この 1 種についての「0 件」は、常に成り立つ。** 嘘ではないが、検査が働いた証拠にはならない。台帳 4 本が読めた事実そのものが、前置きが揃っていることの証拠である。

#### ⑹ 並走している他の spec の着地が、一度確定した判定を黙って古びさせる

この台帳は 2026-09-05 に `\_l[x,y]` を「一部だけが効かない」と判定した。根拠は、対応表 `doc/COMPAT_ARCHITECTURE.md` の §8 に「正典と違う動きが既に登記されている」行があったことである。

**その翌日 2026-09-06 に取り込んだ上流の変更が、その根拠を取り下げていた。** 並走していた `areka-P0-cursor-tag-canon` が着地し、根拠にした行の表題自身が「既知の食い違いは取り下げた」と書き換わっていた。根拠が消えた以上、判定の条件はもう成り立たない。
最後の見直しで気づいて **「実装済み」へ直した**（一部だけが効かない 4 件 → 3 件・実装済み 22 件 → 23 件、ソースへ置く正典の URL も 1 行増えた）。

**取り込みの後に取り直すべきなのは、参照の綴りではなく、参照から導いた結論のほうである。** 綴りが古びていないか（指す先がずれていないか）は誰でも思い付くが、**綴りは生きたまま中身だけが反対の意味に変わる**ことがある。この調査も、取り込みの直後に「§8 の指す先を取り直す」ことは予定に入れていたが、**「§8 から導いた状態の判定を取り直す」ことは予定に入れていなかった。**

#### 備考を 1 周見て回った結果

機械の検査に無い項目なので、**342 件の備考を最後に 1 周、目で見て回った。**

**ソースの行番号を書いた備考は 0 件である。** 行番号は書いた翌日には黙って古びるので、備考は関数名・分岐の腕・定数名で場所を示す決まりにしてある。目で見て回るのと並行して、行番号らしい綴りを 6 通りの網で機械にも拾わせた（拾った候補も 0 件）。網そのものは、わざと行番号を書いた 4 通りの文で赤になり、行番号を含まない文では緑のままになることを確かめてある。

見て回るあいだに、**直すには台帳を書き換えることになる**のでこの段では手を付けなかったことが 2 つある。

- **札の呼び名が 1 か所そろっていない。** 「転記元」は「正典と違う動きが既に登記されている」ときの札と決めてあるが、「語彙だけが登記されている」3 件（`\f[align]`・`\f[valign]`・`\f[underline]`）にもこの札が使われている。同じ根拠を「語彙の登記」と書いた行は別に 9 件ある。読み違いを生む形ではないが、呼び名としては揃っていない。
- **欄の並びが上流の見本と 1 か所だけ逆である。** 別名の写像先の欄が、上流の見本では登場版の前に来るのに対し、この台帳では後ろに来ている。**20 件すべてで並びが揃っており、常設の検査も緑**なので、読む側にも道具にも害が無いと判断してそのままにした。

