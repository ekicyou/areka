---
inclusion: manual
updated_at: 2026-07-31
---

# Roadmap — areka M1（最小 SSP 互換ベースウェア）

> **このロードマップは M1 のみを扱う。** M2 以降は **M1 完成後に実物を見て組み直す**（憶測で先に書かない）。
> 正本配置: 本ファイルが M1 ロードマップ正本（`.kiro/steering/roadmap.md`）。`focus.md`（`inclusion: always`）から辿る。設計判断の正本は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md)。M1 実物スコープは [doc/emo2-conformance-scope.md](../../doc/emo2-conformance-scope.md)。
> **履歴**: 2026-07-31 棚卸④（追記(52)）で旧全文（413行）を [roadmap-history.md](roadmap-history.md) へ凍結退避し、本体を残工程中心へ書き直した。追記①〜(51)・M-boot 23ユニット完了詳報・実機サインオフ発見7件・解除済み時限ゲート・DPI追従クラスタ経緯・wintf 先進坑2節の全文は history が正本（追記㊻の番号衝突など既知の記録欠陥も history 冒頭に注記）。完了ユニットの実装詳細は各 `completed/` spec が正本。

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

## アーキテクチャ横断原則（要約・詳細は history＋記憶＋completed spec）

- **シェル/バルーン統一**: 描画エンジンはシェルとバルーンを区別しない。バルーン＝surface 上の文字レンダリング層。element は他サーフェス参照可（入れ子・再帰合成）・配置は D2D 変換行列が内部表現。
- **アニメエンジンは2つ**: ①さくらスクリプト再生（talk timeline・sakura）＋②SERIKO ループ（seriko）。両者とも dola（絶対時刻台本・duration 権威・CuePlayer/CueSink）上。テキストは①から emo-text へ直接。
- **並行モデル**: 各エンジン＝チャンネル通信のアクター＋独立スレッド。**render/window は UI スレッド固定**。機構=areka-actor／経路=kanade／結線=ghost の責務三分。
- **emo 合成**: 自前コンポジタ（アトラス→1枚物ビットマップ合成→wintf へ完成品のみ）。emo=UI 層全般（合成・マウス/さわり・バルーン文字・選択肢）＝「見える・触れる」の窓口。
- **DPI 追従が基本設計**: k=monitorDPI÷author_dpi で全表示経路がスケール（W4 で実装済み・SSP と別思想・k=1.0 は途中状態）。**キャラ窓の原点は下端中央（足元の中心）**——保存/復元/resize/バルーン追従の四層で統一済み（Bottom 限定）・左上基準で計算しないこと。

**エンジン固有名**（コード/spec/会話の参照はこの名で統一・詳細は記憶 areka-engine-names）:

| # | エンジン | 固有名 | # | エンジン | 固有名 |
|---|---|---|---|---|---|
| ⓪ | ゴーストエンジン（最上位 owner） | `ghost` | ④ | さくらスクリプト再生 | `sakura` |
| ① | SHIORI 通信層 host-32 | `shiori` | ⑤ | SERIKO アニメ | `seriko` |
| ② | parser/loader | `parsers` | ⑥ | render（surface 合成＋UI 層） | `emo` |
| ③ | conductor（SHIORI イベント循環） | `kanade` | | | |

## 完了サマリ（2026-07-31 時点・詳細は completed spec ＋ history）

- **耐力壁突破**（2026-07-01 `pilot-shiori-host-32`）: x64→32bit pasta.dll 駆動 GO。
- **M-boot 23/23 完了**（2026-07-13 `emo2-boot`）: 起動→表示→talk→close の可視一周。①shiori・②parsers トラック全完了。
- **増分ウェーブ W1〜W4＋割込 全完走**: W1（idle-talk∥collision-geometry∥sakura-dialogue-tags）→ W2（input-events∥mayuna-compose）→ W3（sylphya∥seriko-loop∥choice-render）→ 割込（wintf-gpu-test-crash＝workspace テストゲート復旧）→ W4（position-persist∥choice-interact∥emo-dpi-scaling＝DPI 追従 k の全表示経路適用）。横断: cue-playback-duration・surface-resize-resnap・balloon-face-cue・emo-text-viewbox ✅。
- **実機サインオフ発見 7件中 #1〜#6 解決済み**。**#7（冒頭空行）のみ pasta 上流（`ekicyou/pasta` 起票済み）＝areka スコープ外・未解決**。
- 完了 spec = 147（`.kiro/specs/completed/`）。

## M1 残工程ゴール表

| マイルストーン | ゴール（単一文） | ユニット | ウェーブ |
|---|---|---|---|
| M-dialogue（残1） | メニュー一周のカスケード確定（OnChoiceSelectEx→任意名イベント）＋`Status: choosing` | `choice-select-events` | W5 |
| M-dpi（残2） | DPI 追従下の当たり判定 ÷k ＋混在 DPI 窓消失の決着 | `collision-dpi-hittest` ∥ `dpi-window-vanish` | W5 |
| （挙動バグ） | kero（scope1）バルーンが正典どおり `balloonk*` 資産・採寸 per-scope | `kero-balloon` | W5 |
| （横断） | バルーンが可視コンテンツ駆動 show／talk 終了+30s+無フォーカス hide／再表示 | `balloon-visibility` | W6 |
| （挙動バグ） | 表情固着解消＝bindoption 3値正典（mustselect/非宣言=高々1/multiple）準拠 | `bindoption-exclusivity` | W6（追記(52)裁定） |
| （基盤） | 画素演算の f32 排除＝`ScaleRatio` 有理数を text 層まで配管 | `scale-exact-rational` | W6.5 |
| （基盤） | 檻の決定性（tracing 毒化44呼出・スピン13箇所・ハーネス2設計の一本化） | `test-cage-determinism` | W6.5 |
| M-e2e | 適合14項目一周＋DoD＝**M1 完成宣言** | `emo2-conformance-e2e` | W7（最終） |

> 完了済みマイルストーン（M-boot・M-mayuna・M-life・M-dialogue 3/4・横断 cue/sylphya・割込）は history のゴール表参照。M-dual は退役＝e2e 適合 #10 へ吸収。

## ウェーブ編成（着手順の正本・2026-07-31 追記(52) 改訂）

> 各ウェーブは**フルライフサイクル**（要件→設計→タスク→実装→`/kiro-complete`＝PR squash マージ）を完走してから次へ。並走はウェーブ内のみ（1 spec = 1 worktree = 1 PR）。同居は**実測で共有ファイル 0**が原則（潜在近接のみ事前割当契約＋エスケープ条項）。文書フェーズ（要件・設計）は先行可＝先行 spec はウェーブ開始時に settled main へ再突合。

| Wave | ユニット | 開始コマンド | 上流充足・申し送り |
|---|---|---|---|
| W1〜W4・割込 ✅ | （全完走・完了サマリ参照） | — | 詳細は history のウェーブ表 |
| **W5** | `dpi-window-vanish` ∥ `collision-dpi-hittest` ∥ `choice-select-events` ∥ `kero-balloon`（4本） | `/kiro-start areka-P0-dpi-window-vanish`・`/kiro-start areka-P0-collision-dpi-hittest`・`/kiro-start areka-P0-choice-select-events`・`/kiro-start areka-P0-kero-balloon` | 4本のファイル集合は **W4 着地後の再実測（2026-07-31）でも互いに素**。**van**: W4 が確定事実④（WM_DPICHANGED 窓固定）を解消済み（`run_dpi_phase` frame.rs:865-873）＝診断 Q1〜Q4 は新ビルドで再実施→「再現せず・掃除のみ」へ縮退し得る・`GhostWindows` despawn 掃除＋終了時 `Anchored` WARN（follow.rs:792）は残。**col**: W4 が `applied_scale`（presenter.rs:706）で ÷k の席を名指し予約済み（:232/:704/:4361）＝W5 内最小 spec。**se**: `ChoiceSelection` 実物＝`input_events/balloon.rs:43`・消費口 `ChoiceSelectionInbox` :130 の drain が編集面に追加（W5 内は非衝突）・`ExecutionState::Choosing` 語彙実在（status.rs:26/:60）＝駆動側のみ・**ukadoc 訂正義務**（Ref0=ラベル/Ref1=ID・scope doc §1 が逆）・Req2.6 fail-open 申し送り。**ker**: W4 が per-scope の席を意図的に保全済み（measure.rs:127-128/:227-229 の申し送りコメント）＋`balloon_models` マップ（frame.rs:545）＝足場半分完成・**`refresh_actor_binding`（actor.rs:348）＋`run_text_scale_phase`（frame.rs:928）の同一 ActorKey 写像義務**（frame.rs:552-554）・binding 比較の穴（追記㊾）も担当。**rebase 条項（gpu-test-crash エスケープ FIRED）**: spine.rs S3/S4 檻域で**新規 GPU world テスト追加時のみ**オーナースレッド委譲へ乗せる（素の別スレッド Compositor は AV）。**全員**: 窓寸は原点=下端中央基準・判定は絶対 px でなく比 |
| **W6** | `balloon-visibility` ∥ `bindoption-exclusivity`（**2本・2026-07-31 追記(52)裁定**） | `/kiro-start areka-P0-balloon-visibility`・`/kiro-start areka-P0-bindoption-exclusivity` | 2本は実測で素（vis=frame.rs＋emo2_boot 新 module＋TalkDone UI 配線／bind=parsers resolve/model＋seriko bind/state/actor＋assets.rs:196-210）。**vis** ← W5 kero-balloon（`run_attach_phase` 末尾＝無条件 ShowSurface **:531-540**・`connect_balloon_text` **:549-556**／fn **:577-596** の同一関数域を先行改造＝per-scope BalloonModel の実形へ再突合してから着手）・hover donor は**既設消費へ昇格**（`attach_balloon_pointer_handlers` balloon.rs:758・main.rs:363/:693 結線）・TalkDone は kanade 止まり（steady.rs:226）のまま真・`emo2_frame_system` は **7フェーズ**（frame.rs:1297-1327）・討議7点は brief Open Questions 正本。**bind** ← 表情固着（非宣言カテゴリ「まばたき」加算飽和→1400/1402 並行発火→ジト目永久被覆・`Unchanged` 無言握り潰し）＝**挙動バグ**・3値正典へ述語反転＋parsers に multiple 集合・**禁じ手=z-order 変更/ゴースト fixture 修正**・W5 kero-balloon の assets.rs 先行着地後に rebase（:196-210 vs :278-300 異ハンク）・requirements 前に `areka_seriko=debug` で握り潰し直接観測 |
| **W6.5** | `scale-exact-rational` ∥ `test-cage-determinism`（2026-07-30 追記(51) 起票） | `/kiro-start areka-P0-scale-exact-rational`・`/kiro-start areka-P0-test-cage-determinism` | 残件消化ウェーブ。**exact** ← `physical_extent` の `ceil(v×k_f32)` が k=6/5 で 81/1200 件 +1＝「画素演算で f32 禁止」が文字層で破れ→`ScaleRatio` num/den を emo-compose→present→text へ配管。**cage** ← tracing 毒化 6 モジュール/44 呼出・spine.rs 協調スピン 13 箇所（`spin_pumping_ticks` が donor）・ログ捕捉ハーネス競合 2 設計の一本化・`chain.upload` 失敗注入シーム。**W5/W6 と同居不可**: cage は frame.rs＋measure.rs（W5 ker・W6 vis と衝突）＋balloon.rs 18 呼出（W5 se の drain 増設と衝突）。**2本の相互衝突**: `areka-emo-present/src/scale.rs` の `mod tests` 同一ファイル異ハンク＝**着手順の裁定が要る**（両 brief 登記済み） |
| **W7** | `emo2-conformance-e2e`（最終） | `/kiro-start areka-P0-emo2-conformance-e2e` | 全ユニット完了後＝適合14項目（#1 DPI 検証は追従込み・バルーン表示ライフサイクル追補・#10 kero 一式は W5 ker が前提充足・**#3 着せ替え表情は W6 bind が前提充足**）の一周走行→**M1 完成宣言**。着手時義務: brief 再監査（追記㊹で唯一補正無しだった＋上流3本追補済み）・㉘(E)「OnFirstBoot 限定 move の2回目起動蒸発は許容仕様」の実機判断・#7（pasta 上流）は M1 完成を妨げない扱いの確認 |

**干渉台帳（残ペアのみ・2026-07-31 再実測）**: **ker(W5)⇄vis(W6)**〔frame.rs `run_attach_phase` 末尾同一関数域〕／**ker(W5)⇄bind(W6)**〔assets.rs 異ハンク :278-300 vs :196-210〕／**ker(W5)⇄cage(W6.5)**〔frame.rs＋measure.rs〕／**se(W5)⇄cage(W6.5)**〔input_events/balloon.rs（毒化18呼出 vs drain 増設）〕／**vis(W6)⇄cage(W6.5)**〔frame.rs〕／**exact⇄cage（W6.5 内）**〔present scale.rs `mod tests` 異ハンク＝着手順裁定要〕／**van(W5)⇄vis(W6)**〔`placement/spawn.rs`＝**van が実際に触った・2026-07-31 タスク 3.1 着地で確定**。編集内容は⑴`GhostWindows::remove_entry_of(&mut self, Entity) -> Option<(usize, ScopeWindows)>`（entity の属する scope エントリを丸ごと除去・不一致/二重除去は `None` の no-op＝**全域かつ冪等**）を `impl GhostWindows` へ追加、⑵`GhostWindowMarker` へ `#[component(on_remove = on_ghost_window_marker_remove)]` 属性と同名の hook 関数を新設（hook は **Resource のみ**操作＝生存 entity の component を読み書きしない・要件 6.4 の構造的保証）、⑶`use bevy_ecs::lifecycle::HookContext;`／`use bevy_ecs::world::DeferredWorld;` の import 追加、⑷`mod tests` 末尾でなく T-I1 群の直後に `T-V1` 檻 7 本を挿入。**vis への申し送り**: ①vis が同ファイルへ触る場合は `GhostWindowMarker` 定義ハンク（marker の doc＋属性＋hook 関数＝連続 45 行）と `impl GhostWindows` ハンクを避けるか、van 着地後に rebase すること。②**ゴースト窓 entity を despawn すれば `GhostWindows` から scope が自動で消える**——vis が hide/show を despawn/respawn で実装すると窓を隠すたびにレジストリ登録が消える（`spawn_ghost_windows` を再実行しない限り復活しない）。vis の hide は **despawn ではなく可視性の切替**で実装すること。③van はタスク 5.1 で同ファイルへ `DpiSuggestedRectPolicy::ExternalAuthority` 付与（char/balloon 両 spawn バンドル）を追加予定＝spawn バンドル本体も後続で動く。④`hover donor 既設ゆえ vis は触らぬ公算大` の当初見立ては van 側からは変更なし〕／**van(W5)⇄ker(W5)**〔`emo2_boot/frame.rs`＝**同一ファイル・異ハンク**。**van が実際に触った・2026-07-31 タスク 3.2 着地で確定**（要件 6.2/6.3 の消費側存在確認）。van の編集面は⑴`use crate::placement::diag::{DESPAWNED_SKIP_TAG, PlacementRoute};`＝import 1 行の差し替え（:43）、⑵`reconcile_reported_sizes`（報告回収相）の scope×target ループ内へ entity 存在確認 15 行を挿入（現 :1049-1063＝`let Some(window) = window else { warn! … }` 腕の**直後**・報告を take した**後**に置く＝持ち越さない既存契約の維持）、⑶`resnap_with`（再スナップ相）の scope ループ**冒頭**へ同種の存在確認 14 行を挿入（現 :1254-1267＝`source.physical_size` 問い合わせより**手前**）、⑷`mod tests` 末尾ではなく**タスク 1.4 の route 檻節の直後**へ檻 3 本＋補助 3 関数を挿入（現 :3505-3706（**タスク 4.4 の S2 檻 459 行を :2911 へ挿入したため +459 移動**））。**触っていない**のは `run_text_scale_phase`（:948-990）・`balloon_models` 写像（:545 近傍）・`dpi_phase_with` 本体（:801-859）＝いずれも ker の編集面。**ker への申し送り**: ①ker が text スケール相とバルーンモデル写像だけを触る限りハンクは交わらないが、**同一ファイルゆえ先着後 rebase は必須**——git が自動マージしても行番号を引いた doc・檻コメント（van 側は上記 :1049/:1254/:3505 を明示的に引いている）が静かに嘘になる。②van はタスク 5.2（S2 是正）で `dpi_phase_with`（:801-859）を**判断分岐ごと**改造する予定＝ker が同関数へ触るなら着手前に相談すること。③frame.rs へ新しい消費点（per-scope バルーン採寸の回収等）を足すなら、**`crate::placement::diag::DESPAWNED_SKIP_TAG` を判定語として同じ区別**（entity 不在＝`debug!` で打ち切り他 scope 継続／実在するが規約 component 欠落＝`warn!`）を敷くこと（要件 6.2/6.3・混ぜると終了時ログの良性ノイズが本物の異常を埋める）〕。W5 4本のうち **van⇄ker のみ上記のとおり同一ファイル近接**（残り 5 ペアは相互素）・bind×W6.5 素・exact×W5 素は実測確認済み。`status-execution-states`=台帳 spec（着手しない・源着地時に just-in-time）・`surfaces-basepos`／`sakura-time-directives`=**M2 解禁ゲート**（M1 では着手しない）。

## 着手手順

> **brief 全数完備体制**（2026-07-16 開発者指示以降）: M1 残ユニットは全て brief 済み＝着手は該当 brief を読んで `/kiro-start <unit>` へ直行。新規課題の起票は `/kiro-discovery`（再入）で brief just-in-time 生成。`/kiro-spec-batch` は使わない（一括＝工場化）。ウェーブ跨ぎの合流判断は別セッションで一括（記憶 portfolio-convergence-decided-in-separate-session）。

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

**M2 解禁ゲートの spec（brief 済・M1 では着手しない）**: `areka-P0-surfaces-basepos`・`areka-P0-sakura-time-directives`（互換拡充時に解禁）／`areka-P0-status-execution-states`（残状態の源サブシステム着地時に just-in-time・台帳）。

---

**2026-07-31 追記(52)（棚卸④＝本書き直しの由来）**: W4 完走＋残件2件起票（追記㊿(51)）後の `/kiro-discovery` 再入。①brief 無し spec=0 を確認（全12本完備）。②サブエージェント2体の実測監査——(a) roadmap 構造監査＝413行中 ~65% が純履歴・追記㊻番号衝突等の記録欠陥検出→**履歴を roadmap-history.md へ凍結退避し本体を書き直し**（開発者裁定）。(b) 残9 brief の陳腐化＋干渉再実測＝W4 着地で kero-balloon の per-scope 席保全・collision-dpi-hittest の ÷k 席予約・choice-select-events の `ChoiceSelection` 実物化・dpi-window-vanish の確定 gap 解消（診断やり直し）を確認し、brief 7本へ補正ブロック適用。③**bindoption-exclusivity を W6 同居へ編成**（開発者裁定＝balloon-visibility と実測で素・W5 は kero-balloon と assets.rs 異ハンク衝突ゆえ不可・W7 適合 #3 の前提）。④過積載なし（分割不要）・dpi-window-vanish は縮退可能性あり。次フロント＝**W5 の4本**。
