# Requirements Document

## Introduction

emo2（互換ベースウェアの適合 fixture ゴースト）の**メニュー・位置調整・撫で talk が使う 4 語彙**は、parse は成功するのに sakura の compile catch-all（「M-boot 外タグを無視」）で**全て無音落ち**している。結果、ユーザーから見える機能が丸ごと存在しない:

- **`\q[タイトル,イベント名]`**（`menu.pasta` 9 箇所）→ 選択肢が cue にならず、**ダブルクリックメニューが存在しない**。
- **`\_l[5em,2lh]`**（menu 3 箇所・選択肢の区切り位置指定）→ カーソル移動が cue にならず、**メニューの体裁が崩れる**。
- **`\![move,-353,,,0,base,base]`**（`boot.pasta` の **OnFirstBoot**・`menu.pasta` の位置調整）→ 移動が cue にならず、**初回起動時のエモ（相方側）の位置調整が黙って失われている**。
- **`%username`**（`touch.pasta` 2 箇所）→ 展開されず、**撫で talk のバルーンに生文字列 `%username` が露出する**（環境変数の展開はベースウェアの義務＝ukadoc `OnTranslate`「ベースウェアによる環境変数の展開などの後に再び SHIORI へ送られる」）。

本 spec は、この 4 語彙を **settled な cue モデル**（`completed/areka-P0-cue-playback-duration`＝envelope 一律 duration・自己完結した絶対時刻台本・単一 `CueSink`・relevance 単一権威・broadcast 配送・占有 horizon での完了）の上へ **additive に載せ**、fixture script の直入力から**決定論的に正しい cue／barrier 列**が得られ、`\![move]` は**末端まで貫通して実機の初回起動でエモが横へ動く**ところまでを実現する。

本 spec は M-dialogue（メニュー一周）の**先鋒＝契約の正本**である。**choice cue の形**（表示ラベル／ID／references の載せ方）と**「選択肢群＋選択待ち barrier」の並び規則**は本 spec が確定し、下流の `areka-P0-choice-render`（表示）と `areka-P0-choice-select-events`（選択確定カスケード）は**消費のみ**を行う。

正典は ukadoc であり、emo2 は最小適合 fixture にすぎない（正典が沈黙する箇所は areka 裁量＋対応表記録＝互換契約）。

## Boundary Context

- **In scope**:
  - `\q`（選択肢）の cue 化＝**表示ラベル／ID／references を欠落なく運ぶ choice cue の形の確定**（下流の消費契約の正本）。
  - **選択待ち barrier の並び規則の確定**（選択肢を含む talk は選択が解決されるまで完了しない）。
  - `\_l`（カーソル位置）の cue 化＝**単位・相対指定を不透明文字列のまま転写**（解釈は消費側）。
  - `\![move]`（キャラクタ移動）の cue 化＋**末端まで貫通した実際の窓移動**（即時移動のみ・随伴バルーン込み）。
  - `%username` の**展開**（値源は起動構成からの注入・未注入時は既定値）と、未対応システム変数名の**素通し縮退**。
  - 既存の「無視されるタグ」仕様（除外の檻）の**意図的更新**（対象 4 語彙の卒業）と、既存 cue 挙動の非退行。
  - fixture script 直入力による**決定論的検証**と、実 emo2 初回起動での**実機サインオフ**（エモの位置調整）。
- **Out of scope**:
  - 選択肢の**表示・UI・ヒットテスト・ハイライト**、および `\_l` の単位換算（em/lh/%）＝`areka-P0-choice-render` の領分。
  - **選択確定→SHIORI カスケード**（任意名イベント直接発火／`OnChoiceSelect(Ex)` の判別規則）・選択のタイムアウト時間の決定・`OnChoiceTimeout`＝`areka-P0-choice-select-events` の領分。
  - `\![bind]`（`areka-P0-mayuna-compose`）・`\![raise]` 等その他の汎用コマンド（M1 外・従来通り無視のまま）。
  - **時間指定付きの移動アニメーション**（emo2 未使用＝M1 は即時移動へ縮退し語彙のみ保持）。`\![moveasync]` も同様に M1 外。
  - **選択肢タイムアウト属性**（`\*` ／ `\![set,choicetimeout,時間]`＝スクリプト単位属性・fixture 未使用）の実導出。
  - **位置の永続化そのもの**（`ghost.dat` 保存/復元＝`areka-P0-position-persist`）。本 spec は「`\![move]` が永続値を書かない」ことのみを担保する。
  - `%username` 以外のシステム変数の**実導出**（`%selfname`/`%keroname`/`%property[...]`/`%m*` 等＝源が着地した時点で just-in-time）。
- **Adjacent expectations**:
  - **cue 再生の settled モデルは既に main に在る**（duration・絶対時刻台本・broadcast・選択待ち状態と選択解決の口）。本 spec は**そこへ別アームを足す**形であり、時間モデル・配送モデル・完了判定の**規則そのものを再定義しない**。
  - **選択の解決（ユーザーのクリック→どの選択肢が選ばれたか）を起こすのは下流**（choice-render／choice-select-events）。本 spec は「選択待ちで止まり、解決されたら再開する」台本側の契約のみを確定する。
  - **`\_l` 直後の行揃えリセット**（ukadoc: `\_l` 実行直後は左揃えへ戻る）や `@` 相対指定の解決は**表示側の責務**であり、本 spec は記述を欠落なく運ぶことに徹する。
  - **`\![move]` の位置は永続化されない**（ポートフォリオ合流裁定＝保存値はユーザーの明示的ドラッグ確定のみが更新する二層分離）。その帰結として、`areka-P0-position-persist` の初回ゲート導入後は**未ドラッグの 2 回目以降の起動で初回位置調整が既定配置へ戻る**——これは許容仕様であり、最終確認は `areka-P0-emo2-conformance-e2e` の実機適合走行へ申し送る。
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

### Requirement 2: 選択待ち barrier の並び規則（停止と再開の契約＝下流の正本）
**Objective:** sakura として、選択肢を含む talk が**ユーザーの選択を待って停止し、選択で再開する**台本になるようにしたい。そうすれば、メニューが「表示された直後に勝手に終わる」ことなく、階層メニューの往復が成立する。

#### Acceptance Criteria
1. When talk script が 1 つ以上の `\q` を含む, the sakura コンパイラ shall 当該 talk 台本へ**選択待ち barrier をちょうど 1 つ**発行する。
2. The 選択待ち barrier shall 台本内の**全 choice cue より後**に位置する。
3. While 選択待ち barrier に到達して未解決である, the cue 再生ランタイム shall 後続 cue を発火させず、当該 talk を**完了として扱わない**（選択待ちのまま talk 完了を通知しない）。
4. When 選択が解決される, the cue 再生ランタイム shall 停止していた台本の再生を再開する。
5. Where talk script に `\q` が 1 つも含まれない, the sakura コンパイラ shall 選択待ち barrier を発行せず、既存 talk の完了挙動を変えない。
6. The sakura コンパイラ shall 選択待ちに**タイムアウト時間を指定しない**（M1 は無期限待ち）。タイムアウト時間の決定と時間切れ時の振る舞いは本 spec の範囲外とし、語彙（タイムアウト指定の口）のみ保持する。

### Requirement 3: カーソル `\_l` の cue 化（不透明転写）
**Objective:** sakura として、`\_l` のカーソル位置指定を記述通りに cue へ転写したい。そうすれば、単位（em/lh/%/裸数値）や相対指定の解釈を持つ表示側が、後から正典どおりに解決できる。

#### Acceptance Criteria
1. When `\_l[x,y]` を含む talk script をコンパイルする, the sakura コンパイラ shall 対応する cursor cue を発行する（無音で破棄しない）。
2. The cursor cue shall x・y を**記述通りの不透明な文字列**として保持し、単位付き（`5em`/`2lh`/`50%`）・裸数値・相対（`@` 前置）・**空（省略）**の区別を失わない。
3. The sakura コンパイラ shall x・y の**単位換算・座標解決・原点解釈を行わない**（消費側の責務）。
4. The cursor cue shall 発行時点の現在スコープへ帰属する。
5. When x・y の双方が空である, the sakura コンパイラ shall なお cursor cue を発行する（「無効果」の判定は消費側の責務であり、記述の存在を台本から失わせない）。

### Requirement 4: キャラクタ移動 `\![move]` の cue 化
**Objective:** sakura として、`\![move]` を引数の意味を解釈せずに move cue へ転写したい。そうすれば、座標系と基準点の知識を持つ窓配置側が単一の権威として意味を与えられる。

#### Acceptance Criteria
1. When `\![move,...]` を含む talk script をコンパイルする, the sakura コンパイラ shall 対応する move cue を発行する（無音で破棄しない）。
2. The move cue shall 引数列を**記述順のまま・欠落なく**保持し、空引数（省略）を空の要素として保持する。
3. The sakura コンパイラ shall 引数の意味（座標・基準点・時間・名前付き引数形）を**解釈しない**。
4. The move cue shall 発行時点の現在スコープへ帰属する（`\1\![move,...]` は相方側の移動として運ばれる）。
5. Where `\!` コマンドが `move` 以外である, the sakura コンパイラ shall 従来通り cue を発行せず記録して継続する（本 spec の対象外・`\![moveasync]` を含む）。

### Requirement 5: `\![move]` の末端反映（実際に窓が動く）
**Objective:** ゴーストとして、move cue を実際のキャラクタ窓の移動として反映したい。そうすれば、初回起動時の立ち位置調整というユーザーに見える機能が復活する。

#### Acceptance Criteria
1. When move cue が配送される, the ghost shall 対象スコープのキャラクタ窓を指定された位置へ**即時に移動**させる。
2. The ghost shall `\![move]` の引数意味論（基準点・符号・単位・省略引数の扱い）を **ukadoc 正典に従って解決**し、正典が沈黙する箇所は areka 裁量として決定したうえで対応表へ記録する。
3. When 移動対象のキャラクタ窓に随伴するバルーン窓が在る, the ghost shall バルーンを**相対オフセットを保ったまま随伴移動**させる。
4. Where 移動指定に時間（アニメーション）が含まれる, the ghost shall M1 では補間せず**最終位置へ即時反映**し、その縮退を記録する（語彙は保持する）。
5. If 移動対象が解決できない（対象の窓が存在しない等）, then the ghost shall 警告を記録して talk の再生を継続する（無音で失敗せず、異常終了もしない）。

### Requirement 6: `\![move]` と位置永続化の分離
**Objective:** ゴーストとして、script 由来の移動が「ユーザーが決めた定位置」を上書きしないようにしたい。そうすれば、保存値＝ユーザーの明示的な意図、表示位置＝その写像、という二層分離が壊れない。

#### Acceptance Criteria
1. When `\![move]` によりキャラクタ窓が移動する, the areka shall **表示位置のみ**を変更し、永続化の対象となる位置値を更新しない。
2. The `\![move]` 経路 shall ユーザーの明示的なドラッグ確定と**同じ「位置の確定」意味を持たない**（位置を確定するライターを二重化しない）。

### Requirement 7: システム変数 `%username` の展開（ベースウェア義務）
**Objective:** 互換ベースウェアとして、環境変数を表示前に展開したい。そうすれば、撫で talk のバルーンに生の `%username` が露出せず、ゴースト作者が正典どおりの記述で書ける。

#### Acceptance Criteria
1. When `%username` を含む talk script をコンパイルする, the sakura コンパイラ shall 当該トークンを**注入された値へ展開**し、生の `%username` をバルーンへ露出させない。
2. The 展開結果 shall 通常のテキストと**同じ扱い**を受ける（記述順の保持・テキストと同一の再生時間規則の適用）。
3. The areka shall `%username` の値を**起動構成として外部から注入可能**にする（値をハードコードしない）。
4. If `%username` の値が注入されていない, then the areka shall **既定値**へ展開する（生の `%username` を露出させず、結果は決定論的である）。
5. Where 実導出を持たない（M1 未対応の）システム変数名が現れる, the sakura コンパイラ shall 元の記述（`%名前`）を**テキストとしてそのまま出力**し、記録する（情報を失わない縮退・システム変数という語彙は第一級のまま保持する）。
6. The sakura コンパイラ shall システム変数の展開を**名前→値の写像**として行い、OS のユーザー名などの外部環境を暗黙に読まない（同一入力・同一構成なら常に同一出力）。

### Requirement 8: 既存挙動の非退行と除外仕様の意図的更新
**Objective:** areka として、4 語彙の救出が settled な既存資産を壊さないようにしたい。そうすれば、並走する他ユニット（mayuna 等）の additive 拡張と衝突せず、既存の talk 再生の正しさが保たれる。

#### Acceptance Criteria
1. The dola cue 語彙 shall **既存 cue の外部表現（シリアライズ形）を変えずに additive 拡張**され、既存台本データの読み込み互換を保つ。
2. When 未対応タグ（`\![bind]` 等の汎用コマンド・パススルー生データ）を含む script をコンパイルする, the sakura コンパイラ shall 従来通り cue を発行せず、記録して継続する（寛容・異常終了しない）。
3. The 「無視されるタグ」の集合 shall `\q`／`\_l`／`\![move]`／`%username` を**含まない**（既存の除外仕様＝檻を、仕様変更として明示的に更新する）。
4. When 本 spec の対象タグを含む talk script をコンパイルする, the sakura コンパイラ shall 既存の台本規則（冒頭の全消去の前置・duration の焼き込み・絶対時刻整列）を対象タグにも**一貫して**適用する。
5. Where 新しい種別の cue が broadcast 配送される, the areka shall それに関心のない表現者側で**良性にスキップ**させ（記録あり・無音破棄でも異常終了でもない）、既存の表示を変化させない。

### Requirement 9: 決定論的検証と実機サインオフ
**Objective:** 開発者として、4 語彙の写像を実時間や外部環境に依存せず検証し、最後に実機で位置調整を目視確認したい。そうすれば、下流 2 spec が消費する契約が「実物」として固定され、初回起動の位置調整が本当に効いていることを保証できる。

#### Acceptance Criteria
1. The areka shall 本 spec の全写像（script → cue／barrier 列）を、**script 直入力**から検証可能にする（実時間の待機や外部環境への依存を伴わない）。
2. When fixture のメインメニュー script（`\q` 3 個＋`\_l`＋改行）を直入力する, the 検証 shall 期待される cue 列（choice cue 3 個・cursor cue・選択待ち barrier の順序と時刻整列）と一致することを確認する。
3. When fixture の `\1\![move,-353,,,0,base,base]` を直入力する, the 検証 shall 相方側スコープの move cue が引数を保持したまま発行されることを確認する。
4. When `%username` を含む script を値の注入あり／なしで直入力する, the 検証 shall それぞれ注入値／既定値へ展開されたテキストを確認する。
5. The 検証 shall `\![move]` 経路が**永続化対象の位置状態を更新しない**ことを決定論的に確認する（第二の位置ライター混入の恒久的な防止）。
6. When 実 emo2・実 SHIORI・実 DPI で**初回起動（OnFirstBoot 経路）**する, the 開発者 shall エモ（相方側）の立ち位置調整が効いていることを目視でサインオフする（通常起動の talk には移動が無いため、観測は初回起動状態で行う）。
