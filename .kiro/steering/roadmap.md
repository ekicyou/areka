---
inclusion: manual
updated_at: 2026-07-02
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

> **✅ 検証済（2026-07-01・先進坑 `completed/pilot-shiori-host-32`）**: go 基準(1)(2) を実 emo2 `pasta.dll` で実走充足。耐力壁は突破。一次記録は [README](../../crates/pilot/examples/shiori-host-32/README.md)（3 幕）。最終 go 判定は開発者が同記録を見て下す人間判断。

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

- **root（親）**: ゴースト起動時の構築起点定義 `ghost/master/descript.txt` がゴースト全体の親コンストラクタ（`install.txt` は NAR インストーラ配置マニフェスト＝**起動時不使用**・ukadoc 論拠）。
- **ghost**: `areka-P0-package-mount` が起点定義 `ghost/master/descript.txt` を解決してマウントモデルを構築（以下を子として構築）。
- **SHIORI / host-32**: コンストラクタ＝ゴーストフォルダ定義（`ghost/master/descript.txt` の `shiori,pasta.dll` ＋ dir）。
- **seriko＝shell-anim-engine（アニメ②）**: コンストラクタ＝**SERIKO/shell 定義（`surfaces.txt`）＋ balloon 定義（balloon descript）**（統一エンジンゆえ両方が構築入力）。
- **sakura＝sakura-engine（アニメ①）**: コンストラクタ＝**さくらスクリプト**（SHIORI 応答ごと・**runtime・per-talk・transient**）。
- 構築は2系: **load-time**（root→ghost→{SHIORI, shell-anim-engine}・一度）と **runtime**（script→sakura-engine・都度）。

## エンジン固有名（識別子・2026-07-02 確定）

> 7トラック（⓪〜⑥）の各エンジンに**識別用の固有名**を割り当てる。コード／spec／会話での参照はこの名で統一する。命名 register は「役割を体現する人名の手触り（`sakura`／`emo`／`kanade`）」と「伺か正準語（`ghost`／`shiori`／`seriko`／`parsers`）」の混成。詳細・由来は記憶 `areka-engine-names`。

| # | エンジン（説明名） | 固有名 | 由来 |
| --- | --- | --- | --- |
| ⓪ | ゴーストエンジン（最上位 owner・統括） | **`ghost`** | 伺か正準・全体 owner |
| ① | SHIORI 通信層エンジン host-32 | **`shiori`** | SHIORI 通信（crate `shiori-host32-*`／`IShiori` と一貫） |
| ② | parser/loader | **`parsers`** | crate `areka-parsers` と一致（複数形が正） |
| ③ | conductor（SHIORI イベント循環） | **`kanade`** | 奏でる＝全演者（`sakura`／`seriko`）を統べる。同名ゴーストの縁も |
| ④ | sakura-engine（さくらスクリプト再生） | **`sakura`** | さくらスクリプト |
| ⑤ | shell-anim-engine（SERIKO アニメ） | **`seriko`** | SERIKO ランタイム |
| ⑥ | render-engine（シェル/バルーン統一 surface 合成） | **`emo`** | emotion＝感情を描く可視層 |

**未着手ユニット名の固有名整合（2026-07-02 実施）**: `areka-P0-conductor`→`areka-P0-kanade`／`areka-P0-shell-anim-engine`→`areka-P0-seriko-engine`／`areka-P0-shell-anim-loop`→`areka-P0-seriko-loop`／`areka-P0-surface-engine`→`areka-P0-emo-surface`／`areka-P0-text-layer`→`areka-P0-emo-text-layer`。**不変**: `areka-P0-host32-*`（shiori トラック実装進行中＝改名衝突回避・名も既に整合）・`areka-P0-sakura-engine`・`areka-P0-ghost-setup` 等 ghost 系（既に固有名基準）・completed 仕様（歴史）。既知ドリフト: [balloon/model.rs:6](../../crates/areka-parsers/src/balloon/model.rs) doc コメントが旧名 `text-layer`/`surface-engine` を参照（該当ユニット着手時に追随修正）。

## M1 実装ユニット（実現可能な粒度）

> **粒度基準**: 1ユニット＝「コードを走らせて観測できる**単一 pass/fail** を持ち、それを観測するのに別ユニットを先に作る必要がない」もの。done が複数の独立観測に割れるなら粗すぎ→分割。
> 正規名は**暫定**（着手時に確定）。**spec 工場にしない**＝下記はユニット名の登録であり brief.md 群ではない。着手時に最小 spec/task を just-in-time で切る。
> **粒度の真実**: 作業は **M-boot に前倒し集中**（約16ユニット＝M1 の山）。「最初の起動」が本体で以降は薄い増分。

### M-boot ＝ `areka-P0-emo2-boot`（最初の可視結果・最重量＝約16ユニット）
emo2 が起動して喋る。下記 5 トラックを結線して達成（⓪ ゴーストエンジンが全体を統括）。

**⓪ ghost＝ゴーストエンジン（最上位 owner・全エンジンを統括）**
- `areka-P0-ghost-setup` — ゴースト lifecycle（package-mount で構築→boot 統括→close）。✔ descript.txt 起点のマウントから起動〜終了を統括
- `areka-P0-window-placement` — サーフェス窓の生成＋既定位置＋ドラッグ（`areka-mock-shell` 実コードから）。✔ むらさき/エモ窓が出てドラッグ移動

**① shiori＝SHIORI 通信層エンジン host-32（耐力壁・`pilot/shiori-host-32` がトラックを gate）**
- `pilot/shiori-host-32` — 使い捨て feasibility。**✅ 完了（2026-07-01・spec=`completed/pilot-shiori-host-32`・コードは `crates/pilot/examples/shiori-host-32/` に隔離保全）**: go 基準(1)(2) 実走充足＝32bit pasta.dll 1往復（x64 親が emo2 OnBoot `Value` 受領）＋窓持ちループ N秒生存→clean unload。跨ビットネス再入 WM_COPYDATA・`wintf-winmsg-executor` i686 実行時とも GO（fallback 不要）。→ 下流 `areka-P0-host32-*` の go ゲート充足（着手可・最終 go 判定は開発者）
- `areka-P0-host32-ipc` — x64↔32bit helper＋handshake。✔ 往復 echo。**✅ 完了（2026-07-02・spec=`completed/areka-P0-host32-ipc`）**: bytes-over-wire transport を3クレート（`shiori-host32-ipc`=proto / `-host`=x64+arm64 / `-helper`=i686）で本坑再掘（pilot 非コピペ）。トランスポートは WM_COPYDATA 一本化＋再入 RESPONSE（named pipe 不要）。実 i686 helper 越しの往復 echo が無デッドロック・無クラッシュで green（M1 ゲート指標充足）。pasta ロード/SHIORI parse/常駐 lifecycle は下流 `areka-P0-host32-*` の領分（本ユニットは seam のみ所有）
- `areka-P0-host32-shiori-load` — LoadLibrary pasta.dll＋load/unload/request 解決＋load(ghostdir)。✔ load 成功・無crash。**✅ 完了（2026-07-02・spec=`completed/areka-P0-host32-shiori-load`）**: 二層一体で **load_dir per-instance 貫通（D1）＋teardown を Drop(RAII) 全層一貫（D7）** を実現。WS-A=helper が実 i686 SHIORI DLL を `LoadLibraryW`→3 エクスポート解決→`load(load_dir)` 駆動し凍結 wire 上 1 byte ack で成否返送（トラック所有 testdll fixture で成功/失敗/無crash E2E・本物 pasta は env gate）。WS-B=`shiori-abi` を **IShioriFactory 融合 create＋Get/Notify 分離＋GetProperty/SetProperty＋型付き COM 引数＋module entry `shiori_factory`** へ是正（`load_dir` 欠落の根幹欠陥を根絶・reference/mock backend で証明）。凍結境界 `shiori-host32-ipc` 不改変。→ 下流 `areka-P0-host32-request`（request 呼出・SHIORI/3.0 marshal）が次フロント
- `areka-P0-host32-request` — SHIORI/3.0 build＋marshal＋response Value＋charset。✔ x64 が emo2 OnBoot の Value 受領
- `areka-P0-host32-lifecycle` — helper msg loop＋OnSecondChange poll＋unload＋crash監視。✔ N秒運転→clean unload

**② parsers（単体テスト可・host 不要。foundation が先行依存＝完全並行ではない）**
- `areka-P0-sakura-parse` — emo2 タグ subset→token。✔ boot script を token 化
- `areka-P0-parser-foundation` — **パーサー共通基盤**: charset デコード（冒頭 `charset` 行→encoding_rs 再デコード・全パーサー共通）＋ KV 読み込み（素朴マップ・surface 以外の全パーサー共通）。✔ charset 付き入力→KV マップ化（旧 `areka-P0-balloon-parse` を 2026-07-02 開発リジェクト→リネーム。知見は同 brief に集約）。**✅ 完了（2026-07-02・spec=`completed/areka-P0-parser-foundation`）**: `charset::decode`（BOM 読飛→冒頭ASCIIプリスキャン→宣言/既定 encoding_rs デコード）＋ `kv::parse_kv`（素朴 BTreeMap・後勝ち・trim）を `areka-parsers` に確立（encoding_rs 0.8 承認済追加・144 テスト緑）。下流 `shell-parse ∥ balloon-parse ∥ package-mount` の foundation 依存を充足
- `areka-P0-shell-parse` — surfaces.txt/descript→surface モデル（foundation 依存）。✔ emo2 shell parse。**✅ 完了（2026-07-02・spec=`completed/areka-P0-shell-parse`）**: `areka_parsers::shell` を四層（model←lexer←decode←parse）で確立。SERIKO/2.0 subset（overlay・interval 3種 bind/random/bind+random・矩形 collision・animationN 集約〔疎 pattern・負センチネル i64〕・surface.append ターゲット記述子〔範囲は非展開〕・kero.surface.alias 不透明キー写像〔重複保持〕）を寛容パース（Result なし・非パニック・subset 外は passthrough 吸収）。公開 facade `parse(&str)->Shell` へ 13 モデル型を集約。ukadoc 準拠自前テスト（主軸）＋emo2 fixture スモークで検証（225 テスト緑・clippy クリーン・追加依存なし）
- `areka-P0-balloon-parse` — balloon 3段参照優先度（sXXs/kXXs 起点＞descript＞既定）解決→モデル（foundation 依存・着手時に再切り出し）。✔ emo2-kakukaku parse。**✅ 完了（2026-07-02・spec=`completed/areka-P0-balloon-parse`）**: `areka_parsers::balloon`（`model`＝幾何＋フォント subset 型群・`Option` 直持ちで未指定＝`None` を `Some(0)` と区別・`#[non_exhaustive]`／`parse`＝descript＋画像別の後勝ち2層マージ→exact-key 写像・符号保持・寛容 None 降格）を確立。emo2-kakukaku 実物 fixture（R5.1/5.2/5.3）を charset→parse_str で適合検証、distractor キー非漏洩も固定（172 テスト緑・新規依存ゼロ）
- `areka-P0-package-mount` — `ghost/master/descript.txt`＋dir→mount（foundation 依存・起点は descript.txt。`install.txt`＝NAR 配置マニフェストは起動時不使用ゆえスコープ外）。✔ emo2 layout 解決。**✅ 完了（2026-07-02・spec=`completed/areka-P0-package-mount`）**: descript.txt 起点で SHIORI（dir=`ghost/master`・file は `Option` 推測禁止）＋shell（既定 `master` フォールバック・物理存在確認）の2点マウントを解決する `package` module を `areka-parsers` に確立。所在ベース識別（`type` 分岐なし）・foundation（`charset::decode`/`kv::parse_kv`）委譲・致命失敗3種（起点不在/読取不能/shell 不在）を `MountError` で観測可能化・emo2 実 fixture 統合テスト green（164 テスト・回帰なし・clippy clean）。下流 `ghost-setup`/`host-32`/`shell-parse` へ `MountModel` を供給

**runtime 制御階層 kanade／sakura／seriko（上→下に駆動・両 anim engine は dola 上）**
- `areka-P0-kanade` — **kanade（③conductor）**: SHIORI イベント循環（OnSecondChange pump・host-32 送受・Value を sakura-engine へ）。✔ OnBoot→Value 受領→再生開始
- `areka-P0-sakura-engine` — **sakura（④）＝さくらスクリプト再生エンジン**（talk timeline: `\w/\_w` wait・`\s` で seriko へ surface 指令・text を emo（text-layer）へ・seq）。✔ boot script を時系列再生
- `areka-P0-seriko-engine` — **seriko（⑤）＝シェルアニメーションエンジン**（SERIKO ループ＋surface 状態＋MAYUNA bind・render を毎フレーム駆動）。M-boot は静的＋指令適用、ループ(blink)は M-life。✔ 指令された surface を表示

**emo（⑥）＝render engine（統一・`areka-mock-shell`＋dola から増分）**
- `areka-P0-emo-surface` — **シェルもバルーンも同一の surface 合成**。element＝{画像 | 他サーフェス参照（入れ子）}、配置＝**D2D 変換行列**。✔ surface0 ＋バルーン枠を surface として表示
- `areka-P0-emo-text-layer` — バルーン文字を **engine 上に被せる層**（token→glyph→surface 領域）。✔ script がバルーンに描画＋scroll

### 増分（M-boot 後・**エンジン別＝並走可能**）

> 増分はエンジンへ帰属させる。**別エンジンに属する増分は並列着手可**（spanning する旧 unit はエンジン単位に分割済）。マイルストーン（M-dual 等）はエンジン横断の**統合点**であって作業単位ではない。
> **トラック全7**（括弧内が固有名・「エンジン固有名」節が正本）: ⓪ゴーストエンジン(owner・`ghost`)・①SHIORI 通信層(host-32・`shiori`)・②parser/loader(`parsers`)・③conductor(`kanade`)・④sakura-engine(`sakura`)・⑤shell-anim-engine(`seriko`)・⑥render-engine(`emo`)。⓪は最上位 owner（lifecycle/窓配置/位置永続化）。**①SHIORI 通信層・②parser/loader・⓪の大半は M-boot で完了**。増分を持つのは ③〜⑥ ＋ ⓪の位置永続化。

- **⑤ seriko（shell-anim-engine）**: `areka-P0-dual-surface`（side0/1＋surface alias）／ `areka-P0-mayuna-compose`（MAYUNA bind 多層）／ `areka-P0-seriko-loop`（SERIKO ループ＝blink random/bind+random）
- **④ sakura（sakura-engine）**: `areka-P0-sakura-dialogue-tags`（`\q`/`\_l`/`\![move]`）
- **③ kanade（conductor）**: `areka-P0-idle-talk`（OnSecondChange 自発会話）／ `areka-P0-input-events`（OnMouseMove/OnMouseDoubleClick/OnChoiceSelectEx 配信）
- **⑥ emo（render-engine）**: `areka-P0-collision-geometry`（collision→region/actor 写像）／ `areka-P0-choice-render`（選択肢表示）／ `areka-P0-dual-window`（kero 2nd 窓）
- **⓪ ghost（ゴーストエンジン）**: `areka-P0-position-persist`（`ghost.dat` 位置の保存/復元・ghost レベル永続化）

**統合点（マイルストーン＝横断結合）**:
- **M-dual** ＝ seriko:`dual-surface` ＋ emo:`dual-window`
- **M-mayuna** ＝ seriko:`mayuna-compose`
- **M-life** ＝ seriko:`seriko-loop` ＋ kanade:`idle-talk` ＋ kanade:`input-events`(撫で) ＋ emo:`collision-geometry`
- **M-dialogue** ＝ sakura:`sakura-dialogue-tags` ＋ kanade:`input-events`(dblclick/choice) ＋ emo:`choice-render`
- **M-e2e** ＝ `areka-P0-emo2-conformance-e2e`（全エンジン統合・OnClose＋boot→talk→touch→menu→close 一周適合・M1 ゴール充足）

### クロスエンジン I/O（並走依存チェック）

> 一部ユニットは**複数エンジンに入出力**を持つ（例: 撫で反応＝render が入力／SHIORI が出力）。図に線は引かないが**並走性に影響するので依存をチェック**。kanade（conductor）への指示はほぼ SHIORI 層から。

**並走安全（M-boot 済なら独立着手可・依存は M-boot のみ）**:
- kanade:`idle-talk`（OnSecondChange→SHIORI→sakura）
- seriko:`mayuna-compose` / `seriko-loop`
- ghost:`position-persist`（自己＋window-placement）

**クロスエンジン結合（I/O 契約を先に1つ決めれば両側を並列実装可）**:
- **撫で（M-life）**: emo:`collision-geometry`（入力 mouse→region/actor）⟷ kanade:`input-events`（出力 OnMouseMove→SHIORI）。先に **region/actor I/O 契約**。
- **選択肢/メニュー（M-dialogue）**: sakura:`sakura-dialogue-tags`（`\q` 出力）⟷ emo:`choice-render`（表示）⟷ kanade:`input-events`（OnChoiceSelectEx）。先に **\q/選択 契約**。
- **二人立ち（M-dual）**: seriko:`dual-surface` ⟷ emo:`dual-window` ⟷ ghost:`window-placement`。先に **2窓/surface 契約**。
- **移動 `\![move]`**: sakura:`sakura-dialogue-tags` ⟷ ghost:`window-placement`（キャラ移動＝窓移動）。

> **結論**: エンジン所有での並走は原則可。ただし上記**結合クラスタは I/O 契約を先決してから**両側を並列実装する（契約未定で並走すると齟齬）。完全独立ユニットは即着手可。

### マウス制御の所有 ＆ メニュー方針

- **総合的なマウス制御＝⑥emo（render-engine）の責務**（独立仕様は作らない）。窓のマウスメッセージ・**alpha hit-test**・ドラッグは**完了済み基盤**（`event-mouse-basic`/`event-drag-system`/`event-hit-test`/`event-hit-test-alpha-mask`）の上に emo が所有。M1 新規は `collision-geometry`（hit→ゴースト collision region/actor 写像＝「範囲」のみ）だけ。emo が入力を解決し kanade:`input-events` が SHIORI へ配信。
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
- **2026-07-01 追記・着手可能フロント（brief 済み・未着手）**: `/kiro-discovery` で「安全並走バッチ」の brief を just-in-time 生成。① wintf 基盤層 `wintf-dcomp-to-wuc-migration`（表示バックエンド WUC 移行）／`wintf-clickthrough-alpha-toggle`（既存 brief）。② M1 parser 並走 `areka-P0-shell-parse`・`areka-P0-balloon-parse`・`areka-P0-package-mount`（`areka-parsers` へ `shell`/`balloon`/`package` モジュール追加・host 不要・単体テスト可）。③ M1 host-32 `areka-P0-host32-ipc`（pilot go 済で解禁・bytes-over-wire transport・別プロセスゆえ非衝突）→ **✅ 2026-07-02 完了・アーカイブ（`completed/areka-P0-host32-ipc` → `completed/areka-P0-host32-shiori-load`）。下流 `areka-P0-host32-request` が次フロント**。これら 5〜6 本は相互非衝突で即並走可（`ecs/graphics` 系は wuc-migration に一本化）。`wintf-ulw-removal` は clickthrough 完了待ち（brief 済み・ゲート下）。
- `completed/` = 106（歴史・M1 が立つ土台の記録。2026-07-01 `pilot-clickthrough-alpha-toggle`・`pilot-shiori-host-32` を go 済みでアーカイブ／2026-07-02 `areka-P0-host32-ipc`・`areka-P0-parser-foundation`・`areka-P0-host32-shiori-load` を実装完了でアーカイブ）。
- 旧 active/brief（M1 憶測・M2 reference・出荷層）・backlog（P1-P3）・`_rejected/`・旧戦略メモは**削除**（git 履歴に保全。必要時に復元可）。

## M2 以降

**M1 完成後に、実物を見て組み直す。** 本ロードマップでは扱わない（pasta の native x64・`IShiori` in-proc 化、縦書き・ベクトル描画・AI、**owner-draw 右クリック system メニュー（ゴースト管理 chrome）**、互換面拡大＝Shift_JIS/SAORI/里々・YAYA 網羅/NAR 等はその時に）。

---

## wintf 基盤先進坑: クリック透過 αトグル方式（M1 外・wintf 基盤層）

> **位置づけ**: 本トラックは M1（emo2-boot）とは別軸の **wintf 基盤層**の改善。M1 ユニット群（⓪〜⑥）には含めない。ここに記すのは two-tunnel.md（line 87）が `_Depends(confirmed):` ゲートの宿主を roadmap.md と定めているため。

### 動機（既存「ULW 一択」結論の穴）

[tech.md](tech.md) / 本ファイル line 30 はかつて別プロセス透過を **「実質 ULW 一択」** と断定し ULW/DComp 切替式を「実装完了済み」と記録していた（**2026-07-01 撤回済み**: 下記 pilot go を受け tech.md「Key Technical Decisions」／product.md／structure.md を新方針へ是正・steering 同期完了）。旧結論は **HTTRANSPARENT・SetWindowRgn・ULW** の 3 択比較で、**`WS_EX_TRANSPARENT` 動的トグル方式（winit `set_cursor_hittest` 相当・プロセス境界を越える第 4 の手）を検討していなかった**。

**真の動機は「DComp 描画を捨てられない」こと（開発者）。** ULW は CPU ビットマップ方式で **DComp スワップチェーン合成と併用不可**（記憶 areka-transparency-requires-layered-window）。すなわち 3D（DComp/GPU 合成）ウィンドウにとって ULW はそもそも選択肢になり得ず、ULW は別プロセス透過のために 3D 描画を諦める踏み絵になっている。`WS_EX_TRANSPARENT` 動的トグル（別スレッドのカーソル監視＋αマスク問い合わせで透明領域のみ透過・CPU 転送なし）は **DComp 描画を維持したまま別プロセス透過を成立させる事実上唯一の現実解**。**他社 3D マスコットが採用している実証済み手段**でもある（ただし十分な検証・エンバグ対応を要する前提）。既存「ULW 一択」結論と矛盾する新方向ゆえ **先進坑で先に潰す**（二坑モデル教義）。

### pilot（先進坑・使い捨て・✅ **go 済み** 2026-07-01）: `pilot-clickthrough-alpha-toggle`

- 配置: `crates/pilot/examples/pilot-clickthrough-alpha-toggle/`（README 3 幕＋ REPORT.md が一次記録）。spec は `.kiro/specs/completed/pilot-clickthrough-alpha-toggle/` へアーカイブ済み。
- 検証: 透過トップモスト窓＋中央不透明領域、16ms 周期ワーカ（`event_listener`＋`std::thread`・tokio 禁止）がカーソル位置→αマスク関数（仮・円判定）を問い合わせ、状態変化時のみ `SetWindowLongPtr(GWL_EXSTYLE)`＋`SetWindowPos(SWP_FRAMECHANGED)` で `WS_EX_TRANSPARENT` を付け外し。DPI per-monitor-v2。
- **go 基準**（人間判断）: 試験項目 T1〜T8 のうち **T1・T2・T3・T4・T6 が ✅ 必須**、T5・T7・T8 は ✅ または軽微な条件付き合格（理由明記）。レポートは合否問わず作成し依頼者の判断を仰ぐ（AI 単独で go 判定しない）。
- **✅ go 結果（2026-07-01・開発者承認）**: 核心 Unknown 肯定的決着＝**DComp 描画を捨てず `WS_EX_TRANSPARENT` 動的トグルで別プロセスクリック透過が成立**。必須配合（当初想定に無かった）: ① **`WS_EX_LAYERED` を"同伴フラグ"として立てる**（ULW/SLWA 非呼出＝レイヤード描画には使わない。無いと透過が効かない）／② **枠なし窓 `WS_POPUP`（client==window）**。最重要原理: **表示層（DComp visual・content は surface でも合成 swapchain でも可＝3D/Live2D 拡張可）と当たり判定層（HWND スタイル）は独立**。ドラッグ移動の罠（ドラッグ中は位置に関わらず `WS_EX_TRANSPARENT` を外したまま維持）も知見化。詳細は REPORT.md。**⇒ 当初の「`WS_EX_LAYERED` 不使用」前提は撤回**（本坑へ申し送り）。

### main（本坑）: `wintf-clickthrough-alpha-toggle`

```
_Depends(confirmed): pilot-clickthrough-alpha-toggle
```

- ~~pilot の go 判定が出るまで **BLOCKED**~~ → **✅ go 済み（2026-07-01）＝着手可**。pilot 知見はクリーンに掘り直す（コピペ donor 禁止・README/REPORT 検証結果を参照）。**申し送り必須**: ex_style = `WS_EX_NOREDIRECTIONBITMAP|WS_EX_TOPMOST|WS_EX_LAYERED|WS_EX_TRANSPARENT`（LAYERED はフラグのみ・TRANSPARENT のみ動的トグル）／枠なし窓／表示=DComp・当たり判定=HWND スタイルの二層分離／ドラッグ中は透過 OFF 維持。
- 本体 wintf へ `WS_EX_TRANSPARENT` 動的トグルを導入し、本体αマスク（実描画αバッファ／`AlphaMask::is_hit`）参照でキャラ領域のみクリック可にする。
- **ULW との共存方針（開発者決定）**: 至上要件は **DComp 描画の維持**。本方式は DComp 経路に透過能力を授けるもの。本仕様が完全に有効と判断されれば **ULW ルートは破棄**。ただし他社実績ある手段とはいえ**十分な検証期間・エンバグ対応**を置き、**当面は ULW と並走**させる。tech.md/product.md/structure.md の「ULW 一択」記述は **2026-07-01 に撤回・新方針へ更新済み**（pilot go を受けた steering 同期）。
- 接続先候補（調査済み）: `CompositionMode`（`ecs/window/components.rs`・生成時固定）／`compute_ex_style()`（`runtime/window_factory.rs`）／`HitTestMode::AlphaMask`・`AlphaMask::is_hit`（`ecs/layout/hit_test/`・`ecs/widget/bitmap_source/`）／`VsyncEventBridge`（`event_listener`・`runtime/tick_bridge.rs`）／D2D1 staging αバッファ（`ecs/graphics/compositor.rs`）。

### 依存マップ検証（two-tunnel 手動チェックリスト）

- 被覆: 不確実な本坑 `wintf-clickthrough-alpha-toggle` は go ゲート pilot を持つ ✓
- 孤児なし: pilot は対応本坑を名指し、本坑は pilot を `_Depends(confirmed):` で参照 ✓
- 循環なし／DAG: pilot → main の単一エッジ（巡回なし）✓
- 合否基準明示: go 基準（T1・T2・T3・T4・T6 必須）を上記に明示 ✓

---

## wintf 基盤層: 表示合成バックエンドの WUC 移行（M1 外・wintf 基盤層）

> **位置づけ**: 上記クリックスルーと同じく M1（emo2-boot）とは別軸の **wintf 基盤層**改善。表示バックエンドを **DirectComposition → Windows.UI.Composition（WUC / `Compositor`・`DesktopWindowTarget`）** へ寄せる。M1 ユニット群（⓪〜⑥）には含めない。

### 動機と前提

- 表示合成の依存を DComp から WUC へ移し、DirectComposition 依存を廃す。**純粋等価移行**（見た目・再描画を変えない）が要件。WUC 新能力（アニメ/エフェクト）活用は M2 以降。
- **調査（2026-07-01・`/kiro-discovery`）で GO-with-caveats 確定**: `windows` 0.62.2 に WUC 全型＋interop trait（`ICompositorDesktopInterop` 等 5 種）が存在。耐力壁級 Unknown 無し＝**pilot は切らない**（本坑一本）。caveats: ① `Compositor` 生成前に UI スレッドで `CreateDispatcherQueueController(DQTYPE_THREAD_CURRENT)`（**既存 message pump に相乗り・差し替え無し**）／② `Commit()` 廃止（暗黙反映）／③ サーフェス→`SpriteVisual.Brush` に brush が一段挟まる。`WS_EX_NOREDIRECTIONBITMAP` はそのまま流用可。

### spec 分割（2 本・ULW 除去は独立）

- **✅ 完了 `wintf-dcomp-to-wuc-migration`（本坑・2026-07-02 完了）**: **①DComp→WUC 差し替えのみ**。当たり判定・ULW とは独立。ULW アーム（`compositor.rs`/`ulw.rs`/`compositor_systems`）と `CompositionMode` enum には手を入れず完了。実装後: WUC を触る graphics schedule は UI スレッド固定（DispatcherQueue 親和性）・`CompositionMode` 既定は ULW のまま。spec: `.kiro/specs/completed/wintf-dcomp-to-wuc-migration/`。
- **`wintf-ulw-removal`（本坑・brief 済み）**: ULW 一式除去＋`CompositionMode` collapse（GPU 合成単独へ）。brief: `.kiro/specs/wintf-ulw-removal/brief.md`。
  ```
  _Depends: wintf-clickthrough-alpha-toggle（完了）
  ```
  **ULW を安全に消せるのは、本坑クリックスルーが完了して「ULW 無しでも別プロセス透過が成立」と確認できてから**（クリックスルー brief の並走方針＝「完全有効なら ULW 破棄／当面は並走・即時撤去しない」に一致）。`wintf-dcomp-to-wuc-migration` とは触るファイルが別ゆえ**独立**（順序任意・両完了後に `CompositionMode` は WUC 単独へ最終 collapse）。クリックスルーのα源は ULW compositor の staging αバッファではなく per-widget `AlphaMask` を使う想定（design で確認）。

### 依存マップ検証（two-tunnel 手動チェックリスト）

- 順序ゲート: `wintf-ulw-removal` は `wintf-clickthrough-alpha-toggle` 完了が前提（ULW 破棄の安全網）✓
- 独立性: `wintf-dcomp-to-wuc-migration`（表示層・**✅2026-07-02 完了**）は `wintf-clickthrough-alpha-toggle`（当たり判定層）とも `wintf-ulw-removal`（ULW 側・別ファイル群）とも独立・順序任意 ✓
- 循環なし／DAG: clickthrough → ulw-removal の単一エッジのみ（wuc-migration は独立ノード・巡回なし）✓
