$ErrorActionPreference = 'Stop'
$path = Join-Path $PSScriptRoot 'src/backend/handshake_core/src/user_manual/store.rs'
$text = [System.IO.File]::ReadAllText($path)

$legacyImpl = '(?s)#\[cfg\(any\(\)\)\]\r?\nimpl<''a> UserManualStore<''a> \{.*?(?=// ---------------------------------------------------------------------------\r?\n// SurrealDB implementation\.)'
$next = [regex]::Replace($text, $legacyImpl, '', 1)
if ($next -eq $text) {
    throw 'legacy UserManualStore implementation boundary not found'
}

$legacyMappers = '(?s)// ---------------------------------------------------------------------------\r?\n// Retained PostgreSQL row mapping \(non-compiled migration reference\)\.\r?\n// ---------------------------------------------------------------------------\r?\n.*?(?=/// Bounded excerpt centred)'
$clean = [regex]::Replace($next, $legacyMappers, '', 1)
if ($clean -eq $next) {
    throw 'legacy PostgreSQL mapper boundary not found'
}

[System.IO.File]::WriteAllText($path, $clean, [System.Text.UTF8Encoding]::new($false))
