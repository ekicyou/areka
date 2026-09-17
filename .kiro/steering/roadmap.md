---
inclusion: manual
updated_at: 2026-09-13
---

# Roadmap — areka（M1 完成後・M2 組み直し前の裁定枠ロードマップ）

> **M1 は 2026-09-11 に完成宣言済み**（下記「M1 ゴール」）。本ファイルは **brief 済み spec 30 本（うち完了 9）の着手順（ウェーブ）と干渉条件だけ**を持つ。M2 本文はここに書かない（「M2 以降」節）。**M2 ロードマップを起こす材料は 2026-09-13 に揃った**（`ukadoc-coverage-roadmap` 完了・両段とも実施済み）——束と段階と順位の案は `doc/ukadoc-coverage/roadmap-draft.md`、帰属の正本は同 `linkage.md`、段階と順位の正本は同 `briefing.md` にある。次は `/kiro-discovery` 再入で先頭ウェーブ分だけ起票する（「M2 以降」節）。
> 正本配置: 本ファイルが正本（`.kiro/steering/roadmap.md`）。`focus.md`（`inclusion: always`）から辿る。設計判断の正本は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md)。M1 実物スコープは [doc/emo2-conformance-scope.md](../../doc/emo2-conformance-scope.md)。
> **履歴**: 追記①〜(94)・旧ゴール表・旧ウェーブ行・旧干渉台帳・完了詳報は棚卸④〜⑬で [roadmap-history.md](roadmap-history.md) へ退避済み（history が全文正本・非改変）。完了ユニットの実装詳細は各 `completed/` spec が正本。**旧ウェーブ番号の読み替え**: 棚卸⑫（09-02）で W5.95→W6・W6→W7・W6.5→W8・W6.75→W9・W6.9→W10・W6.95→W11・旧 W7（e2e）→W12 へ整数化。棚卸⑬（09-11）で **旧「W13 裁定枠 D〜G」「W13〜W15（裁定枠）」を W13〜W17 へ振り直し**（下表が正本・history と completed spec 内の旧番号は改変しない）。

## M1 ゴール ✅（2026-09-11 完成宣言）

areka（x64）が最小 SSP 互換ベースウェアとして、適合対象ゴースト **emo2**（作者自作・脳=`pasta.dll`・32bit SHIORI）を「そのまま」起動→会話→撫で→メニュー→終了まで E2E 実走させる。emo2 が動く＝同じ汎用 32bit ブリッジで里々/YAYA も動く土台。

**✅ 完成宣言**: 適合検証項目表 20 項目全合格の実機サインオフに開発者の署名（`.kiro/specs/completed/areka-P0-emo2-conformance-e2e/verification/m1-completion.md`・項目別は同 `acceptance-record.md` §8.1）。持ち越し 8 件（同 §6）はいずれも判定を書き換えず、引受先は下の spec 台帳に全て実在する（§13.1 行 3「初回起動限定の位置調整」だけは「違和感があれば個別仕様を切る」の据え置き＝spec なし）。**M2 ロードマップの起点はこの宣言**（同 §7）。

## 実装規律（balloon-system の失敗から得た正）

- **実装ファースト**: 各作業ユニットの成果物は「実際に動く」検証済みコード。**spec 工場の禁止**: 成果物が子 spec になる構造を作らない。1 ユニット＝1 かたまりの動く振る舞い。
- **最小実装＋薄い拡張シーム**: 使う分だけ実装し、拡張は型/レジストリの口だけ残す。抽象は「2 例目の実物」が要求してから。動く資産から建てる。
- **粒度基準**: 1 ユニット＝単一 pass/fail の独立観測。純粋層は fixture/mock 直入力で切る。UI 位置決め・座標系は本番ゴースト（実 emo2）＋実 DPI（≠96）が観測条件（記憶 areka-placement-real-ghost-first）。
- **語彙完備・配線ゼロの追跡**: 先送りシームには狭い `#[allow(dead_code)]`＋実在理由の doc を義務付け、消費者ゼロの検出は棚卸の定期監査項目。
- **1 ファイル 1,000 行**: 機械の番人 `crates/log-capture-kit/tests/file_length_guard_test.rs`（例外表 11 件・暗黙増加不可・**どの spec も例外表に触れない**）。2026-09-11 実測: `areka-emo-text/src/` の `draw.rs` 988・`layout.rs` 955・`actor.rs` 952・`region.rs` 951 が射程＝emo-text を触る spec は新規ファイルで足す。
- **決定論テスト網羅は必達**・**ログ無し失敗経路の禁止**・**終了経路は正規実装**（記憶 deterministic-test-coverage-mandate／areka-log-first-no-silent-failure／canonical-not-minimal-lifecycle）。

## アーキテクチャ横断原則（要約・詳細は history＋記憶＋completed spec）

- **シェル/バルーン統一**: 描画エンジンはシェルとバルーンを区別しない。バルーン＝surface 上の文字レンダリング層。
- **アニメエンジンは 2 つ**: ①さくらスクリプト再生（sakura）＋②SERIKO ループ（seriko）。両者とも dola（絶対時刻台本）上。
- **並行モデル**: 各エンジン＝チャンネル通信のアクター＋独立スレッド。render/window は UI スレッド固定。機構=areka-actor／経路=kanade／結線=ghost。
- **emo 合成**: 自前コンポジタ（アトラス→1 枚物合成→wintf へ完成品のみ）。emo=UI 層全般。**画像は原寸で持ち拡大縮小は D2D 変換行列**（2026-09-11 裁定・`present-gpu-transform-scale` が実装）。
- **DPI 追従が基本設計**: k=monitorDPI÷author_dpi で全表示経路がスケール。キャラ窓の原点は下端中央。バルーン追従は例外＝「窓（char 左上）相対」・offset の単位空間は表示 DPI の物理 px 1 つ。
- **スコープ窓の重なり**: 既定は非強制、`\![set,zorder]` 後は Windows の所有関係（owner）の一直線の鎖で構造保証。
- **意味論は ukadoc から輸入・SSP 実測主義は取らない**（記憶 no-ssp-measurement-import-semantics-from-ukadoc）。**`\!` コマンドは汎用キャリア 1 本**（typed 個別新設禁止・消費側 name 選別）。

**エンジン固有名**（コード/spec/会話の参照はこの名で統一）:

| # | エンジン | 固有名 | # | エンジン | 固有名 |
|---|---|---|---|---|---|
| ⓪ | ゴーストエンジン（最上位 owner） | `ghost` | ④ | さくらスクリプト再生 | `sakura` |
| ① | SHIORI 通信層 host-32 | `shiori` | ⑤ | SERIKO アニメ | `seriko` |
| ② | parser/loader | `parsers` | ⑥ | render（surface 合成＋UI 層） | `emo` |
| ③ | conductor（SHIORI イベント循環） | `kanade` | | | |

## 完了サマリ（M1 全ウェーブ・詳報は completed spec ＋ history）

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

- 完了 spec 直下エントリ＝**184**（`.kiro/specs/completed/` 直下・2026-09-17 実数え＝ディレクトリ 183＋`graphics-rendering-stability.md` 1）。⚠ **引き算で導かず毎回実数えする**（並走 spec が同じ行を更新する）。
- M1 実機サインオフ発見 7 件中 #1〜#6 解決済み・#7（冒頭空行）は pasta 上流。e2e の持ち越し（§13.1 行 1・§13.2 行 4・9・10）は W13 の 4 本が引受先。M-dual は退役（e2e 項目 10 で合格・復活させない）。

## 進行中の spec 台帳（brief 済み 30 本・2026-09-11 実数え 28 ＋ 09-12 起票 1 ＋ 09-13 起票 1。**うち完了 9・進行中 21**＝2026-09-17 に状態列を実数え・着手は `/kiro-start <名>`）

> **種別**の優先順は **バグ → 依存ツリーが長い → その他**。**規模**は brief の申告（棚卸⑬で分割したものは分割後）。**状態**列は `/kiro-complete` が ✅ に更新し、完了数を実数えで更新する。**Fable**列＝要件定義（design）を Fable で行うべきか（○＝Fable・−＝Opus で足りる）。

| # | spec（`areka-P0-` 省略） | 種別 | 規模 | Wave | 前提（先に着地） | Fable | 状態 |
|---|---|---|---|---|---|---|---|
| 1 | `present-gpu-transform-scale` | バグ（性能・**最優先**） | M | **W13** | なし | ○ | ✅ |
| 2 | `completed/kanade-boot-talkdone-drop` | バグ（構造） | S | **W13** | なし | − | ✅ 完了（2026-09-17） |
| 3 | `completed/host32-window-thread-pump` | バグ（構造） | S〜M | **W13** | なし | −（案 1/2 の裁定 1 件） | ✅ 完了（2026-09-17） |
| 4 | `completed/sakura-tag-word-boundary` | バグ（完了契約 13.1 の穴） | S〜M | **W13** | `sakura-bare-tag-lexer` ✅ | − | ✅ 完了（2026-09-17） |
| 5 | `charset-canon` | 正典（里々ゴーストが 1 体も動かない・surfaces.txt 2 箇所は実バグ） | M | **W13** | なし | ○（⑵ 交渉のみ） | ✅ |
| 6 | `ukadoc-coverage-roadmap` | 調査（**M2 ロードマップの生成器**・依存ツリー最長） | M | **W13** | survey 4 本 ✅・e2e ✅＝**両段とも解禁済み** | ○ | ✅ |
| 7 | `completed/text-decoration-canon`（⑴ 基盤＋font 10＋default/disable） | 正典（文字装飾 4 本の先頭ゲート） | L | **W13** | bvc ✅・cursor-tag ✅・line-height ✅ | ○（design） | ✅ 完了（2026-09-13） |
| 8 | `completed/sylphya-set-ledger`（channels ⑶・**09-11 起票**） | 台帳（三重所有の台帳側） | S | **W13** | なし | − | ✅ 完了（2026-09-17） |
| 9 | `completed/balloon-font-descript-keys`（decoration ⑶・**09-11 起票**） | 正典（転記＋書体の配線） | S | **W13** | なし | − | ✅ 完了（2026-09-17） |
| 10 | `surfaces-basepos` | 正典（完全独立） | S | W13 任意／W14 | なし | − | ⚪ |
| 11 | `dpi-transition-two-tick-bounce` | バグ（見た目・判定器） | M | **W14** | #1 | ○ | ⚪ |
| 12 | `property-query-channels`（⑴ スクリプト経路 1〜4） | 正典（プロパティ系の先頭ゲート・依存ツリー長） | M〜L | **W14** | #4・#7（`lexer.rs`／`decode.rs`）・#2（`kanade/schedule`）・#8 | ○ | ⚪ |
| 13 | `zorder-chain-residue`（A 群＝間欠赤を先に） | 台帳（A 群はバグ） | M | **W14** | #3（隣接・同時着手しない） | ○（A 群の決着設計のみ） | ⚪ |
| 14 | `emo-text-canon-residue`（residue ⑶・**09-11 起票**・項目 12 は実バグ） | 台帳（emo-text 残件 4） | M | **W14** | #7（`draw.rs` 分割）・#9（`balloon/parse.rs`） | − | ⚪ |
| 15 | `balloon-canon-residue`（⑴ 系列 1〜6） | 台帳（M2 互換面） | M | **W14** | #1（`areka-emo-present` 同居） | −（議題 2 件） | ⚪ |
| 16 | `property-ipc-transport`（channels ⑵・**09-11 起票**） | 正典（輸送路のライブ実測が前提） | M | **W14** | #3・#5（host32 crate） | ○ | ⚪ |
| 17 | `translate-pipeline` | 正典（→ makoto） | M | **W15** | #2・#12（`kanade/{actor,msg,schedule}`） | ○ | ⚪ |
| 18 | `text-align-shadow-canon`（decoration ⑵・**09-11 起票**） | 正典（寄せ 2＋影 3＋SC8 裁定） | M | **W15** | #7・#14（`layout.rs`） | ○ | ⚪ |
| 19 | `status-execution-states` | 台帳（`balloon`／`minimizing` の源は実在） | S | **W15** | #12（`emo2_boot/mod.rs`） | − | ⚪ |
| 20 | `currentghost-property-tree`（`seriko.*` から `zorder` 除外） | 正典（値の木） | L（19 項スライス可） | **W15** | #12・#8 | −（議題 2 件） | ⚪ |
| 21 | `zorder-property` | 正典（値の導出のみ） | S | **W15** | #8・完了 zsp | − | ⚪ |
| 22 | `makoto-dll-host` | 正典（MAKOTO/2.0） | L（ⓐ loadu は XS 先行可） | **W16** | #5・#17・#16・#3（host32／`shiori/real.rs`） | ○ | ⚪ |
| 23 | `sakura-time-directives` | 正典（allowlist 8 族） | L（A/B/D → C） | **W16** | #7（`compile.rs`）・#12（`dola/cue`）・#17（kanade） | ○（C 群のみ） | ⚪ |
| 24 | `property-catalog-lists` | 正典（島単位） | M | **W16** | #20（`dotted.rs`／`key.rs`／`mod.rs`） | − | ⚪ |
| 25 | `choice-marker-styling` | 正典（`\f[cursor*]` 10） | S | **W16** | #7・#18（`draw.rs`／`decode.rs`） | − | ⚪ |
| 26 | `anchor-tag-canon` | 正典（`\_a`＋装飾 16） | M | **W17** | #7・#18・#25（`decode.rs`／`viewbox_draw.rs`） | ○（16 項目の 3 状態意味論） | ⚪ |
| 27 | `balloon-lifecycle-events`（residue ⑵・**09-11 起票**） | 正典（表示寿命 7〜10） | M | **W17** | #7（項目 9）・#23（項目 7）・#17（`schedule/events.rs`） | −（裁定 2 件） | ⚪ |
| 28 | `tick-gate-adoption` | 性能（既定 OFF の門の採否） | M〜L | **保留** | 「長時間試行禁止」と両立する A/B 設計を要件で先に組む・e2e／pwc の後（単独） | ○ | ⚪ |
| 29 | `nar-install`（**09-12 起票**） | 基盤（検体を `.nar` 保管＋展開で実験環境・M2 の NAR エンジンを先に建てる） | M | **単独枠**（ウェーブの切れ目） | なし（38 ファイル・9 クレートに触るため**全 spec と共有ファイル発生**＝並走不可） | ○ | ⚪ |
| 30 | `shell-implicit-surface`（**09-13 起票**） | バグ（**里々標準テンプレートが 1 枚も絵を出せない**＝`charset-canon` の残り半分。文字は直ったが絵が出ない） | M | **W14** | `charset-canon` ✅・W13① `present-gpu-transform-scale`（`areka-emo-compose` 同居）・`nar-install` より**先**（検体パスが消える） | ○（「element0 より下」の層表現と `surface.append` の順序の裁定 2 件） | ⚪ |

**規模と分割の裁定（棚卸⑬）**: XL 3 本を分割済み＝`property-query-channels` → ⑴本体／⑵`property-ipc-transport`／⑶`sylphya-set-ledger`、`balloon-canon-residue` → ⑴本体（系列）／⑵`balloon-lifecycle-events`／⑶`emo-text-canon-residue`、`text-decoration-canon` → ⑴本体（基盤＋font 10＋default/disable）／⑵`text-align-shadow-canon`／⑶`balloon-font-descript-keys`。分割元 brief に「本 spec が持つ範囲」を追記済み・項目本文は分割元が正本（重複させない）。**L のまま置く 4 本**（`makoto-dll-host`・`sakura-time-directives`・`currentghost-property-tree`・`text-decoration-canon`）は brief 内のスライス（ⓐ loadu／A-B-D 先行／balloon.scope 19 項／font 10 の一部後送り）で要件段階に縮める余地を残す。

## 棚卸⑬の仮裁定（2026-09-11・開発者が覆すまで有効）

1. ~~**`currentghost.seriko.zorder` の三重所有**＝値の導出は `zorder-property` 単独・**SET 台帳行（`seriko.zorder`／`seriko.sticky-window`）は `sylphya-set-ledger`**・`currentghost-property-tree` は `seriko.*` から `zorder` を除外~~ → **2026-09-13 開発者裁定で `zorder` のみ改訂**。`zorder` は**台帳行も値の導出もともに `zorder-property`** が持つ（語彙表への先行登記は行わない＝完了 spec `areka-P0-scope-zorder-pinning` 要件 13.5 の先送りを維持し、`crates/areka/src/placement/zorder_property_deferral_tests.rs` の 3 本を緑のまま残す）。`seriko.sticky-window` の台帳行が `sylphya-set-ledger` である点と、`currentghost-property-tree` が `seriko.*` から `zorder` を除外する点は**据え置き**。したがって `sylphya-set-ledger` の SET 有効群は 21→**25**（26 ではない）。
2. **M2 予約群は brief を起票しない**（SSTP・FMO・DirectSSTP・Plugin/HEADLINE・ネットワーク更新・ゴースト選択 UI・多重ゴースト・NAR・pasta native x64・ベクトル描画・owner-draw メニュー・バルーン美観配置・回転テキスト）。生成器は `ukadoc-coverage-roadmap` 第二段（W13）→ `/kiro-discovery` 再入で先頭ウェーブ分だけ just-in-time 起票（spec 工場化しない）。
3. **e2e §13.1 行 3**（初回起動限定の位置調整が 2 回目以降で既定へ戻るか）は spec を切らず据え置き。⚠ 引受先としていた `present-gpu-transform-scale` の k≠1 サインオフ（2026-09-12 完了）では**実施しなかった**（同 spec `verification/acceptance-record.md` §7 行 6 に「未実施」と登記）。次の実機一周で目視する。

## ウェーブ編成（着手順の正本・2026-09-11 棚卸⑬・整数番号）

> 各ウェーブは**フルライフサイクル**（要件→設計→タスク→実装→`/kiro-complete`＝PR squash マージ）を完走してから次へ。並走はウェーブ内のみ（1 spec = 1 worktree = 1 PR）。同居は**実測で共有ファイル 0**が原則（記憶 prefer-clean-waves-over-max-parallelism）。「同 crate・別ファイル」は⚠付きで許容し、後着が rebase する。文書フェーズは先行可＝先行 spec はウェーブ開始時に settled main へ再突合。**依存は「必ず並走する」前提で厳しめに切った**（同ディレクトリ `kanade/schedule/*`・`areka-parsers/sakura/*` は 1 ウェーブ 1 spec 相当の扱い）。

| Wave | ユニット（優先順） | 開始コマンド | 編成根拠・条件 |
|---|---|---|---|
| W1〜W12 ✅ | 完了サマリ参照 | — | 旧行全文は history |
| **W13**（即時・9 本＋任意 1） | ① `present-gpu-transform-scale` ② `kanade-boot-talkdone-drop` ③ `host32-window-thread-pump` ④ `sakura-tag-word-boundary` ⑤ `charset-canon` ⑥ `ukadoc-coverage-roadmap` ⑦ `text-decoration-canon` ⑧ `sylphya-set-ledger` ⑨ `balloon-font-descript-keys`（⑩ `surfaces-basepos` 任意） | `/kiro-start areka-P0-present-gpu-transform-scale` ／ `/kiro-start areka-P0-kanade-boot-talkdone-drop` ／ `/kiro-start areka-P0-host32-window-thread-pump` ／ `/kiro-start areka-P0-sakura-tag-word-boundary` ／ `/kiro-start areka-P0-charset-canon` ／ `/kiro-start areka-P0-ukadoc-coverage-roadmap` ／ `/kiro-start areka-P0-text-decoration-canon` ／ `/kiro-start areka-P0-sylphya-set-ledger` ／ `/kiro-start areka-P0-balloon-font-descript-keys`（／ `/kiro-start areka-P0-surfaces-basepos`） | バグ 4（①〜④）→ 長い依存ツリーの先頭 3（⑤ → makoto・⑥ → M2 全部・⑦ → 文字装飾 4 本）→ S の独立 2（⑧⑨・rebase 源を先に消す）。全ペア共有ファイル 0（実測・下の干渉台帳）。⚠ 同 crate 別ファイル 3 組＝②⇄③（`areka-kanade`）・③⇄⑤（`shiori-host32-host`）・④⇄⑦（`areka-parsers/sakura`）。⑦ の最初の作業は `draw.rs` 分割。⑥ は `doc/ukadoc-coverage/` のみでコード非接触 |
| **W14**（W13 完走後・8 本） | ① `dpi-transition-two-tick-bounce` ② `property-query-channels` ③ `zorder-chain-residue` ④ `emo-text-canon-residue` ⑤ `property-ipc-transport` ⑥ `balloon-canon-residue` ⑦ `surfaces-basepos`（W13 で未着手なら） ⑧ `shell-implicit-surface` | `/kiro-start areka-P0-dpi-transition-two-tick-bounce` 他 | ① は W13① 着地後に走行 D 形式で再計測してから設計。② は `lexer.rs`／`decode.rs`／`kanade/schedule` が空く W14 が最速。③ は W13③ と隣接ゆえ W14。④ は `layout.rs`／`region.rs`／`balloon/parse.rs` が W13⑦⑨ の後。⑤ は host32 crate が W13③⑤ の後。⑥ は `areka-emo-present` が W13① の後。⚠ 同 crate 別ファイル＝①⇄②（`areka/src/emo2_boot/`）・②⇄⑤（`areka-ghost`）・①⇄⑥（`emo2_boot/frame/`・`areka-emo-present`）。⑧ は `charset-canon` ✅ の残り半分（里々の**絵**）で、`areka-emo-compose`／`emo2_boot/assets.rs`／`placement/measure.rs` に触るため W13① 着地後。⚠ ⑦⇄⑧ は `areka-parsers/src/shell/{decode,model}.rs` で衝突しうる＝**同時に走らせず直列**。⑧ は `nar-install` より必ず先（検体 `vendors/sample_ghost/R_POST_and_KOMAINU/` のパスが nar-install で消える） |
| **W15**（5 本） | ① `translate-pipeline` ② `text-align-shadow-canon` ③ `status-execution-states` ④ `currentghost-property-tree` ⑤ `zorder-property` | `/kiro-start areka-P0-translate-pipeline` 他 | ① は `kanade/{actor,msg,schedule}` が W14② の後。② は `layout.rs` が W14④ の後。③ は `emo2_boot/mod.rs` が W14② の後。④⑤ は仮裁定 1 と W13⑧・W14② の後。⚠ ①⇄③（`areka-kanade` 別ファイル）・④⇄⑤（`dotted.rs` は ④ のみ・⑤ は `placement/zorder_group_ledger.rs`） |
| **W16**（4 本） | ① `makoto-dll-host` ② `sakura-time-directives` ③ `property-catalog-lists` ④ `choice-marker-styling` | `/kiro-start areka-P0-makoto-dll-host` 他 | ① は W13⑤・W14⑤・W15① の後（host32 crate・`shiori/real.rs`・kanade）。② は `compile.rs`（W13⑦）・`dola/cue`（W14②）・kanade（W15①）の後。③ は `dotted.rs`／`key.rs`／`mod.rs` が W15④ の後。④ は `draw.rs`／`decode.rs` が W15② の後。⚠ ①⇄②（`areka-kanade` 別ファイル） |
| **W17**（2 本） | ① `anchor-tag-canon` ② `balloon-lifecycle-events` | `/kiro-start areka-P0-anchor-tag-canon` 他 | ① は `decode.rs`／`viewbox_draw.rs`／`draw.rs` が W16④ の後。② は項目 9（W13⑦）・項目 7 の compile 側（W16②）・`schedule/events.rs`（W15①）の後。共有 0 |
| **単独枠** | `nar-install` | `/kiro-start areka-P0-nar-install` | **ウェーブの切れ目に単独で置く。** 検体パスの参照 38 か所・9 クレートを 1 つの共有ヘルパへ寄せる段を含むため、W13〜W17 のほぼ全 spec と共有ファイルが発生する。並走させると後着が全員 rebase する。着手の判断材料は「いま何本走っているか」だけ＝走行中 0 本の瞬間に入れる。⚠ **W14⑧ `shell-implicit-surface` より後**（nar-install が `vendors/sample_ghost/R_POST_and_KOMAINU/` の展開ツリーを追跡外にするので、implicit-surface の檻が参照する検体パスが先に消える） |
| **保留** | `tick-gate-adoption` | — | 夜間/25 分/n≥3 の実測要求が開発者方針「長時間試行禁止」と正面衝突＝要件段階で「始める前に決着可能な A/B 設計」を組めた時点で単独ウェーブへ。他 spec と並走しない（計測を汚す） |

**干渉台帳（W13・W14 の全ペア実測・2026-09-11・共有ファイル ≥1 の組のみ）**:
- **W13 は共有ファイル 0**。⚠ 同 crate 別ファイル: `kanade-boot-talkdone-drop`（`schedule/{boot,mod}.rs`）⇄ `host32-window-thread-pump`（`shiori/real.rs`）／`host32-window-thread-pump`（`parent_window.rs`）⇄ `charset-canon`（`shiori3.rs`／`client.rs`）／`sakura-tag-word-boundary`（`lexer.rs`）⇄ `text-decoration-canon`（`decode.rs`・`compile.rs`）。`text-decoration-canon` は分割後 `balloon/parse.rs` に触れない（`balloon-font-descript-keys` が持つ・既定層の残り 9 キーは後着が配線）。
- **W14**: `property-query-channels` ⇄ `dpi-transition-two-tick-bounce`＝`areka/src/emo2_boot/`（`mod.rs`／`consumer_ledger.rs` 対 `frame/dpi.rs`）・⇄ `property-ipc-transport`＝`areka-ghost`（`prop_sink.rs`／`runtime.rs` 対 `shiori_inproc.rs`）。`balloon-canon-residue` ⇄ `dpi-transition-two-tick-bounce`＝`areka-emo-present`（`balloon.rs` 対 `presenter/refresh.rs`）・`emo2_boot/frame/`（`attach.rs` 対 `dpi.rs`）。`zorder-chain-residue`（`spine_*_tests.rs`・wintf `tick_bridge.rs`・placement テスト）は W14 内で共有 0。
- **輻輳点（ウェーブ跨ぎ・1 ウェーブ 1 spec）**: `areka-parsers/sakura/decode.rs`＝W13 decoration → W14 channels → W15 align-shadow → W16 choice-marker → W17 anchor／`areka-sylphya/vocab/dotted.rs`＝W13 set-ledger → W15 tree → W16 catalog（zorder-property は触れない）／`areka-kanade/schedule/*`＝W13 talkdone → W14 channels → W15 translate → W16 time-directives → W17 lifecycle-events／`areka-emo-text/src/{draw,layout}.rs`＝W13 decoration → W14 emo-text-residue → W15 align-shadow → W16 choice-marker → W17 anchor／host32 crate＝W13 pump・charset → W14 ipc → W16 makoto／`doc/COMPAT_ARCHITECTURE.md` §8 は各自の節のみ追記・後着 rebase／番人の例外表は**誰も触らない**。
- **保存義務**: `property-query-channels` の本番 sink 追加は e2e spine が数える sink 数を変え得る＝檻が数を固定していたら channels が更新する（挙動不変の証跡として）。`sakura-tag-word-boundary` は `\_a[ID]` の角括弧経路を「不変」対象に含める（anchor の前提）。

## 直接修正候補（spec なし・任意・S）

- `crates/areka-emo-text/src/writing.rs` の未知 `writing_mode` 警告文言（「horizontal_tb へフォールバック」→ 実挙動は「指定なしとして扱う」）。逐語固定のインラインテスト `unknown_value_falls_back_to_horizontal_tb_with_warn` と同時修正。`emo-text-canon-residue` 項目 12 と同一＝先に直せば同 spec から外す。
- `.kiro/steering/roadmap-history.md` の `\w[ms]` 記述（history は非改変ゆえ据え置き・`doc/ukadoc-coverage/briefing-sakura-script.md` 側は 2026-09-11 に注記を追加済み）。
- ✅ 解消済み: `cargo clippy` の `absurd_extreme_comparisons`（`overhang <= 0` は 3 ファイルとも `BAND_OVERHANG_MAX` 比較へ置換済み・2026-09-11 実測）。

## 着手手順

- **brief 全数完備体制**: brief 済み spec **30 本**（2026-09-11 実数え 28 ＋ 09-12 起票の `nar-install` ＋ 09-13 起票の `shell-implicit-surface`）全てに brief あり。**うち完了 3・進行中 27**（状態列の実数え・2026-09-13）＝着手は該当 brief を読んで `/kiro-start <unit>` へ直行。brief の file:line は起票時値＝**着手時に必ず再検証**（棚卸⑬の再測定で実体の消失は 0 件・行番号ドリフトは全 brief に常在）。
- 新規課題の起票は `/kiro-discovery`（再入）で just-in-time。`/kiro-spec-batch` は使わない（一括＝工場化）。ウェーブ跨ぎの合流判断は別セッションで一括（記憶 portfolio-convergence-decided-in-separate-session）。
- **要件定義・設計のサブエージェントは Fable**（上表 Fable 列 ○）・タスク生成と実装は Opus（記憶 requirements-design-need-fable-grade-review／fable-main-opus-subagents-token-policy）。

## 制約

- Rust 2024・マルチクレート（一覧は structure.md）。**32bit 可搬性の適用範囲＝host-32 系（`shiori-host32-*`／`shiori-abi`）のみ**。wintf/areka 本体は x64＋arm64 ネイティブ。
- 透過は WUC/DComp GPU 合成上のクリックスルー機構（`WS_EX_TRANSPARENT` 動的トグル＋αマスク）で成立（ULW は撤去済み）。SHIORI 内部唯一 ABI=`IShiori`(COM, HSTRING/UTF-16)。過去互換は 32bit Rust ホスト。
- 設計判断の変更は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md) を正本として更新。
- 実機運転の定石: 絶対パス起動（相対は pasta.dll LOAD 失敗）・i686 helper を先ビルド・`AREKA_APP_SMOKE_EXIT_MS` 有界自動終了＋`RUST_LOG` grep（記憶 areka-real-machine-signoff-bounded-auto-exit）。自動終了は強制終了の経路で終了挨拶を経ないので、終了挨拶を確かめる走行では自動終了を上限に留め、終了はキャラ窓への Ctrl＋左ダブルクリックで求める（2026-09-17 host32-window-thread-pump の裁定）。
- 常時テストは x86 を避け偽境界で純 x64 決定論（記憶 prefer-x64-fake-boundary-tests-not-x86）。

## M2 以降

**M1 完成後に、実物を見て組み直す（起点＝2026-09-11 の完成宣言）。** 本文はここに書かない。組み直しの材料＝完了した `ukadoc-survey-*` 5 本の台帳（`doc/ukadoc-coverage/`・1,749 項目＝実装済み 88／語彙のみ 440／縮退 22／未対応 1,002／別名 27／対象外 170）→ W13 の `ukadoc-coverage-roadmap`（**2026-09-13 完了**・繋がり評価・段階 A〜E・優先度 4 軸＝壊れ方＞伺からしさ＞資産の広さ＞基盤共有度・M3「伺かの冠」候補）→ `/kiro-discovery` 再入で先頭ウェーブ分の brief を起票し、別セッションの棚卸で roadmap へ反映。

**生成器の出力（2026-09-13）**: 順位対象 1,552 件を 67 束に分け、段階 A〜E と順位を付けた。先頭ウェーブの案は **6 束・324 件**（`roadmap-draft.md`）。起票の候補名は案であって決まった名前ではなく、既存 spec と綴りが重なる 1 件（`areka-P0-nar-install`）は裁定待ちである。裁定候補 30 件・本ロードマップの改訂候補 10 件・是正候補 15 件も同文書に並べた。**この 3 文書は起票のたびに古びる写真ではなく、常設の検査 6 種が台帳との一致を見張っている**（`cargo test -p ukadoc-survey`）。

予約（全て任意・brief なし・仮裁定 2）: アプリ層＝SSTP（9801）・FMO・DirectSSTP・Plugin/HEADLINE・ネットワーク更新・ゴースト/バルーン選択 UI・多重ゴースト。互換面＝Shift_JIS（**`charset-canon` で W13**）・SAORI は実装しない（SHIORI が直接 `LoadLibrary`・台帳 `not-applicable`）・里々/YAYA 網羅・NAR。emo テキスト進化＝回転テキスト（`TextEffects` 予約名 `rotation`／`multicolor` は `text-decoration-canon` が M2 シームのまま残す）。**`\f[sub]`／`\f[sup]`／`\f[outline]` は DirectWrite の標準機能で表せる手段が見つかるまで語彙のみ**（`areka-P0-text-decoration-canon` 2026-09-11 の開発者裁定——6 値の解釈・状態の保持・戻しへの参加までは行い表示は変えない・台帳の `status` は `vocabulary-only`・所有仕様なしで追跡先はこの行）。バルーン美観配置（画面端反転・`[visibility-guard] ClampX` の `warn!` 発火回数が優先度根拠）。pasta の native x64／`IShiori` in-proc・ベクトル描画・owner-draw 右クリックメニュー。

---

**追記台帳（要約・全文は history）**: (51)〜(94) は棚卸⑬までに全文退避（history 参照・(88)〜(94) の全文は「2026-09-11 棚卸⑬退避」節）。

**2026-09-11 追記(95)（棚卸⑬＝M1 完成宣言後の全面再編・roadmap 減量・XL 3 本の分割・三重所有の仮裁定・W13〜W17 の整数振り直し）**: `/kiro-discovery` 再入（開発者「main が進んだのでロードマップの棚卸＆仕様の徹底ブリーフィング。brief 無し spec の精査と起票・粒度調整と分割・肥大化の徹底対処・厳しめの依存で並走ウェーブ・小数点番号の整数化・開始コマンド列挙・Fable 推奨」）。①**brief 欠落 0**（22 本全数にあり）＝「roadmap だけにある spec」は M2 予約群と直接修正候補のみ→仮裁定 2 で起票しない。②**22 brief をサブエージェント 3 体で全数再測定**＝ファイル実在の DRIFT は 4 件だけ（channels の `choice_drain.rs`／`sysvar.rs`／`shiori_inproc.rs` のパス・choice-marker の `emo-present/choice.rs`→`emo-text/choice.rs`）で実体の消失 0・行番号ドリフトは常在。status-execution-states の本文陳腐化（`choosing` 実導出済み）は追記が補正済み。`ukadoc-coverage-roadmap` は**両段とも前提充足**（台帳 4 本＋`report/summary.md` 実在・e2e 完了）だが brief の `report.md` は実在パス `report/summary.md` と食い違う（要件で合わせる）。③**XL 3 本を分割＝新規 brief 6 本**（`sylphya-set-ledger`・`property-ipc-transport`・`balloon-lifecycle-events`・`emo-text-canon-residue`・`text-align-shadow-canon`・`balloon-font-descript-keys`）・分割元 3 本＋`zorder-property`・`currentghost-property-tree` に登記＝28 本。④**三重所有の仮裁定**（案 甲）。⑤**roadmap 減量**＝旧ゴール表・旧ウェーブ行（⓪・W12 A/A′/B/C・旧 W13〜W15）・旧干渉台帳・旧 M2 節・追記(88)〜(94) 全文を history へ（77.8KB→約 3 分の 1）・完了サマリを 1 表に統合・spec 台帳を新設（状態列＝`/kiro-complete` の更新先）。⑥**W13〜W17 の編成**（旧「W13 裁定枠 D〜G」＋旧 W13〜W15 を、バグ→依存ツリー長→その他の順で 5 ウェーブへ・W13 は 9 本で共有ファイル 0）。⑦**直接修正候補の再確認**＝clippy は解消済み・`writing.rs` 文言と `\w[n]` 文書は未解消（briefing-sakura-script.md へ注記を追加・`briefing-property.md` の `roadmap.md:111` 行参照を節名参照へ是正）。

**2026-09-12 追記(96)（`nar-install` 起票＝検体の保管形式を `.nar` へ・M2 の NAR エンジンを開発側の駆動で先に建てる）**: `/kiro-discovery` 再入（開発者「emo2 などを常にインストール（`*.nar` を展開）して実行環境を準備する形式へ移行する spec があるべき。リポジトリには `vendors/sample_ghost/emo2.nar` だけを置き、インストール展開を行うクレートを経由して実験環境を作る」）。①**単一 spec で確定**（開発者「NAR を解釈したい・NAR インストールしたい・NAR インストールによる試験環境構築スキームが欲しいは一度に対応した方がよい」）。②**M2 予約「NAR」との関係を整理**＝予約は**製品機能**（利用者が投げた `.nar` の受け入れ）、本 spec は**開発側の駆動で建てるエンジン**。仮裁定 2「M2 予約群は brief を起票しない」に反しない（予約そのものではなく、その下に敷く資産を先に建てる）。M2 でエンジンをそのまま昇格＝二度実装しない。③**実測 3 件**——検体パスの参照は **38 か所**でうち **26 が同じ本体をコピペした `emo2_root()`**（共有ヘルパは 0）・**本番コードの参照 0**（全て test／example）・`emo2_boot/spine.rs` の `emo2_root()` が**呼ばれるたびに起動記録を `remove_dir_all` している**（汚染を 1 か所だけ手で拭っている。他 37 か所は拭っていない）。④**検体が走行で汚れる問題**＝`profile_areka_root` 配下の `sylphya.toml` に `[boot] count` が入り、**起動記録の有無で SHIORI のイベント列が変わる**（初回 `OnFirstBoot` / 2 回目以降 `OnBoot`）。展開方式にすれば走行ごとに新品になり構造的に消える。`shell/master/profile/` は `.gitignore` に**入っていない**（配線は通っており、シェルスコープの commit が初めて走った日に追跡外ファイルが湧く）ことも同時に解消。⑤**依存の実現性を実走で確認**＝`zip 8.6`（MIT・`default-features = false, features = ["deflate-flate2"]`）で新規は `typed-path` 1 本のみ・`cargo deny --offline check licenses bans sources` 緑・重複警告は 7 件のまま。**`tech.md` への「意図的依存追加」の登記と開発者の承認が要る**（`encoding_rs` と同じ扱い）。⑥**罠 2 件を brief へ登記**＝`ZipArchive::extract()` は RUSTSEC-2025-0168 が住んでいた API かつ名前ベース（CP437）なので**呼ばない**／`zip` は **Shift_JIS の経路を持たない**（ビット 11 未設定は CP437）ため `name_raw()` ＋ `encoding_rs` で自前復号する。⑦**編成は単独枠**（38 ファイル・9 クレート＝全ウェーブと共有ファイル発生・並走不可）。⑧配置の当座の裁定＝検体は**配布形のまま**置く（ukadoc「ゴーストにバルーンを同梱する場合」の正典形で、`emo2/install.txt` がそう宣言している）。インストール済み形は展開器が `target/` に作る＝保管形と消費形の両方の利点を取る。
