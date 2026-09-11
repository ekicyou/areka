# ギャップ分析（research.md）

> 作成: 2026-09-11・対象: `.kiro/specs/areka-P0-ukadoc-coverage-roadmap/requirements.md`（確定版）
> 方法: `crates/ukadoc-survey`・`doc/ukadoc-coverage/`・`.kiro/specs/*/brief.md`・`.kiro/steering/roadmap.md`・`.kiro/specs/completed/areka-P0-emo2-conformance-e2e/` を Grep／Glob／Read と使い捨ての数え上げスクリプトで実測した。数値はすべて本日の作業ツリー（`67e0a4d3`）の写真である。
> 立場: 情報と選択肢を示す。決定は要件ディスカッションと設計に委ねる。

## 1. 分析の要約

- **要件の Introduction の実測値は 1 件を除いて全部正しい**（§3 の表）。食い違いは「持ち越し 4 件の引受先がすべて W13」と書いた点で、`dpi-transition-two-tick-bounce` は roadmap.md の spec 台帳 #11 で **W14** である（要件 9.4 は正しく W14 と書いており、Introduction 7 だけが古い）。もう 1 件、要件 2.5 の「`report-summary` がドメイン別報告 4 本の改行を書き換える」は事実と違う（§3 の注記）。
- **道具の側は再利用できる部品が揃っている**——台帳の読み込み（`ledger::read`）・束の連結成分（`report::bundle::bundles`）・全体報告の描画（`report::summary::render_summary`）・「文書中の id が正典に実在する」検査の前例（`tests/consistency/examples.rs`）・摂動の道具（`tests/consistency/perturb.rs`）・母数 0 を許さない前例（`non_vacuity.rs`）。要件 11 の判定 6 種は、この前例の形をそのまま延ばせば作れる。
- **最大のギャップは「束のカバー率」である。** 機械の束 123 個に現れる id は **460 件**（うち状態が実装済み／語彙のみ／縮退／未対応のものは 425 件）で、要件 4.2 が「ちょうど 1 つの束か単独項目か」を決めよと言う **1,552 件のうち 1,127 件はどの束にも現れない**。名前付き束を機械の束からだけ組むと、単独項目が 1,100 件を超える。ここは要件ディスカッションで「名前付き束は機械の束の外の id を含んでよいか」を決める必要がある（§6 判断 1）。
- **台帳への書き戻し（要件 7）は道具に書き込み手段が無い。** `ledger::write` にあるのは初期値の差し込み（`merge_initial`）だけで、既存の項目の `priority` を書き換える経路は無い。手編集（1,552 項目）か、既存の塊のバイト列を保ったまま欄だけ置き換える小さな副手続きを足すかの選択になる。後者は要件 12.2（接触は判定と「判定に要る読み込み・出力の副手続き」に限る）の読み方に掛かる（§6 判断 3）。
- **要件 11.1 ⑹（全体報告の新しさ）には並走の副作用がある。** `render_summary` はソース木を歩いて証拠の件数を数えるので、常時検査に入れると**他の spec がソースに正典 URL を 1 行足すだけで赤になる**。上流が除外した理由（台帳の取り合い）は消えたが、ソース起因の赤が新しく生まれる（§5.5・§6 判断 5）。

## 2. 既存資産の現状

### 2.1 道具 `crates/ukadoc-survey`（実装済み・完了 spec `ukadoc-survey-toolkit`）

| 層 | 場所 | 中身 | 行数の目安 |
|---|---|---|---|
| 入口 | `src/main.rs`・`src/cli/mod.rs` | 副手続き 8 つの振り分け表 `SUBCOMMANDS`（`catalog`・`ledger-init`・`report`・`report-summary`・`check`・`evidence`・`candidates`・`diff`）。引数は取らない | 26・163 |
| 生成 | `src/cli/generate.rs` | `report()`＝ドメイン別 4 本を `write_lf` で書く・`report_summary()`＝カタログ＋台帳 4 本＋ソース走査の証拠で `render_summary` を書く | 302 |
| 純粋層 | `src/model.rs` | 凍結語彙: `Status` 7・`LinkKind` 6・`THEMES` 8・`Domain` 4。既定の腕を置かない `match` | 322 |
| 純粋層 | `src/ledger/{mod,read,write,blocks}.rs` | `LedgerEntry`（id・status・introduced・alias_of・supersedes・owner・priority・values・links・note）・`Ledger`（`entries: BTreeMap`＋`file_order`）。`read()` は知らない欄を落とす。**書き込みは `merge_initial`（無い id の初期値差し込み・既存塊のバイト列不変）のみ** | 87・399・252・196 |
| 純粋層 | `src/report/bundle.rs` | `bundles(&[(EntryId, EntryId)]) -> Vec<Bundle>`＝無向の連結成分。束 id は構成 id の byte 最小。`alias_of`／`supersedes` は辺にしない | 129 |
| 純粋層 | `src/report/summary.rs`・`domain.rs` | `render_summary(catalog, ledgers, evidence, themes)`・`render_domain(ledger, themes)`。跨ぐ束は `crossing_bundles`、閉じた束は `closed_bundles`（両端が自ドメインの `links` だけ） | 341・234 |
| 検査 | `src/check/{mod,structure,content,freshness,finding}.rs` | `CheckInput`（catalog・ledgers・assignment・themes・evidence・domain_reports）→ `run()` → `Finding` 15 種。全体報告は**構造として届かない**（`CheckInput` に欄が無い） | 214・229・289・121・209 |
| 常時テスト | `tests/consistency/{mod,checks,non_vacuity,perturb,examples,values_md}.rs` | `RepoData::load()`（カタログ・台帳 4・報告 4・values.md・ソース全域）。`checks.rs` は所見 15 種の「実データで 0 件」＋「写しを 1 か所壊すと赤」。`examples.rs` は要件・README の ```toml 囲みにある `"ukadoc:…"` が正典に実在すること | 171・**893**・442・242・378・624 |
| 実行体テスト | `tests/cli_streams.rs` | `CARGO_BIN_EXE_ukadoc-survey` で副手続きを起動し標準出力／エラーを見る | 254 |

補足:
- **1 ファイル 1,000 行の番人**（`crates/log-capture-kit/tests/file_length_guard_test.rs`・`LINE_LIMIT`＝1000）が Rust ファイルを見張る。`checks.rs` は 893 行なので、判定 6 種とその摂動を同じファイルへ足すと確実に超える。新しい兄弟ファイル（例: `tests/consistency/documents.rs`）が要る。
- `io::paths` はコンパイル時の `CARGO_MANIFEST_DIR` からワークスペース根を決める。`examples.rs` は `.kiro/specs/completed/areka-P0-ukadoc-survey-toolkit/requirements.md` を根からの相対で読んでおり、**テストが `.kiro/specs/` を読む前例は既にある**（要件 11.1 ⑸ の brief 数え上げに使える）。
- 行末: `core.autocrlf=true`・`.gitattributes` 無し。台帳・報告は index が LF・作業ツリーが CRLF。`report`／`report-summary` は LF で書き出すが git 上は差分にならない（本日 `report-summary` を走らせて確認: 内容差分 0・改行の警告のみ。実行後に `git checkout` で戻した）。shiori のブリーフィングが警告する「`ledger-init` が台帳 16,144 行の行末を書き換える」は同じ事情で、**バイト比較する検査は割れるが git は割れない**。

### 2.2 台帳 4 本の欄の実測（2026-09-11・要件 Introduction 5 の再計測）

| 欄 | shiori 677 | assets 542 | sakura-script 342 | property 188 | 合計 |
|---|---|---|---|---|---|
| `links` 本数（持つ項目数） | 128（99） | 46（12） | 182（104） | 73（40） | 429（255）→ **関連 0 本の項目 1,494** |
| `values` 1 つ以上 | 348 | 331 | 71 | 3 | 753 → **空 996** |
| `priority` 頭文字 | A58 B61 C180 D160 E25 空193 | A112 B92 C103 D127 E104 空4 | C301 空41 | C186 空2 | 作り方がドメインごとに違う（要件どおり） |
| `owner` 空 | 670 | 365 | 265 | 2 | **1,302** |
| `owner` が完了済み spec | 0 | 63（window-placement 16・mayuna-compose 9・balloon-offset-dpi 6・package-mount 6・balloon-parse 5・shell-parse 5・balloon-vertical-canon 4・emo-atlas 4・windowposition-limit 3・bindoption-exclusivity 3・ghost-setup 1・scope-zorder-pinning 1） | 10（kero-balloon 3・sylphya 2・scope-zorder-pinning 2・cursor-tag-canon 2・sakura-dialogue-tags 1） | 2（sylphya 2） | **75** |
| `owner` が進行中 spec | 7 | 114 | 67 | 184 | 372 |
| `introduced` あり | 98 | 144 | 65 | 99 | 406（カタログの版番号持ち 406 と一致） |
| `note` に「壊れ方:」 | 677 | 542 | 322 | 188 | 状態が実装済み／語彙のみ／縮退／未対応の **1,552 件すべて**に在る |

`note` の「壊れ方:」は**自由文**で、「黙って壊れる／明示エラー／見た目の差」の 3 値を機械で切り出せる形にはなっていない（例: shiori `spec_dll`「できている範囲は明示的なエラーで守られており、できていない範囲は黙って壊れる」）。要件 4.1 ⑺・6.2 の「壊れ方の最悪値」は人手で束ごとに判定し、根拠の id を添える形になる。

### 2.3 機械の束の実測

| 出どころ | 束数 | 備考 |
|---|---|---|
| `report/summary.md`「ドメインを跨いで繋がった束」 | 75 | `| ukadoc:` 行を数えた |
| `report/shiori.md`「ドメイン内で関連が閉じている束」 | 25 | |
| `report/sakura-script.md` 同 | 23 | |
| `report/assets.md`・`report/property.md` 同 | 0・0 | 節はあるが行が無い |
| 合計 | **123** | |

- 123 束に現れる id の総数 **460**（内訳: 未対応 310・語彙のみ 79・実装済み 31・別名 20・対象外 15・縮退 5）。
- 最大の束は `ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1`（53 件）: `list_shiori_event` 31・`list_sakura_script` 6（`\![execute,createnar]`・`\![execute,createupdatedata]`・`\![execute,install,path,…]`・`\![execute,install,url,…]`・`\![update,…]`・`\![updateother,…]`）・`dev_bind`／`dev_nar`／`dev_ownerdraw`／`dev_shell`／`dev_update`・`manual_*` 8 ページ・`list_plugin_event:OnInstallComplete`・`list_shiori_resource:other_homeurl_override`・`descript_ghost:makoto`。**インストール・更新・オーナードローメニュー・翻訳（makoto）・着せ替え（bind）がページ単位の id を介して 1 つに繋がっている。**
- brief が例示する 3 連鎖の現状:
  - ⑴ 時刻: shiori 台帳が `list_plugin_event:OnSecondChange` の行に `configures → descript_plugin:secondchangeinterval` と `same-feature → list_shiori_event:OnSecondChange` を書いており、`summary.md` の束 `ukadoc:descript_plugin:secondchangeinterval_2c_79d2_6570:1`（assets・shiori）に 3 件が入る。**カタログに `descript_ghost` 側の `secondchangeinterval` は無い**（実在は `descript_plugin` のみ）。brief の「descript `secondchangeinterval`」は plugin の descript を指すものとして読める。
  - ⑵ 重なり順: property 台帳が `currentghost.seriko.zorder` から `same-feature → \![set,zorder,…]` を書くが、assets 台帳の `descript_shell:seriko.zorder` は `links` を持たず、`summary.md` に束として**出ていない**（2 台帳の対が跨いだ束になるはずだが現れないのは、sakura-script 側にも assets 側にも対応する `links` が無く、property→sakura-script の 1 本だけでは 2 件の束になり、`seriko.zorder`（assets）が繋がらないため）。要件 3.1 を満たすには assets か sakura-script の台帳に `links` を足す必要がある。
  - ⑶ インストール: `list_shiori_event:OnInstallComplete` ↔ `list_plugin_event:OnInstallComplete` は shiori 内の閉じた束。`dev_nar`・`\![execute,install,…]` は makoto の 53 件束の中。**3 つ目の連鎖は「makoto 束を分けた後に何が残るか」で決まる。**

### 2.4 ブリーフィング 4 本の申し送り（要件 8.2 の全数拾いの当たり）

| 文書 | 行数 | 「統合担当」 | 「裁定案」 | 「是正候補」 | 「申し送り」 | 主な節 |
|---|---|---|---|---|---|---|
| `briefing-shiori.md` | 1,404 | 5 | 0 | 0 | 3 | 「次に読む人への申し送り」⑴〜⑷・「是正の候補」1〜4 |
| `briefing-assets.md` | 863 | 2 | 0 | 3 | 0 | 「隣接 spec の是正候補」⑴〜⑹・「引き受け先が決まっていないもの」4 件・「上流の決まりと本 spec 自身の文書へ回すもの」⑺〜⒃ |
| `briefing-sakura-script.md` | 2,255 | 12 | 3 | 11 | 2 | 「⑷ 担当なし・裁定待ちの一覧」（裁定待ち 2 件・無所有 8 件）・「⑹ 既存の brief と `doc/COMPAT_ARCHITECTURE.md` §8 への是正候補」1〜7・「次に読む人への申し送り」⑴〜⑹ |
| `briefing-property.md` | 1,045 | 2 | 2 | 4 | 1 | 「⑶ 持ち主のいない項目」（2 件）・「⑷ 二重所有の裁定案」（未決 3 問）・「⑸ 既存 brief への是正候補」是正 1〜7＋README への提案 1（優先度の数値を 10 刻み） |
| 完了 spec の `tasks.md` | — | shiori 14・assets 6・sakura-script 5・property 4・toolkit 1 | | | shiori 31・assets 14・sakura-script 13・property 2・toolkit 24 | shiori `tasks.md` 5.1／5.2／7.2 に統合担当宛ての明示行あり |

要件 8.3 が名指しする項目はいずれも上の節に実在する（`OnUserInput*`／`OnTeach*` の境界・群 4 と群 8・`OnArchiveViewerOpen`・拡張イベント 7 件・`\![enter,selectrect]`・toolkit 宛て 5 件＝shiori `tasks.md` 7.2／assets ⒃・引受先なし 4 件／sakura-script 裁定待ち 2・無所有 8・`linkage.md` 不在／property `activeghostlist(…).ext`・記憶の非採用・三重所有）。語による検索だけでは `tasks.md` の「申し送り」84 件の多くが上流 toolkit 宛てや自 spec 内向けなので、**全数を拾ったうえで宛先で仕分ける手順**が要る。

### 2.5 既存 brief 27 本と roadmap.md

- `.kiro/specs/` 直下の `brief.md` は **28 本**（本 spec を含む）。`completed/` は 175 ディレクトリ。
- brief 本文に `ukadoc:` 形の id を書いているのは **5 本だけ**（`surfaces-basepos`・`status-execution-states`・`balloon-canon-residue` が各 1 件、`sylphya-set-ledger`・`balloon-font-descript-keys` は接頭辞のみ）。**「既存 brief が id 単位で所有を宣言している」（要件 7.4）に当たる brief はほぼ無く、所有は台帳の `owner` 欄（372 件・13 spec）の側に在る。** 要件 7.4 の「id 単位で所有を宣言」を「台帳の `owner` に既に書かれている」と読むのか「brief 本文に id がある」と読むのかで `owner` の扱いが大きく変わる（§6 判断 4）。
- roadmap.md の「ウェーブ編成」は W13（9 本＋任意 1）・W14（7）・W15（5）・W16（4）・W17（2）・保留 1。本 spec は W13 ⑥「`doc/ukadoc-coverage/` のみでコード非接触」と登記されている（実際は `crates/ukadoc-survey` にも触るが、同 crate を触る進行中 spec は無い）。
- 「M2 以降」節の予約群: SSTP・FMO・DirectSSTP・Plugin/HEADLINE・ネットワーク更新・ゴースト/バルーン選択 UI・多重ゴースト・Shift_JIS（charset-canon で W13）・SAORI（実装しない）・里々/YAYA 網羅・NAR・回転テキスト・バルーン美観配置・pasta native x64／`IShiori` in-proc・ベクトル描画・owner-draw メニュー（要件 10.8 の対応表の入力）。

### 2.6 M1 完成の実物

- `completed/areka-P0-emo2-conformance-e2e/design.md`「適合検証項目表（D4）」: 項目 1〜20 の表（起動と既定位置／起動挨拶／着せ替え／まばたき／自発会話／撫で／二重クリックでメニュー／シーン遷移／位置調整／二人立ち／利用者名／位置の永続化／終了／縮退／バルーン表示寿命／拡大率切替／掴んで追従／二体の隣接／再表示直後の重なり順／子プロセスへの受け渡し）。
- `verification/m1-completion.md` §6「未達と引受先」: 8 行（持ち越し 7・受容 1）。引受先は `dpi-transition-two-tick-bounce`（**W14**）・`present-gpu-transform-scale`（W13）・`kanade-boot-talkdone-drop`（W13）・`host32-window-thread-pump`（W13）・上流 `ekicyou/pasta`・`zorder-chain-residue` A-2・据え置き（初回位置調整）・受容（採り直し無し）。

## 3. Introduction の事実確認（実測との突き合わせ）

| # | 主張 | 実測（2026-09-11） | 結果 |
|---|---|---|---|
| 1 | `report/summary.md` 145 行・スナップショット 2026-08-24T04:08:57.881Z | 145 行・`generated_at` 一致・`report-summary` 再実行で内容差分 0 | ○ |
| 2 | 1,749＝88／440／22／1,002／27／170／未分類 0 | `summary.md` と台帳の数え直しが一致 | ○ |
| 3 | 跨ぐ束 75・閉じた束 25／23／0／0・合計 123・makoto 束 53 件 | すべて一致 | ○ |
| 4 | `linkage.md` 不在 | 不在。sakura-script ブリーフィング 2180 行付近に所見あり | ○ |
| 5 | `links`／`values`／`priority`／`owner`／`introduced` の全数値 | §2.2 のとおり全部一致（owner の完了済み spec は 75 行） | ○ |
| 6 | SAORI: カタログに `saori` 0 件・`spec_dll` の備考に成立条件 3 つ | 0 件。備考に「32bit の同じプロセスに同居すること・作業ディレクトリ・DLL の探索パスの 3 つ」 | ○ |
| 7 | 適合検証 20 項目・持ち越し 8 行・うち 4 件が **W13** の spec | 20・8 は一致。**`dpi-transition-two-tick-bounce` は W14**（roadmap #11・要件 9.4 も W14） | **△ Introduction 7 の「W13 の spec」が古い** |
| 8 | brief 28 本（本 spec 除き 27） | 28 本 | ○ |
| 9 | イベント 4%・未知キー無言の数値 | `report/shiori.md`・`report/assets.md` のページ別表と一致。`briefing-assets.md`「未知の記述の扱い」節は実在 | ○ |
| 10 | 所見 15 種・toolkit 要件 7.6 の除外理由 | `FindingKind::ALL` 15・toolkit 要件 7 の 6 項目に「並走 4 本が同じファイルを取り合う」 | ○ |
| 11 | ブリーフィング 1,404／863／2,255／1,045 行 | 一致 | ○ |
| 要件 2.5 | 「`report-summary` がドメイン別報告 4 本の改行を書き換える」 | `report_summary()` は `summary.md` しか書かない。ドメイン別 4 本を書くのは `report()`。行末の話は `write_lf`（LF）対 作業ツリー（CRLF）の差で、git 上は差分にならない | **△ 事実と違う。「`report` を走らせた後の改行差分」と読み替えるか、文言の訂正候補** |

## 4. 要件ごとのギャップ（要件 → 既存資産 → 種別）

種別: **Missing**＝作る必要がある／**Constraint**＝既存の作りが決める／**Unknown**＝設計で調べる

| 要件 | 使える既存資産 | ギャップ |
|---|---|---|
| 1 着手条件 | `summary.md`・`m1-completion.md` §4 サインオフ欄・`ls` | なし（文書作業）。1.5「写す時点で数え直す」は数え方の記録が要る |
| 2 報告の作り直し | `report`／`report-summary` 副手続き | Constraint: `report-summary` はソース走査を含む（証拠件数）。2.5 の文言 △ |
| 3 繋がりの補修 | `links` 欄・`bundles()`・`LinkEndpointMissing` 検査 | Missing: 3 連鎖のうち ⑵ は台帳に `links` が無い。Constraint: 束は無向なので**往復に書いても束は変わらない**（3.2 の往復は記録の対称性のためだけ）。makoto 束の分割（3.3）は `links` を削るか `linkage.md` で分けるかの 2 通り |
| 4 `linkage.md` | 束 id の安定性（構成 id の最小値）・`values` 和集合 | Missing: 文書そのもの。**Constraint: 機械の束に無い id が 1,127 件**（§5.1）。⑺ 壊れ方は自由文から人手で判定 |
| 5 段階の定義と写像 | brief の段階表・`values` | Missing: 文書。規則 3 つの適用は人手 |
| 6 4 軸の順位 | `note` 壊れ方・`values`・`links` | Unknown: ⑶ の参照値（里々／YAYA 標準テンプレート辞書の語彙）は ukadoc MCP に**辞書本文が無い**（§5.8）。⑷ 基盤共有度は人手 |
| 7 台帳への書き戻し | `ledger::read`・`blocks::split`（塊の範囲）・`tomlout` | **Missing: 既存項目の欄を書き換える経路が無い**（§5.3）。7.3 の前後表は `priority` 頭文字の集計＝機械化容易 |
| 8 `briefing.md`・申し送り処分 | 4 ブリーフィングと `tasks.md` | Missing: 文書。全数拾いは語の検索＋宛先の仕分け（§2.4） |
| 9 第二段 | D4 20 項目・§6 8 行 | なし（文書）。9.5 は 6.3 の参照値に依存 |
| 10 `roadmap-draft.md` | roadmap.md ウェーブ表・「M2 以降」節・`owner` 集計 | Missing: 文書。10.2 の「owner に持つ id 数」は機械化容易 |
| 11 判定 6 種 | `examples.rs`（文書中 id の実在）・`perturb.rs`・`non_vacuity.rs`・`render_summary` | Missing: 判定 6 種＋摂動＋母数。Constraint: `checks.rs` 893 行→新ファイル。⑹ はソース起因の赤（§5.5）。⑸ は完了時の自己移動（§5.6） |
| 12 非接触 | `io::paths`・`README.md`「誰が何を作り直すか」表 | なし。12.2 の「判定に要る読み込み・出力の副手続き」の範囲が §5.3 に効く |

## 5. 技術課題の詳細

### 5.1 束のカバー率（要件 4.2・11.1 ⑶）

- 状態が実装済み／語彙のみ／縮退／未対応の項目は 1,552 件。機械の束 123 個に現れる id は 460 件（このうち別名 20・対象外 15 は要件 4.3 で除くので 425 件）。**残り 1,127 件は関連を 1 本も持たず、どの束にも現れない。**
- 選択肢:
  - **(a) 名前付き束の構成 id は機械の束の外の id を含んでよい**（人手で足す）。`linkage.md` の ⑵「由来する機械の束 id」は「1 つ以上」の要件なので、束の核は機械の束、周辺は人手、という読み方が成り立つ。判定 ⑶（重ならない・全項目がどちらかに属する）はそのまま検査できる。台帳の `links` は増やさない。
  - **(b) `links` を大量に足して機械の束を広げる**。要件 3.2 の精神に近いが、1,100 件超の関連を書くのは要件の意図（「補修」）を超える規模で、`content` 検査（`LinkEndpointMissing`）と報告 5 本の作り直しが毎回要る。
  - **(c) 単独項目を許容する**（1,127 件の大半を「単独項目」に落とす）。要件 4.2・6 の「順位を決める単位を束にする」が形骸化する。
- 判定 ⑶ の設計はどの案でも同じ（名前付き束の id 集合が互いに素・単独項目一覧との和が 1,552 件と一致）。

### 5.2 makoto 束（53 件）の分割（要件 3.3）

- 53 件が繋がる原因はページ単位の id（`dev_nar`・`dev_update`・`manual_*` 等）を `same-feature`／`triggers` で多数のイベントが指していること（shiori 台帳・イベント行に書く向き）。
- 選択肢: **(a) `linkage.md` で分ける**（機械の束 id は 1 つ・名前付き束は複数・「どの関連が過剰か」を文章で書く。台帳は触らない）／**(b) 過剰な関連を `links` から削り `note` に残す**（機械の束が変わり、束 id も変わりうる＝`linkage.md` の引用が外れる）。要件 3.3 は (a) を基本にし (b) を任意としている。判定 ⑵（引用した機械の束 id が報告に実在）は (b) を選ぶと再生成のたびに確かめ直しになる。

### 5.3 台帳への書き戻し（要件 7）

- 1,552 件の `priority` と、`alias`／`not-applicable` 197 件の `priority = ""`、加えて `owner`・`note`・`links` の編集。
- 道具に「既存の塊の欄を置き換える」経路は無い。`blocks::split` が塊の範囲（`start`・`end`）を返し、`merge_initial` はその範囲外に差し込むだけである。
- 選択肢:
  - **(a) 手編集**（エディタ／使い捨てスクリプト）。要件 11.6 は「判定」を使い捨てに置くなと言うが、書き換え作業そのものは対象外。ただし 1,552 件を機械で置き換えるなら、置き換えの正しさ（他の欄が動かない・行末が保たれる）を確かめる検査は要る。
  - **(b) 副手続き `ledger-set-priority` 相当を道具に足す**（塊のバイト列のうち `priority = "…"` の 1 行だけを置き換える純粋関数＋入出力）。要件 12.2 の「判定に要る読み込み・出力の副手続き」に入るかは読み方次第。入れるなら在中テストとバイト不変の検査（`ledger-init` と同じ流儀）が要る。
  - 行末: `write_lf` で書くと作業ツリーが LF になるが git 上は差分にならない（§2.1）。ただし**同じ作業ツリーで `git diff` 以外の道具（バイト比較）を使う検査は割れる**。
- 数値の刻み: property は `C10`／`C20`／`C30`／`C90` の 10 刻み・assets は 1〜80 の通し（16 件ずつ等分）・shiori は群ごと。property ブリーフィングが README へ「10 刻み」を提案している。要件 7.1 は「同じ束の項目は同じ数値」なので、数値＝束の順位が自然（§6 判断 6）。

### 5.4 判定 6 種の配置と形（要件 11）

| 判定 | 入力 | 既存の前例 | 置き場の案 |
|---|---|---|---|
| ⑴ 文書中の項目 id が正典に実在 | 3 文書の本文・カタログ | `examples.rs` の `quoted_ids`／`toml_blocks` | 新ファイル。**id の拾い方**（引用符付き・囲みの中だけ・地の文も）を決める必要あり（§6 判断 7） |
| ⑵ 引用した機械の束 id が報告に実在 | 3 文書・報告 5 本（または台帳から `bundles()` で作り直し） | `bundles()` | 報告の本文を読むか台帳から作り直すか。作り直す側なら ⑹ と併せて「報告は台帳と一致」が前提になる |
| ⑶ 名前付き束が互いに素・全項目がどちらか一方 | `linkage.md`・台帳 4 本 | なし | `linkage.md` から「束名→構成 id」を機械で読める形が要る（§6 判断 7） |
| ⑷ 段階ごとの束数・項目数が `priority` 頭文字と一致 | `briefing.md` の表・台帳 | なし | 表の行を機械で読む形が要る |
| ⑸ brief 済み未完了 spec 数 | `roadmap-draft.md`・`.kiro/specs/*/brief.md` | `examples.rs` が `.kiro/specs/` を読む | **完了時の罠**（§5.6） |
| ⑹ `summary.md` が作り直した本文と一致 | `summary.md`・カタログ・台帳・証拠 | `freshness.rs`（ドメイン別）・`RepoData` に証拠あり | 統合テスト側で `render_summary` と直接比較（`CheckInput` に欄を足さない案）か、`CheckInput` に欄を足す案（§5.5） |

- 摂動（11.3）: `perturb.rs` の `Perturbed`（メモリ上の写し）を延ばす。文書の写しを 1 か所壊す（id を 1 文字変える・束から id を 1 つ抜く・件数を 1 ずらす）。
- 母数（11.4）: `non_vacuity.rs` の形（下限の定数＋関係の主張）。「文書に引用された id が 0 件でない」「名前付き束が 0 個でない」など。

### 5.5 全体報告を常時検査に入れる副作用（要件 11.1 ⑹・11.2）

- `render_summary` の末尾「ドメインごとの証拠あり件数」は**ソース木を歩いて**数える。常時検査に入れると、他の spec が正典 URL のコメントを 1 行足すだけで `summary.md` が古くなり赤になる。上流が除外した理由は「台帳の取り合い」だったが、**ソース起因の取り合いが新しく生まれる**（W13〜W17 の正典系 spec はどれも URL コメントを足す）。
- 選択肢: **(a) 全文一致**（要件の文言どおり・上の副作用あり・他 spec が `report-summary` を走らせる規約が要る＝README「誰が何を作り直すか」の改訂）／**(b) 証拠の表を除いた一致**（台帳だけの関数にする。`render_summary` を 2 分するか、比較側で末尾の表を切る）／**(c) `render_summary` から証拠の表を外す**（要件 2.3 の「証拠の有無だけを載せる」を別の場所へ）。(b)(c) は上流の設計判断（証拠件数の置き場）に触れる。

### 5.6 brief 数の判定と完了手続きの相互作用（要件 11.1 ⑸・1.3）

- `/kiro-complete` は本 spec を `completed/` へ移した**後に**全体テストを回す（`m1-completion.md` §7）。`.kiro/specs/` 直下の brief は 28→27 になる。「本 spec を除き 27」を「直下の brief 総数 − 1」として実装すると、移動後は 27−1＝26 で赤になる。判定は「直下の brief のうち本 spec 名を除いた数」（移動後も 27）とするか、「本 spec 自身の brief の有無に依らない数え方」を要件 1.3 の文言と揃える必要がある。
- 同じ罠は `examples.rs` の `REQUIREMENTS_MD` 定数が示している（完了手続きの手順 5-2 が `crates/` から feature 名を grep して書き換える）。文書 3 本の中に自 spec のパス（`.kiro/specs/areka-P0-ukadoc-coverage-roadmap/…`）を書けば同じ書き換えの対象になる。

### 5.7 文書の機械可読な形（要件 11.1 ⑴〜⑸）

- 3 文書は人が読む Markdown だが、判定が読むには「構成 id の全列挙」「段階ごとの束数」「brief 数」を取り出せる形が要る。
- 選択肢: **(a) 表の列を固定**（`| 束名 | 機械の束 id | 構成 id | … |`）／**(b) ```toml 囲み**（`examples.rs` と同じ拾い方・`[bundle."名前"] members = [...]`）／**(c) 別ファイル**（`linkage.toml` を機械の正本にし `linkage.md` は解説）。(c) は要件 4.1「8 つを 1 か所に書く」と「同じ数を 2 か所に持たない」（8.8）の間で位置づけが要る。

### 5.8 4 軸のうち ⑶「影響する既存資産の広さ」の参照値（要件 6.3・9.5）

- ukadoc MCP の検索（2026-09-11）: 里々 wiki は「ポストと狛犬（公式テンプレート）」「ゴーストキット」「ゴーストキット改良版Plus」、YAYA wiki は「はろーYAYAわーるど（紺野ややめ）」「SimpleYAYAテンプレート」「紺野りりす」を名指しし、`aya_shiori3.dic`（`OnAiTalk`・`OnSecondChange` 契機）や `OnFirstBoot`／`OnBoot`／`OnClose`／`OnGhostChanged`／`OnUserInput` の使い方の断片は引ける。**しかし辞書の全文（どのイベント・タグ・プロパティを使うか）はスナップショットに入っていない**（記憶にも「標準テンプレート辞書は入っていない」とある）。
- 選択肢: **(a) wiki の記述から引ける語彙だけを参照値にする**（限定的・出典は URL で示せる）／**(b) テンプレート辞書の実物を取得して語彙を数える**（実在ゴーストの走査に近く、要件 6.3 の「実在ゴーストの辞書走査と実走は行わない」との線引きが要る）／**(c) ⑶ を「正典が『必須』『既定』と書く項目」で代用**。要件ディスカッションの議題（§6 判断 8）。

### 5.9 関連の往復（要件 3.2）の実効

- `bundles()` は無向なので、片方向の `links` でも束は成立する。往復に書く効果は「両方の台帳から読める」ことだけで、ドメイン別報告の閉じた束には**跨ぐ関連はどのみち載らない**（両端が自ドメインのときだけ）。shiori 台帳は「イベント行に書く」向きに統一済み（申し送り⑴）。往復にすると同じ繋がりを 2 度数える読み違いが起きうる。要件 3.2 は往復を求めるが、機械的な意義が無いことを設計に書き添えるのが安全。

## 6. 設計判断事項（要件ディスカッションへ）

1. **名前付き束は機械の束の外の id を含んでよいか。** 1,552 件のうち 1,127 件は関連 0 本。含めないなら単独項目が 1,100 件超になり、束を単位にした順位付けが 425 件だけの話になる（§5.1）。
2. **makoto 束（53 件）の分割は `linkage.md` の宣言だけで行うか、`links` も削るか。** 削ると束 id が変わり報告の作り直しと引用の直しが要る（§5.2）。
3. **`priority` の書き戻し 1,552 件は手編集か、道具に「欄だけ置き換える」副手続きを足すか。** 要件 12.2 の「判定に要る読み込み・出力の副手続き」に入るか（§5.3）。
4. **`owner` の扱い。** 要件 7.4 の「brief が id 単位で所有を宣言している項目」は、brief 本文に id を書くものが 5 本しか無い現状では「台帳の `owner` に進行中 spec が既に書かれている 372 件」と読むしかない。完了済み spec を指す 75 件を残すか空にするか（§2.2・§2.5）。
5. **全体報告を常時検査に入れる形。** 全文一致（ソース起因で他 spec が赤になる）か、証拠の表を除いた一致か、証拠の表を `render_summary` から外すか（§5.5）。
6. **`priority` の数値の刻み方。** 束の順位を通し番号にするか、property の提案どおり 10 刻みにするか。README の記述を変えるなら要件 12.4 の README 編集範囲（11.2 の表と「一式」の追記に限る）を広げる必要がある（§5.3）。
7. **3 文書の機械可読な形。** 表の固定列か ```toml 囲みか別ファイルか。判定 ⑴ が拾う id の範囲（囲みの中だけか地の文も）も併せて決める（§5.7）。
8. **⑶「既存資産の広さ」の参照値の実体。** ukadoc MCP には辞書本文が無い。wiki の記述だけで測るか、テンプレート辞書の実物を読むか、正典の「必須／既定」で代用するか（§5.8）。
9. **brief 数の判定と完了時の自己移動。** 「本 spec を除く 27」の数え方を移動後も緑になる形に定める。文書内の自 spec パスの綴りも完了手続きの書き換え対象になる（§5.6）。
10. **要件 2.5 の文言。** 「`report-summary` がドメイン別報告の改行を書き換える」は事実と違う。訂正するか、「`report` 実行後の改行差分」と読み替えるか（§3）。
11. **Introduction 7 の「W13 の spec」。** `dpi-transition-two-tick-bounce` は W14。要件 9.4 は正しいので Introduction の訂正のみ。
12. **要件 3.2 の往復の意義。** 束には効かない（無向）。記録の対称性のためだけと明記するか、往復を求めないか（§5.9）。
13. **判定の新ファイル名と分割。** `checks.rs` 893 行に足せない。`tests/consistency/documents.rs`（判定）＋`documents_perturb.rs`（摂動）＋`documents_non_vacuity.rs`（母数）のような分割案。

### 6.1 上の 13 件の処分（要件ディスカッション 2026-09-11）

| # | 処分 | 反映先 |
|---|---|---|
| 1 | **開発者へ**（議題 1）——名前付き束の構成 id を機械の束の外まで人手で足すか、`links` を正本にするか | 要件 3・4 |
| 2 | 要件で確定: `linkage.md` の宣言だけで分け、`links` は削らない（束 id の安定） | 要件 3.3 |
| 3 | 要件 12.2 で「欄 1 行だけ置き換える副手続き」を許可。足すか手編集かは**設計** | 要件 12.2・設計 |
| 4 | 要件で確定: 進行中 spec 宛て 372 件は保つ・完了済み spec 宛て 75 件は状態で分ける（実装済み／縮退は保つ・未対応／語彙のみは空にして `note`） | 要件 7.4 |
| 5 | 要件で確定: 判定 ⑹ は台帳＋カタログ由来の本文だけ。証拠の表を残すか `evidence` へ移すかは**設計** | 要件 11.1 ⑹・11.2・設計 |
| 6 | 要件で確定: 段階内の束の順位を 1 から通し。README の欄の定義は変えない（10 刻み提案は処分台帳で却下） | 要件 7.1 |
| 7 | **設計**——3 文書の機械可読な形と、判定 ⑴ が拾う id の範囲 | 設計 |
| 8 | **開発者へ**（議題 2）——⑶「既存資産の広さ」の参照値の実体 | 要件 6.3・9.5 |
| 9 | 要件で確定: 数え方は本 spec のディレクトリ名を除いた数（移動前後で 27）。文書とテストに自 spec のパスを書かない | 要件 1.3・11.1 ⑸・12.8 |
| 10 | 要件で確定: `report-summary` は `summary.md` だけを書く。改行の差は手で直さず内容差分だけをコミット | 要件 2.5 |
| 11 | 要件で確定: `dpi-transition-two-tick-bounce` は W14 | Introduction 7 |
| 12 | 要件で確定: 往復に書かない（束は無向・2 度数える読み違いを避ける）。不足 1 本を既存の流儀の向きで足す | 要件 3.2 |
| 13 | **設計**——新ファイルの名前と分割（1,000 行の番人） | 設計 |

## 7. 実装アプローチ

### 案 A: 既存の形を延ばす（文書 3 本＋`tests/consistency/` に兄弟ファイルを足す）

- 文書: `doc/ukadoc-coverage/{linkage,briefing,roadmap-draft}.md` を新規。README は 11.2 の表と「一式」の追記だけ。
- 検査: `tests/consistency/` に新ファイル（判定 6 種・摂動・母数）。`RepoData` に 3 文書と `summary.md` の本文を足す（`mod.rs` の冒頭「`summary.md` は読まない」の但し書きを改める）。`CheckInput`／`FindingKind` は触らない（`check/` の純粋層に欄を足さない）。
- 台帳: 手編集（案 A-1）か、`generate.rs` に「欄だけ置き換える」副手続きを 1 つ足す（案 A-2）。
- 長所: 触る場所が最小・前例どおり・道具の純粋層に手を入れない。短所: 判定の実体がテスト側に住むので `cargo run -- check` からは見えない（要件 11.6 は「標準のテスト実行」で足りる）。

### 案 B: 判定を純粋層 `check/` に足す（`FindingKind` を 15→21 種へ）

- `CheckInput` に 3 文書と `summary.md` の本文を足し、`check/documents.rs` で判定、`FindingKind` に 6 種を追加、`cli check` でも出す。
- 長所: 実行体でも同じ所見が出る・所見の形（ファイル名と id の名指し）が既存と揃う。短所: 上流が「構造として届かない」と設計した `summary.md` の遮断を崩す（意図した変更だが、`freshness.rs` の冒頭の説明と `README` の表を書き換える範囲が広がる）。`content_link_tests.rs` 808 行・`finding_tests.rs` 850 行への追記も 1,000 行に近い。

### 案 C: 折衷（判定 ⑴〜⑸ はテスト側・⑹ は純粋層）

- ⑹ だけは `freshness.rs` の隣に `SummaryReportStale` として置き（`CheckInput` に `summary_report: Option<&str>` を足す）、文書 3 本の判定はテスト側に置く。
- 長所: 報告の新しさは全部 `freshness` に集まる。短所: 2 か所に分かれる理由の説明が要る。

## 8. 工数とリスク

| 部分 | 工数 | リスク | 理由 |
|---|---|---|---|
| 要件 1・2・9・10（文書・機械の作り直し） | S〜M | 低 | 入力が揃っている。9・10 は読み込みが多い |
| 要件 3・4（繋がりの補修・`linkage.md`） | **L** | **中** | 1,552 件の帰属決定＋makoto 束の分割＋3 連鎖の補修。判断 1 の答えで規模が倍以上変わる |
| 要件 5・6・8（段階・順位・ブリーフィング・申し送り処分） | M〜L | 中 | 4 軸のうち ⑶ の参照値が未定（判断 8）。申し送りの全数は 4 文書＋5 `tasks.md` |
| 要件 7（書き戻し 1,552 件） | M | 中 | 書き換え経路の有無（判断 3）と行末の扱い |
| 要件 11（判定 6 種＋摂動＋母数） | M | 低〜中 | 前例あり。⑹ の副作用（判断 5）と ⑸ の完了時の罠（判断 9） |
| 要件 12（非接触） | S | 低 | 触る場所は 2 か所 |

合計の見立ては **L**（brief の M を超える）。理由は要件 4.2 の全数帰属と要件 7 の全数書き戻しが、いずれも 1,552 件を相手にするため。

## 9. 設計フェーズへ持ち越す調査項目（Research Needed）

1. `linkage.md` の機械可読な形と、判定 ⑴ が拾う id の規則（囲みの中だけか）。
2. `render_summary` の証拠の表を切り離す可否（上流設計 D-11・要件 2.3 との整合）。
3. 「欄だけ置き換える」書き込みの純粋関数（`blocks::split` の範囲内で `priority = "…"` の 1 行を差し替え、他のバイトを保つ）の設計と、その在中テスト。
4. 里々「ポストと狛犬」・YAYA「はろーYAYAわーるど」「SimpleYAYA」の辞書が使うイベント・タグ・プロパティを ukadoc MCP の範囲で列挙できるか（できなければ判断 8 の代替）。
5. 3 連鎖 ⑵（`seriko.zorder`）の `links` をどの台帳に何本足せば 1 つの束になるか（assets または sakura-script 側に最低 1 本）。
6. makoto 束 53 件の関連の内訳（どの `kind` がページ id を介して繋いでいるか）を辺ごとに列挙し、分割の境界を決める材料にする。
7. `/kiro-complete` の手順 5-2 が書き換えるパスの綴りと、判定 ⑸ の数え方が移動後も緑になることの確認手順。
