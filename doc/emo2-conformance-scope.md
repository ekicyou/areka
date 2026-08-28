# emo2 適合スコープ解剖（M1 実輪郭の正本・discovery 成果）

> 目的: M1 ゴール「最小 SSP 互換ベースウェア」を**実物で定義**するための emo2 解剖結果。
> 適合対象ゴースト emo2（作者 ekicyou・脳=pasta.dll・32bit SHIORI）が「そのまま動く」ために areka が実装すべき機能の正確な輪郭。推測でなく実ファイル根拠。
> 出所: `C:\home\maz\git\ghost_dev\project\emo2\ghost\emo2`（解剖は4並列 subagent・2026-06-28）。
> 注記: 本書は discovery 分析の保全であり、ロードマップ確定前のドラフト。carving 合意後に各 spec へ配分される。

## 0. 前提（最重要・スコープの土台）

- **emo2 の脳 pasta.dll は PE Machine 0x14C = x86(32bit)**。`descript.txt: shiori,pasta.dll`。x64 areka では in-proc 不可 → **host-32（32bit 別プロセスブリッジ）が必須・耐力壁**。
- **SAORI 不使用（確定）**: ゴースト全体で DLL は pasta.dll ただ1個。`saori|x-saori|exec,saori` 等 grep 全ゼロ。→ **host-32 の「SAORI 同居」機能は M1 不要**（旧 host-32 brief から削減）。
- **areka は脳の中身（`.pasta`/`.lua`/`pasta.toml`/budoux/縦書き設定）を一切解釈しない**。すべて pasta.dll の腹の中。areka の責務は SSP 側のみ＝SHIORI ホスト＋SERIKO 描画＋さくらスクリプト解釈＋バルーン描画。
- charset は emo2 では UTF-8（この子は楽）。**汎用互換（里々/YAYA）では Shift_JIS も要る**が、それは emo2 適合後の生態系拡張。

## 1. host-32 / SHIORI（耐力壁）

- プロトコル: **SHIORI/3.0**、`key: value` CRLF ヘッダ、終端空行。`req.version=30`。
- リクエストで emo2 が読むヘッダ: `ID` / `Reference0..n` / `Status`（talking/choosing/online 等9種で OnSecondChange の発火制御）/ `Sender` / `Charset`。
- 応答: `Value:`（さくらスクリプト本体）＋ 常時 `Charset: UTF-8` / `Sender: Pasta` / `SecurityLevel: local`。空時 **204 No Content**、異常 **500 ＋ X-Error-Reason**。**Surface ヘッダは使わない**（サーフェス指定は `\s[]` で）。
- areka が M1 で**送る必要のある SHIORI イベント**:
  - `OnBoot`（初回 `OnFirstBoot`）— 起動挨拶
  - `OnSecondChange` — **最重要・心臓部**。毎秒。これが OnTalk/OnHour/コールバック/kick を内部駆動する
  - `OnMouseDoubleClick` — メニュー
  - `OnChoiceSelectEx` — メニュー選択肢確定（Reference0=選択肢ラベル・Reference1=`\q[title,id]` の id・Reference2 以降=拡張引数）
  - `OnMouseMove` — 撫で反応（areka が collision 解決して actor/region を Reference に載せる）
  - `OnClose` — 終了挨拶＋`\-`
- **送ってはいけない**: `OnTalk`/`OnHour`（emo2 が OnSecondChange 内で内部生成。二重発火になる）。
- **M1 省略可**: `OnUpdate*`4種・`OnBalloonChange`（ネット更新）・`OnMouseClick`単（未ハンドル＝204）。
- host-32 M1 機能: 32bit SHIORI DLL の `load/unload/request` 動的ロード、HGLOBAL/charset マーシャリング、自前 IPC（フレーミング/タイムアウト/プロセス監視）、自前メッセージループ（窓を作る SHIORI 対応）、毎秒ポーリング。**SAORI 同居は M1 範囲外**。

## 2. SERIKO / シェル（seriko-runtime＋surface-hierarchy の M1 実需）

- SERIKO バージョン: **SERIKO/2.0（version 1）**。`seriko.use_self_alpha,1`（PNG 自己アルファ）、`seriko.alignmenttodesktop,bottom`。
- **2方式が同居**:
  - side0（sakura=むらさき/purple）= **MAYUNA bind 着せ替え**。本体 `surface1000` は静的 element ゼロ、全パーツを `animationNNNN.interval,bind` ＋ `pattern0,overlay,id,0,0,0` で構成。8カテゴリ（腕/口/目/まばたき/眉/紅/キラリ/髪飾り）・約30 bind。bindgroup番号=animation番号=ヘルパーsurface番号で三者一致。
  - side1（kero=エモ/CityPop）= **サーフェス丸ごと差し替え**（軽い）。
- **使用 interval は3種のみ**: `bind` / `random,N` / `bind+random,N`。→ sometimes/rarely/periodic/always/runonce/never/yen-e/**talk,n** は**未使用**（口パクは bind 切替で表現）。
- **method は overlay のみ**（＋負ID `overlay,-1`=層クリア）。overlayfast/base/replace/interpolate/asis/**move**/add/reduce 未使用。
- **element 合成**: 大半が単層、まばたき `1410-1413` のみ2層。**全オフセット 0,0**（→ 汎用シーングラフ不要、単純 z-order overlay で足りる）。
- z-order = **animation ID 昇順の画家アルゴリズム**。
- collision: **矩形のみ**、領域名 **Head / Bust の2種**。collisionex（円/楕円/多角形）未使用。
- surface alias: `kero.surface.alias`（感情名→IDリスト）。`\s[静観]` 等の**日本語エイリアス**を受理する必要（→ `\s[]` 中身は数値前提で parse 禁止・不透明文字列としてサーフェス層へ委譲）。
- MAYUNA descript 設定: `bindgroupN.name/.default`、`bindoptionN.group …,mustselect`、`sakura.menu,auto`。
- **M1 省略可**: collisionex・talk lipsync・periodic アニメ・interpolate/move・element 座標オフセット・多コマ周期アニメ（最大3-4コマのまばたきのみ）。
- **落とし穴**: surface1000 まばたきは `bind+random`（目カテゴリ bind ON かつ random）／kero まばたきは通常 `random,4`。2系統の発火モデル差を区別。

## 3. さくらスクリプト（sakura-script の M1 実需）

- **M1 必須タグ（これで emo2 ほぼ全シーン描画可）**:
  - `\p[n]`（話者/スコープ切替・最重要。builder が `\0\1` を `\p[n]` に変換）
  - `\s[ID]` / `\s[エイリアス]`（**不透明文字列扱い**）
  - `\n` / `\n[percent]`（パーセント=割合改行。`\n[150]`=1.5行）
  - `\w[n]` / `\wN`（短縮形）/ `\_w[ms]`（絶対ミリ秒）
  - `\q[disp,target]` ＋ `\![*]`（選択肢マーカー）
  - `\_l[x,y]`（カーソル絶対位置・em/lh 単位・menu に必須）
  - `\e`（終端）/ `\c`（クリア・builder が出す）/ `\-`（終了）
  - `\![move,dx,dy,...,base,base]`（キャラ位置移動・位置調整の中核）
  - `%username`（SSP システム変数展開）
- **`\!` コマンドは `move` だけ本実装、他はスタブで可**（set/get property・choicetimeout・reload は辞書発火なし）。
- **M1 不要**（emo2 未使用）: raise/open/exec/anchor/timerraise/bind 等 `\!` 系、`\b \_b \i \j \& \f[] \_a \_q \_n \x`。なお `\f[]` の語彙は 2026-08-27 に**文字装飾系 3 spec**（`areka-P0-text-decoration-canon`〔核 17 項目＋基盤〕／`areka-P0-anchor-tag-canon`〔アンカー系 16 項目〕／`areka-P0-choice-marker-styling`〔`cursor*` 10 項目〕・いずれも M2 ゲート）が全 43 項目を分担所有することが確定した。ここでの「M1 不要」は emo2 適合に要らないという意味であり、語彙が未所有という意味ではない。
- 独自論点: `\n[percent]` の割合解釈・`\s[]` 不透明扱い・`%username` 展開は要実装。budoux/縦書きは**痕跡なし・M1 不要**。

## 4. バルーン（balloon-loader の M1 実需）

- emo2 同梱バルーン `emo2-kakukaku`（`type,balloon`）。`descript.txt`（共通既定）→ `balloons0s.txt`/`balloonk0s.txt`（サーフェス別差分）の**順マージ**。
- **M1 必須フィールド**: バルーン本体画像 `balloons0.png`(400×224)/`balloonk0.png`(288×203)＋`use_self_alpha,1` / `windowposition.x/y`（sakura/kero 別）/ `validrect.t/b/l/r` / `origin.x/y` / `wordwrappoint.x`（**負値=右端基準**）/ `font.name`(Yu Gothic UI)/`font.height`(28)/`font.color` / `anchor.font.color` / スクロール矢印 `arrow0/1.x/y`＋画像。
- **M1 省略可**: cursor.*（選択肢ハイライト→矩形反転で代替可）・marker.png・number.*（行番号）・onlinemarker/online0-3・sstpmarker/sstpmessage・communicatebox/balloonc1-4（通信UI）。
- **落とし穴**: 座標の**負値=反対端基準**（`number.xr`/`wordwrappoint.x,-34`/`validrect.bottom,-56`）。s0s/k0s は descript をベースに差分上書き＝マージ実装必須。
- **sakura/kero の左右配置は shell descript の `*.balloon.alignment` が決める** → balloon-loader は shell descript も参照（バルーン単体では決まらない・cross-cutting seam）。

## 5. パッケージローダ／配置規約（package-loader の M1 実需）

- エントリ = `install.txt` の `type`。`type,ghost` → `ghost/master`（脳: descript の `shiori` で SHIORI 起動）＋`shell/master`（立ち絵）＋`balloon.directory` 指定バルーンを**三点同時マウント**。
- バルーン解決2段: ①ルート `balloon.directory`（同梱 `emo2-kakukaku/`）優先 ②無ければ `<root>/balloon/<name>/` フォールバック。
- `delete.txt` は**更新時**の旧パス削除指示。M1 マウントでは無視可。
- **NAR インストーラは M1 範囲外**（展開済みディレクトリを食わせる）。

## 6. 旧ロードマップ spec への影響（rescope サマリ）

| 旧 spec | emo2 実需による rescope |
|---|---|
| areka-P0-seriko-runtime | 「ukadoc **完全マップ**」→ **SERIKO/2.0＋MAYUNA bind・overlay のみ・interval 3種・矩形 collision** へ大幅縮小 |
| areka-P0-shiori-host-32 | **SAORI 同居を M1 から削除**（emo2 未使用）。32bit SHIORI 往復に集中 |
| areka-P0-sakura-script | 全タグ網羅 → **約12タグ＋`\![move]`** に縮小 |
| wintf-P0-surface-hierarchy | 汎用シーングラフ（循環検出/多重同一性）は **emo2 には YAGNI**。overlay z-order＋0,0 合成で足りる。汎用エンジンは M2（ベクトル）へ後ろ倒し。**縦書きはこの後ろ倒しから外れた**——バルーン文字の縦書き（`vertical`／`writing_mode` の受口・縦書き座標意味論・フォント縦書き異体の挙動等価）は **M1・ウェーブ W6.95 の `areka-P0-balloon-vertical-canon` が着地させた**。ただし縦書き全般が M1 になったわけではなく、`\_l` の縦書き座標系・`\f` 系の縦書き写像・プロパティ族は**未実装のまま追跡 spec が所有**する（同 spec が行ったのは語彙登記のみ）。なおこれは適合スコープの判断とは別の話で、§3 の「budoux/縦書きは痕跡なし・M1 不要」（＝emo2 適合 14 項目に不要）は**今も真**である |
| wintf-P0-animation-system | SERIKO 再生は**まばたき（random/bind+random）のみ** |
| areka-P0-balloon-loader | 必須フィールドのみ（§4）。通信/SSTP/online 系は defer |
| compat-ghost-integration | 適合対象を里々→**emo2（pasta 32bit）**。E2E 一周（boot→talk→touch→menu→close） |

## 7. 生態系拡張（emo2 適合の「後」・M1 後半 or 直後）

emo2 が動く＝同じ host-32 機構で他 32bit ゴーストも動く土台。ただし他ゴースト網羅には: **Shift_JIS charset**・**SAORI 同居**・里々/YAYA 固有応答・SERIKO 追加 interval/method・collisionex・NAR インストール 等が順次必要。これらは emo2 マイルストーン達成後の互換面拡大として扱う。
