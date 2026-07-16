---
inclusion: always
updated_at: 2026-07-16
---

# Focus - ロードマップ管理

arekaアルファリリースロードマップと`.kiro/specs/`配下の仕様ポートフォリオを整合させるための運用ガイド。

---

## ROADMAP 参照タイミング

- **セッション開始時**: 次に取り組む仕様を確認
- **仕様完了時**: 次の仕様を決定
- **仕様作成時**: 依存関係を確認
- **仕様の棚卸し時**: 直下・待機・完了・却下の各配置先が妥当か確認

## ROADMAP 更新タイミング

- **仕様フェーズ変更時**: phase列を更新
- **新規仕様作成時**: 実行計画に追加
- **仕様完了時**: 進捗サマリーを更新
- **仕様を却下した時**: ROADMAP対象外であることを確認し、`_rejected/`に隔離

## フォルダー配置ルール

| 状態 | 配置先 |
| ------ | ------ |
| アクティブ（P0） | `.kiro/specs/` 直下 |
| 待機（P1-P3） | `.kiro/specs/backlog/` |
| 完了 | `.kiro/specs/completed/` |
| 却下 | `.kiro/specs/_rejected/` |

## 件数集計ルール

進捗件数は **`spec.json` の `phase` 値ではなく、配置フォルダを基準**に数える（`phase` 値は履歴上ズレるため当てにしない）。

| 配置 | 計上区分 |
| ------ | ------ |
| `.kiro/specs/` 直下（completed/backlog/_rejected 以外） | アクティブ（P0） |
| `.kiro/specs/backlog/` | 待機（P1-P3） |
| `.kiro/specs/completed/` | 完了 |
| `.kiro/specs/_rejected/` | 却下（集計対象外・参考） |
| `spec.json` を持たないディレクトリ（例: `shape-*`） | 構想段階（Phase 0）として別掲 |

- 直下に `phase=completed` のまま残る仕様（例: 旧メタ仕様）があれば `completed/` への移動候補として棚卸しに挙げる
- 棚卸しの基準実数（2026-06-28・clean slate 後）: 完了99（歴史） / **active = 0**（憶測仕様を全伐採・実装ファーストで着手時に作る）。backlog・`_rejected/`・旧戦略メモは削除（git 履歴に保全）。ロードマップは **M1 のみ**（M2+ は M1 完成後に再構築）
- 棚卸しの基準実数（2026-07-01 更新）: 完了 **103**（歴史） / **spec.json 有りの active = 0**（不変） / **直下の brief-only（spec.json 無し＝Phase 0 構想）= 7**（`/kiro-discovery` で just-in-time 生成した**着手可能フロント**: wintf 基盤層 `wintf-dcomp-to-wuc-migration`・`wintf-clickthrough-alpha-toggle`・`wintf-ulw-removal`／M1 parser `areka-P0-shell-parse`・`areka-P0-parser-foundation`（旧 balloon-parse・2026-07-02 開発リジェクト→共通基盤へリネーム・brief-only へ復帰）・`areka-P0-package-mount`／M1 host-32 `areka-P0-host32-ipc`）。`/kiro-start <name>` で本坑ライフサイクル入り＝その時点で spec.json が生えて active へ遷移。件数は配置フォルダ＋spec.json 有無基準（brief-only は Phase 0 別掲）
- 棚卸しの基準実数（2026-07-05 更新・マージ後の実地確認＋全体精査）: 完了 **116**（`areka-P0-actor-foundation`・`wintf-ulw-removal`・`areka-P0-host32-request`・`areka-P0-emo-atlas` が新規完了アーカイブ） / **active = 0** / **brief-only = 7**（既存4: `areka-P0-app-shell`・`areka-P0-emo-compose`・`areka-P0-emo-present`・`areka-P0-window-placement`〔emo-present ゲート下〕＋新規3: **`areka-P0-host32-lifecycle`**〔①最終・死活語彙の正本〕・**`areka-P0-kanade`**〔運行表・talk 起動契約の正本・actor-foundation✅ で解禁〕・**`areka-P0-sakura-engine`**〔再生出力契約の正本・script 直入力で単体観測〕）。**M1 M-boot 進捗 約 10/20**。**即並走可能フロント5本**: app-shell／emo-compose／host32-lifecycle／kanade／sakura-engine（契約の正本連鎖で並走担保: lifecycle→kanade→sakura→seriko/emo-text-layer）。既存 brief の申し開きは実シンボル参照へ再調整済み（07-05）。正本 roadmap.md ポートフォリオ節＋所有マップ
- 棚卸しの基準実数（2026-07-14 更新・M-boot 達成後の実機サインオフ増分フェーズ）: 完了 **130** / **spec.json 有りの active = 0** / **brief-only（Phase 0 構想）= 2**（**`areka-P0-cue-playback-duration`**〔実装中・#3/#4/#6＝dola/sakura/emo-text 横断の再生 duration 権威アーキ〕・**`areka-P0-mayuna-compose`**〔#2 bind 着せ替え・②④⑤垂直スライス〕）。**M-boot（`areka-P0-emo2-boot`）達成済み**（emo2 起動→表示→OnBoot talk→OnClose→clean exit・commit `104a4ac8`）＝M1 の山を越え**実機サインオフ増分フェーズ**へ。実機欠陥7件の仕分け完了（#1 `surface-resize-resnap`✅／#5 emo2-boot この場修正済／#7 上流 pasta 起票＝スコープ外／#2 mayuna-compose／#3#4#6 cue-playback-duration）。**現行の並走フロント**（実装中の cue-playback と cue モデル4ファイル非共有）: **`position-persist`（⓪ghost・完全直交）・`seriko-loop`（⑤seriko・blink）・`idle-talk`（③kanade・自発会話）**＝いずれも brief 未作成（just-in-time で起こす）。**`mayuna-compose` は cue-playback 完了まで時限ゲート下**（roadmap「並走安全」節 2026-07-14 注記）。正本 roadmap.md ポートフォリオ節
- 棚卸しの基準実数（2026-07-16 更新・再入精査⑦＝「伺かアプリとしての体裁」フェーズ）: 完了 **130** / **active = 0** / **brief-only = 6**（実装中 `areka-P0-cue-playback-duration`〔別坑・PR 未提出〕・ゲート下 `areka-P0-mayuna-compose`＋**新規4: `areka-P0-position-persist`〔⓪完全直交・OnFirstBoot 初回ゲート〕・`areka-P0-idle-talk`〔③背骨実装済み＝正典充足へ再スコープ〕・`areka-P0-collision-geometry`〔⑥撫でクラスタ契約 `HitRegion` 正本〕・`areka-P0-input-events`〔③マウス2イベント・dblclick stand-in 退役〕**）。**並走フロント最大5本**（cue-playback 走行中 ∥ 新規4）。**時限ゲート補正**: `seriko-loop` は cue-playback の CueSink 集約と seriko actor 受信面が近接＝ゲート下へ（07-14 判定を補正）。M-dual は大半 M-boot 充足済み＝検証へ縮退。正本 roadmap.md 追記㉕＋並走安全注記
- 棚卸しの基準実数（2026-07-05 更新②・`host32-lifecycle` 完了後）: 完了 **117**（`areka-P0-host32-lifecycle` 新規完了アーカイブ＝①shiori トラック全完了 pilot✅/ipc✅/shiori-load✅/request✅/lifecycle✅） / **active = 0** / **brief-only = 6**（`areka-P0-app-shell`・`areka-P0-emo-compose`・`areka-P0-emo-present`〔emo-compose ゲート下〕・`areka-P0-window-placement`〔emo-present ゲート下〕・`areka-P0-kanade`・`areka-P0-sakura-engine`）。**即並走可能フロント4本**: app-shell／emo-compose／kanade／sakura-engine（`host32-lifecycle` は消化・死活報告 API を kanade が正本消費）。正本 roadmap.md 追記④
- 棚卸しの基準実数（2026-07-05 更新③・`app-shell`＋`kanade` 完了後）: 完了 **119**（追記⑤ `areka-P0-app-shell`＝アプリ骨格・追記⑥ `areka-P0-kanade`＝runtime 制御階層③運行表/talk 起動契約の正本を新規完了アーカイブ） / **active = 0** / **brief-only = 4**（`areka-P0-emo-compose`・`areka-P0-emo-present`〔emo-compose ゲート下〕・`areka-P0-window-placement`〔emo-present ゲート下〕・`areka-P0-sakura-engine`）。**即並走可能フロント2本**: emo-compose／sakura-engine（`app-shell` は消化・`kanade` 完了で talk 起動契約 StartTalk/TalkDone が先決＝下流 sakura-engine 並走続行可）。正本 roadmap.md 追記⑤⑥
- 棚卸しの基準実数（2026-07-05 更新④・`sakura-engine`＋`emo-compose` 完了後）: 完了 **121**（追記⑦ `areka-P0-sakura-engine`＝runtime 制御階層④再生出力契約・追記⑧ `areka-P0-emo-compose`＝emo 直列2・合成コア〔fold→plan→blit→Composer facade・実 emo2 fixture golden・ログ発火/エラー写像も決定論的回帰檻化〕を新規完了アーカイブ） / **active = 0** / **brief-only = 2**（`areka-P0-emo-present`〔**emo-compose 完了でゲート解除＝新フロント**〕・`areka-P0-window-placement`〔emo-present ゲート下〕）。**即並走可能フロント1本**: emo-present（`emo-compose`・`sakura-engine` は消化・下流 emo-present が `ComposedSurface`／正規化 Surface 公開形を正本消費／sakura は再生出力契約を seriko・emo-text-layer へ先決）。正本 roadmap.md 追記⑦⑧
- 棚卸しの基準実数（2026-07-05 更新⑤・再入精査②＝runtime 最終フロント確立）: 完了 **121**（不変） / **active = 0** / **brief-only = 4**（`areka-P0-emo-present`〔実シンボル再調整済み〕・**`areka-P0-seriko-engine`**〔新規 brief・sakura `TalkCue`/`SurfaceSink` 正本✅＋emo-compose `AliasMap`/`BindSet` 正本✅で解禁〕・**`areka-P0-ghost-setup`**〔新規 brief・**talk 契約フォーク（kanade `quit:bool`／永続 channel vs sakura `TalkEndReason` 3値／per-talk spawn）の統一 WS-A＋結線の背骨 WS-B を所有**〕・`areka-P0-window-placement`〔emo-present ゲート下・再調整済み〕）。**M-boot 16/21（残5）**。**即並走可能フロント3本**: emo-present／seriko-engine／ghost-setup（非衝突: example＋emo 新層／新設 crates/areka-seriko／main.rs 結線＋talk 授受面。保護規約=ghost-setup WS-A は `TalkCue`/sink trait を凍結）。正本 roadmap.md 追記⑨
- 棚卸しの基準実数（2026-07-06 更新⑥・`seriko-engine` 完了後）: 完了 **122**（`areka-P0-seriko-engine`＝⑤ シェルアニメーションエンジンを新規完了アーカイブ。新設 crates/areka-seriko） / **active = 0** / **brief-only = 3**（`areka-P0-emo-present`・`areka-P0-ghost-setup`・`areka-P0-window-placement`〔emo-present ゲート下〕）。正本 roadmap.md 追記⑩
- 棚卸しの基準実数（2026-07-09 更新⑦・`ghost-setup` 完了後）: 完了 **123**（`areka-P0-ghost-setup`＝⓪ ghost エンジン起動〜終了統括を新規完了アーカイブ。WS-A talk 契約統一〔新設 `areka-talk`〕＋WS-B 結線の背骨〔新設 `areka-ghost`〕・決定論 spine e2e S1〜S6・env ゲート実 pasta 追験） / **active = 0** / **brief-only = 2**（`areka-P0-emo-present`・`areka-P0-window-placement`〔emo-present ゲート下〕）。正本 roadmap.md 追記⑪
- 棚卸しの基準実数（2026-07-09 更新⑧・`emo-present` 完了後＝⑥emo トラック全完了・並走ブランチ統合の反映）: 完了 **124**（`areka-P0-emo-present` 新規完了アーカイブ＝emo 直列3/3・表示結線。実装完了後の実機まばたきデモ検証で `ComposeCache` のキャッシュ仕様バグ〔surface id 単独キーが bind 差分に衝突〕を発見・是正＝合成入力（surface_id＋BindSet）完全一致キーの容量1メモ化スロットへ再設計） / **active = 0** / **brief-only = 1**（`areka-P0-window-placement`〔**emo-present 完了でゲート解除＝新フロント**〕）。**M-boot 19/21（残2: emo-text-layer・window-placement）**。**即並走可能フロント2本**: window-placement／emo-text-layer（`emo-present`・`seriko-engine`・`ghost-setup` は消化）。正本 roadmap.md 追記⑫
- 棚卸しの基準実数（2026-07-09 更新⑨・再入精査③＝「M-boot 統合」の無所属解消）: 完了 **124**（不変） / **active = 0** / **brief-only = 3**（`areka-P0-window-placement`〔07-09 実シンボル再調整済み＝ghost-setup✅ でシーム単独所有・`attach_target(window: Entity)` 実形・donor に `examples/emo-present.rs`〕・**`areka-P0-emo-text-layer`**〔新規 brief・TextSink 実装＋純粋状態機械＋DirectWrite 縦書き lift＋行列領域内部表現〕・**`areka-P0-emo2-boot`**〔新規 brief・**M-boot 統合＝マイルストーン完成ユニット**（実 sink 差し替え・`SurfaceOutput`→`PresentCommand` アダプタ・窓装着）・**window-placement＋emo-text-layer の完了ゲート下＝並走フロントではない**〕）。**M-boot 19/22（統合ユニット明示計上・残3）**。**即並走可能フロント2本**: window-placement／emo-text-layer（保護規約: placement=crates/areka・emo-present crate 不改変／text-layer=areka-emo-present additive 増分・crates/areka 不触＝衝突面ゼロ）。正本 roadmap.md 追記⑬
- 棚卸しの基準実数（2026-07-09 更新⑩・emo-text-layer 重量査定＋viewbox 切り出し）: 完了 **124**（不変） / **active = 0** / **brief-only = 4**（window-placement・emo-text-layer〔縦書き＝完了条件・横書き先行可・スクロール＝全域再描画確定〕・emo2-boot〔ゲート下〕・**`areka-P0-emo-text-viewbox`**〔新規 brief・⑥emo 増分＝M-boot 外・viewbox 合成スクロール・依存=emo-text-layer のみ＝並走安全〕）。**即並走可能フロント2本のまま**: window-placement／emo-text-layer。正本 roadmap.md 追記⑭
- 棚卸しの基準実数（2026-07-11 更新⑪・window-placement✅＋emo-text-layer✅ マージ後の再入精査④）: 完了 **126**（追記⑮ `areka-P0-window-placement`＝窓生成/既定位置/全面ドラッグ/バルーン追従/bottom吸着・追記⑯ `areka-P0-emo-text-layer`＝新設 crates/areka-emo-text・cue駆動 typewriter・縦横両対応を新規完了アーカイブ） / **active = 0** / **brief-only = 2**（**`areka-P0-emo2-boot`**〔**両ゲート解消＝解禁**・M-boot 最終統合・07-11 実シンボル再調整済み: `GhostWindows` 窓写像✅・`EmoTextSink`/`register_actor_view`✅・`text_slot_view`＝初回 ShowSurface まで None の装着順序制約・`present_frame` 毎フレーム駆動義務・`\b` cue 裁定〕・**`areka-P0-emo-text-viewbox`**〔ゲート解消＝解禁・分離シーム実シンボル `visible_window`→`VisibleWindow`→`DrawExecutor::render` 確認済み〕）。**M-boot 21/22（残1: emo2-boot）**。**即並走可能フロント2本**: emo2-boot／emo-text-viewbox（**交差面ゼロ**: 前者は areka-emo-text 消費のみ・後者は描画実行側のみ改変＋pixel 等価 golden）。新規 spec 不要判定・増分ユニット名 `areka-P0-sakura-glyph-pacing` を④sakura へ登録（pacing 申し送りの宛先解消・brief は着手時）。正本 roadmap.md 追記⑰
- 棚卸しの基準実数（2026-07-11 更新⑫・emo2-boot 要件精査でブロッカー検出→中断）: 完了 **126**（不変） / **active = 1**（`areka-P0-emo2-boot`＝spec.json 生成済み・requirements-generated で**中断保留**——要件ディスカッション議題1で `\b` cue ドメイン三重欠落〔parser Raw 落ち・compile debug! 破棄・`CueCommand` variant 不在〕＋旧形式 `\bN` 本文数字漏れ＋バルーン表示指令の配管不在を検出・R5 前提破綻） / **brief-only = 2**（**`areka-P0-balloon-face-cue`**〔新設・emo2-boot ブロッカー＝②④⑤横断の cue 第一級化・`\s` 完全対称・即着手フロント〕・`areka-P0-emo-text-viewbox`〔並走可・交差面ゼロ〕）。**M-boot 22→23 ユニット（21/23・残2: balloon-face-cue → emo2-boot）**。ブロッカー登記 B1〜B8 は balloon-face-cue brief に収録。正本 roadmap.md 追記⑱
- 棚卸しの基準実数（2026-07-12 更新⑬・`balloon-face-cue` 完了後）: 完了 **127**（`areka-P0-balloon-face-cue`＝②④⑤横断の `\b` cue 第一級化を新規完了アーカイブ＝emo2-boot ブロッカー解消。parser→dola→sakura→seriko→emo-present 回帰の垂直増分・三重無音破棄＋裸形本文漏れ根絶・決定論 E2E／additive／`cargo test --workspace` exit 0） / **active = 1**（`areka-P0-emo2-boot`・**中断保留のまま**＝ブロッカー解消で再開可・再開時に R5 を「実 cue が届く」前提へ改稿） / **brief-only = 1**（`areka-P0-emo-text-viewbox`〔並走可・交差面ゼロ〕）。**M-boot 22/23（残1: emo2-boot・ゲート解消済み）**。**即着手フロント2本**: emo2-boot（balloon-face-cue✅ でブロッカー解消＝R5 再構築可能に）／emo-text-viewbox（増分）。正本 roadmap.md 追記⑲
- 棚卸しの基準実数（2026-07-13 更新⑭・`emo2-boot` 完了＝**M-boot 達成**）: 完了 **128**（`areka-P0-emo2-boot`＝M-boot 統合ユニット〔`areka.exe <emo2>` 起動→本物サーフェス表示→OnBoot typewriter talk→OnClose 握手→clean exit〕を新規完了アーカイブ・`cargo test --workspace` exit 0〔host-32 i686 成果物ビルド後〕） / **active = 0**（M-boot 完成） / **brief-only = 2**（`areka-P0-cue-playback-duration`〔#3/#4/#6・dola+sakura+emo 横断〕・`areka-P0-surface-resize-resnap`〔#1〕）＋登録済み増分 `areka-P0-mayuna-compose`〔#2〕。**M-boot 23/23＝完成**。R9.3 実機サインオフ7件は仕分け済（#7 は pasta 上流＝areka スコープ外）。正本 roadmap.md 追記㉒
- 棚卸しの基準実数（2026-07-13 更新⑮・emo2-boot マージ後の再入精査⑤＝R9.3 remediation 3本の並走性確定）: 完了 **128**（不変・配置フォルダ基準） / **active = 0** / **brief-only = 3**（`areka-P0-cue-playback-duration`〔#3/#4/#6〕・`areka-P0-surface-resize-resnap`〔#1〕・**`areka-P0-mayuna-compose`**〔#2・**新規 brief**＝②④⑤垂直スライス・balloon-face-cue 同型〕）。実コード偵察で**衝突分析確定**: surface-resize-resnap は placement/emo-present-size/frame-drain のみ＝**完全独立**／cue-playback-duration⟷mayuna-compose は dola `CueCommand`＋sakura `compile.rs`/`contract.rs`＋emo-text `state.rs` の4ファイル共有だが**別アーム additive＝マージ可能**。**並走フロント: Wave1＝cue-playback-duration ∥ surface-resize-resnap（near-disjoint 即並走）／Wave2＝mayuna-compose（cue-playback 先行 or `CueCommand` 契約先決で3本並走）**。フェーズ別最適モデル（要件/設計/タスク/実装×3本）は本会話提示＝設計フェーズが最も tier を要し cue-playback-duration の design のみ Fable 級・他は Opus high〜xhigh。正本 roadmap.md 追記㉓
- 棚卸しの基準実数（2026-07-16 更新⑯・再入精査⑧＝体裁フェーズ棚卸・**M1 残工程の brief 全数完備**）: 完了 **130** / **active = 0** / **brief-only = 11**（実装中 `areka-P0-cue-playback-duration`〔別坑〕・ゲート下 `areka-P0-mayuna-compose`・並走中フロント4本 `position-persist`/`idle-talk`/`collision-geometry`/`input-events` ＋ **新規5本: `areka-P0-sakura-dialogue-tags`**〔④・\q/\_l/\![move]/%username の compile 貫通・choice cue 形の正本・cue-playback ゲート下〕・**`areka-P0-choice-render`**〔⑥・選択肢 UI・ChoiceSelection 契約の正本〕・**`areka-P0-choice-select-events`**〔③・新設名＝input-events 分離増分の宛先・任意名イベントカスケード〕・**`areka-P0-seriko-loop`**〔⑤・Tick 注入＋pattern 状態・ゲート下〕・**`areka-P0-emo2-conformance-e2e`**〔⓪統合・**M1 完成宣言ユニット**・適合検証14項目・M-dual 吸収の正本〕）。**roadmap-only（brief 無し）ユニット＝ゼロ**・**M-dual（dual-surface/dual-window）は退役**（e2e #10 へ吸収）。**M1 残工程ゴール表**を roadmap 統合点直下に新設（マイルストーン別の単一文ゴール）。並走フロントは不変（最大5本＝cue-playback 走行中 ∥ position-persist ∥ idle-talk ∥ collision-geometry ∥ input-events）・cue-playback 完了後 Wave＝mayuna ∥ sakura-dialogue-tags → seriko-loop・choice-render ∥ choice-select-events → e2e（最終）。正本 roadmap.md 追記㉖＋M1 残工程ゴール表

## 運用上の注意

- `.kiro/specs/`直下には、進行中の仕様ディレクトリだけでなく、調査メモや戦略文書が単体Markdownとして置かれることがある
- ROADMAPと進捗集計の対象は、原則として`spec.json`を持つ仕様ディレクトリ
- `completed/`配下には履歴上の古いphase値を含む仕様が残るため、集計時は配置場所を優先して判断する

## 参照先

📍 `.kiro/steering/roadmap.md` … **ロードマップ正本**（kiro 標準テンプレート・`inclusion: manual` で非常駐。`/kiro-discovery` 再入・`/kiro-spec-batch` が標準パスで参照）
📍 `.kiro/specs/*/spec.json` … 各仕様の phase/approvals（件数は配置フォルダ基準で数える）
📍 `doc/COMPAT_ARCHITECTURE.md` … 設計判断の正本
📍 `doc/ROADMAP.md` … 旧パスのポインタ stub（正本は steering/roadmap.md）
📍 `.kiro/steering/two-tunnel.md` … 二坑モデル規律の正本（`inclusion: manual` で非常駐）
📍 roadmap.md「エンジン固有名」節 … 7エンジン⓪〜⑥の固有名正本（**ghost / shiori / parsers / kanade / sakura / seriko / emo**・2026-07-02 確定）
