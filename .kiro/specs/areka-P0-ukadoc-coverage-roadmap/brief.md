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
3. **ブリーフィング** `doc/ukadoc-coverage/briefing.md`: 製品品質の段階定義と、段階ごとの実装項目の順序付き一覧。段階の定義案（要件フェーズで確定）:
   - 段階 A: 既存ゴースト資産（里々／YAYA 製の代表 2〜3 体）が起動・会話・撫で・メニュー・終了できる（M1 の emo2 を他ゴーストへ一般化）。
   - 段階 B: 入手して入れて更新できる（nar インストール・ネットワーク更新・ゴースト/バルーン切替 UI・オーナードローメニュー）。
   - 段階 C: 環境と対話する（時刻・窓状態・OS 事象イベント群・`system.*` プロパティ・サウンド）。
   - 段階 D: 外部と繋がる（SSTP・FMO・PLUGIN・HEADLINE・SAORI・`x-ukagaka-link`）。
   - 段階 E: 周辺（外部アプリ連携 Ex イベント・開発者機能・トランスレータ）。
4. **網羅ロードマップ草案** `doc/ukadoc-coverage/roadmap-draft.md`: 段階 A〜E を M2 以降のマイルストーン候補として、束→候補 spec（既存 13 brief の位置づけ込み）→依存順→ウェーブ案を書く。
5. **開発仕様の立ち上げとロードマップ調整**（開発者追記 2026-09-02＝段③④）: 優先順の**先頭ウェーブ分**について開発 spec の brief を `/kiro-discovery` 再入で起票し（台帳 id を brief に列挙＝根拠が引ける）、steering の roadmap.md への反映は棚卸セッションで一括裁定する（記憶 portfolio-convergence-decided-in-separate-session）。**先頭ウェーブより先の束は brief を作らず roadmap-draft の名前付き束のまま置く**（spec 工場化しない・着手時に just-in-time）。既存 13 brief は台帳 id で所有範囲を書き換える是正候補を添える。

## Approach

- 3 台帳の `links` を toolkit で結合し、連結成分（束）を機械で出す→人手で名付け・分割・優先度付け。
- 優先度の根拠は 3 つに限定: ⑴ 壊れ方（黙って壊れる＞明示エラー＞見た目差） ⑵ 影響する既存資産の広さ（里々/YAYA の標準辞書が使う項目か・版番号の古さ） ⑶ 依存基盤の共有度（1 基盤で何束が成立するか）。
- 段階 A の検証対象ゴーストを要件フェーズで開発者と確定する（emo2 以外の代表資産＝ライセンス上テストに使えるもの）。

## Scope

- **In**: 統合・繋がり評価・段階定義・優先順一覧・ロードマップ草案・既存 13 brief の位置づけ。
- **Out**: 実装・steering roadmap.md の書き換え・候補 spec の brief 生成・SSP との実機比較。

## Boundary Candidates

- 「繋がり評価（linkage）」と「優先順（briefing）」と「編成（roadmap-draft）」は別文書・別タスク。

## Out of Boundary

- M2 の技術選定（pasta native x64・`IShiori` in-proc・ベクトル描画・AI）＝正典網羅とは別軸。roadmap-draft では「別軸」と明記して並べない。

## Upstream / Downstream

- **Upstream**: `ukadoc-survey-shiori`・`ukadoc-survey-assets`・`ukadoc-survey-sakura-script`・`ukadoc-survey-property`（4 台帳が `unclassified` 0・alias 仕訳済みで揃うこと）・`ukadoc-survey-toolkit`（report/linkage の再生成）・`emo2-conformance-e2e`（M1 完成の実物・段階 A の起点。**e2e 完了前に本 spec を完了させない**）。
- **Downstream**: `/kiro-discovery` 再入（M2 ロードマップ本文化）・既存 M2 ゲート brief 13 本の順序確定・新規 spec の just-in-time 起票。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: M2 ゲート brief 13 本（位置づけの対象・書き換えない）／`.kiro/steering/roadmap.md`「M2 以降」節（取り込み先・本 spec では非接触）。

## Constraints

- 成果物は `doc/ukadoc-coverage/` 配下のみ。
- 順序の根拠は台帳の行（id）で引けること（「たぶん重要」を書かない）。
- 文書は平易な語で（利用者から見える結果の差で語る・記憶 explain-simply-before-asking-developer）。
