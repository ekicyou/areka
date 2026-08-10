use super::emo2;

// ---------------------------------------------------------------------------
// Balloon anchor offset（R5.4・design「バルーン正典整理」）
// ---------------------------------------------------------------------------

/// バルーン窓の左上（物理 px・スクリーン座標）を算出する（R5.4）。
///
/// 基準は shell descript の正典整列: X「バルーンの右端がサーフェス左端に揃う位置」＋ Y「バルーン
/// 上端＝サーフェス上端」。`sakura.balloon.offsetx/offsety` があればこの基準からの調整として加算し、
/// 無指定なら基準そのもの（既定整列＝バルーン右端＝シェル左端・上端揃え）を返す。マジックギャップは
/// 用いず、シェル位置とバルーン幅から計算する。
///
/// - `shell_x`/`shell_y`: シェル窓左上（物理 px）。
/// - `balloon_w`: バルーン surface 原寸幅（物理 px）。
pub(super) fn compute_balloon_pos(shell_x: i32, shell_y: i32, balloon_w: u32) -> (i32, i32) {
    // 既定基準: バルーン右端 = シェル左端 → 左上 x = シェル左端 − バルーン幅。上端揃え → y = シェル上端。
    let base_x = shell_x - balloon_w as i32;
    let base_y = shell_y;

    match read_balloon_offset() {
        Some((ox, oy)) => {
            tracing::info!(
                offsetx = ox,
                offsety = oy,
                "emo-present: descript の sakura.balloon.offsetx/offsety を既定基準へ適用"
            );
            (base_x + ox, base_y + oy)
        }
        None => {
            tracing::info!(
                base_x,
                base_y,
                "emo-present: balloon offset 無指定 — 既定整列（右端＝シェル左端・上端揃え）で配置"
            );
            (base_x, base_y)
        }
    }
}

/// shell descript（`shell/master/descript.txt`）から `sakura.balloon.offsetx/offsety` を読む。
///
/// 読取失敗・両キー欠如・整数化不能はいずれも `None`（既定整列へフォールバック）で返す（log-first・
/// panic しない）。emo2 fixture は両キーとも持たないため通常は `None` が返る（既定整列が走る）。
/// 部分指定（片方のみ）は仕様外ゆえ安全側で `None` とする。
fn read_balloon_offset() -> Option<(i32, i32)> {
    let path = emo2("shell/master/descript.txt");
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "emo-present: descript.txt の読取に失敗 — 既定整列へフォールバック"
            );
            return None;
        }
    };

    let kv = areka_parsers::kv::parse_kv(&text);
    let ox = kv
        .get("sakura.balloon.offsetx")
        .and_then(|s| s.parse::<i32>().ok());
    let oy = kv
        .get("sakura.balloon.offsety")
        .and_then(|s| s.parse::<i32>().ok());

    match (ox, oy) {
        (Some(x), Some(y)) => Some((x, y)),
        _ => None,
    }
}
