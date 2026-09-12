# Brief: areka-P0-shell-implicit-surface

> **種別**: バグ（正典未実装ゆえ**里々標準テンプレートが 1 枚も絵を出せない**）。②parsers／⑥emo（合成）帰属。
> **源**: `areka-P0-charset-canon` タスク 6.2 の実機確認（2026-09-13）。文字コードの側は直ったのに窓が生えず、原因が別の穴だと判明したため `/kiro-discovery` で分離起票。記録の正本は `.kiro/specs/areka-P0-charset-canon/verification/signoff-record.md` 8 節。
> **対 `charset-canon` の関係**: 同じ「里々のゴーストを 1 体動かす」という目に見える成果の**残り半分**。charset-canon が**文字**を直し、本 spec が**絵**を出す。片方だけでは利用者から見て何も変わらない。

## Problem

**里々の標準テンプレート『Ｒポストと狛犬』が areka で 1 枚も絵を出せない。** 検体は `vendors/sample_ghost/R_POST_and_KOMAINU/`（`charset-canon` で追跡済み）。SHIORI は応答し、`OnFirstBoot` の挨拶も流れ、終了も正常なのに、合成だけが失敗して検証用ダミー窓へ落ちる。

失敗の連鎖（`signoff-record.md` 8 節の逐語）:

```
ERROR areka_emo_compose: 定義層が皆無で外形 0×0 の退化データ: EmptyComposition surface_id=0
ERROR areka::placement::measure: measure: scope0（surface id 0）の採寸合成に失敗 reason=surface 0 has no layers at all (extent 0x0)
ERROR areka: 窓配置の準備に失敗しました——検証用ダミー窓へフォールバックします
```

根は `shell/master/surfaces.txt` の `element` 行が **0 件**であること。このテンプレートは立ち絵を `element` で組まず、**ファイル名の慣習**（`surface0000.png` が面 0 の基準画像）に委ねている。areka はその経路を持たない——面は `crates/areka-parsers/src/shell/decode.rs:191` の `decode_elements` が `elementN,overlay,…` 行**だけ**から組む。

**正典**（`https://ssp.shillest.net/ukadoc/manual/dev_shell.html`）は 2 文で本件を定める:

- 「サーフェスはsurface0000.png、surface0010.png等の様に記述してもsurface0.png、surface10.pngと同様に認識されます。」＝**0 抑制した番号の一致**が判定規則。
- 「なお、surface\*.pngのような名前のpng画像は、element0より下のパーツとみなされる点に注意してください。」＝**`element0` の代替ではなく、その下に敷く層**。

後者が本件の難所である。「element が無いときの代替」なら影響は element 0 件の面に閉じるが、正典は**下に敷く**と書いているので、**element を持つ面にも 1 層増える**。番人役の emo2 がまさにそちら側に居る（下の「B. 退化の集合」）。

**4 桁表記の合理性**（開発者の起票理由・2026-09-13）: 過去互換だけの話ではない。`surface0000.png`〜 の 4 桁で書けばファイル一覧が**番号順にソートされる**ので、シェル作者にとって管理しやすい。ゆえに `surface0.png`／`surface00.png`／`surface000.png`／`surface0000.png` を**同じ面 0 として**受ける 0 抑制の判定が要る。

## Current State

### A. 対象検体（里々標準テンプレート）の実測

`vendors/sample_ghost/R_POST_and_KOMAINU/shell/master/` 直下の画像（全数）と 0 抑制後の番号:

| ファイル | 番号 | 実寸（IHDR） |
|---|---|---|
| `surface0000.png` | 0 | 236×462 |
| `surface0001.png`〜`surface0006.png` | 1〜6 | — |
| `surface0010.png` | 10 | 140×160 |
| `surface0100.png` | 100 | 143×373 |
| `surface0200.png` | 200 | — |

- `.pna`（α マスク）は **0 件**。`menu_background.png` 等は `surface` 接頭辞を持たないので対象外。
- `surfaces.txt` の波括弧は `surface0,1,2,3,4,5,6,100,200` の **9 個**（同ファイル 1・12・23・34・45・56・67・78・84 行）。中身は `collision0`〜`3` と `point.kinoko.center*`、`surface100`／`surface200` は `sakura.balloon.offset*`／`kero.balloon.offset*` のみ。`element` 行は 0、`animation*` 行も 0。
- **宣言あり・画像なし＝0 件**／**画像あり・宣言なし＝1 件（番号 10）**。後者は「宣言が無くても描けなければならない」側の実例。
- `defaultsurface` の宣言は `shell/master/descript.txt` にも `ghost/master/descript.txt` にも **無い**（`grep -i` → 0 件）。ゆえに**面 0 が既定として解決できることが必須**。

### B. 退化の集合（番人 emo2 で 1 層増える面）

`crates/pilot/examples/shiori-host-32/fixtures/emo2` は**壊してはならない側**。`shell/master/` 直下の `surface*` 画像は 2 本のみ（`.pna` は全域で 0 件）:

| ファイル | 0 抑制番号 | 実寸 |
|---|---|---|
| `surface0.png` | 0 | 434×687 |
| `surface10.png` | 10 | 427×463 |

`element` 行を持つ面は 57 個。**交差は `{0, 10}` の 2 件**で、意味が違う:

1. **面 0** — `surfaces.txt:12` が `element0,overlay,surface0.png,0,0`。暗黙の下層も同じ `surface0.png` になり、**同一画像を自分の上に二重合成**する。外形は変わらない。
2. **面 10** — `surfaces.txt:296` が `element0,overlay,CityPop\surface0010.png,0,0`（336×400）。暗黙の下層は**直下の別画像** `surface10.png`（427×463）。**背面に別の絵が増え、外形が 336×400 → 427×463 へ広がる**。

正典が `element0,overlay,surface0.png` の例に「なおこの場合surface1.pngは不要です」と断っているのは、この自己二重合成を避けよという意味に読める。

**既存の檻が捕まえるか**:

- 面 10 → **捕まる**。`crates/areka/src/placement/measure_tests.rs:69-70` が `SCOPE1_W=336`／`SCOPE1_H=400` を逐語固定し、:102-104 で厳密比較する。外形が動けば即赤。
- 面 0 → **黙って通る**。`crates/areka-emo-compose/src/golden_tests_surface0_base_tests.rs:71` の byte 等価 golden は全画素 α=255 の不透明画像を挿すので、premultiplied SourceOver で自分の上に重ねても**バイトが変わらない**。外形も不変なので採寸檻（同 `measure_tests.rs:66-67`）も通る。**面 0 の自己二重合成に対する番人は現に居ない。**

### C. 実装の現場

1. `surfaces.txt` 読取＋復号 → `areka_parsers::shell::parse` → `decode::dispatch_block`（`decode.rs:115`）→ `decode_surface_body`（:171）→ `decode_elements`（:191、`overlay` 行のみ値化）。
2. `SurfaceSet{ surfaces, base_dir, .. }` → `bake` → `EmoWorld::build`（`crates/areka-emo-compose/src/world.rs:74`・実体は `fold::fold_shell` `fold.rs:30-70`）。
3. 外形算出 `plan.rs:480 compute_extent`、裁定は :501-571（不在＝`SurfaceNotFound` :556／**外形 0×0＝`EmptyComposition` :571**）。里々テンプレはここに落ちる。

**ファイル名から番号を取るコードは 1 行も無い**（検索語 `surface0000`／`{:04}`／`strip_prefix("surface")`／`trim_start_matches('0')` → シェル資産経路のヒット 0。`decode.rs:132` の `strip_prefix("surface")` は**ブロック見出しの解析**であってファイル名ではない）。0 抑制の実装も 0 件。

**唯一の前例はバルーン**: `crates/areka-emo-present/src/balloon.rs:309-334 enumerate_file_names`（`read_dir` 1 回・非 UTF-8 名はここで落とす・失敗は log-first で `PresentError`）＋純核 `select_faces`（:265）＋面判定 `face_id_of`（:233-244、接頭辞を大小無視で strip → 拡張子 strip → 残余の全数字検査。`parse::<u32>()` が**先行ゼロを自然に吸収する＝0 抑制はこの形でそのまま満たせる**）。決定論は `BTreeMap` のキー順で担保。**再利用すべき形はここに揃っている。**

### D. 台帳の穴（項目を足せない・理由付き）

- ページ粒度の `ukadoc:dev_shell` は `doc/ukadoc-coverage/ledger/assets.toml:7988` に `status = "absent"`・`owner = ""`・`priority = "D55"` で在る。note 自身が「ページ 1 枚をまとめて指す粗い粒度」と明言し、担当の割り当てを `areka-P0-ukadoc-coverage-roadmap` に委ねている。
- **正典の当該 2 文を項目粒度で持つ行は台帳に存在しない**（検索語 `surface0000`／`暗黙`／`ゼロ埋め`／`ファイル名がそのまま` → 全て 0 件）。
- **今すぐ項目行を足すことはできない。** 台帳の id はカタログ（ukadoc スナップショットから生成）に実在しなければならず、無い id は `crates/ukadoc-survey/src/check/structure.rs:110-120` が `LedgerIdNotInCatalog` で落とす。ページ 1 枚を項目粒度へ割る判断は `ukadoc-coverage-roadmap` の仕事。→ 本 spec は**その依頼を明文で置く**（下の Out of Boundary）。

## Desired Outcome

1. **里々標準テンプレートが絵を出す。** `element` を 1 行も持たないシェルで、面 0 が `surface0000.png` から解決され、挨拶が立ち絵と一緒にバルーンへ出る（実機目視）。`\s[1]`〜`\s[6]`・`\s[10]`・`\s[100]`・`\s[200]` も同じ規則で到達できる。
2. **emo2 が退化しない。** 交差 `{0, 10}` の 2 面について、外形・合成結果ともに適用前と同一であること。**これは副作用の確認ではなく、本 spec の合否判定そのもの**（開発者指示・2026-09-13）。
3. **0 抑制が効く。** `surface0.png`／`surface00.png`／`surface000.png`／`surface0000.png` が同じ面 0 に解決される。4 桁表記のシェルが番号順に並んだまま正しく読める。
4. **面 0 の自己二重合成に番人が付く。** 今日は不透明 golden が黙って通す穴なので、新しい檻が要る（既存 golden の「非空虚性」規律に倣う＝`golden_tests_surface0_base_tests.rs:142` が先例）。

## Approach

1. **列挙と 0 抑制の権威を 1 本作る**（新規ファイル）。バルーンの `enumerate_file_names`／`face_id_of` を形の手本にし、`read_dir` 1 回・非 UTF-8 名は落とす・`BTreeMap` で決定論。`surfaceNNNN.png` は ASCII なのでこの縮退で足りる。
2. **合流点は「シェルディレクトリを知り、かつ面を組む最初の下流」**＝`crates/areka/src/emo2_boot/assets.rs:274-315`（本番）と `crates/areka/src/placement/measure.rs:332-383`（採寸）の**2 経路**。`areka-parsers` は転記層で `fs` を持たない（`lib.rs:7`／`package/mod.rs:3` が「parser ファミリ内で唯一の I/O は package module」と明記）ので、そこは家ではない。
   - ⚠ **前例的警告**: `measure.rs:385-400` の rustdoc が、バルーン系列で「列挙規則が独立 2 実装へ分かれると実機でしか出ない欠陥になる」ため**権威 1 本**へ寄せた経緯を記している。暗黙サーフェスでも同じ罠がある。**2 経路が同じ 1 本を呼ぶこと**を設計の不変条件にする。
3. **「element0 より下」の表現を決める。** `Element.layer` は現在 `u32`（`decode.rs:202`・ソートは `decode.rs:215`／`fold.rs:118`）で、**`element0` より下は u32 では表現できない**。⑴ 先頭固定の base スロットを別フィールドで持つ／⑵ layer を符号付きにする／⑶ 合成計画の側で先頭に挿す——の 3 案から設計フェーズで裁定する。転記層の意味を変えない ⑴ か ⑶ が有力。
4. **`surface.append` との相互作用を先に決める。** `fold.rs:86-120` の append は**その時点で既存の id にのみ**追記し、非存在は新設せず warn（:103）する。暗黙 base を `fold_shell` より前に作ると、**宣言の無い面への append が通るようになる**＝挙動が変わる。作る位置は要件で明示する。
5. **檻**: ⑴ 里々テンプレを入力にした決定論テスト（element 0 件で面 0 の外形が 236×462 になる）⑵ 0 抑制の 4 表記が同一番号に解決される純関数テスト ⑶ emo2 の面 0／面 10 の外形と合成バイトの不変（面 0 は**不透明画像では恒真になる**ので、α を持つ検体か別画像で非空虚性を示すこと）⑷ 実機サインオフ 2 体（里々＝絵が出る／emo2＝適用前と同一）。

## Scope

- **In**: シェルディレクトリ直下の `surface*.png` 列挙と 0 抑制番号の導出／暗黙の base 層を `element0` の下に敷く合成／本番経路と採寸経路の合流（権威 1 本）／`surface.append` との順序の確定／決定論檻＋実機 2 体サインオフ。
- **Out**:
  - `.pna`（α マスク）の番号共有——検体 2 体とも `.pna` を 1 件も持たず、実測で裏が取れない。語彙だけ残して別 spec へ。
  - DAP/DDP/DFP/DGP/GIF/JPEG/BMP の後方互換読み——正典が「非推奨」と明記。png 以外は読まない。
  - `defaultsurface`（シェル面の既定サーフェス番号）——消費経路が codebase に 1 本も無い（`grep defaultsurface` → 0 件）。本 spec は「既定 0」を前提に置くだけで、宣言の実導出は持たない。
  - `alias.txt`／`surfaces*.txt` 断片の列挙——読む道が今日も無い（台帳 `assets.toml:5980` の note が「読む道が無い」と記載）。
  - 着せ替え（`surface.append` の意味論そのもの）・当たり判定・`point.basepos`（`areka-P0-surfaces-basepos` が持つ）。

## Boundary Candidates

- **列挙と番号の導出**（純関数＋`read_dir` 1 回）＝新規ファイル 1 本。バルーン前例と同型。
- **合成計画への挿入**（`element0` より下の層）＝`areka-emo-compose` 側。
- **2 経路の合流**（`assets.rs`／`measure.rs`）＝`areka` 本体側。

## Out of Boundary

- **台帳の項目粒度化**。正典の当該 2 文を項目として持つ行を作るには、`ukadoc:dev_shell` のページ 1 枚をカタログの側で項目へ割る必要があり、それは `areka-P0-ukadoc-coverage-roadmap` の仕事（台帳 note 自身が委ねている）。本 spec は実装を持ち、**台帳の粒度は持たない**。→ `ukadoc-coverage-roadmap` へ「`dev_shell` の 2 文を項目へ割り、担当を本 spec にする」を依頼として渡す。
- `ukadoc:dev_shell` の残りの範囲（着せ替え・当たり判定・ネットワーク更新）。本 spec はページの 2 文だけを引き受ける。

## Upstream / Downstream

- **Upstream**: `areka-P0-charset-canon`（検体の追跡・実機手順・`signoff-record.md` 8 節の障害記録が本 spec の源）。completed の shell-parse 系（転記層の形）。`areka-P0-emo2-conformance-e2e`（emo2 の檻＝番人の実体）。
- **Downstream**: 里々／YAYA の実ゴースト適合一般（emo2 以外の実シェルが「そのまま」動く土台）。M2 のシェル互換拡充（`.pna`・非推奨形式・`defaultsurface`）。

## Existing Spec Touchpoints

- **`areka-P0-nar-install`（未着手・単独枠）— 順序の裁定が要る**。nar-install の brief は `R_POST_and_KOMAINU` を `.nar` へ畳んで展開ツリーを追跡外にすると書いており（同 brief 87・95 行）、**本 spec が参照する `vendors/sample_ghost/R_POST_and_KOMAINU/shell/master/` のパスは nar-install の後に消える**。さらに nar-install は 38 ファイル・9 クレートに触るため並走不可（同 128 行）。⇒ **直列化必須**。推奨は**本 spec が先**（検体パスが安定しているうちに檻を作り、nar-install の側が共有ヘルパへ寄せるときに一緒に付け替える）。
- **`areka-P0-surfaces-basepos`（未着手）**— 同じ `areka-parsers/src/shell/{decode,model}.rs` に触る可能性がある。同じ W14 に居るが**同時には走らせない**（直列・後着が rebase）。
- **`areka-P0-present-gpu-transform-scale`（W13）**— `areka-emo-compose`／`areka-emo-present` に同居しうる。着地後に入る。
- **`areka-P0-charset-canon`（W13・完了直前）**— シェル本文の復号を持つが面の構築には触れない。共有ファイル 0。

## Constraints

- **emo2 が壊れないことは合否判定そのもの**（開発者指示 2026-09-13）。交差 `{0, 10}` の 2 面を名指しで検査し、面 0 は**不透明 golden が恒真になる**ことを踏まえた非空虚な檻を置く。
- **意味論は ukadoc から輸入し、SSP 実測主義は取らない**（記憶 `no-ssp-measurement-import-semantics-from-ukadoc`）。上の 2 文が正典の逐語。
- **列挙規則の権威は 1 本**（`measure.rs:385-400` の前例）。本番と採寸で 2 実装に分けない。
- **転記層は解釈しない**（`areka-parsers` は `fs` を持たない・`lib.rs:7`）。ファイル列挙を parser に置かない。
- **テスト規約**: 兄弟ファイル `<stem>_tests.rs` を `src/` に置く／共有ヘルパは `<stem>_test_support.rs`／一時パスは `temp-path-kit`（`areka-parsers` は既に `[dev-dependencies]` 済み）／`log-capture-kit` は **`[dev-dependencies]` のみ**／1,000 行の番人（`crates/log-capture-kit/tests/file_length_guard_test.rs`）の**例外表には触らない**＝新規は 1,000 行未満の新ファイルで足す。
- **実機運転の定石**: 絶対パス起動・i686 helper 先ビルド・`AREKA_APP_SMOKE_EXIT_MS` 有界自動終了＋`RUST_LOG` grep。⚠ **`RUST_LOG` は target 名**で、未設定・書式不正でも黙って `info` へ落ちる（`crates/areka/src/main.rs:141`）。「0 件」を主張するなら**同じ走行の中に debug 行が実在する**ことを示すこと（`charset-canon` の `signoff-record.md` 6.3 節の申し送り）。
- **想定規模**: 新規 1 ファイル＋`assets.rs`＋`measure.rs`＋（layer 表現を変えるなら）`shell/model.rs`・`decode.rs`・`fold.rs`・`normalized.rs`・`plan.rs` とその兄弟檻。クレート 2〜5・ファイル 5〜12。**M**。
