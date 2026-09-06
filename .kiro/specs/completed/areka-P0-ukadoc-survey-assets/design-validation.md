# 設計検証レポート: areka-P0-ukadoc-survey-assets

- 実施日: 2026-09-03
- 対象: `.kiro/specs/areka-P0-ukadoc-survey-assets/design.md`（556 行）
- 突き合わせた文書: 同 spec の `requirements.md`（承認済み）・`research.md`・`brief.md`、上流契約 `.kiro/specs/areka-P0-ukadoc-survey-toolkit/requirements.md`（付録 A・付録 B）、`.kiro/steering/`
- 実測環境: 作業ツリー `claude/areka-p0-ukadoc-survey-33754d`。スナップショットは `%APPDATA%\npm\node_modules\ukagaka-doc-mcp\data\index.json`（`version` = 1・`generatedAt` = 2026-08-24T04:08:57.881Z）
- 本レポートは非対話で作成した。開発者への質問は行っていない。

## 検証サマリー

生産手順としての設計は再現でき、要件 85 項目すべてに引受先がある。台帳の骨組み生成手順（§6.1）を本レポートで独立に走らせたところ、担当 24 ページ・542 件・ページ別内訳・id の並び順まで設計の記載と 1 件も違わずに再現できた。一方で、⑴ 版番号の境界事例 7 件という判断（D6）が実測と合わず、⑵ 正典キーを読んでいるディレクトリが 1 つ調査範囲から抜けており、⑶ URL を置く位置の大半が Rust の doc コメントとして成立しない、という 3 点が残る。いずれも局所の修正で直り、構造の作り直しは要らない。

## 重大な問題（3 件）

### 🔴 重大 1: D6 の「語境界で結果が割れる 7 項目」が実測と合わない

**問題**: 設計 §5 の D6 は「前後に英数字・下線・ピリオドが続かない `\d+\.\d+\.\d+`」を版番号の抽出規則と定め、この規則と緩い規則で結果が割れる項目を 7 件挙げている。同じ規則をスナップショットの 542 件に当てて数え直すと、割れるのは **2 件だけ**（`ukadoc:dev_nar`・`ukadoc:dev_update`）である。割れるのは本文が `SSP2.3.00` のように版番号を英字へ直接くっつけている場合に限られる。D6 の表の 7 行のうち 6 行は「語境界あり＝なし」と書いているが、実際には語境界付きでも版番号が取れる。

| 項目 | 設計の記載（語境界あり） | 実測（語境界あり） | 本文の形 |
|---|---|---|---|
| `descript_balloon:cursor_...` | なし | **2.5.40** | `SSP 2.5.40`（空白あり） |
| `descript_ghost:cursor_...` | なし | **2.5.41** | 同上 |
| `descript_ghost:shiori.cache_...` | なし | **2.4.73・2.4.74** | `SSP 2.4.73まで1、2.4.74以降は0` |
| `descript_ghost:shiori.logo.file_...` | なし | **2.4.26・2.8.56** | 同上 |
| `descript_install:_76f8_5bfe_30d1_30b9...` | なし | **2.5.17** | 同上 |
| `ukadoc:manual_shell` | なし | **2.2.57・2.7.38** | 同上 |
| `ukadoc:dev_nar` | 2.7.52 | 2.7.52（一致） | `SSP2.3.00`（くっついている） |
| `ukadoc:dev_update`（**表に無い**） | — | なし（緩い規則では 2.3.00） | `SSP2.3.00` |

**影響**: 版番号がはっきり取れる 5 項目の「登場した版」が空のまま確定する。要件 2.7 は「版番号が無ければ空」と言っているだけで、取れるのに空にしてよいとは言っていない。さらに §11 の V10 が「D6 の 7 項目は空であること」を検査条件にしているため、**この誤りを検査が緑で追認する**。世代表（要件 4.1・4.2）の版番号にもそのまま流れ込む。なお 7 件はいずれも `descript_shell_surfaces` の項目ではないので、137 行の世代表そのものは直接は汚れない。

**提案**: D6 の表を実測で作り直し、空にする対象を `dev_nar`・`dev_update` の 2 件に絞る。残り 5 件は語境界付きで取れた版（複数あるときは最も古い版）を書く。V10 の文言も「D6 の 2 項目は空」に直す。§12 の訂正 3（ギャップ分析の 5 件を 7 件へ直したもの）も同時に取り下げる。

**追跡**: 要件 2.7・4.1・4.2 / **証拠**: design.md §5 D6・§11 V10・§12 訂正 3

### 🔴 重大 2: 正典キーを読んでいる `crates/areka/src/placement/` が調査の視野に入っていない

**問題**: §6.3（機械で決まる 178 件）と §7.2（URL を置く場所）は、`areka-parsers`・`areka-emo-compose`・`areka/src/emo2_boot/frame/zorder_descript.rs` の 6 ファイルだけを見ている。しかし本ドメインの正典項目に当たる descript キーを、`crates/areka/src/placement/` が別途読んでいる。

| 正典項目 | 実際の読取箇所 | 設計の扱い |
|---|---|---|
| `descript_shell:seriko.zorder` | `crates/areka/src/placement/config.rs:138` | §7.2 は `emo2_boot/frame/zorder_descript.rs:47` と書くが、**同行は doc コメントの一行**であって読取ではない |
| `descript_shell:seriko.dpi` | 同 `config.rs:140`・`crates/areka/src/placement/source.rs:45` | 表に無い |
| `descript_shell:seriko.sticky-window` | 同 `config.rs:139` | 表に無い |
| `descript_ghost`／`descript_shell` の `seriko.alignmenttodesktop`／`alignmentondesktop`（`sakura.`／`kero.`／`char*.` の各形・正典側に 8 項目） | 同 `config.rs:227`・`:233-236` | 表に無い |
| バルーン descript の `dpi` | `source.rs:48` | 表に無い |

**影響**: これらの項目は `areka-parsers` の受理キー表には現れないので、§6.3 の規則だけで仕訳すると `absent` へ落ちる。実際には読んでいるので、状態（`implemented`／`vocabulary-only`／`degraded`）の判定が事実と食い違い、要件 6.1（`implemented` には URL）を満たす置き場所も無い。判定は 1 つの決めごとでは済まない——`seriko.zorder` は本番の経路（`placement/mod.rs:300` → `main.rs:209`・`:235` → `emo2_boot/frame/wiring.rs:194`）まで届いているが、`build_placement_config` 自体は `#[allow(dead_code)]` の足場（`config.rs:125`）でテストからしか呼ばれていない形もあり、項目ごとに見分けが要る。

**提案**: §6.3 の証拠を集める範囲に `crates/areka/src/placement/` を加え、§7.2 の表に `placement/config.rs`・`placement/source.rs` の行を足す。`seriko.zorder` の置き場所は `config.rs:138` へ直す。行数はいずれも余裕がある（`config.rs` 703 行・`source.rs` 284 行）。

**追跡**: 要件 2.1・2.2・6.1・6.2 / **証拠**: design.md §6.3・§7.2

### 🔴 重大 3: `/// ukadoc:` を置く位置の大半が式の途中で、doc コメントとして成立しない

**問題**: §7.2 は「各引き行の直上」に 1 行ずつ置くと定める。ところが該当する行の多くは**式の途中の引数**か**構造体リテラルの欄**である。例——`crates/areka-parsers/src/balloon/parse.rs:71-101` の 28 か所はすべて `WindowPosition::new(...)`・`Origin::new(...)`・`FontColor::new(...)` などの引数、`crates/areka-parsers/src/package/resolve.rs:69-72` の 4 か所は `GhostNames { ... }` の欄である。この位置に `///` を置くと、`rustc` は `unused doc comment`（`rustdoc does not generate documentation for expression fields` / `for expressions`）の警告を出す。本レポートで同型の最小コードを実際にコンパイルして確認した。

**影響**: 上流契約 要件 5.1・付録 A.3 が凍結した書式は `/// ukadoc: <正典 URL>` である。書式どおりに置くと、54 行のうち少なくとも 32 行が警告を生み、しかも rustdoc には出ないので「doc コメント」としては空振りする。ワークスペースに `deny(warnings)` の設定は無いので既存テストは赤にならない（要件 6.8 は形式上は満たす）が、ビルドのたびに新しい警告が並ぶ。

**提案**: 置き場所を「項目（関数・定数・構造体の欄の定義）」の直上へ寄せるか、式の途中では `//` にする、のどちらかを設計で明示的に決める。後者を選ぶ場合は上流契約の書式から外れるので、§12 と同じ形で「上流へ回す是正候補」として記録する（要件 10.8）。あわせて §11 の V11 の収集規則を「`ukadoc:` の語ではなく URL を伴う行だけを拾う」と明記する（上流契約 要件 5.6。現に URL を伴わない `/// ukadoc:` の行が `crates/areka-parsers/src/sakura/decode_tests.rs:438` に 1 件ある）。

**追跡**: 要件 6.1・6.3・6.8 / 上流契約 要件 5.1・5.3・5.6 / **証拠**: design.md §7.2・§11 V11

## 設計の良い点

1. **骨組みの生成手順が本当に再現できる**。§6.1 の 9 手順をこのレポートで独立に実行したところ、担当 24 ページ 542 件・ページ別内訳（balloon 162／surfaces 137／shell 102／ghost 74／install 15／plugin 13／headline 9／surfacetable 6／spec_update_file 9／ページ全体 15）・「id はすべて ASCII で文字順とバイト順が一致する」「引用符・逆斜線・空白は 0 件」「最長 153 文字」まで設計の記載どおりに再現できた。使い捨てスクリプトを残さない判断（D1）が、生成規則を本文に全部書くことで正しく埋め合わされている。

2. **依存の向きが一方向に固定されている**。「台帳 → ブリーフィング／ソースの URL」で、逆流させないと明記し（§4）、段取り（§10）でも段 5 を段 4 より後に固定している。おかげで V11 が「ソース中の URL の集合＝台帳の `implemented` の集合」という 1 本の等式になり、どちら向きのずれも 1 つの検査で拾える。上流の道具が無い間の退避路（D7・V15）も、差し替える場所を冒頭 1 段落に閉じ込めてある。

## 依頼された観点ごとの確認結果

### (a) 生産手順の再現性と決定性

問題なし。骨組み（§6.1）・世代表（D5・§9.2）・検査（§11）の 3 本立てで、入力が同じなら結果が同じになる。V1〜V12 を「わざと 1 か所壊した写しで赤を出してから本番に当てる」と決めている点（§11 末尾）は、道具の較正として妥当。CRLF の扱い（D8）も実測どおり（`core.autocrlf` = true・`.gitattributes` 無しを確認）。

### (b) 要件 85 項目の網羅

**85/85 が §14 の対応表に載っている**。数え直した内訳は R1=12・R2=9・R3=9（3.5a を含む）・R4=7・R5=10・R6=9・R7=6・R8=8・R9=7・R10=8 で合計 85。指摘のあった 3.5a（`debug!` だけの記録は分類を変えず壊れ方を「黙って壊れる」にする）は D4 に明示があり、1.12（ライブ確認の範囲）は §9.6・§10 段 0 に落ちている。要件 6.4（語彙表の先頭にページ URL）は「本ドメインでは使わない」と D10 が理由付きで見送っているが、6.4 自体が許可の条項なので違反ではない。

### (c) 上流契約 付録 A への適合

適合している。1 項目 1 テーブルの複数行形式、欄名 `owner`・`supersedes`、状態 7 語彙、関連 6 種別、テーマ 8 つ、`alias_of` は `alias` のときだけ・指す先は `alias` でない、初期値の並び、id の文字順、`_` で分割しない id 抽出（付録 B）——いずれも §6.1・§6.2・§6.4 が付録どおりに写している。版番号の規則だけは上流が未凍結で、本 spec が仮に決める旨を §3 の表で明示しており、扱いとして正しい（中身は重大 1 のとおり要修正）。

**1 点だけ確認しきれない前提**がある。D6 は「境界事例を空にすれば上流の道具がどちらの規則を採っても上流契約 要件 6.7 の照合が赤にならない」と書くが、6.7 の条文は「カタログの版番号が 1 つ以上あるときは、台帳の登場版がその中に含まれること」であって、台帳側が空の場合を検査から外すとは書いていない。道具が緩い規則を採ってカタログに版番号を持たせた場合、空の登場版が「含まれない」と判定される余地が残る。上流の設計が着地するまでの残余の危険として §13 に 1 行足すのが安全。

### (d) file:line の逐条確認（23 群・うち 1 群が誤り）

| 引用箇所 | 結果 |
|---|---|
| `areka-parsers/src/charset/prescan.rs:54`（charset の照合） | OK |
| 同 `:51` が分割・`:52` が continue（§12 訂正 6） | OK |
| `areka-parsers/src/package/resolve.rs:64`（ghost の decode） | OK |
| 同 `:69`・`:70`・`:71`・`:72`・`:78`・`:83`（ゴースト 6 キー） | OK |
| 同 `:111-121`（bindgroup／bindoption の定数群・11 行ちょうど） | OK |
| 同 `:156`（shell の decode） | OK |
| 同 `:8`（install.txt 非接触の宣言）・`:296`（唯一の `warn!`） | OK |
| `areka-parsers/src/balloon/parse.rs:121-125` が注記・`:126` がコード（§12 訂正 2） | OK |
| 同 `:9`・`:39`・`:110`・`:113`・`:115`・`:119` | OK |
| `areka-parsers/src/kv/parse.rs:20`・`:26`・`:39` | OK |
| `areka-parsers/src/charset/decode.rs:35`・`:52`（`debug!` 2 件） | OK |
| `areka-parsers/src/shell/decode.rs:118`・`:122`・`:127`・`:132`（塊の見出し 4 語） | OK |
| 同 `:197`・`:198`・`:234-236`・`:310`・`:323`・`:334` | OK |
| 同 `:385-396`（間隔語）・`:387`・`:388`・`:391` | OK |
| 同 `:490`・`:495`・`:496`・`:501`・`:502`・`:90`（charset 素通り） | OK |
| `areka-seriko/src/table.rs:105-137`・`:106`・`:107-109`・`:110-117`・`:111-115`・`:118-127`・`:128-136`（§12 訂正 7） | OK |
| `areka-emo-compose/src/method.rs:130-132`・`:142`・`:143`・`:145`・`:148`・`:153`・`:160-161`・`:173`・`:176-177`・`:186-204`（19 行ちょうど）・`:236-248`（§12 訂正 8） | OK |
| `areka-parsers/src/package/validation_tests.rs:113`・`:122-125`・`:127`（§12 訂正 9） | OK |
| `log-capture-kit/tests/workspace_scan/mod.rs:38`（`LINE_LIMIT = 1000`）・`:82`・`:103`（§12 訂正 10） | OK |
| `areka/src/main.rs:141`（既定のログ水準 `info`） | OK |
| `areka-sylphya/src/vocab/dotted.rs:24`・`:25`・`:103-104` | OK |
| `areka-emo-present/src/balloon.rs:418`・`areka/src/emo2_boot/assets.rs:279`・`areka/src/placement/measure.rs:333` | OK |
| `doc/emo2-conformance-scope.md:73`・`:75`・`:76`・`:82`・`:92`／`:89` が空行（§12 訂正 5） | OK |
| `doc/COMPAT_ARCHITECTURE.md:122`（見出し）・`:128-207`（データ行 80 行） | OK |
| 試験用ゴーストの 7 本の配置（§12 訂正 4＝offsetdpi と wplimit は `fixtures/` 直下） | OK |
| **`areka/src/emo2_boot/frame/zorder_descript.rs:47`（`seriko.zorder` の読取）** | **誤り**。同行は `apply_descript_base` の doc コメントの一行。実際の読取は `crates/areka/src/placement/config.rs:138` |

§12 の「実測の訂正」10 件は、確認できた 8 件すべてが正しかった（訂正 1・3 は本レポートで別途扱う。訂正 3 は重大 1 のとおり誤り）。

### (e) 1 ファイル 1,000 行の上限

超えない。上限は `crates/log-capture-kit/tests/workspace_scan/mod.rs:38` の `LINE_LIMIT = 1000`、走査は `crates/` 配下の `.rs` のみ（`:82`・`:103`）。

| ファイル | 現在 | §7.2 の追加後 |
|---|---|---|
| `areka-parsers/src/package/resolve.rs` | 949 | 961 |
| `areka-parsers/src/shell/decode.rs` | 558 | 567 |
| `areka-emo-compose/src/method.rs` | 395 | 396 |
| `areka-parsers/src/balloon/parse.rs` | 168 | 196 |
| `areka/src/emo2_boot/frame/zorder_descript.rs` | 92 | 93 |
| `areka-parsers/src/charset/prescan.rs` | 60 | 63 |

重大 2 の是正で `placement/config.rs`（703 行）・`placement/source.rs`（284 行）が加わっても余裕がある。

### (f) §15 の未決 4 件

| # | 内容 | 判定 |
|---|---|---|
| 1 | 上流の版番号抽出規則 | **先送りしてよいが中身が誤り**。凌ぎ方（D6）そのものが実測と合わないので、重大 1 として先に直す |
| 2 | `surface.append`・`kero.surface.alias` の正典上の実在 | 先送りしてよい。ライブが引けないときは `implemented` にしない（D9）と決めてあり、判断が保守側へ倒れる |
| 3 | `text-decoration-canon` の「13 キー」の実体 | 先送りしてよい。台帳側で名前を確定し是正候補に回すだけで、隣接 spec を止めない |
| 4 | `ukadoc:manual_translator` の担当が 2 本に割れる | 先送りしてよい。`owner` に `translate-pipeline`・備考と `links` に `makoto-dll-host` という扱いは要件 7.1（既存 spec 名を書き、新しい追跡先を作らない）と矛盾せず、検査（V6・V7）にも影響しない |

### (g) 設計が拾えていない危険

§13 は 7 件を挙げており、スナップショットの陳腐化（実測でも `generatedAt` = 2026-08-24・ukadoc 1,749 件・38 ページを確認）・ページ跨ぎの名前の重なり・二重管理・上流の着地・行数上限は押さえてある。拾えていないのは次の 4 つ。上 2 つは重大 2・3 と同じ根である。

1. `crates/areka/src/placement/` が正典キーを読んでいること（重大 2）。
2. `///` を式の途中に置くと doc コメントとして成立しないこと（重大 3）。
3. 上流契約 要件 6.7 と「空の登場版」の関係（上記 (c) の末尾）。
4. V14 の差分検査の範囲。「変更対象が `doc/ukadoc-coverage/` 配下と `crates/` の doc コメントだけ」と書いてあるが、本 spec 自身の `.kiro/specs/areka-P0-ukadoc-survey-assets/`（`tasks.md`・本レポートなど）は必ず差分に出る。除外の対象として明記しておかないと、V14 は必ず赤になる。

## その他の気づき（重大ではない）

- §8 の担当表は要件 7.2 の文言をそのまま写して `areka-P0-windowposition-limit` を「ゴースト側 `windowposition.*`」としているが、本ドメインの正典側で `windowposition` を持つのは `descript_balloon` の 3 項目（`windowposition.x`・`.y`・`.limit`）だけで、`descript_ghost` には無い。§12 の訂正として 1 行足すのが筋。
- §7.2 の `shell/decode.rs` の欄は「9」と書きつつ 8 か所しか挙げていない（末尾が「ほか」）。上限の計算に効くので、確定させて数を合わせたい。
- §7.2 は `resolve.rs` のシェル 6 形について「定数群（`:111-121`）の直上にまとめて 6 行」と言うが、直上の `:110` は既存の doc コメントなので、6 行は `SAKURA_BINDGROUP_PREFIX` 1 個の説明文へ合流する。証拠としては拾えるが、どの定数がどの項目かは読み取れなくなる。
- V12 は「表の再生成結果とブリーフィング中の表がバイト一致」を求める。CRLF で書く決め（D8）と組み合わせるので、比較は作業ツリーのファイル同士で行うことを 1 行添えたい。
- 要件の「repo 全体で正典 URL の doc コメントは 0 件」は正しい（`ukadoc.jp` 等の URL を含む Rust の行は 0 件）。ただし URL を伴わない `/// ukadoc:` の行が 1 件ある（`crates/areka-parsers/src/sakura/decode_tests.rs:438`）ので、V11 の収集はこれを拾わないこと（重大 3 の提案の後半）。

## 最終判定

**GO（条件付き）**

**理由**: 生産手順は独立に再現でき（542 件の骨組みをこのレポートで再現できた）、要件 85 項目に漏れが無く、上流契約の付録 A にも従っている。残る 3 件はいずれも台帳を書き始める前の局所修正で直り、設計の骨格を作り直す必要が無い。

**先へ進む前に済ませること**（設計ディスカッションで裁定）:

1. 重大 1——D6 の表を実測で作り直し（空にするのは `dev_nar`・`dev_update` の 2 件）、V10 の文言と §12 訂正 3 を合わせる。**台帳の「登場した版」を埋める前に必要**。
2. 重大 2——§6.3 の証拠収集範囲へ `crates/areka/src/placement/` を加え、§7.2 の `seriko.zorder` を `placement/config.rs:138` へ直し、`seriko.dpi`・`seriko.sticky-window`・`alignmenttodesktop` 系・バルーンの `dpi` の置き場所を表に足す。**状態を仕訳する前に必要**。
3. 重大 3——式の途中に置く場合の書き方（`//` にするか、項目の直上へ寄せるか）を決め、上流契約の書式から外れる場合は是正候補として登記する。あわせて V11 の収集規則に「URL を伴う行だけ」を明記する。**段 5（ソースの URL）の前に必要**。

**次の段階**: 上の 3 件を設計ディスカッションで裁定し、design.md へ反映してから `/kiro-spec-tasks areka-P0-ukadoc-survey-assets` へ進む。
