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
| `\![move]` の基準位置 `base`（basepos）解決 | 正典既定 basepos のみ実導出＝x=サーフェス幅÷2・y=下端（`height`）（`CanonDefaultBasepos`：`BaseposResolver` 型シームの M1 実装・座標は `WindowPos.size` 物理 px のみを源とし論理 px 系を経由しない） | R5.2 明文・emo2 は `point.basepos` を宣言せず正典既定がそのまま適用される正規経路（fixture は Y=fix ゆえ実効は basepos.x のみ）・R-6 二重スケール欠陥の構造遮断 | areka-P0-sakura-dialogue-tags |
| 宣言 `point.basepos` の実導出（サーフェス個別 basepos） | 本 spec の範囲外＝**型シーム予約**（`BaseposResolver` トレイトの別実装差替点・M1 は `CanonDefaultBasepos` 固定）・実導出は追跡 spec へ申し送り | emo2 未宣言ゆえ源が着地するまで実導出しない・差替可能な型シームで縮退を第一級保持 | areka-P0-sakura-dialogue-tags → 追跡 `areka-P0-surfaces-basepos` |
| SET 無効な正準語彙（`SET_EFFECTIVE` 非登録の点付き語彙）への書込挙動 | 受理（`Ok`）＋警告ログ（`NotSettable`）＋非反映（鏡像・永続いずれも書き換えない） | 正典（ukadoc `list_propertysystem.html`）は SET 無効項目への書込失敗挙動を明示しない＝沈黙。呼出側の起動継続を妨げないため受理＋警告＋非反映を areka 裁量とする（R3.4）。SET 有効群は `RuntimeCommand`・正準語彙外の自由 dotted key は `StoreWrite`（host 区画反映）と区別する | areka-P0-sylphya（vocab/dotted.rs `SET_EFFECTIVE`・R3.4） |
| `%selfname2`（descript `sakura.name2`）が未定義のときの値 | 素通し縮退（`selfname2` エントリを積まない＝フラット静的値を publish せず、消費側で `%selfname2` 原文がそのまま素通しになる）。既定値やフォールバックを**創作しない** | 正典（ukadoc）は `sakura.name2` 未定義時の `%selfname2` 展開値を明示しない＝沈黙。sakura.name2 は本体側の任意別名であり、未宣言時に本体名等を代入すると誤った別名を捏造するため、素通し縮退で「値なし」を忠実に表す（R4.4） | areka-P0-sylphya（ghost `sylphya_wiring.rs` `derive_flat_statics`・R4.4） |
| `%keroname`（descript `kero.name`）が未定義のときの値 | 本体側の名前（`sakura.name`）へフォールバック（SSP 互換）。`sakura.name` も未定義なら素通し縮退（`keroname` エントリを積まない） | 正典（ukadoc）は `kero.name` 未定義時の `%keroname` 展開挙動を明示しない＝沈黙。SSP は kero 名未指定時に本体名を流用する de-facto 挙動があり、これに互換する。本体名も無ければ捏造せず素通し（R4.5） | areka-P0-sylphya（ghost `sylphya_wiring.rs` `derive_flat_statics`・R4.5） |
| バルーン系列の正規名 `balloonp0def{ID}` / `balloonp1def{ID}` を scope 0 / 1 の**第一候補として先行探索**すること | areka 裁量の**正規化拡張**として採用。連鎖は 3 段連結＝当該 scope 自身の候補（正規名→旧名）＋相方系列（scope 2 以上のみ `balloonk`）＋デフォルト定義（scope 1 以上のみ `balloonp0def`→`balloons`） | 正典は p 系列を「**三人目以降**」としてのみ記述し（`balloonp*def*` に加え `arrowp2def` 以降・`markerp2def` 以降・`clickwaitp2def` 以降の 3 箇所が独立に明言）、scope 0/1 の正規名を定義していない＝正典沈黙。SSP が無視するファイルを areka が拾う互換乖離の可能性を提示したうえで開発者が採用を裁定。デフォルトの地位は scope 0 のみが持ち、scope 1 はデフォルトではないため n≧2 の連鎖に `balloonp1def` を含めない | areka-P0-kero-balloon（R1.10 / R7.7(a)・`areka-emo-present::balloon::prefix_chain`） |
| 同一 scope を指す**語彙が二系統**ある事実と、内部表現の正準形 | 内部表現は **scope 番号（`u32`）のみ**へ一本化。`Sakura`/`Kero` 等の 2 値列挙も、さくらスクリプト側の別語彙も内部の正準表現として採用しない。接頭辞は scope 番号から表データ経由で導出する | 正典上、同一 scope をさくらスクリプトは `\0`／`\h`・`\1`／`\u` と呼び、バルーンファイル名は `s`(sakura)／`k`(kero) と呼ぶ（ファイル名は `balloons` であって `balloonh` ではない）。どちらを正準とするかは正典沈黙。2 値列挙へ潰すと三人目以降（`balloonp{n}def`）を表現できず作り直しになるため番号を正準とした | areka-P0-kero-balloon（R1.9 / R7.7(b)・`SeriesFamily` 表データ） |
| 装飾族に**接尾辞なしの旧名がもう一段**存在する事実 | **未実装（語彙記録）**。縮退シームの所在＝`SeriesFamily.scope0_legacy` / `scope1_legacy` が**可変長の候補列**であり、`["arrows", "arrow"]` のように一段深い旧名を構造改変なしで表現できる | 正典は「`arrows` が本体用・旧バージョン対応のために `arrow` で代用を推奨」と記し、`markers` に対する `marker` も同型。吹き出し族には接尾辞なしの旧名が存在しないため本 spec では現れないが、他族の scope 別対応（本 spec では Out of scope）が同一機構を再利用できる形を設計上の評価軸とした | areka-P0-kero-balloon（R7.7(c)・檻 `series_family_table_expresses_deeper_legacy_names`） |
| ID 単位フォールバックで後段接頭辞の面を採用したとき、**どの面別上書き層を適用するか** | **採用した画像に対応する同接頭辞の `{採用接頭辞}{ID}s.txt`** を用いる（scope 1 が `balloons1` へ縮退した面には `balloons1s.txt` を適用し、`balloonk1s.txt` を探しに行かない） | **正典整合（解釈）**。正典は面別上書きを「**対応する ID のサーフェス（画像）に対して**」適用すると定めるため、採用画像にその画像の上書き層が対応するのは正典の帰結であり areka 裁量ではない | areka-P0-kero-balloon（R2.3 / R7.4・`ResolvedFace::override_file_name`） |
| `\b[ID]` の ID が指す名前空間 | **当該 scope が解決した系列内の面 ID** と解釈する（scope 1 で `\b[1]` は scope 1 の系列の面 1＝`balloonk1.png` があればそれ、無ければ ID 単位フォールバックで `balloons1.png`） | **正典整合（解釈）**。ID は cue 経路のどこでも接頭辞変換を受けず、接頭辞は World 構築時に消費されるため、`balloon_target(N)` の World は scope N 自身の系列から組まれる。正典は `\b` の ID が系列をまたぐとは記していない | areka-P0-kero-balloon（R5.1 / R7.4） |
| ghost descript の `balloon.defaultsurface` / `kero.balloon.defaultsurface` / `char*.balloon.defaultsurface` による初期表示面宣言 | **非追従（未実装）**＝正典既定値 **0** のみを実装。各 scope の初期表示面は当該 scope が解決した系列の面 ID 0 | 語彙記録＋縮退シーム。emo2 は両キー無宣言ゆえ既定 0 で現状差が出ない。面 ID の偶奇＝左右向きセット意味論および表示位置に応じた左右面の自動切替も本 spec では導入しない | areka-P0-kero-balloon（R2.6 / R7.4） |
| `windowposition.x` の**キーワード指定**（`center` / `top` / `bottom`）と `windowposition.limit` | いずれも**未実装（語彙記録＋縮退シーム）**。数値指定のみを実装する。`limit`（バルーンを強制的に画面内へ維持する 0/1・**正典既定 1**）は現行の非クランプ方針を維持する | 正典は `center`（SSP のみ `top` も同じ）でシェルの中央上へ、`bottom`（SSP のみ）で中央下へ固定すると定め、`y` の基本位置もこの指定により変わる（`center,top`＝バルーン下端とシェル画像上端が接する／`bottom`＝バルーン上端とシェル画像下端が接する／**数値指定＝バルーンとシェル画像の上端が重なる**）。areka は数値指定の基本位置のみを実装した。`limit` 未実装のため、ゴーストが画面端に寄っているとバルーンが画面外へはみ出し得る（実機で観測・追跡対象） | areka-P0-kero-balloon（R7.4・Out of scope 明記） |
| **`windowposition.x` の符号規約**（正典が x 方向の基本位置に沈黙しているため実機確定した項目） | **画面座標系のオフセットとしてそのまま適用**する（正＝画面右／負＝画面左）。**バルーンがキャラのどちら側に置かれるかによる符号変換を行わない**。基本位置は左置き＝キャラ窓の左隣／右置き＝キャラ窓の右隣（上端揃え） | **実機確定（2026-07-31・R7.6）＝参照実装 SSP を受理オラクルとした**。同一ゴースト emo2 ＋バルーン emo2-kakukaku を実 DPI 120（k=1.25）で SSP に表示させ窓矩形を DPI aware で実測した結果、sakura（balloon 左置き・`wp.x=+266`）は基本位置から画面**右**へ +332、kero（balloon 右置き・`wp.x=−190`）は画面**左**へ −237 であり、**左右で反転していない**。⚠ **ukadoc は「数値指定の場合シェル側が+、シェルから離れる側が-」と記しており、参照実装の挙動はこの文言と食い違う**（kero 側）。areka は互換ベースウェアであり **SSP の実挙動を正**とする。当初 areka は文言どおり左右反転を実装しており、実機で kero 側が 475px 乖離していた | areka-P0-kero-balloon（R3.3 実装時訂正 / R7.6 / R7.4・`placement/windowposition.rs`・証跡 `real-run-signoff-2026-07-31.log`） |
| **サーフェス寸変動時のバルーン追従基準**（正典は resize 時にバルーン相対をどう保つかに沈黙） | **キャラ窓の左上（窓相対）で追従し、リサイズで `BalloonFollow.offset` を補正しない**。runtime・保存の双方で同一基準＝全アンカーで `balloon_pos − char_pos ≡ offset` 不変。**キャラ窓位置そのものの原点（下端中央）は別基準として維持**（`char_pos_to_origin_x` 系は無改変）。 | **実機確定（2026-07-31）＝参照実装 SSP を受理オラクルとした**。同一ゴースト emo2＋実 DPI 120（k=1.25）で、SSP のバルーンは**観測時つねに現在表示中の**キャラ窓に対し窓相対 (−168,−161) にある（char 477×683 表示時に balloon offset (−168,−161)）。※単発観測から言えるのはこの不変量までで「SSP が切替時にバルーンを動かした」という時系列断定は不可——ただし是正形はどの仮説でも同一。areka は boot 採寸窓（543×859）基準で置いたきり据え置き、切替後（478×684）にバルーンを 336px 上空へ浮かせていた。是正は先行 `completed/areka-P0-surface-resize-resnap` **Req2.6**（追従 offset を維持＝窓相対契約）の**復元**にあたる。当該補正は `completed/areka-P0-position-persist` が実機サインオフ最終盤に SSP 突合なしで導入したもので（要件・設計に記載なし・commit 9d5c8bd）、Bottom アンカー限定ゆえ内部でセマンティクスが分裂していた。1px 差の扱いは下行「丸め」と同根。**本裁定が否定した先行 AC は position-persist の R2.2**（「バルーン相対オフセットを……キャラのアンカー辺を基準として保存・復元する。**キャラ窓の左上を基準としてはならない**」）**と R8.5**（「寸法変動に対して不変」）——アーカイブ済み spec は非改変とし、上書きの事実を本表と現行 spec に記録する。**永続値の移行策は設けない**: `PersistKey::BalloonOffset` はキー据え置きのまま値の基準がアンカー辺→char 左上へ変わるため、旧ストアを持つ環境では復元位置が char 高さぶん飛ぶ（檻の実測で −730 ⇄ −43）。M1 開発段階の dev fixture のみで、旧値自体が本欠陥に由来するため意図的に移行しない（実機検証前に profile を消す既存手順で足りる）。 | areka-P0-kero-balloon（R3.8 実装時訂正・`placement/{follow.rs,persist.rs}`） |
| `windowposition` 調整量の **k 適用時の丸め**が SSP と 1px 食い違うこと | areka は既存丸め権威 `ScaleRatio::scale_len`（round half away from zero）を**そのまま用い、SSP へ合わせ込まない**。結果 `266×1.25=332.5` を areka は 333、SSP は 332 とし、確定オフセットが scope あたり最大 1px ずれる | 本 spec の R3.6 が「新たな丸め規約を導入しない」と定めるため、SSP 追従のための独自丸めを持ち込まない。1px は視認不能であり、丸め権威を 1 箇所に保つ利益が上回る。実機実測での乖離は sakura (−167 対 SSP −168)・kero (+182 対 SSP +183) の各 1px のみ | areka-P0-kero-balloon（R3.6 / R7.4） |
| バルーン**面 ID 判定の厳格化**（本 spec 適用前後で字義上唯一の非同一点） | 「接頭辞の厳密一致 → `.png` 除去 → 残余の**全数字**」の 3 段判定とし、符号付き表記（`balloons+0.png` 等）を面として採用しない。旧実装は `stem.parse::<u32>()` 任せで先頭 `+` を受理し得た | 正典の面 ID 表記は符号を持たず、実資産に該当名は現れないため後方互換（R5.4）への実害はない。入力ウィンドウ用 `balloonc*` を `balloonk` と誤認する事故は接頭辞完全一致で構造的に不可能 | areka-P0-kero-balloon（R1.5 / R7.4・`face_id_of`） |
| `ScaleRatio::scale_len` の「**非ゼロ長は最小 1px**」規約を `windowposition` 調整量へ継承したこと | 継承する（絶対値 1 の `windowposition` 成分は k 縮小時も 0 へ潰れない） | 当該規約は**長さ**のために作られた権威だが、オフセット専用の「0 へ潰れてよい」例外を設けること自体が R3.6 の禁じる新丸め規約の導入に当たる。残差は 1px 以内かつ符号保存 | areka-P0-kero-balloon（R3.6 / R7.4） |
| `%username` の SHIORI Resource `username` GET が 204 No Content／空値を応答した場合の値 | 既定値へ決定論的に縮退。**縮退の定義点は sakura 側に残置**（唯一の定義点＝`areka_sakura::sysvar::DEFAULT_USERNAME`＝`ユーザーさん`）——kanade の prefetch は 204/空値を `ResourceOutcome::NoContent` として「不在」を sink へ渡すのみで、鏡像に既定値を**書かない**（既定値の二重定義を作らない・R4.2）。照会失敗（タイムアウト/IPC 断）も同様に不在扱いで boot を殺さず続行（`ResourceOutcome::Failed`・R4.1） | 正典（ukadoc）は 204/空値応答時の展開値を明示しない＝沈黙。実 SHIORI 照会経路（kanade boot prefetch・OnInitialize 後/OnFirstBoot 前）を通したうえで、不在時の既定値は dialogue-tags で確立済みの唯一定義点に一元化する（既定値の源を 1 箇所に保つ・R4.1/R4.2） | areka-P0-sylphya（kanade `schedule/resources.rs`・`schedule/boot.rs` prefetch／既定値の定義点は areka-P0-sakura-dialogue-tags に残置） |
| 選択確定カスケードの正典沈黙分岐一式（`On` 始まり ID で `OnChoiceSelectEx`/`OnChoiceSelect` を先行発火しないこと・カスケード最終段 204 の扱い・選択解決後に選択肢集合を破棄すること・選択肢タイムアウト既定値 30000ms・選択待ち中の複合 `Status` wire `talking,choosing`・`script:` 前置と CROW 複数 ID 形の M1 非対応縮退・秒→ms の四捨五入・タイムアウト指令 `NaN` の無期限畳み） | **詳細台帳へ委譲**＝`doc/choice-cascade-compat.md`（裁定 R-1〜R-6・1〜10 を `provenance = ukadoc \| ssp_secondary \| areka_discretion` の 3 値・正典引用・反証つきで記録） | 本 spec は正典沈黙分岐を 10 件超持ち込むため 1 行に畳めない。§2 沈黙ルールの運用実体として専用台帳を新設し、本表からはポインタで引く（DD-14）。裁定を変更する際は下流 `areka-P0-emo2-conformance-e2e` の期待値を再確認すること | areka-P0-choice-select-events（`doc/choice-cascade-compat.md`・DD-14/DD-15・Req2.8/8.1/8.2） |
| **バルーンが現れる契機**（正典は出現条件そのものに沈黙） | バルーン内に**可視の文字（グリフ）が置かれた瞬間**を唯一の契機とする。改行・カーソル移動・待機・内容消去はいずれも契機にならない。判定は「リビール済みグリフ数が増えた、かつその scope が現に不可視」のときだけ表示（`decide_content`） | **areka 裁量**。正典はバルーンを閉じる側（`\![set,balloontimeout,時間]`・`OnBalloonClose`）だけを規定し、開く条件を定めていない。観測量に `TextLayerState::visible_glyphs`（`crates/areka-emo-text/src/state.rs:440`＝リビール済みのみを数える）を採ったため、上記の一致は実装の帰結であって別条件を持ち込んだものではない。喋った scope だけが出ること・次の会話で出直すことも、この単一規則から導かれる | areka-P0-balloon-visibility（R7.6 / R2.1・R2.3・R2.5・`crates/areka/src/emo2_boot/balloon_visibility.rs:519`） |
| **喋っていない scope のバルーンを表示しないこと**（正典は scope 別の表示条件に沈黙） | 会話中・会話跨ぎとも表示しない。上行の単一規則の帰結であり（無発話 scope はグリフが増えないため契機が成立しない）、scope 別の追加ルールは設けていない | **areka 裁量**。参照実装 SSP は喋った scope のバルーンだけを出すが、正典に該当条文が無い | areka-P0-balloon-visibility（R7.6 / R2.7・R3.3） |
| **ポインタ滞在・ドラッグ中のタイムアウト延命と、解除後の計測やり直し**（正典はいずれにも沈黙） | 抑止条件が 1 つでも成立する間は満了を保留し、**すべての抑止が解けたフレームで現在時刻を起点に既定時間を計り直す**（抑止前の残り時間を再開しない）。抑止の種別は⑴バルーン窓のドラッグ中⑵可視な当該 scope のバルーン上へのポインタ滞在⑶選択肢表示中、の 3 つ。抑止の観測が取れないフレームは**抑止なし**として扱う | **areka 裁量**。正典はカウント起点を「スクリプトの表示が終わってから」としか定めず、読んでいる最中の延命に触れていない。観測不能を抑止と読むと消えないまま固着する側へ倒れるため、非抑止側へ縮退させた。⑶の選択肢の状態は `areka-P0-choice-select-events`（完了済み）が所有する値を受け取るだけで、本 spec は選択肢のタイムアウト規約を持たない（二重所有を作らない） | areka-P0-balloon-visibility（R7.6 / R5.2・R5.3・R5.5・`balloon_visibility.rs:716`・やり直しの起点計算は `:642-643`） |
| **バルーンの面切替が可視性を変えないこと**（`\b[ID]` および面のアニメーション定義の反復再生由来を含む。正典は両者の関係に沈黙） | 面 ID と可視性を直交させる。不可視のバルーンへ面切替が届いても表示状態（合成・供給面・文字の配置先・寸法・拡大率）だけが更新され、可視にはならない。可視化を行うのは専用の入口 `EmoPresenter::show_target` のみ | **areka 裁量**。表示対象ごとに可視性の所有者（指令駆動／外部所有）を持たせ、バルーンを外部所有として表示指令から可視化手順だけを外した。非表示側（`\b[-1]` 等）は所有者に依らず従来どおり即時に効く非対称を保つ | areka-P0-balloon-visibility（R7.6 / R6.2・R6.9・`crates/areka-emo-present/src/presenter/target.rs:31`・`presenter/show.rs:194`/`:221`・`presenter/visibility.rs:69`） |
| **会話終了後にバルーンを消すまでの既定の待ち時間**（正典は「本体設定の喋りタイムアウト」の**存在**のみを示し、値には沈黙） | **30 秒**。定義箇所は `DEFAULT_BALLOON_TIMEOUT_SECS` ただ 1 つ。満了予定の起点は**会話の占有区間の終端**（待機を含む台本の終わり）＋既定時間であり、「計測が成り立った最初のフレームの現在時刻」＋既定時間ではない（観測はフレーム単位で飛び飛びに入るため、後者では観測の遅れがそのまま満了のずれになる）。実機で満了を観測するための短縮のみ環境変数 `AREKA_BALLOON_TIMEOUT_MS`（正の整数ミリ秒）で受ける＝本番の既定経路では未設定 | **正典整合（既定値が存在すること）＋areka 裁量（値そのもの）**。正典 `OnSurfaceRestore` は「SSP の場合は本体設定の喋りタイムアウトで設定された秒数+１５秒後に発生（つまりバルーンが閉じてから１５秒後）」「喋りタイムアウト設定が０のときだけ `OnSurfaceRestore` が発生するまでバルーンが表示され続ける」と記し、**ベースウェア本体設定の既定値が存在する前提**を確定させるが、その秒数自体は示していない。起点は `\![set,balloontimeout,時間]` の「スクリプトの表示が終わってからカウントされる」に一致させた | areka-P0-balloon-visibility（R7.6 / R4.2・R4.3・R9.5・`balloon_visibility.rs:73`・起点の確立は `:654`） |
| **会話が中断で終わったときのタイムアウト起点**（正典は中断終了時の起点に沈黙） | 正常終了と**同一の起点値**＝台本の占有区間の終端を用いる。中断のみを理由とする即時非表示の経路は実装に存在せず、起点の誤差は必ず表示を保持する側へ倒れる。中断が起きた時刻そのものを起点に採る精密化は行わない | **areka 裁量**。中断時刻を起点にするには中断の理由と位置を表示側から会話進行側へ渡す配線が要り、それは下行の `OnBalloonBreak` の実発火に必要な情報と同一である。配線を伴わない精密化だけを先に入れると同じ情報を二重に作ることになるため、発火系と一体で扱う | areka-P0-balloon-visibility（R7.6 / R4.6・`balloon_visibility.rs:654`）→ 追跡 `areka-P0-balloon-canon-residue`（**解禁条件**＝M2 の互換面拡大時、または `areka-P0-emo2-conformance-e2e` が欠陥を検出したとき。roadmap.md:132 の M2 解禁ゲート） |
| `\![set,balloontimeout,時間]`（バルーン表示のタイムアウト時間指定）の**バルーン寿命側の実導出** | **M1 非受理**（完全な語彙と意味論の記録のみ）。意味論＝単位はミリ秒／カウント起点はスクリプトの表示が終わってから／`0` または負数（正典の箇条書きでは「`0` か `-1`」）でタイムアウトなし／時間指定を省略すると既定値へ戻る／そのスクリプト中のみ有効。現状このタグは汎用 `\!` コマンドとして転記されるだけで、可視性の判断側は上行の既定の待ち時間しか読まない | **正典**（意味論は ukadoc `list_sakura_script.html` の当該項そのまま）。**縮退の理由は実物根拠**——適合対象ゴースト emo2 の辞書全体に `balloontimeout` が 1 件も現れず、実導出しても走る経路が無い（`.kiro/specs/areka-P0-balloon-visibility/brief.md:69` の fixture 実測）。⚠ 本表**上方の「compile 側時間指令 allowlist」の行が同じタグの compile 側（台本コンパイル時の干渉）を扱っており、そちらの追跡先は `areka-P0-sakura-time-directives`** である。本行はバルーンの表示寿命という別側面を扱い、両者は排他ではない——実導出時は双方が必要になる | areka-P0-balloon-visibility（R7.1）→ 追跡 `areka-P0-balloon-canon-residue`（**解禁条件**＝M2 の互換面拡大時。roadmap.md:132 の M2 解禁ゲート） |
| `OnBalloonClose` ／ `OnBalloonTimeout` ／ `OnBalloonBreak` の SHIORI 発火 | **M1 非発火**（語彙・Reference 割当・受け渡し口の型のみを残す）。Reference 割当＝`OnBalloonClose`: Ref0＝閉じる際に表示されていたスクリプト／`OnBalloonTimeout`: Ref0＝タイムアウトした際に表示されていたスクリプト・Ref1＝残り時間／`OnBalloonBreak`: Ref0＝中断の操作が起きたスクリプト・Ref1＝中断が起きたバルーンの scope 番号（本体側 0・相方側 1・以降も）・Ref2＝中断位置（スクリプト先頭からの文字数・さくらスクリプトのタグを含む）。表示側から会話進行側へ渡す情報は `BalloonLifecycleNotice` として型で押さえてある | **正典**（Reference 割当は ukadoc `list_shiori_event.html` の各項そのまま）。**縮退の理由は実物根拠**——emo2 の辞書に 3 イベントのハンドラがいずれも現れず消費者ゼロ（`.kiro/specs/areka-P0-balloon-visibility/brief.md:69`。`OnBalloonChange` のみ存在するがメニューからのバルーン切替で本件と別物）。**予約した口には現時点で構築側も消費側も存在しない**——その事実と、それでも型を残す理由（語彙と Reference 割当をコード側にも固定し、語彙だけが増えて配線の追跡が失われる失敗を繰り返さないため）を注記と `#[allow(dead_code)]` の対で明示し、棚卸で検出できる形にしてある。実発火が着地すれば許容ごと外れる | areka-P0-balloon-visibility（R7.2 / R7.8・`crates/areka/src/emo2_boot/talk_lifecycle.rs:159-196`）→ 追跡 `areka-P0-balloon-canon-residue`（**解禁条件**＝M2 の互換面拡大時。roadmap.md:132 の M2 解禁ゲート） |
| `OnBalloonClick` という SHIORI イベントが**正典に存在しない**こと | **独自のクリック閉鎖イベントを新設しない**。クリックによる閉鎖は `OnBalloonClose`（「バルーンを閉じた際に発生」）へ集約される。上行の予約型へクリック専用の分岐を足すことも禁じる | **正典**。ukadoc のイベント一覧に `OnBalloonClick` は無く、`OnBalloonClose` の説明も閉鎖の手段を限定していない。**本行は先送りではなく確定判断**（互換面を広げても判断は変わらない）ため、受け皿となる追跡 spec を持たない | areka-P0-balloon-visibility（R7.3・`talk_lifecycle.rs:181`） |
| `\x` ／ `\x[noclear]`（バルーンをクリック待ちにする） | **M1 非実装**（完全な語彙と意味論の記録のみ）。意味論＝`\x` はバルーンをクリック待ちにし、クリック後 scope がリセットされて `\0` となり `\f`（文字装飾）系の効果も解除される（`\e` で解除される指定は継続）／`\x[noclear]` は一時クリック待ちで、クリック後もバルーンの内容と scope が保持され `\f` 系の効果も残る。現状この 2 つは台本の転記層で**原文のままの寛容パススルー**（`Instruction::Raw`）へ落ち、下流に消費者がいない | **正典**（意味論は ukadoc `list_sakura_script.html` の 2 項そのまま）。**縮退の理由は実物根拠**——emo2 の辞書に `\x` が現れない（`.kiro/specs/areka-P0-balloon-visibility/brief.md:69`）。クリック待ちは可視性ではなく会話の進行を止める機能であり、本 spec の単一規則（可視の文字があるか）では表現できないため、可視性の側で近似実装しない | areka-P0-balloon-visibility（R7.4・**縮退シームは 2 経路ある**——裸形 `\x` は `crates/areka-parsers/src/sakura/decode.rs:180` → `decode_passthrough_bare`（`:325`）、角括弧形 `\x[noclear]` は同 `:213` → `decode_passthrough_tag`（`:303`）。どちらも `Instruction::Raw` へ落ちる）→ 追跡 `areka-P0-balloon-canon-residue`（**解禁条件**＝M2 の互換面拡大時。roadmap.md:132 の M2 解禁ゲート） |
| ukadoc `Status [SSP拡張]` の `balloon`（表示中のバルーン ID 群）の実導出・報告 | **M1 非導出**（語彙は `areka-P0-idle-talk` が第一級構造として保持済み・非アクティブへ縮退したまま）。**実導出したときの唯一の情報源は本 spec が確立したバルーンの表示状態**（読み口 `EmoPresenter::target_visible`）とする。可視性の第 2 の帳簿を別に作らない | **正典**（`Status [SSP拡張]` の語彙）。`ExecutionState::Balloon` は本体に構築側の呼び手が 1 つも無く、実測差替シームのまま。UI スレッドが持つ可視性を会話進行側へ運ぶ配線が要るため、源が着地した時点で台帳 spec が拾う | areka-P0-balloon-visibility（R7.5・`crates/areka-kanade/src/status.rs:37`・`crates/areka-emo-present/src/presenter/read.rs:208`）→ 追跡 `areka-P0-status-execution-states`（**解禁条件**＝残状態の源サブシステムが着地した時点で just-in-time。roadmap.md:108・:132。`balloon` については本 spec の着地で源が揃っている） |
