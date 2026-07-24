# Requirements Document

## Introduction

areka（ukadoc 準拠の互換ベースウェア）には現在、「名前で引ける値」の解決機構が存在せず、確定コースのままでは同一関心が 3 箱へ分裂する: (a) %系環境変数の値源（emo2 の撫で talk で `%username` は W1 の暫定 provider による既定値展開のみ・実 SHIORI 照会は未結線）、(b) `areka-P0-position-persist` が要件化した専用のゴースト別永続ストア（未承認）、(c) 既存の IShioriHost プロパティストア（dotted key・同期即答・充填源未結線）。開発者裁定により、areka は統一プロパティシステム **sylphya** を 1 つだけ新設する。「プロパティ」とは名前で引けるものすべてであり、%フラット名前空間と点付き名前空間は単一の系の 2 つの窓である（ukadoc は両方法の併存を記すが単一名前空間の正準宣言は無く、統一は正典と無矛盾な areka のアーキテクチャ判断として本 spec が確立する）。

本ユニットは、(1) 単一名前空間・key モデル・読み口 API（talk 決定論用の凍結スナップショット＋逐次解決）・値源（backing）差替シーム・永続 backing を 1 機構として提供し、(2) フラット語彙 26 トークン・点付き 10 ルート枝・SHIORI Resource 全 159 項目の全語彙を**完全形で第一級保持**したうえで、M1 は源のあるものだけ実導出し残りは差替シーム付きで決定論的に縮退させ、(3) `%username` の実 SHIORI 照会経路と `%selfname`/`%selfname2`/`%keroname` の descript 導出を実装し、(4) position-persist の 4 永続フィールドを収容する層別永続 backing（areka アプリ/SHIORI/シェル/バルーンの永続スコープ・対応層 profile フォルダ配置・TOML 直列化・原子的書込・寛容読取・バージョン付き形式）を提供し、(5) 既存の暫定 provider と IShioriHost プロパティストアを統合して「同じ関心の 2 箱目・3 箱目」を消す。プロパティ解決に起因するいかなる失敗でもゴーストの起動・実行を停止させない（失敗はログ＋定義済み縮退）。

## Boundary Context

- **In scope**: 単一名前空間と key モデル（フラット 26 トークン・点付き 10 ルート枝＋汎用プロパティ名 17 種＋セレクタ 5 形・SHIORI Resource 159 項目の第一級保持）／読み口 2 形（talk 開始時の凍結スナップショット＝dialogue-tags R7 契約の供給側・逐次解決）／値源（backing）差替シームと決定論的縮退（素通し・既定値・NOT_FOUND）／M1 実導出: `%username`（SHIORI Resource GET 照会・204→既定値縮退）・`%selfname`（descript sakura.name）・`%selfname2`（descript sakura.name2・読取拡張含む）・`%keroname`（descript kero.name）・点付き `baseware.name`/`baseware.version`／永続 backing（窓位置 scope 別・バルーン相対オフセット scope 別・起動記録・vanish 回数の 4 key 族、層別永続スコープ〔areka アプリ/SHIORI/シェル/バルーン〕と対応層 profile フォルダ配置、TOML 直列化、原子的書込・寛容読取・バージョン付き形式・往復耐久）／ghost の暫定 provider（W1 差替点）の sylphya 読み口への差し替え（消費側契約無改変）／IShioriHost プロパティ応答（GetProperty/SetProperty）の sylphya への統合（第 2 ストアの解消）／SET 有効群の書込型シーム予約（実書込なし）／全判断分岐の決定論単体テスト檻。
- **Out of scope**: 縮退指定語彙の実導出（時刻系 5・画面系 2・exh/et/wronghour・単語ランダム系 10・インストール文脈系 2・system.\*・ghostlist/activeghostlist/currentghost/balloonlist/headlinelist/pluginlist/history/rateofuselist 配下——各々差替シームと追跡宿題を残す）／`\![vanish]` 実装（カウント増分の発生源＝M2）・ゴースト切替・多重ゴースト／SSTP EXECUTE GetProperty・SetProperty（SSTP サーバ自体が M1 外）／SHIORI・PLUGIN イベント property.get/property.set の発火実装（ext 亜枝の実働＝M2）／単語ランダム系の候補辞書（出所自体が未確認）／さくらスクリプト展開器そのものの改変（展開器は dialogue-tags 完了済の領分・本ユニットは値源と読み口のみ）／SET 有効群への実書込（M2）／SSP `ghost.dat` バイナリ互換（不要・areka 自由形式）／ネットワーク系プロパティの実照会・ヘッドライン/更新系機能／選択肢 UI・メニュー描画。
- **Adjacent expectations**: 完了済 `areka-P0-sakura-dialogue-tags` の R7 契約（名前→値の凍結スナップショット・既定値の唯一定義点・素通し縮退・per-talk 凍結）が消費側の正本であり、本ユニットは消費側契約を**無改変**のまま供給側を実体化する（差替点は W1 が ghost 側に用意済みの供給シーム）。`areka-P0-position-persist`（未承認・後続ウェーブ）は本ユニットの永続 backing の**消費者**へ再切削され、復元意味論（アンカー再射影・OnFirstBoot ゲート運行・Reference0 注入・観測点消費）は同 spec に残存する（デルタの正本＝本 spec brief.md 申し送り節）。既存 IShioriHost プロパティストアの観測挙動（dotted key・GetProperty 同期即答・欠落 key はプロパティ不在エラー）は統合後も維持し、統合の方式・段階は design で確定する（Boundary Candidate 継承）。SHIORI 照会は既存の GET 経路（任意リソース ID の照会が機構的に可能）を用い、動的 key 照会の内部制約は design 論点。次の Boundary Candidates は design 討議へ引き継ぐ: `%property[...]` の字句拡張（現行走査規則ではトークン化不能）／`%screenwidth`・`%screenheight` の実導出（物理/論理 px 契約の確定が前提）／`currentghost` の name 系点付き実導出／暦時計注入シームの新設を本 spec で行うか追跡 spec へ譲るか。永続直列化形式は要件討議（2026-07-23）で開発者裁定により **TOML** に確定済み（採用クレートと導入形は design で確定）。正典は ukadoc とし、正典が沈黙する箇所（既定値・未定義時挙動・SET 失敗挙動等）は areka 裁量＋対応表記録とする。

## Requirements

### Requirement 1: 単一名前空間と語彙の完全形第一級保持

**Objective:** As a ゴースト開発者（さくらスクリプト・プロパティシステム利用者）, I want %フラット環境変数・点付きプロパティ木・SHIORI Resource 系の全語彙が単一の名前空間で第一級に扱われること, so that どの窓から引いても同じ系で解決され、機能の先送りが語彙の喪失にならない

#### Acceptance Criteria

1. The sylphya プロパティシステム shall フラット語彙 26 トークン（`%month` `%day` `%hour` `%minute` `%second`・`%username`・`%selfname`・`%selfname2`・`%keroname`・`%screenwidth` `%screenheight`・`%exh`・`%et` `%wronghour`・`%ms` `%mz` `%ml` `%mc` `%mh` `%mt` `%me` `%mp` `%m?` `%dms`・`%lastghostname` `%lastobjectname`）の全数を key モデル上の第一級エントリとして保持する（縮退対象の語彙も落とさない）。
2. The sylphya プロパティシステム shall 点付き語彙のルート枝 10 本（`system` `baseware` `ghostlist` `activeghostlist` `currentghost` `balloonlist` `headlinelist` `pluginlist` `history` `rateofuselist`）と汎用プロパティ名 17 種を key モデル上の第一級の枝・名前として保持する。
3. The sylphya プロパティシステム shall セレクタ 5 形（括弧名選択・`.index(ID)`・`.current`・`.count`・数値括弧〔`scope(ID)` 等〕）を key モデルの文法として完全に解釈可能とする（解決の成否は枝ごとの値源に従う）。
4. The sylphya プロパティシステム shall SHIORI Resource 語彙（全 159 項目: SHIORI 情報 5・ゴースト情報 43・更新情報 1・オーナードローメニュー画像/文字色群・`*button.caption` 91 種＋同数の `*button.visible` ファミリ・tooltip 2）を SHIORI 照会系の名前族として key モデル上に第一級で保持する。
5. The sylphya プロパティシステム shall フラット名前空間と点付き名前空間を単一名前空間の 2 つの窓として提供し、消費者ごとに別系統の解決機構を持たない（%系の解決と `%property[プロパティ名]` が指す点付き木の解決は同一の系で行われる）。
6. The sylphya プロパティシステム shall `%*` を表示タグの別記法（構文）として語彙記録のみ行い、プロパティ解決の対象としない。`\%`（エスケープ記法）は語彙に含めない。

### Requirement 2: 読み口（凍結スナップショットと逐次解決）

**Objective:** As a 消費エンジン（さくらスクリプト展開・ゴースト運行・IShioriHost 応答）, I want 用途に応じた 2 形の読み口, so that talk 再生の決定論と逐次照会の双方が満たされる

#### Acceptance Criteria

1. When talk が開始されるとき, the sylphya プロパティシステム shall その時点の解決値を凍結した名前→値スナップショットを生成可能とし、同一 talk の展開中はスナップショットの値が変化しない（per-talk 凍結）。
2. The sylphya プロパティシステム shall 既存のスナップショット消費契約（dialogue-tags R7: 名前→値写像・値あり→その値・`username` 欠落→既定値・未対応名→素通し）を消費側無改変のまま満たすスナップショットを供給可能とする。
3. The sylphya プロパティシステム shall 逐次解決の読み口（名前 1 件を与えて値または不在を同期的に得る）を提供する。
4. The sylphya プロパティシステム shall 読み口の結果に値の由来（どの値源・backing から来たか）を含めない（消費者は由来へ依存できない）。
5. While 値源の状態が同一であるとき, the sylphya プロパティシステム shall 同一の問い合わせ元コンテキストからの同一名の解決に対して常に同一の結果を返す（決定論）。
6. The sylphya プロパティシステム shall 解決要求に問い合わせ元コンテキスト（どの SHIORI／ゴーストからの照会か）を第一級で受け取り、問い合わせ元に相対的な語彙（`currentghost` 系等）を問い合わせ元ごとに解決可能な形とする（`system.*` 等の大域語彙は問い合わせ元へ依存しない。M1 は単一ゴースト運行のため実挙動は単一だが、読み口 API の形は問い合わせ元コンテキストを含む）。
7. The sylphya プロパティシステム shall 同期読み経路（スナップショット生成・逐次解決・IShioriHost GetProperty 応答）において他エンジン・他スレッドへのブロッキング照会を行わず、プロパティ解決がシステム全体の直列化点（大域ロック）にならないこと。値の供給は非同期（値源からの push 充填）とし、遅延しうる値源（SHIORI 照会・将来のネットワーク系）を同期読み経路に載せない。

### Requirement 3: 決定論的縮退と値源差替シーム

**Objective:** As a 正典忠実性の維持者, I want 未実導出の語彙が決定論的に縮退しつつ差替シームを備えること, so that 正典機能の先送りが「完全語彙＋縮退シーム」の規律で管理される

#### Acceptance Criteria

1. When 解決要求されたフラット名が M1 実導出対象でない、または値源が値を提供しないとき, the sylphya プロパティシステム shall 当該名に定義された縮退（`%名前` の原文をそのまま返す素通し、または既定値）へ決定論的に落とし、その事実を記録する。
2. When 点付き名が解決できないとき, the sylphya プロパティシステム shall 「不在（NOT_FOUND）」を決定論的に返す（panic・無音失敗にしない）。
3. The sylphya プロパティシステム shall 縮退中の全語彙について、値源（backing）の登録だけで実導出へ置換できる差替シームを備える（key モデル・読み口・消費側契約を変更せずに実導出化できる）。
4. Where SET 有効群（`surface.num`・`animation.num`・`seriko.defaultsurface`・mousecursor 群・seriko.cursor/tooltip・menu/bind.menu 群）が対象のとき, the sylphya プロパティシステム shall 書込 API の型シームのみを予約し、M1 では実書込を行わない。型シームは SET の 2 意味論——運行コマンド書込（`surface.num` SET＝`\s[]` 等価などランタイムへの命令）とストア書込（永続値の更新）——を区別できる形とする。SET 無効項目への書込失敗挙動は正典沈黙のため areka 裁量として対応表へ記録する。
6. The sylphya プロパティシステム shall 値源（backing）の差替シームを、実体層の別——静的構成（load-time 由来）・リアルタイム運行状態（他エンジンが所有する状態の取得/反映）・システム環境（OS 由来・注入シーム必須）・SHIORI 照会・永続——を収容可能な形とする（M1 で実装する backing は M1 実導出対象と永続に限り、リアルタイム運行状態層・システム環境層は縮退のままシームの型のみ層の存在を表現する）。
5. The sylphya プロパティシステム shall ext 亜枝（`activeghostlist(...).ext.*`・`pluginlist(...).ext.*`）の語彙と対応イベント名（property.get/property.set）を予約のみとし、M1 ではイベント発火を行わない。

### Requirement 4: M1 実導出（フラット名: username・selfname・selfname2・keroname）

**Objective:** As an areka ユーザー, I want `%username`・`%selfname`・`%selfname2`・`%keroname` が実際の値源から展開されること, so that バルーンに生の %変数が露出しない

#### Acceptance Criteria

1. When `%username` の値が要求されるとき, the sylphya プロパティシステム shall SHIORI Resource `username` への GET 照会を経た値を供給する（照会経路は本物の SHIORI 照会値源を通る）。
2. If SHIORI が username リソース照会へ 204 No Content または空値を応答したとき, the sylphya プロパティシステム shall 既定値へ決定論的に縮退し、既定値は既存の唯一定義点（dialogue-tags で確立済みの既定ユーザー名）と同一の値とする（既定値の二重定義を作らない）。
3. When `%selfname` の値が要求されるとき, the sylphya プロパティシステム shall ゴーストの descript 由来の本体側の名前（sakura.name）を供給する。
4. When `%selfname2` の値が要求されるとき, the sylphya プロパティシステム shall descript の sakura.name2 由来の値を供給する（現状未読取の sakura.name2 の読取拡張を含む）。If sakura.name2 が未定義のとき, the sylphya プロパティシステム shall 決定論的な縮退規則（正典沈黙＝areka 裁量）を適用し対応表へ記録する。
5. When `%keroname` の値が要求されるとき, the sylphya プロパティシステム shall descript の kero.name 由来の値を供給する。If kero.name が未定義のとき, the sylphya プロパティシステム shall SSP 互換の縮退（本体側の名前へのフォールバック）を適用し対応表へ記録する。

### Requirement 5: M1 実導出（点付き名: baseware）

**Objective:** As a 点付きプロパティの消費者, I want `baseware.name`・`baseware.version` が実値で解決されること, so that 点付き解決経路が最小実証される

#### Acceptance Criteria

1. When `baseware.name` または `baseware.version` が点付き解決されるとき, the sylphya プロパティシステム shall areka 自身の名称・バージョンに対応する実値を返す。
2. When 上記以外の点付き名（`system.*`・リスト系ルート枝配下等）が解決されるとき, the sylphya プロパティシステム shall Requirement 3 の NOT_FOUND 縮退に従う。

### Requirement 6: 永続 backing（4 key 族の器）

**Objective:** As a 後続 spec（position-persist）と将来の消費者, I want ゴースト単位の永続 key 族が sylphya に収容されること, so that 専用永続ストアの二重実装が発生しない

#### Acceptance Criteria

1. The sylphya プロパティシステム shall areka 独自名前空間の永続 key 族として、窓位置（キャラクタースコープ別）・バルーン相対オフセット（キャラクタースコープ別）・起動記録・vanish 回数の 4 key 族を保存・復元可能とする。
2. When 永続値の保存が要求されるとき, the sylphya プロパティシステム shall 書込が途中で中断しても以前に保存済みの有効状態を破壊しない原子的な確定（一時書込→置換）で保存する。
3. If 永続状態が存在しない・読み取れない・解釈できない（破損・未知形式/バージョンを含む）とき, the sylphya プロパティシステム shall 警告ログを記録して「不在」として寛容に縮退し、呼び出し側の起動継続を妨げない。
4. The sylphya プロパティシステム shall 永続直列化形式として TOML を採用し、バージョン識別可能な形式として将来の形式進化時に旧形式を判別できるようにする。
5. The sylphya プロパティシステム shall 永続状態を層別の永続スコープ（areka アプリレベル・SHIORI〔ゴースト〕レベル・シェルレベル・バルーンレベル）で管理し、各層の永続情報を対応する層の profile フォルダへ保存する（伺か慣行準拠）。スコープ内は所属実体（ゴースト等）固有の識別で分離し、他実体・他層の状態と混同しない。
6. The sylphya プロパティシステム shall 4 key 族すべてについて保存→復元の往復で値等価を保証する。
7. If 永続化に起因するいかなる失敗（保存失敗・読取失敗・形式異常）が発生しても, the sylphya プロパティシステム shall panic せず、エラー/警告ログと定義済み縮退で継続する（ゴーストの起動・実行を停止させない）。

### Requirement 7: 既存実装との統合（3 箱分裂の解消）

**Objective:** As an areka 開発者, I want 「名前で引ける値」の解決機構が sylphya の 1 つに統合されること, so that 同じ関心の二重・三重実装が排除される

#### Acceptance Criteria

1. When ゴーストが talk 用スナップショットを供給するとき, the areka システム shall 既存の暫定供給シーム（W1 が用意済みの差替点）を sylphya 読み口由来のスナップショット生成へ差し替え、消費側（さくらスクリプト展開）の契約と挙動を無改変に保つ。
2. The areka システム shall IShioriHost のプロパティ応答（GetProperty/SetProperty）が扱う値を sylphya の管理下へ統合し、独立に充填される第 2 のプロパティストアを存置しない。統合後も既存の観測挙動（dotted key・GetProperty 同期即答・欠落 key はプロパティ不在エラー）を維持する。
3. The areka システム shall %系環境変数の値源・永続ストア・IShioriHost プロパティストアのいずれについても、sylphya 以外の「名前で引ける値」解決機構を新設・存置しない。

### Requirement 8: 横断規律（ログ・環境変数・非決定源）

**Objective:** As an areka 運用者, I want プロパティ解決の失敗が常に観測可能で、非決定源が檻の外に漏れないこと, so that 無音失敗と非決定テストが発生しない

#### Acceptance Criteria

1. The sylphya プロパティシステム shall 失敗経路で無音失敗をしない（失敗はエラー/警告ログ＋定義済み縮退で処理し、panic は致命限定かつ直前ログ付きとする）。
2. Where 本番ランタイムの挙動を環境変数で制御する場合, the sylphya プロパティシステム shall `AREKA_` 名前空間の環境変数のみを読む。
3. The sylphya プロパティシステム shall 暦時計・OS ユーザー名等の非決定な外部環境を暗黙に直読しない（外部環境へのアクセスは必ず注入可能な値源シームを経由し、M1 の縮退実装は外部環境を読まない）。

### Requirement 9: 受け入れ検証（決定論檻＋実機サインオフ）

**Objective:** As a 受け入れ検証者, I want 全判断分岐が決定論的に検証でき、実機で `%username` の展開を確認できること, so that 非決定な I/O に依存せず回帰を檻に入れられる

#### Acceptance Criteria

1. The sylphya プロパティシステム shall 全判断分岐（フラット/点付き解決・素通し/既定値/NOT_FOUND 縮退・per-talk 凍結・SHIORI 照会値源の 204 縮退・寛容読取・原子的書込・往復値等価・層別永続スコープ分離・問い合わせ元コンテキスト分岐・セレクタ文法解釈）を、注入シーム経由の決定論的な単体テストで検証可能とする。
2. The sylphya プロパティシステム shall 決定論テストを x64 純粋テストとして実行可能とする（実 SHIORI・実ファイルシステム障害・実 OS 環境は偽境界の注入で代替する）。
3. The 受け入れ判定 shall 実機（emo2）の撫で talk で `%username` が生文字列としてバルーンへ露出せず、既定値（204 縮退）で表示されること、かつその経路が実 SHIORI 照会値源を通ったことを、有界自動終了＋ログ確認の決定論的判定で確認することを必達とする。
4. The sylphya プロパティシステム shall `%selfname`・`%selfname2`・`%keroname` が descript 実値（および未定義時の縮退規則）どおりに解決されることを決定論的な単体テストで検証可能とする。
