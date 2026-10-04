# 実機の確かめの記録（areka-P0-shell-balloon・タスク 12.5）

設計の「実機の確かめ」に挙げた 10 項目のうち、ログで判定できるものを本物の areka と emo2 で走らせて確かめた。**ログで判定した 5 項目はすべて成立**。画面を見たり手で触ったりしないと判定できない残りは「目視待ち（開発者）」とし、4 章に再現の手順を書いた。未成立の項目は無い。

## 1. 走らせ方

| 項目 | 値 |
|---|---|
| 日時 | 2026-10-04 02:34〜02:43 JST（ログの時刻は UTC の 17:34〜17:43） |
| 版 | HEAD `7c60fefb`（未コミットの変更 0 件）。`cargo build -p areka --bin areka`（debug）と `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc` |
| 根 | ワークツリーの `target\sbx\A` と `target\sbx\B`。どちらも `nar-sample-path emo2` が展開した `target\nar-samples\manual\emo2` の写しに、`areka.exe` と 32bit の補助プロセス（PE の機種 `014C`＝32bit を確かめた）を置いたもの |
| 環境変数 | `NO_COLOR=1`・`RUST_LOG=info,areka=debug,areka_emo_text=debug,areka_emo_present=debug,areka_kanade=debug`（行き先の切り替え・箱の登録と片付け・面の装着・表示の判断が出る水準）・`AREKA_APP_SMOKE_EXIT_MS`（有界の自動終了・A は 80 秒・B は 40 秒）・`AREKA_BALLOON_TIMEOUT_MS=5000`（時間切れを見やすくするため 5 秒）・`AREKA_ROOT`／`AREKA_PROFILE_DIR`＝根・`TMP`／`TEMP`＝根の `tmp` |
| 起動 | `<根>\areka.exe "<根>\ghost\emo2"`（ゴーストは絶対パスで渡す） |
| 道具 | 本フォルダの `real-machine-run.ps1`（`-Prepare` で根を作り、`-Run A｜B` で起こす）。製品のコードとリポジトリの検体（`.nar`）には触れていない。1〜3 章の記録の後に、1 回の起動で 1 項目だけを確かめる `-Item` を足した（4 章・項目は手順に分け、相方のバルーンの「次へ」で人が進める・そのため今の `-Prepare` の根の master には、確かめ用の surface1110〜1112 が足されている） |

走行の一覧（`target\sbx\runs.txt` より）:

| 回 | プロセス番号 | 終わり方 | 記録 |
|---|---|---|---|
| A1 | 532 | 80 秒の自動終了（`app_exit origin=Smoke`・`ghost shutdown sequence completed`）。窓の撮影の失敗で見張りの側の手順が先に止まったため終了コードは取れていない | `target\sbx\A\run-A1.log` |
| A2 | 11752 | 自動終了・終了コード 0。A1 で替えたシェル `second` の記憶がゴースト側に残り、`second` で起きた（5 章）。判定には使わない | `run-A2.log` |
| A3 | 33096 | 自動終了・終了コード 0。**判定の本体** | `target\sbx\A\run.log` |
| B | 35160 | 自動終了・終了コード 0 | `target\sbx\B\run.log` |

`ERROR` は A の各回が `\s[9999]` の 2 行だけ（3.4）、B は 0 行。自分で起こしたプロセス以外は止めていない。走行の後に areka と補助プロセスが残っていないことを確かめた。

### 書き足した箱（ukadoc の語で）

検体の `shell\master\surfaces.txt` の末尾に足した（`charset,UTF-8`・CRLF のまま）。

| 名前 | balloon.*ブレス | 置き場所（surface.append*ブレスの element定義） |
|---|---|---|
| tate1 | `size,48,320`・`vertical,1`・青の文字 | surface0 の element1（370,40）、surface1100 の element1（20,80） |
| tate2 | `size,48,320`・`vertical,1`・赤の文字 | surface0 の element2（310,40） |
| fuda | `size,240,64`・`font.follow,balloon`・緑の文字 | surface1100 の element2（70,460） |
| hami（根 B だけ） | `size,320,120` | surface0 の element3（250,620）＝右と下へはみ出す（surface0 の絵は 434×687） |

surface1101 は箱を持たない。根 A には `shell\master` を写した `shell\second` を作り（`add_shell_copy` と同じ手順: フォルダを写し、`descript.txt` の `name` の行だけを `name,second` に替える）、surface0 の tate1 を（30,40）、tate2 を（90,40）に置いた。

### 台本

MCP の待受には道具がまだ 1 つも無く（`ToolRegistry::default()`）、外から台本を流す口が無い。そこで写したゴーストの台本 `ghost\master\scripts\pasta\shiori\event\second_change.lua` だけを差し替えた。起動の台詞のあと、話していない秒が 8 回たまるごとに次の段を出す。

- 起動（根 A）: `\0\s[0]\b[tate1]一の箱です。…\b[tate2]二の箱です。…\b[nai]…\s[1100]付いて回る。…\b[fuda]\f[color,255,0,0]札は赤。…\b[tate1]一は赤くない。…\s[9999]無い番号でも箱。…\s[1101]普通のバルーンへ。…\s[0]箱へ戻る。…\1\s[10]相方です。…\e`（「…」は `\w9` を 4 つ）
- 段 1: `\0\s[0]\b[tate2]シェルを替える。…\![change,shell,second]\e` → `OnShellChanged` の返事は `\s` を書かない `\0新しいシェルの箱。…\e`
- 段 2: 箱の中の選択肢 `\0\s[0]\b[tate1]選んで\n\q[はい,OnSbYes]\n\q[いいえ,OnSbNo]\e`
- 段 3: ダブルクリックの中断を試すための長い台詞
- 起動（根 B）: `\0\s[0]\b[hami]はみ出す箱です。…（長い文字）…\e`

### 画面の撮影

起こした areka の窓の四角だけを撮る手順を用意したが、この実行環境からは画面を撮れなかった（`CopyFromScreen` が「ハンドルが無効です」）。窓の四角（位置と大きさ）の記録だけは取れた（`<根>\windows.txt`・1 秒ごと）。見た目の判定は開発者の目視に回した。

## 2. 判定の一覧

| # | 確かめる項目 | 結果 | 根拠 |
|---|---|---|---|
| 1 | 台詞が絵の中に出て `\b[名前]` で書き分けられ、サーフェスを替えると文字が付いて回る | **成立**（ログ）／見た目は**目視待ち** | 3.1 |
| 2 | 窓のドラッグとモニタをまたぐ拡大率の変化で文字が絵とずれない | **目視待ち（開発者）** | 4 章 |
| 3 | 箱を出し入れするときにシェルの絵がちらつかない | **目視待ち（開発者）** | 4 章 |
| 4 | 箱の置き場所が違うサーフェスへ替えた瞬間のずれ | **ずれは 1 フレームで収まる**（ログ）。ただし向きは設計の見込みと逆の場合があった。目に付くかは**目視待ち** | 3.2 |
| 5 | はみ出す定義で、はみ出した部分が窓の端で切れ、窓の大きさが変わらず、警告が 1 行出る | 警告 1 行と窓の大きさは**成立**（ログ）／窓の端で切れる見た目は**目視待ち** | 3.3 |
| 6 | シェルに無い番号の `\s` で箱の文字が出続け、普通のバルーンが出ない | **成立** | 3.4 |
| 7 | 箱のあるサーフェスのままシェルを切り替えると、`\s` を書かない台詞が新しいシェルの箱へ出る | **成立** | 3.5 |
| 8 | 透明な画素の位置は箱の四角の中でも下のアプリへクリックが抜ける | **目視待ち（開発者）** | 4 章 |
| 9 | 箱の選択肢を選べる・話している最中の箱の左ダブルクリックで止まる・話していないときは立ち絵のダブルクリックになる | **目視待ち（開発者）**（選択肢が時間切れを止めることだけはログで成立） | 3.6・4 章 |
| 10 | 普通のバルーンと同じ待ち時間で箱の文字が消え、ポインタを置いているあいだは消えない | 同じ待ち時間で消えるのは**成立**（ログ）／ポインタで止まるのは**目視待ち** | 3.6・4 章 |

## 3. 項目ごとの根拠（A3 と B のログ・逐語の抜き書き）

時刻は UTC。行頭の `actor{actor=emo-text}:` は省いた。

### 3.1 書き分けと、サーフェスの切替で付いて回る（項目 1）

読み込み（ブレス 3 つ・箱を持つサーフェス 2 つ）と、起動の台詞の流れ:

```
17:39:26.240917  INFO areka_emo_present::shell_target: shell: 箱を読んだ（書かれた balloon.*ブレスの数・箱の置き場所を持つサーフェスの数） braces=3 surfaces=2
17:39:26.764047 DEBUG areka_emo_text::state::route: \s による行き先の決定 actor=0 surface=Some(0) dest=Box(BoxName("tate1"))
17:39:26.787509 DEBUG areka_emo_text::actor::boxes: 箱の置き場所を登録した（面は次の提示で作る） actor=0 name="tate1" surface=0 element=1 x=370 y=40 k=2.0
17:39:26.791143  INFO areka_emo_text::actor::present: テキスト供給面を予約スロットへ装着した（…） actor=0 place=Box(BoxName("tate1")) … physical_size=(96, 640)
17:39:28.868510 DEBUG areka_emo_text::state::route: \b[名前] で行き先を箱へ切り替え actor=0 name="tate2"
17:39:28.880743 DEBUG areka_emo_text::actor::boxes: 箱の置き場所を登録した（面は次の提示で作る） actor=0 name="tate2" surface=0 element=2 x=310 y=40 k=2.0
17:39:30.964610  WARN areka_emo_text::state::route: \b[名前] の箱が今のサーフェスに無い——行き先を切り替えない name="nai" surface=0 actor=0
17:39:32.824773 DEBUG areka_emo_text::state::route: \s による行き先の決定 actor=0 surface=Some(1100) dest=Box(BoxName("tate1"))
17:39:32.830640 DEBUG areka_emo_text::actor::boxes: 箱の置き場所の登録を外した（文字は保つ） actor=0 place=Box(BoxName("tate2"))
17:39:32.831283 DEBUG areka_emo_text::actor::boxes: 箱の置き場所の登録を外した（文字は保つ） actor=0 place=Box(BoxName("tate1"))
17:39:32.832108 DEBUG areka_emo_text::actor::boxes: 箱の置き場所を登録した（面は次の提示で作る） actor=0 name="tate1" surface=1100 element=1 x=20 y=80 k=2.0
17:39:34.866954 DEBUG areka_emo_text::state::decoration: 箱に閉じる箱へ——\f の指定を写さない actor=0 dest=Box(BoxName("fuda"))
17:39:34.879818 DEBUG areka_emo_text::actor::boxes: 箱の置き場所を登録した（面は次の提示で作る） actor=0 name="fuda" surface=1100 element=2 x=70 y=460 k=2.0
17:39:43.464155 DEBUG areka_emo_text::state::route: \s による行き先の決定 actor=0 surface=Some(0) dest=Box(BoxName("tate1"))
17:39:43.472819 DEBUG areka_emo_text::actor::boxes: 箱の置き場所を登録した（面は次の提示で作る） actor=0 name="tate1" surface=0 element=1 x=370 y=40 k=2.0
17:39:43.473826 DEBUG areka_emo_text::actor::boxes: 箱の置き場所を登録した（面は次の提示で作る） actor=0 name="tate2" surface=0 element=2 x=310 y=40 k=2.0
```

- `\b[tate2]` で 2 つ目の箱へ書き分け、無い名前 `nai` は警告 1 行（名前・サーフェス番号・スコープ）で行き先を変えなかった。
- `\s[1100]` で tate1 は surface1100 の置き場所（20,80）へ登録し直され、行き先は tate1 のまま＝文字が付いて回った。surface1100 に無い tate2 は登録が外れ、文字は保たれた。
- `\s[0]` に戻ると tate1・tate2 とも元の置き場所で登録し直された（`\s[1101]` のあいだ外れていた箱の文字が戻る）。
- `font.follow,balloon` の fuda へ入るときは装飾の指定を写さなかった（要件 3.14）。fuda の赤が tate1 に移っていないかの見た目は目視待ち。
- 面の大きさは拡大率 2.0 で tate1 が 96×640（48×320 の 2 倍）、fuda が 480×128。

### 3.2 置き場所の違うサーフェスへ替えた瞬間（項目 4）

起動の台詞の 4 回の `\s`（surface0 → 1100 → 9999 → 1101 → 0）と、A1 の同じ 4 回を並べると、絵の差し替え（`apply(ShowSurface)`）と文字の層の `\s` の受け取りの順は回によって前後した。

A3 の surface0 → 1100（絵が先に届いた回）:

```
17:39:32.819120  INFO areka_emo_present::presenter::show: apply(ShowSurface): 表示・マスクを更新 target_id=TargetId(0) surface_id=1100 …
17:39:32.820876 DEBUG areka_emo_text::actor::attach: 文字層の再追従: …（このフレームの文字の拡大率の相）
17:39:32.824773 DEBUG areka_emo_text::state::route: \s による行き先の決定 actor=0 surface=Some(1100) dest=Box(BoxName("tate1"))
17:39:32.828460 DEBUG areka_emo_text::actor::attach: 文字層の再追従: …（次のフレームの文字の拡大率の相）
17:39:32.830640 DEBUG areka_emo_text::actor::boxes: 箱の置き場所の登録を外した（文字は保つ） actor=0 place=Box(BoxName("tate2"))
17:39:32.832108 DEBUG areka_emo_text::actor::boxes: 箱の置き場所を登録した（面は次の提示で作る） actor=0 name="tate1" surface=1100 element=1 x=20 y=80 k=2.0
```

A1 の surface0 → 1100（文字の層が先に受け取った回）:

```
17:34:26.803971 DEBUG areka_emo_text::state::route: \s による行き先の決定 actor=0 surface=Some(1100) dest=Box(BoxName("tate1"))
17:34:26.816104  INFO areka_emo_present::presenter::show: apply(ShowSurface): 表示・マスクを更新 target_id=TargetId(0) surface_id=1100 …
17:34:26.822305 DEBUG areka_emo_text::actor::boxes: 箱の置き場所の登録を外した（文字は保つ） actor=0 place=Box(BoxName("tate2"))
17:34:26.823668 DEBUG areka_emo_text::actor::boxes: 箱の置き場所を登録した（面は次の提示で作る） actor=0 name="tate1" surface=1100 element=1 x=20 y=80 k=2.0
```

読み方:

- 1 フレームの中の並びは「表示の指令の適用（絵の差し替え）→ 文字の拡大率の相（ここで箱の置き場所を合わせる `sync_boxes`）→ 文字の提示」。文字の層の `\s` の受け取りはフレームの外（UI スレッドの受信の口）で起き、置き場所に反映されるのは**次のフレームの `sync_boxes`**である（`crates/areka/src/emo2_boot/frame.rs` の `emo2_frame_system` の並び、`crates/areka-emo-text/src/actor.rs` の `spawn_emo_text`）。
- A1 の型（文字の層が先）: 受け取ったフレームはまだ前の絵・前の置き場所のまま、次のフレームで絵の差し替えと置き場所の付け替えが同じフレームに入る。**ずれるフレームは 0**。A1・A3 の 8 回の切替（`\s[9999]` を除く）のうち 7 回がこの型で、どの回も受け取りの次のフレームに絵の差し替えと置き場所の付け替えがそろっていた（あいだに文字の拡大率の相の行が挟まっていない）。
- A3 の surface0 → 1100 の型（絵が先）: 絵が先に差し替わったフレーム（32.819〜）は、**新しい絵の上に前の置き場所（surface0 の tate1・tate2 の位置）の文字が 1 フレーム残り**、次のフレーム（32.830〜）で付け替わった。**ずれは 1 フレームで収まった**。
- 設計の見込み（design.md「描画は毎フレーム、…」の段落: 「その 1 フレームは、前の絵の上に新しい置き場所で箱の文字が出る」）は、seriko が遅れる向きだけを想定していた。実際に出たのは**逆の向き（新しい絵の上に前の置き場所の文字）**で、原因は「`\s` の受け取りが置き場所に反映されるのが次のフレームの `sync_boxes`」であること。設計の見込みの向きの記述はこの観察に合わせて直す必要がある（本タスクでは design.md を書き換えていない）。
- 目に付くかどうかは目視待ち。目に付く場合の起票先の案は **`areka-P0-shell-balloon-frame-align`**（「1 フレーム遅らせる」手当てではなく、`\s` の受け取りと置き場所の付け替えを同じフレームに収める形で解く）。

### 3.3 はみ出す定義（項目 5・根 B）

```
17:42:26.352637  INFO areka_emo_present::shell_target: shell: 箱を読んだ（…） braces=4 surfaces=2
17:42:27.241094 DEBUG areka_emo_text::state::route: \b[名前] で行き先を箱へ切り替え actor=0 name="hami"
17:42:27.248740  WARN areka_emo_text::actor::boxes: 箱がサーフェスの画像からはみ出している——採ったうえで、はみ出した部分は窓の端で切れる surface=0 name="hami" rect=(250, 620, 320, 120) surface_size=(434, 687)
17:42:27.248842 DEBUG areka_emo_text::actor::boxes: 箱の置き場所を登録した（面は次の提示で作る） actor=0 name="hami" surface=0 element=3 x=250 y=620 k=2.0
17:42:27.253190  INFO areka_emo_text::actor::present: テキスト供給面を予約スロットへ装着した（…） actor=0 place=Box(BoxName("hami")) … physical_size=(640, 240)
```

- 警告は**ちょうど 1 行**（B の記録全体で `箱がサーフェスの画像からはみ出している` の行は 1 行・40 秒の走行のあいだ台詞の文字は出続けた）で、サーフェス番号・名前・箱の四角・サーフェスの大きさを持つ。
- 窓の大きさ: 本体の窓は走行の全 37 回の記録で `rect=(2012,330,2880,1704) size=868x1374`（＝surface0 の 434×687 の 2 倍）のまま変わらなかった（`target\sbx\B\windows.txt`）。箱の面（640×240・窓の中の位置 (500,1240)）は窓の右端と下端を越えるが、窓は広がっていない。
- はみ出した部分が窓の端で切れて見えるかは目視待ち。いまのところ設計の記述（「採用する（窓の端で切れる）」）と食い違う観察は無い。

### 3.4 シェルに無い番号の `\s`（項目 6）

```
17:39:39.022079 DEBUG areka_emo_text::state::route: 解決できない \s——行き先もサーフェス番号も変えない actor=0
17:39:39.027816 ERROR areka_emo_present::presenter::show: apply(ShowSurface): 合成失敗 → 表示は適用前のまま（reply Err） target_id=TargetId(0) surface_id=9999 error=surface 9999 not found
（続く「無い番号でも箱。」は tate1 へ。次の普通のバルーンの表示は \s[1101] の後だけ）
17:39:41.278708  INFO areka::emo2_boot::balloon_visibility::phase: [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="content" visible=true
```

- `\s[9999]` の後も行き先は tate1 のままで、文字は箱へ入り続けた。スコープ 0 の普通のバルーンが初めて見えたのは箱の無い surface1101 へ替えた 41.278 で、9999 のあいだ（39.022〜41.216）に可視の遷移は 0 件。起動から 41.278 までスコープ 0 の普通のバルーンは一度も出ていない。
- `\s[1101]` から `\s[0]` へ戻ると、普通のバルーンは文字が 0 になったことで隠れた（`17:39:43.470585 … scope=0 trigger="clear" visible=false`）。
- `ERROR` の 2 行は、シェルに無い面を表示しようとした合成の失敗で、本 spec の前からある振る舞い（`areka-P0-shell-balloon-switch` の記録にも同じ種類がある）。表示は前の絵のまま続いた。

### 3.5 箱のあるサーフェスのままシェルを切り替える（項目 7）

```
17:39:54.879547 DEBUG areka_emo_text::actor::boxes: 箱の置き場所を登録した（面は次の提示で作る） actor=0 name="tate2" surface=0 element=2 x=310 y=40 k=2.0
17:39:57.351923  INFO areka_emo_present::shell_target: shell: 箱を読んだ（…） braces=3 surfaces=1
17:39:57.399530  INFO areka_emo_present::presenter::show: apply(ShowSurface): 表示・マスクを更新 target_id=TargetId(0) surface_id=0 …
17:39:57.417830 DEBUG areka_emo_text::actor::boxes: 箱の束を受け取った（前の箱の面を片付け、表を差し替える） dropped_surfaces=1 boxes=2 surfaces=1 …
17:39:57.418863 DEBUG areka_emo_text::state::route: 箱の表を差し替え（番号は保ち、行き先を既定へ引き直す） surfaces=1
17:39:57.423487  INFO areka::emo2_boot::frame::switch: シェル・バルーンの切替を終えた event="skin_switch_done" kind=Shell to=second epoch=1 marked=None swap_ms=44.814 …
17:39:57.424287  INFO actor{actor=kanade}: kanade: 応答にスクリプト——再生起動 event="steady_talk" talk_id=3 origin="OnShellChanged"
17:39:57.471905 DEBUG areka_emo_text::actor::boxes: 箱の置き場所を登録した（面は次の提示で作る） actor=0 name="tate1" surface=0 element=1 x=30 y=40 k=2.0
```

- 切替の前は master の tate2（310,40）、切替の後の `\s` を書かない台詞は **second の surface0 の tate1（30,40）**へ入った。前のシェルの箱の面は片付けられた（`dropped_surfaces=1`）。

### 3.6 時間切れと、選択肢が待ちを止めること（項目 10・9 の一部）

```
17:39:47.628120  INFO …balloon_visibility::phase: [balloon-visibility] タイムアウト計測を開始（起点＝会話の占有終端） display_end=20.799999999999983 deadline=25.799999999999983
17:39:52.628039 DEBUG areka_emo_text::actor::boxes: 箱を隠す印を立てた（次の台詞の頭まで） actor=0
17:39:52.628156 DEBUG areka_emo_text::actor::boxes: 箱を隠す印を立てた（次の台詞の頭まで） actor=1
17:39:52.628565  INFO …balloon_visibility::phase: [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="timeout" visible=false
17:39:52.628681  INFO …balloon_visibility::phase: [balloon-visibility] バルーンの可視状態が遷移した scope=1 trigger="timeout" visible=false
17:39:52.633636 DEBUG areka_emo_text::actor::boxes: 箱の置き場所の登録を外した（文字は保つ） actor=0 place=Box(BoxName("tate1"))
```

- 起動の台詞の後、計測の開始から**ちょうど 5.0 秒**で、箱だけに文字があるスコープ 0 と、普通のバルーンのスコープ 1 が**同じ時刻に**隠れた。箱の面も同時に外れた。
- 箱だけのとき（シェル second の台詞の後）も 5.0 秒で隠れた（`17:40:01.578181` 開始 → `17:40:06.578325 箱を隠す印を立てた`）。根 B も 5.0 秒（`17:42:35.229503` 開始 → `17:42:40.229735` 非表示）。
- 箱の中に選択肢を出したあいだは、時間切れが止まった（選択肢を数に入れた・要件 6.11）:

```
17:40:14.078005  INFO …balloon_visibility::phase: [balloon-visibility] 抑止が成立しているためタイムアウトによる非表示を見送った dragging=false hover=false choice=true deadline=5.15
```

## 4. 目視待ちの項目の手順（開発者）

1 回の起動で 1 つの項目だけを確かめる（`-Item`）。項目は番号つきの**手順**に分けてあり、1 手順が 1 本の台詞になる。どの手順も次の形で出る。

1. 相方（右の子）の普通のバルーンに、見出し「項目 n・手順 k/N: …」と、その手順で何が出て何を見るかの説明。
2. 紫の子の箱に、確かめる文字。
3. 相方のバルーンに選択肢「次へ」「もう一度」。**クリックするまで次の手順へ進まない**（どの手順も頭で `\![set,choicetimeout,0]` を出して選択肢の時間切れを切ってある。0 は無期限＝`areka-sakura` の `parse_choice_timeout` と `areka-kanade` の `choice_deadline` で「0・負は期限なし」）。「もう一度」はその手順を最初から出し直す。

選択肢は相方のバルーンの中にあるので、確かめる箱の場所とは重ならない。選択肢を待つあいだは、どのスコープに選択肢があってもバルーンの時間切れが止まる（`balloon_visibility_wait` の `observe_suppression` はスコープを問わず選択肢を数える）ので、**紫の子の箱の文字も消えずに残る**。最後の手順は相方のバルーンの「この項目はおしまい。」で終わり、選択肢を出さない（その後はバルーンの待ち時間で消える）。

選択肢を待つ手順で台詞が中断されたとき（箱を誤ってダブルクリックした等）は、話していない秒が 3 回たまるとその手順が出し直される。例外が 3 つある。

- 項目 7 の手順 2: 次へ進む合図は、紫の子の札の中の選択肢「はい」「いいえ」。
- 項目 9 の手順 2・項目 10 の手順 2: 「話していない」状態を確かめるため選択肢を出さない（選択肢を待つあいだは「話している最中」に数えられる・`fold_talking`）。次へ進む合図は**ダブルクリック**（項目 9 は青い文字、項目 10 は右の子）。
- 項目 8 の手順 2: 台詞を中断するのが確かめる操作なので、中断の 3 秒後に次の手順（手順 3）が出る。

ゴースト自身の台詞（ランダムトーク・なでなで・メニュー・選択肢の時間切れ・シェル切替の台詞）はその回のあいだ出ない。自動終了は 15 分（人がクリックで進めるため）。途中でやめるときは窓を閉じてよい。バルーンの待ち時間・ログの水準は項目ごとに決めてある（`-ExitMs`・`-RustLog` を明示すればそちらが勝つ）。ログは毎回 `<根>\run.log` に上書きされるので、項目ごとに走らせた直後に照らす。

**吹き出しの絵について**: 検体のシェルの `surface0.png` には、吹き出し（「アヒルやアヒル！…」）が**絵として描き込まれている**。areka のバルーンではないので、つかむと立ち絵ごと動く（要件 9.1・9.2・9.7 どおり）。項目ごとの確かめでは、吹き出しの描き込まれていない絵（`purple\0\base1.png`・`base2.png`。着せ替えの土台なので顔は描かれていない）で作った surface1110〜1112 に箱を置いた。どの手順も台詞の頭で `\0\s[1110]`（項目 7 と項目 1 の手順 4 は `\s[1111]`）を出すので、起動の直後の一瞬だけ surface0 が見える。その吹き出しは絵の一部。

準備（1 度だけ・根 A と B を作り直す。areka は起こさない。台本は `-Run` のたびに書き直すので、台本だけを替えたときは作り直さなくてよい）:

```
pwsh -NoProfile -File "C:\home\maz\git\areka\.claude\worktrees\areka-p0-status-execution-bf7596\.kiro\specs\areka-P0-shell-balloon\real-machine-run.ps1" -Prepare
```

項目ごとの箱の置き場所（surface.append*ブレスの element定義・サーフェスの画像の中の座標）: surface1110 は tate1（青・縦書き）が右肩 (330,40)・tate2（赤・縦書き）が左肩 (10,40)、surface1111 は tate1 が左上 (10,80)・fuda（緑・横書き・`font.follow,balloon`）が足元 (70,470)、surface1112 は箱なし。根 B だけ surface1110 に hami（はみ出す箱・(200,480)・320×120）を足してある。

見出しの「2 章の #」は 2 章の判定の一覧の番号（`-Item` の番号とは別）。ログは PowerShell の `Select-String` で照らす。行の欄は `event="box_press"`・`box="tate1"`・`verdict=Break` の形で出る（`box_hover_changed` の `from`／`to` は、箱の外なら欄そのものが出ない）。

**手順の印（どの手順のあいだの行かを分ける）**: 下の 1 行で、手順の切り替わりの時刻の並びが取れる。項目ごとの判定の行は、この印のあいだの時刻で読む。

```
Select-String -Path "C:\home\maz\git\areka\.claude\worktrees\areka-p0-status-execution-bf7596\target\sbx\A\run.log" -SimpleMatch -Pattern 'event="boot_talk"','event="choice_selected"','event="steady_talk"'
```

- `event="boot_talk"` … 手順 1 が出た。
- `event="choice_selected" scope=1 id=OnSbI<n>S<k>` … 相方のバルーンで選んだ（`label=次へ` なら手順 k へ進んだ、`label=もう一度` なら手順 k の出し直し）。続く `event="steady_talk" … origin="OnChoiceEvent"` がその手順の台詞の始まり。
- `event="steady_talk" … origin="OnSecondChange"` … 中断の後の出し直し（項目 8 では手順 3）。
- `event="steady_talk" … origin="OnMouseDoubleClick"` … ダブルクリックで次の手順が出た（項目 9・10 の手順 3）。
- 選択肢を待つ手順ごとに `[balloon-visibility] 抑止が成立しているためタイムアウトによる非表示を見送った … choice=true` が 1 行出る（箱の文字が残ったことの裏付け）。

「もう一度」を押した回数だけ、その手順の判定の行も増える。下の「1 行」は手順を 1 回出したときの数。

### 項目 1（2 章の #1）箱に文字が出る・`\b[名前]` の書き分け・サーフェスの切替で付いて回る

```
pwsh -NoProfile -File "C:\home\maz\git\areka\.claude\worktrees\areka-p0-status-execution-bf7596\.kiro\specs\areka-P0-shell-balloon\real-machine-run.ps1" -Run A -Item 1
```

| 手順 | 見出し | 出るもの | 見ること |
|---|---|---|---|
| 1/5 | 右肩に青・左肩に赤の縦書きが出る | 右肩に青い縦書き「一の箱（青）です。」、左肩に赤い縦書き「二の箱（赤）です。」 | 2 つの箱に色違いで書き分けられる |
| 2/5 | 無い名前の箱を指しても赤い列のまま | 左肩の赤い列に「二の箱（赤）です。」「無い名前の後も赤。」 | 名前の無い箱を指した後も赤い列に続く |
| 3/5 | 腕の形が替わると青い文字が左上へ移る | 右肩に青い「付いて回る。」→ 2 秒後に腕の形が替わり、青い文字が左上へ移って「腕が替わった。」が続く | 文字が絵と一緒に左上へ移る |
| 4/5 | 足元の緑の札だけが赤い文字になる | 足元の札に赤い「札は赤。」、左上の青い列に「一は青のまま。」 | 札の赤が青い列へ移らない |
| 5/5 | 普通の吹き出しへ移り、箱へ戻る | 右肩に「青の箱。」→ 紫の子の普通のバルーンに「普通の吹き出しへ。」→ 3 秒後に右肩の箱へ「箱へ戻る。」。相方に「この項目はおしまい。」 | 普通のバルーンと箱を行き来する |

- 合格の姿: 各手順の「見ること」のとおりに見える。
- ログで判定（手順の印とあわせて読む）:

```
Select-String -Path "C:\home\maz\git\areka\.claude\worktrees\areka-p0-status-execution-bf7596\target\sbx\A\run.log" -SimpleMatch -Pattern 'event="choice_selected"','\s による行き先の決定','\b[名前]','箱の置き場所を登録した','\f の指定を写さない','バルーンの可視状態が遷移した'
```

  手順 1 で tate1 が `surface=1110 element=1 x=330 y=40`・tate2 が `x=10 y=40` で登録、手順 2 で `name="nai"` の警告がちょうど 1 行、手順 3 で `\s[1111]` の後に tate1 が `surface=1111 element=1 x=10 y=80` で登録し直し、手順 4 で fuda へ入るときに `\f の指定を写さない` が 1 行（fuda は `surface=1111 element=2 x=70 y=470`）、手順 5 で `\s[1112]` の後に `scope=0 trigger="content" visible=true` が 1 行。

### 項目 2（2 章の #2）ドラッグと拡大率の変化でずれない

```
pwsh -NoProfile -File "C:\home\maz\git\areka\.claude\worktrees\areka-p0-status-execution-bf7596\.kiro\specs\areka-P0-shell-balloon\real-machine-run.ps1" -Run A -Item 2
```

| 手順 | 見出し | 出るもの | すること・見ること |
|---|---|---|---|
| 1/4 | 右肩に青・左肩に赤の文字が出る | 右肩に「青の文字」、左肩に「赤の文字」 | 2 つの文字を確かめる |
| 2/4 | ドラッグしても文字が絵とずれない | 同じ文字 | 紫の子の体（文字の無い所）をつかんでドラッグする。最中も離した後も文字が絵と一緒に動く |
| 3/4 | 拡大率の違うモニタへ移してもずれない | 同じ文字 | 拡大率の違うモニタがあれば、そちらへ持っていって戻す（無ければそのまま次へ）。文字が絵とずれない |
| 4/4 | まとめ | 相方に「この項目はおしまい。」 | — |

- 合格の姿: ドラッグの最中も、モニタをまたいだ後も、文字が絵とずれない。
- ログで判定（ずれそのものは目視）:

```
Select-String -Path "C:\home\maz\git\areka\.claude\worktrees\areka-p0-status-execution-bf7596\target\sbx\A\run.log" -SimpleMatch -Pattern 'event="choice_selected"','[diag.window_move]','箱の置き場所を登録した'
```

  手順 3（`id=OnSbI2S3` の後）でモニタをまたいだ後に、箱の置き場所が新しい拡大率（`k=`）で登録し直されている。

### 項目 3（2 章の #3）ちらつかない

```
pwsh -NoProfile -File "C:\home\maz\git\areka\.claude\worktrees\areka-p0-status-execution-bf7596\.kiro\specs\areka-P0-shell-balloon\real-machine-run.ps1" -Run A -Item 3
```

| 手順 | 見出し | 出るもの | 見ること |
|---|---|---|---|
| 1/3 | これから 8 回くり返す | 説明だけ | 次の手順で立ち絵を見続ける用意 |
| 2/3 | 立ち絵がちらつかないか見る | 箱に文字が出る・`\c` で消える・腕の形が替わる・普通のバルーンへ移る・箱へ戻る、を 8 回（約 70 秒）。終わると「次へ」「もう一度」 | 立ち絵が一瞬消える・白くなる・跳ねることが無い |
| 3/3 | まとめ | 相方に「この項目はおしまい。」 | — |

- 合格の姿: 手順 2 のあいだ、立ち絵が一瞬消える・白くなる・跳ねることが無い。
- ログで判定（ちらつきそのものは目視）:

```
Select-String -Path "C:\home\maz\git\areka\.claude\worktrees\areka-p0-status-execution-bf7596\target\sbx\A\run.log" -Pattern ' ERROR '
```

  0 行。

### 項目 4（2 章の #4）置き場所の替わる瞬間の 1 フレーム

```
pwsh -NoProfile -File "C:\home\maz\git\areka\.claude\worktrees\areka-p0-status-execution-bf7596\.kiro\specs\areka-P0-shell-balloon\real-machine-run.ps1" -Run A -Item 4
```

| 手順 | 見出し | 出るもの | 見ること |
|---|---|---|---|
| 1/3 | これから 16 回替わる | 説明だけ | — |
| 2/3 | 替わる瞬間に前の位置に文字が残るか見る | 青い「行き来する文字」が、3 秒ごとに腕の形が替わるたびに右肩（surface1110）と左上（surface1111）を行き来する（16 回・約 50 秒）。終わると「次へ」「もう一度」 | 替わる瞬間に、前の位置に文字が一瞬残って見えるか |
| 3/3 | まとめ | 相方に「この項目はおしまい。」 | — |

- 合格の姿: 前の位置に文字が一瞬残って見えても目に付かない（目に付くなら `areka-P0-shell-balloon-frame-align` として起票）。
- ログで判定:

```
Select-String -Path "C:\home\maz\git\areka\.claude\worktrees\areka-p0-status-execution-bf7596\target\sbx\A\run.log" -SimpleMatch -Pattern 'event="choice_selected"','apply(ShowSurface)','\s による行き先の決定','箱の置き場所を登録した'
```

  3.2 の読み方で、手順 2（`id=OnSbI4S2` から `id=OnSbI4S3` まで）の 16 回の切替のうち、絵の差し替え（`apply(ShowSurface)`）が文字の層の `\s` の受け取り（`\s による行き先の決定`）より先に来た回（前の置き場所の文字が 1 フレーム残る型）を数える。台詞の頭の `\s[1110]` は切替に数えない。

### 項目 5（2 章の #5）はみ出し（根 B）

```
pwsh -NoProfile -File "C:\home\maz\git\areka\.claude\worktrees\areka-p0-status-execution-bf7596\.kiro\specs\areka-P0-shell-balloon\real-machine-run.ps1" -Run B -Item 5 -Watch
```

| 手順 | 見出し | 出るもの | 見ること |
|---|---|---|---|
| 1/2 | 右下の箱が絵からはみ出す | 紫の子の右下に、絵（382×547）の右と下へはみ出す箱 hami の長い文字 | はみ出した部分が立ち絵の窓の端で切れ、窓が大きくならない |
| 2/2 | まとめ | 相方に「この項目はおしまい。」 | — |

- 合格の姿: 手順 1 のあいだ、はみ出した部分が立ち絵の窓の端で切れて見え、立ち絵の窓が大きくならない。
- ログで判定:

```
Select-String -Path "C:\home\maz\git\areka\.claude\worktrees\areka-p0-status-execution-bf7596\target\sbx\B\run.log" -SimpleMatch -Pattern '箱がサーフェスの画像からはみ出している'
(Get-Content "C:\home\maz\git\areka\.claude\worktrees\areka-p0-status-execution-bf7596\target\sbx\B\windows.txt") -replace '^.*hwnd=(\d+).*size=(\S+)$','$1 $2' | Sort-Object -Unique
```

  警告が手順 1 を出した回数だけ（1 回なら 1 行・`surface=1110 name="hami" rect=(200, 480, 320, 120) surface_size=(382, 547)`）。窓の記録は、立ち絵の窓の hwnd の大きさが 1 通りだけ（起動直後の surface0 の大きさの行が別にあってもよい）。

### 項目 6（2 章の #8）透明な画素のクリックが下へ抜ける

```
pwsh -NoProfile -File "C:\home\maz\git\areka\.claude\worktrees\areka-p0-status-execution-bf7596\.kiro\specs\areka-P0-shell-balloon\real-machine-run.ps1" -Run A -Item 6
```

| 手順 | 見出し | 出るもの | すること・見ること |
|---|---|---|---|
| 1/4 | 紫の子の後ろに窓を置く | 右肩に青い「文字」 | 紫の子の右肩の後ろにメモ帳などの窓を置く |
| 2/4 | 青い文字の下をクリックすると後ろの窓へ抜ける | 同じ | 青い文字の下（同じ縦の列の、何も描かれていない所。箱の四角の中）を 1 回クリックする。後ろの窓が選ばれる |
| 3/4 | 青い文字そのものは後ろへ抜けない | 同じ | 青い文字そのものを 1 回だけクリックする。後ろの窓が選ばれない |
| 4/4 | まとめ | 相方に「この項目はおしまい。」 | — |

- 合格の姿: 手順 2 で後ろの窓が選ばれ、手順 3 では選ばれない。手順 2 で前に出たメモ帳が相方のバルーンを隠したら、メモ帳をずらしてから「次へ」を押す。
- ログで判定:

```
Select-String -Path "C:\home\maz\git\areka\.claude\worktrees\areka-p0-status-execution-bf7596\target\sbx\A\run.log" -SimpleMatch -Pattern 'event="choice_selected"','event="box_press"'
```

  `box_press` は手順 3（`id=OnSbI6S3` から `id=OnSbI6S4` まで）の 1 行（`box="tate1" double_click=None selected_now=false verdict=ShellOp`）だけ。手順 2 のクリックは areka に届かないので行が増えない（相方のバルーンの「次へ」は箱ではないので `box_press` にならない）。

### 項目 7（2 章の #9 のうち選択肢）箱の選択肢の強調とクリック

```
pwsh -NoProfile -File "C:\home\maz\git\areka\.claude\worktrees\areka-p0-status-execution-bf7596\.kiro\specs\areka-P0-shell-balloon\real-machine-run.ps1" -Run A -Item 7
```

| 手順 | 見出し | 出るもの | すること・見ること |
|---|---|---|---|
| 1/3 | 足元の緑の札に選択肢が出る | 説明だけ | — |
| 2/3 | 札の選択肢に重ねてからクリックする | 足元の緑の札（fuda・横書き）に「選んで」と、選択肢「はい」「いいえ」。相方のバルーンに「次へ」は出ない | ポインタを「はい」「いいえ」に重ねて動かし、強調を見てからどちらかをクリックする（それが次へ進む合図） |
| 3/3 | 返事が同じ札に出る | 札に「はいを選んだ。」か「いいえを選んだ。」。相方に「この項目はおしまい。」 | 返事が同じ札に出る |

- 合格の姿: 重ねた選択肢が強調され、クリックすると返事が同じ札に出る。
- ログで判定:

```
Select-String -Path "C:\home\maz\git\areka\.claude\worktrees\areka-p0-status-execution-bf7596\target\sbx\A\run.log" -SimpleMatch -Pattern 'event="box_choice_hover_inject"','event="choice_selected"','event="box_press"'
```

  手順 2 のあいだに強調の注入（`box="fuda" ordinal=Some(0)`／`Some(1)`）、`choice_selected` が `scope=0 id=OnSbYes`（または `OnSbNo`）`box="fuda"` で 1 行、`box_press` が `box="fuda" selected_now=true verdict=ConsumedBySelection` で 1 行。相方のバルーンの `choice_selected`（`scope=1 id=OnSbI7S2`）は手順 2 へ進んだ印。

### 項目 8（2 章の #9 のうち中断）話している最中の箱の左ダブルクリックで中断

```
pwsh -NoProfile -File "C:\home\maz\git\areka\.claude\worktrees\areka-p0-status-execution-bf7596\.kiro\specs\areka-P0-shell-balloon\real-machine-run.ps1" -Run A -Item 8
```

この回はログに `kanade=trace` を自動で足す。

| 手順 | 見出し | 出るもの | すること・見ること |
|---|---|---|---|
| 1/3 | 話している最中に箱をダブルクリックする | 説明だけ | — |
| 2/3 | 青い文字を左ダブルクリック | 右肩に青い「ここをダブルクリック」。そのまま 60 秒話し続ける | 話しているあいだに青い文字を左ダブルクリックする。台詞が止まって文字（相方のバルーンも）が消える。止まると 3 秒後に手順 3 が出る |
| 3/3 | 止まったか | 相方に判定の説明と「この項目はおしまい。」 | 手順 2 で左肩に赤い「止まらなかった」が出ていない |

- 合格の姿: 手順 2 で台詞が止まって文字が消え、赤い「止まらなかった」が出ない（止まらずに 60 秒たつと赤い「止まらなかった」と「次へ」「もう一度」が出る＝不合格の印）。
- ログで判定:

```
Select-String -Path "C:\home\maz\git\areka\.claude\worktrees\areka-p0-status-execution-bf7596\target\sbx\A\run.log" -SimpleMatch -Pattern 'event="choice_selected"','event="box_press"','balloon_break_accepted','id=OnMouseDoubleClick','event="steady_talk"'
```

  手順 2（`id=OnSbI8S2` の後）に `box_press` が 2 行（1 打目 `double_click=None verdict=ShellOp`、2 打目 `double_click=Left verdict=Break`）、`balloon_break_accepted` が 1 行、`id=OnMouseDoubleClick` が 0 行。その約 3 秒後に `event="steady_talk" … origin="OnSecondChange"` が 1 行（手順 3）。

### 項目 9（2 章の #9 のうち立ち絵へ）話していないときの箱のダブルクリックは立ち絵へ

```
pwsh -NoProfile -File "C:\home\maz\git\areka\.claude\worktrees\areka-p0-status-execution-bf7596\.kiro\specs\areka-P0-shell-balloon\real-machine-run.ps1" -Run A -Item 9
```

この回だけバルーンの待ち時間は 300 秒（手順 2 の文字が読み終わる前に消えないように）。ログに `kanade=trace` を自動で足す。

| 手順 | 見出し | 出るもの | すること・見ること |
|---|---|---|---|
| 1/3 | 話していないときに箱をダブルクリックする | 説明だけ | — |
| 2/3 | 青い文字を左ダブルクリック | 右肩に青い「ここをダブルクリック」。出た時点で話し終わっている。**選択肢は出ない** | 青い文字を左ダブルクリックする（それが次へ進む合図） |
| 3/3 | 立ち絵へ届いた | 左肩に赤い「立ち絵へのダブルクリックが届いた。」。相方に「この項目はおしまい。」 | 赤い文字が出る |

- 合格の姿: 手順 2 のダブルクリックで手順 3 の赤い文字が出る（ダブルクリックが立ち絵へ届いた＝ゴーストの `OnMouseDoubleClick` が手順 3 を返した）。
- ログで判定:

```
Select-String -Path "C:\home\maz\git\areka\.claude\worktrees\areka-p0-status-execution-bf7596\target\sbx\A\run.log" -SimpleMatch -Pattern 'event="choice_selected"','event="box_press"','id=OnMouseDoubleClick','event="steady_talk"'
```

  手順 2（`id=OnSbI9S2` の後）に `box_press` の 2 打目が `double_click=Left verdict=ShellOp`、`id=OnMouseDoubleClick` の `SHIORI 送出` が 1 行、続いて `event="steady_talk" … origin="OnMouseDoubleClick"` が 1 行（手順 3）。

### 項目 10（2 章の #10）ポインタを置いているあいだ消えない

```
pwsh -NoProfile -File "C:\home\maz\git\areka\.claude\worktrees\areka-p0-status-execution-bf7596\.kiro\specs\areka-P0-shell-balloon\real-machine-run.ps1" -Run A -Item 10
```

この回だけバルーンの待ち時間は 8 秒。

| 手順 | 見出し | 出るもの | すること・見ること |
|---|---|---|---|
| 1/3 | ポインタを置いているあいだ消えない | 説明だけ | — |
| 2/3 | 青い文字の上にポインタを置いて待つ | 右肩に青い「ここに置く」。5 秒後に台詞が終わる。**選択肢は出ない** | 青い文字（の描かれている所）の上にポインタを置いて動かさずに 15 秒ほど待つ。消えなければ、ポインタを立ち絵からも吹き出しからも外す。8 秒で消えたら、右の子をダブルクリックする（それが次へ進む合図） |
| 3/3 | まとめ | 相方に「この項目はおしまい。」 | — |

- 合格の姿: 手順 2 でポインタを置いているあいだは消えず、外すと 8 秒で消える。
- ログで判定:

```
Select-String -Path "C:\home\maz\git\areka\.claude\worktrees\areka-p0-status-execution-bf7596\target\sbx\A\run.log" -SimpleMatch -Pattern 'event="choice_selected"','event="box_hover_changed"','抑止が成立しているため','抑止が解けたので','バルーンの可視状態が遷移した','event="steady_talk"'
```

  手順 2（`id=OnSbI10S2` の後）に `box_hover_changed` の `to="tate1"` が 1 行以上、`choice=false hover=true` の見送りが 1 行（手順 1 の `choice=true` の行とは別）、外した後に計測のやり直しが 1 行、その 8 秒後に `trigger="timeout" visible=false`。右の子のダブルクリックで `event="steady_talk" … origin="OnMouseDoubleClick"` が 1 行（手順 3）。

`-Item` を付けない `-Run A`／`-Run B` は、1〜3 章の記録を取った最初の通し（起動の台詞＋3 つの段・箱は surface0 などの吹き出しの描き込まれた絵）をそのまま流す。

## 4.1 目視の結果（2026-10-04・開発者が「次へ」で進め、0.7 秒ごとの画面の取り込みとログで判定）

| 項目 | 結果 | 根拠（画面の取り込みの時刻 UTC・ログ） |
|---|---|---|
| 1 | **成立**（5 手順とも意図どおり） | 手順 1（00:28:46）右肩に青い縦書き「一の箱（青）です。」・左肩に赤い縦書き「二の箱（赤）です。」。手順 2（00:28:54）赤い列に「無い名前の後も赤。」が続き、列があふれて左へ折り返す・`\b[nai]` の警告 1 行で行き先は替わらない。手順 3（00:29:00→00:29:03）右肩の青「付いて回る。」が、腕の形が替わる（surface1111）と左上へ移り「腕が替わった。」が続く。手順 4（00:29:09）足元の札に赤い「札は赤。」、左上の列は青のまま「一は青のまま。」（`\f の指定を写さない` 1 行）。手順 5（00:29:19→00:29:25）紫の子の普通の吹き出しに「普通の吹き出しへ。」が出て、3 秒後に右肩の箱へ「箱へ戻る。」が戻る |
| 2 | **成立**（ドラッグの最中と、拡大率の違うモニタへの往復） | 00:45:29.75 に紫の子のドラッグを始め（`DragStartEvent entity=25v0`）、ドラッグの最中（00:45:30.04）も「赤の文字」「青の文字」が左肩・右肩の位置のまま付いて動く。00:45:30.43 に拡大率 2.0→1.5 のモニタへ入り、同じフレームで tate1・tate2 が `k=1.5` で登録し直され、向こうのモニタ（00:45:30.82・00:45:31.62）でも絵との位置関係は同じ。00:45:32.71 に 2.0 へ戻って同じフレームで `k=2.0` で登録し直され、戻った後（00:45:33.91）も同じ。取り込みは 0.7 秒ごとなので、1 フレームのずれの有無はこの取り込みでは見えない（項目 4 で見る） |
| 3 | **成立**（ちらつき無し） | 手順 2（00:48:29.8〜00:49:39.5・8 回のくり返し）のあいだ、約 0.08 秒ごとの取り込み 846 枚を数えた。立ち絵の窓の大きさが同じ前後の 3 枚で、真ん中だけ描かれた画素が 15% 以上減る（立ち絵が一瞬消える・白くなる）枚は 0 枚。描かれた画素が最も少ない枚（00:48:59.30 ほか）は、腕の形が surface1111 で箱の文字が `\c` で空になった正常な姿だった |

- 道具の直し: 相方のバルーンは見える行が少なく、見出しの後の長い説明が流れ去って「どの手順か」が読めなかった。相方には「項目 n 手順 k/N」と見出しだけを出すようにした（説明はこの章の各項目の表）。

## 5. 気付いたこと（判定は変えない）

- **項目 4 の向き**（3.2）: 設計の見込みと逆の向き（新しい絵の上に前の置き場所の文字）が 1 フレーム出ることがある。原因は「文字の層が `\s` を受け取るのはフレームの外で、置き場所に反映されるのは次のフレームの `sync_boxes`」であること。設計の見込みの向きの記述（design.md「描画は毎フレーム、…」の段落）を直す必要がある。目に付くなら上の起票先の案で扱う。
- **`Status` の 1 フレームの空**: 箱の置き場所が替わる切替（surface0 → 1100 など）と、普通のバルーンと箱の受け渡し（`\s[1101]`・`\s[0]`）のたびに、`balloon(ID群)` の写しが 1 フレームだけ空になった（例 `17:39:32.827841 … event="balloon_status_reported" bindings=[]` → 次のフレーム `17:39:32.845732 … bindings=[BalloonBinding { character_id: 0, balloon_id: 0 }]`）。`\s` の受け取りで箱の四角の写しが外れ、次のフレームの提示で戻るまでのあいだ。そのあいだにゴーストへ届く要求があれば `balloon` が欠ける。3.2 と同じ根なので、起票するなら同じ spec で扱う。
- **箱だけのときの `Status` の警告**: 普通のバルーンを一度出して隠した後に箱だけに文字が出ると、`WARN … 見えているバルーンの番号が取れないので 0 として届ける event="balloon_status_surface_unknown" scope=0` が 1 度出た（A1・A2・A3 で各 1 行）。設計（「番号は今までどおり…`current_surface_id`（取れなければ 0）」）どおり 0 で届いているが、箱を使う普通の使い方で警告の段の行が出る。水準か、隠した後の番号の持ち方を見直す余地がある。
- **A2 がシェル `second` で起きた**: 最後に使ったシェルの記憶は、areka の記憶の置き場（`AREKA_PROFILE_DIR`）とは別に、ゴースト側（`ghost\master\profile\areka\sylphya.toml`）にも残る。`real-machine-run.ps1` の `-Run` は、この記憶も毎回消すように直した（A3 から）。A2 自体は second の定義どおりに振る舞った（surface1100 に箱が無いので `\b[fuda]`・`\b[tate1]` が警告 1 行ずつで切り替わらなかった）。
- **本 spec と関係の無い既存の警告**: `purple/a/null.png` の全透明（2〜3 行）、`emo2-kakukaku` の折返し基準（1 行）、自動終了の `kanade: 強制終了指示——終了系列（Forced）へ直行`（1 行・`areka-P0-status-execution-states` の自動終了の走行にも同じ行がある）。
- **台本の流し方**: 起動の台本は `sample-ghost-kit` の `nar-sample-path`（`manual_paths`）で展開した写しの中だけで差し替えた。`SampleRoot::acquire` の使い捨ての写しは値を落とすと消えるため、プロセスの外で長く使う実機の根には向かない（`areka-P0-shell-balloon-switch`・`areka-P0-status-execution-states` の記録と同じ取り方）。
- 生のログ・窓の記録は `target\sbx\` に残した（コミットしない）。
