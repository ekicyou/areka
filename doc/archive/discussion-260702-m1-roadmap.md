# ディスカッション記録 2026-07-02 — M1 ロードマップ現況・エンジン固有名確定

> ⚠️ **これは 2026-07-02 時点の状況スナップショット**（陳腐化し得る）。
> **正本は [.kiro/steering/roadmap.md](../.kiro/steering/roadmap.md)**（M1 専用）／設計判断は [doc/COMPAT_ARCHITECTURE.md](COMPAT_ARCHITECTURE.md)（旧 framing 残存・追従保留）／実物スコープは [doc/emo2-conformance-scope.md](emo2-conformance-scope.md)。
> 食い違ったら**この文書でなく正本を見ること**。
>
> 添付図: [roadmap-tree.svg](discussion-260702-roadmap-tree.svg) ／ [concurrency-model.svg](discussion-260702-concurrency-model.svg)
> 前版: 2026-06-28 版（M1 再設計ディスカッション記録）を改題・全面更新（旧版は git 履歴に保全）。

## 0. 2026-06-28 → 07-02 に起きたこと（差分要約）

06-28 の大整理（M1 専用化・未着手仕様の全伐採・7トラック・実現可能粒度＝当日のコミット列は git 履歴参照）から4日で、**耐力壁の突破と足場の完成**まで進んだ。

- **06-30**: i686 ビルド実証（rustup target＋PowerShell 必須）・x64↔32bit 通信は **WM_COPYDATA 一本化**方針・クリック透過第4の手（`WS_EX_TRANSPARENT` 動的トグル）の先進坑掘削開始。
- **07-01**: **両先進坑 go** — `pilot-shiori-host-32`（x64→32bit `pasta.dll` 1往復＋窓持ちループ生存＝**M1 の生死を分けた耐力壁を突破**）／`pilot-clickthrough-alpha-toggle`（DComp 描画を捨てず別プロセスクリック透過成立）。WUC 移行調査 GO-with-caveats（pilot 不要判定）。
- **07-02**: **② parsers トラック全完了**（foundation／shell-parse／balloon-parse／package-mount。sakura-parse は既完）・**① host32-ipc 完了**（実 i686 helper 越し往復 echo green）・**wintf の WUC 移行完了**・**エンジン固有名確定**（本記録 §1）・**root 構築起点の訂正**（§6）。

## 1. エンジン固有名（2026-07-02 確定）

7トラック（⓪〜⑥）の各エンジンに識別用の固有名を割当。**コード／spec／会話の参照はこの名で統一**する。

| # | 固有名 | エンジン（説明名） | 由来 |
|---|---|---|---|
| ⓪ | **`ghost`** | ゴーストエンジン（最上位 owner・統括） | 伺か正準語 |
| ① | **`shiori`** | SHIORI 通信層エンジン host-32 | SHIORI 通信（crate `shiori-host32-*`／`IShiori` と一貫） |
| ② | **`parsers`** | parser / loader | crate `areka-parsers` と一致（複数形が正） |
| ③ | **`kanade`** | conductor（SHIORI イベント循環） | **奏でる**＝演者（sakura／seriko）を統べる。同名ゴーストの縁も |
| ④ | **`sakura`** | sakura-engine（さくらスクリプト再生） | さくらスクリプト |
| ⑤ | **`seriko`** | shell-anim-engine（SERIKO アニメ） | SERIKO ランタイム |
| ⑥ | **`emo`** | render-engine（シェル/バルーン統一 surface 合成） | **emotion**＝感情を描く可視層 |

- 命名 register: 「役割を体現する人名の手触り」（kanade／sakura／emo）と「伺か正準語」（ghost／shiori／seriko／parsers）の混成。
- **注意**: `emo`（エンジン）と `emo2`（適合対象ゴースト実体）は別概念。会話で混同しない。
- **未着手ユニット名も固有名基準へ改名済**（roadmap 正本 2026-07-02）: `conductor`→`kanade`／`shell-anim-engine`→`seriko-engine`／`shell-anim-loop`→`seriko-loop`／`surface-engine`→`emo-surface`／`text-layer`→`emo-text-layer`。**不変**: `host32-*`（shiori トラック実装進行中の改名衝突回避・名も既に整合）・`sakura-engine`・`ghost-*`・completed 仕様（歴史）。
- 既知ドリフト: `crates/areka-parsers/src/balloon/model.rs:6` の doc コメントが旧名 `text-layer`/`surface-engine` を参照（emo ユニット着手時に追随・roadmap に記録済み）。

## 2. M1 ゴール

areka（**x64**）が最小 SSP 互換ベースウェアとして、**emo2**（作者自作・脳=`pasta.dll`・**32bit SHIORI**）を「そのまま」起動→会話→撫で→メニュー→終了まで E2E 実走させる。
emo2 が動く＝同じ汎用 32bit ブリッジで里々/YAYA も動く土台（互換＝普及の入口）。M1 スコープは emo2 が実際に使う機能で実物定義（完全網羅・予測実装はしない）。

> **07-01 追記**: このゴールの生死を分ける唯一の耐力壁「x64 areka が emo2 の 32bit `pasta.dll` を駆動できるか」は先進坑で**実走突破済み**（go 判定＝開発者）。以降は既知技術の組立フェーズ。

## 3. 実装規律（balloon-system の失敗から・不変）

- **実装ファースト**: 成果物は「emo2 が実際に動く」検証済みコード。spec でも先回りの抽象でもない。
- **spec 工場の禁止**: 成果物が子 spec になる構造を作らない。1ユニット＝1かたまりの動く振る舞い。
- **最小実装＋薄い拡張シーム**: emo2 が使う分だけ実装、拡張は型/レジストリの口だけ。
- **動く資産から建てる**: `areka-mock-shell`（窓＋Typewriter）・`areka-P0-shiori-reference`（native 脳デモ）・dola・**完成した parsers／host32-ipc** から増分。
- **brief は前もって量産しない**: 着手時に1本ずつ just-in-time（`/kiro-discovery` 再入 or `/kiro-spec-init`）。`/kiro-spec-batch` は使わない。
- **二坑モデルの実証**: 不確実な耐力壁は pilot（使い捨て・go 基準明示）で先に潰す→ 07-01 に2本連続で go を出し規律の有効性を実証。本坑は pilot 知見を**クリーンに掘り直す**（コピペ donor 禁止）。

## 4. 基盤データ構造（M1 着手時から組み込む・不変）

- **シェル/バルーン統一**: 描画エンジン（emo）上で区別しない。バルーン＝シェル surface 上の文字層。バルーン枠も surface＝アニメ可。
- **element に他サーフェス参照可**（入れ子）→ surface 合成は再帰的。
- **element 配置＝D2D 変換行列**（x,y は単位行列の特例・回転/拡縮そのまま）。emo2 は単位平行移動＋平面 overlay のみ使うが、構造は最初から持つ。
- 旧「汎用シーングラフは M2」の**部分的前倒し**（データ構造のみ M1・上位演出エンジンは M2）。

## 5. アニメーションエンジンは2つ（フォーク）

```
kanade（③ conductor・SHIORI イベント循環）
  └ sakura（④ さくらスクリプト再生・talk timeline）
       ├─(shell anim: \s 等)→ seriko（⑤ SERIKO ループ）→ surface 合成 ┐
       └─(text: typewriter)──────────────────→ emo text-layer ─────────┤→ emo（⑥ render）合成→画面
```

テキスト描画はシェルアニメではないので、sakura は text を emo（text-layer）へ**直接**指令（seriko を経由しない）。両アニメエンジンは **dola（完了・タイミング層）**上。
※「アニメ2エンジン」の旧番号（①=sakura・②=seriko）は7トラック番号（④⑤）と紛らわしいため、以後は**固有名で参照**する。

## 6. 構築（初期化）モデル — 各エンジンのコンストラクタ

> **⚠️ 06-28 版からの訂正**: root 構築起点は **`ghost/master/descript.txt`** である。旧版の「root＝`install.txt`」は誤り（`install.txt` は NAR インストーラの配置マニフェスト＝**起動時不使用**・ukadoc 論拠）。この訂正を受けて `areka-P0-package-mount` は descript.txt 駆動へ書き直され、07-02 に完了した。

- **root（親）**: `ghost/master/descript.txt`（ゴースト全体の親コンストラクタ。`shiori`→DLL・`seriko.defaultsurfacedirectoryname`→shell dir）。
- **ghost**: `package-mount`（完了）が起点定義を解決して SHIORI／shell の2点マウントモデルを構築。実行時 owner＝ghost エンジン。
- **shiori（host-32）**: コンストラクタ＝ゴーストフォルダ定義（descript.txt の `shiori,pasta.dll`＋dir）。
- **seriko**: コンストラクタ＝SERIKO/shell 定義（`surfaces.txt`）＋ balloon 定義（統一エンジンゆえ両方が構築入力）。
- **sakura**: コンストラクタ＝さくらスクリプト（**runtime・per-talk・transient**）。
- 2系: **load-time**（root→ghost→{shiori, seriko}・一度）と **runtime**（script→sakura・都度）。
- balloon の所在解決は baseware 共有・ユーザ選択ゆえ root 起点のスコープ外。
- 読込エンコーディング: charset 宣言（冒頭 ASCII プリスキャン）依存・宣言無ければレガシー＝ANSI。既定はハードコードせず呼び出し側指定（SSP 既定=ANSI）。parsers の `charset::decode` が実装済み。

## 7. 全7トラックと進捗（→ [roadmap-tree.svg](discussion-260702-roadmap-tree.svg)）

**M-boot（emo2 が起動して喋る・16ユニット）の完了 約 7/16**。②parsers は全完了、①shiori は pilot✅/ipc✅/shiori-load✅（request・lifecycle 残）。
> ⚠️ 07-03 追記: 本記録作成中に `areka-P0-host32-shiori-load`（①）と `wintf-clickthrough-alpha-toggle`（M1外）が並行で完了・main へ着地。以下 §7/§10/§11 はその完了を反映済み。

| # | 固有名 | 役割 | M-boot 状況 | 増分（M-boot 後・並走可） |
|---|---|---|---|---|
| ⓪ | ghost | 最上位 owner（lifecycle/窓配置/位置永続化・統括） | ghost-setup／window-placement 未着手 | position-persist |
| ① | shiori | 32bit pasta.dll 駆動・耐力壁 | pilot ✅go・ipc ✅・shiori-load ✅・**request＝次フロント**・lifecycle | なし（M-boot で完了） |
| ② | parsers | 定義→model・構築入力 | **全完了 ✅**（foundation/sakura-parse/shell-parse/balloon-parse/package-mount） | なし（M-boot で完了） |
| ③ | kanade | SHIORI イベント循環 | 未着手 | idle-talk / input-events |
| ④ | sakura | さくらスクリプト再生 | 未着手 | sakura-dialogue-tags |
| ⑤ | seriko | SERIKO ループ＋surface 状態＋MAYUNA | 未着手 | dual-surface / mayuna-compose / seriko-loop |
| ⑥ | emo | surface＋text 合成＋**総合マウス制御** | emo-surface／emo-text-layer 未着手 | collision-geometry / choice-render / dual-window |

- **① shiori の完了内訳**: `host32-ipc`＝bytes-over-wire transport を3クレート（`shiori-host32-ipc`=proto／`-host`=x64+arm64／`-helper`=i686）で本坑再掘。**WM_COPYDATA 一本化＋再入 RESPONSE**（named pipe 不要）。実 i686 helper 越し往復 echo が無デッドロック・無クラッシュ。
- **② parsers の完了内訳**: `foundation`（charset デコード＋KV 素朴マップ・encoding_rs 承認済）→ 3兄弟が依存。`shell`（SERIKO/2.0 subset・四層 model←lexer←decode←parse・寛容パース）／`balloon`（幾何＋フォント・descript＋画像別の後勝ち2層マージ）／`package`（descript.txt 起点2点マウント・`MountError` 観測可能）。**parse は忠実な転記層**（範囲非展開・記述子保持）＝ツリー展開は下流のエンジン構築側。
- 粒度基準（不変）＝「走らせて観測できる単一 pass/fail を持ち、観測に別ユニットを要さない」。マイルストーン M-dual/M-mayuna/M-life/M-dialogue/M-e2e は**エンジン横断の統合点**であって作業単位ではない。

## 8. 並走とクロスエンジン I/O

- **並走安全（M-boot 済なら独立着手可）**: kanade:idle-talk ／ seriko:mayuna-compose・seriko-loop ／ ghost:position-persist。
- **クロスエンジン結合（I/O 契約を先決→両側並列）**:
  - 撫で（M-life）: emo:collision-geometry（入力 mouse→region/actor）⟷ kanade:input-events（→SHIORI 出力）。契約＝**region/actor**。
  - 選択肢（M-dialogue）: sakura:dialogue-tags ⟷ emo:choice-render ⟷ kanade:input-events。契約＝**\q/選択**。
  - 二人立ち（M-dual）: seriko:dual-surface ⟷ emo:dual-window ⟷ ghost:window-placement。契約＝**2窓/surface**。
  - 移動（\![move]）: sakura:dialogue-tags ⟷ ghost:window-placement（キャラ移動＝窓移動）。
- **マウス**: 総合マウス制御＝emo の責務。低レベル（窓 msg/alpha hit-test/drag）は完了基盤（`event-mouse-basic`/`event-drag-system`/`event-hit-test`/`event-hit-test-alpha-mask`）。M1 新規は collision-geometry のみ。
- **owner-draw 右クリックメニューは M2**（M1 メニュー＝balloon \q 選択肢）。

## 9. 並行モデル（→ [concurrency-model.svg](discussion-260702-concurrency-model.svg)）

- 各エンジン層＝**チャンネル通信のアクターモデル**、**エンジンインスタンスごとに独立スレッド**（async 中心でなくスレッド独立＝ゴーストの連続ループ/message pump に馴染む）。
- **エンジン間 I/O 契約＝channel メッセージ型**（§8 の契約がそのまま channel）。
- **但し ⑥emo/window は UI スレッド固定**（D2D 単一スレッド＋window アフィニティ）＝他 actor は worker から描画/入力を channel で emo へ。
- **07-02 深掘り（WUC 移行完了の実測知見）**: 合成バックエンドは DComp→**WUC**（`Compositor`＋`DesktopWindowTarget`）へ純粋等価移行済み。WUC は areka の **MTA UI スレッド**（`COINIT_MULTITHREADED`）上で `DQTAT_COM_NONE`（apartment 不変）の DispatcherQueue で動く＝「STA 前提」は誤りと実測で確定。WUC を触る graphics schedule は UI スレッド固定（DispatcherQueue 親和性）——**アクターモデルの「emo＝UI スレッド固定」制約と完全整合**。
- ①shiori は**別プロセス**＝天然のアクター境界。IPC は **WM_COPYDATA**（窓持ち SHIORI の message pump へ自然配送）。**x64⟷x86 を跨ぐのは生バイト列のみ**（HGLOBAL は 32bit ローカル／HSTRING は x64 ローカル＝各プロセス自前通貨・どちらも跨がない）。
- ④sakura は per-talk transient（talk ごとに生成・破棄）。②parsers は load-time に一度走るだけで actor ではない。

## 10. wintf 基盤層（M1 外・並走軸）

M1（emo2-boot）とは別軸で進む wintf 基盤の改善。M1 ユニットとはファイル群が別＝**並走非衝突**。

| spec | 状態 | 内容 |
|---|---|---|
| `wintf-dcomp-to-wuc-migration` | **✅ 完了（07-02）** | 表示合成 DComp→WUC 純粋等価移行。ULW アーム・`CompositionMode` は不触 |
| `pilot-clickthrough-alpha-toggle` | **✅ go（07-01）** | `WS_EX_TRANSPARENT` 動的トグルで **DComp 描画を捨てず**別プロセスクリック透過成立。必須配合＝`WS_EX_LAYERED` 同伴フラグ＋枠なし `WS_POPUP`。表示層と当たり判定層は独立 |
| `wintf-clickthrough-alpha-toggle` | **✅ 完了（07-02）** | 本体αマスク（`AlphaMask::is_hit`）参照でキャラ領域のみクリック可を wintf 本体へ実装。当面 ULW と並走 |
| `wintf-ulw-removal` | **着手可**（clickthrough 完了でゲート解除） | ULW 一式除去＋`CompositionMode` collapse（GPU 合成単独へ） |

## 11. 次の一手（2026-07-02 時点のフロント）

即並走可能なフロント（別ワークツリー・1 feature = 1 branch = 1 PR）:

1. **shiori: `areka-P0-host32-request`**（次フロント・SHIORI/3.0 build＋marshal＋response Value＋charset）→ 以降 lifecycle と逐次。
2. **emo: `areka-P0-emo-surface`**（render 基盤・mock-shell＋dola から増分）／**ghost: `areka-P0-window-placement`**（mock-shell 実コードから）— 同一素材起点ゆえ軽い競合注意・各1本ずつ。
3. **wintf: `wintf-ulw-removal`**（clickthrough 完了でゲート解除・ULW 除去＋`CompositionMode` collapse）。
4. 結合クラスタの **channel 契約の先決**（region/actor・\q/選択・2窓/surface）は該当ユニット着手時に。

brief は着手時に just-in-time で1本ずつ（§3 の規律）。

## 12. 関連記憶（user auto-memory）

areka-engine-names（**new**・固有名の正本ポインタ） / areka-roadmap-drift / areka-compat-baseware-strategy /
areka-unified-shell-balloon-graphics / areka-two-animation-engines / areka-engine-construction-model /
areka-concurrency-model / areka-ghost-boot-descript-not-install（root 訂正） / areka-wuc-runs-on-mta-thread（WUC=MTA 実測） /
areka-host32-ipc-and-i686-build / areka-shiori-layer-naming-and-memory / areka-parser-foundation-carveout /
areka-parser-transcribes-tree-downstream / ukadoc-mcp-preferred-source（正典は ukadoc・emo2 は最小 fixture）。
