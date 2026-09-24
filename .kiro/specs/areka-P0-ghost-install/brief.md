# Brief: areka-P0-ghost-install

> 2026-09-18 `/kiro-discovery` 再入（棚卸⑭＝α ゴールへの組み直し）で起票。`doc/ukadoc-coverage/roadmap-draft.md` 段階 B 順位 1 の束「インストール」の**製品側**（利用者が `.nar` を渡す体験）と、順位 4 の束「投げ込み」（`OnFileDrop2` 等）のうち窓へ落とす経路を引き受ける。エンジン（コンテナ読取・`install.txt` 解釈・安全な展開）は `areka-P0-nar-install` が持つ。
> 本文の file:line は**起票時の実測値**（2026-09-18）。着手時に必ず引き直すこと。

## 2026-09-20 棚卸⑮の再測定

**実測の追記（main `fe157df1`）**

- **名前の衝突。** 本文の `InstallRequest { path }`／`InstallOutcome` は、`areka_nar::{InstallRequest, InstallOutcome}`（`crates/areka-nar/src/lib.rs`・`install.rs`）と同名で意味が違う。要件段階で改名する。
- 「`Shell::` の呼び出しは 0 件」は、いま 1 件（`crates/areka/src/readme.rs` の `ShellExecuteW`）。
- ファイル選択に要る `Win32_UI_Controls_Dialogs` は未有効（根の `Cargo.toml` に在るのは `Win32_UI_Controls` のみ）＝機能を 1 行足す。
- `WM_DROPFILES` の受け口は `crates/wintf/src/ecs/window_proc/mod.rs` の振り分けの表に 1 分岐足す形。表に `WM_DROPFILES` は 0 行。
- **申し送り 4 件のうち 2〜4 番（長さの上限・巻き戻せなかったときの元の木の在りか・`work` の欄の検査）は `areka-P0-nar-install-hardening`（台帳 #52）へ移した。** 本仕様に残るのは 1 番（網羅台帳 `descript_install` の 11 行を実装済みへ動かす）と、`crates/areka-nar/src` への正典 URL のコメント行の追記。
  - **2026-09-23 `nar-install-hardening` の要件ディスカッションからの申し送り**: 部品の側は「巻き戻せなかった宛先ごとの元の木の在りかを失敗の値に載せる」「元の木を含む作業フォルダは失敗から 7 日のあいだ片付けで消さず、期限を過ぎたら次の展開の開始時に消す」まで（同 spec 要件 2）。**利用者にどう見せるかは本仕様で決める**: 在りかを告知に載せるか・「7 日後に自動で消える」と伝えるか・元の木を連番付きの別ゴースト（例 `ghost/<名前>-1`）として救い出す形を採るか（開発者の 09-23 の問いかけ。部品が 7 日残すので後から選べる）。なお同じ 7 日の規則は、**成功した展開の後片付けに失敗して旧木 `old-<k>/` が残った作業フォルダ**にも同じく掛かる（部品は両者を区別しない・同 spec 設計「棚の片付けの判定」）。告知の文面はこの場合も含めて決める。
- 切替の相手は 2 本に分かれた: ゴーストの `.nar` を入れた直後の切替は `areka-P0-ghost-shell-balloon-switch`、シェル・バルーンの `.nar` は `areka-P0-shell-balloon-switch`。**本仕様は両方の着地を待つ。**
- 分割後の想定タスクは 17〜20 本（上限の内側）。台本の入口は `consumer_ledger.rs` と `emo2_boot/mod.rs` の 4 点・kanade の 5 ファイル・網羅台帳を触る＝切替の 2 本と `network-update` とは必ず直列。

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

- **2026-09-18 `nar-install` 設計からの申し送り（使用中の宛先）**: `areka-nar` の展開は「作業フォルダに組んでから宛先と入れ替える」形で、宛先の中のファイル（起動中のゴーストの `shiori.dll` 等）が開かれていると入れ替えが失敗し、宛先は無傷のまま `NarError::Io { phase: Commit, rolled_back: true }` が返る。**エンジンは SHIORI の解放を試みない**。起動中のゴーストへ入れる・更新する・切り替える経路は、呼び出し側が先に SHIORI をアンロード（`OnClose` 相当の終了経路）してから `install` を呼ぶこと。

- **2026-09-19 `nar-install` 実装完了からの申し送り（4 件・使用中の宛先）**:
  1. **網羅台帳 `descript_install` の 11 行を「実装済み」へ動かすのは本仕様である。** `doc/ukadoc-coverage/ledger/assets.toml` の 11 項目は引受先の欄に `areka-P0-nar-install` を書いたまま、状態は `absent` で止めてある（`*.directory`／`*.refresh`／`*.refreshundeletemask`／`*.source.directory`／`accept`／`charset`／`directory`／`name`／`refresh`／`refreshundeletemask`／`type` の 11 行）。台帳が定める「実装済み」は 2 つを同時に満たすことで、いま満たしているのは 0 である——⑴ 定義箇所に正典 URL の 1 行（`// ukadoc:`）があること: `crates/areka-nar/src/` にこの綴りは **0 行**（同じ綴りは他クレートに 194 行あるので、数え方は当たる）。⑵ areka が正典どおりに動くこと: `areka-nar` を依存に持つのは試験専用の `sample-ghost-kit` 1 つだけで、`crates/areka/Cargo.toml` の依存一覧に `areka-nar` は無い＝**本体から辿れない**。本仕様が本体へ繋ぐ配線を入れた**そのコミットで**、URL の 1 行を置き、11 行を `implemented` へ動かし、`cargo run -p ukadoc-survey -- report` と `-- report-summary` の 2 本を作り直すこと。
  2. **`.nar` の中の 1 要素あたりの名前・パスの長さに上限が無い。** 本 brief の制約「パスの安全性は `areka-nar` が持つ」は正しく、否定しない——`areka-nar` は名前の検査（絶対パス・親への上り・区切りの正規化）を持っている。決まっていないのは**長さ**だけである。総量には上限があり（`areka-nar` の `container` の `MAX_TOTAL_DECLARED_SIZE` ＝ 1 GiB）、**1 要素あたりの長さだけが決まっていない**ので、30 万文字の名前を持つ要素がそのまま受理される。`nar-install` 側は自ら「実装の欠陥ではなく設計の穴」と書いており（同仕様の `tasks.md` 実装メモ 3.2・`validation-report.md` の引受先の節）、直すには「どこまでの長さを受け入れるか」を誰かが決めるしかない。**利用者が投げ込む `.nar` を受けるのは本仕様なので、受け入れる長さを決めるのは受け口の側の仕様である**——値を決めて `areka-nar` に渡す（または入口で撥ねる）形を要件で置くこと。中身を信用しない前提はそのままで、信用しない相手に「どこで線を引くか」を足す話である。
  3. **確定に失敗したときの表示に、作業フォルダの場所を載せるかが未決。** `areka-nar` の失敗の型（`NarError`）は巻き戻せたかどうか（`rolled_back`）と詰まった宛先のパスは持つが、**利用者の元の木が `<根>/.nar-work/<プロセス識別子>-<連番>/old-<k>/` に生き残っていることを知る手段を持たない**（場所は失敗の記録の `work` の欄にだけ出る）。巻き戻しに失敗したとき、利用者は「どこで詰まったか」は分かるが「自分の元のゴーストがどこにあるか」は分からない。失敗の告知（`OnInstallFailure` の理由と利用者向けの表示）を作るのは本仕様なので、載せるか載せないかをここで決めること。
  4. **失敗の記録の `work` の欄は、確定の段の失敗では一度も検査されていない。** `areka-nar` の全数対応のテストは 13 の固定入力を持つが、13 件はすべて作業フォルダを掘る前（`WorkArea::create` の手前）で拒否されるため、そのテストが `work` について主張しているのは「**空であること**」だけである（`lib_vocabulary_tests.rs` の全数対応のテスト）。確定の段で失敗したときに `work` が実際の場所を持つことは、まだどのテストも見ていない。本仕様が「起動中のゴーストへ入れて確定に失敗する」経路を `NarArchive::install` 経由で踏むテストを書くとき、そこで併せて埋めること。
