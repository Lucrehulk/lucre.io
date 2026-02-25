use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::atomic::AtomicI32;
use std::time::Instant;
use tokio::sync::mpsc;
use crate::entity_classes::Barrel;
use crate::constants::STAT_COUNT;

#[derive(Clone, Debug)]
pub struct BaseZone { pub x: f32, pub y: f32, pub w: f32, pub h: f32, pub team_id: u8, pub party_code: String }

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub text: String,
    pub duration: u128,
    pub created_at: Instant,
}

pub struct PlayerSession {
    pub client_id: usize, 
    pub sender: mpsc::UnboundedSender<Vec<u8>>, 
    pub is_connected: bool,
    pub disconnect_at: Option<Instant>, 
    pub ip: IpAddr, 
    pub saved_name: String, 
    pub saved_team: u8,
    pub entity_id: Option<usize>, 
    pub last_camera_pos: (f32, f32), 
    pub fov_factor: f32, 
    pub entity_created_at: Instant
}

#[derive(Clone, Debug)]
pub struct DeathCause { pub name: String, pub class_id: u8, pub color: u8 }

pub struct TurretState { 
    pub config_idx: usize, 
    pub initial_facing: f32, 
    pub facing: f32, 
    pub target_pos: Option<[f32; 2]>, 
    pub barrels: Vec<Barrel> 
}

pub struct GameEntity {
    pub facing: f32, pub barrels: Vec<Barrel>, pub shape: u8, pub team: u8, pub color: u8, pub class_id: u8, 
    pub name: String, pub score: u32, pub level: u8, pub stat_points: u8, pub stats: [u8; STAT_COUNT], pub render_score: bool,
    pub parent_id: usize, pub parent_barrel_index: usize, pub health: AtomicI32, pub max_health: i32,
    pub regen: i32, pub damage: i32, pub reload_ticks: usize, pub fov_factor: f32, pub spin_rate: f32,      
    pub orbit_radius: f32, pub creation_tick: usize, pub lifetime: usize, pub entity_type: u8, 
    pub dead: bool, pub is_bot: bool, pub is_firing: bool, pub is_override: bool, pub auto_fire: bool, 
    pub auto_spin_enabled: bool, pub should_render_health: bool, pub current_path: Vec<[f32; 2]>,
    pub last_path_calc: usize, pub invulnerable: bool, pub target_pos: [f32; 2], pub client_id: Option<usize>,
    pub death_cause: Option<DeathCause>, pub turrets: Vec<TurretState>, pub layer_facings: Vec<f32>,
    pub messages: Vec<ChatMessage>,
}

impl GameEntity {
    pub fn new(
        facing: f32, barrels: Vec<Barrel>, shape: u8, team: u8, color: u8, class_id: u8,
        name: String, score: u32, render_score: bool, parent_id: usize, parent_barrel_index: usize,
        health: i32, max_health: i32, regen: i32, damage: i32, reload_ticks: usize, fov_factor: f32,
        spin_rate: f32, orbit_radius: f32, creation_tick: usize, lifetime: usize, entity_type: u8,
        is_bot: bool, should_render_health: bool, invulnerable: bool, client_id: Option<usize>,
    ) -> Self {
        Self { 
            facing, barrels, shape, team, color, class_id, name, score,
            level: 45, stat_points: 0, stats: [0; STAT_COUNT], render_score, parent_id,
            parent_barrel_index, health: AtomicI32::new(health), max_health, regen, damage, reload_ticks,
            fov_factor, spin_rate, orbit_radius, creation_tick, lifetime, entity_type,
            dead: false, is_bot: is_bot, is_firing: false, is_override: false, auto_fire: false, 
            auto_spin_enabled: false, should_render_health, current_path: Vec::new(),
            last_path_calc: 0, invulnerable, target_pos: [0.0, 0.0], client_id,
            death_cause: None, turrets: Vec::new(), layer_facings: Vec::new(),
            messages: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameEvent {
    None,
    BossWarning { start_tick: usize }, 
    BossFight { boss_id: usize },
    EggWarning { start_tick: usize },
    EggHunt { egg_id: usize, end_tick: usize },
    EggReward { boss_id: usize },
    KOTHWarning { start_tick: usize },
    KOTHActive { end_tick: usize, zone_x: f32, zone_y: f32, zone_w: f32, zone_h: f32 },
    CelestialBattleWarning { start_tick: usize },
    CelestialBattleFight { celestial_ids: Vec<usize> },
    CelestialBattleReward { start_tick: usize, winning_team: u8, surviving_id: usize },
    BountyWarning { start_tick: usize },
    BountyClaiming { claim_id: usize, end_tick: usize },
    BountyHunted { target_id: usize },
    PaintJobWarning { start_tick: usize },
    PaintJobActive { end_tick: usize },
    MeteorShowerWarning { start_tick: usize },
    MeteorShowerActive { end_tick: usize, next_spawn_tick: usize, meteor_ids: Vec<usize> },
}

pub enum ServerCommand { 
    Connect(mpsc::UnboundedSender<Vec<u8>>, mpsc::UnboundedSender<usize>, String, IpAddr), 
    Disconnect(usize), 
    Input(usize, Vec<u8>) 
}
