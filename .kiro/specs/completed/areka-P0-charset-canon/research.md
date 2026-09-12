# ギャップ分析: areka-P0-charset-canon

> 作成: 2026-09-11（`kiro-validate-gap`）。対象: 確定済み `requirements.md`（12 要件）と現行コードの差。
> 引用は「何の定義行か」で示し、行番号は補助として添える（同じ日の別 spec が同じファイルを触ると行番号はずれる）。
> 本書は情報と選択肢を並べるものであり、最終決定は要件ディスカッションと設計フェーズで行う。

## 0. 要約（3〜5 点）

1. **既存パターンは揃っている**。ファイル層の文字コード解決（`areka-parsers/src/charset/`＝`decode(bytes, default)`・`DefaultEncoding` の固定写像・`encoding_rs::Encoding::for_label`）が正本として稼働しており、通信層はそれと同じ `encoding_rs` 0.8.35（workspace 依存・現在の利用者は `areka-parsers` のみ）に載せるだけでよい。ラベルの別名寛容（`sjis`／`windows-31j`／`shift-jis` 等 → Shift_JIS）・前後空白・大小文字の寛容は `for_label` が既に備える。
2. **欠けている能力は 4 つ**: ⑴ `shiori3.rs` の `Charset` が `enum { Utf8 }` で符号化・復号ともに UTF-8 固定（応答の `Charset` ヘッダは読み飛ばし）、⑵ 交渉状態（次に使う文字コード）を持つ場所が無い（`ShioriConnection::get/notify` が呼び出しごとに `Shiori3Client::new` を作り捨てる）、⑶ descript の `shiori.encoding`／`shiori.forceencoding` を読む場所が無い（`ShioriMount` は `dir`／`file` のみ）、⑷ surfaces.txt の本番読取 2 経路が `read_to_string`（UTF-8 決め打ち）。
3. **推奨は brief の ⒝（`&'static encoding_rs::Encoding` の newtype）＋ 交渉状態を `ShioriConnection`（kanade `real.rs`）に持たせる混成案**。型は host32 crate（`shiori3.rs`）に置き、初期値は `ShioriMount` に足す 2 フィールド（生ラベル・転記のみ）→ `areka-ghost` の boot でラベル解決とログ → `real_connect` → `ShioriConnection` へ渡す。`GhostBootOptions`（27 か所の構築点）・32bit helper・IPC・SHIORI/4 in-proc 経路は触らない。
4. **要件の内側に 1 つ矛盾候補**: 裁定 ⑵ の推奨 (a)（不正なバイト並びを代替文字で吸収）を採ると、既存テスト `parse_invalid_utf8_is_parse_error`（不正 UTF-8 → `Err(Parse)` を期待）の期待値が変わる。要件 4.8「既存テストは期待値を変えずに緑」と要件 4.7／10.2(a) は両立しない。要件ディスカッションで決める（§8 項目 1）。
5. **持ち越し調査は 3 件**: ⒤ emo2 の最初の要求（prefetch の `username` GET か boot 系列の最初のイベントか）が本当に ASCII のみか、⒥ pasta が要求の `Charset` ヘッダ値を検査するか（`lua_request.rs`「req.charset: utf-8であること」の実体）、⒦ 実機確認に使う有界の自動終了の仕組みの正本（完了仕様 `emo2-conformance-e2e` の実機手順を設計で引く）。

## 1. 調査の範囲と方法

- 入力: `spec.json`（ja）・`requirements.md`・`brief.md`・steering（`product.md`／`tech.md`／`structure.md`／`logging.md`／`roadmap.md` の W13 行と干渉台帳）。
- 手段: Grep／Glob／Read によるコード実測。外部依存は cargo レジストリの `encoding_rs-0.8.35` ソース（`c:\rust\cargo\registry\src\index.crates.io-*\encoding_rs-0.8.35\src\lib.rs`）を直接読んだ。Web 参照は不要だった。
- 対象クレート: `shiori-host32-host`（codec・client）・`areka-kanade`（`shiori/real.rs`）・`areka-ghost`（`runtime.rs`／`shiori_wiring.rs`／`shiori_inproc.rs`）・`areka-parsers`（`charset/`／`package/`／`shell/`）・`areka`（`emo2_boot/assets.rs`／`placement/measure.rs`）・`ukadoc-survey`（台帳検査）・`doc/COMPAT_ARCHITECTURE.md`・`doc/ukadoc-coverage/ledger/*.toml`。

## 2. 要件 ⇄ 既存資産の対応表

タグ: **Missing**＝実装が無い／**Constraint**＝既存構造が課す制約／**Unknown**＝設計前に調べる／**Ready**＝既存資産で足りる。

| 要件 | 既存資産（実測） | ギャップ |
|---|---|---|
| 1.1〜1.2 対応集合＝ファイル層と同一・別名寛容 | `encoding_rs::Encoding::for_label`（`lib.rs` の `for_label` は前後の ASCII 空白を捨て小文字化して照合。ラベル表に `sjis`／`x-sjis`／`ms_kanji`／`shift-jis`／`shift_jis`／`windows-31j`／`csshiftjis` → `SHIFT_JIS`）。`areka-parsers/src/charset/decode.rs` の `decode` が同関数で解決 | **Ready**（通信層が同じ関数を呼べばよい）。**Constraint**: `shiori-host32-host` は `encoding_rs` に未依存（§5.6） |
| 1.3 既定の固定写像 | `charset/model.rs` の `DefaultEncoding::to_encoding`（`Ansi→SHIFT_JIS`／`Utf8→UTF_8`）。ただし **`pub(crate)`**（doc に「公開面に出さない」・`#[allow(dead_code)]` の注記は既に古い＝`decode` が消費済み） | **Constraint**: 通信層の初期値へ同じ写像を渡すには ⑴ `to_encoding` を `pub` にする（`areka-parsers` 1 行＋doc）か ⑵ `areka-ghost` に同じ 2 腕の `match` を書く（写像の二重化）。§8 項目 4 |
| 1.5／10.3 UTF-16／replacement の限界 | `encoding_rs` の `Encoding::output_encoding()` が `REPLACEMENT`／`UTF_16BE`／`UTF_16LE` を `UTF_8` へ写す（`lib.rs`）。`encode()` の戻り値の第 2 要素が実際に使われた文字コード | **Ready**（判定は `enc.output_encoding() != enc` の 1 式）。裁定 (a) なら「解決できないラベル」と同じ扱いにして理由だけログで区別する |
| 2.1〜2.6 descript の 2 キー | `package/model.rs` の `ShioriMount { dir, file }`（`#[non_exhaustive]`・`Clone/Debug/PartialEq/Eq`）。`package/resolve.rs` の `resolve` が `parse_kv` の `BTreeMap` から `shiori` を取り出す行の直下に 2 キーを足せる。構築点は `resolve.rs` 1 か所＋`model_tests.rs` 3 か所（crate 内のみ） | **Missing**。**Constraint**: `resolve.rs` は 959 行（§5.3） |
| 2.7 emo2 の最初の要求 | emo2 の ghost descript（`crates/pilot/examples/shiori-host-32/fixtures/emo2/ghost/master/descript.txt`）は `charset,UTF-8` のみ・`shiori.encoding` 無し。pasta は全応答に `Charset: UTF-8`（`pasta_shiori/src/shiori.rs` の応答組立 2 か所・`error.rs` のエラー応答）。testdll（`shiori-host32-testdll`）の `parse_request` は request line と `ID` しか見ない＝`Charset: Shift_JIS` を送っても 200 が返る | **Unknown ⒤**: 最初の要求の References が ASCII のみか（§7）。**Unknown ⒥**: pasta が `Charset` 値を検査するか |
| 3.1〜3.4 要求の符号化・ヘッダ位置・UTF-8 差分 0 | `shiori3.rs` の `build_request` は `String` を組んで `into_bytes`。`Charset:` は request line 直後の最初のヘッダ（要件 3.3 は既に満たす）。`Encoding::encode(&str)` は UTF-8 なら `Cow::Borrowed`（バイト列は同一・差分 0 が構造的に成り立つ） | **Missing**（`header_value()` を `Encoding::name()` へ・`into_bytes` を `encode` へ）。`Encoding::name()` は `"UTF-8"`／`"Shift_JIS"`／`"EUC-JP"`／`"ISO-2022-JP"`（ukadoc 表記と一致） |
| 3.5／10.1 変換できない文字 | `encode()` は変換できない文字を 10 進の数値文字参照 `&#NNNN;` に置換し、`had_errors: bool` を返す（**個数は返さない**） | **Constraint**: 要件 3.5 の「置換した文字数」を得るには `new_encoder()`＋`encode_from_utf8_without_replacement` の小さなループで `Unmappable` を数える（十数行）か、要件を「置換の有無」へ緩める。§8 項目 6 |
| 3.6 helper・IPC 変更 0 | `shiori-host32-helper/src/shiori_proxy.rs` 冒頭「意味を持たない生バイト列を…運ぶだけ」。IPC は `&[u8]` を運ぶのみ | **Ready**。ただし同ファイルの doc「`request` は UTF-8（下流・本仕様非呼出）」が古くなる（§8 項目 9） |
| 4.1〜4.8 応答の復号と交渉 | `parse_response(bytes, request_charset)` は `std::str::from_utf8` で即時失敗・`Charset` ヘッダは読み飛ばし（末尾コメント「Charset / Reference0 / Marker / 未知ヘッダ等は読み飛ばす」）。ステータス行・ヘッダ名の解析は復号後の `&str` に対して行う | **Missing**（復号前に ASCII でヘッダ走査 → 解決 → 全体復号 → 既存の行解析）。復号は `Encoding::decode_without_bom_handling`（BOM 判定で宣言と違う文字コードへ飛ばないため）を候補にする。§8 項目 7 |
| 5.1〜5.3 状態の持続・NOTIFY 非採用 | `areka-kanade/src/shiori/real.rs` の `ShioriConnection { pub window, pub helper }`（構築点 2: `areka-ghost/src/shiori_wiring.rs` と `areka-kanade/tests/kanade/real_helper_test.rs`）。`ShioriBackend` の `get/notify` は `&mut self` → 状態を持てる。接続はセッションごとに 1 回作られる（要件 5.2 は構造で成立） | **Missing**（§5.2） |
| 5.4 in-proc 変更 0 | `areka-ghost/src/shiori_inproc.rs` の `build_input`（`charset: Charset::Utf8`）と `map_get_outcome`（`parse_response(.., Charset::Utf8)`） | **Constraint**: 型を変えると 2 か所が機械的に追随（`Charset::UTF_8` 定数へ）。挙動は不変 |
| 6.1〜6.6 surfaces.txt | `areka/src/emo2_boot/assets.rs` の「シェル: surfaces.txt 読取 → parse → bake」の `read_to_string`（同ファイルは `charset::{DefaultEncoding, decode}` を **既に import 済み**）。`areka/src/placement/measure.rs` `build_shell_assets` の `read_to_string`（doc に「emo2 の surfaces.txt は `charset,UTF-8` 宣言＝UTF-8 読み」）。`areka-parsers/src/shell/decode.rs` は `charset` 行を寛容スキップ（冒頭 doc） | **Missing**（各 1 行: `fs::read` → `decode(&bytes, DefaultEncoding::Ansi)`）。エラー型（`BootWiringError::ShellRead { source: io::Error }`・`PlacementError::Measure`）は `fs::read` の `io::Error` をそのまま載せられる |
| 7.1〜7.4 ログ | steering `logging.md`（構造化フィールド優先・`warn!`＝後退・`info!`＝ライフサイクル・`debug!`＝開発者向け）。`decode.rs` の `tracing::debug!(label = %name, ..)` が先例 | **Missing**。「同じ内容につき 1 回」（7.4／4.4）は交渉状態の側に「既に警告したラベル」の小さな集合を持つ |
| 8.1〜8.5 正典文書と台帳 | `doc/COMPAT_ARCHITECTURE.md` §7 の行「SHIORI: …、Charset交渉の具体。」・§8 の表（列: 項目／裁量／根拠／出典 spec）。台帳: `ledger/shiori.toml` `Charset:1`＝`degraded`（owner 本 spec）・`Charset:2`＝`absent`（owner 空）、`ledger/assets.toml` `shiori.encoding`／`shiori.forceencoding`＝`absent`・`descript_shell_surfaces:charset`＝`absent`（いずれも owner 本 spec）。`implemented` には**定義箇所に `// ukadoc: <URL>` 1 行**の証拠が要る（`doc/ukadoc-coverage/README.md` §3・`ImplementedWithoutEvidence`） | **Missing**。**Constraint**: `build_request` の `Charset:` 書き出し行には現在 ukadoc コメントが **無い**（9 行は Method／Sender／Status／ID／Reference／ステータスコード／Value／ErrorLevel／ErrorDescription）。surfaces `charset` の「定義箇所」をどこにするかは §8 項目 8 |
| 9.1〜9.8 決定論テスト | `shiori3.rs`／`client.rs` はファイル内 `#[cfg(test)] mod tests`（純関数・窓不要・x64 のみ）。`tests/*.rs` は i686 helper と testdll を要し不在なら panic（`shiori_request_e2e.rs` 冒頭）。ログ観測は `log-capture-kit`（11 crate が dev-dep・`shiori-host32-host` は未採用）。`areka-ghost` には `test_log_capture::capture_events` | **Ready**（置き場所と道具はある）。`shiori3.rs` は 598 行→新テストは兄弟ファイル（`#[path = "shiori3_charset_tests.rs"]`・kanade `real.rs` と同型）に置けば 1,000 行の上限に余裕 |
| 11.1〜11.3 実機確認 | `areka-ghost/tests/ghost/real_pasta_test.rs`（env `HOST32_PASTA_DLL` で有効化・OnBoot 一周・有界待機）。Shift_JIS のゴーストは開発者指定 | **Unknown ⒦**: areka 本体の有界自動終了の正本（`crates/areka/src` に `AUTO_EXIT`／`exit_after` の語は無い→完了仕様 `emo2-conformance-e2e` の実機手順を引く） |
| 12.1〜12.5 変えないもの | 1,000 行番人の例外表（`log-capture-kit/tests/file_length_guard_test.rs`）に本 spec が触るファイルは無い。既存の scripted fake（`ShioriWiring::Custom`／kanade `FakeBackend`）は codec を通らない | **Ready** |

## 3. 欠けている能力の要点

1. **文字コードを表す型**: `Charset` は `enum { Utf8 }`（`ShiftJis` はコメント上のシームのみ）。`header_value()` は `const fn` で `"UTF-8"` 固定。
2. **符号化・復号**: `build_request` は `String::into_bytes`、`parse_response` は `from_utf8`。復号前に応答の `Charset` を読む段が無い。
3. **交渉状態**: `Shiori3Client<'a>` は `&'a ParentMessageWindow` を借用する短命の値。`ShioriConnection` が窓を所有しつつ client を長期保持すると自己参照になるため、状態は client ではなく接続側に置く必要がある。
4. **descript 2 キーの読取**: `ShioriMount` に無い。`GhostBootOptions` には `default_encoding` があるが通信の文字コードは渡っていない。`shiori_wiring.rs` に charset の語は無い。
5. **surfaces.txt**: 2 経路とも `read_to_string`。
6. **ログ**: 上記のどれにも文字コードに関するログ行が無い。
7. **依存**: `shiori-host32-host` に `encoding_rs` が無い。

## 4. 実装アプローチの選択肢

### 4.1 型の持ち方（brief ⒜／⒝）

| 案 | 内容 | 利点 | 難点 |
|---|---|---|---|
| ⒜ enum 拡張 | `Charset { Utf8, ShiftJis, Other(&'static Encoding) }` 等 | 既存の `match` 形を保てる | 「任意の文字コード」を列挙で表す矛盾。ラベル→variant の表を通信層が別に持つ（要件 1.1「独自の一覧を持たない」に反する）。`Other` を足した時点で enum の意味が無い |
| ⒝ newtype（推奨） | `pub struct Charset(&'static encoding_rs::Encoding)`。定数 `Charset::UTF_8`／`SHIFT_JIS`、`for_label(&str) -> Option<Charset>`（`output_encoding != self` は `None`）、`name()`、`encode(&str)`、`decode(&[u8])` | ファイル層と同じ基盤・表を持たない・`Copy/Eq/Debug` は `Encoding` が備える（`impl PartialEq/Eq/Hash/Debug for Encoding`） | `shiori-host32-host` に `encoding_rs` 依存を足す（§5.6）。in-proc 呼出点 2 か所の綴りが変わる |

### 4.2 交渉状態と配線（A 拡張／B 新設／C 混成）

| 案 | 内容 | 利点 | 難点 |
|---|---|---|---|
| A: 既存を拡張 | `ShioriConnection` に `charset: Charset` と `forced: bool`（＋警告済みラベル集合）を直接持ち、`get` の中で `Shiori3Client` に渡し、返ってきた応答の `Charset` で更新する | ファイル数が最少 | 交渉の規則（forced なら無視・未知は継続・切替ログ）が kanade の `real.rs` に書かれる＝規則と codec が別 crate に散る。kanade のテスト fake は trait しか見ないので規則のテストは host32 側でしか書けないのに、規則は kanade にある |
| B: 新設 | host32 crate に純粋な `CharsetNegotiator`（`new(initial, forced)`・`current()`・`on_response_header(Option<&str>)`・ログと重複抑止を内包）を新設し、`Shiori3Client::get(&mut negotiator, ..)` が消費。`ShioriConnection` は `negotiator` を 1 フィールド持つだけ | 規則と codec が同じ crate に閉じ、窓無しでテストできる（要件 9.3「応答による採用が次の要求に現れる」を純関数の連続呼出で固定できる）。kanade 側は 1 フィールド＋引数渡し | 型が 1 つ増える |
| C: 混成（推奨） | 型 ⒝＋B の negotiator を host32 に置き、初期値の決定（descript 2 キーの解決とログ）は `areka-ghost` の boot に置く。`ShioriMount` は生ラベル 2 つを転記するだけ（「parser は転記層」の方針どおり） | 各層の責務が既存の方針と一致（parsers＝転記・ghost＝結線と起動ログ・host32＝wire の規則） | 触る crate が 5 つ（parsers／ghost／kanade／host32／areka）。ただし各 crate の差分は小さい |

### 4.3 初期値の配線経路（案 C の内側）

```
package::resolve（parsers）: ShioriMount { dir, file, encoding: Option<String>, force_encoding: Option<String> }  ← 生ラベル転記＋ukadoc URL コメント 2 行
        ↓ mount.shiori.clone()（runtime.rs の ShioriWiring::Helper 腕・既存）
areka-ghost boot: Charset::for_label(force) → for_label(encoding) → 既定（default_encoding の写像）
                  警告ログ（解決不能）＋情報ログ（決定と根拠）        ← 要件 2.4／2.6
        ↓ real_connect(helper_exe, shiori, initial)（引数 1 つ追加）
ShioriConnection { window, helper, negotiator }                     ← 構築点 2 か所（shiori_wiring.rs・real_helper_test.rs）
        ↓ get: Shiori3Client::get(&mut self.negotiator, ..) ／ notify: negotiator.current() を読むだけ
shiori3 codec: build_request(charset) → encode ／ parse_response(bytes, request_charset) → ヘッダ走査 → decode
```

`GhostBootOptions`（27 構築点）と `resolve_kanade_config` は触らない。in-proc は `InProcBackend` が `Charset::UTF_8` 固定のまま negotiator を持たない。

## 5. 個別論点（brief／要件が挙げたズレの検証と提案）

### 5.1 in-proc 呼出点 2 つ（検証: 一致）
`shiori_inproc.rs` の `build_input` と `map_get_outcome` が `Charset::Utf8` を渡している。⒝ では `Charset::UTF_8` へ綴りが変わるだけで挙動は不変。`build_input` の doc「`build_request` の出力は…常に有効な UTF-8」は `charset` が UTF-8 なら引き続き真（`encode` は UTF-8 で `Cow::Borrowed`）。要件 5.4「変更 0」は「挙動 0」と読み、綴りの追随は許す前提で要件 4.8 の「呼び出し形の機械的な追随は可」と揃える。

### 5.2 交渉状態の置き場所（検証: 一致）
`Shiori3Client::new(&self.window)` を `get`／`notify` のたびに作り捨てているのは実測どおり。長寿命の接続オブジェクトは `ShioriConnection`（`real.rs`）で、`ShioriBackend` の各メソッドが `&mut self` なので状態を置ける。`Shiori3Client` を接続に保持する案は `&'a ParentMessageWindow` 借用との自己参照になるため採らない。提案は §4.2 の B（host32 側の純粋な negotiator を `ShioriConnection` が 1 フィールド持つ）。`spawn_shiori_actor` の `connect` closure は接続ごとに 1 回なので、要件 5.2（load ごとに初期値へ戻る）は構造で成立する。

### 5.3 `resolve.rs` 959 行（検証: 一致）
2 キーの追加は「`map.get("shiori.encoding").cloned()` ＋ ukadoc URL コメント」の 2 行×2 で 4〜6 行。**直接足しても 1,000 行未満**だが余裕は 30 行台になる。選択肢:
- (a) `resolve.rs` に直接足す（最小・余裕 30 行台）。
- (b) `package/shiori_encoding.rs`（新設・10〜20 行）に `read_shiori_encoding(&map) -> (Option<String>, Option<String>)` を置き、`resolve` から 1 行で呼ぶ。テストは `resolve_tests.rs`（300 行）へ。
- (c) `areka-ghost` で descript を読み直す——`MountModel` は kv を保持しないので二重読み込みになる。採らない。
`model.rs`（447 行）へのフィールド 2 つは問題なし。`#[non_exhaustive]` なので外部の構築点は無い（crate 内の `model_tests.rs` 3 か所を追随）。

### 5.4 surfaces.txt の 2 経路（検証: 一致）
`assets.rs` は `decode`／`DefaultEncoding` を既に import しており、`read_to_string` → `fs::read` ＋ `decode(&bytes, DefaultEncoding::Ansi)` の差し替えは 1 行ずつ。`measure.rs` は import 追加が要る。`shell::parse(&str)` の入力型は変わらない。テスト側の読取（`areka-emo-compose/src/world.rs`・`areka-seriko/src/resolve.rs`）は触らない。**shell の decode 層が `charset` 行を寛容スキップする**ので、Shift_JIS 宣言の行が解析結果に混ざることは無い。要件 9.5 のテスト固定物は各 crate 内の `#[cfg(test)]` で `Vec<u8>` 定数として持てば実ファイルは不要。

### 5.5 ukadoc URL コメント 9 行（検証: 一致・位置を確認）
`build_request`: Method（request line 直前）・Sender・Status・ID・Reference の 5 行。`parse_response`: ステータスコード・Value・ErrorLevel・ErrorDescription の 4 行。`Charset:` の書き出し行と読み取り分岐には**現在コメントが無い**ので、本 spec が `Charset:1`（要求側）・`Charset:2`（応答側）の URL を各 1 行足す（台帳を `implemented` にする証拠として必須）。組立の順序を変えない限り 9 行はそのまま残る。

### 5.6 `encoding_rs` 依存の追加可否
- workspace: `Cargo.toml` `[workspace.dependencies] encoding_rs = "0.8"`（ロック 0.8.35）。利用者は `areka-parsers` のみ（`vendors/pasta` のサンプルは別）。
- `shiori-host32-host` の依存: `shiori-host32-ipc`・`wintf-winmsg-executor`・`event-listener`・`thiserror`・`tracing`・`windows`。`encoding_rs = { workspace = true }` を足すのは既存 workspace 依存の利用者が 1 つ増えるだけで、新規の外部依存ではない（steering `tech.md`「encoding_rs (0.8): …意図的依存追加＝2026-07-02 承認済」）。`encoding_rs` は `windows` 非依存・純 Rust なので host32 crate の「純粋・決定的・`windows` 非依存」の codec 方針と合う。
- **避けるべき辺**: host32 → `areka-parsers`（parsers は上位の純パーサ群であり、host32 の下に置く理由が無い）。`DefaultEncoding` の写像を通信層で使うには `areka-ghost`（両方に依存済み）で橋渡しする（§8 項目 4）。
- i686 helper は依存を足さない（変更 0）。

### 5.7 初期 charset の配線（§4.3 のとおり）
`runtime.rs` の `ShioriWiring::Helper` 腕は `real_connect(helper_exe, mount.shiori.clone())` を呼んでおり、`ShioriMount` に 2 フィールドが増えれば追加の配線無しで届く。ラベル解決とログは `real_connect` の直前（boot 本体）か `real_connect` の内側かの 2 択——boot 本体に置くと `ShioriWiring::Custom`／`InProc` の腕と対称に「Helper のときだけ解決」となり、`InProc` は UTF-8 固定で解決を通らない（要件 5.4）。`options.default_encoding` は boot 本体にある。

### 5.8 既存テストの構造と新テストの置き場所
- `shiori3.rs`／`client.rs` の決定論テストはファイル内 `mod tests`（純関数・窓・helper 不要）。`tests/*.rs`（`shiori_request_e2e.rs` 等）は i686 成果物が無いと明示 panic するので、新しい 3 系統の往復テストは **ファイル内または兄弟ファイル**に置く（x64 のみで緑）。
- 期待バイト列は定数で書く（要件 9.1）: 「あ」＝UTF-8 `E3 81 82`／Shift_JIS `82 A0`／EUC-JP `A4 A2`（ISO-2022-JP を足すなら `1B 24 42 24 22 1B 28 42`）。
- ログ観測: host32 crate に `log-capture-kit` を dev-dep で足す（11 crate が同じ形で採用済み・出荷依存は増えない）。`areka-ghost` の 2 キー優先順とログのテストは既存の `test_log_capture::capture_events` で書ける。
- kanade の `FakeBackend`（`real_tests.rs`）は trait しか見ないので変更不要。`ShioriConnection` の構築点 2 か所（`shiori_wiring.rs`・`real_helper_test.rs`）に negotiator を足す。
- testdll は `Charset` を見ない（`parse_request` は request line と `ID` のみ）ので、既定 Shift_JIS の最初の要求でも `shiori_request_e2e` は緑のまま。helper の `main_loopback_tests.rs` の `Charset: UTF-8` 定数はバイト列を直接流す helper 側テストで、codec を通らない。

### 5.9 既存テストと裁定 ⑵ の矛盾（要件内の齟齬・要ディスカッション）
`shiori3.rs` の `parse_invalid_utf8_is_parse_error` は不正な UTF-8 バイト列に `Err(ShioriError::Parse)` を期待する。裁定 ⑵(a)（代替文字で吸収して続行）を採ると、この期待は `Ok`＋U+FFFD 混じりの `Value` に変わる。要件 4.8 は「UTF-8 の応答に対する既存テストは期待値を変えずに緑」と定めるため、(a) を採るなら 4.8 の但し書き（このテストは裁定 ⑵ に従い期待値を更新する）が要る。(b) を採れば矛盾は無いが、Shift_JIS の 1 バイトの乱れで応答全体が消える現状の性質を全文字コードへ広げることになる。

### 5.10 emo2 の最初の要求（Unknown ⒤・⒥ → **要件ディスカッションで実測・解消済み**）
要件 2.7 は「本文が ASCII のみのとき」と条件付きで書かれている。2026-09-11 の要件ディスカッションで実測した結果:
- ⒤ **解消**。boot 系列（`schedule/boot.rs`）の最初の送出は `OnInitialize` NOTIFY（`events.rs` の `on_initialize`＝「References なし」）、最初の応答待ちイベントは `username` 照会 GET（`resources.rs` の `resource_username`＝`references: Vec::new()`）。`Sender` は `areka`、`Status` は ukadoc の ASCII 語彙。したがって最初の要求（採用前に既定 Shift_JIS で送る要求）は本文 ASCII のみで、`Charset` ヘッダの値以外にバイト差は生じない。
- ⒥ **解消**。pasta の `lua_request.rs` は `Rule::key_charset => table.set("charset", value)` でテーブルへ転記するだけ。Lua 側（`pasta_lua/scripts/pasta/shiori/{entry,event/init,event/register,res}.lua`）で `req.charset` を参照するのは doc コメントのみで、値の検査・拒否は 0 箇所。応答側は `res.lua` が常に `Charset: UTF-8` を書く。
- 結論: 要件 2.7／12.1 の再検討は不要。emo2 固定物は無改変のまま、最初の応答で UTF-8 を採用して以後不変。

### 5.11 台帳の証拠の置き場所
`implemented` の証拠は「定義箇所に置かれた `// ukadoc: <URL>` 1 行」（呼び出し側には書かない・URL の後ろに語を続けない）。候補: `Charset:1`＝`build_request` の `Charset:` 書き出し行の直上、`Charset:2`＝`parse_response` の `Charset` ヘッダ判定の腕、`shiori.encoding`／`shiori.forceencoding`＝`resolve.rs`（または新設モジュール）の `map.get(..)` 行の直上、surfaces `charset`＝**定義箇所は shell の decode 層の `charset` 行スキップの腕**（`areka-parsers/src/shell/decode.rs`）か、読取経路の `decode` 呼出行か——README は「呼び出し側には書かない」と定めるので前者が規約に合う（§8 項目 8）。台帳の `owner` は着地時に `""` へ戻すか残すかは台帳 README の慣行に従う（完了 spec の PR#139〜#141 の更新形を設計で引く）。

## 6. 工数とリスク

- **工数: M**（brief の見立てと一致）。差分の中心は `shiori3.rs`（codec 2 関数＋型＋ヘッダ走査 ≒ 80〜120 行）・negotiator（≒ 60 行）・kanade `real.rs`（≒ 10 行）・`areka-ghost` boot（≒ 30 行）・parsers（≒ 10 行）・areka 2 経路（≒ 4 行）・テスト（3 系統×往復＋ログ観測＋surfaces 3 固定物 ≒ 300 行）・文書と台帳。
- **リスク: Medium**。理由: ⑴ 最初の要求のヘッダ値が UTF-8 のゴーストでも変わる（§5.10 の未確認 2 点が実機で欠陥になり得る）、⑵ 裁定 ⑵ と既存テストの齟齬、⑶ 同じ crate の別ファイルを W13 の `host32-window-thread-pump`（`parent_window.rs`）が触る（共有ファイル 0・rebase のみ）。技術面（`encoding_rs` の API・純関数の置き換え）は既知で低リスク。

## 7. 設計フェーズへの推奨と持ち越し調査

**推奨**: 型 ⒝＋案 C（host32 に `Charset` newtype と純粋な negotiator・`ShioriMount` に生ラベル 2 つ・`areka-ghost` boot で解決とログ・`ShioriConnection` が negotiator を保持）。surfaces.txt は 2 行の差し替え。新テストは兄弟ファイルに置き `log-capture-kit` を host32 の dev-dep に足す。

**Research Needed（設計で先に潰す）**:
1. ~~⒤ emo2 の最初の GET とその References が ASCII のみか~~ → **解消済み**（§5.10: `OnInitialize` NOTIFY と `username` GET はともに References なし）。
2. ~~⒥ pasta が要求の `Charset` ヘッダ値を検査・拒否するか~~ → **解消済み**（§5.10: 転記のみ・検査 0 箇所）。
3. ~~⒦ 実機確認の有界自動終了とログ検索の正本手順~~ → **設計で解消**（§9.2: areka bin は `args[1]`＝ghost root・`args[2]`＝balloon root・helper は実行ファイル隣接（`boot_config.rs` の `default_helper_exe_path`）・有界の自動終了は `AREKA_APP_SMOKE_EXIT_MS`（`main.rs` の `SMOKE_EXIT_ENV`）・出力水準は `RUST_LOG`。検体＝里々標準テンプレート（開発者が絶対パスを指定・要件 11.1）。design.md「実機確認」）。
4. ~~応答の復号に `decode_without_bom_handling` か `decode` か~~ → **設計で解消**（BOM 判定なし。宣言と食い違う BOM で交渉結果を黙って覆さないため。UTF-8 の有効な入力では `from_utf8` と同じ文字列になり既存テストの期待値が変わらない。design.md「shiori3 codec」）。
5. ~~ISO-2022-JP の要求を全文まとめて `encode` してよいか~~ → **設計で解消**（可。`Encoding::encode` は終端で ASCII 状態へ戻す。design.md「shiori3 codec」）。
6. ~~台帳の `owner` 欄と `introduced` 欄の着地時の書き方~~ → **設計で解消**（`implemented` でも `owner` は残す（先例: `choice-marker-styling` の balloon 行群）・`introduced` は版番号不明なら `""` のまま・`Charset:2` の空 owner は本 spec へ・報告は `report`／`report-summary` で作り直す（常設検査に `DomainReportStale` がある）。design.md「正典文書・台帳」）。

## 8. 設計判断項目（要件ディスカッションへ）

> 2026-09-11 要件ディスカッションでの仕分け: **項目 1 は開発者裁定（要件 10.2 と一体）**。**項目 9・10 は要件側で解消済み**（下記に結果を併記）。**項目 2〜8・11・12 は設計フェーズ（`/kiro-spec-design`）で解決した**——決定は §9.3 に 1 行ずつ記す（正本は `design.md`）。

1. ~~**裁定 ⑵ と要件 4.8 の齟齬**~~ → **開発者裁定で解消**（要件 10.2＝(a) 代替文字で吸収・警告ログ 1 行。`parse_invalid_utf8_is_parse_error` の期待値更新を要件 4.8 の唯一の例外として明記済み）。
2. **型の持ち方**: ⒝ newtype（推奨）か ⒜ enum 拡張か。裁定 ⑶ は要件側で (a) に確定済み（要件 10.3）——`Charset::for_label` が UTF-16／replacement を `None` にするか、別の失敗理由を返してログの根拠フィールドで区別するかは設計で選ぶ（要件 10.3 は区別を「してよい」とする）。
3. **交渉状態の置き場所**: B（host32 の純粋 negotiator を `ShioriConnection` が保持・推奨）か A（`ShioriConnection` に素のフィールドと規則を書く）か。
4. **既定の固定写像の共有方法**: `DefaultEncoding::to_encoding` を `pub` に格上げする（parsers 1 行＋doc の「公開面に出さない」を改訂）か、`areka-ghost` に同じ 2 腕の `match` を置く（写像の二重化・要件 1.3 の「同じ固定写像」を言葉で担保）か。
5. **descript 2 キーの読取位置**: `resolve.rs` に直接（余裕 30 行台）か、`package/shiori_encoding.rs` 新設（推奨）か。`ShioriMount` のフィールド名（`encoding`／`force_encoding`）と型（生ラベル `Option<String>`＝転記のみ）。
6. **要件 3.5 の「置換した文字数」**: `Encoder` のループで数える（十数行）か、「置換の有無」へ緩めるか。
7. **応答の復号方式**: ヘッダ走査は ASCII でバイト列を直接読む（`prescan_charset` と同じ考え方・ただし別形式なので再利用せず 15 行程度を書く）ことでよいか。復号 API は BOM 判定なしでよいか。
8. **台帳の証拠の定義箇所**: surfaces `charset` の URL コメントを shell decode 層の `charset` スキップの腕に置く（規約どおり）か、読取経路の `decode` 呼出行に置く（規約の「呼び出し側には書かない」に反する）か。
9. ~~**helper の古くなる doc コメント**~~ → **要件側で解消**（要件 12.2 を改訂: 挙動・バイト列に関わる変更 0 のまま、当該コメント 1 行の文言追随は可）。
10. ~~**要件 2.7／12.1 の再検討の条件**~~ → **解消**（§5.10 の実測で最初の要求は ASCII のみ・pasta は `Charset` 値を検査しない。再検討不要）。
11. **`Shiori3Client` の API 形**: `new(window)` を残して charset 引数付きの構築子を足すか、`get/notify` の引数で negotiator を受けるか（案 B なら後者が自然）。
12. **ログの対象名**: 既存は `target: "shiori-actor"`／`"ghost-boot"`。文字コードの決定・切替・後退のログの `target` と `event` 名を steering `logging.md` に沿って決める（例: `event = "charset_initial"`／`"charset_switched"`／`"charset_label_unresolved"`）。

## 9. 設計フェーズの記録（2026-09-11・`kiro-spec-design`）

### 9.1 要約
- **Discovery Scope**: Extension（既存系の拡張・light discovery）。外部調査は不要（`encoding_rs` 0.8.35 の API は §1 で読了済み）。サブエージェントは使わず、§2〜§5 の主張をすべてコードで再確認した。
- **Key Findings（設計を変えた再確認 4 点）**:
  1. emo2 の `shell/master/surfaces.txt` は **BOM 無し**（先頭バイト `63 68 61 72 73 65 74 2c 55 54 46 2d 38`）。`read_to_string` → `decode` の差し替えで解析結果が同一（要件 6.3）であることの根拠。BOM 付き UTF-8 の surfaces.txt では U+FEFF が消える（既知の差として登記）。
  2. `placement/measure.rs` は `emo2_boot` を import しない（バルーン側の関数は `areka_emo_present::balloon` から取る）。surfaces.txt 読取の共通ヘルパを作ると新しい依存辺が要るので、2 経路とも 1 行の差し替えにし、決定論テストは `decode`＋`shell::parse` の性質を parsers 側で固定する。
  3. `shell/decode.rs` に `charset` 専用の腕は無い（未知行として素通し）。台帳の証拠は `charset/prescan.rs` の `charset` キー一致の腕（balloon／ghost／shell の URL 3 行が既に並ぶ）に 4 行目として置く——§8 項目 8 の 2 案のどちらでもない、規約に最も合う場所。
  4. `ukadoc-survey` の常設検査には `DomainReportStale`（台帳から作り直した報告と一致しない）がある。台帳 5 行を書き換えたら `report`／`report-summary` の作り直しが必須（怠ると赤）。
- 上記のほか §2〜§5 の実測はすべて一致（`Shiori3Client::new` の呼び出し点は kanade 2＋host32 E2E テスト 5・`ParsedResponse` の構造体リテラルは client.rs テスト 8＋shiori3.rs テスト 1・host32 の既存ログは `target` を持たない・emo2 ghost descript は `charset,UTF-8` のみ）。

### 9.2 調査ログ（設計で追加した分）
- **実機確認の道具**: `crates/areka/src/boot_config.rs` の「`args[1]` = ghost root、`args[2]` = balloon root」・`default_helper_exe_path`（実行ファイル隣接）・`ghost_boot_options` が `DefaultEncoding::Ansi`＋`ShioriWiring::Helper` を組む。`crates/areka/src/main.rs` の `SMOKE_EXIT_ENV = "AREKA_APP_SMOKE_EXIT_MS"` が有界の自動終了。`emo2-conformance-e2e` の手順書は「絶対パスで起動・有界の自動終了・出力水準を記録」。
- **ログ捕捉の条件**: `areka-ghost/src/test_log_capture.rs` の doc——捕捉されるのは呼出スレッドで同期発火したイベントだけ。よって初期値の決定とログは `real_connect` が closure を返す**前**（起動スレッド）で行う。host32 の negotiator はテストが直接呼ぶので `log_capture_kit::capture` で足りる。
- **`Charset` の派生**: `encoding_rs::Encoding` は `PartialEq`／`Eq`／`Hash`／`Debug` を実装。定数は `static` の `UTF_8_INIT`／`SHIFT_JIS_INIT` への参照で作れる。
- **1,000 行**: 触るファイルの見込み最大は `resolve.rs` 963 行（直接追加）。`shiori3.rs` は約 660・`shiori_wiring.rs` は約 260。

### 9.3 設計判断（§8 項目 2〜8・11・12 の決定）

| 項目 | 決定 | 理由 |
|---|---|---|
| 2 型の持ち方 | ⒝ newtype `Charset(&'static Encoding)`。新モジュール `shiori-host32-host/src/charset.rs`。`for_label` は `Result<Charset, LabelError>`（`Unknown`／`NotEncodable`）で理由を区別し、扱いは同じ | 列挙しない＝ラベル表を持たない（1.1）。理由の区別は 5 行で済み、利用者が `UTF-16` を書いたときに「未知」と出るより正確（10.3「してよい」） |
| 3 交渉状態の置き場 | B。host32 の `CharsetNegotiator` を `ShioriConnection` が 1 フィールド持つ | 規則と codec が同じ crate に閉じ、窓無しでテストできる。kanade は状態を置くだけ |
| 4 既定写像の共有 | `areka-ghost/src/shiori_wiring.rs` の `default_charset(DefaultEncoding)`＝2 腕の `match`（テストで両腕を固定） | host32 は parsers に依存できない。`to_encoding` を公開して橋渡しすると「UTF-16 でない」検査の失敗腕が要り、固定写像では到達不能＝記録のない死んだ失敗経路になる。2 腕の定数写像の方が小さく失敗経路が無い |
| 5 descript 2 キー | `resolve.rs` に直接（2 行＋URL 2 行＝963 行）。`ShioriMount { encoding, force_encoding }: Option<String>`（生ラベル・trim しない） | 新モジュールは 4 行のために作らない。番人が 1,000 行未満を守る |
| 6 置換した文字数 | `Charset::encode -> (bytes, replaced)`。置換があったときだけ文字ごとに符号化して数える | 要件 3.5 の文言を保ちつつ通常経路の追加コスト 0。`Encoder` の状態機械ループを書かない |
| 7 復号方式 | 復号前に ASCII でヘッダ行を走査（`\n` 分割・`\r` 除去・名前を小文字化して `charset` 一致・値を trim）→ `CharsetPolicy` で復号に使う文字コードを決める → `decode_without_bom_handling` 相当で全体復号 → 既存の行解析 | 対応集合のヘッダ領域は常に ASCII 互換（UTF-16 系は型に入らない）。BOM 判定なし＝宣言を黙って覆さない・UTF-8 の既存テスト不変。`prescan_charset` は別形式なので再利用しない |
| 8 台帳の証拠 | `charset/prescan.rs` の `charset` キー一致の腕（既存 URL 3 行の隣）に 1 行 | 定義箇所であり「呼び出し側に書かない」「1 項目 1 行」の規約に合う。`shell/decode.rs` には腕が無い |
| 11 `Shiori3Client` の API | `new(window, &mut negotiator)`／`with_sender(window, &mut negotiator, sender)`。`get`／`notify` は `&mut self`・引数不変 | 接続側は `Shiori3Client::new(&self.window, &mut self.negotiator)` のフィールド別借用で自己参照を避ける。`new(window)` を残すと UTF-8 固定の別経路が残る |
| 12 ログの対象名 | host32: `target: "shiori-charset"`、ghost: 既存 `"ghost-boot"`。event＝`charset_initial`／`charset_label_unresolved`／`charset_switched`／`charset_forced_ignores_header`／`charset_unmappable_replaced`／`charset_invalid_bytes_replaced` | kanade `"shiori-actor"`・ghost `"ghost-boot"` と同じ `target`＋`event` 形。実機確認が grep する名前を design.md の表で固定 |

追加の決定（§8 に無かったもの）:
- **codec はログを出さず事実を返す**（`EncodedRequest.replaced`・`ParsedResponse.charset_header`／`decode_had_errors`）。理由: 要件 7.4「同じ内容の後退は警告 1 回」は状態を要し、純粋な codec に置けない。負けた案: codec が `warn!` を直接出す（in-proc にも波及し、重複抑止ができない）。
- **`CharsetPolicy { Negotiate(Charset), Force(Charset) }`** を codec の第 2 引数にする。負けた案: `bool honor_header` 引数（読めない）。in-proc は `Force(UTF_8)`＝従来どおりヘッダを見ない。
- **応答の status に依らず `Charset` を採用**（204・400・500 でも）。emo2 の最初の応答待ちイベント `username` 照会が 204 でも UTF-8 に切り替わる。
- **surfaces.txt の共通ヘルパを作らない**（9.1 の 2）。
- **同一性の要件 6.3 の根拠は BOM 無しの実測**（9.1 の 1）。

### 9.4 設計統合（一般化・採用／自作・簡素化）
- 一般化: 「ラベル→文字コード」「符号化」「復号」を 1 つの値型 `Charset` に寄せ、descript の宣言・応答ヘッダ・既定の 3 経路が同じ `for_label`／定数を通る。M2 の SSTP／NAR は同じ型をそのまま使える（インタフェースの一般化のみ・実装は本要件の範囲）。
- 採用: `encoding_rs`（既存・承認済み）。ラベル解決・数値文字参照・代替文字はすべて同ライブラリの既定動作。自作した部分は ASCII のヘッダ走査 15 行程度と交渉規則のみ。
- 簡素化: 新設の型は 4 つ（`Charset`・`LabelError`・`CharsetPolicy`・`CharsetNegotiator`・`EncodedRequest` を含めて 5）。新しいエラー型 0・新しい trait 0・新しい crate 0・共通ヘルパ 0。

### 9.5 リスクと対策
- 最初の要求の `Charset: Shift_JIS`（UTF-8 のゴースト）——pasta は検査しない（§5.10 実測）。他の UTF-8 SHIORI で問題が出れば `shiori.encoding,UTF-8` が ukadoc どおりの回避策（§8 に登記）。
- 同 crate 別ファイルの並走（`host32-window-thread-pump` が `parent_window.rs`）——共有ファイル 0・rebase のみ。
- `ParsedResponse`／`build_request` の形の変更が in-proc・host32 E2E に波及——機械的追随のみ（要件 4.8／5.4 が許容）。`Charset` の変更で E2E（i686 依存）が壊れていないかは x64 の `cargo test --workspace` では見えない→タスクで E2E ファイルのコンパイル確認（`cargo test -p shiori-host32-host --no-run`）を明記する。
- 台帳の報告の作り直し忘れ——常設検査が赤にする（`DomainReportStale`）。

### 9.6 参照
- ukadoc: `spec_shiori3.html#Charset:1`／`#Charset:2`・`descript_ghost.html#shiori.encoding_…`／`#shiori.forceencoding_…`・`descript_shell_surfaces.html#charset_…`（要件の引用どおり）。
- `encoding_rs` 0.8.35 `src/lib.rs`: `for_label`・`name`・`encode`・`decode_without_bom_handling`・`output_encoding`。
- steering: `logging.md`（レベルと構造化フィールド）・`structure.md`（兄弟テストファイル・`#[path]` 接続宣言）・`tech.md`（`encoding_rs` 承認済み）。
- `doc/ukadoc-coverage/README.md` §3（証拠の書き方）。
