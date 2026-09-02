# Requirements Document

## Project Description (Input)
ukadoc（SSP 公式仕様書）1,749 項目を網羅的に分類するための道具を用意する。現状 areka には「正典のどの項目が実装済み／語彙だけ登記済み／縮退／未対応／対象外なのか」を一望できる台帳が無く、spec ごとに ukadoc を手作業で数え直している。本 spec は新規の調査用クレートと `doc/ukadoc-coverage/` 配下の台帳一式を建て、台帳の形式・仕訳の規則・整合検査・報告の再生成を凍結する。後続の調査 spec 4 本（shiori／assets／sakura-script／property）が同じ道具・同じ形式で並走して台帳を書けるようにし、`ukadoc-coverage-roadmap` がそれを統合して優先度を決められるようにする。areka の実行時コードは 1 行も変更しない。

## Introduction

本 spec は「ukadoc 網羅調査」6 本のうち唯一コードを書く**道具 spec** である。作るものは 2 つある。

1. **正典の写し（カタログ）と areka の判定（台帳）を分けて置く場所** — `doc/ukadoc-coverage/` 配下。カタログは機械生成のみ、台帳は調査 spec の担当者が人手で書き、機械が検査する。
2. **その一式を扱う調査用クレート** — カタログ生成・証拠収集・整合検査・報告生成の 4 つの働きを持つ。

利用者は 3 種類いる。⑴ 台帳を書く調査 spec の担当者（shiori／assets／sakura-script／property の 4 本）、⑵ 台帳を統合して優先度を決める `ukadoc-coverage-roadmap` の担当者、⑶ 将来の機能 spec の実装者（実装した項目の定義箇所に正典 URL を 1 行置き、台帳の該当行を更新する）。

本 spec の中心的な価値は「**形式の凍結**」にある。要件が承認された時点で台帳の行形式・状態語彙・担当ドメインの分割・仕訳の規則が確定し、調査 spec 4 本は本 spec の実装完了を待たずに並走を始められる（機械検査は後から追いつく）。したがって本要件は、後続 5 本が依存する契約そのものを含む。

**現状の実測（2026-09-02）**

- ukadoc スナップショットは npm グローバルの `ukagaka-doc-mcp`（パッケージ版 0.2.7）が持つ単一 JSON（`%APPDATA%\npm\node_modules\ukagaka-doc-mcp\data\index.json`・2,716,948 バイト・`version` = 1・`generatedAt` = `2026-08-24T04:08:57.881Z`）。全 2,983 entry のうち `source` が `ukadoc` のものが 1,749 件、38 ページに分かれる。残る 1,234 件（satori_wiki 745・yaya_wiki 448・aosora_wiki 41）は対象外。
- ukadoc の entry id はコロン区切りで 2 形ある。`ukadoc:<ページ>:<アンカー>:<連番>`（1,730 件）と、アンカーを持たないページ全体の `ukadoc:<ページ>`（19 件）。第 2 セグメントがページ名で、ページ名自身が下線を含む。
- 全 1,749 件が `https://` の URL を持ち、URL は entry ごとに相異なる。フラグメントを外すと 38 種（ページ数と一致）に縮む。
- カテゴリは 6 種（shiori_event 637・descript 518・sakurascript 342・protocol 237・file_structure 8・dev_guide 7）。カテゴリとページは 1 対 1 ではなく、`protocol` は `list_propertysystem` 188 と `spec_*` 49 に分かれる。
- `doc/ukadoc-coverage/` も調査用クレートも現時点で存在しない。ソース中に「ukadoc」と URL を併記した doc コメントは 0 件（「ukadoc」の語だけを含む散文コメントは 4 件あり、例: `crates/areka-parsers/src/sakura/lexer.rs:54`）。
- 既に機械可読な正典資産が 2 系統ある。`doc/shiori/fragments/`（フラグメント 38 本＋`_manifest.toml`・イベント entry 287／リソース entry 159／field 802／沈黙裁定 9・entry は `[entry."名前"]` 形式のキー付きテーブル＝`doc/shiori/fragments/events/01.lifecycle.toml:5`・件数は `doc/shiori/README.md:37`・生成器は未実装と宣言済み `doc/shiori/README.md:41`）と、`crates/areka-sylphya/src/vocab/`（`flat.rs:32` の 26 件・`dotted.rs:17` の 10 件・`dotted.rs:37` の 17 件・`dotted.rs:72` の 21 件・`shiori_resource.rs:45` の 159 件、件数固定テスト `crates/areka-sylphya/src/ledger_key_determinism_tests.rs:201-204`）。本 spec はこれらを置き換えない。
- 実装側の許可表は小さい。送出イベント 11 件（`crates/areka-kanade/src/schedule/events.rs:70-82`・件数を固定するテストは無い）、照会リソース 1 件（`crates/areka-kanade/src/schedule/resources.rs:32`・固定テストは `resources.rs:114-118`）、`\![...]` の消費側登録 4 件（`crates/areka/src/emo2_boot/consumer_ledger.rs:221`）。
- 既存の対応表は `doc/COMPAT_ARCHITECTURE.md:122` の沈黙ルール対応表（データ行 80・`:128-207`）と `doc/emo2-conformance-scope.md:78` の見直し表（データ行 7）。台帳の「縮退」と備考の転記元になる。

## Boundary Context

- **In scope**:
  - `doc/ukadoc-coverage/` 一式（カタログ・台帳 4 本・テーマ定義・報告・README）の新設と、その形式・語彙・分割の凍結。
  - 調査用クレート 1 本（カタログ生成・証拠収集・整合検査・報告生成）。
  - 全 1,749 項目の初期台帳（全行「未分類」）。
  - ソース側に置く正典 URL の書き方の規約（どこに何を何行書くか）。
- **Out of scope**:
  - 個々の項目の分類・優先度付け・繋がりの登記そのもの（調査 spec 4 本が行う）。段階 A〜E の最終順序（`ukadoc-coverage-roadmap` が決める）。
  - ukadoc 本文の repo 同梱。ukadoc 以外の 3 ソース（yaya／里々／蒼空 wiki）。
  - areka 実行時コードの変更。既存の `doc/shiori/fragments/` 生成器の実装。
  - ukadoc スナップショットを作る外部パッケージ自体の改修、および SSP 実機との挙動比較。
  - 「正典 URL の置き忘れ」（実装済みなのに一度も URL を置かなかった項目）を検出する運用。完了処理側の DoD で扱い、本 spec では決めない。
- **Adjacent expectations**:
  - 調査 spec 4 本は「台帳ファイル 1 本＝spec 1 本」で共有ファイルを持たず並走する。本 spec は 4 本が同時に書いても衝突しない分割を与える。
  - `ukadoc-coverage-roadmap` は台帳から統合報告を再生成できることを前提にする。
  - 将来の機能 spec は「実装した項目の定義箇所に正典 URL を 1 行置く」規約に従う。本 spec はその規約と機械検査を用意するが、既存コードへの URL 付与作業そのものは調査 spec と各機能 spec の仕事とする。
  - 新設クレートはワークスペースの標準テスト実行（`cargo test --workspace`）と、ファイル行数の上限テスト（1,000 行・`crates/log-capture-kit/tests/workspace_scan/mod.rs:38`・`crates/` 配下を自動走査するため新クレートも即座に対象になる＝`mod.rs:79-83`）の対象に自動的に入る。

## Requirements

### Requirement 1: 正典カタログの生成
**Objective:** 調査 spec の担当者として、ukadoc 1,749 項目の一覧を機械可読な 1 ファイルで手に入れたい。それにより spec ごとに正典を数え直さずに済む。

#### Acceptance Criteria
1. When カタログ再生成が実行される, the ukadoc 調査ツールキット shall スナップショット中の `source` が `ukadoc` である全 entry（実測 1,749 件）について 1 項目 1 行の `doc/ukadoc-coverage/catalog.toml` を出力する。
2. When カタログ再生成が実行される, the ukadoc 調査ツールキット shall 各行に「項目 id・ページ名・見出し・カテゴリ・本文から抽出した SSP 版番号（抽出できなければ空）・本文のハッシュ・正典 URL」を記録する。
3. The ukadoc 調査ツールキット shall 本文そのものをカタログに記録せず、本文の変更検出はハッシュだけで行う。
4. When スナップショット中の entry の `source` が `ukadoc` 以外である, the ukadoc 調査ツールキット shall その entry をカタログに含めない。
5. When 同一のスナップショットに対してカタログ再生成を 2 回続けて実行する, the ukadoc 調査ツールキット shall 行の順序を含めて 1 バイトも違わない出力を生成する。
6. When カタログ再生成が実行される, the ukadoc 調査ツールキット shall スナップショットの版・生成日時・提供パッケージの版・全 entry 件数・うち ukadoc の件数をカタログの冒頭に記録する。
7. Where 環境変数 `AREKA_UKADOC_SNAPSHOT` が設定されている, the ukadoc 調査ツールキット shall 既定の場所より優先してその場所のスナップショットを読む。
8. If スナップショットが読めない（存在しない・壊れている）, then the ukadoc 調査ツールキット shall 探した絶対パスと理由を示すエラーを出して失敗し、既存のカタログを書き換えない。
9. The ukadoc 調査ツールキット shall アンカーを持たないページ全体の id（19 件）とアンカー付きの id（1,730 件）の双方を、同じ 1 行形式で区別なく収容する。

### Requirement 2: 台帳の行形式と状態語彙の凍結
**Objective:** 調査 spec 4 本の担当者として、書くべき 1 行の形が spec 横断で同一であってほしい。それにより台帳の形式が spec ごとに割れず、統合報告が機械で作れる。

#### Acceptance Criteria
1. The ukadoc 調査ツールキット shall 台帳の 1 行に次の欄を持たせる: 項目 id・状態・登場した版・別名の参照先（任意）・後継の参照先（任意）・担当 spec・優先度（段階 1 文字＋数値）・伺からしさのテーマ（0 個以上）・関連（種別と相手 id の対）・備考。
2. The ukadoc 調査ツールキット shall 状態の語彙を次の 7 つだけに限る: `implemented`（実装済み）・`vocabulary-only`（語彙のみ登記）・`degraded`（縮退）・`absent`（未対応）・`alias`（別名）・`not-applicable`（対象外）・`unclassified`（未分類）。
3. The ukadoc 調査ツールキット shall 証拠の欄（正典 URL を見つけたソースの場所）を人手で書かせず、証拠収集の結果として機械が埋める欄として定義する。
4. When 状態が `alias` である, the ukadoc 調査ツールキット shall その行に「正典側の id への写像があるか否か」だけを持たせ、実装状態の判定は写像先の正典行に委ねる。
5. The ukadoc 調査ツールキット shall 台帳の行形式・状態語彙・欄の意味を `doc/ukadoc-coverage/README.md` に記載する。
6. While 本要件が承認済みである, the ukadoc 調査ツールキット shall 台帳の行形式・状態語彙・ドメイン分割を変更しない（変更には本要件の改訂を要する）。
7. The ukadoc 調査ツールキット shall カタログ（機械生成のみ・正典の写し）と台帳（人手で記入・機械で検査・areka の判定）を別ファイル・別責務として保つ。

### Requirement 3: 担当ドメインの分割と初期台帳
**Objective:** 調査 spec 4 本の担当者として、自分のファイルだけを編集すれば済む分割がほしい。それにより 4 本が同時に走っても互いの作業が衝突しない。

#### Acceptance Criteria
1. The ukadoc 調査ツールキット shall 台帳を 4 ファイルに分け、ページ単位で次のとおり割り当てる。

   | 台帳ファイル | 担当ページ | 件数 |
   |---|---|---|
   | `doc/ukadoc-coverage/ledger/shiori.toml` | list_shiori_event 290・list_shiori_event_ex 168・list_shiori_resource 159・list_plugin_event 19・memo_shiorievent 1・spec_shiori3 26・spec_fmo_mutex 6・spec_web 3・spec_sstp 2・spec_dll 1・spec_plugin 1・spec_headline 1 | 677 |
   | `doc/ukadoc-coverage/ledger/assets.toml` | descript_balloon 162・descript_shell_surfaces 137・descript_shell 102・descript_ghost 74・descript_install 15・descript_plugin 13・descript_headline 9・descript_shell_surfacetable 6・spec_update_file 9・manual_balloon／manual_directory／manual_ghost／manual_install／manual_owner_draw_menu／manual_shell／manual_translator／manual_update 各 1・dev_bind／dev_nar／dev_ownerdraw／dev_shell／dev_shell_error／dev_update／memo 各 1 | 542 |
   | `doc/ukadoc-coverage/ledger/sakura-script.toml` | list_sakura_script | 342 |
   | `doc/ukadoc-coverage/ledger/property.toml` | list_propertysystem | 188 |

2. The ukadoc 調査ツールキット shall 1 つのページに属する項目を 1 つの台帳ファイルだけに置き、同じ id を 2 つ以上の台帳に置かない。
3. When 初期台帳が生成される, the ukadoc 調査ツールキット shall カタログの全 1,749 id について 1 行ずつを、状態 `unclassified`・担当 spec 未設定の状態で書き出す。
4. The ukadoc 調査ツールキット shall 4 つの台帳ファイルを互いに独立して編集できる状態に保つ（1 つの台帳の編集が他の台帳の内容を要求しない）。
5. If カタログに、どの台帳にも割り当てが無いページが現れる, then the ukadoc 調査ツールキット shall そのページ名を明示して失敗し、割り当ての追加を促す。

### Requirement 4: 仕訳の規則の凍結
**Objective:** 調査 spec 4 本の担当者として、新旧の書式・世代・繋がり・伺からしさの判断規則が 1 か所に書かれていてほしい。それにより 4 本の判断がばらつかない。

#### Acceptance Criteria
1. The ukadoc 調査ツールキット shall 同じ機能に複数の書式があるとき「最も新しい書式を正典、それ以外を別名」とし、向きを ⑴ 正典本文の注記（廃止予定・旧・統合された旨）→ ⑵ SSP 版番号 → ⑶ 人手の判断、の順で決める規則を README に記載する。
2. When 項目に SSP 版番号が無い, the ukadoc 調査ツールキット shall その項目を「世代不明」として扱い、最も古いものとは決めつけない規則を README に記載する。
3. The ukadoc 調査ツールキット shall 関連の種別を次の 6 つに限る: `alias_of`（旧→新）・`supersedes`（新→旧）・`triggers`（操作・タグ→イベント）・`configures`（設定キー→挙動・タグ・イベント）・`queries`（タグ・イベント→プロパティ）・`same-feature`（同じ機能の別の面）。
4. The ukadoc 調査ツールキット shall 伺からしさのテーマを `doc/ukadoc-coverage/values.md` に 8 つだけ凍結する: 気配・触れ合い・掛け合い・装い・記憶・交わり・気配り・更新。
5. When テーマを定義する, the ukadoc 調査ツールキット shall 各テーマについて「1 行の定義」「その項目が無いと利用者がゴーストの何を失うか」「代表となる項目 2〜3 件」を書く。
6. The ukadoc 調査ツールキット shall テーマの付与規則を 1 つだけ定める:「この項目が無いと利用者はゴーストの何を失うか」に答えられるテーマだけを付け、答えられなければ何も付けない。
7. The ukadoc 調査ツールキット shall 優先度の根拠が 4 つあり序列が固定であることを README に記載する: ⑴ 壊れ方（黙って壊れる〔誤った結果を正常な顔で見せる場合を含む〕＞明示的なエラー＞見た目の差）＞ ⑵ 伺からしさのテーマ ＞ ⑶ 影響する既存資産の広さ ＞ ⑷ 依存する基盤の共有度。
8. The ukadoc 調査ツールキット shall 段階（A〜E）の最終決定を本 spec の外（`ukadoc-coverage-roadmap`）に置き、調査 spec は仮の段階を付けるだけであることを README に記載する。
9. The ukadoc 調査ツールキット shall 台帳の各行に「壊れ方の段」の根拠（どのログが出るか・出ないか）を備考へ書ける場所を用意する。

### Requirement 5: 実装済みの証拠をソースから集める
**Objective:** 台帳を読む人として、「実装済み」と書かれた行の根拠がソース側に実在することを確かめたい。それにより根拠が陳腐化したまま気づかない状態を避けられる。

#### Acceptance Criteria
1. The ukadoc 調査ツールキット shall 実装済みの証拠を「ソースの定義箇所に置かれた正典 URL 1 行の doc コメント」と定め、行番号や内部 ID を証拠に使わない。
2. The ukadoc 調査ツールキット shall URL を書く場所を定義箇所だけ（許可表の要素・分岐の腕・語彙表の 1 行）に限り、呼び出し側には書かない規約を README に記載する。
3. The ukadoc 調査ツールキット shall 1 項目につき 1 行・説明文を伴わない書き方を規約とし、実装済み項目の定義行 1 行ずつを超える増量を求めない。
4. Where 正典の名前をそのまま並べた語彙表である, the ukadoc 調査ツールキット shall 表の先頭にページ URL を 1 つ置く書き方を許し、表の要素文字列とカタログの見出しを名前で突き合わせて個々の項目に対応付ける。
5. When 証拠収集が実行される, the ukadoc 調査ツールキット shall ソース全域を走査して正典 URL を集め、カタログの id へ解決し、台帳の証拠欄を機械で埋める。
6. If コメントに「ukadoc」の語はあるが正典 URL を伴わない, then the ukadoc 調査ツールキット shall それを証拠として扱わない（現に 4 件の散文コメントが存在するため）。
7. Where 項目が未実装である, the ukadoc 調査ツールキット shall ソース側に何も書かせない（未対応であることは台帳が持つ）。
8. When 証拠収集が実行される, the ukadoc 調査ツールキット shall 正典 URL がまだ置かれていない既存コードから、URL を置く作業の手掛かりとなる候補（イベント名の文字列・`\![...]` の消費側の名前・設定キーの表・「縮退」「無視」「未知」などを含むログ行）を、証拠とは明確に区別した別の出力として提示する。
9. The ukadoc 調査ツールキット shall 候補の提示を証拠として扱わず、状態の判定は調査 spec の人手に委ねる。

### Requirement 6: 整合検査が標準のテスト実行で常時走る
**Objective:** 開発者として、台帳と正典とソースの食い違いが放置されない仕組みがほしい。それにより整理や作り替えで根拠が消えたことに気づける。

#### Acceptance Criteria
1. The ukadoc 調査ツールキット shall 整合検査をワークスペースの標準テスト実行に常時含め、ネットワークにも実機にも依存しない決定的なテストとして実行する。
2. While ukadoc スナップショットがその環境に存在しない, the 整合検査 shall 赤にならない（repo 内のカタログを正本として検査する）。
3. When 整合検査が実行される, the 整合検査 shall 台帳に現れる全 id がカタログに実在することを確かめる。
4. When 整合検査が実行される, the 整合検査 shall カタログの全 id が 4 つの台帳のいずれか 1 つにちょうど 1 回だけ現れることを確かめる。
5. When 整合検査が実行される, the 整合検査 shall ソース中の正典 URL がすべてカタログに実在することを確かめる。
6. When 整合検査が実行される, the 整合検査 shall 状態が `implemented` の行に少なくとも 1 つの証拠（正典 URL の出現）があることを確かめる。
7. When 整合検査が実行される, the 整合検査 shall 関連の両端の id が実在すること、`alias_of` の指す先が `alias` でないこと（別名の連鎖の禁止）、記録された登場版がカタログの抽出値と矛盾しないことを確かめる。
8. When 整合検査が実行される, the 整合検査 shall 台帳に書かれたテーマ名がテーマ定義に実在することを確かめる。
9. The ukadoc 調査ツールキット shall 未分類の件数を台帳側に固定値として持たせ、実際の未分類件数がその値を上回ったときに検査が赤になるようにする（減少は常に許す）。
10. If 正典 URL の綴りが違う・`implemented` なのに証拠が消えた・テーマ名の綴りが違う・未分類が固定値より増えた, then the 整合検査 shall 赤になり、該当する id と場所を示す。
11. While 実装が入れ替わっても正典 URL が定義箇所に残っている, the 整合検査 shall 赤にならない（行の増減や整理では壊れない）。
12. If 整合検査が失敗する, then the 整合検査 shall 何がどう食い違ったかを示す出力を残す（黙って失敗しない）。

### Requirement 7: 報告の決定論的な再生成
**Objective:** `ukadoc-coverage-roadmap` の担当者として、台帳から網羅状況の一覧を機械で作りたい。それにより優先度の議論を数字の上で行える。

#### Acceptance Criteria
1. When 報告生成が実行される, the ukadoc 調査ツールキット shall `doc/ukadoc-coverage/report.md` に「状態の分布（全体・ドメイン別）」「SSP 世代別の対応表」「別名の一覧」「関連で繋がった束の一覧」「テーマ別の状態分布」を出力する。
2. When 同じカタログと同じ台帳に対して報告生成を 2 回続けて実行する, the ukadoc 調査ツールキット shall 同一の内容を出力する。
3. When 整合検査が実行される, the 整合検査 shall repo 内の報告が現在の台帳から再生成したものと一致することを確かめる。
4. If 報告が台帳と食い違う, then the 整合検査 shall 赤になり、報告の再生成が必要であることを示す。
5. The ukadoc 調査ツールキット shall 報告に現れる状態の呼び名を平易な日本語（実装済み・語彙のみ・縮退・未対応・別名・対象外・未分類）で表示する。

### Requirement 8: スナップショット更新時の差分
**Objective:** 開発者として、正典のスナップショットが新しくなったときに何が変わったかを知りたい。それにより台帳を見直す範囲を絞れる。

#### Acceptance Criteria
1. When 現行のカタログより新しいスナップショットに対して差分が要求される, the ukadoc 調査ツールキット shall 追加された項目・削除された項目・本文が変わった項目をそれぞれ id 付きで列挙する。
2. When 本文の変更を判定する, the ukadoc 調査ツールキット shall カタログに記録された本文ハッシュと新しいスナップショットの本文ハッシュを比べる。
3. If 差分に削除された項目が含まれ、かつその id が台帳に現れる, then the ukadoc 調査ツールキット shall その id を「台帳の見直しが要る項目」として明示する。
4. The ukadoc 調査ツールキット shall 差分の算出をスナップショットが要る作業として扱い、標準のテスト実行の合否に影響させない。

### Requirement 9: 既存資産との非重複と非接触
**Objective:** 開発者として、この道具が既存の実行時コードや既存の正典資産を壊さないと確信したい。それにより並走中の他 spec に影響が及ばない。

#### Acceptance Criteria
1. The ukadoc 調査ツールキット shall areka の実行時コードを変更せず、既存クレートから参照されない。
2. The ukadoc 調査ツールキット shall 既存の 2 系統の機械可読資産（`doc/shiori/fragments/` の契約カタログと `crates/areka-sylphya/src/vocab/` の語彙台帳）を置き換えず、カタログ id からそれらの entry 名へ結ぶ対応だけを持つ。
3. The ukadoc 調査ツールキット shall 同じ項目を 2 か所で数えさせない（対応が付いた項目は既存資産側の名前で辿れる）。
4. The ukadoc 調査ツールキット shall ukadoc の本文を repo に取り込まず、記録するのは URL・見出し・ハッシュ・取得元の版に限る（`.kiro/specs/completed/areka-P0-shiori-protocol/ukadoc/SOURCES.md:9-13` と同じ扱い）。
5. The ukadoc 調査ツールキット shall 台帳・報告・README で使う日本語を平易な語に限り、プロジェクト内でしか通じない言い回しを持ち込まない。
6. The ukadoc 調査ツールキット shall 新設するファイルをいずれも 1,000 行未満に保つ（ワークスペースのファイル行数上限テストの対象に自動的に入るため）。
7. The ukadoc 調査ツールキット shall 環境変数名を `AREKA_` で始める既存の命名規約に従う（例: `crates/areka/src/main.rs:854`・`crates/areka/src/boot_config.rs:95`）。
8. The ukadoc 調査ツールキット shall ロードマップ本文（`.kiro/steering/roadmap.md`）を変更しない。
