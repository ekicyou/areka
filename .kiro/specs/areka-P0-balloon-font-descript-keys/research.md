# ギャップ分析: areka-P0-balloon-font-descript-keys

> 実施 2026-09-11（`kiro-validate-gap`）。対象ブランチ `claude/areka-p0-balloon-font-keys-b2e272`。
> 本書は**判断ではなく材料**である。案は複数提示し、最終決定は要件ディスカッション／設計フェーズに委ねる。
> 本書の数値はすべてこのブランチでの実測であり、brief・roadmap の記述をそのまま写したものは 1 件も無い。

---

## 1. 要約（5 点）

- **キー集合 14 は正典側で裏が取れた**。`doc/ukadoc-coverage/catalog.toml` の接頭辞なし `font.*` は 14 行ちょうど（`font.bold`／`font.color.b`／`.g`／`.r`／`font.height`／`font.italic`／`font.name`／`font.outline`／`font.shadowcolor.b`／`.g`／`.r`／`font.shadowstyle`／`font.strike`／`font.underline`）。台帳 `ledger/assets.toml` も同じ 14 項目で、状態は `implemented` 4・`absent` 10。要件 1.1／1.2 の前提は実測と一致する。
- **2 層マージ・接頭辞漏れ防止・未指定の区別は、すでに機構として在る**。`parse.rs` の 2 層マージはキー非依存（`descript.clone()` に画像別層を後勝ち `insert`）で、写像はその後に 1 回だけ走る。完全一致の `get` で引くので接頭辞付きキーは構造的に混ざらない。よって**要件 4.1〜4.5 を満たすための新規コードは 0 行**で、必要なのはテストによる固定だけである（先例: `vertical` の 2 層テストが「マージ改変 0 行」を証跡として明記している）。
- **最大の制約は `Font::new` の呼び出し 50 か所**。`Font::new` 50・`FontColor::new` 52・`BalloonModel::new` 43（実測）が `areka-emo-text`／`areka` にまたがって散在する。コンストラクタの引数を伸ばす案を採ると編集集合が 8 crate 以上へ広がり、brief が保証した「W13 の他 spec と共有ファイル 0」も要件 5 の「既存の解析結果を変えない」も守りにくくなる。本リポジトリには**この問題を解いた先例が 3 つある**（`with_cursor`／`with_windowposition_raw`／`with_vertical_raw` の additive ビルダ）。
- **要件に書かれた数のうち 2 つが実測と食い違う**。⑴ 要件 6.1 の「既に置かれている 9 本」は実測 **4 本**（`parse.rs` の `font.color.r`／`.g`／`.b`／`font.height` のみ。`font.name` には正典 URL コメントが無い）＝新たに置くのは **10 本**。⑵ 要件 1.3 の「3 か所」は実測 **4 文書**（`areka-P0-text-align-shadow-canon/brief.md:27` が漏れている）。どちらも要件本文の改訂を要する可能性があるため、下の「設計判断項目」へ上げる。
- **網羅調査の道具は本仕様の作業を止めない**。`vocabulary-only` は状態語彙に実在し（`model.rs`）、証拠（正典 URL）の要否を見るのは `Status::Implemented` の行だけなので、`vocabulary-only` の行に URL コメントを置いても検査は赤にならない。逆に `SourceUrlNotInCatalog`（要件 6.5）と `DomainReportStale`（要件 7.9）は実在の判定であり、URL の綴り写しと `report` の作り直しは必須である。

---

## 2. 現状の実測

### 2.1 転記層（`crates/areka-parsers/src/balloon/`）

| ファイル | 行数 | 役割 |
| --- | ---: | --- |
| `parse.rs` | 186 | 2 層マージ＋`map_merged` 写像＋`get_scalar` ヘルパ |
| `model.rs` | 529 | `BalloonModel` と sub-struct（I/O 契約の正本） |
| `parse_tests.rs` | 409 | `parse`／`parse_str` の決定論テスト |
| `model_tests.rs` | 596 | モデル型の決定論テスト |
| `validation_tests.rs` | 208 | 構造規律の検査 |
| `kv/parse.rs` | 43 | 行分割・最初のカンマで分割・trim・後勝ち・値は生文字列 |

**`map_merged` が引く `font.*` は 5 本**（`font.color.r`／`.g`／`.b` を `u8` で個別に引き `FontColor` へ、`font.name` を `Option<String>` でそのまま、`font.height` を `u32` で）。残る 9 本は引く先が無い。上流 `kv::parse_kv` は未知キーも `BTreeMap` に保持するので、値が落ちるのは転記層である（brief の記述と一致）。

**2 層マージはキー非依存**である。

- `parse()` が `descript.clone()` を作り、画像別層があれば各エントリを後勝ち `insert` して「マージ済み 1 マップ」を作る。
- `map_merged()` はそのマージ済み 1 マップからのみ引く。

つまり 3 段優先度（画像別 ＞ 既定層 ＞ 未指定）は**キーを 1 本足すごとに自動で成立する**。要件 4.1／4.2／4.3 は実装作業ではなくテスト作業である。

**接頭辞漏れ防止も構造的に成立している**。`get_scalar` は `merged.get(key)` の完全一致で、`anchor.font.color.r` のような distractor は別キーなので当たらない。`parse_tests.rs:162` の `distractor_keys_do_not_leak_into_modeled_scalars` がすでに `anchor.font.color.r`／`number.font.height`／`sstpmessage.font.height`／`cursor.font.color.r` の 4 本を固定している。要件 4.4 は**この既存テストを壊さずに影のキーを足す**形になる。

**値の保ち方の先例が 3 種ある**。

| 先例 | 型 | 判断 |
| --- | --- | --- |
| `font.color.r`／`.g`／`.b` | `Option<u8>`（`get_scalar`） | 範囲外・非数値は `None` へ降格 |
| `font.height` | `Option<u32>` | 同上 |
| `vertical`／`writing_mode`／`budoux_newline`／`windowposition.limit` | `Option<String>`（生文字列転記） | 検証・語彙判定・縮退はすべて下流 |

要件 2.5（語彙外の値を落とさず素通し）と要件 2.6（`none`・数値・未指定の 3 値区別）は、**`get_scalar` 系では満たせない**——`get_scalar::<u8>("300")` は `None` になり、宣言された事実そのものが消えるからである。これは後段の案 A/B/C の分岐点である。

### 2.2 モデル層の拡張様式（決定的な制約）

`Font` は `#[non_exhaustive]`・非公開フィールド・read-only アクセサ。`BalloonModel` には additive ビルダの先例が 3 本ある（`with_cursor` 16 箇所・`with_windowposition_raw`・`with_vertical_raw` 9 箇所）。いずれも doc コメントに「既存呼び出し側は無改変」と理由が明記されている。

**実測した呼び出し数**:

| コンストラクタ | 呼び出し箇所 | 主な所在 |
| --- | ---: | --- |
| `Font::new` | 50 | `areka-emo-text/src/`（actor・draw・layout・wrap・writing・region ほか）・`areka-emo-text/tests/`・`areka/src/input_events/` |
| `FontColor::new` | 52 | 同上 |
| `BalloonModel::new` | 43 | 同上 |

`Font::new` の引数を伸ばすと、この 50 箇所すべてが編集対象になる。brief の「編集集合＝`balloon/{parse,model}.rs`＋兄弟テスト＋`ledger/assets.toml`」と roadmap の「W13 は共有ファイル 0」を同時に破るので、**案の選択はここでほぼ決まる**。

### 2.3 網羅調査の道具（`crates/ukadoc-survey`）

- 状態語彙は 6 種（`implemented`／`vocabulary-only`／`degraded`／`absent`／`alias`／対象外）。`vocabulary-only` は実在する（`model.rs:79`・表示名「語彙のみ」）。台帳の状態語がこの語彙外だと**読み込みそのものが止まる**（`a_status_word_outside_the_seven_stops_the_ledger_read`）。
- 証拠の行の形は厳格である（`evidence/extract.rs`）: 字下げを除いた行頭が `//`／`///`／`//!` のいずれかで始まり、`ukadoc:` の後に**空白 1 つ以上＋ちょうど 1 語**。説明文を続けると拾われない。要件 6.3 はこの実装そのままである。
- **証拠の要否を見るのは `implemented` の行だけ**（`check/content.rs:104` の早期 return）。`vocabulary-only`／`absent` の行に URL コメントが在っても所見は出ない。要件 6.1 の「14 本すべてに置く」は道具と衝突しない。
- 実在する判定は 15 種。本仕様に効くのは `SourceUrlNotInCatalog`（URL がカタログに無い＝要件 6.5）と `DomainReportStale`（ドメイン別報告が台帳と食い違う＝要件 7.9）。常設テストは `crates/ukadoc-survey/tests/consistency/`（`real_repo_data_produces_no_findings` ほか）にあり、ネットワークもスナップショットも要らない。
- **優先度・束の名前・備考の文面は機械が見ていない**。要件 7.2／7.3／7.5 は人が守る規約であり、赤にはならない。ここは最終検証で全数を数え直す類の要件である（steering「全項目に○○型の要件はタスク別レビューに映らない」）。

### 2.4 台帳の現状（14 項目・実測）

| 項目 | 状態 | 担当 | 優先度 |
| --- | --- | --- | --- |
| `font.color.r`／`.g`／`.b`・`font.height` | `implemented` | `areka-P0-text-decoration-canon` | E66 |
| `font.name` | `absent` | 同上 | A11 |
| `font.bold`・`italic`・`outline`・`strike`・`underline` | `absent` | 同上 | A11 |
| `font.shadowcolor.r`／`.g`／`.b`・`font.shadowstyle` | `absent` | 同上 | A11 |

`report/assets.md` の現在値は 実装済み 42／語彙のみ 65／縮退 8／未対応 423／別名 4／合計 542、`descript_balloon` 行は 20／5／4／133／0／0／0／162。9 項目が `absent` → `vocabulary-only` へ、`font.name` が `absent` → `degraded` へ動くと（後者は 2026-09-11 開発者裁定・議題 1）、全体が 語彙のみ 74／縮退 9／未対応 413、`descript_balloon` が 20／14／5／123 になる見込み。**この数は手計算の見込みであり、道具に数えさせるまでは根拠にしない**（作り直しは道具が行う）。

### 2.5 正典の確認（ukadoc MCP・2026-09-11）

要件 3.2／3.3 が登記する既定値と語彙を、正典本文で照合した。

| 項目 | 正典本文 | 既定値 |
| --- | --- | --- |
| `font.shadowstyle` | 「フォントの陰落ち色のスタイル。offset で右下にずれた表示、outline で縁取り」・2.5.27 | `offset` |
| `font.shadowcolor.r` | 「フォントの陰落ち色赤(0〜255) none で無効化」 | `none` |
| `font.bold` | 「常に太字フォント。0 で無効/1 で有効」 | `0` |
| `font.outline` | 「常に外枠線をつける。0 で無効/1 で有効」 | `0` |

要件 3.2／3.3 の登記値は**正典と一致する**（`font.outline` が 0/1 の 5 本目であることも確認済み）。設計フェーズで残る確認は `font.name`＝`ＭＳ ゴシック`／`font.height`＝`12`／`font.color.*`＝`0` の 3 系統だけである。

---

## 3. 要件 → 資産の対応表

タグ: **済** 既存資産で満たされる／**要追加** 新規コード・文書が要る／**未知** 研究待ち／**制約** 既存構造からの縛り。

| 要件 | 対応する資産 | ギャップ |
| --- | --- | --- |
| 1.1 キー集合 14 | `catalog.toml:113-126`（14 行）・`ledger/assets.toml:1541-1756`（14 塊） | **済**（照合元が実在・数も一致） |
| 1.2 内訳 5／9／0 | `parse.rs` の `map_merged`（5 本）・台帳（`implemented` 4＋`absent` 10） | **済**（ただし「写像済み 5」と「台帳 implemented 4」は別勘定。`font.name` は写像済みだが `absent`＝要件 7.6 が理由を持つ） |
| 1.3 「13 キー」の是正 3 か所 | 実測 4 文書（下の設計判断 ①） | **要追加＋未知** |
| 1.4 食い違い時はカタログを照合元に | `catalog.toml` | **済** |
| 2.1／2.3 0/1 系 5 本＋`shadowstyle` の保持 | 写像先が無い | **要追加** |
| 2.2／2.6 影色 3 成分の個別保持・`none`／数値／未指定の 3 値区別 | `FontColor`（`Option<u8>`）は**この 3 値を表せない** | **要追加（型の選択が論点）** |
| 2.4 未指定と宣言値の区別 | `Option<T>` 直持ちの既存規律 | **済（様式）／要追加（対象）** |
| 2.5 語彙外値の素通し | `get_scalar` は `None` へ降格させてしまう | **制約（案の分岐点）** |
| 2.7 失敗しない・警告しない | `parse` は `Result` を返さず panic しない | **済** |
| 3.1 既定値を代入しない | 既存規律（未指定は `None`） | **済** |
| 3.2〜3.4 既定値表の登記・下流への割り振り | 正典で照合済み（§2.5） | **要追加（文書）** |
| 4.1〜4.3 2 層優先度 | キー非依存の 2 層マージ（`parse.rs`） | **済（機構）／要追加（テスト）** |
| 4.4 接頭辞漏れ防止 | 完全一致 `get`＋既存 distractor テスト | **済（機構）／要追加（影のキーを足す）** |
| 4.5 非写像キーの無視 | 完全一致引きゆえ自然に成立 | **済** |
| 5.1／5.2 既存解析結果の不変 | 既存 5 キーの写像に触れない設計が可能 | **制約（案の選択で決まる）** |
| 5.3 既存テストの期待値を緩めない | `parse_tests.rs`・`model_tests.rs`・`validation_tests.rs` | **済（規律）** |
| 5.4 見た目が変わらない | 読み手が未着地＝消費側が無い | **済** |
| 6.1 14 本の URL コメント | 実測 4 本（`font.name` は未設置） | **要追加 10 本**（設計判断 ②） |
| 6.2 URL はカタログから写す | `catalog.toml` の `url` 欄 | **済** |
| 6.3 行の形 | `evidence/extract.rs` の実装どおり | **済（規律）** |
| 6.4 定義箇所だけに置く | 既存 4 本は `parse.rs` の写像行に在る（設計判断 ③） | **要追加／未知** |
| 6.5 カタログに無い URL 0 件 | `SourceUrlNotInCatalog` 判定＋常設テスト | **済（検査が在る）** |
| 7.1 9 項目を `vocabulary-only` へ | 状態語彙に実在 | **要追加** |
| 7.2／7.3 備考の書き換え | 機械は見ない＝人手＋最終の全数確認 | **要追加（見落としやすい）** |
| 7.4 影 4 項目の担当変更 | 現在 14 項目すべて `text-decoration-canon` | **要追加** |
| 7.5 優先度据え置き・束名の是正 | A11 の束名「読む経路が無い」が 9 項目で偽になる（設計判断 ④） | **要追加／未知** |
| 7.6 `implemented` 4 項目の据え置き | 台帳の現状と一致 | **済** |
| 7.7 `font.name` を `degraded` へ | 状態語彙に `degraded` は実在（`model.rs:80`・表示名「縮退」） | **要追加**（2026-09-11 開発者裁定・議題 1） |
| 7.8 `report/assets.md` の作り直し・`summary.md` 不触 | `cargo run -p ukadoc-survey -- report`（ドメイン別 4 本のみ） | **済（道具が在る）** |
| 7.9 検査 0 件 | `cargo test -p ukadoc-survey` | **済** |
| 8.1〜8.4 下流への引き渡し | 下流 2 spec は **brief のみ**（requirements.md 不在＝未着地）を実測 | **済（判定材料）／要追加（申し送り文書）** |
| 9.1〜9.6 決定論テスト | `parse_tests.rs` の既存様式をそのまま延長できる | **要追加** |
| 9.7 「14 である」ことを判定にする | 該当する既存テストが無い | **要追加（設計判断 ⑤）** |
| 9.8 出力を切り詰めない | steering 既知の罠（`\| tail` が exit code を隠す） | **済（規律）** |

---

## 4. 実装案

### 案 A — `Font` の既存コンストラクタを伸ばす（**非推奨**）

`Font::new(name, height, color, bold, italic, outline, strike, underline, shadow_color, shadow_style)` のように引数を足す。

- ✅ 型が 1 つで済み、アクセサの所在が分かりやすい。
- ❌ **`Font::new` の呼び出し 50 箇所を全部書き換える**。`areka-emo-text`（15 ファイル以上）・`areka` にまたがり、brief の編集集合と roadmap の「W13 共有ファイル 0」を同時に破る。
- ❌ 要件 5.1／5.2 の「既存の解析結果を変えない」の証明コストが跳ね上がる（50 箇所の書き換えが挙動不変であることを別途示す必要が出る）。
- ❌ 引数 10 個の位置引数はこのリポジトリのどの先例よりも長い。

### 案 B — additive ビルダで足す（**先例に忠実**）

`Font` に additive フィールドを足し、`Font::with_style(...)`／`Font::with_shadow(...)`（または `BalloonModel::with_font_extras(...)`）で相乗りさせる。`Font::new` の署名は不変。

- ✅ **`with_cursor`／`with_windowposition_raw`／`with_vertical_raw` と同じ流儀**。3 度採られた判断であり、doc コメントに理由まで残っている。
- ✅ 呼び出し 50 箇所が無改変＝要件 5.1／5.2 が構造で満たされる。`#[non_exhaustive]` と `Default` 派生により後方互換。
- ✅ 編集集合が `balloon/{parse,model}.rs`＋兄弟テストに閉じる（brief の Constraints どおり）。
- ❌ 「取り出し口」が `font().name()` と `font().style().bold()` のように 2 段になる可能性がある（下流の読みやすさの論点＝設計判断 ⑥）。
- ❌ additive フィールドが 4 本目になる。`BalloonModel` のフィールドが増え続けることの是非は設計で一度点検すべき。

**値の型の下位選択**（案 B 内の分岐・要件 2.5／2.6 に直結）:

| 下位案 | 中身 | 評価 |
| --- | --- | --- |
| B-1 全部 `Option<String>` の生文字列転記 | `vertical`／`writing_mode`／`windowposition.limit` と同じ | 要件 2.5（語彙外の素通し）と 2.6（`none`／数値／未指定）を**そのまま**満たす。判定はすべて下流。型が 1 種で済む |
| B-2 0/1 系 5 本は `Option<u8>`、影色は `Option<String>`、`shadowstyle` は `Option<String>` | 既存 `font.color.*` の流儀を部分的に継承 | `font.bold,2` が `Some(2)` として残るので要件 2.5 は満たすが、`font.bold,yes` は `None` へ落ちて**宣言の事実が消える**＝要件 2.5 違反 |
| B-3 影色だけ `Option<ShadowComponent>`（`None`／`Keyword(String)`／`Value(u8)` の 3 値 enum） | 3 値を型で表す | 要件 2.6 に最も忠実。ただし転記層が語彙（`none`）を知ることになり、steering「parser は転記層・解釈は下流」と摩擦する |

**B-1 が最も規律に合う**。転記層が値の形を一切知らずに済み、要件 2.5／2.6／2.7 がいずれも「文字列をそのまま持つ」だけで成立する。要件 2.6 の「3 つを互いに区別できる形」は `None`／`Some("none")`／`Some("64")` で満たされる。

### 案 C — 新しい型を独立させる（ハイブリッド）

`model.rs` に `FontDecoration`（0/1 系 5 本）と `FontShadow`（影 4 本）の 2 型を新設し、`BalloonModel` へ additive に載せる（`Font` には触れない）。

- ✅ `Font` を全く触らないので要件 5.1 の証明が最も軽い。
- ✅ **下流 2 spec の分担（書体 10 ＝ `text-decoration-canon`／影 4 ＝ `text-align-shadow-canon`）が型の境界と一致する**＝要件 3.4／8.4 の申し送りが型の形で表現される。
- ✅ `model.rs` の行数増を新しい節に閉じ込められる（529 → 750 前後の見込み・1,000 行番人の余裕内）。
- ❌ `font().bold()` ではなく `decoration().bold()` になり、「バルーンの書体設定」が `Font` と別の型に分かれる。概念の一体性は下がる。
- ❌ 型が 2 つ増える（`#[non_exhaustive]`・`Default`・アクセサ一式）。

**案 B（B-1）と案 C は排他ではない**。`Font` に additive で載せるか `BalloonModel` に載せるかだけの違いであり、いずれも `Font::new` の 50 箇所を触らない。設計フェーズで決めるのは「取り出し口の見え方」であって、実現可能性ではない。

---

## 5. 規模と危険度

| 区分 | 判定 | 根拠 |
| --- | --- | --- |
| 規模 | **S**（1〜3 日） | 新規ロジックは「9 本を引いて持つ」だけ。2 層マージ・漏れ防止・寛容写像はすべて既存機構。作業量の実体はテストと台帳・文書の整備 |
| 危険度（コード） | **低** | 完全一致引きの追加は既存キーに触れない。`Font::new` を触らない案なら呼び出し 50 箇所が無改変 |
| 危険度（文書・台帳） | **中** | 要件 7.2／7.3／7.5 は機械が見ない人手の書き換えで、14 項目×複数行にわたる。steering の既知の罠「全項目に○○型の要件はタスク別レビューに映らない」に正面から該当する |
| 危険度（数の齟齬） | **中** | 要件 6.1 の「9 本」と 1.3 の「3 か所」が実測と食い違う（§1・§6）。着手前に決着しないと、実装が要件どおりでも検証で赤くなる |

**行数の見張り**: `model.rs` 529・`model_tests.rs` 596・`parse_tests.rs` 409。1,000 行番人（`crates/log-capture-kit/tests/file_length_guard_test.rs`）の対象。案 C で型 2 つを足すと `model.rs` が 750 前後、テストを 9 キー分足すと `model_tests.rs` が 800〜900 に達し得る。**着地時点で余裕が 100 行を切るなら、兄弟ファイルへのテーマ分割を設計に織り込むこと**（`<stem>_<テーマ>_tests.rs`・structure.md の導出規則に従う）。

---

## 6. 実測と要件の食い違い（要ディスカッション）

| # | 要件の記述 | 実測 | 影響 |
| --- | --- | --- | --- |
| ⒜ | 要件 6.1「既に置かれている 9 本を含めて 14 本」 | 既設は **4 本**（`parse.rs` の `font.color.r`／`.g`／`.b`／`font.height`）。`font.name` にも無い | 新規に置くのは **10 本**。要件本文の数の訂正が要るか、「9 本」を別の意味（新規に置く 9 本＝影 4＋0/1 系 5）と読むかの確認が要る。後者と読んでも `font.name` の 1 本が誰にも数えられない |
| ⒝ | 要件 1.3「3 か所（本仕様 brief・分割元 brief・roadmap）」 | 「基底 13 キー」を書く**生きた文書は 4 本**。漏れているのは `.kiro/specs/areka-P0-text-align-shadow-canon/brief.md:27` | 是正漏れが 1 件出る。なお `roadmap.md` は「13 キー」とは書かず `:124` に「残り 8 キー」とだけ書く。`roadmap-history.md:749` にも「descript 13 キー」が在るが、steering が history を**非改変**と定めているので対象外と考えられる |
| ⒞ | 要件 7.5「束の名前だけを実態に合わせ」 | A11 の束名は「台詞の書体・読む経路が無い」。9 項目が `vocabulary-only` になると偽になる。一方 `font.name` は**現に読まれている**のに同じ A11＋同じ束名のまま据え置き（要件 7.6） | 束名を変えると `font.name` だけが古い束名に残るか、読まれているのに「読む経路が無い」束に残るかの二択になる。台帳の規則「同じ束の項目には同じ優先度」は保てるが、束名と実態の対応は 1 件ほつれる |

---

## 7. 設計判断項目（要件ディスカッションへ）

> いずれも「答えで作業が変わる」ものに絞った。決めるのは開発者であり、本書は選択肢と各案の帰結だけを示す。
>
> **2026-09-11 要件ディスカッションでの決着**——①②④ は実測が答えを一意に決めたため、要件本文を直して閉じた（①＝既設 4 本・新規 10 本／②＝生きた 4 文書に広げ、履歴と完了済みは対象外／④＝`A11` の 10 項目すべてで束名を改める。現在の束名は `font.name` についてすでに偽だったため、これが嘘を残さない唯一の形）。⑤ は開発者裁定で案 ⒜ に決着（下記）。**設計フェーズへ残るのは ③⑥⑦⑧ の 4 件**である。
>
> **2026-09-13 設計フェーズでの決着**——③⑥⑦⑧ はいずれも §10 の設計判断（DD1〜DD4）で閉じた。design.md が正本であり、本節は経緯の記録として残す。
>
> **本書が拾えていなかった 2 件**（要件ディスカッションで追加・いずれも開発者裁定済み）——⑼ `font.name` の台帳状態 `absent` は実態と合っていない（現に読まれ描画に使われており、効かないのは 2 点だけ＝縮退）。`degraded` へ是正する（要件 7.7・議題 1）。⑽ `doc/ukadoc-coverage/briefing-assets.md` の是正候補の段が、13 キーの是正によって件数（6 か所）と引き取り先（`text-decoration-canon`）の両方で古くなる。同じコミットで書き換える（要件 1.5・議題 3）。

1. **要件 6.1 の「9 本」をどう読むか**。実測は既設 4 本・未設置 10 本。⒜ 要件の数を 4／10 へ訂正する、⒝ 「新規に置く 9 本」と読み替えて `font.name` の 1 本を別扱いにする、⒞ 実装は 14 本を揃えるだけとし数の記述は設計で正す——のいずれか。**要件本文は確定済みなので、訂正が要るなら要件ディスカッションで決める必要がある。**
2. **要件 1.3 の是正先を 3 か所から 4 か所へ広げるか**。`text-align-shadow-canon/brief.md:27` を含めるかどうか。含めないと「基底 13 キー」の記述が 1 本生き残る。`roadmap-history.md` は非改変の方針があるため対象外とする前提でよいか。
3. **正典 URL コメントの置き場**。既設 4 本は `parse.rs` の写像行に在る（`get_scalar` 呼び出しの直前）。要件 6.4 は「定義箇所だけに置き、呼び出し側には置かない」と言うが、本仕様では「キーを引く行」が実質の定義箇所である。⒜ 既設と同じく `parse.rs` に 14 本を揃える（一貫性が高い）、⒝ `model.rs` のフィールド定義側に置く（「定義箇所」の語に忠実だが既設 4 本と場所が割れる）、⒞ 両方に置く（機械は重複を赤にしないが、要件 6.4 の趣旨に反する）。
4. **台帳の束名と `font.name` の扱い**（§6 ⒞）。⒜ 9 項目を新しい束名へ移し `font.name` は現状の束名に残す、⒝ `font.name` も含めた 10 項目の束名を「読む経路はあるが使う側が無い／描画が正典どおりでない」の形に整える、⒞ 束名を変えず備考の文だけで実態を説明する。**優先度 A11 は要件 7.5 が据え置きと定めているので、変えるのは名前と備考だけ。**
5. ~~**「14 である」ことの判定の形**（要件 9.7）~~ **決着: 案 ⒜**（2026-09-11 開発者裁定・議題 2）。転記層のテスト内にキー名の表を持ち要素数 14 を判定する。カタログとの突合は**意図的に買わない**——areka は SSP の複製ではなく別のアプリであり、正典への追従は実際の要望が出た時点で判断する。実装側の後退は赤に、正典側の増加は緑のまま。編集集合は brief の約束どおりに保たれる。以下は決着前の材料である。⒜ 転記層のテストで、写像したキー名の一覧を定数として持ち、その要素数が 14 であることを `assert` する（カタログとは切れる＝カタログが増えても気付かない）、⒝ テストが `doc/ukadoc-coverage/catalog.toml` を読み、接頭辞なし `font.*` の行数と写像キー数を突き合わせる（カタログ追随するが、パーサ crate がドキュメントを読む依存が生まれる）、⒞ `ukadoc-survey` 側の常設テストに 1 本足す（道具の所有物になるが、本仕様の編集集合が `crates/ukadoc-survey` へ広がる）。**steering「検査は表示するだけでなく判定させよ」「母数 0 の緑は恒真」に照らすと ⒝ か ⒞ が筋。編集集合との兼ね合いが論点。**
6. **取り出し口の見え方**（案 B と案 C の選択）。⒜ `Font` に additive で載せる（`font().bold()` 相当の 1 段）、⒝ `FontDecoration`／`FontShadow` の 2 型を `BalloonModel` に載せる（下流 2 spec の分担と型の境界が一致）。どちらも `Font::new` の 50 箇所を触らない。**下流が読む形を先に決める話なので、要件 8.4 の申し送り内容と一体で決めるのが自然。**
7. **値の型**（案 B-1／B-2／B-3）。要件 2.5（語彙外の素通し）と 2.6（`none`／数値／未指定の 3 値）を同時に満たすのは **B-1 の全生文字列転記**が最も素直で、`vertical` の先例と同じ。数値として持ちたい要望があるなら、`font.bold,yes` のような値が黙って消える帰結を受け入れるかの確認が要る。
8. **着地順の判定時点**（要件 8.3）。下流 2 spec（`text-decoration-canon`・`text-align-shadow-canon`）は**現時点で brief しか無い**（`requirements.md` 不在＝未着地）ことを実測した。よって要件 8.1 の「先に着地する」側になる見込みだが、要件 8.3 は「着地時点の実測で判定」と定めているので、**最終タスクで再測定する手順を設計に明記するか**を決める。

---

## 8. 設計フェーズへ持ち越す研究項目（Research Needed）

- `font.name`＝`ＭＳ ゴシック`／`font.height`＝`12`／`font.color.*`＝`0` の既定値を正典本文で再照合する（`shadowstyle`／`shadowcolor`／`bold`／`outline` は本書で照合済み）。
- `report/assets.md` の作り直し後の実数（語彙のみ 74／未対応 414 の見込み）を、道具の出力で確定させる。§2.4 の数は手計算の見込みであり、**道具に数えさせるまでは根拠にしない**。
- 1,000 行番人に対する着地後の余裕を実測し、余裕が乏しければテストのテーマ分割を設計に含める。
- 要件 7.8 の「統合担当への申し送り」の置き場（`ukadoc-coverage-roadmap` の brief への追記か、本仕様の文書内の節か）を決める。

---

## 9. 次の段

~~要件ディスカッションで上の設計判断 8 件に決着を付けた後、`/kiro-design areka-P0-balloon-font-descript-keys` で設計フェーズへ進む。~~ 済み（2026-09-13・design.md 生成）。次は設計検証（`kiro-validate-design`）→ 設計ディスカッション → `/kiro-spec-tasks`。

---

## 10. 設計フェーズの記録（2026-09-13・`kiro-spec-design`）

### 10.1 要約

- **Feature**: `areka-P0-balloon-font-descript-keys`
- **Discovery Scope**: Extension（既存の転記層への拡張・軽量ディスカバリ）。外部依存の追加は 0 のため WebSearch は不要。正典の照合だけ ukadoc MCP で行った。
- **Key Findings**:
  - §8 に残していた既定値 3 系統を正典本文で照合した。`font.name`＝「デフォルトはＭＳ ゴシック」・`font.height`＝12（ピクセル）・`font.color.r`＝0。要件 3.2 の登記値はすべて正典と一致（`font.shadowcolor.r`＝none も再確認）。
  - リポジトリ内のバルーン定義（`crates/areka-emo-text/examples/fixtures/emo2-vertical*/descript.txt`・`crates/pilot/examples/shiori-host-32/fixtures/emo2*/descript.txt`）に未写像 9 キーの宣言は **0 件**。既存のフィクスチャ適合テストは新フィールドが `None` のままなので赤にならない（要件 5.3 の前提が実測で立つ）。
  - 新しい 2 型を読む本番コードは **0 か所**（`areka-emo-text/src/draw.rs` の `ResolvedFont::resolve` は `font()` の name/height/color しか読まない）。要件 5.4 の根拠であり、台帳の備考に書く「どのログも出ない」根拠でもある。
  - 網羅調査の道具は担当 spec の実在を見ない（`check/structure.rs` の owner は台帳のドメイン所属の検査であり、`owner` 欄の spec 名は判定しない）。影 4 項目の担当を `text-align-shadow-canon`（brief のみの spec）へ変えても赤にならない。
  - 下流 2 spec（`text-decoration-canon`・`text-align-shadow-canon`）はこのブランチ上で `brief.md` のみ＝先着の見込み。判定は最終タスクで再測定する（DD4）。

### 10.2 調査ログ

#### 正典既定値の再照合（§8 持ち越し）
- **Context**: 要件 3.2 の登記値のうち `font.name`／`font.height`／`font.color.*` が未照合だった。
- **Sources**: ukadoc MCP（`descript_balloon` の `font.name,フォント名`・`font.height,数値`・`font.color.r,数値`・`font.shadowcolor.r,数値`）。
- **Findings**: 上記のとおり全一致。`font.name` の本文は「バルーンのフォルダに置いたフォントファイルも指定可能（SSPのみ）」「カンマ区切りで…優先度順」＝台帳 `font.name` 備考の ⒜⒝ の根拠そのもの。
- **Implications**: design.md の引き渡し表（C7）に既定値をそのまま登記。転記層は代入しない。

#### 転記層の実形と拡張点
- **Context**: 9 キーをどこへどう足すか。
- **Sources**: `crates/areka-parsers/src/balloon/parse.rs`（186 行）・`model.rs`（529 行）・`mod.rs`（26 行）・`parse_tests.rs`（409 行）・`model_tests.rs`（596 行）・`validation_tests.rs`（208 行）。
- **Findings**: `map_merged` の正典 URL コメントは全部（幾何・`font.color.*`・`font.height`・`cursor.*`）「キーを引く行の直前」に在る。`font.name` だけ説明コメントのみで URL 行が無い。生文字列転記の先例は `writing_mode`／`budoux_newline`／`vertical`／`windowposition_raw` の 4 本。additive ビルダは `with_cursor`／`with_windowposition_raw`／`with_vertical_raw` の 3 本。テストモジュールは `mod.rs` から `mod parse_tests;` 等で直接宣言する歴史的形式。
- **Implications**: DD1（URL は `parse.rs` に揃える）・DD2（additive 2 型）・DD6（既存テストファイルへ追加）。

#### 1,000 行番人に対する余裕（§8 持ち越し）
- **Findings**: 着地後見込みは `parse.rs` ~220・`model.rs` ~650・`parse_tests.rs` ~640・`model_tests.rs` ~670。いずれも余裕 ≥300 行。番人は `crates/log-capture-kit/tests/file_length_guard_test.rs`（`LINE_LIMIT` 1,000）。
- **Implications**: テーマ分割は不要。着地時の実測が 1,000 行に迫るときだけ `<stem>_<テーマ>.rs` へ分ける（DD6。steering の目安は 1,000 行の 1 つだけで、中間の閾値は無い——設計検証で「900 行」が独自の数値だと指摘され訂正）。

#### 申し送りの置き場（§8 持ち越し）
- **Findings**: `areka-P0-ukadoc-coverage-roadmap` は同じ W13 で並走中。その brief に追記すると共有ファイルが生まれる（roadmap「W13 は共有ファイル 0」）。`doc/ukadoc-coverage/README.md` は `report/summary.md` を統合担当の仕事と定めている。
- **Implications**: DD8——申し送りは design.md の C7 と完了時の PR 本文に書く。統合担当の文書は触らない。

#### 網羅調査の道具の当たり
- **Findings**: 判定 15 種（`check/finding.rs`）のうち効くのは `SourceUrlNotInCatalog` と `DomainReportStale`。証拠の要否は `implemented` だけ（`check/content.rs`）。状態語彙は `README.md` の表どおり（`vocabulary-only`＝「名前だけ登記してあり、受け取っても何もしない」・`degraded`＝「動くが正典どおりではない。どう違うかを note に書く」）。`report` の作り直しはドメイン別 4 本を一括で作る＝他ドメインに差分が出ないことを `git status` で確かめる手順を設計に置いた。

### 10.3 設計判断（design.md DD1〜DD8 の根拠の詳細）

#### Decision: 正典 URL の置き場（§7 ③）
- **Alternatives**: ⒜ `parse.rs` のキーを引く行（既設 4 本と同じ）／⒝ `model.rs` のフィールド定義／⒞ 両方。
- **Selected**: ⒜。
- **Rationale**: 同ファイルの他の全 URL がこの位置。要件 6.4 の「定義箇所」はこのリポジトリでは「キーを引く行」。⒞ は重複で趣旨に反し、⒝ は既設と割れる。
- **Trade-offs**: `model.rs` を読む人は URL を見ない（`parse.rs` を 1 回辿れば足りる）。

#### Decision: 取り出し口の見え方（§7 ⑥）
- **Alternatives**: 案 B（`Font` に additive で載せる）／案 C（`FontDecorationRaw`／`FontShadowRaw` を `BalloonModel` に載せる）。
- **Selected**: 案 C。
- **Rationale**: `Font` を触らないので要件 5.1 が構造で成立（`Font::new` 50 呼出・`BalloonModel::new` 43 呼出とも無改変）。型の境界が下流 2 spec の分担（書体 10／影 4）と一致し、要件 3.4／8.4 の申し送りが型の形で表現される。`with_cursor` と同じ流儀。
- **Trade-offs**: 「バルーンの書体設定」が `font()`・`font_decoration_raw()`・`font_shadow_raw()` の 3 口に分かれる。深さは `cursor().style()` と同じ 1 段。

#### Decision: 値の型（§7 ⑦）
- **Alternatives**: B-1 全部 `Option<String>`／B-2 0/1 系を `Option<u8>`／B-3 影色を 3 値 enum。
- **Selected**: B-1。
- **Rationale**: 要件 2.5（語彙外の素通し）と 2.6（`none`／数値／未指定）を「文字列をそのまま持つ」だけで満たし、転記層が語彙を知らずに済む。B-2 は `font.bold,yes` が消える。B-3 は転記層が `none` を知る。
- **Follow-up**: 下流が数値として読みたい場合は下流で `parse::<u8>()` する（既定値の適用と同じ層で行う）。

#### Decision: 着地順の判定時点（§7 ⑧）
- **Selected**: 最終タスクで再測定し、判定表（design.md C7）で決める。今の実測は先着の見込み。
- **Rationale**: 要件 8.3 が「着地時点の実測」と定める。判定表を固定しておけば思い込みで決めない。

#### Decision: 「14 である」の判定の形（要件 9.7・裁定案 ⒜ の具体化）
- **Selected**: `FONT_BASE_KEYS: [&str; 14]` を持ち、14 本すべてに固有の値を入れて `parse()` し、キー→アクセサの対応関数で全部が読み戻せることを判定する。
- **Rationale**: 要素数だけの判定は配列の型で恒真になる。「読み戻せる」を判定にすれば写像を 1 本消すと赤になる。カタログは読まない（要件 9.8）。較正手順（1 本外して赤を確かめる）を設計に置いた。

#### Decision: テストの置き場
- **Selected**: 既存の `parse_tests.rs`／`model_tests.rs` へ追加（新ファイル 0）。
- **Rationale**: 余裕 ≥300 行。ファイルを増やすほどの分量ではない。

#### Decision: 台帳の束名（要件 7.5・裁定 ④ の具体化）
- **Selected**: A11 の 10 項目を「台詞の書体・読めるが正典どおりに描かれない」で揃える。
- **Rationale**: 9 項目（読めるが使う側が無い）と `font.name`（読めて使うが正典どおりでない）の両方を 1 つの名で包む。`implemented` 4 項目の束は無改変。

#### Decision: 統合担当への申し送りの置き場
- **Selected**: design.md C7 と完了時の PR 本文。`ukadoc-coverage-roadmap` の brief は触らない。
- **Rationale**: 同ウェーブ並走の spec と共有ファイルを作らない。

### 10.4 総合（synthesis）

- **一般化**: 要件 2.1／2.2／2.3 は「生文字列を個別 `Option` で持つ」という 1 つの形の 9 回の適用であり、既存の 4 先例（`vertical` ほか）と同じ。新しい抽象は作らない。
- **作るか採るか**: 新規の外部依存 0。道具（`ukadoc-survey`）・番人（`file_length_guard_test`）はいずれも既存を使う。
- **簡素化**: 3 値 enum・数値型・`Font` の改変・カタログ突合・新テストファイル・統合担当の文書編集をいずれも採らなかった。判断分岐の新設は 0。

### 10.5 リスクと対処

- 後着 spec（`emo-text-canon-residue` 項目 14・`anchor-tag-canon`）との `map_merged` 隣接——9 本を 1 塊で置き、後着が rebase する。
- `text-decoration-canon` の brief 7 か所の是正——同 spec が並走中に brief を編集していれば隣接行の rebase。意味的衝突なし。
- 束名・備考は機械が見ない——最終検証で 14 項目を全数読み直す手順を design.md に置いた。
- 統合担当への申し送りの到達性——README の役割分担と PR 本文に依る。

### 10.6 参照

- ukadoc `descript_balloon`: `font.name,フォント名`／`font.height,数値`／`font.color.r,数値`／`font.shadowcolor.r,数値`／`font.shadowstyle,形態指定`／`font.bold,0/1`／`font.outline,0/1`（URL は `doc/ukadoc-coverage/catalog.toml` の当該行）。
- `doc/ukadoc-coverage/README.md`——状態語彙の表・証拠の行の形・`report/summary.md` の担当。
- `.kiro/steering/structure.md`——Parser Crate の規律（転記層・`Option` で未指定・`#[non_exhaustive]`）・Unit Tests の兄弟ファイル規約・1,000 行の目安。
- `.kiro/steering/roadmap.md`——W13 干渉台帳（共有ファイル 0・後着 rebase）。

---

## 11. `main` 取り込み後の改訂記録（2026-09-17）

### 11.1 何が起きたか

タスク承認直後（`a6804540`）に開発者の指摘で `origin/main` を取り込んだ（`7ab2ad89`・衝突 0）。`main` には `areka-P0-text-decoration-canon`（PR#148）と `areka-P0-ukadoc-coverage-roadmap`（PR#147）の完了が入っており、本仕様の前提が **6 点**崩れていた。設計検証・タスク生成をやり直す前に、要件→設計→タスクの順で改訂した（開発者裁定 09-17「推奨で。要件→設計→タスク分解まで実行」）。

### 11.2 崩れた前提と実測（すべて取り込み後のブランチで測った）

| # | 旧前提 | 実測 | 帰結 |
|---|---|---|---|
| ⑴ | 下流 2 spec とも brief のみ＝本仕様が先着 | `text-decoration-canon` は `completed/`。受け口 `look.rs::LookLayers::from_balloon(…, font_overrides, disable_overrides)` が在り、本番の呼び出し（`draw.rs::ResolvedFont::resolve_with_background`）は両引数に空の列を渡している | **後着確定**＝本仕様が配線する（DD4 改訂・要件 8 全面改訂） |
| ⑵ | 見た目を変えない（5.4） | 配線すれば `font.bold,1` は太字になる | 「9 キーと `disable.font.*` を書いていないバルーンの見た目を変えない」へ書き直し（5.4）。効くことは 5.5 として明示。フィクスチャの宣言は 0 件（`grep -rlE '^font\.(bold|…)'`・`disable\.font` とも 0） |
| ⑶ | 台帳 10 項目の担当は `text-decoration-canon`・優先度 `A11` | 担当は完了アーカイブ時に本仕様へ移送済み。優先度は 15 項目とも `A5`（全体の並べ直し） | 7.4・7.5 を実測へ（担当＝その状態を着地させた spec、という慣行を `implemented` 28 項目で確認） |
| ⑷ | 是正対象は生きた 4 文書 | `text-decoration-canon` の brief は `completed/` へ（非改変の側）。`main` が本仕様の brief に「残り 8 キー」を 1 か所書き足した | 3 文書 7 か所へ（1.3・1.5・C6） |
| ⑸ | `font.name` の縮退は 2 点 | カンマ区切りの候補列は `resolve_with_background` が記述順のまま `TextLook::name` へ渡す | 残るのはフォントファイルの 1 点（7.7）。引受先は起票しない（開発者方針） |
| ⑹ | `disable.font.*` は対象外 | 完了済み spec の台帳備考と最終検証が「読み取りは `balloon-font-descript-keys`」と本仕様を名指し。受け口 `disable_overrides` も用意済み | 本仕様が引き受ける（開発者承認 09-17・推奨案 A）。2.8・2.9・4.6・7.10・8.2 を追加 |

### 11.3 新しい設計判断（DD9〜DD11）と改訂（DD4・DD7・DD8）

- **DD9 無効表示層の型**——`DisableFont { font: Font, decoration: FontDecorationRaw, shadow: FontShadowRaw }` の束 1 つ。書体名・大きさ・色は既存 `Font` を値で再利用し（`get_scalar` の縮退規則を継ぐ）、残り 9 本は基底と同じ 2 型。新しい値の型を増やさず「同じキーは同じ形」。却下: 14 フィールドを平らに並べる型（`Font` と重複）。
- **DD10 配線の形と記録**——純粋モジュール `balloon_overrides.rs` に `font_overrides`／`disable_overrides` を置き、宣言されたキーだけを `\f` と同じ形のトークン列へ写す。受け口の `apply_overrides` は綴り誤りを**黙って飛ばし**、doc で「記録はバルーン定義を読む側が出す」と本仕様へ申し送っていた。転記層はログを出さない契約なので、配線が公開 API（`apply_font_tag`・`LookLayers::from_balloon`）で**事前検証**して `warn!` を 1 度出す。判定の実体は受け口 1 か所に留まる（配線は語彙表を持たない）。**設計検証で是正（09-17）**——初稿は「`Err` の条件は綴りだけで決まるので `LookLayers::default()` を土台にしてよい」と書いたが、相対・百分率の `height` は「今効いている大きさ」に依存する（`look.rs` の `apply_height`）。土台をバルーン定義から組んだ実際の 2 層にし、受け口と同じ順序で探り用の `TextLook` へ適用する形へ改めた（W9 で較正）。却下: 受け口を改変して飛ばした理由を返す形（完了済み spec の面を触る）・`draw.rs` へ直書き（COM 無しでテストできない・737 行を伸ばす）。
- **DD11 影 4 キーは渡さない**——受け口は `shadowcolor`／`shadowstyle` を所有外として見た目を変えない。3 成分を 1 トークンへ束ねる形と `none` の扱いは影まわりの仕様の語彙判断なので、本仕様が先に固定しない。W6 で「渡さない」を零として判定する。
- **DD4 改訂**——着地順は実測で確定（書体＝後着・影＝先着）。「最終タスクで再測定」は不要になり、最終検証で影まわりの状態を 1 度だけ再確認する判定表に置き換えた。
- **DD7 改訂**——優先度が全項目 `A5` に揃い束名が優先度の鍵ではなくなったので、状態ごとに既設の束名へ揃える（`implemented`／`vocabulary-only`／`degraded` の 3 つ）。「束の順位: N」の文は触らない。
- **DD8 改訂**——統合担当が完了し、README が「台帳を触った人が `report-summary` を走らせる」と定めた。本仕様が `summary.md` を作り直す。

### 11.4 受け口の実測（配線の設計の根拠・`look.rs`）

- `apply_font_tag` が受ける語彙: `bold`／`italic`／`underline`／`strike`／`outline`（`parse_switch`: `true`/`1`/`false`/`0`/`default`/`disable`）・`sub`／`sup`・`height`・`color`（`parse_color`: 3 成分か 1 語）・`name`（候補列）・一括の `default`／`disable`。所有外 `UNOWNED_KEYS = ["align","valign","shadowcolor","shadowstyle"]`＋`cursor*`／`anchor*` は `Ok(Some(Note::Unowned))`。未知キーは `Err(REASON_UNKNOWN_KEY)`。`outline` は `Ok(Some(Note::VocabularyOnly))`。
- `Err` の条件のうち `REASON_BAD_SWITCH`・`REASON_VALUE_COUNT`・`REASON_COMPONENT_COUNT`・`REASON_UNKNOWN_KEY` は値の綴りだけで決まるが、**`height` の相対・百分率は層の値に依る**（`apply_height`: `current.height + delta` が `0` 以下なら `Err`）。事前検証の土台は既定の層ではなく実際の層でなければならない（設計検証の指摘 1）。
- 兄弟テストの結び方: クレートは `lib.rs` にテストモジュールを宣言せず、本番ファイルの末尾で `#[cfg(test)] #[path = "…"] mod …;` と結ぶ（`look.rs`・`draw.rs` の末尾・`structure.md`）。
- 束名「台詞の書体・読めるが正典どおりに描かれない」は台帳 4 本に 0 件＝新設（設計検証の指摘 3）。
- `apply_overrides` の `base` は差し込み前の層の複製で、`default`／`disable` の語は差し込み前の層を指す。正典の descript にこの語は現れない。
- `LookLayers`・`TextLook` のフィールド、`TextLook::ukadoc_default()`、`apply_font_tag`、`FontTagIssue` はすべて `pub`。
- `lib.rs` の `PURE_SOURCES`（54 本）に新モジュールと兄弟テストを登録しないと `every_source_file_is_either_scanned_or_explicitly_excluded` が赤になる。`draw.rs` は `SOURCES_OUTSIDE_THE_PURE_SCAN` 側。

### 11.5 台帳の実測（09-17）

15 項目とも `priority = "A5"`。`absent` 10 の `owner` は本仕様、`implemented` 4 は `text-decoration-canon`、`disable.font.*` は `vocabulary-only`・`text-decoration-canon`。「13 キーと書くが」の一文は `implemented` 4 項目にだけ残る。束名は「読む経路が無い」（順位 11）・「正典どおりに動く」（66）・「名前だけ受けて使わない」（37）の 3 種。`report/*.md`・`ledger/*.toml` の保存形は `i/lf`（作業ツリーは `w/crlf`）。

### 11.6 参照（追加）

- `crates/areka-emo-text/src/look.rs`——`LookLayers::from_balloon`・`apply_overrides`・`apply_font_tag`・`UNOWNED_KEYS`・`parse_switch`。
- `crates/areka-emo-text/src/draw.rs`——`ResolvedFont::resolve_with_background`（受け口の呼び出し・空の列 2 つ）。
- `.kiro/specs/completed/areka-P0-text-decoration-canon/tasks.md`——9.2／9.4／最終検証の申し送り（`absent` 10 行の移送・影の所有の裁定・`disable.font.*` の引受先）。
- ukadoc `descript_balloon` `disable.font.(フォント定義),(指定)`（SSP 2.5.51）。

## 12. 実装着手前のベースライン（2026-09-17・タスク 1）

- `git submodule update --init vendors/pasta` → `git submodule status` は ` 048d646c… vendors/pasta (v0.1.6-1-g048d646c)`（先頭 `-` 解消）。
- `cargo test -p areka-parsers` exit 0——`Running unittests src\lib.rs` 1 本＋Doc-tests。435 passed / 0 failed。
- `cargo test -p areka-emo-text` exit 0——`Running` 15 本。843 passed / 0 failed。
- `cargo test -p ukadoc-survey` exit 0——`Running` 4 本。725 passed / 0 failed。
- `cargo run -p ukadoc-survey -- check` exit 0——「食い違い 0 件」・証拠のある項目 263 件。
- `cargo test -p log-capture-kit --test file_length_guard_test` exit 0——6 passed / 0 failed。
- 出力は切り詰めずファイルへ保存し、`Running`／`test result:` を全行集計した。着手前の赤は 0 件＝切り分け不要。

## 13. 較正①（転記層・タスク 3.4・2026-09-17）

`parse.rs` をスクラッチへ複製して一時的に壊し、`cargo test -p areka-parsers` を走らせてから複製で書き戻した（`git diff -- crates/areka-parsers/src/balloon/parse.rs` は空）。

| 壊し方 | 結果 | 赤になったテスト |
|---|---|---|
| 1a: `font.strike` の引きを `None` に | exit 101・449 passed / 5 failed | T11 `font_base_key_table_has_fourteen_entries_and_each_is_read_back_by_the_mapping`（`基底 font.strike / left: None / right: Some("108")`）・T1 `font_decoration_raw_five_keys_transcribed_verbatim`・T7・T8・T9 |
| 1b: `disable.font.strike` の引きを `None` に | exit 101・452 passed / 2 failed | T11（`disable.font.strike / left: None / right: Some("208")`）・T13 `disable_font_keys_are_transcribed_in_the_same_shape_as_base` |
| レビュー側: `disable.font.strike`／`underline` の引きを交差 | 赤 | T11・T13 |
| レビュー側: テストの読み戻し関数の `"font.strike"` を `underline()` へ | 赤 | T11 |
| レビュー側: 表のキー名を綴り誤り（`font.outline`） | 赤（未知キーで panic） | T11 |

書き戻し後は 454 passed / 0 failed。

## 14. W9 の前提が転記層で崩れた（2026-09-17・タスク 4.1 実装時）

- **実測**: `parse.rs` は `disable.font.height`（および `font.height`）を `get_scalar::<u32>` で読む。`-10`・`+4`・`150%` は `None` へ落ち、`balloon_overrides::overrides` へ届かない。配線に届く大きさは常に非負整数の絶対値で、受け口 `look.rs` の `apply_height` で「今効いている大きさ」に依存する `Relative`／`Percent` の分岐には入らない。基底の列は飾り 5 本だけで大きさを動かさない。
- **帰結**: 設計検証の Critical Issue 1（土台を既定層で代用すると受け口と判定が食い違う）は、`descript.txt` から到達する入力では起こらない。土台を実層にする実装（DD10）は受け口との一致を構造で保つためそのまま残す（コスト 0・将来 `height` の読み方が変わっても崩れない）。
- **W9 の差し替え**: 到達しない経路を檻にしない（「檻は到達する経路を踏ませよ」）。W9 は到達する受け口判定 `disable.font.height,0`（`apply_height` の「正でない」で `Err`）→列に在り `warn!` 1 件、`disable.font.height,20` → 0 件へ改める。
- **較正③の後半**（土台を既定層へ差し替えて W9 だけ赤）は、転記層を通る入力では判定が変わらず赤を作れないので行わない。前半（事前検証を外すと W3・W4・W9 の警告判定が赤）は行う。
- 判断区分: 勝者が明白な how（到達しない入力は檻にできない）ゆえ開発者議題にせず、design.md の W9 行と較正③・tasks.md 4.3／4.4 を追随させて結果を報告する。
