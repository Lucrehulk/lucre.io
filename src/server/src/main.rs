mod mazegen;
mod entity_classes;
mod constants;
mod models;
mod pathfinding;
mod utils;
use crate::mazegen::MazeManager;
use crate::entity_classes::*;
use crate::constants::*;
use crate::models::*;
use crate::pathfinding::*;
use crate::utils::*;
use collision_engine::engine::engine::Room;
use collision_engine::engine::config::config_data::{ROOM_SIZE, SPATIAL_GRID_DIMENSION, TICK_TIME, THREADS, STORE_COLLISIONS};
use rand::Rng;
use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
use std::{collections::{HashMap, HashSet}, net::IpAddr, sync::{Arc, atomic::Ordering}, time::{Duration, Instant}, f32::consts::{FRAC_PI_2, PI}};
use tokio::{net::TcpListener, sync::mpsc, time::sleep};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
const NODE_SIZE: f32 = ROOM_SIZE / (SPATIAL_GRID_DIMENSION as f32);

fn send_notification(client_sender: &mpsc::UnboundedSender<Vec<u8>>, notification_message: &str) {
    let mut notification_packet = vec![80];
    let message_bytes = notification_message.as_bytes();
    notification_packet.push(message_bytes.len() as u8);
    notification_packet.extend(message_bytes);
    let _ = client_sender.send(notification_packet);
}

fn apply_entity_stats(game_entity: &mut GameEntity, entity_class_definition: &EntityClass) {
    let base_ticks_to_heal = 40.0 * (1000.0 / TICK_TIME as f32);
    let min_ticks_to_heal = 15.0 * (1000.0 / TICK_TIME as f32);
    let stat_progress = (game_entity.stats[0] as f32) / 7.0;
    let ticks_to_heal = base_ticks_to_heal * (1.0 - stat_progress) + min_ticks_to_heal * stat_progress;
    let old_max_health = game_entity.max_health;
    game_entity.max_health = entity_class_definition.base_health.saturating_add(game_entity.stats[1] as i32 * 200);
    game_entity.regen = (game_entity.max_health as f32 / ticks_to_heal).ceil() as i32;
    if game_entity.max_health > old_max_health && !game_entity.dead {
        if game_entity.max_health == i32::MAX {
             game_entity.health.store(i32::MAX, Ordering::Relaxed)
        } else {
             let health_difference = game_entity.max_health - old_max_health;
             let current_health = game_entity.health.load(Ordering::Relaxed);
             game_entity.health.store(current_health + health_difference, Ordering::Relaxed);
        }
    }
    game_entity.damage = entity_class_definition.base_damage + (game_entity.stats[2] as i32 * 25);
    let reload_reduction_factor = 1.0 - (game_entity.stats[6] as f32 * 0.08);
    game_entity.reload_ticks = (entity_class_definition.base_reload as f32 * reload_reduction_factor).max(2.0) as usize;
}

fn spawn_game_entity(world_room: &mut Room, game_entities: &mut HashMap<usize, GameEntity>, tank_classes: &HashMap<u8, EntityClass>, position_x: f32, position_y: f32, facing_angle: f32, entity_mass: f32, friction: f32, velocity_x: f32, velocity_y: f32, terminal_velocity: f32, movement_acceleration: f32, entity_radius: f32, body_type: u8, is_static: bool, shape_id: u8, team_id: u8, color_id: u8, class_id: u8, barrels: Vec<Barrel>, name: String, score: u32, render_score: bool, parent_id_optional: Option<usize>, parent_barrel_index: usize, initial_health: i32, max_health: i32, regeneration: i32, damage: i32, reload_ticks: usize, fov_factor: f32, spin_rate: f32, orbit_radius: f32, creation_tick: usize, lifetime: usize, entity_type: u8, is_bot: bool, should_render_health: bool, invulnerable: bool, client_id: Option<usize>) -> usize {
    let entity_id = world_room.create_entity(position_x, position_y, entity_mass, friction, velocity_x, velocity_y, terminal_velocity, movement_acceleration, entity_radius, body_type, is_static);
    let parent_id = parent_id_optional.unwrap_or(entity_id);
    let mut game_entity = GameEntity::new(facing_angle, barrels, shape_id, team_id, color_id, class_id, name, score, render_score, parent_id, parent_barrel_index, initial_health, max_health, regeneration, damage, reload_ticks, fov_factor, spin_rate, orbit_radius, creation_tick, lifetime, entity_type, is_bot, should_render_health, invulnerable, client_id);
    if tank_classes.contains_key(&class_id) {
        let entity_definition = tank_classes.get(&class_id).unwrap();
        if game_entity.spin_rate == 0.0 { game_entity.spin_rate = entity_definition.auto_spin; };
        for (index, turret_config) in entity_definition.turrets.iter().enumerate() {
            if tank_classes.contains_key(&turret_config.class_id) {
                let turret_class_definition = tank_classes.get(&turret_config.class_id).unwrap();
                game_entity.turrets.push(TurretState { config_idx: index, initial_facing: facing_angle + turret_config.angle_offset, facing: facing_angle + turret_config.angle_offset, target_pos: None, barrels: turret_class_definition.barrels.clone() });
            }
        }
        game_entity.layer_facings = vec![facing_angle; entity_definition.layers.len()];
    }
    if is_bot && game_entity.class_id != CLASS_CRASHER {
        game_entity.auto_fire = true;
        game_entity.stat_points = 0;
        game_entity.stats = [MAX_STAT_LEVEL; STAT_COUNT];
        if tank_classes.contains_key(&class_id) {
             let entity_definition = tank_classes.get(&class_id).unwrap();
             apply_entity_stats(&mut game_entity, entity_definition);
             if let Some(physics_entity) = world_room.entities.get_mut(entity_id) {
                 let speed_multiplier = 1.0 + (game_entity.stats[7] as f32 * 0.15);
                 physics_entity.terminal_velocity_in_direction = entity_definition.base_speed * speed_multiplier;
                 let acceleration_multiplier = 0.05 + (game_entity.stats[8] as f32 * 0.005);
                 physics_entity.movement_acceleration = acceleration_multiplier;
                 physics_entity.movement_acceleration_45_deg = acceleration_multiplier * 0.70710678;
             }
        }
    }
    if entity_type == ENTITY_POLYGON || entity_type == ENTITY_BULLET || entity_type == ENTITY_DRONE { game_entity.invulnerable = false; };
    game_entities.insert(entity_id, game_entity);
    return entity_id;
}

fn setup_map_and_bases(maze_manager: &mut MazeManager, rng: &mut ThreadRng) -> Vec<BaseZone> {
    let mut game_bases = Vec::new();
    let mut available_team_colors = vec![10, 11, 12, 15];
    available_team_colors.shuffle(rng);
    let game_mode = rng.gen_range(0..3);
    match game_mode {
        0 => {
             let team_count = 2;
             let selected_colors = &available_team_colors[0..team_count];
             let corner_positions = vec![(0, 0), (SPATIAL_GRID_DIMENSION - BASE_SIZE_SQUARE, SPATIAL_GRID_DIMENSION - BASE_SIZE_SQUARE)];
             for (index, &(base_x, base_y)) in corner_positions.iter().enumerate() {
                 let width = BASE_SIZE_SQUARE;
                 let height = BASE_SIZE_SQUARE;
                 for x in base_x..(base_x + width) {
                     for y in base_y..(base_y + height) {
                         maze_manager.set_tile(x, y, false);
                     }
                 }
                 game_bases.push(BaseZone { x: base_x as f32 * NODE_SIZE, y: base_y as f32 * NODE_SIZE, w: width as f32 * NODE_SIZE, h: height as f32 * NODE_SIZE, team_id: selected_colors[index], party_code: generate_code(rng) });
             }
        },
        1 => {
             let team_count = 2;
             let selected_colors = &available_team_colors[0..team_count];
             let is_vertical_layout = rng.gen_bool(0.5);
             if is_vertical_layout {
                 let strip_positions = vec![0, SPATIAL_GRID_DIMENSION - BASE_WIDTH_STRIP];
                 for (index, &base_x) in strip_positions.iter().enumerate() {
                     let width = BASE_WIDTH_STRIP;
                     let height = SPATIAL_GRID_DIMENSION;
                     for x in base_x..(base_x + width) {
                         for y in 0..height {
                             maze_manager.set_tile(x, y, false);
                         }
                     }
                     game_bases.push(BaseZone { x: base_x as f32 * NODE_SIZE, y: 0.0, w: width as f32 * NODE_SIZE, h: height as f32 * NODE_SIZE, team_id: selected_colors[index], party_code: generate_code(rng) });
                 }
             } else {
                 let strip_positions = vec![0, SPATIAL_GRID_DIMENSION - BASE_WIDTH_STRIP];
                 for (index, &base_y) in strip_positions.iter().enumerate() {
                     let width = SPATIAL_GRID_DIMENSION;
                     let height = BASE_WIDTH_STRIP;
                     for y in base_y..(base_y + height) {
                         for x in 0..width {
                             maze_manager.set_tile(x, y, false);
                         }
                     }
                     game_bases.push(BaseZone { x: 0.0, y: base_y as f32 * NODE_SIZE, w: width as f32 * NODE_SIZE, h: height as f32 * NODE_SIZE, team_id: selected_colors[index], party_code: generate_code(rng) });
                 }
             }
        },
        _ => {
            let team_count = 4;
            let selected_colors = &available_team_colors[0..team_count];
            let corner_positions = vec![(0, 0), (SPATIAL_GRID_DIMENSION - BASE_SIZE_SQUARE, 0), (0, SPATIAL_GRID_DIMENSION - BASE_SIZE_SQUARE), (SPATIAL_GRID_DIMENSION - BASE_SIZE_SQUARE, SPATIAL_GRID_DIMENSION - BASE_SIZE_SQUARE)];
            for (index, &(base_x, base_y)) in corner_positions.iter().enumerate() {
                let width = BASE_SIZE_SQUARE;
                let height = BASE_SIZE_SQUARE;
                for x in base_x..(base_x + width) {
                    for y in base_y..(base_y + height) {
                        maze_manager.set_tile(x, y, false);
                    }
                }
                game_bases.push(BaseZone { x: base_x as f32 * NODE_SIZE, y: base_y as f32 * NODE_SIZE, w: width as f32 * NODE_SIZE, h: height as f32 * NODE_SIZE, team_id: selected_colors[index], party_code: generate_code(rng) });
            }
        }
    }
    return game_bases;
}

#[tokio::main]
async fn main() {
    let mut world_room = Room::init();
    let mut game_entities: HashMap<usize, GameEntity> = HashMap::new();
    let mut player_sessions: HashMap<usize, PlayerSession> = HashMap::new();
    let mut ip_to_client_map: HashMap<IpAddr, usize> = HashMap::new();
    let mut next_client_id: usize = 1;
    let mut bot_entity_ids: HashSet<usize> = HashSet::new();
    let mut rng = rand::rng();
    let tank_classes_definitions = init_tank_classes();
    let mut current_tick_counter: usize = 0;
    let map_type = 0;
    let mut maze_manager = match map_type {
        0 => MazeManager::instantiate_map(SPATIAL_GRID_DIMENSION, SPATIAL_GRID_DIMENSION, false),
        1 => MazeManager::instantiate_basic_erosion_maze(SPATIAL_GRID_DIMENSION, SPATIAL_GRID_DIMENSION, 0.55, &mut rng),
        _ => MazeManager::instantiate_branched_erosion_maze(SPATIAL_GRID_DIMENSION, SPATIAL_GRID_DIMENSION, 0.45, 10, 10, &mut rng),
    };
    let is_maze_mode = map_type != 0;
    let mut active_bases = setup_map_and_bases(&mut maze_manager, &mut rng);
    let active_team_colors: Vec<u8> = active_bases.iter().map(|b| b.team_id).collect();
    let mut base_init_packet = vec![11];
    base_init_packet.extend((active_bases.len() as u16).to_le_bytes());
    for base in &active_bases {
        base_init_packet.extend(base.x.to_le_bytes());
        base_init_packet.extend(base.y.to_le_bytes());
        base_init_packet.extend(base.w.to_le_bytes());
        base_init_packet.extend(base.h.to_le_bytes());
        base_init_packet.push(base.team_id);
    }
    let mut shared_base_packet = Arc::new(base_init_packet);
    let quantized_maze = maze_manager.square_quantize_maze();
    let mut wall_init_packet = vec![6];
    wall_init_packet.extend((quantized_maze.len() as u32).to_le_bytes());
    let mut wall_coordinates = HashSet::new();
    for square_data in quantized_maze {
        let grid_x = square_data[0] as f32;
        let grid_y = square_data[1] as f32;
        let magnitude = square_data[2] as f32;
        let wall_radius = WALL_SIZE * magnitude;
        let position_x = grid_x * WALL_DIAMETER + wall_radius;
        let position_y = grid_y * WALL_DIAMETER + wall_radius;
        wall_init_packet.extend(position_x.to_le_bytes());
        wall_init_packet.extend(position_y.to_le_bytes());
        wall_init_packet.extend(wall_radius.to_le_bytes());
        spawn_game_entity(&mut world_room, &mut game_entities, &tank_classes_definitions, position_x, position_y, 0.0, f32::MAX, 0.0, 0.0, 0.0, 0.0, 0.0, wall_radius, 0, true, 4, 255, 7, 0, Vec::new(), "".to_string(), 0, false, None, 0, SENTINEL_HEALTH, SENTINEL_HEALTH, 0, 0, 0, 1.0, 0.0, 0.0, current_tick_counter, usize::MAX, ENTITY_WALL, false, false, false, None);
    }
    for tile_data in maze_manager.true_tiles { wall_coordinates.insert((tile_data[0], tile_data[1])); };
    let shared_wall_packet = Arc::new(wall_init_packet);
    let pathfinding_walls: HashSet<(i32, i32)> = wall_coordinates.iter().map(|&(x, y)| (x as i32, y as i32)).collect();
    let maze_pathfinder = Pathfinder::new(SPATIAL_GRID_DIMENSION as i32, SPATIAL_GRID_DIMENSION as i32, pathfinding_walls);
    let mut empty_tiles = Vec::new();
    for tile in maze_manager.false_tiles { empty_tiles.push([tile[0] as f32, tile[1] as f32]); };
    let mut spawn_coordinate_generator = |rng: &mut ThreadRng, current_bases: &[BaseZone], team_id_opt: Option<u8>| -> [f32; 2] {
        if let Some(target_team_id) = team_id_opt {
            if let Some(base) = current_bases.iter().find(|b| b.team_id == target_team_id) {
                let pad_x = (NODE_SIZE * 0.5).min(base.w * 0.49).max(0.0);
                let pad_y = (NODE_SIZE * 0.5).min(base.h * 0.49).max(0.0);
                let min_x = base.x + pad_x;
                let max_x = (base.x + base.w - pad_x).max(min_x);
                let min_y = base.y + pad_y;
                let max_y = (base.y + base.h - pad_y).max(min_y);
                return [rng.gen_range(min_x..=max_x), rng.gen_range(min_y..=max_y)];
            }
        }
        if empty_tiles.is_empty() { return [ROOM_SIZE/2.0, ROOM_SIZE/2.0]; };
        loop {
            let selected_tile = empty_tiles[rng.gen_range(0..empty_tiles.len())];
            let min_x = selected_tile[0] * NODE_SIZE + NODE_SIZE*0.1;
            let max_x = min_x + NODE_SIZE*0.8;
            let min_y = selected_tile[1] * NODE_SIZE + NODE_SIZE*0.1;
            let max_y = min_y + NODE_SIZE*0.8;
            let random_x = rng.gen_range(min_x..max_x);
            let random_y = rng.gen_range(min_y..max_y);
            if team_id_opt.is_none() {
                let is_inside_base = current_bases.iter().any(|b| { random_x >= b.x && random_x <= b.x + b.w && random_y >= b.y && random_y <= b.y + b.h });
                if is_inside_base { continue; };
            }
            return [random_x, random_y];
        }
    };
    let tcp_listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    println!("Server running at wss://127.0.0.1:8080");
    let (server_command_sender, mut server_command_receiver) = mpsc::unbounded_channel::<ServerCommand>();
    {
        let server_command_sender = server_command_sender.clone();
        let tank_definitions_copy = tank_classes_definitions.clone();
        let walls_packet_copy = shared_wall_packet.clone();
        let bases_packet_copy = shared_base_packet.clone();
        tokio::spawn(async move {
            loop {
                let (connection_stream, _) = tcp_listener.accept().await.unwrap();
                let client_ip = connection_stream.peer_addr().unwrap().ip();
                let command_sender = server_command_sender.clone();
                let initial_tank_definitions = tank_definitions_copy.clone();
                let wall_packet_ref = walls_packet_copy.clone();
                let base_packet_ref = bases_packet_copy.clone();
                tokio::spawn(async move {
                    let websocket_stream = match accept_async(connection_stream).await { Ok(ws) => ws, Err(_e) => { return } };
                    let (mut websocket_write, mut websocket_read) = websocket_stream.split();
                    let (client_data_sender, mut client_data_receiver) = mpsc::unbounded_channel::<Vec<u8>>();
                    let (client_id_sender, mut client_id_receiver) = mpsc::unbounded_channel::<usize>();
                    let handshake_message = websocket_read.next().await;
                    let player_nickname = match handshake_message { Some(Ok(Message::Binary(bytes))) => String::from_utf8_lossy(&bytes).to_string(), Some(Ok(Message::Text(txt))) => txt, _ => "Player".to_string(), };
                    let sanitized_name = if player_nickname.len() > 24 { player_nickname[..24].to_string() } else { player_nickname };
                    if command_sender.send(ServerCommand::Connect(client_data_sender, client_id_sender, sanitized_name, client_ip)).is_err() { return; };
                    let assigned_client_id = match client_id_receiver.recv().await { Some(id) => id, None => return };
                    let mut config_packet = vec![0];
                    config_packet.extend(ROOM_SIZE.to_le_bytes());
                    config_packet.extend((SPATIAL_GRID_DIMENSION as u32).to_le_bytes());
                    let _ = websocket_write.send(Message::Binary(config_packet)).await;
                    let mut definition_packet = vec![3];
                    definition_packet.extend((initial_tank_definitions.len() as u32).to_le_bytes());
                    for tank_def in initial_tank_definitions.values() {
                        definition_packet.push(tank_def.id);
                        let name_byte_slice = tank_def.name.as_bytes();
                        definition_packet.push(name_byte_slice.len() as u8);
                        definition_packet.extend(name_byte_slice);
                        definition_packet.extend(tank_def.body_radius.to_le_bytes());
                        definition_packet.push(tank_def.shape);
                        definition_packet.extend(tank_def.fov_factor.to_le_bytes());
                        definition_packet.push(tank_def.layers.len() as u8);
                        for layer in &tank_def.layers {
                            definition_packet.push(layer.shape);
                            definition_packet.push(layer.color);
                            definition_packet.extend(layer.scale.to_le_bytes());
                            definition_packet.extend(layer.spin_speed.to_le_bytes());
                            definition_packet.push(if layer.spin_direction { 1 } else { 0 });
                        }
                        definition_packet.extend((tank_def.barrels.len() as u32).to_le_bytes());
                        for barrel in &tank_def.barrels {
                            definition_packet.extend(barrel.angle_offset.to_le_bytes());
                            definition_packet.extend(barrel.offset_forward.to_le_bytes());
                            definition_packet.extend(barrel.offset_side.to_le_bytes());
                            definition_packet.extend(barrel.recoil.to_le_bytes());
                            definition_packet.extend(barrel.spread.to_le_bytes());
                            definition_packet.push(barrel.layer);
                            definition_packet.push(barrel.vertices.len() as u8);
                            for v in &barrel.vertices {
                                definition_packet.extend(v[0].to_le_bytes());
                                definition_packet.extend(v[1].to_le_bytes());
                            }
                        }
                        definition_packet.extend((tank_def.turrets.len() as u32).to_le_bytes());
                        for turret in &tank_def.turrets {
                            definition_packet.push(turret.class_id);
                            definition_packet.extend(turret.offset_forward.to_le_bytes());
                            definition_packet.extend(turret.offset_side.to_le_bytes());
                            definition_packet.extend(turret.angle_offset.to_le_bytes());
                            definition_packet.extend(turret.size.to_le_bytes());
                            definition_packet.extend(turret.stat_multiplier.to_le_bytes());
                            definition_packet.push(turret.layer);
                            definition_packet.push(if turret.override_target { 1 } else { 0 });
                            definition_packet.push(if turret.render_layer { 1 } else { 0 });
                        }
                    }
                    let _ = websocket_write.send(Message::Binary(definition_packet)).await;
                    let _ = websocket_write.send(Message::Binary(wall_packet_ref.to_vec())).await;
                    let _ = websocket_write.send(Message::Binary(base_packet_ref.to_vec())).await;
                    loop {
                        tokio::select! {
                            incoming_message = websocket_read.next() => {
                                match incoming_message {
                                    Some(Ok(Message::Binary(data_bytes))) => {
                                        let _ = command_sender.send(ServerCommand::Input(assigned_client_id, data_bytes));
                                    }
                                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => {
                                        let _ = command_sender.send(ServerCommand::Disconnect(assigned_client_id));
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                            outgoing_packet = client_data_receiver.recv() => {
                                match outgoing_packet {
                                    Some(packet_data) => {
                                        if websocket_write.send(Message::Binary(packet_data)).await.is_err() { break; };
                                    }
                                    None => break,
                                }
                            }
                        }
                    }
                    let _ = command_sender.send(ServerCommand::Disconnect(assigned_client_id));
                });
            }
        });
    }
    let mut current_leader_entity_id: Option<usize> = None;
    let mut active_game_event = GameEvent::None;
    let mut last_event_end_tick = 0;
    let mut saved_base_configuration: Vec<BaseZone> = Vec::new();
    loop {
        let tick_start_time = Instant::now();
        current_tick_counter += 1;
        while let Ok(server_command) = server_command_receiver.try_recv() {
            match server_command {
                ServerCommand::Connect(client_sender, client_id_reply, client_name_str, client_ip) => {
                let existing_client_id_option = None;
                if let Some(id) = existing_client_id_option {
                    if let Some(session) = player_sessions.get_mut(&id) {
                        session.sender = client_sender.clone();
                        session.is_connected = true;
                        session.disconnect_at = None;
                        let _ = client_id_reply.send(id);
                        send_notification(&client_sender, &format!("Welcome back, {}.", session.saved_name));
                        sleep(Duration::from_millis(50)).await;
                        if let Some(entity_id) = session.entity_id {
                            let mut camera_packet = vec![2];
                            camera_packet.extend((entity_id as u64).to_le_bytes());
                            let _ = session.sender.send(camera_packet);
                            if let Some(game_entity) = game_entities.get(&entity_id) {
                                if tank_classes_definitions.contains_key(&game_entity.class_id) {
                                    let definition = tank_classes_definitions.get(&game_entity.class_id).unwrap();
                                    let mut upgrades_packet = vec![4];
                                    upgrades_packet.extend((entity_id as u64).to_le_bytes());
                                    upgrades_packet.push(definition.upgrades.len() as u8);
                                    upgrades_packet.extend(&definition.upgrades);
                                    let _ = client_sender.send(upgrades_packet);
                                    let mut stats_packet = vec![9];
                                    stats_packet.push(game_entity.stat_points);
                                    stats_packet.extend(&game_entity.stats);
                                    let _ = client_sender.send(stats_packet);
                                }
                            }
                            if let Some(base) = active_bases.iter().find(|b| b.team_id == session.saved_team) {
                                let party_code_bytes = base.party_code.as_bytes();
                                let mut code_packet = vec![15];
                                code_packet.push(party_code_bytes.len() as u8);
                                code_packet.extend(party_code_bytes);
                                let _ = client_sender.send(code_packet);
                            }
                        }
                    }
                } else {
                    let new_client_id = next_client_id;
                    next_client_id += 1;
                    let (parsed_name, requested_code) = if let Some(index) = client_name_str.find('#') {
                        (client_name_str[..index].to_string(), Some(client_name_str[index+1..].to_string()))
                    } else {
                        (client_name_str, None)
                    };
                    let mut best_team_id = 0;
                    let mut matched_party_code = false;
                    if let Some(code) = requested_code {
                        if let Some(base) = active_bases.iter().find(|b| b.party_code == code) {
                            best_team_id = base.team_id;
                            matched_party_code = true;
                        }
                    }
                    if !matched_party_code {
                        let mut team_counts = HashMap::new();
                        for &color_id in &active_team_colors { team_counts.insert(color_id, 0); };
                        for session in player_sessions.values() { *team_counts.entry(session.saved_team).or_insert(0) += 1; };
                        best_team_id = if !active_team_colors.is_empty() { active_team_colors[0] } else { 10 };
                        let mut min_member_count = usize::MAX;
                        for (&team_id, &count) in &team_counts { if count < min_member_count { min_member_count = count; best_team_id = team_id; } }
                    }
                    let [spawn_x, spawn_y] = spawn_coordinate_generator(&mut rng, &active_bases, Some(best_team_id));
                    let starting_tank_definition = tank_classes_definitions.get(&0).unwrap();
                    let starting_color = if starting_tank_definition.color != 255 { starting_tank_definition.color } else { best_team_id };
                    let new_entity_id = spawn_game_entity(&mut world_room, &mut game_entities, &tank_classes_definitions, spawn_x, spawn_y, 0.0, starting_tank_definition.mass, 0.95, 0.0, 0.0, starting_tank_definition.base_speed, 0.05, starting_tank_definition.body_radius, 1, true, starting_tank_definition.shape, best_team_id, starting_color, starting_tank_definition.id, starting_tank_definition.barrels.clone(), parsed_name.clone(), 26000, true, None, 0, starting_tank_definition.base_health, starting_tank_definition.base_health, starting_tank_definition.base_regen, starting_tank_definition.base_damage, starting_tank_definition.base_reload, starting_tank_definition.fov_factor, 0.0, 0.0, current_tick_counter, usize::MAX, ENTITY_TANK, false, true, true, Some(new_client_id));
                    if let Some(game_entity) = game_entities.get_mut(&new_entity_id) { game_entity.stat_points = 45; };
                    player_sessions.insert(new_client_id, PlayerSession { client_id: new_client_id, sender: client_sender.clone(), is_connected: true, disconnect_at: None, ip: client_ip, saved_name: parsed_name.clone(), saved_team: best_team_id, entity_id: Some(new_entity_id), last_camera_pos: (spawn_x, spawn_y), fov_factor: 1.0, entity_created_at: Instant::now() });
                    let _ = client_id_reply.send(new_client_id);
                    let mut camera_packet = vec![2];
                    camera_packet.extend((new_entity_id as u64).to_le_bytes());
                    let _ = client_sender.send(camera_packet);
                    send_notification(&client_sender, &format!("Welcome, {}.", parsed_name));
                    sleep(Duration::from_millis(50)).await;
                    let mut upgrades_packet = vec![4];
                    upgrades_packet.extend((new_entity_id as u64).to_le_bytes());
                    upgrades_packet.push(starting_tank_definition.upgrades.len() as u8);
                    upgrades_packet.extend(&starting_tank_definition.upgrades);
                    let _ = client_sender.send(upgrades_packet);
                    let mut stats_packet = vec![9];
                    stats_packet.push(45);
                    stats_packet.extend(&[0u8; STAT_COUNT]);
                    let _ = client_sender.send(stats_packet);
                    if let Some(base) = active_bases.iter().find(|b| b.team_id == best_team_id) {
                        let party_code_bytes = base.party_code.as_bytes();
                        let mut party_packet = vec![15];
                        party_packet.push(party_code_bytes.len() as u8);
                        party_packet.extend(party_code_bytes);
                        let _ = client_sender.send(party_packet);
                    }
                }
            },
                ServerCommand::Disconnect(disconnected_client_id) => { if let Some(session) = player_sessions.get_mut(&disconnected_client_id) { session.is_connected = false; session.disconnect_at = Some(Instant::now()); } },
                ServerCommand::Input(input_client_id, input_data) => {
                    let mut controlled_entity_id_opt = None;
                    if let Some(session) = player_sessions.get(&input_client_id) { controlled_entity_id_opt = session.entity_id; };
                    if controlled_entity_id_opt.is_none() {
                        if !input_data.is_empty() && input_data[0] == 4 {
                            handle_input(input_client_id, input_data, &mut world_room, &mut game_entities, &tank_classes_definitions, &mut rng, &mut spawn_coordinate_generator, current_tick_counter, &mut player_sessions, &active_bases);
                        }
                    } else if let Some(entity_id) = controlled_entity_id_opt {
                        handle_input(entity_id, input_data, &mut world_room, &mut game_entities, &tank_classes_definitions, &mut rng, &mut spawn_coordinate_generator, current_tick_counter, &mut player_sessions, &active_bases);
                    }
                }
            }
        }
        let active_bot_count: usize = bot_entity_ids.iter().filter(|&&id| { if let Some(game_entity) = game_entities.get(&id) { !game_entity.dead } else { false } }).count();
        if active_bot_count < MAX_BOTS && !active_team_colors.is_empty() {
            let mut team_member_counts = HashMap::new();
            for &color in &active_team_colors { team_member_counts.insert(color, 0); };
            for game_entity in game_entities.values() {
                if !game_entity.dead && game_entity.entity_type == ENTITY_TANK {
                    if let Some(count) = team_member_counts.get_mut(&game_entity.team) {
                        *count += 1;
                    }
                }
            }
            let minimum_team_count = team_member_counts.values().min().cloned().unwrap_or(0);
            let candidate_teams: Vec<u8> = team_member_counts.iter().filter(|&(_, &c)| c == minimum_team_count).map(|(&t, _)| t).collect();
            let selected_team = if !candidate_teams.is_empty() { candidate_teams[rng.gen_range(0..candidate_teams.len())] } else { active_team_colors[0] };
            let [bot_x, bot_y] = spawn_coordinate_generator(&mut rng, &active_bases, Some(selected_team));
            let high_tier_classes = [12, 13, 16, 18, 19, 20, 21, 24, 26, 27, 31, 36, 37, 39, 40, 41, 42, 43, 45, 51, 217];
            let bot_class_id = high_tier_classes[rng.gen_range(0..high_tier_classes.len())];
            let bot_definition = tank_classes_definitions.get(&bot_class_id).unwrap_or(tank_classes_definitions.get(&0).unwrap());
            let bot_names_list = ["Alpha", "Bravo", "Charlie", "Delta", "Echo", "Foxtrot", "Golf", "Hotel", "India", "Juliet"];
            let bot_name = format!("[AI] {}", bot_names_list[rng.gen_range(0..bot_names_list.len())]);
            let bot_color = if bot_definition.color != 255 { bot_definition.color } else { selected_team };
            let bot_id = spawn_game_entity(&mut world_room, &mut game_entities, &tank_classes_definitions, bot_x, bot_y, 0.0, bot_definition.mass, 0.95, 0.0, 0.0, bot_definition.base_speed, 0.05, bot_definition.body_radius, 1, true, bot_definition.shape, selected_team, bot_color, bot_definition.id, bot_definition.barrels.clone(), bot_name, 26000, true, None, 0, bot_definition.base_health, bot_definition.base_health, bot_definition.base_regen, bot_definition.base_damage, bot_definition.base_reload, bot_definition.fov_factor, 0.0, 0.0, current_tick_counter, usize::MAX, ENTITY_TANK, true, true, false, None);
            bot_entity_ids.insert(bot_id);
        }
        let mut potential_target_list: Vec<(usize, f32, f32, u8, u8)> = Vec::new();
        for (entity_id, game_entity) in &game_entities {
            if !game_entity.dead && !game_entity.invulnerable && (game_entity.entity_type == ENTITY_TANK || game_entity.entity_type == ENTITY_POLYGON) {
                if let Some(physics_entity) = world_room.entities.get(*entity_id) {
                    potential_target_list.push((*entity_id, physics_entity.x, physics_entity.y, game_entity.team, game_entity.entity_type));
                }
            }
        }
        let mut ai_command_list: Vec<(usize, Vec<u8>)> = Vec::new();
        let mut drone_movement_updates: Vec<(usize, f32, f32)> = Vec::new();
        let unsafe_game_entities_ptr = &mut game_entities as *mut HashMap<usize, GameEntity>;
        for (entity_id, game_entity) in  unsafe { &mut *unsafe_game_entities_ptr } {
             if game_entity.dead { continue; };
             if tank_classes_definitions.contains_key(&game_entity.class_id) {
                 let definition = tank_classes_definitions.get(&game_entity.class_id).unwrap();
                 for (index, layer) in definition.layers.iter().enumerate() {
                     if index < game_entity.layer_facings.len() {
                         if layer.spin_speed != 0.0 {
                             if layer.spin_direction { game_entity.layer_facings[index] += layer.spin_speed }
                             else { game_entity.layer_facings[index] -= layer.spin_speed };
                             while game_entity.layer_facings[index] > PI * 2.0 { game_entity.layer_facings[index] -= PI * 2.0; };
                             while game_entity.layer_facings[index] < 0.0 { game_entity.layer_facings[index] += PI * 2.0; };
                         }
                     }
                 }
             }
             if game_entity.entity_type == ENTITY_DRONE {
                 let mut is_parent_active = false;
                 let mut parent_position = [0.0, 0.0, 0.0];
                 let mut parent_facing = 0.0;
                 let mut is_parent_override = false;
                 let mut is_parent_autofire = false;
                 let mut parent_target_pos = [0.0, 0.0];
                 if let Some(parent_entity) = game_entities.get(&game_entity.parent_id) {
                     if !parent_entity.dead {
                         is_parent_active = true;
                         is_parent_override = parent_entity.is_override;
                         is_parent_autofire = parent_entity.auto_fire;
                         parent_target_pos = parent_entity.target_pos;
                         parent_facing = parent_entity.facing;
                         if let Some(parent_physics) = world_room.entities.get(game_entity.parent_id) {
                             parent_position = [parent_physics.x, parent_physics.y, parent_physics.radius];
                         }
                     }
                 }
                 if !is_parent_active { game_entity.health.store(0, Ordering::Relaxed); continue; }
                 if let Some(physics_entity) = world_room.entities.get(*entity_id) {
                    let drone_x = physics_entity.x;
                    let drone_y = physics_entity.y;
                    let mut move_target_x = parent_position[0];
                    let mut move_target_y = parent_position[1];
                    let mut should_orbit = false;
                    if is_parent_override { should_orbit = true }
                    else if is_parent_autofire { move_target_x = parent_target_pos[0]; move_target_y = parent_target_pos[1]; }
                    else {
                        let drone_vision_radius = (BASE_VIEW_WIDTH / 2.0) * game_entity.fov_factor;
                        let drone_vision_sq = drone_vision_radius * drone_vision_radius;
                        let mut best_drone_target = None;
                        let mut min_target_dist = f32::MAX;
                        for (target_id, tx, ty, target_team, target_type) in &potential_target_list {
                            if *target_id == *entity_id || *target_id == game_entity.parent_id { continue; };
                            if *target_team == game_entity.team && *target_team != 0 && *target_team != 255 { continue; };
                            let diff_x = tx - drone_x;
                            let diff_y = ty - drone_y;
                            let dist_sq = diff_x*diff_x + diff_y*diff_y;
                            if dist_sq > drone_vision_sq { continue; };
                            let type_priority = if *target_type == ENTITY_TANK { 1 } else { 0 };
                            if type_priority > -1 { if dist_sq < min_target_dist { min_target_dist = dist_sq; best_drone_target = Some([*tx, *ty]); } }
                        }
                        if let Some(t) = best_drone_target { move_target_x = t[0]; move_target_y = t[1]; } else { should_orbit = true };
                    }
                    if should_orbit {
                        let orbit_time = current_tick_counter as f32 * 0.02 + (game_entity.creation_tick as f32);
                        let orbit_distance = parent_position[2] * 2.5;
                        move_target_x = parent_position[0] + orbit_time.cos() * orbit_distance;
                        move_target_y = parent_position[1] + orbit_time.sin() * orbit_distance;
                    }
                    let diff_to_parent_x = parent_position[0] - drone_x;
                    let diff_to_parent_y = parent_position[1] - drone_y;
                    let dist_to_parent = (diff_to_parent_x * diff_to_parent_x + diff_to_parent_y * diff_to_parent_y).sqrt();
                    let avoidance_radius = parent_position[2] + physics_entity.radius + 2.0;
                    if dist_to_parent < avoidance_radius {
                        let diff_to_target_x = move_target_x - drone_x;
                        let diff_to_target_y = move_target_y - drone_y;
                        if (diff_to_target_x * diff_to_parent_x + diff_to_target_y * diff_to_parent_y) > 0.0 {
                            let push_vector_x = -diff_to_parent_x;
                            let push_vector_y = -diff_to_parent_y;
                            let push_length = dist_to_parent;
                            if push_length > 0.1 {
                                move_target_x += (push_vector_x / push_length) * 50.0;
                                move_target_y += (push_vector_y / push_length) * 50.0;
                            }
                        }
                    }
                    let diff_x = move_target_x - drone_x;
                    let diff_y = move_target_y - drone_y;
                    let distance = (diff_x*diff_x + diff_y*diff_y).sqrt();
                    if distance > 1.0 { drone_movement_updates.push((*entity_id, diff_x/distance, diff_y/distance)); game_entity.facing = diff_y.atan2(diff_x); }
                 }
                 continue;
             }
             for barrel in &mut game_entity.barrels {
                 if barrel.visual_recoil > 0.0 {
                     barrel.visual_recoil = (barrel.visual_recoil - 0.1).max(0.0);
                     let recoil_scale = 1.0 - (0.2 * barrel.visual_recoil);
                     if barrel.vertices.len() >= 4 {
                         for (vertex_idx, original_vertex) in barrel.original_vertices.iter().enumerate() {
                             if vertex_idx < barrel.vertices.len() {
                                 if original_vertex[0] > 0.01 { barrel.vertices[vertex_idx][0] = original_vertex[0] * recoil_scale; };
                             }
                         }
                     }
                 }
             }
             let spin_amount = if game_entity.spin_rate != 0.0 { game_entity.spin_rate } else if game_entity.auto_spin_enabled { 0.02 } else { 0.0 };
             if spin_amount != 0.0 {
                 game_entity.facing += spin_amount;
                 while game_entity.facing > PI * 2.0 { game_entity.facing -= PI * 2.0; };
                 while game_entity.facing < 0.0 { game_entity.facing += PI * 2.0; };
             }
             if game_entity.is_bot {
                 if let Some(physics_entity) = world_room.entities.get(*entity_id) {
                     let bot_x = physics_entity.x;
                     let bot_y = physics_entity.y;
                     let bot_team = game_entity.team;
                     let mut best_bot_target = None;
                     let mut min_bot_dist_sq = f32::MAX;
                     let mut best_target_priority = -1;
                     let bot_view_radius = (BASE_VIEW_WIDTH / 2.0) * game_entity.fov_factor;
                     let bot_view_sq = bot_view_radius * bot_view_radius;
                     for (target_id, tx, ty, target_team, target_type) in &potential_target_list {
                         if *target_id == *entity_id { continue; };
                         if *target_team == bot_team && *target_team != 0 && *target_team != 255 { continue; };
                         let diff_x = tx - bot_x;
                         let diff_y = ty - bot_y;
                         let dist_sq = diff_x*diff_x + diff_y*diff_y;
                         if dist_sq > bot_view_sq { continue; };
                         if *target_type == ENTITY_POLYGON { continue; };
                         let priority = if *target_type == ENTITY_TANK { 1 } else { 0 };
                         if priority > best_target_priority || (priority == best_target_priority && dist_sq < min_bot_dist_sq) {
                             best_target_priority = priority;
                             min_bot_dist_sq = dist_sq;
                             best_bot_target = Some([*tx, *ty]);
                         }
                     }
                     let is_aggressive_bot = BOSS_IDS.contains(&game_entity.class_id) || CELESTIAL_IDS.contains(&game_entity.class_id) || game_entity.class_id == CLASS_CRASHER;
                     let current_health = game_entity.health.load(Ordering::Relaxed);
                     let is_low_health = (current_health as f32) < (game_entity.max_health as f32 * 0.3);
                     let retreat_mode = !is_aggressive_bot && is_low_health;
                     let final_movement_target;
                     if let Some([tx, ty]) = best_bot_target {
                         game_entity.target_pos = [tx, ty];
                         final_movement_target = [tx, ty];
                         if !game_entity.current_path.is_empty() { game_entity.current_path.clear(); };
                     } else {
                        let distance_to_wander_target = (game_entity.target_pos[0] - bot_x).powi(2) + (game_entity.target_pos[1] - bot_y).powi(2);
                        if (game_entity.target_pos[0].abs() < 1.0 && game_entity.target_pos[1].abs() < 1.0) || distance_to_wander_target < 2500.0 || current_tick_counter > game_entity.last_path_calc + 400 {
                            let map_center = ROOM_SIZE / 2.0;
                            let mut valid_target_found = false;
                            for _ in 0..10 {
                                let new_wander_x = rng.gen_range((map_center - 250.0)..(map_center + 250.0));
                                let new_wander_y = rng.gen_range((map_center - 250.0)..(map_center + 250.0));
                                let grid_x = (new_wander_x / NODE_SIZE).floor() as i32;
                                let grid_y = (new_wander_y / NODE_SIZE).floor() as i32;
                                if !maze_pathfinder.walls.contains(&(grid_x, grid_y)) {
                                    game_entity.target_pos = [new_wander_x, new_wander_y];
                                    valid_target_found = true;
                                    break;
                                }
                            }
                            if !valid_target_found {
                                game_entity.target_pos = [ROOM_SIZE / 2.0, ROOM_SIZE / 2.0];
                            }
                            game_entity.last_path_calc = current_tick_counter;
                        }
                        final_movement_target = game_entity.target_pos;
                     }
                     let mut next_path_node = final_movement_target;
                     let use_direct_pathing = !is_maze_mode;
                     if !use_direct_pathing {
                         if current_tick_counter > game_entity.last_path_calc + 15 || game_entity.current_path.is_empty() {
                             game_entity.last_path_calc = current_tick_counter;
                             game_entity.current_path = maze_pathfinder.find_path([bot_x, bot_y], [final_movement_target[0], final_movement_target[1]]);
                         }
                         if let Some(node) = game_entity.current_path.last() {
                             let distance_to_node = (node[0] - bot_x).powi(2) + (node[1] - bot_y).powi(2);
                             if distance_to_node < 225.0 {
                                 game_entity.current_path.pop();
                                 if let Some(next) = game_entity.current_path.last() { next_path_node = *next; };
                             } else { next_path_node = *node; }
                         }
                     }
                     let mut diff_x = next_path_node[0] - bot_x;
                     let mut diff_y = next_path_node[1] - bot_y;
                     if retreat_mode && best_bot_target.is_some() { diff_x = -diff_x; diff_y = -diff_y; }
                     let mut movement_code = 16;
                     if diff_x.abs() > 0.5 || diff_y.abs() > 0.5 {
                         let horizontal_dir = if diff_x > 0.5 { 0 } else if diff_x < -0.5 { 1 } else { 8 };
                         let vertical_dir = if diff_y > 0.5 { 0 } else if diff_y < -0.5 { 1 } else { 8 };
                         if horizontal_dir != 8 && vertical_dir != 8 { movement_code = 4 + horizontal_dir * 2 + vertical_dir }
                         else if horizontal_dir == 8 && vertical_dir != 8 { movement_code = vertical_dir + 2 }
                         else if horizontal_dir != 8 && vertical_dir == 8 { movement_code = horizontal_dir };
                     }
                     ai_command_list.push((*entity_id, vec![1, movement_code]));
                     let facing_angle = (final_movement_target[1] - bot_y).atan2(final_movement_target[0] - bot_x);
                     if game_entity.spin_rate.abs() < f32::EPSILON && !game_entity.auto_spin_enabled {
                         let mut angle_packet = vec![0];
                         angle_packet.extend(facing_angle.to_le_bytes());
                         ai_command_list.push((*entity_id, angle_packet));
                     }
                     let should_shoot = best_bot_target.is_some();
                     ai_command_list.push((*entity_id, vec![2, if should_shoot { 1 } else { 0 }]));
                 }
             }
        }
        for (bot_id, packet) in ai_command_list {
            handle_input(bot_id, packet, &mut world_room, &mut game_entities, &tank_classes_definitions, &mut rng, &mut spawn_coordinate_generator, current_tick_counter, &mut player_sessions, &active_bases);
        }
        for (drone_id, dx, dy) in drone_movement_updates {
            if let Some(physics_entity) = world_room.entities.get_mut(drone_id) {
                let speed = physics_entity.terminal_velocity_in_direction;
                physics_entity.velocity_x += dx * speed * 0.1;
                physics_entity.velocity_y += dy * speed * 0.1;
            }
        }
        let mut spawn_bullet_queue: Vec<(f32, usize, f32, u8, u8, f32, f32, f32, f32, usize, i32, u8, f32, f32, bool, u8, usize)> = Vec::with_capacity(100);
        let mut recoil_update_list: Vec<(usize, f32, f32)> = Vec::with_capacity(50);
        for (entity_id, game_entity) in game_entities.iter_mut() {
            if game_entity.dead || game_entity.invulnerable { continue; };
            if let Some(physics_entity) = world_room.entities.get(*entity_id) {
                let parent_pos_x = physics_entity.x;
                let parent_pos_y = physics_entity.y;
                let parent_fov = game_entity.fov_factor;
                let view_radius = (BASE_VIEW_WIDTH / 2.0) * parent_fov;
                let view_range_squared = view_radius * view_radius;
                let mut layer_scale_factors = Vec::new();
                let mut turret_configurations = Vec::new();
                if tank_classes_definitions.contains_key(&game_entity.class_id) {
                    let definition = tank_classes_definitions.get(&game_entity.class_id).unwrap();
                    for l in &definition.layers { layer_scale_factors.push(l.scale) }; turret_configurations = definition.turrets.clone();
                }
                if !game_entity.turrets.is_empty() {
                    for (turret_index, turret_state) in game_entity.turrets.iter_mut().enumerate() {
                         let turret_config = &turret_configurations[turret_state.config_idx];
                         for barrel in &mut turret_state.barrels {
                             if barrel.visual_recoil > 0.0 {
                                 barrel.visual_recoil = (barrel.visual_recoil - 0.1).max(0.0);
                                 let recoil_scale = 1.0 - (0.2 * barrel.visual_recoil);
                                 if barrel.vertices.len() >= 4 {
                                     for (vertex_index, vertex) in barrel.original_vertices.iter().enumerate() {
                                         if vertex_index < barrel.vertices.len() {
                                             if vertex[0] > 0.01 { barrel.vertices[vertex_index][0] = vertex[0] * recoil_scale; };
                                         }
                                     }
                                 }
                             }
                         }
                         let layer_idx = turret_config.layer as usize;
                         let current_layer_angle = if layer_idx == 0 { game_entity.facing } else if layer_idx <= game_entity.layer_facings.len() { game_entity.layer_facings[layer_idx - 1] } else { game_entity.facing };
                         let current_layer_scale = if layer_idx == 0 { 1.0 } else if layer_idx <= layer_scale_factors.len() { layer_scale_factors[layer_idx - 1] } else { 1.0 };
                         let current_layer_radius = physics_entity.radius * current_layer_scale;
                         let forward_angle = current_layer_angle;
                         let side_angle = current_layer_angle + FRAC_PI_2;
                         let turret_world_x = parent_pos_x + forward_angle.cos() * (turret_config.offset_forward * current_layer_radius) + side_angle.cos() * (turret_config.offset_side * current_layer_radius);
                         let turret_world_y = parent_pos_y + forward_angle.sin() * (turret_config.offset_forward * current_layer_radius) + side_angle.sin() * (turret_config.offset_side * current_layer_radius);
                         turret_state.initial_facing = current_layer_angle + turret_config.angle_offset;
                         if turret_config.override_target || game_entity.is_override {
                             turret_state.facing = turret_state.initial_facing;
                             turret_state.target_pos = None;
                         } else {
                             let mut best_turret_target = None;
                             let mut min_turret_dist_sq = f32::MAX;
                             let mut best_turret_score = -1;
                             let is_center_mounted = turret_config.offset_forward.abs() < 0.1 && turret_config.offset_side.abs() < 0.1;
                             for (target_id, tx, ty, target_team, target_type) in &potential_target_list {
                                 if *target_id == *entity_id { continue; };
                                 if *target_team == game_entity.team && *target_team != 0 && *target_team != 255 { continue; };
                                 let diff_x = tx - turret_world_x;
                                 let diff_y = ty - turret_world_y;
                                 let angle_to_target = diff_y.atan2(diff_x);
                                 if !is_center_mounted {
                                     let mut angle_difference = angle_to_target - turret_state.initial_facing;
                                     while angle_difference < -PI { angle_difference += PI * 2.0; };
                                     while angle_difference > PI { angle_difference -= PI * 2.0; };
                                     if angle_difference.abs() > 1.6 { continue; };
                                 }
                                 let dist_sq = diff_x*diff_x + diff_y*diff_y;
                                 if dist_sq > view_range_squared { continue; };
                                 let type_score = if *target_type == ENTITY_TANK { 1 } else { 0 };
                                 if type_score > best_turret_score || (type_score == best_turret_score && dist_sq < min_turret_dist_sq) {
                                     best_turret_score = type_score;
                                     min_turret_dist_sq = dist_sq;
                                     best_turret_target = Some([*tx, *ty, angle_to_target]);
                                 }
                             }
                             if let Some([_tx, _ty, target_angle]) = best_turret_target {
                                 let mut angle_diff = target_angle - turret_state.facing;
                                 while angle_diff < -PI { angle_diff += PI * 2.0; };
                                 while angle_diff >= PI { angle_diff -= PI * 2.0; };
                                 turret_state.facing += angle_diff * 0.15;
                                 turret_state.target_pos = Some([_tx, _ty]);
                             }
                             else {
                                 let mut angle_diff = turret_state.initial_facing - turret_state.facing;
                                 while angle_diff < -PI { angle_diff += PI * 2.0; };
                                 while angle_diff >= PI { angle_diff -= PI * 2.0; };
                                 turret_state.facing += angle_diff * 0.1;
                                 turret_state.target_pos = None;
                             }
                         }
                         let should_fire_turret = turret_state.target_pos.is_some() || game_entity.auto_fire || game_entity.is_firing || turret_config.override_target;
                         let stat_multiplier = if turret_config.stat_multiplier > 0.0 { turret_config.stat_multiplier } else { 1.0 };
                         let total_reload_ticks = (game_entity.reload_ticks as f32 / stat_multiplier) as usize;
                         if should_fire_turret {
                              if !turret_state.barrels.is_empty() {
                                  if current_tick_counter >= turret_state.barrels[0].last_fired + total_reload_ticks {
                                      turret_state.barrels[0].last_fired = current_tick_counter;
                                  }
                              }
                              let turret_barrel_count = turret_state.barrels.len();
                              let anchor_tick = turret_state.barrels[0].last_fired;
                              for barrel_idx in 0..turret_barrel_count {
                                  let delay = (total_reload_ticks as f32 * turret_state.barrels[barrel_idx].delay_fraction) as usize;
                                  let target_tick = anchor_tick + delay;
                                  let is_ready = current_tick_counter >= target_tick;
                                  let is_fresh = turret_state.barrels[barrel_idx].last_fired < anchor_tick || (barrel_idx == 0 && turret_state.barrels[barrel_idx].last_fired == current_tick_counter);
                                  let should_fire = if barrel_idx == 0 {
                                      turret_state.barrels[0].last_fired == current_tick_counter
                                  } else {
                                      is_ready && turret_state.barrels[barrel_idx].last_fired < anchor_tick
                                  };
                                  if should_fire {
                                      let turret_barrel = &mut turret_state.barrels[barrel_idx];
                                      if !turret_barrel.can_shoot { continue; };
                                      if turret_barrel.bullet_type == BulletType::Drone { if turret_barrel.child_ids.len() >= turret_barrel.max_children { continue; } }
                                      turret_barrel.last_fired = current_tick_counter;
                                      turret_barrel.visual_recoil = 1.0;
                                      let speed_stat = 1.0 + (game_entity.stats[3] as f32 * 0.15);
                                      let health_stat = 100 + (game_entity.stats[4] as i32 * 100);
                                      let damage_stat = (game_entity.stats[5] as f32 * 100.0);
                                      let firing_angle = turret_state.facing + turret_barrel.angle_offset + (if turret_barrel.spread > 0.0 { rng.gen_range(-turret_barrel.spread/2.0..=turret_barrel.spread/2.0) } else { 0.0 });
                                      let world_turret_radius = physics_entity.radius * turret_config.size;
                                      let projectile_radius = if !turret_barrel.vertices.is_empty() { world_turret_radius * turret_barrel.vertices[0][1].abs() } else { 4.0 };
                                      let safe_spawn_dist = world_turret_radius + projectile_radius + 0.1;
                                      let side_angle_offset = turret_state.facing + turret_barrel.angle_offset + FRAC_PI_2;
                                      let mut bullet_x = turret_world_x + (turret_state.facing + turret_barrel.angle_offset).cos() * safe_spawn_dist + side_angle_offset.cos() * (turret_barrel.offset_side * world_turret_radius);
                                      let mut bullet_y = turret_world_y + (turret_state.facing + turret_barrel.angle_offset).sin() * safe_spawn_dist + side_angle_offset.sin() * (turret_barrel.offset_side * world_turret_radius);
                                      let dist_from_center_sq = (bullet_x - parent_pos_x).powi(2) + (bullet_y - parent_pos_y).powi(2);
                                      let min_safe_center_dist = physics_entity.radius + projectile_radius + 0.1;
                                      if dist_from_center_sq < min_safe_center_dist.powi(2) {
                                          let angle_from_center = (bullet_y - parent_pos_y).atan2(bullet_x - parent_pos_x);
                                          bullet_x = parent_pos_x + angle_from_center.cos() * min_safe_center_dist;
                                          bullet_y = parent_pos_y + angle_from_center.sin() * min_safe_center_dist;
                                      }
                                      let bullet_speed = (0.8 * turret_barrel.bullet_speed_mult * speed_stat) * stat_multiplier;
                                      let bullet_health = (health_stat as f32 * stat_multiplier) as i32;
                                      let bullet_life_ticks = if turret_barrel.bullet_type == BulletType::Drone { usize::MAX } else { ((60.0 * turret_barrel.bullet_lifetime_mult) * stat_multiplier) as usize };
                                      let final_damage = (((game_entity.damage as f32 * turret_barrel.damage_mult) + damage_stat) * 0.05) * stat_multiplier;
                                      let (has_friction, friction_val, base_mass, lifetime_mod, entity_type_val) = match turret_barrel.bullet_type { BulletType::Standard => (false, 1.0, 0.0001, 1.0, ENTITY_BULLET), BulletType::Trap => (true, 0.2, 0.0002, 2.5, ENTITY_BULLET), BulletType::Drone => (true, 0.2, 0.0003, f32::MAX, ENTITY_DRONE), };
                                      let mass_multiplier = 1.0 + (game_entity.stats[9] as f32 * 0.2);
                                      let adjusted_base_mass = if BOSS_IDS.contains(&game_entity.class_id) || CELESTIAL_IDS.contains(&game_entity.class_id) { base_mass * 2.0 } else { base_mass };
                                      let final_bullet_mass = adjusted_base_mass * mass_multiplier;
                                      let final_bullet_life = if entity_type_val == ENTITY_DRONE { usize::MAX } else { (bullet_life_ticks as f32 * lifetime_mod) as usize };
                                      let encoded_parent_index = (turret_index + 1) * 10000 + barrel_idx;
                                      spawn_bullet_queue.push((firing_angle, final_bullet_life, bullet_speed, game_entity.team, game_entity.color, bullet_x, bullet_y, final_damage, projectile_radius, *entity_id, bullet_health, turret_barrel.projectile_shape, friction_val, final_bullet_mass, has_friction, entity_type_val, encoded_parent_index));
                                  }
                              }
                         }
                    }
                }
                let mut total_recoil_vector_x = 0.0;
                let mut total_recoil_vector_y = 0.0;
                let mut did_fire_main_barrel = false;
                let speed_stat_factor = 1.0 + (game_entity.stats[3] as f32 * 0.15);
                let health_stat_val = 100 + (game_entity.stats[4] as i32 * 100);
                let damage_stat_val = (game_entity.stats[5] as f32 * 100.0);
                let wants_to_shoot_main = game_entity.is_firing || game_entity.auto_fire;
                let barrel_count = game_entity.barrels.len();
                let total_reload_ticks = game_entity.reload_ticks as usize;
                if wants_to_shoot_main && !game_entity.barrels.is_empty() {
                    if current_tick_counter >= game_entity.barrels[0].last_fired + total_reload_ticks {
                        game_entity.barrels[0].last_fired = current_tick_counter;
                    }
                }
                let anchor_tick = if !game_entity.barrels.is_empty() { game_entity.barrels[0].last_fired } else { 0 };
                for barrel_index in 0..barrel_count {
                    let delay = (total_reload_ticks as f32 * game_entity.barrels[barrel_index].delay_fraction) as usize;
                    let target_tick = anchor_tick + delay;
                    let should_fire = if barrel_index == 0 {
                        game_entity.barrels[0].last_fired == current_tick_counter && wants_to_shoot_main
                    } else {
                        current_tick_counter >= target_tick && game_entity.barrels[barrel_index].last_fired < anchor_tick
                    };
                    if should_fire {
                        let barrel = &mut game_entity.barrels[barrel_index];
                        if !barrel.can_shoot { continue; };
                        if !barrel.fire_lock && !wants_to_shoot_main && game_entity.entity_type != ENTITY_DRONE { continue; };
                        if barrel.bullet_type == BulletType::Drone { if barrel.child_ids.len() >= barrel.max_children { continue; } }
                        barrel.last_fired = current_tick_counter;
                        did_fire_main_barrel = true;
                        barrel.visual_recoil = 1.0;
                        let projectile_radius = if !barrel.vertices.is_empty() { physics_entity.radius * barrel.vertices[0][1].abs() } else { 4.0 };
                        let spread_angle_val = if barrel.spread > 0.0 { rng.gen_range(-barrel.spread/2.0..=barrel.spread/2.0) } else { 0.0 };
                        let layer_index = barrel.layer as usize;
                        let layer_angle = if layer_index == 0 { game_entity.facing } else if layer_index <= game_entity.layer_facings.len() { game_entity.layer_facings[layer_index - 1] } else { game_entity.facing };
                        let layer_scale = if layer_index == 0 { 1.0 } else if layer_index <= layer_scale_factors.len() { layer_scale_factors[layer_index - 1] } else { 1.0 };
                        let layer_radius = physics_entity.radius * layer_scale;
                        let firing_angle = layer_angle + barrel.angle_offset + spread_angle_val;
                        total_recoil_vector_x += -firing_angle.cos() * (barrel.recoil * RECOIL_DAMPENING);
                        total_recoil_vector_y += -firing_angle.sin() * (barrel.recoil * RECOIL_DAMPENING);
                        let safe_spawn_distance = layer_radius + projectile_radius + 0.1;
                        let side_angle_barrel = layer_angle + barrel.angle_offset + FRAC_PI_2;
                        let spawn_pos_x = physics_entity.x + (layer_angle + barrel.angle_offset).cos() * safe_spawn_distance + side_angle_barrel.cos() * (barrel.offset_side * layer_radius);
                        let spawn_pos_y = physics_entity.y + (layer_angle + barrel.angle_offset).sin() * safe_spawn_distance + side_angle_barrel.sin() * (barrel.offset_side * layer_radius);
                        let bullet_velocity = 0.8 * barrel.bullet_speed_mult * speed_stat_factor;
                        let (has_friction, friction_amount, base_mass, lifetime_modifier, projectile_type) = match barrel.bullet_type { BulletType::Standard => (false, 1.0, 0.0001, 1.0, ENTITY_BULLET), BulletType::Trap => (true, 0.2, 0.0001, 2.5, ENTITY_BULLET), BulletType::Drone => (true, 0.2, 0.0002, f32::MAX, ENTITY_DRONE), };
                        let mass_modifier = 1.0 + (game_entity.stats[9] as f32 * 0.2);
                        let adjusted_mass = if BOSS_IDS.contains(&game_entity.class_id) || CELESTIAL_IDS.contains(&game_entity.class_id) { base_mass * 2.0 } else { base_mass };
                        let final_mass_val = adjusted_mass * mass_modifier;
                        let bullet_lifetime = if barrel.bullet_type == BulletType::Drone { usize::MAX } else { (60.0 * barrel.bullet_lifetime_mult * lifetime_modifier) as usize };
                        let bullet_damage = ((game_entity.damage as f32 * barrel.damage_mult) + damage_stat_val) * 0.05;
                        spawn_bullet_queue.push((firing_angle, bullet_lifetime, bullet_velocity, game_entity.team, game_entity.color, spawn_pos_x, spawn_pos_y, bullet_damage, projectile_radius, *entity_id, health_stat_val, barrel.projectile_shape, friction_amount, final_mass_val, has_friction, projectile_type, barrel_index));
                    }
                }
                if did_fire_main_barrel && game_entity.entity_type == ENTITY_TANK { recoil_update_list.push((*entity_id, total_recoil_vector_x, total_recoil_vector_y)); };
            }
        }
        for (entity_id, recoil_x, recoil_y) in recoil_update_list { if let Some(physics_entity) = world_room.entities.get_mut(entity_id) { physics_entity.velocity_x += recoil_x; physics_entity.velocity_y += recoil_y; } }
        for (firing_angle, life_ticks, speed, team, color, x, y, damage, radius, parent_id, health, shape, friction, mass, has_friction, entity_type, parent_barrel_index) in spawn_bullet_queue {
            let vel_x = firing_angle.cos() * speed;
            let vel_y = firing_angle.sin() * speed;
            let new_bullet_id = spawn_game_entity(&mut world_room, &mut game_entities, &tank_classes_definitions, x, y, 0.0, mass, 1.0 - friction, vel_x, vel_y, speed, 0.0, radius, entity_type, has_friction, shape, team, color, 0, Vec::new(), "".to_string(), 0, false, Some(parent_id), parent_barrel_index, health, health, 0, damage as i32, 0, 1.0, 0.0, 0.0, current_tick_counter, life_ticks, entity_type, false, false, false, None);
            if let Some(parent_entity) = game_entities.get_mut(&parent_id) {
                if parent_barrel_index >= 10000 {
                    let turret_index = (parent_barrel_index / 10000) - 1;
                    let barrel_index = parent_barrel_index % 10000;
                    if turret_index < parent_entity.turrets.len() {
                        let turret = &mut parent_entity.turrets[turret_index];
                        if barrel_index < turret.barrels.len() {
                            turret.barrels[barrel_index].child_ids.insert(new_bullet_id);
                        }
                    }
                } else {
                    if parent_barrel_index != usize::MAX && parent_barrel_index < parent_entity.barrels.len() {
                        parent_entity.barrels[parent_barrel_index].child_ids.insert(new_bullet_id);
                    }
                }
            }
        }
        if STORE_COLLISIONS {
            let mut processed_collision_pairs = HashSet::new();
            for thread_index in 0..THREADS {
                for collision_pair in &world_room.stored_collisions[thread_index] {
                    let (id_a, id_b) = if collision_pair[0] < collision_pair[1] { (collision_pair[0], collision_pair[1]) } else { (collision_pair[1], collision_pair[0]) };
                    if !processed_collision_pairs.insert((id_a, id_b)) { continue; };
                    let mut kill_event_a = None;
                    let mut kill_event_b = None;
                    {
                        if let (Some(entity_a), Some(entity_b)) = (game_entities.get(&id_a), game_entities.get(&id_b)) {
                            let is_same_team = entity_a.team == entity_b.team && entity_a.team != 0 && entity_a.team != 255;
                            if !entity_a.dead && !entity_b.dead && !is_same_team {
                                let is_wall_a = entity_a.entity_type == ENTITY_WALL;
                                let is_bullet_a = entity_a.entity_type == ENTITY_BULLET || entity_a.entity_type == ENTITY_DRONE;
                                let is_wall_b = entity_b.entity_type == ENTITY_WALL;
                                let is_bullet_b = entity_b.entity_type == ENTITY_BULLET || entity_b.entity_type == ENTITY_DRONE;
                                if entity_a.parent_id == id_b || entity_b.parent_id == id_a { continue; };
                                if is_wall_a || is_wall_b {
                                    if is_wall_a && is_bullet_b { entity_b.health.store(0, Ordering::Relaxed) }
                                    else if is_wall_b && is_bullet_a { entity_a.health.store(0, Ordering::Relaxed) };
                                    continue;
                                }
                                if entity_a.invulnerable || entity_b.invulnerable { continue; };
                                let mut a_inside_safe_base = false;
                                if let Some(physics_a) = world_room.entities.get(id_a) {
                                    for base in &active_bases {
                                        if base.team_id == entity_a.team && base.w > NODE_SIZE * 1.5 && physics_a.x >= base.x && physics_a.x <= base.x + base.w && physics_a.y >= base.y && physics_a.y <= base.y + base.h {
                                            a_inside_safe_base = true; break;
                                        }
                                    }
                                }
                                let mut b_inside_safe_base = false;
                                if let Some(physics_b) = world_room.entities.get(id_b) {
                                    for base in &active_bases {
                                        if base.team_id == entity_b.team && base.w > NODE_SIZE * 1.5 && physics_b.x >= base.x && physics_b.x <= base.x + base.w && physics_b.y >= base.y && physics_b.y <= base.y + base.h {
                                            b_inside_safe_base = true; break;
                                        }
                                    }
                                }
                                let health_a = entity_a.health.load(Ordering::Relaxed);
                                let health_b = entity_b.health.load(Ordering::Relaxed);
                                let damage_a = (entity_a.damage as f32 * BODY_DAMAGE_MULT) as i32;
                                let damage_b = (entity_b.damage as f32 * BODY_DAMAGE_MULT) as i32;
                                let min_damage_a = (entity_a.max_health as f32 * COLLISION_MIN_IMPACT) as i32;
                                let min_damage_b = (entity_b.max_health as f32 * COLLISION_MIN_IMPACT) as i32;
                                let final_damage_to_a = damage_b.max(min_damage_a);
                                let final_damage_to_b = damage_a.max(min_damage_b);
                                if health_a != SENTINEL_HEALTH {
                                    let applied_damage = if a_inside_safe_base { 0 } else { final_damage_to_a };
                                    let new_health = health_a - applied_damage;
                                    entity_a.health.store(new_health, Ordering::Relaxed);
                                    if health_a > 0 && new_health <= 0 {
                                        if entity_a.entity_type == ENTITY_TANK || entity_a.entity_type == ENTITY_POLYGON {
                                            let victim_name_str = if entity_a.entity_type == ENTITY_POLYGON { "".to_string() } else if entity_a.name.is_empty() { "an unnamed entity".to_string() } else { entity_a.name.clone() };
                                            kill_event_a = Some((id_b, entity_a.score, victim_name_str));
                                        }
                                    }
                                }
                                if health_b != SENTINEL_HEALTH {
                                    let applied_damage = if b_inside_safe_base { 0 } else { final_damage_to_b };
                                    let new_health = health_b - applied_damage;
                                    entity_b.health.store(new_health, Ordering::Relaxed);
                                    if health_b > 0 && new_health <= 0 {
                                        if entity_b.entity_type == ENTITY_TANK || entity_b.entity_type == ENTITY_POLYGON {
                                            let victim_name_str = if entity_b.entity_type == ENTITY_POLYGON { "".to_string() } else if entity_b.name.is_empty() { "an unnamed entity".to_string() } else { entity_b.name.clone() };
                                            kill_event_b = Some((id_a, entity_b.score, victim_name_str));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    let mut process_kill = |killer_id: usize, score_value: u32, victim_name: String, victim_real_id: usize, dead_entity_id: usize| {
                        let mut current_killer_id = killer_id;
                        for _ in 0..5 {
                            if let Some(entity) = game_entities.get(&current_killer_id) {
                                if entity.parent_id != current_killer_id { current_killer_id = entity.parent_id; } else { break; };
                            } else { break; };
                        }
                        let killer_death_info = if let Some(killer) = game_entities.get(&current_killer_id) { Some(DeathCause { name: if killer.entity_type == ENTITY_POLYGON { tank_classes_definitions.get(&killer.class_id).unwrap().name.clone() } else if killer.name.is_empty() { "an unnamed entity".to_string() } else { killer.name.clone() }, class_id: killer.class_id, color: killer.color }) } else { None };
                        if let Some(scoring_entity) = game_entities.get_mut(&current_killer_id) {
                            let float_score = score_value as f32;
                            scoring_entity.score += ((float_score) / float_score.log2()) as u32;
                            if let Some(client_id) = scoring_entity.client_id {
                                if let Some(session) = player_sessions.get(&client_id) {
                                    if !victim_name.is_empty() { send_notification(&session.sender, &format!("You killed {}.", victim_name)); };
                                }
                            }
                        }
                        if let Some(info) = &killer_death_info {
                            if let Some(leader_id) = current_leader_entity_id {
                                if victim_real_id == leader_id {
                                    for session in player_sessions.values() {
                                        if session.is_connected { send_notification(&session.sender, &format!("{} was usurped by {}.", victim_name, info.name)); };
                                    }
                                }
                            }
                        }
                        if let Some(dead_entity) = game_entities.get_mut(&dead_entity_id) { dead_entity.death_cause = killer_death_info; };
                    };
                    if let Some((kid, score, name)) = kill_event_a { process_kill(kid, score, name, id_a, id_a); };
                    if let Some((kid, score, name)) = kill_event_b { process_kill(kid, score, name, id_b, id_b); };
                }
            }
        }
        let mut cleanup_entity_ids = Vec::new();
        let should_regen = current_tick_counter % 20 == 0;
        for (entity_id, game_entity) in &mut game_entities {
            let current_health = game_entity.health.load(Ordering::Relaxed);
            if current_health == SENTINEL_HEALTH || game_entity.dead { continue; };
            if current_health <= 0 { game_entity.dead = true; cleanup_entity_ids.push(*entity_id); continue; }
            if should_regen && current_health < game_entity.max_health {
                let new_health = (current_health + (game_entity.regen * 20)).min(game_entity.max_health);
                game_entity.health.store(new_health, Ordering::Relaxed);
            }
        }
        for entity_id in cleanup_entity_ids {
            let entity_hierarchy_data = if let Some(game_entity) = game_entities.get(&entity_id) {
                if let Some(client_id) = game_entity.client_id {
                     if let Some(session) = player_sessions.get_mut(&client_id) {
                         if let Some(physics_entity) = world_room.entities.get(entity_id) {
                             session.last_camera_pos = (physics_entity.x, physics_entity.y);
                             session.fov_factor = game_entity.fov_factor;
                         }
                         session.entity_id = None;
                         if session.is_connected {
                             let (killer_name, killer_color, killer_class) = if let Some(cause) = &game_entity.death_cause { (cause.name.clone(), cause.color, cause.class_id) } else { ("An unnamed entity".to_string(), 0, 0) };
                             let name_bytes = killer_name.as_bytes();
                             let safe_name_len = name_bytes.len().min(255) as u8;
                             let mut death_packet = vec![20];
                             death_packet.push(safe_name_len);
                             death_packet.extend(&name_bytes[0..safe_name_len as usize]);
                             death_packet.extend(game_entity.score.to_le_bytes());
                             let seconds_alive = session.entity_created_at.elapsed().as_secs_f32() as u32;
                             death_packet.extend(seconds_alive.to_le_bytes());
                             death_packet.push(killer_color);
                             death_packet.push(killer_class);
                             death_packet.push(game_entity.color);
                             death_packet.push(game_entity.class_id);
                             let _ = session.sender.send(death_packet);
                         }
                     }
                }
                (game_entity.entity_type, game_entity.parent_id, game_entity.parent_barrel_index)
            } else { (ENTITY_BULLET, usize::MAX, usize::MAX) };
            if entity_hierarchy_data.0 == ENTITY_DRONE {
                if let Some(parent_entity) = game_entities.get_mut(&entity_hierarchy_data.1) {
                    if entity_hierarchy_data.2 >= 10000 {
                        let turret_idx = (entity_hierarchy_data.2 / 10000) - 1;
                        let barrel_idx = entity_hierarchy_data.2 % 10000;
                        if turret_idx < parent_entity.turrets.len() {
                            if barrel_idx < parent_entity.turrets[turret_idx].barrels.len() {
                                parent_entity.turrets[turret_idx].barrels[barrel_idx].child_ids.remove(&entity_id);
                            }
                        }
                    } else if entity_hierarchy_data.2 != usize::MAX && entity_hierarchy_data.2 < parent_entity.barrels.len() {
                        parent_entity.barrels[entity_hierarchy_data.2].child_ids.remove(&entity_id);
                    }
                }
            }
            world_room.remove_entity(entity_id);
            game_entities.remove(&entity_id);
            bot_entity_ids.remove(&entity_id);
        }
        let event_warning_duration = (10000.0 / TICK_TIME as f32) as usize;
        match active_game_event.clone() {
            GameEvent::None => {
                if current_tick_counter > last_event_end_tick + 100 {
                     let random_event_roll = rng.gen_range(0..7);
                     if random_event_roll == 0 {
                         active_game_event = GameEvent::BossWarning { start_tick: current_tick_counter };
                         for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, "A visitor approaches..."); }; }
                     } else if random_event_roll == 1 {
                         active_game_event = GameEvent::EggWarning { start_tick: current_tick_counter };
                         for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, "An egg will appear soon..."); }; }
                     } else if random_event_roll == 2 {
                         active_game_event = GameEvent::KOTHWarning { start_tick: current_tick_counter };
                         for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, "King of the Hill is about to begin..."); }; }
                     } else if random_event_roll == 3 {
                         active_game_event = GameEvent::CelestialBattleWarning { start_tick: current_tick_counter };
                         for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, "Teams are recieving powerful Celestials..."); }; }
                     } else if random_event_roll == 4 {
                         active_game_event = GameEvent::BountyWarning { start_tick: current_tick_counter };
                         for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, "A bounty is being placed..."); }; }
                     } else if random_event_roll == 5 {
                         active_game_event = GameEvent::PaintJobWarning { start_tick: current_tick_counter };
                         for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, "The paint job event is about to begin: cover the map in your color!"); }; }
                     } else if random_event_roll == 6 {
                         active_game_event = GameEvent::MeteorShowerWarning { start_tick: current_tick_counter };
                         for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, "WARNING: Meteor shower detected! avoid the meteors!"); }; }
                     }
                }
            },
            GameEvent::BossWarning { start_tick } => {
                if current_tick_counter > start_tick + event_warning_duration {
                    let (boss_definition, boss_class_id, spawn_message) = if rng.gen_range(0.0..1.0) < 0.99 {
                        let selected_boss_id = BOSS_IDS[rng.gen_range(0..BOSS_IDS.len())];
                        let boss_class = tank_classes_definitions.get(&selected_boss_id).unwrap();
                        (boss_class, selected_boss_id, format!("A {} has spawned!", boss_class.name))
                     } else {
                        let selected_celestial_id = CELESTIAL_IDS[rng.gen_range(0..CELESTIAL_IDS.len())];
                        let celestial_class = tank_classes_definitions.get(&selected_celestial_id).unwrap();
                        (celestial_class, selected_celestial_id, format!("Celestial {} has spawned!", celestial_class.name))
                     };
                     let [spawn_x, spawn_y] = spawn_coordinate_generator(&mut rng, &active_bases, None);
                     let boss_team_id = 5;
                     let boss_color = if boss_definition.color != 255 { boss_definition.color } else { boss_team_id };
                     let spawned_boss_id = spawn_game_entity(&mut world_room, &mut game_entities, &tank_classes_definitions, spawn_x, spawn_y, 0.0, boss_definition.mass, 0.98, 0.0, 0.0, boss_definition.base_speed, 0.02, boss_definition.body_radius, 1, true, boss_definition.shape, boss_team_id, boss_color, boss_definition.id, boss_definition.barrels.clone(), boss_definition.name.clone(), boss_definition.score, true, None, 0, boss_definition.base_health, boss_definition.base_health, boss_definition.base_regen, boss_definition.base_damage, boss_definition.base_reload, boss_definition.fov_factor, 0.0, 0.0, current_tick_counter, usize::MAX, ENTITY_TANK, true, true, false, None);
                     bot_entity_ids.insert(spawned_boss_id);
                     active_game_event = GameEvent::BossFight { boss_id: spawned_boss_id };
                     for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, &spawn_message); }; }
                }
            },
            GameEvent::BossFight { boss_id } => {
                let mut is_boss_alive = false;
                if let Some(game_entity) = game_entities.get(&boss_id) { if !game_entity.dead && (BOSS_IDS.contains(&game_entity.class_id) || CELESTIAL_IDS.contains(&game_entity.class_id)) { is_boss_alive = true; }; }
                if !is_boss_alive {
                     for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, "The visitor has been defeated!"); }; }
                     last_event_end_tick = current_tick_counter; active_game_event = GameEvent::None;
                }
            },
            GameEvent::EggWarning { start_tick } => {
                 if current_tick_counter > start_tick + event_warning_duration {
                     let egg_definition = tank_classes_definitions.get(&CLASS_EGG).unwrap();
                     let center_x = ROOM_SIZE / 2.0;
                     let center_y = ROOM_SIZE / 2.0;
                     let spawned_egg_id = spawn_game_entity(&mut world_room, &mut game_entities, &tank_classes_definitions, center_x, center_y, 0.0, egg_definition.mass, 0.99, 0.0, 0.0, 0.0, 0.0, egg_definition.body_radius, 1, false, egg_definition.shape, 100, egg_definition.color, egg_definition.id, Vec::new(), "The Egg".to_string(), 0, false, None, 0, SENTINEL_HEALTH, SENTINEL_HEALTH, 0, 0, 0, 1.0, 0.0, 0.0, current_tick_counter, usize::MAX, ENTITY_TANK, false, false, false, None);
                     let hunt_timeout_ticks = (300000.0 / TICK_TIME as f32) as usize;
                     active_game_event = GameEvent::EggHunt { egg_id: spawned_egg_id, end_tick: current_tick_counter + hunt_timeout_ticks };
                     for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, "The egg has appeared! Push it to your base!"); }; }
                 }
            },
            GameEvent::EggHunt { egg_id, end_tick } => {
                let mut is_egg_captured = false;
                let mut capturing_team_id = 0;
                let mut does_egg_exist = false;
                if let Some(egg_physics) = world_room.entities.get(egg_id) {
                    does_egg_exist = true;
                    for base in &active_bases {
                        if base.team_id == 0 || base.team_id == 35 { continue; };
                        if egg_physics.x >= base.x && egg_physics.x <= base.x + base.w && egg_physics.y >= base.y && egg_physics.y <= base.y + base.h {
                            is_egg_captured = true; capturing_team_id = base.team_id; break;
                        }
                    }
                }
                if !does_egg_exist {
                     last_event_end_tick = current_tick_counter; active_game_event = GameEvent::None;
                } else if current_tick_counter > end_tick {
                     if let Some(egg_entity) = game_entities.get_mut(&egg_id) { egg_entity.dead = true; egg_entity.health.store(0, Ordering::Relaxed); };
                     last_event_end_tick = current_tick_counter; active_game_event = GameEvent::None;
                     for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, "The egg was lost in the void."); }; }
                } else if is_egg_captured {
                    if let Some(egg_entity) = game_entities.get_mut(&egg_id) { egg_entity.dead = true; egg_entity.health.store(0, Ordering::Relaxed); };
                    let reward_boss_id = BOSS_IDS[rng.gen_range(0..BOSS_IDS.len())];
                    let reward_definition = tank_classes_definitions.get(&reward_boss_id).unwrap();
                    let [spawn_x, spawn_y] = spawn_coordinate_generator(&mut rng, &active_bases, Some(capturing_team_id));
                    let reward_entity_id = spawn_game_entity(&mut world_room, &mut game_entities, &tank_classes_definitions, spawn_x, spawn_y, 0.0, reward_definition.mass, 0.98, 0.0, 0.0, reward_definition.base_speed, 0.02, reward_definition.body_radius, 1, true, reward_definition.shape, capturing_team_id, capturing_team_id, reward_definition.id, reward_definition.barrels.clone(), reward_definition.name.clone(), reward_definition.score, true, None, 0, reward_definition.base_health, reward_definition.base_health, reward_definition.base_regen, reward_definition.base_damage, reward_definition.base_reload, reward_definition.fov_factor, 0.0, 0.0, current_tick_counter, usize::MAX, ENTITY_TANK, true, true, false, None);
                    bot_entity_ids.insert(reward_entity_id);
                    active_game_event = GameEvent::EggReward { boss_id: reward_entity_id };
                    for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, &format!("The egg has been claimed! A powerful ally has come to aid the {} team!", yield_team_name(capturing_team_id))); }; }
                }
            },
            GameEvent::EggReward { boss_id } => {
                let mut is_reward_boss_active = false;
                if let Some(game_entity) = game_entities.get(&boss_id) { if !game_entity.dead && (BOSS_IDS.contains(&game_entity.class_id) || CELESTIAL_IDS.contains(&game_entity.class_id)) { is_reward_boss_active = true; }; }
                if !is_reward_boss_active {
                     for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, "The egg hatched team boss has fallen!"); }; }
                     last_event_end_tick = current_tick_counter; active_game_event = GameEvent::None;
                }
            },
            GameEvent::KOTHWarning { start_tick } => {
                if current_tick_counter > start_tick + event_warning_duration {
                    let center_grid_pos = SPATIAL_GRID_DIMENSION as f32 / 2.0;
                    let zone_tile_size = 5.0;
                    let half_zone_size = zone_tile_size / 2.0;
                    let zone_x = (center_grid_pos - half_zone_size) * NODE_SIZE;
                    let zone_y = (center_grid_pos - half_zone_size) * NODE_SIZE;
                    let zone_w = zone_tile_size * NODE_SIZE;
                    let zone_h = zone_tile_size * NODE_SIZE;
                    active_bases.push(BaseZone { x: zone_x, y: zone_y, w: zone_w, h: zone_h, team_id: 35, party_code: "KOTH".to_string() });
                    let mut base_update_packet = vec![11];
                    base_update_packet.extend((active_bases.len() as u16).to_le_bytes());
                    for base in &active_bases {
                        base_update_packet.extend(base.x.to_le_bytes());
                        base_update_packet.extend(base.y.to_le_bytes());
                        base_update_packet.extend(base.w.to_le_bytes());
                        base_update_packet.extend(base.h.to_le_bytes());
                        base_update_packet.push(base.team_id);
                    }
                    shared_base_packet = Arc::new(base_update_packet.clone());
                    for session in player_sessions.values() { if session.is_connected { let _ = session.sender.send(base_update_packet.clone()); }; }
                    let koth_event_duration = (30000.0 / TICK_TIME as f32) as usize;
                    active_game_event = GameEvent::KOTHActive { end_tick: current_tick_counter + koth_event_duration, zone_x, zone_y, zone_w, zone_h };
                    for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, "Control the center zone!"); }; }
                }
            },
            GameEvent::KOTHActive { end_tick, zone_x, zone_y, zone_w, zone_h } => {
                if current_tick_counter >= end_tick {
                    active_bases.pop();
                    let mut base_update_packet = vec![11];
                    base_update_packet.extend((active_bases.len() as u16).to_le_bytes());
                    for base in &active_bases {
                        base_update_packet.extend(base.x.to_le_bytes());
                        base_update_packet.extend(base.y.to_le_bytes());
                        base_update_packet.extend(base.w.to_le_bytes());
                        base_update_packet.extend(base.h.to_le_bytes());
                        base_update_packet.push(base.team_id);
                    }
                    shared_base_packet = Arc::new(base_update_packet.clone());
                    for session in player_sessions.values() { if session.is_connected { let _ = session.sender.send(base_update_packet.clone()); }; }
                    last_event_end_tick = current_tick_counter; active_game_event = GameEvent::None;
                    for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, "King of the Hill has ended!"); }; }
                } else {
                    for (entity_id, game_entity) in &mut game_entities {
                        if !game_entity.dead && game_entity.entity_type == ENTITY_TANK {
                            if let Some(physics_entity) = world_room.entities.get(*entity_id) {
                                if physics_entity.x >= zone_x && physics_entity.x <= zone_x + zone_w && physics_entity.y >= zone_y && physics_entity.y <= zone_y + zone_h {
                                    game_entity.score += 100;
                                }
                            }
                        }
                    }
                }
            },
            GameEvent::CelestialBattleWarning { start_tick } => {
                if current_tick_counter > start_tick + event_warning_duration {
                    let mut available_celestial_ids = CELESTIAL_IDS.to_vec();
                    available_celestial_ids.shuffle(&mut rng);
                    let mut spawned_celestial_list = Vec::new();
                    let mut handled_team_ids = HashSet::new();
                    for base in &active_bases {
                        let team_id = base.team_id;
                        if team_id == 0 || team_id == 35 || team_id == 255 { continue; };
                        if !handled_team_ids.insert(team_id) { continue; };
                        if let Some(celestial_id) = available_celestial_ids.pop() {
                            let celestial_definition = tank_classes_definitions.get(&celestial_id).unwrap();
                            let [spawn_x, spawn_y] = spawn_coordinate_generator(&mut rng, &active_bases, Some(team_id));
                            let celestial_entity_id = spawn_game_entity(&mut world_room, &mut game_entities, &tank_classes_definitions, spawn_x, spawn_y, 0.0, celestial_definition.mass, 0.98, 0.0, 0.0, celestial_definition.base_speed, 0.02, celestial_definition.body_radius, 1, true, celestial_definition.shape, team_id, team_id, celestial_definition.id, celestial_definition.barrels.clone(), celestial_definition.name.clone(), celestial_definition.score, true, None, 0, celestial_definition.base_health, celestial_definition.base_health, celestial_definition.base_regen, celestial_definition.base_damage, celestial_definition.base_reload, celestial_definition.fov_factor, 0.0, 0.0, current_tick_counter, usize::MAX, ENTITY_TANK, true, true, false, None);
                            bot_entity_ids.insert(celestial_entity_id);
                            spawned_celestial_list.push(celestial_entity_id);
                        }
                    }
                    active_game_event = GameEvent::CelestialBattleFight { celestial_ids: spawned_celestial_list };
                    for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, "Celestials have spawned! Defend yours!"); }; }
                }
            },
            GameEvent::CelestialBattleFight { celestial_ids } => {
                let mut surviving_teams = HashSet::new();
                let mut last_survivor_id = None;
                for &id in &celestial_ids { if let Some(game_entity) = game_entities.get(&id) { if !game_entity.dead { surviving_teams.insert(game_entity.team); last_survivor_id = Some(id); } } }
                if surviving_teams.len() == 1 {
                    if let Some(survivor_id) = last_survivor_id {
                         let winning_team_id = game_entities.get(&survivor_id).map(|ge| ge.team).unwrap_or(0);
                         active_game_event = GameEvent::CelestialBattleReward { start_tick: current_tick_counter, winning_team: winning_team_id, surviving_id: survivor_id };
                         for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, &format!("The last celestial is standing... the {} team has won the event!", yield_team_name(winning_team_id))); }; }
                    }
                } else if surviving_teams.is_empty() {
                    last_event_end_tick = current_tick_counter; active_game_event = GameEvent::None;
                    for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, "All celestials destroyed."); }; }
                }
            },
            GameEvent::CelestialBattleReward { start_tick, winning_team, surviving_id } => {
                let reward_wait_ticks = (5000.0 / TICK_TIME as f32) as usize;
                if current_tick_counter > start_tick + reward_wait_ticks {
                     if let Some(surviving_entity) = game_entities.get_mut(&surviving_id) { surviving_entity.dead = true; surviving_entity.health.store(0, Ordering::Relaxed); };
                     for game_entity in game_entities.values_mut() { if !game_entity.dead && game_entity.entity_type == ENTITY_TANK && game_entity.team == winning_team && game_entity.client_id.is_some() { game_entity.score += 50000; }; }
                     last_event_end_tick = current_tick_counter; active_game_event = GameEvent::None;
                     for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, &format!("Event Over. All members of the {} team have been rewarded 50k Score!", yield_team_name(winning_team))); }; }
                }
            },
            GameEvent::BountyWarning { start_tick } => {
                if current_tick_counter > start_tick + event_warning_duration {
                    let center_x = ROOM_SIZE / 2.0;
                    let center_y = ROOM_SIZE / 2.0;
                    let bounty_entity_id = spawn_game_entity(&mut world_room, &mut game_entities, &tank_classes_definitions, center_x, center_y, 0.0, 100.0, 0.9, 0.0, 0.0, 0.0, 0.0, 25.0, 1, false, 0, 255, 3, 0, Vec::new(), "Bounty".to_string(), 0, false, None, 0, SENTINEL_HEALTH, SENTINEL_HEALTH, 0, 0, 0, 1.0, 0.0, 0.0, current_tick_counter, usize::MAX, ENTITY_TANK, false, false, true, None);
                    let bounty_timeout_ticks = (60000.0 / TICK_TIME as f32) as usize;
                    active_game_event = GameEvent::BountyClaiming { claim_id: bounty_entity_id, end_tick: current_tick_counter + bounty_timeout_ticks };
                    for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, "The bounty has appeared at the center!"); }; }
                }
            },
            GameEvent::BountyClaiming { claim_id, end_tick } => {
                let mut bounty_claimer_id = None;
                let mut bounty_orb_exists = false;
                if let Some(orb_physics) = world_room.entities.get(claim_id) {
                    bounty_orb_exists = true;
                    for session in player_sessions.values() {
                        if !session.is_connected { continue; };
                        if let Some(player_entity_id) = session.entity_id {
                             if let Some(player_physics) = world_room.entities.get(player_entity_id) {
                                 let diff_x = player_physics.x - orb_physics.x;
                                 let diff_y = player_physics.y - orb_physics.y;
                                 let distance = (diff_x*diff_x + diff_y*diff_y).sqrt();
                                 if distance < (player_physics.radius + orb_physics.radius) { bounty_claimer_id = Some(player_entity_id); break; }
                             }
                        }
                    }
                }
                if !bounty_orb_exists {
                    last_event_end_tick = current_tick_counter; active_game_event = GameEvent::None;
                } else if let Some(player_id) = bounty_claimer_id {
                    if let Some(game_entity) = game_entities.get_mut(&player_id) {
                        game_entity.team = 3;
                        game_entity.color = 3;
                        game_entity.score = 1000000;
                        game_entity.stat_points = 0;
                        game_entity.stats = [MAX_STAT_LEVEL; STAT_COUNT];
                        game_entity.health.store(game_entity.max_health, Ordering::Relaxed);
                        if tank_classes_definitions.contains_key(&game_entity.class_id) {
                             let definition = tank_classes_definitions.get(&game_entity.class_id).unwrap();
                             apply_entity_stats(game_entity, definition);
                        }
                        if let Some(client_id) = game_entity.client_id { if let Some(session) = player_sessions.get(&client_id) { let mut stats_packet = vec![9]; stats_packet.push(game_entity.stat_points); stats_packet.extend(&game_entity.stats); let _ = session.sender.send(stats_packet); } }
                    }
                    if let Some(bounty_orb) = game_entities.get_mut(&claim_id) { bounty_orb.dead = true; bounty_orb.health.store(0, Ordering::Relaxed); };
                    active_game_event = GameEvent::BountyHunted { target_id: player_id };
                    for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, "A player has assumed the bounty! Hunt them down!"); }; }
                } else if current_tick_counter > end_tick {
                    if let Some(bounty_orb) = game_entities.get_mut(&claim_id) { bounty_orb.dead = true; bounty_orb.health.store(0, Ordering::Relaxed); };
                    last_event_end_tick = current_tick_counter; active_game_event = GameEvent::None;
                    for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, "The bounty vanished unclaimed."); }; }
                }
            },
            GameEvent::BountyHunted { target_id } => {
                let mut is_target_alive = false;
                if let Some(game_entity) = game_entities.get(&target_id) { if !game_entity.dead { is_target_alive = true; }; }
                if !is_target_alive {
                    last_event_end_tick = current_tick_counter; active_game_event = GameEvent::None;
                    for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, "The bounty has been claimed!"); }; }
                }
            },
            GameEvent::PaintJobWarning { start_tick } => {
                if current_tick_counter > start_tick + event_warning_duration {
                    saved_base_configuration = active_bases.clone();
                    active_bases.clear();
                    let mut empty_base_packet = vec![11];
                    empty_base_packet.extend((0u16).to_le_bytes());
                    let packet_arc = Arc::new(empty_base_packet);
                    for session in player_sessions.values() { if session.is_connected { let _ = session.sender.send(packet_arc.to_vec()); }; }
                    let paint_job_duration = (60000.0 / TICK_TIME as f32) as usize;
                    active_game_event = GameEvent::PaintJobActive { end_tick: current_tick_counter + paint_job_duration };
                    for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, "The paint job event has begun! The winning team will receive 50k score!"); }; }
                }
            },
            GameEvent::PaintJobActive { end_tick } => {
                if current_tick_counter >= end_tick {
                    let mut team_tile_counts: HashMap<u8, u32> = HashMap::new();
                    for base in &active_bases { *team_tile_counts.entry(base.team_id).or_insert(0) += 1; };
                    let mut winning_team = 0;
                    let mut max_tiles = 0;
                    for (team, count) in team_tile_counts { if count > max_tiles { max_tiles = count; winning_team = team; } }
                    if winning_team != 0 {
                        for game_entity in game_entities.values_mut() { if !game_entity.dead && game_entity.team == winning_team { game_entity.score += 50000; }; }
                        let victory_message = format!("The {} team wins! 50k score has been rewarded to everyone on the team!", yield_team_name(winning_team));
                        for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, &victory_message); }; }
                    } else {
                        for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, "The paint job event has resulted in a tie!"); }; }
                    }
                    active_bases = saved_base_configuration.clone();
                    saved_base_configuration.clear();
                    let mut restore_base_packet = vec![11];
                    restore_base_packet.extend((active_bases.len() as u16).to_le_bytes());
                    for base in &active_bases {
                        restore_base_packet.extend(base.x.to_le_bytes());
                        restore_base_packet.extend(base.y.to_le_bytes());
                        restore_base_packet.extend(base.w.to_le_bytes());
                        restore_base_packet.extend(base.h.to_le_bytes());
                        restore_base_packet.push(base.team_id);
                    }
                    let packet_arc = Arc::new(restore_base_packet);
                    for session in player_sessions.values() { if session.is_connected { let _ = session.sender.send(packet_arc.to_vec()); }; }
                    last_event_end_tick = current_tick_counter; active_game_event = GameEvent::None;
                } else {
                    let mut has_base_changed = false;
                    for (entity_id, game_entity) in &game_entities {
                        if !game_entity.dead && game_entity.entity_type == ENTITY_TANK && game_entity.team != 0 && game_entity.team != 255 {
                             if let Some(physics_entity) = world_room.entities.get(*entity_id) {
                                 let grid_x = (physics_entity.x / NODE_SIZE).floor();
                                 let grid_y = (physics_entity.y / NODE_SIZE).floor();
                                 let base_x = grid_x * NODE_SIZE;
                                 let base_y = grid_y * NODE_SIZE;
                                 if grid_x >= 0.0 && grid_x < SPATIAL_GRID_DIMENSION as f32 && grid_y >= 0.0 && grid_y < SPATIAL_GRID_DIMENSION as f32 {
                                     let mut found_base_index = None;
                                     for (index, base) in active_bases.iter().enumerate() { if (base.x - base_x).abs() < 1.0 && (base.y - base_y).abs() < 1.0 { found_base_index = Some(index); break; } }
                                     if let Some(index) = found_base_index {
                                         if active_bases[index].team_id != game_entity.team { active_bases[index].team_id = game_entity.team; has_base_changed = true; }
                                     } else {
                                         active_bases.push(BaseZone { x: base_x, y: base_y, w: NODE_SIZE, h: NODE_SIZE, team_id: game_entity.team, party_code: "".to_string() }); has_base_changed = true;
                                     }
                                 }
                             }
                        }
                    }
                    if has_base_changed {
                        let mut base_update_packet = vec![11];
                        base_update_packet.extend((active_bases.len() as u16).to_le_bytes());
                        for base in &active_bases {
                            base_update_packet.extend(base.x.to_le_bytes());
                            base_update_packet.extend(base.y.to_le_bytes());
                            base_update_packet.extend(base.w.to_le_bytes());
                            base_update_packet.extend(base.h.to_le_bytes());
                            base_update_packet.push(base.team_id);
                        }
                        let packet_arc = Arc::new(base_update_packet);
                        for session in player_sessions.values() { if session.is_connected { let _ = session.sender.send(packet_arc.to_vec()); }; }
                    }
                }
            },
            GameEvent::MeteorShowerWarning { start_tick } => {
                if current_tick_counter > start_tick + event_warning_duration {
                    let shower_duration = (60000.0 / TICK_TIME as f32) as usize;
                    for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, "The meteor shower has begun!"); }; }
                    active_game_event = GameEvent::MeteorShowerActive { end_tick: current_tick_counter + shower_duration, next_spawn_tick: current_tick_counter, meteor_ids: Vec::new() };
                }
            },
            GameEvent::MeteorShowerActive { end_tick, mut next_spawn_tick, mut meteor_ids } => {
                if current_tick_counter >= end_tick {
                    last_event_end_tick = current_tick_counter; active_game_event = GameEvent::None;
                    for session in player_sessions.values() { if session.is_connected { send_notification(&session.sender, "The meteor shower has ended!"); }; }
                    for id in meteor_ids {
                        world_room.remove_entity(id);
                        game_entities.remove(&id);
                    }
                } else {
                    if current_tick_counter >= next_spawn_tick {
                        let meteor_spawn_count = 1;
                        let meteor_definition = tank_classes_definitions.get(&CLASS_METEOR).unwrap();
                        let map_width = ROOM_SIZE;
                        let map_height = ROOM_SIZE;
                        let spawn_buffer = meteor_definition.body_radius + 5.0;
                        for _ in 0..meteor_spawn_count {
                            let spawn_side = rng.gen_range(0..4);
                            let (spawn_x, spawn_y, target_x_min, target_x_max, target_y_min, target_y_max) = match spawn_side {
                                0 => (rng.gen_range(spawn_buffer..map_width-spawn_buffer), -spawn_buffer, 0.0, map_width, map_height, map_height + spawn_buffer),
                                1 => (map_width + spawn_buffer, rng.gen_range(spawn_buffer..map_height-spawn_buffer), -spawn_buffer, 0.0, 0.0, map_height),
                                2 => (rng.gen_range(spawn_buffer..map_width-spawn_buffer), map_height + spawn_buffer, 0.0, map_width, -spawn_buffer, 0.0),
                                _ => (-spawn_buffer, rng.gen_range(spawn_buffer..map_height-spawn_buffer), map_width, map_width + spawn_buffer, 0.0, map_height),
                            };
                            let target_x = rng.gen_range(target_x_min..target_x_max);
                            let target_y = rng.gen_range(target_y_min..target_y_max);
                            let diff_x = target_x - spawn_x;
                            let diff_y = target_y - spawn_y;
                            let distance = (diff_x*diff_x + diff_y*diff_y).sqrt();
                            let meteor_speed = meteor_definition.base_speed * rng.gen_range(1.0..1.5);
                            let velocity_x = (diff_x / distance) * meteor_speed;
                            let velocity_y = (diff_y / distance) * meteor_speed;
                            let meteor_id = spawn_game_entity(&mut world_room, &mut game_entities, &tank_classes_definitions, spawn_x, spawn_y, 0.0, meteor_definition.mass, 0.0, velocity_x, velocity_y, meteor_speed, 0.0, meteor_definition.body_radius, 1, false, meteor_definition.shape, TEAM_METEOR, TEAM_METEOR, meteor_definition.id, vec![], "Meteor".to_string(), 0, false, None, 0, meteor_definition.base_health, meteor_definition.base_health, 0, meteor_definition.base_damage, 0, 1.0, meteor_definition.auto_spin, 0.0, current_tick_counter, usize::MAX, ENTITY_TANK, false, false, false, None);
                            meteor_ids.push(meteor_id);
                        }
                        next_spawn_tick = current_tick_counter + rng.gen_range(60..300);
                        active_game_event = GameEvent::MeteorShowerActive { end_tick, next_spawn_tick, meteor_ids };
                    }
                }
            }
        }
        let polygon_count = game_entities.values().filter(|ge| ge.entity_type == ENTITY_POLYGON).count();
        if polygon_count < MAX_POLYGONS {
             let [spawn_x, spawn_y] = spawn_coordinate_generator(&mut rng, &active_bases, None);
             let spawn_roll = rng.gen_range(0..100);
             let poly_class_id = if spawn_roll < 5 { CLASS_CRASHER } else if spawn_roll < 15 { CLASS_HEXAGON } else if spawn_roll < 35 { CLASS_PENTAGON } else if spawn_roll < 65 { CLASS_TRIANGLE } else { CLASS_SQUARE };
             if tank_classes_definitions.contains_key(&poly_class_id) {
                 let poly_definition = tank_classes_definitions.get(&poly_class_id).unwrap();
                 let is_crasher_type = poly_class_id == CLASS_CRASHER;
                 let is_bot_entity = is_crasher_type;
                 let spin_direction = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
                 let spin_speed = if is_crasher_type { 0.0 } else { rng.gen_range(0.005..0.01) * spin_direction };
                 spawn_game_entity(&mut world_room, &mut game_entities, &tank_classes_definitions, spawn_x, spawn_y, rng.gen_range(0.0..6.28), poly_definition.mass, 0.99, 0.0, 0.0, poly_definition.base_speed, 0.05, poly_definition.body_radius, 1, true, poly_definition.shape, TEAM_POLYGON, poly_definition.color, poly_definition.id, Vec::new(), "".to_string(), poly_definition.score, false, None, 0, poly_definition.base_health, poly_definition.base_health, 0, poly_definition.base_damage, 0, 1.0, spin_speed, 0.0, current_tick_counter, usize::MAX, ENTITY_POLYGON, is_bot_entity, true, false, None);
            }
        }
        world_room.update();
        let mut expired_entity_ids = Vec::with_capacity(20);
        for (entity_id, game_entity) in game_entities.iter() {
            if game_entity.lifetime != usize::MAX {
                let entity_age = current_tick_counter.saturating_sub(game_entity.creation_tick);
                if entity_age >= game_entity.lifetime { expired_entity_ids.push(*entity_id); };
            }
        }
        let mut disconnected_session_ids = Vec::new();
        for (client_id, session) in &player_sessions {
            if !session.is_connected {
                let mut should_close_session = false;
                if let Some(entity_id) = session.entity_id { if let Some(game_entity) = game_entities.get(&entity_id) { if game_entity.invulnerable { should_close_session = true; expired_entity_ids.push(entity_id); } } }
                if !should_close_session { if let Some(disconnect_time) = session.disconnect_at { if disconnect_time.elapsed() > Duration::from_secs(30) { should_close_session = true; if let Some(entity_id) = session.entity_id { expired_entity_ids.push(entity_id); }; } } }
                if should_close_session { disconnected_session_ids.push(*client_id); ip_to_client_map.remove(&session.ip); }
            }
        }
        for id in disconnected_session_ids { player_sessions.remove(&id); };
        for entity_id in expired_entity_ids {
            let entity_hierarchy_data = if let Some(game_entity) = game_entities.get(&entity_id) { (game_entity.entity_type, game_entity.parent_id, game_entity.parent_barrel_index) } else { (ENTITY_BULLET, usize::MAX, usize::MAX) };
            if entity_hierarchy_data.0 == ENTITY_DRONE { if let Some(parent_entity) = game_entities.get_mut(&entity_hierarchy_data.1) { if entity_hierarchy_data.2 != usize::MAX && entity_hierarchy_data.2 < parent_entity.barrels.len() { parent_entity.barrels[entity_hierarchy_data.2].child_ids.remove(&entity_id); }; } }
            world_room.remove_entity(entity_id); game_entities.remove(&entity_id); bot_entity_ids.remove(&entity_id);
        }
        let mut leaderboard_packet = vec![7];
        let mut scored_entity_list: Vec<(usize, &String, u32, u8, u8)> = game_entities.values().filter(|ge| !ge.dead && ge.render_score && ge.entity_type == ENTITY_TANK).map(|ge| (ge.parent_id, &ge.name, ge.score, ge.class_id, ge.color)).collect();
        scored_entity_list.sort_by(|a, b| b.2.cmp(&a.2));
        let top_10_players = scored_entity_list.iter().take(10).collect::<Vec<_>>();
        leaderboard_packet.push(top_10_players.len() as u8);
        for (_, player_name, player_score, class_id, color_id) in &top_10_players {
            leaderboard_packet.push(*class_id);
            leaderboard_packet.push(*color_id);
            let name_bytes = player_name.as_bytes();
            let safe_name_len = name_bytes.len().min(255) as u8;
            leaderboard_packet.push(safe_name_len);
            leaderboard_packet.extend(&name_bytes[0..safe_name_len as usize]);
            leaderboard_packet.extend(player_score.to_le_bytes());
        }
        let shared_leaderboard_packet = Arc::new(leaderboard_packet);
        if !top_10_players.is_empty() {
            let new_leader_id = top_10_players[0].0;
            if let Some(current_leader) = current_leader_entity_id {
                if new_leader_id != current_leader { current_leader_entity_id = Some(new_leader_id); };
            } else {
                current_leader_entity_id = Some(new_leader_id);
            }
        }
        let mut entity_packet_cache: HashMap<usize, Vec<u8>> = HashMap::with_capacity(game_entities.len());
        for (entity_id, game_entity) in &mut game_entities {
            if game_entity.dead { continue; };
            if let Some(physics_entity) = world_room.entities.get(*entity_id) {
                game_entity.messages.retain(|m| m.created_at.elapsed().as_millis() < m.duration);
                let mut update_packet = Vec::with_capacity(128);
                update_packet.extend((physics_entity.index as u32).to_le_bytes());
                update_packet.extend(physics_entity.x.to_le_bytes());
                update_packet.extend(physics_entity.y.to_le_bytes());
                update_packet.extend(physics_entity.radius.to_le_bytes());
                update_packet.push(physics_entity.body_type);
                update_packet.push(game_entity.shape);
                update_packet.push(game_entity.color);
                update_packet.push(game_entity.class_id);
                update_packet.extend(game_entity.score.to_le_bytes());
                update_packet.push(if game_entity.render_score { 1 } else { 0 });
                let name_bytes = game_entity.name.as_bytes();
                let safe_name_len = name_bytes.len().min(255) as u8;
                update_packet.push(safe_name_len);
                update_packet.extend(&name_bytes[0..safe_name_len as usize]);
                let current_health = game_entity.health.load(Ordering::Relaxed);
                let should_show_health = game_entity.should_render_health && current_health != SENTINEL_HEALTH && current_health > 0;
                update_packet.push(if should_show_health { 1 } else { 0 });
                if should_show_health {
                    let health_fraction = current_health as f32 / game_entity.max_health as f32;
                    update_packet.extend(health_fraction.to_le_bytes());
                }
                update_packet.extend(game_entity.facing.to_le_bytes());
                update_packet.push(if game_entity.invulnerable { 1 } else { 0 });
                update_packet.extend((game_entity.barrels.len() as u32).to_le_bytes());
                for barrel in &game_entity.barrels {
                    update_packet.extend(barrel.angle_offset.to_le_bytes());
                    update_packet.extend(barrel.offset_forward.to_le_bytes());
                    update_packet.extend(barrel.offset_side.to_le_bytes());
                    update_packet.extend(barrel.recoil.to_le_bytes());
                    update_packet.extend(barrel.spread.to_le_bytes());
                    update_packet.push(barrel.layer);
                    update_packet.push(barrel.vertices.len() as u8);
                    for v in &barrel.vertices {
                        update_packet.extend(v[0].to_le_bytes());
                        update_packet.extend(v[1].to_le_bytes());
                    }
                }
                update_packet.push(game_entity.turrets.len() as u8);
                for turret in &game_entity.turrets {
                    update_packet.extend(turret.facing.to_le_bytes());
                    update_packet.push(turret.barrels.len() as u8);
                    for barrel in &turret.barrels {
                        update_packet.push(barrel.vertices.len() as u8);
                        for v in &barrel.vertices {
                            update_packet.extend(v[0].to_le_bytes());
                            update_packet.extend(v[1].to_le_bytes());
                        }
                    }
                }
                update_packet.push(game_entity.layer_facings.len() as u8);
                for layer_facing in &game_entity.layer_facings { update_packet.extend(layer_facing.to_le_bytes()); };
                update_packet.push(game_entity.messages.len() as u8);
                for message in &game_entity.messages {
                    let message_bytes = message.text.as_bytes();
                    let message_len = message_bytes.len().min(255) as u8;
                    update_packet.push(message_len);
                    update_packet.extend(&message_bytes[0..message_len as usize]);
                }
                entity_packet_cache.insert(*entity_id, update_packet);
            }
        }
        let tick_duration = tick_start_time.elapsed();
        let ms_per_tick = tick_duration.as_secs_f32() * 1000.0;
        let connected_client_count = player_sessions.values().filter(|s| s.is_connected).count() as u32;
        let total_entity_count = game_entities.len() as u32;
        let mut server_stats_packet = vec![12];
        server_stats_packet.extend(ms_per_tick.to_le_bytes());
        server_stats_packet.extend(total_entity_count.to_le_bytes());
        server_stats_packet.extend(connected_client_count.to_le_bytes());
        let shared_stats_packet = Arc::new(server_stats_packet);
        for (client_id, session) in &player_sessions {
            if !session.is_connected { continue; };
            let player_team_id = session.saved_team;
            let mut minimap_packet = vec![5];
            let mut minimap_entities = Vec::new();
            for (entity_id, game_entity) in &game_entities {
                if !game_entity.dead && game_entity.entity_type == ENTITY_TANK {
                    let is_boss_entity = BOSS_IDS.contains(&game_entity.class_id) || CELESTIAL_IDS.contains(&game_entity.class_id);
                    if (session.entity_id.map_or(false, |eid| eid == *entity_id)) || (game_entity.team == player_team_id && game_entity.team != 0 && game_entity.team != 255) || is_boss_entity || game_entity.class_id == CLASS_EGG || game_entity.color == 3 {
                        if let Some(physics_entity) = world_room.entities.get(*entity_id) { minimap_entities.push((*entity_id, physics_entity.x, physics_entity.y, game_entity.color)); };
                    }
                }
            }
            minimap_packet.extend((minimap_entities.len() as u32).to_le_bytes());
            for (map_id, map_x, map_y, map_color) in minimap_entities {
                minimap_packet.extend((map_id as u32).to_le_bytes());
                minimap_packet.extend(map_x.to_le_bytes());
                minimap_packet.extend(map_y.to_le_bytes());
                minimap_packet.push(map_color);
            }
            let mut world_update_packet = vec![1];
            let mut camera_x = session.last_camera_pos.0;
            let mut camera_y = session.last_camera_pos.1;
            let mut fov_multiplier = session.fov_factor;
            if let Some(player_entity_id) = session.entity_id {
                if let Some(player_physics) = world_room.entities.get(player_entity_id) {
                    camera_x = player_physics.x;
                    camera_y = player_physics.y;
                    if let Some(game_entity) = game_entities.get(&player_entity_id) { fov_multiplier = game_entity.fov_factor; };
                }
            }
            let view_width_half = (BASE_VIEW_WIDTH / 2.0) * fov_multiplier;
            let view_height_half = (BASE_VIEW_HEIGHT / 2.0) * fov_multiplier;
            let viewport_min_x = camera_x - view_width_half - VIEW_PADDING;
            let viewport_max_x = camera_x + view_width_half + VIEW_PADDING;
            let viewport_min_y = camera_y - view_height_half - VIEW_PADDING;
            let viewport_max_y = camera_y + view_height_half + VIEW_PADDING;
            for entity in &world_room.entities {
                if !entity.replace {
                    if (entity.x + entity.radius) < viewport_min_x || (entity.x - entity.radius) > viewport_max_x || (entity.y + entity.radius) < viewport_min_y || (entity.y - entity.radius) > viewport_max_y { continue; };
                    if let Some(packet_data) = entity_packet_cache.get(&entity.index) { world_update_packet.extend(packet_data); };
                }
            }
            let _ = session.sender.send(world_update_packet);
            let _ = session.sender.send(minimap_packet);
            let _ = session.sender.send(shared_leaderboard_packet.to_vec());
            let _ = session.sender.send(shared_stats_packet.to_vec());
        }
        if tick_duration < Duration::from_millis(TICK_TIME) { sleep(Duration::from_millis(TICK_TIME) - tick_duration).await; };
    }
}

fn handle_input<F>(entity_id: usize, packet_data: Vec<u8>, world_room: &mut Room, game_entities: &mut HashMap<usize, GameEntity>, tank_classes_definitions: &HashMap<u8, EntityClass>, rng: &mut ThreadRng, spawn_coordinate_generator: &mut F, current_tick_counter: usize, player_sessions: &mut HashMap<usize, PlayerSession>, active_bases: &[BaseZone]) where F: FnMut(&mut ThreadRng, &[BaseZone], Option<u8>) -> [f32; 2] {
    let is_respawn_request = packet_data[0] == 4;
    if is_respawn_request {
        if let Some(session) = player_sessions.get_mut(&entity_id) {
             let starting_definition = tank_classes_definitions.get(&0).unwrap();
             let [respawn_x, respawn_y] = spawn_coordinate_generator(rng, active_bases, Some(session.saved_team));
             let team_id = session.saved_team;
             let player_name = session.saved_name.clone();
             let new_entity_id = spawn_game_entity(world_room, game_entities, tank_classes_definitions, respawn_x, respawn_y, 0.0, starting_definition.mass, 0.95, 0.0, 0.0, starting_definition.base_speed, 0.05, starting_definition.body_radius, 1, true, starting_definition.shape, team_id, if starting_definition.color != 255 { starting_definition.color } else { team_id }, starting_definition.id, starting_definition.barrels.clone(), player_name, 26000, true, None, 0, starting_definition.base_health, starting_definition.base_health, starting_definition.base_regen, starting_definition.base_damage, starting_definition.base_reload, starting_definition.fov_factor, 0.0, 0.0, current_tick_counter, usize::MAX, ENTITY_TANK, false, true, true, Some(session.client_id));
            if let Some(game_entity) = game_entities.get_mut(&new_entity_id) { game_entity.stat_points = 45; };
            session.entity_id = Some(new_entity_id);
            session.last_camera_pos = (respawn_x, respawn_y);
            session.fov_factor = 1.0;
            session.entity_created_at = Instant::now();
            let mut camera_packet = vec![2];
            camera_packet.extend((new_entity_id as u64).to_le_bytes());
            let _ = session.sender.send(camera_packet);
            let mut upgrades_packet = vec![4];
            upgrades_packet.extend((new_entity_id as u64).to_le_bytes());
            upgrades_packet.push(starting_definition.upgrades.len() as u8);
            upgrades_packet.extend(&starting_definition.upgrades);
            let _ = session.sender.send(upgrades_packet);
            let mut stats_packet = vec![9];
            stats_packet.push(45);
            stats_packet.extend(&[0u8; STAT_COUNT]);
            let _ = session.sender.send(stats_packet);
        }
        return;
    }
    match packet_data[0] {
        0 => {
            if let Some(game_entity) = game_entities.get_mut(&entity_id) {
                if packet_data.len() >= 13 {
                    let angle_bytes: [u8; 4] = packet_data[1..5].try_into().unwrap();
                    let world_x_bytes: [u8; 4] = packet_data[5..9].try_into().unwrap();
                    let world_y_bytes: [u8; 4] = packet_data[9..13].try_into().unwrap();
                    if game_entity.spin_rate.abs() < f32::EPSILON && !game_entity.auto_spin_enabled {
                        game_entity.facing = f32::from_le_bytes(angle_bytes);
                    }
                    game_entity.target_pos = [f32::from_le_bytes(world_x_bytes), f32::from_le_bytes(world_y_bytes)];
                } else if packet_data.len() >= 5 {
                    if game_entity.spin_rate.abs() < f32::EPSILON && !game_entity.auto_spin_enabled {
                        let angle_bytes: [u8; 4] = packet_data[1..5].try_into().unwrap();
                        game_entity.facing = f32::from_le_bytes(angle_bytes);
                    }
                }
            }
        },
        1 => {
            if let Some(_physics_entity) = world_room.entities.get_mut(entity_id) {
                let direction_code = packet_data[1];
                if direction_code != 16 {
                    if let Some(game_entity) = game_entities.get_mut(&entity_id) { game_entity.invulnerable = false; };
                }
                if direction_code > 1 && direction_code < 4 { world_room.stop_entity_movement_x(entity_id) }
                else if direction_code < 2 { world_room.stop_entity_movement_y(entity_id) }
                else if direction_code == 16 { world_room.stop_entity_movement(entity_id) };
                world_room.create_entity_movement_from_cardinal_direction(entity_id, direction_code);
            }
        },
        2 => {
            let is_firing = if packet_data.len() > 1 { packet_data[1] == 1 } else { false };
            if let Some(game_entity) = game_entities.get_mut(&entity_id) { game_entity.is_firing = is_firing; }
        },
        3 => {
            let target_class_id = packet_data[1];
            let mut is_upgrade_valid = false;
            if let Some(game_entity) = game_entities.get(&entity_id) {
                if tank_classes_definitions.contains_key(&game_entity.class_id) {
                    let current_definition = tank_classes_definitions.get(&game_entity.class_id).unwrap();
                    if current_definition.upgrades.contains(&target_class_id) { is_upgrade_valid = true; };
                }
            }
            if is_upgrade_valid && tank_classes_definitions.contains_key(&target_class_id) {
                let new_definition = tank_classes_definitions.get(&target_class_id).unwrap();
                let next_tier_upgrades = new_definition.upgrades.clone();
                let mut client_id_optional = None;
                if let Some(game_entity) = game_entities.get_mut(&entity_id) {
                    client_id_optional = game_entity.client_id;
                    game_entity.barrels = new_definition.barrels.clone();
                    game_entity.shape = new_definition.shape;
                    game_entity.class_id = new_definition.id;
                    game_entity.fov_factor = new_definition.fov_factor;
                    game_entity.spin_rate = new_definition.auto_spin;
                    game_entity.color = if new_definition.color != 255 { new_definition.color } else { game_entity.team };
                    game_entity.turrets.clear();
                    for (index, turret_config) in new_definition.turrets.iter().enumerate() {
                        let turret_class = tank_classes_definitions.get(&turret_config.class_id).unwrap();
                        game_entity.turrets.push(TurretState { config_idx: index, initial_facing: game_entity.facing + turret_config.angle_offset, facing: game_entity.facing + turret_config.angle_offset, target_pos: None, barrels: turret_class.barrels.clone() });
                    }
                    game_entity.layer_facings = vec![game_entity.facing; new_definition.layers.len()];
                    if new_definition.score != u32::MAX { game_entity.score = new_definition.score; };
                    game_entity.max_health = new_definition.base_health.into();
                    game_entity.health = new_definition.base_health.into();
                    apply_entity_stats(game_entity, &new_definition);
                    if let Some(physics_entity) = world_room.entities.get_mut(entity_id) {
                        physics_entity.radius = new_definition.body_radius;
                        physics_entity.mass = new_definition.mass;
                        let speed_multiplier = 1.0 + (game_entity.stats[7] as f32 * 0.15);
                        physics_entity.terminal_velocity_in_direction = new_definition.base_speed * speed_multiplier;
                        let acceleration_multiplier = 0.05 + (game_entity.stats[8] as f32 * 0.005);
                        physics_entity.movement_acceleration = acceleration_multiplier;
                        physics_entity.movement_acceleration_45_deg = acceleration_multiplier * 0.70710678;
                    }
                }
                if let Some(client_id) = client_id_optional {
                    if let Some(session) = player_sessions.get(&client_id) {
                        let mut upgrades_packet = vec![4];
                        upgrades_packet.extend((entity_id as u64).to_le_bytes());
                        upgrades_packet.push(next_tier_upgrades.len() as u8);
                        upgrades_packet.extend(next_tier_upgrades);
                        let _ = session.sender.send(upgrades_packet);
                        send_notification(&session.sender, &format!("Upgraded to {}.", new_definition.name));
                        if let Some(game_entity) = game_entities.get(&entity_id) {
                            let mut stats_packet = vec![9];
                            stats_packet.push(game_entity.stat_points);
                            stats_packet.extend(&game_entity.stats);
                            let _ = session.sender.send(stats_packet);
                        }
                    }
                }
            }
        },
        6 => {
            if packet_data.len() >= 2 {
                let message_length = packet_data[1] as usize;
                if packet_data.len() >= 2 + message_length {
                    let message_bytes = &packet_data[2..2+message_length];
                    let chat_text = String::from_utf8_lossy(message_bytes).to_string();
                    if !chat_text.is_empty() {
                         if let Some(game_entity) = game_entities.get_mut(&entity_id) {
                             if game_entity.messages.len() < 3 {
                                 game_entity.messages.push(ChatMessage { text: chat_text, duration: 5000, created_at: Instant::now() });
                             }
                         }
                    }
                }
            }
        },
        8 => {
             let stat_index = packet_data[1] as usize;
             if stat_index < STAT_COUNT {
                 let mut did_upgrade = false;
                 let mut client_id_optional = None;
                 if let Some(game_entity) = game_entities.get_mut(&entity_id) {
                     client_id_optional = game_entity.client_id;
                     if game_entity.stat_points > 0 && game_entity.stats[stat_index] < MAX_STAT_LEVEL {
                         game_entity.stat_points -= 1;
                         game_entity.stats[stat_index] += 1;
                         did_upgrade = true;
                         let entity_definition = tank_classes_definitions.get(&game_entity.class_id).unwrap();
                         apply_entity_stats(game_entity, entity_definition);
                         if stat_index == 7 {
                             if let Some(physics_entity) = world_room.entities.get_mut(entity_id) {
                                 let speed_multiplier = 1.0 + (game_entity.stats[7] as f32 * 0.15);
                                 physics_entity.terminal_velocity_in_direction = entity_definition.base_speed * speed_multiplier;
                             }
                         }
                         if stat_index == 8 {
                            if let Some(physics_entity) = world_room.entities.get_mut(entity_id) {
                                let acceleration_val = 0.05 + (game_entity.stats[8] as f32 * 0.005);
                                physics_entity.movement_acceleration = acceleration_val;
                                physics_entity.movement_acceleration_45_deg = acceleration_val * 0.70710678;
                            }
                        }
                     }
                 }
                 if did_upgrade {
                     if let Some(client_id) = client_id_optional {
                         if let Some(session) = player_sessions.get(&client_id) {
                             if let Some(game_entity) = game_entities.get(&entity_id) {
                                 let mut stats_packet = vec![9];
                                 stats_packet.push(game_entity.stat_points);
                                 stats_packet.extend(&game_entity.stats);
                                 let _ = session.sender.send(stats_packet);
                             }
                         }
                     }
                 }
             }
        },
        10 => {
            let toggle_type_code = packet_data[1];
            let mut client_id_optional = None;
            if let Some(game_entity) = game_entities.get_mut(&entity_id) {
                client_id_optional = game_entity.client_id;
                if toggle_type_code == 1 { game_entity.auto_fire = !game_entity.auto_fire; };
                if toggle_type_code == 2 { game_entity.auto_spin_enabled = !game_entity.auto_spin_enabled; };
                if toggle_type_code == 3 { game_entity.is_override = !game_entity.is_override; };
            }
            if let Some(client_id) = client_id_optional {
                if let Some(session) = player_sessions.get(&client_id) {
                    if let Some(game_entity) = game_entities.get(&entity_id) {
                        let toggle_message = match toggle_type_code {
                            1 => if game_entity.auto_fire { "Auto Fire Enabled." } else { "Auto Fire Disabled." },
                            2 => if game_entity.auto_spin_enabled { "Auto Spin Enabled." } else { "Auto Spin Disabled." },
                            3 => if game_entity.is_override { "Override Enabled." } else { "Override Disabled." },
                            _ => ""
                        };
                        send_notification(&session.sender, toggle_message);
                    }
                }
            }
        },
        250 => {
            if let Some(game_entity) = game_entities.get(&entity_id) {
                if let Some(client_id) = game_entity.client_id {
                    if let Some(session) = player_sessions.get(&client_id) {
                        let _ = session.sender.send(packet_data);
                    }
                }
            }
        }
        _ => {}
    }
}