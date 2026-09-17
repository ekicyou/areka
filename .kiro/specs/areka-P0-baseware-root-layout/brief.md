# Brief: areka-P0-baseware-root-layout

> 2026-09-18 `/kiro-discovery` 再入（棚卸⑭＝α ゴールへの組み直し）で起票。開発者の指示「α 版として第三者に使い始めてもらえる機能セット＝ゴースト・シェル・バルーンのファイル管理、インストール、ネットワーク更新、最低限のメニュー。つまりアプリとしてのデスクトップマスコット管理」。
> 本文の file:line は**起票時の実測値**（2026-09-18・サブエージェント探索）。着手時に必ず引き直すこと（roadmap「着手手順」）。

## Problem

**誰の何が困っているか**: areka を初めて手にする第三者（ゴーストの利用者）と、その人に配る開発者。

今日の `areka.exe` は「起動するゴーストを argv で 1 体指定する開発用の実行体」であって、アプリではない。

1. **起動するゴーストの置き場という概念が無い。** `crates/areka/src/boot_config.rs:52-65` の `resolve_config_inputs` は `args[1]`＝ゴーストの根・`args[2]`＝バルーンの根を受け取り、無ければ `default_ghost_root()`（`boot_config.rs:32-34`＝`CARGO_MANIFEST_DIR` 相対の `ghost/master`）へ落ちる。この既定は**配布した exe では実在しない**。実在しなくても `warn!` だけで起動を続ける（`crates/areka/src/main.rs:167-177`）。
2. **インストール済みのゴースト・シェル・バルーンを数える手段が無い。** `ghostpathlist`／`installedghostname`／`balloonpathlist` は `crates/**/*.rs` で 0 件。ベースウェア直下の `ghost/`・`balloon/` という 2 つのフォルダを知っているコードは 1 行も無い（`crates/areka-parsers/src/package/resolve.rs:21-29` が知るのは **1 体のゴーストの内側**の `ghost/master`・`shell/` だけ）。
3. **「前回どのゴーストとバルーンを使ったか」を覚える場所が無い。** `areka-sylphya` の永続化は `App | Ghost | Shell | Balloon` の 4 スコープを持ち（`crates/areka-sylphya/src/persist/mod.rs:54-64`）、`App` スコープの根は `crates/areka/src/boot_config.rs:94-103`（`AREKA_PROFILE_DIR`、無ければ `<exe dir>/profile/areka/`）に配線済みだが、**`App` スコープに載る鍵は今日 0 個**。鍵の族は 4 つ（`persist/mod.rs:123-142`＝窓位置・バルーン offset・起動回数・消滅回数）で、全て `Ghost` スコープに落ちる。

第三者にとっての帰結: exe を起動しても何も出ない。ゴーストを置く場所も、置いた後に選ぶ手段も無い。

## Current State

- **ゴースト 1 体の解決**は動いている。`crates/areka-parsers/src/package/resolve.rs:41-118` が `<ghost_root>/ghost/master/descript.txt` を `charset` で復号し、`name`／`sakura.name`／`kero.name`／`shiori`／`shiori.encoding` 等を取り、`shell/<name>` の実在を要求する（`ShellDirMissing`）。これが「1 体分の素性」の読み手であり、列挙はこの読み手を根の直下のフォルダ全てに当てれば作れる。
- **バルーン**は argv の第 2 引数で別の根を指す。ゴーストの `install.txt` の `balloon.directory` や `descript.txt` の `recommended.balloon` は読んでいない。
- **`nar-install`**（単独枠・本仕様の前提）は `.nar` を `install.txt` の意味論で展開し、その展開先を `target/` 配下に作る。展開先の形＝**インストール済み形**は `<根>/ghost/<directory>/`・`<根>/balloon/<balloon.directory>/` であり、これは ukadoc「全体の構成」（`ukadoc:manual_directory`）の「ベースウェアのゴースト格納フォルダ」「バルーン格納フォルダ」そのもの。**nar-install の展開先が、そのまま本仕様の「根」の形になる。**
- 起動時の `warn!` 継続（`main.rs:167-177`）は「無いものは無いと言って止まる」（記憶 areka-log-first-no-silent-failure）に反している。

## Desired Outcome

完了時に次が真になっている。

1. **areka.exe は「根」を 1 つ持つ。** 既定は exe の隣（`<exe dir>/`）。`<根>/ghost/<名>/`・`<根>/balloon/<名>/` が正典の格納フォルダ。開発時は `AREKA_ROOT`（本番 env 変数の命名規約は記憶 areka-runtime-env-naming）で差し替えられ、テストは `nar-install` の共有ヘルパが作る `target/` 配下の根をそのまま使う。
2. **列挙できる。** 根の直下を走査し、ゴースト（`ghost/master/descript.txt` を持つ）・各ゴーストのシェル（`shell/<名>/descript.txt`・`menu,hidden` を尊重）・バルーン（`descript.txt` で `type,balloon`）を、**素性（`name`・`craftman`・`craftmanw`・`id`・`readme`・`thumbnail.png` の有無）付き**で返す。素性の項目は束「配布物の素性」（`doc/ukadoc-coverage/linkage.md` の `[bundle."配布物の素性"]`・22 件）から α に要る分だけを取る（作者名と説明書。`recommended.*` は列挙の並び順に使わない＝α 後）。
3. **前回の選択を覚える。** `App` スコープに `areka.last.ghost`（フォルダ名）・`areka.last.balloon`（フォルダ名）・ゴーストごとの最後のシェル（`Ghost` スコープ）が載る。起動時の解決順は **argv の明示 → 記憶 → 根に 1 体だけ居ればそれ → 0 体なら「ゴーストが無い」画面**（後述の裁定）。
4. **無いときに黙らない。** 根・ゴースト・バルーンが解決できない場合は `error!` と利用者向けのメッセージボックス（Win32 `MessageBoxW`＝新規依存 0）で「どこに何を置けば良いか」を告げて終了する。`warn!` で起動を続ける経路は消す。
5. **バルーンの既定**: ゴーストが `install.txt` で同梱したバルーン → 記憶 → 根に 1 つだけ → 0 なら areka 同梱の既定バルーン（裁定候補・下記）。

## Approach

**選んだ形**: 「根」を表す 1 つの値型（`BasewareRoot`）を `areka-ghost` に置き、列挙・記憶・起動解決をその上の関数として足す。新クレートは作らない。

| 段 | 中身 | 検証 |
|---|---|---|
| ① 根と列挙 | `BasewareRoot { dir }` と `list_ghosts()`／`list_shells(ghost)`／`list_balloons()`。素性は `package/resolve.rs` の既存の descript 読みを流用し、バルーン側は `areka-parsers/src/balloon/` の descript 読みを流用 | 固定の根（`nar-install` の共有ヘルパが展開した emo2＋R_POST_and_KOMAINU＋バルーン 2 つ）で件数と素性を突き合わせる決定論テスト。`menu,hidden` のシェルを 1 つ足した fixture で除外を確かめる |
| ② 記憶 | `App` スコープに `areka.last.ghost`／`areka.last.balloon`、`Ghost` スコープに `areka.last.shell` を足す（`persist/mod.rs:123-142` の族に 1 族追加・TOML の表は `[last]`） | 書いて読み戻す往復＋「無い鍵は None」 |
| ③ 起動解決 | `resolve_config_inputs` を「根＋記憶＋列挙」から解決する形へ置き換え、argv は**上書き**として残す（実機サインオフの手順は絶対パス起動のまま動く） | 4 通り（argv／記憶／1 体／0 体）の決定論テスト・0 体は `error!` と終了コード |

**なぜ `areka-ghost` に置くか**: 列挙は descript の読み手（parsers）と永続化（sylphya）の両方に触る配線であり、`areka-ghost` は「マウント → SHIORI → sink → sylphya」の結線層（`crates/areka-ghost/src/runtime.rs`）。parsers は「外部パーサ依存を入れない・I/O を持たない」方針なので走査は置けない。

**取らない形**: 設定ファイル（`areka.toml`）の新設。記憶は sylphya の `App` スコープで足りる（配線済み・原子的書き込み済み `persist/io.rs:47-66`）。設定ファイルは「設定画面」が要る日に一緒に考える（α 後）。

## Scope

- **In**:
  - `BasewareRoot`（既定＝exe の隣・`AREKA_ROOT` で差し替え）と `ghost/`・`balloon/` の走査
  - ゴースト・シェル・バルーンの列挙（素性＝`name`・`craftman(w)`・`id`・`readme`・`thumbnail.png` 有無・`menu,hidden`）
  - `App`／`Ghost` スコープの「最後に使った」鍵 3 つ
  - 起動解決の置き換え（argv 上書き → 記憶 → 1 体 → 0 体で告知して終了）
  - 「ゴーストが無い」「バルーンが無い」の利用者向け告知（`MessageBoxW`）
  - `nar-install` の共有ヘルパが返す根を `BasewareRoot` として受ける（テストと実機の入口を 1 つにする）
- **Out**:
  - `.nar` の展開（`nar-install`）・投げ込みや対話でのインストール（`ghost-install`）
  - 切り替えの実行（`ghost-shell-balloon-switch`）・メニュー（`popup-menu-minimal`）
  - `recommended.balloon`／`recommended.ghost` による並び替えや案内（α 後）
  - プロパティシステムの `ghostlist`／`balloonlist`／`currentghost.shelllist.*`（`property-catalog-lists`・`currentghost-property-tree` の所有・α 後）。本仕様の列挙関数はその日の**供給源**になる形で置く
  - 複数の根（ポータブル運用と `%APPDATA%` の併用）。α は 1 つ

## Boundary Candidates

- **根の解決**（exe の隣／env／argv の優先順）
- **列挙と素性**（走査＋descript 読み。parsers の既存読み手を再利用）
- **記憶**（sylphya の鍵族 1 つ）
- **起動解決**（4 通りの分岐＝決定論テストの対象）

## Out of Boundary

- インストール・更新・切替・メニュー（それぞれの spec）
- 設定画面・設定ファイル
- 多重ゴースト（根は 1 つ・同時起動は 1 体）

## Upstream / Downstream

- **Upstream**: `areka-P0-nar-install`（展開先の形＝根の形・共有ヘルパ）／`areka-parsers`（`package/resolve.rs`・`balloon/` の descript 読み）／`areka-sylphya`（`App` スコープ・`FsPersistIo`）／完了仕様 `areka-P0-charset-canon`（descript の charset 復号）。
- **Downstream**: `ghost-shell-balloon-switch`（列挙から切替先を選ぶ）・`popup-menu-minimal`（列挙をサブメニューに出す）・`ghost-install`（インストール先＝根）・`network-update`（更新対象のフォルダ＝根の下）・`alpha-release-signoff`（配布 zip の形＝根の形）。α 後の `property-catalog-lists` は本仕様の列挙関数を供給源にする。

## Existing Spec Touchpoints

- **Extends**: なし（新規）。
- **Adjacent**:
  - `areka-P0-nar-install`（単独枠・**先に着地**）——展開先の形を本仕様の根の形と一致させる。nar-install の brief に「展開先はベースウェア根の形」を追記済み（2026-09-18）。
  - `areka-P0-property-catalog-lists`（α 後）——`ghostlist`／`balloonlist` の値の供給源が本仕様の列挙になる。所有は動かさない。
  - `areka-P0-status-execution-states`（α 後・`emo2_boot/mod.rs`）——起動解決を触る本仕様と同じファイルに触る可能性。α 後なので順序で解決。

## Constraints

- 新規の外部依存 0（`MessageBoxW` は `windows` crate の既存機能 `Win32_UI_WindowsAndMessaging`）。
- 1 ファイル 1,000 行の番人・例外表に触れない。`crates/areka/src/main.rs`・`boot_config.rs` の現在行数を着手時に測ってから足す。
- 決定論テスト網羅は必達。走査は固定の根で、記憶は一時ディレクトリ（`temp-path-kit`）で、起動解決は 4 分岐すべてを踏ませる。
- ログ無し失敗経路の禁止＝根が解決できない経路は `error!`＋利用者向け告知＋非 0 終了。
- **裁定候補 ⑴（開発者）**: 既定バルーンを areka に同梱するか。第三者のゴーストの多くはバルーンを同梱しない（ukadoc「インストール」は同梱を任意としている）ので、α 配布物に**作者自作の `emo2-kakukaku`（現在 `nar-install` が `.nar` 化する予定のバルーン 2 つの元）を既定バルーンとして同梱する**案を推す。同梱しないなら「バルーンが無い」告知で止まる（利用者はバルーン `.nar` を先に入れる）。
- **裁定候補 ⑵（開発者）**: 根の既定を exe の隣（ポータブル）にするか `%APPDATA%\areka\` にするか。α は exe の隣を推す（zip を展開して起動＝SSP と同じ体験・書き込み権限の問題が起きない場所へ利用者が置く）。
