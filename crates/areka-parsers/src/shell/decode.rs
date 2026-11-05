//! 意味層（Decode）— 構文トークン → 値正規化済み `Shell`（emo2 subset 限定）。
//!
//! Lexer（`lexer::lex`）が産んだ構文トークン列 `Vec<Token>` を消費し、シェルサーフェス
//! モデル `Shell` を組み立てる（依存方向 `model ← lexer ← decode`）。下流が再パース
//! 不要なのは、この層が値を decode しきるため（要件 1.2）。sakura 同様に `Result` を
//! 返さず・パニックせず・空入力で空を返す寛容決定的パスを採る（要件 2.x）。
//!
//! 本ファイル（タスク 4.1）が担うのは **decode スケルトンの枠組みのみ**:
//! - トップレベルのトークン走査と、ブロックヘッダによる**ブロックディスパッチ**
//!   （設計 System Flows「iterate top level tokens → 各ブロック種別」）。
//! - charset 行の寛容スキップ（要件 3.1）・descript ブロックの寛容スキップ（要件 3.2）。
//!   いずれもモデルに保持しない（`Shell` に charset/descript フィールドは存在しない）。
//! - `surfaceNNN` ブロックからヘッダ記述子（多 id 形 `N,M`／`N-M` 含む）を append と共通の
//!   ターゲットパーサで捕捉し、代表 id ＋構成要素枠を持つ `Surface` を積む（要件 4.1/12.5(b)）。
//!   読めない記述子・番号は 0 にせず飛ばして崩れ（`ShellIssue`）を積む（指摘 36）。範囲の幅は
//!   シェル全体の合計で上限 `RANGE_WIDTH_CAP` をかける（指摘 6）。
//! - 空入力・ヘッダのみ・ヘッダ欠落・未知ブロック・未知行を寛容吸収し
//!   走査を中断しない（要件 3.3/9.2）。
//!
//! **ディスパッチ優先順位（重要）**: `kero.surface.alias` と `surface.append10` は
//! いずれも部分文字列 "surface" を含むため、素朴な `surfaceNNN` 判定より**先**に
//! これらを分類する。優先順位は
//! `descript` → `kero.surface.alias` → `surface.append*` → `surfaceNNN` → 未知。
//!
//! **スコープ境界（タスク 4.3〜4.6 のシーム）**: 本ファイルは surface の element/collision/
//! animation まで decode する（タスク 4.2 で element/collision・タスク 4.3 で animation 実装済み）。
//! animationN 集約は `decode_animations`（再利用可能ヘルパ）が担い、`decode_surface_body`
//! から呼ぶ。残りはシーム:
//! - `surface.append*` ターゲット捕捉＋追記 collision/animation → タスク 4.4 が
//!   `decode_append_block` を実装し `Shell.appends` を充填する。append の animation 群は
//!   `decode_animations` を再利用する（同一集約規則ゆえヘルパを共用）。
//! - `kero.surface.alias` 写像 → タスク 4.5 が `decode_alias_block` を実装し
//!   `Shell.aliases` を充填する（不透明キー・順序付き ID・重複保持）。
//! - subset 外・不正の寛容吸収の最終化 → タスク 4.6 が確定。overlay・base（element）以外の
//!   element/pattern メソッド・3 種以外の interval・collisionex・非数/欠損フィールドは
//!   いずれも本ファイルの既存シーム（`.get()` ＋ `unwrap_or`・method/kind 判定による
//!   スキップ）で値化せず passthrough 吸収し、既定値へ倒す。新規実装は不要で、4.6 は
//!   この寛容挙動を確認テストで確定させる（要件 2.3/4.5/5.7/6.3/9.2/10.4）。

use super::lexer::Token;
use super::model::{
    AliasKey, Animation, AppendTarget, Collision, CollisionName, DefRef, DrawMethod, Element,
    ElementPath, Interval, Pattern, RANGE_WIDTH_CAP, Shell, ShellIssue, SortOrder, Surface,
    SurfaceAlias, SurfaceAppend,
};

/// 1 つのシェルを読む途中の状態: シェル全体の範囲の幅の合計（指摘 6）・今のブレスの見出し・積んだ崩れ。
#[derive(Default)]
pub(crate) struct Reading {
    range_width: u64,
    heading: String,
    pub(crate) issues: Vec<ShellIssue>,
}

impl Reading {
    /// ブレスの見出しを崩れの名指しに使う原文として控える。
    pub(crate) fn begin(&mut self, header: &[String]) {
        self.heading = header.join(",");
    }

    fn unreadable(&mut self, text: &str) {
        let heading = self.heading.clone();
        let text = text.to_string();
        self.issues
            .push(ShellIssue::UnreadableNumber { heading, text });
    }

    fn too_wide(&mut self, text: &str) {
        let heading = self.heading.clone();
        let text = text.to_string();
        self.issues.push(ShellIssue::RangeTooWide { heading, text });
    }

    fn wait_range(&mut self, text: &str) {
        let heading = self.heading.clone();
        let text = text.to_string();
        self.issues.push(ShellIssue::WaitRange { heading, text });
    }
}

/// 空の `Shell`（読み始め）。
pub(crate) fn empty_shell() -> Shell {
    Shell {
        surfaces: Vec::new(),
        appends: Vec::new(),
        aliases: Vec::new(),
        animation_sort: None,
        collision_sort: None,
        definitions: Vec::new(),
    }
}

/// 構文トークン列を値正規化済みの `Shell` へ写像する（テストの口・崩れは捨てる）。
#[cfg(test)]
pub(crate) fn decode(tokens: Vec<Token>) -> Shell {
    let mut shell = empty_shell();
    let mut reading = Reading::default();
    for token in &tokens {
        decode_token(&mut shell, token, &mut reading);
    }
    shell
}

/// 構文トークン 1 つを `shell` へ写す（`parse_all` が登場順に呼ぶ）。
///
/// - トップレベルを走査し、ブロックヘッダでディスパッチする。charset は読み飛ばし保持しない
///   （要件 3.1）。`surfaceNNN` は枠を積む（要件 4.1）。
/// - 失敗しない（`Result` でない・要件 2.x）。飛ばしたものは `reading` に崩れを積む。
/// - 同一入力で同一出力（決定的）。
pub(crate) fn decode_token(shell: &mut Shell, token: &Token, reading: &mut Reading) {
    match token {
        Token::Brace { header, body } => {
            reading.begin(header);
            dispatch_block(shell, header, body, reading);
        }
        // トップレベル行の `animation-sort`/`collision-sort` を値化する（要件 12.5(a)/1.6）。
        // それ以外（charset 行の非保持・要件 3.1 等）は寛容に読み飛ばす（要件 3.3/9.2）。
        Token::TopLevel(fields) => decode_sort_key(shell, fields),
        // 崩れの知らせは `parse_all` が `ShellIssue` へ写す（ここでは読まない）。
        Token::Unclosed { .. } | Token::Reopened { .. } => {}
    }
}

/// ブレスの見出し（CSV フィールド列）でブロック種別を判定し振り分ける。
///
/// 優先順位は `descript` → `kero.surface.alias` → `surface.append*` → `surfaceNNN` → 未知。
/// alias/append は "surface" を含むため plain `surfaceNNN` より先に判定する（誤分類回避）。
///
/// `body` は閉じたブレスの本体行（CSV フィールド列）。surface ブロックの
/// element/collision decode に用いる。
fn dispatch_block(
    shell: &mut Shell,
    header: &[String],
    body: &[Vec<String>],
    reading: &mut Reading,
) {
    let head = header.first().map(String::as_str).unwrap_or("");

    if head == "descript" {
        // descriptブレスの中の並びの鍵を読む（指摘 4・ukadoc の descriptブレス記述例）。後の行が勝つ。
        // ほかの行（version 等）は保持しない（要件 3.2）。
        for fields in body {
            decode_sort_key(shell, fields);
        }
        return;
    }
    if head == "kero.surface.alias" {
        // alias 写像（要件 8）。本体各行の `KEY,[id,...]` を opaque キー＋順序付き ID へ写す。
        decode_alias_block(shell, body, reading);
        return;
    }
    if head == "surface.alias" {
        // `sakura.surface.alias` と同じく読まない（`surface` の番号の無い surface を作らない・指摘 36）。
        return;
    }
    if head.starts_with("surface.append") {
        // append 追記定義（要件 7）。ヘッダのターゲット指定と本体 collision/animation を decode する。
        decode_append_block(shell, header, body, reading);
        return;
    }
    if let Some(header_number) = head.strip_prefix("surface") {
        // `surfaceNNN` ブロック（要件 4.1）。ヘッダの多 id 形（`N,M` 列挙・`N-M` 範囲）を
        // append と共通のターゲットパーサで記述子リストへ捕捉する（要件 12.5(b)）。従来の
        // 単一 `parse::<u32>().unwrap_or(0)` は多 id 形を破損させていた（design Postcondition）。
        // ヘッダ以降のフィールド（列挙・範囲）を rest として渡す。
        let rest = header.get(1..).unwrap_or(&[]);
        let targets = parse_targets(header_number, rest, reading);
        if targets.is_empty() {
            // 記述子が 1 つも残らないブレスは積まない（指摘 6・36）。
            return;
        }
        // 代表 id は先頭ターゲットの値（`surface1-3`→1・`surface0,5`→0・design Postcondition）。
        let id = representative_id(&targets);
        let (elements, collisions, animations) = decode_surface_body(body, reading);
        // 定義ストリームへ登場順に push（index は push 前の len＝これから積む位置・要件 12.5(d)）。
        shell
            .definitions
            .push(DefRef::Surface(shell.surfaces.len()));
        shell.surfaces.push(Surface {
            id,
            targets,
            elements,
            collisions,
            animations,
        });
        // 末尾分岐ゆえ早期 return は不要（この後は未知ブロック吸収コメントのみ）。
    }

    // 未知ブロック（未対応ヘッダ）は寛容に吸収する（要件 9.2）。何も積まない。
}

/// surface ブロック本体（element/collision/animation 行）を対応型へ decode する。
///
/// タスク 4.2 が element overlay ＋ collision（レイヤ昇順・矩形）を充填する。
/// - element overlay・base: `elementN,overlay,PATH,X,Y`（`base` も同じ）を `Element` へ
///   （画像パスは無加工・レイヤ昇順で安定ソート・要件 4.2/4.3/4.4）。
/// - collision: `collisionN,始点X,始点Y,終点X,終点Y,ID` を `Collision` へ（left/top/
///   right/bottom ＋不透明領域名・出現順・要件 6.1/6.2）。
///
/// animation 行（`animationN.interval` / `animationN.patternM`）は `decode_animations`
/// が animation ID で集約する（タスク 4.3）。overlay・base 以外の element メソッド・
/// collisionex 等は値化せず読み飛ばして passthrough 吸収する（タスク 4.6 が確認・
/// パニックしない・要件 4.5/9.2）。
fn decode_surface_body(
    body: &[Vec<String>],
    reading: &mut Reading,
) -> (Vec<Element>, Vec<Collision>, Vec<Animation>) {
    // element overlay・base は surface・append 双方で同一表現ゆえ共有ヘルパで decode する（要件 12.5(c)）。
    let elements: Vec<Element> = decode_elements(body, reading);
    // collision は surface・append 双方で同一表現ゆえ共有ヘルパで decode する（要件 6.1/6.2/7.3）。
    let collisions: Vec<Collision> = decode_collisions(body, reading);
    // animation は本体行全体を走査する `decode_animations` で集約する（要件 5.6）。
    let animations: Vec<Animation> = decode_animations(body, reading);

    (elements, collisions, animations)
}

/// 本体行群の `elementN,overlay,PATH,X,Y`・`elementN,base,PATH,X,Y` を `Element` へ decode する
/// （要件 4.2/4.3/4.4・areka-P0-element-base-method 要件 1.1）。
///
/// surface 本体・`surface.append` 本体（要件 12.5(c)）双方から呼べる **再利用可能ヘルパ**。
/// append の element も通常 surface と同一のモデル表現・同一集約規則で保持する。
///
/// - 第 2 欄が `is_image_element_method` で真の行（`overlay` と `base`）だけを扱い、どちらも同じ
///   `Element` にする。ほかのメソッド（replace 等）は値化せず読み飛ばして passthrough 吸収する
///   （要件 4.5/10.4）。
/// - 画像パス（field[2]）は無加工保持（区切り正規化なし・要件 4.3）。
/// - element はレイヤインデックス昇順・安定ソート（同レイヤは出現順維持・要件 4.4）。
fn decode_elements(body: &[Vec<String>], reading: &mut Reading) -> Vec<Element> {
    let mut elements: Vec<Element> = Vec::new();

    for fields in body {
        let key = fields.first().map(String::as_str).unwrap_or("");

        if let Some(rest) = key.strip_prefix("element")
            && is_image_element_method(fields.get(1).map(String::as_str).unwrap_or(""))
        {
            // element の N をレイヤインデックスに用いる。読めなければ飛ばす（指摘 36）。
            let Ok(layer) = rest.parse::<u32>() else {
                reading.unreadable(key);
                continue;
            };
            elements.push(Element {
                layer,
                // 画像パス（field[2]）は無加工保持（区切り正規化なし・要件 4.3）。
                path: ElementPath::new(field_string(fields, 2)),
                x: field_i64(fields, 3),
                y: field_i64(fields, 4),
            });
        }
        // collision 行は `decode_collisions`・animation 行は `decode_animations` が別走査で
        // 処理する。その他 subset 外の行は値化せず passthrough 吸収する（要件 9.2/10.4）。
    }

    // element はレイヤインデックス昇順・安定ソート（同レイヤは出現順維持・要件 4.4）。
    elements.sort_by_key(|e| e.layer);

    elements
}

/// 画像の element定義として値にする描画メソッドの語か（完全一致・`overlay` と `base`）。
///
/// element定義の `base` は、`element0` では置き換えられる側が空で、`element1` 以降は正典が
/// `overlay` に読み替えるので、どの番号でも `overlay` と同じ値にする
/// （`ukadoc:descript_shell_surfaces`・areka-P0-element-base-method 要件 1.1・1.4）。
/// 大文字の綴り・空の欄・`add`・`bind`・`balloon` などは偽（描けない語は値にしない）。
pub(super) fn is_image_element_method(word: &str) -> bool {
    matches!(word, "overlay" | "base")
}

/// 本体行群の `collisionN,始点X,始点Y,終点X,終点Y,ID` を `Collision` へ decode する（要件 6.1/6.2）。
///
/// surface 本体・`surface.append` 本体（タスク 4.4）双方から呼べる **再利用可能ヘルパ**。
/// append の collision も通常 surface と同一のモデル表現で保持する（同一 `Collision` 型・要件 7.3）。
///
/// - ukadoc 順序 始点X/始点Y/終点X/終点Y ＝ left/top/right/bottom。
/// - `collisionex`（円/楕円/多角形・要件 6.3）は値化せず passthrough 吸収する（タスク 4.6
///   が確認）。純 `collisionN`（N が数字のみ）だけを materialize する。
/// - collision は出現順で保持する（要件 6.1）。欠落・非数値は既定 0（要件 3.3・パニックしない）。
fn decode_collisions(body: &[Vec<String>], reading: &mut Reading) -> Vec<Collision> {
    let mut collisions: Vec<Collision> = Vec::new();

    for fields in body {
        let key = fields.first().map(String::as_str).unwrap_or("");
        if let Some(rest) = key.strip_prefix("collision") {
            // `collisionex` 等は扱わない。N が数字のみの純 collisionN だけ。
            if rest.chars().all(|c| c.is_ascii_digit()) {
                // 番号の無い・桁あふれの番号は飛ばす（指摘 36）。
                let Ok(index) = rest.parse::<u32>() else {
                    reading.unreadable(key);
                    continue;
                };
                collisions.push(Collision {
                    index,
                    left: field_i64(fields, 1),
                    top: field_i64(fields, 2),
                    right: field_i64(fields, 3),
                    bottom: field_i64(fields, 4),
                    // 領域名（field[5]）は opaque・無加工保持（要件 6.2）。
                    name: CollisionName::new(field_string(fields, 5)),
                });
            }
        }
    }

    collisions
}

/// 本体行の `idx` フィールドを無加工の `String` として取り出す（欠落は空文字列）。
fn field_string(fields: &[String], idx: usize) -> String {
    fields.get(idx).cloned().unwrap_or_default()
}

/// 本体行の `idx` フィールドを `i64` として取り出す（欠落・非数値は既定 0・要件 3.3）。
fn field_i64(fields: &[String], idx: usize) -> i64 {
    fields
        .get(idx)
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0)
}

/// pattern定義のウェイトの欄を読む（指摘 5）。`最小-最大` で両側が数なら小さい方を待ちにして
/// `WaitRange` を積む（ランダムに選ぶのは未対応）。それ以外は数でなければ 0（要件 3.3）。
fn pattern_wait(fields: &[String], idx: usize, reading: &mut Reading) -> u32 {
    let text = fields.get(idx).map_or("", |s| s.trim());
    if let Some((low, high)) = text.split_once('-')
        && let (Ok(low), Ok(high)) = (low.trim().parse::<u32>(), high.trim().parse::<u32>())
    {
        reading.wait_range(text);
        return low.min(high);
    }
    text.parse::<u32>().unwrap_or(0)
}

/// 本体行の `idx` フィールドを `u32` として取り出す（欠落・非数値は既定 0・要件 3.3）。
fn field_u32(fields: &[String], idx: usize) -> u32 {
    fields
        .get(idx)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0)
}

/// 本体行群の `animationN.interval` / `animationN.patternM` を animation ID で集約する（要件 5）。
///
/// タスク 4.3 の中核。surface 本体・`surface.append` 本体（タスク 4.4）双方から呼べる
/// **再利用可能ヘルパ**（append の animation も本ヘルパの同一集約規則を用いる・要件 7.3）。
///
/// 集約規則:
/// - **ドット分割＋ ID 集約**（要件 5.6）: field[0] を最初の `.` で分割し、接頭辞
///   `animationN` から N（`animation` 以降の数字・`unwrap_or(0)`）を、接尾辞から
///   `interval` / `patternM` を得る。同一 N の行は 1 個の `Animation` へ束ねる。
///   animation は N の**初出順**で保持する（z-order 実順序付けはしない・要件 5.6）。
/// - **interval 忠実転記**（要件 5.1/5.2/5.3/8.2）: `interval,KIND[,K]` の KIND を
///   `bind` → `Bind`、`random` → `Random{k}`、`bind+random` → `BindRandom{k}` に正規化し、
///   未認識キーワード（`sometimes`/`rarely`/`always` 等・要件 8.2）は原文のまま
///   `Interval::Other(keyword)` へ転記する（fallback-Bind 撤去・討議 #1 裁定＝語彙を
///   落とさない黙らない）。K は field[2] を u32・欠落既定 0。interval 行は当該 ID の
///   slot を確定させる（interval のみで pattern が無くとも初出順で animation を積む）。
///   pattern が interval 行より先に現れても同一 ID へ束ねる（順序寛容）。
/// - **既定 interval**: ある ID に pattern はあるが interval 行が一度も現れない場合、
///   pattern を失わせずパニックも起こさぬよう既定 `Interval::Bind` で `Animation` を積む
///   （`interval_written` は `false`＝interval 行を書いた ID だけ `true`）。
/// - **pattern の忠実転記＋疎 index ＋負センチネル**（要件 4.6/5.4/5.5/8.4）:
///   `patternM,描画メソッド,ID,WAIT,X,Y` の M（`pattern` 以降の数字・`unwrap_or(0)`）を
///   **そのまま**保持し、欠番を合成しない（疎許容・要件 5.4）。field[1] を描画メソッドと
///   して verbatim 転記する（overlay フィルタ撤去済み＝非 overlay も落とさない・要件
///   4.6/8.4・意味解釈は下流）。`surface_id` は field[2] を **i64** で保持し、負値
///   （`-1`/`-2`）をセンチネルとして失わない（要件 5.5・意味付けは下流）。出現順で保持する。
///
/// 失敗しない（`Vec` を返す・パニックしない・空/崩れ入力に寛容・要件 2.x/9.2）。
fn decode_animations(body: &[Vec<String>], reading: &mut Reading) -> Vec<Animation> {
    // ID の初出順を保つため Vec で管理し、線形探索で既存 animation を引く
    // （emo2 subset の animation 数は小さく、決定性と初出順保持を優先する）。
    let mut animations: Vec<Animation> = Vec::new();

    for fields in body {
        let key = fields.first().map(String::as_str).unwrap_or("");

        // `animationN.suffix` のみ対象。`animation` で始まらない行は他型ゆえ無視する。
        let rest = match key.strip_prefix("animation") {
            Some(rest) => rest,
            None => continue,
        };
        // 最初の `.` で `N` と suffix（`interval` / `patternM`）へ分割する。
        // `.` が無い崩れキー（例 `animationX`）は subset 外ゆえ読み飛ばす（要件 9.2）。
        let (id_text, suffix) = match rest.split_once('.') {
            Some(pair) => pair,
            None => continue,
        };
        // 読めない ID は 0 にせず行を飛ばす（指摘 36・パニックしない）。
        let Ok(id) = id_text.parse::<u32>() else {
            reading.unreadable(key);
            continue;
        };

        if suffix == "interval" {
            // interval 行を忠実転記する（要件 8.2・討議 #1 裁定）。3 種（bind/random/
            // bind+random）は駆動語彙へ、未認識キーワード（sometimes 等）は原文のまま
            // `Interval::Other` へ転記する（fallback-Bind は撤去済み＝語彙を落とさない・
            // 黙らない）。interval 行は当該 ID の slot を確定させる（初出順保持）。
            let interval = normalize_interval(fields);
            let anim = animation_slot(&mut animations, id);
            anim.interval = interval;
            anim.interval_written = true;
            continue;
        }

        if let Some(index_text) = suffix.strip_prefix("pattern") {
            // 全 pattern 行を忠実転記する（要件 4.6/8.4）。overlay フィルタは撤去済み＝
            // 非 overlay（replace/base/制御系 等）も落とさず method を運ぶ。意味解釈
            // （合成方法・実装可否）は下流の責務（parser は原文を無加工で運ぶ）。
            // 欄 2 が `(` の並びなら `)` まで継ぎ直し、待ち・X・Y はその次から読む（要件 2.2）。
            // pattern index は疎許容・そのまま保持（欠番を合成しない・要件 5.4）。読めなければ飛ばす（指摘 36）。
            let Ok(index) = index_text.parse::<u32>() else {
                reading.unreadable(key);
                continue;
            };
            let (target, tail) = pattern_target(fields);
            let pattern = Pattern {
                index,
                // field[1] を描画メソッドとして verbatim 転記（欠落は空文字＝下流 `Unknown`
                // 吸収）。位置・型は不変で surface_id 以降を破壊しない（要件 4.6/8.4）。
                method: DrawMethod::new(field_string(fields, 1)),
                // surface_id は i64・負センチネル（-1/-2）を失わない（要件 5.5）。
                surface_id: field_i64(fields, 2),
                target,
                wait: pattern_wait(fields, tail, reading),
                x: field_i64(fields, tail + 1),
                y: field_i64(fields, tail + 2),
            };
            let anim = animation_slot(&mut animations, id);
            anim.patterns.push(pattern);
            continue;
        }

        // `interval` / `patternM` 以外の suffix（subset 外）は読み飛ばす（要件 9.2）。
    }

    animations
}

/// pattern定義の欄 2 の原文と、待ちを読む欄の位置を返す（要件 2.2・2.3・2.6）。
///
/// 字句の段は `[ ]` の中のカンマしか守らないので、欄 2 が `(` で始まり `)` を含まないときだけ、
/// `)` を含む欄（無ければ行の終わり）までを `,` で継ぎ直す。語の意味は知らない（原文を写すだけ）。
fn pattern_target(fields: &[String]) -> (String, usize) {
    let col = field_string(fields, 2);
    if !col.starts_with('(') || col.contains(')') {
        return (col, 3);
    }
    let end = fields[2..]
        .iter()
        .position(|f| f.contains(')'))
        .map_or(fields.len(), |p| 2 + p + 1);
    (fields[2..end].join(","), end)
}

/// `animations` 内の ID `id` の `Animation` への可変参照を返す。無ければ既定 `Interval::Bind`
/// ＋空 pattern の枠を初出順で末尾に積んでから返す（interval が pattern より後に来ても
/// 同一 ID へ束ねられるようにする・要件 5.6）。
fn animation_slot(animations: &mut Vec<Animation>, id: u32) -> &mut Animation {
    if let Some(pos) = animations.iter().position(|a| a.id == id) {
        return &mut animations[pos];
    }
    animations.push(Animation {
        id,
        // 認識可能な interval 行を見るまでの暫定既定（要件 5.1 の既定挙動）。
        interval: Interval::Bind,
        interval_written: false,
        patterns: Vec::new(),
    });
    let last = animations.len() - 1;
    &mut animations[last]
}

/// `animationN.interval,KIND[,K]` 行を `Interval` へ忠実転記する（要件 5.1/5.2/5.3/8.2）。
///
/// KIND（field[1]）が駆動 3 種（bind/random/bind+random）ならその語彙へ、それ以外の
/// 未認識キーワード（sometimes/rarely/always 等）は原文のまま
/// `Interval::Other(keyword)` へ転記する（fallback-Bind 撤去・討議 #1 裁定・要件 8.2）。
/// 転記層は語彙を落とさず黙らない。KIND 欠落は空文字 `Other("")`（下流吸収・要件 3.3）。
/// K（field[2]）は u32・欠落既定 0（要件 3.3）。
///
/// 引き金の 3 語（areka-P0-seriko-trigger-intervals 要件 1.1〜1.5）は小文字の完全一致だけを
/// 読む: `runonce` → `Runonce`（第 3 欄以降は読まない）、`talk`／`periodic` は field[2] が
/// 1 以上の整数なら `Talk{n}`／`Periodic{secs}`、読めないとき（欠落・0・非数値）は第 2 欄以降を
/// `,` で繋いだ原文を `Other` へ運ぶ（失敗せず記録も出さない・無効の記録は下流の表が出す）。
fn normalize_interval(fields: &[String]) -> Interval {
    match fields.get(1).map(String::as_str) {
        Some("bind") => Interval::Bind,
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#random_2c_6570_5024:1
        Some("random") => Interval::Random {
            k: field_u32(fields, 2),
        },
        Some("bind+random") => Interval::BindRandom {
            k: field_u32(fields, 2),
        },
        Some("runonce") => Interval::Runonce,
        // `field_u32` は欠落・非数値を 0 へ倒すので、0 が「正の整数として読めない」の全部。
        Some(word @ ("talk" | "periodic")) => match (word, field_u32(fields, 2)) {
            (_, 0) => Interval::Other(fields[1..].join(",").into()),
            ("talk", n) => Interval::Talk { n },
            (_, secs) => Interval::Periodic { secs },
        },
        // 未認識キーワード・欠落は原文（欠落は空文字）を `Other` へ忠実転記する（要件 8.2）。
        other => Interval::Other(other.unwrap_or("").into()),
    }
}

/// `surface.append*` 追記ブロックを `SurfaceAppend` へ decode し `shell.appends` へ積む（要件 7）。
///
/// - ターゲット指定はヘッダから `parse_targets` で **記述子リスト**として捕捉する
///   （展開しない・ヘッダ数値は第1要素として一様に扱う・要件 7.1/7.2）。
/// - collision/animation は通常 surface と同一表現で保持する。collision は共有ヘルパ
///   `decode_collisions`、animation は共有ヘルパ `decode_animations`（タスク 4.3）を
///   再利用する（同一集約規則ゆえ共用・要件 7.3）。
/// - append は出現順で `shell.appends` に保持する（要件 7.1）。
/// - 失敗しない（`Result` でない・パニックしない・要件 2.x）。
fn decode_append_block(
    shell: &mut Shell,
    header: &[String],
    body: &[Vec<String>],
    reading: &mut Reading,
) {
    // ヘッダ先頭 `surface.appendNNN` から接頭辞を剥がしヘッダ数値 NNN を得る。
    // 読めない数は `parse_target_element` が飛ばす（パニックしない・指摘 36）。
    let head = header.first().map(String::as_str).unwrap_or("");
    let header_number = head.strip_prefix("surface.append").unwrap_or("");
    // ヘッダ以降のフィールド（列挙・範囲）を rest として渡す。
    let rest = header.get(1..).unwrap_or(&[]);
    let targets = parse_targets(header_number, rest, reading);
    if targets.is_empty() {
        // 記述子が 1 つも残らないブレスは積まない（指摘 6・36）。
        return;
    }

    // 定義ストリームへ登場順に push（index は push 前の len＝これから積む位置・要件 12.5(d)）。
    shell.definitions.push(DefRef::Append(shell.appends.len()));
    shell.appends.push(SurfaceAppend {
        targets,
        // append 内 element も転記する（従来黙殺・要件 12.5(c)/2.4）。
        elements: decode_elements(body, reading),
        collisions: decode_collisions(body, reading),
        animations: decode_animations(body, reading),
    });
}

/// append ターゲット指定を記述子リスト `Vec<AppendTarget>` へ捕捉する（展開しない・要件 7.2）。
///
/// ヘッダ数値 `header` を **第1要素**とし、後続 `rest`（列挙・範囲）を続けた順序付きリストを返す。
/// ヘッダ数値は列挙された単一 ID と同格・一様に扱い、カテゴリ番号的な特別扱いはしない
/// （メモリ「キーワード直後の数値は id リスト第1要素・surface0-2 と一様」）。範囲 `a-b` は
/// `Range` 記述子として保持し、個別 ID へ展開しない（展開・実 surface ツリーへの転記は下流）。
///
/// 入力例: `header="10"`, `rest=["2100-2110", "2200-2210"]`
///   → `[Single(10), Range{2100,2110}, Range{2200,2210}]`。
/// 入力例: `header="2200"`, `rest=[]` → `[Single(2200)]`。
///
/// 読めない記述子と、範囲の幅の合計が上限を越える記述子は飛ばし、`reading` に崩れを積む
/// （指摘 36・6）。箱の転記も同じ関数を通るので同じ上限がかかる。
pub(super) fn parse_targets(
    header: &str,
    rest: &[String],
    reading: &mut Reading,
) -> Vec<AppendTarget> {
    // ヘッダ数値は列挙要素と同一コードパスで第1要素として積む（一様扱い）。
    std::iter::once(header)
        .chain(rest.iter().map(String::as_str))
        .filter_map(|field| parse_target_element(field, reading))
        .collect()
}

/// 単一ターゲットフィールドを `AppendTarget` へ写像する（ヘッダ数値・列挙要素で共用・要件 7.2/2.5）。
///
/// - 先頭 `!` は除外指定（`!N`→`Exclude`・`!a-b`→`ExcludeRange`）。記述子のまま保持し減算しない（要件 2.5）。
/// - `-` を含むフィールドは `a-b` 範囲とみなし `Range{start,end}` を返す（展開しない）。それ以外は `Single`。
///   範囲の両端は trim する。
/// - 読めない数は 0 にせず `None`（`UnreadableNumber`・指摘 36）。
/// - `Range` の幅（`end - start + 1`・`u64`）をシェル全体で足し、合計が `RANGE_WIDTH_CAP` を越えるなら
///   `None`（`RangeTooWide`・指摘 6）。番号の値と除外の範囲（広げない）にはかけない。
fn parse_target_element(field: &str, reading: &mut Reading) -> Option<AppendTarget> {
    let number = |text: &str| text.trim().parse::<u32>().ok();
    let pair = |text: &str| {
        let (start, end) = text.split_once('-')?;
        Some((number(start)?, number(end)?))
    };
    // 除外指定 `!`（satori 実例・ukadoc）。`!` を剥がして範囲/単一を判定する（要件 2.5）。
    let target = match field.strip_prefix('!') {
        Some(excluded) if excluded.contains('-') => {
            pair(excluded).map(|(start, end)| AppendTarget::ExcludeRange { start, end })
        }
        Some(excluded) => number(excluded).map(AppendTarget::Exclude),
        None if field.contains('-') => {
            pair(field).map(|(start, end)| AppendTarget::Range { start, end })
        }
        None => number(field).map(AppendTarget::Single),
    };
    let Some(target) = target else {
        reading.unreadable(field);
        return None;
    };
    if let AppendTarget::Range { start, end } = target {
        // 逆向きの範囲も同じ幅で数える（少なく見積もらない）。
        let width = u64::from(start.abs_diff(end)) + 1;
        if reading.range_width + width > RANGE_WIDTH_CAP {
            reading.too_wide(field);
            return None;
        }
        reading.range_width += width;
    }
    Some(target)
}

/// ターゲット記述子リストの **代表 id** を返す（`Surface.id` 用・design Postcondition）。
///
/// 先頭ターゲットの値を採る: `Single(n)`/`Exclude(n)`→n・`Range{start,..}`/`ExcludeRange{start,..}`→start。
/// 空リスト（崩れヘッダ）は既定 0（要件 3.3・パニックしない）。
fn representative_id(targets: &[AppendTarget]) -> u32 {
    match targets.first() {
        Some(AppendTarget::Single(n)) | Some(AppendTarget::Exclude(n)) => *n,
        Some(AppendTarget::Range { start, .. })
        | Some(AppendTarget::ExcludeRange { start, .. }) => *start,
        _ => 0,
    }
}

/// トップレベル行が `animation-sort`／`collision-sort` なら値を `shell` へ充填する（要件 12.5(a)/1.6）。
///
/// - field[0] がキー名・field[1] が値（`ascend`→`Ascend`・`descend`→`Descend`）。
/// - それ以外の空でない値は原文のまま `Other` で運ぶ。空の値・欠落は `None`（既定適用は下流・要件 3.3/9.2）。
/// - キーが両者いずれでもない行（`charset` 等）は何もしない（非保持・要件 3.1）。
fn decode_sort_key(shell: &mut Shell, fields: &[String]) {
    let key = fields.first().map(String::as_str).unwrap_or("");
    let value = fields.get(1).map(String::as_str);
    let order = match value {
        Some("ascend") => Some(SortOrder::Ascend),
        Some("descend") => Some(SortOrder::Descend),
        // 知らない値は原文を運ぶ（既定 descend/none の解釈と知らせは下流）。
        Some(other) if !other.is_empty() => Some(SortOrder::Other(other.into())),
        _ => None,
    };
    match key {
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#animation-sort_2c_30bd_30fc_30c8_9806_5e8f:1
        "animation-sort" => shell.animation_sort = order,
        "collision-sort" => shell.collision_sort = order,
        _ => {}
    }
}

/// `kero.surface.alias` ブロックの本体行群を `SurfaceAlias` 群へ decode し `shell.aliases` へ積む（要件 8）。
///
/// 各本体行は lexer が CSV 分割済みで、`KEY,[id,...]` は field[0]==KEY・field[1]=="[id,...]"
/// として届く（`[...]` 内カンマは lexer が保護するため 1 フィールド・要件 8.1）。
///
/// - **キーは不透明**（field[0]）: 数値（`6`）・日本語（`静観`）いずれも意味解釈せず
///   verbatim に `AliasKey` へ包む（数値 parse しない・要件 8.2）。
/// - **値は順序付き数値 ID**: field[1] の角括弧を剥がし、内側をカンマ分割して u32 へ
///   写す（出現順保持・要件 8.3）。読めない項目は 0 にせず飛ばして崩れを積む（指摘 36）。
/// - **重複キーは潰さない**: `shell.aliases` は `Vec` ゆえ各行を単純 push し、同一キーの
///   複数出現を全保持する（衝突解決は下流・要件 8.4）。出現順を保つ。
/// - キー欠落（空行相当）は寛容に読み飛ばす。値欠落・空 `[]` は空 ids で保持する（要件 9.2）。
///
/// 失敗しない（`Result` でない・パニックしない・要件 2.x）。
fn decode_alias_block(shell: &mut Shell, body: &[Vec<String>], reading: &mut Reading) {
    for fields in body {
        // キーは field[0]。欠落した崩れ行（空フィールド列）は読み飛ばす（要件 9.2）。
        let key = match fields.first() {
            Some(k) => k,
            None => continue,
        };
        // 定義ストリームへ登場順に push（各行＝1エントリ・index は push 前の len・要件 12.5(d)）。
        shell.definitions.push(DefRef::Alias(shell.aliases.len()));
        shell.aliases.push(SurfaceAlias {
            // キーは不透明・無加工（数値 parse しない・要件 8.2）。
            key: AliasKey::new(key.clone()),
            // 値 `[id,...]` を順序付き u32 リストへ（角括弧剥がし＋カンマ分割・要件 8.3）。
            ids: parse_alias_ids(fields.get(1).map(String::as_str).unwrap_or(""), reading),
        });
    }
}

/// alias 値フィールド `[id,id,...]` を順序付き `Vec<u32>` へ写す（要件 8.3）。
///
/// - 前後の `[` `]` を剥がす（無ければ全体をそのまま扱う寛容措置・要件 9.2）。
/// - 内側をカンマ分割し各要素を u32 化する。空要素・非数値は飛ばして崩れを積む（指摘 36）。
/// - 空（`[]` または空文字列）は空 `Vec` を返す（要件 9.2）。
fn parse_alias_ids(value: &str, reading: &mut Reading) -> Vec<u32> {
    // 角括弧を防御的に剥がす（`[2106,2206]` → `2106,2206`）。無ければそのまま。
    let inner = value
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(value)
        .trim();
    if inner.is_empty() {
        return Vec::new();
    }
    inner
        .split(',')
        .filter_map(|s| {
            let id = s.trim().parse::<u32>().ok();
            if id.is_none() {
                reading.unreadable(s.trim());
            }
            id
        })
        .collect()
}

/// 文字コードごとの固定物 4 種が同一の解析結果を産むことを固定する兄弟テスト（タスク 4.2・要件 9.5）。
#[cfg(test)]
#[path = "decode_charset_tests.rs"]
mod charset_tests;

/// 引き金の 3 語（`talk,数値`・`runonce`・`periodic,数値`）の読みを固定する兄弟テスト
/// （areka-P0-seriko-trigger-intervals 要件 9.1）。
#[cfg(test)]
#[path = "decode_interval_trigger_tests.rs"]
mod interval_trigger_tests;

/// pattern定義の欄 2 の原文（`(` で始まる並びの継ぎ直しを含む）を固定する兄弟テスト
/// （seriko-pattern-calls 要件 2.2・2.3・2.6・2.7）。
#[cfg(test)]
#[path = "decode_tests_pattern_target_tests.rs"]
mod pattern_target_tests;
