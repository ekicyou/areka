# 網羅ロードマップ草案

**この文書は草案である。** 段階の割り当て・束から候補 spec への案・ウェーブの並びは、いずれも
本 spec が材料として並べたものであり、正本の roadmap への反映は棚卸セッションで一括して裁定
する。この文書を読んで直接 roadmap を書き換えない。

束の名前は `linkage.md` のものをそのまま使う。段階と順位は `briefing.md` のものをそのまま使う。
この文書は同じ値を写さず、名前と欄で指す。項目を指すときは必ず引用符か逆引用符で囲む。

spec の表と M2 予約群の対応表は、それぞれ「既存 brief の位置づけ」と「M2 予約群の対応表」の節に
囲みで置いた。読み手は、囲みに見出しだけがあって行が無い表を「0 行」として扱う。

## 読み方

**段階 A〜E は M2 以降のマイルストーンの候補である。** 5 つの節は `briefing.md` 3-3 の順位表の
並びのまま、束ごとに 4 つの欄を持つ。M3「伺かの冠」の受入基準の候補は下の別の節に置いた。

- **束**: `linkage.md` が名付けた名前。単独項目は項目 id が名前である。
- **候補 spec 名の案**: この束を起票するとしたら何と呼ぶかの案で、決まった名前ではない。
  付け方は 3 つ——⑴ 構成 id の**全数**を 1 つの既存 spec が `owner` に持つ束は、その spec 名を
  そのまま案にして新しい名前を立てない、⑵ **過半**を 1 つの既存 spec が持つ束は新しい名前に
  「残余」と添え、その spec が着地した後に残る分だけを指す、⑶ それ以外は新しい名前を置く。
  ⑴ は **1 行**・⑵ は **7 行**である。
  案を置かない行が **2 行**あり、その場に理由を書いた。
  案を置いた行は **65 行**で、新しい名前は **63** である
  （⑴ の行が既存の名前を使い、`ukadoc:manual_directory` と `ukadoc:manual_ghost` の 2 行が
  同じ案を共有するため、行数より少ない）。
- **依存する既存 spec**: その束の構成 id を台帳の `owner` に持つ spec と件数である。進行中の
  spec には正本のロードマップのウェーブを、封じた spec には「完了」を添える。1 つも無い束は
  **0 本**と書く（数え方: 構成 id を台帳の `owner` で引き、空でない宛先を数えた）。0 本の束は
  **33** である（2026-09-19 に数え直した。前回の 36 から 3 減ったのは、3 つの束が同日に
  宛先を得たからである——「インストール」が `areka-P0-nar-install`、「更新」が
  `areka-P0-network-update`、「作り付けの窓」が `areka-P0-popup-menu-minimal` で、どれも
  0 本から 1 本へ変わった。数え方は 5 つの段階の表の「依存する既存 spec」の欄が
  **0 本**である行を数える形で、同じ数え方を分岐点の版に当てると 36 が返る＝較正済み。
  **この 33 は検査の外にある手書きの数である**: 分岐点から合流までの間、本流の側は
  束を 2 つ減らしておきながら散文を 1 しか減らしておらず、自分の表（34）とも食い違って
  いた。数え直す仕組みは `coverage-roadmap-refresh`（台帳 #45）が引き受ける）。
- **波の案**: 下の規則で決めた M2 の波。正本の W13〜W17 とは別の番号で、続き番号を振るのは
  棚卸セッションである。

波の案の付け方の規則。

1. M2 の波は正本のウェーブ編成（W13〜W17）の後に始まる。この文書はその編成を入力として扱い、
   並べ替えは「裁定候補」の節に提案として書くだけで、正本を書き換えない。
2. 波は段階の順に切る——第 3 波が段階 B、第 4 波が C、第 5 波が D、第 6 波が E である。
   段階 A だけは 2 つに割り、順位表で同順位が現れる手前（順位 1〜6）を第 1 波＝先頭ウェーブに、
   残り（順位 7〜18）を第 2 波にする。
3. 束は、その束の構成 id を `owner` に持つ進行中 spec のウェーブより後の波に置く。規則 1 が
   この規則を吸収するので、規則 3 が独立に効く束は **0 束**である（数え方: 67 の束と単独項目
   すべてについて `owner` の進行中 spec のウェーブを引き、W17 より後のものを数えた）。

## 既存 brief の位置づけ

**数え方**: spec の置き場の直下（完了した spec を封じる場所は数えない）にあるディレクトリのうち、
説明書のファイルを持つものを数え、そこに**本文書を作っている spec 自身のディレクトリ名**が
あれば除く。2026-09-18 に数え直した結果は **28** である。本文書を作っている spec は既に封じる
場所へ移っているので、いま除いたものは **0 個**である（2026-09-13 は 28 から自分の 1 つを除いて
27 だった）。

**`[briefs].count` が数えているのは、この数え方の結果ではなく表の行数である。** 機械が見張るのは
「冒頭の数と `[[spec]]` の行数が一致すること」だけで、置き場のディレクトリ数とは比べない（下の
⑴）。2026-09-13 にはその 2 つがたまたま同じ 27 だったが、**2026-09-19 に `areka-P0-nar-install` の
行が増え、同日さらに `areka-P0-network-update` の行が増えて 29 になった**ので、いまは別の数である。行が増えた理由は下の「2026-09-19 の追加」にある。

**この表は 2026-09-13 に撮った写真である。** 生きた spec の総数と一致し続けることは主張しない
——新しい spec が起票されても、別の spec が完了して封じられても、この表は赤にならない。機械が
見張るのは ⑴ 冒頭の数と表の行数が一致すること、⑵ 表の各 spec 名が、spec の置き場か完了した
spec を封じる場所のどちらかに実在するディレクトリ名であること、⑶ 各行の宛先の件数が台帳の
数え直しと一致すること、⑷ 各行の束の名前が帰属の文書に在ることの 4 つだけである。表の spec が
改名・削除されれば赤になる。

**行の集合と数（27）は、同じ 2026-09-13 でも別のものを見ている。** 行の集合はこの節を書いた
時点の作業用の枝から撮ったもので、その後に本流を取り込み直したことで置き場の中身が **4 本**
動いた。数（27）だけは取り込み直した後に数え直しており、**行の中身は数え直していない**。
入れ替わった 4 本は次のとおりである（数え方: **2026-09-13 時点の**表の 27 行の名前と、同日の
置き場の直下で説明書を持つディレクトリ名から本 spec 自身を除いた 27 個を、両向きに突き合わせた。
表は 2026-09-19 に 29 行になったので、いま数え直すと同じ結果にはならない）。

- 表にあるが封じる場所へ移ったもの **3 本**: `areka-P0-charset-canon`・`areka-P0-present-gpu-transform-scale`・`areka-P0-text-decoration-canon`（2026-09-13 完了。宛先に残る 17 件は実装済み 13＋語彙のみ 4 で、語彙のみの 4 件〔`sub`／`sup`／`outline` 系〕は本 spec が意図して見送った項目ゆえ `[[owner_completed]]` の条件〔全件が実装済みか縮退〕を満たさない。分割 ⑵⑶ が受け持つ 15 件は同日 `text-align-shadow-canon`〔5〕と`areka-P0-balloon-font-descript-keys`〔10〕へ付け替え済み）
- いま置き場にあるが表に無いもの **0 本**（2026-09-13 の時点では `areka-P0-nar-install` と
  `areka-P0-shell-implicit-surface` の 2 本がこちら側に居た。2026-09-19 に前者の行が、
  2026-09-20 に後者の行が表へ増えたので、いまは 0 本である。下の「2026-09-19 の追加」と
  「2026-09-20 の追加」を見よ）

行の集合はこのまま置く——この表は「着手した時点の写真」であって生きた一覧ではなく、機械が
見張る 4 つはどれも赤にならない（封じる場所へ移った 3 本はそこに実在するので ⑵ を通り、宛先の
件数も束の名前も変わっていない）。`snapshot_on` は行の集合を撮った日そのものなので
**2026-09-13 のまま**にした。日付は正しく、食い違っているのは「同じ日のどの時点の置き場か」
だけだからである。

**2026-09-19 の追加 ⑴**: `areka-P0-nar-install` の行を 1 行足して、表は **28 行**になった。
写真を撮り直したのではない。足さざるを得なかったのは、同日この spec が網羅台帳
`ledger/assets.toml` の `descript_install` の 11 項目を自分の宛先として登記したからで、機械は
**台帳の非空の宛先がすべてこの表か `briefing.md` の `[[owner_completed]]` に在ること**を見張る
（上の ⑴〜⑷ の外にある 5 つ目の見張りで、要件 7.4 ⑶ に当たる）。登記だけして表を直さないと
その場で赤になるので、台帳と同じコミットで足した。足した行の中身は
`stage = "B"`・`bundle = "インストール"`・`owner_count = 11`・`wave = "A0"` で、ウェーブは正本の
ロードマップの写しである。

**2026-09-19 の追加 ⑵**: `areka-P0-network-update` の行を 1 行足して、表は **29 行**になった。
理由は ⑴ と同じで、同日この台帳の `descript_install` の「相対パス」（delete.txt の行の書式）
1 項目の宛先をこの名前で埋めたからである。それまでこの項目の宛先は空で、備考が「引受先の
候補は……まだ起票されていない」と書いていたが、その spec は 2026-09-18 に起票されて説明書が
実在し、説明書の側もこの項目の id を delete.txt の行の形として引いていた——備考のほうが
古かったので、備考を実態へ直すのと同じ手で宛先を埋め、この行を足した。足した行の中身は
`stage = "B"`・`bundle = "更新"`・`owner_count = 1`・`wave = "A4"` で、ウェーブは正本の
ロードマップの写しである。

**2026-09-19 の追加 ⑶（合流時に数え直した）**: `areka-P0-popup-menu-minimal` の行を 1 行足した。
理由は ⑴ ⑵ と同じで、同 spec が台帳の「作り付けの窓」の束に宛先を置いたからである。
**この行は本流とは別の枝で足していたので、上の ⑴ ⑵ の「28 行」「29 行」は合流前の途中の姿
であり、いまの数ではない。** 合流した後に数え直した実数は **31 行**（`[[spec]]` の塊を数えた）
で、うち束を持つ行が **17 行**・`none = true` の行が **14 行**である。`[briefs].count` はこの 31 に
合わせ、`snapshot_on` は行の集合に最後に手を入れた日として **2026-09-19** にした（それまでの
「撮った日そのもの」という読み方は、後から足した行が 4 本に達した時点で実態と離れるため、
合流の時点で改めた）。

**2026-09-20 の追加**: `areka-P0-balloon-break` の行を 1 行足した。理由は上の ⑴ ⑵ ⑶ と同じで、
同 spec が台帳 `ledger/sakura-script.toml` の `\![enter,nouserbreakmode]`・
`\![leave,nouserbreakmode]` の 2 項目を自分の宛先として登記したからである。足した行の中身は
`stage = "A"`・`bundle = "動作モードの出入り"`・`owner_count = 2`・
`wave = "α 後"` で、段階は「動作モードの出入り」が順位表で
置かれている段階の写し、ウェーブは正本のロードマップの写しである（正本の #47 は段が「α 後」で
ウェーブ番号を持たず、開発者指示で着手した行である）。**この行は本流とは別の枝で足していたので、ここに書いていた行数は合流前の途中の姿である**
——いまの数は下の「合流」の段落が正本である。

**2026-09-20 の追加**: `areka-P0-shell-implicit-surface` の行を 1 行足した。理由は上の ⑴ ⑵ ⑶ と同じで、同日この spec が台帳 `ledger/assets.toml` の間隔の語
`sometimes`・`rarely` の 2 項目を自分の宛先として登記したからである。2 項目はどちらも
`linkage.md` の `[bundle."サーフェスアニメーション"]` に属するので、束の欄はその名前になる。
足した行の中身は `stage = "A"`・`bundle = "サーフェスアニメーション"`・`owner_count = 2`・
`wave = "A1"` で、ウェーブは正本のロードマップの写しである。
**この行も本流とは別の枝で足していたので、ここに書いていた行数は合流前の途中の姿である。**

**2026-09-20 の合流（数え直した）**: 上の 2 つの追加は別々の枝で足していたので、どちらの
「32 行」も合流前の途中の姿であり、いまの数ではない。合流した後に数え直した実数は **33 行**
（`[[spec]]` の塊を数えた）で、うち束を持つ行が **19 行**・`none = true` の行が **14 行**である
（3 つとも数え直した値で、引き算では出していない）。`[briefs].count` はこの 33 に合わせ、
`snapshot_on` は行の集合に最後に手を入れた日として **2026-09-20** のままにした。

**2026-09-24 の追加**: `areka-P0-baseware-root-layout` の行を 1 行足した。理由は上の ⑴ ⑵ ⑶ と同じで、
同日この spec が台帳 `ledger/assets.toml` の 15 項目（バルーンの `name`・`craftman`・`craftmanw`・
`id`・`type`・`readme`、シェルの `craftman`・`craftmanw`・`id`・`readme`・`menu,hidden`、ゴーストの
`name`・`craftman`・`craftmanw`・`id`）を列挙の読み手を置いて実装済みへ移し、自分の宛先として
登記したからである。15 項目は `linkage.md` の 4 つの束にまたがる（「配布物の素性」10・
「起動と挨拶」3・「名前の記憶」1・「メニュー」1）ので、束の欄にはいちばん多い「配布物の素性」を
書いた。段階ごとの表の「依存する既存 spec」の欄も、この 4 つの束の行にこの spec を件数付きで足した。
足した行の中身は `stage = "B"`・`bundle = "配布物の素性"`・`owner_count = 15`・
`wave = "A1 後段"` で、段階は「配布物の素性」が順位表で置かれている段階の写し、ウェーブは正本の
ロードマップの写しである。行数は **34 行**（`[[spec]]` の塊を数えた）で、うち束を持つ行が
**20 行**・`none = true` の行が **14 行**である。`[briefs].count` はこの 34 に合わせ、`snapshot_on` は
行の集合に最後に手を入れた日として **2026-09-24** にした。

**2026-09-27 の追加**: `areka-P0-ghost-shell-balloon-switch` の行を 1 行足した。理由は上の ⑴ ⑵ ⑶ と同じで、
同日この spec が台帳 `ledger/shiori.toml` の `OnGhostChanging`・`OnGhostChanged` と
`ledger/sakura-script.toml` の `\![change,ghost,ゴースト名(,--option=raise-event)]` の 3 項目を実装済みへ移し、
自分の宛先として登記したからである。3 項目はどれも `linkage.md` の束「切替」に属するので、束の欄には
「切替」を書いた。段階ごとの表の「依存する既存 spec」の欄も、「切替」の行にこの spec を件数付きで足した。
足した行の中身は `stage = "B"`・`bundle = "切替"`・`owner_count = 3`・`wave = "B3-①"` で、段階は
「切替」が順位表で置かれている段階の写し、ウェーブは正本のロードマップの写しである。行数は **35 行**
（`[[spec]]` の塊を数えた）で、うち束を持つ行が **21 行**・`none = true` の行が **14 行**である
（3 つとも数え直した値で、引き算では出していない）。`[briefs].count` はこの 35 に合わせ、`snapshot_on` は
行の集合に最後に手を入れた日として **2026-09-27** にした。

**2026-09-27 の 2 行目の追加**: `areka-P0-ghost-change-name-resolution` の行を 1 行足した。理由は上の ⑴ ⑵ ⑶ と
同じで、同日この spec が台帳 `ledger/sakura-script.toml` の `\+`・`\_+` の 2 項目を実装済みへ移し、自分の宛先として
登記したからである。2 項目はどちらも `linkage.md` の束「切替」に属するので、束の欄には「切替」を書いた。
段階ごとの表の「依存する既存 spec」の欄も、「切替」の行にこの spec を件数付きで足した。足した行の中身は
`stage = "B"`・`bundle = "切替"`・`owner_count = 2`・`wave = "B4-②"` で、段階は「切替」が順位表で置かれて
いる段階の写し、ウェーブは正本のロードマップの写しである。行数は **36 行**（`[[spec]]` の塊を数えた）で、
うち束を持つ行が **22 行**・`none = true` の行が **14 行**である（3 つとも数え直した値で、引き算では出して
いない）。`[briefs].count` はこの 36 に合わせ、`snapshot_on` は同じ日なので **2026-09-27** のまま変えていない。

**2026-09-29 の追加**: `areka-P0-ghost-install` の行を 1 行足した。理由は上の ⑴ ⑵ ⑶ と同じで、同日この spec が
台帳 `ledger/shiori.toml` のインストール系のイベント 8 項目（`OnInstallBegin`・`OnInstallComplete`・
`OnInstallCompleteEx`・`OnInstallCompleteAll`・`OnInstallFailure`・`OnInstallRefuse`・`OnGhostTermsAccept`・
`OnGhostTermsDecline`）と `ghostinstallbutton.caption`、`ledger/sakura-script.toml` の
`\![execute,install,path,ファイル名]`・`%lastghostname`・`%lastobjectname`、`ledger/assets.toml` のゴーストの
`install.accept` の計 13 項目を実装済みへ移し、自分の宛先として登記したからである。13 項目は `linkage.md` の
2 つの束にまたがる（「インストール」12・「メニュー」1）ので、束の欄にはいちばん多い「インストール」を書いた。
段階ごとの表の「依存する既存 spec」の欄も、この 2 つの束の行にこの spec を件数付きで足した。
`ghostinstallbutton.caption` の宛先は `areka-P0-popup-menu-minimal` から移した（同 spec の備考が「枠を登記した日に
実装済みへ改める」と引受先に名指ししていた）ので、`areka-P0-popup-menu-minimal` の行の `owner_count` は 15 から
14 へ、「メニュー」の行のその spec の件数は 13 から 12 へ変わった（どちらも台帳を数え直した値）。足した行の
中身は `stage = "B"`・`bundle = "インストール"`・`owner_count = 13`・`wave = "B5-②"` で、段階は「インストール」が
順位表で置かれている段階の写し、ウェーブは正本のロードマップの写しである。「インストール」の束の構成は
40 件で、この spec の 12 件は過半に満たないので、下の「意図しない重なり」の行の結論（裁定はまだ要る）は
変わらない。行数は **37 行**（`[[spec]]` の塊を数えた）で、うち束を持つ行が **23 行**・`none = true` の行が
**14 行**である（3 つとも数え直した値で、引き算では出していない）。`[briefs].count` はこの 37 に合わせ、
`snapshot_on` は行の集合に最後に手を入れた日として **2026-09-29** にした。

**2026-09-29 の 2 行目の追加**: `areka-P0-file-drop` の行を 1 行足した。理由は上の ⑴ ⑵ ⑶ と同じで、同日この
spec が台帳 `ledger/shiori.toml` の `OnFileDrop2`・`OnDirectoryDrop` の 2 項目を実装済みへ移し、自分の宛先として
登記したからである。2 項目はどちらも `linkage.md` の束「投げ込み」に属するので、束の欄には「投げ込み」を書いた。
段階 B の表の「投げ込み」の行は、依存する既存 spec の欄を **0 本**からこの spec の件数付きへ直し、候補 spec 名の
案の欄もこの spec を引受先として書いた。「投げ込み」の束の構成は **10 件**（`linkage.md` の `members` を数えた）で、
この spec の 2 件は ⑴（全数）にも ⑵（過半）にも当たらない。ドラッグ中の知らせ（`OnFileDropping`）と、文字・URL・
他のオブジェクトの投げ込みの残り 7 件は宛先が空のままである。足した行の中身は `stage = "B"`・
`bundle = "投げ込み"`・`owner_count = 2`・`wave = "B6"` で、段階は「投げ込み」が順位表で置かれている段階の写し、
ウェーブは正本のロードマップの写しである。行数は **38 行**（`[[spec]]` の塊を数えた）で、うち束を持つ行が
**24 行**・`none = true` の行が **14 行**である（3 つとも数え直した値で、引き算では出していない）。
`[briefs].count` はこの 38 に合わせ、`snapshot_on` は同じ日なので **2026-09-29** のまま変えていない。

**2026-10-01 の追加**: `areka-P0-shell-balloon-switch` の行を 1 行足した。理由は上の ⑴ ⑵ ⑶ と同じで、この spec が
台帳 `ledger/shiori.toml` の `OnShellChanging`・`OnShellChanged`・`OnBalloonChange` と `shellrootbutton.caption`・
`balloonrootbutton.caption`、`ledger/sakura-script.toml` の `\![change,shell,シェル名(,--option=raise-event)]`・
`\![change,balloon,バルーン名]` の計 7 項目を実装済みへ移し、自分の宛先として登記したからである。7 項目は
`linkage.md` の 2 つの束にまたがる（「切替」5・「メニュー」2）ので、束の欄にはいちばん多い「切替」を書いた。
段階ごとの表の「依存する既存 spec」の欄も、この 2 つの束の行にこの spec を件数付きで足した。2 つの
`*rootbutton.caption` の宛先は `areka-P0-popup-menu-minimal` から移した（同 spec の備考が「枠を登記した日に
実装済みへ改める」と引受先に名指ししていた）。同じ日に、2026-09-27 に「ゴースト」枠を登記したのに語彙のみの
まま残っていた `ghostrootbutton.caption` も実装済みへ移し、宛先を枠を登記した `areka-P0-ghost-shell-balloon-switch`
へ移した（台帳の兄弟 2 行と食い違わないように・要件 10.1「登記済みの枠の一覧も今の形へ」）。このため
`areka-P0-popup-menu-minimal` の行の `owner_count` は 13 から 10 へ、`areka-P0-ghost-shell-balloon-switch` の行の
`owner_count` は 3 から 4 へ変わった。「メニュー」の行は台帳を数え直して書き直した——`areka-P0-popup-menu-minimal`
の件数は 8 件で、`areka-P0-ghost-shell-balloon-switch`（1 件）を足し、2026-09-30 に
`updatebutton.caption` を引き受けた `areka-P0-network-update`（1 件）がこの行に載っていなかったのも足した。
足した行の中身は `stage = "B"`・`bundle = "切替"`・`owner_count = 7`・`wave = "B8"` で、段階は「切替」が
順位表で置かれている段階の写し、ウェーブは正本のロードマップの写しである。「切替」の束の構成は **32 件**
（`linkage.md` の `members` を数えた）で、この spec の 5 件は ⑴（全数）にも ⑵（過半）にも当たらない。
行数は **39 行**（`[[spec]]` の塊を数えた）で、うち束を持つ行が **25 行**・`none = true` の行が **14 行**である
（3 つとも数え直した値で、引き算では出していない）。`[briefs].count` はこの 39 に合わせ、`snapshot_on` は
行の集合に最後に手を入れた日として **2026-10-01** にした。
段階 B「切替」の行の候補 spec 名の案 `areka-P0-shell-balloon-switch` は、この spec と同じ綴りである。下の「候補 spec 名の案が
既存の説明書と同じ綴りになっている行」の数え直しは、検査の外にある手書きの数の棚卸として
`coverage-roadmap-refresh` に任せ、ここでは触っていない。

**2026-10-03 の追加**: `areka-P0-choice-timeout-directive` の行を 1 行足した。理由は上の ⑴ ⑵ ⑶ と同じで、この spec が
台帳 `ledger/sakura-script.toml` の `\![set,choicetimeout,時間]` の 1 項目を実装済みへ移し、自分の宛先として登記した
からである。この項目の宛先は `sakura-time-directives` から移した（この spec は時間の指令の中から選択肢の
時間切れだけを切り出したもので、正本のロードマップの台帳が「`sakura-time-directives` から切り出し」と書いている）。
このため `sakura-time-directives` の行の `owner_count` は 11 から 10 へ変わった（台帳 4 本の宛先の欄を
この名前で引いて数え直した値）。項目は `linkage.md` の束「会話」に属するので、束の欄には「会話」を書いた。段階 A の
表の「会話」の行は、台帳を数え直して `sakura-time-directives` の件数を 5 件から 4 件へ直し、この spec を
件数付きで足した。「会話」の束の構成は **46 件**（`linkage.md` の `members` を数えた）で、この spec の 1 件は
⑴（全数）にも ⑵（過半）にも当たらないので、候補 spec 名の案の欄は変えていない。`sakura-time-directives` が
持つ残りの 10 件の束の散り方（「会話」4・「窓の配置と重なり」2・「切替」2・「同期オブジェクト」1・「サウンド」1）のうち、
変わったのは「会話」だけである。足した行の中身は `stage = "A"`・`bundle = "会話"`・`owner_count = 1`・
`wave = "C1-②"` で、段階は「会話」が順位表で置かれている段階の写し、ウェーブは正本のロードマップの写しである。
行数は **40 行**（`[[spec]]` の塊を数えた）で、うち束を持つ行が **26 行**・`none = true` の行が **14 行**である
（3 つとも数え直した値で、引き算では出していない）。`[briefs].count` はこの 40 に合わせ、`snapshot_on` は
行の集合に最後に手を入れた日として **2026-10-03** にした。

**2026-10-03 の追加（2 行目）**: `sakura-time-critical` の行を 1 行足した。`areka-P0-status-execution-states` の
完了の手続き（同 spec 要件 6.3）で、台帳 `ledger/sakura-script.toml` の `\t`（`ukadoc:list_sakura_script:_5ct:1`）の
宛先をこの spec へ移したからである。この項目は 6.3 の記録で一度 `sakura-time-directives` へ送ったが、
同じ日に `\t` と `Status` の timecritical を受け持つ専用の spec が起票されたので付け替えた。このため
`sakura-time-directives` の `owner_count` は 10 のまま、`areka-P0-status-execution-states` は 4 から 3 へ
変わった。項目は束「会話」に属するので、束の欄には「会話」を書き、段階 A の表と節の「会話」の行にこの spec を
件数付きで足した（1 件は ⑴ にも ⑵ にも当たらないので、候補 spec 名の案の欄は変えていない）。足した行の中身は
`stage = "A"`・`bundle = "会話"`・`owner_count = 1`・`wave = "C4 の候補"` で、ウェーブは正本のロードマップの
写しである。行数は **41 行**で、うち束を持つ行が **27 行**・`none = true` の行が **14 行**である（3 つとも
数え直した値）。`[briefs].count` はこの 41 に合わせた。

**2026-10-05 の追加**: `areka-P0-mouse-drag-events` の行を 1 行足した。理由は上の ⑴ ⑵ ⑶ と同じで、この spec が
台帳 `ledger/shiori.toml` の `OnMouseDragStart` と `OnMouseDragEnd` の 2 項目を実装済みへ移し、自分の宛先として
登記したからである（2 項目はそれまで宛先が空だったので、宛先を移された spec は無い）。2 項目は `linkage.md` の束
「撫で」に属するので、束の欄には「撫で」を書き、段階 A の表の「撫で」の行にこの spec を件数付きで足した。「撫で」の
束の構成は **33 件**（`linkage.md` の `members` を数えた）で、この spec の 2 件は ⑴（全数）にも ⑵（過半）にも
当たらないので、候補 spec 名の案の欄は変えていない。足した行の中身は `stage = "A"`・`bundle = "撫で"`・
`owner_count = 2`・`wave = "C3-④"` で、段階は「撫で」が順位表で置かれている段階の写し、ウェーブは正本の
ロードマップの写しである。行数は **42 行**（`[[spec]]` の塊を数えた）で、うち束を持つ行が **28 行**・
`none = true` の行が **14 行**である（3 つとも数え直した値で、引き算では出していない）。`[briefs].count` はこの
42 に合わせ、`snapshot_on` は行の集合に最後に手を入れた日として **2026-10-05** にした。

**2026-10-05 の追加（2 行目）**: `areka-P0-element-base-method` の行を 1 行足した。この spec が台帳
`ledger/assets.toml` の element定義の行と `base` の行（`ukadoc:descript_shell_surfaces:base:1`）の 2 項目を
自分の宛先として登記したからである。element定義の行の宛先はそれまで `areka-P0-shell-parse` だったので、
`areka-P0-shell-parse` の件数は 4 から 3 へ変わった（`base` の行は宛先が空だった）。2 項目は `linkage.md` の束
「絵の重ね方」に属するので、束の欄には「絵の重ね方」を書き、段階 A の表の「絵の重ね方」の行で
`areka-P0-shell-parse` の 1 件を外してこの spec を件数付きで足した。「絵の重ね方」の束の構成は **75 件**
（`linkage.md` の `members` を数えた）で、この spec の 2 件は ⑴（全数）にも ⑵（過半）にも当たらないので、
候補 spec 名の案の欄は変えていない。足した行の中身は `stage = "A"`・`bundle = "絵の重ね方"`・
`owner_count = 2`・`wave = "C4 以降のバグの席"` で、段階は「絵の重ね方」が順位表で置かれている段階の写し、
ウェーブは正本のロードマップの写しである。行数は **43 行**（`[[spec]]` の塊を数えた）で、うち束を持つ行が
**29 行**・`none = true` の行が **14 行**である（3 つとも数え直した値で、引き算では出していない）。
`[briefs].count` はこの 43 に合わせ、`snapshot_on` は行の集合に最後に手を入れた日として **2026-10-05** のままにした。

**2026-10-05 の付け替え**: `areka-P0-element-base-method` の行を `draw-methods-canon` の行に替えた。
`areka-P0-element-base-method` は element定義の `base` を描けるようにし、描けない描画メソッドの行に警告を
出すようにして完了した。残りの描画メソッドと pattern定義の `base` を持つ spec として
`draw-methods-canon` を起票し、台帳 `ledger/assets.toml` の同じ 2 項目（element定義の行と
`ukadoc:descript_shell_surfaces:base:1`）の宛先をこの spec へ移したからである。2 項目の状態は変えていない。
束は同じ「絵の重ね方」で、2 件は ⑴（全数）にも ⑵（過半）にも当たらないので、候補 spec 名の案の欄は変えていない。
段階 A の表の「絵の重ね方」の行では、`areka-P0-element-base-method` の 2 件をこの spec の 2 件に替えた。
替えた行の中身は `stage = "A"`・`bundle = "絵の重ね方"`・`owner_count = 2`・
`wave = "シェルの element の列（element-base-method の後）"` である。1 行を 1 行に替えたので、行数は **43 行**・
束を持つ行が **29 行**・`none = true` の行が **14 行**のまま変わらない（3 つとも数え直した値で、引き算では
出していない）。`[briefs].count` は 43 のまま、`snapshot_on` は **2026-10-05** のままにした。

**2026-10-06 の追加**: `areka-P0-open-external-tags` の行を 1 行足した。理由は上の ⑴ ⑵ ⑶ と同じで、この spec が
台帳 `ledger/sakura-script.toml` の `\j[ID]`・`\![open,file,ファイル名]`・`\![open,browser,パラメータ]`・
`\![open,explorer,ファイル]`・`\![open,editor,ファイル,表示行]`・`\![open,mailer,パラメータ]` の 6 項目を実装済み
（`\j` と `explorer` は縮退）へ移し、自分の宛先として登記したからである（6 項目はそれまで宛先が空だったので、宛先を
移された spec は無い）。6 項目は `linkage.md` の束「外部アプリ」に属するので、束の欄には「外部アプリ」を書き、
段階 E の表の「外部アプリ」の行の「依存する既存 spec」を **0 本**からこの spec の件数付きへ替えた。「外部アプリ」の
束の構成は **72 件**（`linkage.md` の `members` を数えた）で、この spec の 6 件は ⑴（全数）にも ⑵（過半）にも
当たらないので、候補 spec 名の案の欄は変えていない。足した行の中身は `stage = "E"`・`bundle = "外部アプリ"`・
`owner_count = 6`・`wave = "C4-⑮"` で、段階は「外部アプリ」が順位表で置かれている段階の写し、ウェーブは正本の
ロードマップの写しである。行数は **44 行**（`[[spec]]` の塊を数えた）で、うち束を持つ行が **30 行**・
`none = true` の行が **14 行**である（3 つとも数え直した値で、引き算では出していない）。`[briefs].count` はこの
44 に合わせ、`snapshot_on` は行の集合に最後に手を入れた日として **2026-10-06** にした。

**2026-10-06 の追加**: `areka-P0-self-alpha-declaration` の行を 1 行足した。この spec が台帳 `ledger/assets.toml` の
シェルの `seriko.use_self_alpha` とバルーンの `use_self_alpha` の 2 項目を縮退へ直し、自分の宛先として登記したから
である。シェルの項目はそれまで宛先が空だった。バルーンの項目の宛先は `areka-P0-default-balloon-bundle` から移した
ので、同 spec の行の `owner_count` は 1 から 0 へ変わり（台帳 4 本の宛先の欄をこの名前で引いて数え直した値）、
引くものが無くなったので束も段階も決まらない。その行は `none = true` と理由を持つ形へ直した。2 項目は `linkage.md`
の束「絵の重ね方」に属するので、新しい行の束の欄には「絵の重ね方」を書き、段階 A の表の「絵の重ね方」の行は
`areka-P0-default-balloon-bundle`（1 件）をこの spec（2 件）に置き換えた。「絵の重ね方」の束の構成は **75 件**
（`linkage.md` の `members` を数えた）で、この spec の 2 件は ⑴（全数）にも ⑵（過半）にも当たらないので、候補
spec 名の案の欄は変えていない。足した行の中身は `stage = "A"`・`bundle = "絵の重ね方"`・`owner_count = 2`・
`wave = "C4-⑬"` で、段階は「絵の重ね方」が順位表で置かれている段階の写し、ウェーブは正本のロードマップの
写しである。行数は **45 行**（`[[spec]]` の塊を数えた）で、うち束を持つ行が **30 行**・`none = true` の行が
**15 行**である（3 つとも数え直した値で、引き算では出していない）。`[briefs].count` はこの 45 に合わせ、
`snapshot_on` は行の集合に最後に手を入れた日として **2026-10-06** にした。

**2026-10-06 の追加**: `areka-P0-ghost-standard-balloon` の行を 1 行足した。理由は上の ⑴ ⑵ ⑶ と同じで、この spec が
台帳 `ledger/assets.toml` の `descript_ghost` の `balloon`（実装済みへ）と `default.balloon.path`（縮退へ）の 2 項目を
自分の宛先として登記したからである（2 項目はそれまで宛先が空だったので、宛先を移された spec は無い）。2 項目は
`linkage.md` の束「descript の転記」に属するので、束の欄には「descript の転記」を書き、段階 A の表の「descript の転記」の
行にこの spec を件数付きで足した。「descript の転記」の束の構成は **15 件**（`linkage.md` の `members` を数えた）で、
この spec の 2 件は ⑴（全数）にも ⑵（過半）にも当たらないので、候補 spec 名の案の欄は変えていない。足した行の中身は
`stage = "A"`・`bundle = "descript の転記"`・`owner_count = 2`・`wave = "C4-⑪"` で、段階は「descript の転記」が
順位表で置かれている段階の写し、ウェーブは正本のロードマップの写しである。行数は **46 行**（`[[spec]]` の塊を
数えた）で、うち束を持つ行が **31 行**・`none = true` の行が **15 行**である（3 つとも数え直した値で、引き算では
出していない）。`[briefs].count` はこの 46 に合わせ、`snapshot_on` は行の集合に最後に手を入れた日として
**2026-10-07** にした。

**2026-10-07 の追加**: `areka-P0-choice-script-prefix` の行を 1 行足した。理由は上の ⑴ ⑵ ⑶ と同じで、この spec が
台帳 `ledger/sakura-script.toml` の `\q[タイトル,script:実行内容]` の 1 項目を実装済みへ移し、自分の宛先として
登記したからである（この項目はそれまで宛先が空だったので、宛先を移された spec は無い）。項目は `linkage.md` の束
「会話」に属するので、束の欄には「会話」を書き、段階 A の表の「会話」の行にこの spec を件数付きで足した。「会話」の
束の構成は **46 件**（`linkage.md` の `members` を数えた）で、この spec の 1 件は ⑴（全数）にも ⑵（過半）にも
当たらないので、候補 spec 名の案の欄は変えていない。足した行の中身は `stage = "A"`・`bundle = "会話"`・
`owner_count = 1`・`wave = "C4-⑭"` で、段階は「会話」が順位表で置かれている段階の写し、ウェーブは正本の
ロードマップの写しである。行数は **47 行**（`[[spec]]` の塊を数えた）で、うち束を持つ行が **32 行**・
`none = true` の行が **15 行**である（3 つとも数え直した値で、引き算では出していない）。`[briefs].count` はこの
47 に合わせ、`snapshot_on` は行の集合に最後に手を入れた日として **2026-10-07** にした。

**2026-10-07 の追加**: `areka-P0-seriko-trigger-intervals` の行を 1 行足した。`areka-P0-animated-image-playback` が
単独の `always` を駆動するようにし、`bind+always` のような `always` を含む組み合わせだけを駆動しないまま残したので、
台帳 `ledger/assets.toml` の `always` の項目（`ukadoc:descript_shell_surfaces:always:1`）を語彙のみから縮退へ移し、
組み合わせを引き受けるこの spec を宛先として登記したからである（それまで宛先は空だった）。この項目は `linkage.md` の
束「サーフェスアニメーション」に属するので、束の欄には「サーフェスアニメーション」を書き、段階 A の表と節の
「サーフェスアニメーション」の行にこの spec を件数付きで足した。束の構成は **47 件**（`linkage.md` の `members` を
数えた）で、この spec の 1 件は ⑴（全数）にも ⑵（過半）にも当たらないので、候補 spec 名の案の欄は変えていない。
足した行の中身は `stage = "A"`・`bundle = "サーフェスアニメーション"`・`owner_count = 1`・`wave = "C5 の候補"` で、
段階は「サーフェスアニメーション」が順位表で置かれている段階の写し、ウェーブは正本のロードマップの写しである。
行数は **48 行**（`[[spec]]` の塊を数えた）で、うち束を持つ行が **33 行**・`none = true` の行が **15 行**である
（3 つとも数え直した値で、引き算では出していない）。`[briefs].count` はこの 48 に合わせ、`snapshot_on` は
行の集合に最後に手を入れた日として **2026-10-07** にした。

**2026-10-08 の付け替え**: `areka-P0-balloon-lifecycle-events` の行を `none = true` から束を持つ行に替えた。
この spec が `OnBalloonClose`・`OnBalloonTimeout`・`OnBalloonBreak` を送るようにし、`\![set,balloontimeout,時間]` を
読むようにして、台帳 `ledger/shiori.toml` の 3 項目と `ledger/sakura-script.toml` の 1 項目を実装済み 3 件・縮退 1 件
（`OnBalloonBreak`）へ移し、宛先を分割元の `balloon-canon-residue` からこの spec へ移したからである
（是正候補の節の「表示寿命 5 件」のうち 4 件。残る `\x[noclear]` は分割元の宛先のまま）。このため
`balloon-canon-residue` の `owner_count` は 26 から 22 へ、この spec は 0 から 4 へ変わった（台帳 4 本の
宛先の欄をこの名前で引いて数え直した値）。4 項目は `linkage.md` の束「会話」に属するので、束の欄には「会話」を書き、
段階 A の表と節の「会話」の行で `balloon-canon-residue` の件数を 5 件から 1 件へ直してこの spec を件数付きで
足した。「会話」の束の構成は **46 件**で、この spec の 4 件は ⑴（全数）にも ⑵（過半）にも当たらないので、候補 spec
名の案の欄は変えていない。替えた行の中身は `stage = "A"`・`bundle = "会話"`・`owner_count = 4`・`wave = "C4-⑩"` で、
段階は「会話」が順位表で置かれている段階の写し、ウェーブは正本のロードマップの写しである。行数は **48 行**のまま
（`[[spec]]` の塊を数えた）、束を持つ行が **34 行**・`none = true` の行が **14 行**になった（3 つとも数え直した値で、
引き算では出していない）。`[briefs].count` は 48 のまま、`snapshot_on` は **2026-10-08** にした。

**2026-10-10 の付け替え**: `areka-P0-anchor-tag-canon` の行の数と束を直し、`anchor-style-canon` の行を 1 行足した。
`areka-P0-anchor-tag-canon` が `\_a` のアンカーの働き（範囲・押下・イベント）を実装して、台帳 `ledger/sakura-script.toml` の
根 2 項目（`\_a[ID,r2,r3...]`・`\_a[OnID,r0,r1...]`）と `ledger/shiori.toml` の `OnAnchorSelect`・`OnAnchorSelectEx` の
2 項目を実装済みへ移し、イベントの 2 項目を自分の宛先として登記したからである（この 2 項目はそれまで宛先が空だった）。
同時に、2026-10-04 に見た目を切り出した `anchor-style-canon` が受け持つ行——`ledger/sakura-script.toml` の
`\f[anchor*]` 16 項目と `ledger/assets.toml` の descript の `anchor.*` 43 項目——の宛先を、分割元からこの spec へ移した。
このため `areka-P0-anchor-tag-canon` の `owner_count` は 61 から 4 へ変わり、`anchor-style-canon` は 59 になった
（台帳 4 本の宛先の欄をそれぞれの名前で引いて数え直した値）。`areka-P0-anchor-tag-canon` の 4 項目は `linkage.md` の束
「会話」に 3 件・「バルーンのリンク」に 1 件なので、束の欄を「バルーンのリンク」から、いちばん多くを含む「会話」へ替えた
（段階はどちらも A）。`anchor-style-canon` の 59 項目はすべて「バルーンのリンク」に属する。段階 A の表と節の
「会話」の行で `areka-P0-anchor-tag-canon` の件数を 1 件から 3 件へ直し、「バルーンのリンク」の行の依存する既存 spec を
`anchor-style-canon`（59 件）と `areka-P0-anchor-tag-canon`（1 件）に替えた。「バルーンのリンク」の束の構成は
**60 件**（`linkage.md` の `members` を数えた）で、全数を持つ spec は無くなったが、持たれていない 1 件は実装済みなので、
未対応の 59 件の全数を持つ `anchor-style-canon` を ⑴ と同じ読みで引受先にした（⑵ の「残余」の名前を立てても、
それが指す残りが無い）。⑴ の行は **1 行**のままで、新しい名前の数も変わらない。**この読みは台帳の付け替えに合わせて
置いたもので、裁定ではない**——次の棚卸で確かめること。足した行の中身は `stage = "A"`・`bundle = "バルーンのリンク"`・
`owner_count = 59`・`wave = "文字とバルーンの列（choice-marker-styling の隣）"`、替えた行の中身は `stage = "A"`・
`bundle = "会話"`・`owner_count = 4`・`wave = "C5-①"` で、段階は束が順位表で置かれている段階の写し、ウェーブは正本の
ロードマップの写しである。行数は **49 行**（`[[spec]]` の塊を数えた）で、うち束を持つ行が **35 行**・`none = true` の
行が **14 行**である（3 つとも数え直した値で、引き算では出していない）。`[briefs].count` はこの 49 に合わせ、
`snapshot_on` は行の集合に最後に手を入れた日として **2026-10-10** にした。

**2026-10-10 の付け替えと追加（`areka-P0-seriko-trigger-intervals`・上の同日の付け替えの後）**: `areka-P0-seriko-trigger-intervals` の `owner_count` を 1 から 3 へ直し、
`seriko-interval-combinations` の行を 1 行足した。`areka-P0-seriko-trigger-intervals` が `talk,数値`・
`runonce`・`periodic,数値` を駆動するようにして、台帳 `ledger/assets.toml` の 3 項目を語彙のみから実装済みへ移し、
宛先を空からこの spec にしたからである。あわせて `always` の項目（`ukadoc:descript_shell_surfaces:always:1`）の
宛先を、`+` の組み合わせを引き受ける `seriko-interval-combinations`（同日この spec から切り出して起票した）へ
移した。`always` の状態（縮退）は変えていない。4 項目はどれも `linkage.md` の束「サーフェスアニメーション」に
属するので、段階 A の表と節の「サーフェスアニメーション」の行で `areka-P0-seriko-trigger-intervals` の件数を 1 件から
3 件へ直し、`seriko-interval-combinations` を件数付きで足した。束の構成は **47 件**で、3 件も 1 件も
⑴（全数）にも ⑵（過半）にも当たらないので、候補 spec 名の案の欄は変えていない。足した行の中身は `stage = "A"`・
`bundle = "サーフェスアニメーション"`・`owner_count = 1`・
`wave = "シェルの element の列（seriko-trigger-intervals・extent-element-offset の後）"` で、段階は
「サーフェスアニメーション」が順位表で置かれている段階の写し、ウェーブは正本のロードマップの写しである。
上の同日の付け替え（`anchor-style-canon` の行）と合わせて、行数は **50 行**（`[[spec]]` の塊を数えた）で、
うち束を持つ行が **36 行**・`none = true` の行が **14 行**である（3 つとも 2 つの枝を合わせた後に
数え直した値で、引き算では出していない。2 つの枝はどちらも自分の 1 行だけを足して「49 行」と書いていた）。
`[briefs].count` はこの 50 に合わせ、`snapshot_on` は **2026-10-10** のままにした。

**2026-10-18 の付け替えと追加（`areka-P0-move-canon`）**: `areka-P0-move-canon` の行を 1 行足し、
`sakura-time-directives` の `owner_count` を 10 から 9 へ直した。`areka-P0-move-canon` が `\![move]` の
2 つの書き方・参照点・基準を正典どおりに動かし、`\![execute,resetwindowpos]` と「シェル位置初期化」の項目
（`OnResetWindowPos`・`switchmovetodefaultpositionbutton.caption`）を実装したので、台帳の 6 項目の宛先をこの spec に
した。そのうち `\![move]` の項目（`ukadoc:list_sakura_script:_5c_21_5bmove_5d:1`）は `sakura-time-directives`
から移した（状態は縮退のまま。時間の部分が残る）。`\![set,position]` と `\![reset,position]` の 2 項目
（`ukadoc:list_sakura_script:_5c_21_5bset_2cposition_2cx_2cy_2c_30b9_30b3_30fc_30d7ID_5d:1`・
`ukadoc:list_sakura_script:_5c_21_5breset_2cposition_5d:1`）は、SSP 2.9.18 も実装していないので受けないと決めて
対象外へ移し、`linkage.md` の束「窓の配置と重なり」の構成から除いた。このため段階 A の表と節の「窓の配置と重なり」は、
構成を 126 件から 124 件へ、`sakura-time-directives` の件数を 2 件から 1 件へ直し、`areka-P0-move-canon`
（3 件）を足した。「メニュー」の行にも `areka-P0-move-canon`（1 件・`switchmovetodefaultpositionbutton.caption`）を
足した。足した行の中身は `stage = "A"`・`bundle = "窓の配置と重なり"`・`owner_count = 6`・`wave = "v0.0.3"` で、
束は 6 項目のうち 3 項目が属する束、ウェーブは正本のロードマップの版の写しである。3 件・1 件はどちらの束でも
⑴（全数）にも ⑵（過半）にも当たらないので、候補 spec 名の案の欄は変えていない。行数は **51 行**（`[[spec]]` の
塊を数えた）で、うち束を持つ行が **37 行**・`none = true` の行が **14 行**である（3 つとも数え直した値）。
`[briefs].count` はこの 51 に合わせ、`snapshot_on` は **2026-10-18** にした。

**2026-10-19 の付け替えと追加（`areka-P0-mcp-reload`）**: `areka-P0-mcp-reload` の行を 1 行足し、
`balloon-canon-residue` の `owner_count` を 22 から 21 へ直した。`areka-P0-mcp-reload` が MCP の `reload` と同じ
入口で `\![reload,ghost]`・`\![reload,shell]`・`\![reload,balloon]`・`\![reload,shiori]`・`\![reload,descript,パラメータ]`
の 5 つを読み直すようにしたので、台帳 `ledger/sakura-script.toml` の 5 項目を実装済みへ移し、宛先をこの spec にした。
そのうち `\![reload,balloon]`（`ukadoc:list_sakura_script:_5c_21_5breload_2cballoon_5d:1`）は
`balloon-canon-residue` から移した（残りの 4 項目はそれまで宛先が空だった）。5 項目は `linkage.md` の 2 つの束に
またがる（「切替」3・「開発者機能」2）ので、束の欄にはいちばん多い「切替」を書いた。段階 B の表の「切替」の行は、
`balloon-canon-residue`（1 件）を外してこの spec（3 件）を足し、段階 E の表の「開発者機能」の行は
**0 本**からこの spec の件数付き（2 件）へ替えた。「切替」の束の構成は **32 件**、「開発者機能」は **20 件**
（`linkage.md` の `members` を数えた）で、3 件・2 件はどちらも ⑴（全数）にも ⑵（過半）にも当たらないので、候補 spec
名の案の欄は変えていない。足した行の中身は `stage = "B"`・`bundle = "切替"`・`owner_count = 5`・`wave = "C9-④"` で、
段階は「切替」が順位表で置かれている段階の写し、ウェーブは正本のロードマップの写しである。行数は **52 行**
（`[[spec]]` の塊を数えた）で、うち束を持つ行が **38 行**・`none = true` の行が **14 行**である（3 つとも数え直した値）。
`[briefs].count` はこの 52 に合わせ、`snapshot_on` は **2026-10-19** にした。

**2026-10-23 の追加（`areka-P0-window-link-directives`）**: `areka-P0-window-link-directives` の行を 1 行足した。`areka-P0-window-link-directives` が
`\![set,alignmenttodesktop,方向]`（`free` と綴り `alignmentondesktop` を含む）・`\![set,sticky-window,…]`・`\![reset,sticky-window]`・
`\![execute,resetballoonpos]` と、シェルの descript の `seriko.sticky-window`・SHIORI リソース `resetballoonpositionbutton.caption` を
実装したので、台帳の 8 項目を実装済みへ移し、宛先をこの spec にした（8 項目とも、それまで宛先が空だった）。ゴーストとシェルの
descript の `seriko.alignmenttodesktop` 系の 8 項目は、未対応から縮退へ移した（起動の直後の既定の位置だけが `bottom` と
同じになる）。宛先は空のままにした。8 項目は `linkage.md` の 2 つの束にまたがる（「窓の配置と重なり」7・「メニュー」1）ので、
束の欄にはいちばん多い「窓の配置と重なり」を書いた。段階 A の表と節の「窓の配置と重なり」の行にこの spec（7 件）を足し、
構成 124 件の状態を数え直した。「メニュー」の行にもこの spec（1 件・`resetballoonpositionbutton.caption`）を足した。
「窓の配置と重なり」の束の構成は **124 件**、「メニュー」は **145 件**（`linkage.md` の `members` を数えた）で、7 件・1 件は
どちらも ⑴（全数）にも ⑵（過半）にも当たらないので、候補 spec 名の案の欄は変えていない。足した行の中身は `stage = "A"`・
`bundle = "窓の配置と重なり"`・`owner_count = 8`・`wave = "C10-②"` で、段階は「窓の配置と重なり」が順位表で置かれている
段階の写し、ウェーブは正本のロードマップの写しである。行数は **53 行**（`[[spec]]` の塊を数えた）で、うち束を持つ行が
**39 行**・`none = true` の行が **14 行**である（3 つとも数え直した値）。`[briefs].count` はこの 53 に合わせ、
`snapshot_on` は **2026-10-23** にした。

**2026-10-23 の追加（`areka-P0-script-security-level`）**: `areka-P0-script-security-level` の行を 1 行足した。この spec が
SHIORI への要求に `SecurityLevel`（出どころから `local`／`external`）・`SecurityOrigin`（今は常に `null`）・`SenderType`
を付け、応答の `SecurityLevel` を読むようにしたので、台帳 `ledger/shiori.toml` の 4 項目（要求の `SecurityLevel`・
`SecurityOrigin`・`SenderType` と応答の `SecurityLevel`）の宛先をこの spec にした（4 項目ともそれまで宛先が空だった。
`SecurityOrigin` は縮退、残りの 3 項目は実装済み）。4 項目はどれも `linkage.md` の束「SHIORI の要求と応答」に属するので、
段階 A の表の「SHIORI の要求と応答」の行にこの spec（4 件）を足した。束の構成は **25 件**（`linkage.md` の `members` を
数えた）で、4 件は ⑴（全数）にも ⑵（過半）にも当たらないので、候補 spec 名の案の欄は変えていない。足した行の中身は
`stage = "A"`・`bundle = "SHIORI の要求と応答"`・`owner_count = 4`・`wave = "C10-①"` で、段階は「SHIORI の要求と応答」が
順位表で置かれている段階の写し、ウェーブは正本のロードマップの写しである。行数は **54 行**（`[[spec]]` の塊を数えた）で、
うち束を持つ行が **40 行**・`none = true` の行が **14 行**である（3 つとも数え直した値。同じ日の `areka-P0-window-link-directives` の行を含む）。`[briefs].count` はこの 54 に
合わせ、`snapshot_on` は **2026-10-23** にした。

**2026-10-23 の追加（`areka-P0-pasta-in-windows-verification` の台帳の突き合わせ）**: 7 行を足し、段階 A と段階 B の表の
8 つの束の行の「依存する既存 spec」に、この 7 本を件数付きで足した。
`areka-P0-pasta-in-windows-verification` がぱすた（「窓際のぱすたさん」）を areka で動かし、SSP と比べて崩れた所のうち
兄弟の spec が引き取る項目の行に、引き取る spec の名前を宛先として書いたからである（同 spec の要件 9.2）。
足したのは `descript-window-keys`（2 件）・`areka-P0-window-link-directives`（1 件）・
`popup-menu-shell-look`（22 件）・`shell-mouse-cursor`（2 件）・`script-small-tags`（3 件）・
`stay-on-top-directives`（1 件）・`mouse-click-wheel-events`（3 件）で、34 項目はどれもそれまで
宛先が空だった（状態は変えていない）。7 本とも実装はまだなので行には束を割り当てず、`none = true` と理由を書いた
（`areka-P0-sylphya-set-ledger` の行と同じ書き方）。ただし、散った先の全部は各段階の節の「依存する既存 spec」の欄が
持つ決まり（下の「段階と束の決め方」）なので、束を持たない行の宛先も表には足した——`areka-P0-sylphya-set-ledger`・
`text-align-shadow-canon` が表に件数付きで載っているのと同じ扱いである。34 項目を `linkage.md` の束の
`members` で引くと、「メニュー」22（`popup-menu-shell-look`）・「窓の配置と重なり」3
（`descript-window-keys`・`areka-P0-window-link-directives`・`stay-on-top-directives` が 1 件ずつ）・
「descript の転記」1（`descript-window-keys`）・「マウスの矢印」2（`shell-mouse-cursor`）・
「選択肢の目印」「切替」「バルーンの文字」1 件ずつ（どれも `script-small-tags`）・「撫で」3
（`mouse-click-wheel-events`）で、どの束にも属さない項目は無い。段階 A の表の 7 行（「窓の配置と重なり」・
「バルーンの文字」・「descript の転記」・「マウスの矢印」・「メニュー」・「撫で」・「選択肢の目印」）と段階 B の表の
「切替」の行にこれを足し、節を持つ「窓の配置と重なり」と「バルーンの文字」は節の「**依存する既存 spec**」の行にも
同じものを足した。束の構成は「メニュー」**145 件**・「窓の配置と重なり」**124 件**・「descript の転記」**15 件**・
「マウスの矢印」**40 件**・「選択肢の目印」**40 件**・「切替」**32 件**・「バルーンの文字」**63 件**・「撫で」**33 件**
（`linkage.md` の `members` を数えた）で、足した件数はどれも ⑴（全数）にも ⑵（過半）にも当たらないので、候補 spec
名の案の欄は変えていない。「選択肢の目印」は `choice-marker-styling` が 39 件の過半を持つ ⑵ の行のままだが、
それまで宛先が空だった残りの 1 件（`\![*]`）が `script-small-tags` の宛先になったので、40 件の全部に
宛先がある。「残余」の名前が指すのはこの 1 件で、**この読みは裁定ではない**——次の棚卸で確かめること。依存する
既存 spec が **0 本**の束の数は変わらない（8 つの束はどれも前から宛先を持っていた）。ウェーブは正本のロードマップの
写しで、着手中の `areka-P0-window-link-directives` は `C10-②`、残りの 6 本は版の `v0.1.0` を書いた。各 spec は
着手したときに自分の行の束と段階を決め直してよいが、そのときは**ここにある自分の行を書き直し、同じ名前の行を
もう 1 行足さない**。並走して進んでいる `areka-P0-window-link-directives` が自分の行を新しく足すと、同じ名前の
`[[spec]]` が 2 行になり、`[briefs].count` も枝ごとに別の数へ書き換えられて合流でぶつかる。書き直すときは、
上の表の欄の件数もあわせて直すこと。行数は **59 行**（`[[spec]]` の塊を数えた）で、うち束を持つ行が **38 行**・
`none = true` の行が **21 行**である（3 つとも数え直した値）。`[briefs].count` はこの 59 に合わせ、`snapshot_on` は
**2026-10-23** にした。

**2026-10-23 の 2 回目の追加（`areka-P0-pasta-in-windows-verification` のタスク 5.3 の申し送り）**: 4 行を足し、
3 行の `owner_count` を直した。同 spec がぱすたの検証の申し送りを v0.1.0 の中で片づけるために `/kiro-discovery` で
5 本を起票し（開発者の決め・2026-10-23「0.1.0までに解決しない場合、独立して起票せよ」）、ぱすたが使う項目と走行の
測りが読めなかった項目の行の宛先を、そのうちの 4 本へ移したからである（同 spec の要件 9.2）。足したのは
`shell-scale-linkage`（7 件）・`mcp-scope-rect-property`（2 件）・`balloon-scroll-arrows`
（4 件）・`balloon-communicatebox`（4 件）で、17 項目のうち 10 項目は前の宛先から移した——
`currentghost-property-tree` から 5 件（`currentghost.scope(ID).scaling`・`currentghost.balloon.scope(ID).scaling`・
`currentghost.scope(ID).currentmonitor.dpi` は `shell-scale-linkage` へ、`currentghost.scope(ID).rect`・
`currentghost.balloon.scope(ID).rect` は `mcp-scope-rect-property` へ）・`balloon-canon-residue` から
4 件（`arrow0.x/y`・`arrow1.x/y`）・`property-query-channels` から 1 件（`\![get,property,…]`）。残る 7 項目
（`communicatebox.x/y/width/height`・`OnShellScaling`・`OnBalloonScaling`・ゴーストの descript の `balloon.syncscale`）は
それまで宛先が空だった（状態はどれも変えていない）。これに合わせて `owner_count` を
`currentghost-property-tree` は 64 から **59**、`balloon-canon-residue` は 21 から **17**、
`property-query-channels` は 6 から **5** へ直した。起票した 5 本のうち `animated-surface-log-flood`
は台帳に宛先を持たない（記録の行の量の話で、正典の項目ではない）ので行を足していない。4 本とも実装はまだなので、
行には束を割り当てず `none = true` と理由を書いた（上の 7 本と同じ書き方）。17 項目を `linkage.md` の束の `members`
で引くと、「窓の配置と重なり」6（`shell-scale-linkage` 4・`mcp-scope-rect-property` 2）・「切替」2
（`shell-scale-linkage`）・「環境の照会」1（`shell-scale-linkage`）・「バルーンの付属画像」4
（`balloon-scroll-arrows`）・「コミュニケート」4（`balloon-communicatebox`）で、どの束にも属さない項目は
無い。段階 A の表の「窓の配置と重なり」（`currentghost-property-tree` を 16 件から 11 件へ）と「バルーンの
付属画像」（`balloon-canon-residue` を 9 件から 5 件へ）・段階 B の表の「切替」・段階 C の表の「環境の照会」
（`property-query-channels` を 3 件から 2 件へ）・段階 D の表の「コミュニケート」の 5 行にこれを足し、節を持つ
「窓の配置と重なり」は節の「**依存する既存 spec**」の行も同じに直した。束の構成は「窓の配置と重なり」**124 件**・
「切替」**32 件**・「環境の照会」**28 件**・「バルーンの付属画像」**22 件**・「コミュニケート」**17 件**（`linkage.md` の
`members` を数えた）で、足した件数も減らした後の件数も ⑴（全数）にも ⑵（過半）にも当たらないので、候補 spec 名の案の
欄は変えていない。「コミュニケート」は依存する既存 spec が **0 本**から 1 本へ変わったので、5 つの段階の表で
「依存する既存 spec」が **0 本**の行は **30** から **29** になった（表の行を数え直した値）。
宛先が移ったので、上の「段階と束の決め方」の規則で 3 行の束を引き直した（数え方: 3 本それぞれの名前で台帳 4 本の
宛先を引き、束ごとの件数を数えた）。`currentghost-property-tree` は「マウスの矢印」15・「バルーンの文字」13・
「窓の配置と重なり」11 の順になったので、束を「窓の配置と重なり」から「**マウスの矢印**」へ替えた（段階は A のまま）。
`balloon-canon-residue` は「SSTP」7・「バルーンの付属画像」5 の順になったので、束を「**SSTP**」へ、段階を A から
**D** へ替えた。`property-query-channels` は「環境の照会」2 と「多重ゴースト」2 で並んだ。並んだときは順位表で
前に置かれている束を採る決まりで、「環境の照会」は段階 C、「多重ゴースト」は段階 D なので、束は「環境の照会」のまま
である。上の節が 2026-09 の写真として書いている「差が 1 件しかない行が 3 行」の 3 行のうち 2 行は差が 2 件に開き、
同数で並んだ行は `areka-P0-charset-canon` と `property-query-channels` の **2 行**になった。
**この 3 行の束と段階の読み替えは裁定ではない**——各 spec が着手したときに自分の行を決め直してよいのは上と同じで、
そのときは上の表の欄の件数もあわせて直すこと。行数は **63 行**（`[[spec]]` の塊を数えた）で、うち束を持つ行が
**38 行**・`none = true` の行が **25 行**である（3 つとも数え直した値）。`[briefs].count` はこの 63 に合わせた。
`snapshot_on` は **2026-10-23** のままである。

**2026-10-23 の合流（`areka-P0-pasta-in-windows-verification` の枝に既定ブランチを取り込んだ）**: 上の 2 つの追加（`areka-P0-window-link-directives`・`areka-P0-script-security-level`）と、その上の 2 つの追加（`areka-P0-pasta-in-windows-verification` の台帳の突き合わせとタスク 5.3 の申し送り）は別々の枝で書かれ、どちらも
`areka-P0-window-link-directives` の行を足していた——上で心配したとおり、同じ名前の `[[spec]]` が 2 行になった。
合流では既定ブランチの行（`stage = "A"`・`bundle = "窓の配置と重なり"`・`owner_count = 8`・`wave = "C10-②"`）を残し、
こちらの枝の行（`none = true`・`owner_count = 1`）を消した。こちらの枝が宛先を書いたシェルの descript の
`seriko.sticky-window` は、既定ブランチで同 spec が実装済みにした 8 項目に入っているからである（台帳 `assets.toml` の
その項目は既定ブランチの状態と備考を採った）。同じ理由で、段階 A の表と節の「窓の配置と重なり」の
`areka-P0-window-link-directives` は既定ブランチの **7 件**（こちらの枝の 1 件を含む）、「メニュー」の行は既定ブランチの 1 件と
こちらの枝の `popup-menu-shell-look`（22 件）を両方載せた。`currentghost-property-tree` の「窓の配置と重なり」は
こちらの枝の **11 件**（既定ブランチは 16 件のまま触っていない）。`areka-P0-window-link-directives` と
`areka-P0-script-security-level` は既定ブランチで完了したが、完了した spec の行を表に残して `wave` も写したままにする
のはこの表の決まり（`areka-P0-charset-canon`・`areka-P0-mcp-reload` と同じ）なので、行は消していない。
行数は **64 行**（`[[spec]]` の塊を数えた）で、うち束を持つ行が **40 行**・`none = true` の行が **24 行**である
（3 つとも数え直した値）。`[briefs].count` はこの 64 に合わせた。`snapshot_on` は **2026-10-23** のままである。


**候補 spec 名の案が既存の説明書と同じ綴りになっている行は 3 行あり、そのうち裁定が要るものは
2 行である。** 数え方: 5 つの段階の表のうち案を置いた行（行数と、そこから新しい名前の数が
どう決まるかは「読み方」にある。ここには写さない）の綴りを、いまの spec の置き場の直下に
あるディレクトリ名と突き合わせ、一致した行を数えた。一致は **3 行**である。
**2026-09-19 に 2 行から 3 行へ数え直した**——3 行目（段階 B「更新」）は、その spec が
2026-09-18 に起票された時点で既に一致していたのに、この節が数え直されなかったために
2 行のまま残っていた。表示するだけの数は必ず古びるので、宛先を触るときは必ず数え直すこと。

- **意図した重なり 1 行**: 段階 A「バルーンのリンク」の `anchor-style-canon`
  （2026-10-10 に `areka-P0-anchor-tag-canon` から替えた。理由は上の「2026-10-10 の付け替え」）。
  この行は ⑴ の規則の読みで、束の構成 id のうち未対応の全数を持つ既存 spec をそのまま引受先に
  すると欄に明記してある。「読み方」が数える新しい名前には、この行を数えていない。
- **意図しない重なり 1 行**: 段階 B「インストール」の `areka-P0-nar-install`（依存する既存 spec
  は 2026-09-19 に **0 本**から **`areka-P0-nar-install`（A0・11 件）**へ変わった）。この行は
  「読み方」が数える新しい名前のうちの 1 つ、つまり「まだ無い名前の案」として書かれているのに、
  同じ綴りの spec が既に起票されていて説明書を持つ。案を別の綴りにするか、既存の
  説明書のほうを束の引受先として扱うかを決める必要がある。
  **2026-09-19 の追記**: 同日その spec が束の構成 id のうち `install.txt` のキー 11 項目を自分の
  宛先として台帳に登記した。構成は **40 件**なので 11 件は過半に満たず、⑴（全数）にも
  ⑵（過半＋「残余」）にも当たらない。この行は ⑶（新しい名前）のままで、**重なりは解消していない**
  ——裁定はまだ要る。変わったのは「依存する既存 spec **0 本**」が事実でなくなったことだけである。
  同ページの残り 4 見出し（`bootghost` と、`delete.txt`・`developer_options.txt` の行の書式 3 件）の
  うち `delete.txt` の行の書式 1 件は 2026-09-19 に宛先を `areka-P0-network-update` で埋めた（下の
  3 行目）。残る 3 件（`bootghost` と `developer_options.txt` の行の書式 2 件）は宛先を空のまま
  残してあり、その理由は台帳の備考にある。
- **裁定が要る重なり 2 行目**: 段階 B「更新」の `areka-P0-network-update`。この spec は
  2026-09-18 に起票されて説明書を持つので、この行も「まだ無い名前の案」ではなくなっている。
  2026-09-19 にその spec が束の構成 id のうち `delete.txt` の行の書式 1 項目を自分の宛先として
  台帳に登記した。構成は **50 件**なので 1 件は過半に遠く、インストールの行と同じく ⑴（全数）にも
  ⑵（過半＋「残余」）にも当たらない。**重なりは解消していない**——裁定はまだ要る。変わったのは
  「依存する既存 spec **0 本**」が事実でなくなったことだけである。

```toml
[briefs]
count = 64
snapshot_on = "2026-10-23"
```

**段階と束の決め方**: その spec が台帳 4 本の宛先の欄に持つ id を全部引き、**いちばん多くを含む
束**を 1 つ書き、段階はその束が順位表で置かれている段階を写した。宛先が 2 つ以上の束に散る spec
は 14 本のうち **9 本**あり、散った先の全部は各段階の節の「依存する既存 spec」の欄が持っている。
いちばん多い束と 2 番目の差が **1 件**しかない行が **3 行**ある——`currentghost-property-tree`
（16 対 15）・`property-query-channels`（3 対 2）・`areka-P0-status-execution-states`
（2 対 1）。この 3 行は宛先が 1 件動くだけで束が入れ替わる。

同数で並んで決められなかった行は **1 行**ある（数え方: 14 本それぞれで束ごとの件数を降順に
並べ、先頭と 2 番目が同数の行を数えた）。`areka-P0-charset-canon` が「起動と挨拶」2 件と
「SHIORI の要求と応答」2 件で並んだ。並んだときは順位表で前に置かれている束を採る——「起動と
挨拶」は順位 4、「SHIORI の要求と応答」は順位 18 なので、表は「起動と挨拶」を書いている。
段階はどちらも A なので、この行の段階の欄はどちらを採っても変わらない。

**束の欄を持たない spec は 14 本**である（数え方: 28 行を 1 行ずつ見て `none = true` の行を
数えた。28 − 14 ＝ 14 という引き算ではなく、28 行を 1 本ずつ当たって数えた）。この 14 本は
`none = true` と理由を持ち、束の名前も**段階の欄も持たない**。段階は束が順位表で置かれている
段階の写しなので、束が決まらなければ段階も決まらない。決まらないものを既定値で埋めると値で
ない綴りが値のふりをするので、欄ごと省いた（読み手が省略を強制する。束を持つ 14 行は必ず段階を
持ち、持たない 14 行は必ず持たない）。決まらないことのほうを `reason` に書いた。

**2026-09-18 に `areka-P0-default-balloon-bundle` の行を 1 行足した。**台帳の宛先にこの名前を
書いたため、宛先の検査（`[[spec]]` にも `[[owner_completed]]` にも無い宛先を赤にする腕）が行を
要求するからである。足した後の表は **28 行**で、束を持つ行は **14 行**（上の 2026-09-13 の写真の
13 行＋この 1 行）・束を持たない `none = true` の行は **14 行**のままである——上の段落が書いている
13／14 は写真を撮った時点の数で、この 1 行はそこに含まれていない。

**この 14 本の内訳は、2026-09-18 に数え直すと 2 通りに割れる**（数え方: 14 本それぞれの名前で
台帳 4 本の宛先の欄を引き、件数を数えた）。**10 本**は宛先が **0 件**——引くものが無いので束も
決まらない。残る **4 本**は宛先を持つのに束をまだ割り当てていないもので、
`text-align-shadow-canon`（9 件）・`areka-P0-balloon-font-descript-keys`（7 件）・
`areka-P0-sylphya-set-ledger`（1 件）・`zorder-property`（1 件）である。この 4 本は
どれも 2026-09-17 以降に他の spec の完了で宛先を受け取った行で、`reason` に「束はまだ割り当てて
いない」と書いてある。節を書いた 2026-09-13 の時点では 14 本とも 0 件だったので、当時の数え方
（「0 件だったものを数えた」）はいま 10 本しか返さない。**束の欄を持たないことと宛先が 0 件で
あることは、もう同じではない。**

**ウェーブの欄**は正本のウェーブ編成をそのまま写したもので、本文書は書き換えない。`保留` は
編成のどのウェーブにも入っていない 1 本である。

```toml
[[spec]]
name = "property-catalog-lists"
stage = "C"
bundle = "一覧と汎用プロパティの照会"
owner_count = 120
wave = "W16"

[[spec]]
name = "currentghost-property-tree"
stage = "A"
bundle = "マウスの矢印"
owner_count = 59
wave = "W15"

[[spec]]
name = "anchor-style-canon"
stage = "A"
bundle = "バルーンのリンク"
owner_count = 59
wave = "文字とバルーンの列（choice-marker-styling の隣）"

[[spec]]
name = "areka-P0-anchor-tag-canon"
stage = "A"
bundle = "会話"
owner_count = 4
wave = "C5-①"

[[spec]]
name = "choice-marker-styling"
stage = "A"
bundle = "選択肢の目印"
owner_count = 39
wave = "W16"

[[spec]]
name = "areka-P0-text-decoration-canon"
stage = "A"
bundle = "バルーンの文字"
owner_count = 16
wave = "W13"

[[spec]]
name = "balloon-canon-residue"
stage = "D"
bundle = "SSTP"
owner_count = 17
wave = "W14"

[[spec]]
name = "areka-P0-popup-menu-minimal"
stage = "A"
bundle = "メニュー"
owner_count = 10
wave = "A0"

[[spec]]
name = "sakura-time-directives"
stage = "A"
bundle = "会話"
owner_count = 9
wave = "W16"

[[spec]]
name = "areka-P0-nar-install"
stage = "B"
bundle = "インストール"
owner_count = 11
wave = "A0"

[[spec]]
name = "areka-P0-network-update"
stage = "B"
bundle = "更新"
owner_count = 30
wave = "A4"

[[spec]]
name = "property-query-channels"
stage = "C"
bundle = "環境の照会"
owner_count = 5
wave = "W14"

[[spec]]
name = "areka-P0-charset-canon"
stage = "A"
bundle = "起動と挨拶"
owner_count = 5
wave = "W13"

[[spec]]
name = "makoto-dll-host"
stage = "E"
bundle = "トランスレータ"
owner_count = 4
wave = "W16"

[[spec]]
name = "areka-P0-status-execution-states"
stage = "A"
bundle = "動作モードの出入り"
owner_count = 3
wave = "W15"

[[spec]]
name = "areka-P0-balloon-break"
stage = "A"
bundle = "動作モードの出入り"
owner_count = 2
wave = "α 後"

[[spec]]
name = "areka-P0-shell-implicit-surface"
stage = "A"
bundle = "サーフェスアニメーション"
owner_count = 2
wave = "A1"

[[spec]]
name = "surfaces-basepos"
stage = "A"
bundle = "窓の配置と重なり"
owner_count = 2
wave = "W13 任意／W14"

[[spec]]
name = "areka-P0-translate-pipeline"
stage = "E"
bundle = "トランスレータ"
owner_count = 2
wave = "W15"

[[spec]]
name = "areka-P0-balloon-font-descript-keys"
none = true
reason = "束はまだ割り当てていない。分割 ⑶ で受け持つ書体の欄は 2026-09-13 に分割元 areka-P0-text-decoration-canon の完了に合わせてこの名前へ付け替えた（当時 10 件）。2026-09-17 の完了で影 4 件の宛先を text-align-shadow-canon へ移し、disable.font.* を分割元から引き取った（7 件）"
owner_count = 7
wave = "W13"

[[spec]]
name = "text-align-shadow-canon"
none = true
reason = "束はまだ割り当てていない。分割 ⑵ で受け持つ寄せ 2 と影 3 は 2026-09-13 に分割元 areka-P0-text-decoration-canon の完了に合わせてこの名前へ付け替えた（当時 5 件）。2026-09-17 に areka-P0-balloon-font-descript-keys の完了でバルーン定義の影 4 件の宛先がこの名前へ移った（9 件）"
owner_count = 9
wave = "W15"

[[spec]]
name = "areka-P0-balloon-lifecycle-events"
stage = "A"
bundle = "会話"
owner_count = 4
wave = "C4-⑩"

[[spec]]
name = "areka-P0-sylphya-set-ledger"
none = true
reason = "束はまだ割り当てていない。2026-09-17 に currentghost.seriko.sticky-window の宛先をこの名前で記入した（1 件・SET 有効群の語彙表の行）。説明書が登記すると書くサウンドの語彙は、いまも property-catalog-lists の宛先である。是正候補の節にこの spec の行がある"
owner_count = 1
wave = "W13"

[[spec]]
name = "property-ipc-transport"
none = true
reason = "台帳 4 本の宛先の欄をこの名前で引いて 0 件。説明書の範囲は輸送路の実測と 32bit 側の実装で、正典の項目に当たる行を持たない"
owner_count = 0
wave = "W14"

[[spec]]
name = "emo-text-canon-residue"
none = true
reason = "台帳 4 本の宛先の欄をこの名前で引いて 0 件。説明書の範囲は文字の折返しと描画の内側で、正典の項目に当たる行を持たない"
owner_count = 0
wave = "W14"

[[spec]]
name = "zorder-property"
none = true
reason = "束はまだ割り当てていない。説明書が正本と書く重なり順のプロパティ 1 件の宛先を、開発者裁定（2026-09-13・先送り維持）に従い 2026-09-17 にこの名前で記入した（1 件）。是正候補の節にこの spec の行がある"
owner_count = 1
wave = "W15"

[[spec]]
name = "zorder-chain-residue"
none = true
reason = "台帳 4 本の宛先の欄をこの名前で引いて 0 件"
owner_count = 0
wave = "W14"

[[spec]]
name = "areka-P0-sakura-tag-word-boundary"
none = true
reason = "台帳 4 本の宛先の欄をこの名前で引いて 0 件。さくらスクリプトの調査は、この説明書から届いた主張 11 件がいずれも所有を宣言する形をしていないと記録している"
owner_count = 0
wave = "W13"

[[spec]]
name = "areka-P0-present-gpu-transform-scale"
none = true
reason = "台帳 4 本の宛先の欄をこの名前で引いて 0 件"
owner_count = 0
wave = "W13"

[[spec]]
name = "areka-P0-kanade-boot-talkdone-drop"
none = true
reason = "台帳 4 本の宛先の欄をこの名前で引いて 0 件"
owner_count = 0
wave = "W13"

[[spec]]
name = "areka-P0-host32-window-thread-pump"
none = true
reason = "台帳 4 本の宛先の欄をこの名前で引いて 0 件"
owner_count = 0
wave = "W13"

[[spec]]
name = "dpi-transition-two-tick-bounce"
none = true
reason = "台帳 4 本の宛先の欄をこの名前で引いて 0 件"
owner_count = 0
wave = "W14"

[[spec]]
name = "tick-gate-adoption"
none = true
reason = "台帳 4 本の宛先の欄をこの名前で引いて 0 件。門の採否は性能の話で、正典の項目に当たる行を持たない"
owner_count = 0
wave = "保留"

[[spec]]
name = "areka-P0-default-balloon-bundle"
none = true
reason = "台帳 4 本の宛先の欄をこの名前で引いて 0 件。持っていた 1 件（バルーンの use_self_alpha）は 2026-10-06 に areka-P0-self-alpha-declaration へ移った"
owner_count = 0
wave = "A0"

[[spec]]
name = "areka-P0-baseware-root-layout"
stage = "B"
bundle = "配布物の素性"
owner_count = 15
wave = "A1 後段"

[[spec]]
name = "areka-P0-ghost-shell-balloon-switch"
stage = "B"
bundle = "切替"
owner_count = 4
wave = "B3-①"

[[spec]]
name = "areka-P0-ghost-change-name-resolution"
stage = "B"
bundle = "切替"
owner_count = 2
wave = "B4-②"

[[spec]]
name = "areka-P0-ghost-install"
stage = "B"
bundle = "インストール"
owner_count = 13
wave = "B5-②"

[[spec]]
name = "areka-P0-file-drop"
stage = "B"
bundle = "投げ込み"
owner_count = 2
wave = "B6"

[[spec]]
name = "areka-P0-shell-balloon-switch"
stage = "B"
bundle = "切替"
owner_count = 7
wave = "B8"

[[spec]]
name = "areka-P0-choice-timeout-directive"
stage = "A"
bundle = "会話"
owner_count = 1
wave = "C1-②"

[[spec]]
name = "sakura-time-critical"
stage = "A"
bundle = "会話"
owner_count = 1
wave = "C4 の候補"

[[spec]]
name = "areka-P0-mouse-drag-events"
stage = "A"
bundle = "撫で"
owner_count = 2
wave = "C3-④"

[[spec]]
name = "draw-methods-canon"
stage = "A"
bundle = "絵の重ね方"
owner_count = 2
wave = "シェルの element の列（element-base-method の後）"

[[spec]]
name = "areka-P0-open-external-tags"
stage = "E"
bundle = "外部アプリ"
owner_count = 6
wave = "C4-⑮"

[[spec]]
name = "areka-P0-self-alpha-declaration"
stage = "A"
bundle = "絵の重ね方"
owner_count = 2
wave = "C4-⑬"

[[spec]]
name = "areka-P0-ghost-standard-balloon"
stage = "A"
bundle = "descript の転記"
owner_count = 2
wave = "C4-⑪"

[[spec]]
name = "areka-P0-choice-script-prefix"
stage = "A"
bundle = "会話"
owner_count = 1
wave = "C4-⑭"

[[spec]]
name = "areka-P0-seriko-trigger-intervals"
stage = "A"
bundle = "サーフェスアニメーション"
owner_count = 3
wave = "C5 の候補"

[[spec]]
name = "seriko-interval-combinations"
stage = "A"
bundle = "サーフェスアニメーション"
owner_count = 1
wave = "シェルの element の列（seriko-trigger-intervals・extent-element-offset の後）"

[[spec]]
name = "areka-P0-move-canon"
stage = "A"
bundle = "窓の配置と重なり"
owner_count = 6
wave = "v0.0.3"

[[spec]]
name = "areka-P0-mcp-reload"
stage = "B"
bundle = "切替"
owner_count = 5
wave = "C9-④"

[[spec]]
name = "areka-P0-window-link-directives"
stage = "A"
bundle = "窓の配置と重なり"
owner_count = 8
wave = "C10-②"

[[spec]]
name = "areka-P0-script-security-level"
stage = "A"
bundle = "SHIORI の要求と応答"
owner_count = 4
wave = "C10-①"

[[spec]]
name = "descript-window-keys"
none = true
reason = "束はまだ割り当てていない。2026-10-23 に areka-P0-pasta-in-windows-verification がぱすたの走行の崩れの引き取り先として、ゴーストの descript の sakura.seriko.defaultsurface と balloon.dontmove の宛先をこの名前で記入した（2 件）"
owner_count = 2
wave = "v0.1.0"

[[spec]]
name = "popup-menu-shell-look"
none = true
reason = "束はまだ割り当てていない。2026-10-23 に areka-P0-pasta-in-windows-verification がぱすたの走行の崩れの引き取り先として、シェルの descript のうちぱすたが書く menu.* の 21 項目と sakura.menu の宛先をこの名前で記入した（22 件）"
owner_count = 22
wave = "v0.1.0"

[[spec]]
name = "shell-mouse-cursor"
none = true
reason = "束はまだ割り当てていない。2026-10-23 に areka-P0-pasta-in-windows-verification がぱすたの走行の崩れの引き取り先として、surfaces.txt の mouseup* と system:hand の宛先をこの名前で記入した（2 件）"
owner_count = 2
wave = "v0.1.0"

[[spec]]
name = "script-small-tags"
none = true
reason = "束はまだ割り当てていない。2026-10-23 に areka-P0-pasta-in-windows-verification がぱすたの走行の崩れの引き取り先として、さくらスクリプトの \\![*]・\\![bind-noevent]・\\_u[0x0000] の宛先をこの名前で記入した（3 件）"
owner_count = 3
wave = "v0.1.0"

[[spec]]
name = "stay-on-top-directives"
none = true
reason = "束はまだ割り当てていない。2026-10-23 に areka-P0-pasta-in-windows-verification がぱすたの走行の崩れの引き取り先として、さくらスクリプトの \\![set,windowstate,!stayontop] の宛先をこの名前で記入した（1 件）"
owner_count = 1
wave = "v0.1.0"

[[spec]]
name = "mouse-click-wheel-events"
none = true
reason = "束はまだ割り当てていない。2026-10-23 に areka-P0-pasta-in-windows-verification がぱすたの走行の崩れの引き取り先として、OnMouseClick・OnMouseEnter・OnMouseLeave の宛先をこの名前で記入した（3 件）。2026-10-24 に本 spec の実装で OnMouseUp・OnMouseUpEx・OnMouseClick・OnMouseClickEx・OnMouseWheel・OnMouseEnter・OnMouseLeave・OnMouseEnterAll・OnMouseLeaveAll を実装済み・持ち主＝本 spec に書き直した（9 件）"
owner_count = 9
wave = "v0.1.0"

[[spec]]
name = "shell-scale-linkage"
none = true
reason = "束はまだ割り当てていない。2026-10-23 に areka-P0-pasta-in-windows-verification が、ぱすたが使う倍率の連動の項目（\\![get,property]・currentghost.scope(ID).scaling・currentghost.balloon.scope(ID).scaling・currentghost.scope(ID).currentmonitor.dpi・OnShellScaling・OnBalloonScaling・ゴーストの descript の balloon.syncscale）の宛先を、v0.1.0 の中で片づけるために起票したこの名前で記入した（7 件）"
owner_count = 7
wave = "v0.1.0"

[[spec]]
name = "mcp-scope-rect-property"
none = true
reason = "束はまだ割り当てていない。2026-10-23 に areka-P0-pasta-in-windows-verification が、走行の測りが窓の位置を読めなかった崩れの引き取り先として、currentghost.scope(ID).rect と currentghost.balloon.scope(ID).rect の宛先をこの名前で記入した（2 件）"
owner_count = 2
wave = "v0.1.0"

[[spec]]
name = "balloon-scroll-arrows"
none = true
reason = "束はまだ割り当てていない。2026-10-23 に areka-P0-pasta-in-windows-verification が、ぱすたの縦書きのバルーンが使う arrow0.x/y・arrow1.x/y の宛先を、v0.1.0 の中で片づけるために起票したこの名前で記入した（4 件）"
owner_count = 4
wave = "v0.1.0"

[[spec]]
name = "balloon-communicatebox"
none = true
reason = "束はまだ割り当てていない。2026-10-23 に areka-P0-pasta-in-windows-verification が、ぱすたの縦書きのバルーンが使う communicatebox.x/y/width/height の宛先を、v0.1.0 の中で片づけるために起票したこの名前で記入した（4 件）"
owner_count = 4
wave = "v0.1.0"
```

**新しい説明書の登記先はこの文書ではない。** 起票した spec を登記するのは正本のロードマップの
spec 台帳で、この表はそれを写した写真である。

## 段階 A

節目は「そこにいて、触れて、話す」である（定義は `briefing.md` 2-1）。並びは `briefing.md` 3-3 の
順位表のままで、**25 行**ある。波は第 1 波（先頭ウェーブ）・第 2 波。

| 順位 | 束 | 候補 spec 名の案 | 依存する既存 spec | 波の案 |
| ---: | --- | --- | --- | --- |
| 1 | 会話 | `areka-P0-talk-script-canon` | `areka-P0-balloon-lifecycle-events`（C4-⑩・4 件）／`sakura-time-directives`（W16・4 件）／`balloon-canon-residue`（W14・1 件）／`areka-P0-anchor-tag-canon`（C5-①・3 件）／`sakura-time-critical`（C4 の候補・1 件）／`areka-P0-kero-balloon`（完了・2 件）／`areka-P0-cursor-tag-canon`（完了・1 件）／`areka-P0-choice-timeout-directive`（C1-②・1 件）／`areka-P0-choice-script-prefix`（C4-⑭・1 件） | 第 1 波（先頭ウェーブ） |
| 2 | 窓の配置と重なり | `areka-P0-window-placement-canon` | `currentghost-property-tree`（W15・11 件）／`surfaces-basepos`（W13 任意／W14・2 件）／`sakura-time-directives`（W16・1 件）／`areka-P0-move-canon`（v0.0.3・3 件）／`areka-P0-window-link-directives`（C10-②・7 件）／`zorder-property`（W15・1 件）／`areka-P0-scope-zorder-pinning`（完了・3 件）／`areka-P0-windowposition-limit`（完了・3 件）／`areka-P0-balloon-offset-dpi`（完了・2 件）／`areka-P0-sylphya-set-ledger`（完了・1 件）／`descript-window-keys`（v0.1.0・1 件）／`stay-on-top-directives`（v0.1.0・1 件）／`shell-scale-linkage`（v0.1.0・4 件）／`mcp-scope-rect-property`（v0.1.0・2 件） | 第 1 波（先頭ウェーブ） |
| 3 | 名前の記憶 | `areka-P0-user-name-memory` | `currentghost-property-tree`（W15・1 件）／`areka-P0-package-mount`（完了・2 件）／`areka-P0-sylphya`（完了・2 件）／`areka-P0-sakura-dialogue-tags`（完了・1 件）／`areka-P0-baseware-root-layout`（A1 後段・1 件） | 第 1 波（先頭ウェーブ） |
| 4 | 起動と挨拶 | `areka-P0-boot-greeting-canon` | `areka-P0-charset-canon`（完了・2 件）／`areka-P0-package-mount`（完了・1 件）／`areka-P0-baseware-root-layout`（A1 後段・3 件） | 第 1 波（先頭ウェーブ） |
| 5 | バルーンの文字 | `areka-P0-balloon-font-canon`（残余） | `currentghost-property-tree`（W15・13 件）／`text-align-shadow-canon`（W15・9 件）／`areka-P0-text-decoration-canon`（完了・16 件）／`areka-P0-balloon-font-descript-keys`（完了・7 件）／`areka-P0-balloon-parse`（完了・5 件）／`areka-P0-balloon-vertical-canon`（完了・4 件）／`areka-P0-cursor-tag-canon`（完了・1 件）／`script-small-tags`（v0.1.0・1 件） | 第 1 波（先頭ウェーブ） |
| 6 | サーフェスアニメーション | `areka-P0-seriko-animation-canon` | `currentghost-property-tree`（W15・6 件）／`areka-P0-shell-parse`（完了・2 件）／`areka-P0-seriko-trigger-intervals`（C5 の候補・3 件）／`seriko-interval-combinations`（シェルの element の列・1 件） | 第 1 波（先頭ウェーブ） |
| 7 | 入力窓とダイアログ | `areka-P0-inputbox-dialog` | **0 本** | 第 2 波 |
| 7 | 自発発話 | `areka-P0-idle-talk-canon` | **0 本** | 第 2 波 |
| 8 | 終了 | `areka-P0-shutdown-canon` | **0 本** | 第 2 波 |
| 9 | キーとゲームパッド | `areka-P0-key-gamepad-events` | **0 本** | 第 2 波 |
| 10 | descript の転記 | `areka-P0-descript-transcribe` | `balloon-canon-residue`（W14・4 件）／`areka-P0-package-mount`（完了・1 件）／`areka-P0-ghost-standard-balloon`（C4-⑪・2 件）／`descript-window-keys`（v0.1.0・1 件） | 第 2 波 |
| 10 | バルーンのリンク | `anchor-style-canon`（既存 spec がそのまま引受先・構成 60 件のうち未対応の 59 件の全数を `owner` に持つ。残る 1 件は `areka-P0-anchor-tag-canon` が実装済みにした） | `anchor-style-canon`（文字とバルーンの列・59 件）／`areka-P0-anchor-tag-canon`（C5-①・1 件） | 第 2 波 |
| 10 | マウスの矢印 | `areka-P0-mouse-cursor-canon` | `currentghost-property-tree`（W15・15 件）／`shell-mouse-cursor`（v0.1.0・2 件） | 第 2 波 |
| 11 | メニュー | `areka-P0-ownerdraw-menu-canon` | `areka-P0-popup-menu-minimal`（A0・8 件）／`property-catalog-lists`（W16・4 件）／`areka-P0-shell-balloon-switch`（B8・2 件）／`areka-P0-baseware-root-layout`（A1 後段・1 件）／`areka-P0-ghost-install`（B5-②・1 件）／`areka-P0-network-update`（B7・1 件）／`areka-P0-ghost-shell-balloon-switch`（B3-①・1 件）／`areka-P0-move-canon`（v0.0.3・1 件）／`areka-P0-window-link-directives`（C10-②・1 件）／`popup-menu-shell-look`（v0.1.0・22 件） | 第 2 波 |
| 12 | 撫で | `areka-P0-touch-events-canon` | `currentghost-property-tree`（W15・5 件）／`areka-P0-shell-parse`（完了・1 件）／`areka-P0-mouse-drag-events`（C3-④・2 件）／`mouse-click-wheel-events`（v0.1.0・3 件） | 第 2 波 |
| 13 | バルーンの付属画像 | `areka-P0-balloon-inline-image` | `balloon-canon-residue`（W14・5 件）／`balloon-scroll-arrows`（v0.1.0・4 件） | 第 2 波 |
| 14 | イベントの呼び起こし | `areka-P0-raise-event-tag` | `property-query-channels`（W14・1 件） | 第 2 波 |
| 14 | 選択肢の目印 | `areka-P0-choice-marker-rest`（残余） | `choice-marker-styling`（W16・39 件）／`script-small-tags`（v0.1.0・1 件） | 第 2 波 |
| 15 | 絵の重ね方 | `areka-P0-surface-composition-canon` | `draw-methods-canon`（シェルの element の列・2 件）／`areka-P0-self-alpha-declaration`（C4-⑬・2 件） | 第 2 波 |
| 16 | 動作モードの出入り | `areka-P0-passive-mode-states` | `areka-P0-status-execution-states`（W15・2 件）／`areka-P0-balloon-break`（α 後・2 件） | 第 2 波 |
| 17 | 定義ファイルの文字コード | なし（構成 2 件がどちらも実装済みで、作る仕事が残っていない） | **0 本** | 第 2 波 |
| 17 | 組み込みの置換語 | `areka-P0-builtin-substitution` | **0 本** | 第 2 波 |
| 18 | SHIORI の要求と応答 | `areka-P0-shiori-request-canon` | `areka-P0-charset-canon`（完了・2 件）／`areka-P0-status-execution-states`（W15・1 件）／`areka-P0-script-security-level`（C10-①・4 件） | 第 2 波 |
| 18 | シェル定義の転記 | `areka-P0-shell-definition-transcribe` | `areka-P0-charset-canon`（完了・1 件） | 第 2 波 |
| 18 | 同期オブジェクト | `areka-P0-sync-object-tags` | `sakura-time-directives`（W16・1 件） | 第 2 波 |

## 段階 B

節目は「迎えて、育てて、見送る」である（定義は `briefing.md` 2-1）。並びは `briefing.md` 3-3 の
順位表のままで、**12 行**ある。波は第 3 波。

| 順位 | 束 | 候補 spec 名の案 | 依存する既存 spec | 波の案 |
| ---: | --- | --- | --- | --- |
| 1 | インストール | `areka-P0-nar-install`（同じ綴りの既存 spec と重なる・裁定待ち） | `areka-P0-nar-install`（A0・11 件）／`areka-P0-ghost-install`（B5-②・12 件） | 第 3 波 |
| 1 | 更新 | `areka-P0-network-update`（同じ綴りの既存 spec と重なる・裁定待ち） | `areka-P0-network-update`（A4・1 件） | 第 3 波 |
| 2 | 切替 | `areka-P0-shell-balloon-switch` | `currentghost-property-tree`（W15・4 件）／`sakura-time-directives`（W16・2 件）／`areka-P0-kero-balloon`（完了・1 件）／`areka-P0-ghost-shell-balloon-switch`（B3-①・3 件）／`areka-P0-ghost-change-name-resolution`（B4-②・2 件）／`areka-P0-shell-balloon-switch`（B8・5 件）／`areka-P0-mcp-reload`（C9-④・3 件）／`script-small-tags`（v0.1.0・1 件）／`shell-scale-linkage`（v0.1.0・2 件） | 第 3 波 |
| 3 | 消滅 | `areka-P0-vanish-canon` | **0 本** | 第 3 波 |
| 4 | 投げ込み | `areka-P0-file-drop`（既存 spec が引受先・構成 10 件のうち 2 件） | `areka-P0-file-drop`（B6・2 件） | 第 3 波 |
| 5 | 休止と復帰 | `areka-P0-shiori-cache-suspend` | **0 本** | 第 3 波 |
| 5 | 好感度の絵柄 | `areka-P0-favorite-rate-record`（残余） | `property-catalog-lists`（W16・24 件） | 第 3 波 |
| 5 | 着せ替え | `areka-P0-dressup-bind-canon` | `areka-P0-mayuna-compose`（完了・3 件）／`areka-P0-bindoption-exclusivity`（完了・2 件） | 第 3 波 |
| 6 | `ukadoc:manual_balloon` | `areka-P0-balloon-package-layout` | **0 本** | 第 3 波 |
| 7 | 配布物の素性 | `areka-P0-package-identity` | `areka-P0-popup-menu-minimal`（A0・1 件）／`areka-P0-ghost-setup`（完了・1 件）／`areka-P0-baseware-root-layout`（A1 後段・10 件） | 第 3 波 |
| 8 | `ukadoc:manual_directory` | `areka-P0-package-directory-layout` | **0 本** | 第 3 波 |
| 8 | `ukadoc:manual_ghost` | `areka-P0-package-directory-layout` | **0 本** | 第 3 波 |

## 段階 C

節目は「察してくれる」である（定義は `briefing.md` 2-1）。並びは `briefing.md` 3-3 の
順位表のままで、**14 行**ある。波は第 4 波。

| 順位 | 束 | 候補 spec 名の案 | 依存する既存 spec | 波の案 |
| ---: | --- | --- | --- | --- |
| 1 | スクリーンセーバー | `areka-P0-screensaver-events` | **0 本** | 第 4 波 |
| 1 | バッテリー | `areka-P0-battery-events` | **0 本** | 第 4 波 |
| 2 | OS の変化の察知 | `areka-P0-os-state-events` | **0 本** | 第 4 波 |
| 2 | ディスプレイ変化 | `areka-P0-display-change-events` | **0 本** | 第 4 波 |
| 3 | 最小化 | `areka-P0-minimize-state` | **0 本** | 第 4 波 |
| 4 | 壁紙 | `areka-P0-wallpaper-events` | **0 本** | 第 4 波 |
| 4 | 通知領域 | `areka-P0-tray-presence` | **0 本** | 第 4 波 |
| 5 | スリープ復帰 | `areka-P0-sleep-resume-events` | **0 本** | 第 4 波 |
| 6 | フルスクリーン退避 | `areka-P0-fullscreen-retreat` | **0 本** | 第 4 波 |
| 7 | ごみ箱 | `areka-P0-recyclebin-query` | **0 本** | 第 4 波 |
| 8 | 一覧と汎用プロパティの照会 | `areka-P0-generic-property-query`（残余） | `property-catalog-lists`（W16・27 件）／`currentghost-property-tree`（W15・4 件）／`areka-P0-sylphya`（完了・2 件） | 第 4 波 |
| 9 | サウンド | `areka-P0-sound-playback`（残余） | `property-catalog-lists`（W16・21 件）／`sakura-time-directives`（W16・1 件） | 第 4 波 |
| 9 | 予定表 | `areka-P0-schedule-query` | **0 本** | 第 4 波 |
| 10 | 環境の照会 | `areka-P0-system-property-query`（残余） | `property-catalog-lists`（W16・25 件）／`property-query-channels`（W14・2 件）／`shell-scale-linkage`（v0.1.0・1 件） | 第 4 波 |

## 段階 D

節目は「仲間がいる」である（定義は `briefing.md` 2-1）。並びは `briefing.md` 3-3 の
順位表のままで、**7 行**ある。波は第 5 波。

| 順位 | 束 | 候補 spec 名の案 | 依存する既存 spec | 波の案 |
| ---: | --- | --- | --- | --- |
| 1 | 呼び出し | `ghost-call` | **0 本** | 第 5 波 |
| 2 | SSTP | `areka-P0-sstp-server` | `balloon-canon-residue`（W14・7 件） | 第 5 波 |
| 2 | コミュニケート | `areka-P0-communicate-events` | `balloon-communicatebox`（v0.1.0・4 件） | 第 5 波 |
| 3 | FMO | `areka-P0-fmo-share` | **0 本** | 第 5 波 |
| 4 | 多重ゴースト | `areka-P0-multi-ghost` | `property-catalog-lists`（W16・5 件）／`property-query-channels`（W14・2 件） | 第 5 波 |
| 5 | PLUGIN | `areka-P0-plugin-host` | `property-catalog-lists`（W16・8 件） | 第 5 波 |
| 6 | リンク | `areka-P0-ukagaka-link` | **0 本** | 第 5 波 |

## 段階 E

節目は「周辺」である（定義は `briefing.md` 2-1）。並びは `briefing.md` 3-3 の
順位表のままで、**9 行**ある。波は第 6 波。

| 順位 | 束 | 候補 spec 名の案 | 依存する既存 spec | 波の案 |
| ---: | --- | --- | --- | --- |
| 1 | 外部アプリ | `areka-P0-external-app-bridge` | `areka-P0-open-external-tags`（C4-⑮・6 件） | 第 6 波 |
| 2 | 開発者機能 | `areka-P0-developer-tools` | `areka-P0-mcp-reload`（C9-④・2 件） | 第 6 波 |
| 3 | ヘッドライン | `areka-P0-headline-host` | `property-catalog-lists`（W16・6 件） | 第 6 波 |
| 4 | トランスレータ | `areka-P0-translator-canon`（残余） | `makoto-dll-host`（W16・4 件）／`areka-P0-translate-pipeline`（W15・2 件） | 第 6 波 |
| 5 | 薦める場所 | `areka-P0-recommend-sites` | **0 本** | 第 6 波 |
| 6 | 作り付けの窓 | `areka-P0-baseware-windows` | `areka-P0-popup-menu-minimal`（A0・1 件） | 第 6 波 |
| 6 | 読み上げと聞き取り | `areka-P0-voice-io` | **0 本** | 第 6 波 |
| 7 | 書庫 | `areka-P0-archive-io` | **0 本** | 第 6 波 |
| 8 | `ukadoc:memo` | なし（ベースウェアの機能比較表と雑多な覚え書きのページで、作る振る舞いを指していない） | **0 本** | 第 6 波 |

## 先頭ウェーブ

先頭ウェーブ（M2 の第 1 波）に入れる束は **6** である——段階 A の順位 1〜6 で、`briefing.md`
3-3 の段階 A の順位表で同順位が現れる手前までを採った。順位 7 で初めて 2 つの束が同じ順位に
並ぶので、そこから先を入れるには同順位の解消（下の「裁定候補」）が要る。この 6 束には
`briefing.md` 3-4 の順序の印（`override`）が **0 行**、2-5 の段階の裁定候補に挙がった束が
**0 束**含まれる（数え方: 3-4 の 2 行と 2-5 の 10 件が名指す束の名前を、この 6 つと突き合わせた）。

6 束の構成 id は合わせて **324 件**である。うち **84 件**を進行中の spec が、
**31 件**を封じた spec が `owner` に持つ（数え方: 6 束の `members` を台帳の `owner` で
引き、宛先が正本の spec 台帳にある名前か `completed/` の名前かで分けて数えた）。この 3 つの
数はこの節にだけ置く。ほかの節で触れるときは値を写さず、この節を指す。

**この 3 つの数と、下に並ぶ状態の数は、いずれも 2026-09-13 に台帳 4 本を引いて数えた写真で
ある**（状態の数え方: 束の構成 id を台帳の `status` で引いて状態ごとに数えた）。生きた値の
正本は台帳であり、**この節の数はどれも判定が数え直さない**。宛先が 1 件動いても、spec が
1 本封じられても赤にならないので、台帳を触った人が手で直す。

この節はそのまま `/kiro-discovery` 再入の入力になる。束ごとに、候補 spec 名の案・3 行の要約
（問題・現状・何が変わるか）・依存する既存 spec・構成 id の全列挙を持たせた。

### 会話（段階 A・順位 1）

**候補 spec 名の案**: `areka-P0-talk-script-canon`

**3 行の要約**

- 問題: 台本を読み上げてバルーンへ文字を送る中核でありながら、送りと待ちと改行と選択の待ちに関わる正典の語彙の半分を超える分が未対応か語彙だけで、雛形が書く綴りに当たると黙って落ちる。
- 現状: 構成 46 件の状態は実装済み 24・未対応 15・語彙のみ 5・縮退 2（2026-10-10 に台帳から数え直した）で、`briefing.md` 7-7 が数えた「一般化で壊れる」75 件のうち 14 件がこの束にある。
- 何が変わるか: 里々製・ヤヤ製の雛形が書く会話の綴りが素通りせずに再生され、バルーンの寿命と選択の待ちが台本の指定で決まるようになる。

**依存する既存 spec**: `sakura-time-directives`（W16・4 件）／`areka-P0-balloon-lifecycle-events`（C4-⑩・4 件）／`balloon-canon-residue`（W14・1 件）／`areka-P0-anchor-tag-canon`（C5-①・3 件）／`sakura-time-critical`（C4 の候補・1 件）／`areka-P0-kero-balloon`（完了・2 件）／`areka-P0-cursor-tag-canon`（完了・1 件）／`areka-P0-choice-timeout-directive`（完了・1 件）

**構成 id（全 46 件）**

- `ukadoc:list_sakura_script:_5c0_3082_3057_304f_306f_5ch:1`
- `ukadoc:list_sakura_script:_5c1_3082_3057_304f_306f_5cu:1`
- `ukadoc:list_sakura_script:_5cC:1`
- `ukadoc:list_sakura_script:_5c_21_5bquicksection_2cfalse_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bquicksection_2ctrue_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2cautoscroll_2cdisable_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2cautoscroll_2cenable_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2cballoontimeout_2c_6642_9593_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2cballoonwait_2c_500d_7387_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2cchoicetimeout_2c_6642_9593_5d:1`
- `ukadoc:list_sakura_script:_5c_2a:1`
- `ukadoc:list_sakura_script:_5c__21:1`
- `ukadoc:list_sakura_script:_5c__3f:1`
- `ukadoc:list_sakura_script:_5c__q_5bID_2c..._5d:1`
- `ukadoc:list_sakura_script:_5c__w_5b_6642_9593_5d:1`
- `ukadoc:list_sakura_script:_5c_a_5bID_2cr2_2cr3..._5d:1`
- `ukadoc:list_sakura_script:_5c_q:1`
- `ukadoc:list_sakura_script:_5c_s_5bID1_2cID2_2cID3..._5d:1`
- `ukadoc:list_sakura_script:_5c_w_5b_6642_9593_5d:1`
- `ukadoc:list_sakura_script:_5cc:1`
- `ukadoc:list_sakura_script:_5ce:1`
- `ukadoc:list_sakura_script:_5cn:1`
- `ukadoc:list_sakura_script:_5cn_5b_30d1_30fc_30bb_30f3_30c8_5d:1`
- `ukadoc:list_sakura_script:_5cn_5bhalf_5d:1`
- `ukadoc:list_sakura_script:_5cp_5bID_756a_53f7_5d:1`
- `ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cID1_2cID2_2cID3..._5d:1`
- `ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cID_2cr2_2cr3..._5d:1`
- `ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cOnID_2cr0_2cr1_2c..._5d:1`
- `ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cscript_3a_5b9f_884c_5185_5bb9_5d:1`
- `ukadoc:list_sakura_script:_5cs_5bID_756a_53f7_5d:1`
- `ukadoc:list_sakura_script:_5ct:1`
- `ukadoc:list_sakura_script:_5cw_6642_9593:1`
- `ukadoc:list_sakura_script:_5cx_5bnoclear_5d:1`
- `ukadoc:list_shiori_event:OnAnchorEnter:1`
- `ukadoc:list_shiori_event:OnAnchorHover:1`
- `ukadoc:list_shiori_event:OnAnchorSelect:1`
- `ukadoc:list_shiori_event:OnAnchorSelectEx:1`
- `ukadoc:list_shiori_event:OnBalloonBreak:1`
- `ukadoc:list_shiori_event:OnBalloonClose:1`
- `ukadoc:list_shiori_event:OnBalloonTimeout:1`
- `ukadoc:list_shiori_event:OnChoiceEnter:1`
- `ukadoc:list_shiori_event:OnChoiceHover:1`
- `ukadoc:list_shiori_event:OnChoiceSelect:1`
- `ukadoc:list_shiori_event:OnChoiceSelectEx:1`
- `ukadoc:list_shiori_event:OnChoiceTimeout:1`
- `ukadoc:list_shiori_resource:balloon_tooltip:1`

### 窓の配置と重なり（段階 A・順位 2）

**候補 spec 名の案**: `areka-P0-window-placement-canon`

**3 行の要約**

- 問題: 立ち位置・相方との隣接・バルーンの貼り付き・重なり順が 1 つの束で決まるのに、構成の 3 分の 2 が未対応で、作者が定義ファイルに書いた位置の指定がほとんど効かない。
- 現状: 構成 124 件の状態は実装済み 16・未対応 66・語彙のみ 29・縮退 13 で、壊れる 75 件のうち 7 件がここにある。M1 の実機一周はこの束を 8 項目（項目 1・9・10・12・16・17・18・19）で見て全部合格しているが、それは emo2 の 1 体で使う範囲である。
- 何が変わるか: 作者が書いた原点と余白と重なりの指定どおりに二体とバルーンが並び、拡大率を変えても隣接が崩れなくなる。

**依存する既存 spec**: `currentghost-property-tree`（W15・11 件）／`surfaces-basepos`（W13 任意／W14・2 件）／`sakura-time-directives`（W16・1 件）／`areka-P0-move-canon`（v0.0.3・3 件）／`areka-P0-window-link-directives`（C10-②・7 件）／`zorder-property`（W15・1 件）／`areka-P0-scope-zorder-pinning`（完了・3 件）／`areka-P0-windowposition-limit`（完了・3 件）／`areka-P0-balloon-offset-dpi`（完了・2 件）／`areka-P0-sylphya-set-ledger`（完了・1 件）／`descript-window-keys`（v0.1.0・1 件）／`stay-on-top-directives`（v0.1.0・1 件）／`shell-scale-linkage`（v0.1.0・4 件）／`mcp-scope-rect-property`（v0.1.0・2 件）

**構成 id（全 124 件）**

- `ukadoc:descript_balloon:dpi_2c_63a8_5968DPI:1`
- `ukadoc:descript_balloon:windowposition.limit_2c0_2f1:1`
- `ukadoc:descript_balloon:windowposition.x_2c_5ea7_6a19:1`
- `ukadoc:descript_balloon:windowposition.y_2c_5ea7_6a19:1`
- `ukadoc:descript_ghost:balloon.dontmove_2ctrue:1`
- `ukadoc:descript_ghost:balloon.syncscale_2ctrue:1`
- `ukadoc:descript_ghost:char_2a.defaultleft_2cX_5ea7_6a19:1`
- `ukadoc:descript_ghost:char_2a.defaulttop_2cY_5ea7_6a19:1`
- `ukadoc:descript_ghost:char_2a.defaultx_2cX_5ea7_6a19:1`
- `ukadoc:descript_ghost:char_2a.defaulty_2cY_5ea7_6a19:1`
- `ukadoc:descript_ghost:char_2a.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1`
- `ukadoc:descript_ghost:kero.defaultleft_2cX_5ea7_6a19:1`
- `ukadoc:descript_ghost:kero.defaulttop_2cY_5ea7_6a19:1`
- `ukadoc:descript_ghost:kero.defaultx_2cX_5ea7_6a19:1`
- `ukadoc:descript_ghost:kero.defaulty_2cY_5ea7_6a19:1`
- `ukadoc:descript_ghost:kero.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1`
- `ukadoc:descript_ghost:sakura.defaultleft_2cX_5ea7_6a19:1`
- `ukadoc:descript_ghost:sakura.defaulttop_2cY_5ea7_6a19:1`
- `ukadoc:descript_ghost:sakura.defaultx_2cX_5ea7_6a19:1`
- `ukadoc:descript_ghost:sakura.defaulty_2cY_5ea7_6a19:1`
- `ukadoc:descript_ghost:sakura.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1`
- `ukadoc:descript_ghost:seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1`
- `ukadoc:descript_shell:char_2a.balloon.dontmove_2c_6570_5024:1`
- `ukadoc:descript_shell:char_2a.balloon.syncscale_2ctrue:1`
- `ukadoc:descript_shell:char_2a.defaultleft_2cX_5ea7_6a19:1`
- `ukadoc:descript_shell:char_2a.defaulttop_2cY_5ea7_6a19:1`
- `ukadoc:descript_shell:char_2a.defaultx_2cX_5ea7_6a19:1`
- `ukadoc:descript_shell:char_2a.defaulty_2cY_5ea7_6a19:1`
- `ukadoc:descript_shell:char_2a.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1`
- `ukadoc:descript_shell:kero.balloon.alignment_2c_4f4d_7f6e_60c5_5831:1`
- `ukadoc:descript_shell:kero.balloon.dontmove_2c_6570_5024:1`
- `ukadoc:descript_shell:kero.balloon.offsetx_2c_5ea7_6a19:1`
- `ukadoc:descript_shell:kero.balloon.offsety_2c_5ea7_6a19:1`
- `ukadoc:descript_shell:kero.balloon.syncscale_2ctrue:1`
- `ukadoc:descript_shell:kero.defaultleft_2cX_5ea7_6a19:1`
- `ukadoc:descript_shell:kero.defaulttop_2cY_5ea7_6a19:1`
- `ukadoc:descript_shell:kero.defaultx_2cX_5ea7_6a19:1`
- `ukadoc:descript_shell:kero.defaulty_2cY_5ea7_6a19:1`
- `ukadoc:descript_shell:kero.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1`
- `ukadoc:descript_shell:sakura.balloon.alignment_2c_4f4d_7f6e_60c5_5831:1`
- `ukadoc:descript_shell:sakura.balloon.dontmove_2c_6570_5024:1`
- `ukadoc:descript_shell:sakura.balloon.offsetx_2c_5ea7_6a19:1`
- `ukadoc:descript_shell:sakura.balloon.offsety_2c_5ea7_6a19:1`
- `ukadoc:descript_shell:sakura.balloon.syncscale_2ctrue:1`
- `ukadoc:descript_shell:sakura.defaultleft_2cX_5ea7_6a19:1`
- `ukadoc:descript_shell:sakura.defaulttop_2cY_5ea7_6a19:1`
- `ukadoc:descript_shell:sakura.defaultx_2cX_5ea7_6a19:1`
- `ukadoc:descript_shell:sakura.defaulty_2cY_5ea7_6a19:1`
- `ukadoc:descript_shell:sakura.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1`
- `ukadoc:descript_shell:seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1`
- `ukadoc:descript_shell:seriko.dpi_2c_63a8_5968DPI:1`
- `ukadoc:descript_shell:seriko.sticky-window_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c...:1`
- `ukadoc:descript_shell:seriko.zorder_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c...:1`
- `ukadoc:descript_shell_surfaces:balloon.offsetx_2c_5ea7_6a19:1`
- `ukadoc:descript_shell_surfaces:balloon.offsety_2c_5ea7_6a19:1`
- `ukadoc:descript_shell_surfaces:kero.balloon.offsetx_2c_5ea7_6a19:1`
- `ukadoc:descript_shell_surfaces:kero.balloon.offsety_2c_5ea7_6a19:1`
- `ukadoc:descript_shell_surfaces:point.basepos.x_2c_5ea7_6a19:1`
- `ukadoc:descript_shell_surfaces:point.basepos.y_2c_5ea7_6a19:1`
- `ukadoc:descript_shell_surfaces:point.centerx_2c_5ea7_6a19:1`
- `ukadoc:descript_shell_surfaces:point.centery_2c_5ea7_6a19:1`
- `ukadoc:descript_shell_surfaces:point.kinoko.centerx_2c_5ea7_6a19:1`
- `ukadoc:descript_shell_surfaces:point.kinoko.centery_2c_5ea7_6a19:1`
- `ukadoc:descript_shell_surfaces:sakura.balloon.offsetx_2c_5ea7_6a19:1`
- `ukadoc:descript_shell_surfaces:sakura.balloon.offsety_2c_5ea7_6a19:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.rect:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.scaling:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.x:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.y:1`
- `ukadoc:list_propertysystem:currentghost.scope.count:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.currentmonitor.bpp:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.currentmonitor.dpi:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.currentmonitor.primary:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.currentmonitor.rect:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.currentmonitor.work:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.rect:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.scaling:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.surface.x:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.surface.y:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.x:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.y:1`
- `ukadoc:list_propertysystem:currentghost.seriko.sticky-window:1`
- `ukadoc:list_propertysystem:currentghost.seriko.zorder:1`
- `ukadoc:list_sakura_script:_5c4:1`
- `ukadoc:list_sakura_script:_5c5:1`
- `ukadoc:list_sakura_script:_5c_21_5bexecute_2cresetballoonpos_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bexecute_2cresetwindowpos_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5block_2cballoonmove_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5block_2cballoonrepaint_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5block_2crepaint_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bmove_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bmoveasync_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5breset_2csticky-window_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5breset_2czorder_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2calignmentondesktop_2cbottom_307e_305f_306ftop_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2calignmenttodesktop_2c_65b9_5411_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2calignmenttodesktop_2cfree_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2calpha_2c_6570_5024_2c_30aa_30d7_30b7_30e7_30f3_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2cballoonalign_2cID_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2cballoonoffset_2cx_2cy_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2csticky-window_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c..._5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2cwindowstate_2c_21stayontop_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2cwindowstate_2cstayontop_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2czorder_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c..._5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bunlock_2cballoonmove_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bunlock_2cballoonrepaint_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bunlock_2crepaint_5d:1`
- `ukadoc:list_sakura_script:_5cv:1`
- `ukadoc:list_shiori_event:OnOffscreen:1`
- `ukadoc:list_shiori_event:OnOverlap:1`
- `ukadoc:list_shiori_event:OnResetWindowPos:1`
- `ukadoc:list_shiori_event:hwnd:1`
- `ukadoc:list_shiori_resource:char_2a.defaultleft:1`
- `ukadoc:list_shiori_resource:char_2a.defaulttop:1`
- `ukadoc:list_shiori_resource:char_2a.defaultx:1`
- `ukadoc:list_shiori_resource:char_2a.defaulty:1`
- `ukadoc:list_shiori_resource:kero.defaultleft:1`
- `ukadoc:list_shiori_resource:kero.defaulttop:1`
- `ukadoc:list_shiori_resource:kero.defaultx:1`
- `ukadoc:list_shiori_resource:kero.defaulty:1`
- `ukadoc:list_shiori_resource:sakura.defaultleft:1`
- `ukadoc:list_shiori_resource:sakura.defaulttop:1`
- `ukadoc:list_shiori_resource:sakura.defaultx:1`
- `ukadoc:list_shiori_resource:sakura.defaulty:1`

### 名前の記憶（段階 A・順位 3）

**候補 spec 名の案**: `areka-P0-user-name-memory`

**3 行の要約**

- 問題: 名前を尋ねて覚え、次から呼びかける——伺かの象徴的な振る舞いの口が、保存の側と読み戻しの側の両方で欠けている。
- 現状: 構成 22 件の状態は実装済み 9・未対応 11・語彙のみ 2 で、壊れる 75 件のうち 2 件がここにある（`ukadoc:list_shiori_event:OnNotifyUserInfo:1` と `ukadoc:descript_ghost:name_2c_30b4_30fc_30b9_30c8_540d:1`）。
- 何が変わるか: 名前を尋ねる会話が最後まで成立し、覚えた名前が起動をまたいで残り、台詞の中の呼びかけに使えるようになる。

**依存する既存 spec**: `currentghost-property-tree`（W15・1 件）／`areka-P0-package-mount`（完了・2 件）／`areka-P0-sylphya`（完了・2 件）／`areka-P0-sakura-dialogue-tags`（完了・1 件）

**構成 id（全 22 件）**

- `ukadoc:descript_ghost:char_2a.name_2c_540d_524d:1`
- `ukadoc:descript_ghost:kero.name_2c_540d_524d:1`
- `ukadoc:descript_ghost:name.allowoverride_2c_6570_5024:1`
- `ukadoc:descript_ghost:name_2c_30b4_30fc_30b9_30c8_540d:1`
- `ukadoc:descript_ghost:sakura.name2_2c_540d_524d:1`
- `ukadoc:descript_ghost:sakura.name_2c_540d_524d:1`
- `ukadoc:descript_shell:char_2a.name_2c_540d_524d:1`
- `ukadoc:descript_shell:kero.name_2c_540d_524d:1`
- `ukadoc:descript_shell:sakura.name_2c_540d_524d:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.name:1`
- `ukadoc:list_sakura_script:_25keroname:1`
- `ukadoc:list_sakura_script:_25selfname2:1`
- `ukadoc:list_sakura_script:_25selfname:1`
- `ukadoc:list_sakura_script:_25username:1`
- `ukadoc:list_sakura_script:_5c_21_5bopen_2cteachbox_5d:1`
- `ukadoc:list_shiori_event:OnNotifyUserInfo:1`
- `ukadoc:list_shiori_event:OnTeach:1`
- `ukadoc:list_shiori_event:OnTeachInputCancel:1`
- `ukadoc:list_shiori_event:OnTeachStart:1`
- `ukadoc:list_shiori_event:installedkeroname:1`
- `ukadoc:list_shiori_event:installedsakuraname:1`
- `ukadoc:list_shiori_resource:username:1`

### 起動と挨拶（段階 A・順位 4）

**候補 spec 名の案**: `areka-P0-boot-greeting-canon`

**3 行の要約**

- 問題: 定義ファイルを読んでゴーストを組み立て、SHIORI を読み込んで最初の 3 つのイベントを送るまでの口のうち、作者名や種別といった素性の欄が未対応で、ゴースト一覧に正しい名前が出ない。
- 現状: 構成 20 件の状態は実装済み 7・未対応 9・語彙のみ 4 で、壊れる 75 件のうち 4 件がここにあり、4 件とも素性の欄である。
- 何が変わるか: 配布されたゴーストを入れると素性が読み取られ、初回と 2 回目以降の挨拶が正典の順で送られるようになる。

**依存する既存 spec**: `areka-P0-charset-canon`（完了・2 件）／`areka-P0-package-mount`（完了・1 件）

**構成 id（全 20 件）**

- `ukadoc:descript_ghost:charset_2c_6587_5b57_30b3_30fc_30c9:1`
- `ukadoc:descript_ghost:craftman_2c_4f5c_8005_540d:1`
- `ukadoc:descript_ghost:craftmanurl_2cURL:1`
- `ukadoc:descript_ghost:craftmanw_2c_4f5c_8005_540d:1`
- `ukadoc:descript_ghost:id_2cID_540d:1`
- `ukadoc:descript_ghost:shiori.cache_2c_6570_5024:1`
- `ukadoc:descript_ghost:shiori.encoding_2c_6587_5b57_30b3_30fc_30c9:1`
- `ukadoc:descript_ghost:shiori.escape_unknown_2c0_2f1:1`
- `ukadoc:descript_ghost:shiori.forceencoding_2c_6587_5b57_30b3_30fc_30c9:1`
- `ukadoc:descript_ghost:shiori.version_2c_30d0_30fc_30b8_30e7_30f3:1`
- `ukadoc:descript_ghost:shiori_2c_30d5_30a1_30a4_30eb_540d:1`
- `ukadoc:descript_ghost:title_2c_8868_793a_540d:1`
- `ukadoc:descript_ghost:type_2c_7a2e_5225:1`
- `ukadoc:list_shiori_event:OnBoot:1`
- `ukadoc:list_shiori_event:OnFirstBoot:1`
- `ukadoc:list_shiori_event:OnInitialize:1`
- `ukadoc:list_shiori_resource:craftman:1`
- `ukadoc:list_shiori_resource:craftmanw:1`
- `ukadoc:list_shiori_resource:name:1`
- `ukadoc:list_shiori_resource:version:1`

### バルーンの文字（段階 A・順位 5）

**候補 spec 名の案**: `areka-P0-balloon-font-canon`（残余）

**3 行の要約**

- 問題: 作者が選んだ書体も文字色も効かず、どのゴーストも同じ見た目で喋る。
- 現状: 構成 63 件の状態は実装済み 13・未対応 32・語彙のみ 10・縮退 8 で、壊れる 75 件のうち 4 件がここにある。63 件のうち 45 件は進行中の 2 spec が `owner` に持つので、この束に残る仕事はその残余である。（この行の数は 2026-09-13 の写真である。すぐ下の「依存する既存 spec」の欄は 2026-09-18 に台帳から数え直したので、進行中の 2 spec の件数はこの 45 と合わない。節全体の撮り直しは統合担当へ申し送り済み）
- 何が変わるか: 作者の書体と色の指定が画面に出て、縮退している 8 件が正典どおりの振る舞いへ戻る。

**依存する既存 spec**: `currentghost-property-tree`（W15・13 件）／`text-align-shadow-canon`（W15・9 件）／`areka-P0-text-decoration-canon`（完了・16 件）／`areka-P0-balloon-font-descript-keys`（完了・7 件）／`areka-P0-balloon-parse`（完了・5 件）／`areka-P0-balloon-vertical-canon`（完了・4 件）／`areka-P0-cursor-tag-canon`（完了・1 件）／`script-small-tags`（v0.1.0・1 件）

**構成 id（全 63 件）**

- `ukadoc:descript_balloon:disable.font._28_30d5_30a9_30f3_30c8_5b9a_7fa9_29_2c_28_6307_5b9a_29:1`
- `ukadoc:descript_balloon:font.bold_2c0_2f1:1`
- `ukadoc:descript_balloon:font.color.b_2c_6570_5024:1`
- `ukadoc:descript_balloon:font.color.g_2c_6570_5024:1`
- `ukadoc:descript_balloon:font.color.r_2c_6570_5024:1`
- `ukadoc:descript_balloon:font.height_2c_6570_5024:1`
- `ukadoc:descript_balloon:font.italic_2c0_2f1:1`
- `ukadoc:descript_balloon:font.name_2c_30d5_30a9_30f3_30c8_540d:1`
- `ukadoc:descript_balloon:font.outline_2c0_2f1:1`
- `ukadoc:descript_balloon:font.shadowcolor.b_2c_6570_5024:1`
- `ukadoc:descript_balloon:font.shadowcolor.g_2c_6570_5024:1`
- `ukadoc:descript_balloon:font.shadowcolor.r_2c_6570_5024:1`
- `ukadoc:descript_balloon:font.shadowstyle_2c_5f62_614b_6307_5b9a:1`
- `ukadoc:descript_balloon:font.strike_2c0_2f1:1`
- `ukadoc:descript_balloon:font.underline_2c0_2f1:1`
- `ukadoc:descript_balloon:origin.x_2c_5ea7_6a19_20_2a1:1`
- `ukadoc:descript_balloon:origin.y_2c_5ea7_6a19_20_2a1:1`
- `ukadoc:descript_balloon:validrect.bottom_2c_5ea7_6a19_20_2a1:1`
- `ukadoc:descript_balloon:validrect.left_2c_5ea7_6a19_20_2a1:1`
- `ukadoc:descript_balloon:validrect.right_2c_5ea7_6a19_20_2a1:1`
- `ukadoc:descript_balloon:validrect.top_2c_5ea7_6a19_20_2a1:1`
- `ukadoc:descript_balloon:vertical_2c0_2f1:1`
- `ukadoc:descript_balloon:wordwrappoint.x_2c_5ea7_6a19_20_2a1:1`
- `ukadoc:descript_balloon:wordwrappoint.y_2c_5ea7_6a19_20_2a1:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.background.color:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.basepos.x:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.basepos.y:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.char_width:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.count:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.lines.initial:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.lines:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.num:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.validheight.initial:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.validheight:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.validwidth.initial:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.validwidth:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.vertical:1`
- `ukadoc:list_sakura_script:_5c_26_5bID_5d:1`
- `ukadoc:list_sakura_script:_5c__w_5banimation_2cID_5d:1`
- `ukadoc:list_sakura_script:_5c_l_5bx_2cy_5d:1`
- `ukadoc:list_sakura_script:_5c_m_5b0x00_5d:1`
- `ukadoc:list_sakura_script:_5c_n:1`
- `ukadoc:list_sakura_script:_5c_u_5b0x0000_5d:1`
- `ukadoc:list_sakura_script:_5cc_5bchar_2c_6570_5024_2c_958b_59cb_4f4d_7f6e_5d:1`
- `ukadoc:list_sakura_script:_5cc_5bline_2c_6570_5024_2c_958b_59cb_4f4d_7f6e_5d:1`
- `ukadoc:list_sakura_script:_5cf_5balign_2c_5bc4_305b_308b_5074_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bbold_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bcolor_2c_8272_6307_5b9a_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bdefault_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bdisable_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bheight_2c_6570_5024_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bitalic_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bname_2c_30d5_30a9_30f3_30c8_540d_5d:1`
- `ukadoc:list_sakura_script:_5cf_5boutline_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bshadowcolor_2c_8272_6307_5b9a_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bshadowcolor_2cnone_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bshadowstyle_2c_5f62_614b_6307_5b9a_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bstrike_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bsub_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bsup_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bunderline_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bvalign_2c_5bc4_305b_308b_5074_5d:1`
- `ukadoc:list_shiori_event:OnNotifyFontInfo:1`

### サーフェスアニメーション（段階 A・順位 6）

**候補 spec 名の案**: `areka-P0-seriko-animation-canon`

**3 行の要約**

- 問題: 目も口も動かない止め絵のまま立ち、まばたきも面の切り替えも作者の定義どおりには動かない。
- 現状（棚卸の時点の数。2026-10-10 の `seriko-trigger-intervals` の後の台帳では実装済み 7・語彙のみ 10・縮退 3）: 構成 47 件の状態は実装済み 2・未対応 27・語彙のみ 16・縮退 2 で、壊れる 75 件のうち 4 件がここにある。M1 の実機一周はこの束を 3 項目（項目 3・4・10）で見て合格しているが、それは emo2 が使う範囲の再生である。
- 何が変わるか: シェルの定義に書かれた再生の指定が一通り効き、面の切り替えと重ね絵が作者の意図どおりに動く。

**依存する既存 spec**: `currentghost-property-tree`（W15・6 件）／`areka-P0-shell-parse`（完了・2 件）／`areka-P0-seriko-trigger-intervals`（C5 の候補・3 件）／`seriko-interval-combinations`（シェルの element の列・1 件）

**構成 id（全 47 件）**

- `ukadoc:descript_shell_surfaces:alternativestart_2c_28ID1_2cID2..._29:1`
- `ukadoc:descript_shell_surfaces:alternativestop_2c_28ID1_2cID2..._29:1`
- `ukadoc:descript_shell_surfaces:always:1`
- `ukadoc:descript_shell_surfaces:animation-sort_2c_30bd_30fc_30c8_9806_5e8f:1`
- `ukadoc:descript_shell_surfaces:animation_2a.interval_2c_30a4_30f3_30bf_30fc_30d0_30eb:1`
- `ukadoc:descript_shell_surfaces:animation_2a.name_2c_5b9a_7fa9_540d:1`
- `ukadoc:descript_shell_surfaces:animation_2a.option_2c_30aa_30d7_30b7_30e7_30f3:1`
- `ukadoc:descript_shell_surfaces:animation_2a.option_2cbackground:1`
- `ukadoc:descript_shell_surfaces:animation_2a.option_2cexclusive:1`
- `ukadoc:descript_shell_surfaces:animation_2a.option_2cshared-index:1`
- `ukadoc:descript_shell_surfaces:animation_2a.pattern_2a_2c_63cf_753b_30e1_30bd_30c3_30c9_2c_30b5_30fc_30d5_30a7_30b9_756a_53f7_2c_30a6_30a7_30a4_30c8_2c:1`
- `ukadoc:descript_shell_surfaces:endtalk:1`
- `ukadoc:descript_shell_surfaces:insert_2cID:1`
- `ukadoc:descript_shell_surfaces:never:1`
- `ukadoc:descript_shell_surfaces:parallelstart_2c_28ID1_2cID2..._29:1`
- `ukadoc:descript_shell_surfaces:parallelstop_2c_28ID1_2cID2..._29:1`
- `ukadoc:descript_shell_surfaces:periodic_2c_6570_5024:1`
- `ukadoc:descript_shell_surfaces:random_2c_6570_5024:1`
- `ukadoc:descript_shell_surfaces:rarely:1`
- `ukadoc:descript_shell_surfaces:runonce:1`
- `ukadoc:descript_shell_surfaces:sometimes:1`
- `ukadoc:descript_shell_surfaces:start_2cID:1`
- `ukadoc:descript_shell_surfaces:starttalk:1`
- `ukadoc:descript_shell_surfaces:stop_2cID:1`
- `ukadoc:descript_shell_surfaces:talk_2c_6570_5024:1`
- `ukadoc:descript_shell_surfaces:yen-e:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.animation.num:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.seriko.defaultsurface:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.surface.num:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.surface_28ID_29.rect:1`
- `ukadoc:list_propertysystem:currentghost.seriko.surfacelist.all:1`
- `ukadoc:list_propertysystem:currentghost.seriko.surfacelist.defined:1`
- `ukadoc:list_sakura_script:_5c_21_5banim_2cadd_2coverlay_2cID_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5banim_2cadd_2ctext_2cx_2cy_2c_6a2a_5e45_2c_7e26_5e45_2c_6587_5b57_5217_2c_8868_793a_6642_9593_2cr_2cg_2cb_2c_658:1`
- `ukadoc:list_sakura_script:_5c_21_5banim_2cclear_2cID_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5banim_2coffset_2cID_2cx_5ea7_6a19_2cy_5ea7_6a19_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5banim_2cpause_2cID_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5banim_2cresume_2cID_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5banim_2cstop_2cID_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5beffect2_2c_8ffd_52a0_30b5_30fc_30d5_30a7_30b9ID_2c_30d7_30e9_30b0_30a4_30f3_540d_2c_901f_5ea6_500d_7387_2c_30d1:1`
- `ukadoc:list_sakura_script:_5c_21_5beffect_2c_30d7_30e9_30b0_30a4_30f3_540d_2c_901f_5ea6_500d_7387_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bfilter_2c_30d7_30e9_30b0_30a4_30f3_540d_2c_8d77_52d5_6642_9593_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bfilter_5d:1`
- `ukadoc:list_sakura_script:_5ci_5bID_2cwait_5d:1`
- `ukadoc:list_sakura_script:_5ci_5bID_756a_53f7_5d:1`
- `ukadoc:list_shiori_event:OnSurfaceChange:1`
- `ukadoc:list_shiori_event:OnSurfaceRestore:1`

## M2 予約群の対応表

**数え方**: 正本のロードマップの「M2 以降」の節は、予約を中黒（・）と全角の斜線（／）で区切って
並べている。この 2 つの区切りで切ると **17 項目**である。半角の斜線は 1 つの名前の内側なので
切らない——`Plugin/HEADLINE`・`ゴースト/バルーン選択 UI`・`里々/YAYA 網羅` の 3 つがそれである。

下の表は **18 行**ある。`Plugin/HEADLINE` だけが 2 つの束に分かれたので 2 行に割り、残る 16 項目は
1 行ずつである。**写った項目は 10 項目（11 行）、写らなかった項目は 7 項目**である（数え方: 下の
囲みで `none = true` を持つ行を数えて 7、束の名前を持つ行を数えて 11、割った 1 項目を戻して 10。
10 ＋ 7 ＝ 17 で節の列挙の全数と合う）。

読み取りにくい 3 つの写し先には根拠を添える。

- **NAR** は束「開発者機能」に写った。この束が nar を作る台本の指示と、作り始め・作り終わりを
  知らせる 2 つのイベントと、開発者向けの解説ページを 1 つに持っているためである。
- **ゴースト/バルーン選択 UI** は束「作り付けの窓」に写った。ゴースト側とバルーン側の選択の窓を
  開く台本の指示が、どちらもこの束に居るためである。
- **バルーン美観配置**（画面端での反転）は束「窓の配置と重なり」に写った。画面の端で位置を丸める
  かどうかを決める `ukadoc:descript_balloon:windowposition.limit_2c0_2f1:1` がこの束に居るためで
  ある。

```toml
[[reserved]]
name = "SSTP（9801）"
bundle = "SSTP"

[[reserved]]
name = "FMO"
bundle = "FMO"

[[reserved]]
name = "DirectSSTP"
none = true
reason = "カタログでこの綴りを引いて 0 件。写る先の項目が無い"

[[reserved]]
name = "Plugin"
bundle = "PLUGIN"

[[reserved]]
name = "HEADLINE"
bundle = "ヘッドライン"

[[reserved]]
name = "ネットワーク更新"
bundle = "更新"

[[reserved]]
name = "ゴースト/バルーン選択 UI"
bundle = "作り付けの窓"

[[reserved]]
name = "多重ゴースト"
bundle = "多重ゴースト"

[[reserved]]
name = "Shift_JIS"
bundle = "定義ファイルの文字コード"

[[reserved]]
name = "SAORI は実装しない"
none = true
reason = "カタログでこの綴りを引いて 0 件。写る先の項目が無く、実装しないという裁定は正本のロードマップが持つ"

[[reserved]]
name = "里々/YAYA 網羅"
none = true
reason = "カタログで里々と YAYA の綴りを引いて 0 件。標準テンプレート辞書の語彙は順位の根拠として引くだけで、正典の項目ではない"

[[reserved]]
name = "NAR"
bundle = "開発者機能"

[[reserved]]
name = "回転テキスト"
none = true
reason = "カタログで回転の綴りを引いて 0 件。予約されているのは areka の側の名前だけで、正典の項目が無い"

[[reserved]]
name = "バルーン美観配置"
bundle = "窓の配置と重なり"

[[reserved]]
name = "pasta の native x64"
none = true
reason = "areka の内側の実装方式の選択で、台帳の項目 0 件。数え方は別軸の節にある"

[[reserved]]
name = "IShiori の in-proc 化"
none = true
reason = "areka の内側の実装方式の選択で、台帳の項目 0 件。数え方は別軸の節にある"

[[reserved]]
name = "ベクトル描画"
none = true
reason = "areka の内側の実装方式の選択で、台帳の項目 0 件。数え方は別軸の節にある"

[[reserved]]
name = "owner-draw 右クリックメニュー"
bundle = "メニュー"
```

## 既存 brief への是正候補

**説明書の本体は 1 文字も書き換えていない。** ここに並べるのは案だけで、直すかどうかを決めるのは
各 spec の持ち主である。

拾い方は 2 つ。⑴ 調査 4 本のブリーフィングが「隣の spec の説明書への是正候補」として送ってきた
ものを全数読み、写真に写っている 27 本に宛てたものだけを採った。⑵ 本 spec で見つけたものを
加えた——2026-09-11 に 3 本を割って生まれた 6 本の説明書が、割る前の宛先のままになっている件で
ある。**差の理由が調査の側で説明済みのもの（族の名前だけを挙げた説明書に族の中身まで届いた、
別名になった項目の宛先が根の項目に付いた、など）は食い違いに数えていない。**

SHIORI の調査から届いた是正候補のうち、写真に写っている 27 本に宛てたものは **0 件**である
（数え方: あの文書の是正の候補の節の 4 件を 1 つずつ当たり、宛先が完了済みの調査 spec 自身・
正典への差し戻し・その場で直した台帳の 3 種だけだった）。

全部で **15 件**である。

| spec 名 | 食い違う id | 直し方の案 |
| --- | --- | --- |
| `property-catalog-lists` | `ukadoc:list_propertysystem:activeghostlist.index_28ID_29.ext._62e1_5f35_30d7_30ed_30d1_30c6_30a3_540d:1` と `ukadoc:list_propertysystem:activeghostlist_28_30b4_30fc_30b9_30c8_540d_2f_672c_4f53_5074_540d_2f_30d1_30b9_29.ext._62e1_5f35_30d7_30ed_30d1_30c6_30:1` の 2 件 | 拡張の 2 件は一覧の 5 件の中に既に入っている。合計を 12 から **10** へ直す |
| `property-catalog-lists` | `ukadoc:list_propertysystem:headlinelist.count:1`・`ukadoc:list_propertysystem:pluginlist.count:1` の 2 件 | 合計 11 は動かさず、内訳をヘッドライン **3**・プラグイン **5** へ直す（件数の葉を 2 つ数え落としている） |
| `property-catalog-lists` | 履歴の件数の葉 4 件（下に全列挙） | 「8」を **12** へ直す。4 つの枝それぞれに件数の葉がもう 1 つある |
| `property-catalog-lists` | 使える前置きが 1 つに限られる 4 件（下に全列挙） | 「17」を **13 ＋ 4** に割り、「どのカタログ根の下でも乗算される」という説明を 13 件の側だけに掛ける |
| `currentghost-property-tree` | バルーンのマウスの矢印 4 件（下に全列挙） | 括弧書きで名前だけ挙げてどの件数にも足していない 4 件を数に入れ、「≈65」を **66** へ直す |
| `currentghost-property-tree` | `ukadoc:list_propertysystem:currentghost.balloon._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1`・`ukadoc:list_propertysystem:currentghost.balloon.count:1` | 頭を落とした短い綴りを完全な名前へ書き直す。落とすと別の一族があるように読める |
| `zorder-property` | `ukadoc:list_propertysystem:currentghost.seriko.zorder:1` | **三重所有の仮裁定（案 甲）を採る場合にかぎり**、「一覧への登録は別の spec が行う。本説明書が正本として持つのは読み書きの書式であって一覧の 1 行ではない」の形へ直す。いまは同じ 1 行について正反対の指示が 2 本の説明書に残っている |
| `balloon-canon-residue` | `ukadoc:descript_ghost:sakura.balloon.defaultsurface_2c_6570_5024:1`・`ukadoc:descript_ghost:char_2a.balloon.defaultsurface_2c_6570_5024:1` | 初期表示面の綴りを 2 つではなく **4 つ**挙げる（本体側とキャラクタ番号で指す形が抜けている）。あわせて範囲の行「6 項目＋追加登記の 7〜10」を、番号の付いた項目が 12 まである実態へ追随させる |
| `areka-P0-text-decoration-canon` | `ukadoc:descript_balloon:font.outline_2c0_2f1:1` | 書体の欄を「基底 13 キー」ではなく **14** と書く。この 1 件が数に入っていない。分割先の説明書も同じ 13 を写しているので、両方を直す |
| `areka-P0-balloon-font-descript-keys` | 書体の欄 14 件（下に全列挙） | 2026-09-11 の分割 ⑶ でこの spec の持ち分になったが、台帳の宛先は分割元の `areka-P0-text-decoration-canon` のままである。宛先をこの spec へ移すか、分割元の説明書が範囲から外れたことを書く |
| `text-align-shadow-canon` | 寄せ 2 件と影 3 件（下に全列挙） | 分割 ⑵ でこの spec の持ち分になったが、台帳の宛先は分割元の `areka-P0-text-decoration-canon` のままである。宛先をこの spec へ移す |
| `areka-P0-balloon-lifecycle-events` | 表示寿命 5 件（下に全列挙） | 分割 ⑵ でこの spec の持ち分になったが、台帳の宛先は分割元の `balloon-canon-residue` のままである。宛先をこの spec へ移す。台帳の備考と受け渡し口の注記が名指ししている所有者名も同じ読み替えが要る（**2026-10-06 解消**: 3 つのイベントと `\![set,balloontimeout,時間]` の 4 件は、この spec が送る・読むようにして宛先をこの spec へ移し、台帳の備考を書き直した。受け渡し口の型 `BalloonLifecycleNotice` は消した。残る `\x[noclear]` は `talk-fast-forward` が持つ仕事で、宛先は分割元のまま） |
| `areka-P0-status-execution-states` | `ukadoc:list_sakura_script:_5c_21_5benter_2cnouserbreakmode_5d:1` | 説明書が書いている綴りが正典のどの項目にも当たらない。正典の綴りへ直す。**この 1 件は重い**——その項目の担当を主張している説明書自身が違う綴りを書いている |
| `areka-P0-charset-canon` | 宛先の欄が空の 6 件（下に全列挙） | 説明書は範囲にも範囲外にもこの 6 件を挙げていない。範囲に入れるか対象外と書くかを決める。**これは誤りではなく沈黙なので、台帳の備考には何も書かれていない** |
| `areka-P0-sylphya-set-ledger` | サウンドの語彙 18 件（下に全列挙） | 説明書はサウンドの語彙族の登記と台帳の宛先の記入を自分の範囲だと書いているが、台帳の宛先は 18 件とも `property-catalog-lists` である。どちらが持つかを決める（**2026-09-17 解消**: 記録用の語彙表 `SOUND_PROP_NAMES` は `areka-P0-sylphya-set-ledger`〔PR#151〕、値の導出と台帳の宛先は `property-catalog-lists` のまま。正本は `doc/COMPAT_ARCHITECTURE.md` §8 の【所有の相互参照】行） |

### 全列挙

**履歴の件数の葉 4 件**

- `ukadoc:list_propertysystem:history.balloon.count:1`
- `ukadoc:list_propertysystem:history.ghost.count:1`
- `ukadoc:list_propertysystem:history.headline.count:1`
- `ukadoc:list_propertysystem:history.plugin.count:1`

**使える前置きが 1 つに限られる 4 件**

- `ukadoc:list_propertysystem:menu:1`
- `ukadoc:list_propertysystem:sakura.bind.menu:1`
- `ukadoc:list_propertysystem:kero.bind.menu:1`
- `ukadoc:list_propertysystem:char_2a.bind.menu:1`

**バルーンのマウスの矢印 4 件**

- `ukadoc:list_propertysystem:currentghost.balloon.mousecursor:1`
- `ukadoc:list_propertysystem:currentghost.balloon.mousecursor.arrow:1`
- `ukadoc:list_propertysystem:currentghost.balloon.mousecursor.text:1`
- `ukadoc:list_propertysystem:currentghost.balloon.mousecursor.wait:1`

**書体の欄 14 件**

- `ukadoc:descript_balloon:font.bold_2c0_2f1:1`
- `ukadoc:descript_balloon:font.color.b_2c_6570_5024:1`
- `ukadoc:descript_balloon:font.color.g_2c_6570_5024:1`
- `ukadoc:descript_balloon:font.color.r_2c_6570_5024:1`
- `ukadoc:descript_balloon:font.height_2c_6570_5024:1`
- `ukadoc:descript_balloon:font.italic_2c0_2f1:1`
- `ukadoc:descript_balloon:font.name_2c_30d5_30a9_30f3_30c8_540d:1`
- `ukadoc:descript_balloon:font.outline_2c0_2f1:1`
- `ukadoc:descript_balloon:font.shadowcolor.b_2c_6570_5024:1`
- `ukadoc:descript_balloon:font.shadowcolor.g_2c_6570_5024:1`
- `ukadoc:descript_balloon:font.shadowcolor.r_2c_6570_5024:1`
- `ukadoc:descript_balloon:font.shadowstyle_2c_5f62_614b_6307_5b9a:1`
- `ukadoc:descript_balloon:font.strike_2c0_2f1:1`
- `ukadoc:descript_balloon:font.underline_2c0_2f1:1`

**寄せ 2 件と影 3 件**

- `ukadoc:list_sakura_script:_5cf_5balign_2c_5bc4_305b_308b_5074_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bvalign_2c_5bc4_305b_308b_5074_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bshadowcolor_2c_8272_6307_5b9a_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bshadowcolor_2cnone_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bshadowstyle_2c_5f62_614b_6307_5b9a_5d:1`

**表示寿命 5 件**

- `ukadoc:list_sakura_script:_5c_21_5bset_2cballoontimeout_2c_6642_9593_5d:1`
- `ukadoc:list_sakura_script:_5cx_5bnoclear_5d:1`
- `ukadoc:list_shiori_event:OnBalloonBreak:1`
- `ukadoc:list_shiori_event:OnBalloonClose:1`
- `ukadoc:list_shiori_event:OnBalloonTimeout:1`

**宛先の欄が空の文字コードの 6 件**

- `ukadoc:descript_ghost:charset_2c_6587_5b57_30b3_30fc_30c9:1`
- `ukadoc:descript_shell:charset_2c_6587_5b57_30b3_30fc_30c9:1`
- `ukadoc:descript_balloon:charset_2c_6587_5b57_30b3_30fc_30c9:1`
- `ukadoc:descript_plugin:charset_2c_6587_5b57_30b3_30fc_30c9:1`
- `ukadoc:descript_headline:charset_2c_6587_5b57_30b3_30fc_30c9:1`
- `ukadoc:descript_ghost:shiori.escape_unknown_2c0_2f1:1`

**サウンドの語彙 18 件**

- `ukadoc:list_propertysystem:duration:1`
- `ukadoc:list_propertysystem:error:1`
- `ukadoc:list_propertysystem:id:1`
- `ukadoc:list_propertysystem:loop:1`
- `ukadoc:list_propertysystem:meta.album:1`
- `ukadoc:list_propertysystem:meta.albumartist:1`
- `ukadoc:list_propertysystem:meta.artist:1`
- `ukadoc:list_propertysystem:meta.artwork:1`
- `ukadoc:list_propertysystem:meta.genre:1`
- `ukadoc:list_propertysystem:meta.title:1`
- `ukadoc:list_propertysystem:meta.track:1`
- `ukadoc:list_propertysystem:meta.year:1`
- `ukadoc:list_propertysystem:name:2`
- `ukadoc:list_propertysystem:path:2`
- `ukadoc:list_propertysystem:pause:1`
- `ukadoc:list_propertysystem:playing:1`
- `ukadoc:list_propertysystem:position:1`
- `ukadoc:list_propertysystem:preload:1`

## 別軸

M2 の技術選定 4 つ——pasta の native x64・`IShiori` の in-proc 化・ベクトル描画・AI——は、
段階にも波にも並べず、順位も付けない。この 1 節だけに置く。

| 技術選定 | 台帳の項目 | 順位 | 波 |
| --- | ---: | --- | --- |
| pasta の native x64 | 0 件 | 付けない | 割り当てない |
| `IShiori` の in-proc 化 | 0 件 | 付けない | 割り当てない |
| ベクトル描画 | 0 件 | 付けない | 割り当てない |
| AI | 0 件 | 付けない | 割り当てない |

「台帳の項目 0 件」の根拠と数え方は `linkage.md`「別軸」にある——4 つはいずれも正典の記述では
なく areka の内側の実装方式の選択で、カタログにも台帳 4 本にも対応する項目が立っていない。
構成 id を列挙できないので、順位の 4 つの根拠（壊れ方・テーマの数・影響する既存資産の広さ・
依存基盤の共有度）はどれも構成 id から導く値であり、この 4 つを同じ物差しに載せられない。

波を割り当てないのも同じ理由である。どの技術選定を採っても段階と束の順序は変わらず、変わるのは
1 つの束を実装するときの中身の作り方だけである。したがってこの 4 つは、段階の節の 67 行の
どこにも現れない（**0 行**。数え方: 5 つの段階の表の「束」の欄を全部読み、4 つの名前を探した）。

## M3 の受入基準の候補

M3「伺かの冠」の受入基準の候補として、**「テーマ 8 つすべてで代表束が実装済み」**を登記する
（要件 5.7）。**決めるのは階梯の議論（棚卸セッション）であり、本文書は候補を置くだけである。**

代表束の決め方も候補が 2 つある。

- 案 ⑴: そのテーマを持つ束のうち、順位表で最も前に来るもの（段階の若い順、同じ段階なら
  順位の小さい順）を代表とする。下の表がその案を当てた結果である。
- 案 ⑵: 1 つの束が 2 つ以上のテーマの代表を兼ねないようにし、テーマごとに別の束を代表とする。

| テーマ | そのテーマを持つ束と単独項目 | 案 ⑴ の代表 | 代表の段階・順位 |
| --- | ---: | --- | --- |
| 気配 | 7 | 会話 | 段階 A・順位 1 |
| 触れ合い | 7 | キーとゲームパッド | 段階 A・順位 9 |
| 掛け合い | 7 | 会話 | 段階 A・順位 1 |
| 装い | 25 | 窓の配置と重なり | 段階 A・順位 2 |
| 記憶 | 9 | 名前の記憶 | 段階 A・順位 3 |
| 交わり | 13 | 会話 | 段階 A・順位 1 |
| 気配り | 12 | 会話 | 段階 A・順位 1 |
| 更新 | 2 | インストール | 段階 B・順位 1 |

数え方: `linkage.md` の 67 の囲みの `themes` を全部読み、そのテーマを含む囲みを数えて、
`briefing.md` 3-3 の順位表の並びで最も前に来るものを代表に採った。

案 ⑴ を当てると、8 つのテーマのうち **4** つを **1** 束
（会話）が兼ねる。これが案 ⑵ を候補に残す理由である——1 つの束が動いただけで
複数のテーマの受入が同時に立つと、受入基準が数えるものが薄くなる。どちらを採るかも棚卸
セッションの裁定に委ねる。

## 裁定候補

### ウェーブの並べ替え（要件 10.3）

正本のウェーブ編成（W13〜W17）は入力である。この文書は並べ替えを提案として置くだけで、
正本を編集しない。提案は **3 件**である。

**W-1 M2 の波を入れる場所**

- 現在のウェーブ: 正本の編成は W13〜W17 で、M2 の波は 1 つも編成されていない。
- 提案: M2 の第 1 波を W17 の後に置く。
- 理由: 先頭ウェーブ 6 束の構成 id のうち、進行中の spec が `owner` に持つぶん（件数は
  「先頭ウェーブ」の節にある）の宛先の最も遅いウェーブが W17 である。W17 より前に第 1 波を
  差し込むと、同じ id を 2 つの spec が同時に触る。

**W-2 `areka-P0-anchor-tag-canon` を繰り上げるか、宛先を 1 件だけ移すか**

- 現在のウェーブ: W17（正本の編成の最後）。
- 提案: `ukadoc:list_sakura_script:_5c_a_5bID_2cr2_2cr3..._5d:1` の 1 件を先頭ウェーブの候補
  spec `areka-P0-talk-script-canon` の宛先へ移すか、`areka-P0-anchor-tag-canon` を W16 以前へ
  繰り上げる。
- 理由: 先頭ウェーブ 6 束が `owner` に持つ進行中 spec の宛先（件数は「先頭ウェーブ」の節に
  ある）のうち、W17 に掛かるのはこの **1 件**だけで、残りの最も遅いウェーブは W16 である。
  この 1 件のために第 1 波の始まりが 1 ウェーブ遅れる。

**W-3 `surfaces-basepos` の任意を外すか**

- 現在のウェーブ: W13 任意／W14（正本の spec 台帳とウェーブ編成が「任意」と書く）。
- 提案: W13 で確定させる。
- 理由: この spec は先頭ウェーブの束「窓の配置と重なり」の構成 id 2 件——
  `ukadoc:descript_shell_surfaces:point.basepos.x_2c_5ea7_6a19:1` と
  `ukadoc:descript_shell_surfaces:point.basepos.y_2c_5ea7_6a19:1`——を `owner` に持つ。
  任意のまま W14 へ流れると、先頭ウェーブの前提が 1 ウェーブ後ろへずれる。

### 同順位の解消（要件 6.7）

4 つの根拠がすべて同じ値になって同順位に並んだ組は **13 組・29 束**
である（`briefing.md` 3-5。数え方はそこにある）。同順位のままでは組の中のどれを先に作るかが
決まらないので、解消を裁定候補に上げる。**本文書では順序を作っていない**——4 つの根拠のほかに
順序の根拠を足すと、要件 6.1 が凍結した序列に 5 つ目の根拠を足すことになる。

| # | 段階 | 順位 | 同順位の束 | 波の案 |
| ---: | :---: | ---: | --- | --- |
| 1 | A | 7 | 入力窓とダイアログ・自発発話 | 第 2 波 |
| 2 | A | 10 | descript の転記・バルーンのリンク・マウスの矢印 | 第 2 波 |
| 3 | A | 14 | イベントの呼び起こし・選択肢の目印 | 第 2 波 |
| 4 | A | 17 | 定義ファイルの文字コード・組み込みの置換語 | 第 2 波 |
| 5 | A | 18 | SHIORI の要求と応答・シェル定義の転記・同期オブジェクト | 第 2 波 |
| 6 | B | 5 | 休止と復帰・好感度の絵柄・着せ替え | 第 3 波 |
| 7 | B | 8 | `ukadoc:manual_directory`・`ukadoc:manual_ghost` | 第 3 波 |
| 8 | C | 1 | スクリーンセーバー・バッテリー | 第 4 波 |
| 9 | C | 2 | OS の変化の察知・ディスプレイ変化 | 第 4 波 |
| 10 | C | 4 | 壁紙・通知領域 | 第 4 波 |
| 11 | C | 9 | サウンド・予定表 | 第 4 波 |
| 12 | D | 2 | SSTP・コミュニケート | 第 5 波 |
| 13 | E | 6 | 作り付けの窓・読み上げと聞き取り | 第 6 波 |

**先頭ウェーブに掛かる組は 0 組である**（数え方: 上の表の段階が A で順位が 6 以下の
行を数えた）。先頭ウェーブの 6 束は順位 1〜6 に 1 つずつ並んでおり、どれを先に起票するかは
裁定を待たずに決まる。第 2 波より後は、この表の解消が着手順を決める。

**決めること**: 各組の中の順序。**答えで変わること**: その波の中でどの束から手を着けるか。
段階と波は動かないので、利用者から見える節目の順序は変わらない。

### 段階の割り当てと順序の印（`briefing.md` にある裁定候補）

段階の割り当ての裁定候補 **10 件**は `briefing.md` 2-5 に、順序の主張から外した `override` の
**2 行**は同 3-4 にある。どちらも本文書では決めない。裁定で段階が動けば、この文書の段階の節と
波の割り当ても組み直す。

**先頭ウェーブに掛かるものは 0 件である**（数え方: 2-5 の 10 件と 3-4 の 2 行が名指す束の名前を
先頭ウェーブの 6 束と突き合わせ、一致するものを数えた）。掛かるのは第 2 波より後の束だけである。
