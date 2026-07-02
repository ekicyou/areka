# ギャップ分析（areka-P0-package-mount）

> 生成: 2026-07-02 / フェーズ: requirements-generated / 言語: ja
> スキル: kiro-validate-gap（`rules/gap-analysis.md` フレームワーク準拠）
> 対象: 確定済み requirements.md（Req 1–5）と既存コードベースの実装ギャップ

## 分析サマリ（要約）

- **接ぎ木先は確立済み**: `areka-parsers` クレートに `sakura` の module パターン（`mod.rs` で `pub use` 集約・`model ← lexer ← decode ← parse` の一方向依存・`#[non_exhaustive]` enum・最小 derive）が確立し、依存する `areka-P0-parser-foundation`（`charset::decode` / `kv::parse_kv`）も完成済み。本 spec の `package` module はこの 2 API を直接消費すれば descript.txt 読み込みを再実装せず済む（Req 1.4 充足の道が既にある）。
- **不足能力は「ツリー解決 + マウントモデル型」の 1 点に集約**: charset/KV は再利用で埋まる。真に新規なのは (a) マウントモデル型の定義（現状 未存在＝本 spec が正本所有）、(b) ディレクトリツリーの物理存在確認（`std::fs` によるファイル/ディレクトリ存在チェック）、(c) `type,ghost` 受理ガード・`seriko.defaultsurfacedirectoryname` 既定 `master` 解決の 3 ロジック。
- **エラー方針が唯一の実質的設計判断**: `sakura` は `Result` 無しの寛容パースだが、mount は「不在ファイル/不在ディレクトリ」という現実の失敗を持ち得る（Req 1.5 / 2.3 / 3.3 / 5.1）。foundation の 2 API はどちらも I/O を持たず純粋（`&[u8]`/`&str` 入力）だが、`package` は初めて `std::fs`（ファイル読取・ディレクトリ存在確認）を持ち込む module になる。これは design で決すべき本質的判断項目。
- **ukadoc で既定値を正典確認済み**: `seriko.defaultsurfacedirectoryname` の既定は `master`（=`shell/master`）で確定。`shiori` の canonical 既定は `shiori.dll`（emo2 は `pasta.dll` で上書き）。加えて ukadoc は **`shell/master` フォルダを伺か仕様上「必須」** と明記（無いと互換性問題）＝ Req 3.3 の「shell マウント先不在＝失敗」を正典が裏打ちする。
- **過剰実装リスクが高い領域**: emo2 の `ghost/master/descript.txt` は `charset,type,id,name,sakura.name,kero.name,shiori,craftman*,homeurl` のみを持ち、`seriko.defaultsurfacedirectoryname` すら**書いていない**（＝既定 `master` に落ちる経路が emo2 の主経路）。Req 5.2「emo2 使用フィールドのみ」を厳守すると、解決対象キーは `type / name / sakura.name / kero.name / shiori / seriko.defaultsurfacedirectoryname`（最後は既定フォールバック確認用）に限定される。

---

## 1. 現状調査（Current State）

### 1.1 クレート構造とパターン

`crates/areka-parsers/src/lib.rs`:
```
pub mod charset;   // foundation（完了）
pub mod kv;        // foundation（完了）
pub mod sakura;    // 既存の兄弟パーサ
```
lib.rs の doc コメントが「兄弟モジュール（shell / balloon / **package** 等）は各 spec が追加する」と明記済み。本 spec は `pub mod package;` の 1 行追加が接ぎ木点。

### 1.2 確立済み module パターン（`sakura` を範とする）

`crates/areka-parsers/src/sakura/mod.rs` が示す規約:
- `mod.rs` は内部 submodule を `mod` 宣言し、公開面（型・facade 関数）を `pub use` で集約。
- 依存方向は一方向: `model ← lexer ← decode ← parse`。
- テストは submodule ごとに `#[cfg(test)] mod xxx_tests;`＋クレート横断の `validation_tests`。
- モデル型は `#[non_exhaustive]`＋最小 derive（`sakura::Instruction` は `Clone, Debug, PartialEq` のみ、`f32`/`Duration` を含むため `Eq`/`Hash`/`serde` を付さない）。不透明 NewType はフィールド非公開＋read-only アクセサ。
- **`sakura::parse` は `Result` 無し**（`pub fn parse(input: &str) -> Vec<Instruction>`）。寛容パス（`Instruction::Raw`）で不正を握り潰す設計。

`charset/mod.rs` / `kv/mod.rs` も同一の「skeleton → 段階実装 → `pub use` 集約」規約。

### 1.3 依存する foundation の実 API（消費対象）

| API | シグネチャ | 性質 |
|---|---|---|
| `areka_parsers::charset::decode` | `decode(bytes: &[u8], default: DefaultEncoding) -> String` | I/O 無し・純粋・`Result`/panic 無し・BOM 吸収・不正並びは U+FFFD 置換 |
| `areka_parsers::charset::DefaultEncoding` | `enum { Ansi, Utf8 }`（`#[non_exhaustive]`） | `Ansi→SHIFT_JIS` / `Utf8→UTF_8` 固定写像 |
| `areka_parsers::kv::parse_kv` | `parse_kv(text: &str) -> BTreeMap<String, String>` | I/O 無し・純粋・後勝ち・trim・空行/カンマ無し行スキップ・値は文字列保持 |

**重要な含意**: 両 API とも入力は**メモリ上のバイト列/文字列**であり、**ファイル読取そのものは foundation の外**。したがって `package` は「ファイルを読む」責務（`std::fs::read`）を自前で持つ最初の module になる。emo2 の descript.txt は `charset,UTF-8` 宣言ありゆえ、`decode(bytes, DefaultEncoding::Utf8)` で読み → `parse_kv` で `BTreeMap` 化 → キー参照、が素直な合成。

### 1.4 emo2 fixture 実レイアウト（検証対象）

`crates/pilot/examples/shiori-host-32/fixtures/emo2/`:
- 起点: `ghost/master/descript.txt` — 実内容:
  ```
  charset,UTF-8 / type,ghost / id.emo2 / name,えも？？ / sakura.name,むらさき /
  kero.name,エモ / shiori,pasta.dll / craftman,ekicyou / ... / homeurl,...
  ```
  - `id.emo2` は**カンマ無し行**（`.` 区切り）＝ `parse_kv` が自動スキップ（Req 5.2 の未使用フィールド無視と整合）。
  - **`seriko.defaultsurfacedirectoryname` は不在**＝ emo2 の shell 解決は既定 `master` 経路に落ちる（Req 3.1 の主検証パス）。
  - SHIORI DLL 実体: `ghost/master/pasta.dll` が存在（`shiori,pasta.dll` の指す先が実在）。
- shell: `shell/master/`（`descript.txt`（`type,shell`）・`surfaces.txt` 相当の `*.txt`・`purple/`・`CityPop/` 画像群）が実在。
- **ノイズ**: ルート `install.txt`・`emo2-kakukaku/`（別ゴースト＝バルーン？）・`delete.txt`・`updates.txt`・`ghost/master/dic/`・`ghost/master/scripts/` は本 spec のスコープ外（Req 5.3 / Boundary Out）。

### 1.5 ukadoc 正典確認（emo2 は最小サンプルにすぎない）

| キー | ukadoc 既定 | 出典 |
|---|---|---|
| `seriko.defaultsurfacedirectoryname` | **`master`** | `descript_ghost` — 「初回起動時…標準で読み込まれるシェルのディレクトリ名」既定 `master` |
| `shiori` | `shiori.dll`（emo2 は `pasta.dll` で上書き） | `descript_ghost` — 「そのゴーストが使用する SHIORI のファイル名」 |
| `shell/master` の存在 | **仕様上必須** | `dev_shell_error` — 「伺かの仕様上、shell/master フォルダは必ず必要で、無いと互換性の問題が発生」。回避には `seriko.defaultsurfacedirectoryname` で別ディレクトリ指定が必要 |

→ Req 3.3（shell マウント先不在＝観測可能な失敗）は ukadoc が裏打ち。ただし「`shiori` 未指定時に既定 `shiori.dll` を推測するか」は Req 2.3 が「推測しない（欠落を観測可能に表現）」と**明示的に上書き**している点に留意（下記 設計判断②）。

---

## 2. 要件→資産マッピング（Requirement-to-Asset Map）

| 要件 | 必要な技術要素 | 既存資産 | ギャップ種別 |
|---|---|---|---|
| R1.1 起点 `ghost/master/descript.txt` 読取 | ファイル読取＋charset decode | `charset::decode`（decode のみ・**read は無**） | Missing（`std::fs::read` は自前） |
| R1.2 `type,ghost` 受理 | KV 参照＋等値ガード | `kv::parse_kv`（map 化まで） | Missing（ガードロジック新規・小） |
| R1.3 `name`/`sakura.name`/`kero.name` 取得 | map lookup → モデル格納 | `kv::parse_kv` | Missing（モデル型＋詰め替え） |
| R1.4 charset/KV を foundation へ委譲 | 委譲呼び出し | `charset` + `kv`（そのまま利用可） | **充足済み（再利用）** |
| R1.5 起点不在＝観測可能失敗 | ファイル存在/読取失敗の表現 | 既存に前例なし（sakura は Result 無し） | **Constraint（エラー方針＝設計判断①）** |
| R2.1 SHIORI dir=`ghost/master` | パス合成（定数由来） | `std::path::Path` | Missing（自明・小） |
| R2.2 `shiori,<file>` 取得 | map lookup | `kv::parse_kv` | Missing（詰め替え） |
| R2.3 `shiori` 無指定＝欠落を観測可能に（推測禁止） | 欠落表現（`shiori.dll` へ推測しない） | 既存前例なし | Constraint（設計判断②：欠落の型表現） |
| R3.1 `seriko.default...` 無指定＝`shell/master` | 既定フォールバック | ukadoc 既定 `master` 確定済 | Missing（既定定数） |
| R3.2 指定あり＝`shell/<name>` | パス合成 | `std::path::Path` | Missing（自明・小） |
| R3.3 shell dir 不在＝観測可能失敗 | ディレクトリ存在確認 | `std::fs`（前例なし） | Constraint（設計判断①と同根） |
| R4.1 マウントモデルを単一値で返却 | モデル型定義 | **未存在**（本 spec が正本所有） | **Missing（新規型・中核）** |
| R4.2 UI/COM/host 非依存 | 純粋＋`std::fs` のみ | クレート方針と整合 | 充足（設計制約） |
| R4.3 emo2 レイアウトを解決してテスト通過 | fixture 参照テスト | emo2 実物・sakura のテスト配置流儀 | Missing（テスト新規） |
| R5.1 欠落＝明示的失敗（sakura と対照） | エラー方針 | — | Constraint（設計判断①） |
| R5.2 未使用フィールド無視 | KV は分類しないので自然に無視 | `kv::parse_kv`（分類しない） | **充足済み** |
| R5.3 `install.txt`/NAR/balloon を読まない | 単に触らない | — | 充足（スコープ規律） |

---

## 3. 実装アプローチ選択肢

### Option A: `areka-parsers` に `package` module を追加（`sakura` パターン踏襲）— 【推奨】
**適合理由**: requirements/brief がこの配置を明示。foundation と同一クレート内で `charset`/`kv` を直接 `use` でき、`sakura` の module 規約がそのまま雛形になる。

- 追加物: `lib.rs` に `pub mod package;`。内部は `sakura` 流儀で
  - `model`（マウントモデル型＝正本所有・`#[non_exhaustive]`＋最小 derive）
  - `resolve`（ツリー解決＝`std::fs` 存在確認＋パス合成＋既定フォールバック）
  - `parse`/facade（公開 `pub fn ...`）
  の一方向依存（`model ← resolve ← facade`）。過剰分割は回避（`sakura` の 4 分割は必要だったが package は責務が小さいため 2–3 submodule で十分の可能性）。
- **トレードオフ**: ✅ 既存パターン最大活用・foundation 直接再利用・独立単体テスト容易 / ❌ `areka-parsers` が初めて `std::fs`（I/O）を持つ module を抱える＝「純粋パーサ」クレート性質からの逸脱をどう整理するか（設計判断③）。

### Option B: I/O 境界を facade 引数に切り出し（純度維持ハイブリッド）
**適合理由**: 「クレートは純粋関数群」という lib.rs の宣言（"外部状態や host 環境に依存しない純粋関数群"）を守りたい場合。

- ファイル読取・存在確認を呼び出し側（`ghost-setup`）が行い、`package` は「読み込み済みバイト列＋存在確認済みフラグ」を受け取って解決するか、`Fs` トレイト/クロージャを注入する。
- **トレードオフ**: ✅ クレート純度維持・テストで実 fixture 不要（in-memory 可）/ ❌ requirements（R1.1「起点を読み込む」・R4.3「emo2 実 fixture ツリーを解決」）は loader が**ツリーを走査する**ことを含意し、抽象化が過剰実装（Req 5.2 禁止）になりうる。emo2 fixture テストの旨みが薄れる。

### Option C: 薄い `Result` を返す純関数＋最小 `std::fs`（A の具体形）
A の内側の設計判断を具体化した中庸案。`package::resolve(ghost_root: &Path) -> Result<MountModel, MountError>` とし、`std::fs` は resolve 内に閉じる。charset/KV は foundation へ委譲。emo2 実 fixture で success/欠落系をテスト。
- **トレードオフ**: ✅ requirements の 5 要件（特に R1.5/R2.3/R3.3/R5.1 の観測可能失敗）へ最短 / ❌ `sakura` の Result 無し流儀と非対称になる（が requirements 自身が「sakura とは異なる」と明記＝許容される非対称）。

**推奨**: Option A の器に Option C の中身（`Result<MountModel, MountError>`＋`std::fs` を resolve に閉じ込め）。設計フェーズで下記判断を確定する。

---

## 4. 設計判断項目（要件ディスカッションへ送る）

1. **エラー方針: `Result` か「欠落を保持するモデル」か。**
   `sakura` は `Result` 無し寛容パースだが、mount は不在ファイル/ディレクトリという現実の失敗を持つ（R1.5/R2.3/R3.3/R5.1）。候補: (a) `resolve(...) -> Result<MountModel, MountError>`（失敗＝早期 return、最も素直）/ (b) `MountModel` 内に `shiori: Option<String>` 等の「欠落を型で表現」＋致命（起点不在・shell dir 不在）のみ `Result`（ハイブリッド）/ (c) 全欠落を enum で保持し `Result` を使わない。R2.3「`shiori` 未指定は欠落を観測可能に（推測禁止）」は "致命ではない欠落" の余地を示唆＝(b) が要件と最も整合的か。**要判断: どの欠落が致命（`Err`）でどれが非致命（`Option`/欠落マーカー）か。**

2. **`shiori` 未指定時の扱い（推測禁止の徹底）。**
   ukadoc の canonical 既定は `shiori.dll` だが、R2.3 は「推測しない・欠落を観測可能に」と**既定推測を禁止**している。判断: 既定 `shiori.dll` を**採用しない**方針（要件優先）を design に明記し、`Option<String>`（None＝未指定）で表すか、非致命の警告付き欠落とするか。

3. **クレート純度と `std::fs` の持ち込み。**
   `areka-parsers` の lib.rs doc は「純粋関数群」と宣言。`package` は初めて I/O（`std::fs::read`・ディレクトリ存在確認）を持つ。判断: (a) I/O を `resolve` 内に閉じ込め doc を「（package のみ）ローカルツリー走査を許容」と補記 / (b) Option B のトレイト注入で純度を厳守。R4.2「ローカルディレクトリツリーとその中のテキストファイルのみを入力」は I/O 許容を示唆＝(a) が要件寄り。

4. **マウントモデル型の形状（正本所有）。**
   本 spec が型の正本。含めるフィールド候補: ゴースト識別（`type` 受理は bool ガードで足り、格納不要か）・`name`/`sakura.name`/`kero.name`・SHIORI マウント先（dir=`ghost/master` の `PathBuf` ＋ file 名）・shell マウント先（`shell/<dir>` の `PathBuf`）。判断: パスは `PathBuf` か相対 `String` か（下流 `host-32` は CP_ACP 世界＝brief 注記。ただし本 spec は「パス文字列の解決まで」）。`#[non_exhaustive]`＋最小 derive を踏襲するか。

5. **submodule 分割粒度（過剰実装回避）。**
   `sakura` は `model/lexer/decode/parse` の 4 分割だが package の責務は小さい。判断: `model`＋`resolve`（＋公開 facade は `resolve` に同居 or `mod.rs` の `pub use` のみ）で足りるか、それとも parse（KV→フィールド抽出）と resolve（ツリー解決）を分けるか。過剰分割は `kv` mod.rs の「単一責務ゆえ内部 1 本」の判断を範とする。

6. **`type,ghost` 受理失敗の扱い。** → **【要件ディスカッション #1 で解決済（2026-07-02・SSP 準拠）】**
   ~~R1.2 は「`type,ghost` を含むとき受理」。含まない/別 type のときの挙動が未明示。~~ ukadoc `type,種別`（ghost）＝「省略不可／**SSP では ghost/master にある descript.txt なら ghost と識別される**」＋`manual_ghost`「これがないと本体に認識されない」より、**SSP は所在ベース識別**と確定。要件を改訂: 識別は `ghost/master/descript.txt` の所在で行い（R1.2）、`type,ghost` は確認的で欠落は失敗としない（R1.3）、失敗は descript.txt 不在のみ（R1.6）。過剰な type-mismatch 分岐は作らない（Req 5.2 過剰実装禁止＋SSP 所在識別）。設計判断としては消滅（残る fatal/非致命の型表現は設計判断①に包含）。

---

## 5. 工数・リスク

- **工数: S（1–3 日）**。新規ロジックはツリー解決＋モデル型＋既定フォールバック＋失敗表現に限定。charset/KV は完成済み foundation の再利用で、パターン（sakura）も確立済み。emo2 実 fixture が既に存在しテスト材料が揃う。
- **リスク: Low**。既知技術（`std::fs`・`BTreeMap` lookup）・明確なスコープ・正典（ukadoc）で既定値確定済み・過剰実装禁止が明文化。唯一の非自明点はエラー方針（設計判断①②）だが requirements が方向性（sakura と異なる明示的失敗）を既に与えている。

## 6. 設計フェーズへの申し送り（Research Needed / 推奨）

- **推奨アプローチ**: Option A（`package` module 追加）＋ Option C の中身（`resolve(&Path) -> Result<MountModel, MountError>`・`std::fs` を resolve に閉じ込め・charset/KV は foundation 委譲）。
- **Research Needed（design で確定）**:
  - 致命/非致命の欠落境界（設計判断①）と `MountError` の variant 設計。
  - `shiori` 未指定の型表現（`Option` vs 欠落マーカー）＝ R2.3 推測禁止との整合（設計判断②）。
  - マウントモデルのパス表現（`PathBuf` 絶対 or 相対、下流 host-32 の CP_ACP 前提との受け渡し境界）（設計判断④）。
  - submodule 分割粒度の最終決定（設計判断⑤）。
- **正典参照済み（追加調査不要）**: `seriko.defaultsurfacedirectoryname` 既定=`master`、`shell/master` 必須、`shiori` canonical 既定=`shiori.dll`（ただし R2.3 で推測禁止）。
