# Requirements Document

## Project Description (Input)
α の一周は「`.nar` を窓へ落とす」から始まるが、areka のゴースト窓（WUC 合成＝`WS_EX_NOREDIRECTIONBITMAP`・クリック透過は `WS_EX_TRANSPARENT` の付け外し）にエクスプローラからの落とし物のメッセージ `WM_DROPFILES` が実際に届くかは、誰も測っていない（2026-09-26 時点で `WM_DROPFILES`／`DragAcceptFiles`／`DragQueryFile`／`IDropTarget` は `crates/` に 0 件）。本坑 `areka-P0-ghost-install` は「投げ込みは `WM_DROPFILES` で受ける」を前提に設計する予定だが、届かなければ設計は `IDropTarget`（OLE・STA のスレッド）へ倒れ、WUC が MTA で動く前提との衝突を解くことになる。本坑の要件を書く前に、使い捨ての先進坑で答えを出す。成果物はコードではなく知見（go／違う／直す＋学び）で、一次記録は `crates/pilot/examples/pilot-dropfiles-on-wuc-window/README.md`（3 幕）。go 判定は開発者。

## Introduction

本仕様は先進坑（pilot・使い捨て）である（`.kiro/steering/two-tunnel.md` の規律に従う）。確かめたい点は 1 つ——**本番と同じ拡張スタイルで建てた WUC 合成・クリック透過つきのゴースト窓へ、エクスプローラから `.nar` を落としたとき、落とし物のメッセージが届いてファイルのパスが取れるか。絵の外（透過している所）へ落としたときは、窓に届かず背後の窓へ抜けるか**——である。

観測は開発者の手（エクスプローラからのドラッグ＆ドロップ）と、example が出すログの読み取りで行う。example は上限時間の内側で自分で終わる。

本仕様の成果物はコードではなく知見（go／違う／直す＋学び）であり、一次記録は `crates/pilot/examples/pilot-dropfiles-on-wuc-window/README.md`（3 幕）である。go／違う／直す の判定は開発者（人間）が README を見て下す。

合否基準（brief より）:
- **go**: 絵の上で落とすと落とし物のメッセージが届き、ファイルのパスが取れる。絵の外では届かない（背後の窓へ抜ける）。
- **直す**: 届くが条件つき（例: 透過の付け外しの時機・受け入れの宣言の付け方・別の権限で動くプロセスからのメッセージを通す設定の要否）で、本坑の中で直せる。直し方を README に書く。
- **違う**: どう組んでも届かない。`IDropTarget` へ倒す。その場合の STA の置き場所の見立ても学びに書く。

## Boundary Context

- **In scope**:
  - `crates/pilot/examples/pilot-dropfiles-on-wuc-window/` の example 1 本（`crates/pilot/examples/_template/` を写して着手）。
  - wintf で窓を 1 枚建て（本番のゴースト窓と同じ拡張スタイルに受け入れの宣言 `WS_EX_ACCEPTFILES` を足したもの・透明な所と不透明な所を持つ絵 1 枚・クリック透過の機構あり）、落とし物のメッセージを受けてパスをログへ出す実験。
  - 受け入れの宣言と、受け取り（メッセージの到着・パスの取り出し・後片付け）の観測。
  - 透過の付け外しとの噛み合いの観測（絵の上／絵の外・付け外しの前後で受け取りが変わるか）。
  - 有界な自動終了。
  - README 3 幕（動機・概要・検証結果）。
  - 実験に要る依存の `crates/pilot/Cargo.toml` への追加（wintf は登録済み。追加は pilot 側が依存するだけ）。
- **Out of scope**:
  - `.nar` の展開・インストール（`areka-nar` と本坑 `areka-P0-ghost-install` の領分）。
  - `OnFileDrop2`／`OnDirectoryDrop` の送出、ファイル選択の箱（本坑 `areka-P0-ghost-install`）。
  - テキスト・URL の投げ込み（α 後）。
  - `IDropTarget` の試作（「違う」となったとき、その見立てを学びに書くだけ）。
  - wintf 本体・areka 本体のコードの変更。
  - 先進坑コードの production への流用（本坑は README の知見を見てクリーンに掘り直す）。
- **Adjacent expectations**:
  - 下流の本坑 `areka-P0-ghost-install` は、本先進坑の go 判定を `_Depends(confirmed): pilot-dropfiles-on-wuc-window` の前提依存として持つ（`.kiro/steering/roadmap.md` に記載済み）。本坑の design は README の検証結果を参照し、同じ結果を二重に書かない。
  - 同じウェーブで並走する `areka-P0-ghost-shell-balloon-switch` と触るファイルの重なりは 0（本先進坑は `crates/pilot/` の下だけを触る）。
  - 既存の wintf（窓の生成・窓手続きの振り分け・クリック透過の機構）はそのまま使う側に留まり、本先進坑はそれを変更しない。手本は先進坑 `pilot-balloon-asset-swap`（wintf の窓の建て方・クリック透過への窓の登録・上限時間つきの終了）。

## Requirements

### Requirement 1: 先進坑の規律と隔離

**Objective:** As a 開発者, I want 先進坑のコードが出荷されるどの crate からも依存されない場所に閉じていること, so that 判定が go／違う／直す のどれであっても、コードを安全に捨てるか知見として残しておける

#### Acceptance Criteria
1. The pilot example shall コードを `crates/pilot/examples/pilot-dropfiles-on-wuc-window/` の下にだけ置き、フォルダ名を spec 名 `pilot-dropfiles-on-wuc-window` と一致させる。
2. The pilot example shall `crates/pilot/examples/_template/` を写して着手する。
3. The pilot shall コードと依存の変更を `crates/pilot/` の下（`crates/pilot/Cargo.toml` を含む）に限り、他の crate のファイルを変更しない。
4. The pilot shall 他の crate の `Cargo.toml` に `pilot` への依存を一切足さない。
5. While 先進坑のコードを書いている間, the pilot shall 整形・命名・テストの厳しさを緩めてよいが、前項 3・4 の隔離は必ず守る。
6. The pilot shall 一次成果物をコードではなく知見（go／違う／直す＋学び）として扱う。

### Requirement 2: 検証台となる窓

**Objective:** As a 開発者, I want 本番のゴースト窓と同じ条件（WUC 合成・クリック透過の付け外し）の窓に受け入れの宣言だけを足した検証台, so that ここで出た答えがそのまま本番の窓にも当てはまると信じられる

#### Acceptance Criteria
1. When example を起動したとき, the pilot example shall wintf で窓を 1 枚建て、実際の画面に表示する。
2. The pilot example shall 窓の様式を本番のゴースト窓と同じ（枠なしのポップアップ・拡張スタイルは本番の `WS_EX_LAYERED | WS_EX_TOOLWINDOW` に受け入れの宣言 `WS_EX_ACCEPTFILES` を足したもの）にし、wintf が付け外しする分（`WS_EX_LAYERED` を外し `WS_EX_NOREDIRECTIONBITMAP` を足す）はそのまま wintf に任せる。
3. The pilot example shall 既定の走行では、受け入れの宣言を拡張スタイルに `WS_EX_ACCEPTFILES` を足すだけで行い、`DragAcceptFiles` を呼ばない（第 5 要件 4 の手当てとして試す走行は除く）。
4. The pilot example shall 透明な所と不透明な所の両方を持つ絵 1 枚を窓に表示し、どちらの所もエクスプローラからファイルを落とせる大きさ（少なくとも 100×100 画素）にする。
5. The pilot example shall 窓を wintf のクリック透過の機構に登録し、絵の透明な所ではクリックが背後の窓へ抜け、不透明な所ではクリックが窓に届く状態にする。
6. When 窓を建てた直後, the pilot example shall 窓の拡張スタイルの実際の値を読み戻し、受け入れの宣言・透過・`WS_EX_NOREDIRECTIONBITMAP` の各ビットの有無をログへ出す。
7. When クリック透過の機構が透過を付けたり外したりしたとき, the pilot example shall その切り替えが開発者に見えるよう、切り替えの記録が出るログの水準で走る。
8. The pilot example shall 開発者の画面の拡大率（開発機は 200%）を変えることを求めず、そのままの拡大率で検証できる。

### Requirement 3: 落とし物の受け取りとログ

**Objective:** As a 開発者, I want 落とし物のメッセージが窓に届いたことと、そこから取れたパスがログで分かること, so that 「届いた」「取れた」を目視でなくログの行で判定できる

#### Acceptance Criteria
1. The pilot example shall wintf 本体のコードを変えずに、落とし物のメッセージ（`WM_DROPFILES`）を example の側で受け取る。
2. When 落とし物のメッセージが窓に届いたとき, the pilot example shall 到着した時刻・落とした位置（窓の中の座標）・その位置が絵の不透明な所か透明な所か・その時点で透過が付いていたか外れていたかをログへ出す。
3. When 落とし物のメッセージが窓に届いたとき, the pilot example shall 落とされたファイルの数と、各ファイルのパスをログへ出す。
4. When 落とされたファイルのパスを取り出し終えたとき, the pilot example shall 落とし物の後片付け（`DragFinish` に当たる解放）を行う。
5. If パスの取り出しに失敗したとき, the pilot example shall 失敗した理由をログへ出し、止まったまま待たない。
6. The pilot example shall 落とし物に関するログの行を、他のログから機械で選び出せる決まった目印（決まった語）を付けて出す。

### Requirement 4: 絵の外への投げ込みの観測

**Objective:** As a 開発者, I want 絵の透明な所へ落としたときに窓が受け取らず背後の窓へ抜けることを確かめる手立て, so that 投げ込みの当たり判定がクリックの当たり判定と一致していることを判定できる

#### Acceptance Criteria
1. The pilot example shall 受け取った落とし物のメッセージを、落とした位置が絵の透明な所か不透明な所かにかかわらず例外なく第 3 要件どおりログへ出す（絵の透明な所へ落としたときに行が出ないことが「窓に届かなかった」の証拠になるように）。
2. The pilot shall 検証の手順に、背後に落とし物を受け取れる窓を置いて絵の透明な所へ落とし、背後の窓に落とし物が渡ったことを開発者が目で確かめる項を含める。背後の受け手は、落としても元のファイルを動かしたり壊したりしないもの（例: 捨ててよい作業用フォルダへ写した `.nar` を落とす・同じドライブのフォルダの窓へ落とすと既定で移動になることに注意する）とする。
3. The pilot shall 検証の手順に、透過が付いている状態と外れている状態のそれぞれで絵の不透明な所へ落とす項を含め、透過の付け外しの前後で受け取りが変わるかを観測する。
4. When 透過の付け外しが 1 回以上起きた後に絵の不透明な所へ落としたとき, the pilot example shall 受け入れの宣言が付け外しの後も窓に残っているか（第 2 要件 6 と同じ読み戻し）をログへ出す。

### Requirement 5: 検証の手順と合否の見立て

**Objective:** As a 開発者, I want 何をどの順に落とせばよいかが決まっていて、結果を go／違う／直す のどれに当てはめるかの規則が先に書かれていること, so that 実験のあとで基準を動かさずに判定できる

#### Acceptance Criteria
1. The pilot shall 検証の手順を、開発者がエクスプローラから `.nar` を落とす次の項で組む——ⓐ 絵の不透明な所へ落とす・ⓑ 絵の透明な所へ落とす（背後に受け取れる窓を置く）・ⓒ 透過を何度か付け外しさせた（カーソルを絵の内外へ出し入れした）後に絵の不透明な所へ落とす。
2. The pilot shall 検証の手順に、example を管理者として起動していたか（落とす側のエクスプローラと権限が違うか）を記録する項を含める。
3. When ⓐとⓒで落とし物のメッセージが届いてパスが取れ、ⓑで届かず背後の窓に渡ったとき, the pilot shall 結果を **go** の見立てとする。
4. If ⓐ〜ⓒのいずれかで届かない、または条件によって届き方が変わったとき, the pilot shall 「違う」と結論する前に、条件つきで届く可能性のある次の手当てを順に試して各結果をログと README に記録する——受け入れの宣言の付け方（窓を建てた後に付け直す・`DragAcceptFiles` で宣言する）・透過の付け外しの時機との関係・権限の違うプロセスからの落とし物のメッセージを通す設定（`ChangeWindowMessageFilterEx` に当たるもの）。
5. When 前項の手当てのいずれかで届くようになったとき, the pilot shall 結果を **直す** の見立てとし、効いた手当てを README に書く。
6. When 前項の手当てをすべて試しても届かないとき, the pilot shall 結果を **違う** の見立てとし、`IDropTarget` へ倒す場合の STA の置き場所の見立てを学びに書く。
7. The pilot shall 見立てを README に書くが、go／違う／直す の判定は開発者（人間）に委ね、Claude Code 単独で go を宣言して本坑へ進まない。

### Requirement 6: 有界な自動終了

**Objective:** As a 開発者, I want example が人手を待たずに決まった時間の内側で自分で終わること, so that 落とす操作を忘れても窓が残らず、繰り返し試せる

#### Acceptance Criteria
1. The pilot example shall 起動から決まった上限時間の内側で、人の操作なしに自分で終了する。
2. Where 環境変数で上限時間が与えられている場合, the pilot example shall その値を上限として使う（`AREKA_APP_SMOKE_EXIT_MS` に相当する仕組み）。
3. Where 環境変数で上限時間が与えられていない場合, the pilot example shall 開発者が手でファイルを落とす余裕のある既定の上限時間を使い、上限なしで走り続けない。
4. When 終了するとき, the pilot example shall 終了の理由（上限時間の到達・失敗）と、走行中に受け取った落とし物の回数をログへ出す。

### Requirement 7: README 3 幕と判定

**Objective:** As a 開発者, I want 実験の動機・作った物・結果が README 1 つにまとまっていること, so that README を見て go／違う／直す を判定し、本坑 `areka-P0-ghost-install` の設計がそこを参照できる

#### Acceptance Criteria
1. The pilot shall 一次記録として `crates/pilot/examples/pilot-dropfiles-on-wuc-window/README.md` を、動機・概要・検証結果の 3 幕で作る。
2. The README shall 動機の幕で、対応する本坑 `areka-P0-ghost-install` を名指しし、届かなければ本坑の設計が `IDropTarget` へ変わることを書く。
3. The README shall 概要の幕で、作った物と実行法（`cargo run -p pilot --example pilot-dropfiles-on-wuc-window`・上限時間の与え方・落とす操作の手順）を書く。
4. The README shall 検証結果の幕で、第 5 要件の各項ⓐ〜ⓒの結果（届いたか・取れたパス・その時点の透過の状態・受け入れの宣言が残っていたか）・管理者として起動していたか・試した手当てとその結果・学び・日付を書く。
5. The README shall 学びに、落とし物のメッセージが wintf の窓手続きまで届くか（届くなら本坑では窓手続きの振り分けの表に 1 分岐足すだけで足りるか）を書く。
6. The README shall 検証結果の幕に、結果が合否基準（go／直す／違う）のどれに当たるかの見立てを添えるが、判定の欄は開発者が下すまで確定させない。
