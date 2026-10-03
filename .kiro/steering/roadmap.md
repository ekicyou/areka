---
inclusion: manual
updated_at: 2026-10-03
---

# Roadmap — areka（α 後・M3 へ向けた並べ直し）

> **M1 は 2026-09-11、M2（α）は 2026-10-02 に完成宣言済み**。本ファイルは 2026-10-02 の棚卸⑳で、α 後の着手順へ書き直した。**完了した spec の行・棚卸⑭〜⑲の裁定・α のウェーブ表・旧「登記だけの行」の全文は [roadmap-history.md](roadmap-history.md) の「2026-10-02 棚卸⑳退避」節へ逐語で移した**（history が全文正本・非改変）。
> 正本配置: 本ファイルが正本（`.kiro/steering/roadmap.md`）。`focus.md`（`inclusion: always`）から辿る。設計判断の正本は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md)。ukadoc 網羅の段階・順位の正本は `doc/ukadoc-coverage/`（`briefing.md`・`linkage.md`・`roadmap-draft.md`）。完了した spec の実装の詳細は各 `completed/` spec が正本。
> **読む順**: いま何を始めるか＝「ウェーブ編成」。個々の spec の中身＝「spec 台帳」と各 brief。brief を持たない宿題＝「覚え書き」。

## M1 ゴール ✅（2026-09-11 完成宣言）

areka（x64）が最小 SSP 互換ベースウェアとして、適合対象ゴースト **emo2**（作者自作・脳=`pasta.dll`・32bit SHIORI）を「そのまま」起動→会話→撫で→メニュー→終了まで E2E 実走させる。emo2 が動く＝同じ汎用 32bit ブリッジで里々/YAYA も動く土台。

**✅ 完成宣言**: 適合検証項目表 20 項目全合格の実機サインオフに開発者の署名（`.kiro/specs/completed/areka-P0-emo2-conformance-e2e/verification/m1-completion.md`・項目別は同 `acceptance-record.md` §8.1）。持ち越し 8 件（同 §6）はいずれも判定を書き換えず、引受先は下の spec 台帳に全て実在する（§13.1 行 3「初回起動限定の位置調整」だけは「違和感があれば個別仕様を切る」の据え置き＝spec なし）。

## M2 ゴール（α）✅（2026-10-02 完成宣言）＝第三者がデスクトップマスコットを管理できる

**✅ 完成宣言**: 第三者の手順の検証項目表（13 項目と付随の確認 2 つ）の実機サインオフに開発者の署名（`.kiro/specs/completed/areka-P0-alpha-release-signoff/verification/alpha-completion.md`・項目別は同 `acceptance-record.md` §7）。署名の根拠のコミット `8460506d`・zip `areka-alpha-x64-20261002-8460506.zip`。

**利用者の一周**: zip を展開して起動 → `.nar` を窓へ落とす（またはメニューから選ぶ）→ ゴーストが起動する → 右クリックメニューでゴースト／シェル／バルーンを替える → ネットワーク更新で作者の修正を受け取る → 終了 → 再起動で前回の状態に戻る。表現力は M1 の水準（emo2 が普通に動く）で据え置いた。メニューは Win32 標準。

**持ち越し**: 完成判定 §6 の 5 行は 4 本の spec（`install-live-target-hazards`・`drag-click-without-move`・`package-check-temp-cleanup`〔10-02 に `release-package-versioned` へ合流〕・`install-companion-canon`）へ起票済み＝ウェーブ C1・C2。emo2 の 2 件は ghost_dev へ申し送り済み。

## M3 ゴール（未確定）

M3 のゴールはまだ決めていない（**開発者の決めごと**）。候補は `doc/ukadoc-coverage/roadmap-draft.md`「M3 の受入基準の候補」（「伺かの冠」）。決まるまでの着手順は、開発者指示（2026-10-02）「**バグ修正系を優先し、あとは優先度の高い機能**」に従う。優先度の物差しは `briefing.md` の 4 軸（壊れ方 ＞ 伺からしさ ＞ 資産の広さ ＞ 基盤共有度）と、開発者が時期を指定した spec 群（シェル内バルーン＝「α 後なるべく早い時期」）。

## 実装規律（balloon-system の失敗から得た正）

- **実装ファースト**: 各作業ユニットの成果物は「実際に動く」検証済みコード。**spec 工場の禁止**: 成果物が子 spec になる構造を作らない。1 ユニット＝1 かたまりの動く振る舞い。
- **最小実装＋薄い拡張シーム**: 使う分だけ実装し、拡張は型/レジストリの口だけ残す。抽象は「2 例目の実物」が要求してから。動く資産から建てる。
- **粒度基準**: 1 ユニット＝単一 pass/fail の独立観測。純粋層は fixture/mock 直入力で切る。UI 位置決め・座標系は本番ゴースト（実 emo2）＋実 DPI（≠96）が観測条件（記憶 areka-placement-real-ghost-first）。
- **語彙完備・配線ゼロの追跡**: 先送りシームには狭い `#[allow(dead_code)]`＋実在理由の doc を義務付け、消費者ゼロの検出は棚卸の定期監査項目。
- **1 ファイル 1,000 行**: 機械の番人 `crates/log-capture-kit/tests/file_length_guard_test.rs`（例外表 11 件・暗黙増加不可・**どの spec も例外表に触れない**）。2026-10-02 実測: `areka-emo-text/src/` の `layout.rs` 977・`region.rs` 977・`actor.rs` 975・`viewbox_draw.rs` 914 と `crates/areka/src/` の `main.rs` 946・`input_events/balloon.rs` 930・`emo2_boot/balloon_visibility.rs` 923 が射程＝`emo-text-file-split`（C1）が先に分ける。それまでは新規ファイルで足す。
- **決定論テスト網羅は必達**・**ログ無し失敗経路の禁止**・**終了経路は正規実装**（記憶 deterministic-test-coverage-mandate／areka-log-first-no-silent-failure／canonical-not-minimal-lifecycle）。
- **外部依存の追加は `tech.md` へ「意図的依存追加」を登記し開発者が承認する**（`encoding_rs` の前例）。α で登記したのは **`miniz_oxide` 0.9（`nar-install`・2026-09-18 承認済・伸長のみ・推移的依存は `adler2` 1 本）**、`md-5` は要らなくなった（`update-engine` 2026-09-24 完了＝MD5 は OS の CNG・`tech.md` 登記済み）。HTTP は WinHTTP（`windows` crate の機能フラグ）で crate を足さない。

## アーキテクチャ横断原則（要約・詳細は history＋記憶＋completed spec）

- **シェル/バルーン統一**: 描画エンジンはシェルとバルーンを区別しない。バルーン＝surface 上の文字レンダリング層。
- **アニメエンジンは 2 つ**: ①さくらスクリプト再生（sakura）＋②SERIKO ループ（seriko）。両者とも dola（絶対時刻台本）上。
- **並行モデル**: 各エンジン＝チャンネル通信のアクター＋独立スレッド。render/window は UI スレッド固定。機構=areka-actor／経路=kanade／結線=ghost。
- **emo 合成**: 自前コンポジタ（アトラス→1 枚物合成→wintf へ完成品のみ）。emo=UI 層全般。**画像は原寸で持ち拡大縮小は D2D 変換行列**（2026-09-11 裁定・`present-gpu-transform-scale` が実装）。
- **DPI 追従が基本設計**: k=monitorDPI÷author_dpi で全表示経路がスケール。キャラ窓の原点は下端中央。バルーン追従は例外＝「窓（char 左上）相対」・offset の単位空間は表示 DPI の物理 px 1 つ。
- **スコープ窓の重なり**: 既定は非強制、`\![set,zorder]` 後は Windows の所有関係（owner）の一直線の鎖で構造保証。
- **意味論は ukadoc から輸入・SSP 実測主義は取らない**（記憶 no-ssp-measurement-import-semantics-from-ukadoc）。**`\!` コマンドは汎用キャリア 1 本**（typed 個別新設禁止・消費側 name 選別）。
- **アプリの寿命は窓の数から切り離す**（2026-09-18・`ghost-shell-balloon-switch` が実装）: 切替中に窓が 0 になっても終了しない。終了は明示の `AppExit`。
- **ベースウェアの根は 1 つ**（2026-09-18・`baseware-root-layout` が実装）: `<根>/ghost/<名>/`・`<根>/balloon/<名>/`＝ukadoc「全体の構成」の格納フォルダ。`nar-install` の展開先もこの形。

**エンジン固有名**（コード/spec/会話の参照はこの名で統一）:

| # | エンジン | 固有名 | # | エンジン | 固有名 |
|---|---|---|---|---|---|
| ⓪ | ゴーストエンジン（最上位 owner） | `ghost` | ④ | さくらスクリプト再生 | `sakura` |
| ① | SHIORI 通信層 host-32 | `shiori` | ⑤ | SERIKO アニメ | `seriko` |
| ② | parser/loader | `parsers` | ⑥ | render（surface 合成＋UI 層） | `emo` |
| ③ | conductor（SHIORI イベント循環） | `kanade` | | | |

## 完了サマリ（詳報は各 `completed/` spec ＋ history）

| Wave | 完了日 | ユニット（PR） | 一行要約 |
|---|---|---|---|
| 耐力壁 | 07-01 | `pilot-shiori-host-32` | x64→32bit pasta.dll 駆動 GO |
| M-boot | 07-13 | `emo2-boot` 23/23 | 起動→表示→talk→close の可視一周 |
| W1〜W4 | 〜07-29 | idle-talk／collision-geometry／sakura-dialogue-tags／input-events／mayuna-compose／sylphya／seriko-loop／choice-render／position-persist／choice-interact／emo-dpi-scaling 他 | 増分ウェーブ全完走・DPI 追従 k 着地 |
| W5 | 08-01 | `choice-select-events`・`kero-balloon`（PR#97）・`dpi-window-vanish`（PR#98） | M-dialogue 完走 |
| W6 | 08-10 | `file-slimming`（PR#103） | 1,000 行超 54→0 |
| W7 | 08-13 | `collision-dpi-hittest`（PR#100）・`balloon-visibility`（PR#106）・`bindoption-exclusivity`（PR#105）・`ghost-window-zorder`（PR#107）・`scope-chain-gap`（PR#108） | 5/5 |
| W8 | 08-15 | `recompose-budget`（PR#112）・`scale-exact-rational`（PR#110）・`windowposition-limit`（PR#111） | 1 コマ 22,210→1,240µs |
| W9 | 08-22 | `dpi-transition-atomicity`（PR#114） | 書込散らばり ms→µs |
| W10 | 08-27 | `draw-load-parity`（PR#118）・`test-cage-determinism`（PR#119） | 門の手がかり→`tick-gate-adoption`／log-capture-kit |
| W11 | 09-02 | `present-write-coherence`（PR#123）・`balloon-vertical-canon`（PR#124）・`balloon-offset-dpi`（PR#125）・`scope-zorder-pinning`（PR#126） | 4/4 |
| ⓪ | 09-03 | `sakura-bare-tag-lexer`（PR#134） | `\_X` bare 漏れ修正・完了検証で `sakura-tag-word-boundary` 起票 |
| W12 | 09-05〜09-11 | `cursor-tag-canon`（PR#137）・`emo-text-line-height-canon`（PR#142）・`emo2-conformance-e2e`（PR#143・**M1 完成宣言**） ∥ 調査系 `ukadoc-survey-toolkit`（PR#136）・`-property`（PR#138）・`-shiori`（PR#139）・`-sakura-script`（PR#140）・`-assets`（PR#141） | 行送り 35→30・e2e 20 項目全合格・ukadoc 1,749 項目の台帳 4 本＋`report/summary.md` |
| W13 | 09-12〜09-17 | `present-gpu-transform-scale`（PR#145）・`charset-canon`（PR#146）・`ukadoc-coverage-roadmap`（PR#147）・`text-decoration-canon`（PR#148）・`kanade-boot-talkdone-drop`（PR#149）・`sakura-tag-word-boundary`（PR#150）・`sylphya-set-ledger`（PR#151）・`host32-window-thread-pump`（PR#153）・`balloon-font-descript-keys`（PR#154） | 9/9・1 コマ 41〜78→2.65 ms・任意 charset・67 束の網羅ロードマップ・文字装飾 3 書字方向 |
| A0（α） | 09-19 | `nar-install`（PR#158）・`default-balloon-bundle`（PR#157）・`popup-menu-minimal`（PR#159）・`balloon-origin-outside-validrect`（PR#161） | `.nar` の読取と安全な展開・既定バルーン（Staysee・CC0）・Win32 標準の右クリックメニュー |
| A1（α） | 09-20〜09-24 | `shell-implicit-surface`（PR#162）・`balloon-break`（PR#164）・`app-lifetime-separation`（PR#169）・`shiori-loadu`（PR#170）・`default-balloon-nar-fold`（PR#171）・`wintf-drag-state-rest-contract`（PR#172）・`nar-install-hardening`（PR#174）・`keycolor-clickthrough-coverage`（PR#175）・`update-engine`（PR#176）・`baseware-root-layout`（PR#177） | 里々のテンプレートが絵を出せる・抜き色・根 `ghost/`・`balloon/` と列挙・寿命を窓の数から分離・更新のエンジン |
| B1〜B2（α） | 09-24〜09-26 | `pilot-balloon-asset-swap`（PR#182）・`shiori-fault-notice`（PR#183）・`ghost-restart-unit`（PR#184） | SHIORI の失敗の告知・同じプロセスでゴーストを起こし直す形 |
| B3〜B4（α） | 09-26〜09-28 | `alpha-package`（PR#190）・`pilot-dropfiles-on-wuc-window`（PR#191）・`ghost-shell-balloon-switch`（PR#192）・`ghost-change-name-resolution`（PR#194）・`session-mark-residue`（PR#195） | ゴースト切替・汎用の通知の入口 `KanadeMsg::RaiseEvent`・起動中の印（きれいに終わらなければ次は emo2 が `halt` を伝える）・配布 zip |
| B5〜B6（α） | 09-28〜09-30 | `frame-phases-after-exit`（PR#197）・`ghost-install`（PR#198）・`file-drop`（PR#201）・`balloon-color-emoji`（PR#202） | メニューと台本と投げ込みからのインストール・カラー絵文字と書記素クラスタ |
| B7〜B9（α） | 09-30〜10-02 | `network-update`（PR#203）・`shell-balloon-switch`（PR#205）・`balloon-reappear-short-talk`（PR#206）・`alpha-release-signoff`（PR#210・**α 完成宣言**） | ネットワーク更新・シェルとバルーンの切替・実機一周 13 項目 |
| C1 | 10-03〜 | `restart-chain-finalize-stall`（PR#214）・`choice-timeout-directive`（PR#215）・`drag-click-without-move`（PR#216） | 初期配置の確定の見送りはゴースト待ち〔窓が無い・まだ一度も表示されていない〕を数えず、areka 自身の待ちだけを数える（起こし直しの後の `deferrals=600` の空鳴りを止めた）・`\![set,choicetimeout,時間]` を compile が読んで選択待ちへ渡す（`0`・負の値で時間切れなし・最後の指定が勝つ・読めない値は既定＋WARN）。再生層 dola は選択待ちの区切りを値で飛ばしも解きもしない |・動かさないクリックで窓の位置を保存しない（wintf の累積器が開始を積んでいない終了を捨て、知らせの種を積んだ順の待ち行列で運ぶ。速いドラッグも開始 → 終了の順に届き、キャラクターは行き先へ置いてから保存）

- 完了 spec 直下エントリ＝**216**（`.kiro/specs/completed/` 直下・2026-10-03 実数え＝フォルダ 215＋`graphics-rendering-stability.md` 1）。⚠ **引き算で導かず毎回実数えする**。
- M1 の持ち越しのうち残るのは `dpi-transition-two-tick-bounce`（開発者が許容）と `zorder-chain-residue` A-2（据え置き）だけ。M-dual は退役。
- 個々の完了行の全文（種別・議題・完了時の所見）は history「2026-10-02 棚卸⑳退避」。

## ウェーブ編成（着手順の正本・2026-10-02 棚卸⑳）

> 各ウェーブは**フルライフサイクル**（要件 → 設計 → タスク → 実装 → `/kiro-complete`＝PR の squash マージ）を完走してから次へ。1 spec ＝ 1 worktree ＝ 1 PR。**同じウェーブに入れるのは、棚卸⑳の実測で触るソースファイルの重なりが 0 の組だけ**。許す重なりは 3 つだけ＝表の末尾への追記（`doc/COMPAT_ARCHITECTURE.md` §8）・生成物（`THIRD-PARTY-NOTICES.md`・`doc/ukadoc-coverage/report/`＝手で直さず作り直す）・steering。文書だけの段は置かない。
> **並び順の決まり（2026-10-03 開発者）**: 優先度は 3 段＝**① バグ ② リリース関係・バルーン関係・動く画像関係 ③ その他**。各ウェーブはまず①②で席を埋め、③は①②と触るファイルが重ならず、①②を後ろへ押さない席にだけ入れる。ウェーブの中の番号は優先順（上から着手）。

| Wave | ユニット（優先順） | 開始コマンド | 編成根拠（触るファイル）|
|---|---|---|---|
| **C1**（7 本並走・**バグ 5 本＋優先 2 本**＝バルーンの列と MCP の先頭） | ① `drag-click-without-move`（バグ・✅ 10-03 完了） ∥ ② `choice-timeout-directive`（バグ・✅ 10-03 完了） ∥ ③ `restart-chain-finalize-stall`（バグ・✅ 10-03 完了） ∥ ④ `status-execution-states`（バグ・潜在） ∥ ⑤ `release-package-versioned`（バグ＝道具の後片付けを合流・配布の列の先頭） ∥ ⑥ `emo-text-file-split`（文字とバルーンの列の先頭） ∥ ⑦ `mcp-server-core`（MCP の土台・依存を足す 1 本） | `/kiro-start areka-P0-<名>` | ①＝`crates/wintf/src/ecs/{window_proc,drag}/`・`crates/areka/src/placement/` のテスト。②＝`crates/areka-sakura/src/compile.rs`。③＝`crates/areka/src/emo2_boot/frame/drain_resnap.rs`。④＝`crates/areka-kanade/src/status.rs` と kanade への届け口・`crates/areka/src/emo2_boot/` の届け元（`user_break_cue.rs`・`balloon_visibility_phase.rs` など）・`crates/areka/src/update/desk.rs`。**④ は ⑥ の 6 本（`emo2_boot/balloon_visibility.rs`・`input_events/balloon.rs` ほか）・`emo2_boot/mod.rs`・`input_events/mod.rs`・③ の `drain_resnap.rs` に触らない**（要るなら止めて報告）。⑤＝`tools/package-alpha.ps1`・`crates/areka/src/boot_config.rs`。⑥＝`crates/areka-emo-text/src/{actor,layout,viewbox,viewbox_draw}.rs`・`input_events/balloon.rs`・`emo2_boot/balloon_visibility.rs` とモジュールの宣言。⑦＝新規 `crates/areka-mcp/`・`crates/areka/Cargo.toml`（1 行）・`crates/areka/src/main.rs`（`fn main()` の 2 か所）・`Cargo.lock`・`THIRD-PARTY-NOTICES.md`（生成物）。④⑥は `main.rs` に触らない |
| **C2**（6 本並走・C1 の後・**優先 4 本＋その他 2 本**） | ① `shell-balloon` ∥ ② `release-ci-workflow`（配布） ∥ ③ `crates-io-publish`（配布） ∥ ④ `mcp-tool-entrances`（MCP の入り口 10 本） ∥ ⑤ `translate-pipeline`（その他・C3 の `balloon-lifecycle-events` の前提） ∥ ⑥ `install-companion-canon`（その他） | `/kiro-start areka-P0-<名>` | ①＝シェルのパーサ・`areka-seriko/src/actor.rs`・emo-text・emo-present の差し込み口・`emo2_boot/`・`input_events/`。②＝`.github/workflows/release.yml`。③＝各 `Cargo.toml` の欄（依存は変えない・C1 で入った `areka-mcp` の `publish` も決める）・`.github/workflows/crates-io.yml`・`dist/README.txt`・`README.md`。④＝`crates/areka-mcp/src/tools/`・新規 `crates/areka/src/mcp/`・`ghost_session.rs`・`main.rs`。⑤＝`crates/areka-kanade/src/`（`schedule/steady.rs` の分割を含む）・`areka-sakura/src/sysvar.rs`。⑥＝`crates/areka-nar/src/`・`crates/areka/src/install/terms.rs`・`crates/areka-ghost/src/catalog.rs`・`crates/areka/src/boot_config.rs`。①は `emo-text-file-split` と `status-execution-states`（どちらも `emo2_boot`）の後、②③⑥は `release-package-versioned` の後、④は `mcp-server-core` の後。**④ は `Cargo.toml` を触らない**（③ と共有しないため・要るなら止めて報告）。`install-live-target-hazards` は ③ と `dist/README.txt` を分け合うので C4 の候補 |
| **初回リリース**（C2 の後・spec でなく手順） | `release-cycle` の初回（`v0.0.2`）＝版を上げる PR → squash マージ → 手元で crates.io へ初回の公開と Trusted Publishing の設定 → タグ → Actions が x64／arm64 の zip と Release | `/kiro-impl areka-P0-release-cycle` | 初回だけは `/kiro-start areka-P0-release-cycle` で要件〜タスクを作ってから。以後は毎回 `/kiro-impl` |
| **C3**（8 本・**予定**＝C2 の着地で brief が動くので、着手の前に触るファイルを照合し直す） | ① `winget-manifest-submission`（配布・初回の手提出＝Release の実在が要る） ∥ ② `balloon-font-file` ∥ ③ `surface-element-nesting` ∥ ④ `balloon-lifecycle-events` ∥ ⑤ `animated-image-decode`（動く画像の土台・依存を足す 1 本） ∥ ⑥ `mcp-get-property` ∥ ⑦ `mcp-expression-table` ∥ ⑧ `mcp-log-history` | `/kiro-start areka-P0-<名>` | ①＝`dist/winget/`・`.github/workflows/winget.yml`・`dist/README.txt`・`README.md`。②＝emo-text の `draw_catalog.rs`・`draw.rs`・`draw_metrics.rs`・`actor_decoration.rs`・`emo2_boot/frame/attach.rs`。③＝`crates/areka-parsers/src/shell/{model,decode}.rs`・`areka-emo-atlas/src/manifest.rs`・`areka-emo-compose`・`areka-seriko`・`areka-emo-present/src/cache.rs`。④＝kanade の `schedule/{steady,events}.rs`・`emo2_boot/{talk_lifecycle.rs, balloon_visibility 系}`・`input_events/` のバルーン。⑤＝`crates/areka-emo-atlas/src/{decode.rs,decode/,lib.rs,table.rs}`・`crates/areka-emo-atlas/Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md`（生成物）＝**`AtlasKey` と `manifest.rs` を変えない設計で**（③ と共有 0）。⑥⑦⑧＝`mcp-tool-entrances` の設計が固定する「自分のツールのファイル」と、⑥ `areka-ghost/src/runtime.rs`・⑦ 表情の表・⑧ tracing の履歴。**照合の要点**＝②と④は同じ `emo2_boot` の中の別のファイル・②と③は emo-text と compose の境目・⑥〜⑧は `mcp-tool-entrances` の design の干渉台帳で確かめる。8 本が多ければ ⑥〜⑧ から C4 へ回す |
| **C4 の候補**（次の棚卸で組む） | バグ＝C3 までに見つかったもの ／ 優先＝`animated-image-playback`（`animated-image-decode`＋`surface-element-nesting` の後）・`anchor-tag-canon` の働きの側（`balloon-font-file` と `choice-timeout-directive` の後）・`balloon-canon-residue`・`mcp-kanade-tools`・`mcp-dump-images`・`mcp-reload`・`mcp-stdio-bridge`（→ 最後に `mcp-strict-errors`） ／ その他＝`install-live-target-hazards`・`coverage-roadmap-refresh`・`property-query-channels`・`popup-menu-residue`・`sakura-time-critical`（`status-execution-states` の後） | — | 同じ決まりで バグ → 優先 → その他 の順に席を埋める |
| **保留** | `tick-gate-adoption` | — | 「長い試行はしない」と両立する短い A/B の測り方を先に組む。計測を汚すので他と並べない |

### 直列の列（同じ列の spec は同時に走らせない・列が違えば並べられる）

棚卸⑳の再測定で、**同じ場所へ行を足す spec の群れ**が 5 つ見つかった。ファイルを分割しても重なりは消えない（同じ表・同じ関数へ足すため）。C3 以降のウェーブは、各列の先頭から 1 本ずつ取り、取った組の接触ファイルを照合してから組む。

| 列 | 重なる場所 | 並び（先頭から） |
|---|---|---|
| **文字とバルーン** | `crates/areka-emo-text/src/`（`state.rs` の `CueCommand::Custom` の腕・実行時の状態の構造体）・`crates/areka-parsers/src/balloon/{model,parse}.rs`・`crates/areka/src/input_events/` のバルーン | `emo-text-file-split`（C1）→ `shell-balloon`（C2）→ `balloon-font-file`（C3）→ `anchor-tag-canon`（働きの側・優先度 高）→ `text-typesetting` → `talk-fast-forward` → `balloon-markers` → `text-ruby` → `text-reveal-fade` → `balloon-scroll-fade` → `text-align-shadow-canon` → `choice-marker-styling` → `anchor-tag-canon`（装飾）→ `text-reveal-dance`（夢） |
| **シェルの element** | `crates/areka-parsers/src/shell/{model,decode}.rs`・`crates/areka-emo-compose/`・`crates/areka-emo-atlas/src/manifest.rs` | `shell-balloon`（C2）→ `surface-element-nesting`（C3）→ `animated-image-playback`（`animated-image-decode`＝C1 も前提・C4 の候補）→ `balloon-element-order` → `surfaces-basepos`（据え置き） |
| **kanade の進行** | `crates/areka-kanade/src/schedule/{steady,events,change}.rs`・`msg.rs` | `translate-pipeline`（C2）→ `balloon-lifecycle-events`（C3・項目 8・10＝優先の段なので前へ）→ `property-query-channels` → `network-update-canon-order` → `mcp-kanade-tools` → `makoto-dll-host` |
| **台本のコンパイル** | `crates/areka-sakura/src/compile.rs` | ~~`choice-timeout-directive`（C1）~~ ✅ 10-03 → `anchor-tag-canon`（働きの側）→ `talk-fast-forward` → `sakura-time-directives`（残り） |
| **配布と公開** | `tools/package-alpha.ps1`・`.github/workflows/`・`Cargo.toml` 群・`dist/README.txt`・`README.md` | `release-package-versioned`（C1）→ `release-ci-workflow` ∥ `crates-io-publish`（C2・workflow のファイルを分けて共有 0）→ `release-cycle` の初回（`v0.0.2`）→ `winget-manifest-submission`（C3）→ `release-code-signing`（任意）。**開発者「インストーラー関係は優先リリースしたい」（10-02）＝各ウェーブに 1〜2 本ずつ必ず入れる** |
| **`emo2_boot` の結線** | `crates/areka/src/emo2_boot/{mod,ghost_switch,consumer_ledger}.rs`・`frame/{attach,switch}.rs` | `status-execution-states`（C1・バグ）→ `shell-balloon`（C2）→ `balloon-canon-residue`（項目 2 から）→ `property-query-channels` → `currentghost-property-tree` → `property-catalog-lists` → `network-update-canon-order` |

- **依存を足す spec は 1 ウェーブに 1 本**（`Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`tech.md` が重なる）: `mcp-server-core`（C1）・`animated-image-decode`（C3）・`mcp-stdio-bridge`（C4 の候補）。
- **網羅台帳をまとめて書き換える `coverage-roadmap-refresh` は、台帳の行を直す spec と同じウェーブに置かない**。
- **列に属さず、いつでも単独で取れる**: `coverage-roadmap-refresh`・`zorder-property`・`popup-menu-residue`。

**保存義務（据え置き）**: 既存の終了経路（右クリックメニューの「終了」→ `OnClose` の握手 → `ghost_quit`）の決定論テストを 1 本も落とさない。実機サインオフの「絶対パス起動」（argv 上書き）を残す。

## spec 台帳（brief を持つ 56 本・2026-10-02 棚卸⑳の 57 本から完了 2 本を外し、10-03 に 1 本足した）

> **spec は名前で呼ぶ**（2026-09-26 開発者指示）: 報告・brief・コミット・PR で spec を指すときは spec 名（`areka-P0-` は省略してよい）を書く。「#数字」は `PR#185` の形の PR 番号にだけ使う。古い文書に台帳番号が出てきたら、その時点の表（history）で名前へ読み替える。
> **段＝優先度の 3 段**（2026-10-03 開発者「1 バグ・2 リリース関係・バルーン関係・アニメーション画像関係・3 その他」）: **バグ**＝1 段目／**優先**＝2 段目（配布と公開・文字とバルーンの列・シェルの element の列・動く画像・バルーンのイベントと残件・**SSP MCP の移植 10 本**＝10-03 に開発者が追加）／**その他**＝3 段目（ゴーストのインストール・kanade・プロパティ・道具）／**据え置き**＝当面着手しない（理由は行に）／**保留**。どのウェーブに居るかは「並び」の列。完了した spec はこの表に置かない（完了サマリと history）。
> **段の数え方**: `awk '/^\| spec（`areka-P0-` 省略）/{f=1;next} f&&/^\|/{print} f&&!/^\|/{f=0}' .kiro/steering/roadmap.md | awk -F'|' 'NR>1{gsub(/ /,"",$3);print $3}' | sort | uniq -c`（2026-10-03 の数え＝バグ 3・優先 32・その他 13・据え置き 7・保留 1 ＝ 56・`restart-chain-finalize-stall`・`choice-timeout-directive`・`drag-click-without-move` の完了で外し、`drag-cancel-borrow-miss`・`sakura-time-critical` の起票で足した）
> **規模**は棚卸⑳の再測定の見立て（タスク数）。**上限は 1 spec 20 タスク**。「要件で切る」と書いた行は、着手のときに要件の段で切り出す（先に起票しない＝spec 工場の禁止）。**Fable**列＝要件定義を Fable で起動したセッションで進めることを勧める（○）か、Opus で足りる（−）か。各 brief の末尾「2026-10-02 棚卸⑳の再測定」が、崩れた前提・触るファイル・議題の正本。

| spec（`areka-P0-` 省略） | 段 | 何をするか | 規模 | 並び | 前提（先に着地） | Fable |
|---|---|---|---|---|---|---|
| `drag-cancel-borrow-miss`（**10-03 起票**・`drag-click-without-move` の完了時に発見） | バグ | wndproc のハンドラが World を借りられないとき、取り消しの終了を積まずに累積器の「ドラッグ中の対象」が残る穴を塞ぐ（再入の条件・実機は未観測） | S（4〜7） | C1 の後の空き席 | なし | − |
| `status-execution-states` | バグ | SHIORI へ渡す `Status` に `online`・`nouserbreak`・`balloon` を載せる（今は載らず、更新中や割り込み禁止の区間に雑談が割り込みうる＝潜在） | S〜M（5〜8） | **C1-④** | なし（同じ C1 の `emo-text-file-split` の 6 本と `drain_resnap.rs` に触らない約束） | − |
| `release-package-versioned`（**10-02 起票**・配布と公開・**`package-check-temp-cleanup` を合流**） | バグ | 先頭で `-Check` の展開先と記録を `target\` の下へ移して片付ける（道具のバグ）。続けて配布 zip を版入りの固定の名前（`areka-{版}-{x64\|arm64}.zip`）と SHA256 で作る・arm64 の zip・リンク経由の起動でも exe の本当の場所から根を引く | M（11〜17） | **C1-⑤** | なし | − |
| `install-live-target-hazards` | その他 | 表示中のシェル・使用中のバルーンへの上書きと、起動中のゴーストへ入れる途中の Windows の終了を**実測してから**扱いを決める | S〜M（8〜14） | C4 の候補（`dist/README.txt` を C2・C3 の配布と分け合う） | なし | ○ |
| `install-companion-canon` | その他 | 同梱インストールの `install.txt` の読み方を ukadoc に揃える 4 点＋起動時の同梱バルーンの問い | S〜M（8〜12） | **C2-⑥** | `release-package-versioned`（`boot_config.rs`） | ○ |
| `emo-text-file-split`（**10-02 起票**） | 優先 | 文字まわりの上限に張り付いた 6 ファイルを、振る舞いを変えずに分ける（後続 10 本以上の「分割が先」をここで済ませる） | S（5〜8） | **C1-⑥** | なし | − |
| `mcp-server-core` | 優先 | 新クレート `areka-mcp`＝公式 SDK `rmcp` で `127.0.0.1` の MCP サーバ（ツール 0 本・既定ポート 9821） | M（11〜14） | **C1-⑦** | なし | ○ |
| `shell-balloon` | 優先 | シェルの絵の中に台詞を書く（areka 独自）。`balloon.名前`ブレス・element定義 `elementN,balloon,名前,X,Y`・行き先は「スコープ × 名前」 | L（15〜19） | **C2-①** | `emo-text-file-split` | ○ |
| `translate-pipeline` | その他 | `OnTranslate` の往復（里々・YAYA の辞書が文を直す段）。今は直す前の文が出ている | M（14〜18） | **C2-⑤** | なし | ○ |
| `mcp-tool-entrances` | 優先 | MCP のツール 10 本の定義を SSP と逐語一致・World への橋・9 本はダミー | M（13〜17） | **C2-④** | `mcp-server-core` | ○ |
| `release-ci-workflow`（**10-02 起票**・配布と公開） | 優先 | タグ `v*` が押されたときだけ動く GitHub Actions（x64／arm64 の zip → SHA256 → GitHub Release を公開）。テストの門は手元のまま | S（6〜9） | **C2-②** | `release-package-versioned` | − |
| `crates-io-publish`（**10-02 起票**・配布と公開） | 優先 | 部品のクレートの `publish` を開けて同じ版で crates.io へ（自分の workflow `crates-io.yml`・Trusted Publishing・初回は `release-cycle` の初回に手元から）。`cargo install` は利用者向けの入れ方にしない | S〜M（8〜12） | **C2-③** | `release-package-versioned`・`mcp-server-core`（どちらも C1） | − |
| `release-cycle`（**10-02 起票**・配布と公開・**繰り返し spec**） | 優先 | 開発者が実装を打ったときだけ版を +0.0.1 → PR → squash マージ → タグ。初回が `v0.0.2`（crates.io の初回の公開もこの中） | XS（手順 7） | **初回リリース**（C2 の後） | `release-ci-workflow`・`crates-io-publish` | − |
| `winget-manifest-submission`（**10-02 起票**・配布と公開） | 優先 | winget の名乗り `Areka.Areka.Portable`（zip＋portable・x64 と arm64）・初回の手提出・以後は winget-releaser。`Areka.Areka` はインストーラー版のために空けておく | S（6〜9） | **C3-①**（予定） | `release-cycle` の初回 | − |
| `balloon-font-file` | 優先 | `font.name`・`\f[name]` のフォントファイル | M（7〜10） | **C3-②**（予定） | `shell-balloon` | − |
| `anchor-tag-canon` | 優先 | `\_a` のリンク（今は押しても何も起きない＝優先度 高）と装飾 16。**要件で「働き」と「装飾」に切る・働きが先** | 20 超 → 切る | 文字とバルーンの列・コンパイルの列 | `shell-balloon`・`choice-timeout-directive` | ○ |
| `text-typesetting` | 優先 | 禁則・ぶら下げ・縦中横・字の向き（areka 独自） | M〜L（13〜18） | 文字とバルーンの列 | `shell-balloon` | ○ |
| `talk-fast-forward` | 優先 | クリックでの早送り・`\x`／`\x[noclear]`・`clickwaitmarker.*` | M（13〜18） | 文字とバルーンの列・コンパイルの列 | `shell-balloon` | ○ |
| `balloon-markers` | 優先 | `arrow*` と手動スクロール・`onlinemarker`・`number`・SSTP の印（表示だけ）・装飾の系列 | M〜L（14〜19） | 文字とバルーンの列 | `shell-balloon` | − |
| `text-ruby` | 優先 | ルビ `\![text,ruby,親文字,ルビ]`・`line_height`・`letter_spacing` | M〜L | 文字とバルーンの列 | `text-typesetting` | ○ |
| `text-reveal-fade` | 優先 | 1 字ずつの表示で字が透明から不透明へ変わる（既定は無効） | M（8〜12） | 文字とバルーンの列 | `shell-balloon` | − |
| `balloon-scroll-fade` | 優先 | 自動スクロールで押し出される行のフェード（既定は無効） | S〜M | 文字とバルーンの列 | `balloon-markers` | − |
| `text-align-shadow-canon` | 優先 | `\f[align]`／`\f[valign]`・影。**20 を超えそうなら「影」と「寄せ」に切る** | M（20 に近い） | 文字とバルーンの列 | `text-typesetting` | ○ |
| `choice-marker-styling` | 優先 | `\f[cursor*]` 10 項目と本物の下線 | S | 文字とバルーンの列 | `text-align-shadow-canon`（同じファイル） | − |
| `surface-element-nesting` | 優先 | element定義でサーフェスを部品として置く（数字だけ＝サーフェスの番号）・子の時計は独立 | M〜L（16〜22） | **C3-③**（予定） | `shell-balloon` | ○ |
| `animated-image-decode` | 優先 | 動く GIF・APNG・WebP の全コマと待ち時間を読む（`image` クレートを本番へ＝承認が要る） | M（11〜15） | **C3-⑤**（予定） | なし | ○ |
| `animated-image-playback` | 優先 | 動く絵を子サーフェスへ分解して自動で回す・`import`・interval `always` | M〜L（15〜20） | C4 の候補・シェルの element の列 | `animated-image-decode`・`surface-element-nesting` | ○ |
| `balloon-element-order` | 優先 | `balloon` の element定義を並び順どおりの重ね順で描く | M | シェルの element の列 | `shell-balloon`・`animated-image-playback` | ○ |
| `property-query-channels` | その他 | `\![get/set,property]`・`%property[…]`。**`\![embed]` は要件で切り離す**。許可の表の迂回を最初に決める | M（12〜16） | kanade の列・`emo2_boot` の列 | `translate-pipeline` | ○ |
| `balloon-lifecycle-events` | 優先 | `OnBalloonClose`／`OnBalloonTimeout`／`OnBalloonBreak`（項目 8・10 を先に。項目 7 の `balloontimeout` は `sakura-time-directives` を待つ） | M | **C3-④**（予定） | `translate-pipeline`・`emo-text-file-split` | − |
| `network-update-canon-order` | その他 | 更新のイベントを ukadoc の発生順序に揃える（MD5 の取り直し・台詞の終わりを待つ・総括の 2 段） | M（10〜15） | kanade の列・`emo2_boot` の列 | なし（列の順番待ち） | ○ |
| `balloon-canon-residue` | 優先 | 残り 5 件（項目 2＝面の偶奇で左右のバルーンを選ぶ、を先に。項目 4 `\![reload,balloon]` は切替の仕組みに載せる） | M | C4 の候補・`emo2_boot` の列 | `shell-balloon` | − |
| `currentghost-property-tree` | その他 | `currentghost.*` の約 65 値。**要件で `balloon.scope` の 19 項目を先に切る** | 20 超 → 切る | `emo2_boot` の列 | `property-query-channels`（読む道）か `mcp-get-property` | − |
| `property-catalog-lists` | その他 | `system.*` と一覧（`ghostlist`・`balloonlist`＝列挙は α で実在）。**要件で 2 つに切る** | 20 超 → 切る | `emo2_boot` の列 | `currentghost-property-tree` | − |
| `zorder-property` | その他 | `currentghost.seriko.zorder` の読み書き | S | 単独で取れる | 書く側は `property-query-channels` | − |
| `sakura-time-directives` | その他 | 残りの時間の指令（`quicksection`・`balloonwait`・`balloontimeout`）。C・D 群は消費する者が現れるまで置く | 20 超 → 切る | コンパイルの列 | `choice-timeout-directive`・`talk-fast-forward` | ○ |
| `sakura-time-critical`（**10-03 起票**・`emo-text-file-split` の実機の観測から） | その他 | 台本の `\t`（タイムクリティカル）を読み、台本の終わりか中断・選択まで、マウス系などの通知を止めて `Status` に `timecritical` を載せる（今は読まない＝`\t` を書いたメニューもなでなでの返事に置き換わる） | S〜M（6〜10） | コンパイルの列・kanade の列 | `status-execution-states` | ○ |
| `makoto-dll-host` | その他 | MAKOTO/2.0 の DLL を掛ける。**要件で「ホストと鎖」と「load/unload/reload」に切る** | 20 超 → 切る | kanade の列 | `translate-pipeline` | ○ |
| `mcp-stdio-bridge` | 優先 | Claude Desktop 用の stdio ⇔ HTTP 中継 exe | S（7〜10） | C4 の候補（依存の席を C3 の `animated-image-decode` が使う・`dist/README.txt` を C3 の winget と分け合う） | `mcp-server-core`・`package-check-temp-cleanup` | − |
| `mcp-get-property` | 優先 | `get_property` | S | **C3-⑥**（予定） | `mcp-tool-entrances` | − |
| `mcp-kanade-tools` | 優先 | `get_status`・`sakurascript`・`raise_event`（許可の表の迂回は `property-query-channels` と一度で設計） | M〜L | C4 の候補（C3 の `balloon-lifecycle-events` と kanade を分け合う） | `mcp-tool-entrances` | ○ |
| `mcp-expression-table` | 優先 | `get_expression_table` | S〜M | **C3-⑦**（予定） | `mcp-tool-entrances` | ○ |
| `mcp-log-history` | 優先 | `get_log`（tracing の履歴） | M | **C3-⑧**（予定） | `mcp-tool-entrances` | ○ |
| `mcp-reload` | 優先 | `reload` と台本の `\![reload,…]` | M〜L | C4 の候補（`mcp-kanade-tools` と kanade を分け合いうる） | `mcp-tool-entrances` | ○ |
| `mcp-dump-images` | 優先 | `dump_surface`・`dump_balloon` | M | C4 の候補（C3 の `balloon-font-file` と emo-text の境目が近い） | `mcp-tool-entrances` | ○ |
| `mcp-strict-errors` | 優先 | `strict`（不在の面・未知のタグなどをエラーログへ） | M | MCP の最後 | `mcp-log-history`・`mcp-kanade-tools` | ○ |
| `coverage-roadmap-refresh`（**10-02 に覚え書き `ukadoc-coverage-custody` を合流**） | その他 | 網羅台帳の手書きの数と持ち主が黙って偽になるのを止める（検査が `completed/` を走査・128 行の持ち主の付け替え・実在しない仕様名 107 項目） | M（14〜18） | 単独で取れる（台帳を直す spec と同じウェーブに置かない） | なし | − |
| `popup-menu-residue` | その他 | メニューの残件 4（記録の文言・テストの穴・World 借用中の `ShellExecuteW`）。次に `menu/` を触る spec へ相乗りが安い | S（6〜8） | 単独で取れる | なし | − |
| `surfaces-basepos` | 据え置き | surfaces.txt の `point.basepos`。宣言するシェルが要るまで始めない | S | シェルの element の列の最後 | — | − |
| `property-ipc-transport` | 据え置き | 台本を通さないプロパティの読み取り。**最初の 1 段（調べて裁定）で終わる形へ縮める** | S（裁定まで） | — | — | ○ |
| `dpi-transition-two-tick-bounce` | 据え置き | 拡大率の切替で位置と大きさが 1 コマずれる（開発者が許容）。着手するなら最初に測り直し、跳ねが無ければ取り下げ | S〜M | — | — | ○ |
| `zorder-chain-residue` | 据え置き | 間欠赤のテスト族と文書。09-11 以降 main で赤 0 件＝先回りしない | M | — | — | ○（A 群） |
| `text-reveal-dance` | 据え置き | 字が現れるときだけ跳ねる・揺れる（夢・開発者が望んだときだけ） | M〜L | — | `text-reveal-fade` | ○ |
| `emo-text-canon-residue` | 据え置き | **取り下げ予定・着手しない**。残っていた 1 件（折り返しの警告にバルーンの名前を出す）は `shell-balloon` が引き取った。フォルダを消すと網羅台帳の整合検査（`roadmap-draft.md` の `[[spec]]` の表）が赤くなるので、表から外すのと一緒に `coverage-roadmap-refresh` が片付ける | — | — | — | − |
| `release-code-signing`（**10-02 起票**・配布と公開・任意） | 据え置き | SignPath Foundation（オープンソース向け・無償）に申請し、受理されたら CI で署名する。winget に署名は要らないので急がない | S（5〜8） | 配布と公開の列の最後 | `release-ci-workflow`・数回のリリースの実績 | − |
| `tick-gate-adoption` | 保留 | 既定で切の門を入れるか（待機中の CPU 17〜22% → 3% 未満の見込み）。短い A/B の測り方を先に組む | M〜L | 保留 | — | ○ |

## 覚え書き（brief なし・引受先が消えたまま忘れないための一覧）

> brief を書くほど固まっていない宿題。**着手の判断は棚卸で行う**（格上げするときは `/kiro-discovery`）。登記時の全文（数え方つき）は history「2026-10-02 棚卸⑳退避」。

**製品の穴**

- **`present-write-coherence` の未達 40 件**（性能・L）。完了仕様が自ら「引受先なし・新規仕様の起票が必要」と書いた残量＝`visualize_to_write_us` が上限 16,667µs の 12.6〜18.4 倍・32 窓中 0 窓が上限以下。開発者裁定で「未達のまま GO」済み。着手前に測り直す。
- **正典語彙の孤児 2 件**（S）。`font.outline`（白抜き）の**描画**の引受先／「スタイルシートのキーワード」の持ち主。
- **配布物を束ねる／作る側の 3 件**（XS〜S）。`install.txt` の `type,package` を解く・`developer_options.txt`（配布物を作る道具が読む）。
- **壊れたゴーストを表示し続ける形**（互換・M）。SSP は切替先の SHIORI が死んでいても切替を成功扱いにして表示し続ける。areka は「既定ゴーストへ戻して `OnBoot` の Ref6＝`halt`」を採った（09-26 裁定）。あるべき姿としては残る。
- **SSTP の受信**（L）。`balloon-markers` の `sstpmarker`／`sstpmessage` が実際に画面に出るのはこれが入ってから。起票するときは同 spec の縮退の口を埋める。
- **更新のオプションと `OnUpdateCheck*`**（10-02 登記）。完了 `network-update` が「α 後」とした更新のオプション（`checkonly` など）・`OnUpdateCheck*` の 4 語・`other_homeurl_override`。`network-update-canon-order` は範囲外と明記していて、持ち主が居ない。
- **3 人目以降のキャラクターの窓**（10-02 登記）。`char{n≧2}` の窓は出ない（説明書の既知の制限）。`popup-menu-residue` は完了済みの `ghost-shell-balloon-switch` へ渡すと書いたままで、持ち主が居ない。
- **説明書の既知の制限のうち持ち主の居ないもの**（10-02 登記・`dist/README.txt`）: 管理者として動かした areka へは投げ込みが届かない／`%TEMP%\areka\download\` に取ってきた書庫が残る／`.update-work` を手で戻す／引数で起動したときは更新が次の起動から効く／ゴーストの指示で取ってくるのは `.nar` だけ／署名なし。仕様としての決めかどうかを含めて次の棚卸で仕分ける。
- **`shell-implicit-surface` の着地で残した 7 件**（09-20 登記・検体 3 体に該当 0 件）: ⑴ `.pna` は非対応のまま（開発者方針）⑵ `use_self_alpha,full`（全面不透明）は未実装 ⑶ 透過の宣言を読む経路が無い（常に `1` 扱い・`(Off, KeyColor)` も未実装）⑷ 全画素が透明になる面の窓・当たり判定が未確認 ⑸ `surfaces.txt` が無い／波括弧 0 個で画像だけのシェルは起動の失敗のまま ⑹ `overlay` 以外の描画メソッドの `element0` を持つ面で画像が土台に使われるずれ ⑺ `sometimes`・`rarely` 以外の間隔の語は動かない（`always` は `animated-image-playback` が入れる）。**同じく引受先なし**: `surface.append` の行に**しか**現れない絵のファイル名は焼かれない。

**潜在の性質（実害は観測されていない）**

- **SHIORI へ渡す置き場所のパスの形**（XS）。`load`／`loadu` へ渡すパスが `…\ghost/master` の形（区切りの混在・末尾の区切りなし）。出どころは `crates/areka-parsers/src/package/resolve_shell.rs` の定数 `GHOST_MASTER`（09-24 の登記は `resolve.rs` と書いていた＝10-02 に訂正）。正典は沈黙。直すなら SHIORI へ渡す 2 つの口（`crates/areka-ghost/src/{shiori_inproc,shiori_wiring}.rs`）だけで揃えるのが影響が狭い。
- **wintf の兄弟の重なり順が描画と当たり判定で逆**（XS〜S）。`visual_hierarchy_sync_system` は先頭の子を最上に描き、`hit_test` は最後の子から調べる。本番は 1 窓に装着が 1 組だけなので表に出ない。**裁定は、子を初めて複数作る `shell-balloon` か `surface-element-nesting` の要件に乗せる**（どちらの順に揃えるか）。訂正（10-02）: 文字の層の差し込み口は、表示中は当たり判定を持つ。
- **`InProc` の SHIORI を外から終わらせる手が無い**。本番は `Helper` だけを選ぶので今日は踏まない。`InProc` を本番に使うときに決める。

**道具と試験**

- **`tools/perf` の実走していない 3 経路**（S）。`invoke-perf-run.ps1` へ `-GhostRoot`／`-BalloonRoot` を渡す実走・`invoke-followup-checks.ps1` の単独起動・`check-quiet.ps1`。**自己検査の赤（終了コード 4）は 10-02 の実行で再現しなかった**（`SELFTEST RESULT ok=9 ng=0`・終了コード 0）。性能改善ループを次に回す前に 3 経路を通す。
- **一度だけ落ちた試験 2 本**（原因未調査・再発したら出力を添えて起票）: `sample-ghost-kit` の展開テストの os error 5（09-26）／i686 の `shiori_proxy::tests::testdll_drop_invokes_courtesy_unload`（09-30）。

## 直接修正候補（spec なし）

> **2026-10-02 棚卸⑳で直した**: 利用条件の文の切り詰めが絵文字の途中で切れる件（`crates/areka/src/install/terms.rs`・書記素クラスタで数える・テスト付き）／`completed/` が抜けて切れていたパス 8 か所／完了した spec を「後続」と書いていたコメント（`install/mod.rs`・`menu/mod.rs`・`doc/COMPAT_ARCHITECTURE.md` §8）／`talk_lifecycle.rs` の予約の持ち主の名前／steering の古い数と状態（`focus.md`・`product.md`・`structure.md`・`tech.md`）。

残っている候補（どれも振る舞いを変えない掃除・**開発者の一声で着手**）:

1. **根の `README.md`・`doc/ARCHITECTURE.md`・`doc/CONSTITUTION.md` の状態の記述が古い**。`areka *(予定)*`・DirectComposition（今は WUC）・`bevy_ecs 0.18`（今は 0.19）・`Taffy 0.9.2`（今は 0.13）・「アルファリリース目標: ぱすたさん」（α は「第三者が管理できる」で完了）・Phase A〜E の表・ツリーが 3 クレート（今は 29）。部分的に直すより状態の節を書き直す規模で、外から見える顔なので**書き直してよいかを開発者が決める**。
2. **段階的な実装の名残のコメントと `#[allow(dead_code)]`**（`crates/areka/src/placement/{mod,persist,spawn,source,config}.rs`・`placement/follow/anchor.rs`・`emo2_boot/{mod,move_cue,spine,spine_conformance_script,hit_region}.rs`・`placement/transition_diag.rs` ほか約 25 か所）。「task N が結線するまで」と書いたまま、もう結線済み。allow を外せるかは `--force-warn dead_code` のビルドで確かめる（examples が `#[path]` で取り込むので要るものが残りうる）。
3. **「M2」を先送り先として書いたコメント 52 行**（`areka-emo-text/src/{canvas,viewbox,writing}.rs` ほか）。M2 は 2026-09-18 に「α」へ決め直されて完了したので意味がずれている。置き換える語（「α 後」か「M3 以降」）は M3 のゴールが決まってからの方が手戻りが無い。
4. **`tech.md` の主要な依存の一覧に無い本番の依存**（`budouy`・`rectangle-pack`・`bitflags`・`async-io`・`async-channel`・`human-panic`）。一覧は網羅を謳っていないので急がない。

取り下げた 1 件を再登記しないこと——判定器（`crates/areka/src/placement/transition_judge_verdict.rs`）の窓ごとの書込上限が見送り窓を除いていないのは**意図どおり**（`completed/areka-P0-dpi-transition-atomicity/mechanism-ledger.md` §13.1・2026-09-24 に開発者が再確認）。

## 生きている決まり（棚卸⑭〜⑲の裁定のうち今も効くもの・全文は history）

1. **着手の優先度は 3 段＝① バグ ② リリース関係・バルーン関係・動く画像関係・MCP 関係 ③ その他**（2026-10-03 開発者・09-20 の「バグ修正 → 優先度の高い機能」を 3 段に細かくした。MCP は同日「複合 spec なので早めに」で ② へ）。③は①②と触るファイルが重ならない席にだけ入れる。バグかどうかは実測で分ける（利用者から見える実害／潜在／テストの穴／正典の追加）。
2. **規模の上限は 1 spec 20 タスク**。超えるものは要件の段で切る。**一度切り出した spec を、上限を少しまたぐ理由でさらに削らない**（09-28）。
3. **並走の物差し＝触るソースファイルの重なり 0**（09-20）。**文書だけの段は置かない**（09-27・要件は前の spec の着地で古くなる）。
4. **エンジンを切り出した spec は網羅台帳に触らない**。台帳の更新は結線の側がまとめて行う（09-20）。
5. **`zorder-chain-residue` の A 群は先回りしない**（main で赤が出たら、その時点で A 群だけを単独で挟む）。
6. **メニューは Win32 標準**・オーナードローと着せ替えメニューは予約のまま。**ゴースト切替はプロセス内**・アプリの寿命は窓の数から切り離す。**HTTP は WinHTTP・MD5 は OS の CNG**。**投げ込みは `WM_DROPFILES`**。**消滅（`\![vanishbymyself]`）は要望が出たら S で切る**。
7. **失敗は既定ゴースト（emo2）の台詞で伝える**（`OnBoot` の Ref6＝`halt`・Ref7＝落ちたゴーストの名前）。メッセージボックスは出さない。**インストールと切り替えは別のイベント**（入れた後の切替を areka は主導しない）。
8. **要件定義・設計のサブエージェントは起動中のモデルを継承し、上位へ上げない**（09-30）。Fable 列は「Fable で起動したセッションを勧める」だけ。
9. **並走できる spec の紹介は「完全並走できるものだけ」・Fable 推奨を先頭に、名前＋一言＋コマンドの短い形**（09-26）。
10. **話の最中（選択待ちを含む）に届いたマウス系の返事は今の話を置き換える＝正典どおり**（2026-10-03・`emo-text-file-split` の実機で「メニューがなでなでの返事に置き換わる」を観測して確かめた。正本は完了 `input-events` の DD-IE-1／DD-IE-2）。止めるのはゴーストの側（`Status` の `talking`／`choosing` を見て黙る）か台本の `\t`。areka の穴は `\t` を読まないことだけ＝`sakura-time-critical`。

## 棚卸⑳の裁定（2026-10-02・main `03e8d7d6`・開発者指示「main が進んだので棚卸＆深掘り＆徹底ブリーフィング。完了ロードマップの整理。バグ修正 → 優先度の高い機能」）

> α 完成宣言（PR#210）の直後の棚卸。brief を持つ 49 本すべてを、サブエージェント 5 系統（Opus）で main `03e8d7d6` に照らして再測定し、結果を各 brief の「2026-10-02 棚卸⑳の再測定」節へ書いた。

1. **本ファイルを書き直した**。完了行 40・棚卸⑭〜⑲の裁定・α のウェーブ表・旧「登記だけの行」を history へ逐語で移し、台帳は brief を持つ行だけにした（202 KB → 約 60 KB）。台帳の列「状態」と、brief を持たない行は表から外した（後者は「覚え書き」節）。
2. **即時修正を実施して閉じた**（「直接修正候補」節の冒頭）。残した 4 件は開発者の一声を待つ。
3. **バグを 2 本起票した**: `choice-timeout-directive`（再測定で見つけた・`sakura-time-directives` から切り出し。正典はミリ秒・`0` か `-1` で時間切れなし、areka は読まずに 30 秒で閉じる）と `restart-chain-finalize-stall`（覚え書きの格上げ・2 回観測。窓の無い区間も見送りを数え続ける）。
4. **土台を 1 本起票した**: `emo-text-file-split`。文字まわりの 10 本以上の brief が「分割が先」と書きながら誰が切るかを決めておらず、任せると同じファイルを別々の形で切る。
5. **`status-execution-states` をバグ（潜在）へ改めた**。出どころが無いとされた 3 状態（`online`・`nouserbreak`・`balloon`）は α で実在するようになり、載せないことで雑談が更新や割り込み禁止の区間に割り込みうる。
6. **覚え書き `ukadoc-coverage-custody` を `coverage-roadmap-refresh` へ合流した**（同じ仕組みの同じファイルを直す。持ち主が完了済みの行は 29 → 128 行に増えていた）。
7. **`emo-text-canon-residue` は取り下げ予定にした**（残り 1 件を `shell-balloon` が引き取る）。フォルダを `_rejected/` へ移すと網羅台帳の整合検査が赤くなる（`roadmap-draft.md` の `[[spec]]` の表が名前を持つ）ことを実際に赤で確かめたので、フォルダは置いたまま、片付けは `coverage-roadmap-refresh` に任せる。
8. **旧 S1・S2 の「並走できる見込み」は外れだった**。`shell-balloon` ∥ `balloon-font-file`・`text-typesetting` ∥ `talk-fast-forward` はどちらもソースを共有する。文字まわりの spec は同じ表・同じ関数へ行を足すので、分割しても並走できない＝「直列の列」として並べた。`animated-image-decode` ∥ `surface-element-nesting` も、brief どおりに作ると `manifest.rs` を共有する（両方が設計で避ければ 0 にできる＝各 brief に書いた）。
9. **20 タスクを超える 6 本は、着手のときに要件の段で切る**（`anchor-tag-canon`・`currentghost-property-tree`・`property-catalog-lists`・`sakura-time-directives`・`makoto-dll-host`・`property-query-channels` の `\![embed]`）。いま起票はしない（先の brief は着地で古くなる）。
10. **Fable で要件定義を勧める spec（C1・C2）**: `mcp-server-core`（rmcp の版・HTTP の土台・tokio の閉じ込め）・`install-live-target-hazards`（測ってから決める・OS の終了）・`install-companion-canon`（正典の読みと安全の検査）・`shell-balloon`・`translate-pipeline`・`mcp-tool-entrances`。Opus で足りる: `drag-click-without-move`・`choice-timeout-directive`・`restart-chain-finalize-stall`・`release-package-versioned`・`emo-text-file-split`・`release-ci-workflow`・`crates-io-publish`・`winget-manifest-submission`。
11. **C1 の想定タスクの合計＝51〜79**（7 本）。**C2＝64〜89**（6 本）。
12a. **追記（同日・開発者「インストーラー関係は優先リリースしたい。なるべくウェーブに含める」）**: 配布と公開の 6 本を C1〜C3 へ組み込んだ。`package-check-temp-cleanup`（同じスクリプトの道具のバグ）を `release-package-versioned` へ合流して C1 へ前倒し（合流後 11〜17 タスク）、`boot_config.rs` を共有する `install-companion-canon` を C2 へ回した。`crates-io-publish` は自分の workflow を持たせて `release-ci-workflow` と C2 で並走させる。C2 の後に `release-cycle` の初回（`v0.0.2`）、C3 の先頭に `winget-manifest-submission`。
12b. **優先度の 3 段で組み直した（2026-10-03・開発者「フェーズ優先度を 1 バグ・2 リリース関係・バルーン関係・アニメーション画像関係・3 その他に。ウェーブを見直して」）**: 台帳の段を 3 段へ改め（バグ 5・優先 22・その他 22・据え置き 7・保留 1 ＝ 57）、ウェーブを組み直した。C1＝バグ 5 本（`status-execution-states` を C3 の候補から前倒し＝同じ C1 の `emo-text-file-split` の 6 本と `drain_resnap.rs` に触らない約束つき）＋`emo-text-file-split`＋`animated-image-decode`（依存を足す席を `mcp-server-core` から譲り受けた）。C2＝`shell-balloon`・`release-ci-workflow`・`crates-io-publish`＋その他の `translate-pipeline`・`install-companion-canon`（どちらも優先の spec と触るファイルが重ならない空き席）。C3（予定）＝`winget-manifest-submission`・`balloon-font-file`・`surface-element-nesting`・`balloon-lifecycle-events`＋その他の `mcp-server-core`。その他の `install-live-target-hazards`・`mcp-tool-entrances` は C4 の候補へ下げた。`crates-io-publish` の前提から `mcp-server-core` を外した（`areka-mcp` の `publish` は後から入る `mcp-server-core` が自分で決める）。kanade の列は `balloon-lifecycle-events` を `property-query-channels` の前へ。文字とバルーンの列の全体（`text-typesetting` ほか）を「バルーン関係」に数えた。想定タスクの合計＝C1 43〜65・C2 51〜70。
12c. **MCP を ② へ上げた（同日・開発者「MCP 関係って複合 spec だったから早めに着手入れてほしい」）**: SSP MCP の移植 10 本の段を「優先」へ。`mcp-server-core` を C1-⑦ へ戻し、依存を足す席を譲った `animated-image-decode` は C3 へ（`animated-image-playback` は `surface-element-nesting`＝C3 を待つので、動く画像の列は 1 段も遅れない）。`mcp-tool-entrances` は C2-④（`Cargo.toml` を触らない約束）。個別のツールのうち kanade と emo-text から遠い `mcp-get-property`・`mcp-expression-table`・`mcp-log-history` を C3 へ、`mcp-kanade-tools`・`mcp-dump-images`・`mcp-reload`・`mcp-stdio-bridge` を C4 の候補、`mcp-strict-errors` を最後に。`crates-io-publish` の前提に `mcp-server-core`（C1）を戻した（`areka-mcp` の `publish` を C2 で決める）。数え＝バグ 5・優先 32・その他 12・据え置き 7・保留 1 ＝ 57。
12. **開発者に決めてほしいこと**: ⑴ M3 のゴール ⑵ 根の `README.md` ほかの状態の節を書き直してよいか（直接修正候補 1）。どちらも C1 の着手を止めない。

## 着手手順

- 着手は該当の brief を読んで `/kiro-start <名>` へ直行する。brief の file:line は起票時値＝**着手時に必ず再検証**（C1・C2 の brief は main `03e8d7d6` で再測定済み。それ以外は崩れた前提と触るファイルの見立てまで）。
- 新しい課題の起票は `/kiro-discovery`（再入）で just-in-time。`/kiro-spec-batch` は使わない（一括＝工場化）。ウェーブをまたぐ合流の判断は別セッションで一括。
- **ukadoc 台帳の `owner`**: 各 spec の要件の段で台帳（`doc/ukadoc-coverage/ledger/`）の `owner` に登記し、同時に `roadmap-draft.md` の `owner_count` を追随させる。完了のときに状態を実装済みへ直す（直さないと `coverage-roadmap-refresh` の数え直しの対象が増える）。
- 完了のとき（`/kiro-complete`）は、本ファイルの台帳からその行を消し、「完了サマリ」へ 1 行足す。

## 制約

- Rust 2024・マルチクレート（一覧は structure.md）。**32bit 可搬性の適用範囲＝host-32 系（`shiori-host32-*`／`shiori-abi`）のみ**。wintf/areka 本体は x64＋arm64 ネイティブ。
- 透過は WUC/DComp GPU 合成上のクリックスルー機構（`WS_EX_TRANSPARENT` 動的トグル＋αマスク）で成立（ULW は撤去済み）。SHIORI 内部唯一 ABI=`IShiori`(COM, HSTRING/UTF-16)。過去互換は 32bit Rust ホスト。
- 設計判断の変更は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md) を正本として更新。
- 実機運転の定石: 起動に渡す**検体の絶対パスは `cargo run -p sample-ghost-kit --bin nar-sample-path -- emo2` が印字する**（`folder=` がゴースト・`balloon.<名>=` が同梱バルーン。呼ぶたびに `manual/<検体>/` を作り直すので、**2 つの端末で同時に呼ぶと互いの木を消す**＝1 度印字してから使う）。絶対パス起動（相対は pasta.dll LOAD 失敗）・i686 helper を先ビルド・`AREKA_APP_SMOKE_EXIT_MS` 有界自動終了＋`RUST_LOG` grep（記憶 areka-real-machine-signoff-bounded-auto-exit）。自動終了は強制終了の経路で終了挨拶を経ないので、終了挨拶を確かめる走行では自動終了を上限に留め、終了はキャラ窓の右クリックメニューの「終了」で求める（2026-09-17 host32-window-thread-pump の裁定は Ctrl＋左ダブルクリックだったが、2026-09-19 に `areka-P0-popup-menu-minimal` がその入口を取り除いた。強制退避の Ctrl＋Shift＋左ダブルクリックは残る）。
- 常時テストは x86 を避け偽境界で純 x64 決定論（記憶 prefer-x64-fake-boundary-tests-not-x86）。**ネットへ出るテストを常時テストに入れない**（`network-update` は偽 `HttpFetch`）。

## テーマ別の決めごと（起票のときに開発者が確定した中身・要件の段で覆してよい）

> 4 つのテーマ（SSP MCP の移植・シェル内バルーン・動く画像・文字の現れ方）と α の持ち越しについて、起票のときに決めたことを残す。**着手の並びは「ウェーブ編成」節が正本**で、起票時の「ウェーブ」の小節（M1〜M4・S1〜S4・V1〜V2・R1）は棚卸⑳の再測定で組み直したので history へ移した（旧 S1・S2 の並走の見込みは外れ・文字まわりは `emo-text-file-split` を先頭に直列）。「Existing Spec Updates」の小節（各 brief への追記）はすべて済み。

### SSP MCP の移植（α 後・2026-09-29 `/kiro-discovery` で起票）

> 開発者指示（2026-09-29）「ssp mcp tool の完全移植のための spec 群を立ち上げて。実装は α リリースの後。areka の 127.0.0.1 の適当なポートでサーバを開く形。基本実装 → 空のダミー関数を置いて入り口だけ全部整備 → 個別のコマンド実装。平行開発しやすいように spec 分割」。**事実の正本**は [doc/ssp-mcp/survey.md](../../doc/ssp-mcp/survey.md)（SSP 2.9.05 への実測・ツール定義の逐語は同フォルダの JSON）。ukadoc MCP には MCP の節が索引されていない＝出典は ukadoc の「その他の機能」と SSTP 仕様と実測。

- **決めたこと（起票時・要件の段で開発者が覆してよい）**: ⑴ 本体が 127.0.0.1 で HTTP を待ち受け、Claude Code・Cursor は HTTP で直接つなぐ。**Claude Desktop 用に SSP の `mcp.exe` 相当の中継 exe を `mcp-stdio-bridge` で作る**（2026-09-29 開発者判断。Desktop の `claude_desktop_config.json` は `command` の欄が必須で `url` を書けない〔Desktop 2.9939.4.0 の検査関数で確認〕・コネクタは Anthropic のクラウドからつなぐので 127.0.0.1 へ届かない〔公式の案内〕）。⑵ 既定ポート 9821・`AREKA_MCP_PORT`（`0` で待ち受けない）・既定で有効（SSP と同じ）。9801 は SSP と同時に動かすと衝突し、将来の SSTP の口でもあるので避けた。⑶ SSP の欠陥 2 件（表情表の文字化け・script ログの JSON エスケープ漏れ）は移植しない。⑸ **MCP のプロトコルは自作せず公式 Rust SDK `rmcp` を使う・tokio 依存を入れてよい**（2026-09-29 開発者判断「tokio 依存は入れちゃってもよい。mcp を自作するのは避けた方がよさそう」）。MCP の新しい版への追随は rmcp の版上げで行い、SSP の輸送の癖（survey §2）は参考に格下げ＝ツールの名前・引数・結果の文字列だけを SSP と一致させる。中継 exe は写すだけなので rmcp を使わない。⑷ 分け方は開発者の 3 段（基本 → 入り口 → 個別）に、個別を「触るエンジン」で 6 本に割り、strict を 4 段目に出した。
- **採らなかった分け方**: ツールごとに 1 spec（10 本）＝`get_status`・`sakurascript`・`raise_event` が kanade の同じ箇所を触り並走できず、`get_active_ghost_list` などは小さすぎる。基本と入り口を 1 本＝開発者の 3 段に反し、M3 の並走の土台（触るファイルの固定）が基本の検査と同じ spec に埋もれる。
- **既存 spec の更新**: `makoto-dll-host`（brief へ「MCP の reload makoto の口を埋める」を追記済み）。`status-execution-states` は `get_status` の消費者が 1 つ増えるだけ（brief の更新なし）。

#### Specs (dependency order)

- [ ] areka-P0-mcp-server-core -- 127.0.0.1 の HTTP・JSON-RPC・MCP の版と検査（ツール 0 本）。Dependencies: α 完成宣言
- [ ] areka-P0-mcp-tool-entrances -- ツール 10 本の定義・引数検査・ghost_name の解決・World への橋・ダミー 9 本。Dependencies: areka-P0-mcp-server-core
- [ ] areka-P0-mcp-stdio-bridge -- Claude Desktop 用の stdio ⇔ HTTP 中継 exe。Dependencies: areka-P0-mcp-server-core
- [ ] areka-P0-mcp-get-property -- get_property。Dependencies: areka-P0-mcp-tool-entrances
- [ ] areka-P0-mcp-kanade-tools -- get_status・sakurascript・raise_event。Dependencies: areka-P0-mcp-tool-entrances
- [ ] areka-P0-mcp-expression-table -- get_expression_table。Dependencies: areka-P0-mcp-tool-entrances
- [ ] areka-P0-mcp-log-history -- get_log。Dependencies: areka-P0-mcp-tool-entrances
- [ ] areka-P0-mcp-reload -- reload と \![reload,…]。Dependencies: areka-P0-mcp-tool-entrances, areka-P0-shell-balloon-switch
- [ ] areka-P0-mcp-dump-images -- dump_surface・dump_balloon。Dependencies: areka-P0-mcp-tool-entrances
- [ ] areka-P0-mcp-strict-errors -- strict の記録。Dependencies: areka-P0-mcp-log-history, areka-P0-mcp-kanade-tools

### シェル内バルーン（α 後・2026-10-01 `/kiro-discovery` で起票）

> 開発者指示（2026-10-01）「areka では設計上、シェルとバルーンの描画区別を付けないように留意してもらっていたと思う。実際に、シェル内にバルーン領域を持つゴーストを設計したい。イメージは『窓際のぱすたさん』の縦書きキャッチコピー（ghost_dev リポジトリ `doc/assets/text_layout/縦書きキャッチコピー.html`）。実装時期は α 後なるべく早い時期」。**完全に areka 独自の仕様＝差別化のための機能**（開発者）。参考の寸法は同ゴーストの `doc/画面設計_縦書き要件定義.md` §3.1（白い窓の絵の中に、天 62px・外側 14px・右の台詞欄 2 列・左の独白欄 3 列・本文 27px・列ピッチ 48px）。

- **起票時の実測（main `35209987`）**: 「シェルとバルーンを区別しない」は一番下の層（emo-present の窓ごとの組み立て＝どの窓にも文字の層の差し込み口がある・拡大率も流れる）では本当で、その上（文字の描き手の結線・`BalloonModel` だけの設定・1 スコープ 1 か所・文字の数でバルーンの窓が出る）は区別している。土台の作り直しは要らず、上の層の配線と宣言の決まりが要る。正典（ukadoc）には、シェルの中の文字の場所・1 人に 2 か所・ルビ・縦中横・禁則・クリックでの早送りのいずれも無い（調査で 0 件）。
- **決めたこと（開発者確定・要件の段で覆してよい）**:
  1. surfaces.txt の新しいブレス `balloon.名前`（ukadoc の「surface*ブレス」と同じ用語の型）。中身はバルーンの descript.txt と同じキー＋大きさ（仮称 `size`）。大きさはブレスが持つ（サーフェスごとに大きさが変わるのは良くない）。
  2. 置くのは `surface*`ブレスの element定義 `elementN,balloon,名前,X,Y`（描画メソッド `balloon`＝画素を重ねるのでなく文字を描く）。数は任意。将来の「`surface1000` を element定義で置く」とも整合する（正典の pattern定義が描画メソッドを残しファイル名の欄でサーフェスを指す前例）。
  3. **バルーンの中身はすべて互換**＝同じバルーンの実装で、描く先が替わるだけ。背景の絵は持たない。当てはまらないのは `windowposition.*`・`use_self_alpha`・`use_input_alpha` だけ。
  4. 行き先は「スコープ × 名前」・既定は一番若い element番号・`\b[名前]` で切り替え・普通のバルーンと併用しない・`\c` は今の行き先だけ・選択肢も出す・フォントはシェル → ゴーストのフォルダの順。
  5. サーフェスの切り替え: 同じ名前があれば文字ごと移る／無ければ文字を持ったまま隠し既定へ書く／1 つも無ければ普通のバルーンへ。
  6. 早送り＝話している最中の 1 クリックで次の `\x` か台詞の終わりまで（箱のクリックはシェルのイベントにしない）。**クリック待ちは台本に明示した `\x` だけ＝自動の改ページは作らない**（「デスクトップマスコットは利用者の状況お構いなしに喋るもの」）。あふれは正典の自動スクロール＋（独立 spec の）押し出しのフェード。
  7. ルビ `\![text,ruby,親文字,ルビ]`・縦中横 `\![text,combine-upright,…]`（台本の独自拡張は SSP の流儀に倣い `\!` に入れ、`\![text,…]` を文字組みの入り口にする）。キーは CSS のプロパティ名の `-` を `_` にした名前（`line_break`〔既定 `anywhere`＝互換〕・`hanging_punctuation`・`text_combine_upright`・`text_orientation`・`line_height`・`letter_spacing`）。
  8. 未実装の印（`arrow*`・`onlinemarker.*`・`number.*`・`sstpmarker.*`・`sstpmessage.*`）も今回の spec 群で実装まで持つ（開発者指示）。SSTP の 2 項目は表示だけ作り、受信は登記だけの行「SSTP の受信」。
- **採らなかったもの**: 透明なバルーンをシェルに重ねる案（正典だけで組めるが窓が 2 枚・1 人 2 か所が無理）・バルーンの側に「シェルの上に描く」と書かせる案（バルーンが特定のシェルの寸法に縛られる）・`balloon.*`ブレスは見た目だけで矩形はサーフェスの専用行に書く案（将来のサーフェスの element定義と書き方が分かれる）・青空文庫式のルビ（唐突）・新しいタグ（`\_r` など）・自動の改ページ・背景の絵・SSP の「縦書きでは同梱フォントを標準ゴシックへ差し替える」（GDI の制約で、areka の DirectWrite には無い）。

#### Specs (dependency order)

- [ ] areka-P0-shell-balloon -- `balloon.*`ブレス・描画メソッド `balloon`・シェルの窓への結線・行き先と切り替え・併用しない。Dependencies: α 完成宣言
- [ ] areka-P0-balloon-font-file -- `font.name`・`\f[name]` のフォントファイル（バルーン／シェル → ゴーストのフォルダ）。Dependencies: α 完成宣言
- [ ] areka-P0-text-typesetting -- 禁則・ぶら下げ・縦中横・字の向き・縦書きの字形。Dependencies: α 完成宣言
- [ ] areka-P0-talk-fast-forward -- クリックでの早送り・`\x`／`\x[noclear]`・`clickwaitmarker.*`。Dependencies: areka-P0-shell-balloon
- [ ] areka-P0-text-ruby -- `\![text,ruby,…]`・`line_height`・`letter_spacing`。Dependencies: areka-P0-text-typesetting
- [ ] areka-P0-balloon-markers -- `arrow*` と手動スクロール・`onlinemarker`・`number` と `\![set,balloonnum]`・SSTP の印（表示だけ）・装飾の系列。Dependencies: areka-P0-shell-balloon
- [ ] areka-P0-balloon-scroll-fade -- 押し出される行（列）のフェード（areka 独自・既定は無効）。Dependencies: areka-P0-balloon-markers
- [ ] areka-P0-balloon-element-order -- `balloon` の element定義を並び順どおりの重ね順で描く（`shell-balloon` の縮めを外す追跡 spec）。Dependencies: areka-P0-shell-balloon

### 動く画像（α 後・2026-10-01 `/kiro-discovery` で起票）

> 開発者指示（2026-10-01）「アニメーションをサポートする画像ファイルに対応して欲しい。具体的には、画像読み込み時にサブエレメント分解して、アニメーション表示を回す案を出しておきます。arekaのシェルエレメント管理は再帰構造を持つように設計指示していたので、ちゃんと実現されているなら可能なはず。webp形式サポートとかがよいかな？apngもあるけど。時期はα後」。正典は ukadoc の element定義の項「surface*.pngまたはelement定義にアニメGIF/APNG/WebPアニメを指定すると、SERIKO定義を書かなくても自動的にアニメーションする(SSP 2.7.38～)」と、pattern の描画メソッド `import`（2.7.50）。

- **起票時の実測（main `5e37745e`）**: 本番の読み込みは WIC の `GetFrame(0)`＝1 枚目のコマだけ。**再帰は半分だけ実現**＝pattern → サーフェスの参照は `flatten_surface` が再帰する（位置のずれの加算・循環の停止）が、element は画像専用で他のサーフェスを指す道が型に無く（設計文書の「element が他サーフェスを参照」は未実装）、入れ子の内側は `PatternState` を見ない（内側のアニメーションは動かない）。seriko の表は `always`・`runonce`・`bind` を記録しない。合成器が描くのは `overlay` だけ。
- **決めたこと（開発者確定・要件の段で覆してよい）**: ⑴ GIF・APNG・WebP の 3 形式を同時に入れる（`image` クレートが 3 形式とも重ね済みのコマと待ち時間を返す＝1 形式に絞っても手間は同じ）。⑵ 動く絵だけ `image`・静止画は WIC のまま（WIC は APNG を読めず、WebP は Windows の拡張機能しだい）。⑶ 読み込み時にコマへ分解し、SERIKO の型へ写して今の seriko の時計で回す＝アニメのエンジンは 2 つのまま。⑷ 読み込み／再生に分ける。`import` は再生の側に含める（別にしても並走が増えない）。⑸ interval `always` を再生の側で入れる。⑹ **階層化エレメントを `surface-element-nesting` として起こし、動く絵はその上に載せる**＝読み込み時にコマから「`always` でコマを順に指す子サーフェス」を作り、element でそれを置く（開発者「手書きで定義できるようになる→画像を自動に定義分解する。ながれできれい」）。⑺ 書き方は element定義のファイル名の欄に**数字だけ＝サーフェスの番号**（正典の pattern定義の前例・`shell-balloon` の見込みと同じ。`surface100`〔拡張子の無い画像と紛れる〕と新しい描画メソッド〔overlay 以外で重ねる道を塞ぐ〕は採らない）。⑻ 子の当たり判定は親へ持ち込む（位置をずらし・親の element の順で手前奥）。⑼ 内側の `balloon` の element定義は警告して無視（最初の版）。⑽ **子の時計は独立し、親の面の切り替えで巻き戻らない**（開発者「1→2 に切り替えたとき、両方から参照されている 100 のアニメーションがリセットされるとダサい」）＝「スコープ × 子サーフェス」に 1 つ・初めて見えたときに動き出し、シェルが替わるかゴーストが降りるまで止めない（`1 → 3 → 1` でも巻き戻らない）・一番上のサーフェス自身のアニメーションは今までどおり切り替えで最初から。
- **採らなかった分け方**: 画像ごとに独立した時計（アニメのエンジンが 3 つ目になる）・WIC だけ（APNG が読めない・WebP が環境しだい）。
- **階層化エレメントの議題（2026-10-01・決着）**: 開発者の問い「階層化エレメントが扱えるか・surfaces.txt でどう定義できるようにするか」。実測は上のとおり「再帰の骨組み（合成器・アトラス）は在り、入口（element の型）と内側の時計が無い」。決めたことは ⑹〜⑽。「シェル内バルーン」節のセッションへ実測と方針を共有した。
- **依存の追加**: `image`（本番へ移す・`gif` の機能を足す）は `tech.md` の「意図的依存追加」への登記と開発者の承認が要る＝`animated-image-decode` の要件の段で。

#### Specs (dependency order)

- [ ] areka-P0-animated-image-decode -- 動く GIF・APNG・WebP の全部のコマ・待ち時間・繰り返し回数を読み、アトラスにコマの番号で載せる。Dependencies: α 完成宣言
- [ ] areka-P0-surface-element-nesting -- element定義でサーフェスを置く（数字だけ＝番号）・子の当たり判定・子の時計は独立。Dependencies: α 完成宣言
- [ ] areka-P0-animated-image-playback -- 動く絵を子サーフェスへ分解して置く自動アニメーション・`import` メソッド・interval `always`。Dependencies: areka-P0-animated-image-decode, areka-P0-surface-element-nesting

### 文字の現れ方（α 後・2026-10-01 `/kiro-discovery` で起票）

> 開発者指示（2026-10-01）「文字のタイプライター表現において、フェードインしながら文字を表示するモードが欲しい。よくノベルゲームで見ますよね。より夢のある仕様では、ニンテンドーのゲームみたいに、文字が踊りながら表示されるエフェクトも欲しいけど、これは Option の夢 spec として別に切ってください」。

- **起票時の実測（main `9cf09f5d`）**: 1 字ずつの表示は字ごとの「現れる時刻」を既に持つ（`areka-emo-text/src/state.rs` の `visible`）＝フェードも動きも「現れる時刻からの経過」の純関数で決まる。手が要るのは描画の側で、今は一度描いた字を描き直さない。
- **決めたこと（開発者確定）**: ⑴ 有効にする書き方はバルーンのキー `text_reveal`（`balloon.*`ブレスにも書ける）と台本 `\![text,reveal,…]`（台詞の終わりまで）の両方。⑵ 早送りのクリックと `\_q` の中では即座に不透明・定位置。⑶ 既定のフェードの長さは 150ms。⑷ 夢の方は**現れるときだけ動いて止まる**（跳ねる＝ポーンと跳ねて定位置に戻る・揺れる）。ずっと動き続ける演出は負荷が大きく目に毒なので作らない。
- **採らなかったもの**: キーだけ（ここぞという台詞だけ切り替えられない）・早送りでもフェードを待たせる・ずっと動き続ける演出。

#### Specs (dependency order)

- [ ] areka-P0-text-reveal-fade -- 字が透明から不透明へ変わる現れ方（`text_reveal,fade`・`\![text,reveal,fade,…]`）。Dependencies: α 完成宣言
- [ ] areka-P0-text-reveal-dance -- 現れるときだけ跳ねる・揺れる（夢・任意）。Dependencies: areka-P0-text-reveal-fade

### alpha-release-signoff の持ち越し（α 後・2026-10-02 `/kiro-discovery` で起票）

> 出どころは `alpha-release-signoff` の完成判定 `verification/alpha-completion.md` §6「持ち越した事項」の、引受先を「α 後の `/kiro-discovery`」と書いた 5 行（受入記録 `verification/acceptance-record.md` の §8.1 ⑴・⑵・§8.5・§8.6・§8.8）。開発者指示（2026-10-02）「あ、起票はあとでやってくれますよね。「実装完了を承認」スキルは最後に実施しますし。その前提で、今は実装に戻ってください。」。どの行も α の判定を書き換えない。

- **分け方**: 5 行を 4 本にした。§8.1 の ⑴ と ⑵ は、どちらも「インストールの最中に使用中のものと当たる」危険で、同じ手続き（`crates/areka/src/install/`）と同じ実機の手順（`target\` の下の根で入れる）を使い、**測ってから扱いを決める**という進め方も同じなので 1 本（`install-live-target-hazards`）にまとめた。§8.5（窓の位置の保存）・§8.6（配布スクリプト）・§8.8（`install.txt` の読み方）は触る場所が互いに重ならないので 1 本ずつにした。
- **採らなかった分け方**: §8.1 と §8.8 を 1 本（どちらもインストール）＝§8.8 は ukadoc の読み方を揃える正典の仕事で、実測を待たずに進められる。実測待ちの ⑴⑵ と束ねると、正典の仕事が実機の結果を待つことになる。§8.5 を登記だけの行にする＝設計の意図（掴んで離したときだけ保存）に反するバグで、初回だけの位置合わせの規則を崩すので brief を持たせた。
- **段**: 4 本とも「α 後」（開発者指示）。`drag-click-without-move` は種別がバグなので、α 後の棚卸で「バグ → その他」の順の先頭に並べ直してよい。

#### Specs (dependency order)

- [ ] areka-P0-install-live-target-hazards -- 表示中のシェル・使用中のバルーンのフォルダへの上書きと、起動中のゴーストへ入れる途中の Windows の終了を実測し、扱いを決めて実装する。Dependencies: α 完成宣言
- [x] areka-P0-drag-click-without-move -- 動かさない左クリックで窓の位置を保存しない（バグ）。Dependencies: α 完成宣言
- [x] areka-P0-package-check-temp-cleanup -- **10-02 に `release-package-versioned` へ合流**（同じスクリプト・C1 の先頭のタスク）
- [ ] areka-P0-install-companion-canon -- 同梱インストールの `install.txt` の読み方を ukadoc に揃える 4 点＋起動時の同梱バルーンの問い。Dependencies: α 完成宣言

### 配布と公開（winget・crates.io・2026-10-02 `/kiro-discovery` で起票）

> 開発者指示（2026-10-02）「winget 対応インストーラーを作成したい。最終的には繰り返し spec を実行するたびに、バージョン番号を 0.0.1 加算しながらリリースを作るような運用。アーカイブの署名が必要なのか？無料で済ませたい」「とにかく無料でできる手段。CI 実行でよいが、PR のたびに実施されるとバージョン管理が変にならないか？リリースタイミングはこちらで決めたい」「可能なら crates.io へのリリースも組み込んで」。

- **起票時の実測（main `76e17654`）**: 版は `Cargo.toml` の `[workspace.package] version = "0.0.1"` 1 か所（21 クレートが継承・SHIORI へ渡す版もここから）。配布は `tools/package-alpha.ps1` が手元で x64 の zip を日付とコミットの名前で作るだけ。GitHub Releases 0 件・`.github/` なし・SHA256 なし・署名なし。`/kiro-complete` は版にもタグにも触らない。crates.io には `areka`・`dola`・`wintf` の 0.0.1 だけ（名前の確保）。部品のクレートの名前（`areka-*`・`shiori-*`・`log-capture-kit`・`temp-path-kit`）はすべて空いている。
- **調べた事実（2026-10-02・出典は `winget-manifest-submission`・`release-code-signing` の brief）**: winget の zip／portable に署名の義務は無い（MSIX だけ要る）。portable はスタートメニューのショートカットを作らない。PATH へ出すリンク経由の起動で隣の DLL を見失う既知の問題は `ArchiveBinariesDependOnPath: true` で避ける。初回の提出は手で（1 PR 1 版・人の承認・実例で 2 日）、以後は winget-releaser（Release の公開がきっかけ）。無料の署名は SignPath Foundation だけ（CI のビルドが前提）。Microsoft の Artifact Signing は日本の個人には使えない。crates.io は Trusted Publishing で長生きするトークン無しに出せる（新しいクレートの初回は手元から）。
- **決めたこと（開発者確定・2026-10-02）**:
  1. **入れ物は zip のまま**（winget の `zip`＋`portable`）。アイコンが欲しくなったら後からインストーラーを別 spec で足す。
  2. **名乗りは zip 版＝`Areka.Areka.Portable`（「areka ポータブル」）・インストーラー版＝`Areka.Areka`（空けておく）**。入れ物の種類ごとに別の名乗り（portable で入れた人の環境へ次の版でインストーラーを被せる乗り換えを winget は綺麗に扱えない）。発行者は areka プロジェクト自身（`ekicyou.areka` は却下）。
  3. **x64 と arm64 を最初から並べる**（arm64 はこのプロジェクトの動機）。補助 exe は i686 のまま。
  4. **きっかけはタグ**。普通の PR では何も起きない。繰り返し spec `release-cycle` を開発者が打ったときだけ、版を +0.0.1 → PR → squash マージ → `v{版}` のタグ → GitHub Actions がビルド・zip・SHA256・Release → crates.io → winget へ PR。版の正本は `Cargo.toml`、タグはそれを写す。
  5. **無料で**: 未署名で出す（規約上の障害は無い）。CI ができたら SignPath Foundation に申請する（任意）。有料の証明書は開発者がその場で決める。
  6. **「外部 CI は持たない」はテストの門の話**＝ビルドと配布だけを Actions に乗せる。zip の起動確認（`-Check`）は窓を出すので手元に残す。
  7. **crates.io は部品と本体の公開・名前の確保のため**。`cargo install areka` では 32bit の補助 exe が付かないので、利用者向けの入れ方は winget か zip。
- **採らなかったもの**: main への push で自動リリース（PR のたびに版が上がる）／feature の PR の中で版を上げる（並走する枝が同じ行を取り合う）／`cargo release` で main へ直接 push（PR 経由の決まりに反する）／zip の名前に日付とコミットを残す（版と重ねると長く、道具の自動判定が迷う）／MSIX（署名が必須で無料の道が無い）／zip 版とインストーラー版を同じ名乗りにする。
- **既知の制限として説明書に書くこと**: スタートメニューにアイコンは出ない（コマンド名 `areka` か、インストール先のフォルダから）／Smart App Control を有効にしている環境では未署名の exe が止まる／arm64 の実機の確かめは開発者の手元に機械が無ければ利用者の報告待ち。

#### Specs (dependency order)

- [ ] areka-P0-release-package-versioned -- `-Check` の後片付け（合流）・版入りの固定の名前と SHA256・arm64 の zip・リンク経由の起動での場所の解決。Dependencies: none（C1）
- [ ] areka-P0-release-ci-workflow -- タグ `v*` で動く GitHub Actions（ビルド → zip → Release）。Dependencies: areka-P0-release-package-versioned
- [ ] areka-P0-release-cycle -- 繰り返し spec。版 +0.0.1 → PR → squash マージ → タグ（初回は crates.io の手元公開を含む）。Dependencies: areka-P0-release-ci-workflow, areka-P0-crates-io-publish
- [ ] areka-P0-winget-manifest-submission -- `Areka.Areka.Portable` のマニフェスト・初回の手提出・winget-releaser。Dependencies: areka-P0-release-cycle
- [ ] areka-P0-crates-io-publish -- 部品のクレートの publish と Trusted Publishing・自分の workflow `crates-io.yml`。Dependencies: areka-P0-release-package-versioned, areka-P0-mcp-server-core（C2・`release-ci-workflow` と並走）
- [ ] areka-P0-release-code-signing -- SignPath Foundation への申請と CI での署名（任意）。Dependencies: areka-P0-release-ci-workflow

## 予約（全て任意・brief なし）

アプリ層＝FMO・DirectSSTP・Plugin/HEADLINE・多重ゴースト・トレイアイコン・消滅・設定画面と設定ファイル・`\![update,platform]`（本体の更新）・**インストーラー版（winget の名乗り `Areka.Areka`・データは `%APPDATA%` へ・スタートメニューのアイコン）**・`\![execute,createnar]`／`createupdatedata`（作る側＝開発者機能）。SSTP の受信は「覚え書き」。互換面＝SAORI は実装しない（SHIORI が直接 `LoadLibrary`・台帳 `not-applicable`）・里々/YAYA 網羅。emo テキスト進化＝回転テキスト（`TextEffects` 予約名 `rotation`／`multicolor`）。**`\f[sub]`／`\f[sup]`／`\f[outline]` は DirectWrite の標準機能で表せる手段が見つかるまで語彙のみ**（2026-09-11 裁定・台帳 `vocabulary-only`・追跡先はこの行）。バルーン美観配置。pasta の native x64／`IShiori` in-proc・ベクトル描画・owner-draw 右クリックメニュー（`popup-menu-minimal` の構造の上に見た目を被せる）・着せ替えメニュー。段階 A の先頭ウェーブ 6 束（`roadmap-draft.md`・324 件）は M3 のゴールを決めるときの材料。

---

**追記台帳**: 追記(95)〜(103) の要約は history「2026-09-26 番号表記の廃止に伴う退避」節。
