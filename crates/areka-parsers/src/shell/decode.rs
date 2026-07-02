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
//! - `surfaceNNN` ブロックから surface ID を取り出し、（当面空の）構成要素枠を持つ
//!   `Surface` を積む（要件 4.1）。ID 抽出失敗は `unwrap_or(0)` で既定 0 に倒す（要件 3.3）。
//! - 空入力・ヘッダのみ・ヘッダ欠落・未知ブロック・未知行・`Raw` を寛容吸収し
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
//! - subset 外・不正の寛容吸収の最終化 → タスク 4.6。

use super::lexer::Token;
use super::model::{
    AliasKey, Animation, AppendTarget, Collision, CollisionName, Element, ElementPath, Interval,
    Pattern, Shell, Surface, SurfaceAlias, SurfaceAppend,
};

/// 構文トークン列を値正規化済みの `Shell` へ写像する（mod 内・`parse` が結線する）。
///
/// - 入力 `tokens` は lexer 出力（構文区切り済み）。
/// - トップレベルを走査し、ブロックヘッダでディスパッチする。charset/descript は
///   読み飛ばし保持しない（要件 3.1/3.2）。`surfaceNNN` は枠を積む（要件 4.1）。
/// - 失敗しない（`Vec`/`Shell` を返す・`Result` でない・要件 2.x）。空入力で空 `Shell`。
/// - 同一入力で同一出力（決定的）。
pub(crate) fn decode(tokens: Vec<Token>) -> Shell {
    let mut shell = Shell {
        surfaces: Vec::new(),
        appends: Vec::new(),
        aliases: Vec::new(),
    };

    // ブロックの本体行（BlockStart..BlockEnd の間の Line 群）を蓄えるカーソル。
    // BlockStart でヘッダを開き、以降の Line を積み、BlockEnd でヘッダ＋本体を
    // ディスパッチする。ブロック外の Line は崩れ入力ゆえ寛容に読み飛ばす（要件 3.3/9.2）。
    let mut open: Option<(Vec<String>, Vec<Vec<String>>)> = None;

    for token in tokens {
        match token {
            Token::BlockStart(header) => {
                // 直前の BlockStart が BlockEnd を見ずに再び開いた崩れ入力でも走査を
                // 止めない。開いていた分をここで確定させてから新ブロックを開く。
                if let Some((prev_header, prev_body)) = open.take() {
                    dispatch_block(&mut shell, &prev_header, &prev_body);
                }
                open = Some((header, Vec::new()));
            }
            Token::Line(fields) => {
                // ブロック本体行はカーソルに蓄える。ブロック外の Line は吸収する。
                if let Some((_, body)) = open.as_mut() {
                    body.push(fields);
                }
            }
            Token::BlockEnd => {
                // ブロック終端でヘッダ＋本体をディスパッチする。
                if let Some((header, body)) = open.take() {
                    dispatch_block(&mut shell, &header, &body);
                }
            }
            // トップレベル行・不正断片は個別処理しない。charset 行の非保持（要件 3.1）を
            // 含め寛容に読み飛ばす（要件 3.3/9.2）。値化はタスク 4.3〜4.6 の領分。
            Token::TopLevel(_) | Token::Raw(_) => {}
        }
    }

    // BlockEnd を見ずに EOF に達した崩れ入力（未閉じブロック）でも、開いていた分を
    // 取りこぼさずディスパッチする（走査を中断しない・要件 9.2）。
    if let Some((header, body)) = open.take() {
        dispatch_block(&mut shell, &header, &body);
    }

    shell
}

/// ブロックヘッダ（`BlockStart` の CSV フィールド列）でブロック種別を判定し振り分ける。
///
/// 優先順位は `descript` → `kero.surface.alias` → `surface.append*` → `surfaceNNN` → 未知。
/// alias/append は "surface" を含むため plain `surfaceNNN` より先に判定する（誤分類回避）。
///
/// `body` は BlockStart..BlockEnd 間の本体行（CSV フィールド列）。surface ブロックの
/// element/collision decode に用いる。
fn dispatch_block(shell: &mut Shell, header: &[String], body: &[Vec<String>]) {
    let head = header.first().map(String::as_str).unwrap_or("");

    if head == "descript" {
        // descript ブロックは寛容スキップ・非保持（要件 3.2）。本体も破棄する。
        return;
    }
    if head == "kero.surface.alias" {
        // alias 写像（要件 8）。本体各行の `KEY,[id,...]` を opaque キー＋順序付き ID へ写す。
        decode_alias_block(shell, body);
        return;
    }
    if head.starts_with("surface.append") {
        // append 追記定義（要件 7）。ヘッダのターゲット指定と本体 collision/animation を decode する。
        decode_append_block(shell, header, body);
        return;
    }
    if let Some(rest) = head.strip_prefix("surface") {
        // `surfaceNNN` ブロック（要件 4.1）。NNN を取り出し、本体を decode して積む。
        // 非数値・欠落は `unwrap_or(0)` で既定 0 に倒す（要件 3.3・パニックしない）。
        let id = rest.parse::<u32>().unwrap_or(0);
        let (elements, collisions, animations) = decode_surface_body(body);
        shell.surfaces.push(Surface {
            id,
            elements,
            collisions,
            animations,
        });
        return;
    }

    // 未知ブロック（未対応ヘッダ）は寛容に吸収する（要件 9.2）。何も積まない。
}

/// surface ブロック本体（element/collision/animation 行）を対応型へ decode する。
///
/// タスク 4.2 が element overlay ＋ collision（レイヤ昇順・矩形）を充填する。
/// - element overlay: `elementN,overlay,PATH,X,Y` を `Element` へ（画像パスは無加工・
///   レイヤ昇順で安定ソート・要件 4.2/4.3/4.4）。
/// - collision: `collisionN,始点X,始点Y,終点X,終点Y,ID` を `Collision` へ（left/top/
///   right/bottom ＋不透明領域名・出現順・要件 6.1/6.2）。
///
/// animation 行（`animationN.interval` / `animationN.patternM`）は `decode_animations`
/// が animation ID で集約する（タスク 4.3）。overlay 以外の element メソッド・
/// collisionex 等の寛容吸収はタスク 4.6 の領分ゆえ、扱わない行は単に読み飛ばす
/// （パニックしない・要件 9.2）。
fn decode_surface_body(body: &[Vec<String>]) -> (Vec<Element>, Vec<Collision>, Vec<Animation>) {
    let mut elements: Vec<Element> = Vec::new();
    // collision は surface・append 双方で同一表現ゆえ共有ヘルパで decode する（要件 6.1/6.2/7.3）。
    let collisions: Vec<Collision> = decode_collisions(body);
    // animation は本体行全体を走査する `decode_animations` で集約する（要件 5.6）。
    let animations: Vec<Animation> = decode_animations(body);

    for fields in body {
        let key = fields.first().map(String::as_str).unwrap_or("");

        // element overlay 行（要件 4.2/4.3/4.4）。field[1] == "overlay" のみ扱う。
        // overlay 以外のメソッドはタスク 4.6 の寛容吸収に委ね、ここでは読み飛ばす。
        if let Some(rest) = key.strip_prefix("element") {
            let is_overlay = fields.get(1).map(String::as_str) == Some("overlay");
            if is_overlay {
                elements.push(Element {
                    // element の N をレイヤインデックスに用いる。非数値は既定 0（要件 3.3）。
                    layer: rest.parse::<u32>().unwrap_or(0),
                    // 画像パス（field[2]）は無加工保持（区切り正規化なし・要件 4.3）。
                    path: ElementPath::new(field_string(fields, 2)),
                    x: field_i64(fields, 3),
                    y: field_i64(fields, 4),
                });
            }
            continue;
        }

        // collision 行は `decode_collisions` が別走査で処理するためここでは扱わない。
        // animation 行（`animationN.*`）も `decode_animations` が別走査で集約する。
        // その他 subset 外の行はタスク 4.6 のシームゆえ読み飛ばす（要件 9.2）。
    }

    // element はレイヤインデックス昇順・安定ソート（同レイヤは出現順維持・要件 4.4）。
    elements.sort_by_key(|e| e.layer);

    (elements, collisions, animations)
}

/// 本体行群の `collisionN,始点X,始点Y,終点X,終点Y,ID` を `Collision` へ decode する（要件 6.1/6.2）。
///
/// surface 本体・`surface.append` 本体（タスク 4.4）双方から呼べる **再利用可能ヘルパ**。
/// append の collision も通常 surface と同一のモデル表現で保持する（同一 `Collision` 型・要件 7.3）。
///
/// - ukadoc 順序 始点X/始点Y/終点X/終点Y ＝ left/top/right/bottom。
/// - `collisionex`（要件 6.3 のタスク 4.6 寛容吸収）は扱わない。純 `collisionN`（N が数字のみ）だけ。
/// - collision は出現順で保持する（要件 6.1）。欠落・非数値は既定 0（要件 3.3・パニックしない）。
fn decode_collisions(body: &[Vec<String>]) -> Vec<Collision> {
    let mut collisions: Vec<Collision> = Vec::new();

    for fields in body {
        let key = fields.first().map(String::as_str).unwrap_or("");
        if let Some(rest) = key.strip_prefix("collision") {
            // `collisionex` 等は扱わない。N が数字のみの純 collisionN だけ。
            if rest.chars().all(|c| c.is_ascii_digit()) {
                collisions.push(Collision {
                    index: rest.parse::<u32>().unwrap_or(0),
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
/// - **interval 3 種**（要件 5.1/5.2/5.3）: `interval,KIND[,K]` の KIND を
///   `bind` → `Bind`、`random` → `Random{k}`、`bind+random` → `BindRandom{k}` に正規化する
///   （K は field[2] を u32・欠落既定 0）。interval は animation 定義の始点（ukadoc）だが、
///   pattern が interval 行より先に現れても同一 ID へ束ねる（順序寛容）。
/// - **既定 interval**: ある ID に pattern はあるが認識可能な interval 行が無い場合、
///   pattern を失わせずパニックも起こさぬよう既定 `Interval::Bind` で `Animation` を積む。
///   3 種以外の interval キーワード（`sometimes`/`periodic` 等・要件 5.7）は、本タスクでは
///   「認識可能な interval 無し」と同様に扱い既定 `Bind` に倒す。要件 5.7 の正式な寛容吸収は
///   タスク 4.6 の領分ゆえここでは実装しない。
/// - **pattern の疎 index ＋負センチネル**（要件 5.4/5.5）: `patternM,overlay,ID,WAIT,X,Y`
///   の M（`pattern` 以降の数字・`unwrap_or(0)`）を**そのまま**保持し、欠番を合成しない
///   （疎許容・要件 5.4）。`surface_id` は field[2] を **i64** で保持し、負値（`-1`/`-2`）を
///   センチネルとして失わない（要件 5.5・意味付けは下流）。overlay 以外の method は
///   タスク 4.6 の寛容吸収シームゆえ本タスクでは読み飛ばす。pattern は出現順で保持する。
///
/// 失敗しない（`Vec` を返す・パニックしない・空/崩れ入力に寛容・要件 2.x/9.2）。
fn decode_animations(body: &[Vec<String>]) -> Vec<Animation> {
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
        // 非数値 ID は既定 0（要件 3.3・パニックしない）。
        let id = id_text.parse::<u32>().unwrap_or(0);

        if suffix == "interval" {
            // interval 3 種のみ正規化する。それ以外（要件 5.7）は束ねずスキップし、
            // 該当 ID は既定 Bind に倒れる（このタスクのシーム決定）。
            if let Some(interval) = normalize_interval(fields) {
                let anim = animation_slot(&mut animations, id);
                anim.interval = interval;
            }
            continue;
        }

        if let Some(index_text) = suffix.strip_prefix("pattern") {
            // overlay メソッドのみ充填する（要件 5.4/5.5）。非 overlay はタスク 4.6 のシーム。
            if fields.get(1).map(String::as_str) == Some("overlay") {
                let pattern = Pattern {
                    // pattern index は疎許容・そのまま保持（欠番を合成しない・要件 5.4）。
                    index: index_text.parse::<u32>().unwrap_or(0),
                    // surface_id は i64・負センチネル（-1/-2）を失わない（要件 5.5）。
                    surface_id: field_i64(fields, 2),
                    wait: field_u32(fields, 3),
                    x: field_i64(fields, 4),
                    y: field_i64(fields, 5),
                };
                let anim = animation_slot(&mut animations, id);
                anim.patterns.push(pattern);
            }
            continue;
        }

        // `interval` / `patternM` 以外の suffix（subset 外）は読み飛ばす（要件 9.2）。
    }

    animations
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
        patterns: Vec::new(),
    });
    let last = animations.len() - 1;
    &mut animations[last]
}

/// `animationN.interval,KIND[,K]` 行を `Interval` へ正規化する（要件 5.1/5.2/5.3）。
///
/// KIND（field[1]）が 3 種のいずれでもない場合は `None` を返す（呼び出し側で既定 Bind へ倒す・
/// 要件 5.7 の正式吸収はタスク 4.6）。K（field[2]）は u32・欠落既定 0（要件 3.3）。
fn normalize_interval(fields: &[String]) -> Option<Interval> {
    match fields.get(1).map(String::as_str) {
        Some("bind") => Some(Interval::Bind),
        Some("random") => Some(Interval::Random {
            k: field_u32(fields, 2),
        }),
        Some("bind+random") => Some(Interval::BindRandom {
            k: field_u32(fields, 2),
        }),
        _ => None,
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
fn decode_append_block(shell: &mut Shell, header: &[String], body: &[Vec<String>]) {
    // ヘッダ先頭 `surface.appendNNN` から接頭辞を剥がしヘッダ数値 NNN を得る。
    // 剥がせない崩れヘッダでも `parse_target_element` が既定 0 に倒す（パニックしない）。
    let head = header.first().map(String::as_str).unwrap_or("");
    let header_number = head.strip_prefix("surface.append").unwrap_or("");
    // ヘッダ以降のフィールド（列挙・範囲）を rest として渡す。
    let rest = header.get(1..).unwrap_or(&[]);

    shell.appends.push(SurfaceAppend {
        targets: parse_targets(header_number, rest),
        collisions: decode_collisions(body),
        animations: decode_animations(body),
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
fn parse_targets(header: &str, rest: &[String]) -> Vec<AppendTarget> {
    let mut targets: Vec<AppendTarget> = Vec::with_capacity(1 + rest.len());
    // ヘッダ数値は列挙要素と同一コードパスで第1要素として積む（一様扱い）。
    targets.push(parse_target_element(header));
    for field in rest {
        targets.push(parse_target_element(field));
    }
    targets
}

/// 単一ターゲットフィールドを `AppendTarget` へ写像する（ヘッダ数値・列挙要素で共用・要件 7.2）。
///
/// `-` を含むフィールドは `a-b` 範囲とみなし `Range{start,end}` を返す（展開しない）。それ以外は
/// `Single`。数値化失敗は `unwrap_or(0)` で既定 0 に倒す（要件 3.3・パニックしない）。
fn parse_target_element(field: &str) -> AppendTarget {
    if let Some((start, end)) = field.split_once('-') {
        AppendTarget::Range {
            start: start.parse::<u32>().unwrap_or(0),
            end: end.parse::<u32>().unwrap_or(0),
        }
    } else {
        AppendTarget::Single(field.parse::<u32>().unwrap_or(0))
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
///   写す（出現順保持・要件 8.3）。非数値・空要素は既定 0 に倒す（要件 3.3・パニックしない）。
/// - **重複キーは潰さない**: `shell.aliases` は `Vec` ゆえ各行を単純 push し、同一キーの
///   複数出現を全保持する（衝突解決は下流・要件 8.4）。出現順を保つ。
/// - キー欠落（空行相当）は寛容に読み飛ばす。値欠落・空 `[]` は空 ids で保持する（要件 9.2）。
///
/// 失敗しない（`Result` でない・パニックしない・要件 2.x）。
fn decode_alias_block(shell: &mut Shell, body: &[Vec<String>]) {
    for fields in body {
        // キーは field[0]。欠落した崩れ行（空フィールド列）は読み飛ばす（要件 9.2）。
        let key = match fields.first() {
            Some(k) => k,
            None => continue,
        };
        shell.aliases.push(SurfaceAlias {
            // キーは不透明・無加工（数値 parse しない・要件 8.2）。
            key: AliasKey::new(key.clone()),
            // 値 `[id,...]` を順序付き u32 リストへ（角括弧剥がし＋カンマ分割・要件 8.3）。
            ids: parse_alias_ids(fields.get(1).map(String::as_str).unwrap_or("")),
        });
    }
}

/// alias 値フィールド `[id,id,...]` を順序付き `Vec<u32>` へ写す（要件 8.3）。
///
/// - 前後の `[` `]` を剥がす（無ければ全体をそのまま扱う寛容措置・要件 9.2）。
/// - 内側をカンマ分割し各要素を u32 化する。空要素・非数値は既定 0（要件 3.3・パニックしない）。
/// - 空（`[]` または空文字列）は空 `Vec` を返す（要件 9.2）。
fn parse_alias_ids(value: &str) -> Vec<u32> {
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
        .map(|s| s.trim().parse::<u32>().unwrap_or(0))
        .collect()
}
