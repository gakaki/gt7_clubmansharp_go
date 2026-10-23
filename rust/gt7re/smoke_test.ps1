# End-to-end smoke test for gt7re. Writes a pass/fail report.
$ErrorActionPreference = "Continue"
$root = "C:\gt7\gt7\_re"
$exe  = Join-Path $root "rust\gt7re\target\release\gt7re.exe"
$out  = Join-Path $root "rust\out\smoke"
New-Item -ItemType Directory -Force -Path $out | Out-Null

$results = @()
function Run-Case($name, $block) {
    Write-Host "== $name"
    try {
        & $block
        if ($LASTEXITCODE -ne 0) { throw "exit code $LASTEXITCODE" }
        $script:results += [pscustomobject]@{ Case = $name; Status = "PASS" }
    } catch {
        $script:results += [pscustomobject]@{ Case = $name; Status = "FAIL: $_" }
    }
}

Run-Case "resolve" { & $exe resolve "common/render.txs" "rtext/common/us.rt2" "nope/x.bin" }
Run-Case "stats" { & $exe stats }
Run-Case "rt2-info" { & $exe rt2 info (Join-Path $root "extracted\text\rtext\common\us.rt2") }
Run-Case "rt2-decode" { & $exe rt2 decode (Join-Path $root "extracted\text\rtext\common\us.rt2") --out $out --keystream (Join-Path $root "keys\rt2_rust\us.ks.bin") }
Run-Case "rt2-recover" { & $exe rt2 decode (Join-Path $root "extracted\text\rtext\common\gb.rt2") --out $out }
Run-Case "packed" { & $exe packed "C:\gt7\gt7\contents\6\X\PVEZ3" --out (Join-Path $out "packed.bin") }
Run-Case "udp-layout" { & $exe udp layout }
Run-Case "map4" { & $exe map4 --file "C:\gt7\gt7\contents\B\W\6HX61" --out (Join-Path $out "map4.json") }
Run-Case "rada" { & $exe rada --file "C:\gt7\gt7\contents\A\7\PK4HF" --out (Join-Path $out "rada.json") }
Run-Case "txs5-index" { & $exe txs5-index --out (Join-Path $out "txs5.json") }
Run-Case "idp0-extract" { & $exe idp0-extract --out (Join-Path $out "idp0") --limit 3 --ext .txs }
Run-Case "strings" { & $exe strings --kind "UI project (MPRJ)" --out (Join-Path $out "strings.json") --max 5 }
Run-Case "catalog" { & $exe catalog --out (Join-Path $out "catalog") }
Run-Case "extract-kind" { & $exe extract-kind --kind "sound (SNDZ)" --out (Join-Path $out "audio") --limit 3 }
Run-Case "mdl5-index" { & $exe mdl5-index --out (Join-Path $out "mdl5.json") }
Run-Case "link-all" { & $exe link-all --out (Join-Path $out "by_kind") }

$results | Format-Table -AutoSize
$md = @("# gt7re smoke test report", "", ("Time: " + (Get-Date -Format 'yyyy-MM-dd HH:mm:ss')), "", "| case | result |", "|---|---|")
foreach ($r in $results) { $md += "| $($r.Case) | $($r.Status) |" }
[IO.File]::WriteAllText((Join-Path $root "report\VERIFICATION.md"), ($md -join "`n"), [Text.UTF8Encoding]::new($false))
$fail = ($results | Where-Object { $_.Status -ne "PASS" }).Count
Write-Host "`nfailures: $fail / $($results.Count)"
