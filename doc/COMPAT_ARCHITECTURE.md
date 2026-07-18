# areka 互換アーキテクチャ設計台帳

| 項目 | 内容 |
|------|------|
| **Date** | 2026-06-26 |
| **Status** | 合意済み設計方針（ディスカッション確定分） |
| **位置づけ** | areka を ukadoc 準拠の互換ベースウェア（SSP代替）として確立するための上位設計 |

> 本書はディスカッションで確定した戦略・アーキテクチャ判断の記録（議事の正本）。
> 各トラックの spec 化はここを典拠に行う。実装詳細ではなく「判断と理由」を残す。

---

## 1. 位置づけ：互換ベースウェア先行

areka は「ぱすたさん専用の試作」から、**ukadoc準拠の互換ベースウェア（SSP代替）**へ狙いを定め直す。

- **二枚看板**: ①互換ベースウェア（既存伺か資産を動かす）／②ぱすたさん（native旗艦ゴースト）
- **順序**: **①互換ベースウェアを先行**。理由＝(a)既存ゴーストが実際に動く達成感がモチベーションを最大化、(b)最難関の互換部を前倒しで潰しリスクを早期に溶かす。
- ぱすたさんは「互換土台の上で動くnative旗艦」として②で実装する。

---

## 2. 互換契約（コンフォーマンス）

- **典拠は ukadoc。ukadoc が正典（source of truth）。** SSP実挙動の模倣ではなく、ukadoc記載の仕様に準拠する。
- **SERIKO/MAYUNA = ukadoc記載の完全マップ**。
- **さくらスクリプト = ukadoc記載タグを優先度順に漸進**（会話必須タグから。`\![...]` 系の青天井は段階対応）。
- **沈黙ルール**: ukadocが沈黙/曖昧な箇所は、SSP実挙動を**二次参照**しareka裁量で決定し、判断を**対応表に明記**。ukadoc更新時は正典に従い是正。
- 効能: 適合性の判定基準が「読める文書の記述」になり、互換の進捗が**可視・反証可能**になる。

---

## 3. レイヤーモデル

```
フォーマット層   surfaces.txt(SERIKO/MAYUNA)   さくらスクリプト        pasta(native脳)
                       │                          │                     │
解釈層           SERIKOランタイム         さくらスクリプトrunner   （pastaが script/cue を吐く）
   ＝「何を」          └──────────┬───────────────┘                     │
                                  │ 再生制御を要求                       │
タイミング層 ── dola（タイマ・補間・再生制御・スケジュール／"いつ"に特化した下位層）
                                  │ 駆動
描画/窓層 ───── wintf（GPU合成透過(WUC)・サーフェス合成・当たり判定・balloon・hit-test/pointer）
```

- **dola は "いつ" だけを司る純粋なタイミング基盤**（下位層）。SERIKO/さくらスクリプトランナーは dola を基盤に実装される上位層。
- SERIKOランタイムはオーケストレータ：pattern発火タイミング→dola、サーフェス合成→wintf、talk/mouse/bindトリガ→wintfイベント、へ差配する。

---

## 4. 階層サーフェス／アニメーションエンジン（T1）

**SERIKOを"平坦サブセット"として内包する上位エンジン**を native-first で建てる。

- **最大の拡張**: サーフェス定義内のエレメントが、画像ファイルだけでなく**別のサーフェス定義をレイヤーとして参照**できる。結果として**アニメーションが階層構造（シーングラフ）**になる。
- **既存資産への対応**:
  - 入れ子の合成ツリー → wintf **visual-tree**（visual-tree-implementation/synchronization/clip）＋ ECS親子伝播（GlobalArrangement）
  - 入れ子の時間軸 → dola **nested-storyboard**
  - ノード毎のtick → **DolaAnimator**
  - 「サーフェス定義＝ECSサブツリー（VisualGraphics＋子）」「エレメント＝子エンティティ」「別サーフェス参照＝サブツリー埋め込み」
- **MAYUNA(bind/着せ替え)を一般化して内包**：平坦なbindグループは、この階層ツリーの特殊形として落ちる。
- **典拠の二層化**: ukadoc正典が効くのは**SERIKOサブセット**まで。**階層参照拡張は areka-native**（ukadocに正解なし→areka自身が仕様と検証基準を定義する別spec）。混ぜないこと。
- **要設計**: (a) サーフェス参照の**循環検出**（A→B→A）、(b) 同一サブサーフェスの**多重インスタンス同一性**。
- **順序**: 階層エンジンを先に建て、**SERIKOローダはその平坦サブセットへ落とすフロントエンドとして後付け**。逆順は骨が歪む。
- 土台 spec: `wintf-P0-animation-system`（dola→wintf バインディング層）。スコープに「SERIKOランタイムが要求する再生制御プリミティブ＋階層合成」を含める。

---

## 5. SHIORI ホスティング（T3）

**最短ゴールに包含**（既存SHIORIが動かないと面白くない）。

### 内部唯一ABI = `IShiori`（COM）
- areka本体は常に **`IShiori`（COM, 文字列はHSTRING/UTF-16）**だけを握る。呼び出し側に「ネイティブ/過去互換」の分岐を出さず、**生成（アクティベーション）経路だけ**が異なる。

### ネイティブ経路（x64／CPUネイティブ・x86除外）
- 脳が `IShiori` を直接実装。**in-proc COM 直結**（マーシャリングゼロ・最速）。望めばout-of-proc COMで分離も可。
- **push対応**: load時に areka実装の `IShioriHost`(sink) を渡し、脳が `host->Raise(script)` で能動wakeup。
- pasta（native旗艦の脳）はこの経路で `IShiori` を実装。

### 過去互換経路（フォールバック）
- **32bit Rustホスト**（`areka-shiori-host`, i686ターゲットの随伴バイナリ）。
- 本物の `shiori.dll` を `LoadLibraryW`＋`GetProcAddress` で**実行時ロード**（`extern "C"` cdecl）。`load`/`unload`/`request`。
- **64bit areka側で早期に HSTRING(UTF-16) → Charsetヘッダ解析 → charset符号化バイト列**へ変換し、**自前IPC**（名前付きパイプ/共有メモリ）でバイト列を運ぶ。※HGLOBALはプロセスローカルなので跨げない。
- 32bitホストが受信バイト列を `GlobalAlloc` で HGLOBAL 化して旧SHIORIへ渡す。SHIORI規約の所有権（要求HGLOBALはDLLが解放／応答HGLOBALはホストが解放）はホスト内に閉じる。
- **SAORIサブDLLは同32bitプロセスに同居**（実物を飼うことで互換をタダで得る。里々/YAYAをソースビルドしないこと——bitness連鎖で詰む）。
- **自前メッセージループ**を持ち、窓を作るSHIORIも満たす。COMアパートメント問題を回避。
- wakeup は**毎秒 `OnSecondChange` ポーリング**（ukadoc標準。旧DLLはpull専用）。

### 文字列とWinRTの切り分け（重要・誤解しないこと）
- **プロセス内のHSTRING取り回し（生成・読み・解放）はWinRTランタイム非依存**。`RoInitialize`不要。windows-rsの`HSTRING`は純Rust実装でOSのHSTRINGとレイアウト互換。**32bitホストでもWinRTなしでHSTRINGを扱える**。
- WinRTが要るのは「HSTRING型引数を古典COMのOOP越しに**自動マーシャリング**する時」だけ。**本設計はそれを発生させない**（ネイティブ=in-proc／レガシー=自前IPC＋早期byte化）。

---

## 6. この上に乗る既存の解決済み資産

- GPU 合成透過（WUC）・クリック透過（別プロセスへ）・当たり判定（hit-test/alpha-mask）→ 伺か土台の最難関は完了済み（`completed/wintf-dcomp-migration-*`, `wintf-P0-click-through`, `event-hit-test-alpha-mask`）。※透過の初期実装は ULW だったが、DComp→WUC 移行と ULW 撤去（`wintf-ulw-removal`）を経て、現在は GPU 合成単独経路。クリック透過は `WS_EX_LAYERED` 同伴フラグ＋α マスク当たり判定で成立する。
- balloon描画・typewriter・WIC画像・event/drag/pointer・dola runtime・cue/pasta-cue 連携。
- 非同期基盤（`async-executor`/`bevy_tasks`）・world scheduling（vsync/frame）→ OnSecondChangeクロックとrequest非同期圧送（UIを止めない）の土台。

### 合成バックエンド判断: DirectComposition → Windows.UI.Composition（WUC）移行（2026-07-02・spec `wintf-dcomp-to-wuc-migration`）

- **判断**: wintf の表示合成バックエンドを **DirectComposition（DComp）から WinRT の Windows.UI.Composition（WUC）へ純粋等価移行**し、DComp 依存を廃する。device（`Compositor`＋`CompositionGraphicsDevice`）／target（`DesktopWindowTarget`）／visual 木（`ContainerVisual`/`SpriteVisual`）／surface（`CompositionDrawingSurface`＋`CompositionSurfaceBrush`）／frame-apply（明示 `Commit()` 廃止→DispatcherQueue 暗黙反映）を WUC 相当へ写像。描画結果・再描画・入力は不変。
- **理由**: 合成基盤を WUC 系へ寄せ、将来の合成機能（本 spec では非活用）への地ならしと DComp 依存の廃止。
- **スレッド前提（実測確定）**: 本番 UI スレッドは MTA（`WinApp::new` の `CoInitializeEx(COINIT_MULTITHREADED)`）。WUC は MTA 上で動作し、DispatcherQueue は **`DQTAT_COM_NONE`**（apartment 不変）で生成する（既存 message pump に相乗り・pump 非差し替え）。設計初稿の「STA 前提／ASTA」は R1 スパイクの実測で否定。
- **ULW 除去済み（後日 spec `wintf-ulw-removal` にて実施）**: 本 WUC 移行時点で残置していた ULW アーム・`CompositionMode` enum・ULW 前提の描画分岐は `wintf-ulw-removal` で除去され、GPU 合成（WUC）単独経路へ collapse した。`compute_ex_style` は合成モード引数・分岐を持たない branchless 単一経路（`WS_EX_NOREDIRECTIONBITMAP` 付与）となり、`WS_EX_LAYERED` は生成時に付与しない。クリック透過（当たり判定層・`apply_layered_companion` による `WS_EX_LAYERED` 実行時同伴フラグ）は不変で存続。
- **既知の制約**: WUC の per-corner 角丸 clip（`RoundedRectangleIndividual`）は `CompositionPath`＝`IGeometrySource2D`（Win2D）を要し windows 0.62.2 単体では厳密構築不可。areka 本体は個別半径未使用のため均一半径近似で写す（要検討: Win2D 依存の是非）。

---

## 7. 未決・要設計（spec化時に詰める）

- 階層サーフェス: 循環検出ポリシー、多重インスタンス同一性、SERIKO平坦サブセットへの写像規則。
- さくらスクリプト: 優先タグの確定リストと対応表のフォーマット（ukadoc条項→挙動→検証）。
- SHIORI: 32bitホストのIPCプロトコル（フレーミング/エラー/タイムアウト/プロセス監視）、`IShiori`/`IShioriHost` のメソッド面、Charset交渉の具体。
- 沈黙ルールの「対応表」の置き場所と運用。→ §8 に登記を開始（各 spec が正典沈黙箇所の areka 裁量を追記する）。

---

## 8. 沈黙ルール対応表（正典沈黙箇所の areka 裁量記録）

ukadoc が沈黙/曖昧な箇所を areka 裁量で決定した記録（§2 沈黙ルールの運用実体）。各 spec が実装着地時に自らの裁量を追記する。ukadoc 更新時は正典に従い是正する。

| 項目 | 裁量 | 根拠 | 出典 spec |
|---|---|---|---|
| `%username` 既定値（スナップショット未解決時） | `ユーザーさん` | 正典沈黙・伺かの伝統的な未指定時デフォルト・決定論定数（唯一の定義点＝`areka_sakura::sysvar::DEFAULT_USERNAME`） | areka-P0-sakura-dialogue-tags（開発者裁定 2026-07-18・設計ディスカッション#2） |
| compile 側時間指令 allowlist（`quicksection`／`set,balloonwait`／`set,choicetimeout`／`set,balloontimeout`／`embed`／`sound,wait`／`wait,syncobject`／同期 `move` 系の持続時間引数 等） | M1 は**非実導出**（語彙保持＋縮退のみ・`\!` 全体は汎用キャリア cue へ転写し compile は allowlist の意味を追加解釈しない） | 正典が compile 干渉する時間指令を明示列挙せず・emo2 未使用ゆえ源が着地するまで実導出しない（R4.3 但書）・実導出は追跡 spec `areka-P0-sakura-time-directives` へ申し送り | areka-P0-sakura-dialogue-tags |
| `\![move]` の裸 `base`（ドット無しの基準位置トークン） | `base.base` と等価に解する（`parse_move_directive`：ドット無しトークンは `X基準=Y基準=token` として展開） | 正典形式は `X基準.Y基準` の 2 軸・fixture `move,-353,,,0,base,base` の de-facto（R5.2 明文） | areka-P0-sakura-dialogue-tags |
| `\![move]` の名前付き `--key=value` 形（ukadoc 記述例の形式） | M1 縮退＝`Err(MoveDegradation::NamedForm)`（記録付き良性スキップ・語彙は将来 additive・positional のみ実導出） | emo2 は positional 形のみ使用・positional が canon 正の実導出経路 | areka-P0-sakura-dialogue-tags |
| `\![move]` の基準 `screen`／`primaryscreen`／`me`／`global` | M1 縮退（`Ok` で語彙保持＋`MoveDirective::m1_degradations` が `UnsupportedBase` を記録・数値スコープ基準のみ実導出） | emo2 未使用（fixture は数値スコープ `0`）・非スコープ基準の解決は源が着地するまで実導出しない | areka-P0-sakura-dialogue-tags |
| `\![move]` の時間指定 `time>0`（アニメーション付き移動） | 最終位置へ即時反映＋縮退記録（`Ok` で `duration_ms` 保持・`m1_degradations` が `TimedMoveImmediate` を記録） | R5.4 明文・fixture は time 空=0・補間は M1 外 | areka-P0-sakura-dialogue-tags |
