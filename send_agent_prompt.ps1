param(
    [string]$Prompt = "Purger les caches et actualiser les flux télémétriques"
)

$payload = @{ prompt = $Prompt } | ConvertTo-Json -Compress
$response = Invoke-RestMethod -Uri "http://127.0.0.1:8765/prompt" -Method Post -ContentType "application/json; charset=utf-8" -Body $payload
$response | ConvertTo-Json
