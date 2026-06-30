---
inclusion: manual
updated_at: 2026-06-28
---

# Roadmap — areka M1（最小 SSP 互換ベースウェア）

> **このロードマップは M1 のみを扱う。** M2 以降は **M1 完成後に実物を見て組み直す**（憶測で先に書かない）。
> 正本配置: 本ファイルが M1 ロードマップ正本（kiro 標準パス `.kiro/steering/roadmap.md`）。`focus.md`（`inclusion: always`）から辿る。設計判断の正本は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md)。M1 実物スコープは [doc/emo2-conformance-scope.md](../../doc/emo2-conformance-scope.md)。

## M1 ゴール

areka（**x64**）が最小 SSP 互換ベースウェアとして、適合対象ゴースト **emo2**（作者自作・脳=`pasta.dll`・**32bit SHIORI**）を「**そのまま**」起動→会話→撫で→メニュー→終了まで E2E 実走させる。

- emo2 が動く＝同じ汎用 32bit ブリッジで里々/YAYA も動く土台（互換＝普及の入口）。
- 「伺かっぽいマスコット」ではなく「**伺か互換系**」であること自体が長期ロードマップの起点。
- M1 スコープは emo2 が実際に使う機能で**実物定義**（[doc/emo2-conformance-scope.md](../../doc/emo2-conformance-scope.md)）。完全網羅・予測実装はしない。

## 実装規律（balloon-system の失敗から得た正）

旧ロードマップは「**spec が spec を産む工場**」と「**憶測の過剰アーキテクチャ**」で、実装まで進んでも使えない物を生んだ（実例: 旧 `wintf-P0-balloon-system` は子 spec を 8 本産んだだけで動くバルーンはゼロ。`crates` に実装なし）。同じ病を二度と入れない規律：

- **実装ファースト**: 各作業ユニットの成果物は「emo2 が実際に動く」検証済みコード。spec でも、先回りの抽象でもない。
- **spec 工場の禁止**: 成果物が子 spec になる構造を作らない。1ユニット＝1かたまりの動く振る舞い。
- **最小実装＋薄い拡張シーム**: emo2 が使う分だけ実装し、拡張は型/レジストリの口だけ残す。choice/link/text-effect・追加 SERIKO interval/method・SAORI 等は**実装しない**（emo2 未使用）。抽象は「2 例目の実物」が要求してから。
- **動く資産から建てる**: ゼロから再アーキテクトしない（下記「立つ土台」）。

## 立つ土台（completed・実コードあり）

- 透過 ULW/DComp 切替（別プロセス自動透過）、event/hit-test/alpha-mask、dola 演出ランタイム（コア〜ループ/nested）。
- **SHIORI 契約チェーン**: `areka-P0-shiori-com`（内部唯一 ABI `IShiori`）→ `-shiori-protocol` → `-shiori-protocol-split` → `-shiori-reference`（reference DLL・native 脳デモ・`shiori_create` 入口）。
- **mock-shell**（`areka-mock-shell`）: 窓表示＋縦書き Typewriter バルーン＋ドラッグ追従＝**動くレンダリング素材**。
- pasta エンジン source: vendored `vendors/pasta`（M2 の native x64 化の素・**M1 では使わない**）。

## 唯一の耐力壁（先進坑で先に潰す）

「**x64 areka が emo2 の 32bit `pasta.dll` を駆動できるか**」が M1 の生死を分ける。ここだけ先進坑（pilot・使い捨て）で先に検証し、**go 判定（開発者・人間判断）**を取ってから本実装へ。二坑モデル規律は [two-tunnel.md](two-tunnel.md)。

- 先進坑: `crates/pilot/examples/shiori-host-32/`。
- **go 基準**: x64 から 32bit `pasta.dll` を 1 往復（load→OnBoot→`Value` 受領→unload）成功 ＋ 窓持ち SHIORI のメッセージループ生存。SAORI は emo2 未使用ゆえ対象外。

## 横断データ構造（M1 着手時から組み込む）

> 後付けが高コストな基盤構造ゆえ**最初から正しく持つ**（最小実装・構造は拡張可能）。詳細は記憶 areka-unified-shell-balloon-graphics。

- **シェル/バルーン統一**: 描画エンジン上でシェルとバルーンを**区別しない**。バルーン＝シェル surface 上に被さる**文字レンダリング層**。バルーン枠も surface＝普通にアニメ可。
- **element に他サーフェス参照可**: SERIKO の element に画像だけでなく**他サーフェスを指定可**（入れ子）。surface 合成は再帰的＝入れ子アニメの基盤。
- **element 配置＝D2D 変換行列**: 基本 X,Y でなく **D2D 変換行列**を内部表現。x,y は単位行列の特例。回転・拡縮など D2D が普通に出来る構造をそのまま取り込む。emo2 は単位平行移動＋平面 overlay のみ使うが、**構造（行列＋入れ子＋統一エンジン）は最初から持つ**。
- 位置づけ: 旧「汎用シーングラフは M2 後ろ倒し」の**部分的前倒し**（データ構造のみ M1・上位演出エンジンは依然 M2）。
- **アニメーションエンジンは2つ**（記憶 areka-two-animation-engines）: ①**さくらスクリプト再生エンジン**（talk timeline）＋ ②**シェルアニメーションエンジン**（SERIKO ループ）。`conductor`（SHIORI イベント循環）→ ① へ script。**① は下流が2つに分岐**: shell アニメ（`\s` 等）は ② へ／**テキスト（typewriter）は render(text-layer) へ直接**（テキスト描画はシェルアニメではないため ② を経由しない）。② は surface 合成を render に毎フレーム駆動。両 engine は **dola（完了・タイミング層）**上。
- **並行モデル（設計指針・記憶 areka-concurrency-model）**: 各エンジン層＝**チャンネル通信のアクターモデル**、**エンジンインスタンスごとに独立スレッド**（async 中心でなくスレッド独立＝ゴーストの連続ループ/message pump に馴染む）。**エンジン間 I/O 契約＝チャンネルのメッセージ型**（クロスエンジン I/O 節と一致）。**但し ⑥render/window は UI スレッド固定**（D2D 単一スレッド＋window アフィニティ）＝他 actor は worker スレッドから描画/入力を channel で render へ。①host-32 は別プロセス＝天然のアクター境界（IPC が channel）。④sakura は per-talk transient。

## 構築（初期化）モデル — 各エンジンのコンストラクタ

> コンストラクタ＝そのエンジンを構築する**定義入力**。独立 spec でも各エンジン unit に埋め込みでも可＝**埋め込み方針**（別 unit を増やさない）。記憶 areka-engine-construction-model。

- **root（親）**: ルート定義 `install.txt` がゴースト全体の親コンストラクタ。
- **ghost**: `areka-P0-package-mount` が root 定義を解決して構築（以下を子として構築）。
- **SHIORI / host-32**: コンストラクタ＝ゴーストフォルダ定義（`ghost/master/descript.txt` の `shiori,pasta.dll` ＋ dir）。
- **shell-anim-engine（②）**: コンストラクタ＝**SERIKO/shell 定義（`surfaces.txt`）＋ balloon 定義（balloon descript）**（統一エンジンゆえ両方が構築入力）。
- **sakura-engine（①）**: コンストラクタ＝**さくらスクリプト**（SHIORI 応答ごと・**runtime・per-talk・transient**）。
- 構築は2系: **load-time**（root→ghost→{SHIORI, shell-anim-engine}・一度）と **runtime**（script→sakura-engine・都度）。

## M1 実装ユニット（実現可能な粒度）

> **粒度基準**: 1ユニット＝「コードを走らせて観測できる**単一 pass/fail** を持ち、それを観測するのに別ユニットを先に作る必要がない」もの。done が複数の独立観測に割れるなら粗すぎ→分割。
> 正規名は**暫定**（着手時に確定）。**spec 工場にしない**＝下記はユニット名の登録であり brief.md 群ではない。着手時に最小 spec/task を just-in-time で切る。
> **粒度の真実**: 作業は **M-boot に前倒し集中**（約16ユニット＝M1 の山）。「最初の起動」が本体で以降は薄い増分。

### M-boot ＝ `areka-P0-emo2-boot`（最初の可視結果・最重量＝約16ユニット）
emo2 が起動して喋る。下記 5 トラックを結線して達成（⓪ ゴーストエンジンが全体を統括）。

**⓪ ゴーストエンジン（最上位 owner・全エンジンを統括）**
- `areka-P0-ghost-setup` — ゴースト lifecycle（package-mount で構築→boot 統括→close）。✔ install.txt から起動〜終了を統括
- `areka-P0-window-placement` — サーフェス窓の生成＋既定位置＋ドラッグ（`areka-mock-shell` 実コードから）。✔ むらさき/エモ窓が出てドラッグ移動

**① SHIORI 通信層エンジン host-32（耐力壁・`pilot/shiori-host-32` がトラックを gate）**
- `pilot/shiori-host-32` — 使い捨て feasibility。✔ go: 32bit pasta.dll 1往復
- `areka-P0-host32-ipc` — x64↔32bit helper＋pipe＋handshake/lifecycle。✔ 往復 echo
- `areka-P0-host32-shiori-load` — LoadLibrary pasta.dll＋load/unload/request 解決＋load(ghostdir)。✔ load 成功・無crash
- `areka-P0-host32-request` — SHIORI/3.0 build＋marshal＋response Value＋charset。✔ x64 が emo2 OnBoot の Value 受領
- `areka-P0-host32-lifecycle` — helper msg loop＋OnSecondChange poll＋unload＋crash監視。✔ N秒運転→clean unload

**parsers（並行・単体テスト可・host 不要）**
- `areka-P0-sakura-parse` — emo2 タグ subset→token。✔ boot script を token 化
- `areka-P0-shell-parse` — surfaces.txt/descript→surface モデル。✔ emo2 shell parse
- `areka-P0-balloon-parse` — balloon descript/Ns マージ→モデル。✔ emo2-kakukaku parse
- `areka-P0-package-mount` — install.txt/dir→mount。✔ emo2 layout 解決

**runtime 制御階層（上→下に駆動・両 anim engine は dola 上）**
- `areka-P0-conductor` — SHIORI イベント循環（OnSecondChange pump・host-32 送受・Value を sakura-engine へ）。✔ OnBoot→Value 受領→再生開始
- `areka-P0-sakura-engine` — **さくらスクリプト再生エンジン**（talk timeline: `\w/\_w` wait・`\s` で shell-engine へ surface 指令・text を text-layer へ・seq）。✔ boot script を時系列再生
- `areka-P0-shell-anim-engine` — **シェルアニメーションエンジン**（SERIKO ループ＋surface 状態＋MAYUNA bind・render を毎フレーム駆動）。M-boot は静的＋指令適用、ループ(blink)は M-life。✔ 指令された surface を表示

**render engine（統一・`areka-mock-shell`＋dola から増分）**
- `areka-P0-surface-engine` — **シェルもバルーンも同一の surface 合成**。element＝{画像 | 他サーフェス参照（入れ子）}、配置＝**D2D 変換行列**。✔ surface0 ＋バルーン枠を surface として表示
- `areka-P0-text-layer` — バルーン文字を **engine 上に被せる層**（token→glyph→surface 領域）。✔ script がバルーンに描画＋scroll

### 増分（M-boot 後・**エンジン別＝並走可能**）

> 増分はエンジンへ帰属させる。**別エンジンに属する増分は並列着手可**（spanning する旧 unit はエンジン単位に分割済）。マイルストーン（M-dual 等）はエンジン横断の**統合点**であって作業単位ではない。
> **トラック全7**: ⓪ゴーストエンジン(owner)・①SHIORI 通信層(host-32)・②parser/loader・③conductor・④sakura-engine・⑤shell-anim-engine・⑥render-engine。⓪は最上位 owner（lifecycle/窓配置/位置永続化）。**①SHIORI 通信層・②parser/loader・⓪の大半は M-boot で完了**。増分を持つのは ③〜⑥ ＋ ⓪の位置永続化。

- **② shell-anim-engine**: `areka-P0-dual-surface`（side0/1＋surface alias）／ `areka-P0-mayuna-compose`（MAYUNA bind 多層）／ `areka-P0-shell-anim-loop`（SERIKO ループ＝blink random/bind+random）
- **① sakura-engine**: `areka-P0-sakura-dialogue-tags`（`\q`/`\_l`/`\![move]`）
- **conductor**: `areka-P0-idle-talk`（OnSecondChange 自発会話）／ `areka-P0-input-events`（OnMouseMove/OnMouseDoubleClick/OnChoiceSelectEx 配信）
- **render-engine**: `areka-P0-collision-geometry`（collision→region/actor 写像）／ `areka-P0-choice-render`（選択肢表示）／ `areka-P0-dual-window`（kero 2nd 窓）
- **⓪ ゴーストエンジン**: `areka-P0-position-persist`（`ghost.dat` 位置の保存/復元・ghost レベル永続化）

**統合点（マイルストーン＝横断結合）**:
- **M-dual** ＝ shell-anim:`dual-surface` ＋ render:`dual-window`
- **M-mayuna** ＝ shell-anim:`mayuna-compose`
- **M-life** ＝ shell-anim:`shell-anim-loop` ＋ conductor:`idle-talk` ＋ conductor:`input-events`(撫で) ＋ render:`collision-geometry`
- **M-dialogue** ＝ sakura:`sakura-dialogue-tags` ＋ conductor:`input-events`(dblclick/choice) ＋ render:`choice-render`
- **M-e2e** ＝ `areka-P0-emo2-conformance-e2e`（全エンジン統合・OnClose＋boot→talk→touch→menu→close 一周適合・M1 ゴール充足）

### クロスエンジン I/O（並走依存チェック）

> 一部ユニットは**複数エンジンに入出力**を持つ（例: 撫で反応＝render が入力／SHIORI が出力）。図に線は引かないが**並走性に影響するので依存をチェック**。conductor への指示はほぼ SHIORI 層から。

**並走安全（M-boot 済なら独立着手可・依存は M-boot のみ）**:
- conductor:`idle-talk`（OnSecondChange→SHIORI→sakura）
- shell-anim:`mayuna-compose` / `shell-anim-loop`
- ghost:`position-persist`（自己＋window-placement）

**クロスエンジン結合（I/O 契約を先に1つ決めれば両側を並列実装可）**:
- **撫で（M-life）**: render:`collision-geometry`（入力 mouse→region/actor）⟷ conductor:`input-events`（出力 OnMouseMove→SHIORI）。先に **region/actor I/O 契約**。
- **選択肢/メニュー（M-dialogue）**: sakura:`sakura-dialogue-tags`（`\q` 出力）⟷ render:`choice-render`（表示）⟷ conductor:`input-events`（OnChoiceSelectEx）。先に **\q/選択 契約**。
- **二人立ち（M-dual）**: shell-anim:`dual-surface` ⟷ render:`dual-window` ⟷ ghost:`window-placement`。先に **2窓/surface 契約**。
- **移動 `\![move]`**: sakura:`sakura-dialogue-tags` ⟷ ghost:`window-placement`（キャラ移動＝窓移動）。

> **結論**: エンジン所有での並走は原則可。ただし上記**結合クラスタは I/O 契約を先決してから**両側を並列実装する（契約未定で並走すると齟齬）。完全独立ユニットは即着手可。

### マウス制御の所有 ＆ メニュー方針

- **総合的なマウス制御＝⑥render-engine の責務**（独立仕様は作らない）。窓のマウスメッセージ・**alpha hit-test**・ドラッグは**完了済み基盤**（`event-mouse-basic`/`event-drag-system`/`event-hit-test`/`event-hit-test-alpha-mask`）の上に render が所有。M1 新規は `collision-geometry`（hit→ゴースト collision region/actor 写像＝「範囲」のみ）だけ。render が入力を解決し conductor:`input-events` が SHIORI へ配信。
- **M1 のメニュー＝バルーン `\q` 選択肢**（emo2 の double-click メニュー）＝ `choice-render`＋`sakura-dialogue-tags`＋`input-events`。
- **owner-draw メニュー（SSP 風 右クリック system メニュー・ゴースト管理 chrome）は M2**（OS owner-draw・上記 balloon 選択肢とは別物）。

## 着手手順（just-in-time briefing・spec 工場回避）

> brief は**前もって量産しない**（balloon-system の spec 工場の轍を踏まない）。着手するユニットの brief だけを着手時に書く。

1. 次ユニットを選ぶ（`pilot/shiori-host-32` → M-boot の⓪①②＋③〜⑥最小 → 増分）。
2. ロードマップから**そのユニットの依存を読む**: M-boot 前提か／pilot ゲート下か／クロスエンジン I/O 結合クラスタ（契約先決要）か／並走安全か。
3. `/kiro-discovery`（再入・「ロードマップから次の仕様をブリーフィング」）または `/kiro-spec-init <unit>` で**そのユニットの brief を1つだけ**生成。
4. 以降は通常フロー（kiro-start → design → tasks → impl）。**`/kiro-spec-batch` は使わない**（一括＝工場化）。

**依存の所在**（機械可読の per-unit `Dependencies:` 行は持たない・人/AI がロードマップから導出）:
- 順序ゲート＝「M-boot 前提」「pilot が host-32 トラックを gate」
- 並走単位＝エンジン別帰属（⓪〜⑥）
- クロスエンジン結合＝I/O 契約4クラスタ（撫で/選択肢/二人立ち/移動）＝channel 契約（並行モデル節）
- 並走安全＝完全独立ユニットのリスト（クロスエンジン I/O 節）

## 制約

- Rust 2024・マルチクレート（wintf/dola/areka ＋最小依存 `shiori-abi`）。32bit 可搬性を崩さない。
- 透過は ULW/DComp 切替式（実装済み・ULW 既定）。SHIORI 内部唯一 ABI=`IShiori`(COM, HSTRING/UTF-16)。過去互換は 32bit Rust ホスト（flat-C/HGLOBAL/charset/自前 IPC）。
- 設計判断の変更は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md) を正本として更新。

## ポートフォリオ（2026-06-28・clean slate）

- `.kiro/specs/` 直下 active = **0**（憶測仕様を全伐採し更地化。実装ファーストで着手時に作る）。
- `completed/` = 99（歴史・M1 が立つ土台の記録）。
- 旧 active/brief（M1 憶測・M2 reference・出荷層）・backlog（P1-P3）・`_rejected/`・旧戦略メモは**削除**（git 履歴に保全。必要時に復元可）。

## M2 以降

**M1 完成後に、実物を見て組み直す。** 本ロードマップでは扱わない（pasta の native x64・`IShiori` in-proc 化、縦書き・ベクトル描画・AI、**owner-draw 右クリック system メニュー（ゴースト管理 chrome）**、互換面拡大＝Shift_JIS/SAORI/里々・YAYA 網羅/NAR 等はその時に）。

---

## wintf 基盤先進坑: クリック透過 αトグル方式（M1 外・wintf 基盤層）

> **位置づけ**: 本トラックは M1（emo2-boot）とは別軸の **wintf 基盤層**の改善。M1 ユニット群（⓪〜⑥）には含めない。ここに記すのは two-tunnel.md（line 87）が `_Depends(confirmed):` ゲートの宿主を roadmap.md と定めているため。

### 動機（既存「ULW 一択」結論の穴）

[tech.md](tech.md) line 83 / 本ファイル line 30 は別プロセス透過を **「実質 ULW 一択」** と断定し ULW/DComp 切替式を「実装完了済み」と記録する。しかしこの結論は **HTTRANSPARENT・SetWindowRgn・ULW** の 3 択比較で、**`WS_EX_TRANSPARENT` 動的トグル方式（winit `set_cursor_hittest` 相当・プロセス境界を越える第 4 の手）を検討していない**。

**真の動機は「DComp 描画を捨てられない」こと（開発者）。** ULW は CPU ビットマップ方式で **DComp スワップチェーン合成と併用不可**（記憶 areka-transparency-requires-layered-window）。すなわち 3D（DComp/GPU 合成）ウィンドウにとって ULW はそもそも選択肢になり得ず、ULW は別プロセス透過のために 3D 描画を諦める踏み絵になっている。`WS_EX_TRANSPARENT` 動的トグル（別スレッドのカーソル監視＋αマスク問い合わせで透明領域のみ透過・CPU 転送なし）は **DComp 描画を維持したまま別プロセス透過を成立させる事実上唯一の現実解**。**他社 3D マスコットが採用している実証済み手段**でもある（ただし十分な検証・エンバグ対応を要する前提）。既存「ULW 一択」結論と矛盾する新方向ゆえ **先進坑で先に潰す**（二坑モデル教義）。

### pilot（先進坑・使い捨て）: `pilot-clickthrough-alpha-toggle`

- 配置: `crates/pilot/examples/pilot-clickthrough-alpha-toggle/`（README 3 幕＋ REPORT.md が一次記録）。
- 検証: 透過トップモスト窓＋中央不透明領域、16ms 周期ワーカ（`event_listener`＋`std::thread`・tokio 禁止）がカーソル位置→αマスク関数（仮・円判定）を問い合わせ、状態変化時のみ `SetWindowLongPtr(GWL_EXSTYLE)`＋`SetWindowPos(SWP_FRAMECHANGED)` で `WS_EX_TRANSPARENT` を付け外し。`WS_EX_LAYERED` 不使用・`WM_NCHITTEST` 自前ハンドル禁止・DPI per-monitor-v2。
- **go 基準**（人間判断）: 試験項目 T1〜T8 のうち **T1・T2・T3・T4・T6 が ✅ 必須**、T5・T7・T8 は ✅ または軽微な条件付き合格（理由明記）。レポートは合否問わず作成し依頼者の判断を仰ぐ（AI 単独で go 判定しない）。

### main（本坑）: `wintf-clickthrough-alpha-toggle`

```
_Depends(confirmed): pilot-clickthrough-alpha-toggle
```

- pilot の go 判定が出るまで **BLOCKED**（go 前着手は規律違反）。pilot 知見はクリーンに掘り直す（コピペ donor 禁止・README 検証結果を参照）。
- 本体 wintf へ `WS_EX_TRANSPARENT` 動的トグルを導入し、本体αマスク（実描画αバッファ／`AlphaMask::is_hit`）参照でキャラ領域のみクリック可にする。
- **ULW との共存方針（開発者決定）**: 至上要件は **DComp 描画の維持**。本方式は DComp 経路に透過能力を授けるもの。本仕様が完全に有効と判断されれば **ULW ルートは破棄**。ただし他社実績ある手段とはいえ**十分な検証期間・エンバグ対応**を置き、**当面は ULW と並走**させる。tech.md/本ファイルの「ULW 一択」記述は本トラック確定時に更新対象。
- 接続先候補（調査済み）: `CompositionMode`（`ecs/window/components.rs`・生成時固定）／`compute_ex_style()`（`runtime/window_factory.rs`）／`HitTestMode::AlphaMask`・`AlphaMask::is_hit`（`ecs/layout/hit_test/`・`ecs/widget/bitmap_source/`）／`VsyncEventBridge`（`event_listener`・`runtime/tick_bridge.rs`）／D2D1 staging αバッファ（`ecs/graphics/compositor.rs`）。

### 依存マップ検証（two-tunnel 手動チェックリスト）

- 被覆: 不確実な本坑 `wintf-clickthrough-alpha-toggle` は go ゲート pilot を持つ ✓
- 孤児なし: pilot は対応本坑を名指し、本坑は pilot を `_Depends(confirmed):` で参照 ✓
- 循環なし／DAG: pilot → main の単一エッジ（巡回なし）✓
- 合否基準明示: go 基準（T1・T2・T3・T4・T6 必須）を上記に明示 ✓
