# Brief: areka-P0-package-mount（本坑 / main・M1 M-boot / parser・loader トラック）

> **種別**: 本坑（main）。M1 `areka-P0-emo2-boot` の **parser/loader トラック**（並行・単体テスト可・host 不要）。**依存は無し＝即着手可・安全並走**（`shell-parse ∥ balloon-parse ∥ package-mount`）。
> **位置づけ**: ゴースト全体の**親コンストラクタ入力の解決**（`install.txt`→レイアウト）。runtime では `ghost-setup` がこれを使ってゴーストを構築し、`shell-parse`/`balloon-parse` へファイルパスを供給する。単体では fixture tree に対し独立にテスト可。

## Problem

emo2 パッケージの `install.txt` とディレクトリツリーを解決して**マウントモデル**（ゴーストレイアウト＝SHIORI パス／shell パス／balloon パス）を返す loader が無い。これが無いと他 parser がどのファイルを読むべきか（surfaces.txt/descript の所在）を決められない。

## Current State（調査済み・接ぎ木先）

- **`areka-parsers` クレート**（`crates/areka-parsers/`）: `sakura` の確立パターンを踏襲。本 spec は **`package` モジュール**を追加（`Result` 無しの寛容パース or 最小 `Result`＝レイアウト解決失敗の扱いは design 議題・sakura は `Result` 無しだが mount は「ファイル不在」という現実の失敗を持ち得る点に注意）。
- **emo2 fixture**: `crates/pilot/examples/shiori-host-32/fixtures/emo2/`（`install.txt` 7 行＝`type,ghost`／`name,えも？？`／`directory,emo2`／`balloon.directory,emo2-kakukaku`。`ghost/master/descript.txt`＝`shiori,pasta.dll`。`shell/master/surfaces.txt`。`emo2-kakukaku/descript.txt`）。
- **出力モデル型は未存在**＝本 spec で定義（mount 結果＝ghost layout）。

## Desired Outcome

emo2 `install.txt`＋ツリーを解決し、マウントモデル（SHIORI パス＝`ghost/master`／SERIKO パス＝`shell/master`／balloon パス）を返せる。emo2 fixture layout を正しく解決して pass（roadmap ✔「emo2 layout 解決」）。

## Approach

`areka-parsers` に `package` モジュールを追加。**emo2 最小**（`doc/emo2-conformance-scope.md`）:

- エントリ `type,ghost` → **3 点マウント**: `ghost/master`（SHIORI）＋`shell/master`（SERIKO）＋`<balloon.directory>`（balloons）。
- 必要フィールド: `type,ghost`・`name`・`directory`・`balloon.directory`。
- **balloon 解決 2 段**: ①ルート直下 `balloon.directory`（`emo2-kakukaku/`）→ ②fallback `<root>/balloon/<name>/`。
- **M1 省略**: NAR インストーラ・`delete.txt`（ファイル掃除）・SAORI 同居。
- 文字コード注意: SHIORI ロード時ディレクトリは CP_ACP（Shift_JIS）系の世界（host-32 側の関心・本 spec はパス文字列の解決まで）。

## Scope

- **In**: `areka_parsers::package` モジュール。マウントモデル型定義。`install.txt` パース（type/name/directory/balloon.directory）＋ツリー解決（ghost/master・shell/master・balloon 2 段解決）。emo2 fixture テスト。
- **Out**: SHIORI ロード・pasta.dll 駆動（host-32 トラック）。surfaces.txt/descript の中身パース（`shell-parse`/`balloon-parse`）。ゴースト lifecycle 構築（`ghost-setup`）。NAR/delete.txt/SAORI。

## Boundary Candidates

- `install.txt` フィールドパース
- ツリー解決（ghost/master・shell/master）
- balloon.directory 2 段解決（ルート → fallback）
- マウントモデル（各パスの束）

## Out of Boundary

- ファイル**内容**のパース（他 parser）。SHIORI/DLL（host-32）。lifecycle 構築（`ghost-setup`）。

## Upstream / Downstream

- **Upstream**: `areka-parsers`（`sakura` パターン）／emo2 fixture tree。
- **Downstream**: `areka-P0-ghost-setup`（マウント結果でゴースト構築）／runtime で `shell-parse`・`balloon-parse` へファイルパス供給／`areka-P0-host32-shiori-load`（`ghost/master` パス＝pasta.dll ロード先）。

## Existing Spec Touchpoints

- **Extends**: `areka-parsers`。
- **Adjacent**: `areka-P0-shell-parse` / `areka-P0-balloon-parse`（同クレート別モジュール・非衝突）。マウント結果が両 parser の入力パスを与える（runtime 結線・spec 着手は独立）。

## Constraints

- Rust 2024・std 中心・`tracing` のみ。**エラー方針は design 議題**（`sakura` は `Result` 無しだが mount は「不在ファイル」等の現実の失敗を持ち得る＝最小 `Result` or 明示的欠落表現を検討）。
- **過剰実装禁止**（emo2 使用フィールドのみ）。emo2 実物 fixture で検証。
- 不確実な install.txt/レイアウト仕様は推測せず `doc/emo2-conformance-scope.md`／fixture を正とし、無ければ質問。
