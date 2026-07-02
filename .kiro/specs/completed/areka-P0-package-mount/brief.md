# Brief: areka-P0-package-mount（本坑 / main・M1 M-boot / parser・loader トラック）

> **種別**: 本坑（main）。M1 `areka-P0-emo2-boot` の **parser/loader トラック**（並行・単体テスト可・host 不要）。**依存: `areka-P0-parser-foundation`（charset デコード＋KV 共通基盤・2026-07-02 完了）が先行**。foundation 完了後は `shell-parse ∥ balloon-parse ∥ package-mount` で並走安全。
> **位置づけ**: ゴースト起動時の**構築起点入力の解決**（`ghost/master/descript.txt`→マウントレイアウト）。runtime では `ghost-setup` がこのマウントモデルを使ってゴーストを構築し、`shell-parse`/`balloon-parse` へファイルパスを供給する。単体では fixture tree に対し独立にテスト可。

> **⚠️ 起点の訂正（2026-07-02・ukadoc 正典準拠）**: 本 spec の**起動時マウント起点は `ghost/master/descript.txt`** である。`install.txt` は **NAR インストーラ配置マニフェスト（D&D インストール時のアーカイブ種別・名称・配置先ディレクトリ設定）** であって**起動時には使わない**（ukadoc 論拠）。旧 brief は起点を `install.txt` と誤記していたため全面訂正した。
> **spec 名について**: `package-mount` という名称は「install.txt パッケージ解決」を連想させ実体（descript.txt 起点のゴーストツリー解決）とズレる（リネーム候補）。名称は現状 FINAL 扱いで変更しないが、**要件本文は「descript.txt 起点のゴーストツリー解決」として読む**こと。

## Problem

展開済みゴーストパッケージの `ghost/master/descript.txt`（起動時の構築起点定義）とディレクトリツリーを解決して**マウントモデル**（ゴーストレイアウト＝SHIORI パス／shell パス）を返す loader が無い。これが無いと他 parser がどのファイル（surfaces.txt/descript の中身）を読むべきかを決められず、host-32 がどのディレクトリの SHIORI DLL をロードすべきかも決められない。

## Current State（調査済み・接ぎ木先）

- **`areka-parsers` クレート**（`crates/areka-parsers/`）: `sakura` の確立パターンを踏襲。本 spec は **`package` モジュール**を追加（`Result` 無しの寛容パース or 最小 `Result`＝マウント解決失敗の扱いは design 議題。`sakura` は `Result` 無しだが mount は「ファイル/ディレクトリ不在」という現実の失敗を持ち得る点に注意）。
- **`areka-P0-parser-foundation`**（2026-07-02 完了）: `areka_parsers::charset` デコード＋`areka_parsers::kv` 素朴 KV マップを提供。descript.txt の読み込みは**この共通基盤を用いる**（charset 判定・KV 分割は再実装しない）。
- **emo2 fixture**: `crates/pilot/examples/shiori-host-32/fixtures/emo2/`。起点＝`ghost/master/descript.txt`（`shiori,pasta.dll`）。shell＝`shell/master/surfaces.txt`。**注: fixture 内の `install.txt` は NAR 配置マニフェストであって起動時マウント入力ではない**（本 spec は読まない）。
- **出力モデル型は未存在**＝本 spec で定義（mount 結果＝ghost layout）。

## Desired Outcome

emo2 の `ghost/master/descript.txt`＋ツリーを解決し、マウントモデル（SHIORI マウント先＝`ghost/master` ディレクトリ＋`shiori,<file>` 名＝`pasta.dll`／shell マウント先＝`shell/<seriko.defaultsurfacedirectoryname>`・既定 `shell/master`）を返せる。emo2 fixture layout を正しく解決して pass（roadmap ✔「emo2 layout 解決」）。

## Approach

`areka-parsers` に `package` モジュールを追加。**emo2 最小**（`doc/emo2-conformance-scope.md`）＝**descript.txt 起点の 2 点マウント**:

- 起点 `ghost/master/descript.txt` を foundation（charset＋KV）で読み、`type,ghost` を確認（ゴースト受理ガード）。
- 名前情報 `name`／`sakura.name`／`kero.name` を取得しマウントモデルへ。
- **SHIORI マウント先**: ディレクトリ＝`ghost/master`（起点定義の置かれるディレクトリ）＋ `shiori,<file>`（emo2＝`pasta.dll`）。
- **shell マウント先**: `shell/<seriko.defaultsurfacedirectoryname>`。指定が無ければ ukadoc 既定の `master`（＝`shell/master`）。
- **balloon 所在解決は Out**（下記 Out of Boundary 参照）。マウントは SHIORI＋shell の 2 点に限る。
- **M1 省略**: NAR インストーラ・`install.txt`（起動時不使用）・`delete.txt`（ファイル掃除）・`updates.txt`・SAORI 同居。
- 文字コード注意: SHIORI ロード時ディレクトリは CP_ACP（Shift_JIS）系の世界（host-32 側の関心・本 spec はパス文字列の解決まで）。

## Scope

- **In**: `areka_parsers::package` モジュール。マウントモデル型定義。`ghost/master/descript.txt` パース（`type`／`name`／`sakura.name`／`kero.name`／`shiori`／`seriko.defaultsurfacedirectoryname`）＋ツリー解決（SHIORI＝`ghost/master`・shell＝`shell/<dir>` 既定 `master`）。欠落（不在ファイル/ディレクトリ）の観測可能な失敗表現。emo2 fixture テスト。
- **Out**: `install.txt`／NAR 配置解決（起動時不使用の配置マニフェスト）。balloon 所在解決（下記）。surfaces.txt/descript の**中身**パース（`shell-parse`/`balloon-parse`）。SHIORI ロード・pasta.dll 駆動（host-32 トラック）。ゴースト lifecycle 構築（`ghost-setup`）。`delete.txt`/`updates.txt`/SAORI。charset 判定・KV マップ化そのものの実装（foundation 依存）。

## Boundary Candidates

- `ghost/master/descript.txt` フィールドパース（type/name/shiori/seriko.defaultsurfacedirectoryname）
- ツリー解決（SHIORI＝ghost/master・shell＝shell/<dir> 既定 master）
- マウントモデル（各パスの束）
- 欠落（不在ファイル/ディレクトリ）の観測可能な失敗表現

## Out of Boundary

- **`install.txt`／NAR 配置解決**: install.txt は NAR インストーラ配置マニフェスト＝**起動時不使用**（ukadoc）。本 spec の入力ではない。
- **balloon 所在解決**: balloon は **baseware 共有・ユーザ選択**であり、ゴーストパッケージ単独からは所在を確定できない（ukadoc 論拠・記憶 areka-ghost-boot-descript-not-install）。よって起点マウント解決のスコープ外。ukadoc 上ゴースト `descript.txt` の balloon 系キー（`balloon,バルーン名`／`recommended.balloon`／`default.balloon.path`）は**バルーン「名」の希望表明**にすぎず、既定値も「SSP標準 or **ユーザーが設定した標準バルーン**」＝実使用 balloon は baseware のユーザ設定が決める（所在確定ではない）。
- ファイル**内容**のパース（他 parser）。SHIORI/DLL（host-32）。lifecycle 構築（`ghost-setup`）。

## Upstream / Downstream

- **Upstream**: `areka-parsers`（`sakura` パターン）／`areka-P0-parser-foundation`（charset＋KV 基盤・完了）／emo2 fixture tree。
- **Downstream**: `areka-P0-ghost-setup`（マウント結果でゴースト構築）／runtime で `shell-parse`・`balloon-parse` へファイルパス供給／`areka-P0-host32-shiori-load`（`ghost/master` パス＝pasta.dll ロード先）。

## Existing Spec Touchpoints

- **Extends**: `areka-parsers`（`package` モジュール追加）／`areka-P0-parser-foundation`（charset＋KV を利用）。
- **Adjacent**: `areka-P0-shell-parse` / `areka-P0-balloon-parse`（同クレート別モジュール・非衝突）。マウント結果（shell マウント先パス）が `shell-parse` の入力を与える（runtime 結線・spec 着手は独立）。

## Constraints

- Rust 2024・std 中心・`tracing` のみ。**エラー方針は design 議題**（`sakura` は `Result` 無しだが mount は「不在ファイル/ディレクトリ」等の現実の失敗を持ち得る＝最小 `Result` or 明示的欠落表現を検討）。
- **過剰実装禁止**（emo2 使用フィールドのみ）。emo2 実物 fixture で検証。
- **正典は ukadoc**。emo2 実物 fixture は最小適合サンプルにすぎず書式の聖典ではない。不確実な descript.txt/レイアウト仕様は推測せず ukadoc／`doc/emo2-conformance-scope.md`／fixture を正とし、無ければ質問。
