# Brief: areka-P0-position-persist

> **種別**: 本坑（main）増分。⓪ ghost 帰属（M-life 構成要素）。roadmap 増分「⓪ ghost: `position-persist`（`ghost.dat` 位置の保存/復元・ghost レベル永続化）」の brief 化。
> **調査日**: 2026-07-16（再入精査⑦・実装シーム偵察＋ukadoc 裏取り）。
> **並走性**: **cue モデル完全非接触**＝実装中の `areka-P0-cue-playback-duration` と**真に並走可**（dola/sakura/emo-text/seriko のいずれも触らない）。

## Problem

areka は再起動のたびにゴースト窓が既定位置（右下吸着）へ戻り、ユーザーが決めた位置を忘れる。さらに **OnFirstBoot が毎回発火**する（起動記録を持たないため「初回かどうか」を判定できない）——emo2 は毎起動で初回挨拶を喋り、SSP 互換の「2回目以降は OnBoot」にならない。「伺かアプリとしての体裁」（位置を覚える・初回と通常起動を区別する）に必要な **ghost レベルの永続化層が不在**。

## Current State

- **窓位置の決定フロー（実シンボル・2026-07-16 偵察）**: spawn 時は `resolve_placement`（`crates/areka/src/placement/resolver.rs`・`mod.rs:106-112` 経由）→ `spawn_ghost_windows`（`spawn.rs:139-233`）が外部引数 `placements` から `WindowPos`（物理 px・`spawn.rs:245-251`）へ転記。**初期位置を外から与える注入口は既にある**（`placements` 引数）。
- **restore の口は無い**: `prepare_stages`（`placement/mod.rs:121-131`）にコメント「位置の記憶・復元（ghost.dat）は一切行わない」と明記・テストで固定（`mod.rs:503-565`＝ghost.dat を plant しても出力不変）。**本 spec がこの檻を意図的に更新する**（陳腐化＝仕様判断で退役）。
- **ドラッグ後の位置**: 非 Free 窓は `on_char_drag`（`follow.rs:260-302`・単一ライター）が `WindowPos` へ書込。**保存トリガに使える観測点**＝`on_char_drag_end`（`follow.rs:319-350`）・バルーン単独ドラッグ `on_balloon_drag`（`follow.rs:443-488`）。バルーン相対 offset は**セッション内のみ記憶**（`follow.rs:220-223` コメント「永続化 ghost.dat は M-life の領分」＝**本 spec 宛の申し送り**）。
- **vanish count はハードコード "0"**: `events.rs:47-52`（コメント `events.rs:44-46`「M1 は vanish count 等の永続値を持たない…固定値 "0"」）。boot cascade は毎回 `OnInitialize`→`OnFirstBoot`→(204)→`OnBoot`（`crates/areka-kanade/src/schedule/boot.rs:38-83`）＝**初回判定なしで OnFirstBoot を常に発射**。
- **永続ファイルを読む口は無い**: `GhostRuntime::boot`（`crates/areka-ghost/src/runtime.rs:301-389`）・`config.rs:28-67`（shell descript の `name` のみ）。ghost path 配下への書込み慣行も無し。
- **復元に使える公開 API**: `move_window_to`（`follow.rs:500-519`・物理 px・BalloonFollow 随伴）／`resize_window_to`（`follow.rs:551-628`）／spawn 時注入（上記）。surface-resize-resnap ✅ の `project_anchor`（5 アンカー射影）＋単一ライターが再吸着の既存正本。

## Desired Outcome

窓をドラッグ→終了→再起動で**前回位置に復元**され、**2回目以降の起動は OnFirstBoot を跳ばして OnBoot から**始まる。永続状態（窓位置・バルーン相対 offset・起動記録・vanish count 構造）はゴースト単位のファイル（以下 ghost.dat と呼称・形式は areka 自由）に保存される。

**✔ 観測（単一 pass/fail）**: 決定論 unit＝(a) 保存→復元 roundtrip で位置・offset・起動記録が値等価 (b) 破損/欠損ファイル→warn ログ＋既定位置へ寛容縮退 (c) モニタ構成変化（保存位置が work_area 外）→アンカー再射影で画面内へ縮退 (d) 初回（ファイル無し）は OnFirstBoot・2回目以降は boot cascade が OnFirstBoot を skip。＋実機＝ドラッグ→終了→再起動→位置一致の人間サインオフ。

## Approach

1. **永続モデルと IO（⓪ ghost 所有・新モジュール）**: `crates/areka-ghost`（または areka 本体）に versioned な `GhostState`（scope 別窓位置〔物理 px＋モニタ識別 or 論理正規化＝design で確定〕・バルーン相対 offset・booted 記録・vanish count）＋ atomic 書込（temp→rename）・読取寛容（破損＝既定縮退・[[areka-log-first-no-silent-failure]]）。**保存先ディレクトリは design 冒頭で確定**（ukadoc `file_structure` の profile 慣行を必読——SSP は `ghost/master/profile/` 系。areka 名前空間サブディレクトリ推奨・fixture を汚さない配慮も検討）。
2. **復元＝spawn 注入**: boot 時に GhostState を読み、`spawn_ghost_windows` の `placements` へ**復元値 ∪ 既定 resolver** の純粋 merge を注入（restore 有→復元位置・無→従来 resolver）。復元位置は `project_anchor` 系で work_area へ再射影（モニタ構成変化の縮退・bottom 吸着維持）。
3. **保存＝DragEnd＋shutdown flush**: `on_char_drag_end`／`on_balloon_drag` の系に保存トリガ（頻度は design 判断＝DragEnd 毎 or dirty フラグ＋終了時 flush）。`main.rs` の shutdown 経路（`main.rs:315-328`）で最終 flush。
4. **OnFirstBoot ゲート**: kanade の boot cascade へ「初回か否か」を**構築時パラメータで注入**（`resolve_kanade_config` 系）——2回目以降は OnFirstBoot を skip して OnBoot 直行（ukadoc: OnFirstBoot は「初回起動した際に発生」・204 なら OnBoot フォールスルーは既存実装済み）。vanish count は GhostState から Ref0 へ転記（M1 は `\![vanish]` 未実装ゆえ常に 0 だが**読取経路を正**にする＝`events.rs:44-52` の固定値コメント退役）。
5. **檻の更新**: `placement/mod.rs:503-565` の「ghost.dat 不使用」固定テストを本仕様の新契約（plant→復元される）へ書き換え（[[obsolete-vs-broken-test-policy]]）。

## クロスユニット契約（並走を詰ませない事前考慮・2026-07-16）

- **cue-playback-duration と交差面ゼロ**: 本 spec の編集面＝placement（follow/spawn/mod）・areka-ghost（config/runtime 注入）・kanade boot（events/boot の Ref0 とゲート）・main flush。dola cue/sakura compile/emo-text state/seriko には**一切触れない**＝時限ゲート非該当・真並走。
- **input-events（撫でクラスタ）との近接**: input-events は `spawn.rs` の pressed ハンドラ（`on_ghost_pressed` `spawn.rs:321-344`）を差し替える。本 spec は `follow.rs` の DragEnd 観測＋`spawn.rs` は placements 注入（`:139` 引数）のみ＝**別関数・additive**で並走可（同時マージの近接編集に注意・どちらも新規アーム/フック追加に留める）。
- **kanade 構築時注入の形**: boot cascade への「初回フラグ＋vanish count」は **kanade 構築 config の additive フィールド**とし、kanade の純粋状態機械の決定論テスト資産を壊さない（既存テストは既定値＝毎回 OnFirstBoot で不変に保てる形を design で確認）。
- **将来消費者**: `\![vanish]`（M2）が vanish count のインクリメントを、ゴースト切替（M2）が per-ghost 状態の分離を、本 spec の GhostState をそのまま使う——**ghost 識別子キーの構造だけ**最初から持つ（過剰実装はしない）。

## ukadoc 必読（design 着手時に ukadoc MCP `get_doc`/`search_docs` で正典参照・2026-07-16 裏取り済み）

- **`ukadoc:list_shiori_event:OnFirstBoot:1`**: 「初回起動した際に発生。SSP では 204 の場合、続けて OnBoot が発生」・**Reference0＝vanish された回数**（裏取り済み・kanade 既存実装の 204 フォールスルーは正典どおり）。
- **`file_structure` カテゴリ**: ゴーストフォルダの profile 慣行（保存先ディレクトリの確定材料）。ghost.dat の**中身の形式は正典に無い**（baseware 自由）＝areka versioned 形式で可、と design に明記。
- **`descript_shell` の `seriko.alignmenttodesktop`**: 復元位置と bottom 吸着の整合（復元後も吸着規則が優先＝`project_anchor` 再射影）。

## Scope

- **In**: GhostState 永続モデル＋atomic IO＋寛容読取／spawn への復元注入（merge 純関数）／DragEnd・shutdown の保存トリガ／バルーン相対 offset の永続化（session-only の持ち上げ）／OnFirstBoot 初回ゲート＋vanish count 読取経路（値は 0）／モニタ構成変化の縮退再射影／既存「不使用」檻の契約更新。
- **Out**: `\![vanish]` の実装（M2・カウント増分の発生源）／ゴースト切替・多重ゴースト（M2）／ウィンドウ以外の設定永続化（音量等・存在しない）／`\![move]` との相互作用（M-dialogue・`sakura-dialogue-tags`）／SSP の ghost.dat バイナリ互換（不要・areka 自由形式）。

## Boundary Candidates

- 永続モデル＋IO（純粋・決定論全網羅）／復元 merge（純関数）／保存トリガ結線（UI 観測点）／kanade 初回ゲート（構築時注入）。

## Out of Boundary

- 窓生成・既定位置解決そのもの（window-placement ✅ 完了・本 spec は入力を差すだけ）／再吸着機構（surface-resize-resnap ✅ の `project_anchor` を消費）。

## Upstream / Downstream

- **Upstream**: `completed/areka-P0-window-placement`（placements 注入口・DragEnd 観測点・座標契約=物理 px 単一通貨）／`completed/areka-P0-surface-resize-resnap`（`project_anchor` 再射影・単一ライター）／`completed/areka-P0-ghost-setup`（GhostRuntime::boot・config 解決層）／`completed/areka-P0-kanade`（boot cascade・events）。
- **Downstream**: M-life（統合点）／M2 `\![vanish]`・ゴースト切替（GhostState 構造の消費者）／`emo2-conformance-e2e`（一周適合に「再起動で位置維持」を含め得る）。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-P0-window-placement`（follow.rs:220-223 の「M-life の領分」申し送りを履行）。
- **Adjacent**: `areka-P0-input-events`（spawn.rs 近接・別関数＝並走可）／`areka-P0-cue-playback-duration`（**交差面ゼロ**）／`areka-P0-idle-talk`（kanade 近接・boot.rs vs steady.rs で別フェーズ＝並走可・events.rs は双方 additive）。

## Constraints

- Rust 2024・新規 crates.io 依存なし（serde 系が要るなら既存ツリー内依存で・追加は要承認）・tokio 不使用。
- **決定論**: IO 以外は純関数化して全網羅（[[deterministic-test-coverage-mandate]]・[[test-only-decision-branches-not-proven-wiring]]）。ファイル IO は temp dir 注入で決定論 unit。
- **実機受け入れ**: 実 emo2・実 DPI（≠96）・マルチモニタでドラッグ→再起動→位置一致（[[areka-placement-real-ghost-first]]）。起動は絶対パス必須（相対だと helper が pasta.dll を LoadLibrary できず MOD_NOT_FOUND）。
- 失敗経路は error!/warn!＋縮退（[[areka-log-first-no-silent-failure]]）——永続化の失敗で起動を殺さない。
