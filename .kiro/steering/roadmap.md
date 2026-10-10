---
inclusion: manual
updated_at: 2026-10-10
---

# Roadmap — areka（α 後・M3 へ向けた並べ直し）

> **M1 は 2026-09-11、M2（α）は 2026-10-02 に完成宣言済み**。本ファイルは 2026-10-02 の棚卸⑳で、α 後の着手順へ書き直した。**完了した spec の行・棚卸⑭〜⑲の裁定・α のウェーブ表・旧「登記だけの行」の全文は [roadmap-history.md](roadmap-history.md) の「2026-10-02 棚卸⑳退避」節へ逐語で移した**（history が全文正本・非改変）。2026-10-04 の棚卸㉑で、旧ウェーブ表（C1・C2）と棚卸⑳の裁定を同じ history の「2026-10-04 棚卸㉑退避」節へ移した。2026-10-05 の棚卸㉒で、旧ウェーブ表（C3 と予定の C4）・直列の列・覚え書き・直接修正候補・棚卸㉑の裁定を「2026-10-05 棚卸㉒退避」節へ移した。2026-10-10 の棚卸㉓で、旧ウェーブ表（C4）・初回リリースの段取り・直列の列・brief にした覚え書き・直接修正候補・棚卸㉒の裁定を「2026-10-10 棚卸㉓退避」節へ移した。
> 正本配置: 本ファイルが正本（`.kiro/steering/roadmap.md`）。`focus.md`（`inclusion: always`）から辿る。設計判断の正本は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md)。ukadoc 網羅の段階・順位の正本は `doc/ukadoc-coverage/`（`briefing.md`・`linkage.md`・`roadmap-draft.md`）。完了した spec の実装の詳細は各 `completed/` spec が正本。
> **読む順**: いま何を始めるか＝「ウェーブ編成」。個々の spec の中身＝「spec 台帳」と各 brief。brief を持たない宿題＝「覚え書き」。

## M1 ゴール ✅（2026-09-11 完成宣言）

areka（x64）が最小 SSP 互換ベースウェアとして、適合対象ゴースト **emo2**（作者自作・脳=`pasta.dll`・32bit SHIORI）を「そのまま」起動→会話→撫で→メニュー→終了まで E2E 実走させる。emo2 が動く＝同じ汎用 32bit ブリッジで里々/YAYA も動く土台。

**✅ 完成宣言**: 適合検証項目表 20 項目全合格の実機サインオフに開発者の署名（`.kiro/specs/completed/areka-P0-emo2-conformance-e2e/verification/m1-completion.md`・項目別は同 `acceptance-record.md` §8.1）。持ち越し 8 件（同 §6）はいずれも判定を書き換えず、引受先は下の spec 台帳に全て実在する（§13.1 行 3「初回起動限定の位置調整」だけは「違和感があれば個別仕様を切る」の据え置き＝spec なし）。

## M2 ゴール（α）✅（2026-10-02 完成宣言）＝第三者がデスクトップマスコットを管理できる

**✅ 完成宣言**: 第三者の手順の検証項目表（13 項目と付随の確認 2 つ）の実機サインオフに開発者の署名（`.kiro/specs/completed/areka-P0-alpha-release-signoff/verification/alpha-completion.md`・項目別は同 `acceptance-record.md` §7）。署名の根拠のコミット `8460506d`・zip `areka-alpha-x64-20261002-8460506.zip`。

**利用者の一周**: zip を展開して起動 → `.nar` を窓へ落とす（またはメニューから選ぶ）→ ゴーストが起動する → 右クリックメニューでゴースト／シェル／バルーンを替える → ネットワーク更新で作者の修正を受け取る → 終了 → 再起動で前回の状態に戻る。表現力は M1 の水準（emo2 が普通に動く）で据え置いた。メニューは Win32 標準。

**持ち越し**: 完成判定 §6 の 5 行は 4 本の spec（`install-live-target-hazards`・`drag-click-without-move`・`package-check-temp-cleanup`〔10-02 に `release-package-versioned` へ合流〕・`install-companion-canon`〔10-03 に要件の段で `install-companion-reading`・`ghost-standard-balloon`・`shell-companion-balloon` の 3 本へ引き継いで閉じた〕）へ起票済み＝ウェーブ C1〜C4。emo2 の 2 件は ghost_dev へ申し送り済み。

## M3 ゴール（未確定）

M3 のゴールはまだ決めていない（**開発者の決めごと**）。候補は `doc/ukadoc-coverage/roadmap-draft.md`「M3 の受入基準の候補」（「伺かの冠」）。決まるまでの着手順は、開発者指示（2026-10-02）「**バグ修正系を優先し、あとは優先度の高い機能**」に従う。優先度の物差しは `briefing.md` の 4 軸（壊れ方 ＞ 伺からしさ ＞ 資産の広さ ＞ 基盤共有度）と、開発者が時期を指定した spec 群（シェル内バルーン＝「α 後なるべく早い時期」）。

## 実装規律（balloon-system の失敗から得た正）

- **実装ファースト**: 各作業ユニットの成果物は「実際に動く」検証済みコード。**spec 工場の禁止**: 成果物が子 spec になる構造を作らない。1 ユニット＝1 かたまりの動く振る舞い。
- **最小実装＋薄い拡張シーム**: 使う分だけ実装し、拡張は型/レジストリの口だけ残す。抽象は「2 例目の実物」が要求してから。動く資産から建てる。
- **粒度基準**: 1 ユニット＝単一 pass/fail の独立観測。純粋層は fixture/mock 直入力で切る。UI 位置決め・座標系は本番ゴースト（実 emo2）＋実 DPI（≠96）が観測条件（記憶 areka-placement-real-ghost-first）。
- **語彙完備・配線ゼロの追跡**: 先送りシームには狭い `#[allow(dead_code)]`＋実在理由の doc を義務付け、消費者ゼロの検出は棚卸の定期監査項目。
- **1 ファイル 1,000 行**: 機械の番人 `crates/log-capture-kit/tests/file_length_guard_test.rs`（例外表 10 件・暗黙増加不可・**どの spec も例外表に触れない**）。2026-10-10 棚卸㉓の実測で上限の近く（本番のファイル）: `crates/areka/src/placement/transition_judge.rs` 996・`crates/areka-emo-text/src/surface.rs` **995（もう足せない）**・kanade の `schedule/mod.rs` 955・`schedule/steady.rs` 950・`crates/areka/src/main.rs` 950・`emo2_boot/consumer_ledger.rs` 943（うち約 500 行は中のテスト＝最初に触る spec が兄弟のファイルへ出す）・`emo2_boot/spine.rs` 937・kanade の `msg.rs` 926（うち約 415 行は中のテスト）・`emo2_boot/mod.rs` 912・`emo2_boot/ghost_switch.rs` 902・`placement/mod.rs` 902・kanade の `actor.rs` 900。テストの兄弟ファイルにも 950〜993 行が 15 本ある（`measure_tests.rs` 993・`actor_choice_contract_tests.rs` 991・`ghost_switch_tests.rs` 988・`balloon_visibility_tests.rs` 988・`runtime_tests.rs` 986・`emo2_boot/assets_tests.rs` 979 ほか）。足すときは新しいファイルへ。
- **決定論テスト網羅は必達**・**ログ無し失敗経路の禁止**・**終了経路は正規実装**（記憶 deterministic-test-coverage-mandate／areka-log-first-no-silent-failure／canonical-not-minimal-lifecycle）。
- **外部依存の追加は `tech.md` へ「意図的依存追加」を登記し開発者が承認する**（`encoding_rs` の前例）。α で登記したのは **`miniz_oxide` 0.9（`nar-install`・2026-09-18 承認済・推移的依存は `adler2` 1 本。2026-10-04 に `mcp-dump-images` が PNG の圧縮でも使うことを承認）**、`md-5` は要らなくなった（`update-engine` 2026-09-24 完了＝MD5 は OS の CNG・`tech.md` 登記済み）。HTTP は WinHTTP（`windows` crate の機能フラグ）で crate を足さない。

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
| C1 | 10-03〜 | `restart-chain-finalize-stall`（PR#214）・`choice-timeout-directive`（PR#215）・`drag-click-without-move`（PR#216）・`emo-text-file-split`（PR#217）・`release-package-versioned`（PR#218）・`mcp-server-core`（PR#219） | 初期配置の確定の見送りはゴースト待ち〔窓が無い・まだ一度も表示されていない〕を数えず、areka 自身の待ちだけを数える（起こし直しの後の `deferrals=600` の空鳴りを止めた）・`\![set,choicetimeout,時間]` を compile が読んで選択待ちへ渡す（`0`・負の値で時間切れなし・最後の指定が勝つ・読めない値は既定＋WARN）。再生層 dola は選択待ちの区切りを値で飛ばしも解きもしない・動かさないクリックで窓の位置を保存しない（wintf の累積器が開始を積んでいない終了を捨て、知らせの種を積んだ順の待ち行列で運ぶ。速いドラッグも開始 → 終了の順に届き、キャラクターは行き先へ置いてから保存）・文字とバルーンの 6 本と `region.rs` を振る舞いを変えずに役割で分けた（最大 571 行・新しい子 12 本＋`region_tests.rs`・`layout_inner` を分岐の種類ごとの関数へ・前後のテスト 9,371 本が名前も結果も一致・実機で 4 つの振る舞いを確認）・配布 zip を版入りの固定の名前（`areka-{版}-{x64\|arm64}.zip`）と隣の `.sha256` で x64・arm64 とも作る（`tools/package.ps1`＝旧名から改名・完成品は全段が緑のときだけ現れる・`-Check` の展開先は `target\package` の下で合格なら片付ける）・winget のリンク経由で起動しても exe の本当の場所から根・補助 exe・記憶の置き場を引く（`boot_config.rs` の `exe_location`・手元のマニフェストで実機確認）・新クレート `areka-mcp`（rmcp 3.5.0・無状態・JSON 単発・ツール 0 本・登録口・help・`Origin`／`Host` の検査）を `fn main()` の 1 行で立てる。既定の待受は 9801 → 9821 を SSP と早い者勝ち・使用中なら隣 +1〜+9（開発者裁定）。実機で Claude Code 2.1.283 が無状態版で `tools/list` を拒んだのを `ttlMs`／`cacheScope` で直した・SSP との差の一覧 18 行（直した 1）
| C2 | 10-03〜 | `mcp-tool-entrances`（PR#223）・`release-ci-workflow`（PR#224）・`crates-io-publish`（PR#225）・`translate-pipeline`（PR#226）・`shell-balloon`（PR#227）・`install-companion-canon`（PR#221・実施せず 3 本へ分解）・README の書き直し（PR#222） | MCP のツール 10 本を SSP 2.9.05 と同じ定義・同じ並びで出し（名前・必須の欄・型の誤りは `-32602`）、`ghost_name` を SSP と同じ文言で解決して UI スレッドの World へ届ける橋を立てた。`get_active_ghost_list` だけ本物・残り 9 本は `NG:not implemented yet`。返事は MCP 側も UI 側も塞がずに待ち、10 秒で打ち切り・終了の途中は即答。3 段目の spec はツールごとのファイルだけを書き換える（干渉台帳）。実機で Claude Code から要件 8.5 の ⑴〜⑹ を確認／タグ `v*` の push だけで動く `release.yml` を置いた（x64／arm64 の zip と SHA256 → 4 つ揃いの Release を公開・失敗なら残さない・乾いた走り・権限は `contents: write` だけで後段を呼ばない）。テストの門は手元のまま／汎用のライブラリ `wintf`・`dola` だけを同じ版で crates.io へ出す形を作った（30 クレートの `publish` の行に印と理由・`areka` は「出さない」へ・`dola` の版の指定を根の `[workspace.dependencies]` へ集め、版上げで動くのは根の 2 行と `Cargo.lock` だけ）・公開前の確認 `tools/crates-io.ps1`（引数なし＝ネットを使わず包むだけ・`-Verify`＝組み立てまで・`-Pending`＝まだ出ていないクレート。判定は毎回埋めた見本で較正）を全体テストの段に・公開の段 `.github/workflows/crates-io.yml`（タグ `v*` の push を自分で受けて同じタグの `release` の走りの成功を待ってから出す〔案 B・10-03 裁定・`workflow_run` は Trusted Publishing が断る〕・やり直しは Actions の画面の Re-run／Run workflow・Trusted Publishing だけ・上げる前に止まれる）・手順書 `doc/crates-io-publish.md`。実走は `release-cycle` の初回（`v0.0.2`）で、その前に `wintf`・`dola` へ Trusted Publishing の設定が要る／SHIORI の台詞を再生の前に `OnTranslate` へ 1 回通す（運行表の出口 1 か所で 5 種類の経路を捕まえる・環境変数を展開してから送る・204 なら今日と同じ表示・MAKOTO の口は素通しで `makoto-dll-host` へ）。emo2 の実機で 13 回すべて 204・表示の並びは前と同じ |
| C3 | 10-04〜 | `mcp-get-property`（10-04）・`install-companion-reading`（10-04）・`mcp-expression-table`（10-04）・`host32-testdll-marker-race`（10-04）・`animated-image-decode`（10-05）・`mcp-log-history`（10-05）・`shell-balloon-frame-align`（10-05）・`drag-cancel-borrow-miss`（10-05）・`surface-element-nesting`（10-05）・`mouse-drag-events`（10-05）・`mcp-dump-images`（10-05） | MCP の `get_property` が宛先のゴーストの記憶（sylphya）をそのゴースト自身の問い手で読み、値は素のまま（空の値も成功）・値の無い名前は SSP と同じ `NG:Cannot find such property name.` で答える（実行系の読み口 `GhostRuntime::sylphya_reader()` を 1 本足しただけ・名前は手直ししない）／書庫を入れるときの同梱のバルーンを ukadoc の探索の順（無印 → `balloon0` → `balloon1` …・`*.directory` の行の有無で数え、最初の欠番で打ち切る・先頭に 0 を付けた番号は引かない）で読み、`*.source.directory` を階層付きの相対パス（`\` と `/` のどちらでも・`..` と空の段は手前を打ち消さずに取り除いて記録・段が残らなければ断る）として読んで、段の数だけ剥がして置く。同梱の `*.directory` の区切りは `_` へ置き換えずに今どおり断る（正典の「`_` に置換される」は採らない）。記録の種類は 7 種（探索で読まなかった鍵・取り除いた取り出し元を足した）・α の検体 3 体は記録 0 件のまま／MCP の `get_expression_table` が今のシェルの `surfacetable.txt` だけを読み、SSP 2.9.05 と同じ表（見出し・区切り・各行を `\r\n` で終える・スコープは `\0`／`\1`／`\p[n]`）を素の値で答える。既定の名前 15 件を書かれていない ID にだけ重ね、`__disabled`・`__parts` の行は載せず ID だけを消す（読み手 `parse_surfacetable` は転記だけで、`areka-parsers` の `shell` に足した）。期待値は spec の `ssp-measurements.md` に写した SSP の答え（検体 1〜9）。ファイルが無ければ既定だけ・読めない行は行番号つきの `warn!` 1 回／i686 の helper のテストの印の揺れを、偽の DLL の `unload` が「印が自分の置き場と同じフォルダにあるときだけ書く」ことで塞いだ（自分の置き場はモジュールのパスから引く・時間待ちなし）。別のフォルダの写しの unload が印を書かないことを確かめるテストを 1 本足し、直す前の赤を記録。錠 `TESTDLL_SERIAL` は印の環境変数を差すテストだけの直列化に改め、テストを足すときの決まりを錠の説明 1 か所に書いた。i686 の段 10 回連続で緑／APNG・動く WebP の全部のコマ・待ち時間（ミリ秒へ四捨五入）・繰り返し回数を `image` 0.25.10（`png`・`webp` だけ・`image-webp` は透過の重なりの欠陥を直した固定コミット 0.2.5 から取り込み・取り込みが効いていることを全体テストで判定）で読み、アトラスにコマの番号で載せる（0 番は今までの鍵・2 枚目以降は鍵のエントリの後ろ・`AtlasTable::animation(親)`）。見分けはファイルの見出しだけ（拡張子は見ない・動く GIF は 2026-10-04 の裁定で非対応）。3 つの上限（枚数 1,024・絵 1 つ 67,108,864 画素・合計 268,435,456 画素・`AREKA_ANIMATED_IMAGE_MAX_*`）を超えるか読めなければ、動きの 1 枚目 → 今までの 1 枚読み → 失敗の一覧の 3 段で 1 枚へ縮める。静止画の道・`AtlasKey`・呼び手 5 ファイルは差分 0（emo2 の照合も無改変で緑）。起票 2 本（`element-clipping-option`・`placement-measure-bake-once`）／MCP の `get_log` が本物の履歴を返す（info 以上の出来事を `RUST_LOG` と独立に 5 種別へ既存のモジュールのパスで振り分け・全種別で 1 本の通し番号・種別ごと 1,000 件・本文 4,096 文字・出す側は 0 行）。ログの出口は `log_history::init()` の 1 行（標準出力は不変）。後続の `mcp-kanade-tools`・`mcp-strict-errors` との取り決め（target `areka::log::script`／`areka::log::error`・欄 `ghost`・`label`）は `doc/ssp-mcp/log-convention.md` が正本で、表と規則の一致をテストで判定。起票 1 本（`ghost-session-test-load-flake`）／シェルの箱と普通のバルーンの窓の切り替えを、台本の `\s` を受け取った時でなく表示層の絵が替わったフレームに揃えた。文字の層（`actor_box.rs`）は受け取った `\s` の番号を 1 か所も読まず、結線が渡す絵の番号（`current_surface_id`）だけから箱の置き場所・写し・窓に出す文字の数を導く。`Status` の届けはフレームの終わり（提示の後）の相へ移し、箱だけのときは覚えた番号で載せて `balloon_status_surface_unknown` の警告を出さない。枠の檻 7 本で直す前の赤を記録。起票 0 本／画面更新の最中に届いたドラッグを終える 5 種のメッセージ（ESC の押下・`WM_CANCELMODE`・非活性化・捕捉の喪失・左ボタンの離し）を、窓のメッセージの入口が捨てずに、World を使わないドラッグの扱い（`ecs/drag/reentry.rs`）へ渡す。終了の種を積むのはドラッグの状態を休ませる 1 か所だけにし（開始済みのときだけ・World を借りずに届く累積器の控えを通す）、ハンドラは休ませる関数を呼ぶだけにした。完了 spec `wintf-winmsg-executor` 要件 4.3 の入口の安全スキップを 5 種に限って上書き（COMPAT §8）。入口を通る再入のテスト 33 本で直す前の赤 25 本を記録。実機はドラッグで保存 1 件・動かさないクリックで 0 件・ESC で今どおりを確かめた。起票 0 本／element定義のファイル名の欄が数字だけならサーフェスの番号として読み（`ElementKind`・`0100` は 100・u32 を超える数は無い番号と同じ）、子を親の element定義の番号の順の位置へ何段でも重ねる（外形も子へ再帰・同じ子を何か所にも置ける）。無い番号・循環・子の中の箱は読み込み 1 回につき 1 度 `warn!`（合成の中は `debug!`）。子の当たり判定は親に無い名前だけを段ごとに位置をずらして持ち込む（`HitRegions`・pattern定義の先からは持ち込まない）。部品（element定義の子と pattern定義の先）は `PatternState` の部品の欄と seriko の `PartClocks`（スコープ × 部品の番号 × animation の番号）で一番上の切り替えから独立に動き、見えない部品は抽選しない。面の切り替え・着せ替えの変化では `refresh_parts` が部品の続きのコマを載せた `Show` を 1 件だけ出す。入れ子の無いシェルの絵・外形・当たり判定・乱数の回数・発行の回数は前と同じ（emo2 は `Show` 73 件・乱数 118 回を HEAD で焼き込んで不変・刻みが 880ms 以上止まった直後の端は要件 7.5 で規則どおり）。COMPAT §8 に 2 行（数字だけの欄・pattern定義の先のアニメーション）。実機は検体で J1〜J8 合格。起票 1 本（`extent-element-offset`）／キャラクター窓のドラッグで `OnMouseDragStart`・`OnMouseDragEnd` を ukadoc の 7 つの Reference（`OnMouseDoubleClick` の左ボタンと同じ並び）で送る（取り消しでも終了を送り、座標は押した位置・離したときは位置の保存の後の窓から見た位置・バルーン窓と右ボタンと動かさないクリックでは送らない・送らない経路は理由つきの記録）。ダブルクリックの 2 回目の押下からもドラッグを始める（開発者の裁定＝wintf の `mouse_dblclick_wheel.rs` の 1 か所）。送ってよい表は 48 語・台帳 2 行を実装済み・COMPAT §8 に 6 行。実機はクローディアで R1〜R7 合格。起票 4 本（`element-base-method`・`collisionex-regions`・`char-position-save-on-exit`・`areka-test-threads-av`） |
| C4 | 10-05〜 | `element-base-method`（10-05）・`mcp-get-status`（10-06）・`open-external-tags`（10-06）・`self-alpha-declaration`（10-06）・`budoux-reveal-reflow`（10-06）・`mcp-dump-images-residue`（10-06）・`mcp-ghost-name-match`（10-06）・`tools-utf8-child-output`（10-06・バグの席）・`ghost-standard-balloon`（10-07）・`choice-script-prefix`（10-07）・`animated-image-playback`（10-07）・`char-position-save-on-exit`（10-08）・`mcp-author-tools`（10-08）・`balloon-lifecycle-events`（10-08）・`wintf-tooltip`（10-08）・`ghost-session-test-load-flake`（10-10） | element定義の描画メソッド `base` を `overlay` と同じに描く（読み手の語の集合を `is_image_element_method` の 1 関数で `overlay`・`base` の 2 語にしただけで、`Element` に欄は足していない・element番号・`surface.append*`ブレス・数字だけの欄・複数の番号の見出しでも同じ）。ほかの語の element定義の行は `parse_undrawn_elements` が原文のまま拾い、`load_shell_target` が読み込み 1 回につき 1 行 1 件 `warn!`（見出し・element番号・語）。合成器・`plan.rs`・`Element`／`ShellTarget` の型は差分 0。既存テストの期待値の書き換えは名指しの 3 本だけ。COMPAT §8 に 1 行（element定義の `base` の X,Y は位置として使う）。実機は無改変のクローディアで surface6・11・26 が 333×500・警告 0 行。起票 1 本（`draw-methods-canon`）／MCP の `get_status` が宛先のゴーストの実行の状態を本物で答える（kanade の殻が `KanadeMsg::StatusQuery` をその場で受け、SHIORI へ送る `Status` と同じ `ExecutionStatus::derive` の値を返す・運行は 1 歩も進めない・`schedule/` は差分 0）。UI スレッドは待たず、後から答える置き場で毎フレーム覗く。答える前に降りたら宛先の解決と同じ文言・置き場に問い合わせ先が無ければ `NG:Status is not available`＋`warn!` 1 件。SSP との差 6 行は `doc/ssp-mcp/get-status-diff-areka.md`。実機は emo2 へ 400 回呼んで 4 項目合格・warn 以上の増加 0。起票 1 本（`farewell-talk-status`）。`open-external-tags`: `\j[http/https/mailto/file:///…]` と `\![open,file/browser/explorer/editor/mailer]` を OS の既定のアプリで開く（`\j` は読み手の腕 1 本で汎用の運び手へ・受け口 `ReadmeCueSink` が唯一の規則 `classify` で分類・専用のスレッド `open-external` で `ShellExecuteExW`・OS の窓は出さず、関連付けの無いファイルは呼ぶ前の `AssocQueryStringW` で 1155 の失敗として記録＝実機で OpenWith の窓が出た欠陥を直した）。説明書も同じ 1 か所へ移し、`ShellExecute` を綴るのは `readme/os_port.rs` だけ。記録は `get_log` の `status`／`error` に出る。受け取り手の表 15→21 行・台帳 6 行（`\j`・`explorer` は degraded）・COMPAT §8 に裁量 11 行。行き先を取り出す `link_destinations` は後続の `link-context-copy`・`balloon-link-hover` へ。起票 0 本。`self-alpha-declaration`: シェルの `seriko.use_self_alpha`（読み込みのたび）とバルーンの `use_self_alpha`（フォルダ直下の descript.txt を 1 回・`balloons*s.txt` は見ない）を読み、`1`/`true`・`full`・`0` を描き分ける（決まりは atlas の `Normalizer::plan` の 1 か所・動く絵は 1 枚目で決めて全コマへ・正規化は失敗しなくなり `NormalizeError` を削除）。宣言なしは「α<255 の画素が在れば α・無ければ左上の色を抜く」の areka 独自の決まり（10-05 裁定・正典の既定は `0`）。`.pna` は使わず数を `warn!`。`surfaces.txt` が無い・面を定義しないシェルは面の画像だけで組む。実機は えも？？・Staysee・claudia・R_POST・konnoyayame の 7 場合で前の版と画素の違い 0。台帳 2 行を degraded・説明書の既知の制限を書き直し。起票 0 本 ・`budoux-reveal-reflow`: `budoux_newline,1` のバルーンで、合図が分かれて届いても表示済みの字が次の行へ飛ばない。再生の前に台本の全部を受け手へ先渡し（dola の `CueSink::preview`・`register_sink` で 1 度）、文字の層が状態の写しで空回しして場所×区間ごとの全文を求め、文節の区切りを全文で決める（届いた字が全文の先頭と一致しなければ届いた字で区切って `warn!` 1 度）。dola の `TimedSchedule` の到達・期限・完了の判定を `start_time + offset` の足し算へ直した（予定時刻ちょうどの tick で合図を取りこぼしていた）。実機の emo2 初回起動トークで「‥」が最初から 2 行目の頭に出ることを MCP の `dump_balloon` で確認。起票 2 本（`choice-ranges-one-function`・`reflow-scroll-path-test`〔ともに優先〕） ・`mcp-dump-images-residue`: `dump_balloon` の文字の面の読み戻しを UI スレッドから外した（呼び出しの時点で写しを積み、`D3D11_MAP_FLAG_DO_NOT_WAIT` で毎フレーム覗いて、読めたら符号化のスレッドへ渡す＝`PendingReadBack`）。`dump_surface` と共通の覗く関数の状態 `Wait`（装着待ち → 読み出し待ち → 符号化待ち）で、装着待ちの間にゴーストが替わったら `NG:Specified ghost is not active`（写した後は写した時点の絵を返す）。判断の文言は `Refusal` の型で想定外の失敗（`error!` 1 件）と分け、`later` を通る組も符号化のスレッドで `catch_unwind` 付きに。実機の emo2（拡大率 200%）で `dump_balloon` 6 回の UI スレッドの時間は最大 939 µs（前は 14.2 ms）・ERROR 0。起票 0 本 ・`mcp-ghost-name-match`: MCP の `ghost_name` の照合を SSP 2.9.07 の実測（survey §7.4）に合わせた。名前と本体側名（`sakura.name`・`ActiveGhost` に欄を足した）を、渡された文字列の前後の空白を除き半角の英字の大小を畳んで比べる。フルパスは今の比べ方のまま。空文字・空白だけは省略の扱いをやめて `NG:Cannot find active ghost from specified name`。相方の名前・`sakura.name2` は見ない。本番のコードで振る舞いを変えたのは `resolve.rs` だけ（`get_log.rs` は注釈）。実機の emo2 で survey §7.4 の全行を 3 本のツールへ当てて 49 件すべて表どおり・ERROR 0。起票 0 本 ・`tools-utf8-child-output`: リリースの道具（`tools/package.ps1`・`tools/crates-io.ps1`・`release.yml`・`crates-io.yml`）が子の出力（`cargo metadata` ほか）を UTF-8 のバイトとして読む（新しい `tools/utf8-child.ps1` の `Invoke-Utf8Child`）。端末の文字コードの書き替えを道具と CI の段から撤去し、道具が端末へ出す文を ASCII だけにした。判定 `tools/encoding-check.ps1`（規則 A〜E と 932 の子の端末での実走）を全体テストの段 `encoding check` に足した。外から見える形（`-Check`・`-Verify`・`-Pending` の終了コードと標準出力・2 つの workflow のきっかけ）は変えていない。開発者の 932 の端末で `-Check` 緑・`-Verify` 0・main の `release.yml` の乾いた走り success。起票 1 本（`perf-tools-console-encoding`〔その他〕） ・`ghost-standard-balloon`: 起動のときのバルーンの決め方に、ゴーストの descript.txt の `default.balloon.path`（根のバルーンの置き場の直下のフォルダ名 1 段・区切りや `..` を含む値は当たらない扱い）と `balloon`（バルーンの `name`・無ければフォルダ名）をこの順に突き合わせる段を記憶の段の次・同梱の段の前に足し、同梱は無印の `balloon.directory` が無ければ `balloon0.directory` を使う。決め方の順は起動と切替で同じ（`resolve_balloon_for_ghost` の 1 か所）で、切替の行き先には `switch_balloon_resolved` を 1 件記録する。テストの一時の根の入口 `TempPath::under_target`（`target\test-roots`）を足した。実機の claudia で 4 通り（同梱・記憶・descript・見つからず同梱へ）を確かめた。起票 3 本（`dump-balloon-debug-timeout`〔バグ〕・`test-roots-under-target`・`dev-helper-x64-clobber`〔ともにその他〕） ・`choice-script-prefix`: `\q[…,script:…]` を選ぶと待ちを閉じ、`script:` の後ろを新しいトーク（出どころの語 `choice_script`）として再生する（`plan_cascade` の結論を `Unsupported` から `Script` へ・結末は `steady.rs` の子の `steady_choice_script.rs`・解決を先頭に再生開始をその後ろに並べる・入れ子の `script:` も 2 段で動く）。正典が黙っている所は areka の決めごと 4 つ（イベントを起こさない・翻訳に通さない・第 3 引数以降は数だけ警告・空なら待ちを閉じるだけ）として `doc/choice-cascade-compat.md` に書いた。台帳の行を実装済みへ。起票 0 本 ・`animated-image-playback`: APNG・動く WebP をシェルの面・element定義・バルーンの面の上で、SERIKO を書かなくても動かす（読み込みで動く絵を子（`ElementKind::Film`・`PartKey::Film`）へ分解し、子の見えているコマを `PatternState` の欄で持つ・再生は seriko の部品の時計 1 本で、シェルとバルーンに同じ仕組み）。interval `always` を単独で駆動し（表示中ずっと回す・経過 0 のコマが合成の休みのコマと一致）、`bind+always` などの組み合わせは `seriko-trigger-intervals` へ申し送り（台帳は縮退）。バルーンの窓の見える・見えないを seriko へ知らせ（`SerikoMsg::Stage`）、隠れているバルーンの合成は先送りし、出し直しの前に出た指令は出番の世代（`StageAck`）で見分けて回数つきの子のコマを外す。静止画だけのシェル・バルーン（`emo2` を含む）の見た目と合成の回数は変わらない。外形の計算を `plan_extent.rs`、`always` を `plan_always.rs` へ分けた。COMPAT §8 に節「動く絵と `always` の再生」と【上書き】の行。実機で 2 形式を確かめた。起票 3 本（`present-emit-tail-latency`・`balloon-text-area-collapse`〔ともにバグ〕・`seriko-rebuild-hidden-lottery`〔その他〕）と申し送り 3 件／起動の最後に全キャラクター窓を並べ終えた時点で、記憶に位置が無いキャラクターの位置を書く（`placement/persist.rs` の `persist_unremembered_char_positions`・呼び手は `finalize_chain_once_with` の印の直後の 1 か所）。書く時機はドラッグの確定と並べ終えた時点の 2 つ（完了 spec `position-persist` の要件 1.9 を改め、COMPAT §8 に 1 行）。「記憶から戻した窓」は値の比較でなく記憶に x・y があるかで見分ける（`has_saved_char_pos`）。台本の移動の指示（`\![move]`）で動いた窓は書かない（縦だけの移動も・既定の y は元の既定から取る）。実機はクローディアで R1〜R3 合格（相方が前回の位置に立つ）・emo2 は起動の台詞で相方を `\![move]` するので書かれないことを確かめた（要件 5.5 を改めた）。設計の段で起票 1 本（`dpi-realign-remembered-chain`）・完了時の起票 0 本。wintf に、マウスが止まったことの検出（OS の待ち時間の 2 倍・出し直しは 1 倍）と標準のツールチップを足した（範囲を登録して文字を預ける静的な使い方・知らせで文字を渡す動的な使い方・安全地帯・重ねた範囲は出ている間も WinUI のように切り替える）。areka.exe とサンプルは comctl32 の版 6 をマニフェストで申告し、版 6 の最大の幅は窓の DPI で割り戻して渡す。完了時の起票 1 本（`emo2-real-run-wrap-timeout`）。`ghost-session-test-load-flake`: 機械が重いときのテストの赤を負荷の再現（`tools/load-flake.ps1`・22 論理 CPU に空回し 44 本）で読み分け、テストの待ちを待ちの部品 `spine_wait.rs`（`wait_until` ほか・進みの目印つき・打ち切りの文言は `［止まった］`・`［進んではいた］`・`［進みは不明］`・`［相手が居ない］`）へ集めた。足場のプールのスレッドを 1 本・GPU の装置の許可を同時に 4 つに絞り、検体の作業フォルダの後片付けの順（os error 5）と偽の SHIORI の時刻しだいの `OnBalloonTimeout` を直した。本番の順序の取り違えは 0 件。開発者の裁定は 2 つ＝案 A（本番を変えず、負荷の下で許す赤の線を steering `tech.md` に書いた）・`areka-nar` の名前替えを外のプロセスの短い掴みの間だけ試し直すために境界を広げた（`rename_patiently`・約 2 秒まで）。最後の負荷の 5 回で許さない赤 0・静かな机 1.06 倍・全体テスト 306 秒。完了時の起票 2 本（`test-wait-marker-gaps`〔バグ〕・`actor-thread-log-capture`〔その他〕）。バルーンの寿命の 3 イベント（`OnBalloonBreak`・`OnBalloonClose`・`OnBalloonTimeout`）を SHIORI へ送り、`\![set,balloontimeout]` を読む（`balloon-lifecycle-events`・10-08・PR#266＝棚卸㉓でこの行に足した） |
| C5 の前（単独） | 10-10 | `farewell-talk-status`（10-10・バグ） | `farewell-talk-status`: 終了の挨拶・切り替えの送り出しの台詞・切り替えの別れの台詞の再生中に、実行の状態へ `talking`（中断の無効化モードなら `nouserbreak` も）を載せる。kanade の「再生中か」の判定 `talk_active_of` を、再生中のトークの番号を引く `current_talk_id` へ委ねた（本番のコードは 1 か所）。`get_status` の答えと、お別れの台詞の `OnTranslate` の `Status` に出る。握手の要求（`OnClose`・`OnGhostChanging`）は会話なしのまま。お別れの間の `\![raise,…]`・バルーンのイベントは今も送らない（brief の読みを訂正）。SSP との差の一覧から該当の行を消した。棚卸㉓の裁定 13 ⑴ のとおり、C5 の着手の前に単独で通した。起票 0 本。 |
| C5（⑥） | 10-10 | `ssp-bts-salvage`（10-10・文書だけ） | `ssp-bts-salvage`: SSP の課題管理（BTS・https://bts.shillest.net/ ）の全 808 件を通読し、深刻度「要望」187 件と、要望・質問・挙動の説明と読めた 47 件を、234 行の台帳にした（`.kiro/specs/completed/areka-P0-ssp-bts-salvage/bts-ledger.md`・13 主題）。areka との関係＝未着手 151・対象外 53・brief あり 15・実装済み 13・方針とぶつかる 2。末尾に起票の候補 88 件（作る話 75・確かめてから 4・決める話 9）。英語の題 176 行は日本語に訳した。一覧は `search.php` の入口から CSV で 1 回取り（絞り込みの欄の送信は匿名では拒まれる）、頁は 237 枚を 1 つずつ順に読んだ。steering `structure.md` に在りかを 1 行。コードは変えていない。同じブランチで開発者の `/kiro-discovery` の起票 5 本（`app-icon`・`tasktray-icon`・`tasktray-ghost-icon`・`tasktray-balloon`・`tasktray-minimize`）。完了時の棚卸＝その場の解決 0 件・起票 0 件 |

- 完了 spec 直下エントリ＝**256**（`.kiro/specs/completed/` 直下・2026-10-10 `ssp-bts-salvage` の完了後の実数え＝フォルダ 255＋`graphics-rendering-stability.md` 1。その前の 255＝2026-10-10 `farewell-talk-status` の完了後。その前の 254＝2026-10-10 `ghost-session-test-load-flake` の完了後。その前の 253＝2026-10-08 `wintf-tooltip` の完了後。その前の 252＝`balloon-lifecycle-events` の完了後。フォルダのうち `install-companion-canon` は実施せず 3 本へ引き継いで閉じたもの）。⚠ **引き算で導かず毎回実数えする**。
- M1 の持ち越しのうち残るのは `dpi-transition-two-tick-bounce`（開発者が許容）と `zorder-chain-residue` A-2（据え置き）だけ。M-dual は退役。
- 個々の完了行の全文（種別・議題・完了時の所見）は history「2026-10-02 棚卸⑳退避」。

### リリース

| 版 | 日付 | GitHub Release | winget-pkgs の PR | 赤で起票した spec |
|---|---|---|---|---|
| v0.0.2 | 2026-10-06 | https://github.com/ekicyou/areka/releases/tag/v0.0.2 | — | — |

## ウェーブ編成（着手順の正本・2026-10-10 棚卸㉓）

> 各ウェーブは**フルライフサイクル**（要件 → 設計 → タスク → 実装 → `/kiro-complete`＝PR の squash マージ）を完走してから次へ。1 spec ＝ 1 worktree ＝ 1 PR。**同じウェーブに入れるのは、棚卸㉓の実測で触るソースファイルの重なりが 0 の組だけ**。許す重なりは 5 つだけ＝表の末尾への追記（`doc/COMPAT_ARCHITECTURE.md` §8）・生成物（`THIRD-PARTY-NOTICES.md`・`doc/ukadoc-coverage/report/`＝手で直さず作り直す）・steering・網羅台帳 `doc/ukadoc-coverage/ledger/*.toml` の**別々の行**・説明書 `dist/README.txt` の**別々の行**。文書だけの段は置かない。
> **並び順の決まり（2026-10-07 開発者・10-05 の 3 段を改めた）**: 依存の木（働きの依存＋同じソースファイルを触る spec＋直列の列の順）の**先頭**だけを候補にし、**① ぶら下がる未完了の spec が多い ② 区分が高い（A ＞ B ＞ C ＞ D）③ 起票が古い**の順に並べ、上から互いに触るファイルが重ならないものを**最大 8 本**取る。据え置き・保留・取り下げ予定は木に入れない。**測ることが本体の spec はウェーブに並べない**（ほかの仕事を汚し、汚される）。区分 A でぶら下がる数が同じものは、開発者が「高」「早めに」と言ったものを先にした（棚卸㉓の読み＝違えば直す）。
> **1 ウェーブに 1 本だけの席**: ⑴ 依存を足す spec と版上げ（`Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`tech.md`）＝C5 は `mcp-stdio-bridge` ⑵ emo-text に新しいソースファイルを足す spec（`lib.rs` の純粋な一覧の数の行＝今 73）＝C5 は `anchor-tag-canon`。
> C4（16 本）は 10-10 にすべて着地＝「完了サマリ」。C4 の行の全文（編成根拠と約束つき）・初回リリースの段取り・棚卸㉒の裁定は history「2026-10-10 棚卸㉓退避」。

| Wave | ユニット（並べた順） | 開始コマンド | 編成根拠（触るファイル・棚卸㉓の実測＝各 brief の「2026-10-10 棚卸㉓の再測定」） |
|---|---|---|---|
| **C5**（8 本並走・Fable 4＋Opus 4・想定タスクの合計＝約 72〜99） | ① `anchor-tag-canon`（C・ぶら下がる約 16 本＝リンクの 3 本と文字とバルーンの列の続き・Fable） ∥ ② `seriko-trigger-intervals`（A 高・約 9 本・口パクの `talk`・Fable） ∥ ③ `property-name-case-fold`（C・7 本・Opus） ∥ ④ `extent-element-offset`（B・約 5 本・Fable） ∥ ⑤ `baseware-root-list`（A・約 4 本・Fable） ∥ ⑥ ✅ `ssp-bts-salvage`（10-10 着地・A 高・文書だけ・Opus） ∥ ⑦ `mcp-stdio-bridge`（A・`Cargo.lock` の席・Opus） ∥ ⑧ `winget-manifest-submission`（A・手提出まで・Opus） | `/kiro-start areka-P0-<名>` | ①＝`crates/areka-parsers/src/sakura/{decode,model,mod}.rs` と `\_a` を素通しと決めているテスト 3 本・`crates/areka-sakura/src/compile.rs`（要れば `drive.rs`・`contract.rs`）・emo-text の `state.rs`・`actor.rs`・`actor_present.rs`・`choice.rs`・`viewbox_draw_render.rs`＋新しいファイルと `lib.rs`（席）・`crates/areka/src/input_events/{balloon,balloon_moved,balloon_pressed,balloon_exit,choice_drain,shell_box,shell_box_handler,user_break}.rs`・kanade の `msg.rs`・`actor.rs`・`lib.rs`・`schedule/{events,mod}.rs`＋新しいファイル・台帳 `sakura-script.toml`・`shiori.toml` の行・COMPAT §8。設計しだいで dola の `cue/command.rs`。②＝`crates/areka-parsers/src/shell/{model,decode}.rs`・`crates/areka-seriko/src/{table,looper,parts,actor,timeline,state}.rs`・台帳 `assets.toml` の行。③＝`crates/areka-sylphya/src/{key,actor,reader}.rs`・`persist/{mod,format}.rs` と兄弟のテスト・`crates/areka/src/mcp/get_property_tests.rs`（1 か所）・`doc/ssp-mcp/survey.md` §7.3・COMPAT §8。④＝`crates/areka-emo-compose/src/plan_extent.rs` と兄弟のテスト（`plan_extent_tests.rs`・`plan_nesting_extent_tests.rs`・`plan_extent_film_tests.rs`・`log_firing_tests.rs`）・COMPAT §8。⑤＝`crates/areka-ghost/src/catalog.rs`・`crates/areka/src/{boot_config,boot_resolve,main,alert}.rs`・`emo2_boot/{ghost_switch,shell_balloon_resolve,shell_balloon_switch}.rs`・`menu/{ghost_frame,balloon_frame}.rs`＋メニューの新しいファイル・`install/{desk,procedure,overwrite}.rs`・`update/desk.rs`・`readme/opener.rs`・`dist/README.txt`（自分の行）・COMPAT §8。⑥＝自分の spec フォルダと `structure.md` の 1〜2 行だけ。⑦＝新しいクレート `crates/areka-mcp-bridge/`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md`（生成物）・`crates/areka-mcp/src/help.rs` と `help_tests.rs`・`tools/package.ps1`・`dist/README.txt`（自分の行）。⑧＝新しい `dist/winget/` だけ。**同じウェーブの約束（破るなら止めて報告）**: ① は `emo2_boot/consumer_ledger.rs` と emo2_boot のほかのファイル・`crates/areka/src/menu/`・seriko・`shell/` の読み手に触らない（`\_a` を汎用の `\!` の運び手で運ぶ形を選ぶと `consumer_ledger.rs` に行が要る＝選ぶなら止めて報告。C5 に同じファイルを触る spec は居ないので、報告の後に約束を解ける）／② は `crates/areka-parsers/src/sakura/`・`crates/areka-sakura/`・`emo2_boot/consumer_ledger.rs`・`crates/areka-emo-compose/` に触らない（`\i[ID]`・`yen-e`・`never`・`animation*.name` は `seriko-script-triggers`、`always` との組み合わせは `seriko-interval-combinations` の持ち物。読み手の型を変えて compose のテストの字面が壊れたら止めて報告＝`plan_extent*.rs` には触らない）／③ は areka-sylphya の外では `get_property_tests.rs` と `survey.md` だけ／④ は `plan_extent.rs` とその兄弟のテストの外に触らない（`plan.rs`・`nesting.rs`・seriko に触るなら止めて報告）／⑤ は areka-sylphya のソースに触らない（根の並びの記憶は今ある書き込みの口を使う）・`emo2_boot/mod.rs`・`consumer_ledger.rs`・`frame/` に触らない・`main.rs`（950 行）と `ghost_switch.rs`（902 行）へは足さず新しいファイルへ・`Cargo.toml` に触らない（`Win32_UI_Shell`・`Win32_System_Com` は根の `Cargo.toml` で有効）。20 タスクを超えたら要件の段で「メニューからの足し引きとフォルダ選びと記憶」を次の spec へ切る／⑥ はコードに触らない。SSP の課題管理（https://bts.shillest.net/）を 1 本の流れで順に読む（並べて取りに行かない・一覧の CSV を落とすなら開発者の許しを先に取る）／⑦ は `crates/areka/src/main.rs`・`crates/areka-mcp/src/{dispatch,server,lib}.rs` に触らない（中継の置き場所は説明の文の側で `areka.exe` の隣から導く）。**`v0.0.3` の版上げは ⑦ の PR が開いていないときに**（下の「版上げと winget の決まり」）／⑧ は `README.md`・`dist/README.txt`・`.github/workflows/` に触らない（それらは `winget-release-automation` の持ち物）。完了の線は「winget-pkgs への PR を出し、自動の検査が緑」 |
| **次のウェーブの候補**（次の棚卸で組む・C5 の着地で brief が動く） | **A**＝`emily-ghost-verification`（C5 に入らなかった先頭・重なり 0）・`mcp-kanade-tools`（kanade の列の頭・ぶら下がる約 12 本・① の後）・`range-choice-tag`（① の後）・`ghost-inner-balloon`（⑤ の後）・`mcp-reload`（⑤ の後＝`ghost_switch.rs`・`shell_balloon_switch.rs`）・`animated-image-import`（② と ④ の後・`collisionex-regions` と読み手を分け合う）・`balloon-font-file`（① の後）・`winget-release-automation`（⑧ の手提出が取り込まれてから）・`script-security-level`（`mcp-kanade-tools` の後）／**B**＝✅ `farewell-talk-status`（10-10 着地・C5 の着手の前に単独で通した）・`dpi-realign-remembered-chain`（重なり 0）・`choice-balloon-timeout-stuck`（出し入れの側で直すなら重なり 0）・`emo2-real-run-wrap-timeout`（重なり 0）・`balloon-text-area-collapse`（① の後）／**C**＝`collisionex-regions`（② の後）・`currentghost-property-tree`（③ の後）・`network-update-canon-order`（⑤ の後）・`balloon-canon-residue`（② の後＝seriko の `actor.rs`）・`choice-ranges-one-function`（① の後）・`reflow-scroll-path-test`（重なり 0）・`placement-measure-bake-once`（重なり 0） | — | 同じ決まりで、先頭を「ぶら下がる数 → 区分 → 古さ」で並べて最大 8 本。C5 に入らなかった理由は「棚卸㉓の裁定」9 |
| **ウェーブの外（測ることが本体・静かな机で 1 本ずつ）** | `areka-test-threads-av` → `test-wait-marker-gaps`（同じテストの実行ファイル `--bin areka` を回す＝この順）・`present-emit-tail-latency`（半分が計測）・`install-live-target-hazards`（測ってから扱いを決める）・`dump-balloon-debug-timeout`（確かめだけ＝次の MCP の実機の確かめに相乗り） | — | 机の空きを開発者が決めたときに 1 本ずつ挟む。重い走行は赤が出たテストに絞り、全体の負荷の走行は最後の 1 回だけ（1 回 約 50 分） |
| **保留** | `tick-gate-adoption` | — | 「長い試行はしない」と両立する短い A/B の測り方を先に組む。計測を汚すので他と並べない |

### 版上げと winget の決まり（棚卸㉒の「初回リリースの段取り」のうち今も効くもの）

- **版上げの PR を出すとき、`Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md` を触る開いた PR は 0 本**。ほかの PR が開いていても、タグは版上げの squash コミットに打つので Release の中身は決まる。字の上で衝突しないまま `Cargo.lock` が古くなる（`--locked` の組み立てが落ちる）のを防ぐため、依存を足す spec とは同じ時に走らせない。C5 では `mcp-stdio-bridge` が席を持つ＝`release-cycle` の `v0.0.3` は、同 spec の着手の前か着地の後。
- **winget の順の決まり**: winget-releaser は既に winget-pkgs に在るパッケージしか更新できない。初回の手提出（人の承認・実例で約 2 日）が取り込まれるまで `winget.yml` を main へ入れない（入れた後にタグを打つと `winget.yml` が赤になる）。棚卸㉓で、手提出まで（`winget-manifest-submission`）と、取り込まれた後の自動化（`winget-release-automation`）に分けた。

### 直列の列（同じ列の spec は同時に走らせない・列が違えば並べられる）

棚卸⑳の再測定で、**同じ場所へ行を足す spec の群れ**が見つかった。ファイルを分割しても重なりは消えない（同じ表・同じ関数へ足すため）。ウェーブは、各列の先頭から 1 本ずつ取り、取った組の接触ファイルを照合してから組む。棚卸㉒で kanade の列の並びを優先度の段に合わせ、シェルの element の列から `self-alpha-declaration` を外した。

| 列 | 重なる場所 | 並び（先頭から・✅＝完了） |
|---|---|---|
| **文字とバルーン** | `crates/areka-emo-text/src/`（`state.rs` の `CueCommand::Custom` の腕・`actor*.rs`・`viewbox_draw*.rs`・**`lib.rs` の純粋な一覧**＝新しいファイルを足す spec はどれも触る）・`crates/areka-parsers/src/balloon/{model,parse}.rs`・`crates/areka/src/input_events/` のバルーンと箱（`shell_box.rs` の `judge_box_press`・`user_break.rs` の `on_box_press`） | ✅ `emo-text-file-split` → ✅ `shell-balloon` → ✅ `shell-balloon-frame-align` → ✅ `budoux-reveal-reflow`（バグ・C4-②・10-06 着地）→ **`anchor-tag-canon`（働き・C5-①）** → `range-choice-tag` → `link-context-copy` → `balloon-link-hover`（10-05 起票の「バルーンのリンクと OS の連携」・開発者の依頼で優先度の高い機能として `text-typesetting` 以降より前へ置いた）→ `balloon-font-file`（`actor_present.rs`・`lib.rs` が ② と、`actor.rs`・`viewbox_draw_render.rs` が `anchor-tag-canon` と重なる。リンクの列のどこかと重ならない席が見つかれば前へ）→ `text-typesetting`（② の「出した字の行を動かさない」約束の上に乗る）→ `talk-fast-forward` → `balloon-markers` → `text-ruby` → `text-reveal-fade` → `balloon-scroll-fade` → `text-align-shadow-canon` → `choice-marker-styling` → `anchor-style-canon`（`look.rs`・`state_decoration.rs`・`viewbox_draw_render.rs`・`viewbox_draw_decoration.rs` を分け合うので直列）→ `text-reveal-dance`（夢）。**`choice-marker-styling` を列から外して並べられた相手（`budoux-reveal-reflow`）は着地した**＝今は列の頭（`anchor-tag-canon`）と `choice.rs` を分け合うので列の中。**列の共通の約束**: 新しいキーはバルーン定義ごとの値（`ResolvedBalloonText` の側）に置く。fade・scroll-fade・dance は「動いている途中の字を見えている字に数えるか」の答えを、当たりの矩形・箱の写し（`actor_box.rs` の `refresh_shown_boxes`）・窓を出す数（`balloon_shown_glyphs`）の 3 か所で揃える。emo-text の `judge_box_click`・`BoxPressVerdict` は 2 値でなく 4 値（棚卸㉑の記述の誤り・棚卸㉒で直した） |
| **シェルの element** | `crates/areka-parsers/src/shell/{model,decode}.rs`・`crates/areka-emo-compose/src/{plan,plan_extent,plan_always,fold,method,atlas_bind}.rs`・`crates/areka-emo-atlas/src/manifest.rs`・`crates/areka-seriko/src/{table,looper,parts,actor}.rs` | ✅ `shell-balloon` → ✅ `surface-element-nesting` → ✅ `element-base-method`（10-05）∥ ✅ `animated-image-playback`（本体・C4-⑫・`bind_atlas` の直後で分解・10-07 着地）→ **`extent-element-offset`（C5-④・`plan_extent.rs` だけ）∥ `seriko-trigger-intervals`（C5-②・棚卸㉓で分けた後＝読み手の `shell/{model,decode}.rs` と seriko だけ）** → `collisionex-regions`（読み手）→ `animated-image-import`（読み手・`manifest.rs`・`plan.rs`・`plan_extent.rs`・seriko の `table.rs`・`parts.rs`）→ `seriko-interval-combinations`（棚卸㉓で切り出し・`plan_extent.rs`・`plan.rs`・`nesting.rs`・`plan_always.rs`）→ `element-clipping-option` → `draw-methods-canon`（読み手と描けるかの門・`plan.rs`・`blit.rs`）→ `draw-methods-blend`（棚卸㉓で切り出し・`method.rs`・`blit.rs`）→ `balloon-element-order`（wintf の兄弟の重なり順の裁定の持ち主）→ `surfaces-basepos`（据え置き）。parsers の `shell/{model,decode}.rs` を触る 5 本（`draw-methods-canon`・`collisionex-regions`・`seriko-trigger-intervals`・`animated-image-import`・`element-clipping-option`）と、compose の `plan.rs`（子の `plan_extent.rs` と、兄弟の `plan_always.rs` を含む）を触る 5 本（`draw-methods-canon`・`extent-element-offset`・`animated-image-import`・`element-clipping-option`・`balloon-element-order`）はそれぞれ互いに直列。**`self-alpha-declaration`（C4-⑬）は列から外した**（本番の決め打ちは `shell_target.rs`・`balloon.rs` の 2 か所だけで、`manifest.rs`・`atlas_bind.rs` の 3 か所は試験の中だった＝棚卸㉑の記述の誤り） |
| **kanade の進行** | `crates/areka-kanade/src/schedule/{steady,events,mod,change,boot}.rs`・`msg.rs`・`actor.rs`・`lib.rs` の `pub use`。**`steady.rs` 950 行・`schedule/mod.rs` 955 行・`msg.rs` 926 行・`actor.rs` 900 行（10-08 `balloon-lifecycle-events` の後）は上限の近く**＝足して 1,000 を超える spec が先頭のタスクで分割するか、新しいファイルへ足す | ✅ `translate-pipeline` → ✅ `mouse-drag-events` → ✅ `balloon-lifecycle-events`（C4-⑩・10-08・`schedule/`）∥ ✅ `mcp-get-status`（C4-⑥・10-06・`msg.rs`・`actor.rs`＝列の外へ出せる組）∥ ✅ `choice-script-prefix`（C4-⑭・10-07・`schedule/choice.rs` とその子・`steady.rs` の `on_choice`＝⑩ と ⑥ のファイルに触らない組。`script:` の台本をトークとして走らせる口は、後の `mcp-kanade-tools` の `sakurascript` が使い回せる）→ `mcp-kanade-tools`（`sakurascript`・`raise_event`・許可の表の迂回を作る）→ `mcp-reload`（`src/change.rs`・`schedule/boot.rs`）→ `script-security-level`（出どころの運搬・host32-host の列も）→ `mcp-strict-errors` ／ その他の段＝`sakura-time-critical` → `property-query-channels`（棚卸㉒で優先の段の後ろへ下げた） |
| **台本のコンパイル** | `crates/areka-sakura/src/compile.rs`・`crates/areka-parsers/src/sakura/{lexer,decode,model}.rs` | ✅ `choice-timeout-directive` → ✅ `open-external-tags`（C4-⑮・`\j` の腕）→ ✅ `mcp-author-tools`（C4-⑨・`parse_noted` の印＝`decode.rs` の各腕が命令と一緒に印を返す形・`compile.rs` の `parse_choice_timeout` の公開。後の spec はこの上で腕を足す）→ **`anchor-tag-canon`（働き・C5-①）** → `range-choice-tag`（`\__q` の腕）→ `talk-fast-forward` → `sakura-time-directives` → `balloon-break-position`（その他の段・`TalkDone` まで位置を通す）。`\i[ID]` は棚卸㉓で `seriko-script-triggers` へ切り出した（`compile.rs`・`sakura/decode.rs` を触る＝`anchor-tag-canon` の後）。`sakura-time-critical` は `\t` を `decode_bare` で写せば `compile.rs` に触れないが、`sakura/decode.rs` はこの列の共有のファイルなので列の中に居る（棚卸㉓） |
| **host32-host** | `crates/shiori-host32-host/src/{shiori3.rs, client.rs}`・`crates/areka-ghost/src/{runtime.rs, shiori_inproc.rs}`・kanade の `shiori/real.rs` | `script-security-level`（`SenderType`・`SecurityLevel` の運搬の持ち主＝棚卸㉒）→ `mcp-shiori-query`（`X-MCP-PassThru-*`）→ `makoto-dll-host`（新規 `makoto.rs`・`runtime.rs`・`Cargo.lock`）→ `makoto-reload-directives` → `property-ipc-transport`（輸送を作るときだけ）。`property-query-channels` は運搬を `script-security-level` に寄せて列から外れた |
| **配布と公開** | `tools/package.ps1`・`.github/workflows/`・`Cargo.toml` 群・`Cargo.lock`・`THIRD-PARTY-NOTICES.md`（版上げで約 30 行）・`dist/README.txt`・`README.md` | ✅ `release-package-versioned` → ✅ `release-ci-workflow` ∥ ✅ `crates-io-publish` → ✅ `tools-utf8-child-output`（バグ・10-06 着地・`package.ps1`・`crates-io.ps1`・`release.yml` の読み方）→ ✅ `release-cycle` の初回（`v0.0.2`・10-06）→ **`winget-manifest-submission`（C5-⑧・手提出まで）** → `winget-release-automation`（棚卸㉓で切り出し・手提出が取り込まれてから）∥ **`mcp-stdio-bridge`（C5-⑦・`Cargo.lock` の席）**→ `install-live-target-hazards`（`dist/README.txt` の既知の制限）→ `release-code-signing`（任意・同梱の i686 `pasta.dll` は署名されないまま残る＝議題） |
| **`emo2_boot` の結線** | `crates/areka/src/emo2_boot/{mod,ghost_switch,consumer_ledger,shell_balloon_switch,switch_assets}.rs`・`frame/{attach,switch,wiring}.rs` | ✅ `shell-balloon-frame-align` → ✅ `balloon-lifecycle-events`（C4-⑩・10-08・`mod.rs`）→ `balloon-font-file`（`frame/attach.rs`・`frame/switch.rs`）→ `balloon-canon-residue` ∥ `shell-companion-balloon`（`frame/switch.rs` を分け合うので同時には走らせない）→ `mcp-reload`（`mod.rs`・`ghost_switch.rs`・`shell_balloon_switch.rs`）。`consumer_ledger.rs` は ✅ `open-external-tags`（C4-⑮・`open` の受け取り手・21 行）と ✅ `mcp-author-tools`（C4-⑨・2 行を足して 23 行）が先に触り、`mcp-reload` は後→ `extra-character-windows`（`switch_assets.rs` も）→ `currentghost-property-tree` → `currentghost-property-others`・`system-property-values` → `property-catalog-lists` |
| **プロパティの動く値** | `crates/areka-sylphya/src/{actor,mirror,key,reader,vocab/dotted}.rs`・`persist/`・`areka-ghost/src/sylphya_wiring.rs` | `property-name-case-fold`（`classify_set` の大小の迂回をふさぐ＝`property-query-channels` が書く道を開く前に着地させる）→ `currentghost-property-tree`（動く値を出す口の持ち主・読む道は `mcp-get-property` が作った）→ `currentghost-property-others` ∥ `system-property-values` → `property-catalog-lists` → `zorder-property` |

- **棚卸㉓で列に足したこと**（表の行は上の書き換えだけ・ここが足し分の正本）:
  - **列をまたぐ重なり**: 文字とバルーンの列の頭 `anchor-tag-canon` は、`OnAnchorSelect` を送るために kanade の `msg.rs`・`actor.rs`・`lib.rs`・`schedule/{events,mod}.rs` も触る＝kanade の列の頭（`mcp-kanade-tools`・`farewell-talk-status`・`script-security-level`・`sakura-time-critical`）と同じウェーブに置けない。
  - **`\!` の名前を足す spec の共有の席**（`mcp-author-tools` の着地でできた）: `check_script` は `crates/areka/src/emo2_boot/consumer_ledger.rs` に無い `\!` の名前を「知らない命令」と答える＝`\!` の名前を足す spec はどれも、この表に行を足し `consumer_ledger_agreement_tests.rs` に見本を足す（`mcp-reload`・`seriko-script-triggers`・`text-typesetting`・`text-ruby`・`text-reveal-fade`・`balloon-markers`・`sakura-time-directives`・`sakura-time-critical`・`property-query-channels`・`sakura-embed-directive`・`makoto-reload-directives`・`inputbox-user-input`）。互いに直列。
  - **カタログの鎖**（`crates/areka-ghost/src/catalog.rs`・`crates/areka/src/{boot_resolve,boot_config}.rs` を 3 本が作り直す）: `baseware-root-list`（C5-⑤）→ `ghost-inner-balloon` → `shell-companion-balloon`。バルーンの項目に「どこから来たか」の欄が要るのは前の 2 本＝先に着地する `baseware-root-list` が決める。`ghost-inner-balloon` の「`shell-companion-balloon` の後」は働きの依存でなくファイルの順だった。
  - **kanade の列**: ✅ `farewell-talk-status`（10-10 着地・`schedule/mod.rs` は判定の本体 1 か所だけ）→ `mcp-kanade-tools`（最初のタスクで `msg.rs` の中のテストを兄弟へ出し、`schedule/mod.rs` を割る。許可の表の迂回は `schedule/change.rs` の `on_raise_event` に作る。`src/change.rs`・`schedule/boot.rs` には触らない＝`mcp-reload` と並べられる）→ `script-security-level` → `mcp-shiori-query`（`mcp-kanade-tools` の後＝棚卸㉒の裁定どおり）→ `mcp-strict-errors`。`mouse-click-wheel-events`・`inputbox-user-input`（棚卸㉓で起票）は `schedule/events.rs` を触るのでこの列。
  - **シェルの element の列の読み手の鎖**（`shell/{model,decode}.rs`）: `seriko-trigger-intervals` → `collisionex-regions` → `animated-image-import` → `element-clipping-option` → `draw-methods-canon`。`config-parse-diagnostics`（棚卸㉓で起票）は読み手の `kv/parse.rs`・`shell/`・`balloon/parse.rs` を触るので、この鎖と文字とバルーンの列の両方が空いたとき。`balloon-canon-residue` は seriko の `actor.rs` を触るので `seriko-trigger-intervals` の後。
  - **文字とバルーンの列で `anchor-tag-canon` の後に並ぶ小さいもの**: `balloon-text-area-collapse`（`actor_present.rs`）・`choice-ranges-one-function`（`choice.rs`・`actor_present.rs`・`actor.rs`）・`font-size-keywords`（棚卸㉓で起票・`look.rs`・`state_decoration.rs`・`mcp/check_script_judge.rs`）。**列の外で取れる**: `reflow-scroll-path-test`（今ある `actor_lookahead_tests.rs` へ足すだけ）・`choice-balloon-timeout-stuck`（出し入れの側 `emo2_boot/balloon_visibility_wait.rs` で直すなら）・`emo2-real-run-wrap-timeout`（`crates/areka/tests/emo2_real_run.rs` だけ）。
  - **列に居なかった spec の置き場**: `emily-ghost-verification`（`areka-nar` の `plan.rs`・`install/judge.rs`・検体の登録＝どの列とも重ならない。見つけた崩れは 1 件ずつ `/kiro-discovery` で起票）・`dpi-realign-remembered-chain`（`placement/` と `emo2_boot/frame/{dpi,drain_resnap}.rs`＝`emo2_boot` の結線の列のファイルには触らない）・✅ `ssp-bts-salvage`（10-10 着地・文書だけ）・`shiori4-api`（host32-host の列の最後）・`seriko-rebuild-hidden-lottery`（`seriko-trigger-intervals` の後）・`test-roots-under-target`（入口の差し替えなら `temp-path-kit` だけ）・`dev-helper-x64-clobber`（`tools/test-all.ps1`）・`actor-thread-log-capture`（`areka-actor`）・`perf-tools-console-encoding`（`tools/perf/`）・`impl-watch`（新しいクレート＝`Cargo.lock` の席）・`elevated-drop-filter`（棚卸㉓で起票・`placement/spawn.rs`）・`check-script-arg-checks`（棚卸㉓で起票・`mcp/check_script_judge.rs` と `emo2_boot/{change_cue,switch_cue,readme_cue}.rs`＝`check_script_judge.rs` を触る spec は互いに直列）。
- **依存を足す spec と版上げは 1 ウェーブに 1 本**（`Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`tech.md` が重なる）: C5 は `mcp-stdio-bridge`（新しいクレート）が席を使う。`impl-watch`・`makoto-dll-host`（新しいクレート）・`system-property-values`（`windows` の機能を足す）・`release-cycle` の版上げも同じ席を使う。
- **網羅台帳をまとめて書き換える `coverage-roadmap-refresh` は、台帳の行を直す spec と同じウェーブに置かない**（台帳に触ると書く未完了の brief が 40 本を超えたので、行の重なりで照合する形を同 spec の議題にした）。
- **列に属さず、いつでも単独で取れる**: `coverage-roadmap-refresh`・`popup-menu-residue`・`clippy-199-lints`（段 1＝`dola` の `validate/`・`runtime/`・`compile/` と `shiori-host32-host/tests/` は空き席でも取れる。`shiori-abi` は `shiori4-api` と分け合う）・`property-name-case-fold`（sylphya だけ）。
- **MCP の 3 段目の約束**: SSP と同じ 10 本の中身を作る spec は、今までどおりツールのファイル（`crates/areka/src/mcp/<ツール>.rs`）と自分のエンジンの側だけを触る（`mcp/mod.rs`・`handler.rs` は触らない）。areka 独自のツールを足す spec（`mcp-user-response`・`mcp-shiori-query`）は `doc/ssp-mcp/areka-tools.md` の「⑹ あとからツールを 1 本足す手順」の表のファイル（`crates/areka-mcp/src/tools/mod.rs`・`handler.rs`・`tools_own_tests.rs`・`tools_own_socket_tests.rs`・`crates/areka/src/mcp/mod.rs` と、その文書）を触るので、互いに、また MCP の共有ファイルを触る他の spec と同じウェーブに置かない。`handler.rs` の `INSTRUCTIONS` に残る「not implemented yet」の 1 文は、今までどおり最後に着地する `mcp-strict-errors` が消す（`mcp-author-tools` が足した独自のツールの 1 文は残す）＝`mcp-reload` は `mcp-strict-errors` より先に着地させる。今も `NG:not implemented yet` を返すのは 10 本のうち 3 本（`sakurascript`・`raise_event`＝`mcp-kanade-tools`・`reload`＝`mcp-reload`）。
- **SHIORI への要求に追加のヘッダを運ぶ道**は `script-security-level`・`property-query-channels`・`mcp-shiori-query` の 3 本が要る。入れ物は先に着手した 1 本が作り、`ShioriBackend` の引数を変えずに既定の実装を持つ新しいメソッドを足す形を推す（実装 19 か所への波及を止める）。

**保存義務（据え置き）**: 既存の終了経路（右クリックメニューの「終了」→ `OnClose` の握手 → `ghost_quit`）の決定論テストを 1 本も落とさない。実機サインオフの「絶対パス起動」（argv 上書き）を残す。

## spec 台帳（brief を持つ 101 本・2026-10-10 `winget-manifest-submission` のブランチが main を取り込んだ後の実数え＝main の 99 本に、同 spec のタスク 3.6 の `/kiro-discovery`（手元の確かめで見つかった件の起票）で起票した 2 本〔`user-data-root`〔優先・区分 A〕・`write-failure-notice`〔バグ・区分 B〕〕が加わった。その前の 99＝2026-10-10 `ssp-bts-salvage` の完了で同 spec を外した（完了時の棚卸の起票は 0 本）。その前の 100＝2026-10-10 `ssp-bts-salvage` のブランチで、`/kiro-discovery`（開発者「タスクトレイの土台仕様を起票したうえで、上流の ukadoc 項目を後続仕様にせよ」「より上位に、アプリアイコンとタスクトレイアイコンを作成して areka.exe に埋め込むスペックを用意しないとダメ」）により `tasktray-icon`・`tasktray-ghost-icon`・`tasktray-balloon`・`tasktray-minimize`・`app-icon`〔どれも その他・区分 A〕の 5 本を起票し、main `226109e8` を取り込んだ（95＋5）。その前の 95＝2026-10-10 `farewell-talk-status` の完了で同 spec を外した（完了時の棚卸の起票は 0 本）。その前の 96＝2026-10-10 棚卸㉓の実数え＝main `ee3af616` の 86 本に、分割で出した 4 本〔`seriko-script-triggers`・`seriko-interval-combinations`・`draw-methods-blend`・`winget-release-automation`〕と、覚え書きからの起票 6 本〔`check-script-arg-checks`・`config-parse-diagnostics`・`font-size-keywords`・`elevated-drop-filter`・`mouse-click-wheel-events`・`inputbox-user-input`〕を足した。その前の 86＝2026-10-10 `/kiro-discovery`（開発者「SHIORI4 の API 整備の spec はあるか」）で `shiori4-api`〔その他・区分 C〕を起票した。その前の 85＝2026-10-10 `/kiro-discovery`（開発者「SSP の課題管理は要件の宝庫」「早めに実施したい」）で `ssp-bts-salvage`〔優先・区分 A〕を起票した。その前の 84＝2026-10-10 `ghost-session-test-load-flake` へ main `e04e6209` を取り込み、main で起票済みの `impl-watch`〔その他・PR#277・台帳の行が無かった〕を足した。その前の 83＝2026-10-10 `ghost-session-test-load-flake` の完了で同 spec を外し、完了時の棚卸で `test-wait-marker-gaps`〔バグ〕・`actor-thread-log-capture`〔その他〕を起票した〔差し引き +1〕。その前の 82＝2026-10-08 `wintf-tooltip` の完了で同 spec を外し、完了時の棚卸で `emo2-real-run-wrap-timeout`〔バグ〕を起票した〔差し引き 0〕。その前の 82＝2026-10-08 `balloon-lifecycle-events` の完了で同 spec を外した（完了時の棚卸の起票は 0 本）。その前の 83＝2026-10-08 main `8d5c70fb` の取り込みで、`balloon-lifecycle-events` のブランチで起票済みの 2 本〔`balloon-break-position`〔その他〕・`choice-balloon-timeout-stuck`〔バグ〕〕を足した。その前の 81＝2026-10-08 `mcp-author-tools` の完了で同 spec を外した。その前の 82＝2026-10-08 `char-position-save-on-exit` の完了で同 spec を外し、同 spec のブランチで起票済みの `dpi-realign-remembered-chain`〔バグ〕〔2026-10-05・設計ディスカッション 議題 1〕を main の取り込みで足した〔差し引き 0〕。その前の 82＝2026-10-08 `/kiro-discovery`（開発者「SSP から取り出した、えみりゴーストがうまく動かないらしい」）で `emily-ghost-verification`〔優先・区分 A〕を起票した。その前の 81＝2026-10-07 `animated-image-playback` の完了で外した。その前の 82＝2026-10-07 `animated-image-playback` の完了時の棚卸で 3 本〔`present-emit-tail-latency`・`balloon-text-area-collapse`〔ともにバグ〕・`seriko-rebuild-hidden-lottery`〔その他〕〕を起票した。その前の 79＝2026-10-07 `choice-script-prefix` の完了で外した。その前の 80＝2026-10-07 `ghost-standard-balloon` の完了で外した。その前の 81＝2026-10-07 `ghost-standard-balloon` の完了時の棚卸で 3 本〔`dump-balloon-debug-timeout`〔バグ〕・`test-roots-under-target`・`dev-helper-x64-clobber`〔ともにその他〕〕を起票した。その前の 78＝2026-10-07 `ghost-standard-balloon` へ main を取り込んだ後の実数え＝main の 77 本に、2026-10-05 `ghost-standard-balloon` の要件の討議の `/kiro-discovery` で起票した `ghost-inner-balloon`〔優先〕を足した。その前の 77＝2026-10-06 `tools-utf8-child-output` の完了で外し、完了時の棚卸で `perf-tools-console-encoding`〔その他〕を起票した。その前の 77＝2026-10-06 `mcp-ghost-name-match` の完了で外した。その前の 78＝2026-10-06 `mcp-dump-images-residue` の完了で外した。その前の 79＝2026-10-06 `budoux-reveal-reflow` の完了で外し、完了時の棚卸で `choice-ranges-one-function`・`reflow-scroll-path-test`〔ともに優先〕を起票した。その前の 78＝2026-10-06 `self-alpha-declaration` の完了で外した。その前の 79＝2026-10-06 `open-external-tags` の完了で外した。その前の 80＝2026-10-06 `mcp-get-status` の完了で外した。その前の 81＝2026-10-05 `mcp-get-status` の要件ディスカッションで `farewell-talk-status`〔バグ〕を起票した。その前の 80＝2026-10-05 `element-base-method` の完了で外し、完了時の棚卸で `draw-methods-canon`〔優先〕を起票した。その前の 80＝2026-10-05 `/kiro-discovery`（`release-cycle` の初回の実装からの依頼）で `tools-utf8-child-output`〔バグ〕を起票した。その前の 79＝2026-10-05 `/kiro-discovery`「ゴーストフォルダの複数管理」で 2 本〔`baseware-root-list`・`dev-folder-alias`〔ともにその他〕〕を起票した。その前の 77＝2026-10-05 棚卸㉒の実数え＝main `ec072853` の 75 本〔`/kiro-discovery`「バルーンのリンクと OS の連携」で 7 本〔`choice-script-prefix`・`open-external-tags`・`range-choice-tag`・`link-context-copy`・`wintf-tooltip`・`balloon-link-hover`〔以上 優先〕・`shell-tooltip`〔その他〕〕を起票した〕に、切り出し 2 本〔`mcp-get-status`・`animated-image-import`〕を足した。その前の 68＝`mcp-dump-images` の完了で外し、完了時の棚卸で `mcp-dump-images-residue`〔優先〕を起票した。その前の 68＝`/kiro-discovery` で `budoux-reveal-reflow`〔バグ〕を起票した。その前の 67＝`mouse-drag-events` の完了で外し、完了時の棚卸で `element-base-method`〔バグ〕・`char-position-save-on-exit`〔バグ〕・`areka-test-threads-av`〔バグ〕・`collisionex-regions`〔優先〕を起票した。その前の 64＝2026-10-05 `/kiro-discovery` で「MCP の areka 独自ツールと影響の段」の 5 本〔`mcp-author-tools`・`mcp-user-response`・`mcp-shiori-query`・`script-security-level`・`script-impact-tiers`〕を起票した。その前の 59＝`surface-element-nesting` の完了で外し、完了時の棚卸で `extent-element-offset`〔バグ〕を起票した。その前の 59＝`drag-cancel-borrow-miss` の完了で外した。その前の 60＝`shell-balloon-frame-align` の完了で外した。その前の 61＝`mcp-log-history` の完了で外した。その前の 62＝`mcp-log-history` の完了時の棚卸で `ghost-session-test-load-flake`〔バグ〕を起票した。その前の 61＝`animated-image-decode` の完了で外した。その前の 62＝`animated-image-decode` の実装で `element-clipping-option`〔優先〕と完了時の棚卸で `placement-measure-bake-once`〔優先〕を起票した。その前の 60＝`host32-testdll-marker-race` の完了で外した。その前の 61＝`mcp-expression-table` の完了で外した。その前の 62＝`install-companion-reading` の完了で外した。その前の 63＝`mcp-get-property` の完了で外した。その前の 10-04 の数え＝`mcp-get-property` の要件ディスカッションで `property-name-case-fold`・`mcp-ghost-name-match` を起票して 64 本・2026-10-04 棚卸㉑の実数え＝53 本に、切り出し 5 本〔`anchor-style-canon`・`sakura-embed-directive`・`makoto-reload-directives`・`currentghost-property-others`・`system-property-values`〕と覚え書きからの起票 4 本〔`self-alpha-declaration`・`seriko-trigger-intervals`・`extra-character-windows`・`update-check-options`〕を足した。それより前の数えの経緯は history「2026-10-04 棚卸㉑退避」）

> **spec は名前で呼ぶ**（2026-09-26 開発者指示）: 報告・brief・コミット・PR で spec を指すときは spec 名（`areka-P0-` は省略してよい）を書く。「#数字」は `PR#185` の形の PR 番号にだけ使う。古い文書に台帳番号が出てきたら、その時点の表（history）で名前へ読み替える。
> **2026-10-10 の改め（棚卸㉓）**: 着手の並びは 4 区分（生きている決まり 1）。「段」の列の末尾に **区分**（A 開発者が起票を頼んだ／B バグ／C ukadoc の拾い残し・バグでない持ち越し／D 夢・据え置き・保留）を足した。区分の数え＝**A 39・B 10・C 44・D 8 ＝ 101**（2026-10-10 `winget-manifest-submission` のブランチが main を取り込んだ後の実数え＝`user-data-root`〔A〕・`write-failure-notice`〔B〕の起票が加わった。その前の A 38・B 9・C 44・D 8 ＝ 99 は、2026-10-10 `ssp-bts-salvage`〔A〕の完了後の実数え。その前の A 39・B 9・C 44・D 8 ＝ 100 は、10-10 にタスクトレイの 4 本と `app-icon`〔どれも A〕を足した数。その前の A 34・B 9・C 44・D 8 ＝ 95 は、2026-10-10 `farewell-talk-status` の完了後の実数え。棚卸㉓の時点は A 34・B 10・C 44・D 8 ＝ 96）。段の語（バグ／優先／その他／据え置き／保留）は 10-03・10-05 の分け方の名残で、並びには使わない。区分の読みに迷った行（開発者の指示が「分け方」だけ・名指しの範囲外の切り出し）は C に置いた＝「棚卸㉓の裁定」12。
> **段＝優先度の 3 段**（2026-10-03 開発者「1 バグ・2 リリース関係・バルーン関係・アニメーション画像関係・3 その他」）: **バグ**＝1 段目／**優先**＝2 段目（配布と公開・文字とバルーンの列・シェルの element の列・動く画像・バルーンのイベントと残件・**SSP MCP の移植 10 本**＝10-03 に開発者が追加・`mouse-drag-events`＝10-04 に開発者が個別に上げた）／**その他**＝3 段目（ゴーストのインストール・kanade・プロパティ・道具）／**据え置き**＝当面着手しない（理由は行に）／**保留**。どのウェーブに居るかは「並び」の列。完了した spec はこの表に置かない（完了サマリと history）。
> **段の数え方**: `awk '/^\| spec（`areka-P0-` 省略）/{f=1;next} f&&/^\|/{print} f&&!/^\|/{f=0}' .kiro/steering/roadmap.md | awk -F'|' 'NR>1{gsub(/ /,"",$3);print $3}' | sort | uniq -c`（2026-10-10 `winget-manifest-submission` のブランチが main を取り込んだ後の実数え＝バグ 10・優先 42・その他 41・据え置き 7・保留 1 ＝ 101〔優先 1 本〔`user-data-root`〕とバグ 1 本〔`write-failure-notice`〕が加わった〕。その前の `ssp-bts-salvage` の完了後の実数え＝バグ 9・優先 41・その他 41・据え置き 7・保留 1 ＝ 99〔優先 1 本〔同 spec〕を外した〕。その前のタスクトレイの 4 本と `app-icon` の起票の後の実数え＝バグ 9・優先 42・その他 41・据え置き 7・保留 1 ＝ 100〔その他 5 本を足した。段の欄には「・区分 A」などが付くので、段の語で数えるときは `・` より後ろを落とす〕。その前の `farewell-talk-status` の完了後の実数え＝バグ 9・優先 42・その他 36・据え置き 7・保留 1 ＝ 95〔バグ 1 本〔同 spec〕を外した。優先のうち 2 本は「優先（**高**…）」と書いた行〕。その前の棚卸㉓の後の実数え＝バグ 10・優先 42・その他 36・据え置き 7・保留 1 ＝ 96。その前の 2026-10-10 `shiori4-api` の起票の後の実数え＝バグ 10・優先 38・その他 30・据え置き 7・保留 1 ＝ 86〔その他 1 本〔同 spec〕を足した〕。その前の `ssp-bts-salvage` の起票の後の実数え＝バグ 10・優先 38・その他 29・据え置き 7・保留 1 ＝ 85〔優先 1 本〔同 spec〕を足した〕。その前の main `e04e6209` の取り込みの後の実数え＝バグ 10・優先 37・その他 29・据え置き 7・保留 1 ＝ 84〔その他 1 本〔`impl-watch`〕を足した〕。その前の `ghost-session-test-load-flake` の完了後の実数え＝バグ 10・優先 37・その他 28・据え置き 7・保留 1 ＝ 83〔バグ 1 本〔同 spec〕を外し、バグ 1 本〔`test-wait-marker-gaps`〕とその他 1 本〔`actor-thread-log-capture`〕を足した〕。その前は 2026-10-08 `wintf-tooltip` の完了後の実数え＝バグ 10・優先 37・その他 27・据え置き 7・保留 1 ＝ 82〔優先 1 本〔同 spec〕を外し、バグ 1 本〔`emo2-real-run-wrap-timeout`〕を足した〕。その前は 2026-10-08 `balloon-lifecycle-events` の完了後の実数え＝バグ 9・優先 38・その他 27・据え置き 7・保留 1 ＝ 82〔優先 1 本〔同 spec〕を外した〕。その前は 2026-10-08 main `8d5c70fb` の取り込み後の実数え＝バグ 9・優先 39・その他 27・据え置き 7・保留 1 ＝ 83〔`balloon-lifecycle-events` のブランチで起票済みのバグ 1 本〔`choice-balloon-timeout-stuck`〕とその他 1 本〔`balloon-break-position`〕を足した〕。その前は 2026-10-08 `mcp-author-tools` の完了後の実数え＝バグ 8・優先 39・その他 26・据え置き 7・保留 1 ＝ 81〔優先 1 本〔同 spec〕を外した〕。その前は 2026-10-08 `char-position-save-on-exit` の完了後の実数え＝バグ 8・優先 40・その他 26・据え置き 7・保留 1 ＝ 82〔バグ 1 本〔同 spec〕を外し、バグ 1 本〔`dpi-realign-remembered-chain`〕を足した〕。その前は 2026-10-08 `emily-ghost-verification` の起票後の実数え＝バグ 8・優先 40・その他 26・据え置き 7・保留 1 ＝ 82〔優先 1 本を足した〕。その前は 2026-10-07 `animated-image-playback` の完了後の実数え＝バグ 8・優先 39・その他 26・据え置き 7・保留 1 ＝ 81〔優先 1 本を外した。その前の 82＝2026-10-07 `animated-image-playback` の完了時の棚卸の後の実数え＝バグ 8・優先 40・その他 26・据え置き 7・保留 1〔バグ 2 本・その他 1 本を足した。その前の 79＝2026-10-07 `choice-script-prefix` の完了後の実数え＝バグ 6・優先 40・その他 25・据え置き 7・保留 1〔優先 1 本を外した。その前の 80＝2026-10-07 `ghost-standard-balloon` の完了後の実数え＝バグ 6・優先 41・その他 25・据え置き 7・保留 1〔優先 1 本を外した。その前の 81＝2026-10-07 `ghost-standard-balloon` の完了時の棚卸の後の実数え＝バグ 6・優先 42・その他 25・据え置き 7・保留 1 ＝ 81〔バグ 1 本・その他 2 本を起票した。その前の 78＝2026-10-07 `ghost-standard-balloon` へ main を取り込んだ後の実数え＝バグ 5・優先 42・その他 23・据え置き 7・保留 1 ＝ 78〔`ghost-inner-balloon`〔優先〕を足した。その前の 77＝2026-10-06 `tools-utf8-child-output` の完了後の実数え＝バグ 5・優先 41・その他 23・据え置き 7・保留 1 ＝ 77〔バグ 1 本を外し、その他 1 本を起票した。その前の 77＝2026-10-06 `mcp-ghost-name-match` の完了後の実数え＝バグ 6・優先 41・その他 22・据え置き 7・保留 1〔優先 1 本を外した。その前の 78＝2026-10-06 `mcp-dump-images-residue` の完了後の実数え＝バグ 6・優先 42・その他 22・据え置き 7・保留 1〔優先 1 本を外した。その前の 79＝2026-10-06 `budoux-reveal-reflow` の完了後の実数え＝バグ 6・優先 43・その他 22・据え置き 7・保留 1〔バグ 1 本を外し、優先 2 本を起票した。その前の 78＝2026-10-06 `self-alpha-declaration` の完了後の実数え＝バグ 7・優先 41・その他 22・据え置き 7・保留 1〔優先 1 本を外した〕。その前の 79＝2026-10-06 `open-external-tags` の完了後の実数え＝バグ 7・優先 42・その他 22・据え置き 7・保留 1〔優先 1 本を外した〕。その前の 80＝2026-10-06 `mcp-get-status` の完了後の実数え＝バグ 7・優先 43・その他 22・据え置き 7・保留 1〔優先 1 本を外した〕。その前の 81＝2026-10-06 `farewell-talk-status` を main へ取り込んだ後の実数え＝バグ 7・優先 44・その他 22・据え置き 7・保留 1 ＝ 81〔`element-base-method`〔バグ〕の完了で外し `draw-methods-canon`〔優先〕を足した後に、`farewell-talk-status`〔バグ〕を足した。その前の 80＝2026-10-05 `tools-utf8-child-output` の起票後の数え＝バグ 7・優先 43・その他 22・据え置き 7・保留 1〔バグ 1 本を足した。その前の 79＝「ゴーストフォルダの複数管理」でその他 2 本を足した。その前の 77＝棚卸㉒の数え＝「バルーンのリンクと OS の連携」の 7 本〔優先 6・その他 1〕と切り出し 2 本〔優先〕を足した。その前の 68＝`mcp-dump-images` の完了で外し、完了時の棚卸で `mcp-dump-images-residue`〔優先〕を起票した。その前の 68＝`budoux-reveal-reflow`〔バグ〕を起票した。その前の 67＝`mouse-drag-events` の完了で外し、完了時の棚卸で 4 本〔バグ 3・優先 1〕を起票した。その前の 64＝「MCP の areka 独自ツールと影響の段」の 5 本〔優先〕を起票した。その前の 59＝`surface-element-nesting` の完了で外し、完了時の棚卸で `extent-element-offset`〔バグ〕を起票した。その前の 59＝`drag-cancel-borrow-miss` の完了で外した。その前の 60＝`shell-balloon-frame-align` の完了で外した。その前の 61＝`mcp-log-history` の完了で外し、完了時の棚卸で `ghost-session-test-load-flake`〔バグ〕を起票した。その前の 61＝`animated-image-decode` の完了で外し、`element-clipping-option`・`placement-measure-bake-once`〔優先〕を起票した。その前の 60＝`host32-testdll-marker-race` の完了で外した。その前の 61＝`mcp-expression-table` の完了で外した。その前の 62＝`install-companion-reading` の完了で外した。その前の 63＝`mcp-get-property` の完了で外した。その前の 64＝棚卸㉑の 62 に `property-name-case-fold`・`mcp-ghost-name-match` を足した〕。それより前の数えの経緯は history「2026-10-04 棚卸㉑退避」）
> **規模**は棚卸㉓の再測定の見立て（タスク数）。**上限は 1 spec 20 タスク**。「要件で切る」と書いた行は、着手のときに要件の段で切り出す（先に起票しない＝spec 工場の禁止）。**Fable**列＝要件定義を Fable で起動したセッションで進めることを勧める（○）か、Opus で足りる（−）か。各 brief の末尾「2026-10-10 棚卸㉓の再測定」（と分割・測定・申し送りの節）が、崩れた前提・触るファイル・議題の正本。**「並び」「前提」の列には起票や前の棚卸の時点の書き方が残る行がある**＝食い違うときは brief の節と「ウェーブ編成」が正本。

| spec（`areka-P0-` 省略） | 段 | 何をするか | 規模 | 並び | 前提（先に着地） | Fable |
|---|---|---|---|---|---|---|
| `choice-balloon-timeout-stuck`（**10-05 起票**・`balloon-lifecycle-events` の設計で発見） | バグ・区分 B | 選択肢を含む台詞のバルーンが、選んだ後も・選択肢の時間切れで解除した後も時間切れで消えず、`OnBalloonTimeout` も届かない。文字の層の「選択肢が表示中か」（`TextLayerRuntime::choice_active`）が「選択肢の行が残っているか」で答え、行は内容の消去でしか消えないため、抑止が次のトークまで効き続ける（ソースの読みで確認・実機は未確認） | S（3〜5・文字の層で直すなら 5〜7） | 文字とバルーンの列（`areka-emo-text` の `actor*.rs`・`state.rs`）・`balloon-lifecycle-events` の後 | `balloon-lifecycle-events` | ○ |
| `emo2-real-run-wrap-timeout`（**10-08 起票**・`wintf-tooltip` の完了時に発見） | バグ・区分 B | 明示したときだけ走る `emo2_real_run`（`AREKA_EMO2_REAL_RUN=1`）が `wrap=BudouxWordWrap` を待つ所で赤。200% の画面の debug ビルドで文字の取り付けが起動から約 2.5 秒かかり、3 秒の自動終了に間に合わない見込み（マニフェストを外しても同じ＝前からの問題）。時間を測って切り分け、自動終了の残りに頼らず観測で待つ形にする | S（3〜6） | 単独（`crates/areka/tests/emo2_real_run.rs`・要れば `emo2_boot` の装着の段） | なし | − |
| `test-wait-marker-gaps`（**10-10 起票**・`ghost-session-test-load-flake` の完了時の棚卸） | バグ・区分 B | テストの待ちの残りを steering `tech.md` の「テストの待ちと、負荷の下で許す赤」に揃える。目印が UI の側の仕事を数えない待ち（`abort_tests` の 1 本・`［止まった］` が実質 30 秒の壁時計）・目印が条件と同時にしか動かない `join_bounded`（spine と install の写し 2 つ。install の 2 つは許す赤の範囲の外＝目印を良くするか、開発者の裁定で範囲を広げる）・`areka-nar` の短い掴みの檻（放すまで約 200 ms 対 試し直し約 2 秒）・上限の無い `GpuPermit::take`・目印の無い `spin_wait_until` の直接の呼び出し 15 か所（`［進みは不明］` を出しうる） | M（8〜11） | ウェーブの外（測ることが本体・`areka-test-threads-av` の後） | `areka-test-threads-av`（同じテストの実行ファイル） | ○ |
| `extent-element-offset`（**10-05 起票**・`surface-element-nesting` の完了時の棚卸） | バグ・区分 B | 外形（`flatten_extent`・10-06 に `animated-image-playback` が `plan.rs` から `plan_extent.rs` へ移した）が画像の element定義の X,Y を数えず、ベースの画像の外へはみ出す画像の element が記録なしに切られる（emo-compose を作った時から・入れ子の子の中の element も漏れる）。ukadoc の外形の定義を確かめ、全命令の和集合に揃えるか、ベースで切ると明文化して記録を出すかを裁定する | S（3〜6） | **C5-④**・シェルの element の列 | なし（`element-base-method`・`animated-image-playback` は着地済み） | ○ |
| `draw-methods-canon`（**10-05 起票**・`element-base-method` の完了時の棚卸） | 優先・区分 A | `overlay`・`base` 以外の描画メソッド（`overlayfast`・`replace`・`interpolate`・`asis`・`reduce`・`blend-*` ほか）を element定義・pattern定義で描き、pattern定義の `base` も描く。開発者の方針（10-05）「Direct2D が支える全部に対応できるよう、wintf も含めて広げる」。合成を CPU の整数の転写のまま広げるか D2D へ移すかは要件・設計で決める。読み手の `is_image_element_method` を広げ `Element` に欄を足すと、描けない行の警告の対象が自動で減る **棚卸㉓で分けた**: `blend-*` は `draw-methods-blend` へ。 | M〜L（13〜16・棚卸㉓で分けた後） | シェルの element の列（`element-clipping-option` の後） | `element-base-method`（✅ 10-05 完了） | ○ |
| `choice-ranges-one-function`（**10-06 起票**・`budoux-reveal-reflow` の完了時の棚卸） | 優先・区分 C | 選択肢の強調とクリックを受ける範囲を出す本番の手順（`present_actor` の中の `annotate_lines` → `line_bands` → `derive_hit_rows`）を関数 1 つにまとめ、検査（`actor_lookahead_shapes_tests.rs` の検査 8）が並びを組み直さずにそれを呼ぶ形へ。今は本番の並びが変わっても檻が気付けない。見た目・クリックの結果は変えない | S（3〜4） | 文字とバルーンの列（`anchor-tag-canon` の後＝`choice.rs`・`actor_present.rs`・`actor.rs`） | `budoux-reveal-reflow`（✅ 10-06 完了） | − |
| `reflow-scroll-path-test`（**10-06 起票**・`budoux-reveal-reflow` の完了時の棚卸） | 優先・区分 C | 区間の全文で配置する枝と、あふれのスクロール（`visible_window`）を組み合わせて踏む経路の検査を足す（既存の `actor_scroll_retain_tests.rs` は先渡しなしで流すので届いた字の枝しか通らない＝`budoux-reveal-reflow` の要件 4.2・4.3 は構造の保証だけ）。検査だけで本番は変えない | S（2〜3） | いつでも（今ある `actor_lookahead_tests.rs` へ足すだけ） | `budoux-reveal-reflow`（✅ 10-06 完了） | − |
| `dpi-realign-remembered-chain`（**10-05 起票**・`char-position-save-on-exit` の設計の検証 議題 1・開発者「案イを起票」） | バグ・区分 B | 拡大率が変わったあとの詰め直し（`placement/chain_realign.rs`）は、位置を覚えているキャラクターを対象から外す。`char-position-save-on-exit` が入ると 2 回目の起動から全員が対象外になり、隣り合っていた 2 人のあいだに隙間（実測 359px）が開いたままになる。隣り合っていた組は覚えていても隣り合ったままに見えるようにする（決め方は要件の段で裁定） | S〜M（6〜9） | いつでも（`placement/` と `emo2_boot/frame/{dpi,drain_resnap}.rs`・重なり 0） | なし（`char-position-save-on-exit` は着地済み） | ○ |
| `areka-test-threads-av`（**10-05 起票**・`mouse-drag-events` の 2.2 の検証で発見） | バグ・区分 B | `cargo test -p areka --bin areka -- --test-threads=4` が `STATUS_ACCESS_VIOLATION` で落ちる（drag のテストを外しても・既定のスレッド数では落ちない・再現する）。二分探索で落ちる組を特定し、スレッドに縛られる資源の扱いを直す。再現しなければ記録して据え置きへ | S〜M（5〜9） | ウェーブの外（測ることが本体・`test-wait-marker-gaps` より先） | なし | ○ |
| `collisionex-regions`（**10-05 起票**・`mouse-drag-events` の実機で発見） | 優先・区分 C | `collisionex`（矩形・円・楕円・多角形の当たり判定）を読まない（`decode_collisions` は数字だけの `collisionN` だけ・台帳 `absent`）。`collisionex` だけで書くクローディアでは全マウスイベントの Reference4 がいつも空。読み手に形を足し、`hit.rs` で形ごとに内外を判定する | S〜M（8〜11） | シェルの element の列（読み手の鎖＝`seriko-trigger-intervals` の後） | `seriko-trigger-intervals`（同じ `shell/{model,decode}.rs`） | − |
| `install-live-target-hazards` | その他・区分 C | 表示中のシェル・使用中のバルーンへの上書きと、起動中のゴーストへ入れる途中の Windows の終了を**実測してから**扱いを決める | S〜M（8〜14・うち計測 5〜6） | ウェーブの外（測ってから扱いを決める・`balloon-font-file` を待たない） | なし | ○ |
| `shell-companion-balloon`（**10-03 起票**・同 3/3） | 優先・区分 C | シェルの書庫の同梱バルーンをそのシェルに紐づけ、着替えと起動で使う（今はシェルごとのバルーンが無い） | M（10〜14） | カタログの鎖の 3 本目・`emo2_boot` の列 | `baseware-root-list`・`ghost-inner-balloon`（同じ `catalog.rs`・`boot_resolve.rs`・`boot_config.rs`） | ○ |
| `ghost-inner-balloon`（**10-05 起票**・`ghost-standard-balloon` の要件の討議から） | 優先・区分 A | ゴーストが自分のフォルダの中の `balloon/<名>/` にバルーンを抱え、起動・メニュー・切替・記憶で根の置き場のものと同じに扱う（ukadoc に無い areka 独自・書庫からの入れ方も含む） | M〜L（12〜18） | カタログの鎖の 2 本目 | `baseware-root-list`（同じ 3 ファイル） | ○ |
| `release-cycle`（**10-02 起票**・配布と公開・**繰り返し spec**） | 優先・区分 A | 開発者が実装を打ったときだけ版を +0.0.1 → PR → squash マージ → タグ。初回が `v0.0.2`（`wintf`・`dola` の Trusted Publishing の設定もこの中） | XS〜S（初回 8〜11・2 回目から 6） | **C4-⑤**（初回リリース・`Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md` を触る開いた PR が 0 本なら他と並べてよい＝棚卸㉒） | `release-ci-workflow`・`crates-io-publish`・✅ **`tools-utf8-child-output`**（10-06 着地・初回の実装は main を取り込んで段 3 を頭から回し直す） | − |
| `winget-manifest-submission`（**10-02 起票**・配布と公開） | 優先・区分 A | winget の名乗り `Areka.Areka.Portable`（zip＋portable・x64 と arm64）・初回の手提出・以後は winget-releaser。`Areka.Areka` はインストーラー版のために空けておく **棚卸㉓で分けた**: `winget.yml` と説明書の行は `winget-release-automation` へ。 | S（5〜7・棚卸㉓で分けた後） | **C5-⑧**（`dist/winget/` だけ） | なし（`v0.0.2` は 10-06 に公開済み） | − |
| `balloon-font-file` | 優先・区分 A | `font.name`・`\f[name]` のフォントファイル | M（8〜11） | 文字とバルーンの列（`anchor-tag-canon` の後＝`actor.rs`・`viewbox_draw_render.rs`・`lib.rs`）・`emo2_boot` の列 | `shell-balloon-frame-align`（✅ 10-05 完了） | − |
| `anchor-tag-canon` | 優先・区分 C | `\_a` のリンクの**働き**（今は押しても何も起きない＝優先度 高）・`\_a[OnID,…]` の形・箱の押下の結論。見た目は既定の 1 種類（装飾は 10-04 に `anchor-style-canon` へ切った） | M（13〜16） | **C5-①**（文字とバルーンの列と台本のコンパイルの列の頭・kanade の列とも重なる） | なし（`shell-balloon`・`choice-timeout-directive` は着地済み） | ○ |
| `text-typesetting` | 優先・区分 A | 禁則・ぶら下げ・縦中横・字の向き（areka 独自） | L（17〜22・超えたら要件の段で「禁則とぶら下げ」と「縦中横と字の向き」に） | 文字とバルーンの列 | `shell-balloon` | ○ |
| `talk-fast-forward` | 優先・区分 A | クリックでの早送り・`\x`／`\x[noclear]`・`clickwaitmarker.*` | L（18〜23・超えたら要件の段で「クリックの早送り」と「`\x` と印」に） | 文字とバルーンの列・コンパイルの列 | `shell-balloon` | ○ |
| `balloon-markers` | 優先・区分 A | `arrow*` と手動スクロール・`onlinemarker`・`number`・SSTP の印（表示だけ）・装飾の系列 | L（18〜22・超えたら要件の段で「矢印と手動スクロール」と「状態の印」に） | 文字とバルーンの列 | `shell-balloon` | ○ |
| `text-ruby` | 優先・区分 A | ルビ `\![text,ruby,親文字,ルビ]`・`line_height`・`letter_spacing` | M〜L（13〜17） | 文字とバルーンの列 | `text-typesetting` | ○ |
| `text-reveal-fade` | 優先・区分 A | 1 字ずつの表示で字が透明から不透明へ変わる（既定は無効） | M（10〜13） | 文字とバルーンの列 | `shell-balloon` | ○ |
| `balloon-scroll-fade` | 優先・区分 A | 自動スクロールで押し出される行のフェード（既定は無効） | S〜M（6〜9） | 文字とバルーンの列 | `balloon-markers` | − |
| `text-align-shadow-canon` | 優先・区分 C | `\f[align]`／`\f[valign]`・影。**20 を超えそうなら「影」と「寄せ」に切る** | L（18〜22・一度切り出した spec＝超えたら要件の段で影 → 寄せ） | 文字とバルーンの列 | `text-typesetting` | ○ |
| `choice-marker-styling` | 優先・区分 C | `\f[cursor*]` 10 項目と本物の下線 | S（5〜8・descript の `cursor.notselect.*` まで入れると 9〜12） | 文字とバルーンの列 | `anchor-tag-canon`（同じ `choice.rs`） | − |
| `animated-image-import`（**10-05 起票**・棚卸㉒で `animated-image-playback` から切り出し） | 優先・区分 A | pattern定義の描画メソッド `import` の画像を名前を保って読み、焼いて置く（今は読み手がファイル名を 0 に化かし記録も出ない） | M（10〜14） | シェルの element の列（読み手の鎖＝`collisionex-regions` の後・`extent-element-offset` の後） | `seriko-trigger-intervals`・`extent-element-offset`（ファイルの重なり） | ○ |
| `element-clipping-option`（**10-05 起票**・`animated-image-decode` のタスク 6.3＝要件 9.3） | 優先・区分 C | element定義の `--clipping,左,上,右,下` で矩形だけを描く・付けた element定義では動く絵として読まない（正典 C2）・オプションの並びを読み手で転記（`--alpha`・`--source`・`--scaling` の描画は範囲外） | M（11〜15） | シェルの element の列（`element-base-method`・`animated-image-playback`・`extent-element-offset`・`animated-image-import` の後） | `animated-image-decode`・`surface-element-nesting`・`element-base-method` | ○ |
| `placement-measure-bake-once`（**10-05 起票**・`animated-image-decode` の完了時の棚卸＝research.md 9.14 節） | 優先・区分 C | 起動の採寸が絵を全部焼いて寸法だけ使い、動く絵の全コマを読んでは捨てる（上限いっぱいの絵 1 つで起動の焼く時間が release 0.19 → 3.2 秒）。バルーンも scope ごとに採寸と資産組み立てで 2 回ずつ焼く。焼くのを 1 回にするか、採寸を全コマ無しで済ませる | S〜M（案 A 10〜14／案 B 5〜8） | いつでも（案 B は `placement/measure.rs` と emo-present の `shell_target.rs`） | `animated-image-decode` | − |
| `balloon-element-order` | 優先・区分 A | `balloon` の element定義を並び順どおりの重ね順で描く。wintf の兄弟の重なり順（描画と当たり判定が逆）の裁定の持ち主 | M（10〜14） | シェルの element の列 | `shell-balloon`・`animated-image-playback` | ○ |
| `property-query-channels` | その他・区分 C | `\![get/set,property]`・`%property[…]`・`SenderType`（`\![embed]` は 10-04 に `sakura-embed-directive` へ切った）。許可の表の迂回を最初に決める | M（11〜14） | kanade の列（その他の段＝`mcp-kanade-tools` の後）。host32-host の列からは外れた | `translate-pipeline`・`property-name-case-fold`・`mcp-kanade-tools`（迂回の口） | ○ |
| `balloon-break-position`（**10-05 起票**・`balloon-lifecycle-events` の要件の討議） | その他・区分 C | `OnBalloonBreak` の Reference2（中断位置＝台本の先頭からの文字数・タグも数える）。台本の中の位置を字句から命令・cue・再生・完了の知らせ `TalkDone`（27 ファイル・60 か所）まで通す。数え方・どの台本か・待機の途中の 3 つを要件で決める | M（9〜13） | 台本のコンパイルの列の最後・kanade の列 | `balloon-lifecycle-events` | ○ |
| `network-update-canon-order` | その他・区分 C | 更新のイベントを ukadoc の発生順序に揃える（MD5 の取り直し・台詞の終わりを待つ・総括の 2 段） | M（10〜14） | `baseware-root-list` の後（同じ `update/desk.rs`） | `baseware-root-list`（ファイルの重なり） | ○ |
| `balloon-canon-residue` | 優先・区分 C | 残り（項目 2＝面の偶奇で左右のバルーンを選ぶ・普通のバルーンだけ、を先に。項目 4 `\![reload,balloon]` は 10-04 に `mcp-reload` へ移した） | M（10〜14） | `emo2_boot` の列・`seriko-trigger-intervals` の後（seriko の `actor.rs`） | `seriko-trigger-intervals`（ファイルの重なり） | − |
| `currentghost-property-tree` | その他・区分 C | 動く値を出す口（持ち主）＋`currentghost.balloon.scope` の 19 項目（残りは 10-04 に `currentghost-property-others` へ切った） | M（13〜16） | `emo2_boot` の列・プロパティの動く値の列 | `property-name-case-fold`（同じ `actor.rs`） | ○ |
| `property-catalog-lists` | その他・区分 C | 一覧（`ghostlist`・`balloonlist`・`activeghostlist`）と基盤待ちの枝の登記（`system.*` は 10-04 に `system-property-values` へ切った） | M（10〜14） | プロパティの動く値の列 | `currentghost-property-tree` | − |
| `zorder-property` | その他・区分 C | `currentghost.seriko.zorder` の読み書き | S（5〜8・書き込みの届け先を作るなら 8〜11） | プロパティの動く値の列 | `currentghost-property-tree`（動く値の口）・書く側は `property-query-channels` | − |
| `sakura-time-directives` | その他・区分 C | `quicksection`・`balloonwait`（`\_q` は議題。`balloontimeout` は 10-05 に `balloon-lifecycle-events` へ移した）。C・D 群は覚え書きへ戻し、`\![embed]` は `sakura-embed-directive` へ（10-04） | M（8〜12） | コンパイルの列 | `choice-timeout-directive`・`talk-fast-forward` | − |
| `sakura-time-critical`（**10-03 起票**・`emo-text-file-split` の実機の観測から） | その他・区分 C | 台本の `\t`（タイムクリティカル）を読み、台本の終わりか中断・選択まで、マウス系などの通知を止めて `Status` に `timecritical` を載せる（今は読まない＝`\t` を書いたメニューもなでなでの返事に置き換わる） | S〜M（8〜11） | kanade の列・台本のコンパイルの列（`sakura/decode.rs`） | `status-execution-states` | ○ |
| `makoto-dll-host` | その他・区分 C | MAKOTO/2.0 の DLL のホストと起動時の鎖（後から差し替えられる入れ物の形）。`\![load/unload/reload,makoto]` は 10-04 に `makoto-reload-directives` へ切った | L（15〜18） | host32-host の列 | なし（`Cargo.lock` の席・host32-host の列の順） | ○ |
| `mcp-stdio-bridge` | 優先・区分 A | Claude Desktop 用の stdio ⇔ HTTP 中継 exe | S〜M（8〜12） | **C5-⑦**（新しいクレート＝`Cargo.lock` の席） | なし（`mcp-server-core`・`release-package-versioned` は着地済み） | − |
| `mcp-kanade-tools` | 優先・区分 A | `sakurascript`・`raise_event`（`get_status` は 10-05 に `mcp-get-status` へ切った）。許可の表を迂回して任意の名前のイベントを送る口を作る（`property-query-channels`・`mcp-shiori-query` が使う） | M〜L（12〜16） | kanade の列の頭（次のウェーブの候補＝`anchor-tag-canon` の後） | `anchor-tag-canon`・`farewell-talk-status`（ファイルの重なり） | ○ |
| `mcp-reload` | 優先・区分 A | `reload` と台本の `\![reload,…]`（`\![reload,balloon]` も持つ＝10-04 の裁定） | L（17〜22・超えたら `\![reload,shiori]` を切る） | `emo2_boot` の列（`baseware-root-list` の後＝`ghost_switch.rs`・`shell_balloon_switch.rs`） | `baseware-root-list`（ファイルの重なり） | ○ |
| `mcp-strict-errors` | 優先・区分 A | `strict`（不在の面・未知のタグなどをエラーログへ） | S〜M（6〜10）か M〜L（12〜18）＝入口で診るか、再生まで印を運ぶか | MCP の最後・kanade の列 | `mcp-log-history`・`mcp-kanade-tools` | ○ |
| `mcp-user-response`（**10-05 起票**・同） | 優先・区分 A | 選択肢・入力欄の答えと、なでる・クリックなどの操作を、`since_id` 付きでエージェントへ返すツール 1 本（さくらスクリプトでは届かない返り道・SSP にも無い） | M〜L（12〜18） | kanade の列（choice とマウスの配線） | `mcp-author-tools`・`mcp-kanade-tools`・`mouse-drag-events` | ○ |
| `mcp-shiori-query`（**10-05 起票**・同・開発者「SHIORI への情報要求が無い・raise_event はトークになるだけ」） | 優先・区分 A | ゴーストへ情報を問うツール 1 本＝SSTP NOTIFY と同じ振る舞いに `X-MCP-PassThru-*` の往復を足す（作法は正典の `X-SSTP-PassThru-*` を写し、名前の頭だけ経路に合わせる＝開発者裁定・ghost_terminal の `ShioriEcho` 系が実例）。データはヘッダで返り、台本が空なら喋らない。SHIORI リソースを外から直接読む形は正典に無い＝要件の議題（推しは採らない） | M〜L（12〜16） | kanade の列（`actor_resources` の隣・host32-host の `shiori3.rs` も触る＝`script-security-level` と同じウェーブに置かない） | `mcp-kanade-tools`・`script-security-level`（ヘッダの入れ物） | ○ |
| `script-security-level`（**10-05 起票**・同・開発者「セキュリティレベルでさくらスクリプトに制約が出る・areka に未実装」） | 優先・区分 A | 台本とイベントに出どころを付けて運ぶ。SHIORI へ `SecurityLevel`・`SenderType` を正典の値で渡し、応答の `SecurityLevel` を読み、ukadoc が「外部からは不可」と書いたタグを止める（MCP は SSP と同じく Owned SSTP＝`local`） | L（16〜22・超えたら要件の段で「ヘッダ」と「台本の印とタグの制約」に） | C5 の候補・kanade の列（host32-host の `shiori3.rs` も触る） | `mcp-kanade-tools`（kanade の 5 ファイルの重なり） | ○ |
| `script-impact-tiers`（**10-05 起票**・同） | 優先・区分 A | タグの影響の段＝高（ベースウェアの外）・中（ベースウェアの中で演技を超えるもの）・低（アクターの演技の範疇・ゴースト切替を含む＝**制約なし**・開発者確定）の正本の表と、出どころごとの扱い（中・高は要件の議題） | M（10〜14） | `script-security-level` の後 | `script-security-level`・`mcp-kanade-tools` | ○ |
| `range-choice-tag`（**10-05 起票**・同） | 優先・区分 A | `\__q[ID,…]…\__q`（範囲を丸ごと選択肢にする・複数行・画像のバナー）。未実装・引受先なしだった（`anchor-tag-canon` の brief の「実装済み」は誤り＝同 brief を直した） | M（10〜14） | 文字とバルーンの列・台本のコンパイルの列（`anchor-tag-canon` の次） | `anchor-tag-canon`・`choice-script-prefix` | ○ |
| `link-context-copy`（**10-05 起票**・同・**areka 独自の拡張**＝ukadoc に右クリック・クリップボードの正典は無い） | 優先・区分 A | 選択肢・アンカー・範囲の選択肢を右クリックすると小さなメニュー（`menu/win32.rs` を使い回す）を出し、行き先をクリップボードへ。決め方は 3 段＝`script:` の中の開く系の行き先 → ID や引数の `http(s)://`・`file:///`・`mailto:` → 表示されている文字（開発者裁定） | S〜M（7〜10） | 文字とバルーンの列（`input_events/` のバルーンと箱の押下） | `open-external-tags`・`anchor-tag-canon`・`range-choice-tag` | − |
| `balloon-link-hover`（**10-05 起票**・同・開発者「マウスホバーって出せる？ URL とか」） | 優先・区分 A | 正典の `balloon_tooltip`・`OnChoiceHover`・`OnAnchorHover` を実装し、ゴーストが何も返さないときだけ行き先（`link-context-copy` の 1・2 段目）を出す | M（10〜13） | 文字とバルーンの列と kanade の列（`link-context-copy` の次） | `wintf-tooltip`・`link-context-copy`・`anchor-tag-canon`・`range-choice-tag` | ○ |
| `shell-tooltip`（**10-05 起票**・同） | その他・区分 A | キャラクター窓のツールチップ＝surfaces.txt の tooltipブレス（`sakura.tooltips` など・今は黙って吸収）・SHIORI の `tooltip`・`currentghost.seriko.tooltip.*`。どれも引受先なしだった | M（8〜12） | シェルの element の列とプロパティの動く値の列（着手の前に照合） | `wintf-tooltip` | − |
| `baseware-root-list`（**10-05 起票**・ゴーストフォルダの複数管理） | その他・区分 A | 根を「`c:\areka` → `c:\ssp`」のように並びで持ち、上から探して最初に見つかったものを使う（`PATH` と同じ・下の同名は隠れる＝見分ける鍵はフォルダ名のまま）。新規のインストールは先頭の根・既存への上書きはその場。`AREKA_ROOT` の `;` 区切り・メニューの「根を追加…」「根を外す ▸」（OS のフォルダ選択）。SSP のフォルダ設定の個別の項目は写さない | L（16〜20） | **C5-⑤**（カタログの鎖の 1 本目） | なし | ○ |
| `dev-folder-alias`（**10-05 起票**・同） | その他・区分 A | 開発中の git の作業ツリーを「名前 → フォルダ」で登録し、探す順のいちばん上に置く（ゴースト・バルーン・ゴーストごとのシェル）。同じ名前の入っているものを隠して開発版を動かせる。エイリアスの先へはインストールも更新も書き込まない。登録は App の記憶へ手で書く＋環境変数（メニューなし） | M（8〜12） | `baseware-root-list` の後・同じ列 | `baseware-root-list` | − |
| `perf-tools-console-encoding`（**10-06 起票**・`tools-utf8-child-output` の完了時の棚卸） | その他・区分 C | 性能改善ループの道具（`tools/perf/`）の `Invoke-PwshChild` が、親と共有の端末の上で子の中の `[Console]::OutputEncoding` を UTF-8 へ書き替える（注記「子の中だけ」と実際がずれる＝共有の端末のコードページが変わりうる）。端末を書き替えずに読む側で直し、端末へ出す文の扱いと `tools/encoding-check.ps1` の対象に `tools/perf/` を入れるかを決める | S（5〜8） | いつでも（`tools/perf/` だけ・ほかの spec と重なり 0） | なし | − |
| `dump-balloon-debug-timeout`（**10-07 起票**・`ghost-standard-balloon` の完了時の棚卸） | バグ・区分 B | debug 版の areka で MCP の `dump_balloon` が呼び出し側の時間切れ（"task cancelled"）で答えを返さなかった（10-06・`mcp-dump-images-residue` の着地の前のコード）。まず main の debug 版で再現を確かめ、残っていれば原因を直す。再現しなければ記録して閉じる | XS（確かめだけ 1〜2・再現すれば 4〜7） | ウェーブの外（次の MCP の実機の確かめに相乗り） | `mcp-dump-images-residue`（✅ 10-06 完了） | − |
| `test-roots-under-target`（**10-07 起票**・`ghost-standard-balloon` の完了時の棚卸） | その他・区分 C | 既存のテストの `TempPath::new`（OS の一時フォルダ・80 ファイル 240 か所）を `target\test-roots` の下へ移し、`%TEMP%` を使うテストが黙って入らないようにする（入口を替えるか呼び出しを置き換えるかは要件で決める） | S〜M（入口の差し替え 5〜8） | いつでも（`temp-path-kit` とテストだけ・`ghost-session-test-load-flake`・`areka-test-threads-av` と同時に走らせない） | `ghost-standard-balloon`（`TempPath::under_target`） | − |
| `dev-helper-x64-clobber`（**10-07 起票**・`ghost-standard-balloon` の完了時の棚卸） | その他・区分 C | 全体テストの後は `target\debug\shiori-host32-helper.exe` が x64 版になり、手で起こす `target\debug\areka.exe` が 32bit の SHIORI を `0x800700C1` で読めない。開発時の helper の置き場の罠を解き、向きが違えば記録を 1 行出す | S（3〜5） | いつでも（`tools/test-all.ps1` か host-32 の起動だけ） | なし | − |
| `present-emit-tail-latency`（**10-07 起票**・`animated-image-playback` の完了時の棚卸） | バグ・区分 B | シェルの絵の適用の約 1.2% が 16 ms を超える（最大 315〜360 ms）。全部が `presenter/show.rs` の最後に記録した段から `timing.emit(` までの測っていない区間。区間に段を足して原因を特定し、根本を直す。動く絵は適用の回数を約 13 倍にするので本数も増える | S〜M（5〜10・半分は計測） | ウェーブの外（測ることが本体） | `animated-image-playback`（✅ 10-07） | − |
| `balloon-text-area-collapse`（**10-07 起票**・`animated-image-playback` の完了時の棚卸） | バグ・区分 B | バルーンの文字の描画範囲が 0 以下に潰れると、文字の層が毎フレーム `create_composition_swap_chain` に失敗し（0x887A0001）、WARN 1 行と ERROR 2 行をゴーストが終わるまで出し続ける。0 寸なら供給面を作らず描かない・記録は状態が変わったときの 1 回 | S（4〜6） | 文字とバルーンの列（`anchor-tag-canon` の後＝`actor_present.rs`） | `anchor-tag-canon`（ファイルの重なり） | − |
| `seriko-rebuild-hidden-lottery`（**10-07 起票**・`animated-image-playback` の完了時の棚卸） | その他・区分 C | `parts.rs` の `rebuild` の 1 回目が、外側が別のコマに居る刻みでも見えていない部品の `random` の抽選を回すことがある（見た目の誤りは未確認・乱数の消費の並びが変わるだけ）。見える部品だけを引くか、今の決まりとして記録して閉じるかを要件で決める | XS〜S（記録だけ 1〜2・直すなら 4〜6） | `seriko-trigger-intervals` の後（同じ `parts.rs`） | なし | − |
| `actor-thread-log-capture`（**10-10 起票**・`ghost-session-test-load-flake` の完了時の棚卸） | その他・区分 C | `areka-actor` の `spawn_actor` が、テストの記録の捕捉（`log_capture_kit::capture`・呼んだスレッドだけ）を子のスレッドへ引き継がない。背景のスレッド（`install`・`kanade` など）の失敗の記録（パスと OS のエラー）がテストの赤の文言に出ない。本番の記録を変えずに、捕まえているテストへ届くようにする（走り始めのフックは `fn(&str)` で状態を持てず、枠は `thread_roles` が使っている） | S（4〜6） | いつでも（`areka-actor` と捕捉の檻だけ・`areka-actor` に触る spec と同時に走らせない） | なし | ○ |
| `impl-watch`（**10-10 起票**・main の PR#277・`/kiro-discovery`） | その他・区分 C | 机の貸し出し（マージの机・負荷テストの机・停止要請と再開）を、調停役の Claude セッション（スキル `kiro-watch`）からコンソールアプリ `areka-impl-watch` へ移す。各セッションが Bash から直に呼び、LLM のメッセージを使わない（送信の上限・調停役が落ちたときの取りこぼし・`target/` の掃除で状態が消える、を断つ）。2026-10-08 の規則 7 つはそのまま引き継ぐ | M〜L（15〜19） | いつでも（新しいクレート＝`Cargo.lock` の席・スキル 3 本の書き替えは続きの spec） | なし | ○ |
| `coverage-roadmap-refresh`（**10-02 に覚え書き `ukadoc-coverage-custody` を合流**） | その他・区分 C | 網羅台帳の手書きの数と持ち主が黙って偽になるのを止める（検査が `completed/` を走査・128 行の持ち主の付け替え・実在しない仕様名 107 項目） | M（14〜18） | 台帳の行の状態や持ち主を変える spec が 1 本も走らない席（注記だけ足す spec とは並べられる） | なし | ○ |
| `popup-menu-residue` | その他・区分 C | メニューの残件（記録の文言・テストの穴。World 借用中の `ShellExecuteW`〔項目 10〕は 2026-10-06 `open-external-tags` で解消＝開く処理を専用のスレッドへ移した）。次に `menu/` を触る spec へ相乗りが安い | S（4〜6・残り 7 件） | 単独で取れる | なし | − |
| `clippy-199-lints` | その他・区分 C | clippy 1.99 で既存のコードに出た lint（dola・emo-compose・kanade・shiori 系・areka の約 40＋19 か所）を、振る舞いを変えずに消して `cargo clippy --workspace --all-targets -- -D warnings` を緑にする。test-all に clippy の段を足すかは議題 | M（12〜16・測り直しで 20 を超えたら段 1 と段 2 に） | C5 の候補（段 1＝`dola`・`shiori-abi`・`areka-mcp` は空き席でも取れる・段 2 は列が空いてから・最初のタスクで測り直す） | なし | − |
| `anchor-style-canon`（**10-04 起票**・`anchor-tag-canon` から切り出し） | 優先・区分 C | `\f[anchor*]` 16 項目×3 状態・descript の `anchor(.notselect|.visited).font.*`・訪問済み・縦書きの下線（普通のバルーンと箱） | M（10〜13・descript の `anchor.*` の残りまで入れると 12〜16） | 文字とバルーンの列（`choice-marker-styling` の隣） | `anchor-tag-canon`・`text-align-shadow-canon` | ○ |
| `seriko-trigger-intervals`（**10-04 起票**・同 ⑺） | 優先（**高**＝10-05 開発者・棚卸㉒の後・`animated-image-playback` の要件討議の議題 4「引受先のロードマップ優先度を高優先度に」）・区分 A | **口パクの `talk` が動かない**のを直す。`runonce`・`never`・`yen-e`・`periodic` と `\i[ID]`（`\i[ID,wait]` とアニメーションから別のアニメーションを呼ぶメソッドは別途）。**`always` を含む組み合わせ（`bind+always` ほか・`+` の語順と 3 語以上の読み方）も引き受ける**（`animated-image-playback` からの申し送り・同 brief） **棚卸㉓で分けた**: 残すのは `talk`・`runonce`・`periodic`。`yen-e`・`never`・`\i[ID]`・`animation*.name` は `seriko-script-triggers` へ、`always` との組み合わせは `seriko-interval-combinations` へ。 | M（12〜15・棚卸㉓で分けた後） | **C5-②**・シェルの element の列（読み手の鎖の頭） | なし | ○ |
| `extra-character-windows`（**10-04 起票**・覚え書き「3 人目以降のキャラクターの窓」） | その他・区分 C | `char{n≧2}` を持つゴーストで 3 人目からも窓とバルーンを出す（今は `derive_scopes` が 0・1 に固定） | M〜L（14〜20） | `emo2_boot` の列 | `balloon-canon-residue`・`shell-companion-balloon` | ○ |
| `update-check-options`（**10-04 起票**・覚え書き「更新のオプションと `OnUpdateCheck*`」） | その他・区分 C | 更新のオプション（`checkonly`・`testonly`・`recovery`）・`OnUpdateCheck*` の 4 語・`other_homeurl_override`（今は受けて何もしない） | M（10〜14） | `network-update-canon-order` の後（同じ `update/`・`update_cue.rs`） | `network-update-canon-order` | ○ |
| `sakura-embed-directive`（**10-04 起票**・`property-query-channels` から切り出し） | その他・区分 C | `\![embed]`（SHIORI へ問い合わせた結果を台本へ埋め込む） | M（12〜16） | kanade の列の後 | `property-query-channels` | ○ |
| `makoto-reload-directives`（**10-04 起票**・`makoto-dll-host` から切り出し） | その他・区分 C | `\![load/unload/reload,makoto]`・切替や更新の後の付け直し・`mcp-reload` の口 | M（7〜10） | host32-host の列 | `makoto-dll-host` | − |
| `currentghost-property-others`（**10-04 起票**・`currentghost-property-tree` から切り出し） | その他・区分 C | `currentghost.*` の残り（`balloon.scope` の 19 項目の外） | M（12〜16） | プロパティの動く値の列 | `currentghost-property-tree` | − |
| `system-property-values`（**10-04 起票**・`property-catalog-lists` から切り出し） | その他・区分 C | `system.*` と Win32 の採り口 | M〜L（14〜18・`windows` の機能を足す＝`Cargo.toml` の席） | プロパティの動く値の列 | `currentghost-property-tree` | − |
| `property-name-case-fold`（**10-04 起票**・`mcp-get-property` の要件ディスカッションから） | その他・区分 C | プロパティの名前の英字の大小を区別せずに引く（SSP 2.9.07 に合わせる・ukadoc は黙っている）。書き込みの検査の大小の迂回もふさぐ。3 つの口（`GetProperty`・`%property[]`・`get_property`）は読み手に追従するだけ | S〜M（5〜9） | **C5-③**・プロパティの動く値の列の頭 | なし | − |
| `seriko-script-triggers`（**10-10 起票**・棚卸㉓で `seriko-trigger-intervals` から切り出し） | 優先・区分 A | SERIKO の起動のきっかけのうち台本の側＝`yen-e`・`never`・台本の `\i[ID]`・`animation*.name` の読み取り | S〜M（8〜10） | シェルの element の列・台本のコンパイルの列（`anchor-tag-canon` の後） | `seriko-trigger-intervals`・`anchor-tag-canon` | ○ |
| `seriko-interval-combinations`（**10-10 起票**・棚卸㉓で同 spec から切り出し） | 優先・区分 A | `bind+always` など、`always` と組み合わせた interval を動かす（正典は「`+` でつなげる」の 1 文だけ＝どの語を・どの順で・いくつまでが議題） | S（4〜6・`always` を含まない組み合わせまで入れると +1〜2） | シェルの element の列（compose の `nesting.rs`・`plan.rs`・`plan_always.rs`・`plan_extent.rs` と seriko の `table.rs`・`parts.rs`・`looper.rs`＝`extent-element-offset`・`animated-image-import`・`seriko-script-triggers`・`draw-methods-canon` と直列） | `seriko-trigger-intervals`・`extent-element-offset` | ○ |
| `draw-methods-blend`（**10-10 起票**・棚卸㉓で `draw-methods-canon` から切り出し） | 優先・区分 A | `blend-*` の描画メソッド 28 種と `-fast` の形 27・古い別名 2 を描く（compose の `method.rs`・`blit.rs`・台帳の 57 行。読み手には触らない＝`draw-methods-canon` の読み手が `blend-*` の語も運ぶ形にする。ukadoc は計算式を書いていない＝議題） | M（9〜12） | シェルの element の列（`draw-methods-canon` の後） | `draw-methods-canon` | ○ |
| `winget-release-automation`（**10-10 起票**・棚卸㉓で `winget-manifest-submission` から切り出し） | 優先・区分 A | `.github/workflows/winget.yml` と `README.md`・`dist/README.txt` の行。初回の手提出が winget-pkgs に取り込まれてから始める | XS〜S（3〜4） | 配布と公開の列（`winget-manifest-submission` の後） | `winget-manifest-submission`＋winget-pkgs の取り込み＋`user-data-root`（着地まで着手しない・次の版を出さない） | − |
| `check-script-arg-checks`（**10-10 起票**・棚卸㉓で覚え書きから） | その他・区分 C | 各機能の受け口が自分で読む引数の誤りを `check_script` が診る。10 個のうち 7 個（`\![move]`・`\![set,zorder]`・`\f`・`\_l`・`update`・`install`・`bind`）は判定が既に純粋な関数＝呼ぶだけ。受け口の中で判定している 3 個（`change`・ゴーストとシェルの切替・readme）を取り出す | M（8〜12） | `mcp/check_script_judge.rs`（`mcp-strict-errors`・`script-impact-tiers`・`anchor-tag-canon`・`talk-fast-forward`・`font-size-keywords` ほかと分け合う）・`emo2_boot/{change_cue,switch_cue,readme_cue}.rs` | なし | − |
| `config-parse-diagnostics`（**10-10 起票**・棚卸㉓で覚え書きから・優先度は低い） | その他・区分 C | 設定の読み取り（`parse_kv`・`shell::parse`・`balloon::parse`）が捨てた行・読まなかったキーを言えるようにする（値で返す形を推す・ログの上限と水準が議題） | M（9〜13） | 読み手の鎖（シェルの element の列）と文字とバルーンの列の両方が空いたとき | なし | − |
| `font-size-keywords`（**10-10 起票**・棚卸㉓で覚え書きから） | その他・区分 C | `\f[height,…]` のスタイルシートのキーワード（`larger` など）で字の大きさを変える（今は語彙だけ受けて記録を出す。ukadoc は語の一覧も大きさの対応も書いていない＝areka の決まりを作る） | S（3〜5） | 文字とバルーンの列（`look.rs`・`state_decoration.rs`）と `mcp/check_script_judge.rs` | なし | ○ |
| `elevated-drop-filter`（**10-10 起票**・棚卸㉓で覚え書きから） | その他・区分 C | 管理者として動かした areka へ投げ込みを届けるか（Windows の権限の境界をゆるめる）、既知の制限のまま閉じるかを決める（説明書には既に制限として書いてある＝やらないなら直しは 0） | S（2〜4・やらないと決めれば 0） | `placement/spawn.rs`（`dpi-realign-remembered-chain`・`extra-character-windows` と分け合う） | なし（開発者の決めごとが議題 1） | ○ |
| `mouse-click-wheel-events`（**10-10 起票**・棚卸㉓で覚え書きから） | その他・区分 C | 単押しのクリックとホイール（`OnMouseClick`・`OnMouseClickEx`・`OnMouseWheel`）を SHIORI へ送る。正典では `OnMouseClick` は「離したときに `OnMouseUp` へ返事が無ければ」送る＝`OnMouseUp`・`OnMouseDown` の組（これも持ち主なし）をどうするかが議題。待って見分ける仕組みは要らない（離した時に送る） | S〜M（6〜10・`OnMouseUp` の組を先に送るなら 10〜13） | kanade の列（`schedule/events.rs`・`msg.rs`）と `input_events/mod.rs`・要れば `menu/trigger.rs` | なし | ○ |
| `inputbox-user-input`（**10-10 起票**・棚卸㉓で覚え書きから） | その他・区分 C | 入力欄 `\![open,inputbox]`・`\![close,inputbox,ID]` と `OnUserInput`・`OnUserInputCancel`（パスワード・日付・スライダーなどの兄弟の入力欄は範囲外） | M〜L（Win32 標準の入力なら 12〜16・バルーンに合わせた箱なら 20 を超える＝要件で切る） | `emo2_boot` の列（`consumer_ledger.rs`・`mod.rs`）と kanade の列（`schedule/events.rs`） | なし（Win32 標準の入力か、バルーンに合わせた箱かが議題 1） | ○ |
| `surfaces-basepos` | 据え置き・区分 D | surfaces.txt の `point.basepos`。宣言するシェルが要るまで始めない | S | シェルの element の列の最後 | — | − |
| `property-ipc-transport` | 据え置き・区分 D | 台本を通さないプロパティの読み取り。**最初の 1 段（調べて裁定）で終わる形へ縮める** | XS〜S（裁定まで 2〜4） | — | — | ○ |
| `dpi-transition-two-tick-bounce` | 据え置き・区分 D | 拡大率の切替で位置と大きさが 1 コマずれる（開発者が許容）。着手するなら最初に測り直し、跳ねが無ければ取り下げ | S〜M | — | — | ○ |
| `zorder-chain-residue` | 据え置き・区分 D | 間欠赤のテスト族と文書。09-11 以降 main で赤 0 件＝先回りしない（A-2 は 10-05 に `ghost-session-test-load-flake` へ移した） | S〜M | — | — | ○（A 群） |
| `text-reveal-dance` | 据え置き・区分 D | 字が現れるときだけ跳ねる・揺れる（夢・開発者が望んだときだけ） | M〜L | — | `text-reveal-fade` | ○ |
| `emo-text-canon-residue` | 据え置き・区分 D | **取り下げ予定・着手しない**。残っていた 1 件（折り返しの警告にバルーンの名前を出す）は `shell-balloon` が引き取った。フォルダを消すと網羅台帳の整合検査（`roadmap-draft.md` の `[[spec]]` の表）が赤くなるので、表から外すのと一緒に `coverage-roadmap-refresh` が片付ける | — | — | — | − |
| `release-code-signing`（**10-02 起票**・配布と公開・任意） | 据え置き・区分 D | SignPath Foundation（オープンソース向け・無償）に申請し、受理されたら CI で署名する。winget に署名は要らないので急がない | S（6〜9） | 配布と公開の列の最後 | `release-ci-workflow`・数回のリリースの実績 | − |
| `tick-gate-adoption` | 保留・区分 D | 既定で切の門を入れるか（待機中の CPU 17〜22% → 3% 未満の見込み）。短い A/B の測り方を先に組む | M〜L | 保留 | — | ○ |
| `emily-ghost-verification`（**10-08 起票**・開発者「えみりゴーストがうまく動かないらしい・何とか動くところまで」・区分 A） | 優先・区分 A | SSP 同梱の既定ゴースト「Emily/Phase4.5」（YAYA 32bit・3 人・`.pna` 27 枚・オーナードローのメニュー）を検体 `emily4` として起こし、「起動・会話・撫で・メニュー・終了」まで通す。第一の壁＝書庫に `balloon/` が無いのに `balloon.source.directory,balloon` を名乗り、`areka-nar` が `CompanionSourceMissing` でインストール全体を断る（その 1 つだけ飛ばすか、検体を直すかを要件で裁定）。検体は 10-08 に `vendors/sample_ghost/emily4.nar` へ置いた（CC BY-NC 4.0・登記表は未登記＝この spec で登記）。範囲外の未対応は全部起票 | M（8〜12） | 次のウェーブの先頭の候補（`areka-nar` の `plan.rs`・`install/judge.rs`・検体の登録＝重なり 0。見つけた崩れは 1 件ずつ起票） | なし | ○ |
| `shiori4-api`（**10-10 起票**・開発者「SHIORI4 の API 整備の spec はあるか・どこかの段階で着手しないといけない」・区分 C） | その他・区分 C | areka 独自の SHIORI の正準形式（SHIORI4・COM の `IShiori`）の API を整える。いまは最低限の決まりだけで、本番の経路は SHIORI4 を通らず（`ShioriBackend` の番号付き Reference → `shiori-host32-host` が SHIORI/3.0 を組む）、SHIORI4 ⇄ SHIORI3 のアダプタは注釈だけ、紙の契約（JSON-RPC・`doc/shiori/fragments/`）を読むコードは無く、InProc の経路が流すのも SHIORI/3.0 のテキスト。方針（10-10 開発者）＝SHIORI4 と SHIORI3 は重ねず並べる＝「areka(kanade) → SHIORI の共通の窓口 → SHIORI3／SHIORI4」（SHIORI3 への変換の層は今のまま。06-30 の「本体は `IShiori` だけを握る」を改める）。すること＝プロパティへの入口の整理・**シンク系インターフェース（SHIORI からベースウェアへの問い合わせ＝`IShioriHost` のプロパティの読み書きなど）を本番で使えるように**・中身の形式を 1 つに。細かい調査と仕様の検討は着手のときに行う | L を超える見込み（要件で切る） | 直列の列「host32-host」の後ろ（`shiori3.rs`・`client.rs`・`shiori_inproc.rs`・`runtime.rs`・kanade の `shiori/real.rs`）。`property-ipc-transport` と主題が重なるので着手のときに持ち分を決め直す。許可の表を決める `property-query-channels` の後 | 実装がある程度整備されてから（開発者） | ○ |
| `app-icon`（**10-10 起票**・開発者「より上位に、アプリアイコンとタスクトレイアイコンを作成して areka.exe に埋め込むスペックを用意しないとダメ」「fal も使えるし」・区分 A） | その他・区分 A | areka の顔になるアイコンを作って `areka.exe` に埋める。**絵は要件の段で作って選ぶ**（10-10 開発者「絵を作るのは要件フェーズで行った方が良い」）＝要件の討議の中で fal（画像生成）の案を出し、開発者が選んだ原画を要件が名指す。実装では、アプリ用と通知領域用（16 ピクセルから出るので形を絞る）の 2 種に仕上げ、複数の寸法（16・20・24・32・40・48・64・256）を 1 つの `.ico` にまとめる。埋め込みは今のマニフェストと同じ道（`build.rs` から bin にだけ届く指示）。議題＝絵柄（steering に決まりが無い）・埋め方（先に組んだ `.res` を置く＝依存なし／`.rc` を組む依存を足す）・作った絵の権利の書き方・32bit の補助の exe にも入れるか | S（5〜8 見込み・要件の段に、開発者と絵を選ぶ往復が入る） | `crates/areka/build.rs`・`crates/areka/Cargo.toml`・新しいアイコンのフォルダ・`tools/` のスクリプト。ソースには触らないので、ほかと並べやすい（重なりは次の棚卸で測る） | なし | − |
| `tasktray-icon`（**10-10 起票**・開発者「タスクトレイの土台仕様を起票」「タスクトレイで出てくるべきメニューは今出てるシステムメニューと同じ」・区分 A） | その他・区分 A | 通知領域（タスクトレイ）に areka のアイコンを 1 つ出す土台。プロセスに 1 つの持ち物として、見えない窓とアイコンを持つ（ゴーストの切替で消えない・エクスプローラーの立ち上げ直しで出し直す・終了で消す）。マウスを重ねると「areka/[起動中のゴースト名]」。右クリックで、本体側のキャラクターの右クリックと同じメニューを出す（`menu/` の組み立てと表示はそのまま使い、キャラクターの窓に縛られた依頼の形 `MenuRequest` を広げる）。標準アイコンの絵は前提の `app-icon` が exe に埋める。議題＝左クリックとダブルクリックで何をするか・ゴーストがメニューを止めているときの扱い・ゴーストが居ない間の右クリック | M（10〜14 見込み・要件で測る） | `menu/trigger.rs`・`menu/mod.rs`・`main.rs`・新しい `tray/`・wintf に通知領域の部品。`menu/` を触る `popup-menu-residue` と直列（残件の相乗り先の候補）。重なりは次の棚卸で測る | `app-icon` | ○ |
| `tasktray-ghost-icon`（**10-10 起票**・同上・区分 A） | その他・区分 A | ゴーストが通知領域のアイコンと文言を決められるようにする。descript の `icon,ファイル名` と、台本の `\![set,tasktrayicon,ファイル名.ico,テキスト(,--duration=待機時間(,--runcount=繰り返し回数))]`。`.ico` の読み込みは OS に任せる。動く絵の指定（`--duration`・`--runcount`）は、開発者の見立て（10-10「今はタスクトレイは基本隠れるので、動く意味は無い」）に沿って動かさない方向で議題にする | S（5〜8 見込み） | `\!` の受け口と descript の読み取り。重なりは着手のときに測る | `tasktray-icon` | − |
| `tasktray-balloon`（**10-10 起票**・同上・区分 A） | その他・区分 A | 台本の `\![set,trayballoon,オプション,オプション,オプション...]` で OS の通知を出し、押した・消えたを `OnTrayBalloonClick`・`OnTrayBalloonTimeout` で SHIORI へ送る。SSP の作者が BTS で説明した挙動 3 件（知らせの遅れ・OS の設定で出ない・時間切れが先に届く＝BTS 0000331・0000407・0000561）を議題にする | S〜M（6〜10 見込み） | kanade の列（`schedule/events.rs`）と `\!` の受け口・wintf の通知領域の部品 | `tasktray-icon` | − |
| `tasktray-minimize`（**10-10 起票**・同上・区分 A） | その他・区分 A | アイコン化（最小化）。メニューの「アイコン化」と台本の `\![set,windowstate,minimize]` でキャラクターとバルーンを隠し、通知領域のアイコンのダブルクリックで戻す。`OnWindowStateMinimize`・`OnWindowStateRestore` を送り、アイコン化の間は `icon.minimize,ファイル名` の絵にする。文言は `hidebutton.caption`。`alwaystrayiconvisiblebutton.caption`（「常にトレイアイコンを表示」）は項目を作るかどうかから議題。名指しの ukadoc の項目 2 つ（`icon.minimize`・`alwaystrayiconvisiblebutton.caption`）がアイコン化なしでは使いどころが無いので、持ち主の無かったアイコン化ごと 1 本にした。議題＝アイコン化の間に喋るか・時計を進めるか | M（10〜15 見込み・膨らめば要件の段で「状態と入り口」と「アイコン・文言・Status」に） | `menu/`・kanade の列（`schedule/events.rs`）・配置の層・wintf の窓の表示 | `tasktray-icon`・`tasktray-ghost-icon` | ○ |
| `user-data-root`（**10-10 起票**・`winget-manifest-submission` の手元の確かめで発見・開発者「消えるなら winget は使えない。アプリをインストーラー形式にするとか、ファイル置き場を指定できるようにするとか、整備が必要」） | 優先・区分 A | winget で入れた areka で、利用者の物（後から入れたゴーストとバルーン・areka の記憶・ゴーストの記憶・`.nar` の作業フォルダ）が入れ先のフォルダ（exe の隣）に在るために起きる 2 つを直す: winget の上げ直しと外し方で消える（実測で 6 つ）／機械の全員向け（`--scope machine`）では書けない（areka の記憶が書けず、ゴーストを後から入れられない）。推す向きは「利用者の物を入れ先の外の書ける根に置き、同梱の物は exe の隣に残して根の並びの下の段にする」（**開発者の裁定はまだ**＝要件の討議で受ける。推さなかった向き＝インストーラー版・zip の並びだけを変える、は brief に）。説明書の「■ 記憶の置き場」の直しを含む。winget が外した後に残す PATH の項目は範囲外 | M〜L（要件で測る） | カタログの鎖（`catalog.rs`・`boot_config.rs`・`boot_resolve.rs` と `install/`＝`baseware-root-list` の後。鎖の中の順は次の棚卸で決める）。配布と公開の列では `winget-release-automation` の前（この spec が着地するまで、説明書に winget の行を載せず、winget-pkgs へ次の版を出さない） | `baseware-root-list` | ○ |
| `write-failure-notice`（**10-10 起票**・`winget-manifest-submission` の手元の確かめ〔機械の全員向け〕で発見） | バグ・区分 B | areka が書けない場所に在るときの伝え方を直す: areka の記憶が書けなくても利用者へ何も伝えず、起動中の印（前回がきれいに終わらなかったことに気付く仕組み）が黙って働かなくなる／記録の文 2 か所が実際と合わない（`last_used_recorded`・`session_mark_clear_degraded`）／書けないせいのインストールの失敗が、書庫の破損と同じ語 `extraction` でゴーストへ渡り、「ファイルが壊れてるのかもね。」と伝わる。メッセージボックスも正典に無いイベントも足さない。`OnInstallFailure` の失敗理由の語は要件の段で ukadoc を引き直す | S〜M（要件で測る） | `boot_resolve.rs`・`install/judge.rs`・`doc/COMPAT_ARCHITECTURE.md` §8（`baseware-root-list`・`install-live-target-hazards` と同じファイル＝同じ時に走らせるなら触る関数を照らす）。`user-data-root` を待たない | なし | ○ |

## 覚え書き（brief なし・引受先が消えたまま忘れないための一覧）

> brief を書くほど固まっていない宿題。**着手の判断は棚卸で行う**（格上げするときは `/kiro-discovery`）。登記時の全文（数え方つき）は history「2026-10-02 棚卸⑳退避」、棚卸㉒の前の全文は history「2026-10-05 棚卸㉒退避」。棚卸㉓で brief にした 5 項目（6 本）の全文は history「2026-10-10 棚卸㉓退避」。

**製品の穴**

- **`present-write-coherence` の未達 40 件**（性能・L）。完了仕様が自ら「引受先なし・新規仕様の起票が必要」と書いた残量＝`visualize_to_write_us` が上限 16,667µs の 12.6〜18.4 倍・32 窓中 0 窓が上限以下。開発者裁定で「未達のまま GO」済み。着手前に測り直す。`tools/perf` の 3 経路・`tick-gate-adoption` の測り方と同じく「性能改善ループが動いてから」の宿題。
- **`recommended.balloon`／`recommended.balloon.path`**（10-03 登記・S）。ゴーストの descript.txt の推奨バルーンで、これ以外へ切り替えると「強い内容の警告」を出す（ukadoc）。メッセージボックスを出さない方針（失敗は既定ゴーストの台詞で伝える）とどう折り合うかを決めてから起票する。`ghost-standard-balloon` が範囲外と明記。網羅台帳は `absent`・担当なし。
- **`install.txt` の `type,package`**（XS〜S）。今は `crates/areka-nar/src/manifest.rs` の `parse_kind` が明示のエラーで断る（黙って壊れはしない）。`developer_options.txt`（配布物を作る道具が読む）は棚卸㉒で「予約」の `createnar` の行へ移した。
- **壊れたゴーストを表示し続ける形**（互換・M・優先度は低い）。SSP は切替先の SHIORI が死んでいても切替を成功扱いにして表示し続ける。areka は「既定ゴーストへ戻して `OnBoot` の Ref6＝`halt`」を採った（09-26 裁定・生きている決まり 7 と逆向き）。あるべき姿としては残る。開発者が閉じてもよい。
- **SSTP の受信**（L）。`balloon-markers` の `sstpmarker`／`sstpmessage` が実際に画面に出るのはこれが入ってから。起票するときは同 spec の縮退の口を埋める。**MCP が 9801 を SSP と早い者勝ちで握る形と、SSTP の待受のポートの扱いがぶつかる**ことを議題に入れる（棚卸㉒）。
- **`sakura-time-directives` から戻した C・D 群**（10-04 登記）。時間の指令のうち、消費する者が現れるまで置くもの（同 brief の「2026-10-04 棚卸㉑で切った後の範囲」）。消費する者が現れたら、その spec の要件の段で引き取る。
- **`sample-ghost-kit` の `fold-samples` の `strip_folder` が取り出し元の先頭の 1 段しか比べない**（XS・`completed/areka-P0-install-companion-reading/tasks.md` の Implementation Notes）。設計で広げないと決めたもの。階層付きの取り出し元を持つ検体ができたら直す。
- **バルーンの文中に画像を置く `\_b[…,inline]`**（棚卸㉓で `range-choice-tag` の再測定から登記・規模は未測定）。読まず、網羅台帳の持ち主も空で、roadmap のどこにも載っていなかった。`range-choice-tag` の brief の例 1（バナーの画像を選択肢にする）はこれが無いと動かない＝同 spec は画像を範囲外に置く。起票するなら文字とバルーンの列。
- **`collisionex` の 5 つ目の型 `region`**（棚卸㉓で `collisionex-regions` の再測定から登記・S）。ukadoc の `collisionex` は矩形・円・楕円・多角形のほかに、画像の色で当たりを決める `region`（「ファイル名,R,G,B,領域反転」）を持つ。`collisionex-regions` の brief と台帳の行は 4 つだけ＝同 spec は `region` を記録して読み飛ばす。使うシェルが出てきたら起票する。

**潜在の性質（実害は観測されていない）**

- **SHIORI へ渡す置き場所のパスの形**（XS）。`load`／`loadu` へ渡すパスが `…\ghost/master` の形（区切りの混在・末尾の区切りなし）。出どころは `crates/areka-parsers/src/package/resolve_shell.rs` の定数 `GHOST_MASTER`（`"ghost/master"`）を `join` する所（helper も末尾に区切りを足さない＝`shiori_proxy.rs` の `init_bytes`）。正典は沈黙。直すなら SHIORI へ渡す 2 つの口（`crates/areka-ghost/src/{shiori_inproc,shiori_wiring}.rs`）だけで揃えるのが影響が狭い。**棚卸㉓**: SHIORI へ送るバイトが変わるので開発者の決めごと。`emily-ghost-verification`（YAYA が動かないときの疑い先）と `shiori4-api`（議題の候補）の brief へ申し送った。

**道具と試験**

- **`tools/perf` の実走していない 3 経路**（S）。`invoke-perf-run.ps1` へ `-GhostRoot`／`-BalloonRoot` を渡す実走・`invoke-followup-checks.ps1` の単独起動・`check-quiet.ps1`。性能改善ループを次に回す前に 3 経路を通す（`tools/perf` は `76e17654` 以降、`check-quiet.ps1` に候補 0 件の守りを 2 行足しただけ＝10-10 `dbb3c758`）。
- **`image-webp` の取り込みを外す作業**（XS・`animated-image-decode` が持ち主をここに置いた）。`image-webp` は上流の GitHub の固定コミット（0.2.5）から取り込んでいる（`tech.md` の登記）。**棚卸のたびに crates.io で `image-webp` の最新版を引き**、0.2.5 以上が出ていたら `tech.md` の取り外し条件どおりに外す＝根の `Cargo.toml` の `[patch.crates-io]` の行・`deny.toml` の `allow-git` の行・`crates/areka-emo-atlas/src/webp_pin_tests.rs` と同じクレートの `lib.rs` の `mod webp_pin_tests;` の宣言を外し、`cargo update -p image-webp` で公開版へ戻して検体のテストを通す。**2026-10-10 棚卸㉓の確認: crates.io の最新は 0.2.4（2025-08-27）のまま＝まだ外せない**。
- **一度だけ落ちた試験と、一度だけ出た出力**（原因未調査・再発したら出力を添えて起票）: `wintf --test graphics` の `STATUS_ACCESS_VIOLATION`（10-04・他のセッションの cargo と並走した `cargo test --workspace -j 4` で 60 秒を超えて止まった後に落ちた。単独では 97 本すべて緑。完了 `wintf-gpu-test-crash` と同じ形・`areka-test-threads-av` は範囲外と明記）。`areka-mcp` の `server::server_gate_help_tests::bad_origin_is_403_before_mcp` が全体テストの負荷の下で 1 回赤（10-04・os error 10053・単独では緑）。`cargo test -p areka-emo-present` の出力に `__rust_alloc_error_handler` のバックトレース行が 1 回だけ混じった（10-05・終了コード 0）。wintf のツールチップのテスト（マウスを動かす検査）で、テストの実行ファイルが 1 回だけ `0xc0000409` で落ちた（10-08・人の手のマウスが混ざった直後・再現せず。`completed/areka-P0-wintf-tooltip/tasks.md` の Implementation Notes。また出たら `--nocapture` の出力を添える）。**棚卸㉒で外したもの**: `fallback_tests` の 2 本・`install::desk::overwrite_tests` の 2 本・`emo2_boot::spine` の 3 本・`sample-ghost-kit` の展開テストの os error 5（`target\nar-samples\work` の退避と同じ族）は `ghost-session-test-load-flake` が引き取った。`tools/test-all.ps1` の x64 の段の終了コード -1 は C: の空きが 35MB だったときの環境の問題として閉じた。

## 直接修正候補（spec なし）

> **2026-10-10 棚卸㉓で直した**（開発者の定めの「軽微な修正は直ちに実施してクローズ」・どれも動きを変えない）: ⑴ 着地した spec を「これからの持ち主」と書いたままのコメント（`crates/areka-emo-text/src/state_decoration.rs` の `\x` の引受先を `talk-fast-forward` へ・`emo2_boot/balloon_visibility_decision.rs`・`balloon_visibility_wait.rs`・emo-text の `actor_attach.rs` の「足す予定の spec」）⑵ 段階実装の名残の「後続タスクで実装」のコメント（kanade の `lib.rs`・`schedule/mod.rs` の 3 か所・`areka-parsers` の `charset`・`kv`・`package`・`sakura`・`shell` の `mod.rs`・`emo2_boot/mod.rs`・`placement/persist.rs`・`emo2_boot/move_cue.rs`・`areka-seriko` の `state.rs` の 2 か所・`areka-actor` の `lib.rs`）＝ 16 ファイル ⑶ `README.md` の MCP の 1 文（10 本のうち 6 本 → 7 本が動く・`check_script` もある）⑷ `structure.md` の道具の一覧に `tools/load-flake.ps1` ⑸ roadmap の古い数と書き違い（1,000 行の番人の例外表 11 → 10 件と上限の近くのファイルの実測・完了サマリの C4 の行に抜けていた `balloon-lifecycle-events`・直列の列の `balloon-lifecycle-events` が触ったファイル〔`frame/wiring.rs` は触っていない〕・`mcp-shiori-query` の依存の行・覚え書きの 3 か所）⑹ `focus.md`・`product.md` の数と状態。
>
> **直さずに各 spec へ渡したもの**（brief の「2026-10-10 棚卸㉓の再測定」「棚卸㉓の申し送り」に記録）: 網羅台帳の持ち主の書き違い（`shiori.toml` の `property.get:1`・`property.set:1` → `property-ipc-transport`／`sakura-script.toml` の `\![embed]` → `sakura-embed-directive`／`assets.toml` の `anchor.*` 43 行と `sakura-script.toml` の `\f[anchor*]` 16 行 → `anchor-style-canon`／`sakura-time-directives` が範囲から外した 6 行／持ち主が空の `OnAnchorSelect`・`OnAnchorHover`・`OnChoiceHover`・`balloon_tooltip`・`\__q` の行）は `roadmap-draft.md` の `owner_count` と一緒に動かす決まりなので各 spec の要件の段で。`crates/areka-emo-compose/src/log_firing_tests.rs` の古い行番号のコメントは `extent-element-offset` へ・`doc/ssp-mcp/survey.md` §7.3 の「ゴーストの名前は完全一致」の古い記述は `property-name-case-fold` へ・`method.rs` の「全 19 種」のコメントは `draw-methods-blend` へ・`emo2_boot/spine.rs` の「4 本」のコメントは `popup-menu-residue` へ・`choice.rs` の警告の文の「M1 未対応」は `choice-marker-styling` へ・`doc/ssp-mcp/areka-tools.md` の手書きの「今 23 組」は次に行を足す spec へ。`dist/README.txt` の「動く絵の上限」の節（動く絵が動くことを書いていない）は、同じファイルを触る C5 の 2 本とぶつからない次の機会に（`animated-image-import` の brief へ）。
>
> **2026-10-05 棚卸㉒・10-04 棚卸㉑・10-03・10-02 棚卸⑳で直したもの**は history「2026-10-10 棚卸㉓退避」の旧「直接修正候補」節と、そこから辿る「2026-10-05 棚卸㉒退避」を参照。

残っている候補: なし。

取り下げた 1 件を再登記しないこと——判定器（`crates/areka/src/placement/transition_judge_verdict.rs`）の窓ごとの書込上限が見送り窓を除いていないのは**意図どおり**（`completed/areka-P0-dpi-transition-atomicity/mechanism-ledger.md` §13.1・2026-09-24 に開発者が再確認）。

## 生きている決まり（棚卸⑭〜⑲の裁定のうち今も効くもの・全文は history）

1. **着手の優先度は 4 区分＝A 開発者が起票を頼んだもの ＞ B バグ ＞ C ukadoc の拾い残し・バグでない持ち越し ＞ D 夢・据え置き・保留**（2026-10-07 開発者・10-05 の 3 段「① バグ ② リリース ③ MCP・バルーン・アニメーション」を改めた）。ウェーブは、依存の木（働きの依存＋同じソースファイルを触る spec＋直列の列の順）の先頭だけを「ぶら下がる未完了の spec の数 → 区分 → 起票の古さ」で並べ、上から互いに触るファイルが重ならない最大 8 本を取る（手順の正本は `/kiro-next` のスキル）。据え置き・保留・取り下げ予定は木に入れない。**測ることが本体の spec（遅れ・性能・負荷の再現）はウェーブに並べない**。バグかどうかは実測で分ける（利用者から見える実害／潜在／テストの穴／正典の追加）。
2. **規模の上限は 1 spec 20 タスク**。超えるものは要件の段で切る。**一度切り出した spec を、上限を少しまたぐ理由でさらに削らない**（09-28）。
3. **並走の物差し＝触るソースファイルの重なり 0**（09-20）。**文書だけの段は置かない**（09-27・要件は前の spec の着地で古くなる）。
4. **エンジンを切り出した spec は網羅台帳に触らない**。台帳の更新は結線の側がまとめて行う（09-20）。
5. **`zorder-chain-residue` の A 群は先回りしない**（main で赤が出たら、その時点で A 群だけを単独で挟む）。
6. **メニューは Win32 標準**・オーナードローと着せ替えメニューは予約のまま。**ゴースト切替はプロセス内**・アプリの寿命は窓の数から切り離す。**HTTP は WinHTTP・MD5 は OS の CNG**。**投げ込みは `WM_DROPFILES`**。**消滅（`\![vanishbymyself]`）は要望が出たら S で切る**。
7. **失敗は既定ゴースト（emo2）の台詞で伝える**（`OnBoot` の Ref6＝`halt`・Ref7＝落ちたゴーストの名前）。メッセージボックスは出さない。**インストールと切り替えは別のイベント**（入れた後の切替を areka は主導しない）。
8. **要件定義・設計のサブエージェントは起動中のモデルを継承し、上位へ上げない**（09-30）。Fable 列は「Fable で起動したセッションを勧める」だけ。
9. **並走できる spec の紹介は「完全並走できるものだけ」・Fable 推奨を先頭に、名前＋一言＋コマンドの短い形**（09-26）。
10. **話の最中（選択待ちを含む）に届いたマウス系の返事は今の話を置き換える＝正典どおり**（2026-10-03・`emo-text-file-split` の実機で「メニューがなでなでの返事に置き換わる」を観測して確かめた。正本は完了 `input-events` の DD-IE-1／DD-IE-2）。止めるのはゴーストの側（`Status` の `talking`／`choosing` を見て黙る）か台本の `\t`。areka の穴は `\t` を読まないことだけ＝`sakura-time-critical`。

## 棚卸㉓の裁定（2026-10-10・main `ee3af616`・開発者指示＝`/kiro-next` の定め「main が進んだら棚卸・深掘り・徹底ブリーフィング。軽微な修正は直ちに実施してクローズ。ロードマップだけにあって brief の無いものを精査して立ち上げ。負荷が高すぎる仕様は分割。厳しめの依存の木の先頭から最大 8 本。Fable と Opus に分けて報告」）

> C4（16 本）の着地の後の棚卸。brief を持つ 86 本のうち、繰り返しの `release-cycle` を除く 85 本すべてを、サブエージェント 7 系統（Opus）で main `ee3af616` に照らして再測定し、結果を各 brief の「2026-10-10 棚卸㉓の再測定」節へ書いた（前提の変化・触るファイル・規模・先に要るもの・区分・モデル・分割の案・穴）。8 系統目が即時修正の候補と brief の無い宿題を洗った。走っている spec は無かった（開いた PR 0 本）。棚卸㉒の裁定の全文・旧ウェーブ表・初回リリースの段取り・旧直接修正候補は history「2026-10-10 棚卸㉓退避」。10-07 の「棚卸㉓」はスキル `kiro-next` を作るための試し走りで、main に入れずに捨てた＝これが本番の棚卸㉓。

1. **並べ方を 4 区分と依存の木に改めた**（開発者・10-07。生きている決まり 1 を書き換えた）。台帳の「段」の列に区分を足した。
2. **即時修正を実施して閉じた**（「直接修正候補」節）: 古いコメント 16 ファイル・`README.md`・steering・roadmap の数と書き違い。動きを変えるもの・SHIORI へ送るバイトが変わるもの・台帳の持ち主の付け替えは直さず、各 spec へ渡した。
3. **3 本を分けた**（新しい brief 4 本）: `seriko-trigger-intervals`（22〜27 と測れた＝口パクの `talk`・`runonce`・`periodic` を残し、台本の側〔`yen-e`・`never`・`\i[ID]`・`animation*.name`〕を `seriko-script-triggers` へ、`always` との組み合わせを `seriko-interval-combinations` へ。残した側は台本のコンパイルの列と compose に触れなくなり、`anchor-tag-canon`・`extent-element-offset` と並べられる）／`draw-methods-canon`（初めて測って 20〜24＝読み手と描けるかの門と α の 5 語を残し、`blend-*` を `draw-methods-blend` へ。Direct2D で描く道を選んだら wintf だけの spec をそのとき起票する）／`winget-manifest-submission`（人の承認の約 2 日が 1 本の PR の中に座る＝手提出までを残し、取り込まれた後の自動化を `winget-release-automation` へ）。
4. **分けなかったもの**（どれも 20 をまたぐ見立て＝要件の段で測って切る。切り方の案は各 brief に書いた）: `text-typesetting`（17〜22）・`talk-fast-forward`（18〜23）・`balloon-markers`（18〜22）・`mcp-reload`（17〜22）・`script-security-level`（16〜22）・`baseware-root-list`（16〜20）・`clippy-199-lints`（12〜16・実数は 2〜3 倍の恐れ）。`text-align-shadow-canon`（18〜22）は一度切り出した spec なので削らない。
5. **覚え書きから 6 本を起票した**（どれも区分 C）: `check-script-arg-checks`・`config-parse-diagnostics`・`font-size-keywords`・`elevated-drop-filter`（Windows の権限の境界をゆるめる話＝やらないと決めて閉じるのも答え）・`mouse-click-wheel-events`・`inputbox-user-input`。**覚え書きに残したもの**: `present-write-coherence` の未達（性能改善ループ待ち）・`recommended.balloon`・壊れたゴーストを表示し続ける形・SSTP の受信（開発者の決めごと待ち）・`type,package`（検体待ち）・時間の指令の C・D 群（消費する者待ち）・SHIORI へ渡すパスの形（送るバイトが変わる）・道具の 3 件。**新しく登記した**: `\_b[…,inline]`・`collisionex` の `region`・一度だけ落ちたテスト 1 件。
6. **列を直した**（「直列の列」の「棚卸㉓で列に足したこと」）: `anchor-tag-canon` が kanade の列と重なる・`\!` の名前を足す spec の共有の席（`consumer_ledger.rs`）・カタログの鎖の順（`baseware-root-list` → `ghost-inner-balloon` → `shell-companion-balloon`）・読み手の鎖・列に居なかった 18 本の置き場。
7. **古くなっていた記述を直した**（各 brief に記録）: `mcp-get-status` は `actor_status.rs` を作っていない・許可の表の迂回の場所は `schedule/events.rs` でなく `schedule/change.rs`・`balloon-lifecycle-events` は `frame/wiring.rs` に触っていない・「`\_q` は `anchor-tag-canon` まで字句にできない」は誤り（既に字句になる＝`sakura-time-directives` の議題は閉じてよい）・`balloon-break-position` の「字句が位置を捨てる」は古い（`mcp-author-tools` が位置を返すようにした）・`dev-helper-x64-clobber` の起こす場所は kanade でなく `shiori-host32-host` の `process_host.rs`・`winget-manifest-submission` の「Release も無い」・`dump-balloon-debug-timeout` の疑った原因（UI スレッドで GPU の読み戻しを待つ）は `mcp-dump-images-residue` で形ごと消えている。
8. **ウェーブ C5 を 8 本で組んだ**（「ウェーブ編成」の表）。先頭を「ぶら下がる数 → 区分 → 古さ」で並べ、上から重ならないものを取った: `anchor-tag-canon`（約 16）・`seriko-trigger-intervals`（約 9）・`property-name-case-fold`（7）・`extent-element-offset`（約 5）・`baseware-root-list`（約 4）・`ssp-bts-salvage`・`mcp-stdio-bridge`・`winget-manifest-submission`。Fable で要件定義を勧める＝`anchor-tag-canon`・`seriko-trigger-intervals`・`extent-element-offset`・`baseware-root-list`。Opus で足りる＝`property-name-case-fold`・`ssp-bts-salvage`・`mcp-stdio-bridge`・`winget-manifest-submission`。
9. **先頭なのに C5 に入らなかったもの**: `mcp-kanade-tools`（約 12・`anchor-tag-canon` と kanade の `msg.rs`・`lib.rs`・`schedule/{events,mod}.rs` が重なる）・`script-security-level`・`farewell-talk-status`（✅ 10-10 着地）・`sakura-time-critical`（同じ理由）・`mcp-reload`（`baseware-root-list` と `ghost_switch.rs`・`shell_balloon_switch.rs`）・`network-update-canon-order`・`ghost-inner-balloon`・`shell-companion-balloon`（`baseware-root-list` と重なる）・`balloon-canon-residue`（`seriko-trigger-intervals` と seriko の `actor.rs`）・`collisionex-regions`・`animated-image-import`（読み手の鎖）・`balloon-font-file`・`balloon-text-area-collapse`・`choice-ranges-one-function`・`choice-marker-styling`（`anchor-tag-canon` と重なる）。重なりは無いが 8 本の上限で外れた＝`emily-ghost-verification`（A）・`dpi-realign-remembered-chain`・`choice-balloon-timeout-stuck`・`emo2-real-run-wrap-timeout`（B）・`reflow-scroll-path-test`・`placement-measure-bake-once`（C）。測ることが本体なので並べなかった＝`areka-test-threads-av`・`test-wait-marker-gaps`・`present-emit-tail-latency`・`install-live-target-hazards`。
10. **再測定で見つけた穴（各 brief に記録・起票は不要）**: `dpi-realign-remembered-chain`＝直す前の動きは 2 回目の起動からすべてのキャラクターが「覚えている」扱いになり、拡大率が変わっても詰め直しが起きない（main で生きている後戻り）。起動と起動の間に拡大率が変わった場合も同じ穴／`areka-test-threads-av`＝スレッドの数を絞らなくても負荷の下で落ちる（5 回に 1 回）。GPU の同時の数を 4 に絞る仕組みが入った後でも落ちた／`draw-methods-canon`＝`BlendKind` は「全 19 種」と書くが台帳は 28 種／`collisionex-regions`＝ukadoc の型は 5 つ／`choice-marker-styling`・`anchor-style-canon`＝台帳が割り当てる descript の行（`cursor.notselect.*`・`anchor.style` ほか）が範囲に入っていない／`system-property-values`＝`windows` の機能を 3 つ足す（`Cargo.toml` の席）／`actor-thread-log-capture`＝素のスレッドを起こす本番の 7 か所は `spawn_actor` を直しても届かない／`test-wait-marker-gaps`＝同じ待ちの写しがほかのクレートのテストにも 5 つ／`install-live-target-hazards`＝上書きの展開は UI スレッドで同期に走り、名前替えの試し直し（1 つ約 2 秒）が積もると終了の待ちの上限 3 秒を超えうる／`sakura-embed-directive`・`property-query-channels`＝台本の全文を先に渡す仕組み（`budoux-reveal-reflow`）と、再生の途中で差し込む文字の折り合いが設計の芯になった。
11. **区分 A の中の並び**: ぶら下がる数が同じ A は、開発者が「高」「早めに」と言ったもの（`ssp-bts-salvage`・`seriko-trigger-intervals`）を先に、残りを起票の古さで並べた。その結果 `emily-ghost-verification`（10-08 起票）が 9 番目になり C5 から外れた。
12. **区分の読みに迷った行**（C に置いた・違えば直す）: `collisionex-regions`（「実機で未対応はすべて起票」の一括の指示）・`balloon-break-position`（「今は作らない。起票を行え」）・`sakura-time-critical`（実機での開発者の気付きが出どころだが正典どおりの動き）・`property-name-case-fold`（「起票して残す」）・`shell-companion-balloon`（指示は分け方だけ）・`impl-watch`（brief に出どころの行が無い）。`shiori4-api` は開発者が brief に「区分 C・急がない」と書いた。`shell-tooltip` は A だが開発者が段を「その他」に置いた＝急がない。
13. **開発者に決めてほしいこと**（どれも C5 の着手を止めない）: ⑴ ✅ **済み**（10-10 に単独で先に通して着地した）: `farewell-talk-status`（バグ・`schedule/mod.rs` へ 3 行）を C5 の着手の前に単独で先に通すか（推し＝通す。C5 の `anchor-tag-canon` とも次の `mcp-kanade-tools` とも同じファイルなので、並べると 2 ウェーブ待つ）⑵ 上限の 8 本を超えて、重なりの無いバグ 3 本（`dpi-realign-remembered-chain`・`choice-balloon-timeout-stuck`・`emo2-real-run-wrap-timeout`）と `emily-ghost-verification` を足すか ⑶ 測ることが本体の 4 本をいつ挟むか（静かな机が要る）⑷ 区分の読み（上の 11・12）⑸ ✅ **済み**（10-10・開発者の「はい」を得て一覧の CSV を 1 回取り、頁は 1 つずつ順に読んだ）: `ssp-bts-salvage` がネットへ出ること（一覧の CSV を落とすなら許しが要る）⑹ 覚え書きの決めごと待ち 3 件（`recommended.balloon`・壊れたゴーストを表示し続ける形・SSTP の受信）⑺ M3 のゴール（棚卸⑳から持ち越し）。**開発者の手作業**＝winget-pkgs の fork と初回の手提出（`winget-manifest-submission`）・`v0.0.3` を打つなら `mcp-stdio-bridge` の PR が開いていないとき。

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

- **決めたこと（起票時・要件の段で開発者が覆してよい）**: ⑴ 本体が 127.0.0.1 で HTTP を待ち受け、Claude Code・Cursor は HTTP で直接つなぐ。**Claude Desktop 用に SSP の `mcp.exe` 相当の中継 exe を `mcp-stdio-bridge` で作る**（2026-09-29 開発者判断。Desktop の `claude_desktop_config.json` は `command` の欄が必須で `url` を書けない〔Desktop 2.9939.4.0 の検査関数で確認〕・コネクタは Anthropic のクラウドからつなぐので 127.0.0.1 へ届かない〔公式の案内〕）。⑵ 既定ポート 9821・`AREKA_MCP_PORT`（`0` で待ち受けない）・既定で有効（SSP と同じ）。9801 は SSP と同時に動かすと衝突し、将来の SSTP の口でもあるので避けた。⑶ SSP の欠陥 2 件（表情表の文字化け・script ログの JSON エスケープ漏れ）は移植しない。⑸ **MCP のプロトコルは自作せず公式 Rust SDK `rmcp` を使う・tokio 依存を入れてよい**（2026-09-29 開発者判断「tokio 依存は入れちゃってもよい。mcp を自作するのは避けた方がよさそう」）。MCP の新しい版への追随は rmcp の版上げで行い、SSP の輸送の癖（survey §2）は参考に格下げ＝ツールの名前・引数・結果の文字列だけを SSP と一致させる。中継 exe は写すだけなので rmcp を使わない。⑷ 分け方は開発者の 3 段（基本 → 入り口 → 個別）に、個別を「触るエンジン」で 6 本に割り、strict を 4 段目に出した。2026-10-03 の開発者裁定で ⑵ の既定ポートは覆った（SSP も 9821 で待ち受けていた）＝既定は 9801 → 9821 の早い者勝ち、どちらも使用中なら隣の 9802 → 9822 … 9810 → 9830（計 20 候補・最初に束ねた 1 つ）・`AREKA_MCP_PORT` の指定はその 1 つだけ（`mcp-server-core` の設計判断 B-13）。
- **採らなかった分け方**: ツールごとに 1 spec（10 本）＝`get_status`・`sakurascript`・`raise_event` が kanade の同じ箇所を触り並走できず、`get_active_ghost_list` などは小さすぎる。基本と入り口を 1 本＝開発者の 3 段に反し、M3 の並走の土台（触るファイルの固定）が基本の検査と同じ spec に埋もれる。
- **既存 spec の更新**: `makoto-dll-host`（brief へ「MCP の reload makoto の口を埋める」を追記済み）。`status-execution-states` は `get_status` の消費者が 1 つ増えるだけ（brief の更新なし）。

#### 3 段目の spec が触るファイル（干渉台帳・`mcp-tool-entrances` の design から転記）

| spec | アプリ本体側（`crates/areka/src/mcp/`） | プロトコル側（`crates/areka-mcp/src/tools/`・要るときだけ） | 自分のエンジン |
|---|---|---|---|
| `mcp-get-property` | `get_property.rs`・`get_property_tests.rs` | `get_property.rs` | sylphya・`areka-ghost` の実行系 |
| `mcp-expression-table` | `get_expression_table.rs`・`get_expression_table_tests.rs` | `get_expression_table.rs` | 表情の表（シェルの定義） |
| `mcp-log-history` | `get_log.rs`・`get_log_tests.rs` | `get_log.rs` | tracing の履歴 |
| `mcp-kanade-tools` | `get_status.rs`・`sakurascript.rs`・`raise_event.rs` と各 `_tests.rs` | 同名の 3 ファイル | kanade |
| `mcp-reload` | `reload.rs`・`reload_tests.rs` | `reload.rs` | 読み直しの経路 |
| `mcp-dump-images` | `dump_surface.rs`・`dump_balloon.rs` と各 `_tests.rs` | 同名の 2 ファイル | emo の読み戻し・base64 の符号化 |
| `mcp-strict-errors` | `sakurascript.rs`・`raise_event.rs` と各 `_tests.rs` | 同名の 2 ファイル | エラーログ（`mcp-kanade-tools`・`mcp-log-history` の後＝直列） |

- **後から答えるツール**（kanade・SHIORI・sylphya に問うもの）は、自分のファイルから `mcp::later` を呼ぶ。毎フレーム覗く系を自分で登録しない＝`mcp/mod.rs`・`ghost_session.rs` を触らない。
- **3 段目が触らない共有ファイル**: `areka-mcp` の `handler.rs`・`registry.rs`・`check.rs`・`tools/mod.rs`・`tools/bridge.rs`・`tools/outcome.rs` と各テスト、`crates/areka` の `mcp/mod.rs`・`mcp/resolve.rs` と各テスト、`main.rs`・`ghost_session.rs`。触る要が出たら、その spec の要件に理由を書く。
- **C3 の照合の要点**: C3-⑦〜⑨（`mcp-get-property`・`mcp-expression-table`・`mcp-log-history`）は、上の表でファイルの重なりが 0＝共有ファイル 0。重なりうるのは「自分のエンジン」の側だけ（例: `mcp-get-property` と `property-query-channels`）。
- プロトコル側のファイルは定義と引数の型を `mcp-tool-entrances` で完成させたので、3 段目は多くの場合アプリ本体側だけを触れば足りる。値の範囲や列挙の検査（`NG:Unknown …`）は、アプリ本体側の処理が結果として返す。

#### Specs (dependency order)

- [x] areka-P0-mcp-server-core -- 127.0.0.1 の HTTP・JSON-RPC・MCP の版と検査（ツール 0 本）。Dependencies: α 完成宣言
- [x] areka-P0-mcp-tool-entrances -- ツール 10 本の定義・引数検査・ghost_name の解決・World への橋・ダミー 9 本。Dependencies: areka-P0-mcp-server-core
- [ ] areka-P0-mcp-stdio-bridge -- Claude Desktop 用の stdio ⇔ HTTP 中継 exe。Dependencies: areka-P0-mcp-server-core
- [x] areka-P0-mcp-get-property -- get_property。Dependencies: areka-P0-mcp-tool-entrances
- [x] areka-P0-mcp-get-status -- get_status（10-05 棚卸㉒で mcp-kanade-tools から切り出し）。Dependencies: areka-P0-mcp-tool-entrances
- [ ] areka-P0-mcp-kanade-tools -- sakurascript・raise_event。Dependencies: areka-P0-mcp-tool-entrances, areka-P0-mcp-get-status
- [x] areka-P0-mcp-expression-table -- get_expression_table。Dependencies: areka-P0-mcp-tool-entrances
- [x] areka-P0-mcp-log-history -- get_log。Dependencies: areka-P0-mcp-tool-entrances
- [ ] areka-P0-mcp-reload -- reload と \![reload,…]。Dependencies: areka-P0-mcp-tool-entrances, areka-P0-shell-balloon-switch
- [x] areka-P0-mcp-dump-images -- dump_surface・dump_balloon。Dependencies: areka-P0-mcp-tool-entrances
- [ ] areka-P0-mcp-strict-errors -- strict の記録。Dependencies: areka-P0-mcp-log-history, areka-P0-mcp-kanade-tools
- [x] areka-P0-mcp-ghost-name-match -- ghost_name の照合を SSP 2.9.07 に合わせる（大小・本体側名・前後の空白・空文字）。共有の resolve.rs を触る＝3 段目と同じウェーブに置かない。Dependencies: areka-P0-mcp-tool-entrances（席は 3 段目の着地の後）

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

- [x] areka-P0-shell-balloon -- `balloon.*`ブレス・描画メソッド `balloon`・シェルの窓への結線・行き先と切り替え・併用しない。Dependencies: α 完成宣言
- [x] areka-P0-shell-balloon-frame-align -- 箱の置き場所の付け替えを `\s` と同じフレームに収める・`Status` の 1 フレームの欠けと箱だけのときの警告（`shell-balloon` の実機の確かめの気付き・バグ）。Dependencies: areka-P0-shell-balloon
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
- **依存の追加**: `image`（本番へ移す・`gif` の機能を足す）は `tech.md` の「意図的依存追加」への登記と開発者の承認が要る＝`animated-image-decode` の要件の段で。→ **要件の段で覆った（2026-10-04 裁定）**: 動く GIF には対応しない＝⑴ の 3 形式は APNG・WebP の 2 形式になり、`image` の機能は `png`・`webp` だけ（`gif` は足さない）。承認と登記は `tech.md` の `image`・`image-webp` の項。

#### Specs (dependency order)

- [x] areka-P0-animated-image-decode -- APNG・動く WebP（動く GIF は 2026-10-04 の裁定で非対応）の全部のコマ・待ち時間・繰り返し回数を読み、アトラスにコマの番号で載せる。Dependencies: α 完成宣言
- [x] areka-P0-surface-element-nesting -- element定義でサーフェスを置く（数字だけ＝番号）・子の当たり判定・子の時計は独立。Dependencies: α 完成宣言
- [x] areka-P0-animated-image-playback -- 動く絵を子サーフェスへ分解して置く自動アニメーション・interval `always`。Dependencies: areka-P0-animated-image-decode, areka-P0-surface-element-nesting
- [ ] areka-P0-animated-image-import -- `import` メソッド（10-05 棚卸㉒で animated-image-playback から切り出し）。Dependencies: areka-P0-animated-image-playback
- [ ] areka-P0-placement-measure-bake-once -- 起動の採寸で動く絵の全コマを読んで捨てない・バルーンを scope ごとに 2 回焼かない（2026-10-05 起票）。Dependencies: areka-P0-animated-image-decode
- [ ] areka-P0-element-clipping-option -- element定義の `--clipping` で矩形だけを描き、付けた element定義では動く絵として読まない（正典 C2・2026-10-05 起票）。Dependencies: areka-P0-animated-image-decode, areka-P0-surface-element-nesting
- [ ] areka-P0-extent-element-offset -- 外形が画像の element定義の X,Y を数えず、はみ出す画像が黙って切られる穴を、ukadoc の外形の定義に照らして直す（2026-10-05 起票）。Dependencies: areka-P0-surface-element-nesting

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
- [x] areka-P0-install-companion-canon -- **10-03 に要件の段で 3 本へ引き継いで閉じた**（下の「同梱バルーンの正典」節）

### 同梱バルーンの正典（2026-10-03 `/kiro-discovery` 再入で `install-companion-canon` を分解）

> 開発者指示（2026-10-03）「現地点で対応していない機能に影響するのであれば、ロードマップを調整し、複数specへの分解で対応して欲しい。本specは「実施せず、別specに引き継ぐ」としてspec作成次第PR終了するのがよいかも」。要件の段で、ukadoc「同時インストール」の「ゴーストやシェルに紐づくバルーンとして設定されるのは最初の1個だけである。」が、areka に無い 2 つの機能（ゴーストの標準バルーン・シェルごとのバルーン）に触れると分かった。

- **分け方**: 境界で 3 本に分けた。
  - インストールの読み方＝`areka-nar` だけ。
  - ゴーストの標準バルーン＝起動時の解決の鎖（`catalog.rs`・`boot_resolve.rs`・`boot_config.rs`）。
  - シェルに紐づくバルーン＝起動の順番と着替えの配線（`emo2_boot`）。
  - 3 本目は `shell-balloon`（C2）と `frame/switch.rs` を共有するので、その後に置く。
- **前身の資料**: `completed/areka-P0-install-companion-canon/` の書きかけの requirements.md（要件 1〜4・6〜8＝1 本目の下書き・要件 5＝2 本目の出発点）。
- **採らなかった分け方**:
  - 前身のまま 1 本で「ゴーストの最初の 1 個」まで入れ、シェルを後回しにする＝正典の半分だけを入れ、descript の `balloon`・`default.balloon.path`（同じ鎖の同じ段の並びの問い）を置き去りにする。
  - 3 本目を 2 本目に混ぜる＝`emo2_boot` を触るので `shell-balloon` を待つことになり、2 本目まで遅れる。
- **覚え書きへ**: `recommended.balloon(.path)`（警告の出し方の方針が先）。

#### Specs (dependency order)

- [x] areka-P0-install-companion-reading -- 書庫を入れるときの同梱の読み方を ukadoc に揃える 4 点と並びの順。Dependencies: none（C2-⑥）
- [x] areka-P0-ghost-standard-balloon -- 起動時のゴーストの標準バルーン（同梱の最初の 1 個・descript の `balloon`／`default.balloon.path`）。Dependencies: areka-P0-install-companion-reading
- [ ] areka-P0-shell-companion-balloon -- シェルの書庫の同梱バルーンをそのシェルに紐づけ、着替えと起動で使う。Dependencies: areka-P0-ghost-standard-balloon, areka-P0-shell-balloon
- [ ] areka-P0-ghost-inner-balloon -- ゴーストの中の `balloon/<名>/` にバルーンを抱える（areka 独自・一覧・記憶・切替・書庫からの入れ方を揃える）。2026-10-05 `ghost-standard-balloon` の要件の討議で開発者が「nar 修正と一緒に別 spec」と決めた。Dependencies: areka-P0-ghost-standard-balloon, areka-P0-shell-companion-balloon

### MCP の areka 独自ツールと影響の段（2026-10-05 `/kiro-discovery` で起票）

> 開発者指示（2026-10-05）「MCP ツールを SSP 互換で作ってもらったが、エージェント目線でこんなツールがあったらいいな、面白いかも、という MCP ツールを、深掘り検証・ウェブ探索して提案してほしい。デスクトップマスコットのすそ野を広げるような提案が欲しい」。

- **調べたこと（起票時）**:
  - よそのエージェントの「体」は、待つ間の 3 つ（今どの段か・呼ばれているか・終わったか）が中心。Codex Pets（OpenAI・2026-05）・OpenPets（MCP の `status`／`react`／`say`）・Vibe Island がその例。「見せる」から「その場で答えを返す」へ進んでいる。
  - SSP の MCP（2.7.06〜）には、押し出し・声・状態の語彙・ゴーストを探す道具が無い。
  - 伺かの AI ゴーストの共通の弱点は、API キーの置き場・同期で止まる・記憶が無い・表情の語彙を教えられない、の 4 つ（最後は `get_expression_table` が答え）。
  - MCP の 2026-07-28 版で sampling は非推奨と報告あり（未確認）。ゴーストが LLM を呼ぶ形は採らない。
- **決めたこと（開発者・要件の段で覆してよい）**:
  - ⑴ **さくらスクリプトでできるツールは優先度を下げる**。「さくらスクリプトを知らないエージェントは無い・スキルでもカバーできる。SSP で喋らせる機能がさくらスクリプトに限定されているのは、さくらスクリプトでできることが強力だから」。採らなかったもの（さくらスクリプトかスキルで足りる）は次のとおり。
    - 平文と感情から喋らせる `say`・掛け合い。
    - エージェントの状態を渡す `set_agent_state`（`raise_event`＋`\s` で足りる・状態の語彙はスキルや文書で配る）。
    - `install_ghost`（`\![execute,install,…]`）・`switch_ghost`（`\![change,…]`）。
  - ⑵ **安全の考え方**: 「エージェントは通常、強大なアクセス権限を最初から持っている。ベースウェア側で多少制約してもあまり関係ない」。区分けするなら、影響の段で分ける。
    - **高**: ベースウェアの外に環境影響を与えるもの（特に留意）。
    - **中**: ベースウェア内の環境への影響。
    - **低**: その場のゴーストの挙動にのみ影響するもの（気にしない）。
    - 「ゴースト切り替えも低。アクター同士の演技の範疇であれば制約なし」。
    - 中・高で何をするかは `script-impact-tiers` の要件の議題。
  - ⑶ 正典の `SecurityLevel`／`SenderType`（ukadoc の spec_shiori3・spec_sstp）が areka に無いこと（SHIORI へは `local` 固定）を開発者が指摘し、`script-security-level` を起票した。影響の段とは別の軸。MCP は SSP と同じく Owned SSTP＝`local` なので、正典の制約は MCP には掛からない。
  - ⑷ 開発者の指摘（同日）「今の MCP ツールは SHIORI への情報要求が無い。raise_event はトークになるだけで MCP 応答にはならない」。SSP の `raise_event` の結果の本文には返り台本が入るが、再生され、データではない。正典で外とゴーストがデータを往復させる口 `X-SSTP-PassThru-*`（要求のヘッダを SHIORI へ通し、SHIORI の応答のヘッダを SSTP の応答へ中継・ghost_terminal の `ShioriEcho` 系が実例）を MCP へ写す `mcp-shiori-query` を起票した。開発者裁定「正典寄りの形」＝台本が返れば NOTIFY と同じく再生し、データはヘッダで返す（areka 独自の「再生しない」は持たない）。ヘッダの名前は `X-MCP-PassThru-*`（開発者「プロトコル的には `X-` で始まるヘッダは任意に命名可能」・正典の `X-SSTP-PassThru-*` は SSTP の経路に限る定義なので経路で名前を分ける）。
- **採らなかった案（理由）**:
  - ゴーストの検索（SSP 側のオンライン MCP がある）。
  - 声（areka に音の層が無い・別テーマ）。
  - 集中タイマー・記憶（ゴーストの領分）。
  - 利用者がいるかの検知（デスクトップマスコットの枠を超える・プライバシー）。
  - 画像 1 枚からシェルを作る案（面白いが大きい・要望が出たら起票）。
  - MCP の購読や Claude Code の channels による押し出し（研究プレビュー・`mcp-user-response` の後で検討）。

#### Specs (dependency order)

- [x] areka-P0-mcp-author-tools -- 独自ツールの登録口と橋＋`check_script`（読むだけ・10-05 に 1 本へ絞った）。Dependencies: areka-P0-mcp-tool-entrances
- [ ] areka-P0-script-security-level -- 出どころの運搬・SHIORI の `SecurityLevel`／`SenderType`・ukadoc のタグの制約。Dependencies: none
- [ ] areka-P0-script-impact-tiers -- 影響の段（高・中・低）の正本の表と出どころごとの扱い。Dependencies: areka-P0-script-security-level, areka-P0-mcp-kanade-tools
- [ ] areka-P0-mcp-user-response -- 選択肢・入力・操作をエージェントへ返す。Dependencies: areka-P0-mcp-author-tools, areka-P0-mcp-kanade-tools, areka-P0-mouse-drag-events
- [ ] areka-P0-mcp-shiori-query -- ゴーストへ情報を問う（SSTP NOTIFY 相当＋`X-MCP-PassThru-*` の往復）。Dependencies: areka-P0-mcp-author-tools, areka-P0-mcp-kanade-tools

### 配布と公開（winget・crates.io・2026-10-02 `/kiro-discovery` で起票）

> 開発者指示（2026-10-02）「winget 対応インストーラーを作成したい。最終的には繰り返し spec を実行するたびに、バージョン番号を 0.0.1 加算しながらリリースを作るような運用。アーカイブの署名が必要なのか？無料で済ませたい」「とにかく無料でできる手段。CI 実行でよいが、PR のたびに実施されるとバージョン管理が変にならないか？リリースタイミングはこちらで決めたい」「可能なら crates.io へのリリースも組み込んで」。

- **起票時の実測（main `76e17654`）**: 版は `Cargo.toml` の `[workspace.package] version = "0.0.1"` 1 か所（21 クレートが継承・SHIORI へ渡す版もここから）。配布は配布スクリプト（今の `tools/package.ps1`）が手元で x64 の zip を日付とコミットの名前で作るだけ。GitHub Releases 0 件・`.github/` なし・SHA256 なし・署名なし。`/kiro-complete` は版にもタグにも触らない。crates.io には `areka`・`dola`・`wintf` の 0.0.1 だけ（名前の確保）。部品のクレートの名前（`areka-*`・`shiori-*`・`log-capture-kit`・`temp-path-kit`）はすべて空いている。
- **調べた事実（2026-10-02・出典は `winget-manifest-submission`・`release-code-signing` の brief）**: winget の zip／portable に署名の義務は無い（MSIX だけ要る）。portable はスタートメニューのショートカットを作らない。PATH へ出すリンク経由の起動で隣の DLL を見失う既知の問題は `ArchiveBinariesDependOnPath: true` で避ける。初回の提出は手で（1 PR 1 版・人の承認・実例で 2 日）、以後は winget-releaser（Release の公開がきっかけ）。無料の署名は SignPath Foundation だけ（CI のビルドが前提）。Microsoft の Artifact Signing は日本の個人には使えない。crates.io は Trusted Publishing で長生きするトークン無しに出せる（新しいクレートの初回は手元から）。
- **決めたこと（開発者確定・2026-10-02）**:
  1. **入れ物は zip のまま**（winget の `zip`＋`portable`）。アイコンが欲しくなったら後からインストーラーを別 spec で足す。
  2. **名乗りは zip 版＝`Areka.Areka.Portable`（「areka ポータブル」）・インストーラー版＝`Areka.Areka`（空けておく）**。入れ物の種類ごとに別の名乗り（portable で入れた人の環境へ次の版でインストーラーを被せる乗り換えを winget は綺麗に扱えない）。発行者は areka プロジェクト自身（`ekicyou.areka` は却下）。
  3. **x64 と arm64 を最初から並べる**（arm64 はこのプロジェクトの動機）。補助 exe は i686 のまま。
  4. **きっかけはタグ**。普通の PR では何も起きない。繰り返し spec `release-cycle` を開発者が打ったときだけ、版を +0.0.1 → PR → squash マージ → `v{版}` のタグ → GitHub Actions がビルド・zip・SHA256・Release → crates.io → winget へ PR。版の正本は `Cargo.toml`、タグはそれを写す。
  5. **無料で**: 未署名で出す（規約上の障害は無い）。CI ができたら SignPath Foundation に申請する（任意）。有料の証明書は開発者がその場で決める。
  6. **「外部 CI は持たない」はテストの門の話**＝ビルドと配布だけを Actions に乗せる。zip の起動確認（`-Check`）は窓を出すので手元に残す。
  7. **crates.io は部品と本体の公開・名前の確保のため** ⇒ **10-03 `crates-io-publish` の要件討議で改め**: 版を出し続けるのは汎用のライブラリ `wintf`・`dola` だけ。`areka` は 0.0.1 の名前の確保のまま・本体と部品は出さない。`cargo install areka` では 32bit の補助 exe が付かないので、利用者向けの入れ方は winget か zip。
  8. **後段の起こし方（10-03・`release-ci-workflow` の要件 8＋セッション間の合意）**: `release.yml` は後段を呼ばず合図も送らない（権限は `contents: write` だけ）。`GITHUB_TOKEN` で公開した Release は `release: published` を起こさないので、winget は `release` の走りの終わり（`workflow_run`）を受けて「タグの push で始まった」かつ「成功」で絞る。crates.io は Trusted Publishing が `workflow_run` を断るので、自分もタグの push で起き、同じコミットの `release` の走りが成功で終わるのを待ってから出す。開発者「手でコマンドを打つ手順は作らない・自動で流れるならどの案でもよい」。
- **採らなかったもの**: main への push で自動リリース（PR のたびに版が上がる）／feature の PR の中で版を上げる（並走する枝が同じ行を取り合う）／`cargo release` で main へ直接 push（PR 経由の決まりに反する）／zip の名前に日付とコミットを残す（版と重ねると長く、道具の自動判定が迷う）／MSIX（署名が必須で無料の道が無い）／zip 版とインストーラー版を同じ名乗りにする。
- **既知の制限として説明書に書くこと**: スタートメニューにアイコンは出ない（コマンド名 `areka` か、インストール先のフォルダから）／Smart App Control を有効にしている環境では未署名の exe が止まる／arm64 の実機の確かめは開発者の手元に機械が無ければ利用者の報告待ち。

#### Specs (dependency order)

- [x] areka-P0-release-package-versioned -- `-Check` の後片付け（合流）・版入りの固定の名前と SHA256・arm64 の zip・リンク経由の起動での場所の解決。Dependencies: none（C1）
- [x] areka-P0-release-ci-workflow -- タグ `v*` で動く GitHub Actions（ビルド → zip → Release）。Dependencies: areka-P0-release-package-versioned
- [ ] areka-P0-release-cycle -- 繰り返し spec。版 +0.0.1 → PR → squash マージ → タグ（初回は `wintf`・`dola` の Trusted Publishing の設定を含む）。Dependencies: areka-P0-release-ci-workflow, areka-P0-crates-io-publish, areka-P0-tools-utf8-child-output
- [x] areka-P0-tools-utf8-child-output -- リリースの道具が `cargo metadata` の UTF-8 の出力をコンソールの文字コードで解く穴を、読む側で直す（バグ・2026-10-05 `/kiro-discovery`・`release-cycle` の初回の実装からの依頼）。Dependencies: none
- [ ] areka-P0-winget-manifest-submission -- `Areka.Areka.Portable` のマニフェスト・初回の手提出・winget-releaser。Dependencies: areka-P0-release-cycle
- [x] areka-P0-crates-io-publish -- `wintf`・`dola` だけを crates.io へ（Trusted Publishing・自分の workflow `crates-io.yml`）。Dependencies: areka-P0-release-package-versioned, areka-P0-mcp-server-core（C2・`release-ci-workflow` と並走）
- [ ] areka-P0-release-code-signing -- SignPath Foundation への申請と CI での署名（任意）。Dependencies: areka-P0-release-ci-workflow

### mouse-drag-events の持ち越し（2026-10-05 `/kiro-discovery` で起票）

> 出どころは `mouse-drag-events` の実機（4.2・クローディア）と完了時の棚卸（`completed/areka-P0-mouse-drag-events/tasks.md` の Implementation Notes・`verification/real-machine.md` 4 章）。開発者の方針（2026-10-05）「実機で areka が未対応だったためにうまくいかなかった件はすべて起票」。

- **分け方**: 4 本にした。`base` と `collisionex` はどちらも `shell/decode.rs` を触るが、描画（合成器）と当たり判定（`hit.rs`）で直す先が分かれるので別にした（同じ列で直列）。位置の保存は `placement/`、テストの AV はテストの土台で、触る所が重ならない。
- **負荷のときの赤**（`install` の desk の上書き・`session_end` の `sync_send`・`emo2_boot::ghost_switch` の `boot_event_tests`）は新しく起票せず、`ghost-session-test-load-flake` の brief の Current State へ観測を足した。

#### Specs (dependency order)

- [x] areka-P0-element-base-method -- element定義の描画メソッド `base` を読んで描き、他の未対応の描画メソッドの行は記録を出す（バグ）。Dependencies: areka-P0-surface-element-nesting
- [ ] areka-P0-collisionex-regions -- `collisionex` の 4 つの形を読んで当たり判定に使い、Reference4 に名前を載せる。Dependencies: areka-P0-element-base-method
- [x] areka-P0-mcp-dump-images-residue -- `dump_balloon` の文字の面の読み戻しを UI スレッドから外し、`dump` の 2 本の後始末 5 件を片付ける。Dependencies: areka-P0-mcp-dump-images, areka-P0-mcp-kanade-tools（後始末の ⑥ だけ）
- [x] areka-P0-char-position-save-on-exit -- 起動の最後に並べ終えた時点で、記憶に位置が無いキャラクター窓の位置を書き、再起動で前回の並びに立つ（バグ・書く時機は 2026-10-05 の要件ディスカッションで「正常な終了」から改めた）。Dependencies: なし
- [ ] areka-P0-dpi-realign-remembered-chain -- 拡大率が変わったあと、位置を覚えているキャラクターどうしも隣り合ったままに見えるよう詰め直す（バグ）。Dependencies: areka-P0-char-position-save-on-exit
- [ ] areka-P0-areka-test-threads-av -- `--test-threads=4` の `STATUS_ACCESS_VIOLATION` の組を特定して直す（バグ）。Dependencies: なし

### mcp-get-status の持ち越し（2026-10-05 `/kiro-discovery` で起票）

> 出どころは `mcp-get-status` の要件ディスカッション議題 1（開発者「1 でよい」）。`mcp-get-status` は要件 2.1 を普段の会話と起動の挨拶の再生中に絞り、お別れの台詞の間に `talking` が出ないことを SSP との差の一覧に書く。`schedule/` は C4 で `balloon-lifecycle-events` が持つので、直すのは別の spec にした。

#### Specs (dependency order)

- [x] areka-P0-farewell-talk-status -- 終了の挨拶・切り替えのお別れの台詞の再生中に実行の状態へ `talking` を載せる（バグ）。Dependencies: areka-P0-mcp-get-status, areka-P0-balloon-lifecycle-events

### バルーンのリンクと OS の連携（2026-10-05 `/kiro-discovery` で起票）

> 開発者の問い（2026-10-05）「さくらスクリプトで、リンク（選択肢）をクリックしたらファイルを開いたりできるか？ また、リンクを右クリックしたらリンクに登録されている文字列（URL とか）をコピペできたりしますか？ バルーンからアプリを開く用途は結構あると思う。SHIORI でもできなくはないけど、あったらうれしい機能だと思う。ukadoc を調べて、既に存在するタグなのかどうかも確認せよ」。

- **ukadoc で確かめたこと**:
  - クリックで開くのは正典にある。`\q[タイトル,script:…]`（`\__q` も同じ）と、`\j[http/file:///mailto:…]`・`\![open,file/browser/explorer/editor/mailer]` の組み合わせで、SHIORI を通さずに書ける（例 `\q[メモ帳,"script:\![open,file,notepad.exe]"]`・`,` を含む台本は `"…"` で括る）。`\_a` には `script:` の形が無く、SHIORI の `OnAnchorSelect` の応答で `\j` を返すのが作法。
  - 右クリックでコピーは正典に無い。ukadoc 最新版の本文を grep して、クリップボードを扱うタグは 0 件。選択肢やアンカーのイベントにボタンの種類の Reference も無い＝areka 独自の拡張。
  - ホバーは正典にある＝SHIORI リソース `balloon_tooltip`（選択肢だけ）・`OnChoiceHover`・`OnAnchorHover`。キャラクター窓には tooltipブレス・SHIORI の `tooltip`・`currentghost.seriko.tooltip.*`。
- **areka の起票時の状態**:
  - バルーンから開けるのは `\![open,readme]` だけ（`script:` は警告 1 行で捨て・`\j` は読まずに捨て・`\![open,…]` の受け取り手は readme だけ）。
  - `\__q` は未実装で引受先なし（`anchor-tag-canon` の brief の「実装済み」は誤り）。
  - バルーンの右クリックは何も起きず、クリップボードとツールチップのコードは 0 件。
- **決めたこと（開発者・要件の段で覆してよい）**:
  - ⑴ 右クリックでコピーする文字列は 3 段＝`script:` の中の開く系の行き先 → ID や引数の `http(s)://`・`file:///`・`mailto:` → 表示されている文字。内部の ID はコピーしない。
  - ⑵ 見せ方は小さなメニュー（選ぶとコピー・行き先 1 つにつき 1 項目・右クリックは選択にもタイムアウトにも触れない）。
  - ⑶ ホバーは正典の 3 つを実装し、ゴーストが何も返さないときだけ行き先を出す。土台は wintf に分ける（「tooltip は色々汎用的にできた方が良い」）。
  - ⑷ `\__q` は `anchor-tag-canon` に足さず別の spec にして、その後に作る。
  - ⑸ 開く系は `script-impact-tiers` より先に入れてよい。開くたびに必ず記録する。
  - ⑹ 外部アプリは OS の既定のアプリで開く。「SSP は OS 既定のアプリが無い時代から続いてるアプリなので、いまは OS の受け口を最大限活用すべき」。
  - ⑺ キャラクター窓のツールチップも起票する（段はその他）。
  - 開発者指示（同日）「この内容を『優先度の高い機能』として取り込み、ウェーブの再検討を」＝棚卸のセッションへ依頼した。
- **既存の spec の直し**: `anchor-tag-canon`（`\__q` の誤りを直し、範囲の仕組みに後の 3 本が乗ると書いた）・`script-impact-tiers`（開く系が先に入り、開く処理の 1 か所へ同意の窓を差し込むと書いた）。

#### Specs (dependency order)

- [x] areka-P0-choice-script-prefix -- `\q[…,script:…]` の実行。Dependencies: なし
- [x] areka-P0-open-external-tags -- `\j` と `\![open,file/browser/explorer/editor/mailer]` を OS の既定のアプリで開く・開く処理の 1 か所と記録・行き先を取り出す関数。Dependencies: なし
- [x] areka-P0-wintf-tooltip -- wintf の止まったことの検出と汎用のツールチップ。Dependencies: なし
- [ ] areka-P0-range-choice-tag -- `\__q` の範囲の選択肢。Dependencies: areka-P0-anchor-tag-canon, areka-P0-choice-script-prefix
- [ ] areka-P0-link-context-copy -- 右クリックのメニューで行き先をコピー（areka 独自）。Dependencies: areka-P0-open-external-tags, areka-P0-anchor-tag-canon, areka-P0-range-choice-tag
- [ ] areka-P0-balloon-link-hover -- `balloon_tooltip`・`OnChoiceHover`・`OnAnchorHover` と既定の行き先の説明。Dependencies: areka-P0-wintf-tooltip, areka-P0-link-context-copy
- [ ] areka-P0-shell-tooltip -- キャラクター窓の tooltipブレス・SHIORI の `tooltip`・`currentghost.seriko.tooltip.*`（その他）。Dependencies: areka-P0-wintf-tooltip

### ゴーストフォルダの複数管理（2026-10-05 `/kiro-discovery` で起票）

> 開発者の依頼（2026-10-05）「ゴーストフォルダの複数管理の仕組みが欲しい。SSP は以下 URL の仕様だが、この仕様に従う必要はない。今どきの、良い感じの管理方式無いかなあ」（SSP ヘルプ `ssphelp:config-folder`）。続けて「特定のディレクトリを、特定 ID のゴースト・シェル・バルーンとしてエイリアス出来たら便利。開発中は git リポジトリを適当な場所に展開して開発用ゴーストをいじることが多いので」。

- **起票時の状態**: 根は 1 つ（`AREKA_ROOT` → exe の隣）。根を 1 つと決め打ちしている本番の箇所は約 14 ファイル・35〜40 か所。記憶の鍵（`LastGhost` ほか）はどれもフォルダ名だけ。完了 spec `baseware-root-layout` が「複数の根」を α 後へ送っていた。
- **決めたこと（開発者・要件の段で覆してよい）**:
  - ⑴ 種類ごと（ゴーストフォルダ・バルーンフォルダ）の登録ではなく、**根を並びで登録する**（例「`c:\areka` → `c:\ssp`」）。上から探して最初に見つかったものを使い、下の同じ名前は隠れる（`PATH` と同じ）。こうすると見分ける鍵はフォルダ名のままで足り、記憶の鍵を作り直さずに済む。
  - ⑵ 新しく入れるものは先頭の根へ入れる。すでにあるものの上書き・ネットワーク更新はその場で行う。インストールのたびに行き先は尋ねない。
  - ⑶ 並びは App の記憶に置く。`AREKA_ROOT` は `;` 区切りで並び全体を上書きする。メニューに「根を追加…」（OS のフォルダ選択）と「根を外す ▸」を置く。並べ替えの画面・設定画面は作らない。
  - ⑷ SSP のフォルダ設定の個別の項目（一時的な除外・フォルダごとのランダム切替や自動更新の除外など）は写さない。
  - ⑸ 開発用のエイリアス＝「名前 → フォルダ」の表を、探す順のいちばん上に置く（ゴースト・バルーン・ゴーストごとのシェル）。エイリアスの先へはインストールも更新も書き込まない（作業ツリーを守る）。登録は手で書く＋環境変数（メニューなし）。
  - ⑹ 同じゴーストを SSP と areka で同時に起動したときの SHIORI の保存ファイルの書き合いは防がず、説明書の注意書きにする。
- **分け方**: 境目は「フォルダ名 → 実際の場所」を引く関数 1 つ。前の spec がこの関数を作って呼び手を寄せ、後の spec はその手前に段を 1 枚足すだけにする。

#### Specs (dependency order)

- [ ] areka-P0-baseware-root-list -- 根の並び・上から探す関数と和の列挙・インストール先・`AREKA_ROOT` の `;` 区切り・メニューの根の 2 項目（その他）。Dependencies: なし
- [ ] areka-P0-dev-folder-alias -- ゴースト・バルーン・シェルのエイリアスを探す順の先頭に置く・エイリアスの先へは書き込まない（その他）。Dependencies: areka-P0-baseware-root-list

### balloon-lifecycle-events の持ち越し（2026-10-05 要件の討議で起票）

> 開発者裁定（2026-10-05・`balloon-lifecycle-events` の要件の討議 議題 1）「中断位置は今は作らない。起票を行え」。`balloon-lifecycle-events` は `OnBalloonBreak` の Reference2 を空で送り、互換対応表と網羅の台帳に縮退として記録する。

#### Specs (dependency order)

- [ ] areka-P0-balloon-break-position -- `OnBalloonBreak` の Reference2 に中断位置を入れる（台本の中の位置を字句から完了の知らせまで通す）（その他）。Dependencies: areka-P0-balloon-lifecycle-events

### element-base-method の持ち越し（2026-10-05 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `element-base-method` の要件の討議（2026-10-05・開発者「残りの描画メソッドは起票」）と完了時の棚卸（`completed/areka-P0-element-base-method/tasks.md` の Implementation Notes）。台帳 `ledger/assets.toml` の element定義の行と `base:1` の行の担当をこの spec へ移した。

#### Specs (dependency order)

- [ ] areka-P0-draw-methods-canon -- 残りの描画メソッドを element定義・pattern定義で描く（pattern定義の `base` を含む・D2D が支える全部へ・wintf も含めて）。Dependencies: areka-P0-element-base-method

### budoux-reveal-reflow の持ち越し（2026-10-06 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `budoux-reveal-reflow` の最終確認の点検と完了時の棚卸（`completed/areka-P0-budoux-reveal-reflow/tasks.md` の Implementation Notes）。調停役 areka棚卸 の指示で 2 本を起票した。どちらもバグではなく檻の穴。

#### Specs (dependency order)

- [ ] areka-P0-choice-ranges-one-function -- 選択肢の範囲を出す本番の手順を関数 1 つにまとめ、検査はそれを呼ぶ（優先）。Dependencies: areka-P0-budoux-reveal-reflow
- [ ] areka-P0-reflow-scroll-path-test -- 区間の全文で配置する枝とあふれのスクロールを組み合わせて踏む経路の検査（優先）。Dependencies: areka-P0-budoux-reveal-reflow

### tools-utf8-child-output の持ち越し（2026-10-06 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `tools-utf8-child-output` の design の Non-Goals（`tools/perf/` は範囲外）と完了時の棚卸（`completed/areka-P0-tools-utf8-child-output/tasks.md` の Implementation Notes）。

#### Specs (dependency order)

- [ ] areka-P0-perf-tools-console-encoding -- `tools/perf/` の子 pwsh が共有の端末の文字コードを書き替える所を、読む側で直す（その他）。Dependencies: areka-P0-tools-utf8-child-output

### ghost-standard-balloon の持ち越し（2026-10-07 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `ghost-standard-balloon` の完了時の棚卸（`completed/areka-P0-ghost-standard-balloon/tasks.md` の Implementation Notes）。棚卸のほかの 2 件は起票しない: `baseware-root-list` の brief への型の注記はその場で足し、網羅台帳の束の小分類「読む経路が無い」の古さは `coverage-roadmap-refresh` の brief へ申し送った。

#### Specs (dependency order)

- [ ] areka-P0-dump-balloon-debug-timeout -- debug 版の `dump_balloon` の時間切れを再現して直す（バグ・再現しなければ記録して閉じる）。Dependencies: areka-P0-mcp-dump-images-residue
- [ ] areka-P0-test-roots-under-target -- テストの一時フォルダを OS の一時フォルダから `target\test-roots` の下へ移す（その他）。Dependencies: areka-P0-ghost-standard-balloon
- [ ] areka-P0-dev-helper-x64-clobber -- 全体テストの後に i686 の helper が x64 版で上書きされる開発時の罠を解く（その他）。Dependencies: none

### animated-image-playback の持ち越し（2026-10-07 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `animated-image-playback` の完了時の棚卸（`completed/areka-P0-animated-image-playback/tasks.md` の Implementation Notes・`research.md`「実機の確かめ」の「範囲外（要起票）」）。6 件のうち 3 件は新しい spec、3 件は既存の brief への申し送り。MCP の `sakurascript`・`raise_event` が無い件は `mcp-kanade-tools` の受け持ち（起票済み）。

#### Existing Spec Updates

- [ ] areka-P0-clippy-199-lints -- 列挙の外の clippy の赤 2 件（`areka-seriko` の `actor.rs` の `large_enum_variant`・`looper.rs` の `collapsible_if`）と、10-06 時点で残る列挙済みの赤を申し送った。Dependencies: none
- [ ] areka-P0-coverage-roadmap-refresh -- `briefing-assets.md` の「SERIKO/MAYUNA 世代別対応表」が検査の外で台帳と食い違ったままの件を申し送った。Dependencies: none
- [ ] areka-P0-draw-methods-canon -- 手書きの `always` のコマの `overlay` 以外の描画メソッドが描かれない件と、`plan.rs` の「非 Overlay method の現在コマ」の `warn!` が `always` の周ごとに出続ける件を申し送った。Dependencies: none

#### Specs (dependency order)

- [ ] areka-P0-present-emit-tail-latency -- シェルの絵の適用の 16 ms 超えの尾を、測っていない区間の計測から原因を特定して直す（バグ）。Dependencies: areka-P0-animated-image-playback
- [ ] areka-P0-balloon-text-area-collapse -- バルーンの文字の描画範囲が潰れたときの毎フレームの供給面の失敗と記録の洪水を止める（バグ）。Dependencies: none
- [ ] areka-P0-seriko-rebuild-hidden-lottery -- 見えていない部品の抽選が回る件を、見える部品だけにするか決まりとして記録する（その他）。Dependencies: areka-P0-seriko-trigger-intervals

### wintf-tooltip の持ち越し（2026-10-08 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `wintf-tooltip` の完了時の棚卸（`completed/areka-P0-wintf-tooltip/tasks.md` の Implementation Notes）。2 件のうち 1 件は新しい spec、1 件は既存の brief の受け持ち。

#### Existing Spec Updates

- [ ] areka-P0-clippy-199-lints -- `cargo clippy --workspace --all-targets` が deny の水準のエラー 5 件で止まる（`areka-emo-text` のテスト 4 ファイル〔`line_pitch_readback`・`choice_fixture`・`emo2_fixture_e2e`・`decoration_readback`〕・`areka-kanade` の `actor_raise_reply_tests.rs`。clippy 1.99 の `absurd_extreme_comparisons` など）。brief の「測り直し」の範囲（`areka-emo-text` は列挙に無かった）として申し送った。Dependencies: none

#### Specs (dependency order)

- [ ] areka-P0-emo2-real-run-wrap-timeout -- `emo2_real_run` が `wrap=BudouxWordWrap` の待ちで赤になる件を、観測で待つ形にして直す（バグ）。Dependencies: none

### ghost-session-test-load-flake の持ち越し（2026-10-10 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `ghost-session-test-load-flake` の完了時の棚卸（`completed/areka-P0-ghost-session-test-load-flake/tasks.md` の Implementation Notes の 5.6 と「範囲外の申し送り」⑴〜⑺・`load-repro.md` の 4.5〜4.9）。新しい spec は 2 本。⑴ 既定のスレッドの数でも負荷の下で `0xc0000005` で落ちた件は `areka-test-threads-av` の brief へ、⑸ `rename_patiently` の天井（掴まれた宛先では名前替え 1 つにつき約 2 秒・UI スレッドが塞がる）は `install-live-target-hazards` の brief へ足した。⑶ `a_failed_commit_…` の一時フォルダが OS の一時フォルダの下にある件は `TempPath::new` の呼び出しで、起票済みの `test-roots-under-target` の範囲（新しく起票しない）。

#### Existing Spec Updates

- [ ] areka-P0-areka-test-threads-av -- 既定のスレッドの数でも負荷の下で `0xc0000005` で落ちた観測（`load-repro.md` の 4.7・4.8・4.9）を申し送った。Dependencies: none
- [ ] areka-P0-install-live-target-hazards -- `areka-nar` の `rename_patiently` の天井と、brief の「`install.rs` は変わっていない」の行が古くなったことを申し送った。Dependencies: none

#### Specs (dependency order)

- [ ] areka-P0-test-wait-marker-gaps -- テストの待ちの残り（目印が UI の側の仕事を数えない待ち・`join_bounded`・`areka-nar` の短い掴みの檻・`GpuPermit::take`・目印の無い `spin_wait_until` 15 か所）を steering `tech.md` の線に揃える（バグ）。Dependencies: areka-P0-ghost-session-test-load-flake
- [ ] areka-P0-actor-thread-log-capture -- `spawn_actor` で起こした背景のスレッドの記録を、本番の記録を変えずにテストの捕捉へ届ける（その他）。Dependencies: none

### 棚卸㉓の分割と起票（2026-10-10 `/kiro-next`）

> 出どころは棚卸㉓の再測定（各 brief の「2026-10-10 棚卸㉓の再測定」「棚卸㉓の分割」「棚卸㉓の測定」節）。分割 3 本から新しい spec 4 本、覚え書きから新しい spec 6 本。理由と残したものは「棚卸㉓の裁定」3〜5。

#### Existing Spec Updates

- [ ] areka-P0-seriko-trigger-intervals -- 口パクの `talk`・`runonce`・`periodic` だけを残した（読み手の `shell/` と seriko だけを触る）。Dependencies: none
- [ ] areka-P0-draw-methods-canon -- 読み手が描画メソッドを運ぶこと・描けるかの門・pattern の `base`・α の 5 語・警告の洪水の停止を残した。Dependencies: areka-P0-element-clipping-option
- [ ] areka-P0-winget-manifest-submission -- マニフェストのひな形・手元での確かめ・初回の手提出までを残した（`README.md`・`.github/workflows/` に触らない）。Dependencies: none
- [ ] areka-P0-script-impact-tiers -- 台本が内部の運び手の名前を直に書ける件を議題に足した。Dependencies: none
- [ ] areka-P0-mcp-reload -- 受け取り手の表の `"bind"` の字面の件を申し送った。Dependencies: none
- [ ] areka-P0-shell-tooltip, areka-P0-balloon-link-hover -- 主モニタより上のモニタでの位置（負の y）の実機の確かめを申し送った。Dependencies: none
- [ ] areka-P0-emily-ghost-verification, areka-P0-shiori4-api -- SHIORI へ渡すフォルダのパスの形（区切りの混在・末尾の区切りなし）を疑い先・議題の候補として申し送った。Dependencies: none

#### Specs (dependency order)

- [ ] areka-P0-seriko-script-triggers -- `yen-e`・`never`・台本の `\i[ID]`・`animation*.name` の読み取り（区分 A）。Dependencies: areka-P0-seriko-trigger-intervals, areka-P0-anchor-tag-canon
- [ ] areka-P0-seriko-interval-combinations -- `bind+always` など `always` との組み合わせ（区分 A）。Dependencies: areka-P0-seriko-trigger-intervals, areka-P0-extent-element-offset
- [ ] areka-P0-draw-methods-blend -- `blend-*` の描画メソッドと `-fast` の形・古い別名（区分 A）。Dependencies: areka-P0-draw-methods-canon
- [ ] areka-P0-winget-release-automation -- `winget.yml` と説明書の行。初回の手提出が winget-pkgs に取り込まれてから（区分 A）。Dependencies: areka-P0-winget-manifest-submission
- [ ] areka-P0-check-script-arg-checks -- 各機能の受け口の引数の判定を純粋な関数へ取り出し、`check_script` が診る（区分 C）。Dependencies: none
- [ ] areka-P0-config-parse-diagnostics -- 設定の読み取りが捨てた行と読まなかったキーを言えるようにする（区分 C・優先度は低い）。Dependencies: none
- [ ] areka-P0-font-size-keywords -- `\f[height,…]` のスタイルシートのキーワードで字の大きさを変える（区分 C）。Dependencies: none
- [ ] areka-P0-elevated-drop-filter -- 管理者として動かした areka へ投げ込みを届けるか、既知の制限として閉じるかを決める（区分 C）。Dependencies: none
- [ ] areka-P0-mouse-click-wheel-events -- 単押しのクリックとホイールのイベントを SHIORI へ送る（区分 C）。Dependencies: none
- [ ] areka-P0-inputbox-user-input -- 入力欄 `\![open,inputbox]` と `OnUserInput`・`OnUserInputCancel`（区分 C）。Dependencies: none

### タスクトレイ（2026-10-10 `/kiro-discovery` で起票）

> 出どころは開発者の指示（2026-10-10「タスクトレイ自体は対応すべき」「タスクトレイの土台仕様を起票したうえで、上流の ukadoc 項目を後続仕様にせよ」「タスクトレイで出てくるべきメニューは今出てるシステムメニューと同じ」）。予約「アプリ層」に名前だけ在った「トレイアイコン」を、土台 1 本と後続 3 本にした。同じ日に、開発者の指摘（「より上位に、アプリアイコンとタスクトレイアイコンを作成して areka.exe に埋め込むスペックを用意しないとダメ」「fal も使えるし」）で、絵を作って exe に埋める `app-icon` を土台の前提として足した。正典は ukadoc MCP で確かめた（各 brief の Current State）。

- **土台と後続に分けた理由**: 通知領域のアイコンは、ゴーストの切替で消えないアプリの持ち物として作る（今は、切替の途中に窓が 0 枚になる）。この持ち物が無いと、アイコンの差し替えも通知もアイコン化からの戻しも作れない。
- **メニュー**: 通知領域用のメニューは作らない。今の右クリックのメニュー（枠 7 つ）をそのまま出す。SSP のヘルプも、アイコンの右クリックで本体側のキャラクターのメニューが開くと書いている。
- **アイコンの動く絵は作らない方向**（開発者 10-10「今はタスクトレイは基本隠れるので、動く意味は無い」）。OS は通知領域のアイコンを動画として受け取らないので、動かすにはアプリがこまを差し替え続けることになる。`\![set,tasktrayicon,…]` の `--duration`・`--runcount` の扱いは `tasktray-ghost-icon` の議題。BTS の要望 3 件（BTS 0000390・0000416・0000418）もそこで閉じる。
- **アイコン化を足した経緯**: 名指しの項目のうち `icon.minimize,ファイル名` と `alwaystrayiconvisiblebutton.caption` は、アイコン化が無いと使いどころが無い。アイコン化そのもの（`\![set,windowstate,minimize]`・`OnWindowStateMinimize`・`OnWindowStateRestore`・`hidebutton.caption`）も持ち主が無かったので `tasktray-minimize` に入れた。
- **アイコンの絵は `app-icon` が作る**（リポジトリに `.ico` が 1 つも無い・exe に埋めているのはマニフェストだけ）。絵は要件の段で作る（開発者 10-10「絵を作るのは要件フェーズで行った方が良い」）＝要件の討議の中で fal の案を出して開発者が選ぶ。実装では、アプリ用と通知領域用の 2 種を `areka.exe` に埋める。予約「インストーラー版」のスタートメニューのアイコンも同じ絵を使う。絵柄は開発者が決める（steering に決まりが無い）。
- **ウェーブへの組み入れは次の棚卸で行う**（触るファイルの重なりをまだ測っていない）。

#### Specs (dependency order)

- [ ] areka-P0-app-icon -- アプリ用と通知領域用のアイコンを作り、`areka.exe` に埋める（区分 A）。Dependencies: none
- [ ] areka-P0-tasktray-icon -- 通知領域にアイコンを出す土台。文言は「areka/[起動中のゴースト名]」・右クリックで今のメニュー（区分 A）。Dependencies: areka-P0-app-icon
- [ ] areka-P0-tasktray-ghost-icon -- descript の `icon,ファイル名` と `\![set,tasktrayicon,…]` でゴーストがアイコンと文言を決める（区分 A）。Dependencies: areka-P0-tasktray-icon
- [ ] areka-P0-tasktray-balloon -- `\![set,trayballoon,…]` で OS の通知を出し、`OnTrayBalloonClick`・`OnTrayBalloonTimeout` を送る（区分 A）。Dependencies: areka-P0-tasktray-icon
- [ ] areka-P0-tasktray-minimize -- アイコン化（最小化）と `icon.minimize`・`hidebutton.caption`・`alwaystrayiconvisiblebutton.caption`（区分 A）。Dependencies: areka-P0-tasktray-icon, areka-P0-tasktray-ghost-icon

### winget-manifest-submission の手元の確かめで見つかった件（2026-10-10 `/kiro-discovery` で起票）

> 出どころは `winget-manifest-submission` のタスク 3.6（同 spec の要件 4.6・4.8・6.3）。根拠は同 spec の `verification/winget-local-check.md` の「見つかった件と起票」の 6 行と、その行が指す節。開発者（同 spec の要件の討議・利用者の物が消えるおそれについて）「消えるなら winget は使えない。アプリをインストーラー形式にするとか、ファイル置き場を指定できるようにするとか、整備が必要」。

- **起票時の実測（0.0.2・開発機 1 台・手元のマニフェストで入れた形）**: winget の上げ直しと外し方（`--purge` を付けない）のどちらでも、利用者の物が 6 つ消えた（後から入れたゴースト 1・後から入れたバルーン 3・ゴーストの記憶 2）。areka の記憶は残った。機械の全員向け（`--scope machine`）では、ゴーストは立つが、areka の記憶は書けず、ゴーストを後から入れられなかった。外した後は、利用者の側でも機械の側でも PATH の項目が 1 件残った。
- **分け方**: 6 件を、新しい spec 2 本と、今ある spec 2 本へ割り付けた。
  - 「上げ直しと外し方で消える」「機械の全員向けでは書けない」「外した後に入れ先のフォルダが残る」は、根が同じ（利用者の物が入れ先のフォルダの中に在る）なので 1 本＝`user-data-root`。
  - 「書けないときに利用者へ伝わらない・記録の文が合わない」「書けないせいのインストールの失敗が、書庫の破損として伝わる」は、置き場を直しても、ほかの理由（読み取り専用・空きが無い・権限）で起きるので、別の 1 本＝`write-failure-notice`。
  - 0.0.2 の MCP の `sakurascript` が仮の受け口である件は、今ある `mcp-kanade-tools` の brief がすでに持つ。
  - 外した後に PATH の項目が残る件は winget の側の動きで、areka からは直せない＝説明書の既知の制限の材料として `winget-release-automation` へ渡す。
- **向き（起票のときの推し・開発者の裁定はまだ）**: 利用者の物を入れ先の外の書ける根に置き、同梱の物は exe の隣に残して根の並びの下の段にする。裁定は `user-data-root` の要件の討議で受ける。
- **採らなかった分け方**: 6 件を 1 本にまとめる＝置き場の直しは `baseware-root-list` の後ろでしか進められず、伝え方のバグがそれを待つことになる。インストーラー版（名乗り `Areka.Areka`）を起票する＝大きく、ポータブル版で入れた人は今のまま残る（「予約」の行のまま）。
- **順の決まり**: `user-data-root` が着地するまで、説明書に winget の行を載せず、winget-pkgs へ次の版を出さない（`winget-release-automation` の前提に加わる。同 spec の brief への申し送りは、2026-10-10 に `winget-manifest-submission` のタスク 5.1 が書いた。タスク 4.3 が 1 行を足した）。

#### Existing Spec Updates

- [ ] areka-P0-mcp-kanade-tools -- 入れた 0.0.2 の `sakurascript` が台本を受け取らない件（中身を入れる仕事は同 spec の brief がすでに持つ。brief は直していない）。Dependencies: none
- [ ] areka-P0-winget-release-automation -- winget で外した後に PATH の項目が残る件を、説明書の既知の制限の材料として渡す（申し送りの文は、2026-10-10 に `winget-manifest-submission` のタスク 5.1 が同 spec の brief へ書いた。タスク 4.3 が 1 行を足した）。Dependencies: areka-P0-winget-manifest-submission

#### Specs (dependency order)

- [ ] areka-P0-user-data-root -- winget で入れた areka で、利用者のゴースト・バルーン・記憶が上げ直しと外し方で消えず、機械の全員向けでも書けるように、置き場を入れ先の外へ出す（区分 A）。Dependencies: areka-P0-baseware-root-list
- [ ] areka-P0-write-failure-notice -- areka が書けないときに利用者へ伝え、記録の文を実際に合わせ、書けないせいのインストールの失敗を書庫の破損と見分ける（区分 B）。Dependencies: none

## 予約（全て任意・brief なし）

アプリ層＝FMO・DirectSSTP・Plugin/HEADLINE・多重ゴースト・消滅（トレイアイコンは 2026-10-10 に `tasktray-icon` ほか 3 本として起票した＝「テーマ別の決めごと」の「タスクトレイ」）・設定画面と設定ファイル（**動く絵の 3 つの上限を設定項目として引き取る**＝今は環境変数 `AREKA_ANIMATED_IMAGE_MAX_FRAMES`・`AREKA_ANIMATED_IMAGE_MAX_PIXELS`・`AREKA_ANIMATED_IMAGE_MAX_TOTAL_PIXELS` で変える・`animated-image-decode` の要件 6.9〜6.12。値は `bake_with_limits` へ渡す `AnimationLimits` をそのまま使える）・`\![update,platform]`（本体の更新）・**インストーラー版（winget の名乗り `Areka.Areka`・データは `%APPDATA%` へ・スタートメニューのアイコン）**・`\![execute,createnar]`／`createupdatedata`（作る側＝開発者機能・配布物を作る道具が読む `developer_options.txt` もここ＝10-05 に覚え書きから移した）。SSTP の受信は「覚え書き」。互換面＝SAORI は実装しない（SHIORI が直接 `LoadLibrary`・台帳 `not-applicable`）・里々/YAYA 網羅。emo テキスト進化＝回転テキスト（`TextEffects` 予約名 `rotation`／`multicolor`）。**`\f[sub]`／`\f[sup]`／`\f[outline]` は DirectWrite の標準機能で表せる手段が見つかるまで語彙のみ**（2026-09-11 裁定・台帳 `vocabulary-only`・追跡先はこの行）。バルーン美観配置。pasta の native x64／`IShiori` in-proc（本番に使うときは `InProc` の SHIORI を外から終わらせる手を決める＝10-05 に覚え書きから移した）・ベクトル描画・owner-draw 右クリックメニュー（`popup-menu-minimal` の構造の上に見た目を被せる）・着せ替えメニュー。段階 A の先頭ウェーブ 6 束（`roadmap-draft.md`・324 件）は M3 のゴールを決めるときの材料。

---

**追記台帳**: 追記(95)〜(103) の要約は history「2026-09-26 番号表記の廃止に伴う退避」節。
