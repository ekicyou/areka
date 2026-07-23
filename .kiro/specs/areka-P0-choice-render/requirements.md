# Requirements Document

## Introduction

emo2 のメニュー（`menu.pasta`）は `\q[おしゃべり頻度,Onおしゃべり頻度メニュー]` 等で選択肢の**表示**を要求するが、areka にはバルーン内選択肢 UI が存在しない。供給側は着地済み（`CuePlayer` が Choice cue を配送列へ第一級配送しつつ照合用 `pending_choices` へも積み、`WaitForChoice` バリアで停止する二重真実源）だが、**表示器が居ない**——emo-text の typewriter 状態機械は `Choice`／`Cursor`（`\_l`）cue を「actor ごと一度 warn して良性スキップする明示シーム（choice-render の宛先）」として実装済みで、描画・ヒット幾何を持つ住人が居ない。結果、メニューが**見えない**。

本仕様は M-dialogue の**表示半分（描画）**である。Choice cue（＋直後の `WaitForChoice` バリア）を受けた表示層が、バルーン内容キャンバスへ選択肢行を resident として描画し、`\_l[x,y]`（em/lh 単位カーソル）で字下げ配置を効かせ、**注入された hover 状態**に応じて選択肢行のハイライト（fixture 指定の cursor.\* スタイル準拠＝矩形塗り＋文字色切替、未指定バルーンは矩形反転へ縮退）を決定論的に描く。あわせて、選択肢の**行ヒットジオメトリ（行矩形 → 選択肢 id 対応）**と**hover 状態注入 API** の契約を本仕様が正本として確立し、下流 `areka-P0-choice-interact`（実ポインタ配線・hover 追従駆動・クリック解決・`ChoiceSelection` 発行）が消費する。選択肢は talk の一部として `Clear`/`ClearAll`/新 talk で消滅し、表示と hit 幾何を同時（原子的）に無効化する。

本仕様は additive 増分に徹する。上流（dola cue 配送・`pending_choices`／emo-text 状態機械・レイアウト・D2D 描画・`TextSurface`/`TextSlotView`・emo-present）はいずれも完成済みで、本仕様は既存の typewriter/scroll/viewbox 決定論資産と emo-present 本体を改変しない。完了時、実 emo2 でメニューの選択肢行が実 DPI で**見えて**、注入 hover で選択肢行が**光る**（実ポインタ追従駆動とクリック選択は `areka-P0-choice-interact` の領分ゆえ本仕様では判定しない）。

## Boundary Context

- **In scope**:
  - 配送された `Choice` cue（`id`／`text`／`references`）を消費し、選択肢を 1 行 1 項目のテキスト行 resident としてバルーン内容キャンバスへ描画すること（描画は既存グリフ行経路の再利用）。
  - `Cursor` cue（`\_l[x,y]` の不透明転写）を消費し、em/lh 単位を物理 px へ換算してレイアウトカーソルを移動し、選択肢の字下げ配置へ反映すること。
  - 選択肢の**行ヒットジオメトリ（行矩形 → 選択肢 id 対応）の契約正本**を確立し、下流（choice-interact）が照会可能な形で公開すること（幾何の保持であり、実ポインタ照会・クリック解決は行わない）。
  - **hover 状態注入 API の契約正本**を確立し、注入された「ハイライト対象の選択肢」に応じて選択肢行のハイライトを決定論的に描画すること（cursor.\* スタイル準拠＝矩形塗り＋文字色切替を第一候補、cursor.\* 未指定バルーンは矩形反転へ縮退）。ハイライト変化は差分（ダーティ矩形）再描画に乗せ、全域再描画へ退行しないこと。
  - 選択肢のライフサイクル: `Clear`/`ClearAll`/新 talk で選択肢 resident を消滅させ、行ヒットジオメトリを同時（原子的）に無効化すること（表示と hit の片方だけが古い状態を作らない）。
  - 決定論的なエンドツーエンド観測（注入 cue＋注入 hover 状態＋readback pixel・synthetic pointer/sleep 不使用）と、判断分岐の実行テスト網羅。純関数化可能な領域（em/lh 換算・行ヒット幾何導出）の GPU 不要な全網羅。
  - 実 emo2・実 pasta.dll・実 DPI（≠96）でメニュー選択肢行が見えること、および注入 hover でハイライトが光ることの人間サインオフ。
- **Out of scope**:
  - `\q`／`\_l` → cue コンパイル（`completed/areka-P0-sakura-dialogue-tags`・choice cue／`Cursor` cue 形の正本）。
  - 実ポインタ配線・pointer move による hover 追従駆動・クリック解決・`ChoiceSelection`（選択の I/O 契約）の**定義と発行**（すべて `areka-P0-choice-interact` の領分）。
  - 選択確定 → SHIORI カスケード・タイムアウト・`Status: choosing`（`areka-P0-choice-select-events`）。
  - cursor.\* 画像ハイライト（マウスカーソル画像＝別物）・marker.\* キー・`\_a` アンカー・`\__q`（自動改行つき選択肢）・`\![*]`（emo2 の dic に無し・M1 外＝型/語彙シームのみ）。
  - balloonc\*（communicate UI・M2）／選択肢のスクロール完全対応（emo2 メニューは短い＝実測範囲で最小）。
- **Adjacent expectations**:
  - Choice cue の受け取りは、cue-playback の settled 配送モデルに従い**配送された `Choice` cue の消費**で確定する（`pending_choices()` 直読みではなく、配送列＝表示の真実源に乗る）。`WaitForChoice` バリアは供給側が発行済みで、本仕様は「選択肢表示中」を照会可能にするに留める（バリアの解決＝再開は下流の領分）。
  - 選択肢の視覚仕様（cursor.\* スタイルの具体マップ＝style／brush.color／pen.color／font.color／blendmethod、クリック領域が行全幅か文字幅か、em/lh 単位がフォント高基準か行高基準か、負値/省略の縮退）の**正典確定は設計フェーズ冒頭で ukadoc＋SSP 実観察により行う**。emo2 fixture（`emo2-kakukaku/descript.txt` の cursor.\* 実指定・`menu.pasta` の `\n` 区切り 2〜4 項目）は最小適合サンプルであり典拠にしない。本仕様は M1 で emo2 が実使用する形（fixture 指定 cursor.\* スタイル＋短い縦並びメニュー）を実導出し、その他の正典形は語彙・構造を保持しつつ非アクティブに縮退させ差替シームを残す。
  - 面引数（選択肢 `text`・`references`・`\_l` の x,y）はパース/転写段階では不透明に忠実転写され、単位換算・配置解決は表示側（本仕様）の下流責務とする（既存の面引数不透明転写規約と対称）。
  - `ResidentContent` は `#[non_exhaustive]` 済みであり、選択肢住人の表現（既存グリフ行への hit メタ並置か新 variant か）は additive に選定できる（既存住人種の解決・描画を変更しない）。

## Requirements

### Requirement 1: 選択肢 cue の消費と選択肢行の描画

**Objective:** ゴースト作者として、記述した `\q[…]` 由来の選択肢が表示層で捨てられず、バルーン内容キャンバスに選択肢行として実際に描画されることを求める。これにより、メニューが**見える**。

#### Acceptance Criteria

1. When 配送された `Choice` cue（`id`／`text`／`references` を保持）を表示層が消費するとき、the 表示層 shall 当該選択肢を 1 行 1 項目のテキスト行 resident としてバルーン内容キャンバスへ描画対象に加える（`Choice` cue の良性スキップシームを描画へ置換する）。
2. When 複数の `Choice` cue を順に消費するとき、the 表示層 shall 各選択肢を配送順に対応する複数の選択肢行として保持し、`text` の文字列内容を忠実に（不透明転写のまま）描画する。
3. When `Choice` cue 列に続いて `WaitForChoice` バリアで供給側が停止しているとき、the 表示層 shall 「選択肢表示中」を外部から照会可能な状態として提示する（照会のみ・バリアの解決は行わない）。
4. The 表示層 shall 選択肢行の描画を既存のグリフ行描画経路の再利用で行い、既存の本文テキスト（typewriter/scroll）の描画・決定論挙動を変更しない（additive 増分）。
5. If `text` が空、または `Choice` cue が 1 件も無いまま `WaitForChoice` バリアに至る台本であるとき、then the 表示層 shall 当該事象を致命扱いせずログとして記録し、既存の待機状態観測（空の選択肢集合）を保つ。

### Requirement 2: `\_l` カーソルの消費と選択肢の字下げ配置

**Objective:** ゴースト作者として、`\_l[x,y]`（em/lh 単位）によるカーソル移動が選択肢の配置に効き、fixture どおりの字下げ配置で選択肢が並ぶことを求める。これにより、メニューの見た目が正典に忠実になる。

#### Acceptance Criteria

1. When `Cursor` cue（`\_l[x,y]` の不透明転写）を消費するとき、the 表示層 shall x を em 単位、y を lh（行高）単位として物理 px へ換算し、以降のコンテンツ配置に用いるレイアウトカーソルを当該位置へ移動する。
2. The 表示層 shall em/lh → 物理 px の換算をフォントメトリクスと表示層の座標空間契約（image px 空間・スケール）に整合させ、DPI（≠96）でも一貫した物理配置を与える。
3. When カーソル移動後に選択肢行が配置されるとき、the 表示層 shall 移動後のカーソル位置を起点として選択肢行を字下げ配置し、その配置を行ヒットジオメトリ（Requirement 3）へ反映する。
4. If `\_l` の座標が負値または省略されるとき、then the 表示層 shall 正典（設計フェーズで ukadoc 確定）に定めた縮退挙動へ従い、未確定の形は語彙を保持したまま安全に縮退（ログ記録のうえ状態不変スキップ）させる。
5. The 表示層 shall `Cursor` cue の消費を既存の改行遅延（newline-defer）・折返し（budoux-newline）の不変条件と整合させ、既存レイアウト挙動を退行させない。

### Requirement 3: 選択肢の行ヒットジオメトリと id 対応の契約

**Objective:** 保守者として、各選択肢行のクリック可能領域（行矩形）と選択肢 id の対応が、下流（choice-interact）が照会可能な第一級の契約として提供されることを求める。これにより、対話半分がポインタ解決を組み立てられる接続点が確立する。

#### Acceptance Criteria

1. When 選択肢行が配置・描画されるとき、the 表示層 shall 各選択肢行のヒット矩形（表示座標系）と対応する選択肢 id を結び付けた行ヒットジオメトリを保持する。
2. The 表示層 shall 行ヒットジオメトリを下流が照会可能な形で公開し、当該契約（行矩形 → 選択肢 id 対応）の正本を本仕様が所有する。
3. The 表示層 shall 行ヒットジオメトリを字下げ配置（Requirement 2）・スクロール可視窓の反映後の実表示位置に一致させ、描画された選択肢行と hit 領域が座標整合する（描画とヒットの片方だけがずれた状態を作らない）。
4. The 表示層 shall 行ヒットジオメトリの導出を純粋なレイアウト計算として GPU 不要に全網羅可能な形で構成する。
5. The 表示層 shall 実ポインタの照会・クリック解決・`ChoiceSelection` の発行を行わない（それらは `areka-P0-choice-interact` の領分であり、本仕様は幾何と id 対応の提供に留める）。

### Requirement 4: hover 状態の注入とハイライト描画

**Objective:** ゴースト作者として、選択肢にカーソルを合わせた行が視覚的にハイライトされ、どれを選ぼうとしているかが分かることを求める。これにより、選択肢が**光る**。

#### Acceptance Criteria

1. The 表示層 shall 「ハイライト対象の選択肢（またはハイライト無し）」を外部から注入できる hover 状態 API を公開し、当該 API 契約の正本を本仕様が所有する（注入駆動＝決定論・実ポインタ非依存）。
2. When ある選択肢行が hover 状態として注入されているとき、the 表示層 shall 当該行のハイライト（バルーンが cursor.\* スタイルを指定していれば指定色の矩形塗り＋文字色切替）を選択肢行の描画に重ねる。
3. Where バルーンが cursor.\* スタイルを指定していないとき、the 表示層 shall ハイライトを矩形反転（設計で確定する縮退仕様）で描画する。
4. When hover 状態が変化する（対象行の切替・ハイライト無しへの復帰）とき、the 表示層 shall 影響領域のダーティ矩形のみを差分再描画し、キャンバス全域の再描画へ退行しない。
5. While hover 状態が「ハイライト無し」であるとき、the 表示層 shall いずれの選択肢行にもハイライトを描画しない。
6. The 表示層 shall ハイライトをバルーン内容キャンバス内で合成し、emo-present（present 層）を改変せずに完結させる（1 枚物合成の思想に従う）。

### Requirement 5: 選択肢ライフサイクルと表示・hit の原子的無効化

**Objective:** ゴースト作者として、talk が切り替わったときに前の選択肢が残らず、表示と当たり判定が同時に消えることを求める。これにより、古い選択肢を誤って選べる状態が生じない。

#### Acceptance Criteria

1. When `Clear`／`ClearAll`／新しい talk の開始により内容がリセットされるとき、the 表示層 shall 保持していた選択肢行 resident を消滅させ、当該選択肢を描画対象から除外する。
2. When 上記のリセットが発生するとき、the 表示層 shall 行ヒットジオメトリ（Requirement 3）を同時（原子的）に無効化し、表示と hit の片方だけが古い状態に残らないようにする。
3. When 新しい talk が新しい `Choice` cue 列を伴うとき、the 表示層 shall 前の選択肢を残さず新しい選択肢集合のみを描画・ヒット対象として保持する。
4. The 表示層 shall 選択肢の消滅を hover 状態のクリア（ハイライト無し）と整合させ、消滅済み選択肢へのハイライトが残らないようにする。

### Requirement 6: M1 取り扱い範囲と縮退境界

**Objective:** 保守者として、選択肢表示の正典形のうち emo2 が実使用する形を実導出し、残る形は語彙・シームを保持したまま安全に縮退させる境界を求める。これにより、正典機能を第一級に保ちつつ M1 の描画範囲を明確化する。

#### Acceptance Criteria

1. The 表示層 shall 短い縦並びメニュー（`\n` 区切り 2〜4 項目）＋fixture 指定 cursor.\* スタイル＋`\_l` 字下げを M1 の実導出対象とする。
2. The 表示層 shall cursor.\* 画像キー（マウスカーソル画像＝別物）・marker.\* キー・`\_a` アンカー・`\__q`（自動改行つき選択肢）・`\![*]` を M1 では実導出せず、型/語彙シームとして保持する。
3. The 表示層 shall 選択肢のスクロール完全対応を M1 では追わず、emo2 メニュー実測範囲（短いメニュー）の描画・ヒット整合に留める。
4. The 表示層 shall `ChoiceSelection`（選択の I/O 契約）の定義・発行、実ポインタ配線、hover 追従駆動、クリック解決を本仕様で行わず、`areka-P0-choice-interact` の領分として明示的に除外する。
5. Where 正典で未確定・M1 非対象の形（cursor.\* 未指定の縮退・`\_l` の負値/省略・emo2 未使用の cursor.\* サブキー）が与えられるとき、the 表示層 shall 当該入力を致命扱いせず語彙を保持したまま安全に縮退（ログ記録のうえ既定/スキップ）させる。

### Requirement 7: 決定論的なエンドツーエンド観測とテスト網羅

**Objective:** 開発者として、注入 cue と注入 hover 状態から選択肢の描画・字下げ・ハイライト・ライフサイクルまでを決定論的に観測できることを求める。これにより、選択肢表示パイプラインが回帰檻で保護される。

#### Acceptance Criteria

1. When `Choice` cue×3＋`\_l` cue を注入するとき、the 表示層 shall キャンバス readback 上で選択肢 3 行と字下げ配置の描画を観測可能にする。
2. When ある選択肢行の hover 状態を注入する／解除するとき、the 表示層 shall ハイライト矩形（cursor.\* スタイル準拠）の on/off 対を pixel 檻として観測可能にする。
3. When `Clear`／新 talk を注入するとき、the 表示層 shall 選択肢行の消滅と行ヒットジオメトリの無効化を観測可能にする。
4. The 表示層 shall 上記の観測を synthetic pointer・sleep を用いず、注入した cue・hover 状態・Tick のみで決定論的に成立させる。
5. The 表示層 shall 各増分点（cue 消費・`\_l` 換算・行ヒット幾何導出・ハイライト描画・ライフサイクル無効化）の入力依存の判断分岐を実行テストで網羅し、em/lh 換算と行ヒット幾何導出を純関数として GPU 不要で全網羅する。
6. The 表示層 shall 実フォントでの出力画像目視（既定フォント盲点の回避）を pixel 檻と併用し、本仕様の検証に必要な最小適合 fixture（cursor.\* 指定バルーン・短いメニュー）を test-local に自前で用意する。

### Requirement 8: 実機による選択肢表示の人間サインオフ

**Objective:** 開発者として、実ゴースト・実描画環境でメニューの選択肢行が実際に見え、注入 hover で光ることを人間の目視で確認できることを求める。これにより、メニューが見えない欠陥が解消されたことを保証する。

#### Acceptance Criteria

1. When 実 emo2・実 pasta.dll・実 DPI（≠96）でメニューを表示するとき、the 表示層 shall 選択肢行を `\_l` 字下げ配置のとおりに可視状態で提示する。
2. When 実機表示中に注入 hover 状態を与えるとき、the 表示層 shall 対象選択肢行のハイライトを目視可能に提示する。
3. The 表示層 shall 実機サインオフを本番ゴースト表示を先行させたうえで行い、単発デモへの合わせ込みを判定根拠にしない。
4. The 表示層 shall 実ポインタ追従駆動によるハイライトとクリック選択を本仕様の実機判定に混ぜない（それらは `areka-P0-choice-interact` の領分）。
5. Where 実機起動を行うとき、the 表示層 shall pasta.dll を `LoadLibrary` 可能にするため絶対パスで起動する。

### Requirement 9: 既存資産の非退行（additive 制約）

**Objective:** 保守者として、完成済み上流エンジンへの本増分が既存の全テスト緑・依存方針・ビルド制約を崩さないことを求める。これにより、additive 原則が担保される。

#### Acceptance Criteria

1. The 表示層 shall 本増分の適用後も `cargo test --workspace` を exit 0 で成功させ、既存テストをすべて緑に保つ。
2. The 表示層 shall 新規の外部（crates.io）依存を追加しない。
3. The 表示層 shall Rust 2024 で構築し、tokio を導入しない。
4. The 表示層 shall WUC/D2D 操作を UI スレッド固定で行い、既存のスレッド親和制約を破らない。
5. The 表示層 shall 既存の typewriter/scroll/viewbox 決定論資産・既存住人種（`ResidentContent` の解決・描画）・既存 cue ワイヤ形を変更せず、新しい cue variant を新設しない（既存の `Choice`／`Cursor` cue の消費に留める）。
6. The emo-present crate 本体 shall 本増分によって改変されない（描画はバルーン内容キャンバス内で完結し、`TextSlotView` の読み口増分が要る場合も additive に留める）。
