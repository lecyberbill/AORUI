Architecture globale du système

┌─────────────────────────────────────────────────────────────┐
│                 CORE RUNTIME (Thread Tokio)                 │
│                                                             │
│   [ Agent FSM ] ◄──► [ Memory/Context ] ◄──► [ Tools Exec ] │
│         ▲                                                   │
│         │ (AgentActions / UIStatePatches)                   │
│         ▼                                                   │
│   ┌─────────────────────────────────────────────────────┐   │
│   │             CENTRAL DISPATCHER / BUS                │   │
│   │               (tokio::sync::mpsc)                   │   │
│   └─────────────────────────┬───────────────────────────┘   │
└─────────────────────────────┼───────────────────────────────┘
                              │ IPC / Channel crossbeam
┌─────────────────────────────┼───────────────────────────────┐
│                             ▼                               │
│   [ Declarative Tree ] ──► [ Taffy Layout ] (Layout Cache)  │
│                                     │                       │
│                                     ▼                       │
│              [ Instanced Quads & SDF Pipeline ]             │
│                                     │                       │
│                 RENDER THREAD (winit + wgpu)                │
└─────────────────────────────────────────────────────────────┘

1. Le pont État / Actions (Protocole typé)

Pour garantir la séparation stricte entre le moteur graphique et la boucle cognitive de l'agent :
Rust

use serde::{Deserialize, Serialize};

/// Événements remontés de la vue vers l'agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UiAction {
    UserPromptSubmitted(String),
    StepApproved(String),
    AgentInterrupted,
    ParameterAdjusted { key: String, value: f32 },
}

/// Mises à jour de l'UI projetées par l'agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentUiPatch {
    StatusChanged { state: String, glow_hue: [f32; 4] },
    StepLogged { step_id: String, tool: String, status: StepStatus },
    MetricUpdated { key: String, value: f32 },
    ScratchpadAppended(String),
    ModalRequested { title: String, content: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    Running,
    Success,
    Failed(String),
}

2. Le descripteur déclaratif Cyber/SDF

Chaque composant produit une spécification de boîte de layout (taffy::Style) et un jeu d'instances SDF injectées directement dans les buffers GPU.
Rust

use taffy::prelude::*;

pub struct GlassCardProps {
    pub title: String,
    pub glow_color: [f32; 4],
    pub glow_intensity: f32,
    pub corner_radius: f32,
}

pub struct AgentStatusBadge {
    pub current_state: String,
    pub pulse_frequency: f32,
    pub active_color: [f32; 4],
}

// Représentation mémoire bas niveau alignée pour le vertex buffer wgpu (bytemuck)
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuSdfInstance {
    pub bounds: [f32; 4],         // [x, y, width, height]
    pub bg_color: [f32; 4],       // [r, g, b, a] avec translucidité glass
    pub glow_color: [f32; 4],     // [r, g, b, a]
    pub radius: f32,
    pub border_width: f32,
    pub glow_intensity: f32,
    pub blur_factor: f32,         // Index pour échantillonnage de passe de flou
}

3. Pipeline de rendu multi-passes (Verre dépoli + SDF Bloom)

Le pipeline graphique s'exécute en 3 passes synchronisées sans jamais bloquer le worker de l'agent :

[ Pass 1 : Offscreen Framebuffer ]
    Rendu du fond dynamique (particules, grille cyber, widgets d'arrière-plan).
                │
                ▼
[ Pass 2 : Dual-Kawase Downsample ]
    Génération d'une texture floue à demi-résolution (1/2 ou 1/4 du viewport).
                │
                ▼
[ Pass 3 : SDF Composition & UI ]
    Évaluation des cartes glassmorphism par pixel :
    - Échantillonnage de la texture floue avec offset (réfraction)
    - Tracé de bordure nette
    - Décroissance exponentielle continue du halo néon (SDF outer glow)
    - Surimpression du texte vectoriel via Glyphon

4. La boucle de vie de l'application
Rust

pub struct AppContext {
    pub ui_to_agent: tokio::sync::mpsc::Sender<UiAction>,
    pub agent_to_ui: crossbeam_channel::Receiver<AgentUiPatch>,
}

pub struct AgentWindowApp {
    pub layout_engine: TaffyTree<()>,
    pub gpu_instances: Vec<GpuSdfInstance>,
    pub rx_patches: crossbeam_channel::Receiver<AgentUiPatch>,
    pub tx_actions: tokio::sync::mpsc::Sender<UiAction>,
}

impl AgentWindowApp {
    pub fn update_and_render(&mut self, renderer: &mut GpuRenderer) {
        // 1. Dépiler les patches émis par l'agent asynchrone (non-bloquant)
        while let Ok(patch) = self.rx_patches.try_recv() {
            self.apply_agent_patch(patch);
        }

        // 2. Résoudre le graphe Taffy Flexbox/Grid
        self.layout_engine.compute_layout(
            /* root */, 
            taffy::geometry::Size::MAX_CONTENT
        ).unwrap();

        // 3. Mettre à jour le GPU Instance Buffer
        self.rebuild_sdf_instances();

        // 4. Dispatch du draw call unique instancié
        renderer.draw_frame(&self.gpu_instances);
    }

    fn apply_agent_patch(&mut self, patch: AgentUiPatch) {
        match patch {
            AgentUiPatch::StatusChanged { glow_hue, .. } => {
                // Modifie instantanément les tokens du shader sans recompiler l'UI
                self.update_accent_glow(glow_hue);
            }
            AgentUiPatch::StepLogged { .. } => {
                // Insère un nouveau nœud dans la timeline Taffy
            }
            _ => {}
        }
    }
}

5. Arborescence du dépôt cible
Plaintext

cyber-agent-ui/
├── crates/
│   ├── ui-core/              # Types partagés, tokens de design, protocole d'actions
│   │   └── src/
│   ├── ui-gpu/               # Moteur wgpu, shaders WGSL, passe Dual-Kawase, SDF
│   │   ├── shaders/
│   │   │   ├── sdf_card.wgsl
│   │   │   └── blur_dual_kawase.wgsl
│   │   └── src/
│   ├── ui-layout/            # Intégration Taffy, gestionnaire d'arbres, hit-testing
│   │   └── src/
│   └── agent-runtime/        # Boucle cognitive Tokio, FSM d'états, registres d'outils
│       └── src/
└── examples/
    └── agent_dashboard.rs    # Démarrage fenêtre + spawn de l'agent

Prochaine étape d'exécution

Pour valider le pipeline d'un bout à l'autre sans surcharger le code, l'idéal est de brancher le flux minimal :

    Implémenter le shader WGSL SDF avec passage du tableau GpuSdfInstance.

    Connecter un faux agent asynchrone qui envoie un patch de couleur néon toutes les 2 secondes via le channel.

    Vérifier la fluidité du rendu et l'absence de saccades sur le thread d'affichage.