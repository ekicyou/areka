# Requirements Document

## Introduction

emo2（互換ベースウェアの適合 fixture ゴースト）の**メニュー・位置調整・撫で talk が使う 4 語彙**は、parse は成功するのに sakura の compile catch-all（「M-boot 外タグを無視」）で**全て無音落ち**している。結果、ユーザーから見える機能が丸ごと存在しない:

- **`\q[タイトル,イベント名]`**（`menu.pasta` 9 箇所）→ 選択肢が cue にならず、**ダブルクリックメニューが存在しない**。
- **`\_l[5em,2lh]`**（menu 3 箇所・選択肢の区切り位置指定）→ カーソル移動が cue にならず、**メニューの体裁が崩れる**。
- **`\![move,-353,,,0,base,base]`**（`boot.pasta` の **OnFirstBoot**・`menu.pasta` の位置調整）→ 移動が cue にならず、**初回起動時のエモ（相方側）の位置調整が黙って失われている**。
- **`%username`**（`touch.pasta` 2 箇所）→ 展開されず、**撫で talk のバルーンに生文字列 `%username` が露出する**（環境変数の展開はベースウェアの義務＝ukadoc `OnTranslate`「ベースウェアによる環境変数の展開などの後に再び SHIORI へ送られる」）。

本 spec は、この 4 語彙を **settled な cue モデル**（`completed/areka-P0-cue-playback-duration`＝envelope 一律 duration・自己完結した絶対時刻台本・単一 `CueSink`・relevance 単一権威・broadcast 配送・占有 horizon での完了）の上へ **additive に載せ**、fixture script の直入力から**決定論的に正しい cue／barrier 列**が得られ、`\![move]` は**末端まで貫通して実機の初回起動でエモが横へ動く**ところまでを実現する。

本 spec は M-dialogue（メニュー一周）の**先鋒＝契約の正本**である。**choice cue の形**（表示ラベル／ID／references の載せ方）と**「選択肢群＋選択待ち barrier」の並び規則**は本 spec が確定し、下流の `areka-P0-choice-render`（表示）と `areka-P0-choice-select-events`（選択確定カスケード）は**消費のみ**を行う。さらに **`\!` 汎用コマンドキャリア cue の形**も本 spec が確定する（2026-07-18 裁定＝正典実測 183 コマンド〔下限・非有界〕の個別 typed 化は破綻するため、単一の不透明キャリア＋消費側のコマンド名選別。`\![bind]` を消費する `areka-P0-mayuna-compose` 以降の全 `\!` 消費者はこのキャリアに乗る）。

正典は ukadoc であり、emo2 は最小適合 fixture にすぎない（正典が沈黙する箇所は areka 裁量＋対応表記録＝互換契約）。

## Boundary Context

- **In scope**:
  - `\q`（選択肢）の cue 化＝**表示ラベル／ID／references を欠落なく運ぶ choice cue の形の確定**（下流の消費契約の正本）。
  - **選択待ち barrier の並び規則の確定**（選択肢を含む talk は選択が解決されるまで完了しない）。
  - `\_l`（カーソル位置）の cue 化＝**単位・相対指定を不透明文字列のまま転写**（解釈は消費側）。
  - **`\!` ベースウェアコマンド名前空間全体の汎用キャリア cue 化**（単一の不透明キャリア＝契約の正本。正典 183 コマンド〔下限〕の全転写・コマンドごとの typed cue 語彙の個別新設は禁止・消費はコマンド名による消費側選別）。
  - `\![move]`（キャラクタ移動）＝汎用キャリアの最初の実消費者＋**末端まで貫通した実際の窓移動**（即時移動のみ・随伴バルーン込み）。
  - `%username` の**展開**（値源は起動構成からの注入・未注入時は既定値）と、未対応システム変数名の**素通し縮退**。
  - 既存の「無視されるタグ」仕様（除外の檻）の**意図的更新**（対象 4 語彙の卒業）と、既存 cue 挙動の非退行。
  - fixture script 直入力による**決定論的検証**と、実 emo2 初回起動での**実機サインオフ**（エモの位置調整）。
- **Out of scope**:
  - 選択肢の**表示・UI・ヒットテスト・ハイライト**、および `\_l` の単位換算（em/lh/%）＝`areka-P0-choice-render` の領分。
  - **選択確定→SHIORI カスケード**（任意名イベント直接発火／`OnChoiceSelect(Ex)` の判別規則）・選択のタイムアウト時間の決定・`OnChoiceTimeout`＝`areka-P0-choice-select-events` の領分。
  - `\![bind]` の**消費**（seriko の動的 bind 状態＝`areka-P0-mayuna-compose`）・`\![raise]` 等その他コマンドの**消費**（M1 外）。※いずれも cue 化（転写）は本 spec の汎用キャリアが行い、compile の無音落ちには戻さない——縮退は消費側の良性スキップ。
  - **compile 側 allowlist（時間指令系）の実導出**（`quicksection`／`set,balloonwait`／`set,choicetimeout`／`set,balloontimeout`／`embed`／`sound,wait`／`wait,syncobject` 等＝R4.3 の但し書き。emo2 未使用・語彙保持＋縮退のみ。実導出は源が着地した時点の追跡 spec へ）。
  - **時間指定付きの移動アニメーション**（emo2 未使用＝M1 は即時移動へ縮退し語彙のみ保持）。`\![moveasync]` も同様に M1 外。
  - **選択肢タイムアウト属性**（`\*` ／ `\![set,choicetimeout,時間]`＝スクリプト単位属性・fixture 未使用）の実導出。
  - **位置の永続化そのもの**（`ghost.dat` 保存/復元＝`areka-P0-position-persist`）。本 spec は「`\![move]` が永続値を書かない」ことのみを担保する。
  - **プロパティシステム本体の設計・所有**（名前→値の解決機構・値の所有と永続化・live/SHIORI/永続の各 backing・`%property[...]` dotted ツリーの解決）＝`areka-P0-sylphya`（**brief 確定済**・crate `crates/areka-sylphya`・最下層配置は依存グラフが強制）の領分。本 spec は sakura が**読み口スナップショットを消費する**ところまでで、機構は持たない。
  - `%username` 以外のシステム変数の**実導出**（`%selfname`/`%keroname`/`%property[...]`/`%m*` 等＝源が着地した時点で just-in-time・値源は sylphya が用意）。
- **Adjacent expectations**:
  - **cue 再生の settled モデルは既に main に在る**（duration・絶対時刻台本・broadcast・選択待ち状態と選択解決の口）。本 spec は**そこへ別アームを足す**形であり、時間モデル・配送モデル・完了判定の**規則そのものを再定義しない**。
  - **選択の解決（ユーザーのクリック→どの選択肢が選ばれたか）を起こすのは下流**（choice-render／choice-select-events）。本 spec は「選択待ちで止まり、解決されたら再開する」台本側の契約のみを確定する。
  - **choice cue の配送は責務二分**（設計判断#1 帰結）: **配送列＝配置/表示情報の単一真実源**（choice-render が cue 列として消費）／**先積みバッグ＝解決照合の単一真実源**（choice-select-events が `resolve_choice` の id 照合に使う）。本 spec は「choice cue を他 cue と順序を保って配送する」ところまでを正本として確定し、settled な先積み一択の観測挙動を仕様変更として明示的に更新する（R8.6）。表示・ヒットテストは choice-render の領分。
  - **選択解決の口は本 spec が定義・W5 が消費**（R2.7）: 解決は talk アクター境界の型付き入力（`SakuraMsg` の additive アーム）経由でのみ到達する。`CuePlayer::resolve_choice` を**外部から直接呼ぶ経路は構造的に存在しない**（アクター内に閉じている）ため、`areka-P0-choice-select-events` は「`resolve_choice` を直接呼ぶ」のでなく本 spec が定義するこの口へ選択 id を投入する。W5 brief の旧記述（直接呼び出し前提）は本 spec 着地時に訂正する申し送り済み。
  - **`\!` 汎用キャリアは W2（mayuna-compose）へ申し送り済み**（2026-07-18 調停）: 同 brief の typed `CueCommand::Bind` variant 計画（dola variant＋compile アーム＋`cue_target_of` アーム＋emo-text 無視列挙）は本キャリア裁定で**廃止へ差替**——mayuna は「汎用 cue のコマンド名 `bind` の消費者」（parsers 名前解決表・seriko 動的 bind 状態・emo-present 回帰は存続）へ縮小し、共有編集面 4 ファイルの近接警告は W1 への一方向依存に解消される。境界原則: **コンテンツタグ（`\s`/`\b`/`\q`/`\_l` 等）＝typed cue／`\!` コマンド名前空間＝汎用キャリア**（balloon-face-cue の「同型」引用はコンテンツタグにのみ有効）。
  - **`\_l` 直後の行揃えリセット**（ukadoc: `\_l` 実行直後は左揃えへ戻る）や `@` 相対指定の解決は**表示側の責務**であり、本 spec は記述を欠落なく運ぶことに徹する。
  - **`\![move]` の位置は永続化されない**（ポートフォリオ合流裁定＝保存値はユーザーの明示的ドラッグ確定のみが更新する二層分離）。その帰結として、`areka-P0-position-persist` の初回ゲート導入後は**未ドラッグの 2 回目以降の起動で初回位置調整が既定配置へ戻る**——これは許容仕様であり、最終確認は `areka-P0-emo2-conformance-e2e` の実機適合走行へ申し送る。
  - **システム変数の値源は sylphya（プロパティシステム）が用意し、⓪ ghost が talk 開始時に読み口スナップショットとして手渡す**。本 spec は sakura がそのスナップショットを消費する契約（R7）のみを確定し、sylphya の実体着地を待たずに W1 を出荷できる（emo2 は 204 固定＝暫定 provider でも本実装でも観測は既定値で不変）。discovery 完了＝`areka-P0-sylphya` brief 確定（既存 spec への具体デルタ・ウェーブ配置提案・roadmap 宿題は同 brief 末尾「申し送り」節が正本）。本 spec のブロッカーでないことは不変。W1 の暫定 provider（ghost が既定値スナップショットを充填）は sylphya 着地時に sylphya 読み口からのスナップショット生成へ**差し替える**（sakura 側契約は無改変＝差替シーム）。
  - 実機起動は**絶対パス必須**（相対パスでは SHIORI helper が DLL を読めず MOD_NOT_FOUND）。

## Requirements

### Requirement 1: 選択肢 `\q` の cue 化（choice cue 形＝下流の正本）
**Objective:** sakura として、`\q` の記述内容を情報欠落なく choice cue へ写像したい。そうすれば、選択肢を表示する側（choice-render）と選択確定を SHIORI へカスケードする側（choice-select-events）が、正典の ID 規則を後から再現でき、契約の再定義や二重解釈が起きない。

#### Acceptance Criteria
1. When `\q[タイトル,ID]` を含む talk script をコンパイルする, the sakura コンパイラ shall 当該選択肢に対応する choice cue を発行する（無音で破棄しない）。
2. The choice cue shall 表示ラベル（第 1 引数）と ID（第 2 引数）を**区別可能な別データ**として保持する（正典の引数順＝第 1 = タイトル・第 2 = ID）。
3. When `\q` が第 3 引数以降（references）を伴う, the sakura コンパイラ shall それらを**記述順を保った参照列**として choice cue に保持し、欠落させない。
4. The sakura コンパイラ shall `\q` の各引数を**不透明な文字列**として転写し、ID の解釈（`On` 始まり＝任意名イベントの直接発火／`script:` 形／複数 ID 形の判別／Reference 番号の割付／カスケード則）を行わない。
5. The choice cue shall 発行時点の**現在スコープ**（`\0` 本体側／`\1` 相方側）へ帰属する。
6. When 同一 talk に複数の `\q` が現れる, the sakura コンパイラ shall **スクリプト内の記述順**を保った順序で choice cue を発行する。
7. Where `\q` が正典の旧仕様形（`\q[ID][タイトル]`）または `script:` 形である, the sakura コンパイラ shall M1 では従来通り実導出せず、記述を失わない縮退（無視の記録）に留める（emo2 未使用・語彙は下流の裁定へ残す）。
8. When choice cue が消費側へ配送される, the cue 再生ランタイム shall 台本内の他 cue（改行・カーソル・テキスト等）との**相対順序を保って**配送し、選択肢と非選択肢の交互配置（例: `\_l` が最後の選択肢を一段下げる体裁）を下流が再構成可能にする（choice cue を配送列から隠し持たない）。

### Requirement 2: 選択待ち barrier の並び規則（停止と再開の契約＝下流の正本）
**Objective:** sakura として、選択肢を含む talk が**ユーザーの選択を待って停止し、選択で再開する**台本になるようにしたい。そうすれば、メニューが「表示された直後に勝手に終わる」ことなく、階層メニューの往復が成立する。

#### Acceptance Criteria
1. When talk script が 1 つ以上の `\q` を含む, the sakura コンパイラ shall 当該 talk 台本へ**選択待ち barrier をちょうど 1 つ**発行する。
2. The 選択待ち barrier shall 台本内の**全 choice cue より後**に位置する。
3. While 選択待ち barrier に到達して未解決である, the cue 再生ランタイム shall 後続 cue を発火させず、当該 talk を**完了として扱わない**（選択待ちのまま talk 完了を通知しない）。
4. When 選択が解決される, the cue 再生ランタイム shall 停止していた台本の再生を再開する。
5. Where talk script に `\q` が 1 つも含まれない, the sakura コンパイラ shall 選択待ち barrier を発行せず、既存 talk の完了挙動を変えない。
6. The sakura コンパイラ shall 選択待ちに**タイムアウト時間を指定しない**（M1 は無期限待ち）。タイムアウト時間の決定と時間切れ時の振る舞いは本 spec の範囲外とし、語彙（タイムアウト指定の口）のみ保持する。
7. The sakura talk アクター shall 選択解決を**アクター入力（`SakuraMsg` の additive アーム）として受け取る型付きの口**を定義する。`CuePlayer::resolve_choice` はアクター内に閉じ外部から直接呼べないため、解決はこの talk アクター境界の入力経由でのみ到達する（既存の `Start`／`Tick`／`Close` と同格の入力種を 1 つ足す・`#[non_exhaustive]` ゆえ非破壊）。本 spec はこの口を**定義するのみ**であり、口を叩く消費（ユーザークリック→選択確定→SHIORI カスケード）は下流 `areka-P0-choice-select-events`（W5）が行う。

### Requirement 3: カーソル `\_l` の cue 化（不透明転写）
**Objective:** sakura として、`\_l` のカーソル位置指定を記述通りに cue へ転写したい。そうすれば、単位（em/lh/%/裸数値）や相対指定の解釈を持つ表示側が、後から正典どおりに解決できる。

#### Acceptance Criteria
1. When `\_l[x,y]` を含む talk script をコンパイルする, the sakura コンパイラ shall 対応する cursor cue を発行する（無音で破棄しない）。
2. The cursor cue shall x・y を**記述通りの不透明な文字列**として保持し、単位付き（`5em`/`2lh`/`50%`）・裸数値・相対（`@` 前置）・**空（省略）**の区別を失わない。
3. The sakura コンパイラ shall x・y の**単位換算・座標解決・原点解釈を行わない**（消費側の責務）。
4. The cursor cue shall 発行時点の現在スコープへ帰属する。
5. When x・y の双方が空である, the sakura コンパイラ shall なお cursor cue を発行する（「無効果」の判定は消費側の責務であり、記述の存在を台本から失わせない）。

### Requirement 4: `\!` 汎用コマンドの cue 化（汎用キャリア＝契約の正本・move を含む）
**Objective:** sakura として、`\![name,args...]` のベースウェアコマンド名前空間**全体**を、名前と引数を解釈せず**単一種別の汎用コマンド cue** へ転写したい。そうすれば、正典で 183 コマンド（下限・52 族・SSP の版とともに非有界に増える）を数える名前空間を型付き語彙の個別実装で追いかける破綻を避け、意味の知識を持つ消費側（ghost＝move・seriko＝bind 等）が解釈を担い、未対応コマンドも語彙を失わず台本に第一級で残る。

#### Acceptance Criteria
1. When `\![name,args...]` を含む talk script をコンパイルする, the sakura コンパイラ shall コマンド名と引数列を保持した**単一種別の汎用コマンド cue** を発行する（無音で破棄しない・コマンドごとの型付き cue 語彙を**新設しない**——`move` も `bind` も `moveasync` も同一キャリアに乗る）。
2. The 汎用コマンド cue shall 引数列を**カンマ分割の生トークン列**として記述順のまま・欠落なく保持し、空引数（省略スロット）を空トークンとして、名前付き引数（`--key=value` 形）を素通しのトークンとして保持する（位置形・名前付き形の両形が正典に併存するため双方を透過・fixture の `\![move,-353,,,0,base,base]` は空トークン 2 個を保った 6 トークン列となる）。
3. The sakura コンパイラ shall コマンド名・引数の意味（座標・基準点・対象・時間）を**解釈しない**。ただし、テキスト再生時間の焼き込み・barrier パラメータ・実行時未確定の待機に構造上干渉する**明示 allowlist の時間指令系**（`quicksection`／`set,balloonwait`／`set,choicetimeout`／`set,balloontimeout`／`embed`／`sound,wait`／`wait,syncobject`／同期 `move` 系の持続時間引数）に限り、転写に**加えて** compile 自身が追加解釈してよい（allowlist 外の compile 側解釈は禁止）。M1 は allowlist の実導出を行わず語彙保持＋縮退に留める（emo2 未使用・実導出は源が着地した時点で just-in-time）。
4. The 汎用コマンド cue shall 発行時点の現在スコープへ帰属する（`\1\![move,...]` は相方側として運ばれる）。
5. The 汎用コマンド cue の消費 shall **コマンド名による消費側選別**で行われる: 1 つのコマンド名を action する消費者は**高々 1 つ**とし、名前→担当消費者の対応は**単一の権威表**が所有する（消費者ごとの私的名前リストへ分散させない）。担当外・未知のコマンド名は全消費者が記録付きで**良性スキップ**し、envelope duration の honor は従来どおり不変とする。

### Requirement 5: `\![move]` の末端反映（実際に窓が動く）
**Objective:** ゴーストとして、move cue を実際のキャラクタ窓の移動として反映したい。そうすれば、初回起動時の立ち位置調整というユーザーに見える機能が復活する。

#### Acceptance Criteria
1. When 汎用コマンド cue のうちコマンド名 `move` のもの（以下「move cue」）が配送される, the ghost shall 対象スコープのキャラクタ窓を指定された位置へ**即時に移動**させる（消費の選別は R4.5 の名前選別に従う）。
2. The ghost shall `\![move]` の引数意味論（基準点・符号・単位・省略引数の扱い）を **ukadoc 正典に従って解決**し、正典が沈黙する箇所は areka 裁量として決定したうえで対応表へ記録する。基準位置 `base` の解決は**正典既定の basepos（x=サーフェス幅÷2・y=下端）のみを実導出**する（emo2 は `point.basepos` を宣言せず正典既定がそのまま適用される正規経路・fixture は Y=fix ゆえ実効は basepos.x のみ）。宣言 `point.basepos` の実導出は本 spec の範囲外とし、差し替え可能な型シームを予約したうえで追跡 spec `areka-P0-surfaces-basepos` へ申し送る。裸 `base`（ドット無し形・正典形式は `X基準.Y基準`）は `base.base` と等価に解する（areka 裁量・対応表記録）。
3. When 移動対象のキャラクタ窓に随伴するバルーン窓が在る, the ghost shall バルーンを**相対オフセットを保ったまま随伴移動**させる。
4. Where 移動指定に時間（アニメーション）が含まれる, the ghost shall M1 では補間せず**最終位置へ即時反映**し、その縮退を記録する（語彙は保持する）。
5. If 移動対象が解決できない（対象の窓が存在しない等）, then the ghost shall 警告を記録して talk の再生を継続する（無音で失敗せず、異常終了もしない）。

### Requirement 6: `\![move]` と位置永続化の分離
**Objective:** ゴーストとして、script 由来の移動が「ユーザーが決めた定位置」を上書きしないようにしたい。そうすれば、保存値＝ユーザーの明示的な意図、表示位置＝その写像、という二層分離が壊れない。

#### Acceptance Criteria
1. When `\![move]` によりキャラクタ窓が移動する, the areka shall **表示位置のみ**を変更し、永続化の対象となる位置値を更新しない。
2. The `\![move]` 経路 shall ユーザーの明示的なドラッグ確定と**同じ「位置の確定」意味を持たない**（位置を確定するライターを二重化しない）。

### Requirement 7: システム変数 `%username` の展開（プロパティシステム読み口の消費）
**Objective:** 互換ベースウェアとして、環境変数を表示前に展開したい。そうすれば、撫で talk のバルーンに生の `%username` が露出せず、ゴースト作者が正典どおりの記述で書ける。かつ sakura は**値源を所有せず、プロパティシステム（sylphya・別 spec で新設予定の単一名前空間）の読み口スナップショットを消費するだけ**にしたい。そうすれば、名前→値の解決が単一機構へ集約され、`%username` だけの偽ストア（1 エントリのプロパティシステム）を作らずに済む。

#### Acceptance Criteria
1. When `%username` を含む talk script をコンパイルする, the sakura コンパイラ shall 当該トークンを**手渡された名前→値スナップショットの値へ展開**し、生の `%username` をバルーンへ露出させない。
2. The 展開結果 shall 通常のテキストと**同じ扱い**を受ける（記述順の保持・テキストと同一の再生時間規則の適用）。
3. The sakura コンパイラ shall システム変数の値を **talk 開始時に手渡される名前→値スナップショット（プロパティシステム読み口の凍結像）から解決**し、**自ら値源を所有しない**（永続化層・SHIORI・OS 環境のいずれも sakura は直接読まない）。スナップショットを埋めるのは ⓪ ghost の責務であり、`%username` の正典的な値源は SHIORI リソース（`GET SHIORI/3.0`・`ID: username`）である（emo2/pasta は 204 No Content を返す＝実機観測値は常に既定値）。スナップショットの生成元は最終的に `areka-P0-sylphya` の読み口（凍結像）であり、provider の差し替えで sakura 側の契約・実装は変わらない（差替シーム）。
4. If スナップショットに当該名の値が無い（未解決・SHIORI が 204 等）, then the sakura コンパイラ shall **既定値**へ展開する（生の `%username` を露出させず、結果は決定論的である）。既定値の具体は正典が規定しないため areka 裁量＋対応表記録とする。
5. Where スナップショットに載らない（M1 未解決の）システム変数名が現れる, the sakura コンパイラ shall 元の記述（`%名前`）を**テキストとしてそのまま出力**し、記録する（情報を失わない縮退・システム変数という語彙は第一級のまま保持する）。
6. The sakura コンパイラ shall システム変数の展開を**名前→値スナップショットの参照として純粋に行い**、OS のユーザー名などの外部環境を暗黙に読まない（同一入力・同一スナップショットなら常に同一出力＝決定論・no I/O）。

### Requirement 8: 既存挙動の非退行と除外仕様の意図的更新
**Objective:** areka として、4 語彙の救出が settled な既存資産を壊さないようにしたい。そうすれば、並走する他ユニット（mayuna 等）の additive 拡張と衝突せず、既存の talk 再生の正しさが保たれる。

#### Acceptance Criteria
1. The dola cue 語彙 shall **既存 cue の外部表現（シリアライズ形）を変えずに additive 拡張**され、既存台本データの読み込み互換を保つ。
2. When パススルー生データ（構文区切りできない Raw）を含む script をコンパイルする, the sakura コンパイラ shall 従来通り cue を発行せず、記録して継続する（寛容・異常終了しない）。`\!` コマンドは本除外から**卒業**し、R4 の汎用コマンド cue として全て台本に載る（未対応コマンドの縮退は compile の無音落ちでなく**消費側の良性スキップ**へ移る）。
3. The 「無視されるタグ」の集合 shall `\q`／`\_l`／**`\!` コマンド名前空間全体**／`%username` を**含まない**（既存の除外仕様＝檻を、仕様変更として明示的に更新する。compile 側に残る除外は Raw のみ）。
4. When 本 spec の対象タグを含む talk script をコンパイルする, the sakura コンパイラ shall 既存の台本規則（冒頭の全消去の前置・duration の焼き込み・絶対時刻整列）を対象タグにも**一貫して**適用する。
5. Where 新しい種別の cue が broadcast 配送される, the areka shall それに関心のない表現者側で**良性にスキップ**させ（記録あり・無音破棄でも異常終了でもない）、既存の表示を変化させない。
6. The 除外仕様の意図的更新 shall sakura compile の除外集合に留まらず、settled な cue 再生ランタイムの挙動——「choice cue を配送列から分離し表現者へ surface しない先積み一択」——にも及ぶ。choice cue は他 cue と**同一の配送列へ順序を保って broadcast** され、先積みバッグは選択解決時の id 照合専用（配置/表示情報は配送列が単一真実源・解決照合はバッグが単一真実源＝責務二分）に限定して並存する。
7. The 除外仕様の意図的更新 shall relevance 分類の権威文言にも及ぶ: `cue_target_of` は**型レベル分類の単一権威**に限定され、汎用コマンド cue の型レベル分類（担当スロットなし）は「誰も action しない」でなく「**コマンド名レベルの選別（R4.5）への委譲**」を意味するものへ意図的に改訂する（settled 側の「分類不能＝どの演者も action しない」という注釈・rustdoc の前提は本 spec が仕様変更として更新する——settled 型定義が `Custom` を「消費者固有コマンド」と定義しながら注釈が「誰も action しない」と読む内部矛盾を、型設計の意図側へ解消する）。duration honor（envelope 一律）は改訂しない。

### Requirement 9: 決定論的検証と実機サインオフ
**Objective:** 開発者として、4 語彙の写像を実時間や外部環境に依存せず検証し、最後に実機で位置調整を目視確認したい。そうすれば、下流 2 spec が消費する契約が「実物」として固定され、初回起動の位置調整が本当に効いていることを保証できる。

#### Acceptance Criteria
1. The areka shall 本 spec の全写像（script → cue／barrier 列）を、**script 直入力**から検証可能にする（実時間の待機や外部環境への依存を伴わない）。
2. When fixture のメインメニュー script（`\q` 3 個＋`\_l`＋改行）を直入力する, the 検証 shall 期待される cue 列（choice cue 3 個・cursor cue・選択待ち barrier の順序と時刻整列）と一致することを確認する。
3. When fixture の `\1\![move,-353,,,0,base,base]` を直入力する, the 検証 shall 相方側スコープの汎用コマンド cue（コマンド名 `move`）が**空トークンを含む生引数列**を保持したまま発行されることを確認する。
3b. When 未知・M1 未対応のコマンド名を持つ `\!` を含む script を直入力し配送する, the 検証 shall 汎用コマンド cue が台本に第一級で現れ、全消費者が記録付きで良性スキップして talk 再生が完了することを決定論的に確認する（名前 partition の檻: 権威表上の 1 コマンド名の担当が高々 1 であることの検査を含む）。
4. When `%username` を含む script を、値ありスナップショット／値なしスナップショットで直入力する, the 検証 shall それぞれスナップショット値／既定値へ展開されたテキストを確認する（sakura が値源を持たず、手渡された写像のみを参照することの決定論檻）。
5. The 検証 shall `\![move]` 経路が**永続化対象の位置状態を更新しない**ことを決定論的に確認する（第二の位置ライター混入の恒久的な防止）。
6. When 実 emo2・実 SHIORI・実 DPI で**初回起動（OnFirstBoot 経路）**する, the 開発者 shall エモ（相方側）の立ち位置調整が効いていることを目視でサインオフする（通常起動の talk には移動が無いため、観測は初回起動状態で行う）。
7. When fixture のメインメニュー script を直入力し cue を**配送**する, the 検証 shall choice cue 3 個が改行・カーソル cue との交互位置を保って**配送列**に現れること（compile 出力の順序だけでなく、消費側が観測する配送列で体裁再構成可能性が成立すること）を決定論的に確認する。
8. When 選択解決を**注入した入力**（実クリックでなく `SakuraMsg` の解決アームへ id を直接投入）で与える, the 検証 shall 選択待ち barrier で停止していた talk が再開し完了へ到達することを決定論的に確認する（口の存在と再開挙動の檻）。end-to-end の解決（ユーザークリック→カスケード→SHIORI 発火）は本 spec の検証範囲外であり、`areka-P0-choice-select-events`（決定論）および `areka-P0-emo2-conformance-e2e`（実機メニュー一周）へ申し送る。
