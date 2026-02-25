use std::collections::{HashMap, HashSet};
use std::f32::consts::{FRAC_PI_2, PI};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BulletType { Standard, Trap, Drone }

#[derive(Clone, Debug)]
pub struct Barrel {
    pub vertices: Vec<[f32; 2]>, pub angle_offset: f32, pub offset_forward: f32, pub offset_side: f32,    
    pub delay_fraction: f32, pub last_fired: usize, pub bullet_lifetime_mult: f32, pub recoil: f32,            
    pub damage_mult: f32, pub bullet_speed_mult: f32, pub spread: f32, pub original_vertices: Vec<[f32; 2]>,
    pub visual_recoil: f32, pub bullet_type: BulletType, pub projectile_shape: u8, pub can_shoot: bool,
    pub max_children: usize, pub child_ids: HashSet<usize>, pub fire_lock: bool, pub layer: u8
}

#[derive(Clone, Debug)]
pub struct TurretConfig {
    pub class_id: u8, pub offset_forward: f32, pub offset_side: f32, pub angle_offset: f32,  
    pub size: f32, pub stat_multiplier: f32, pub override_target: bool, pub render_layer: bool,
    pub layer: u8
}

#[derive(Clone, Debug)]
pub struct LayerConfig {
    pub shape: u8,
    pub color: u8, 
    pub scale: f32,
    pub spin_speed: f32,
    pub spin_direction: bool,
}

#[derive(Clone)]
pub struct EntityClass {
    pub id: u8, pub name: String, pub mass: f32, pub barrels: Vec<Barrel>, pub shape: u8, pub body_radius: f32, pub base_speed: f32,
    pub base_health: i32, pub base_damage: i32, pub base_regen: i32, pub base_reload: usize, pub fov_factor: f32, 
    pub upgrades: Vec<u8>, pub auto_spin: f32, pub color: u8, pub score: u32, pub turrets: Vec<TurretConfig>, 
    pub layers: Vec<LayerConfig>
}

pub fn make_barrel(vertices: Vec<[f32; 2]>, angle_offset: f32, offset_forward: f32, offset_side: f32, delay_fraction: f32, last_fired: usize, bullet_lifetime_mult: f32, recoil: f32, damage_mult: f32, bullet_speed_mult: f32, spread: f32, bullet_type: BulletType, projectile_shape: u8, can_shoot: bool, max_children: usize, fire_lock: bool, layer: u8) -> Barrel {
    Barrel { original_vertices: vertices.clone(), vertices, angle_offset, offset_forward, offset_side, delay_fraction, last_fired, bullet_lifetime_mult, recoil, damage_mult, bullet_speed_mult, spread, visual_recoil: 0.0, bullet_type, projectile_shape, can_shoot, max_children, child_ids: HashSet::new(), fire_lock, layer }
}
pub fn make_rect_barrel(width: f32, length: f32) -> Vec<[f32; 2]> { let hw = width * 0.5; vec![[0.0, -hw], [length, -hw], [length, hw], [0.0, hw]] }
pub fn make_trap_barrel(base_width: f32, tip_width: f32, length: f32) -> Vec<[f32; 2]> { let hbw = base_width * 0.5; let htw = tip_width * 0.5; vec![[0.0, -hbw], [length, -htw], [length, htw], [0.0, hbw]] }

pub fn init_tank_classes() -> HashMap<u8, EntityClass> {
    let b_hp = 3000;
    let b_dmg = 20;
    let b_spd = 0.225;
    let b_reload = 50;
    let b_regen = 5;

    let r = 0.85;
    let celestial_r = 0.85;
    
    let make_celestial = |id: u8, name: &str, color: u8, main_turret_id: u8, pentagon_turret_id: u8, multipliers: [f32; 2]| -> EntityClass {
        let mut turrets = Vec::new();
        let angle_step = (PI * 2.0) / 9.0;
        for i in 0..9 {
            let angle = i as f32 * angle_step + 0.174533;
            let offset_forward = f32::cos(angle) * celestial_r; 
            let offset_side = f32::sin(angle) * celestial_r;
            turrets.push(TurretConfig { class_id: 178, offset_forward, offset_side, angle_offset: angle, size: 0.25, stat_multiplier: 1.0, render_layer: false, override_target: true, layer: 0 });
        }
        let angle_step_second = (PI * 2.0) / 7.0;
        for i in 0..7 {
            let angle = i as f32 * angle_step_second + 0.6732;
            let offset_forward = f32::cos(angle) * celestial_r; 
            let offset_side = f32::sin(angle) * celestial_r;
            turrets.push(TurretConfig { class_id: main_turret_id, offset_forward, offset_side, angle_offset: angle, size: 0.25, stat_multiplier: multipliers[0], render_layer: false, override_target: false, layer: 1 });
        }
        let angle_step_third = (PI * 2.0) / 5.0;
        for i in 0..5 {
            let angle = i as f32 * angle_step_third + 0.3142;
            let offset_forward = f32::cos(angle) * celestial_r; 
            let offset_side = f32::sin(angle) * celestial_r;
            turrets.push(TurretConfig { class_id: pentagon_turret_id, offset_forward, offset_side, angle_offset: angle, size: 0.2, stat_multiplier: multipliers[1], render_layer: false, override_target: false, layer: 2 });
        }
        let layers = vec![
            LayerConfig { shape: 7, color: 255, scale: 0.7, spin_speed: 0.015, spin_direction: false }, 
            LayerConfig { shape: 5, color: 255, scale: 0.4, spin_speed: 0.007, spin_direction: true },
        ];
        EntityClass {
            id, name: name.to_string(), mass: 100.0, shape: 9, body_radius: 40.0, base_speed: 0.04,
            base_health: 150000, base_damage: 1500, base_regen: 500, base_reload: 60,
            fov_factor: 5.5, auto_spin: 0.015, color, score: 1000000,
            upgrades: vec![210, 211, 212, 213, 214, 215],
            barrels: vec![],
            turrets,
            layers,
        }
    };

    let mut octo_trapper_barrels = Vec::new();
    for i in 0..8 {
        let angle = (i as f32) * (PI / 4.0);
        octo_trapper_barrels.push(make_barrel(make_rect_barrel(0.42, 1.2), angle, 0.0, 0.0, 0.0, 0, 1.0, 0.0, 1.0, 1.0, 0.0, BulletType::Standard, 0, false, 0, false, 0));
        let offset_forwrd = f32::cos(angle) * 1.2; 
        let offset_sde = f32::sin(angle) * 1.2;
        octo_trapper_barrels.push(make_barrel(make_trap_barrel(0.42, 1.5, 0.42), angle, offset_forwrd, offset_sde, 0.0, 0, 1.5, 0.2, 1.0, 4.0, 0.0, BulletType::Trap, 131, true, 0, false, 0));
    }

    let classes = vec![
        EntityClass { 
            id: 0, name: "Basic".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: b_hp, base_damage: b_dmg, base_regen: b_regen, base_reload: b_reload, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![1, 2, 3, 4, 99],
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.8, 1.9), 0.0, 0.0, 0.0, 0.0, 0, 1.0, 0.2, 1.0, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0) 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 1, name: "Twin".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: b_hp, base_damage: b_dmg, base_regen: b_regen, base_reload: b_reload, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![5, 9, 6],
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.7, 2.0), 0.0, 0.0, -0.5, 0.0, 0, 1.0, 0.15, 0.65, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 2.0), 0.0, 0.0, 0.5, 0.0, 0, 1.0, 0.15, 0.65, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 2, name: "Sniper".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd * 0.9, 
            base_health: (b_hp as f32 * 1.2) as i32, base_damage: (b_dmg as f32 * 1.2) as i32, base_regen: b_regen, base_reload: (b_reload as f32 * 1.5) as usize, fov_factor: 1.4, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![7, 17, 30, 15],
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.9, 2.5), 0.0, 0.0, 0.0, 0.0, 0, 1.5, 0.5, 2.5, 1.5, 0.0, BulletType::Standard, 0, true, 0, false, 0) 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 3, name: "Machine Gun".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: b_hp, base_damage: b_dmg, base_regen: b_regen, base_reload: b_reload, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![8, 25, 19],
            barrels: vec![ 
                make_barrel(make_trap_barrel(0.8, 1.4, 1.8), 0.0, 0.0, 0.0, 0.0, 0, 0.7, 0.1, 0.7, 1.0, 0.4, BulletType::Standard, 0, true, 0, false, 0) 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 4, name: "Flank Guard".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: b_hp, base_damage: b_dmg, base_regen: b_regen, base_reload: b_reload, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![11, 9, 6, 23],
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.8, 1.9), 0.0, 0.0, 0.0, 0.0, 0, 1.0, 0.2, 1.0, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.8, 1.6), PI, 0.0, 0.0, 0.0, 0, 1.0, 0.2, 1.0, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 5, name: "Triple Shot".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: b_hp, base_damage: b_dmg, base_regen: b_regen, base_reload: b_reload, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![20, 21],
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.7, 2.0), 0.0, 0.0, 0.0, 0.0, 0, 1.0, 0.15, 0.7, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 1.9), -0.785, 0.0, 0.0, 0.0, 0, 1.0, 0.15, 0.7, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 1.9), 0.785, 0.0, 0.0, 0.0, 0, 1.0, 0.15, 0.7, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 9, name: "Quad Tank".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: b_hp, base_damage: b_dmg, base_regen: b_regen, base_reload: b_reload, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![27],
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.8, 1.9), 0.0, 0.0, 0.0, 0.0, 0, 1.0, 0.2, 1.0, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.8, 1.9), 1.5708, 0.0, 0.0, 0.0, 0, 1.0, 0.2, 1.0, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.8, 1.9), 3.14159, 0.0, 0.0, 0.0, 0, 1.0, 0.2, 1.0, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.8, 1.9), 4.71239, 0.0, 0.0, 0.0, 0, 1.0, 0.2, 1.0, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 6, name: "Double Twin".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: (b_spd * 1.1), 
            base_health: (b_hp as f32 * 1.1) as i32, base_damage: b_dmg, base_regen: b_regen, base_reload: b_reload, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![217],
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.7, 2.0), 0.0, 0.0, -0.5, 0.0, 0, 1.0, 0.15, 0.65, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 2.0), 0.0, 0.0, 0.5, 0.0, 0, 1.0, 0.15, 0.65, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 2.0), PI, 0.0, -0.5, 0.0, 0, 1.0, 0.15, 0.65, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 2.0), PI, 0.0, 0.5, 0.0, 0, 1.0, 0.15, 0.65, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 217, name: "Auto Double".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: (b_spd * 1.1), 
            base_health: (b_hp as f32 * 1.1) as i32, base_damage: b_dmg, base_regen: b_regen, base_reload: b_reload, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![],
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.7, 2.0), 0.0, 0.0, -0.5, 0.0, 0, 1.0, 0.15, 0.65, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 2.0), 0.0, 0.0, 0.5, 0.0, 0, 1.0, 0.15, 0.65, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 2.0), PI, 0.0, -0.5, 0.0, 0, 1.0, 0.15, 0.65, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 2.0), PI, 0.0, 0.5, 0.0, 0, 1.0, 0.15, 0.65, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
            ], 
            turrets: vec![ 
                TurretConfig { class_id: 0, offset_forward: 0.0, offset_side: 0.0, angle_offset: 0.0, size: 0.5, stat_multiplier: 1.0, render_layer: true, override_target: false, layer: 0 } 
            ], 
            layers: vec![], 
        },
        EntityClass { 
            id: 7, name: "Assassin".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd * 0.9, 
            base_health: (b_hp as f32 * 1.2) as i32, base_damage: (b_dmg as f32 * 1.2) as i32, base_regen: b_regen, base_reload: (b_reload as f32 * 2.0) as usize, fov_factor: 1.6, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![31], 
            barrels: vec![ 
                make_barrel(make_rect_barrel(1.0, 3.2), 0.0, 0.0, 0.0, 0.0, 0, 1.5, 1.5, 4.0, 1.8, 0.0, BulletType::Standard, 0, true, 0, false, 0) 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 17, name: "Overseer".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd * 0.9, 
            base_health: (b_hp as f32 * 1.1) as i32, base_damage: (b_dmg as f32 * 1.4) as i32, base_regen: b_regen, base_reload: (b_reload as f32 * 5.0) as usize, fov_factor: 1.2, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![18],
            barrels: vec![ 
                make_barrel(make_trap_barrel(0.6, 1.8, 0.6), FRAC_PI_2, 0.0, 0.7, 0.0, 0, 100.0, 0.1, 1.0, 1.0, 0.0, BulletType::Drone, 3, true, 4, true, 0), 
                make_barrel(make_trap_barrel(0.6, 1.8, 0.6), -FRAC_PI_2, 0.0, -0.7, 0.0, 0, 100.0, 0.1, 1.0, 1.0, 0.0, BulletType::Drone, 3, true, 4, true, 0), 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 30, name: "Hunter".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: b_hp, base_damage: b_dmg, base_regen: b_regen, base_reload: (b_reload as f32 * 1.6) as usize, fov_factor: 1.5, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![36, 37],
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.9, 2.5), 0.0, 0.0, 0.0, 0.0, 0, 1.5, 0.2, 0.7, 1.25, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(1.2, 2.2), 0.0, 0.0, 0.0, 0.25, 0, 1.5, 0.2, 0.7, 1.25, 0.0, BulletType::Standard, 0, true, 0, false, 0) 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 15, name: "Trapper".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: (b_hp as f32 * 1.2) as i32, base_damage: (b_dmg as f32 * 1.5) as i32, base_regen: b_regen, base_reload: (b_reload as f32 * 1.5) as usize, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![39, 40, 41, 42, 16],
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.6, 1.2), 0.0, 0.0, 0.0, 0.0, 0, 1.0, 0.0, 1.0, 1.0, 0.0, BulletType::Standard, 0, false, 0, false, 0), 
                make_barrel(make_trap_barrel(0.6, 1.5, 0.6), 0.0, 1.2, 0.0, 0.0, 0, 1.5, 0.2, 1.0, 4.0, 0.0, BulletType::Trap, 131, true, 0, false, 0), 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 8, name: "Destroyer".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd * 0.8, 
            base_health: (b_hp as f32 * 1.4) as i32, base_damage: (b_dmg as f32 * 2.0) as i32, base_regen: b_regen, base_reload: (b_reload as f32 * 3.0) as usize, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![51, 43],
            barrels: vec![ 
                make_barrel(make_rect_barrel(1.6, 1.9), 0.0, 0.0, 0.0, 0.0, 0, 1.2, 9.0, 35.0, 0.4, 0.0, BulletType::Standard, 0, true, 0, false, 0) 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 25, name: "Gunner".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: b_hp, base_damage: b_dmg, base_regen: b_regen, base_reload: b_reload, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![45, 37, 39],
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.3, 1.6), 0.0, 0.0, 0.20, 0.0, 0, 0.7, 0.05, 0.7, 1.0, 0.1, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.3, 1.6), 0.0, 0.0, -0.20, 0.0, 0, 0.7, 0.05, 0.7, 1.0, 0.1, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.3, 1.6), 0.0, -0.2, 0.6, 0.0, 0, 0.7, 0.05, 0.7, 1.0, 0.1, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.3, 1.6), 0.0, -0.2, -0.6, 0.0, 0, 0.7, 0.05, 0.7, 1.0, 0.1, BulletType::Standard, 0, true, 0, false, 0), 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 11, name: "Tri-Angle".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: (b_spd * 1.33) * 1.5, 
            base_health: (b_hp as f32 * 1.1) as i32, base_damage: b_dmg, base_regen: b_regen, base_reload: b_reload, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![12, 13],
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.8, 1.9), 0.0, 0.0, 0.0, 0.0, 0, 1.0, 0.2, 1.0, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 1.6), 2.6, 0.0, 0.0, 0.0, 0, 0.5, 1.5, 0.2, 0.8, 0.1, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 1.6), -2.6, 0.0, 0.0, 0.0, 0, 0.5, 1.5, 0.2, 0.8, 0.1, BulletType::Standard, 0, true, 0, false, 0), 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 23, name: "Auto 3".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: (b_hp as f32 * 1.1) as i32, base_damage: b_dmg, base_regen: b_regen, base_reload: b_reload, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX,  
            upgrades: vec![24], 
            barrels: vec![], 
            turrets: vec![ 
                TurretConfig { class_id: 0, offset_forward: 1.0, offset_side: 0.0, angle_offset: 0.0, size: 0.5, stat_multiplier: 1.0, render_layer: false, override_target: false, layer: 0 }, 
                TurretConfig { class_id: 0, offset_forward: -0.5, offset_side: 0.8660254, angle_offset: 2.0943951, size: 0.5, stat_multiplier: 1.0, render_layer: false, override_target: false, layer: 0 }, 
                TurretConfig { class_id: 0, offset_forward: -0.5, offset_side: -0.8660254, angle_offset: 4.1887902, size: 0.5, stat_multiplier: 1.0, render_layer: false, override_target: false, layer: 0 }, 
            ], 
            layers: vec![], 
        },
        EntityClass { 
            id: 19, name: "Sprayer".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: b_hp, base_damage: b_dmg, base_regen: b_regen, base_reload: b_reload, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![], 
            barrels: vec![ 
                make_barrel(make_trap_barrel(0.8, 1.4, 1.8), 0.0, 0.0, 0.0, 0.0, 0, 0.7, 0.1, 0.7, 1.0, 0.4, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.5, 1.6), 0.0, 0.0, 0.0, 0.0, 0, 0.7, 0.05, 0.7, 1.0, 0.1, BulletType::Standard, 0, true, 0, false, 0), 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 20, name: "Triplet".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: b_hp, base_damage: b_dmg, base_regen: b_regen, base_reload: b_reload, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![], 
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.6, 1.9), 0.0, 0.0, 0.0, 0.0, 0, 0.8, 0.1, 0.7, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.6, 1.8), 0.0, 0.0, 0.6, 0.0, 0, 0.8, 0.1, 0.7, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.6, 1.8), 0.0, 0.0, -0.6, 0.0, 0, 0.8, 0.1, 0.7, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 21, name: "Penta Shot".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: b_hp, base_damage: b_dmg, base_regen: b_regen, base_reload: b_reload, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![], 
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.7, 1.6), -0.7, 0.0, 0.0, 0.0, 0, 1.0, 0.15, 0.7, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 1.6), 0.7, 0.0, 0.0, 0.0, 0, 1.0, 0.15, 0.7, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 1.8), -0.35, 0.0, 0.0, 0.0, 0, 1.0, 0.15, 0.7, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 1.8), 0.35, 0.0, 0.0, 0.0, 0, 1.0, 0.15, 0.7, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 2.0), 0.0, 0.0, 0.0, 0.0, 0, 1.0, 0.15, 0.7, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 16, name: "Octo Trapper".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: (b_hp as f32 * 1.3) as i32, base_damage: (b_dmg as f32 * 1.4) as i32, base_regen: b_regen, base_reload: (b_reload as f32 * 1.8) as usize, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![], 
            barrels: octo_trapper_barrels, 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 27, name: "Octo Tank".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: b_hp, base_damage: b_dmg, base_regen: b_regen, base_reload: b_reload, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![], 
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.8, 1.9), 0.0, 0.0, 0.0, 0.0, 0, 1.0, 0.2, 1.0, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0),
                make_barrel(make_rect_barrel(0.8, 1.9), 0.78539, 0.0, 0.0, 0.0, 0, 1.0, 0.2, 1.0, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0),
                make_barrel(make_rect_barrel(0.8, 1.9), 1.57079, 0.0, 0.0, 0.0, 0, 1.0, 0.2, 1.0, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0),
                make_barrel(make_rect_barrel(0.8, 1.9), 2.35619, 0.0, 0.0, 0.0, 0, 1.0, 0.2, 1.0, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0),
                make_barrel(make_rect_barrel(0.8, 1.9), 3.14159, 0.0, 0.0, 0.0, 0, 1.0, 0.2, 1.0, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0),
                make_barrel(make_rect_barrel(0.8, 1.9), 3.92699, 0.0, 0.0, 0.0, 0, 1.0, 0.2, 1.0, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0),
                make_barrel(make_rect_barrel(0.8, 1.9), 4.71238, 0.0, 0.0, 0.0, 0, 1.0, 0.2, 1.0, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0),
                make_barrel(make_rect_barrel(0.8, 1.9), 5.49778, 0.0, 0.0, 0.0, 0, 1.0, 0.2, 1.0, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0),
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 31, name: "Ranger".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd * 0.9, 
            base_health: (b_hp as f32 * 1.2) as i32, base_damage: (b_dmg as f32 * 1.2) as i32, base_regen: b_regen, base_reload: (b_reload as f32 * 2.5) as usize, fov_factor: 2.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![], 
            barrels: vec![ 
                make_barrel(make_rect_barrel(1.1, 3.8), 0.0, 0.0, 0.0, 0.0, 0, 1.5, 1.5, 6.0, 2.0, 0.0, BulletType::Standard, 0, true, 0, false, 0) 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 18, name: "Overlord".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd * 0.9, 
            base_health: (b_hp as f32 * 1.2) as i32, base_damage: (b_dmg as f32 * 1.5) as i32, base_regen: b_regen, base_reload: (b_reload as f32 * 4.0) as usize, fov_factor: 1.3, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![], 
            barrels: vec![ 
                make_barrel(make_trap_barrel(0.6, 1.8, 0.6), FRAC_PI_2, 0.0, 0.7, 0.0, 0, 100.0, 0.1, 1.0, 1.0, 0.0, BulletType::Drone, 3, true, 2, true, 0), 
                make_barrel(make_trap_barrel(0.6, 1.8, 0.6), -FRAC_PI_2, 0.0, -0.7, 0.0, 0, 100.0, 0.1, 1.0, 1.0, 0.0, BulletType::Drone, 3, true, 2, true, 0), 
                make_barrel(make_trap_barrel(0.6, 1.8, 0.6), 0.0, 0.7, 0.0, 0.0, 0, 100.0, 0.1, 1.0, 1.0, 0.0, BulletType::Drone, 3, true, 2, true, 0), 
                make_barrel(make_trap_barrel(0.6, 1.8, 0.6), PI, -0.7, 0.0, 0.0, 0, 100.0, 0.1, 1.0, 1.0, 0.0, BulletType::Drone, 3, true, 2, true, 0), 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 36, name: "Predator".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: b_hp, base_damage: b_dmg, base_regen: b_regen, base_reload: (b_reload as f32 * 2.0) as usize, fov_factor: 1.6, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![], 
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.9, 2.5), 0.0, 0.0, 0.0, 0.0, 0, 1.5, 0.2, 0.7, 1.25, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(1.2, 2.2), 0.0, 0.0, 0.0, 0.2, 0, 1.5, 0.2, 0.7, 1.25, 0.0, BulletType::Standard, 0, true, 0, false, 0),
                make_barrel(make_rect_barrel(1.5, 1.9), 0.0, 0.0, 0.0, 0.4, 0, 1.5, 0.2, 0.7, 1.25, 0.0, BulletType::Standard, 0, true, 0, false, 0),
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 37, name: "Streamliner".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: b_hp, base_damage: b_dmg, base_regen: b_regen, base_reload: (b_reload as f32 * 0.8) as usize, fov_factor: 1.4, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![], 
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.7, 2.5), 0.0, 0.0, 0.0, 0.0, 0, 0.8, 0.05, 0.5, 1.25, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 2.3), 0.0, 0.0, 0.0, 0.2, 0, 0.8, 0.05, 0.5, 1.25, 0.0, BulletType::Standard, 0, true, 0, false, 0),
                make_barrel(make_rect_barrel(0.7, 2.1), 0.0, 0.0, 0.0, 0.4, 0, 0.8, 0.05, 0.5, 1.25, 0.0, BulletType::Standard, 0, true, 0, false, 0),
                make_barrel(make_rect_barrel(0.7, 1.9), 0.0, 0.0, 0.0, 0.6, 0, 0.8, 0.05, 0.5, 1.25, 0.0, BulletType::Standard, 0, true, 0, false, 0),
                make_barrel(make_rect_barrel(0.7, 1.7), 0.0, 0.0, 0.0, 0.8, 0, 0.8, 0.05, 0.5, 1.25, 0.0, BulletType::Standard, 0, true, 0, false, 0),
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 39, name: "Gunner Trapper".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: b_hp, base_damage: b_dmg, base_regen: b_regen, base_reload: b_reload, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![], 
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.6, 1.2), PI, 0.0, 0.0, 0.0, 0, 1.0, 0.0, 1.0, 1.0, 0.0, BulletType::Standard, 0, false, 0, false, 0), 
                make_barrel(make_trap_barrel(0.6, 1.5, 0.6), PI, -1.2, 0.0, 0.0, 0, 1.5, 0.2, 1.0, 4.0, 0.0, BulletType::Trap, 131, true, 0, false, 0),
                make_barrel(make_rect_barrel(0.35, 1.6), 0.0, 0.0, 0.4, 0.0, 0, 0.7, 0.05, 0.7, 1.0, 0.1, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.35, 1.6), 0.0, 0.0, -0.4, 0.0, 0, 0.7, 0.05, 0.7, 1.0, 0.1, BulletType::Standard, 0, true, 0, false, 0), 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 40, name: "Overtrapper".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: b_hp, base_damage: b_dmg, base_regen: b_regen, base_reload: b_reload, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![], 
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.6, 1.2), 0.0, 0.0, 0.0, 0.0, 0, 1.0, 0.0, 1.0, 1.0, 0.0, BulletType::Standard, 0, false, 0, false, 0), 
                make_barrel(make_trap_barrel(0.6, 1.5, 0.6), 0.0, 1.2, 0.0, 0.0, 0, 1.5, 0.2, 1.0, 4.0, 0.0, BulletType::Trap, 131, true, 0, false, 0), 
                make_barrel(make_trap_barrel(0.6, 1.8, 0.6), -2.0944, 0.7 * f32::cos(-2.0944), 0.7 * f32::sin(-2.0944), 0.0, 0, 100.0, 0.1, 1.0, 1.0, 0.0, BulletType::Drone, 3, true, 2, true, 0), 
                make_barrel(make_trap_barrel(0.6, 1.8, 0.6), 2.0944, 0.7 * f32::cos(2.0944), 0.7 * f32::sin(2.0944), 0.0, 0, 100.0, 0.1, 1.0, 1.0, 0.0, BulletType::Drone, 3, true, 2, true, 0),
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 41, name: "Mega Trapper".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: b_hp, base_damage: b_dmg, base_regen: b_regen, base_reload: b_reload * 2, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![], 
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.6, 1.2), 0.0, 0.0, 0.0, 0.0, 0, 1.0, 0.0, 1.0, 1.0, 0.0, BulletType::Standard, 0, false, 0, false, 0), 
                make_barrel(make_trap_barrel(1.2, 2.5, 1.2), 0.0, 1.2, 0.0, 0.0, 0, 1.5, 0.5, 2.5, 4.0, 0.0, BulletType::Trap, 131, true, 0, false, 0) 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 42, name: "Auto Trapper".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: b_hp, base_damage: b_dmg, base_regen: b_regen, base_reload: b_reload, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![], 
            barrels: vec![
                make_barrel(make_rect_barrel(0.6, 1.2), 0.0, 0.0, 0.0, 0.0, 0, 1.0, 0.0, 1.0, 1.0, 0.0, BulletType::Standard, 0, false, 0, false, 0), 
                make_barrel(make_trap_barrel(0.6, 1.5, 0.6), 0.0, 1.2, 0.0, 0.0, 0, 1.5, 0.2, 1.0, 4.0, 0.0, BulletType::Trap, 131, true, 0, false, 0) 
            ], 
            turrets: vec![ 
                TurretConfig { class_id: 0, offset_forward: 0.0, offset_side: 0.0, angle_offset: 0.0, size: 0.5, stat_multiplier: 1.0, render_layer: true, override_target: false, layer: 0 } 
            ], 
            layers: vec![], 
        },
        EntityClass { 
            id: 51, name: "Annihilator".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd * 0.8, 
            base_health: (b_hp as f32 * 1.4) as i32, base_damage: (b_dmg as f32 * 2.0) as i32, base_regen: b_regen, base_reload: (b_reload as f32 * 3.0) as usize, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![], 
            barrels: vec![ 
                make_barrel(make_rect_barrel(2.0, 1.9), 0.0, 0.0, 0.0, 1.42, 0, 1.2, 9.0, 50.0, 0.4, 0.0, BulletType::Standard, 0, true, 0, false, 0) 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 43, name: "Hybrid".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd * 0.8, 
            base_health: (b_hp as f32 * 1.4) as i32, base_damage: (b_dmg as f32 * 2.0) as i32, base_regen: b_regen, base_reload: (b_reload as f32 * 3.0) as usize, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![], 
            barrels: vec![ 
                make_barrel(make_rect_barrel(1.6, 1.9), 0.0, 0.0, 0.0, 0.0, 0, 1.2, 9.0, 35.0, 0.4, 0.0, BulletType::Standard, 0, true, 0, false, 0),
                make_barrel(make_trap_barrel(0.6, 1.8, 0.6), PI, -0.8, 0.0, 0.0, 0, 100.0, 0.1, 1.0, 1.0, 0.0, BulletType::Drone, 3, true, 2, true, 0) 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 45, name: "Auto Gunner".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: b_hp, base_damage: b_dmg, base_regen: b_regen, base_reload: b_reload, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![], 
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.3, 1.6), 0.0, 0.0, 0.20, 0.0, 0, 0.7, 0.05, 0.7, 1.0, 0.1, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.3, 1.6), 0.0, 0.0, -0.20, 0.0, 0, 0.7, 0.05, 0.7, 1.0, 0.1, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.3, 1.6), 0.0, -0.2, 0.6, 0.0, 0, 0.7, 0.05, 0.7, 1.0, 0.1, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.3, 1.6), 0.0, -0.2, -0.6, 0.0, 0, 0.7, 0.05, 0.7, 1.0, 0.1, BulletType::Standard, 0, true, 0, false, 0), 
            ], 
            turrets: vec![ 
                TurretConfig { class_id: 0, offset_forward: 0.0, offset_side: 0.0, angle_offset: 0.0, size: 0.5, stat_multiplier: 1.0, render_layer: true, override_target: false, layer: 0 } 
            ], 
            layers: vec![], 
        },
        EntityClass { 
            id: 12, name: "Booster".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: (b_spd * 1.50) * 1.5, 
            base_health: (b_hp as f32 * 1.1) as i32, base_damage: b_dmg, base_regen: b_regen, base_reload: 30, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![], 
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.8, 1.9), 0.0, 0.0, 0.0, 0.0, 0, 1.0, 0.1, 1.0, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 1.5), 2.5, 0.0, 0.0, 0.0, 0, 0.5, 1.5, 0.2, 0.8, 0.1, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 1.5), -2.5, 0.0, 0.0, 0.0, 0, 0.5, 1.5, 0.2, 0.8, 0.1, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 1.6), 2.7, 0.0, 0.0, 0.0, 0, 0.5, 1.5, 0.2, 0.8, 0.1, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 1.6), -2.7, 0.0, 0.0, 0.0, 0, 0.5, 1.5, 0.2, 0.8, 0.1, BulletType::Standard, 0, true, 0, false, 0), 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 13, name: "Fighter".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: (b_spd * 1.33) * 1.5, 
            base_health: (b_hp as f32 * 1.1) as i32, base_damage: b_dmg, base_regen: b_regen, base_reload: b_reload, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![], 
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.8, 1.9), 0.0, 0.0, 0.0, 0.0, 0, 1.0, 0.2, 1.0, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 1.6), 1.57079, 0.0, 0.0, 0.0, 0, 0.9, 0.5, 0.8, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 1.6), -1.57079, 0.0, 0.0, 0.0, 0, 0.9, 0.5, 0.8, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 1.6), 2.61799, 0.0, 0.0, 0.0, 0, 0.5, 1.5, 0.2, 0.8, 0.1, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 1.6), -2.61799, 0.0, 0.0, 0.0, 0, 0.5, 1.5, 0.2, 0.8, 0.1, BulletType::Standard, 0, true, 0, false, 0), 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 24, name: "Auto 5".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: (b_hp as f32 * 1.15) as i32, base_damage: b_dmg, base_regen: b_regen, base_reload: b_reload, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![], 
            barrels: vec![], 
            turrets: vec![ 
                TurretConfig { class_id: 0, offset_forward: r * 1.0, offset_side: r * 0.0, angle_offset: 0.0, size: 0.5, stat_multiplier: 1.0, render_layer: false, override_target: false, layer: 0 }, 
                TurretConfig { class_id: 0, offset_forward: r * 0.309017, offset_side: r * 0.951057, angle_offset: 1.256637, size: 0.5, stat_multiplier: 1.0, render_layer: false, override_target: false, layer: 0 }, 
                TurretConfig { class_id: 0, offset_forward: r * (-0.809017), offset_side: r * 0.587785, angle_offset: 2.513274, size: 0.5, stat_multiplier: 1.0, render_layer: false, override_target: false, layer: 0 }, 
                TurretConfig { class_id: 0, offset_forward: r * (-0.809017), offset_side: r * (-0.587785), angle_offset: 3.769911, size: 0.5, stat_multiplier: 1.0, render_layer: false, override_target: false, layer: 0 }, 
                TurretConfig { class_id: 0, offset_forward: r * 0.309017, offset_side: r * (-0.951057), angle_offset: 5.026548, size: 0.5, stat_multiplier: 1.0, render_layer: false, override_target: false, layer: 0 }, 
            ], 
            layers: vec![], 
        },
        EntityClass { 
            id: 99, name: "Testbed".to_string(), mass: 100.0, shape: 0, body_radius: 4.0, base_speed: b_spd * 1.5, 
            base_health: b_hp * 10, base_damage: b_dmg * 10, base_regen: b_regen * 10, base_reload: 10, fov_factor: 2.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![0, 14, 10, 100, 101, 102, 103, 106, 210, 211, 212, 213, 214, 215, 205, 50, 209], 
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.8, 1.9), 0.0, 0.0, 0.0, 0.0, 0, 1.0, 0.0, 1.0, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0) 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 14, name: "Spectator".to_string(), mass: 5.0, shape: 0, body_radius: 4.0, base_speed: 20.0, 
            base_health: i32::MAX, base_damage: 600, base_regen: 100, base_reload: 35, fov_factor: 3.0, auto_spin: 0.015, color: 255, score: u32::MAX, 
            upgrades: vec![0], 
            barrels: vec![], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 22, name: "Auto Tank".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: b_hp, base_damage: b_dmg, base_regen: b_regen, base_reload: b_reload, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![24], 
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.8, 1.9), 0.0, 0.0, 0.0, 0.0, 0, 1.0, 0.2, 1.0, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0) 
            ], 
            turrets: vec![ 
                TurretConfig { class_id: 0, offset_forward: 0.0, offset_side: 0.0, angle_offset: 0.0, size: 0.5, stat_multiplier: 1.0, render_layer: true, override_target: false, layer: 0 } 
            ], 
            layers: vec![], 
        },
        EntityClass { 
            id: 209, name: "Railgun".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd * 0.9, 
            base_health: (b_hp as f32 * 1.2) as i32, base_damage: (b_dmg as f32 * 1.2) as i32, base_regen: b_regen, base_reload: (b_reload as f32 * 1.5) as usize, fov_factor: 1.4, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![], 
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.5, 2.5), 0.0, 0.0, -0.25, 0.0, 0, 1.5, 0.5, 2.5, 3.0, 0.0, BulletType::Standard, 0, false, 0, false, 0), 
                make_barrel(make_rect_barrel(0.5, 2.5), 0.0, 0.0, 0.25, 0.0, 0, 1.5, 0.5, 2.5, 3.0, 0.0, BulletType::Standard, 0, false, 0, false, 0), 
                make_barrel(make_rect_barrel(0.5, 3.0), 0.0, 0.0, 0.0, 0.0, 0, 1.5, 0.5, 2.5, 3.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 50, name: "Director".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd * 0.9, 
            base_health: (b_hp as f32 * 1.1) as i32, base_damage: (b_dmg as f32 * 1.4) as i32, base_regen: b_regen, base_reload: (b_reload as f32 * 5.0) as usize, fov_factor: 1.2, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![18], 
            barrels: vec![ 
                make_barrel(make_trap_barrel(0.6, 1.8, 0.6), 0.0, 0.7, 0.0, 0.0, 0, 100.0, 0.1, 1.0, 1.0, 0.0, BulletType::Drone, 3, true, 8, true, 0), 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 178, name: "Celestial Trapper".to_string(), mass: 1.0, shape: 0, body_radius: 4.0, base_speed: b_spd, 
            base_health: (b_hp as f32 * 1.2) as i32, base_damage: (b_dmg as f32 * 1.5) as i32, base_regen: b_regen, base_reload: (b_reload as f32 * 1.5) as usize, fov_factor: 1.0, auto_spin: 0.0, color: 255, score: u32::MAX, 
            upgrades: vec![16], 
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.8, 1.2), 0.0, 0.0, 0.0, 0.0, 0, 1.0, 0.0, 1.0, 1.0, 0.0, BulletType::Standard, 0, false, 0, false, 0), 
                make_barrel(make_trap_barrel(0.8, 2.5, 0.8), 0.0, 1.2, 0.0, 0.0, 0, 1.5, 0.2, 1.0, 4.0, 0.0, BulletType::Trap, 131, true, 0, false, 0), 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 10, name: "Summoner".to_string(), mass: 50.0, shape: 4, body_radius: 12.0, base_speed: 0.06, 
            base_health: 60000, base_damage: 700, base_regen: 200, base_reload: 60, fov_factor: 5.0, auto_spin: 0.02, color: 13, score: 500000, 
            upgrades: vec![], 
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.8, 1.4), 0.0, 0.0, 0.0, 0.0, 0, 100.0, 0.0, 1.5, 1.0, 0.0, BulletType::Drone, 4, true, 4, false, 0), 
                make_barrel(make_rect_barrel(0.8, 1.4), FRAC_PI_2, 0.0, 0.0, 0.0, 15, 100.0, 0.0, 1.5, 1.0, 0.0, BulletType::Drone, 4, true, 4, false, 0), 
                make_barrel(make_rect_barrel(0.8, 1.4), PI, 0.0, 0.0, 0.0, 30, 100.0, 0.0, 1.5, 1.0, 0.0, BulletType::Drone, 4, true, 4, false, 0), 
                make_barrel(make_rect_barrel(0.8, 1.4), -FRAC_PI_2, 0.0, 0.0, 0.0, 45, 100.0, 0.0, 1.5, 1.0, 0.0, BulletType::Drone, 4, true, 4, false, 0), 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 100, name: "Guardian".to_string(), mass: 50.0, shape: 3, body_radius: 12.0, base_speed: 0.12, 
            base_health: 55000, base_damage: 800, base_regen: 200, base_reload: 25, fov_factor: 5.0, auto_spin: 0.0, color: 5, score: 500000, 
            upgrades: vec![], 
            barrels: vec![ 
                make_barrel(make_trap_barrel(0.8, 2.2, 0.8), PI, -0.2, 0.0, 0.0, 0, 2.0, 0.5, 2.0, 1.5, 0.05, BulletType::Drone, 3, true, 8, false, 0), 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 101, name: "Defender".to_string(), mass: 50.0, shape: 3, body_radius: 12.0, base_speed: 0.1, 
            base_health: 65000, base_damage: 900, base_regen: 300, base_reload: 30, fov_factor: 5.0, auto_spin: 0.0, color: 2, score: 500000, 
            upgrades: vec![], 
            barrels: vec![ 
                make_barrel(make_trap_barrel(0.8, 2.0, 0.8), 1.0472, 0.0, 0.0, 0.0, 0, 1.5, 0.2, 1.2, 3.0, 0.0, BulletType::Trap, 131, true, 0, false, 0), 
                make_barrel(make_trap_barrel(0.8, 2.0, 0.8), 3.1415, 0.0, 0.0, 0.0, 10, 1.5, 0.2, 1.2, 3.0, 0.0, BulletType::Trap, 131, true, 0, false, 0), 
                make_barrel(make_trap_barrel(0.8, 2.0, 0.8), 5.2359, 0.0, 0.0, 0.0, 20, 1.5, 0.2, 1.2, 3.0, 0.0, BulletType::Trap, 131, true, 0, false, 0), 
            ], 
            turrets: vec![ 
                TurretConfig { class_id: 0, offset_forward: -0.5, offset_side: 0.0, angle_offset: 3.14159, size: 0.15, stat_multiplier: 1.0, render_layer: true, override_target: false, layer: 0 }, 
                TurretConfig { class_id: 0, offset_forward: -0.5, offset_side: 0.5, angle_offset: 3.14159, size: 0.15, stat_multiplier: 1.0, render_layer: true, override_target: false, layer: 0 }, 
                TurretConfig { class_id: 0, offset_forward: -0.5, offset_side: -0.5, angle_offset: 3.14159, size: 0.15, stat_multiplier: 1.0, render_layer: true, override_target: false, layer: 0 }, 
                TurretConfig { class_id: 0, offset_forward: 0.25, offset_side: 0.433, angle_offset: 1.0472, size: 0.15, stat_multiplier: 1.0, render_layer: true, override_target: false, layer: 0 }, 
                TurretConfig { class_id: 0, offset_forward: -0.183, offset_side: 0.683, angle_offset: 1.0472, size: 0.15, stat_multiplier: 1.0, render_layer: true, override_target: false, layer: 0 }, 
                TurretConfig { class_id: 0, offset_forward: 0.683, offset_side: 0.183, angle_offset: 1.0472, size: 0.15, stat_multiplier: 1.0, render_layer: true, override_target: false, layer: 0 }, 
                TurretConfig { class_id: 0, offset_forward: 0.25, offset_side: -0.433, angle_offset: 5.2359, size: 0.15, stat_multiplier: 1.0, render_layer: true, override_target: false, layer: 0 }, 
                TurretConfig { class_id: 0, offset_forward: 0.683, offset_side: -0.183, angle_offset: 5.2359, size: 0.15, stat_multiplier: 1.0, render_layer: true, override_target: false, layer: 0 }, 
                TurretConfig { class_id: 0, offset_forward: -0.183, offset_side: -0.683, angle_offset: 5.2359, size: 0.15, stat_multiplier: 1.0, render_layer: true, override_target: false, layer: 0 }, 
            ], 
            layers: vec![], 
        },
        EntityClass { 
            id: 102, name: "Fallen Booster".to_string(), mass: 30.0, shape: 0, body_radius: 9.0, base_speed: 0.35, 
            base_health: 80000, base_damage: 1200, base_regen: 400, base_reload: 20, fov_factor: 5.0, auto_spin: 0.0, color: 7, score: 500000, 
            upgrades: vec![], 
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.8, 1.9), 0.0, 0.0, 0.0, 0.0, 0, 1.0, 0.1, 1.0, 1.0, 0.0, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 1.5), 2.6, 0.0, 0.0, 0.0, 0, 0.5, 1.5, 0.2, 0.8, 0.1, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 1.5), -2.6, 0.0, 0.0, 0.0, 0, 0.5, 1.5, 0.2, 0.8, 0.1, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 1.6), 2.7, 0.0, 0.0, 0.0, 0, 0.5, 1.5, 0.2, 0.8, 0.1, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 1.6), -2.7, 0.0, 0.0, 0.0, 0, 0.5, 1.5, 0.2, 0.8, 0.1, BulletType::Standard, 0, true, 0, false, 0), 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 103, name: "Fallen Overlord".to_string(), mass: 30.0, shape: 0, body_radius: 9.0, base_speed: 0.1, 
            base_health: 70000, base_damage: 750, base_regen: 300, base_reload: 20, fov_factor: 5.0, auto_spin: 0.0, color: 7, score: 500000, 
            upgrades: vec![], 
            barrels: vec![ 
                make_barrel(make_trap_barrel(0.6, 1.8, 0.6), FRAC_PI_2, 0.0, 0.7, 0.0, 0, 100.0, 0.1, 1.0, 1.0, 0.0, BulletType::Drone, 3, true, 2, true, 0), 
                make_barrel(make_trap_barrel(0.6, 1.8, 0.6), -FRAC_PI_2, 0.0, -0.7, 0.0, 0, 100.0, 0.1, 1.0, 1.0, 0.0, BulletType::Drone, 3, true, 2, true, 0), 
                make_barrel(make_trap_barrel(0.6, 1.8, 0.6), 0.0, 0.7, 0.0, 0.0, 0, 100.0, 0.1, 1.0, 1.0, 0.0, BulletType::Drone, 3, true, 2, true, 0), 
                make_barrel(make_trap_barrel(0.6, 1.8, 0.6), PI, -0.7, 0.0, 0.0, 0, 100.0, 0.1, 1.0, 1.0, 0.0, BulletType::Drone, 3, true, 2, true, 0), 
            ], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 106, name: "Elite Crasher".to_string(), mass: 20.0, shape: 3, body_radius: 8.0, base_speed: 0.6, 
            base_health: 40000, base_damage: 2000, base_regen: 300, base_reload: 10, fov_factor: 4.0, auto_spin: 0.0, color: 5, score: 250000, 
            upgrades: vec![], 
            barrels: vec![ 
                make_barrel(make_rect_barrel(0.7, 1.2), 2.6, 0.0, 0.0, 0.0, 0, 1.0, 1.0, 0.5, 0.1, 0.5, BulletType::Standard, 0, true, 0, false, 0), 
                make_barrel(make_rect_barrel(0.7, 1.2), -2.6, 0.0, 0.0, 0.0, 0, 1.0, 1.0, 0.5, 0.1, 0.5, BulletType::Standard, 0, true, 0, false, 0), 
            ], 
            turrets: vec![ 
                TurretConfig { class_id: 19, offset_forward: 0.0, offset_side: 0.0, angle_offset: 0.0, size: 0.5, stat_multiplier: 1.0, render_layer: true, override_target: false, layer: 0 } 
            ], 
            layers: vec![], 
        },
        
        make_celestial(210, "Skolas", 30, 51, 8, [0.75, 0.75]),
        make_celestial(211, "Levaszk", 31, 3, 19, [1.5, 1.5]),
        make_celestial(212, "Issakis", 32, 36, 30, [1.25, 1.25]),
        make_celestial(213, "Aksis", 33, 21, 20, [1.2, 1.2]),
        make_celestial(214, "Persys", 34, 37, 209, [1.0, 1.5]),
        make_celestial(215, "Fikrul", 35, 50, 25, [1.5, 1.5]),

        EntityClass { 
            id: 200, name: "Square".to_string(), mass: 0.5, shape: 4, body_radius: 2.0, base_speed: 1.0, 
            base_health: 300, base_damage: 20, base_regen: 0, base_reload: 0, fov_factor: 1.0, auto_spin: 0.0, color: 13, score: 75, 
            upgrades: vec![], 
            barrels: vec![], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 201, name: "Triangle".to_string(), mass: 0.7, shape: 3, body_radius: 3.0, base_speed: 1.0, 
            base_health: 700, base_damage: 30, base_regen: 0, base_reload: 0, fov_factor: 1.0, auto_spin: 0.0, color: 2, score: 100, 
            upgrades: vec![], 
            barrels: vec![], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 202, name: "Pentagon".to_string(), mass: 2.0, shape: 5, body_radius: 4.0, base_speed: 0.5, 
            base_health: 1500, base_damage: 50, base_regen: 0, base_reload: 0, fov_factor: 1.0, auto_spin: 0.0, color: 14, score: 300, 
            upgrades: vec![], 
            barrels: vec![], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 203, name: "Hexagon".to_string(), mass: 10.0, shape: 6, body_radius: 5.0, base_speed: 0.2, 
            base_health: 5000, base_damage: 200, base_regen: 10, base_reload: 0, fov_factor: 1.0, auto_spin: 0.0, color: 0, score: 1000, 
            upgrades: vec![], 
            barrels: vec![], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 204, name: "Crasher".to_string(), mass: 0.9, shape: 3, body_radius: 3.5, base_speed: 0.5, 
            base_health: 700, base_damage: 50, base_regen: 0, base_reload: 0, fov_factor: 1.0, auto_spin: 0.0, color: 5, score: 400, 
            upgrades: vec![], 
            barrels: vec![], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 205, name: "Egg".to_string(), mass: 0.5, shape: 0, body_radius: 12.0, base_speed: 0.0, 
            base_health: i32::MAX, base_damage: 0, base_regen: 0, base_reload: 0, fov_factor: 1.0, auto_spin: 0.0, color: 6, score: 0, 
            upgrades: vec![], 
            barrels: vec![], 
            turrets: vec![], 
            layers: vec![], 
        },
        EntityClass { 
            id: 253, name: "Meteor".to_string(), mass: 1000000.0, shape: 8, body_radius: 15.0, base_speed: 0.7, 
            base_health: i32::MAX, base_damage: 20, base_regen: 0, base_reload: 0, fov_factor: 1.0, auto_spin: 0.05, color: 19, score: 0, 
            upgrades: vec![], 
            barrels: vec![], 
            turrets: vec![], 
            layers: vec![
                LayerConfig { shape: 7, color: 17, scale: 0.9, spin_speed: 0.02, spin_direction: true },
                LayerConfig { shape: 6, color: 33, scale: 0.7, spin_speed: 0.04, spin_direction: false },
            ] 
        },
    ];
    classes.into_iter().map(|c| (c.id, c)).collect()
}
