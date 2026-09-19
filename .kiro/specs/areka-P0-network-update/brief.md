# Brief: areka-P0-network-update

> 2026-09-18 `/kiro-discovery` 再入（棚卸⑭＝α ゴールへの組み直し）で起票。`doc/ukadoc-coverage/roadmap-draft.md` 段階 B 順位 1 の束「更新」（候補名 `areka-P0-network-update`＝同名）。
> 本文の file:line は**起票時の実測値**（2026-09-18）。着手時に必ず引き直すこと。

## Problem

**誰の何が困っているか**: ゴーストを入れた第三者。作者が辞書を直して配布サイトを更新しても、areka では受け取れない。

今日の areka には **HTTP クライアントが 1 つも無い**。`Cargo.lock`（242 パッケージ）に `reqwest`／`ureq`／`hyper`／`curl`／`tokio`／`rustls`／`native-tls` は無く、ルート `Cargo.toml` の `windows` 機能一覧に `Win32_Networking_WinHttp`／`WinInet` も無い。`updates2.dau`／`updates.txt` を読むコードも無い（`updates.txt` は検体 `emo2` の中身として存在するだけで、誰も読まない。2026-09-19 以降の在処は `vendors/sample_ghost/emo2.nar` の中）。`homeurl` の読み手も無い。更新系イベント（`OnUpdateBegin`〜`OnUpdateResult`・束「更新」の `members` 51 件）は全て未対応。

## Current State（正典の要点・`ukadoc:dev_update`／`ukadoc:manual_update`／`ukadoc:spec_update_file:*`）

- **更新先 URL**: ゴーストの `descript.txt` の `homeurl`（末尾 `/`）。起動中は SHIORI リソース `homeurl` が優先（`ukadoc:descript_ghost:homeurl_2cURL:1`）。シェル・バルーンは各 `descript.txt` の `homeurl`。
- **更新定義ファイル**: `homeurl` 直下の `updates2.dau`（無ければ `updates.txt`）。**行の形＝`ファイルパス\x01MD5\x01拡張フィールド…`、改行は CRLF**（`ukadoc:spec_update_file:_884c_30d5_30a9_30fc_30de_30c3_30c8:1`）。パスは `/` 区切り。拡張フィールドは `size=`・`date=`・`charset=`（先頭エントリのみ）。`updates.txt` は `file,` 接頭辞付きの同形式と `charset,` 行。**無効エントリ**: MD5 なし・末尾 `/`・`../` を含む（`ukadoc:spec_update_file:_30bb_30ad_30e5_30ea_30c6_30a3_30c1_30a7_30c3_30af:1`）。URL エンコード: 全エントリがパーセント符号化済みなら復号して使い、そうでなければ URL 側だけ符号化する。
- **手順**（SSP の実装・ukadoc の各イベント定義から）: `OnUpdateBegin`（Ref0＝ゴースト名・Ref1＝フルパス・Ref3＝種別・Ref4＝理由）→ 定義ファイル取得 → ローカルの MD5 と比較 → 差分の一覧で `OnUpdateReady`（Ref0＝総数−1・Ref1＝ファイル名の一覧・Ref3・Ref4）→ 各ファイルを `OnUpdate.OnDownloadBegin`（Ref0＝名・Ref1＝番号・Ref2＝総数−1）→ 落としたファイルの MD5 照合 `OnUpdate.OnMD5CompareBegin`／`Complete`／`Failure` → 全部済んだら `delete.txt`（`delete[数字].txt`）の相対パスを削除 → `OnUpdateComplete`（Ref0＝理由・Ref1＝更新したファイル一覧）または `OnUpdateFailure`（Ref0＝失敗理由・Ref1＝失敗ファイル名）→ 総括 `OnUpdateResult`（Ref*＝`種別\x01OK|NG\x01件数または理由(\x01ファイル名)`）。差分 0 なら `OnUpdateCheckComplete`／`OnUpdateCheckFailure`。ゴースト以外（シェル・バルーン）は `OnUpdateOther*` の名で同じ列。`useorigin1`（番号を 1 始まりにする）はリソース。
- **入口**: `\![updatebymyself]`・`\![update,ghost|shell|balloon(+…)]`・`\![updateother,--shell=…]`・メニュー「ネットワーク更新」・`OnUpdateProcessExec`（Ref0＝`manual`／`auto`／`testonly`・台本を返せば更新処理を差し替えられる）。
- **削除**: `delete.txt`（`ukadoc:descript_install:_76f8_5bfe_30d1_30b9:1`＝ゴーストのホームからの相対パス・`\` 区切り・末尾 `\` はフォルダ）。

## Desired Outcome

完了時に次が真になっている。

1. **メニュー「ネットワーク更新」と `\![updatebymyself]` で、現ゴースト＋現シェル＋現バルーンが更新される。** 手順は上の正典どおり。イベント列と Ref を落とさない。
2. **`updates2.dau` と `updates.txt` の両方を読める。** 無効エントリは捨てて `warn!`。`charset` は先頭エントリの拡張フィールド（無ければ Shift_JIS を既定＝正典「OS デフォルト」を日本語ゴースト前提で固定・裁定候補）。
3. **落としたファイルは MD5 が一致してから本番の位置へ動く**（一時フォルダに落とし、全件一致で `rename`）。一致しなければそのファイルの `OnUpdate.OnMD5CompareFailure` と `OnUpdateFailure` で止め、本番の木は変えない。
4. **`delete.txt` を適用する。** `..` を含む行・絶対パスは無視して `warn!`。
5. **更新後は `\![reload,ghost]` 相当で読み直す**——α では `ghost-shell-balloon-switch` の「同じゴーストへ切替」（`OnGhostChanging` を送らず終了挨拶も再生しない・SSP の `\![reload,ghost]` と同じ振る舞い）で足りる。読み直しの正典 `\![reload,…]` の完全形は α 後。
6. **`\![execute,install,url,URL,nar]`**（束「インストール」）＝URL から一時フォルダへ落として `ghost-install` の `InstallRequest` に渡す。`feed`／`homeurl` 形は α 後。
7. **失敗が黙らない。** DNS・接続・4xx/5xx・MD5 不一致・書き込み失敗はそれぞれ `error!`＋イベントの Ref0（失敗理由）に写す。

## Approach

**選んだ形**: HTTP は OS の **WinHTTP**（`windows` crate の `Win32_Networking_WinHttp` 機能＝新規 crate 0・TLS も OS）。MD5 は `md-5`（RustCrypto・MIT OR Apache-2.0・`deny.toml` の許可リスト内）を **1 本だけ**足す。

| 段 | 中身 | 検証 |
|---|---|---|
| ① 定義ファイル | `updates2.dau`／`updates.txt` の読み手（行の形・拡張フィールド・無効エントリ・URL 符号化の判定）と、ローカル木との差分計算（MD5） | 決定論: 固定の定義ファイルとローカル木で差分を突き合わせる。無効エントリ 3 種。`charset` あり／なし |
| ② 取得 | `HttpFetch` トレイト（`get(url) -> bytes`）と WinHTTP 実装。テストは偽実装（記憶 prefer-x64-fake-boundary-tests-not-x86＝ネットに出ない） | 決定論: 偽 HTTP で「定義ファイル → 差分 → ダウンロード → 照合 → 確定」の一周。実機: 開発者の配布サーバ（または `file://` 相当のローカル HTTP）で 1 周 |
| ③ イベント相 | kanade の schedule に「更新」の相（`OnUpdateBegin` … `OnUpdateResult`・`OnUpdateOther*` の名の付け替え・`useorigin1`） | 決定論: 偽 SHIORI でイベント列と Ref を突き合わせる。差分 0・失敗・成功の 3 経路 |
| ④ 入口 | メニュー登記（`popup-menu-minimal`）・`\![updatebymyself]`／`\![update,…]`／`\![updateother,…]` の消費者・`OnUpdateProcessExec` | 決定論: 台本 → `UpdateRequest`。`OnUpdateProcessExec` に台本が返ったら標準処理を行わない |
| ⑤ 後始末 | `delete.txt`・一時フォルダの掃除・読み直し（同じゴーストへの切替） | 決定論: `delete.txt` の安全性 3 種 |

**取らない形**: `reqwest`／`ureq`（依存が数十増える・TLS スタックの選定と `deny.toml` の再検討が要る）。α は WinHTTP で足り、将来 x64/arm64 以外へ出る予定も無い（roadmap「制約」）。

## Scope

- **In**:
  - 定義ファイルの読み手（`updates2.dau`／`updates.txt`）と差分計算
  - WinHTTP による取得（`HttpFetch` の境界＋実装 1 つ）と偽実装
  - ゴースト・シェル・バルーンの更新（`OnUpdate*`／`OnUpdateOther*` の全列・`OnUpdateResult`／`OnUpdateResultEx`）
  - `OnUpdateCheckComplete`／`OnUpdateCheckFailure`／`OnUpdateProcessExec`／`useorigin1`／`homeurl` リソース／`other_homeurl_override`
  - 入口 4 つ（メニュー・`\![updatebymyself]`・`\![update,…]`・`\![updateother,…]`）
  - `delete.txt`／`delete[数字].txt`
  - 一時フォルダへの取得と原子的確定
  - `\![execute,install,url,URL,nar]`
- **Out**:
  - `\![update,platform]`（本体の更新・`OnBasewareUpdating`／`OnBasewareUpdated`・α 後）
  - `\![execute,createupdatedata]`／`OnUpdatedataCreating`／`OnUpdatedataCreated`（更新ファイルを**作る**側＝束「開発者機能」・α 後）
  - `updates2.dau` の**書き出し**
  - 定期自動更新（`OnUpdateProcessExec` の `auto`・設定画面が要る・α 後）
  - `feed`（RSS）・`homeurl` 形のインストール・`OnURLQuery`（α 後）
  - プラグイン・ヘッドライン・言語パックの更新
  - `.gitignore` 形式のフィルタ・`developer_options.txt`（作る側の話）

## Boundary Candidates

- **定義ファイルと差分**（純粋層・fixture 直入力）
- **取得**（`HttpFetch` の境界＝唯一の外界）
- **イベント相**（kanade）
- **確定と削除**（一時フォルダ → `rename`・`delete.txt`）

## Out of Boundary

- インストール手続き（`ghost-install`）・根（`baseware-root-layout`）・メニューの器（`popup-menu-minimal`）
- 読み直しの完全形 `\![reload,…]`

## Upstream / Downstream

- **Upstream**: `areka-P0-baseware-root-layout`（更新対象のフォルダ）／`areka-P0-popup-menu-minimal`（登記）／`areka-P0-ghost-install`（`InstallRequest`）／`areka-P0-ghost-shell-balloon-switch`（読み直し）／`areka-P0-nar-install`（パス安全性の検査を共用・`charset` 復号）／完了仕様 `areka-P0-charset-canon`。
- **Downstream**: `alpha-release-signoff`（第三者の手順「更新する」）。α 後: `\![update,platform]`・作る側（開発者機能）は本仕様の読み手と `HttpFetch` の上に乗る。

## Existing Spec Touchpoints

- **Extends**: なし（新規）。
- **Adjacent**:
  - `areka-P0-ghost-install`（同じ α ウェーブ・**直列**）——`MenuRegistry` への登記と kanade の schedule を両方が触る。install → update の順。
  - `areka-P0-translate-pipeline`／`sakura-time-directives`／`balloon-lifecycle-events`（α 後・kanade の schedule）——後着が rebase。
  - `areka-P0-property-query-channels`（α 後）——`homeurl` リソースの `GET` は本仕様が薄い読み手を持ち、channels が着地したらそちらへ寄せる。

## Constraints

- **外部依存の追加 1 本（`md-5`）は `tech.md` への「意図的依存追加」の登記と開発者の承認が要る**（`encoding_rs`・`zip` と同じ扱い）。`cargo deny --offline check licenses bans sources` を要件段階で実走して緑を確かめる。
- `windows` crate の機能フラグ `Win32_Networking_WinHttp` を 1 行足す（crate の追加ではない）。
- **ネットへ出るテストを常時テストに入れない。** 決定論は偽 `HttpFetch`。実機 1 周は開発者の配布サーバか、`file://` 相当のローカル HTTP（`crates/pilot` に最小のサーバを置くのも可＝dev-only）。
- 取得は UI スレッドで行わない（背景プール）。進捗イベントは kanade 経由で順序を保つ。
- 落としたファイルが本番の木を汚す経路を作らない（全件 MD5 一致 → `rename`）。
- 1 ファイル 1,000 行。決定論テスト網羅は必達。ログ無し失敗経路の禁止。
- **裁定候補 ⑸（開発者）**: 定義ファイルの `charset` 無指定時の既定を Shift_JIS に固定するか OS 既定（`GetACP`）にするか。日本語ゴースト前提で **Shift_JIS 固定**を推す（`charset-canon` の既定と揃う）。
- 規模 **M**。

- **2026-09-18 `nar-install` 設計からの申し送り（使用中の宛先）**: `areka-nar` の展開は「作業フォルダに組んでから宛先と入れ替える」形で、宛先の中のファイル（起動中のゴーストの `shiori.dll` 等）が開かれていると入れ替えが失敗し、宛先は無傷のまま `NarError::Io { phase: Commit, rolled_back: true }` が返る。**エンジンは SHIORI の解放を試みない**。起動中のゴーストへ入れる・更新する・切り替える経路は、呼び出し側が先に SHIORI をアンロード（`OnClose` 相当の終了経路）してから `install` を呼ぶこと。
