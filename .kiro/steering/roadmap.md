---
inclusion: manual
updated_at: 2026-08-22
---

# Roadmap — areka M1（最小 SSP 互換ベースウェア）

> **このロードマップは M1 のみを扱う。** M2 以降は **M1 完成後に実物を見て組み直す**（憶測で先に書かない）。
> 正本配置: 本ファイルが M1 ロードマップ正本（`.kiro/steering/roadmap.md`）。`focus.md`（`inclusion: always`）から辿る。設計判断の正本は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md)。M1 実物スコープは [doc/emo2-conformance-scope.md](../../doc/emo2-conformance-scope.md)。
> **履歴**: 追記①〜(51)・M-boot 完了詳報等は 2026-07-31 棚卸④で、W5 完了詳報・旧ウェーブ表・旧干渉台帳・追記(53)(54)(56)(57) 全文は 2026-08-01 棚卸⑤で、**W6 行全文・W6 完了詳報バレット（col/vis/scg）・追記(58)(60)(61)(62)(63)(64) 全文は 2026-08-14 棚卸⑧で**、**W6 完了詳報バレット全文・旧 W6/W6.5 行全文・旧干渉台帳・追記(65)(66)(67)(68) 全文は 2026-08-15 棚卸⑨で**、**旧 W6.75 行全文・旧ゴール表 atom 行・追記(69)(75) 全文は 2026-08-22 棚卸⑩で**、いずれも [roadmap-history.md](roadmap-history.md) へ退避（history が全文正本・既知の記録欠陥〔㊻番号衝突・**(55) 欠番**〕も history 冒頭と棚卸⑤節に注記）。完了ユニットの実装詳細は各 `completed/` spec が正本。

## M1 ゴール

areka（**x64**）が最小 SSP 互換ベースウェアとして、適合対象ゴースト **emo2**（作者自作・脳=`pasta.dll`・**32bit SHIORI**）を「**そのまま**」起動→会話→撫で→メニュー→終了まで E2E 実走させる。

- emo2 が動く＝同じ汎用 32bit ブリッジで里々/YAYA も動く土台（互換＝普及の入口）。
- 「伺かっぽいマスコット」ではなく「**伺か互換系**」であること自体が長期ロードマップの起点。
- M1 スコープは emo2 が実際に使う機能で**実物定義**。完全網羅・予測実装はしない。

## 実装規律（balloon-system の失敗から得た正）

- **実装ファースト**: 各作業ユニットの成果物は「emo2 が実際に動く」検証済みコード。
- **spec 工場の禁止**: 成果物が子 spec になる構造を作らない。1ユニット＝1かたまりの動く振る舞い。
- **最小実装＋薄い拡張シーム**: emo2 が使う分だけ実装し、拡張は型/レジストリの口だけ残す。抽象は「2例目の実物」が要求してから。
- **動く資産から建てる**: ゼロから再アーキテクトしない。
- **粒度基準**: 1ユニット＝単一 pass/fail の独立観測。純粋層は fixture/mock 直入力で切る。**UI 位置決め・座標系は逆＝本番ゴースト（実 emo2）＋実 DPI（≠96）が観測条件**（dpi=96 の自己整合が欠陥を隠す・記憶 areka-placement-real-ghost-first）。
- **⚠️ 語彙完備・配線ゼロの追跡**: 語彙先送りの規律は効いているが配線の追跡が弱い（実例 5 件: `compose_into`・`ZOrder`・`PlacementRoute::SpawnInitial`/`Restore`・`capture_logs` 複製増殖）。先送りシームには狭い `#[allow(dead_code)]`＋実在理由の doc を義務付け、消費者ゼロの検出は棚卸の定期監査項目とする。

## アーキテクチャ横断原則（要約・詳細は history＋記憶＋completed spec）

- **シェル/バルーン統一**: 描画エンジンはシェルとバルーンを区別しない。バルーン＝surface 上の文字レンダリング層。element は他サーフェス参照可（入れ子・再帰合成）・配置は D2D 変換行列が内部表現。
- **アニメエンジンは2つ**: ①さくらスクリプト再生（talk timeline・sakura）＋②SERIKO ループ（seriko）。両者とも dola（絶対時刻台本・duration 権威・CuePlayer/CueSink）上。テキストは①から emo-text へ直接。
- **並行モデル**: 各エンジン＝チャンネル通信のアクター＋独立スレッド。**render/window は UI スレッド固定**。機構=areka-actor／経路=kanade／結線=ghost の責務三分。
- **emo 合成**: 自前コンポジタ（アトラス→1枚物ビットマップ合成→wintf へ完成品のみ）。emo=UI 層全般（合成・マウス/さわり・バルーン文字・選択肢）＝「見える・触れる」の窓口。
- **DPI 追従が基本設計**: k=monitorDPI÷author_dpi で全表示経路がスケール（W4 で実装済み・SSP と別思想・k=1.0 は途中状態）。**キャラ窓の原点は下端中央（足元の中心）**——**キャラ窓位置の保存/復元/resize の三層**で統一済み（Bottom 限定）・左上基準で計算しないこと。**⚠️ バルーン追従は例外＝「窓（char 左上）相対」**（2026-07-31 実機裁定・`kero-balloon`）——全アンカーで `balloon_pos − char_pos ≡ offset` 不変・リサイズで offset を補正しない・保存も生 offset。旧記述「四層で統一」は誤り（詳細は history）。

**エンジン固有名**（コード/spec/会話の参照はこの名で統一・詳細は記憶 areka-engine-names）:

| # | エンジン | 固有名 | # | エンジン | 固有名 |
|---|---|---|---|---|---|
| ⓪ | ゴーストエンジン（最上位 owner） | `ghost` | ④ | さくらスクリプト再生 | `sakura` |
| ① | SHIORI 通信層 host-32 | `shiori` | ⑤ | SERIKO アニメ | `seriko` |
| ② | parser/loader | `parsers` | ⑥ | render（surface 合成＋UI 層） | `emo` |
| ③ | conductor（SHIORI イベント循環） | `kanade` | | | |

## 完了サマリ（2026-08-22 時点・詳細は completed spec ＋ history）

- **耐力壁突破**（2026-07-01 `pilot-shiori-host-32`）: x64→32bit pasta.dll 駆動 GO。
- **M-boot 23/23 完了**（2026-07-13 `emo2-boot`）: 起動→表示→talk→close の可視一周。①shiori・②parsers トラック全完了。
- **増分ウェーブ W1〜W4＋割込 全完走**: W1〜W3（idle-talk／collision-geometry／sakura-dialogue-tags／input-events／mayuna-compose／sylphya／seriko-loop／choice-render）→ 割込（wintf-gpu-test-crash）→ W4（position-persist ∥ choice-interact ∥ emo-dpi-scaling＝DPI 追従 k の全表示経路適用）。横断: cue-playback-duration・surface-resize-resnap・balloon-face-cue・emo-text-viewbox ✅。
- **W5 完了 4/4**（col は W6 へ編入）: `choice-select-events` ✅（07-31・**M-dialogue 4/4 完走**・実 DPI 120 人間サインオフ）・`kero-balloon` ✅（08-01・PR#97・**SSP 裁定 2 件**＝`windowposition.x` 符号非反転／**バルーン追従は窓相対**）・`dpi-window-vanish` ✅（08-01・PR#98・S1〜S4 是正・実機全判定 PASS・確定台帳の裁定付き訂正運用が実例確立）。詳報バレット全文は history。
- **W5.95 完了**: `file-slimming` ✅（08-10・PR#103）＝ソース肥大の全域是正。**最大 8,472→986 行・1,000 行超 54→0 本・ファイル内テスト 500 行超 49→0 本**・三不変量（テスト総数 4,790 不変・本文一致 61/61・対応表全単射）成立。一度 GO を撤回して群 8 を追加した経緯は `completed/areka-P0-file-slimming/verification/notes.md` §39/§54。
- **W6 完了 5/5**（2026-08-05〜08-13・**完走**）: `collision-dpi-hittest` ✅（PR#100・当たり判定 ÷k＝丸め権威 `unscale_coord`・実 DPI 2 水準サインオフ）・`balloon-visibility` ✅（PR#106・可視性を presenter 第一級概念へ＝空バルーン根治・残件は `balloon-canon-residue` へ送付済み）・`bindoption-exclusivity` ✅（PR#105・表情固着根治＝bindoption 3 値正典＋保持コマ掃除 3 段・`mayuna-compose` R4.5/D11 覆し）・`ghost-window-zorder` ✅（PR#107・案 A Win32 owner 構造保証・topmost 帯引き込みは `HWND_TOP` へ是正）・`scope-chain-gap` ✅（PR#108・P2 幅差隙間根治＝SSP 実測 8 本正典＋一度きりの `finalize_chain_once`）。詳報バレット全文は history（棚卸⑨退避）。
- **W6.5 完了 3/3**（**完走**・2026-08-14〜08-15）:
  - `recompose-budget` ✅（08-15）＝**常駐アイドルの CPU 消費の決着**。要件 3.1「定常アロケーション完全ゼロ」を実機で成立（4 発生点すべて 0・長時間 1503 適用/1425 秒でも 0）。**1 コマ適用 22,210 → 1,240µs（18 分の 1）**・p95 77,880 → 7,152µs・**CPU 24.9% → 約 11%**。四段＝定常アロケーション 0／冗長ゼロ埋め除去／リサンプル計算の作り直し（乗算 24→12・範囲検査 16→4・恒等性は横方向 42.9 億組の**全数検査**で確定）／**キャッシュ容量 1 → 3・LRU（開発者裁定・上流 R4.1 まで追随改訂）**。**メモリリークと負荷の単調上昇はいずれも実測で消滅**（Private 12→24 分で +0.1MB・回帰の傾き +0.84 → −0.045 %/分）。**ビルド設定は裁定済＝`opt-level='z'` 据え置きが正しい**（是正後は O3 が 26〜33% *遅い*）。⚠**最大の成果は「どこを削っても無駄か」を確定させたこと**——`apply_show` は着手前でも CPU の 10.4%・現在 3.3% しか占めず、真の最大項は `try_tick_world` が 13 スケジュールを 120回/秒 全部回していること（1 tick 578µs・**tick の 98% は表示に変化なし**）。**SSP との同一手順比較を初実施**（areka 10.97% 対 SSP 3.05%）し、要件 4.4 の出所不明だった較正値に初めて裏を取った。残る未達（⑵ 進行境界スキップ・⑷a CPU）は **`draw-load-parity`（W8・優先度低）へ引受け済み**。
  - `scale-exact-rational` ✅（08-14・PR#110）＝**f32 供給面寸の +1 を許容する裁定の登記**。厳密化（`ScaleRatio` num/den の文字層配管）は**却下**され、着地物は⑴既知欠陥登記を裁定済みへ書き換え⑵「寸法演算に f32 を使わない」絶対規則の 4 箇所へ唯一の例外を明記⑶前提の決定論テスト⑷下流申し送り。**実行時の挙動は不変**——製品コード 3 ファイルから doc 行を除くと main と byte 一致（式・署名・use・属性の不変が構造として成立）。裁定の土台を檻に入れた実測: 到達 23 比 × 寸 1..=1200 ＝ 27,600 組で差は常に 0 か 1（**−1 は 1 件も出ない**＝文字が切れる方向に転ばない）・差 1 は 162 件＝**6/5 と 12/5 で各 81 件**・残る 21 比は 0 件。正体は「1.2 の f32 表現」一点。**下流の宿題**: 適合 e2e は供給面寸の判定に **+1 許容**が要る（窓 client 寸は丸め権威経由ゆえ従来どおり絶対値・両 brief へ追記(68) で申し送り済み）。
- **W6.75 `dpi-transition-atomicity` ✅**（08-22・**PR#114**・**残 `balloon-offset-dpi`**）＝**DPI 遷移中の窓の跳ねの是正と、その機序の確定**。逐次適用は消えた——1 遷移で 4 窓の書込が散らばる幅が **93〜158ms → 40〜101µs**（約 1,500〜2,000 分の 1・B-2a 合流＋B-2b `DeferWindowPos` 一括）。決定論 8 遷移すべて PASS（同一フレーム・窓ごと 1 回・経路 A 0・接地点差 0・連鎖 1 回・随伴の同一フレーム性）。**実機サインオフは `ATOM-SIGNOFF: FAIL` のまま開発者裁定 GO で閉じた**——合否は書き換えていないので「FAIL だが GO」と読むこと。未達は µs 2 系統（`visualize_to_write_us` 210,329〜306,301µs＝上限 16,667µs の 12.6〜18.4 倍）に限られ、引受先は `present-write-coherence`（W8）。**37 タスク**（うち **6.5 と 7.5 は検証・最終ゲートが本番の欠陥を掘り当てて新規起票**——どちらも「整合ゲートの守備範囲の列挙に穴がある」同型で、当日どのテストも赤にしていなかった）。恒久資産＝既定 OFF の観測チャネル `wintf::transition`（10 種のレコード）・実機ログの機械判定ランナー・サインオフ手順書。**マージ時に main の Bevy 0.19 が `ExecutorKind` を撤去しており、本 spec の檻 5 本を実行器 API の移行で追随させた**（`get_executor_kind` に後継が無いため、空振り防止の番人は構築側の字面検査へ移した）。
- **実機サインオフ発見 7件中 #1〜#6 解決済み**。**#7（冒頭空行）のみ pasta 上流（`ekicyou/pasta` 起票済み）＝areka スコープ外・未解決**。
- 完了 spec 直下エントリ = **160**（`.kiro/specs/completed/` 直下＝ディレクトリ 159 ＋ 単体ファイル `graphics-rendering-stability.md`・2026-08-22 実測＝`dpi-transition-atomicity` の完了で +1）。計数は**直下エントリ数**で行うこと（ディレクトリ数だけ数えると 1 ずれる）。

## M1 残工程ゴール表（2026-08-22 棚卸⑩・完了行は完了サマリへ集約済み）

| 種別 | ゴール（単一文） | ユニット | ウェーブ |
|---|---|---|---|
| 基盤 | 檻の決定性（毒化サイトの全面硬化〔インベントリは両方向ドリフト中＝cage brief 棚卸⑩が最新スナップショット〕・ハーネス 2 設計の一本化・注入シーム） | `test-cage-determinism` | **W6.9**（**次ウェーブ・dlp と 2 本並走**・これ以上後送しない） |
| 性能 | 描画・フレーム駆動の負荷を SSP 同等圏へ（旧実測 CPU 3.6 倍＝Bevy 0.19 前・要再計測） | `draw-load-parity` | **W6.9**（**次ウェーブ・cage と 2 本並走**・2026-08-22 開発者指示で前倒し） |
| 見た目 | 遷移中に絵と窓が同じ提示フレームで揃う（要件 4.2 の実機側・可視化→書込の隙間 0.21〜0.31 秒） | `present-write-coherence` | **W6.95**（bod と 2 本並走・cage の後＝追記(75) 直列裁定を充足） |
| 挙動バグ | DPI 遷移時の `BalloonFollow.offset` スケール意味論確定（キーワード中央揃えの遷移後ずれ解消を含む＝atom D10 裁定の実装側） | `balloon-offset-dpi` | **W6.95**（**開発者指示 2026-08-22＝優先度低**・pwc と 2 本並走・**e2e より後ろへは送れない**〔適合 #1 が前提〕・追記(70) が申し送り正本） |
| M-e2e | 適合14項目一周＋DoD＝**M1 完成宣言** | `emo2-conformance-e2e` | **W7**（最終） |

> 完了済みマイルストーンのゴール表は history 参照。M-dual は退役＝e2e 適合 #10 へ吸収。

## ウェーブ編成（着手順の正本・2026-08-22 棚卸⑩改訂）

> 各ウェーブは**フルライフサイクル**（要件→設計→タスク→実装→`/kiro-complete`＝PR squash マージ）を完走してから次へ。並走はウェーブ内のみ（1 spec = 1 worktree = 1 PR）。同居は**実測で共有ファイル 0**が原則。文書フェーズ（要件・設計）は先行可＝先行 spec はウェーブ開始時に settled main へ再突合。優先順位: **挙動バグ → 依存ツリーが長く早期着手が効くもの → その他**。詳細な申し送りは各 brief の追記ブロックが正本（roadmap は編成と条件のみ持つ）。

| Wave | ユニット | 開始コマンド | 編成根拠・条件 |
|---|---|---|---|
| W1〜W6.75 ✅ | （W5=4/4・W5.95=slimming・W6=5本・W6.5=budget ∥ exact ∥ wpl・W6.75=atom。完了サマリ参照） | — | 詳細は history（旧 W6/W6.5 行全文は棚卸⑨・旧 W6.75 行全文は棚卸⑩退避）。**生存する申し送り**: ⑴**⚠ vis は zorder の再表示シーム `ReassertZOrder` を消費せずに着地**（再表示直後のバルーン隣接は実機未確認）——**e2e／cage で拾うこと** ⑵配置系 spec は `window-placement` R2.9 を正典として引用しない（正典は COMPAT §8 経由で scg へ一意に辿る） ⑶**atom の実機未達 2 系統（µs）は pwc（W6.95 へ前倒し）が引受け済み**・atom→後続の申し送りは追記(70)〜(78) として各 brief に登記済み（台帳参照） |
| **W6.9**（次ウェーブ・2 本並走） | `test-cage-determinism` | `/kiro-start areka-P0-test-cage-determinism` | **dlp と並走（2026-08-22 開発者指示＝並行数最大化・棚卸⑩補で同居成立）**。同居裁定: **`command.rs` は丸ごと dlp 所有・本 spec は非接触**（`SELF_INITIATED_DEPTH` の `Cell<i32>` 化は dlp が flush 接触のついでに実施〔dlp brief 追記(74)⑹「ついでに片づくなら安い」〕・本 spec は錠 `lock_self_initiated_for_test()` の退役だけを dlp 着地後の rebase で受ける）＝**実測共有ファイル 0**。`presenter/show.rs` `apply_show` 鎖（budget→atom→④）の**最後尾**。vis 先着（推奨）は充足済み。着手時に vis/exact/budget/atom/bod の実形へ rebase・毒化インベントリの**全面再計数必達**——**2026-08-22 棚卸⑩実測で両方向ドリフト**: atlas/compose の `log_capture.rs`・seriko `actor_test_support.rs`/`looper_tests.rs` は**域外で硬化済みへ転じた**一方、**未硬化ヘルパ定義が 10 ファイル**（slimming 分割由来＋atom 新設 `frame_test_support.rs`・`frame_chain_finalize_tests.rs`・`talk_lifecycle_tests.rs`・`balloon_test_support.rs`・`choice_drain.rs` 等＝「後置するほどコピーが増える」の 3 度目の実証・詳細は cage brief 棚卸⑩ブロック）。④は `#[cfg(test)]` fault フラグ小案を第一候補・観測点＝upload エラー分岐 **:306-310**（fmt PR#115＋atom 後・2026-08-22 実測・旧 :297-301 失効。atom は同分岐を意図的に不動と登記済み＝追記(72)⑸）。atom からの申し送り＝**追記(72)(76) が brief 登記済み**（`wintf::transition` 語彙不変条件・決定論テストは flush 不到達・(76)⑹ `SELF_INITIATED_DEPTH` は**同居裁定により dlp 実施へ移管**＝左記）。bind が登記した info ログ檻 3 本＋既存間欠赤 1 本（`bind_default_exclusive_replace_emits_show_and_info_marker`）も担当クラス。W6 申し送り⑴（`ReassertZOrder` 未消費）は檻で拾える範囲を検討。**追記(79) の裁定が⒜（行数番人テスト）に決まった場合の置き場候補は本 spec** |
| **W6.9**（次ウェーブ・2 本並走） | `draw-load-parity` | `/kiro-start areka-P0-draw-load-parity` | **cage と並走（2026-08-22 開発者指示で W8 から前倒し**——旧「後日別セッション」裁定を上書き。M1 完成を妨げない位置づけ自体は不変**）**。同居裁定: **`command.rs` は丸ごと本 spec 所有**＝`SELF_INITIATED_DEPTH` の `Cell<i32>` 化（:49・追記(74)⑹）も本 spec が flush 接触のついでに実施し、着地形を cage へ申し送る。budget が「削るべき対象は自分の境界の外」を実測で示したため分離起票。**apply_show は CPU の 3.3% しか占めない**（着手前でも 10.4%）ので、`presenter/show.rs` は対象外。真の最大項は **`try_tick_world` が 13 スケジュールを 120回/秒 全部回していること**（1 tick 578µs・壁時計 6.85%・**tick の 98% は表示に変化なし**）。内訳は FrameFinalize 182µs(31.5%)＋Draw 143µs(24.8%) で 56%。**wintf 中核（フレーム駆動）に手を入れる spec** なので、クリック透過・αマスク追随の「毎フレーム評価」前提（`runtime/mod.rs:231-237` が R2.4 として明記）と正面から調停が要る。**⚠ 比較の前提に未検証の穴**——SSP は 100% 描画→200% 引き伸ばしの可能性が高く（開発者観察＝文字がぼやける）、事実なら画素の仕事量が 1/4 で「3.6 倍」は画素あたり 0.9 倍＝互角。**目標を絶対値で置くか画素あたり効率で置くかは要件段階の裁定事項**。atom は着地済み＝着手時に atom 実形（`DeferWindowPos` 一括 flush・`wintf::transition` 観測チャネル）へ rebase・cage が登記した `SELF_INITIATED_DEPTH` 是正（`command.rs` 1 行）と着手順調整。**⚠ brief の全性能数値（try_tick_world 578µs・13 スケジュール等）は Bevy 0.19/Taffy 更新（2026-08-19 `bf2d7950`・`ExecutorKind` 撤去を含む実行器改稿）より前の実測**＝要件段階の再計測は元々必達だったが、実行器そのものが変わったため**傾向すら持ち越せない前提**で臨むこと（2026-08-22 棚卸⑩登記） |
| **W6.95**（2 本並走） | `present-write-coherence` | `/kiro-start areka-P0-present-write-coherence` | **bod と並走（2026-08-22 開発者指示で W8 から前倒し・「cage の後」条件は W6.9 完走で充足**——「大改造が必要なら無理に治さなくて良い」裁定は不変＝要件段階でまず規模を見積もり、tick 大改造に及ぶなら見送りも正**）**。bod とはファイル素（本 spec＝`presenter/show.rs` 可視化の段・bod＝`placement/follow` 系）——B-4 を採る場合のみ `mount.rs` 配置契約で bod の関心事に意味論上近接（要ウォッチ・下記台帳）。atom が要件 4.2 を**決定論では満たし実機では満たさずに**閉じたぶんの引受先。出発点は atom 設計 **C8 の B-3（可視化の 2 相化・第一候補）／B-4（窓内下端中央補償・緩和）** で、候補表の外へは広げない（atom 要件 3.4 を継承）。**実測の起点**（`atom-73-signoff-1`・8 遷移・全遷移を走査した値）: `visualize_to_write_us` **210,329〜306,301µs**（上限 16,667µs の **12.6〜18.4 倍**・違反 32 件）／`flush_total_us` 143,231〜231,910µs（同 8 件）＝**実機専用系統の違反は計 40 件**。**B-2b は隙間を縮めなかった**（`flush_total` 平均 192,247→**188,711**µs＝**−1.8%**・OS 側が過半＝L7）。**窓ごとの隙間はむしろ +27% 伸びた**（全窓の書込がバッチ末尾へ揃った帰結・台帳 §11.6）。接触面は `presenter/show.rs` の可視化の段（`apply_show`:46 の末尾＝`set_visible`:375／`set_bounds`:381／`Visualize` 発行:392）。**cage（W6.9）の後**に置く（同じ `apply_show` 鎖を触るため）。B-4 を採る場合は当たり判定の原点（`collision-dpi-hittest`）と `mount.rs` の配置契約に触れるので atom 要件 10.1 の再確認が要る。**tick 構造の大改造に及ぶなら atom 要件 9.3 に従い分割を再裁定する**（要件段階でまず規模を見積もること）。判定器・観測語彙・サインオフ手順書は atom の着地物を流用（新設不要） |
| **W6.95**（2 本並走） | `balloon-offset-dpi` | `/kiro-start areka-P0-balloon-offset-dpi` | **pwc と並走（2026-08-22 開発者指示＝優先度低のため W6.8 単独案を撤回し本ウェーブへ後送。ただし e2e の前提〔適合 #1 DPI 検証は追従込み〕なので e2e より後ろへは送れない）**。atom D10／要件 6.5 の裁定に従う実装側: キーワード基本位置は DPI 遷移で再導出しない・`BalloonFollow.offset` を k 倍で追随（両者排他・**申し送り正本＝bod brief 追記(70)＋棚卸⑩ブロック**）。決定論行列に「キーワード由来 offset が k 倍で中央揃えを保つ」ケース必達。rebase 必達: U4 doc＝**`follow/window_move.rs:24`**・`windowposition.rs` 単位混在 doc＝**:191-194**（2026-08-22 実測）。**cage 後着の利得**: 檻は一本化済み共有ハーネスで書ける（import 追随の手戻りが消える）。SSP オラクル観測（モニタ跨ぎバルーン挙動）は要件 research——**無ければ areka 設計原則から導出し COMPAT へ「areka 裁量」登記**（裁定密度の頂） |
| **W7** | `emo2-conformance-e2e`（最終） | `/kiro-start areka-P0-emo2-conformance-e2e` | 全ユニット完了後＝適合14項目（#1 DPI 検証は追従込み・バルーン表示ライフサイクル追補・#3 は bind・#10 は ker が前提充足）の一周走行→**M1 完成宣言**。着手時義務: brief 全面再監査・㉘(E) の実機判断・#7（pasta 上流）は M1 完成を妨げない扱いの確認・**W6 申し送り＝再表示直後のバルーン隣接（`ReassertZOrder` 未消費）の実機確認** |

**干渉台帳（生存ペアのみ・2026-08-22 棚卸⑩補で並行数最大化編成の実形へ再解決・旧全文は history）**:
- **cage⇄dlp（W6.9 同居ペア）**〔唯一の共有候補＝`command.rs`。**同居裁定（棚卸⑩補）: `command.rs` は丸ごと dlp 所有・cage は非接触**——`SELF_INITIATED_DEPTH` の `Cell<i32>` 化（:49・追記(74)⑹/(76)⑹）は dlp が flush 接触のついでに実施し着地形を cage へ申し送り、cage は錠 `lock_self_initiated_for_test()` の退役だけを dlp 着地後の rebase（または wave 内合流）で受ける＝**実測共有ファイル 0 が成立**。第二次接触＝cage の共有 leaf crate が各 crate の Cargo.toml dev-deps へ 1 行ずつ追加（dlp は Cargo.toml 非接触の見込み・軽微）〕
- **bod⇄cage**〔cage③ が正典ハーネス `placement/test_support.rs` を共有 crate へ改組・bod の新設檻が同ハーネスの消費者。**編成で直列化済み（W6.9→W6.95）＝cage 先行に反転**（棚卸⑩補）: bod は最初から一本化済みハーネスで檻を書ける＝import 追随の手戻りが構造的に消えた〕
- **cage⇄pwc**〔同じ `apply_show` 鎖（cage④＝upload エラー分岐 :306-310・pwc＝可視化の段 :375-392）。**編成で直列化済み（W6.9→W6.95）**＝pwc は cage の後（追記(75) 登記済み・充足）〕
- **pwc⇄bod（W6.95 同居ペア）**〔ファイル素（pwc＝`presenter/show.rs`／`mount.rs`・bod＝`placement/follow` 系＋`windowposition.rs`＋`persist.rs`）。**要ウォッチ**: pwc が B-4（窓内下端中央補償）を採る場合のみ `mount.rs` 配置契約・当たり判定原点で bod の関心事（バルーン相対位置）に意味論上近接——両 spec の design 時に相互の要件を照合すること〕
- **dlp⇄pwc**〔両者とも提示タイミングの軸に触れる（dlp＝tick 駆動・pwc＝可視化順序）。**編成で直列化済み（W6.9→W6.95）**＝pwc は dlp の着地した tick 実形の上で規模を見積もる〕
- **show.rs アンカー（2026-08-22 実測・fmt PR#115＋atom 改稿で全面ドリフト）**: `apply_show` :46 起点・budget 域（compose/resample/mask/insert）:97-173・upload エラー分岐（cage④）**:306-310**・atom 観測点 :347-359／:389-399・可視化の段（pwc）`set_visible` :375／`set_bounds` :381／`Visualize` 発行 :392。旧 :43／:95-170／:280-330／:297-301 は全て失効
- **退役（2026-08-22 棚卸⑩・atom 完走により後続 spec の design 前 rebase 義務へ転化）**: atom⇄bod（bod は atom 実形へ rebase・正本＝bod brief 追記(70)＋棚卸⑩ブロック）・atom⇄dlp（dlp は atom 実形へ rebase・正本＝dlp brief 追記(71)(74)(78)）。棚卸⑨退役分（budget⇄atom・cage④⇄budget・atom⇄wpl・wpl⇄bod・exact→bod）は history 参照
- `status-execution-states`=台帳 spec（着手しない・源着地時に just-in-time）・`surfaces-basepos`／`sakura-time-directives`／`balloon-canon-residue`=**M2 解禁ゲート**（M1 では着手しない）

## 着手手順

> **brief 全数完備体制**: M1 残ユニット 5 本（bod・cage・e2e・dlp・pwc）＋M2 ゲート 4 本＝全 9 本 brief 済み＝着手は該当 brief を読んで `/kiro-start <unit>` へ直行。新規課題の起票は `/kiro-discovery`（再入）で brief just-in-time 生成。`/kiro-spec-batch` は使わない（一括＝工場化）。ウェーブ跨ぎの合流判断は別セッションで一括（記憶 portfolio-convergence-decided-in-separate-session）。

## 制約

- Rust 2024・マルチクレート（wintf/dola/areka ＋ `areka-parsers` ＋最小依存 `shiori-abi` ＋ host-32 3クレート `shiori-host32-ipc`/`-host`/`-helper`）。
- **32bit 可搬性の適用範囲＝host-32 系（`shiori-host32-*`／`shiori-abi`）のみ**。wintf/areka 本体は x64＋arm64 ネイティブ（i686 検証を本体 spec に課さない）。
- 透過は WUC/DComp GPU 合成上のクリックスルー機構（`WS_EX_TRANSPARENT` 動的トグル＋αマスク）で成立（ULW は撤去済み）。SHIORI 内部唯一 ABI=`IShiori`(COM, HSTRING/UTF-16)。過去互換は 32bit Rust ホスト。
- 設計判断の変更は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md) を正本として更新。
- 実機運転の定石: 絶対パス起動（相対は pasta.dll LOAD 失敗）・i686 helper を先ビルド・`AREKA_APP_SMOKE_EXIT_MS` 有界自動終了＋`RUST_LOG` grep（記憶 areka-real-machine-signoff-bounded-auto-exit）。

## M2 以降

**M1 完成後に、実物を見て組み直す。** 本ロードマップでは扱わない（pasta の native x64・`IShiori` in-proc 化、ベクトル描画・AI、**owner-draw 右クリック system メニュー（ゴースト管理 chrome）**、互換面拡大＝Shift_JIS/SAORI/里々・YAYA 網羅/NAR 等はその時に）。

**アプリ層の M2 予約（2026-07-05 ukadoc 裏付け・全て任意＝emo2 単体起動に不要）**: SSTP ポート（9801）ホスティング・FMO・DirectSSTP・Plugin/HEADLINE/SAORI ホスティング・ネットワーク更新（`\![update,platform]`・OnBasewareUpdating/Updated 系）・ゴースト/バルーン選択 UI（OnGhostChanging 系）・多重ゴースト運用。

**emo テキスト進化の予約（M2 候補）**: ①回転テキストの実挙動（M1 で行列変換領域を内部表現として持ち込み済み）②ポップアート級の文字装飾（text effects——1枚物自前合成ゆえ文字も合成レイヤ）。

**バルーン美観配置政策の予約（M2・2026-08-01 `dpi-window-vanish` task 6.2 が先送り登記）**: 画面端でのバルーン**左右反転**をはじめとする SSP 互換の美観配置政策。M1 が持つのは遷移ガードによる「完全不可視への遷移を防ぐ安全網」まで（*見えない会話*より*重なった会話*を優先する裁定）。**縮退シーム＝`[visibility-guard] ClampX` の `warn!`**（`route=BalloonFollow` 行）——安全網の発火回数が M2 着手時の優先度根拠。**M1 では起票しない**。

**M2 解禁ゲートの spec（brief 済・M1 では着手しない）**: `areka-P0-surfaces-basepos`・`areka-P0-sakura-time-directives`（互換拡充時に解禁）／`areka-P0-status-execution-states`（残状態の源サブシステム着地時に just-in-time・台帳）／`areka-P0-balloon-canon-residue`（balloon 正典残語彙 10 項目の受け皿＝kero-balloon 由来 6 件＋balloon-visibility 由来 4 件）。

---

**追記台帳（要約・全文は history）**: (51) W6.5 残件 2 spec 起票（07-30）／(52) 棚卸④＝roadmap 履歴分離・W6 に bind 編入・brief 補正 8 本（07-31）／(53) `recompose-budget` 起票＝`compose_into` 本番未配線・アイドル 1 コア 13〜22%（07-31）／(54) `ghost-window-zorder` 起票＝owner 無し・全 `SWP_NOZORDER`・「語彙完備・配線ゼロ」3 例目（07-31）／**(55) 欠番**（atom 起票の本体がマージで脱落・内容は atom brief＋(57)⑴ が正本）／(56) 未登記先送り棚卸＝孤児 17 件・新規 brief 4 本起票（08-01）／(57) van 完了＝S5 担当が atom で確定・確定台帳の裁定付き訂正運用が確立・檻の空虚性通算 9 例（08-01）／(58) 棚卸⑤＝W5 3本マージ後の全面棚卸・全配置確定・干渉行列 55 ペア再実測（08-01）／(59) col 実装中の申し送り＝cage brief ①インベントリを 95 呼出/12 モジュールへ拡大（08-05・PR#101・cage brief 内が全文正本）／(60) 棚卸⑥＝col マージ後の軽量アンカー監査・実害ドリフト 2 ファイルのみ（08-06）／(61) `file-slimming` 起票＝W5.95 新設・肥大の 7〜8 割は in-file 檻（08-06）／(62) 棚卸⑦＝slimming 後の干渉台帳全面再解決・緩和 3 ペア（08-11）／(63) `bindoption-exclusivity` 完了＝根因 2 層・mayuna R4.5/D11 覆し・25 分実機 PASS（08-11）／(64) `ghost-window-zorder` 完了＝案 A owner 確定・topmost 帯引き込み是正 `HWND_TOP`（08-13）／(65) 棚卸⑧＝W6 完走後の全面再解決・退役 5 ペア・W6 行/詳報/追記(58)(60)-(64) を history 退避（08-14）／(66) 開発者裁定＝性能最優先・budget を W6.5 へ前倒し（08-14）／(67) 開発者裁定＝atom 文書併走不採用・W6.5 着地後の単独直列へ（08-14）／(68) `draw-load-parity` 起票＝W8 新設・優先度低・SSP 同一手順比較初実施（areka 10.97% 対 SSP 3.05%・**⚠SSP は 100% 描画→引き伸ばしの疑い＝目標を絶対値か画素あたり効率かは要件裁定事項**）・真の最大項＝`try_tick_world` 120回/秒 全走・budget 判定式⑵（catch-up）も引受け（08-15）／(69) 棚卸⑨＝W6.5 完走後の全面再解決・W6.75 単独直列確定・退役 5 ペア（08-15・全文は history）／**(70)〜(78) は atom（W6.75）走行中に各 brief へ直接登記された申し送り群＝各 brief が全文正本・roadmap には一行のみ**: (70) atom→bod＝キーワード基本位置は遷移で再導出しない・offset k 倍は bod の責務（D10／要件 6.5・統合候補失効・08-21）／(71) atom→dlp・e2e＝窓書込指令の形と一括 flush の中身の変化（08-21）／(72) atom→cage＝観測チャネル `wintf::transition` 新設・窓書込檻の前提変化（08-21）／(73) atom→dlp・e2e＝一周走行で使える遷移観測チャネル増（08-21）／(74) atom→dlp＝一括 flush を `DeferWindowPos` 1 バッチへ移す確定（08-21）／(75) `present-write-coherence` 起票＝W8 に 2 本目・atom の実機未達 µs 2 系統の引受先（08-22・全文は history・pwc brief が正本）／(76) atom→cage＝(72) 続き・`in_batch` 追加・`SELF_INITIATED_DEPTH` 是正の登記（08-22）／(77) atom→e2e＝(73) 続き・「FAIL だが GO」の読み方（08-22）／(78) atom→dlp＝(74) 着地後実測・B-2b は隙間を縮めず −1.8%（08-22）。

**2026-08-22 追記(79)（1 ファイル 1,000 行の目安が漂流している——`atom` の最終ゲートで実測。開発者の裁定が要る）**: `.kiro/steering/structure.md:176` は「**1 ファイル 1,000 行以下の目安は本番ファイル・テストファイルの双方に適用する**」と定めており、`file-slimming`（W5.95・08-10・PR#103）が **1,000 行超 54→0 本**まで落として達成した。**その後、機械的な番人が 1 つも無いまま漂流している。** 2026-08-22 の実測（`crates/**/*.rs` 全数）:

| 行数 | ファイル | 出自 |
|---|---|---|
| 1,604 | `crates/areka-emo-present/src/cache_tests.rs` | **main に既存** |
| 1,330 | `crates/areka-seriko/src/actor_bind_loop_tests.rs` | **main に既存** |
| 1,255 | `crates/areka/src/emo2_boot/frame_transition_branch_tests.rs` | atom（新規） |
| 1,223 | `crates/areka/src/placement/follow/window_move.rs` | atom（**965 → 1,223**） |
| 1,076 | `crates/areka-emo-compose/src/plan_ops_tests.rs` | **main に既存** |
| 1,047 | `crates/areka-emo-present/src/presenter/budget_tests.rs` | **main に既存** |
| 1,039 | `crates/areka/src/placement/transition_judge_tests.rs` | atom（新規） |
| 1,037 | `crates/areka/src/placement/transition_judge_verdict_tests.rs` | atom（新規） |
| 1,033 | `crates/areka-seriko/src/bind.rs` | **main に既存** |

**要点は「atom が壊した」ではなく「既に 5 本破れていて atom が 4 本足した」ことである。** 5 本は本ブランチの分岐点（`git merge-base origin/main HEAD`）の時点で既に超えており、`recompose-budget`・`kero-balloon`・`bindoption-exclusivity` 等の後続 spec が積み上げたものである。**目安は「目安」と書かれており、テストも lint も守っていない。**

- **atom は自分の申し送りを消化しなかった**——`tasks.md` の「task 5.5 への申し送り: `window_move.rs` は 974 行で 1,000 行の目安まで残り 26 行・着手前にファイル分割が要る」は実行されず、裁定の記録も残らないまま群 5〜7 が進んだ。**最終ゲートまで誰も気づかなかった。**
- **atom の内側で今これを直すのは高くつく**——`window_move.rs` を割ると spec 文書 7 本に散る **41 箇所**の `window_move.rs:<行>` が全部動く。**file:line の陳腐化はこの spec が 4 度踏んだ失敗そのもの**であり、実機サインオフを閉じた直後にそれを自ら仕込むのは筋が悪い。ゆえに **atom の内側では割らず、数量つきで報告して裁定を仰ぐ**（本追記がその報告である）。
- **引受先は現時点で存在しない。** `file-slimming` は `.kiro/specs/completed/` にあり申し送りを消化できない。上の 9 本のいずれかを名指ししている**生存 spec は 1 本も無い**（`.kiro/specs/` の completed を除く全ディレクトリを 5 本のファイル名で走査して 0 件・2026-08-22 実測）。**「ウェーブ名は担当者ではない」の規律に従い、実在の引受先が決まるまで先送りとは呼ばない——本追記が唯一の記録である。**
- **開発者に問うべきは 3 択**: ⒜ 目安を機械で守る（行数の番人テストを 1 本置き、既存 9 本は例外表に載せて漸減させる。置き場所の候補は `test-cage-determinism`＝W6.9）／⒝ 掃除の spec を 1 本起票する（`file-slimming` 第 2 期）／⒞ 目安のままとし `structure.md:176` に「番人は無い・漂流は許容する」と明記して**期待値を実態へ合わせる**。**現状は「規則があるのに誰も測っていない」＝最も悪い形**である。

**2026-08-22 追記(80)（棚卸⑩＝W6.75 完走後の全面再解決・次ウェーブ＝bod 単独）**: `/kiro-discovery` 再入。前回⑨以降の main 差分＝atom 完走（PR#114・37 タスク・「FAIL だが GO」）＋ **spec 外の直接コミット群**（Bevy 0.19/Taffy 更新 `bf2d7950`〔08-19・`ExecutorKind` 撤去含む〕・wintf visual/transform2d 新設・全域 cargo fmt PR#115・kiro-complete 手順是正 PR#116）。①**brief 無し spec=0・新規起票 0・分割の新規事案 0**（残 9 brief 全実在＝M1 残 5〔bod・cage・e2e・dlp・pwc〕＋M2 ゲート 4・走行中 spec 無し・completed 直下 160 実測一致）。②**次ウェーブ＝W6.8 bod 単独**。厳しめ精査でも並走候補ゼロ: cage との同居は cage③ の正典ハーネス改組×bod の檻消費で import 面の実干渉＝却下（「少しでも干渉するなら分ける」）・e2e は全ユニット後・dlp/pwc は W8＝開発者裁定で M1 後。直列＝**bod（W6.8）→cage（W6.9）→e2e（W7）**が M1 のクリティカルパス。③**干渉台帳を atom 完走後の実形へ再解決**——退役 2（atom⇄bod・atom⇄dlp＝rebase 義務へ転化）・生存 3（bod⇄cage・cage⇄pwc・cage⇄dlp＝いずれも編成で直列化済み）。show.rs は fmt＋atom で全アンカードリフト（`apply_show` :43→:46・cage④ :297-301→:306-310・pwc の可視化の段 :375/:381/:392・2026-08-22 実測）。④**cage の毒化インベントリが両方向ドリフト**——atlas/compose `log_capture.rs`・seriko `actor_test_support.rs` 等は**域外で硬化済みへ転じた**一方、未硬化ヘルパ定義は **10 ファイル**（slimming 分割由来＋atom 新設 `frame_test_support.rs` 等の新顔 7 本）＝「後置するほどコピーが増える」の 3 度目の実証（機械判定スナップショットは cage brief 棚卸⑩ブロック）。⑤**dlp の全性能数値は Bevy 更新前の実測と判明**＝実行器改稿を跨ぐため傾向も持ち越せない前提を dlp 行と brief へ登記。⑥**追記番号の欠落を台帳へ整合**——(70)〜(78) は atom 走行中に各 brief へ直接登記された申し送り群で roadmap 台帳に一行も無かった＝一行要約を登記（各 brief が全文正本）。⑦**roadmap 減量**: 旧 W6.75 行全文・旧ゴール表 atom 行・追記(69)(75) 全文を history へ退避。⑧**追記(79)（1,000 行目安の漂流）は裁定待ちのまま**——⒜ 行数番人テスト（置き場候補＝cage）／⒝ file-slimming 第 2 期起票／⒞ 目安の明文緩和、の 3 択を開発者へ再提示（棚卸⑩の推奨は⒜＝cage の「檻の決定性」ミッションと同系で追加コストが最小）。

**2026-08-22 追記(81)（棚卸⑩補＝開発者指示による編成改訂・並行数最大化）**: 開発者指示 2 点——⑴ **bod は優先度低**（棚卸⑩の「W6.8 bod 単独」案を撤回）⑵ **次ウェーブの並行実施数をなるべく増やす**。厳しめ精査の再解決: **W6.9＝cage ∥ dlp（2 本並走）**——dlp を W8 から前倒し（旧「後日別セッション」裁定を上書き・「M1 完成を妨げない」位置づけは不変）。唯一の共有候補 `command.rs` は**同居裁定＝丸ごと dlp 所有・cage 非接触**（`SELF_INITIATED_DEPTH` の `Cell<i32>` 化は dlp が flush 接触のついでに実施〔(74)⑹ 想定済みの形〕・cage は錠の退役を rebase で受ける）で**実測共有ファイル 0 が成立**。→ **W6.95＝pwc ∥ bod（2 本並走）**——pwc の「cage の後」（追記(75)）は W6.9 完走で充足・bod⇄pwc はファイル素（B-4 採用時のみ `mount.rs` で意味論近接＝要ウォッチ）・**bod の cage 後着はむしろ利得**（一本化済みハーネスで檻を書ける）。→ **W7＝e2e 単独（最終・不変）**。**3 並走が組めない理由**: pwc は cage と同じ `apply_show` 鎖（直列裁定が正典）・bod は開発者指示で後送・e2e は全ユニット後・M2 ゲート 4 本は解禁条件未成立——残 5 spec で幅 2-2-1 が上限である。**W8 は解散**（2 本とも前倒しで M1 ウェーブ列へ編入）。**bod は e2e より後ろへ送れない**ことを明記（適合 #1「DPI 検証は追従込み」の前提＝優先度低でも W6.95 が下限）。干渉台帳は本編成の実形へ更新済み（生存 5 ペア・全て同居裁定または直列化で解決）。
