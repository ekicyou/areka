# 網羅ロードマップ草案

**この文書は草案である。** 段階の割り当て・束から候補 spec への案・ウェーブの並びは、いずれも
本 spec が材料として並べたものであり、正本の roadmap への反映は棚卸セッションで一括して裁定
する。この文書を読んで直接 roadmap を書き換えない。

束の名前は `linkage.md` のものをそのまま使う。段階と順位は `briefing.md` のものをそのまま使う。
この文書は同じ値を写さず、名前と欄で指す。項目を指すときは必ず引用符か逆引用符で囲む。

まだ 1 行も無い表（spec の表・M2 予約群の対応表）は、囲みに見出しだけを置くことができないので
書いていない。読み手は「その表が無い」を「0 行」として扱う。

## 読み方

**段階 A〜E は M2 以降のマイルストーンの候補である。** 5 つの節は `briefing.md` 3-3 の順位表の
並びのまま、束ごとに 4 つの欄を持つ。M3「伺かの冠」の受入基準の候補は下の別の節に置いた。

- **束**: `linkage.md` が名付けた名前。単独項目は項目 id が名前である。
- **候補 spec 名の案**: この束を起票するとしたら何と呼ぶかの案で、決まった名前ではない。
  付け方は 3 つ——⑴ 構成 id の**全数**を 1 つの既存 spec が `owner` に持つ束は、その spec 名を
  そのまま案にして新しい名前を立てない、⑵ **過半**を 1 つの既存 spec が持つ束は新しい名前に
  「残余」と添え、その spec が着地した後に残る分だけを指す、⑶ それ以外は新しい名前を置く。
  ⑴ は **1 行**・⑵ は **7 行**である。
  案を置かない行が **2 行**あり、その場に理由を書いた。
  案を置いた行は **65 行**で、新しい名前は **63** である
  （⑴ の行が既存の名前を使い、`ukadoc:manual_directory` と `ukadoc:manual_ghost` の 2 行が
  同じ案を共有するため、行数より少ない）。
- **依存する既存 spec**: その束の構成 id を台帳の `owner` に持つ spec と件数である。進行中の
  spec には正本のロードマップのウェーブを、封じた spec には「完了」を添える。1 つも無い束は
  **0 本**と書く（数え方: 構成 id を台帳の `owner` で引き、空でない宛先を数えた）。0 本の束は
  **37** である。
- **波の案**: 下の規則で決めた M2 の波。正本の W13〜W17 とは別の番号で、続き番号を振るのは
  棚卸セッションである。

波の案の付け方の規則。

1. M2 の波は正本のウェーブ編成（W13〜W17）の後に始まる。この文書はその編成を入力として扱い、
   並べ替えは「裁定候補」の節に提案として書くだけで、正本を書き換えない。
2. 波は段階の順に切る——第 3 波が段階 B、第 4 波が C、第 5 波が D、第 6 波が E である。
   段階 A だけは 2 つに割り、順位表で同順位が現れる手前（順位 1〜6）を第 1 波＝先頭ウェーブに、
   残り（順位 7〜18）を第 2 波にする。
3. 束は、その束の構成 id を `owner` に持つ進行中 spec のウェーブより後の波に置く。規則 1 が
   この規則を吸収するので、規則 3 が独立に効く束は **0 束**である（数え方: 67 の束と単独項目
   すべてについて `owner` の進行中 spec のウェーブを引き、W17 より後のものを数えた）。

## 既存 brief の位置づけ

下の囲みは着手時（2026-09-12）に置いた器である。**まだ数えていない仮の 0 であり、段 5
（タスク 6.2）で数え直した値に置き換え、そのときに撮った日付へ `snapshot_on` を打ち直す。**

```toml
[briefs]
count = 0
snapshot_on = "2026-09-12"
```

<!-- 段 5（タスク 6.2）で書く:
     - 数え方（本 spec 自身のディレクトリ名を除いて数えること。パスは書かない）。
     - spec ごとに段階・束・宛先に持つ id の数・ウェーブを 1 表に。どの束にも属さない spec は
       そう書く。
     - この表が着手時の写真であること（撮った日付を添える）と、生きた総数との一致は主張しない
       こと。 -->

## 段階 A

節目は「そこにいて、触れて、話す」である（定義は `briefing.md` 2-1）。並びは `briefing.md` 3-3 の
順位表のままで、**25 行**ある。波は第 1 波（先頭ウェーブ）・第 2 波。

| 順位 | 束 | 候補 spec 名の案 | 依存する既存 spec | 波の案 |
| ---: | --- | --- | --- | --- |
| 1 | 会話 | `areka-P0-talk-script-canon` | `areka-P0-balloon-canon-residue`（W14・5 件）／`areka-P0-sakura-time-directives`（W16・5 件）／`areka-P0-anchor-tag-canon`（W17・1 件）／`areka-P0-status-execution-states`（W15・1 件）／`areka-P0-kero-balloon`（完了・2 件）／`areka-P0-cursor-tag-canon`（完了・1 件） | 第 1 波（先頭ウェーブ） |
| 2 | 窓の配置と重なり | `areka-P0-window-placement-canon` | `areka-P0-currentghost-property-tree`（W15・16 件）／`areka-P0-surfaces-basepos`（W13 任意／W14・2 件）／`areka-P0-sakura-time-directives`（W16・1 件）／`areka-P0-scope-zorder-pinning`（完了・3 件）／`areka-P0-windowposition-limit`（完了・3 件）／`areka-P0-balloon-offset-dpi`（完了・2 件） | 第 1 波（先頭ウェーブ） |
| 3 | 名前の記憶 | `areka-P0-user-name-memory` | `areka-P0-currentghost-property-tree`（W15・1 件）／`areka-P0-package-mount`（完了・2 件）／`areka-P0-sylphya`（完了・2 件）／`areka-P0-sakura-dialogue-tags`（完了・1 件） | 第 1 波（先頭ウェーブ） |
| 4 | 起動と挨拶 | `areka-P0-boot-greeting-canon` | `areka-P0-charset-canon`（W13・2 件）／`areka-P0-package-mount`（完了・1 件） | 第 1 波（先頭ウェーブ） |
| 5 | バルーンの文字 | `areka-P0-balloon-font-canon`（残余） | `areka-P0-text-decoration-canon`（W13・32 件）／`areka-P0-currentghost-property-tree`（W15・13 件）／`areka-P0-balloon-parse`（完了・5 件）／`areka-P0-balloon-vertical-canon`（完了・4 件）／`areka-P0-cursor-tag-canon`（完了・1 件） | 第 1 波（先頭ウェーブ） |
| 6 | サーフェスアニメーション | `areka-P0-seriko-animation-canon` | `areka-P0-currentghost-property-tree`（W15・6 件）／`areka-P0-shell-parse`（完了・2 件） | 第 1 波（先頭ウェーブ） |
| 7 | 入力窓とダイアログ | `areka-P0-inputbox-dialog` | **0 本** | 第 2 波 |
| 7 | 自発発話 | `areka-P0-idle-talk-canon` | **0 本** | 第 2 波 |
| 8 | 終了 | `areka-P0-shutdown-canon` | **0 本** | 第 2 波 |
| 9 | キーとゲームパッド | `areka-P0-key-gamepad-events` | **0 本** | 第 2 波 |
| 10 | descript の転記 | `areka-P0-descript-transcribe` | `areka-P0-balloon-canon-residue`（W14・4 件）／`areka-P0-package-mount`（完了・1 件） | 第 2 波 |
| 10 | バルーンのリンク | `areka-P0-anchor-tag-canon`（既存 spec がそのまま引受先・構成 60 件の全数を `owner` に持つ） | `areka-P0-anchor-tag-canon`（W17・60 件） | 第 2 波 |
| 10 | マウスの矢印 | `areka-P0-mouse-cursor-canon` | `areka-P0-currentghost-property-tree`（W15・15 件） | 第 2 波 |
| 11 | メニュー | `areka-P0-ownerdraw-menu-canon` | `areka-P0-property-catalog-lists`（W16・4 件） | 第 2 波 |
| 12 | 撫で | `areka-P0-touch-events-canon` | `areka-P0-currentghost-property-tree`（W15・5 件）／`areka-P0-shell-parse`（完了・1 件） | 第 2 波 |
| 13 | バルーンの付属画像 | `areka-P0-balloon-inline-image` | `areka-P0-balloon-canon-residue`（W14・9 件） | 第 2 波 |
| 14 | イベントの呼び起こし | `areka-P0-raise-event-tag` | **0 本** | 第 2 波 |
| 14 | 選択肢の目印 | `areka-P0-choice-marker-rest`（残余） | `areka-P0-choice-marker-styling`（W16・39 件） | 第 2 波 |
| 15 | 絵の重ね方 | `areka-P0-surface-composition-canon` | `areka-P0-shell-parse`（完了・1 件） | 第 2 波 |
| 16 | 動作モードの出入り | `areka-P0-passive-mode-states` | `areka-P0-status-execution-states`（W15・2 件） | 第 2 波 |
| 17 | 定義ファイルの文字コード | なし（構成 2 件がどちらも実装済みで、作る仕事が残っていない） | **0 本** | 第 2 波 |
| 17 | 組み込みの置換語 | `areka-P0-builtin-substitution` | **0 本** | 第 2 波 |
| 18 | SHIORI の要求と応答 | `areka-P0-shiori-request-canon` | `areka-P0-charset-canon`（W13・1 件）／`areka-P0-status-execution-states`（W15・1 件） | 第 2 波 |
| 18 | シェル定義の転記 | `areka-P0-shell-definition-transcribe` | `areka-P0-charset-canon`（W13・1 件） | 第 2 波 |
| 18 | 同期オブジェクト | `areka-P0-sync-object-tags` | `areka-P0-sakura-time-directives`（W16・1 件） | 第 2 波 |

## 段階 B

節目は「迎えて、育てて、見送る」である（定義は `briefing.md` 2-1）。並びは `briefing.md` 3-3 の
順位表のままで、**12 行**ある。波は第 3 波。

| 順位 | 束 | 候補 spec 名の案 | 依存する既存 spec | 波の案 |
| ---: | --- | --- | --- | --- |
| 1 | インストール | `areka-P0-nar-install` | **0 本** | 第 3 波 |
| 1 | 更新 | `areka-P0-network-update` | **0 本** | 第 3 波 |
| 2 | 切替 | `areka-P0-shell-balloon-switch` | `areka-P0-currentghost-property-tree`（W15・4 件）／`areka-P0-sakura-time-directives`（W16・2 件）／`areka-P0-balloon-canon-residue`（W14・1 件）／`areka-P0-kero-balloon`（完了・1 件） | 第 3 波 |
| 3 | 消滅 | `areka-P0-vanish-canon` | **0 本** | 第 3 波 |
| 4 | 投げ込み | `areka-P0-file-drop-events` | **0 本** | 第 3 波 |
| 5 | 休止と復帰 | `areka-P0-shiori-cache-suspend` | **0 本** | 第 3 波 |
| 5 | 好感度の絵柄 | `areka-P0-favorite-rate-record`（残余） | `areka-P0-property-catalog-lists`（W16・24 件） | 第 3 波 |
| 5 | 着せ替え | `areka-P0-dressup-bind-canon` | `areka-P0-mayuna-compose`（完了・3 件）／`areka-P0-bindoption-exclusivity`（完了・2 件） | 第 3 波 |
| 6 | `ukadoc:manual_balloon` | `areka-P0-balloon-package-layout` | **0 本** | 第 3 波 |
| 7 | 配布物の素性 | `areka-P0-package-identity` | `areka-P0-ghost-setup`（完了・1 件） | 第 3 波 |
| 8 | `ukadoc:manual_directory` | `areka-P0-package-directory-layout` | **0 本** | 第 3 波 |
| 8 | `ukadoc:manual_ghost` | `areka-P0-package-directory-layout` | **0 本** | 第 3 波 |

## 段階 C

節目は「察してくれる」である（定義は `briefing.md` 2-1）。並びは `briefing.md` 3-3 の
順位表のままで、**14 行**ある。波は第 4 波。

| 順位 | 束 | 候補 spec 名の案 | 依存する既存 spec | 波の案 |
| ---: | --- | --- | --- | --- |
| 1 | スクリーンセーバー | `areka-P0-screensaver-events` | **0 本** | 第 4 波 |
| 1 | バッテリー | `areka-P0-battery-events` | **0 本** | 第 4 波 |
| 2 | OS の変化の察知 | `areka-P0-os-state-events` | **0 本** | 第 4 波 |
| 2 | ディスプレイ変化 | `areka-P0-display-change-events` | **0 本** | 第 4 波 |
| 3 | 最小化 | `areka-P0-minimize-state` | **0 本** | 第 4 波 |
| 4 | 壁紙 | `areka-P0-wallpaper-events` | **0 本** | 第 4 波 |
| 4 | 通知領域 | `areka-P0-tray-presence` | **0 本** | 第 4 波 |
| 5 | スリープ復帰 | `areka-P0-sleep-resume-events` | **0 本** | 第 4 波 |
| 6 | フルスクリーン退避 | `areka-P0-fullscreen-retreat` | **0 本** | 第 4 波 |
| 7 | ごみ箱 | `areka-P0-recyclebin-query` | **0 本** | 第 4 波 |
| 8 | 一覧と汎用プロパティの照会 | `areka-P0-generic-property-query`（残余） | `areka-P0-property-catalog-lists`（W16・27 件）／`areka-P0-currentghost-property-tree`（W15・4 件）／`areka-P0-sylphya`（完了・2 件） | 第 4 波 |
| 9 | サウンド | `areka-P0-sound-playback`（残余） | `areka-P0-property-catalog-lists`（W16・21 件）／`areka-P0-sakura-time-directives`（W16・1 件） | 第 4 波 |
| 9 | 予定表 | `areka-P0-schedule-query` | **0 本** | 第 4 波 |
| 10 | 環境の照会 | `areka-P0-system-property-query`（残余） | `areka-P0-property-catalog-lists`（W16・25 件）／`areka-P0-property-query-channels`（W14・3 件） | 第 4 波 |

## 段階 D

節目は「仲間がいる」である（定義は `briefing.md` 2-1）。並びは `briefing.md` 3-3 の
順位表のままで、**7 行**ある。波は第 5 波。

| 順位 | 束 | 候補 spec 名の案 | 依存する既存 spec | 波の案 |
| ---: | --- | --- | --- | --- |
| 1 | 呼び出し | `areka-P0-ghost-call` | **0 本** | 第 5 波 |
| 2 | SSTP | `areka-P0-sstp-server` | `areka-P0-balloon-canon-residue`（W14・7 件） | 第 5 波 |
| 2 | コミュニケート | `areka-P0-communicate-events` | **0 本** | 第 5 波 |
| 3 | FMO | `areka-P0-fmo-share` | **0 本** | 第 5 波 |
| 4 | 多重ゴースト | `areka-P0-multi-ghost` | `areka-P0-property-catalog-lists`（W16・5 件）／`areka-P0-property-query-channels`（W14・2 件） | 第 5 波 |
| 5 | PLUGIN | `areka-P0-plugin-host` | `areka-P0-property-catalog-lists`（W16・8 件） | 第 5 波 |
| 6 | リンク | `areka-P0-ukagaka-link` | **0 本** | 第 5 波 |

## 段階 E

節目は「周辺」である（定義は `briefing.md` 2-1）。並びは `briefing.md` 3-3 の
順位表のままで、**9 行**ある。波は第 6 波。

| 順位 | 束 | 候補 spec 名の案 | 依存する既存 spec | 波の案 |
| ---: | --- | --- | --- | --- |
| 1 | 外部アプリ | `areka-P0-external-app-bridge` | **0 本** | 第 6 波 |
| 2 | 開発者機能 | `areka-P0-developer-tools` | **0 本** | 第 6 波 |
| 3 | ヘッドライン | `areka-P0-headline-host` | `areka-P0-property-catalog-lists`（W16・6 件） | 第 6 波 |
| 4 | トランスレータ | `areka-P0-translator-canon`（残余） | `areka-P0-makoto-dll-host`（W16・4 件）／`areka-P0-translate-pipeline`（W15・1 件） | 第 6 波 |
| 5 | 薦める場所 | `areka-P0-recommend-sites` | **0 本** | 第 6 波 |
| 6 | 作り付けの窓 | `areka-P0-baseware-windows` | **0 本** | 第 6 波 |
| 6 | 読み上げと聞き取り | `areka-P0-voice-io` | **0 本** | 第 6 波 |
| 7 | 書庫 | `areka-P0-archive-io` | **0 本** | 第 6 波 |
| 8 | `ukadoc:memo` | なし（ベースウェアの機能比較表と雑多な覚え書きのページで、作る振る舞いを指していない） | **0 本** | 第 6 波 |

## 先頭ウェーブ

先頭ウェーブ（M2 の第 1 波）に入れる束は **6** である——段階 A の順位 1〜6 で、`briefing.md`
3-3 の段階 A の順位表で同順位が現れる手前までを採った。順位 7 で初めて 2 つの束が同じ順位に
並ぶので、そこから先を入れるには同順位の解消（下の「裁定候補」）が要る。この 6 束には
`briefing.md` 3-4 の順序の印（`override`）が **0 行**、2-5 の段階の裁定候補に挙がった束が
**0 束**含まれる（数え方: 3-4 の 2 行と 2-5 の 10 件が名指す束の名前を、この 6 つと突き合わせた）。

6 束の構成 id は合わせて **324 件**である。うち **85 件**を進行中の spec が、
**29 件**を封じた spec が `owner` に持つ（数え方: 6 束の `members` を台帳の `owner` で
引き、宛先が正本の spec 台帳にある名前か `completed/` の名前かで分けて数えた）。

下の状態の数は **2026-09-13 に台帳 4 本を引いて数えた写真**である（数え方: 束の構成 id を
台帳の `status` で引いて状態ごとに数えた）。生きた値の正本は台帳であり、この節の数は
判定が数え直さない。

この節はそのまま `/kiro-discovery` 再入の入力になる。束ごとに、候補 spec 名の案・3 行の要約
（問題・現状・何が変わるか）・依存する既存 spec・構成 id の全列挙を持たせた。

### 会話（段階 A・順位 1）

**候補 spec 名の案**: `areka-P0-talk-script-canon`

**3 行の要約**

- 問題: 台本を読み上げてバルーンへ文字を送る中核でありながら、送りと待ちと改行と選択の待ちに関わる正典の語彙の半分を超える分が未対応か語彙だけで、雛形が書く綴りに当たると黙って落ちる。
- 現状: 構成 46 件の状態は実装済み 16・未対応 18・語彙のみ 10・縮退 2 で、`briefing.md` 7-7 が数えた「一般化で壊れる」75 件のうち 14 件がこの束にある。
- 何が変わるか: 里々製・ヤヤ製の雛形が書く会話の綴りが素通りせずに再生され、バルーンの寿命と選択の待ちが台本の指定で決まるようになる。

**依存する既存 spec**: `areka-P0-balloon-canon-residue`（W14・5 件）／`areka-P0-sakura-time-directives`（W16・5 件）／`areka-P0-anchor-tag-canon`（W17・1 件）／`areka-P0-status-execution-states`（W15・1 件）／`areka-P0-kero-balloon`（完了・2 件）／`areka-P0-cursor-tag-canon`（完了・1 件）

**構成 id（全 46 件）**

- `ukadoc:list_sakura_script:_5c0_3082_3057_304f_306f_5ch:1`
- `ukadoc:list_sakura_script:_5c1_3082_3057_304f_306f_5cu:1`
- `ukadoc:list_sakura_script:_5cC:1`
- `ukadoc:list_sakura_script:_5c_21_5bquicksection_2cfalse_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bquicksection_2ctrue_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2cautoscroll_2cdisable_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2cautoscroll_2cenable_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2cballoontimeout_2c_6642_9593_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2cballoonwait_2c_500d_7387_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2cchoicetimeout_2c_6642_9593_5d:1`
- `ukadoc:list_sakura_script:_5c_2a:1`
- `ukadoc:list_sakura_script:_5c__21:1`
- `ukadoc:list_sakura_script:_5c__3f:1`
- `ukadoc:list_sakura_script:_5c__q_5bID_2c..._5d:1`
- `ukadoc:list_sakura_script:_5c__w_5b_6642_9593_5d:1`
- `ukadoc:list_sakura_script:_5c_a_5bID_2cr2_2cr3..._5d:1`
- `ukadoc:list_sakura_script:_5c_q:1`
- `ukadoc:list_sakura_script:_5c_s_5bID1_2cID2_2cID3..._5d:1`
- `ukadoc:list_sakura_script:_5c_w_5b_6642_9593_5d:1`
- `ukadoc:list_sakura_script:_5cc:1`
- `ukadoc:list_sakura_script:_5ce:1`
- `ukadoc:list_sakura_script:_5cn:1`
- `ukadoc:list_sakura_script:_5cn_5b_30d1_30fc_30bb_30f3_30c8_5d:1`
- `ukadoc:list_sakura_script:_5cn_5bhalf_5d:1`
- `ukadoc:list_sakura_script:_5cp_5bID_756a_53f7_5d:1`
- `ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cID1_2cID2_2cID3..._5d:1`
- `ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cID_2cr2_2cr3..._5d:1`
- `ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cOnID_2cr0_2cr1_2c..._5d:1`
- `ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cscript_3a_5b9f_884c_5185_5bb9_5d:1`
- `ukadoc:list_sakura_script:_5cs_5bID_756a_53f7_5d:1`
- `ukadoc:list_sakura_script:_5ct:1`
- `ukadoc:list_sakura_script:_5cw_6642_9593:1`
- `ukadoc:list_sakura_script:_5cx_5bnoclear_5d:1`
- `ukadoc:list_shiori_event:OnAnchorEnter:1`
- `ukadoc:list_shiori_event:OnAnchorHover:1`
- `ukadoc:list_shiori_event:OnAnchorSelect:1`
- `ukadoc:list_shiori_event:OnAnchorSelectEx:1`
- `ukadoc:list_shiori_event:OnBalloonBreak:1`
- `ukadoc:list_shiori_event:OnBalloonClose:1`
- `ukadoc:list_shiori_event:OnBalloonTimeout:1`
- `ukadoc:list_shiori_event:OnChoiceEnter:1`
- `ukadoc:list_shiori_event:OnChoiceHover:1`
- `ukadoc:list_shiori_event:OnChoiceSelect:1`
- `ukadoc:list_shiori_event:OnChoiceSelectEx:1`
- `ukadoc:list_shiori_event:OnChoiceTimeout:1`
- `ukadoc:list_shiori_resource:balloon_tooltip:1`

### 窓の配置と重なり（段階 A・順位 2）

**候補 spec 名の案**: `areka-P0-window-placement-canon`

**3 行の要約**

- 問題: 立ち位置・相方との隣接・バルーンの貼り付き・重なり順が 1 つの束で決まるのに、構成の 3 分の 2 が未対応で、作者が定義ファイルに書いた位置の指定がほとんど効かない。
- 現状: 構成 126 件の状態は実装済み 7・未対応 84・語彙のみ 30・縮退 5 で、壊れる 75 件のうち 7 件がここにある。M1 の実機一周はこの束を 8 項目（項目 1・9・10・12・16・17・18・19）で見て全部合格しているが、それは emo2 の 1 体で使う範囲である。
- 何が変わるか: 作者が書いた原点と余白と重なりの指定どおりに二体とバルーンが並び、拡大率を変えても隣接が崩れなくなる。

**依存する既存 spec**: `areka-P0-currentghost-property-tree`（W15・16 件）／`areka-P0-surfaces-basepos`（W13 任意／W14・2 件）／`areka-P0-sakura-time-directives`（W16・1 件）／`areka-P0-scope-zorder-pinning`（完了・3 件）／`areka-P0-windowposition-limit`（完了・3 件）／`areka-P0-balloon-offset-dpi`（完了・2 件）

**構成 id（全 126 件）**

- `ukadoc:descript_balloon:dpi_2c_63a8_5968DPI:1`
- `ukadoc:descript_balloon:windowposition.limit_2c0_2f1:1`
- `ukadoc:descript_balloon:windowposition.x_2c_5ea7_6a19:1`
- `ukadoc:descript_balloon:windowposition.y_2c_5ea7_6a19:1`
- `ukadoc:descript_ghost:balloon.dontmove_2ctrue:1`
- `ukadoc:descript_ghost:balloon.syncscale_2ctrue:1`
- `ukadoc:descript_ghost:char_2a.defaultleft_2cX_5ea7_6a19:1`
- `ukadoc:descript_ghost:char_2a.defaulttop_2cY_5ea7_6a19:1`
- `ukadoc:descript_ghost:char_2a.defaultx_2cX_5ea7_6a19:1`
- `ukadoc:descript_ghost:char_2a.defaulty_2cY_5ea7_6a19:1`
- `ukadoc:descript_ghost:char_2a.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1`
- `ukadoc:descript_ghost:kero.defaultleft_2cX_5ea7_6a19:1`
- `ukadoc:descript_ghost:kero.defaulttop_2cY_5ea7_6a19:1`
- `ukadoc:descript_ghost:kero.defaultx_2cX_5ea7_6a19:1`
- `ukadoc:descript_ghost:kero.defaulty_2cY_5ea7_6a19:1`
- `ukadoc:descript_ghost:kero.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1`
- `ukadoc:descript_ghost:sakura.defaultleft_2cX_5ea7_6a19:1`
- `ukadoc:descript_ghost:sakura.defaulttop_2cY_5ea7_6a19:1`
- `ukadoc:descript_ghost:sakura.defaultx_2cX_5ea7_6a19:1`
- `ukadoc:descript_ghost:sakura.defaulty_2cY_5ea7_6a19:1`
- `ukadoc:descript_ghost:sakura.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1`
- `ukadoc:descript_ghost:seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1`
- `ukadoc:descript_shell:char_2a.balloon.dontmove_2c_6570_5024:1`
- `ukadoc:descript_shell:char_2a.balloon.syncscale_2ctrue:1`
- `ukadoc:descript_shell:char_2a.defaultleft_2cX_5ea7_6a19:1`
- `ukadoc:descript_shell:char_2a.defaulttop_2cY_5ea7_6a19:1`
- `ukadoc:descript_shell:char_2a.defaultx_2cX_5ea7_6a19:1`
- `ukadoc:descript_shell:char_2a.defaulty_2cY_5ea7_6a19:1`
- `ukadoc:descript_shell:char_2a.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1`
- `ukadoc:descript_shell:kero.balloon.alignment_2c_4f4d_7f6e_60c5_5831:1`
- `ukadoc:descript_shell:kero.balloon.dontmove_2c_6570_5024:1`
- `ukadoc:descript_shell:kero.balloon.offsetx_2c_5ea7_6a19:1`
- `ukadoc:descript_shell:kero.balloon.offsety_2c_5ea7_6a19:1`
- `ukadoc:descript_shell:kero.balloon.syncscale_2ctrue:1`
- `ukadoc:descript_shell:kero.defaultleft_2cX_5ea7_6a19:1`
- `ukadoc:descript_shell:kero.defaulttop_2cY_5ea7_6a19:1`
- `ukadoc:descript_shell:kero.defaultx_2cX_5ea7_6a19:1`
- `ukadoc:descript_shell:kero.defaulty_2cY_5ea7_6a19:1`
- `ukadoc:descript_shell:kero.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1`
- `ukadoc:descript_shell:sakura.balloon.alignment_2c_4f4d_7f6e_60c5_5831:1`
- `ukadoc:descript_shell:sakura.balloon.dontmove_2c_6570_5024:1`
- `ukadoc:descript_shell:sakura.balloon.offsetx_2c_5ea7_6a19:1`
- `ukadoc:descript_shell:sakura.balloon.offsety_2c_5ea7_6a19:1`
- `ukadoc:descript_shell:sakura.balloon.syncscale_2ctrue:1`
- `ukadoc:descript_shell:sakura.defaultleft_2cX_5ea7_6a19:1`
- `ukadoc:descript_shell:sakura.defaulttop_2cY_5ea7_6a19:1`
- `ukadoc:descript_shell:sakura.defaultx_2cX_5ea7_6a19:1`
- `ukadoc:descript_shell:sakura.defaulty_2cY_5ea7_6a19:1`
- `ukadoc:descript_shell:sakura.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1`
- `ukadoc:descript_shell:seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1`
- `ukadoc:descript_shell:seriko.dpi_2c_63a8_5968DPI:1`
- `ukadoc:descript_shell:seriko.sticky-window_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c...:1`
- `ukadoc:descript_shell:seriko.zorder_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c...:1`
- `ukadoc:descript_shell_surfaces:balloon.offsetx_2c_5ea7_6a19:1`
- `ukadoc:descript_shell_surfaces:balloon.offsety_2c_5ea7_6a19:1`
- `ukadoc:descript_shell_surfaces:kero.balloon.offsetx_2c_5ea7_6a19:1`
- `ukadoc:descript_shell_surfaces:kero.balloon.offsety_2c_5ea7_6a19:1`
- `ukadoc:descript_shell_surfaces:point.basepos.x_2c_5ea7_6a19:1`
- `ukadoc:descript_shell_surfaces:point.basepos.y_2c_5ea7_6a19:1`
- `ukadoc:descript_shell_surfaces:point.centerx_2c_5ea7_6a19:1`
- `ukadoc:descript_shell_surfaces:point.centery_2c_5ea7_6a19:1`
- `ukadoc:descript_shell_surfaces:point.kinoko.centerx_2c_5ea7_6a19:1`
- `ukadoc:descript_shell_surfaces:point.kinoko.centery_2c_5ea7_6a19:1`
- `ukadoc:descript_shell_surfaces:sakura.balloon.offsetx_2c_5ea7_6a19:1`
- `ukadoc:descript_shell_surfaces:sakura.balloon.offsety_2c_5ea7_6a19:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.rect:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.scaling:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.x:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.y:1`
- `ukadoc:list_propertysystem:currentghost.scope.count:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.currentmonitor.bpp:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.currentmonitor.dpi:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.currentmonitor.primary:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.currentmonitor.rect:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.currentmonitor.work:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.rect:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.scaling:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.surface.x:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.surface.y:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.x:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.y:1`
- `ukadoc:list_propertysystem:currentghost.seriko.sticky-window:1`
- `ukadoc:list_propertysystem:currentghost.seriko.zorder:1`
- `ukadoc:list_sakura_script:_5c4:1`
- `ukadoc:list_sakura_script:_5c5:1`
- `ukadoc:list_sakura_script:_5c_21_5bexecute_2cresetballoonpos_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bexecute_2cresetwindowpos_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5block_2cballoonmove_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5block_2cballoonrepaint_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5block_2crepaint_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bmove_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bmoveasync_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5breset_2cposition_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5breset_2csticky-window_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5breset_2czorder_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2calignmentondesktop_2cbottom_307e_305f_306ftop_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2calignmenttodesktop_2c_65b9_5411_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2calignmenttodesktop_2cfree_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2calpha_2c_6570_5024_2c_30aa_30d7_30b7_30e7_30f3_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2cballoonalign_2cID_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2cballoonoffset_2cx_2cy_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2cposition_2cx_2cy_2c_30b9_30b3_30fc_30d7ID_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2csticky-window_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c..._5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2cwindowstate_2c_21stayontop_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2cwindowstate_2cstayontop_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bset_2czorder_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c..._5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bunlock_2cballoonmove_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bunlock_2cballoonrepaint_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bunlock_2crepaint_5d:1`
- `ukadoc:list_sakura_script:_5cv:1`
- `ukadoc:list_shiori_event:OnOffscreen:1`
- `ukadoc:list_shiori_event:OnOverlap:1`
- `ukadoc:list_shiori_event:OnResetWindowPos:1`
- `ukadoc:list_shiori_event:hwnd:1`
- `ukadoc:list_shiori_resource:char_2a.defaultleft:1`
- `ukadoc:list_shiori_resource:char_2a.defaulttop:1`
- `ukadoc:list_shiori_resource:char_2a.defaultx:1`
- `ukadoc:list_shiori_resource:char_2a.defaulty:1`
- `ukadoc:list_shiori_resource:kero.defaultleft:1`
- `ukadoc:list_shiori_resource:kero.defaulttop:1`
- `ukadoc:list_shiori_resource:kero.defaultx:1`
- `ukadoc:list_shiori_resource:kero.defaulty:1`
- `ukadoc:list_shiori_resource:sakura.defaultleft:1`
- `ukadoc:list_shiori_resource:sakura.defaulttop:1`
- `ukadoc:list_shiori_resource:sakura.defaultx:1`
- `ukadoc:list_shiori_resource:sakura.defaulty:1`

### 名前の記憶（段階 A・順位 3）

**候補 spec 名の案**: `areka-P0-user-name-memory`

**3 行の要約**

- 問題: 名前を尋ねて覚え、次から呼びかける——伺かの象徴的な振る舞いの口が、保存の側と読み戻しの側の両方で欠けている。
- 現状: 構成 22 件の状態は実装済み 9・未対応 11・語彙のみ 2 で、壊れる 75 件のうち 2 件がここにある（`ukadoc:list_shiori_event:OnNotifyUserInfo:1` と `ukadoc:descript_ghost:name_2c_30b4_30fc_30b9_30c8_540d:1`）。
- 何が変わるか: 名前を尋ねる会話が最後まで成立し、覚えた名前が起動をまたいで残り、台詞の中の呼びかけに使えるようになる。

**依存する既存 spec**: `areka-P0-currentghost-property-tree`（W15・1 件）／`areka-P0-package-mount`（完了・2 件）／`areka-P0-sylphya`（完了・2 件）／`areka-P0-sakura-dialogue-tags`（完了・1 件）

**構成 id（全 22 件）**

- `ukadoc:descript_ghost:char_2a.name_2c_540d_524d:1`
- `ukadoc:descript_ghost:kero.name_2c_540d_524d:1`
- `ukadoc:descript_ghost:name.allowoverride_2c_6570_5024:1`
- `ukadoc:descript_ghost:name_2c_30b4_30fc_30b9_30c8_540d:1`
- `ukadoc:descript_ghost:sakura.name2_2c_540d_524d:1`
- `ukadoc:descript_ghost:sakura.name_2c_540d_524d:1`
- `ukadoc:descript_shell:char_2a.name_2c_540d_524d:1`
- `ukadoc:descript_shell:kero.name_2c_540d_524d:1`
- `ukadoc:descript_shell:sakura.name_2c_540d_524d:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.name:1`
- `ukadoc:list_sakura_script:_25keroname:1`
- `ukadoc:list_sakura_script:_25selfname2:1`
- `ukadoc:list_sakura_script:_25selfname:1`
- `ukadoc:list_sakura_script:_25username:1`
- `ukadoc:list_sakura_script:_5c_21_5bopen_2cteachbox_5d:1`
- `ukadoc:list_shiori_event:OnNotifyUserInfo:1`
- `ukadoc:list_shiori_event:OnTeach:1`
- `ukadoc:list_shiori_event:OnTeachInputCancel:1`
- `ukadoc:list_shiori_event:OnTeachStart:1`
- `ukadoc:list_shiori_event:installedkeroname:1`
- `ukadoc:list_shiori_event:installedsakuraname:1`
- `ukadoc:list_shiori_resource:username:1`

### 起動と挨拶（段階 A・順位 4）

**候補 spec 名の案**: `areka-P0-boot-greeting-canon`

**3 行の要約**

- 問題: 定義ファイルを読んでゴーストを組み立て、SHIORI を読み込んで最初の 3 つのイベントを送るまでの口のうち、作者名や種別といった素性の欄が未対応で、ゴースト一覧に正しい名前が出ない。
- 現状: 構成 20 件の状態は実装済み 5・未対応 11・語彙のみ 4 で、壊れる 75 件のうち 4 件がここにあり、4 件とも素性の欄である。
- 何が変わるか: 配布されたゴーストを入れると素性が読み取られ、初回と 2 回目以降の挨拶が正典の順で送られるようになる。

**依存する既存 spec**: `areka-P0-charset-canon`（W13・2 件）／`areka-P0-package-mount`（完了・1 件）

**構成 id（全 20 件）**

- `ukadoc:descript_ghost:charset_2c_6587_5b57_30b3_30fc_30c9:1`
- `ukadoc:descript_ghost:craftman_2c_4f5c_8005_540d:1`
- `ukadoc:descript_ghost:craftmanurl_2cURL:1`
- `ukadoc:descript_ghost:craftmanw_2c_4f5c_8005_540d:1`
- `ukadoc:descript_ghost:id_2cID_540d:1`
- `ukadoc:descript_ghost:shiori.cache_2c_6570_5024:1`
- `ukadoc:descript_ghost:shiori.encoding_2c_6587_5b57_30b3_30fc_30c9:1`
- `ukadoc:descript_ghost:shiori.escape_unknown_2c0_2f1:1`
- `ukadoc:descript_ghost:shiori.forceencoding_2c_6587_5b57_30b3_30fc_30c9:1`
- `ukadoc:descript_ghost:shiori.version_2c_30d0_30fc_30b8_30e7_30f3:1`
- `ukadoc:descript_ghost:shiori_2c_30d5_30a1_30a4_30eb_540d:1`
- `ukadoc:descript_ghost:title_2c_8868_793a_540d:1`
- `ukadoc:descript_ghost:type_2c_7a2e_5225:1`
- `ukadoc:list_shiori_event:OnBoot:1`
- `ukadoc:list_shiori_event:OnFirstBoot:1`
- `ukadoc:list_shiori_event:OnInitialize:1`
- `ukadoc:list_shiori_resource:craftman:1`
- `ukadoc:list_shiori_resource:craftmanw:1`
- `ukadoc:list_shiori_resource:name:1`
- `ukadoc:list_shiori_resource:version:1`

### バルーンの文字（段階 A・順位 5）

**候補 spec 名の案**: `areka-P0-balloon-font-canon`（残余）

**3 行の要約**

- 問題: 作者が選んだ書体も文字色も効かず、どのゴーストも同じ見た目で喋る。
- 現状: 構成 63 件の状態は実装済み 13・未対応 32・語彙のみ 10・縮退 8 で、壊れる 75 件のうち 4 件がここにある。63 件のうち 45 件は進行中の 2 spec が `owner` に持つので、この束に残る仕事はその残余である。
- 何が変わるか: 作者の書体と色の指定が画面に出て、縮退している 8 件が正典どおりの振る舞いへ戻る。

**依存する既存 spec**: `areka-P0-text-decoration-canon`（W13・32 件）／`areka-P0-currentghost-property-tree`（W15・13 件）／`areka-P0-balloon-parse`（完了・5 件）／`areka-P0-balloon-vertical-canon`（完了・4 件）／`areka-P0-cursor-tag-canon`（完了・1 件）

**構成 id（全 63 件）**

- `ukadoc:descript_balloon:disable.font._28_30d5_30a9_30f3_30c8_5b9a_7fa9_29_2c_28_6307_5b9a_29:1`
- `ukadoc:descript_balloon:font.bold_2c0_2f1:1`
- `ukadoc:descript_balloon:font.color.b_2c_6570_5024:1`
- `ukadoc:descript_balloon:font.color.g_2c_6570_5024:1`
- `ukadoc:descript_balloon:font.color.r_2c_6570_5024:1`
- `ukadoc:descript_balloon:font.height_2c_6570_5024:1`
- `ukadoc:descript_balloon:font.italic_2c0_2f1:1`
- `ukadoc:descript_balloon:font.name_2c_30d5_30a9_30f3_30c8_540d:1`
- `ukadoc:descript_balloon:font.outline_2c0_2f1:1`
- `ukadoc:descript_balloon:font.shadowcolor.b_2c_6570_5024:1`
- `ukadoc:descript_balloon:font.shadowcolor.g_2c_6570_5024:1`
- `ukadoc:descript_balloon:font.shadowcolor.r_2c_6570_5024:1`
- `ukadoc:descript_balloon:font.shadowstyle_2c_5f62_614b_6307_5b9a:1`
- `ukadoc:descript_balloon:font.strike_2c0_2f1:1`
- `ukadoc:descript_balloon:font.underline_2c0_2f1:1`
- `ukadoc:descript_balloon:origin.x_2c_5ea7_6a19_20_2a1:1`
- `ukadoc:descript_balloon:origin.y_2c_5ea7_6a19_20_2a1:1`
- `ukadoc:descript_balloon:validrect.bottom_2c_5ea7_6a19_20_2a1:1`
- `ukadoc:descript_balloon:validrect.left_2c_5ea7_6a19_20_2a1:1`
- `ukadoc:descript_balloon:validrect.right_2c_5ea7_6a19_20_2a1:1`
- `ukadoc:descript_balloon:validrect.top_2c_5ea7_6a19_20_2a1:1`
- `ukadoc:descript_balloon:vertical_2c0_2f1:1`
- `ukadoc:descript_balloon:wordwrappoint.x_2c_5ea7_6a19_20_2a1:1`
- `ukadoc:descript_balloon:wordwrappoint.y_2c_5ea7_6a19_20_2a1:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.background.color:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.basepos.x:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.basepos.y:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.char_width:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.count:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.lines.initial:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.lines:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.num:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.validheight.initial:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.validheight:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.validwidth.initial:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.validwidth:1`
- `ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.vertical:1`
- `ukadoc:list_sakura_script:_5c_26_5bID_5d:1`
- `ukadoc:list_sakura_script:_5c__w_5banimation_2cID_5d:1`
- `ukadoc:list_sakura_script:_5c_l_5bx_2cy_5d:1`
- `ukadoc:list_sakura_script:_5c_m_5b0x00_5d:1`
- `ukadoc:list_sakura_script:_5c_n:1`
- `ukadoc:list_sakura_script:_5c_u_5b0x0000_5d:1`
- `ukadoc:list_sakura_script:_5cc_5bchar_2c_6570_5024_2c_958b_59cb_4f4d_7f6e_5d:1`
- `ukadoc:list_sakura_script:_5cc_5bline_2c_6570_5024_2c_958b_59cb_4f4d_7f6e_5d:1`
- `ukadoc:list_sakura_script:_5cf_5balign_2c_5bc4_305b_308b_5074_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bbold_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bcolor_2c_8272_6307_5b9a_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bdefault_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bdisable_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bheight_2c_6570_5024_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bitalic_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bname_2c_30d5_30a9_30f3_30c8_540d_5d:1`
- `ukadoc:list_sakura_script:_5cf_5boutline_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bshadowcolor_2c_8272_6307_5b9a_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bshadowcolor_2cnone_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bshadowstyle_2c_5f62_614b_6307_5b9a_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bstrike_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bsub_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bsup_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bunderline_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
- `ukadoc:list_sakura_script:_5cf_5bvalign_2c_5bc4_305b_308b_5074_5d:1`
- `ukadoc:list_shiori_event:OnNotifyFontInfo:1`

### サーフェスアニメーション（段階 A・順位 6）

**候補 spec 名の案**: `areka-P0-seriko-animation-canon`

**3 行の要約**

- 問題: 目も口も動かない止め絵のまま立ち、まばたきも面の切り替えも作者の定義どおりには動かない。
- 現状: 構成 47 件の状態は実装済み 2・未対応 27・語彙のみ 16・縮退 2 で、壊れる 75 件のうち 4 件がここにある。M1 の実機一周はこの束を 3 項目（項目 3・4・10）で見て合格しているが、それは emo2 が使う範囲の再生である。
- 何が変わるか: シェルの定義に書かれた再生の指定が一通り効き、面の切り替えと重ね絵が作者の意図どおりに動く。

**依存する既存 spec**: `areka-P0-currentghost-property-tree`（W15・6 件）／`areka-P0-shell-parse`（完了・2 件）

**構成 id（全 47 件）**

- `ukadoc:descript_shell_surfaces:alternativestart_2c_28ID1_2cID2..._29:1`
- `ukadoc:descript_shell_surfaces:alternativestop_2c_28ID1_2cID2..._29:1`
- `ukadoc:descript_shell_surfaces:always:1`
- `ukadoc:descript_shell_surfaces:animation-sort_2c_30bd_30fc_30c8_9806_5e8f:1`
- `ukadoc:descript_shell_surfaces:animation_2a.interval_2c_30a4_30f3_30bf_30fc_30d0_30eb:1`
- `ukadoc:descript_shell_surfaces:animation_2a.name_2c_5b9a_7fa9_540d:1`
- `ukadoc:descript_shell_surfaces:animation_2a.option_2c_30aa_30d7_30b7_30e7_30f3:1`
- `ukadoc:descript_shell_surfaces:animation_2a.option_2cbackground:1`
- `ukadoc:descript_shell_surfaces:animation_2a.option_2cexclusive:1`
- `ukadoc:descript_shell_surfaces:animation_2a.option_2cshared-index:1`
- `ukadoc:descript_shell_surfaces:animation_2a.pattern_2a_2c_63cf_753b_30e1_30bd_30c3_30c9_2c_30b5_30fc_30d5_30a7_30b9_756a_53f7_2c_30a6_30a7_30a4_30c8_2c:1`
- `ukadoc:descript_shell_surfaces:endtalk:1`
- `ukadoc:descript_shell_surfaces:insert_2cID:1`
- `ukadoc:descript_shell_surfaces:never:1`
- `ukadoc:descript_shell_surfaces:parallelstart_2c_28ID1_2cID2..._29:1`
- `ukadoc:descript_shell_surfaces:parallelstop_2c_28ID1_2cID2..._29:1`
- `ukadoc:descript_shell_surfaces:periodic_2c_6570_5024:1`
- `ukadoc:descript_shell_surfaces:random_2c_6570_5024:1`
- `ukadoc:descript_shell_surfaces:rarely:1`
- `ukadoc:descript_shell_surfaces:runonce:1`
- `ukadoc:descript_shell_surfaces:sometimes:1`
- `ukadoc:descript_shell_surfaces:start_2cID:1`
- `ukadoc:descript_shell_surfaces:starttalk:1`
- `ukadoc:descript_shell_surfaces:stop_2cID:1`
- `ukadoc:descript_shell_surfaces:talk_2c_6570_5024:1`
- `ukadoc:descript_shell_surfaces:yen-e:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.animation.num:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.seriko.defaultsurface:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.surface.num:1`
- `ukadoc:list_propertysystem:currentghost.scope_28ID_29.surface_28ID_29.rect:1`
- `ukadoc:list_propertysystem:currentghost.seriko.surfacelist.all:1`
- `ukadoc:list_propertysystem:currentghost.seriko.surfacelist.defined:1`
- `ukadoc:list_sakura_script:_5c_21_5banim_2cadd_2coverlay_2cID_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5banim_2cadd_2ctext_2cx_2cy_2c_6a2a_5e45_2c_7e26_5e45_2c_6587_5b57_5217_2c_8868_793a_6642_9593_2cr_2cg_2cb_2c_658:1`
- `ukadoc:list_sakura_script:_5c_21_5banim_2cclear_2cID_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5banim_2coffset_2cID_2cx_5ea7_6a19_2cy_5ea7_6a19_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5banim_2cpause_2cID_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5banim_2cresume_2cID_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5banim_2cstop_2cID_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5beffect2_2c_8ffd_52a0_30b5_30fc_30d5_30a7_30b9ID_2c_30d7_30e9_30b0_30a4_30f3_540d_2c_901f_5ea6_500d_7387_2c_30d1:1`
- `ukadoc:list_sakura_script:_5c_21_5beffect_2c_30d7_30e9_30b0_30a4_30f3_540d_2c_901f_5ea6_500d_7387_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bfilter_2c_30d7_30e9_30b0_30a4_30f3_540d_2c_8d77_52d5_6642_9593_2c_30d1_30e9_30e1_30fc_30bf_5d:1`
- `ukadoc:list_sakura_script:_5c_21_5bfilter_5d:1`
- `ukadoc:list_sakura_script:_5ci_5bID_2cwait_5d:1`
- `ukadoc:list_sakura_script:_5ci_5bID_756a_53f7_5d:1`
- `ukadoc:list_shiori_event:OnSurfaceChange:1`
- `ukadoc:list_shiori_event:OnSurfaceRestore:1`

## M2 予約群の対応表

<!-- 段 5（タスク 6.2）で書く: 予約群の各項目がどの束に写ったか。写らなかった項目は名前と
     理由を書く（0 件なら 0 と書く）。 -->

## 既存 brief への是正候補

<!-- 段 5（タスク 6.2）で書く: 台帳の id と brief の所有宣言が食い違うものを、spec 名・
     食い違う id・直し方の案の 3 列で列挙する。brief の本体は書き換えない。 -->

## 別軸

M2 の技術選定 4 つ——pasta の native x64・`IShiori` の in-proc 化・ベクトル描画・AI——は、
段階にも波にも並べず、順位も付けない。この 1 節だけに置く。

| 技術選定 | 台帳の項目 | 順位 | 波 |
| --- | ---: | --- | --- |
| pasta の native x64 | 0 件 | 付けない | 割り当てない |
| `IShiori` の in-proc 化 | 0 件 | 付けない | 割り当てない |
| ベクトル描画 | 0 件 | 付けない | 割り当てない |
| AI | 0 件 | 付けない | 割り当てない |

「台帳の項目 0 件」の根拠と数え方は `linkage.md`「別軸」にある——4 つはいずれも正典の記述では
なく areka の内側の実装方式の選択で、カタログにも台帳 4 本にも対応する項目が立っていない。
構成 id を列挙できないので、順位の 4 つの根拠（壊れ方・テーマの数・影響する既存資産の広さ・
依存基盤の共有度）はどれも構成 id から導く値であり、この 4 つを同じ物差しに載せられない。

波を割り当てないのも同じ理由である。どの技術選定を採っても段階と束の順序は変わらず、変わるのは
1 つの束を実装するときの中身の作り方だけである。したがってこの 4 つは、段階の節の 67 行の
どこにも現れない（**0 行**。数え方: 5 つの段階の表の「束」の欄を全部読み、4 つの名前を探した）。

## M3 の受入基準の候補

M3「伺かの冠」の受入基準の候補として、**「テーマ 8 つすべてで代表束が実装済み」**を登記する
（要件 5.7）。**決めるのは階梯の議論（棚卸セッション）であり、本文書は候補を置くだけである。**

代表束の決め方も候補が 2 つある。

- 案 ⑴: そのテーマを持つ束のうち、順位表で最も前に来るもの（段階の若い順、同じ段階なら
  順位の小さい順）を代表とする。下の表がその案を当てた結果である。
- 案 ⑵: 1 つの束が 2 つ以上のテーマの代表を兼ねないようにし、テーマごとに別の束を代表とする。

| テーマ | そのテーマを持つ束と単独項目 | 案 ⑴ の代表 | 代表の段階・順位 |
| --- | ---: | --- | --- |
| 気配 | 7 | 会話 | 段階 A・順位 1 |
| 触れ合い | 7 | キーとゲームパッド | 段階 A・順位 9 |
| 掛け合い | 7 | 会話 | 段階 A・順位 1 |
| 装い | 25 | 窓の配置と重なり | 段階 A・順位 2 |
| 記憶 | 9 | 名前の記憶 | 段階 A・順位 3 |
| 交わり | 13 | 会話 | 段階 A・順位 1 |
| 気配り | 12 | 会話 | 段階 A・順位 1 |
| 更新 | 2 | インストール | 段階 B・順位 1 |

数え方: `linkage.md` の 67 の囲みの `themes` を全部読み、そのテーマを含む囲みを数えて、
`briefing.md` 3-3 の順位表の並びで最も前に来るものを代表に採った。

案 ⑴ を当てると、8 つのテーマのうち **4** つを **1** 束
（会話）が兼ねる。これが案 ⑵ を候補に残す理由である——1 つの束が動いただけで
複数のテーマの受入が同時に立つと、受入基準が数えるものが薄くなる。どちらを採るかも棚卸
セッションの裁定に委ねる。

## 裁定候補

### ウェーブの並べ替え（要件 10.3）

正本のウェーブ編成（W13〜W17）は入力である。この文書は並べ替えを提案として置くだけで、
正本を編集しない。提案は **3 件**である。

**W-1 M2 の波を入れる場所**

- 現在のウェーブ: 正本の編成は W13〜W17 で、M2 の波は 1 つも編成されていない。
- 提案: M2 の第 1 波を W17 の後に置く。
- 理由: 先頭ウェーブ 6 束の構成 324 件のうち 85 件を進行中の spec が
  `owner` に持ち、その宛先の最も遅いウェーブが W17 である。W17 より前に第 1 波を差し込むと、
  同じ id を 2 つの spec が同時に触る。

**W-2 `areka-P0-anchor-tag-canon` を繰り上げるか、宛先を 1 件だけ移すか**

- 現在のウェーブ: W17（正本の編成の最後）。
- 提案: `ukadoc:list_sakura_script:_5c_a_5bID_2cr2_2cr3..._5d:1` の 1 件を先頭ウェーブの候補
  spec `areka-P0-talk-script-canon` の宛先へ移すか、`areka-P0-anchor-tag-canon` を W16 以前へ
  繰り上げる。
- 理由: 先頭ウェーブ 6 束が `owner` に持つ進行中 spec の 85 件のうち、W17 に掛かるのは
  この 1 件だけで、残り 84 件の最も遅いウェーブは W16 である。この 1 件のために第 1 波の
  始まりが 1 ウェーブ遅れる。

**W-3 `areka-P0-surfaces-basepos` の任意を外すか**

- 現在のウェーブ: W13 任意／W14（正本の spec 台帳とウェーブ編成が「任意」と書く）。
- 提案: W13 で確定させる。
- 理由: この spec は先頭ウェーブの束「窓の配置と重なり」の構成 id 2 件——
  `ukadoc:descript_shell_surfaces:point.basepos.x_2c_5ea7_6a19:1` と
  `ukadoc:descript_shell_surfaces:point.basepos.y_2c_5ea7_6a19:1`——を `owner` に持つ。
  任意のまま W14 へ流れると、先頭ウェーブの前提が 1 ウェーブ後ろへずれる。

### 同順位の解消（要件 6.7）

4 つの根拠がすべて同じ値になって同順位に並んだ組は **13 組・29 束**
である（`briefing.md` 3-5。数え方はそこにある）。同順位のままでは組の中のどれを先に作るかが
決まらないので、解消を裁定候補に上げる。**本文書では順序を作っていない**——4 つの根拠のほかに
順序の根拠を足すと、要件 6.1 が凍結した序列に 5 つ目の根拠を足すことになる。

| # | 段階 | 順位 | 同順位の束 | 波の案 |
| ---: | :---: | ---: | --- | --- |
| 1 | A | 7 | 入力窓とダイアログ・自発発話 | 第 2 波 |
| 2 | A | 10 | descript の転記・バルーンのリンク・マウスの矢印 | 第 2 波 |
| 3 | A | 14 | イベントの呼び起こし・選択肢の目印 | 第 2 波 |
| 4 | A | 17 | 定義ファイルの文字コード・組み込みの置換語 | 第 2 波 |
| 5 | A | 18 | SHIORI の要求と応答・シェル定義の転記・同期オブジェクト | 第 2 波 |
| 6 | B | 5 | 休止と復帰・好感度の絵柄・着せ替え | 第 3 波 |
| 7 | B | 8 | `ukadoc:manual_directory`・`ukadoc:manual_ghost` | 第 3 波 |
| 8 | C | 1 | スクリーンセーバー・バッテリー | 第 4 波 |
| 9 | C | 2 | OS の変化の察知・ディスプレイ変化 | 第 4 波 |
| 10 | C | 4 | 壁紙・通知領域 | 第 4 波 |
| 11 | C | 9 | サウンド・予定表 | 第 4 波 |
| 12 | D | 2 | SSTP・コミュニケート | 第 5 波 |
| 13 | E | 6 | 作り付けの窓・読み上げと聞き取り | 第 6 波 |

**先頭ウェーブに掛かる組は 0 組である**（数え方: 上の表の段階が A で順位が 6 以下の
行を数えた）。先頭ウェーブの 6 束は順位 1〜6 に 1 つずつ並んでおり、どれを先に起票するかは
裁定を待たずに決まる。第 2 波より後は、この表の解消が着手順を決める。

**決めること**: 各組の中の順序。**答えで変わること**: その波の中でどの束から手を着けるか。
段階と波は動かないので、利用者から見える節目の順序は変わらない。

### 段階の割り当てと順序の印（`briefing.md` にある裁定候補）

段階の割り当ての裁定候補 **10 件**は `briefing.md` 2-5 に、順序の主張から外した `override` の
**2 行**は同 3-4 にある。どちらも本文書では決めない。裁定で段階が動けば、この文書の段階の節と
波の割り当ても組み直す。

**先頭ウェーブに掛かるものは 0 件である**（数え方: 2-5 の 10 件と 3-4 の 2 行が名指す束の名前を
先頭ウェーブの 6 束と突き合わせ、一致するものを数えた）。掛かるのは第 2 波より後の束だけである。
