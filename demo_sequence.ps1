param(
    [int]$StepDelay = 4
)

function Send-AgentAction([string]$StepTitle, [string]$Prompt, [string]$VisualEffect) {
    Write-Host "`n------------------------------------------------------------" -ForegroundColor Cyan
    Write-Host ">> [DEMO AGENT] $StepTitle" -ForegroundColor Yellow
    Write-Host "   Prompt envoye : '$Prompt'" -ForegroundColor White
    Write-Host "   Effet visuel  : $VisualEffect" -ForegroundColor Green
    Write-Host "------------------------------------------------------------" -ForegroundColor Cyan

    $payload = @{ prompt = $Prompt } | ConvertTo-Json -Compress
    $null = Invoke-RestMethod -Uri "http://127.0.0.1:8765/prompt" -Method Post -ContentType "application/json; charset=utf-8" -Body $payload
    Start-Sleep -Seconds $StepDelay
}

Write-Host "`n============================================================" -ForegroundColor Magenta
Write-Host " DEMARRAGE DE LA DEMONSTRATION EN DIRECT DU COPILOT AETHEROS" -ForegroundColor Magenta
Write-Host "============================================================" -ForegroundColor Magenta

Send-AgentAction "ETAPE 1 / 5 : Diagnostic et Purge des Caches" "Purger les caches et actualiser les flux télémétriques" "La carte KPI Latence s'illumine en cyan, latence ramenee a 14.2ms."

Send-AgentAction "ETAPE 2 / 5 : Changement de Portee Temporelle (7 Jours)" "Ajuster la fenêtre télémétrique sur 7j" "Le segmented picker bascule sur 7j, la courbe GPU se recalcule."

Send-AgentAction "ETAPE 3 / 5 : Filtrage de la Table des Processus" "Filtrer les conteneurs et analyser: postgres" "La table des micro-services s'anime et isole les services postgres."

Send-AgentAction "ETAPE 4 / 5 : Incident et Demande d'Autorisation" "Inspecter et redémarrer le processus: ai-inference-engine" "Ouverture de la modale Bleu Nuit 'Confirmation requise' avec boutons de validation."

Send-AgentAction "ETAPE 5 / 5 : Retour au Mode Operationnel (1h)" "Ajuster la fenêtre télémétrique sur 1h" "Reinitialisation de la portee telemetrique sur 1h pour monitoring direct."

Write-Host "`n============================================================" -ForegroundColor Green
Write-Host " DEMONSTRATION TERMINEE AVEC SUCCES !" -ForegroundColor Green
Write-Host "============================================================`n" -ForegroundColor Green
