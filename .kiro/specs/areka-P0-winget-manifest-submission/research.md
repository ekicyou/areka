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

## 9. 出典

- リポジトリ: `tools/package.ps1`（`Test-ZipContent` の許可表・段「assemble」の写し）・`crates/areka/src/boot_config.rs`（`follow_exe_links`・`exe_location`・`resolve_boot_from` の `root_resolved`）・`crates/areka/src/ghost_session.rs`・`crates/areka/src/boot_resolve.rs`（`last_ghost_not_found`）・`dist/README.txt`（「■ 記憶の置き場」）・`.kiro/steering/structure.md`（「その他の最上位」）・`.kiro/specs/completed/areka-P0-release-package-versioned/verification/winget-local-check.md`・同 `design.md` の「Out of Boundary」・`.kiro/specs/areka-P0-winget-release-automation/brief.md`。
- GitHub（`gh`・読むだけ）: `ekicyou/areka` の Release `v0.0.2`／`microsoft/winget-pkgs` の `manifests/a/`・`doc/manifest/schema/`・直近の PR 2 件のファイル／`microsoft/winget-cli`・`microsoft/winget-create`・`russellbanks/Komac` の最新の Release。
- winget-cli のソース（タグ `v1.29.380`）: `src/AppInstallerCLICore/Workflows/PortableFlow.cpp`（`GetPortableProductCode`・展開したフォルダを 1 段だけ回す所・`VerifyExpectedState` の後の `--force` の分かれ・`Purge` の決め）／`src/AppInstallerCLICore/PortableInstaller.cpp`（`VerifyPortableFile`・`InstallFile`・`RemoveFile`・`ApplyDesiredState`・`RemoveInstallDirectory`）／`src/AppInstallerCommonCore/Manifest/ManifestCommon.cpp`（`DoesInstallerTypeIgnoreScopeFromManifest`）。
- 外の文書: https://github.com/microsoft/winget-pkgs/blob/master/doc/manifest/schema/1.12.0/installer.md ・ https://github.com/microsoft/winget-pkgs/blob/master/doc/Authoring.md ・ https://github.com/microsoft/winget-create/blob/main/doc/submit.md ・ https://github.com/russellbanks/Komac/blob/main/README.md
