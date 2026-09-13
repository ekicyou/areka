# Brief: areka-P0-ukadoc-coverage-roadmap

> 起票: 2026-09-02（`/kiro-discovery` Path D・ukadoc 網羅調査 5 本の 5 本目＝統合・最終）。
> **種別**: 調査 spec（統合台帳＋ブリーフィング＋M2+ ロードマップ草案・実行時コード非接触）。
> **開発者要望の最終成果物＝「各項目間の繋がりを評価して分類し、areka を製品品質にするために必要だと思われる順に実装項目を洗い出したブリーフィング」と「網羅ロードマップ」。**

## Problem

M1 ロードマップは「M2 以降は M1 完成後に実物を見て組み直す（憶測で先に書かない）」と定めている。組み直しの材料は ⑴ M1 完走で見えた実物（e2e 適合 14 項目）と ⑵ 正典全体に対する網羅台帳の 2 つだが、⑵ が無い。3 本の調査 spec が持ち寄る台帳（shiori 677・assets 542・script-property 530＝1,749）はドメイン別であり、**ドメインを跨ぐ繋がり**（例: descript `secondchangeinterval` ↔ event OnSecondChange ↔ plugin event OnSecondChange／descript `seriko.zorder` ↔ `\![set,zorder]` ↔ `currentghost.seriko.zorder`／install.txt ↔ nar ↔ OnInstallComplete ↔ `\![execute,install,...]`）は誰の台帳にも収まらない。「製品品質に必要な順」は繋がりの評価なしには決められない。

## Current State

- ukadoc 側の粗い繋がり実測（2026-09-02）: 他項目の title を本文に含む entry は 194/1,749・277 辺。機械抽出は繋がりの一部しか拾えない＝3 台帳の `links` 登記（人手）が主で、機械抽出は補助。
- roadmap M2 予約（既記載）: pasta native x64・`IShiori` in-proc・ベクトル描画・AI・**オーナードロー右クリックメニュー**・Shift_JIS/SAORI/里々・YAYA 網羅/**NAR**・SSTP 9801・FMO・DirectSSTP・Plugin/HEADLINE/SAORI ホスティング・ネットワーク更新・ゴースト/バルーン選択 UI・多重ゴースト・emo テキスト進化（回転・装飾）・バルーン美観配置。**これらは列挙であって順序も根拠も無い**＝本 spec が台帳の裏付けで順序を与える。
- M2 ゲート brief 13 本（既存）: 順序未決・相互依存は各 brief の Upstream/Downstream に散在。
- areka 側の粗い網羅率（2026-09-02 サブエージェント実測・各 survey brief に file:line）: 送出イベント 11/290・照会リソース 1/159（語彙は 159 登記済み）・`\!` 消費者 4 登録・ghost descript 7 系統/74・balloon descript 29/162・surfaces interval 3 種・collision 矩形のみ・install/update/nar/SSTP/FMO/SAORI/HEADLINE/PLUGIN は 0。**未知 descript キーは無言で捨てる**のが既定挙動。段階 A（既存資産の一般化）の主障壁は「イベント 4%」と「未知キー無言」の 2 つになる見込み（要件フェーズで台帳から確定）。

## Desired Outcome

1. **統合台帳** `doc/ukadoc-coverage/report.md`（toolkit が再生成）に 1,749 項目の status 分布（実装済み／語彙のみ／縮退／未対応／対象外）がドメイン別・SSP 世代別に出る。
2. **繋がりの評価** `doc/ukadoc-coverage/linkage.md`: ドメイン跨ぎの連鎖（descript↔event↔property↔tag↔resource）を「機能の束」として名付け、束ごとに「成立に要る最小の基盤」と「束が欠けると壊れる既存ゴーストの振る舞い」を書く。
3. **ブリーフィング** `doc/ukadoc-coverage/briefing.md`: 製品品質の段階定義と、段階ごとの実装項目の順序付き一覧。**段階は「利用者が体験できる節目」で名付け、束は機構で切り、テーマ（toolkit 規則 9）が束→段階の写像を決める**（開発者裁定 2026-09-02・議題 7・要件フェーズで確定）:

   | 段階 | 節目（利用者の体験） | 主な束 | 旧定義からの移動 |
   |---|---|---|---|
   | A | そこにいて、触れて、話す | 起動と挨拶・会話・撫で・メニュー・終了・**自分から喋る（ランダムトーク・時報・分）**・**名前を尋ねて覚える** | 時報／OnMinuteChange を旧 C→A（気配）・OnUserInput／`\![open,inputbox]` を旧 C→A（記憶・初回起動の定番演目）。M1 の emo2 を里々／YAYA 製の代表 2〜3 体へ一般化 |
   | B | 迎えて、育てて、見送る | nar インストール（D&D 含む）・ネットワーク更新・シェル／バルーン切替・オーナードローメニュー・**消滅** | OnVanish 系を旧 D→B（記憶＝所有の一生）・ファイル D&D（OnFileDrop2）を旧 C→B（触れ合い＋更新の 2 テーマ） |
   | C | 察してくれる | スリープ復帰・バッテリー・スクリーンセーバー・フルスクリーン退避・最小化・ディスプレイ変化・サウンド | `system.*` 照会は C 末尾（テーマ 0・基盤共有度だけで順位） |
   | D | 仲間がいる | 多重ゴースト・コミュニケート・呼び出し・SSTP・FMO・PLUGIN・`x-ukagaka-link` | 変更なし |
   | E | 周辺 | 外部アプリ Ex・開発者機能・トランスレータ・ヘッドライン | ヘッドラインを旧 D→E（テーマ 0） |

   **SAORI の扱い**（開発者確認 2026-09-02）: SAORI は SHIORI が `LoadLibrary` して直接呼ぶ（里々 `satori_conf.txt` の `@saori`・YAYA の自前ローダー・蒼空 `Saori.Load()`）ものであり、**areka はプロトコルを実装しない**（`doc/COMPAT_ARCHITECTURE.md:87`「同 32bit プロセスに同居」）。台帳では `not-applicable`（実装主体は SHIORI）・note に成立条件（host-32 の同居環境＝32bit 同一プロセス・作業ディレクトリ `ghost/master`・DLL 検索パス）を書き、**段階 A の検証項目**（里々製ゴーストで `saori/*.dll` が読めるか）として links で繋ぐ。旧定義で D に置いたのは機構の都合であり、実装項目ではない。
4. **網羅ロードマップ草案** `doc/ukadoc-coverage/roadmap-draft.md`: 段階 A〜E を M2 以降のマイルストーン候補として、束→候補 spec（既存 13 brief の位置づけ込み）→依存順→ウェーブ案を書く。
5. **開発仕様の立ち上げとロードマップ調整**（開発者追記 2026-09-02＝段③④）: 優先順の**先頭ウェーブ分**について開発 spec の brief を `/kiro-discovery` 再入で起票し（台帳 id を brief に列挙＝根拠が引ける）、steering の roadmap.md への反映は棚卸セッションで一括裁定する（記憶 portfolio-convergence-decided-in-separate-session）。**先頭ウェーブより先の束は brief を作らず roadmap-draft の名前付き束のまま置く**（spec 工場化しない・着手時に just-in-time）。既存 13 brief は台帳 id で所有範囲を書き換える是正候補を添える。

## Approach

**二段で進める**（開発者裁定 2026-09-02・議題 3）:

| 段 | 着手条件 | 作るもの |
|---|---|---|
| 第一段 | survey 4 本の台帳が `unclassified` 0 で揃い次第（**e2e を待たない**） | `report.md`（状態分布）・`linkage.md`（束の一覧＝例「時刻イベント束」＝descript `secondchangeinterval`＋OnSecondChange／OnMinuteChange／OnHourTimeSignal＋plugin OnSecondChange＋`system.clock.*`・束が無いと壊れる振る舞い・成立に要る最小基盤）・`briefing.md` **草案**（段階 A〜E への仮置きと仮順位） |
| 第二段 | **e2e（M1 完成）後** | `briefing.md` 確定版（実物＝適合 14 項目の結果で仮順位を直し、理由を e2e の項目番号で書く）・先頭ウェーブ分の開発 spec brief（`/kiro-discovery` 再入・台帳 id を列挙）・ロードマップ反映（棚卸セッションで一括裁定） |

方針「M2 は M1 完成後に実物を見て組み直す」は、**決定にあたる第二段を実物の後ろに置く**ことで守る。

- 4 台帳の `links` を toolkit で結合し、連結成分（束）を機械で出す→人手で名付け・分割・優先度付け。
- **優先度の根拠は 4 つ・序列固定**（toolkit 規則 6・開発者裁定 2026-09-02 議題 6）: ⑴ 壊れ方（黙って壊れる＞明示エラー＞見た目差）＞ ⑵ **伺からしさ**（toolkit 規則 9 のテーマ 8 つ＝気配／触れ合い／掛け合い／装い／記憶／交わり／気配り／更新・4 台帳の `values[]` を束ごとに和集合で集約）＞ ⑶ 影響する既存資産の広さ（里々/YAYA の標準辞書が使う項目か・版番号の古さ）＞ ⑷ 依存基盤の共有度（1 基盤で何束が成立するか）。⑵ は「よく使う」ではなく「無いと伺かでなくなる」を測る＝⑶ の補集合（象徴的だが稀な群を拾う）。**上げ方**: テーマ 1 つ＝同段階内で先頭群へ／2 つ以上＝段階を 1 つ繰り上げ可／テーマ 0 かつ壊れ方が見た目差以下＝段階 E 候補。**新旧両軸で高い唯一の群＝「更新」**（開発者 2026-09-02「ネットワーク更新に関する軸は優先すべきかも」）を段階 B の先頭に置く。`report.md` の「テーマ別の状態分布」節が briefing の根拠表になる。
- **M3「伺かの冠」との接続**（マイルストーン階梯＝棚卸セッションの裁定待ち）: 受入基準の候補として「**テーマ 8 つ全てで代表束が実装済み**」を roadmap-draft に登記する（決めるのは階梯の議論のとき）。
- **「使用頻度」の参照元と段階 A の検証対象**（開発者裁定 2026-09-02・議題 4）: 当面は**里々／YAYA の標準テンプレート辞書**（新規ゴースト作成の雛形・ukadoc MCP の satori／yaya wiki で裏取り可）が使う語彙を参照値にする。実在ゴーストの辞書走査と実走検証は行わない。開発者の本音は「実在ゴーストを指定したい」だが良い候補が無く、候補があるとすれば自作の「どっとさくら」。**外部から「このゴーストを動かして」という要望が来た時点で、そのゴーストを参照元・検証対象に切り替える**。段階 A の温度感は「当面 emo2 が動けばよい」。

## Scope

- **In**: 統合・繋がり評価・段階定義・優先順一覧・ロードマップ草案・既存 13 brief の位置づけ。
- **Out**: 実装・steering roadmap.md の書き換え・候補 spec の brief 生成・SSP との実機比較。

## Boundary Candidates

- 「繋がり評価（linkage）」と「優先順（briefing）」と「編成（roadmap-draft）」は別文書・別タスク。

## Out of Boundary

- M2 の技術選定（pasta native x64・`IShiori` in-proc・ベクトル描画・AI）＝正典網羅とは別軸。roadmap-draft では「別軸」と明記して並べない。

## Upstream / Downstream

- **Upstream**: `ukadoc-survey-shiori`・`ukadoc-survey-assets`・`ukadoc-survey-sakura-script`・`ukadoc-survey-property`（4 台帳が `unclassified` 0・alias 仕訳済みで揃うこと）・`ukadoc-survey-toolkit`（report/linkage の再生成）・`emo2-conformance-e2e`（M1 完成の実物・段階 A の起点。**第一段は e2e 前に進め、第二段＝順位確定・開発 spec 起票・ロードマップ反映は e2e 完了後**）。
- **Downstream**: `/kiro-discovery` 再入（M2 ロードマップ本文化）・既存 M2 ゲート brief 13 本の順序確定・新規 spec の just-in-time 起票。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: M2 ゲート brief 13 本（位置づけの対象・書き換えない）／`.kiro/steering/roadmap.md`「M2 以降」節（取り込み先・本 spec では非接触）。

## Constraints

- 成果物は `doc/ukadoc-coverage/` 配下のみ。
- 順序の根拠は台帳の行（id）で引けること（「たぶん重要」を書かない）。
- 文書は平易な語で（利用者から見える結果の差で語る・記憶 explain-simply-before-asking-developer）。
