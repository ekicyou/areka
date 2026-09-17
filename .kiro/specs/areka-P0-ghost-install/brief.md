# Brief: areka-P0-ghost-install

> 2026-09-18 `/kiro-discovery` 再入（棚卸⑭＝α ゴールへの組み直し）で起票。`doc/ukadoc-coverage/roadmap-draft.md` 段階 B 順位 1 の束「インストール」の**製品側**（利用者が `.nar` を渡す体験）と、順位 4 の束「投げ込み」（`OnFileDrop2` 等）のうち窓へ落とす経路を引き受ける。エンジン（コンテナ読取・`install.txt` 解釈・安全な展開）は `areka-P0-nar-install` が持つ。
> 本文の file:line は**起票時の実測値**（2026-09-18）。着手時に必ず引き直すこと。

## Problem

**誰の何が困っているか**: 配布サイトから `.nar` を落としてきた第三者。

正典（`ukadoc:manual_install`）: 「install.txtが適切に用意されていれば、D&Dなどの手段でインストーラ機能が働き、自動的にインストールできる」。今日の areka には D&D も、ファイル選択も、`\![execute,install,path,…]` も無い。`WM_DROPFILES`／`DragQueryFile`／`IDropTarget`／`RegisterDragDrop` は `crates/` に 0 件（`wintf/src/ecs/drag/` はマウスによる窓の移動であってシェルの D&D ではない）。インストール系イベント（`OnInstallBegin`／`OnInstallComplete(Ex)`／`OnInstallCompleteAll`／`OnInstallFailure`／`OnInstallRefuse`）も 0 件。

`nar-install` が着地しても、それは**開発者が `target/` に検体を展開する道具**であり、利用者が `.nar` を渡す入口は別に要る。

## Current State

- **エンジン**: `areka-P0-nar-install`（単独枠・本仕様の前提）が `areka-nar` クレートを建てる——zip 読取・`install.txt`（`type`／`name`／`directory`／`accept`／`charset`／`refresh`／`refreshundeletemask`／`*.directory`／`*.source.directory`）・ファイル名の文字コード・パス安全性・原子的確定。展開先の形は `<根>/ghost/<directory>/`・`<根>/balloon/<balloon.directory>/`（`baseware-root-layout` の根の形）。
- **根**: `baseware-root-layout` が「インストール先」を与える。
- **窓**: wintf の Win32 窓。`Win32_UI_Shell` は有効（ルート `Cargo.toml`）だが `Shell::` の呼び出しは 0 件。`DragAcceptFiles`＋`WM_DROPFILES` は `Win32_UI_Shell` に居る＝依存追加 0。
- **既存の「投げ込み」**: 無し。`OnFileDrop2`（Ref0＝パス・byte 1 区切り・Ref1＝スコープ・Ref2＝MIME）が現行仕様（`ukadoc:list_shiori_event:OnFileDrop2:1`「このイベントが現時点での最新仕様となる」）。

## Desired Outcome

完了時に次が真になっている。

1. **キャラクター窓へ `.nar`（または `.zip`）を落とすとインストールされる。** 手順: `OnInstallBegin` → `areka-nar` で展開（`accept` が別のゴーストを名指ししていれば `OnInstallRefuse` で止める）→ 成功なら `OnInstallCompleteEx`（無応答なら `OnInstallComplete`）（Ref0＝識別子 `ghost`／`shell`／`balloon`／…・Ref1＝`install.txt` の `name`・Ref2＝同梱バルーンの名前）→ 複数なら最後に `OnInstallCompleteAll`。失敗は `OnInstallFailure`（Ref0＝失敗理由）。
2. **メニュー「インストール…」からファイル選択で同じことができる**（`GetOpenFileNameW`＝`Win32_UI_Controls_Dialogs`・依存追加 0）。
3. **台本 `\![execute,install,path,フルパス]` でも同じことができる**（相対パス不可・正典どおり）。
4. **`terms.txt`（`terms.md`）があればインストール前に表示し、受諾で `OnGhostTermsAccept`・拒否で `OnGhostTermsDecline`。** 表示は `MessageBoxW`（OK＝受諾／キャンセル＝拒否）。Markdown の装飾は解釈しない（本文のまま出す）。
5. **インストール直後にそのゴーストへ切り替える**（`ghost-shell-balloon-switch` の `lastinstalled`）。バルーンだけを入れたときは切り替えない（記憶だけ更新）。シェルだけを入れたときはそのゴーストが起動中ならシェル一覧が増える。
6. **`.nar` 以外のファイルを落としたら `OnFileDrop2`** として SHIORI へ渡す（束「投げ込み」の最小＝ファイルとフォルダ。URL・テキストは α 後）。
7. **何も黙って消えない。** 展開の失敗・`accept` 不一致・`install.txt` 不在は `error!`／`warn!` と利用者向け告知の両方。

## Approach

**選んだ形**: 3 つの入口（D&D・ファイル選択・台本）を 1 本の `InstallRequest { path }` に畳み、`areka-nar` を呼ぶ 1 つの手続きへ流す。

| 段 | 中身 | 検証 |
|---|---|---|
| ① 手続き | `install(root, path) -> InstallOutcome`（`areka-nar` の展開＋`accept` 判定＋結果の型）。イベント列は kanade に「インストール相」を足して送る | 決定論: 固定 `.nar` 4 種（ghost・ghost with balloon・balloon・accept 付き shell）で `InstallOutcome` とイベントの Ref を突き合わせる |
| ② 入口 A 台本 | `\![execute,install,path,…]` の消費者（汎用キャリアの name 選別） | 決定論: 台本 → `InstallRequest` |
| ③ 入口 B メニュー | `popup-menu-minimal` の `MenuRegistry` に「インストール…」を登記・`GetOpenFileNameW` | 実窓 1 本 |
| ④ 入口 C D&D | `DragAcceptFiles`＋`WM_DROPFILES`（wintf の窓手続きに 1 分岐）→ 拡張子で `.nar`/`.zip` なら `InstallRequest`、それ以外は `OnFileDrop2` | 決定論: `WM_DROPFILES` の偽メッセージ → 分岐。実窓: 実際に落とす |
| ⑤ 利用条件 | `terms.txt` の検出と `MessageBoxW`・受諾／拒否イベント | 決定論: あり／なし／拒否 |

`WM_DROPFILES` を選ぶ理由: `IDropTarget`（OLE）は COM の初期化（`OleInitialize`＝STA）を要求し、wintf の WUC は MTA で動く（記憶 areka-wuc-runs-on-mta-thread）。`DragAcceptFiles` は COM を要求しない。テキストや URL の投げ込み（`OnTextDrop`／`OnURLDropping`）には `IDropTarget` が要るので、それは α 後にする。

## Scope

- **In**:
  - `InstallRequest` と手続き 1 本（`areka-nar` の呼び出し・`accept` 判定・結果）
  - 入口 3 つ（D&D＝`WM_DROPFILES`・ファイル選択・`\![execute,install,path,…]`）
  - イベント 7 種（`OnInstallBegin`／`OnInstallComplete`／`OnInstallCompleteEx`／`OnInstallCompleteAll`／`OnInstallFailure`／`OnInstallRefuse`／`OnInstallReroute` は多重ゴースト前提なので**送らない**＝`OnInstallRefuse` に落とす）
  - `terms.txt`／`terms.md` の表示と `OnGhostTermsAccept`／`OnGhostTermsDecline`
  - インストール直後の切替（`lastinstalled`）
  - `.nar` 以外の `OnFileDrop2`（ファイル・フォルダ）・`OnFileDropping`（ドラッグ中＝`WM_DROPFILES` では取れないので**送らない**・α 後）
  - 環境変数の置換語 `%lastghostname`／`%lastobjectname`（束「インストール」の 2 件・切替の名前解決と同じ値）
- **Out**:
  - `.nar` の読取と展開そのもの（`nar-install`）
  - `\![execute,install,url,…]`・URL の投げ込み（HTTP が要る＝`network-update`）
  - `readme.txt` の表示（インストール時に開く慣行はあるが α はメニュー「説明書」で足りる）
  - `type,package`／`bootghost`／`plugin`／`headline`／`calendar*`／`language`（`areka-nar` が受理する type は `ghost`・`shell`・`supplement`・`balloon` の 4 つ。他は `OnInstallFailure` 理由 `unsupported type`）
  - `OnTextDrop`／`OnURLDrag*`／`OnOtherObjectDrop*`（`IDropTarget` が要る・α 後）
  - `installedghostname` 等のリソース（PLUGIN 側・α 後）

## Boundary Candidates

- **手続き**（`areka-nar` の上の 1 関数と結果型）
- **入口 3 つ**（それぞれ 1 分岐で `InstallRequest` を作る）
- **イベント相**（kanade の schedule に「インストール」の相）
- **投げ込みの振り分け**（拡張子で install か `OnFileDrop2` か）

## Out of Boundary

- 展開の意味論と安全性（`nar-install`）
- 根の場所（`baseware-root-layout`）
- 切替の実行（`ghost-shell-balloon-switch`）

## Upstream / Downstream

- **Upstream**: `areka-P0-nar-install`（`areka-nar`）／`areka-P0-baseware-root-layout`（根）／`areka-P0-ghost-shell-balloon-switch`（`lastinstalled`）／`areka-P0-popup-menu-minimal`（`MenuRegistry`）／完了仕様 `areka-P0-charset-canon`（`terms.txt` の `charset,` 行）。
- **Downstream**: `network-update`（`\![execute,install,url,…]` は本仕様の `InstallRequest` に「落としてきたファイル」を渡すだけ）・`alpha-release-signoff`（第三者の手順「`.nar` を落とす」）。

## Existing Spec Touchpoints

- **Extends**: なし（新規）。
- **Adjacent**:
  - `areka-P0-nar-install` の Out「利用者が投げた `.nar` を受け取る UI／D&D／インストーラ体験。M2 予約群の範囲」——**その範囲が本仕様**。nar-install の brief に追記済み（2026-09-18）。
  - `areka-P0-translate-pipeline`／`sakura-time-directives`（α 後・kanade の schedule）——本仕様が相を 1 つ足す。後着が rebase。

## Constraints

- 新規の外部依存 0（`Win32_UI_Shell`・`Win32_UI_Controls_Dialogs` は `windows` crate の機能フラグ。後者が未有効なら機能フラグを 1 行足す＝crate の追加ではない）。
- `WM_DROPFILES` は落とされた側の窓の手続きで受ける。窓は UI スレッド固定（記憶 areka-concurrency-model）。展開は時間が掛かるので UI スレッドで行わない（背景プール `WintfTaskPool` へ）。
- パスの安全性は `areka-nar` が持つ。本仕様は **`.nar` の中身を信用しない**前提を崩さない（`accept` の判定も展開前に `install.txt` だけ読んで行う）。
- 決定論テスト網羅は必達。固定 `.nar` 4 種は `nar-install` の fixture を再利用し、増やすなら同じ場所へ。
- 1 ファイル 1,000 行。
- 規模 **M**。
