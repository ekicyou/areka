# Brief: areka-P0-self-alpha-declaration

> 2026-10-04 棚卸㉑で起票（roadmap「覚え書き」の「`shell-implicit-surface` の着地で残した 7 件」のうち ⑵⑶⑸ と、バルーンの `use_self_alpha` を格上げ）。**段は優先（バルーン）**＝説明書の既知の制限「半透明を前提に作られたバルーンだけが正しく表示されます」を消す。

## Problem

- **利用者**: 説明書 `dist/README.txt` の「◆ 既知の制限: 半透明を前提に作られたバルーンだけが正しく表示されます」のとおり、「半透明を使わない」と書いたバルーンは、ふちや影が作者の意図どおりに出ない。
- **ゴースト作者**: シェルの `seriko.use_self_alpha`（https://ssp.shillest.net/ukadoc/manual/descript_shell.html#seriko.use_self_alpha_2c_5024:1）とバルーンの `use_self_alpha`（https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#use_self_alpha_2c_5024:1）を何と書いても、areka は常に `1` として扱う。
  - 正典の既定は `0`。`full` は「アルファチャンネルのない画像も全て不透明として扱う」。
  - 描画メソッドの説明は `full` を勧めている（例: `blend-add-fast` の「seriko.use_self_alpha,fullを設定することを推奨する」）。
- **ゴースト作者**: `surfaces.txt` が無い、または波括弧が 0 個で画像だけのシェルは起動に失敗する。正典の作り方の説明（https://ssp.shillest.net/ukadoc/manual/dev_shell.html）は「「surface○○.png」…という名前の画像を用意するか」と、画像だけのシェルを認めている。

## Current State

- 透過の扱いを表す型 `UseSelfAlpha`（`On`・`Full`・`Off`）は `crates/areka-emo-atlas/src/normalize.rs` に正典どおり 3 通りある。`Normalizer::select_source` の動作表も 3 通りを書いている。
- ただし実装されている腕は `On` の下の 2 つ（α を採る・左上の色を抜く）だけ。`Full` の下の「全面不透明」（`AlphaSource::Opaque`）と、`Off` の下の抜き色は「シーム・未実装」。
- 呼ぶ側はどこも `UseSelfAlpha::On` を決め打ちで渡し、宣言を読む経路が無い。本番の決め打ちは次の 5 か所:
  - `crates/areka-emo-present/src/shell_target.rs` の `build_shell_target`（シェルの絵）
  - `crates/areka-emo-present/src/balloon.rs` の `build_balloon_target_from_faces`（バルーンの絵）
  - `crates/areka-emo-compose/src/atlas_bind.rs`
  - `crates/areka-emo-atlas/src/manifest.rs`
  - `crates/areka-emo-atlas/src/lib.rs`
- 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml`: `seriko.use_self_alpha` は `absent`・担当なし。バルーンの `use_self_alpha` は縮退（「`use_self_alpha,0` と書いたバルーンも 1 として扱われる」）。
- 画像だけのシェル: `crates/areka-emo-present/src/shell_target.rs` の `load_shell_target` は、`surfaces.txt` が読めないと `ShellLoadError::Read`、面を 1 つも産まないと `ShellLoadError::Empty` で失敗する。`surface<数字>.png` を面として認める仕組み（完了 `shell-implicit-surface`）は、`surfaces.txt` が在る前提の上に乗っている。

## Desired Outcome

- シェルの `seriko.use_self_alpha` とバルーンの `use_self_alpha` の宣言が届き、`1`・`full`・`0`（既定）がそれぞれ正典どおりに描かれる。
- `surfaces.txt` が無い、または波括弧 0 個のシェルでも、`surface<数字>.png` だけで起動できる。
- 説明書の「半透明を前提に作られたバルーンだけが正しく表示されます」の節と、台帳の該当行を実態に合わせて直せる。

## Approach

- 宣言を読む口を、シェルとバルーンの descript の読み手（`areka-parsers`）に足し、5 か所の決め打ちを「読んだ値を渡す」に置き換える。
- `normalize.rs` の残る 2 つの腕（`Full` の全面不透明・`Off` の抜き色）を実装する。`Off` は α 付きの絵が乗算済みで届く問題がある（議題 1）。
- 画像だけのシェルは、`surfaces.txt` が無いことを「空の定義」と同じに扱い、面の画像だけで表を組む。

## Scope

- **In**:
  - ⑵ `full`（全面不透明）。
  - ⑶ 宣言を読む経路（シェルとバルーンの両方）と `Off` の下の抜き色。
  - ⑸ `surfaces.txt` が無い／波括弧 0 個で画像だけのシェル。
  - 決定論のテスト（合成した画像で 3 通り × α あり・なし）・網羅台帳の行（`assets.toml`）・`dist/README.txt` の該当の節・検体での見た目の確かめ。
- **Out**:
  - ⑴ `.pna`（開発者方針で非対応のまま。台帳と説明書の記述は保つ）。
  - ⑷ 全画素が透明になる面の窓・当たり判定（要件の段で確かめる項目に入れるかを決める）。
  - バルーンの `use_input_alpha`（入力ボックス系のバルーン）。
  - ⑹ と `surface.append` の行にだけ現れるファイル名（議題 2）。

## Boundary Candidates

- 宣言の読み取り（`areka-parsers` のシェル・バルーンの descript）。
- 正規化の腕（`areka-emo-atlas` の `normalize.rs`）。
- 読んだ値の受け渡し（emo-present・emo-compose・emo-atlas の 5 か所）。
- シェルの読み込みの入口（`shell_target.rs` の `load_shell_target`）。

## Out of Boundary

- 動く絵の読み込み（`animated-image-decode`）と、element でサーフェスを置く入口（`surface-element-nesting`）。どちらも同じクレートを触るので、本 spec はその後に並べる。
- 合成の描画メソッドそのもの（`overlay` 以外のメソッドの中身）。

## Upstream / Downstream

- **Upstream**: 完了 `emo-atlas`（型の口）・完了 `shell-implicit-surface`（抜き色の `On` の腕・面の画像の慣習）・完了 `balloon-parse`。
- **Downstream**: `balloon-canon-residue`・`shell-companion-balloon`（バルーンの見た目の確かめに使う検体が増える）。

## Existing Spec Touchpoints

- **Extends**: 完了 `emo-atlas`（`UseSelfAlpha` の 3 通りのうち 1 通りだけ着地）・完了 `shell-implicit-surface`（7 件の残りのうち ⑵⑶⑸）。
- **Adjacent**: `animated-image-decode`（`areka-emo-atlas/src/{decode.rs, lib.rs}`）・`surface-element-nesting`（`areka-emo-atlas/src/manifest.rs`・`areka-emo-compose`）・`animated-image-playback`。

## Constraints

- 既定を正典どおり `0` にすると、宣言の無い既存のシェル・バルーンの見た目が変わりうる。同梱の えも？？ と Balloon for Staysee Syncfield、`vendors/sample_ghost/` の検体で変化の有無を確かめる（議題 3）。
- 乗算済みの BGRA で統一する決まり（`normalize.rs` の D8）を崩さない。
- テストは兄弟ファイルへ。1 ファイル 1,000 行以下。ログの無い失敗の経路を作らない。

## 2026-10-04 棚卸㉑で起票

- **出どころ**: roadmap「覚え書き」の「`shell-implicit-surface` の着地で残した 7 件」の ⑵・⑶・⑸。説明書の「◆ 既知の制限: 半透明を前提に作られたバルーンだけが正しく表示されます」。
- **規模**: S〜M（8〜12 タスク）。
- **前提**: `animated-image-decode`（emo-atlas の decode と `lib.rs`）と `surface-element-nesting`（`manifest.rs`・emo-compose）の後。
- **触るファイル**: `crates/areka-emo-atlas/src/{normalize.rs, manifest.rs, lib.rs}`・`crates/areka-emo-present/src/{shell_target.rs, balloon.rs}`・`crates/areka-emo-compose/src/atlas_bind.rs`・`crates/areka-parsers/src/`（シェルとバルーンの descript の読み手）・`doc/ukadoc-coverage/ledger/assets.toml`・`dist/README.txt`。
- **共有しうる相手**: `animated-image-decode`・`surface-element-nesting`・`animated-image-playback`（emo-atlas・emo-compose）・`balloon-font-file`／`balloon-canon-residue`（バルーンの descript の読み手）・`dist/README.txt` を触る配布の spec。
- **議題**:
  1. `0` のときの抜き色。α 付きの絵は乗算済みで届くので、デコードの段で α を捨てるか、正規化の段でまっすぐの α に戻すか。
  2. ⑹（`overlay` 以外の描画メソッドの `element0` を持つ面で画像が土台に使われるずれ）と、`surface.append` の行にだけ現れる絵のファイル名を、本 spec に同居させるか。
  3. 既定を `0` にしたとき、宣言の無い既存の資産の見た目が変わる場合の扱い。


## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: S〜M（7〜10 タスク）。起票時の 8〜12 より下振れ（本番の決め打ちが 2 か所だけだった）。切らない。
- 前提の状態: `animated-image-decode`・`surface-element-nesting` とも着地済み＝今すぐ着手できる。
- 崩れた前提／古くなった位置:
  - **本番の決め打ちは 2 か所だけ**: シェルを焼く集合を組む `build_shell_target_with_boxes`（`areka-emo-present/src/shell_target.rs`）と、バルーンの `build_balloon_target_from_faces`（`balloon.rs`）。Current State が挙げる `manifest.rs`・`lib.rs`・`atlas_bind.rs` の 3 か所は、どれも `#[cfg(test)]` の試験の中で本番の経路ではない。→ `manifest.rs`・`atlas_bind.rs`・emo-compose を触らずに済み、**シェルの element の列から外せる**（roadmap の「`manifest.rs`・`atlas_bind.rs` を共有するので playback とは別のウェーブ」の理由は消えた）。
  - `animated-image-decode` が抜き色の処理を `clear_key_color`（`normalize.rs`）へ切り出し、動く絵の 2 枚目以降のコマ（`animated.rs`）へ 1 枚目の左上の色を使い回す形にした。色は焼く入口（`lib.rs` の `bake_with_limits`）が `Normalizer::key_color` で鍵ごとに求めて渡す。`full`・`0` の腕を足すとき、コマの側も同じ設定で揃うかを確かめる（揃わなければ `lib.rs`・`animated.rs` も触る）。
  - 宣言の読み口: バルーンは `balloon.rs` が `descript.txt` の基層を既に寛容に読んでいる（`read_descript_layer`）＝`areka-parsers` のバルーンの読み手（文字とバルーンの列）に触らずに引ける。シェルの `descript.txt` は emo-present では読んでいない（読むのは `placement/source.rs`）。
  - 画像だけのシェル: `load_shell_target` の `surfaces.txt` の読み取りの失敗と「面が 1 つも無い」の判定は起票時のまま。`surface-element-nesting` の design-validation が「`surface.append*`ブレスの画像が焼かれない」を本 spec の議題 2 に預けたまま。
- 触るファイル（並走の照合用）:
  - `crates/areka-emo-atlas/src/normalize.rs`（コマにも効かせるなら `lib.rs`・`animated.rs`）と兄弟のテスト
  - `crates/areka-emo-present/src/{shell_target.rs, balloon.rs}`
  - `doc/ukadoc-coverage/ledger/assets.toml`・`dist/README.txt`（`winget-manifest-submission`・`mcp-stdio-bridge` と同じファイル＝同じウェーブなら別々の節か確かめる）
- 議題（答えで作業が変わるものだけ）: 起票時の 3 つに 1 つ足す。
  4. シェルの宣言を `load_shell_target` の中で読むか、引数で渡すか。推しは中で読む＝呼び手（`placement/measure.rs`・`emo2_boot/assets.rs`・MCP の試験の支え）を変えずに済み、`placement-measure-bake-once` と並べられる。
- 見つけた穴: Current State の「本番の決め打ちは次の 5 か所」は誤り（上のとおり 2 か所）。

### 同じウェーブ C4 の約束（2026-10-05 棚卸㉒・破るなら止めて報告）

- compose・seriko に触らない。emo-present は `shell_target.rs`・`balloon.rs` だけ（C4 で emo-present を触るのは本 spec だけ）。`dist/README.txt` は既知の制限の行だけ（`release-cycle` が「時点」の行を直す）。
