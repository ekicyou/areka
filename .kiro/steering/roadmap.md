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

- 透過（別プロセス自動透過）＝GPU 合成（WUC）＋クリックスルー機構（`WS_EX_TRANSPARENT` 動的トグル・**ULW は 2026-07-05 `wintf-ulw-removal` で撤去済み**）、event/hit-test/alpha-mask、dola 演出ランタイム（コア〜ループ/nested）。
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
- **通信基盤の統一設計（2026-07-03 確定・責務三分）**: **機構**（envelope・spawn/join・停止・**UI 配送ブリッジ**）＝横断基盤ユニット **`areka-P0-actor-foundation`**（⓪ ghost 帰属・parser-foundation の並行版・brief 済）／**経路**（実行時の全体調整＝運行表）＝**kanade**（基盤の最大消費者・ただし所有者ではない——kanade 非経由の seriko→emo 等も基盤に載る）／**結線**（スレッド起動・channel 接続・終了）＝**ghost**（ghost-setup）。各エンジン仕様は自前の channel 流儀を発明せず actor-foundation の規約に載ること（brief の通信注記が正本）。

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

**未着手ユニット名の固有名整合（2026-07-02 実施）**: `areka-P0-conductor`→`areka-P0-kanade`／`areka-P0-shell-anim-engine`→`areka-P0-seriko-engine`／`areka-P0-shell-anim-loop`→`areka-P0-seriko-loop`／`areka-P0-surface-engine`→`areka-P0-emo-surface`（**2026-07-03 さらに直列3分割**: `emo-atlas`→`emo-compose`→`emo-present`）／`areka-P0-text-layer`→`areka-P0-emo-text-layer`。**不変**: `areka-P0-host32-*`（shiori トラック実装進行中＝改名衝突回避・名も既に整合）・`areka-P0-sakura-engine`・`areka-P0-ghost-setup` 等 ghost 系（既に固有名基準）・completed 仕様（歴史）。既知ドリフト: [balloon/model.rs:6](../../crates/areka-parsers/src/balloon/model.rs) doc コメントが旧名 `text-layer`/`surface-engine` を参照（該当ユニット着手時に追随修正）。

## M1 実装ユニット（実現可能な粒度）

> **粒度基準**: 1ユニット＝「コードを走らせて観測できる**単一 pass/fail** を持ち、それを観測するのに別ユニットを先に作る必要がない」もの。done が複数の独立観測に割れるなら粗すぎ→分割。
> **観測の独立化（2026-07-03 明文化）**: 制御階層ユニット（kanade/sakura/seriko/emo）の単体観測は**実上流を待たず fixture/mock 入力で切る**（例: sakura＝script 文字列直入力・emo-compose＝parsed Shell モデル直入力・オフスクリーン pixel テストで観測）。実上流との結線は M-boot 統合（emo2-boot）で観測。これが「別ユニットを先に作る必要がない」の含意＝トラック間の並走はこの規約で担保される（トラック内は逐次）。
> **適用境界＝本番ゴースト先行の原則（2026-07-05 追記・window-placement リジェクトの教訓）**: 上記規約は**純粋層**（parser／codec／合成）にのみ適用。**UI 位置決め・座標系ユニット（window-placement・collision-geometry 等）は逆**——本番ゴースト（emo2 実 surface 表示）＋**実 DPI（≠96）実行**が観測条件であり、単発デモ（ハードコード窓・架空 work_area）への合わせ込みは**無効**（dpi=96 の自己整合が欠陥を隠すことが実証済み。記憶 areka-placement-real-ghost-first／areka-window-placement-dpi-coordinate-defect）。
> 正規名は**暫定**（着手時に確定）。**spec 工場にしない**＝下記はユニット名の登録であり brief.md 群ではない。着手時に最小 spec/task を just-in-time で切る。
> **粒度の真実**: 作業は **M-boot に前倒し集中**（約20ユニット＝M1 の山・2026-07-03 emo 3分割＋actor-foundation・07-05 app-shell 追加で 16→20）。「最初の起動」が本体で以降は薄い増分。

### M-boot ＝ `areka-P0-emo2-boot`（最初の可視結果・最重量＝約20ユニット）
emo2 が起動して喋る。下記 5 トラックを結線して達成（⓪ ゴーストエンジンが全体を統括）。

**⓪ ghost＝ゴーストエンジン（最上位 owner・全エンジンを統括）**
- `areka-P0-actor-foundation` — **エンジン間通信の横断基盤**（parser-foundation の並行版）: envelope（メッセージ enum・返信 Sender 同梱）・spawn/join・停止手順（Close→drain→join）・**UI スレッド配送ブリッジ**（pump 統合）。`std::sync::mpsc` 起点＝依存ゼロ。kanade の先行依存・現行フロントと並走安全（新設モジュール＝非衝突）。✔ toy アクター試験（worker⇄worker＋worker→UI pump echo）。**✅ 完了（2026-07-04・spec=`completed/areka-P0-actor-foundation`）**: 新設クレート `crates/areka-actor` に「規約（lib.rs rustdoc 5 規約）＋薄いヘルパ（`spawn_actor`/`run_inbox`/`reply_channel`）＋UI 配送ブリッジ（`spawn_ui`/`UiSender`＝async-channel unbounded＋`wintf-winmsg-executor` pump 内 drain）」の 3 点を確立（公開面 12 シンボル・`pub trait` 0・過剰抽象なし・既存クレート改修ゼロ）。停止規約は Close＝即時停止（積み残し破棄）・全 Sender drop 正常終了・handler Err は記録して継続（終了経路 2 本のみ）・panic は join で観測。UI ブリッジは三点組合せ（`spawn_local`＋`MessageLoop::run`＋`PostThreadMessageW` heartbeat）を **PRIMARY 方式**で実証（off-thread 検出は executor 0.0.5 に probe 不在ゆえ log-first の case (c)）。toy(a) worker⇄worker＋toy(b) worker→UI pump 実走 echo が bounded で green（23 テスト・新規依存は承認済 `async-channel` のみ＝ツリー内既在）。下流 kanade/ghost-setup/emo-present の envelope 先行依存を充足
- `areka-P0-app-shell` — **アプリ骨格＋デモ保全**（アプリ組み上げ三段の第一段・早期・小粒）: 現 main.rs のモック UI を `examples/mock-shell.rs` へ**別名保全**（挙動不変・shiori 系モジュールは src 残留）＋本番 main 骨格（tracing/panic/WinApp/構成入力＝ghost・balloon path 解決〔引数 or 既定 fixture・ukadoc 上ハードコード正当〕）＋ghost-setup 差し込み口（空）。**効果: emo-present⇄window-placement の main.rs 衝突が構造ごと解消**。✔ example が従来挙動＋骨格 main が構成解決→正常終了＋shiori e2e green（**brief 済 2026-07-05**）
- `areka-P0-ghost-setup` — ゴースト lifecycle（**アプリ組み上げ三段の第二段**・app-shell 骨格の差し込み口に実装）: package-mount で構築→**エンジン結線**（actor-foundation の結線層＝スレッド起動/channel 接続/join を所有）→kanade へ boot 指示→close 握手（kanade の OnClose 運行の完了）を待って全エンジンを落とす。✔ descript.txt 起点のマウントから起動〜終了を統括
- `areka-P0-window-placement` — サーフェス窓の生成＋既定位置＋ドラッグ（既定位置＝ukadoc `seriko.alignmenttodesktop` カスケード準拠・窓数は構成入力・二人立ちの本格結線は M-dual）。**⚠️ 順序ゲート: `areka-P0-emo-present` 完了後**（本番ゴースト実表示に対して実装・検証する。2026-07-05 に demo 前提の着手が実 DPI 座標破綻＝論理/物理混在でワークツリーごとリジェクト→brief 改稿済み・wintf 座標契約の確定を design 必須先行に）。✔ 本番 emo2 表示＋**実 DPI（≠96）**で既定位置・ドラッグ・バルーン追従が正しい（dpi=96 のみの緑は不合格）（**brief 済・07-05 改稿**）

**① shiori＝SHIORI 通信層エンジン host-32（耐力壁・`pilot/shiori-host-32` がトラックを gate）**
- `pilot/shiori-host-32` — 使い捨て feasibility。**✅ 完了（2026-07-01・spec=`completed/pilot-shiori-host-32`・コードは `crates/pilot/examples/shiori-host-32/` に隔離保全）**: go 基準(1)(2) 実走充足＝32bit pasta.dll 1往復（x64 親が emo2 OnBoot `Value` 受領）＋窓持ちループ N秒生存→clean unload。跨ビットネス再入 WM_COPYDATA・`wintf-winmsg-executor` i686 実行時とも GO（fallback 不要）。→ 下流 `areka-P0-host32-*` の go ゲート充足（着手可・最終 go 判定は開発者）
- `areka-P0-host32-ipc` — x64↔32bit helper＋handshake。✔ 往復 echo。**✅ 完了（2026-07-02・spec=`completed/areka-P0-host32-ipc`）**: bytes-over-wire transport を3クレート（`shiori-host32-ipc`=proto / `-host`=x64+arm64 / `-helper`=i686）で本坑再掘（pilot 非コピペ）。トランスポートは WM_COPYDATA 一本化＋再入 RESPONSE（named pipe 不要）。実 i686 helper 越しの往復 echo が無デッドロック・無クラッシュで green（M1 ゲート指標充足）。pasta ロード/SHIORI parse/常駐 lifecycle は下流 `areka-P0-host32-*` の領分（本ユニットは seam のみ所有）
- `areka-P0-host32-shiori-load` — LoadLibrary pasta.dll＋load/unload/request 解決＋load(ghostdir)。✔ load 成功・無crash。**✅ 完了（2026-07-02・spec=`completed/areka-P0-host32-shiori-load`）**: 二層一体で **load_dir per-instance 貫通（D1）＋teardown を Drop(RAII) 全層一貫（D7）** を実現。WS-A=helper が実 i686 SHIORI DLL を `LoadLibraryW`→3 エクスポート解決→`load(load_dir)` 駆動し凍結 wire 上 1 byte ack で成否返送（トラック所有 testdll fixture で成功/失敗/無crash E2E・本物 pasta は env gate）。WS-B=`shiori-abi` を **IShioriFactory 融合 create＋Get/Notify 分離＋GetProperty/SetProperty＋型付き COM 引数＋module entry `shiori_factory`** へ是正（`load_dir` 欠落の根幹欠陥を根絶・reference/mock backend で証明）。凍結境界 `shiori-host32-ipc` 不改変。→ 下流 `areka-P0-host32-request`（request 呼出・SHIORI/3.0 marshal）が次フロント
- `areka-P0-host32-request` — SHIORI/3.0 build＋marshal＋response Value＋charset。✔ x64 が emo2 OnBoot の Value 受領。**✅ 完了（2026-07-05・spec=`completed/areka-P0-host32-request`）**: x64 純 codec（`build_request`/`parse_response`＝汎用 ID・CRLF/空行終端・status 200/204/311/312/400/500 寛容・未知ヘッダ tolerate・Charset 省略時継承）＋出口 API `Shiori3Client`（GET/NOTIFY 単一 request 経路合流・NOTIFY も同期往復で応答破棄・`RequestError` 区別語彙〔wire timeout／SHIORI エラー／helper 死活／handshake〕・`AREKA_SHIORI_REQUEST_TIMEOUT_MS` env seam〔0=無限=debug opt-in〕）＋helper i686 `ShioriByteProxy::request` 実呼出（HGLOBAL 非対称契約＝入力 callee-free／応答 caller-free・二重解放なし）＋Reply アーム echo→proxy 駆動置換（RefCell 再入規律）＋testdll fixture 拡張（固定 200+Value／204）＋決定的 E2E（実 i686 helper 越し GET→Value 抽出／NOTIFY→204 破棄）＋env-gated 実 pasta OnBoot 追験。凍結境界 `shiori-host32-ipc` 不改変・`crates/pilot` 非依存・i686 PowerShell ビルド/テスト green（x64+i686 計 101 テスト）。→ 下流 `areka-P0-host32-lifecycle`（常駐 msg loop＋crash 監視）が次フロント
- `areka-P0-host32-lifecycle` — helper msg loop＋OnSecondChange poll＋unload＋crash監視。✔ N秒運転→clean unload

**② parsers（単体テスト可・host 不要。foundation が先行依存＝完全並行ではない）**
- `areka-P0-sakura-parse` — emo2 タグ subset→token。✔ boot script を token 化
- `areka-P0-parser-foundation` — **パーサー共通基盤**: charset デコード（冒頭 `charset` 行→encoding_rs 再デコード・全パーサー共通）＋ KV 読み込み（素朴マップ・surface 以外の全パーサー共通）。✔ charset 付き入力→KV マップ化（旧 `areka-P0-balloon-parse` を 2026-07-02 開発リジェクト→リネーム。知見は同 brief に集約）。**✅ 完了（2026-07-02・spec=`completed/areka-P0-parser-foundation`）**: `charset::decode`（BOM 読飛→冒頭ASCIIプリスキャン→宣言/既定 encoding_rs デコード）＋ `kv::parse_kv`（素朴 BTreeMap・後勝ち・trim）を `areka-parsers` に確立（encoding_rs 0.8 承認済追加・144 テスト緑）。下流 `shell-parse ∥ balloon-parse ∥ package-mount` の foundation 依存を充足
- `areka-P0-shell-parse` — surfaces.txt/descript→surface モデル（foundation 依存）。✔ emo2 shell parse。**✅ 完了（2026-07-02・spec=`completed/areka-P0-shell-parse`）**: `areka_parsers::shell` を四層（model←lexer←decode←parse）で確立。SERIKO/2.0 subset（overlay・interval 3種 bind/random/bind+random・矩形 collision・animationN 集約〔疎 pattern・負センチネル i64〕・surface.append ターゲット記述子〔範囲は非展開〕・kero.surface.alias 不透明キー写像〔重複保持〕）を寛容パース（Result なし・非パニック・subset 外は passthrough 吸収）。公開 facade `parse(&str)->Shell` へ 13 モデル型を集約。ukadoc 準拠自前テスト（主軸）＋emo2 fixture スモークで検証（225 テスト緑・clippy クリーン・追加依存なし）
- `areka-P0-balloon-parse` — balloon 3段参照優先度（sXXs/kXXs 起点＞descript＞既定）解決→モデル（foundation 依存・着手時に再切り出し）。✔ emo2-kakukaku parse。**✅ 完了（2026-07-02・spec=`completed/areka-P0-balloon-parse`）**: `areka_parsers::balloon`（`model`＝幾何＋フォント subset 型群・`Option` 直持ちで未指定＝`None` を `Some(0)` と区別・`#[non_exhaustive]`／`parse`＝descript＋画像別の後勝ち2層マージ→exact-key 写像・符号保持・寛容 None 降格）を確立。emo2-kakukaku 実物 fixture（R5.1/5.2/5.3）を charset→parse_str で適合検証、distractor キー非漏洩も固定（172 テスト緑・新規依存ゼロ）
- `areka-P0-package-mount` — `ghost/master/descript.txt`＋dir→mount（foundation 依存・起点は descript.txt。`install.txt`＝NAR 配置マニフェストは起動時不使用ゆえスコープ外）。✔ emo2 layout 解決。**✅ 完了（2026-07-02・spec=`completed/areka-P0-package-mount`）**: descript.txt 起点で SHIORI（dir=`ghost/master`・file は `Option` 推測禁止）＋shell（既定 `master` フォールバック・物理存在確認）の2点マウントを解決する `package` module を `areka-parsers` に確立。所在ベース識別（`type` 分岐なし）・foundation（`charset::decode`/`kv::parse_kv`）委譲・致命失敗3種（起点不在/読取不能/shell 不在）を `MountError` で観測可能化・emo2 実 fixture 統合テスト green（164 テスト・回帰なし・clippy clean）。下流 `ghost-setup`/`host-32`/`shell-parse` へ `MountModel` を供給

**runtime 制御階層 kanade／sakura／seriko（上→下に駆動・両 anim engine は dola 上）**
- `areka-P0-kanade` — **kanade（③conductor）**: SHIORI イベント循環（OnSecondChange pump・host-32 送受・Value を sakura-engine へ）。**actor-foundation 先行依存**（kanade＝基盤の最大消費者・実行時経路＝運行表の所有者）。**boot/close の発火順序も kanade の運行表**（ukadoc 正典・2026-07-05 調査済み＝app-shell brief に転記元あり）: boot＝OnInitialize NOTIFY→OnFirstBoot（Ref0=vanish count）/OnGhostChanged/OnGhostCalled/OnVanished の 204 フォールスルー→OnBoot（Ref0=shell 名）→basewareversion NOTIFY／close＝OnClose→応答スクリプト**再生完了待ち**（`\-`）→204 なら OnCloseAll→終了（タイムアウトは de-facto＝design で確定）。M-boot は毎回 OnBoot で開始可（vanish count 永続化は position-persist）。✔ OnBoot→Value 受領→再生開始
- `areka-P0-sakura-engine` — **sakura（④）＝さくらスクリプト再生エンジン**（talk timeline: `\w/\_w` wait・`\s` で seriko へ surface 指令・text を emo（text-layer）へ・seq）。✔ boot script を時系列再生
- `areka-P0-seriko-engine` — **seriko（⑤）＝シェルアニメーションエンジン**（SERIKO ループ＋surface 状態＋MAYUNA bind・render を毎フレーム駆動）。M-boot は静的＋指令適用、ループ(blink)は M-life。✔ 指令された surface を表示

**emo（⑥）＝render engine（統一・`areka-mock-shell`＋dola から増分）**

> **合成方式（2026-07-03 開発者決定・記憶 areka-emo-own-compositor-atlas）**: 合成は **emo 自前コンポーネント（wintf Visual 合成非依存）**＝element 画像をアトラスへ正規化貼付→base＋element を layer 順に **1枚物ビットマップへ自前合成**→wintf へは完成品1枚のみ供給。論拠: DComp/WUC の visual ブレンドは実質 SourceOver 系のみで SERIKO 合成メソッド群をピクセル正確に写像不能。
> **粒度分割（2026-07-03・旧 `areka-P0-emo-surface` を直列3ユニットへ分割）**: 旧ユニットはアトラス基盤・合成コア・表示結線を一身に抱え粗すぎ（単一 pass/fail 不成立）。以下の直列チェーンで完走する（各ユニットが独立観測を持つ・トラック内逐次）。
> **クロスユニット契約（自律開発継続性・2026-07-03 fixture 実測で確定）**: 直列チェーンの「手前が考えないと後続が詰む」要素は各 brief の**「クロスユニット契約」節が正本**——①`AtlasEntry`/頁バッファ型＋マニフェスト導出＝emo-atlas 所有 ②**合成入力＝surface id＋bind 有効集合**（surface1000 全 bind 対策）＋正規化ツリー公開形（collisions/animations 保持）＋`ComposedSurface` 出力型＝emo-compose ③text-layer スロット予約＋bind 初期解決＋Window entity 受取口＋ulw-removal API 変動調整＝emo-present。設計/実装セッションは着手前に該当 brief の同節を読み、**契約型は上流 brief の正本を消費**（再定義しない）。

- `completed/areka-P0-emo-atlas` ✅**完了（2026-07-05）** — **アトラス基盤**（直列1）: 画像デコード→透過正規化（キーカラー/`.pna`/`use_self_alpha`→premultiplied BGRA・挿入時一度だけ）→**αトリミング**（α=0 領域を除外した有効矩形＋オフセット記録・例 100×100 中有効 10×10 なら 10×10＋offset で焼付）→packing（クレート利用・padding 1〜2px・複数頁・重複排除）→UV/オフセット表。✔ emo2 element 画像群が焼付され、トリム矩形・オフセット・正規化画素が単体テストで一致（表示不要・純粋層）
- `areka-P0-emo-compose` — **合成コア**（直列2・atlas 依存）: Shell モデル→実サーフェスツリー構築（疎 id・appends・aliases・範囲の展開＝parser 転記層の下流。**成果物は collisions/animations を保持した公開正規化形**＝seriko/collision-geometry が同じ結果を消費）→合成プラン（layer 順・変換行列・合成メソッド）→**アトラス転写で1枚物ビットマップ合成**。**合成入力＝surface id＋bind 有効集合**（emo2 side0 本体 surface1000 は静的 element ゼロ・全パーツ bind ゆえ、有効 bind の pattern0 を animation ID 昇順で静的合成——これが無いと M-boot でむらさきが空白）。入れ子 surface 参照の再帰＋循環検出。合成メソッドは emo2 使用分のみ＝実測 overlay（写像表は全量）。✔ emo2 surface0（＋bind 集合を与えた surface1000）の合成結果がオフスクリーン pixel テストで一致（表示不要）
- `areka-P0-emo-present` — **表示結線**（直列3・compose 依存）: 合成済み1枚→wintf 表示口（窓あたり visual 最小限）＋**AlphaMask を合成結果から生成**（clickthrough 直結）＋surface 指令 API（id 切替）＋合成キャッシュ（無効化規則）。バルーン枠（`balloons*.png`・fixture 直指定）も同一機構で表示。✔ 専用 example で surface0＋バルーン枠が表示・キャラ領域のみクリック可
- `areka-P0-emo-text-layer` — バルーン文字を **engine 上に被せる層**（token→glyph→surface 領域・emo-present 依存）。**縦書き/横書き両対応**（wintf 縦書き資産を lift・emo2 使用方向を design で確認）・**描画先＝行列変換領域を内部表現**（回転領域書込みの構造・M1 実挙動は恒等/平行移動のみ・文字装飾は型シームのみ→M2）。✔ script がバルーンに描画＋scroll

### 増分（M-boot 後・**エンジン別＝並走可能**）

> 増分はエンジンへ帰属させる。**別エンジンに属する増分は並列着手可**（spanning する旧 unit はエンジン単位に分割済）。マイルストーン（M-dual 等）はエンジン横断の**統合点**であって作業単位ではない。
> **トラック全7**（括弧内が固有名・「エンジン固有名」節が正本）: ⓪ゴーストエンジン(owner・`ghost`)・①SHIORI 通信層(host-32・`shiori`)・②parser/loader(`parsers`)・③conductor(`kanade`)・④sakura-engine(`sakura`)・⑤shell-anim-engine(`seriko`)・⑥render-engine(`emo`)。⓪は最上位 owner（lifecycle/窓配置/位置永続化）。**①SHIORI 通信層・②parser/loader・⓪の大半は M-boot で完了**。増分を持つのは ③〜⑥ ＋ ⓪の位置永続化。

- **⑤ seriko（shell-anim-engine）**: `areka-P0-dual-surface`（side0/1＋surface alias）／ `areka-P0-mayuna-compose`（**bind 状態の動的管理**＝bindgroup 切替・着せ替えメニュー連動。**bind の静的合成適用は emo-compose が M-boot で所有済み**＝境界 2026-07-03 明確化）／ `areka-P0-seriko-loop`（SERIKO ループ＝blink random/bind+random）
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

**アプリ組み上げの所有マップ（2026-07-05 確定・「誰がアプリとしての areka を組むか」）**:
- **三段構え**: ① `app-shell`（骨格＝main.rs 所有・構成入力・デモ保全・早期）→ ② `ghost-setup`（骨格の差し込み口にエンジン結線・boot 指示・close 握手待ち・エンジン群完了後）→ ③ `emo2-conformance-e2e`（完成アプリでの一周適合＝M1 ゴール証明）。
- **boot/close のイベント発火順序＝kanade の運行表**（上記 kanade 行・app は器に徹する）／**永続化（vanish count・窓位置）＝position-persist**（M-life）。
- **M2 送り（ukadoc 裏付け済み・全て任意）**: SSTP ポート（9801）・FMO・DirectSSTP・Plugin/HEADLINE/SAORI ホスティング・ネットワーク更新（OnBasewareUpdating 系）・ゴースト/バルーン選択 UI。M1 骨格はこれらの口を持たない。

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

### emo の責務範囲（UI 層宣言・2026-07-03 明文化） ＆ メニュー方針

> **⑥ emo＝ゴーストの UI 層全般**を所有する（描画だけの engine ではない）: ① surface 合成（emo-atlas/-compose/-present）② **マウス/さわり反応**（下記）③ **バルーンテキスト表示**（emo-text-layer）④ 選択肢表示（choice-render）。「見える・触れる」はすべて emo が窓口＝kanade へは解決済みイベント（region/actor 等）だけを渡す。

- **総合的なマウス制御・さわり反応＝⑥emo の責務**（独立仕様は作らない）。窓のマウスメッセージ・**alpha hit-test**・ドラッグは**完了済み基盤**（`event-mouse-basic`/`event-drag-system`/`event-hit-test`/`event-hit-test-alpha-mask`）の上に emo が所有。M1 新規は `collision-geometry`（hit→ゴースト collision region/actor 写像＝「範囲」のみ）だけ。emo が入力を解決し kanade:`input-events` が SHIORI へ配信（撫で＝OnMouseMove 連打の解釈は SHIORI 側の領分）。
- **テキスト表示の進化路線（emo-text-layer 起点・M1→M2 追跡）**: M1＝縦書き/横書き両対応（wintf 縦書き資産 `vertical-text-layout`/Typewriter を lift・emo2 使用方向を design で確認）＋**描画先は「矩形」でなく行列変換領域を内部表現**（surface 合成の行列原則と同型・回転領域への文字書込みを構造として持つ。M1 実挙動は恒等/平行移動のみ）。M2＝回転テキストの実挙動解禁・**ポップアート級の文字装飾**（アウトライン/多色/シャドウ/変形等の text effects）——1枚物合成方式ゆえ文字も合成パスの1レイヤ＝装飾の自由度は自前合成が担保する（この拡張性が自前合成採用の追加論拠）。
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

- Rust 2024・マルチクレート（wintf/dola/areka ＋ `areka-parsers` ＋最小依存 `shiori-abi` ＋ host-32 3クレート `shiori-host32-ipc`/`-host`/`-helper`）。
- **32bit 可搬性の適用範囲＝host-32 系（`shiori-host32-*`／`shiori-abi`）のみ**。wintf/areka 本体は x64＋arm64 ネイティブ（i686 検証を本体 spec に課さない）。
- 透過は WUC/DComp GPU 合成上のクリックスルー機構（`WS_EX_TRANSPARENT` 動的トグル＋`WS_EX_LAYERED` 同伴フラグ＋αマスク `AlphaMask::is_hit`）で成立（**ULW は 2026-07-05 `wintf-ulw-removal` で撤去済み**・旧「ULW/DComp 切替式」記述は失効）。SHIORI 内部唯一 ABI=`IShiori`(COM, HSTRING/UTF-16)。過去互換は 32bit Rust ホスト（flat-C/HGLOBAL/charset/自前 IPC）。
- 設計判断の変更は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md) を正本として更新。

## ポートフォリオ（2026-06-28・clean slate）

- `.kiro/specs/` 直下 active = **0**（憶測仕様を全伐採し更地化。実装ファーストで着手時に作る）。
- **2026-07-01 追記・着手可能フロント（当時 brief 済み）**: `/kiro-discovery` で「安全並走バッチ」の brief を just-in-time 生成。① wintf 基盤層 `wintf-dcomp-to-wuc-migration`（**✅ 完了**）／`wintf-clickthrough-alpha-toggle`（**✅ 2026-07-02 完了・アーカイブ**）。② M1 parser 並走 `shell`/`balloon`/`package`（**✅ 全完了**）。③ M1 host-32 `areka-P0-host32-ipc`（**✅ 完了**）→ `areka-P0-host32-shiori-load`（**✅ 完了**）。これら 5〜6 本は相互非衝突で即並走可（`ecs/graphics` 系は wuc-migration に一本化）。
- **2026-07-03 現況（2026-07-05 更新・マージ後の実地確認）**: `completed/` = **116**（`areka-P0-actor-foundation`・`wintf-ulw-removal`・`areka-P0-host32-request`・`areka-P0-emo-atlas` を完了・アーカイブ）。**active = 0**・**brief-only = 4**（**emo 直列3分割 `completed/areka-P0-emo-atlas`✅完了→`-emo-compose`→`-emo-present`（残2）**〔旧 emo-surface を粒度分割・自前合成＋αトリミングアトラス（packing＝`rectangle-pack`=0.4.2 承認・採用済）〕／**`areka-P0-window-placement`**〔alignmenttodesktop カスケード・窓数構成入力・**emo-present ゲート下**〕／**`areka-P0-app-shell`**〔新設=アプリ組み上げ三段の第一段・main.rs 骨格化＋モックデモを `examples/mock-shell.rs` へ別名保全〕）。**⓪actor-foundation 完了・wintf:`ulw-removal` は 2026-07-05 完了**（ULW 一式除去＋`CompositionMode` collapse・WUC 単独へ）・**①shiori:`host32-request` は 2026-07-05 完了**（凍結 IPC 不改変・helper echo→実呼出＋x64 Shiori3Codec・request 往復で Value 受領）・**⑥emo:`emo-atlas` は 2026-07-05 完了**（直列1・素材基盤層＝アトラス表＋頁バッファ）。**②parsers トラック全完了・⓪actor-foundation 完了・M-boot 約 10/20**（①shiori: pilot✅/ipc✅/shiori-load✅/request✅・lifecycle 残）。①shiori 次フロント `host32-lifecycle` は他と非衝突＝即並走可。
- **2026-07-05 追記（window-placement リジェクト→依存マップ再検討）**: window-placement の demo 前提着手（07-03 brief）が**実 DPI 座標破綻（論理/物理混在・dpi=96 でのみ自己整合）でワークツリーごとリジェクト**。全 brief を「雑なデモゴースト前提」観点で総点検した結果——**demo 前提は window-placement のみ**（emo-atlas/-compose＝純粋層・emo-present＝実 emo2 fixture 入力・host32-request＝実 pasta fixture・actor-foundation＝機構層・ulw-removal＝描画等価検証・app-shell＝デモ保全そのものが目的。いずれも実装完了時の demo 混入は無い）。**依存マップ是正**: ① window-placement は **emo-present の順序ゲート下**へ（本番ゴースト先行の原則・brief 改稿済み）② emo-present に**実 DPI 観測**＋座標契約文書化（下流が前提にできる形）を追加 ③ 粒度基準に「観測の独立化の適用境界」を追記。
- **2026-07-05 追記②（アプリ組み上げの所有確定・`app-shell` 新設）**: 「エンジンが揃った後、誰がアプリとしての areka を組むか」が無所属と判明→**三段構え**を確定（①`areka-P0-app-shell`＝骨格・main.rs 所有・**モックデモを `examples/mock-shell.rs` へ別名保全**・brief 済／②`ghost-setup`＝結線／③`emo2-conformance-e2e`＝適合証明）。boot/close のイベント発火順序は kanade の運行表へ帰属（ukadoc 調査済み・kanade 行に転記）。アプリ層の任意要素（SSTP/FMO/Plugin/更新/選択 UI）は M2 予約に裏付けつきで記録。**即並走可能フロントは3本**: `areka-P0-app-shell`（main.rs を触るのは当面本ユニットのみ＝非衝突。早期完了が emo-present/window-placement の衝突を構造ごと解消）／`areka-P0-emo-compose`（純粋層・新設モジュール＝非衝突）／`areka-P0-host32-lifecycle`（①shiori 逐次次フロント・非衝突）。
- 旧 active/brief（M1 憶測・M2 reference・出荷層）・backlog（P1-P3）・`_rejected/`・旧戦略メモは**削除**（git 履歴に保全。必要時に復元可）。

## M2 以降

**M1 完成後に、実物を見て組み直す。** 本ロードマップでは扱わない（pasta の native x64・`IShiori` in-proc 化、ベクトル描画・AI、**owner-draw 右クリック system メニュー（ゴースト管理 chrome）**、互換面拡大＝Shift_JIS/SAORI/里々・YAYA 網羅/NAR 等はその時に）。

**アプリ層の M2 予約（2026-07-05 ukadoc 裏付け・全て任意＝emo2 単体起動に不要）**: SSTP ポート（9801）ホスティング・FMO・DirectSSTP・Plugin/HEADLINE/SAORI ホスティング・ネットワーク更新（`\![update,platform]`・OnBasewareUpdating/Updated 系）・ゴースト/バルーン選択 UI（OnGhostChanging 系）・多重ゴースト運用。

**emo テキスト進化の予約（M2 候補・2026-07-03 開発者表明）**: ①**回転テキストの実挙動**（M1 で内部表現＝行列変換領域は構造として持ち込み済み・M2 で回転値を解禁）②**ポップアート級の文字装飾**（アウトライン/多色/シャドウ/変形等の text effects——1枚物自前合成ゆえ文字も合成レイヤの一つ＝装飾は emo 内で完結）。M1 の emo-text-layer が「行列領域＋装飾シーム」を仕込むことで、ここへ滑らかに接続する。

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

### main（本坑）: ✅ `wintf-clickthrough-alpha-toggle`（**2026-07-02 完了・アーカイブ** `completed/wintf-clickthrough-alpha-toggle`）

```
_Depends(confirmed): pilot-clickthrough-alpha-toggle
```

- **✅ 実装完了（2026-07-02・開発者承認）**: GPU 合成（WUC/DComp）を維持したまま別プロセスへのクリック透過を本体 wintf に実装。必須配合＝`WS_EX_LAYERED` 同伴フラグ（`apply_layered_companion`）＋`WS_EX_TRANSPARENT` 動的トグル（`apply_click_through`）＋別スレッドのカーソル監視＋シーングラフ・ヒットテスト連動（`ScreenToClient`→`hit_test_in_window`）。areka を WUC 化し実マスコットで実動確認（窓=`HitTest::none()`・画像=`HitTest::alpha_mask()`）。`docs/click_through.md` 整備・切り分け検証台 `crates/areka/examples/clickthrough_two_rects.rs`。ULW は並走残置（撤去は `wintf-ulw-removal`）。spec: `.kiro/specs/completed/wintf-clickthrough-alpha-toggle/`。
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
- **✅ 完了 `wintf-ulw-removal`（本坑・2026-07-05 完了・開発者承認）**: ULW 一式除去＋`CompositionMode` collapse（GPU 合成単独へ）完遂。`compositor.rs`/`compositor_systems/`/`com/ulw.rs` 削除・`CompositionMode` enum/フィールド/再エクスポート撤去・全 production 参照追随・examples3本削除。残す WUC 経路とクリックスルー機構（`apply_layered_companion`＋`ecs/clickthrough/`）は非改変（main 比 diff ゼロ）＝純粋機能ドロップ。α源は per-widget `AlphaMask::is_hit` のみ（staging α撤去）。実マスコット起動で WUC 単独描画・クリックスルー透過を目視サインオフ済み。spec: `.kiro/specs/completed/wintf-ulw-removal/`。
  ```
  _Depends: wintf-clickthrough-alpha-toggle（完了）✓
  ```
  **ULW を安全に消せるのは、本坑クリックスルーが完了して「ULW 無しでも別プロセス透過が成立」と確認できてから**（クリックスルー brief の並走方針＝「完全有効なら ULW 破棄／当面は並走・即時撤去しない」に一致）＝前提充足済み。`wintf-dcomp-to-wuc-migration` とは触るファイルが別ゆえ**独立**（両完了で `CompositionMode` は WUC 単独へ最終 collapse 完了）。クリックスルーのα源は ULW compositor の staging αバッファではなく per-widget `AlphaMask` を使用（実装で確認済み）。

### 依存マップ検証（two-tunnel 手動チェックリスト）

- 順序ゲート: `wintf-ulw-removal` は `wintf-clickthrough-alpha-toggle` 完了が前提（ULW 破棄の安全網）✓
- 独立性: `wintf-dcomp-to-wuc-migration`（表示層・**✅2026-07-02 完了**）は `wintf-clickthrough-alpha-toggle`（当たり判定層）とも `wintf-ulw-removal`（ULW 側・別ファイル群）とも独立・順序任意 ✓
- 循環なし／DAG: clickthrough → ulw-removal の単一エッジのみ（wuc-migration は独立ノード・巡回なし）✓
