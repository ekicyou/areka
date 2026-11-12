# ギャップ分析: dola-yaml-backend

> 2026-11-13 `/kiro-validate-gap`（`main` 069f1a48 の上の作業枝 `claude/kiro-start-dola-yaml-backend-cd8f8a` で測った）。要件は `requirements.md`（案 A＝替える／案 B＝外す の両方の項を持つ形）。

## 1. 分析のまとめ

- **今の形は「機能 1 つ・依存 1 つ・テスト 1 本」だけ**。`crates/dola/src` に `serde_yaml` の語も `cfg(feature = "yaml")` も無い。`#[cfg(feature = "yaml")]` は往復のテストの 1 か所だけ。ワークスペースで `dola` の機能 `yaml` を有効にするクレートは 0 本（`Cargo.toml` を全部 grep・`features` の指定 0 件）。道具（`tools/test-all.ps1`・`tools/crates-io.ps1`・CI）にも `--features yaml`／`--all-features` の指定は無い＝機能 `yaml` のテストは一度も回っていない。
- **仮の見逃しが隠しているのはこの 1 件だけ**（実測）: `deny.toml` の `ignore` を空にした写しで `cargo deny --config <写し> check advisories` を回すと、赤は `RUSTSEC-2024-0449`（`unsafe-libyaml`）の 1 件だけ。ほかは `async-executor` の yanked の警告（`yanked = "warn"`＝終了コードに響かない）。⇒ 機能 `yaml` から `unsafe-libyaml` が消えれば、見逃しを消しても `advisories` は緑になる見込み（要件 1.3）。
- **`THIRD-PARTY-NOTICES.md` はどちらの案でも動かない見込み**: 生成の道具は `cargo about generate --workspace about.hbs -o THIRD-PARTY-NOTICES.md`（`tools/test-all.ps1` 76 行目）で、任意の機能を有効にしない。今の謝辞に `serde_yaml`・`unsafe-libyaml`・`ryu` は載っていない（grep 0 件）。`audit-dola`（#440）も `toml` 0.8 を消したのに謝辞は動いていない＝同じ形の先例。動くのは `Cargo.lock` だけ。
- **案 B（外す）は先例どおりの小さな削除**（XS）。案 A（替える）は依存の追加と、`#[serde(untagged)]` の多い `dola` の型での読み方の違い（下の 4.2）の確かめが要る（S・調べ物が残る）。
- crates.io の `dola` は 0.0.1〜0.1.0 の全版で機能 `yaml`（と既定の `json`）を持って公開済み。総ダウンロード 79（2026-11-13 の crates.io の API）。0.0.1→0.0.2 で機能 `runtime` を外した先例もある。

## 2. 今の形（測った事実）

| 場所 | 今の中身 | 案 A | 案 B |
|---|---|---|---|
| `crates/dola/Cargo.toml` 20〜22・30〜31 行目 | `[dependencies.serde_yaml]`（0.9・optional）・`[features] yaml = ["dep:serde_yaml"]` | 依存を差し替え・機能名は保つ | 両方を消す（`[features]` 節ごと消える＝任意の機能 0） |
| `crates/dola/tests/general/integration_test/serialization_round_trip.rs` 1・40〜55 行目 | 先頭の説明「JSON / YAML」・`#[cfg(feature = "yaml")] mod yaml_integration_tests`（`serde_yaml::to_string`／`from_str`） | 呼び先を新しいクレートへ | 40〜55 行目と先頭の「/ YAML」を消す |
| `crates/dola/tests/general/integration_test/mod.rs` 1 行目 | 「JSON/TOML/YAML round-trip」 | 「JSON/YAML」へ | 「JSON」へ（TOML は既に消えている＝今も古い） |
| `crates/dola/tests/compile/boundary_test.rs` 3・259〜263 行目 | 先頭の説明「外部入力（JSON/TOML/YAML）」・注記「流入経路は YAML（.nan/.inf）のみ（機能 `yaml` により既定ビルド外…）」 | 注記を直し、YAML の `.nan`／`.inf` の境界のテストを足す（要件 4.2・4.3） | 注記を要件 4.4 の形へ |
| `crates/dola/README.md` 13・17 行目 | 「YAML support via the `yaml` feature flag」 | 文はほぼそのまま | 「書式の機能を持たない・serde の導出で利用者が選ぶ」へ |
| `deny.toml` 41〜47 行目 | `ignore` に `RUSTSEC-2024-0449` 1 件とコメント | 消す | 消す |
| `Cargo.lock` | `dola` の依存に `serde_yaml`。`serde_yaml` 0.9.34+deprecated → `indexmap`・`itoa`・`ryu`・`serde`・`unsafe-libyaml` | 下の 4.1 | `serde_yaml`・`unsafe-libyaml`・`ryu` の 3 項目が消える（`ryu` を引くのは `serde_yaml` だけ。`indexmap` は bevy・rmcp・toml、`itoa` は serde_json・http・hyper も引く＝残る） |
| `THIRD-PARTY-NOTICES.md` | YAML の系統は載っていない | 変わらない見込み（作り直して確かめる） | 変わらない見込み |
| `.kiro/steering/tech.md` 52 行目 | `serde_yaml` の行（「`yaml` の行き先は `dola-yaml-backend`」） | 新しい依存の行へ | 行を消す |
| `.kiro/steering/structure.md` 262 行目 | dola の Dependencies「`serde` + feature flags (`json`, `toml`, `yaml`)」 | `json`・`toml` を消す | 機能の記述を消す |
| `.kiro/steering/product.md` 38 行目 | 「宣言的アニメーション定義フォーマット（JSON/TOML/YAML）」 | 書式の列挙を直す | 書式の列挙を外す |
| `.kiro/steering/roadmap.md` 207 行目 | 「`v0.1.0` の後・`audit-dola`」の項目（末尾「機能 `yaml` の行き先は `dola-yaml-backend` が決める」） | 書き直して 1 項目に | 書き直して 1 項目に |

範囲の外として残す語（要件の Out of scope）: `crates/dola/src/compile/mod.rs` 72 行目・`crates/dola/src/value.rs` 7 行目の「JSON/TOML/YAML」。ただし `value.rs` 8 行目「TOML の整数/浮動小数点区別を保持」も利用者側の書式の例として残る。

### 2.1 先例（`audit-dola` #440 の削除の形）

- `crates/dola/Cargo.toml`: 任意の `serde_json`・`toml` と機能 `default`・`json`・`toml` を消し、`serde_json` を `[dev-dependencies]` へ。README の 13・17 行目を書き換え。
- `Cargo.lock`: `toml` 0.8.23 とその系統（`serde_spanned` 0.6・`toml_datetime` 0.6・`toml_edit` 0.22・`winnow` 0.7 ほか）が消え、残った 1 版の名前から版の添え字が取れた（`"toml 1.1.6+spec-1.1.0"` → `"toml"`）。
- `THIRD-PARTY-NOTICES.md`・`deny.toml`: 変更なし。
- roadmap の申し送りは完了の時点で足し忘れ、棚卸㊱が後から足した（要件 6 が拾う）。

## 3. 要件ごとの対応表

| 要件 | 今ある物 | 足りない物／印 |
|---|---|---|
| 1.1 依存の木に `unsafe-libyaml`・`serde_yaml` が無い | — | **Missing**（どちらの案でも `Cargo.toml` の直しで消える） |
| 1.2〜1.4 見逃しの削除・緑・他の行を変えない | `deny.toml` の 1 件 | 見逃しの 1 件だけが赤の原因と実測済み（1.3 は案 B なら確度が高い。案 A は新しい系統の勧告の有無で決まる＝**Unknown**、設計で `cargo deny` を回して確かめる） |
| 1.5 別の勧告で赤なら報告 | — | 今は該当なし（yanked の `async-executor` は警告だけ）。**Constraint**: 着地までに RustSec の取り込みで新しい勧告が届くことはありうる（2026-10-15 の赤と同じ経路） |
| 2.2〜2.5 案 A | — | **Missing**: 依存の差し替え・往復のテスト・完了の証跡で `--features yaml` を 1 回回す道（全体テストは任意の機能を回さない） |
| 2.6・2.7 案 B | 先例（#440） | **Missing**: 削除だけ |
| 3.1〜3.3 JSON と型を変えない | JSON の往復 3 本（`json_integration_tests`） | どちらの案でも `src` に触らない＝満たしやすい。案 A でも `src` は無変更で済む（serde の導出だけで乗る） |
| 4.1・4.4 注記 | `boundary_test.rs` 259〜263 行目 | **Missing**（文の直し） |
| 4.2・4.3 案 A の境界のテスト | — | **Missing＋Unknown**（下の 4.2：新しいクレートは untagged の腕の `.nan` を既定で断る可能性） |
| 5.1〜5.4 README・steering・`Cargo.lock` | 2 章の表 | **Missing**（文の直し）。steering の 3 か所は `audit-dola` の分で今も古い |
| 5.5 謝辞は道具で作り直す | `tools/test-all.ps1 -License` の段が作り直し、差分を知らせる | どちらの案でも差分 0 の見込み（2 章） |
| 6.1〜6.6 申し送り 1 項目 | roadmap 207 行目の `audit-dola` の項目 | **Missing**（書き直し）。頭は「`v0.1.0` の後」（今の最新の公開版は v0.1.0） |
| 7.1・7.2 全体テスト | `tools/test-all.ps1 -License` | 既存の手順どおり |

## 4. 案の比べ

### 4.1 案 A＝替える（機能名 `yaml` を保ち、読み書きを差し替える）

**候補の今の状況（2026-11-13 に crates.io の API と RustSec で測った）**

| クレート | 最新の版・日付 | ライセンス | 保守と勧告 | メモ |
|---|---|---|---|---|
| `serde-saphyr` | 2.1.1・2026-11-05 | MIT OR Apache-2.0（`allow` に入る） | 活発（総 1,860 万 DL）。RustSec にこのクレートの頁は無い＝勧告 0 | 勧告 `RUSTSEC-2024-0449` が serde の利用者向けに名指しする代わり。MSRV 1.89（手元は rustc 1.100.0・edition 2024＝問題なし）。既定の機能 `serialize`・`deserialize`・`comments`。読み手は `granit-parser` 2.1.0（2026-11-02・MIT OR Apache-2.0・純 Rust） |
| `noyalib` | 0.0.60・2026-11-10 | MIT OR Apache-2.0 | 勧告が 2 つ目に挙げる代わり。0.0.x で版が若い | 総 127 万 DL |
| `serde_yml` | — | — | **`RUSTSEC-2025-0068`（2025-09-12・unsound かつ保守されていない）** | 候補から外れる |

- **`Cargo.lock` の動き（見込み・設計で `cargo tree` で実測）**: 消える＝`serde_yaml`・`unsafe-libyaml`・`ryu`。増える＝`serde-saphyr`・`granit-parser`・`annotate-snippets`（とその依存）・`encoding_rs_io`・`nohash-hasher` ほか。既に在る物を使う＝`serde_core`・`num-traits`・`smallvec`・`base64`・`zmij`・`encoding_rs`。`default-features = false` で `serialize`＋`deserialize` だけにすれば `comments` の分が減る（**Research Needed**）。
- **ライセンス**: 直接の 2 つは `allow` に入る。系統の全部は `cargo deny check licenses` で実測する（`deny.toml` の `[graph] all-features = true` なので機能 `yaml` の系統も監査に入る）。
- **触るファイル**: 2 章の表のとおり＋`tech.md` の依存の表へ 1 行（「意図的依存追加」の承認の書き方に合わせる）。
- 利点: crates.io で機能 `yaml` を指定していた利用者は書き直し不要（要件 2.2）。
- 欠点: 依存が増える（`dola` に機能 `yaml` が足す物は依然「依存が 1 つ増えるだけ」＝`dola` の公開の API は YAML の関数を持たない）。往復のテストを常時回す段が無い（要件の議題 2）。読み方の違いの確かめが要る（次節）。

### 4.2 案 A の調べ物: `.nan`／`.inf` と `#[serde(untagged)]`

- `dola` の型は `#[serde(untagged)]` が多い（`value.rs` の `DynamicValue`・`transition.rs` 2 か所・`storyboard.rs` 3 か所・`easing.rs`）。untagged の enum は serde が中身を一度 `deserialize_any` で受けてから腕を試す。
- `serde-saphyr` 2.1.1 の説明書（docs.rs）: 型の決まった `f64` は `.nan`・`.inf` を受ける。型の決まらない読み（`deserialize_any`）は既定の `Options::non_finite_float_policy = Reject` で `.nan`・`.inf`・`-.inf` を**誤りとして断る**（`AsString`・`PassThrough` に替えられる）。
- ⇒ `DynamicValue::Float` や `transition.rs` の `Scalar(f64)` の腕に来た `.nan` は、`serde_yaml` では値として読めて validate の数値の規則が拒む、`serde-saphyr` では読み込みで断られる、という違いになる見込み。どちらも要件 4.2 の「断られるか、validate が拒むか」は満たすが、4.3 の「違いをコメントに書く」の材料になる。**Research Needed**（設計で 1 本ずつ実測）。
- 往復のテスト（要件 2.5）も untagged を通る。整数と小数の区別（`Integer(i64)` を `Float(f64)` より先に試す）や `1.0` の書き出しの形が `serde_yaml` と同じに往復するかは未確認＝**Research Needed**。

### 4.3 案 B＝外す（機能 `yaml` と任意の依存を消す）

- 確かめたこと: ワークスペースで機能 `yaml` を有効にするクレート 0・道具と CI の指定 0・`src` の使い 0・再輸出 0。
- `Cargo.lock`: `dola` の依存の行から `serde_yaml` が消え、`serde_yaml`・`unsafe-libyaml`・`ryu` の 3 項目が消える。足す依存 0。
- `THIRD-PARTY-NOTICES.md`: 差分 0 の見込み。
- 触るファイル: `crates/dola/Cargo.toml`・往復のテスト・`integration_test/mod.rs` 1 行目・`boundary_test.rs` の注記（と先頭の説明）・README・`deny.toml`・`Cargo.lock`・steering 3 つ・roadmap の 1 項目。
- 利点: 先例（#440・0.0.2 の `runtime`）と同じ形で「dola は書式の機能を持たない」に揃う。新しい依存の監査・読み方の違いの調べが不要。テストを回さない機能が残らない。
- 欠点: crates.io の利用者のうち `features = ["yaml"]` を書いていた人のビルドが通らなくなる（壊れる変更・直すのは 1 行）。総 DL 79 で、外の使い手の有無は crates.io からは分からない。

### 4.4 案 C（混ぜる）について

「機能 `yaml` を残し中身を空にする（依存を持たない機能名だけ）」は、名前を保つが何もしない機能になり、要件 2.1 の「1 つだけの形」と README の正しさ（5.1）に反しやすい。「`deprecated` の印を付けて 1 版残す」は Cargo の機能に非推奨の印が無く、依存を残せば勧告も残る。⇒ 現実的な第 3 の案は見当たらない。

## 5. 規模と危うさ

| 案 | 規模 | 危うさ | 一言の理由 |
|---|---|---|---|
| A | S（4〜6 タスク） | 中 | 依存の追加と untagged での `.nan` の扱い・往復の形の違いを実測で確かめる必要がある |
| B | XS（3〜4 タスク） | 低 | 先例どおりの削除で、使い手 0・`src` 無変更・足す依存 0 |

## 6. 設計の段への申し送り

> **2026-11-13 要件の討議の結論（改め）**: 機能 `yaml` を外し、機能 `toml` を入れる（開発者「残すならtoml、yamlを外してtomlを入れよ」「tomlを外せなどという判断はした覚えがない」）。下の「Research Needed（案 A を採る場合だけ）」は不要。代わりに設計で確かめること: ⑴ `toml` 1（ワークスペースの版・既定の機能）で `complete_document_toml_roundtrip` が往復するか（`audit-dola` の前にあった TOML の往復のテストは一度も回っていない＝`#[serde(untagged)]`・`Option`・表の入れ子が TOML で書けるかは未確認）⑵ TOML の `nan`・`inf` を型の決まった `f64` の欄と untagged の欄に入れたときの振る舞い ⑶ 機能 `toml` のテストを全体テストで常に回す道（議題 4＝常に回すと決まった）⑸ 機能 `toml` の下の読み書きの入口の名前と誤りの型（議題 3＝入口を持つと決まった）⑷ `cargo tree -p dola --features toml` で新しく引くクレートが無いこと（`toml` 1 は既に `Cargo.lock` にある）。

- **推す形の材料**（決めるのは要件の討議）: 要件の推奨は案 B。測った事実はそれを支える（使い手 0・テストが回ったことが無い・謝辞が動かない・先例あり）。案 A を採る理由になりうるのは「crates.io の利用者の `features = ["yaml"]` を壊さない」1 点。
- **Research Needed（案 A を採る場合だけ）**:
  1. `serde-saphyr` を `default-features = false, features = ["serialize", "deserialize"]` で引いたときの `cargo tree -p dola --features yaml` と `cargo deny check`（licenses・advisories）の実測。
  2. `complete_document_yaml_roundtrip` が `serde-saphyr` で等しく往復するか（untagged・整数と小数）。
  3. `.nan`・`.inf`・`-.inf` を `DynamicValue`・`TransitionValue` 系の untagged の腕と、型の決まった `f64` の欄に入れたときの振る舞い（読み込みで断るか・validate が拒むか）。
  4. 完了の証跡で `cargo test -p dola --features yaml` を 1 回回して記録する手順（全体テストの段は足さない、が要件の議題 2 の推奨）。
- **どちらの案でも確かめること**: `deny.toml` の見逃しを消した後の `cargo deny check` の終了コード 0（実測の写しでは赤はこの 1 件だけ）。`tools/test-all.ps1 -License` で `THIRD-PARTY-NOTICES.md` の差分 0。
- **道具のメモ**: `cargo deny` 0.20.2 で設定の写しを使うときは `cargo deny --config <path> check advisories`（`--config` は `check` の前）。

## 7. 出典

- RustSec `RUSTSEC-2024-0449`（unsafe-libyaml・unmaintained・代わりに saphyr／serde-saphyr・noyalib）: https://rustsec.org/advisories/RUSTSEC-2024-0449.html
- RustSec `serde_yml` の頁（`RUSTSEC-2025-0068`）: https://rustsec.org/packages/serde_yml.html
- crates.io API: https://crates.io/api/v1/crates/serde-saphyr ・ https://crates.io/api/v1/crates/serde-saphyr/2.1.1 ・ https://crates.io/api/v1/crates/granit-parser ・ https://crates.io/api/v1/crates/noyalib ・ https://crates.io/api/v1/crates/dola
- serde-saphyr 2.1.1 の説明書（`non_finite_float_policy`）: https://docs.rs/serde-saphyr/2.1.1/serde_saphyr/
