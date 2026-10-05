---
inclusion: manual
updated_at: 2026-10-05
---

# Roadmap — areka（α 後・M3 へ向けた並べ直し）

> **M1 は 2026-09-11、M2（α）は 2026-10-02 に完成宣言済み**。本ファイルは 2026-10-02 の棚卸⑳で、α 後の着手順へ書き直した。**完了した spec の行・棚卸⑭〜⑲の裁定・α のウェーブ表・旧「登記だけの行」の全文は [roadmap-history.md](roadmap-history.md) の「2026-10-02 棚卸⑳退避」節へ逐語で移した**（history が全文正本・非改変）。2026-10-04 の棚卸㉑で、旧ウェーブ表（C1・C2）と棚卸⑳の裁定を同じ history の「2026-10-04 棚卸㉑退避」節へ移した。2026-10-05 の棚卸㉒で、旧ウェーブ表（C3 と予定の C4）・直列の列・覚え書き・直接修正候補・棚卸㉑の裁定を「2026-10-05 棚卸㉒退避」節へ移した。
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
- **1 ファイル 1,000 行**: 機械の番人 `crates/log-capture-kit/tests/file_length_guard_test.rs`（例外表 11 件・暗黙増加不可・**どの spec も例外表に触れない**）。2026-10-05 棚卸㉒の実測で上限の近く（本番のファイル）: `crates/areka/src/emo2_boot/spine.rs` **1,000（ちょうど・1 行も足せない＝`ghost-session-test-load-flake` が最初のタスクで待ちの部品を出す）**・`placement/transition_judge.rs` 996・`main.rs` 953・kanade の `schedule/steady.rs` 947・`schedule/mod.rs` 938・`msg.rs` 909。テストの兄弟ファイルにも 970〜991 行が 10 本ある（`actor_choice_contract_tests.rs` 991・`spine_conformance_lap_tests.rs` 990・`ghost_switch_tests.rs` 988・`runtime_tests.rs` 986・`balloon_visibility_tests.rs` 982・`measure_tests.rs` 981 ほか）。足すときは新しいファイルへ。
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

- 完了 spec 直下エントリ＝**238**（`.kiro/specs/completed/` 直下・2026-10-05 `mcp-dump-images` の完了後の実数え＝フォルダ 237＋`graphics-rendering-stability.md` 1。フォルダのうち `install-companion-canon` は実施せず 3 本へ引き継いで閉じたもの）。⚠ **引き算で導かず毎回実数えする**。
- M1 の持ち越しのうち残るのは `dpi-transition-two-tick-bounce`（開発者が許容）と `zorder-chain-residue` A-2（据え置き）だけ。M-dual は退役。
- 個々の完了行の全文（種別・議題・完了時の所見）は history「2026-10-02 棚卸⑳退避」。

## ウェーブ編成（着手順の正本・2026-10-05 棚卸㉒）

> 各ウェーブは**フルライフサイクル**（要件 → 設計 → タスク → 実装 → `/kiro-complete`＝PR の squash マージ）を完走してから次へ。1 spec ＝ 1 worktree ＝ 1 PR。**同じウェーブに入れるのは、棚卸㉒の実測で触るソースファイルの重なりが 0 の組だけ**。許す重なりは 5 つだけ＝表の末尾への追記（`doc/COMPAT_ARCHITECTURE.md` §8）・生成物（`THIRD-PARTY-NOTICES.md`・`doc/ukadoc-coverage/report/`＝手で直さず作り直す）・steering・網羅台帳 `doc/ukadoc-coverage/ledger/*.toml` の**別々の行**・説明書 `dist/README.txt` の**別々の行**（棚卸㉒で足した。同じ行を 2 本が書き換えるなら同じウェーブに置かない）。文書だけの段は置かない。
> **並び順の決まり（2026-10-05 開発者・10-03 の 3 段を改めた）**: **① バグ ② リリース関係（まず 1 回出す） ③ 優先度の高い機能＝MCP・バルーン・アニメーション**。それ以外（台帳の段「その他」）は**開発者が一覧から優先度を判断する**まで、ウェーブに自分からは入れない。ウェーブの中の番号は優先順（上から着手）。
> C3（11 本）は 10-05 にすべて着地＝「完了サマリ」。C3 の行の全文（編成根拠と約束つき）と、棚卸㉑で予定した C4（7 本）の行は history「2026-10-05 棚卸㉒退避」。棚卸㉑の予定の C4 のうち、`winget-manifest-submission`（Release の実在が要る）・`balloon-font-file`（`budoux-reveal-reflow` と重なる）・`mcp-stdio-bridge`（版上げと同じ `Cargo.lock`）・`anchor-tag-canon`（`budoux-reveal-reflow` と重なる）は C5 へ回した。

| Wave | ユニット（優先順） | 開始コマンド | 編成根拠（触るファイル・棚卸㉒の実測＝各 brief の「2026-10-05 棚卸㉒の再測定」） |
|---|---|---|---|
| **C4**（16 本並走・**バグ 4＋リリース 1＋MCP 4＋バルーン 2＋バルーンのリンク 3＋アニメーション 2**） | ① `ghost-session-test-load-flake`（バグ） ∥ ② `budoux-reveal-reflow`（バグ） ∥ ③ `element-base-method`（バグ） ∥ ④ `char-position-save-on-exit`（バグ） ∥ ⑤ `release-cycle`（**初回リリース `v0.0.2`**） ∥ ⑥ `mcp-get-status`（棚卸㉒で切り出し） ∥ ⑦ `mcp-ghost-name-match` ∥ ⑧ `mcp-dump-images-residue`（項目 1〜5） ∥ ⑨ `mcp-author-tools` ∥ ⑩ `balloon-lifecycle-events` ∥ ⑪ `ghost-standard-balloon` ∥ ⑫ `animated-image-playback`（本体・`import` は棚卸㉒で切り出し） ∥ ⑬ `self-alpha-declaration` ∥ ⑭ `choice-script-prefix`（10-05 起票の「バルーンのリンクと OS の連携」・開発者の依頼で優先度の高い機能として取り込んだ） ∥ ⑮ `open-external-tags`（同） ∥ ⑯ `wintf-tooltip`（同） | `/kiro-start areka-P0-<名>`（`release-cycle` も初回は `/kiro-start`。2 回目からは `/kiro-impl areka-P0-release-cycle`） | ①＝`crates/areka/src/emo2_boot/spine.rs`（1,000 行ちょうど＝待ちの部品 `spin_wait_until`・`run_bounded` を新しいファイルへ出す）・`ghost_switch_test_support.rs`・`ghost_session_switch{,_fallback}_tests.rs`・`ghost_session_restart_tests.rs`・`install/desk_overwrite_tests.rs`・`session_end_sync_send_tests.rs`。②＝emo-text の `actor_present.rs`・`state.rs`・`segment.rs`（注記）・新しいテストと `lib.rs`（純粋な一覧の行）。③＝`crates/areka-parsers/src/shell/decode.rs`（議題しだいで `model.rs`）・compose の `fold.rs`・`method.rs`・`plan.rs`・台帳 `assets.toml` の 2 行・COMPAT §8。④＝`crates/areka/src/placement/persist.rs`・`placement/follow/drag_follow.rs`・`emo2_boot/frame.rs` か `ghost_switch.rs`・`session_end.rs`。⑤＝根の `Cargo.toml` の 2 行・`Cargo.lock`（約 30 行）・`THIRD-PARTY-NOTICES.md`（生成物・版の約 30 行）・`dist/README.txt` の「時点」の行・`README.md` の 1 行（議題 2 で取るなら）・roadmap の記録 1 行。⑥＝`crates/areka/src/mcp/get_status{,_tests}.rs`・kanade の `msg.rs`（変種 1 つ）・`actor.rs`（腕 1 つ）・新規 `actor_status.rs`。⑦＝`crates/areka/src/mcp/{resolve,resolve_tests,mcp_tests}.rs`（10-05 の要件ディスカッションで広げた: 型 `ActiveGhost` に欄を足すので、この型を字面で組む `mcp/` のテストファイル 10 本へ 1 行ずつ・`get_log_tests.rs` の 1 本の期待・`get_log.rs` の注釈・survey §7.4 の注記 1 行。⑥・⑧・⑨ と同じ字面に触る＝後から着地する側が rebase で自分の字面へ 1 行足す）。⑧＝`crates/areka/src/mcp/{dump_surface,dump_balloon}*.rs`・emo-text の `surface.rs`。⑨＝`crates/areka-mcp/src/{tools/mod,tools/bridge,handler,help,registry}.rs`・`crates/areka/src/mcp/mod.rs`・新しいツール 3 本のファイル（`main.rs` も見込み）。⑩＝kanade の `schedule/{events,events_change_tests,user_break,mod}.rs`・`lib.rs`・`crates/areka/src/emo2_boot/{talk_lifecycle,balloon_visibility,balloon_visibility_wait,balloon_visibility_phase,mod}.rs`・`frame/wiring.rs`・台帳 `shiori.toml` の 3 行。⑪＝`crates/areka-ghost/src/catalog.rs`＋兄弟のテスト・`crates/areka/src/{boot_resolve,boot_resolve_tests,boot_config}.rs`・台帳 `assets.toml`（③ と別の行）・COMPAT §8。⑫＝`crates/areka-seriko/src/{table,looper,parts,timeline}.rs`＋兄弟のテスト・compose の `world.rs`・`atlas_bind.rs`・分解の新しいモジュール・台帳 `assets.toml`（別の行）。⑬＝`crates/areka-emo-atlas/src/normalize.rs`（コマにも効かせるなら `lib.rs`・`animated.rs`）・emo-present の `shell_target.rs`・`balloon.rs`・台帳・`dist/README.txt` の既知の制限の行（⑤ と別の行）。⑭＝kanade の `schedule/choice.rs`（`plan_cascade`）・`schedule/steady.rs` の `on_choice`（呼び出しの付け替えだけ）・`choice.rs` の子として置く新しいファイル（例 `schedule/choice/script.rs`）と兄弟のテスト。⑮＝`crates/areka-parsers/src/sakura/decode.rs`（`\j` の腕・議題しだいで `model.rs`）・`crates/areka-sakura/src/compile.rs`・`crates/areka/src/emo2_boot/consumer_ledger.rs`（`open` の受け取り手）・`crates/areka/src/readme.rs`（開く処理を一般化）とその子として置く新しいファイル・台帳 `sakura-script.toml` の行・COMPAT §8。⑯＝`crates/wintf/src/` だけ（止まったことの検出は `ecs/window_proc/mouse_move.rs`・`ecs/pointer/types/mod.rs` の隣・ツールチップは新しいモジュール。必要な Windows の機能 `Win32_UI_Controls` は根の `Cargo.toml` で有効＝`Cargo.toml` に触らない）。**同じウェーブの約束（破るなら止めて報告）**: ⑭ は `schedule/mod.rs`・`lib.rs`（⑩）・`msg.rs`・`actor.rs`（⑥）に触らない（新しいファイルは `choice.rs` の子にして `mod.rs` へ宣言を足さない）／⑮ は `main.rs`（⑨ が触る見込み）・`emo2_boot/mod.rs`（⑩）に触らない（開く処理の新しいファイルは `readme.rs` の子）・dola の `CueCommand` に種類を足さない（`\j` は汎用の `\!` の運び手へ写す）／⑯ は wintf の外と `Cargo.toml` に触らない／ ② が emo-text に新しいファイルを足すのは ② だけ（`lib.rs` の純粋な一覧の数の行を 2 本が同じ数に書くと git が黙って誤った数にまとめる）＝⑧ は emo-text にファイルを足さない／③ は compose の `plan.rs` の外形（`flatten_extent`）を変えない（`extent-element-offset` の持ち物）／④ は `emo2_boot/mod.rs`・`spine.rs`・`frame/wiring.rs` に触らない（終了の共通の入口が `spine.rs` にあると分かったら止めて報告）／⑤ の版上げの PR を出すとき、`Cargo.toml`・`Cargo.lock` を触る開いた PR は 0 本（C4 のほかの 12 本はどれも触らない＝依存を足すなら止めて報告）。版上げの全体テストの後に main が動いたら取り込んで回し直す（Release の段はテストを回さない）／⑥ は `schedule/`・`lib.rs` に触らない／⑦〜⑨ は互いのファイルに触らない＝⑧ は `mcp/mod.rs` に触らない（覗く関数が符号化のスレッドを起こし、仕上がるまで `None` を返す形）・⑨ は `mcp_tests.rs`・`consumer_ledger.rs`・`get_status.rs` に触らない／⑩ は kanade の `msg.rs`・`actor.rs` に触らない（⑥ の持ち物）。`schedule/mod.rs`（938 行）が 1,000 を超えるなら先頭のタスクで分割する／⑫ は parsers の `shell/`・atlas の `manifest.rs`・compose の `plan.rs`・`fold.rs`・`method.rs`・emo-present・`emo2_boot/assets.rs` に触らない（分解は `bind_atlas` の直後）／⑬ は compose・seriko に触らない |
| **C5 の候補**（次の棚卸で組む・C4 の着地で brief が動く） | バグ＝`areka-test-threads-av`（① と同じウェーブに置かない）・`extent-element-offset`（③ の後・`plan.rs`）＋C4 までに見つかったもの ／ リリース＝`winget-manifest-submission`（初回の Release の後・`winget.yml` を main へ入れるのは初回の手提出が取り込まれてから＝下の決まり）・`mcp-stdio-bridge`（新クレート＝`Cargo.lock` の席・版上げと同じウェーブに置かない） ／ MCP＝`mcp-kanade-tools`（`sakurascript`・`raise_event`・⑥ の後）・`mcp-reload`（→ 最後に `mcp-strict-errors`）・`mcp-user-response`（⑨ と `mcp-kanade-tools` の後）・`mcp-shiori-query`（⑨ の後）・`script-security-level` → `script-impact-tiers` ／ バルーン＝**`anchor-tag-canon`（② と ⑮ の後・文字とバルーンの列の C5 の席はこれ＝後続のリンクの 3 本〔`range-choice-tag` → `link-context-copy` → `balloon-link-hover`〕がこれを待つ）**・`balloon-canon-residue`（⑬ と emo-present の `balloon.rs` を分け合うので C4 から外した）・`shell-companion-balloon`（⑪ の後）・`balloon-font-file`（`anchor-tag-canon` と `actor.rs`・`viewbox_draw_render.rs` が重なる＝リンクの列の後。照合して重ならない席があれば前へ）→ 文字とバルーンの列の続き ／ アニメーション＝`seriko-trigger-intervals`（⑫ の後・働きの依存は無いが seriko の表と時計を分け合う）・`animated-image-import`（⑫ の後）・`collisionex-regions`（③ の後）・`placement-measure-bake-once`・`element-clipping-option`（③・⑫・`extent-element-offset`・`animated-image-import` の後） | — | 同じ決まりで バグ → リリース → MCP・バルーン・アニメーション の順に席を埋める。その他の段は開発者が一覧から選んだものだけを、①〜③ と触るファイルが重ならない席に入れる |
| **保留** | `tick-gate-adoption` | — | 「長い試行はしない」と両立する短い A/B の測り方を先に組む。計測を汚すので他と並べない |

### 初回リリースの段取り（`release-cycle` の初回・C4-⑤）

- **開発者の手が要るもの**: ⑴ タグを打つ前に crates.io で `wintf`・`dola` それぞれに Trusted Publishing を足す（持ち主 `ekicyou`・リポジトリ `areka`・workflow `crates-io.yml`・environment は空・手順書 `doc/crates-io-publish.md` 2 節。どちらも crates.io に在るので手元からの初回の公開は要らない）⑵ 手元の全体テスト（`tools/test-all.ps1`）と `tools/package.ps1 -Check`（窓が出る）⑶ 版上げの PR の squash マージとタグ `v0.0.2` の push ⑷ 赤のときに Re-run するかの判断。**勧め**: タグの前に main で `release.yml` を手で 1 回起動して乾いた走りをする（`animated-image-decode` が `image-webp` を git の固定コミットから取り込んだ後の main では、まだ一度も走っていない）。シークレットの登録は要らない。
- **開いている PR の条件を改めた（棚卸㉒）**: brief の「開いている PR 0 本」の本当の要件は「`Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md` を触る開いた PR が 0 本」。ほかの PR が開いていても、タグは版上げの squash コミットに打つので Release の中身は決まる。字の上で衝突しないまま `Cargo.lock` が古くなる（`--locked` の組み立てが落ちる）のを防ぐため、依存を足す spec とは同じウェーブに置かない。
- **開発者に決めてほしいこと**（要件ディスカッションで聞く・C4 の着手は止めない）: ⑴ 初回の PR の分け方（推し: 初回だけ spec の文書と版上げを 1 本の PR に入れる。roadmap への記録の 1 行は次の棚卸に相乗り）⑵ `README.md` の「まだ GitHub Releases での配布はしていない」の行（推し: 初回の本 spec が直す＝`v0.0.2` の Release が出た時点で偽になるのを残さない）⑶ 版上げの道具（`cargo set-version --bump patch --workspace` は `[workspace.dependencies]` の `dola` の `version` も動かす見込み＝cargo-edit 0.13.13 が手元にある。動かし忘れても `cargo check` が落ちるので見落としは起きない）。
- **winget の順の決まり**: winget-releaser は既に winget-pkgs に在るパッケージしか更新できない。`winget-manifest-submission` は初回の手提出（人の承認・実例で約 2 日）が取り込まれるまで `winget.yml` を main へ入れない（入れた後にタグを打つと `winget.yml` が赤になる）。

### 直列の列（同じ列の spec は同時に走らせない・列が違えば並べられる）

棚卸⑳の再測定で、**同じ場所へ行を足す spec の群れ**が見つかった。ファイルを分割しても重なりは消えない（同じ表・同じ関数へ足すため）。ウェーブは、各列の先頭から 1 本ずつ取り、取った組の接触ファイルを照合してから組む。棚卸㉒で kanade の列の並びを優先度の段に合わせ、シェルの element の列から `self-alpha-declaration` を外した。

| 列 | 重なる場所 | 並び（先頭から・✅＝完了） |
|---|---|---|
| **文字とバルーン** | `crates/areka-emo-text/src/`（`state.rs` の `CueCommand::Custom` の腕・`actor*.rs`・`viewbox_draw*.rs`・**`lib.rs` の純粋な一覧**＝新しいファイルを足す spec はどれも触る）・`crates/areka-parsers/src/balloon/{model,parse}.rs`・`crates/areka/src/input_events/` のバルーンと箱（`shell_box.rs` の `judge_box_press`・`user_break.rs` の `on_box_press`） | ✅ `emo-text-file-split` → ✅ `shell-balloon` → ✅ `shell-balloon-frame-align` → `budoux-reveal-reflow`（バグ・C4-②）→ `anchor-tag-canon`（働き・C5）→ `range-choice-tag` → `link-context-copy` → `balloon-link-hover`（10-05 起票の「バルーンのリンクと OS の連携」・開発者の依頼で優先度の高い機能として `text-typesetting` 以降より前へ置いた）→ `balloon-font-file`（`actor_present.rs`・`lib.rs` が ② と、`actor.rs`・`viewbox_draw_render.rs` が `anchor-tag-canon` と重なる。リンクの列のどこかと重ならない席が見つかれば前へ）→ `text-typesetting`（② の「出した字の行を動かさない」約束の上に乗る）→ `talk-fast-forward` → `balloon-markers` → `text-ruby` → `text-reveal-fade` → `balloon-scroll-fade` → `text-align-shadow-canon` → `choice-marker-styling` ∥ 隣に `anchor-style-canon` → `text-reveal-dance`（夢）。**列から外して並べられるのは `choice-marker-styling` だけ**（相手は `budoux-reveal-reflow` に限る）。**列の共通の約束**: 新しいキーはバルーン定義ごとの値（`ResolvedBalloonText` の側）に置く。fade・scroll-fade・dance は「動いている途中の字を見えている字に数えるか」の答えを、当たりの矩形・箱の写し（`actor_box.rs` の `refresh_shown_boxes`）・窓を出す数（`balloon_shown_glyphs`）の 3 か所で揃える。emo-text の `judge_box_click`・`BoxPressVerdict` は 2 値でなく 4 値（棚卸㉑の記述の誤り・棚卸㉒で直した） |
| **シェルの element** | `crates/areka-parsers/src/shell/{model,decode}.rs`・`crates/areka-emo-compose/src/{plan,fold,method,atlas_bind}.rs`・`crates/areka-emo-atlas/src/manifest.rs`・`crates/areka-seriko/src/{table,looper,parts,actor}.rs` | ✅ `shell-balloon` → ✅ `surface-element-nesting` → `element-base-method`（バグ・C4-③）∥ `animated-image-playback`（本体・C4-⑫・`bind_atlas` の直後で分解）→ `extent-element-offset`（`plan.rs`・③ の後）∥ `collisionex-regions`（`shell/decode.rs`・③ の後）∥ `seriko-trigger-intervals`（seriko の表・⑫ の後）→ `animated-image-import`（読み手・`manifest.rs`・`plan.rs`）→ `element-clipping-option` → `balloon-element-order`（wintf の兄弟の重なり順の裁定の持ち主）→ `surfaces-basepos`（据え置き）。parsers の `shell/{model,decode}.rs` を触る 5 本（③・`collisionex-regions`・`seriko-trigger-intervals`・`animated-image-import`・`element-clipping-option`）と、compose の `plan.rs` を触る 5 本（③・`extent-element-offset`・`animated-image-import`・`element-clipping-option`・`balloon-element-order`）はそれぞれ互いに直列。**`self-alpha-declaration`（C4-⑬）は列から外した**（本番の決め打ちは `shell_target.rs`・`balloon.rs` の 2 か所だけで、`manifest.rs`・`atlas_bind.rs` の 3 か所は試験の中だった＝棚卸㉑の記述の誤り） |
| **kanade の進行** | `crates/areka-kanade/src/schedule/{steady,events,mod,change,boot}.rs`・`msg.rs`・`actor.rs`・`lib.rs` の `pub use`。**`steady.rs` 947 行・`schedule/mod.rs` 938 行・`msg.rs` 909 行・`actor.rs` 876 行は上限の近く**＝足して 1,000 を超える spec が先頭のタスクで分割するか、新しいファイルへ足す | ✅ `translate-pipeline` → ✅ `mouse-drag-events` → `balloon-lifecycle-events`（C4-⑩・`schedule/`）∥ `mcp-get-status`（C4-⑥・`msg.rs`・`actor.rs`＝列の外へ出せる組）∥ `choice-script-prefix`（C4-⑭・`schedule/choice.rs` とその子・`steady.rs` の `on_choice`＝⑩ と ⑥ のファイルに触らない組。`script:` の台本をトークとして走らせる口は、後の `mcp-kanade-tools` の `sakurascript` が使い回せる）→ `mcp-kanade-tools`（`sakurascript`・`raise_event`・許可の表の迂回を作る）→ `mcp-reload`（`src/change.rs`・`schedule/boot.rs`）→ `script-security-level`（出どころの運搬・host32-host の列も）→ `mcp-strict-errors` ／ その他の段＝`sakura-time-critical` → `property-query-channels`（棚卸㉒で優先の段の後ろへ下げた） |
| **台本のコンパイル** | `crates/areka-sakura/src/compile.rs`・`crates/areka-parsers/src/sakura/{lexer,decode,model}.rs` | ✅ `choice-timeout-directive` → `open-external-tags`（C4-⑮・`\j` の腕）→ `anchor-tag-canon`（働き・C5）→ `range-choice-tag`（`\__q` の腕）→ `talk-fast-forward` → `sakura-time-directives`。`seriko-trigger-intervals` の `\i[ID]` も `compile.rs`・`sakura/decode.rs` を触る（`anchor-tag-canon` と同じウェーブに置かない）。`sakura-time-critical` は `\t` を `decode_bare` で写せば `compile.rs` に触れない |
| **host32-host** | `crates/shiori-host32-host/src/{shiori3.rs, client.rs}`・`crates/areka-ghost/src/{runtime.rs, shiori_inproc.rs}`・kanade の `shiori/real.rs` | `script-security-level`（`SenderType`・`SecurityLevel` の運搬の持ち主＝棚卸㉒）→ `mcp-shiori-query`（`X-MCP-PassThru-*`）→ `makoto-dll-host`（新規 `makoto.rs`・`runtime.rs`・`Cargo.lock`）→ `makoto-reload-directives` → `property-ipc-transport`（輸送を作るときだけ）。`property-query-channels` は運搬を `script-security-level` に寄せて列から外れた |
| **配布と公開** | `tools/package.ps1`・`.github/workflows/`・`Cargo.toml` 群・`Cargo.lock`・`THIRD-PARTY-NOTICES.md`（版上げで約 30 行）・`dist/README.txt`・`README.md` | ✅ `release-package-versioned` → ✅ `release-ci-workflow` ∥ ✅ `crates-io-publish` → `release-cycle` の初回（`v0.0.2`・C4-⑤）→ `winget-manifest-submission`（C5）∥ `mcp-stdio-bridge`（C5・`Cargo.lock` を版上げと分け合うので初回の後）→ `install-live-target-hazards`（`dist/README.txt` の既知の制限）→ `release-code-signing`（任意・同梱の i686 `pasta.dll` は署名されないまま残る＝議題） |
| **`emo2_boot` の結線** | `crates/areka/src/emo2_boot/{mod,ghost_switch,consumer_ledger,shell_balloon_switch,switch_assets}.rs`・`frame/{attach,switch,wiring}.rs` | ✅ `shell-balloon-frame-align` → `balloon-lifecycle-events`（C4-⑩・`mod.rs`・`frame/wiring.rs`）→ `balloon-font-file`（`frame/attach.rs`・`frame/switch.rs`）→ `balloon-canon-residue` ∥ `shell-companion-balloon`（`frame/switch.rs` を分け合うので同時には走らせない）→ `mcp-reload`（`mod.rs`・`ghost_switch.rs`・`shell_balloon_switch.rs`）。`consumer_ledger.rs` は `open-external-tags`（C4-⑮・`open` の受け取り手）が先に触り、`mcp-reload` は後→ `extra-character-windows`（`switch_assets.rs` も）→ `currentghost-property-tree` → `currentghost-property-others`・`system-property-values` → `property-catalog-lists` |
| **プロパティの動く値** | `crates/areka-sylphya/src/{actor,mirror,key,vocab/dotted}.rs`・`areka-ghost/src/sylphya_wiring.rs` | `property-name-case-fold`（`classify_set` の大小の迂回をふさぐ＝`property-query-channels` が書く道を開く前に着地させる）→ `currentghost-property-tree`（動く値を出す口の持ち主・読む道は `mcp-get-property` が作った）→ `currentghost-property-others` ∥ `system-property-values` → `property-catalog-lists` → `zorder-property` |

- **依存を足す spec と版上げは 1 ウェーブに 1 本**（`Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`tech.md` が重なる）: C4 は `release-cycle` の版上げが席を使う。新しいクレートを足す `mcp-stdio-bridge`・`makoto-dll-host` も同じ席を使う。
- **網羅台帳をまとめて書き換える `coverage-roadmap-refresh` は、台帳の行を直す spec と同じウェーブに置かない**（台帳に触ると書く未完了の brief が 40 本を超えたので、行の重なりで照合する形を同 spec の議題にした）。
- **列に属さず、いつでも単独で取れる**: `coverage-roadmap-refresh`・`popup-menu-residue`・`clippy-199-lints`（段 1＝`dola`・`shiori-abi`・`shiori-host32-host/tests/` は空き席でも取れる）・`property-name-case-fold`（sylphya だけ）・`wintf-tooltip`（wintf だけ・C4-⑯）。
- **MCP の 3 段目の約束**: ツールのファイル（`crates/areka/src/mcp/<ツール>.rs`）と自分のエンジンの側だけを触る。`mcp/mod.rs`・`handler.rs` は触らない（例外は登録口を持つ `mcp-author-tools`）。`handler.rs` の `INSTRUCTIONS` に残る「not implemented yet」の 1 文は、最後に着地する `mcp-strict-errors` が消す＝`mcp-reload` は `mcp-strict-errors` より先に着地させる。今も `NG:not implemented yet` を返すのは 10 本のうち 4 本（`get_status`＝`mcp-get-status`・`sakurascript`・`raise_event`＝`mcp-kanade-tools`・`reload`＝`mcp-reload`）。
- **SHIORI への要求に追加のヘッダを運ぶ道**は `script-security-level`・`property-query-channels`・`mcp-shiori-query` の 3 本が要る。入れ物は先に着手した 1 本が作り、`ShioriBackend` の引数を変えずに既定の実装を持つ新しいメソッドを足す形を推す（実装 19 か所への波及を止める）。

**保存義務（据え置き）**: 既存の終了経路（右クリックメニューの「終了」→ `OnClose` の握手 → `ghost_quit`）の決定論テストを 1 本も落とさない。実機サインオフの「絶対パス起動」（argv 上書き）を残す。

## spec 台帳（brief を持つ 79 本・2026-10-05 `/kiro-discovery`「ゴーストフォルダの複数管理」で 2 本〔`baseware-root-list`・`dev-folder-alias`〔ともにその他〕〕を起票した。その前の 77＝2026-10-05 棚卸㉒の実数え＝main `ec072853` の 75 本〔`/kiro-discovery`「バルーンのリンクと OS の連携」で 7 本〔`choice-script-prefix`・`open-external-tags`・`range-choice-tag`・`link-context-copy`・`wintf-tooltip`・`balloon-link-hover`〔以上 優先〕・`shell-tooltip`〔その他〕〕を起票した〕に、切り出し 2 本〔`mcp-get-status`・`animated-image-import`〕を足した。その前の 68＝`mcp-dump-images` の完了で外し、完了時の棚卸で `mcp-dump-images-residue`〔優先〕を起票した。その前の 68＝`/kiro-discovery` で `budoux-reveal-reflow`〔バグ〕を起票した。その前の 67＝`mouse-drag-events` の完了で外し、完了時の棚卸で `element-base-method`〔バグ〕・`char-position-save-on-exit`〔バグ〕・`areka-test-threads-av`〔バグ〕・`collisionex-regions`〔優先〕を起票した。その前の 64＝2026-10-05 `/kiro-discovery` で「MCP の areka 独自ツールと影響の段」の 5 本〔`mcp-author-tools`・`mcp-user-response`・`mcp-shiori-query`・`script-security-level`・`script-impact-tiers`〕を起票した。その前の 59＝`surface-element-nesting` の完了で外し、完了時の棚卸で `extent-element-offset`〔バグ〕を起票した。その前の 59＝`drag-cancel-borrow-miss` の完了で外した。その前の 60＝`shell-balloon-frame-align` の完了で外した。その前の 61＝`mcp-log-history` の完了で外した。その前の 62＝`mcp-log-history` の完了時の棚卸で `ghost-session-test-load-flake`〔バグ〕を起票した。その前の 61＝`animated-image-decode` の完了で外した。その前の 62＝`animated-image-decode` の実装で `element-clipping-option`〔優先〕と完了時の棚卸で `placement-measure-bake-once`〔優先〕を起票した。その前の 60＝`host32-testdll-marker-race` の完了で外した。その前の 61＝`mcp-expression-table` の完了で外した。その前の 62＝`install-companion-reading` の完了で外した。その前の 63＝`mcp-get-property` の完了で外した。その前の 10-04 の数え＝`mcp-get-property` の要件ディスカッションで `property-name-case-fold`・`mcp-ghost-name-match` を起票して 64 本・2026-10-04 棚卸㉑の実数え＝53 本に、切り出し 5 本〔`anchor-style-canon`・`sakura-embed-directive`・`makoto-reload-directives`・`currentghost-property-others`・`system-property-values`〕と覚え書きからの起票 4 本〔`self-alpha-declaration`・`seriko-trigger-intervals`・`extra-character-windows`・`update-check-options`〕を足した。それより前の数えの経緯は history「2026-10-04 棚卸㉑退避」）

> **spec は名前で呼ぶ**（2026-09-26 開発者指示）: 報告・brief・コミット・PR で spec を指すときは spec 名（`areka-P0-` は省略してよい）を書く。「#数字」は `PR#185` の形の PR 番号にだけ使う。古い文書に台帳番号が出てきたら、その時点の表（history）で名前へ読み替える。
> **2026-10-05 の改め**: 着手の並びは ① バグ ② リリース ③ MCP・バルーン・アニメーション（生きている決まり 1）。この表の「段」の列は下の 10-03 の分け方のまま使い、「優先」の中をリリース → MCP・バルーン・アニメーションの順に取る。「その他」・据え置き・保留は開発者が一覧から選ぶ。
> **段＝優先度の 3 段**（2026-10-03 開発者「1 バグ・2 リリース関係・バルーン関係・アニメーション画像関係・3 その他」）: **バグ**＝1 段目／**優先**＝2 段目（配布と公開・文字とバルーンの列・シェルの element の列・動く画像・バルーンのイベントと残件・**SSP MCP の移植 10 本**＝10-03 に開発者が追加・`mouse-drag-events`＝10-04 に開発者が個別に上げた）／**その他**＝3 段目（ゴーストのインストール・kanade・プロパティ・道具）／**据え置き**＝当面着手しない（理由は行に）／**保留**。どのウェーブに居るかは「並び」の列。完了した spec はこの表に置かない（完了サマリと history）。
> **段の数え方**: `awk '/^\| spec（`areka-P0-` 省略）/{f=1;next} f&&/^\|/{print} f&&!/^\|/{f=0}' .kiro/steering/roadmap.md | awk -F'|' 'NR>1{gsub(/ /,"",$3);print $3}' | sort | uniq -c`（2026-10-05 「ゴーストフォルダの複数管理」の起票後の数え＝バグ 6・優先 43・その他 22・据え置き 7・保留 1 ＝ 79〔その他 2 本を足した。その前の 77＝棚卸㉒の数え＝「バルーンのリンクと OS の連携」の 7 本〔優先 6・その他 1〕と切り出し 2 本〔優先〕を足した。その前の 68＝`mcp-dump-images` の完了で外し、完了時の棚卸で `mcp-dump-images-residue`〔優先〕を起票した。その前の 68＝`budoux-reveal-reflow`〔バグ〕を起票した。その前の 67＝`mouse-drag-events` の完了で外し、完了時の棚卸で 4 本〔バグ 3・優先 1〕を起票した。その前の 64＝「MCP の areka 独自ツールと影響の段」の 5 本〔優先〕を起票した。その前の 59＝`surface-element-nesting` の完了で外し、完了時の棚卸で `extent-element-offset`〔バグ〕を起票した。その前の 59＝`drag-cancel-borrow-miss` の完了で外した。その前の 60＝`shell-balloon-frame-align` の完了で外した。その前の 61＝`mcp-log-history` の完了で外し、完了時の棚卸で `ghost-session-test-load-flake`〔バグ〕を起票した。その前の 61＝`animated-image-decode` の完了で外し、`element-clipping-option`・`placement-measure-bake-once`〔優先〕を起票した。その前の 60＝`host32-testdll-marker-race` の完了で外した。その前の 61＝`mcp-expression-table` の完了で外した。その前の 62＝`install-companion-reading` の完了で外した。その前の 63＝`mcp-get-property` の完了で外した。その前の 64＝棚卸㉑の 62 に `property-name-case-fold`・`mcp-ghost-name-match` を足した〕。それより前の数えの経緯は history「2026-10-04 棚卸㉑退避」）
> **規模**は棚卸㉑の再測定の見立て（タスク数）。**上限は 1 spec 20 タスク**。「要件で切る」と書いた行は、着手のときに要件の段で切り出す（先に起票しない＝spec 工場の禁止）。**Fable**列＝要件定義を Fable で起動したセッションで進めることを勧める（○）か、Opus で足りる（−）か。各 brief の末尾「2026-10-04 棚卸㉑の再測定」（と切り出し・起票の節）が、崩れた前提・触るファイル・議題の正本。

| spec（`areka-P0-` 省略） | 段 | 何をするか | 規模 | 並び | 前提（先に着地） | Fable |
|---|---|---|---|---|---|---|
| `ghost-session-test-load-flake`（**10-04 起票**・`mcp-log-history` の完了時に発見） | バグ | 機械が重いと `ghost_session_*`・`shell_balloon_switch_session_*` のテストが毎回違う組で 5〜10 件赤（単独・静かな時は緑・`host32-testdll-marker-race` の完了時の全体テストでも `fallback_tests` 2 本が赤＝覚え書き）。まず負荷付きで再現し、壁時計の締切の待ちを観測の待ちへ置き換える（sleep で直さない）。再現しなければ記録して据え置きへ。**棚卸㉒で `emo2_boot::spine` の 3 本・`install::desk::overwrite_tests`・`zorder-chain-residue` の A-2 を引き取った**（`spine.rs` は 1,000 行ちょうど） | S〜M（5〜8） | **C4-①** | なし | − |
| `extent-element-offset`（**10-05 起票**・`surface-element-nesting` の完了時の棚卸） | バグ | 外形（`plan.rs` の `flatten_extent`）が画像の element定義の X,Y を数えず、ベースの画像の外へはみ出す画像の element が記録なしに切られる（emo-compose を作った時から・入れ子の子の中の element も漏れる）。ukadoc の外形の定義を確かめ、全命令の和集合に揃えるか、ベースで切ると明文化して記録を出すかを裁定する | S（3〜6） | C5 の候補（バグ）・シェルの element の列（`element-base-method` の後＝同じ `plan.rs`） | `element-base-method` | ○ |
| `budoux-reveal-reflow`（**10-05 起票**・開発者「emo2 の初回起動トークでバルーンに不自然な改行」の調査） | バグ | `budoux_newline,1` のとき、文節の区切りを「その時点までに届いた字」だけで毎フレーム計算し直す（`actor_present.rs` の `segment_plan(actor_state.items())`）ので、台詞が `\_w` で分かれて少しずつ届くと、表示済みの字が次の行へ飛ぶ（emo2 の「イイジャン！‥‥」→「ええと、」が届いて「‥‥」が 2 行目へ）。SSP では pasta が改行を台本に先に書くので飛ばない＝areka だけの症状（実機と SSP で撮影済み）。完了済み `budoux-newline` の要件 7.2 を実際の届き方で満たし直す。案 1（出した字の行を固定・文字の層の中で閉じる）を推す。pasta の余分な空行（pasta 側で `paragraph-break-tag-only-talk` として起票済み）と行頭の「‥‥」（禁則＝`text-typesetting`・SSP でも同じ）は範囲外＝brief に記録 | S（4〜7） | **C4-②**・文字とバルーンの列の先頭（`balloon-font-file`・`anchor-tag-canon` とは `actor_present.rs`・`state.rs`・`lib.rs` が重なる） | なし | ○ |
| `element-base-method`（**10-05 起票**・`mouse-drag-events` の実機で発見） | バグ | element定義の描画メソッド `base` を描けない（`areka-parsers` の `decode_elements` が `overlay` 以外の行を捨て、`areka-emo-compose` の `is_implemented` も `overlay` だけ）。`element0,base,<別の絵>`＋`overlay` の面（クローディアの surface6・11・26）が上乗せの部品の大きさだけで描かれ、キャラクターが消える（記録なし・起動直後の `\s[26]` でも起きる）。読み手に描画メソッドの欄を足して `base` を描き、他の未対応の描画メソッドは記録を出す | S〜M（3〜10・議題 1 しだい＝ukadoc は element1 以降の `base` を overlay に読み替える） | **C4-③**・シェルの element の列（`extent-element-offset`・`collisionex-regions`・`element-clipping-option` はこの後） | なし | ○ |
| `char-position-save-on-exit`（**10-05 起票**・`mouse-drag-events` の実機 R7・開発者「不自然」） | バグ | キャラクター窓の位置を記憶に書くのがドラッグの終了だけ（`drag_follow.rs` の `on_char_drag_end`）。一度もドラッグしていない相方は再起動で前回の並びに戻らず、`chain_finalize` で本体の左隣へ並べ直される。正常な終了で全キャラクターの位置を書く（`position-persist` の発火規律 Req1.9 の改訂を要件の段で裁定） | S（4〜8） | **C4-④**（`placement/` を触る spec と同時に走らせない・ゴーストの切替でも同じ問題＝書く時機が議題） | なし | ○ |
| `areka-test-threads-av`（**10-05 起票**・`mouse-drag-events` の 2.2 の検証で発見） | バグ | `cargo test -p areka --bin areka -- --test-threads=4` が `STATUS_ACCESS_VIOLATION` で落ちる（drag のテストを外しても・既定のスレッド数では落ちない・再現する）。二分探索で落ちる組を特定し、スレッドに縛られる資源の扱いを直す。再現しなければ記録して据え置きへ | S（3〜6） | C5 の候補（バグ・`ghost-session-test-load-flake` と同時に走らせない） | なし | − |
| `collisionex-regions`（**10-05 起票**・`mouse-drag-events` の実機で発見） | 優先 | `collisionex`（矩形・円・楕円・多角形の当たり判定）を読まない（`decode_collisions` は数字だけの `collisionN` だけ・台帳 `absent`）。`collisionex` だけで書くクローディアでは全マウスイベントの Reference4 がいつも空。読み手に形を足し、`hit.rs` で形ごとに内外を判定する | S〜M（7〜10） | C5 の候補・シェルの element の列（`element-base-method` の後＝`shell/decode.rs`） | `element-base-method` | − |
| `install-live-target-hazards` | その他 | 表示中のシェル・使用中のバルーンへの上書きと、起動中のゴーストへ入れる途中の Windows の終了を**実測してから**扱いを決める | S〜M（8〜14） | C5 の候補（⑴ は `balloon-font-file` の後に測る・`dist/README.txt` を配布の列と分け合う） | なし | ○ |
| `ghost-standard-balloon`（**10-03 起票**・同 2/3） | 優先 | 起動時のゴーストの標準バルーン＝同梱の最初の 1 個（無印が無ければ `balloon0`）と descript の `balloon`・`default.balloon.path`（今はどれも効かない） | S〜M（6〜10） | **C4-⑪** | `install-companion-reading`（✅ 10-04 完了） | ○ |
| `shell-companion-balloon`（**10-03 起票**・同 3/3） | 優先 | シェルの書庫の同梱バルーンをそのシェルに紐づけ、着替えと起動で使う（今はシェルごとのバルーンが無い） | M（10〜14・議題 4 で「一度に替える」なら 12〜16） | C5 の候補・`emo2_boot` の列 | `shell-balloon`・`ghost-standard-balloon` | ○ |
| `release-cycle`（**10-02 起票**・配布と公開・**繰り返し spec**） | 優先 | 開発者が実装を打ったときだけ版を +0.0.1 → PR → squash マージ → タグ。初回が `v0.0.2`（`wintf`・`dola` の Trusted Publishing の設定もこの中） | XS〜S（初回 8〜11・2 回目から 6） | **C4-⑤**（初回リリース・`Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md` を触る開いた PR が 0 本なら他と並べてよい＝棚卸㉒） | `release-ci-workflow`・`crates-io-publish` | − |
| `winget-manifest-submission`（**10-02 起票**・配布と公開） | 優先 | winget の名乗り `Areka.Areka.Portable`（zip＋portable・x64 と arm64）・初回の手提出・以後は winget-releaser。`Areka.Areka` はインストーラー版のために空けておく | S（7〜10） | C5 の候補（Release の後・`winget.yml` は初回の手提出が取り込まれてから main へ） | `release-cycle` の初回 | − |
| `balloon-font-file` | 優先 | `font.name`・`\f[name]` のフォントファイル | M（8〜11） | C5 の候補（`budoux-reveal-reflow` の後＝`actor_present.rs`・`lib.rs`） | `shell-balloon-frame-align`（✅ 10-05 完了） | − |
| `anchor-tag-canon` | 優先 | `\_a` のリンクの**働き**（今は押しても何も起きない＝優先度 高）・`\_a[OnID,…]` の形・箱の押下の結論。見た目は既定の 1 種類（装飾は 10-04 に `anchor-style-canon` へ切った） | M（12〜15） | **C5 の先頭**（`budoux-reveal-reflow`・`open-external-tags` の後＝`state.rs`・`actor_present.rs`・`decode.rs` が重なる。リンクの 3 本がこれを待つ）・文字とバルーンの列・コンパイルの列 | なし（`shell-balloon`・`choice-timeout-directive` は着地済み） | ○ |
| `text-typesetting` | 優先 | 禁則・ぶら下げ・縦中横・字の向き（areka 独自） | M〜L（14〜18） | 文字とバルーンの列 | `shell-balloon` | ○ |
| `talk-fast-forward` | 優先 | クリックでの早送り・`\x`／`\x[noclear]`・`clickwaitmarker.*` | M〜L（15〜19） | 文字とバルーンの列・コンパイルの列 | `shell-balloon` | ○ |
| `balloon-markers` | 優先 | `arrow*` と手動スクロール・`onlinemarker`・`number`・SSTP の印（表示だけ）・装飾の系列 | L（16〜20） | 文字とバルーンの列 | `shell-balloon` | − |
| `text-ruby` | 優先 | ルビ `\![text,ruby,親文字,ルビ]`・`line_height`・`letter_spacing` | M〜L（12〜16） | 文字とバルーンの列 | `text-typesetting` | ○ |
| `text-reveal-fade` | 優先 | 1 字ずつの表示で字が透明から不透明へ変わる（既定は無効） | M（9〜12） | 文字とバルーンの列 | `shell-balloon` | − |
| `balloon-scroll-fade` | 優先 | 自動スクロールで押し出される行のフェード（既定は無効） | S〜M（6〜9） | 文字とバルーンの列 | `balloon-markers` | − |
| `text-align-shadow-canon` | 優先 | `\f[align]`／`\f[valign]`・影。**20 を超えそうなら「影」と「寄せ」に切る** | M〜L（17〜21・切らない＝超えたら要件の段で影 → 寄せ） | 文字とバルーンの列 | `text-typesetting` | ○ |
| `choice-marker-styling` | 優先 | `\f[cursor*]` 10 項目と本物の下線 | S（5〜8） | 文字とバルーンの列 | `text-align-shadow-canon`（同じファイル） | − |
| `animated-image-playback` | 優先 | 動く絵を子サーフェスへ分解して自動で回す・interval `always`・バルーンの面（`import` は 10-05 に `animated-image-import` へ切った） | M（14〜17） | **C4-⑫**・シェルの element の列（分解は `bind_atlas` の直後＝parsers・`manifest.rs`・`plan.rs` に触らない） | `animated-image-decode`・`surface-element-nesting` | ○ |
| `animated-image-import`（**10-05 起票**・棚卸㉒で `animated-image-playback` から切り出し） | 優先 | pattern定義の描画メソッド `import` の画像を名前を保って読み、焼いて置く（今は読み手がファイル名を 0 に化かし記録も出ない） | S〜M（6〜9） | C5 の候補・シェルの element の列（`animated-image-playback` の後） | `animated-image-playback` | ○ |
| `element-clipping-option`（**10-05 起票**・`animated-image-decode` のタスク 6.3＝要件 9.3） | 優先 | element定義の `--clipping,左,上,右,下` で矩形だけを描く・付けた element定義では動く絵として読まない（正典 C2）・オプションの並びを読み手で転記（`--alpha`・`--source`・`--scaling` の描画は範囲外） | M（10〜14） | シェルの element の列（`element-base-method`・`animated-image-playback`・`extent-element-offset`・`animated-image-import` の後） | `animated-image-decode`・`surface-element-nesting`・`element-base-method` | ○ |
| `placement-measure-bake-once`（**10-05 起票**・`animated-image-decode` の完了時の棚卸＝research.md 9.14 節） | 優先 | 起動の採寸が絵を全部焼いて寸法だけ使い、動く絵の全コマを読んでは捨てる（上限いっぱいの絵 1 つで起動の焼く時間が release 0.19 → 3.2 秒）。バルーンも scope ごとに採寸と資産組み立てで 2 回ずつ焼く。焼くのを 1 回にするか、採寸を全コマ無しで済ませる | S〜M（8〜12） | `animated-image-playback` の前が望ましい（再生が着地すると動く絵の検体が増える）・`emo2_boot` の結線の列と `placement` | `animated-image-decode` | − |
| `balloon-element-order` | 優先 | `balloon` の element定義を並び順どおりの重ね順で描く。wintf の兄弟の重なり順（描画と当たり判定が逆）の裁定の持ち主 | M（10〜14） | シェルの element の列 | `shell-balloon`・`animated-image-playback` | ○ |
| `property-query-channels` | その他 | `\![get/set,property]`・`%property[…]`・`SenderType`（`\![embed]` は 10-04 に `sakura-embed-directive` へ切った）。許可の表の迂回を最初に決める | M（13〜17・`SenderType` の運搬を `script-security-level` へ寄せたので 11〜14 の見込み） | kanade の列（その他の段＝`mcp-kanade-tools` の後）。host32-host の列からは外れた | `translate-pipeline`・`property-name-case-fold`・`mcp-kanade-tools`（迂回の口） | ○ |
| `balloon-lifecycle-events` | 優先 | `OnBalloonClose`／`OnBalloonTimeout`／`OnBalloonBreak`（項目 8・10 を先に。項目 7 の `balloontimeout` は `sakura-time-directives` を待つ） | M（9〜13・Ref2 の中断位置を作るなら L 18〜24＝要件の段で切る） | **C4-⑩**・kanade の列（`msg.rs`・`actor.rs` に触らない＝`mcp-get-status` と並ぶ） | `mouse-drag-events`（✅ 10-05 完了） | − |
| `network-update-canon-order` | その他 | 更新のイベントを ukadoc の発生順序に揃える（MD5 の取り直し・台詞の終わりを待つ・総括の 2 段） | M（10〜14） | C5 の候補・`emo2_boot` の切替系（kanade の列から外した） | なし（列の順番待ち） | ○ |
| `balloon-canon-residue` | 優先 | 残り（項目 2＝面の偶奇で左右のバルーンを選ぶ・普通のバルーンだけ、を先に。項目 4 `\![reload,balloon]` は 10-04 に `mcp-reload` へ移した） | M（12〜16） | C5 の候補・`emo2_boot` の列 | `shell-balloon` | − |
| `currentghost-property-tree` | その他 | 動く値を出す口（持ち主）＋`currentghost.balloon.scope` の 19 項目（残りは 10-04 に `currentghost-property-others` へ切った） | M（13〜16） | `emo2_boot` の列・プロパティの動く値の列 | `property-query-channels`（読む道）か `mcp-get-property` | − |
| `property-catalog-lists` | その他 | 一覧（`ghostlist`・`balloonlist`・`activeghostlist`）と基盤待ちの枝の登記（`system.*` は 10-04 に `system-property-values` へ切った） | M（10〜14） | プロパティの動く値の列 | `currentghost-property-tree` | − |
| `zorder-property` | その他 | `currentghost.seriko.zorder` の読み書き | S（5〜8・書き込みの届け先を作るなら 8〜11） | プロパティの動く値の列 | `currentghost-property-tree`（動く値の口）・書く側は `property-query-channels` | − |
| `sakura-time-directives` | その他 | `quicksection`・`balloonwait`・`balloontimeout` のコンパイル側（`\_q` は議題）。C・D 群は覚え書きへ戻し、`\![embed]` は `sakura-embed-directive` へ（10-04） | M（10〜14） | コンパイルの列 | `choice-timeout-directive`・`talk-fast-forward` | ○ |
| `sakura-time-critical`（**10-03 起票**・`emo-text-file-split` の実機の観測から） | その他 | 台本の `\t`（タイムクリティカル）を読み、台本の終わりか中断・選択まで、マウス系などの通知を止めて `Status` に `timecritical` を載せる（今は読まない＝`\t` を書いたメニューもなでなでの返事に置き換わる） | S〜M（7〜10） | C5 の候補・kanade の列（`\t` を `decode_bare` で写せば台本のコンパイルの列から外れる） | `status-execution-states` | ○ |
| `makoto-dll-host` | その他 | MAKOTO/2.0 の DLL のホストと起動時の鎖（後から差し替えられる入れ物の形）。`\![load/unload/reload,makoto]` は 10-04 に `makoto-reload-directives` へ切った | L（15〜18） | host32-host の列 | `property-query-channels`（host32-host の列） | ○ |
| `mcp-stdio-bridge` | 優先 | Claude Desktop 用の stdio ⇔ HTTP 中継 exe | S〜M（8〜12） | C5 の候補（新クレート＝`Cargo.lock` の席・初回リリースの版上げの後） | なし（`mcp-server-core`・`release-package-versioned` は着地済み） | − |
| `mcp-kanade-tools` | 優先 | `sakurascript`・`raise_event`（`get_status` は 10-05 に `mcp-get-status` へ切った）。許可の表を迂回して任意の名前のイベントを送る口を作る（`property-query-channels`・`mcp-shiori-query` が使う） | M〜L（11〜15） | C5 の候補・kanade の列（その他の段より前へ出した） | `mcp-tool-entrances`・`mcp-get-status`（`msg.rs`・`actor.rs`） | ○ |
| `mcp-get-status`（**10-05 起票**・棚卸㉒で `mcp-kanade-tools` から切り出し） | 優先 | MCP の `get_status` が宛先のゴーストの実行の状態を SSP と同じ書式で返す（kanade の殻がその場で `ExecutionStatus::derive(..).render()` を返す・`schedule/` に触らない） | S（4〜6） | **C4-⑥**・kanade の列の外 | `mcp-tool-entrances`・`status-execution-states` | − |
| `mcp-reload` | 優先 | `reload` と台本の `\![reload,…]`（`\![reload,balloon]` も持つ＝10-04 の裁定） | L（15〜20・超えたら `\![reload,shiori]` を切る） | C5 の候補・kanade の列 | `mcp-tool-entrances` | ○ |
| `mcp-dump-images-residue`（**10-05 起票**・`mcp-dump-images` の完了時の棚卸） | 優先 | `dump_balloon` の文字の面の読み戻しが UI スレッドに残り、実機で UI スレッドの側が最大 14.2 ms（線 16 ms に近い）＝非同期の読み戻しにする。ほかに 5 件（届かない枝が判断の文言を `error!` つきで返す・`later` を通る組は UI スレッドで符号化し `catch_unwind` が無い・符号化のスレッドの記録をテストで数えられない・預けている間のゴーストの替わりが未確認・`sakurascript` が入ったら要件 7.7 ⑵ を撮り直す） | S〜M（5〜9） | **C4-⑧**（項目 1〜5・`mcp/mod.rs` に触らない。⑥ の撮り直しは `mcp-kanade-tools` の実機確認へ移せる） | `mcp-dump-images`（✅ 10-05 完了） | − |
| `mcp-strict-errors` | 優先 | `strict`（不在の面・未知のタグなどをエラーログへ） | M〜L（12〜18） | MCP の最後・kanade の列 | `mcp-log-history`・`mcp-kanade-tools` | ○ |
| `mcp-ghost-name-match`（**10-04 起票**・`mcp-get-property` の要件ディスカッションから） | 優先 | MCP の `ghost_name` の照合を SSP 2.9.07 に合わせる（英字の大小・本体側名・名前の前後の空白・空文字＝survey §7.4）。`mcp-tool-entrances` の裁定 6・7 を上書き | S（3〜5） | **C4-⑦**（3 段目は着地済み・`resolve.rs` は C3 で差分 0） | `mcp-tool-entrances`・3 段目の着地 | − |
| `mcp-author-tools`（**10-05 起票**・`/kiro-discovery`「エージェント目線の MCP ツール」） | 優先 | areka 独自ツールの登録口と橋（SSP の 10 本は変えない）＋読むだけの 3 本＝`check_script`（台本を再生せずに解釈）・`list_capabilities`（対応タグの一覧）・`validate_ghost`（設定ファイルの検査）。ツール名の接頭辞は要件の議題 | M〜L（14〜20・超えたら `validate_ghost` を切る） | **C4-⑨**（MCP の共有ファイルを触る＝`mcp_tests.rs`・`consumer_ledger.rs`・ほかのツールのファイルに触らない） | `mcp-tool-entrances` | ○ |
| `mcp-user-response`（**10-05 起票**・同） | 優先 | 選択肢・入力欄の答えと、なでる・クリックなどの操作を、`since_id` 付きでエージェントへ返すツール 1 本（さくらスクリプトでは届かない返り道・SSP にも無い） | M〜L（12〜18） | kanade の列（choice とマウスの配線） | `mcp-author-tools`・`mcp-kanade-tools`・`mouse-drag-events` | ○ |
| `mcp-shiori-query`（**10-05 起票**・同・開発者「SHIORI への情報要求が無い・raise_event はトークになるだけ」） | 優先 | ゴーストへ情報を問うツール 1 本＝SSTP NOTIFY と同じ振る舞いに `X-MCP-PassThru-*` の往復を足す（作法は正典の `X-SSTP-PassThru-*` を写し、名前の頭だけ経路に合わせる＝開発者裁定・ghost_terminal の `ShioriEcho` 系が実例）。データはヘッダで返り、台本が空なら喋らない。SHIORI リソースを外から直接読む形は正典に無い＝要件の議題（推しは採らない） | M（10〜14） | kanade の列（`actor_resources` の隣・host32-host の `shiori3.rs` も触る＝`script-security-level` と同じウェーブに置かない） | `mcp-author-tools` | ○ |
| `script-security-level`（**10-05 起票**・同・開発者「セキュリティレベルでさくらスクリプトに制約が出る・areka に未実装」） | 優先 | 台本とイベントに出どころを付けて運ぶ。SHIORI へ `SecurityLevel`・`SenderType` を正典の値で渡し、応答の `SecurityLevel` を読み、ukadoc が「外部からは不可」と書いたタグを止める（MCP は SSP と同じく Owned SSTP＝`local`） | M〜L（12〜18） | C5 の候補・kanade の列（host32-host の `shiori3.rs` も触る） | なし | ○ |
| `script-impact-tiers`（**10-05 起票**・同） | 優先 | タグの影響の段＝高（ベースウェアの外）・中（ベースウェアの中で演技を超えるもの）・低（アクターの演技の範疇・ゴースト切替を含む＝**制約なし**・開発者確定）の正本の表と、出どころごとの扱い（中・高は要件の議題） | M（10〜14） | `script-security-level` の後 | `script-security-level`・`mcp-kanade-tools` | ○ |
| `choice-script-prefix`（**10-05 起票**・`/kiro-discovery`「バルーンのリンクと OS の連携」） | 優先 | `\q[…,script:…]` を選ぶと `script:` の後ろを新しいトークとして実行する（今は `plan_cascade` が `Unsupported` にして警告 1 行で待ちを閉じるだけ）。出どころは元の台本と同じ扱い | S（5〜8） | **C4-⑭**・kanade の列（`schedule/choice.rs`・`on_choice`＝`steady.rs` は上限の近く→`choice.rs` の子の新しいファイル） | なし | − |
| `open-external-tags`（**10-05 起票**・同・開発者「バルーンからアプリを開く用途は結構ある」） | 優先 | `\j[http/https/file:///mailto:…]` と `\![open,file/browser/explorer/editor/mailer]` を OS の既定のアプリで開く（SSP の外部アプリの設定は写さない＝開発者「OS の受け口を最大限活用」）。開く処理を 1 か所に集めて毎回記録・台本から行き先を取り出す関数も持つ。`script-impact-tiers` より先に入れてよい（開発者裁定） | M（10〜14） | **C4-⑮**・台本のコンパイルの列（`decode.rs`）と `emo2_boot` の列（`consumer_ledger.rs`）・`readme.rs` の開く処理を一般化（新しいファイルは `readme.rs` の子） | なし | − |
| `range-choice-tag`（**10-05 起票**・同） | 優先 | `\__q[ID,…]…\__q`（範囲を丸ごと選択肢にする・複数行・画像のバナー）。未実装・引受先なしだった（`anchor-tag-canon` の brief の「実装済み」は誤り＝同 brief を直した） | M（10〜14） | 文字とバルーンの列・台本のコンパイルの列（`anchor-tag-canon` の次） | `anchor-tag-canon`・`choice-script-prefix` | ○ |
| `link-context-copy`（**10-05 起票**・同・**areka 独自の拡張**＝ukadoc に右クリック・クリップボードの正典は無い） | 優先 | 選択肢・アンカー・範囲の選択肢を右クリックすると小さなメニュー（`menu/win32.rs` を使い回す）を出し、行き先をクリップボードへ。決め方は 3 段＝`script:` の中の開く系の行き先 → ID や引数の `http(s)://`・`file:///`・`mailto:` → 表示されている文字（開発者裁定） | S〜M（6〜10） | 文字とバルーンの列（`input_events/` のバルーンと箱の押下） | `open-external-tags`・`anchor-tag-canon`・`range-choice-tag` | − |
| `wintf-tooltip`（**10-05 起票**・同・開発者「tooltip は色々汎用的にできた方が良い」） | 優先 | wintf の土台＝マウスが止まったことの検出（OS の待ち時間）と汎用のツールチップ（静的に範囲と文字を登録する口と、文字が後から届く動的な口） | M（8〜12） | **C4-⑯**・どの列にも入らない（wintf だけ・`Cargo.toml` に触らない） | なし | ○ |
| `balloon-link-hover`（**10-05 起票**・同・開発者「マウスホバーって出せる？ URL とか」） | 優先 | 正典の `balloon_tooltip`・`OnChoiceHover`・`OnAnchorHover` を実装し、ゴーストが何も返さないときだけ行き先（`link-context-copy` の 1・2 段目）を出す | M（8〜12） | 文字とバルーンの列と kanade の列（`link-context-copy` の次） | `wintf-tooltip`・`link-context-copy`・`anchor-tag-canon`・`range-choice-tag` | ○ |
| `shell-tooltip`（**10-05 起票**・同） | その他 | キャラクター窓のツールチップ＝surfaces.txt の tooltipブレス（`sakura.tooltips` など・今は黙って吸収）・SHIORI の `tooltip`・`currentghost.seriko.tooltip.*`。どれも引受先なしだった | M（8〜12） | シェルの element の列とプロパティの動く値の列（着手の前に照合） | `wintf-tooltip` | − |
| `baseware-root-list`（**10-05 起票**・ゴーストフォルダの複数管理） | その他 | 根を「`c:\areka` → `c:\ssp`」のように並びで持ち、上から探して最初に見つかったものを使う（`PATH` と同じ・下の同名は隠れる＝見分ける鍵はフォルダ名のまま）。新規のインストールは先頭の根・既存への上書きはその場。`AREKA_ROOT` の `;` 区切り・メニューの「根を追加…」「根を外す ▸」（OS のフォルダ選択）。SSP のフォルダ設定の個別の項目は写さない | M〜L（14〜18） | `emo2_boot` の結線の列（`ghost_switch.rs`・`shell_balloon_switch.rs`）と `install/`（`install-live-target-hazards`）に掛かる＝着手の前に照合 | なし | ○ |
| `dev-folder-alias`（**10-05 起票**・同） | その他 | 開発中の git の作業ツリーを「名前 → フォルダ」で登録し、探す順のいちばん上に置く（ゴースト・バルーン・ゴーストごとのシェル）。同じ名前の入っているものを隠して開発版を動かせる。エイリアスの先へはインストールも更新も書き込まない。登録は App の記憶へ手で書く＋環境変数（メニューなし） | M（8〜12） | `baseware-root-list` の後・同じ列 | `baseware-root-list` | − |
| `coverage-roadmap-refresh`（**10-02 に覚え書き `ukadoc-coverage-custody` を合流**） | その他 | 網羅台帳の手書きの数と持ち主が黙って偽になるのを止める（検査が `completed/` を走査・128 行の持ち主の付け替え・実在しない仕様名 107 項目） | M（14〜18） | C5 の候補（台帳を触る spec が 1 本も走らない席） | なし | − |
| `popup-menu-residue` | その他 | メニューの残件 4（記録の文言・テストの穴・World 借用中の `ShellExecuteW`）。次に `menu/` を触る spec へ相乗りが安い | S（5〜7） | 単独で取れる | なし | − |
| `clippy-199-lints` | その他 | clippy 1.99 で既存のコードに出た lint（dola・emo-compose・kanade・shiori 系・areka の約 40＋19 か所）を、振る舞いを変えずに消して `cargo clippy --workspace --all-targets -- -D warnings` を緑にする。test-all に clippy の段を足すかは議題 | S〜M（8〜12） | C5 の候補（段 1＝`dola`・`shiori-abi`・`areka-mcp` は空き席でも取れる・段 2 は列が空いてから・最初のタスクで測り直す） | なし | − |
| `anchor-style-canon`（**10-04 起票**・`anchor-tag-canon` から切り出し） | 優先 | `\f[anchor*]` 16 項目×3 状態・descript の `anchor(.notselect|.visited).font.*`・訪問済み・縦書きの下線（普通のバルーンと箱） | M（10〜13） | 文字とバルーンの列（`choice-marker-styling` の隣） | `anchor-tag-canon`・`text-align-shadow-canon` | − |
| `self-alpha-declaration`（**10-04 起票**・覚え書き「`shell-implicit-surface` の残り」⑵⑶⑸） | 優先 | シェルの `seriko.use_self_alpha` とバルーンの `use_self_alpha` を読み、`full`（全面不透明）と `0`（α を無視して抜き色）を正典どおりにする（今は常に `1` 扱い）。`surfaces.txt` の無い画像だけのシェルを起動の失敗にしない | S〜M（7〜10） | **C4-⑬**（シェルの element の列から外した＝本番の決め打ちは `shell_target.rs`・`balloon.rs` の 2 か所だけ） | `animated-image-decode`・`surface-element-nesting` | ○ |
| `seriko-trigger-intervals`（**10-04 起票**・同 ⑺） | 優先 | **口パクの `talk` が動かない**のを直す。`runonce`・`never`・`yen-e`・`periodic` と `\i[ID]`（`\i[ID,wait]` とアニメーションから別のアニメーションを呼ぶメソッドは別途） | M〜L（16〜20） | C5 の候補・シェルの element の列（働きの依存は無いが seriko の表と時計を `animated-image-playback` と分け合う） | `animated-image-playback`（ファイルの重なりだけ） | ○ |
| `extra-character-windows`（**10-04 起票**・覚え書き「3 人目以降のキャラクターの窓」） | その他 | `char{n≧2}` を持つゴーストで 3 人目からも窓とバルーンを出す（今は `derive_scopes` が 0・1 に固定） | M〜L（14〜20） | `emo2_boot` の列 | `balloon-canon-residue`・`shell-companion-balloon` | ○ |
| `update-check-options`（**10-04 起票**・覚え書き「更新のオプションと `OnUpdateCheck*`」） | その他 | 更新のオプション（`checkonly`・`testonly`・`recovery`）・`OnUpdateCheck*` の 4 語・`other_homeurl_override`（今は受けて何もしない） | M（10〜14） | `network-update-canon-order` の後（同じ `update/`・`update_cue.rs`） | `network-update-canon-order` | ○ |
| `sakura-embed-directive`（**10-04 起票**・`property-query-channels` から切り出し） | その他 | `\![embed]`（SHIORI へ問い合わせた結果を台本へ埋め込む） | M（10〜14） | kanade の列の後 | `property-query-channels` | ○ |
| `makoto-reload-directives`（**10-04 起票**・`makoto-dll-host` から切り出し） | その他 | `\![load/unload/reload,makoto]`・切替や更新の後の付け直し・`mcp-reload` の口 | M（7〜10） | host32-host の列 | `makoto-dll-host` | − |
| `currentghost-property-others`（**10-04 起票**・`currentghost-property-tree` から切り出し） | その他 | `currentghost.*` の残り（`balloon.scope` の 19 項目の外） | M〜L（14〜18） | プロパティの動く値の列 | `currentghost-property-tree` | − |
| `system-property-values`（**10-04 起票**・`property-catalog-lists` から切り出し） | その他 | `system.*` と Win32 の採り口 | M〜L（14〜18） | プロパティの動く値の列 | `currentghost-property-tree` | − |
| `property-name-case-fold`（**10-04 起票**・`mcp-get-property` の要件ディスカッションから） | その他 | プロパティの名前の英字の大小を区別せずに引く（SSP 2.9.07 に合わせる・ukadoc は黙っている）。書き込みの検査の大小の迂回もふさぐ。3 つの口（`GetProperty`・`%property[]`・`get_property`）は読み手に追従するだけ | S〜M（5〜9） | プロパティの動く値の列 | — | ○ |
| `surfaces-basepos` | 据え置き | surfaces.txt の `point.basepos`。宣言するシェルが要るまで始めない | S | シェルの element の列の最後 | — | − |
| `property-ipc-transport` | 据え置き | 台本を通さないプロパティの読み取り。**最初の 1 段（調べて裁定）で終わる形へ縮める** | XS〜S（裁定まで 2〜4） | — | — | ○ |
| `dpi-transition-two-tick-bounce` | 据え置き | 拡大率の切替で位置と大きさが 1 コマずれる（開発者が許容）。着手するなら最初に測り直し、跳ねが無ければ取り下げ | S〜M | — | — | ○ |
| `zorder-chain-residue` | 据え置き | 間欠赤のテスト族と文書。09-11 以降 main で赤 0 件＝先回りしない（A-2 は 10-05 に `ghost-session-test-load-flake` へ移した） | S〜M | — | — | ○（A 群） |
| `text-reveal-dance` | 据え置き | 字が現れるときだけ跳ねる・揺れる（夢・開発者が望んだときだけ） | M〜L | — | `text-reveal-fade` | ○ |
| `emo-text-canon-residue` | 据え置き | **取り下げ予定・着手しない**。残っていた 1 件（折り返しの警告にバルーンの名前を出す）は `shell-balloon` が引き取った。フォルダを消すと網羅台帳の整合検査（`roadmap-draft.md` の `[[spec]]` の表）が赤くなるので、表から外すのと一緒に `coverage-roadmap-refresh` が片付ける | — | — | — | − |
| `release-code-signing`（**10-02 起票**・配布と公開・任意） | 据え置き | SignPath Foundation（オープンソース向け・無償）に申請し、受理されたら CI で署名する。winget に署名は要らないので急がない | S（6〜9） | 配布と公開の列の最後 | `release-ci-workflow`・数回のリリースの実績 | − |
| `tick-gate-adoption` | 保留 | 既定で切の門を入れるか（待機中の CPU 17〜22% → 3% 未満の見込み）。短い A/B の測り方を先に組む | M〜L | 保留 | — | ○ |

## 覚え書き（brief なし・引受先が消えたまま忘れないための一覧）

> brief を書くほど固まっていない宿題。**着手の判断は棚卸で行う**（格上げするときは `/kiro-discovery`）。登記時の全文（数え方つき）は history「2026-10-02 棚卸⑳退避」、棚卸㉒の前の全文は history「2026-10-05 棚卸㉒退避」。

**製品の穴**

- **`present-write-coherence` の未達 40 件**（性能・L）。完了仕様が自ら「引受先なし・新規仕様の起票が必要」と書いた残量＝`visualize_to_write_us` が上限 16,667µs の 12.6〜18.4 倍・32 窓中 0 窓が上限以下。開発者裁定で「未達のまま GO」済み。着手前に測り直す。`tools/perf` の 3 経路・`tick-gate-adoption` の測り方と同じく「性能改善ループが動いてから」の宿題。
- **正典語彙の孤児 1 件**（S）。「スタイルシートのキーワード」（`larger` などは語彙のみ・使われたら記録を出す縮退）の持ち主。網羅台帳の `\f[height]` の行は `text-decoration-canon` の持ち物で implemented。起票するなら文字とバルーンの列で S の 1 本（`look.rs` だけ）。
- **`recommended.balloon`／`recommended.balloon.path`**（10-03 登記・S）。ゴーストの descript.txt の推奨バルーンで、これ以外へ切り替えると「強い内容の警告」を出す（ukadoc）。メッセージボックスを出さない方針（失敗は既定ゴーストの台詞で伝える）とどう折り合うかを決めてから起票する。`ghost-standard-balloon` が範囲外と明記。網羅台帳は `absent`・担当なし。
- **`install.txt` の `type,package`**（XS〜S）。今は `crates/areka-nar/src/manifest.rs` の `parse_kind` が明示のエラーで断る（黙って壊れはしない）。`developer_options.txt`（配布物を作る道具が読む）は棚卸㉒で「予約」の `createnar` の行へ移した。
- **壊れたゴーストを表示し続ける形**（互換・M・優先度は低い）。SSP は切替先の SHIORI が死んでいても切替を成功扱いにして表示し続ける。areka は「既定ゴーストへ戻して `OnBoot` の Ref6＝`halt`」を採った（09-26 裁定・生きている決まり 7 と逆向き）。あるべき姿としては残る。開発者が閉じてもよい。
- **SSTP の受信**（L）。`balloon-markers` の `sstpmarker`／`sstpmessage` が実際に画面に出るのはこれが入ってから。起票するときは同 spec の縮退の口を埋める。**MCP が 9801 を SSP と早い者勝ちで握る形と、SSTP の待受のポートの扱いがぶつかる**ことを議題に入れる（棚卸㉒）。
- **`sakura-time-directives` から戻した C・D 群**（10-04 登記）。時間の指令のうち、消費する者が現れるまで置くもの（同 brief の「2026-10-04 棚卸㉑で切った後の範囲」）。消費する者が現れたら、その spec の要件の段で引き取る。
- **管理者として動かした areka へは投げ込みが届かない**（S）。完了 `file-drop` の要件 8.9 は既知の制限として決めた。手当てするなら窓の生成の後（`crates/areka/src/placement/spawn.rs` の `WS_EX_ACCEPTFILES` を付ける所）で `ChangeWindowMessageFilterEx` により `WM_DROPFILES`・`WM_COPYDATA`・`0x49` を通す。起票するなら名前は `elevated-drop-filter`（触るのは `spawn.rs` と兄弟のテストだけ）。
- **持ち主のいない SHIORI のイベント 2 群**（棚卸㉒で `mcp-user-response` の再測定から登記）: ⑴ 単押しのクリックとホイール（`OnMouseClick`・`OnMouseClickEx`・`OnMouseWheel`）は送っていない（台帳 `absent`・持ち主なし。ホイールは wintf の `PointerState.wheel` に既にあり areka が読んでいない）⑵ 入力欄 `\![open,inputbox]` と `OnUserInput`・`OnUserInputCancel` は 0 件（台帳 `absent`・持ち主なし）。`mcp-user-response` は答えの返り道を作るだけで、これらの入口は持たない。起票するときは ⑴ は kanade の列（許可の表）と `input_events/`、⑵ はバルーンの UI。
- **`sample-ghost-kit` の `fold-samples` の `strip_folder` が取り出し元の先頭の 1 段しか比べない**（XS・`completed/areka-P0-install-companion-reading/tasks.md` の Implementation Notes）。設計で広げないと決めたもの。階層付きの取り出し元を持つ検体ができたら直す。

**潜在の性質（実害は観測されていない）**

- **SHIORI へ渡す置き場所のパスの形**（XS）。`load`／`loadu` へ渡すパスが `…\ghost/master` の形（区切りの混在・末尾の区切りなし）。出どころは `crates/areka-parsers/src/package/resolve_shell.rs` の `join("ghost/master")`（helper も末尾に区切りを足さない＝`shiori_proxy.rs` の `init_bytes`）。正典は沈黙。直すなら SHIORI へ渡す 2 つの口（`crates/areka-ghost/src/{shiori_inproc,shiori_wiring}.rs`）だけで揃えるのが影響が狭い。

**道具と試験**

- **`tools/perf` の実走していない 3 経路**（S）。`invoke-perf-run.ps1` へ `-GhostRoot`／`-BalloonRoot` を渡す実走・`invoke-followup-checks.ps1` の単独起動・`check-quiet.ps1`。性能改善ループを次に回す前に 3 経路を通す（`tools/perf` は `76e17654` 以降変わっていない）。
- **`image-webp` の取り込みを外す作業**（XS・`animated-image-decode` が持ち主をここに置いた）。`image-webp` は上流の GitHub の固定コミット（0.2.5）から取り込んでいる（`tech.md` の登記）。**棚卸のたびに crates.io で `image-webp` の最新版を引き**、0.2.5 以上が出ていたら `tech.md` の取り外し条件どおりに外す＝根の `Cargo.toml` の `[patch.crates-io]` の行・`deny.toml` の `allow-git` の行・`crates/areka-emo-atlas/src/webp_pin_tests.rs` と同じクレートの `lib.rs` の `mod webp_pin_tests;` の宣言を外し、`cargo update -p image-webp` で公開版へ戻して検体のテストを通す。**2026-10-05 棚卸㉒の確認: crates.io の最新は 0.2.4（2025-08-27）＝まだ外せない**。
- **一度だけ落ちた試験と、一度だけ出た出力**（原因未調査・再発したら出力を添えて起票）: `wintf --test graphics` の `STATUS_ACCESS_VIOLATION`（10-04・他のセッションの cargo と並走した `cargo test --workspace -j 4` で 60 秒を超えて止まった後に落ちた。単独では 97 本すべて緑。完了 `wintf-gpu-test-crash` と同じ形・`areka-test-threads-av` は範囲外と明記）。`areka-mcp` の `server::server_gate_help_tests::bad_origin_is_403_before_mcp` が全体テストの負荷の下で 1 回赤（10-04・os error 10053・単独では緑）。`cargo test -p areka-emo-present` の出力に `__rust_alloc_error_handler` のバックトレース行が 1 回だけ混じった（10-05・終了コード 0）。**棚卸㉒で外したもの**: `fallback_tests` の 2 本・`install::desk::overwrite_tests` の 2 本・`emo2_boot::spine` の 3 本・`sample-ghost-kit` の展開テストの os error 5（`target\nar-samples\work` の退避と同じ族）は `ghost-session-test-load-flake` が引き取った。`tools/test-all.ps1` の x64 の段の終了コード -1 は C: の空きが 35MB だったときの環境の問題として閉じた。

## 直接修正候補（spec なし）

> **2026-10-05 棚卸㉒で直した**（開発者「軽微な修正は直ちに実施してクローズ」）: ⑴ `crates/areka/src/emo2_boot/move_cue.rs` のモジュール全体の `#![allow(dead_code)]`（「結線の着地で撤去する」と書いたまま結線は済んでいた）を外した。外すと鳴った 4 つのうち 3 つ（`m1_degradations` ほか）は、**本番で一度も呼ばれておらず、`\![move]` の縮退（`screen` などの基準・時間付きの移動）を記録なしに受けていた**＝ログ無しの縮退だったので、送り出しの所で `warn!` を出すように直した。残る 1 つ（`RefPoint::BASE_BASE`）はテストだけが使うので `#[cfg(test)]` にした。`cargo check -p areka --all-targets` の警告 0・`move_cue` のテスト 37 本緑 ⑵ 段階実装の名残のコメント（emo-atlas の `decode.rs` の「後続タスク」・`log_history.rs` の `last_id()` の持ち主を `mcp-strict-errors` と名指し）⑶ `README.md` の MCP の 1 文（「ツールはこれから整備」→ 10 本のうち 6 本が動く）⑷ steering の古い数と状態（`focus.md`・`product.md`・`structure.md` に動く絵の読み込みと `log_history.rs`）⑸ brief の書き損じ（7 本の末尾に紛れた書き込みの道具の残りかす 2 行・`ghost-standard-balloon` の本文の取り下げ済みの「`_` への置き換え」）。
>
> **直さずに各 spec へ渡したもの**: 網羅台帳の持ち主の書き違い（`shiori.toml` の `property.get:1`・`property.set:1` は `property-ipc-transport` へ・`SenderType` の行は `script-security-level` へ・`sakura-script.toml` の `\![embed]` は `sakura-embed-directive` へ・`OnBalloon*` の行は `balloon-lifecycle-events` へ）は、`roadmap-draft.md` の `owner_count` と一緒に動かす決まり（「着手手順」）なので、各 spec の要件の段で直す（brief に記録済み）。
>
> **2026-10-04 棚卸㉑・10-03・10-02 棚卸⑳で直したもの**は history「2026-10-05 棚卸㉒退避」の旧覚え書きの節の後ろ（旧「直接修正候補」）を参照。

残っている候補: なし。

取り下げた 1 件を再登記しないこと——判定器（`crates/areka/src/placement/transition_judge_verdict.rs`）の窓ごとの書込上限が見送り窓を除いていないのは**意図どおり**（`completed/areka-P0-dpi-transition-atomicity/mechanism-ledger.md` §13.1・2026-09-24 に開発者が再確認）。

## 生きている決まり（棚卸⑭〜⑲の裁定のうち今も効くもの・全文は history）

1. **着手の優先度は 3 段＝① バグ ② リリース関係（まず 1 回出す） ③ 優先度の高い機能＝MCP・バルーン・アニメーション**（2026-10-05 開発者・10-03 の「① バグ ② リリース・バルーン・動く画像・MCP ③ その他」を改めた）。**それ以外（台帳の段「その他」・据え置き・保留）は開発者が一覧から優先度を判断する**まで、ウェーブに自分からは入れない。バグかどうかは実測で分ける（利用者から見える実害／潜在／テストの穴／正典の追加）。
2. **規模の上限は 1 spec 20 タスク**。超えるものは要件の段で切る。**一度切り出した spec を、上限を少しまたぐ理由でさらに削らない**（09-28）。
3. **並走の物差し＝触るソースファイルの重なり 0**（09-20）。**文書だけの段は置かない**（09-27・要件は前の spec の着地で古くなる）。
4. **エンジンを切り出した spec は網羅台帳に触らない**。台帳の更新は結線の側がまとめて行う（09-20）。
5. **`zorder-chain-residue` の A 群は先回りしない**（main で赤が出たら、その時点で A 群だけを単独で挟む）。
6. **メニューは Win32 標準**・オーナードローと着せ替えメニューは予約のまま。**ゴースト切替はプロセス内**・アプリの寿命は窓の数から切り離す。**HTTP は WinHTTP・MD5 は OS の CNG**。**投げ込みは `WM_DROPFILES`**。**消滅（`\![vanishbymyself]`）は要望が出たら S で切る**。
7. **失敗は既定ゴースト（emo2）の台詞で伝える**（`OnBoot` の Ref6＝`halt`・Ref7＝落ちたゴーストの名前）。メッセージボックスは出さない。**インストールと切り替えは別のイベント**（入れた後の切替を areka は主導しない）。
8. **要件定義・設計のサブエージェントは起動中のモデルを継承し、上位へ上げない**（09-30）。Fable 列は「Fable で起動したセッションを勧める」だけ。
9. **並走できる spec の紹介は「完全並走できるものだけ」・Fable 推奨を先頭に、名前＋一言＋コマンドの短い形**（09-26）。
10. **話の最中（選択待ちを含む）に届いたマウス系の返事は今の話を置き換える＝正典どおり**（2026-10-03・`emo-text-file-split` の実機で「メニューがなでなでの返事に置き換わる」を観測して確かめた。正本は完了 `input-events` の DD-IE-1／DD-IE-2）。止めるのはゴーストの側（`Status` の `talking`／`choosing` を見て黙る）か台本の `\t`。areka の穴は `\t` を読まないことだけ＝`sakura-time-critical`。

## 棚卸㉒の裁定（2026-10-05・main `f26aa1c1`・開発者指示「main が進んだので棚卸＆深掘り＆徹底ブリーフィング。即時修正は実施してクローズ。ロードマップだけにあって brief の無いものを精査して立ち上げ。負荷が高すぎる仕様は分割を検討。厳しめの目線で並走できるウェーブを。優先度は ① バグ ② リリース（とりあえず 1 回出したい）③ 優先度の高い機能（MCP・バルーン・アニメーション）。低優先は一覧だけ見せて判断する」）

> C3（11 本）の着地の後の棚卸。brief を持つ 68 本すべてを、サブエージェント 7 系統（Opus）で main `f26aa1c1` に照らして再測定し、結果を各 brief の「2026-10-05 棚卸㉒の再測定」節へ書いた（触るファイル・規模・議題・穴）。8 系統目が即時修正の候補と brief の無い宿題を洗った。棚卸㉑の裁定の全文・旧ウェーブ表・旧覚え書き・旧直接修正候補は history「2026-10-05 棚卸㉒退避」。

1. **優先度の段を改めた**（開発者・10-05）: ① バグ ② リリース関係（まず 1 回出す） ③ MCP・バルーン・アニメーション。それ以外は開発者が一覧から判断する（生きている決まり 1 を書き換えた）。台帳の「段」の列は今までどおり（バグ／優先／その他）で、優先の中の並びを ② → ③ とする。
2. **即時修正を実施して閉じた**（「直接修正候補」節）: `move_cue.rs` の古い allow を外した結果、**`\![move]` の縮退が本番で記録されていなかった**のを見つけて `warn!` を足した（ログ無しの縮退の禁止）。ほかはコメント・README・steering・brief の書き損じだけ。網羅台帳の持ち主の書き違い 4 件は `owner_count` と一緒に動かす決まりなので各 spec へ渡した。
3. **2 本を切り出した**（どちらも理由は並走・境界がはっきり別の場所）: `mcp-kanade-tools` → `get_status` を `mcp-get-status`（S・kanade の殻がその場で答える＝`schedule/` に触らない）／`animated-image-playback` → `import` を `animated-image-import`（S〜M・`import` だけが読み手・`manifest.rs`・`plan.rs` を触る。本体は 18〜22 → 14〜17）。**切らなかったもの**: `script-security-level`（12〜18・上振れで 22＝超えたら要件の段で「ヘッダ」と「台本の印とタグの制約」に）・`text-align-shadow-canon`・`balloon-markers`・`mcp-reload`・`mcp-author-tools`（いずれも超えたら要件の段で切る案を brief に書いた）。
4. **ロードマップだけにあって brief の無い宿題は、起票に当たるものが無かった**: 覚え書き・予約の全項目を照らし、どれも「開発者の決めごと待ち」「消費する者待ち」「優先度の 3 段の外で小さい」のいずれか。閉じたもの＝`shell-implicit-surface` の残り ⑴⑷⑹（`self-alpha-declaration` の brief に入っていた）・wintf の兄弟の重なり順（`balloon-element-order` の brief と二重の登記）・`InProc` の SHIORI（予約の「`IShiori` in-proc」の行へ）・`developer_options.txt`（予約の `createnar` の行へ）・負荷のときの赤 4 族（`ghost-session-test-load-flake` が引き取った）。**新しく登記した**＝持ち主のいない SHIORI のイベント 2 群（単押しのクリックとホイール・入力欄）・`fold-samples` の `strip_folder`。
5. **二重の持ち主を 1 つにした**: `SenderType`・`SecurityLevel` を SHIORI へ運ぶ仕組みは `script-security-level`（`property-query-channels` から寄せた・`property-query-channels` は host32-host の列から外れた）。許可の表を迂回して任意の名前のイベントを送る口は `mcp-kanade-tools`（`property-query-channels`・`mcp-shiori-query` が使う）。`zorder-chain-residue` の A-2 は `ghost-session-test-load-flake` へ（同じ `spine.rs`）。
6. **列を直した**: kanade の列で優先の段（`mcp-get-status`・`mcp-kanade-tools`・`mcp-reload`・`script-security-level`・`mcp-strict-errors`）をその他の段（`sakura-time-critical`・`property-query-channels`）より前へ。シェルの element の列から `self-alpha-declaration` を外した（棚卸㉑の「本番の決め打ち 5 か所」は誤り＝本番は 2 か所）。`seriko-trigger-intervals` は `animated-image-playback` に働きの上では依存しない（ファイルを分け合うだけ）。`extent-element-offset` は `element-base-method` の後（同じ `plan.rs`）。文字とバルーンの列の共有点に emo-text の `lib.rs`（純粋な一覧の数の行）を足した。
7. **棚卸㉑の記述の誤りを直した**: `self-alpha-declaration` の本番の決め打ちの数（5 → 2）・`judge_box_click`・`BoxPressVerdict` は 2 値でなく 4 値・`balloon-font-file` の「箱の `font.name` に渡す所」は実在しない（同 spec が最初の読み手を作る）・`budoux-reveal-reflow` と `balloon-font-file` は「重ならない見込み」でなく `actor_present.rs`・`lib.rs` が重なる。
8. **ウェーブ C4 を 16 本で組んだ**（バグ 4・リリース 1・MCP 4・バルーン 2・バルーンのリンク 3・アニメーション 2・想定タスクの合計＝109〜168。リンクの 3 本は下の 13）。初回リリース（`release-cycle` の `v0.0.2`）を「ウェーブの間の手順」から C4 の 1 席へ入れた＝「開いている PR 0 本」の条件を「`Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md` を触る開いた PR 0 本」に改めた（理由は「初回リリースの段取り」）。許す重なりに「説明書 `dist/README.txt` の別々の行」を足した。
9. **C4 から C5 へ回したもの**: `winget-manifest-submission`（Release の実在が要る・`winget.yml` は初回の手提出が取り込まれてから main へ）・`mcp-stdio-bridge`（版上げと同じ `Cargo.lock`）・`balloon-font-file`・`anchor-tag-canon`（どちらも `budoux-reveal-reflow` と重なる＝バグを先に）・`balloon-canon-residue`（`self-alpha-declaration` と emo-present の `balloon.rs`）・`mcp-reload`（`char-position-save-on-exit` と `ghost_switch.rs`、`balloon-lifecycle-events` と `emo2_boot/mod.rs`）・`areka-test-threads-av`（`ghost-session-test-load-flake` とテストの土台）・`extent-element-offset`（`element-base-method` と `plan.rs`）。
10. **再測定で見つけた穴（各 brief に記録・起票は不要）**: `animated-image-playback` ＝ `import` の行のファイル名が読み手で 0 に化け記録も出ない（`animated-image-import` が直す）／`element-base-method` ＝ ukadoc は element1 以降の `base` を overlay に読み替えると書く（記録の対象でなく overlay で描くのが正典）・読み手の `Element` に欄を足すと 6 クレート約 33 ファイルの直書きが壊れる（議題 1）／`char-position-save-on-exit` ＝ゴーストの切替でも同じ問題・終了の通知の後では書き込みが届かない恐れ（書く時機は「ゴーストを降ろす前の共通の入口」が議題）／`property-name-case-fold` ＝`classify_set` が大小まで完全一致で比べ、`CURRENTGHOST.NAME` が「書けない名前」をすり抜ける（今は台本から SET へ届く道が無いので害は無い＝`property-query-channels` より先に着地させる）／`ghost-session-test-load-flake` ＝足場を使うファイルは 9 本でなく 28 本・`spin_wait_until` の 100 万回の空回しが負荷を自分で強める疑い／`areka-test-threads-av` ＝`mcp-dump-images` で実際の GPU を使うテストが 15 本増えた／`winget-manifest-submission` ＝ winget-releaser は既に在るパッケージしか更新できない／`release-code-signing` ＝同梱の i686 `pasta.dll` は署名されないまま／`currentghost-property-others` ＝ `get_property_tests.rs` の 1 本が `currentghost.name` の値なしを期待している（出すと赤＝期待を直す）。
11. **Fable で要件定義を勧める spec（C4）**: `budoux-reveal-reflow`・`element-base-method`・`char-position-save-on-exit`・`mcp-author-tools`・`ghost-standard-balloon`・`animated-image-playback`・`self-alpha-declaration`・`wintf-tooltip`。Opus で足りる: `ghost-session-test-load-flake`・`release-cycle`・`mcp-get-status`・`mcp-ghost-name-match`・`mcp-dump-images-residue`・`balloon-lifecycle-events`・`choice-script-prefix`・`open-external-tags`。
12. **開発者に決めてほしいこと**: ⑴ その他の段（台帳の「その他」20 本＋据え置き 7 本＋保留 1 本）の優先度（一覧は報告に添えた）⑵ `release-cycle` の初回の PR の分け方と `README.md` の行の直し手（推しは「初回の段取り」節。要件ディスカッションで聞く）⑶ M3 のゴール（棚卸⑳から持ち越し）。どれも C4 の着手を止めない。開発者の手作業＝crates.io の `wintf`・`dola` に Trusted Publishing を足す（タグの前）。
13. **「バルーンのリンクと OS の連携」の 7 本を優先度の高い機能として取り込んだ**（PR#243・main `ec072853` で起票。開発者の依頼で起票したセッションから相談を受けた）: 先頭の 3 本＝`choice-script-prefix`（C4-⑭）・`open-external-tags`（C4-⑮）・`wintf-tooltip`（C4-⑯）を C4 へ前倒しした。照合: ⑭ は kanade の `schedule/choice.rs`・`steady.rs` の `on_choice` だけで、新しいファイルを `choice.rs` の子にすれば ⑩ の `schedule/mod.rs`・`lib.rs` と ⑥ の `msg.rs`・`actor.rs` に触れない／⑮ の `sakura/decode.rs`・`compile.rs`・`consumer_ledger.rs`・`readme.rs` は C4 のほかの誰も触らない（開く処理の新しいファイルを `readme.rs` の子にすれば ⑨ の見込みの `main.rs` にも触れない）／⑯ は wintf だけで、C4 に wintf を触る spec は無い（`Win32_UI_Controls` は根の `Cargo.toml` で有効＝版上げの条件にも触れない）。**`anchor-tag-canon` は C4 へ動かせない**（② の `state.rs`・`actor_present.rs`、⑮ の `decode.rs`、⑩ の kanade の `events.rs`・`schedule/mod.rs`・`lib.rs` と重なる）＝C5 の文字とバルーンの列の先頭に置いた。文字とバルーンの列は `budoux-reveal-reflow` → `anchor-tag-canon` → `range-choice-tag` → `link-context-copy` → `balloon-link-hover` → `balloon-font-file` → `text-typesetting` … の順にした（リンクの列を `text-typesetting` 以降より前へ・`balloon-font-file` は重ならない席が見つかれば前へ）。`shell-tooltip`（その他）は開発者の判断の一覧へ。`open-external-tags` の brief の「MCP の `sakurascript` は実装済み」は誤り（まだ `NG:not implemented yet`＝`mcp-kanade-tools`）＝同 brief を直した。

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
- [ ] areka-P0-mcp-get-status -- get_status（10-05 棚卸㉒で mcp-kanade-tools から切り出し）。Dependencies: areka-P0-mcp-tool-entrances
- [ ] areka-P0-mcp-kanade-tools -- sakurascript・raise_event。Dependencies: areka-P0-mcp-tool-entrances, areka-P0-mcp-get-status
- [x] areka-P0-mcp-expression-table -- get_expression_table。Dependencies: areka-P0-mcp-tool-entrances
- [x] areka-P0-mcp-log-history -- get_log。Dependencies: areka-P0-mcp-tool-entrances
- [ ] areka-P0-mcp-reload -- reload と \![reload,…]。Dependencies: areka-P0-mcp-tool-entrances, areka-P0-shell-balloon-switch
- [x] areka-P0-mcp-dump-images -- dump_surface・dump_balloon。Dependencies: areka-P0-mcp-tool-entrances
- [ ] areka-P0-mcp-strict-errors -- strict の記録。Dependencies: areka-P0-mcp-log-history, areka-P0-mcp-kanade-tools
- [ ] areka-P0-mcp-ghost-name-match -- ghost_name の照合を SSP 2.9.07 に合わせる（大小・本体側名・前後の空白・空文字）。共有の resolve.rs を触る＝3 段目と同じウェーブに置かない。Dependencies: areka-P0-mcp-tool-entrances（席は 3 段目の着地の後）

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
- [ ] areka-P0-animated-image-playback -- 動く絵を子サーフェスへ分解して置く自動アニメーション・interval `always`。Dependencies: areka-P0-animated-image-decode, areka-P0-surface-element-nesting
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
- [ ] areka-P0-ghost-standard-balloon -- 起動時のゴーストの標準バルーン（同梱の最初の 1 個・descript の `balloon`／`default.balloon.path`）。Dependencies: areka-P0-install-companion-reading
- [ ] areka-P0-shell-companion-balloon -- シェルの書庫の同梱バルーンをそのシェルに紐づけ、着替えと起動で使う。Dependencies: areka-P0-ghost-standard-balloon, areka-P0-shell-balloon

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

- [ ] areka-P0-mcp-author-tools -- 独自ツールの登録口と橋＋`check_script`・`list_capabilities`・`validate_ghost`（読むだけ）。Dependencies: areka-P0-mcp-tool-entrances
- [ ] areka-P0-script-security-level -- 出どころの運搬・SHIORI の `SecurityLevel`／`SenderType`・ukadoc のタグの制約。Dependencies: none
- [ ] areka-P0-script-impact-tiers -- 影響の段（高・中・低）の正本の表と出どころごとの扱い。Dependencies: areka-P0-script-security-level, areka-P0-mcp-kanade-tools
- [ ] areka-P0-mcp-user-response -- 選択肢・入力・操作をエージェントへ返す。Dependencies: areka-P0-mcp-author-tools, areka-P0-mcp-kanade-tools, areka-P0-mouse-drag-events
- [ ] areka-P0-mcp-shiori-query -- ゴーストへ情報を問う（SSTP NOTIFY 相当＋`X-MCP-PassThru-*` の往復）。Dependencies: areka-P0-mcp-author-tools

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
- [ ] areka-P0-release-cycle -- 繰り返し spec。版 +0.0.1 → PR → squash マージ → タグ（初回は `wintf`・`dola` の Trusted Publishing の設定を含む）。Dependencies: areka-P0-release-ci-workflow, areka-P0-crates-io-publish
- [ ] areka-P0-winget-manifest-submission -- `Areka.Areka.Portable` のマニフェスト・初回の手提出・winget-releaser。Dependencies: areka-P0-release-cycle
- [x] areka-P0-crates-io-publish -- `wintf`・`dola` だけを crates.io へ（Trusted Publishing・自分の workflow `crates-io.yml`）。Dependencies: areka-P0-release-package-versioned, areka-P0-mcp-server-core（C2・`release-ci-workflow` と並走）
- [ ] areka-P0-release-code-signing -- SignPath Foundation への申請と CI での署名（任意）。Dependencies: areka-P0-release-ci-workflow

### mouse-drag-events の持ち越し（2026-10-05 `/kiro-discovery` で起票）

> 出どころは `mouse-drag-events` の実機（4.2・クローディア）と完了時の棚卸（`completed/areka-P0-mouse-drag-events/tasks.md` の Implementation Notes・`verification/real-machine.md` 4 章）。開発者の方針（2026-10-05）「実機で areka が未対応だったためにうまくいかなかった件はすべて起票」。

- **分け方**: 4 本にした。`base` と `collisionex` はどちらも `shell/decode.rs` を触るが、描画（合成器）と当たり判定（`hit.rs`）で直す先が分かれるので別にした（同じ列で直列）。位置の保存は `placement/`、テストの AV はテストの土台で、触る所が重ならない。
- **負荷のときの赤**（`install` の desk の上書き・`session_end` の `sync_send`・`emo2_boot::ghost_switch` の `boot_event_tests`）は新しく起票せず、`ghost-session-test-load-flake` の brief の Current State へ観測を足した。

#### Specs (dependency order)

- [ ] areka-P0-element-base-method -- element定義の描画メソッド `base` を読んで描き、他の未対応の描画メソッドの行は記録を出す（バグ）。Dependencies: areka-P0-surface-element-nesting
- [ ] areka-P0-collisionex-regions -- `collisionex` の 4 つの形を読んで当たり判定に使い、Reference4 に名前を載せる。Dependencies: areka-P0-element-base-method
- [ ] areka-P0-mcp-dump-images-residue -- `dump_balloon` の文字の面の読み戻しを UI スレッドから外し、`dump` の 2 本の後始末 5 件を片付ける。Dependencies: areka-P0-mcp-dump-images, areka-P0-mcp-kanade-tools（後始末の ⑥ だけ）
- [ ] areka-P0-char-position-save-on-exit -- 正常な終了で全キャラクター窓の位置を記憶に書き、再起動で前回の並びに立つ（バグ）。Dependencies: なし
- [ ] areka-P0-areka-test-threads-av -- `--test-threads=4` の `STATUS_ACCESS_VIOLATION` の組を特定して直す（バグ）。Dependencies: なし

### バルーンのリンクと OS の連携（2026-10-05 `/kiro-discovery` で起票）

> 開発者の問い（2026-10-05）「さくらスクリプトで、リンク（選択肢）をクリックしたらファイルを開いたりできるか？ また、リンクを右クリックしたらリンクに登録されている文字列（URL とか）をコピペできたりしますか？ バルーンからアプリを開く用途は結構あると思う。SHIORI でもできなくはないけど、あったらうれしい機能だと思う。ukadoc を調べて、既に存在するタグなのかどうかも確認せよ」。

- **ukadoc で確かめたこと**:
  - クリックで開くのは正典にある。`\q[タイトル,script:…]`（`\__q` も同じ）と、`\j[http/file:///mailto:…]`・`\![open,file/browser/explorer/editor/mailer]` の組み合わせで、SHIORI を通さずに書ける（例 `\q[メモ帳,script:\![open,file,notepad.exe]]`）。`\_a` には `script:` の形が無く、SHIORI の `OnAnchorSelect` の応答で `\j` を返すのが作法。
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

- [ ] areka-P0-choice-script-prefix -- `\q[…,script:…]` の実行。Dependencies: なし
- [ ] areka-P0-open-external-tags -- `\j` と `\![open,file/browser/explorer/editor/mailer]` を OS の既定のアプリで開く・開く処理の 1 か所と記録・行き先を取り出す関数。Dependencies: なし
- [ ] areka-P0-wintf-tooltip -- wintf の止まったことの検出と汎用のツールチップ。Dependencies: なし
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

## 予約（全て任意・brief なし）

アプリ層＝FMO・DirectSSTP・Plugin/HEADLINE・多重ゴースト・トレイアイコン・消滅・設定画面と設定ファイル（**動く絵の 3 つの上限を設定項目として引き取る**＝今は環境変数 `AREKA_ANIMATED_IMAGE_MAX_FRAMES`・`AREKA_ANIMATED_IMAGE_MAX_PIXELS`・`AREKA_ANIMATED_IMAGE_MAX_TOTAL_PIXELS` で変える・`animated-image-decode` の要件 6.9〜6.12。値は `bake_with_limits` へ渡す `AnimationLimits` をそのまま使える）・`\![update,platform]`（本体の更新）・**インストーラー版（winget の名乗り `Areka.Areka`・データは `%APPDATA%` へ・スタートメニューのアイコン）**・`\![execute,createnar]`／`createupdatedata`（作る側＝開発者機能・配布物を作る道具が読む `developer_options.txt` もここ＝10-05 に覚え書きから移した）。SSTP の受信は「覚え書き」。互換面＝SAORI は実装しない（SHIORI が直接 `LoadLibrary`・台帳 `not-applicable`）・里々/YAYA 網羅。emo テキスト進化＝回転テキスト（`TextEffects` 予約名 `rotation`／`multicolor`）。**`\f[sub]`／`\f[sup]`／`\f[outline]` は DirectWrite の標準機能で表せる手段が見つかるまで語彙のみ**（2026-09-11 裁定・台帳 `vocabulary-only`・追跡先はこの行）。バルーン美観配置。pasta の native x64／`IShiori` in-proc（本番に使うときは `InProc` の SHIORI を外から終わらせる手を決める＝10-05 に覚え書きから移した）・ベクトル描画・owner-draw 右クリックメニュー（`popup-menu-minimal` の構造の上に見た目を被せる）・着せ替えメニュー。段階 A の先頭ウェーブ 6 束（`roadmap-draft.md`・324 件）は M3 のゴールを決めるときの材料。

---

**追記台帳**: 追記(95)〜(103) の要約は history「2026-09-26 番号表記の廃止に伴う退避」節。
