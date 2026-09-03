# ギャップ分析: areka-P0-ukadoc-survey-assets

> 実施日: 2026-09-02 / 対象: 確定済み requirements.md（要件 1〜10）と現行コードベース・正典スナップショットの差
> 方針: **決めない・並べる**。事実と選択肢だけを示し、決定は要件ディスカッションと設計フェーズに委ねる。
> ここに書いた事実はすべて、この作業ツリーで `file:line` を実際に開くか、スナップショットを機械で数え直して確かめた。確かめられなかったものは「未確認」と明記する。
> 上流契約 = `.kiro/specs/areka-P0-ukadoc-survey-toolkit/requirements.md`（承認済み・付録 A / 付録 B を含む）。本 spec はこれに従い、再発明しない。

---

## 0. 要約（5 行）

- **要件が挙げた件数はすべて再現した**。担当 24 ページ合計 542・アンカー無し 15・相異なる見出し 477・重複群 40／関与 105・ページ内重複は `descript_shell_surfaces` の `bind` 1 組だけ・descript 518 のうち読点付き 425（キー名 349 種）／読点無し 93——これらは要件本文の数値と 1 件も違わなかった（§1.1）。
- **台帳の 3 分の 1 は機械で埋められる**。542 件のうち **178 件**は「areka の受理キー表との文字列一致」「合成メソッド語の写像表との突き合わせ」「実装ゼロが確定しているページ」の 3 つの規則だけで状態が決まる。残る 364 件も大半は素直に「未対応」で、人手が本当に要るのは優先度・テーマ・備考の文章である（§1.4）。
- **正典に居場所が無い areka の語が 6 つある**。`surface.append`・`kero.surface.alias`・`bind+random` はスナップショット全 1,749 件のどこにも現れず（見出し 0・本文 0）、`descript`・`surface`・`ascend`・`descend` は独立した項目を持たない。要件 4.3 は「`bind+random` が駆動する」ことを表に書けと命じるが、**貼り付ける先の項目 id が無い**（§1.1・§3-2）。
- **スナップショットは 2.8.82 で止まっている**。全 1,749 件の本文に `2.8.83` 以降は 1 件も現れない。一方 `doc/COMPAT_ARCHITECTURE.md:185` は「正典が 2.8.83 で改訂した」箇所を登記している。本ドメインの最新は `descript_shell_surfaces` の `element*` が 2.8.82、`descript_balloon` の `vertical` が 2.8.80（§1.1・§6-1）。
- **要件本文に 3 か所の事実誤りを見つけた**。⑴ `updates.txt`・`delete.txt`・`install.txt` の実ファイルが `crates/` 配下に**実在する**（要件 5.6 は 0 件と書く）。⑵ `areka-parsers` の記録経路は 1 つではなく 3 つ（要件 3.3 は 1 つと書く）。⑶ 版番号の件数は抽出規則しだいで 5 件動く（§3）。いずれも仕訳の結論を変えるものではないが、そのまま書き写すと台帳の備考が誤りを引き継ぐ。

---

## 1. 現状の実測

### 1.1 正典側（スナップショットを機械で数え直した）

スナップショットは実在する。`C:\Users\maz-o\AppData\Roaming\npm\node_modules\ukagaka-doc-mcp\data\index.json`（2,716,948 バイト・`npm root -g` が返す場所の下）。最上位キーは `version`（= 1）・`generatedAt`（= `2026-08-24T04:08:57.881Z`）・`entries`（2,983 件）。環境変数 `AREKA_UKADOC_SNAPSHOT` は未設定。上流契約 付録 B の手順（コロンで分割・`_` で分割しない）がそのまま動く。

| 確認項目 | 実測値 | 要件の記載 | 一致 |
|---|---|---|---|
| 担当 24 ページの合計 | 542 | 542 | ○ |
| ページ別内訳 | balloon 162・surfaces 137・shell 102・ghost 74・install 15・plugin 13・headline 9・surfacetable 6・update_file 9・`manual_*` 各 1（8 本）・`dev_*` 各 1（6 本）・`memo` 1 | 同 | ○ |
| カテゴリ内訳 | descript 518・protocol 9・file_structure 8・dev_guide 7 | 同 | ○ |
| アンカーを持たないページ全体の項目 | 15（`manual_*` 8・`dev_*` 6・`memo` 1） | 15 | ○ |
| 相異なる見出し | 477 種 | 477 | ○ |
| 重複群／関与件数 | 40 群・105 件 | 40／105 | ○ |
| ページ内の重複 | `descript_shell_surfaces` の `bind` 1 組のみ | 同 | ○ |
| descript 518 のうち読点付き | 425（読点前のキー名 349 種） | 425／349 | ○ |
| descript 518 のうち読点なし | 93 | 93 | ○ |
| 項目 id に逆斜線・引用符・非 ASCII・空白 | いずれも 0 件（最大長 153 文字・542 件すべて相異なる） | — | — |
| 見出しに逆斜線 1 件・単引用符 3 件・二重引用符 0 件 | 同 | — | — |

**項目 id は TOML のキーとしてそのまま書ける**（引用符も逆斜線も含まない）。上流契約 付録 A.3 が言う逆斜線の書き分けは、本ドメインでは 1 件も必要にならない。

**重複 40 群の中身**（要件 6.5・7.6 に直結）。上位は `charset,文字コード` が 8 ページ（本ドメインの descript 系 8 ページ全部）、`type,種別`・`homeurl,URL`・`readme,ファイル名`・`readme.charset,文字コード` が各 5 ページ、`craftman`／`craftmanw`／`craftmanurl` が各 4 ページ。ページを跨いだ重複だけで、descript 各ページの中ではキー名は一意（`descript_ghost` 74 件＝74 種・`descript_shell` 102 件＝102 種・`descript_balloon` 162 件＝162 種）。**したがって「ページを決めてからキー名で引く」ならぶつからない**。

ドメインを跨ぐ同名も実在した。`homeurl` は本ドメイン 5 ページに加えて `list_propertysystem` と `list_shiori_resource` にもある（計 7 ページ）。`name` は 9 ページ、`id` は 5 ページ、`craftman` は 5 ページ。要件 7.6 の例（`homeurl`）は実測どおりである。

**版番号**（要件 2.7・4.2 の入力）。「本文に `x.y.z` が現れる件数」はページごとに次のとおり（語境界を付けた抽出）。

| ページ | 版番号を含む件数 | 相異なる版番号 | 最新 |
|---|---|---|---|
| `descript_shell_surfaces` | 71 / 137 | 23 | 2.8.82 |
| `descript_balloon` | 31 / 162 | 12 | 2.8.80 |
| `descript_shell` | 22 / 102 | 7 | 2.8.53 |
| `descript_ghost` | 8 / 74 | 6 | 2.6.89 |
| `descript_install` | 1 / 15 | 1 | 2.7.52 |
| `descript_plugin`／`descript_headline` | 各 1 | 各 1 | 2.5.10 |
| `spec_update_file`・`descript_shell_surfacetable` | 0 | 0 | — |
| `manual_balloon` 1・`dev_nar` 1・他の `manual_*`／`dev_*`／`memo` | 0 | — | — |

本ドメインで 2 つ以上の版番号を含む項目は 3 件。ドメイン全体の最新は 2.8.82（`descript_shell_surfaces` の `element*`）。

**areka の語のうち正典に居場所が無いもの**（本ドメインで最も重い発見）。

| areka の語 | 見出しとして | 本文中の出現 | 備考 |
|---|---|---|---|
| `surface.append` | 0 件 | **全 1,749 件で 0 件** | `shell/decode.rs:127` が認識する塊の見出し |
| `kero.surface.alias` | 0 件 | **全 1,749 件で 0 件** | 同 `:122`。`ukadoc:manual_shell` は「`alias.txt` 旧仕様・`sakura(kero).alias` ブレス・現在は surfaces.txt に統合」と書くのみで、綴りが違う |
| `bind+random` | 0 件 | **全 1,749 件で 0 件** | 同 `:391` が認識し `areka-seriko/src/table.rs:107-109` が駆動する。**要件 4.3 が表に載せろと言う 2 語のうち片方に貼り先が無い** |
| `descript` | 0 件 | 134 件（他項目の説明文の中） | 同 `:118` が認識する塊 |
| `surface` | 0 件 | 55 件（同上） | 同 `:132` |
| `ascend`／`descend` | 0 件 | 各 2 件（`animation-sort`／`collision-sort` の本文の中） | 同 `:495`・`:496`。値の語であって項目ではない |

つまり **surfaces.txt の「塊の見出し」に相当する正典項目は存在しない**。ukadoc の `descript_shell_surfaces` は「キー 1 個＝1 アンカー」で作られており、ファイルの骨格（どういう塊があるか）は `ukadoc:manual_shell`（ページ全体項目・アンカー無し）の散文に埋まっている。要件 1.10 が「粒度が粗い」と書いた 15 件が、実際にはこの穴を埋める唯一の受け皿になる。

**導線の外部依存先は機械で列挙できる**（要件 5.4・5.5）。本文検索で次が確認できた。

- さくらスクリプト台帳側: `ukadoc:list_sakura_script:\![execute,install,path,ファイル名]`・`\![execute,install,url,URL,(feed|nar|homeurl のいずれか)]`・`\![execute,createnar]`・`\![execute,createupdatedata]`・`\![update,更新対象(,オプション...)]`・`\![updateother,...]`（id はアンカー符号化された形で実在する）。
- shiori 台帳側: `OnInstallComplete`・`OnInstallCompleteAll`・`OnInstallRefuse`・`OnInstallReroute`・`OnNarCreating`・`OnNarCreated`・`OnUpdatedataCreating`・`OnUpdatedataCreated`・`OnUpdateProcessExec`・`OnUpdateBegin`・`OnUpdateReady`・`OnUpdateComplete`・`OnUpdateFailure`・`OnUpdate.OnDownloadBegin`・`OnUpdate.OnMD5CompareBegin`・`OnUpdate.OnMD5CompareComplete`（`OnUpdate` の本文一致は 28 件あり、上記はその一部）。

**独立項目を持たない配布ファイル**（要件 5.3 が名指しするもの）。`delete.txt`（本文 3 件）・`developer_options.txt`（7 件）・`thumbnail.png`（9 件）・`updates2.dau`（12 件）・`install.txt`（7 件）は、いずれも**見出しとしては 0 件**で、`ukadoc:manual_update`・`ukadoc:manual_install`・`ukadoc:manual_directory`・`ukadoc:dev_nar`・`ukadoc:dev_update` の散文にしか現れない。導線ブリーフィングの各段は、この 5 つのページ全体項目に寄りかかることになる。

### 1.2 areka 側（file:line はこの作業ツリーで再検証した）

**ゴースト descript**（`crates/areka-parsers/src/package/resolve.rs`）

- 完全一致で引くキーは 6 つ。`name`（`:69`）・`sakura.name`（`:70`）・`sakura.name2`（`:71`）・`kero.name`（`:72`）・`shiori`（`:78`）・`seriko.defaultsurfacedirectoryname`（`:83`）。
- 冒頭の宣言 `:7-8` は「参照キーは name / sakura.name / kero.name / shiori / seriko.defaultsurfacedirectoryname のみ（install.txt / balloon 系 / NAR には触れない）」と書くが、**`sakura.name2` が列挙から漏れている**（実際には `:71` で読んでいる）。要件が指摘したとおり陳腐化している。

**シェル descript**（同ファイル）

- 接頭辞＋接尾辞の組で 6 形。`sakura.bindgroup`／`kero.bindgroup`（`:111`・`:112`）× `.default`（`:114`）／`.name`（`:116`）と、`sakura.bindoption`／`kero.bindoption`（`:118`・`:119`）× `.group`（`:121`）。照合の本体は `:171-215`。オプション語は `mustselect`（`:123`）と `multiple`（`:129`）の 2 語で、`:256-257` が拾う。
- `seriko.zorder` は parsers ではなくアプリ側が読む（`crates/areka/src/emo2_boot/frame/zorder_descript.rs:1`・`:12-13`・`:47`）。

**charset**（`crates/areka-parsers/src/charset/`）

- 宣言の抽出は `prescan.rs:54`（`key.trim().eq_ignore_ascii_case("charset")`）。クレート内で唯一、大小文字を無視する照合である。分割は最初の読点だけ（`:52`）。
- 解決は `decode.rs:30`。**未対応ラベルは `tracing::debug!` で記録を残して既定へ落ちる（`:35`）**。デコード中の不正バイト列も `tracing::debug!` で記録を残す（`:52`）。

**KV 化**（`crates/areka-parsers/src/kv/parse.rs`）

- 関数は `:20`、最初の読点で分割 `:26`、キーが空の行はスキップ `:34`、同じキーは後勝ちで上書き `:39`。分類も型付けも記録もしない。

**バルーン descript**（`crates/areka-parsers/src/balloon/parse.rs`・全 168 行）

- 完全一致で引くキーは **30 種・照合箇所 31**（`windowposition.x` だけ `:72` と `:80` の 2 か所で引く）。位置 `:72`・`:73`・`:80`・`:81`、幾何 `:84`・`:85`・`:88`・`:89`・`:92`〜`:95`、フォント `:98`〜`:100`・`:104`・`:105`、書字方向 `:110`（`writing_mode`）・`:113`（`budoux_newline`）・`:119`（`vertical`）、選択カーソル `:128`・`:130`〜`:132`・`:135`〜`:137`・`:140`〜`:142`・`:145`。
- 「未知キーは完全一致引きゆえ自然に無視」の明文は `:9`・`:39`・`:122-126`。
- `vertical` が正典キーであること（SSP 2.8.80 で確立・`0`／`1`）の明文は `:115`。areka 独自の拡張は `writing_mode`（`:110`）と `budoux_newline`（`:113`）の 2 つだけ。

**surfaces.txt**（`crates/areka-parsers/src/shell/decode.rs`・全 558 行）

- 認識する語は 17。塊の見出し＝`descript`（`:118`）・`kero.surface.alias`（`:122`）・`surface.append`（`:127`）・`surface`（`:132`）。塊の中＝`element`（`:197`）＋第 2 欄が `overlay`（`:198`）・`collision`（`:234`）・`animation`（`:310`）・`interval`（`:323`）・`pattern`（`:334`）。間隔語＝`bind`（`:387`）・`random`（`:388`）・`bind+random`（`:391`）。並び順＝`animation-sort`（`:501`）・`collision-sort`（`:502`）と値 `ascend`（`:495`）・`descend`（`:496`）。
- `charset` は照合されず素通りする（明文は `:90` と `:490`）。
- `element` は**第 2 欄が `overlay` の行だけを値にし、それ以外のメソッドは何も記録せず読み飛ばす**（`:187-188`・実装 `:197-199`）。
- `collisionex`（円・楕円・多角形）は**何も記録せず読み飛ばす**（明文 `:226-227`・実装 `:234-236`）。純粋な `collisionN`（N が数字だけ）のみを値にする。
- 未知の間隔語は原文のまま `Interval::Other` へ忠実に写す（`:380-396`）。

**間隔語の駆動側**（`crates/areka-seriko/src/table.rs`）

- 採録するのは `Random`（`:107`）と `BindRandom`（`:108-109`）の 2 つだけ。`Bind` は静的な着せ替えゆえ非採録で `tracing::debug!` を出す（`:111-117`）。`Other` は**元の語を添えて** `tracing::debug!` を出す（`:118-127`）。将来の値も `:128-135` で記録付き非採録。
- つまり「`sometimes` と書いたのに動かない」は記録に残る。**黙って落ちるわけではない**（要件 3.5 の壊れ方判定に効く）。

**合成メソッド**（`crates/areka-emo-compose/src/method.rs`・全 395 行）

- 実挙動を持つのは `Overlay` だけ（`:130-132`）。語彙の受け口は 10 種（`:242-251` が全数を列挙するテストを持つ）。
- 名前の解決は `:142`。小文字化してハイフンと下線を除いてから照合する（`:143-145`）。`overlay`／`add`／`bind` は同義として `Overlay` へ（`:148`）。`base` は `:153`。
- **合成メソッドの写像表は正典の別名関係をすでに持っている**。`parse_blend`（`:173`）が旧書式 `overlaymultiply` = `blend-multiply-fast`、`overlayscreen` = `blend-screen-fast` を明示写像する（`:176-177`）。種別は 19 種（`:185-203`）で、それぞれ通常と `-fast` の 2 形＝**38 名を解決できる**。
- 解決できない名前は `tracing::warn!` を出して `Unknown` へ吸収する（`:160-161`）。

**記録経路の全数**（要件 3.3 に直結）。`crates/areka-parsers/src/` のテスト以外のファイルで記録を出す行は **3 つ**である。

| ファイル:行 | 段 | 内容 |
|---|---|---|
| `crates/areka-parsers/src/package/resolve.rs:296` | `warn!` | bindgroup の名前宣言にパーツ名が無い |
| `crates/areka-parsers/src/charset/decode.rs:35` | `debug!` | 未対応の charset ラベル → 既定へ落とす |
| `crates/areka-parsers/src/charset/decode.rs:52` | `debug!` | デコード中の不正バイト列を代替文字で吸収 |

エラー段（`error!`）は 0 件。**要件 3.3 の「記録を残す経路が 1 つだけ」は `warn!` に限れば正しいが、記録全体では 3 つある**（§3-3）。

**インストール・更新・nar**

- Rust から `updates2.dau` を参照する行 0 件、`updates.txt` を参照する行 0 件、`.nar` を参照する行 0 件、`OnUpdate` を参照する Rust の行 0 件（`OnUpdate` の 5 件はすべて試験用ゴースト辞書 `crates/pilot/examples/shiori-host-32/fixtures/emo2/ghost/master/dic/update.pasta:4`・`:8`・`:12`・`:16`・`:17` の中）。
- **ただし実ファイルは存在する**（要件 5.6 の「0 件」と食い違う・§3-1）。`crates/pilot/examples/shiori-host-32/fixtures/emo2/updates.txt`・`.../emo2/ghost/master/updates.txt`・`.../emo2/delete.txt`・`.../emo2/install.txt`・`.../emo2/emo2-kakukaku/install.txt`・`.../emo2-kakukaku-offsetdpi/install.txt`・`.../emo2-kakukaku-wplimit/install.txt` の 7 本。
- しかもこれらは**正典書式の実物**である。`updates.txt` の 1 行目は `charset,UTF-8`、2 行目以降は `file,<相対パス><MD5>size=<数値>date=<時刻>`（`crates/pilot/examples/shiori-host-32/fixtures/emo2/updates.txt:1-2`）で、`ukadoc:spec_update_file` の「行フォーマット」「行種別」「必須フィールド」「拡張フィールド」の実例になっている。`install.txt` は `Charset,UTF-8`／`type,ghost`／`name,えも？？`／`directory,emo2`／`balloon.directory,emo2-kakukaku`／`balloon.source.directory,emo2-kakukaku`（`.../emo2/install.txt:1-6`）で、`descript_install` の `*.directory`・`*.source.directory` の実例になっている。`delete.txt` は `charset,UTF-8` と削除対象 1 行（`.../emo2/delete.txt:1-2`）。
- install.txt を「触れない」とする宣言は `crates/areka-parsers/src/package/resolve.rs:8`、結果に影響しないことを固定するテストは `crates/areka-parsers/src/package/validation_tests.rs:113`（採取元の列挙）・`:123-126`（doc コメント）・`:128`（テスト本体）。

**プラグイン・ヘッドライン・オーナードローメニュー・surfacetable**

- `crates/areka-parsers/src/` に plugin／headline の解析は無い（モジュールは `balloon`／`charset`／`kv`／`package`／`sakura`／`shell` の 6 つのみ）。
- `crates/` 全体で `menuitem`・`ownerdraw`・`owner_draw` を含む Rust の行は 0 件。`surfacetable` を含む Rust の行も 0 件（2 件はいずれも試験用 `updates.txt` の中身）。
- 名前の予約だけはある。`crates/areka-sylphya/src/vocab/dotted.rs:24`（`headlinelist`）・`:25`（`pluginlist`）、および `:104` の「M1 は名前の予約に留め、イベント発火（ext 亜枝の実働）は M2 の領分」。

**成果物の置き場と証拠の現状**

- `doc/ukadoc-coverage/` は存在しない（`ls` で不在を確認）。
- `crates/**/*.rs` で「ukadoc」の語を含む行は 156、そのうち `http` を同じ行に持つ行は **0**、`shillest` を含む行も **0**。本 spec が置く URL が最初の 1 件になる。

### 1.3 取り込める既存の判定表

本ドメインの仕訳をゼロから起こす必要はない。既に「項目 × areka の判定 × 根拠 × 担当 spec」の形をした表が repo にある。

| 出所 | 行数 | 何が入っているか |
|---|---|---|
| `doc/COMPAT_ARCHITECTURE.md:126`（見出し `:122`・区切り `:127`・データ行 `:128-207`） | 80 | 項目・裁量・根拠・出典 spec の 4 欄。本ドメインに関わる行は `:135`（`point.basepos`）・`:137`（`sakura.name2`）・`:138`（`kero.name`）・`:139-144`（バルーン系列名と `balloon.defaultsurface` 族）・`:145-150`／`:166-175`（`windowposition` 族）・`:176-188`（`vertical`／`writing_mode`／縦書き族）・`:189-191`（バルーン位置の単位）・`:192-206`（`seriko.zorder`）・`:207`（`currentghost.seriko.zorder`＝プロパティ台帳側） |
| `doc/emo2-conformance-scope.md:80`（見出し `:78`・データ行 `:82-88`） | 7 | `:82` が SERIKO/MAYUNA の縮小（「完全マップ」→ SERIKO/2.0 ＋ MAYUNA bind・overlay・interval 3 種・矩形 collision）。要件 4.6 の引用先はここで正しい |
| `.kiro/specs/completed/areka-P0-mayuna-compose/design.md:61`（データ行 `:63-68`） | 6 | 「正典キー／タグ・意味・**分類**・M1 取り扱い」。分類の語が本 spec の状態語彙にほぼそのまま写せる（実導出／実装済／部分実導出／範囲外／語彙保持・実挙動なし）。`bindgroup*.name`・`bindgroup*.default`・`bindgroup*.addid`・`bindoption*.group` を名指しで判定済み |
| `.kiro/specs/completed/areka-P0-balloon-vertical-canon/design.md:733`（データ行 `:735-747`） | 13 | 縦書き 13 項目の裁量表。`COMPAT_ARCHITECTURE.md:176-188` の元になった表なので、取り込むときは重複に注意 |
| `.kiro/specs/completed/areka-P0-balloon-visibility/design.md:490`（データ行 `:492-502`） | 11 | 「事項・裁量／記録内容・根拠区分・**追跡先**」。追跡先の欄がそのまま担当 spec になる。`COMPAT_ARCHITECTURE.md:155-165` の元 |
| `.kiro/specs/completed/areka-P0-kero-balloon/research.md:24`（データ行 `:26-33`）＋ `design.md:729-733` | 9＋5 | バルーン系列名・`windowposition.*`・`balloon.defaultsurface` 族の正典対照 |
| `.kiro/specs/completed/areka-P0-balloon-parse/research.md:49`（データ行 `:51-58`）＋ `:60` | 8＋1 | バルーン descript のキー × ファイル層。`:60` が「モデル化しないキー」を明示列挙（`arrow0/1.x/y`・`number.xr/y`・`onlinemarker.*`・`sstpmarker/message.*`） |
| `.kiro/specs/completed/areka-P0-package-mount/research.md:66`（データ行 `:68-70`） | 3 | ゴースト descript の既定値（`seriko.defaultsurfacedirectoryname` = `master`・`shiori` = `shiori.dll`） |
| `.kiro/specs/completed/areka-P0-shell-parse/design.md:525`（データ行 `:527-532`）＋ `:548-553` | 6＋6 | surfaces.txt の行文法と、吸収・エラーの対照 |

**未着手の隣接 spec 側**（要件 7.2 の担当取り込み）

- `areka-P0-balloon-canon-residue/brief.md` — 番号付き項目は **12 個**（1〜6 が `:11`〜`:16`、7〜10 が `:24`〜`:27`、11・12 が `:33`・`:35`）。Scope 行 `:58` は「6 項目＋追加登記の 7〜10」と書いており **10 のまま追随していない**（項目 11・12 が 2026-08-29 追加であることは `:29` に明記）。要件 7.3 の指示（12 を採る）は実測と一致する。なお `:91` に「項目 13」があるが**消化済み**と明記されているので、12 と数えるのが正しい。
- `areka-P0-surfaces-basepos/brief.md:38` — `point.basepos.x`／`.y` の 2 キー。
- `areka-P0-text-decoration-canon/brief.md:33` — バルーン descript の `font.*` 基底「13 キー」と `disable.font.*`。⚠ **13 という数だけがあり、13 個の名前は列挙されていない**（`:19` は解析済み 5＋未解析の族名を書くだけ）。台帳側で名前を確定する必要がある。
- `areka-P0-anchor-tag-canon/brief.md:30`・`:16` — `anchor.font.*`／`anchor.notselect.font.*`／`anchor.visited.font.*` の 3 族。
- `areka-P0-choice-marker-styling/brief.md:26`・`:8` — バルーン descript の `cursor.*`。`:12` に「`underline` を `SquareFill` へ落とす（1 度だけ警告）」という既存の縮退判定があり、そのまま取り込める。
- `areka-P0-charset-canon/brief.md:19-26` — 7 行の層別表（各行に ✅／⚠ decode 迂回／❌ 未解析／対象外の判定と file:line）。`:50` が `shiori.encoding`／`shiori.forceencoding` と surfaces.txt 読取 2 箇所を担当と宣言。⚠ **`:51` の Out 行が `updates2.dau`・`install.txt`・`readme.charset`・`surfacetable.txt` の文字コードを対象外と宣言している**——つまりこれらの `charset` 項目は**担当なし**になる。
- `areka-P0-zorder-property/brief.md:9` — `currentghost.seriko.zorder` 1 件のみ（プロパティ台帳側）。`:52` に `currentghost-property-tree`・`property-query-channels` との**三重所有の未決**が登記されている。本ドメインの項目ではないので assets 台帳は関連で指すだけでよい。
- `areka-P0-zorder-chain-residue/brief.md` — 正典キー名を 1 つも持たない（テストと文書の負債表）。本ドメインの取り込み対象にならない。

**担当 spec 名の実在**（要件 7.2 が名指しする 11 本）。`package-mount`・`shell-parse`・`balloon-parse`・`balloon-visibility`・`kero-balloon`・`balloon-vertical-canon`・`balloon-offset-dpi`・`bindoption-exclusivity`・`windowposition-limit`・`scope-zorder-pinning` は `.kiro/specs/completed/` に、`surfaces-basepos` は `.kiro/specs/` に実在する。**宙に浮いた名前は 1 つも無い**。

### 1.4 台帳 542 件の作り方の規模

機械で状態が決まる分と、人手が要る分を数えた。

| 区分 | 件数 | 決め方 |
|---|---|---|
| areka の受理キー表と見出しが文字列一致する | 62 | ゴースト 7・シェル 8（ワイルドカード形）・バルーン 29・surfaces 18 |
| 合成メソッドの `blend-*` | 55 | 写像表（`method.rs:173-204`）で解ける 38 ＝ 語彙のみ／解けない 17 ＝ 未対応 |
| プラグイン・ヘッドラインの descript | 22 | 解析コードが無い＝全件未対応（名前の予約のみ・`dotted.rs:24-25`） |
| `descript_install` | 15 | 実装ゼロ＝全件未対応 |
| ページ全体項目 | 15 | 粒度が粗い旨を備考に書く（要件 1.10） |
| `spec_update_file` | 9 | 実装ゼロ＝全件未対応 |
| **上記の小計** | **178** | **機械で下書きできる** |
| 残り | 364 | 人手。ただし大半は素直に「未対応」で、実質の作業は優先度・テーマ・備考の文章 |

**キー名の文字列一致は実際に成立する**。バルーン 30 キーのうち 28 が `descript_balloon` の見出しと 1 文字も違わずに一致した（一致しないのは areka 独自の `writing_mode` と `budoux_newline` の 2 つだけ＝正典に項目が無いので台帳の行も無い）。ゴーストは 7/7 一致。**シェルだけは形が違う**——ukadoc は `sakura.bindgroup*.default` のようにワイルドカードで書き、areka は接頭辞＋番号＋接尾辞で照合する。**「ukadoc の `*` は areka の数値番号に対応する」という読み替え規則を 1 つ決めれば機械化できる**（§7-4）。

**`blend-*` 55 件の内訳**（要件 4.4 に直結）。写像表が解ける 38 件（19 種 × 通常／`-fast`）と、解けずに `Unknown` へ落ちる 17 件——`blend-add-glow`・`blend-add-glow-fast`・`blend-color-dodge-glow`・`blend-color-dodge-glow-fast`・`blend-dither`・`blend-linear-burn(-fast)`・`blend-linear-light(-fast)`・`blend-pin-light(-fast)`・`blend-soft-light(-fast)`・`blend-subtract(-fast)`・`blend-vivid-light(-fast)`。前者は「語彙のみ」、後者は「未対応」で、しかも**後者は `warn!` が出る**（`method.rs:160`）＝壊れ方の段が違う。

**`bind` 2 件の中身**（要件 4.7）。`ukadoc:descript_shell_surfaces:bind:1` は間隔語（「そのアニメーションをサーフェスの着せ替えとして定義する」）、`:2` は合成メソッド（「新規レイヤを着せ替えパーツとする。……現在は add が互換。処理の内容は overlay と同義」）。**`:2` の本文が別名の向きを自分で書いている**——`add` が後継で `overlay` と同義。これは `method.rs:148` の `"overlay" | "add" | "bind" => Overlay` とぴたり一致する。要件 2.5 の順序（本文の注記 → 版番号 → 人手）の第 1 段でそのまま決まる好例である。

**間隔語の別名関係も本文が書いている**。`sometimes` =「毎秒 2 分の 1 の確率で再生」、`rarely` =「毎秒 4 分の 1」——つまり `random,2`／`random,4` と同じ意味である。`always`・`runonce`・`never` は本文に定義がある。いずれも areka では `Interval::Other` に落ち、**元の語を添えた記録が出る**（`table.rs:118-127`）。

---

## 2. 要件と既存資産の対応（不足・未確定・制約）

| 要件 | 使える既存資産 | 状態 | 補足 |
|---|---|---|---|
| 1（台帳の新設・全数収容） | スナップショット直読み（付録 B の手順が実際に動く） | **不足だが作れる** | 542 件・id は TOML キーとしてそのまま書ける。`doc/ukadoc-coverage/` は未作成なのでディレクトリごと新設 |
| 1.7（道具未着地時の写し取り） | 付録 B の Python 例 | 使える | 本分析で同じ手順を実行し 542 を再現した |
| 2（仕訳と状態語彙） | `COMPAT_ARCHITECTURE.md:128-207`・`mayuna-compose/design.md:63-68` ほか §1.3 の 9 本 | 使える | 分類の語が既に近い。写す向きの対応を 1 度決めればよい |
| 2.9（沈黙ルール表 44 行／うち 16 行） | 同上 | **未確定** | 80 行に「どのドメインの項目か」を示す欄が無く、44 と 16 は機械で再現できない（§6-3） |
| 3（未知の記述の扱い・8 種） | `resolve.rs:296`・`decode.rs:35`・`:52`・`kv/parse.rs:39`・`shell/decode.rs:234`・`table.rs:111-135`・`method.rs:160` | 使える | ただし要件 3.3 の「1 つだけ」は `warn!` 限定の話（§3-3） |
| 4（SERIKO/MAYUNA 世代表） | `emo2-conformance-scope.md:82`・`mayuna-compose/design.md:63-68`・`shell-parse/design.md:527-532` | 使える | `bind+random` に貼り先の id が無い（§3-2） |
| 5（導線ブリーフィング） | `ukadoc:manual_install`・`manual_update`・`manual_directory`・`dev_nar`・`dev_update` の散文＋試験用 `install.txt`／`updates.txt`／`delete.txt` の実物 | 使える | 実装は本当にゼロだが、**実データはある**（§3-1） |
| 6（ソースへの URL） | 前例なし（`http` 付き ukadoc 行 0 件） | **不足** | 書き方は上流契約 5.1〜5.4 が凍結済み。`charset` が 8 ページに散る問題がある（§7-3） |
| 7（担当の取り込み） | §1.3 の隣接 brief 9 本＋完了 spec 10 本 | 使える | 名前はすべて実在。`text-decoration-canon` の「13 キー」だけ名前が未列挙 |
| 8（テーマと優先度） | `.kiro/steering/roadmap.md:148` がテーマ 8 と序列を登記 | 使える | `doc/ukadoc-coverage/values.md` は未作成（上流の道具の成果物）。テーマ名は roadmap から引ける |
| 9（報告の再生成） | 無し（道具未着地） | **未確定** | 要件 9.3 の退避路（台帳とブリーフィングだけを成果物にする）を採る前提で進むのが現実的 |
| 10（非接触） | 1,000 行の上限は `crates/**/*.rs` のみ（`crates/log-capture-kit/tests/workspace_scan/mod.rs:38`・`:81`・`:103`） | 制約なし | **`doc/` 配下のデータファイルは上限の対象外**。台帳が何千行になっても検査は赤にならない |

---

## 3. 要件の記述と実測の食い違い（訂正候補）

要件は確定済みなので本 spec では書き換えない。**台帳と文書に書き写すときにここを直す**か、要件ディスカッションで訂正するかを選ぶことになる。

> **2026-09-03 要件ディスカッションでの処置**: 下記 1〜7 はすべて要件本文へ訂正を反映した（1 → Introduction と要件 5.6／2 → 要件 4.3／3 → Introduction と要件 3.3／4 → 要件 2.7 のまま・判断 10 へ／5 → Introduction／6 → 要件 5.7／7 → 要件 7.2）。併せて §7 の判断 2・4・6・9 は要件側の文言で解決した（要件 4.3・1.3・2.8・2.9・7.4）。

1. **`updates.txt`・`delete.txt`・`install.txt` は `crates/` 配下に実在する**（要件 5.6 は「いずれも 0 件」と書く）。実ファイル 7 本の一覧と中身は §1.2 に示した。「Rust のコードから参照する行が 0 件」なら正しい。**この差は導線ブリーフィングにとって有利な材料である**——正典書式の実物が手元にあるので、`spec_update_file` 9 項目と `descript_install` 15 項目を実データに当てて確かめられる。
2. **`bind+random` に対応する正典項目が無い**（要件 4.3 は表に載せろと言う）。スナップショット全 1,749 件で見出し 0・本文 0。`bind` と `random` は別々の項目として実在するので、⑴ 2 つの項目の備考に分けて書く、⑵ areka 独自の語として本文（ブリーフィング）側だけに書く、⑶ ページ全体項目 `ukadoc:manual_shell` の備考に書く、の 3 択になる（§7-2）。
3. **記録経路は 1 つではなく 3 つ**（要件 3.3 は `resolve.rs:296` の 1 つだけと書く）。`charset/decode.rs:35`（未対応ラベル）と `:52`（不正バイト列）はいずれも `debug!` で記録を残す。`warn!` に限れば 1 つで正しい。**要件 3.5 の壊れ方判定（黙って壊れるか）に直接効くので、段を明記して書き分けるのが安全**。同じことが parsers の外にもあり、`areka-seriko/src/table.rs:111-135` と `areka-emo-compose/src/method.rs:160` も記録を出す。
4. **版番号の件数は抽出規則で変わる**。要件が挙げる「balloon 32・ghost 11・install 2」は、`\d+\.\d+\.\d+` を**語境界なし**で当てた場合の値であり、語境界を付けると 31・8・1 になる（surfaces 71・shell 22 はどちらでも同じ）。要件は「抽出規則は上流の道具が凍結するので件数を固定しない」と明記しているので矛盾ではないが、**上流の道具が語境界付きを採ると台帳の登場版が 5 件ずれ、上流契約 6.7 の照合が赤になりうる**（§6-2）。
5. **`balloon/parse.rs` の行番号が 1 つずれている**。要件は「`vertical` は正典キー（`:116` に SSP 2.8.80 と明記）」と書くが、実際の明文は `:115`、`vertical` を引く行は `:119`。同様に「未知キーの明文 `:124-125`」は実際には `:122-126`。ほかの参照（`:9`・`:39`・`:110`・`:113`）は一致した。
6. **`emo2-conformance-scope.md` に install／update／nar の言及が 3 か所ある**（要件 5.7 は「既存の言及がいずれも『対象外』の宣言」と書く。中身は正しいが所在が未記載）。`:75`（`delete.txt` は M1 マウントでは無視可）・`:76`（NAR インストーラは M1 範囲外）・`:89`（生態系拡張として `collisionex`・NAR インストール等が M1 後）。沈黙ルール表に該当行 0 行という主張は、キーワード走査でも裏付けられた。
7. **`charset-canon` が `updates2.dau`／`install.txt`／`surfacetable.txt` の文字コードを対象外と宣言している**（`areka-P0-charset-canon/brief.md:51`）。要件 7.2 は `charset-canon` を「surfaces.txt のファイル別 `charset`」の担当として取り込めと言うが、その隣にある `descript_install` の `charset`・`descript_shell_surfacetable` の `charset` は**担当が空のまま残る**。これは誤りではなく、要件 7.5（担当未定は空のまま）の通常運用に落ちる。

---

## 4. 進め方の選択肢

成果物は文書 3 本＋ソースの doc コメントで、書く順番と機械化の度合いが選択肢になる。

### 案 A: 上流の道具を待たず、下書きを自作の使い捨てスクリプトで作る

- スナップショットから 542 件の骨組み（`[entry."<id>"]` ＋ 全欄の空値）を機械で出し、§1.4 の 3 規則で状態を先に埋め、残りを人手で通す。
- スクリプトは `crates/` の外（作業用の一時ディレクトリ）に置き、成果物の TOML だけをコミットする。
- ✅ 542 件の写し間違い・並び順・重複が構造的に起きない。✅ 上流の道具が着地したとき、上流契約 3.3a（既存項目を書き換えず不足分だけ挿入）とそのまま噛み合う。✅ 要件 10.1／10.2 の編集対象の制限を破らない（`crates/` に触れない）。
- ❌ スクリプトが成果物として残らないので、再実行するときに書き直しになる。❌ 上流の道具の抽出規則（特に版番号）と食い違う余地がある。

### 案 B: 全部を人手で書く

- 上流契約 付録 B の手順で id 一覧だけを得て、542 行を手で書く。
- ✅ 道具の癖に引きずられない。✅ 1 件ずつ本文を読むので、別名や世代の判断が丁寧になる。
- ❌ 542 件・1 件 10 行として 5,000 行超を手で書くことになり、並び順と重複の事故が現実的に避けられない。❌ 版番号の転記が一番間違えやすい。

### 案 C: 台帳を 2 段に分けて書く（骨組みは機械・判定は人手・段ごとにコミット）

- 第 1 段: 542 件の骨組み（id・空欄）を機械で出してコミットする。**この時点で「全数が入っている」ことが確定する**（要件 1.4）。
- 第 2 段: ページ単位で状態・担当・関連を埋める。埋める順は「既存の判定表がある面 → 実装ゼロが確定している面 → 残り」＝ balloon／shell／surfaces → install／update／plugin／headline → ページ全体項目。
- 第 3 段: テーマと優先度を全数に付ける（要件 8）。ここは 1 件ずつではなく群でまとめて決める（更新 14 件・装い・触れ合いなど）。
- 第 4 段: 実装済みと判定した項目にだけソースへ URL を置き、ブリーフィングを書く。
- ✅ 途中で止まっても「どこまで進んだか」が台帳の未分類件数で読める。✅ 差分が段ごとに読める。✅ 案 A の機械化を第 1 段に閉じ込められるので、道具が着地したときの再生成範囲が小さい。
- ❌ コミットが増える。❌ 第 3 段のテーマ付けを後回しにすると、第 2 段の備考を書き直したくなることがある。

**所見**: 案 C（案 A を第 1 段に内包する形）が、要件 1.4 の「ちょうど 542 個」という強い条件と、要件 9.3 の「道具が未着地なら台帳とブリーフィングだけ」という退避路の両方に素直に噛み合う。ただしこれは選択肢の提示であって決定ではない。

---

## 5. 規模と危険度

| 対象 | 規模 | 危険度 | 一言 |
|---|---|---|---|
| 台帳 542 件 | **L**（1〜2 週） | 中 | 178 件は機械で下書きでき、残りの大半は素直に未対応。時間を食うのは備考の文章と優先度 |
| ブリーフィング（未知の扱い 8 節・世代表 137 行・導線 6 段） | **M**（3〜7 日） | 中 | 材料は揃っている。世代表 137 行は台帳から機械で出せる形にすると二重管理を避けられる |
| ソースの doc コメント | **S**（1〜3 日） | 低〜中 | 置く先は 60 か所前後。`charset` の 8 ページ問題（§7-3）だけが判断を要する |
| 報告の再生成 | **S** または 0 | 低 | 上流の道具が着地していなければ要件 9.3 で不要 |
| 全体 | **L** | 中 | 実行時コードに触れないので回帰の危険は低い。危険は「台帳が正典と食い違ったまま固まる」ことに集中する |

---

## 6. 危険と研究課題

1. **スナップショットが 2.8.82 で止まっている（要研究）**。全 1,749 件の本文に `2.8.83` 以降は 0 件。`doc/COMPAT_ARCHITECTURE.md:185` は「正典が 2.8.83 で改訂した」`currentghost.balloon.scope(ID).validwidth`／`validheight`／`lines` を登記しており、**スナップショットにその改訂が入っていない**ことになる。本ドメインへの影響は 2 つ——⑴ 2.8.83 以降に追加された descript／surfaces のキーがあれば台帳から丸ごと抜ける、⑵ 既存項目の本文が改訂されていれば別名の向きの判断（要件 2.5 の第 1 段）が古い本文に基づく。**何が抜けているかはスナップショットからは分からない**。ライブの ukadoc を引いて差分を確かめるか、上流の要件 8（スナップショット更新時の差分）が着地するのを待つかの判断が要る。なお `descript_balloon` の `vertical` は 2.8.80、`wordwrappoint.y` も 2.8.80 で、縦書き周りは特に新しい面である。
2. **版番号の抽出規則が上流と食い違うと整合検査が赤になる**。上流契約 6.7 は「台帳の登場版がカタログの版番号に含まれること」を検査する。本分析では語境界の有無で 5 件（ghost 3・balloon 1・install 1）が動いた。**台帳を書く前に、どちらの規則で読むかを決めておくのが安全**。決められないなら、要件 2.7 の「版番号が無い項目は空」に倒して、境界事例では空を入れておく手もある。
3. **要件 2.9 の「44 行」「16 行」が機械で再現できない**。沈黙ルール表 80 行にはドメインを示す欄が無く、キーワードで拾うと 72 行（`balloon` が `\b[ID]` タグや `OnBalloon*` イベントの行も拾うため）、縮退の語で拾うと 32 行になる。目視で本ドメインの descript／surfaces キーに絞れば 44 前後になるのは妥当だが、**「全数読んだ」ことを後から確かめる手段が無い**。台帳の備考に転記元の行番号を書く運用にすれば、逆引きで数えられるようになる（§7-6）。
4. **名前の重なりで別ページの項目を取り違える**。`charset` は本ドメインの descript 系 8 ページ全部に、`homeurl`／`name`／`craftman`／`id` はドメインを跨いで存在する。特に `sakura.name`・`sakura.name2`・`kero.name`・`char*.name` は `descript_ghost` と `descript_shell` の**両方**にあるが、areka が読むのは**ゴースト側だけ**（`resolve.rs:69-72` はゴーストの `descript.txt` を読む経路）。同じ名前で状態が「実装済み」と「未対応」に割れるので、要件 7.6 の区別を必ず備考に書く必要がある。
5. **改行コード**。この作業ツリーのテキストファイルはすべて復帰文字付き（`doc/COMPAT_ARCHITECTURE.md` は 216 行すべて CRLF・`Cargo.toml` 107 行・`crates/areka-parsers/src/kv/parse.rs` 43 行で確認）。`.gitattributes` は存在せず、Git のシステム設定が変換を担っている。**新規に書く TOML と Markdown も同じ流儀に揃える**こと（機械生成のスクリプトを使う場合は改行の指定を明示する）。
6. **ファイル行数の上限は関係ない**。上限テストは `crates/` 配下の `.rs` だけを走査する（`crates/log-capture-kit/tests/workspace_scan/mod.rs:81` が `root.join("crates")` を起点にし `:103` が `.rs` だけを拾う。上限値は `:38`）。台帳が数千行になっても赤にならない。上流契約 9.6 の記載と一致する。
7. **`surface.append`・`kero.surface.alias` がスナップショットに全く無い（要研究）**。ページのアンカーがキー単位で作られているため、ファイルの塊構造を説明する散文が項目化されていない可能性が高い（`ukadoc:manual_shell` には `alias.txt` が旧仕様として現在は surfaces.txt に統合された旨の記述がある）。ライブのページを見れば `surface.append` の記述が実在するかどうかは確かめられる。**areka が実在しない書式を読んでいるのか、正典の写しが不完全なのか**を分けておかないと、状態を「実装済み」と書く根拠が立たない。
8. **上流の道具が本 spec の完了前に着地した場合**、要件 9.1／9.2 が発効して報告の再生成とワークスペースのテスト実行が完了条件に加わる。着地の有無で完了条件が変わるので、**どちらの前提で作業を進めるかを最初に固定しておく**とやり直しが減る。

---

## 7. 設計判断項目（要件ディスカッションへ送るもの）

> **2026-09-03 要件ディスカッションでの仕分け**:
> - **要件側で解決済み**（判断 2・4・6・9）: `bind+random` は `random` の備考＋表の注記（要件 4.3）／`*` の読み替え規則は台帳冒頭に 1 行（要件 1.3）／沈黙ルール表の転記元は行番号付き＋44 行の一覧をブリーフィングに 1 節（要件 2.8・2.9）／隣接 brief の誤りは備考にも 1 行（要件 7.4）。
> - **設計フェーズ（`/kiro-spec-design`）で解決するもの**（判断 1・3・5・7・8・10 と §6-5・§6-7）: 骨組みを使い捨てスクリプトで作るか（1）／`charset` の URL を何ページ分・どこに置くか（3）／記録の段の書き分け方（5・分類は 3 つのまま＝要件 3.3 で固定済み・段の書き方は設計）／世代表 137 行を台帳から機械で出すか（7）／道具未着地の前提を固定するか（8・要件 9.1〜9.3 が両方の枝を持つので要件は変えない）／版番号の抽出規則（10）／改行コードの流儀（§6-5）／`surface.append`・`kero.surface.alias` の正典上の実在（§6-7・§8-1）。
> - **開発者裁定**（本ディスカッションのカテゴリ C）: ⑴ `debug!` 段の記録を壊れ方の判定でどう扱うか（判断 5 の派生・要件 3.5）／⑵ ライブの ukadoc をどこまで引くか（§6-1・§8-1）。裁定の結果は要件本文へ反映する。

### 判断 1: 台帳の骨組みを機械で作るか、手で書くか

- **触れる要件**: 1.4・1.5・1.7・1.8
- **選択肢**: ⑴ 使い捨てスクリプトで 542 行の骨組みを生成し、成果物の TOML だけをコミットする。⑵ 上流契約 付録 B の手順で id 一覧を出力し、それを見ながら手で書く。⑶ 上流の道具の着地を待って初期台帳の生成に乗る。
- **推奨**: ⑴。要件 1.4 は「ちょうど 542 個」「id の集合が完全一致」という機械的な条件であり、手写しは 542 件の規模で必ず事故る。⑶ は着手条件（要件確定であって実装完了ではない）に反する。
- **補足**: ⑴ を採る場合、スクリプトは `crates/` の外に置く（要件 10.1・10.2 が `crates/` への変更を doc コメントだけに限っているため）。

### 判断 2: `bind+random` をどこに書くか

- **触れる要件**: 4.3・1.4・1.6
- **事実**: `bind+random` はスナップショット全 1,749 件で見出し 0 件・本文 0 件。`bind`（`descript_shell_surfaces:bind:1`）と `random`（同 `random,数値`）は別々に実在する。areka は `shell/decode.rs:391` で認識し `areka-seriko/src/table.rs:108-109` で駆動する。
- **選択肢**: ⑴ `bind` と `random` の 2 項目の備考に分けて書き、台帳に新しい行は作らない。⑵ ページ全体項目 `ukadoc:manual_shell` の備考にまとめて書く。⑶ 台帳には書かず、世代表とブリーフィングの本文だけに書く。
- **推奨**: ⑴ ＋ ⑶ の併用。要件 1.6 が「担当 24 ページ以外の id を書かない」と言い、要件 1.4 が件数を 542 に固定しているので、**新しい行を作る余地は無い**。`random` 側の備考に「areka は `bind+random` という組み合わせ形も駆動する（`decode.rs:391`・`table.rs:108`）。この形に対応する正典項目はスナップショットに無い」と書くのが最も追跡しやすい。

### 判断 3: `charset` の URL をどこに 1 行置くか

- **触れる要件**: 6.1・6.2・6.3・6.5
- **事実**: `charset` は本ドメインの descript 系 8 ページすべてに項目がある（`descript_ghost`・`descript_shell`・`descript_balloon`・`descript_shell_surfaces`・`descript_shell_surfacetable`・`descript_install`・`descript_plugin`・`descript_headline`）。areka の実装は 1 か所だけ（`crates/areka-parsers/src/charset/prescan.rs:54`）で、しかもファイル種別を区別しない。さらに surfaces.txt では `charset` が照合されず素通りする（`shell/decode.rs:490`）ので、`descript_shell_surfaces` の `charset` は実装済みではない。
- **選択肢**: ⑴ `prescan.rs:54` に、実装済みと判定したページ分の URL を複数行並べる（要件 6.3 の「1 項目 1 行」に従い、項目ごとに 1 行ずつ）。⑵ 上流契約 5.4 の「語彙表の先頭にページ URL 1 つ」を援用し、代表 1 ページ分だけ置く。⑶ `charset` は全ページとも「実装済み」と判定せず、経路が種別を区別しないことを備考に書いて URL を置かない。
- **推奨**: ⑴。要件 6.5 が「名前による突き合わせに頼らず id ごとの URL を書く」と明示しているので、複数行になるのは仕様どおりである。ただし何ページ分を実装済みとするか（ゴースト・シェル・バルーンは通る／surfaces は通らない／install・plugin・headline は読む経路自体が無い）の判定を先に決める必要がある。

### 判断 4: ukadoc のワイルドカードと areka の番号の読み替え規則

- **触れる要件**: 1.8・2.1・6.5・7.2
- **事実**: ukadoc は `sakura.bindgroup*.default,数値`・`element*,描画メソッド,...`・`animation*.interval,インターバル`・`collision*,...` のようにワイルドカードで書く。areka は接頭辞と接尾辞で照合し、間の数値を番号として取る（`resolve.rs:111-121`・`shell/decode.rs:197`・`:234`・`:310`）。この読み替えを 1 つ決めれば、シェル 8 形と surfaces 主要 6 形が機械で一致する。
- **選択肢**: ⑴ 「ukadoc の `*` は 1 個以上の数字に対応する」という規則を台帳の冒頭（要件 1.3 のページ一覧の隣）に 1 行で書く。⑵ 項目ごとの備考に毎回書く。⑶ ブリーフィングにだけ書く。
- **推奨**: ⑴。要件 1.8 が「id とアンカーの文字列を見た目で直さずそのまま写す」と命じているので、id はワイルドカードのまま残る。読み替えを 1 か所に書いておかないと、読む人が `*` を余計な文字だと思う。

### 判断 5: 未知の記述の扱いを「記録の段」で書き分けるか

- **触れる要件**: 3.1・3.3・3.5
- **事実**: 記録経路は段が 3 通りある。`warn!` が 2 か所（`resolve.rs:296`・`method.rs:160`）、`debug!` が 5 か所（`charset/decode.rs:35`・`:52`・`table.rs:113`・`:120`・`:130`）、まったく無言が 3 か所（`kv/parse.rs:39` の後勝ち・`shell/decode.rs:198` の overlay 以外の element・`:234-236` の `collisionex`）。要件 3.1 は「黙って捨てる／記録を残す／エラーにする」の 3 分類を求めている。
- **選択肢**: ⑴ 3 分類はそのまま使い、「記録を残す」の中で段（`warn!`／`debug!`）を副次の情報として書く。⑵ 4 分類（黙って捨てる／控えめな記録／警告／エラー）に増やす。⑶ 3 分類のまま段を書かない。
- **推奨**: ⑴。要件 10.8 が上流の凍結した仕訳の規則を変えるなと言っており、分類そのものを増やすのは避けたい。一方で要件 3.5 の「黙って壊れるか」の判定は段で結論が変わる——既定の記録の設定で `debug!` が見えないなら、利用者から見れば黙って壊れているのと変わらない。**段を備考に書き、判定の理由をそこに残すのが両立する形**である。
- **併記の必要**: 要件 3.3 が「記録経路は 1 つだけ」と書いている点は、`warn!` 限定である旨を添えないと事実と食い違う（§3-3）。

### 判断 6: 沈黙ルール表 44 行を「全数読んだ」ことをどう示すか

- **触れる要件**: 2.8・2.9
- **事実**: `doc/COMPAT_ARCHITECTURE.md:128-207` の 80 行にドメインの欄が無く、44 と 16 は機械で再現できない（§6-3）。
- **選択肢**: ⑴ 状態を `degraded` にした項目の備考に転記元の行番号（例「沈黙ルール表 `:145`」）を必ず書き、後から数えられるようにする。⑵ ブリーフィングに「本ドメインに関わる 44 行」の行番号一覧を 1 節として載せる。⑶ 数を追わず、読んだ結果だけを台帳に反映する。
- **推奨**: ⑴ ＋ ⑵。要件 2.8 は既に転記元を備考に書けと言っているので ⑴ は追加負担がほぼ無い。⑵ を足すと「44」という数の根拠が文書として残り、表が増えたときに差分で気づける。

### 判断 7: SERIKO/MAYUNA 世代表 137 行を台帳から機械で出すか、手で書くか

- **触れる要件**: 4.1・4.2・9.3
- **事実**: 要件 4.2 は「表の版番号を台帳の登場した版の欄から取り、台帳と食い違わせない」と命じるが、上流の道具が未着地なら一致を機械で検査する仕組みが無い。表は 137 行（うち版番号を持つのは 71 件）。
- **選択肢**: ⑴ 台帳から表を機械で起こす小さなスクリプトを作業用に持ち、ブリーフィングへ貼る。⑵ 手で書いて、台帳を直すたびに手で追随する。⑶ 表を「項目 id と状態」だけにして版番号を載せず、版番号は台帳を見よと書く（要件 4.1 が版を載せろと言っているので、この案は要件の改訂が要る）。
- **推奨**: ⑴。137 行の手写しは §6-2 の版番号ずれと同じ事故を招く。

### 判断 8: 上流の道具が未着地である前提を固定するか

- **触れる要件**: 9.1・9.2・9.3
- **事実**: `doc/ukadoc-coverage/` は存在せず、調査用クレートも存在しない。上流 spec は要件承認済み（`.kiro/specs/areka-P0-ukadoc-survey-toolkit/spec.json`）で実装はこれから。
- **選択肢**: ⑴ 「未着地」を前提に固定し、要件 9.3 の退避路（台帳とブリーフィングだけ・冒頭に再生成が要る旨を書く）で完了させる。⑵ 着地を待って要件 9.1／9.2 の完全形で完了させる。⑶ 完了の直前に有無を見て分岐する。
- **推奨**: ⑴。着手条件が「上流の要件確定であって実装完了ではない」（要件 Introduction・上流契約 2.1）と明記されており、⑵ は並走の前提を崩す。⑶ は成果物の形が最後まで決まらないので、ブリーフィング冒頭の文言を書き直す手戻りが出る。

### 判断 9: 隣接 spec の誤りの是正候補をどこまで拾うか

- **触れる要件**: 7.3・7.4
- **事実**: 既に 2 件が確定している。⑴ `areka-P0-balloon-canon-residue/brief.md:58` の Scope 行が「10 項目」のまま（実際は `:11-16`・`:24-27`・`:33`・`:35` の 12 項目）。⑵ `areka-P0-charset-canon/brief.md:51` が `updates2.dau`・`install.txt`・`surfacetable.txt` の文字コードを対象外と宣言しており、それらの `charset` 項目は担当が空のまま残る。さらに ⑶ `areka-P0-zorder-property/brief.md:52` が三重所有の未決を登記しており、⑷ `areka-P0-text-decoration-canon/brief.md:33` の「13 キー」は名前が列挙されていない。
- **選択肢**: ⑴ 要件 7.4 のとおりブリーフィングに是正候補として並べるだけにする。⑵ 並べたうえで、台帳の当該項目の備考にも「担当 spec の記述が古い」と書く。⑶ 拾わない。
- **推奨**: ⑵。要件 7.4 は brief を書き換えるなと言っているだけで、備考に書くことは禁じていない。台帳を読む人が担当 spec を開いたときに食い違いに気づけるようにしておく価値がある。

### 判断 10: 版番号の抽出規則を先に決めるか

- **触れる要件**: 2.7・4.2・上流契約 6.7
- **事実**: 語境界の有無で 5 件（ghost 3・balloon 1・install 1）動く（§3-4）。上流契約 6.7 は「台帳の登場版がカタログの版番号に含まれること」を検査する。
- **選択肢**: ⑴ 語境界付き（厳しい側）で読むと台帳の冒頭に宣言し、道具が着地したら照合する。⑵ 語境界なし（緩い側）＝要件本文と同じ規則で読む。⑶ 5 件の境界事例だけ `introduced` を空にしておく（要件 2.7 が「版番号が無ければ空」と言っているので形式上は通る）。
- **推奨**: ⑴ ＋ ⑶ の併用。厳しい側で読むほうが誤検出（他の数字を版番号と読む）が起きない。そのうえで、どちらとも読める 5 件だけ空にしておけば、上流の道具がどちらの規則を採っても検査が赤にならない。

---

## 8. 設計フェーズへ持ち越す研究項目

1. **ライブの ukadoc で 3 点を確かめる**——⑴ `descript_shell_surfaces` に `surface.append` の記述が実在するか、⑵ `kero.surface.alias` の正しい綴りは何か（`manual_shell` は `sakura(kero).alias` と書く）、⑶ 2.8.83 以降に本ドメインのページへ追加された項目があるか。
2. **上流の道具の版番号抽出規則**——語境界を付けるか否か。上流 spec の設計が決まるまでは判断 10 の併用案で凌げる。
3. **`text-decoration-canon` の「13 キー」の実体**——`descript_balloon` の `font.*` は実測で `font.bold`／`font.color.b`／`.g`／`.r`／`font.height`／`font.italic`／`font.name`／`font.outline`／`font.shadowcolor.b`／`.g`／`.r`／`font.shadowstyle`／`font.strike`／`font.underline` の 14 種ある。13 が何を除いた数なのかは当該 brief から読み取れない。
4. **`values.md` が未作成である間のテーマ名の正本**——`.kiro/steering/roadmap.md:148` が 8 つのテーマ名を登記しているので当面はここを引ける。道具の着地後に `doc/ukadoc-coverage/values.md` へ切り替わる。
