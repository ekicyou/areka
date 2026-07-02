# Gap Analysis: areka-P0-parser-foundation

> 目的: 確定済み requirements.md と既存コードベースの差分を分析し、設計フェーズの判断材料を提供する。**決定ではなく情報と選択肢**を提示する（正典 ukadoc・emo2 fixture は最小適合サンプル）。
> 分析日: 2026-07-02 / 対象言語: ja / phase: requirements-generated

---

## 1. 現状調査（Current State Investigation）

### 1.1 統合先クレート `crates/areka-parsers`

- **クレート性格**: `Cargo.toml` の description が "Pure, std-only parser family for areka (sakura script and siblings); host-independent"。依存は `tracing`（workspace）**のみ**。`publish = false`・edition 2024・workspace メンバ。
- **モジュール構成**（`src/lib.rs` → `pub mod sakura;` のみ公開。兄弟モジュールは各 spec が追加すると明記）:
  - `src/lib.rs` — クレート doc＋公開面集約。「兄弟モジュール（shell / balloon / package 等）は各 spec が追加する」と既に明記済み。
  - `src/sakura/mod.rs` — 公開面集約。内部モジュールは `mod`（非公開）、`#[cfg(test)] mod *_tests;`、末尾に `pub use` で型と関数を公開。
  - `src/sakura/{model,lexer,decode,parse}.rs` ＋ 各 `*_tests.rs` ＋ `validation_tests.rs`。
- **依存方向**: `model ← lexer ← decode ← parse`。`parse.rs` が唯一の公開 facade（`pub fn parse(input: &str) -> Vec<Instruction>`）。

### 1.2 既存 `sakura` モジュールの規律（本フィーチャが踏襲すべき discipline）

`requirements.md`（隣接期待）と `brief.md`（層構造）が指す「sakura 規律」は、実コードで以下のように具現化されている:

| 規律項目 | 実コードでの現れ | 出典 |
|---|---|---|
| `Result` を返さない寛容処理 | `parse` / `decode` とも常に `Vec<_>` を返す。`Result`・`?`・`panic!` 皆無 | `parse.rs:28`, `decode.rs:53` |
| panic しない | 引数欠落・非数は `unwrap_or`/`unwrap_or_default` で吸収（例 `wait_from_arg` は `parse::<u64>().ok().unwrap_or(0)`） | `decode.rs:227-236` |
| `tracing` のみ（他の副作用なし） | 現状 `decode`/`parse` は純粋関数でログ呼び出しすら無い。副作用は tracing に限定する方針 | `Cargo.toml:12-13` |
| in-source テスト | `#[cfg(test)] mod *_tests;` を `mod.rs` に列挙。テストは同一クレート内 `src/sakura/*_tests.rs` | `mod.rs:14-33` |
| 公開パス経由の契約固定 | `validation_tests.rs` が `super::parse::parse` を通した end-to-end で契約を固定（内部関数を直叩きしない層を用意） | `validation_tests.rs:1-20` |
| 不透明 NewType＋read-only アクセサ | `SurfaceArg(String)`・`NewLineRatio(f32)` はフィールド非公開、`new()`＋`as_str()`/`ratio()` のみ公開（dola `ActorKey` 流儀と明記） | `model.rs:56-86` |
| `#[non_exhaustive]`・最小派生 | `Instruction` は `#[non_exhaustive]`＋`derive(Clone, Debug, PartialEq)` のみ（`f32`/`Duration` を含むため `Eq`/`Hash`/`serde` 無し） | `model.rs:23-24` |
| テスト期待値の直書き＋出典明示 | テストは実スクリプト断片をリテラルで直書きし doc コメントで要件番号を紐付ける（`include_str!` を使わない） | `decode_tests.rs`, `validation_tests.rs` |

### 1.3 命名衝突リスク（重要な観測）

- **既存 `src/sakura/decode.rs` は「構文トークン → Instruction」の意味デコーダ**であり、本フィーチャが要求する `decode`（**charset バイト列デコード**）とは **全く別概念で同名**。両者は別モジュールツリー（`sakura::decode` vs 新規トップレベル `decode`）に置けば衝突しないが、可読性・混同リスクの design 判断が必要（後述 決定項目 D2）。

### 1.4 fixture 実地確認（emo2）

- 所在: `crates/pilot/examples/shiori-host-32/fixtures/emo2/`。requirements/brief が参照する `emo2-kakukaku/{descript.txt, balloons0s.txt, balloonk0s.txt}`・`shell/master/descript.txt`・`ghost/master/descript.txt` が実在。
- **実測**:
  - `descript.txt` 系（balloon/shell/ghost）は先頭行 `charset,UTF-8`、**UTF-8・CRLF**。
  - per-surface テーブル `balloons0s.txt` / `balloonk0s.txt` は **charset 行なし・ASCII・CRLF**（`windowposition.x,266` 等の KV 行のみ）。→ **charset 宣言が無いファイルが実在する**＝Requirement 2.3（宣言なし→UTF-8 既定）の生きた検証対象。
  - **Shift_JIS 実ファイルは fixture に存在しない** → Requirement 2.2/7.2 の非 UTF-8 検証は「合成入力」で行う要件どおり（実 fixture では取れない）。
- **クレート跨ぎ問題**: fixture は `crates/pilot` 配下。本フィーチャのテストは `crates/areka-parsers` 内。Requirement 7.3 が「`include_str!` のクレート跨ぎ依存を避け、期待値はリテラル直書き＋出典（正本ファイル名・行）明示」と定めるのは、この配置差を踏まえた妥当な制約（既存 sakura テストと同一流儀）。

### 1.5 外部依存 `encoding_rs`（開発者承認済み・唯一の追加依存）

- API（Encoding Standard 準拠・Gecko 由来）:
  - `Encoding::for_label(label: &[u8]) -> Option<&'static Encoding>` — ラベル文字列（例 `b"Shift_JIS"`）から Encoding を引く。**未対応/不正ラベルは `None`** → Requirement 2.4 の「UTF-8 へ寛容フォールバック」に直結。
  - `encoding.decode(bytes: &[u8]) -> (Cow<str>, &'static Encoding, bool)` — 非ストリーミング。第3要素 `had_errors`（不正並びを U+FFFD 等で置換したか）。**`Result` を返さず・panic しない**＝ Requirement 2.5/2.6/6.1 に構造的に合致。BOM があれば `decode` が sniff して対応（Requirement 5.2）。
  - 定数 `encoding_rs::UTF_8` / `encoding_rs::SHIFT_JIS` が `&'static Encoding` として存在。
- **バージョン**: workspace に未登録。crates.io 最新系（0.8 系）を `crates/areka-parsers/Cargo.toml` に追加、または `Cargo.toml [workspace.dependencies]` へ集約するかは design 判断（決定項目 D5）。
- **std-only 原則からの意図的逸脱**: クレート description の "std-only" は `encoding_rs` 追加で破れるが、これは開発者承認済みの意図的逸脱。description 文言を更新するか否かは軽微な design 判断。

### 1.6 ukadoc 正典確認（charset）

- `descript_balloon`/`descript_ghost`（`headline`/`install` も同文言）: 「表示する文字コード。旧い環境との互換性を考慮する場合は Shift_JIS、それ以外は UTF-8 を推奨。（省略時は OS 標準設定または SSP 国際化設定）」。
- SHIORI3 protocol `Charset`: 「文字コード。最初の行、または少なくとも文字コードが ASCII 範囲以外の行の前が望ましい」→ **冒頭 ASCII プリスキャン方式が正当**（charset 名は ASCII、非 ASCII 行より前に置かれる想定）。
- **追加観測**: ukadoc には `readme.charset,文字コード` という **別キー**（説明テキストファイルの文字コード・2.5.10）が存在。本フィーチャの `decode` は「渡されたバイト列の冒頭 `charset` 行」を見るだけで、`readme.charset` のような**別ファイルを指すキー**は解釈しない（=各 spec 固有層／out of scope）。要件境界と矛盾しないが、design で「`decode` が探すのは自ファイルの `charset` 行のみ」と明記しておくと混同を防げる。

---

## 2. 要件実現性分析（Requirement-to-Asset Map）

| 要件 | 技術的必要物 | 既存資産 | ギャップ種別 |
|---|---|---|---|
| R1 charset 宣言検出（冒頭 ASCII プリスキャン・寛容抽出） | バイト列冒頭の走査＋`charset,<name>` 抽出（空白/大小/CRLF 寛容） | 無し（新規） | **Missing**（実装容易・純粋文字列処理） |
| R2 宣言エンコードで全体デコード＋未対応/不正フォールバック | `encoding_rs` の `for_label`＋`decode` | `encoding_rs`（承認済・未導入） | **Missing**（依存導入＋薄いラッパ） |
| R3 純粋性（I/O なし・決定的・副作用なし） | `&[u8] -> String` の純粋関数 | 既存 sakura が同型の純粋規律を確立済み | **Constraint（追従）** |
| R4 KV マップ化（後勝ち・分類なし・型付けなし・順序非保持） | 行 `split_once(',')` → `HashMap<String,String>` 相当 | 無し（新規）。std のみで可 | **Missing**（実装容易） |
| R5 行区切り/BOM 寛容 | CRLF/LF 両対応の行分割・BOM 吸収 | 無し。BOM は `encoding_rs::decode` が処理 | **Missing/Constraint** |
| R6 寛容規律（no panic / no Result / tracing 可） | 既存 sakura と同一規律 | sakura 実装が手本 | **Constraint（追従）** |
| R7 単体テスト（UTF-8 fixture＋SJIS 合成＋公開パス契約） | in-source テスト・リテラル直書き・公開 API 経由 | sakura の `*_tests.rs`/`validation_tests.rs` が手本 | **Constraint（追従）** |

### 複雑性シグナル
- 大半が **アルゴリズム的純粋処理**（外部連携は `encoding_rs` 1 本のみ・DB/UI/認証なし）。既存クレートに確立済みの規律へ素直に乗る。新規性は低い。

---

## 3. 実装アプローチ選択肢

### Option A: 既存 `sakura` モジュール流儀を踏襲した**新規トップレベル 2 モジュール**（`areka_parsers::decode` / `areka_parsers::kv`）
- **配置**: `src/decode/`（または `src/decode.rs`）＋`src/kv.rs`（or `src/kv/`）を `lib.rs` に `pub mod` 追加。sakura と同じく内部は非公開、公開面は薄い `pub fn`／型を集約。
- **公開契約候補**: `pub fn decode(bytes: &[u8]) -> String`（charset プリスキャン＋全体デコード合成）／`pub fn parse_kv(text: &str) -> BTreeMap<String,String>`（名称は design 判断）。
- **トレードオフ**: ✅ sakura 規律と一貫（学習コスト最小・レビュー容易）。✅ 後続 balloon/shell/package から `use areka_parsers::{decode, ...}` で自然に依存。✅ 各モジュール単一責務。❌ `decode` の**名前が `sakura::decode` と衝突的**（別ツリーゆえ技術的衝突はないが概念混同のリスク）。

### Option B: `sakura` と並ぶ**共通基盤 namespace（例 `areka_parsers::foundation` / `common`）配下に `decode`/`kv`**
- **配置**: `src/foundation/mod.rs` → `pub mod decode; pub mod kv;`。公開は `areka_parsers::foundation::decode` 等。
- **トレードオフ**: ✅ 「共通基盤」という feature の意図が構造に表れる。✅ `sakura::decode` との名前混同を namespace が緩和。❌ 後続 spec の import パスが 1 段深くなる。❌ 「foundation」という追加の命名概念を導入（過剰な階層化の懸念）。

### Option C: ハイブリッド — **charset デコードだけ意図の伝わる名前**（例 `charset` モジュール）＋ `kv` はトップレベル
- **配置**: `src/charset.rs`（`pub fn decode(bytes) -> String` など）＋`src/kv.rs`。requirements の module 名 "Decode module" は保ちつつ、モジュール名で用途を明示し `sakura::decode` との混同を回避。
- **トレードオフ**: ✅ 命名衝突を最も明快に解消（`charset::decode` は自明）。✅ 階層を増やさない。❌ requirements は「`decode`」という module 名を使っている（AC 群が "the Decode module" と表記）ため、命名を `charset` にすると requirements 文言との一貫性の説明が要る（design で「Decode module ＝ `charset` モジュール」と対応付ければ足りる。requirements は編集不可）。

> **注**: requirements.md は「`decode`／`kv` の 2 モジュール」「the Decode module / the KV module」という抽象を使っており、物理モジュール名を厳密指定してはいない。したがって A/B/C いずれも requirements 適合可能。命名は純粋な design 判断。

---

## 4. Effort / Risk

| 項目 | 評価 | 根拠（1 行） |
|---|---|---|
| Effort | **S（1–3 日）** | 純粋関数 2 本＋テスト。既存 sakura 規律に素直に乗り、外部連携は `encoding_rs` 1 本のみ。 |
| Risk | **Low** | 未知技術なし・アーキ変更なし・スコープ明確。唯一の変数は `encoding_rs` のラベル解釈と SJIS 合成テストの期待値作成（いずれも既知解）。 |

---

## 5. Research Needed（設計フェーズへ持ち越す確認項目）

1. **冒頭プリスキャン範囲**: 「冒頭部」を先頭 N バイト固定にするか、最初の非 ASCII 行 or 最初の空行までにするか。SHIORI3 の「charset は非 ASCII 行より前」を根拠に上限バイト数を design で確定（過大スキャン防止）。
2. **`charset` 行の書式寛容度の厳密境界**: 大小無視・前後空白 trim・`charset` と値の区切り（`,`）以外に `charset:` 等の異体を許容するか（ukadoc は `charset,` のみ）。R1.3 の「寛容」の具体範囲。
3. **SJIS 合成テストの正本化**: fixture に SJIS 実ファイルが無い（R7.2）。合成入力のバイト列と期待文字列をどう作り、どう「採取元」を記録するか（`encoding_rs::SHIFT_JIS.encode` でラウンドトリップ生成 vs 手打ちバイト literal）。
4. **KV マップ型の選択**: `HashMap`（R4.8 順序非保持と整合）か `BTreeMap`（決定的テスト比較が容易・順序は "保持しない" 要件に反しない）か。テスト決定性の観点も含めて design 判断。
5. **`encoding_rs` の依存宣言場所**: `crates/areka-parsers/Cargo.toml` 直書き vs `[workspace.dependencies]` 集約。バージョン固定方針（他 workspace 依存はほぼバージョン明記）。
6. **既定エンコード指定 API と ANSI の具体写像**（要件ディスカッション #1 で確定した方針の design 落とし込み）: `decode` は既定エンコードを引数で受け取る（ANSI／UTF-8 の切り替え・SHIORI/4 は UTF-8 固定）。(a) 引数を `{Ansi, Utf8}` の 2 値 enum とし、`Ansi` を固定コードページ（伺か JP 文脈では CP932=Shift_JIS 相当）へ写像するか、(b) エンジン側が OS ANSI コードページを解決して具体エンコード（`encoding_rs::Encoding` ラベル）を渡すか。**純粋性（R3）維持のため `decode` 自身は OS ロケールを読まない**——(a) は areka 内で決定的だが非 JP ロケール差を捨象、(b) は決定性を保ちつつ ANSI 解決の責務をエンジンへ寄せる。design で確定。

---

## 6. 設計フェーズへの推奨（Recommendations）

- **推奨アプローチ**: 命名衝突（`sakura::decode` との混同）を避けつつ requirements の 2 モジュール抽象を素直に満たす **Option C（`charset` ＋ `kv` のトップレベル）** を第一候補、次点 Option A。Option B は「共通基盤が今後さらに増える」見込みが強い場合のみ。**過剰な階層化は brief の「過剰・予測実装禁止」に反する**点に留意。
- **契約設計の核**: `decode` は `&[u8] -> String`（`Result` 不使用）で `encoding_rs::{Encoding::for_label, Encoding::decode}` を薄く包む。未対応ラベル（`for_label == None`）・宣言なし → `UTF_8`。`had_errors` は破棄せず必要なら tracing。`kv` は `&str -> Map`、`split_once(',')`・trim・後勝ち・空行/カンマ無し行スキップ・値は String 保持。
- **規律の機械的踏襲**: 型は最小派生＋`#[non_exhaustive]`（NewType を導入する場合）、テストは in-source＋公開パス契約（sakura の `validation_tests.rs` 相当を用意）＋リテラル期待値＋出典コメント。
- **持ち越し研究**: §5 の 5 項目（プリスキャン範囲・書式寛容境界・SJIS 合成テスト正本化・KV マップ型・依存宣言場所）を design 冒頭の決定事項として明示。

---

---

## 7. 設計フェーズ結果（Design Synthesis / Decisions — 2026-07-02 追記）

> `/kiro-design` により生成。discovery 種別: **light（Extension）**。既存 `areka-parsers` への 2 モジュール追加であり、コードベース分析＋ ukadoc/encoding_rs 事実は §1 で完了済み。追加の web リサーチ・subagent 分散は不要と判断。design.md へ全決定を反映済み。

### 7.1 §5 持ち越し 6 項目の確定（design.md「決定事項」表と一致）

| ID | 確定 | 一行根拠 |
|----|------|----------|
| D1 プリスキャン範囲 | 先頭〜最初の非 ASCII バイト、上限 4096B・行単位走査 | SHIORI3「charset は非 ASCII 行より前」＋ R1.5（charset 名 ASCII）。上限は過大スキャン防止 |
| D2 書式寛容度 | 区切り `,` のみ・キー大小無視・前後 trim・CRLF/LF 両対応。`charset:` 等異体は不許容 | R1.3＋ ukadoc `charset,` 単一書式。異体許容は過剰実装 |
| D3 SJIS 合成正本化 | `encoding_rs::SHIFT_JIS.encode(<期待文字列リテラル>)` でラウンドトリップ生成・期待文字列直書き＋合成コメント | R7.2/R7.3。fixture に SJIS 実ファイル無し（§1.4） |
| D4 KV マップ型 | `std::collections::BTreeMap<String,String>` | R4.8（順序非保持）両立＋決定的テスト比較（HashMap 非決定順回避） |
| D5 encoding_rs 宣言場所 | ルート `[workspace.dependencies] encoding_rs="0.8"` ＋ crate は `{ workspace = true }` | 既存 workspace 依存はバージョン集約が慣行（`Cargo.toml:15-31`） |
| D6 既定エンコード API＋ANSI 写像 | `decode(bytes: &[u8], default: DefaultEncoding) -> String`。`DefaultEncoding{Ansi,Utf8}`（`#[non_exhaustive]`）。`Ansi`→CP932(`SHIFT_JIS`) 固定写像・`Utf8`→`UTF_8`。SHIORI/4 は `Utf8` | 要件ディスカッション #1（既定は引数指定・SHIORI/4 は UTF-8 固定）＋ R3 純粋性（OS ロケール非参照）。option(a) 採用＝要件語彙 ANSI/UTF-8 と 1:1。将来の OS ロケール厳密解決は variant 追加で後方互換吸収 |

### 7.2 Synthesis 成果（一般化・build-vs-adopt・簡素化）

- **命名一般化（Option C 採用）**: requirements の「Decode module / KV module」抽象を、物理モジュール名 `charset`／`kv` へ落とす。`sakura::decode`（意味デコーダ）との同名混同を `charset` 命名で解消（§1.3・§6 第一候補）。requirements は module 名を厳密指定しておらず適合。
- **build-vs-adopt**: charset デコードは `encoding_rs` を **adopt**（薄いラッパ・for_label/decode が Option/タプルで規律に構造合致）。KV は std のみで **build**（外部依存不要・素朴 split_once）。
- **簡素化（過剰実装回避）**: `kv` は単一責務ゆえ内部 `parse.rs` 1 本（sakura の 4 分割を機械的に真似ない）。`kv` 戻り値は素朴 `BTreeMap` で NewType を導入しない（分類・型付けは各 spec 固有層＝R4.2）。`DefaultEncoding` のみ `#[non_exhaustive]` enum とし、それ以外の型追加はしない。
- **charset 内部分割**: `prescan`（検出）と `decode`（デコード合成）へ 2 分割（sakura の lexer/decode 分割に対応・単体テスト境界を明確化）。

### 7.3 設計レビューゲート結果

- **合格（修正パス 0 回）**。機械チェック（要件 ID 全数 1.1–7.5 が traceability に出現・境界 4 セクション充足・File Structure Plan 具体パス・境界↔ファイル整合・orphan コンポーネントなし）＋判断チェック（契約は具体 Rust シグネチャ＋pre/post/invariant・build-vs-adopt 明記・投機的抽象なし）をすべて通過。要件ギャップ・矛盾は検出されず。

### 7.4 未解決事項

- なし（§5 の 6 項目すべて design で確定・要件との矛盾なし）。

## 参照
- 既存規律実装: `crates/areka-parsers/src/sakura/{mod,model,decode,parse}.rs`・`decode_tests.rs`・`validation_tests.rs`
- fixture: `crates/pilot/examples/shiori-host-32/fixtures/emo2/`（descript 系＝UTF-8/CRLF、balloon{s,k}0s.txt＝charset 無し ASCII）
- ukadoc（正典）: `descript_balloon`/`descript_ghost` の `charset,文字コード`・`spec_shiori3` の `Charset`（冒頭・非 ASCII 行前が望ましい）
- 外部依存: `encoding_rs`（`Encoding::for_label` / `Encoding::decode` は Option/タプルを返し Result・panic なし）
