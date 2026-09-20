---
inclusion: manual
updated_at: 2026-09-19
---

# Roadmap — areka（M2＝α 版・第三者がデスクトップマスコットを管理できる最小のアプリ）

> **M1 は 2026-09-11 に完成宣言済み**（下記「M1 ゴール」）。**M2 のゴールは 2026-09-18 の棚卸⑭で「α 版」に決めた**（下記「M2 ゴール（α）」）——開発者の指示「α 版として第三者に使い始めてもらうことができるだけの機能セット。大事なのはゴースト・シェル・バルーンのファイル管理、インストール、ネットワーク更新、つまりアプリとしてのデスクトップマスコット管理。オーナードローは不要だが最低限のメニューは要る。表現力増強は emo2 が普通に動いている水準で一旦よい」。本ファイルは **brief 済み spec 38 本（うち完了 9・α 10・α 後 19）の着手順（ウェーブ）と干渉条件だけ**を持つ。
> 正本配置: 本ファイルが正本（`.kiro/steering/roadmap.md`）。`focus.md`（`inclusion: always`）から辿る。設計判断の正本は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md)。M1 実物スコープは [doc/emo2-conformance-scope.md](../../doc/emo2-conformance-scope.md)。ukadoc 網羅の段階・順位の正本は `doc/ukadoc-coverage/`（`briefing.md`・`linkage.md`・`roadmap-draft.md`）。
> **履歴**: 追記①〜(94)・旧ゴール表・旧ウェーブ行・旧干渉台帳・完了詳報は棚卸④〜⑬で [roadmap-history.md](roadmap-history.md) へ退避済み。**棚卸⑭（09-18）で旧 W14〜W17 のウェーブ行と W14 の干渉台帳を退避**（history が全文正本・非改変）。完了ユニットの実装詳細は各 `completed/` spec が正本。**旧ウェーブ番号の読み替え**: 棚卸⑫（09-02）で W5.95→W6・W6→W7・W6.5→W8・W6.75→W9・W6.9→W10・W6.95→W11・旧 W7（e2e）→W12 へ整数化。棚卸⑬（09-11）で旧「W13 裁定枠 D〜G」「W13〜W15（裁定枠）」を W13〜W17 へ振り直し。**棚卸⑭（09-18）で W14〜W17 を「α 後」へ格下げし、α のウェーブを A0〜A5 と呼ぶ（09-18 同日に A0＝nar-install へ反転し 6 段）**（history と completed spec 内の旧番号は改変しない）。

## M1 ゴール ✅（2026-09-11 完成宣言）

areka（x64）が最小 SSP 互換ベースウェアとして、適合対象ゴースト **emo2**（作者自作・脳=`pasta.dll`・32bit SHIORI）を「そのまま」起動→会話→撫で→メニュー→終了まで E2E 実走させる。emo2 が動く＝同じ汎用 32bit ブリッジで里々/YAYA も動く土台。

**✅ 完成宣言**: 適合検証項目表 20 項目全合格の実機サインオフに開発者の署名（`.kiro/specs/completed/areka-P0-emo2-conformance-e2e/verification/m1-completion.md`・項目別は同 `acceptance-record.md` §8.1）。持ち越し 8 件（同 §6）はいずれも判定を書き換えず、引受先は下の spec 台帳に全て実在する（§13.1 行 3「初回起動限定の位置調整」だけは「違和感があれば個別仕様を切る」の据え置き＝spec なし）。

## M2 ゴール（α）＝第三者がデスクトップマスコットを管理できる（2026-09-18 棚卸⑭で確定）

**利用者の一周**: zip を展開して起動 → `.nar` を窓へ落とす（またはメニューから選ぶ）→ ゴーストが起動する → 右クリックメニューでゴースト／シェル／バルーンを替える → ネットワーク更新で作者の修正を受け取る → 終了 → 再起動で前回の状態に戻る。**表現力は M1 の水準（emo2 が普通に動く）で据え置く。オーナードローメニューは持たない（Win32 標準メニュー）。**

**完成の器**: `areka-P0-alpha-release-signoff`（配布 zip＋第三者の手順 11 項目の実機サインオフ＋開発者の署名）。M1 と同じく人間判断・自動判定にしない。

**α に要る振る舞いと今日の実在（2026-09-18 探索）**: インストーラ・列挙・切替・ネットワーク・メニュー・投げ込みの **6 種とも実装 0**。再利用できるのは単一ゴーストのマウント解決（`areka-parsers/src/package/resolve.rs`）・スコープ別 TOML 永続化（`App` スコープが空のまま配線済み・`areka-sylphya/src/persist/mod.rs`）・`nar-install` の brief（エンジン側）。詳細は各 brief。

**正典の出どころ**: `doc/ukadoc-coverage/roadmap-draft.md` 段階 B（「迎えて、育てて、見送る」）の束「インストール」「更新」「切替」「配布物の素性」「投げ込み」と、段階 A 順位 11 の束「メニュー」・段階 E の束「作り付けの窓」の一部。**段階 A の先頭ウェーブ 6 束（会話・窓の配置・名前の記憶・起動と挨拶・バルーンの文字・サーフェスアニメーション）は α 後へ回す**——「表現力は据え置き」の裁定による。段階 B を A より先に置くのは `briefing.md` の順位付け 4 軸（壊れ方＞伺からしさ＞資産の広さ＞基盤共有度）と逆だが、**開発者が「第三者が使い始められる」を M2 の物差しに置き換えた**ためである（棚卸⑭仮裁定 1）。

## 実装規律（balloon-system の失敗から得た正）

- **実装ファースト**: 各作業ユニットの成果物は「実際に動く」検証済みコード。**spec 工場の禁止**: 成果物が子 spec になる構造を作らない。1 ユニット＝1 かたまりの動く振る舞い。
- **最小実装＋薄い拡張シーム**: 使う分だけ実装し、拡張は型/レジストリの口だけ残す。抽象は「2 例目の実物」が要求してから。動く資産から建てる。
- **粒度基準**: 1 ユニット＝単一 pass/fail の独立観測。純粋層は fixture/mock 直入力で切る。UI 位置決め・座標系は本番ゴースト（実 emo2）＋実 DPI（≠96）が観測条件（記憶 areka-placement-real-ghost-first）。
- **語彙完備・配線ゼロの追跡**: 先送りシームには狭い `#[allow(dead_code)]`＋実在理由の doc を義務付け、消費者ゼロの検出は棚卸の定期監査項目。
- **1 ファイル 1,000 行**: 機械の番人 `crates/log-capture-kit/tests/file_length_guard_test.rs`（例外表 11 件・暗黙増加不可・**どの spec も例外表に触れない**）。2026-09-11 実測: `areka-emo-text/src/` の `draw.rs` 988・`layout.rs` 955・`actor.rs` 952・`region.rs` 951 が射程＝emo-text を触る spec は新規ファイルで足す。
- **決定論テスト網羅は必達**・**ログ無し失敗経路の禁止**・**終了経路は正規実装**（記憶 deterministic-test-coverage-mandate／areka-log-first-no-silent-failure／canonical-not-minimal-lifecycle）。
- **外部依存の追加は `tech.md` へ「意図的依存追加」を登記し開発者が承認する**（`encoding_rs` の前例）。α で登記したのは **`miniz_oxide` 0.9（`nar-install`・2026-09-18 承認済・伸長のみ・推移的依存は `adler2` 1 本）**、候補に残るのは `md-5`（`network-update`・承認待ち）。HTTP は WinHTTP（`windows` crate の機能フラグ）で crate を足さない。

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

## 完了サマリ（M1 全ウェーブ＋W13・詳報は completed spec ＋ history）

| Wave | 完了日 | ユニット（PR） | 一行要約 |
|---|---|---|---|
| 耐力壁 | 07-01 | `pilot-shiori-host-32` | x64→32bit pasta.dll 駆動 GO |
| M-boot | 07-13 | `emo2-boot` 23/23 | 起動→表示→talk→close の可視一周 |
| W1〜W4 | 〜07-29 | idle-talk／collision-geometry／sakura-dialogue-tags／input-events／mayuna-compose／sylphya／seriko-loop／choice-render／position-persist／choice-interact／emo-dpi-scaling 他 | 増分ウェーブ全完走・DPI 追従 k 着地 |
| W5 | 08-01 | `choice-select-events`・`kero-balloon`（#97）・`dpi-window-vanish`（#98） | M-dialogue 完走 |
| W6 | 08-10 | `file-slimming`（#103） | 1,000 行超 54→0 |
| W7 | 08-13 | `collision-dpi-hittest`（#100）・`balloon-visibility`（#106）・`bindoption-exclusivity`（#105）・`ghost-window-zorder`（#107）・`scope-chain-gap`（#108） | 5/5 |
| W8 | 08-15 | `recompose-budget`（#112）・`scale-exact-rational`（#110）・`windowposition-limit`（#111） | 1 コマ 22,210→1,240µs |
| W9 | 08-22 | `dpi-transition-atomicity`（#114） | 書込散らばり ms→µs |
| W10 | 08-27 | `draw-load-parity`（#118）・`test-cage-determinism`（#119） | 門の手がかり→`tick-gate-adoption`／log-capture-kit |
| W11 | 09-02 | `present-write-coherence`（#123）・`balloon-vertical-canon`（#124）・`balloon-offset-dpi`（#125）・`scope-zorder-pinning`（#126） | 4/4 |
| ⓪ | 09-03 | `sakura-bare-tag-lexer`（#134） | `\_X` bare 漏れ修正・完了検証で `sakura-tag-word-boundary` 起票 |
| W12 | 09-05〜09-11 | `cursor-tag-canon`（#137）・`emo-text-line-height-canon`（#142）・`emo2-conformance-e2e`（#143・**M1 完成宣言**） ∥ 調査系 `ukadoc-survey-toolkit`（#136）・`-property`（#138）・`-shiori`（#139）・`-sakura-script`（#140）・`-assets`（#141） | 行送り 35→30・e2e 20 項目全合格・ukadoc 1,749 項目の台帳 4 本＋`report/summary.md` |
| W13 | 09-12〜09-17 | `present-gpu-transform-scale`（#145）・`charset-canon`（#146）・`ukadoc-coverage-roadmap`（#147）・`text-decoration-canon`（#148）・`kanade-boot-talkdone-drop`（#149）・`sakura-tag-word-boundary`（#150）・`sylphya-set-ledger`（#151）・`host32-window-thread-pump`（#153）・`balloon-font-descript-keys`（#154） | 9/9・1 コマ 41〜78→2.65 ms・任意 charset・67 束の網羅ロードマップ・文字装飾 3 書字方向 |

- 完了 spec 直下エントリ＝**184**（`.kiro/specs/completed/` 直下・2026-09-17 実数え＝ディレクトリ 183＋`graphics-rendering-stability.md` 1）。⚠ **引き算で導かず毎回実数えする**（並走 spec が同じ行を更新する）。
- M1 実機サインオフ発見 7 件中 #1〜#6 解決済み・#7（冒頭空行）は pasta 上流。e2e の持ち越し（§13.1 行 1・§13.2 行 4・9・10）のうち行 4・行 10 は W13 で解決、行 1 は `dpi-transition-two-tick-bounce`、行 9 は `zorder-chain-residue` A-2。M-dual は退役（e2e 項目 10 で合格・復活させない）。

## spec 台帳（**表は 47 行**。うち **brief 済み 42 本**＝2026-09-13 の 30 ＋ 09-18 起票 8 ＋ 09-19 起票 3 ＋ 09-20 起票 1（#47 `balloon-break`）、**#38〜#42 の 5 行は brief を持たない「登記だけの行」**＝下の節を見よ。**状態列の実数え（2026-09-20・#47 の起票時に 1 行足しただけ＝他の行は 09-19 の数え直しのまま）＝完了 13・α 7・α 後 26・保留 1 ＝ 47**。着手は brief 持ちが `/kiro-start <名>`・登記だけの行は `/kiro-discovery` から）

> **段**列: **α**＝M2 のゴールに要る／**α 後**＝brief を保ったまま据え置く（着手は α 完了後の棚卸で並べ直す）。**α に関係しない spec は、並走できてもウェーブに入れない**（2026-09-18 開発者指示・「隙間」枠は廃止）。**規模**は brief の申告。**状態**列は `/kiro-complete` が ✅ に更新し、完了数を実数えで更新する。**Fable**列＝要件定義（design）を Fable で行うべきか（○＝Fable・−＝Opus で足りる）。α 後の行の Wave 列は棚卸⑬当時の番号（W14〜W17）を参考として残す＝**着手順の正本ではない**。

| # | spec（`areka-P0-` 省略） | 段 | 種別 | 規模 | Wave | 前提（先に着地） | Fable | 状態 |
|---|---|---|---|---|---|---|---|---|
| 1 | `completed/present-gpu-transform-scale` | ✅ | バグ（性能） | M | W13 | — | ○ | ✅ 完了（2026-09-12） |
| 2 | `completed/kanade-boot-talkdone-drop` | ✅ | バグ（構造） | S | W13 | — | − | ✅ 完了（2026-09-17） |
| 3 | `completed/host32-window-thread-pump` | ✅ | バグ（構造） | S〜M | W13 | — | − | ✅ 完了（2026-09-17） |
| 4 | `completed/sakura-tag-word-boundary` | ✅ | バグ | S〜M | W13 | — | − | ✅ 完了（2026-09-17） |
| 5 | `completed/charset-canon` | ✅ | 正典 | M | W13 | — | ○ | ✅ 完了（2026-09-13） |
| 6 | `completed/ukadoc-coverage-roadmap` | ✅ | 調査 | M | W13 | — | ○ | ✅ 完了（2026-09-13） |
| 7 | `completed/text-decoration-canon` | ✅ | 正典 | L | W13 | — | ○ | ✅ 完了（2026-09-13） |
| 8 | `completed/sylphya-set-ledger` | ✅ | 台帳 | S | W13 | — | − | ✅ 完了（2026-09-17） |
| 9 | `completed/balloon-font-descript-keys` | ✅ | 正典 | S | W13 | — | − | ✅ 完了（2026-09-17） |
| 11 | `completed/nar-install` | ✅ | 基盤（`.nar` の読取・`install.txt`・安全な展開＝`areka-nar`・検体を `.nar` 保管・**検体名 → 根の共有ヘルパ＝サンプルゴースト試験の仕組みの種**） | M | **A0-①**（**即時着手可**） | なし。38 ファイル（検体参照のある既存テスト・example）を書き換える＝**それらに触らない spec とは並走可**（09-18 実測で改訂） | ○ | ✅ 完了（2026-09-19） |
| 10 | `shell-implicit-surface`（09-13 起票） | **α** | バグ（**里々標準テンプレートが 1 枚も絵を出せない**・第三者の最初の 1 体が里々である前提で α の入口） | M | **A1-①** | `charset-canon` ✅・`present-gpu-transform-scale` ✅・#11（検体は共有ヘルパ経由で受ける＝09-18 に順序を反転） | ○（「element0 より下」の層表現と `surface.append` の順序の裁定 2 件） | ⚪ |
| 12 | `baseware-root-layout`（**09-18 起票**） | **α** | 基盤（根 `ghost/`・`balloon/`・列挙と素性・最後の選択の記憶・起動解決・「無い」告知） | S〜M | **A1-②** | #11（展開先の形＝根の形・共有ヘルパ） | −（裁定候補 1 件） | ⚪ |
| 37 | `default-balloon-bundle`（**09-18 起票・2 度目の再入**） | **α** | 資産（既定バルーンの選定・展開フォルダ `vendors/sample_ghost/StayseeBalloon/` で保管（**`.nar` への畳み込みは未了**＝#42 へ登記）・候補 CC0 `Balloon for Staysee Syncfield`・第三者告知・`use_self_alpha` 常時 1 の裁量登記） | S | **A0-③**（**即時着手可**） | なし。#11 と並走＝**既存の検体参照ファイルに触らず新規テストのみ**。id 定数 1 行は #12 が足す・`COMPAT_ARCHITECTURE.md` §8 は #10 と別節 | −（見た目の採否は開発者が実機で・**今日にでも argv で確認可**） | ✅ 完了（2026-09-19） |
| 46 | `completed/balloon-origin-outside-validrect` | **α** | バグ（互換。`origin` を validrect の外に宣言したバルーンで**行頭が 1 文字欠ける**。SSP では欠けない＝PR #124 の撤去の前提「SSP でも壊れた定義」が目視で反証された） | S | **A0-④**（**即時着手可**） | なし（#11 ✅・#37 ✅）。編集は `areka-emo-text/src/region.rs` の 1 関数＋兄弟テストの新設＋`COMPAT_ARCHITECTURE.md` §8 の 3 行＝**#11 の 38 ファイルに `region.rs` は含まれない（実測）** | −（裁定済みの復元・要件で決めるのは記録のレベルだけ） | ✅ 完了（2026-09-19）。**完了 spec `areka-P0-balloon-vertical-canon` 要件 3.10 を上書き**（3.11・3.7 は不変） |
| 13 | `ghost-shell-balloon-switch`（**09-18 起票**） | **α** | 正典（`\![change,ghost\|shell\|balloon]`・`OnGhostChanging/Changed`・`OnShellChanging/Changed`・`OnBalloonChange`・**アプリ寿命の分離**） | L（①寿命＋②バルーンを先行スライス可） | **A2**（単独） | #12（列挙・記憶）・`host32-window-thread-pump` ✅・`kanade-boot-talkdone-drop` ✅ | ○（α 最大の構造変更） | ⚪ |
| 14 | `completed/popup-menu-minimal`（**09-18 起票**） | ✅ | 製品（Win32 標準メニュー・項目 7 種・`*button.caption`・登記式） | M | **A0-②**（第 1 スライス＝説明書・終了・caption・visible・`MenuRegistry`）→ サブメニューは #12（列挙）・#13（`SwitchRequest`）が登記 | なし（**検体参照 0＝#11 と並走可**）。サブメニューの登記だけ #12・#13 の後 | −（裁定候補 2 件） | ✅ 完了（2026-09-19） |
| 15 | `ghost-install`（**09-18 起票**） | **α** | 製品（D&D `WM_DROPFILES`・ファイル選択・`\![execute,install,path]`・`OnInstall*`・`terms.txt`・`OnFileDrop2`） | M | **A4-①** | #11・#12・#13（`lastinstalled`）・#14（登記） | − | ⚪ |
| 16 | `network-update`（**09-18 起票**） | **α** | 製品（`homeurl`・`updates2.dau`/`updates.txt`・MD5・`delete.txt`・`OnUpdate*`・WinHTTP） | M | **A4-②**（#15 の後・直列） | #12・#14・#15（`InstallRequest`）・#13（読み直し） | ○（イベント列 51 件の Ref） | ⚪ |
| 17 | `alpha-release-signoff`（**09-18 起票**） | **α** | 完成の器（配布 zip・第三者の手順 11 項目・実機サインオフ・宣言） | S〜M | **A5** | #10〜#16・#37 全部 | − | ⚪ |
| 18 | `dpi-transition-two-tick-bounce` | α 後 | バグ（見た目・判定器・開発者裁定「拡大率の切替は頻繁に起こらないため許容」） | M | 旧 W14 | #1 ✅ | ○ | ⚪ |
| 19 | `zorder-chain-residue`（A 群＝間欠赤） | α 後 | 台帳（A 群はバグ。**A-2 の壁時計テスト族 4 本が A2 の切替テストで赤を出したら、その時点で A 群だけを単独枠に入れる**＝先回りしない） | M | 旧 W14 | #3 ✅ | ○（A 群のみ） | ⚪ |
| 20 | `surfaces-basepos` | α 後 | 正典（完全独立） | S | 旧 W14 | なし | − | ⚪ |
| 21 | `property-query-channels`（⑴ スクリプト経路 1〜4） | α 後 | 正典（プロパティ系の先頭ゲート） | M〜L | 旧 W14 | #4・#7・#2・#8 ✅ | ○ | ⚪ |
| 22 | `emo-text-canon-residue`（⑶・項目 12 は実バグ） | α 後 | 台帳（emo-text 残件 4） | M | 旧 W14 | #7・#9 ✅ | − | ⚪ |
| 23 | `balloon-canon-residue`（⑴ 系列 1〜6） | α 後 | 台帳（M2 互換面） | M | 旧 W14 | #1 ✅ | −（議題 2 件） | ⚪ |
| 24 | `property-ipc-transport`（channels ⑵） | α 後 | 正典（輸送路のライブ実測が前提） | M | 旧 W14 | #3・#5 ✅ | ○ | ⚪ |
| 25 | `translate-pipeline` | α 後 | 正典（→ makoto） | M | 旧 W15 | #2 ✅・#21 | ○ | ⚪ |
| 26 | `text-align-shadow-canon`（decoration ⑵） | α 後 | 正典（寄せ 2＋影 3＋SC8 裁定） | M | 旧 W15 | #7 ✅・#22 | ○ | ⚪ |
| 27 | `status-execution-states` | α 後 | 台帳 | S | 旧 W15 | #21 | − | ⚪ |
| 28 | `currentghost-property-tree`（`seriko.*` から `zorder` 除外） | α 後 | 正典（値の木） | L（19 項スライス可） | 旧 W15 | #21・#8 ✅ | −（議題 2 件） | ⚪ |
| 29 | `zorder-property` | α 後 | 正典（値の導出） | S | 旧 W15 | #8 ✅・完了 zsp | − | ⚪ |
| 30 | `makoto-dll-host` | α 後 | 正典（MAKOTO/2.0） | L（ⓐ loadu は XS 先行可） | 旧 W16 | #5 ✅・#25・#24・#3 ✅ | ○ | ⚪ |
| 31 | `sakura-time-directives` | α 後 | 正典（allowlist 8 族） | L（A/B/D → C） | 旧 W16 | #7 ✅・#21・#25 | ○（C 群のみ） | ⚪ |
| 32 | `property-catalog-lists` | α 後 | 正典（島単位・`ghostlist`／`balloonlist` の供給源は #12 の列挙） | M | 旧 W16 | #28・**#12** | − | ⚪ |
| 33 | `choice-marker-styling` | α 後 | 正典（`\f[cursor*]` 10） | S | 旧 W16 | #7 ✅・#26 | − | ⚪ |
| 34 | `anchor-tag-canon` | α 後 | 正典（`\_a`＋装飾 16） | M | 旧 W17 | #7 ✅・#26・#33 | ○ | ⚪ |
| 35 | `balloon-lifecycle-events`（residue ⑵） | α 後 | 正典（表示寿命 7〜10） | M | 旧 W17 | #7 ✅・#31・#25 | −（裁定 2 件） | ⚪ |
| 36 | `tick-gate-adoption` | 保留 | 性能（既定 OFF の門の採否） | M〜L | 保留 | 「長時間試行禁止」と両立する A/B 設計を要件で先に組む（単独） | ○ | ⚪ |
| 38 | `ukadoc-coverage-custody`（仮称・**未起票**＝brief なし・2026-09-19 登記） | α 後 | 調査の番人と統合担当の建て直し（下の節「登記だけの行」を見よ） | S〜M | α 後 | なし（着手は α 完了後の棚卸で just-in-time） | − | ⚪ |
| 39 | `present-write-coherence` の未達 40 件（仮称・**未起票**＝brief なし・2026-09-19 登記） | α 後 | 性能（完了仕様が「引受先なし・新規仕様の起票が必要」と自ら書いた残量） | L | α 後 | なし。着手前に当時の実測を採り直す | ○ | ⚪ |
| 40 | 正典語彙の孤児 2 件（仮称・**未起票**＝brief なし・2026-09-19 登記） | α 後 | 正典（`font.outline` の白抜き**描画**／「スタイルシートのキーワード」の所有） | S | α 後 | なし | − | ⚪ |
| 41 | 配布物を束ねる／作る側の 3 件（仮称・**未起票**＝brief なし・2026-09-19 登記） | α 後 | 正典（`type,package` を解く・`developer_options.txt` を読む側＝配布物を**作る**道具） | XS〜S | α 後 | なし | − | ⚪ |
| 42 | 既定バルーンを `.nar` へ畳む（仮称・**未起票**＝brief なし・2026-09-19 登記） | **α** | 資産（`vendors/sample_ghost/StayseeBalloon/` が展開フォルダのまま残っている＝`#37` が `#11` より先に着地したため、どちらの申し送りも実行されなかった。畳んで登記表 `SAMPLES` に 1 行足し、直書きの検体数 2 か所を 1 つ増やす（09-19 に `konnoyayame` が先に入ったので今は 5→6。既知の名前の一覧 2 か所にも足す）） | XS | **α・A1 の前に**（着手は `/kiro-discovery` から。手順は `vendors/sample_ghost/README.md`） | #11 ✅・#37 ✅（両方着地済み＝いつでも着手可） | − | ⚪ |
| 43 | `wintf-drag-state-rest-contract`（**09-19 起票**） | α 後 | バグ（構造・潜在＝wintf のドラッグ状態は `JustEnded` で休み続けるのに説明は「1 フレームのみ」・`reset_to_idle` の呼び手が製品に無い。#14 の実機確認でメニューが出なくなる実害 1 件・利用側は是正済み） | S | α 後（単独・いつでも） | #14（`menu/trigger.rs` が main に在ること） | −（直し方 2 案の裁定 1 件） | ⚪ |
| 44 | `popup-menu-residue`（**09-19 起票**） | α 後 | 台帳（#14 の最終検証の残件 10＝別窓の預かりの誤配・台本の `\![open,readme]` の記録の重さ・文言と接頭辞・テストの穴 2・引受先の無い語彙 4 件） | S | α 後（`menu/` に触る次の spec へ相乗り可） | #14 | − | ⚪ |
| 45 | `coverage-roadmap-refresh`（**09-19 起票**） | α 後 | 台帳（`doc/ukadoc-coverage/` の検査の外にある手書きの数の棚卸と撮り直し・候補名 `areka-P0-network-update` の重なり・波の欄の旧編成・各 spec の「統合担当への申し送り」の受け皿。**#38（登記だけの行）と守備範囲が重なる**＝合流の可否は α 完成宣言のあとの棚卸で決める） | S〜M | α 後（A0 の 3 本が main へ入った直後が最も安い） | #11・#14・#37 | − | ⚪ |
| 47 | `balloon-break`（**09-20 起票**・開発者指示） | α 後 | 互換（バルーンの左ダブルクリックで再生中の台本を止め、全バルーンを即時に隠す。ukadoc に操作の定義は無いが `\![enter,nouserbreakmode]`・`\t`・`OnBalloonBreak` が存在を前提にしている。`nouserbreakmode` の無効化を含む。`OnBalloonBreak` の発火は #35 に残す。**09-20 要件裁定で範囲が 1 つ増えた**＝`\-` を含む台本の中断は終了する・終了イベントの別れの台詞は `\-` の有無によらず必ず終了で終わる〔完了 `areka-P0-kanade` 要件 4.5 と `areka-P0-popup-menu-minimal` 要件 5.4 を上書き・終了の握手が定常へ戻る経路は 0 本になる・`kanade/schedule/close.rs` の終了拒否の腕とテスト 3 本（`close.rs` 2 本＋`close_test_handshake_tests.rs` 1 本）が変わる〕） | M | **開発者指示で 09-20 着手**（α のゴールには要らないが、指示が段の規則に優先する） | なし（先行必須の未完 spec は無い。`kanade/schedule/*` を触る #35・`translate-pipeline`・`sakura-time-directives` とは並走させない） | −（裁定 6 件・いずれも利用者から見える結果で語れる） | ⚪ |

## 登記だけの行（#38〜#42・2026-09-19・brief なし）

> **なぜ起票しないのか**: どれも α（第三者がデスクトップマスコットを管理できる）に関係しない。開発者方針「α に関係しない spec は、並走できても
> ウェーブに入れない」と「spec 工場禁止・起票は just-in-time」に従い、**行だけ立てて brief は書かない**。引受先が消えたまま忘れられることだけを防ぐ。
> 着手の判断は α 完成宣言のあとの棚卸で行う。**本節の数はすべて 2026-09-19 に数え直した実測で、数え方を併記してある。**

- **#38 `ukadoc-coverage-custody`（仮称）＝網羅台帳の番人と統合担当の建て直し**。いま起きていることは 4 つある。
  ⑴ **整合検査が「完了した spec が引受先になっている行」を検出できない。** 判定 ⑸ の腕 f は「台帳の非空の宛先がすべて `roadmap-draft.md` の `[[spec]]` か `briefing.md` の `[[owner_completed]]` に在ること」しか見ない。`[[owner_completed]]` は**手書きで凍結した 14 名の一覧**（数え方: `grep -c '^\[\[owner_completed\]\]' doc/ukadoc-coverage/briefing.md ＝ 14）で、`completed/` を走査しない。spec が完了しても `[[spec]]` の行はそのまま残るので、腕 f は緑のまま通る。実際に `completed/` に在る areka-P0- 仕様は **82 本**（数え方: `find .kiro/specs/completed -maxdepth 1 -type d -name 'areka-P0-*' | wc -l`。同ディレクトリの全ディレクトリは 183・areka-P0- 以外が 101）。
  ⑵ **その穴を 3 つの所見が通り抜けている。** ⓐ 台帳 4 本の宛先に現れる仕様のうち **18 本**が既に `completed/` に在り、そのうち **4 本**（`text-decoration-canon` 16 行・`balloon-font-descript-keys` 7 行・`charset-canon` 5 行・`sylphya-set-ledger` 1 行＝**計 29 行**）は `[[owner_completed]]` の 14 名に入っていない。ⓑ そのうち**状態が `vocabulary-only` のまま＝もう誰も仕上げられない行が 5 行**ある（`text-decoration-canon` 3・`balloon-font-descript-keys` 1・`sylphya-set-ledger` 1。数え方: 台帳 4 本の `status` と `owner` の行を対にして数え、`owner` が `completed/` に在る仕様の行だけを残した）。ⓒ `roadmap-draft.md` の `[[spec]]` **8 行**が、完了済みの仕様を未完として載せている（`text-decoration-canon`・`charset-canon`・`balloon-font-descript-keys`・`sylphya-set-ledger`・`sakura-tag-word-boundary`・`present-gpu-transform-scale`・`kanade-boot-talkdone-drop`・`host32-window-thread-pump`）。
  ⑶ **台帳 107 項目が「一度も起票されたことのない仕様名」を縮退の根拠に引いている。** `areka-P0-seriko-runtime`・`areka-P0-balloon-loader`・`areka-P0-shiori-host-32` の 3 つで、いずれも `.kiro/specs/` の直下にも `completed/` にも無い（旧ロードマップの名前）。数え方: 台帳 4 本を `[entry.` で区切り、3 つの綴りのどれかを含む区画を数えた＝`assets.toml` 105・`shiori.toml` 2・残り 2 本は 0。
  ⑷ **統合担当が不在のまま指示だけが残っている。** `doc/ukadoc-coverage/README.md` は「跨いだ関連を書いたら、そのことを統合担当（`ukadoc-coverage-roadmap`）にも申し送ること」と今も書いているが、その仕様は 2026-09-13 に完了して `completed/` に在る。今後どの仕様が台帳を触っても、不在の相手へ申し送ることになる。さらにその成果物のはずだった**「無所有一覧」が作られないまま担当が消えた**ため、`\_` の仲間で角括弧を持たないタグ **8 件**（`\_!`・`\_+`・`\_?`・`\__c`・`\__q`・`\__t`・`\__v`・`\_n`＝`briefing-sakura-script.md`「引受先の候補」の表がこの一覧での裁定を待つと書き、台帳の宛先は 8 件とも空）、さくらスクリプトの台帳で宛先が空の **263 項目**（同台帳の全 342 項目のうち）、および段階が開発者の裁定待ちのまま仮値で置かれた **11 束**（`README.md`「段階（A〜E）は統合担当が付け、台帳へ書き戻してある」）が宙に浮いている。
- **#39 `present-write-coherence` の未達 40 件**。完了仕様 `.kiro/specs/completed/areka-P0-present-write-coherence/requirements.md`「未達の登記」の節が自ら「**引受先なし**……将来是正する場合は**新規仕様の起票が必要**」と書いている。残量は `visualize_to_write_us` が実測 **210,329〜306,301µs** に対して上限 16,667µs＝**12.6〜18.4 倍**で、上限以下で終わった窓は **32 窓中 0**。これに `flush_total_us` の 8 件を足して**計 40 件**。開発者裁定で「未達のまま GO」済みだが、**この残量はいま `roadmap-history.md` にしか無く、生きた本ファイルからは見えない**（本ファイルの完了サマリは W11 を「4/4」と完了扱いする）。追う仕様が 1 本も無い状態を、この行で可視にしておく。
- **#40 正典語彙の孤児 2 件**。ⓐ `font.outline`（白抜き）は受け口まで配線済みだが**描画の引受先が未起票**——台帳 `assets.toml` の `descript_balloon:font.outline` の備考が「受け口は白抜きを状態だけ更新して表示は変えない……白抜きの描画の引受先は未起票」と書き、名指しの `text-decoration-canon` は語彙登記で打ち止めかつ `completed/` に在る（さくらスクリプト側の `\f[outline,…]` も同じく `vocabulary-only`・宛先は同じ完了仕様）。ⓑ 「スタイルシートのキーワード」は**所有仕様が 0 本**——`completed/areka-P0-text-decoration-canon/tasks.md` の裁定 9.3 が「`completed/` を含む全 spec を走査して確認」と書いており、追跡先は `doc/COMPAT_ARCHITECTURE.md` §8 の行と台帳の備考だけである。
- **#41 配布物を束ねる／作る側の 3 件**。台帳 `assets.toml` の `descript_install` の 3 項目が、備考で自ら「引受先がまだ起票されていない」と書いている。内訳は ⓐ `type,package`（複数の配布物を 1 つに束ねた形）を解く仕様 1 件——`areka-nar` は種別を `ghost`・`shell`・`balloon`・`supplement` の 4 つに限り `package` を明示的に拒否する、ⓑ `developer_options.txt` の行の書式 2 件——読む相手はベースウェアではなく**配布物を作る道具**なので、areka が作る側の道具を持ったときにその仕様が引き受ける。いずれも α の外（`ghost-install` の Out・`network-update` の Out に明記）。
- **#38 と #43〜#45 の関係（2026-09-19 の合流メモ）**: #38 `ukadoc-coverage-custody`（登記だけ）と #45 `coverage-roadmap-refresh`（brief あり）は、どちらも「網羅台帳の手書きの数と引受先が黙って偽になる」問題を見ている。#45 は数の棚卸と検査の追加、#38 は整合検査の穴（完了 spec が引受先のまま・未起票の仕様名を根拠に引く 107 項目・統合担当の不在）で、重なるが同一ではない。**合流の可否はここで決めず、α 完成宣言のあとの棚卸で一括して裁定する**（`/kiro-start areka-P0-coverage-roadmap-refresh` を先に回す場合は、要件段階で #38 の 4 件を取り込むかを最初の議題にすること）。

**規模と分割の裁定（棚卸⑬・据え置き）**: XL 3 本を分割済み＝`property-query-channels` → ⑴本体／⑵`property-ipc-transport`／⑶`sylphya-set-ledger`、`balloon-canon-residue` → ⑴本体／⑵`balloon-lifecycle-events`／⑶`emo-text-canon-residue`、`text-decoration-canon` → ⑴本体／⑵`text-align-shadow-canon`／⑶`balloon-font-descript-keys`。**L のまま置く 3 本**（`makoto-dll-host`・`sakura-time-directives`・`currentghost-property-tree`）と α の `ghost-shell-balloon-switch` は brief 内のスライスで要件段階に縮める余地を残す。


**#42 既定バルーンを `.nar` へ畳む（α・XS・A1 の前に）**

`vendors/sample_ghost/StayseeBalloon/` が**展開フォルダのまま残っている**。#37 が #11 より先にマージされた結果、
#37 の brief は「`.nar` 化は #11 が引き受ける」と書いて完了し、#11 の brief は「畳み込みは #37 が行う」と書いて
完了した——**双方が相手を指したまま両方とも着地し、誰も畳まなかった**。#11 の要件 8.1 と
`vendors/sample_ghost/README.md` が定める「この場所に置くもの」に反する状態である。

やること 3 つ（同じコミットで）: ⑴ `cargo run -p sample-ghost-kit --example fold-samples -- --from vendors/sample_ghost/StayseeBalloon`
で畳む（引数無しの形は使えない）・⑵ 登記表 `SAMPLES` に 1 行足す・⑶ `crates/sample-ghost-kit/src/lib_tests.rs` が
直書きしている検体数 **2 か所を 1 つ増やす**（09-19 に `konnoyayame` が先に入ったので今は 5 → 6。`unknown_sample_fails_with_all_five_known_names` の名前の一覧 2 か所と関数名の数も直す）（`every_registered_sample_lands_where_its_registry_row_says` と
`every_sample_nar_installs_exactly_the_elements_its_registry_row_declares`。母数 0 で緑にならないための較正なので
消さずに数だけ直す）。⑷ 展開形を追跡から外す。手順の正本は `vendors/sample_ghost/README.md`。

**#37 は完了済みなので先送りを吸収できない**（記憶 deferral-requires-verified-owner）。ゆえにここへ登記した。
## 棚卸⑭の仮裁定（2026-09-18・開発者が覆すまで有効）

1. **M2 のゴールは α（第三者が管理できる）であり、表現力の増強ではない。** 段階 A の先頭ウェーブ 6 束（`roadmap-draft.md`）と α 後 19 本は brief を保ったまま据え置く。α 完了後の棚卸で並べ直す（`briefing.md` の順位付けはそのとき再び効く）。
2. **旧・仮裁定 2「M2 予約群は brief を起票しない」を改訂**: 予約群のうち **NAR（製品側）・ネットワーク更新・ゴースト/バルーン選択 UI（メニューのサブメニューとして）** は α に入るので起票した（本日 6 本）。残り（SSTP・FMO・DirectSSTP・Plugin/HEADLINE・多重ゴースト・pasta native x64・ベクトル描画・**owner-draw メニュー**・バルーン美観配置・回転テキスト）は引き続き起票しない。
3. **メニューは Win32 標準（`HMENU`）**。オーナードロー（`menu_*.png`・`menu.font.*`）は M2 予約のまま。着せ替えメニュー（`sakura.menuitem*`）も α 後。
4. **ゴースト切替はプロセス内**（再起動しない）。アプリの寿命を窓の数から切り離す（`AppExit`）。
5. **HTTP は WinHTTP・MD5 は `md-5` 1 本**（承認待ち・`tech.md` 登記は `network-update` の要件で）。**`nar-install` の伸長は `zip` を採らず `miniz_oxide` で決着**（2026-09-18 承認済・`tech.md` 登記済）。
6. **消滅（`\![vanishbymyself]`・`OnVanish*`）は α に含めない**（アンインストールはフォルダ削除で足りる）。要望が出たら `ghost-shell-balloon-switch` の隣に S で切る。
7. **投げ込みは `WM_DROPFILES`**（`IDropTarget` は OLE の STA を要求し WUC の MTA と衝突しうる）。テキスト・URL の投げ込みは α 後。
8. **開発者への裁定候補は各 brief の末尾に登記**: ~~⑴ 既定バルーンの同梱（`emo2-kakukaku` を推す）~~ → **同日 2 度目の再入で裁定・spec 化**（下 10）／⑵ 根の既定は exe の隣（推す）／⑶ 右クリックで `OnMouseClick` も送るか（送らないを推す）／⑷ トレイアイコン（含めないを推す）／⑸ 更新定義ファイルの既定 charset（Shift_JIS 固定を推す）。残る 4 件は推奨で進めて構わない。
9. 棚卸⑬の仮裁定 1（`zorder` の三重所有）と 3（e2e §13.1 行 3 の据え置き）は据え置き。
10. **既定バルーン（2026-09-18 開発者裁定）**: `emo2-kakukaku` は癖が強く既定に向かない。SSP 同梱「SSPデフォルト+」は再配布条件が公開されておらず借用の根拠が薄い。候補は SSP 本家の作者が **CC0** で出している `Balloon for Staysee Syncfield`（readme「煮るなり焼くなり好きにしてください」・専用指定なし）。**areka は常に `use_self_alpha,1`・`.pna` 非対応**（開発者確認・`areka-emo-present/src/balloon.rs` の設計と一致）。独立 spec `default-balloon-bundle`（#37・A2 並走）で扱い、見た目の採否は同 spec の要件段階で開発者が実機で決める。次善は自作の無地バルーン。
11. **α に関係しない spec は、並走できてもウェーブに入れない**（2026-09-18 開発者指示）。「隙間」枠を廃止し、`dpi-transition-two-tick-bounce`・`zorder-chain-residue` を α 後へ戻した。例外は 1 つだけ＝α の spec のテストを実際に赤にした間欠赤（zorder A-2 の壁時計テスト族）は、その時点で A 群だけを単独枠に挟む（先回りしない）。
12. **nar-install を α の先頭に置く**（2026-09-18 開発者「nar 関係は早く進めないとダメ」）。`shell-implicit-surface` との順序を反転＝implicit-surface は共有ヘルパを最初から使う。**サンプルゴーストを増やして試験する仕組み**は、nar-install の共有ヘルパ（検体名 → 根）と `vendors/sample_ghost/*.nar` の保管慣行がそのまま器になる＝別 spec は切らない（検体を 1 体足す作業は `.nar` 1 つと名前 1 行）。α の検証に使う検体は emo2（pasta）・R_POST_and_KOMAINU（里々）・既定バルーンの 3 つ。~~YAYA の検体を α に足すかは `alpha-release-signoff` の要件段階で決める（裁定候補 ⑹）~~ → **2026-09-19 決着＝足す**（開発者提案）。YAYA 標準テンプレート「紺野ややめ」を `vendors/sample_ghost/konnoyayame.nar` として登記済み（配布物そのまま・シェルは CC BY-NC-ND 2.1 JP＝畳み直さない・areka の配布物へ同梱しない。出どころは同フォルダの README）。**確かめたのは「正しく展開される」までで、areka 上の実走は未実施**＝出た不具合は個別に起票する。

## ウェーブ編成（着手順の正本・2026-09-18 棚卸⑭・α）

> 各ウェーブは**フルライフサイクル**（要件→設計→タスク→実装→`/kiro-complete`＝PR squash マージ）を完走してから次へ。並走はウェーブ内のみ（1 spec = 1 worktree = 1 PR）。同居は**実測で共有ファイル 0**が原則（記憶 prefer-clean-waves-over-max-parallelism）。α は**ほぼ直列**である——切替・根・メニューが `main.rs`／`areka-ghost/src/runtime.rs`／`kanade/schedule/*` を順に触るため、並走させると後着が毎回 rebase する。文書フェーズ（要件・設計）は先行可＝先行 spec はウェーブ開始時に settled main へ再突合。

| Wave | ユニット（優先順） | 開始コマンド | 編成根拠・条件 |
|---|---|---|---|
| W1〜W13 ✅ | 完了サマリ参照 | — | 旧行全文は history |
| **A0**（4 本・並走。**4 本とも 2026-09-19 に完了＝A0 は閉じた**） | ① ~~`nar-install`~~ ✅ ∥ ② ~~`popup-menu-minimal`~~ ✅ ∥ ③ ~~`default-balloon-bundle`~~ ✅ ∥ ④ ~~`balloon-origin-outside-validrect`~~ ✅（#46） | `/kiro-start areka-P0-nar-install` ／ `/kiro-start areka-P0-popup-menu-minimal` ／ `/kiro-start areka-P0-default-balloon-bundle` ／ `/kiro-start areka-P0-balloon-origin-outside-validrect` | ① **α の先頭。** 38 ファイル・9 クレートの検体参照を 1 つの共有ヘルパへ寄せる段を含む。**「並走不可」は 2026-09-18 に実測で改訂**＝書き換えるのは検体パスを参照する既存のテスト・example だけ（ディレクトリ別: `emo2_boot` 6・`areka-emo-text/tests` 5・`placement` 4 …）なので、**検体参照のある既存ファイルに触らず新規ファイルだけ足す spec とは共有 0**。展開先の形はベースウェアの根に揃える。共有ヘルパは「検体名 → 根」＝サンプルゴースト試験の仕組みの種。依存承認は **`miniz_oxide`（伸長のみ・2026-09-18 承認済。`zip` は採らなかった）**。② は `input_events/`・wintf のポインタ経路・新規 `menu.rs`＝**検体参照 0（実測）**。A0 で着地させるのは**第 1 スライス＝右クリック→Win32 メニュー・「説明書」「終了」・`*button.caption`・`popupmenu.visible`・登記式 `MenuRegistry`**。ゴースト／シェル／バルーンのサブメニューは列挙（A1-②）と `SwitchRequest`（A2）が着地した時に登記で足す（設計はその口を A0 で切る）。③ は**展開フォルダのまま着地した**（2026-09-19・① より先にマージされたため）。`.nar` への畳み込みと登記表 `SAMPLES` への 1 行は**未了**＝#42 へ登記した。表示検証は**新規テストファイルのみ**（`emo2_boot/*` の既存テストを触らない＝① と共有 0）。`THIRD-PARTY-NOTICES.md` は `cargo about` の自動生成で CC0 のバルーンは載らない＝資産の告知は README（`alpha-release-signoff`）側。**見た目の採否は今日にでも可能**＝フォルダを置いて `areka.exe <ghost> <balloon>` の argv 第 2 引数で起動すれば済む（コード変更 0） 。④ は `areka-emo-text/src/region.rs` の開始点の解決 1 関数と兄弟テストだけ＝①〜③ と共有 0（`region.rs` は検体パスを参照しない・実測）。`COMPAT_ARCHITECTURE.md` §8 は行の書き換えなので、同表へ行を足した ③ とは本ブランチ（後着）が取り込んだ |
| **A1**（2 本・並走） | ① `shell-implicit-surface` ∥ ② `baseware-root-layout` | `/kiro-start areka-P0-shell-implicit-surface` ／ `/kiro-start areka-P0-baseware-root-layout` | いずれも A0-① の共有ヘルパを前提。① は `areka-emo-compose`／`emo2_boot/assets.rs`／`placement/measure.rs`／`areka-parsers/src/shell/*`。② は `boot_config.rs`／`main.rs`／`areka-ghost/src/runtime.rs`／`areka-sylphya/src/persist/*`＋既定バルーン id の定数（A0-③ の成果物）＋メニューへの列挙サブメニューの登記（A0-② の `MenuRegistry`）。⚠ 同 crate 別ファイル ①⇄②＝`crates/areka/src/`＝後着 rebase。着手時に実測で確かめ、共有が出れば直列へ。② の裁定 ⑵ |
| **A2**（単独） | `ghost-shell-balloon-switch` | `/kiro-start areka-P0-ghost-shell-balloon-switch` | α 最大の構造変更（アプリ寿命の分離）。`main.rs`／`runtime.rs`／`kanade/schedule/*`／emo の再ロード。要件で ①寿命＋②バルーンを先行スライスにしてよい。実機は emo2 ⇄ R_POST_and_KOMAINU の往復。`zorder-chain-residue` A-2 の壁時計テスト族が本ウェーブの切替テストで赤を出したら、その時点で A 群だけを単独枠に挟む（先回りしない） |
| **A3** | （空き＝`popup-menu-minimal` は A0-② へ前倒し。A2 の切替が着地した時点でメニューのサブメニュー登記は A2 の成果物として同時に済む） | — | 段の番号は据え置き（A4・A5 の名前を動かさない） |
| **A4**（直列 2 本） | ① `ghost-install` → ② `network-update` | `/kiro-start areka-P0-ghost-install` → `/kiro-start areka-P0-network-update` | 両方が `MenuRegistry` の登記と `kanade/schedule/*` に相を足す＝**同時に走らせず直列**。② は ① の `InstallRequest` を使う（`\![execute,install,url]`）。`md-5` の依存承認・`Win32_Networking_WinHttp` の機能フラグ。裁定 ⑸ |
| **A5** | `alpha-release-signoff` | `/kiro-start areka-P0-alpha-release-signoff` | 配布 zip（PowerShell スクリプト 1 本）＋第三者の手順 11 項目の実機一周＋開発者の署名。**α 完成宣言の器** |
| **α 後** | 台帳 #20〜#35（旧 W14〜W17） | — | 旧ウェーブ行と干渉台帳は history「2026-09-18 棚卸⑭退避」節。α 完了後の棚卸で `briefing.md` の順位に沿って並べ直す |
| **保留** | `tick-gate-adoption` | — | 夜間/25 分/n≥3 の実測要求が開発者方針「長時間試行禁止」と正面衝突＝要件段階で「始める前に決着可能な A/B 設計」を組めた時点で単独ウェーブへ。他 spec と並走しない（計測を汚す） |

**干渉台帳（α・2026-09-18・brief の申告に基づく＝着手時に実測で更新）**:
- **輻輳点**: `crates/areka/src/main.rs`＝A1-② → A2／`crates/areka-ghost/src/runtime.rs`＝A1-② → A2／`crates/areka-kanade/src/schedule/*`＝A2（切替の握手） → A4-①（インストール相） → A4-②（更新相）／`crates/areka/src/input_events/mod.rs`＝A3（右クリック 1 分岐）・A4-①（`WM_DROPFILES` は wintf の窓手続き側）／`MenuRegistry`（A3 の新規ファイル）＝A4-① → A4-②／`crates/areka-sylphya/src/persist/mod.rs`＝A1-②（鍵族 1 つ）／`crates/areka-nar`（A0 新設）＝A4-①・A4-② が呼ぶだけ（触らない）／共有ヘルパ（A0 新設・検体名 → 根）＝A1 の 3 本が呼ぶだけ（③ は検体名を 1 つ登記＝1 行）。
- **α 後との交差**: `runtime.rs`／`prop_sink.rs`（`property-query-channels`・`property-ipc-transport`）・`kanade/schedule/*`（`translate-pipeline`・`sakura-time-directives`・`balloon-lifecycle-events`）・`emo2_boot/mod.rs`（`status-execution-states`）は α 後が後着＝**α 後の brief の file:line は α 完了時に全数再測定**（棚卸⑬と同じ手順・サブエージェント）。
- **保存義務**: A3 は既存の終了経路（終了指示 `CloseRequest` → `OnClose` 握手 → `ghost_quit`。入口は 2026-09-19 から右クリックメニューの「終了」＝`areka-P0-popup-menu-minimal` が結線済みの Ctrl＋左ダブルクリックの入口を取り除いた）の決定論テストを 1 本も落とさず、入口（`AppExit`）を 1 つ増やすだけにする。A2 は実機サインオフの「絶対パス起動」（argv 上書き）を残す。

## 直接修正候補（spec なし・任意・S）

- `crates/areka-emo-text/src/writing.rs` の未知 `writing_mode` 警告文言（「horizontal_tb へフォールバック」→ 実挙動は「指定なしとして扱う」）。逐語固定のインラインテスト `unknown_value_falls_back_to_horizontal_tb_with_warn` と同時修正。`emo-text-canon-residue` 項目 12 と同一＝先に直せば同 spec から外す。
- `.kiro/steering/roadmap-history.md` の `\w[ms]` 記述（history は非改変ゆえ据え置き・`doc/ukadoc-coverage/briefing-sakura-script.md` 側は 2026-09-11 に注記を追加済み）。
- `crates/areka/src/main.rs:167-177` の「ゴーストの根が実在しなくても `warn!` で起動を続ける」経路（記憶 areka-log-first-no-silent-failure に反する）＝`baseware-root-layout` が置き換えるので**先に直さない**（二度触らない）。

## 着手手順

- **brief 全数完備体制**: brief 済み spec **38 本**全てに brief あり。**うち完了 9・α 10・α 後 19**（状態列の実数え・2026-09-18）＝着手は該当 brief を読んで `/kiro-start <unit>` へ直行。brief の file:line は起票時値＝**着手時に必ず再検証**（棚卸⑬の再測定で実体の消失は 0 件・行番号ドリフトは全 brief に常在。09-18 起票の 6 本はサブエージェント探索の実測値）。
- 新規課題の起票は `/kiro-discovery`（再入）で just-in-time。`/kiro-spec-batch` は使わない（一括＝工場化）。ウェーブ跨ぎの合流判断は別セッションで一括（記憶 portfolio-convergence-decided-in-separate-session）。
- **要件定義・設計のサブエージェントは Fable**（上表 Fable 列 ○）・タスク生成と実装は Opus（記憶 requirements-design-need-fable-grade-review／fable-main-opus-subagents-token-policy）。
- **ukadoc 台帳の `owner`**: α の 6 本は起票時点で台帳（`doc/ukadoc-coverage/ledger/`）の `owner` に登記していない。各 spec の要件段階で登記し、同時に `roadmap-draft.md` の `owner_count` を追随させる（記憶 sylphya-set-ledger の教訓＝担当欄を埋めたら同時に追随）。

## 制約

- Rust 2024・マルチクレート（一覧は structure.md）。**32bit 可搬性の適用範囲＝host-32 系（`shiori-host32-*`／`shiori-abi`）のみ**。wintf/areka 本体は x64＋arm64 ネイティブ。
- 透過は WUC/DComp GPU 合成上のクリックスルー機構（`WS_EX_TRANSPARENT` 動的トグル＋αマスク）で成立（ULW は撤去済み）。SHIORI 内部唯一 ABI=`IShiori`(COM, HSTRING/UTF-16)。過去互換は 32bit Rust ホスト。
- 設計判断の変更は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md) を正本として更新。
- 実機運転の定石: 起動に渡す**検体の絶対パスは `cargo run -p sample-ghost-kit --bin nar-sample-path -- emo2` が印字する**（`folder=` がゴースト・`balloon.<名>=` が同梱バルーン。呼ぶたびに `manual/<検体>/` を作り直すので、**2 つの端末で同時に呼ぶと互いの木を消す**＝1 度印字してから使う）。絶対パス起動（相対は pasta.dll LOAD 失敗）・i686 helper を先ビルド・`AREKA_APP_SMOKE_EXIT_MS` 有界自動終了＋`RUST_LOG` grep（記憶 areka-real-machine-signoff-bounded-auto-exit）。自動終了は強制終了の経路で終了挨拶を経ないので、終了挨拶を確かめる走行では自動終了を上限に留め、終了はキャラ窓の右クリックメニューの「終了」で求める（2026-09-17 host32-window-thread-pump の裁定は Ctrl＋左ダブルクリックだったが、2026-09-19 に `areka-P0-popup-menu-minimal` がその入口を取り除いた。強制退避の Ctrl＋Shift＋左ダブルクリックは残る）。
- 常時テストは x86 を避け偽境界で純 x64 決定論（記憶 prefer-x64-fake-boundary-tests-not-x86）。**ネットへ出るテストを常時テストに入れない**（`network-update` は偽 `HttpFetch`）。

## α 後（M2 の残りと M3）

**α 完成宣言（`alpha-release-signoff`）を起点に組み直す。** 材料＝`doc/ukadoc-coverage/`（段階 A の先頭ウェーブ 6 束・324 件が `roadmap-draft.md` にそのまま残っている）＋ α 後 19 本の brief。順位付けは `briefing.md` の 4 軸（壊れ方＞伺からしさ＞資産の広さ＞基盤共有度）へ戻す。M3「伺かの冠」の受入基準の候補は `roadmap-draft.md`「M3 の受入基準の候補」。

予約（全て任意・brief なし・仮裁定 2 改訂版）: アプリ層＝SSTP（9801）・FMO・DirectSSTP・Plugin/HEADLINE・多重ゴースト・**トレイアイコン**（裁定 ⑷）・**消滅**（裁定 6）・設定画面と設定ファイル・`\![update,platform]`（本体の更新）・`\![execute,createnar]`／`createupdatedata`（作る側＝開発者機能）。互換面＝SAORI は実装しない（SHIORI が直接 `LoadLibrary`・台帳 `not-applicable`）・里々/YAYA 網羅。emo テキスト進化＝回転テキスト（`TextEffects` 予約名 `rotation`／`multicolor`）。**`\f[sub]`／`\f[sup]`／`\f[outline]` は DirectWrite の標準機能で表せる手段が見つかるまで語彙のみ**（2026-09-11 裁定・台帳 `vocabulary-only`・追跡先はこの行）。バルーン美観配置。pasta の native x64／`IShiori` in-proc・ベクトル描画・**owner-draw 右クリックメニュー**（裁定 3・`popup-menu-minimal` の構造の上に見た目を被せる）・着せ替えメニュー。

---

**追記台帳（要約・全文は history）**: (51)〜(94) は棚卸⑬までに全文退避（history 参照）。

**2026-09-11 追記(95)（棚卸⑬＝M1 完成宣言後の全面再編・roadmap 減量・XL 3 本の分割・三重所有の仮裁定・W13〜W17 の整数振り直し）**: `/kiro-discovery` 再入。①brief 欠落 0。②22 brief をサブエージェント 3 体で全数再測定＝実体の消失 0・行番号ドリフト常在。③XL 3 本を分割＝新規 brief 6 本。④三重所有の仮裁定（案 甲・09-13 に `zorder` のみ改訂）。⑤roadmap 減量（77.8KB→約 3 分の 1）。⑥W13〜W17 の編成。⑦直接修正候補の再確認。（全文は history「2026-09-11 棚卸⑬退避」節の後ろ・本追記の旧全文は 09-18 に history へ）

**2026-09-12 追記(96)（`nar-install` 起票）**: 検体の保管形式を `.nar` へ・M2 の NAR エンジンを開発側の駆動で先に建てる。単一 spec・単独枠・`zip 8.6`（`typed-path` 1 本のみ新規）・`ZipArchive::extract()` を呼ばない・Shift_JIS は `name_raw()`＋`encoding_rs` で自前復号。（旧全文は 09-18 に history へ。**依存は 09-18 に `zip` を採らず `miniz_oxide` 0.9＋`adler2` で決着**＝コンテナの読み手は自前で持つ）

**2026-09-18 追記(97)（棚卸⑭＝M2 ゴールを α へ・6 本起票・W14〜W17 を α 後へ）**: `/kiro-discovery` 再入（開発者「ロードマップの調整を行い必要な spec を起票せよ。α 版として第三者に使い始めてもらえる機能セット＝ゴースト・シェル・バルーンのファイル管理、インストール、ネットワーク更新、最低限のメニュー。オーナードロー不要。表現力は emo2 が動く水準で一旦よい。ukadoc を確認した上で深掘り」）。①**現状探索（サブエージェント・35 ツール呼び出し）**＝管理機能 7 種（インストーラ・列挙・切替・ネットワーク・メニュー・トレイ・投げ込み）は**全て実装 0**。起動は argv 1 本（`boot_config.rs:52-65`）・根の概念なし・`App` スコープは配線済みで鍵 0・HTTP クライアント 0・右クリックはダブルクリックとしてのみ SHIORI へ・終了は Ctrl＋左ダブルクリックのみ・構造的に単一ゴースト（窓 0 で `app.run()` が返る `main.rs:317`）。②**ukadoc 精読**＝`manual_install`／`manual_directory`／`manual_update`／`dev_update`／`spec_update_file`（行の形 `パス\x01MD5\x01拡張…`・CRLF・無効エントリ 3 種・URL 符号化の判定・`charset` は先頭エントリ）／`descript_install` の 16 キー／`OnInstall*` 7 種／`OnUpdate*`・`OnUpdateOther*` の全列と `OnUpdateResult` の Ref 形式／`\![change,…]` 3 種と `--option=raise-event`／`OnGhostChanging` 204→`OnClose`・`OnGhostChanged` 204→`OnBoot`／メニューの `*button.caption` リソース 92 件と `popupmenu.visible|type`／`OnFileDrop2` が現行仕様。束の帰属は `linkage.md` の `[bundle."インストール"]`（41 件）・`"更新"`（51 件）・`"切替"`（32 件）・`"メニュー"`・`"配布物の素性"`（22 件）・`"投げ込み"`（10 件）・`"作り付けの窓"`（17 件）。③**起票 6 本**＝`baseware-root-layout`・`ghost-shell-balloon-switch`・`popup-menu-minimal`・`ghost-install`・`network-update`・`alpha-release-signoff`（brief は全て file:line 付き・裁定候補 5 件を末尾に登記）。既存の `nar-install`（エンジン）と `shell-implicit-surface`（里々の絵）を α に格上げ。④**ウェーブ A0〜A6**（ほぼ直列＝`main.rs`／`runtime.rs`／`kanade/schedule` の輻輳）。⑤**W14〜W17 の 19 本を α 後へ**（brief 維持・旧ウェーブ行と干渉台帳は history へ退避）。⑥仮裁定 9 件（上節）。⑦`product.md` の「アルファリリースターゲット」表を α の定義へ更新・`nar-install` brief に「展開先＝根の形・製品側は `ghost-install`」を追記。⑧番人の確認＝`cargo test -p ukadoc-survey` の検査は `roadmap.md` を読まず（読むのは `roadmap-draft.md` と `.kiro/specs/` 直下の brief 持ちディレクトリ）、spec ディレクトリの追加は緑（`spec_checks.rs` の `adding_another_spec_directory_stays_green_and_removing_a_listed_one_turns_red`）。台帳の `owner` は各 spec の要件段階で登記。

**2026-09-18 追記(98)（同日 2 度目の再入＝既定バルーンを独立 spec `default-balloon-bundle` へ）**: 開発者「同梱バルーンは癖が無いものが必要。emo2-kakukaku では厳しい。SSP から借用でもよいと思うが…」→ 調査: SSP 同梱バルーンの再配布条件は公開されておらず（公式・FAQ・ukadoc に記載なし・SSP ソースは GitHub に無し）、代替として SSP 本家の作者による **CC0** の `Balloon for Staysee Syncfield`（`github.com/ponapalt/StayseeBalloon`・readme「転載・再配布・同梱・改変等：煮るなり焼くなり好きにしてください」・専用指定なし・`use_self_alpha,1`・`.pna` 無し・`font.height,12`）を提示。開発者「**areka は常に `use_self_alpha,1`。pna 対応は不要**」（`areka-emo-present/src/balloon.rs:18-20` の設計と一致・descript の `use_self_alpha` は読まず常に 1 が areka の裁量）→「spec を作ってそこで扱う。ここまでの議論を brief に残せ」。①**起票 `default-balloon-bundle`**（S・A2 並走・brief に議論の記録を全文）。②`baseware-root-layout`・`alpha-release-signoff` の裁定候補 ⑴ を同 spec へ移管（両 brief に追記）。③仮裁定 8 を改訂・仮裁定 10 を追加。④台帳 37 本（α 9）。

**2026-09-18 追記(99)（同日 3 度目の再入＝nar-install を先頭へ・α 無関係の spec をウェーブから除外）**: 開発者「次に実施可能な spec は何か。nar 関係は早く進めないとダメ。サンプルゴーストをいろいろ試験するための仕組み作りも nar 関係。α リリース優先のため、並行実施可能な別 spec があっても α に関係なければウェーブに含めない」。①**A0＝`nar-install`（即時着手可・単独）**へ反転。`shell-implicit-surface` の brief「推奨は本 spec が先」を取り下げ（同 brief に追記）＝implicit-surface は共有ヘルパを最初から使う。②**A1＝3 本並走**（implicit-surface ∥ root-layout ∥ balloon-bundle・共有ファイル 0 の見込み・⚠同 crate 別ファイル 2 組）。以降 A2 switch → A3 menu → A4 install→update → A5 signoff（**A0〜A5 の 6 段**）。③**「隙間」枠を廃止**＝`dpi-transition-two-tick-bounce`・`zorder-chain-residue` を α 後へ（仮裁定 11・zorder A-2 は α のテストを実際に赤にした時だけ単独枠に挟む）。④**サンプルゴースト試験の仕組みは別 spec を切らない**（仮裁定 12）＝nar-install の共有ヘルパ（検体名 → 根）と `vendors/sample_ghost/*.nar` の慣行が器。YAYA 検体の追加は `alpha-release-signoff` の要件で裁定（候補 ⑹）。

**2026-09-18 追記(100)（同日 4 度目の再入＝「並走不可」を実測で改訂・A0 を 3 本並走へ）**: 開発者「メニューとサンプルバルーンは最初に実施できないか」→ 実測: nar-install が書き換える 38 ファイルは全て検体パスを参照する既存のテスト・example（`emo2_boot` 6・`areka-emo-text/tests` 5・`placement` 4 …）。`popup-menu-minimal` の接触面（`input_events/`・wintf ポインタ経路・新規 `menu.rs`）は検体参照 0＝共有 0。`default-balloon-bundle` は「保管を展開フォルダで行い（`.nar` 化は nar-install が引き受け）・表示検証は新規テストのみ・既存の検体参照テストを触らない」の 3 条件で共有 0。`THIRD-PARTY-NOTICES.md` は `cargo about` 自動生成で CC0 バルーンは載らない。①**A0＝nar-install ∥ popup-menu-minimal（第 1 スライス＝説明書・終了・caption・visible・`MenuRegistry`）∥ default-balloon-bundle**。②A1＝implicit-surface ∥ root-layout（root-layout が既定バルーン id と列挙サブメニューの登記を足す）。③A3 は空き（番号は据え置き）。④**見た目の採否は今日にでも可能**＝フォルダを置いて argv 第 2 引数で起動（コード変更 0）。⑤教訓: 「N ファイルに触るから並走不可」は**どのファイルかを見てから**言う（記憶 prefer-clean-waves は「干渉するなら分ける」であって「触る本数が多ければ分ける」ではない）。

**2026-09-19 追記(101)（`popup-menu-minimal` の最終検証 GO・引受先の無い問題を 3 本起票）**: 開発者「推奨タスクはツールチップにせず spec に起票・どこにも分類されていない問題は起票」。① **#38 `wintf-drag-state-rest-contract`**＝実機確認 9.3 が見つけた「左クリックを 1 度するとメニューが出なくなる」の根（wintf のドラッグ状態の説明と実装の食い違い）。利用側は #14 で是正済み・契約の是正は wintf の範囲なので別 spec。② **#39 `popup-menu-residue`**＝最終検証の Warning／Note のうち #14 で直さなかった 10 件。③ **#40 `coverage-roadmap-refresh`**＝網羅調査の文書の手書きの数（検査の外）と、各 spec が完了時に残してきた「統合担当への申し送り」の受け皿（他 spec 由来の 4 件は brief に「未検証」と明記）。いずれも α の必須ではない（段は α 後）。**既存 spec への追記 2 件**: `shell-implicit-surface`（里々のメニュー項目名の実機観察 1 件を引き受け）・`alpha-release-signoff`（実機手順の教訓 3 件）。**起票しなかったもの**: main（PR #157）の取り込みと台帳の数え直しは #14 の `/kiro-complete` の手順そのもの／`vendors/sample_ghost/R_POST_and_KOMAINU/ghost/master/profile/` の未追跡の保存データは、検体が `.nar` 保管（#11）へ移れば展開先へ書かれるので再発しない（今ある 1 個は開発者の許可を得て消す）。

**2026-09-19 追記(102)（起票は 09-18＝main への着地は #101 の後・09-18 の 5 度目の再入＝`origin` を validrect の外に宣言したバルーンで行頭が欠ける欠陥を起票）**: 開発者が ghost_dev 側の emo2 開発版を areka で起動したところ、両バルーンとも行頭が 1 文字欠けた（「ちがうよう。」→「がうよう。」）。原因は開発版のバルーン定義に残っていた `origin.x,0`／`origin.y,0`（面別層の `validrect.left` は 36／24）で、2 行を消すと直る。**同じ定義は SSP では欠けない**（開発者の目視）。開発者「ukadoc を確認し、バグが認められるなら修正 spec を立ち上げるように」。①**ukadoc の確認**＝`origin.x` は「通常は指定せず validrect の定義に任せる」・`validrect` は「テキストを描画してよい範囲」・範囲外の宣言には沈黙。②**バグと認めた**＝PR #124（`balloon-vertical-canon`）が寄せ戻しを撤去した根拠は `COMPAT_ARCHITECTURE.md` §8 の「validrect 外の宣言は SSP でも壊れた定義」という**SSP を見ずに置いた仮定**で、今回の目視が反証した。areka の文字の面は validrect と同じ範囲しか持たないので、範囲外の開始点は「字義どおりに表示」されず切り落とされるだけ＝ukadoc の 2 文と両立する解は「範囲の内へ戻す」しかない。同梱の検体が無傷だったのは、撤去と検体の `origin` 削除を同じ PR で揃えたからにすぎない。③**起票 `balloon-origin-outside-validrect`**（S・**#46**（台帳の番号は main との合流で #38 → #46 へ遷した――main 側が 09-19 に #38〜#42 を「登記だけの行」で埋めたため）・**α の A0-④・即時着手可**）＝範囲外に宣言された成分を「宣言なし」として書字開始角へ落とす（撤去前の形の復元・最寄りの辺へは寄せない）。編集は `areka-emo-text/src/region.rs` の 1 関数＋兄弟テストの新設（`region.rs` は 951 行）＋COMPAT §8 の 3 行。A0 の 3 本と共有 0（`nar-install` の 38 ファイルに `region.rs` は含まれない・実測）。④台帳は main との合流後に実数え＝表 46 行・brief 済み 41 本・完了 12・α 8・α 後 25・保留 1。⑤教訓: **「SSP でも壊れている」は測っていなければ仮定である**——正典の沈黙を埋める裁定に SSP の挙動を理由として書くなら、目視 1 件でよいので裏を取る（SSP 実測主義は取らないが、目視証跡は根拠に使える）。

**2026-09-19 追記(103)（`balloon-origin-outside-validrect` 完了＝A0 の 4 本目・A0 は閉じた）**: 範囲外に解決された `origin` 成分を「宣言なし」として書字開始角へ落とす規則が着地した（**最寄りの辺へは寄せない**・両端は範囲内・成分ごとに独立・負値は反対辺基準で解決してから範囲判定・記録は解決側 DEBUG と装着 1 回につき成分 1 つ WARN 1 件の 2 段）。**完了 spec `areka-P0-balloon-vertical-canon` 要件 3.10 を上書きした**（3.11 の未宣言時の縮退・3.7 の負値解決は不変）。2026-08-27 の撤去の取り下げであり、取り下げの理由と経緯は `doc/COMPAT_ARCHITECTURE.md` §8 の「撤去の取り下げ」の行に登記した。実装は `crates/areka-emo-text` の中で完結（新設ファイル 0・依存の追加 0）。検証＝crate 17 ターゲット 896 passed、ワークスペース 8120 passed / 0 failed。①**教訓（テスト）**: 「最寄りの辺へ寄せる」後戻りを捕まえるのは**縦書きを回すテストだけ**——横書きでは書字開始角と最寄りの辺が同じ点になるため、横書きだけの 6 本は素通りさせる。②**教訓（レビュー）**: 「全項目に○○」型の要件は語での全文検索では守れない（旧規則は「字義」「クランプ」「撤去」を 1 語も使わずに書ける）。効いたのは概念の共起（`origin` と 宣言／指定）を全数読んで分類する走査である。③**教訓（檻）**: 振る舞いが正しいことと、それを固定する検査が在ることは別——構造体へ欄を足して `PartialEq` の意味が変わったとき、その欄だけが違う 2 値を作るテストが無ければ、将来欄を外しても緑のまま通る。
