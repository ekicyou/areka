# Implementation Plan

> 対象: `crates/areka-parsers` に `package` module を追加（`model` ＋ `resolve` の 2 submodule）。
> 依存: `areka-P0-parser-foundation`（`charset::decode` / `kv::parse_kv`・完了済み）。
> 正典: ukadoc（SSP 仕様）。emo2 fixture は最小サンプル。

- [ ] 1. Foundation: `package` module の接ぎ木とスケルトン
- [x] 1.1 `areka-parsers` クレートへ `package` module を接ぎ木する
  - `lib.rs` に `package` module を公開追加し、doc コメントに「`package` module のみローカルディレクトリツリー走査（`std::fs`）を行い、他 module は従来どおり純粋関数群」である旨を補記する
  - `package` の `mod.rs` を作成し、公開面（`MountModel`・付随型・`MountError`・`resolve`）を集約する枠と、内部依存方向（`model ← resolve`）の doc を置く
  - `mod.rs` の module doc に「本 module は parser ファミリ内で唯一 I/O（`std::fs` 読取のみ）と `Result` を持つ。理由＝マウントは物理不在という現実の失敗を持つため（`sakura` の寛容パースと意図的に非対称）」を明記する
  - 観測可能な完了: `cargo build -p areka-parsers` が成功し、`package` module が公開されて再エクスポート面が解決する（`resolve` はこの時点でスタブ可）
  - _Requirements: 4.2_

- [ ] 2. Core: マウントモデル型（`model` submodule）
- [x] 2.1 解決済みマウント所在と観測可能失敗の型を定義する
  - ゴースト名前情報（`name`／`sakura.name`／`kero.name` を各 `Option`・未指定は欠落として保持し推測しない）、SHIORI マウント先（ディレクトリ＋ファイル名 `Option`）、shell マウント先（ディレクトリ）を束ねたマウントモデル型を正本として定義する
  - マウント解決の致命的失敗を表す失敗型を定義する（起点不在・起点読取不能・shell ディレクトリ不在の 3 種）
  - 型は `#[non_exhaustive]` ＋最小 derive（`Clone, Debug, PartialEq, Eq`）とし、`serde` は付さない（兄弟型と整合）。パスは `PathBuf`
  - 観測可能な完了: 型がコンパイルされ、`model_tests` で構築・フィールドアクセス・名前/ファイル名の `Option` 既定（欠落＝`None`）が検証されて緑
  - _Requirements: 1.4, 1.6, 2.1, 2.3, 3.3, 4.1, 5.1_
  - _Boundary: package::model_

- [ ] 3. Core: マウント解決（`resolve` submodule）
- [x] 3.1 descript.txt 起点の解決（正常系）を実装する
  - `ghost/master/descript.txt` を `std::fs::read` → `charset::decode(bytes, default_encoding)` → `kv::parse_kv` の合成で読み込む（charset 判定・KV 分割は foundation へ委譲し再実装しない）
  - `default_encoding` は呼び出し側から受け取り `decode` へ素通しする（package はエンコーディング既定を判断しない・SSP 準拠既定は ANSI・非 UTF-8 拒否のエンフォースは下流 SHIORI 層）
  - ゴースト識別は所在ベース（`ghost/master/descript.txt` の所在で受理）とし、`type,ghost` は確認的で `type` 行の欠落を失敗としない（type 分岐を作らない）
  - `name`/`sakura.name`/`kero.name`（欠落は `None`）、SHIORI マウント先（ディレクトリ＝`ghost/master`・`shiori,<file>` は欠落なら `None` で推測しない）、shell マウント先（`seriko.defaultsurfacedirectoryname` 指定時は `shell/<名>`、無指定時は ukadoc 既定 `shell/master`）を解決し、shell ディレクトリの物理存在を確認して単一のマウントモデルを構築する
  - 参照キーは `name`/`sakura.name`/`kero.name`/`shiori`/`seriko.defaultsurfacedirectoryname` のみとし、`install.txt`／balloon 系キー／NAR には一切触れない
  - 観測可能な完了: 正常なツリーに対し `resolve` が `Ok(MountModel)` を返し、SHIORI ディレクトリ・shell ディレクトリ（存在確認済み）・名前・shiori ファイル名が期待どおり格納される
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 2.1, 2.2, 2.3, 3.1, 3.2, 4.1, 4.2, 5.2, 5.3_
  - _Depends: 2.1_
  - _Boundary: package::resolve_
- [x] 3.2 マウント解決の失敗系を実装する
  - 起点 `ghost/master/descript.txt` が不在なら起点不在の失敗、所在するが読取に失敗したら起点読取不能の失敗、解決した shell ディレクトリが不在なら shell 不在の失敗を、いずれも観測可能な失敗（`Err`）として早期 return する（黙って空を返さない）
  - `sakura` の `Result` 無し寛容パースとは意図的に非対称であることをコード近傍に保つ
  - 観測可能な完了: 各欠落・読取失敗条件に対し `resolve` が対応する失敗 variant を返す（起点不在／読取不能／shell 不在の 3 経路が区別できる）
  - _Requirements: 1.6, 3.3, 5.1_
  - _Depends: 3.1_
  - _Boundary: package::resolve_

- [ ] 4. Validation: テスト
- [x] 4.1 (P) `resolve` の単体テスト（欠落・境界系）を追加する
  - 起点不在→起点不在失敗、`shiori` 未指定→ファイル名 `None`（既定へ推測しないことを検証）、`seriko.defaultsurfacedirectoryname` 未指定かつ `shell/master` 実在→既定 `master` フォールバック、shell 指定だがディレクトリ不在→shell 不在失敗、`type,ghost` 欠落でも所在ベースで受理→`Ok`、descript.txt をディレクトリとして作成し読取失敗→起点読取不能失敗（クロスプラットフォームで確実に誘発可能）
  - 観測可能な完了: 上記 6 ケースの単体テストが緑となり、失敗型の全 3 variant が実行検証される
  - _Requirements: 1.2, 1.3, 2.3, 3.1, 3.3, 5.1_
  - _Depends: 3.2_
  - _Boundary: package::resolve（resolve_tests）_
- [ ] 4.2 (P) emo2 実 fixture の統合テストを追加する
  - `crates/pilot/examples/shiori-host-32/fixtures/emo2/` を入力に、SHIORI ファイル名＝`pasta.dll`・SHIORI ディレクトリ＝`ghost/master`・shell＝既定 `shell/master`（emo2 は `seriko.defaultsurfacedirectoryname` 不在）に解決されることを検証する
  - 名前情報（`name`／`sakura.name`／`kero.name`）が UTF-8 で正しく取得される（foundation 経由のデコードを含む）ことを検証する
  - emo2 の未使用フィールド（カンマ無し行・`craftman`・`homeurl`・ルート `install.txt`・`emo2-kakukaku/`・`delete.txt`）が結果に一切影響しないことを検証する
  - 観測可能な完了: emo2 レイアウトが正しく解決され統合テストが緑（roadmap「emo2 layout 解決」を充足）
  - _Requirements: 1.4, 1.5, 4.1, 4.3, 5.2, 5.3_
  - _Depends: 3.2_
  - _Boundary: package（validation_tests）_
