# Requirements Document

> 本文の file:line と件数は **2026-09-18 の本ブランチでの実測値**。着手時に必ず引き直すこと。

## Introduction

### 誰が困っているか

areka を初めて手にする第三者。第三者のゴーストの多くはバルーンを同梱しない（ukadoc「インストール」は同梱を任意の形として挙げるのみ。ukadoc「全体の構成」は「バルーンはゴーストごとでなく、一つのベースウェアの元に統括的に管理され、全ゴーストで共有される」と定める）。SSP は本体に既定のバルーンを 2 つ同梱しているので、利用者はバルーンの存在を意識せずにゴーストを入れて動かせる。**areka には同梱バルーンが無い**ので、バルーンを持たないゴーストを入れた第三者は「バルーンが無い」で止まる。

### いま何が起きているか（本ブランチで実測・2026-09-18）

- **areka のバルーン資産**は作者自作の検証用 `emo2-kakukaku`（emo2 同梱・`crates/pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku/`）と、その検証用派生 2 つ（`emo2-kakukaku-offsetdpi`・`emo2-kakukaku-wplimit`）だけである。開発者裁定（2026-09-18）「同梱バルーンは癖が無いものが必要。`emo2-kakukaku` では厳しい」により、これらは既定バルーンにしない。
- **第三者が配布した検体の保管慣行**は `vendors/sample_ghost/<名>/` の**展開フォルダ**（`R_POST_and_KOMAINU` が前例）。同フォルダの `.gitattributes` が `* -text` で改行変換を止め、同フォルダの `.gitignore` がリポジトリ直下の `*_test.txt`／`*_dump.txt` 除外を打ち消している。`.nar` への畳み込みは並走する `areka-P0-nar-install` の仕事（同 brief 2026-09-18 追記「畳む対象に `vendors/sample_ghost/StayseeBalloon/` を含める」）。
- **バルーンの起動経路**は `areka.exe <ゴーストの根> <バルーンの根>`（`crates/areka/src/boot_config.rs` の `resolve_config_inputs`＝`args[2]` がバルーンの根）。フォルダを置くだけでコード変更 0 で表示できる。
- **areka の透過の扱い**は宣言に依らず固定である。バルーンの枠は `crates/areka-emo-present/src/balloon.rs` のモジュール doc が「PNG α 尊重（R5.2）: `use_self_alpha,1` 相当＝`UseSelfAlpha::On` で bake する」「`.pna` 対応は `probe_pna` の既存 seam に委ね本 spec では追加しない」と定め、起動時の焼き込み `crates/areka/src/emo2_boot/assets.rs` も `use_self_alpha: UseSelfAlpha::On` を固定で渡す。バルーン定義の読み手 `crates/areka-parsers/src/balloon/parse.rs` が引くキーは `windowposition.*`・`origin.*`・`wordwrappoint.*`・`validrect.*`・`font.*`（基底 14）・`disable.font.*`・`cursor.*`・`vertical` であり、`use_self_alpha`／`use_input_alpha`／`paint_transparent_region_black` は**読まない**（`crates/` の本番コードで 0 件）。開発者裁定（2026-09-18）「**areka は常に `use_self_alpha,1`。pna 対応は不要**」。
- **網羅台帳の現状**: `doc/ukadoc-coverage/ledger/assets.toml` の項目 `ukadoc:descript_balloon:use_self_alpha_2c_5024:1` は `status = "absent"`・`owner = ""`・`priority = "A15"`・備考「壊れ方: 黙って壊れる。記録: なし。areka はバルーン側の透過の扱いを宣言で切り替えられない」。束は `linkage.md` の `[bundle."絵の重ね方"]`。隣の `use_input_alpha_2c_6570_5024:1`・`paint_transparent_region_black_2c_6570_5024:1` も `absent`・`owner = ""`。台帳の宛先には検査（`crates/ukadoc-survey/tests/consistency/spec_checks.rs` 腕 c・f）があり、**宛先に書く spec 名は `doc/ukadoc-coverage/roadmap-draft.md` の `[[spec]]` 行（`owner_count` 付き・現在 27 行＝`[briefs].count = 27`）に載っていなければ赤**になる。
- **第三者告知** `THIRD-PARTY-NOTICES.md` は `cargo about` の自動生成（先頭に「手で編集しないでください」）。cargo 依存でない資産は載らない。第三者向け README は `areka-P0-alpha-release-signoff` が書く（本ブランチの `README.md` は開発者向けで、検体の出典は 1 件も載っていない）。
- **`doc/COMPAT_ARCHITECTURE.md` §8「沈黙ルール対応表」**は「項目｜裁量｜根拠｜出典 spec」の 4 列で、各 spec が実装着地時に自らの裁量を追記する。透過の扱いを登記した行は現在 0 行。

### 候補（2026-09-18 調査）

**`Balloon for Staysee Syncfield`**（id `StayseeBalloon`・作者 SSP BUGTRAQ＝SSP 本家の作者・`github.com/ponapalt/StayseeBalloon`）。LICENSE は **CC0-1.0**、readme 原文「■転載・再配布・同梱・改変等について　煮るなり焼くなり好きにしてください。License : CC0」「特定のゴーストを意識して作ったバルーンですが、専用指定はしていません」「半透明（アルファチャンネル）ONを前提に作っており、OFFではまともに見られません」。descript は `charset,Shift_JIS`・`type,balloon`・`use_self_alpha,1`・`use_input_alpha,1`・`validrect.left,22`／`top,20`／`right,-26`／`bottom,-47`・`font.height,12`・`vertical` **無し**。ファイルは `balloons0〜3.png`・`balloonk0〜1.png`・`balloonc0〜4.png`・`arrow0/1.png`・`online0〜8.png`・`marker.png`・`sstp.png`・`thumbnail.pnr`・`readme.txt`・`descript.txt`・`install.txt`・`LICENSE`。`.pna` 無し。唯一の注意事項「半透明前提」は areka の固定の扱いと一致する。

不採用の候補: SSP 同梱の「SSPデフォルト+」「balloon for Emily/P4」は再配布条件が公開されておらず借用の根拠が薄い。自作の無地バルーンは既製品より高くつくので、CC0 候補が見た目で不採用になった場合の**次善**として残す。

### 何を変えるか

CC0 の既製バルーンを `vendors/sample_ghost/StayseeBalloon/` に原作ファイル無改変で置き、areka で崩れずに表示されることを**新規の決定論テストだけ**で 1 周し（既存の検体参照ファイルには触らない）、開発者が実機で見た目を確認して採否を決める。あわせて出典と CC0 を記録し、「`use_self_alpha` は常に 1・`.pna` 非対応」を areka の裁量として §8 と台帳に登記し、既定バルーンの id を下流（`baseware-root-layout`・`nar-install`・`alpha-release-signoff`）へ申し送る。**本番コードの変更は 0 行**（id 定数の配線は `baseware-root-layout` が足す）。

## Boundary Context

- **In scope**（第三者・開発者・下流の spec から見える範囲）:
  - 既定バルーンの選定（候補 `StayseeBalloon`）と、開発者の実機目視による採否の記録。
  - `vendors/sample_ghost/StayseeBalloon/` への展開フォルダ保管（原作ファイル無改変・バイト保存・取得元の記録）。
  - StayseeBalloon を検体にした表示検証——決定論テスト（新規ファイルのみ）＋実機目視 1 度（表示スケール k≠1 を含む）。崩れは areka 側の欠陥として本仕様内で直すか、引受先を実在確認して先送りする。
  - 出典・CC0 の記録（本仕様の `verification/`）と、第三者向け README への申し送り。
  - 裁量の登記: `doc/COMPAT_ARCHITECTURE.md` §8 に 1 行、台帳 `assets.toml` の `use_self_alpha` 項目の状態・備考・宛先、`roadmap-draft.md` の `[[spec]]` 行、報告の作り直し。
  - 既定バルーン id `StayseeBalloon` の下流への申し送り。
- **Out of scope**:
  - `.pna`・`use_self_alpha,0`・`use_input_alpha`・`paint_transparent_region_black` の実装（読まない・常に 1 の裁量）。台帳の `use_input_alpha`・`paint_transparent_region_black` の 2 項目は**触らない**（`absent`・宛先空のまま＝変更 0）。
  - `thumbnail.pnr` の透過解釈（列挙は `thumbnail.png` のみ・`.pnr` は将来の `baseware-root-layout` 拡張）。
  - 既定バルーン id の定数と解決順への配線（`areka-P0-baseware-root-layout` A1-② が足す）。
  - `.nar` への畳み込みと共有ヘルパ（`areka-P0-nar-install` A0-①）。
  - 配布 zip の生成・第三者向け README・`THIRD-PARTY-NOTICES.md`（`areka-P0-alpha-release-signoff`。本仕様は「入れるもの」と「書くべき出典」を決めて申し送るだけ。`THIRD-PARTY-NOTICES.md` は手で編集しない＝本仕様の変更 0）。
  - バルーンの `homeurl` によるネットワーク更新（`areka-P0-network-update`）。
  - 複数の既定バルーン（SSP は 2 つ・α は 1 つ）・`recommended.balloon` による案内（α 後）。
  - `sstpmessage.*`・`number.*`・`onlinemarker`・`communicatebox.*`・`arrow*`・`online*`・`marker.png`・`sstp.png`（StayseeBalloon に在るが areka が読まない・使わない資産。表示に使わないことは検証で確かめるが、実装は `areka-P0-balloon-canon-residue` 等・α 後）。
  - 縦書き（StayseeBalloon の descript に `vertical` が無いので横書きのみ。縦書きでの検証は 0 件）。
  - 既存の検体（`emo2-kakukaku` とその派生 2 つ）の内容変更・既存テストの期待値変更。
- **Adjacent expectations**:
  - **並走**: `areka-P0-nar-install`（A0-①）と `areka-P0-popup-menu-minimal`（A0-②）。本仕様は `vendors/sample_ghost/StayseeBalloon/`・新規テストファイル・`verification/`・`COMPAT_ARCHITECTURE.md` §8・台帳 3 文書だけに触り、`nar-install` が書き換える検体参照ファイル（実測 38）と共有 0 を保つ。
  - **下流**: `areka-P0-nar-install`（畳む対象と共有ヘルパの検体名に `StayseeBalloon` を足す）・`areka-P0-baseware-root-layout`（解決順の最後に既定バルーン id を渡す定数 1 つ）・`areka-P0-alpha-release-signoff`（zip に `balloon/StayseeBalloon/` を入れる・README に出典と CC0 と「半透明前提のバルーンだけが正しく表示される」既知の制限を書く）。
  - **前提**（完了済み）: `areka-P0-balloon-parse`・`balloon-vertical-canon`・`balloon-font-descript-keys`（descript の読み手）・`areka-P0-kero-balloon`（`balloonk*`）・`areka-P0-charset-canon`（descript の `charset,Shift_JIS` の復号）。

## Requirements

### Requirement 1: 既定バルーンの選定と採否

**Objective:** areka を配布する開発者として、第三者に渡しても恥ずかしくない「癖の無い」既定バルーンを 1 つ、再配布条件が確かなものから選びたい。そうすれば、バルーンを持たないゴーストを入れた第三者が最初の 1 歩で止まらない。

#### Acceptance Criteria

1. The 本仕様 shall 既定バルーンの候補を `Balloon for Staysee Syncfield`（id `StayseeBalloon`・CC0-1.0）とし、選定の根拠（再配布条件が LICENSE と readme の両方で確認できること・専用指定が無いこと・半透明前提が areka の固定の扱いと一致すること）を本文に書き残す。
2. When 候補を `vendors/sample_ghost/StayseeBalloon/` に置いた, the 開発者 shall `areka.exe <ゴーストの根> <バルーンの根>` で起動して実機で見た目を 1 度確認し、採否を裁定する（判断の観点は `font.height,12` の小ささと薄い青の色味が「癖が無い」に足るか）。
3. When 開発者が採用を裁定した, the 本仕様 shall その裁定（日付・観点・結論）を `verification/` の記録に書き、Requirement 2 以降を `StayseeBalloon` で進める。
4. If 開発者が不採用を裁定した, then the 本仕様 shall 次善の「自作の無地バルーン」へ切り替える前に本要件書を改訂し（候補名・保管フォルダ名・id・出典の記述を差し替える）、改訂前の候補で作った資産を残さない。
5. The 本仕様 shall 「SSPデフォルト+」「balloon for Emily/P4」を候補にしない理由（再配布条件が公開されていない）と、`emo2-kakukaku` を既定にしない理由（開発者裁定「癖が強い」）を本文に書き残す。

### Requirement 2: リポジトリでの保管——原作ファイル無改変の展開フォルダ

**Objective:** 検体を管理する開発者として、既定バルーンを既存の検体と同じ場所・同じ形で、原作のバイト列そのままに保管したい。そうすれば、`.nar` へ畳む側（`nar-install`）が畳む対象を 1 つ増やすだけで済み、ハッシュによる陳腐化検出も成り立つ。

#### Acceptance Criteria

1. The 本仕様 shall StayseeBalloon を `vendors/sample_ghost/StayseeBalloon/` の**展開フォルダ**として保管し、`.nar` は作らない（畳み込みは `areka-P0-nar-install` の仕事）。
2. The 保管フォルダ shall 原作の配布物に含まれる全ファイル（`descript.txt`・`install.txt`・`readme.txt`・`LICENSE`・`thumbnail.pnr`・`balloons0〜3.png`・`balloonk0〜1.png`・`balloonc0〜4.png`・`arrow0/1.png`・`online0〜8.png`・`marker.png`・`sstp.png`）を、**1 バイトも変えず**・1 つも欠かさず・余分なファイルを加えずに含む。areka が読まないファイル（`thumbnail.pnr`・`online*`・`marker.png`・`sstp.png`・`balloonc*`・`arrow*`・`install.txt`）も削らない。
3. The 本仕様 shall 取得元（リポジトリ URL・取得したコミットのハッシュ・取得日・readme に書かれた版と日付）を `verification/` の記録に書き、保管したフォルダの全ファイルのハッシュ一覧を同じ記録に置く（陳腐化と改変の検出の基準）。
4. The 保管フォルダ shall `vendors/sample_ghost/.gitattributes`（`* -text`）の効力の下に置かれ、`git check-attr text` が全ファイルで `unset` を返す（改行変換 0）。
5. The 本仕様 shall 保管する全ファイル名を リポジトリ直下の `.gitignore`（`*_test.txt`／`*_dump.txt`）と照合し、大小無視で当たる名前が **0 件**であることを記録に書く（当たる名前があれば `vendors/sample_ghost/.gitignore` の打ち消しで拾う）。
6. If 取得した配布物の中身が本文の候補の記述（ファイル一覧・descript の要点・LICENSE の種別）と食い違う, then the 本仕様 shall 本文を実測に合わせて是正してから保管し、食い違いの内容を記録に書く。
7. The 本仕様 shall 既存の検体（`R_POST_and_KOMAINU`・`emo2` とその派生バルーン 2 つ）の中身と置き場所を 1 バイトも変えない。

### Requirement 3: areka で崩れずに表示される——決定論テスト

**Objective:** 第三者として、既定バルーンで喋るゴーストの文字が枠から出たり欠けたりしないでほしい。開発者として、その保証を実機に頼らず毎回のテストで持ちたい。

#### Acceptance Criteria

1. The 本仕様 shall StayseeBalloon を検体にした決定論テストを**新規のテストファイルだけ**で足し、既存のテスト・example（検体パスを参照する実測 38 ファイルを含む）を 1 行も変えない。
2. The 新規テスト shall 検体フォルダのパスを **1 か所の定数**にだけ持ち、`nar-install` が共有ヘルパへ寄せるときに 1 行の付け替えで済む形にする。
3. When StayseeBalloon の `descript.txt`（`charset,Shift_JIS`）を読んだ, the areka shall `type,balloon`・`id,StayseeBalloon`・`name,Balloon for Staysee Syncfield`・`validrect.left,22`／`top,20`／`right,-26`／`bottom,-47`・`font.height,12` を宣言どおりに読み取り、`vertical` は未指定（横書き）として扱う。
4. When StayseeBalloon の枠画像を本体側（`balloons*`）と相方側（`balloonk*`）で焼き込んだ, the areka shall 面 0 をどちらの側でも解決し、透過部分（α）を保ったまま合成する（半透明前提の画像がそのまま見える）。
5. When StayseeBalloon の枠に半角のみ・全角のみ・半角全角混在の本文を流し込んだ, the areka shall 各行を `wordwrappoint` を超えたら折り返し、`validrect` を 1 画素も超えずに描く（既存の検体で固定している折返し・描画範囲の観測と同じ観測を StayseeBalloon で取る）。
6. When StayseeBalloon の枠に選択肢（`\q`）と遅延座標指定（`\_l`）を含む台本を流し込んだ, the areka shall 選択肢の目印と本文の位置が `validrect` の内側に収まり、`\_l` の指定位置に本文が置かれる。
7. When 表示スケール k≠1（少なくとも 1 つ・既存の検体で使っている値）で StayseeBalloon を配置した, the areka shall 枠の寸法と `validrect`・`origin`・`wordwrappoint` の座標を同じ k で拡大し、k=1 と同じ行数・同じ折返し位置になる。
8. The 新規テスト shall StayseeBalloon に在って areka が使わない資産（`balloonc*`・`arrow*`・`online*`・`marker.png`・`sstp.png`・`thumbnail.pnr`）が焼き込みの列挙に**載らない**こと、および載らないことが `error!` を 1 件も出さないことを固定する。
9. If 上の 3.3〜3.8 のいずれかで崩れが出た, then the 本仕様 shall バルーンを改変して合わせるのではなく **areka 側の欠陥**として本仕様内で直すか、引受先の spec が生きている（`.kiro/specs/` 直下に実在し `completed/` でない）ことを確認してから先送りし、どちらにしたかを `verification/` に書く。
10. The 新規テスト shall 既存の検体（`emo2-kakukaku` とその派生）で固定している観測の期待値を 1 つも緩めない（既存テストが赤になったら期待値を書き換えず原因を直す）。
11. The 新規テストファイル shall 1 ファイル 1,000 行以下（`file_length_guard_test.rs` の番人が緑）とし、例外表には触らない。

### Requirement 4: areka で崩れずに表示される——実機目視

**Objective:** 決定論テストが隠す欠陥（色味・にじみ・実機の DPI 切替）を、開発者が 1 度は自分の目で確かめたい。

#### Acceptance Criteria

1. When 決定論テストが緑になった, the 開発者 shall 実機で StayseeBalloon を既定の表示スケール（k=1）と k≠1 の 2 通りで 1 度ずつ表示し、半角・全角・選択肢の見え方を目視する。
2. The 本仕様 shall 目視の証跡（スクリーンショットは GPU 合成窓のため読み戻し画像か、読み戻しが無ければ実機の観察記録）と、表示に使ったゴースト・表示スケール・日付を `verification/` に残す。
3. If 目視で崩れが見つかった, then the 本仕様 shall Requirement 3.9 と同じ扱い（areka 側の欠陥として直す／引受先を実在確認して先送り）で処理し、直した場合は同じ崩れを再現する決定論テストを足す。

### Requirement 5: 出典と CC0 の記録

**Objective:** 配布物を受け取る第三者として、同梱されたバルーンが誰のもので、どんな条件で入っているかを知りたい。開発者として、CC0 に帰属義務は無くとも礼儀として出典と作者名を書きたい。

#### Acceptance Criteria

1. The 本仕様 shall 本仕様の `verification/` に、資産名（`Balloon for Staysee Syncfield`）・id（`StayseeBalloon`）・作者（readme・descript の `craftman` に書かれた名）・ライセンス（CC0-1.0）・出典 URL（`github.com/ponapalt/StayseeBalloon`・descript の `homeurl`）・取得コミットと日付を 1 か所に書く。
2. The 本仕様 shall 第三者向け README に載せるべき文（出典・作者・CC0・「areka は半透明前提のバルーンだけが正しく表示される」という既知の制限）を同じ記録に**そのまま写せる形**で用意し、`areka-P0-alpha-release-signoff` の brief に申し送りとして 1 段追記する。
3. The 本仕様 shall `THIRD-PARTY-NOTICES.md` を手で編集しない（自動生成ファイルであり、cargo 依存でない資産は載らない＝本仕様の変更 0）。
4. The 本仕様 shall 原作の `readme.txt`・`LICENSE`・`install.txt` を保管フォルダにそのまま含めることで、配布物側でも出典と条件が読める状態にする（Requirement 2.2 の再確認・追加の告知ファイルを保管フォルダの中に**作らない**）。

### Requirement 6: 透過の扱いの裁量を登記する

**Objective:** バルーンを作る作者と、areka を保守する開発者として、「`use_self_alpha` を 0 と書いても効かない・`.pna` は読まない」が areka の意図した裁量であることを、正典沈黙箇所の対応表と網羅台帳の両方で読めるようにしたい。そうすれば「黙って壊れる」ではなく「意図して固定している」と分かる。

#### Acceptance Criteria

1. The 本仕様 shall `doc/COMPAT_ARCHITECTURE.md` §8 の表に 1 行を足し、項目「バルーンの `use_self_alpha`／`use_input_alpha`／`paint_transparent_region_black` と `.pna`」・裁量「宣言を読まず常に `use_self_alpha,1` 相当（PNG の α をそのまま尊重）で焼く。`.pna` は読まない」・根拠（開発者裁定 2026-09-18・`areka-emo-present/src/balloon.rs` と `emo2_boot/assets.rs` の固定値・半透明前提のバルーンだけが正しく表示されるという既知の制限）・出典 spec（本仕様）を書く。
2. The 本仕様 shall 台帳 `doc/ukadoc-coverage/ledger/assets.toml` の項目 `ukadoc:descript_balloon:use_self_alpha_2c_5024:1` を `status = "degraded"`（動くが正典どおりではない＝宣言に依らず常に 1）・`owner = "areka-P0-default-balloon-bundle"` に改め、備考に「どう違うか」（`0` と書いたバルーンでも 1 として扱う・記録は出ない・裁量は §8 に登記）を書く。`priority` は変えない。
3. The 本仕様 shall 台帳の隣の 2 項目（`use_input_alpha_2c_6570_5024:1`・`paint_transparent_region_black_2c_6570_5024:1`）を**変えない**（`absent`・宛先空のまま。理由: 本仕様は既定バルーンの資産が主で、これらは §8 の 1 行で裁量が読めれば足りる。宛先を本仕様にすると完了時に実装済みか縮退が求められる）。
4. When 台帳の宛先に本仕様の名前を書いた, the 本仕様 shall `doc/ukadoc-coverage/roadmap-draft.md` の `[[spec]]` に本仕様の行（`name`・`stage = "A"`（項目の優先度 `A15` の段階）・`bundle = "絵の重ね方"`（`linkage.md` に実在する束名）・`owner_count` は台帳 4 本を数え直した実数（本仕様の裁定どおりなら 1）・`wave` は正本 `roadmap.md` のウェーブ表記 `A0`）を足し、`[briefs].count` を行数に合わせて 1 増やす。
5. When 台帳を触った, the 本仕様 shall ドメイン別報告と全体報告（`cargo run -p ukadoc-survey -- report` と `report-summary`）を作り直し、`cargo test -p ukadoc-survey` の常設の検査（判定 ⑸ の腕 a〜f を含む）が緑であることを記録に書く。
6. The 本仕様 shall 本番コードに正典 URL のコメントを**足さない**（`degraded` は証拠を要求されない・本番コードの変更 0 を保つ）。

### Requirement 7: 下流への申し送り——既定バルーンの id

**Objective:** 根の解決順を作る `baseware-root-layout` と、`.nar` へ畳む `nar-install` と、配布 zip を作る `alpha-release-signoff` の実装者として、既定バルーンの id とフォルダ名を 1 か所で確かめたい。

#### Acceptance Criteria

1. The 本仕様 shall 既定バルーンの id `StayseeBalloon`（descript の `id`）とフォルダ名 `StayseeBalloon`（`install.txt` の `directory`）が同じ綴りであることを実測で確かめ、`verification/` と本仕様の brief に「既定バルーン id ＝ `StayseeBalloon`」と書き残す。
2. The 本仕様 shall 本番コードに既定バルーン id の定数を**置かない**（定数と解決順への配線は `areka-P0-baseware-root-layout` が 1 行で足す）。
3. When 保管と検証が終わった, the 本仕様 shall `areka-P0-nar-install`（畳む対象と共有ヘルパの検体名）・`areka-P0-baseware-root-layout`（既定 id）・`areka-P0-alpha-release-signoff`（zip の `balloon/StayseeBalloon/`・README の出典文）の各 brief に申し送りを 1 段ずつ追記し、追記先が `.kiro/specs/` 直下に実在することを確かめる。
4. If 申し送り先の spec が既に `completed/` へ移っている, then the 本仕様 shall その spec には追記せず、`.kiro/steering/roadmap.md` の当該 spec の行に申し送りを書く。

### Requirement 8: 非回帰と制約

**Objective:** 並走する 2 本の spec と、既存の 7,600 本超のテストを壊さずに着地させたい。

#### Acceptance Criteria

1. The 本仕様 shall 本番コード（`crates/*/src/` の非テストファイル）の変更を **0 行**とする。
2. The 本仕様 shall 新規の外部依存を **0** とし、`Cargo.toml` を 1 つも変えない。
3. The 本仕様 shall `areka-P0-nar-install` が書き換える検体参照ファイル（実測 38）と `areka-P0-popup-menu-minimal` の接触面（`input_events/`・新規 `menu.rs`）に触れず、共有ファイル 0 を保つ。
4. When 本仕様の変更を取り込んだ, the ワークスペース shall `cargo test --workspace` が着手前と同じ本数で緑になり（i686 helper が要るテストは既存の手順どおり）、`cargo fmt --check` と 1,000 行の番人が緑である。
5. The 本仕様 shall 保管フォルダ・新規テスト・`verification/`・`COMPAT_ARCHITECTURE.md` §8・台帳 3 文書（`assets.toml`・`roadmap-draft.md`・報告）・隣接 brief への申し送り以外のファイルに触れない。
