<#
.SYNOPSIS
  kiro-watch: hands out two kinds of desk one holder at a time.
    merge desk      one per repo  (shared files such as roadmap.md and ledger counts)
    load-test desk  one per machine (a quiet CPU); while it is held or queued, no merge starts
                    in any repo, and every other participant is asked to stop after its current
                    task is committed

.DESCRIPTION
  Only the kiro-watch coordinator session runs this script. It never sends messages itself:
  every command recomputes the plan and writes the messages to send into
  <worktree>/target/kiro-watch/outbox.json (UTF-8). The coordinator reads that file and sends
  each entry as written.

  Rules (developer, 2026-10-08):
    1  merge and load test: one holder per desk
    2  a load test needs every other participant stopped at "task done and committed"
    3  only sessions that asked to take part are coordinated
    4  load-test requests go before merge requests
    5  a session granted a merge is never asked to stop
  Merge queue order inside a repo: bug first, then request time. Load-test queue: request time.

  State: <worktree>/target/kiro-watch/state.json (written atomically). Terminal output is ASCII
  only; names and Japanese text go to the outbox / status files.

  Commands (Id is the session id from the cross-session message header, local_...):
    join     -Id -Name -Repo              take part (also implied by merge / loadtest)
    leave    -Id                          stop taking part (drops its queue entries)
    merge    -Id -Name -Repo -Spec [-Bug] ask for the merge desk of Repo
    merged   -Id [-Pr] [-Sha]             merge finished (releases the merge desk; Id leaves)
    loadtest -Id -Name -Repo -Purpose     ask for the load-test desk
    loaddone -Id                          load test finished (releases the load-test desk)
    stopped  -Id                          answer to a stop request
    cancel   -Id                          withdraw every request / desk of Id
    note     -Id -Text                    text appended to Id's next grant message
    status                                write status.md, print counts
    next                                  recompute only (e.g. after editing state by hand)

  Run: pwsh -NoProfile -File .claude/skills/kiro-watch/kiro-watch.ps1 <command> ...
#>
#Requires -Version 7
param(
    [Parameter(Mandatory, Position = 0)]
    [ValidateSet('join', 'leave', 'merge', 'merged', 'loadtest', 'loaddone', 'stopped', 'cancel', 'note', 'status', 'next')]
    [string]$Command,
    [string]$Id,
    [string]$Name,
    [string]$Repo,
    [string]$Spec,
    [switch]$Bug,
    [string]$Purpose,
    [string]$Pr,
    [string]$Sha,
    [string]$Text
)

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

$root = (& git rev-parse --show-toplevel 2>$null)
if (-not $root) { throw 'kiro-watch: run inside a git worktree' }
$dir = Join-Path $root 'target/kiro-watch'
$statePath = Join-Path $dir 'state.json'
$outboxPath = Join-Path $dir 'outbox.json'
$statusPath = Join-Path $dir 'status.md'
$msg = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'messages.json') -Raw -Encoding utf8 | ConvertFrom-Json -AsHashtable
$now = (Get-Date).ToString('yyyy-MM-ddTHH:mm:ssK')
$out = [System.Collections.Generic.List[object]]::new()

function New-State {
    [ordered]@{
        participants = [ordered]@{}   # id -> {id,name,repo,status,since}; status: working|stop-requested|stopped
        merge        = [ordered]@{}   # repo -> {holder, queue[], last}
        load         = [ordered]@{ holder = $null; queue = @() }
        notes        = [ordered]@{}   # id -> text
        idleNotified = $true
        log          = @()
    }
}

function Read-State {
    if (-not (Test-Path -LiteralPath $statePath)) { return New-State }
    $s = Get-Content -LiteralPath $statePath -Raw -Encoding utf8 | ConvertFrom-Json -AsHashtable
    if ($null -eq $s.load.queue) { $s.load.queue = @() }
    foreach ($r in @($s.merge.Keys)) { if ($null -eq $s.merge[$r].queue) { $s.merge[$r].queue = @() } }
    if ($null -eq $s.log) { $s.log = @() }
    return $s
}

function Write-State($s) {
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    $tmp = "$statePath.tmp"
    $s | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $tmp -Encoding utf8NoBOM
    Move-Item -LiteralPath $tmp -Destination $statePath -Force
}

function Format-Msg([string]$key, [hashtable]$vars) {
    $t = $msg[$key]
    foreach ($k in $vars.Keys) { $t = $t.Replace("{$k}", [string]$vars[$k]) }
    return $t
}

function Add-Out([string]$to, [string]$kind, [string]$body) {
    $name = if ($to -and $state.participants.Contains($to)) { $state.participants[$to].name } else { '' }
    $text = if ($kind -eq 'developer') { $body } else { $msg.prefix + $body }
    $out.Add([ordered]@{ to = $to; name = $name; kind = $kind; text = $text })
}

function Add-Log([string]$line) { $state.log = @($state.log) + "$now $line" }

function Need([string[]]$names) {
    foreach ($n in $names) {
        if (-not (Get-Variable -Name $n -ValueOnly)) { throw "kiro-watch: $Command needs -$n" }
    }
}

function Join-Participant([string]$id, [string]$name, [string]$repo) {
    if ($state.participants.Contains($id)) {
        $p = $state.participants[$id]
        if ($name) { $p.name = $name }
        if ($repo) { $p.repo = $repo }
        return
    }
    $state.participants[$id] = [ordered]@{ id = $id; name = $name; repo = $repo; status = 'working'; since = $now }
    Add-Log "join $id"
}

function Get-RepoDesk([string]$repo) {
    if (-not $state.merge.Contains($repo)) {
        $state.merge[$repo] = [ordered]@{ holder = $null; queue = @(); last = $null }
    }
    return $state.merge[$repo]
}

function Test-InMergeQueue([string]$id) {
    foreach ($r in $state.merge.Keys) {
        foreach ($q in @($state.merge[$r].queue)) { if ($q.id -eq $id) { return $true } }
    }
    return $false
}

function Test-MergeHolder([string]$id) {
    foreach ($r in $state.merge.Keys) {
        $h = $state.merge[$r].holder
        if ($h -and $h.id -eq $id) { return $true }
    }
    return $false
}

function Test-AnyMergeHeld {
    foreach ($r in $state.merge.Keys) { if ($state.merge[$r].holder) { return $true } }
    return $false
}

function Remove-From([string]$id) {
    foreach ($r in $state.merge.Keys) {
        $d = $state.merge[$r]
        $d.queue = @(@($d.queue) | Where-Object { $_.id -ne $id })
        if ($d.holder -and $d.holder.id -eq $id) { $d.holder = $null }
    }
    $state.load.queue = @(@($state.load.queue) | Where-Object { $_.id -ne $id })
    if ($state.load.holder -and $state.load.holder.id -eq $id) { $state.load.holder = $null }
}

function Get-NoteSuffix([string]$id) {
    if (-not $state.notes.Contains($id)) { return '' }
    $t = $msg.noteHeader + $state.notes[$id]
    $state.notes.Remove($id)
    return $t
}

function Format-Last($last) {
    if (-not $last) { return $msg.mergeLastNone }
    $parts = @()
    if ($last.pr) { $parts += "PR#$($last.pr)" }
    if ($last.sha) { $parts += "main $($last.sha)" }
    if ($last.spec) { $parts += $last.spec }
    return ($parts -join ' / ')
}

# Recompute the plan: stop requests, grants, resumes. Idempotent; only state changes emit messages.
function Invoke-Plan {
    $load = $state.load
    $loadWanted = $load.holder -or (@($load.queue).Count -gt 0)

    if ($loadWanted) {
        $candidate = if ($load.holder) { $load.holder.id } else { @($load.queue)[0].id }
        # who must be stopped: every participant except the candidate, merge holders (rule 5)
        # and sessions waiting in a merge queue (already idle by protocol)
        $needStop = @($state.participants.Keys | Where-Object {
                $_ -ne $candidate -and -not (Test-MergeHolder $_) -and -not (Test-InMergeQueue $_)
            })
        $holderName = $state.participants[$candidate].name
        $purpose = if ($load.holder) { $load.holder.purpose } else { @($load.queue)[0].purpose }
        foreach ($id in $needStop) {
            $p = $state.participants[$id]
            if ($p.status -eq 'working') {
                $p.status = 'stop-requested'; $p.since = $now
                Add-Out $id 'stop' (Format-Msg 'stopRequest' @{ holder = $holderName; purpose = $purpose })
                Add-Log "stop-request $id"
            }
        }
        if (-not $load.holder -and -not (Test-AnyMergeHeld)) {
            $pending = @($needStop | Where-Object { $state.participants[$_].status -ne 'stopped' })
            if ($pending.Count -eq 0) {
                $req = @($load.queue)[0]
                $load.queue = @(@($load.queue) | Select-Object -Skip 1)
                $load.holder = [ordered]@{ id = $req.id; purpose = $req.purpose; requested = $req.requested; granted = $now }
                $state.participants[$req.id].status = 'working'
                $names = @($needStop | ForEach-Object { $state.participants[$_].name })
                $stoppedText = if ($names.Count) { $names -join '、' } else { $msg.loadGrantNobody }
                Add-Out $req.id 'grant-load' (Format-Msg 'loadGrant' @{ stopped = $stoppedText; note = (Get-NoteSuffix $req.id) })
                Add-Log "grant-load $($req.id)"
            }
        }
        $state.idleNotified = $false
        return
    }

    # no load test wanted: release everyone still stopped or asked to stop
    foreach ($id in @($state.participants.Keys)) {
        $p = $state.participants[$id]
        if ($p.status -eq 'stopped') {
            $p.status = 'working'; $p.since = $now
            Add-Out $id 'resume' $msg.resume
            Add-Log "resume $id"
        }
        elseif ($p.status -eq 'stop-requested') {
            $p.status = 'working'; $p.since = $now
            Add-Out $id 'resume' $msg.resumeCancel
            Add-Log "resume-cancel $id"
        }
    }

    # merge desks: one holder per repo; bug first, then request time
    $busy = $false
    foreach ($repo in @($state.merge.Keys)) {
        $d = $state.merge[$repo]
        if (-not $d.holder -and @($d.queue).Count -gt 0) {
            $sorted = @(@($d.queue) | Sort-Object @{ Expression = { if ($_.bug) { 0 } else { 1 } } }, @{ Expression = { $_.requested } })
            $req = $sorted[0]
            $d.queue = @($sorted | Select-Object -Skip 1)
            $d.holder = [ordered]@{ id = $req.id; spec = $req.spec; bug = $req.bug; requested = $req.requested; granted = $now }
            Add-Out $req.id 'grant-merge' (Format-Msg 'mergeGrant' @{ repo = $repo; last = (Format-Last $d.last); note = (Get-NoteSuffix $req.id) })
            Add-Log "grant-merge $repo $($req.id)"
        }
        if ($d.holder -or @($d.queue).Count -gt 0) { $busy = $true }
    }

    if ($busy) { $state.idleNotified = $false }
    elseif (-not $state.idleNotified) {
        $lasts = @($state.merge.Keys | ForEach-Object { "$_ " + (Format-Last $state.merge[$_].last) })
        $lastText = if ($lasts.Count) { $lasts -join ' ; ' } else { $msg.mergeLastNone }
        Add-Out '' 'developer' (Format-Msg 'developerIdle' @{ last = $lastText })
        $state.idleNotified = $true
    }
}

function Write-Status {
    $lines = @("# kiro-watch status ($now)", '', '## participants', '', '| id | name | repo | status | since |', '|---|---|---|---|---|')
    foreach ($id in $state.participants.Keys) {
        $p = $state.participants[$id]
        $lines += "| $id | $($p.name) | $($p.repo) | $($p.status) | $($p.since) |"
    }
    $lines += @('', '## load-test desk', '')
    $h = $state.load.holder
    $lines += if ($h) { "- holder: $($h.id) ($($state.participants[$h.id].name)) $($h.purpose) since $($h.granted)" } else { '- holder: (none)' }
    foreach ($q in @($state.load.queue)) { $lines += "- queued: $($q.id) ($($state.participants[$q.id].name)) $($q.purpose) at $($q.requested)" }
    foreach ($repo in $state.merge.Keys) {
        $d = $state.merge[$repo]
        $lines += @('', "## merge desk: $repo", '', "- last: $(Format-Last $d.last)")
        $lines += if ($d.holder) { "- holder: $($d.holder.id) ($($state.participants[$d.holder.id].name)) $($d.holder.spec) since $($d.holder.granted)" } else { '- holder: (none)' }
        foreach ($q in @($d.queue)) { $lines += "- queued: $($q.id) $($q.spec) bug=$($q.bug) at $($q.requested)" }
    }
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    $lines -join "`n" | Set-Content -LiteralPath $statusPath -Encoding utf8NoBOM
}

$state = Read-State
$ack = $null

switch ($Command) {
    'join' {
        Need 'Id', 'Repo'
        Join-Participant $Id $Name $Repo
    }
    'leave' {
        Need 'Id'
        Remove-From $Id
        if ($state.participants.Contains($Id)) { $state.participants.Remove($Id) }
        Add-Log "leave $Id"
    }
    'merge' {
        Need 'Id', 'Repo', 'Spec'
        Join-Participant $Id $Name $Repo
        $d = Get-RepoDesk $Repo
        $dup = (Test-InMergeQueue $Id) -or ($d.holder -and $d.holder.id -eq $Id)
        if (-not $dup) {
            $d.queue = @(@($d.queue) + [ordered]@{ id = $Id; spec = $Spec; bug = [bool]$Bug; requested = $now })
            Add-Log "merge-request $Repo $Id $Spec bug=$([bool]$Bug)"
        }
        $ack = 'merge'
    }
    'merged' {
        Need 'Id'
        $found = $false
        foreach ($repo in $state.merge.Keys) {
            $d = $state.merge[$repo]
            if ($d.holder -and $d.holder.id -eq $Id) {
                $d.last = [ordered]@{ pr = $Pr; sha = $Sha; spec = $d.holder.spec; at = $now }
                $d.holder = $null
                $found = $true
                Add-Log "merged $repo $Id pr=$Pr sha=$Sha"
            }
        }
        if (-not $found) { Write-Host "warn: $Id holds no merge desk" }
        # a completed spec ends the session's participation (it rejoins from /kiro-impl if it runs again)
        Remove-From $Id
        if ($state.participants.Contains($Id)) { $state.participants.Remove($Id); Add-Log "leave $Id (merged)" }
    }
    'loadtest' {
        Need 'Id', 'Repo', 'Purpose'
        Join-Participant $Id $Name $Repo
        $dup = (@($state.load.queue) | Where-Object { $_.id -eq $Id }) -or ($state.load.holder -and $state.load.holder.id -eq $Id)
        if (-not $dup) {
            $state.load.queue = @(@($state.load.queue) + [ordered]@{ id = $Id; purpose = $Purpose; requested = $now })
            Add-Log "load-request $Id"
        }
        # a participant asking for a load test is not stopped for its own test
        $state.participants[$Id].status = 'working'
        $ack = 'load'
    }
    'loaddone' {
        Need 'Id'
        if ($state.load.holder -and $state.load.holder.id -eq $Id) {
            $state.load.holder = $null
            Add-Log "load-done $Id"
        }
        else { Write-Host "warn: $Id holds no load-test desk" }
    }
    'stopped' {
        Need 'Id'
        if ($state.participants.Contains($Id) -and $state.participants[$Id].status -eq 'stop-requested') {
            $state.participants[$Id].status = 'stopped'; $state.participants[$Id].since = $now
            Add-Log "stopped $Id"
        }
        else { Write-Host "warn: $Id was not asked to stop" }
    }
    'cancel' {
        Need 'Id'
        Remove-From $Id
        Add-Log "cancel $Id"
    }
    'note' {
        Need 'Id', 'Text'
        $state.notes[$Id] = $Text
    }
    'status' { }
    'next' { }
}

if ($Command -ne 'status') { Invoke-Plan }

# acknowledge a request only when the plan did not grant it at once
if ($ack -and -not ($out | Where-Object { $_.to -eq $Id -and $_.kind -like 'grant-*' })) {
    if ($ack -eq 'merge') {
        $d = $state.merge[$Repo]
        $sorted = @(@($d.queue) | Sort-Object @{ Expression = { if ($_.bug) { 0 } else { 1 } } }, @{ Expression = { $_.requested } })
        $pos = 1 + [array]::IndexOf(@($sorted | ForEach-Object { $_.id }), $Id) + $(if ($d.holder) { 1 } else { 0 })
        $loadNote = if ($state.load.holder -or @($state.load.queue).Count) { $msg.loadQueuedNote } else { '' }
        Add-Out $Id 'ack' (Format-Msg 'mergeQueued' @{ repo = $Repo; spec = $Spec; pos = $pos; loadNote = $loadNote })
    }
    else {
        $pos = 1 + [array]::IndexOf(@(@($state.load.queue) | ForEach-Object { $_.id }), $Id) + $(if ($state.load.holder) { 1 } else { 0 })
        Add-Out $Id 'ack' (Format-Msg 'loadQueued' @{ purpose = $Purpose; pos = $pos })
    }
}
Write-State $state
Write-Status
New-Item -ItemType Directory -Force -Path $dir | Out-Null
ConvertTo-Json -InputObject @($out) -Depth 5 | Set-Content -LiteralPath $outboxPath -Encoding utf8NoBOM

$merging = @($state.merge.Keys | Where-Object { $state.merge[$_].holder }).Count
$mergeQueued = 0; foreach ($r in $state.merge.Keys) { $mergeQueued += @($state.merge[$r].queue).Count }
$stopping = @($state.participants.Values | Where-Object { $_.status -ne 'working' }).Count
Write-Host ("ok {0}: messages={1} participants={2} load-holder={3} load-queued={4} merging={5} merge-queued={6} not-working={7}" -f `
        $Command, $out.Count, $state.participants.Count, [bool]$state.load.holder, @($state.load.queue).Count, $merging, $mergeQueued, $stopping)
Write-Host "outbox: $outboxPath"
Write-Host "status: $statusPath"
