# ギャップ分析: areka-P0-sylphya-set-ledger

> 実施 2026-09-11（`/kiro-validate-gap`）。対象は確定済みの `requirements.md`（要件 1〜9）と現行コードベース。
> 本書は**情報と選択肢**を出すものであり、最終決定は行わない。決定は要件ディスカッションで行う。
>
> 開発者裁定で**既に固定されている前提**（選択肢として扱わない）:
> - `currentghost.seriko.zorder` は先送りのまま登記しない。`crates/areka/src/placement/zorder_property_deferral_tests.rs` の 3 本（t_zpd10／t_zpd30／t_zpd40）は緑のまま、同ファイルは編集対象外。
> - サウンド 18 葉は**既存の表へ相乗り**する。`vocab/` に新しい公開 const を作らない（t_zpd12 が赤になる）。
> - 編集の及ぶ範囲は `crates/areka-sylphya/src/vocab/dotted.rs`（＋同クレートの兄弟テスト）・`doc/ukadoc-coverage/ledger/property.toml`・`doc/COMPAT_ARCHITECTURE.md` §8 の 3 つだけ。

---

## 1. 結論の要約

1. **語彙表の追加そのものは小さい**。触るのは配列 2 本（`SET_EFFECTIVE` 21→25・`GENERIC_PROP_NAMES` 17→30）と、その件数を固定している 2 ファイルの記述だけで、新しい型も新しい関数も要らない。規模は **S**、技術リスクは **Low**。
2. **要件 1.3／1.4 は、実際の書込キーに対しては成立しない**。名前の解決は `classify_set`（`crates/areka-sylphya/src/actor.rs:136-147`）が**キー文字列全体の完全一致**で `SET_EFFECTIVE` を引くため、`currentghost.sound(要素名).pause` は `pause` の登記とは一度も突き合わされない。これは本機能が作る欠陥ではなく、**既存 21 項が全て同じ状態**にある（`surface.num` も実キー `currentghost.scope(ID).surface.num` とは一致しない）。→ 議題 ②。
3. **本当のリスクは「葉の名前の巻き添え」**。`GENERIC_PROP_NAMES` は**どんなキーでも末尾のひと区切りが一致すれば**正準語彙とみなす（`actor.rs:157-161`）。ここへ `id`・`loop`・`error`・`duration`・`preload` のような一般的な語を足すと、ゴーストが自由に使っていた `myplugin.id` のようなキーが「保存される」から「受理して捨てる」へ変わる。要件 3.7 が列挙を求め、要件 7.2 が変えるなと言っている、まさにその衝突である。→ 議題 ③。
4. **`meta.` を付けた綴りを採ると、8 葉の巻き添えは完全に 0 になる**。区切り記号を含む名前は 1 区切りの名前と等しくなり得ないため、`meta.album` は末尾一致を一度も起こさない。既存の `sakura.bind.menu`・`shiori.変数名` と同じ書き方であり、正典の見出しとも台帳 id とも綴りが揃う。→ 議題 ①（推奨は `meta.` 付き）。
5. **証跡の URL を足すと、編集範囲の外にある報告書の数字が古びる**。`/// ukadoc: <項目 URL>` の行は網羅調査の証拠抽出器が読む（`crates/ukadoc-survey/src/evidence/resolve.rs:1-18`）。`doc/ukadoc-coverage/report/summary.md:142` の property の「証拠あり 2」が 6〜24 へ動くが、同ファイルは要件 9.1 の範囲外。→ 議題 ⑥（要件 9.3 の報告事案）。

---

## 2. 現況の実測（すべて file:line で裏取り済み）

### 2.1 語彙台帳の現物

| 表 | 定義 | 件数 | 意味 |
| --- | --- | ---: | --- |
| `DOTTED_ROOTS` | `vocab/dotted.rs` のルート枝の配列 | 10 | 点付きキーの根。ここに載る根を持つキーは正準語彙 |
| `GENERIC_PROP_NAMES` | 同ファイルの汎用名の配列 | 17 | **末尾のひと区切り**がここに載れば正準語彙（かつ SET 無効） |
| `SET_EFFECTIVE` | 同ファイルの SET 有効群の配列 | 21 | **キー文字列の完全一致**でだけ引かれる |
| `FLAT_VOCAB` | `vocab/flat.rs` | 26 | 本機能の対象外 |
| `SHIORI_RESOURCE_IDS` | `vocab/shiori_resource.rs` | 159 | 本機能の対象外 |

`vocab/` の公開 const は現物 8 本（上記 5 ＋ `SYNTAX_RECORDS`・`EXT_EVENT_GET`・`EXT_EVENT_SET`）。この 8 本が「走査する 5 本」と「走査しない 3 本」のどちらかへちょうど 1 回現れることを t_zpd12 が見張っている（`zorder_property_deferral_tests.rs` の該当テスト）。**新しい公開 const を足せば必ず赤になる**ので、相乗りは選択ではなく制約である。

### 2.2 名前の解決の実際（要件 1.3／1.5・議題 ② の根拠）

`classify_set`（`crates/areka-sylphya/src/actor.rs:136-147`）は 3 段で判定する。

1. `SET_EFFECTIVE.iter().any(|(k, _)| *k == key)` ——**キー文字列そのものの完全一致**。前方一致も末尾一致も無い。
2. `is_canonical_vocab(key)`（同 `:150-165`）—— 解釈したうえで「根の名前が `DOTTED_ROOTS` に載る」**または**「末尾の区切りの名前が `GENERIC_PROP_NAMES` に載る」なら `NotSettable`。
3. どちらでもなければ `StoreWrite`（asker 別の区画へ保存する）。

キーの解釈（`crates/areka-sylphya/src/key.rs:104-113`）は単純に `.` で分割する。したがって **1 つの区切りの名前に `.` は入り得ない**。

この形から出る帰結（いずれも現行コードで確定）:

- 実キー `currentghost.sound(要素名).pause` は、段 1 で `pause` と一致しない（文字列が違う）。段 2 で根 `currentghost` が `DOTTED_ROOTS` に載るので `NotSettable` になる。**`pause` を `SET_EFFECTIVE` へ足しても、この結果は 1 ビットも変わらない。**
- 同じことが既存 21 項にも当てはまる。台帳 `property.toml` が「正典 SET 有効」と記録する 26 行のうち、見出しが末尾形そのものなのは `menu`・`sakura.bind.menu`・`kero.bind.menu`・`char*.bind.menu`・`pause`・`playing`・`position` の 7 行だけで、残りは `currentghost.` や `currentghost.scope(ID).` の前置きを持つ。
- 末尾形は**機械的に切り出したものではない**。台帳行 `currentghost.seriko.cursor.scope(ID).mouse????list.index(ID2).path` の末尾のひと区切りは `path` だが、語彙表の綴りは `seriko.cursor.path` である。要件 1.5 の「先頭の親枝とセレクタを除いた末尾の形」は**人が縮めた綴り**であり、規則から導ける形ではない。

`crates/areka-sylphya/src` 全体に、フルキーを末尾形へ直す処理は**存在しない**（`actor.rs:159` の `.last()` は段 2 の葉判定のみ・`reader.rs`／`key.rs` に該当処理なし）。

### 2.3 照合元の台帳（`doc/ukadoc-coverage/ledger/property.toml`・188 件）

**「正典 SET 有効」と記録する行は実測ちょうど 26 行**（要件 4.2 の主張と一致）。内訳:

| # | 台帳 id（`ukadoc:list_propertysystem:` 以下） | 語彙表での扱い |
| ---: | --- | --- |
| 1〜11 | `char*.bind.menu`／`kero.bind.menu`／`menu`／`sakura.bind.menu`／`currentghost.mousecursor` 系 6／`currentghost.balloon.mousecursor` 系 4 | 既存 21 項（末尾形で 1 対 1） |
| 12〜14 | `currentghost.scope(ID).{animation.num, seriko.defaultsurface, surface.num}` | 既存 21 項 |
| 15・16 | `…cursor…index(ID2).path` と `…cursor…(当たり判定名).path` | **2 行 → 語彙表 1 項**（`seriko.cursor.path`） |
| 18・19 | `…tooltip…index(ID2).text` と `…tooltip…(当たり判定名).text` | **2 行 → 語彙表 1 項**（`seriko.tooltip.text`） |
| 17 | `currentghost.seriko.sticky-window` | **本機能が新規登記** |
| 20 | `currentghost.seriko.zorder` | **先送り（登記しない）** |
| 23〜25 | `pause`／`playing`／`position` | **本機能が新規登記** |

要件 4.2 の 2 通りの数え方は実測と合う: 26 − 1（zorder）= 25 行 → 15/16 と 18/19 が各 1 項へ縮むので 23 項 → areka が正典より先取りしている 2 項（`seriko.cursor.name`・`seriko.tooltip.name`。台帳側に対応する「SET 有効」行が無いことを確認済み）を足して 25。

`.ext.拡張プロパティ名` 系 4 行が 26 に入っていない理由も台帳本文に明記されている——「印は無い・ext 亜枝の中継形のため 26 件には数えない」（`property.toml` の `activeghostlist.index(ID).ext.…` 行の note）。要件 4.4 はこの文言をそのまま引ける。

### 2.4 サウンド語彙族 18 葉の現物（議題 ① の根拠）

台帳では、サウンド族の頭 3 行（`currentghost.sound.count`・`currentghost.sound.index(ID).サウンドプロパティ名`・`currentghost.sound(要素名).サウンドプロパティ名`）とは**別に、葉の名前が独立した行として 18 行**ある。全 18 行の note に「前置き: currentghost.sound の族の頭 2 本（index(ID) で位置を指す形・要素名で指す形）の下で使う」と書かれており、**葉であることが台帳自身によって明示されている**。

| # | 台帳 id | 正典 URL（`…/list_propertysystem.html#` 以下） | 正典 SET | `GENERIC_PROP_NAMES` に既出か |
| ---: | --- | --- | :-: | :-: |
| 1 | `duration:1` | `duration:1` | — | 新規 |
| 2 | `error:1` | `error:1` | — | 新規 |
| 3 | `id:1` | `id:1` | — | 新規 |
| 4 | `loop:1` | `loop:1` | — | 新規 |
| 5 | `name:2` | `name:2` | — | **既出**（`name`・`dotted.rs:38`） |
| 6 | `path:2` | `path:2` | — | **既出**（`path`・`dotted.rs:43`） |
| 7 | `preload:1` | `preload:1` | — | 新規 |
| 8 | `pause:1` | `pause:1` | **有効** | （SET 有効群へ） |
| 9 | `playing:1` | `playing:1` | **有効** | （SET 有効群へ） |
| 10 | `position:1` | `position:1` | **有効** | （SET 有効群へ） |
| 11〜18 | `meta.{album, albumartist, artist, artwork, genre, title, track, year}:1` | 同 | — | 新規 8 |

**要件 4.6 が要求する件数の導出（実数え）**:

- 18 葉 − SET 有効 3 葉（`pause`・`playing`・`position`）= **設定できない正準語彙 15 葉**。
- 15 葉のうち `name`・`path` は既に `GENERIC_PROP_NAMES` に載っている（同じ綴りなので二重登記になる）→ **新規は 13 葉**。
- したがって `GENERIC_PROP_NAMES` は **17 → 30**。
- `SET_EFFECTIVE` は 21 ＋ 4（`seriko.sticky-window`・`pause`・`playing`・`position`）= **25**（要件 4.1 と一致）。

なお `name:2`／`path:2` は `name:1`／`path:1`（汎用名としての行）とは**別の台帳行**である（`property.toml:1717`／`:1733`／`:1752`／`:1768`）。正典の側では別項目、areka の語彙表の側では同じ 1 つの綴り——この非対称は要件 4.6 の「導出を文書に明記する」で必ず書く必要がある。

### 2.5 件数を固定している記述の実在位置（要件 4.7）

要件 4.7 は「SET 有効群の 21 は計 4 箇所」と実測を記しているが、**再実測では literal と関数名を合わせて 6 箇所**ある。

`crates/areka-sylphya/src/vocab/dotted.rs`:
- `:188` 説明文「基本 3＋mousecursor 10＋seriko.cursor/tooltip 4＋menu 4 = 21」
- `:190` **テスト関数の名前** `set_effective_has_21_entries`
- `:191` `assert_eq!(SET_EFFECTIVE.len(), 21)`
- `:209` `let required: [&str; 21]`

`crates/areka-sylphya/src/ledger_key_determinism_tests.rs`:
- `:210` 説明文「全 21 項（基本 3＋…）」
- `:211` `let required: [&str; 21]`

汎用名の 17 はさらに多く、**編集範囲の中に 8 箇所**ある:
- `dotted.rs:1`（モジュール冒頭の「汎用名 17」）・`:30`（同一行に 2 回＝説明文と `len() == 17`）・`:146`（関数名 `generic_prop_names_has_17_entries`）・`:147`（assert）・`:164`（`[&str; 17]`）
- `ledger_key_determinism_tests.rs:195`（説明文「汎用名 17」）・`:203`（assert）

**編集範囲の外**にも 17 の写しがある:
- `.kiro/steering/roadmap` 系ではなく `.kiro/steering/structure.md:288`「`dotted.rs` ルート枝 10＋汎用名 17＋SET 意味論」
- `.kiro/specs/areka-P0-property-catalog-lists/brief.md:21` と `:73`（他 spec の brief）

→ 議題 ⑤。

### 2.6 担当欄が空の 2 行（要件 6）

`owner = ""` は実測ちょうど 2 行（`property.toml:952` と `:1068`）。

- `:952` = `currentghost.seriko.sticky-window`（`status = "vocabulary-only"`・`introduced = "2.8.78"`・note 末尾に「裁定待ち: …争点は値の導出の担い手と、語彙表の 1 行の持ち主。」）
- `:1068` = `currentghost.seriko.zorder`（`status = "degraded"`・`introduced = "2.8.78"`・note に「転記元: doc/COMPAT_ARCHITECTURE.md:207」と「裁定待ち: …3 spec が争う」）

網羅調査の検査（`crates/ukadoc-survey/src/check/structure.rs:109-146`）は **`owner` の中身を一切見ない**（見るのは id がカタログに実在するか・ページの担当が合うか・id の並び順）。したがって:

- 担当欄に何を書いても検査は赤にならない＝**`areka-P0-zorder-property` という綴りが実在する spec を指しているかは誰も見張っていない**（実在は確認済み: `.kiro/specs/areka-P0-zorder-property/brief.md`）。
- 要件 6.5 の「空きが 0 であることを示せる」を**判定する**検査は現状どこにも無い。要件 4.8 が「印字するだけでなく判定させよ」と言っている以上、この 0 も判定形で持つ必要があるが、その置き場所が編集範囲に無い。→ 議題 ④。

### 2.7 台帳の note に残る古い指し先（要件 6.4 との衝突）

`property.toml` には「語彙表だけを触る spec: areka-P0-property-query-channels」という行が **22 行**ある。内訳は本機能が登記するサウンド 18 葉と、範囲外の `.ext.*` 4 行。

本 spec は `property-query-channels` から語彙台帳の担当を切り出して独立したもの（`brief.md` 冒頭）なので、**サウンド 18 行の指し先は本 spec が着地した時点で古くなる**。しかし要件 6.4 は「6.1〜6.3 以外の行の内容を変更しない」と定めており、直せない。→ 議題 ⑦。

### 2.8 互換記録 §8（要件 8）

`doc/COMPAT_ARCHITECTURE.md` の §8 は `:122` から始まる 4 列の表。`currentghost.seriko.zorder` の行は **`:207` の 1 行だけ**（`:206` は `seriko.zorder` の descript キーについての訂正行で、プロパティの行ではない）。`:207` は既に「先送りした語彙の正本は追跡 spec `areka-P0-zorder-property` の brief の『完全語彙』節」と書いており、要件 8.3 が言う「語彙を書き写さない」は**既に守られている**（本機能はこれを持ち越さない＝新しく書き写さない、と読める）。

要件 8.2「既存の記録行を書き換えずに足せる形」は、`:207` の直後に 1 行足す形で満たせる。表の 4 列（項目／areka の裁量／根拠／出典 spec）の体裁を踏襲すればよい。

### 2.9 既存テストの影響（要件 7.4／7.5 の切り分けの下地）

現行で `classify_set` に渡されているキーの全数（本番・テスト）:

- `surface.num`・`menu`・`seriko.defaultsurface` → RuntimeCommand
- `myplugin.customstate`・`foo.bar.baz`・`mystuff.custom`・`path.to.key` → StoreWrite
- `baseware.name`・`system.foo`・`username`・`currentghost.seriko.zorder` → NotSettable
- `a..b`・`""` → StoreWrite（解釈不能）

**このうち、本機能の登記で分類が変わるものは 1 つも無い**（末尾が `duration`／`error`／`id`／`loop`／`preload` のキーは既存テストに存在しない。`path.to.key` の末尾は `key`）。したがって要件 7.4 の切り分けで「本機能が挙動を壊した」側へ落ちる既存テストは現時点では見当たらず、赤くなるのは件数を固定している記述だけ（＝「台帳が古い前提を固定していた」側）になる見込み。

---

## 3. 要件 → 資産の対応表

| 要件 | 受け皿となる資産 | 判定 |
| --- | --- | --- |
| 1.1／1.2 登記 | `SET_EFFECTIVE`（`dotted.rs:72-99`） | **あり**（配列に 4 要素追加） |
| 1.3／1.4 分類 | `classify_set`（`actor.rs:136`）・`SylphyaCore::apply` の Set アーム（`actor.rs:299-350`） | **Constraint**：末尾形はフルキーと一致しない（§2.2）→ 議題 ② |
| 1.5 綴りの導き方 | 既存 21 項の綴り | **Unknown**：規則ではなく人の縮め方（§2.2 末尾）→ 議題 ② |
| 1.6 既存 21 項の不変 | 配列の先頭から順に保持 | **あり**（末尾追加で満たせる） |
| 2.1〜2.5 先送りの尊重 | `zorder_property_deferral_tests.rs` の t_zpd10／t_zpd30／t_zpd40 | **あり**（`zorder` を足さなければ緑のまま） |
| 3.1〜3.3 サウンド 18 葉の相乗り | `GENERIC_PROP_NAMES`（`dotted.rs:37-55`） | **あり**。ただし表の名前は「汎用プロパティ名」で、サウンド固有の葉を入れると表題と中身が食い違う → 議題 ① |
| 3.4／3.5 分類・参照 | `is_canonical_vocab`・`reader.rs` | **あり**（実キーは根 `currentghost` で既に `NotSettable`／`NotFound`） |
| 3.6 名簿検査 | t_zpd12（公開 const は 8 本のまま） | **あり**（新しい公開 const を作らない限り緑） |
| 3.7 巻き添えの列挙 | —— | **Missing**：実測と列挙が要る（§4 議題 ③） |
| 4.1〜4.7 件数と導出 | `dotted.rs` のテスト群・`ledger_key_determinism_tests.rs:200-245` | **あり**。ただし写しの数が要件の実測（4）より多い（6／8）→ 議題 ⑤ |
| 4.8 判定する検査 | 既存の `assert_eq!` 群 | **あり**（既に判定形。印字だけの箇所は無い） |
| 4.9 重複の検出 | `set_effective_no_duplicate_keys`・`generic_prop_names_no_duplicates` | **あり**（集合サイズ比較が既にある） |
| 4.10 項目ごとの分類の確認 | `actor_tests.rs:16-57` の 5 本 | **あり**（同じ形で 4 項目ぶん足す） |
| 5.1〜5.4 正典 URL の証跡 | `catalog.toml`（URL 欄）・既存の `/// ukadoc:` 書式（`placement/source.rs:45` ほか） | **あり**。ただし証拠抽出器が読む（§2.9 の外）→ 議題 ⑥ |
| 6.1〜6.4 担当欄 | `property.toml:952`／`:1068` | **あり**（2 行の `owner` と note を書き換え） |
| 6.5 空き 0 の提示 | —— | **Missing**：判定する置き場所が編集範囲に無い → 議題 ④ |
| 7.1〜7.5 挙動の不変 | §2.9 の全数 | **あり**（既存テストは 1 本も分類が変わらない）。ただし要件 7.2 と議題 ③ が衝突 |
| 8.1〜8.4 相互参照 | `COMPAT_ARCHITECTURE.md:207` | **あり**（直後に 1 行追加） |
| 9.1〜9.3 編集範囲 | —— | **Constraint**：議題 ⑤（steering）・⑥（報告書）・⑦（台帳 note）が範囲外に触れたくなる |

---

## 4. 設計議題（要件ディスカッションへ送るもの）

### 議題 ① `meta.` 系 8 葉の綴り —— `meta.album` か `album` か【要件 4.6 が明示を求めている】

**利用者から見える差**: `meta.` を外すと、ゴーストが自由に使っていた `どこかの名前.album` のようなプロパティが「保存されて読み戻せる」から「受理して捨てる」へ変わる。`meta.` を付ければこの変化は起きない。

| 案 | 綴り | 正典・台帳との一致 | 末尾一致の巻き添え | 既存の書き方との一致 |
| --- | --- | --- | --- | --- |
| **甲（推奨）** | `meta.album` … `meta.year` | 台帳 id・正典見出し・カタログ title の 3 つと逐語一致 | **0 件**（区切りを含む名前は 1 区切りの名前と等しくならない＝`key.rs:104-113`） | `sakura.bind.menu`・`kero.bind.menu`・`char*.bind.menu`・`shiori.変数名` と同型（既に 4 例ある） |
| 乙 | `album` … `year` | 綴りが正典から離れる（台帳へ戻る手がかりが URL 注記だけになる） | **8 件ぶん発生**。`title`・`year`・`id`・`artist` などは一般的すぎ、自由なキーを広く巻き込む | `name`・`path`・`index` と同型 |

甲を採ると「載せたが末尾一致は起こさない名前」が 8 つ増えるが、既存の 4 例（`sakura.bind.menu` 等）と同じ性質であり、**この表は照合の材料であると同時に語彙の記録でもある**という既存の建て付けから外れない。

**この議題が決まらないと 4.6 の件数の導出が書けない**（甲でも乙でも数は 30 で同じだが、導出の文言が変わる）。

### 議題 ② 末尾形は実際の書込キーと突き合わされない —— 要件 1.3／1.4 の読み方【最重要】

**事実**（§2.2）: 名前の解決は完全一致なので、`currentghost.sound(要素名).pause` は `pause` の登記と一度も比較されない。分類は根 `currentghost` によって `NotSettable` のままである。**既存 21 項もすべて同じ**で、`surface.num` が RuntimeCommand になるのは「`surface.num` という文字列そのものを渡したとき」だけである。

**利用者から見える差**: 本リリースではどちらでも**何も変わらない**（RuntimeCommand も NotSettable も、値は反映されず記録が 1 行残るだけ）。差が出るのは `\![set,property,…]` の経路（`areka-P0-property-query-channels`）が着地して、フルキーが実際に流れ込むようになったとき。

選択肢:

- **案 A（記録するだけ・推奨）**: 語彙表は「正典の名前の台帳」であると割り切り、末尾形のまま登記する。フルキーを末尾形へ直す処理（正規化）が要ることを `dotted.rs` の説明文に 1 段落で残し、その担い手が `property-query-channels` であることを書く。本 spec は正規化を作らない（要件 9.1 の範囲外＝`actor.rs` に触れる必要が出る）。
  - 要件 1.3／1.4 は「末尾形のキーを渡したときの分類」として読む。これは既存 21 項に対する 4.10 の検査と同じ読み方であり、`actor_tests.rs:16-24` の既存 3 本がまさにその形。
- 案 B: 語彙表へフルキーの雛形（`currentghost.sound(要素名).pause` のような形）で登記する。→ 要件 1.5（既存と同じ末尾形）に正面から反し、要件 1.6（既存 21 項を変えない）とも噛み合わない（既存が末尾形のままなら表の中に 2 つの書き方が混在する）。**採らない方が良い**。
- 案 C: 本 spec で `classify_set` に正規化を足す。→ 編集範囲（要件 9.1）の外。要件 9.3 に従い「行わず報告する」対象。

**A を採る場合でも、要件 1.3／1.4 の条文が「実キーでも RuntimeCommand になる」と読めてしまう点は、条文の言い回しの確認が要る。**

### 議題 ③ 巻き添えで分類が変わる名前の列挙【要件 3.7 が明示を求めている・要件 7.2 と衝突】

`GENERIC_PROP_NAMES` は**どんなキーでも末尾のひと区切りが一致すれば**正準語彙にする。新規 13 葉のうち、区切りを含まない 5 つ（議題 ① で甲を採る場合）が巻き添えを起こす。

| 足す名前 | 変わるキーの形 | 変化 | 利用者から見える意味 |
| --- | --- | --- | --- |
| `duration` | 根が `DOTTED_ROOTS` に無く末尾が `duration` のキー（例 `myplugin.duration`・単独の `duration`） | StoreWrite → NotSettable | **保存されて読み戻せていた値が、保存されなくなる**（呼出は成功したまま・警告が 1 行出る） |
| `error` | 同上（例 `mytool.error`） | 同上 | 同上 |
| `id` | 同上（例 `myplugin.id`） | 同上 | 同上。**`id` は最も使われやすい語** |
| `loop` | 同上 | 同上 | 同上 |
| `preload` | 同上 | 同上 | 同上 |

（`name`・`path` は既出なので変化なし。`meta.*` は甲なら変化なし。`pause`・`playing`・`position` は `SET_EFFECTIVE` へ入るので完全一致だけ＝**その 3 つの文字列そのもの**を渡したときに StoreWrite → RuntimeCommand へ変わる。これも要件 7.2 が言う「正準語彙外の自由な名前の扱い」の変化に当たる。）

**現時点で実際に赤くなる既存テストは 0 件**（§2.9）。だが要件 7.2 は「変えない」と定めており、上の 8 つ（`duration`・`error`・`id`・`loop`・`preload`・`pause`・`playing`・`position`）は確実に変える。**要件 3.7 が要求している列挙は、そのまま要件 7.2 の例外の申請になる。**

選択肢:

- **案 甲（推奨）**: 8 件を要件 3.7 の列挙として design に書き切り、要件 7.2 の「変えない」は**既存 21 項と、この 8 つを除く自由な名前について**と読む。理由は「正典が正準語彙だと定めている名前を areka が自由なキーとして扱い続けるほうが正典からの乖離が大きい」で通る。既に `menu` という一般的な語が同じ形で載っている先例がある（`GENERIC_PROP_NAMES` と `SET_EFFECTIVE` の双方）。
- 案 乙: 巻き添えの起きる 5 葉（`duration`・`error`・`id`・`loop`・`preload`）だけ `sound.duration` のような接頭辞を付けて登記する。→ 正典にも台帳にもその綴りは無く、要件 1.5／5.2 の「台帳と綴りを揃える」から外れる。登記の目的（後続が台帳から辿れる）を損なう。
- 案 丙: 5 葉を登記しない。→ 要件 3.1（18 葉すべて）に反する。

### 議題 ④ 「担当欄の空きが 0」を**判定する**検査の置き場所【要件 6.5 ＋ 4.8】

要件 6.5 は 0 を「示せる」ことを求め、要件 4.8 は「印字するだけでなく判定させよ」と定める。だが:

- 網羅調査の検査（`check/structure.rs`）は `owner` の中身を見ない（§2.6）。そこへ規則を足すのは編集範囲の外（`crates/ukadoc-survey/`）。
- 編集範囲の中で TOML を読めるのは `crates/areka-sylphya/src/` の兄弟テストだが、**sylphya から `doc/` の台帳を読むテストを置くと、完了時のアーカイブ移動や doc の改稿でそのテストが壊れる**（既知の失敗様式＝`kiro-complete` のアーカイブがコードからの spec 文書読みを壊す）。ただし本件の読み先は `doc/` であってアーカイブされる `.kiro/specs/` ではないので、この危険は当たらない。

選択肢:

- 案 A: `crates/areka-sylphya/src/` に兄弟テストを 1 本置き、`doc/ukadoc-coverage/ledger/property.toml` を読んで `owner = ""` の行数が 0 であることを判定する。較正として「同じ読み取りが `owner = "areka-P0-` の行を必ず 180 行以上拾う」を同居させる（母数 0 の恒真を避ける）。範囲内で完結する唯一の案。
  - 難点: sylphya が `doc/` へ依存する形は他に例が無い。相対パスの組み立ては `zorder_property_deferral_tests.rs:163-168` が `CARGO_MANIFEST_DIR` から `..` で兄弟クレートへ降りる先例を持つので、同じ形で `../../doc/…` へ届く。
- 案 B: 判定は作らず、design と完了報告に実数（0）と数え方を書く。→ 要件 4.8 の精神に反する（この 0 は「表示するだけの数」になり、次に誰かが空欄を作っても誰も気づかない）。
- 案 C: `crates/ukadoc-survey/src/check/` に規則を足す。→ 編集範囲の外（要件 9.3 の報告事案）。**恒久的にはここが正しい置き場所**なので、範囲外である旨と引受先候補（`areka-P0-ukadoc-coverage-roadmap`）を報告する形が要る。

### 議題 ⑤ 件数の写しが要件 4.7 の実測より多い【要件 4.7 ＋ 9.1】

要件 4.7 は「21 は計 4 箇所」と記すが、再実測では**関数名を含めて 6 箇所**、汎用名の 17 は**編集範囲内に 8 箇所**ある（§2.5）。さらに 17 は編集範囲の外にも 3 箇所ある:

- `.kiro/steering/structure.md:288`「`dotted.rs` ルート枝 10＋汎用名 17＋SET 意味論」
- `.kiro/specs/areka-P0-property-catalog-lists/brief.md:21`（表の「汎用プロパティ名（共有葉）｜17」）と `:73`（「`GENERIC_PROP_NAMES` 17＝`:37-55`」）

決めること:

1. テスト関数の名前（`set_effective_has_21_entries`・`generic_prop_names_has_17_entries`）に数を入れ続けるか。→ 入れ続けるなら 4.7 の「そのすべてを更新」の対象に関数名も含めると design で明示する。数を名前から外す（`set_effective_count_is_fixed` 等）ほうが次回の追随箇所が減るが、**既存の命名規約を本 spec が変えることになる**。
2. `structure.md:288` を更新するか。→ 要件 9.1 の範囲外。放置すれば steering が古びる（「表示するだけの数は必ず古びる」の典型）。**要件 9.3 に従い報告する**か、範囲を 1 行だけ広げる裁定を取るかの二択。
3. `property-catalog-lists` の brief 2 箇所。→ 他 spec の持ち物。その spec が着手時に引き直す（並走 brief は陳腐化する、という既知の規律どおり）ので、本 spec は触らず報告のみでよい見込み。

### 議題 ⑥ `/// ukadoc:` の証跡が報告書の数字を動かす【要件 5.1 ＋ 9.1】

網羅調査の証拠抽出器は、ソース中の URL を 3 段で解決する（`crates/ukadoc-survey/src/evidence/resolve.rs:1-30`）:

1. **カタログの項目 URL と完全一致すれば、その 1 項目の証拠になる。**
2. 一致しなければページ URL として扱い、直後のスライス定数の要素名と見出しを突き合わせる。

要件 5.2 は「カタログの同一 id が持つ URL と一致させる」＝**アンカー付きの項目 URL**を書けと定めているので、段 1 に当たる。したがって:

- `dotted.rs` に `/// ukadoc: …#pause:1` 等を置くと、その項目が「証拠あり」に変わる。
- `doc/ukadoc-coverage/report/summary.md:142` の property の「証拠あり」は現在 **2**。本機能が 4 項目（要件 5.4 は本機能が登記した項目のみを対象とする）なら **6**、18 葉すべてに置くなら **最大 24** になる。
- `doc/ukadoc-coverage/report/property.md` も同様に古びる。
- **どちらも要件 9.1 の編集範囲の外。**

なお現状 `dotted.rs` には `/// ukadoc:` の行が **1 行も無い**（`shiori_resource.rs:45` にはページ URL が 1 行ある）。したがって本機能が初めて URL を持ち込むことになり、段 2（ページ URL ＋ スライス定数の突き合わせ）が誤って発動しないよう、**項目 URL（アンカー付き）だけを書き、ページ URL の単独行は置かない**ことを design で明示するのが安全。

決めること: 報告書の再生成を（a）範囲外として報告するにとどめるか、（b）1 行だけ範囲を広げる裁定を取るか、（c）`ukadoc-coverage-roadmap` へ引き渡すか。

### 議題 ⑦ 台帳の note に残る「語彙表だけを触る spec」の指し先【要件 6.4】

サウンド 18 行の note が `areka-P0-property-query-channels` を指しているが、語彙表の担当は本 spec へ移った（§2.7）。要件 6.4 は 6.1〜6.3 以外の行を変えるなと定めているので直せない。

選択肢:
- 案 A: 直さず、`COMPAT_ARCHITECTURE.md` §8 へ足す 1 行に「サウンド 18 葉の語彙表の担当は `areka-P0-sylphya-set-ledger`」と併記して、記録の上で上書きする（先例＝§8 の `:206`／`:210` がアーカイブ済み文書の誤記を表の行で上書きしている）。要件 8.4 は「§8 の該当行の周辺に限る」なので、zorder の行の直後に置く 1 行へ収まるかは design で詰める。
- 案 B: 要件 6.4 の例外として 18 行の note 1 行だけを差し替える裁定を取る。→ 変更行数が 18 行になり、後続 spec との衝突面が増える（本 spec が先に着地する利点を削る）。
- 案 C: 放置して報告のみ。→ 「誰も引き取らない行を終わらせる」という要件 6 の目的と向きが逆。

### 議題 ⑧ `GENERIC_PROP_NAMES` の表題と中身のずれ

表の名前は「汎用プロパティ名」で、説明文（`dotted.rs:30-36`）は「リスト系ルート枝配下で共通に現れる名前族」と定義している。サウンド族の 15 葉は `currentghost.sound` の下でしか使われないので、**この定義には当てはまらない**。

決めること: 説明文を「正準語彙のうち、末尾のひと区切りで照合する名前の表」のような、実際の使われ方に合った定義へ改めるか。改めるなら 1 段落の書き換えで済み、編集範囲の中（`dotted.rs`）。改めないと、表題と中身が食い違ったまま後続 4 spec がさらに名前を足していくことになる。

---

## 5. 実装方式の選択肢

### 案 A: 既存の 2 配列へ足すだけ（**推奨**）

- `SET_EFFECTIVE` の末尾に 4 要素、`GENERIC_PROP_NAMES` の末尾に 13 要素を追加。既存要素は 1 つも動かさない（要件 1.6）。
- 件数を固定している記述を `dotted.rs`（6／8 箇所）と `ledger_key_determinism_tests.rs`（2／2 箇所）で更新。
- `dotted.rs` の兄弟テストに、本機能が足した 17 項目（4 ＋ 13）の分類を項目ごとに確かめるテストを 1〜2 本追加（要件 4.10）。既存 `actor_tests.rs:16-57` と同じ形。
- `/// ukadoc:` 注記は各要素の直前に 1 行（要件 5.1／5.3）。
- **トレードオフ**: 新しいファイルも型も増えない／`dotted.rs` は 289 行 → 追加後もおよそ 340〜360 行で 1,000 行の見張りに余裕がある／表題と中身のずれ（議題 ⑧）が残る。

### 案 B: サウンド族を別の公開 const に分ける

- **採れない**。t_zpd12 が `vocab/` の公開 const を実物から抜き出して名簿と突き合わせるため、新しい公開 const を作ると必ず赤になる（`zorder_property_deferral_tests.rs` は編集対象外）。開発者裁定でも相乗りが固定されている。
- 私有（`pub` を付けない）const にして `GENERIC_PROP_NAMES` を連結する形なら t_zpd12 は通るが、`GENERIC_PROP_NAMES` が `&[&str]` の const である以上、const 文脈での連結は素直に書けず、**得るもの（表の見た目の整理）に対して複雑さが釣り合わない**。

### 案 C: 案 A ＋ 台帳の空き 0 を判定する兄弟テスト（議題 ④ 案 A）

- 案 A に、`doc/ukadoc-coverage/ledger/property.toml` を読んで `owner = ""` が 0 行であることを判定するテストを 1 本足す。
- **トレードオフ**: 要件 6.5／4.8 を範囲内で満たせる唯一の形／sylphya から `doc/` を読む依存が新しく生まれる（先例なし・ただしパスの組み立ては `zorder_property_deferral_tests.rs:163-168` と同型）／恒久的な置き場所は `ukadoc-survey` の検査の側であり、そちらへ移す日が来ることを `ponytail:` 相当の注記で残すのが素直。

---

## 6. 規模とリスク

| 項目 | 判定 | 理由 |
| --- | --- | --- |
| 規模 | **S**（1〜3 日） | 配列 2 本への追加と件数の追随。新しい型・関数・ファイルなし。実装量より「数の導出を文書に書く」ほうが重い |
| 技術リスク | **Low** | 既存パターンの延長。コンパイル時に固定される定数のみ |
| 設計リスク | **Medium** | 議題 ②（末尾形が実キーと一致しない）と議題 ③（巻き添え 8 件 vs 要件 7.2）は条文の読み方に関わる。ここを曖昧にしたまま実装すると、後続 `property-query-channels` が「台帳に載っているのに効かない」を再発見することになる |
| 範囲リスク | **Medium** | 議題 ④⑤⑥⑦ がいずれも編集範囲の外へ触れたくなる。要件 9.3 の報告を design の時点で用意しておかないと、実装中に判断を迫られる |

---

## 7. 設計フェーズへ持ち越す調査（Research Needed）

1. **正典本文の確認（ukadoc MCP / 一次 HTML）**——`list_propertysystem.html` の「サウンドプロパティ名」の表で、⒜印（SET 有効）が付いているのが本当に `pause`・`playing`・`position` の 3 つだけか。台帳の 26 行は完了 spec `ukadoc-survey-property` の成果物だが、同 spec の完了時に「実測が設計を 7 度覆した」記録があるため、**この 3 件は正典本文で引き直す価値がある**（4 件目があれば要件 4.1 の 25 が崩れる）。
2. **`meta.` 系 8 葉の版番号の食い違い**——台帳は `introduced = "2.8.73"`、カタログの `versions` も `["2.8.73"]` だが、要件と brief は「サウンドの 3 葉（SSP 2.8.72）」と 2.8.72 で通している。18 葉のうち 10 葉が 2.8.72・8 葉が 2.8.73 である点は、文書に書くときに混ぜない。
3. **`dotted.rs` の行数**——現在 289 行。案 A で 340〜360 行程度。1,000 行の見張りには当たらないが、後続 4 spec が同じファイルへ足すので、`property-catalog-lists` までの合計見込みを design で一度概算しておくと、分割の要否を後から慌てて判断せずに済む。
4. **`ukadoc-survey check` の実走**——`crates/ukadoc-survey` の検査は cargo test ではなく CLI から走る（テストは見本データを使う）。議題 ⑥ の影響を数で確かめるには、実装後に CLI を 1 回走らせて証拠件数の前後を採る必要がある。design で「いつ走らせるか」を決めておく。
5. **`seriko.cursor.name`／`seriko.tooltip.name` の先取り登記**——要件 4.5 が「正典と食い違っている既知の状態であり、本機能は解消しない」と定めている。台帳の側にこの 2 項に対応する行が無いことは確認済みだが、**なぜ areka が先取りしたのか**の出典（設計判断の記録）は未確認。要件 4.2 の導出を書くときに出典を 1 つ添えられるかを design で確かめる。

---

## 8. 要件ディスカッションでの処理（2026-09-11）

### 8.1 この場で決めた（設計は再検討しない）

| 元議題 | 決定 | 根拠 |
| --- | --- | --- |
| 議題 ① `meta.` 系 8 葉の綴り | **`meta.album` 〜 `meta.year`（案 甲）** | 巻き添え 0 件（区切りを含む名前は 1 区切りの名前と等しくならない＝`key.rs:104-113`）・台帳 id／正典見出し／カタログ title と逐語一致・既存 4 例（`sakura.bind.menu` 等）と同型。乙は `title`／`year`／`id` のような一般語で自由なキーを広く巻き込む一方、得るものが無い |
| 議題 ⑤(1) テスト関数名に数を入れ続けるか | **現行の命名規約を変えない**（`set_effective_has_25_entries` のように数を入れたまま更新する） | 命名規約の変更は本 spec の目的の外。要件 4.7 が「そのすべてを更新」と定めているので、名前に入った数も更新対象として design に明記すれば足りる |
| 議題 ⑧ `GENERIC_PROP_NAMES` の表題と中身のずれ | **説明文を実際の使われ方に合う定義へ改める**（編集範囲内・1 段落） | サウンド 15 葉は `currentghost.sound` の下でしか使われず、現行の「リスト系ルート枝配下で共通に現れる名前族」に当てはまらない。後続 4 spec がさらに名前を足す前に直すほうが安い |

### 8.2 設計フェーズへ送る（カテゴリ B）

1. **正典本文の引き直し（最優先）**——`list_propertysystem.html` のサウンドプロパティ表で ⒜印（SET 有効）が本当に `pause`・`playing`・`position` の 3 つだけか。4 件目があれば要件 4.1 の 25 が崩れる。完了 spec `ukadoc-survey-property` は「実測が設計を 7 度覆した」記録を持つため、台帳を信じるだけで済ませない。
2. **版番号を混ぜない**——サウンド 18 葉は 10 葉が SSP 2.8.72・`meta.*` 8 葉が **2.8.73**。brief と要件が「サウンドの 3 葉（2.8.72）」と書いているのは 3 葉に限れば正しいが、18 葉の導出を書くときに一括で 2.8.72 と書かない。
3. **`dotted.rs` の行数見込み**——現在 289 行、本機能後およそ 340〜360 行。後続 `currentghost-property-tree`（W15）・`property-catalog-lists`（W16）が同じファイルへ足すため、1,000 行の見張りに対する合計見込みを design で一度概算する。
4. **`ukadoc-survey check` の実走時期**——同検査は cargo test ではなく CLI から走る。証拠件数の前後を採るために実装後に 1 回走らせる、その時期を design で決める。
5. **`seriko.cursor.name`／`seriko.tooltip.name` の先取り登記の出典**——要件 4.5 が「解消しない」と定めた既知の食い違い。要件 4.2 の導出に出典を 1 つ添えられるかを design で確かめる。

### 8.3 ディスカッションで覆った前提（2026-09-13）

開発者指摘「プロパティ名に命名規則は無い／仕様上はどんな名前も許容される」を受けて正典を引き直した結果、**議題 ①〜③ の前提そのものが誤り**と判明した。記録として残す。

- **正典は葉の名前を族ごとに分けて定めている**。`ghostlist.index(ID).汎用プロパティ名` 等の見出しで定義される「汎用プロパティ名」と、`currentghost.sound(要素名).サウンドプロパティ名`／`currentghost.sound.index(ID).サウンドプロパティ名` の下でのみ使う「サウンドプロパティ名」は**別の族**（正典本文が「サウンドプロパティ名は後述」と切り分けている）。`\![set,property,プロパティ名,値]` の記述も「プロパティシステムへの値の入力」のみで、名前の綴りに規則を課していない。
- **「葉の名前が単独で意味を持つ」という定めは正典に存在しない**。したがって `is_canonical_vocab`（`crates/areka-sylphya/src/actor.rs`）の `根が正準 ∨ 葉が汎用名` の第 2 項は areka 固有の作りである。正典のプロパティキーは必ず 10 本のルート枝で始まるため（`DOTTED_ROOTS` の説明文が一次 HTML 全文検証を根拠に明記）、**第 1 項が真なら第 2 項は無意味**＝第 2 項が結果を変えるのは根が正準でないとき、すなわち正典に無いキーに対してだけである。
- **語彙表の読み手は `classify_set` ただ 1 か所**（実測: `GENERIC_PROP_NAMES`／`SET_EFFECTIVE` の非テスト参照は `actor.rs` のみ）。記録用の表を仕分けから切り離せば、登記が挙動を動かすことはない。
- よって**議題 ③ の巻き添え 8 件は発生しない**（要件 3.7 を「変えない」へ改訂・要件 7.2 は例外不要）。**議題 ① の `meta.` 綴りも巻き添えの観点では論点でなくなる**が、正典・台帳 id・カタログ title との逐語一致という理由は残るので 8.1 の決定（`meta.album` 形）は維持する。
- **議題 ② は議題 ①③ と同じ根っこ**——記録用の表と照合用の表を同一視していること——なので、要件 10 として 1 つにまとめて起票対象にした。

**要件 10 の引受先**: `areka-P0-property-query-channels`（実在確認済み・`.kiro/specs/areka-P0-property-query-channels/brief.md`・W14＝本 spec の後）。`\![set,property,...]` の経路を開通させる spec であり、フルキーが実際に流れ込む最初の地点なので、正規化の要否と仕分けの規則を決めるのはこの spec が最も自然。同 brief の追記(88) は本 spec への分割を既に登記しており、`dotted.rs` には触れないと明記している（＝衝突しない）。**同 spec が引き受けられない場合は単独 spec として起票すること**（引受先の無い先送りにしない）。

### 8.4 議題 ④〜⑦ の処理（編集範囲を広げない）

| 元議題 | 処理 | 理由 |
| --- | --- | --- |
| ④ 担当欄の空き 0 を判定する検査 | **新しい検査は作らない**。要件 6.5 は変更適用後の 1 回の実数えで満たす（数え方を報告に明記）。恒久的な見張りの置き場所は `crates/ukadoc-survey/src/check/` であり、`areka-P0-ukadoc-coverage-roadmap` へ申し送る | 要件 4.8 の「判定させよ」が掛かるのは**件数檻**（`dotted.rs` の `assert_eq!` 群＝既に判定形）であって、台帳の担当欄ではない。議題 ④ は 2 つを混同していた。sylphya から `doc/` を読むテストを新設するのは、置き場所として誤り |
| ⑤ 件数の写しが範囲外にもある | `.kiro/steering/structure.md` の「汎用名 17」は**本 spec では動かない**（記録用の表を分けたので `GENERIC_PROP_NAMES` は 17 のまま）。新表の存在ぶんだけ記述が不足するので、設計で 1 行の追随可否を判断する。`areka-P0-property-catalog-lists` の brief 2 箇所は他 spec の持ち物なので触らない（着手時に引き直す規律どおり） | 相乗りをやめた結果、範囲外の数字が動く件は大半が消えた |
| ⑥ `/// ukadoc:` が報告書の証拠数を動かす | 範囲外として**報告**する。`doc/ukadoc-coverage/report/summary.md` の property「証拠あり」は 2 → 21（19 項目ぶん）。再生成は `areka-P0-ukadoc-coverage-roadmap` へ申し送る。実装では**アンカー付きの項目 URL だけ**を書き、ページ URL の単独行は置かない（証拠抽出器の第 2 段が誤発動しないため） | 報告書は要件 9.1 の範囲外。数え直しの担い手は網羅調査側 |
| ⑦ 台帳 note の指し先 18 行が古びる | `doc/COMPAT_ARCHITECTURE.md` §8 へ足す 1 行に「サウンドプロパティ名 18 葉の語彙表の担当は `areka-P0-sylphya-set-ledger`」を併記し、記録の上で上書きする（先例＝§8 が既にアーカイブ済み文書の誤記を表の行で上書きしている）。台帳の 18 行は要件 6.4 のとおり触らない | 18 行を書き換えると後続 spec との衝突面が増え、本 spec が先に着地する利点を削る |

### 8.5 steering への追随（実施済み）

`.kiro/steering/roadmap.md`「棚卸⑬の仮裁定」1 を改訂した（2026-09-13）。`zorder` は台帳行も値の導出もともに `areka-P0-zorder-property` が持ち、`sylphya-set-ledger` の SET 有効群は 21→**25**（仮裁定の文面にあった 26 ではない）。`seriko.sticky-window` と `currentghost-property-tree` に関する部分は据え置き。

---

## 9. 設計フェーズの調査（2026-09-13・`/kiro-spec-design -y`）

> Discovery Scope: **Extension（light discovery）**。既存の語彙表への末尾追加と記録用配列の新設であり、外部依存・新技術は無い。サブエージェントは使わず主文脈で file:line を直接裏取りした。

### 9.1 正典本文の引き直し（§8.2 の 1・最優先）

- **Context**: 台帳の 26 行は完了 spec の成果物だが、同 spec は「実測が設計を 7 度覆した」記録を持つ。4 件目の ⒜印があれば要件 4.1 の 25 が崩れる。
- **Sources Consulted**: ukadoc MCP `get_doc` でサウンドプロパティ名 18 葉と `currentghost.seriko.sticky-window` の全文（計 19 件）を個別に取得。`search_docs`（`サウンドプロパティ名`・`playing`）で族の頭 2 本と `currentghost.sound.count` も確認。
- **Findings**:
  - `[SET有効]` の記述を持つのは **`pause`・`playing`・`position` の 3 葉ちょうど**（`pause`＝`\![sound,pause／resume]` 等価、`playing`＝`\![sound,play／stop]` 等価、`position`＝`\![sound,option,--seektime=]` 等価）。残り 15 葉（`duration`・`error`・`id`・`loop`・`name`・`path`・`preload`・`meta.*` 8）には記述なし。`sticky-window` にはあり。
  - 版の刻印: 10 葉が 2.8.72、`meta.*` 8 葉が **2.8.73**、`sticky-window` が 2.8.78。カタログ `versions` 欄と一致。
  - 見た版: MCP スナップショットのプロパティ節は **2.8.80 で陳腐化していることが既知**（記憶 `ukadoc-mcp-preferred-source`）。項目の刻印は上記のとおりで、本 spec が扱う 19 項目はいずれも 2.8.80 以前に定義されたものなので、陳腐化の影響は「19 項目より後に増えた葉を見落とす」方向にしか働かない（正典の増減は自動で見張らない＝開発者方針 2026-09-11）。
- **Implications**: 25 は崩れない。設計は 3 葉で確定。

### 9.2 版番号を混ぜない（§8.2 の 2）

- 設計の `SOUND_PROP_NAMES` 節と `dotted.rs` の説明文は「括弧を含まない 10 葉（2.8.72）／`meta.` 8 葉（2.8.73）」と分けて書く。brief と要件の「サウンドの 3 葉（2.8.72）」は 3 葉に限れば正しいので改訂しない。

### 9.3 `dotted.rs` の行数見込み（§8.2 の 3）

- 現在 289 行。本機能後およそ 360 行（SET 4＋URL 1＋新表 18＋URL 18＋説明文 25 前後＋兄弟テスト接続 3）。新設テストは兄弟ファイル `vocab/dotted_set_ledger_tests.rs`（およそ 150 行）へ出す（`structure.md` の兄弟テスト規律）。
- 後続: `areka-P0-currentghost-property-tree`（W15）は縮退宣言文の改訂＝数行、`areka-P0-property-catalog-lists`（W16）はリスト系の名前の追加＝多く見ても 100 行。合計 500 行台で 1,000 行の見張りには当たらない。分割不要。

### 9.4 `ukadoc-survey check` の実走時期（§8.2 の 4）

- `doc/ukadoc-coverage/README.md`: 副手続きは `catalog`／`ledger-init`／`report`／`report-summary`／`check`／`evidence`／`candidates`／`diff` の 8 つ。「台帳を触ったら `cargo test -p ukadoc-survey` を走らせること」。
- 決定: 実装後に `cargo test -p ukadoc-survey` と `cargo run -p ukadoc-survey -- check` を 1 回ずつ。`evidence` は実装の**前後**で 1 回ずつ走らせ、property の証拠件数 2 → 21 を完了報告に記録する。`report`／`report-summary` は範囲外なので走らせない（走らせると `doc/ukadoc-coverage/report/*.md` が編集集合の外で動く）。
- 検査の規則を確認（`crates/ukadoc-survey/src/check/content.rs`）: `ImplementedWithoutEvidence` は `status = "implemented"` の行だけが対象・`SourceUrlNotInCatalog` はカタログに無い URL だけが対象。本機能の URL はすべてカタログの `url` 欄と完全一致させるので赤にならない。`owner` の中身はどの規則も見ない。

### 9.5 `seriko.cursor.name`／`seriko.tooltip.name` の先取りの出典（§8.2 の 5）

- 出典 **あり**: 完了 spec `areka-P0-ukadoc-survey-property` の `design.md` 規則 8 突合表・区分 ⑶「21 名のうち、正典が SET 有効としない id を指す名前＝2 名／4 id」（2026-09-05 訂正込み。`(当たり判定名)` の 2 件は「index 指定との互換用の記述であり、特に意味はない」、`.index(ID2)` の 2 件は「…の当たり判定名」と述べるが、印が無く族の頭の継承も無い点は 4 件とも同じ）。同 `research.md` §5-2 で正典本文により決着。
- 要件 4.2 の導出に「出典＝完了 spec `ukadoc-survey-property` design 規則 8 区分 ⑶」を添える。なぜ areka が先取りしたかの設計判断の記録は sylphya の完了 spec 側に無い（brief 逐語転記の 21 名に最初から含まれていた）ので、「既知の食い違い・本機能は解消しない」と書くにとどめる。

### 9.6 証拠抽出器の記号と配列要素位置の注記（要件 5.3 の形の確定）

- `crates/ukadoc-survey/src/evidence/extract.rs`: 行頭の空白を除いて `///`・`//!`・`//` のいずれかで始まり、`ukadoc:`＋空白＋URL **1 語**の行だけを証拠として拾う（説明文つきは拾わない・コードの尻尾の `// ukadoc:` も拾わない）。
- 配列要素の直上に doc コメント `///` は置けない（式への属性は安定化されていない）。`//` は抽出器が同じ規則で読む 3 記号の 1 つで、先例 `crates/areka/src/placement/config.rs` の配列要素直上 `// ukadoc:` がある。→ 設計は `// ukadoc: <アンカー付き URL>` で確定。
- `resolve.rs`: アンカー無しのページ URL の単独行は「直後のスライス定数の要素名を突き合わせる」第 2 段を起動する。→ ページ URL の単独行は置かない（設計に明記）。

### 9.7 先送りテストの名簿登記の実際の形（要件 3.6／9.2 の設計上の確定）

- `zorder_property_deferral_tests.rs` の再読で確定した事実:
  - t_zpd12 ① は `vocabulary_tables()` が実際に読む表の const 名の列と `SCANNED_VOCAB_TABLES` の**一致**を判定する。名簿だけ足して関数を足さないと赤。→ 登記は const と関数の**対**。
  - t_zpd11 は `tables.len() == 5` と 5 本の較正行を持つ。名簿を 6 本にすると `5` が赤。→ t_zpd11 の `5` は「名簿の本数の写し」であり、要件 7.4 の「台帳が古い前提を固定していた」側（要件 7.5 が更新側に置くなと定めるのは t_zpd10／12／30／40 で、t_zpd11 は含まれない）。
  - t_zpd40 は `crates/areka-sylphya/src` の**全ソース本文**から探し語 `zorder`（小文字・部分一致）を探す。追跡 spec の名前 `areka-P0-zorder-property` もこの綴りを含む。→ `dotted.rs` の説明文・新設テスト・URL 注記のどこにも書けない。設計に明記した。
- `SOUND_PROP_NAMES` は `pub` が必須（t_zpd12 の抜き出しは `pub const` 行だけを見る。私有にすると t_zpd10 の走査から漏れる）。

### 9.8 設計判断の記録（synthesis）

| 判断 | 選択 | 却下した案と理由 |
|---|---|---|
| サウンド 18 葉の置き場所 | 新設 `pub const SOUND_PROP_NAMES: &[&str]`（記録用・`classify_set` が読まない） | `GENERIC_PROP_NAMES` へ相乗り（§5 案 A）→ 葉一致の判定に参加して要件 7.2 が成立しない。§8.3 で前提が覆り、開発者裁定「相乗り」も撤回済み（記録用の表を分けることが要件 3.3 に明文化された） |
| 新表の型 | `&[&str]` | `&[(&str, SetSemantics)]`→ 記録用の表に意味論は不要。`&[(&str, &str)]`（URL を要素に持つ）→ 証拠抽出器はコメント行しか読まないので URL を要素に持っても証拠にならず、二重管理になる |
| URL 注記の置き場所 | `SET_EFFECTIVE` に 1（sticky-window）・`SOUND_PROP_NAMES` に 18 | SET 側にも 3 葉分を置く→ 正典の項目としては 1 つ（要件 5.1）。証拠件数も二重に数えない |
| 新設テストの置き場所 | 兄弟ファイル `vocab/dotted_set_ledger_tests.rs`（`#[path]` 接続） | `dotted.rs` のインライン `mod tests` へ追記→ `structure.md` の「新規のテストモジュールは兄弟ファイルへ」に反する。`actor_tests.rs` へ追記→ 表の件数・構造の検査まで actor 側に置くのは責務がずれる |
| 要件 3.3／7.2 の判定形 | `actor.rs` の本文に `SOUND_PROP_NAMES` の参照が 0 回（較正: `GENERIC_PROP_NAMES` ≥ 1） | 分類の literal 検査だけ→ 「読まない」という構造の主張が検査に写らない（零は明示的に判定する） |
| 要件 6.5 の 0 | 完了時の 1 回の実数え（`grep -c`＋対照） | 新しい判定テスト→ 開発者裁定（§8.4 ④）で不採用 |
| 一般化 | 無し | 「語彙表の登記を汎用化する機構」は現要件に無い。配列 2 本で足りる |
| Build vs Adopt | 既存の `const` スライスと `assert_eq!` をそのまま使う | 新しい依存・マクロ・生成器は不要 |

### 9.9 リスクと軽減

- `zorder` の綴りが sylphya のソースへ紛れ込む（説明文・失敗メッセージ・spec 名）→ 設計に禁止を明記。実装後の `cargo test -p areka --lib placement::zorder_property_deferral_tests` が t_zpd40 で赤にする。
- `sound(bgm.mp3)` のような括弧内に `.` を含む例を対照に使う → `parse_dotted` が解釈不能で StoreWrite に落ち、対照にならない。設計で `sound(bgm)` に固定。
- 件数の写しの取りこぼし → 設計の一覧（21 の 7 箇所・5／8 の 2 箇所）を実装後に `grep` で 0 件確認。
- 報告書の証拠件数が古びる → 範囲外として申し送り（設計「範囲外の申し送り」1）。
