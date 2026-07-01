-- murasaki1000.lua - 5軸コード→surface1000さくらスクリプト変換モジュール
-- サーフェスチェッカーが生成する5軸コード(例: A0813)を
-- \s[1000] + \![bind,...] コマンド列に変換する

local log = require "@pasta_log"

--- @class murasaki1000
local M = {}

-- ===== 4軸マッピングテーブル =====

--- 腕軸: コード文字 → {カテゴリ, パーツ名}
local ARM = {
    A = { "腕", "伸び" }, -- bindgroup1100
    B = { "腕", "組み" }, -- bindgroup1101
}

--- 口軸: コード文字 → {カテゴリ, パーツ名}
local MOUTH = {
    ["1"] = { "口", "笑顔1" }, -- bindgroup1200
    ["2"] = { "口", "笑顔2" }, -- bindgroup1201
    ["3"] = { "口", "口開" }, -- bindgroup1202
    ["4"] = { "口", "うや？" }, -- bindgroup1203
    ["5"] = { "口", "うう‥" }, -- bindgroup1204
    ["6"] = { "口", "む？" }, -- bindgroup1205
    ["7"] = { "口", "‥‥" }, -- bindgroup1206
    ["8"] = { "口", "にこっ" }, -- bindgroup1207
    ["9"] = { "口", "小口" }, -- bindgroup1208
    A = { "口", "中口" }, -- bindgroup1209
    B = { "口", "うぇ～" }, -- bindgroup1210
    C = { "口", "ハァ～" }, -- bindgroup1211
}

--- 眉軸: コード文字 → {カテゴリ, パーツ名}
local BROW = {
    ["1"] = { "眉", "通常" }, -- bindgroup1500
    ["2"] = { "眉", "オコ" }, -- bindgroup1501
    ["3"] = { "眉", "悲しみ" }, -- bindgroup1502
    ["4"] = { "眉", "シュン" }, -- bindgroup1503
}

--- 目軸: コード文字 → {カテゴリ, パーツ名, まばたきパーツ名}
local EYE = {
    ["1"] = { "目", "べそ", "----" }, -- bindgroup1300
    ["2"] = { "目", "ジトー", "ジトー" }, -- bindgroup1301
    ["3"] = { "目", "通常", "通常" }, -- bindgroup1302
    ["4"] = { "目", "笑顔", "----" }, -- bindgroup1303
    ["5"] = { "目", "静観", "----" }, -- bindgroup1304
    ["6"] = { "目", "半目", "半目" }, -- bindgroup1305
}

-- ===== AXES 軸定義テーブル =====

--- @class AxisDef
--- @field pos integer 5軸コード内の文字位置（1-indexed）
--- @field name string 軸名（エラーメッセージ用）
--- @field map table<string, string[]>|nil コード文字→{カテゴリ,パーツ名}（目軸のみ第3要素にまばたきパーツ名）

local AXES = {
    { pos = 1, name = "腕", map = ARM },
    { pos = 2, name = "紅", map = nil }, -- 紅は特殊処理（"0"/"1" のみ）
    { pos = 3, name = "口", map = MOUTH },
    { pos = 4, name = "眉", map = BROW },
    { pos = 5, name = "目", map = EYE },
}

-- ===== 内部処理関数 =====

--- \s[...] ラッパーを除去して内部の5軸コードを返す
--- @param s string 入力文字列
--- @return string 裸の5軸コード
local function strip_wrapper(s)
    local inner = string.match(s, "^\\s%[(.-)%]$")
    return inner or s
end

--- 5軸コードのバリデーション
--- @param code string 5文字の軸コード
--- @return string|nil エラーメッセージ（正常時は nil）
local function validate(code)
    if #code ~= 5 then
        return string.format(
            "murasaki1000.build: コード長が不正です (got %d, expected 5): '%s'",
            #code, code)
    end
    for _, axis in ipairs(AXES) do
        local char = code:sub(axis.pos, axis.pos)
        if axis.map == nil then
            -- 紅軸: "0" または "1" のみ有効
            if char ~= "0" and char ~= "1" then
                return string.format(
                    "murasaki1000.build: %s軸の値が不正です: '%s' (code: '%s')",
                    axis.name, char, code)
            end
        else
            -- 他4軸: テーブルキー存在確認
            if not axis.map[char] then
                return string.format(
                    "murasaki1000.build: %s軸の値が不正です: '%s' (code: '%s')",
                    axis.name, char, code)
            end
        end
    end
    return nil
end

--- 単一の5軸コードからさくらスクリプト文字列を組み立てる
--- @param code string バリデーション済みの5文字コード
--- @return string さくらスクリプト
local function build_one(code)
    local result = "\\s[1000]"

    for _, axis in ipairs(AXES) do
        local char = code:sub(axis.pos, axis.pos)
        if axis.map == nil then
            -- 紅軸: 0/1 を明示的に設定（前回状態が残るため）
            result = result .. string.format("\\![bind,紅,差し,%s]", char)
        else
            local entry = axis.map[char]
            result = result .. string.format("\\![bind,%s,%s,1]", entry[1], entry[2])
            local mabataki = entry[3]
            if mabataki then
                result = result .. string.format("\\![bind,まばたき,%s,1]", mabataki)
            end
        end
    end

    return result
end

-- ===== 公開 API =====

--- 5軸コードからsurface1000用さくらスクリプトを生成する。
--- 複数コードが渡された場合、ランダムに1つを選択する。
--- \s[XXXXX] 形式の入力も受け付ける（ラッパー自動除去）。
--- @param ... string 1個以上の5軸コード（例: "A0813"）または \s[...] 形式
--- @return string さくらスクリプト文字列（\s[1000] + \![bind,...] の連結）。エラー時は空文字列
function M.build(...)
    local n = select("#", ...)
    if n == 0 then
        log.error("murasaki1000.build: 引数が必要です")
        return ""
    end

    local args = { ... }

    -- 全引数に strip_wrapper を適用
    for i = 1, n do
        args[i] = strip_wrapper(args[i])
    end

    -- 全引数をバリデーション（Fail Fast）
    for i = 1, n do
        local err = validate(args[i])
        if err then
            log.error(err)
            return ""
        end
    end

    -- ランダムに1つ選択して変換
    local chosen = args[math.random(n)]
    log.debug({ fn = "M.build", n = n, chosen = chosen })
    return build_one(chosen)
end

return M
