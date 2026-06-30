-- touch_detect.lua - ストローキング検知モジュール
-- OnMouseMove の連続イベントから「撫でる」動作を検知する

local log = require "@pasta_log"
local M = {}

-- 定数
local INITIAL_DELAY = 2    -- 初期遅延（秒）
local DEBOUNCE_TIMEOUT = 2 -- デバウンスタイムアウト（秒）

-- 内部状態
local state = "OFF"        -- "OFF" | "COUNTING" | "FIRED"
local current_target = nil -- "region:actor" 形式
local start_time = 0       -- 蓄積開始時刻
local last_time = 0        -- 最終受信時刻

--- ストローキング判定
--- @param actor string アクター番号（"0" or "1"）
--- @param region string 当たり判定名（"Head", "Bust" 等）
--- @return boolean 発火すべきなら true
function M.check(actor, region)
    local now = os.time()
    local target = region .. ":" .. actor

    -- check() 呼び出し状態ログ (Req 2.1)
    log.trace({ fn = "check", state = state, target = current_target, elapsed = now - start_time })

    -- ターゲット変更 or デバウンスタイムアウト → リセット
    if target ~= current_target or (now - last_time) > DEBOUNCE_TIMEOUT then
        state = "OFF"
        -- leave / リセット理由ログ (Req 2.4)
        local reason = (target ~= current_target) and "target_change" or "timeout"
        if current_target ~= nil then
            log.info({ fn = "check", action = "leave", reason = reason, target = current_target })
        end
        current_target = target
        start_time = now
    end

    last_time = now

    if state == "OFF" then
        state = "COUNTING"
        -- OFF→COUNTING 遷移ログ (Req 2.2)
        log.info({ fn = "check", transition = "OFF->COUNTING", target = target })
        start_time = now
        return false
    elseif state == "COUNTING" then
        if (now - start_time) >= INITIAL_DELAY then
            state = "FIRED"
            -- COUNTING→FIRED 遷移ログ (Req 2.3)
            log.info({ fn = "check", transition = "COUNTING->FIRED", target = target, elapsed = now - start_time })
            return true
        end
        return false
    elseif state == "FIRED" then
        -- 発火済み → 次のリセットまで無視
        return false
    end

    return false
end

--- 状態リセット（テスト用）
function M.reset()
    state = "OFF"
    current_target = nil
    start_time = 0
    last_time = 0
end

return M
