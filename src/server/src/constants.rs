use collision_engine::engine::config::config_data::ROOM_SIZE;

pub const MAP_DIMENSION: usize = 32;
pub const WALL_DIAMETER: f32 = ROOM_SIZE / (MAP_DIMENSION as f32);
pub const WALL_SIZE: f32 = WALL_DIAMETER * 0.5;
pub const MAX_POLYGONS: usize = 1000; 
pub const MAX_BOTS: usize = 50; 
pub const SENTINEL_HEALTH: i32 = i32::MAX; 
pub const BASE_VIEW_WIDTH: f32 = 160.0; 
pub const BASE_VIEW_HEIGHT: f32 = 80.0;
pub const VIEW_PADDING: f32 = 8.0; 

pub const MAX_STAT_LEVEL: u8 = 7;
pub const STAT_COUNT: usize = 10; 

pub const BASE_SIZE_SQUARE: usize = 8; 
pub const BASE_WIDTH_STRIP: usize = 4;  

pub const BODY_DAMAGE_MULT: f32 = 8.0; 
pub const RECOIL_DAMPENING: f32 = 0.15;
pub const COLLISION_MIN_IMPACT: f32 = 0.005;

pub const ENTITY_TANK: u8 = 0;
pub const ENTITY_BULLET: u8 = 1;
pub const ENTITY_POLYGON: u8 = 2;
pub const ENTITY_DRONE: u8 = 3;
pub const ENTITY_WALL: u8 = 4;

pub const BOSS_IDS: [u8; 6] = [10, 100, 101, 102, 103, 106];
pub const CELESTIAL_IDS: [u8; 6] = [210, 211, 212, 213, 214, 215];

pub const TEAM_POLYGON: u8 = 100;

pub const CLASS_SQUARE: u8 = 200;
pub const CLASS_TRIANGLE: u8 = 201;
pub const CLASS_PENTAGON: u8 = 202;
pub const CLASS_HEXAGON: u8 = 203;
pub const CLASS_CRASHER: u8 = 204;
pub const CLASS_EGG: u8 = 205;
pub const CLASS_METEOR: u8 = 253;

pub const TEAM_METEOR: u8 = 254;
