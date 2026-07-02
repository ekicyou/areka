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
//!   `Shell.aliases` を充填する。
//! - subset 外・不正の寛容吸収の最終化 → タスク 4.6。
//! 現段階ではこれらのシーム関数は枠のみ（空を返す/何もしない）に留める。

use super::lexer::Token;
use super::model::{
    Animation, Collision, CollisionName, Element, ElementPath, Interval, Pattern, Shell, Surface,
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
        // alias 写像はタスク 4.5 の領分。現段階では吸収のみ（何も積まない）。
        decode_alias_block(shell, header);
        return;
    }
    if head.starts_with("surface.append") {
        // append 追記定義はタスク 4.4 の領分。現段階では吸収のみ（何も積まない）。
        decode_append_block(shell, header);
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
    let mut collisions: Vec<Collision> = Vec::new();
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

        // collision 矩形行（要件 6.1/6.2）。ukadoc 順序 始点X/始点Y/終点X/終点Y。
        if let Some(rest) = key.strip_prefix("collision") {
            // `collisionex`（要件 6 のタスク 4.6 寛容吸収）は扱わない。純 collisionN のみ。
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
            continue;
        }

        // animation 行（`animationN.*`）は `decode_animations` が別走査で集約するため、
        // このループでは扱わない。その他 subset 外の行はタスク 4.6 のシームゆえ読み飛ばす
        // （要件 9.2）。
    }

    // element はレイヤインデックス昇順・安定ソート（同レイヤは出現順維持・要件 4.4）。
    elements.sort_by_key(|e| e.layer);

    (elements, collisions, animations)
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

/// `surface.append*` 追記ブロックを `SurfaceAppend` へ decode する枠（タスク 4.4）。
///
/// タスク 4.1 では何も積まない（吸収のみ）。タスク 4.4 がヘッダのターゲット指定捕捉
/// （`parse_targets`・展開しない）と、本体の collision/animation 充填を実装し、
/// `shell.appends` へ push する。
fn decode_append_block(_shell: &mut Shell, _header: &[String]) {
    // タスク 4.4 で実装。現段階は寛容吸収（要件 9.2）。
}

/// `kero.surface.alias` ブロックを `SurfaceAlias` 群へ decode する枠（タスク 4.5）。
///
/// タスク 4.1 では何も積まない（吸収のみ）。タスク 4.5 が各 `KEY,[id,...]` 行を
/// opaque キー＋順序付き数値 ID リストへ写像し（重複キー保持）、`shell.aliases` へ
/// push する。
fn decode_alias_block(_shell: &mut Shell, _header: &[String]) {
    // タスク 4.5 で実装。現段階は寛容吸収（要件 9.2）。
}
