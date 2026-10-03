# ギャップ分析: areka-P0-crates-io-publish

> **2026-10-03 要件討議の議題 3 で範囲が改まった**: crates.io へ版を出し続けるのは `wintf`・`dola` の 2 つだけ（`areka` は 0.0.1 の名前の確保のまま・本体と部品は出さない）。以下の分析は改める前の「21 クレートを出す」前提で書かれている。読み替え:
> - パスだけの依存に版を足すのは `wintf` → `dola` の 1 行だけ（要件 1.7）。59 行の書き足し・案 A／案 B（§4・判断事項 1）は要らない。ただし、出さない `areka`・`areka-emo-present`・`areka-emo-text` が持つ `wintf = { version = "0.0.1", path = ... }` の 3 行は、版を 0.0.2 に上げると「0.0.1 ちょうど」に合わず組み立てが落ちる。版の指定を外すか、版上げで一緒に動かすかを設計で決める（`release-cycle` の版上げの手順への申し送り）。
> - 初回の手元からの公開・新しいクレートの公開の速さの上限・`crates/areka/shell/org/` の大きさ（判断事項 9）は関係しなくなった（`areka` を包まない）。
> - 組み立てまでの確認の所要（§2.4.1 の約 12 分）は 21 クレート分の値。2 クレートでは短くなる見込み（設計で測る）。
> - 判断事項 2（きっかけ）は議題 1、判断事項 3（全体テスト）は議題 2 で決定済み。判断事項 5（部品の説明）は消え、判断事項 7（README の文）は `wintf`・`dola` についてだけ要件 5.6 になった。

> 2026-10-03 実施（`kiro-validate-gap`）。基準のコミットは `d2723772`（main `d4f9e93d` ＋ spec の初期化）。
> 手元の道具は cargo 1.99.0／rustc 1.99.0。
> 実測は、追跡されたファイルを `git archive` でワークツリーの `target\crates-io-gap\ws\` へ写し、その写しの中だけで欄を書き換えて行った（リポジトリの追跡ファイルには一切触れていない・crates.io へは何も上げていない・`cargo login` もしていない）。写しは実測の後に消した。

## 1. 分析の要約

- **いちばん大きな穴は「手元のパスだけの依存」**。公開する一覧のクレートの通常の依存のうち、ワークスペース内のクレートを指す 59 行が `version` を持たない（`path` だけ）。今のまま `cargo publish --workspace --dry-run` を回すと、最初の 1 本で「all dependencies must have a version requirement specified when publishing」と止まる（実測）。59 行に版を書き足した写しでは、21 クレートすべてが包めた。
- **版を書き足すと、版を上げるたびにその行も動く**。0.0.x の版の指定は「その版ちょうど」の意味になるので、`release-cycle` が版を 0.0.2 に上げるとき、`Cargo.toml` の 1 行だけでなく、この 62 行（今ある `wintf` の 3 行を含む）も同時に書き換えないと組み立てが通らない。`release-cycle` の brief は「`Cargo.toml` の 1 行（版）」を前提にしているので、どこで版をそろえるかは設計で決める必要がある（後述の判断事項 1）。
- **GitHub の仕組み上、`release: published` だけでは要件 4.1 を満たせない**。`release.yml` が既定の `GITHUB_TOKEN` で作った Release は、別の workflow を起こさない（GitHub の再帰防止の決まり）。代わりに使われがちな `workflow_run` は、crates.io の Trusted Publishing が「安全上の理由で受け付けない」と明言して拒む。きっかけの選び方は設計の最重要の判断になる（判断事項 2）。
- **既存の型はそのまま使える**。手元の確認は `tools/` の PowerShell スクリプト（`tools/test-all.ps1`・`tools/package.ps1` と同じ書き方＝段ごとの合否・終了コード）に乗せられる。`cargo publish --workspace` は依存の順に並べ、まだ crates.io に無い部品を手元の仮の置き場で補って組み立てる（要件 2.1・2.2 は cargo の機能で満たせる）。ただし大きさの上限（2.5）と一覧の食い違い（2.4）は cargo が見ないので、自分で判定を書く必要がある。
- **大きさは足りるが余裕は薄い**。包んだ `areka` は圧縮後 6.6 MB（上限 10 MB の 66%）。中身の 4 割は `crates/areka/shell/org/` の元絵（`.pdn`・`.design`・`.zip`）で、本体の組み立てには使われていない。

## 2. 今の状態（調べた事実）

### 2.1 ワークスペースとクレートの欄

- 根の `Cargo.toml` の `[workspace.package]` は `version = "0.0.1"`・`edition = "2024"`・`authors`・`license = "MIT"`・`repository = "https://github.com/ekicyou/areka"`・`publish = false`。
- 30 クレート（`crates/*`）すべてが `version`・`edition`・`authors`・`license`・`repository` を `workspace = true` で受け継いでいる（`shiori-host32-testdll`・`shiori-host32-testdll-loadu` の 2 つだけ `workspace = "../.."` の行が無いが、members に入っており挙動は同じ）。
- `publish` は 30 クレートすべてが**自分で明示**している（`publish.workspace = true` を使うクレートは 0）。`true` は `areka`・`dola`・`wintf` の 3 つ、残り 27 は `false`。つまり根の `publish = false` は今どのクレートにも効いていない。
  - 注意: cargo は `publish` の欄が無いクレートを「出せる」と扱う。根の既定を残すか外すかで、新しいクレートを足したときの既定の向きが変わる（判断事項 6）。
- `description` は 30 クレートすべてにある（空のものは 0）。ただし、公開する一覧の 18 部品のうち 9 つは日本語で spec 名や内部の呼び名を含む（例: `areka-emo-compose`「三段直列チェーン 2/3」・`areka-ghost`「⓪ghost 結線層」・`areka-mcp`「spec: areka-P0-mcp-server-core」）。crates.io は日本語の説明を受け付ける（要件 1.3 は満たしている）。
- `keywords`・`categories` は `areka`・`dola`・`wintf` だけ。要件は求めていない。
- `readme` の欄を書いたクレートは 0。cargo は欄が無くてもクレートのフォルダの `README.md` を自動で拾い、包んだ `Cargo.toml` に `readme = "README.md"` を入れる。公開する一覧で `README.md` を持つのは `areka`・`dola`・`wintf`・`shiori-host32-host` の 4 つ（要件 1.4 は欄を足さずに満たせる見込み・設計で確かめる）。
  - `areka`・`dola`・`wintf` の README は「0.0.1・名前の確保のための公開」と書いたまま。0.0.2 を出すと crates.io の頁にこの文が載り続ける（要件の外・判断事項 7）。
- `LICENSE-MIT` は根にだけあり、各クレートのフォルダには無い。`license = "MIT"` の欄だけで crates.io は受け付ける（実測でも包めた）。

### 2.2 依存の形（公開する一覧の 21 クレート）

| 調べたこと | 結果 |
|---|---|
| 通常の依存でワークスペース内を指し、`version` が無い行 | **59 行**（`areka` 17・`areka-ghost` 10・`areka-emo-text` 7・`areka-seriko` 5・`areka-emo-present` 4・`areka-kanade` 4・`areka-sakura` 4・`areka-emo-compose` 2・ほか 6 クレート各 1） |
| 通常の依存で `version` が既にある行 | 3 行（`areka`・`areka-emo-present`・`areka-emo-text` の `wintf = { version = "0.0.1", path = "../wintf" }`） |
| テスト専用の依存（`[dev-dependencies]`）で `path` だけの行 | 多数（`log-capture-kit`・`temp-path-kit`・`sample-ghost-kit`・`shiori4-testdll`・`dola`・`shiori-host32-host`）。cargo は包むときに版の無いテスト専用の依存を外すので、そのままでよい（実測で問題なし） |
| 通常の依存が出さない一覧のクレートを指す行 | 0（`areka-update` の常設テスト `dev_only_tools_stay_out_of_production_dependencies` と `log-capture-kit` の走査テストが、テスト用の道具が通常の依存へ入らないことを見張っている） |
| `build.rs` | 0 |
| `[patch]`・`.cargo/config.toml`・`rust-toolchain` | 0 |

- 公開する一覧 21＝`areka` の通常の依存の閉包は、マニフェストから数え直して要件の一覧と一致した（`areka-mcp` を含む・`shiori-host32-helper` は `areka` から辿れない）。

### 2.3 クレートの外を読む `include_str!` など

- 本番のコードでクレートの外のファイルを読むものは 0。
- テストの中（`#[cfg(test)]` の下）にだけ、クレートの外を読むものが 1 つある: `crates/areka/src/input_events/file_drop_wiring_tests.rs` の `include_str!("../../../wintf/src/ecs/window_proc/drop_files.rs")`。包んだ後の組み立て（ライブラリと bin だけ）では読まれないので、公開前の確認は通る。ただし crates.io から落としたクレートでテストを回すと、この 1 本は組み立てで落ちる（要件の外・記録だけ）。
- 他のテストが `CARGO_MANIFEST_DIR` からクレートの外（`../wintf/...`・`../..`）を読むものも数本あるが、どれも実行時の読み込みでテストの中だけ。

### 2.4 実測: 公開前の確認を写しで回した結果

| 段 | 写しへの変更 | 結果 |
|---|---|---|
| ① 今のまま（18 部品の `publish` を `true` にしただけ） | `publish` の 18 行 | **失敗**: `areka` の包みで「dependency `areka-actor` does not specify a version」 |
| ② ①＋59 行に `version = "0.0.1"` | ＋59 行 | `cargo publish --workspace --dry-run --no-verify` が 21 クレートすべてを包めた。`areka`・`dola`・`wintf` は「0.0.1 は既に crates.io に在る」の**警告**（乾いた走りでは止まらない） |
| ③ ②で組み立てまで（`--no-verify` なし） | 同上 | **緑**（終了コード 0・11 分 55 秒・§2.4.1）。まだ crates.io に無い 18 部品は cargo が手元の仮の置き場（`target/package/tmp-registry`）で補った＝要件 2.2 は cargo の機能で満たせる |

- ①②で `Cargo.lock` の中身は変わらなかった（改行コードを除いて一致）＝欄の変更と `version` の書き足しでは `Cargo.lock` は動かない（要件 1.6 の前提は成り立つ）。
- 出す一覧の外の 9 クレートは `publish = false` なので `--workspace` から自動で外れた。
- 包んだ大きさ（圧縮後）: `areka` 6,609,525 バイト・`wintf` 1,087,503・`areka-emo-text` 746,420・`areka-kanade` 352,583・`areka-emo-present` 317,849・ほかはすべて 210 KB 未満。上限 10 MB を超えるものは無い。
  - `areka` の展開後 12.4 MB のうち 5.0 MB が `shell/`（うち大半が `shell/org/` の元絵: `org.design` 0.9 MB・`.pdn` 4 本で約 1.7 MB・`2.zip` 0.2 MB ほか）。`shell/base.png` は example 2 本が使うが、`shell/org/` を参照するコードは見当たらない。

#### 2.4.1 組み立てまで含めた確認の所要

- ③ `cargo publish --workspace --dry-run`（組み立て込み）は**終了コード 0・11 分 55 秒**（写しの `target\` が空の状態から・この開発機・並列は cargo の既定）。21 クレートすべてで `Verifying` が通り、出た警告は「乾いた走りなので上げない」21 件と「0.0.1 は既に在る」3 件だけ。
- cargo は 21 クレートを 1 つずつ包んだ形で組み立てる。外部の依存の機能の組み合わせがクレートごとに違うため、同じ外部クレートを何度も組み立て直す（`windows` 11 回・`bevy_ecs` 5 回・組み立ての単位は計 386）。`target\` が温まっていても、全体テストに足すと毎回数分〜10 分程度は延びる見込みで、brief の「1〜2 分」より長い。
- 組み立てを省く `--no-verify` なら包むだけで、すぐ終わった（時間は測っていない）。
- 包んだ `areka` の `Cargo.toml` には `readme = "README.md"` が自動で入り、版の無いテスト専用の依存（`log-capture-kit` など）は消えていた（要件 1.4 は欄を足さずに満たせることを確認）。

### 2.5 GitHub Actions まわり

- リポジトリに `.github/` は無い（workflow 0 本）。`release.yml` は同じウェーブの `release-ci-workflow` が新しく作る（`push: tags: ['v*']`・権限は `contents: write` だけ・`GITHUB_TOKEN`・`gh release create` で**公開**の Release を作る予定）。
- **`GITHUB_TOKEN` で作った Release は `release: published` の workflow を起こさない**（GitHub の決まり: `GITHUB_TOKEN` が起こした出来事は、`workflow_dispatch` と `repository_dispatch` を除き、新しい workflow の実行を作らない）。同じ形でつまずいた公開リポジトリの報告が多数ある。
- **crates.io の Trusted Publishing は `workflow_run` を拒む**。拒むときの文言は「Trusted Publishing does not support the `workflow_run` event trigger due to security concerns」で、`push`・`release`・`workflow_dispatch` を使えと案内する。
- Trusted Publishing の流れ: crates.io の各クレートの設定に「持ち主・リポジトリ・workflow のファイル名（・任意で environment）」を登録 → workflow は `permissions: id-token: write` で `rust-lang/crates-io-auth-action` を呼び、30 分ほどで切れる鍵を受け取って `cargo publish` に渡す。**クレートが crates.io に 1 版も無いと設定できない**（brief の記述どおり）。
- crates.io の公開の速さの上限（公表値）: 新しいクレートは「続けて 5 つ、その後 10 分に 1 つ」、既にあるクレートの新しい版は「続けて 30、その後 1 分に 1 つ」。初回の 18 の新しいクレートは、5 つの後に 13 × 10 分＝約 2 時間 10 分かかる見込み（要件 3.5 の「途中で止められる」とおり）。2 回目以降は 21 の新しい版＝30 の枠に収まる。
- **`cargo publish --workspace` は、どれか 1 つでもその版が既に在ると、何も上げずに止まる**（乾いた走りでは警告で済むが、本番では止まる）。「既に在るものを飛ばす」旗は cargo に無い。よく使われる回り道は、既に在るものを調べて `--exclude` で外す方法。要件 3.6・4.8 はこの判定を自分で書く必要がある。

### 2.6 文書

- 根の `README.md`: 「プロジェクト概要」から「ライセンス」までの節。入れ方の節は無い（「配布物（zip）を受け取った方向けの使い方は dist/README.txt」の 1 行だけ）。「二層構造」「クレート構成」の節は `areka (予定)` のままで古い（本 spec の範囲外）。
- `dist/README.txt`: ■見出しの節（起動・終了・右クリックメニュー・.nar の入れ方・更新のしかた・記憶の置き場・既知の制限・同梱しているバルーン・同梱物とライセンス）。入れ方・crates.io の節は無い。「時点」の行は `release-cycle` が毎回書き換える。
- `.kiro/steering/tech.md`: Testing 節に「外部 CI は持たない」とある（`release-ci-workflow` が「テストの門は手元・ビルドと配布は Actions」へ改める予定）。crates.io の決めごとは無い。同じ節を両 spec が触るので、追記の場所を分ける配慮が要る。

### 2.7 全体テスト（`tools/test-all.ps1`）

- 66 行。`Step '名前' { コマンド }` で段を足す形。段が赤でも最後まで回し、末尾に一覧と終了コード。公開前の確認を足すなら `Step 'crates.io 公開前の確認' { pwsh -File tools/<新しいスクリプト>.ps1 }` の 1 行で済む（判断事項 3）。

### 2.8 `tools/package.ps1` との置き場の重なり

- `tools/package.ps1` は `target/package/` を `--target-dir` と zip の置き場に使う。cargo の包み（`cargo package`／`cargo publish`）も既定で `target/package/` に `.crate` と仮の置き場（`tmp-registry`・`tmp-crate`）を作る。名前は重ならない（`areka-0.0.2.crate` と `areka-0.0.2-x64.zip`・`stage-x64`）が、同じ場所を 2 つの道具が使う。公開前の確認の置き場を `target/` の下の別のフォルダへ分けるかは設計で決める（要件 2.8 は `target\` の下ならよい）。

## 3. 要件と今の資産の対応

| 要件 | 今ある物 | 足りない物 | 種別 |
|---|---|---|---|
| 1.1 出す印（21） | 3 つは `true` | 18 部品の `publish = true` | 欠け |
| 1.2 出さない印と理由（9） | 9 つとも `publish = false` は明示済み | 理由の 1 行のコメント（`pilot` だけ近い説明が既に在る） | 欠け |
| 1.3 説明・ライセンス・リポジトリ | すべて在る | なし（言葉づかいは判断事項 5） | − |
| 1.4 README を頁に | cargo の自動検出 | 包んだ結果で確かめるだけ | 確認 |
| 1.5 依存の組み合わせを変えない | − | **59 行に版の指定を足すことが「依存の組み合わせを変えない」に収まるか**の解釈（組み合わせ・外部の版は変わらない） | 制約 |
| 1.6 `Cargo.lock` が動いたら止める | 実測で動かない | 手順に検査の 1 行 | − |
| 2.1〜2.3 依存の順・未公開の部品・出さない物を除く | `cargo publish --workspace --dry-run` の機能 | 呼び出すスクリプト | 欠け |
| 2.4 一覧の食い違い | なし | `cargo metadata` から「`areka` の通常の依存の閉包」と「出す印のあるクレート」を比べる判定 | 欠け |
| 2.5 大きさの上限 | なし（乾いた走りは大きさを見ない） | `.crate` の大きさの判定 | 欠け |
| 2.6・2.7 失敗の名前・終了コード | cargo の出力 | スクリプトでまとめる | 欠け |
| 2.8 追跡ファイルを書き換えない・`target\` の外に置かない | cargo は `target/package/` を使う | 汚れた作業木の扱い（`--allow-dirty` を付けるか） | 判断 |
| 2.9 完了時に緑 | − | 0.0.1 のままの乾いた走りで 3 つが「既に在る」の警告になる点の扱い（警告で緑とみなすか） | 判断 |
| 3.1〜3.10 手順書 | なし | 新しい手順書（置き場は判断事項 8）と、残りだけを出す操作 | 欠け |
| 4.1 Release の公開で動く | なし | **きっかけの選び方**（`GITHUB_TOKEN` と `workflow_run` の 2 つの壁） | 制約・要調査 |
| 4.2 ほかの出来事で動かない | − | きっかけ次第で書き方が変わる | 判断 |
| 4.3 タグと版の一致 | `release-ci-workflow` が同じ検査を `release.yml` に持つ予定 | 自分の workflow にも同じ検査 | 欠け |
| 4.4 公開前の確認を先に | 要件 2 のスクリプト | Windows のランナーで回す（クレートが Windows 専用の API に依存） | 欠け |
| 4.5 1 版も無いクレートで止める | なし | crates.io の索引（`https://index.crates.io/`）か `cargo info` で調べる判定 | 欠け |
| 4.6 Trusted Publishing だけ | なし | `rust-lang/crates-io-auth-action`・`id-token: write` | 欠け |
| 4.7・4.8 依存の順・既に在る版を飛ばす | `cargo publish --workspace` | 既に在る物を `--exclude` で外す判定（要件 3.6 と共有できる） | 欠け |
| 4.9 出せた物・出せなかった物を記録 | なし | 実行の要約（`GITHUB_STEP_SUMMARY` など） | 欠け |
| 4.10 Release・winget に触らない | 別の workflow に分ければ自然に満たす | − | − |
| 4.11 秘密を記録に出さない | auth-action は鍵を隠す | `git remote -v` などを使わない書き方 | 制約 |
| 5.1・5.2 README 2 本 | 節が無い | 新しい節（winget の行を後から足せる形） | 欠け |
| 5.3 winget の行を足せる形 | − | 節の形を決めるだけ | − |
| 5.4・5.5 tech.md | 無し | 新しい節 | 欠け |

## 4. 実装の進め方の候補

### 案 A: 今ある物を広げる（各 `Cargo.toml` に直接書き足す）

- 18 部品の `publish = true`・9 つへ理由のコメント・59 行に `version = "0.0.1"` を各クレートの `Cargo.toml` へ直接足す（今の `wintf = { version = "0.0.1", path = "../wintf" }` と同じ形）。
- 公開前の確認は `tools/` に PowerShell を 1 本（`cargo publish --workspace --dry-run` を呼び、一覧の比較と大きさの判定を足す）。
- workflow は `.github/workflows/crates-io.yml` を 1 本。
- 利点: 今の書き方と同じで読みやすい。根の `Cargo.toml` に触らない。
- 欠点: **版を上げるたびに 62 行が動く**。`release-cycle` の「1 行を書き換える小さなスクリプト」の案は成り立たず、`cargo set-version --workspace`（cargo-edit・依存側の版の指定も一緒に直す）を使うか、自前の書き換えが 21 ファイルを回る形になる。手で直すと 1 行の取りこぼしで組み立てが落ちる。

### 案 B: 新しい置き場を作る（版の指定を根に集める）

- 根の `[workspace.dependencies]` に 21 クレートを `areka-actor = { version = "0.0.1", path = "crates/areka-actor" }` の形で並べ、各クレートは `areka-actor = { workspace = true }` で受ける。
- 利点: 版を上げるときに動くのは根の `Cargo.toml` の 22 行（`[workspace.package]` の 1 行＋21 行）だけで、1 つのファイルに閉じる。`release-cycle` の置換のスクリプトも 1 ファイルで済む。
- 欠点: 59＋3 行の書き方が変わる（意味は同じ）。`areka-mcp` の `Cargo.toml` のコメント「根の [workspace.dependencies] は変えない（要件 8.6）」は完了 spec の決めごとで、これと食い違って見える（中身は外部の依存の話で、本件はワークスペース内の依存なので矛盾はしないが、コメントの言い換えが要る）。テスト専用の依存（`log-capture-kit` など）まで集めるかどうかで変更の量が変わる（集めないなら 21 行だけ）。`log-capture-kit` の走査テストは `{ path = ... }` の素の形を見本にしているが、判定は「依存の表のどこに名前が出るか」なので書き方の違いでは崩れない見込み（設計で確かめる）。

### 案 C: 組み合わせ

- `Cargo.toml` 群は案 A か B のどちらか、公開前の確認と「既に在る物を外す」判定は**手元と CI で同じスクリプト**を使う（要件 2 と 4.4・4.8、要件 3.6 と 4.8 が同じ判定を求めているため）。
- workflow は自分のファイル（`crates-io.yml`）に閉じ、きっかけだけ判断事項 2 で選ぶ。
- 利点: 手元で緑なら CI でも同じ判定が通る。判定のテスト（較正）も 1 か所。
- 欠点: スクリプトが「確認」と「出す」の 2 つの顔を持つ（引数で分ける）。

## 5. 規模と危うさ

- **規模: M（3〜7 日・8〜12 タスク）**。欄の変更は機械的だが 21＋9 ファイル。公開前の確認のスクリプトに判定が 3 つ（一覧の食い違い・大きさ・既に在る版）。workflow は本番で一度も試せない（初回は `release-cycle`）ので、乾いた走りの口を作る手間が要る。
- **危うさ: 中**。技術は既知（cargo の機能と公表の仕組み）だが、(1) きっかけの壁（`GITHUB_TOKEN`・`workflow_run`）は設計の選び方を誤ると「本番で一度も動かない workflow」になる、(2) 一度出した版は消せない、(3) workflow の本番の動きは本 spec の中では確かめられず `release-cycle` の初回で初めて分かる。

## 6. 要件討議・設計へ持ち越す判断事項

要件の「要件討議で決める事項」1〜4 に加えて、調べて分かったもの。

1. **版の指定をどこに置くか（案 A／案 B）**。どちらも `release-cycle` の版上げの手順を変える。案 A なら版上げは 62 行＋1 行（`cargo set-version --workspace` を使えば自動）、案 B なら根の 22 行。`release-cycle` の brief の議題（cargo-edit に頼るか自前の 1 行か）と合わせて決める必要がある。また、要件 1.5「依存の組み合わせを変えない」に版の指定の書き足しが含まれないことを確認しておくとよい。
2. **公開の段のきっかけ**（要件 4.1・4.2 に直結）。候補:
   - (a) `release: published` のまま、`release-ci-workflow` 側の `release.yml` に「Release を作った後に `gh workflow run crates-io.yml -f tag=v{版}` を 1 行足す」を頼む（`workflow_dispatch` は `GITHUB_TOKEN` でも起きる・Trusted Publishing も受け付ける）。本 spec は `release.yml` に触らないので、相手の spec への申し送りになる。
   - (b) `crates-io.yml` を `push: tags: ['v*']` で起こし、その版の Release が**公開**になるのを待ってから出す（待つ上限を決める）。`release.yml` に触らずに済むが、「普通の push で動き出さない」（要件 4.2）の読み方が変わる（タグの push で起きて、Release が出なければ何もせず終える）。待つ間もランナーの時間を使う（公開リポジトリは無料）。
   - (c) `release.yml` が Release を作るときの鍵を、`GITHUB_TOKEN` から GitHub App の鍵などへ替える。秘密の置き場を使うので brief の方針と合わない。
   - `workflow_run` は crates.io が拒むので候補にならない。
   - どの案でも、手で同じ版をやり直す口（`workflow_dispatch`）を足しておくと、途中で止まった回のやり直し（要件 4.8）に使える。
3. **全体テストに公開前の確認を足すか**（要件の議題 2）。実測の所要は §2.4.1（組み立て込みで約 12 分・空の `target\` から）。brief と要件の「1〜2 分ほど延びる見込み」より長い。「組み立ては省き包むだけ（`--no-verify`）」を全体テストに、「組み立てまで」を手順書と公開の段に、と分ける案もある。
4. **公開の段で、公開前の確認（組み立て込み）と本番の公開を 1 回の鍵で済ませるか**。Trusted Publishing の鍵は 30 分ほどで切れる。組み立ては手元で約 12 分（ランナーではもっとかかりうる）かかり、さらに cargo は 1 本上げるごとに索引への反映を待つので、21 本を順に上げ終える前に鍵が切れるおそれがある。「確認は鍵を取る前に済ませ、鍵を取った後は `--no-verify` で上げる」と分ける案が考えられる。
5. **部品の説明の言葉づかい**（要件の議題 4）。日本語で内部の呼び名・spec 名を含むもの 9 つ（`areka-emo-compose`・`areka-emo-present`・`areka-emo-text`・`areka-ghost`・`areka-mcp`・`areka-nar`・`areka-sakura`・`areka-seriko`・`areka-update`）。英語でも内部の呼び名が混ざるものが 2 つ（`areka-kanade` の「運行表」・`areka-emo-atlas` の「emo render engine」）。
6. **根の `[workspace.package] publish = false` を残すか外すか**。今どのクレートにも効いていない。残して出さないクレートを `publish.workspace = true` に寄せる／外して全クレートに明示させる（要件 5.5 の決めごとと噛み合う方）／そのまま残す、の 3 通り。cargo は欄が無いと「出す」扱いなので、新しいクレートが欄を書き忘れたときの向きが変わる（要件 2.4 の判定が拾う）。
7. **`areka`・`dola`・`wintf` の README の「0.0.1・名前の確保のための公開」の文**。0.0.2 を出すと crates.io の頁に載り続ける。本 spec で直すか（`README.md` は要件 1.4 で頁に載せる対象）、別に回すか。
8. **手順書の置き場**（要件 3.1）。`doc/` の下に新しく置くか、`.kiro/steering/tech.md` に直接書くか、`release-cycle` の spec の中に置くか。
9. **`crates/areka/shell/org/` を包みから外すか**（`exclude`）。大きさは今は上限内だが、`areka` は 6.6 MB で余裕が 3.4 MB。元絵は本体の組み立てに使われていない。外すと要件 2.5 の余裕が増え、元絵が crates.io に配られることも無くなる。外さない場合も、要件 2.5 の判定が先に止めるので危険ではない。
10. **0.0.1 のままの乾いた走りでの「既に在る」の警告の扱い**（要件 2.9）。本 spec の完了時点の版は 0.0.1 なので、`areka`・`dola`・`wintf` は「既に在る」の警告が出る。公開前の確認はこの警告を緑として扱うか（乾いた走りでは cargo も止まらない）を決めておく。要件 4.8 の「既に在る物は飛ばす」と同じ判定に寄せると一貫する。
11. **汚れた作業木での確認**（要件 2.8）。cargo は未コミットの変更があると包むのを拒む。開発中に回すために `--allow-dirty` を付けるか、手元の公開の前だけはきれいな作業木を必須にするか。

## 7. 要調査（設計で確かめる）

- `cargo publish --workspace --dry-run` の組み立て込みの、Windows のランナーでの所要（手元は約 12 分）と、`Swatinem/rust-cache` などで縮むか。
- `release.yml` の具体の形（`release-ci-workflow` の設計待ち）と、判断事項 2 の (a) を頼む場合の取り決め。
- Trusted Publishing の設定で environment（GitHub の環境）を使うか（使えば公開の段に人の承認を挟める）。
- `cargo publish --workspace --exclude <既に在る物>` で、外したクレートに依存する残りのクレートが crates.io の版を正しく引くこと（初回の途中で止まった回のやり直しの形）。
- crates.io の索引で「その版が在るか」を調べる方法（`https://index.crates.io/` の索引ファイル・`cargo info`・`cargo search`）のうち、反映の遅れ（上げた直後に索引へ出るまでの数十秒）に強いもの。
- 根の `[workspace.dependencies]` に内部のクレートを並べる案 B で、`log-capture-kit` の走査テスト・`areka-update` の常設テストが崩れないこと。

## 8. 出典

- GitHub の再帰防止の決まり（`GITHUB_TOKEN` が起こした出来事は新しい workflow を作らない）と同種の報告: [community discussion #25281](https://github.com/orgs/community/discussions/25281)・[vig-os/tessera #438](https://github.com/vig-os/tessera/issues/438)
- crates.io が `workflow_run` を拒む文言: [water-rs/.github #2](https://github.com/water-rs/.github/issues/2)
- crates.io の公開の速さの上限: [crates.io rate limits](https://crates.io/docs/rate-limits)
- Trusted Publishing の仕組み: [RFC 3691](https://rust-lang.github.io/rfcs/3691-trusted-publishing-cratesio.html)・[rust-lang/crates-io-auth-action](https://github.com/rust-lang/crates-io-auth-action)
- `cargo publish --workspace` が既に在る版で止まる件と `--exclude` の回り道: [Epistates/turbovault #75](https://github.com/Epistates/turbovault/pull/75)（cargo の issue rust-lang/cargo#13397 を参照）
