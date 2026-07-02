# Requirements Document

## Project Description (Input)

展開済みゴーストパッケージの `ghost/master/descript.txt`（起動時のゴースト構築の起点定義）とディレクトリツリーを解決して、ゴーストの**マウントモデル**（SHIORI マウント先パス／shell マウント先パス、後続 parser・エンジンが読むべきファイルの所在の束）を返す loader を `areka-parsers` クレートに追加する（`package` モジュール）。これが無いと `shell-parse`／`balloon-parse` などの下流がどのファイル（surfaces.txt / descript の中身など）を読むべきかを決められず、host-32 がどのディレクトリの SHIORI DLL をロードすべきかも決められない。`areka-P0-parser-foundation`（charset デコード＋KV 共通基盤・2026-07-02 完了）に依存する。UI・COM・ホスト非依存で、fixture tree に対し単体テスト可能。

> **起点の確定（ukadoc 正典準拠）**: 起動時マウントの起点は `ghost/master/descript.txt` である。`install.txt` は **NAR インストーラ配置マニフェスト（D&D インストール時のアーカイブ種別・名称・配置先設定）** であって**起動時には読まれない**（ukadoc 論拠）。よって `install.txt` および NAR 配置解決は本 spec のスコープ外。balloon 所在解決も本 spec のスコープ外（baseware 共有・ユーザ選択であり、ゴーストパッケージ単独からは所在を確定できない。ゴースト `descript.txt` の balloon 系キーはバルーン「名」の希望表明にすぎず、パッケージ内の所在ではない）。したがって本 spec のマウントは **SHIORI＋shell の 2 点**に限る。spec 名 `package-mount` は FINAL だが、要件本文は「descript.txt 起点のゴーストツリー解決」として読むこと。

## Introduction

本機能は、ローカルに展開済みのゴーストパッケージのディレクトリツリーを走査・解決し、後続の parser・エンジンが必要とするファイル群の所在（マウントモデル）を返す純粋 loader である。ukadoc を正典とし、起動時のゴースト構築の起点入力である `ghost/master/descript.txt`（SHIORI ファイル名・ゴースト名・既定シェルディレクトリ名）と、ukadoc の既定シェルディレクトリ規約に従って、SHIORI マウント先と shell マウント先を解決する。emo2 実 fixture を検証対象とし、過剰実装を避ける（emo2 が使用するフィールドのみ）。

## Boundary Context

- **In scope**:
  - 展開済みゴーストパッケージの `ghost/master/descript.txt` を起点とした、ゴースト識別（`type,ghost` の確認）と名前情報（`name`／`sakura.name`／`kero.name`）の取得。
  - SHIORI マウント先ディレクトリの解決（`ghost/master`）と、そこで宣言される SHIORI ファイル名（emo2 では `pasta.dll`）の取得。
  - shell マウント先ディレクトリの解決（ukadoc の既定シェルディレクトリ名規約に従い、指定が無ければ `master` = `shell/master`）。
  - 解決結果を単一のマウントモデル値として返すこと。
  - 必要なファイル／ディレクトリが欠落している場合の観測可能な失敗の表現。
- **Out of scope**:
  - `install.txt` の読み取り・NAR インストーラ配置解決（起動時に読まれない配置マニフェストゆえ、本 spec の入力ではない）。
  - balloon 所在解決（baseware 共有・ユーザ選択であり、ゴーストパッケージ単独からは確定できない。ゴースト `descript.txt` の `balloon`／`recommended.balloon`／`default.balloon.path` はバルーン「名」の希望表明にすぎずパッケージ内の所在ではない）。
  - ファイル**内容**の意味解析（surfaces.txt / descript の中身のパースは `shell-parse`／`balloon-parse` の担当）。
  - SHIORI DLL のロード・pasta.dll 駆動（host-32 トラック）。
  - ゴースト lifecycle 構築（`areka-P0-ghost-setup`）。
  - `delete.txt`（ファイル掃除）・`updates.txt`・SAORI 同居。
  - charset 判定・KV マップ化そのものの実装（`areka-P0-parser-foundation` が提供する共通基盤に依存）。
- **Adjacent expectations**:
  - **依存（先行）**: `areka-P0-parser-foundation`（`areka_parsers::charset` デコード＋`areka_parsers::kv` の素朴 KV マップ）が完了済みであること。descript.txt の読み込みはこの基盤を用いる。
  - **下流**: マウントモデルは `areka-P0-ghost-setup`（ゴースト構築）と、runtime での `shell-parse`／`balloon-parse` へのファイルパス供給、`areka-P0-host32-shiori-load`（SHIORI ロード先ディレクトリ）に消費される。本 spec はパス文字列の解決までで、消費側の挙動は所有しない。
  - **正典**: 伺か仕様の判断は ukadoc を正とする。emo2 fixture は最小サンプルにすぎない。

## Requirements

### Requirement 1: ゴースト起点定義（ghost/master/descript.txt）の解決

**Objective:** 互換ベースウェアの loader として、ゴーストパッケージから起点定義（`ghost/master/descript.txt`）を読み取りたい。そうすれば下流がゴーストの識別情報と SHIORI 所在を得られる。

#### Acceptance Criteria

1. When 展開済みゴーストパッケージのルートパスが与えられたとき, the package loader shall `ghost/master/descript.txt` を起点として読み込む。
2. When `ghost/master/descript.txt` が `type,ghost` を含むとき, the package loader shall それをゴーストパッケージとして受理する。
3. When 起点定義に `name`／`sakura.name`／`kero.name` が含まれるとき, the package loader shall それらの名前値をマウントモデルに含める。
4. The package loader shall 起点定義ファイルの文字コード判定と KV 読み込みを `areka-P0-parser-foundation` が提供する共通基盤（charset デコード＋KV マップ化）に委ね、本 spec では文字コード判定や KV 分割ロジックを重複実装しない。
5. If `ghost/master/descript.txt` が存在しないとき, then the package loader shall マウント解決の失敗として観測可能に表現する（黙って空を返さない）。

### Requirement 2: SHIORI マウント先の解決

**Objective:** loader として、ゴーストの SHIORI 所在（ディレクトリと DLL ファイル名）を解決したい。そうすれば host-32 トラックがどのディレクトリの、どの DLL をロードすべきかを決められる。

#### Acceptance Criteria

1. The package loader shall SHIORI マウント先ディレクトリを `ghost/master`（起点定義が置かれるディレクトリ）として解決する。
2. When 起点定義に `shiori,<ファイル名>` が含まれるとき, the package loader shall その SHIORI ファイル名（emo2 では `pasta.dll`）をマウントモデルに含める。
3. If 起点定義に `shiori` の指定が無いとき, then the package loader shall SHIORI ファイル名の欠落を観測可能に表現する（誤ったパスを推測しない）。

### Requirement 3: shell マウント先の解決

**Objective:** loader として、既定で読み込まれるシェルのディレクトリを解決したい。そうすれば下流の `shell-parse` がどの `shell/<dir>` を読むべきかを決められる。

#### Acceptance Criteria

1. When 起点定義に既定シェルディレクトリ名（`seriko.defaultsurfacedirectoryname`）の指定が無いとき, the package loader shall shell マウント先を `shell/master` として解決する（ukadoc 既定＝`master`）。
2. Where 既定シェルディレクトリ名が起点定義で指定されているとき, the package loader shall shell マウント先を `shell/<指定名>` として解決する。
3. If 解決した shell マウント先ディレクトリが存在しないとき, then the package loader shall shell マウント解決の失敗を観測可能に表現する。

### Requirement 4: マウントモデルの返却

**Objective:** loader として、解決したパス群を単一の値として返したい。そうすれば `ghost-setup` と下流 parser が一箇所から必要な所在を取得できる。

#### Acceptance Criteria

1. When ゴーストパッケージの解決が成功したとき, the package loader shall 解決済みのゴースト識別・名前情報・SHIORI マウント先・shell マウント先を束ねたマウントモデルを返す。
2. The package loader shall UI・COM・SHIORI ホストに依存せず、ローカルディレクトリツリーとその中のテキストファイルのみを入力として動作する。
3. While 与えられた入力が emo2 実 fixture のツリーであるとき, the package loader shall emo2 のレイアウト（`ghost/master` の SHIORI＝`pasta.dll`、`shell/master`）を正しく解決してテストを通す。

### Requirement 5: 失敗の扱いとスコープ外フィールドの無視

**Objective:** loader として、現実に起こりうる欠落（不在ファイル・不在ディレクトリ）を明示的に扱い、かつ本 spec が読まないフィールドは無視したい。そうすれば下流が沈黙した誤りではなく明確な失敗を受け取り、過剰実装も避けられる。

#### Acceptance Criteria

1. If 必須の起点定義またはマウント先ディレクトリが欠落しているとき, then the package loader shall その欠落を観測可能な失敗として表現する（`sakura` パーサの `Result` 無し寛容パースとは異なり、マウントは不在という現実の失敗を持ち得る点に留意）。
2. The package loader shall 想定外・未使用のフィールドを無視し、emo2 が使用するフィールドのみを解決対象とする（過剰実装禁止）。
3. The package loader shall `install.txt`／NAR 配置マニフェストを起点マウントの入力として読まず、balloon 所在解決も行わない（ともに起動時マウントのスコープ外）。
