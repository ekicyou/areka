# ギャップ分析: areka-P0-winget-manifest-submission

- 作成: 2026-10-10（ブランチ `claude/areka-p0-winget-manifest-1c4fd7`・コミット `be7b624f`）
- 対象: 確定済みの `requirements.md`（要件 1〜6）と、今のリポジトリ・公開済みの Release・winget の今の版との差
- 進め方: リポジトリは Grep／Glob／Read、GitHub は `gh` の読むだけの問い合わせ、winget は `winget --version` と、開発機に入っている版と同じタグ（`v1.29.380`）の winget-cli のソースの読み取り。`winget install`・`winget settings` は実行していない。
- この文書は材料と選択肢を並べるもので、決めは要件討議と設計で行う。

## 1. まとめ

- **作る物は小さい**。リポジトリに足すのは `dist/winget/` の YAML 3 ファイルと、本仕様のフォルダの中の記録・手順書、steering の `structure.md` の 1 か所、後ろの spec の brief への書き足しだけ。コードは 1 行も要らない。マニフェストに書く値（URL・ハッシュ・公開日・ライセンス）は、すべて今ある物から引ける。
- **伸ばせる型が在る**。実機の確かめの手順と記録の形は、完了 `areka-P0-release-package-versioned` の `verification/winget-local-check.md` がそのまま土台になる（設定を変えた時刻の表・手順と結果・置き場・既知の制限）。起動の判定に使う記録の行（`root_resolved`・`本物のゴースト窓を開きました`）も今のコードに在る。
- **いちばん大きい発見**: winget-cli のソース（`v1.29.380`）を読むかぎり、zip の最上位にあるフォルダ `ghost`・`balloon` は、**上げ直しでも、`--purge` を付けない外し方でも、中身ごと消される**見込み（下の 3.1）。利用者が後から入れたゴースト・バルーンと、ゴーストとシェルの記憶はこの 2 つのフォルダの中に在る。残る見込みなのは areka の記憶（`profile\areka\`）だけ。brief の「アンインストールで消えるのは winget が置いたファイルだけ（利用者のゴーストは残る）」は、この読みと合わない。これはソースの読みで、実機ではまだ測っていない（要件 4 が測る）。
- **その発見が議題 3・議題 5 に効く**。消すかどうかは「入っている古い版の記録」が決めるので、`v0.0.2` を初回に出すと、`v0.0.2` から次の版への上げ直しでは、後の版でどう直しても利用者のゴーストが消える（下の 3.1 の最後）。
- **外の決まりで未確定の物が残る**: マニフェストの書式の版（winget-pkgs の文書に 1.28.0 が増えている）・提出の道具（`komac` に手元のマニフェストを出すコマンドが在るか）・既定のロケールの決まり・初回の提出者の同意の手続き。どれも設計で引き直せる大きさ。

## 2. 今の状態（確かめた事実）

### 2.1 リポジトリ

| 見た物 | 結果 |
|---|---|
| `dist/` で追跡している物 | `dist/README.txt` の 1 つだけ。`dist/winget/` は無い |
| `.github/workflows/` | `release.yml`・`crates-io.yml` の 2 本。`winget.yml` は無い |
| `crates/`・`tools/`・`README.md`・`dist/README.txt` の中の「winget」の語 | 0 件（winget を知っているコードも道具も無い） |
| 版の正本（根の `Cargo.toml` の `[workspace.package]`） | `version = "0.0.2"`・`license = "MIT"`・`repository = "https://github.com/ekicyou/areka"`。根に `LICENSE-MIT` が在る |
| `tools/package.ps1` の中身の検査 | zip の最上位に許す名前は `areka.exe`・`shiori-host32-helper.exe`・`ghost`・`balloon`・`README.txt`・`LICENSE-MIT`・`THIRD-PARTY-NOTICES.md`・`BUILD-INFO.txt` の 8 つ（ファイル 6・フォルダ 2）。`ghost/` の直下は `emo2` だけ、`balloon/` の直下は `emo2-kakukaku`・`StayseeBalloon` だけ。`profile/` を含む項目が在れば否 |
| `dist/` から zip へ写す物 | `dist/README.txt` の 1 つだけ（`Copy-Item -LiteralPath 'dist/README.txt'`）。`dist/winget/` を足しても zip は変わらない |
| `tools/package.ps1` の「git status 不変の確認」 | 始めと終わりの `git status --porcelain` を比べるだけ。`dist/winget/` が追跡されていても、走っている間に変わらなければ通る |
| 起動した exe のリンクを解く仕組み | `crates/areka/src/boot_config.rs` の `follow_exe_links`（判断）・`exe_location`（1 度だけ解いて覚える）。`resolve_root` がこれを通る |
| 起動の記録の行 | `boot_config.rs` の `resolve_boot_from` が `event = "root_resolved"` を `info` で出す（`root=`・`source=`）。`crates/areka/src/ghost_session.rs` が `本物のゴースト窓を開きました…` を出す。`tools/package.ps1` の `LOG_MARKER_WINDOWS` も同じ文言を使っている |
| 前回のゴーストが無いときの起動 | `crates/areka/src/boot_resolve.rs` は、記憶のゴーストが `ghost\` に無ければ `last_ghost_not_found` を `warn` で出して次の候補へ進む（1 体だけならそれ、複数なら既定の `emo2`） |
| 検体の `.nar` | `vendors/sample_ghost/` に `claudia.nar`・`emily4.nar`・`konnoyayame.nar`・`R_POST_and_KOMAINU.nar` などが在る（要件 4.1 の「後から入れたゴースト」に使える） |
| steering `structure.md` の `dist/` の説明 | 「その他の最上位」の行の末尾に「`dist/`＝配布物へそのまま入れる文書（第三者向け `README.txt`）」と在る。要件 2.7 が直すのはここ |

### 2.2 公開済みの Release（`gh release view v0.0.2`）

- Release は `v0.0.2` の 1 件だけ。公開は `2026-10-06T12:49:21Z`（日本時間でも同じ 10 月 6 日＝`ReleaseDate` は `2026-10-06` で迷わない）。下書きでも先行版でもない。
- 置いてある物と、GitHub が示す zip そのもののハッシュ:

| ファイル | 大きさ | GitHub が示す SHA256 |
|---|---|---|
| `areka-0.0.2-x64.zip` | 8,695,089 | `66d3a9a01a611dd36b9c3af276c0cd071ef87c28ba82b6cf8fc314fa6a751c9c` |
| `areka-0.0.2-arm64.zip` | 8,552,648 | `77c5356a8f77cb121f828cfe761d7c63588c822790fb80be78234f0f9fdd79fb` |

- URL は `https://github.com/ekicyou/areka/releases/download/v0.0.2/areka-0.0.2-{arch}.zip`。
- `.sha256` の 2 ファイルも在る（中身は今回は取ってきていない）。要件 1.5 は「`.sha256` に書かれた値と同じ」を求めるので、設計では `.sha256` の中身・GitHub が示すハッシュ・取ってきた zip のハッシュの 3 つを突き合わせる形が取れる。`tools/package.ps1` は `.sha256` を小文字で書く。winget の道具は大文字で書くのが通例（2026-10-03 の実測も大文字にして通している）。

### 2.3 開発機と winget

- winget は `v1.29.380`（winget-cli の最新の Release も同じ `v1.29.380`・2026-09-21）。
- `komac`・`wingetcreate` はどちらも開発機に入っていない（入れるのは開発者の手）。`komac` の最新は `v2.16.0`（2026-03-29）、`wingetcreate` の最新は `v1.12.13.0`（2026-07-23）。
- 開発者のアカウントに winget-pkgs のフォークはまだ無い（`gh repo view ekicyou/winget-pkgs` は「見つからない」）。
- winget-pkgs に `manifests/a/Areka/` は無い。`Areka.Areka` を含む PR も見つからない＝名乗りはぶつかっていない。置き場の並びは `manifests/a/Areka/Areka/Portable/0.0.2/` になる（名乗りの区切りごとにフォルダ）。
- 前の確かめの残りは無い（`%LOCALAPPDATA%\Microsoft\WinGet\Packages\Areka.Areka.Portable__DefaultSource` は無い）。
- 開発機の利用者の PATH には、別の道具のフォルダ `…\WinGet\Packages\rhysd.actionlint_Microsoft.Winget.Source_8wekyb3d8bbwe` が載っている＝winget-pkgs から入れた portable の入れ先の名前は `<名乗り>_Microsoft.Winget.Source_8wekyb3d8bbwe` の形で、手元のマニフェストの `<名乗り>__DefaultSource` より 23 字長い。

### 2.4 winget-pkgs の文書（2026-10-10）

- `doc/manifest/schema/` に `1.12.0` と **`1.28.0`** が在る。直近に取り込まれた PR 2 件（どちらも道具が出した物・うち 1 件は zip＋portable）は `ManifestVersion: 1.12.0` を使っている。
- `1.12.0` の installer の説明: `ArchiveBinariesDependOnPath` は「`true` で入れ先を PATH に直接足す。付けない・`false` は既定の動き」。`NestedInstallerFiles` は portable のときだけ複数書ける。`UpgradeBehavior` は `install`・`uninstallPrevious`・`deny`。`ReleaseDate` は `YYYY-MM-DD`。
- `doc/Authoring.md`: 既定のロケールを en-US にせよという決まりは見当たらない。名乗りは「ふつう `Publisher.Package`、区切りを足してよい」。手で書いた YAML は `winget validate --manifest` を通す。手元で入れる確かめは `LocalManifestFiles` をオンにして `winget install --manifest`。より隔離した確かめに Windows サンドボックスのスクリプト（`SandboxTest.ps1`）が在る。

## 3. winget の振る舞い（ソースの読み・開発機と同じタグ `v1.29.380`）

読んだのは `src/AppInstallerCLICore/Workflows/PortableFlow.cpp` と `src/AppInstallerCLICore/PortableInstaller.cpp`。以下はソースの読みで、実機の結果ではない。要件 4 の実測が正本になる。

### 3.1 上げ直しと外し方で何が消えるか

1. **zip から入れるとき、winget が覚えるのは zip の最上位の項目だけ**。`PortableFlow.cpp` は展開したフォルダを `std::filesystem::directory_iterator`（1 段だけ）で回し、フォルダなら `CreateDirectoryEntry`、ファイルなら `CreateFileEntry` を作る。areka の zip では、ファイル 6 つ（`areka.exe`・`shiori-host32-helper.exe`・`README.txt`・`LICENSE-MIT`・`THIRD-PARTY-NOTICES.md`・`BUILD-INFO.txt`）と、フォルダ 2 つ（`ghost`・`balloon`）になる。
2. **覚えたフォルダは、消すときに中身ごと消す**。`PortableInstaller::RemoveFile` は、フォルダの項目を `std::filesystem::remove_all(filePath)` で消す。空かどうかは見ない。
3. **上げ直しも外し方も、覚えた項目を全部消す所から始まる**。`PortableInstaller::ApplyDesiredState` は、入れ先に古い版の記録が在れば、その項目すべてに `RemoveFile` を呼び、その後で新しい版の項目を入れる。
4. **入れ先のフォルダそのものは、空でなければ残す**。`RemoveInstallDirectory` は、`--purge` のときだけ入れ先を丸ごと消し、付けないときは空なら消し、空でなければ「ファイルが残っている」という文を出して残す。
5. **書き換えの検査はファイルの項目だけ**。`VerifyPortableFile` は、ファイルの項目の SHA256 とリンクの先だけを見る。フォルダの項目は見ない。合わないと、`--force` を付けないかぎり、上げ直しも外し方も止まる（`PortableHashMismatchOverrideRequired`）。

これを areka に当てはめた見込み（要件 4.2・4.3 の項目ごと）:

| 項目 | 置き場 | 上げ直し | `--purge` なしの外し方 |
|---|---|---|---|
| 利用者が後から入れたゴースト | `ghost\<ゴースト>\` | 消える見込み | 消える見込み |
| 利用者が後から入れたバルーン（要件には無い） | `balloon\<バルーン>\` | 消える見込み | 消える見込み |
| ゴーストの記憶・シェルの記憶（同梱の えも？？ の分も） | `ghost\<ゴースト>\…\profile\areka\` | 消える見込み | 消える見込み |
| areka の記憶 | `profile\areka\`（最上位・winget は覚えていない） | 残る見込み | 残る見込み（入れ先のフォルダも残る） |
| 同梱のファイル | 最上位の 6 ファイルと `ghost\emo2`・`balloon\…` | 置き換わる見込み | 消える |
| 上げ直しの後に、前のゴーストで立つか | − | 立たない見込み（areka の記憶は残るが、指す先のゴーストが無い → `last_ghost_not_found` を出して えも？？ で立つ） | − |

- `.nar` を入れる途中で残る作業用フォルダ（`.nar-work`）も最上位なので、残る側に入る見込み。
- 書き換えの検査に掛かるのは最上位の 6 ファイルだけ。areka がこれらを書き換える経路は探していない（ふつうは書き換えない）。ゴーストのネットワーク更新が書き換えるのは `ghost\` の下なので、検査には掛からない見込み。
- **後の版では救えない**。上の 3 のとおり、何を消すかを決めるのは「いま入っている版が覚えた項目」。`v0.0.2` を winget で入れた人が次の版へ上げるときは、次の版の zip の形をどう変えても、`v0.0.2` が覚えた `ghost`・`balloon` が中身ごと消される見込み。公開済みの `v0.0.2` の zip の中の説明書に、注意を書き足すこともできない。
- 利用者の winget の設定に「外すときに portable を丸ごと消す」（`UninstallPurgePortablePackage`）が在ると、何も付けない外し方でも `--purge` と同じになる（上げ直しの中の外しには効かない）。実測の記録には、この設定が既定のままであることを書く必要がある。

### 3.2 `ArchiveBinariesDependOnPath` を付けたときの入り方

- `PortableInstaller::InstallFile` は、この欄が真のとき、リンクを作らずに入れ先のフォルダを PATH に足す（`AddToPathVariable(installDirectory)`）。「コマンド ライン エイリアスが追加されました」の文は、リンクを作らなくても出る。PATH を足したときは、端末を開き直すよう促す文も出る。
- 付けないときは、リンクを作ろうとして、作れなければ同じく入れ先のフォルダを PATH に足す。2026-10-03 の実測（開発者モードがオフだと PATH になる）と合う。
- 外すときは、PATH に足した項目を外す経路が在る（`RemoveFromPathVariable`）。要件 3.6 の「PATH に足された項目が残っていない」は、この経路の確かめになる。
- 付ける側で増えること: 入れ先のフォルダが丸ごと PATH に載るので、`areka.exe` のほかに `shiori-host32-helper.exe` も名前だけで呼べるようになる。
- 3.1 の消える・残るは、この欄を付けても付けなくても変わらない。

### 3.3 そのほか

- **入れ先の名前**: `GetPortableProductCode` は `<名乗り>_<取り寄せ元の名前>` を作る。手元のマニフェストでは取り寄せ元が `*DefaultSource` になる（2026-10-03 の実測の `Areka.Areka.Portable__DefaultSource` と合う）。上げ直しのとき、名乗りか取り寄せ元が入っている物と違うと、`--force` なしでは止まる（`PortablePackageAlreadyExists`）＝上げ直しの実測は、入れたときと同じく手元のマニフェストで行う必要がある。
- **入れる範囲（利用者ごと／機械ごと）はマニフェストで縛れない**。`ManifestCommon.cpp` の `DoesInstallerTypeIgnoreScopeFromManifest` は portable を「マニフェストの範囲を見ない種類」に入れている。利用者が `--scope machine` を付けると、入れ先は Program Files の側になり、areka は exe の隣に記憶を書けない見込み（測っていない）。

## 4. 要件と、今ある物の対応

| 要件 | 要る物 | 今ある物 | 差 |
|---|---|---|---|
| 1.1〜1.5・1.7 マニフェストの中身 | 名乗り・表示名・発行者・URL・zip と portable の欄・2 つの CPU 種別・ハッシュ・MIT・公開日・タグ | 値はすべて引ける（2.1・2.2） | **無い**: YAML 3 ファイル。**未決**: 書式の版（1.12.0 か 1.28.0）・要件に名前の無い欄（`Moniker`・`LicenseUrl`・`MinimumOSVersion`・`UpgradeBehavior`・`ReleaseNotesUrl`）を入れるか |
| 1.6 `ArchiveBinariesDependOnPath` | 欄 1 つ | 振る舞いはソースで読めた（3.2） | 議題 1 のまま。新しい材料は 3.2 |
| 1.8 ロケール 1 つ | 説明文の言語 | en-US を求める決まりは見当たらない（2.4） | 議題 4 のまま |
| 2.1 置き場 | `dist/winget/` の下の並び | 無い | **未決**: 並び（5.2） |
| 2.2 `winget validate` | 開発機の winget v1.29.380 | 在る。読むだけのコマンドで、設定の変更は要らない | 手順と記録だけ |
| 2.4 zip が変わらない | − | `tools/package.ps1` が写すのは `dist/README.txt` だけ | 差なし（確かめは静的に済む） |
| 2.6・2.8 変更 0 | − | 触るのは新規のフォルダと文書だけ | 差なし |
| 2.7 `structure.md` | `dist/` の説明 | 1 文が在る（2.1） | 1 か所の書き足し |
| 3.1〜3.8 手元で入れて起動 | 手順・記録の形・判定の行 | `winget-local-check.md` の型・`root_resolved`・`本物のゴースト窓を開きました` | **制約**: `LocalManifestFiles` は管理者の手・`areka` を打つ端末は PATH を読み直した新しい物であること。**未決**: 有界の自動終了（`AREKA_APP_SMOKE_EXIT_MS`）を使うか |
| 4.1 状態を作る | 後から入れたゴースト・両方の記憶 | 検体の `.nar`・右クリックメニューの「インストール…」と「ゴースト」 | **未決**: 状態の作り方（5.3） |
| 4.2 上げ直し | より新しい版を名乗るマニフェスト・`winget upgrade` | 無い | **未知**: 手元のマニフェストで `winget upgrade --manifest` が入っている物を見つけるか。**未決**: 「置き換わった」をどう見分けるか（5.3）。結果の見込みは 3.1 |
| 4.3 外し方 | `--purge` なしの `winget uninstall` | 2026-10-03 は `--purge` だけ | 測るだけ。結果の見込みは 3.1 |
| 4.6 消えると分かったとき | `/kiro-discovery` での起票 | − | 3.1 の見込みどおりなら起票が要る。議題 5 に新しい材料（3.1 の最後） |
| 5.1 提出の手順書 | 道具・コマンド・フォーク・同意の手続き | 道具は未導入・フォークは無い | **未決**: 道具（5.1）。**未知**: 初回の提出者に求められる同意の手続き |
| 5.2〜5.7 提出と検査 | PR・印の読み取り | `gh` で PR の印は読める | **制約**: 提出は開発者の手。**外の不確かさ**: 未署名の exe が走査に掛かるか |
| 6.1〜6.3 記録と申し送り | `verification/`・後ろの spec の brief | `areka-P0-winget-release-automation` の brief が在り、議題 3 に「消えると分かったとき」の受け口が在る | 書き足すだけ |

## 5. 進め方の案

### 5.1 マニフェストの作り方と出し方

| 案 | 中身 | 良い所 | 弱い所 |
|---|---|---|---|
| A 手で書いて、`wingetcreate submit` で出す | YAML 3 ファイルを手で書く → `winget validate` → 手元で入れて確かめる → 同じフォルダを `wingetcreate submit` に渡す | 確かめた物と出す物と雛形が同じファイルになる（要件 2.3・3.1 にそのまま合う）。道具は提出にしか使わない | 欄の書き落としは `winget validate` 頼み。`wingetcreate submit` が 3 ファイルのフォルダを受けるかは文書に書かれていない（確かめが要る） |
| B 道具に作らせて、その場で出す | `komac new` か `wingetcreate new` に URL を渡し、問いに答えて、そのまま提出する | 道具が zip を見て欄を埋める。今の決まりに合った書式で出る | 出す前に手元で入れて確かめる順（要件 3 → 5）と合わない。`ArchiveBinariesDependOnPath` は手で足す必要が在り、出した物と雛形がずれやすい |
| C 道具に作らせてファイルに出し、整えて確かめ、同じ道具で出す | 道具の「提出せずにファイルへ出す」口で作る → 手で整える → `winget validate` → 手元で確かめる → 提出 | brief の Approach（「道具に作らせ、手で整える」）のとおり。道具の書式と手の直しの両方が取れる | 手順が 1 段増える。`komac` に「手元のマニフェストを出す」コマンドが在るかは、説明書の一覧では見つからなかった（`new`・`update`・`remove` など） |

- どの案でも、開発者の手で要るのは、道具を入れる・トークンか GitHub へのログイン・フォーク（道具が作るか手で作る）・提出の操作。
- `komac` は classic のトークン（`public_repo`）を求めると説明書に在る。`wingetcreate` はトークンを渡すか、渡さなければ GitHub へのログインを求める。
- 道具を使わずに `git` と `gh` だけで出す道も在るが、要件 5.1 は道具を 2 つのどちらかと決めている。

### 5.2 `dist/winget/` の下の並び

| 案 | 並び | 良い所 | 弱い所 |
|---|---|---|---|
| 平ら | `dist/winget/Areka.Areka.Portable.*.yaml` | いちばん短い | 版が見えないので、後の版が出た後に「今の版」と読み違えやすい（雛形は初回に出した物の写しのまま止まる） |
| 版のフォルダ | `dist/winget/0.0.2/…`（brief の例） | 「初回に出した版の写し」だと読める。`winget validate --manifest dist/winget/0.0.2` と打てる | フォルダが 1 段 |
| winget-pkgs と同じ並び | `dist/winget/manifests/a/Areka/Areka/Portable/0.0.2/…` | 出す先の並びとそのまま見比べられる | 5 段深い。雛形 3 ファイルには重い |

### 5.3 上げ直しの実測の組み方

- **より新しい版を名乗るマニフェスト**: ⒜ 版の欄だけ上げて、同じ `v0.0.2` の zip と同じハッシュを指す（brief の案・ネットワークだけで済む）／⒝ zip を手元で詰め直して `target\` から配る（2026-10-03 と同じ手元の配り方が要る）。
- **「置き換わった」の見分け**: ⒜ では新旧の中身が同じなので、上げ直しの前に同梱のファイルへ目印を付ける必要が在る。目印は `ghost\emo2\` の下に付ける（最上位の 6 ファイルを書き換えると、3.1 の 5 の検査に掛かって上げ直しが止まる見込み）。
- **状態の作り方**: ⒜ 開発者が右クリックメニューから検体の `.nar` を入れ、「ゴースト」で切り替える（本物の経路。`LastGhost` と両方の記憶が自然にできる）／⒝ フォルダを手で写す（速いが、記憶と「前に使っていたゴースト」は別に作る必要が在る）。
- **上げ直しのコマンド**: 要件 4.2 は `winget upgrade`。手元のマニフェストで入れた物を `winget upgrade --manifest` が見つけるかは未知。見つけないときの代わり（入っている所へもう一度 `winget install --manifest` を打つ）を要件の言う上げ直しと数えてよいかは、設計で決める。
- areka が動いたままだと winget は `areka.exe` を消せない。上げ直しと外し方の前に areka を終える手順が要る。

## 6. 規模と危うさ

- **規模: S**（1〜3 日の作業）。YAML 3 ファイルと文書だけで、コードは 0。brief の見立て（5〜7 タスク）と合う。ただし、提出した後に自動の検査の結果を待つ時間が別に掛かる。
- **危うさ: 中**。理由は 2 つ。① 3.1 の見込みが実機でも確かめられると、「初回の提出を今の版で進めるか」の決めが変わりうる。② 自動の検査（未署名の exe のウイルス対策の走査・無人のインストールとアンインストール）は外の仕組みで、赤になったときに本仕様の中では直せないことが在る（要件 5.6 が受ける）。作る物そのものの危うさは低い。

## 7. 設計へ持ち越す調べもの

1. **書式の版**: 1.12.0 と 1.28.0 のどちらで出すか。winget-pkgs が受ける版・道具が出す版・1.28.0 で増えた欄が areka に要るかを引き直す（開発機の winget は v1.29.380 なので、どちらも `winget validate` に掛けられる見込み）。
2. **道具**: `wingetcreate submit` が 3 ファイルのフォルダを受けるか。`komac` に手元のマニフェストを出す口と、提出せずにファイルへ出す口が在るか。どちらの道具がフォークを自分で作るか。それぞれのトークンの種類と、トークンをどこに覚えるか。
3. **初回の提出者の手続き**: Microsoft のリポジトリが求める同意（CLA）の手順と、PR のひな形の確かめ項目。
4. **自動の検査の印**: 赤のときに付く印の名前と意味（走査・無人のインストール・URL）。要件 5.4 の記録に何を書くかを決める材料。
5. **`winget upgrade --manifest`**: 手元のマニフェストで入れた物を見つけるか（実機で分かる）。
6. **3.1 の見込みの確かめ**: 要件 4 の実測で、表の各行を「残った」「消えた」「置き換わった」で埋める。外し方の後に入れ先のフォルダと `profile\areka\` が残るかも見る。
7. **`--scope machine`**: Program Files の側に入れたときに areka が記憶を書けるか。測らないなら、既知の制限として後ろの spec へ申し送るか、要件 6.3 の起票の相手にするかを決める。
8. **既定のロケール**: 日本語を既定にした既存のパッケージの例と、審査で言われた例が在るか（決まりとしては見当たらなかった）。
9. **名乗りと発行者の食い違い**: 名乗りの頭は `Areka`、`Publisher` は `ekicyou`。自動の検査には掛からない見込みだが、人の審査で問われうる。答えの文を手順書に用意するか。

## 8. 要件討議へ出す点

答えで作業が変わるものだけを並べる。

1. **`ghost`・`balloon` が上げ直しと外し方で中身ごと消える見込み（3.1）を受けて、初回の提出をどうするか**（議題 3・議題 5 に関わる）。仮置きは「`v0.0.2` で進める」。分かれ目は次の 3 つ。
   - ⒜ 仮置きのまま `v0.0.2` で出す。`v0.0.2` から次の版へ上げた人のゴーストは消える見込みで、後の版では救えない。置き場を変える仕事は別の spec。
   - ⒝ 実測（要件 4）までを本仕様で済ませ、消えると確かめられたら、提出は置き場を直した版が出てからにする（完了の線「PR の検査が通った」に届かないまま止まる形になる）。
   - ⒞ 実測の結果を見てから、開発者がその場で決める（要件 4.6 の「止めずに進める」を「報告して判断を仰ぐ」に読み替える）。
2. **`ArchiveBinariesDependOnPath` を付けるか**（議題 1）。新しい材料: 付けると `shiori-host32-helper.exe` も PATH から呼べるようになる／付けなくても開発者モードがオフの利用者は同じ PATH の形になる／3.1 の消える・残るはどちらでも同じ。
3. **書式の版を 1.12.0 のままにするか**（要件 1.3 は「提出の時点で受け付ける版」）。winget-pkgs の文書に 1.28.0 が在るが、直近の取り込みは 1.12.0。
4. **説明文の言語**（議題 4）。en-US を求める決まりは見当たらなかった。
5. **道具と作り方**（5.1 の A・B・C）。要件 3.1 の「提出するマニフェストそのもので確かめる」に素直に合うのは A と C。
6. **`dist/winget/` の下の並び**（5.2）。
7. **上げ直しの実測の組み方**（5.3）: 同じ zip で版だけ上げるか・状態を開発者がメニューから作るか・`winget upgrade --manifest` が効かないときの代わりを認めるか。
8. **要件に名前の無い欄を入れるか**: `Moniker`（`areka`）・`LicenseUrl`・`MinimumOSVersion`・`ReleaseNotesUrl`・`UpgradeBehavior`。要件 1.9 が禁じているのは、署名・インストーラー・ショートカット・依存の欄だけ。
9. **`--scope machine` で入れた形の扱い**（3.3）: 測るか、既知の制限として申し送るだけにするか。
10. **PR の印を誰が読むか**: 開発者が写すか、AI が `gh` の読むだけの問い合わせで読むか（提出の操作とトークンは開発者の手のまま）。
11. **`areka-P0-release-cycle` の文書の持ち主の名前の直し**（議題 6）。`winget-manifest-submission` の名前が出る行は、同 spec の `requirements.md` に 4 行・`design.md` に 1 行・`brief.md` に 7 行。うち `winget.yml` の持ち主として読み替えが要る行は、直す spec が数え直す。

### 要件討議での仕分け（2026-10-10）

- **設計で決める**（`/kiro-spec-design`）: 3 書式の版／5 道具と作り方／6 `dist/winget/` の下の並び／7 上げ直しの実測の組み方／8 要件に名前の無い欄。あわせて「7. 設計へ持ち越す調べもの」の 9 件。
- **要件へ反映済み**: 10 PR の印を誰が読むか（要件 5.4＝AI が読むだけの問い合わせで読んでよい）。3.1 の読みから、要件 4 の測る項目に利用者のバルーンとシェルの記憶を足し、winget の設定が既定のままで測ったことを記録に書く、とした。
- **開発者と決めた**（要件討議・2026-10-10。正本は `requirements.md` の議題の節）: 1 初回の提出＝実測の結果に関わらず `v0.0.2` を 1 回出して名乗りを押さえる。消える物が在れば、注意書き・置き場を直す spec の起票・「直るまで説明書に載せない／次の版を出さない」の申し送り／2 `ArchiveBinariesDependOnPath`＝付ける／4 説明文の言語＝既定を英語・追加で日本語の 4 ファイル。タグも決定／9 `--scope machine`＝1 回だけ測る／11 `areka-P0-release-cycle` の文書の名前の直し＝本仕様が完了のときに直す。

## 9. 出典

- リポジトリ: `tools/package.ps1`（`Test-ZipContent` の許可表・段「assemble」の写し）・`crates/areka/src/boot_config.rs`（`follow_exe_links`・`exe_location`・`resolve_boot_from` の `root_resolved`）・`crates/areka/src/ghost_session.rs`・`crates/areka/src/boot_resolve.rs`（`last_ghost_not_found`）・`dist/README.txt`（「■ 記憶の置き場」）・`.kiro/steering/structure.md`（「その他の最上位」）・`.kiro/specs/completed/areka-P0-release-package-versioned/verification/winget-local-check.md`・同 `design.md` の「Out of Boundary」・`.kiro/specs/areka-P0-winget-release-automation/brief.md`。
- GitHub（`gh`・読むだけ）: `ekicyou/areka` の Release `v0.0.2`／`microsoft/winget-pkgs` の `manifests/a/`・`doc/manifest/schema/`・直近の PR 2 件のファイル／`microsoft/winget-cli`・`microsoft/winget-create`・`russellbanks/Komac` の最新の Release。
- winget-cli のソース（タグ `v1.29.380`）: `src/AppInstallerCLICore/Workflows/PortableFlow.cpp`（`GetPortableProductCode`・展開したフォルダを 1 段だけ回す所・`VerifyExpectedState` の後の `--force` の分かれ・`Purge` の決め）／`src/AppInstallerCLICore/PortableInstaller.cpp`（`VerifyPortableFile`・`InstallFile`・`RemoveFile`・`ApplyDesiredState`・`RemoveInstallDirectory`）／`src/AppInstallerCommonCore/Manifest/ManifestCommon.cpp`（`DoesInstallerTypeIgnoreScopeFromManifest`）。
- 外の文書: https://github.com/microsoft/winget-pkgs/blob/master/doc/manifest/schema/1.12.0/installer.md ・ https://github.com/microsoft/winget-pkgs/blob/master/doc/Authoring.md ・ https://github.com/microsoft/winget-create/blob/main/doc/submit.md ・ https://github.com/russellbanks/Komac/blob/main/README.md

---

# 設計フェーズの調べと決定（2026-10-10）

- 作成: 2026-10-10（ブランチ `claude/areka-p0-winget-manifest-1c4fd7`・コミット `97d2161e` の上）
- 対象: 上の「要件討議での仕分け」が設計へ回した 5 点と、「7. 設計へ持ち越す調べもの」の 9 件。
- 進め方: winget-pkgs・winget-cli・winget-create・Komac の文書とソースを `gh` の読むだけの問い合わせで読み、Microsoft Learn の 2 頁を取ってきて読み、開発機の `winget` の説明（`--help`）を読んだ。`winget install`・`winget settings` は実行していない。ファイルは取ってきていない（Release の値は GitHub が示す情報だけを読んだ）。
- 上のギャップ分析は「YAML 3 ファイル」と書いているが、要件討議で日本語のロケールを足して 4 ファイルになった（要件 1.3）。上の文は当時のまま残す。

## まとめ

- **Feature**: `areka-P0-winget-manifest-submission`
- **Discovery Scope**: Extension（軽い調べ。今ある型＝完了 `areka-P0-release-package-versioned` の手元の確かめを伸ばす仕事で、調べの中心は外の決まりの引き直し）
- **Key Findings**:
  - winget-pkgs が今勧める書式は 1.12 のまま（PR のひな形の確かめ項目）。1.28.0 は文書に在るが、増えた欄は DSC 向けだけ。
  - `wingetcreate submit` も `komac submit` も、手元のマニフェストのフォルダを受ける。どちらも読み直して書き出すので、出た物は手元のファイルと字面が変わりうる＝提出の後に、PR の中身を雛形へ写し戻して見比べる段が要る。
  - 自動の検査は、今は GitHub の上で 10 段の検査として走る。通ると `Validation-Completed` が付き、その後に人が見る。実際の承認待ちの PR には `Azure-Pipeline-Passed`・`Validation-Completed`・`New-Package` が並んで付いている。初めての人には別の仕組みが `Needs-CLA` を付ける。
  - `winget upgrade` に `--manifest` が在る（開発機の v1.29.380 の説明と、ソースの分かれ）。上げ直しの実測は、版の欄だけ上げた確かめ専用のマニフェストで組める。
  - 注意書きに使える欄は `InstallationNotes`（既定のロケールと追加のロケールの両方に在り、インストールの終わりに出る）。

## 調べの記録

### 書式の版（持ち越し 1）

- **きっかけ**: winget-pkgs の文書に 1.28.0 が増えていた。要件 1.3 は「提出の時点で受け付ける版」。
- **読んだ物**: winget-pkgs の `.github/PULL_REQUEST_TEMPLATE.md`・`doc/manifest/README.md`・`doc/Authoring.md`・`doc/FirstContribution.md`・`doc/manifest/schema/1.12.0/installer.md` と `1.28.0/installer.md`（欄の一覧を機械で見比べた）。
- **分かったこと**:
  - PR のひな形の確かめ項目は「Manifest conforms to the 1.12 schema」。
  - `doc/manifest/README.md`: 「置き場は、新しい書式の受け付けを、対応した端末が行き渡るまで遅らせることが多い。PR のひな形に書いてある版を使ってほしい」。
  - `doc/Authoring.md` の参照先も 1.12.0。
  - 1.28.0 の installer の説明で増えた欄は `DesiredStateConfiguration`・`PowerShell`・`ModuleName`・`RepositoryUrl`・`Resources` など DSC 向けの物だけ。
  - `doc/FirstContribution.md`: 全ファイルの 1 行目に書式の場所を示す行（`# yaml-language-server: $schema=...`）を付ける・1 つのファイルにまとめた形（singleton）は不可・マニフェスト以外のファイルを同じ PR に入れない。
- **設計への効き方**: 1.12.0 で書く。提出の直前に、ひな形の行がまだ 1.12 かを読んで確かめる。

### 提出の道具（持ち越し 2）

- **きっかけ**: 要件 5.1 は `komac` か `wingetcreate` のどちらかを設計で選ぶ。ギャップ分析は「`wingetcreate submit` がフォルダを受けるか」「`komac` に手元のマニフェストを出す口が在るか」を未知としていた。
- **読んだ物**: winget-create の `doc/submit.md`・`doc/token.md`・`README.md`・`src/WingetCreateCLI/Commands/SubmitCommand.cs`・`src/WingetCreateCLI/Commands/BaseCommand.cs`・`src/WingetCreateCore/Common/GitHub.cs`／Komac の `README.md`・`src/commands/submit.rs`／両方の最新の Release。
- **分かったこと**:
  - `wingetcreate submit <フォルダ>`: `SubmitCommand` の定義は、渡された物がフォルダなら、中のファイルを全部読んでマニフェストの組（version・installer・既定のロケール・追加のロケール）に直し、検査してから出す。＝4 ファイルのフォルダを受ける。フォルダに YAML 以外が在ると読み損ねる作りなので、雛形のフォルダには YAML だけを置く。
  - フォーク: `SubmitPRAsync` の定義は、開発者のアカウントにフォークが無ければ作り、古ければ追い付かせてから、フォークの新しい枝へ書いて PR を出す（`BaseCommand` の `SubmitPRToFork` の既定は真）。PR の本文は winget-pkgs のひな形がそのまま入る（確かめ項目の印は付いていない）。題は `--prtitle` で渡せる。
  - トークン: `--token` を付けなければ、ブラウザでの GitHub へのログインになる。`doc/token.md` は「`--token` は記録に残りうる。手元ではログインの流れを勧める」と書く。覚えたログインは `wingetcreate token --clear` で消せる。トークンを使うなら classic の `public_repo`（細かい権限のトークンは不可）。
  - 出る物: 道具がマニフェストを読み直して書き出す。冒頭に道具の名前の行と書式の場所の行が付く。
  - `komac submit <フォルダ>`: ソースに在る（`--dry-run` も在る）。説明書の命令の一覧には載っていない。classic の `public_repo` のトークンを覚えさせる必要が在る。こちらも読み直して書き出す。
  - 最新の版: `wingetcreate` v1.12.13.0（2026-07-23）・`komac` v2.16.0（2026-03-29）。
- **設計への効き方**: `wingetcreate submit` を選ぶ。提出の後に写し戻しの段を置く。

### 初回の提出者の手続き（持ち越し 3）

- **読んだ物**: winget-pkgs の `.github/PULL_REQUEST_TEMPLATE.md`・`doc/FirstContribution.md`・`doc/Validation.md`、winget-create の `README.md`。
- **分かったこと**:
  - 同意（CLA）は、検査とは別の仕組みが見る。済んでいないと `Needs-CLA` が付き、取り込めない。同意は PR の案内に従って 1 回行えば、Microsoft のどのリポジトリにも効く。
  - PR の本文の確かめ項目: 同意／同じ変更の PR がほかに開いていない／1 つのマニフェストだけ／`winget validate --manifest` を通した／`winget install --manifest` で確かめた／1.12 の書式。
  - PR の題の形: 「New package: Publisher.Name version X.Y.Z」。
  - ふつうの提出に、先に Issue を立てる必要は無い。
- **設計への効き方**: 手順書に、同意と確かめ項目に印を付ける段を入れる。本仕様の流れ（検査 → 手元で入れる → 提出）は確かめ項目をすべて満たす。

### 自動の検査と印（持ち越し 4）

- **読んだ物**: winget-pkgs の `doc/Validation.md`・Microsoft Learn の提出の説明（頁の更新は 2026-07-14）・winget-pkgs で開いている「New package」の PR 8 件の印（`gh search prs`）。
- **分かったこと**:
  - 検査は GitHub の上の 10 段（PR の形／マニフェスト／URL／URL の配布元／決まりの文／一覧との整合／インストーラーの走査／インストールの確かめ／インストーラーの情報／まとめ）。前の段が赤だと後の段は飛ばされることが在る。やり直しは、モデレーターが PR に `@wingetbot run` と書いて起こす。
  - 通ると `Validation-Completed` が付き、その後にモデレーターが見て、承認すると `Moderator-Approved` が付いて自動で取り込まれる。
  - 開いている PR の実際の印: 承認待ちの物は `Azure-Pipeline-Passed`・`Validation-Completed`・`New-Package` の 3 つ。同意がまだの物はそれに `Needs-CLA` が付いている。＝`Validation-Completed` が付いていても、取り込まれてはいない。
  - Microsoft Learn の頁は `Azure-Pipeline-Passed` を「検査を通り、承認待ち」、`Validation-Completed` を「取り込まれる印」と書く。要件の事実の節はこの頁を写している。実際の並びとは少し違うが、要件 5.7 の完了の線は「自動の検査が通り、人の承認待ちになった」という中身で書かれているので、食い違いにはならない。
  - 失敗の印と意味は `doc/Validation.md` の各段と Learn の表に在る（設計の「赤の仕分け」の表に写した）。`Needs-Author-Feedback` は 10 日応えないと PR が閉じられる（Learn）。
  - インストールの確かめは、管理者でない利用者として、無人で入れ、入れた後に実行ファイルを見つけ、Defender で走査する。
- **設計への効き方**: 完了の判定を「`Validation-Completed` が付き、`Needs-CLA`・`Needs-Author-Feedback`・失敗の印が 0」とする。

### `winget upgrade --manifest`（持ち越し 5）

- **読んだ物**: 開発機の `winget upgrade --help`（v1.29.380）・Microsoft Learn の `winget upgrade` の説明（頁の更新は 2026-07-21）・winget-cli のタグ `v1.29.380` の `src/AppInstallerCLICore/Commands/UpgradeCommand.cpp`。
- **分かったこと**:
  - `-m,--manifest` が在る。Learn は「手元の YAML から上げ直しを行う」と書く。
  - ソースでは、`--manifest` が在るときの分かれは、マニフェストを読む → 入っている物の中からそのマニフェストに当たる物を探す → 1 つに決まることを確かめる → 入っている版より新しいことを確かめる → 入れる、の順。
  - 入っている物を見つけられるかは、手元のマニフェストで入れた物の覚えられ方に依るので、実機でしか確かめられない。
  - `--uninstall-previous` と `--purge` は `upgrade` にも在る（実測では付けない。既定の動きを測る）。
- **設計への効き方**: 上げ直しは `winget upgrade --manifest`。見つけられなければ、同じフォルダで `winget install --manifest` を打つ代わりの手を認め、どちらを使ったかを記録する（要件 4.2 の「上げ直し」と数える。どちらも、ギャップ分析の 3.1 の 3 で読んだ同じ処理を通る）。

### 入れ先と、外すときの設定（持ち越し 6・7 の前提）

- **読んだ物**: winget-cli のタグ `v1.29.380` の `doc/Settings.md`・開発機の `winget uninstall --help`。
- **分かったこと**:
  - 利用者ごとの入れ先の既定は `%LOCALAPPDATA%/Microsoft/WinGet/Packages/`（`portablePackageUserRoot`）、機械の全員向けの既定は `%PROGRAMFILES%/WinGet/Packages/`（`portablePackageMachineRoot`）。
  - 「外すときにポータブルな物の入れ先を丸ごと消す」は、利用者の設定の `uninstallBehavior.purgePortablePackage`（既定は偽）。
  - `winget uninstall` にも `-m,--manifest` が在る（手元のマニフェストで入れた物を外す、もう 1 つの口）。
- **設計への効き方**: 段 8 の入れ先の見込みと、要件 4.5 の「設定が既定のまま」の読み方が決まる。3.1 の見込みの確かめ（持ち越し 6）は、設計の段 5・段 6 の表がそのまま受ける。

### 注意書きの欄

- **読んだ物**: winget-pkgs の `doc/manifest/schema/1.12.0/defaultLocale.md`・`locale.md`、winget-cli の `src/AppInstallerCLICore/Workflows/InstallFlow.cpp`。
- **分かったこと**: `InstallationNotes` は「インストールが終わったときに利用者へ示す文」で、既定のロケールにも追加のロケールにも書ける。ソースでは `DisplayInstallationNotes` が、選ばれたロケールの文を出す。`Moniker` は既定のロケールだけの欄で、追加のロケールには無い。短い説明は 3〜256 字。タグは 16 個まで。
- **設計への効き方**: 要件 1.10 の「入れた直後に利用者へ示される欄」を `InstallationNotes` とする。

### 既定のロケール（持ち越し 8）と、名乗りと発行者（持ち越し 9）

- **読んだ物**: winget-pkgs の `doc/Authoring.md`・`doc/FirstContribution.md`・`doc/Policies.md`・`doc/manifest/schema/1.12.0/version.md`。
- **分かったこと**: 既定のロケールを en-US にせよという決まりは、どれにも無い。名乗りは「ふつう `Publisher.Package`。区切りを足してよい」で、発行者の欄と名乗りの頭が同じであることを求める文は無い。検査の「URL の配布元」は、`InstallerUrl` が発行者の正式な配布元から来ていることを見る（`PackageUrl` を書き、そこから辿れると確かめやすい、と在る）。
- **設計への効き方**: 既定は英語のまま（要件討議の決定）。`PackageUrl` をリポジトリにし、`InstallerUrl` が同じリポジトリの Release を指す形にする。人に問われたときの答えの文を手順書に用意する。

### 実在するマニフェストの形と、Release の値

- **読んだ物**: winget-pkgs の `manifests/r/rhysd/actionlint/1.7.9/` の 3 ファイル（zip＋portable・x86 と x64 と arm64・道具が出した物）／`gh api` での `ekicyou/areka` の Release `v0.0.2` とタグ `v0.0.2` の `LICENSE-MIT`／winget-pkgs の `manifests/a/Areka`・`ekicyou/winget-pkgs`・`Areka.Areka` を含む PR。
- **分かったこと**:
  - 道具が出す形: 1 行目が道具の名前、2 行目が書式の場所の行。installer は `InstallerType: zip`・`NestedInstallerType: portable`・`NestedInstallerFiles`・`ReleaseDate`・`Installers`（CPU の種類ごとに URL とハッシュ。ハッシュは大文字）の順。既定のロケールは `Publisher`・`PublisherUrl`・`PublisherSupportUrl`・`PackageName`・`PackageUrl`・`License`・`LicenseUrl`・`ShortDescription`・`Tags`・`ReleaseNotesUrl` を持つ。
  - Release `v0.0.2`: 公開 `2026-10-06T12:49:21Z`・下書きでも先行版でもない。GitHub が示す値は x64 `66d3a9a0…6a751c9c`・arm64 `77c5356a…9fdd79fb`（上の 2.2 と同じ）。タグ `v0.0.2` の根に `LICENSE-MIT` が在る。
  - `manifests/a/Areka` は無い。`ekicyou/winget-pkgs` は無い。`Areka.Areka` を含む PR は 0 件。
- **設計への効き方**: 雛形の欄の並びを道具が出す形に寄せておく（写し戻しの差を小さくする）。

## 持ち越した 9 件の答え

| 件 | 答え |
|---|---|
| 1 書式の版 | 1.12.0（PR のひな形が勧める版） |
| 2 道具 | `wingetcreate submit`。フォルダを受ける・フォークを自分で作る・ブラウザでのログインで済む。`komac submit` も在るが採らない |
| 3 初回の提出者の手続き | 同意（CLA）は PR の案内に従って 1 回。PR の本文の確かめ項目 6 つに印を付ける（当てはまるときだけの「Issue への結び付け」は数えない） |
| 4 自動の検査の印 | 10 段。通ると `Validation-Completed`（`Azure-Pipeline-Passed` も付く）。失敗の印は設計の「赤の仕分け」の表 |
| 5 `winget upgrade --manifest` | 命令は在る。入っている物を見つけるかは実機で分かる。見つけないときの代わりの手を設計で決めた |
| 6 3.1 の見込みの確かめ | 設計の段 5・段 6 の表で測る（この調べでは測っていない） |
| 7 `--scope machine` | 入れ先の既定は `%PROGRAMFILES%/WinGet/Packages/`。段 8 で 1 回測る（要件討議の決定） |
| 8 既定のロケール | en-US を求める決まりは無い。要件討議の決定（既定は英語・追加で日本語）のまま |
| 9 名乗りと発行者の食い違い | 決まりには掛からない。問われたときの答えの文を手順書に用意する |

## 設計の決定

### 決定: 書式の版は 1.12.0

- **選ばなかった案**: 1.28.0（文書に在る最新）。
- **理由**: 置き場が勧めているのは PR のひな形に書いた版で、それが 1.12。1.28.0 で増えた欄は areka に要らない。開発機の winget v1.29.380 はどちらも読めるが、新しい書式は古い winget の利用者に届かない。
- **残す確かめ**: 提出の直前に、ひな形の行を読み直す。

### 決定: 雛形は手で書き、`wingetcreate submit` で出す

- **選ばなかった案**: ① 道具に作らせてそのまま出す（`wingetcreate new`・`komac new`）＝出す前に手元で確かめる順と合わず、`ArchiveBinariesDependOnPath` と日本語のロケールは結局手で足す。② 道具に作らせてファイルへ出し、手で整える＝値がすべて分かっている 4 ファイルに、道具の問いに答える手間を足すだけ。③ `komac submit`＝説明書の一覧に無い口で、classic のトークンを作って覚えさせる手間が要る。④ `git` と `gh` だけで出す＝字面がそのまま出る利点は在るが、要件 5.1 が道具を 2 つのどちらかと決めている。
- **理由**: 確かめた 4 ファイルをそのまま道具に渡せる。Microsoft の文書が勧める道具で、手元のフォルダを出す口が文書に在る。トークンを作る場面が無い。
- **代償**: 道具が読み直して書き出すので、出た物の字面が変わりうる。提出の後に PR の中身を雛形へ写し戻し、欄と値が変わっていないことを確かめる段を置いた。

### 決定: 置き場は `dist/winget/0.0.2/`

- **選ばなかった案**: 平ら（`dist/winget/*.yaml`）＝版が見えず、後の版が出た後に「今の版」と読み違える。winget-pkgs と同じ 5 段＝4 ファイルには深すぎる。
- **理由**: 雛形は初回に出した版の写しのまま止まる。版のフォルダなら、そのまま `winget validate` と `wingetcreate submit` に渡せる。
- **決まり**: このフォルダには YAML だけを置く（説明書きのファイルを足さない。道具がフォルダの中のファイルを全部マニフェストとして読むため）。説明は steering の `structure.md` に書く。

### 決定: 上げ直しの実測の組み方

- **より新しい版**: 版の欄だけ `0.0.2.1` に上げ、同じ zip と同じハッシュを指す。`0.0.3` にしないのは、実在しうる次の版と取り違えないため。zip を手元で詰め直して配る案は採らない（手元の配り方と別の zip が要り、測りたいこと＝winget が何を消すか＝は同じ zip でも変わらない）。
- **「置き換わった」の見分け**: 同梱の `ghost\emo2\readme.txt` に 1 行足し、隣に目印のファイルを置く。最上位の 6 ファイルには触らない（winget がハッシュを見ていて、合わないと上げ直しが止まる＝上の 3.1 の 5）。
- **状態の作り方**: 開発者が右クリックメニューから作る（本物の経路）。フォルダを手で写して作る案は、前に使っていたゴーストの覚えと記憶を別に作る必要が在るので採らない。ただし、作った状態の写しを取っておき、外し方の実測の前には写しから戻す（メニューの操作を 2 度しない）。
- **使う検体**: ゴーストは `vendors/sample_ghost/claudia.nar`（同梱のバルーンを 2 つ持つ）、バルーンは `vendors/sample_ghost/emo2-kakukaku-wplimit.nar`（`install.txt` の `type,balloon`）。
- **一覧を取る時機**: 上げ直しの直後、areka を起動する前（起動すると areka が記憶を作り直すので、消えた物が在ったように見えなくなる）。

### 決定: 要件に名前の無い欄

- **入れる**: `LicenseUrl`・`ReleaseNotesUrl`・`PublisherSupportUrl`（道具が GitHub のパッケージで自動で埋める欄。実在のマニフェストにも在る）。
- **入れない**: `Moniker`（`areka` の 1 語はインストーラー版 `Areka.Areka` のために空ける）／`MinimumOSVersion`（測った値が無い）／`UpgradeBehavior`（既定のまま。`uninstallPrevious` にしても消える物は同じ見込みで、`deny` は上げ直しを止めるだけで外すときの消え方は変わらない。消えると分かったときの手当ては、要件討議で「注意書き・起票・次の版を出さない」と決まっている）／`Description`・`Author`・`Copyright`。

### 決定: 完了の線の判定

- **判定**: PR に `Validation-Completed` が付き、`Needs-CLA`・`Needs-Author-Feedback`・失敗の印がどれも付いていない。
- **理由**: 実際の承認待ちの PR の印の並びと、winget-pkgs の `doc/Validation.md` の書き方に合わせた。`Needs-CLA` が付いたままでは人の承認へ進めないので、完了に数えない。

### 決定: 誰の手で行うか

- **決まり**: 機械の状態を変える操作（winget の設定・出し入れ・入れ先のフォルダへの書き込み・提出）は開発者の手、読むだけの操作と文書は AI。
- **理由**: 要件 3.1・3.4・3.6・4.7・4.8・5.2 が開発者の手と書いている。線を 1 本にしておくと、段ごとに迷わない。

## 統合の見直し

- **まとめられる物**: 段 3・段 5・段 8 の「起動して判定する」は、同じ有界の起動と同じ記録の行で判定する。後片付けの確かめ（要件 3.6 と 4.8）は、同じ 4 項目の表を使い回す。
- **作らずに借りる物**: 検査は `winget validate`、提出は `wingetcreate`、PR の読み取りは `gh`、起動の判定は areka が今出している記録の行。新しいスクリプトは 0 本。手元の確かめの手順と記録の形は、完了 `areka-P0-release-package-versioned` の `verification/winget-local-check.md` を伸ばす。
- **削った物**: `dist/winget/` の中の説明書きのファイル（`structure.md` の 1 か所で足りる）／確かめを自動で回すスクリプト（開発者の手の操作が挟まるので、文書に書いたコマンドで足りる）／zip を作り直して中身が変わらないことを確かめる走行（`tools/**` の変更が 0 なので、写す経路の読みで足りる）／PR を常に見張る仕組み（区切りごとに読みに行けば足りる）／Windows サンドボックスでの確かめ。

## 危うさと手当て

- **未署名の exe が走査に掛かる**（外の仕組み）— 設計の「赤の仕分け」で「雛形の直しでは消えない」に仕分け、要件 5.6 のとおり記録・報告・起票する。完了としない。
- **`wingetcreate submit` が欄か値を変えて出す** — 写し戻しの見比べで見つけ、止めて報告する。
- **`winget upgrade --manifest` が入っている物を見つけない** — 代わりの手を決めてあり、使ったコマンドを記録する。
- **自然な操作ではシェルの記憶ができない** — 作る操作を足す。それでもできなければ「測れなかった」と理由を書いて報告する。
- **設定がオンのまま作業が切れる** — 「変えた設定」の表に戻した時刻が無いことで分かる。再開のときに、後片付けと設定の戻しを先に済ませる。
- **PR に直しを求める印が付いたまま 10 日過ぎる** — 手順書に期限を書き、後ろの spec への申し送りにも書く。

## 参照したスキルと指針

- `kiro-spec-design` の決まり（`design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`）。この仕事に当たる分野別のスキル（画面の設計・アクセシビリティなど）は無い。
- 設計の見直し（review gate）: 要件の番号 46 個がすべて設計の対応表に在ることを機械で確かめた（欠け 0・余り 0）。直しは 1 回（頼る物の印を平易な語に直す・Microsoft Learn との書き方の違いの述べ方・一覧を取る時機・状態を作ってから上げ直しまで起動しない決まり・「有界」の言い換え・ショートカットの欄が書式に無いことの明記）。要件の食い違いは見つからなかった。

## 設計フェーズの出典

- winget-pkgs（`gh`・読むだけ・2026-10-10）: `.github/PULL_REQUEST_TEMPLATE.md`／`doc/manifest/README.md`／`doc/FirstContribution.md`／`doc/Authoring.md`／`doc/Policies.md`／`doc/Validation.md`／`doc/manifest/schema/1.12.0/` の `version.md`・`installer.md`・`defaultLocale.md`・`locale.md`／`doc/manifest/schema/1.28.0/installer.md`／`manifests/r/rhysd/actionlint/1.7.9/`／開いている「New package」の PR の印。
- winget-create（`gh`・読むだけ）: `doc/submit.md`・`doc/token.md`・`README.md`・`src/WingetCreateCLI/Commands/SubmitCommand.cs`・`src/WingetCreateCLI/Commands/BaseCommand.cs`・`src/WingetCreateCore/Common/GitHub.cs`。
- Komac（`gh`・読むだけ）: `README.md`・`src/commands/submit.rs`。
- winget-cli（タグ `v1.29.380`）: `doc/Settings.md`・`src/AppInstallerCLICore/Commands/UpgradeCommand.cpp`・`src/AppInstallerCLICore/Workflows/InstallFlow.cpp`。
- 開発機の winget v1.29.380: `winget upgrade --help`・`winget uninstall --help`・`winget validate --help`。
- Microsoft Learn: https://learn.microsoft.com/en-us/windows/package-manager/package/repository ・ https://learn.microsoft.com/en-us/windows/package-manager/winget/upgrade
- リポジトリ: `tools/package.ps1`（段「assemble」の写し・起動の確かめの環境変数と目印の行）・`vendors/sample_ghost/README.md` と検体の `install.txt`・`.kiro/steering/structure.md`（「その他の最上位」の行）・`.kiro/specs/areka-P0-release-cycle/requirements.md` と `design.md`（本仕様の名前が出る行）。
