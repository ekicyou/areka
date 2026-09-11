# Requirements Document

## Project Description (Input)

ukadoc 網羅調査 6 本の最終＝統合 spec。完了した調査 4 本（shiori 677・assets 542・sakura-script 342・property 188＝1,749 項目）の台帳はドメイン別に閉じており、ドメインを跨ぐ繋がり（設定キー ↔ イベント ↔ プロパティ ↔ タグ ↔ リソース）は誰の台帳にも収まっていない。M2 以降のロードマップは「M1 完成後に実物を見て組み直す」と定められており、その材料は ⑴ M1 完成宣言（2026-09-11・`emo2-conformance-e2e`）が残した実物と ⑵ 正典全体に対する網羅台帳の 2 つである。

- **誰が困っているか**: M2 以降の着手順を決める開発者と、`/kiro-discovery` 再入で次の開発 spec を起票する担当。1,749 項目のうち未対応 1,002 件をどの順で、どの束で、どの段階に置けばよいかを、台帳の id で根拠が引ける形で言える文書が無い。
- **現状**: 全体報告 `doc/ukadoc-coverage/report/summary.md` は機械で作れるが、束の名付け・段階 A〜E の最終決定・既存 brief の位置づけ・M2 予約群の順序は誰も決めていない。4 本の調査が統合担当へ送った申し送り（裁定待ち・是正候補・道具の穴）は 4 本のブリーフィングに散在している。
- **何が変わるか**: `doc/ukadoc-coverage/` に、⑴ 作り直した全体報告、⑵ 束の名付けと解説 `linkage.md`、⑶ 段階と優先順の統合ブリーフィング `briefing.md`、⑷ 網羅ロードマップ草案 `roadmap-draft.md` を置き、台帳 4 本の仮置きの段階を確定値に書き戻す。文書が述べる件数と束の構成は標準のテスト実行で機械が判定する。areka の実行時コードには触れない。

## Introduction

本 spec は「ukadoc 網羅調査」6 本のうちの統合 spec である。上流の調査 4 本（`ukadoc-survey-shiori`・`-assets`・`-sakura-script`・`-property`）と道具 1 本（`ukadoc-survey-toolkit`）はすべて `.kiro/specs/completed/` に封じられており、**統合担当に宛てた申し送りの引受先は本 spec しか無い**。

brief（2026-09-02）は二段構えを定めた——第一段（台帳が揃い次第）と第二段（M1 完成後）。**2026-09-11 の時点で両段とも着手条件を満たしている**（roadmap.md の spec 台帳 #6「survey 4 本 ✅・e2e ✅＝両段とも解禁済み」）。したがって本要件は両段を 1 本の spec として扱い、第二段の順位付けは M1 完成宣言の実物（適合検証項目表 20 項目と持ち越し 8 件）で根拠付ける。

### 着手時の実測（2026-09-11・brief の記載に対する補正）

brief は 2026-09-02 の値で書かれている。要件は以下の実測値を正とする。

1. **全体報告の実在パスは `doc/ukadoc-coverage/report/summary.md`**（145 行）である。brief の `report.md` は実在しない綴りであり、本要件で置き換える。作り直す副手続きは `cargo run -p ukadoc-survey -- report-summary`。ドメイン別の報告は `report/{shiori,assets,sakura-script,property}.md` の 4 本で、`cargo run -p ukadoc-survey -- report` が作り直す。元にしたスナップショットの生成日時は 2026-08-24T04:08:57.881Z。
2. **状態の分布**（`summary.md` から）: 1,749 ＝ 実装済み 88／語彙のみ 440／縮退 22／未対応 1,002／別名 27／対象外 170／未分類 **0**（4 台帳とも 0）。
3. **機械が出している束**: ドメインを跨いだ束は `summary.md` に **75**（`| ukadoc:` で始まる行を数えた）。ドメイン内で閉じた束はドメイン別報告に shiori **25**・sakura-script **23**・assets **0**・property **0**（各報告の「ドメイン内で関連が閉じている束」節の行数）。合計 **123 束**。最大の束は `ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1` を束 id とし、`makoto`・nar・更新・インストール・オーナードローメニューの正典ページと `OnUpdate*` 系イベントが 1 つに繋がっている（構成 id 53 件）。機械の連結成分は人手で分けなければ束として使えない。
4. **`doc/ukadoc-coverage/linkage.md` は存在しない**（`ls doc/ukadoc-coverage/` に無い）。README は「束の名付けと解説は統合担当が `linkage.md` に書く」と定め、調査 4 本はいずれも「作らず更新もしない」と要件に書いている。sakura-script 調査は「要件 11.5 が名指しする `linkage.md` が実在しない」を所見として統合担当へ渡している。
5. **台帳の欄の埋まり方**（4 台帳を機械で数えた）:
   - `links`: assets 46 本（12 項目）・property 73 本（40 項目）・sakura-script 182 本（104 項目）・shiori 128 本（99 項目）。**関連を 1 本も持たない項目が 1,494 件**ある。
   - `values`（テーマ）: assets 331 項目・property 3 項目・sakura-script 71 項目・shiori 348 項目に 1 つ以上。**テーマが空の項目が 996 件**。sakura-script の「触れ合い 0・記憶 0」と property の「記憶 0」は取りこぼしではなく、調査側が理由を書いて意図的に空にしたものである。
   - `priority`（段階＋数値）: **ドメインごとに作り方が違う**。assets は順位を 16 件ずつ A〜E に等分（A 112／B 92／C 103／D 127／E 104／空 4）、shiori は索引の群ごとの仮置き（A 58／B 61／C 180／D 160／E 25／空 193）、sakura-script と property は **C だけ**（C 301／空 41、C 186／空 2）。assets 調査は「段階の中身は roadmap の A〜E 登記の名前と合っていない」と申し送っている。**段階の頭文字を 4 台帳の間で比べることは現状できない。**
   - `owner`（担当 spec）: 空が assets 365・property 2・sakura-script 265・shiori 670 ＝ **1,302 件**。埋まっている行の宛先には完了済み spec（`window-placement`・`mayuna-compose`・`sylphya`・`cursor-tag-canon` 等）も混じる。
   - `introduced`: 埋まっているのは 144＋99＋65＋98 ＝ **406 件**で、カタログが版番号を持つ 406 件と一致する。
6. **SAORI の行は台帳に無い**。カタログに `saori` を含む id は **0 件**（`grep -ic saori catalog.toml`）。brief の「台帳では `not-applicable`」は実現できない記述であり、実際には shiori 台帳の `ukadoc:spec_dll` の備考（群 14c）が「areka はプロトコルを実装せず、成立条件は 32bit 同一プロセス・作業ディレクトリ・DLL 探索パスの 3 つ」と書いている。
7. **M1 完成の実物**: 適合検証項目表は **20 項目**（brief の言う 14 項目に上流の申し送りで 6 項目を追補したもの。唯一の置き場は `.kiro/specs/completed/areka-P0-emo2-conformance-e2e/design.md` の D4 の表）。完成判定は `verification/m1-completion.md`（2026-09-11・開発者サインオフ）。同 §6 に**持ち越し 8 行**（7 件持ち越し・1 件受容）があり、うち 4 件は起票済み spec（W13 の `present-gpu-transform-scale`・`kanade-boot-talkdone-drop`・`host32-window-thread-pump` と W14 の `dpi-transition-two-tick-bounce`）が引受先である。
8. **brief 済み未完了 spec は 28 本**（`.kiro/specs/` 直下の `brief.md` を数えた。`completed/` を除く）。本 spec 自身を除くと **27 本**。brief の「13 本」は 2026-09-11 の棚卸⑬（XL 3 本の分割・新規 6 本）で古くなった。27 本の着手順は roadmap.md の「ウェーブ編成」が W13〜W17＋保留 1（`tick-gate-adoption`）として持っており、**本 spec はそれを入力として扱い、書き換えない**。
9. **段階 A の主障壁**（brief が「要件フェーズで台帳から確定」とした 2 つ）を台帳で確かめた:
   - 「イベント 4%」——`list_shiori_event` 290 件 ＝ 実装済み **11**／語彙のみ 3／未対応 **273**／別名 3（実装済みは 3.8%）。`list_shiori_resource` 159 件 ＝ 実装済み 1／語彙のみ 158。**確定**。
   - 「未知キー無言」——`descript_ghost` 74 件 ＝ 実装済み 7／語彙のみ 1／未対応 66、`descript_balloon` 162 件 ＝ 20／5／縮退 4／未対応 133、`descript_shell` 102 件 ＝ 11／2／89、`descript_shell_surfaces` 137 件 ＝ 4／57／縮退 4／未対応 68／別名 4。未知の記述が無言で捨てられる経路は `briefing-assets.md`「未知の記述の扱い」節が実測している。**確定**。
10. **道具の常時検査**は所見 15 種を持つが、`summary.md` は構造として検査に届かない（完了 spec toolkit 要件 7.6・理由は「並走 4 本が同じファイルを取り合う」）。shiori 調査は「この古さは常時検査では永久に赤にならない」と申し送っている。**並走 4 本はすべて完了したので、除外の理由は消えている。**
11. **調査 4 本のブリーフィング**は shiori 1,404 行・assets 863 行・sakura-script 2,255 行・property 1,045 行で、各 1 か所以上に「統合担当」宛ての申し送り・裁定案・是正候補を持つ。

### 上流から継承する契約（本 spec は再定義しない）

| 契約 | 出典 |
|---|---|
| 台帳の項目形式・状態の 7 語・関連の種別 6 つ・テーマ 8 つ・ドメインの分割は凍結。欄を足さない | `.kiro/specs/completed/areka-P0-ukadoc-survey-toolkit/requirements.md` 要件 2・3・4・付録 A／`doc/ukadoc-coverage/README.md` |
| 優先度の根拠は 4 つで序列固定（壊れ方 ＞ 伺からしさ ＞ 資産の広さ ＞ 基盤共有度）。1 で差が付いたら 2 以下は見ない | README「優先度 — 根拠は 4 つ、序列は固定」（要件 4.7 が README を正本と定める） |
| テーマの付与規則は 1 つ（「無いと利用者は何を失うか」に答えられるものだけ） | `doc/ukadoc-coverage/values.md` |
| 報告は機械が作る。手で編集しない。束 id は人の文書から引用し、報告へ説明を書き足さない | README「4. 報告の扱い」 |
| `alias` の行は実装状況を持たない（写像先の正典行に委ねる） | README「状態の 7 語」 |
| 段階 A〜E の最終決定は統合担当が 4 台帳を見て行う。調査 4 本の値は仮置き | README「段階（A〜E）の最終決定はここでは行わない」 |
| M2 以降は M1 完成後に実物を見て組み直す。M2 予約群は brief を起票せず、先頭ウェーブ分だけ `/kiro-discovery` 再入で just-in-time に起票する | `.kiro/steering/roadmap.md`「M2 以降」節・仮裁定 2 |
| SSP の実測に頼らず、意味論は ukadoc から引く | 開発者方針（2026-09-05） |

## Boundary Context

- **In scope**:
  - 全体報告 `report/summary.md` とドメイン別報告 4 本の作り直し（機械）。
  - 4 台帳の `links`・`values`・`priority`・`owner`・`note` の統合のための編集（形式は凍結のまま）。
  - 束の名付けと解説 `doc/ukadoc-coverage/linkage.md`（新規）。
  - 段階 A〜E の定義と優先順の統合ブリーフィング `doc/ukadoc-coverage/briefing.md`（新規・第一段の草案と第二段の確定を同じファイルで持つ）。
  - 網羅ロードマップ草案 `doc/ukadoc-coverage/roadmap-draft.md`（新規）。
  - 上の 3 文書が述べる件数・id・束の構成を標準のテスト実行で判定する検査（道具 `crates/ukadoc-survey` の側に置く）。
  - 調査 4 本が統合担当へ送った申し送りの全数の処分。
  - 既存 27 brief の位置づけと、台帳 id 単位の是正候補の列挙。
- **Out of scope**:
  - 実装（areka の実行時の挙動は 1 ビットも変えない）。
  - `.kiro/steering/roadmap.md` の書き換え（棚卸セッションで一括裁定）。
  - 候補 spec の brief の生成（`/kiro-discovery` 再入が下流で行う。本 spec はその入力を作る）。
  - 既存 27 brief と調査 4 本のブリーフィングの書き換え（是正は本 spec の文書の側に候補として書く）。
  - SSP との実機比較・実在ゴーストの辞書走査と実走検証（例外は要件 6.3 の標準テンプレート辞書 2〜3 本の静的読み取りだけ。実走はしない）。
  - M2 の技術選定（pasta native x64・`IShiori` in-proc・ベクトル描画・AI）の順位付け。
- **Adjacent expectations**:
  - `/kiro-discovery` 再入は `roadmap-draft.md` の先頭ウェーブの節（束の名前・構成 id・候補 spec 名）をそのまま brief の材料に使える。
  - 棚卸セッションは `roadmap-draft.md` の段階 A〜E と M3 候補の受入基準を roadmap.md の「M2 以降」節へ取り込む判断材料として読める。
  - 本 spec が完了しても道具の常時検査（`cargo test -p ukadoc-survey`）は緑のまま残り、以後に台帳や新規 3 文書が食い違えば赤になる。
  - W13 の他 spec と共有するファイルは 0（roadmap.md 干渉台帳）。本 spec が触るのは `doc/ukadoc-coverage/` と `crates/ukadoc-survey` だけで、後者は他のどの進行中 spec も触らない。

## Requirements

### Requirement 1: 着手条件と実在パスの確定

**Objective:** 統合担当として、上流の成果物が揃っていること・brief の綴りが実物と食い違う点を先に確定したい。古い綴りや古い数を文書に持ち込まないためである。

#### Acceptance Criteria

1. When 本 spec の作業を始める, the 統合調査 shall 4 台帳の未分類が 0 件であること・`report/summary.md` が実在すること・`emo2-conformance-e2e` が `completed/` にあり `verification/m1-completion.md` に開発者の署名があることの 3 つを確かめ、確かめた日付と方法を `briefing.md` の冒頭に書く。
2. The 統合調査 shall brief の `doc/ukadoc-coverage/report.md` を実在パス `doc/ukadoc-coverage/report/summary.md` に読み替え、新規 3 文書のどこにも `report.md` の綴りを書かない。
3. The 統合調査 shall brief の「M2 ゲート brief 13 本」を、着手時に `.kiro/specs/` 直下（`completed/` を除く）の `brief.md` のうち本 spec 自身のディレクトリ名を除いて数えた実数（2026-09-11 時点で 27 本）に読み替え、その数え方を `roadmap-draft.md` に書く。数え方は本 spec が `completed/` へ移った後も同じ値（27）を返すものとする（本 spec の brief の有無に依らない）。
4. If 上流の成果物のいずれかが着手時に欠けている, then the 統合調査 shall 欠けている物の絶対パスを添えて止まり、代替の値を推測で書かない。
5. The 統合調査 shall 本文書「着手時の実測」の各値を成果物へ写すとき、写した時点で数え直し、数え直した値と手順を書く（実測値は書いた時点の写真であり、引き算や引用で導かない）。

### Requirement 2: 全体報告とドメイン別報告の作り直し

**Objective:** 読み手として、1,749 項目の状態分布がドメイン別・テーマ別・SSP 世代別に機械の出力として揃っていてほしい。ブリーフィングの根拠表として引用するためである。

#### Acceptance Criteria

1. When 台帳 4 本のいずれかを編集した, the 統合調査 shall `cargo run -p ukadoc-survey -- report` と `cargo run -p ukadoc-survey -- report-summary` を走らせ、台帳と報告 5 本を同じコミットに入れる。
2. The 統合調査 shall `report/summary.md` と `report/<ドメイン>.md` を手で編集せず、食い違いは作り直しで解消する。
3. The 統合調査 shall 状態分布の「SSP 世代別」の見方をドメイン別報告 4 本の「SSP 世代別の対応表」節に委ね、全体報告に無い表を新規 3 文書へ手書きで写さない（世代別の合計を述べるときは 4 本の表から数え、数えた手順を添える）。
4. The 統合調査 shall `summary.md` の「テーマ別の状態分布」節を `briefing.md` の根拠表として引用し、写しを作らない。
5. When `report` または `report-summary` を走らせた, the 統合調査 shall 作業ツリーの改行の違い（副手続きは LF で書き、作業ツリーは CRLF）を手で直さず、`git diff` に出る内容の差分だけをコミットに入れ、常時検査が緑であることを確かめてから進む（`report-summary` が書くのは `summary.md` だけであり、ドメイン別 4 本を書くのは `report` である）。

### Requirement 3: 繋がりの補修と束の再生成

**Objective:** 統合担当として、ドメインを跨ぐ連鎖が台帳の `links` に往復で書かれていて、機械の束としてそのまま出る状態にしたい。人手の名付けが機械の一覧から引用できる形になるためである。

#### Acceptance Criteria

1. The 統合調査 shall brief が例示する 3 つの連鎖——⑴ plugin の descript `secondchangeinterval`（カタログに在るのは `descript_plugin` 側だけで、ghost の descript には無い）↔ `OnSecondChange` ↔ plugin `OnSecondChange`、⑵ descript `seriko.zorder` ↔ `\![set,zorder]` ↔ `currentghost.seriko.zorder`、⑶ install.txt ↔ nar ↔ `OnInstallComplete` ↔ `\![execute,install,...]`——が、それぞれ 1 つの機械の束（または `linkage.md` が名付けた 1 つの束）に収まることを、束 id と構成 id で示す。
2. When ドメインを跨ぐ連鎖が機械の束として現れない（例: `descript_shell:seriko.zorder` は `links` を持たず、property→sakura-script の 1 本だけでは繋がらない）, the 統合調査 shall 不足している関連を片方の台帳に 1 本書き（相手 id はカタログから写す。向きは各台帳の既存の流儀に従い、shiori はイベント行に書く）、同じ繋がりを往復で 2 度書かない（束は向きを持たないので往復に意味が無く、2 度数える読み違いを生む）。書き足した本数をドメインごとに `linkage.md` に書く（0 本のドメインも 0 と書く）。
3. When 機械の束が異なる機能を 1 つに繋いでいる（例: `makoto` を束 id とする 53 件の束）, the 統合調査 shall その束を `linkage.md` で複数の名前付き束に分け、分けた各束が由来する機械の束 id と、どの関連（`kind` と両端の id）が過剰な繋がりだったかを書く。分割のために台帳の `links` を削らない（機械の束 id を安定に保ち、文書からの引用が作り直しで外れないようにする）。
4. The 統合調査 shall `links` に書く関連の種別を README の 6 つに限り、`alias_of`・`supersedes` の欄だけで結んだ対を束として扱わない。
5. The 統合調査 shall `links` を編集した後の常時検査（`cargo test -p ukadoc-survey`）が緑であることを、編集のたびに確かめる。
6. The 統合調査 shall `links` の追加をドメインを跨ぐ連鎖の骨格（brief の例示 3 連鎖と、名前付き束の核になる跨ぐ関連）の不足分に限り、名前付き束への帰属を表すためだけの関連（同じ設定ページのキー同士の同居など、README の種別 6 つに当てはまらない関係）を `links` に書かない（帰属の正本は `linkage.md`・開発者裁定 2026-09-11 議題 1）。

### Requirement 4: 束の名付けと解説（`linkage.md`）

**Objective:** 読み手として、ドメインを跨ぐ連鎖が「機能の束」として人の言葉で名付けられ、束ごとに「成立に要る最小の基盤」と「束が欠けると壊れる既存ゴーストの振る舞い」が読めてほしい。順位を決める単位を束にするためである。

#### Acceptance Criteria

1. The 統合調査 shall `doc/ukadoc-coverage/linkage.md` を新規に作り、名前付き束ごとに次の 8 つを 1 か所に書く: ⑴ 束の名前（機構で切る）、⑵ 由来する機械の束 id（0 個以上・報告から引用。機械の束を核にし、核を持たない束は「人手のみ」と書く）、⑶ 構成 id の全列挙（機械の束に現れない id を人手で足してよく、足した id には人手の印を付けて数を書く）、⑷ 跨ぐドメイン、⑸ 成立に要る最小の基盤、⑹ 束が欠けると壊れる既存ゴーストの振る舞い（利用者から見える結果の差で書く）、⑺ 壊れ方の最悪値（黙って壊れる／明示エラー／見た目の差のいずれか。構成 id の `note` から引く）、⑻ テーマの集合（構成 id の `values` の和集合）。
2. The 統合調査 shall 状態が `implemented`・`vocabulary-only`・`degraded`・`absent` のいずれかである項目の全数（2026-09-11 時点で 1,552 件）について、各項目がちょうど 1 つの名前付き束に属するか、または「単独項目」の一覧に属するかを `linkage.md` で決め、単独項目の件数をドメインごとに書く（0 件のドメインも 0 と書く）。帰属の正本は `linkage.md` であり（開発者裁定 2026-09-11 議題 1）、関連を 1 本も持たない項目（2026-09-11 時点で 1,494 件・うち順位の対象 1,127 件）は台帳の `links` を足さずに名前付き束へ人手で入れる。機械の束に現れる id の数・人手で足した id の数・単独項目の数の 3 つを `linkage.md` に書き、合計が対象の全数と一致することを示す。
3. The 統合調査 shall 状態が `alias`・`not-applicable` の項目（27 件・170 件）を束の構成から除き、除いた件数と理由を `linkage.md` に書く。
4. The 統合調査 shall 束の名前に「たぶん」「重要そう」等の推量の語を使わず、⑹ を書けない束を名付けない（書けないものは単独項目に落とし、理由を書く）。
5. The 統合調査 shall SAORI について、台帳の行が無いこと（カタログに `saori` を含む id が 0 件）を明記したうえで、`ukadoc:spec_dll` の備考にある成立条件 3 つ（32bit 同一プロセスの同居・作業ディレクトリ `ghost/master`・DLL 探索パス）を「段階 A の検証項目」として `linkage.md` の専用の節に書き、実装項目としてどの束にも入れない。
6. The 統合調査 shall M2 の技術選定（pasta native x64・`IShiori` in-proc・ベクトル描画・AI）を束として名付けず、`linkage.md` に「別軸」として 1 節だけ置いて理由を書く。
7. The 統合調査 shall 束の解説を `linkage.md` にだけ書き、報告 5 本へ説明を書き足さない。

### Requirement 5: 段階 A〜E の定義と束から段階への写像

**Objective:** 開発者として、製品品質の段階が「利用者が体験できる節目」で名付けられ、束がどの段階に入るかの規則が固定されていてほしい。順位が人の気分で動かないためである。

#### Acceptance Criteria

1. The 統合調査 shall 段階 A〜E を次の節目で定義し、`briefing.md` の冒頭に表として置く: A「そこにいて、触れて、話す」・B「迎えて、育てて、見送る」・C「察してくれる」・D「仲間がいる」・E「周辺」。
2. The 統合調査 shall brief の段階表（開発者裁定 2026-09-02 議題 7）が定めた束の配置を初期値とする——A: 起動と挨拶・会話・撫で・メニュー・終了・自分から喋る（ランダムトーク・時報・分）・名前を尋ねて覚える／B: nar インストール（D&D 含む）・ネットワーク更新・シェル／バルーン切替・オーナードローメニュー・消滅／C: スリープ復帰・バッテリー・スクリーンセーバー・フルスクリーン退避・最小化・ディスプレイ変化・サウンド／D: 多重ゴースト・コミュニケート・呼び出し・SSTP・FMO・PLUGIN・`x-ukagaka-link`／E: 外部アプリ Ex・開発者機能・トランスレータ・ヘッドライン。
3. The 統合調査 shall 「更新」のテーマを持つ束を段階 B の先頭に置き、`system.*` の照会を段階 C の末尾に置く。
4. The 統合調査 shall 束から段階への写像の規則を次の 3 つに固定し、`briefing.md` に書く: ⑴ テーマ 1 つ＝同じ段階の中で先頭群へ、⑵ テーマ 2 つ以上＝段階を 1 つ繰り上げてよい、⑶ テーマ 0 かつ壊れ方が見た目の差以下＝段階 E の候補。
5. If 台帳の根拠（壊れ方・テーマ・構成 id）が 5.2 の初期配置と食い違う, then the 統合調査 shall 初期配置を黙って変えず、「初期配置」「台帳が示す配置」「差の理由（id 付き）」を並べた裁定候補として `briefing.md` に書き、確定は開発者の裁定に委ねる。
6. The 統合調査 shall 段階の写像を書き終えた時点で、名前付き束の全数がちょうど 1 つの段階を持つことと、単独項目の全数が段階を持つことを `briefing.md` の表で示す（段階ごとの束数と項目数。0 の段階も 0 と書く）。
7. The 統合調査 shall M3「伺かの冠」の受入基準の候補として「テーマ 8 つすべてで代表束が実装済み」を `roadmap-draft.md` に登記し、決定は階梯の議論（棚卸セッション）に委ねる旨を添える。

### Requirement 6: 優先順の根拠と順位付け

**Objective:** 開発者として、束の順位が固定序列の 4 つの根拠から機械的に導かれ、根拠が台帳の id で引けてほしい。「たぶん重要」で並べた順を排するためである。

#### Acceptance Criteria

1. The 統合調査 shall 順位の根拠を README の 4 つ（⑴ 壊れ方 ＞ ⑵ 伺からしさのテーマ ＞ ⑶ 影響する既存資産の広さ ＞ ⑷ 依存基盤の共有度）に限り、序列を入れ替えない。
2. The 統合調査 shall 各束の順位の行に 4 つの根拠の値と、⑴ ⑵ の値が由来する構成 id を書く（⑴ は `note` の「壊れ方:」の記述、⑵ は `values` の和集合）。
3. The 統合調査 shall ⑶「影響する既存資産の広さ」の参照値を、里々／YAYA の標準テンプレート辞書（里々「ポストと狛犬」・YAYA「はろーYAYAわーるど」または「SimpleYAYA」の 2〜3 本に限る）の配布物を取得して辞書ファイルを静的に読み、使われているイベント・タグ・プロパティ・設定キーをカタログの id に写して数えたものとする（開発者裁定 2026-09-11 議題 2）。配布元の URL・取得日・辞書ファイル名・id への写し方を `briefing.md` に書き、配布物そのものはリポジトリに入れない。ukadoc MCP の里々／YAYA wiki は裏取りに使う（wiki には辞書の本文が無い）。テンプレート以外の実在ゴーストの辞書走査と、テンプレートを含むいかなるゴーストの実走も行わない。
8. If テンプレート辞書の配布物が取得できない, then the 統合調査 shall ukadoc MCP の wiki が名指しする語彙だけを参照値にし、退路を使ったことと取得できなかった配布元を `briefing.md` に書き、⑶ が空欄になった束を「根拠不足」の一覧（6.6）に置く。
4. The 統合調査 shall ⑵ を「よく使う」ではなく「無いと伺かでなくなる」として測り、⑶ で拾えない象徴的だが稀な束（例: 名前を尋ねて覚える）を落とさない。
5. The 統合調査 shall ⑷「依存基盤の共有度」を「その基盤 1 つで成立する束の数」として数え、数えた束の名前を書く。
6. The 統合調査 shall 順位の行に「たぶん」「おそらく」「重要そう」の語を書かず、根拠の値が空欄の束を順位表に載せない（載せられない束は「根拠不足」の一覧に id 付きで置く）。
7. If 2 つの束が 4 つの根拠すべてで同じ値になる, then the 統合調査 shall 同順位として並べ、同順位の解消を先頭ウェーブの選定（要件 10）の段で開発者の裁定候補に上げる。

### Requirement 7: 台帳への確定値の書き戻し

**Objective:** 読み手として、確定した段階と順位が台帳の `priority` 欄に 4 ドメイン共通の作り方で入っていてほしい。ブリーフィングの件数を台帳から機械で数え直せるようにするためである。

#### Acceptance Criteria

1. The 統合調査 shall 状態が `implemented`・`vocabulary-only`・`degraded`・`absent` の項目の全数について、`priority` を「確定した段階 1 文字（A〜E）＋段階内の順位を表す数値」で書き戻す（数値は段階内の束の順位を 1 から通しで振り、同じ束の項目は同じ数値。同順位の束は同じ数値を持ち、その次の順位は 1 増やす＝密な順位 1,2,2,3。単独項目も 1 つの束として番号を持つ）。形式は凍結された「段階 1 文字＋数値」のまま変えず、README の欄の定義も変えない（property 調査の「10 刻み」提案は要件 8 の処分台帳で却下の理由を書く）。
2. The 統合調査 shall 状態が `alias`・`not-applicable` の項目の `priority` を `""` にし、その件数（別名 27・対象外 170）を `briefing.md` に書く。
3. The 統合調査 shall 書き戻しの前後で 4 台帳の作り方の違い（assets の 16 件等分・shiori の群ごとの仮置き・sakura-script と property の C のみ）が解消されたことを、ドメイン別の段階分布の表（前／後）で示す。
4. The 統合調査 shall `owner` を次の規則で扱う: ⑴ 台帳に既に書かれている進行中 spec の宛先（2026-09-11 時点で 372 件・13 spec）は保つ、⑵ 完了済み spec を指す宛先（同 75 件）は、その項目の状態が `implemented` または `degraded` なら「実装した spec の記録」として保ち、`absent` または `vocabulary-only` なら空にして理由を `note` に書く（完了済み spec は未対応の項目を引き受けられない）。残した宛先の spec 名と件数を `briefing.md` の骨組みに列挙し、判定はその列挙だけを見る（生きた `completed/` の全走査で他 spec の完了に連動しない）、⑶ brief がまだ無い候補 spec の名前を `owner` に書かない（候補 spec への割り当ては `roadmap-draft.md` の側に持つ）、⑷ 規則 ⑴〜⑶ で `owner` を変えた件数を `briefing.md` に書く（0 も書く）。
5. When 台帳の `note` に段階や順位の根拠を書き足す, the 統合調査 shall 既存の記述を消さず、行番号を書かない。
6. The 統合調査 shall 書き戻しの後に `cargo test -p ukadoc-survey` が緑であることを確かめ、赤になった所見の種別と直し方を作業記録に残す。

### Requirement 8: 統合ブリーフィング（`briefing.md`）と申し送りの処分

**Objective:** 開発者として、「各項目間の繋がりを評価して分類し、製品品質に必要だと思われる順に実装項目を洗い出したブリーフィング」を 1 本で読みたい。4 本のブリーフィングを読み直さずに次の判断ができるためである。

#### Acceptance Criteria

1. The 統合調査 shall `doc/ukadoc-coverage/briefing.md` を新規に作り、次の節を持たせる: ⑴ 着手条件の確認（要件 1）、⑵ 段階の定義と写像の規則（要件 5）、⑶ 段階ごとの順序付き束一覧（要件 6。束名・順位・4 つの根拠・構成 id 数・`linkage.md` への参照）、⑷ 段階 A の主障壁の確定（本文書「着手時の実測」9 の数を台帳から数え直して書く）、⑸ 根拠表（`summary.md` テーマ別の状態分布への参照）、⑹ 申し送りの処分台帳、⑺ 第二段の改訂記録（要件 9）、⑻ 既存 brief への是正候補（要件 10.7 への参照）。
2. The 統合調査 shall 調査 4 本のブリーフィング（`briefing-{shiori,assets,sakura-script,property}.md`）と完了 spec の `tasks.md` に書かれた統合担当宛ての申し送りを全数拾い（少なくとも「統合担当」「裁定案」「是正候補」「申し送り」の語で 4 本と 4 spec の `tasks.md` を検索する）、1 件ごとに「出典（文書名と節名）・内容の要約・処分（採用／却下／開発者の裁定候補）・処分の理由」を ⑹ に書く。
3. The 統合調査 shall ⑹ に少なくとも次を含める: shiori——「掛け合い」と「交わり」の境界（`OnUserInput*`／`OnTeach*`）、群 4 と群 8 でテーマの扱いが逆向きであること、`OnArchiveViewerOpen` の決め方、拡張イベント「7 件」を件数の根拠に使わないこと、`\![enter,selectrect]` の正典側の誤記、`summary.md` の作り直しの時機、上流 toolkit 宛てだった 5 件（版番号の抽出が 2 桁版を落とす・証拠なし所見の因果・`ledger-init` の罠 2 つ・証拠の重複）／assets——段階の名前が roadmap の A〜E と合わないこと、優先度の作り方が成果物に無いこと（⒃）、引き受け先の無い 4 件／sakura-script——裁定待ち 2 件（`\![embed,...]` と `\![move]` の分担）、無所有 8 件（`\_!`・`\_+`・`\_?`・`\__c`・`\__q`・`\__t`・`\__v`・`\_n`）、`linkage.md` 不在、「触れ合い 0・記憶 0」が構造的であること／property——`activeghostlist(…).ext` 系 4 件の SET の読み、テーマ「記憶」の非採用（2 件か最大 36 件か）、三重所有（roadmap 仮裁定 1「案 甲」との整合）。
4. When 申し送りの処分が「採用」である, the 統合調査 shall 反映先（台帳の id・新規 3 文書の節）を書き、反映したことを常時検査または本 spec の検査（要件 11）で確かめられる形にする。
5. When 申し送りの処分が「開発者の裁定候補」である, the 統合調査 shall 3 行の前置き（何が問題か・何を決めるか・答えで利用者から見える結果がどう変わるか）を添え、答えで作業が変わらないものは裁定候補にせず自分で決めて理由を書く。
6. The 統合調査 shall 裁定待ち 2 件（`\![embed,...]`・`\![move]`）を統合担当として裁定し（調査側の裁定案を採るか、採らない理由を書く）、結果を該当 id の `owner`・`note` と ⑹ に書く。
7. The 統合調査 shall `briefing.md` を平易な語で書き、利用者から見える結果の差で語る（作業中だけで通じる符牒や隠語を持ち込まない）。
8. The 統合調査 shall 4 本のブリーフィングの本文を写さず、参照で済ませる（同じ数を 2 か所に持たない）。

### Requirement 9: 第二段——M1 完成の実物で順位を直す

**Objective:** 開発者として、順位が「M1 完成後に実物を見て組み直す」方針どおりに、適合検証の結果と持ち越し事項で裏付けられていてほしい。憶測で先に書いた順を採用しないためである。

#### Acceptance Criteria

1. The 統合調査 shall 第一段の順位（要件 6）を「草案」と明記したうえで、第二段で適合検証項目表 20 項目（`.kiro/specs/completed/areka-P0-emo2-conformance-e2e/design.md` D4 の表）の各行と、`verification/m1-completion.md` §6 の持ち越し 8 行を 1 行ずつ読み、順位を変える束・変えない束を決める。
2. When 第二段で束の順位または段階を変える, the 統合調査 shall 「変更前・変更後・理由」を書き、理由には適合検証項目の番号（1〜20）または持ち越し行の見出しを引用する。
3. When 第二段で順位を変えない, the 統合調査 shall 「20 項目と持ち越し 8 行のいずれも順位を動かす根拠にならなかった」と明記し、確かめた項目番号を列挙する（変えなかったことを沈黙で表さない）。
4. The 統合調査 shall 持ち越し 8 行のうち既に spec が起票済みの 4 件（W13 の `present-gpu-transform-scale`・`kanade-boot-talkdone-drop`・`host32-window-thread-pump` と W14 の `dpi-transition-two-tick-bounce`）を新たな束にせず、既存 brief の位置づけ（要件 10）に載せる。
5. The 統合調査 shall 段階 A の温度感を「当面 emo2 が動けばよい」とし、段階 A の束が M1 の emo2 を里々／YAYA 製の代表 2〜3 体へ一般化するときに壊れる項目を、6.3 の参照値（標準テンプレート辞書の語彙）で id 単位に示す。
6. The 統合調査 shall 「外部から『このゴーストを動かして』という要望が来た時点で参照元・検証対象をそのゴーストに切り替える」旨を `briefing.md` に書き、現時点では切り替え先が無いこと（候補があるとすれば自作の「どっとさくら」）を明記する。

### Requirement 10: 網羅ロードマップ草案（`roadmap-draft.md`）

**Objective:** 棚卸セッションと `/kiro-discovery` 再入の担当として、段階 A〜E をマイルストーン候補に写し、束→候補 spec→依存順→ウェーブ案が台帳 id 付きで読みたい。先頭ウェーブ分の brief をそのまま起票できるためである。

#### Acceptance Criteria

1. The 統合調査 shall `doc/ukadoc-coverage/roadmap-draft.md` を新規に作り、段階 A〜E を M2 以降のマイルストーン候補として並べ、段階ごとに束（`linkage.md` の名前）→候補 spec 名→依存順→ウェーブ案を書く。
2. The 統合調査 shall 既存の brief 済み未完了 spec の全数（着手時に数えた実数・2026-09-11 時点で 27 本）について、各 spec が「どの段階のどの束に属するか・台帳で `owner` に持つ id の数・roadmap.md のウェーブ（W13〜W17／保留）」を 1 表に書き、どの束にも属さない spec があればそう書く（0 本なら 0 と書く）。表は着手時の写真であり（撮った日付を添える）、生きた総数との一致は主張しない（判定は表の各名前の実在と行数の一致。要件 11.1 ⑸）。新しい brief の登記先は roadmap.md の spec 台帳であって本文書ではない。
3. The 統合調査 shall roadmap.md のウェーブ編成（W13〜W17）を入力として扱い、並べ替えを提案するときは「現在のウェーブ・提案・理由（id 付き）」を裁定候補として書き、roadmap.md 自体は編集しない。
4. The 統合調査 shall 先頭ウェーブ（M2 の最初のウェーブ）に入れる束を名指しし、束ごとに候補 spec 名の案・構成 id の全列挙・依存する既存 spec・`/kiro-discovery` 再入の入力になる 3 行の要約（問題・現状・何が変わるか）を書く。
5. The 統合調査 shall 先頭ウェーブより後の束について brief を作らず、名前付き束と候補 spec 名の案のまま置く（spec 工場化しない）。
6. The 統合調査 shall M2 の技術選定（pasta native x64・`IShiori` in-proc・ベクトル描画・AI）を「別軸」の節に置いて段階やウェーブと並べず、順位を付けない。
7. The 統合調査 shall 既存 27 brief のうち、台帳 id と brief の所有宣言が食い違うもの（調査 4 本のブリーフィングの是正候補を統合し、本 spec で見つけたものを加える）を、spec 名・食い違う id・直し方の案の 3 列で列挙し、brief 本体は書き換えない。
8. The 統合調査 shall M3「伺かの冠」の受入基準の候補（要件 5.7）と、M2 予約群（roadmap.md「M2 以降」節の列挙）の各項目がどの束に写ったかの対応表を書き、写らなかった予約項目があればその名前と理由を書く（0 件なら 0 と書く）。
9. The 統合調査 shall `roadmap-draft.md` に「本文書は草案であり、roadmap.md への反映は棚卸セッションで一括裁定する」旨を冒頭に書く。

### Requirement 11: 文書の主張を機械が判定する

**Objective:** 読み手として、新規 3 文書が述べる id・束・件数が台帳と食い違ったときに標準のテスト実行が赤になってほしい。印字するだけの数が古びるのを防ぐためである。

#### Acceptance Criteria

1. The 統合調査 shall 次の判定を標準のテスト実行（`cargo test -p ukadoc-survey`）に加え、いずれも失敗時にファイル名と id（または束名）を名指しする: ⑴ `linkage.md`・`briefing.md`・`roadmap-draft.md` に引用された項目 id の全数がカタログに実在する、⑵ 引用された機械の束 id の全数が報告 5 本の束の一覧に実在する、⑶ `linkage.md` の名前付き束の構成 id が互いに重ならず、状態が `implemented`・`vocabulary-only`・`degraded`・`absent` の全項目が名前付き束か単独項目のちょうど一方に属する、⑷ `briefing.md` が述べる段階ごとの束数・項目数が台帳の `priority` の頭文字から数えた数と一致する、⑸ `roadmap-draft.md` の spec 表について、表の各 spec 名が `.kiro/specs/` 直下または `completed/` に実在し、述べた数が表の行数と一致し、台帳の非空 `owner` がすべて表の名前か「完了済み spec を `owner` に残した宛先の列挙」（`briefing.md`）の名前であり、その列挙の spec が `completed/` に実在してその名前を `owner` に持つ項目の状態がすべて `implemented` か `degraded` である（他 spec の起票・完了で赤にならず、表の spec の改名・削除で赤になる形にする。本 spec が `completed/` へ移った後も同じ結果になる数え方にする）、⑹ `report/summary.md` のうちカタログと台帳 4 本から決まる本文が、作り直した本文と一致する。
2. The 統合調査 shall ⑹ について、完了 spec toolkit 要件 7.6 が全体報告を常時検査から外した理由（並走 4 本が同じファイルを取り合う）が調査 4 本の完了で消えたことを根拠に、除外を本 spec で覆す旨を `README.md`「誰が何を作り直すか」の表と本 spec の文書に書く。ただし `summary.md` 末尾の「証拠あり件数」はソース木を歩いて数える値であり、判定に入れると他の spec が正典 URL のコメントを 1 行足すだけで赤になるため、⑹ の判定対象から外す（証拠の表を `summary.md` に残すか `evidence` 副手続きの出力へ移すかは設計で決め、残す場合は「判定の対象外」と表の直上に書く）。
3. The 統合調査 shall 判定の各種別について、実データの写しを 1 か所だけ壊すと赤になることを示すテストを併せて置く（既知の欠陥を再現して赤にできない判定は判定として数えない）。
4. The 統合調査 shall 判定の各種別について、対象が 0 件でないこと（母数 0 の緑を恒真にしない）を確かめるテストを置く。
5. The 統合調査 shall 新規 3 文書に書く件数のうち 0 になるもの（単独項目 0 のドメイン・束に写らなかった予約項目 0 件など）を空欄や省略で表さず「0」と書き、数え方を添える。
6. The 統合調査 shall 判定を `doc/ukadoc-coverage/` の外の使い捨ての場所（作業用スクリプトのみ）に置かず、標準のテスト実行から外れた検査を「判定」と呼ばない。
7. When 判定が赤になった, the 統合調査 shall 文書の側を台帳に合わせて直すか、台帳を直して報告を作り直すかのどちらかで解消し、判定を緩めて緑にしない。

### Requirement 12: 非接触と境界

**Objective:** 併走する W13 の spec と将来の読み手として、本 spec が触る場所が限定されていて、実行時の挙動と他 spec の所有物に影響しないことを確かめたい。

#### Acceptance Criteria

1. The 統合調査 shall areka の実行時コード（`crates/ukadoc-survey` 以外の crate）に 1 行も触れない。
2. The 統合調査 shall `crates/ukadoc-survey` への接触を、要件 11 の判定（テストと、判定に要る読み込み・出力の副手続き）と、要件 7 の書き戻しに使う副手続き（既存の項目の塊のバイト列を保ったまま `priority`・`owner` の欄 1 行だけを置き換えるもの。足すか手編集で済ませるかは設計で決める）に限り、台帳の項目形式・状態の語彙・関連の種別・テーマ 8 つ・ドメインの分割を変えない。
3. The 統合調査 shall `.kiro/steering/roadmap.md`・既存 27 brief・調査 4 本のブリーフィング・`values.md`・`catalog.toml` を編集しない（是正はすべて本 spec の文書の側に候補として書く）。
4. The 統合調査 shall `README.md` の編集を、要件 11.2 の表の更新とその直下の説明の段落、11.2 に伴い事実でなくなる記述（「報告の扱い」節と「全体報告は黙って古くなる」節の「常時の検査に入っていない」の文）の是正、および「この一式に入っているもの」への新規 3 文書の追記に限る。
5. The 統合調査 shall SSP との実機比較・実在ゴーストの走行を行わず、意味論の根拠は ukadoc の URL と逐語引用で示す。
6. The 統合調査 shall 新規 3 文書と台帳の `note` に行番号を書かず、引用は「何の定義行か」または節名で指す。
7. The 統合調査 shall 完了時の報告に、⑴ 作った文書 3 本の絶対パスと行数、⑵ 台帳の編集件数（`links`・`values`・`priority`・`owner`・`note` ごと。0 も書く）、⑶ 判定の種別数と摂動テストの本数、⑷ 開発者の裁定候補の一覧、⑸ `/kiro-discovery` 再入へ渡す先頭ウェーブの束名と候補 spec 名、⑹ 棚卸セッションへ渡す roadmap.md の改訂候補、の 6 つを書く。
8. The 統合調査 shall 新規 3 文書と `crates/ukadoc-survey` のテストに本 spec 自身の spec ディレクトリのパス（`.kiro/specs/areka-P0-ukadoc-coverage-roadmap/…`）を書かない（完了手続きが `completed/` へ移す際にパスの綴りを書き換える対象になり、判定が移動の前後で違う結果を返すため）。
