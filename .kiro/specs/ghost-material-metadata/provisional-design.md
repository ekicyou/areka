# 暫定の設計: ghost-material-metadata

> **作り直し済み（2026-11-20）**: この暫定の設計は、開発者の決め（`doc/ideas/novel-decisions-2026-11-20.md`）の前の物である。決めで作り直した物は `doc/ideas/novel-redesign-2026-11-20/`（骨組みは `00-architecture.md`）。この文書は、確かめた事実（`path:行`）の控えとして残す。

## 1. 前置き

- **これは暫定の設計である。正式な `design.md` ではない。** 要件定義（`requirements.md`）はまだ無い。brief と、pasta の骨子・取り決めから暫定の要件を起こし、設計の段まで一度降りて、上位の決定へ差し戻しが要る矛盾を探すための記録である。
- 日付: 2026-11-19。基にした main: `49b6f3ab5`。
- 深さは、ノベル（演劇）に関わる所（役・席・舞台の定義・役者のシェルの引き当て）を先にした。単体のフリーシェルのインストールと、作者のページへの案内は、大枠だけである。
- 読んだ物:
  - areka: `.kiro/specs/ghost-material-metadata/brief.md`・`.kiro/specs/shell-multi-compose/brief.md`・`doc/ideas/novel-platform.md`・`.kiro/steering/roadmap.md` の「利用統計とノベルゲーム基盤」の節（1705〜1735 行）・`.kiro/specs/property-catalog-lists/brief.md`（grep だけ）。
  - pasta（読むだけ）: `novel-areka-contract/final-model-2026-11-19.md`（以下「骨子」）・同 `brief.md` の「決定（2026-11-19…最終形）」の節（以下「取り決めの brief」）・同 `play-script-guide.md`（役・席で grep）。
  - areka の側の見立て: `areka-novel-rollback-cast-nar-2026-11-19.md`（全部）・`final-form-check`・`text-destination`・`stage-as-actors`・`system-actor`・`window-modes`（役・席・descript で grep）。
  - コード: 3 章に `path:行` で挙げた物。
  - ukadoc（MCP）: descript の `id`・`craftmanurl`・`menu,hidden`・install.txt の `accept` と概要・プロパティの `craftmanurl`・`menu`。

## 2. 暫定の要件

出どころの略: 【B】＝この単位の brief、【骨】＝骨子、【取】＝取り決めの brief の「決定（最終形）」、【N】＝`doc/ideas/novel-platform.md` の「決めたこと」。

| 番号 | 要件 | 出どころ |
| ---- | ---- | -------- |
| R1 | areka は、ゴースト・シェル・バルーンを、素材 ID `種別:id` で指せなければならない。ゴーストに同梱の素材は親の id を前に付け（`shell:<ゴーストの id>/<シェルの id>`）、単体の素材は前に何も付けない。 | 【B】Desired Outcome・【N】7 |
| R2 | 素材の id は、descript.txt の `id` の値とする。`id` が無いか空のとき、areka はフォルダ名を id とする。 | 【B】Approach「ID の規則」 |
| R3 | 手元で同じ素材 ID が 2 つ在るとき、areka は、決まった順（根の並びの上から・同じ根の中はフォルダ名のバイト順）で最初の 1 つに決め、かぶりを `warn!` 1 行で記録しなければならない。 | 【B】Approach・Constraints |
| R4 | 親を書かないシェルの id を、ゴースト G から探すとき、areka は ⑴ G の `shell\` の下 ⑵ 根の `free-shell\` の下 ⑶ 根の `ghost\<id>\` の既定のシェル、の順に探し、最初に当たった物に決めなければならない。親まで書いた ID は、探さずに、それだけを指す。 | 【B】Approach「探す順」・【N】7 |
| R5 | 当たらないとき、areka は、次の候補（作者が書いた既定）へ戻し、「未導入」と `warn!` 1 行で記録しなければならない。候補が尽きたら、その役者を飛ばす。 | 【B】Desired Outcome・`shell-multi-compose` の brief の Constraints |
| R6 | areka は、シェルの descript.txt から、役（`character`・`background`・`stage`。書いていなければ `character`）と、「借りないで」の印を読めなければならない。 | 【B】Desired Outcome・【取】「舞台の窓と箱」 |
| R7 | areka は、ゴーストの descript.txt（`ghost\master\descript.txt`）から、舞台の定義（並びの名前とシェルの対応・席の名前と位置の対応）を読めなければならない。 | 【取】「舞台の窓と箱」の「席の名前と位置の対応は、舞台の定義（`descript.txt` の areka の独自キー）に書く」 |
| R8 | 「借りないで」の印の在るシェルを、親でないゴーストが借りようとしたとき、areka は、そのシェルを当たりにせず、記録して R5 へ進まなければならない。同梱のシェル（親が今のゴースト）は、印が在っても使える。 | 【B】Approach「借りてよいか」・【N】5 |
| R9 | 役が `character` でないシェルを、areka は、シェルの切り替えのメニューに並べてはならない。 | 【取】「舞台の窓と箱」の「シェルの切り替えのメニューは、areka が役で隠す」 |
| R10 | 目録の素性は、`craftmanurl` と、足したキー（ライセンス・支援先）を持たなければならない（完了 `baseware-root-layout` の要件 2.9 を改める）。 | 【B】Scope・Existing Spec Touchpoints |
| R11 | areka は、今のゴースト・シェル・バルーンと、舞台の定義が借りたシェルについて、作者の名前と、作者の配布の場所・支援先を、メニューから見られ、開けなければならない。 | 【B】Desired Outcome・Approach「借りたシェルの作者表記」 |
| R12 | areka は、単体のフリーシェルを、根の `free-shell\<フォルダ名>\` へインストールできなければならない。 | 【B】Scope・【N】7 |
| R13 | 独自キーを 1 つも持たないゴースト・シェル・バルーンでは、目録の並び・シェルの切り替えのメニュー・切り替えの結果・起動の結果が、今と変わってはならない。 | 【B】Desired Outcome の最後・進行役の指示 |
| R14 | 独自キーは、正典のキーの名前と衝突せず、正典のキーの意味を変えてはならない。 | 【B】Approach「キー」・Constraints |
| R15 | かぶり・未導入・「借りないで」・読めない独自キーの値は、どれも `warn!` 1 行で記録しなければならない。メッセージボックスは出さない。 | 【B】Constraints |
| R16 | 役・席・並びの名前は、さくらスクリプトの命令の型にも、pasta の辞書の形にも、新しい種別を足してはならない（areka が descript から読むだけ）。 | 【取】「画面の部品はアクター」・【骨】2.7 |
| R17 | 作者向けの文書（キーの一覧・書き方・「id は一度決めたら変えない」）を持たなければならない。 | 【B】Scope・Approach |

## 3. 今の作り（確かめた事実）

- **descript.txt の読み手は、平らな `key,value` の表である。ブレスを知らない。**
  - `crates/areka-parsers/src/kv/parse.rs:17-30`（`parse_kv_with_line`）: 行を最初のカンマで分ける。カンマの無い行は飛ばす。同じキーは後の行が勝つ。`{`・`}` だけの行はカンマが無いので飛ぶが、ブレスの中の `key,value` の行は、最上位のキーとして表に入る。
  - 同 `:47-59`（`parse_kv_lowercased`）: キーを ASCII で小文字にそろえ、空の値のキーを落とす。
- **ゴーストの descript の読み**: `crates/areka-parsers/src/package/resolve_shell.rs:51-75`（`read_ghost_descript`）と `:142-217`（`resolve_with_shell`）。読むキーは名前・`shiori` 系・`readme`・`seriko.defaultsurfacedirectoryname` だけ（同 `:6-8`）。結果の型は `MountModel`（`package/model.rs:28-42`・`#[non_exhaustive]`）。既定のシェル名の規則は `default_shell_name`（`resolve_shell.rs:100-113`・非公開）。
- **目録**: `crates/areka-ghost/src/catalog.rs`。
  - 素性の型 `Identity`（`:113-139`）は 7 項目。コメント「7 項目以外は足さない（要件 2.9）」。組むのは `identity()`（`:458-477`）。`craftmanurl` は読まない。`homeurl` は別の関数（`:336`）。
  - シェルの列挙 `scan_shells`（`:194-207`）。`menu` が `hidden` のシェルを除く（`:200`）。口は `list_shells`（除く・`:183`）と `list_all_shells`（除かない・`:189`）。
  - 根の置き場は `ghost_store`（`:85`）と `balloon_store`（`:98`）の 2 つ。`free-shell` は無い。
  - 根の並びをまたぐ和と「隠れ」は `crates/areka-ghost/src/catalog_roots.rs:221-249`（`union`）。フォルダ名の同じ・違うの規則は `same_folder_name`（`:252`）の 1 つ。ゴーストの中のバルーンを先に探す前例は `balloons_for`（`:156-175`）。
- **シェルの切り替えのメニュー**: `crates/areka/src/menu/shell_frame.rs:39` が `list_shells` を呼び、目録の並びのまま子項目にする（`:54-96`）。
- **`list_shells` のほかの使い手**: 切り替えの候補の隠しの印（`crates/areka-runtime/src/shell_balloon_resolve.rs:91-105`。`random` は隠しを除く・`:213-216`）と、`updateother` の名前の引き当て（`crates/areka/src/update/desk.rs:632`）。
- **メニューの枠**: `crates/areka/src/menu/mod.rs:51-76` の `Frame`（12 個）と `Frame::ORDER`（`:80`・長さ 12 の配列）。作者のページへの案内の枠は無い。
- **インストール**:
  - 種別は `InstallKind`＝`Ghost`・`Shell`・`Supplement`・`Balloon`（`crates/areka-nar/src/manifest.rs:94-99`）。install.txt の知らないキーは、警告 1 件にして読み飛ばす（同ファイルの頭の説明 `:20-24`）。
  - シェルの宛先は、宛先のゴーストの `shell\<directory>`（`crates/areka-nar/src/plan.rs:212-220`）。根を尋ねる口 `InstallRoots` は `ghost_root`・`balloon_root` の 2 つ（`crates/areka-nar/src/install.rs:53-58`）。
  - **`accept` の無い `type,shell` を、areka は今、失敗にする**（`crates/areka/src/install/judge.rs:40-48` の `AcceptMissing` → `procedure.rs:219-231` で `invalid type`）。
- **areka の独自キーは、descript.txt にはまだ 1 つも無い。** 探し方: `crates/` の `*.rs` を `"areka\.` で grep。当たったのは sylphya の保存のキー（`crates/areka-sylphya/src/persist/mod.rs:178` の `areka.window.scope(n).x` など。descript でなくプロファイルの側）だけ。surfaces.txt の側には、`balloon.*ブレス` の中の `size`・`font.follow` が areka 独自のキーとして在る（`crates/areka-emo-compose/src/boxes.rs:39`・`:396`）。
- **`audit-ghost-catalog` は完了している**（`.kiro/specs/completed/audit-ghost-catalog/`）。brief の Upstream の条件（「目録を先に触るなら、その着地の後に」）は満たされた。
- **ukadoc（正典）**:
  - `id,ID名`（シェル）: 「そのシェルのID名。半角英数のみ。(SSPは全文字許容) 今のところ起動時のコンソールのような表示に出てくる以外の用途はない。」（<https://ukatech.shillest.net/ukadoc-spec/descript_shell.html#id>）
  - `craftmanurl,URL`: 「作者のURL。」（<https://ukatech.shillest.net/ukadoc-spec/descript_shell.html#craftmanurl>）。プロパティの汎用の名前にも在る: 「対象の配布先URL。」（<https://ukatech.shillest.net/ukadoc-spec/list_propertysystem.html#craftmanurl>）
  - `menu,hidden`（シェルの descript）: 「シェルをゴーストのシェル切り替えメニューに表示しなくなる。」対応は SSP（<https://ukatech.shillest.net/ukadoc-spec/descript_shell.html#menu>）。
  - install.txt の `accept`: 「追加シェルや追加ファイル（supplement）など、特定ゴーストに渡す内容では必ず用いる。」既定値は「挙動なし」（<https://ukatech.shillest.net/ukadoc-spec/descript_install.html#accept>）。概要は「追加シェルやサプリメントの場合、想定外のゴーストにインストールされる事を避けるためにaccept指定が利用できる。」（<https://ukatech.shillest.net/ukadoc-spec/descript_install.html#introduction_install.txt>）
  - **「SSP は descript.txt の知らないキーを読み飛ばす」と書いた文は、ukadoc に見つからなかった。** 探し方: `search_docs` で descript の区分を「読み飛ば」（0 件）・「無視」（22 件。どれも個別のキーの中の引数の話）・「独自」（1 件。surfaces.txt の SERIKO の話）。brief の「SSP は知らないキーを読み飛ばす」は、正典では裏が取れていない。

## 4. 設計

### 4.1 「役」は両立するか（先に結論）

取り決めの brief は、「役（背景 → 役者 → トークの場所）」で自動の配置と字の行き先を決めると書き、同じ brief が「『背景』『ボタン』『システム』という役や種別を、pasta にも取り決めにも作らない」と書く。

- **作りの上では両立する。** 役を、シェル（素材）の descript.txt に素材の作者が書く宣言にし、areka だけが読む。さくらスクリプトの命令にも、pasta のアクター辞書にも、役は出てこない（R16）。pasta が知るのはアクターの名前だけで、役を知らないまま台本が書ける。
- **役の値は 3 つのまま増やさない。** 帯やボタンを載せた絵のアクター（「システム」）は、役を書かない（`character`）。位置と重ね順を明示して置く（取り決めの brief の「位置を明示したアクターは、自動の並べ方に数えない」と、`play` の重ね順の命令で足りる）。「ボタン」「システム」という役は作らない。
- **字面ではぶつかる。** 「『背景』…という役…を…取り決めにも作らない」と「役（背景 → 役者 → トークの場所）」は、同じ文書の中で、「背景」という役が在る・無いの両方を言っている。取り決めの文書には、役の語と 3 つの値が要る（作者がシェルの descript に役を書かないと、自動の配置と字の行き先が決まらない）。9 章に書く。
- **シェルの切り替えのメニューを隠すことは、役が無くても正典でできる。** 正典の `menu,hidden` が同じ働きを持ち、areka はもう読んでいる（`catalog.rs:200`）。areka は役でも隠す（R9）が、SSP でも隠すには、作者が `menu,hidden` を書く。

### 4.2 キー（名前はどれも仮）

どれも、`areka.` で始まる平らなキーにする。**descript.txt にブレスは書かない。** 理由は 3 章の事実である（ブレスの中の行が最上位のキーとして読まれる。たとえば中に `name,窓際` と書くと、ゴーストの `name` を上書きする）。番号つきの形は、正典の `sakura.bindgroup*.name,カテゴリ名,パーツ名` と同じ型にした（キーは ASCII だけ・名前は値の側）。

| 書く所 | キー | 値 | 既定 |
| ------ | ---- | -- | ---- |
| シェルの descript.txt | `areka.role` | `character`・`background`・`stage` | `character` |
| シェルの descript.txt | `areka.borrow` | `deny`（借りないで） | 借りてよい |
| ゴースト・シェル・バルーンの descript.txt | `areka.license` | 文字列（1 行） | 無し |
| ゴースト・シェル・バルーンの descript.txt | `areka.supporturl` | URL | 無し |
| ゴーストの descript.txt（`ghost\master\`） | `areka.stage.actor*` | `並びの名前,素材 ID(,素材 ID…)` | 無し |
| ゴーストの descript.txt（`ghost\master\`） | `areka.stage.seat*` | `席の名前,X,Y` | 無し |

- `*` は 0 からの番号。番号の抜けは許し、在る物を番号の順に読む。同じ名前が 2 回出たら、番号の小さい方を使い、記録する。
- `areka.stage.actor*` の素材 ID の並びは、候補の順である。前から探し、最初に当たった物を使う（R5 の「既定へ戻す」は、作者が最後に同梱のシェルを書くことで表す）。
- `areka.stage.seat*` の X・Y は、基準の大きさ（1920×1080・取り決めの brief）の上の整数。アクターのどこを合わせるかは、この単位では決めない（`shell-multi-compose` と取り決めの要件定義）。
- 読めない値（知らない役の語・数でない座標・項目の足りない行）は、その行を使わずに記録する（R15）。
- 作者・配布の場所は、正典の `craftman`・`craftmanw`・`craftmanurl` をそのまま使う。
- 「演劇である」の印・基準の大きさのキー・席の要件（差し替えの条件）・席の錠は、**作らない**（7 章の仮定 A3・A4・A5）。

例（演劇のゴースト。背景は同梱・さくらは借り物で、無ければ同梱の代役）:

```
// ghost\master\descript.txt
areka.stage.actor0,さくら,shell:sakura_ghost/master,shell:standin
areka.stage.seat0,窓際,1400,1080
areka.stage.seat1,教卓,960,1080

// shell\教室\descript.txt
name,教室
menu,hidden
areka.role,background

// shell\台詞の箱\descript.txt
name,台詞の箱
menu,hidden
areka.role,stage
```

### 4.3 構成（どの crate のどの層に何を足すか）

| 層 | 足す物 | I/O |
| -- | ------ | --- |
| `areka-parsers`（`package` の下に 1 ファイル・仮 `material.rs`） | 素材 ID の型と読み書き（`MaterialKind`＝`Ghost`・`Shell`・`Balloon`、`MaterialRef { kind, parent, id }`）。シェルの独自キーの読み（`ShellRole`・借りてよいか）。舞台の定義の読み（`StageDefinition { actors, seats }`）。どれも、キーを小文字にした表（`parse_kv_lowercased` の結果）から値を作る関数で、読めなかった行の一覧を一緒に返す | 無し |
| `areka-parsers`（`package/model.rs`・`resolve_shell.rs`） | `MountModel` に `stage: StageDefinition` を足す。`resolve_with_shell` が、もう読んでいるゴーストの descript の表から組む（ファイルを読み足さない） | 今のまま |
| `areka-ghost`（`catalog.rs`） | `Identity` に `craftmanurl`・`license`・`support_url` を足す（要件 2.9 を改める）。`ShellEntry` に `role`・`borrowable` を足す（シェルだけの欄は `Identity` に入れない）。`BasewareRoot::free_shell_store`（`<根>\free-shell`）と、その走査。`scan_shells` の「隠す」の判定に「役が `character` でない」を足す | 読むだけ |
| `areka-ghost`（`catalog_roots.rs`） | `free-shell` の和（今の `union` に `StoreKind::FreeShell` を足す。根の並び・`dev-folder-alias` の別名は、この口を通るので、そのまま効く） | 読むだけ |
| `areka-ghost`（新しい 1 ファイル・仮 `material_resolve.rs`） | シェルの引き当て（R3・R4・R8）と、役者の引き当て（R5・舞台の定義の候補の順）。呼ぶたびに今のファイルの状態から作る（`balloons_for` と同じ流儀。控えを持たない） | 読むだけ |
| `areka-nar`・`crates/areka/src/install/` | `accept` の無い `type,shell` を、単体のフリーシェルとして `<根>\free-shell\<directory>\` へ置く（今は失敗・3 章）。`InstallRoots` に `free_shell_root` を足す | 書く |
| `crates/areka/src/menu/` | 作者のページの枠を 1 つ足す（`Frame` と `Frame::ORDER` が 13 になる）。今のゴースト・シェル・バルーンと、借りたシェルの、作者の名前・`craftmanurl`・`areka.supporturl` を並べ、選ぶと OS の既定のブラウザで開く。開くのは `http`・`https` で始まる値だけ。ほかは開かずに記録する | — |
| `doc/` | 作者向けの文書（R17）。`doc/COMPAT_ARCHITECTURE.md` の独自の名前の節に、キーを載せる | — |

### 4.4 主な流れ

```
ゴーストを起こす／読み直す／切り替える（＝変わった時点）
  resolve_with_shell ── ghost\master\descript.txt ──▶ MountModel.stage（並びの名前・候補・席）
        │
        ▼ 舞台の定義の役者を、起こした時に 1 回ずつ引き当てる（UI スレッドの外でよい）
  material_resolve::resolve_actor(roots, ghost_dir, stage, 名前)
        │  候補を前から: 親つきの ID → その場所だけ／短い ID → ⑴同梱 ⑵free-shell ⑶ghost\<id>\ の既定のシェル
        │  「借りないで」・かぶり・未導入 → warn! 1 行ずつ
        ▼
  引き当ての表（並びの名前 → フォルダ・素材 ID・役・作者・配布の場所）
        │                                   │
        ▼ shell-multi-compose（下流）       ▼ メニューの作者のページの枠（R11）
  並びの命令で名前を引く・役で重ね順と字の行き先を決める・席の名前で位置を引く
```

- **並びの名前の引き当て**（並びの命令が来た時に `shell-multi-compose` が引く。この単位は表と関数を渡す）:
  1. 舞台の定義に、その名前の `areka.stage.actor*` が在れば、起こした時に作った引き当ての表の答え。
  2. 無ければ、今のゴーストの同梱のシェルを、`name` → フォルダ名の順で探す（今のシェルの切り替えと同じ規則・`shell_balloon_resolve.rs:191` の `find_by_name_or_folder`。隠しのシェルも候補）。
  3. どちらも外れたら、その役者を飛ばし、記録する。
- **借り物は、舞台の定義に書いた物だけ**にする。命令の引数から、ほかのゴーストの置き場を探しに行かない。理由: ⑶ の探し方は、根の全部のゴーストの descript を読む（id を比べるため）。台本の再生の途中に UI スレッドでそれをしない。起こした時に 1 回で済ませ、借り物の一覧（作者表記）と「未導入」も、その時に出そろう。
- **id の比べ方**は、フォルダ名と同じ規則 1 つ（`same_folder_name`）を使う。`id` に `/`・`:`・`,` を含む物は、素材 ID に書けないので、フォルダ名を id にして記録する。
- **⑶ の「既定のシェル」**は、そのゴーストの今日の規則（`seriko.defaultsurfacedirectoryname`、無ければ `master`）で決める。

### 4.5 状態の持ち方・時刻

- 保存する状態は無い。`MountModel.stage` と引き当ての表は、ゴーストの部品と同じ寿命で持つ（起こすたびに作り直す）。
- 変わる時点は分かる（ゴーストを起こす・読み直す・切り替える、インストールが終わる）ので、プッシュである。引き当ての表は、その時点で作り直す。毎フレーム比べる物は無い。
- 目録（メニュー・一覧）は、今のまま、開くたびに読む。
- 時刻を扱う所は無い。1 フレーム遅らせる所も無い。

### 4.6 失敗の扱い

| 起きたこと | 動き | 記録 |
| ---------- | ---- | ---- |
| 同じ素材 ID が 2 つ | 決まった順で最初の 1 つ | `warn!` 1 行（両方のフォルダ） |
| 候補が当たらない（未導入） | 次の候補へ。尽きたら役者を飛ばす | `warn!` 1 行（並びの名前・素材 ID） |
| 「借りないで」の印 | 当たりにしない。次の候補へ | `warn!` 1 行（素材 ID・作者） |
| 独自キーの値が読めない | その行を使わない（役は `character`） | `warn!` 1 行（キー・値・ファイル） |
| 開けない URL（`http`・`https` でない） | 開かない | `warn!` 1 行 |
| 単体のフリーシェルの置き場が書けない | 今のインストールの失敗の道（`OnInstallFailure`） | 今のまま |

どれも、メッセージボックスを出さない。pasta へ知らせるイベントは足さない（7 章 A8）。

### 4.7 演劇でないふつうのゴーストで変わる所・変わらない所

- **変わらない**: 独自キーが無ければ、役は全部 `character`・舞台の定義は空・引き当ての表は空で、`scan_shells` の結果・シェルの切り替えのメニュー・`random`・`updateother`・起動は、今と同じである（R13 をテストで固定する）。`Identity` に足す欄は、読む側が居なければ何も変えない。
- **変わる（この単位が意図して変える 2 つ）**:
  1. メニューに、作者のページの枠が 1 つ増える（`craftmanurl` は正典のキーなので、今あるゴーストでも値が在れば出る）。
  2. `accept` の無い `type,shell` の書庫が、失敗でなく、`free-shell\` へ入る（SSP は、同じ書庫を今のゴーストへ入れる。正典の読みは 3 章）。

### 4.8 ほかの単位との境

- **この単位が持つ**: キーの読み・素材 ID・探す順・「借りないで」・役の値を渡すこと・メニューで役のシェルを隠すこと・`free-shell\` の置き場とインストール・作者のページの枠。
- **`shell-multi-compose` が持つ**: 舞台の窓・並びの命令を受けること・スコープの番号・役から重ね順と自動の位置を決めること・席の名前から位置を引くこと・字の行き先（トークの場所の箱）・借りたフォルダからシェルを読み込むこと。この単位は、`play` の命令を 1 つも読まない。
- 素材 ID を利用統計へ渡すことは `areka-usage-stats`、世界で 1 つであることの保証は外の U1（brief の Out のまま）。

## 5. pasta に期待する事

この単位は、さくらスクリプトの命令もイベントも足さない。期待は、descript と、並びの命令の引数の綴りに限られる。

| # | 期待 | 骨子・取り決めの brief と | 文 |
| - | ---- | ------------------------ | -- |
| P1 | 並びの命令（仮 `\![play,cast,…]`）の引数は、アクターの名前（遊ぶ人に見せる名前）である。素材 ID やフォルダ名を、命令に書かない | **合っている** | 取り決めの brief「命令の表」: 「話者の名前…既定は並びの名前で、替えたいときだけ打つ」 |
| P2 | 役・種別を、命令の引数にも、アクター辞書にも書かない。役は、シェルの descript の `areka.role` だけ | **作りは合っている。字面は食い違う** | 取り決めの brief「画面の部品はアクター」: 「『背景』『ボタン』『システム』という役や種別を、pasta にも取り決めにも作らない。」／同「舞台の窓と箱」: 「位置を明示しないアクターは、並びと役（背景 → 役者 → トークの場所）から自動で置く。」「ベースウェアが『トークの場所』の役のアクターの箱へ出す。」「シェルの切り替えのメニューは、areka が役で隠す。」 |
| P3 | 並びの命令に載る名前と、ト書きの対象（位置・透明度など）に載る名前は、同じ綴りである（areka は 1 つの表で引く） | **決まっていない**（食い違いの恐れ） | 骨子 2.4: 「対象はアクターの名前。日本語の名前は、`pasta.toml` の別名表でタグの頭の並びに置き換わる。別名の無い名前は、そのまま流す。」— 置き換わった後の綴りが、並びの名前と同じかは書いていない |
| P4 | 席の名前は、作者が書いた綴りのまま、位置の命令の引数に載る | **合っている** | `play-script-guide.md` 106 行: 「席の名前か数値。席の名前は舞台の定義に書く」 |
| P5 | 舞台の定義は、ゴーストの `ghost\master\descript.txt` に、作者が手で書く。pasta（`pasta_check release`）は、descript を書き換えずに .nar へ載せる | **合っている**（pasta の側の文は無い。areka の【N】11「descript のキーは `pasta_check release` が何もしなくても .nar に載る」） | 取り決めの brief「舞台の窓と箱」: 「席の名前と位置の対応は、舞台の定義（`descript.txt` の areka の独自キー）に書く。」「配布はゴースト（`.nar`）のまま。」 |
| P6 | 借り物の役者は、舞台の定義（`areka.stage.actor*`）に必ず書く。書いていない名前は、同梱のシェルだけから探す | **取り決めに文が無い**（足してほしい） | 取り決めの brief には、並びの名前からシェルを決める決まりが無い（「席の名前と位置の対応」だけが在る） |
| P7 | SSP でも役者や背景のシェルをシェルの切り替えのメニューから隠したい作者は、正典の `menu,hidden` を書く（見本とマニュアルに 1 行） | **足りない**（食い違いではない） | 取り決めの brief は areka の側だけを書く: 「シェルの切り替えのメニューは、areka が役で隠す。」 |
| P8 | 役者が未導入で飛ばされても、pasta は知らない（知らせるイベントは無い）。台本は、その役者が居ないまま進む | **取り決めに文が無い** | — |
| P9 | `pasta_check validate` でキーをどこまで検査するか（P2 の相談）は、正式の要件の段の最初に決める | **合っている** | この単位の brief の Constraints（【N】11） |

イベント（GET か NOTIFY か・Reference の並び）・順番・タイミングで、pasta に期待する物は無い。舞台の定義を読むのは、ゴーストを起こした時だけである。台本の途中で舞台の定義を替える口は作らない。

## 6. ほかの単位に期待する事

| 単位 | 期待・重なり |
| ---- | ------------ |
| `shell-multi-compose`（下流） | `MountModel.stage` と引き当ての表を読むだけにし、descript を自分で読まない。役の既定（書いていなければ `character`）を前提にする。同じ型（`StageDefinition`・`ShellRole`）を別に作らない。借りたフォルダ（`shell\` の下でない場所）からシェルを読み込む口は、向こうが持つ（今の `resolve_with_shell` は `ghost_root\shell\<名>` だけ・`resolve_shell.rs:180`）。引き当てを起こした時に済ませる前提（4.4）を、向こうの並びの命令の作りと合わせる |
| `property-catalog-lists` | 同じ `catalog.rs` の `Identity` を読む。`craftmanurl` は、プロパティの汎用の名前（`…shelllist(名前).craftmanurl`）の源になる。どちらが先でも、後の側が合わせる |
| `install-package-type` | 同じ `areka-nar` の `manifest.rs`・`plan.rs` を触る。`type,package` の中の「`accept` の無い `type,shell`」も、同じ判定で `free-shell\` へ行く |
| `dev-folder-alias` | 探す順の先頭の別名は、根の並び（`BasewareRoots`）の口を通せば、そのまま効く。`free-shell\` に別名を足すかは向こうで決める |
| `areka-usage-stats`・外の U1 | 素材 ID の形（4.3）を写す。種別は 3 つ（`play` は無い・7 章 A3） |
| 演劇のパッケージ（roadmap の受け口） | 種別の議論（`type,ghost`＋印か `/play/` か）は、取り決めの brief の「配布はゴースト（`.nar`）のまま」で、ゴーストの形に決まったと読む。印は作らない（7 章 A3） |
| `extra-character-windows`・`multi-ghost-runtime` | 触るファイルの重なりは見つからなかった（brief を `ghost-material-metadata`・`free-shell`・フリーシェルで grep して 0 件） |
| pasta の P2 | キーの一覧（4.2）を渡す。名前は仮なので、正式の要件で確定してから |

順番: この単位は、v0.2.0 の描画の作り直しを待たない。`shell-multi-compose` より前に着地できる。

## 7. 置いた仮定

- **A1**: 独自キーの頭は `areka.`。名前は全部仮（brief「名前は要件の段で決める・たとえば `areka.` で始める」）。
- **A2**: 舞台の定義と席は、ブレスでなく、平らな番号つきのキーで書く。brief の「席の要件は、正典の `surface*` ブレスと同じ書き方で演劇の descript に並べ」（【N】6）を、3 章の事実（descript の読み手はブレスを知らない）で読み替えた。開発者「まああとの議論で変わるかもしれないけど」。
- **A3**: 素材 ID の種別は `ghost`・`shell`・`balloon` の 3 つ。`play` と「演劇である」の印は作らない。演劇はゴーストの形で配り（取り決めの brief）、ベースウェアは「演劇」を知らない（骨子 1 章）ので、印を読む側が居ない。brief の探す順の `/play/{play}/shell/{shell}/` は、`<今のゴースト>\shell\{shell}\` と読む。
- **A4**: 「席」は、取り決めの brief の意味（名前の付いた位置）で読む。brief の「席の要件」（差し替えるシェルに求める条件）と、「この席は替えさせない」の錠、利用者による差し替えは、使う側が骨子・取り決めに居ないので、作らない（brief が「提供する能力」を外したのと同じ理由）。
- **A5**: 基準の大きさのキーは作らない（取り決めの brief が 1920×1080 に固定した）。
- **A6**: 役は、シェル（素材）の属性である。並びの名前ごとに役を書き換える口は作らない。役の値は 3 つで、既定は `character`。
- **A7**: 「借りないで」は、親でないゴーストから借りる時だけ効く。借りたシェルの中の画像の再配布などは、扱わない（ライセンスは文字列を見せるだけ）。
- **A8**: 未導入・「借りないで」を、pasta へイベントで知らせない。取得先の URL を示す口も作らない（舞台の定義に取得先を書くキーを足さない。記録に素材 ID を出すだけ）。
- **A9**: `accept` の無い `type,shell` を、単体のフリーシェルと読む。install.txt に独自キーを足さない。
- **A10**: SSP は、descript.txt の `areka.` で始まる知らないキーを読み飛ばす（brief の前提。正典に文は無く、実機でも確かめていない）。
- **A11**: 作者のページの枠は、今のメニュー（Win32 のメニュー）に足す。About の画面は作らない（メニューの作り直しは windows-reactor 待ち）。

## 8. 決まっていない所（開発者に諮る問いの形）

1. **役は、シェルの属性か、並びの名前（アクター）の属性か。** 取り決めの brief は「役のアクター」と書き、この設計はシェルの descript に書く（A6）。→ アクターの属性なら、`areka.role` を消して、舞台の定義の行（`areka.stage.actor*`）に役の欄を足す。役の無い借り物のシェルを背景や箱に使えるようになる。
2. **並びの名前からシェルを決める決まりは、これでよいか**（舞台の定義の `areka.stage.actor*` → 無ければ同梱のシェルの `name` → フォルダ名。借り物は舞台の定義に書いた物だけ）。→ 命令の引数から根の全部を探す形にするなら、引き当てを起こした時に済ませる作り（4.4）をやめ、目録の控えを持つ作りになる。
3. **舞台の定義をブレスでなく平らなキーで書くこと**（A2）。→ ブレスを通すなら、descript の読み手（`kv`）にブレスを足し、SSP がブレスの中の行をどう読むかを実機で測る仕事が増える。別のファイル（例 `stage.txt`）にする道も開く（brief は「別のファイルは作らない」）。
4. **`play` の種別と「演劇である」の印を作らないこと**（A3）。→ 作るなら、目録にゴーストと演劇を分ける欄が増え、メニューの「ゴースト」の枠の並べ方と `areka-usage-stats` の集め方が変わる。
5. **席の要件・錠・利用者による差し替えを外すこと**（A4）。→ 残すなら、舞台の定義に条件のキーが増え、借りたシェルと突き合わせる検査がこの単位に入る（+3〜4 タスク）。
6. **`accept` の無い `type,shell` を単体のフリーシェルと読むこと**（A9）。→ 印を別に持つなら、install.txt に独自キーが 1 つ増え、今の「`accept` が無ければ失敗」は残る。
7. **役で隠すシェルを、`menu,hidden` と同じ扱いにすること**（`random` と `updateother` の名前の引き当てからも外れる）。→ メニューだけにするなら、隠す所を `scan_shells` でなく `shell_frame.rs` に置く。
8. **未導入の役者を pasta へ知らせる口が要るか**（A8）。→ 要るなら、イベント（GET）と Reference の並びを取り決めに足し、`ALLOWED_EVENT_IDS` の決まりを広げる仕事が、この単位か受け口に増える。
9. **作者のページの枠を今のメニューに足すか、windows-reactor のメニューの作り直しを待つか**（A11）。→ 待つなら、R11 はこの単位から外れ、目録の欄（R10）だけが残る。

## 9. 差し戻しが要りそうな所

上位の決定（骨子・取り決めの brief）へ。

1. **「役」の語の書き分け（字面の矛盾。作りは変わらない）。**
   - ぶつかる文: 取り決めの brief「画面の部品はアクター（開発者の決定）」の「『背景』『ボタン』『システム』という役や種別を、pasta にも取り決めにも作らない。」と、同「舞台の窓と箱」の「位置を明示しないアクターは、並びと役（背景 → 役者 → トークの場所）から自動で置く。」「ベースウェアが『トークの場所』の役のアクターの箱へ出す。」「シェルの切り替えのメニューは、areka が役で隠す。」
   - 骨子 2.7 は「役や種別を、pasta に作らない」と書き、「取り決めにも」とは書かないので、骨子とはぶつからない。
   - 直す向き: 取り決めの brief の前の文を、「アクターの定義（pasta の辞書）と、さくらスクリプトの命令に、役や種別を作らない」に絞る。後の文に、「役は、シェルの descript.txt に素材の作者が書く、ベースウェアだけが読む宣言で、値は背景・役者・トークの場所の 3 つ。書いていなければ役者」を足す。
2. **並びの名前からシェルを決める決まりが、取り決めに無い。** 取り決めの brief は「席の名前と位置の対応は、舞台の定義…に書く」とだけ書く。並びの名前（遊ぶ人に見せる名前）と、シェル（同梱・借り物）の対応も、舞台の定義に書く、と足してほしい（5 章 P6）。あわせて、並びの名前とト書きの対象の名前が同じ綴りであること（P3。骨子 2.4 の「別名表でタグの頭の並びに置き換わる」の後の綴り）。
3. **構造の差し戻しは無い。** 「配布はゴースト（`.nar`）のまま」「舞台の定義は descript.txt の areka の独自キー」は、この設計で受けられる。ただし、SSP が descript の知らないキーを読み飛ばすこと（A10）が実機で崩れたら、「舞台の定義を descript.txt に書く」が差し戻しになる。

areka の中の brief への直し（上位の決定ではない）: この単位の brief の「席の要件は…ブレスと同じ書き方」「演劇」の種別・`/play/` の探す順・「席の要件」「錠」は、取り決めの最終形の後では古い（7 章 A2〜A4）。正式の要件の段で読み替える。

## 10. 規模の見立て

- **16〜21 タスク。** 内訳の見立て: 読み手（素材 ID・シェルのキー・舞台の定義）3／目録（素性の拡張・`free-shell` の走査と和・役で隠す）3／引き当て（探す順とかぶり・「借りないで」・役者の候補）4／`MountModel` と起こした時の結線・インストールの後の作り直し 2／単体のフリーシェルのインストール 2〜3／作者のページの枠 2／作者向けの文書と独自の名前の台帳 2／ふつうのゴーストが変わらないことの固定 1。
- brief の見立ては M〜L（12〜20）。**上の端か、少し超える。** 増えた分は、舞台の定義（並びの名前の対応と席）である（brief を書いた時には、席は「要件」で、舞台の定義という物は無かった）。減った分は、席の要件・錠・`play` の種別（A3・A4）。
- ノベルに関わる芯（読み手・目録の役・引き当て・`MountModel`）は 11〜13 で、v0.2.0 の描画を待たずに作れる。

## 11. 確かめた物と机の上だけの物

- **実行して確かめた物: 0。** ビルド・テスト・areka と SSP の起動は、していない。
- **コードを読んで確かめた物**（3 章の `path:行`）: descript の読み手が平らな表であること・ゴーストの descript の読みと `MountModel`・目録の素性の 7 項目と `menu,hidden` の除外・`list_shells` の 3 つの使い手・根の和の作り・メニューの枠の数・インストールの種別とシェルの宛先・`accept` の無いシェルが今は失敗であること・descript に areka の独自キーがまだ無いこと（grep）。
- **ukadoc で確かめた物**: `id`・`craftmanurl`（descript とプロパティ）・`menu,hidden`・`accept` と install.txt の概要（3 章の引用）。
- **机の上だけの物**:
  - SSP が descript.txt の `areka.` で始まるキーを読み飛ばすこと（A10）。正典に文が無く、実機でも測っていない。**この単位でいちばん危ない仮定**である。
  - SSP が、`accept` の無い `type,shell` を今のゴーストへ入れること（ukadoc の「既定値: 挙動なし」「accept指定が利用できる」からの読み）。
  - 完了 `baseware-root-layout` の要件 2.9 の文面（`catalog.rs:113` のコメントと brief で読んだだけで、完了の requirements.md は開いていない）。
  - pasta の別名表が、アクターの名前を何に置き換えるか（骨子 2.4 の 1 文だけで、pasta のコードは読んでいない）。
  - `pasta_check release` が descript を書き換えずに .nar へ載せること（areka の【N】11 の文だけ）。
  - 起こした時の引き当てを UI スレッドの外へ出す結線の形・インストールの後の作り直しの入口・メニューの枠を足したときの字の資源（`captions`）の直しは、コードを読んでいない見立てである。
  - 規模の数は、内訳からの見立てである。
