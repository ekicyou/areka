---
inclusion: manual
updated_at: 2026-08-11
---

# Roadmap — areka M1（最小 SSP 互換ベースウェア）

> **このロードマップは M1 のみを扱う。** M2 以降は **M1 完成後に実物を見て組み直す**（憶測で先に書かない）。
> 正本配置: 本ファイルが M1 ロードマップ正本（`.kiro/steering/roadmap.md`）。`focus.md`（`inclusion: always`）から辿る。設計判断の正本は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md)。M1 実物スコープは [doc/emo2-conformance-scope.md](../../doc/emo2-conformance-scope.md)。
> **履歴**: 追記①〜(51)・M-boot 完了詳報等は 2026-07-31 棚卸④で、W5 完了詳報・旧ウェーブ表・旧干渉台帳・追記(53)(54)(56)(57) 全文は 2026-08-01 棚卸⑤で、いずれも [roadmap-history.md](roadmap-history.md) へ退避（history が全文正本・既知の記録欠陥〔㊻番号衝突・**(55) 欠番**〕も history 冒頭と棚卸⑤節に注記）。完了ユニットの実装詳細は各 `completed/` spec が正本。

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

## 完了サマリ（2026-08-01 時点・詳細は completed spec ＋ history）

- **耐力壁突破**（2026-07-01 `pilot-shiori-host-32`）: x64→32bit pasta.dll 駆動 GO。
- **M-boot 23/23 完了**（2026-07-13 `emo2-boot`）: 起動→表示→talk→close の可視一周。①shiori・②parsers トラック全完了。
- **増分ウェーブ W1〜W4＋割込 全完走**: W1〜W3（idle-talk／collision-geometry／sakura-dialogue-tags／input-events／mayuna-compose／sylphya／seriko-loop／choice-render）→ 割込（wintf-gpu-test-crash）→ W4（position-persist ∥ choice-interact ∥ emo-dpi-scaling＝DPI 追従 k の全表示経路適用）。横断: cue-playback-duration・surface-resize-resnap・balloon-face-cue・emo-text-viewbox ✅。
- **W5 完了 4/4**（`collision-dpi-hittest` は W6 へ編入のうえ 2026-08-05 に着地）:
  - `choice-select-events` ✅（2026-07-31）＝**M-dialogue 4/4 完走**。メニュー一周を実 emo2・実 DPI(120) で人間サインオフ。
  - `kero-balloon` ✅（2026-08-01・PR#97）＝kero が正典どおり `balloonk*`・採寸 per-scope。**SSP 裁定 2 件**（`windowposition.x` 符号非反転／**バルーン追従は窓相対**＝position-persist の Bottom 限定補正を撤去）。
  - `dpi-window-vanish` ✅（2026-08-01・PR#98）＝混在 DPI 窓消失の決着（S1〜S4 是正・実機全判定 PASS・`S1-SOURCE-CUT` 84/84 陽性→0 へ反転・`ground_y` 定数）。`TEARDOWN-SILENCE` 確定原因の誤りを裁定経由で訂正（確定台帳の訂正運用が実例確立）。マージ時の ker との意味的衝突は **ker の裁定済み正典（窓相対）を採用**。
  - `collision-dpi-hittest` ✅（2026-08-05）＝**DPI 追従下の当たり判定 ÷k の決着**。丸め権威 `ScaleRatio::unscale_coord`（DD-1 画素中心逆写像・整数のみ）＋合成純関数 `hit_region_scaled` ＋ production 入口 `hit_region_client`（私有 `applied` 直読・f32 非経由）。SHIORI 配信座標を**サーフェス px** へ正準化（throttle は client px 維持）。**実 DPI 2 水準サインオフ合格**——125%(k=5/4・478×684) と 200%(k=2/1・764×1094) で**互いに異なる拡大寸**を実測し、`collision-geometry` Task 4.2 が 2026-07-18 に「k=1.0 固定ゆえ検証自体が成立しない」で却下された観測条件を初めて突破。本番 emo2 絶対パス起動で shell 実 k を確認（R-2 消化）＋開発者が撫で／さわり反応を直接目視。
  - `balloon-visibility` ✅（2026-08-13）＝**起動時の空バルーン出しっぱなしの決着**。可視性の所有権を presenter に第一級の概念として追加し、起動時は「不可視のまま確立」（文字の配置先・面・境界・k は確立し可視性だけを付けない）。表示は可視グリフ数の増加、非表示はゼロ下降とタイムアウト（既定 30 秒・占有終端起点）で、ドラッグ／ポインタ滞在／選択肢表示中は抑止。判断は純関数、配線は薄く保つ。**相順を固定する検査がリポジトリに初めて入った**（相を drain の前／reconcile の後ろへ動かすと別々の檻が単独で赤くなる）。**実機 2 回で開発者サインオフ合格**（DPI 192・3 会話サイクル・誤りログ 0 件）。検証は変異で全行の識別力を実測しており、途中 2 度「変異で殺せない空虚な檻」を差し戻しで摘出した。残件は明示的な非表示指令の実機確認 1 件のみで、emo2 fixture に発行元が無いため `balloon-canon-residue` へ送付済み。
  - `scope-chain-gap` ✅（2026-08-13）＝**P2 連鎖の幅差隙間の決着**。SSP 実測オラクル 8 本で確定規則（完全隣接・隙間 0・拡大率不変）を採り、先行 `window-placement` R2.9 の受入基準を COMPAT §8 で上書き（参照実装のラベルが実挙動と突合されないまま貼られていた）。**実機 2 水準（200% 必達・100% 対照）とも gap 0**。承認判断の実機目視から**要件 7 が差し戻しで新設**——「初期配置は実表示寸が確定するまで暫定」とし確定時に連鎖を**一度だけ**解き直す機構で、定常表示の 52px も 0 へ。境界外 2 件（`\![move]` の dx/dy を k 倍＝意図的 SSP 非互換・clippy エラー 0 化）は開発者指示で同ブランチ処理。
- **実機サインオフ発見 7件中 #1〜#6 解決済み**。**#7（冒頭空行）のみ pasta 上流（`ekicyou/pasta` 起票済み）＝areka スコープ外・未解決**。
- 完了 spec = **155**（`.kiro/specs/completed/` の**直下エントリ数**＝ディレクトリ 154 ＋ 単体ファイル `graphics-rendering-stability.md` 1）。**ディレクトリ数を採ると 1 ずれる**ので計数時は直下エントリで数えること（旧値 151 は `file-slimming` マージ PR#103 が本行の更新を落とした stale であった）。

## M1 残工程ゴール表（2026-08-01 棚卸⑤・全配置確定）

| 種別 | ゴール（単一文） | ユニット | ウェーブ |
|---|---|---|---|
| ~~挙動バグ（M-dpi 残1）~~ | ~~DPI 追従下の当たり判定 ÷k~~ | `completed/collision-dpi-hittest` | ✅ **完了**（2026-08-05・W6 で着地） |
| ~~基盤~~ | ~~ソースファイル肥大の是正＝檻の兄弟ファイル分離＋follow.rs/frame.rs 本体分割（挙動変更ゼロ・テスト総数不変）~~ **実績は全域**（1,000 行超の全ファイルを分割） | `completed/file-slimming` | ✅ **完了**（2026-08-10・W5.95 単独で着地。最大 8,472→986 行・1,000 行超 54→0 本） |
| ~~横断~~ | ~~バルーンが可視コンテンツ駆動 show／talk 終了+30s+無フォーカス hide／再表示~~ | `completed/balloon-visibility` | ✅ **完了**（2026-08-13・W6 で着地） |
| ~~挙動バグ~~ | ~~表情固着解消＝bindoption 3値正典（mustselect/非宣言=高々1/multiple）準拠~~ **実績は根因 2 層**（3 値意味論＋bind から外れた ID の保持コマ掃除 3 段） | `completed/bindoption-exclusivity` | ✅ **完了**（2026-08-11・W6 で着地） |
| 挙動バグ | バルーンが他アプリ窓の背後へ埋もれない＝各バルーンを自シェルの直前へ維持 | `ghost-window-zorder` | **W6**（確定） |
| ~~挙動バグ~~ | ~~P2 連鎖の幅差隙間（実機 123px）解消＝SSP 実測で正典確定~~ | `completed/scope-chain-gap` | ✅ **完了**（2026-08-13・W6 で着地） |
| 基盤 | 画素演算の f32 排除＝`ScaleRatio` 有理数を text 層まで配管 | `scale-exact-rational` | **W6.5** |
| 挙動バグ | `windowposition.limit` 正典既定 1 実装＝バルーン画面外はみ出し解消 | `windowposition-limit` | **W6.5**（確定・scg 檻へ rebase） |
| 挙動バグ | 常駐アイドルの CPU 消費＝毎フレーム全再合成のアロケーション予算是正 | `recompose-budget` | **W6.75**（確定） |
| 挙動バグ | 拡大率切替時の跳ね＝遷移の原子性＋work area 追随（+36px/24px 浮き） | `dpi-transition-atomicity` | **W6.75**（確定・再観測は W6 中） |
| 挙動バグ | DPI 遷移時の `BalloonFollow.offset` スケール意味論確定 | `balloon-offset-dpi` | **W6.75**（atom 縮退時は atom と統合） |
| 基盤 | 檻の決定性（毒化 **95 呼出/12 モジュール**・ハーネス 2 設計の一本化・注入シーム） | `test-cage-determinism` | **W6.9**（W6.5 から改訂）。取りこぼし（areka-seriko/actor.rs 他）は**追記(59) 済**（2026-08-05 PR#101）＝①インベントリを 45/7 → 95/12 へ拡大・偽陽性=赤の側も明記 |
| M-e2e | 適合14項目一周＋DoD＝**M1 完成宣言** | `emo2-conformance-e2e` | **W7**（最終） |

> 完了済みマイルストーンのゴール表は history 参照。M-dual は退役＝e2e 適合 #10 へ吸収。

## ウェーブ編成（着手順の正本・2026-08-01 追記(58) 改訂）

> 各ウェーブは**フルライフサイクル**（要件→設計→タスク→実装→`/kiro-complete`＝PR squash マージ）を完走してから次へ。並走はウェーブ内のみ（1 spec = 1 worktree = 1 PR）。同居は**実測で共有ファイル 0**が原則。文書フェーズ（要件・設計）は先行可＝先行 spec はウェーブ開始時に settled main へ再突合。優先順位: **挙動バグ → 依存ツリーが長く早期着手が効くもの → その他**。詳細な申し送りは各 brief の追記(58)ブロックが正本（roadmap は編成と条件のみ持つ）。

| Wave | ユニット | 開始コマンド | 編成根拠・条件 |
|---|---|---|---|
| W1〜W5 ✅ | （W5 は 4/4 着地・完了サマリ参照。col は W6 へ編入のうえ 2026-08-05 着地） | — | 詳細は history |
| ~~**W5.95**（単独）~~ ✅ | ~~`file-slimming`~~ → `completed/file-slimming` | — | ✅ **完了**（2026-08-10・64 コミット・PR で squash マージ）。**最大 8,472→986 行・1,000 行超 54→0 本・ファイル内テスト 500 行超 49→0 本。** 三不変量（テスト総数 4,790 不変・本文一致 61/61・対応表 1,163 行の全単射）成立。**一度 GO を撤回して群 8 を追加**（1,000 行超 13 本＝17,450 行を 92 本へ分割）——経緯は `completed/areka-P0-file-slimming/verification/notes.md` §39／§54 |
| **W6**（残 1本・col ✅／vis ✅／bind ✅／scg ✅ 着地済み） | ~~`collision-dpi-hittest`~~ ✅ ∥ ~~`balloon-visibility`~~ ✅ ∥ ~~`bindoption-exclusivity`~~ ✅ ∥ `ghost-window-zorder` ∥ ~~`scope-chain-gap`~~ ✅ | `/kiro-start areka-P0-ghost-window-zorder` | **実測で全 10 ペア素**（2026-08-01 干渉再測定）。全て挙動バグ級＋vis は W7 前提の最長依存（vis→cage→e2e）。編成面の条件: ⑴**vis の hover は既設消費**（spawn.rs 非接触・触るなら despawn hook ハンク回避）→ **条件を満たしたまま 2026-08-13 に着地**（spawn.rs 非接触）⑵**bind は `BindResolver::empty()` 署名維持**（呼出元 4 箇所）→ **条件を満たしたまま 2026-08-11 に着地**（`empty()` の署名は不変・呼出元は無傷。`BindResolver::new` は `BindOptionDecls` 構造体引数へ移行したが全 13 呼出元を同時追随させた）⑶**zorder は案 A（owner）の WUC 共存実機検証が最初のタスク**・vis は zorder の「自シェルより手前」保証を前提にしてよい⑷~~scg は要件前に SSP 実測必須~~ → **scg 着地（2026-08-13）。後続は 3 点に留意**——(a) `windowposition-limit` は scg の檻へ rebase 必達（P2 式と連鎖の持ち越し状態が変わり、`finalize_chain_once` という**第 2 の位置ライター**が resolver の外に増えた＝P4 クランプを経由しない）(b) `dpi-transition-atomicity` は `ChainFinalized`／`ChainFinalizeStall` の寿命を設計判断として引き取る (c) 配置系 spec は `window-placement` R2.9 を正典として引用しないこと（正典は COMPAT §8 経由で scg へ一意に辿る）⑸~~col は presenter.rs :867 域~~ → **col 着地により presenter.rs の実形が変化**（`hit_region_client`／`applied_ratio`／`ClientHit` 新設・`hit_region` 本体は不変）。同ファイル後続（exact/budget/atom/cage④）は着手時に col 後の presenter.rs へ rebase すること。**⑹`dpi-transition-atomicity` を文書フェーズ限定で同居**（下行）——観測は spec の外では実施できない（本プロジェクトに spec 外の作業経路は無い）ため、**第 1 段再観測は atom 自身の requirements フェーズの research として行う**。`/kiro-start` は要件討議まででコードに触れないので W6 の残 1 本と衝突しない。design 以降は W6.75 まで進めないこと（settled main へ再突合してから） |
| **W6 併走（文書のみ）** | `dpi-transition-atomicity`（**requirements フェーズまで**） | `/kiro-start areka-P0-dpi-transition-atomicity` | 第 1 段再観測＝要件の土台。S1 是正で実測①（859ms・`SetWindowPos` 8 回のうち 4 回）が失効しているため、**再採取しないと要件が書けない**。観測結果で W6.75 の形が決まる（縮退→bod と統合／残存→3 分割検討）。**⛔ design 以降は W6.75 で**（presenter.rs `apply_show`・frame.rs dpi 相で col/vis と衝突するため） |
| **W6.5**（2本） | `scale-exact-rational` ∥ `windowposition-limit` | `/kiro-start areka-P0-scale-exact-rational`・`/kiro-start areka-P0-windowposition-limit` | 実測で素。**wpl は scg 着地後必達**（resolver.rs 同一関数 `resolve` :131-190 内 P2/P5 が 30 行差＋mod.rs fixture 檻共有＝scg の新期待値へ rebase。逆順だと limit クランプ後の値へ scg が二重補正）。exact は presenter.rs :665 域＝col の実形へ rebase。**exact は bod の前提**（丸め権威）ゆえ W6.75 より先 |
| **W6.75** | `recompose-budget` ∥ `dpi-transition-atomicity`（＋`balloon-offset-dpi`） | `/kiro-start areka-P0-recompose-budget`・`/kiro-start areka-P0-dpi-transition-atomicity` | **分岐は W6 中の atom 再観測結果で確定**: ⑴**縮退時（859ms/8 回が S1 是正で消滅）**＝atom は「+36px work-area 追随＋檻」へ縮退し **bod と統合して 1 spec 化**（follow.rs/persist.rs 系）→ budget（presenter.rs/cache.rs 系）と**並走可**。⑵**残存時**＝atom は presenter.rs apply_show 域へ広がるため **budget 先行→atom 直列**（budget 着地後に 859ms を再測すると合成コスト ≒143ms の帰着切り分けが最良）・bod は atom に従う。いずれでも bind 着地済み（W6）ゆえ budget の CPU 単調上昇の (a)bind 同根/(b)活性集合の切り分けが可能 |
| **W6.9** | `test-cage-determinism` | `/kiro-start areka-P0-test-cage-determinism` | `presenter/show.rs` `apply_show` 鎖（budget→atom→④）の**最後尾**。vis 先着は slimming の別ファイル化で必達→**推奨**へ緩和（追記62）。着手時に vis/exact/budget/atom の実形へ rebase・毒化 45 呼出の再計数（後置するほどコピーが増える構造が実証済み＝**これ以上後ろへ置かない**）。④は `#[cfg(test)]` fault フラグ小案を第一候補（presenter.rs 衝突が :510 の 1 呼出へ縮退） |
| **W7** | `emo2-conformance-e2e`（最終） | `/kiro-start areka-P0-emo2-conformance-e2e` | 全ユニット完了後＝適合14項目（#1 DPI 検証は追従込み・バルーン表示ライフサイクル追補・#3 は bind・#10 は ker が前提充足）の一周走行→**M1 完成宣言**。着手時義務: brief 全面再監査・㉘(E) の実機判断・#7（pasta 上流）は M1 完成を妨げない扱いの確認 |

**干渉台帳（生存ペアのみ・2026-08-11 棚卸⑦で slimming 後の新レイアウトへ全面再解決）**:
- **scg⇄wpl**〔**同ハンク級・直列必達（scg 先）**: resolver.rs `resolve_placement`（:124 起点）内 P2 **:151-159**／P5 **:180-188** ＝30 行差＋placement fixture 檻共有。resolver.rs は slimming で未分割＝干渉不変〕
- **budget⇄atom**〔**同ハンク＋因果・slimming 後も同居**: `presenter/show.rs` の `apply_show`（:23 起点）内——budget=compose/resample **:55-70**／atom=原子スワップ域 **:186-215**。859ms が合成コスト帰着なら budget へ差し戻し＝budget 先着後の再測が切り分け最良。atom 縮退時は解消〕
- **cage④⇄budget**〔`presenter/show.rs` 同居継続: cage④ 観測点＝upload エラー分岐 **:190-206** が budget :55-70 と同ファイル。④小案で縮退可。cage は W6.9 最後尾で解決〕
- **cage⇄vis**〔**slimming で緩和＝別ファイル化**: vis 本体= `frame/attach.rs`（`run_attach_phase` :155・`connect_balloon_text` :375）／cage①= `frame_test_support.rs`（`capture_logs` :115＋呼出 8 箇所は `frame_drain_text_tests.rs`・`frame_text_scale_tests.rs`）。**vis 先着は「必達」から「推奨」へ格下げ**（vis の attach 檻書換が test 兄弟ファイル側に落ちるため衝突は限定的）〕
- **atom⇄bod**〔follow 系共有＝**統合候補（縮退時は統合が既定路線）**。slimming で follow.rs は `follow/{anchor,drag_follow,visibility,window_move,work_area}.rs` へ分割——atom の主戦場は `follow/window_move.rs`（701 行・DPI 遷移域）・persist は従来どおり `placement/persist.rs`。分離時は atom 先着→bod rebase〕
- **atom⇄vis**〔**slimming で緩和＝別ファイル化**: vis=`frame/attach.rs` ∥ atom=`frame/dpi.rs`（全 393 行・`run_dpi_phase` :385）＋`frame/drain_resnap.rs`（`resnap_shell_targets` :147）。共有点はファサード `frame.rs:135-163` のフェーズ列のみ＝**順序変更時のみ直列注意**〕
- **atom⇄wpl**〔wpl の limit クランプが follow 側に落ちた場合のみ衝突＝wpl の SSP 観測（適用時点）確定後に再判定〕
- **atom⇄zorder**〔`SetWindowPos` flags 別ビットゆえ素の見込み。atom が flush 経路（tick_bridge/command.rs）を改造する場合のみ着手時再突合〕
- **wpl⇄bod**〔windowposition.rs（331 行）異ハンク: wpl=:39 変換域／bod=:93-94 合流欄。小ファイルゆえ先着後 rebase〕
- **presenter 直列鎖（slimming で分解）**〔旧 presenter.rs 単一ファイル鎖は解消——presenter.rs は 109 行ファサード・本体は `presenter/{hub,show,refresh,read,hit,target}.rs`。**exact は `presenter/read.rs`（f32 汚染点 :109・`applied_ratio` :171）＝budget/atom/cage④ の `show.rs` と別ファイルへ緩和**。残る同居は budget/atom/cage④ の `show.rs` 3 者のみ（上記ペア参照）。col 新設の `ClientHit`/`hit_region_client` は `presenter/hit.rs`（:16／:82）。ウェーブ順がそのまま先着順＝各後続は先行の実形へ rebase〕
- **因果のみ（コードは素）**: ~~bind→budget（CPU 上昇の切り分け）~~ → **bind 着地（2026-08-11）で切り分けが可能になった**＝CPU 単調上昇の仮説 (a)bind 同根は構造的に消滅し、残るのは (b)活性集合／毎フレーム全再合成のみ。**budget（W6.75）は着手ゲートが開いている**（実測値・第 0 段の計時ログ設計は budget brief の 2026-08-11 追記が正本）・exact→bod（丸め権威）・zorder→vis（手前保証の前提）・vis⇄scg/wpl（balloon 位置は char 従属・hide 中の limit 補正は無意味＝順序自由）
- **軽微**: cage③の test_support 共有化で placement 系（scg/wpl/bod）の import 行が追随＝実質共存可
- **slim→全 spec（着地済み・2026-08-10）**: `file-slimming` の着地で**全 brief の file:line が一度ずれた**。各 spec は design 前 rebase（既存規律）で吸収すること（slim・棚卸は brief を書き換えて回らない）。**とくに `test-cage-determinism`（W6.9）は送付所見 9 件すべての宛先**で、アンカーは `completed/areka-P0-file-slimming/verification/notes.md` §37 が HEAD 時点で全数再解決済み。主要アンカーの新位置と干渉判定は**棚卸⑦（追記62）で本台帳へ全面反映済み**
- `status-execution-states`=台帳 spec（着手しない・源着地時に just-in-time）・`surfaces-basepos`／`sakura-time-directives`／`balloon-canon-residue`=**M2 解禁ゲート**（M1 では着手しない）

## 着手手順

> **brief 全数完備体制**: M1 残ユニットは全て brief 済み（12 本・2026-08-01 棚卸⑤で全 brief へ追記(58) 補正適用済み）＝着手は該当 brief を読んで `/kiro-start <unit>` へ直行。新規課題の起票は `/kiro-discovery`（再入）で brief just-in-time 生成。`/kiro-spec-batch` は使わない（一括＝工場化）。ウェーブ跨ぎの合流判断は別セッションで一括（記憶 portfolio-convergence-decided-in-separate-session）。

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

**追記台帳（要約・全文は history）**: (51) W6.5 残件 2 spec 起票（07-30）／(52) 棚卸④＝roadmap 履歴分離・W6 に bind 編入・brief 補正 8 本（07-31）／(53) `recompose-budget` 起票＝`compose_into` 本番未配線・アイドル 1 コア 13〜22%（07-31）／(54) `ghost-window-zorder` 起票＝owner 無し・全 `SWP_NOZORDER`・「語彙完備・配線ゼロ」3 例目（07-31）／**(55) 欠番**（atom 起票の本体がマージで脱落・内容は atom brief＋(57)⑴ が正本）／(56) 未登記先送り棚卸＝孤児 17 件・新規 brief 4 本起票（08-01）／(57) van 完了＝S5 担当が atom で確定・確定台帳の裁定付き訂正運用が確立・檻の空虚性通算 9 例（08-01）／(59) col 実装中の申し送り＝cage brief ①インベントリを 95 呼出/12 モジュールへ拡大・偽陽性側も明記（08-05・PR#101・cage brief 内が全文正本）。

**2026-08-06 追記(60)（棚卸⑥＝col マージ後の軽量アンカー監査）**: `/kiro-discovery` 再入。前回⑤以降の main 差分は col 実装（PR#100）と cage brief 追記(59)（PR#101）のみ＝brief 無し spec **0**・新規起票 **0**・ウェーブ編成**変更なし**を確認。col 着地の brief アンカー実測監査（W6 残 4 本＋atom＋exact＋budget・16 項目）: **実害ドリフトは balloon.rs（+155・col のテスト増）と presenter.rs（+17）の 2 ファイルのみ**で、意味論的陳腐化はゼロ（TalkDone 未配線・無条件 ShowSurface・f32 汚染経路・resolver.rs P2/P5 は全て現物一致）。補正: bind（`empty()` :1482）・exact（汚染点 :682＋**col 新設の `unscale_coord`/`applied_ratio`/`ClientHit` を前提へ編入**＝重複設計防止）・budget（apply_show 一族 +17）・vis（全アンカー無傷の再確認）・atom（全一致確認）の 5 brief へ追記(60) 適用＋本台帳の干渉行番号を col 後の実形へ更新。zorder・scg・wpl・cage は無傷（cage は (59) で更新済み）。

**2026-08-06 追記(61)（`file-slimming` 起票＝W5.95 新設）**: 開発者提起「1 file 当たりの行数肥大」を実測——最大 8,472 行（follow.rs）だが**肥大の 7〜8 割は in-file 檻**（本体は大半 500〜1,000 行で健全・本体過大は follow.rs 1,997／frame.rs 1,498 の 2 本のみ）。in-file 檻の実害 2 点（檻挿入によるアンカードリフト税＝col の balloon.rs +155 が実例／同一ファイル干渉の増幅）を認定し、`areka-P0-file-slimming` を起票。**配置は W5.95（単独・W6 実装より前）**——当初の W6.95 案（cage 後）は「実装が 1 本も走っていない今は衝突相手ゼロ」の開発者指摘で却下し、実装空白期の先行が最安と裁定。檻の兄弟ファイル分離（in-crate 規律は in-file を要求しない）＋2 本の本体分割・挙動変更ゼロ・テスト総数不変。W6 文書フェーズは並走可。

**2026-08-11 追記(62)（棚卸⑦＝file-slimming マージ後の干渉台帳全面再解決）**: `/kiro-discovery` 再入。前回⑥以降の main 差分は file-slimming 実装（PR#103・64 コミット）とメモのみ＝brief 無し spec **0**・新規起票 **0**・**ウェーブ編成の骨格は変更なし**（W6 残 4 本並走→W6.5→W6.75→W6.9→W7）。slimming の分割で干渉台帳を実測再解決——**緩和 3 ペア**: ⑴cage⇄vis（vis 本体=`frame/attach.rs` ∥ cage①=`frame_test_support.rs` へ別ファイル化＝vis 先着は必達→推奨へ格下げ）⑵exact⇄budget（exact=`presenter/read.rs` ∥ budget=`presenter/show.rs`）⑶atom⇄vis（`frame/dpi.rs` ∥ `frame/attach.rs`・共有はファサード `frame.rs:135-163` フェーズ列のみ）。**同居継続 2 ペア**: budget⇄atom・cage④⇄budget（いずれも `presenter/show.rs` の `apply_show` 内＝W6.75/W6.9 の直列条件は不変）。scg⇄wpl は resolver.rs 未分割で不変（P5 のみ −2 行）。旧「presenter.rs 直列鎖」は**ファイル単位では解消**（同居は show.rs 3 者へ縮小）。付随発見: bind brief の `looper.rs:215 BindResolver::new` は現物に不在（等価サイトは `areka-seriko/src/bind.rs:356,370`）＝bind の design 前 rebase で吸収すること。brief 書換は規律どおり実施せず（design 前 rebase が正）。

**2026-08-11 追記(63)（`bindoption-exclusivity` 完了＝表情固着の根治・根因は 2 層・`mayuna-compose` R4.5/D11 の覆し）**: W6 の挙動バグ 1 本が着地（PR#105）。**根因は 1 層ではなく 2 層あった**。

**第 1 層（当初捕捉・bind 集合の単調肥大）**: `bindoption` を ukadoc 正典の **3 値意味論**（`mustselect`＝ちょうど 1 個・解除不可／**非宣言（既定）＝高々 1 個・解除可**／`multiple`＝複数可）へ是正した。**採取層が `multiple` 宣言を捨てていた**（非宣言と明示 multiple を下流で区別できない情報欠落の根）ため、判定は「`mustselect` か、さもなくば加算」の 2 値に縮退しており、非宣言カテゴリで bind 集合が単調に肥大していた。⑴採取層は `+` 区切り複数オプション（正典「オプションは+区切りで複数可」）を含めてオプション語ごとに認識・スコープ別に転記、⑵判定は 3 値 enum（`BindChoicePolicy`）＋単一アクセサ `BindResolver::policy` へ一本化（旧 2 値述語は退役）、⑶適用は「複数可と宣言されていないカテゴリの着衣＝排他置換」へ反転、⑷`mustselect` への脱衣指示は正典「解除不可」どおり集合を変えず読み流し `warn!` で痕跡を残す（**無言の握り潰しを作らない**）。

**第 2 層（実機 J3 不合格で発見・保持コマの残留）**: 第 1 層の是正で機械判定 J1/J2 は反転したのに**開発者の目視ではジト目が固着し続けた**。真因は「**bind 集合から外れたアニメーションの最終コマが誰にも掃除されない**」——`-1` 終端でない残留型アニメの保持コマが、所属 ID が bind から外れたあとも表示状態に居座る経路の不在である。**3 段で是正**: ①`areka-seriko` の bind 確定時に「旧集合 − 新集合」の ID の保持コマを**発行前に**除去、②SERIKO ループの進行相で bind 抽選種アニメの再生を停止（①単独では進行相が最終コマを置き直すことを RED で直接観測した）、③合成側の合流ゲートで bind 種アニメの非所属 ID を落とす（最終防衛線・`areka-emo-compose` は seriko へ依存せず面台帳から判定するので層の向きは不変）。この 2 層目の取り込みは開発者裁定によるスコープ拡張であり、要件 7・設計 D9 が正本。**当初の「`looper.rs` 無改変」宣言は D9 で明示的に撤回した。**

**⚠️ `completed/areka-P0-mayuna-compose` の R4.5／D11 を覆した**（完了済み spec の判断の訂正）。覆された判断＝**「`multiple`（紅等・非宣言＝既定）はスクリプト明示 on/off で従来どおり成立ゆえ語彙保持のまま」「非宣言は既定＝非排他で無視」**（`completed/areka-P0-mayuna-compose/requirements.md:85` の R4.5、`同/design.md:68` の 3 分類表と `:142` の D11）。覆しの根拠は 2 系統:

- **正典**: ukadoc `descript_shell` の `bindoption` 既定値は「選択解除可能、**複数選択不可**」。非宣言＝非排他（加算）は正典に反する。
- **実機証拠**: 2026-08-11 の emo2 実走直接観測——非宣言のまばたきカテゴリで bind 集合が **{1403}→{1403,1400}→{1403,1400,1402}** と単調肥大し、飽和後にゴーストが送った**是正指示 28 件がすべて「無変化」として握り潰され**、表情が非可逆に固着した。ゴーストは正典作法（on のみ送る）で完全にシロ。

`mayuna-compose` は `mustselect` についても同型の誤仮定（「ゴーストが明示 off を送るはず」）を 2026-07-23 に実機で反証された前例があり、**同じ穴の 2 度目**である。完了済み spec の文書は改変しない規律ゆえ現物は無改変とし、覆しの根拠は本 spec の設計文書（`§mayuna-compose 覆しの記録`）に、2026-08-11 の裁定（`mustselect` 解除不可を本 spec で拾う・警告水準の選定根拠）は同 `D1` に記録した。

**⚠️ 本番挙動の変化（次の読み手が驚く点）**: 既定の意味反転で、実 emo2 で**新たに排他になるのは非宣言かつ複数パーツの 3 カテゴリ——まばたき（1400-1403）・キラリ（1700/1701）・髪飾り（1800/1801）**。とくに**髪飾りは `bindgroup1800.default,1` で起動時オン（リボン）**のため、今後 `\![bind,髪飾り,ボンボン,1]` を送ると**既定オンのリボンが自動で外れる**。これは正典どおりの意図した是正であり、退行ではない。

**先送りの登記（残る正典乖離・完全語彙）**: 正典の `mustselect` は「必ず 1 つ選択」（**ちょうど 1 個**）だが、本 spec が実装したのは **off 無視（解除不可）まで**である。**起動時の充足は shell の `default,1` 宣言へ委譲する（既存縮退の維持）**。したがって **`mustselect` カテゴリに `default` 宣言が 1 つも無い shell では、最初の on 指示が届くまで当該カテゴリは起動時ゼロ個のまま**になる。emo2 は既定集合 `{1100,1207,1302,1500,1800}` が全 `mustselect` カテゴリ（腕 1100・口 1207・目 1302・眉 1500）を被覆するため**実害なし**。**起動時自動充足（先頭パーツの自動選択等）は実装しない**——縮退シーム＝ポリシー判定の一元アクセサ（この 1 点を変えれば起動時充足を後付けできる）。追跡は本追記が担う（M1 では起票しない・実害が観測されたら `/kiro-discovery` で起票）。あわせて **`char*.bindoption*.group`（char2 以降）の走査は既存縮退のまま**（採取層は `sakura.`/`kero.` 接頭辞のみ走査）——`bindoption` 固有の新規乖離ではなく `bindgroup` 系全体が持つ既存の縮退で、本 spec は形を揃えただけである。従来の引受先だった **M-dual は退役済み**（e2e 適合 #10 へ吸収）ゆえ、受け皿としてここに登記する。さらに**掃除 3 段の隙間 2 件**（⒜bind 集合の要素だが interval が純 `random` のアニメ・⒝合成側ゲートの面種非限定）を設計へ登記済み——いずれも現行 emo2 fixture では構成不能ゆえ檻を作らず、要件の再裁定を伴う。

**実機サインオフ（合格・2026-08-11）**: 実 emo2（実 pasta.dll・辞書込みフルゴースト・絶対パス起動・有界自動終了）で 4 回実走。**J1**（同一時間帯に複数まばたきパーツが並行発火する痕跡の不在）＝是正前 **違反 109/169** → 是正後 **0/66** → 掃除 3 段後 **0/61** → 25 分長期 **0/285**。**J2**（片側カテゴリの恒久沈黙＝飽和パターンの不在）＝是正前 **回数差 22／末尾時刻差 316.8 秒** → 是正後 **差 1** → 7 分 **差 3・末尾差 2.19 秒（PASS＝正準判定）**。**J3**（ジト目からの表情復帰の目視）＝bind 層のみでは **FAIL（固着再現）** → 掃除 3 段後の **25 分長期で PASS**（ジト目が 13 回 bind され、いずれも次の表情変更で正しく復帰・開発者が直接目視確認）。25 分実走の J2 条件 A（差 9 > 閾値 4）は手順書 §7 の申し送りどおり検分し、**飽和ではなく走行時間比例のドリフト**と裁定（閾値 4 は 7 分較正値・25 分の想定は 3×(25/7)≈10.7・条件 B は末尾 21.1 秒前まで継続で PASS）——正準判定は同一較正条件の 7 分 PASS を採る。掃除の実機発火は①11 回＋②3 回。判定手順は `signoff-scan.py`（標準ライブラリのみ）＋`signoff-procedure.md` として再実行可能な形で残し、是正前の保全ログで**既知ケース較正（必ず赤になること）も確認済み**。

**申し送り 2 件**: ⑴bind 経路のコード内に**出所が単一 spec に定まらない裸の設計 ID 引用**（`D1`・`D4`・`D7`・`D8` 等）が残っている。本 spec の設計 ID と番号が正面衝突しているが、実測すると複数 spec 由来が混在しており、**誤った spec 名を冠するのは裸で残すより有害**なため本 spec では無改変とした（本 spec の一掃対象は `D11`／`R4.5`／`Req 4.5` の 3 文字列に明示限定・受入条件は充足）。出所の全数特定と接頭辞付与は別途裁定すること。⑵本 spec が追加した info ログの檻 3 本と、既存の間欠赤 1 本（`bind_default_exclusive_replace_emits_show_and_info_marker`＝120 反復中 2 回失敗を実測）は**すべて `test-cage-determinism`（W6.9）の担当クラス**（ログ捕捉ハーネスの非決定性）である。因果は独立で本 spec の是正内容とは無関係。

**2026-08-01 追記(58)（棚卸⑤＝W5 3本マージ後の全面棚卸・本改訂の由来）**: `/kiro-discovery` 再入（開発者指示: roadmap 棚卸＋brief 精査＋肥大化対処＋ウェーブ再編）。サブエージェント 2 体で実測——(a) **brief 陳腐化監査**（12 本全数・main `ec9687c` 突合）: 重大 1（atom＝着手ゲート開放・S1 是正で実測①②の前提失効・+36px スコープは有効のまま）・数値全面更新 1（cage＝スピン 13→残 2・毒化 44/6→45/7・medium へ縮小）・要補正 4（vis/bind/zorder/e2e＝W5 前提の完了形化）・軽微 6。**7 本へ追記(58) 補正ブロック適用済み**。(b) **干渉行列再構築**（11 spec・55 ペア）: 生存衝突 12 ペア＋因果 4 ペア・素 26 ペア。**新発見**: scg⇄wpl は resolver.rs 同一関数内 30 行差＝同ハンク級へ格上げ。①**brief 無し spec=0 を確認**（roadmap 登記 16 本全てに brief 実在）。②**全配置確定**（従来の配置裁定待ち 6 本を一括裁定）: W6=5 本並走（col 編入・zorder/scg 編入）・wpl=W6.5・budget/atom/bod=W6.75（atom 縮退時は bod と統合）・cage=W6.9 へ後送（apply_show 鎖の最後尾・ただしこれ以上は後送しない）。③**過積載 1 件**: atom は 3 関心（観測基盤/原子性/work-area 追随）で分割規定を brief へ登記——ただし分割判断は W6 中の再観測（コード非接触・即実施可能）を待つ。④roadmap 減量: W5 詳報・旧ウェーブ表・旧干渉台帳・追記(53)(54)(56)(57) 全文を history へ退避し、干渉台帳を生存ペアのみへ再構築・**(55) 欠番を正式裁定**（㊻の教訓どおり注記のみ・追記順序の乱れ〔(57) が (56) より先〕も要約台帳化で解消）。⑤「語彙完備・配線ゼロ」通算 5 例を受け、実装規律へ定期監査項目として昇格。
